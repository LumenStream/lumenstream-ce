use std::{
    collections::{HashMap, HashSet},
    future::Future,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::Context;
use chrono::{DateTime, Duration, Utc};
use regex::Regex;
use serde_json::{Value, json};
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::search::{build_search_keys, normalize_media_title};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanMode {
    Full,
    Incremental,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanExistingItemPolicy {
    Skip,
    Upsert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanProbePolicy {
    Enabled,
    Disabled,
}

#[derive(Debug, Default, Clone)]
pub struct ScanSummary {
    pub scanned_files: usize,
    pub upserted_items: usize,
    pub subtitle_files: usize,
    pub duplicate_merged: usize,
    pub metadata_missing: usize,
}

#[derive(Debug, Default, Clone)]
struct ParsedNfo {
    title: Option<String>,
    overview: Option<String>,
    year: Option<i32>,
    official_rating: Option<String>,
    season_number: Option<i32>,
    episode_number: Option<i32>,
    tmdb_id: Option<String>,
    tags: Vec<String>,
    taglines: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LibraryTypeMode {
    Movies,
    Series,
    Mixed,
}

impl LibraryTypeMode {
    fn from_raw(raw: &str) -> Self {
        if raw.eq_ignore_ascii_case("movie") || raw.eq_ignore_ascii_case("movies") {
            return Self::Movies;
        }
        if raw.eq_ignore_ascii_case("series")
            || raw.eq_ignore_ascii_case("show")
            || raw.eq_ignore_ascii_case("shows")
            || raw.eq_ignore_ascii_case("tv")
            || raw.eq_ignore_ascii_case("tvshows")
        {
            return Self::Series;
        }
        Self::Mixed
    }
}

#[derive(Debug, Clone)]
struct SeriesHierarchyRefs {
    series_id: Uuid,
    series_name: String,
    season_id: Option<Uuid>,
    season_name: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct VersionGroupCandidateRow {
    id: Uuid,
    item_type: String,
    name: String,
    path: String,
    series_id: Option<Uuid>,
    season_number: Option<i32>,
    episode_number: Option<i32>,
    bitrate: Option<i32>,
    stream_url: Option<String>,
    nfo_title: Option<String>,
    production_year: Option<i32>,
    nfo_year: Option<i32>,
    metadata_season_number: Option<i32>,
    metadata_parent_index_number: Option<i32>,
    metadata_episode_number: Option<i32>,
    metadata_index_number: Option<i32>,
}

#[derive(Debug, Clone)]
struct SubtitleDirEntry {
    path: PathBuf,
    ext_lower: String,
}

#[derive(Debug, Default)]
struct SubtitleDirCache {
    entries_by_dir: HashMap<PathBuf, Vec<SubtitleDirEntry>>,
}

#[derive(Debug, Default)]
struct ModifiedSinceCache {
    values: HashMap<PathBuf, bool>,
}

const RUNTIME_TICKS_PER_SECOND: f64 = 10_000_000.0;

fn clean_strm_line(line: &str) -> &str {
    line.trim().trim_start_matches('\u{feff}')
}

fn normalize_scan_scope_prefix(scope_path: Option<&Path>) -> Option<String> {
    scope_path.and_then(|path| {
        let normalized = path
            .to_string_lossy()
            .trim()
            .trim_end_matches('/')
            .to_string();
        if normalized.is_empty() {
            None
        } else {
            Some(normalized)
        }
    })
}

async fn load_existing_media_paths(
    pool: &PgPool,
    library_id: Uuid,
    scope_path: Option<&Path>,
) -> anyhow::Result<HashSet<String>> {
    let scope_prefix = normalize_scan_scope_prefix(scope_path);
    let rows: Vec<String> = sqlx::query_scalar(
        r#"
SELECT path
FROM media_items
WHERE library_id = $1
  AND (
      $2::text IS NULL
      OR path = $2
      OR path LIKE ($2 || '/%')
  )
        "#,
    )
    .bind(library_id)
    .bind(scope_prefix)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().collect())
}

fn should_skip_existing_item(
    policy: ScanExistingItemPolicy,
    path: &str,
    existing_paths: &HashSet<String>,
) -> bool {
    matches!(policy, ScanExistingItemPolicy::Skip) && existing_paths.contains(path)
}

fn should_probe_mediainfo_sidecar(
    policy: ScanProbePolicy,
    has_mediainfo_file: bool,
    stream_url: Option<&str>,
) -> bool {
    matches!(policy, ScanProbePolicy::Enabled)
        && should_generate_mediainfo_sidecar(has_mediainfo_file, stream_url)
}

fn parse_stream_url_from_strm_content(strm_raw: &str) -> Option<String> {
    let mut fallback = None;
    for line in strm_raw.lines() {
        let cleaned = clean_strm_line(line);
        if cleaned.is_empty() || cleaned.starts_with('#') {
            continue;
        }

        if cleaned.starts_with("http://")
            || cleaned.starts_with("https://")
            || cleaned.starts_with("gdrive://")
            || cleaned.starts_with("s3://")
            || cleaned.starts_with("lumenbackend://")
        {
            return Some(cleaned.to_string());
        }

        if fallback.is_none() {
            fallback = Some(cleaned.to_string());
        }
    }

    fallback
}

/// Map a media file path to a deterministic cache path, mirroring the directory structure.
fn mediainfo_cache_path(cache_dir: &str, media_path: &Path, stem: &str) -> PathBuf {
    let rel = media_path
        .parent()
        .unwrap_or(Path::new(""))
        .to_string_lossy();
    let rel_clean = rel.trim_start_matches('/');
    Path::new(cache_dir)
        .join(rel_clean)
        .join(format!("{stem}-mediainfo.json"))
}

pub async fn scan_library<F, Fut>(
    pool: &PgPool,
    library_id: Uuid,
    root_paths: &[String],
    library_type: &str,
    scope_path: Option<&Path>,
    subtitle_exts: &[String],
    mode: ScanMode,
    since: Option<DateTime<Utc>>,
    grace_seconds: i64,
    mediainfo_cache_dir: &str,
    existing_item_policy: ScanExistingItemPolicy,
    probe_policy: ScanProbePolicy,
    mut on_progress: F,
) -> anyhow::Result<ScanSummary>
where
    F: FnMut(i64, i64) -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    let mut roots = root_paths
        .iter()
        .map(|root| root.trim())
        .filter(|root| !root.is_empty())
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if roots.is_empty() {
        on_progress(1, 1).await?;
        return Ok(ScanSummary::default());
    }

    let mut dedup_roots = HashSet::new();
    roots = roots
        .into_iter()
        .map(|root| {
            std::fs::canonicalize(&root)
                .with_context(|| format!("library root path does not exist: {}", root.display()))
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|root| dedup_roots.insert(root.to_string_lossy().to_string()))
        .collect::<Vec<_>>();

    let library_type_mode = LibraryTypeMode::from_raw(library_type);
    let walk_roots = if let Some(scope_root) = scope_path {
        vec![scope_root.to_path_buf()]
    } else {
        roots.clone()
    };
    for walk_root in &walk_roots {
        if !walk_root.exists() {
            anyhow::bail!("scan scope path does not exist: {}", walk_root.display());
        }
    }

    let threshold = since.map(|ts| ts - Duration::seconds(grace_seconds.max(0)));
    let subtitle_ext_lookup = build_subtitle_ext_lookup(subtitle_exts);
    let mut subtitle_dir_cache = SubtitleDirCache::default();
    let mut modified_since_cache = ModifiedSinceCache::default();
    let mut summary = ScanSummary::default();
    let mut version_group_dirty = false;
    let mut existing_paths = load_existing_media_paths(pool, library_id, scope_path).await?;

    let mut target_paths = HashSet::new();
    let mut targets = Vec::<(PathBuf, PathBuf)>::new();
    for walk_root in &walk_roots {
        for entry in WalkDir::new(walk_root)
            .follow_links(true)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
        {
            let path = entry.path();
            if path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("strm"))
                != Some(true)
            {
                continue;
            }

            if mode == ScanMode::Incremental
                && !should_scan_incremental_with_cache(
                    path,
                    threshold,
                    &subtitle_ext_lookup,
                    &mut subtitle_dir_cache,
                    &mut modified_since_cache,
                )?
            {
                continue;
            }

            let key = path.to_string_lossy().to_string();
            if !target_paths.insert(key) {
                continue;
            }

            let root = roots
                .iter()
                .filter(|candidate| path.starts_with(candidate))
                .max_by_key(|candidate| candidate.components().count())
                .cloned()
                .unwrap_or_else(|| walk_root.to_path_buf());
            targets.push((path.to_path_buf(), root));
        }
    }

    let total = targets.len() as i64;
    if total == 0 {
        on_progress(1, 1).await?;
    }

    for (idx, (path, root)) in targets.iter().enumerate() {
        let completed = idx as i64 + 1;
        if completed % 25 == 0 || completed == total {
            on_progress(total, completed).await?;
        }

        summary.scanned_files += 1;
        let path_str = path.to_string_lossy().to_string();
        if should_skip_existing_item(existing_item_policy, &path_str, &existing_paths) {
            continue;
        }
        existing_paths.insert(path_str.clone());

        let strm_raw = tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("failed to read strm file: {}", path.display()))?;
        let stream_url = parse_stream_url_from_strm_content(&strm_raw);

        let stem = path
            .file_stem()
            .and_then(|v| v.to_str())
            .unwrap_or_default()
            .to_string();

        let nfo_path = path.with_extension("nfo");
        let nfo_raw = if nfo_path.exists() {
            Some(
                tokio::fs::read_to_string(&nfo_path)
                    .await
                    .unwrap_or_default(),
            )
        } else {
            None
        };
        let parsed_nfo = nfo_raw.as_deref().map(parse_nfo).unwrap_or_default();

        let mediainfo_path = mediainfo_cache_path(mediainfo_cache_dir, path, &stem);
        let legacy_mediainfo_path = path.with_file_name(format!("{}-mediainfo.json", stem));
        if should_probe_mediainfo_sidecar(
            probe_policy,
            mediainfo_path.exists() || legacy_mediainfo_path.exists(),
            stream_url.as_deref(),
        ) {
            if let Some(probe_target) = stream_url.as_deref() {
                if let Err(err) = generate_mediainfo_sidecar(&mediainfo_path, probe_target).await {
                    warn!(
                        path = %path.display(),
                        probe_target,
                        error = ?err,
                        "failed to auto-generate mediainfo sidecar with ffprobe"
                    );
                }
            }
        }

        let effective_mediainfo_path = if mediainfo_path.exists() {
            &mediainfo_path
        } else {
            &legacy_mediainfo_path
        };
        let mediainfo_raw = if effective_mediainfo_path.exists() {
            Some(
                tokio::fs::read_to_string(effective_mediainfo_path)
                    .await
                    .unwrap_or_default(),
            )
        } else {
            None
        };
        let mediainfo = mediainfo_raw
            .as_deref()
            .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
            .unwrap_or(Value::Null);

        let normalized_mediainfo = normalize_mediainfo(&mediainfo);
        let runtime_ticks = extract_runtime_ticks(&normalized_mediainfo);
        let bitrate = extract_bitrate(&normalized_mediainfo).map(|v| v as i32);

        let item_type = detect_item_type(path, &parsed_nfo, library_type_mode);
        let season_number = parsed_nfo
            .season_number
            .or_else(|| parse_season_number(path));
        let episode_number = parsed_nfo
            .episode_number
            .or_else(|| parse_episode_number(path));
        let hierarchy = if library_type_mode == LibraryTypeMode::Series {
            Some(
                ensure_series_hierarchy_items(
                    pool,
                    library_id,
                    root.as_path(),
                    path,
                    season_number,
                )
                .await?,
            )
        } else {
            None
        };
        let series_id = if item_type == "Episode" {
            hierarchy
                .as_ref()
                .map(|h| h.series_id)
                .or_else(|| derive_series_id(path))
        } else {
            None
        };

        let item_name = resolve_item_name(parsed_nfo.title.as_deref(), &stem);
        let series_name = hierarchy.as_ref().map(|h| h.series_name.clone());
        let season_id = hierarchy.as_ref().and_then(|h| h.season_id);
        let season_name = hierarchy.as_ref().and_then(|h| h.season_name.clone());

        let images = find_images(path);
        let missing = missing_metadata_fields(
            &parsed_nfo,
            runtime_ticks,
            bitrate,
            stream_url.as_deref(),
            !images.is_empty(),
        );
        if !missing.is_empty() {
            summary.metadata_missing += 1;
        }

        let metadata = json!({
            "source": {
                "strm": path_str,
                "nfo": nfo_path.exists(),
                "mediainfo_json": effective_mediainfo_path.exists()
            },
            "strm_url": stream_url,
            "production_year": parsed_nfo.year,
            "official_rating": parsed_nfo.official_rating,
            "tags": parsed_nfo.tags.clone(),
            "taglines": parsed_nfo.taglines.clone(),
            "nfo": {
                "title": parsed_nfo.title,
                "overview": parsed_nfo.overview,
                "year": parsed_nfo.year,
                "official_rating": parsed_nfo.official_rating,
                "season_number": parsed_nfo.season_number,
                "episode_number": parsed_nfo.episode_number,
                "tmdb_id": parsed_nfo.tmdb_id,
                "tags": parsed_nfo.tags,
                "taglines": parsed_nfo.taglines
            },
            "series_name": series_name,
            "season_id": season_id,
            "season_name": season_name,
            "mediainfo": normalized_mediainfo,
            "images": images,
            "missing_fields": missing
        });

        let item_id = upsert_media_item(
            pool,
            library_id,
            &item_type,
            &item_name,
            &path_str,
            series_id,
            season_number,
            episode_number,
            runtime_ticks,
            bitrate,
            stream_url.as_deref(),
            &metadata,
        )
        .await?;

        summary.upserted_items += 1;
        if item_type == "Movie" || item_type == "Episode" {
            version_group_dirty = true;
        }

        let subtitles =
            find_subtitles_with_cache(path, &subtitle_ext_lookup, &mut subtitle_dir_cache);
        let replaced = replace_subtitles(pool, item_id, subtitles).await?;
        if replaced.changed {
            summary.subtitle_files += replaced.inserted;
        }

        if let Some(stream_url) = stream_url {
            if merge_duplicates(pool, item_id, &stream_url).await? {
                summary.duplicate_merged += 1;
                version_group_dirty = true;
            }
        }
    }

    let grouped_items = if version_group_dirty {
        rebuild_version_groups(pool, library_id).await?
    } else {
        0
    };

    info!(
        scanned = summary.scanned_files,
        upserted = summary.upserted_items,
        subtitles = summary.subtitle_files,
        duplicate_merged = summary.duplicate_merged,
        version_grouped = grouped_items,
        metadata_missing = summary.metadata_missing,
        mode = ?mode,
        "library scan completed"
    );

    Ok(summary)
}

pub async fn sync_subtitles<F, Fut>(
    pool: &PgPool,
    library_id: Option<Uuid>,
    subtitle_exts: &[String],
    mode: ScanMode,
    grace_seconds: i64,
    mut on_progress: F,
) -> anyhow::Result<i64>
where
    F: FnMut(i64, i64) -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    let grace_seconds = grace_seconds.max(0);
    let items: Vec<ItemPathRow> =
        match (library_id, mode) {
            (Some(library_id), ScanMode::Incremental) => {
                sqlx::query_as::<_, ItemPathRow>(
                    r#"
SELECT m.id, m.path
FROM media_items m
LEFT JOIN library_scan_state s ON s.library_id = m.library_id
WHERE m.library_id = $1
  AND (
      s.last_subtitle_sync_finished_at IS NULL
      OR m.updated_at >= s.last_subtitle_sync_finished_at - ($2 || ' seconds')::interval
  )
ORDER BY m.updated_at DESC
                "#,
                )
                .bind(library_id)
                .bind(grace_seconds)
                .fetch_all(pool)
                .await?
            }
            (None, ScanMode::Incremental) => {
                sqlx::query_as::<_, ItemPathRow>(
                    r#"
SELECT m.id, m.path
FROM media_items m
LEFT JOIN library_scan_state s ON s.library_id = m.library_id
WHERE
    s.last_subtitle_sync_finished_at IS NULL
    OR m.updated_at >= s.last_subtitle_sync_finished_at - ($1 || ' seconds')::interval
ORDER BY m.updated_at DESC
                "#,
                )
                .bind(grace_seconds)
                .fetch_all(pool)
                .await?
            }
            (Some(library_id), ScanMode::Full) => sqlx::query_as::<_, ItemPathRow>(
                "SELECT id, path FROM media_items WHERE library_id = $1 ORDER BY updated_at DESC",
            )
            .bind(library_id)
            .fetch_all(pool)
            .await?,
            (None, ScanMode::Full) => {
                sqlx::query_as::<_, ItemPathRow>(
                    "SELECT id, path FROM media_items ORDER BY updated_at DESC",
                )
                .fetch_all(pool)
                .await?
            }
        };

    let subtitle_ext_lookup = build_subtitle_ext_lookup(subtitle_exts);
    let mut subtitle_dir_cache = SubtitleDirCache::default();
    let mut touched = 0_i64;
    let total = items.len() as i64;
    if total == 0 {
        on_progress(1, 1).await?;
    }
    for (idx, item) in items.into_iter().enumerate() {
        let item_path = Path::new(&item.path);
        if item_path.exists() {
            let subtitles =
                find_subtitles_with_cache(item_path, &subtitle_ext_lookup, &mut subtitle_dir_cache);
            let replaced = replace_subtitles(pool, item.id, subtitles).await?;
            if replaced.changed {
                touched += 1;
            }
        }
        let completed = idx as i64 + 1;
        if completed % 25 == 0 || completed == total {
            on_progress(total, completed).await?;
        }
    }

    Ok(touched)
}

pub async fn repair_metadata<F, Fut>(
    pool: &PgPool,
    library_id: Option<Uuid>,
    subtitle_exts: &[String],
    mediainfo_cache_dir: &str,
    mut on_progress: F,
) -> anyhow::Result<i64>
where
    F: FnMut(i64, i64) -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    let items: Vec<ItemPathRow> = if let Some(library_id) = library_id {
        sqlx::query_as::<_, ItemPathRow>(
            "SELECT id, path FROM media_items WHERE library_id = $1 ORDER BY updated_at DESC",
        )
        .bind(library_id)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, ItemPathRow>(
            "SELECT id, path FROM media_items ORDER BY updated_at DESC",
        )
        .fetch_all(pool)
        .await?
    };

    let subtitle_ext_lookup = build_subtitle_ext_lookup(subtitle_exts);
    let mut subtitle_dir_cache = SubtitleDirCache::default();
    let mut repaired = 0_i64;
    let total = items.len() as i64;
    if total == 0 {
        on_progress(1, 1).await?;
    }

    for (idx, item) in items.into_iter().enumerate() {
        let item_path = Path::new(&item.path);
        if !item_path.exists() {
            let completed = idx as i64 + 1;
            if completed % 25 == 0 || completed == total {
                on_progress(total, completed).await?;
            }
            continue;
        }

        let stem = item_path
            .file_stem()
            .and_then(|v| v.to_str())
            .unwrap_or_default()
            .to_string();

        let nfo_path = item_path.with_extension("nfo");
        let mediainfo_path = mediainfo_cache_path(mediainfo_cache_dir, item_path, &stem);
        let legacy_mediainfo_path = item_path.with_file_name(format!("{}-mediainfo.json", stem));

        let parsed_nfo = if nfo_path.exists() {
            parse_nfo(
                &tokio::fs::read_to_string(&nfo_path)
                    .await
                    .unwrap_or_default(),
            )
        } else {
            ParsedNfo::default()
        };

        let effective_path = if mediainfo_path.exists() {
            &mediainfo_path
        } else {
            &legacy_mediainfo_path
        };
        let mediainfo_value = if effective_path.exists() {
            let raw = tokio::fs::read_to_string(effective_path)
                .await
                .unwrap_or_default();
            normalize_mediainfo(&serde_json::from_str::<Value>(&raw).unwrap_or(Value::Null))
        } else {
            Value::Null
        };

        let runtime_ticks = extract_runtime_ticks(&mediainfo_value);
        let bitrate = extract_bitrate(&mediainfo_value).map(|v| v as i32);
        let parsed_tags = if parsed_nfo.tags.is_empty() {
            None
        } else {
            Some(parsed_nfo.tags.clone())
        };
        let parsed_taglines = if parsed_nfo.taglines.is_empty() {
            None
        } else {
            Some(parsed_nfo.taglines.clone())
        };

        sqlx::query(
            r#"
UPDATE media_items
SET runtime_ticks = COALESCE($2, runtime_ticks),
    bitrate = COALESCE($3, bitrate),
    metadata = COALESCE(metadata, '{}'::jsonb)
        || jsonb_strip_nulls(
            jsonb_build_object(
                'production_year', to_jsonb($8::int),
                'official_rating', to_jsonb($13::text),
                'nfo', jsonb_strip_nulls(
                    jsonb_build_object(
                        'title', to_jsonb($4::text),
                        'overview', to_jsonb($5::text),
                        'tmdb_id', to_jsonb($6::text),
                        'year', to_jsonb($8::int),
                        'official_rating', to_jsonb($13::text),
                        'season_number', to_jsonb($9::int),
                        'episode_number', to_jsonb($10::int),
                        'tags', to_jsonb($11::text[]),
                        'taglines', to_jsonb($12::text[])
                    )
                ),
                'tags', to_jsonb($11::text[]),
                'taglines', to_jsonb($12::text[]),
                'mediainfo', to_jsonb($7::jsonb)
            )
        ),
    updated_at = now()
WHERE id = $1
            "#,
        )
        .bind(item.id)
        .bind(runtime_ticks)
        .bind(bitrate)
        .bind(parsed_nfo.title)
        .bind(parsed_nfo.overview)
        .bind(parsed_nfo.tmdb_id)
        .bind(mediainfo_value)
        .bind(parsed_nfo.year)
        .bind(parsed_nfo.season_number)
        .bind(parsed_nfo.episode_number)
        .bind(parsed_tags)
        .bind(parsed_taglines)
        .bind(parsed_nfo.official_rating)
        .execute(pool)
        .await?;

        let subtitles =
            find_subtitles_with_cache(item_path, &subtitle_ext_lookup, &mut subtitle_dir_cache);
        replace_subtitles(pool, item.id, subtitles).await?;
        repaired += 1;
        let completed = idx as i64 + 1;
        if completed % 25 == 0 || completed == total {
            on_progress(total, completed).await?;
        }
    }

    Ok(repaired)
}

