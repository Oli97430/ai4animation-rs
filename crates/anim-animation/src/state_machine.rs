//! Animation state machine: named states with conditional transitions.
//!
//! Each state references a motion source (clip index or procedural type).
//! Transitions fire when their condition is met, using crossfade blending.

use std::collections::HashMap;
use crate::blend::AnimationTransition;

pub type StateId = usize;

/// A complete animation state machine.
pub struct StateMachine {
    pub name: String,
    pub states: Vec<State>,
    pub transitions: Vec<Transition>,
    pub parameters: Parameters,
    /// Currently active state.
    pub active_state: StateId,
    /// Transition currently in progress (blending from one state to another).
    pub active_transition: Option<ActiveTransition>,
    /// Time elapsed in the current state (seconds).
    pub state_elapsed: f32,
}

/// A single state in the machine.
#[derive(Clone)]
pub struct State {
    pub name: String,
    pub motion_source: MotionSource,
    /// Visual position in the node graph editor (x, y).
    pub position: [f32; 2],
}

/// What animation a state plays.
#[derive(Clone, Debug)]
pub enum MotionSource {
    /// Index into AppState.loaded_models.
    Clip { model_index: usize },
    /// Procedural animation type ("idle", "walk", "run", "jump").
    Procedural { anim_type: String },
    /// No animation (empty / entry state).
    None,
}

/// A transition between two states.
#[derive(Clone)]
pub struct Transition {
    pub from: StateId,
    pub to: StateId,
    pub condition: TransitionCondition,
    pub crossfade_duration: f32,
    /// Higher priority transitions are checked first.
    pub priority: u8,
}

/// Condition that must be met for a transition to fire.
#[derive(Clone, Debug)]
pub enum TransitionCondition {
    /// Bool parameter equals expected value.
    BoolParam { name: String, value: bool },
    /// Float parameter satisfies comparison.
    FloatThreshold { name: String, op: CompareOp, value: f32 },
    /// State has been active for at least N seconds.
    TimeElapsed { seconds: f32 },
    /// Current animation has reached its end (non-looping).
    AnimationEnd,
    /// Always true (immediate transition, useful for entry states).
    Always,
}

/// Comparison operators for float conditions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CompareOp {
    Greater,
    Less,
    GreaterEq,
    LessEq,
    Equal,
}

impl CompareOp {
    pub fn evaluate(&self, a: f32, b: f32) -> bool {
        match self {
            CompareOp::Greater => a > b,
            CompareOp::Less => a < b,
            CompareOp::GreaterEq => a >= b,
            CompareOp::LessEq => a <= b,
            CompareOp::Equal => (a - b).abs() < 1e-6,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            CompareOp::Greater => ">",
            CompareOp::Less => "<",
            CompareOp::GreaterEq => ">=",
            CompareOp::LessEq => "<=",
            CompareOp::Equal => "==",
        }
    }

    pub fn all() -> &'static [CompareOp] {
        &[CompareOp::Greater, CompareOp::Less, CompareOp::GreaterEq, CompareOp::LessEq, CompareOp::Equal]
    }
}

/// Named parameters that control state transitions.
#[derive(Clone, Default)]
pub struct Parameters {
    pub bools: HashMap<String, bool>,
    pub floats: HashMap<String, f32>,
}

impl Parameters {
    pub fn set_bool(&mut self, name: &str, value: bool) {
        self.bools.insert(name.to_string(), value);
    }

    pub fn set_float(&mut self, name: &str, value: f32) {
        self.floats.insert(name.to_string(), value);
    }

    pub fn get_bool(&self, name: &str) -> bool {
        self.bools.get(name).copied().unwrap_or(false)
    }

    pub fn get_float(&self, name: &str) -> f32 {
        self.floats.get(name).copied().unwrap_or(0.0)
    }
}

/// An in-flight transition between two states.
pub struct ActiveTransition {
    /// Index into StateMachine.transitions.
    pub transition_index: usize,
    /// Target state we're blending toward.
    pub target_state: StateId,
    /// Blend controller (reuses blend.rs AnimationTransition).
    pub blend: AnimationTransition,
}

/// Event emitted when a state change completes.
#[derive(Debug)]
pub struct StateChangeEvent {
    pub from: StateId,
    pub to: StateId,
}

