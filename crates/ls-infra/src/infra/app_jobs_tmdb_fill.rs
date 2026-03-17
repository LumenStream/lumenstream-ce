impl AppInfra {
    pub async fn rescrape_item_metadata_from_tmdb(
        &self,
        item_id: Uuid,
        force_image_refresh: bool,
    ) -> anyhow::Result<bool> {
        let Some(item) = sqlx::query_as::<_, TmdbFillItemRow>(
            r#"
SELECT id, library_id, name, item_type, path, season_number, episode_number, metadata
FROM media_items
WHERE id = $1
LIMIT 1
            "#,
        )
        .bind(item_id)
        .fetch_optional(&self.pool)
        .await?
        else {
            return Ok(false);
        };

        let started_at = Utc::now();
        let _ = self
            .fill_metadata_from_tmdb_with_options(&item, force_image_refresh)
            .await?;
        if let Err(err) = self.index_people_since(started_at).await {
            warn!(error = %err, item_id = %item.id, "failed to index people after item rescrape");
        }
        Ok(true)
    }

    async fn fill_metadata_from_tmdb(
        &self,
        item: &TmdbFillItemRow,
    ) -> anyhow::Result<TmdbFillStatus> {
        self.fill_metadata_from_tmdb_with_options(item, false).await
    }

    async fn fill_metadata_from_tmdb_with_options(
        &self,
        item: &TmdbFillItemRow,
        force_image_refresh: bool,
    ) -> anyhow::Result<TmdbFillStatus> {
        if !self.scraper_is_enabled() || self.config_snapshot().tmdb.api_key.trim().is_empty() {
            return Ok(TmdbFillStatus::Skipped);
        }

        let fill_result = if item.item_type.eq_ignore_ascii_case("Episode") {
            self.fill_episode_from_tmdb(item, force_image_refresh).await
        } else if item.item_type.eq_ignore_ascii_case("Movie") {
            self.fill_movie_from_tmdb(item, force_image_refresh).await
        } else if item.item_type.eq_ignore_ascii_case("Series") {
            self.fill_series_from_tmdb(item, force_image_refresh).await
        } else {
            Ok(false)
        };

        match fill_result {
            Ok(filled) => {
                if filled {
                    self.metrics
                        .tmdb_success_total
                        .fetch_add(1, Ordering::Relaxed);
                    Ok(TmdbFillStatus::Filled)
                } else {
                    Ok(TmdbFillStatus::Skipped)
                }
            }
            Err(err) => {
                self.metrics
                    .tmdb_failure_total
                    .fetch_add(1, Ordering::Relaxed);
                self.record_tmdb_failure(
                    item.id,
                    &item.name,
                    &item.item_type,
                    self.config_snapshot().tmdb.retry_attempts.max(1) as i32,
                    &err.to_string(),
                )
                .await?;
                Ok(TmdbFillStatus::Failed)
            }
        }
    }

    async fn fill_movie_from_tmdb(
        &self,
        item: &TmdbFillItemRow,
        force_image_refresh: bool,
    ) -> anyhow::Result<bool> {
        let media_path = Path::new(&item.path);
        let metadata_tmdb_id = item.metadata.get("tmdb_id").and_then(Value::as_i64);
        let tmdb_binding_manual = movie_tmdb_binding_is_manual(&item.metadata);
        let nfo_tmdb_id = resolve_movie_nfo_tmdb_id(media_path);
        let hints = build_movie_match_hints(item, media_path);

        let mut candidates = Vec::<(i64, &'static str)>::new();
        if let Some(movie_id) = metadata_tmdb_id {
            candidates.push((movie_id, "metadata_tmdb_id"));
        }
        if let Some(movie_id) = nfo_tmdb_id {
            if !candidates.iter().any(|(id, _)| *id == movie_id) {
                candidates.push((movie_id, "nfo_tmdbid"));
            }
        }
        if let Some(imdb_id) = resolve_movie_nfo_imdb_id(media_path) {
            if let Some(movie_id) = self.tmdb_find_by_imdb(&imdb_id, "movie").await? {
                if !candidates.iter().any(|(id, _)| *id == movie_id) {
                    candidates.push((movie_id, "nfo_imdbid"));
                }
            }
        }

        let mut selected: Option<(
            i64,
            Value,
            &'static str,
            MovieMatchAssessment,
            Option<String>,
            Option<i32>,
        )> = None;
        let mut had_conflict = false;

        if tmdb_binding_manual && let Some(movie_id) = metadata_tmdb_id {
            let Some(candidate_details) = self.tmdb_fetch_movie_details(movie_id).await? else {
                warn!(
                    item_id = %item.id,
                    movie_id,
                    path = %item.path,
                    "manual tmdb binding points to a missing movie; skip automatic rematch"
                );
                return Ok(false);
            };
            let release_dates = self.tmdb_fetch_movie_release_dates(movie_id).await?;
            let premiere_date =
                tmdb_movie_premiere_date(&candidate_details, release_dates.as_ref());
            let canonical_year = tmdb_movie_release_year(&candidate_details, release_dates.as_ref());
            let assessment = assess_movie_match(&hints, &candidate_details, canonical_year);
            if !assessment.confident(&hints) {
                info!(
                    item_id = %item.id,
                    movie_id,
                    title_exact = assessment.title_exact,
                    title_partial = assessment.title_partial,
                    year_gap = ?assessment.year_gap,
                    query_title = %hints.query_title,
                    query_year = ?hints.year,
                    "manual tmdb binding bypasses local title confidence checks"
                );
            }
            selected = Some((
                movie_id,
                candidate_details,
                "manual_tmdb_id",
                assessment,
                premiere_date,
                canonical_year,
            ));
        }

        if selected.is_none() {
            for (candidate_id, source) in candidates {
                let Some(candidate_details) = self.tmdb_fetch_movie_details(candidate_id).await? else {
                    continue;
                };
                let release_dates = self.tmdb_fetch_movie_release_dates(candidate_id).await?;
                let premiere_date =
                    tmdb_movie_premiere_date(&candidate_details, release_dates.as_ref());
                let canonical_year = tmdb_movie_release_year(&candidate_details, release_dates.as_ref());
                let assessment = assess_movie_match(&hints, &candidate_details, canonical_year);
                if assessment.confident(&hints) {
                    selected = Some((
                        candidate_id,
                        candidate_details,
                        source,
                        assessment,
                        premiere_date,
                        canonical_year,
                    ));
                    break;
                }
                had_conflict = true;
                warn!(
                    item_id = %item.id,
                    candidate_id,
                    source,
                    title_exact = assessment.title_exact,
                    title_partial = assessment.title_partial,
                    year_gap = ?assessment.year_gap,
                    query_title = %hints.query_title,
                    query_year = ?hints.year,
                    "tmdb candidate conflicts with local hints; trying scored search fallback"
                );
            }
        }

        if selected.is_none() {
            let search_results = self.tmdb_search_movie_results(&hints.query_title, hints.year).await?;
            for (candidate_id, preliminary_score, preliminary_assessment) in
                rank_movie_search_candidates(&hints, &search_results)
                    .into_iter()
                    .take(6)
            {
                let Some(candidate_details) = self.tmdb_fetch_movie_details(candidate_id).await? else {
                    continue;
                };
                let release_dates = self.tmdb_fetch_movie_release_dates(candidate_id).await?;
                let premiere_date =
                    tmdb_movie_premiere_date(&candidate_details, release_dates.as_ref());
                let canonical_year =
                    tmdb_movie_release_year(&candidate_details, release_dates.as_ref());
                let assessment = assess_movie_match(&hints, &candidate_details, canonical_year);
                let score = score_movie_candidate(&candidate_details, assessment);
                if !assessment.confident(&hints) || score < 100 {
                    continue;
                }
                info!(
                    item_id = %item.id,
                    movie_id = candidate_id,
                    score,
                    preliminary_score,
                    preliminary_title_exact = preliminary_assessment.title_exact,
                    preliminary_title_partial = preliminary_assessment.title_partial,
                    preliminary_year_gap = ?preliminary_assessment.year_gap,
                    title_exact = assessment.title_exact,
                    title_partial = assessment.title_partial,
                    year_gap = ?assessment.year_gap,
                    query_title = %hints.query_title,
                    query_year = ?hints.year,
                    "selected tmdb movie candidate via scored search"
                );
                selected = Some((
                    candidate_id,
                    candidate_details,
                    "scored_search",
                    assessment,
                    premiere_date,
                    canonical_year,
                ));
                break;
            }
        }

        let Some((movie_id, details, source, assessment, release_date, production_year)) = selected else {
            if had_conflict {
                warn!(
                    item_id = %item.id,
                    path = %item.path,
                    query_title = %hints.query_title,
                    query_year = ?hints.year,
                    "unable to find a confident TMDB match; skip update to avoid wrong binding"
                );
            }
            return Ok(false);
        };

        let corrected_binding = metadata_tmdb_id.is_some_and(|id| id != movie_id)
            || nfo_tmdb_id.is_some_and(|id| id != movie_id);
        if corrected_binding {
            info!(
                item_id = %item.id,
                old_tmdb_id = ?metadata_tmdb_id.or(nfo_tmdb_id),
                new_tmdb_id = movie_id,
                source,
                title_exact = assessment.title_exact,
                year_gap = ?assessment.year_gap,
                "repaired movie tmdb binding"
            );
        }
        let movie_release_dates = self.tmdb_fetch_movie_release_dates(movie_id).await?;
        let official_rating = tmdb_movie_official_rating(
            movie_release_dates.as_ref(),
            &self.config_snapshot().tmdb.language,
        );
        let movie_keyword_tags = self
            .tmdb_get_json_opt(&format!("{TMDB_API_BASE}/movie/{movie_id}/keywords"))
            .await?
            .map(|payload| extract_tmdb_keywords(&payload))
            .unwrap_or_default();

        let media_path = Path::new(&item.path);
        let dir = media_path.parent().unwrap_or_else(|| Path::new("."));
        let stem = media_path
            .file_stem()
            .and_then(|v| v.to_str())
            .unwrap_or_default();
        let people_candidates = extract_tmdb_people(details.get("credits"), TMDB_TOP_CAST_LIMIT);
        let people_json = self
            .upsert_people_for_item(item.id, &people_candidates)
            .await?;

        let poster_path = details
            .get("poster_path")
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
            .map(str::to_string);
        let backdrop_path = details
            .get("backdrop_path")
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
            .map(str::to_string);
        let logo_path = select_tmdb_logo_path(&details, &self.config_snapshot().tmdb.language);
        if force_image_refresh {
            remove_image_candidates(dir, stem, "primary");
            remove_image_candidates(dir, stem, "backdrop");
            remove_image_candidates(dir, stem, "logo");
        }
        if force_image_refresh || image_candidates(dir, stem, "primary").is_empty() {
            if let Some(poster) = poster_path.as_deref() {
                let target = dir.join(format!("{stem}.jpg"));
                if let Err(err) = self
                    .ensure_tmdb_image(poster, &target, force_image_refresh)
                    .await
                {
                    warn!(error = %err, path = %target.display(), "failed to download movie poster");
                }
            }
        }
        if force_image_refresh || image_candidates(dir, stem, "backdrop").is_empty() {
            if let Some(backdrop) = backdrop_path.as_deref() {
                let target = dir.join("fanart.jpg");
                if let Err(err) = self
                    .ensure_tmdb_image(backdrop, &target, force_image_refresh)
                    .await
                {
                    warn!(error = %err, path = %target.display(), "failed to download movie backdrop");
                }
            }
        }
        if force_image_refresh || image_candidates(dir, stem, "logo").is_empty() {
            if let Some(logo) = logo_path.as_deref() {
                let target = dir.join(format!("logo.{}", tmdb_logo_extension(logo)));
                if let Err(err) = self
                    .ensure_tmdb_image(logo, &target, force_image_refresh)
                    .await
                {
                    warn!(error = %err, path = %target.display(), "failed to download movie logo");
                }
            }
        }

        let primary_image_tag = image_candidates(dir, stem, "primary")
            .first()
            .and_then(|path| image_file_tag(path));
        let backdrop_image_tags = image_candidates(dir, stem, "backdrop")
            .first()
            .and_then(|path| image_file_tag(path))
            .map(|tag| vec![tag]);
        let logo_image_tag = image_candidates(dir, stem, "logo")
            .first()
            .and_then(|path| image_file_tag(path));

        let patch = json!({
            "tmdb_id": movie_id,
            "provider_ids": { "Tmdb": movie_id.to_string() },
            "overview": details.get("overview").and_then(Value::as_str),
            "premiere_date": release_date,
            "production_year": production_year,
            "genres": extract_tmdb_genres(&details),
            "tags": movie_keyword_tags.clone(),
            "studios": extract_tmdb_studios(&details),
            "official_rating": official_rating.clone(),
            "community_rating": details.get("vote_average").and_then(Value::as_f64),
            "sort_name": details
                .get("title")
                .or_else(|| details.get("name"))
                .and_then(Value::as_str),
            "nfo": {
                "title": details.get("title").or_else(|| details.get("name")).and_then(Value::as_str),
                "tmdb_id": movie_id.to_string(),
                "official_rating": official_rating.clone(),
                "mpaa": official_rating,
            },
            "people": people_json,
            "primary_image_tag": primary_image_tag,
            "backdrop_image_tags": backdrop_image_tags,
            "logo_image_tag": logo_image_tag,
            "tmdb_raw": details,
        });

        let mut merged = merge_missing_json(item.metadata.clone(), &patch);
        if corrected_binding {
            let mut rebind = serde_json::Map::<String, Value>::new();
            rebind.insert("tmdb_id".to_string(), json!(movie_id));
            rebind.insert(
                "provider_ids".to_string(),
                json!({ "Tmdb": movie_id.to_string() }),
            );
            rebind.insert(
                "nfo".to_string(),
                json!({ "tmdb_id": movie_id.to_string() }),
            );
            for key in [
                "overview",
                "premiere_date",
                "production_year",
                "genres",
                "studios",
                "official_rating",
                "community_rating",
                "sort_name",
                "people",
                "primary_image_tag",
                "backdrop_image_tags",
                "logo_image_tag",
                "tmdb_raw",
            ] {
                if let Some(value) = patch.get(key) {
                    if !is_missing_value(value) {
                        rebind.insert(key.to_string(), value.clone());
                    }
                }
            }
            merged = merge_item_metadata_patch(merged, &Value::Object(rebind));
        }
        merge_tmdb_tags_into_metadata(&mut merged, &movie_keyword_tags);
        let changed = merged != item.metadata;
        if changed {
            self.persist_item_metadata(item.id, &merged).await?;
        }

        let movie_nfo_path = media_path.with_extension("nfo");
        if let Err(err) = write_movie_nfo(&movie_nfo_path, &merged) {
            warn!(error = %err, path = %movie_nfo_path.display(), "failed to write movie nfo");
        }

        if let Some(title) = merged.get("sort_name").and_then(Value::as_str)
            .map(str::trim).filter(|v| !v.is_empty())
        {
            self.update_item_name_and_search_keys(item.id, title).await?;
        }

        Ok(changed || !people_candidates.is_empty())
    }

    async fn tmdb_fetch_movie_details(&self, movie_id: i64) -> anyhow::Result<Option<Value>> {
        let details_endpoint = format!(
            "{TMDB_API_BASE}/movie/{movie_id}?language={lang}&append_to_response=credits,images&include_image_language={include_image_language}",
            lang = urlencoding::encode(&self.config_snapshot().tmdb.language),
            include_image_language = tmdb_include_image_language(&self.config_snapshot().tmdb.language),
        );
        self.tmdb_get_json_opt(&details_endpoint).await
    }

    async fn tmdb_fetch_movie_release_dates(&self, movie_id: i64) -> anyhow::Result<Option<Value>> {
        let endpoint = format!("{TMDB_API_BASE}/movie/{movie_id}/release_dates");
        self.tmdb_get_json_opt(&endpoint).await
    }

    async fn tmdb_search_movie_results(
        &self,
        query_title: &str,
        year: Option<i32>,
    ) -> anyhow::Result<Vec<Value>> {
        let cleaned = search::normalize_media_title(query_title);
        let query = if cleaned.trim().is_empty() {
            query_title.trim().to_string()
        } else {
            cleaned
        };
        if query.is_empty() {
            return Ok(Vec::new());
        }

        let mut endpoint = format!(
            "{TMDB_API_BASE}/search/movie?query={query}&language={lang}",
            query = urlencoding::encode(&query),
            lang = urlencoding::encode(&self.config_snapshot().tmdb.language),
        );
        if let Some(year) = year {
            endpoint.push_str(&format!("&year={year}"));
        }
        let payload = self.tmdb_get_json(&endpoint).await?;
        let mut results = payload
            .get("results")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if results.is_empty() && year.is_some() {
            let endpoint = format!(
                "{TMDB_API_BASE}/search/movie?query={query}&language={lang}",
                query = urlencoding::encode(&query),
                lang = urlencoding::encode(&self.config_snapshot().tmdb.language),
            );
            let payload = self.tmdb_get_json(&endpoint).await?;
            results = payload
                .get("results")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
        }
        Ok(results)
    }

    async fn fill_series_from_tmdb(
        &self,
        item: &TmdbFillItemRow,
        force_image_refresh: bool,
    ) -> anyhow::Result<bool> {
        let item_path = Path::new(&item.path);
        let series_dir = if item_path.is_dir() {
            item_path
        } else {
            item_path.parent().unwrap_or_else(|| Path::new("."))
        };
        let stem = series_dir
            .file_name()
            .and_then(|v| v.to_str())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| {
                item_path
                    .file_stem()
                    .and_then(|v| v.to_str())
                    .filter(|v| !v.is_empty())
                    .unwrap_or_default()
            });

        let series_name = item
            .metadata
            .get("series_name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string)
            .or_else(|| {
                if item.name.trim().is_empty() {
                    None
                } else {
                    Some(item.name.trim().to_string())
                }
            })
            .or_else(|| {
                series_dir
                    .file_name()
                    .and_then(|v| v.to_str())
                    .map(str::to_string)
            })
            .unwrap_or_else(|| item.name.clone());

        // Priority 1: existing metadata tmdb_id / series_tmdb_id
        let mut tv_id = item.metadata.get("series_tmdb_id").and_then(Value::as_i64)
            .or_else(|| item.metadata.get("tmdb_id").and_then(Value::as_i64));

        // Priority 2: NFO file <tmdbid>
        if tv_id.is_none() {
            tv_id = resolve_series_nfo_tmdb_id(series_dir);
            if tv_id.is_some() {
                info!(tmdb_id = ?tv_id, path = %item.path, "resolved series tmdb_id from nfo");
            }
        }

        // Priority 3: NFO <imdbid> → TMDB find API
        if tv_id.is_none() {
            if let Some(imdb_id) = resolve_series_nfo_imdb_id(series_dir) {
                tv_id = self.tmdb_find_by_imdb(&imdb_id, "tv").await?;
            }
        }

        // Priority 4: TMDB search by name (original fallback)
        if tv_id.is_none() {
            let Some(tv_first) = self.tmdb_search_first("tv", &series_name).await? else {
                return Ok(false);
            };
            tv_id = tv_first.get("id").and_then(Value::as_i64);
        }

        let Some(tv_id) = tv_id else {
            return Ok(false);
        };

        let tv_endpoint = format!(
            "{TMDB_API_BASE}/tv/{tv_id}?language={lang}&append_to_response=credits,images,content_ratings&include_image_language={include_image_language}",
            lang = urlencoding::encode(&self.config_snapshot().tmdb.language),
            include_image_language = tmdb_include_image_language(&self.config_snapshot().tmdb.language),
        );
        let Some(tv_details) = self.tmdb_get_json_opt(&tv_endpoint).await? else {
            return Ok(false);
        };
        let official_rating = tmdb_tv_official_rating(
            tv_details.get("content_ratings"),
            &self.config_snapshot().tmdb.language,
        );
        let series_keyword_tags = self
            .tmdb_get_json_opt(&format!("{TMDB_API_BASE}/tv/{tv_id}/keywords"))
            .await?
            .map(|payload| extract_tmdb_keywords(&payload))
            .unwrap_or_default();
        let logo_path = select_tmdb_logo_path(&tv_details, &self.config_snapshot().tmdb.language);

        let people_candidates = extract_tmdb_people(tv_details.get("credits"), TMDB_TOP_CAST_LIMIT);
        let people_json = self
            .upsert_people_for_item(item.id, &people_candidates)
            .await?;

        if force_image_refresh {
            remove_image_candidates(series_dir, stem, "primary");
            remove_image_candidates(series_dir, stem, "backdrop");
            remove_image_candidates(series_dir, stem, "logo");
        }
        if force_image_refresh || image_candidates(series_dir, stem, "primary").is_empty() {
            if let Some(poster) = tv_details.get("poster_path").and_then(Value::as_str) {
                let target = series_dir.join("poster.jpg");
                if let Err(err) = self
                    .ensure_tmdb_image(poster, &target, force_image_refresh)
                    .await
                {
                    warn!(error = %err, path = %target.display(), "failed to download series poster");
                }
            }
        }
        if force_image_refresh || image_candidates(series_dir, stem, "backdrop").is_empty() {
            if let Some(backdrop) = tv_details.get("backdrop_path").and_then(Value::as_str) {
                let target = series_dir.join("fanart.jpg");
                if let Err(err) = self
                    .ensure_tmdb_image(backdrop, &target, force_image_refresh)
                    .await
                {
                    warn!(error = %err, path = %target.display(), "failed to download series backdrop");
                }
            }
        }
        if force_image_refresh || image_candidates(series_dir, stem, "logo").is_empty() {
            if let Some(logo) = logo_path.as_deref() {
                let target = series_dir.join(format!("logo.{}", tmdb_logo_extension(logo)));
                if let Err(err) = self
                    .ensure_tmdb_image(logo, &target, force_image_refresh)
                    .await
                {
                    warn!(error = %err, path = %target.display(), "failed to download series logo");
                }
            }
        }

        let primary_image_tag = image_candidates(series_dir, stem, "primary")
            .first()
            .and_then(|path| image_file_tag(path));
        let backdrop_image_tags = image_candidates(series_dir, stem, "backdrop")
            .first()
            .and_then(|path| image_file_tag(path))
            .map(|tag| vec![tag]);
        let logo_image_tag = image_candidates(series_dir, stem, "logo")
            .first()
            .and_then(|path| image_file_tag(path));
        let premiere_date = tv_details
            .get("first_air_date")
            .and_then(Value::as_str)
            .map(str::to_string);
        let production_year = premiere_date.as_deref().and_then(parse_year_from_date);

        let patch = json!({
            "tmdb_id": tv_id,
            "series_tmdb_id": tv_id,
            "provider_ids": { "Tmdb": tv_id.to_string() },
            "series_name": tv_details.get("name").and_then(Value::as_str),
            "overview": tv_details.get("overview").and_then(Value::as_str),
            "premiere_date": premiere_date,
            "production_year": production_year,
            "genres": extract_tmdb_genres(&tv_details),
            "tags": series_keyword_tags.clone(),
            "studios": extract_tmdb_studios(&tv_details),
            "official_rating": official_rating.clone(),
            "community_rating": tv_details.get("vote_average").and_then(Value::as_f64),
            "sort_name": tv_details.get("name").and_then(Value::as_str),
            "nfo": {
                "title": tv_details.get("name").and_then(Value::as_str),
                "tmdb_id": tv_id.to_string(),
                "official_rating": official_rating.clone(),
                "mpaa": official_rating,
            },
            "people": people_json,
            "primary_image_tag": primary_image_tag,
            "backdrop_image_tags": backdrop_image_tags,
            "logo_image_tag": logo_image_tag,
            "tmdb_raw": tv_details,
        });

        let mut merged = merge_missing_json(item.metadata.clone(), &patch);
        merge_tmdb_tags_into_metadata(&mut merged, &series_keyword_tags);
        let changed = merged != item.metadata;
        if changed {
            self.persist_item_metadata(item.id, &merged).await?;
        }

        let tvshow_nfo_path = resolve_tvshow_nfo_path(series_dir);
        if let Err(err) = write_tvshow_nfo(&tvshow_nfo_path, &merged) {
            warn!(error = %err, path = %tvshow_nfo_path.display(), "failed to write series tvshow nfo");
        }

        if let Some(title) = merged.get("series_name").and_then(Value::as_str)
            .map(str::trim).filter(|v| !v.is_empty())
        {
            self.update_item_name_and_search_keys(item.id, title).await?;
        }

        Ok(changed || !people_candidates.is_empty())
    }

    async fn fill_episode_from_tmdb(
        &self,
        item: &TmdbFillItemRow,
        force_image_refresh: bool,
    ) -> anyhow::Result<bool> {
        let media_path = Path::new(&item.path);
        let dir = media_path.parent().unwrap_or_else(|| Path::new("."));
        let stem = media_path
            .file_stem()
            .and_then(|v| v.to_str())
            .unwrap_or_default();
        let series_dir = media_path
            .parent()
            .and_then(Path::parent)
            .or_else(|| media_path.parent())
            .unwrap_or_else(|| Path::new("."));
        let series_stem = series_dir
            .file_name()
            .and_then(|v| v.to_str())
            .filter(|v| !v.is_empty())
            .unwrap_or_default();

        // Priority 1: existing metadata series_tmdb_id
        let mut tv_id = item.metadata.get("series_tmdb_id").and_then(Value::as_i64);

        // Priority 2: series directory NFO <tmdbid>
        if tv_id.is_none() {
            tv_id = resolve_series_nfo_tmdb_id(series_dir);
            if tv_id.is_some() {
                info!(tmdb_id = ?tv_id, path = %media_path.display(), "resolved episode series tmdb_id from nfo");
            }
        }

        // Priority 3: series directory NFO <imdbid> → TMDB find API
        if tv_id.is_none() {
            if let Some(imdb_id) = resolve_series_nfo_imdb_id(series_dir) {
                tv_id = self.tmdb_find_by_imdb(&imdb_id, "tv").await?;
            }
        }

        // Priority 4: TMDB search by series name (original fallback)
        if tv_id.is_none() {
            let series_name = item
                .metadata
                .get("series_name")
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| {
                    series_dir
                        .file_name()
                        .and_then(|v| v.to_str())
                        .map(str::to_string)
                })
                .unwrap_or_else(|| item.name.clone());
            let Some(tv_first) = self.tmdb_search_first("tv", &series_name).await? else {
                return Ok(false);
            };
            tv_id = tv_first.get("id").and_then(Value::as_i64);
        }

        let Some(tv_id) = tv_id else {
            return Ok(false);
        };

        let tv_endpoint = format!(
            "{TMDB_API_BASE}/tv/{tv_id}?language={lang}&append_to_response=credits,images,content_ratings",
            lang = urlencoding::encode(&self.config_snapshot().tmdb.language)
        );
        let tv_details = self.tmdb_get_json(&tv_endpoint).await?;
        let episode_keyword_tags = self
            .tmdb_get_json_opt(&format!("{TMDB_API_BASE}/tv/{tv_id}/keywords"))
            .await?
            .map(|payload| extract_tmdb_keywords(&payload))
            .unwrap_or_default();
        let season_number = item
            .season_number
            .or_else(|| infer_season_episode_from_path(media_path).map(|(s, _)| s));
        let episode_number = item
            .episode_number
            .or_else(|| infer_season_episode_from_path(media_path).map(|(_, e)| e));

        let season_details = if let Some(season) = season_number {
            let endpoint = format!(
                "{TMDB_API_BASE}/tv/{tv_id}/season/{season}?language={lang}&append_to_response=credits,images",
                lang = urlencoding::encode(&self.config_snapshot().tmdb.language)
            );
            self.tmdb_get_json_opt(&endpoint).await?
        } else {
            None
        };
        let episode_details = if let (Some(season), Some(episode)) = (season_number, episode_number)
        {
            let endpoint = format!(
                "{TMDB_API_BASE}/tv/{tv_id}/season/{season}/episode/{episode}?language={lang}&append_to_response=credits,images",
                lang = urlencoding::encode(&self.config_snapshot().tmdb.language)
            );
            self.tmdb_get_json_opt(&endpoint).await?
        } else {
            None
        };
        let official_rating = episode_details
            .as_ref()
            .and_then(|value| value.get("content_rating"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .or_else(|| {
                tmdb_tv_official_rating(
                    tv_details.get("content_ratings"),
                    &self.config_snapshot().tmdb.language,
                )
            });

        let credits_source = episode_details
            .as_ref()
            .and_then(|v| v.get("credits"))
            .or_else(|| tv_details.get("credits"));
        let people_candidates = extract_tmdb_people(credits_source, TMDB_TOP_CAST_LIMIT);
        let people_json = self
            .upsert_people_for_item(item.id, &people_candidates)
            .await?;

        if force_image_refresh {
            remove_image_candidates(dir, stem, "thumb");
            remove_image_candidates(series_dir, series_stem, "primary");
            remove_image_candidates(series_dir, series_stem, "backdrop");
        }
        if force_image_refresh || image_candidates(dir, stem, "thumb").is_empty() {
            if let Some(still) = episode_details
                .as_ref()
                .and_then(|v| v.get("still_path"))
                .and_then(Value::as_str)
                .filter(|v| !v.is_empty())
            {
                let target = dir.join(format!("{stem}-thumb.jpg"));
                if let Err(err) = self
                    .ensure_tmdb_image(still, &target, force_image_refresh)
                    .await
                {
                    warn!(error = %err, path = %target.display(), "failed to download episode still");
                }
            }
        }
        let series_poster = series_dir.join("poster.jpg");
        if force_image_refresh || !series_poster.exists() {
            if let Some(poster) = tv_details.get("poster_path").and_then(Value::as_str) {
                if let Err(err) = self
                    .ensure_tmdb_image(poster, &series_poster, force_image_refresh)
                    .await
                {
                    warn!(error = %err, path = %series_poster.display(), "failed to download series poster");
                }
            }
        }
        let series_backdrop = series_dir.join("fanart.jpg");
        if force_image_refresh || !series_backdrop.exists() {
            if let Some(backdrop) = tv_details.get("backdrop_path").and_then(Value::as_str) {
                if let Err(err) = self
                    .ensure_tmdb_image(backdrop, &series_backdrop, force_image_refresh)
                    .await
                {
                    warn!(error = %err, path = %series_backdrop.display(), "failed to download series backdrop");
                }
            }
        }
        if let (Some(season), Some(season_payload)) = (season_number, season_details.as_ref()) {
            if let Some(season_poster) = season_payload.get("poster_path").and_then(Value::as_str) {
                let target = dir.join(format!("season{:02}.jpg", season));
                if force_image_refresh {
                    let season_stem = format!("Season {season:02}");
                    remove_image_candidates(dir, &season_stem, "primary");
                }
                if force_image_refresh || !target.exists() {
                    if let Err(err) = self
                        .ensure_tmdb_image(season_poster, &target, force_image_refresh)
                        .await
                    {
                        warn!(error = %err, path = %target.display(), "failed to download season poster");
                    }
                }
            }
        }

        let episode_tmdb_id = episode_details
            .as_ref()
            .and_then(|v| v.get("id"))
            .and_then(Value::as_i64);
        let episode_name = episode_details
            .as_ref()
            .and_then(|v| v.get("name"))
            .and_then(Value::as_str)
            .map(str::to_string);
        let air_date = episode_details
            .as_ref()
            .and_then(|v| v.get("air_date"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| {
                tv_details
                    .get("first_air_date")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            });
        let production_year = air_date.as_deref().and_then(parse_year_from_date);
        let overview = episode_details
            .as_ref()
            .and_then(|v| v.get("overview"))
            .and_then(Value::as_str)
            .filter(|v| !v.trim().is_empty())
            .map(str::to_string)
            .or_else(|| {
                tv_details
                    .get("overview")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            });

        let season_details_clone = season_details.clone();
        let patch = json!({
            "tmdb_id": episode_tmdb_id.unwrap_or(tv_id),
            "series_tmdb_id": tv_id,
            "episode_tmdb_id": episode_tmdb_id,
            "provider_ids": { "Tmdb": tv_id.to_string() },
            "series_name": tv_details.get("name").and_then(Value::as_str),
            "season_name": season_details
                .as_ref()
                .and_then(|v| v.get("name"))
                .and_then(Value::as_str),
            "sort_name": episode_name.as_deref(),
            "overview": overview,
            "premiere_date": air_date,
            "production_year": production_year,
            "genres": extract_tmdb_genres(&tv_details),
            "tags": episode_keyword_tags.clone(),
            "studios": extract_tmdb_studios(&tv_details),
            "official_rating": official_rating.clone(),
            "community_rating": episode_details
                .as_ref()
                .and_then(|v| v.get("vote_average"))
                .and_then(Value::as_f64)
                .or_else(|| tv_details.get("vote_average").and_then(Value::as_f64)),
            "people": people_json,
            "nfo": {
                "title": episode_name.as_deref(),
                "official_rating": official_rating.clone(),
                "mpaa": official_rating,
            },
            "primary_image_tag": image_candidates(dir, stem, "thumb")
                .first()
                .and_then(|path| image_file_tag(path)),
            "tmdb_raw": {
                "tv": tv_details,
                "season": season_details,
                "episode": episode_details,
            },
        });

        let mut merged = merge_missing_json(item.metadata.clone(), &patch);
        merge_tmdb_tags_into_metadata(&mut merged, &episode_keyword_tags);
        let changed = merged != item.metadata;
        if changed {
            self.persist_item_metadata(item.id, &merged).await?;
        }

        let episode_nfo_path = media_path.with_extension("nfo");
        if let Err(err) =
            write_episode_nfo(&episode_nfo_path, &merged, season_number, episode_number)
        {
            warn!(error = %err, path = %episode_nfo_path.display(), "failed to write episode nfo");
        }
        let tvshow_nfo_path = resolve_tvshow_nfo_path(series_dir);
        if let Err(err) = write_tvshow_nfo(&tvshow_nfo_path, &merged) {
            warn!(error = %err, path = %tvshow_nfo_path.display(), "failed to write tvshow nfo");
        }
        if let Some(season) = season_number {
            let season_nfo_path = resolve_season_nfo_path(dir, season);
            if let Err(err) = write_season_nfo(&season_nfo_path, &merged, season) {
                warn!(error = %err, path = %season_nfo_path.display(), "failed to write season nfo");
            }
        }

        if let Some(ref ep_title) = episode_name {
            if !ep_title.trim().is_empty() {
                self.update_item_name_and_search_keys(item.id, ep_title).await?;
            }
        }

        // Propagate metadata to Season item
        if let Some(season_payload) = season_details_clone.as_ref() {
            let metadata_season_id = item
                .metadata
                .get("season_id")
                .and_then(Value::as_str)
                .and_then(|raw| Uuid::parse_str(raw).ok());
            let metadata_series_id = item
                .metadata
                .get("series_id")
                .and_then(Value::as_str)
                .and_then(|raw| Uuid::parse_str(raw).ok());
            let season_dir_str = dir.to_string_lossy().to_string();
            let season_item: Option<TmdbFillItemRow> = if let Some(season_id) = metadata_season_id {
                sqlx::query_as(
                    "SELECT id,name,item_type,path,season_number,episode_number,metadata FROM media_items WHERE id=$1 AND item_type='Season' LIMIT 1",
                )
                .bind(season_id)
                .fetch_optional(&self.pool)
                .await?
            } else if let (Some(series_id), Some(season)) = (metadata_series_id, season_number) {
                sqlx::query_as(
                    "SELECT id,name,item_type,path,season_number,episode_number,metadata FROM media_items WHERE item_type='Season' AND series_id=$1 AND season_number=$2 ORDER BY updated_at DESC LIMIT 1",
                )
                .bind(series_id)
                .bind(season)
                .fetch_optional(&self.pool)
                .await?
            } else {
                sqlx::query_as(
                    "SELECT id,name,item_type,path,season_number,episode_number,metadata FROM media_items WHERE path=$1 AND item_type='Season' LIMIT 1",
                )
                .bind(&season_dir_str)
                .fetch_optional(&self.pool)
                .await?
            };

            if let Some(si) = season_item {
                let season_item_path = Path::new(&si.path);
                let season_item_dir = season_item_path.parent().unwrap_or_else(|| Path::new("."));
                let season_item_stem = season_item_path
                    .file_stem()
                    .and_then(|v| v.to_str())
                    .unwrap_or_default();
                let season_patch = json!({
                    "overview": season_payload.get("overview").and_then(Value::as_str).filter(|v| !v.trim().is_empty()),
                    "season_name": season_payload.get("name").and_then(Value::as_str),
                    "series_tmdb_id": tv_id,
                    "primary_image_tag": image_candidates(season_item_dir, season_item_stem, "primary")
                        .first()
                        .and_then(|path| image_file_tag(path)),
                });
                let season_merged = merge_missing_json(si.metadata.clone(), &season_patch);
                if season_merged != si.metadata {
                    self.persist_item_metadata(si.id, &season_merged).await?;
                }
                if let Some(sname) = season_payload.get("name").and_then(Value::as_str)
                    .map(str::trim).filter(|v| !v.is_empty())
                {
                    self.update_item_name_and_search_keys(si.id, sname).await?;
                }
                if let Some(season) = season_number {
                    let season_nfo_path = resolve_season_nfo_path(dir, season);
                    if let Err(err) = write_season_nfo(&season_nfo_path, &season_merged, season) {
                        warn!(error = %err, "failed to write season nfo from episode fill");
                    }
                }
            }
        }

        Ok(changed || !people_candidates.is_empty())
    }

    async fn ensure_tmdb_image(
        &self,
        tmdb_path: &str,
        target_path: &Path,
        force_overwrite: bool,
    ) -> anyhow::Result<Option<String>> {
        if tmdb_path.trim().is_empty() {
            return Ok(None);
        }
        if target_path.exists() && !force_overwrite {
            return Ok(Some(target_path.to_string_lossy().to_string()));
        }

        let endpoint = format!("{TMDB_IMAGE_BASE}{tmdb_path}");
        self.wait_tmdb_rate_limit().await;
        self.metrics
            .tmdb_http_requests_total
            .fetch_add(1, Ordering::Relaxed);
        let response = self
            .http_client
            .get(&endpoint)
            .timeout(std::time::Duration::from_secs(
                self.config_snapshot().tmdb.timeout_seconds.max(1),
            ))
            .send()
            .await
            .with_context(|| format!("failed to request tmdb image: {endpoint}"))?;
        if !response.status().is_success() {
            anyhow::bail!(
                "tmdb image returned status {} for {endpoint}",
                response.status()
            );
        }
        let bytes = response
            .bytes()
            .await
            .context("failed to read tmdb image body")?;
        if let Some(parent) = target_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("failed to create image dir: {}", parent.display()))?;
        }
        tokio::fs::write(target_path, bytes)
            .await
            .with_context(|| format!("failed to write image file: {}", target_path.display()))?;
        Ok(Some(target_path.to_string_lossy().to_string()))
    }

    async fn upsert_people_for_item(
        &self,
        item_id: Uuid,
        candidates: &[TmdbPersonCandidate],
    ) -> anyhow::Result<Vec<Value>> {
        if candidates.is_empty() {
            return Ok(Vec::new());
        }

        sqlx::query("DELETE FROM media_item_people WHERE media_item_id = $1")
            .bind(item_id)
            .execute(&self.pool)
            .await?;

        let mut out = Vec::with_capacity(candidates.len());

        for candidate in candidates {
            let profile_path = candidate.profile_path.clone();
            let person_image_target = person_image_cache_path(
                &self.config_snapshot().tmdb.person_image_cache_dir,
                candidate.tmdb_id,
            );
            let image_path = if let Some(profile_path) = profile_path.as_deref() {
                match self
                    .ensure_tmdb_image(profile_path, &person_image_target, false)
                    .await
                {
                    Ok(path) => path,
                    Err(err) => {
                        warn!(
                            error = %err,
                            path = %person_image_target.display(),
                            person_tmdb_id = candidate.tmdb_id,
                            "failed to download person image"
                        );
                        if person_image_target.exists() {
                            Some(person_image_target.to_string_lossy().to_string())
                        } else {
                            None
                        }
                    }
                }
            } else if person_image_target.exists() {
                Some(person_image_target.to_string_lossy().to_string())
            } else {
                None
            };

            let primary_image_tag = image_path
                .as_ref()
                .map(|path| auth::hash_api_key(&format!("{}:{path}", candidate.tmdb_id)));
            let person_meta = json!({
                "tmdb_id": candidate.tmdb_id,
                "profile_path": profile_path,
            });

            let row = sqlx::query_as::<_, PersonRow>(
                r#"
INSERT INTO people (id, tmdb_id, name, profile_path, image_path, primary_image_tag, metadata)
VALUES ($1, $2, $3, $4, $5, $6, $7)
ON CONFLICT(tmdb_id) DO UPDATE SET
    name = COALESCE(NULLIF(EXCLUDED.name, ''), people.name),
    profile_path = COALESCE(EXCLUDED.profile_path, people.profile_path),
    image_path = EXCLUDED.image_path,
    primary_image_tag = EXCLUDED.primary_image_tag,
    metadata = COALESCE(people.metadata, '{}'::jsonb) || EXCLUDED.metadata,
    updated_at = now()
RETURNING id, name, image_path, primary_image_tag, metadata, created_at
                "#,
            )
            .bind(Uuid::now_v7())
            .bind(candidate.tmdb_id)
            .bind(&candidate.name)
            .bind(profile_path)
            .bind(image_path)
            .bind(primary_image_tag)
            .bind(person_meta)
            .fetch_one(&self.pool)
            .await?;

            sqlx::query(
                r#"
INSERT INTO media_item_people (media_item_id, person_id, person_type, role, sort_order)
VALUES ($1, $2, $3, $4, $5)
ON CONFLICT(media_item_id, person_id, person_type) DO UPDATE SET
    role = EXCLUDED.role,
    sort_order = EXCLUDED.sort_order
                "#,
            )
            .bind(item_id)
            .bind(row.id)
            .bind(&candidate.person_type)
            .bind(&candidate.role)
            .bind(candidate.sort_order)
            .execute(&self.pool)
            .await?;

            out.push(json!({
                "id": row.id.to_string(),
                "name": row.name,
                "type": candidate.person_type,
                "role": candidate.role,
                "primary_image_tag": row.primary_image_tag,
            }));
        }

        Ok(out)
    }

    async fn persist_item_metadata(&self, item_id: Uuid, metadata: &Value) -> anyhow::Result<()> {
        sqlx::query(
            r#"
UPDATE media_items
SET metadata = $2,
    updated_at = now()
WHERE id = $1
            "#,
        )
        .bind(item_id)
        .bind(metadata)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update_item_name_and_search_keys(
        &self,
        item_id: Uuid,
        new_name: &str,
    ) -> anyhow::Result<()> {
        let keys = crate::search::build_search_keys(new_name);
        sqlx::query("UPDATE media_items SET name=$2, search_text=$3, search_pinyin=$4, search_initials=$5, updated_at=now() WHERE id=$1")
            .bind(item_id)
            .bind(new_name)
            .bind(keys.text)
            .bind(keys.pinyin)
            .bind(keys.initials)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

}

#[derive(Debug, Clone)]
struct MovieMatchHints {
    query_title: String,
    normalized_titles: Vec<String>,
    year: Option<i32>,
}

fn movie_tmdb_binding_is_manual(metadata: &Value) -> bool {
    metadata
        .get("tmdb_binding_source")
        .and_then(Value::as_str)
        .is_some_and(|value| value.eq_ignore_ascii_case("manual"))
}

#[derive(Debug, Clone, Copy)]
struct MovieMatchAssessment {
    title_exact: bool,
    title_partial: bool,
    year_gap: Option<i32>,
}

impl MovieMatchAssessment {
    fn confident(self, hints: &MovieMatchHints) -> bool {
        let has_title_hints = !hints.normalized_titles.is_empty();
        if has_title_hints {
            if self.title_exact {
                return self.year_gap.map(|gap| gap <= 2).unwrap_or(true);
            }
            if self.title_partial {
                return self.year_gap.map(|gap| gap <= 1).unwrap_or(false);
            }
            return false;
        }
        if hints.year.is_some() {
            return self.year_gap.map(|gap| gap <= 1).unwrap_or(false);
        }
        true
    }
}

fn build_movie_match_hints(item: &TmdbFillItemRow, media_path: &Path) -> MovieMatchHints {
    let nfo_title = item
        .metadata
        .get("nfo")
        .and_then(|value| value.get("title"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let item_name = item
        .name
        .trim()
        .to_string();
    let stem_name = media_path
        .file_stem()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let parent_name = media_path
        .parent()
        .and_then(|value| value.file_name())
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    let query_title = nfo_title
        .clone()
        .or(stem_name.clone())
        .or(parent_name.clone())
        .or_else(|| (!item_name.is_empty()).then(|| item_name.clone()))
        .unwrap_or_default();
    let mut normalized_titles = Vec::<String>::new();
    for raw in [nfo_title.clone(), stem_name, parent_name]
        .into_iter()
        .flatten()
    {
        if let Some(normalized) = normalize_tmdb_match_title(&raw) {
            if !normalized_titles.iter().any(|value| value == &normalized) {
                normalized_titles.push(normalized);
            }
        }
    }
    if normalized_titles.is_empty()
        && !item_name.is_empty()
        && let Some(normalized) = normalize_tmdb_match_title(&item_name)
    {
        normalized_titles.push(normalized);
    }

    let year = item
        .metadata
        .get("nfo")
        .and_then(|value| value.get("year"))
        .and_then(value_year_hint)
        .or_else(|| item.metadata.get("production_year").and_then(value_year_hint))
        .or_else(|| movie_year_hint_from_path(media_path));

    MovieMatchHints {
        query_title,
        normalized_titles,
        year,
    }
}

fn movie_year_hint_from_path(media_path: &Path) -> Option<i32> {
    let path_str = media_path.to_string_lossy();
    let re = Regex::new(r"(?:19|20)\d{2}").ok()?;
    re.find_iter(&path_str)
        .last()
        .and_then(|m| m.as_str().parse::<i32>().ok())
}

fn tmdb_movie_title_norms(payload: &Value) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for key in ["title", "name", "original_title", "original_name"] {
        if let Some(normalized) = payload
            .get(key)
            .and_then(Value::as_str)
            .and_then(normalize_tmdb_match_title)
        {
            if !out.iter().any(|value| value == &normalized) {
                out.push(normalized);
            }
        }
    }
    out
}

fn tmdb_movie_primary_year(payload: &Value) -> Option<i32> {
    payload
        .get("release_date")
        .or_else(|| payload.get("first_air_date"))
        .and_then(Value::as_str)
        .and_then(parse_year_from_date)
}

fn normalize_tmdb_release_date(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let date = trimmed.get(..10)?;
    if NaiveDate::parse_from_str(date, "%Y-%m-%d").is_ok() {
        Some(date.to_string())
    } else {
        None
    }
}

fn tmdb_movie_release_dates(release_dates: Option<&Value>) -> Vec<String> {
    let Some(payload) = release_dates else {
        return Vec::new();
    };
    let mut dates = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    for country in payload
        .get("results")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        for entry in country
            .get("release_dates")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let Some(release_date) = entry.get("release_date").and_then(Value::as_str) else {
                continue;
            };
            let Some(normalized) = normalize_tmdb_release_date(release_date) else {
                continue;
            };
            if seen.insert(normalized.clone()) {
                dates.push(normalized);
            }
        }
    }
    dates
}

fn tmdb_rating_country_preferences(language: &str) -> Vec<String> {
    let mut countries = Vec::<String>::new();
    if let Some(region) = language
        .split(['-', '_'])
        .nth(1)
        .map(str::trim)
        .filter(|value| value.len() == 2)
        .map(|value| value.to_ascii_uppercase())
    {
        countries.push(region);
    }
    let normalized = language.to_ascii_lowercase();
    if normalized.starts_with("zh") {
        countries.extend(["CN", "HK", "TW"].into_iter().map(str::to_string));
    } else if normalized.starts_with("ja") {
        countries.push("JP".to_string());
    } else if normalized.starts_with("ko") {
        countries.push("KR".to_string());
    }
    countries.extend(["US", "GB"].into_iter().map(str::to_string));
    let mut seen = HashSet::new();
    countries.retain(|country| seen.insert(country.clone()));
    countries
}

fn pick_tmdb_country_rating(
    ratings: Vec<(String, String)>,
    language: &str,
) -> Option<String> {
    if ratings.is_empty() {
        return None;
    }
    let preferences = tmdb_rating_country_preferences(language);
    for preferred in preferences {
        if let Some((_, rating)) = ratings
            .iter()
            .find(|(country, _)| country.eq_ignore_ascii_case(&preferred))
        {
            return Some(rating.clone());
        }
    }
    ratings.first().map(|(_, rating)| rating.clone())
}

fn tmdb_movie_country_certification(country_payload: &Value) -> Option<String> {
    let mut best: Option<(i32, String)> = None;
    for entry in country_payload
        .get("release_dates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(certification) = entry
            .get("certification")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let release_type = entry.get("type").and_then(Value::as_i64).unwrap_or(-1);
        let release_rank = match release_type {
            3 => 0, // theatrical
            2 => 1, // premiere
            1 => 2, // limited theatrical
            4 => 3, // digital
            5 => 4, // physical
            6 => 5, // tv
            _ => 9,
        };
        if best
            .as_ref()
            .map(|(rank, _)| release_rank < i64::from(*rank))
            .unwrap_or(true)
        {
            best = Some((release_rank as i32, certification.to_string()));
        }
    }
    best.map(|(_, value)| value)
}

fn tmdb_movie_official_rating(release_dates: Option<&Value>, language: &str) -> Option<String> {
    let Some(payload) = release_dates else {
        return None;
    };
    let ratings = payload
        .get("results")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|country_payload| {
            let country = country_payload
                .get("iso_3166_1")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())?
                .to_ascii_uppercase();
            let rating = tmdb_movie_country_certification(country_payload)?;
            Some((country, rating))
        })
        .collect::<Vec<_>>();
    pick_tmdb_country_rating(ratings, language)
}

