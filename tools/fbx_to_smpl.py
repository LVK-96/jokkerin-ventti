import bpy
import json
import math
import sys
import argparse
import mathutils
from pathlib import Path

from fbx_tools import HIERARCHY, FBX_TO_JSON, suppress_stdout_stderr

def process_fbx(input_path, anim_name, step=1, verbose=False):
    # Clear and import
    bpy.ops.wm.read_factory_settings(use_empty=True)
    try:
        with suppress_stdout_stderr(enabled=not verbose):
            bpy.ops.import_scene.fbx(filepath=str(input_path), use_manual_orientation=False)
    except RuntimeError as e:
        print(f"Error importing FBX: {e}")
        sys.exit(1)

    armature = None
    for obj in bpy.data.objects:
        if obj.type == 'ARMATURE':
            armature = obj
            break

    if not armature:
        print("No armature found in FBX")
        return None

    # Get animation action
    if not armature.animation_data or not armature.animation_data.action:
        print("No animation data found")
        return None

    action = armature.animation_data.action
    frame_start = int(action.frame_range[0])
    frame_end = int(action.frame_range[1])
    fps = bpy.context.scene.render.fps
    duration = (frame_end - frame_start) / fps

    print(f"Processing animation: Frames {frame_start}-{frame_end} (step {step}) @ {fps} FPS, Duration: {duration:.2f}s")

    # --- 1. Compute Rest Pose Rotations (Local Game Space) ---
    # Store World Quats and Positions for Delta calculation
    rest_world_quats = {}
    rest_world_pos = {}

    # Track min Z in rest pose to normalize heights
    min_rest_z = float('inf')

    for bone_name, parent_name in HIERARCHY:
        bone = armature.data.bones.get(bone_name)
        if not bone: continue

        # Blender Rest Global Matrix
        mat_world = armature.matrix_world @ bone.matrix_local
        pos = mat_world.to_translation()
        min_rest_z = min(min_rest_z, pos.z)

        quat_world = mat_world.to_quaternion()

        # Swizzle to Game Space
        # q = (w, x, y, z) -> (w, x, z, -y)
        q_game_world = mathutils.Quaternion((quat_world.w, quat_world.x, quat_world.z, -quat_world.y))

        rest_world_quats[bone_name] = q_game_world
        rest_world_pos[bone_name] = pos

    # Height from floor to pelvis in rest pose
    grounded_rel_pelvis_z = rest_world_pos["Pelvis"].z - min_rest_z

    # =========================================================================
    # PASS 1: Collect all quaternions for all frames
    # =========================================================================
    print("Pass 1: Collecting quaternions...")

    frame_quats = []
    frame_root_positions = []
    frame_times = []

    for frame in range(frame_start, frame_end + 1, step):
        bpy.context.scene.frame_set(frame)
        time = (frame - frame_start) / fps
        frame_times.append(time)

        delta_world_quats = {}
        bone_local_quats = {}

        for bone_name, parent_name in HIERARCHY:
            pose_bone = armature.pose.bones.get(bone_name)
            if not pose_bone: continue

            # --- A. Step 1: Compute Anim World Quat (Swizzled) ---
            mat_world = armature.matrix_world @ pose_bone.matrix
            quat_world = mat_world.to_quaternion()
            q_anim_world = mathutils.Quaternion((quat_world.w, quat_world.x, quat_world.z, -quat_world.y))

            # --- B. Step 2: Compute World Delta (Anim vs Rest) ---
            if bone_name in rest_world_quats:
                 q_rest_world = rest_world_quats[bone_name]
                 q_delta_world = q_anim_world @ q_rest_world.conjugated()
            else:
                 q_delta_world = q_anim_world

            delta_world_quats[bone_name] = q_delta_world

            # --- C. Step 3: Compute Engine Local Quat ---
            if parent_name and parent_name in delta_world_quats:
                q_parent_delta = delta_world_quats[parent_name]
                q_local = q_parent_delta.conjugated() @ q_delta_world
            else:
                q_local = q_delta_world

            bone_local_quats[bone_name] = q_local

            # Handle Root Position (Pelvis)
            if bone_name == "Pelvis":
                world_pos = mat_world.to_translation()

                # rel_y = 0 means "standing on ground" in the engine
                # world_pos.z is absolute Blender Z.
                # grounded_rel_pelvis_z is the height of the pelvis above the floor in the rest pose.
                # subtracting them gives the height delta relative to a grounded standing pose.
                rel_x = world_pos.x - rest_world_pos[bone_name].x
                rel_y = world_pos.z - grounded_rel_pelvis_z
                rel_z = -(world_pos.y - rest_world_pos[bone_name].y)

                frame_root_positions.append([
                    round(rel_x, 4),
                    round(rel_y, 4),
                    round(rel_z, 4)
                ])

        frame_quats.append(bone_local_quats)

    # =========================================================================
    # PASS 2: Normalize quaternion signs across frames
    # =========================================================================
    print("Pass 2: Normalizing quaternion signs...")

    for frame_idx in range(1, len(frame_quats)):
        for bone_name in frame_quats[frame_idx]:
            if bone_name not in frame_quats[frame_idx - 1]:
                continue
            prev_q = frame_quats[frame_idx - 1][bone_name]
            curr_q = frame_quats[frame_idx][bone_name]
            dot = prev_q.w * curr_q.w + prev_q.x * curr_q.x + prev_q.y * curr_q.y + prev_q.z * curr_q.z
            if dot < 0:
                frame_quats[frame_idx][bone_name] = mathutils.Quaternion((-curr_q.w, -curr_q.x, -curr_q.y, -curr_q.z))

    # =========================================================================
    # PASS 3: Build keyframes
    # =========================================================================
    print("Pass 3: Converting to Euler angles...")

    keyframes = []

    for frame_idx in range(len(frame_quats)):
        pose_data = {}
        # Omit initializing all bones to None, just add what we need
        # pose_data["root_position"] = None

        for bone_name in frame_quats[frame_idx]:
            json_bone = FBX_TO_JSON.get(bone_name)
            if not json_bone: continue
            q = frame_quats[frame_idx][bone_name]

            # Optimization: Omit identity rotations (default pose)
            # w > 0.9999 means virtually no rotation
            if q.w > 0.9999:
                continue

            pose_data[json_bone] = {
                "w": round(q.w, 4),
                "x": round(q.x, 4),
                "y": round(q.y, 4),
                "z": round(q.z, 4)
            }

        if frame_idx < len(frame_root_positions):
            rp = frame_root_positions[frame_idx]
            # Optimization: Omit root position if it's virtually zero
            if abs(rp[0]) > 0.001 or abs(rp[1]) > 0.001 or abs(rp[2]) > 0.001:
                pose_data["rp"] = [round(rp[0], 3), round(rp[1], 3), round(rp[2], 3)]

        keyframes.append({
            "t": round(frame_times[frame_idx], 3),
            "p": pose_data
        })

    return keyframes

def main():
    argv = sys.argv
    if "--" in argv:
        argv = argv[argv.index("--") + 1:]
    else:
        argv = []

    parser = argparse.ArgumentParser()
    parser.add_argument("input", help="Input FBX file")
    parser.add_argument("-o", "--output", help="Output JSON file")
    parser.add_argument("--name", help="Animation name")
    parser.add_argument("--step", type=int, default=2, help="Frame step (default 2, e.g. 15fps from 30fps source)")
    parser.add_argument("-v", "--verbose", action="store_true", help="Enable verbose Blender output")
    args = parser.parse_args(argv)

    input_path = Path(args.input).resolve()
    keyframes = process_fbx(input_path, args.name, step=args.step, verbose=args.verbose)

    if keyframes:
        output_data = {
            "v": 2,
            "n": args.name if args.name else input_path.stem,
            "d": keyframes[-1]["t"],
            "kf": keyframes
        }

        output_path = Path(args.output).resolve() if args.output else input_path.with_suffix('.json')
        with open(output_path, "w") as f:
            json.dump(output_data, f, indent=None, separators=(',', ':'))

        print(f"Success: {output_path}")

if __name__ == "__main__":
    main()
