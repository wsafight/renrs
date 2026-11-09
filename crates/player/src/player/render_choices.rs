use macroquad::prelude::*;
use renrs::WaitState;

use crate::frontend::{UiAction, UiActions};

use super::app::App;
use super::ui_common::{choice_button, color, draw_centered, wrap_lines};
use super::{CANVAS_HEIGHT, CANVAS_WIDTH};

impl App {
    pub(super) fn draw_choices(&mut self, mouse: Vec2, actions: &UiActions) {
        let Some(options) = self.runtime.as_ref().and_then(|runtime| {
            if let Some(WaitState::Choice { options }) = runtime.waiting() {
                Some(options)
            } else {
                None
            }
        }) else {
            return;
        };
        if self.focus.selected().is_none() && actions.pressed(UiAction::Up) {
            self.selected_choice = self.selected_choice.saturating_sub(1);
        }
        if self.focus.selected().is_none() && actions.pressed(UiAction::Down) {
            self.selected_choice = (self.selected_choice + 1).min(options.len().saturating_sub(1));
        }
        if self.focus.selected().is_none() {
            if actions.pressed(UiAction::Home) {
                self.selected_choice = 0;
            }
            if actions.pressed(UiAction::End) {
                self.selected_choice = options.len().saturating_sub(1);
            }
            self.selected_choice = self
                .selected_choice
                .saturating_add_signed(-mouse_wheel().1.round() as isize)
                .min(options.len().saturating_sub(1));
        }
        let rows = 5.min(options.len());
        let start = self.selected_choice.saturating_sub(rows.saturating_sub(1));
        let heights: Vec<_> = options
            .iter()
            .skip(start)
            .take(rows)
            .map(|option| {
                // One-line options dominate; measuring is a shaped-cache hit
                // while wrap_lines rebuilds the layout every frame.
                let trimmed = option.trim();
                let lines = if !trimmed.is_empty()
                    && !option.contains('\n')
                    && super::text::measure_width(trimmed, self.theme.ui_font_size) <= 676.0
                {
                    1
                } else {
                    wrap_lines(option, 676.0, self.theme.ui_font_size).len()
                };
                (lines as f32 * f32::from(self.theme.ui_font_size + 4) + 16.0).clamp(52.0, 92.0)
            })
            .collect();
        let total_height = heights.iter().sum::<f32>() + rows.saturating_sub(1) as f32 * 14.0;
        let mut y = (CANVAS_HEIGHT - total_height) / 2.0 + 20.0;
        let mut selected = None;
        for ((index, option), height) in options
            .iter()
            .enumerate()
            .skip(start)
            .take(rows)
            .zip(heights)
        {
            let rect = Rect::new(290.0, y, 700.0, height);
            y += height + 14.0;
            if choice_button(
                rect,
                option,
                mouse,
                index == self.selected_choice,
                &self.theme,
            ) {
                selected = Some(index);
            }
        }
        if options.len() > rows {
            draw_centered(
                &format!("{}-{} / {}", start + 1, start + rows, options.len()),
                CANVAS_WIDTH / 2.0,
                y + 16.0,
                18,
                color(&self.theme.text_color),
            );
        }
        if self.focus.selected().is_none() && actions.pressed(UiAction::Activate) {
            selected = Some(self.selected_choice);
        }
        if let Some(index) = selected {
            let result = self
                .runtime
                .as_mut()
                .expect("playing requires runtime")
                .choose(index);
            self.handle_wait(result);
        }
    }
}
