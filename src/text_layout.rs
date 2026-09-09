use renrs::text::{TextRun, TextStyle};
use unicode_segmentation::UnicodeSegmentation;
pub mod visual;

#[derive(Debug, Clone, PartialEq)]
pub struct TextFragment {
    pub text: String,
    pub style: TextStyle,
    pub width: f32,
    boundaries: Vec<usize>,
    source_indices: Vec<usize>,
}

impl TextFragment {
    pub fn character_count(&self) -> usize {
        self.boundaries.len().saturating_sub(1)
    }

    pub(crate) fn source_indices(&self) -> &[usize] {
        &self.source_indices
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
    source_index: usize,
}

#[derive(Debug, Clone)]
enum Token {
    Text(Vec<StyledChar>),
    Space(Vec<StyledChar>),
    Break,
}

#[cfg(test)]
pub fn layout_runs(
    runs: &[TextRun],
    fallback: &str,
    visible_characters: usize,
    maximum_width: f32,
    measure: impl FnMut(&str) -> f32,
) -> Vec<TextLine> {
    layout_runs_with_clusters(
        runs,
        fallback,
        visible_characters,
        maximum_width,
        measure,
        grapheme_boundaries,
    )
}

pub fn layout_runs_with_clusters(
    runs: &[TextRun],
    fallback: &str,
    visible_characters: usize,
    maximum_width: f32,
    mut measure: impl FnMut(&str) -> f32,
    mut cluster_boundaries: impl FnMut(&str) -> Vec<usize>,
) -> Vec<TextLine> {
    let owned_fallback;
    let runs = if runs.is_empty() {
        owned_fallback = [TextRun {
            text: fallback.to_owned(),
            style: TextStyle::default(),
            cue: None,
        }];
        &owned_fallback[..]
    } else {
        runs
    };
    let mut characters = Vec::new();
    let mut source_index = 0;
    'runs: for run in runs {
        for character in run.text.chars() {
            if characters.len() >= visible_characters {
                break 'runs;
            }
            characters.push(StyledChar {
                character,
                style: run.style.clone(),
                source_index,
            });
            source_index += 1;
        }
    }
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
                    let boundaries = cluster_boundaries(&token);
                    let boundaries = if valid_boundaries(&token, &boundaries) {
                        boundaries
                    } else {
                        grapheme_boundaries(&token)
                    };
                    let mut character_offset = 0;
                    for offsets in boundaries.windows(2) {
                        let text = &token[offsets[0]..offsets[1]];
                        let count = text.chars().count();
                        let cluster =
                            characters[character_offset..character_offset + count].to_vec();
                        character_offset += count;
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

fn grapheme_boundaries(text: &str) -> Vec<usize> {
    text.grapheme_indices(true)
        .map(|(offset, _)| offset)
        .chain(std::iter::once(text.len()))
        .collect()
}

fn valid_boundaries(text: &str, boundaries: &[usize]) -> bool {
    boundaries.first() == Some(&0)
        && boundaries.last() == Some(&text.len())
        && boundaries
            .windows(2)
            .all(|pair| pair[0] < pair[1] && text.is_char_boundary(pair[1]))
}

#[cfg(test)]
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
            previous.source_indices.extend(fragment.source_indices);
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
    let mut source_indices = Vec::new();
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
                std::mem::take(&mut source_indices),
                style.take().unwrap(),
                width,
                merge_first || !first_fragment,
            );
            first_fragment = false;
        }
        style = Some(character.style);
        text.push(character.character);
        source_indices.push(character.source_index);
    }
    if let Some(style) = style {
        let width = measure(&text);
        push_fragment(
            line,
            text,
            source_indices,
            style,
            width,
            merge_first || !first_fragment,
        );
    }
}

fn push_fragment(
    line: &mut TextLine,
    text: String,
    source_indices: Vec<usize>,
    style: TextStyle,
    width: f32,
    merge: bool,
) {
    if text.is_empty() {
        return;
    }
    if merge
        && let Some(previous) = line.fragments.last_mut()
        && previous.style == style
    {
        previous.text.push_str(&text);
        previous.width += width;
        previous.source_indices.extend(source_indices);
    } else {
        line.fragments.push(TextFragment {
            text,
            style,
            width,
            boundaries: Vec::new(),
            source_indices,
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
#[path = "text_layout/tests.rs"]
mod tests;
