impl AppInfra {
    pub fn scraper_is_enabled(&self) -> bool {
        let cfg = self.config_snapshot();
        cfg.scraper.enabled || cfg.tmdb.enabled
    }

    fn scraper_policy_settings(&self) -> ScraperPolicySettings {
        let cfg = self.config_snapshot();
        let scraper = cfg.normalized_scraper_config();
        ScraperPolicySettings {
            default_strategy: scraper.default_strategy,
            default_routes: ls_scraper::ScraperDefaultRoutes {
                movie: scraper.default_routes.movie,
                series: scraper.default_routes.series,
                image: scraper.default_routes.image,
            },
        }
    }

    fn scraper_available_provider_ids(&self) -> Vec<String> {
        let cfg = self.config_snapshot();
        let scraper = cfg.normalized_scraper_config();
        ls_scraper::normalize_provider_chain(&scraper.providers)
    }

    fn parse_library_scraper_policy(value: &Value) -> Option<ScraperLibraryPolicy> {
        if !value.is_object() {
            return None;
        }
        serde_json::from_value(value.clone()).ok()
    }

    async fn build_scrape_plan(
        &self,
        library_id: Option<Uuid>,
        item_type: &str,
        force_image_refresh: bool,
    ) -> anyhow::Result<ScrapePlan> {
        let settings = self.scraper_policy_settings();
        let available = self.scraper_available_provider_ids();
        let scenario = infer_scenario_from_item_type(item_type);
        let purpose = if force_image_refresh {
            ScraperRoutePurpose::Image
        } else {
            ScraperRoutePurpose::Metadata
        };
        let library_policy = match library_id {
            Some(library_id) => self
                .get_library_by_id(library_id)
                .await?
                .and_then(|library| Self::parse_library_scraper_policy(&library.scraper_policy)),
            None => None,
        };

        Ok(resolve_provider_chain(
            &settings,
            library_policy.as_ref(),
            scenario,
            purpose,
            &available,
        ))
    }

    fn provider_enabled_in_registry(&self, provider_id: &str) -> bool {
        self.scraper_available_provider_ids()
            .iter()
            .any(|item| item.eq_ignore_ascii_case(provider_id))
    }

    fn tmdb_scraper_provider_status(&self) -> ScraperProviderStatus {
        let cfg = self.config_snapshot();
        let descriptor = TmdbScraperProvider;
        let enabled = self.scraper_is_enabled() && self.provider_enabled_in_registry("tmdb");
        let configured = !cfg.tmdb.api_key.trim().is_empty();
        build_scraper_provider_status(
            &descriptor,
            enabled,
            configured,
            if !enabled {
                "scraper disabled or tmdb not selected in provider chain"
            } else if !configured {
                "missing tmdb api key"
            } else {
                "ready"
            },
        )
    }

    fn tvdb_scraper_provider_status(&self) -> ScraperProviderStatus {
        let cfg = self.config_snapshot();
        let descriptor = TvdbScraperProvider;
        let enabled = self.scraper_is_enabled()
            && cfg.scraper.tvdb.enabled
            && self.provider_enabled_in_registry("tvdb");
        let configured = !cfg.scraper.tvdb.api_key.trim().is_empty();
        build_scraper_provider_status(
            &descriptor,
            enabled,
            configured,
            if !enabled {
                "scraper disabled, tvdb not enabled, or tvdb not selected in provider chain"
            } else if !configured {
                "missing tvdb api key"
            } else {
                "ready"
            },
        )
    }

    fn bangumi_scraper_provider_status(&self) -> ScraperProviderStatus {
        let cfg = self.config_snapshot();
        let descriptor = BangumiScraperProvider;
        let enabled = self.scraper_is_enabled()
            && cfg.scraper.bangumi.enabled
            && self.provider_enabled_in_registry("bangumi");
        let configured = !cfg.scraper.bangumi.access_token.trim().is_empty();
        build_scraper_provider_status(
            &descriptor,
            enabled,
            configured,
            if !enabled {
                "scraper disabled, bangumi not enabled, or bangumi not selected in provider chain"
            } else if !configured {
                "missing bangumi access token"
            } else {
                "ready"
            },
        )
    }

