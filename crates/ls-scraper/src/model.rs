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
    pub scenario_defaults: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ScraperLibraryPolicy {
    #[serde(default)]
    pub scenario_defaults: BTreeMap<String, Vec<String>>,
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
    available: &[String],
) -> ScrapePlan {
    let scenario_key = scenario.as_str().to_string();
    let library_chain = library_policy
        .and_then(|policy| policy.scenario_defaults.get(&scenario_key))
        .cloned()
        .unwrap_or_default();
    let default_chain = settings
        .scenario_defaults
        .get(&scenario_key)
        .cloned()
        .unwrap_or_default();

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
    use std::collections::BTreeMap;

    use super::{
        ScrapeExternalIds, ScraperLibraryPolicy, ScraperPolicySettings, ScraperScenario,
        infer_scenario_from_item_type, normalize_provider_chain, resolve_provider_chain,
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
        let mut scenario_defaults = BTreeMap::new();
        scenario_defaults.insert(
            "movie_metadata".to_string(),
            vec!["tmdb".to_string(), "tvdb".to_string()],
        );
        let settings = ScraperPolicySettings {
            default_strategy: "primary_with_fallback".to_string(),
            scenario_defaults,
        };
        let mut library_defaults = BTreeMap::new();
        library_defaults.insert(
            "movie_metadata".to_string(),
            vec!["fanart".to_string(), "tmdb".to_string()],
        );
        let library_policy = ScraperLibraryPolicy {
            scenario_defaults: library_defaults,
        };

        let plan = resolve_provider_chain(
            &settings,
            Some(&library_policy),
            ScraperScenario::MovieMetadata,
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
    fn library_override_can_explicitly_enable_bangumi() {
        let mut scenario_defaults = BTreeMap::new();
        scenario_defaults.insert(
            "series_metadata".to_string(),
            vec!["tmdb".to_string(), "tvdb".to_string()],
        );
        let settings = ScraperPolicySettings {
            default_strategy: "primary_with_fallback".to_string(),
            scenario_defaults,
        };
        let mut library_defaults = BTreeMap::new();
        library_defaults.insert(
            "series_metadata".to_string(),
            vec![
                "bangumi".to_string(),
                "tvdb".to_string(),
                "tmdb".to_string(),
            ],
        );
        let plan = resolve_provider_chain(
            &settings,
            Some(&ScraperLibraryPolicy {
                scenario_defaults: library_defaults,
            }),
            ScraperScenario::SeriesMetadata,
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
}
