//! Configurable keyboard shortcuts.

use egui::Key;
use std::collections::HashMap;

/// A keyboard shortcut: key + modifier flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyBinding {
    pub key: Key,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl KeyBinding {
    pub const fn new(key: Key) -> Self {
        Self { key, ctrl: false, shift: false, alt: false }
    }

    pub const fn ctrl(key: Key) -> Self {
        Self { key, ctrl: true, shift: false, alt: false }
    }

    pub const fn ctrl_shift(key: Key) -> Self {
        Self { key, ctrl: true, shift: true, alt: false }
    }

    /// Check if this binding is currently pressed.
    pub fn is_pressed(&self, input: &egui::InputState) -> bool {
        input.key_pressed(self.key)
            && input.modifiers.ctrl == self.ctrl
            && input.modifiers.shift == self.shift
            && input.modifiers.alt == self.alt
    }

    /// Display label for UI.
    pub fn label(&self) -> String {
        let mut parts = Vec::new();
        if self.ctrl { parts.push("Ctrl"); }
        if self.shift { parts.push("Shift"); }
        if self.alt { parts.push("Alt"); }
        parts.push(key_name(self.key));
        parts.join("+")
    }
}

/// All configurable actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Save,
    OpenProject,
    Undo,
    Redo,
    PlayPause,
    GoToStart,
    GoToEnd,
    PrevFrame,
    NextFrame,
    ResetCamera,
    FocusModel,
    ToggleLoop,
    ToggleMirror,
    ToggleGrid,
    ToolSelect,
    ToolMove,
    ToolRotate,
    ToolMeasure,
    ToolIk,
    ResetTool,
    AxisX,
    AxisY,
    AxisZ,
    SelectAll,
    ViewFront,
    ViewRight,
    ViewTop,
    ToggleOnionSkin,
    ToggleDopeSheet,
    CopyPose,
    PastePose,
    MirrorPose,
}

impl Action {
    /// Human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Save => "Sauvegarder",
            Self::OpenProject => "Ouvrir projet",
            Self::Undo => "Annuler",
            Self::Redo => "Refaire",
            Self::PlayPause => "Play / Pause",
            Self::GoToStart => "Debut",
            Self::GoToEnd => "Fin",
            Self::PrevFrame => "Image precedente",
            Self::NextFrame => "Image suivante",
            Self::ResetCamera => "Reset camera",
            Self::FocusModel => "Focus modele",
            Self::ToggleLoop => "Boucle",
            Self::ToggleMirror => "Miroir",
            Self::ToggleGrid => "Grille",
            Self::ToolSelect => "Outil: Select",
            Self::ToolMove => "Outil: Move",
            Self::ToolRotate => "Outil: Rotate",
            Self::ToolMeasure => "Outil: Measure",
            Self::ToolIk => "Outil: IK",
            Self::ResetTool => "Reset outil",
            Self::AxisX => "Axe X",
            Self::AxisY => "Axe Y",
            Self::AxisZ => "Axe Z",
            Self::SelectAll => "Tout selectionner",
            Self::ViewFront => "Vue face",
            Self::ViewRight => "Vue droite",
            Self::ViewTop => "Vue dessus",
            Self::ToggleOnionSkin => "Onion Skinning",
            Self::ToggleDopeSheet => "Dope Sheet",
            Self::CopyPose => "Copier pose",
            Self::PastePose => "Coller pose",
            Self::MirrorPose => "Miroir pose",
        }
    }

    /// All actions in display order.
    pub fn all() -> &'static [Action] {
        &[
            Self::Save, Self::OpenProject, Self::Undo, Self::Redo,
            Self::PlayPause, Self::GoToStart, Self::GoToEnd,
            Self::PrevFrame, Self::NextFrame,
            Self::ResetCamera, Self::FocusModel,
            Self::ToggleLoop, Self::ToggleMirror, Self::ToggleGrid,
            Self::ToolSelect, Self::ToolMove, Self::ToolRotate,
            Self::ToolMeasure, Self::ToolIk, Self::ResetTool,
            Self::AxisX, Self::AxisY, Self::AxisZ,
            Self::SelectAll, Self::ViewFront, Self::ViewRight, Self::ViewTop,
            Self::ToggleOnionSkin, Self::ToggleDopeSheet,
            Self::CopyPose, Self::PastePose, Self::MirrorPose,
        ]
    }
}

/// Configurable keyboard shortcut map.
pub struct ShortcutMap {
    bindings: HashMap<Action, KeyBinding>,
}

impl ShortcutMap {
    pub fn new() -> Self {
        Self { bindings: Self::defaults() }
    }

