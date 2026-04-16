//! Asset browser panel — directory navigation for loading animation files.

use egui::{Ui, RichText, ScrollArea, Color32};
use std::path::{Path, PathBuf};
use crate::app_state::AppState;
use crate::theme::accent;

/// Supported file extensions for the asset browser.
const SUPPORTED_EXTENSIONS: &[&str] = &["glb", "gltf", "bvh", "npz", "fbx"];

/// State for the asset browser panel.
pub struct AssetBrowserState {
    /// Current directory being browsed.
    pub current_dir: PathBuf,
    /// Cached directory listing.
    pub entries: Vec<DirEntry>,
    /// Whether the listing needs to be refreshed.
    pub dirty: bool,
    /// Search/filter text.
    pub filter: String,
    /// Show only supported animation files.
    pub filter_supported: bool,
    /// Selected entry index.
    pub selected: Option<usize>,
    /// Panel visibility.
    pub visible: bool,
}

#[derive(Clone)]
pub struct DirEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub extension: String,
    pub size: u64,
}

impl AssetBrowserState {
    pub fn new() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            current_dir,
            entries: Vec::new(),
            dirty: true,
            filter: String::new(),
            filter_supported: true,
            selected: None,
            visible: false,
        }
    }

    /// Refresh the directory listing.
    pub fn refresh(&mut self) {
        self.entries.clear();
        self.selected = None;

        if let Ok(read_dir) = std::fs::read_dir(&self.current_dir) {
            let mut dirs = Vec::new();
            let mut files = Vec::new();

            for entry in read_dir.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = path.is_dir();
                let extension = path.extension()
                    .map(|e| e.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);

                let entry = DirEntry { path, name, is_dir, extension, size };
                if is_dir {
                    dirs.push(entry);
                } else {
                    files.push(entry);
                }
            }

            // Sort: directories first (alpha), then files (alpha)
            dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

            self.entries.extend(dirs);
            self.entries.extend(files);
        }

        self.dirty = false;
    }

    /// Navigate to a directory.
    pub fn navigate_to(&mut self, path: &Path) {
        if path.is_dir() {
            self.current_dir = path.to_path_buf();
            self.dirty = true;
        }
    }

    /// Go up to parent directory.
    pub fn navigate_up(&mut self) {
        if let Some(parent) = self.current_dir.parent() {
            self.current_dir = parent.to_path_buf();
            self.dirty = true;
        }
    }
}

impl Default for AssetBrowserState {
    fn default() -> Self {
        Self::new()
    }
}

enum BrowserAction {
    Select(usize),
    NavigateTo(PathBuf),
    LoadFile(PathBuf, String),
}

/// Show the asset browser panel.
/// `asset_manager` is optional — if provided, files are loaded through the cache.
pub fn show(ui: &mut Ui, browser: &mut AssetBrowserState, state: &mut AppState) {
    show_with_manager(ui, browser, state, None);
}

