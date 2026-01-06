#!/bin/bash

# Exit on error
set -e

# Source FBX files
PLACEHOLDER_FBX="fbx/placeholder.fbx"

echo "Syncing SMPL Skeleton and Animations..."

# 1. Update Bone IDs & Hierarchy in Rust (using the new FBX as reference)
echo "[1/3] Generating Bone Hierarchy (wasm/src/bone/id.rs)..."
export PYTHONPATH="$(pwd)/tools"
blender --background --python-use-system-env --python tools/extract_hierarchy.py -- "$PLACEHOLDER_FBX" -o wasm/src/bone/id.rs

# 2. Update Rest Pose Constants in Rust (using the new FBX as reference)
echo "[2/3] Generating Rest Pose Constants (wasm/src/skeleton_constants.rs)..."
blender --background --python-use-system-env --python tools/extract_rest_pose.py -- "$PLACEHOLDER_FBX" -o wasm/src/skeleton_constants.rs

# 3. Update Animation JSONs
echo "[3/3] Updating animations..."

# Master Placeholder - always generate this
echo "  [Master Placeholder] -> src/assets/animations/placeholder.anim"
blender --background --python-use-system-env --python tools/fbx_to_smpl.py -- "$PLACEHOLDER_FBX" --name "Placeholder" -o "src/assets/animations/placeholder.anim"

# Updates all exercises from workouts.json
EXERCISES=$(jq -r '.exercises[].name' src/assets/Workouts/jokkeri_ventti.json)

while IFS= read -r EX_NAME; do
    if [ -z "$EX_NAME" ]; then continue; fi

    # Create filename: "Ab Crunch" -> "ab_crunch"
    BASENAME=$(echo "$EX_NAME" | tr '[:upper:]' '[:lower:]' | tr ' ' '_' | tr '-' '_' | tr '/' '_' | tr -d '()')
    FBX_FILE="fbx/${BASENAME}.fbx"
    OUT_FILE="src/assets/animations/${BASENAME}.anim"

    if [ -f "$FBX_FILE" ]; then
        echo "  [$EX_NAME] -> $OUT_FILE (Using specific FBX)"
        blender --background --python-use-system-env --python tools/fbx_to_smpl.py -- "$FBX_FILE" --name "$EX_NAME" -o "$OUT_FILE"
    else
        echo "  [$EX_NAME] -> (Skipping, no matching FBX in $FBX_FILE)"
        # We don't generate a JSON here; Rust will fall back to placeholder.anim
        if [ -f "$OUT_FILE" ]; then
            rm "$OUT_FILE" # Clean up old placeholders
        fi
    fi
done <<< "$EXERCISES"

echo "Done! All skeleton data and animations updated."
echo "Syncing finished successfully."