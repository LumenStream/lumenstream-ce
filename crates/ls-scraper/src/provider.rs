use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ScraperCapability {
    Search,
    Details,
    Images,
    People,
    ExternalIds,
}

impl ScraperCapability {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Search => "search",
            Self::Details => "details",
            Self::Images => "images",
            Self::People => "people",
            Self::ExternalIds => "external_ids",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScraperProviderHealthReport {
    pub healthy: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScraperProviderStatus {
    pub provider_id: String,
    pub display_name: String,
    pub provider_kind: String,
    pub enabled: bool,
    pub configured: bool,
    pub healthy: bool,
    pub capabilities: Vec<String>,
    pub scenarios: Vec<String>,
    pub message: String,
    pub checked_at: Option<DateTime<Utc>>,
}

pub trait ScraperProviderDescriptor {
    fn provider_id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn provider_kind(&self) -> &'static str;
    fn capabilities(&self) -> Vec<ScraperCapability>;
    fn scenarios(&self) -> Vec<crate::model::ScraperScenario>;
}
