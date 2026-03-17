use anyhow::Context;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    ImageAssetPatch, PersonPatch, ScrapeExternalIds, ScrapePatch, ScrapeResult, ScraperCapability,
    ScraperProviderDescriptor, ScraperScenario,
};

pub const TVDB_DEFAULT_BASE_URL: &str = "https://api4.thetvdb.com/v4";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvdbConfig {
    pub enabled: bool,
    pub base_url: String,
    pub api_key: String,
    pub pin: String,
    pub timeout_seconds: u64,
}

impl Default for TvdbConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: TVDB_DEFAULT_BASE_URL.to_string(),
            api_key: String::new(),
            pin: String::new(),
            timeout_seconds: 15,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TvdbScrapeResult {
    pub provider_id: String,
    pub item_id: String,
    pub title: Option<String>,
    pub original_title: Option<String>,
    pub overview: Option<String>,
    pub premiere_date: Option<String>,
    pub production_year: Option<i32>,
    pub genres: Vec<String>,
    pub tags: Vec<String>,
    pub studios: Vec<String>,
    pub official_rating: Option<String>,
    pub community_rating: Option<f64>,
    pub external_ids: ScrapeExternalIds,
    pub people: Vec<PersonPatch>,
    pub images: Vec<ImageAssetPatch>,
    pub raw: Value,
}

impl TvdbScrapeResult {
    pub fn into_scrape_result(self, scenario: ScraperScenario) -> ScrapeResult {
        let provider_ids = build_provider_ids(&self.external_ids);
        let series_name = matches!(
            scenario,
            ScraperScenario::SeriesMetadata
                | ScraperScenario::SeasonMetadata
                | ScraperScenario::EpisodeMetadata
        )
        .then(|| self.title.clone())
        .flatten();
        let patch = ScrapePatch {
            metadata: json!({
                "overview": self.overview,
                "premiere_date": self.premiere_date,
                "production_year": self.production_year,
                "genres": self.genres,
                "tags": self.tags,
                "studios": self.studios,
                "official_rating": self.official_rating,
                "community_rating": self.community_rating,
                "sort_name": self.title,
                "title": self.title,
                "original_title": self.original_title,
                "series_name": series_name,
                "people": self.people,
                "scraper_raw": { "tvdb": self.raw.clone() },
            }),
            provider_ids,
            images: self.images,
            people: Vec::new(),
            tags: Vec::new(),
        };
        ScrapeResult {
            provider_id: self.provider_id,
            scenario: scenario.as_str().to_string(),
            patch,
            raw: self.raw,
            warnings: Vec::new(),
            complete: true,
        }
    }
}

pub struct TvdbClient<'a> {
    http_client: &'a Client,
    config: &'a TvdbConfig,
}

impl<'a> TvdbClient<'a> {
    pub fn new(http_client: &'a Client, config: &'a TvdbConfig) -> Self {
        Self {
            http_client,
            config,
        }
    }

    pub async fn health_check(&self) -> anyhow::Result<()> {
        let _ = self.login_token().await?;
        Ok(())
    }

    pub async fn scrape_series_by_title(
        &self,
        query: &str,
        year: Option<i32>,
    ) -> anyhow::Result<Option<TvdbScrapeResult>> {
        let token = self.login_token().await?;
        let endpoint = format!(
            "{}/search?query={}&type=series",
            self.base_url(),
            urlencoding::encode(query),
        );
        let payload = self.get_json(&endpoint, &token).await?;
        let candidate = payload
            .get("data")
            .and_then(Value::as_array)
            .and_then(|items| select_tvdb_candidate(items, year));
        let Some(candidate) = candidate else {
            return Ok(None);
        };
        let item_id = candidate
            .get("tvdb_id")
            .or_else(|| candidate.get("id"))
            .and_then(Value::as_i64)
            .context("tvdb series candidate missing id")?;
        self.fetch_series(item_id, &token).await
    }

    pub async fn scrape_movie_by_title(
        &self,
        query: &str,
        year: Option<i32>,
    ) -> anyhow::Result<Option<TvdbScrapeResult>> {
        let token = self.login_token().await?;
        let endpoint = format!(
            "{}/search?query={}&type=movie",
            self.base_url(),
            urlencoding::encode(query),
        );
        let payload = self.get_json(&endpoint, &token).await?;
        let candidate = payload
            .get("data")
            .and_then(Value::as_array)
            .and_then(|items| select_tvdb_candidate(items, year));
        let Some(candidate) = candidate else {
            return Ok(None);
        };
        let item_id = candidate
            .get("tvdb_id")
            .or_else(|| candidate.get("id"))
            .and_then(Value::as_i64)
            .context("tvdb movie candidate missing id")?;
        self.fetch_movie(item_id, &token).await
    }

