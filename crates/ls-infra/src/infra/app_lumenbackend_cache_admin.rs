#[derive(Debug, Clone, Default)]
struct RuntimeSchemaFieldValidators {
    min: Option<f64>,
    max: Option<f64>,
    min_length: Option<usize>,
    max_length: Option<usize>,
    pattern: Option<String>,
    url: bool,
}

#[derive(Debug, Clone)]
struct RuntimeSchemaFieldSpec {
    key: String,
    field_type: String,
    required: bool,
    default_value: Option<Value>,
    option_values: Vec<String>,
    validators: RuntimeSchemaFieldValidators,
}

fn json_number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(raw) => raw.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn json_bool(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(v) => Some(*v),
        Value::String(raw) => match raw.trim().to_ascii_lowercase().as_str() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

fn config_value_by_path<'a>(config: &'a Value, key: &str) -> Option<&'a Value> {
    if key.trim().is_empty() {
        return None;
    }
    let mut current = config;
    for part in key.split('.') {
        if part.trim().is_empty() {
            return None;
        }
        let map = current.as_object()?;
        current = map.get(part)?;
    }
    Some(current)
}

fn set_config_value_by_path(config: &mut Value, key: &str, value: Value) {
    if key.trim().is_empty() {
        return;
    }

    let parts = key.split('.').collect::<Vec<_>>();
    if parts.is_empty() {
        return;
    }

    let mut current = config;
    for (idx, part) in parts.iter().enumerate() {
        if part.trim().is_empty() {
            return;
        }
        let is_last = idx + 1 == parts.len();
        let Some(map) = current.as_object_mut() else {
            return;
        };
        if is_last {
            map.insert((*part).to_string(), value);
            return;
        }
        current = map
            .entry((*part).to_string())
            .or_insert_with(|| Value::Object(serde_json::Map::new()));
    }
}

fn collect_leaf_paths(value: &Value, prefix: Option<&str>, acc: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            if map.is_empty() {
                if let Some(path) = prefix {
                    acc.push(path.to_string());
                }
                return;
            }
            for (key, child) in map {
                let next = if let Some(path) = prefix {
                    format!("{path}.{key}")
                } else {
                    key.to_string()
                };
                collect_leaf_paths(child, Some(&next), acc);
            }
        }
        _ => {
            if let Some(path) = prefix {
                acc.push(path.to_string());
            }
        }
    }
}

