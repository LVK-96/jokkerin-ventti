import sys
import argparse
from pathlib import Path

import bpy
import mathutils

from fbx_tools import FBX_TO_RUST_CONST, suppress_stdout_stderr

def main():
    argv = sys.argv
    if "--" in argv:
        argv = argv[argv.index("--") + 1:]
    else:
        argv = []

    parser = argparse.ArgumentParser(description="Extract rest pose constants from FBX to Rust.")
    parser.add_argument("input", help="Input FBX file")
    parser.add_argument("-o", "--output", required=True, help="Output Rust file")
    parser.add_argument("-v", "--verbose", action="store_true", help="Enable verbose Blender output")
    args = parser.parse_args(argv)

    input_path = Path(args.input).resolve()
    output_path = Path(args.output).resolve()

    # Clear and import - Standard defaults
    bpy.ops.wm.read_factory_settings(use_empty=True)
    try:
        with suppress_stdout_stderr(enabled=not args.verbose):
            bpy.ops.import_scene.fbx(
                filepath=str(input_path),
                use_manual_orientation=False,
            )
    except RuntimeError as e:
        print(f"Error importing FBX: {e}")
        sys.exit(1)

    # Find armature
    armature = None
    for obj in bpy.data.objects:
        if obj.type == 'ARMATURE':
            armature = obj
            break

    if armature is None:
        print("No armature found in FBX")
        sys.exit(1)

    # Output to file
    with open(output_path, "w") as f:
        f.write(f"// Generated from {input_path.name}\n")
        f.write(f"// Base mapping: Blender Z (Up) -> Game Y (Up), Blender Y (Forward) -> Game -Z (Back)\n")
        f.write("use glam::Vec3A;\n\n")

        for bone_name, const_name in FBX_TO_RUST_CONST.items():
            bone = armature.data.bones.get(bone_name)
            if not bone:
                continue

            head = armature.matrix_world @ bone.head_local

            # Mapping: Blender Z-up -> Game Y-up
            # Game X = Blender X
            # Game Y = Blender Z (Height)
            # Game Z = -Blender Y (Depth/Forward is -Z)
            gx = head.x
            gy = head.z
            gz = -head.y

            # Print Rust constant
            f.write(f"pub const {const_name}: Vec3A = Vec3A::new({gx:.3f}, {gy:.3f}, {gz:.3f});\n")

        print(f"Success: {output_path}")

if __name__ == "__main__":
    main()
