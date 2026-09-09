use super::app::App;
use super::ui_common::{ButtonState, slot_button, toolbar_button};
use crate::frontend::{FocusScope, UiAction, UiActions};
use macroquad::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SlotGroup {
    Manual,
    Quick,
    Auto,
}

impl SlotGroup {
    const fn details(self) -> (&'static str, &'static str, usize) {
        match self {
            Self::Manual => ("Manual", "slot", 6),
            Self::Quick => ("Quick", "quick", 3),
            Self::Auto => ("Auto", "auto", 5),
        }
    }
}

impl App {
    #[allow(clippy::too_many_lines)]
    pub(super) fn draw_slots(&mut self, mouse: Vec2, actions: &UiActions, saving: bool) {
        let kind = if saving {
            renrs::screens::ScreenKind::Save
        } else {
            renrs::screens::ScreenKind::Load
        };
        if self.draw_custom_screen(kind, mouse, actions) {
            return;
        }
        if saving {
            self.slot_group = SlotGroup::Manual;
        }
        let scope = if saving {
            FocusScope::SaveSlots
        } else {
            FocusScope::LoadSlots
        };
        let (_, prefix, count) = self.slot_group.details();
        let tab_count = if saving { 0 } else { 3 };
        if self.overlay_backdrop(
            if saving { "Save" } else { "Load" },
            mouse,
            actions,
            scope,
            1 + tab_count + count * 2 + usize::from(self.slot_group == SlotGroup::Manual) * 2,
            Some(1 + tab_count),
        ) {
            return;
        }
        if !saving {
            for (index, group) in [SlotGroup::Manual, SlotGroup::Quick, SlotGroup::Auto]
                .into_iter()
                .enumerate()
            {
                if toolbar_button(
                    Rect::new(420.0 + index as f32 * 148.0, 24.0, 136.0, 42.0),
                    group.details().0,
                    mouse,
                    self.slot_group == group,
                    ButtonState::new(
                        true,
                        self.focus.is(index + 1),
                        self.focus.is(index + 1) && actions.pressed(UiAction::Activate),
                    ),
                    &self.theme,
                ) {
                    self.slot_group = group;
                    return;
                }
            }
        }
        if self.slot_group == SlotGroup::Manual {
            if actions.pressed(UiAction::PageUp) {
                self.storage.page = self.storage.page.saturating_sub(1);
            }
            if actions.pressed(UiAction::PageDown) {
                self.storage.page = (self.storage.page + 1).min(9);
            }
            for (offset, label) in ["<", ">"].into_iter().enumerate() {
                if toolbar_button(
                    Rect::new(500.0 + offset as f32 * 220.0, 655.0, 56.0, 40.0),
                    label,
                    mouse,
                    false,
                    ButtonState::new(
                        true,
                        self.focus.is(1 + tab_count + count * 2 + offset),
                        self.focus.is(1 + tab_count + count * 2 + offset)
                            && actions.pressed(UiAction::Activate),
                    ),
                    &self.theme,
                ) {
                    self.storage.page = if offset == 0 {
                        self.storage.page.saturating_sub(1)
                    } else {
                        (self.storage.page + 1).min(9)
                    };
                }
            }
            super::ui_common::draw_centered(
                &format!("{} / 10", self.storage.page + 1),
                638.0,
                682.0,
                20,
                super::ui_common::color(&self.theme.text_color),
            );
        }
        let start = if self.slot_group == SlotGroup::Manual {
            self.storage.page * 6
        } else {
            0
        };
        self.prepare_slot_previews(prefix, start, count);
        let layout = self.theme.layout.slots;
        for index in 1..=count {
            let name = format!("{prefix}-{}", start + index);
            let existing = self.storage.slots.iter().find(|slot| slot.name == name);
            let compatible = existing.is_some_and(|slot| {
                !slot.corrupt
                    && (slot.project_id.is_empty() || slot.project_id == self.program.project_id)
            });
            let focused = self.focus.is(tab_count + index);
            if slot_button(
                Rect::new(
                    layout.x + 104.0,
                    layout.y + (index - 1) as f32 * (layout.item_height + layout.gap),
                    layout.width - 168.0,
                    layout.item_height,
                ),
                start + index,
                existing,
                mouse,
                ButtonState::new(
                    saving || compatible,
                    focused,
                    focused && actions.pressed(UiAction::Activate),
                ),
                &self.theme,
            ) {
                if saving {
                    self.save_slot(&name);
                } else {
                    self.load_slot(&name);
                }
                return;
            }
            let tool_rect = Rect::new(
                layout.x + layout.width - 56.0,
                layout.y + (index - 1) as f32 * (layout.item_height + layout.gap),
                56.0,
                layout.item_height,
            );
            if let Some(texture) = self.storage.thumbnails.get(&name) {
                draw_texture_ex(
                    texture,
                    layout.x,
                    tool_rect.y,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(vec2(96.0, layout.item_height)),
                        ..Default::default()
                    },
                );
            }
            if toolbar_button(
                tool_rect,
                "...",
                mouse,
                false,
                ButtonState::new(
                    true,
                    self.focus.is(tab_count + count + index),
                    self.focus.is(tab_count + count + index) && actions.pressed(UiAction::Activate),
                ),
                &self.theme,
            ) {
                self.storage.tools_slot = name;
                self.storage.note = existing.map_or_else(String::new, |slot| slot.note.clone());
                self.overlay = Some(super::app::Overlay::SaveTools);
            }
        }
    }
}