fn tmdb_tv_official_rating(content_ratings: Option<&Value>, language: &str) -> Option<String> {
    let Some(payload) = content_ratings else {
        return None;
    };
    let ratings = payload
        .get("results")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let country = entry
                .get("iso_3166_1")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())?
                .to_ascii_uppercase();
            let rating = entry
                .get("rating")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())?
                .to_string();
            Some((country, rating))
        })
        .collect::<Vec<_>>();
    pick_tmdb_country_rating(ratings, language)
}

fn tmdb_movie_release_year(payload: &Value, release_dates: Option<&Value>) -> Option<i32> {
    let release_date_year = tmdb_movie_release_dates(release_dates)
        .into_iter()
        .filter_map(|value| parse_year_from_date(&value))
        .min();
    release_date_year.or_else(|| tmdb_movie_primary_year(payload))
}

fn tmdb_movie_premiere_date(payload: &Value, release_dates: Option<&Value>) -> Option<String> {
    let mut dates = tmdb_movie_release_dates(release_dates);
    dates.sort();
    dates.into_iter().next().or_else(|| {
        payload
            .get("release_date")
            .and_then(Value::as_str)
            .and_then(normalize_tmdb_release_date)
    })
}

fn assess_movie_match(
    hints: &MovieMatchHints,
    payload: &Value,
    candidate_year: Option<i32>,
) -> MovieMatchAssessment {
    let candidate_titles = tmdb_movie_title_norms(payload);
    let mut title_exact = false;
    let mut title_partial = false;
    for local_title in &hints.normalized_titles {
        for candidate_title in &candidate_titles {
            if local_title == candidate_title {
                title_exact = true;
                break;
            }
            if local_title.len() >= 4
                && candidate_title.len() >= 4
                && (candidate_title.contains(local_title) || local_title.contains(candidate_title))
            {
                title_partial = true;
            }
        }
        if title_exact {
            break;
        }
    }

    let year_gap = match (hints.year, candidate_year) {
        (Some(local_year), Some(tmdb_year)) => Some((local_year - tmdb_year).abs()),
        _ => None,
    };
    MovieMatchAssessment {
        title_exact,
        title_partial,
        year_gap,
    }
}

