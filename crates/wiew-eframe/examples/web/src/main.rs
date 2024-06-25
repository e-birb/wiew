use std::ops::Deref;
use std::sync::{Arc, Mutex};

use eframe::egui::{self, Color32, Pos2, Stroke};
use eframe::egui::Layout;
use eframe::wgpu::{CompareFunction, PrimitiveTopology};
use wiew_eframe::wiew::external::wgpu;
use wiew_eframe::wiew::external::nalgebra::Vector3;
use wiew_eframe::wiew::external::rotation3::Rotation;
use wiew_eframe::wiew::instance::Instance3d;
use wiew_eframe::wiew::{Pass, Render, Resource};
use wiew_eframe::wiew::{instance::Instance3dBuffer, pipelines::flat::FlatPipeline, RenderContext, VertexBuffer};
use wiew_eframe::wiew::pipelines::flat;
use wiew_eframe::{Eframe3dView, EframeWiewManager, Scene3d, Scene3dBackground};


#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("Hello, world!");

    #[cfg(not(target_arch = "wasm32"))]
    eframe::run_native(
        "Wiew",
        Default::default(),
        Box::new(|cc| Box::new(App::new(cc).unwrap())),
    ).unwrap();
}

#[cfg(target_arch = "wasm32")]
fn main() {
    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    web_sys::console::log_1(&"Hello using web-sys".into());

    let mut web_options = eframe::WebOptions::default();
    web_options.wgpu_options = egui_wgpu::WgpuConfiguration {
        supported_backends: wgpu::Backends::GL | wgpu::Backends::BROWSER_WEBGPU,
        //supported_backends: wgpu::Backends::GL,
        //device_descriptor: std::sync::Arc::new(|adapter| {
        //    Default::default()
        //}),
        ..Default::default()
    };
    web_options.depth_buffer = 0;

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "my-canvas", // hardcode it
                web_options,
                Box::new(|cc| Box::new(App::new(cc).unwrap())),
            )
            .await
            .expect("failed to start eframe");
    });
}

struct App {
    wiew: Eframe3dView,
    settings: Arc<Mutex<Settings>>,
}

impl App {
    pub fn new<'a>(cc: &'a eframe::CreationContext<'a>) -> Option<Self> {
        let wgpu_render_state = cc.wgpu_render_state.as_ref().expect("no wgpu_render_state, did you set eframe::Renderer::Wgpu?");
        let resources = EframeWiewManager::new(wgpu_render_state.target_format);

        // Because the graphics pipeline must have the same lifetime as the egui render pass,
        // instead of storing the pipeline in our `Custom3D` struct, we insert it into the
        // `paint_callback_resources` type map, which is stored alongside the render pass.
        wgpu_render_state
            .renderer
            .write()
            .callback_resources
            .insert(resources);

        let settings = Arc::new(Mutex::new(Settings {
            grid: true,
            bg_top_left: Color32::from_rgba_premultiplied(14, 41, 29, 255),
            bg_tot_right: Color32::from_rgba_premultiplied(54, 22, 22, 255),
            bg_bottom_left: Color32::from_rgba_premultiplied(20, 17, 51, 255),
            bg_bottom_right: Color32::from_rgba_premultiplied(42, 20, 55, 255),
        }));

        /*
        6, 38, 24, 255                74, 0, 0, 255
        10, 0, 67, 255                97, 33, 127, 255
        */

        let wiew = Eframe3dView::new(MyScene::new(settings.clone()));

        Some(Self {
            //scene,
            wiew,
            settings,
        })
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                let mut settings = self.settings.lock().unwrap();
                ui.color_edit_button_srgba(&mut settings.bg_top_left);
                ui.color_edit_button_srgba(&mut settings.bg_tot_right);
            });
            ui.horizontal(|ui| {
                let mut settings = self.settings.lock().unwrap();
                ui.color_edit_button_srgba(&mut settings.bg_bottom_left);
                ui.color_edit_button_srgba(&mut settings.bg_bottom_right);
            });
            ui.checkbox(&mut self.settings.lock().unwrap().grid, "grid");
            ui.with_layout(Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
            //ui.with_layout(Layout::top_down_justified(egui::Align::Center), |ui| {
                egui::Frame::canvas(ui.style()).show(ui, |ui| {
                    self.wiew.paint(ui);
                });
            });
        });
    }
}

