use crate::bone::BoneId;
use crate::state::App;
use wasm_bindgen::prelude::*;
/// Default time interval when adding keyframes
const DEFAULT_KEYFRAME_INTERVAL: f32 = 0.5;

// App methods for editor functionality
#[wasm_bindgen]
impl App {
    /// Start editing an animation clip for the given exercise
    pub fn start_editing(&mut self, exercise: crate::bone::AnimationId) -> bool {
        if let Some(clip) = self.state.animation_library.get_clip(exercise).cloned() {
            let name = clip.name.clone();
            self.state.start_editing(clip);
            log::info!("Started editing: {}", name);
            true
        } else {
            log::warn!("No animation loaded for exercise ID: {:?}", exercise);
            false
        }
    }

    /// Stop editing and clear the current session
    pub fn stop_editing(&mut self) {
        self.state.stop_editing();
        log::info!("Stopped editing");
    }

    /// Check if an editing session is active
    pub fn is_editing(&self) -> bool {
        self.state.editor().is_some()
    }

    /// Get the number of keyframes in the current clip
    pub fn get_keyframe_count(&self) -> usize {
        self.state
            .editor()
            .map(|session| session.clip.keyframes.len())
            .unwrap_or(0)
    }

    /// Get the time of the current keyframe
    pub fn get_keyframe_time(&self) -> f32 {
        self.state
            .editor()
            .and_then(|session| {
                session
                    .clip
                    .keyframes
                    .get(session.keyframe_index)
                    .map(|kf| kf.time)
            })
            .unwrap_or(0.0)
    }

    /// Get the current keyframe index
    pub fn get_keyframe_index(&self) -> usize {
        self.state
            .editor()
            .map(|session| session.keyframe_index)
            .unwrap_or(0)
    }

    /// Set the current keyframe index for editing
    pub fn set_keyframe_index(&mut self, index: usize) {
        if let Some(session) = self.state.editor_mut() {
            if index < session.clip.keyframes.len() {
                session.keyframe_index = index;
            }
        }
    }

    /// Add a new keyframe as a copy of the one at after_index
    pub fn add_keyframe(&mut self, after_index: usize) {
        if let Some(session) = self.state.editor_mut() {
            let clip = &mut session.clip;
            if after_index < clip.keyframes.len() {
                let prev_keyframe = clip.keyframes[after_index].clone();
                let new_time = if after_index + 1 < clip.keyframes.len() {
                    (prev_keyframe.time + clip.keyframes[after_index + 1].time) / 2.0
                } else {
                    prev_keyframe.time + DEFAULT_KEYFRAME_INTERVAL
                };

                let mut new_keyframe = prev_keyframe;
                new_keyframe.time = new_time;
                clip.keyframes.insert(after_index + 1, new_keyframe);

                if let Some(last) = clip.keyframes.last() {
                    if last.time > clip.duration {
                        clip.duration = last.time + DEFAULT_KEYFRAME_INTERVAL;
                    }
                }
                log::info!("Added keyframe at time {:.2}s", new_time);
            }
        }
    }

    /// Remove a keyframe by index (won't remove last keyframe)
    pub fn delete_keyframe(&mut self, index: usize) {
        if let Some(session) = self.state.editor_mut() {
            let clip = &mut session.clip;
            if clip.keyframes.len() > 1 && index < clip.keyframes.len() {
                clip.keyframes.remove(index);
                // Adjust keyframe_index if needed
                if session.keyframe_index >= clip.keyframes.len() {
                    session.keyframe_index = clip.keyframes.len().saturating_sub(1);
                }
                log::info!("Removed keyframe {}", index);
            }
        }
    }

    /// Export the current clip as JSON
    pub fn export_clip_json(&self) -> String {
        self.state
            .editor()
            .map(|session| {
                session.clip.to_json_string().unwrap_or_else(|e| {
                    log::error!("Failed to export animation: {}", e);
                    String::from("{}")
                })
            })
            .unwrap_or_else(|| String::from("{}"))
    }

