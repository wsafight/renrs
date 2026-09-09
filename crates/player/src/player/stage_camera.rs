use super::app::App;
use macroquad::prelude::*;
use renrs::runtime::{StageState, VisualEffect};
use renrs::syntax::TransformState;

pub(super) struct StageCamera;
impl StageCamera {
    pub(super) fn new(transform: TransformState, target: Option<&RenderTarget>) -> Self {
        let mut camera = Camera2D::from_display_rect(Rect::new(0.0, 0.0, 1280.0, 720.0));
        camera.zoom *= transform.scale;
        camera.rotation = transform.rotation;
        camera.offset = vec2(transform.x / 640.0, -transform.y / 360.0);
        camera.render_target = target.cloned();
        push_camera_state();
        set_camera(&camera);
        Self
    }
}
impl Drop for StageCamera {
    fn drop(&mut self) {
        pop_camera_state();
    }
}
impl App {
    pub(super) fn camera_transform(&self, stage: &StageState) -> TransformState {
        if let Some(renrs::WaitState::Effect {
            effect:
                VisualEffect::Transform {
                    alias,
                    from,
                    to,
                    seconds,
                    easing,
                },
        }) = self.runtime.as_ref().and_then(renrs::Runtime::waiting)
            && alias == "camera"
        {
            let progress = if self.theme.reduced_motion || *seconds <= f32::EPSILON {
                1.0
            } else {
                1.0 - self.effect_remaining / seconds
            };
            return from.interpolate(*to, easing.sample(progress));
        }
        stage.camera
    }
}
