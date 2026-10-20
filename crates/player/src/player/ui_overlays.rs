use super::app::{App, Overlay};
use super::text::draw_text;
use super::ui_common::{color, color_alpha, compact_button, keyboard_adjustment, slider};
use super::{CANVAS_HEIGHT, CANVAS_WIDTH};
use crate::frontend::{FocusAxis, FocusScope, UiAction, UiActions};
use macroquad::prelude::*;

impl App {
    pub(super) fn draw_settings(&mut self, mouse: Vec2, actions: &UiActions) {
        if self.draw_custom_screen(renrs::screens::ScreenKind::Settings, mouse, actions) {
            return;
        }
        if self.overlay_backdrop(
            "Settings",
            mouse,
            actions,
            FocusScope::Settings,
            10,
            Some(1),
        ) {
            self.persist_settings();
            return;
        }
        let language = self.localizer.language().unwrap_or("Source text");
        if compact_button(
            Rect::new(340.0, 110.0, 600.0, 48.0),
            &format!("Language: {language}"),
            mouse,
            self.focus.is(1),
            self.focus.is(1) && actions.pressed(UiAction::Activate),
            &self.theme,
        ) {
            self.overlay = Some(Overlay::Languages);
            return;
        }
        for (index, label, value, minimum, maximum, step) in [
            (
                2,
                "Text speed",
                &mut self.settings.text_speed,
                10.0,
                100.0,
                5.0,
            ),
            (
                3,
                "Auto-play delay",
                &mut self.settings.auto_delay,
                0.5,
                10.0,
                0.5,
            ),
            (
                4,
                "Music volume",
                &mut self.settings.music_volume,
                0.0,
                1.0,
                0.05,
            ),
            (
                5,
                "Sound volume",
                &mut self.settings.sound_volume,
                0.0,
                1.0,
                0.05,
            ),
            (
                6,
                "Voice volume",
                &mut self.settings.voice_volume,
                0.0,
                1.0,
                0.05,
            ),
        ] {
            let previous = *value;
            *value = slider(
                Rect::new(340.0, 225.0 + (index - 2) as f32 * 72.0, 600.0, 48.0),
                label,
                *value,
                minimum,
                maximum,
                mouse,
                self.focus.is(index),
                keyboard_adjustment(actions, self.focus.is(index)),
                step,
                &self.theme,
            );
            self.settings_dirty |= (previous - *value).abs() > f32::EPSILON;
        }
        self.audio.apply_volume(&self.settings);
        self.draw_skip_preferences(mouse, actions);
        if compact_button(
            Rect::new(440.0, 645.0, 400.0, 42.0),
            "Accessibility",
            mouse,
            self.focus.is(9),
            self.focus.is(9) && actions.pressed(UiAction::Activate),
            &self.theme,
        ) {
            self.overlay = Some(Overlay::Accessibility);
        }
        if is_mouse_button_released(MouseButton::Left) {
            self.persist_settings();
        }
    }

    fn draw_skip_preferences(&mut self, mouse: Vec2, actions: &UiActions) {
        let skip_unread = if self.settings.skip_unread {
            "Skip: all text"
        } else {
            "Skip: read text only"
        };
        let after_choices = if self.settings.skip_after_choices {
            "After choices: keep skipping"
        } else {
            "After choices: stop skipping"
        };
        for (index, x, label, value) in [
            (7, 340.0, skip_unread, &mut self.settings.skip_unread),
            (
                8,
                650.0,
                after_choices,
                &mut self.settings.skip_after_choices,
            ),
        ] {
            if compact_button(
                Rect::new(x, 565.0, 290.0, 42.0),
                label,
                mouse,
                self.focus.is(index),
                self.focus.is(index) && actions.pressed(UiAction::Activate),
                &self.theme,
            ) {
                *value = !*value;
                self.settings_dirty = true;
            }
        }
    }

    pub(super) fn persist_settings(&mut self) {
        if self.settings_dirty {
            match self.settings.save(&self.settings_path) {
                Ok(()) => self.settings_dirty = false,
                Err(error) => self.notice = Some((error, 6.0)),
            }
        }
    }

    pub(super) fn overlay_backdrop(
        &mut self,
        title: &str,
        mouse: Vec2,
        actions: &UiActions,
        scope: FocusScope,
        count: usize,
        initial: Option<usize>,
    ) -> bool {
        draw_rectangle(
            0.0,
            0.0,
            CANVAS_WIDTH,
            CANVAS_HEIGHT,
            color_alpha(&self.theme.panel_color, 0.98),
        );
        draw_text(
            super::ui_text::tr(title),
            64.0,
            64.0,
            f32::from(self.theme.heading_font_size),
            color(&self.theme.text_color),
        );
        self.focus
            .update(scope, count, initial, FocusAxis::Vertical, actions);
        let closed = compact_button(
            Rect::new(1100.0, 20.0, 116.0, 38.0),
            "Back",
            mouse,
            self.focus.is(0),
            self.focus.is(0) && actions.pressed(UiAction::Activate),
            &self.theme,
        ) || actions.pressed(UiAction::Back);
        if closed {
            self.overlay = None;
        }
        closed
    }
}
