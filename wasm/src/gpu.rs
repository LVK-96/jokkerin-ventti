use static_assertions::const_assert_eq;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures;
use wgpu::util::DeviceExt;

use crate::skeleton::{RENDER_BONE_COUNT, SkinnedVertex, generate_bind_pose_mesh};

// Shared background/sky color
const SKY_COLOR: wgpu::Color = wgpu::Color {
    r: 0.8,
    g: 0.9,
    b: 1.0,
    a: 1.0,
};

/// WGSL Uniform struct
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub view: [[f32; 4]; 4],       // 64 bytes
    pub projection: [[f32; 4]; 4], // 64 bytes
    pub aspect: f32,               // 4 bytes
    pub screen_height: f32,        // 4 bytes
    pub _padding: [f32; 6],        // 24 bytes -> total 160 bytes
}
// Size of the uniforms struct needs to be a multiple of 16 bytes
const_assert_eq!(std::mem::size_of::<Uniforms>(), 160);

impl Default for Uniforms {
    fn default() -> Self {
        Self {
            view: glam::Mat4::IDENTITY.to_cols_array_2d(),
            projection: glam::Mat4::IDENTITY.to_cols_array_2d(),
            aspect: 1.0,
            screen_height: 600.0,
            _padding: [0.0; 6],
        }
    }
}

pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    // Render pipelines
    pub skeleton_pipeline: wgpu::RenderPipeline,
    pub grid_pipeline: wgpu::RenderPipeline,
    // GPU Buffers
    pub vertex_buffer: wgpu::Buffer,
    pub bone_uniform_buffer: wgpu::Buffer,
    pub uniform_buffer: wgpu::Buffer,
    // Depth texture
    pub depth_texture: wgpu::Texture,
    pub depth_view: wgpu::TextureView,
    // Bind groups
    pub uniform_bind_group: wgpu::BindGroup,
    pub bone_bind_group: wgpu::BindGroup,
    // Render state
    pub uniforms: Uniforms,
    pub vertex_count: u32,
}

/// Shader sources
const SKELETON_SHADER: &str = include_str!("shaders/skeleton.wgsl");
const GRID_SHADER: &str = include_str!("shaders/grid.wgsl");

fn get_canvas_size(window: &web_sys::Window, canvas: &web_sys::HtmlCanvasElement) -> (u32, u32) {
    // CSS pixels * device pixel ratio = physical pixels
    let dpr = window.device_pixel_ratio();
    // Update to canvas buffer size: (widt, height)
    (
        (canvas.client_width() as f64 * dpr) as u32,
        (canvas.client_height() as f64 * dpr) as u32,
    )
}

