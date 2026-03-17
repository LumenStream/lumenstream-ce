use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AgentProviderCapability {
    Search,
    Subscribe,
    Download,
    Metadata,
    Notify,
}

impl AgentProviderCapability {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Search => "search",
            Self::Subscribe => "subscribe",
            Self::Download => "download",
            Self::Metadata => "metadata",
            Self::Notify => "notify",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProviderHealthReport {
    pub healthy: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProviderStatus {
    pub provider_id: String,
    pub display_name: String,
    pub provider_kind: String,
    pub enabled: bool,
    pub configured: bool,
    pub healthy: bool,
    pub capabilities: Vec<String>,
    pub message: String,
    pub checked_at: Option<DateTime<Utc>>,
}

pub trait AgentProviderDescriptor {
    fn provider_id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn provider_kind(&self) -> &'static str;
    fn capabilities(&self) -> Vec<AgentProviderCapability>;
}
