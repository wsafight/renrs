use super::text::draw_text;
use macroquad::prelude::*;
use renrs::Runtime;

use crate::frontend::{FocusAxis, FocusScope, UiAction, UiActions};

use super::app::{App, Overlay};
use super::ui_common::{ButtonState, color, color_alpha, toolbar_button};

impl App {
    pub(super) fn draw_toolbar(&mut self, mouse: Vec2, actions: &UiActions) {
        self.focus.update(
            FocusScope::PlayingToolbar,
            7,
            None,
            FocusAxis::Horizontal,
            actions,
        );
        if actions.pressed(UiAction::Back) {
            self.focus.clear();
        }
        draw_rectangle(
            self.theme.layout.toolbar.x,
            self.theme.layout.toolbar.y,
            self.theme.layout.toolbar.width,
            self.theme.layout.toolbar.height,
            color_alpha(&self.theme.panel_color, 0.94),
        );
        draw_text(
            &self.program.title,
            self.theme.layout.toolbar.x + 24.0,
            self.theme.layout.toolbar.y + self.theme.layout.toolbar.height / 2.0 + 8.0,
            f32::from(self.theme.ui_font_size.saturating_sub(2)),
            color(&self.theme.text_color),
        );
        let can_rollback = self.runtime.as_ref().is_some_and(Runtime::can_rollback);
        let mut x = self.theme.layout.toolbar_buttons_x;
        let button_y =
            self.theme.layout.toolbar.y + (self.theme.layout.toolbar.height - 36.0) / 2.0;
        if toolbar_button(
            Rect::new(x, button_y, 76.0, 36.0),
            "Back",
            mouse,
            false,
            self.toolbar_button_state(0, can_rollback, actions),
            &self.theme,
        ) {
            self.rollback_story();
        }
        x += 84.0;
        if toolbar_button(
            Rect::new(x, button_y, 76.0, 36.0),
            "Auto",
            mouse,
            self.playback.auto,
            self.toolbar_button_state(1, true, actions),
            &self.theme,
        ) {
            self.playback.auto = !self.playback.auto;
            self.auto_remaining = self.settings.auto_delay;
        }
        x += 84.0;
        if toolbar_button(
            Rect::new(x, button_y, 76.0, 36.0),
            "Skip",
            mouse,
            self.playback.skip_read,
            self.toolbar_button_state(2, true, actions),
            &self.theme,
        ) {
            self.playback.skip_read = !self.playback.skip_read;
            self.skip_remaining = 0.08;
        }
        x += 84.0;
        let buttons = [
            ("Save", Overlay::SaveSlots, 76.0),
            ("Load", Overlay::LoadSlots, 76.0),
            ("History", Overlay::History, 92.0),
            ("Settings", Overlay::Settings, 100.0),
        ];
        for (offset, (label, overlay, width)) in buttons.into_iter().enumerate() {
            let focus_index = offset + 3;
            if toolbar_button(
                Rect::new(x, button_y, width, 36.0),
                label,
                mouse,
                false,
                self.toolbar_button_state(focus_index, true, actions),
                &self.theme,
            ) {
                self.overlay = Some(overlay);
            }
            x += width + 8.0;
        }
        if actions.pressed(UiAction::QuickSave) {
            self.quick_save();
        } else if actions.pressed(UiAction::QuickLoad) {
            self.quick_load();
        } else if actions.pressed(UiAction::OpenHistory) {
            self.overlay = Some(Overlay::History);
        }
    }

    fn toolbar_button_state(
        &self,
        index: usize,
        enabled: bool,
        actions: &UiActions,
    ) -> ButtonState {
        ButtonState::new(
            enabled,
            self.focus.is(index),
            self.focus.is(index) && actions.pressed(UiAction::Activate),
        )
    }
}
