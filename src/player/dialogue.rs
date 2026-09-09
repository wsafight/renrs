use super::text::{draw_text, measure_width};
use super::ui_common::color;
#[cfg(test)]
use crate::text_layout::layout_runs;
use crate::text_layout::{TextFragment, TextLine, layout_runs_with_clusters};
use macroquad::prelude::*;
use renrs::runtime::DialogueState;
use renrs::text::{TextCue, TextRun};
use renrs::theme::Theme;
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DialogueCue {
    pub(super) position: usize,
    pub(super) kind: TextCue,
}

#[derive(Default)]
pub(super) struct DialogueView {
    dialogue: Option<DialogueState>,
    theme: Option<Theme>,
    lines: Vec<TextLine>,
    pages: Vec<Range<usize>>,
    cues: Vec<Vec<DialogueCue>>,
    page: usize,
    rows: usize,
    next_cue: usize,
    nvl_key: Option<(usize, usize, DialogueState)>,
    pub(super) nvl: Option<std::sync::Arc<DialogueState>>,
}

impl DialogueView {
    pub(super) const fn page(&self) -> usize {
        self.page
    }

    pub(super) fn restore_page(&mut self, page: usize) {
        self.page = page.min(self.page_count().saturating_sub(1));
        self.next_cue = 0;
    }
    pub(super) fn invalidate(&mut self) {
        self.dialogue = None;
        self.page = 0;
        self.next_cue = 0;
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
        let line_height = reading_line_height(dialogue, theme);
        self.rows = ((theme.layout.dialogue_rect.height - 64.0) / line_height)
            .floor()
            .max(1.0) as usize;
        self.lines.clear();
        self.pages.clear();
        self.cues.clear();
        let fallback_runs;
        let runs = if dialogue.runs.is_empty() {
            fallback_runs = [TextRun {
                text: dialogue.text.clone(),
                style: renrs::text::TextStyle::default(),
                cue: None,
            }];
            &fallback_runs[..]
        } else {
            &dialogue.runs
        };
        let mut section = Vec::new();
        for run in runs {
            if let Some(cue @ TextCue::Page { .. }) = run.cue {
                self.push_section(&section, Some(cue), theme);
                section.clear();
            } else {
                section.push(run.clone());
            }
        }
        if !section.is_empty() || self.pages.is_empty() {
            self.push_section(&section, None, theme);
        }
        self.page = 0;
        self.next_cue = 0;
        self.dialogue = Some(dialogue.clone());
        self.theme = Some(theme.clone());
    }

    fn page_lines(&self) -> impl Iterator<Item = &TextLine> {
        let range = self.page_range();
        self.lines[range].iter()
    }

    pub(super) fn character_count(&self) -> usize {
        self.page_lines()
            .flat_map(|line| &line.fragments)
            .map(crate::text_layout::TextFragment::character_count)
            .sum()
    }

    pub(super) fn cue(&self) -> Option<DialogueCue> {
        self.cues
            .get(self.page)
            .and_then(|cues| cues.get(self.next_cue))
            .copied()
    }

    pub(super) fn consume_cue(&mut self) -> Option<DialogueCue> {
        let cue = self.cue()?;
        self.next_cue += 1;
        Some(cue)
    }

    pub(super) fn no_wait(&self) -> bool {
        self.dialogue
            .as_ref()
            .is_some_and(|dialogue| dialogue.no_wait)
    }

    pub(super) fn next_page(&mut self) -> bool {
        if self.page + 1 < self.page_count() {
            self.page += 1;
            self.next_cue = 0;
            true
        } else {
            false
        }
    }

    fn page_count(&self) -> usize {
        if self.pages.is_empty() {
            self.lines.len().div_ceil(self.rows.max(1)).max(1)
        } else {
            self.pages.len()
        }
    }

    fn page_range(&self) -> Range<usize> {
        self.pages.get(self.page).cloned().unwrap_or_else(|| {
            let start = (self.page * self.rows).min(self.lines.len());
            start..(start + self.rows).min(self.lines.len())
        })
    }

    fn push_section(&mut self, runs: &[TextRun], page_cue: Option<TextCue>, theme: &Theme) {
        let fallback = runs.iter().map(|run| run.text.as_str()).collect::<String>();
        let section_lines = layout_runs_with_clusters(
            runs,
            &fallback,
            usize::MAX,
            (theme.layout.dialogue_rect.width - 68.0).max(40.0),
            |text| measure_width(text, theme.dialogue_font_size),
            super::text::cluster_boundaries,
        );
        let mut source_offset = 0;
        let mut source_cues = Vec::new();
        for run in runs {
            if let Some(cue) = run.cue
                && !matches!(cue, TextCue::NoWait | TextCue::Page { .. })
            {
                source_cues.push((source_offset, cue));
            }
            source_offset += run.text.chars().count();
        }
        let retained = section_lines
            .iter()
            .flat_map(|line| &line.fragments)
            .flat_map(TextFragment::source_indices)
            .copied()
            .collect::<Vec<_>>();
        let visual_cues = source_cues
            .into_iter()
            .map(|(position, cue)| (retained.partition_point(|source| *source < position), cue))
            .collect::<Vec<_>>();
        let section_start = self.lines.len();
        let mut visual_start = 0;
        self.lines.extend(section_lines);
        let section_end = self.lines.len();
        for start in (section_start..section_end).step_by(self.rows.max(1)) {
            let end = (start + self.rows).min(section_end);
            let page_length = self.lines[start..end]
                .iter()
                .flat_map(|line| &line.fragments)
                .map(TextFragment::character_count)
                .sum::<usize>();
            let is_last = end == section_end;
            let mut cues = visual_cues
                .iter()
                .filter(|(position, _)| {
                    *position >= visual_start && (*position < visual_start + page_length || is_last)
                })
                .map(|(position, kind)| DialogueCue {
                    position: position.saturating_sub(visual_start).min(page_length),
                    kind: *kind,
                })
                .collect::<Vec<_>>();
            if is_last && let Some(kind) = page_cue {
                cues.push(DialogueCue {
                    position: page_length,
                    kind,
                });
            }
            cues.sort_by_key(|cue| cue.position);
            self.pages.push(start..end);
            self.cues.push(cues);
            visual_start += page_length;
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
        if self.page_count() > 1 {
            let pages = self.page_count();
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

    #[test]
    fn page_cues_are_consumed_independently() {
        let lines = layout_runs(&[], "one\ntwo", usize::MAX, 10.0, |text| text.len() as f32);
        let first = DialogueCue {
            position: 3,
            kind: TextCue::Wait { hundredths: None },
        };
        let second = DialogueCue {
            position: 3,
            kind: TextCue::Fast,
        };
        let mut view = DialogueView {
            lines,
            pages: vec![0..1, 1..2],
            cues: vec![vec![first], vec![second]],
            rows: 1,
            ..DialogueView::default()
        };
        assert_eq!(view.consume_cue(), Some(first));
        assert_eq!(view.cue(), None);
        assert!(view.next_page());
        assert_eq!(view.cue(), Some(second));
    }
}
