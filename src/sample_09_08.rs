use bytemuck::{Pod, Zeroable};
use cgmath::{vec3, Matrix4};
use std::{borrow::Cow, mem::size_of};
use wgpu::util::DeviceExt;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    _pos: [f32; 4],
    _tex_coords: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Effect {
    _rate: f32,
}

fn vertex(x: f32, y: f32, tex_coords: [f32; 2]) -> Vertex {
    Vertex {
        _pos: [x, y, 0.0, 1.0],
        _tex_coords: tex_coords,
    }
}

fn create_vertices() -> Vec<Vertex> {
    vec![
        vertex(-1.0, -1.0, [0.0, 1.0]),
        vertex(1.0, 1.0, [1.0, 0.0]),
        vertex(1.0, -1.0, [1.0, 1.0]),
        vertex(-1.0, 1.0, [0.0, 0.0]),
    ]
}

async fn run(event_loop: EventLoop<()>, window: Window) {
    let size = window.inner_size();
    let instance = wgpu::Instance::new(wgpu::BackendBit::all());
    let surface = unsafe { instance.create_surface(&window) };
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            // Request an adapter which can render to our surface
            compatible_surface: Some(&surface),
        })
        .await
        .expect("Failed to find an appropriate adapter");

    // Create the logical device and command queue
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
            },
            None,
        )
        .await
        .expect("Failed to create device");

    // Load the shaders from disk
    let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../assets/09_08.wgsl"))),
        flags: wgpu::ShaderFlags::all(),
    });

    let background_matrix_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("global matrix"),
        contents: bytemuck::bytes_of::<[[f32; 4]; 4]>(
            &Matrix4::<f32>::from_translation(vec3(0.0, 0.0, 0.1)).into(),
        ),
        usage: wgpu::BufferUsage::UNIFORM,
    });

    let foreground_matrix_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("global matrix"),
        contents: bytemuck::bytes_of::<[[f32; 4]; 4]>(&Matrix4::<f32>::from_scale(0.5).into()),
        usage: wgpu::BufferUsage::UNIFORM,
    });

    let effect_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("wipe"),
        contents: bytemuck::cast_slice(&[Effect { _rate: 0.0 }]),
        usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
    });

    let uniform_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStage::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("uniform_bind_group_layout"),
        });

    let effect_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStage::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("wipe_bind_group_layout"),
        });

    let background_matrix_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &uniform_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: background_matrix_buffer.as_entire_binding(),
        }],
        label: Some("background_matrix_bind_group"),
    });

    let foreground_matrix_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &uniform_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: foreground_matrix_buffer.as_entire_binding(),
        }],
        label: Some("foreground_matrix_bind_group"),
    });

    let effect_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &effect_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: effect_buffer.as_entire_binding(),
        }],
        label: Some("effect_bind_group"),
    });

    let foreground_bytes = include_bytes!("../assets/rustacean-orig-noshadow.png");
    let foreground_texture = texture::Texture::from_bytes(
        &device,
        &queue,
        foreground_bytes,
        "rustacean-orig-noshadow.png",
    )
    .unwrap();

    let background_bytes = include_bytes!("../assets/stone_00081.jpg");
    let background_texture =
        texture::Texture::from_bytes(&device, &queue, background_bytes, "stone_00081.jpg").unwrap();

    let texture_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler {
                        comparison: false,
                        filtering: true,
                    },
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });

    let foreground_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &texture_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&foreground_texture.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&foreground_texture.sampler),
            },
        ],
        label: Some("foreground_bind_group"),
    });

    let background_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &texture_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&background_texture.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&background_texture.sampler),
            },
        ],
        label: Some("foreground_bind_group"),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[
            &texture_bind_group_layout,
            &uniform_bind_group_layout,
            &effect_bind_group_layout,
        ],
        push_constant_ranges: &[],
    });

    let swapchain_format = adapter.get_swap_chain_preferred_format(&surface).unwrap();

    let vertex_size = std::mem::size_of::<Vertex>();
    let vertex_buffers = [wgpu::VertexBufferLayout {
        array_stride: vertex_size as wgpu::BufferAddress,
        step_mode: wgpu::InputStepMode::Vertex,
        attributes: &[
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: 0,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: size_of::<[f32; 4]>() as wgpu::BufferAddress,
                shader_location: 1,
            },
        ],
    }];

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &vertex_buffers,
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[wgpu::ColorTargetState {
                format: swapchain_format.into(),
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        operation: wgpu::BlendOperation::Add,
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    },
                    alpha: wgpu::BlendComponent::REPLACE,
                }),
                write_mask: wgpu::ColorWrite::ALL,
            }],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: texture::Texture::DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
    });

    let mut sc_desc = wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
        format: swapchain_format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Mailbox,
    };

    let mut depth_texture =
        texture::Texture::create_depth_texture(&device, &sc_desc, "depth_texture");

    let mut swap_chain = device.create_swap_chain(&surface, &sc_desc);

    let vertex_data = create_vertices();
    let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertex_data),
        usage: wgpu::BufferUsage::VERTEX,
    });

    let index_data: Vec<u32> = vec![0, 1, 2, 3, 1, 0];
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Index Buffer"),
        contents: bytemuck::cast_slice(&index_data),
        usage: wgpu::BufferUsage::INDEX,
    });

    let start = std::time::Instant::now();

    event_loop.run(move |event, _, control_flow| {
        // Have the closure take ownership of the resources.
        // `event_loop.run` never returns, therefore we must do this to ensure
        // the resources are properly cleaned up.
        let _ = (&instance, &adapter, &shader, &pipeline_layout);

        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                // Recreate the swap chain with the new size
                sc_desc.width = size.width.max(1);
                sc_desc.height = size.height.max(1);
                swap_chain = device.create_swap_chain(&surface, &sc_desc);
                depth_texture =
                    texture::Texture::create_depth_texture(&device, &sc_desc, "depth_texture");
            }
            Event::RedrawRequested(_) => {
                let secs = start.elapsed().as_secs_f32();
                let new_wipe = Effect { _rate: secs % 1.0 };

                queue.write_buffer(&effect_buffer, 0, bytemuck::cast_slice(&[new_wipe]));

                let frame = swap_chain
                    .get_current_frame()
                    .expect("Failed to acquire next swap chain texture")
                    .output;
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[wgpu::RenderPassColorAttachment {
                            view: &frame.view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                                store: true,
                            },
                        }],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &depth_texture.view,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                store: true,
                            }),
                            stencil_ops: None,
                        }),
                    });
                    rpass.set_pipeline(&render_pipeline);
                    rpass.set_vertex_buffer(0, vertex_buf.slice(..));
                    rpass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    rpass.set_bind_group(2, &effect_bind_group, &[]);

                    rpass.set_bind_group(0, &background_bind_group, &[]);
                    rpass.set_bind_group(1, &background_matrix_bind_group, &[]);
                    rpass.draw_indexed(0..index_data.len() as u32, 0, 0..1);

                    rpass.set_bind_group(0, &foreground_bind_group, &[]);
                    rpass.set_bind_group(1, &foreground_matrix_bind_group, &[]);
                    rpass.draw_indexed(0..index_data.len() as u32, 0, 0..1);
                }

                queue.submit(Some(encoder.finish()));
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => {}
        }
    });
}