#[cfg(test)]
fn should_scan_incremental(
    path: &Path,
    threshold: Option<DateTime<Utc>>,
    subtitle_exts: &[String],
) -> anyhow::Result<bool> {
    let subtitle_ext_lookup = build_subtitle_ext_lookup(subtitle_exts);
    let mut subtitle_dir_cache = SubtitleDirCache::default();
    let mut modified_since_cache = ModifiedSinceCache::default();
    should_scan_incremental_with_cache(
        path,
        threshold,
        &subtitle_ext_lookup,
        &mut subtitle_dir_cache,
        &mut modified_since_cache,
    )
}

fn should_scan_incremental_with_cache(
    path: &Path,
    threshold: Option<DateTime<Utc>>,
    subtitle_ext_lookup: &HashSet<String>,
    subtitle_dir_cache: &mut SubtitleDirCache,
    modified_since_cache: &mut ModifiedSinceCache,
) -> anyhow::Result<bool> {
    let Some(threshold) = threshold else {
        return Ok(true);
    };

    if modified_since_cache.modified_since(path, threshold)? {
        return Ok(true);
    }

    let nfo_path = path.with_extension("nfo");
    if modified_since_cache.modified_since(&nfo_path, threshold)? {
        return Ok(true);
    }

    if let Some(parent) = path.parent() {
        if modified_since_cache.modified_since(parent, threshold)? {
            return Ok(true);
        }
    }

    for subtitle in find_subtitles_with_cache(path, subtitle_ext_lookup, subtitle_dir_cache) {
        if modified_since_cache.modified_since(&subtitle, threshold)? {
            return Ok(true);
        }
    }

    Ok(false)
}

