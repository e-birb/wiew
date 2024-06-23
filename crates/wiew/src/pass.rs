use std::any::Any;

use wgpu::{BindGroup, CommandEncoder, RenderPass};

/// A pass that can be executed on a render surface.
///
/// Usually, in wgpu, you will prepare the necessary resources for rendering
/// (such as buffers, textures, etc.) and then create a render pass to render
/// to a surface. This requires you to have two blocks of code: one to prepare
/// the resources and one to use the render pass.  
/// Matching this two phases (making sure you prepared the right resources and
/// used them consistently) is not "developer friendly".  
/// This struct aims to solve this problem by allowing you to defer the render commands.
///
/// # Example
/// TODO ...
pub struct Pass<'a> {
    surface_info: SurfaceInfo,
    pub globals: &'a wgpu::BindGroup,
    descriptor: Option<Box<dyn FnOnce(&'a mut CommandEncoder) -> RenderPass<'a> + 'a>>,
    steps: Vec<(Box<dyn Any + Send + Sync>, Box<dyn for<'rp> Fn(&mut wgpu::RenderPass<'rp>, &'rp wgpu::BindGroup, &'rp dyn Any) + 'a>)>,
}

impl<'a> Pass<'a> {
    pub fn new(
        surface_info: SurfaceInfo,
        camera_bind_group: &'a wgpu::BindGroup,
        descriptor: impl FnOnce(&'a mut CommandEncoder,
    ) -> RenderPass<'a> + 'a) -> Self {
        Self {
            surface_info,
            globals: camera_bind_group,
            descriptor: Some(Box::new(descriptor)),
            steps: Vec::new(),
        }
    }

    pub fn surface_info(&self) -> &SurfaceInfo {
        &self.surface_info
    }

    /// Execute the pass.
    ///
    /// This will consume the pass and execute the deferred render commands.
    ///
    /// # Remarks
    /// This method has to be explicitly called, otherwise the recorded commands
    /// will not be executed.
    pub fn exec(mut self, encoder: &'a mut CommandEncoder) {
        let mut render_pass = (self.descriptor.take().unwrap())(encoder);
        for (data, step) in &self.steps {
            step(&mut render_pass, &self.globals, &**data);
        }
    }

    /// Defer a render command.
    ///
    /// A render command is a closure that takes:
    /// - a [`RenderPass`]: the [wgpu] render pass
    /// - the [`BindGroup`] relative to the global resources
    /// - the data that was passed to this method
    ///
    /// # Arguments
    /// - `data`: the data that will be passed to the closure, use this to store
    ///   the necessary data for the render command
    /// - `command`: the render command
    ///
    /// # Example
    /// ```no_run
    /// # use wgpu::*;
    /// # use wiew::*;
    /// # let mut pass = unreachable!();
    /// # let texture: () = unreachable!();
    /// pass.defer(
    ///     texture,
    ///     |rp, globals, texture| {
    ///         // ...
    ///     },
    /// );
    /// ```
    pub fn defer<Data: 'static + Send + Sync, F>(&mut self, data: Data, command: F)
    where
        F: for<'rp> Fn(&mut wgpu::RenderPass<'rp>, &'rp BindGroup, &'rp Data) + 'static
    {
        let data: Box<dyn Any + Send + Sync> = Box::new(data);

        self.steps.push((
            data,
            Box::new(move |render_pass: &mut wgpu::RenderPass, globals: &wgpu::BindGroup, data: &dyn Any| {
                let data = <dyn Any>::downcast_ref::<Data>(data as &dyn Any).unwrap();
                command(render_pass, globals, data);
            }),
        ));
    }
}

impl<'a> Drop for Pass<'a> {
    fn drop(&mut self) {
        if self.descriptor.is_some() {
            log::error!("Pass dropped but without being `Pass::exec`-ed");
        }
    }
}

/// Information about the surface that the pass will render to.
pub struct SurfaceInfo {
    /// The width of the surface.
    pub width: u32,
    /// The height of the surface.
    pub height: u32,
    /// The format of the surface.
    pub format: wgpu::TextureFormat,
    /// The depth format of the surface, if it has one.
    pub depth_format: Option<wgpu::TextureFormat>,
}