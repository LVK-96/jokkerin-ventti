// Shader for 3D stickman figure with skeletal animation (skinning)
// Enhanced with Blinn-Phong specular, Fresnel rim lighting, and improved colors

// Matches Rust Uniforms struct layout (160 bytes total)
struct Uniforms {
    view: mat4x4<f32>,          // bytes 0-63
    projection: mat4x4<f32>,    // bytes 64-127
    aspect: f32,                // byte 132
    screen_height: f32,         // byte 136
    _padding: vec2<f32>,        // byte 140-147
    _padding4: vec4<f32>,       // bytes 148-159
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

// Camera position (constant - matches the position in gpu.rs)
const CAMERA_POS: vec3<f32> = vec3<f32>(2.5, 1.2, 3.0);


// Bone matrices
// 29 matrices: 13 cylinders + 1 head + 15 debug spheres = 29
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
    @location(2) bone_index: f32,
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
    let normal = normalize(in.world_normal);
    let view_dir = normalize(CAMERA_POS - in.world_pos);
    
    // === Light Setup ===
    let light_dir = normalize(vec3<f32>(0.4, 0.7, 0.5));
    let back_light_dir = -light_dir; // For transmittance
    
    // ============================================
    // SUBSURFACE SCATTERING APPROXIMATION
    // ============================================
    // Reference: GPU Gems 3, "Real-Time Approximations to Subsurface Scattering"
    
    // --- Wrap Diffuse ---
    // Softens the terminator (light/dark boundary) like real skin/wax
    let wrap = 0.5;  // How much light wraps around the surface
    let ndotl_raw = dot(normal, light_dir);
    let ndotl_wrap = (ndotl_raw + wrap) / (1.0 + wrap);
    let diffuse = max(ndotl_wrap, 0.0);
    
    // --- Transmittance (Light passing through) ---
    // Simulates light entering the back and scattering through
    let transmittance_power = 3.0;
    let transmittance_scale = 0.6;
    let vdotl = dot(view_dir, -light_dir);  // View aligned with light = more transmission
    let transmittance = pow(saturate(vdotl), transmittance_power) * transmittance_scale;
    
    // --- Thickness approximation from curvature ---
    // Convex surfaces (spheres) appear thinner at edges
    let ndotv = max(dot(normal, view_dir), 0.0);
    let thickness = pow(ndotv, 0.8);  // Center = thick, edges = thin
    
    // SSS color - light that scatters takes on material color
    // Using a subtle warm/red tint like subsurface blood or internal glow
    let sss_color = vec3<f32>(0.15, 0.08, 0.05);  // Deep warm glow
    let sss_intensity = transmittance * (1.0 - thickness) * 0.8;
    
    // --- Fresnel Rim (edge glow) ---
    let fresnel = pow(1.0 - ndotv, 3.0);
    let rim_color = vec3<f32>(0.12, 0.15, 0.25);  // Cool blue edge
    let rim = fresnel * 0.4;
    
    // --- Specular (tight highlight) ---
    let half_vec = normalize(light_dir + view_dir);
    let ndoth = max(dot(normal, half_vec), 0.0);
    let specular = pow(ndoth, 80.0) * 0.7;
    
    // === Color Palette ===
    // Debug joints (indices 14-27): pink/magenta
    // Debug joints (indices 14-27): pink/magenta
    // Head (13): treated as normal bone
    if (in.bone_index >= 14.0 && in.bone_index <= 28.0) {
        return vec4<f32>(1.0, 0.0, 1.0, 1.0);
    }
    
    // Base color: very dark (nearly black with subtle blue)
    let base_color = vec3<f32>(0.02, 0.02, 0.04);
    
    // === Final Composition ===
    var lit_color = base_color;
    
    // Diffuse contribution (subtle, keeps it dark)
    lit_color += base_color * diffuse * 0.3;
    
    // SSS glow (the magic - internal light scattering)
    lit_color += sss_color * sss_intensity;
    
    // Rim light (Fresnel edge)
    lit_color += rim_color * rim;
    
    // Specular highlight
    lit_color += vec3<f32>(0.8, 0.85, 1.0) * specular;
    
    // Tone mapping (preserve contrast)
    lit_color = lit_color / (lit_color + vec3<f32>(0.3));
    
    return vec4<f32>(lit_color, 1.0);
}
