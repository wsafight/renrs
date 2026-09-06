use super::text::draw_text;
use macroquad::prelude::*;
use renrs::Runtime;

use super::app::App;
use super::ui_common::{color, wrap_lines};
use crate::frontend::{FocusScope, UiAction, UiActions};

#[derive(Default)]
pub(super) struct HistoryView {
    lines: Vec<(String, Color, Option<String>)>,
    offset: usize,
    ready: bool,
    pub(super) custom:
        std::collections::HashMap<(u32, u16), std::sync::Arc<[super::ui_screen_lists::ListItem]>>,
}

impl HistoryView {
    pub(super) fn invalidate(&mut self) {
        self.ready = false;
        self.custom.clear();
    }
}

impl App {
    #[allow(clippy::too_many_lines)]
    pub(super) fn draw_history(&mut self, mouse: Vec2, actions: &UiActions) {
        if self.draw_custom_screen(renrs::screens::ScreenKind::History, mouse, actions) {
            return;
        }
        if self.overlay_backdrop("History", mouse, actions, FocusScope::History, 1, Some(0)) {
            return;
        }
        let line_height = f32::from(self.theme.ui_font_size) + 12.0;
        let rows = (560.0 / line_height).floor() as usize;
        if !self.history_view.ready {
            let history = self.runtime.as_ref().map_or(&[][..], Runtime::history);
            let mut lines = Vec::new();
            for dialogue in history {
                if let Some(name) = &dialogue.speaker_name {
                    for line in wrap_lines(name, 1060.0, self.theme.ui_font_size) {
                        lines.push((line, color(&dialogue.speaker_color), None));
                    }
                }
                for (index, line) in wrap_lines(&dialogue.text, 1000.0, self.theme.ui_font_size)
                    .into_iter()
                    .enumerate()
                {
                    lines.push((
                        line,
                        color(&self.theme.text_color),
                        (index == 0).then(|| dialogue.voice_path.clone()).flatten(),
                    ));
                }
                lines.push((String::new(), WHITE, None));
            }
            self.history_view.offset = lines.len().saturating_sub(rows);
            self.history_view.lines = lines;
            self.history_view.ready = true;
        }
        let view = &mut self.history_view;
        let maximum = view.lines.len().saturating_sub(rows);
        let wheel = if Rect::new(64.0, 90.0, 1152.0, 590.0).contains(mouse) {
            mouse_wheel().1.round() as isize * 3
        } else {
            0
        };
        let delta = isize::from(actions.pressed(UiAction::Down))
            - isize::from(actions.pressed(UiAction::Up))
            + isize::try_from(rows).unwrap_or(isize::MAX)
                * (isize::from(actions.pressed(UiAction::PageDown))
                    - isize::from(actions.pressed(UiAction::PageUp)))
            - wheel;
        view.offset = view.offset.saturating_add_signed(delta).min(maximum);
        if actions.pressed(UiAction::Home) {
            view.offset = 0;
        }
        if actions.pressed(UiAction::End) {
            view.offset = maximum;
        }
        if maximum > 0 {
            let track = Rect::new(1188.0, 100.0, 16.0, 560.0);
            let thumb_height = (track.h * rows as f32 / view.lines.len() as f32).max(24.0);
            if track.contains(mouse) && is_mouse_button_down(MouseButton::Left) {
                let ratio = ((mouse.y - track.y - thumb_height / 2.0) / (track.h - thumb_height))
                    .clamp(0.0, 1.0);
                view.offset = (ratio * maximum as f32).round() as usize;
            }
            draw_rectangle(
                track.x,
                track.y,
                track.w,
                track.h,
                color(&self.theme.surface_color),
            );
            draw_rectangle(
                track.x,
                track.y + (track.h - thumb_height) * view.offset as f32 / maximum as f32,
                track.w,
                thumb_height,
                color(&self.theme.focus_color),
            );
        }
        if view.lines.is_empty() {
            draw_text(
                "No dialogue yet",
                96.0,
                132.0,
                f32::from(self.theme.ui_font_size),
                color(&self.theme.muted_text_color),
            );
        }
        for (index, (text, tint, voice)) in
            view.lines.iter().skip(view.offset).take(rows).enumerate()
        {
            draw_text(
                text,
                96.0,
                108.0 + f32::from(self.theme.ui_font_size) + index as f32 * line_height,
                f32::from(self.theme.ui_font_size),
                *tint,
            );
            if let Some(path) = voice {
                let rect = Rect::new(
                    1125.0,
                    108.0 + index as f32 * line_height,
                    42.0,
                    line_height,
                );
                draw_triangle(
                    vec2(rect.x + 8.0, rect.y + 5.0),
                    vec2(rect.x + 8.0, rect.y + 25.0),
                    vec2(rect.x + 28.0, rect.y + 15.0),
                    color(&self.theme.focus_color),
                );
                if rect.contains(mouse) {
                    if is_mouse_button_pressed(MouseButton::Left) {
                        self.audio.replay_voice(path);
                    } else {
                        super::ui_common::draw_fitted_centered(
                            Rect::new(930.0, 70.0, 250.0, 30.0),
                            &super::ui_text::tr("Replay voice"),
                            18,
                            color(&self.theme.text_color),
                        );
                    }
                }
            }
        }
    }
}