fn score_movie_search_candidate(
    hints: &MovieMatchHints,
    payload: &Value,
) -> Option<(i64, i32, MovieMatchAssessment)> {
    let movie_id = payload.get("id").and_then(Value::as_i64)?;
    let assessment = assess_movie_match(hints, payload, tmdb_movie_release_year(payload, None));
    let score = score_movie_candidate(payload, assessment);
    Some((movie_id, score, assessment))
}

fn score_movie_candidate(payload: &Value, assessment: MovieMatchAssessment) -> i32 {
    let mut score = 0_i32;
    if assessment.title_exact {
        score += 120;
    } else if assessment.title_partial {
        score += 60;
    } else {
        score -= 50;
    }
    if let Some(year_gap) = assessment.year_gap {
        score += match year_gap {
            0 => 40,
            1 => 22,
            2 => 8,
            3 => -8,
            _ => -30,
        };
    }
    let vote_count = payload
        .get("vote_count")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    score += ((vote_count / 50).min(12)) as i32;
    if payload.get("adult").and_then(Value::as_bool) == Some(true) {
        score -= 15;
    }
    score
}

fn rank_movie_search_candidates(
    hints: &MovieMatchHints,
    results: &[Value],
) -> Vec<(i64, i32, MovieMatchAssessment)> {
    let mut ranked = results
        .iter()
        .filter_map(|result| score_movie_search_candidate(hints, result))
        .filter(|(_, score, assessment)| {
            (assessment.title_exact || assessment.title_partial) && *score >= 40
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| right.1.cmp(&left.1));
    ranked
}

#[cfg(test)]
fn select_best_movie_search_candidate(
    hints: &MovieMatchHints,
    results: &[Value],
) -> Option<(i64, i32, MovieMatchAssessment)> {
    let mut best: Option<(i64, i32, MovieMatchAssessment)> = None;
    for (movie_id, score, assessment) in rank_movie_search_candidates(hints, results) {
        if !assessment.confident(hints) {
            continue;
        }
        if score < 100 {
            continue;
        }
        match best {
            Some((_, best_score, _)) if best_score >= score => {}
            _ => best = Some((movie_id, score, assessment)),
        }
    }
    best
}

fn remove_image_candidates(dir: &Path, stem: &str, image_type: &str) {
    for candidate in image_candidates(dir, stem, image_type) {
        match std::fs::remove_file(&candidate) {
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => {
                warn!(
                    error = %err,
                    image_type,
                    path = %candidate.display(),
                    "failed to remove existing image before forced refresh"
                );
            }
        }
    }
}

fn person_image_cache_path(cache_dir: &str, tmdb_id: i64) -> PathBuf {
    Path::new(cache_dir).join(format!("person-{tmdb_id}.jpg"))
}

fn tmdb_logo_extension(tmdb_path: &str) -> &'static str {
    match Path::new(tmdb_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => "png",
        Some("jpg") | Some("jpeg") => "jpg",
        Some("webp") => "webp",
        _ => "png",
    }
}

