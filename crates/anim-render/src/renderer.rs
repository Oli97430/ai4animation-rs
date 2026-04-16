//! Deferred rendering pipeline for the 3D scene.
//!
//! Pass order:
//! 1. Shadow map (depth-only from light perspective)
//! 2. G-Buffer (albedo+spec, normal+gloss, linear depth)
//! 3. SSAO + shadow lookup
//! 4. Blur SSAO (horizontal + vertical Gaussian)
//! 5. Deferred lighting (combines G-buffer + SSAO)
//! 6. Bloom post-process
//! 7. FXAA anti-aliasing → final target
//! 8. Debug line overlay (grid + skeleton + gizmos)

use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;
use crate::camera::Camera;
use crate::debug_draw::DebugDraw;
use crate::grid::GridConfig;
use crate::vertex::BasicVertex;
use crate::skinned_mesh::SkinnedMeshData;
use crate::render_settings::RenderSettings;

// ════════════════════════════════════════════════════════════════════
// Constants
// ════════════════════════════════════════════════════════════════════

const SHADOW_MAP_SIZE: u32 = 2048;

const GBUF_ALBEDO_FMT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const GBUF_NORMAL_FMT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const GBUF_DEPTH_FMT:  wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const HW_DEPTH_FMT:    wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const SSAO_FMT:        wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const HDR_FMT:         wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

// ════════════════════════════════════════════════════════════════════
// GPU uniform structs  (repr(C) + Pod/Zeroable for bytemuck)
// ════════════════════════════════════════════════════════════════════

/// Camera + light info, shared by G-buffer and line shaders.
/// Layout matches both `gbuffer.wgsl` and `line.wgsl` ViewUniforms.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ViewUniforms {
    view_proj:   [[f32; 4]; 4], // 0
    camera_pos:  [f32; 3],      // 64
    near:        f32,           // 76
    light_dir:   [f32; 3],      // 80
    far:         f32,           // 92
} // 96 bytes

/// Light view-projection for shadow map pass.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct LightUniforms {
    light_vp: [[f32; 4]; 4], // 64 bytes
}

/// SSAO + shadow sampling parameters.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SsaoParams {
    inv_view_proj: [[f32; 4]; 4], // 0
    view_matrix:   [[f32; 4]; 4], // 64
    light_vp:      [[f32; 4]; 4], // 128
    screen_size:   [f32; 2],      // 192
    near:          f32,           // 200
    far:           f32,           // 204
    radius:        f32,           // 208
    bias:          f32,           // 212
    intensity:     f32,           // 216
    shadow_bias:   f32,           // 220
} // 224 bytes

/// Gaussian blur direction (horizontal or vertical).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct BlurParams {
    direction: [f32; 2],
    _pad:      [f32; 2],
} // 16 bytes

/// Deferred lighting parameters.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct LightingParams {
    camera_pos:       [f32; 3], // 0
    exposure:         f32,      // 12
    light_dir:        [f32; 3], // 16
    sun_strength:     f32,      // 28
    sun_color:        [f32; 3], // 32
    sky_strength:     f32,      // 44
    sky_color:        [f32; 3], // 48
    ground_strength:  f32,      // 60
    ambient_strength: f32,      // 64
    _gap:             [f32; 3], // 68  (fills WGSL alignment gap)
    _pad:             [f32; 3], // 80  (matches WGSL vec3 _pad)
    _tail:            f32,      // 92  (struct tail padding)
} // 96 bytes

/// Bloom post-process parameters.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct BloomParams {
    texel_size: [f32; 2],
    spread:     f32,
    intensity:  f32,
} // 16 bytes

/// FXAA parameters.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct FxaaParams {
    texel_size: [f32; 2],
    _pad:       [f32; 2],
} // 16 bytes

// ════════════════════════════════════════════════════════════════════
// Bind-group-layout helpers
// ════════════════════════════════════════════════════════════════════

