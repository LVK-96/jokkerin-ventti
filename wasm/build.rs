//! Build script for animation validation (V2 only)

use serde::Deserialize;
use std::fs;
use std::path::Path;

// Rotation-based structs (V2)
// minimal struct to verify it is an Object, not an Array (Legacy)
#[derive(Debug, Deserialize)]
struct RotationPoseCheck {}

#[derive(Debug, Deserialize)]
struct RotationKeyframeCheck {
    #[allow(dead_code)]
    time: f32,
    #[allow(dead_code)]
    pose: RotationPoseCheck,
}

#[derive(Debug, Deserialize)]
struct RotationAnimationClipCheck {
    #[allow(dead_code)]
    name: String,
    #[serde(default)]
    version: u32,
    #[allow(dead_code)]
    keyframes: Vec<RotationKeyframeCheck>,
}

fn validate_animation_file(path: &Path) -> Result<(), String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;

    // Validate V2 format
    let clip: RotationAnimationClipCheck = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse as V2/Rotation format: {}", e))?;

    if clip.version != 2 {
        return Err(format!("Invalid version: expected 2, got {}", clip.version));
    }

    Ok(())
}

fn main() {
    let animation_dir = Path::new("../src/assets/animations");
    if !animation_dir.exists() {
        println!("cargo:warning=Animation directory not found");
        return;
    }

    // Rerun if the animations directory changes
    println!("cargo:rerun-if-changed={}", animation_dir.display());

    let mut has_errors = false;

    if let Ok(entries) = fs::read_dir(animation_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                // Tell cargo to rerun if this file changes
                println!("cargo:rerun-if-changed={}", path.display());

                match validate_animation_file(&path) {
                    Ok(_) => {
                        // Success
                    }
                    Err(e) => {
                        println!(
                            "cargo:warning=VALIDATION ERROR in {:?}: {}",
                            path.file_name().unwrap(),
                            e
                        );
                        has_errors = true;
                    }
                }
            }
        }
    }

    if has_errors {
        panic!(
            "Animation validation failed found invalid/legacy files! Please convert them to V2."
        );
    }
}