    /// Get joint info for a bone in the current keyframe
    pub fn get_bone_info(&self, bone_index: usize) -> Option<JointInfo> {
        if bone_index >= crate::bone::BoneId::COUNT {
            return None;
        }

        self.state.editor().and_then(|session| {
            let pose = &session.clip.keyframes.get(session.keyframe_index)?.pose;
            let id = crate::bone::BoneId::ALL[bone_index];
            let rot = pose.local_rotations[bone_index];
            let (rx, ry, rz) = rot.to_euler(glam::EulerRot::XYZ);
            let pos = pose.get_position(id);

            Some(JointInfo {
                x: pos.x,
                y: pos.y,
                z: pos.z,
                rx: rx.to_degrees(),
                ry: ry.to_degrees(),
                rz: rz.to_degrees(),
            })
        })
    }

    /// Set joint rotation for a bone in the current keyframe
    pub fn set_bone_rotation(&mut self, bone_index: usize, rx: f32, ry: f32, rz: f32) {
        if bone_index >= crate::bone::BoneId::COUNT {
            return;
        }

        if let Some(session) = self.state.editor_mut() {
            if session.keyframe_index < session.clip.keyframes.len() {
                let pose = &mut session.clip.keyframes[session.keyframe_index].pose;
                let id = crate::bone::BoneId::ALL[bone_index];
                let q = glam::Quat::from_euler(
                    glam::EulerRot::XYZ,
                    rx.to_radians(),
                    ry.to_radians(),
                    rz.to_radians(),
                );
                *pose = std::mem::take(pose).with_rotation(id, q);
            }
        }
    }

    /// Set joint position for a bone using IK/FK in the current keyframe
    pub fn set_bone_position(&mut self, bone_index: usize, x: f32, y: f32, z: f32) {
        if let Some(session) = self.state.editor_mut() {
            if session.keyframe_index >= session.clip.keyframes.len() {
                return;
            }

            let pose = &mut session.clip.keyframes[session.keyframe_index].pose;
            let target_pos = glam::Vec3::new(x, y, z);

            if bone_index == 0 {
                // Root position
                *pose = std::mem::take(pose)
                    .with_root_position(target_pos)
                    .apply_floor_constraint();
                return;
            }

            let (bone_id, chain) = match get_bone_and_chain(bone_index) {
                Some(res) => res,
                None => return,
            };

            if !chain.is_empty() {
                // IK
                *pose = std::mem::take(pose)
                    .apply_ik(&chain, target_pos)
                    .apply_floor_constraint();
            } else {
                // FK Logic
                apply_fk_to_target(pose, bone_id, target_pos);

                // Apply floor constraint
                *pose = std::mem::take(pose).apply_floor_constraint();
            }
        }
    }

    /// Apply a screen-space drag to a joint
    #[allow(clippy::too_many_arguments)]
    pub fn drag_joint(
        &mut self,
        joint_index: usize,
        dx: f32,
        dy: f32,
        view: &[f32],
        proj: &[f32],
        width: f32,
        height: f32,
    ) {
        if view.len() < 16 || proj.len() < 16 {
            return;
        }

        let view_mat = glam::Mat4::from_cols_array(view.try_into().unwrap());
        let proj_mat = glam::Mat4::from_cols_array(proj.try_into().unwrap());

        if let Some(session) = self.state.editor_mut() {
            if session.keyframe_index >= session.clip.keyframes.len() {
                return;
            }

            let pose = &mut session.clip.keyframes[session.keyframe_index].pose;

            // Root drag
            if joint_index == 0 {
                let current_pos = pose.root_position;
                let target_pos =
                    project_and_offset(current_pos, dx, dy, width, height, view_mat, proj_mat);
                *pose = std::mem::take(pose)
                    .with_root_position(target_pos)
                    .apply_floor_constraint();
                return;
            }

            let (bone_id, chain) = match get_bone_and_chain(joint_index) {
                Some(res) => res,
                None => return,
            };

            let current_pos = pose.get_position(bone_id);
            let target_pos =
                project_and_offset(current_pos, dx, dy, width, height, view_mat, proj_mat);

            if !chain.is_empty() {
                // IK
                *pose = std::mem::take(pose)
                    .apply_ik(&chain, target_pos)
                    .apply_floor_constraint();
            } else {
                // FK
                apply_fk_to_target(pose, bone_id, target_pos);
            }
        }
    }

