use super::app::App;
use super::ui_common::{ButtonState, button, color, draw_text_block, ellipsize, slot_button};
use super::ui_screen_actions::ScreenCommand;
use crate::frontend::{FocusAxis, FocusScope, UiAction, UiActions};
use macroquad::prelude::*;
use renrs::screens::{Action, ListSource, ScreenKind, Screens, Widget};
use renrs::syntax::Value;
use renrs::{ProjectSource, theme::Theme};

pub(super) fn load_screens(source: &ProjectSource) -> Result<Screens, String> {
    if !source.contains(renrs::screens::SCREENS_FILE) {
        return Ok(Screens::default());
    }
    Screens::from_slice(
        &source
            .read(renrs::screens::SCREENS_FILE)
            .map_err(|error| error.to_string())?,
    )
}

impl App {
    pub(super) fn dialogue_theme(&self) -> Theme {
        if self
            .runtime
            .as_ref()
            .is_some_and(|runtime| runtime.stage().nvl)
        {
            let mut theme = self.theme.clone();
            theme.layout.dialogue_rect = renrs::theme::ThemeRect {
                x: 80.0,
                y: 80.0,
                width: 1120.0,
                height: 580.0,
            };
            return theme;
        }
        let Some(elements) = self.screen_layouts.elements(ScreenKind::Dialogue) else {
            return self.theme.clone();
        };
        let Some(element) = elements
            .iter()
            .find(|item| matches!(item.widget, Widget::Dialogue))
        else {
            return self.theme.clone();
        };
        let mut theme = self.element_theme(element.style.as_deref());
        theme.layout.dialogue_rect = element.bounds;
        theme
    }
    #[allow(clippy::too_many_lines)]
    pub(super) fn draw_custom_screen(
        &mut self,
        kind: ScreenKind,
        mouse: Vec2,
        actions: &UiActions,
    ) -> bool {
        let Some(elements) = self.screen_layouts.elements(kind) else {
            return false;
        };
        let elements = self.prepare_viewports(kind, &elements, mouse);
        let interactive =
            kind != ScreenKind::Hud && (kind != ScreenKind::MainMenu || self.overlay.is_none());
        if kind == ScreenKind::MainMenu {
            self.draw_background(self.title_background.as_deref());
        } else if !matches!(
            kind,
            ScreenKind::Hud | ScreenKind::Dialogue | ScreenKind::Choices
        ) {
            clear_background(color(&self.theme.panel_color));
        }
        if interactive
            && !matches!(
                kind,
                ScreenKind::MainMenu | ScreenKind::Dialogue | ScreenKind::Choices
            )
            && actions.pressed(UiAction::Back)
        {
            self.screen_command(ScreenCommand::Action(Action::Close));
            return true;
        }
        let mut count = 0;
        let mut lists = std::collections::HashMap::new();
        for (index, element) in elements.iter().enumerate() {
            match &element.widget {
                Widget::Choices => count += self.story_options().len(),
                Widget::Button { .. }
                | Widget::Set { .. }
                | Widget::Slider { .. }
                | Widget::Toggle { .. }
                | Widget::Input { .. }
                | Widget::Drag { .. }
                | Widget::Drop { .. }
                | Widget::Extension { .. } => count += 1,
                Widget::DataList {
                    item_height,
                    variable,
                    ..
                } => count += self.data_rows(variable, element.bounds.height, *item_height),
                Widget::List {
                    source,
                    item_height,
                    gap,
                } => {
                    let theme = self.element_theme(element.style.as_deref());
                    let items = self.screen_list_items(
                        *source,
                        kind,
                        element.bounds.width,
                        theme.ui_font_size,
                    );
                    let rows =
                        ((element.bounds.height + gap) / (item_height + gap)).floor() as usize;
                    if !matches!(source, ListSource::History) {
                        count += rows.min(items.len());
                    }
                    lists.insert(index, (items, rows));
                }
                _ => {}
            }
        }
        if interactive {
            let scope = match kind {
                ScreenKind::MainMenu => FocusScope::MainMenu,
                ScreenKind::Save => FocusScope::SaveSlots,
                ScreenKind::Load => FocusScope::LoadSlots,
                ScreenKind::Settings => FocusScope::Settings,
                ScreenKind::Dialogue => FocusScope::Dialogue,
                ScreenKind::Choices => FocusScope::Choices,
                _ => FocusScope::History,
            };
            self.focus.update(
                scope,
                count,
                if kind == ScreenKind::Dialogue {
                    None
                } else {
                    Some(0)
                },
                FocusAxis::Vertical,
                actions,
            );
        }
        let mut focus = 0;
        let mut command = None;
        for (index, element) in elements.iter().enumerate() {
            let _clip = super::ui_composition::ClipGuard::new(element, self.canvas_target.as_ref());
            let mouse = if super::ui_composition::hit_clip(element, mouse) {
                mouse
            } else {
                vec2(-100_000.0, -100_000.0)
            };
            let bounds = element.bounds;
            let rect = Rect::new(bounds.x, bounds.y, bounds.width, bounds.height);
            let theme = self.element_theme(element.style.as_deref());
            if let Some(background) = element
                .style
                .as_ref()
                .and_then(|name| self.screens.styles.get(name))
                .and_then(|style| style.background_color.as_deref())
            {
                draw_rectangle(rect.x, rect.y, rect.w, rect.h, color(background));
            }
            match &element.widget {
                Widget::Drag { .. }
                | Widget::Drop { .. }
                | Widget::DataList { .. }
                | Widget::Extension { .. } => {
                    if let Some(next) = self.draw_data_control(
                        kind,
                        element.key,
                        element,
                        mouse,
                        actions,
                        interactive,
                        &mut focus,
                    ) {
                        command = Some(next);
                    }
                }
                Widget::Dialogue => {
                    let previous = self.theme.clone();
                    self.theme = theme;
                    self.theme.layout.dialogue_rect = bounds;
                    let stage = self
                        .runtime
                        .as_ref()
                        .map(renrs::Runtime::shared_stage)
                        .unwrap_or_default();
                    if let Some(dialogue) = &stage.dialogue {
                        let interactive = matches!(
                            self.runtime.as_ref().and_then(renrs::Runtime::waiting),
                            Some(renrs::WaitState::Dialogue)
                        );
                        self.draw_dialogue(dialogue, mouse, actions, interactive);
                    }
                    self.theme = previous;
                }
                Widget::Choices => {
                    let options = self.story_options();
                    let rows = (rect.h / 64.0).floor().max(1.0) as usize;
                    let selected = self.focus.selected().unwrap_or(focus).saturating_sub(focus);
                    let start = selected
                        .saturating_sub(rows - 1)
                        .min(options.len().saturating_sub(rows));
                    for (index, text) in options.iter().enumerate().skip(start).take(rows) {
                        let row =
                            Rect::new(rect.x, rect.y + (index - start) as f32 * 64.0, rect.w, 56.0);
                        if button(
                            row,
                            text,
                            mouse,
                            interactive,
                            self.focus.is(focus + index),
                            self.focus.is(focus + index) && actions.pressed(UiAction::Activate),
                            &theme,
                        ) {
                            command = Some(ScreenCommand::Choose(index));
                        }
                    }
                    focus += options.len();
                }
                Widget::Toggle { text, setting } => {
                    self.screen_toggle(*setting, text, rect, mouse, actions, interactive, focus);
                    focus += 1;
                }
                Widget::Input {
                    variable,
                    max_length,
                    text,
                } => {
                    if let Some(runtime) = &mut self.runtime {
                        let mut value = match runtime.variables().get(variable) {
                            Some(Value::String(value)) => value.clone(),
                            _ => String::new(),
                        };
                        let before = value.clone();
                        super::ui_common::draw_fitted_centered(
                            Rect::new(rect.x, rect.y, rect.w * 0.3, rect.h),
                            text,
                            theme.ui_font_size,
                            color(&theme.text_color),
                        );
                        super::ui_save_tools::text_field(
                            Rect::new(rect.x + rect.w * 0.3, rect.y, rect.w * 0.7, rect.h),
                            &mut value,
                            interactive && self.focus.is(focus),
                            &theme,
                        );
                        value = value.chars().take(*max_length).collect();
                        if value != before
                            && let Err(error) =
                                runtime.set_screen_variable(variable, Value::String(value))
                        {
                            self.notice = Some((error.to_string(), 5.0));
                        }
                    }
                    focus += 1;
                }
                Widget::Text { text } => draw_text_block(
                    &self.screen_text(text),
                    rect,
                    theme.ui_font_size,
                    color(&theme.text_color),
                ),
                Widget::Image { path } => {
                    if let Some(texture) = self.assets.textures.get(path) {
                        let scale = (rect.w / texture.width()).min(rect.h / texture.height());
                        let size = vec2(texture.width(), texture.height()) * scale;
                        draw_texture_ex(
                            texture,
                            rect.x + (rect.w - size.x) / 2.0,
                            rect.y + (rect.h - size.y) / 2.0,
                            WHITE,
                            DrawTextureParams {
                                dest_size: Some(size),
                                ..Default::default()
                            },
                        );
                    }
                }
                Widget::Button { text, action } => {
                    let enabled = interactive && self.screen_action_enabled(*action);
                    let text =
                        ellipsize(&self.screen_text(text), rect.w - 24.0, theme.ui_font_size);
                    if button(
                        rect,
                        &text,
                        mouse,
                        enabled,
                        interactive && self.focus.is(focus),
                        interactive && self.focus.is(focus) && actions.pressed(UiAction::Activate),
                        &theme,
                    ) {
                        command = Some(ScreenCommand::Action(*action));
                    }
                    focus += 1;
                }
                Widget::Set {
                    text,
                    variable,
                    expression,
                } => {
                    let enabled = interactive
                        && self.runtime.as_ref().is_some_and(|runtime| {
                            matches!(
                                runtime.waiting(),
                                Some(renrs::WaitState::Dialogue | renrs::WaitState::Choice { .. })
                            )
                        });
                    if button(
                        rect,
                        &self.screen_text(text),
                        mouse,
                        enabled,
                        self.focus.is(focus),
                        self.focus.is(focus) && actions.pressed(UiAction::Activate),
                        &theme,
                    ) {
                        command = Some(ScreenCommand::Set(variable.clone(), expression.clone()));
                    }
                    focus += 1;
                }
                Widget::Slider { text, setting } => {
                    if interactive {
                        self.screen_slider(*setting, text, rect, mouse, focus, actions, &theme);
                    }
                    focus += 1;
                }
                Widget::List {
                    source,
                    item_height,
                    gap,
                } => {
                    let Some((items, rows)) = lists.get(&index) else {
                        continue;
                    };
                    let key = format!("{kind:?}:{}", element.key);
                    let offset = self.screen_scroll.entry(key).or_insert_with(|| {
                        if matches!(source, ListSource::History) {
                            items.len().saturating_sub(*rows)
                        } else {
                            0
                        }
                    });
                    if rect.contains(mouse) {
                        *offset = offset.saturating_add_signed(-mouse_wheel().1.round() as isize);
                    }
                    if interactive {
                        let rows_signed = isize::try_from(*rows).unwrap_or(isize::MAX);
                        let delta = rows_signed
                            * (isize::from(actions.pressed(UiAction::PageDown))
                                - isize::from(actions.pressed(UiAction::PageUp)));
                        *offset = offset.saturating_add_signed(delta);
                        if actions.pressed(UiAction::Home) {
                            *offset = 0;
                        }
                        if actions.pressed(UiAction::End) {
                            *offset = items.len();
                        }
                    }
                    *offset = (*offset).min(items.len().saturating_sub(*rows));
                    for (row, item) in items.iter().skip(*offset).take(*rows).enumerate() {
                        let row_rect = Rect::new(
                            rect.x,
                            rect.y + row as f32 * (item_height + gap),
                            rect.w,
                            *item_height,
                        );
                        let state = ButtonState::new(
                            interactive && item.command.is_some(),
                            interactive && self.focus.is(focus),
                            interactive
                                && self.focus.is(focus)
                                && actions.pressed(UiAction::Activate),
                        );
                        let clicked = match source {
                            ListSource::History => {
                                draw_text_block(
                                    &item.text,
                                    row_rect,
                                    theme.ui_font_size,
                                    color(&theme.text_color),
                                );
                                false
                            }
                            ListSource::Languages => button(
                                row_rect,
                                &item.text,
                                mouse,
                                interactive,
                                interactive && self.focus.is(focus),
                                interactive
                                    && self.focus.is(focus)
                                    && actions.pressed(UiAction::Activate),
                                &theme,
                            ),
                            _ => slot_button(
                                row_rect,
                                row + *offset + 1,
                                item.slot.as_ref(),
                                mouse,
                                state,
                                &theme,
                            ),
                        };
                        if clicked {
                            command.clone_from(&item.command);
                        }
                        if !matches!(source, ListSource::History) {
                            focus += 1;
                        }
                    }
                    if items.is_empty() {
                        draw_text_block(
                            &super::ui_text::tr("Empty"),
                            rect,
                            theme.ui_font_size,
                            color(&theme.muted_text_color),
                        );
                    }
                }
                Widget::Row { .. }
                | Widget::Column { .. }
                | Widget::Stack { .. }
                | Widget::Viewport { .. }
                | Widget::Hotspot { .. } => {}
            }
        }
        if let Some(command) = command {
            self.screen_command(command);
        }
        true
    }

