//! AI chat panel — promptable interface to control the entire editor.
//!
//! Users type natural language commands; the AI responds with text and/or
//! structured commands that are executed on the editor.

use egui::{Ui, Color32, RichText, ScrollArea, Stroke, Key};
use anim_ai::{AiSession, AiCommand, SceneContext};
use anim_ai::provider::AiRole;
use anim_ai::session::AiStatus;
use crate::app_state::AppState;
use crate::theme::accent;

/// A file attached by the user for the AI to process.
#[derive(Debug, Clone)]
pub struct AttachedFile {
    /// Full path on disk.
    pub path: std::path::PathBuf,
    /// Display name (filename only).
    pub name: String,
    /// File size in bytes.
    pub size: u64,
    /// Recognized category for auto-import.
    pub category: FileCategory,
}

/// Category of an attached file (determines how the AI processes it).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileCategory {
    /// 3D model (.glb, .gltf, .fbx)
    Model,
    /// Animation (.bvh, .npz)
    Animation,
    /// AI/ONNX model (.onnx, .pt)
    AiModel,
    /// Metadata/config (.json, .toml, .yaml)
    Config,
    /// Image/texture (.png, .jpg, .hdr)
    Image,
    /// Unknown file type
    Other,
}

impl FileCategory {
    fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "glb" | "gltf" | "fbx" => Self::Model,
            "bvh" | "npz" => Self::Animation,
            "onnx" | "pt" | "pth" => Self::AiModel,
            "json" | "toml" | "yaml" | "yml" => Self::Config,
            "png" | "jpg" | "jpeg" | "hdr" | "exr" => Self::Image,
            _ => Self::Other,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Model => "Modèle 3D",
            Self::Animation => "Animation",
            Self::AiModel => "Modèle IA",
            Self::Config => "Config",
            Self::Image => "Image",
            Self::Other => "Fichier",
        }
    }

    fn color(&self) -> Color32 {
        match self {
            Self::Model => Color32::from_rgb(100, 180, 255),
            Self::Animation => Color32::from_rgb(255, 180, 80),
            Self::AiModel => Color32::from_rgb(180, 120, 255),
            Self::Config => Color32::from_rgb(120, 200, 140),
            Self::Image => Color32::from_rgb(255, 140, 160),
            Self::Other => Color32::from_rgb(160, 160, 170),
        }
    }
}

impl AttachedFile {
    fn from_path(path: std::path::PathBuf) -> Self {
        let name = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let ext = path.extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();
        let category = FileCategory::from_extension(&ext);
        Self { path, name, size, category }
    }

    fn size_display(&self) -> String {
        if self.size < 1024 {
            format!("{} o", self.size)
        } else if self.size < 1024 * 1024 {
            format!("{:.1} Ko", self.size as f64 / 1024.0)
        } else {
            format!("{:.1} Mo", self.size as f64 / (1024.0 * 1024.0))
        }
    }
}

/// Persistent state for the AI chat panel.
pub struct AiChatState {
    pub input_buffer: String,
    pub session: AiSession,
    pub visible: bool,
    pub auto_scroll: bool,
    /// Pending commands from AI (consumed by the main loop).
    pub pending_commands: Vec<AiCommand>,
    /// Show the settings sub-panel inline.
    pub show_settings: bool,
    /// Cached list of discovered models (from Ollama or API).
    pub discovered_models: Vec<String>,
    /// Status message for connection test.
    pub connection_status: Option<(String, bool)>,
    /// Files attached by the user for the current message.
    pub attached_files: Vec<AttachedFile>,
}

impl Default for AiChatState {
    fn default() -> Self {
        let mut state = Self {
            input_buffer: String::new(),
            session: AiSession::new(),
            visible: false,
            auto_scroll: true,
            pending_commands: Vec::new(),
            show_settings: false,
            discovered_models: Vec::new(),
            connection_status: None,
            attached_files: Vec::new(),
        };

        // Auto-detect available Ollama models at startup
        state.auto_detect_models();

        state
    }
}