    pub async fn list_scraper_provider_statuses(
        &self,
    ) -> anyhow::Result<Vec<ScraperProviderStatus>> {
        Ok(vec![
            self.tmdb_scraper_provider_status(),
            self.tvdb_scraper_provider_status(),
            self.bangumi_scraper_provider_status(),
        ])
    }

    pub async fn test_scraper_provider(
        &self,
        provider_id: &str,
    ) -> anyhow::Result<Option<ScraperProviderStatus>> {
        let cfg = self.config_snapshot();
        match provider_id.to_ascii_lowercase().as_str() {
            "tmdb" => Ok(Some(self.tmdb_scraper_provider_status())),
            "tvdb" => {
                let mut status = self.tvdb_scraper_provider_status();
                if status.enabled && status.configured {
                    let provider_config = ls_scraper::TvdbConfig {
                        enabled: cfg.scraper.tvdb.enabled,
                        base_url: cfg.scraper.tvdb.base_url.clone(),
                        api_key: cfg.scraper.tvdb.api_key.clone(),
                        pin: cfg.scraper.tvdb.pin.clone(),
                        timeout_seconds: cfg.scraper.tvdb.timeout_seconds,
                    };
                    let client = TvdbClient::new(
                        &self.http_client,
                        &provider_config,
                    );
                    match client.health_check().await {
                        Ok(()) => {
                            status.healthy = true;
                            status.message = "ready".to_string();
                        }
                        Err(err) => {
                            status.healthy = false;
                            status.message = err.to_string();
                        }
                    }
                }
                Ok(Some(status))
            }
            "bangumi" => {
                let mut status = self.bangumi_scraper_provider_status();
                if status.enabled && status.configured {
                    let provider_config = ls_scraper::BangumiConfig {
                        enabled: cfg.scraper.bangumi.enabled,
                        base_url: cfg.scraper.bangumi.base_url.clone(),
                        access_token: cfg.scraper.bangumi.access_token.clone(),
                        timeout_seconds: cfg.scraper.bangumi.timeout_seconds,
                        user_agent: cfg.scraper.bangumi.user_agent.clone(),
                    };
                    let client = BangumiClient::new(
                        &self.http_client,
                        &provider_config,
                    );
                    match client.health_check().await {
                        Ok(()) => {
                            status.healthy = true;
                            status.message = "ready".to_string();
                        }
                        Err(err) => {
                            status.healthy = false;
                            status.message = err.to_string();
                        }
                    }
                }
                Ok(Some(status))
            }
            _ => Ok(None),
        }
    }

