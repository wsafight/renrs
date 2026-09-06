use renrs::text::{TextRun, TextStyle};
use unicode_segmentation::UnicodeSegmentation;
pub mod visual;

#[derive(Debug, Clone, PartialEq)]
pub struct TextFragment {
    pub text: String,
    pub style: TextStyle,
    pub width: f32,
    boundaries: Vec<usize>,
}

impl TextFragment {
    pub fn character_count(&self) -> usize {
        self.boundaries.len().saturating_sub(1)
    }

    #[cfg(test)]
    pub fn prefix(&self, characters: usize) -> &str {
        &self.text[..self.boundaries[characters.min(self.character_count())]]
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TextLine {
    pub fragments: Vec<TextFragment>,
    pub width: f32,
}

#[derive(Debug, Clone)]
struct StyledChar {
    character: char,
    style: TextStyle,
}

#[derive(Debug, Clone)]
enum Token {
    Text(Vec<StyledChar>),
    Space(Vec<StyledChar>),
    Break,
}

pub fn layout_runs(
    runs: &[TextRun],
    fallback: &str,
    visible_characters: usize,
    maximum_width: f32,
    mut measure: impl FnMut(&str) -> f32,
) -> Vec<TextLine> {
    let owned_fallback;
    let runs = if runs.is_empty() {
        owned_fallback = [TextRun {
            text: fallback.to_owned(),
            style: TextStyle::default(),
        }];
        &owned_fallback[..]
    } else {
        runs
    };
    let characters = runs
        .iter()
        .flat_map(|run| {
            run.text.chars().map(|character| StyledChar {
                character,
                style: run.style.clone(),
            })
        })
        .take(visible_characters)
        .collect::<Vec<_>>();
    let tokens = tokenize(characters);
    let mut lines = Vec::new();
    let mut current = TextLine::default();

    for token in tokens {
        match token {
            Token::Break => finish_line(&mut lines, &mut current, true),
            Token::Space(_) if current.fragments.is_empty() => {}
            Token::Space(characters) => {
                let text = characters
                    .iter()
                    .map(|item| item.character)
                    .collect::<String>();
                let width = measure(&text);
                if current.width + width > maximum_width {
                    finish_line(&mut lines, &mut current, false);
                    continue;
                }
                push_styled_chars(&mut current, characters, &mut measure, false);
            }
            Token::Text(characters) => {
                let width = measure(
                    &characters
                        .iter()
                        .map(|item| item.character)
                        .collect::<String>(),
                );
                if current.width > 0.0 && current.width + width > maximum_width {
                    finish_line(&mut lines, &mut current, false);
                    if characters.iter().all(|item| item.character.is_whitespace()) {
                        continue;
                    }
                }
                if width > maximum_width {
                    let token: String = characters.iter().map(|ch| ch.character).collect();
                    let mut offset = 0;
                    for text in token.graphemes(true) {
                        let count = text.chars().count();
                        let cluster = characters[offset..offset + count].to_vec();
                        offset += count;
                        let character_width = measure(text);
                        if current.width > 0.0 && current.width + character_width > maximum_width {
                            finish_line(&mut lines, &mut current, false);
                        }
                        push_styled_chars(&mut current, cluster, &mut measure, true);
                    }
                } else {
                    push_styled_chars(&mut current, characters, &mut measure, true);
                }
            }
        }
    }
    if !current.fragments.is_empty() || lines.is_empty() {
        finish_line(&mut lines, &mut current, true);
    }
    for line in &mut lines {
        line.width = 0.0;
        for fragment in &mut line.fragments {
            fragment.width = measure(&fragment.text);
            line.width += fragment.width;
        }
    }
    lines
}

pub fn wrap_plain(text: &str, maximum_width: f32, measure: impl FnMut(&str) -> f32) -> Vec<String> {
    layout_runs(&[], text, usize::MAX, maximum_width, measure)
        .into_iter()
        .map(|line| {
            line.fragments
                .into_iter()
                .map(|fragment| fragment.text)
                .collect()
        })
        .collect()
}

fn tokenize(characters: Vec<StyledChar>) -> Vec<Token> {
    let mut tokens = Vec::<Token>::new();
    let mut current = Vec::<StyledChar>::new();
    let mut current_kind = None;

    for character in characters {
        if character.style.ruby.is_some() {
            if current
                .last()
                .is_some_and(|previous| previous.style.ruby != character.style.ruby)
            {
                flush_token(&mut tokens, &mut current, current_kind.take());
            }
            current_kind = Some(4);
            current.push(character);
            continue;
        }
        if current_kind == Some(4) {
            flush_token(&mut tokens, &mut current, current_kind.take());
        }
        if character.character == '\n' {
            flush_token(&mut tokens, &mut current, current_kind.take());
            tokens.push(Token::Break);
            continue;
        }
        let kind = if character.character.is_whitespace() {
            0_u8
        } else if breakable_cjk(character.character)
            || opening_punctuation(character.character)
            || closing_punctuation(character.character)
        {
            1
        } else {
            2
        };
        if closing_punctuation(character.character) {
            flush_token(&mut tokens, &mut current, current_kind.take());
            if let Some(Token::Text(previous)) = tokens.last_mut() {
                previous.push(character);
            } else {
                tokens.push(Token::Text(vec![character]));
            }
            continue;
        }
        if kind == 1 {
            if current_kind == Some(3) {
                current.push(character);
                flush_token(&mut tokens, &mut current, current_kind.take());
                continue;
            }
            flush_token(&mut tokens, &mut current, current_kind.take());
            if opening_punctuation(character.character) {
                current_kind = Some(3);
                current.push(character);
            } else {
                tokens.push(Token::Text(vec![character]));
            }
            continue;
        }
        if current_kind != Some(kind) {
            flush_token(&mut tokens, &mut current, current_kind.take());
            current_kind = Some(kind);
        }
        current.push(character);
    }
    flush_token(&mut tokens, &mut current, current_kind);
    tokens
}

fn flush_token(tokens: &mut Vec<Token>, current: &mut Vec<StyledChar>, kind: Option<u8>) {
    if current.is_empty() {
        return;
    }
    let characters = std::mem::take(current);
    if kind == Some(0) {
        tokens.push(Token::Space(characters));
    } else {
        tokens.push(Token::Text(characters));
    }
}

fn finish_line(lines: &mut Vec<TextLine>, current: &mut TextLine, keep_empty: bool) {
    while current
        .fragments
        .last()
        .is_some_and(|fragment| fragment.text.chars().all(char::is_whitespace))
    {
        if let Some(fragment) = current.fragments.pop() {
            current.width -= fragment.width;
        }
    }
    let mut merged = Vec::<TextFragment>::with_capacity(current.fragments.len());
    for fragment in current.fragments.drain(..) {
        if let Some(previous) = merged.last_mut()
            && previous.style == fragment.style
        {
            previous.text.push_str(&fragment.text);
            previous.width += fragment.width;
        } else {
            merged.push(fragment);
        }
    }
    current.fragments = merged;
    for fragment in &mut current.fragments {
        fragment.boundaries = fragment
            .text
            .char_indices()
            .map(|(offset, _)| offset)
            .chain(std::iter::once(fragment.text.len()))
            .collect();
    }
    if keep_empty || !current.fragments.is_empty() {
        lines.push(std::mem::take(current));
    }
}

fn push_styled_chars(
    line: &mut TextLine,
    characters: Vec<StyledChar>,
    measure: &mut impl FnMut(&str) -> f32,
    merge_first: bool,
) {
    let mut text = String::new();
    let mut style = None::<TextStyle>;
    let mut first_fragment = true;
    for character in characters {
        if style
            .as_ref()
            .is_some_and(|value| value != &character.style)
        {
            let width = measure(&text);
            push_fragment(
                line,
                std::mem::take(&mut text),
                style.take().unwrap(),
                width,
                merge_first || !first_fragment,
            );
            first_fragment = false;
        }
        style = Some(character.style);
        text.push(character.character);
    }
    if let Some(style) = style {
        let width = measure(&text);
        push_fragment(line, text, style, width, merge_first || !first_fragment);
    }
}

fn push_fragment(line: &mut TextLine, text: String, style: TextStyle, width: f32, merge: bool) {
    if text.is_empty() {
        return;
    }
    if merge
        && let Some(previous) = line.fragments.last_mut()
        && previous.style == style
    {
        previous.text.push_str(&text);
        previous.width += width;
    } else {
        line.fragments.push(TextFragment {
            text,
            style,
            width,
            boundaries: Vec::new(),
        });
    }
    line.width += width;
}

fn breakable_cjk(character: char) -> bool {
    matches!(
        character as u32,
        0x2e80..=0x2fff
            | 0x3000..=0x303f
            | 0x3040..=0x30ff
            | 0x31f0..=0x31ff
            | 0x3400..=0x4dbf
            | 0x4e00..=0x9fff
            | 0xac00..=0xd7af
            | 0xf900..=0xfaff
    )
}

fn opening_punctuation(character: char) -> bool {
    matches!(
        character,
        '（' | '《' | '〈' | '「' | '『' | '【' | '〔' | '“' | '‘'
    )
}

fn closing_punctuation(character: char) -> bool {
    matches!(
        character,
        '，' | '。'
            | '！'
            | '？'
            | '、'
            | '；'
            | '：'
            | '）'
            | '》'
            | '〉'
            | '」'
            | '』'
            | '】'
            | '〕'
            | '”'
            | '’'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn character_width(text: &str) -> f32 {
        text.chars().count() as f32
    }

    #[test]
    fn wraps_english_at_word_boundaries() {
        assert_eq!(
            wrap_plain("hello world", 6.0, character_width),
            ["hello", "world"]
        );
    }

    #[test]
    fn keeps_cjk_closing_punctuation_off_a_new_line() {
        assert_eq!(
            wrap_plain("你好，世界。", 3.0, character_width),
            ["你好，", "世界。"]
        );
    }

    #[test]
    fn keeps_cjk_opening_punctuation_with_the_next_character() {
        assert_eq!(
            wrap_plain("甲乙（丙丁）", 3.0, character_width),
            ["甲乙", "（丙", "丁）"]
        );
    }

    #[test]
    fn keeps_style_runs_after_layout() {
        let runs = [
            TextRun {
                text: "plain ".to_owned(),
                style: TextStyle::default(),
            },
            TextRun {
                text: "bold".to_owned(),
                style: TextStyle {
                    bold: true,
                    color: None,
                    ..TextStyle::default()
                },
            },
        ];
        let lines = layout_runs(&runs, "", usize::MAX, 20.0, character_width);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].fragments.len(), 2);
        assert!(lines[0].fragments[1].style.bold);
    }

    #[test]
    fn reveal_limit_is_applied_before_wrapping() {
        assert_eq!(wrap_plain("unused", 20.0, character_width), ["unused"]);
        let lines = layout_runs(&[], "abcdef", 3, 20.0, character_width);
        assert_eq!(lines[0].fragments[0].text, "abc");
    }

    #[test]
    fn cached_reveal_boundaries_match_unicode_characters() {
        let text = "A你好\u{1f600}e\u{301}";
        let lines = layout_runs(&[], text, usize::MAX, 100.0, character_width);
        let fragment = &lines[0].fragments[0];
        for count in 0..=text.chars().count() + 1 {
            assert_eq!(
                fragment.prefix(count),
                text.chars().take(count).collect::<String>()
            );
        }
    }
}
