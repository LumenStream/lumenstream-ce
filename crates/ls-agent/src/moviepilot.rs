use anyhow::Context;
use chrono::Utc;
use ls_config::{AgentMoviePilotConfig, AgentMoviePilotFilterConfig};
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::{debug, warn};

use crate::provider::{
    AgentProviderCapability, AgentProviderDescriptor, AgentProviderHealthReport,
    AgentProviderStatus,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MoviePilotResponse {
    #[serde(default)]
    pub success: bool,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub data: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MoviePilotContext {
    #[serde(default)]
    pub meta_info: Option<MoviePilotMediaInfo>,
    #[serde(default)]
    pub media_info: Option<MoviePilotMediaInfo>,
    pub torrent_info: MoviePilotTorrentInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MoviePilotMediaInfo {
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub en_title: String,
    #[serde(default)]
    pub year: String,
    #[serde(default)]
    pub title_year: String,
    #[serde(default)]
    pub tmdb_id: i64,
    #[serde(default)]
    pub imdb_id: String,
    #[serde(default)]
    pub tvdb_id: String,
    #[serde(default)]
    pub season: i32,
    #[serde(default)]
    pub original_title: String,
    #[serde(default)]
    pub release_date: String,
    #[serde(default)]
    pub backdrop_path: String,
    #[serde(default)]
    pub poster_path: String,
    #[serde(default)]
    pub overview: String,
    #[serde(default)]
    pub first_air_date: String,
    #[serde(default)]
    pub original_name: String,
    #[serde(default)]
    pub number_of_episodes: i32,
    #[serde(default)]
    pub number_of_seasons: i32,
    #[serde(default)]
    pub resource_pix: String,
    #[serde(default)]
    pub resource_type: String,
    #[serde(default)]
    pub video_encode: String,
    #[serde(default)]
    pub season_episode: String,
    #[serde(default)]
    pub total_episode: i32,
    #[serde(default)]
    pub begin_episode: i32,
    #[serde(default)]
    pub end_episode: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MoviePilotTorrentInfo {
    #[serde(default)]
    pub site: i32,
    #[serde(default)]
    pub site_name: String,
    #[serde(default)]
    pub site_cookie: String,
    #[serde(default)]
    pub site_ua: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub enclosure: String,
    #[serde(default)]
    pub size: f64,
    #[serde(default)]
    pub seeders: i32,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub downloadvolumefactor: f64,
    #[serde(default)]
    pub freedate: String,
    #[serde(default)]
    pub freedate_diff: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MoviePilotSubscriptionPayload {
    #[serde(default)]
    pub id: i32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub year: String,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub tmdbid: i64,
    #[serde(default)]
    pub season: i32,
    #[serde(default)]
    pub poster: String,
    #[serde(default)]
    pub backdrop: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub total_episode: i32,
    #[serde(default)]
    pub lack_episode: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MoviePilotDownloadPayload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_in: Option<MoviePilotMediaInfo>,
    pub torrent_in: MoviePilotTorrentInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoviePilotFilterDecision {
    pub filtered: Vec<MoviePilotContext>,
    pub raw_only: bool,
}

#[derive(Debug, Clone)]
pub struct MoviePilotClient {
    base_url: String,
    username: String,
    password: String,
    timeout_seconds: i64,
    client: Client,
    access_token: Option<String>,
    token_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MoviePilotProvider {
    config: AgentMoviePilotConfig,
    client: MoviePilotClient,
}

impl MoviePilotClient {
    pub fn new(
        base_url: &str,
        username: &str,
        password: &str,
        timeout_seconds: i64,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            username: username.to_string(),
            password: password.to_string(),
            timeout_seconds,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(timeout_seconds.max(1) as u64))
                .build()
                .context("failed to build moviepilot http client")?,
            access_token: None,
            token_type: None,
        })
    }

    pub fn is_configured(&self) -> bool {
        !self.base_url.is_empty() && !self.username.is_empty() && !self.password.is_empty()
    }

    pub async fn authenticate(&mut self) -> anyhow::Result<()> {
        let endpoint = format!("{}/api/v1/login/access-token", self.base_url);
        let response = self
            .client
            .post(endpoint)
            .header("Accept", "application/json")
            .form(&[
                ("username", self.username.as_str()),
                ("password", self.password.as_str()),
            ])
            .send()
            .await
            .context("moviepilot authentication request failed")?;
        let response = ensure_success(response).await?;
        let payload: Value = response
            .json()
            .await
            .context("failed to decode moviepilot auth response")?;
        self.access_token = payload
            .get("access_token")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        self.token_type = payload
            .get("token_type")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        if self.access_token.is_none() {
            anyhow::bail!("moviepilot auth missing access_token");
        }
        Ok(())
    }

    async fn ensure_auth(&mut self) -> anyhow::Result<String> {
        if self.access_token.is_none() {
            self.authenticate().await?;
        }
        Ok(format!(
            "{} {}",
            self.token_type
                .clone()
                .unwrap_or_else(|| "Bearer".to_string()),
            self.access_token.clone().unwrap_or_default()
        ))
    }

    pub async fn search_by_tmdb(
        &mut self,
        tmdb_id: i64,
        season: Option<i32>,
    ) -> anyhow::Result<MoviePilotResponse> {
        let mut url = format!("{}/api/v1/search/media/tmdb:{tmdb_id}", self.base_url);
        if let Some(season) = season.filter(|value| *value > 0) {
            url.push_str(&format!("?season={season}"));
        }
        self.get_json(url).await
    }

    pub async fn search_by_title(&mut self, title: &str) -> anyhow::Result<MoviePilotResponse> {
        let encoded = urlencoding::encode(title.trim());
        let url = format!("{}/api/v1/search/title?keyword={encoded}", self.base_url);
        self.get_json(url).await
    }

    pub async fn create_subscription(
        &mut self,
        payload: &MoviePilotSubscriptionPayload,
    ) -> anyhow::Result<MoviePilotResponse> {
        self.post_json(
            format!("{}/api/v1/subscribe", self.base_url),
            serde_json::to_value(payload).context("failed to serialize subscription payload")?,
        )
        .await
    }

    pub async fn submit_download(
        &mut self,
        payload: &MoviePilotDownloadPayload,
    ) -> anyhow::Result<MoviePilotResponse> {
        let payload_value =
            serde_json::to_value(payload).context("failed to serialize download payload")?;

        if payload.media_in.is_some() {
            match self
                .post_json(
                    format!("{}/api/v1/download", self.base_url),
                    payload_value.clone(),
                )
                .await
            {
                Ok(response) => return Ok(response),
                Err(err) => {
                    warn!(
                        error = %err,
                        "moviepilot contextual download failed, falling back to torrent-only endpoint"
                    );
                }
            }
        }

        self.post_json(
            format!("{}/api/v1/download/add", self.base_url),
            json!({ "torrent_in": payload.torrent_in }),
        )
        .await
    }

    async fn get_json(&mut self, url: String) -> anyhow::Result<MoviePilotResponse> {
        let auth = self.ensure_auth().await?;
        let response = self
            .client
            .get(url)
            .header("Authorization", auth)
            .header("Accept", "application/json")
            .send()
            .await
            .context("moviepilot get request failed")?;
        parse_response(response).await
    }

    async fn post_json(&mut self, url: String, body: Value) -> anyhow::Result<MoviePilotResponse> {
        let auth = self.ensure_auth().await?;
        let response = self
            .client
            .post(url)
            .header("Authorization", auth)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&body)
            .send()
            .await
            .context("moviepilot post request failed")?;
        parse_response(response).await
    }

    pub fn timeout_seconds(&self) -> i64 {
        self.timeout_seconds
    }
}

impl MoviePilotProvider {
    pub fn from_config(config: &AgentMoviePilotConfig) -> anyhow::Result<Self> {
        Ok(Self {
            config: config.clone(),
            client: MoviePilotClient::new(
                &config.base_url,
                &config.username,
                &config.password,
                config.timeout_seconds,
            )?,
        })
    }

    pub fn configured(&self) -> bool {
        self.client.is_configured()
    }

    pub async fn check_health(&mut self) -> AgentProviderHealthReport {
        if !self.config.enabled {
            return AgentProviderHealthReport {
                healthy: false,
                message: "provider disabled".to_string(),
            };
        }
        if !self.configured() {
            return AgentProviderHealthReport {
                healthy: false,
                message: "provider not configured".to_string(),
            };
        }
        match self.client.authenticate().await {
            Ok(()) => AgentProviderHealthReport {
                healthy: true,
                message: "authentication succeeded".to_string(),
            },
            Err(err) => AgentProviderHealthReport {
                healthy: false,
                message: err.to_string(),
            },
        }
    }

    pub async fn status(&mut self) -> AgentProviderStatus {
        let report = self.check_health().await;
        AgentProviderStatus {
            provider_id: self.provider_id().to_string(),
            display_name: self.display_name().to_string(),
            provider_kind: self.provider_kind().to_string(),
            enabled: self.config.enabled,
            configured: self.configured(),
            healthy: report.healthy,
            capabilities: self
                .capabilities()
                .into_iter()
                .map(|cap| cap.as_str().to_string())
                .collect(),
            message: report.message,
            checked_at: Some(Utc::now()),
        }
    }

    pub fn into_client(self) -> MoviePilotClient {
        self.client
    }
}

impl AgentProviderDescriptor for MoviePilotProvider {
    fn provider_id(&self) -> &'static str {
        "moviepilot"
    }

    fn display_name(&self) -> &'static str {
        "MoviePilot"
    }

    fn provider_kind(&self) -> &'static str {
        "subscription_download"
    }

    fn capabilities(&self) -> Vec<AgentProviderCapability> {
        vec![
            AgentProviderCapability::Search,
            AgentProviderCapability::Subscribe,
            AgentProviderCapability::Download,
        ]
    }
}

