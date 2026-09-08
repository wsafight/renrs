use super::Face;
use rustybuzz::{Direction, UnicodeBuffer};
use unicode_bidi::BidiInfo;
use unicode_script::{Script, UnicodeScript};
use unicode_segmentation::UnicodeSegmentation;
#[cfg(test)]
mod tests {
    include!("shaping/tests.rs");
}

#[derive(Clone, Copy)]
pub(super) struct Glyph {
    pub(super) face: usize,
    pub(super) id: ab_glyph::GlyphId,
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) advance: f32,
    pub(super) cluster: usize,
}

pub(super) fn shape(faces: &[Face], text: &str) -> Vec<Glyph> {
    let bidi = BidiInfo::new(text, None);
    let mut output = Vec::new();
    for paragraph in &bidi.paragraphs {
        let (levels, runs) = bidi.visual_runs(paragraph, paragraph.range.clone());
        for run in runs {
            let rtl = levels[run.start].is_rtl();
            let run_start = run.start;
            let mut spans: Vec<(usize, Script, usize, String)> = Vec::new();
            for (offset, grapheme) in text[run].grapheme_indices(true) {
                let face = faces
                    .iter()
                    .position(|face| {
                        grapheme.chars().all(|ch| {
                            ch.is_control()
                                || matches!(ch, '\u{200c}' | '\u{200d}' | '\u{fe0e}' | '\u{fe0f}')
                                || face.glyph(ch).0 != 0
                        })
                    })
                    .unwrap_or(0);
                let script = grapheme
                    .chars()
                    .map(|ch| ch.script())
                    .find(|script| !matches!(script, Script::Common | Script::Inherited))
                    .unwrap_or(Script::Common);
                if let Some((previous_face, previous_script, _, text)) = spans.last_mut()
                    && *previous_face == face
                    && (script == Script::Common
                        || *previous_script == Script::Common
                        || *previous_script == script)
                {
                    text.push_str(grapheme);
                    if script != Script::Common {
                        *previous_script = script;
                    }
                } else {
                    spans.push((face, script, run_start + offset, grapheme.to_owned()));
                }
            }
            if rtl {
                spans.reverse();
            }
            for (index, _, span_start, span) in spans {
                let face =
                    rustybuzz::Face::from_slice(faces[index].data(), 0).expect("validated font");
                let mut buffer = UnicodeBuffer::new();
                buffer.push_str(&span);
                buffer.set_direction(if rtl {
                    Direction::RightToLeft
                } else {
                    Direction::LeftToRight
                });
                buffer.guess_segment_properties();
                let shaped = rustybuzz::shape(&face, &[], buffer);
                let scale = 1.0 / face.units_per_em() as f32;
                for (info, position) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
                    output.push(Glyph {
                        face: index,
                        id: ab_glyph::GlyphId(info.glyph_id as u16),
                        x: position.x_offset as f32 * scale,
                        y: -position.y_offset as f32 * scale,
                        advance: position.x_advance as f32 * scale,
                        cluster: span_start + info.cluster as usize,
                    });
                }
            }
        }
    }
    output
}
