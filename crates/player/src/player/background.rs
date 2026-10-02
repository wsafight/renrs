use macroquad::prelude::*;
use renrs::syntax::TransformState;

use super::app::App;
use super::ui_common::color_alpha;
use super::{CANVAS_HEIGHT, CANVAS_WIDTH};

impl App {
    pub(super) fn draw_background(&self, path: Option<&str>) {
        self.draw_background_tinted(path, 1.0, TransformState::identity());
    }

    pub(super) fn draw_background_tinted(
        &self,
        path: Option<&str>,
        opacity: f32,
        transform: TransformState,
    ) {
        if path.is_none() || opacity < 1.0 || transform != TransformState::identity() {
            draw_rectangle(
                0.0,
                0.0,
                CANVAS_WIDTH,
                CANVAS_HEIGHT,
                color_alpha(&self.theme.background_color, opacity),
            );
        }
        if let Some(texture) = path.and_then(|path| self.assets.textures.get(path)) {
            let scale = (CANVAS_WIDTH / texture.width()).max(CANVAS_HEIGHT / texture.height());
            let width = texture.width() * scale * transform.scale;
            let height = texture.height() * scale * transform.scale;
            let x = (CANVAS_WIDTH - width) / 2.0 + transform.x;
            let y = (CANVAS_HEIGHT - height) / 2.0 + transform.y;
            draw_texture_ex(
                texture,
                x,
                y,
                Color::new(1.0, 1.0, 1.0, opacity * transform.alpha),
                DrawTextureParams {
                    dest_size: Some(vec2(width, height)),
                    rotation: transform.rotation.to_radians(),
                    pivot: Some(vec2(x + width / 2.0, y + height / 2.0)),
                    ..Default::default()
                },
            );
        }
    }
}
