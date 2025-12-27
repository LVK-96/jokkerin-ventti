// Shader for 3D stickman figure with skeletal animation (skinning)

struct Uniforms {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    time: f32,
    aspect: f32,
    screen_height: f32,
    _padding: f32,
    _padding4: vec4<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

// Bone matrices
// 29 matrices: 13 cylinders + 1 head + 15 debug spheres
@group(1) @binding(0) var<uniform> bone_matrices: array<mat4x4<f32>, 29>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) bone_index: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) world_pos: vec3<f32>,
    @location(2) bone_index: f32, // Passed to fragment for coloring debug joints
}

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    let bone_matrix = bone_matrices[vertex.bone_index];
    
    // Transform position and normal by the bone matrix
    let world_pos = bone_matrix * vec4<f32>(vertex.position, 1.0);
    let world_normal = bone_matrix * vec4<f32>(vertex.normal, 0.0);
    
    out.clip_position = uniforms.projection * uniforms.view * world_pos;
    out.world_pos = world_pos.xyz;
    out.world_normal = normalize(world_normal.xyz);
    out.bone_index = f32(vertex.bone_index);
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Light setup
    let light_dir = normalize(vec3<f32>(0.5, 0.8, 0.5));
    let normal = normalize(in.world_normal);
    
    let ndotl = max(dot(normal, light_dir), 0.0);
    let ambient = 0.3;
    let diffuse = ndotl * 0.7;
    
    // Color logic
    // Bone indices 14-28 are debug spheres (pink)
    if in.bone_index >= 14.0 {
        return vec4<f32>(1.0, 0.0, 1.0, 1.0); // Pink for debug joints
    }
    
    // Normal geometry: dark gray
    let base_color = vec3<f32>(0.1, 0.1, 0.1);
    let lit_color = base_color * (ambient + diffuse);
    
    return vec4<f32>(lit_color, 1.0);
}