    async fn fill_metadata_with_scraper(
        &self,
        item: &TmdbFillItemRow,
        force_image_refresh: bool,
    ) -> anyhow::Result<TmdbFillStatus> {
        if !self.scraper_is_enabled() {
            return Ok(TmdbFillStatus::Skipped);
        }

        let plan = self
            .build_scrape_plan(item.library_id, &item.item_type, force_image_refresh)
            .await?;
        if plan.provider_chain.is_empty() {
            info!(item_id = %item.id, item_type = %item.item_type, "scraper plan resolved no providers; skip");
            return Ok(TmdbFillStatus::Skipped);
        }

        let mut current_item = item.clone();
        let mut primary_provider: Option<String> = None;
        let mut applied_sources = Vec::<String>::new();
        let mut had_error = false;
        let mut metadata_changed = false;

        for provider_id in &plan.provider_chain {
            match provider_id.as_str() {
                "tmdb" => {
                    let status = if primary_provider.is_none() && force_image_refresh {
                        self.fill_metadata_from_tmdb_with_options(&current_item, true)
                            .await?
                    } else {
                        self.fill_metadata_from_tmdb(&current_item).await?
                    };
                    match status {
                        TmdbFillStatus::Filled => {
                            if primary_provider.is_none() {
                                primary_provider = Some("tmdb".to_string());
                            }
                            push_unique_provider(&mut applied_sources, "tmdb");
                            metadata_changed = true;
                            if let Some(reloaded) = self.load_fill_item(current_item.id).await? {
                                current_item = reloaded;
                            }
                        }
                        TmdbFillStatus::Failed => had_error = true,
                        TmdbFillStatus::Skipped => {}
                    }
                }
                "tvdb" | "bangumi" => {
                    match self
                        .scrape_non_tmdb_provider(
                            provider_id,
                            &current_item,
                            primary_provider.is_none(),
                            force_image_refresh,
                        )
                        .await
                    {
                        Ok(Some(next_item)) => {
                            if primary_provider.is_none() {
                                primary_provider = Some(provider_id.clone());
                            }
                            push_unique_provider(&mut applied_sources, provider_id);
                            metadata_changed = true;
                            current_item = next_item;
                        }
                        Ok(None) => {}
                        Err(err) => {
                            had_error = true;
                            self.record_scraper_failure(&current_item, provider_id, &err.to_string())
                                .await?;
                            warn!(
                                error = %err,
                                provider_id,
                                item_id = %current_item.id,
                                "scraper provider failed"
                            );
                        }
                    }
                }
                other => {
                    warn!(provider_id = other, item_id = %current_item.id, "unknown scraper provider in chain");
                }
            }
        }

        if let Some(primary) = primary_provider.as_deref() {
            let normalized = finalize_scraper_metadata(
                current_item.metadata.clone(),
                primary,
                &applied_sources,
            );
            if normalized != current_item.metadata {
                self.persist_item_metadata(current_item.id, &normalized).await?;
                metadata_changed = true;
            }
        }

        if metadata_changed {
            Ok(TmdbFillStatus::Filled)
        } else if had_error {
            Ok(TmdbFillStatus::Failed)
        } else {
            Ok(TmdbFillStatus::Skipped)
        }
    }

    pub async fn rescrape_item_metadata(
        &self,
        item_id: Uuid,
        force_image_refresh: bool,
    ) -> anyhow::Result<bool> {
        let Some(item) = self.load_fill_item(item_id).await? else {
            return Ok(false);
        };
        let started_at = Utc::now();
        let _ = self
            .fill_metadata_with_scraper(&item, force_image_refresh)
            .await?;
        if let Err(err) = self.index_people_since(started_at).await {
            warn!(error = %err, item_id = %item.id, "failed to index people after item rescrape");
        }
        Ok(true)
    }

