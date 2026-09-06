use super::text::draw_text;
use macroquad::prelude::*;

use crate::frontend::{FocusAxis, FocusScope, UiAction, UiActions};

use super::app::{App, Overlay};
use super::ui_common::{button, color, color_alpha, draw_fitted_centered};

impl App {
    pub(super) fn draw_main_menu(&mut self, mouse: Vec2, actions: &UiActions) {
        if self.draw_custom_screen(renrs::screens::ScreenKind::MainMenu, mouse, actions) {
            return;
        }
        self.draw_background(self.title_background.as_deref());
        let panel = self.theme.layout.main_menu_panel;
        draw_rectangle(
            panel.x,
            panel.y,
            panel.width,
            panel.height,
            color_alpha(&self.theme.panel_color, 0.94),
        );
        self.draw_main_menu_heading();
        self.collection_button(mouse);

        let buttons = self.theme.layout.main_menu_buttons;
        let mut y = buttons.y;
        let interactive = self.overlay.is_none();
        if interactive {
            self.focus.update(
                FocusScope::MainMenu,
                5,
                Some(0),
                FocusAxis::Vertical,
                actions,
            );
        }
        if button(
            Rect::new(buttons.x, y, buttons.width, buttons.item_height),
            "New game",
            mouse,
            interactive,
            interactive && self.focus.is(0),
            interactive && self.focus.is(0) && actions.pressed(UiAction::Activate),
            &self.theme,
        ) {
            self.start_new_game();
        }
        y += buttons.item_height + buttons.gap;
        let has_saves = self.storage.slots.iter().any(|slot| {
            !slot.corrupt
                && (slot.project_id.is_empty() || slot.project_id == self.program.project_id)
        });
        if button(
            Rect::new(buttons.x, y, buttons.width, buttons.item_height),
            "Continue",
            mouse,
            interactive && has_saves,
            interactive && self.focus.is(1),
            interactive && self.focus.is(1) && actions.pressed(UiAction::Activate),
            &self.theme,
        ) {
            self.load_latest();
        }
        y += buttons.item_height + buttons.gap;
        if button(
            Rect::new(buttons.x, y, buttons.width, buttons.item_height),
            "Load",
            mouse,
            interactive && has_saves,
            interactive && self.focus.is(2),
            interactive && self.focus.is(2) && actions.pressed(UiAction::Activate),
            &self.theme,
        ) {
            self.overlay = Some(Overlay::LoadSlots);
        }
        y += buttons.item_height + buttons.gap;
        if button(
            Rect::new(buttons.x, y, buttons.width, buttons.item_height),
            "Settings",
            mouse,
            interactive,
            interactive && self.focus.is(3),
            interactive && self.focus.is(3) && actions.pressed(UiAction::Activate),
            &self.theme,
        ) {
            self.overlay = Some(Overlay::Settings);
        }
        y += buttons.item_height + buttons.gap;
        if button(
            Rect::new(buttons.x, y, buttons.width, buttons.item_height),
            "Quit",
            mouse,
            interactive,
            interactive && self.focus.is(4),
            interactive && self.focus.is(4) && actions.pressed(UiAction::Activate),
            &self.theme,
        ) {
            self.request_quit();
        }
        draw_text(
            concat!("v", env!("CARGO_PKG_VERSION")),
            panel.x + 68.0,
            panel.y + panel.height - 38.0,
            f32::from(self.theme.ui_font_size.saturating_sub(4)),
            color(&self.theme.muted_text_color),
        );
    }

    fn draw_main_menu_heading(&self) {
        let panel = self.theme.layout.main_menu_panel;
        draw_fitted_centered(
            Rect::new(panel.x + 40.0, panel.y + 48.0, panel.width - 80.0, 118.0),
            &self.program.title,
            self.theme.title_font_size,
            color(&self.theme.text_color),
        );
    }
}
