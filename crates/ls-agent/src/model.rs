use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

pub const USER_STATUS_PROCESSING: &str = "processing";
pub const USER_STATUS_SUCCESS: &str = "success";
pub const USER_STATUS_FAILED: &str = "failed";
pub const USER_STATUS_ACTION_REQUIRED: &str = "action_required";
pub const USER_STATUS_CLOSED: &str = "closed";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRequest {
    pub id: Uuid,
    pub request_type: String,
    pub source: String,
    pub user_id: Option<Uuid>,
    pub title: String,
    pub content: String,
    pub media_type: String,
    pub tmdb_id: Option<i64>,
    pub media_item_id: Option<Uuid>,
    pub series_id: Option<Uuid>,
    pub season_numbers: Vec<i32>,
    pub episode_numbers: Vec<i32>,
    pub status_user: String,
    pub status_admin: String,
    pub agent_stage: String,
    pub priority: i32,
    pub auto_handled: bool,
    pub admin_note: String,
    pub agent_note: String,
    pub provider_payload: Value,
    pub provider_result: Value,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRequestEvent {
    pub id: Uuid,
    pub request_id: Uuid,
    pub event_type: String,
    pub actor_user_id: Option<Uuid>,
    pub actor_username: Option<String>,
    pub summary: String,
    pub detail: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentManualAction {
    pub action: String,
    pub label: String,
    pub description: String,
}

impl AgentManualAction {
    pub fn new(action: &str, label: &str, description: &str) -> Self {
        Self {
            action: action.to_string(),
            label: label.to_string(),
            description: description.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRequestDetail {
    pub request: AgentRequest,
    pub events: Vec<AgentRequestEvent>,
    pub workflow_kind: String,
    pub workflow_steps: Vec<crate::workflow::AgentWorkflowStepState>,
    pub required_capabilities: Vec<String>,
    pub manual_actions: Vec<AgentManualAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentRequestCreateInput {
    pub request_type: String,
    pub source: String,
    pub title: String,
    pub content: String,
    pub media_type: String,
    pub tmdb_id: Option<i64>,
    pub media_item_id: Option<Uuid>,
    pub series_id: Option<Uuid>,
    pub season_numbers: Vec<i32>,
    pub episode_numbers: Vec<i32>,
}

pub fn normalize_int_list(values: &[i32]) -> Vec<i32> {
    let mut out = values
        .iter()
        .copied()
        .filter(|value| *value > 0)
        .collect::<Vec<_>>();
    out.sort_unstable();
    out.dedup();
    out
}

pub fn is_open_admin_status(status: &str) -> bool {
    matches!(
        status,
        "new" | "analyzing" | "auto_processing" | "review_required" | "approved"
    )
}

pub fn admin_status_to_user_status(status: &str) -> &'static str {
    match status {
        "completed" => USER_STATUS_SUCCESS,
        "rejected" | "failed" => USER_STATUS_FAILED,
        "ignored" => USER_STATUS_CLOSED,
        "review_required" => USER_STATUS_ACTION_REQUIRED,
        _ => USER_STATUS_PROCESSING,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        USER_STATUS_ACTION_REQUIRED, USER_STATUS_FAILED, USER_STATUS_PROCESSING,
        USER_STATUS_SUCCESS, admin_status_to_user_status, is_open_admin_status, normalize_int_list,
    };

    #[test]
    fn normalize_int_list_discards_non_positive_and_dedupes() {
        assert_eq!(normalize_int_list(&[3, 1, 1, 0, -2, 2]), vec![1, 2, 3]);
    }

    #[test]
    fn open_status_detection_matches_expected_states() {
        assert!(is_open_admin_status("new"));
        assert!(is_open_admin_status("review_required"));
        assert!(!is_open_admin_status("completed"));
        assert!(!is_open_admin_status("failed"));
    }

    #[test]
    fn user_status_mapping_matches_admin_status() {
        assert_eq!(admin_status_to_user_status("new"), USER_STATUS_PROCESSING);
        assert_eq!(
            admin_status_to_user_status("review_required"),
            USER_STATUS_ACTION_REQUIRED
        );
        assert_eq!(
            admin_status_to_user_status("completed"),
            USER_STATUS_SUCCESS
        );
        assert_eq!(admin_status_to_user_status("failed"), USER_STATUS_FAILED);
    }
}