    async fn load_fill_item(&self, item_id: Uuid) -> anyhow::Result<Option<TmdbFillItemRow>> {
        sqlx::query_as::<_, TmdbFillItemRow>(
            r#"
SELECT id, library_id, name, item_type, path, season_number, episode_number, metadata
FROM media_items
WHERE id = $1
LIMIT 1
            "#,
        )
        .bind(item_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(Into::into)
    }

    async fn scrape_non_tmdb_provider(
        &self,
        provider_id: &str,
        item: &TmdbFillItemRow,
        as_primary: bool,
        force_image_refresh: bool,
    ) -> anyhow::Result<Option<TmdbFillItemRow>> {
        let result = match provider_id {
            "tvdb" => self.scrape_with_tvdb(item).await?,
            "bangumi" => self.scrape_with_bangumi(item).await?,
            _ => None,
        };

        let Some(result) = result else {
            return Ok(None);
        };
        let merged = apply_scrape_result_to_metadata(
            &item.metadata,
            &result,
            as_primary,
            provider_id,
        );
        if merged == item.metadata {
            return Ok(Some(TmdbFillItemRow {
                metadata: merged,
                ..item.clone()
            }));
        }

        self.persist_item_metadata(item.id, &merged).await?;
        let merged = self
            .persist_scraper_images(item, &merged, &result, force_image_refresh)
            .await?;
        self.write_scraper_nfo(item, &merged).await;
        self.update_item_title_from_metadata(item.id, item.item_type.as_str(), &merged)
            .await?;

        Ok(Some(TmdbFillItemRow {
            metadata: merged,
            ..item.clone()
        }))
    }

    async fn scrape_with_tvdb(&self, item: &TmdbFillItemRow) -> anyhow::Result<Option<ScrapeResult>> {
        let cfg = self.config_snapshot();
        if !cfg.scraper.tvdb.enabled || cfg.scraper.tvdb.api_key.trim().is_empty() {
            return Ok(None);
        }
        let provider_config = ls_scraper::TvdbConfig {
            enabled: cfg.scraper.tvdb.enabled,
            base_url: cfg.scraper.tvdb.base_url.clone(),
            api_key: cfg.scraper.tvdb.api_key.clone(),
            pin: cfg.scraper.tvdb.pin.clone(),
            timeout_seconds: cfg.scraper.tvdb.timeout_seconds,
        };
        let client = TvdbClient::new(&self.http_client, &provider_config);
        let query = scraper_query_title(item);
        let year = scraper_query_year(item);
        let scenario = infer_scenario_from_item_type(&item.item_type);
        let result = match scenario {
            ScraperScenario::MovieMetadata => client.scrape_movie_by_title(&query, year).await?,
            ScraperScenario::SeriesMetadata | ScraperScenario::SeasonMetadata => {
                client.scrape_series_by_title(&query, year).await?
            }
            ScraperScenario::EpisodeMetadata => {
                client
                    .scrape_episode_by_title(
                        &scraper_series_query_title(item),
                        item.season_number,
                        item.episode_number,
                        year,
                    )
                    .await?
            }
            _ => None,
        };
        Ok(result.map(|payload| payload.into_scrape_result(scenario)))
    }

    async fn scrape_with_bangumi(
        &self,
        item: &TmdbFillItemRow,
    ) -> anyhow::Result<Option<ScrapeResult>> {
        let cfg = self.config_snapshot();
        if !cfg.scraper.bangumi.enabled || cfg.scraper.bangumi.access_token.trim().is_empty() {
            return Ok(None);
        }
        let provider_config = ls_scraper::BangumiConfig {
            enabled: cfg.scraper.bangumi.enabled,
            base_url: cfg.scraper.bangumi.base_url.clone(),
            access_token: cfg.scraper.bangumi.access_token.clone(),
            timeout_seconds: cfg.scraper.bangumi.timeout_seconds,
            user_agent: cfg.scraper.bangumi.user_agent.clone(),
        };
        let client = BangumiClient::new(&self.http_client, &provider_config);
        let year = scraper_query_year(item);
        let scenario = infer_scenario_from_item_type(&item.item_type);
        let result = match scenario {
            ScraperScenario::MovieMetadata => None,
            ScraperScenario::SeriesMetadata | ScraperScenario::SeasonMetadata => {
                client
                    .scrape_series_by_title(&scraper_series_query_title(item), year)
                    .await?
            }
            ScraperScenario::EpisodeMetadata => {
                client
                    .scrape_episode_by_title(
                        &scraper_series_query_title(item),
                        item.season_number,
                        item.episode_number,
                        year,
                    )
                    .await?
            }
            _ => None,
        };
        Ok(result.map(|payload| payload.into_scrape_result(scenario)))
    }

    async fn persist_scraper_images(
        &self,
        item: &TmdbFillItemRow,
        metadata: &Value,
        result: &ScrapeResult,
        force_image_refresh: bool,
    ) -> anyhow::Result<Value> {
        let image_context = scraper_image_context(item);
        for image in &result.patch.images {
            let Some(url) = image
                .remote_path
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                continue;
            };
            let Some(target) = image_target_path(&image_context, image) else {
                continue;
            };
            if let Err(err) = self.ensure_remote_image(url, &target, force_image_refresh).await {
                warn!(error = %err, provider_id = %result.provider_id, path = %target.display(), "failed to download scraper image");
            }
        }

        let refreshed = refresh_image_tags_for_item(metadata, &image_context);
        if refreshed != *metadata {
            self.persist_item_metadata(item.id, &refreshed).await?;
        }
        Ok(refreshed)
    }