impl AiChatState {
    /// Try to discover available models from the current provider and
    /// fall back to a valid model if the configured one isn't available.
    pub fn auto_detect_models(&mut self) {
        let models = self.session.list_models();
        if models.is_empty() {
            return;
        }
        self.discovered_models = models.iter().map(|m| m.id.clone()).collect();

        // If the current model isn't in the list, pick the best available one
        if !self.discovered_models.contains(&self.session.config.model) {
            // Prefer larger capable models for instruction following
            let preferred = ["gemma4:26b", "qwen3:30b", "qwen2.5:32b", "gemma2:27b",
                            "qwen2.5:14b", "gemma4:e4b", "llama3.2:latest"];
            let picked = preferred.iter()
                .find(|p| self.discovered_models.contains(&p.to_string()))
                .map(|s| s.to_string())
                .unwrap_or_else(|| self.discovered_models[0].clone());

            log::info!(
                "Modèle '{}' non trouvé, basculé sur '{}'",
                self.session.config.model, picked
            );
            self.session.config.model = picked;
        }

        let count = self.discovered_models.len();
        self.connection_status = Some((
            format!("{} modèle(s) Ollama détecté(s)", count), true
        ));
    }
}

/// Build a SceneContext from the current AppState.
pub fn build_scene_context(state: &AppState) -> SceneContext {
    let models: Vec<anim_ai::context::ModelContext> = state.loaded_models.iter().enumerate().map(|(i, asset)| {
        let (frame_count, framerate) = if let Some(ref m) = asset.motion {
            (m.num_frames(), m.framerate)
        } else {
            (0, 30.0)
        };
        anim_ai::context::ModelContext {
            index: i,
            name: asset.name.clone(),
            bone_count: asset.model.joint_names.len(),
            bone_names: asset.model.joint_names.clone(),
            has_mesh: asset.skinned_mesh.is_some(),
            has_animation: asset.motion.is_some(),
            frame_count,
            framerate,
        }
    }).collect();

    let (current_frame, total_frames, total_time) = if let Some(motion) = state.active_motion() {
        let frame = (state.timestamp * motion.framerate) as usize;
        (frame, motion.num_frames(), motion.total_time())
    } else {
        (0, 0, 0.0)
    };

    SceneContext {
        models,
        active_model: state.active_model,
        playback: anim_ai::context::PlaybackContext {
            playing: state.playing,
            timestamp: state.timestamp,
            speed: state.playback_speed,
            looping: state.looping,
            mirrored: state.mirrored,
            current_frame,
            total_frames,
            total_time,
        },
        camera: anim_ai::context::CameraContext {
            distance: state.camera.distance,
            yaw: state.camera.yaw,
            pitch: state.camera.pitch,
        },
        display: anim_ai::context::DisplayContext {
            skeleton: state.show_skeleton,
            mesh: state.show_mesh,
            grid: state.show_grid,
            velocities: state.show_velocities,
            contacts: state.show_contacts,
            trajectory: state.show_trajectory,
        },
        tool: format!("{:?}", state.active_tool).to_lowercase(),
        selected_entity: if state.multi_selection.is_empty() {
            None
        } else {
            Some(state.multi_selection[0])
        },
        frame_count: total_frames,
    }
}

