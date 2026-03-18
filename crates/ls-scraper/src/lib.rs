pub mod bangumi;
pub mod model;
pub mod nfo;
pub mod provider;
pub mod tmdb;
pub mod tvdb;

pub use bangumi::{BangumiClient, BangumiConfig, BangumiScrapeResult, BangumiScraperProvider};
pub use model::{
    ImageAssetPatch, PersonPatch, ScrapeCandidate, ScrapeContext, ScrapeExternalIds,
    ScrapeMatchHints, ScrapePatch, ScrapePlan, ScrapeResult, ScraperDefaultRoutes,
    ScraperLibraryPolicy, ScraperPolicySettings, ScraperRoutePurpose, ScraperScenario,
    infer_scenario_from_item_type, normalize_provider_chain, resolve_provider_chain,
};
pub use nfo::{
    NfoDocument, NfoItemKind, NfoSidecarHint, build_nfo_document, read_nfo_sidecar_hints,
    render_nfo_document, write_nfo_document,
};
pub use provider::{
    ScraperCapability, ScraperProviderDescriptor, ScraperProviderHealthReport,
    ScraperProviderStatus,
};
pub use tmdb::TmdbScraperProvider;
pub use tvdb::{TvdbClient, TvdbConfig, TvdbScrapeResult, TvdbScraperProvider};