    async fn ensure_remote_image(
        &self,
        image_url: &str,
        target_path: &Path,
        force_overwrite: bool,
    ) -> anyhow::Result<Option<String>> {
        if image_url.trim().is_empty() {
            return Ok(None);
        }
        if target_path.exists() && !force_overwrite {
            return Ok(Some(target_path.to_string_lossy().to_string()));
        }

        let response = self
            .http_client
            .get(image_url)
            .timeout(std::time::Duration::from_secs(20))
            .send()
            .await
            .with_context(|| format!("failed to request scraper image: {image_url}"))?
            .error_for_status()
            .with_context(|| format!("scraper image returned non-success: {image_url}"))?;
        let bytes = response.bytes().await?;
        if let Some(parent) = target_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(target_path, &bytes).await?;
        Ok(Some(target_path.to_string_lossy().to_string()))
    }

    async fn write_scraper_nfo(&self, item: &TmdbFillItemRow, metadata: &Value) {
        let path = Path::new(&item.path);
        if item.item_type.eq_ignore_ascii_case("Movie") {
            let nfo_path = path.with_extension("nfo");
            if let Err(err) = write_movie_nfo(&nfo_path, metadata) {
                warn!(error = %err, path = %nfo_path.display(), "failed to write scraper movie nfo");
            }
            return;
        }

        if item.item_type.eq_ignore_ascii_case("Series") {
            let series_dir = Path::new(&item.path);
            let nfo_path = resolve_tvshow_nfo_path(series_dir);
            if let Err(err) = write_tvshow_nfo(&nfo_path, metadata) {
                warn!(error = %err, path = %nfo_path.display(), "failed to write scraper tvshow nfo");
            }
            return;
        }

        if item.item_type.eq_ignore_ascii_case("Episode") {
            let dir = path.parent().unwrap_or_else(|| Path::new("."));
            let series_dir = path
                .parent()
                .and_then(Path::parent)
                .or_else(|| path.parent())
                .unwrap_or_else(|| Path::new("."));
            let episode_nfo_path = path.with_extension("nfo");
            if let Err(err) = write_episode_nfo(
                &episode_nfo_path,
                metadata,
                item.season_number,
                item.episode_number,
            ) {
                warn!(error = %err, path = %episode_nfo_path.display(), "failed to write scraper episode nfo");
            }
            let tvshow_nfo_path = resolve_tvshow_nfo_path(series_dir);
            if let Err(err) = write_tvshow_nfo(&tvshow_nfo_path, metadata) {
                warn!(error = %err, path = %tvshow_nfo_path.display(), "failed to write scraper tvshow nfo");
            }
            if let Some(season) = item.season_number {
                let season_nfo_path = resolve_season_nfo_path(dir, season);
                if let Err(err) = write_season_nfo(&season_nfo_path, metadata, season) {
                    warn!(error = %err, path = %season_nfo_path.display(), "failed to write scraper season nfo");
                }
            }
        }
    }

    async fn update_item_title_from_metadata(
        &self,
        item_id: Uuid,
        item_type: &str,
        metadata: &Value,
    ) -> anyhow::Result<()> {
        let title = if item_type.eq_ignore_ascii_case("Series") {
            metadata
                .get("series_name")
                .or_else(|| metadata.get("sort_name"))
                .and_then(Value::as_str)
        } else {
            metadata
                .get("title")
                .or_else(|| metadata.get("sort_name"))
                .and_then(Value::as_str)
        }
        .map(str::trim)
        .filter(|value| !value.is_empty());
        if let Some(title) = title {
            self.update_item_name_and_search_keys(item_id, title).await?;
        }
        Ok(())
    }

