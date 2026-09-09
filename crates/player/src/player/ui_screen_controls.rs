use super::app::App;
use super::ui_common::{color, draw_fitted_centered, draw_focus_outline};
use crate::frontend::{UiAction, UiActions};
use macroquad::prelude::*;
use renrs::screens::TogglePreference;

impl App {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn screen_toggle(
        &mut self,
        setting: TogglePreference,
        text: &str,
        rect: Rect,
        mouse: Vec2,
        actions: &UiActions,
        interactive: bool,
        focus: usize,
    ) {
        let value = match setting {
            TogglePreference::HighContrast => &mut self.settings.high_contrast,
            TogglePreference::ReducedMotion => &mut self.settings.reduced_motion,
            TogglePreference::WaitVoice => &mut self.settings.wait_voice,
            TogglePreference::SelfVoicing => &mut self.settings.self_voicing,
        };
        let square = Rect::new(rect.x + 8.0, rect.y + (rect.h - 24.0) / 2.0, 24.0, 24.0);
        draw_rectangle_lines(
            square.x,
            square.y,
            square.w,
            square.h,
            2.0,
            color(&self.theme.text_color),
        );
        if *value {
            draw_rectangle(
                square.x + 5.0,
                square.y + 5.0,
                14.0,
                14.0,
                color(&self.theme.focus_color),
            );
        }
        draw_fitted_centered(
            Rect::new(rect.x + 40.0, rect.y, rect.w - 40.0, rect.h),
            text,
            self.theme.ui_font_size,
            color(&self.theme.text_color),
        );
        draw_focus_outline(rect, interactive && self.focus.is(focus), &self.theme);
        if interactive
            && (rect.contains(mouse) && is_mouse_button_pressed(MouseButton::Left)
                || self.focus.is(focus) && actions.pressed(UiAction::Activate))
        {
            *value = !*value;
            self.settings_dirty = true;
            self.theme =
                super::ui_accessibility::accessible_theme(&self.project_theme, &self.settings);
            self.dialogue_view.invalidate();
        }
    }
}