async fn ensure_success(response: Response) -> anyhow::Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    anyhow::bail!("moviepilot request failed with status {}: {}", status, body);
}

async fn parse_response(response: Response) -> anyhow::Result<MoviePilotResponse> {
    let response = ensure_success(response).await?;
    let body = response
        .json::<MoviePilotResponse>()
        .await
        .context("failed to decode moviepilot response")?;
    Ok(body)
}

pub fn filter_search_results(
    contexts: &[MoviePilotContext],
    media_type: &str,
    requested_season: Option<i32>,
    requested_year: Option<&str>,
    filter: &AgentMoviePilotFilterConfig,
) -> MoviePilotFilterDecision {
    let mut filtered = Vec::new();
    let mut raw_only = false;

    for ctx in contexts {
        let title_upper = ctx.torrent_info.title.to_ascii_uppercase();
        if filter.excluded_keywords.iter().any(|keyword| {
            !keyword.trim().is_empty() && title_upper.contains(&keyword.to_ascii_uppercase())
        }) {
            debug!(title = %ctx.torrent_info.title, "filtered moviepilot result by excluded keyword");
            continue;
        }
        if ctx.torrent_info.seeders < filter.min_seeders.max(1) {
            continue;
        }
        let meta = ctx.meta_info.as_ref().or(ctx.media_info.as_ref());
        if let Some(year) = requested_year.filter(|value| !value.trim().is_empty()) {
            if let Some(meta) = meta {
                if !meta.year.is_empty() && meta.year != year {
                    continue;
                }
            }
        }
        if media_type.eq_ignore_ascii_case("series")
            && let Some(season) = requested_season.filter(|value| *value > 0)
            && let Some(meta) = meta
        {
            if meta.season > 0 && meta.season != season {
                continue;
            }
            if !meta.season_episode.is_empty()
                && !meta
                    .season_episode
                    .to_ascii_uppercase()
                    .contains(&format!("S{season:02}"))
            {
                continue;
            }
        }

        let size_gb = ctx.torrent_info.size / 1024.0 / 1024.0 / 1024.0;
        if media_type.eq_ignore_ascii_case("movie") && size_gb > filter.max_movie_size_gb {
            continue;
        }
        if media_type.eq_ignore_ascii_case("series") {
            let episodes = meta
                .map(|info| {
                    if info.total_episode > 0 {
                        info.total_episode
                    } else if info.begin_episode > 0 && info.end_episode >= info.begin_episode {
                        info.end_episode - info.begin_episode + 1
                    } else {
                        1
                    }
                })
                .unwrap_or(1)
                .max(1) as f64;
            if size_gb / episodes > filter.max_episode_size_gb {
                continue;
            }
        }

        filtered.push(ctx.clone());
    }

    if filtered.is_empty() && !contexts.is_empty() {
        raw_only = true;
    }

    filtered.sort_by(|left, right| score_result(right, filter).cmp(&score_result(left, filter)));
    MoviePilotFilterDecision { filtered, raw_only }
}

