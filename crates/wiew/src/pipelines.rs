
use std::{collections::HashMap, sync::{Arc, Mutex}};


use crate::{Pass, RenderContext, Resource};


pub mod stupid_triangle;
pub mod flat;

pub struct SurfaceFormats {
    pub target_formats: Vec<wgpu::TextureFormat>,
    pub depth_format: Option<wgpu::TextureFormat>,
}

pub struct Pipeline {
    builder: Arc<dyn Fn(&mut RenderContext, &SurfaceFormats) -> wgpu::RenderPipeline + Send + Sync>,
    res: Arc<Mutex<Res>>,
}

struct Res {
    target_formats: Vec<wgpu::TextureFormat>,
    pipelines: HashMap<Option<wgpu::TextureFormat>, Resource<wgpu::RenderPipeline>>,
}

impl Pipeline {
    pub fn from_builder<F>(builder: F) -> Self
    where
        F: Fn(&mut RenderContext, &SurfaceFormats) -> wgpu::RenderPipeline + 'static + Send + Sync,
    {
        let builder = Arc::new(builder);

        let res = Arc::new(Mutex::new(Res {
            target_formats: Vec::new(),
            pipelines: HashMap::new(),
        }));

        Self {
            builder,
            res,
        }
    }

    pub fn get(
        &self,
        cx: &mut RenderContext,
        pass: &mut Pass,
    ) -> Arc<wgpu::RenderPipeline> {
        let mut res = self.res.lock().unwrap();
        let format = &pass.surface_info().format;
        let depth_format = &pass.surface_info().depth_format;

        let ok = res.target_formats.iter().any(|f| f == format);

        if !ok {
            let mut target_formats = res.target_formats.clone();
            target_formats.push(*format);

            *res = Res {
                target_formats: target_formats.clone(),
                pipelines: HashMap::new(),
            };
        }

        let pipeline = match res.pipelines.get(depth_format) {
            Some(pipeline) => pipeline.clone(),
            None => {
                let formats = SurfaceFormats {
                    target_formats: res.target_formats.clone(),
                    depth_format: depth_format.clone(),
                };
    
                let builder = self.builder.clone();
    
                let pipeline = Resource::new(move |cx: &mut RenderContext| builder(cx, &formats));

                res.pipelines.insert(depth_format.clone(), pipeline.clone());

                pipeline
            }
        };

        cx.resource(&pipeline)
    }
}