use super::*;

#[test]
fn shapes_joining_scripts_and_selects_fonts_without_missing_glyphs() {
    let fonts = Face::defaults();
    for (text, face) in [("\u{633}\u{644}\u{627}\u{645}", 1), ("\u{928}\u{92e}\u{938}\u{94d}\u{924}\u{947}", 2)] {
        let glyphs = shape(&fonts, text);
        assert!(!glyphs.is_empty());
        assert!(glyphs.iter().all(|glyph| glyph.id.0 != 0 && glyph.face == face));
        assert!(glyphs.iter().map(|glyph| glyph.advance).sum::<f32>() > 0.0);
        assert!(glyphs.len() <= text.chars().count());
    }
    let glyphs = shape(&fonts, "Hello \u{4e2d}\u{6587} \u{633}\u{644}\u{627}\u{645}");
    assert!(glyphs.iter().all(|glyph| glyph.id.0 != 0));
    assert!(glyphs.iter().any(|glyph| glyph.face == 1));
}
