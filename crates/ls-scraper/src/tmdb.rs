use crate::{ScraperCapability, ScraperProviderDescriptor, ScraperScenario};

#[derive(Debug, Default, Clone, Copy)]
pub struct TmdbScraperProvider;

impl ScraperProviderDescriptor for TmdbScraperProvider {
    fn provider_id(&self) -> &'static str {
        "tmdb"
    }

    fn display_name(&self) -> &'static str {
        "TMDB"
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