fn build_subtitle_ext_lookup(subtitle_exts: &[String]) -> HashSet<String> {
    subtitle_exts
        .iter()
        .map(|ext| ext.trim())
        .filter(|ext| !ext.is_empty())
        .map(|ext| ext.to_ascii_lowercase())
        .collect()
}

impl ModifiedSinceCache {
    fn modified_since(&mut self, path: &Path, threshold: DateTime<Utc>) -> anyhow::Result<bool> {
        if let Some(value) = self.values.get(path) {
            return Ok(*value);
        }

        let value = path_modified_since(path, threshold)?;
        self.values.insert(path.to_path_buf(), value);
        Ok(value)
    }
}

fn path_modified_since(path: &Path, threshold: DateTime<Utc>) -> anyhow::Result<bool> {
    if !path.exists() {
        return Ok(false);
    }

    let metadata = std::fs::metadata(path)
        .with_context(|| format!("failed to stat path: {}", path.display()))?;
    let modified = metadata
        .modified()
        .ok()
        .map(DateTime::<Utc>::from)
        .unwrap_or_else(Utc::now);
    Ok(modified >= threshold)
}

async fn upsert_media_item(
    pool: &PgPool,
    library_id: Uuid,
    item_type: &str,
    name: &str,
    path: &str,
    series_id: Option<Uuid>,
    season_number: Option<i32>,
    episode_number: Option<i32>,
    runtime_ticks: Option<i64>,
    bitrate: Option<i32>,
    stream_url: Option<&str>,
    metadata: &Value,
) -> anyhow::Result<Uuid> {
    let search_keys = build_search_keys(name);
    let item_id = sqlx::query_scalar(
        r#"
INSERT INTO media_items (
    id,
    library_id,
    item_type,
    name,
    search_text,
    search_pinyin,
    search_initials,
    path,
    series_id,
    season_number,
    episode_number,
    runtime_ticks,
    bitrate,
    stream_url,
    metadata
) VALUES (
    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15
)
ON CONFLICT(path) DO UPDATE SET
    library_id = EXCLUDED.library_id,
    item_type = EXCLUDED.item_type,
    name = EXCLUDED.name,
    search_text = EXCLUDED.search_text,
    search_pinyin = EXCLUDED.search_pinyin,
    search_initials = EXCLUDED.search_initials,
    series_id = EXCLUDED.series_id,
    season_number = EXCLUDED.season_number,
    episode_number = EXCLUDED.episode_number,
    runtime_ticks = EXCLUDED.runtime_ticks,
    bitrate = EXCLUDED.bitrate,
    stream_url = EXCLUDED.stream_url,
    metadata = media_items.metadata || EXCLUDED.metadata,
    updated_at = now()
RETURNING id
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(library_id)
    .bind(item_type)
    .bind(name)
    .bind(search_keys.text)
    .bind(search_keys.pinyin)
    .bind(search_keys.initials)
    .bind(path)
    .bind(series_id)
    .bind(season_number)
    .bind(episode_number)
    .bind(runtime_ticks)
    .bind(bitrate)
    .bind(stream_url.map(str::to_string))
    .bind(metadata)
    .fetch_one(pool)
    .await?;

    Ok(item_id)
}

async fn ensure_series_hierarchy_items(
    pool: &PgPool,
    library_id: Uuid,
    root: &Path,
    episode_path: &Path,
    season_number: Option<i32>,
) -> anyhow::Result<SeriesHierarchyRefs> {
    let Some(season_dir) = episode_path.parent() else {
        anyhow::bail!(
            "episode path has no parent directory: {}",
            episode_path.display()
        );
    };
    let season_dir = season_dir.to_path_buf();
    let series_dir = season_dir
        .parent()
        .filter(|parent| *parent != root)
        .unwrap_or(&season_dir)
        .to_path_buf();

    let series_name = directory_display_name(&series_dir, "Series");
    let series_path = series_dir.to_string_lossy().to_string();
    let series_metadata = json!({
        "series_name": series_name.clone(),
        "source": { "directory": series_path.clone() },
        "images": find_directory_images(&series_dir),
    });
    let series_id = upsert_media_item(
        pool,
        library_id,
        "Series",
        &series_name,
        &series_path,
        None,
        None,
        None,
        None,
        None,
        None,
        &series_metadata,
    )
    .await?;

    // Canonical semantics: every Episode must belong to a persisted Season row.
    // When there is no physical season directory, synthesize a stable Season path under the series directory.
    let (season_no, current_season_name, season_path_buf) =
        resolve_season_identity(&season_dir, &series_dir, season_number);
    let season_path = season_path_buf.to_string_lossy().to_string();
    let season_metadata = json!({
        "series_id": series_id,
        "series_name": series_name.clone(),
        "season_number": season_no,
        "season_name": current_season_name.clone(),
        "source": { "directory": season_dir.to_string_lossy().to_string() },
        "images": find_directory_images(&season_dir),
    });
    let current_season_id = upsert_media_item(
        pool,
        library_id,
        "Season",
        &current_season_name,
        &season_path,
        Some(series_id),
        Some(season_no),
        None,
        None,
        None,
        None,
        &season_metadata,
    )
    .await?;

    Ok(SeriesHierarchyRefs {
        series_id,
        series_name,
        season_id: Some(current_season_id),
        season_name: Some(current_season_name),
    })
}

fn resolve_season_identity(
    season_dir: &Path,
    series_dir: &Path,
    season_number_hint: Option<i32>,
) -> (i32, String, PathBuf) {
    let season_no = season_number_hint
        .or_else(|| parse_season_number_from_directory_name(season_dir))
        .unwrap_or(1)
        .max(0);
    if season_dir == series_dir {
        let season_name = format!("Season {season_no:02}");
        return (
            season_no,
            season_name.clone(),
            series_dir.join(&season_name),
        );
    }

    (
        season_no,
        season_display_name(season_dir, Some(season_no)),
        season_dir.to_path_buf(),
    )
}

fn should_generate_mediainfo_sidecar(has_mediainfo_file: bool, stream_url: Option<&str>) -> bool {
    if has_mediainfo_file {
        return false;
    }

    let Some(stream_url) = stream_url.map(str::trim).filter(|value| !value.is_empty()) else {
        return false;
    };

    stream_url.starts_with("http://") || stream_url.starts_with("https://")
}

async fn generate_mediainfo_sidecar(
    mediainfo_path: &Path,
    probe_target: &str,
) -> anyhow::Result<()> {
    let output = tokio::process::Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-print_format")
        .arg("json")
        .arg("-show_format")
        .arg("-show_streams")
        .arg("-show_chapters")
        .arg(probe_target)
        .output()
        .await
        .with_context(|| format!("failed to execute ffprobe: {probe_target}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "ffprobe failed for target {} with status {}: {}",
            probe_target,
            output.status,
            stderr.trim()
        );
    }

    let ffprobe_payload: Value = serde_json::from_slice(&output.stdout)
        .with_context(|| format!("failed to parse ffprobe output: {probe_target}"))?;
    let generated = build_mediainfo_from_ffprobe(&ffprobe_payload);
    let encoded =
        serde_json::to_vec_pretty(&generated).context("failed to encode mediainfo json")?;

    if let Some(parent) = mediainfo_path.parent() {
        tokio::fs::create_dir_all(parent).await.with_context(|| {
            format!("failed to create mediainfo directory: {}", parent.display())
        })?;
    }
    tokio::fs::write(mediainfo_path, encoded)
        .await
        .with_context(|| {
            format!(
                "failed to write mediainfo file: {}",
                mediainfo_path.display()
            )
        })?;

    Ok(())
}

fn build_mediainfo_from_ffprobe(ffprobe_payload: &Value) -> Value {
    let runtime_ticks = ffprobe_payload
        .get("format")
        .and_then(|format| format.get("duration"))
        .and_then(parse_duration_seconds_to_ticks);
    let bitrate = ffprobe_payload
        .get("format")
        .and_then(|format| format.get("bit_rate"))
        .and_then(value_to_i64);
    let container = ffprobe_payload
        .get("format")
        .and_then(|format| format.get("format_name"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let streams = ffprobe_payload
        .get("streams")
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let chapters = ffprobe_payload
        .get("chapters")
        .and_then(Value::as_array)
        .map(|raw| {
            raw.iter()
                .enumerate()
                .filter_map(|(index, chapter)| ffprobe_chapter_to_mediainfo(chapter, index))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    json!([
        {
            "MediaSourceInfo": {
                "RunTimeTicks": runtime_ticks,
                "Bitrate": bitrate,
                "Container": container,
                "MediaStreams": streams,
                "Chapters": chapters
            }
        }
    ])
}

fn parse_position_seconds_to_ticks(value: &Value) -> Option<i64> {
    let seconds = match value {
        Value::Number(v) => v.as_f64(),
        Value::String(v) => v.parse::<f64>().ok(),
        _ => None,
    }?;

    if !seconds.is_finite() || seconds < 0.0 {
        return None;
    }

    let ticks = seconds * RUNTIME_TICKS_PER_SECOND;
    if ticks > i64::MAX as f64 {
        return None;
    }

    Some(ticks.round() as i64)
}

fn ffprobe_chapter_to_mediainfo(chapter: &Value, fallback_index: usize) -> Option<Value> {
    let chapter_obj = chapter.as_object()?;
    let chapter_index = chapter_obj
        .get("id")
        .and_then(value_to_i64)
        .and_then(|value| i32::try_from(value).ok())
        .or_else(|| i32::try_from(fallback_index).ok())
        .unwrap_or(i32::MAX);
    let start_position_ticks = chapter_obj
        .get("start_time")
        .and_then(parse_position_seconds_to_ticks)
        .or_else(|| {
            chapter_obj
                .get("start")
                .and_then(parse_position_seconds_to_ticks)
        })
        .unwrap_or(0);
    let name = chapter_obj
        .get("tags")
        .and_then(Value::as_object)
        .and_then(|tags| tags.get("title"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("Chapter {}", chapter_index.saturating_add(1)));

    Some(json!({
        "ChapterIndex": chapter_index,
        "StartPositionTicks": start_position_ticks,
        "Name": name,
        "MarkerType": "Chapter"
    }))
}

fn parse_duration_seconds_to_ticks(value: &Value) -> Option<i64> {
    let seconds = match value {
        Value::Number(v) => v.as_f64(),
        Value::String(v) => v.parse::<f64>().ok(),
        _ => None,
    }?;

    if !seconds.is_finite() || seconds <= 0.0 {
        return None;
    }

    let ticks = seconds * RUNTIME_TICKS_PER_SECOND;
    if ticks > i64::MAX as f64 {
        return None;
    }

    Some(ticks.round() as i64)
}

async fn replace_subtitles(
    pool: &PgPool,
    item_id: Uuid,
    subtitles: Vec<std::path::PathBuf>,
) -> anyhow::Result<SubtitleReplaceResult> {
    let subtitles = build_subtitle_rows(subtitles);
    let existing: Vec<SubtitleStateRow> = sqlx::query_as(
        "SELECT path, language, is_default FROM subtitles WHERE media_item_id = $1 ORDER BY path ASC",
    )
    .bind(item_id)
    .fetch_all(pool)
    .await?;

    if existing == subtitles {
        return Ok(SubtitleReplaceResult {
            changed: false,
            inserted: subtitles.len(),
        });
    }

    sqlx::query("DELETE FROM subtitles WHERE media_item_id = $1")
        .bind(item_id)
        .execute(pool)
        .await?;

    for subtitle in &subtitles {
        sqlx::query(
            r#"
INSERT INTO subtitles (id, media_item_id, path, language, is_default)
VALUES ($1, $2, $3, $4, $5)
ON CONFLICT (path) DO NOTHING
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(item_id)
        .bind(&subtitle.path)
        .bind(&subtitle.language)
        .bind(subtitle.is_default)
        .execute(pool)
        .await?;
    }

    Ok(SubtitleReplaceResult {
        changed: true,
        inserted: subtitles.len(),
    })
}

fn normalized_candidate_title(row: &VersionGroupCandidateRow) -> Option<String> {
    let raw_title = row
        .nfo_title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(row.name.as_str());
    let normalized = normalize_media_title(raw_title).trim().to_ascii_lowercase();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn movie_version_group_key(row: &VersionGroupCandidateRow) -> Option<String> {
    let title = normalized_candidate_title(row)?;
    let folder_name = Path::new(&row.path)
        .parent()
        .and_then(Path::file_name)
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let folder = normalize_media_title(folder_name)
        .trim()
        .to_ascii_lowercase();
    if folder.len() <= 1 {
        return None;
    }
    let production_year = row.production_year.or(row.nfo_year).unwrap_or(0);

    Some(format!("movie|{folder}|{title}|{production_year}"))
}

fn episode_version_group_key(row: &VersionGroupCandidateRow) -> Option<String> {
    let series_id = row.series_id?;
    let season_number = row
        .season_number
        .or(row.metadata_season_number)
        .or(row.metadata_parent_index_number)?;
    let episode_number = row
        .episode_number
        .or(row.metadata_episode_number)
        .or(row.metadata_index_number)?;
    let title = normalized_candidate_title(row).unwrap_or_else(|| "episode".to_string());

    Some(format!(
        "episode|{series_id}|{season_number}|{episode_number}|{title}"
    ))
}

fn version_group_key(row: &VersionGroupCandidateRow) -> Option<String> {
    if row.item_type.eq_ignore_ascii_case("movie") {
        return movie_version_group_key(row);
    }
    if row.item_type.eq_ignore_ascii_case("episode") {
        return episode_version_group_key(row);
    }
    None
}

fn candidate_bitrate(row: &VersionGroupCandidateRow) -> i32 {
    row.bitrate.unwrap_or(0)
}

fn is_remote_stream_candidate(row: &VersionGroupCandidateRow) -> bool {
    let stream_url = row.stream_url.as_deref().unwrap_or_default();

    if stream_url.starts_with("http://")
        || stream_url.starts_with("https://")
        || stream_url.starts_with("gdrive://")
        || stream_url.starts_with("s3://")
        || stream_url.starts_with("lumenbackend://")
    {
        return true;
    }

    Path::new(&row.path)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("strm"))
}

fn compare_version_priority(
    left: &VersionGroupCandidateRow,
    right: &VersionGroupCandidateRow,
) -> std::cmp::Ordering {
    let left_remote = is_remote_stream_candidate(left);
    let right_remote = is_remote_stream_candidate(right);
    left_remote
        .cmp(&right_remote)
        .then_with(|| candidate_bitrate(right).cmp(&candidate_bitrate(left)))
        .then_with(|| left.path.cmp(&right.path))
        .then_with(|| left.id.cmp(&right.id))
}

fn deterministic_version_group_id(library_id: Uuid, group_key: &str) -> Uuid {
    Uuid::new_v5(&library_id, group_key.as_bytes())
}

async fn rebuild_version_groups(pool: &PgPool, library_id: Uuid) -> anyhow::Result<usize> {
    let candidates = sqlx::query_as::<_, VersionGroupCandidateRow>(
        r#"
SELECT
    id,
    item_type,
    name,
    path,
    series_id,
    season_number,
    episode_number,
    bitrate,
    stream_url,
    NULLIF(BTRIM(metadata #>> '{nfo,title}'), '') AS nfo_title,
    CASE
        WHEN COALESCE(metadata->>'production_year', '') ~ '^-?[0-9]+$'
        THEN (metadata->>'production_year')::int
        ELSE NULL
    END AS production_year,
    CASE
        WHEN COALESCE(metadata #>> '{nfo,year}', '') ~ '^-?[0-9]+$'
        THEN (metadata #>> '{nfo,year}')::int
        ELSE NULL
    END AS nfo_year,
    CASE
        WHEN COALESCE(metadata->>'season_number', '') ~ '^-?[0-9]+$'
        THEN (metadata->>'season_number')::int
        ELSE NULL
    END AS metadata_season_number,
    CASE
        WHEN COALESCE(metadata->>'parent_index_number', '') ~ '^-?[0-9]+$'
        THEN (metadata->>'parent_index_number')::int
        ELSE NULL
    END AS metadata_parent_index_number,
    CASE
        WHEN COALESCE(metadata->>'episode_number', '') ~ '^-?[0-9]+$'
        THEN (metadata->>'episode_number')::int
        ELSE NULL
    END AS metadata_episode_number,
    CASE
        WHEN COALESCE(metadata->>'index_number', '') ~ '^-?[0-9]+$'
        THEN (metadata->>'index_number')::int
        ELSE NULL
    END AS metadata_index_number
FROM media_items
WHERE library_id = $1
  AND item_type IN ('Movie', 'Episode')
        "#,
    )
    .bind(library_id)
    .fetch_all(pool)
    .await?;

    let mut tx = pool.begin().await?;
    sqlx::query(
        r#"
UPDATE media_items
SET version_group_id = NULL,
    version_rank = 0
WHERE library_id = $1
  AND item_type IN ('Movie', 'Episode')
  AND (version_group_id IS NOT NULL OR version_rank <> 0)
        "#,
    )
    .bind(library_id)
    .execute(&mut *tx)
    .await?;

    let mut groups: HashMap<String, Vec<VersionGroupCandidateRow>> = HashMap::new();
    for candidate in candidates {
        if let Some(key) = version_group_key(&candidate) {
            groups.entry(key).or_default().push(candidate);
        }
    }

    let mut grouped_items = 0_usize;
    for (key, mut group) in groups {
        if group.len() < 2 {
            continue;
        }

        group.sort_by(compare_version_priority);
        let group_id = deterministic_version_group_id(library_id, key.as_str());
        for (rank, candidate) in group.iter().enumerate() {
            sqlx::query(
                r#"
UPDATE media_items
SET version_group_id = $2,
    version_rank = $3
WHERE id = $1
                "#,
            )
            .bind(candidate.id)
            .bind(group_id)
            .bind(i32::try_from(rank).unwrap_or(i32::MAX))
            .execute(&mut *tx)
            .await?;
            grouped_items += 1;
        }
    }

    tx.commit().await?;
    Ok(grouped_items)
}

async fn merge_duplicates(pool: &PgPool, item_id: Uuid, stream_url: &str) -> anyhow::Result<bool> {
    if stream_url.trim().is_empty() {
        return Ok(false);
    }

    let duplicate_of: Option<Uuid> = sqlx::query_scalar(
        r#"
SELECT id
FROM media_items
WHERE stream_url = $1 AND id <> $2
ORDER BY updated_at DESC
LIMIT 1
        "#,
    )
    .bind(stream_url)
    .bind(item_id)
    .fetch_optional(pool)
    .await?;

    let Some(duplicate_of) = duplicate_of else {
        return Ok(false);
    };

    sqlx::query(
        r#"
UPDATE media_items
SET metadata = COALESCE(metadata, '{}'::jsonb)
    || jsonb_build_object('duplicate_of', to_jsonb($2::uuid), 'duplicate_reason', 'same_stream_url'),
    updated_at = now()
WHERE id = $1
        "#,
    )
    .bind(item_id)
    .bind(duplicate_of)
    .execute(pool)
    .await?;

    Ok(true)
}

fn parse_nfo(content: &str) -> ParsedNfo {
    let tags = dedupe_text_values_case_insensitive(
        capture_tags(content, "tag")
            .into_iter()
            .chain(capture_tags(content, "style")),
    );
    let taglines = dedupe_text_values_case_insensitive(capture_tags(content, "tagline"));

    ParsedNfo {
        title: capture_tag(content, "title"),
        overview: capture_tag(content, "plot").or_else(|| capture_tag(content, "overview")),
        year: capture_tag(content, "year").and_then(|v| v.parse::<i32>().ok()),
        official_rating: capture_tag(content, "mpaa")
            .or_else(|| capture_tag(content, "officialrating"))
            .or_else(|| capture_tag(content, "contentrating"))
            .or_else(|| capture_tag(content, "certification")),
        season_number: capture_tag(content, "season").and_then(|v| v.parse::<i32>().ok()),
        episode_number: capture_tag(content, "episode").and_then(|v| v.parse::<i32>().ok()),
        tmdb_id: capture_tag(content, "tmdbid")
            .or_else(|| capture_tag(content, "tmdb_id"))
            .or_else(|| capture_uniqueid_tmdb(content)),
        tags,
        taglines,
    }
}

fn capture_tags(content: &str, tag: &str) -> Vec<String> {
    let pattern = format!(r"(?is)<{tag}>(.*?)</{tag}>", tag = regex::escape(tag));
    let Ok(re) = Regex::new(&pattern) else {
        return Vec::new();
    };

    re.captures_iter(content)
        .filter_map(|captures| captures.get(1))
        .map(|m| {
            let s = html_unescape_minimal(m.as_str().trim());
            match s
                .strip_prefix("<![CDATA[")
                .and_then(|v| v.strip_suffix("]]>"))
            {
                Some(inner) => inner.trim().to_string(),
                None => s,
            }
        })
        .filter(|value| !value.is_empty())
        .collect()
}

fn dedupe_text_values_case_insensitive(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        let normalized = trimmed.to_ascii_lowercase();
        if seen.insert(normalized) {
            out.push(trimmed.to_string());
        }
    }
    out
}

fn capture_tag(content: &str, tag: &str) -> Option<String> {
    let pattern = format!(r"(?is)<{tag}>(.*?)</{tag}>", tag = regex::escape(tag));
    let re = Regex::new(&pattern).ok()?;
    let captures = re.captures(content)?;
    captures
        .get(1)
        .map(|m| {
            let s = html_unescape_minimal(m.as_str().trim());
            match s
                .strip_prefix("<![CDATA[")
                .and_then(|v| v.strip_suffix("]]>"))
            {
                Some(inner) => inner.trim().to_string(),
                None => s,
            }
        })
        .filter(|v| !v.is_empty())
}

fn capture_uniqueid_tmdb(content: &str) -> Option<String> {
    let re = Regex::new(r#"(?is)<uniqueid[^>]*type=["']tmdb["'][^>]*>(.*?)</uniqueid>"#).ok()?;
    let captures = re.captures(content)?;
    captures
        .get(1)
        .map(|m| m.as_str().trim().to_string())
        .filter(|v| !v.is_empty())
}

fn missing_metadata_fields(
    nfo: &ParsedNfo,
    runtime_ticks: Option<i64>,
    bitrate: Option<i32>,
    stream_url: Option<&str>,
    has_images: bool,
) -> Vec<&'static str> {
    let mut out = Vec::new();
    if nfo.title.is_none() {
        out.push("title");
    }
    if nfo.overview.is_none() {
        out.push("overview");
    }
    if runtime_ticks.is_none() {
        out.push("runtime_ticks");
    }
    if bitrate.is_none() {
        out.push("bitrate");
    }
    if nfo.tmdb_id.is_none() {
        out.push("tmdb_id");
    }
    if stream_url.map(|v| v.trim().is_empty()).unwrap_or(true) {
        out.push("stream_url");
    }
    if !has_images {
        out.push("images");
    }
    out
}

fn normalize_mediainfo(input: &Value) -> Value {
    if input.is_null() {
        return Value::Null;
    }

    if let Some(arr) = input.as_array() {
        return Value::Array(arr.clone());
    }

    if let Some(obj) = input.as_object() {
        if let Some(value) = obj.get("MediaSourceWithChapters") {
            if let Some(arr) = value.as_array() {
                return Value::Array(arr.clone());
            }
        }
        if let Some(value) = obj.get("mediaSourceWithChapters") {
            if let Some(arr) = value.as_array() {
                return Value::Array(arr.clone());
            }
        }
    }

    input.clone()
}

fn extract_runtime_ticks(mediainfo: &Value) -> Option<i64> {
    let first = mediainfo.as_array()?.first()?;

    if let Some(value) = first
        .get("RunTimeTicks")
        .or_else(|| first.get("runTimeTicks"))
    {
        return value_to_i64(value);
    }

    let media_source = first
        .get("MediaSourceInfo")
        .or_else(|| first.get("mediaSourceInfo"));
    if let Some(media_source) = media_source {
        if let Some(v) = media_source
            .get("RunTimeTicks")
            .or_else(|| media_source.get("runTimeTicks"))
        {
            return value_to_i64(v);
        }
    }

    None
}

fn extract_bitrate(mediainfo: &Value) -> Option<i64> {
    let first = mediainfo.as_array()?.first()?;

    if let Some(value) = first.get("Bitrate").or_else(|| first.get("bitrate")) {
        return value_to_i64(value);
    }

    let media_source = first
        .get("MediaSourceInfo")
        .or_else(|| first.get("mediaSourceInfo"));
    if let Some(media_source) = media_source {
        if let Some(v) = media_source
            .get("Bitrate")
            .or_else(|| media_source.get("bitrate"))
        {
            return value_to_i64(v);
        }
    }

    None
}

fn value_to_i64(value: &Value) -> Option<i64> {
    if let Some(v) = value.as_i64() {
        return Some(v);
    }
    if let Some(v) = value.as_u64() {
        return i64::try_from(v).ok();
    }
    value.as_str().and_then(|s| i64::from_str(s).ok())
}

fn detect_item_type(path: &Path, nfo: &ParsedNfo, library_type: LibraryTypeMode) -> String {
    match library_type {
        LibraryTypeMode::Movies => "Movie".to_string(),
        LibraryTypeMode::Series => "Episode".to_string(),
        LibraryTypeMode::Mixed => detect_item_type_mixed(path, nfo),
    }
}

fn detect_item_type_mixed(path: &Path, nfo: &ParsedNfo) -> String {
    if nfo.season_number.is_some() || nfo.episode_number.is_some() {
        return "Episode".to_string();
    }

    let s = path.to_string_lossy();
    let re = Regex::new(r"(?i)s\d{1,2}e\d{1,3}").ok();
    if let Some(re) = re {
        if re.is_match(&s) {
            return "Episode".to_string();
        }
    }

    "Movie".to_string()
}

fn parse_season_number(path: &Path) -> Option<i32> {
    let s = path.to_string_lossy();
    if let Some(re) = Regex::new(r"(?i)s(\d{1,2})e\d{1,3}").ok()
        && let Some(cap) = re.captures(&s)
    {
        return cap.get(1)?.as_str().parse::<i32>().ok();
    }

    parse_season_number_from_directory_name(path.parent()?)
}

fn parse_season_number_from_directory_name(dir: &Path) -> Option<i32> {
    let name = dir.file_name()?.to_string_lossy();
    if let Some(re) = Regex::new(r"(?i)season[\s._-]*(\d{1,2})").ok()
        && let Some(cap) = re.captures(&name)
    {
        return cap.get(1)?.as_str().parse::<i32>().ok();
    }
    if let Some(re) = Regex::new(r"(?i)^s(\d{1,2})$").ok()
        && let Some(cap) = re.captures(&name)
    {
        return cap.get(1)?.as_str().parse::<i32>().ok();
    }
    None
}

fn parse_episode_number(path: &Path) -> Option<i32> {
    let s = path.to_string_lossy();
    let re = Regex::new(r"(?i)s\d{1,2}e(\d{1,3})").ok()?;
    let cap = re.captures(&s)?;
    cap.get(1)?.as_str().parse::<i32>().ok()
}

fn derive_series_id(path: &Path) -> Option<Uuid> {
    let parent = path.parent()?.parent().or_else(|| path.parent())?;
    let series_name = parent.file_name()?.to_string_lossy();
    Some(Uuid::new_v5(&Uuid::NAMESPACE_URL, series_name.as_bytes()))
}

#[cfg(test)]
fn find_subtitles(path: &Path, subtitle_exts: &[String]) -> Vec<std::path::PathBuf> {
    let subtitle_ext_lookup = build_subtitle_ext_lookup(subtitle_exts);
    let mut subtitle_dir_cache = SubtitleDirCache::default();
    find_subtitles_with_cache(path, &subtitle_ext_lookup, &mut subtitle_dir_cache)
}

fn find_subtitles_with_cache(
    path: &Path,
    subtitle_ext_lookup: &HashSet<String>,
    subtitle_dir_cache: &mut SubtitleDirCache,
) -> Vec<PathBuf> {
    let Some(dir) = path.parent() else {
        return Vec::new();
    };
    let stem = path
        .file_stem()
        .and_then(|v| v.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let stem_tokens = subtitle_name_tokens(stem.as_str());
    let episode_regex = Regex::new(r"(?i)s\d{1,2}e\d{1,3}").ok();
    let stem_episode_marker = episode_regex
        .as_ref()
        .and_then(|regex| regex.find(stem.as_str()))
        .map(|capture| capture.as_str().to_ascii_lowercase());
    let media_ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase());
    let is_strm_media = media_ext.as_deref() == Some("strm");
    let entries = subtitle_dir_cache.directory_entries(dir);
    let single_media_file_in_dir = media_ext.as_ref().is_some_and(|ext| {
        entries
            .iter()
            .filter(|entry| entry.ext_lower == *ext)
            .count()
            == 1
    });

    let mut all_subtitles = Vec::new();
    let mut strict_matches = Vec::new();
    for entry in entries {
        if !subtitle_ext_lookup.contains(&entry.ext_lower) {
            continue;
        }
        all_subtitles.push(entry.path.clone());
        if subtitle_matches_media_stem(
            &stem,
            &stem_tokens,
            stem_episode_marker.as_deref(),
            episode_regex.as_ref(),
            entry,
        ) {
            strict_matches.push(entry.path.clone());
        }
    }

    let mut out = if !strict_matches.is_empty() {
        strict_matches
    } else if is_strm_media && single_media_file_in_dir {
        all_subtitles
    } else {
        Vec::new()
    };

    out.sort();
    out.dedup();
    out
}

fn subtitle_matches_media_stem(
    media_stem: &str,
    media_tokens: &[String],
    media_episode_marker: Option<&str>,
    episode_regex: Option<&Regex>,
    subtitle_entry: &SubtitleDirEntry,
) -> bool {
    let subtitle_stem = subtitle_entry
        .path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if subtitle_stem.is_empty() {
        return false;
    }

    if subtitle_stem == media_stem {
        return true;
    }
    if subtitle_has_media_stem_prefix(media_stem, &subtitle_stem) {
        return true;
    }

    let media_stem_has_digit = media_stem.chars().any(|ch| ch.is_ascii_digit());
    if !media_tokens.is_empty() {
        let subtitle_tokens = subtitle_name_tokens(subtitle_stem.as_str());
        if subtitle_tokens.len() >= media_tokens.len()
            && subtitle_tokens
                .iter()
                .take(media_tokens.len())
                .eq(media_tokens.iter())
            && subtitle_extra_tokens_allowed(
                &subtitle_tokens[media_tokens.len()..],
                media_stem_has_digit,
            )
        {
            return true;
        }
    }

    if let (Some(media_episode_marker), Some(episode_regex)) = (media_episode_marker, episode_regex)
        && let Some(subtitle_episode_marker) = episode_regex
            .find(subtitle_stem.as_str())
            .map(|capture| capture.as_str().to_ascii_lowercase())
        && subtitle_episode_marker == media_episode_marker
    {
        return true;
    }

    false
}

fn subtitle_has_media_stem_prefix(media_stem: &str, subtitle_stem: &str) -> bool {
    if !subtitle_stem.starts_with(media_stem) {
        return false;
    }
    let remainder = &subtitle_stem[media_stem.len()..];
    if remainder.is_empty() {
        return true;
    }

    let Some(prefix) = remainder.chars().next() else {
        return true;
    };
    if !matches!(prefix, '.' | ' ' | '-' | '_' | '[' | '(') {
        return false;
    }

    let normalized = remainder
        .trim_start_matches(|ch: char| matches!(ch, '.' | ' ' | '-' | '_' | '[' | ']' | '(' | ')'));
    if normalized.is_empty() {
        return true;
    }

    let media_stem_has_digit = media_stem.chars().any(|ch| ch.is_ascii_digit());
    let extra_tokens = normalized
        .split(|ch: char| matches!(ch, '.' | ' ' | '-' | '_' | '[' | ']' | '(' | ')'))
        .filter(|segment| !segment.is_empty())
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>();
    subtitle_extra_tokens_allowed(&extra_tokens, media_stem_has_digit)
}

fn subtitle_name_tokens(value: &str) -> Vec<String> {
    value
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.to_ascii_lowercase())
        .collect()
}

fn subtitle_extra_tokens_allowed(tokens: &[String], media_stem_has_digit: bool) -> bool {
    tokens.iter().all(|token| {
        subtitle_token_is_alpha_tag(token)
            || subtitle_token_is_release_tag(token, media_stem_has_digit)
    })
}

fn subtitle_token_is_alpha_tag(token: &str) -> bool {
    !token.is_empty()
        && token.len() <= 16
        && token
            .chars()
            .all(|ch| ch.is_ascii_alphabetic() || matches!(ch, '-' | '_'))
}

fn subtitle_token_is_release_tag(token: &str, media_stem_has_digit: bool) -> bool {
    if token.is_empty()
        || token.len() > 16
        || !token
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
    {
        return false;
    }

    let has_digit = token.chars().any(|ch| ch.is_ascii_digit());
    if !has_digit {
        return false;
    }
    media_stem_has_digit || !token.chars().all(|ch| ch.is_ascii_digit())
}

impl SubtitleDirCache {
    fn directory_entries(&mut self, dir: &Path) -> &[SubtitleDirEntry] {
        use std::collections::hash_map::Entry;

        match self.entries_by_dir.entry(dir.to_path_buf()) {
            Entry::Occupied(entry) => entry.into_mut().as_slice(),
            Entry::Vacant(entry) => {
                let files = std::fs::read_dir(dir)
                    .ok()
                    .into_iter()
                    .flat_map(|entries| entries.flatten())
                    .filter_map(|entry| {
                        let path = entry.path();
                        if !path.is_file() {
                            return None;
                        }
                        let ext_lower = path.extension()?.to_str()?.to_ascii_lowercase();
                        Some(SubtitleDirEntry { path, ext_lower })
                    })
                    .collect::<Vec<_>>();
                entry.insert(files).as_slice()
            }
        }
    }
}

fn find_images(path: &Path) -> Vec<String> {
    let Some(dir) = path.parent() else {
        return Vec::new();
    };

    let stem = path
        .file_stem()
        .and_then(|v| v.to_str())
        .unwrap_or_default();

    find_images_in_dir(dir, stem)
}

fn find_directory_images(dir: &Path) -> Vec<String> {
    let stem = dir.file_name().and_then(|v| v.to_str()).unwrap_or_default();
    find_images_in_dir(dir, stem)
}

fn find_images_in_dir(dir: &Path, stem: &str) -> Vec<String> {
    let mut out = Vec::new();
    for name in [
        format!("{}.jpg", stem),
        format!("{}.jpeg", stem),
        format!("{}.png", stem),
        "poster.jpg".to_string(),
        "folder.jpg".to_string(),
        "fanart.jpg".to_string(),
        "thumb.jpg".to_string(),
    ] {
        let p = dir.join(name);
        if p.exists() {
            out.push(p.to_string_lossy().to_string());
        }
    }

    out.sort();
    out.dedup();
    out
}

fn directory_display_name(dir: &Path, fallback: &str) -> String {
    let from_dir = dir
        .file_name()
        .and_then(|v| v.to_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(normalize_media_title)
        .filter(|v| !v.is_empty());
    if let Some(value) = from_dir {
        return value;
    }

    let fallback_clean = normalize_media_title(fallback.trim());
    if fallback_clean.is_empty() {
        fallback.trim().to_string()
    } else {
        fallback_clean
    }
}

fn season_display_name(season_dir: &Path, season_number: Option<i32>) -> String {
    let from_dir = directory_display_name(season_dir, "");
    if !from_dir.is_empty() {
        return from_dir;
    }

    if let Some(season_number) = season_number {
        return format!("Season {:02}", season_number.max(0));
    }
    "Season".to_string()
}

fn resolve_item_name(parsed_nfo_title: Option<&str>, stem: &str) -> String {
    let raw = parsed_nfo_title
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or(stem);
    let cleaned = normalize_media_title(raw);
    if cleaned.is_empty() {
        "Media".to_string()
    } else {
        cleaned
    }
}

fn infer_subtitle_language(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_string_lossy().to_lowercase();
    let tokens = name
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();

    for token in tokens {
        let lang = match token {
            "zh" | "zho" | "chi" | "chs" | "cht" | "cn" | "chinese" => Some("zh"),
            "en" | "eng" | "english" => Some("en"),
            "ja" | "jp" | "jpn" | "japanese" => Some("ja"),
            "ko" | "kr" | "kor" | "korean" => Some("ko"),
            "es" | "spa" | "spanish" => Some("es"),
            "fr" | "fra" | "fre" | "french" => Some("fr"),
            _ => None,
        };
        if let Some(lang) = lang {
            return Some(lang.to_string());
        }
    }

    None
}

fn infer_subtitle_default(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or_default();
    let lower = name.to_lowercase();
    lower.contains(".default.") || lower.contains(" default") || lower.contains("[default]")
}

fn html_unescape_minimal(input: &str) -> String {
    input
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

#[derive(Debug, sqlx::FromRow)]
struct ItemPathRow {
    id: Uuid,
    path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
struct SubtitleStateRow {
    path: String,
    language: Option<String>,
    is_default: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct SubtitleReplaceResult {
    changed: bool,
    inserted: usize,
}

fn build_subtitle_rows(subtitles: Vec<PathBuf>) -> Vec<SubtitleStateRow> {
    let mut out = Vec::new();
    let mut has_default_subtitle = false;

    for (idx, subtitle) in subtitles.into_iter().enumerate() {
        let language = infer_subtitle_language(&subtitle);
        let mut is_default = infer_subtitle_default(&subtitle);
        if !has_default_subtitle && idx == 0 {
            is_default = true;
        }
        if is_default {
            has_default_subtitle = true;
        }
        out.push(SubtitleStateRow {
            path: subtitle.to_string_lossy().to_string(),
            language,
            is_default,
        });
    }

    out
}

#[cfg(test)]
mod tests {
    use super::{
        LibraryTypeMode, ParsedNfo, ScanExistingItemPolicy, ScanProbePolicy,
        VersionGroupCandidateRow, build_mediainfo_from_ffprobe, capture_tag,
        compare_version_priority, detect_item_type, directory_display_name, extract_bitrate,
        extract_runtime_ticks, find_subtitles, infer_subtitle_default, infer_subtitle_language,
        missing_metadata_fields, normalize_mediainfo, parse_duration_seconds_to_ticks,
        parse_episode_number, parse_nfo, parse_season_number, parse_stream_url_from_strm_content,
        resolve_item_name, resolve_season_identity, season_display_name,
        should_generate_mediainfo_sidecar, should_probe_mediainfo_sidecar, should_scan_incremental,
        should_skip_existing_item, version_group_key,
    };
    use chrono::{Duration, Utc};
    use serde_json::json;
    use std::{collections::HashSet, path::Path};
    use uuid::Uuid;

    #[test]
    fn parse_nfo_reads_core_fields() {
        let content = "<movie><title>My &amp; Movie</title><plot>x</plot><tmdbid>1</tmdbid><mpaa>TV-MA</mpaa></movie>";
        let parsed = parse_nfo(content);
        assert_eq!(parsed.title.as_deref(), Some("My & Movie"));
        assert_eq!(parsed.overview.as_deref(), Some("x"));
        assert_eq!(parsed.tmdb_id.as_deref(), Some("1"));
        assert_eq!(parsed.official_rating.as_deref(), Some("TV-MA"));
        assert_eq!(capture_tag(content, "title").as_deref(), Some("My & Movie"));
    }

    #[test]
    fn parse_nfo_strips_cdata_from_plot() {
        let content = r#"<movie><plot><![CDATA[冰雪奇缘简介]]></plot></movie>"#;
        let parsed = parse_nfo(content);
        assert_eq!(parsed.overview.as_deref(), Some("冰雪奇缘简介"));
    }

    #[test]
    fn parse_nfo_reads_tags_and_styles_as_tags() {
        let content = r#"
<movie>
  <tag>历史</tag>
  <tag> 历史 </tag>
  <style><![CDATA[战争]]></style>
  <style>History</style>
  <tagline>史诗巨作</tagline>
</movie>
        "#;
        let parsed = parse_nfo(content);
        assert_eq!(
            parsed.tags,
            vec![
                "历史".to_string(),
                "战争".to_string(),
                "History".to_string()
            ]
        );
        assert_eq!(parsed.taglines, vec!["史诗巨作".to_string()]);
    }

    #[test]
    fn extract_runtime_and_bitrate_from_mediainfo() {
        let info = json!([
            {
                "MediaSourceInfo": {
                    "RunTimeTicks": 72000000,
                    "Bitrate": 5000000
                }
            }
        ]);

        let normalized = normalize_mediainfo(&info);
        assert_eq!(extract_runtime_ticks(&normalized), Some(72_000_000));
        assert_eq!(extract_bitrate(&normalized), Some(5_000_000));
    }

    #[test]
    fn infer_subtitle_language_from_filename() {
        assert_eq!(
            infer_subtitle_language(Path::new("movie.zh.ass")).as_deref(),
            Some("zh")
        );
        assert_eq!(
            infer_subtitle_language(Path::new("movie.chi.ass")).as_deref(),
            Some("zh")
        );
        assert_eq!(
            infer_subtitle_language(Path::new("movie.english.srt")).as_deref(),
            Some("en")
        );
    }

    #[test]
    fn infer_default_subtitle_flag_from_filename() {
        assert!(infer_subtitle_default(Path::new("movie.default.ass")));
        assert!(infer_subtitle_default(Path::new("movie [default].srt")));
        assert!(!infer_subtitle_default(Path::new("movie.zh.ass")));
    }

    #[test]
    fn find_subtitles_accepts_resolution_and_language_suffixes() {
        let temp = tempfile::tempdir().expect("tempdir");
        let media = temp.path().join("幸福终点站 (2004).strm");
        let subtitle_a = temp.path().join("幸福终点站 (2004) - 2160p.chi.srt");
        let subtitle_b = temp.path().join("幸福终点站 (2004).default.ass");
        let subtitle_other = temp.path().join("幸福终点站 (2005).chi.srt");
        std::fs::write(&media, "https://example.com/video").expect("write media");
        std::fs::write(&subtitle_a, "1\n00:00:00,000 --> 00:00:01,000\nhello")
            .expect("write subtitle a");
        std::fs::write(&subtitle_b, "1\n00:00:00,000 --> 00:00:01,000\nhello")
            .expect("write subtitle b");
        std::fs::write(&subtitle_other, "1\n00:00:00,000 --> 00:00:01,000\nhello")
            .expect("write subtitle other");

        let subtitles = find_subtitles(&media, &["srt".to_string(), "ass".to_string()]);
        assert!(subtitles.contains(&subtitle_a));
        assert!(subtitles.contains(&subtitle_b));
        assert!(!subtitles.contains(&subtitle_other));
    }

    #[test]
    fn find_subtitles_matches_episode_marker_when_titles_differ() {
        let temp = tempfile::tempdir().expect("tempdir");
        let media = temp.path().join("少年骇客.S01E03.strm");
        let subtitle_match = temp.path().join("Ben10.S01E03.chi.srt");
        let subtitle_other = temp.path().join("Ben10.S01E04.chi.srt");
        std::fs::write(&media, "https://example.com/video").expect("write media");
        std::fs::write(&subtitle_match, "1\n00:00:00,000 --> 00:00:01,000\nhello")
            .expect("write subtitle match");
        std::fs::write(&subtitle_other, "1\n00:00:00,000 --> 00:00:01,000\nhello")
            .expect("write subtitle other");

        let subtitles = find_subtitles(&media, &["srt".to_string()]);
        assert_eq!(subtitles, vec![subtitle_match]);
    }

    #[test]
    fn find_subtitles_falls_back_when_directory_has_single_media_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        let media = temp.path().join("粗野派 (2024).strm");
        let subtitle = temp.path().join("The.Brutalist.2024.WEB-DL.ass");
        std::fs::write(&media, "https://example.com/video").expect("write media");
        std::fs::write(&subtitle, "1\n00:00:00,000 --> 00:00:01,000\nhello")
            .expect("write subtitle");

        let subtitles = find_subtitles(&media, &["ass".to_string()]);
        assert_eq!(subtitles, vec![subtitle]);
    }

    #[test]
    fn find_subtitles_does_not_fallback_with_multiple_media_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        let media_a = temp.path().join("movie-a.strm");
        let media_b = temp.path().join("movie-b.strm");
        let subtitle = temp.path().join("The.Brutalist.2024.WEB-DL.ass");
        std::fs::write(&media_a, "https://example.com/video-a").expect("write media a");
        std::fs::write(&media_b, "https://example.com/video-b").expect("write media b");
        std::fs::write(&subtitle, "1\n00:00:00,000 --> 00:00:01,000\nhello")
            .expect("write subtitle");

        let subtitles = find_subtitles(&media_a, &["ass".to_string()]);
        assert!(subtitles.is_empty());
    }

    #[test]
    fn missing_metadata_checker_marks_required_fields() {
        let parsed = parse_nfo("<movie><title>A</title></movie>");
        let missing = missing_metadata_fields(&parsed, None, Some(1), Some("http://x"), false);
        assert!(missing.contains(&"overview"));
        assert!(missing.contains(&"runtime_ticks"));
        assert!(missing.contains(&"tmdb_id"));
        assert!(missing.contains(&"images"));
    }

    #[test]
    fn detect_item_type_and_episode_numbers_from_filename() {
        let path = Path::new("/library/Show.Name.S01E02.strm");
        let parsed = ParsedNfo::default();
        assert_eq!(
            detect_item_type(path, &parsed, LibraryTypeMode::Mixed),
            "Episode"
        );
        assert_eq!(parse_season_number(path), Some(1));
        assert_eq!(parse_episode_number(path), Some(2));
    }

    #[test]
    fn detect_item_type_respects_library_type_mode() {
        let path = Path::new("/library/Movie.2024.strm");
        let parsed = ParsedNfo::default();
        assert_eq!(
            detect_item_type(path, &parsed, LibraryTypeMode::Movies),
            "Movie"
        );
        assert_eq!(
            detect_item_type(path, &parsed, LibraryTypeMode::Series),
            "Episode"
        );
    }

    #[test]
    fn parse_season_number_from_season_directory_name() {
        let path = Path::new("/library/Show Name/Season 03/episode.strm");
        assert_eq!(parse_season_number(path), Some(3));
    }

    #[test]
    fn should_scan_incremental_respects_threshold() {
        let temp = tempfile::tempdir().expect("tempdir");
        let file = temp.path().join("movie.strm");
        std::fs::write(&file, "https://example.com/video").expect("write strm");
        let subtitle_exts = vec!["srt".to_string()];

        assert!(should_scan_incremental(&file, None, &subtitle_exts).expect("scan none threshold"));

        let past_threshold = Utc::now() - Duration::seconds(60);
        assert!(
            should_scan_incremental(&file, Some(past_threshold), &subtitle_exts)
                .expect("scan past")
        );

        let future_threshold = Utc::now() + Duration::seconds(60);
        assert!(
            !should_scan_incremental(&file, Some(future_threshold), &subtitle_exts)
                .expect("scan future")
        );
    }

    #[test]
    fn should_scan_incremental_when_subtitle_changed() {
        let temp = tempfile::tempdir().expect("tempdir");
        let file = temp.path().join("movie.strm");
        let subtitle = temp.path().join("movie.zh.srt");
        let subtitle_exts = vec!["srt".to_string()];
        std::fs::write(&file, "https://example.com/video").expect("write strm");

        let threshold = Utc::now() + Duration::seconds(1);
        std::thread::sleep(std::time::Duration::from_millis(1200));
        std::fs::write(&subtitle, "1\n00:00:00,000 --> 00:00:01,000\nhello")
            .expect("write subtitle");

        assert!(
            should_scan_incremental(&file, Some(threshold), &subtitle_exts)
                .expect("scan on subtitle change")
        );
    }

    #[test]
    fn should_generate_mediainfo_sidecar_only_for_new_item_with_missing_file() {
        assert!(should_generate_mediainfo_sidecar(
            false,
            Some("https://example.com/video")
        ));
        assert!(!should_generate_mediainfo_sidecar(
            true,
            Some("https://example.com/video")
        ));
        assert!(!should_generate_mediainfo_sidecar(false, Some("   ")));
        assert!(!should_generate_mediainfo_sidecar(
            false,
            Some("gdrive://x")
        ));
    }

    #[test]
    fn should_skip_existing_item_when_policy_is_skip() {
        let mut existing_paths = HashSet::new();
        existing_paths.insert("/library/movie-a.strm".to_string());

        assert!(should_skip_existing_item(
            ScanExistingItemPolicy::Skip,
            "/library/movie-a.strm",
            &existing_paths
        ));
        assert!(!should_skip_existing_item(
            ScanExistingItemPolicy::Skip,
            "/library/movie-b.strm",
            &existing_paths
        ));
        assert!(!should_skip_existing_item(
            ScanExistingItemPolicy::Upsert,
            "/library/movie-a.strm",
            &existing_paths
        ));
    }

    #[test]
    fn should_probe_mediainfo_sidecar_respects_probe_policy() {
        assert!(should_probe_mediainfo_sidecar(
            ScanProbePolicy::Enabled,
            false,
            Some("https://example.com/video")
        ));
        assert!(!should_probe_mediainfo_sidecar(
            ScanProbePolicy::Disabled,
            false,
            Some("https://example.com/video")
        ));
    }

    #[test]
    fn parse_stream_url_from_strm_content_skips_comment_lines() {
        let content = "#EXTM3U\n#EXTINF:-1,Movie\nhttps://example.com/video.mp4\n";
        assert_eq!(
            parse_stream_url_from_strm_content(content).as_deref(),
            Some("https://example.com/video.mp4")
        );
    }

    #[test]
    fn parse_stream_url_from_strm_content_falls_back_to_first_non_comment_line() {
        let content = "# comment\n/mnt/media/movie.mkv\nhttps://example.com/video.mp4\n";
        assert_eq!(
            parse_stream_url_from_strm_content(content).as_deref(),
            Some("https://example.com/video.mp4")
        );

        let content = "# comment\n/mnt/media/movie.mkv\n";
        assert_eq!(
            parse_stream_url_from_strm_content(content).as_deref(),
            Some("/mnt/media/movie.mkv")
        );
    }

    #[test]
    fn build_mediainfo_from_ffprobe_maps_runtime_and_bitrate() {
        let ffprobe_payload = json!({
            "format": {
                "duration": "7.2",
                "bit_rate": "5000000",
                "format_name": "matroska,webm"
            },
            "streams": [
                { "codec_type": "video", "bit_rate": "4800000" },
                { "codec_type": "audio", "bit_rate": "192000" }
            ],
            "chapters": [
                { "id": 0, "start_time": "0.000000", "tags": { "title": "Intro" } },
                { "id": 1, "start_time": "2.500000", "tags": { "title": "Act 1" } }
            ]
        });

        let generated = build_mediainfo_from_ffprobe(&ffprobe_payload);
        let normalized = normalize_mediainfo(&generated);
        assert_eq!(extract_runtime_ticks(&normalized), Some(72_000_000));
        assert_eq!(extract_bitrate(&normalized), Some(5_000_000));
        let chapters = normalized[0]["MediaSourceInfo"]["Chapters"]
            .as_array()
            .expect("mediainfo chapters");
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].get("StartPositionTicks"), Some(&json!(0)));
        assert_eq!(chapters[0].get("ChapterIndex"), Some(&json!(0)));
        assert_eq!(chapters[0].get("Name"), Some(&json!("Intro")));
        assert_eq!(chapters[0].get("MarkerType"), Some(&json!("Chapter")));
        assert_eq!(
            chapters[1].get("StartPositionTicks"),
            Some(&json!(25_000_000))
        );
        assert_eq!(chapters[1].get("ChapterIndex"), Some(&json!(1)));
    }

    #[test]
    fn parse_duration_seconds_to_ticks_handles_invalid_values() {
        assert_eq!(
            parse_duration_seconds_to_ticks(&json!("3.5")),
            Some(35_000_000)
        );
        assert_eq!(parse_duration_seconds_to_ticks(&json!(0)), None);
        assert_eq!(parse_duration_seconds_to_ticks(&json!(-1)), None);
        assert_eq!(parse_duration_seconds_to_ticks(&json!("bad")), None);
    }

    #[test]
    fn resolve_item_name_strips_year_and_resolution_tokens() {
        assert_eq!(
            resolve_item_name(None, "疯狂海盗团2 (2025) - 2160p"),
            "疯狂海盗团2"
        );
        assert_eq!(
            resolve_item_name(Some("Movie.Title.2025.1080p"), "ignored"),
            "Movie Title"
        );
    }

    #[test]
    fn directory_display_name_uses_cleaned_directory_name() {
        let dir = Path::new("/tmp/疯狂海盗团2 (2025) - 2160p");
        assert_eq!(directory_display_name(dir, "Series"), "疯狂海盗团2");
    }

    #[test]
    fn season_display_name_falls_back_when_directory_only_contains_noise() {
        let dir = Path::new("/tmp/2025 - 2160p");
        assert_eq!(season_display_name(dir, Some(2)), "Season 02");
    }

    #[test]
    fn resolve_season_identity_uses_real_season_directory_when_present() {
        let season_dir = Path::new("/tmp/show/Season 01");
        let series_dir = Path::new("/tmp/show");
        let (season_no, season_name, season_path) =
            resolve_season_identity(season_dir, series_dir, Some(1));
        assert_eq!(season_no, 1);
        assert_eq!(season_name, "Season 01");
        assert_eq!(season_path, season_dir);
    }

    #[test]
    fn resolve_season_identity_synthesizes_stable_season_path_for_flat_layout() {
        let series_dir = Path::new("/tmp/show-flat");
        let (season_no, season_name, season_path) =
            resolve_season_identity(series_dir, series_dir, None);
        assert_eq!(season_no, 1);
        assert_eq!(season_name, "Season 01");
        assert_eq!(season_path, series_dir.join("Season 01"));
    }

    #[test]
    fn version_group_key_groups_movie_versions_in_same_folder() {
        let first = VersionGroupCandidateRow {
            id: Uuid::new_v4(),
            item_type: "Movie".to_string(),
            name: "Movie 1080p".to_string(),
            path: "/library/Movie/Movie 1080p.strm".to_string(),
            series_id: None,
            season_number: None,
            episode_number: None,
            bitrate: Some(3_000_000),
            stream_url: Some("https://cdn.example.com/movie-1080p.mkv".to_string()),
            nfo_title: None,
            production_year: Some(2025),
            nfo_year: None,
            metadata_season_number: None,
            metadata_parent_index_number: None,
            metadata_episode_number: None,
            metadata_index_number: None,
        };
        let second = VersionGroupCandidateRow {
            id: Uuid::new_v4(),
            item_type: "Movie".to_string(),
            name: "Movie 2160p".to_string(),
            path: "/library/Movie/Movie 2160p.strm".to_string(),
            series_id: None,
            season_number: None,
            episode_number: None,
            bitrate: Some(5_000_000),
            stream_url: Some("https://cdn.example.com/movie-2160p.mkv".to_string()),
            nfo_title: None,
            production_year: Some(2025),
            nfo_year: None,
            metadata_season_number: None,
            metadata_parent_index_number: None,
            metadata_episode_number: None,
            metadata_index_number: None,
        };

        assert_eq!(version_group_key(&first), version_group_key(&second));
    }

    #[test]
    fn version_group_key_groups_episode_versions_by_series_and_number() {
        let series_id = Uuid::new_v4();
        let first = VersionGroupCandidateRow {
            id: Uuid::new_v4(),
            item_type: "Episode".to_string(),
            name: "Pilot 1080p".to_string(),
            path: "/library/Show/Season 01/Show S01E01 1080p.strm".to_string(),
            series_id: Some(series_id),
            season_number: Some(1),
            episode_number: Some(1),
            bitrate: Some(2_000_000),
            stream_url: Some("https://cdn.example.com/ep1-1080p.mkv".to_string()),
            nfo_title: None,
            production_year: None,
            nfo_year: None,
            metadata_season_number: None,
            metadata_parent_index_number: None,
            metadata_episode_number: None,
            metadata_index_number: None,
        };
        let second = VersionGroupCandidateRow {
            id: Uuid::new_v4(),
            item_type: "Episode".to_string(),
            name: "Pilot 4K".to_string(),
            path: "/library/Show/Season 01/Show S01E01 2160p.strm".to_string(),
            series_id: Some(series_id),
            season_number: Some(1),
            episode_number: Some(1),
            bitrate: Some(4_000_000),
            stream_url: Some("https://cdn.example.com/ep1-2160p.mkv".to_string()),
            nfo_title: None,
            production_year: None,
            nfo_year: None,
            metadata_season_number: None,
            metadata_parent_index_number: None,
            metadata_episode_number: None,
            metadata_index_number: None,
        };

        assert_eq!(version_group_key(&first), version_group_key(&second));
    }

    #[test]
    fn compare_version_priority_prefers_local_streams() {
        let local = VersionGroupCandidateRow {
            id: Uuid::new_v4(),
            item_type: "Movie".to_string(),
            name: "Movie local".to_string(),
            path: "/library/Movie/Movie-local.mkv".to_string(),
            series_id: None,
            season_number: None,
            episode_number: None,
            bitrate: Some(4_000_000),
            stream_url: Some("/mnt/media/movie-local.mkv".to_string()),
            nfo_title: None,
            production_year: None,
            nfo_year: None,
            metadata_season_number: None,
            metadata_parent_index_number: None,
            metadata_episode_number: None,
            metadata_index_number: None,
        };
        let remote = VersionGroupCandidateRow {
            id: Uuid::new_v4(),
            item_type: "Movie".to_string(),
            name: "Movie remote".to_string(),
            path: "/library/Movie/Movie-remote.strm".to_string(),
            series_id: None,
            season_number: None,
            episode_number: None,
            bitrate: Some(8_000_000),
            stream_url: Some("https://cdn.example.com/movie-remote.mkv".to_string()),
            nfo_title: None,
            production_year: None,
            nfo_year: None,
            metadata_season_number: None,
            metadata_parent_index_number: None,
            metadata_episode_number: None,
            metadata_index_number: None,
        };

        assert!(compare_version_priority(&local, &remote).is_lt());
    }

    // ── find_subtitles tests ──────────────────────────────────────────

    fn subtitle_exts() -> Vec<String> {
        vec!["srt".into(), "ass".into(), "ssa".into(), "sub".into()]
    }

    #[test]
    fn find_subtitles_basic_match() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Movie.mkv"), b"").unwrap();
        std::fs::write(dir.path().join("Movie.chi.srt"), b"").unwrap();
        std::fs::write(dir.path().join("Movie.eng.srt"), b"").unwrap();
        std::fs::write(dir.path().join("Movie.srt"), b"").unwrap();

        let result = find_subtitles(&dir.path().join("Movie.mkv"), &subtitle_exts());
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn find_subtitles_rejects_overlapping_stem() {
        // "Movie.2024.chi.srt" should NOT match stem "Movie"
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Movie.mkv"), b"").unwrap();
        std::fs::write(dir.path().join("Movie.2024.mkv"), b"").unwrap();
        std::fs::write(dir.path().join("Movie.chi.srt"), b"").unwrap();
        std::fs::write(dir.path().join("Movie.2024.chi.srt"), b"").unwrap();

        let for_movie = find_subtitles(&dir.path().join("Movie.mkv"), &subtitle_exts());
        assert_eq!(
            for_movie.len(),
            1,
            "Movie.mkv should only match Movie.chi.srt"
        );
        assert!(for_movie[0].to_str().unwrap().contains("Movie.chi.srt"));

        let for_movie_2024 = find_subtitles(&dir.path().join("Movie.2024.mkv"), &subtitle_exts());
        assert_eq!(for_movie_2024.len(), 1);
        assert!(
            for_movie_2024[0]
                .to_str()
                .unwrap()
                .contains("Movie.2024.chi.srt")
        );
    }

    #[test]
    fn find_subtitles_allows_forced_flag() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Movie.mkv"), b"").unwrap();
        std::fs::write(dir.path().join("Movie.forced.chi.srt"), b"").unwrap();

        let result = find_subtitles(&dir.path().join("Movie.mkv"), &subtitle_exts());
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn find_subtitles_allows_locale_code() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Movie.mkv"), b"").unwrap();
        std::fs::write(dir.path().join("Movie.zh-CN.srt"), b"").unwrap();

        let result = find_subtitles(&dir.path().join("Movie.mkv"), &subtitle_exts());
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn find_subtitles_ignores_unrelated_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Movie.mkv"), b"").unwrap();
        std::fs::write(dir.path().join("Other.chi.srt"), b"").unwrap();
        std::fs::write(dir.path().join("Movie.nfo"), b"").unwrap();

        let result = find_subtitles(&dir.path().join("Movie.mkv"), &subtitle_exts());
        assert!(result.is_empty());
    }
}