fn select_tmdb_logo_path(details: &Value, preferred_language: &str) -> Option<String> {
    let logos = details
        .get("images")
        .and_then(|images| images.get("logos"))
        .and_then(Value::as_array)?;

    let preferred = preferred_language
        .split(['-', '_'])
        .next()
        .map(str::trim)
        .filter(|lang| !lang.is_empty())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let mut best: Option<(i32, f64, String)> = None;
    for logo in logos {
        let Some(path) = logo
            .get("file_path")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|path| !path.is_empty())
        else {
            continue;
        };

        let lang = logo
            .get("iso_639_1")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or_default()
            .to_ascii_lowercase();
        let rank = if !preferred.is_empty() && lang == preferred {
            3
        } else if lang == "en" {
            2
        } else if lang.is_empty() {
            1
        } else {
            0
        };
        let score = logo.get("vote_average").and_then(Value::as_f64).unwrap_or(0.0);

        let replace = match best.as_ref() {
            None => true,
            Some((best_rank, best_score, _)) => {
                rank > *best_rank || (rank == *best_rank && score > *best_score)
            }
        };
        if replace {
            best = Some((rank, score, path.to_string()));
        }
    }

    best.map(|(_, _, path)| path)
}

fn tmdb_include_image_language(preferred_language: &str) -> String {
    let mut languages = Vec::<String>::new();

    let normalized = preferred_language.trim();
    if !normalized.is_empty() {
        languages.push(normalized.to_ascii_lowercase());
    }
    if let Some(base) = normalized
        .split(['-', '_'])
        .next()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_ascii_lowercase)
    {
        languages.push(base);
    }
    languages.push("en".to_string());
    languages.push("null".to_string());

    languages.sort();
    languages.dedup();
    urlencoding::encode(&languages.join(",")).to_string()
}
