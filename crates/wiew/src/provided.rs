use std::{ops::Deref, sync::{Arc, Mutex}};

use rotation3::Placement3;
use wgpu::{util::DeviceExt, Buffer, PrimitiveTopology};

use crate::{instance::Instance3d, pipelines::flat::{self, FlatIdentityPipeline, FlatPipeline}, Pass, ProjectionCameraBuffer, Render, RenderContext, Res, SurfaceInfo, Trackball, TrackballCamera, VertexBuffer, View};


pub trait Scene3d: 'static + Send + Sync {
    fn raster(
        &mut self,
        cx: &mut RenderContext,
        pass: &mut Pass,
    );

    fn background_color(&self) -> Scene3dBackground {
        Scene3dBackground::DEFAULT_BG_RAINBOW
    }

    fn grid(&self) -> bool {
        true
    }
}

pub struct Scene3dBackground {
    pub top_left: [f32; 4],
    pub top_right: [f32; 4],
    pub bottom_left: [f32; 4],
    pub bottom_right: [f32; 4],
}

impl Scene3dBackground {
    pub const DEFAULT_BG_HORIZONTAL: Self = Self::vertical_gradient(
        [0x1c as f32 / 255.0, 0x1c as f32 / 255.0, 0x1c as f32 / 255.0, 1.0],
        [0x26 as f32 / 255.0, 0x19 as f32 / 255.0, 0x38 as f32 / 255.0, 1.0],
    );

    pub const DEFAULT_BG_RAINBOW: Self = Scene3dBackground {
        top_left: [14.0 / 255.0, 41.0 / 255.0, 29.0 / 255.0, 255.0 / 255.0],
        top_right: [54.0 / 255.0, 22.0 / 255.0, 22.0 / 255.0, 255.0 / 255.0],
        bottom_left: [20.0 / 255.0, 17.0 / 255.0, 51.0 / 255.0, 255.0 / 255.0],
        bottom_right: [42.0 / 255.0, 20.0 / 255.0, 55.0 / 255.0, 255.0 / 255.0],
    };

    pub const fn transparent() -> Self {
        Self {
            top_left: [0.0, 0.0, 0.0, 0.0],
            top_right: [0.0, 0.0, 0.0, 0.0],
            bottom_left: [0.0, 0.0, 0.0, 0.0],
            bottom_right: [0.0, 0.0, 0.0, 0.0],
        }
    }

    pub const fn uniform_color(color: [f32; 4]) -> Self {
        Self {
            top_left: color,
            top_right: color,
            bottom_left: color,
            bottom_right: color,
        }
    }

    pub const fn horizontal_gradient(left: [f32; 4], right: [f32; 4]) -> Self {
        Self {
            top_left: left,
            top_right: right,
            bottom_left: left,
            bottom_right: right,
        }
    }

    pub const fn vertical_gradient(top: [f32; 4], bottom: [f32; 4]) -> Self {
        Self {
            top_left: top,
            top_right: top,
            bottom_left: bottom,
            bottom_right: bottom,
        }
    }

    fn vertices(&self) -> [flat::Vertex; 5] {
        [
            flat::Vertex { position: [-1.0, -1.0, 0.0], color: self.bottom_left },
            flat::Vertex { position: [1.0, -1.0, 0.0], color: self.bottom_right },
            flat::Vertex { position: [1.0, 1.0, 0.0], color: self.top_right },
            flat::Vertex { position: [-1.0, 1.0, 0.0], color: self.top_left },

            // central: average
            flat::Vertex {
                position: [0.0, 0.0, 0.0],
                color: [
                    (self.top_left[0] + self.top_right[0] + self.bottom_left[0] + self.bottom_right[0]) / 4.0,
                    (self.top_left[1] + self.top_right[1] + self.bottom_left[1] + self.bottom_right[1]) / 4.0,
                    (self.top_left[2] + self.top_right[2] + self.bottom_left[2] + self.bottom_right[2]) / 4.0,
                    (self.top_left[3] + self.top_right[3] + self.bottom_left[3] + self.bottom_right[3]) / 4.0,
                ],
            },
        ]
    }
}

pub struct MyView3d {
    camera: Arc<Mutex<TrackballCamera>>,

    depth_texture: Option<(u32, u32, Res<(wgpu::Texture, wgpu::TextureView)>)>,

