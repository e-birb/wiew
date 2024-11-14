
use std::{collections::HashMap, ops::Deref, sync::{Arc, Mutex}};

pub use wiew;

use wiew::*;
use wiew::external::wgpu;
use wiew::external::rotation3::*;
use wiew::pipelines::flat::{self, FlatIdentityPipeline, FlatPipeline};

use eframe::{egui::{PointerButton, Sense}, egui_wgpu::{CallbackTrait, RenderState, ScreenDescriptor}, wgpu::{util::DeviceExt, Buffer}};
use wgpu::{ColorTargetState, PrimitiveTopology, TextureFormat};

use crate::{instance::Instance3d, Pass, ProjectionCameraBuffer, RenderContext, Resource, ResourceRegistry, SurfaceInfo, Trackball, TrackballCamera, VertexBuffer, View};

pub struct Eframe3dView {
    eframe_view: EframeView,
    camera: Arc<Mutex<TrackballCamera>>,
}

impl Eframe3dView {
    pub fn new(
        //render_state: &RenderState,
        scene: impl Scene3d,
    ) -> Self {
        let camera = Arc::new(Mutex::new(TrackballCamera::new()));

        let eframe_view = EframeView::new(MyView3d::new(scene, camera.clone()));
        Self {
            eframe_view,
            camera,
        }
    }

    pub fn paint(&self, ui: &mut eframe::egui::Ui) {
        use eframe::egui;

        let (rect, r) =  ui.allocate_at_least(
            egui::Vec2::new(10.0, 10.0),
            Sense::drag().union(Sense::focusable_noninteractive()).union(Sense::click()),
        );

        {
            let mut camera = self.camera.lock().unwrap();

            if r.dragged_by(PointerButton::Primary) {
                let delta = r.drag_delta();
                ui.input(|i| {
                    if let Some(pos) = i.pointer.latest_pos() {
                        let pos = pos - rect.min;
                        camera.mouse_rotation(
                            (pos.x, pos.y),
                            (pos.x + delta.x as f32, pos.y + delta.y as f32),
                            rect.width(),
                            rect.height(),
                        );
                    }
                });
            }
    
            if r.dragged_by(PointerButton::Secondary) {
                let delta = r.drag_delta();
                ui.input(|i| {
                    if let Some(pos) = i.pointer.latest_pos() {
                        let pos = pos - rect.min;
                        camera.mouse_pan(
                            (pos.x, pos.y),
                            (pos.x + delta.x as f32, pos.y + delta.y as f32),
                            rect.width(),
                            rect.height(),
                        );
                    }
                });
            }

            ui.input(|i| {
                let on_rect = i.pointer.hover_pos().map(|p| rect.contains(p)).unwrap_or(false);

                if on_rect {
                    let dy = i.smooth_scroll_delta.y;
                    camera.mouse_zoom(- dy as f32 / 200.0);
                    let dx = i.smooth_scroll_delta.x;
                    camera.mouse_roll(dx as f32 / 200.0);

                    if let Some(m) = i.multi_touch() {
                        let d = m.zoom_delta;
                        camera.mouse_zoom_r(d); // TODO proper scaling
                    }
                }
            });
        }

        self.eframe_view.paint(ui, rect);
    }
}

pub struct EframeView {
    id: Arc<()>,
    view: Arc<Mutex<Box<dyn View>>>,
}

impl EframeView {
    pub fn new(
        //render_state: &RenderState,
        view: impl View,
    ) -> Self {
        Self {
            id: Arc::new(()),
            view: Arc::new(Mutex::new(Box::new(view))),
        }
    }

    pub fn paint_callback(&self, width: u32, height: u32) -> EframeWiewCallback {
        EframeWiewCallback::new(
            self.id.clone(),
            self.view.clone(),
            width,
            height,
        )
    }

    pub fn paint(&self, ui: &mut eframe::egui::Ui, rect: eframe::egui::Rect) {
        let _idx = ui.painter().add(eframe::egui_wgpu::Callback::new_paint_callback(
            rect,
            self.paint_callback(rect.width() as u32, rect.height() as u32),
        ));
    }
}

pub struct EframeWiewCallback {
    id: Arc<()>,
    width: u32,
    height: u32,
    view: Arc<Mutex<Box<dyn View>>>,
}

