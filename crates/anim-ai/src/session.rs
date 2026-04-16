//! AI session — manages conversation history, background inference, command parsing.

use std::sync::mpsc;

use crate::provider::{AiProvider, AiConfig, AiMessage, AiRole};
use crate::command::{self, AiCommand};
use crate::context::SceneContext;
use crate::ollama::OllamaProvider;
use crate::openai::OpenAiProvider;
use crate::claude::ClaudeProvider;

/// Status of the AI session.
#[derive(Debug, Clone, PartialEq)]
pub enum AiStatus {
    /// Idle, ready for input.
    Ready,
    /// Waiting for AI response.
    Thinking,
    /// Error occurred.
    Error(String),
    /// Not configured / provider unavailable.
    Disconnected,
}

/// A completed exchange in the conversation history.
#[derive(Debug, Clone)]
pub struct ChatEntry {
    pub role: AiRole,
    pub content: String,
    /// Commands extracted from this message (if assistant).
    pub commands: Vec<AiCommand>,
    /// When this message was created.
    pub timestamp: std::time::Instant,
}

/// Message from the background AI thread back to the main thread.
pub enum AiResponse {
    /// AI responded with text + parsed commands.
    Complete {
        text: String,
        commands: Vec<AiCommand>,
    },
    /// AI request failed.
    Failed(String),
}

/// AI session: conversation history + async inference.
pub struct AiSession {
    pub config: AiConfig,
    pub history: Vec<ChatEntry>,
    pub status: AiStatus,

    /// Channel for receiving async responses from the AI thread.
    response_rx: Option<mpsc::Receiver<AiResponse>>,
}

impl AiSession {
    pub fn new() -> Self {
        Self {
            config: AiConfig::default(),
            history: Vec::new(),
            status: AiStatus::Ready,
            response_rx: None,
        }
    }

    /// Create a provider instance based on the current config.
    fn create_provider(config: &AiConfig) -> Box<dyn AiProvider> {
        match config.provider.as_str() {
            "openai" => Box::new(OpenAiProvider::new()),
            "claude" => Box::new(ClaudeProvider::new()),
            _ => Box::new(OllamaProvider::new()), // default = local
        }
    }

    /// Test connectivity to the current provider.
    pub fn test_connection(&self) -> bool {
        let provider = Self::create_provider(&self.config);
        provider.ping().unwrap_or(false)
    }

    /// List models available from the current provider.
    pub fn list_models(&self) -> Vec<crate::provider::ModelInfo> {
        let provider = Self::create_provider(&self.config);
        provider.list_models().unwrap_or_default()
    }

    /// Send a user message. Inference runs on a background thread.
    /// Returns immediately; call `poll()` each frame to check for responses.
    pub fn send(&mut self, user_message: &str, scene_context: Option<&SceneContext>) {
        if self.status == AiStatus::Thinking {
            return; // Already processing
        }

        // Add user message to history
        self.history.push(ChatEntry {
            role: AiRole::User,
            content: user_message.to_string(),
            commands: Vec::new(),
            timestamp: std::time::Instant::now(),
        });

        self.status = AiStatus::Thinking;

        // Build messages for the API
        let mut messages: Vec<AiMessage> = Vec::new();

        // Inject scene context as a system-like user message
        if let Some(ctx) = scene_context {
            let context_msg = format!(
                "[État actuel de l'éditeur]\n{}\n[Fin de l'état]",
                ctx.to_summary()
            );
            messages.push(AiMessage {
                role: AiRole::User,
                content: context_msg,
            });
            messages.push(AiMessage {
                role: AiRole::Assistant,
                content: "Compris, j'ai pris connaissance de l'état actuel de l'éditeur.".into(),
            });
        }

        // Add conversation history (last 20 messages max for context window)
        let start = if self.history.len() > 20 { self.history.len() - 20 } else { 0 };
        for entry in &self.history[start..] {
            messages.push(AiMessage {
                role: entry.role,
                content: entry.content.clone(),
            });
        }

        // Spawn background thread for inference
        let (tx, rx) = mpsc::channel();
        self.response_rx = Some(rx);

        let config = self.config.clone();
        std::thread::spawn(move || {
            let provider = Self::create_provider(&config);
            match provider.chat(&config, &messages) {
                Ok(response_text) => {
                    let (commands, text) = command::parse_commands(&response_text);
                    let display_text = if text.is_empty() && !commands.is_empty() {
                        // If only commands, show a summary
                        let cmd_names: Vec<String> = commands.iter()
                            .map(|c| format!("{:?}", std::mem::discriminant(c)))
                            .collect();
                        format!("[Commandes exécutées: {}]", cmd_names.len())
                    } else if !text.is_empty() && !commands.is_empty() {
                        format!("{}\n\n[+ {} commande(s) exécutée(s)]", text, commands.len())
                    } else {
                        response_text
                    };
                    let _ = tx.send(AiResponse::Complete { text: display_text, commands });
                }
                Err(e) => {
                    let _ = tx.send(AiResponse::Failed(e.to_string()));
                }
            }
        });
    }

    /// Poll for async AI responses. Call this each frame.
    /// Returns parsed commands to execute, if any.
    pub fn poll(&mut self) -> Option<Vec<AiCommand>> {
        let rx = self.response_rx.as_ref()?;

        match rx.try_recv() {
            Ok(AiResponse::Complete { text, commands }) => {
                self.history.push(ChatEntry {
                    role: AiRole::Assistant,
                    content: text,
                    commands: commands.clone(),
                    timestamp: std::time::Instant::now(),
                });
                self.status = AiStatus::Ready;
                self.response_rx = None;
                if commands.is_empty() { None } else { Some(commands) }
            }
            Ok(AiResponse::Failed(err)) => {
                self.history.push(ChatEntry {
                    role: AiRole::Assistant,
                    content: format!("❌ Erreur: {}", err),
                    commands: Vec::new(),
                    timestamp: std::time::Instant::now(),
                });
                self.status = AiStatus::Error(err);
                self.response_rx = None;
                None
            }
            Err(mpsc::TryRecvError::Empty) => None, // Still waiting
            Err(mpsc::TryRecvError::Disconnected) => {
                self.status = AiStatus::Error("Thread IA déconnecté".into());
                self.response_rx = None;
                None
            }
        }
    }

    /// Clear conversation history.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Update the configuration.
    pub fn set_config(&mut self, config: AiConfig) {
        self.config = config;
    }
}

impl Default for AiSession {
    fn default() -> Self {
        Self::new()
    }
}
