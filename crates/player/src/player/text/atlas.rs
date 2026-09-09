use super::font::Bitmap;
use etagere::{AtlasAllocator, size2};
use macroquad::prelude::{FilterMode, Image, Rect, Texture2D, pop_camera_state, push_camera_state};

pub(super) const SIDE: u16 = 1024;

pub(super) struct Atlas {
    allocator: AtlasAllocator,
    texture: Texture2D,
}

impl Atlas {
    pub(super) fn new() -> Self {
        let blank = Image::gen_image_color(SIDE, SIDE, macroquad::prelude::BLANK);
        let texture = Texture2D::from_image(&blank);
        texture.set_filter(FilterMode::Linear);
        Self {
            allocator: AtlasAllocator::new(size2(i32::from(SIDE), i32::from(SIDE))),
            texture,
        }
    }

    pub(super) fn insert(&mut self, bitmap: &Bitmap) -> Option<Rect> {
        let allocation = self.allocator.allocate(size2(
            i32::from(bitmap.width) + 2,
            i32::from(bitmap.height) + 2,
        ))?;
        let x = allocation.rectangle.min.x;
        let y = allocation.rectangle.min.y;
        let mut padded = Image::gen_image_color(
            bitmap.width + 2,
            bitmap.height + 2,
            macroquad::prelude::BLANK,
        );
        let stride = usize::from(bitmap.width) * 4;
        for row in 0..usize::from(bitmap.height) {
            let offset = ((row + 1) * usize::from(padded.width) + 1) * 4;
            padded.bytes[offset..offset + stride]
                .copy_from_slice(&bitmap.rgba[row * stride..(row + 1) * stride]);
        }
        self.texture.update_part(
            &padded,
            x,
            y,
            i32::from(padded.width),
            i32::from(padded.height),
        );
        Some(Rect::new(
            (x + 1) as f32,
            (y + 1) as f32,
            f32::from(bitmap.width),
            f32::from(bitmap.height),
        ))
    }

    pub(super) fn clear(&mut self) {
        // Flush queued glyph quads before reusing their texels, preserving the active camera.
        push_camera_state();
        pop_camera_state();
        self.allocator.clear();
    }

    pub(super) fn texture(&self) -> &Texture2D {
        &self.texture
    }
}