    /// Default key bindings.
    fn defaults() -> HashMap<Action, KeyBinding> {
        let mut m = HashMap::new();
        m.insert(Action::Save, KeyBinding::ctrl(Key::S));
        m.insert(Action::OpenProject, KeyBinding::ctrl(Key::O));
        m.insert(Action::Undo, KeyBinding::ctrl(Key::Z));
        m.insert(Action::Redo, KeyBinding::ctrl(Key::Y));
        m.insert(Action::PlayPause, KeyBinding::new(Key::Space));
        m.insert(Action::GoToStart, KeyBinding::new(Key::Home));
        m.insert(Action::GoToEnd, KeyBinding::new(Key::End));
        m.insert(Action::PrevFrame, KeyBinding::new(Key::ArrowLeft));
        m.insert(Action::NextFrame, KeyBinding::new(Key::ArrowRight));
        m.insert(Action::ResetCamera, KeyBinding::new(Key::R));
        m.insert(Action::FocusModel, KeyBinding::new(Key::F));
        m.insert(Action::ToggleLoop, KeyBinding::new(Key::L));
        m.insert(Action::ToggleMirror, KeyBinding::new(Key::M));
        m.insert(Action::ToggleGrid, KeyBinding::new(Key::G));
        m.insert(Action::ToolSelect, KeyBinding::new(Key::Num1));
        m.insert(Action::ToolMove, KeyBinding::new(Key::Num2));
        m.insert(Action::ToolRotate, KeyBinding::new(Key::Num3));
        m.insert(Action::ToolMeasure, KeyBinding::new(Key::Num4));
        m.insert(Action::ToolIk, KeyBinding::new(Key::Num5));
        m.insert(Action::ResetTool, KeyBinding::new(Key::Escape));
        m.insert(Action::AxisX, KeyBinding::new(Key::X));
        m.insert(Action::AxisY, KeyBinding::new(Key::Y));
        m.insert(Action::AxisZ, KeyBinding::new(Key::Z));
        m.insert(Action::SelectAll, KeyBinding::ctrl(Key::A));
        m.insert(Action::ViewFront, KeyBinding::ctrl(Key::Num1));
        m.insert(Action::ViewRight, KeyBinding::ctrl(Key::Num3));
        m.insert(Action::ViewTop, KeyBinding::ctrl(Key::Num7));
        m.insert(Action::ToggleOnionSkin, KeyBinding::new(Key::O));
        m.insert(Action::ToggleDopeSheet, KeyBinding::new(Key::D));
        m.insert(Action::CopyPose, KeyBinding::ctrl(Key::C));
        m.insert(Action::PastePose, KeyBinding::ctrl(Key::V));
        m.insert(Action::MirrorPose, KeyBinding::ctrl(Key::M));
        m
    }

    /// Check if a given action was triggered this frame.
    pub fn pressed(&self, action: Action, input: &egui::InputState) -> bool {
        self.bindings.get(&action).is_some_and(|b| b.is_pressed(input))
    }

    /// Get the binding for an action.
    pub fn get(&self, action: Action) -> Option<&KeyBinding> {
        self.bindings.get(&action)
    }

    /// Set a new binding for an action.
    pub fn set(&mut self, action: Action, binding: KeyBinding) {
        self.bindings.insert(action, binding);
    }

    /// Reset all bindings to defaults.
    pub fn reset_defaults(&mut self) {
        self.bindings = Self::defaults();
    }

    /// Get the label for an action's current binding.
    pub fn label(&self, action: Action) -> String {
        self.bindings.get(&action).map_or("(none)".to_string(), |b| b.label())
    }
}

impl Default for ShortcutMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Human-readable key name.
fn key_name(key: Key) -> &'static str {
    match key {
        Key::A => "A", Key::B => "B", Key::C => "C", Key::D => "D",
        Key::E => "E", Key::F => "F", Key::G => "G", Key::H => "H",
        Key::I => "I", Key::J => "J", Key::K => "K", Key::L => "L",
        Key::M => "M", Key::N => "N", Key::O => "O", Key::P => "P",
        Key::Q => "Q", Key::R => "R", Key::S => "S", Key::T => "T",
        Key::U => "U", Key::V => "V", Key::W => "W", Key::X => "X",
        Key::Y => "Y", Key::Z => "Z",
        Key::Num0 => "0", Key::Num1 => "1", Key::Num2 => "2", Key::Num3 => "3",
        Key::Num4 => "4", Key::Num5 => "5", Key::Num6 => "6", Key::Num7 => "7",
        Key::Num8 => "8", Key::Num9 => "9",
        Key::Space => "Space", Key::Enter => "Enter", Key::Escape => "Esc",
        Key::Tab => "Tab", Key::Backspace => "Backspace", Key::Delete => "Del",
        Key::Home => "Home", Key::End => "End",
        Key::ArrowUp => "Up", Key::ArrowDown => "Down",
        Key::ArrowLeft => "Left", Key::ArrowRight => "Right",
        Key::F1 => "F1", Key::F2 => "F2", Key::F3 => "F3", Key::F4 => "F4",
        Key::F5 => "F5", Key::F6 => "F6", Key::F7 => "F7", Key::F8 => "F8",
        Key::F9 => "F9", Key::F10 => "F10", Key::F11 => "F11", Key::F12 => "F12",
        _ => "?",
    }
}

/// All known keys for the binding editor dropdown.
pub fn all_keys() -> &'static [Key] {
    &[
        Key::A, Key::B, Key::C, Key::D, Key::E, Key::F, Key::G, Key::H,
        Key::I, Key::J, Key::K, Key::L, Key::M, Key::N, Key::O, Key::P,
        Key::Q, Key::R, Key::S, Key::T, Key::U, Key::V, Key::W, Key::X,
        Key::Y, Key::Z,
        Key::Num0, Key::Num1, Key::Num2, Key::Num3, Key::Num4,
        Key::Num5, Key::Num6, Key::Num7, Key::Num8, Key::Num9,
        Key::Space, Key::Enter, Key::Escape, Key::Tab,
        Key::Backspace, Key::Delete, Key::Home, Key::End,
        Key::ArrowUp, Key::ArrowDown, Key::ArrowLeft, Key::ArrowRight,
        Key::F1, Key::F2, Key::F3, Key::F4, Key::F5, Key::F6,
        Key::F7, Key::F8, Key::F9, Key::F10, Key::F11, Key::F12,
    ]
}
