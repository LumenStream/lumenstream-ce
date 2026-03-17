use anyhow::Context;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    ImageAssetPatch, PersonPatch, ScrapeExternalIds, ScrapePatch, ScrapeResult, ScraperCapability,
    ScraperProviderDescriptor, ScraperScenario,
};

pub const BANGUMI_DEFAULT_BASE_URL: &str = "https://api.bgm.tv";
pub const BANGUMI_DEFAULT_USER_AGENT: &str = "lumenstream/0.1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiConfig {
    pub enabled: bool,
    pub base_url: String,
    pub access_token: String,
    pub timeout_seconds: u64,
    pub user_agent: String,
}

impl Default for BangumiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: BANGUMI_DEFAULT_BASE_URL.to_string(),
            access_token: String::new(),
            timeout_seconds: 15,
            user_agent: BANGUMI_DEFAULT_USER_AGENT.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BangumiScrapeResult {
    pub provider_id: String,
    pub subject_id: String,
    pub title: Option<String>,
    pub original_title: Option<String>,
    pub overview: Option<String>,
    pub premiere_date: Option<String>,
    pub production_year: Option<i32>,
    pub genres: Vec<String>,
    pub tags: Vec<String>,
    pub external_ids: ScrapeExternalIds,
    pub people: Vec<PersonPatch>,
    pub images: Vec<ImageAssetPatch>,
    pub raw: Value,
}

impl BangumiScrapeResult {
    pub fn into_scrape_result(self, scenario: ScraperScenario) -> ScrapeResult {
        let mut provider_ids = std::collections::BTreeMap::new();
        if let Some(value) = self.external_ids.tmdb.as_ref() {
            provider_ids.insert("Tmdb".to_string(), value.clone());
        }
        if let Some(value) = self.external_ids.imdb.as_ref() {
            provider_ids.insert("Imdb".to_string(), value.clone());
        }
        if let Some(value) = self.external_ids.tvdb.as_ref() {
            provider_ids.insert("Tvdb".to_string(), value.clone());
        }
        if let Some(value) = self.external_ids.bangumi.as_ref() {
            provider_ids.insert("Bangumi".to_string(), value.clone());
        }

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
                "sort_name": self.title,
                "title": self.title,
                "original_title": self.original_title,
                "series_name": series_name,
                "people": self.people,
                "scraper_raw": { "bangumi": self.raw.clone() },
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

pub struct BangumiClient<'a> {
    http_client: &'a Client,
    config: &'a BangumiConfig,
}

impl<'a> BangumiClient<'a> {
    pub fn new(http_client: &'a Client, config: &'a BangumiConfig) -> Self {
        Self {
            http_client,
            config,
        }
    }

    pub async fn health_check(&self) -> anyhow::Result<()> {
        let endpoint = format!("{}/v0/me", self.base_url());
        let _ = self
            .request(reqwest::Method::GET, &endpoint)
            .await?
            .send()
            .await
            .with_context(|| format!("bangumi health check request failed: {endpoint}"))?
            .error_for_status()
            .with_context(|| format!("bangumi health check failed: {endpoint}"))?;
        Ok(())
    }

    pub async fn scrape_series_by_title(
        &self,
        query: &str,
        year: Option<i32>,
    ) -> anyhow::Result<Option<BangumiScrapeResult>> {
        let endpoint = format!("{}/v0/search/subjects", self.base_url());
        let payload = self
            .request(reqwest::Method::POST, &endpoint)
            .await?
            .json(&json!({
                "keyword": query,
                "sort": "match",
                "filter": { "type": [2] }
            }))
            .send()
            .await
            .with_context(|| format!("bangumi search request failed: {endpoint}"))?
            .error_for_status()
            .with_context(|| format!("bangumi search returned non-success: {endpoint}"))?
            .json::<Value>()
            .await
            .context("failed to decode bangumi search response")?;

        let candidate = payload
            .get("data")
            .and_then(Value::as_array)
            .and_then(|items| select_bangumi_candidate(items, year));
        let Some(candidate) = candidate else {
            return Ok(None);
        };
        let subject_id = candidate
            .get("id")
            .and_then(Value::as_i64)
            .context("bangumi subject missing id")?;
        self.fetch_subject(subject_id).await
    }

