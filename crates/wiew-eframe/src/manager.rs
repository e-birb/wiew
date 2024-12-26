use std::sync::Arc;

use eframe::{egui::{ahash::{HashMap, HashMapExt}, mutex::Mutex}, wgpu::{self, TextureFormat}};
use wiew::ResourceRegistry;

use crate::PresentationStuff;

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
        self.resource_registry.lock().clean();
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

    pub fn cleanup(&mut self) {
        // remove all refs that have no other refs
        self.render_textures.retain(|_, (weak, _)| weak.strong_count() > 0);
    }

    pub(crate) fn get_eframe_view_resources(
        &self,
        id: &Arc<()>,
    ) -> Option<&EframeWiewResources> {
        self.render_textures.get(&(Arc::as_ptr(id) as usize)).map(|(_, r)| r)
    }

    pub(crate) fn get_or_create_eframe_view_resources(
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

pub(crate) struct EframeWiewResources {
    _render_texture: wgpu::Texture,
    pub(crate) render_texture_view: wgpu::TextureView,
    current_width: u32,
    current_height: u32,
    pub(crate) presentation: PresentationStuff,
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