    /// Get screen positions of all joints (for picking)
    pub fn get_joint_positions(
        &self,
        view: &[f32],
        proj: &[f32],
        width: f32,
        height: f32,
    ) -> Vec<f32> {
        if view.len() < 16 || proj.len() < 16 {
            return Vec::new();
        }

        let view_mat = glam::Mat4::from_cols_array(view.try_into().unwrap());
        let proj_mat = glam::Mat4::from_cols_array(proj.try_into().unwrap());
        let view_proj = proj_mat * view_mat;

        self.state
            .editor()
            .and_then(|session| {
                let pose = &session.clip.keyframes.get(session.keyframe_index)?.pose;
                pose.compute_all();
                let cache = pose.cache.borrow();
                use crate::bone::BoneId;
                

                let mut positions = Vec::with_capacity(BoneId::COUNT * 2);
                
                // Iterate over all bones in order of BoneId
                for i in 0..BoneId::COUNT {
                    let joint = cache.world_positions[i];
                    let world_pos = glam::Vec4::new(joint.x, joint.y, joint.z, 1.0);
                    let clip_pos = view_proj * world_pos;

                    if clip_pos.w > 0.0 {
                        let ndc_x = clip_pos.x / clip_pos.w;
                        let ndc_y = clip_pos.y / clip_pos.w;
                        let screen_x = (ndc_x + 1.0) * 0.5 * width;
                        let screen_y = (1.0 - ndc_y) * 0.5 * height;
                        positions.push(screen_x);
                        positions.push(screen_y);
                    } else {
                        positions.push(-1000.0);
                        positions.push(-1000.0);
                    }
                }
                Some(positions)
            })
            .unwrap_or_default()
    }
}

/// Joint position and rotation info for UI display
#[wasm_bindgen]
pub struct JointInfo {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub rx: f32,
    pub ry: f32,
    pub rz: f32,
}

// Helper functions (not exported to JS)

/// Apply FK rotation to point bone toward target direction
fn apply_fk_to_target(
    pose: &mut crate::bone::RotationPose,
    bone_id: BoneId,
    target_pos: glam::Vec3,
) {
    let pivot_pos = if let Some(parent) = crate::bone::BONE_HIERARCHY[bone_id.index()].parent {
        pose.get_position(parent)
    } else {
        pose.root_position
    };
    let target_dir = (target_pos - pivot_pos).normalize_or_zero();
    if target_dir.length_squared() > crate::EPSILON {
        let parent_rot = compute_world_rotation(pose, bone_id);
        let default_dir = crate::bone::BONE_HIERARCHY[bone_id.index()]
            .direction
            .normalize();
        let target_dir_local = parent_rot.inverse() * target_dir;
        let delta_rot = glam::Quat::from_rotation_arc(default_dir, target_dir_local);

        *pose = std::mem::take(pose).with_rotation(bone_id, delta_rot.normalize());
    }
}

