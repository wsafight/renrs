use super::app::{App, Overlay};
use super::ui_common::{button, color, draw_fitted_centered};
use crate::frontend::{FocusScope, UiAction, UiActions};
use macroquad::prelude::*;

impl App {
    pub(super) fn draw_collection(&mut self, mouse: Vec2, actions: &UiActions) {
        if self.overlay_backdrop(
            "Collection",
            mouse,
            actions,
            FocusScope::History,
            1,
            Some(0),
        ) {
            return;
        }
        let profile = &self.storage.profile;
        let config = &self.program.progress;
        let items: Vec<_> = [
            (&config.achievements, &profile.achievements),
            (&config.gallery, &profile.gallery),
            (&config.endings, &profile.endings),
        ]
        .into_iter()
        .flat_map(|(items, unlocked)| {
            items
                .iter()
                .map(move |item| (item, unlocked.contains(&item.id)))
        })
        .collect();
        let pages = items.len().div_ceil(6).max(1);
        let page = self
            .screen_scroll
            .entry("collection".to_owned())
            .or_default();
        if actions.pressed(UiAction::PageUp) {
            *page = page.saturating_sub(1);
        }
        if actions.pressed(UiAction::PageDown) {
            *page = (*page + 1).min(pages - 1);
        }
        for (index, (item, unlocked)) in items.iter().skip(*page * 6).take(6).enumerate() {
            let x = 80.0 + (index % 3) as f32 * 400.0;
            let y = 130.0 + (index / 3) as f32 * 230.0;
            if *unlocked
                && let Some(texture) = item
                    .image
                    .as_ref()
                    .and_then(|path| self.assets.textures.get(path))
            {
                draw_texture_ex(
                    texture,
                    x,
                    y,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(vec2(340.0, 160.0)),
                        ..Default::default()
                    },
                );
            }
            draw_fitted_centered(
                Rect::new(x, y + 164.0, 340.0, 48.0),
                &super::ui_text::tr(if *unlocked { &item.title } else { "Locked" }),
                24,
                color(&self.theme.text_color),
            );
        }
        for (index, label) in ["<", ">"].into_iter().enumerate() {
            if button(
                Rect::new(510.0 + index as f32 * 200.0, 640.0, 60.0, 44.0),
                label,
                mouse,
                true,
                false,
                false,
                &self.theme,
            ) {
                *page = if index == 0 {
                    page.saturating_sub(1)
                } else {
                    (*page + 1).min(pages - 1)
                };
            }
        }
        draw_fitted_centered(
            Rect::new(570.0, 640.0, 140.0, 44.0),
            &format!("{} / {pages}", *page + 1),
            20,
            color(&self.theme.text_color),
        );
    }

    pub(super) fn collection_button(&mut self, mouse: Vec2) {
        if button(
            Rect::new(1000.0, 640.0, 200.0, 44.0),
            "Collection",
            mouse,
            self.overlay.is_none(),
            false,
            false,
            &self.theme,
        ) {
            self.overlay = Some(Overlay::Collection);
        }
    }
}
