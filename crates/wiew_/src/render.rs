use crate::*;



pub trait Render: 'static + Send + Sync {
    fn render(
        &self,
        cx: &mut RenderContext,
        pass: &mut Pass,
    );
}