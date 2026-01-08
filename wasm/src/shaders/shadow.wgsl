// Drop shadow shader for stickman figure
// Renders skeleton projected onto floor plane (Y=0) as a dark translucent shadow

// Matches Rust Uniforms struct layout (160 bytes total)
struct Uniforms {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    aspect: f32,
    screen_height: f32,
    _padding: vec2<f32>,
    _padding4: vec4<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var<uniform> bone_matrices: array<mat4x4<f32>, 22>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) bone_index: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) alpha: f32,
}

// Light direction for shadow projection
const LIGHT_DIR: vec3<f32> = vec3<f32>(0.5, 0.8, 0.4);

// Shadow projection matrix: projects onto Y=0 plane
fn shadow_projection_matrix() -> mat4x4<f32> {
    // Planar shadow projection
    // Projects points onto Y=0 based on light direction
    let l = normalize(LIGHT_DIR);

    // Standard planar shadow matrix for Y=0 plane with directional light
    // M = I - (n * l^T) / (n . l) where n = (0,1,0)
    // Simplified for Y=0: we just need to project Y to 0 based on XZ offset
    let d = l.y;  // dot(normal, light) where normal = (0,1,0)

    return mat4x4<f32>(
        vec4<f32>(1.0, 0.0, 0.0, 0.0),
        vec4<f32>(-l.x/d, 0.0, -l.z/d, 0.0),
        vec4<f32>(0.0, 0.0, 1.0, 0.0),
        vec4<f32>(0.0, 0.01, 0.0, 1.0)  // Tiny Y offset to prevent z-fighting
    );
}

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let bone_matrix = bone_matrices[vertex.bone_index];
    let shadow_matrix = shadow_projection_matrix();

    // Transform by bone, then project to floor
    let world_pos = bone_matrix * vec4<f32>(vertex.position, 1.0);
    let shadow_pos = shadow_matrix * world_pos;

    out.clip_position = uniforms.projection * uniforms.view * shadow_pos;

    // Fade shadow based on distance from character center
    // Shadows fade out at edges for softer look
    let dist_from_center = length(shadow_pos.xz);
    out.alpha = 1.0 - smoothstep(0.0, 1.5, dist_from_center);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Dark shadow with soft edges
    let shadow_color = vec3<f32>(0.0, 0.0, 0.0);
    let shadow_alpha = 0.25 * in.alpha;  // Subtle shadow

    return vec4<f32>(shadow_color, shadow_alpha);
}
