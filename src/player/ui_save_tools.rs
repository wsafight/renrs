use super::app::{App, Overlay};
use super::saving::Confirmation;
use super::text::draw_text;
use super::ui_common::{button, color, color_alpha, draw_fitted_centered};
use crate::frontend::{FocusAxis, FocusScope, UiAction, UiActions};
use macroquad::prelude::*;
use renrs::save::worker::SaveRequest;

impl App {
    pub(super) fn draw_confirmation(&mut self, mouse: Vec2, actions: &UiActions) {
        self.focus.update(
            FocusScope::Confirmation,
            2,
            Some(0),
            FocusAxis::Horizontal,
            actions,
        );
        draw_rectangle(
            0.0,
            0.0,
            1280.0,
            720.0,
            color_alpha(&self.theme.panel_color, 0.98),
        );
        let title = match &self.storage.confirmation {
            Some(Confirmation::Save(_)) => "Replace this save?",
            Some(Confirmation::Load(_)) => "Load this save and replace current progress?",
            Some(Confirmation::Delete(_)) => "Delete this save?",
            Some(Confirmation::Quit) => "Save current progress and quit?",
            None => {
                self.overlay = None;
                return;
            }
        };
        draw_fitted_centered(
            Rect::new(260.0, 240.0, 760.0, 100.0),
            &super::ui_text::tr(title),
            28,
            color(&self.theme.text_color),
        );
        if button(
            Rect::new(400.0, 380.0, 220.0, 54.0),
            "Cancel",
            mouse,
            true,
            self.focus.is(0),
            actions.pressed(UiAction::Back)
                || self.focus.is(0) && actions.pressed(UiAction::Activate),
            &self.theme,
        ) {
            self.storage.confirmation = None;
            self.overlay = None;
            return;
        }
        if button(
            Rect::new(660.0, 380.0, 220.0, 54.0),
            "Confirm",
            mouse,
            true,
            self.focus.is(1),
            self.focus.is(1) && actions.pressed(UiAction::Activate),
            &self.theme,
        ) {
            self.confirm_pending();
        }
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn draw_save_tools(&mut self, mouse: Vec2, actions: &UiActions) {
        self.focus.update(
            FocusScope::SaveTools,
            7,
            Some(0),
            FocusAxis::Vertical,
            actions,
        );
        draw_rectangle(
            0.0,
            0.0,
            1280.0,
            720.0,
            color_alpha(&self.theme.panel_color, 0.98),
        );
        draw_fitted_centered(
            Rect::new(240.0, 52.0, 800.0, 50.0),
            &self.storage.tools_slot,
            28,
            color(&self.theme.text_color),
        );
        let note_rect = Rect::new(300.0, 220.0, 680.0, 56.0);
        let path_rect = Rect::new(300.0, 330.0, 680.0, 56.0);
        if is_mouse_button_pressed(MouseButton::Left) {
            if note_rect.contains(mouse) {
                self.focus.select(0);
            }
            if path_rect.contains(mouse) {
                self.focus.select(1);
            }
        }
        draw_text(
            super::ui_text::tr("Note"),
            300.0,
            204.0,
            22.0,
            color(&self.theme.text_color),
        );
        draw_text(
            super::ui_text::tr("Import / export file"),
            300.0,
            314.0,
            22.0,
            color(&self.theme.text_color),
        );
        text_field(
            note_rect,
            &mut self.storage.note,
            self.focus.is(0),
            &self.theme,
        );
        text_field(
            path_rect,
            &mut self.storage.file_path,
            self.focus.is(1),
            &self.theme,
        );
        let slot = self.storage.tools_slot.clone();
        let exists = self.storage.slots.iter().any(|save| save.name == slot);
        for (index, (label, enabled)) in [
            ("Save", self.runtime.is_some()),
            ("Delete", exists),
            ("Import", !self.storage.file_path.is_empty()),
            ("Export", exists && !self.storage.file_path.is_empty()),
        ]
        .into_iter()
        .enumerate()
        {
            if button(
                Rect::new(280.0 + index as f32 * 185.0, 440.0, 170.0, 54.0),
                label,
                mouse,
                enabled,
                self.focus.is(index + 2),
                self.focus.is(index + 2) && actions.pressed(UiAction::Activate),
                &self.theme,
            ) {
                match index {
                    0 => self.save_slot(&slot),
                    1 => {
                        self.storage.confirmation = Some(Confirmation::Delete(slot.clone()));
                        self.overlay = Some(Overlay::Confirm);
                    }
                    2 => {
                        if exists {
                            self.notice =
                                Some(("Choose an empty slot before importing".to_owned(), 5.0));
                        } else {
                            self.submit_file_action(SaveRequest::Import {
                                path: self.storage.file_path.clone().into(),
                                slot: slot.clone(),
                                project_id: self.program.project_id.clone(),
                            });
                        }
                    }
                    _ => self.submit_file_action(SaveRequest::Export {
                        path: self.storage.file_path.clone().into(),
                        slot: slot.clone(),
                    }),
                }
            }
        }
        if button(
            Rect::new(530.0, 550.0, 220.0, 50.0),
            "Close",
            mouse,
            true,
            self.focus.is(6),
            actions.pressed(UiAction::Back)
                || self.focus.is(6) && actions.pressed(UiAction::Activate),
            &self.theme,
        ) {
            self.overlay = Some(Overlay::LoadSlots);
        }
    }

    fn submit_file_action(&mut self, request: SaveRequest) {
        if let Err(error) = self.storage.worker.submit(request) {
            self.notice = Some((error, 5.0));
        }
    }
}

pub(super) fn text_field(
    rect: Rect,
    text: &mut String,
    focused: bool,
    theme: &renrs::theme::Theme,
) {
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, color(&theme.surface_color));
    if focused {
        draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            2.0,
            color(&theme.focus_color),
        );
        while let Some(character) = get_char_pressed() {
            if !character.is_control() && text.chars().count() < 1024 {
                text.push(character);
            }
        }
        if is_key_pressed(KeyCode::Backspace) {
            text.pop();
        }
        if (is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::LeftSuper))
            && is_key_pressed(KeyCode::V)
            && let Some(value) = macroquad::miniquad::window::clipboard_get()
        {
            text.extend(
                value
                    .chars()
                    .filter(|c| !c.is_control())
                    .take(1024 - text.chars().count()),
            );
        }
    }
    draw_fitted_centered(
        Rect::new(rect.x + 12.0, rect.y, rect.w - 24.0, rect.h),
        text,
        theme.ui_font_size,
        color(&theme.text_color),
    );
}