pub fn choose_best_result(
    contexts: &[MoviePilotContext],
    media_type: &str,
    requested_season: Option<i32>,
    requested_year: Option<&str>,
    filter: &AgentMoviePilotFilterConfig,
) -> Option<MoviePilotContext> {
    filter_search_results(
        contexts,
        media_type,
        requested_season,
        requested_year,
        filter,
    )
    .filtered
    .into_iter()
    .next()
}

fn score_result(context: &MoviePilotContext, filter: &AgentMoviePilotFilterConfig) -> i32 {
    let mut score = context.torrent_info.seeders.max(0) * 4;
    let meta = context.meta_info.as_ref().or(context.media_info.as_ref());
    if let Some(meta) = meta {
        score += preference_score(&meta.resource_pix, &filter.preferred_resource_pix, 40);
        score += preference_score(&meta.video_encode, &filter.preferred_video_encode, 25);
        score += preference_score(&meta.resource_type, &filter.preferred_resource_type, 15);
    }
    for label in &context.torrent_info.labels {
        score += preference_score(label, &filter.preferred_labels, 12);
    }
    if context.torrent_info.downloadvolumefactor > 0.0
        && context.torrent_info.downloadvolumefactor < 1.0
    {
        score += 10;
    }
    if !context.torrent_info.freedate.is_empty() || !context.torrent_info.freedate_diff.is_empty() {
        score += 8;
    }
    score
}

