use super::app::App;
use super::ui_common::wrap_lines;
use super::ui_screen_actions::ScreenCommand;
use renrs::save::SaveSlot;
use renrs::screens::{ListSource, ScreenKind};
use std::sync::Arc;

pub(super) struct ListItem {
    pub(super) text: String,
    pub(super) slot: Option<SaveSlot>,
    pub(super) command: Option<ScreenCommand>,
}

impl App {
    pub(super) fn screen_list_items(
        &mut self,
        source: ListSource,
        kind: ScreenKind,
        width: f32,
        font_size: u16,
    ) -> Arc<[ListItem]> {
        let source = if matches!(source, ListSource::Saves) {
            match self.slot_group {
                super::ui_slots::SlotGroup::Quick if kind == ScreenKind::Load => {
                    ListSource::QuickSaves
                }
                super::ui_slots::SlotGroup::Auto if kind == ScreenKind::Load => {
                    ListSource::AutoSaves
                }
                _ => ListSource::ManualSaves,
            }
        } else {
            source
        };
        match source {
            ListSource::History => {
                let key = (width.to_bits(), font_size);
                if let Some(items) = self.history_view.custom.get(&key) {
                    return Arc::clone(items);
                }
                let items: Arc<[ListItem]> = self
                    .runtime
                    .as_ref()
                    .map_or_else(Vec::new, |runtime| {
                        runtime
                            .history()
                            .iter()
                            .flat_map(|dialogue| {
                                let text = dialogue.speaker_name.as_ref().map_or_else(
                                    || dialogue.text.clone(),
                                    |name| format!("{name}: {}", dialogue.text),
                                );
                                wrap_lines(&text, width - 24.0, font_size)
                                    .into_iter()
                                    .map(|text| ListItem {
                                        text,
                                        slot: None,
                                        command: None,
                                    })
                            })
                            .collect()
                    })
                    .into();
                self.history_view.custom.insert(key, Arc::clone(&items));
                items
            }
            ListSource::Languages => std::iter::once(None)
                .chain(
                    self.localizer
                        .languages()
                        .map(|language| Some(language.to_owned())),
                )
                .map(|language| ListItem {
                    text: language.as_deref().unwrap_or("Source text").to_owned(),
                    slot: None,
                    command: Some(ScreenCommand::Language(language)),
                })
                .collect(),
            ListSource::Saves
            | ListSource::ManualSaves
            | ListSource::QuickSaves
            | ListSource::AutoSaves => {
                let (prefix, count) = match source {
                    ListSource::ManualSaves => ("slot", 60),
                    ListSource::QuickSaves => ("quick", 3),
                    _ => ("auto", 5),
                };
                let key = (prefix.to_owned(), kind == ScreenKind::Save);
                if let Some(items) = self.storage.screen_lists.get(&key) {
                    return items.clone();
                }
                let slots = &self.storage.slots;
                let items: Arc<[ListItem]> = (1..=count)
                    .map(|index| {
                        let name = format!("{prefix}-{index}");
                        let slot = slots.iter().find(|slot| slot.name == name).cloned();
                        let command = if kind == ScreenKind::Save && prefix == "slot" {
                            Some(ScreenCommand::Save(name.clone()))
                        } else if slot.as_ref().is_some_and(|slot| {
                            !slot.corrupt
                                && (slot.project_id.is_empty()
                                    || slot.project_id == self.program.project_id)
                        }) {
                            Some(ScreenCommand::Load(name.clone()))
                        } else {
                            None
                        };
                        ListItem {
                            text: name,
                            slot,
                            command,
                        }
                    })
                    .collect();
                self.storage.screen_lists.insert(key, items.clone());
                items
            }
        }
    }
}
