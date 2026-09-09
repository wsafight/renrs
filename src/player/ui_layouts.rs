use super::app::{App, Overlay, Screen};
use renrs::screens::{PlacedElement, ScreenKind, Screens, Widget, layout};
use std::sync::Arc;

pub(super) struct ScreenLayouts(Vec<(ScreenKind, Arc<[PlacedElement]>, Vec<String>)>);

impl App {
    pub(super) fn active_screen_images(&self) -> Vec<String> {
        let base = if self.screen == Screen::MainMenu {
            ScreenKind::MainMenu
        } else {
            ScreenKind::Hud
        };
        let mut kinds = vec![base];
        if self.screen != Screen::MainMenu {
            match self.runtime.as_ref().and_then(renrs::Runtime::waiting) {
                Some(renrs::WaitState::Dialogue) => kinds.push(ScreenKind::Dialogue),
                Some(renrs::WaitState::Choice { .. }) => kinds.push(ScreenKind::Choices),
                _ => {}
            }
        }
        if let Some(kind) = self.overlay.and_then(|overlay| match overlay {
            Overlay::History => Some(ScreenKind::History),
            Overlay::Settings => Some(ScreenKind::Settings),
            Overlay::SaveSlots => Some(ScreenKind::Save),
            Overlay::LoadSlots => Some(ScreenKind::Load),
            _ => None,
        }) {
            kinds.push(kind);
        }
        let mut images = kinds
            .into_iter()
            .flat_map(|kind| self.screen_layouts.images(kind))
            .collect::<Vec<_>>();
        if let Some(runtime) = &self.runtime {
            let mut names = runtime.stage().shown_screens.clone();
            if let Some(renrs::WaitState::Screen { name }) = runtime.waiting()
                && !names.contains(name)
            {
                names.push(name.clone());
            }
            images.extend(names.into_iter().flat_map(|name| {
                self.screens
                    .story
                    .get(&name)
                    .into_iter()
                    .flat_map(|screen| layout(screen).unwrap_or_default())
                    .filter_map(|element| match element.widget {
                        Widget::Image { path } => Some(path),
                        _ => None,
                    })
            }));
        }
        images
    }
}

impl ScreenLayouts {
    pub(super) fn new(screens: &Screens) -> Self {
        Self(
            [
                ScreenKind::MainMenu,
                ScreenKind::Hud,
                ScreenKind::Dialogue,
                ScreenKind::Choices,
                ScreenKind::Save,
                ScreenKind::Load,
                ScreenKind::History,
                ScreenKind::Settings,
            ]
            .into_iter()
            .filter_map(|kind| {
                let elements = layout(screens.get(kind)?).ok()?;
                let images = elements
                    .iter()
                    .filter_map(|element| match &element.widget {
                        Widget::Image { path } => Some(path.clone()),
                        _ => None,
                    })
                    .collect();
                Some((kind, elements.into(), images))
            })
            .collect(),
        )
    }
    pub(super) fn elements(&self, kind: ScreenKind) -> Option<Arc<[PlacedElement]>> {
        self.0
            .iter()
            .find(|(key, _, _)| *key == kind)
            .map(|(_, elements, _)| elements.clone())
    }
    pub(super) fn images(&self, kind: ScreenKind) -> impl Iterator<Item = String> + '_ {
        self.0
            .iter()
            .filter(move |(key, _, _)| *key == kind)
            .flat_map(|(_, _, images)| images.iter().cloned())
    }
}
