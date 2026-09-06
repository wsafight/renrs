use super::app::App;
use super::ui_common::button;
use super::ui_screen_actions::ScreenCommand;
use crate::frontend::{UiAction, UiActions};
use macroquad::prelude::*;
use renrs::screens::{PlacedElement, ScreenKind, Widget};
use renrs::syntax::Value;

pub(super) struct ClipGuard(Option<Camera2D>);

impl ClipGuard {
    pub(super) fn new(element: &PlacedElement, target: Option<&RenderTarget>) -> Self {
        if element.viewports.is_empty() {
            return Self(None);
        }
        let clip = clip(element).unwrap_or(Rect::new(0.0, 0.0, 1.0, 1.0));
        let mut camera = Camera2D::from_display_rect(clip);
        camera.render_target = target.cloned();
        camera.viewport = Some((
            clip.x as i32,
            (720.0 - clip.y - clip.h) as i32,
            clip.w as i32,
            clip.h as i32,
        ));
        push_camera_state();
        set_camera(&camera);
        let mut parent = Camera2D::from_display_rect(Rect::new(0.0, 0.0, 1280.0, 720.0));
        parent.render_target = target.cloned();
        Self(Some(parent))
    }
}
impl Drop for ClipGuard {
    fn drop(&mut self) {
        if let Some(parent) = &self.0 {
            pop_camera_state();
            // Macroquad's camera stack does not preserve the viewport.
            set_camera(parent);
        }
    }
}

fn clip(element: &PlacedElement) -> Option<Rect> {
    let mut clip = Rect::new(0.0, 0.0, 1280.0, 720.0);
    for viewport in &element.viewports {
        let bounds = viewport.bounds;
        clip = clip.intersect(Rect::new(bounds.x, bounds.y, bounds.width, bounds.height))?;
    }
    Some(clip)
}

pub(super) fn hit_clip(element: &PlacedElement, mouse: Vec2) -> bool {
    clip(element).is_some_and(|clip| clip.contains(mouse))
}

impl App {
    pub(super) fn prepare_viewports(
        &mut self,
        kind: ScreenKind,
        elements: &[PlacedElement],
        mouse: Vec2,
    ) -> Vec<PlacedElement> {
        let mut hovered = None;
        for element in elements {
            let mut shift = 0.0;
            let mut visible = Rect::new(0.0, 0.0, 1280.0, 720.0);
            for (depth, frame) in element.viewports.iter().enumerate() {
                let rect = Rect::new(
                    frame.bounds.x,
                    frame.bounds.y - shift,
                    frame.bounds.width,
                    frame.bounds.height,
                );
                let Some(intersection) = visible.intersect(rect) else {
                    break;
                };
                visible = intersection;
                let key = format!("{kind:?}:viewport:{}", frame.id);
                if visible.contains(mouse)
                    && hovered
                        .as_ref()
                        .is_none_or(|(previous, _, _)| depth >= *previous)
                {
                    hovered = Some((
                        depth,
                        key.clone(),
                        frame.content_height - frame.bounds.height,
                    ));
                }
                shift += *self.screen_scroll.get(&key).unwrap_or(&0) as f32;
            }
        }
        if let Some((_, key, maximum)) = hovered {
            let offset = self.screen_scroll.entry(key).or_default();
            *offset = offset
                .saturating_add_signed((-mouse_wheel().1 * 36.0) as isize)
                .min(maximum as usize);
        }
        elements
            .iter()
            .filter_map(|element| {
                let mut element = element.clone();
                let mut shift = 0.0;
                for frame in &mut element.viewports {
                    frame.bounds.y -= shift;
                    shift += *self
                        .screen_scroll
                        .get(&format!("{kind:?}:viewport:{}", frame.id))
                        .unwrap_or(&0) as f32;
                }
                element.bounds.y -= shift;
                let bounds = element.bounds;
                clip(&element)?.intersect(Rect::new(
                    bounds.x,
                    bounds.y,
                    bounds.width,
                    bounds.height,
                ))?;
                Some(element)
            })
            .collect()
    }

