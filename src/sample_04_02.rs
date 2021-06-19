use bytemuck::{Pod, Zeroable};
use cgmath::{Matrix4, Quaternion};
use std::{borrow::Cow, mem::size_of};
use wgpu::util::DeviceExt;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct Matrix {
    _matrix: [[f32; 4]; 4],
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

    let dummy_texture = texture::Texture::dummy(&device, &queue, "dummy").unwrap();

    let dummy_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &texture_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&dummy_texture.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&dummy_texture.sampler),
            },
        ],
        label: Some("diffuse_bind_group"),
    });

    let teapot = include_bytes!("../assets/teapot.glb");

    // You can use _images to load a texture but it's more easier to reparse an image from buffers.
    let (document, buffers, _images) = gltf::import_slice(teapot).unwrap();

    let scene = loader::load_first_scene(
        &device,
        &queue,
        &document,
        &buffers,
        &texture_bind_group_layout,
    );

    // Load the shaders from disk
    let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../assets/04_02.wgsl"))),
        flags: wgpu::ShaderFlags::all(),
    });

    let global_matrix_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("global matrix"),
        contents: bytemuck::cast_slice(&[Matrix {
            _matrix: (Matrix4::from_translation(cgmath::vec3(0.0, 0.0, 0.5))
                * Matrix4::from_scale(0.01)
                * Matrix4::from(Quaternion::from(cgmath::Euler {
                    x: cgmath::Rad(0.0),
                    y: cgmath::Rad(0.0),
                    z: cgmath::Rad(0.0),
                })))
            .into(),
        }]),
        usage: wgpu::BufferUsage::UNIFORM,
    });

    let local_matrix_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: scene.primitives.len() as wgpu::BufferAddress * wgpu::BIND_BUFFER_ALIGNMENT,
        usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        mapped_at_creation: false,
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

    let local_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStage::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: wgpu::BufferSize::new(
                        size_of::<Matrix>() as wgpu::BufferAddress
                    ),
                },
                count: None,
            }],
            label: None,
        });

    let directional_light_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: None,
        });

    let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &uniform_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: global_matrix_buffer.as_entire_binding(),
        }],
        label: Some("uniform_bind_group"),
    });

    let local_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &local_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &local_matrix_buffer,
                offset: 0,
                size: wgpu::BufferSize::new(size_of::<Matrix>() as wgpu::BufferAddress),
            }),
        }],
        label: None,
    });

    let light_direction_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("light direction"),
        contents: bytemuck::cast_slice(&Into::<[f32; 3]>::into(cgmath::InnerSpace::normalize(
            cgmath::vec3(1.0f32, -1.0, 1.0),
        ))),
        usage: wgpu::BufferUsage::UNIFORM,
    });

    let light_color_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("light direction"),
        contents: bytemuck::cast_slice(&[0.5f32, 0.5, 0.5]),
        usage: wgpu::BufferUsage::UNIFORM,
    });

    let directional_light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &directional_light_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &light_direction_buffer,
                    offset: 0,
                    size: wgpu::BufferSize::new(size_of::<[f32; 3]>() as wgpu::BufferAddress),
                }),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &light_color_buffer,
                    offset: 0,
                    size: wgpu::BufferSize::new(size_of::<[f32; 3]>() as wgpu::BufferAddress),
                }),
            },
        ],
        label: None,
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[
            &uniform_bind_group_layout,
            &local_bind_group_layout,
            &texture_bind_group_layout,
            &directional_light_bind_group_layout,
        ],
        push_constant_ranges: &[],
    });

    let swapchain_format = adapter.get_swap_chain_preferred_format(&surface).unwrap();

    let vertex_size = std::mem::size_of::<loader::Vertex>();
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
                format: wgpu::VertexFormat::Float32x3,
                offset: size_of::<[f32; 4]>() as wgpu::BufferAddress,
                shader_location: 1,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: (size_of::<[f32; 4]>() + size_of::<[f32; 4]>()) as wgpu::BufferAddress,
                shader_location: 2,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: (size_of::<[f32; 4]>() + size_of::<[f32; 4]>() + size_of::<[f32; 4]>())
                    as wgpu::BufferAddress,
                shader_location: 3,
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
            targets: &[swapchain_format.into()],
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: Some(wgpu::Face::Front),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: texture::Texture::DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less, // 1.
            stencil: wgpu::StencilState::default(),     // 2.
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

    event_loop.run(move |event, _, control_flow| {
        // Have the closure take ownership of the resources.
        // `event_loop.run` never returns, therefore we must do this to ensure
        // the resources are properly cleaned up.
        let _ = (&instance, &adapter, &shader, &pipeline_layout);

        *control_flow = ControlFlow::Wait;
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
                let frame = swap_chain
                    .get_current_frame()
                    .expect("Failed to acquire next swap chain texture")
                    .output;
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                {
                    for (i, primitive) in scene.primitives.iter().enumerate() {
                        queue.write_buffer(
                            &local_matrix_buffer,
                            i as wgpu::BufferAddress * wgpu::BIND_BUFFER_ALIGNMENT,
                            bytemuck::bytes_of(&Matrix {
                                _matrix: primitive.transform.into(),
                            }),
                        );
                    }

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
                    rpass.set_bind_group(0, &uniform_bind_group, &[]);
                    rpass.set_bind_group(3, &directional_light_bind_group, &[]);
                    for (i, primitive) in scene.primitives.iter().enumerate() {
                        rpass.set_bind_group(
                            1,
                            &local_bind_group,
                            &[(i as wgpu::BufferAddress * wgpu::BIND_BUFFER_ALIGNMENT)
                                as wgpu::DynamicOffset],
                        );
                        rpass.set_bind_group(2, &dummy_bind_group, &[]);
                        if let Some(id) = primitive.texture_id {
                            if let Some(bind_group) = &scene.textures[id] {
                                rpass.set_bind_group(2, bind_group, &[]);
                            }
                        }
                        rpass.set_vertex_buffer(0, primitive.vertex_buffer.slice(..));
                        rpass.set_index_buffer(
                            primitive.index_buffer.slice(..),
                            wgpu::IndexFormat::Uint32,
                        );
                        rpass.draw_indexed(0..primitive.index_count, 0, 0..1);
                    }
                }

                queue.submit(Some(encoder.finish()));
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

mod loader {
    use super::*;
    use cgmath::{Matrix4, Quaternion, SquareMatrix, Vector3};
    use wgpu::Device;

    #[repr(C)]
    #[derive(Clone, Copy, Pod, Zeroable)]
    pub struct Vertex {
        _pos: [f32; 4],
        _normal: [f32; 3],
        _pad: f32,
        _color: [f32; 4],
        _tex_coord: [f32; 2],
    }

    pub struct Primitive {
        pub transform: Matrix4<f32>,
        pub vertex_buffer: wgpu::Buffer,
        pub index_buffer: wgpu::Buffer,
        pub index_count: u32,
        pub texture_id: Option<usize>,
    }

    pub struct Scene {
        pub textures: Vec<Option<wgpu::BindGroup>>,
        pub primitives: Vec<Primitive>,
    }

    pub fn load_first_scene(
        device: &Device,
        queue: &wgpu::Queue,
        root: &gltf::Document,
        buffers: &[gltf::buffer::Data],
        texture_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Scene {
        let textures = root
            .materials()
            .map(|material| {
                material
                    .pbr_metallic_roughness()
                    .base_color_texture()
                    .map(|info| {
                        let image = info.texture().source();

                        let image = match image.source() {
                            gltf::image::Source::View { view, mime_type: _ } => {
                                let parent_buffer_data = &buffers[view.buffer().index()].0;
                                let begin = view.offset();
                                let end = begin + view.length();
                                let data = &parent_buffer_data[begin..end];
                                image::load_from_memory(data)
                            }
                            _ => todo!(),
                        }
                        .unwrap();

                        let diffuse_texture =
                            texture::Texture::from_image(device, queue, &image, None).unwrap();

                        let diffuse_bind_group =
                            device.create_bind_group(&wgpu::BindGroupDescriptor {
                                layout: texture_bind_group_layout,
                                entries: &[
                                    wgpu::BindGroupEntry {
                                        binding: 0,
                                        resource: wgpu::BindingResource::TextureView(
                                            &diffuse_texture.view,
                                        ),
                                    },
                                    wgpu::BindGroupEntry {
                                        binding: 1,
                                        resource: wgpu::BindingResource::Sampler(
                                            &diffuse_texture.sampler,
                                        ),
                                    },
                                ],
                                label: Some("diffuse_bind_group"),
                            });

                        diffuse_bind_group
                    })
            })
            .collect();

        let scene = root.scenes().next().unwrap();
        let mut primitives = Vec::new();

        let mut nodes = scene
            .nodes()
            .map(|node| (node, Matrix4::<f32>::identity()))
            .collect::<Vec<_>>();

        while let Some((node, transform)) = nodes.pop() {
            let (trans, rot, scale) = node.transform().decomposed();

            let transform = transform
                * Matrix4::from_translation(Vector3::from(trans))
                * Matrix4::from_nonuniform_scale(scale[0], scale[1], scale[2])
                * Matrix4::from(Quaternion::new(rot[3], rot[0], rot[1], rot[2]));

            if let Some(mesh) = node.mesh() {
                for primitive in mesh.primitives() {
                    let material = primitive.material();
                    let color = material.pbr_metallic_roughness().base_color_factor();

                    let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
                    let vertices = if let Some(coords) = reader.read_tex_coords(0) {
                        reader
                            .read_positions()
                            .unwrap()
                            .zip(reader.read_normals().unwrap())
                            .zip(coords.into_f32())
                            .map(|((p, n), c)| Vertex {
                                _pos: [p[0], p[1], p[2], 1.0],
                                _normal: n,
                                _pad: 0.0,
                                _color: [0.0, 0.0, 0.0, 0.0],
                                _tex_coord: c,
                            })
                            .collect::<Vec<_>>()
                    } else {
                        reader
                            .read_positions()
                            .unwrap()
                            .zip(reader.read_normals().unwrap())
                            .map(|(p, n)| Vertex {
                                _pos: [p[0], p[1], p[2], 1.0],
                                _normal: n,
                                _pad: 0.0,
                                _color: color,
                                _tex_coord: [0.0, 0.0],
                            })
                            .collect::<Vec<_>>()
                    };
                    let indices = reader
                        .read_indices()
                        .unwrap()
                        .into_u32()
                        .collect::<Vec<_>>();

                    let vertex_buffer =
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Vertex Buffer"),
                            contents: bytemuck::cast_slice(&vertices),
                            usage: wgpu::BufferUsage::VERTEX,
                        });

                    let index_buffer =
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Index Buffer"),
                            contents: bytemuck::cast_slice(&indices),
                            usage: wgpu::BufferUsage::INDEX,
                        });

                    primitives.push(Primitive {
                        transform,
                        vertex_buffer,
                        index_buffer,
                        index_count: indices.len() as u32,
                        texture_id: material.index(),
                    })
                }
            }

            nodes.extend(node.children().map(|node| (node, transform)));
        }

        Scene {
            primitives,
            textures,
        }
    }
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

        pub fn dummy(device: &wgpu::Device, queue: &wgpu::Queue, label: &str) -> Result<Self> {
            let img = image::DynamicImage::new_rgba8(16, 16);
            Self::from_image(device, queue, &img, Some(label))
        }

        pub fn from_image(
            device: &wgpu::Device,
            queue: &wgpu::Queue,
            img: &image::DynamicImage,
            label: Option<&str>,
        ) -> Result<Self> {
            let rgba = img.to_rgba8().to_vec();
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