struct Settings {
    grid: bool,
    bg_top_left: egui::Color32,
    bg_tot_right: egui::Color32,
    bg_bottom_left: egui::Color32,
    bg_bottom_right: egui::Color32,
}

struct MyScene {
    settings: Arc<Mutex<Settings>>,
    triangle: Resource<MyShape>,
}

impl MyScene {
    fn new(settings: Arc<Mutex<Settings>>) -> Self {
        Self {
            settings,
            triangle: Resource::new(MyShape::new),
        }
    }
}

impl Scene3d for MyScene {
    fn raster(
        &mut self,
        cx: &mut RenderContext,
        pass: &mut Pass,
    ) {
        let triangle = cx.resource(&self.triangle);
        triangle.render(cx, pass);
    }

    fn grid(&self) -> bool {
        self.settings.lock().unwrap().grid
    }

    fn background_color(&self) -> wiew_eframe::Scene3dBackground {
        let settings = self.settings.lock().unwrap();
        let to_array = |c: Color32| [c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0, c.a() as f32 / 255.0];
        Scene3dBackground {
            top_left: to_array(settings.bg_top_left),
            top_right: to_array(settings.bg_tot_right),
            bottom_left: to_array(settings.bg_bottom_left),
            bottom_right: to_array(settings.bg_bottom_right),
        }
    }
}

struct MyShape {
    vb: VertexBuffer<flat::Vertex>,
    ib: Instance3dBuffer,
    pipeline: FlatPipeline,
}

impl MyShape {
    fn new(cx: &mut RenderContext) -> Self {
        use flat::Vertex;

        let vertices = {
            let mut vertices: Vec<Vertex> = Vec::new();

            let div_a = 100;
            let div_b = 100;

            let n = 3;
            let m = 4;

            let f = |u: f32, v: f32| {
                let x = u.cos() * (2.0 + (u * n as f32 + v * m as f32).cos());
                let y = u.sin() * (2.0 + (u * n as f32 + v * m as f32).cos());
                let p = Vector3::new(x + 5.0, y, 0.0);
                let r = Rotation::from_components_array([0.0, v, 0.0]);
                let p = r.rotate_vector(p) * 0.125;

                let r = v.cos().abs();
                let g = (v.sin() * u.cos()).abs();
                let b = (v.sin() * u.sin()).abs();

                Vertex {
                    position: p.into(),
                    color: [
                        r, g, b,
                        1.0,
                    ],
                }
            };

            for i in 0..div_a {
                for j in 0..div_b {
                    let u = i as f32 * std::f32::consts::PI * 2.0 / div_a as f32;
                    let u_1 = (i + 1) as f32 * std::f32::consts::PI * 2.0 / div_a as f32;
                    let v = j as f32 * std::f32::consts::PI * 2.0 / div_b as f32;
                    let v_1 = (j + 1) as f32 * std::f32::consts::PI * 2.0 / div_b as f32;

                    vertices.push(f(u, v));
                    vertices.push(f(u_1, v));
                    vertices.push(f(u_1, v_1));
                    vertices.push(f(u, v));
                    vertices.push(f(u_1, v_1));
                    vertices.push(f(u, v_1));
                }
            }

            vertices
        };

        println!("vertices: {} ({} triangles)", vertices.len(), vertices.len() / 3);

        let vb = VertexBuffer::from_slice(
            cx.device,
            &vertices,
            //&[
            //    Vertex { position: [0.0, 0.5, 0.0], color: [1.0, 0.0, 0.0, 1.0] },
            //    Vertex { position: [-0.5, -0.5, 0.0], color: [0.0, 1.0, 0.0, 1.0] },
            //    Vertex { position: [0.5, -0.5, 0.0], color: [0.0, 0.0, 1.0, 1.0] },
            //],
            None,
        );

        let ib = Instance3dBuffer::single(
            cx.device,
            Instance3d::from_placement(&Default::default()),
            None,
        );

        let pipeline = FlatPipeline::new(
            PrimitiveTopology::TriangleList,
            CompareFunction::LessEqual,
            true,
        );

        Self { vb, ib, pipeline }
    }
}

impl Render for MyShape {
    fn render(
        &self,
        cx: &mut RenderContext,
        pass: &mut Pass,
    ) {
        self.pipeline.render(
            cx,
            pass,
            &self.vb,
            &self.ib,
        );
    }
}