    fn story_options(&self) -> Vec<String> {
        match self.runtime.as_ref().and_then(renrs::Runtime::waiting) {
            Some(renrs::WaitState::Choice { options }) => options.clone(),
            _ => Vec::new(),
        }
    }

    pub(super) fn element_theme(&self, style: Option<&str>) -> Theme {
        let mut theme = self.theme.clone();
        if let Some(style) = style.and_then(|name| self.screens.styles.get(name)) {
            if let Some(size) = style.font_size {
                theme.ui_font_size = size;
            }
            if let Some(color) = &style.text_color {
                theme.text_color.clone_from(color);
            }
            if let Some(color) = &style.background_color {
                theme.surface_color.clone_from(color);
            }
        }
        theme
    }

    pub(super) fn screen_text(&self, text: &str) -> String {
        let mut variables = self
            .runtime
            .as_ref()
            .map_or_else(Default::default, |runtime| runtime.variables().clone());
        variables.insert(
            "title".to_owned(),
            Value::String(self.program.title.clone()),
        );
        variables.insert(
            "chapter".to_owned(),
            Value::String(
                self.runtime
                    .as_ref()
                    .and_then(renrs::Runtime::current_label)
                    .unwrap_or("")
                    .to_owned(),
            ),
        );
        renrs::runtime::format_text(text, &variables).unwrap_or_else(|_| text.to_owned())
    }
}
