//! Anthropic Claude API backend (Claude 4.5/4.6 Sonnet, Opus, Haiku).

use anyhow::{Result, Context, bail};
use serde::{Deserialize, Serialize};
use crate::provider::{AiProvider, AiConfig, AiMessage, AiRole, ModelInfo};

pub struct ClaudeProvider {
    agent: ureq::Agent,
}

impl ClaudeProvider {
    pub fn new() -> Self {
        Self {
            agent: ureq::Agent::new_with_defaults(),
        }
    }
}

#[derive(Serialize)]
struct ClaudeRequest<'a> {
    model: &'a str,
    max_tokens: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<&'a str>,
    messages: Vec<ClaudeMessage<'a>>,
    temperature: f32,
}

#[derive(Serialize)]
struct ClaudeMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
}

#[derive(Deserialize)]
struct ClaudeContent {
    text: Option<String>,
}

fn role_str(role: AiRole) -> &'static str {
    match role {
        AiRole::System => "user", // Claude API uses system as a top-level param
        AiRole::User => "user",
        AiRole::Assistant => "assistant",
    }
}

impl AiProvider for ClaudeProvider {
    fn name(&self) -> &str {
        "Claude (Anthropic)"
    }

    fn ping(&self) -> Result<bool> {
        Ok(true) // API key validation happens on first real request
    }

    fn list_models(&self) -> Result<Vec<ModelInfo>> {
        Ok(vec![
            ModelInfo { id: "claude-sonnet-4-20250514".into(), name: "Claude Sonnet 4".into(), provider: "claude".into(), context_length: 200000 },
            ModelInfo { id: "claude-opus-4-20250514".into(), name: "Claude Opus 4".into(), provider: "claude".into(), context_length: 200000 },
            ModelInfo { id: "claude-haiku-4-5-20251001".into(), name: "Claude Haiku 4.5".into(), provider: "claude".into(), context_length: 200000 },
        ])
    }

    fn chat(&self, config: &AiConfig, messages: &[AiMessage]) -> Result<String> {
        if config.api_key.is_empty() {
            bail!("Clé API Anthropic manquante. Configurez-la dans les paramètres IA.");
        }

        let url = format!("{}/messages", config.endpoint);

        // Claude API: system prompt is a top-level parameter, not a message
        let system = if config.system_prompt.is_empty() {
            None
        } else {
            Some(config.system_prompt.as_str())
        };

        // Filter out system messages (handled above) and ensure alternation
        let mut msgs: Vec<ClaudeMessage> = Vec::new();
        for m in messages {
            if m.role == AiRole::System {
                continue; // Handled as top-level system param
            }
            msgs.push(ClaudeMessage {
                role: role_str(m.role),
                content: &m.content,
            });
        }

        // Claude requires at least one user message
        if msgs.is_empty() {
            msgs.push(ClaudeMessage {
                role: "user",
                content: "Bonjour",
            });
        }

        let req_body = ClaudeRequest {
            model: &config.model,
            max_tokens: config.max_tokens,
            system,
            messages: msgs,
            temperature: config.temperature,
        };

        let resp: ClaudeResponse = self.agent
            .post(&url)
            .header("x-api-key", &config.api_key)
            .header("anthropic-version", "2025-04-14")
            .header("content-type", "application/json")
            .send_json(&req_body)
            .context("Erreur API Claude")?
            .body_mut()
            .read_json()
            .context("Réponse Claude invalide")?;

        resp.content.first()
            .and_then(|c| c.text.clone())
            .ok_or_else(|| anyhow::anyhow!("Réponse Claude vide"))
    }
}
