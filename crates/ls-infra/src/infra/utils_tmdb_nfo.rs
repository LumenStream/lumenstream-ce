fn subtitle_codec_from_path(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
        .unwrap_or_else(|| "srt".to_string())
}

fn image_candidates(dir: &Path, stem: &str, image_type: &str) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();

    let mut push_existing = |name: &str| {
        let p = dir.join(name);
        if p.exists() {
            files.push(p);
        }
    };

    match image_type.to_ascii_lowercase().as_str() {
        "primary" | "poster" => {
            push_existing(&format!("{stem}.jpg"));
            push_existing(&format!("{stem}.jpeg"));
            push_existing(&format!("{stem}.png"));
            push_existing(&format!("{stem}.webp"));
            push_existing("poster.jpg");
            push_existing("folder.jpg");
            push_existing("cover.jpg");
            if let Some(season_number) = season_number_from_stem(stem) {
                for ext in ["jpg", "jpeg", "png", "webp"] {
                    push_existing(&format!("season{season_number:02}.{ext}"));
                    push_existing(&format!("season{season_number:02}-poster.{ext}"));
                    push_existing(&format!("Season {season_number:02}.{ext}"));
                    push_existing(&format!("Season {season_number:02}-poster.{ext}"));
                }
            }
        }
        "thumb" | "thumbnail" => {
            push_existing(&format!("{stem}-thumb.jpg"));
            push_existing(&format!("{stem}.thumb.jpg"));
            push_existing("thumb.jpg");
        }
        "backdrop" | "fanart" => {
            push_existing("fanart.jpg");
            push_existing(&format!("{stem}-fanart.jpg"));
            push_existing(&format!("{stem}.fanart.jpg"));
        }
        "logo" => {
            for ext in ["png", "webp", "jpg", "jpeg"] {
                push_existing(&format!("logo.{ext}"));
                push_existing(&format!("clearlogo.{ext}"));
                push_existing(&format!("{stem}-logo.{ext}"));
                push_existing(&format!("{stem}.logo.{ext}"));
            }
        }
        _ => {
            push_existing(&format!("{stem}.jpg"));
            push_existing(&format!("{stem}.jpeg"));
            push_existing(&format!("{stem}.png"));
        }
    }

    files.sort();
    files.dedup();
    files
}

fn season_number_from_stem(stem: &str) -> Option<i32> {
    let stem = stem.trim();
    if stem.is_empty() {
        return None;
    }
    if let Some(re) = Regex::new(r"(?i)s(\d{1,2})e\d{1,3}").ok()
        && let Some(caps) = re.captures(stem)
    {
        return caps.get(1)?.as_str().parse::<i32>().ok();
    }
    if let Some(re) = Regex::new(r"(?i)^season[\s._-]*(\d{1,2})$").ok()
        && let Some(caps) = re.captures(stem)
    {
        return caps.get(1)?.as_str().parse::<i32>().ok();
    }
    if let Some(re) = Regex::new(r"(?i)^s(\d{1,2})$").ok()
        && let Some(caps) = re.captures(stem)
    {
        return caps.get(1)?.as_str().parse::<i32>().ok();
    }
    None
}

/// Read `<tmdbid>` from an NFO file. Returns `None` if the file doesn't exist
/// or doesn't contain a valid numeric tmdb id.
fn read_nfo_tmdb_id(nfo_path: &Path) -> Option<i64> {
    ls_scraper::read_nfo_sidecar_hints(nfo_path)?
        .external_ids
        .tmdb
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|&id| id > 0)
}

/// Read `<imdbid>` (or `<imdb_id>`) from an NFO file. Returns `None` if the
/// file doesn't exist or doesn't contain a valid IMDB id (tt-prefixed).
fn read_nfo_imdb_id(nfo_path: &Path) -> Option<String> {
    ls_scraper::read_nfo_sidecar_hints(nfo_path)?
        .external_ids
        .imdb
        .filter(|value| value.starts_with("tt") && value.len() > 2)
}

/// Try to resolve a TMDB ID for a movie from local NFO files.
/// Checks: `{stem}.nfo`, then `movie.nfo` in the same directory.
fn resolve_movie_nfo_tmdb_id(media_path: &Path) -> Option<i64> {
    if let Some(id) = read_nfo_tmdb_id(&media_path.with_extension("nfo")) {
        return Some(id);
    }
    let dir = media_path.parent()?;
    read_nfo_tmdb_id(&dir.join("movie.nfo"))
}

