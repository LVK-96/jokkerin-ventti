// Shader for the stickman figure
// Expands line segments into quads with rounded caps

struct Uniforms {
    view: mat4x4<f32>, // Camera postition & rotation
    projection: mat4x4<f32>, // Camera projection 3D -> 2D
    time: f32, // Animation time
    aspect: f32, // Width / height
    screen_height: f32, // Screen height in pixels

    // Padding to 16-byte aligned blocks
    _padding: f32, 
    _padding4: vec4<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @builtin(vertex_index) vertex_idx: u32, // Set by WebGPU API
    @location(0) start_pos: vec3<f32>, // Bone start vertex
    @location(1) end_pos: vec3<f32>, // Bone end vertex
    @location(2) bone_id: f32, // Unique ID for each bone
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>, // Screen position

    // Distance field coordinates:
    //  - x: Lateral offset from the center line of the bone [-1, 1] (normalized by thickness/2 i.e. radius of the bone)
    //  - y: Longitudinal position along the bone, where 0 is the start and segment_len is the end. Values < 0 or > segment_len create the rounded caps
    // For the head: simply the distance from the center
    @location(0) sdf_coordinates: vec2<f32>, 

    // Is this a head (point segment)
    @location(1) @interpolate(flat) is_head: u32,
    @location(2) @interpolate(flat) segment_len: f32, // Length of the segment
}

// Constants
const EPSILON: f32 = 1e-6;
const BONE_RADIUS: f32 = 0.04;
const HEAD_RADIUS: f32 = 0.12;

// "Constant" injected from the host
override JOINT_HEAD: f32;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // Check if it's a head using the injected constant
    let is_head = in.bone_id == JOINT_HEAD;
    out.is_head = u32(is_head);

    let start_clip = uniforms.projection * uniforms.view * vec4<f32>(in.start_pos, 1.0);
    let end_clip = uniforms.projection * uniforms.view * vec4<f32>(in.end_pos, 1.0);

    // The bones and head are quads (2 triangles, 6 indices)
    // Mapping vertex -> coner: [0, 1, 2, 0, 2, 3]
    // 
    //             is_right=0  is_right=1
    //       is_end=0  0 ------ 1
    //                 | \      |
    //                 |  \     |
    //                 |   \    |
    //                 |    \   |
    //                 |     \  |
    //                 |      \ |
    //       is_end=1  3 ------ 2
    //
    let corner_idx = array<u32, 6>(0u, 1u, 2u, 0u, 2u, 3u)[in.vertex_idx];
    let is_end = f32(corner_idx == 2u || corner_idx == 3u);
    let is_right = f32(corner_idx == 1u || corner_idx == 2u);

    if (is_head) {
        // Expand point into a quad for the circle
        let center_ndc = start_clip.xy / start_clip.w;
        let sdf_coordinates = vec2<f32>(
            is_right * 2.0 - 1.0,
            is_end * 2.0 - 1.0
        );
        out.sdf_coordinates = sdf_coordinates;
        out.segment_len = 0.0;
        
        // Compensate for aspect ratio
        let offset = vec2<f32>(sdf_coordinates.x / uniforms.aspect, sdf_coordinates.y) * HEAD_RADIUS;
        
        // Map to NDC (normalized device coordinates), a square [-1, 1]
        out.clip_position = vec4<f32>(center_ndc + offset, start_clip.z / start_clip.w, 1.0);
    } else {
        // Expand line into a quad with rounded cap expansion

        // Map to NDC (normalized device coordinates), a square [-1, 1]
        let start_ndc = start_clip.xy / start_clip.w;
        let end_ndc = end_clip.xy / end_clip.w;
        let delta_ndc = end_ndc - start_ndc;
        let delta_screen = vec2<f32>(delta_ndc.x * uniforms.aspect, delta_ndc.y);
        let L_screen = length(delta_screen);

        // 2D basis vectors for the bone:
        // Along the bone
        let dir_screen = delta_screen / max(L_screen, EPSILON);
        // Perpendicular to the bone
        let norm_screen = vec2<f32>(-dir_screen.y, dir_screen.x);
        
        // Quad expansion logic: expansion includes space for rounded caps and AA fringe
        // 4.0 / screen_height = ~2 pixels of padding for smooth anti-aliasing
        let aa_padding = 4.0 / uniforms.screen_height;
        let expansion = BONE_RADIUS + aa_padding;
        
        let offset_lat = (is_right * 2.0 - 1.0) * expansion;
        let offset_long = (is_end * L_screen) + (is_end * 2.0 - 1.0) * expansion;

        // Expand the bone in length and width
        let offset_vec = dir_screen * offset_long + norm_screen * offset_lat;
        let final_ndc = start_ndc + vec2<f32>(offset_vec.x / uniforms.aspect, offset_vec.y);
        
        // Map to NDC (normalized device coordinates), a square [-1, 1]
        out.clip_position = vec4<f32>(final_ndc, start_clip.z / start_clip.w, 1.0);
        out.sdf_coordinates = vec2<f32>(offset_lat / BONE_RADIUS, offset_long / BONE_RADIUS);
        out.segment_len = L_screen / BONE_RADIUS;
    }

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var dist: f32 = 0.0;
    
    // Calculate the distance from the center line of the bone
    if (in.is_head == 1u) {
        dist = length(in.sdf_coordinates);
    } else {
        // Rounded cap distance logic
        let dx = in.sdf_coordinates.x;
        let dy = max(0.0, max(-in.sdf_coordinates.y, in.sdf_coordinates.y - in.segment_len));
        dist = length(vec2<f32>(dx, dy));
    }
    
    // 1 pixel wide fade
    let dist_per_pixel = fwidth(dist);
    // Smoothstep, if dist is within 1 pixel of the edge, fade
    let alpha = 1.0 - smoothstep(1.0 - dist_per_pixel, 1.0, dist);
    
    // Fully transparent -> discard
    if (alpha <= 0.0) {
        discard;
    }
    
    // Solid black, fade out at the edges
    return vec4<f32>(0.0, 0.0, 0.0, alpha);
}
