use crate::GPU_STATE;

use wasm_bindgen::prelude::*;

// =============================================================================
// KEYFRAME EDITOR FUNCTIONS
// =============================================================================

/// Enter editor mode for the current exercise
#[wasm_bindgen]
pub fn enter_editor_mode() {
    GPU_STATE.with(|s| {
        let mut state_ref = s.borrow_mut();
        if let Some(state) = state_ref.as_mut() {
            // Clone the current animation for editing
            // Note: keys in animations map are lowercase
            if let Some(clip) = state.animations.get(&state.current_exercise_name) {
                state.editor_clip = Some(clip.clone());
                state.editor_mode = true;
                state.editor_keyframe_index = 0;
                log::info!("Editor mode enabled for: {}", state.current_exercise_name);
            } else {
                log::warn!("No animation loaded for current exercise");
            }
        }
    });
}

/// Exit editor mode
#[wasm_bindgen]
pub fn exit_editor_mode() {
    GPU_STATE.with(|s| {
        let mut state_ref = s.borrow_mut();
        if let Some(state) = state_ref.as_mut() {
            state.editor_mode = false;
            state.editor_clip = None;
            log::info!("Editor mode disabled");
        }
    });
}

/// Get the number of keyframes in the current animation
#[wasm_bindgen]
pub fn get_animation_keyframe_count() -> usize {
    GPU_STATE.with(|s| {
        let state_ref = s.borrow();
        if let Some(state) = state_ref.as_ref() {
            if let Some(clip) = &state.editor_clip {
                return clip.keyframes.len();
            }
        }
        0
    })
}

/// Get the time of the current keyframe
#[wasm_bindgen]
pub fn get_current_keyframe_time() -> f32 {
    GPU_STATE.with(|s| {
        let state_ref = s.borrow();
        if let Some(state) = state_ref.as_ref() {
            if let Some(clip) = &state.editor_clip {
                if state.editor_keyframe_index < clip.keyframes.len() {
                    return clip.keyframes[state.editor_keyframe_index].time;
                }
            }
        }
        0.0
    })
}

/// Set the current keyframe index for editing
#[wasm_bindgen]
pub fn set_editor_keyframe(index: usize) {
    GPU_STATE.with(|s| {
        let mut state_ref = s.borrow_mut();
        if let Some(state) = state_ref.as_mut() {
            if let Some(clip) = &state.editor_clip {
                if index < clip.keyframes.len() {
                    state.editor_keyframe_index = index;
                }
            }
        }
    });
}

/// Get screen positions of all joints for picking
/// Returns a flat array: [x0, y0, x1, y1, ...]
#[wasm_bindgen]
pub fn get_joint_screen_positions() -> Vec<f32> {
    GPU_STATE.with(|s| {
        let state_ref = s.borrow();
        if let Some(state) = state_ref.as_ref() {
            if let Some(clip) = &state.editor_clip {
                if state.editor_keyframe_index < clip.keyframes.len() {
                    let keyframe = &clip.keyframes[state.editor_keyframe_index];
                    // Clone pose because to_skeleton() might need mutable access or just for safety
                    let mut pose = keyframe.pose.clone();
                    let skeleton = pose.to_skeleton();

                    // Compute view-projection matrix from separate view and projection
                    let view = glam::Mat4::from_cols_array_2d(&state.uniforms.view);
                    let projection = glam::Mat4::from_cols_array_2d(&state.uniforms.projection);
                    let view_proj = projection * view;

                    // Convert each joint position to screen space
                    let joints = [
                        skeleton.hips,
                        skeleton.neck,
                        skeleton.neck, // Neck joint (same as spine end)
                        skeleton.head,
                        skeleton.left_shoulder,
                        skeleton.left_elbow,
                        skeleton.left_hand,
                        skeleton.right_shoulder,
                        skeleton.right_elbow,
                        skeleton.right_hand,
                        skeleton.left_hip,
                        skeleton.left_knee,
                        skeleton.left_foot,
                        skeleton.right_hip,
                        skeleton.right_knee,
                        skeleton.right_foot,
                    ];

                    let mut screen_positions = Vec::with_capacity(joints.len() * 2);

                    for joint in &joints {
                        let world_pos = glam::Vec4::new(joint.x, joint.y, joint.z, 1.0);
                        let clip_pos = view_proj * world_pos;

                        // Perspective divide to get NDC
                        if clip_pos.w > 0.0 {
                            let ndc_x = clip_pos.x / clip_pos.w;
                            let ndc_y = clip_pos.y / clip_pos.w;

                            // Convert NDC (-1 to 1) to screen space
                            let screen_x = (ndc_x + 1.0) * 0.5 * state.config.width as f32;
                            let screen_y = (1.0 - ndc_y) * 0.5 * state.config.height as f32; // Y flipped

                            screen_positions.push(screen_x);
                            screen_positions.push(screen_y);
                        } else {
                            // Behind camera
                            screen_positions.push(-1000.0);
                            screen_positions.push(-1000.0);
                        }
                    }

                    return screen_positions;
                }
            }
        }
        Vec::new()
    })
}