/// Try to resolve an IMDB ID for a movie from local NFO files.
fn resolve_movie_nfo_imdb_id(media_path: &Path) -> Option<String> {
    if let Some(id) = read_nfo_imdb_id(&media_path.with_extension("nfo")) {
        return Some(id);
    }
    let dir = media_path.parent()?;
    read_nfo_imdb_id(&dir.join("movie.nfo"))
}

/// Try to resolve a TMDB ID for a TV series from local NFO files.
/// Checks tvshow.nfo / show.nfo / folder.nfo in the series directory.
fn resolve_series_nfo_tmdb_id(series_dir: &Path) -> Option<i64> {
    for candidate in ["tvshow.nfo", "show.nfo", "folder.nfo"] {
        if let Some(id) = read_nfo_tmdb_id(&series_dir.join(candidate)) {
            return Some(id);
        }
    }
    None
}

/// Try to resolve an IMDB ID for a TV series from local NFO files.
fn resolve_series_nfo_imdb_id(series_dir: &Path) -> Option<String> {
    for candidate in ["tvshow.nfo", "show.nfo", "folder.nfo"] {
        if let Some(id) = read_nfo_imdb_id(&series_dir.join(candidate)) {
            return Some(id);
        }
    }
    None
}

fn metadata_string_missing(metadata: &Value, key: &str) -> bool {
    match metadata.get(key) {
        None | Some(Value::Null) => true,
        Some(Value::String(s)) => s.trim().is_empty(),
        Some(Value::Number(_)) => false,
        _ => true,
    }
}

fn metadata_people_need_refresh(metadata: &Value) -> bool {
    let Some(people) = metadata.get("people").and_then(Value::as_array) else {
        return true;
    };
    if people.is_empty() {
        return true;
    }
    people.iter().any(|person| {
        person
            .as_object()
            .and_then(|obj| obj.get("id"))
            .and_then(Value::as_str)
            .map(|id| id.trim().is_empty())
            .unwrap_or(true)
    })
}

fn metadata_backdrop_tags_missing(metadata: &Value) -> bool {
    let Some(tags) = metadata.get("backdrop_image_tags").and_then(Value::as_array) else {
        return true;
    };
    !tags.iter().any(|tag| {
        tag.as_str()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
    })
}