fn preference_score(candidate: &str, preferences: &[String], weight: i32) -> i32 {
    if candidate.trim().is_empty() {
        return 0;
    }
    let candidate_upper = candidate.to_ascii_uppercase();
    preferences
        .iter()
        .position(|item| {
            !item.trim().is_empty() && candidate_upper.contains(&item.to_ascii_uppercase())
        })
        .map(|idx| weight.saturating_sub(idx as i32 * 3).max(1))
        .unwrap_or(0)
}

pub fn decode_search_contexts(payload: &Value) -> Vec<MoviePilotContext> {
    if let Some(items) = payload.as_array() {
        return items
            .iter()
            .filter_map(|item| serde_json::from_value::<MoviePilotContext>(item.clone()).ok())
            .collect();
    }
    if let Some(items) = payload.get("list").and_then(Value::as_array) {
        return items
            .iter()
            .filter_map(|item| serde_json::from_value::<MoviePilotContext>(item.clone()).ok())
            .collect();
    }
    warn!("moviepilot search response does not contain array data");
    Vec::new()
}

pub fn build_subscription_payload(
    title: &str,
    media_type: &str,
    tmdb_id: Option<i64>,
    season: Option<i32>,
    description: &str,
    best_result: Option<&MoviePilotContext>,
) -> MoviePilotSubscriptionPayload {
    let mut payload = MoviePilotSubscriptionPayload {
        name: title.to_string(),
        r#type: if media_type.eq_ignore_ascii_case("movie") {
            "电影".to_string()
        } else {
            "电视剧".to_string()
        },
        tmdbid: tmdb_id.unwrap_or_default(),
        season: season.unwrap_or_default(),
        description: description.to_string(),
        ..Default::default()
    };
    if let Some(result) = best_result {
        if let Some(meta) = result.meta_info.as_ref().or(result.media_info.as_ref()) {
            if payload.year.is_empty() {
                payload.year = meta.year.clone();
            }
            if payload.total_episode == 0 {
                payload.total_episode = meta.total_episode.max(0);
            }
        }
    }
    payload
}