/// Apply a screen-space drag to a joint, computing new rotations via IK
#[wasm_bindgen]
pub fn apply_joint_drag(joint_index: usize, dx: f32, dy: f32) {
    use crate::bone_hierarchy::BoneId;

    GPU_STATE.with(|s| {
        let mut state_ref = s.borrow_mut();
        if let Some(state) = state_ref.as_mut() {
            // Need copies of width/height and matrices to avoid borrow issues with state later
            let width = state.config.width as f32;
            let height = state.config.height as f32;
            let view = glam::Mat4::from_cols_array_2d(&state.uniforms.view);
            let proj = glam::Mat4::from_cols_array_2d(&state.uniforms.projection);

            if let Some(clip) = state.editor_clip.as_mut() {
                if state.editor_keyframe_index < clip.keyframes.len() {
                    let keyframe = &mut clip.keyframes[state.editor_keyframe_index];

                    // 1. Map index to BoneId and Chain
                    // Indices correspond to .to_skeleton() order
                    // 0: Hips
                    // 1: Neck (spine end)
                    // 2: Neck (dup) -> Head start?
                    // 3: Head (end)
                    // ... see get_joint_screen_positions for mapping

                    // Mapping from get_joint_screen_positions:
                    // 0: Hips
                    // 1: Neck
                    // 2: Neck (dup)
                    // 3: Head
                    // 4: LShoulder, 5: LElbow, 6: LHand
                    // 7: RShoulder, 8: RElbow, 9: RHand
                    // 10: LHip, 11: LKnee, 12: LFoot
                    // 13: RHip, 14: RKnee, 15: RFoot

                    // Simple root translation
                    if joint_index == 0 {
                        // Dragging root needs to be camera aware too ideally, but simple xy is okay for now
                        // Or better: project/unproject logic same as others

                        let current_pos = keyframe.pose.root_position;

                        let target_pos =
                            project_and_offset(current_pos, dx, dy, width, height, view, proj);

                        keyframe.pose.set_root_position(target_pos);
                        keyframe.pose.apply_floor_constraint();
                        return;
                    }

                    // IK Chains or FK Bones
                    let (bone_id, chain) = match joint_index {
                        // IK Chains (End Effectors)
                        6 => (
                            BoneId::LeftForearm,
                            vec![
                                BoneId::LeftShoulder,
                                BoneId::LeftUpperArm,
                                BoneId::LeftForearm,
                            ],
                        ),
                        9 => (
                            BoneId::RightForearm,
                            vec![
                                BoneId::RightShoulder,
                                BoneId::RightUpperArm,
                                BoneId::RightForearm,
                            ],
                        ),
                        12 => (BoneId::LeftShin, vec![BoneId::LeftThigh, BoneId::LeftShin]),
                        15 => (
                            BoneId::RightShin,
                            vec![BoneId::RightThigh, BoneId::RightShin],
                        ),
                        3 => (
                            BoneId::Head,
                            vec![BoneId::Spine, BoneId::Neck, BoneId::Head],
                        ),
                        1 => (BoneId::Spine, vec![BoneId::Spine]),
                        2 => (BoneId::Neck, vec![BoneId::Neck]),


                        // FK Bones (Intermediate Joints)
                        5 => (BoneId::LeftUpperArm, vec![]),
                        8 => (BoneId::RightUpperArm, vec![]),
                        4 => (BoneId::LeftShoulder, vec![]),
                        7 => (BoneId::RightShoulder, vec![]),
                        11 => (BoneId::LeftThigh, vec![]),
                        14 => (BoneId::RightThigh, vec![]),

                        _ => {
                            // Not a draggable target
                            return;
                        }
                    };

                    // 2. Calculate target position in world space
                    let current_pos = keyframe.pose.get_position(bone_id);
                    let target_pos =
                        project_and_offset(current_pos, dx, dy, width, height, view, proj);

                    // 3. Apply IK or FK
                    if !chain.is_empty() {
                        keyframe.pose.apply_ik(&chain, target_pos);
                        keyframe.pose.apply_floor_constraint();
                    } else {
                        // FK Logic
                        // Identify pivot bone (parent of the bone being rotated)
                        // The bone_id we mapped is the bone being rotated (e.g. LeftUpperArm)
                        // Its start position is the pivot.
                        // Start pos = End pos of parent.

                        let pivot_pos = if let Some(parent) =
                            crate::bone_hierarchy::BONE_HIERARCHY[bone_id.index()].parent
                        {
                            keyframe.pose.get_position(parent)
                        } else {
                            keyframe.pose.root_position
                        };

                        let target_dir = (target_pos - pivot_pos).normalize_or_zero();
                        if target_dir.length_squared() > 1e-6 {
                            // Get parent world rotation
                            let parent_rot = if let Some(parent) =
                                crate::bone_hierarchy::BONE_HIERARCHY[bone_id.index()].parent
                            {
                                // We need to compute world rotation. RotationPose caches it but we need it up to date.
                                // compute_bone updates it.
                                // However, RotationPose struct doesn't expose get_rotation(bone) (world rot).
                                // It exposes local_rotations (pub).
                                // We need a helper to get world rotation or recompute it.
                                // Accessing private fields is not possible unless we change RotationPose or use a method.
                                // RotationPose has `get_rotation(bone)`? No, it has `compute_bone` which returns World Transform.
                                // But compute_bone is private? No, let's check.

                                // Actually compute_bone updates internal cache.
                                // We can't access internal cache `world_rotations`.
                                // We need to calculate parent world rotation manually or add a getter.

                                // Let's assume we can compute it by traversing up? Expensive.
                                // Or add `get_global_rotation` to RotationPose?
                                // Or just make `world_rotations` public?
                                // Making it public is easiest for Editor.
                                // But for now, let's calculate it by composing locals.

                                let mut rot = keyframe.pose.local_rotations[parent.index()];
                                let mut p = parent;
                                while let Some(grandparent) =
                                    crate::bone_hierarchy::BONE_HIERARCHY[p.index()].parent
                                {
                                    rot = keyframe.pose.local_rotations[grandparent.index()] * rot;
                                    p = grandparent;
                                }
                                rot
                            } else {
                                glam::Quat::IDENTITY
                            };

                            let default_dir = crate::bone_hierarchy::BONE_HIERARCHY
                                [bone_id.index()]
                            .direction
                            .normalize();
                            let target_dir_local = parent_rot.inverse() * target_dir;

                            let delta_rot =
                                glam::Quat::from_rotation_arc(default_dir, target_dir_local);

                            keyframe.pose.set_rotation(bone_id, delta_rot.normalize());
                        }
                    }

                    keyframe.pose.mark_all_dirty();
                }
            }
        }
    });
}

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

    // Project current position to NDC
    let ndc_pos = view_proj.project_point3(pos);

    // Convert NDC to Screen
    let screen_x = (ndc_pos.x + 1.0) * 0.5 * width;
    let screen_y = (1.0 - ndc_pos.y) * 0.5 * height;

    // Apply delta
    let target_screen_x = screen_x + dx;
    let target_screen_y = screen_y + dy; // dy in TS is likely pixel delta. screen_y is Y-down? 
    // Wait, get_joint_screen_positions did: screen_y = (1.0 - ndc_y) * ...
    // So screen Y is 0 at top? Yes (1.0 - 1.0 = 0).
    // Mouse event dy usually + is down.
    // So current + dy is correct for screen space.

    // Convert back to NDC
    // x = (screen / (0.5 * width)) - 1.0
    let target_ndc_x = (target_screen_x / (width * 0.5)) - 1.0;
    // y: screen = (1 - ndc) * h/2 => screen/(h/2) = 1 - ndc => ndc = 1 - screen/(h/2)
    let target_ndc_y = 1.0 - (target_screen_y / (height * 0.5));

    let target_ndc = glam::Vec3::new(target_ndc_x, target_ndc_y, ndc_pos.z);

    // Unproject
    // Inverse ViewProj * NDC_Clip
    let inverse_vp = view_proj.inverse();
    let clip_pos = glam::Vec4::new(target_ndc.x, target_ndc.y, target_ndc.z, 1.0);
    let mut world_pos = inverse_vp * clip_pos;
    if world_pos.w != 0.0 {
        world_pos /= world_pos.w;
    }

    glam::Vec3::new(world_pos.x, world_pos.y, world_pos.z)
}

