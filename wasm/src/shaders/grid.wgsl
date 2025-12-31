// Procedural floor grid shader

// Sky overriden at shader compile time
override SKY_R: f32;
override SKY_G: f32;
override SKY_B: f32;

// White
const FLOOR_R: f32 = 1.0;
const FLOOR_G: f32 = 1.0;
const FLOOR_B: f32 = 1.0;

// Gray
const GRID_R: f32 = 0.7;
const GRID_G: f32 = 0.7;
const GRID_B: f32 = 0.7;

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

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec2<f32>,
}

// Constants
const FLOOR_SIZE: f32 = 50.0;
const MINOR_LINE_THICKNESS: f32 = 0.02;
const MAJOR_LINE_THICKNESS: f32 = 0.03;
const MAJOR_GRID_STEP: f32 = 5.0;
const MINOR_GRID_OPACITY: f32 = 0.3;
const MAJOR_GRID_OPACITY: f32 = 0.5;
const FADE_START: f32 = 5.0;
const FADE_END: f32 = 40.0;
const AA_STRENGTH: f32 = 1.0;

// Large floor quad vertices in world space (Y=0 plane)
const FLOOR_VERTICES: array<vec3<f32>, 6> = array<vec3<f32>, 6>(
    vec3<f32>(-FLOOR_SIZE, 0.0, -FLOOR_SIZE),
    vec3<f32>( FLOOR_SIZE, 0.0, -FLOOR_SIZE),
    vec3<f32>( FLOOR_SIZE, 0.0,  FLOOR_SIZE),
    vec3<f32>(-FLOOR_SIZE, 0.0, -FLOOR_SIZE),
    vec3<f32>( FLOOR_SIZE, 0.0,  FLOOR_SIZE),
    vec3<f32>(-FLOOR_SIZE, 0.0,  FLOOR_SIZE)
);

@vertex
fn vs_main(@builtin(vertex_index) vertex_idx: u32) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = FLOOR_VERTICES[vertex_idx];

    // Transform to clip space
    let view_pos = uniforms.view * vec4<f32>(world_pos, 1.0);
    out.clip_position = uniforms.projection * view_pos;
    out.world_pos = world_pos.xz;

    return out;
}

// Compute grid line intensity
fn on_grid_line(coord: f32, line_width: f32) -> f32 {
    // Grid lines are at every 1 unit
    // Wrap the coordinate to [0, 0.5]
    // By taking the abs of the fraction (centered around 0, subtracting 0.5 twice):
    //  e.g. if coord = 2.1:
    //  fract(2.1 - 0.5) = 0.6
    //  0.6 - 0.5 = 0.1
    //  abs(0.1) = 0.1 -> close to the center of a line
    // (0 is exactly on a line, 0.5 exactly between two lines)
    let wrapped = abs(fract(coord - 0.5) - 0.5);
    // How much coord changes per pixel
    let coord_per_pixel = fwidth(coord);
    // Inside the line: wrapped < line_width * 0.5
    // At the line edge: wrapped - (line_width * 0.5) = 0 -> start fading
    // Outside the line: wrapped > coord_per_pixel * AA_STRENGTH
    return 1.0 - smoothstep(0.0, coord_per_pixel * AA_STRENGTH, wrapped - (line_width * 0.5));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Grid lines at every 1 unit
    let grid_x = on_grid_line(in.world_pos.x, MINOR_LINE_THICKNESS);
    let grid_z = on_grid_line(in.world_pos.y, MINOR_LINE_THICKNESS);
    let grid_intensity = max(grid_x, grid_z);

    // Thicker lines at every MAJOR_GRID_STEP units
    let major_x = on_grid_line(in.world_pos.x / MAJOR_GRID_STEP, MAJOR_LINE_THICKNESS);
    let major_z = on_grid_line(in.world_pos.y / MAJOR_GRID_STEP, MAJOR_LINE_THICKNESS);
    let major_intensity = max(major_x, major_z);

    // Combine grids
    let combined = max(grid_intensity * MINOR_GRID_OPACITY, major_intensity * MAJOR_GRID_OPACITY);

    // Distance-based fade to white
    let dist = length(in.world_pos);
    let fade = 1.0 - smoothstep(FADE_START, FADE_END, dist);

    // Distance-based fade
    let world_dist = length(in.world_pos);
    let horizon_fade = smoothstep(FADE_START, FADE_END, world_dist);
    let grid_fade = 1.0 - horizon_fade;

    // Background color: White floor near camera, Sky blue at horizon
    let floor_color = vec3<f32>(FLOOR_R, FLOOR_G, FLOOR_B);
    let sky_color = vec3<f32>(SKY_R, SKY_G, SKY_B);
    let current_bg = mix(floor_color, sky_color, horizon_fade);

    // Grid color
    let grid_color = vec3<f32>(GRID_R, GRID_G, GRID_B);

    // Mix background with grid lines (grid also fades out at horizon)
    let final_color = mix(current_bg, grid_color, combined * grid_fade);

    return vec4<f32>(final_color, 1.0);
}
