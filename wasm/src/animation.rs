use crate::bone::{AnimationId, RotationAnimationClip, RotationPose};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Animation library - loaded once, read-only during playback
///
/// Stores all available animation clips by enum ID.
/// This avoids hash map lookups entirely.
pub struct AnimationLibrary {
    // Fixed size array, indexed by AnimationId
    clips: [Option<RotationAnimationClip>; AnimationId::COUNT],
}

impl Default for AnimationLibrary {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimationLibrary {
    /// Create empty animation library
    pub fn new() -> Self {
        const NONE_CLIP: Option<RotationAnimationClip> = None;
        Self {
            clips: [NONE_CLIP; AnimationId::COUNT],
        }
    }

    /// Add an animation clip to the library
    pub fn add_clip(&mut self, id: AnimationId, clip: RotationAnimationClip) {
        self.clips[id.index()] = Some(clip);
    }

    /// Get a clip by name
    pub fn get_clip(&self, id: AnimationId) -> Option<&RotationAnimationClip> {
        self.clips[id.index()].as_ref()
    }

    /// Check if a clip exists
    pub fn has_clip(&self, id: AnimationId) -> bool {
        self.clips[id.index()].is_some()
    }
}

/// Playback state - current animation being played
///
/// Immutable value type - can be replaced entirely each frame.
#[derive(Clone, Debug, Default)]
pub struct PlaybackState {
    /// Current exercise ID
    pub exercise: Option<AnimationId>,
    /// Current time in seconds (modulo duration for looping)
    pub time: f32,
}

impl PlaybackState {
    /// Create new playback state
    pub fn new(exercise: AnimationId) -> Self {
        Self {
            exercise: Some(exercise),
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
    pub fn set_exercise(self, exercise: AnimationId) -> PlaybackState {
        PlaybackState {
            exercise: Some(exercise),
            time: 0.0,
        }
    }
}

/// Sample animation
///
/// Given a library and playback state, return the current pose.
/// Returns bind pose if exercise not found.
pub fn sample_animation(library: &AnimationLibrary, state: &PlaybackState) -> RotationPose {
    let id = match state.exercise {
        Some(id) => id,
        None => return RotationPose::bind_pose(),
    };

    // 1. Try to get the specific exercise clip
    if let Some(clip) = library.get_clip(id) {
        return clip.sample(state.time);
    }

    // 2. Fallback to master placeholder if specific clip not loaded
    if let Some(clip) = library.get_clip(AnimationId::Placeholder) {
        return clip.sample(state.time);
    }

    // 3. Absolute fallback is bind pose
    RotationPose::bind_pose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[test]
    #[wasm_bindgen_test]
    fn test_empty_library_returns_bind_pose() {
        let library = AnimationLibrary::new();
        // Just pick first enum variant for testing
        let state = PlaybackState::new(AnimationId::PushUps);

        let pose = sample_animation(&library, &state);
        // Should return bind pose without panicking
        assert_eq!(pose.root_position, RotationPose::bind_pose().root_position);
    }

    #[test]
    #[wasm_bindgen_test]
    fn test_playback_advance() {
        let state = PlaybackState::new(AnimationId::PushUps);
        let advanced = state.advance(1.5);

        assert_eq!(advanced.time, 1.5);
        assert_eq!(advanced.exercise, Some(AnimationId::PushUps));
    }

    #[test]
    #[wasm_bindgen_test]
    fn test_set_exercise_resets_time() {
        let state = PlaybackState {
            exercise: Some(AnimationId::PushUps),
            time: 5.0,
        };
        let changed = state.set_exercise(AnimationId::PushUps);

        assert_eq!(changed.exercise, Some(AnimationId::PushUps));
        assert_eq!(changed.time, 0.0);
    }
}

// App methods for animation
#[cfg(target_arch = "wasm32")]
use crate::state::App;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl App {
    /// Set the current exercise for animation
    pub fn set_exercise(&mut self, id: AnimationId) {
        self.state.playback = self.state.playback.clone().set_exercise(id);
        log::info!("Exercise set to: {:?}", id);
    }

    /// Load an animation clip from JSON string
    /// Call this during startup for each exercise you want to animate
    pub fn load_animation(&mut self, id: AnimationId, json_data: String) -> Result<(), JsValue> {
        // Use from_json helper because RotationAnimationClip doesn't impl Deserialize directly
        let clip = RotationAnimationClip::from_json(&json_data)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse JSON: {}", e)))?;

        self.state.animation_library.add_clip(id, clip);

        Ok(())
    }

    /// Load an animation clip from binary data
    /// This is the preferred method for production - smaller files, faster parsing
    pub fn load_animation_binary(&mut self, id: AnimationId, data: &[u8]) -> Result<(), JsValue> {
        let name = format!("{:?}", id);
        let clip = RotationAnimationClip::from_binary(data, name)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse binary: {}", e)))?;

        self.state.animation_library.add_clip(id, clip);

        Ok(())
    }

    /// Advance simulation time (call each frame with delta time)
    pub fn advance_time(&mut self, delta_ms: f32) {
        let delta_secs = delta_ms / 1000.0;
        self.state.playback.time += delta_secs;
    }
}
