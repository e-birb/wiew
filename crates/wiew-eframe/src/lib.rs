
use std::sync::{Arc, Mutex};

use provided::{MyView3d, Scene3d};
pub use wiew;

use wiew::*;
use wiew::external::wgpu;

use eframe::{egui::{PointerButton, Sense}, egui_wgpu::{CallbackTrait, ScreenDescriptor}};

use crate::{RenderContext, TrackballCamera, View};

mod presentation;
mod manager;

pub use presentation::*;
pub use manager::*;

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

                if on_rect && r.hovered() {
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

    pub fn paint(&self, ui: &mut eframe::egui::Ui, rect: eframe::egui::Rect) {
        let _idx = ui.painter().add(eframe::egui_wgpu::Callback::new_paint_callback(
            rect,
            self.paint_callback(rect.width() as u32, rect.height() as u32),
        ));
    }

    fn paint_callback(&self, width: u32, height: u32) -> EframeWiewCallback {
        EframeWiewCallback::new(
            self.id.clone(),
            self.view.clone(),
            width,
            height,
        )
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

        let mut registry = registry.lock();

        let mut ctx = RenderContext {
            device,
            encoder: egui_encoder,
            queue,
            target: &r.render_texture_view,
            target_format: &r.presentation._target_format,
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