    pub async fn scrape_episode_by_title(
        &self,
        query: &str,
        season_number: Option<i32>,
        episode_number: Option<i32>,
        year: Option<i32>,
    ) -> anyhow::Result<Option<BangumiScrapeResult>> {
        let Some(series) = self.scrape_series_by_title(query, year).await? else {
            return Ok(None);
        };
        let episodes_endpoint = format!(
            "{}/v0/episodes?subject_id={}&type=0",
            self.base_url(),
            series.subject_id
        );
        let payload = self
            .request(reqwest::Method::GET, &episodes_endpoint)
            .await?
            .send()
            .await
            .with_context(|| format!("bangumi episodes request failed: {episodes_endpoint}"))?
            .error_for_status()
            .with_context(|| format!("bangumi episodes returned non-success: {episodes_endpoint}"))?
            .json::<Value>()
            .await
            .context("failed to decode bangumi episodes response")?;
        let candidate = payload
            .get("data")
            .and_then(Value::as_array)
            .and_then(|items| {
                items.iter().find(|item| {
                    let season_matches = season_number.is_none_or(|expected| {
                        item.get("season")
                            .and_then(Value::as_i64)
                            .and_then(|value| i32::try_from(value).ok())
                            == Some(expected)
                    });
                    let episode_matches = episode_number.is_none_or(|expected| {
                        item.get("sort")
                            .and_then(Value::as_i64)
                            .and_then(|value| i32::try_from(value).ok())
                            == Some(expected)
                    });
                    season_matches && episode_matches
                })
            });

        let Some(candidate) = candidate.cloned() else {
            return Ok(Some(series));
        };

        let title = candidate
            .get("name_cn")
            .or_else(|| candidate.get("name"))
            .and_then(Value::as_str)
            .map(str::to_string);
        let premiere_date = candidate
            .get("airdate")
            .and_then(Value::as_str)
            .map(str::to_string);
        let production_year = premiere_date.as_deref().and_then(parse_year_prefix);

        Ok(Some(BangumiScrapeResult {
            provider_id: "bangumi".to_string(),
            subject_id: candidate
                .get("id")
                .and_then(Value::as_i64)
                .map(|value| value.to_string())
                .unwrap_or_else(|| series.subject_id.clone()),
            title,
            original_title: candidate
                .get("name")
                .and_then(Value::as_str)
                .map(str::to_string),
            overview: candidate
                .get("desc")
                .and_then(Value::as_str)
                .map(str::to_string),
            premiere_date,
            production_year,
            genres: series.genres.clone(),
            tags: series.tags.clone(),
            external_ids: series.external_ids.clone(),
            people: Vec::new(),
            images: series.images.clone(),
            raw: json!({
                "subject": series.raw,
                "episode": candidate,
            }),
        }))
    }

    pub async fn fetch_subject(
        &self,
        subject_id: i64,
    ) -> anyhow::Result<Option<BangumiScrapeResult>> {
        let endpoint = format!("{}/v0/subjects/{subject_id}", self.base_url());
        let data = self
            .request(reqwest::Method::GET, &endpoint)
            .await?
            .send()
            .await
            .with_context(|| format!("bangumi subject request failed: {endpoint}"))?
            .error_for_status()
            .with_context(|| format!("bangumi subject returned non-success: {endpoint}"))?
            .json::<Value>()
            .await
            .context("failed to decode bangumi subject response")?;
        Ok(Some(map_bangumi_subject(data)))
    }

    async fn request(
        &self,
        method: reqwest::Method,
        endpoint: &str,
    ) -> anyhow::Result<reqwest::RequestBuilder> {
        let builder = self
            .http_client
            .request(method, endpoint)
            .timeout(std::time::Duration::from_secs(
                self.config.timeout_seconds.max(1),
            ))
            .header(
                reqwest::header::USER_AGENT,
                if self.config.user_agent.trim().is_empty() {
                    BANGUMI_DEFAULT_USER_AGENT
                } else {
                    self.config.user_agent.trim()
                },
            );
        Ok(if self.config.access_token.trim().is_empty() {
            builder
        } else {
            builder.bearer_auth(self.config.access_token.trim())
        })
    }

