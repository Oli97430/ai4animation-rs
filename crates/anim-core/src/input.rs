//! Unified input system — keyboard, mouse, and gamepad abstraction.
//!
//! Platform-independent input state polled each frame from winit/egui.
//! Mirrors Python InputSystem.py but uses egui's input API instead of Raylib.

use glam::Vec3;

/// Deadzone threshold for analog sticks.
pub const STICK_DEADZONE: f32 = 0.1;
/// Deadzone threshold for triggers.
pub const TRIGGER_DEADZONE: f32 = 0.1;

/// Snapshot of all input state for one frame.
#[derive(Debug, Clone)]
pub struct InputState {
    // Keyboard
    pub keys_down: Vec<Key>,
    pub keys_pressed: Vec<Key>,

    // Mouse
    pub mouse_pos: [f32; 2],
    pub mouse_delta: [f32; 2],
    pub mouse_left: bool,
    pub mouse_right: bool,
    pub mouse_middle: bool,
    pub scroll_delta: f32,

    // Gamepad (optional)
    pub gamepad: Option<GamepadState>,

    // Derived movement (WASD/QE → Vec3)
    pub movement: Vec3,
    // Secondary mapping (IJKL → 2D)
    pub secondary: [f32; 2],
}

/// Gamepad state for one controller.
#[derive(Debug, Clone, Default)]
pub struct GamepadState {
    pub connected: bool,
    pub left_stick: [f32; 2],
    pub right_stick: [f32; 2],
    pub left_trigger: f32,
    pub right_trigger: f32,
    pub buttons: GamepadButtons,
}

/// Gamepad button state.
#[derive(Debug, Clone, Default)]
pub struct GamepadButtons {
    pub a: bool,
    pub b: bool,
    pub x: bool,
    pub y: bool,
    pub left_bumper: bool,
    pub right_bumper: bool,
    pub left_stick_press: bool,
    pub right_stick_press: bool,
    pub start: bool,
    pub back: bool,
    pub dpad_up: bool,
    pub dpad_down: bool,
    pub dpad_left: bool,
    pub dpad_right: bool,
}

/// Common key identifiers (subset matching what the editor uses).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    W, A, S, D, Q, E,
    I, J, K, L,
    Space, Escape, Enter, Backspace, Tab,
    Shift, Ctrl, Alt,
    Up, Down, Left, Right,
    Home, End,
    F1, F2, F3, F4, F5,
    Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9,
    X, Y, Z, R, F, G, M, O, N, T, P, C, V, B, H, U,
    Delete,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            keys_down: Vec::new(),
            keys_pressed: Vec::new(),
            mouse_pos: [0.0; 2],
            mouse_delta: [0.0; 2],
            mouse_left: false,
            mouse_right: false,
            mouse_middle: false,
            scroll_delta: 0.0,
            gamepad: None,
            movement: Vec3::ZERO,
            secondary: [0.0; 2],
        }
    }
}

impl InputState {
    /// Check if a key is currently held down.
    pub fn is_key_down(&self, key: Key) -> bool {
        self.keys_down.contains(&key)
    }

    /// Check if a key was just pressed this frame.
    pub fn is_key_pressed(&self, key: Key) -> bool {
        self.keys_pressed.contains(&key)
    }

    /// Compute WASD+QE movement vector from current key state.
    pub fn compute_movement(&mut self) {
        let mut m = [0.0f32; 3];
        if self.is_key_down(Key::W) { m[2] += 1.0; }
        if self.is_key_down(Key::S) { m[2] -= 1.0; }
        if self.is_key_down(Key::A) { m[0] -= 1.0; }
        if self.is_key_down(Key::D) { m[0] += 1.0; }
        if self.is_key_down(Key::Q) { m[1] -= 1.0; }
        if self.is_key_down(Key::E) { m[1] += 1.0; }
        self.movement = Vec3::from_array(m);
    }

    /// Compute IJKL secondary input (like a second joystick on keyboard).
    pub fn compute_secondary(&mut self) {
        let mut s = [0.0f32; 2];
        if self.is_key_down(Key::I) { s[1] += 1.0; }
        if self.is_key_down(Key::K) { s[1] -= 1.0; }
        if self.is_key_down(Key::J) { s[0] -= 1.0; }
        if self.is_key_down(Key::L) { s[0] += 1.0; }
        self.secondary = s;
    }

    /// Get left stick with deadzone applied (from gamepad if present, else keyboard fallback).
    pub fn left_stick(&self) -> [f32; 2] {
        if let Some(ref gp) = self.gamepad {
            apply_deadzone(gp.left_stick)
        } else {
            [self.movement.x, self.movement.z]
        }
    }

    /// Get right stick with deadzone applied.
    pub fn right_stick(&self) -> [f32; 2] {
        if let Some(ref gp) = self.gamepad {
            apply_deadzone(gp.right_stick)
        } else {
            self.secondary
        }
    }

    /// Get left trigger (0.0 = released, 1.0 = fully pressed).
    pub fn left_trigger(&self) -> f32 {
        self.gamepad.as_ref().map_or(0.0, |gp| {
            if gp.left_trigger < TRIGGER_DEADZONE { 0.0 } else { gp.left_trigger }
        })
    }

    /// Get right trigger.
    pub fn right_trigger(&self) -> f32 {
        self.gamepad.as_ref().map_or(0.0, |gp| {
            if gp.right_trigger < TRIGGER_DEADZONE { 0.0 } else { gp.right_trigger }
        })
    }

    /// Is a gamepad connected?
    pub fn gamepad_available(&self) -> bool {
        self.gamepad.as_ref().map_or(false, |gp| gp.connected)
    }
}

/// Apply deadzone to a 2D stick input.
fn apply_deadzone(stick: [f32; 2]) -> [f32; 2] {
    let mut result = stick;
    if result[0].abs() < STICK_DEADZONE { result[0] = 0.0; }
    if result[1].abs() < STICK_DEADZONE { result[1] = 0.0; }
    result
}
