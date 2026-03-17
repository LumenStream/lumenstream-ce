use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use ls_config::AgentLlmConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmParseResult {
    pub media_type: String,
    pub title: String,
    pub season_numbers: Vec<i32>,
    pub episode_numbers: Vec<i32>,
    pub is_ambiguous: bool,
    pub original_text: String,
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

        let system_prompt = "You are an AI assistant specialized in parsing user requests for media downloads.
Your task is to extract the intended media type, title, season numbers, and episode numbers from the user's natural language request.
Output the result in strict JSON format.

The JSON schema should be:
{
    \"media_type\": \"movie\" | \"series\" | \"unknown\",
    \"title\": \"string\",
    \"season_numbers\": [number],
    \"episode_numbers\": [number],
    \"is_ambiguous\": boolean
}

Rules:
1. If the user mentions a movie, set media_type to 'movie'.
2. If the user mentions a TV show, anime, or series, set media_type to 'series'.
3. Extract the title as accurately as possible. If it's ambiguous, set is_ambiguous to true.
4. Extract season and episode numbers if present. If not present, use empty arrays.
5. If you cannot determine the media_type or title, set is_ambiguous to true.";

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
                    "content": text
                }
            ],
            "response_format": { "type": "json_object" },
            "temperature": 0.1
        });

        let response = self
            .client
            .post(&endpoint)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("LLM API request failed: {}", error_text);
        }

        let result: Value = response.json().await?;
        let content = result
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid response format from LLM API"))?;

        let mut parsed: LlmParseResult = serde_json::from_str(content)?;
        parsed.original_text = text.to_string();

        Ok(parsed)
    }
}
