use super::app::App;
use super::ui_common::{color, color_alpha};
use crate::frontend::{FocusAxis, FocusScope, UiAction, UiActions};
use macroquad::prelude::*;
use renrs::screens::{Action, PlacedElement, Widget};
use renrs::syntax::Value;
use renrs::WaitState;

impl App {
    pub(super) fn draw_story_wait(&mut self, mouse: Vec2, actions: &UiActions) {
        let Some(name) = self
            .runtime
            .as_ref()
            .and_then(renrs::Runtime::waiting)
            .and_then(|waiting| match waiting {
                WaitState::Screen { name } => Some(name.clone()),
                _ => None,
            })
        else {
            return;
        };
        if actions.pressed(UiAction::Back) {
            self.continue_story();
            return;
        }
        self.draw_named_screen(&name, mouse, actions, true);
    }

    pub(super) fn draw_shown_screens(&mut self, mouse: Vec2, actions: &UiActions) {
        let names = self
            .runtime
            .as_ref()
            .map(|runtime| runtime.stage().shown_screens.clone())
            .unwrap_or_default();
        let waiting_screen = self
            .runtime
            .as_ref()
            .and_then(renrs::Runtime::waiting)
            .and_then(|waiting| match waiting {
                WaitState::Screen { name } => Some(name.clone()),
                _ => None,
            });
        for name in names {
            if waiting_screen.as_deref() == Some(name.as_str()) {
                continue;
            }
            self.draw_named_screen(&name, mouse, actions, false);
        }
    }

    fn draw_named_screen(
        &mut self,
        name: &str,
        mouse: Vec2,
        actions: &UiActions,
        interactive: bool,
    ) {
        let layout = self.screen_layouts.story(name);
        let Some(super::ui_layouts::StoryLayout::Ready(screen)) = layout else {
            if layout.is_none() && interactive {
                self.notice = Some((format!("unknown screen `{name}`"), 4.0));
            }
            return;
        };
        let focusable = screen
            .elements
            .iter()
            .zip(screen.visible.iter())
            .filter(|(element, visible)| {
                self.element_visible(visible)
                    && matches!(
                        element.widget,
                        Widget::Hotspot { .. } | Widget::Button { .. }
                    )
            })
            .count();
        if interactive {
            self.focus.update(
                FocusScope::StoryScreen,
                focusable,
                (focusable > 0).then_some(0),
                FocusAxis::Vertical,
                actions,
            );
        }
        let mut focus = 0;
        for (element, visibility) in screen.elements.iter().zip(screen.visible.iter()) {
            if self.element_visible(visibility) {
                self.draw_story_element(element, mouse, actions, interactive, &mut focus);
            }
        }
    }

    fn draw_story_element(
        &mut self,
        element: &PlacedElement,
        mouse: Vec2,
        actions: &UiActions,
        interactive: bool,
        focus: &mut usize,
    ) {
        let rect = Rect::new(
            element.bounds.x,
            element.bounds.y,
            element.bounds.width,
            element.bounds.height,
        );
        match &element.widget {
            Widget::Hotspot {
                action,
                variable,
                expression,
            } => {
                let selected = interactive && self.focus.is(*focus);
                *focus += 1;
                if !interactive {
                    return;
                }
                let hovered = rect.contains(mouse);
                if hovered || selected {
                    draw_rectangle(
                        rect.x,
                        rect.y,
                        rect.w,
                        rect.h,
                        color_alpha(&self.theme.focus_color, 0.12),
                    );
                }
                if hovered && is_mouse_button_pressed(MouseButton::Left)
                    || selected && actions.pressed(UiAction::Activate)
                {
                    if let (Some(variable), Some(expression)) = (variable, expression) {
                        self.ui_screen_actions_set(variable, expression);
                    } else if let Some(action) = action {
                        self.story_action(*action);
                    } else {
                        self.continue_story();
                    }
                }
            }
            Widget::Button { text, action } => {
                let selected = interactive && self.focus.is(*focus);
                *focus += 1;
                if super::ui_common::button(
                    rect,
                    text,
                    mouse,
                    interactive,
                    selected,
                    selected && actions.pressed(UiAction::Activate),
                    &self.theme,
                ) {
                    self.story_action(*action);
                }
            }
            Widget::Text { text } => {
                draw_rectangle(
                    rect.x,
                    rect.y,
                    rect.w,
                    rect.h,
                    Color::new(0.0, 0.0, 0.0, 0.0),
                );
                super::ui_common::draw_fitted_centered(
                    rect,
                    text,
                    22,
                    color(&self.theme.text_color),
                );
            }
            Widget::Image { path } => {
                if let Some(texture) = self.assets.textures.get(path) {
                    draw_texture_ex(
                        texture,
                        rect.x,
                        rect.y,
                        WHITE,
                        DrawTextureParams {
                            dest_size: Some(vec2(rect.w, rect.h)),
                            ..Default::default()
                        },
                    );
                }
            }
            _ => {}
        }
    }

    fn element_visible(&self, visibility: &super::ui_layouts::Visibility) -> bool {
        match visibility {
            super::ui_layouts::Visibility::Always => true,
            super::ui_layouts::Visibility::Never => false,
            super::ui_layouts::Visibility::When(expr) => self
                .runtime
                .as_ref()
                .is_none_or(|runtime| {
                    runtime
                        .evaluate_condition(expr)
                        .is_ok_and(|value| value == Value::Boolean(true))
                }),
        }
    }

    fn story_action(&mut self, action: Action) {
        if action == Action::Close {
            if matches!(
                self.runtime.as_ref().and_then(renrs::Runtime::waiting),
                Some(WaitState::Screen { .. })
            ) {
                self.continue_story();
            }
            return;
        }
        self.screen_command(super::ui_screen_actions::ScreenCommand::Action(action));
    }

    fn ui_screen_actions_set(&mut self, variable: &str, expression: &str) {
        self.screen_command(super::ui_screen_actions::ScreenCommand::Set(
            variable.to_owned(),
            expression.to_owned(),
        ));
    }
}