fn get_bone_and_chain(joint_index: usize) -> Option<(BoneId, Vec<BoneId>)> {
    // Indices correspond to BoneId enum values
    match joint_index {
        // Arm Chains (IK on Wrist)
        20 => Some(( // LeftWrist
            BoneId::LeftWrist,
            vec![
                BoneId::LeftCollar,
                BoneId::LeftShoulder,
                BoneId::LeftElbow,
                BoneId::LeftWrist,
            ],
        )),
        21 => Some(( // RightWrist
            BoneId::RightWrist,
            vec![
                BoneId::RightCollar,
                BoneId::RightShoulder,
                BoneId::RightElbow,
                BoneId::RightWrist,
            ],
        )),
        
        // Leg Chains (IK on Foot/Ankle)
        7 => Some(( // LeftAnkle - IK usually to Ankle
             BoneId::LeftAnkle,
             vec![BoneId::LeftHip, BoneId::LeftKnee, BoneId::LeftAnkle]
        )),
        10 => Some(( // LeftFoot - Extending chain
             BoneId::LeftFoot,
             vec![BoneId::LeftHip, BoneId::LeftKnee, BoneId::LeftAnkle, BoneId::LeftFoot]
        )),
        8 => Some(( // RightAnkle
             BoneId::RightAnkle,
             vec![BoneId::RightHip, BoneId::RightKnee, BoneId::RightAnkle]
        )),
        11 => Some(( // RightFoot
             BoneId::RightFoot,
             vec![BoneId::RightHip, BoneId::RightKnee, BoneId::RightAnkle, BoneId::RightFoot]
        )),
        
        // Spine/Head Chain
        15 => Some(( // Head
            BoneId::Head,
            vec![BoneId::Spine1, BoneId::Spine2, BoneId::Spine3, BoneId::Neck, BoneId::Head],
        )),
        
        // Individual Bones (FK)
        0 => None, // Pelvis (Root handled separately)
        1 => Some((BoneId::LeftHip, vec![])),
        2 => Some((BoneId::RightHip, vec![])),
        3 => Some((BoneId::Spine1, vec![])),
        4 => Some((BoneId::LeftKnee, vec![])),
        5 => Some((BoneId::RightKnee, vec![])),
        6 => Some((BoneId::Spine2, vec![])),
        9 => Some((BoneId::Spine3, vec![])),
        12 => Some((BoneId::Neck, vec![])),
        13 => Some((BoneId::LeftCollar, vec![])),
        14 => Some((BoneId::RightCollar, vec![])),
        16 => Some((BoneId::LeftShoulder, vec![])),
        17 => Some((BoneId::RightShoulder, vec![])),
        18 => Some((BoneId::LeftElbow, vec![])),
        19 => Some((BoneId::RightElbow, vec![])),
        
        _ => None,
    }
}

/// Compute world rotation for a bone's parent (helper for FK)
fn compute_world_rotation(pose: &crate::bone::RotationPose, bone_id: BoneId) -> glam::Quat {
    if let Some(parent) = crate::bone::BONE_HIERARCHY[bone_id.index()].parent {
        let mut rot = pose.local_rotations[parent.index()];
        let mut p = parent;
        while let Some(grandparent) = crate::bone::BONE_HIERARCHY[p.index()].parent {
            rot = pose.local_rotations[grandparent.index()] * rot;
            p = grandparent;
        }
        rot
    } else {
        glam::Quat::IDENTITY
    }
}

/// Helper to project a 3D point to screen, offset it, and unproject back
fn project_and_offset(
    pos: glam::Vec3,
    dx: f32,
    dy: f32,
    width: f32,
    height: f32,
    view: glam::Mat4,
    proj: glam::Mat4,
) -> glam::Vec3 {
    let view_proj = proj * view;
    let ndc_pos = view_proj.project_point3(pos);
    let screen_x = (ndc_pos.x + 1.0) * 0.5 * width;
    let screen_y = (1.0 - ndc_pos.y) * 0.5 * height;
    let target_screen_x = screen_x + dx;
    let target_screen_y = screen_y + dy;
    let target_ndc_x = (target_screen_x / (width * 0.5)) - 1.0;
    let target_ndc_y = 1.0 - (target_screen_y / (height * 0.5));
    let target_ndc = glam::Vec3::new(target_ndc_x, target_ndc_y, ndc_pos.z);
    let inverse_vp = view_proj.inverse();
    let clip_pos = glam::Vec4::new(target_ndc.x, target_ndc.y, target_ndc.z, 1.0);
    let mut world_pos = inverse_vp * clip_pos;
    if world_pos.w != 0.0 {
        world_pos /= world_pos.w;
    }
    glam::Vec3::new(world_pos.x, world_pos.y, world_pos.z)
}
