use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ScraperScenario {
    MovieMetadata,
    SeriesMetadata,
    SeasonMetadata,
    EpisodeMetadata,
    PersonMetadata,
    ImageFetch,
    SearchByTitle,
    SearchByExternalId,
}

impl ScraperScenario {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MovieMetadata => "movie_metadata",
            Self::SeriesMetadata => "series_metadata",
            Self::SeasonMetadata => "season_metadata",
            Self::EpisodeMetadata => "episode_metadata",
            Self::PersonMetadata => "person_metadata",
            Self::ImageFetch => "image_fetch",
            Self::SearchByTitle => "search_by_title",
            Self::SearchByExternalId => "search_by_external_id",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ScrapeExternalIds {
    #[serde(default)]
    pub tmdb: Option<String>,
    #[serde(default)]
    pub imdb: Option<String>,
    #[serde(default)]
    pub tvdb: Option<String>,
    #[serde(default)]
    pub bangumi: Option<String>,
    #[serde(default)]
    pub extra: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScrapeMatchHints {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub original_title: Option<String>,
    #[serde(default)]
    pub year: Option<i32>,
    #[serde(default)]
    pub season_number: Option<i32>,
    #[serde(default)]
    pub episode_number: Option<i32>,
    #[serde(default)]
    pub external_ids: ScrapeExternalIds,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScrapeContext {
    #[serde(default)]
    pub item_id: Option<Uuid>,
    #[serde(default)]
    pub library_id: Option<Uuid>,
    pub item_type: String,
    pub path: String,
    #[serde(default)]
    pub metadata: Value,
    #[serde(default)]
    pub hints: ScrapeMatchHints,
    #[serde(default)]
    pub force_image_refresh: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScrapeCandidate {
    pub provider_id: String,
    #[serde(default)]
    pub provider_item_id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub subtitle: String,
    #[serde(default)]
    pub score: i32,
    #[serde(default)]
    pub external_ids: ScrapeExternalIds,
    #[serde(default)]
    pub raw: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImageAssetPatch {
    pub image_type: String,
    pub provider_id: String,
    #[serde(default)]
    pub remote_path: Option<String>,
    #[serde(default)]
    pub local_path: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub tag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersonPatch {
    #[serde(default)]
    pub provider_person_id: Option<String>,
    pub name: String,
    pub person_type: String,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub image_path: Option<String>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScrapePatch {
    #[serde(default)]
    pub metadata: Value,
    #[serde(default)]
    pub provider_ids: BTreeMap<String, String>,
    #[serde(default)]
    pub images: Vec<ImageAssetPatch>,
    #[serde(default)]
    pub people: Vec<PersonPatch>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScrapeResult {
    pub provider_id: String,
    pub scenario: String,
    #[serde(default)]
    pub patch: ScrapePatch,
    #[serde(default)]
    pub raw: Value,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ScraperPolicySettings {
    #[serde(default = "default_strategy")]
    pub default_strategy: String,
    #[serde(default)]
    pub default_routes: ScraperDefaultRoutes,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ScraperDefaultRoutes {
    #[serde(default)]
    pub movie: Vec<String>,
    #[serde(default)]
    pub series: Vec<String>,
    #[serde(default)]
    pub image: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ScraperLibraryPolicy {
    #[serde(default)]
    pub movie: Vec<String>,
    #[serde(default)]
    pub series: Vec<String>,
    #[serde(default)]
    pub image: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScrapePlan {
    pub scenario: String,
    #[serde(default)]
    pub provider_chain: Vec<String>,
    pub strategy: String,
    #[serde(default)]
    pub source: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ScraperRoutePurpose {
    Metadata,
    Image,
}

pub fn infer_scenario_from_item_type(item_type: &str) -> ScraperScenario {
    if item_type.eq_ignore_ascii_case("movie") {
        ScraperScenario::MovieMetadata
    } else if item_type.eq_ignore_ascii_case("series") {
        ScraperScenario::SeriesMetadata
    } else if item_type.eq_ignore_ascii_case("season") {
        ScraperScenario::SeasonMetadata
    } else {
        ScraperScenario::EpisodeMetadata
    }
}

pub fn normalize_provider_chain(values: &[String]) -> Vec<String> {
    let mut dedup = Vec::<String>::new();
    for value in values {
        let normalized = value.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            continue;
        }
        if !dedup.iter().any(|existing| existing == &normalized) {
            dedup.push(normalized);
        }
    }
    dedup
}

pub fn resolve_provider_chain(
    settings: &ScraperPolicySettings,
    library_policy: Option<&ScraperLibraryPolicy>,
    scenario: ScraperScenario,
    purpose: ScraperRoutePurpose,
    available: &[String],
) -> ScrapePlan {
    let scenario_key = scenario.as_str().to_string();
    let library_chain = library_policy
        .map(|policy| match purpose {
            ScraperRoutePurpose::Metadata => match scenario {
                ScraperScenario::MovieMetadata => policy.movie.clone(),
                _ => policy.series.clone(),
            },
            ScraperRoutePurpose::Image => policy.image.clone(),
        })
        .unwrap_or_default();
    let default_chain = match purpose {
        ScraperRoutePurpose::Image => settings.default_routes.image.clone(),
        ScraperRoutePurpose::Metadata => match scenario {
            ScraperScenario::MovieMetadata => settings.default_routes.movie.clone(),
            _ => settings.default_routes.series.clone(),
        },
    };

    let available_chain = normalize_provider_chain(available);
    let requested = if library_chain.is_empty() {
        normalize_provider_chain(&default_chain)
    } else {
        normalize_provider_chain(&library_chain)
    };
    let provider_chain = requested
        .into_iter()
        .filter(|provider_id| available_chain.iter().any(|item| item == provider_id))
        .collect::<Vec<_>>();

    ScrapePlan {
        scenario: scenario_key,
        provider_chain,
        strategy: settings.default_strategy.trim().to_string(),
        source: if library_chain.is_empty() {
            "global_default".to_string()
        } else {
            "library_override".to_string()
        },
    }
}

fn default_strategy() -> String {
    "primary_with_fallback".to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        ScrapeExternalIds, ScraperDefaultRoutes, ScraperLibraryPolicy, ScraperPolicySettings,
        ScraperRoutePurpose, ScraperScenario, infer_scenario_from_item_type,
        normalize_provider_chain, resolve_provider_chain,
    };

    #[test]
    fn infer_scenario_matches_item_type() {
        assert_eq!(
            infer_scenario_from_item_type("Movie"),
            ScraperScenario::MovieMetadata
        );
        assert_eq!(
            infer_scenario_from_item_type("Series"),
            ScraperScenario::SeriesMetadata
        );
        assert_eq!(
            infer_scenario_from_item_type("Episode"),
            ScraperScenario::EpisodeMetadata
        );
    }

    #[test]
    fn normalize_provider_chain_discards_duplicates() {
        assert_eq!(
            normalize_provider_chain(&[
                " TMDB ".to_string(),
                "tmdb".to_string(),
                "tvdb".to_string(),
                "".to_string(),
            ]),
            vec!["tmdb".to_string(), "tvdb".to_string()]
        );
    }

    #[test]
    fn resolve_provider_chain_prefers_library_override() {
        let settings = ScraperPolicySettings {
            default_strategy: "primary_with_fallback".to_string(),
            default_routes: ScraperDefaultRoutes {
                movie: vec!["tmdb".to_string(), "tvdb".to_string()],
                series: vec!["tvdb".to_string(), "tmdb".to_string()],
                image: vec!["fanart".to_string(), "tmdb".to_string()],
            },
        };
        let library_policy = ScraperLibraryPolicy {
            movie: vec!["fanart".to_string(), "tmdb".to_string()],
            series: Vec::new(),
            image: Vec::new(),
        };

        let plan = resolve_provider_chain(
            &settings,
            Some(&library_policy),
            ScraperScenario::MovieMetadata,
            ScraperRoutePurpose::Metadata,
            &["tmdb".to_string(), "fanart".to_string()],
        );
        assert_eq!(
            plan.provider_chain,
            vec!["fanart".to_string(), "tmdb".to_string()]
        );
        assert_eq!(plan.source, "library_override");
    }

    #[test]
    fn external_ids_default_is_empty() {
        assert_eq!(ScrapeExternalIds::default().extra.len(), 0);
        assert!(ScrapeExternalIds::default().bangumi.is_none());
    }

    #[test]
    fn metadata_routes_use_series_default_for_non_movie_scenarios() {
        let settings = ScraperPolicySettings {
            default_strategy: "primary_with_fallback".to_string(),
            default_routes: ScraperDefaultRoutes {
                movie: vec!["tmdb".to_string(), "tvdb".to_string()],
                series: vec![
                    "bangumi".to_string(),
                    "tvdb".to_string(),
                    "tmdb".to_string(),
                ],
                image: vec!["tmdb".to_string(), "fanart".to_string()],
            },
        };
        let plan = resolve_provider_chain(
            &settings,
            None,
            ScraperScenario::SearchByTitle,
            ScraperRoutePurpose::Metadata,
            &[
                "tmdb".to_string(),
                "tvdb".to_string(),
                "bangumi".to_string(),
            ],
        );
        assert_eq!(
            plan.provider_chain,
            vec![
                "bangumi".to_string(),
                "tvdb".to_string(),
                "tmdb".to_string()
            ]
        );
    }

    #[test]
    fn image_routes_prefer_library_image_override() {
        let settings = ScraperPolicySettings {
            default_strategy: "primary_with_fallback".to_string(),
            default_routes: ScraperDefaultRoutes {
                movie: vec!["tmdb".to_string(), "tvdb".to_string()],
                series: vec!["tvdb".to_string(), "tmdb".to_string()],
                image: vec!["tmdb".to_string(), "fanart".to_string()],
            },
        };
        let plan = resolve_provider_chain(
            &settings,
            Some(&ScraperLibraryPolicy {
                movie: vec!["tmdb".to_string()],
                series: vec!["bangumi".to_string(), "tvdb".to_string()],
                image: vec!["tvdb".to_string(), "tmdb".to_string()],
            }),
            ScraperScenario::ImageFetch,
            ScraperRoutePurpose::Image,
            &["tmdb".to_string(), "tvdb".to_string()],
        );
        assert_eq!(
            plan.provider_chain,
            vec!["tvdb".to_string(), "tmdb".to_string()]
        );
        assert_eq!(plan.source, "library_override");
    }

    #[test]
    fn image_routes_fall_back_to_global_image_chain() {
        let settings = ScraperPolicySettings {
            default_strategy: "primary_with_fallback".to_string(),
            default_routes: ScraperDefaultRoutes {
                movie: vec!["tmdb".to_string()],
                series: vec!["bangumi".to_string(), "tvdb".to_string()],
                image: vec!["fanart".to_string(), "tmdb".to_string()],
            },
        };
        let plan = resolve_provider_chain(
            &settings,
            None,
            ScraperScenario::ImageFetch,
            ScraperRoutePurpose::Image,
            &["tmdb".to_string(), "fanart".to_string()],
        );
        assert_eq!(
            plan.provider_chain,
            vec!["fanart".to_string(), "tmdb".to_string()]
        );
        assert_eq!(plan.source, "global_default");
    }
}
