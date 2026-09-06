use super::app::{App, Overlay};
use super::settings::Settings;
use super::text::draw_text;
use super::ui_common::{button, color, color_alpha, slider};
use super::ui_text::tr;
use crate::frontend::{UiAction, UiActions};
use macroquad::prelude::*;
use renrs::theme::Theme;

pub(super) fn accessible_theme(project: &Theme, settings: &Settings) -> Theme {
    let mut theme = project.clone();
    theme.dialogue_font_size =
        (f32::from(project.dialogue_font_size) * settings.font_scale).round() as u16;
    theme.dialogue_line_height *= settings.font_scale;
    theme.reduced_motion |= settings.reduced_motion;
    theme.high_contrast |= settings.high_contrast;
    if theme.high_contrast {
        "#000000".clone_into(&mut theme.panel_color);
        "#242424".clone_into(&mut theme.surface_color);
        "#ffffff".clone_into(&mut theme.text_color);
        "#eeeeee".clone_into(&mut theme.muted_text_color);
        "#ffff00".clone_into(&mut theme.focus_color);
    }
    theme
}

impl App {
    #[allow(clippy::too_many_lines)]
    pub(super) fn draw_accessibility(&mut self, mouse: Vec2, actions: &UiActions) {
        draw_rectangle(
            0.0,
            0.0,
            1280.0,
            720.0,
            color_alpha(&self.theme.panel_color, 0.98),
        );
        super::ui_common::draw_fitted_centered(
            Rect::new(300.0, 60.0, 680.0, 64.0),
            &tr("Accessibility"),
            30,
            color(&self.theme.text_color),
        );
        let before = self.settings.clone();
        if actions.pressed(UiAction::FocusNext) || actions.pressed(UiAction::Down) {
            self.storage.accessibility_focus = (self.storage.accessibility_focus + 1) % 6;
        }
        if actions.pressed(UiAction::FocusPrevious) || actions.pressed(UiAction::Up) {
            self.storage.accessibility_focus = (self.storage.accessibility_focus + 5) % 6;
        }
        let focus = self.storage.accessibility_focus;
        self.settings.font_scale = slider(
            Rect::new(320.0, 220.0, 600.0, 48.0),
            "Text size",
            self.settings.font_scale,
            0.8,
            1.5,
            mouse,
            focus == 0,
            super::ui_common::keyboard_adjustment(actions, focus == 0),
            0.1,
            &self.theme,
        );
        for (index, label, value) in [
            (0, "High contrast", &mut self.settings.high_contrast),
            (1, "Reduced motion", &mut self.settings.reduced_motion),
            (2, "Wait for voice", &mut self.settings.wait_voice),
            (3, "Self voicing", &mut self.settings.self_voicing),
        ] {
            let rect = Rect::new(320.0, 300.0 + index as f32 * 64.0, 600.0, 48.0);
            draw_rectangle_lines(
                rect.x,
                rect.y + 8.0,
                28.0,
                28.0,
                2.0,
                color(&self.theme.text_color),
            );
            if *value {
                draw_line(
                    rect.x + 5.0,
                    rect.y + 20.0,
                    rect.x + 12.0,
                    rect.y + 29.0,
                    3.0,
                    color(&self.theme.focus_color),
                );
                draw_line(
                    rect.x + 12.0,
                    rect.y + 29.0,
                    rect.x + 24.0,
                    rect.y + 13.0,
                    3.0,
                    color(&self.theme.focus_color),
                );
            }
            draw_text(
                tr(label),
                rect.x + 48.0,
                rect.y + 30.0,
                24.0,
                color(&self.theme.text_color),
            );
            super::ui_common::draw_focus_outline(rect, focus == index + 1, &self.theme);
            super::speech::label(&tr(label), focus == index + 1);
            if rect.contains(mouse) && is_mouse_button_pressed(MouseButton::Left)
                || focus == index + 1 && actions.pressed(UiAction::Activate)
            {
                *value = !*value;
            }
        }
        if (before.font_scale - self.settings.font_scale).abs() > f32::EPSILON
            || before.high_contrast != self.settings.high_contrast
            || before.reduced_motion != self.settings.reduced_motion
            || before.wait_voice != self.settings.wait_voice
            || before.self_voicing != self.settings.self_voicing
        {
            self.settings_dirty = true;
            self.theme = accessible_theme(&self.project_theme, &self.settings);
            self.dialogue_view.invalidate();
            self.history_view.invalidate();
        }
        if button(
            Rect::new(530.0, 600.0, 220.0, 50.0),
            "Close",
            mouse,
            true,
            focus == 5,
            actions.pressed(UiAction::Back) || focus == 5 && actions.pressed(UiAction::Activate),
            &self.theme,
        ) {
            self.persist_settings();
            self.overlay = Some(Overlay::Settings);
        }
    }
}
