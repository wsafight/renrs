use ab_glyph::{Font, FontArc, FontRef, GlyphId};

const MAX_FONT_BYTES: usize = 32 * 1024 * 1024;
pub(super) const MAX_GLYPH_SIDE: u16 = 1020;
const FALLBACK: &[u8] = include_bytes!("../../../assets/fonts/NotoSansSC-Medium.ttf");

pub(crate) struct Face {
    font: FontArc,
    pub(super) bytes: usize,
}

pub(super) struct Bitmap {
    pub(super) rgba: Vec<u8>,
    pub(super) width: u16,
    pub(super) height: u16,
    pub(super) left: f32,
    pub(super) top: f32,
    pub(super) scale: f32,
}

impl Face {
    pub(crate) fn from_bytes(bytes: Vec<u8>) -> Result<Self, String> {
        if bytes.len() > MAX_FONT_BYTES {
            return Err("font exceeds 32 MiB".to_owned());
        }
        let size = bytes.len();
        let font = FontArc::try_from_vec(bytes).map_err(|error| error.to_string())?;
        Self::new(font, size)
    }

    pub(crate) fn fallback() -> Self {
        let font = FontArc::new(FontRef::try_from_slice(FALLBACK).expect("bundled font"));
        Self::new(font, FALLBACK.len()).expect("bundled font metrics")
    }

    pub(crate) fn defaults() -> Vec<Self> {
        let mut faces = vec![Self::fallback()];
        for bytes in [
            include_bytes!("../../../assets/fonts/NotoSansArabic.ttf").as_slice(),
            include_bytes!("../../../assets/fonts/NotoSansDevanagari.ttf").as_slice(),
        ] {
            faces.push(
                Self::new(
                    FontArc::new(FontRef::try_from_slice(bytes).expect("bundled font")),
                    bytes.len(),
                )
                .expect("bundled metrics"),
            );
        }
        faces
    }

    pub(crate) fn family(bytes: Vec<Vec<u8>>) -> Result<Vec<Self>, String> {
        if bytes.iter().map(Vec::len).sum::<usize>() > 64 * 1024 * 1024 {
            return Err("font family exceeds 64 MiB".to_owned());
        }
        let mut faces = bytes
            .into_iter()
            .map(Self::from_bytes)
            .collect::<Result<Vec<_>, _>>()?;
        faces.extend(Self::defaults());
        Ok(faces)
    }

    fn new(font: FontArc, bytes: usize) -> Result<Self, String> {
        if font.units_per_em().is_none_or(|value| value <= 0.0) || font.height_unscaled() <= 0.0 {
            return Err("invalid font metrics".to_owned());
        }
        Ok(Self { font, bytes })
    }

    pub(super) fn glyph(&self, character: char) -> GlyphId {
        self.font.glyph_id(character)
    }

    pub(super) fn data(&self) -> &[u8] {
        self.font.font_data()
    }

    #[cfg(test)]
    pub(super) fn advance(&self, glyph: GlyphId, pixels: f32) -> f32 {
        self.font.h_advance_unscaled(glyph) * pixels / self.font.units_per_em().unwrap()
    }

    #[cfg(test)]
    pub(super) fn kern(&self, left: Option<GlyphId>, right: GlyphId, pixels: f32) -> f32 {
        left.map_or(0.0, |left| self.font.kern_unscaled(left, right)) * pixels
            / self.font.units_per_em().unwrap()
    }

    #[cfg(test)]
    pub(super) fn width(&self, text: &str, pixels: f32) -> f32 {
        let mut previous = None;
        let mut width = 0.0;
        for character in text.chars() {
            let glyph = self.glyph(character);
            width += self.kern(previous, glyph, pixels) + self.advance(glyph, pixels);
            previous = Some(glyph);
        }
        width
    }

    pub(super) fn rasterize(&self, glyph: GlyphId, pixels: u16) -> Option<Bitmap> {
        // ab_glyph scales by ascender-to-descender height; our theme uses pixels per em.
        let em_scale = self.font.height_unscaled() / self.font.units_per_em().unwrap();
        let mut outline = self
            .font
            .outline_glyph(glyph.with_scale(f32::from(pixels) * em_scale))?;
        let bounds = outline.px_bounds();
        let side = bounds.width().max(bounds.height());
        let scale = (side / f32::from(MAX_GLYPH_SIDE - 2)).max(1.0);
        if scale > 1.0 {
            outline = self
                .font
                .outline_glyph(glyph.with_scale(f32::from(pixels) * em_scale / scale))?;
        }
        let bounds = outline.px_bounds();
        let width = bounds.width() as u16;
        let height = bounds.height() as u16;
        if width == 0 || height == 0 || width > MAX_GLYPH_SIDE || height > MAX_GLYPH_SIDE {
            return None;
        }
        let mut rgba = vec![0; usize::from(width) * usize::from(height) * 4];
        outline.draw(|x, y, coverage| {
            let offset = (y as usize * usize::from(width) + x as usize) * 4;
            rgba[offset..offset + 4].copy_from_slice(&[
                255,
                255,
                255,
                (coverage.clamp(0.0, 1.0) * 255.0).round() as u8,
            ]);
        });
        Some(Bitmap {
            rgba,
            width,
            height,
            left: bounds.min.x * scale,
            top: bounds.min.y * scale,
            scale,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cjk_metrics_use_em_size_and_only_requested_glyphs_are_rasterized() {
        let face = Face::fallback();
        assert!((face.width("\u{6c49}\u{5b57}", 30.0) - 60.0).abs() < 0.01);
        assert!((face.width("A V", 60.0) - 2.0 * face.width("A V", 30.0)).abs() < 0.01);
        for character in ['A', '\u{6c49}', '\u{5b57}'] {
            assert_ne!(face.glyph(character).0, 0);
            let bitmap = face.rasterize(face.glyph(character), 60).unwrap();
            assert!(
                bitmap
                    .rgba
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .any(|pixel| pixel[3] > 0)
            );
            assert!(bitmap.top < 0.0);
        }
        assert!(face.rasterize(face.glyph(' '), 30).is_none());
        assert!(face.width(" ", 30.0) > 0.0);
        assert!(Face::from_bytes(vec![0; 32]).is_err());
    }

    #[test]
    fn oversized_glyphs_are_downsampled_with_bounded_scratch() {
        let face = Face::fallback();
        let bitmap = face.rasterize(face.glyph('\u{6c49}'), 4096).unwrap();
        assert!(bitmap.width <= MAX_GLYPH_SIDE && bitmap.height <= MAX_GLYPH_SIDE);
        assert!(bitmap.scale > 1.0);
        assert!(bitmap.rgba.len() <= usize::from(MAX_GLYPH_SIDE).pow(2) * 4);
    }
}