/// Show the AI chat panel.
pub fn show(ui: &mut Ui, ai_state: &mut AiChatState, app_state: &mut AppState) {
    // ── Header ─────────────────────────────────────────────
    ui.horizontal(|ui| {
        // Status indicator
        let (status_icon, status_color) = match &ai_state.session.status {
            AiStatus::Ready => ("●", Color32::from_rgb(80, 200, 120)),
            AiStatus::Thinking => ("◌", Color32::from_rgb(255, 200, 60)),
            AiStatus::Error(_) => ("●", Color32::from_rgb(255, 80, 80)),
            AiStatus::Disconnected => ("○", Color32::from_rgb(120, 120, 120)),
        };
        ui.label(RichText::new(status_icon).size(10.0).color(status_color));
        ui.label(RichText::new("IA Assistant").strong().size(12.0).color(accent::TEXT));

        // Provider + model info
        ui.label(
            RichText::new(format!("{} / {}", ai_state.session.config.provider, ai_state.session.config.model))
                .size(10.0)
                .color(accent::MUTED)
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Settings toggle
            if ui.add(
                egui::Button::new(RichText::new("⚙").size(12.0).color(accent::MUTED))
                    .fill(Color32::TRANSPARENT)
                    .rounding(3.0)
            ).clicked() {
                ai_state.show_settings = !ai_state.show_settings;
            }

            // Clear history
            if ui.add(
                egui::Button::new(RichText::new("Effacer").size(10.0).color(accent::MUTED))
                    .fill(Color32::TRANSPARENT)
                    .rounding(3.0)
            ).clicked() {
                ai_state.session.clear_history();
            }

            // Message count
            ui.label(
                RichText::new(format!("{} msgs", ai_state.session.history.len()))
                    .size(10.0)
                    .color(accent::DIM)
            );
        });
    });

    // Separator
    let (_, rect) = ui.allocate_space(egui::vec2(ui.available_width(), 1.0));
    ui.painter().line_segment(
        [rect.left_center(), rect.right_center()],
        Stroke::new(0.5, accent::BORDER),
    );

    // ── Settings (inline, collapsible) ─────────────────────
    if ai_state.show_settings {
        show_settings_inline(ui, ai_state);
        let (_, rect) = ui.allocate_space(egui::vec2(ui.available_width(), 1.0));
        ui.painter().line_segment(
            [rect.left_center(), rect.right_center()],
            Stroke::new(0.5, accent::BORDER),
        );
    }

    // ── Chat History ───────────────────────────────────────
    let history_height = ui.available_height() - 32.0; // Reserve space for input
    ScrollArea::vertical()
        .auto_shrink([false, false])
        .max_height(history_height.max(60.0))
        .stick_to_bottom(ai_state.auto_scroll)
        .show(ui, |ui| {
            if ai_state.session.history.is_empty() {
                ui.add_space(20.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new("Posez une question ou donnez un ordre...")
                            .size(11.0)
                            .color(accent::DIM)
                            .italics()
                    );
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Exemples:")
                            .size(10.0)
                            .color(accent::MUTED)
                    );
                    let examples = [
                        "\"Importe le fichier Model.glb\"",
                        "\"Lance l'animation à vitesse 2x\"",
                        "\"Montre-moi le squelette sans le mesh\"",
                        "\"Centre la caméra sur le modèle\"",
                        "\"Sélectionne l'os Hand_L et applique l'IK\"",
                        "\"Quels os sont dans ce modèle?\"",
                    ];
                    for ex in examples {
                        ui.label(
                            RichText::new(format!("  • {}", ex))
                                .size(10.0)
                                .color(accent::DIM)
                        );
                    }
                });
            }

            let now = std::time::Instant::now();
            for entry in &ai_state.session.history {
                // Format elapsed time
                let elapsed = now.duration_since(entry.timestamp).as_secs();
                let time_str = if elapsed < 60 {
                    format!("{}s", elapsed)
                } else if elapsed < 3600 {
                    format!("{}m", elapsed / 60)
                } else {
                    format!("{}h", elapsed / 3600)
                };

                match entry.role {
                    AiRole::User => {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("Vous:").size(10.5).color(accent::PRIMARY).strong());
                            ui.label(RichText::new(&entry.content).size(10.5).color(accent::TEXT));
                            ui.label(RichText::new(&time_str).size(9.0).color(accent::DIM));
                        });
                    }
                    AiRole::Assistant => {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("IA:").size(10.5).color(Color32::from_rgb(120, 220, 160)).strong());
                            ui.label(RichText::new(&entry.content).size(10.5).color(Color32::from_rgb(200, 205, 220)));
                            ui.label(RichText::new(&time_str).size(9.0).color(accent::DIM));
                        });
                        if !entry.commands.is_empty() {
                            ui.horizontal(|ui| {
                                ui.add_space(24.0);
                                ui.label(
                                    RichText::new(format!("⚡ {} commande(s)", entry.commands.len()))
                                        .size(9.5)
                                        .color(accent::WARNING)
                                );
                            });
                        }
                    }
                    AiRole::System => {} // Don't display system messages
                }
                ui.add_space(2.0);
            }

            // Thinking indicator
            if ai_state.session.status == AiStatus::Thinking {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("IA:").size(10.5).color(Color32::from_rgb(120, 220, 160)).strong());
                    ui.spinner();
                    ui.label(RichText::new("Réflexion en cours...").size(10.5).color(accent::MUTED).italics());
                });
            }
        });

    // ── Attached Files (shown above input when present) ──
    if !ai_state.attached_files.is_empty() {
        ui.add_space(2.0);
        let mut remove_idx: Option<usize> = None;
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("📎").size(10.0).color(accent::MUTED));
            for (i, file) in ai_state.attached_files.iter().enumerate() {
                let tag_color = file.category.color();
                let tag_bg = Color32::from_rgba_premultiplied(
                    tag_color.r() / 4, tag_color.g() / 4, tag_color.b() / 4, 200,
                );
                egui::Frame::none()
                    .fill(tag_bg)
                    .stroke(Stroke::new(0.5, tag_color))
                    .rounding(4.0)
                    .inner_margin(egui::Margin::symmetric(5.0, 2.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 3.0;
                            ui.label(
                                RichText::new(file.category.label())
                                    .size(8.5)
                                    .color(tag_color)
                            );
                            ui.label(
                                RichText::new(&file.name)
                                    .size(9.5)
                                    .color(accent::TEXT)
                            );
                            ui.label(
                                RichText::new(format!("({})", file.size_display()))
                                    .size(8.5)
                                    .color(accent::DIM)
                            );
                            if ui.add(
                                egui::Button::new(RichText::new("✕").size(8.0).color(accent::MUTED))
                                    .fill(Color32::TRANSPARENT)
                                    .frame(false)
                            ).clicked() {
                                remove_idx = Some(i);
                            }
                        });
                    });
            }
        });
        if let Some(idx) = remove_idx {
            ai_state.attached_files.remove(idx);
        }
    }

    // ── Input Row ──────────────────────────────────────────
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        let is_thinking = ai_state.session.status == AiStatus::Thinking;

        // Paperclip button — open file dialog to attach files
        if ui.add_enabled(
            !is_thinking,
            egui::Button::new(
                RichText::new("📎").size(13.0).color(
                    if ai_state.attached_files.is_empty() { accent::MUTED } else { accent::PRIMARY }
                )
            ).fill(Color32::TRANSPARENT).rounding(3.0)
        ).on_hover_text("Joindre un fichier")
         .clicked()
        {
            if let Some(paths) = rfd::FileDialog::new()
                .set_title("Joindre des fichiers")
                .add_filter("Tous les fichiers supportés", &[
                    "glb", "gltf", "fbx", "bvh", "npz",
                    "onnx", "pt", "pth",
                    "json", "toml", "yaml", "yml",
                    "png", "jpg", "jpeg", "hdr",
                ])
                .add_filter("Modèles 3D", &["glb", "gltf", "fbx"])
                .add_filter("Animations", &["bvh", "npz"])
                .add_filter("Modèles IA", &["onnx", "pt", "pth"])
                .add_filter("Configuration", &["json", "toml", "yaml", "yml"])
                .add_filter("Tous", &["*"])
                .pick_files()
            {
                for path in paths {
                    // Avoid duplicates
                    if !ai_state.attached_files.iter().any(|f| f.path == path) {
                        ai_state.attached_files.push(AttachedFile::from_path(path));
                    }
                }
            }
        }

        let has_content = !ai_state.input_buffer.trim().is_empty() || !ai_state.attached_files.is_empty();

        let response = ui.add_sized(
            [ui.available_width() - 30.0, 22.0],
            egui::TextEdit::singleline(&mut ai_state.input_buffer)
                .hint_text(if ai_state.attached_files.is_empty() {
                    "Promptez l'IA..."
                } else {
                    "Message (ou Entrée pour envoyer les fichiers)..."
                })
                .font(egui::TextStyle::Monospace)
                .interactive(!is_thinking)
                .desired_width(f32::INFINITY)
        );

        let enter_pressed = response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        let send_clicked = ui.add_enabled(
            !is_thinking && has_content,
            egui::Button::new(
                RichText::new("➤").size(14.0).color(if is_thinking { accent::DIM } else { accent::PRIMARY })
            ).fill(Color32::TRANSPARENT).rounding(3.0)
        ).clicked();

        if (enter_pressed || send_clicked) && has_content && !is_thinking {
            let user_text = ai_state.input_buffer.trim().to_string();
            ai_state.input_buffer.clear();

            // Build the message: user text + file attachment info
            let msg = build_message_with_files(&user_text, &ai_state.attached_files);

            // Auto-generate import commands for model/animation files
            for file in &ai_state.attached_files {
                match file.category {
                    FileCategory::Model | FileCategory::Animation => {
                        ai_state.pending_commands.push(
                            AiCommand::ImportFile { path: file.path.to_string_lossy().to_string() }
                        );
                    }
                    _ => {}
                }
            }

            // Clear attachments after sending
            ai_state.attached_files.clear();

            // Build scene context and send
            let context = build_scene_context(app_state);
            ai_state.session.send(&msg, Some(&context));

            // Re-focus input
            response.request_focus();
        }

        // Always re-focus after sending
        if !is_thinking && ai_state.input_buffer.is_empty() {
            response.request_focus();
        }
    });
}

