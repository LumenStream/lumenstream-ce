use anyhow::Context;
use reqwest::Client;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};

use ls_config::AgentLlmConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmParseResult {
    #[serde(default)]
    pub request_type: String,
    pub media_type: String,
    pub title: String,
    #[serde(default)]
    pub season_numbers: Vec<i32>,
    #[serde(default)]
    pub episode_numbers: Vec<i32>,
    #[serde(default)]
    pub requires_media_search: bool,
    #[serde(default)]
    pub preferred_sources: Vec<String>,
    #[serde(default)]
    pub avoid_sources: Vec<String>,
    #[serde(default)]
    pub constraints: Vec<String>,
    #[serde(default)]
    pub is_ambiguous: bool,
    #[serde(default)]
    pub original_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmAgentExecutionPlan {
    pub action: String,
    #[serde(default)]
    pub selected_indices: Vec<usize>,
    #[serde(default)]
    pub add_subscription: bool,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub subscription_reason: Option<String>,
    #[serde(default)]
    pub reject_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmAgentLoopAction {
    pub action: String,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub year: Option<i32>,
    #[serde(default)]
    pub media_type: Option<String>,
    #[serde(default)]
    pub season: Option<i32>,
    #[serde(default)]
    pub selected_indices: Vec<usize>,
    #[serde(default)]
    pub question_prompt: Option<String>,
    #[serde(default)]
    pub question_helper_text: Option<String>,
    #[serde(default)]
    pub question_context_brief: Option<String>,
    #[serde(default)]
    pub question_options: Vec<String>,
    #[serde(default)]
    pub allow_free_text: bool,
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct LlmProvider {
    config: AgentLlmConfig,
    client: Client,
}

impl LlmProvider {
    pub fn new(config: &AgentLlmConfig) -> anyhow::Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        Ok(Self {
            config: config.clone(),
            client,
        })
    }

    pub fn is_configured(&self) -> bool {
        self.config.enabled
            && !self.config.base_url.is_empty()
            && !self.config.api_key.is_empty()
            && !self.config.model.is_empty()
    }

    pub async fn parse_intent(&self, text: &str) -> anyhow::Result<LlmParseResult> {
        if !self.is_configured() {
            anyhow::bail!("LLM provider is not configured or disabled");
        }

        let system_prompt = r#"You are an AI assistant specialized in parsing media-related user requests.
Your task is to recognize the user's actual intent and call the provided tool exactly once.

Guidelines:
1. Use request_type=feedback only when the text is clearly not asking the system to process any media title.
2. If the user wants the system to search, replace source, refresh resource, download, 补档, 补集, 补季, 换源, or otherwise act on a media title, set requires_media_search=true.
3. If the user mentions a movie, set media_type=movie. If the user mentions a TV show, anime, or series, set media_type=series.
4. If the request is specifically asking to replace or avoid a current source/platform, use request_type=replace_source.
5. If the request is about missing episodes, use request_type=missing_episode. If it is about missing seasons, use request_type=missing_season. Otherwise use media_request for media-related intents.
6. Extract the media title as accurately as possible.
7. Put preferred platforms/providers into preferred_sources, and providers to avoid into avoid_sources.
8. Put additional preferences like 4K, HDR, subtitles, ad-free, bitrate or quality into constraints.
9. If the media title or type is unclear, set is_ambiguous=true.
10. Always call the tool; do not answer in plain text."#;

        let schema = json!({
            "type": "object",
            "properties": {
                "request_type": {
                    "type": "string",
                    "enum": ["media_request", "replace_source", "missing_episode", "missing_season", "feedback"]
                },
                "media_type": {
                    "type": "string",
                    "enum": ["movie", "series", "unknown"]
                },
                "title": { "type": "string" },
                "season_numbers": {
                    "type": "array",
                    "items": { "type": "integer" },
                    "default": []
                },
                "episode_numbers": {
                    "type": "array",
                    "items": { "type": "integer" },
                    "default": []
                },
                "requires_media_search": { "type": "boolean", "default": false },
                "preferred_sources": {
                    "type": "array",
                    "items": { "type": "string" },
                    "default": []
                },
                "avoid_sources": {
                    "type": "array",
                    "items": { "type": "string" },
                    "default": []
                },
                "constraints": {
                    "type": "array",
                    "items": { "type": "string" },
                    "default": []
                },
                "is_ambiguous": { "type": "boolean", "default": false }
            },
            "required": [
                "request_type",
                "media_type",
                "title",
                "season_numbers",
                "episode_numbers",
                "requires_media_search",
                "preferred_sources",
                "avoid_sources",
                "constraints",
                "is_ambiguous"
            ],
            "additionalProperties": false
        });

        let mut parsed: LlmParseResult = self
            .complete_with_tool(
                system_prompt,
                text,
                "parse_media_request_intent",
                "Extract the user's intent, media entity and source preferences.",
                schema,
            )
            .await?;
        parsed.original_text = text.to_string();
        Ok(parsed)
    }

    pub async fn plan_request_execution(
        &self,
        context: &Value,
    ) -> anyhow::Result<LlmAgentExecutionPlan> {
        if !self.is_configured() {
            anyhow::bail!("LLM provider is not configured or disabled");
        }

        let system_prompt = r#"You are an autonomous media request agent.
You must inspect the provided request context and call the execution planning tool exactly once.

Decision goals:
1. Prefer torrents with the highest seeders.
2. Never choose BluRay / Remux / BDMV / ISO disc-style content.
3. Prefer HDR/HDR10+ and 4K when available.
4. Avoid Dolby Vision / DoVi content.
5. For movies, normally choose at most one torrent.
6. For TV series, you may choose multiple torrents to cover as many episodes as possible.
7. If the series is still ongoing, or current torrents cannot cover enough released episodes, set add_subscription=true.
8. If no safe/usable torrent exists but subscription is appropriate, choose subscribe.
9. If metadata indicates the movie is still in theaters and auto rejection is appropriate, choose reject.
10. If the situation is ambiguous or risky, choose manual_review.
11. Always call the tool; do not answer in plain text."#;

        let schema = json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["download", "download_and_subscribe", "subscribe", "manual_review", "reject"]
                },
                "selected_indices": {
                    "type": "array",
                    "items": { "type": "integer", "minimum": 0 },
                    "default": []
                },
                "add_subscription": { "type": "boolean", "default": false },
                "reason": { "type": "string" },
                "subscription_reason": { "type": ["string", "null"] },
                "reject_reason": { "type": ["string", "null"] }
            },
            "required": [
                "action",
                "selected_indices",
                "add_subscription",
                "reason",
                "subscription_reason",
                "reject_reason"
            ],
            "additionalProperties": false
        });

        self.complete_with_tool(
            system_prompt,
            &serde_json::to_string(context).unwrap_or_else(|_| "{}".to_string()),
            "plan_media_request_execution",
            "Decide whether to download, subscribe, reject or send the request to manual review.",
            schema,
        )
        .await
    }

    pub async fn decide_loop_action(&self, context: &Value) -> anyhow::Result<LlmAgentLoopAction> {
        if !self.is_configured() {
            anyhow::bail!("LLM provider is not configured or disabled");
        }

        let system_prompt = r#"You are an autonomous media request agent running inside a strict loop runtime.
You must inspect the provided context and call the action selection tool exactly once.

Loop rules:
1. Choose exactly one next action for this round.
2. Prefer to resolve metadata ambiguity before searching MoviePilot.
3. MoviePilot search MUST include a year. If the year is unknown, ask the user or use metadata tools first.
4. Use bangumi only for anime/series style titles or when other metadata tools are weak.
5. Use tvdb for series-oriented matching and episode context.
6. Use tmdb for general movie/series matching and year confirmation.
7. If there is enough information and candidate resources exist, decide whether to download, subscribe, or both.
8. If the case is blocked or risky, ask the user or send to manual_review.
9. Never expose internal reasoning in question text or reason fields.
10. Always call the tool; do not answer in plain text."#;

        let schema = json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": [
                        "tmdb_search",
                        "tvdb_search",
                        "bangumi_search",
                        "moviepilot_search",
                        "ask_user",
                        "complete_download",
                        "complete_subscription",
                        "complete_download_and_subscription",
                        "manual_review",
                        "fail_request"
                    ]
                },
                "query": { "type": ["string", "null"] },
                "year": { "type": ["integer", "null"] },
                "media_type": { "type": ["string", "null"] },
                "season": { "type": ["integer", "null"] },
                "selected_indices": {
                    "type": "array",
                    "items": { "type": "integer", "minimum": 0 },
                    "default": []
                },
                "question_prompt": { "type": ["string", "null"] },
                "question_helper_text": { "type": ["string", "null"] },
                "question_context_brief": { "type": ["string", "null"] },
                "question_options": {
                    "type": "array",
                    "items": { "type": "string" },
                    "default": []
                },
                "allow_free_text": { "type": "boolean", "default": true },
                "reason": { "type": "string" }
            },
            "required": [
                "action",
                "query",
                "year",
                "media_type",
                "season",
                "selected_indices",
                "question_prompt",
                "question_helper_text",
                "question_context_brief",
                "question_options",
                "allow_free_text",
                "reason"
            ],
            "additionalProperties": false
        });

        self.complete_with_tool(
            system_prompt,
            &serde_json::to_string(context).unwrap_or_else(|_| "{}".to_string()),
            "decide_agent_loop_action",
            "Choose the single next action for the current agent loop round.",
            schema,
        )
        .await
    }

    async fn complete_with_tool<T: DeserializeOwned>(
        &self,
        system_prompt: &str,
        user_content: &str,
        tool_name: &str,
        tool_description: &str,
        parameters_schema: Value,
    ) -> anyhow::Result<T> {
        let endpoint = format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        );

        let payload = json!({
            "model": self.config.model,
            "messages": [
                {
                    "role": "system",
                    "content": system_prompt
                },
                {
                    "role": "user",
                    "content": user_content
                }
            ],
            "tools": [
                {
                    "type": "function",
                    "function": {
                        "name": tool_name,
                        "description": tool_description,
                        "parameters": parameters_schema
                    }
                }
            ],
            "tool_choice": {
                "type": "function",
                "function": {
                    "name": tool_name
                }
            },
            "temperature": 0.1
        });

        let response = self
            .client
            .post(&endpoint)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .context("failed to call LLM tool endpoint")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("LLM API request failed: {}", error_text);
        }

        let result: Value = response
            .json()
            .await
            .context("failed to parse LLM response JSON")?;
        let arguments = extract_tool_arguments(&result, tool_name)?;
        serde_json::from_str(&arguments).context("failed to deserialize tool arguments")
    }
}

fn extract_tool_arguments(response: &Value, expected_tool_name: &str) -> anyhow::Result<String> {
    let message = response
        .get("choices")
        .and_then(|choices| choices.get(0))
        .and_then(|choice| choice.get("message"))
        .context("missing choices[0].message in LLM response")?;

    if let Some(arguments) = message
        .get("tool_calls")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find_map(|call| {
            let function = call.get("function")?;
            let name = function.get("name")?.as_str()?;
            if name == expected_tool_name {
                function.get("arguments")?.as_str().map(str::to_string)
            } else {
                None
            }
        })
    {
        return Ok(arguments);
    }

    if let Some(arguments) = message
        .get("function_call")
        .and_then(Value::as_object)
        .and_then(|call| {
            let name = call.get("name")?.as_str()?;
            if name == expected_tool_name {
                call.get("arguments")?.as_str().map(str::to_string)
            } else {
                None
            }
        })
    {
        return Ok(arguments);
    }

    anyhow::bail!("LLM response did not contain expected tool call: {expected_tool_name}")
}
