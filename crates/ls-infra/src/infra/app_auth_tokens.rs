impl AppInfra {
    pub async fn authenticate_user(
        &self,
        username: &str,
        password: &str,
        client: &str,
        device_name: &str,
        device_id: &str,
        remote_addr: Option<&str>,
        application_version: Option<&str>,
    ) -> anyhow::Result<AuthenticateUserResult> {
        if self
            .is_auth_blocked(remote_addr, username)
            .await
            .unwrap_or(false)
        {
            self.record_auth_risk_event(
                remote_addr,
                username,
                "blocked_by_risk_window",
                json!({ "reason": "too_many_failures" }),
            )
            .await?;
            return Ok(AuthenticateUserResult::InvalidCredentials);
        }

        let user = sqlx::query_as::<_, UserRow>(
            r#"
SELECT id, username, password_hash, role, is_admin, is_disabled
FROM users
WHERE username = $1
LIMIT 1
            "#,
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        let Some(user) = user else {
            self.record_auth_risk_event(remote_addr, username, "invalid_username", json!({}))
                .await?;
            return Ok(AuthenticateUserResult::InvalidCredentials);
        };

        if user.is_disabled {
            self.record_auth_risk_event(
                remote_addr,
                username,
                "user_disabled",
                json!({ "user_id": user.id }),
            )
            .await?;
            return Ok(AuthenticateUserResult::InvalidCredentials);
        }

        match auth::verify_password(password, &user.password_hash) {
            auth::PasswordVerifyOutcome::Verified => {}
            auth::PasswordVerifyOutcome::Invalid => {
                self.record_auth_risk_event(
                    remote_addr,
                    username,
                    "invalid_password",
                    json!({ "user_id": user.id }),
                )
                .await?;
                return Ok(AuthenticateUserResult::InvalidCredentials);
            }
            auth::PasswordVerifyOutcome::ResetRequired => {
                self.record_auth_risk_event(
                    remote_addr,
                    username,
                    "legacy_password_hash_rejected",
                    json!({ "user_id": user.id }),
                )
                .await?;
                return Ok(AuthenticateUserResult::PasswordResetRequired);
            }
        }

        let token = auth::new_access_token();
        let expires_at = auth::expires_at(self.config_snapshot().auth.token_ttl_hours);

        sqlx::query(
            r#"
INSERT INTO access_tokens (token, user_id, expires_at)
VALUES ($1, $2, $3)
            "#,
        )
        .bind(&token)
        .bind(user.id)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        let normalized_client = if client.is_empty() {
            "ls-client".to_string()
        } else {
            client.to_string()
        };
        let normalized_device_name = if device_name.is_empty() {
            "ls-device".to_string()
        } else {
            device_name.to_string()
        };
        let normalized_device_id = if device_id.is_empty() {
            Uuid::now_v7().to_string()
        } else {
            device_id.to_string()
        };

        let session_id = Uuid::now_v7();
        sqlx::query(
            r#"
INSERT INTO user_sessions (
    id,
    token,
    user_id,
    client,
    device_name,
    device_id,
    remote_addr,
    is_active
) VALUES (
    $1, $2, $3, $4, $5, $6, $7, true
)
            "#,
        )
        .bind(session_id)
        .bind(&token)
        .bind(user.id)
        .bind(&normalized_client)
        .bind(&normalized_device_name)
        .bind(&normalized_device_id)
        .bind(remote_addr.map(str::to_string))
        .execute(&self.pool)
        .await?;

        let now = Utc::now();
        let now_text = format_emby_datetime(now);
        let internal_device_id = derive_internal_device_id(session_id, &normalized_device_id);
        let mut user_dto = to_user_dto(&user, &self.server_id);
        user_dto.last_login_date = Some(now_text.clone());
        user_dto.last_activity_date = Some(now_text.clone());

        let auth_result = AuthenticationResultDto {
            user: user_dto,
            session_info: SessionInfoDto {
                id: session_id.to_string(),
                user_id: user.id.to_string(),
                user_name: user.username.clone(),
                client: normalized_client,
                device_name: normalized_device_name,
                device_id: normalized_device_id,
                server_id: Some(self.server_id.clone()),
                last_activity_date: Some(now_text),
                application_version: Some(
                    application_version
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .unwrap_or(env!("CARGO_PKG_VERSION"))
                        .to_string(),
                ),
                device_type: Some("Unknown".to_string()),
                supported_commands: Some(Vec::new()),
                supports_remote_control: Some(false),
                play_state: Some(default_session_play_state()),
                additional_users: Some(Vec::new()),
                remote_end_point: remote_addr.map(str::to_string),
                protocol: Some("HTTP/1.1".to_string()),
                playable_media_types: Some(Vec::new()),
                playlist_index: Some(0),
                playlist_length: Some(0),
                internal_device_id: Some(internal_device_id),
            },
            access_token: token,
            server_id: self.server_id.clone(),
        };

        Ok(AuthenticateUserResult::Success(auth_result))
    }

    pub async fn resolve_user_from_token(&self, token: &str) -> anyhow::Result<Option<UserDto>> {
        if let Some(user) = self.resolve_user_from_access_token(token).await? {
            return Ok(Some(user));
        }

        self.resolve_user_from_admin_api_key(token).await
    }

    async fn resolve_user_from_access_token(&self, token: &str) -> anyhow::Result<Option<UserDto>> {
        let user = sqlx::query_as::<_, UserRow>(
            r#"
SELECT u.id, u.username, u.password_hash, u.role, u.is_admin, u.is_disabled
FROM access_tokens t
JOIN users u ON u.id = t.user_id
LEFT JOIN user_sessions s ON s.token = t.token
WHERE t.token = $1
  AND t.expires_at > now()
  AND u.is_disabled = false
  AND (s.id IS NULL OR s.is_active = true)
LIMIT 1
            "#,
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;

        if user.is_some() {
            sqlx::query(
                "UPDATE user_sessions SET last_seen_at = now() WHERE token = $1 AND is_active = true",
            )
            .bind(token)
            .execute(&self.pool)
            .await?;
        }

        Ok(user.map(|u| to_user_dto(&u, &self.server_id)))
    }

    async fn resolve_user_from_admin_api_key(
        &self,
        token: &str,
    ) -> anyhow::Result<Option<UserDto>> {
        let key_hash = auth::hash_api_key(token);

        let key_found: Option<Uuid> = sqlx::query_scalar(
            r#"
SELECT id
FROM admin_api_keys
WHERE key_hash = $1
LIMIT 1
            "#,
        )
        .bind(&key_hash)
        .fetch_optional(&self.pool)
        .await?;

        let Some(key_id) = key_found else {
            return Ok(None);
        };

        sqlx::query("UPDATE admin_api_keys SET last_used_at = now() WHERE id = $1")
            .bind(key_id)
            .execute(&self.pool)
            .await?;

        let admin_user = sqlx::query_as::<_, UserRow>(
            r#"
SELECT id, username, password_hash, role, is_admin, is_disabled
FROM users
WHERE is_disabled = false AND (role = 'Admin' OR is_admin = true)
ORDER BY created_at ASC
LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(admin_user.map(|u| to_user_dto(&u, &self.server_id)))
    }

    pub async fn list_public_users(&self) -> anyhow::Result<Vec<UserDto>> {
        let rows = sqlx::query_as::<_, UserRow>(
            r#"
SELECT id, username, password_hash, role, is_admin, is_disabled
FROM users
WHERE is_disabled = false
ORDER BY username ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .iter()
            .map(|row| to_user_dto(row, &self.server_id))
            .collect())
    }

    pub async fn get_user_by_id(&self, user_id: Uuid) -> anyhow::Result<Option<UserDto>> {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
SELECT id, username, password_hash, role, is_admin, is_disabled
FROM users
WHERE id = $1
LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.as_ref().map(|v| to_user_dto(v, &self.server_id)))
    }

    pub async fn revoke_access_token(&self, token: &str) -> anyhow::Result<bool> {
        sqlx::query("UPDATE user_sessions SET is_active = false WHERE token = $1")
            .bind(token)
            .execute(&self.pool)
            .await?;

        let deleted = sqlx::query("DELETE FROM access_tokens WHERE token = $1")
            .bind(token)
            .execute(&self.pool)
            .await?
            .rows_affected();
        Ok(deleted > 0)
    }

    pub async fn list_users_for_admin(&self) -> anyhow::Result<Vec<UserDto>> {
        let rows = sqlx::query_as::<_, UserRow>(
            r#"
SELECT id, username, password_hash, role, is_admin, is_disabled
FROM users
ORDER BY created_at DESC, username ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .iter()
            .map(|row| to_user_dto(row, &self.server_id))
            .collect())
    }

}

fn default_session_play_state() -> Value {
    json!({
        "CanSeek": false,
        "IsPaused": false,
        "IsMuted": false,
        "RepeatMode": "RepeatNone",
        "SleepTimerMode": "None",
        "SubtitleOffset": 0,
        "Shuffle": false,
        "PlaybackRate": 1,
    })
}

fn derive_internal_device_id(session_id: Uuid, device_id: &str) -> i64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(&session_id, &mut hasher);
    std::hash::Hash::hash(&device_id, &mut hasher);
    let value = std::hash::Hasher::finish(&hasher) & (i64::MAX as u64);
    i64::try_from(value).unwrap_or(i64::MAX)
}