impl EframeWiewCallback {
    pub fn new(
        id: Arc<()>,
        view: Arc<Mutex<Box<dyn View>>>,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            id,
            width,
            height,
            view,
        }
    }
}

impl CallbackTrait for EframeWiewCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &ScreenDescriptor,
        egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut eframe::egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let (r, registry) = {
            let manager: &mut EframeWiewManager = callback_resources.get_mut().expect("no manager");
            manager.cleanup(); // TODO why here??

            manager.get_or_create_eframe_view_resources(&self.id, device, self.width, self.height);

            (
                manager.get_eframe_view_resources(&self.id).expect("no resources"),
                &manager.resource_registry,
            )
        };

        let mut v = self.view.lock().unwrap();

        let mut registry = registry.lock().unwrap();

        let mut ctx = RenderContext {
            device,
            encoder: egui_encoder,
            queue,
            target: &r.render_texture_view,
            presentation_target_format: &r.presentation._target_format,
            //target_formats: &[r.presentation._target_format],
            //depth_formats: &[],
            resource_registry: &mut registry,
            w: self.width,
            h: self.height,
        };

        v.view(&mut ctx)
    }

    fn finish_prepare(
        &self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _egui_encoder: &mut wgpu::CommandEncoder,
        _callback_resources: &mut eframe::egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        Vec::new()
    }

    fn paint(
        &self,
        _info: eframe::epaint::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass,
        callback_resources: &eframe::egui_wgpu::CallbackResources,
    ) {
        // TODO assert correct size
        // TODO use the clip rect

        let resources = callback_resources
            .get::<EframeWiewManager>().expect("no manager")
            .get_eframe_view_resources(&self.id).expect("no resources");

        render_pass.set_pipeline(&resources.presentation.pipeline);
        render_pass.set_bind_group(0, &resources.presentation.render_texture_bind_group, &[]);
        render_pass.draw(0..4, 0..1);
    }
}

struct EframeWiewResources {
    _render_texture: wgpu::Texture,
    render_texture_view: wgpu::TextureView,
    current_width: u32,
    current_height: u32,
    presentation: PresentationStuff,
}

impl EframeWiewResources {
    fn new(
        device: &wgpu::Device,
        texture_format: wgpu::TextureFormat,
        target_format: &wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        let render_texture = Self::create_texture(
            device,
            texture_format,
            width,
            height,
        );

        let render_texture_view = render_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let presentation = PresentationStuff::new(device, &render_texture_view, target_format);

        Self {
            _render_texture: render_texture,
            render_texture_view,
            current_width: width,
            current_height: height,
            presentation,
        }
    }