/// Show the asset browser panel with optional AssetManager for cached loading.
pub fn show_with_manager(
    ui: &mut Ui,
    browser: &mut AssetBrowserState,
    state: &mut AppState,
    asset_manager: Option<&mut anim_import::AssetManager>,
) {
    if !browser.visible {
        return;
    }

    if browser.dirty {
        browser.refresh();
    }

    ui.horizontal(|ui| {
        ui.label(RichText::new("📁 Asset Browser").size(12.0).strong().color(accent::PRIMARY));
        ui.separator();

        // Navigation buttons
        if ui.small_button("⬆").on_hover_text("Dossier parent").clicked() {
            browser.navigate_up();
        }
        if ui.small_button("🔄").on_hover_text("Rafraichir").clicked() {
            browser.dirty = true;
        }

        // Current path (truncated)
        let path_str = browser.current_dir.to_string_lossy();
        let display_path = if path_str.len() > 60 {
            format!("...{}", &path_str[path_str.len() - 57..])
        } else {
            path_str.to_string()
        };
        ui.label(RichText::new(&display_path).size(10.0).color(accent::MUTED));
    });

    // Filter bar
    ui.horizontal(|ui| {
        ui.label(RichText::new("🔍").size(10.0));
        let response = ui.add(
            egui::TextEdit::singleline(&mut browser.filter)
                .desired_width(120.0)
                .hint_text("Filtrer...")
                .font(egui::TextStyle::Small)
        );
        if !browser.filter.is_empty() {
            if ui.small_button("✕").clicked() {
                browser.filter.clear();
            }
        }
        ui.checkbox(&mut browser.filter_supported, RichText::new("Anim seul.").size(10.0));
        let _ = response;
    });

    // File listing — collect into owned data to avoid borrow conflict
    let filter_lower = browser.filter.to_lowercase();
    let filter_supported = browser.filter_supported;
    let filtered_indices: Vec<usize> = browser.entries.iter()
        .enumerate()
        .filter(|(_, e)| {
            if !filter_lower.is_empty() && !e.name.to_lowercase().contains(&filter_lower) {
                return false;
            }
            if filter_supported && !e.is_dir {
                return SUPPORTED_EXTENSIONS.contains(&e.extension.as_str());
            }
            true
        })
        .map(|(i, _)| i)
        .collect();

    let file_count = filtered_indices.iter().filter(|&&i| !browser.entries[i].is_dir).count();
    let dir_count = filtered_indices.iter().filter(|&&i| browser.entries[i].is_dir).count();

    // Action to perform after the scroll area (deferred to avoid borrow conflict)
    let mut action: Option<BrowserAction> = None;

    let row_height = 18.0;
    ScrollArea::vertical()
        .max_height(150.0)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            for &idx in &filtered_indices {
                let entry = &browser.entries[idx];
                let is_selected = browser.selected == Some(idx);
                let icon = if entry.is_dir {
                    "📂"
                } else {
                    match entry.extension.as_str() {
                        "glb" | "gltf" => "🧊",
                        "bvh" => "🦴",
                        "npz" => "📊",
                        "fbx" => "🎬",
                        _ => "📄",
                    }
                };

                let text_color = if is_selected {
                    accent::PRIMARY
                } else if entry.is_dir {
                    accent::TEXT
                } else if SUPPORTED_EXTENSIONS.contains(&entry.extension.as_str()) {
                    Color32::from_rgb(200, 220, 255)
                } else {
                    accent::MUTED
                };

                let label = format!("{} {}", icon, entry.name);
                let response = ui.add_sized(
                    [ui.available_width(), row_height],
                    egui::SelectableLabel::new(
                        is_selected,
                        RichText::new(&label).size(11.0).color(text_color),
                    ),
                );

                if !entry.is_dir {
                    response.clone().on_hover_text(format_size(entry.size));
                }

                if response.clicked() {
                    action = Some(BrowserAction::Select(idx));
                }

                if response.double_clicked() {
                    if entry.is_dir {
                        action = Some(BrowserAction::NavigateTo(entry.path.clone()));
                    } else {
                        action = Some(BrowserAction::LoadFile(entry.path.clone(), entry.extension.clone()));
                    }
                }
            }

            if filtered_indices.is_empty() {
                ui.label(RichText::new("(vide)").size(10.5).color(accent::DIM).italics());
            }
        });

    // Bottom bar: file count + load button
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("{} dossiers, {} fichiers", dir_count, file_count))
            .size(10.0).color(accent::DIM));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let can_load = browser.selected.map_or(false, |idx| {
                browser.entries.get(idx).map_or(false, |e| {
                    !e.is_dir && SUPPORTED_EXTENSIONS.contains(&e.extension.as_str())
                })
            });
            if ui.add_enabled(can_load, egui::Button::new(
                RichText::new("📥 Charger").size(11.0)
            )).clicked() {
                if let Some(idx) = browser.selected {
                    if let Some(entry) = browser.entries.get(idx) {
                        action = Some(BrowserAction::LoadFile(entry.path.clone(), entry.extension.clone()));
                    }
                }
            }
        });
    });

    // Execute deferred action
    match action {
        Some(BrowserAction::Select(idx)) => {
            browser.selected = Some(idx);
        }
        Some(BrowserAction::NavigateTo(path)) => {
            browser.navigate_to(&path);
        }
        Some(BrowserAction::LoadFile(path, ext)) => {
            load_file(&path, &ext, state, asset_manager);
        }
        None => {}
    }
}

fn load_file(
    path: &Path,
    extension: &str,
    state: &mut AppState,
    asset_manager: Option<&mut anim_import::AssetManager>,
) {
    // Try cached load via AssetManager first, fall back to direct importer
    let result = if let Some(mgr) = asset_manager {
        mgr.load(path)
    } else {
        match extension {
            "glb" | "gltf" => anim_import::GlbImporter::load(path),
            "bvh" => anim_import::BvhImporter::load(path, 0.01),
            "npz" => anim_import::NpzImporter::load(path),
            "fbx" => anim_import::FbxImporter::load(path),
            _ => {
                state.log_warn(&format!("Format non supporte: {}", extension));
                return;
            }
        }
    };

    match result {
        Ok(model) => state.import_model(model),
        Err(e) => state.log_error(&format!("Erreur: {}", e)),
    }
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