/// Add a new keyframe as a copy of the previous one
#[wasm_bindgen]
pub fn add_keyframe_copy(after_index: usize) {
    GPU_STATE.with(|s| {
        let mut state_ref = s.borrow_mut();
        if let Some(state) = state_ref.as_mut() {
            if let Some(clip) = state.editor_clip.as_mut() {
                if after_index < clip.keyframes.len() {
                    let prev_keyframe = clip.keyframes[after_index].clone();
                    let new_time = if after_index + 1 < clip.keyframes.len() {
                        // Insert between two keyframes
                        (prev_keyframe.time + clip.keyframes[after_index + 1].time) / 2.0
                    } else {
                        // Add at end
                        prev_keyframe.time + 0.5
                    };

                    let mut new_keyframe = prev_keyframe;
                    new_keyframe.time = new_time;

                    clip.keyframes.insert(after_index + 1, new_keyframe);

                    // Update duration if needed
                    if let Some(last) = clip.keyframes.last() {
                        if last.time > clip.duration {
                            clip.duration = last.time + 0.5;
                        }
                    }

                    log::info!("Added keyframe at time {:.2}s", new_time);
                }
            }
        }
    });
}

/// Remove a keyframe by index
#[wasm_bindgen]
pub fn remove_keyframe(index: usize) {
    GPU_STATE.with(|s| {
        let mut state_ref = s.borrow_mut();
        if let Some(state) = state_ref.as_mut() {
            if let Some(clip) = state.editor_clip.as_mut() {
                // Don't remove if it's the last keyframe
                if clip.keyframes.len() > 1 && index < clip.keyframes.len() {
                    clip.keyframes.remove(index);
                    log::info!("Removed keyframe {}", index);
                }
            }
        }
    });
}

