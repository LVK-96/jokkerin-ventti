//! Animation module - pure functional animation state and sampling
//!
//! Separates animation library (read-only clips) from playback state (current exercise/time).

use crate::bone_hierarchy::{RotationAnimationClip, RotationPose};
use std::collections::HashMap;

/// Animation library - loaded once, read-only during playback
///
/// Stores all available animation clips by name.
/// This is separate from playback state so clips can be shared/referenced.
#[derive(Default)]
pub struct AnimationLibrary {
    clips: HashMap<String, RotationAnimationClip>,
}

impl AnimationLibrary {
    /// Create empty animation library
    pub fn new() -> Self {
        Self {
            clips: HashMap::new(),
        }
    }

    /// Add an animation clip to the library
    pub fn add_clip(&mut self, clip: RotationAnimationClip) {
        self.clips.insert(clip.name.clone(), clip);
    }

    /// Get a clip by name
    pub fn get_clip(&self, name: &str) -> Option<&RotationAnimationClip> {
        self.clips.get(name)
    }

    /// Check if a clip exists
    pub fn has_clip(&self, name: &str) -> bool {
        self.clips.contains_key(name)
    }

    /// Get all clip names
    pub fn clip_names(&self) -> impl Iterator<Item = &String> {
        self.clips.keys()
    }
}

/// Playback state - current animation being played
///
/// Immutable value type - can be replaced entirely each frame.
#[derive(Clone, Debug, Default)]
pub struct PlaybackState {
    /// Current exercise name
    pub exercise: String,
    /// Current time in seconds (modulo duration for looping)
    pub time: f32,
}

impl PlaybackState {
    /// Create new playback state
    pub fn new(exercise: String) -> Self {
        Self {
            exercise,
            time: 0.0,
        }
    }

    /// Advance time by delta (does not loop - that's done during sampling)
    pub fn advance(self, delta_seconds: f32) -> PlaybackState {
        PlaybackState {
            time: self.time + delta_seconds,
            ..self
        }
    }

    /// Change exercise, reset time
    pub fn set_exercise(self, exercise: String) -> PlaybackState {
        PlaybackState {
            exercise,
            time: 0.0,
        }
    }
}

/// Sample animation - pure function
///
/// Given a library and playback state, return the current pose.
/// Returns bind pose if exercise not found.
pub fn sample_animation(library: &AnimationLibrary, state: &PlaybackState) -> RotationPose {
    if let Some(clip) = library.get_clip(&state.exercise) {
        clip.sample(state.time)
    } else {
        RotationPose::bind_pose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_library_returns_bind_pose() {
        let library = AnimationLibrary::new();
        let state = PlaybackState::new("nonexistent".to_string());

        let pose = sample_animation(&library, &state);
        // Should return bind pose without panicking
        assert_eq!(pose.root_position, RotationPose::bind_pose().root_position);
    }

    #[test]
    fn test_playback_advance() {
        let state = PlaybackState::new("test".to_string());
        let advanced = state.advance(1.5);

        assert_eq!(advanced.time, 1.5);
        assert_eq!(advanced.exercise, "test");
    }

    #[test]
    fn test_set_exercise_resets_time() {
        let state = PlaybackState {
            exercise: "old".to_string(),
            time: 5.0,
        };
        let changed = state.set_exercise("new".to_string());

        assert_eq!(changed.exercise, "new");
        assert_eq!(changed.time, 0.0);
    }
}

// --- State Management ---
// Moved from lib.rs/gpu.rs

use std::cell::RefCell;
use wasm_bindgen::prelude::*;

thread_local! {
    pub static ANIMATION_LIBRARY: RefCell<AnimationLibrary> = RefCell::new(AnimationLibrary::new());
    pub static PLAYBACK_STATE: RefCell<PlaybackState> = RefCell::new(PlaybackState::default());
}

/// Set the current exercise for animation
#[wasm_bindgen]
pub fn set_exercise(name: String) {
    // Update PlaybackState (reset time to 0)
    PLAYBACK_STATE.with(|p| {
        let mut state = p.borrow_mut();
        *state = state.clone().set_exercise(name.clone());
    });

    log::info!("Exercise set to: {}", name);
}

/// Helper for logging
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// Load an animation clip from JSON string
/// Call this during startup for each exercise you want to animate
#[wasm_bindgen]
pub fn load_animation(name_override: String, json_data: String) -> Result<(), JsValue> {
    // Parse into a generic Value first to check the version
    let v: serde_json::Value = serde_json::from_str(&json_data)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse JSON: {}", e)))?;

    // Check version
    let is_v2 = v.get("version").and_then(|val| val.as_u64()) == Some(2);

    if is_v2 {
        // Version 2: Rotation-based animation
        // Use from_json helper because RotationAnimationClip doesn't impl Deserialize directly
        let mut clip = RotationAnimationClip::from_json(&json_data)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse JSON: {}", e)))?;

        // Override name with the one provided by JS (the map key)
        clip.name = name_override.clone();
        let name = clip.name.clone();

        // Store in AnimationLibrary
        ANIMATION_LIBRARY.with(|lib| {
            lib.borrow_mut().add_clip(clip);
        });

        log::info!("Loaded animation (v2): {}", name);
    } else {
        return Err(JsValue::from_str("Only version 2 animations are supported"));
    }

    Ok(())
}

/// Advance simulation time (call each frame with delta time)
#[wasm_bindgen]
pub fn advance_time(delta_ms: f32) {
    let delta_secs = delta_ms / 1000.0;

    // Update playback state time (for animation sampling)
    PLAYBACK_STATE.with(|p| {
        let mut state = p.borrow_mut();
        state.time += delta_secs;
    });
}