pub fn build_download_payload(result: &MoviePilotContext) -> MoviePilotDownloadPayload {
    build_download_payload_with_context(result, None)
}

pub fn build_download_payload_with_context(
    result: &MoviePilotContext,
    media_info: Option<MoviePilotMediaInfo>,
) -> MoviePilotDownloadPayload {
    MoviePilotDownloadPayload {
        media_in: media_info,
        torrent_in: result.torrent_info.clone(),
    }
}

pub fn summarize_moviepilot_result(result: &MoviePilotContext) -> Value {
    json!({
        "title": result.torrent_info.title,
        "seeders": result.torrent_info.seeders,
        "site_name": result.torrent_info.site_name,
        "size": result.torrent_info.size,
        "labels": result.torrent_info.labels,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        MoviePilotContext, MoviePilotMediaInfo, MoviePilotResponse, MoviePilotTorrentInfo,
        build_download_payload_with_context, choose_best_result, filter_search_results,
    };
    use ls_config::AgentMoviePilotFilterConfig;
    use serde_json::json;

    fn sample_context(title: &str, seeders: i32, season: i32, size_gb: f64) -> MoviePilotContext {
        MoviePilotContext {
            meta_info: Some(MoviePilotMediaInfo {
                title: title.to_string(),
                year: "2025".to_string(),
                season,
                resource_pix: "2160P".to_string(),
                resource_type: "WEB-DL".to_string(),
                video_encode: "X265".to_string(),
                total_episode: 10,
                ..Default::default()
            }),
            media_info: None,
            torrent_info: MoviePilotTorrentInfo {
                title: title.to_string(),
                seeders,
                size: size_gb * 1024.0 * 1024.0 * 1024.0,
                ..Default::default()
            },
        }
    }

    #[test]
    fn filter_search_results_respects_season_and_seeders() {
        let filter = AgentMoviePilotFilterConfig::default();
        let result = filter_search_results(
            &[
                sample_context("bad", 1, 2, 20.0),
                sample_context("ok", 20, 1, 20.0),
            ],
            "series",
            Some(1),
            Some("2025"),
            &filter,
        );
        assert_eq!(result.filtered.len(), 1);
        assert_eq!(result.filtered[0].torrent_info.title, "ok");
    }

    #[test]
    fn choose_best_result_prefers_higher_seeders() {
        let filter = AgentMoviePilotFilterConfig::default();
        let best = choose_best_result(
            &[
                sample_context("a", 5, 1, 10.0),
                sample_context("b", 9, 1, 10.0),
            ],
            "series",
            Some(1),
            Some("2025"),
            &filter,
        )
        .expect("best result");
        assert_eq!(best.torrent_info.title, "b");
    }

    #[test]
    fn moviepilot_response_accepts_null_message() {
        let payload = json!({
            "success": true,
            "message": null,
            "data": []
        });

        let parsed: MoviePilotResponse =
            serde_json::from_value(payload).expect("response should deserialize");
        assert_eq!(parsed.message, None);
    }

    #[test]
    fn contextual_download_payload_includes_media_info() {
        let result = sample_context("show", 10, 1, 12.0);
        let payload = build_download_payload_with_context(
            &result,
            Some(MoviePilotMediaInfo {
                title: "Show".to_string(),
                r#type: "电视剧".to_string(),
                tmdb_id: 42,
                season: 1,
                ..Default::default()
            }),
        );

        assert_eq!(
            payload.media_in.as_ref().map(|value| value.tmdb_id),
            Some(42)
        );
        assert_eq!(payload.torrent_in.title, "show");
    }
}
