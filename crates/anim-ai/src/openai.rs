//! OpenAI API backend (GPT-4o, GPT-4, etc.).

use anyhow::{Result, Context, bail};
use serde::{Deserialize, Serialize};
use crate::provider::{AiProvider, AiConfig, AiMessage, AiRole, ModelInfo};

pub struct OpenAiProvider {
    agent: ureq::Agent,
}

impl OpenAiProvider {
    pub fn new() -> Self {
        Self {
            agent: ureq::Agent::new_with_defaults(),
        }
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    max_tokens: usize,
    temperature: f32,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(Deserialize)]
struct ChatResponseMessage {
    content: Option<String>,
}

fn role_str(role: AiRole) -> &'static str {
    match role {
        AiRole::System => "system",
        AiRole::User => "user",
        AiRole::Assistant => "assistant",
    }
}

impl AiProvider for OpenAiProvider {
    fn name(&self) -> &str {
        "OpenAI"
    }

    fn ping(&self) -> Result<bool> {
        // Just check if we can list models
        Ok(true) // API key validation happens on first real request
    }

    fn list_models(&self) -> Result<Vec<ModelInfo>> {
        // Return common models — listing via API requires auth
        Ok(vec![
            ModelInfo { id: "gpt-4o".into(), name: "GPT-4o".into(), provider: "openai".into(), context_length: 128000 },
            ModelInfo { id: "gpt-4o-mini".into(), name: "GPT-4o Mini".into(), provider: "openai".into(), context_length: 128000 },
            ModelInfo { id: "gpt-4-turbo".into(), name: "GPT-4 Turbo".into(), provider: "openai".into(), context_length: 128000 },
            ModelInfo { id: "o3-mini".into(), name: "o3-mini".into(), provider: "openai".into(), context_length: 200000 },
        ])
    }

    fn chat(&self, config: &AiConfig, messages: &[AiMessage]) -> Result<String> {
        if config.api_key.is_empty() {
            bail!("Clé API OpenAI manquante. Configurez-la dans les paramètres IA.");
        }

        let url = format!("{}/chat/completions", config.endpoint);

        let mut msgs: Vec<ChatMessage> = Vec::with_capacity(messages.len() + 1);

        if !config.system_prompt.is_empty() {
            msgs.push(ChatMessage {
                role: "system",
                content: &config.system_prompt,
            });
        }

        for m in messages {
            msgs.push(ChatMessage {
                role: role_str(m.role),
                content: &m.content,
            });
        }

        let req_body = ChatRequest {
            model: &config.model,
            messages: msgs,
            max_tokens: config.max_tokens,
            temperature: config.temperature,
        };

        let resp: ChatResponse = self.agent
            .post(&url)
            .header("Authorization", &format!("Bearer {}", config.api_key))
            .header("Content-Type", "application/json")
            .send_json(&req_body)
            .context("Erreur API OpenAI")?
            .body_mut()
            .read_json()
            .context("Réponse OpenAI invalide")?;

        resp.choices.first()
            .and_then(|c| c.message.content.clone())
            .ok_or_else(|| anyhow::anyhow!("Réponse OpenAI vide"))
    }
}
