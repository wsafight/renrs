use super::app::{App, Overlay, Screen};
use renrs::screens::{PlacedElement, ScreenKind, Screens, Widget, layout};
use renrs::syntax::Expr;
use std::collections::BTreeMap;
use std::sync::Arc;

pub(super) struct ScreenLayouts {
    fixed: Vec<(ScreenKind, Arc<[PlacedElement]>, Vec<String>)>,
    // `Invalid` marks a story screen whose layout failed to build: it exists
    // but draws nothing, matching the previous lazy-layout behaviour.
    story: BTreeMap<String, StoryLayout>,
}

pub(super) enum StoryLayout {
    Invalid,
    Ready(StoryScreen),
}

pub(super) struct StoryScreen {
    pub(super) elements: Arc<[PlacedElement]>,
    pub(super) images: Arc<[String]>,
    pub(super) visible: Arc<[Visibility]>,
}

/// Visibility of a placed element, resolved once when screens load instead of
/// re-parsing the condition expression every frame.
pub(super) enum Visibility {
    Always,
    Never,
    When(Expr),
}

fn visibility(expression: Option<&str>) -> Visibility {
    let Some(source) = expression else {
        return Visibility::Always;
    };
    match renrs::expression::parse_condition(source, "screens.json", 1, 1) {
        Ok(expr) => Visibility::When(expr),
        Err(_) => Visibility::Never,
    }
}

fn image_paths(elements: &[PlacedElement]) -> impl Iterator<Item = String> + '_ {
    elements.iter().filter_map(|element| match &element.widget {
        Widget::Image { path } => Some(path.clone()),
        _ => None,
    })
}

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
            for name in &names {
                if let Some(StoryLayout::Ready(screen)) = self.screen_layouts.story(name) {
                    images.extend(screen.images.iter().cloned());
                }
            }
        }
        images
    }
}

impl ScreenLayouts {
    pub(super) fn new(screens: &Screens) -> Self {
        let fixed = [
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
            let elements: Arc<[PlacedElement]> = layout(screens.get(kind)?).ok()?.into();
            let images = image_paths(&elements).collect();
            Some((kind, elements, images))
        })
        .collect();
        let story = screens
            .story
            .iter()
            .map(|(name, screen)| {
                let entry = layout(screen).map_or(StoryLayout::Invalid, |elements| {
                    let elements: Arc<[PlacedElement]> = elements.into();
                    StoryLayout::Ready(StoryScreen {
                        images: image_paths(&elements).collect(),
                        visible: elements
                            .iter()
                            .map(|element| visibility(element.visible.as_deref()))
                            .collect(),
                        elements,
                    })
                });
                (name.clone(), entry)
            })
            .collect();
        Self { fixed, story }
    }
    pub(super) fn elements(&self, kind: ScreenKind) -> Option<Arc<[PlacedElement]>> {
        self.fixed
            .iter()
            .find(|(key, _, _)| *key == kind)
            .map(|(_, elements, _)| elements.clone())
    }
    pub(super) fn story(&self, name: &str) -> Option<StoryLayout> {
        self.story.get(name).map(|entry| match entry {
            StoryLayout::Invalid => StoryLayout::Invalid,
            StoryLayout::Ready(screen) => StoryLayout::Ready(StoryScreen {
                elements: Arc::clone(&screen.elements),
                images: Arc::clone(&screen.images),
                visible: Arc::clone(&screen.visible),
            }),
        })
    }
    pub(super) fn images(&self, kind: ScreenKind) -> impl Iterator<Item = String> + '_ {
        self.fixed
            .iter()
            .filter(move |(key, _, _)| *key == kind)
            .flat_map(|(_, _, images)| images.iter().cloned())
    }
}