fn metadata_tags_missing(metadata: &Value) -> bool {
    !metadata
        .get("tags")
        .and_then(Value::as_array)
        .map(|tags| {
            tags.iter().any(|tag| {
                tag.as_str()
                    .map(|value| !value.trim().is_empty())
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn normalize_tmdb_match_title(value: &str) -> Option<String> {
    let normalized = crate::search::normalize_media_title(value);
    let compact = normalized
        .chars()
        .flat_map(char::to_lowercase)
        .filter(|ch| ch.is_alphanumeric())
        .collect::<String>();
    if compact.is_empty() {
        None
    } else {
        Some(compact)
    }
}

fn value_year_hint(value: &Value) -> Option<i32> {
    value
        .as_i64()
        .and_then(|raw| i32::try_from(raw).ok())
        .or_else(|| value.as_u64().and_then(|raw| i32::try_from(raw).ok()))
        .or_else(|| value.as_str().and_then(parse_year_from_date))
}

fn metadata_tmdb_binding_is_manual(metadata: &Value) -> bool {
    metadata
        .get("tmdb_binding_source")
        .and_then(Value::as_str)
        .is_some_and(|value| value.eq_ignore_ascii_case("manual"))
}

fn metadata_tmdb_movie_conflicts_local_hints(metadata: &Value) -> bool {
    if metadata_tmdb_binding_is_manual(metadata) {
        return false;
    }

    let nfo = metadata.get("nfo");
    let local_title = nfo
        .and_then(|value| value.get("title"))
        .and_then(Value::as_str)
        .and_then(normalize_tmdb_match_title);
    let local_year = nfo
        .and_then(|value| value.get("year"))
        .and_then(value_year_hint);
    let tmdb_raw = metadata.get("tmdb_raw");
    let tmdb_title = tmdb_raw
        .and_then(|value| value.get("title").or_else(|| value.get("name")))
        .and_then(Value::as_str)
        .and_then(normalize_tmdb_match_title);
    let tmdb_original_title = tmdb_raw
        .and_then(|value| value.get("original_title").or_else(|| value.get("original_name")))
        .and_then(Value::as_str)
        .and_then(normalize_tmdb_match_title);
    let tmdb_year = tmdb_raw
        .and_then(|value| value.get("release_date").or_else(|| value.get("first_air_date")))
        .and_then(Value::as_str)
        .and_then(parse_year_from_date);

    let title_conflict = if let Some(local_title) = local_title.as_deref() {
        let title_match = tmdb_title.as_deref().is_some_and(|value| {
            value == local_title
                || (value.len() >= 4
                    && (value.contains(local_title) || local_title.contains(value)))
        }) || tmdb_original_title.as_deref().is_some_and(|value| {
            value == local_title
                || (value.len() >= 4
                    && (value.contains(local_title) || local_title.contains(value)))
        });
        !title_match
    } else {
        false
    };
    if !title_conflict {
        return false;
    }

    match (local_year, tmdb_year) {
        (Some(local_year), Some(tmdb_year)) => (local_year - tmdb_year).abs() >= 2,
        // If either side lacks reliable year hints, still recheck wrong-title bindings.
        _ => true,
    }
}

fn should_fill_tmdb_for_item(item_type: &str, item_path: &str, metadata: &Value) -> bool {
    let item_type = item_type.trim().to_ascii_lowercase();
    if !matches!(item_type.as_str(), "movie" | "episode" | "series") {
        return false;
    }

    if metadata_string_missing(metadata, "overview")
        || metadata_string_missing(metadata, "tmdb_id")
        || metadata_people_need_refresh(metadata)
        || metadata_string_missing(metadata, "primary_image_tag")
        || metadata_tags_missing(metadata)
    {
        return true;
    }

    if matches!(item_type.as_str(), "movie" | "series")
        && metadata_string_missing(metadata, "logo_image_tag")
    {
        return true;
    }

    if matches!(item_type.as_str(), "movie" | "series") && metadata_backdrop_tags_missing(metadata)
    {
        return true;
    }
    if item_type == "movie" && metadata_tmdb_movie_conflicts_local_hints(metadata) {
        return true;
    }

    let item_path = Path::new(item_path);
    if item_type == "series" {
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

        return image_candidates(series_dir, stem, "primary").is_empty()
            || image_candidates(series_dir, stem, "backdrop").is_empty()
            || image_candidates(series_dir, stem, "logo").is_empty();
    }

    let dir = item_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = item_path
        .file_stem()
        .and_then(|v| v.to_str())
        .unwrap_or_default();
    if item_type == "episode" {
        return image_candidates(dir, stem, "thumb").is_empty();
    }

    image_candidates(dir, stem, "primary").is_empty()
        || image_candidates(dir, stem, "backdrop").is_empty()
        || (matches!(item_type.as_str(), "movie" | "series")
            && image_candidates(dir, stem, "logo").is_empty())
}

fn infer_season_episode_from_path(path: &Path) -> Option<(i32, i32)> {
    let path_str = path.to_string_lossy();
    let re = Regex::new(r"(?i)s(\d{1,2})e(\d{1,3})").ok()?;
    let caps = re.captures(&path_str)?;
    let season = caps.get(1)?.as_str().parse::<i32>().ok()?;
    let episode = caps.get(2)?.as_str().parse::<i32>().ok()?;
    Some((season, episode))
}

fn parse_year_from_date(value: &str) -> Option<i32> {
    let prefix = value.trim().get(..4)?;
    prefix.parse::<i32>().ok()
}

fn normalize_tmdb_tags(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for value in values {
        let normalized = value.trim();
        if normalized.is_empty() {
            continue;
        }
        let key = normalized.to_ascii_lowercase();
        if seen.insert(key) {
            out.push(normalized.to_string());
        }
    }
    out
}

fn extract_tmdb_keywords(payload: &Value) -> Vec<String> {
    let values = payload
        .get("keywords")
        .or_else(|| payload.get("results"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("name").and_then(Value::as_str))
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    normalize_tmdb_tags(values)
}

fn merge_tmdb_tags_into_metadata(metadata: &mut Value, tmdb_tags: &[String]) {
    if tmdb_tags.is_empty() {
        return;
    }

    let existing_tags = metadata
        .get("tags")
        .and_then(Value::as_array)
        .map(|tags| {
            tags.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let merged = normalize_tmdb_tags(existing_tags.into_iter().chain(tmdb_tags.iter().cloned()));
    if merged.is_empty() {
        return;
    }
    if let Some(object) = metadata.as_object_mut() {
        object.insert("tags".to_string(), json!(merged));
    }
}

fn extract_tmdb_genres(details: &Value) -> Vec<String> {
    details
        .get("genres")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("name").and_then(Value::as_str).map(str::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn extract_tmdb_studios(details: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    if let Some(companies) = details
        .get("production_companies")
        .and_then(Value::as_array)
    {
        for item in companies {
            if let Some(name) = item.get("name").and_then(Value::as_str) {
                out.push(json!({ "name": name }));
            }
        }
    }
    if out.is_empty() {
        if let Some(networks) = details.get("networks").and_then(Value::as_array) {
            for item in networks {
                if let Some(name) = item.get("name").and_then(Value::as_str) {
                    out.push(json!({ "name": name }));
                }
            }
        }
    }
    out
}

fn extract_tmdb_people(credits: Option<&Value>, cast_limit: usize) -> Vec<TmdbPersonCandidate> {
    let Some(credits) = credits else {
        return Vec::new();
    };

    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::<(i64, String)>::new();

    if let Some(cast) = credits.get("cast").and_then(Value::as_array) {
        for (idx, item) in cast.iter().take(cast_limit).enumerate() {
            let Some(tmdb_id) = item.get("id").and_then(Value::as_i64) else {
                continue;
            };
            let Some(name) = item.get("name").and_then(Value::as_str) else {
                continue;
            };
            let person_type = "Actor".to_string();
            if !seen.insert((tmdb_id, person_type.clone())) {
                continue;
            }
            out.push(TmdbPersonCandidate {
                tmdb_id,
                name: name.to_string(),
                person_type,
                role: item
                    .get("character")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                profile_path: item
                    .get("profile_path")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                sort_order: idx as i32,
            });
        }
    }

    if let Some(crew) = credits.get("crew").and_then(Value::as_array) {
        let mut idx = out.len() as i32;
        for item in crew {
            let Some(tmdb_id) = item.get("id").and_then(Value::as_i64) else {
                continue;
            };
            let Some(name) = item.get("name").and_then(Value::as_str) else {
                continue;
            };
            let job = item
                .get("job")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_default();
            let person_type = if job.eq_ignore_ascii_case("Director") {
                Some("Director")
            } else if matches!(job.as_str(), "Writer" | "Screenplay" | "Story" | "Teleplay") {
                Some("Writer")
            } else {
                None
            };
            let Some(person_type) = person_type else {
                continue;
            };
            let person_type = person_type.to_string();
            if !seen.insert((tmdb_id, person_type.clone())) {
                continue;
            }
            out.push(TmdbPersonCandidate {
                tmdb_id,
                name: name.to_string(),
                person_type,
                role: if job.is_empty() { None } else { Some(job) },
                profile_path: item
                    .get("profile_path")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                sort_order: idx,
            });
            idx += 1;
        }
    }

    out
}

fn is_missing_value(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(v) => v.trim().is_empty(),
        Value::Array(v) => v.is_empty(),
        Value::Object(v) => v.is_empty(),
        _ => false,
    }
}

fn merge_missing_json(mut base: Value, patch: &Value) -> Value {
    merge_missing_json_in_place(&mut base, patch);
    base
}

fn merge_missing_json_in_place(base: &mut Value, patch: &Value) {
    match (base, patch) {
        (Value::Object(base_obj), Value::Object(patch_obj)) => {
            for (key, patch_value) in patch_obj {
                match base_obj.get_mut(key) {
                    Some(base_value) => {
                        if is_missing_value(base_value) {
                            *base_value = patch_value.clone();
                        } else {
                            merge_missing_json_in_place(base_value, patch_value);
                        }
                    }
                    None => {
                        base_obj.insert(key.clone(), patch_value.clone());
                    }
                }
            }
        }
        (base_value, patch_value) => {
            if is_missing_value(base_value) {
                *base_value = patch_value.clone();
            }
        }
    }
}

fn image_file_tag(path: &Path) -> Option<String> {
    let metadata = std::fs::metadata(path).ok()?;
    let modified = metadata
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some(auth::hash_api_key(&format!(
        "{}:{modified}",
        path.display()
    )))
}

fn resolve_tvshow_nfo_path(series_dir: &Path) -> PathBuf {
    for candidate in ["tvshow.nfo", "show.nfo", "folder.nfo"] {
        let path = series_dir.join(candidate);
        if path.exists() {
            return path;
        }
    }
    series_dir.join("tvshow.nfo")
}

fn resolve_season_nfo_path(season_dir: &Path, season_number: i32) -> PathBuf {
    let preferred = season_dir.join("season.nfo");
    if preferred.exists() {
        return preferred;
    }
    let lower = season_dir.join(format!("season{:02}.nfo", season_number.max(0)));
    if lower.exists() {
        return lower;
    }
    let title_case = season_dir.join(format!("Season {:02}.nfo", season_number.max(0)));
    if title_case.exists() {
        return title_case;
    }
    preferred
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn normalize_nfo_tag_value(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let normalized = match trimmed
        .strip_prefix("<![CDATA[")
        .and_then(|v| v.strip_suffix("]]>"))
    {
        Some(inner) => inner.trim(),
        None => trimmed,
    };
    if normalized.is_empty() {
        None
    } else {
        Some(normalized.to_string())
    }
}

fn capture_nfo_tag(content: &str, tag: &str) -> Option<String> {
    let pattern = format!(r"(?is)<{tag}>(.*?)</{tag}>", tag = regex::escape(tag));
    let re = Regex::new(&pattern).ok()?;
    let captures = re.captures(content)?;
    captures
        .get(1)
        .and_then(|matched| normalize_nfo_tag_value(matched.as_str()))
}

fn capture_nfo_tags(content: &str, tag: &str) -> Vec<String> {
    let pattern = format!(r"(?is)<{tag}>(.*?)</{tag}>", tag = regex::escape(tag));
    let Ok(re) = Regex::new(&pattern) else {
        return Vec::new();
    };
    re.captures_iter(content)
        .filter_map(|caps| caps.get(1).map(|m| m.as_str().trim().to_string()))
        .filter(|v| !v.is_empty())
        .collect()
}

fn metadata_string(metadata: &Value, key: &str) -> Option<String> {
    metadata
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
        .or_else(|| {
            metadata
                .get("nfo")
                .and_then(|v| v.get(key))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(str::to_string)
        })
}

fn metadata_i64(metadata: &Value, key: &str) -> Option<i64> {
    metadata.get(key).and_then(Value::as_i64).or_else(|| {
        metadata
            .get("nfo")
            .and_then(|v| v.get(key))
            .and_then(Value::as_i64)
    })
}

fn metadata_provider_id(metadata: &Value, provider_key: &str) -> Option<String> {
    metadata
        .get("provider_ids")
        .and_then(|value| value.get(provider_key))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn extract_nfo_studios(metadata: &Value) -> Vec<String> {
    metadata
        .get("studios")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|s| {
                    s.get("name")
                        .and_then(Value::as_str)
                        .or_else(|| s.as_str())
                        .map(str::to_string)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn write_movie_nfo(path: &Path, metadata: &Value) -> anyhow::Result<()> {
    let existing = std::fs::read_to_string(path).ok();
    let title = existing
        .as_deref()
        .and_then(|raw| capture_nfo_tag(raw, "title"))
        .or_else(|| metadata_string(metadata, "sort_name"))
        .or_else(|| metadata_string(metadata, "name"))
        .or_else(|| metadata_string(metadata, "title"));
    let plot = existing
        .as_deref()
        .and_then(|raw| capture_nfo_tag(raw, "plot"))
        .or_else(|| metadata_string(metadata, "overview"));
    let year = existing
        .as_deref()
        .and_then(|raw| capture_nfo_tag(raw, "year"))
        .or_else(|| metadata_i64(metadata, "production_year").map(|v| v.to_string()));
    // Prefer metadata tmdb_id so repaired bindings can overwrite stale sidecar IDs.
    let tmdb_id = metadata_i64(metadata, "tmdb_id")
        .map(|v| v.to_string())
        .or_else(|| existing.as_deref().and_then(|raw| capture_nfo_tag(raw, "tmdbid")));
    let tvdb_id = metadata_provider_id(metadata, "Tvdb")
        .or_else(|| existing.as_deref().and_then(|raw| capture_nfo_tag(raw, "tvdbid")));
    let imdb_id = metadata_provider_id(metadata, "Imdb")
        .or_else(|| existing.as_deref().and_then(|raw| capture_nfo_tag(raw, "imdbid")));
    let genres = existing
        .as_deref()
        .map(|raw| capture_nfo_tags(raw, "genre"))
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| {
            metadata
                .get("genres")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        });
    let tags = existing
        .as_deref()
        .map(|raw| capture_nfo_tags(raw, "tag"))
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| {
            metadata
                .get("tags")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        });
    let mpaa = existing
        .as_deref()
        .and_then(|raw| capture_nfo_tag(raw, "mpaa"))
        .or_else(|| metadata_string(metadata, "official_rating"))
        .or_else(|| metadata_string(metadata, "mpaa"))
        .or_else(|| metadata_string(metadata, "content_rating"))
        .or_else(|| metadata_string(metadata, "certification"));

    if title.is_none() && plot.is_none() && tmdb_id.is_none() && tvdb_id.is_none() {
        return Ok(());
    }

    let mut lines = vec!["<movie>".to_string()];
    if let Some(title) = title {
        lines.push(format!("  <title>{}</title>", xml_escape(&title)));
    }
    if let Some(plot) = plot {
        lines.push(format!("  <plot>{}</plot>", xml_escape(&plot)));
    }
    if let Some(year) = year {
        lines.push(format!("  <year>{}</year>", xml_escape(&year)));
    }
    if let Some(tmdb_id) = tmdb_id {
        lines.push(format!("  <tmdbid>{}</tmdbid>", xml_escape(&tmdb_id)));
        lines.push(format!(
            "  <uniqueid type=\"tmdb\" default=\"true\">{}</uniqueid>",
            xml_escape(&tmdb_id)
        ));
    }
    if let Some(tvdb_id) = tvdb_id {
        lines.push(format!("  <tvdbid>{}</tvdbid>", xml_escape(&tvdb_id)));
        lines.push(format!(
            "  <uniqueid type=\"tvdb\" default=\"false\">{}</uniqueid>",
            xml_escape(&tvdb_id)
        ));
    }
    if let Some(imdb_id) = imdb_id {
        lines.push(format!("  <imdbid>{}</imdbid>", xml_escape(&imdb_id)));
        lines.push(format!(
            "  <uniqueid type=\"imdb\" default=\"false\">{}</uniqueid>",
            xml_escape(&imdb_id)
        ));
    }
    for genre in genres {
        lines.push(format!("  <genre>{}</genre>", xml_escape(&genre)));
    }
    for tag in tags {
        lines.push(format!("  <tag>{}</tag>", xml_escape(&tag)));
    }
    for studio in extract_nfo_studios(metadata) {
        lines.push(format!("  <studio>{}</studio>", xml_escape(&studio)));
    }
    if let Some(r) = metadata.get("community_rating").and_then(Value::as_f64) {
        lines.push(format!("  <rating>{:.1}</rating>", r));
    }
    if let Some(mpaa) = mpaa {
        lines.push(format!("  <mpaa>{}</mpaa>", xml_escape(&mpaa)));
    }
    lines.push("</movie>".to_string());

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create nfo dir: {}", parent.display()))?;
    }
    std::fs::write(path, lines.join("\n"))
        .with_context(|| format!("failed to write movie nfo: {}", path.display()))?;
    Ok(())
}

fn write_tvshow_nfo(path: &Path, metadata: &Value) -> anyhow::Result<()> {
    let existing = std::fs::read_to_string(path).ok();
    let title = existing
        .as_deref()
        .and_then(|raw| capture_nfo_tag(raw, "title"))
        .or_else(|| metadata_string(metadata, "series_name"));
    let plot = existing
        .as_deref()
        .and_then(|raw| capture_nfo_tag(raw, "plot"))
        .or_else(|| metadata_string(metadata, "overview"));
    let year = existing
        .as_deref()
        .and_then(|raw| capture_nfo_tag(raw, "year"))
        .or_else(|| metadata_i64(metadata, "production_year").map(|v| v.to_string()));
    let tmdb_id = existing
        .as_deref()
        .and_then(|raw| capture_nfo_tag(raw, "tmdbid"))
        .or_else(|| metadata_i64(metadata, "series_tmdb_id").map(|v| v.to_string()))
        .or_else(|| metadata_i64(metadata, "tmdb_id").map(|v| v.to_string()));
    let tvdb_id = metadata_provider_id(metadata, "Tvdb")
        .or_else(|| existing.as_deref().and_then(|raw| capture_nfo_tag(raw, "tvdbid")));
    let imdb_id = metadata_provider_id(metadata, "Imdb")
        .or_else(|| existing.as_deref().and_then(|raw| capture_nfo_tag(raw, "imdbid")));
    let tags = existing
        .as_deref()
        .map(|raw| capture_nfo_tags(raw, "tag"))
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| {
            metadata
                .get("tags")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        });
    let mpaa = existing
        .as_deref()
        .and_then(|raw| capture_nfo_tag(raw, "mpaa"))
        .or_else(|| metadata_string(metadata, "official_rating"))
        .or_else(|| metadata_string(metadata, "mpaa"))
        .or_else(|| metadata_string(metadata, "content_rating"))
        .or_else(|| metadata_string(metadata, "certification"));

    if title.is_none() && plot.is_none() && tmdb_id.is_none() && tvdb_id.is_none() {
        return Ok(());
    }

    let mut lines = vec!["<tvshow>".to_string()];
    if let Some(title) = title {
        lines.push(format!("  <title>{}</title>", xml_escape(&title)));
    }
    if let Some(plot) = plot {
        lines.push(format!("  <plot>{}</plot>", xml_escape(&plot)));
    }
    if let Some(year) = year {
        lines.push(format!("  <year>{}</year>", xml_escape(&year)));
    }
    if let Some(tmdb_id) = tmdb_id {
        lines.push(format!("  <tmdbid>{}</tmdbid>", xml_escape(&tmdb_id)));
        lines.push(format!(
            "  <uniqueid type=\"tmdb\" default=\"true\">{}</uniqueid>",
            xml_escape(&tmdb_id)
        ));
    }
    if let Some(tvdb_id) = tvdb_id {
        lines.push(format!("  <tvdbid>{}</tvdbid>", xml_escape(&tvdb_id)));
        lines.push(format!(
            "  <uniqueid type=\"tvdb\" default=\"false\">{}</uniqueid>",
            xml_escape(&tvdb_id)
        ));
    }
    if let Some(imdb_id) = imdb_id {
        lines.push(format!("  <imdbid>{}</imdbid>", xml_escape(&imdb_id)));
        lines.push(format!(
            "  <uniqueid type=\"imdb\" default=\"false\">{}</uniqueid>",
            xml_escape(&imdb_id)
        ));
    }
    let genres = metadata
        .get("genres")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for genre in genres {
        lines.push(format!("  <genre>{}</genre>", xml_escape(&genre)));
    }
    for tag in tags {
        lines.push(format!("  <tag>{}</tag>", xml_escape(&tag)));
    }
    for studio in extract_nfo_studios(metadata) {
        lines.push(format!("  <studio>{}</studio>", xml_escape(&studio)));
    }
    if let Some(r) = metadata.get("community_rating").and_then(Value::as_f64) {
        lines.push(format!("  <rating>{:.1}</rating>", r));
    }
    if let Some(mpaa) = mpaa {
        lines.push(format!("  <mpaa>{}</mpaa>", xml_escape(&mpaa)));
    }
    lines.push("</tvshow>".to_string());

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create nfo dir: {}", parent.display()))?;
    }
    std::fs::write(path, lines.join("\n"))
        .with_context(|| format!("failed to write tvshow nfo: {}", path.display()))?;
    Ok(())
}

fn write_season_nfo(path: &Path, metadata: &Value, season_number: i32) -> anyhow::Result<()> {
    let existing = std::fs::read_to_string(path).ok();
    let title = existing
        .as_deref()
        .and_then(|raw| capture_nfo_tag(raw, "title"))
        .or_else(|| metadata_string(metadata, "season_name"))
        .or_else(|| Some(format!("Season {}", season_number.max(0))));
    let plot = existing
        .as_deref()
        .and_then(|raw| capture_nfo_tag(raw, "plot"))
        .or_else(|| metadata_string(metadata, "overview"));

    let mut lines = vec!["<season>".to_string()];
    if let Some(title) = title {
        lines.push(format!("  <title>{}</title>", xml_escape(&title)));
    }
    lines.push(format!("  <season>{}</season>", season_number.max(0)));
    if let Some(plot) = plot {
        lines.push(format!("  <plot>{}</plot>", xml_escape(&plot)));
    }
    lines.push("</season>".to_string());

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create nfo dir: {}", parent.display()))?;
    }
    std::fs::write(path, lines.join("\n"))
        .with_context(|| format!("failed to write season nfo: {}", path.display()))?;
    Ok(())
}

fn write_episode_nfo(
    path: &Path,
    metadata: &Value,
    season_number: Option<i32>,
    episode_number: Option<i32>,
) -> anyhow::Result<()> {
    let existing = std::fs::read_to_string(path).ok();
    let title = existing
        .as_deref()
        .and_then(|raw| capture_nfo_tag(raw, "title"))
        .or_else(|| metadata_string(metadata, "title"))
        .or_else(|| metadata_string(metadata, "sort_name"));
    let showtitle = existing
        .as_deref()
        .and_then(|raw| capture_nfo_tag(raw, "showtitle"))
        .or_else(|| metadata_string(metadata, "series_name"));
    let plot = existing
        .as_deref()
        .and_then(|raw| capture_nfo_tag(raw, "plot"))
        .or_else(|| metadata_string(metadata, "overview"));
    let aired = existing
        .as_deref()
        .and_then(|raw| capture_nfo_tag(raw, "aired"))
        .or_else(|| metadata_string(metadata, "premiere_date"));
    let tmdb_id = existing
        .as_deref()
        .and_then(|raw| capture_nfo_tag(raw, "tmdbid"))
        .or_else(|| metadata_i64(metadata, "episode_tmdb_id").map(|v| v.to_string()))
        .or_else(|| metadata_i64(metadata, "tmdb_id").map(|v| v.to_string()));
    let tvdb_id = metadata_provider_id(metadata, "Tvdb")
        .or_else(|| existing.as_deref().and_then(|raw| capture_nfo_tag(raw, "tvdbid")));
    let imdb_id = metadata_provider_id(metadata, "Imdb")
        .or_else(|| existing.as_deref().and_then(|raw| capture_nfo_tag(raw, "imdbid")));
    let tags = existing
        .as_deref()
        .map(|raw| capture_nfo_tags(raw, "tag"))
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| {
            metadata
                .get("tags")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        });

    let mut lines = vec!["<episodedetails>".to_string()];
    if let Some(title) = title {
        lines.push(format!("  <title>{}</title>", xml_escape(&title)));
    }
    if let Some(showtitle) = showtitle {
        lines.push(format!(
            "  <showtitle>{}</showtitle>",
            xml_escape(&showtitle)
        ));
    }
    if let Some(plot) = plot {
        lines.push(format!("  <plot>{}</plot>", xml_escape(&plot)));
    }
    if let Some(season) = season_number {
        lines.push(format!("  <season>{season}</season>"));
    }
    if let Some(episode) = episode_number {
        lines.push(format!("  <episode>{episode}</episode>"));
    }
    if let Some(aired) = aired {
        lines.push(format!("  <aired>{}</aired>", xml_escape(&aired)));
    }
    if let Some(tmdb_id) = tmdb_id {
        lines.push(format!("  <tmdbid>{}</tmdbid>", xml_escape(&tmdb_id)));
        lines.push(format!(
            "  <uniqueid type=\"tmdb\" default=\"true\">{}</uniqueid>",
            xml_escape(&tmdb_id)
        ));
    }
    if let Some(tvdb_id) = tvdb_id {
        lines.push(format!("  <tvdbid>{}</tvdbid>", xml_escape(&tvdb_id)));
        lines.push(format!(
            "  <uniqueid type=\"tvdb\" default=\"false\">{}</uniqueid>",
            xml_escape(&tvdb_id)
        ));
    }
    if let Some(imdb_id) = imdb_id {
        lines.push(format!("  <imdbid>{}</imdbid>", xml_escape(&imdb_id)));
        lines.push(format!(
            "  <uniqueid type=\"imdb\" default=\"false\">{}</uniqueid>",
            xml_escape(&imdb_id)
        ));
    }
    let genres = metadata
        .get("genres")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for genre in genres {
        lines.push(format!("  <genre>{}</genre>", xml_escape(&genre)));
    }
    for tag in tags {
        lines.push(format!("  <tag>{}</tag>", xml_escape(&tag)));
    }
    lines.push("</episodedetails>".to_string());

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create nfo dir: {}", parent.display()))?;
    }
    std::fs::write(path, lines.join("\n"))
        .with_context(|| format!("failed to write episode nfo: {}", path.display()))?;
    Ok(())
}
