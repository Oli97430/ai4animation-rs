//! Ollama (local) AI backend — HTTP API to ollama server.

use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use crate::provider::{AiProvider, AiConfig, AiMessage, AiRole, ModelInfo};

pub struct OllamaProvider {
    agent: ureq::Agent,
}

impl OllamaProvider {
    pub fn new() -> Self {
        Self {
            agent: ureq::Agent::new_with_defaults(),
        }
    }
}

#[derive(Serialize)]
struct OllamaChatRequest<'a> {
    model: &'a str,
    messages: Vec<OllamaMessage<'a>>,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Serialize)]
struct OllamaMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: usize,
}

#[derive(Deserialize)]
struct OllamaChatResponse {
    message: OllamaResponseMessage,
}

#[derive(Deserialize)]
struct OllamaResponseMessage {
    content: String,
}

#[derive(Deserialize)]
struct OllamaErrorResponse {
    #[serde(default)]
    error: String,
}

#[derive(Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModel>,
}

#[derive(Deserialize)]
struct OllamaModel {
    name: String,
}

fn role_str(role: AiRole) -> &'static str {
    match role {
        AiRole::System => "system",
        AiRole::User => "user",
        AiRole::Assistant => "assistant",
    }
}

impl AiProvider for OllamaProvider {
    fn name(&self) -> &str {
        "Ollama (Local)"
    }

    fn ping(&self) -> Result<bool> {
        // Use a simple GET to the root endpoint — works on any Ollama version
        let url = "http://localhost:11434/";
        match self.agent.get(url).call() {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let url = "http://localhost:11434/api/tags";
        let resp: OllamaTagsResponse = self.agent.get(url)
            .call()
            .context("Impossible de se connecter à Ollama (est-il lancé?)")?
            .body_mut()
            .read_json()
            .context("Réponse Ollama invalide")?;

        Ok(resp.models.into_iter().map(|m| ModelInfo {
            id: m.name.clone(),
            name: m.name,
            provider: "ollama".into(),
            context_length: 8192,
        }).collect())
    }

    fn chat(&self, config: &AiConfig, messages: &[AiMessage]) -> Result<String> {
        let url = format!("{}/api/chat", config.endpoint);

        let mut msgs: Vec<OllamaMessage> = Vec::with_capacity(messages.len() + 1);

        // Prepend system prompt if configured
        if !config.system_prompt.is_empty() {
            msgs.push(OllamaMessage {
                role: "system",
                content: &config.system_prompt,
            });
        }

        for m in messages {
            msgs.push(OllamaMessage {
                role: role_str(m.role),
                content: &m.content,
            });
        }

        let req_body = OllamaChatRequest {
            model: &config.model,
            messages: msgs,
            stream: false,
            options: OllamaOptions {
                temperature: config.temperature,
                num_predict: config.max_tokens,
            },
        };

        let response = self.agent
            .post(&url)
            .send_json(&req_body);

        match response {
            Ok(mut resp) => {
                let body: String = resp.body_mut().read_to_string()
                    .context("Impossible de lire la réponse Ollama")?;

                // Try to parse as error first
                if let Ok(err) = serde_json::from_str::<OllamaErrorResponse>(&body) {
                    if !err.error.is_empty() {
                        return Err(anyhow::anyhow!("Ollama: {}", err.error));
                    }
                }

                // Parse as chat response
                let chat_resp: OllamaChatResponse = serde_json::from_str(&body)
                    .context("Réponse Ollama invalide")?;
                Ok(chat_resp.message.content)
            }
            Err(ureq::Error::StatusCode(status)) => {
                Err(anyhow::anyhow!(
                    "Ollama erreur HTTP {} — le modèle '{}' est-il installé? (ollama pull {})",
                    status, config.model, config.model
                ))
            }
            Err(e) => {
                Err(anyhow::anyhow!("Erreur de connexion Ollama: {e}"))
            }
        }
    }
}
