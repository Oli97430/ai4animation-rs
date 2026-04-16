//! Scene context — provides the AI with a structured snapshot of the editor state.

use serde::Serialize;

/// Compact snapshot of the editor state for the AI system prompt.
#[derive(Debug, Clone, Serialize)]
pub struct SceneContext {
    pub models: Vec<ModelContext>,
    pub active_model: Option<usize>,
    pub playback: PlaybackContext,
    pub camera: CameraContext,
    pub display: DisplayContext,
    pub tool: String,
    pub selected_entity: Option<usize>,
    pub frame_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelContext {
    pub index: usize,
    pub name: String,
    pub bone_count: usize,
    pub bone_names: Vec<String>,
    pub has_mesh: bool,
    pub has_animation: bool,
    pub frame_count: usize,
    pub framerate: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlaybackContext {
    pub playing: bool,
    pub timestamp: f32,
    pub speed: f32,
    pub looping: bool,
    pub mirrored: bool,
    pub current_frame: usize,
    pub total_frames: usize,
    pub total_time: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct CameraContext {
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct DisplayContext {
    pub skeleton: bool,
    pub mesh: bool,
    pub grid: bool,
    pub velocities: bool,
    pub contacts: bool,
    pub trajectory: bool,
}

impl SceneContext {
    /// Serialize to a compact JSON string for injection into the AI prompt.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Serialize to a pretty-printed JSON string (for debugging).
    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Build a human-readable summary for the system prompt.
    pub fn to_summary(&self) -> String {
        let mut s = String::new();

        if self.models.is_empty() {
            s.push_str("Scene: vide (aucun modèle chargé)\n");
            return s;
        }

        s.push_str(&format!("Scene: {} modèle(s)\n", self.models.len()));
        for m in &self.models {
            let active = if Some(m.index) == self.active_model { " [ACTIF]" } else { "" };
            s.push_str(&format!(
                "  #{} \"{}\" — {} os, mesh={}, anim={} ({} frames, {}fps){}\n",
                m.index, m.name, m.bone_count,
                if m.has_mesh { "oui" } else { "non" },
                if m.has_animation { "oui" } else { "non" },
                m.frame_count, m.framerate, active,
            ));
        }

        let p = &self.playback;
        s.push_str(&format!(
            "Lecture: {} | frame {}/{} | t={:.2}s/{:.2}s | vitesse={:.1}x | boucle={} | miroir={}\n",
            if p.playing { "EN COURS" } else { "PAUSE" },
            p.current_frame, p.total_frames,
            p.timestamp, p.total_time,
            p.speed,
            if p.looping { "oui" } else { "non" },
            if p.mirrored { "oui" } else { "non" },
        ));

        s.push_str(&format!("Outil: {} | ", self.tool));
        if let Some(e) = self.selected_entity {
            s.push_str(&format!("Selection: entité #{}", e));
        } else {
            s.push_str("Selection: aucune");
        }
        s.push('\n');

        let d = &self.display;
        s.push_str(&format!(
            "Affichage: squelette={} mesh={} grille={} vélocités={} contacts={} trajectoire={}\n",
            d.skeleton, d.mesh, d.grid, d.velocities, d.contacts, d.trajectory,
        ));

        s
    }
}