fn main() {
    let event_loop = EventLoop::new();
    let window = winit::window::Window::new(&event_loop).unwrap();
    env_logger::init();
    // Temporarily avoid srgb formats for the swapchain on the web
    pollster::block_on(run(event_loop, window));
}

mod texture {
    use std::num::NonZeroU32;

    use anyhow::*;
    use image::GenericImageView;

    pub struct Texture {
        pub texture: wgpu::Texture,
        pub view: wgpu::TextureView,
        pub sampler: wgpu::Sampler,
    }

    impl Texture {
        pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

        pub fn create_depth_texture(
            device: &wgpu::Device,
            sc_desc: &wgpu::SwapChainDescriptor,
            label: &str,
        ) -> Self {
            let size = wgpu::Extent3d {
                // 2.
                width: sc_desc.width,
                height: sc_desc.height,
                depth_or_array_layers: 1,
            };
            let desc = wgpu::TextureDescriptor {
                label: Some(label),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: Self::DEPTH_FORMAT,
                usage: wgpu::TextureUsage::RENDER_ATTACHMENT // 3.
                | wgpu::TextureUsage::SAMPLED,
            };
            let texture = device.create_texture(&desc);

            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                // 4.
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                compare: Some(wgpu::CompareFunction::LessEqual), // 5.
                lod_min_clamp: -100.0,
                lod_max_clamp: 100.0,
                ..Default::default()
            });

            Self {
                texture,
                view,
                sampler,
            }
        }

        pub fn from_bytes(
            device: &wgpu::Device,
            queue: &wgpu::Queue,
            bytes: &[u8],
            label: &str,
        ) -> Result<Self> {
            let img = image::load_from_memory(bytes)?;
            Self::from_image(device, queue, &img, Some(label))
        }

        pub fn from_image(
            device: &wgpu::Device,
            queue: &wgpu::Queue,
            img: &image::DynamicImage,
            label: Option<&str>,
        ) -> Result<Self> {
            let rgba = img.to_rgba8();
            let dimensions = img.dimensions();

            let size = wgpu::Extent3d {
                width: dimensions.0,
                height: dimensions.1,
                depth_or_array_layers: 1,
            };
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
            });

            queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                },
                &rgba,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: NonZeroU32::new(4 * dimensions.0),
                    rows_per_image: NonZeroU32::new(dimensions.1),
                },
                size,
            );

            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            });

            Ok(Self {
                texture,
                view,
                sampler,
            })
        }
    }
}
