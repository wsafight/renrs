use super::app::App;
use macroquad::prelude::*;
use renrs::runtime::{StageState, VisualEffect};
use renrs::syntax::{TransformState, camera_layer, layer_camera_alias};

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
    pub(super) fn camera_transform_for(
        &self,
        stage: &StageState,
        layer: Option<&str>,
    ) -> TransformState {
        let target_alias = layer.map_or_else(|| "camera".to_owned(), layer_camera_alias);
        let base = stage.camera;
        let layer_camera = layer
            .and_then(|name| stage.layer_cameras.get(name))
            .copied()
            .unwrap_or_default();
        if let Some(renrs::WaitState::Effect {
            effect:
                VisualEffect::Transform {
                    alias: active_alias,
                    from,
                    to,
                    seconds,
                    easing,
                },
        }) = self.runtime.as_ref().and_then(renrs::Runtime::waiting)
            && (active_alias == "camera" || effect_alias_matches(&target_alias, active_alias))
        {
            let progress = if self.theme.reduced_motion || *seconds <= f32::EPSILON {
                1.0
            } else {
                1.0 - self.effect_remaining / seconds
            };
            let animated = from.interpolate(*to, easing.sample(progress));
            if active_alias == "camera" {
                return compose(animated, layer_camera);
            }
            return compose(base, animated);
        }
        compose(base, layer_camera)
    }
}

fn compose(master: TransformState, layer: TransformState) -> TransformState {
    TransformState {
        x: master.x + layer.x,
        y: master.y + layer.y,
        scale: master.scale * layer.scale,
        rotation: master.rotation + layer.rotation,
        alpha: master.alpha * layer.alpha,
        ..layer
    }
}

fn effect_alias_matches(expected: &str, actual: &str) -> bool {
    if expected == "camera" {
        actual == "camera"
    } else {
        camera_layer(actual) == camera_layer(expected)
    }
}