impl StateMachine {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            states: Vec::new(),
            transitions: Vec::new(),
            parameters: Parameters::default(),
            active_state: 0,
            active_transition: None,
            state_elapsed: 0.0,
        }
    }

    /// Add a state and return its ID.
    pub fn add_state(&mut self, name: impl Into<String>, source: MotionSource, position: [f32; 2]) -> StateId {
        let id = self.states.len();
        self.states.push(State {
            name: name.into(),
            motion_source: source,
            position,
        });
        id
    }

    /// Add a transition between two states.
    pub fn add_transition(
        &mut self,
        from: StateId,
        to: StateId,
        condition: TransitionCondition,
        crossfade_duration: f32,
        priority: u8,
    ) {
        self.transitions.push(Transition {
            from,
            to,
            condition,
            crossfade_duration,
            priority,
        });
    }

    /// Set a boolean parameter.
    pub fn set_bool(&mut self, name: &str, value: bool) {
        self.parameters.set_bool(name, value);
    }

    /// Set a float parameter.
    pub fn set_float(&mut self, name: &str, value: f32) {
        self.parameters.set_float(name, value);
    }

    /// Get the current active state.
    pub fn current_state(&self) -> &State {
        &self.states[self.active_state]
    }

    /// Get the target state if a transition is in progress.
    pub fn target_state(&self) -> Option<&State> {
        self.active_transition.as_ref().map(|t| &self.states[t.target_state])
    }

    /// Get the current blend weight (0.0 = fully in active state, 1.0 = fully in target).
    pub fn blend_weight(&self) -> f32 {
        self.active_transition.as_ref()
            .map(|t| t.blend.weight())
            .unwrap_or(0.0)
    }

    /// Is a transition currently blending?
    pub fn is_transitioning(&self) -> bool {
        self.active_transition.is_some()
    }

    /// Update the state machine each frame.
    /// `animation_ended` should be true if the current state's animation reached its last frame.
    /// Returns Some(event) when a transition completes.
    pub fn update(&mut self, dt: f32, animation_ended: bool) -> Option<StateChangeEvent> {
        self.state_elapsed += dt;

        // Update active transition blend
        if let Some(ref mut at) = self.active_transition {
            at.blend.update(dt);

            // Transition completed?
            if !at.blend.is_active() {
                let from = self.active_state;
                let to = at.target_state;
                self.active_state = to;
                self.state_elapsed = 0.0;
                self.active_transition = None;
                return Some(StateChangeEvent { from, to });
            }
            // Don't start new transitions while one is in progress
            return None;
        }

        // Evaluate outgoing transitions from current state (sorted by priority)
        if let Some(trans_idx) = self.evaluate_transitions(animation_ended) {
            let trans = &self.transitions[trans_idx];
            let target = trans.to;
            let duration = trans.crossfade_duration;

            let mut blend = AnimationTransition::new(duration);
            blend.start(0.0, 0.0);

            self.active_transition = Some(ActiveTransition {
                transition_index: trans_idx,
                target_state: target,
                blend,
            });
        }

        None
    }

    /// Find the highest-priority outgoing transition whose condition is met.
    fn evaluate_transitions(&self, animation_ended: bool) -> Option<usize> {
        let mut candidates: Vec<(usize, u8)> = Vec::new();

        for (i, trans) in self.transitions.iter().enumerate() {
            if trans.from != self.active_state {
                continue;
            }
            if self.check_condition(&trans.condition, animation_ended) {
                candidates.push((i, trans.priority));
            }
        }

        // Sort by priority descending, pick highest
        candidates.sort_by(|a, b| b.1.cmp(&a.1));
        candidates.first().map(|(idx, _)| *idx)
    }

    /// Check if a transition condition is currently met.
    fn check_condition(&self, condition: &TransitionCondition, animation_ended: bool) -> bool {
        match condition {
            TransitionCondition::BoolParam { name, value } => {
                self.parameters.get_bool(name) == *value
            }
            TransitionCondition::FloatThreshold { name, op, value } => {
                op.evaluate(self.parameters.get_float(name), *value)
            }
            TransitionCondition::TimeElapsed { seconds } => {
                self.state_elapsed >= *seconds
            }
            TransitionCondition::AnimationEnd => animation_ended,
            TransitionCondition::Always => true,
        }
    }

    /// Number of states.
    pub fn num_states(&self) -> usize {
        self.states.len()
    }

    /// Number of transitions.
    pub fn num_transitions(&self) -> usize {
        self.transitions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_state_machine() {
        let mut sm = StateMachine::new("test");
        let idle = sm.add_state("Idle", MotionSource::None, [0.0, 0.0]);
        let walk = sm.add_state("Walk", MotionSource::None, [200.0, 0.0]);

        sm.add_transition(idle, walk,
            TransitionCondition::BoolParam { name: "walking".into(), value: true },
            0.3, 1);
        sm.add_transition(walk, idle,
            TransitionCondition::BoolParam { name: "walking".into(), value: false },
            0.3, 1);

        assert_eq!(sm.active_state, 0);
        assert!(!sm.is_transitioning());

        // No transition yet (walking is false)
        sm.update(0.016, false);
        assert_eq!(sm.active_state, 0);

        // Set walking = true → should start transition to Walk
        sm.set_bool("walking", true);
        sm.update(0.016, false);
        assert!(sm.is_transitioning());

        // Complete the transition
        for _ in 0..30 {
            sm.update(0.016, false);
        }
        assert_eq!(sm.active_state, 1); // Now in Walk state
    }

    #[test]
    fn test_time_elapsed_transition() {
        let mut sm = StateMachine::new("test");
        sm.add_state("A", MotionSource::None, [0.0, 0.0]);
        sm.add_state("B", MotionSource::None, [200.0, 0.0]);
        sm.add_transition(0, 1,
            TransitionCondition::TimeElapsed { seconds: 0.5 },
            0.2, 1);

        // Not enough time
        sm.update(0.1, false);
        assert!(!sm.is_transitioning());

        // Enough time
        sm.update(0.5, false);
        assert!(sm.is_transitioning());
    }

    #[test]
    fn test_float_threshold() {
        let mut sm = StateMachine::new("test");
        sm.add_state("Idle", MotionSource::None, [0.0, 0.0]);
        sm.add_state("Run", MotionSource::None, [200.0, 0.0]);
        sm.add_transition(0, 1,
            TransitionCondition::FloatThreshold {
                name: "speed".into(),
                op: CompareOp::Greater,
                value: 3.0,
            },
            0.3, 1);

        sm.set_float("speed", 1.0);
        sm.update(0.016, false);
        assert!(!sm.is_transitioning());

        sm.set_float("speed", 5.0);
        sm.update(0.016, false);
        assert!(sm.is_transitioning());
    }

    #[test]
    fn test_priority_ordering() {
        let mut sm = StateMachine::new("test");
        sm.add_state("A", MotionSource::None, [0.0, 0.0]);
        sm.add_state("B", MotionSource::None, [200.0, 0.0]);
        sm.add_state("C", MotionSource::None, [400.0, 0.0]);

        // Both conditions are always true, but C has higher priority
        sm.add_transition(0, 1, TransitionCondition::Always, 0.2, 1);
        sm.add_transition(0, 2, TransitionCondition::Always, 0.2, 5);

        sm.update(0.016, false);
        assert!(sm.is_transitioning());
        // Should be transitioning to C (priority 5 > 1)
        assert_eq!(sm.active_transition.as_ref().unwrap().target_state, 2);
    }

    #[test]
    fn test_parameters() {
        let mut params = Parameters::default();
        assert_eq!(params.get_bool("test"), false);
        assert_eq!(params.get_float("speed"), 0.0);

        params.set_bool("test", true);
        params.set_float("speed", 5.5);
        assert_eq!(params.get_bool("test"), true);
        assert!((params.get_float("speed") - 5.5).abs() < 1e-6);
    }

    #[test]
    fn test_compare_ops() {
        assert!(CompareOp::Greater.evaluate(5.0, 3.0));
        assert!(!CompareOp::Greater.evaluate(3.0, 5.0));
        assert!(CompareOp::Less.evaluate(3.0, 5.0));
        assert!(CompareOp::GreaterEq.evaluate(5.0, 5.0));
        assert!(CompareOp::LessEq.evaluate(3.0, 5.0));
        assert!(CompareOp::Equal.evaluate(3.0, 3.0));
    }
}