    async fn record_scraper_failure(
        &self,
        item: &TmdbFillItemRow,
        provider_id: &str,
        error_message: &str,
    ) -> anyhow::Result<()> {
        let message = format!("[{provider_id}] {error_message}");
        self.record_tmdb_failure(item.id, &item.name, &item.item_type, 1, &message)
            .await?;

        sqlx::query(
            r#"
UPDATE system_events
SET event_type = 'scraper.fill.failed',
    source = $1,
    detail = jsonb_set(COALESCE(detail, '{}'::jsonb), '{provider_id}', to_jsonb($1::text), true)
WHERE id = (
    SELECT id
    FROM system_events
    WHERE event_type = 'tmdb.fill.failed'
    ORDER BY created_at DESC
    LIMIT 1
)
            "#,
        )
        .bind(provider_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn scraper_cache_stats(&self) -> anyhow::Result<TmdbCacheStatsRow> {
        self.tmdb_cache_stats().await
    }

    pub async fn list_scraper_failures(&self, limit: i64) -> anyhow::Result<Vec<TmdbFailureRow>> {
        self.list_tmdb_failures(limit).await
    }

    pub async fn clear_scraper_cache(&self, expired_only: bool) -> anyhow::Result<i64> {
        self.clear_tmdb_cache(expired_only).await
    }

    pub async fn clear_scraper_failures(&self) -> anyhow::Result<i64> {
        self.clear_tmdb_failures().await
    }
}

#[derive(Debug, Clone)]
struct ScraperImageContext {
    item_type: String,
    dir: PathBuf,
    stem: String,
    series_dir: PathBuf,
    series_stem: String,
}

fn build_scraper_provider_status(
    descriptor: &impl ScraperProviderDescriptor,
    enabled: bool,
    configured: bool,
    message: &str,
) -> ScraperProviderStatus {
    ScraperProviderStatus {
        provider_id: descriptor.provider_id().to_string(),
        display_name: descriptor.display_name().to_string(),
        provider_kind: descriptor.provider_kind().to_string(),
        enabled,
        configured,
        healthy: enabled && configured && message == "ready",
        capabilities: descriptor
            .capabilities()
            .into_iter()
            .map(|capability| capability.as_str().to_string())
            .collect(),
        scenarios: descriptor
            .scenarios()
            .into_iter()
            .map(|scenario| scenario.as_str().to_string())
            .collect(),
        message: message.to_string(),
        checked_at: Some(Utc::now()),
    }
}

fn scraper_query_title(item: &TmdbFillItemRow) -> String {
    if item.item_type.eq_ignore_ascii_case("Movie") {
        return build_movie_match_hints(item, Path::new(&item.path)).query_title;
    }
    scraper_series_query_title(item)
}

fn scraper_series_query_title(item: &TmdbFillItemRow) -> String {
    item.metadata
        .get("series_name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            item.metadata
                .get("nfo")
                .and_then(|value| value.get("showtitle").or_else(|| value.get("title")))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
        .unwrap_or_else(|| item.name.clone())
}

fn scraper_query_year(item: &TmdbFillItemRow) -> Option<i32> {
    item.metadata
        .get("production_year")
        .and_then(value_year_hint)
        .or_else(|| {
            item.metadata
                .get("nfo")
                .and_then(|value| value.get("year").or_else(|| value.get("aired")))
                .and_then(value_year_hint)
        })
}

fn apply_scrape_result_to_metadata(
    current: &Value,
    result: &ScrapeResult,
    as_primary: bool,
    provider_id: &str,
) -> Value {
    let sanitized_patch = strip_null_values(&result.patch.metadata);
    let mut next = if as_primary {
        merge_item_metadata_patch(current.clone(), &sanitized_patch)
    } else {
        merge_missing_json(current.clone(), &sanitized_patch)
    };
    merge_provider_ids(&mut next, &result.patch.provider_ids);
    merge_scraper_raw(&mut next, provider_id, &result.raw);
    merge_scraper_people(&mut next, sanitized_patch.get("people"), as_primary);
    next
}

fn finalize_scraper_metadata(mut metadata: Value, primary: &str, applied_sources: &[String]) -> Value {
    ensure_object(&mut metadata);
    let Some(object) = metadata.as_object_mut() else {
        return metadata;
    };
    object.insert(
        "scraper_sources".to_string(),
        json!({
            "primary": primary,
            "applied": applied_sources,
        }),
    );
    if object.get("scraper_raw").is_none()
        && let Some(tmdb_raw) = object.get("tmdb_raw").cloned()
    {
        object.insert("scraper_raw".to_string(), json!({ "tmdb": tmdb_raw }));
    }
    metadata
}

fn merge_provider_ids(metadata: &mut Value, overlay: &std::collections::BTreeMap<String, String>) {
    if overlay.is_empty() {
        return;
    }
    ensure_object(metadata);
    let Some(object) = metadata.as_object_mut() else {
        return;
    };
    let provider_ids = object
        .entry("provider_ids".to_string())
        .or_insert_with(|| json!({}));
    ensure_object(provider_ids);
    if let Some(map) = provider_ids.as_object_mut() {
        for (key, value) in overlay {
            map.entry(key.clone()).or_insert_with(|| Value::String(value.clone()));
        }
    }
}

fn merge_scraper_raw(metadata: &mut Value, provider_id: &str, raw: &Value) {
    ensure_object(metadata);
    let Some(object) = metadata.as_object_mut() else {
        return;
    };
    let scraper_raw = object
        .entry("scraper_raw".to_string())
        .or_insert_with(|| json!({}));
    ensure_object(scraper_raw);
    if let Some(map) = scraper_raw.as_object_mut() {
        map.insert(provider_id.to_string(), raw.clone());
    }
}

fn merge_scraper_people(metadata: &mut Value, people: Option<&Value>, as_primary: bool) {
    let Some(people) = people.and_then(Value::as_array) else {
        return;
    };
    if people.is_empty() {
        return;
    }
    ensure_object(metadata);
    let Some(object) = metadata.as_object_mut() else {
        return;
    };
    let existing = object
        .entry("people".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if as_primary || existing.as_array().is_none_or(|values| values.is_empty()) {
        *existing = Value::Array(people.clone());
        return;
    }

    let Some(existing_arr) = existing.as_array_mut() else {
        *existing = Value::Array(people.clone());
        return;
    };
    for person in people {
        let Some(key) = person_identity_key(person) else {
            continue;
        };
        if !existing_arr
            .iter()
            .filter_map(person_identity_key)
            .any(|existing_key| existing_key == key)
        {
            existing_arr.push(person.clone());
        }
    }
}

fn person_identity_key(person: &Value) -> Option<String> {
    person
        .get("id")
        .or_else(|| person.get("provider_person_id"))
        .and_then(Value::as_str)
        .map(str::to_ascii_lowercase)
        .or_else(|| {
            person
                .get("name")
                .and_then(Value::as_str)
                .map(str::to_ascii_lowercase)
        })
}

fn ensure_object(value: &mut Value) {
    if !value.is_object() {
        *value = json!({});
    }
}

fn strip_null_values(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut next = serde_json::Map::new();
            for (key, child) in map {
                let stripped = strip_null_values(child);
                if !stripped.is_null() {
                    next.insert(key.clone(), stripped);
                }
            }
            Value::Object(next)
        }
        Value::Array(items) => Value::Array(items.iter().map(strip_null_values).collect()),
        other => other.clone(),
    }
}

fn push_unique_provider(target: &mut Vec<String>, provider_id: &str) {
    if !target.iter().any(|item| item.eq_ignore_ascii_case(provider_id)) {
        target.push(provider_id.to_string());
    }
}

fn scraper_image_context(item: &TmdbFillItemRow) -> ScraperImageContext {
    let media_path = PathBuf::from(&item.path);
    if item.item_type.eq_ignore_ascii_case("Series") {
        let series_dir = media_path.clone();
        let series_stem = series_dir
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string();
        return ScraperImageContext {
            item_type: item.item_type.clone(),
            dir: series_dir.clone(),
            stem: series_stem.clone(),
            series_dir,
            series_stem,
        };
    }
    let dir = media_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let stem = media_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_string();
    let series_dir = media_path
        .parent()
        .and_then(Path::parent)
        .or_else(|| media_path.parent())
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let series_stem = series_dir
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_string();
    ScraperImageContext {
        item_type: item.item_type.clone(),
        dir,
        stem,
        series_dir,
        series_stem,
    }
}

fn image_target_path(context: &ScraperImageContext, image: &ImageAssetPatch) -> Option<PathBuf> {
    let extension = remote_image_extension(image.remote_path.as_deref()?)?;
    match image.image_type.to_ascii_lowercase().as_str() {
        "backdrop" => Some(context.series_dir.join(format!("fanart.{extension}"))),
        "logo" => Some(context.series_dir.join(format!("logo.{extension}"))),
        "thumb" => Some(
            context
                .dir
                .join(format!("{}-thumb.{extension}", context.stem)),
        ),
        _ => {
            let dir = if context.item_type.eq_ignore_ascii_case("Series") {
                &context.series_dir
            } else {
                &context.dir
            };
            let stem = if context.item_type.eq_ignore_ascii_case("Series") {
                &context.series_stem
            } else {
                &context.stem
            };
            Some(dir.join(format!("{stem}.{extension}")))
        }
    }
}

fn remote_image_extension(url: &str) -> Option<&'static str> {
    let lower = url.to_ascii_lowercase();
    if lower.contains(".png") {
        Some("png")
    } else if lower.contains(".webp") {
        Some("webp")
    } else if lower.contains(".jpeg") {
        Some("jpeg")
    } else {
        Some("jpg")
    }
}

fn refresh_image_tags_for_item(metadata: &Value, context: &ScraperImageContext) -> Value {
    let mut next = metadata.clone();
    let primary_dir = if context.item_type.eq_ignore_ascii_case("Series") {
        &context.series_dir
    } else {
        &context.dir
    };
    let primary_stem = if context.item_type.eq_ignore_ascii_case("Series") {
        &context.series_stem
    } else {
        &context.stem
    };
    let primary_image_tag = image_candidates(primary_dir, primary_stem, "primary")
        .first()
        .and_then(|path| image_file_tag(path));
    let backdrop_image_tags = image_candidates(&context.series_dir, &context.series_stem, "backdrop")
        .first()
        .and_then(|path| image_file_tag(path))
        .map(|tag| vec![tag]);
    let logo_image_tag = image_candidates(&context.series_dir, &context.series_stem, "logo")
        .first()
        .and_then(|path| image_file_tag(path));
    let thumb_image_tag = image_candidates(&context.dir, &context.stem, "thumb")
        .first()
        .and_then(|path| image_file_tag(path));

    ensure_object(&mut next);
    if let Some(object) = next.as_object_mut() {
        if context.item_type.eq_ignore_ascii_case("Episode") {
            object.insert("primary_image_tag".to_string(), json!(thumb_image_tag));
        } else {
            object.insert("primary_image_tag".to_string(), json!(primary_image_tag));
        }
        object.insert("backdrop_image_tags".to_string(), json!(backdrop_image_tags));
        object.insert("logo_image_tag".to_string(), json!(logo_image_tag));
    }
    next
}
