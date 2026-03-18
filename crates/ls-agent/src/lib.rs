pub mod llm;
pub mod model;
pub mod moviepilot;
pub mod provider;
pub mod workflow;

pub use llm::{LlmAgentExecutionPlan, LlmParseResult, LlmProvider};
pub use model::{
    AgentManualAction, AgentRequest, AgentRequestCreateInput, AgentRequestDetail,
    AgentRequestEvent, USER_STATUS_ACTION_REQUIRED, USER_STATUS_CLOSED, USER_STATUS_FAILED,
    USER_STATUS_PROCESSING, USER_STATUS_SUCCESS, admin_status_to_user_status, is_open_admin_status,
    normalize_int_list,
};
pub use moviepilot::{
    MoviePilotClient, MoviePilotContext, MoviePilotDownloadPayload, MoviePilotExactSearchQuery,
    MoviePilotFilterDecision, MoviePilotMediaInfo, MoviePilotProvider, MoviePilotResponse,
    MoviePilotSubscriptionPayload, MoviePilotTorrentInfo, build_download_payload,
    build_download_payload_with_context, build_subscription_payload, choose_best_result,
    decode_search_contexts, filter_search_results, summarize_moviepilot_result,
};
pub use provider::{
    AgentProviderCapability, AgentProviderDescriptor, AgentProviderHealthReport,
    AgentProviderStatus,
};
pub use workflow::{
    AgentWorkflowKind, AgentWorkflowStepKey, AgentWorkflowStepState,
    WorkflowStepStatus as AgentWorkflowStepStatus, infer_manual_actions, infer_workflow_kind,
    infer_workflow_steps, workflow_required_capabilities,
};
