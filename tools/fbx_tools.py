import os
import sys
from contextlib import contextmanager

@contextmanager
def suppress_stdout_stderr(enabled=True):
    """A context manager that redirects stdout and stderr to /dev/null if enabled."""
    if not enabled:
        yield
        return

    with open(os.devnull, 'w') as fnull:
        old_stdout_fd = os.dup(1)
        old_stderr_fd = os.dup(2)
        try:
            os.dup2(fnull.fileno(), 1)
            os.dup2(fnull.fileno(), 2)
            yield
        finally:
            os.dup2(old_stdout_fd, 1)
            os.dup2(old_stderr_fd, 2)
            os.close(old_stdout_fd)
            os.close(old_stderr_fd)

# SMPL Skeleton Configuration and Mappings
# (Blender Bone Name, Parent Bone Name)
HIERARCHY = [
    ("Pelvis", None),
    ("L_Hip", "Pelvis"),
    ("R_Hip", "Pelvis"),
    ("Spine1", "Pelvis"),
    ("L_Knee", "L_Hip"),
    ("R_Knee", "R_Hip"),
    ("Spine2", "Spine1"),
    ("L_Ankle", "L_Knee"),
    ("R_Ankle", "R_Knee"),
    ("Spine3", "Spine2"),
    ("L_Foot", "L_Ankle"),
    ("R_Foot", "R_Ankle"),
    ("Neck", "Spine3"),
    ("L_Collar", "Spine3"),
    ("R_Collar", "Spine3"),
    ("Head", "Neck"),
    ("L_Shoulder", "L_Collar"),
    ("R_Shoulder", "R_Collar"),
    ("L_Elbow", "L_Shoulder"),
    ("R_Elbow", "R_Shoulder"),
    ("L_Wrist", "L_Elbow"),
    ("R_Wrist", "R_Elbow"),
]

# Blender Name -> JSON property name (v2 schema)
FBX_TO_JSON = {
    "Pelvis": "p",
    "Spine1": "s1",
    "Spine2": "s2",
    "Spine3": "s3",
    "Neck": "n",
    "Head": "h",
    "L_Collar": "lc",
    "R_Collar": "rc",
    "L_Shoulder": "ls",
    "R_Shoulder": "rs",
    "L_Elbow": "le",
    "R_Elbow": "re",
    "L_Wrist": "lw",
    "R_Wrist": "rw",
    "L_Hip": "lh",
    "R_Hip": "rh",
    "L_Knee": "lk",
    "R_Knee": "rk",
    "L_Ankle": "la",
    "R_Ankle": "ra",
    "L_Foot": "lf",
    "R_Foot": "rf"
}

# Blender Name -> Rust Constant Name (skeleton_constants.rs)
FBX_TO_RUST_CONST = {
    "Pelvis": "DEFAULT_PELVIS",
    "L_Hip": "DEFAULT_LEFT_HIP",
    "R_Hip": "DEFAULT_RIGHT_HIP",
    "Spine1": "DEFAULT_SPINE1",
    "L_Knee": "DEFAULT_LEFT_KNEE",
    "R_Knee": "DEFAULT_RIGHT_KNEE",
    "Spine2": "DEFAULT_SPINE2",
    "L_Ankle": "DEFAULT_LEFT_ANKLE",
    "R_Ankle": "DEFAULT_RIGHT_ANKLE",
    "Spine3": "DEFAULT_SPINE3",
    "L_Foot": "DEFAULT_LEFT_FOOT",
    "R_Foot": "DEFAULT_RIGHT_FOOT",
    "Neck": "DEFAULT_NECK",
    "L_Collar": "DEFAULT_LEFT_COLLAR",
    "R_Collar": "DEFAULT_RIGHT_COLLAR",
    "Head": "DEFAULT_HEAD",
    "L_Shoulder": "DEFAULT_LEFT_SHOULDER",
    "R_Shoulder": "DEFAULT_RIGHT_SHOULDER",
    "L_Elbow": "DEFAULT_LEFT_ELBOW",
    "R_Elbow": "DEFAULT_RIGHT_ELBOW",
    "L_Wrist": "DEFAULT_LEFT_WRIST",
    "R_Wrist": "DEFAULT_RIGHT_WRIST",
}

def get_rust_enum_name(name):
    """Map Blender Name to Rust BoneId Enum Variant (PascalCase)"""
    mapping = {
        "Pelvis": "Pelvis",
        "L_Hip": "LeftHip",
        "R_Hip": "RightHip",
        "Spine1": "Spine1",
        "L_Knee": "LeftKnee",
        "R_Knee": "RightKnee",
        "Spine2": "Spine2",
        "L_Ankle": "LeftAnkle",
        "R_Ankle": "RightAnkle",
        "Spine3": "Spine3",
        "L_Foot": "LeftFoot",
        "R_Foot": "RightFoot",
        "Neck": "Neck",
        "L_Collar": "LeftCollar",
        "R_Collar": "RightCollar",
        "Head": "Head",
        "L_Shoulder": "LeftShoulder",
        "R_Shoulder": "RightShoulder",
        "L_Elbow": "LeftElbow",
        "R_Elbow": "RightElbow",
        "L_Wrist": "LeftWrist",
        "R_Wrist": "RightWrist",
    }
    return mapping.get(name, name)
