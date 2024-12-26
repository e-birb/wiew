use std::sync::Arc;

use crate::{Res, ResourceRegistry, SingletonResource};



pub struct RenderContext<'a> {
    pub device: &'a wgpu::Device,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub queue: &'a wgpu::Queue,
    pub target: &'a wgpu::TextureView,
    pub target_format: &'a wgpu::TextureFormat,
    //pub target_formats: &'a [wgpu::TextureFormat],
    //pub depth_formats: &'a [wgpu::TextureFormat],
    pub resource_registry: &'a mut ResourceRegistry,
    pub w: u32,
    pub h: u32,
}

impl<'a> RenderContext<'a> {
    pub fn resource<T: Send + Sync + 'static>(&mut self, res: &Res<T>) -> Arc<T> {
        self.resource_registry.by_id(res.id()).unwrap_or_else(|| {
            let r = res.builder().build(self); // TODO detect cycles

            self.resource_registry.insert(res.clone(), r)
        })
    }

    pub fn singleton<S: SingletonResource>(&mut self) -> Arc<S> {
        self.resource_registry.get_singleton().unwrap_or_else(|| {
            let s = S::init(self); // TODO detect cycles

            self.resource_registry.insert_singleton(s)
        })
    }
}

pub trait View: 'static + Send + Sync {
    fn view(
        &mut self,
        cx: &mut RenderContext,
    ) -> Vec<wgpu::CommandBuffer>;
}