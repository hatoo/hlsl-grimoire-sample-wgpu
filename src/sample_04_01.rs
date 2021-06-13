use bytemuck::{Pod, Zeroable};
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

    let teapot = include_bytes!("../assets/teapot.glb");

    let (document, buffers, _images) = gltf::import_slice(teapot).unwrap();

    let primitives = loader::load_scene(&device, &document.scenes().next().unwrap(), &buffers);

    // Load the shaders from disk
    let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../assets/04_01.wgsl"))),
        flags: wgpu::ShaderFlags::all(),
    });

    let global_matrix_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("global matrix"),
        contents: bytemuck::cast_slice(&[Matrix {
            _matrix: cgmath::Matrix4::from_scale(0.01).into(),
        }]),
        usage: wgpu::BufferUsage::UNIFORM,
    });

    let local_matrix_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: primitives.len() as wgpu::BufferAddress * wgpu::BIND_BUFFER_ALIGNMENT,
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

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&uniform_bind_group_layout, &local_bind_group_layout],
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
                format: wgpu::VertexFormat::Float32x4,
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
            targets: &[swapchain_format.into()],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
    });

    let mut sc_desc = wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
        format: swapchain_format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Mailbox,
    };

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
            }
            Event::RedrawRequested(_) => {
                let frame = swap_chain
                    .get_current_frame()
                    .expect("Failed to acquire next swap chain texture")
                    .output;
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                {
                    for (i, primitive) in primitives.iter().enumerate() {
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
                        depth_stencil_attachment: None,
                    });
                    rpass.set_pipeline(&render_pipeline);
                    rpass.set_bind_group(0, &uniform_bind_group, &[]);
                    for (i, primitive) in primitives.iter().enumerate() {
                        rpass.set_bind_group(
                            1,
                            &local_bind_group,
                            &[(i as wgpu::BufferAddress * wgpu::BIND_BUFFER_ALIGNMENT)
                                as wgpu::DynamicOffset],
                        );
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
        _color: [f32; 4],
    }

    pub struct Primitive {
        pub transform: Matrix4<f32>,
        pub vertex_buffer: wgpu::Buffer,
        pub index_buffer: wgpu::Buffer,
        pub index_count: u32,
    }

    pub fn load_scene(
        device: &Device,
        scene: &gltf::Scene,
        buffers: &[gltf::buffer::Data],
    ) -> Vec<Primitive> {
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
                    let vertices = reader
                        .read_positions()
                        .unwrap()
                        .map(|p| Vertex {
                            _pos: [p[0], p[1], p[2], 1.0],
                            _color: color,
                        })
                        .collect::<Vec<_>>();
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
                    })
                }
            }

            nodes.extend(node.children().map(|node| (node, transform)));
        }

        primitives
    }
}