/// Build a message string that includes user text and attached file metadata.
fn build_message_with_files(user_text: &str, files: &[AttachedFile]) -> String {
    if files.is_empty() {
        return user_text.to_string();
    }

    let mut parts = Vec::new();

    if !user_text.is_empty() {
        parts.push(user_text.to_string());
    }

    let file_descriptions: Vec<String> = files.iter().map(|f| {
        format!(
            "- {} ({}, {}, chemin: {})",
            f.name, f.category.label(), f.size_display(),
            f.path.to_string_lossy()
        )
    }).collect();

    parts.push(format!(
        "[Fichiers joints]\n{}",
        file_descriptions.join("\n")
    ));

    parts.join("\n\n")
}

/// Inline settings for AI provider configuration.
fn show_settings_inline(ui: &mut Ui, ai_state: &mut AiChatState) {
    ui.add_space(4.0);

    let session = &mut ai_state.session;

    egui::Grid::new("ai_settings_grid")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            // Provider selection
            ui.label(RichText::new("Fournisseur").size(10.0).color(accent::MUTED));
            let providers = ["ollama", "openai", "claude"];
            let mut current = providers.iter().position(|p| *p == session.config.provider).unwrap_or(0);
            if egui::ComboBox::from_id_salt("ai_provider")
                .width(140.0)
                .show_index(ui, &mut current, providers.len(), |i| {
                    let label: &str = match providers[i] {
                        "ollama" => "Ollama (Local)",
                        "openai" => "OpenAI",
                        "claude" => "Claude (Anthropic)",
                        _ => providers[i],
                    };
                    egui::WidgetText::from(label)
                }).changed()
            {
                session.config.provider = providers[current].into();
                ai_state.discovered_models.clear();
                ai_state.connection_status = None;
                match session.config.provider.as_str() {
                    "ollama" => {
                        session.config.endpoint = "http://localhost:11434".into();
                        session.config.model = "gemma4:26b".into();
                    }
                    "openai" => {
                        session.config.endpoint = "https://api.openai.com/v1".into();
                        session.config.model = "gpt-4o".into();
                    }
                    "claude" => {
                        session.config.endpoint = "https://api.anthropic.com/v1".into();
                        session.config.model = "claude-sonnet-4-20250514".into();
                    }
                    _ => {}
                }
            }
            ui.end_row();

            // Endpoint
            ui.label(RichText::new("Endpoint").size(10.0).color(accent::MUTED));
            ui.add_sized([200.0, 18.0], egui::TextEdit::singleline(&mut session.config.endpoint)
                .font(egui::TextStyle::Monospace));
            ui.end_row();

            // API Key (password field for cloud providers)
            if session.config.provider != "ollama" {
                ui.label(RichText::new("Clé API").size(10.0).color(accent::MUTED));
                ui.add_sized([200.0, 18.0], egui::TextEdit::singleline(&mut session.config.api_key)
                    .password(true)
                    .font(egui::TextStyle::Monospace));
                ui.end_row();
            }

            // Model — ComboBox if models are discovered, text field as fallback
            ui.label(RichText::new("Modèle").size(10.0).color(accent::MUTED));
            if !ai_state.discovered_models.is_empty() {
                let models = &ai_state.discovered_models;
                let mut selected_idx = models.iter()
                    .position(|m| *m == session.config.model)
                    .unwrap_or(0);
                if egui::ComboBox::from_id_salt("ai_model_select")
                    .width(200.0)
                    .show_index(ui, &mut selected_idx, models.len(), |i| {
                        egui::WidgetText::from(models[i].as_str())
                    }).changed()
                {
                    session.config.model = models[selected_idx].clone();
                }
            } else {
                ui.add_sized([200.0, 18.0], egui::TextEdit::singleline(&mut session.config.model)
                    .font(egui::TextStyle::Monospace));
            }
            ui.end_row();

            // Temperature
            ui.label(RichText::new("Température").size(10.0).color(accent::MUTED));
            ui.add(egui::Slider::new(&mut session.config.temperature, 0.0..=1.0)
                .step_by(0.05)
                .fixed_decimals(2));
            ui.end_row();

            // Max tokens
            ui.label(RichText::new("Max tokens").size(10.0).color(accent::MUTED));
            ui.add(egui::DragValue::new(&mut session.config.max_tokens)
                .range(256..=32768)
                .speed(64));
            ui.end_row();
        });

    // Buttons row
    ui.horizontal(|ui| {
        // Connection test
        if ui.add(
            egui::Button::new(RichText::new("🔌 Tester").size(10.0))
                .rounding(3.0)
        ).clicked() {
            let ok = session.test_connection();
            ai_state.connection_status = Some(if ok {
                ("✓ Connecté".into(), true)
            } else {
                ("✗ Échec connexion".into(), false)
            });
        }

        // Discover models button
        if ui.add(
            egui::Button::new(RichText::new("🔍 Modèles disponibles").size(10.0))
                .rounding(3.0)
        ).clicked() {
            let models = session.list_models();
            ai_state.discovered_models = models.iter().map(|m| m.id.clone()).collect();
            if ai_state.discovered_models.is_empty() {
                ai_state.connection_status = Some(("Aucun modèle trouvé".into(), false));
            } else {
                let count = ai_state.discovered_models.len();
                ai_state.connection_status = Some(
                    (format!("{} modèle(s) trouvé(s)", count), true)
                );
                // Auto-select first model if current model not in list
                if !ai_state.discovered_models.contains(&session.config.model) {
                    session.config.model = ai_state.discovered_models[0].clone();
                }
            }
        }

        // Show connection status
        if let Some((ref msg, ok)) = ai_state.connection_status {
            let color = if ok { Color32::from_rgb(80, 200, 120) } else { Color32::from_rgb(255, 80, 80) };
            ui.label(RichText::new(msg).size(9.5).color(color));
        }
    });

    ui.add_space(4.0);
}