/// Export the current animation as JSON
#[wasm_bindgen]
pub fn export_animation_json() -> String {
    GPU_STATE.with(|s| {
        let state_ref = s.borrow();
        if let Some(state) = state_ref.as_ref() {
            if let Some(clip) = &state.editor_clip {
                match clip.to_json_string() {
                    Ok(json) => return json,
                    Err(e) => {
                        log::error!("Failed to export animation: {}", e);
                        return String::from("{}");
                    }
                }
            }
        }
        String::from("{}")
    })
}

/// Get information about a specific joint (rotation, position)
#[wasm_bindgen]
pub fn get_joint_info(bone_index: usize) -> String {
    GPU_STATE.with(|s| {
        let state_ref = s.borrow();
        if let Some(state) = state_ref.as_ref() {
            // Determine which pose to use (Editor or Animation)
            // For editor, we want the current modified pose.
            let pose = if state.editor_mode {
                if let Some(clip) = &state.editor_clip {
                    if let Some(kf) = clip.keyframes.get(state.editor_keyframe_index) {
                        Some(kf.pose.clone()) // Clone to access
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                // If not in editor mode (playing), sample current time
                if let Some(clip) = state.animations.get(&state.current_exercise_name) {
                    Some(clip.sample(state.uniforms.time))
                } else {
                    None
                }
            };

            if let Some(p) = pose {
                // Return JSON string
                if bone_index < crate::bone_hierarchy::BoneId::COUNT {
                    let id = unsafe { std::mem::transmute::<u8, crate::bone_hierarchy::BoneId>(bone_index as u8) };
                    let rot = p.local_rotations[bone_index];
                    let (x, y, z) = rot.to_euler(glam::EulerRot::XYZ);
                    
                    // We also want world position?
                    // pose.get_position(id) requires mut! but pose is clone so ok.
                    // But RotationPose methods require &mut self to compute lazy.
                    let mut p_mut = p;
                    let pos = p_mut.get_position(id);

                    return format!(
                        "{{ \"x\": {:.2}, \"y\": {:.2}, \"z\": {:.2}, \"rx\": {:.2}, \"ry\": {:.2}, \"rz\": {:.2} }}",
                        pos.x, pos.y, pos.z,
                        x.to_degrees(), y.to_degrees(), z.to_degrees()
                    );
                }
            }
        }
        String::from("{}")
    })
}

#[wasm_bindgen]
pub fn set_joint_rotation(bone_index: usize, rx: f32, ry: f32, rz: f32) {
    GPU_STATE.with(|s| {
        let mut state_ref = s.borrow_mut();
        if let Some(state) = state_ref.as_mut() {
            if let Some(clip) = state.editor_clip.as_mut() {
                if state.editor_keyframe_index < clip.keyframes.len() {
                    let keyframe = &mut clip.keyframes[state.editor_keyframe_index];
                    
                    if bone_index < crate::bone_hierarchy::BoneId::COUNT {
                        let id = unsafe { std::mem::transmute::<u8, crate::bone_hierarchy::BoneId>(bone_index as u8) };
                        // Convert degrees to radians and then to Quat
                        let q = glam::Quat::from_euler(
                            glam::EulerRot::XYZ,
                            rx.to_radians(),
                            ry.to_radians(),
                            rz.to_radians()
                        );
                        keyframe.pose.set_rotation(id, q);
                        keyframe.pose.mark_all_dirty();
                    }
                }
            }
        }
    });
}

#[wasm_bindgen]
pub fn set_joint_position_editor(bone_index: usize, x: f32, y: f32, z: f32) {
    use crate::bone_hierarchy::BoneId;
    GPU_STATE.with(|s| {
        let mut state_ref = s.borrow_mut();
        if let Some(state) = state_ref.as_mut() {
            if let Some(clip) = state.editor_clip.as_mut() {
                if state.editor_keyframe_index < clip.keyframes.len() {
                    let keyframe = &mut clip.keyframes[state.editor_keyframe_index];
                    let target_pos = glam::Vec3::new(x, y, z);

                    if bone_index == 0 {
                        keyframe.pose.set_root_position(target_pos);
                        keyframe.pose.apply_floor_constraint();
                        return;
                    }

                    // Reuse mapping logic (duplicated for now to avoid refactor complexity)
                    let (bone_id, chain) = match bone_index {
                        // IK Chains
                        6 => (BoneId::LeftForearm, vec![BoneId::LeftShoulder, BoneId::LeftUpperArm, BoneId::LeftForearm]),
                        9 => (BoneId::RightForearm, vec![BoneId::RightShoulder, BoneId::RightUpperArm, BoneId::RightForearm]),
                        12 => (BoneId::LeftShin, vec![BoneId::LeftThigh, BoneId::LeftShin]),
                        15 => (BoneId::RightShin, vec![BoneId::RightThigh, BoneId::RightShin]),
                        3 => (BoneId::Head, vec![BoneId::Spine, BoneId::Neck, BoneId::Head]),
                        1 => (BoneId::Spine, vec![BoneId::Spine]),
                        2 => (BoneId::Neck, vec![BoneId::Neck]),

                        // FK Bones
                        5 => (BoneId::LeftUpperArm, vec![]),
                        8 => (BoneId::RightUpperArm, vec![]),
                        4 => (BoneId::LeftShoulder, vec![]),
                        7 => (BoneId::RightShoulder, vec![]),
                        11 => (BoneId::LeftThigh, vec![]),
                        14 => (BoneId::RightThigh, vec![]),
                        _ => return, // Not modifiable by position
                    };

                    if !chain.is_empty() {
                         keyframe.pose.apply_ik(&chain, target_pos);
                         keyframe.pose.apply_floor_constraint();
                    } else {
                         // FK Logic (exact target)
                         // Identical to drag logic
                         let pivot_pos = if let Some(parent) = crate::bone_hierarchy::BONE_HIERARCHY[bone_id.index()].parent {
                              keyframe.pose.get_position(parent)
                         } else {
                              keyframe.pose.root_position
                         };

                         let target_dir = (target_pos - pivot_pos).normalize_or_zero();
                         if target_dir.length_squared() > 1e-6 {
                             // Re-calculate parent rotation
                             let parent_rot = if let Some(parent) = crate::bone_hierarchy::BONE_HIERARCHY[bone_id.index()].parent {
                                  let mut rot = keyframe.pose.local_rotations[parent.index()];
                                  let mut p = parent;
                                  while let Some(grandparent) = crate::bone_hierarchy::BONE_HIERARCHY[p.index()].parent {
                                      rot = keyframe.pose.local_rotations[grandparent.index()] * rot;
                                      p = grandparent;
                                  }
                                  rot
                             } else {
                                  glam::Quat::IDENTITY
                             };

                             let default_dir = crate::bone_hierarchy::BONE_HIERARCHY[bone_id.index()].direction.normalize();
                             let target_dir_local = parent_rot.inverse() * target_dir;
                             let delta_rot = glam::Quat::from_rotation_arc(default_dir, target_dir_local);
                             
                             keyframe.pose.set_rotation(bone_id, delta_rot.normalize());
                         }
                    }
                    keyframe.pose.apply_floor_constraint();
                    keyframe.pose.mark_all_dirty();
                }
            }
        }
    });
}
