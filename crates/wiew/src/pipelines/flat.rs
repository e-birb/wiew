
use std::sync::Arc;

use wgpu::{CompareFunction, Device, PrimitiveState, PrimitiveTopology, ShaderModule};

use crate::{decl_vertex_raw_repr, instance::Instance3d, Pass, ProjectionCameraCommon, RenderContext, SingletonResource, VertexBufferSlice, VertexRawRepr};

use super::Pipeline;

decl_vertex_raw_repr! {
    #[derive(Debug)]
    struct Vertex (Vertex step mode) {
        pub position: [f32; 3] as [7 => Float32x3],
        pub color: [f32; 4] as [8 => Float32x4],
    }
}

/// A shader for flat color
pub struct FlatShader {
    shader: ShaderModule,
}

impl FlatShader {
    /// Create a new flat shader
    pub fn new(
        device: &Device,
    ) -> Self {
        Self {
            shader: device.create_shader_module(wgpu::include_wgsl!("flat.wgsl")),
        }
    }
}

impl SingletonResource for FlatShader {
    fn init(ctx: &mut RenderContext) -> Self {
        Self::new(ctx.device)
    }
}

pub struct FlatPipeline {
    pipeline: Pipeline,
}

impl FlatPipeline {
    pub fn new(
        topology: PrimitiveTopology,
        depth_compare: wgpu::CompareFunction,
        use_depth_stencil: bool,
    ) -> Self {

        let mut primitive: PrimitiveState = Default::default();
        primitive.topology = topology;

        let pipeline = Pipeline::from_builder(move |cx, formats| {
            let shader = cx.singleton::<FlatShader>();

            let camera_common = cx.singleton::<ProjectionCameraCommon>();

            // the layout of our pipeline
            let render_pipeline_layout =
            cx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[
                    camera_common.layout(),
                ],
                push_constant_ranges: &[],
            });

            let targets = formats.target_formats.iter().map(|format| {
                Some(wgpu::ColorTargetState {
                    format: format.clone(),
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })
            }).collect::<Vec<_>>();

            cx.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("flat pipeline"),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader.shader,
                    entry_point: "vs_main",
                    buffers: &[
                        Vertex::desc(),
                        Instance3d::desc(),
                    ],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader.shader,
                    entry_point: "fs_main",
                    targets: &targets,
                    compilation_options: Default::default(),
                }),
                primitive,
                depth_stencil: formats.depth_format.map(|format| wgpu::DepthStencilState {
                    format, // TODO all
                    depth_write_enabled: use_depth_stencil,
                    depth_compare,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: Default::default(),
                multiview: None,
                cache: None,
            })
        });

        Self {
            pipeline,
        }
    }

    pub fn render<'a>(
        &self,
        cx: &mut RenderContext,
        pass: &mut Pass<'a>,
        vertices: impl Into<VertexBufferSlice<Vertex>>,
        instances: impl Into<VertexBufferSlice<Instance3d>>,
    ) {
        let vertices: VertexBufferSlice<Vertex> = vertices.into();
        let instances: VertexBufferSlice<Instance3d> = instances.into();

        let pipeline = self.pipeline.get(cx, pass);

        pass.defer((pipeline, vertices, instances), move |rp, globals, (pipeline, vertices, instances)| {
            rp.set_pipeline(&pipeline);
            rp.set_bind_group(0, &globals, &[]);
            rp.set_vertex_buffer(0, vertices.buffer.slice(..)); // TODO ??? we are doing something redundant
            rp.set_vertex_buffer(1, instances.buffer.slice(..));
            rp.draw(vertices.range.clone(), instances.range.clone());
        });
    }
}

/// A shader for flat color
pub struct FlatIdShader {
    shader: ShaderModule,
}

impl FlatIdShader {
    /// Create a new flat shader
    pub fn new(
        device: &Device,
    ) -> Self {
        Self {
            shader: device.create_shader_module(wgpu::include_wgsl!("flat_id.wgsl")),
        }
    }
}

impl SingletonResource for FlatIdShader {
    fn init(ctx: &mut RenderContext) -> Self {
        Self::new(ctx.device)
    }
}

pub struct FlatIdentityPipeline {
    pipeline: Pipeline,
}

impl FlatIdentityPipeline {
    pub fn new(
        topology: PrimitiveTopology,
    ) -> Self {

        let mut primitive: PrimitiveState = Default::default();
        primitive.topology = topology;

        let pipeline = Pipeline::from_builder(move |cx, formats| {
            let shader = cx.singleton::<FlatIdShader>();

            // the layout of our pipeline
            let render_pipeline_layout =
            cx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

            let targets = formats.target_formats.iter().map(|format| {
                Some(wgpu::ColorTargetState {
                    format: format.clone(),
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })
            }).collect::<Vec<_>>();

            cx.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("flat pipeline"),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader.shader,
                    entry_point: "vs_main",
                    buffers: &[
                        Vertex::desc(),
                        Instance3d::desc(),
                    ],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader.shader,
                    entry_point: "fs_main",
                    targets: &targets,
                    compilation_options: Default::default(),
                }),
                primitive,
                depth_stencil: formats.depth_format.map(|format| wgpu::DepthStencilState {
                    format, // TODO all
                    depth_write_enabled: false,
                    depth_compare: CompareFunction::Always,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: Default::default(),
                multiview: None,
                cache: None,
            })
        });

        Self {
            pipeline,
        }
    }

    pub fn render<'a>(
        &self,
        cx: &mut RenderContext,
        pass: &mut Pass<'a>,
        vertices: impl Into<VertexBufferSlice<Vertex>>,
        instances: impl Into<VertexBufferSlice<Instance3d>>,
        index_buffer: Option<Arc<wgpu::Buffer>>,
    ) {
        let vertices: VertexBufferSlice<Vertex> = vertices.into();
        let instances: VertexBufferSlice<Instance3d> = instances.into();

        let pipeline = self.pipeline.get(cx, pass);

        pass.defer((pipeline, vertices, instances, index_buffer), move |rp, _, (pipeline, vertices, instances, index_buffer)| {
            rp.set_pipeline(&pipeline);
            rp.set_vertex_buffer(0, vertices.buffer.slice(..)); // TODO ??? we are doing something redundant
            rp.set_vertex_buffer(1, instances.buffer.slice(..));
            if let Some(index_buffer) = &index_buffer {
                rp.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                rp.draw_indexed(0..index_buffer.size() as u32 / 2, 0, 0..instances.range.end as u32);
            } else {
                rp.draw(vertices.range.clone(), instances.range.clone());
            }
        });
    }
}