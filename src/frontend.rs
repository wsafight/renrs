use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiAction {
    Activate,
    Back,
    FocusNext,
    FocusPrevious,
    Up,
    Down,
    Left,
    Right,
    QuickSave,
    QuickLoad,
    OpenHistory,
    PageUp,
    PageDown,
    Home,
    End,
}

#[derive(Debug, Clone, Default)]
pub struct UiActions {
    pressed: HashSet<UiAction>,
}

impl UiActions {
    #[must_use]
    pub fn new(actions: impl IntoIterator<Item = UiAction>) -> Self {
        Self {
            pressed: actions.into_iter().collect(),
        }
    }

    #[must_use]
    pub fn pressed(&self, action: UiAction) -> bool {
        self.pressed.contains(&action)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusScope {
    MainMenu,
    PlayingToolbar,
    History,
    Settings,
    Languages,
    SaveSlots,
    LoadSlots,
    Finished,
    Dialogue,
    Choices,
    SaveTools,
    Confirmation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusAxis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone)]
pub struct FocusState {
    scope: FocusScope,
    index: Option<usize>,
}

impl Default for FocusState {
    fn default() -> Self {
        Self {
            scope: FocusScope::MainMenu,
            index: Some(0),
        }
    }
}

impl FocusState {
    pub fn update(
        &mut self,
        scope: FocusScope,
        count: usize,
        initial: Option<usize>,
        axis: FocusAxis,
        actions: &UiActions,
    ) {
        if self.scope != scope {
            self.scope = scope;
            self.index = initial.filter(|index| *index < count);
        } else if self.index.is_some_and(|index| index >= count) {
            self.index = initial.filter(|index| *index < count);
        }
        if count == 0 {
            self.index = None;
            return;
        }

        let previous = actions.pressed(UiAction::FocusPrevious)
            || match axis {
                FocusAxis::Horizontal => actions.pressed(UiAction::Left),
                FocusAxis::Vertical => actions.pressed(UiAction::Up),
            };
        let next = actions.pressed(UiAction::FocusNext)
            || match axis {
                FocusAxis::Horizontal => actions.pressed(UiAction::Right),
                FocusAxis::Vertical => actions.pressed(UiAction::Down),
            };
        if previous {
            self.index = Some(
                self.index
                    .map_or(count - 1, |index| (index + count - 1) % count),
            );
        } else if next {
            self.index = Some(self.index.map_or(0, |index| (index + 1) % count));
        }
    }

    #[must_use]
    pub fn is(&self, index: usize) -> bool {
        self.index == Some(index)
    }

    #[must_use]
    pub const fn selected(&self) -> Option<usize> {
        self.index
    }

    pub fn clear(&mut self) {
        self.index = None;
    }

    pub fn select(&mut self, index: usize) {
        self.index = Some(index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_wraps_in_both_directions() {
        let mut focus = FocusState::default();
        focus.update(
            FocusScope::MainMenu,
            3,
            Some(0),
            FocusAxis::Vertical,
            &UiActions::new([UiAction::Up]),
        );
        assert_eq!(focus.selected(), Some(2));
        focus.update(
            FocusScope::MainMenu,
            3,
            Some(0),
            FocusAxis::Vertical,
            &UiActions::new([UiAction::Down]),
        );
        assert_eq!(focus.selected(), Some(0));
    }

    #[test]
    fn scope_change_uses_requested_initial_focus() {
        let mut focus = FocusState::default();
        focus.update(
            FocusScope::PlayingToolbar,
            7,
            None,
            FocusAxis::Horizontal,
            &UiActions::default(),
        );
        assert_eq!(focus.selected(), None);
        focus.update(
            FocusScope::Settings,
            6,
            Some(1),
            FocusAxis::Vertical,
            &UiActions::default(),
        );
        assert_eq!(focus.selected(), Some(1));
    }

    #[test]
    fn tab_enters_an_unfocused_toolbar() {
        let mut focus = FocusState::default();
        focus.update(
            FocusScope::PlayingToolbar,
            7,
            None,
            FocusAxis::Horizontal,
            &UiActions::default(),
        );
        focus.update(
            FocusScope::PlayingToolbar,
            7,
            None,
            FocusAxis::Horizontal,
            &UiActions::new([UiAction::FocusNext]),
        );
        assert_eq!(focus.selected(), Some(0));
    }
}
