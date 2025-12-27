//! Build script for animation keyframe validation
//!
//! This script runs at compile time and validates all animation JSON files
//! to ensure bone lengths are consistent across all keyframes.

// Include the shared skeleton constants
#[path = "src/skeleton_constants.rs"]
mod skeleton_constants;

use glam::Vec3;
use serde::Deserialize;
use skeleton_constants::BoneLengths;
use std::fs;
use std::path::Path;

/// Skeleton pose from JSON
#[derive(Debug, Deserialize)]
struct Pose {
    hips: Vec3,
    neck: Vec3,
    head: Vec3,
    left_shoulder: Vec3,
    left_elbow: Vec3,
    left_hand: Vec3,
    right_shoulder: Vec3,
    right_elbow: Vec3,
    right_hand: Vec3,
    left_hip: Vec3,
    left_knee: Vec3,
    left_foot: Vec3,
    right_hip: Vec3,
    right_knee: Vec3,
    right_foot: Vec3,
}

#[derive(Debug, Deserialize)]
struct Keyframe {
    time: f32,
    pose: Pose,
}

#[derive(Debug, Deserialize)]
struct AnimationClip {
    name: String,
    #[allow(dead_code)]
    duration: f32,
    keyframes: Vec<Keyframe>,
}

/// Validate a single pose against expected bone lengths
fn validate_pose(pose: &Pose, expected: &BoneLengths, tolerance: f32) -> Vec<String> {
    let mut errors = Vec::new();

    // Helper to check a bone length
    let check = |errors: &mut Vec<String>, name: &str, a: Vec3, b: Vec3, expected_len: f32| {
        let actual = a.distance(b);
        let diff = (actual - expected_len).abs();
        if diff > tolerance {
            errors.push(format!(
                "  {} length: expected {:.3}m, got {:.3}m (diff: {:.3}m)",
                name, expected_len, actual, diff
            ));
        }
    };

    // Spine (hips→neck)
    check(
        &mut errors,
        "Spine (hips→neck)",
        pose.hips,
        pose.neck,
        expected.spine,
    );
    check(
        &mut errors,
        "Head-neck",
        pose.neck,
        pose.head,
        expected.head_neck,
    );

    // Arms (check both sides: upper arm and forearm)
    check(
        &mut errors,
        "Left clavicle",
        pose.neck,
        pose.left_shoulder,
        expected.clavicle,
    );
    check(
        &mut errors,
        "Right clavicle",
        pose.neck,
        pose.right_shoulder,
        expected.clavicle,
    );
    check(
        &mut errors,
        "Left upper arm",
        pose.left_shoulder,
        pose.left_elbow,
        expected.upper_arm,
    );
    check(
        &mut errors,
        "Right upper arm",
        pose.right_shoulder,
        pose.right_elbow,
        expected.upper_arm,
    );
    check(
        &mut errors,
        "Left forearm",
        pose.left_elbow,
        pose.left_hand,
        expected.forearm,
    );
    check(
        &mut errors,
        "Right forearm",
        pose.right_elbow,
        pose.right_hand,
        expected.forearm,
    );

    // Legs (check both sides)
    check(
        &mut errors,
        "Left pelvis",
        pose.hips,
        pose.left_hip,
        expected.pelvis,
    );
    check(
        &mut errors,
        "Right pelvis",
        pose.hips,
        pose.right_hip,
        expected.pelvis,
    );
    check(
        &mut errors,
        "Left thigh",
        pose.left_hip,
        pose.left_knee,
        expected.thigh,
    );
    check(
        &mut errors,
        "Right thigh",
        pose.right_hip,
        pose.right_knee,
        expected.thigh,
    );
    check(
        &mut errors,
        "Left shin",
        pose.left_knee,
        pose.left_foot,
        expected.shin,
    );
    check(
        &mut errors,
        "Right shin",
        pose.right_knee,
        pose.right_foot,
        expected.shin,
    );

    errors
}

/// Validate an animation file
fn validate_animation_file(
    path: &Path,
    expected: &BoneLengths,
    tolerance: f32,
) -> Result<(), String> {
    let contents = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

    let clip: AnimationClip = serde_json::from_str(&contents)
        .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))?;

    let mut all_errors = Vec::new();

    for (i, keyframe) in clip.keyframes.iter().enumerate() {
        let errors = validate_pose(&keyframe.pose, expected, tolerance);
        if !errors.is_empty() {
            all_errors.push(format!(
                "Keyframe {} (t={:.2}s):\n{}",
                i,
                keyframe.time,
                errors.join("\n")
            ));
        }
    }

    if all_errors.is_empty() {
        println!(
            "cargo:warning=✓ {} validated ({} keyframes)",
            clip.name,
            clip.keyframes.len()
        );
        Ok(())
    } else {
        Err(format!(
            "Animation '{}' has invalid bone lengths:\n{}",
            clip.name,
            all_errors.join("\n\n")
        ))
    }
}

fn main() {
    // Bone length tolerance (5% of the bone length, minimum 0.02m)
    const TOLERANCE: f32 = 0.05;

    // Use shared bone lengths from skeleton_constants
    let expected = BoneLengths::from_default();

    // Animation files to validate (relative to wasm crate root)
    let animation_dir = Path::new("../src/assets/animations");

    if !animation_dir.exists() {
        println!("cargo:warning=Animation directory not found, skipping validation");
        return;
    }

    // Rerun if shared constants change
    println!("cargo:rerun-if-changed=src/skeleton_constants.rs");

    let mut has_errors = false;

    // Find all JSON files in the animations directory
    if let Ok(entries) = fs::read_dir(animation_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                // Tell cargo to rerun if this file changes
                println!("cargo:rerun-if-changed={}", path.display());

                if let Err(e) = validate_animation_file(&path, &expected, TOLERANCE) {
                    println!("cargo:warning=VALIDATION ERROR: {}", e);
                    has_errors = true;
                }
            }
        }
    }

    if has_errors {
        panic!("Animation validation failed! Fix the bone lengths in the keyframe files.");
    }

    // Rerun if the animations directory changes
    println!("cargo:rerun-if-changed={}", animation_dir.display());
}