    fn base_url(&self) -> String {
        let trimmed = self.config.base_url.trim().trim_end_matches('/');
        if trimmed.is_empty() {
            BANGUMI_DEFAULT_BASE_URL.to_string()
        } else {
            trimmed.to_string()
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct BangumiScraperProvider;

impl ScraperProviderDescriptor for BangumiScraperProvider {
    fn provider_id(&self) -> &'static str {
        "bangumi"
    }

    fn display_name(&self) -> &'static str {
        "Bangumi"
    }

    fn provider_kind(&self) -> &'static str {
        "metadata"
    }

    fn capabilities(&self) -> Vec<ScraperCapability> {
        vec![
            ScraperCapability::Search,
            ScraperCapability::Details,
            ScraperCapability::Images,
            ScraperCapability::ExternalIds,
        ]
    }

    fn scenarios(&self) -> Vec<ScraperScenario> {
        vec![
            ScraperScenario::SeriesMetadata,
            ScraperScenario::SeasonMetadata,
            ScraperScenario::EpisodeMetadata,
            ScraperScenario::ImageFetch,
            ScraperScenario::SearchByTitle,
        ]
    }
}

fn select_bangumi_candidate<'a>(items: &'a [Value], year: Option<i32>) -> Option<&'a Value> {
    items.iter().max_by_key(|item| {
        let item_year = item
            .get("date")
            .or_else(|| item.get("air_date"))
            .and_then(Value::as_str)
            .and_then(parse_year_prefix);
        let year_score = match (year, item_year) {
            (Some(expected), Some(actual)) if expected == actual => 20,
            (Some(expected), Some(actual)) if (expected - actual).abs() <= 1 => 10,
            (Some(_), Some(_)) => 0,
            _ => 5,
        };
        let rank_score = item
            .get("score")
            .and_then(|score| score.get("rank"))
            .and_then(Value::as_i64)
            .map(|rank| (10000 - rank).max(0) as i32)
            .unwrap_or_default();
        year_score + rank_score
    })
}

fn map_bangumi_subject(data: Value) -> BangumiScrapeResult {
    let subject_id = data
        .get("id")
        .and_then(Value::as_i64)
        .map(|value| value.to_string())
        .unwrap_or_default();
    let title = data
        .get("name_cn")
        .or_else(|| data.get("name"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let original_title = data.get("name").and_then(Value::as_str).map(str::to_string);
    let premiere_date = data.get("date").and_then(Value::as_str).map(str::to_string);
    let production_year = premiere_date.as_deref().and_then(parse_year_prefix);
    let genres = data
        .get("tags")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("name").and_then(Value::as_str).map(str::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let tags = genres.clone();
    let people =
        data.get("infobox")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| {
                        let key = item.get("key").and_then(Value::as_str)?;
                        let person_type = if key.contains("导演") {
                            "Director"
                        } else if key.contains("脚本") || key.contains("原作") {
                            "Writer"
                        } else {
                            return None;
                        };
                        let value = item.get("value")?;
                        let name = value.as_str().map(str::to_string).or_else(|| {
                            value.get("v").and_then(Value::as_str).map(str::to_string)
                        })?;
                        Some(PersonPatch {
                            provider_person_id: None,
                            name,
                            person_type: person_type.to_string(),
                            role: None,
                            image_path: None,
                            metadata: item.clone(),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
    let images = data
        .get("images")
        .map(|images| {
            let mut out = Vec::new();
            if let Some(large) = images
                .get("large")
                .or_else(|| images.get("common"))
                .or_else(|| images.get("medium"))
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
            {
                out.push(ImageAssetPatch {
                    image_type: "primary".to_string(),
                    provider_id: "bangumi".to_string(),
                    remote_path: Some(large.to_string()),
                    local_path: None,
                    language: None,
                    tag: None,
                });
            }
            out
        })
        .unwrap_or_default();

    BangumiScrapeResult {
        provider_id: "bangumi".to_string(),
        subject_id: subject_id.clone(),
        title,
        original_title,
        overview: data
            .get("summary")
            .and_then(Value::as_str)
            .map(str::to_string),
        premiere_date,
        production_year,
        genres,
        tags,
        external_ids: ScrapeExternalIds {
            bangumi: Some(subject_id),
            ..ScrapeExternalIds::default()
        },
        people,
        images,
        raw: data,
    }
}

fn parse_year_prefix(raw: &str) -> Option<i32> {
    raw.get(0..4)?.parse::<i32>().ok()
}
