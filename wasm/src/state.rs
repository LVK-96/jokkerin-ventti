//! Centralized application state with context passing pattern
//!
//! Implements a Context Passing pattern where:
//! 1. `AppState` is a single struct containing all application state
//! 2. Core functions take explicit references (e.g., `&GpuContext`, `&Camera`)
//! 3. WASM bindings are thin wrappers that extract from AppState and call pure functions
//!
//! This design enables:
//! - Unit testing of core logic without global state
//! - Clear dependency graphs

use std::cell::RefCell;

use crate::animation::{AnimationLibrary, PlaybackState};
use crate::camera::Camera;
use crate::gpu::GpuContext;

/// Editor session data - editing state for a single animation clip
pub struct EditorSession {
    pub clip: crate::bone::RotationAnimationClip,
    pub keyframe_index: usize,
}

/// Functions should take explicit references to what they need, not access
/// this struct directly via globals.
pub struct AppState {
    /// WebGPU context - device, queue, pipelines, buffers
    pub gpu: GpuContext,
    /// Loaded animation clips (read-only during playback)
    pub animation_library: AnimationLibrary,
    /// Current animation playback state (exercise, time)
    pub playback: PlaybackState,
    /// Camera orientation and distance
    pub camera: Camera,
    /// Active editor session (singleton - only one at a time)
    pub editor_session: Option<EditorSession>,
}

impl AppState {
    /// Create new application state with initialized GPU context
    /// GpuContext is initialized sepately and passed here
    pub fn new(gpu: GpuContext) -> Self {
        Self {
            gpu,
            animation_library: AnimationLibrary::new(),
            playback: PlaybackState::default(),
            camera: Camera::default(),
            editor_session: None,
        }
    }

    /// Start editing an animation clip
    pub fn start_editing(&mut self, clip: crate::bone::RotationAnimationClip) {
        self.editor_session = Some(EditorSession {
            clip,
            keyframe_index: 0,
        });
    }

    /// Stop editing (clear current session)
    pub fn stop_editing(&mut self) {
        self.editor_session = None;
    }

    /// Get mutable reference to current editor session
    pub fn editor_mut(&mut self) -> Option<&mut EditorSession> {
        self.editor_session.as_mut()
    }

    /// Get immutable reference to current editor session
    pub fn editor(&self) -> Option<&EditorSession> {
        self.editor_session.as_ref()
    }
}

// Global state access, thin wrapper for WASM bindings only
thread_local! {
    static APP_STATE: RefCell<Option<AppState>> = const { RefCell::new(None) };
}

/// Execute a closure with immutable access to AppState
///
/// Returns None if AppState is not initialized
pub fn with_app_state<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&AppState) -> R,
{
    APP_STATE.with(|state| {
        let borrowed = state.borrow();
        borrowed.as_ref().map(f)
    })
}

/// Execute a closure with mutable access to AppState
///
/// Returns None if AppState is not initialized
pub fn with_app_state_mut<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut AppState) -> R,
{
    APP_STATE.with(|state| {
        let mut borrowed = state.borrow_mut();
        borrowed.as_mut().map(f)
    })
}

/// Initialize the global AppState with a GpuContext
///
/// Called once during init_gpu()
pub fn initialize_app_state(gpu: GpuContext) {
    APP_STATE.with(|state| {
        *state.borrow_mut() = Some(AppState::new(gpu));
    });
}