    pub(super) fn data_rows(&self, variable: &str, height: f32, item_height: f32) -> usize {
        let count = match self
            .runtime
            .as_ref()
            .and_then(|runtime| runtime.variables().get(variable))
        {
            Some(Value::List(values)) => values.len(),
            _ => 0,
        };
        count.min((height / item_height).floor() as usize)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_data_control(
        &mut self,
        kind: ScreenKind,
        index: usize,
        element: &PlacedElement,
        mouse: Vec2,
        actions: &UiActions,
        interactive: bool,
        focus: &mut usize,
    ) -> Option<ScreenCommand> {
        let bounds = element.bounds;
        let rect = Rect::new(bounds.x, bounds.y, bounds.width, bounds.height);
        let theme = self.element_theme(element.style.as_deref());
        let enabled = interactive
            && self.runtime.as_ref().is_some_and(|runtime| {
                matches!(
                    runtime.waiting(),
                    Some(renrs::WaitState::Dialogue | renrs::WaitState::Choice { .. })
                )
            });
        let activate = self.focus.is(*focus) && actions.pressed(UiAction::Activate);
        match &element.widget {
            Widget::Extension {
                text,
                name,
                variable,
                input,
            } => {
                let clicked = button(
                    rect,
                    &self.screen_text(text),
                    mouse,
                    enabled,
                    self.focus.is(*focus),
                    activate,
                    &theme,
                );
                *focus += 1;
                if clicked {
                    return Some(ScreenCommand::Extension(
                        variable.clone(),
                        name.clone(),
                        input.clone(),
                    ));
                }
            }
            Widget::Drag { text, expression } => {
                let pressed =
                    enabled && rect.contains(mouse) && is_mouse_button_pressed(MouseButton::Left);
                if button(
                    rect,
                    &self.screen_text(text),
                    mouse,
                    enabled,
                    self.focus.is(*focus),
                    activate,
                    &theme,
                ) || pressed
                {
                    self.drag_expression = Some(expression.clone());
                }
                *focus += 1;
            }
            Widget::Drop { text, variable } => {
                let ready = enabled && self.drag_expression.is_some();
                let released =
                    ready && rect.contains(mouse) && is_mouse_button_released(MouseButton::Left);
                let clicked = button(
                    rect,
                    &self.screen_text(text),
                    mouse,
                    ready,
                    self.focus.is(*focus),
                    activate,
                    &theme,
                );
                *focus += 1;
                if (released || clicked)
                    && let Some(expression) = self.drag_expression.take()
                {
                    return Some(ScreenCommand::Set(variable.clone(), expression));
                }
            }
            Widget::DataList {
                variable,
                selected,
                label,
                item_height,
            } => {
                let values = match self
                    .runtime
                    .as_ref()
                    .and_then(|runtime| runtime.variables().get(variable))
                {
                    Some(Value::List(values)) => values.clone(),
                    _ => return None,
                };
                let rows = self.data_rows(variable, rect.h, *item_height);
                let offset = self
                    .screen_scroll
                    .entry(format!("{kind:?}:data:{index}"))
                    .or_default();
                if rect.contains(mouse) {
                    *offset = offset.saturating_add_signed(-mouse_wheel().1.round() as isize);
                }
                *offset = (*offset).min(values.len().saturating_sub(rows));
                let offset = *offset;
                let mut result = None;
                for (row, value) in values.iter().enumerate().skip(offset).take(rows) {
                    let text = if let (Some(label), Value::Record(fields)) = (label, value) {
                        fields.get(label).unwrap_or(value)
                    } else {
                        value
                    };
                    let text = match text {
                        Value::String(text) => text.clone(),
                        value => serde_json::to_string(value).unwrap_or_default(),
                    };
                    let row_rect = Rect::new(
                        rect.x,
                        rect.y + (row - offset) as f32 * item_height,
                        rect.w,
                        item_height - 4.0,
                    );
                    if button(
                        row_rect,
                        &text,
                        mouse,
                        enabled,
                        self.focus.is(*focus),
                        self.focus.is(*focus) && actions.pressed(UiAction::Activate),
                        &theme,
                    ) {
                        result = Some(ScreenCommand::Set(
                            selected.clone(),
                            format!("get({variable}, {row})"),
                        ));
                    }
                    *focus += 1;
                }
                return result;
            }
            _ => {}
        }
        None
    }
}
