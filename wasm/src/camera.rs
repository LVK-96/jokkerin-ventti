use glam::{Mat4, Quat, Vec3};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Elevation limits as dot product of camera direction with world up
const MIN_UP_DOT: f32 = 0.05; // Camera must be at least slightly above target
const MAX_UP_DOT: f32 = 0.98; // Don't allow looking straight down

/// Target point for orbit camera (center of stickman)
pub const CAMERA_TARGET: Vec3 = Vec3::new(0.0, 0.5, 0.0);

/// The camera orbits around a fixed target point. Its position is determined
/// by rotating a "back" vector (0, 0, distance) by the orientation quaternion.
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    /// Quaternion representing camera's orbital rotation
    pub orientation: Quat,
    /// Distance from target point
    pub distance: f32,
}

impl Default for Camera {
    fn default() -> Self {
        // Default: camera above and to the side of target
        // yaw=0.7, pitch=-0.25 (negative pitch = camera above target)
        let yaw = 0.7_f32;
        let pitch = -0.25_f32;
        let yaw_quat = Quat::from_rotation_y(yaw);
        let pitch_quat = Quat::from_rotation_x(pitch);

        Self {
            orientation: (yaw_quat * pitch_quat).normalize(),
            distance: 4.0,
        }
    }
}

impl Camera {
    /// Create a new camera with specified orientation and distance
    pub fn new(orientation: Quat, distance: f32) -> Self {
        Self {
            orientation,
            distance,
        }
    }

    /// Compute new camera with rotation applied
    ///
    /// Returns a new Camera with the rotation applied, or the original
    /// camera if the rotation would exceed elevation limits.
    pub fn with_rotation(self, axis: Vec3, angle: f32) -> Camera {
        let axis = axis.normalize_or_zero();
        if axis.length_squared() < 0.5 {
            return self; // Invalid axis
        }

        let delta = Quat::from_axis_angle(axis, angle);
        let new_orientation = (delta * self.orientation).normalize();

        // Check if new orientation exceeds elevation limits
        let forward = Vec3::Z;
        let new_dir = new_orientation * forward;
        let up_dot = new_dir.y;

        if (MIN_UP_DOT..=MAX_UP_DOT).contains(&up_dot) {
            Camera {
                orientation: new_orientation,
                ..self
            }
        } else {
            // Check if we're moving toward valid range
            let old_dir = self.orientation * forward;
            let old_up_dot = old_dir.y;
            let moving_to_valid = (old_up_dot < MIN_UP_DOT && up_dot > old_up_dot)
                || (old_up_dot > MAX_UP_DOT && up_dot < old_up_dot);

            if moving_to_valid {
                Camera {
                    orientation: new_orientation,
                    ..self
                }
            } else {
                self // Reject rotation
            }
        }
    }

    /// Compute camera eye position
    pub fn eye_position(&self) -> Vec3 {
        let offset = self.orientation * Vec3::new(0.0, 0.0, self.distance);
        CAMERA_TARGET + offset
    }

    /// Compute camera's local right axis
    ///
    /// This is the axis to rotate around for up/down elevation changes.
    /// Computed as cross product of world up and view direction.
    pub fn right_axis(&self) -> Vec3 {
        let eye = self.eye_position();
        let forward = (CAMERA_TARGET - eye).normalize_or_zero();
        let right = forward.cross(Vec3::Y).normalize_or_zero();
        // Return X axis if degenerate (looking straight up/down)
        if right.length_squared() < 0.5 {
            Vec3::X
        } else {
            right
        }
    }

    /// Compute view matrix
    ///
    /// Uses world up (Y axis) for the up vector to ensure proper orbit behavior
    /// without unwanted roll.
    pub fn view_matrix(&self) -> Mat4 {
        // Use world up for orbit camera (prevents roll)
        Mat4::look_at_rh(self.eye_position(), CAMERA_TARGET, Vec3::Y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_camera_above_floor() {
        let camera = Camera::default();
        let eye = camera.eye_position();
        // Camera should be above the target (y > 0.5)
        assert!(
            eye.y > CAMERA_TARGET.y,
            "Camera eye Y={} should be above target Y={}",
            eye.y,
            CAMERA_TARGET.y
        );
    }

    #[test]
    fn test_camera_rotation_clamping() {
        let camera = Camera::default();

        // Try to rotate way down (should be clamped)
        let rotated = camera.with_rotation(Vec3::X, -std::f32::consts::PI);
        let _eye = rotated.eye_position();

        // Camera should still be above minimum elevation
        let dir = rotated.orientation * Vec3::Z;
        assert!(
            dir.y >= MIN_UP_DOT,
            "Camera should be clamped to min elevation"
        );
    }

    #[test]
    fn test_view_matrix_looks_at_target() {
        let camera = Camera::default();
        let view = camera.view_matrix();

        // View matrix should be valid (not NaN)
        let cols = view.to_cols_array();
        for val in cols {
            assert!(!val.is_nan(), "View matrix should not contain NaN");
        }
    }
}

// App methods for camera control
#[cfg(target_arch = "wasm32")]
use crate::state::App;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl App {
    /// Update camera from spherical coordinates (orbit camera)
    /// azimuth: horizontal angle in radians (0 = front, PI/2 = right side)
    /// elevation: vertical angle in radians (0 = level, PI/2 = top-down)
    /// distance: distance from target point
    pub fn update_camera(&mut self, azimuth: f32, elevation: f32, distance: f32) {
        // Convert spherical to quaternion
        let yaw_quat = Quat::from_rotation_y(azimuth);
        let pitch_quat = Quat::from_rotation_x(elevation);
        let orientation = (yaw_quat * pitch_quat).normalize();

        self.state.camera = Camera {
            orientation,
            distance,
        };
    }

    /// Apply a rotation to the camera around a world-space axis
    ///
    /// Rotates the camera's stored quaternion orientation incrementally.
    /// Clamps elevation to prevent going under floor or directly overhead.
    ///
    /// # Arguments
    /// * `axis_x, axis_y, axis_z` - World-space axis to rotate around (should be normalized)
    /// * `angle` - Rotation angle in radians
    pub fn rotate_camera(&mut self, axis_x: f32, axis_y: f32, axis_z: f32, angle: f32) {
        let axis = Vec3::new(axis_x, axis_y, axis_z);
        self.state.camera = self.state.camera.with_rotation(axis, angle);
    }

    /// Get the camera's right axis (for vertical input rotation)
    pub fn get_camera_right_axis(&self) -> Vec<f32> {
        self.state.camera.right_axis().to_array().to_vec()
    }
}