    fn create_texture(
        device: &wgpu::Device,
        texture_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> wgpu::Texture {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture_desc = wgpu::TextureDescriptor {
            label: Some("render texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: texture_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[texture_format],
        };

        device.create_texture(&texture_desc)
    }

    fn update_size(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.current_width != width || self.current_height != height {
            self._render_texture = Self::create_texture(device, self.presentation._target_format, width, height);
            self.render_texture_view = self._render_texture.create_view(&wgpu::TextureViewDescriptor::default());
            self.current_width = width;
            self.current_height = height;
            self.presentation = PresentationStuff::new(device, &self.render_texture_view, &self.presentation._target_format);
        }
    }
}


/// Manages the resources for the eframe views
///
/// # Remarks
/// You have to call [`init`] during the initialization of the app
/// and [`begin_frame`] at the beginning of [`App::update`](eframe::App::update)!  
/// Failing to call [`init`] will result in a **panic** and failing to call [`begin_frame`] will result in **resource leaks**.
///
/// [`init`]: EframeWiewManager::init
/// [`begin_frame`]: EframeWiewManager::begin_frame
pub struct EframeWiewManager {
    render_textures: HashMap<usize, (std::sync::Weak<()>, EframeWiewResources)>,
    target_format: wgpu::TextureFormat,
    pub resource_registry: Arc<Mutex<ResourceRegistry>>,
}

impl EframeWiewManager {
    fn new(target_format: TextureFormat) -> Self {
        Self {
            render_textures: HashMap::new(),
            target_format,
            resource_registry: Arc::new(Mutex::new(ResourceRegistry::new())),
        }
    }

    pub fn init(cc: &eframe::CreationContext) {
        let wgpu_render_state = cc.wgpu_render_state.as_ref().expect("no wgpu_render_state, did you set eframe::Renderer::Wgpu?");
        let resources = EframeWiewManager::new(wgpu_render_state.target_format);

        // TODO do nothing but log if already initialized... or maybe replace?

        wgpu_render_state
            .renderer
            .write()
            .callback_resources
            .insert(resources);
    }

    fn clean_resources(&mut self) {
        self.resource_registry.lock().unwrap().clean();
    }

    pub fn begin_frame(render_state: &eframe::egui_wgpu::RenderState) {
        render_state
            .renderer
            .write()
            .callback_resources
            .get_mut::<EframeWiewManager>()
            .expect("no manager")
            .clean_resources();
    }

    fn cleanup(&mut self) {
        // remove all refs that have no other refs
        self.render_textures.retain(|_, (weak, _)| weak.strong_count() > 0);
    }

    fn get_eframe_view_resources(
        &self,
        id: &Arc<()>,
    ) -> Option<&EframeWiewResources> {
        self.render_textures.get(&(Arc::as_ptr(id) as usize)).map(|(_, r)| r)
    }

    fn get_or_create_eframe_view_resources(
        &mut self,
        id: &Arc<()>,
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> &EframeWiewResources {
        let (weak, r) = self
            .render_textures
            .entry(Arc::as_ptr(id) as usize)
            .or_insert_with(|| {
                let stuff = EframeWiewResources::new(
                    device,
                    self.target_format.clone(), // TODO ???
                    &self.target_format,
                    width,
                    height,
                );

                (Arc::downgrade(id), stuff)
            });

        r.update_size(device, width, height);

        r
    }
}

struct PresentationStuff {
    render_texture_bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    _target_format: wgpu::TextureFormat, // TODO use
}

impl PresentationStuff {
    pub fn new(
        device: &wgpu::Device,
        texture_view: &wgpu::TextureView,
        target_format: &wgpu::TextureFormat,
    ) -> Self {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let render_texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("render texture bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let render_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("render texture bind group"),
            layout: &render_texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let present_shader = device.create_shader_module(wgpu::include_wgsl!("present.wgsl"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("present pipeline layout"),
            bind_group_layouts: &[&render_texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let mut primitive = wgpu::PrimitiveState::default();
        primitive.topology = wgpu::PrimitiveTopology::TriangleStrip;

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("present pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &present_shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &present_shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: *target_format,
                    //blend: Some(wgpu::BlendState::REPLACE),
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                    
                })],
                compilation_options: Default::default(),
            }),
            primitive,
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            render_texture_bind_group,
            pipeline,
            _target_format: *target_format,
        }
    }
}

pub struct Scene3dBackground {
    pub top_left: [f32; 4],
    pub top_right: [f32; 4],
    pub bottom_left: [f32; 4],
    pub bottom_right: [f32; 4],
}

impl Scene3dBackground {
    pub const DEFAULT_BG: Self = Self::vertical_gradient(
        [0x1c as f32 / 255.0, 0x1c as f32 / 255.0, 0x1c as f32 / 255.0, 1.0],
        [0x26 as f32 / 255.0, 0x19 as f32 / 255.0, 0x38 as f32 / 255.0, 1.0],
    );

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

struct Bg {
    bg_vb: Resource<VertexBuffer<flat::Vertex>>,
    bg_ib: Resource<Buffer>,
    bg_instance: Resource<VertexBuffer<Instance3d>>,
    bg_pipeline: FlatIdentityPipeline,
}

impl Bg {
    pub fn new() -> Self {
        let bg_vb = Resource::new(|cx: &mut RenderContext| VertexBuffer::from_slice(
            cx.device,
            &Scene3dBackground::transparent().vertices(),
            None,
        ));

        let bg_instance = Resource::new(|cx: &mut RenderContext| VertexBuffer::single(
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

        let bg_ib = Resource::new(|cx: &mut RenderContext| {
            cx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("bg index buffer"),
                contents: wiew::external::bytemuck::cast_slice(INDICES),
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

pub trait Scene3d: 'static + Send + Sync {
    fn raster(
        &mut self,
        cx: &mut RenderContext,
        pass: &mut Pass,
    );

    fn background_color(&self) -> Scene3dBackground {
        Scene3dBackground::DEFAULT_BG
    }

    fn grid(&self) -> bool {
        true
    }
}

pub struct MyView3d {
    angle: f32,
    camera: Arc<Mutex<TrackballCamera>>,
    focused: bool,

    depth_texture: Option<(u32, u32, Resource<(wgpu::Texture, wgpu::TextureView)>)>,

    camera_buffer: Resource<Mutex<ProjectionCameraBuffer>>,
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
            angle: 0.0,
            camera,
            focused: false,
            depth_texture: None,
            camera_buffer: Resource::new(|cx: &mut RenderContext| Mutex::new(ProjectionCameraBuffer::new(cx))),
            //triangle: Resource::new(move |cx: &mut wiew::RenderContext| stupid_triangle::Triangle::new(cx, &[presentation_target_format])),
            trackball: Trackball::new(),
            bg: Bg::new(),
            grid: Grid::new(),
            scene: Mutex::new(Box::new(scene)),
        }
    }
}

impl View for MyView3d {
    fn view(
        &mut self,
        cx: &mut RenderContext,
    ) -> Vec<wgpu::CommandBuffer> {
        //self.angle += 0.1;

        let camera = self.camera.lock().unwrap();

        // create depth texture if it doesn't exist or if the size has changed
        if self.depth_texture.is_none() || self.depth_texture.as_ref().unwrap().0 != cx.w || self.depth_texture.as_ref().unwrap().1 != cx.h {
            let depth_texture = Resource::new(|cx: &mut RenderContext| {
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
            format: cx.presentation_target_format.clone(),
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

pub struct Grid {
    resources: Resource<GridResources>,
}

impl Grid {
    pub fn new() -> Self {
        Self {
            resources: Resource::new(|cx: &mut RenderContext| GridResources::new(cx)),
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
    ) -> Self {
        use flat::Vertex;
        let mut vertices: Vec<Vertex> = Vec::new();

        const N: isize = 10;
        const N_DIV: isize = 5;

        const A: f32 = 0.25;

        for major in -N..=N {
            vertices.push(Vertex {
                position: [-N as f32, 0.0, major as f32],
                color: [0.5, 0.5, 0.5, A],
            });
            vertices.push(Vertex {
                position: [if major != 0 { N as f32 } else { 0.0 }, 0.0, major as f32],
                color: [0.5, 0.5, 0.5, A],
            });
            vertices.push(Vertex {
                position: [major as f32, 0.0, -N as f32],
                color: [0.5, 0.5, 0.5, A],
            });
            vertices.push(Vertex {
                position: [major as f32, 0.0, if major != 0 { N as f32 } else { 0.0 }],
                color: [0.5, 0.5, 0.5, A],
            });
            if major == 0 {
                vertices.push(Vertex {
                    position: [0.0, 0.0, major as f32],
                    color: [1.0, 0.25, 0.25, 1.0],
                });
                vertices.push(Vertex {
                    position: [N as f32, 0.0, major as f32],
                    color: [1.0, 0.25, 0.25, 1.0],
                });
                vertices.push(Vertex {
                    position: [major as f32, 0.0, 0.0],
                    color: [0.25, 0.25, 1.0, 1.0],
                });
                vertices.push(Vertex {
                    position: [major as f32, 0.0, N as f32],
                    color: [0.25, 0.25, 1.0, 1.0],
                });
            }
    
            if major == N {
                break;
            }
    
            for minor in 1..N_DIV {
                let t = major as f32 + minor as f32 / N_DIV as f32;
                vertices.push(Vertex {
                    position: [-N as f32, 0.0, t],
                    color: [0.25, 0.25, 0.25, A],
                });
                vertices.push(Vertex {
                    position: [N as f32, 0.0, t],
                    color: [0.25, 0.25, 0.25, A],
                });
                vertices.push(Vertex {
                    position: [t, 0.0, -N as f32],
                    color: [0.25, 0.25, 0.25, A],
                });
                vertices.push(Vertex {
                    position: [t, 0.0, N as f32],
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