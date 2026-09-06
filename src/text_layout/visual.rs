use super::TextLine;
use unicode_bidi::BidiInfo;

pub struct Piece<'a> {
    pub fragment: usize,
    pub full: &'a str,
    pub visible: &'a str,
    pub rtl: bool,
}

pub fn pieces(line: &TextLine, visible: usize) -> (bool, Vec<Piece<'_>>) {
    let text: String = line
        .fragments
        .iter()
        .map(|part| part.text.as_str())
        .collect();
    let bidi = BidiInfo::new(&text, None);
    let Some(paragraph) = bidi.paragraphs.first() else {
        return (false, vec![]);
    };
    let (levels, runs) = bidi.visual_runs(paragraph, paragraph.range.clone());
    let mut output = Vec::new();
    for run in runs {
        let rtl = levels[run.start].is_rtl();
        let mut offset = 0;
        let mut characters = 0;
        let mut parts = Vec::new();
        for (index, fragment) in line.fragments.iter().enumerate() {
            let end = offset + fragment.text.len();
            let start = run.start.max(offset);
            let stop = run.end.min(end);
            if start < stop {
                let full = &fragment.text[start - offset..stop - offset];
                let before = characters + fragment.text[..start - offset].chars().count();
                let count = visible.saturating_sub(before);
                let bytes = full
                    .char_indices()
                    .nth(count)
                    .map_or(full.len(), |(i, _)| i);
                parts.push(Piece {
                    fragment: index,
                    full,
                    visible: &full[..bytes],
                    rtl,
                });
            }
            characters += fragment.character_count();
            offset = end;
        }
        if rtl {
            parts.reverse();
        }
        output.extend(parts);
    }
    (paragraph.level.is_rtl(), output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keeps_numbers_in_order_inside_rtl_text_and_preserves_reveal_order() {
        let lines = crate::text_layout::layout_runs(
            &[],
            "\u{627}\u{628} 123 \u{62c}",
            usize::MAX,
            100.0,
            |s| s.chars().count() as f32,
        );
        let (rtl, parts) = pieces(&lines[0], 2);
        assert!(rtl);
        assert!(
            parts
                .iter()
                .any(|part| part.full == "123" && !part.rtl && part.visible.is_empty())
        );
        assert_eq!(
            parts
                .iter()
                .map(|part| part.visible)
                .collect::<String>()
                .trim(),
            "\u{627}\u{628}"
        );
    }
}
