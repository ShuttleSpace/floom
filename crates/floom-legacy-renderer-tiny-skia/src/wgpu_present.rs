//! Present tiny-skia pixmaps on OHOS through the wgpu GLES surface.
//!
//! softbuffer has no OHOS backend, so the CPU-rasterized pixels are uploaded
//! to a texture each frame and blitted over the already-initialized wgpu
//! surface. The GLES adapters found on OHOS (e.g. the DevEco emulator's
//! DGLES, a GLES 3.0 host-translation layer) do not expose the vertex-shader
//! storage buffers that vger needs, so tiny-skia is the only viable renderer
//! there and this module is its presentation path.

use anyhow::{Result, anyhow};

use floom_legacy_renderer::gpu_resources::GpuResources;
use floom_legacy_renderer::tiny_skia::Pixmap;

const BLIT_WGSL: &str = r#"
@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var out: VertexOutput;
    // Fullscreen triangle. The uv array is flipped vertically because
    // tiny-skia row 0 is the top of the image while texture v=0 maps to the
    // bottom in NDC orientation.
    let p = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    let uv = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, -1.0),
    );
    out.pos = vec4<f32>(p[vi], 0.0, 1.0);
    out.uv = uv[vi];
    return out;
}

@fragment
fn fs(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(tex, samp, in.uv);
}
"#;

pub(crate) struct WgpuPresent {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
    staging: wgpu::Buffer,
    staging_len: u64,
}

impl WgpuPresent {
    pub(crate) fn new(
        gpu_resources: GpuResources,
        surface: wgpu::Surface<'static>,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        let GpuResources {
            adapter, device, queue, ..
        } = gpu_resources;

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|it| {
                matches!(
                    it,
                    wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Bgra8Unorm
                )
            })
            .ok_or_else(|| anyhow!("surface supports neither Rgba8Unorm nor Bgra8Unorm"))?;
        // The pixmap is presented opaque, matching the softbuffer backends
        // which also discard the alpha channel on present.
        let alpha_mode = caps
            .alpha_modes
            .iter()
            .copied()
            .find(|mode| *mode == wgpu::CompositeAlphaMode::Opaque)
            .unwrap_or(wgpu::CompositeAlphaMode::Auto);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: width.max(1),
            height: height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("floom-tiny-skia-blit"),
            source: wgpu::ShaderSource::Wgsl(BLIT_WGSL.into()),
        });

        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("floom-tiny-skia-blit-bgl"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float {
                                filterable: true,
                            },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(
                            wgpu::SamplerBindingType::Filtering,
                        ),
                        count: None,
                    },
                ],
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("floom-tiny-skia-blit-pl"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("floom-tiny-skia-blit-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(format.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("floom-tiny-skia-blit-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // The pixel data is uploaded to a fixed RGBA8 texture (no swizzling
        // for BGRA swapchains; the blit shader handles that) through a
        // staging buffer so the 256-byte row alignment required by
        // `copy_buffer_to_texture` is respected for arbitrary widths.
        let texture = Self::create_texture(&device, config.width, config.height);
        let staging_len = staging_len(config.width, config.height);
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("floom-tiny-skia-blit-staging"),
            size: staging_len,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = texture_bind_group(&device, &bind_group_layout, &sampler, &texture);
        Ok(Self {
            device,
            queue,
            surface,
            config,
            bind_group,
            texture,
            bind_group_layout,
            pipeline,
            sampler,
            staging,
            staging_len,
        })
    }

    fn create_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("floom-tiny-skia-blit-texture"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        })
    }

    fn recreate_texture(&mut self, width: u32, height: u32) {
        self.texture = Self::create_texture(&self.device, width, height);
        self.bind_group =
            texture_bind_group(&self.device, &self.bind_group_layout, &self.sampler, &self.texture);
    }

    /// Mirrors the surface to a new size.
    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);
        if self.config.width != width || self.config.height != height {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.recreate_texture(width, height);
            let staging_len = staging_len(width, height);
            if staging_len > self.staging_len {
                self.staging_len = staging_len;
                self.staging = self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("floom-tiny-skia-blit-staging"),
                    size: staging_len,
                    usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
            }
        }
    }

    /// Upload the pixmap and blit it over the whole surface.
    pub(crate) fn present(&mut self, pixmap: &Pixmap) -> Result<()> {
        let width = pixmap.width();
        let height = pixmap.height();
        if self.config.width != width || self.config.height != height {
            self.resize(width, height);
        }

        let bytes_per_row = padded_bytes_per_row(width);
        if bytes_per_row == width * 4 {
            self.queue
                .write_buffer(&self.staging, 0, pixmap.data());
        } else {
            // Row-by-row write; the trailing padding bytes are never read by
            // the copy (its extent covers `width` texels per row only).
            let row_len = (width * 4) as usize;
            for y in 0..height {
                let offset = (y * bytes_per_row) as wgpu::BufferAddress;
                let row = &pixmap.data()[y as usize * row_len..(y as usize + 1) * row_len];
                self.queue.write_buffer(&self.staging, offset, row);
            }
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("floom-tiny-skia-blit-encoder"),
            });
        encoder.copy_buffer_to_texture(
            wgpu::TexelCopyBufferInfo {
                buffer: &self.staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            self.texture.as_image_copy(),
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let frame = self
            .surface
            .get_current_texture()
            .map_err(|err| anyhow!("failed to acquire surface texture: {err}"))?;
        {
            let view = frame.texture.create_view(&Default::default());
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("floom-tiny-skia-blit-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }
}

fn texture_bind_group(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    texture: &wgpu::Texture,
) -> wgpu::BindGroup {
    let view = texture.create_view(&Default::default());
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("floom-tiny-skia-blit-bg"),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

fn padded_bytes_per_row(width: u32) -> u32 {
    (width * 4 + 255) & !255
}

fn staging_len(width: u32, height: u32) -> u64 {
    (padded_bytes_per_row(width) * height) as u64
}
