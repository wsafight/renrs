#[path = "text/atlas.rs"]
mod atlas;
#[path = "text/font.rs"]
mod font;
#[path = "text/shaping.rs"]
mod shaping;

pub(super) use font::Face;
use macroquad::prelude::{Color, DrawTextureParams, Rect, draw_texture_ex, vec2};
use std::cell::RefCell;
use std::collections::HashMap;

const MAX_ENTRIES: usize = 4096;

#[derive(Clone, Copy)]
struct CachedGlyph {
    source: Rect,
    left: f32,
    top: f32,
    scale: f32,
}

struct Renderer {
    faces: Vec<Face>,
    shaped: HashMap<String, std::sync::Arc<[shaping::Glyph]>>,
    shaped_count: usize,
    atlas: Option<atlas::Atlas>,
    glyphs: HashMap<(usize, u16, u16), Option<CachedGlyph>>,
    rasterizations: u64,
    resets: u64,
}

impl Renderer {
    fn new(face: Face) -> Self {
        Self {
            faces: vec![face],
            shaped: HashMap::new(),
            shaped_count: 0,
            atlas: None,
            glyphs: HashMap::new(),
            rasterizations: 0,
            resets: 0,
        }
    }

    fn glyph(&mut self, face: usize, id: ab_glyph::GlyphId, pixels: u16) -> Option<CachedGlyph> {
        let key = (face, id.0, pixels);
        if let Some(glyph) = self.glyphs.get(&key) {
            return *glyph;
        }
        if self.glyphs.len() >= MAX_ENTRIES {
            self.clear();
        }
        let cached = self.faces[face].rasterize(id, pixels).map(|bitmap| {
            self.rasterizations += 1;
            let atlas = self.atlas.get_or_insert_with(atlas::Atlas::new);
            let source = atlas.insert(&bitmap).unwrap_or_else(|| {
                atlas.clear();
                self.glyphs.clear();
                self.resets += 1;
                atlas.insert(&bitmap).expect("bounded glyph fits atlas")
            });
            CachedGlyph {
                source,
                left: bitmap.left,
                top: bitmap.top,
                scale: bitmap.scale,
            }
        });
        self.glyphs.insert(key, cached);
        cached
    }

    fn clear(&mut self) {
        if let Some(atlas) = &mut self.atlas {
            atlas.clear();
        }
        self.glyphs.clear();
        self.resets += 1;
    }

    fn shape(&mut self, text: &str) -> std::sync::Arc<[shaping::Glyph]> {
        if let Some(shaped) = self.shaped.get(text) {
            return shaped.clone();
        }
        let shaped: std::sync::Arc<[shaping::Glyph]> = shaping::shape(&self.faces, text).into();
        if shaped.len() <= 8192 {
            if self.shaped.len() >= 512 || self.shaped_count + shaped.len() > 32768 {
                self.shaped.clear();
                self.shaped_count = 0;
            }
            self.shaped_count += shaped.len();
            self.shaped.insert(text.to_owned(), shaped.clone());
        }
        shaped
    }
}

thread_local! {
    static RENDERER: RefCell<Renderer> = RefCell::new(Renderer::new(Face::fallback()));
}

pub(super) fn install_family(mut faces: Vec<Face>) {
    if faces.is_empty() {
        faces = Face::defaults();
    }
    RENDERER.with_borrow_mut(|renderer| {
        renderer.clear();
        renderer.faces = faces;
        renderer.shaped.clear();
        renderer.shaped_count = 0;
    });
}

pub(super) fn shutdown() {
    RENDERER.with_borrow_mut(|renderer| {
        renderer.glyphs.clear();
        renderer.atlas = None;
    });
}

pub(super) fn measure_width(text: impl AsRef<str>, size: u16) -> f32 {
    RENDERER.with_borrow_mut(|renderer| {
        renderer
            .shape(text.as_ref())
            .iter()
            .map(|glyph| glyph.advance * f32::from(size))
            .sum()
    })
}

pub(super) fn draw_text(text: impl AsRef<str>, x: f32, y: f32, size: f32, tint: Color) {
    if size <= 0.0 || !size.is_finite() {
        return;
    }
    let dpi = macroquad::window::screen_dpi_scale();
    let pixels = (size * dpi).ceil().clamp(1.0, f32::from(u16::MAX)) as u16;
    let units = size / f32::from(pixels);
    RENDERER.with_borrow_mut(|renderer| {
        let mut pen = x;
        for shaped in renderer.shape(text.as_ref()).iter() {
            if let Some(glyph) = renderer.glyph(shaped.face, shaped.id, pixels) {
                draw_texture_ex(
                    renderer.atlas.as_ref().unwrap().texture(),
                    pen + shaped.x * size + glyph.left * units,
                    y + shaped.y * size + glyph.top * units,
                    tint,
                    DrawTextureParams {
                        source: Some(glyph.source),
                        dest_size: Some(vec2(
                            glyph.source.w * glyph.scale * units,
                            glyph.source.h * glyph.scale * units,
                        )),
                        ..Default::default()
                    },
                );
            }
            pen += shaped.advance * size;
        }
    });
}

pub(super) fn stats() -> serde_json::Value {
    RENDERER.with_borrow(|renderer| {
        serde_json::json!({
            "font_bytes": renderer.faces.iter().map(|face| face.bytes).sum::<usize>(),
            "shaped_glyphs": renderer.shaped_count,
            "atlas_bytes": if renderer.atlas.is_some() { usize::from(atlas::SIDE).pow(2) * 4 } else { 0 },
            "cached_glyphs": renderer.glyphs.len(),
            "rasterizations": renderer.rasterizations,
            "atlas_resets": renderer.resets,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_of_large_unseen_text_does_not_allocate_glyphs_or_a_gpu_atlas() {
        let text: String = (0x4e00..0x7000).filter_map(char::from_u32).collect();
        assert!(measure_width(text, 24) > 100_000.0);
        let stats = stats();
        assert_eq!(stats["cached_glyphs"], 0);
        assert_eq!(stats["atlas_bytes"], 0);
        assert_eq!(stats["rasterizations"], 0);
    }
}