    pub async fn scrape_episode_by_title(
        &self,
        series_query: &str,
        season_number: Option<i32>,
        episode_number: Option<i32>,
        year: Option<i32>,
    ) -> anyhow::Result<Option<TvdbScrapeResult>> {
        let Some(series) = self.scrape_series_by_title(series_query, year).await? else {
            return Ok(None);
        };
        let token = self.login_token().await?;
        let episode_payload = find_tvdb_episode(&series.raw, season_number, episode_number)
            .or_else(|| {
                series
                    .raw
                    .get("seasons")
                    .and_then(Value::as_array)
                    .and_then(|_| None)
            });

        if let Some(episode) = episode_payload
            && let Some(episode_id) = episode.get("id").and_then(Value::as_i64)
        {
            return self.fetch_episode(episode_id, &token).await;
        }

        Ok(Some(series))
    }

    pub async fn fetch_series(
        &self,
        series_id: i64,
        token: &str,
    ) -> anyhow::Result<Option<TvdbScrapeResult>> {
        let endpoint = format!(
            "{}/series/{series_id}/extended?meta=translations",
            self.base_url()
        );
        let payload = self.get_json(&endpoint, token).await?;
        Ok(payload
            .get("data")
            .cloned()
            .map(|data| map_tvdb_item("tvdb", data, ScraperScenario::SeriesMetadata)))
    }

    pub async fn fetch_movie(
        &self,
        movie_id: i64,
        token: &str,
    ) -> anyhow::Result<Option<TvdbScrapeResult>> {
        let endpoint = format!(
            "{}/movies/{movie_id}/extended?meta=translations",
            self.base_url()
        );
        let payload = self.get_json(&endpoint, token).await?;
        Ok(payload
            .get("data")
            .cloned()
            .map(|data| map_tvdb_item("tvdb", data, ScraperScenario::MovieMetadata)))
    }

    pub async fn fetch_episode(
        &self,
        episode_id: i64,
        token: &str,
    ) -> anyhow::Result<Option<TvdbScrapeResult>> {
        let endpoint = format!(
            "{}/episodes/{episode_id}/extended?meta=translations",
            self.base_url()
        );
        let payload = self.get_json(&endpoint, token).await?;
        Ok(payload
            .get("data")
            .cloned()
            .map(|data| map_tvdb_item("tvdb", data, ScraperScenario::EpisodeMetadata)))
    }

    async fn login_token(&self) -> anyhow::Result<String> {
        let endpoint = format!("{}/login", self.base_url());
        let payload = self
            .http_client
            .post(&endpoint)
            .json(&json!({
                "apikey": self.config.api_key,
                "pin": self.config.pin,
            }))
            .timeout(std::time::Duration::from_secs(
                self.config.timeout_seconds.max(1),
            ))
            .send()
            .await
            .with_context(|| format!("tvdb login request failed: {endpoint}"))?
            .error_for_status()
            .with_context(|| format!("tvdb login returned non-success: {endpoint}"))?
            .json::<Value>()
            .await
            .context("failed to decode tvdb login response")?;
        payload
            .get("data")
            .and_then(|data| data.get("token"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .context("tvdb login response missing token")
    }

    async fn get_json(&self, endpoint: &str, token: &str) -> anyhow::Result<Value> {
        self.http_client
            .get(endpoint)
            .bearer_auth(token)
            .timeout(std::time::Duration::from_secs(
                self.config.timeout_seconds.max(1),
            ))
            .send()
            .await
            .with_context(|| format!("tvdb request failed: {endpoint}"))?
            .error_for_status()
            .with_context(|| format!("tvdb returned non-success: {endpoint}"))?
            .json::<Value>()
            .await
            .with_context(|| format!("failed to decode tvdb response: {endpoint}"))
    }

    fn base_url(&self) -> String {
        let trimmed = self.config.base_url.trim().trim_end_matches('/');
        if trimmed.is_empty() {
            TVDB_DEFAULT_BASE_URL.to_string()
        } else {
            trimmed.to_string()
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TvdbScraperProvider;

impl ScraperProviderDescriptor for TvdbScraperProvider {
    fn provider_id(&self) -> &'static str {
        "tvdb"
    }

    fn display_name(&self) -> &'static str {
        "TVDB"
    }

    fn provider_kind(&self) -> &'static str {
        "metadata"
    }

    fn capabilities(&self) -> Vec<ScraperCapability> {
        vec![
            ScraperCapability::Search,
            ScraperCapability::Details,
            ScraperCapability::Images,
            ScraperCapability::People,
            ScraperCapability::ExternalIds,
        ]
    }

    fn scenarios(&self) -> Vec<ScraperScenario> {
        vec![
            ScraperScenario::MovieMetadata,
            ScraperScenario::SeriesMetadata,
            ScraperScenario::SeasonMetadata,
            ScraperScenario::EpisodeMetadata,
            ScraperScenario::PersonMetadata,
            ScraperScenario::ImageFetch,
            ScraperScenario::SearchByTitle,
            ScraperScenario::SearchByExternalId,
        ]
    }
}

fn select_tvdb_candidate<'a>(items: &'a [Value], year: Option<i32>) -> Option<&'a Value> {
    items.iter().max_by_key(|item| {
        let item_year = item
            .get("year")
            .and_then(Value::as_i64)
            .and_then(|value| i32::try_from(value).ok());
        let year_score = match (year, item_year) {
            (Some(expected), Some(actual)) if expected == actual => 20,
            (Some(expected), Some(actual)) if (expected - actual).abs() <= 1 => 10,
            (Some(_), Some(_)) => 0,
            _ => 5,
        };
        let score = item
            .get("score")
            .and_then(Value::as_i64)
            .unwrap_or_default() as i32;
        year_score + score
    })
}