/// Initialize WebGPU context from a canvas element
/// wasm_bindgen + pub async fn
/// -> Generates a promies for JS
#[wasm_bindgen]
pub async fn init_gpu(canvas_id: String) -> Result<(), JsValue> {
    // Set up panic hook for better error messages in browser console
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Info).ok();

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("No document"))?;
    let canvas = document
        .get_element_by_id(&canvas_id)
        .ok_or_else(|| JsValue::from_str("Canvas not found"))?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| JsValue::from_str("Not a canvas"))?;

    let (width, height) = get_canvas_size(&window, &canvas);
    canvas.set_width(width);
    canvas.set_height(height);

    // WebGPU Initialization Flow:
    // 1. Instance: The entry point to the wgpu API, we are targetting WebGPU
    // 2. Surface: The canvas window we draw to
    // 3. Adapter: The physical graphics card (stateless, describes capabilities)
    // 4. Device: The logical connection to the card (stateful, creates resources)
    // 5. Queue: The command queue for submitting work to the GPU
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::BROWSER_WEBGPU,
        ..Default::default()
    });

    // Create surface from canvas
    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
        .map_err(|e| JsValue::from_str(&format!("Failed to create surface: {}", e)))?;

    // Request adapter
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .map_err(|_| JsValue::from_str("Failed to find GPU adapter"))?;

    // Request device and queue
    let (device, queue): (wgpu::Device, wgpu::Queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("Main Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
            memory_hints: Default::default(),
            experimental_features: Default::default(),
            trace: wgpu::Trace::Off,
        })
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to create device: {}", e)))?;

    // Configure surface
    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps.formats[0];

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width,
        height,
        present_mode: wgpu::PresentMode::AutoVsync,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    // Create shader modules
    let skeleton_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Skeleton Shader"),
        source: wgpu::ShaderSource::Wgsl(SKELETON_SHADER.into()),
    });

    let grid_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Grid Shader"),
        source: wgpu::ShaderSource::Wgsl(GRID_SHADER.into()),
    });

    // Create uniform buffer
    let uniforms = Uniforms::default();
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Uniform Buffer"),
        contents: bytemuck::cast_slice(&[uniforms]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    // Create uniform bind group layout
    let uniform_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

    // Create uniform bind group
    let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Uniform Bind Group"),
        layout: &uniform_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });

    // Create bone uniform buffer
    // Holds 29 mat4s
    let bone_buffer_size = (RENDER_BONE_COUNT * 64) as u64;
    let bone_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Bone Matrices Buffer"),
        size: bone_buffer_size,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // Initialize with identity matrices to prevent zero-scale geometry before first update
    let initial_bones = vec![glam::Mat4::IDENTITY.to_cols_array_2d(); RENDER_BONE_COUNT];
    queue.write_buffer(
        &bone_uniform_buffer,
        0,
        bytemuck::cast_slice(&initial_bones),
    );

    // Create bone bind group layout
    let bone_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Bone Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

    // Create bone bind group
    let bone_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Bone Bind Group"),
        layout: &bone_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: bone_uniform_buffer.as_entire_binding(),
        }],
    });

    // Create pipeline layout
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Skeleton Pipeline Layout"),
        bind_group_layouts: &[&uniform_bind_group_layout, &bone_bind_group_layout],
        immediate_size: 0,
    });

    // Constants for the grid shader
    let grid_constants = [
        ("SKY_R", SKY_COLOR.r),
        ("SKY_G", SKY_COLOR.g),
        ("SKY_B", SKY_COLOR.b),
    ];

    // Create skeleton render pipeline
    let skeleton_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Skeleton Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &skeleton_shader,
            entry_point: Some("vs_main"),
            buffers: &[
                // Buffer 0: SkinnedVertex
                wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<SkinnedVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        // position
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        // normal
                        wgpu::VertexAttribute {
                            offset: 12,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        // bone_index
                        wgpu::VertexAttribute {
                            offset: 24,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Uint32,
                        },
                    ],
                },
            ],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &skeleton_shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: None, // No blending for solid 3D objects
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back), // Backface culling
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth24Plus,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });

    // Create depth texture
    let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Depth Texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth24Plus,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

    // Generate bind pose mesh (static)
    let mesh_vertices = generate_bind_pose_mesh();
    let vertex_count = mesh_vertices.len() as u32;
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Skeleton Vertex Buffer"),
        contents: bytemuck::cast_slice(&mesh_vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    // Create grid render pipeline setup
    let grid_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Grid Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

    let grid_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Grid Pipeline Layout"),
        bind_group_layouts: &[&grid_bind_group_layout],
        immediate_size: 0,
    });

    let grid_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Grid Pipeline"),
        layout: Some(&grid_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &grid_shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &grid_constants,
                ..Default::default()
            },
        },
        fragment: Some(wgpu::FragmentState {
            module: &grid_shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &grid_constants,
                ..Default::default()
            },
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth24Plus,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });

    // Set up default camera
    let mut uniforms = Uniforms::default();
    let eye = glam::Vec3::new(2.5, 1.2, 3.0);
    let target = glam::Vec3::new(0.0, 0.5, 0.0);
    let up = glam::Vec3::Y;
    uniforms.view = glam::Mat4::look_at_rh(eye, target, up).to_cols_array_2d();

    let aspect = width as f32 / height as f32;
    uniforms.aspect = aspect;
    uniforms.screen_height = height as f32;
    uniforms.projection = glam::Mat4::perspective_rh(
        std::f32::consts::FRAC_PI_4, // 45 degrees FOV
        aspect,
        0.1,
        100.0,
    )
    .to_cols_array_2d();

    // Update uniform buffer
    queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

    let state = GpuContext {
        device,
        queue,
        surface,
        config,
        skeleton_pipeline,
        grid_pipeline,
        vertex_buffer,
        bone_uniform_buffer,
        uniform_buffer,
        depth_texture,
        depth_view,
        uniform_bind_group,
        bone_bind_group,
        uniforms,
        vertex_count,
    };

    // Initialize centralized AppState with GPU context
    crate::state::initialize_app_state(state);

    log::info!("WebGPU initialized with skeleton pipeline!");
    Ok(())
}

