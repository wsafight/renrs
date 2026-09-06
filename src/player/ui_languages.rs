use super::app::{App, Overlay};
use super::ui_common::{ButtonState, toolbar_button};
use crate::frontend::{FocusScope, UiAction, UiActions};
use macroquad::prelude::*;

impl App {
    pub(super) fn draw_languages(&mut self, mouse: Vec2, actions: &UiActions) {
        let mut languages = vec![None];
        languages.extend(
            self.localizer
                .languages()
                .map(|language| Some(language.to_owned())),
        );
        let current = languages
            .iter()
            .position(|language| language.as_deref() == self.localizer.language())
            .unwrap_or(0);
        if self.overlay_backdrop(
            "Language",
            mouse,
            actions,
            FocusScope::Languages,
            languages.len() + 1,
            Some(current + 1),
        ) {
            self.overlay = Some(Overlay::Settings);
            return;
        }
        let rows = 9;
        if let Some(selected) = self.focus.selected().and_then(|index| index.checked_sub(1)) {
            if selected < self.language_offset {
                self.language_offset = selected;
            }
            if selected >= self.language_offset + rows {
                self.language_offset = selected + 1 - rows;
            }
        }
        self.language_offset = self
            .language_offset
            .saturating_add_signed(-mouse_wheel().1.round() as isize)
            .min(languages.len().saturating_sub(rows));
        for (index, language) in languages
            .iter()
            .enumerate()
            .skip(self.language_offset)
            .take(rows)
        {
            let rect = Rect::new(
                260.0,
                110.0 + (index - self.language_offset) as f32 * 60.0,
                760.0,
                48.0,
            );
            if toolbar_button(
                rect,
                language.as_deref().unwrap_or("Source text"),
                mouse,
                index == current,
                ButtonState::new(
                    true,
                    self.focus.is(index + 1),
                    self.focus.is(index + 1) && actions.pressed(UiAction::Activate),
                ),
                &self.theme,
            ) {
                self.select_language(language.clone());
                break;
            }
        }
    }

    pub(super) fn select_language(&mut self, language: Option<String>) {
        let mut localizer = self.localizer.clone();
        let result = localizer
            .set_language(language.clone())
            .map_err(|error| error.to_string())
            .and_then(|()| {
                self.runtime.as_mut().map_or(Ok(()), |runtime| {
                    runtime
                        .set_localizer(localizer.clone())
                        .map_err(|error| error.to_string())
                })
            });
        if let Err(error) = result {
            self.notice = Some((error, 6.0));
            return;
        }
        self.localizer = localizer;
        super::ui_text::set_localizer(self.localizer.clone());
        self.settings.language = language;
        self.settings.language_selected = true;
        self.settings_dirty = true;
        self.history_view.invalidate();
        self.dialogue_view.invalidate();
        self.visible_characters = 0.0;
        self.auto_remaining = self.settings.auto_delay;
        self.selected_choice = 0;
        if let Err(error) = self.settings.save(&self.settings_path) {
            self.notice = Some((error, 6.0));
        } else {
            self.settings_dirty = false;
        }
        self.overlay = Some(Overlay::Settings);
    }
}
