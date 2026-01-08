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
// 44 matrices for SMPL: 22 bone transforms + 22 debug joint spheres
@group(1) @binding(0) var<uniform> bone_matrices: array<mat4x4<f32>, 44>;

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

    // === Three-Point Lighting Setup ===
    // Key light: Main light source (warm, from front-right-top)
    let key_light_dir = normalize(vec3<f32>(0.5, 0.8, 0.4));
    let key_light_color = vec3<f32>(1.0, 0.95, 0.9);  // Slightly warm

    // Fill light: Soft fill (cool, from front-left)
    let fill_light_dir = normalize(vec3<f32>(-0.4, 0.3, 0.5));
    let fill_light_color = vec3<f32>(0.7, 0.8, 1.0);  // Cool blue tint

    // Rim/Back light: Edge highlight (from behind)
    let rim_light_dir = normalize(vec3<f32>(-0.2, 0.4, -0.8));
    let rim_light_color = vec3<f32>(1.0, 1.0, 1.0);  // Pure white

    // === Wrap Diffuse (for softer terminator) ===
    let wrap = 0.3;

    // Key light contribution (strongest)
    let key_ndotl = dot(normal, key_light_dir);
    let key_wrap = (key_ndotl + wrap) / (1.0 + wrap);
    let key_diffuse = max(key_wrap, 0.0) * 0.7;

    // Fill light contribution (softer)
    let fill_ndotl = max(dot(normal, fill_light_dir), 0.0);
    let fill_diffuse = fill_ndotl * 0.25;

    // === Subsurface Scattering Approximation ===
    // Light passing through thin areas
    let transmittance_power = 3.0;
    let vdotl = dot(view_dir, -key_light_dir);
    let transmittance = pow(saturate(vdotl), transmittance_power) * 0.5;

    // Thickness from curvature (edges = thin)
    let ndotv = max(dot(normal, view_dir), 0.0);
    let thickness = pow(ndotv, 0.8);

    // SSS color (warm internal glow)
    let sss_color = vec3<f32>(0.02, 0.006, 0.003);
    let sss_intensity = transmittance * (1.0 - thickness) * 0.8;

    // === Rim Lighting (Fresnel-based + back light) ===
    let fresnel = pow(1.0 - ndotv, 3.0);
    let back_light = max(dot(normal, rim_light_dir), 0.0);
    let rim_fresnel = fresnel * 0.35;
    let rim_back = back_light * fresnel * 0.2;

    // === Specular Highlight (Blinn-Phong on key light) ===
    let half_vec = normalize(key_light_dir + view_dir);
    let ndoth = max(dot(normal, half_vec), 0.0);
    let specular = pow(ndoth, 80.0) * 0.6;

    // === Base Color ===
    // Very dark with subtle blue tint (linear space)
    let base_color = vec3<f32>(0.0003, 0.0003, 0.0006);

    // === Final Composition ===
    var lit_color = base_color;

    // Key light (warm, dominant)
    lit_color += base_color * key_diffuse * key_light_color;

    // Fill light (cool, subtle)
    lit_color += base_color * fill_diffuse * fill_light_color;

    // SSS glow (warm internal scattering)
    lit_color += sss_color * sss_intensity;

    // Rim lighting (edge definition)
    let rim_color = vec3<f32>(0.012, 0.02, 0.055);  // Cool blue edge
    lit_color += rim_color * rim_fresnel;
    lit_color += rim_light_color * rim_back * 0.1;

    // Specular highlight
    lit_color += vec3<f32>(0.8, 0.85, 1.0) * specular;

    // Tone mapping (Reinhard-style)
    lit_color = lit_color / (lit_color + vec3<f32>(0.3));

    // Gamma correction (linear to sRGB)
    let gamma = 1.0 / 2.2;
    let gamma_corrected = pow(lit_color, vec3<f32>(gamma));

    return vec4<f32>(gamma_corrected, 1.0);
}