/// Resize the WebGPU surface when canvas size changes
/// Call this from a window resize event listener
#[wasm_bindgen]
pub fn resize_surface(canvas_id: String) {
    let window = web_sys::window().expect("No window");
    let document = window.document().expect("No document");
    let canvas = document
        .get_element_by_id(&canvas_id)
        .expect("Canvas not found")
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .expect("Not a canvas");

    let (width, height) = get_canvas_size(&window, &canvas);
    canvas.set_width(width);
    canvas.set_height(height);

    crate::state::with_app_state_mut(|app| {
        let gpu = &mut app.gpu;
        // Update surface configuration
        gpu.config.width = width;
        gpu.config.height = height;
        gpu.surface.configure(&gpu.device, &gpu.config);

        // Recreate depth texture with new dimensions
        let depth_texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth_texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        gpu.depth_texture = depth_texture;
        gpu.depth_view = gpu
            .depth_texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Update aspect ratio and projection matrix
        let aspect = width as f32 / height as f32;
        gpu.uniforms.aspect = aspect;
        gpu.uniforms.screen_height = height as f32;
        gpu.uniforms.projection =
            glam::Mat4::perspective_rh(std::f32::consts::FRAC_PI_4, aspect, 0.1, 100.0)
                .to_cols_array_2d();

        // Write updated uniforms to GPU
        gpu.queue.write_buffer(
            &gpu.uniform_buffer,
            0,
            bytemuck::cast_slice(&[gpu.uniforms]),
        );

        log::info!("Resized to {}x{}", width, height);
    });
}

/// Sync camera state to GPU - updates view matrix from stored quaternion
///
/// Call this after rotate_camera() to push the updated view matrix to the GPU.
#[wasm_bindgen]
pub fn sync_camera() {
    crate::state::with_app_state_mut(|app| {
        let view = app.camera.view_matrix();
        app.gpu.uniforms.view = view.to_cols_array_2d();
        app.gpu.queue.write_buffer(
            &app.gpu.uniform_buffer,
            0,
            bytemuck::cast_slice(&[app.gpu.uniforms]),
        );
    });
}

/// Get the current camera view matrix as a Float32Array (16 floats, column-major)
/// Used by TypeScript for gizmo rendering
#[wasm_bindgen]
pub fn get_current_view_matrix() -> Vec<f32> {
    crate::state::with_app_state(|app| {
        app.gpu
            .uniforms
            .view
            .iter()
            .flat_map(|row| row.iter().copied())
            .collect()
    })
    .unwrap_or_else(|| vec![0.0; 16])
}

/// Get the current projection matrix
/// Used by TypeScript for handle-based joint positioning
#[wasm_bindgen]
pub fn get_current_projection_matrix() -> Vec<f32> {
    crate::state::with_app_state(|app| {
        app.gpu
            .uniforms
            .projection
            .iter()
            .flat_map(|row| row.iter().copied())
            .collect()
    })
    .unwrap_or_else(|| vec![0.0; 16])
}

/// Update bone matrices uniform buffer
/// Call this to push new skeleton pose to the GPU
pub fn update_bone_uniforms(matrices: &[glam::Mat4]) {
    crate::state::with_app_state_mut(|app| {
        app.gpu.queue.write_buffer(
            &app.gpu.bone_uniform_buffer,
            0,
            bytemuck::cast_slice(matrices),
        );
    });
}

/// Render a frame
#[wasm_bindgen]
pub fn render_frame() {
    crate::state::with_app_state(|app| {
        let gpu = &app.gpu;
        let output = match gpu.surface.get_current_texture() {
            Ok(t) => t,
            Err(_) => return, // Surface lost
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Skeleton Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(SKY_COLOR),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &gpu.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            // Draw background grid
            render_pass.set_pipeline(&gpu.grid_pipeline);
            // Grid uses uniform bind group at index 0
            render_pass.set_bind_group(0, &gpu.uniform_bind_group, &[]);
            render_pass.draw(0..6, 0..1);

            // Draw skinned mesh
            render_pass.set_pipeline(&gpu.skeleton_pipeline);
            render_pass.set_bind_group(0, &gpu.uniform_bind_group, &[]);
            render_pass.set_bind_group(1, &gpu.bone_bind_group, &[]);
            render_pass.set_vertex_buffer(0, gpu.vertex_buffer.slice(..));

            render_pass.draw(0..gpu.vertex_count, 0..1);
        }

        gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    });
}
