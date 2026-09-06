use super::app::App;
use macroquad::prelude::*;
use renrs::syntax::{LayeredImage, TransformState};

pub(super) struct Placement {
    pub(super) position: Vec2,
    pub(super) size: Vec2,
    pub(super) pivot: Vec2,
    pub(super) opacity: f32,
}

impl App {
    pub(super) fn draw_image_layers(
        &self,
        image: &LayeredImage,
        transform: TransformState,
        placement: Placement,
    ) {
        let crop = transform.crop.map_or(
            Rect::new(0.0, 0.0, image.width as f32, image.height as f32),
            |crop| Rect::new(crop.x, crop.y, crop.width, crop.height),
        );
        let scale = placement.size / vec2(crop.w, crop.h);
        for layer in &image.layers {
            let path = if self.theme.reduced_motion {
                &layer.path
            } else {
                layer.frame(self.play_time_seconds, self.audio.voice_busy())
            };
            let Some(texture) = self.assets.textures.get(path) else {
                continue;
            };
            let Some(visible) = crop.intersect(Rect::new(
                layer.x,
                layer.y,
                texture.width(),
                texture.height(),
            )) else {
                continue;
            };
            let position =
                placement.position + vec2(visible.x - crop.x, visible.y - crop.y) * scale;
            draw_texture_ex(
                texture,
                position.x,
                position.y,
                Color::new(1.0, 1.0, 1.0, transform.alpha * placement.opacity),
                DrawTextureParams {
                    dest_size: Some(vec2(visible.w, visible.h) * scale),
                    source: Some(Rect::new(
                        visible.x - layer.x,
                        visible.y - layer.y,
                        visible.w,
                        visible.h,
                    )),
                    rotation: transform.rotation.to_radians(),
                    pivot: Some(placement.pivot),
                    ..Default::default()
                },
            );
        }
    }
}
