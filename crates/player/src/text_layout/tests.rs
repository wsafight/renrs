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
            cue: None,
        },
        TextRun {
            text: "bold".to_owned(),
            style: TextStyle {
                bold: true,
                color: None,
                ..TextStyle::default()
            },
            cue: None,
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

#[test]
fn custom_shaping_clusters_are_never_split_across_lines() {
    let lines = layout_runs_with_clusters(&[], "office", usize::MAX, 2.0, character_width, |_| {
        vec![0, 1, 4, 6]
    });
    assert_eq!(
        lines
            .into_iter()
            .map(|line| line.fragments[0].text.clone())
            .collect::<Vec<_>>(),
        ["o", "ffi", "ce"]
    );
}