fn find_tvdb_episode(
    series_payload: &Value,
    season_number: Option<i32>,
    episode_number: Option<i32>,
) -> Option<Value> {
    let seasons = series_payload.get("seasons")?.as_array()?;
    for season in seasons {
        if season_number.is_some()
            && season
                .get("number")
                .and_then(Value::as_i64)
                .and_then(|value| i32::try_from(value).ok())
                != season_number
        {
            continue;
        }
        if let Some(episodes) = season.get("episodes").and_then(Value::as_array) {
            for episode in episodes {
                let number = episode
                    .get("number")
                    .and_then(Value::as_i64)
                    .and_then(|value| i32::try_from(value).ok());
                if episode_number.is_none() || number == episode_number {
                    return Some(episode.clone());
                }
            }
        }
    }
    None
}

fn map_tvdb_item(provider_id: &str, data: Value, scenario: ScraperScenario) -> TvdbScrapeResult {
    let item_id = data
        .get("id")
        .and_then(Value::as_i64)
        .map(|value| value.to_string())
        .unwrap_or_default();
    let title = data
        .get("name")
        .or_else(|| data.get("seriesName"))
        .or_else(|| data.get("movieName"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let original_title = data
        .get("originalName")
        .or_else(|| data.get("nameTranslations"))
        .and_then(Value::as_array)
        .and_then(|translations| translations.first())
        .and_then(Value::as_str)
        .map(str::to_string);
    let premiere_date = data
        .get("firstAired")
        .or_else(|| data.get("aired"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let production_year = premiere_date
        .as_deref()
        .and_then(parse_year_prefix)
        .or_else(|| {
            data.get("year")
                .and_then(Value::as_i64)
                .and_then(|value| i32::try_from(value).ok())
        });
    let genres = data
        .get("genres")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    item.get("name")
                        .or_else(|| item.get("genre"))
                        .and_then(Value::as_str)
                        .map(str::to_string)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let tags = data
        .get("tags")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("name").and_then(Value::as_str).map(str::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let studios = data
        .get("companies")
        .or_else(|| data.get("studios"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("name").and_then(Value::as_str).map(str::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let people = data
        .get("characters")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .take(20)
                .filter_map(|item| {
                    let name = item.get("name").and_then(Value::as_str)?.trim().to_string();
                    if name.is_empty() {
                        return None;
                    }
                    let role = item
                        .get("peopleType")
                        .or_else(|| item.get("type"))
                        .and_then(Value::as_str)
                        .unwrap_or("Actor");
                    Some(PersonPatch {
                        provider_person_id: item
                            .get("personId")
                            .or_else(|| item.get("id"))
                            .and_then(Value::as_i64)
                            .map(|value| value.to_string()),
                        name,
                        person_type: normalize_person_type(role).to_string(),
                        role: item
                            .get("personName")
                            .or_else(|| item.get("character"))
                            .and_then(Value::as_str)
                            .map(str::to_string),
                        image_path: item
                            .get("image")
                            .or_else(|| item.get("imageUrl"))
                            .and_then(Value::as_str)
                            .map(str::to_string),
                        metadata: item.clone(),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let images = collect_tvdb_images(&data, &scenario);
    let external_ids = extract_tvdb_external_ids(&data);

    TvdbScrapeResult {
        provider_id: provider_id.to_string(),
        item_id,
        title,
        original_title,
        overview: data
            .get("overview")
            .and_then(Value::as_str)
            .map(str::to_string),
        premiere_date,
        production_year,
        genres,
        tags,
        studios,
        official_rating: data
            .get("contentRatings")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .and_then(|item| item.get("name").or_else(|| item.get("rating")))
            .and_then(Value::as_str)
            .map(str::to_string),
        community_rating: data
            .get("score")
            .or_else(|| data.get("averageRating"))
            .and_then(Value::as_f64),
        external_ids,
        people,
        images,
        raw: data,
    }
}

fn collect_tvdb_images(data: &Value, scenario: &ScraperScenario) -> Vec<ImageAssetPatch> {
    let mut images = Vec::new();
    if let Some(image) = data
        .get("image")
        .or_else(|| data.get("image_url"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        images.push(ImageAssetPatch {
            image_type: match scenario {
                ScraperScenario::EpisodeMetadata => "thumb".to_string(),
                _ => "primary".to_string(),
            },
            provider_id: "tvdb".to_string(),
            remote_path: Some(image.to_string()),
            local_path: None,
            language: None,
            tag: None,
        });
    }
    if let Some(artworks) = data.get("artworks").and_then(Value::as_array) {
        for artwork in artworks {
            let Some(url) = artwork
                .get("image")
                .or_else(|| artwork.get("thumbnail"))
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
            else {
                continue;
            };
            let image_type = artwork
                .get("type")
                .and_then(Value::as_i64)
                .map(|kind| match kind {
                    3 | 7 => "backdrop",
                    6 => "logo",
                    _ => "primary",
                })
                .unwrap_or("primary");
            images.push(ImageAssetPatch {
                image_type: image_type.to_string(),
                provider_id: "tvdb".to_string(),
                remote_path: Some(url.to_string()),
                local_path: None,
                language: artwork
                    .get("language")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                tag: None,
            });
        }
    }
    images
}

fn extract_tvdb_external_ids(data: &Value) -> ScrapeExternalIds {
    let mut ids = ScrapeExternalIds {
        tvdb: data
            .get("id")
            .and_then(Value::as_i64)
            .map(|value| value.to_string()),
        ..ScrapeExternalIds::default()
    };
    if let Some(remote_ids) = data.get("remoteIds").and_then(Value::as_array) {
        for remote_id in remote_ids {
            let Some(source) = remote_id
                .get("sourceName")
                .or_else(|| remote_id.get("type"))
                .and_then(Value::as_str)
            else {
                continue;
            };
            let Some(value) = remote_id
                .get("id")
                .or_else(|| remote_id.get("value"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                continue;
            };
            if source.eq_ignore_ascii_case("imdb") {
                ids.imdb = Some(value.to_string());
            } else if source.eq_ignore_ascii_case("tmdb") {
                ids.tmdb = Some(value.to_string());
            } else if source.eq_ignore_ascii_case("tvdb") {
                ids.tvdb = Some(value.to_string());
            } else {
                ids.extra
                    .insert(source.to_ascii_lowercase(), value.to_string());
            }
        }
    }
    ids
}

fn build_provider_ids(
    external_ids: &ScrapeExternalIds,
) -> std::collections::BTreeMap<String, String> {
    let mut provider_ids = std::collections::BTreeMap::new();
    if let Some(value) = external_ids.tmdb.as_ref() {
        provider_ids.insert("Tmdb".to_string(), value.clone());
    }
    if let Some(value) = external_ids.imdb.as_ref() {
        provider_ids.insert("Imdb".to_string(), value.clone());
    }
    if let Some(value) = external_ids.tvdb.as_ref() {
        provider_ids.insert("Tvdb".to_string(), value.clone());
    }
    if let Some(value) = external_ids.bangumi.as_ref() {
        provider_ids.insert("Bangumi".to_string(), value.clone());
    }
    provider_ids
}

fn normalize_person_type(raw: &str) -> &'static str {
    if raw.eq_ignore_ascii_case("director") {
        "Director"
    } else if raw.eq_ignore_ascii_case("writer") || raw.eq_ignore_ascii_case("creator") {
        "Writer"
    } else {
        "Actor"
    }
}

fn parse_year_prefix(raw: &str) -> Option<i32> {
    raw.get(0..4)?.parse::<i32>().ok()
}
