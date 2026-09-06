use super::text::{draw_text, measure_width};
use super::ui_common::color;
use crate::text_layout::{TextLine, layout_runs};
use macroquad::prelude::*;
use renrs::runtime::DialogueState;
use renrs::theme::Theme;

#[derive(Default)]
pub(super) struct DialogueView {
    dialogue: Option<DialogueState>,
    theme: Option<Theme>,
    lines: Vec<TextLine>,
    page: usize,
    rows: usize,
    nvl_key: Option<(usize, usize, DialogueState)>,
    pub(super) nvl: Option<std::sync::Arc<DialogueState>>,
}

impl DialogueView {
    pub(super) const fn page(&self) -> usize {
        self.page
    }

    pub(super) fn restore_page(&mut self, page: usize) {
        self.page = page.min(
            self.lines
                .len()
                .div_ceil(self.rows.max(1))
                .saturating_sub(1),
        );
    }
    pub(super) fn invalidate(&mut self) {
        self.dialogue = None;
        self.page = 0;
        self.nvl_key = None;
    }

    pub(super) fn prepare_reading(&mut self, runtime: &renrs::Runtime) {
        let stage = runtime.stage();
        if !stage.nvl || stage.dialogue.is_none() {
            self.nvl = None;
            self.nvl_key = None;
            return;
        }
        let dialogue = stage.dialogue.as_ref().unwrap();
        if self
            .nvl_key
            .as_ref()
            .is_some_and(|(start, length, previous)| {
                *start == stage.nvl_start
                    && *length == runtime.history().len()
                    && previous == dialogue
            })
        {
            return;
        }
        self.nvl = runtime.nvl_dialogue().map(std::sync::Arc::new);
        self.nvl_key = Some((stage.nvl_start, runtime.history().len(), dialogue.clone()));
    }

    pub(super) fn prepare(&mut self, dialogue: &DialogueState, theme: &Theme) {
        if self.dialogue.as_ref() == Some(dialogue) && self.theme.as_ref() == Some(theme) {
            return;
        }
        self.lines = layout_runs(
            &dialogue.runs,
            &dialogue.text,
            usize::MAX,
            (theme.layout.dialogue_rect.width - 68.0).max(40.0),
            |text| measure_width(text, theme.dialogue_font_size),
        );
        let line_height = reading_line_height(dialogue, theme);
        self.rows = ((theme.layout.dialogue_rect.height - 64.0) / line_height)
            .floor()
            .max(1.0) as usize;
        self.page = 0;
        self.dialogue = Some(dialogue.clone());
        self.theme = Some(theme.clone());
    }

    fn page_lines(&self) -> impl Iterator<Item = &TextLine> {
        self.lines
            .iter()
            .skip(self.page * self.rows)
            .take(self.rows)
    }

    pub(super) fn character_count(&self) -> usize {
        self.page_lines()
            .flat_map(|line| &line.fragments)
            .map(crate::text_layout::TextFragment::character_count)
            .sum()
    }

    pub(super) fn next_page(&mut self) -> bool {
        if (self.page + 1) * self.rows < self.lines.len() {
            self.page += 1;
            true
        } else {
            false
        }
    }

    pub(super) fn draw(&self, visible: usize, theme: &Theme) {
        let panel = theme.layout.dialogue_rect;
        let mut remaining = visible;
        let line_height = self
            .dialogue
            .as_ref()
            .map_or(theme.dialogue_line_height, |dialogue| {
                reading_line_height(dialogue, theme)
            });
        for (index, line) in self.page_lines().enumerate() {
            let (rtl, pieces) = crate::text_layout::visual::pieces(line, remaining);
            remaining = remaining.saturating_sub(
                line.fragments
                    .iter()
                    .map(crate::text_layout::TextFragment::character_count)
                    .sum(),
            );
            let mut x = panel.x
                + 34.0
                + if rtl {
                    (panel.width - 68.0 - line.width).max(0.0)
                } else {
                    0.0
                };
            for piece in pieces {
                let fragment = &line.fragments[piece.fragment];
                let text = piece.visible;
                let width = measure_width(piece.full, theme.dialogue_font_size);
                let start = x + if piece.rtl {
                    width - measure_width(text, theme.dialogue_font_size)
                } else {
                    0.0
                };
                let tint = fragment.style.color.as_deref().map_or_else(
                    || {
                        color(if fragment.style.bold {
                            &theme.focus_color
                        } else {
                            &theme.text_color
                        })
                    },
                    color,
                );
                draw_text(
                    text,
                    start,
                    panel.y
                        + 60.0
                        + f32::from(theme.dialogue_font_size)
                        + index as f32 * line_height,
                    f32::from(theme.dialogue_font_size),
                    tint,
                );
                let baseline = panel.y
                    + 60.0
                    + f32::from(theme.dialogue_font_size)
                    + index as f32 * line_height;
                if !text.is_empty() && fragment.style.underline {
                    draw_line(
                        x,
                        baseline + 3.0,
                        x + measure_width(text, theme.dialogue_font_size),
                        baseline + 3.0,
                        1.5,
                        tint,
                    );
                }
                if !text.is_empty()
                    && let Some(ruby) = &fragment.style.ruby
                {
                    let size = (theme.dialogue_font_size / 2).max(10);
                    let measured = measure_width(ruby, size);
                    let scale = (fragment.width / measured.max(1.0)).min(1.0);
                    let width = measured * scale;
                    draw_text(
                        ruby,
                        x + (fragment.width - width) / 2.0,
                        baseline - f32::from(theme.dialogue_font_size),
                        f32::from(size) * scale,
                        tint,
                    );
                }
                x += width;
            }
        }
        if self.lines.len() > self.rows {
            let pages = self.lines.len().div_ceil(self.rows);
            draw_text(
                format!("{} / {pages}", self.page + 1),
                panel.x + panel.width - 112.0,
                panel.y + 34.0,
                18.0,
                color(&theme.muted_text_color),
            );
        }
    }
}

fn reading_line_height(dialogue: &DialogueState, theme: &Theme) -> f32 {
    if dialogue.runs.iter().any(|run| run.style.ruby.is_some()) {
        theme
            .dialogue_line_height
            .max(f32::from(theme.dialogue_font_size) * 1.7)
    } else {
        theme.dialogue_line_height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_preserve_all_characters_and_stop_at_the_last_page() {
        let lines = layout_runs(
            &[],
            "one\ntwo\nthree\nfour\nfive",
            usize::MAX,
            10.0,
            |text| text.len() as f32,
        );
        let mut view = DialogueView {
            lines,
            rows: 2,
            ..DialogueView::default()
        };
        assert_eq!(view.character_count(), 6);
        assert!(view.next_page());
        assert_eq!(view.character_count(), 9);
        assert!(view.next_page());
        assert_eq!(view.character_count(), 4);
        assert!(!view.next_page());
    }
}
