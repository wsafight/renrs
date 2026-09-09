use super::CANVAS_WIDTH;
use super::app::App;
use macroquad::prelude::*;
use renrs::runtime::{StageState, VisualEffect};
use renrs::{Runtime, WaitState};

impl App {
    pub(super) fn draw_fade_overlay(&self, seconds: f32) {
        let alpha = if self.theme.reduced_motion || seconds <= f32::EPSILON {
            0.0
        } else {
            (self.effect_remaining / seconds).clamp(0.0, 1.0)
        };
        draw_rectangle(
            self.theme.layout.toolbar.x,
            self.theme.layout.toolbar.y + self.theme.layout.toolbar.height,
            self.theme.layout.toolbar.width,
            super::CANVAS_HEIGHT - self.theme.layout.toolbar.y - self.theme.layout.toolbar.height,
            Color::new(0.0, 0.0, 0.0, alpha),
        );
    }

    pub(super) fn draw_composite_stage(&self, stage: &StageState) -> bool {
        let Some(WaitState::Effect { effect }) = self.runtime.as_ref().and_then(Runtime::waiting)
        else {
            return false;
        };
        match effect {
            VisualEffect::Push {
                from,
                left,
                seconds,
            } => {
                let progress = self.effect_progress(*seconds);
                let width = CANVAS_WIDTH;
                let direction = if *left { -1.0 } else { 1.0 };
                let offset = direction * width * progress;
                self.draw_stage_shifted(from, 1.0, vec2(offset, 0.0));
                self.draw_stage_shifted(
                    stage,
                    1.0,
                    vec2(-direction * width * (1.0 - progress), 0.0),
                );
                true
            }
            VisualEffect::Wipe {
                from,
                left,
                seconds,
            } => {
                let progress = self.effect_progress(*seconds);
                self.draw_stage_tinted(from, 1.0);
                let width = CANVAS_WIDTH * progress;
                let x = if *left { CANVAS_WIDTH - width } else { 0.0 };
                if width <= f32::EPSILON {
                    return true;
                }
                if let Some(target) = &self.transition_target {
                    push_camera_state();
                    let mut camera = Camera2D::from_display_rect(Rect::new(
                        0.0,
                        0.0,
                        CANVAS_WIDTH,
                        super::CANVAS_HEIGHT,
                    ));
                    camera.render_target = Some(target.clone());
                    set_camera(&camera);
                    self.draw_stage_on(stage, 1.0, Some(target));
                    pop_camera_state();
                    draw_texture_ex(
                        &target.texture,
                        x,
                        0.0,
                        WHITE,
                        DrawTextureParams {
                            dest_size: Some(vec2(width, super::CANVAS_HEIGHT)),
                            source: Some(Rect::new(x, 0.0, width, super::CANVAS_HEIGHT)),
                            flip_y: true,
                            ..Default::default()
                        },
                    );
                } else {
                    self.draw_stage_tinted(stage, 1.0);
                }
                true
            }
            VisualEffect::Punch { vertical, seconds } => {
                let progress = self.effect_progress(*seconds);
                let displacement =
                    (progress * std::f32::consts::TAU * 3.0).sin() * (1.0 - progress) * 18.0;
                let offset = if *vertical {
                    vec2(0.0, displacement)
                } else {
                    vec2(displacement, 0.0)
                };
                self.draw_stage_shifted(stage, 1.0, offset);
                true
            }
            _ => false,
        }
    }

    pub(super) fn effect_progress(&self, seconds: f32) -> f32 {
        if seconds <= f32::EPSILON || self.theme.reduced_motion {
            1.0
        } else {
            (1.0 - (self.effect_remaining / seconds).clamp(0.0, 1.0)).clamp(0.0, 1.0)
        }
    }
}