    camera_buffer: Res<Mutex<ProjectionCameraBuffer>>,
    //triangle: Resource<stupid_triangle::Triangle>,
    trackball: Trackball,

    grid: Grid,
    bg: Bg,

    scene: Mutex<Box<dyn Scene3d>>,
}

impl MyView3d {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn new(scene: impl Scene3d, camera: Arc<Mutex<TrackballCamera>>) -> Self {
        Self {
            camera,
            depth_texture: None,
            camera_buffer: Res::new(|cx: &mut RenderContext| Mutex::new(ProjectionCameraBuffer::new(cx))),
            //triangle: Resource::new(move |cx: &mut wiew::RenderContext| stupid_triangle::Triangle::new(cx, &[presentation_target_format])),
            trackball: Trackball::new(),
            bg: Bg::new(),
            grid: Grid::new(10),
            scene: Mutex::new(Box::new(scene)),
        }
    }
}

impl View for MyView3d {
    fn view(
        &mut self,
        cx: &mut RenderContext,
    ) -> Vec<wgpu::CommandBuffer> {
        let camera = self.camera.lock().unwrap();

        // create depth texture if it doesn't exist or if the size has changed
        if self.depth_texture.is_none() || self.depth_texture.as_ref().unwrap().0 != cx.w || self.depth_texture.as_ref().unwrap().1 != cx.h {
            let depth_texture = Res::new(|cx: &mut RenderContext| {
                let texture = cx.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Depth Texture"),
                    size: wgpu::Extent3d {
                        width: cx.w,
                        height: cx.h,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: Self::DEPTH_FORMAT,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                });

                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

                (texture, view)
            });

            self.depth_texture = Some((cx.w, cx.h, depth_texture));
        }

        let depth_texture = &self.depth_texture.as_ref().unwrap().2;
        let depth_texture = cx.resource(&depth_texture);

        let cam = cx.resource(&self.camera_buffer);
        let mut cam = cam.lock().unwrap();
        cam.prepare(cx.queue, camera.deref(), cx.w as f32 / cx.h as f32);

        let surface_info = SurfaceInfo {
            width: cx.w,
            height: cx.h,
            format: cx.target_format.clone(),
            depth_format: Some(Self::DEPTH_FORMAT),
        };

        let mut pass = Pass::new(surface_info, &cam.bind_group, |encoder| encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("My Render Pass"),
            color_attachments: &[
                // This is what @location(0) in the fragment shader targets
                Some(wgpu::RenderPassColorAttachment {
                    view: cx.target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.2,
                            a: 0.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                }),
            ],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_texture.1,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        }));

        //let tri = ctx.resource(&self.triangle);
        //tri.prepare(ctx.device, ctx.queue, self.angle);
        //pass.defer(tri, |rp, globals, t| {
        //    t.render(rp);
        //});

        let mut scene = self.scene.lock().unwrap();

        self.bg.render(cx, &mut pass, scene.background_color());

        camera.render(cx, &mut pass, &self.trackball);

        //pass.defer(|rp| {
        //    rp.set_pipeline(todo!());
        //});

        if scene.grid() {
            self.grid.render(cx, &mut pass);
        }

        scene.raster(cx, &mut pass);

        pass.exec(cx.encoder);

        Vec::new()
    }
}

struct Bg {
    bg_vb: Res<VertexBuffer<flat::Vertex>>,
    bg_ib: Res<Buffer>,
    bg_instance: Res<VertexBuffer<Instance3d>>,
    bg_pipeline: FlatIdentityPipeline,
}

impl Bg {
    pub fn new() -> Self {
        let bg_vb = Res::new(|cx: &mut RenderContext| VertexBuffer::from_slice(
            cx.device,
            &Scene3dBackground::transparent().vertices(),
            None,
        ));

        let bg_instance = Res::new(|cx: &mut RenderContext| VertexBuffer::single(
            cx.device,
            Instance3d::from_placement(&Default::default()),
            None,
        ));

        let bg_pipeline = FlatIdentityPipeline::new(
            PrimitiveTopology::TriangleList,
        );

        const INDICES: &[u16] = &[
            0, 1, 4,
            1, 2, 4,
            2, 3, 4,
            3, 0, 4,
        ];

        let bg_ib = Res::new(|cx: &mut RenderContext| {
            cx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("bg index buffer"),
                contents: crate::external::bytemuck::cast_slice(INDICES),
                usage: wgpu::BufferUsages::INDEX,
            })
        });

        Self {
            bg_vb,
            bg_ib,
            bg_instance,
            bg_pipeline,
        }
    }

    fn render(
        &mut self,
        cx: &mut RenderContext,
        pass: &mut Pass,
        background: Scene3dBackground,
    ) {
        let bg_vb = cx.resource(&self.bg_vb);
        let bg_instance = cx.resource(&self.bg_instance);
        let bg_ib = cx.resource(&self.bg_ib);

        bg_vb.update_from_slice(cx.queue, &background.vertices());

        self.bg_pipeline.render(
            cx,
            pass,
            bg_vb.slice(..),
            bg_instance.slice(..),
            Some(bg_ib),
        );
    }
}