fn bgl_uniform(binding: u32, vis: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: vis,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn bgl_storage(binding: u32, vis: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: vis,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn bgl_tex2d(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn bgl_depth_tex2d(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Depth,
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn bgl_sampler(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

fn bgl_cmp_sampler(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
        count: None,
    }
}

// ════════════════════════════════════════════════════════════════════
// Fullscreen-pipeline factory
// ════════════════════════════════════════════════════════════════════

fn fullscreen_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    target_fmt: wgpu::TextureFormat,
    label: &str,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: "vs_main",
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: target_fmt,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview: None,
        cache: None,
    })
}

// ════════════════════════════════════════════════════════════════════
// Viewport-sized render textures (recreated on resize)
// ════════════════════════════════════════════════════════════════════

struct RenderTextures {
    size: (u32, u32),
    // G-Buffer
    albedo_spec:   wgpu::TextureView, // Rgba8Unorm
    normal_gloss:  wgpu::TextureView, // Rgba16Float
    linear_depth:  wgpu::TextureView, // Rgba16Float
    hw_depth:      wgpu::TextureView, // Depth32Float (not sampled)
    // SSAO
    ssao_raw:       wgpu::TextureView, // Rgba8Unorm
    ssao_blur_temp: wgpu::TextureView, // Rgba8Unorm
    ssao_blurred:   wgpu::TextureView, // Rgba8Unorm
    // Post-process
    lit_color:     wgpu::TextureView, // Rgba16Float
    bloom_output:  wgpu::TextureView, // Rgba16Float
}

impl RenderTextures {
    fn new(device: &wgpu::Device, w: u32, h: u32) -> Self {
        let tex = |label, fmt: wgpu::TextureFormat| -> wgpu::TextureView {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: fmt,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                     | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&Default::default())
        };

        let hw_depth = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("hw_depth"),
                size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: HW_DEPTH_FMT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&Default::default());

        Self {
            size: (w, h),
            albedo_spec:   tex("gbuf_albedo",   GBUF_ALBEDO_FMT),
            normal_gloss:  tex("gbuf_normal",   GBUF_NORMAL_FMT),
            linear_depth:  tex("gbuf_depth",    GBUF_DEPTH_FMT),
            hw_depth,
            ssao_raw:       tex("ssao_raw",      SSAO_FMT),
            ssao_blur_temp: tex("ssao_blur_tmp", SSAO_FMT),
            ssao_blurred:   tex("ssao_blurred",  SSAO_FMT),
            lit_color:     tex("lit_color",     HDR_FMT),
            bloom_output:  tex("bloom_out",     HDR_FMT),
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// Per-frame mesh GPU data (bone buffers kept alive for bind groups)
// ════════════════════════════════════════════════════════════════════

struct MeshGpuData {
    bone_bg:     wgpu::BindGroup,
    material_bg: wgpu::BindGroup,
    vertex_buf:  wgpu::Buffer,
    index_buf:   wgpu::Buffer,
    index_count: u32,
    _bone_buf:   wgpu::Buffer,    // prevent drop while bind group is in use
    _mat_buf:    wgpu::Buffer,    // material uniform buffer
    _tex:        Option<wgpu::Texture>, // albedo texture (keep alive)
}

// ════════════════════════════════════════════════════════════════════
// SceneRenderer
// ════════════════════════════════════════════════════════════════════

/// The 3D scene renderer with full deferred pipeline.
pub struct SceneRenderer {
    // ── Pipelines ──
    gbuffer_pipeline:  wgpu::RenderPipeline,
    shadow_pipeline:   wgpu::RenderPipeline,
    ssao_pipeline:     wgpu::RenderPipeline,
    blur_pipeline:     wgpu::RenderPipeline,
    lighting_pipeline: wgpu::RenderPipeline,
    bloom_pipeline:    wgpu::RenderPipeline,
    fxaa_pipeline:     wgpu::RenderPipeline,
    line_pipeline:     wgpu::RenderPipeline,

    // ── Bind-group layouts ──
    view_bgl:     wgpu::BindGroupLayout, // ViewUniforms  (gbuffer, line)
    bone_bgl:     wgpu::BindGroupLayout, // BoneMatrices   (gbuffer, shadow)
    material_bgl: wgpu::BindGroupLayout, // MaterialUniforms + texture + sampler
    light_bgl:    wgpu::BindGroupLayout, // LightUniforms  (shadow)
    ssao_bgl:     wgpu::BindGroupLayout, // SsaoParams + textures
    blur_bgl:     wgpu::BindGroupLayout, // BlurParams + tex + sampler
    lighting_bgl: wgpu::BindGroupLayout, // LightingParams + G-buf + ssao
    bloom_bgl:    wgpu::BindGroupLayout, // BloomParams + tex + sampler
    fxaa_bgl:     wgpu::BindGroupLayout, // tex + sampler + FxaaParams

    // ── Uniform buffers ──
    view_buf:     wgpu::Buffer,
    light_buf:    wgpu::Buffer,
    ssao_buf:     wgpu::Buffer,
    blur_h_buf:   wgpu::Buffer,
    blur_v_buf:   wgpu::Buffer,
    lighting_buf: wgpu::Buffer,
    bloom_buf:    wgpu::Buffer,
    fxaa_buf:     wgpu::Buffer,

    // ── Samplers ──
    linear_sampler: wgpu::Sampler,
    shadow_sampler: wgpu::Sampler,

    // ── Shadow map (fixed size, created once) ──
    shadow_map_view: wgpu::TextureView,

    // ── Viewport textures (recreated on resize) ──
    textures: Option<RenderTextures>,

    // ── Material fallback ──
    #[allow(dead_code)]
    fallback_texture_view: wgpu::TextureView,

    // ── Grid ──
    grid_vertex_buffer: Option<wgpu::Buffer>,
    grid_vertex_count:  u32,
}

impl SceneRenderer {
    /// Build all GPU pipelines, layouts, buffers, and samplers.
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        // ────────────────────────────────────────────────────────────
        // Bind-group layouts
        // ────────────────────────────────────────────────────────────

        let vf = wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT;
        let v  = wgpu::ShaderStages::VERTEX;
        let f  = wgpu::ShaderStages::FRAGMENT;

        // group 0 for G-buffer + line overlay
        let view_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("view_bgl"),
            entries: &[bgl_uniform(0, vf)],
        });

        // group 1 for G-buffer + shadow (bone matrices storage)
        let bone_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bone_bgl"),
            entries: &[bgl_storage(0, v)],
        });

        // group 2 for G-buffer (material uniform + albedo texture + sampler)
        let material_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("material_bgl"),
            entries: &[
                bgl_uniform(0, f),
                bgl_tex2d(1),
                bgl_sampler(2),
            ],
        });

        // group 0 for shadow (light VP)
        let light_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("light_bgl"),
            entries: &[bgl_uniform(0, v)],
        });

        // SSAO: params + normal + depth + shadow_depth + cmp_sampler + sampler
        let ssao_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ssao_bgl"),
            entries: &[
                bgl_uniform(0, f),
                bgl_tex2d(1),
                bgl_tex2d(2),
                bgl_depth_tex2d(3),
                bgl_cmp_sampler(4),
                bgl_sampler(5),
            ],
        });

        // Blur / bloom shared pattern: params + tex + sampler
        let blur_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("blur_bgl"),
            entries: &[bgl_uniform(0, f), bgl_tex2d(1), bgl_sampler(2)],
        });

        // Lighting: params + albedo + normal + depth + ssao + sampler
        let lighting_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lighting_bgl"),
            entries: &[
                bgl_uniform(0, f),
                bgl_tex2d(1),
                bgl_tex2d(2),
                bgl_tex2d(3),
                bgl_tex2d(4),
                bgl_sampler(5),
            ],
        });

        // Bloom: same layout as blur
        let bloom_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bloom_bgl"),
            entries: &[bgl_uniform(0, f), bgl_tex2d(1), bgl_sampler(2)],
        });

        // FXAA: tex + sampler + params  (note: different binding order)
        let fxaa_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fxaa_bgl"),
            entries: &[bgl_tex2d(0), bgl_sampler(1), bgl_uniform(2, f)],
        });

        // ────────────────────────────────────────────────────────────
        // Shader modules
        // ────────────────────────────────────────────────────────────

        let load = |label, src| {
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(src),
            })
        };

        let gbuffer_shader  = load("gbuffer",  include_str!("shaders/gbuffer.wgsl").into());
        let shadow_shader   = load("shadow",   include_str!("shaders/shadow.wgsl").into());
        let ssao_shader     = load("ssao",     include_str!("shaders/ssao.wgsl").into());
        let blur_shader     = load("blur",     include_str!("shaders/blur.wgsl").into());
        let lighting_shader = load("lighting", include_str!("shaders/lighting.wgsl").into());
        let bloom_shader    = load("bloom",    include_str!("shaders/bloom.wgsl").into());
        let fxaa_shader     = load("fxaa",     include_str!("shaders/fxaa.wgsl").into());
        let line_shader     = load("line",     include_str!("shaders/line.wgsl").into());

        // ────────────────────────────────────────────────────────────
        // Pipeline layouts
        // ────────────────────────────────────────────────────────────

        let pl = |label, bgls: &[&wgpu::BindGroupLayout]| {
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: bgls,
                push_constant_ranges: &[],
            })
        };

        let gbuffer_pl  = pl("gbuffer_pl",  &[&view_bgl, &bone_bgl, &material_bgl]);
        let shadow_pl   = pl("shadow_pl",   &[&light_bgl, &bone_bgl]);
        let ssao_pl     = pl("ssao_pl",     &[&ssao_bgl]);
        let blur_pl     = pl("blur_pl",     &[&blur_bgl]);
        let lighting_pl = pl("lighting_pl", &[&lighting_bgl]);
        let bloom_pl    = pl("bloom_pl",    &[&bloom_bgl]);
        let fxaa_pl     = pl("fxaa_pl",     &[&fxaa_bgl]);
        let line_pl     = pl("line_pl",     &[&view_bgl]);

        // ────────────────────────────────────────────────────────────
        // Render pipelines
        // ────────────────────────────────────────────────────────────

        let skinned_vertex_layout = crate::vertex::SkinnedVertex::desc();

        // G-buffer: skinned mesh → 3 render targets + HW depth
        let gbuffer_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("gbuffer_pipeline"),
            layout: Some(&gbuffer_pl),
            vertex: wgpu::VertexState {
                module: &gbuffer_shader,
                entry_point: "vs_main",
                buffers: &[skinned_vertex_layout.clone()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &gbuffer_shader,
                entry_point: "fs_main",
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format: GBUF_ALBEDO_FMT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: GBUF_NORMAL_FMT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: GBUF_DEPTH_FMT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: HW_DEPTH_FMT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        // Shadow: depth-only from light perspective
        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow_pipeline"),
            layout: Some(&shadow_pl),
            vertex: wgpu::VertexState {
                module: &shadow_shader,
                entry_point: "vs_main",
                buffers: &[skinned_vertex_layout],
                compilation_options: Default::default(),
            },
            fragment: None, // depth-only, no colour output
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: HW_DEPTH_FMT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        // Full-screen post-process pipelines
        let ssao_pipeline     = fullscreen_pipeline(device, &ssao_shader,     &ssao_pl,     SSAO_FMT,       "ssao_pipeline");
        let blur_pipeline     = fullscreen_pipeline(device, &blur_shader,     &blur_pl,     SSAO_FMT,       "blur_pipeline");
        let lighting_pipeline = fullscreen_pipeline(device, &lighting_shader, &lighting_pl, HDR_FMT,        "lighting_pipeline");
        let bloom_pipeline    = fullscreen_pipeline(device, &bloom_shader,    &bloom_pl,    HDR_FMT,        "bloom_pipeline");
        let fxaa_pipeline     = fullscreen_pipeline(device, &fxaa_shader,     &fxaa_pl,     target_format,  "fxaa_pipeline");

        // Debug lines (same as Phase 1)
        let line_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("line_pipeline"),
            layout: Some(&line_pl),
            vertex: wgpu::VertexState {
                module: &line_shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<BasicVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute { offset: 0,  shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
                        wgpu::VertexAttribute { offset: 12, shader_location: 1, format: wgpu::VertexFormat::Float32x4 },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &line_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: HW_DEPTH_FMT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        // ────────────────────────────────────────────────────────────
        // Uniform buffers
        // ────────────────────────────────────────────────────────────

        let ubuf = |label, size| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };

        let view_buf     = ubuf("view_ub",     std::mem::size_of::<ViewUniforms>() as u64);
        let light_buf    = ubuf("light_ub",    std::mem::size_of::<LightUniforms>() as u64);
        let ssao_buf     = ubuf("ssao_ub",     std::mem::size_of::<SsaoParams>()    as u64);
        let blur_h_buf   = ubuf("blur_h_ub",   std::mem::size_of::<BlurParams>()    as u64);
        let blur_v_buf   = ubuf("blur_v_ub",   std::mem::size_of::<BlurParams>()    as u64);
        let lighting_buf = ubuf("lighting_ub", std::mem::size_of::<LightingParams>() as u64);
        let bloom_buf    = ubuf("bloom_ub",    std::mem::size_of::<BloomParams>()    as u64);
        let fxaa_buf     = ubuf("fxaa_ub",     std::mem::size_of::<FxaaParams>()     as u64);

        // ────────────────────────────────────────────────────────────
        // Samplers
        // ────────────────────────────────────────────────────────────

        let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("linear_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow_sampler"),
            compare: Some(wgpu::CompareFunction::LessEqual),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        // ────────────────────────────────────────────────────────────
        // Shadow map (fixed size, never recreated)
        // ────────────────────────────────────────────────────────────

        let shadow_map_view = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("shadow_map"),
                size: wgpu::Extent3d {
                    width: SHADOW_MAP_SIZE,
                    height: SHADOW_MAP_SIZE,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: HW_DEPTH_FMT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                     | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&Default::default());

        // ────────────────────────────────────────────────────────────
        // Fallback 1x1 white texture (used when mesh has no albedo texture)
        // ────────────────────────────────────────────────────────────

        let fallback_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("fallback_white_1x1"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        // Note: texture will be white once first written — written lazily in render()
        let fallback_texture_view = fallback_texture.create_view(&Default::default());

        // ────────────────────────────────────────────────────────────

        Self {
            gbuffer_pipeline,
            shadow_pipeline,
            ssao_pipeline,
            blur_pipeline,
            lighting_pipeline,
            bloom_pipeline,
            fxaa_pipeline,
            line_pipeline,

            view_bgl,
            bone_bgl,
            material_bgl,
            light_bgl,
            ssao_bgl,
            blur_bgl,
            lighting_bgl,
            bloom_bgl,
            fxaa_bgl,

            view_buf,
            light_buf,
            ssao_buf,
            blur_h_buf,
            blur_v_buf,
            lighting_buf,
            bloom_buf,
            fxaa_buf,

            linear_sampler,
            shadow_sampler,
            shadow_map_view,

            fallback_texture_view,

            textures: None,
            grid_vertex_buffer: None,
            grid_vertex_count: 0,
        }
    }

    // ────────────────────────────────────────────────────────────────
    // Lazy resource creation
    // ────────────────────────────────────────────────────────────────

    fn ensure_textures(&mut self, device: &wgpu::Device, w: u32, h: u32) {
        let need = match &self.textures {
            Some(t) => t.size != (w, h),
            None => true,
        };
        if need && w > 0 && h > 0 {
            self.textures = Some(RenderTextures::new(device, w, h));
        }
    }

    fn ensure_grid(&mut self, device: &wgpu::Device, config: &GridConfig) {
        if self.grid_vertex_buffer.is_none() {
            let dd = DebugDraw::new();
            let verts = dd.grid_lines(config.size, config.divisions);
            self.grid_vertex_count = verts.len() as u32;
            self.grid_vertex_buffer = Some(device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("grid_vertices"),
                    contents: bytemuck::cast_slice(&verts),
                    usage: wgpu::BufferUsages::VERTEX,
                },
            ));
        }
    }

    // ════════════════════════════════════════════════════════════════
    // Main render entry point
    // ════════════════════════════════════════════════════════════════

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        width: u32,
        height: u32,
        camera: &Camera,
        debug_draw: &DebugDraw,
        grid_config: &GridConfig,
        skinned_meshes: &[&SkinnedMeshData],
        settings: &RenderSettings,
    ) {
        if width == 0 || height == 0 {
            return;
        }

        self.ensure_textures(device, width, height);
        self.ensure_grid(device, grid_config);

        let tex = self.textures.as_ref().unwrap();

        // ── Compute matrices ──────────────────────────────────────
        let aspect    = width as f32 / height as f32;
        let vp        = camera.view_projection(aspect);
        let view      = camera.view_matrix();
        let inv_vp    = vp.inverse();
        let light_dir = Vec3::from(settings.light_direction());
        let light_vp  = compute_light_vp(light_dir);

        // ── Upload uniforms (all driven by RenderSettings) ────────
        queue.write_buffer(&self.view_buf, 0, bytemuck::bytes_of(&ViewUniforms {
            view_proj:  vp.to_cols_array_2d(),
            camera_pos: camera.position.into(),
            near:       camera.near,
            light_dir:  light_dir.into(),
            far:        camera.far,
        }));

        queue.write_buffer(&self.light_buf, 0, bytemuck::bytes_of(&LightUniforms {
            light_vp: light_vp.to_cols_array_2d(),
        }));

        let tw = 1.0 / width as f32;
        let th = 1.0 / height as f32;

        let ssao_int = if settings.ssao_enabled { settings.ssao_intensity } else { 0.0 };
        queue.write_buffer(&self.ssao_buf, 0, bytemuck::bytes_of(&SsaoParams {
            inv_view_proj: inv_vp.to_cols_array_2d(),
            view_matrix:   view.to_cols_array_2d(),
            light_vp:      light_vp.to_cols_array_2d(),
            screen_size:   [width as f32, height as f32],
            near:          camera.near,
            far:           camera.far,
            radius:        settings.ssao_radius,
            bias:          settings.ssao_bias,
            intensity:     ssao_int,
            shadow_bias:   settings.shadow_bias,
        }));

        queue.write_buffer(&self.blur_h_buf, 0, bytemuck::bytes_of(&BlurParams {
            direction: [tw, 0.0],
            _pad: [0.0; 2],
        }));
        queue.write_buffer(&self.blur_v_buf, 0, bytemuck::bytes_of(&BlurParams {
            direction: [0.0, th],
            _pad: [0.0; 2],
        }));

        queue.write_buffer(&self.lighting_buf, 0, bytemuck::bytes_of(&LightingParams {
            camera_pos:       camera.position.into(),
            exposure:         settings.exposure,
            light_dir:        light_dir.into(),
            sun_strength:     settings.sun_strength,
            sun_color:        settings.sun_color,
            sky_strength:     settings.sky_strength,
            sky_color:        settings.sky_color,
            ground_strength:  settings.ground_strength,
            ambient_strength: settings.ambient_strength,
            _gap:  [0.0; 3],
            _pad:  [0.0; 3],
            _tail: 0.0,
        }));

        let bloom_int = if settings.bloom_enabled { settings.bloom_intensity } else { 0.0 };
        queue.write_buffer(&self.bloom_buf, 0, bytemuck::bytes_of(&BloomParams {
            texel_size: [tw, th],
            spread:     settings.bloom_spread,
            intensity:  bloom_int,
        }));

        queue.write_buffer(&self.fxaa_buf, 0, bytemuck::bytes_of(&FxaaParams {
            texel_size: [tw, th],
            _pad: [0.0; 2],
        }));

        // ── Create per-frame bind groups ──────────────────────────

        let view_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("view_bg"),
            layout: &self.view_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.view_buf.as_entire_binding(),
            }],
        });

        let light_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("light_bg"),
            layout: &self.light_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.light_buf.as_entire_binding(),
            }],
        });

        let ssao_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ssao_bg"),
            layout: &self.ssao_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.ssao_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&tex.normal_gloss) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&tex.linear_depth) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&self.shadow_map_view) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&self.shadow_sampler) },
                wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::Sampler(&self.linear_sampler) },
            ],
        });

        let blur_h_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blur_h_bg"),
            layout: &self.blur_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.blur_h_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&tex.ssao_raw) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.linear_sampler) },
            ],
        });

        let blur_v_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blur_v_bg"),
            layout: &self.blur_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.blur_v_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&tex.ssao_blur_temp) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.linear_sampler) },
            ],
        });

        let lighting_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lighting_bg"),
            layout: &self.lighting_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.lighting_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&tex.albedo_spec) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&tex.normal_gloss) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&tex.linear_depth) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(&tex.ssao_blurred) },
                wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::Sampler(&self.linear_sampler) },
            ],
        });

        let bloom_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bloom_bg"),
            layout: &self.bloom_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.bloom_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&tex.lit_color) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.linear_sampler) },
            ],
        });

        let fxaa_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fxaa_bg"),
            layout: &self.fxaa_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&tex.bloom_output) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.linear_sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: self.fxaa_buf.as_entire_binding() },
            ],
        });

        // ── Upload mesh GPU data (outlives all passes) ────────────

        let mesh_gpu: Vec<MeshGpuData> = skinned_meshes
            .iter()
            .map(|mesh| {
                let bone_data: Vec<[[f32; 4]; 4]> = mesh
                    .bone_matrices
                    .iter()
                    .map(|m| m.to_cols_array_2d())
                    .collect();
                let bone_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("bone_matrices"),
                    contents: bytemuck::cast_slice(&bone_data),
                    usage: wgpu::BufferUsages::STORAGE,
                });
                let bone_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("bone_bg"),
                    layout: &self.bone_bgl,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: bone_buf.as_entire_binding(),
                    }],
                });

                // Material uniform
                let mat_uniforms = mesh.material_uniforms();
                let mat_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("material_ub"),
                    contents: bytemuck::bytes_of(&mat_uniforms),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

                // Albedo texture (or fallback)
                let (tex_view, owned_tex) = if let Some(ref tex_data) = mesh.texture_data {
                    let tex = device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("mesh_albedo"),
                        size: wgpu::Extent3d { width: tex_data.width, height: tex_data.height, depth_or_array_layers: 1 },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                        view_formats: &[],
                    });
                    queue.write_texture(
                        wgpu::ImageCopyTexture {
                            texture: &tex,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        &tex_data.pixels,
                        wgpu::ImageDataLayout {
                            offset: 0,
                            bytes_per_row: Some(4 * tex_data.width),
                            rows_per_image: None,
                        },
                        wgpu::Extent3d { width: tex_data.width, height: tex_data.height, depth_or_array_layers: 1 },
                    );
                    let view = tex.create_view(&Default::default());
                    (view, Some(tex))
                } else {
                    // Use fallback white texture (borrow from self)
                    // We can't borrow self here, so create a tiny 1x1 white
                    let tex = device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("fallback_1x1"),
                        size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                        view_formats: &[],
                    });
                    queue.write_texture(
                        wgpu::ImageCopyTexture {
                            texture: &tex,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        &[255u8, 255, 255, 255],
                        wgpu::ImageDataLayout {
                            offset: 0,
                            bytes_per_row: Some(4),
                            rows_per_image: None,
                        },
                        wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
                    );
                    let view = tex.create_view(&Default::default());
                    (view, Some(tex))
                };

                let material_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("material_bg"),
                    layout: &self.material_bgl,
                    entries: &[
                        wgpu::BindGroupEntry { binding: 0, resource: mat_buf.as_entire_binding() },
                        wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&tex_view) },
                        wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.linear_sampler) },
                    ],
                });

                let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("mesh_verts"),
                    contents: bytemuck::cast_slice(&mesh.vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("mesh_idx"),
                    contents: bytemuck::cast_slice(&mesh.indices),
                    usage: wgpu::BufferUsages::INDEX,
                });
                MeshGpuData {
                    bone_bg,
                    material_bg,
                    vertex_buf,
                    index_buf,
                    index_count: mesh.indices.len() as u32,
                    _bone_buf: bone_buf,
                    _mat_buf: mat_buf,
                    _tex: owned_tex,
                }
            })
            .collect();

        // ══════════════════════════════════════════════════════════
        // Pass 1 ─ Shadow Map (skipped when shadows disabled)
        // ══════════════════════════════════════════════════════════
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow_pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_map_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            if settings.shadows_enabled {
                pass.set_pipeline(&self.shadow_pipeline);
                pass.set_bind_group(0, &light_bg, &[]);
                for mgd in &mesh_gpu {
                    pass.set_bind_group(1, &mgd.bone_bg, &[]);
                    pass.set_vertex_buffer(0, mgd.vertex_buf.slice(..));
                    pass.set_index_buffer(mgd.index_buf.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..mgd.index_count, 0, 0..1);
                }
            }
            // When disabled, shadow map stays cleared at 1.0 → no shadows
        }

        // ══════════════════════════════════════════════════════════
        // Pass 2 ─ G-Buffer
        // ══════════════════════════════════════════════════════════
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("gbuffer_pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &tex.albedo_spec,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &tex.normal_gloss,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.5, g: 0.5, b: 1.0, a: 0.0 }),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &tex.linear_depth,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 }),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &tex.hw_depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_pipeline(&self.gbuffer_pipeline);
            pass.set_bind_group(0, &view_bg, &[]);
            for mgd in &mesh_gpu {
                pass.set_bind_group(1, &mgd.bone_bg, &[]);
                pass.set_bind_group(2, &mgd.material_bg, &[]);
                pass.set_vertex_buffer(0, mgd.vertex_buf.slice(..));
                pass.set_index_buffer(mgd.index_buf.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mgd.index_count, 0, 0..1);
            }
        }

        // ══════════════════════════════════════════════════════════
        // Pass 3 ─ SSAO + Shadow
        // ══════════════════════════════════════════════════════════
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ssao_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &tex.ssao_raw,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            pass.set_pipeline(&self.ssao_pipeline);
            pass.set_bind_group(0, &ssao_bg, &[]);
            pass.draw(0..3, 0..1);
        }

        // ══════════════════════════════════════════════════════════
        // Pass 4 ─ Blur Horizontal
        // ══════════════════════════════════════════════════════════
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("blur_h_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &tex.ssao_blur_temp,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            pass.set_pipeline(&self.blur_pipeline);
            pass.set_bind_group(0, &blur_h_bg, &[]);
            pass.draw(0..3, 0..1);
        }

        // ══════════════════════════════════════════════════════════
        // Pass 5 ─ Blur Vertical
        // ══════════════════════════════════════════════════════════
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("blur_v_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &tex.ssao_blurred,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            pass.set_pipeline(&self.blur_pipeline);
            pass.set_bind_group(0, &blur_v_bg, &[]);
            pass.draw(0..3, 0..1);
        }

        // ══════════════════════════════════════════════════════════
        // Pass 6 ─ Deferred Lighting
        // ══════════════════════════════════════════════════════════
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("lighting_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &tex.lit_color,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            pass.set_pipeline(&self.lighting_pipeline);
            pass.set_bind_group(0, &lighting_bg, &[]);
            pass.draw(0..3, 0..1);
        }

        // ══════════════════════════════════════════════════════════
        // Pass 7 ─ Bloom
        // ══════════════════════════════════════════════════════════
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &tex.bloom_output,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            pass.set_pipeline(&self.bloom_pipeline);
            pass.set_bind_group(0, &bloom_bg, &[]);
            pass.draw(0..3, 0..1);
        }

        // ══════════════════════════════════════════════════════════
        // Pass 8 ─ FXAA → final target
        // ══════════════════════════════════════════════════════════
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("fxaa_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.12, g: 0.12, b: 0.15, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            pass.set_pipeline(&self.fxaa_pipeline);
            pass.set_bind_group(0, &fxaa_bg, &[]);
            pass.draw(0..3, 0..1);
        }

        // ══════════════════════════════════════════════════════════
        // Pass 9 ─ Debug Line Overlay (grid + skeleton + gizmos)
        // ══════════════════════════════════════════════════════════
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("line_overlay"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // preserve FXAA output
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &tex.hw_depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load, // preserve G-buffer depth
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            pass.set_pipeline(&self.line_pipeline);
            pass.set_bind_group(0, &view_bg, &[]);

            // Grid
            if grid_config.visible {
                if let Some(ref grid_buf) = self.grid_vertex_buffer {
                    pass.set_vertex_buffer(0, grid_buf.slice(..));
                    pass.draw(0..self.grid_vertex_count, 0..1);
                }
            }

            // Debug lines (skeleton, gizmos, velocities, etc.)
            if !debug_draw.lines.is_empty() {
                let line_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("debug_lines"),
                    contents: bytemuck::cast_slice(&debug_draw.lines),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                pass.set_vertex_buffer(0, line_buf.slice(..));
                pass.draw(0..debug_draw.lines.len() as u32, 0..1);
            }
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// Helpers
// ════════════════════════════════════════════════════════════════════

/// Compute an orthographic light-view-projection for shadow mapping.
fn compute_light_vp(light_dir: Vec3) -> Mat4 {
    let light_pos = -light_dir.normalize() * 15.0;
    let light_view = Mat4::look_at_rh(light_pos, Vec3::ZERO, Vec3::Y);
    let light_proj = Mat4::orthographic_rh(-15.0, 15.0, -15.0, 15.0, 0.1, 50.0);
    light_proj * light_view
}