fn parse_runtime_schema_fields(schema: &Value) -> anyhow::Result<Vec<RuntimeSchemaFieldSpec>> {
    let schema_obj = schema
        .as_object()
        .context("runtime schema must be object with sections")?;
    let sections = schema_obj
        .get("sections")
        .and_then(Value::as_array)
        .context("runtime schema sections must be array")?;

    let mut fields = Vec::new();
    let mut seen = HashSet::new();
    for section in sections {
        let section_obj = section
            .as_object()
            .context("runtime schema section must be object")?;
        let section_fields = section_obj
            .get("fields")
            .and_then(Value::as_array)
            .context("runtime schema section.fields must be array")?;
        for field in section_fields {
            let field_obj = field
                .as_object()
                .context("runtime schema field must be object")?;
            let key = field_obj
                .get("key")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_string();
            if key.is_empty() {
                anyhow::bail!("runtime schema field key is required");
            }
            if !seen.insert(key.clone()) {
                anyhow::bail!("runtime schema field key `{key}` is duplicated");
            }

            let field_type = field_obj
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_ascii_lowercase();
            if !matches!(
                field_type.as_str(),
                "string" | "number" | "boolean" | "select" | "password" | "textarea"
            ) {
                anyhow::bail!("runtime schema field `{key}` has unsupported type");
            }

            let required = field_obj
                .get("required")
                .and_then(json_bool)
                .unwrap_or(false);
            let default_value = field_obj.get("default").cloned();

            let option_values = field_obj
                .get("options")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| {
                            if let Some(value) = item.as_str() {
                                let cleaned = value.trim();
                                if cleaned.is_empty() {
                                    None
                                } else {
                                    Some(cleaned.to_string())
                                }
                            } else if let Some(value) =
                                item.get("value").and_then(Value::as_str).map(str::trim)
                            {
                                if value.is_empty() {
                                    None
                                } else {
                                    Some(value.to_string())
                                }
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if field_type == "select" && option_values.is_empty() {
                anyhow::bail!("runtime schema select field `{key}` requires options");
            }

            let validators = if let Some(raw) = field_obj.get("validators") {
                let validators_obj = raw
                    .as_object()
                    .context("runtime schema field.validators must be object")?;
                let pattern = validators_obj
                    .get("pattern")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .map(ToString::to_string);
                if let Some(value) = pattern.as_deref() {
                    Regex::new(value).context("runtime schema validator pattern is invalid")?;
                }
                RuntimeSchemaFieldValidators {
                    min: validators_obj.get("min").and_then(json_number),
                    max: validators_obj.get("max").and_then(json_number),
                    min_length: validators_obj
                        .get("min_length")
                        .and_then(json_number)
                        .map(|v| v.max(0.0) as usize),
                    max_length: validators_obj
                        .get("max_length")
                        .and_then(json_number)
                        .map(|v| v.max(0.0) as usize),
                    pattern,
                    url: validators_obj
                        .get("url")
                        .and_then(json_bool)
                        .unwrap_or(false),
                }
            } else {
                RuntimeSchemaFieldValidators::default()
            };

            fields.push(RuntimeSchemaFieldSpec {
                key,
                field_type,
                required,
                default_value,
                option_values,
                validators,
            });
        }
    }
    Ok(fields)
}

fn validate_runtime_field_value(
    field: &RuntimeSchemaFieldSpec,
    raw_value: &Value,
) -> anyhow::Result<()> {
    match field.field_type.as_str() {
        "boolean" => {
            if json_bool(raw_value).is_none() {
                anyhow::bail!("runtime config field `{}` must be boolean", field.key);
            }
        }
        "number" => {
            let Some(number_value) = json_number(raw_value) else {
                anyhow::bail!("runtime config field `{}` must be number", field.key);
            };
            if let Some(min) = field.validators.min {
                if number_value < min {
                    anyhow::bail!("runtime config field `{}` should be >= {}", field.key, min);
                }
            }
            if let Some(max) = field.validators.max {
                if number_value > max {
                    anyhow::bail!("runtime config field `{}` should be <= {}", field.key, max);
                }
            }
        }
        "select" | "string" | "password" | "textarea" => {
            let Some(text_value) = raw_value.as_str().map(str::trim) else {
                anyhow::bail!("runtime config field `{}` must be string", field.key);
            };
            if field.field_type == "select" && !field.option_values.iter().any(|v| v == text_value)
            {
                anyhow::bail!(
                    "runtime config field `{}` must be one of declared options",
                    field.key
                );
            }
            if let Some(min_len) = field.validators.min_length {
                if text_value.chars().count() < min_len {
                    anyhow::bail!("runtime config field `{}` is too short", field.key);
                }
            }
            if let Some(max_len) = field.validators.max_length {
                if text_value.chars().count() > max_len {
                    anyhow::bail!("runtime config field `{}` is too long", field.key);
                }
            }
            if let Some(pattern) = field.validators.pattern.as_deref() {
                let regex = Regex::new(pattern).context("runtime schema validator pattern is invalid")?;
                if !regex.is_match(text_value) {
                    anyhow::bail!("runtime config field `{}` does not match pattern", field.key);
                }
            }
            if field.validators.url {
                let parsed = reqwest::Url::parse(text_value)
                    .map_err(|_| anyhow::anyhow!("runtime config field `{}` must be valid URL", field.key))?;
                if parsed.scheme() != "http" && parsed.scheme() != "https" {
                    anyhow::bail!("runtime config field `{}` must use http/https URL", field.key);
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_runtime_config_against_schema(
    config: &Value,
    schema_fields: &[RuntimeSchemaFieldSpec],
) -> anyhow::Result<()> {
    if !config.is_object() {
        anyhow::bail!("runtime config must be object");
    }

    let mut leaf_paths = Vec::new();
    collect_leaf_paths(config, None, &mut leaf_paths);
    let allowed = schema_fields
        .iter()
        .map(|field| field.key.as_str())
        .collect::<HashSet<_>>();
    for path in leaf_paths {
        if !allowed.contains(path.as_str()) {
            anyhow::bail!("runtime config field `{path}` is not declared in runtime schema");
        }
    }

    for field in schema_fields {
        let value = config_value_by_path(config, field.key.as_str());
        if value.is_none() || matches!(value, Some(Value::Null)) {
            if field.required && field.default_value.is_none() {
                anyhow::bail!("runtime config field `{}` is required", field.key);
            }
            continue;
        }
        validate_runtime_field_value(field, value.expect("checked above"))?;
    }

    Ok(())
}

fn build_schema_default_runtime_config(schema_fields: &[RuntimeSchemaFieldSpec]) -> Value {
    let mut base = Value::Object(serde_json::Map::new());
    for field in schema_fields {
        if let Some(default_value) = field.default_value.clone() {
            set_config_value_by_path(&mut base, field.key.as_str(), default_value);
        }
    }
    base
}

impl AppInfra {
    pub async fn list_lumenbackend_nodes(&self) -> anyhow::Result<Vec<LumenBackendNode>> {
        let rows = sqlx::query_as::<_, LumenBackendNodeRow>(
            r#"
SELECT node_id, name, enabled, last_seen_at, last_version, last_status, created_at, updated_at
FROM lumenbackend_nodes
ORDER BY enabled DESC, updated_at DESC, node_id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn get_lumenbackend_node(&self, node_id: &str) -> anyhow::Result<Option<LumenBackendNode>> {
        let row = sqlx::query_as::<_, LumenBackendNodeRow>(
            r#"
SELECT node_id, name, enabled, last_seen_at, last_version, last_status, created_at, updated_at
FROM lumenbackend_nodes
WHERE node_id = $1
LIMIT 1
            "#,
        )
        .bind(node_id.trim())
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Into::into))
    }

    pub async fn create_lumenbackend_node(
        &self,
        node_id: &str,
        name: Option<&str>,
        enabled: bool,
    ) -> anyhow::Result<LumenBackendNode> {
        let normalized_node_id = node_id.trim();
        if normalized_node_id.is_empty() {
            anyhow::bail!("node_id is required");
        }

        let normalized_name = name
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);

        let row = sqlx::query_as::<_, LumenBackendNodeRow>(
            r#"
INSERT INTO lumenbackend_nodes (
    node_id,
    name,
    enabled,
    last_status,
    created_at,
    updated_at
)
VALUES (
    $1,
    $2,
    $3,
    '{}'::jsonb,
    now(),
    now()
)
RETURNING node_id, name, enabled, last_seen_at, last_version, last_status, created_at, updated_at
            "#,
        )
        .bind(normalized_node_id)
        .bind(normalized_name)
        .bind(enabled)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    pub async fn update_lumenbackend_node(
        &self,
        node_id: &str,
        name: Option<Option<String>>,
        enabled: Option<bool>,
    ) -> anyhow::Result<Option<LumenBackendNode>> {
        let normalized_node_id = node_id.trim();
        if normalized_node_id.is_empty() {
            anyhow::bail!("node_id is required");
        }
        if name.is_none() && enabled.is_none() {
            anyhow::bail!("at least one field is required");
        }

        let has_name = name.is_some();
        let normalized_name = name.flatten().map(|raw| {
            let cleaned = raw.trim().to_string();
            if cleaned.is_empty() {
                String::new()
            } else {
                cleaned
            }
        });
        let name_value = if has_name {
            normalized_name.and_then(|raw| if raw.is_empty() { None } else { Some(raw) })
        } else {
            None
        };

        let row = sqlx::query_as::<_, LumenBackendNodeRow>(
            r#"
UPDATE lumenbackend_nodes
SET
    name = CASE WHEN $2 THEN $3 ELSE name END,
    enabled = COALESCE($4, enabled),
    updated_at = now()
WHERE node_id = $1
RETURNING node_id, name, enabled, last_seen_at, last_version, last_status, created_at, updated_at
            "#,
        )
        .bind(normalized_node_id)
        .bind(has_name)
        .bind(name_value)
        .bind(enabled)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Into::into))
    }

    pub async fn delete_lumenbackend_node(&self, node_id: &str) -> anyhow::Result<bool> {
        let normalized_node_id = node_id.trim();
        if normalized_node_id.is_empty() {
            anyhow::bail!("node_id is required");
        }

        let bound_domain_count = sqlx::query_scalar::<_, i64>(
            r#"
SELECT COUNT(*)::bigint
FROM playback_domains
WHERE lumenbackend_node_id = $1
            "#,
        )
        .bind(normalized_node_id)
        .fetch_one(&self.pool)
        .await?
        .max(0);
        if bound_domain_count > 0 {
            anyhow::bail!("lumenbackend node is still bound to playback domains");
        }

        sqlx::query(
            r#"
DELETE FROM lumenbackend_runtime_configs
WHERE scope = 'node' AND scope_key = $1
            "#,
        )
        .bind(normalized_node_id)
        .execute(&self.pool)
        .await?;
        sqlx::query(
            r#"
DELETE FROM lumenbackend_node_runtime_schemas
WHERE node_id = $1
            "#,
        )
        .bind(normalized_node_id)
        .execute(&self.pool)
        .await?;

        let deleted = sqlx::query(
            r#"
DELETE FROM lumenbackend_nodes
WHERE node_id = $1
            "#,
        )
        .bind(normalized_node_id)
        .execute(&self.pool)
        .await?;
        Ok(deleted.rows_affected() > 0)
    }

    pub async fn register_lumenbackend_node(
        &self,
        payload: LumenBackendNodeRegister,
    ) -> anyhow::Result<LumenBackendNode> {
        let node_id = payload.node_id.trim();
        if node_id.is_empty() {
            anyhow::bail!("node_id is required");
        }

        let row = sqlx::query_as::<_, LumenBackendNodeRow>(
            r#"
UPDATE lumenbackend_nodes
SET
    name = COALESCE($2, name),
    last_seen_at = now(),
    last_version = COALESCE($3, last_version),
    last_status = $4,
    updated_at = now()
WHERE node_id = $1
RETURNING node_id, name, enabled, last_seen_at, last_version, last_status, created_at, updated_at
            "#,
        )
        .bind(node_id)
        .bind(payload.name.as_deref())
        .bind(payload.version.as_deref())
        .bind(payload.status)
        .fetch_optional(&self.pool)
        .await?;
        row.map(Into::into)
            .ok_or_else(|| anyhow::anyhow!("node not registered"))
    }

    pub async fn heartbeat_lumenbackend_node(
        &self,
        payload: LumenBackendNodeHeartbeat,
    ) -> anyhow::Result<Option<LumenBackendNode>> {
        let node_id = payload.node_id.trim();
        if node_id.is_empty() {
            return Ok(None);
        }

        let row = sqlx::query_as::<_, LumenBackendNodeRow>(
            r#"
UPDATE lumenbackend_nodes
SET
    last_seen_at = now(),
    last_version = COALESCE($2, last_version),
    last_status = $3,
    updated_at = now()
WHERE node_id = $1
RETURNING node_id, name, enabled, last_seen_at, last_version, last_status, created_at, updated_at
            "#,
        )
        .bind(node_id)
        .bind(payload.version.as_deref())
        .bind(payload.status)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    async fn get_latest_lumenbackend_runtime_schema_row(
        &self,
        node_id: &str,
    ) -> anyhow::Result<Option<LumenBackendRuntimeSchemaRow>> {
        let normalized_node_id = node_id.trim();
        if normalized_node_id.is_empty() {
            return Ok(None);
        }

        let row = sqlx::query_as::<_, LumenBackendRuntimeSchemaRow>(
            r#"
SELECT node_id, schema_version, schema_hash, schema, updated_at
FROM lumenbackend_node_runtime_schemas
WHERE node_id = $1
ORDER BY updated_at DESC, schema_version DESC
LIMIT 1
            "#,
        )
        .bind(normalized_node_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn get_lumenbackend_node_runtime_schema(
        &self,
        node_id: &str,
    ) -> anyhow::Result<Option<LumenBackendNodeRuntimeSchema>> {
        let normalized_node_id = node_id.trim();
        if normalized_node_id.is_empty() {
            anyhow::bail!("node_id is required");
        }

        let node_exists = self.get_lumenbackend_node(normalized_node_id).await?.is_some();
        if !node_exists {
            anyhow::bail!("lumenbackend node not found");
        }

        Ok(self
            .get_latest_lumenbackend_runtime_schema_row(normalized_node_id)
            .await?
            .map(Into::into))
    }

    pub async fn upsert_lumenbackend_node_runtime_schema(
        &self,
        node_id: &str,
        schema_version: &str,
        schema_hash: Option<&str>,
        schema: Value,
    ) -> anyhow::Result<LumenBackendNodeRuntimeSchema> {
        let normalized_node_id = node_id.trim();
        if normalized_node_id.is_empty() {
            anyhow::bail!("node_id is required");
        }
        let normalized_version = schema_version.trim();
        if normalized_version.is_empty() {
            anyhow::bail!("runtime schema version is required");
        }
        if !schema.is_object() {
            anyhow::bail!("runtime schema must be object");
        }
        parse_runtime_schema_fields(&schema)?;

        let node_exists = self.get_lumenbackend_node(normalized_node_id).await?.is_some();
        if !node_exists {
            anyhow::bail!("lumenbackend node not found");
        }

        let row = sqlx::query_as::<_, LumenBackendRuntimeSchemaRow>(
            r#"
INSERT INTO lumenbackend_node_runtime_schemas (
    id,
    node_id,
    schema_version,
    schema_hash,
    schema,
    created_at,
    updated_at
)
VALUES (
    $1,
    $2,
    $3,
    $4,
    $5,
    now(),
    now()
)
ON CONFLICT(node_id, schema_version) DO UPDATE SET
    schema_hash = EXCLUDED.schema_hash,
    schema = EXCLUDED.schema,
    updated_at = now()
RETURNING node_id, schema_version, schema_hash, schema, updated_at
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(normalized_node_id)
        .bind(normalized_version)
        .bind(schema_hash.map(str::trim).filter(|value| !value.is_empty()))
        .bind(schema)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    async fn get_latest_lumenbackend_runtime_config_row(
        &self,
        node_scope_key: &str,
    ) -> anyhow::Result<Option<LumenBackendRuntimeConfigRow>> {
        if node_scope_key.trim().is_empty() {
            return Ok(None);
        }

        let row = sqlx::query_as::<_, LumenBackendRuntimeConfigRow>(
            r#"
SELECT version, config
FROM lumenbackend_runtime_configs
WHERE scope = 'node' AND scope_key = $1
ORDER BY version DESC
LIMIT 1
            "#,
        )
        .bind(node_scope_key.trim())
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn get_lumenbackend_node_runtime_config(
        &self,
        node_id: &str,
        include_secrets: bool,
    ) -> anyhow::Result<LumenBackendNodeRuntimeConfig> {
        let node_scope_key = node_id.trim();
        if node_scope_key.is_empty() {
            anyhow::bail!("node_id is required");
        }
        if self.get_lumenbackend_node(node_scope_key).await?.is_none() {
            anyhow::bail!("lumenbackend node not found");
        }

        let row = self
            .get_latest_lumenbackend_runtime_config_row(node_scope_key)
            .await?;
        let version = row.as_ref().map(|item| item.version).unwrap_or(0);
        let mut config = row
            .map(|item| item.config)
            .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
        config = self
            .normalize_lumenbackend_runtime_payload(node_scope_key, config)
            .await?;
        if !include_secrets {
            config = mask_secret_fields(config);
        }

        Ok(LumenBackendNodeRuntimeConfig {
            node_id: node_scope_key.to_string(),
            version,
            config,
        })
    }

    pub async fn upsert_lumenbackend_node_runtime_config(
        &self,
        node_id: &str,
        config: Value,
    ) -> anyhow::Result<LumenBackendNodeRuntimeConfig> {
        let node_scope_key = node_id.trim();
        if node_scope_key.is_empty() {
            anyhow::bail!("node_id is required");
        }
        if !config.is_object() {
            anyhow::bail!("config must be object");
        }
        if self.get_lumenbackend_node(node_scope_key).await?.is_none() {
            anyhow::bail!("lumenbackend node not found");
        }

        let schema_row = self
            .get_latest_lumenbackend_runtime_schema_row(node_scope_key)
            .await?
            .ok_or_else(|| anyhow::anyhow!("runtime schema not reported"))?;
        let schema_fields = parse_runtime_schema_fields(&schema_row.schema)?;
        validate_runtime_config_against_schema(&config, &schema_fields)?;

        let current_row = self
            .get_latest_lumenbackend_runtime_config_row(node_scope_key)
            .await?;
        let mut merged_config = current_row
            .as_ref()
            .map(|row| merge_secret_placeholders(config.clone(), &row.config))
            .unwrap_or(config);
        strip_node_runtime_protected_fields(&mut merged_config);
        validate_runtime_config_against_schema(&merged_config, &schema_fields)?;
        let next_version = current_row
            .as_ref()
            .map(|row| row.version.saturating_add(1))
            .unwrap_or(1);

        sqlx::query(
            r#"
INSERT INTO lumenbackend_runtime_configs (id, scope, scope_key, version, config)
VALUES ($1, 'node', $2, $3, $4)
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(node_scope_key)
        .bind(next_version)
        .bind(merged_config.clone())
        .execute(&self.pool)
        .await?;

        let normalized = self
            .normalize_lumenbackend_runtime_payload(node_scope_key, merged_config)
            .await?;
        Ok(LumenBackendNodeRuntimeConfig {
            node_id: node_scope_key.to_string(),
            version: next_version,
            config: normalized,
        })
    }

    async fn normalize_lumenbackend_runtime_payload(
        &self,
        node_id: &str,
        incoming: Value,
    ) -> anyhow::Result<Value> {
        let mut base = self.default_lumenbackend_runtime_payload(node_id).await?;
        merge_json_values(&mut base, &incoming);
        apply_global_lumenbackend_stream_fields(
            &mut base,
            self.config_snapshot().storage.lumenbackend_route.as_str(),
            self.config_snapshot().storage.lumenbackend_stream_signing_key.as_str(),
            self.config_snapshot().storage.lumenbackend_stream_token_ttl_seconds,
        );
        self.apply_runtime_playback_domains(&mut base).await?;
        Ok(base)
    }

    async fn default_lumenbackend_runtime_payload(&self, node_id: &str) -> anyhow::Result<Value> {
        let schema_row = self.get_latest_lumenbackend_runtime_schema_row(node_id).await?;
        let schema_fields = match schema_row {
            Some(row) => parse_runtime_schema_fields(&row.schema)?,
            None => Vec::new(),
        };
        Ok(build_schema_default_runtime_config(&schema_fields))
    }

    async fn apply_runtime_playback_domains(&self, cfg: &mut Value) -> anyhow::Result<()> {
        let domains = self
            .list_playback_domains()
            .await?
            .into_iter()
            .filter(|item| item.enabled)
            .map(|item| {
                json!({
                    "id": item.id,
                    "name": item.name,
                    "base_url": item.base_url,
                    "priority": item.priority,
                    "is_default": item.is_default,
                    "lumenbackend_node_id": item.lumenbackend_node_id,
                    "traffic_multiplier": item.traffic_multiplier
                })
            })
            .collect::<Vec<_>>();

        if !cfg.is_object() {
            *cfg = Value::Object(serde_json::Map::new());
        }
        if let Some(obj) = cfg.as_object_mut() {
            obj.insert("playback_domains".to_string(), Value::Array(domains));
        }
        Ok(())
    }

    pub async fn get_lumenbackend_runtime_config(
        &self,
        node_id: &str,
    ) -> anyhow::Result<LumenBackendRuntimeConfig> {
        let runtime = self.get_lumenbackend_node_runtime_config(node_id, true).await?;
        Ok(LumenBackendRuntimeConfig {
            version: runtime.version,
            config: runtime.config,
        })
    }

    pub async fn verify_admin_api_key(&self, token: &str) -> anyhow::Result<bool> {
        let token = token.trim();
        if token.is_empty() {
            return Ok(false);
        }

        let key_hash = auth::hash_api_key(token);
        let key_id = sqlx::query_scalar::<_, Option<Uuid>>(
            r#"
SELECT id
FROM admin_api_keys
WHERE key_hash = $1
LIMIT 1
            "#,
        )
        .bind(key_hash)
        .fetch_optional(&self.pool)
        .await?
        .flatten();

        if let Some(key_id) = key_id {
            sqlx::query("UPDATE admin_api_keys SET last_used_at = now() WHERE id = $1")
                .bind(key_id)
                .execute(&self.pool)
                .await?;
            return Ok(true);
        }

        Ok(false)
    }

    pub async fn cleanup_storage_cache(&self, max_age_seconds: Option<i64>) -> anyhow::Result<i64> {
        let cfg = self.config_snapshot();
        let root = Path::new(&cfg.storage.s3_cache_dir);
        if !root.exists() {
            return Ok(0);
        }

        let ttl = max_age_seconds
            .unwrap_or(cfg.storage.s3_cache_ttl_seconds)
            .max(0);
        let threshold = std::time::SystemTime::now()
            .checked_sub(std::time::Duration::from_secs(ttl as u64))
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

        let mut removed = 0_i64;
        for entry in std::fs::read_dir(root)? {
            let entry = match entry {
                Ok(v) => v,
                Err(_) => continue,
            };
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let modified = match entry.metadata().and_then(|m| m.modified()) {
                Ok(v) => v,
                Err(_) => continue,
            };

            if modified <= threshold {
                if std::fs::remove_file(&path).is_ok() {
                    removed += 1;
                }
            }
        }

        if removed > 0 {
            self.metrics
                .cache_cleanup_removed_total
                .fetch_add(removed as u64, Ordering::Relaxed);
        }

        Ok(removed)
    }

    pub async fn tmdb_cache_stats(&self) -> anyhow::Result<TmdbCacheStatsRow> {
        let row = sqlx::query_as::<_, TmdbCacheStatsRow>(
            r#"
SELECT
    COUNT(*)::bigint AS total_entries,
    COUNT(*) FILTER (WHERE has_result)::bigint AS entries_with_result,
    COUNT(*) FILTER (WHERE expires_at < now())::bigint AS expired_entries,
    COALESCE(SUM(hit_count), 0)::bigint AS total_hits
FROM tmdb_cache
            "#,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list_tmdb_failures(&self, limit: i64) -> anyhow::Result<Vec<TmdbFailureRow>> {
        let rows = sqlx::query_as::<_, TmdbFailureRow>(
            r#"
SELECT id, media_item_id, item_name, item_type, attempts, error, created_at
FROM tmdb_failures
ORDER BY created_at DESC
LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn clear_tmdb_cache(&self, expired_only: bool) -> anyhow::Result<i64> {
        let result = if expired_only {
            sqlx::query("DELETE FROM tmdb_cache WHERE expires_at < now()")
                .execute(&self.pool)
                .await?
        } else {
            sqlx::query("DELETE FROM tmdb_cache")
                .execute(&self.pool)
                .await?
        };
        Ok(result.rows_affected() as i64)
    }

    pub async fn clear_tmdb_failures(&self) -> anyhow::Result<i64> {
        let result = sqlx::query("DELETE FROM tmdb_failures")
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() as i64)
    }

    pub async fn invalidate_cached_stream(&self, stream_url: &str) -> anyhow::Result<bool> {
        let cfg = self.config_snapshot();
        let cache_root = Path::new(&cfg.storage.s3_cache_dir);
        if !cache_root.exists() {
            return Ok(false);
        }

        let file_name = format!("{}.cache", auth::hash_api_key(stream_url));
        let file_path = cache_root.join(file_name);
        if !file_path.exists() {
            return Ok(false);
        }

        std::fs::remove_file(file_path)?;
        Ok(true)
    }

}