pub struct Grid {
    resources: Res<GridResources>,
}

impl Grid {
    pub fn new(n: u16) -> Self {
        Self {
            resources: Res::new(move |cx: &mut RenderContext| GridResources::new(cx, n)),
        }
    }
}

impl Render for Grid {
    fn render(
        &self,
        cx: &mut RenderContext,
        pass: &mut Pass,
    ) {
        let res = cx.resource(&self.resources);

        res.flat_pipeline.render(
            cx,
            pass,
            res.vertex_buffer.slice(..),
            res.instance_buffer.slice(..),
        );
    }
}

struct GridResources {
    vertex_buffer: VertexBuffer<flat::Vertex>,
    instance_buffer: VertexBuffer<Instance3d>,
    flat_pipeline: FlatPipeline,
}

impl GridResources {
    pub fn new(
        cx: &mut RenderContext,
        n: u16,
    ) -> Self {
        use flat::Vertex;
        let mut vertices: Vec<Vertex> = Vec::new();

        const N_DIV: isize = 5;

        const A: f32 = 0.25;

        for i in -(n as i32)..=(n as i32) {
            let major = i as f32;
            let l = n as f32;

            vertices.push(Vertex {
                position: [-l, 0.0, major],
                color: [0.5, 0.5, 0.5, A],
            });
            vertices.push(Vertex {
                position: [if i != 0 { l } else { 0.0 }, 0.0, major],
                color: [0.5, 0.5, 0.5, A],
            });
            vertices.push(Vertex {
                position: [major, 0.0, -l],
                color: [0.5, 0.5, 0.5, A],
            });
            vertices.push(Vertex {
                position: [major, 0.0, if i != 0 { l } else { 0.0 }],
                color: [0.5, 0.5, 0.5, A],
            });
            if i == 0 {
                vertices.push(Vertex {
                    position: [0.0, 0.0, major],
                    color: [1.0, 0.25, 0.25, 1.0],
                });
                vertices.push(Vertex {
                    position: [l, 0.0, major],
                    color: [1.0, 0.25, 0.25, 1.0],
                });
                vertices.push(Vertex {
                    position: [major, 0.0, 0.0],
                    color: [0.25, 0.25, 1.0, 1.0],
                });
                vertices.push(Vertex {
                    position: [major, 0.0, l],
                    color: [0.25, 0.25, 1.0, 1.0],
                });
            }
    
            if i == n as i32 {
                break;
            }
    
            for minor in 1..N_DIV {
                let t = major + minor as f32 / N_DIV as f32;
                vertices.push(Vertex {
                    position: [-l, 0.0, t],
                    color: [0.25, 0.25, 0.25, A],
                });
                vertices.push(Vertex {
                    position: [l, 0.0, t],
                    color: [0.25, 0.25, 0.25, A],
                });
                vertices.push(Vertex {
                    position: [t, 0.0, -l],
                    color: [0.25, 0.25, 0.25, A],
                });
                vertices.push(Vertex {
                    position: [t, 0.0, l],
                    color: [0.25, 0.25, 0.25, A],
                });
            }
        }

        let vertex_buffer = VertexBuffer::from_slice(
            cx.device,
            &vertices,
            None,
        );

        let instance_buffer = VertexBuffer::single(
            cx.device,
            Instance3d::from_placement(&Placement3::default()),
            None,
        );

        Self {
            vertex_buffer,
            instance_buffer,
            flat_pipeline: FlatPipeline::new(
                wgpu::PrimitiveTopology::LineList,
                wgpu::CompareFunction::Less,
                true,
            ),
        }
    }
}