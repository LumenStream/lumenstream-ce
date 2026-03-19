use serde::{Deserialize, Serialize};

use crate::{AgentManualAction, provider::AgentProviderCapability};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentWorkflowKind {
    RequestMedia,
    MissingEpisodeRepair,
    MissingSeasonRepair,
    FeedbackTriage,
    Unknown,
}

impl AgentWorkflowKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RequestMedia => "request_media",
            Self::MissingEpisodeRepair => "missing_episode_repair",
            Self::MissingSeasonRepair => "missing_season_repair",
            Self::FeedbackTriage => "feedback_triage",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentWorkflowStepKey {
    Accepted,
    Normalize,
    LibraryCheck,
    GapDetect,
    MetadataEnrich,
    ProviderSearch,
    FilterDispatch,
    ManualReview,
    Verify,
    Notify,
}

impl AgentWorkflowStepKey {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Normalize => "normalize",
            Self::LibraryCheck => "library_check",
            Self::GapDetect => "gap_detect",
            Self::MetadataEnrich => "metadata_enrich",
            Self::ProviderSearch => "provider_search",
            Self::FilterDispatch => "filter_dispatch",
            Self::ManualReview => "manual_review",
            Self::Verify => "verify",
            Self::Notify => "notify",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Accepted => "接单",
            Self::Normalize => "标准化",
            Self::LibraryCheck => "库内检查",
            Self::GapDetect => "缺口检测",
            Self::MetadataEnrich => "元数据补全",
            Self::ProviderSearch => "Provider 搜索",
            Self::FilterDispatch => "筛选与派发",
            Self::ManualReview => "人工接管",
            Self::Verify => "结果校验",
            Self::Notify => "通知回写",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStepStatus {
    Pending,
    Active,
    Completed,
    Blocked,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentWorkflowStepState {
    pub step: String,
    pub label: String,
    pub status: WorkflowStepStatus,
}

pub fn infer_workflow_kind(request_type: &str) -> AgentWorkflowKind {
    match request_type {
        "media_request" | "replace_source" => AgentWorkflowKind::RequestMedia,
        "missing_episode" => AgentWorkflowKind::MissingEpisodeRepair,
        "missing_season" => AgentWorkflowKind::MissingSeasonRepair,
        "feedback" => AgentWorkflowKind::FeedbackTriage,
        _ => AgentWorkflowKind::Unknown,
    }
}

pub fn workflow_required_capabilities(kind: &AgentWorkflowKind) -> Vec<AgentProviderCapability> {
    match kind {
        AgentWorkflowKind::RequestMedia => vec![
            AgentProviderCapability::Search,
            AgentProviderCapability::Download,
            AgentProviderCapability::Subscribe,
            AgentProviderCapability::Notify,
        ],
        AgentWorkflowKind::MissingEpisodeRepair | AgentWorkflowKind::MissingSeasonRepair => vec![
            AgentProviderCapability::Metadata,
            AgentProviderCapability::Search,
            AgentProviderCapability::Download,
            AgentProviderCapability::Subscribe,
            AgentProviderCapability::Notify,
        ],
        AgentWorkflowKind::FeedbackTriage => vec![AgentProviderCapability::Notify],
        AgentWorkflowKind::Unknown => vec![AgentProviderCapability::Notify],
    }
}

pub fn infer_workflow_steps(
    kind: &AgentWorkflowKind,
    agent_stage: &str,
    status_admin: &str,
) -> Vec<AgentWorkflowStepState> {
    let steps = workflow_steps(kind);
    let active = map_stage_to_step(agent_stage);
    let terminal = if matches!(status_admin, "failed" | "rejected") {
        Some(WorkflowStepStatus::Failed)
    } else if status_admin == "review_required" {
        Some(WorkflowStepStatus::Blocked)
    } else {
        None
    };

    steps
        .iter()
        .enumerate()
        .map(|(idx, step)| {
            let status = if matches!(status_admin, "completed" | "ignored") {
                WorkflowStepStatus::Completed
            } else if let Some(active) = active.as_ref() {
                let active_idx = steps
                    .iter()
                    .position(|candidate| candidate == active)
                    .unwrap_or(0);
                if idx < active_idx {
                    WorkflowStepStatus::Completed
                } else if idx == active_idx {
                    terminal.clone().unwrap_or(WorkflowStepStatus::Active)
                } else if status_admin == "review_required"
                    && step == &AgentWorkflowStepKey::ManualReview
                {
                    WorkflowStepStatus::Blocked
                } else {
                    WorkflowStepStatus::Pending
                }
            } else if idx == 0 {
                WorkflowStepStatus::Active
            } else {
                WorkflowStepStatus::Pending
            };

            AgentWorkflowStepState {
                step: step.as_str().to_string(),
                label: step.label().to_string(),
                status,
            }
        })
        .collect()
}

pub fn infer_manual_actions(status_admin: &str, auto_handled: bool) -> Vec<AgentManualAction> {
    let mut actions = Vec::new();
    if matches!(status_admin, "review_required" | "waiting_user") {
        actions.push(AgentManualAction::new(
            "approve",
            "批准并重试",
            "确认自动策略可继续执行，并重新投入自动处理。",
        ));
        actions.push(AgentManualAction::new(
            "reject",
            "拒绝",
            "拒绝该工单并通知用户结果。",
        ));
    }

    if matches!(
        status_admin,
        "new"
            | "analyzing"
            | "auto_processing"
            | "approved"
            | "failed"
            | "review_required"
            | "waiting_user"
    ) {
        actions.push(AgentManualAction::new(
            "manual_complete",
            "手动完成",
            "绕过自动链路，直接将工单标记为完成。",
        ));
        actions.push(AgentManualAction::new(
            "retry",
            "重新触发",
            "重新执行当前工作流，适合临时失败或补参后重跑。",
        ));
    }

    if !matches!(status_admin, "ignored" | "completed") {
        actions.push(AgentManualAction::new(
            "ignore",
            "忽略",
            "关闭当前工单，不再继续自动处理。",
        ));
    }

    if auto_handled && matches!(status_admin, "auto_processing" | "approved") {
        actions.push(AgentManualAction::new(
            "handoff",
            "转人工接管",
            "保留上下文并切换为人工处理模式。",
        ));
    }

    actions
}

fn workflow_steps(kind: &AgentWorkflowKind) -> Vec<AgentWorkflowStepKey> {
    match kind {
        AgentWorkflowKind::RequestMedia => vec![
            AgentWorkflowStepKey::Accepted,
            AgentWorkflowStepKey::Normalize,
            AgentWorkflowStepKey::LibraryCheck,
            AgentWorkflowStepKey::ProviderSearch,
            AgentWorkflowStepKey::FilterDispatch,
            AgentWorkflowStepKey::Verify,
            AgentWorkflowStepKey::Notify,
        ],
        AgentWorkflowKind::MissingEpisodeRepair | AgentWorkflowKind::MissingSeasonRepair => vec![
            AgentWorkflowStepKey::Accepted,
            AgentWorkflowStepKey::GapDetect,
            AgentWorkflowStepKey::MetadataEnrich,
            AgentWorkflowStepKey::ProviderSearch,
            AgentWorkflowStepKey::FilterDispatch,
            AgentWorkflowStepKey::Verify,
            AgentWorkflowStepKey::Notify,
        ],
        AgentWorkflowKind::FeedbackTriage => vec![
            AgentWorkflowStepKey::Accepted,
            AgentWorkflowStepKey::Normalize,
            AgentWorkflowStepKey::ManualReview,
            AgentWorkflowStepKey::Notify,
        ],
        AgentWorkflowKind::Unknown => vec![
            AgentWorkflowStepKey::Accepted,
            AgentWorkflowStepKey::Normalize,
            AgentWorkflowStepKey::ManualReview,
            AgentWorkflowStepKey::Notify,
        ],
    }
}

fn map_stage_to_step(stage: &str) -> Option<AgentWorkflowStepKey> {
    match stage {
        "queued" => Some(AgentWorkflowStepKey::Accepted),
        "normalize" => Some(AgentWorkflowStepKey::Normalize),
        "analyzing" => Some(AgentWorkflowStepKey::Normalize),
        "searching" => Some(AgentWorkflowStepKey::ProviderSearch),
        "awaiting_user" => Some(AgentWorkflowStepKey::ManualReview),
        "finalizing" => Some(AgentWorkflowStepKey::FilterDispatch),
        "completed" => Some(AgentWorkflowStepKey::Notify),
        "failed" => Some(AgentWorkflowStepKey::Notify),
        "manual_review" => Some(AgentWorkflowStepKey::ManualReview),
        "library_check" => Some(AgentWorkflowStepKey::LibraryCheck),
        "gap_detect" => Some(AgentWorkflowStepKey::GapDetect),
        "metadata_enrich" => Some(AgentWorkflowStepKey::MetadataEnrich),
        "mp_search" => Some(AgentWorkflowStepKey::ProviderSearch),
        "mp_download" | "mp_subscribe" => Some(AgentWorkflowStepKey::FilterDispatch),
        "verify" => Some(AgentWorkflowStepKey::Verify),
        "notify" => Some(AgentWorkflowStepKey::Notify),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AgentWorkflowKind, WorkflowStepStatus, infer_manual_actions, infer_workflow_kind,
        infer_workflow_steps, workflow_required_capabilities,
    };

    #[test]
    fn request_type_maps_to_expected_workflow_kind() {
        assert_eq!(
            infer_workflow_kind("media_request"),
            AgentWorkflowKind::RequestMedia
        );
        assert_eq!(
            infer_workflow_kind("replace_source"),
            AgentWorkflowKind::RequestMedia
        );
        assert_eq!(
            infer_workflow_kind("missing_episode"),
            AgentWorkflowKind::MissingEpisodeRepair
        );
    }

    #[test]
    fn workflow_steps_mark_review_as_blocked() {
        let steps = infer_workflow_steps(
            &AgentWorkflowKind::FeedbackTriage,
            "manual_review",
            "review_required",
        );
        assert!(steps.iter().any(|step| {
            step.step == "manual_review" && step.status == WorkflowStepStatus::Blocked
        }));
    }

    #[test]
    fn manual_actions_include_handoff_for_auto_processing() {
        let actions = infer_manual_actions("auto_processing", true);
        assert!(actions.iter().any(|action| action.action == "handoff"));
    }

    #[test]
    fn missing_episode_workflow_requires_metadata_capability() {
        let caps = workflow_required_capabilities(&AgentWorkflowKind::MissingEpisodeRepair);
        assert!(caps.iter().any(|cap| cap.as_str() == "metadata"));
    }
}
