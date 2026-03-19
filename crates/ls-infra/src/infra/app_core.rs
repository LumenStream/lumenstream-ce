impl AppInfra {
    pub async fn init(mut config: AppConfig) -> anyhow::Result<Self> {
        let pool = db::connect(&config).await?;
        db::migrate(&pool).await?;

        if let Some(web_settings) = Self::load_web_settings_from_pool(&pool).await? {
            config.apply_web_config(&web_settings);
        }

        let search_backend = Self::init_search_backend().await;
        let (notification_tx, _) = broadcast::channel(256);
        let (task_run_tx, _) = broadcast::channel(512);
        let (recharge_order_tx, _) = broadcast::channel(256);
        let (agent_request_tx, _) = broadcast::channel(512);

        let this = Self {
            pool,
            config: Arc::new(RwLock::new(config)),
            server_id: Uuid::now_v7().to_string(),
            http_client: Client::new(),
            search_backend,
            metrics: Arc::new(InfraMetrics::default()),
            tmdb_last_request: Arc::new(Mutex::new(None)),
            resized_image_locks: Arc::new(Mutex::new(HashMap::new())),
            notification_tx,
            task_run_tx,
            recharge_order_tx,
            agent_request_tx,
        };
        this.bootstrap().await?;
        this.ensure_web_settings_seeded().await?;
        this.ensure_playback_domains_seeded().await?;
        this.ensure_task_definitions_seeded().await?;
        this.requeue_orphaned_running_jobs().await?;

        Ok(this)
    }

    pub fn runtime_metrics_snapshot(&self) -> Value {
        self.metrics.snapshot()
    }

    pub fn config_snapshot(&self) -> AppConfig {
        self.config
            .read()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone()
    }

    fn apply_runtime_web_config(&self, settings: &WebAppConfig) {
        let mut guard = self
            .config
            .write()
            .unwrap_or_else(|poison| poison.into_inner());
        guard.apply_web_config(settings);
    }

    async fn init_search_backend() -> Option<SearchBackend> {
        let url = MEILI_URL;
        let api_key = meili_api_key();
        let index_uid = MEILI_INDEX;

        let client = match MeiliClient::new(url, api_key.as_deref()) {
            Ok(v) => v,
            Err(err) => {
                warn!(error = %err, meili_url = %url, "failed to initialize meilisearch client");
                return None;
            }
        };

        let index = match client.get_index(index_uid).await {
            Ok(v) => v,
            Err(get_err) => {
                let created = match client.create_index(index_uid, Some("id")).await {
                    Ok(task) => task,
                    Err(create_err) => {
                        warn!(
                            error = %create_err,
                            previous_error = %get_err,
                            index = %index_uid,
                            "failed to get or create meilisearch index"
                        );
                        return None;
                    }
                };

                if let Err(err) = created.wait_for_completion(&client, None, None).await {
                    warn!(error = %err, index = %index_uid, "failed to wait create-index task");
                    return None;
                }

                client.index(index_uid)
            }
        };

        if let Ok(task) = index
            .set_searchable_attributes(["name", "name_pinyin", "name_initials"])
            .await
        {
            if let Err(err) = task.wait_for_completion(&client, None, None).await {
                warn!(error = %err, index = %index_uid, "failed to apply meilisearch searchable attributes");
            }
        }

        if let Ok(task) = index
            .set_filterable_attributes(["item_type", "library_id", "series_id"])
            .await
        {
            if let Err(err) = task.wait_for_completion(&client, None, None).await {
                warn!(error = %err, index = %index_uid, "failed to apply meilisearch filterable attributes");
            }
        }

        info!(meili_url = %url, index = %index_uid, "meilisearch backend enabled");

        Some(SearchBackend { client, index })
    }

    pub async fn get_web_settings(&self) -> anyhow::Result<WebAppConfig> {
        let mut settings = Self::load_web_settings_from_pool(&self.pool)
            .await?
            .unwrap_or_else(|| self.config_snapshot().web_config());
        self.config_snapshot()
            .normalize_web_config_for_edition(&mut settings);
        Ok(settings)
    }

    pub async fn upsert_web_settings(&self, settings: &WebAppConfig) -> anyhow::Result<()> {
        let mut normalized = settings.clone();
        self.config_snapshot()
            .normalize_web_config_for_edition(&mut normalized);
        Self::save_web_settings_to_pool(&self.pool, &normalized).await?;
        self.apply_runtime_web_config(&normalized);
        Ok(())
    }

    async fn ensure_web_settings_seeded(&self) -> anyhow::Result<()> {
        if Self::load_web_settings_from_pool(&self.pool)
            .await?
            .is_none()
        {
            Self::save_web_settings_to_pool(&self.pool, &self.config_snapshot().web_config()).await?;
        }
        Ok(())
    }

    async fn ensure_playback_domains_seeded(&self) -> anyhow::Result<()> {
        let existing =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*)::BIGINT FROM playback_domains")
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0);
        if existing > 0 {
            return Ok(());
        }

        let cfg = self.config_snapshot();
        let nodes = cfg
            .storage
            .lumenbackend_nodes
            .iter()
            .filter_map(|node| normalize_lumenbackend_node(node))
            .collect::<Vec<_>>();
        if nodes.is_empty() {
            return Ok(());
        }

        for (idx, node) in nodes.into_iter().enumerate() {
            let _ = sqlx::query(
                r#"
INSERT INTO playback_domains (id, name, base_url, enabled, priority, is_default)
VALUES ($1, $2, $3, true, 0, $4)
ON CONFLICT(name) DO NOTHING
                "#,
            )
            .bind(Uuid::now_v7())
            .bind(format!("legacy-{}", idx + 1))
            .bind(node)
            .bind(idx == 0)
            .execute(&self.pool)
            .await;
        }

        Ok(())
    }

    async fn ensure_task_definitions_seeded(&self) -> anyhow::Result<()> {
        for task in default_task_definitions() {
            sqlx::query(
                r#"
INSERT INTO task_definitions (task_key, display_name, enabled, cron_expr, default_payload, max_attempts)
VALUES ($1, $2, $3, $4, $5, $6)
ON CONFLICT(task_key) DO NOTHING
                "#,
            )
            .bind(task.task_key)
            .bind(task.display_name)
            .bind(task.enabled)
            .bind(task.cron_expr)
            .bind(task.default_payload)
            .bind(task.max_attempts)
            .execute(&self.pool)
            .await?;
        }

        // Remove retired task definitions so they no longer appear in task center.
        sqlx::query(
            "DELETE FROM task_definitions WHERE task_key = ANY($1::text[])",
        )
        .bind(vec!["cache_prewarm", "scan_library_no_probe", "tmdb_fill"])
            .execute(&self.pool)
            .await?;

        // Mark pending/queued legacy jobs as dead letters to avoid repeated unknown-kind retries.
        sqlx::query(
            r#"
UPDATE jobs
SET status = 'failed',
    dead_letter = true,
    error = 'task removed: cache_prewarm',
    finished_at = now()
WHERE kind = 'cache_prewarm'
  AND status IN ('pending', 'queued', 'running')
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn load_web_settings_from_pool(pool: &PgPool) -> anyhow::Result<Option<WebAppConfig>> {
        let mut value: Option<Value> =
            sqlx::query_scalar("SELECT value FROM web_settings WHERE key = $1 LIMIT 1")
                .bind(WEB_SETTINGS_KEY)
                .fetch_optional(pool)
                .await?;

        if let Some(payload) = value.as_mut() {
            if migrate_legacy_default_library_paths(payload) {
                sqlx::query(
                    r#"
UPDATE web_settings
SET value = $2,
    updated_at = now()
WHERE key = $1
                    "#,
                )
                .bind(WEB_SETTINGS_KEY)
                .bind(payload.clone())
                .execute(pool)
                .await?;
            }
        }

        value
            .map(|v| {
                serde_json::from_value::<WebAppConfig>(v)
                    .context("failed to deserialize web settings payload")
            })
            .transpose()
    }

    async fn save_web_settings_to_pool(
        pool: &PgPool,
        settings: &WebAppConfig,
    ) -> anyhow::Result<()> {
        let value =
            serde_json::to_value(settings).context("failed to serialize web settings payload")?;

        sqlx::query(
            r#"
INSERT INTO web_settings (key, value, updated_at)
VALUES ($1, $2, now())
ON CONFLICT(key) DO UPDATE SET
    value = EXCLUDED.value,
    updated_at = now()
            "#,
        )
        .bind(WEB_SETTINGS_KEY)
        .bind(value)
        .execute(pool)
        .await?;

        Ok(())
    }

    async fn bootstrap(&self) -> anyhow::Result<()> {
        let has_admin_user = self.has_admin_user().await?;
        validate_runtime_bootstrap_credentials(has_admin_user, &self.config_snapshot().auth)
            .context("failed runtime bootstrap credential validation")?;

        self.ensure_admin_user().await?;
        self.ensure_default_library().await?;
        Ok(())
    }

    async fn has_admin_user(&self) -> anyhow::Result<bool> {
        let existing_admin: Option<bool> =
            sqlx::query_scalar("SELECT true FROM users WHERE is_admin = true LIMIT 1")
                .fetch_optional(&self.pool)
                .await?;

        Ok(existing_admin.unwrap_or(false))
    }

    async fn ensure_admin_user(&self) -> anyhow::Result<()> {
        let cfg = self.config_snapshot();
        let username = cfg.auth.bootstrap_admin_user.trim();
        if username.is_empty() {
            return Ok(());
        }

        let exists: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM users WHERE username = $1 LIMIT 1")
                .bind(username)
                .fetch_optional(&self.pool)
                .await?;
        if exists.is_some() {
            return Ok(());
        }

        let hash = auth::hash_password(&cfg.auth.bootstrap_admin_password);
        let admin_user_id = Uuid::now_v7();
        sqlx::query(
            r#"
INSERT INTO users (id, username, password_hash, role, is_admin, is_disabled)
VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(admin_user_id)
        .bind(username)
        .bind(hash)
        .bind("Admin")
        .bind(true)
        .bind(false)
        .execute(&self.pool)
        .await?;

        self.ensure_default_user_stream_policy(admin_user_id)
            .await?;
        self.ensure_default_playlist(admin_user_id).await?;

        info!(username, "bootstrap admin user created");
        Ok(())
    }

    async fn ensure_default_library(&self) -> anyhow::Result<()> {
        let cfg = self.config_snapshot();
        let mut configured_paths = cfg
            .scan
            .default_library_paths
            .iter()
            .map(|path| path.trim().to_string())
            .filter(|path| !path.is_empty())
            .collect::<Vec<_>>();
        configured_paths = normalize_library_paths(&configured_paths);
        if configured_paths.is_empty() {
            return Ok(());
        }

        let existing_default = self
            .get_library_by_name(cfg.scan.default_library_name.as_str())
            .await?;
        if let Some(existing) = existing_default {
            let _ = self
                .replace_library_paths(existing.id, &configured_paths)
                .await?;
            return Ok(());
        }

        let _ = self
            .create_library(
                cfg.scan.default_library_name.as_str(),
                &configured_paths,
                "Mixed",
            )
            .await?;

        Ok(())
    }

}

fn migrate_legacy_default_library_paths(payload: &mut Value) -> bool {
    let Some(scan) = payload.get_mut("scan").and_then(Value::as_object_mut) else {
        return false;
    };

    let mut changed = false;
    if !scan.contains_key("default_library_paths") {
        let migrated = scan
            .remove("default_library_path")
            .and_then(|raw| raw.as_str().map(str::to_string))
            .map(|raw| raw.trim().to_string())
            .filter(|raw| !raw.is_empty())
            .map(|raw| vec![Value::String(raw)])
            .unwrap_or_default();
        scan.insert(
            "default_library_paths".to_string(),
            Value::Array(migrated),
        );
        changed = true;
    }

    if scan.remove("default_library_path").is_some() {
        changed = true;
    }

    changed
}
