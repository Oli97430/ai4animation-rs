//! Recent files tracking — persisted to disk as JSON.

use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf};

const MAX_RECENT: usize = 10;
const RECENT_FILE_NAME: &str = "recent_files.json";

/// A single recent file entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentEntry {
    pub path: String,
    /// Display name (file name without extension).
    pub name: String,
    /// File type: "a4a", "glb", "bvh", "npz", "fbx".
    pub file_type: String,
    /// Unix timestamp of when it was last opened.
    pub timestamp: u64,
}

/// Manages the list of recently opened files.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecentFiles {
    pub entries: Vec<RecentEntry>,
}

impl RecentFiles {
    /// Load from the config directory, or create empty if not found.
    pub fn load() -> Self {
        let path = config_path();
        if path.exists() {
            if let Ok(json) = std::fs::read_to_string(&path) {
                if let Ok(recent) = serde_json::from_str(&json) {
                    return recent;
                }
            }
        }
        Self::default()
    }

    /// Save to the config directory.
    pub fn save(&self) {
        let path = config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, json);
        }
    }

    /// Add a file to the recent list (moves to top if already present).
    pub fn add(&mut self, file_path: &Path) {
        let path_str = file_path.to_string_lossy().to_string();

        // Remove existing entry for this path
        self.entries.retain(|e| e.path != path_str);

        // Extract file info
        let name = file_path.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "?".into());
        let file_type = file_path.extension()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Insert at the front
        self.entries.insert(0, RecentEntry {
            path: path_str,
            name,
            file_type,
            timestamp,
        });

        // Trim to max
        self.entries.truncate(MAX_RECENT);

        // Auto-save
        self.save();
    }

    /// Remove entries whose files no longer exist on disk.
    pub fn prune(&mut self) {
        let before = self.entries.len();
        self.entries.retain(|e| Path::new(&e.path).exists());
        if self.entries.len() != before {
            self.save();
        }
    }

    /// Get icon for file type.
    pub fn icon_for(file_type: &str) -> &'static str {
        match file_type {
            "a4a" => "📋",
            "glb" | "gltf" => "🗿",
            "bvh" => "🦴",
            "npz" => "📊",
            "fbx" => "📦",
            _ => "📄",
        }
    }

    /// Check if the list is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Get the path to the recent files JSON.
fn config_path() -> PathBuf {
    // Store in the executable's directory for portability
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            return dir.join(RECENT_FILE_NAME);
        }
    }
    // Fallback: current directory
    PathBuf::from(RECENT_FILE_NAME)
}

/// Auto-save state: tracks when the last auto-save happened.
pub struct AutoSave {
    /// Interval between auto-saves in seconds.
    pub interval: f32,
    /// Time elapsed since last save.
    pub elapsed: f32,
    /// Whether auto-save is enabled.
    pub enabled: bool,
    /// Path to the auto-save file.
    pub backup_path: Option<PathBuf>,
}

impl Default for AutoSave {
    fn default() -> Self {
        Self {
            interval: 300.0, // 5 minutes
            elapsed: 0.0,
            enabled: true,
            backup_path: None,
        }
    }
}

impl AutoSave {
    /// Tick the auto-save timer. Returns true if it's time to save.
    pub fn tick(&mut self, dt: f32) -> bool {
        if !self.enabled {
            return false;
        }
        self.elapsed += dt;
        if self.elapsed >= self.interval {
            self.elapsed = 0.0;
            true
        } else {
            false
        }
    }

    /// Get or create the backup file path.
    pub fn get_backup_path(&mut self) -> PathBuf {
        if let Some(ref p) = self.backup_path {
            return p.clone();
        }
        let path = if let Ok(exe) = std::env::current_exe() {
            exe.parent()
                .unwrap_or(Path::new("."))
                .join("autosave.a4a")
        } else {
            PathBuf::from("autosave.a4a")
        };
        self.backup_path = Some(path.clone());
        path
    }
}
