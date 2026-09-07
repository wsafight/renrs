use super::text::{draw_text, measure_width};
use macroquad::prelude::*;
use renrs::save::SaveSlot;
use renrs::theme::Theme as ProjectTheme;
use renrs::validator::parse_hex_color;

use crate::frontend::{UiAction, UiActions};
use crate::text_layout::wrap_plain;

#[derive(Debug, Clone, Copy)]
pub(super) struct ButtonState {
    enabled: bool,
    focused: bool,
    keyboard_activate: bool,
}

impl ButtonState {
    pub(super) const fn new(enabled: bool, focused: bool, keyboard_activate: bool) -> Self {
        Self {
            enabled,
            focused,
            keyboard_activate,
        }
    }
}

pub(super) fn button(
    rect: Rect,
    label: &str,
    mouse: Vec2,
    enabled: bool,
    focused: bool,
    keyboard_activate: bool,
    theme: &ProjectTheme,
) -> bool {
    let translated = super::ui_text::tr(label);
    let label = translated.as_str();
    super::speech::label(label, focused);
    let hovered = enabled && rect.contains(mouse);
    let background = if !enabled {
        color_alpha(&theme.surface_color, 0.55)
    } else if hovered {
        color(&theme.accent_color)
    } else {
        color(&theme.surface_color)
    };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, background);
    draw_focus_outline(rect, focused, theme);
    draw_fitted_centered(
        rect,
        label,
        theme.ui_font_size,
        if enabled {
            color(&theme.text_color)
        } else {
            color_alpha(&theme.muted_text_color, 0.72)
        },
    );
    enabled && (keyboard_activate || hovered && is_mouse_button_pressed(MouseButton::Left))
}

pub(super) fn compact_button(
    rect: Rect,
    label: &str,
    mouse: Vec2,
    focused: bool,
    keyboard_activate: bool,
    theme: &ProjectTheme,
) -> bool {
    let translated = super::ui_text::tr(label);
    let label = translated.as_str();
    let hovered = rect.contains(mouse);
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        if hovered {
            color(&theme.accent_color)
        } else {
            color(&theme.surface_color)
        },
    );
    draw_focus_outline(rect, focused, theme);
    draw_fitted_centered(
        rect,
        label,
        theme.ui_font_size.saturating_sub(2),
        color(&theme.text_color),
    );
    keyboard_activate || hovered && is_mouse_button_pressed(MouseButton::Left)
}

pub(super) fn toolbar_button(
    rect: Rect,
    label: &str,
    mouse: Vec2,
    active: bool,
    state: ButtonState,
    theme: &ProjectTheme,
) -> bool {
    let translated = super::ui_text::tr(label);
    let label = translated.as_str();
    let ButtonState {
        enabled,
        focused,
        keyboard_activate,
    } = state;
    super::speech::label(label, focused);
    let hovered = enabled && rect.contains(mouse);
    let background = if !enabled {
        color_alpha(&theme.surface_color, 0.55)
    } else if active || hovered {
        color(&theme.accent_color)
    } else {
        color(&theme.surface_color)
    };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, background);
    draw_focus_outline(rect, focused, theme);
    draw_fitted_centered(
        rect,
        label,
        theme.ui_font_size.saturating_sub(4),
        if enabled {
            color(&theme.text_color)
        } else {
            color_alpha(&theme.muted_text_color, 0.65)
        },
    );
    enabled && (keyboard_activate || hovered && is_mouse_button_pressed(MouseButton::Left))
}

pub(super) fn choice_button(
    rect: Rect,
    label: &str,
    mouse: Vec2,
    selected: bool,
    theme: &ProjectTheme,
) -> bool {
    let hovered = rect.contains(mouse);
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        if hovered || selected {
            color_alpha(&theme.accent_color, 0.97)
        } else {
            color_alpha(&theme.panel_color, 0.96)
        },
    );
    draw_focus_outline(rect, selected, theme);
    draw_fitted_centered(rect, label, theme.ui_font_size, color(&theme.text_color));
    hovered && is_mouse_button_pressed(MouseButton::Left)
}

pub(super) fn slot_button(
    rect: Rect,
    index: usize,
    slot: Option<&SaveSlot>,
    mouse: Vec2,
    state: ButtonState,
    theme: &ProjectTheme,
) -> bool {
    let ButtonState {
        enabled,
        focused,
        keyboard_activate,
    } = state;
    let hovered = enabled && rect.contains(mouse);
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        if hovered {
            color(&theme.accent_color)
        } else {
            color(&theme.surface_color)
        },
    );
    draw_focus_outline(rect, focused, theme);
    draw_text(
        format!("Slot {index}"),
        rect.x + 24.0,
        rect.y + f32::from(theme.ui_font_size) + 7.0,
        f32::from(theme.ui_font_size),
        color(&theme.focus_color),
    );
    let description = slot.map_or_else(
        || super::ui_text::tr("Empty"),
        |slot| {
            if slot.corrupt {
                "Corrupt save - overwrite this slot to repair it".to_owned()
            } else {
                let hours = slot.play_time_seconds / 3600;
                let minutes = slot.play_time_seconds % 3600 / 60;
                let chapter = slot.chapter.as_deref().unwrap_or("unknown chapter");
                let title = if slot.note.is_empty() {
                    &slot.title
                } else {
                    &slot.note
                };
                format!("{chapter}  {hours:02}:{minutes:02}  {title}")
            }
        },
    );
    let font_size = theme
        .ui_font_size
        .saturating_sub(4)
        .min((rect.h * 0.3) as u16);
    let short = ellipsize(&description, (rect.w - 48.0).max(10.0), font_size);
    draw_text(
        &short,
        rect.x + 24.0,
        rect.y + rect.h - 10.0,
        f32::from(font_size),
        color(&theme.muted_text_color),
    );
    enabled && (keyboard_activate || hovered && is_mouse_button_pressed(MouseButton::Left))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn slider(
    rect: Rect,
    label: &str,
    mut value: f32,
    minimum: f32,
    maximum: f32,
    mouse: Vec2,
    focused: bool,
    keyboard_adjustment: i8,
    keyboard_step: f32,
    theme: &ProjectTheme,
) -> f32 {
    draw_text(
        super::ui_text::tr(label),
        rect.x,
        rect.y - 14.0,
        f32::from(theme.ui_font_size),
        color(&theme.text_color),
    );
    let track = Rect::new(rect.x, rect.y + 12.0, rect.w, 8.0);
    draw_rectangle(
        track.x,
        track.y,
        track.w,
        track.h,
        color(&theme.surface_color),
    );
    if is_mouse_button_down(MouseButton::Left)
        && Rect::new(rect.x - 10.0, rect.y - 4.0, rect.w + 20.0, 44.0).contains(mouse)
    {
        let ratio = ((mouse.x - track.x) / track.w).clamp(0.0, 1.0);
        value = minimum + ratio * (maximum - minimum);
    }
    if focused && keyboard_adjustment != 0 {
        value = (value + f32::from(keyboard_adjustment) * keyboard_step).clamp(minimum, maximum);
    }
    let ratio = (value - minimum) / (maximum - minimum);
    draw_rectangle(
        track.x,
        track.y,
        track.w * ratio,
        track.h,
        color(&theme.accent_color),
    );
    draw_circle(
        track.x + track.w * ratio,
        track.y + 4.0,
        11.0,
        color(if focused {
            &theme.focus_color
        } else {
            &theme.text_color
        }),
    );
    draw_focus_outline(
        Rect::new(rect.x - 12.0, rect.y - 30.0, rect.w + 100.0, 68.0),
        focused,
        theme,
    );
    let display = if maximum <= 1.0 {
        format!("{}%", (value * 100.0).round())
    } else {
        format!("{}", value.round())
    };
    draw_text(
        &display,
        rect.x + rect.w + 24.0,
        rect.y + 22.0,
        f32::from(theme.ui_font_size.saturating_sub(2)),
        color(&theme.muted_text_color),
    );
    value
}

pub(super) fn keyboard_adjustment(actions: &UiActions, focused: bool) -> i8 {
    if !focused {
        0
    } else if actions.pressed(UiAction::Left) {
        -1
    } else {
        i8::from(actions.pressed(UiAction::Right))
    }
}

pub(super) fn draw_focus_outline(rect: Rect, focused: bool, theme: &ProjectTheme) {
    if focused {
        draw_rectangle_lines(
            rect.x - 3.0,
            rect.y - 3.0,
            rect.w + 6.0,
            rect.h + 6.0,
            3.0,
            color(&theme.focus_color),
        );
    }
}

pub(super) fn wrap_lines(text: &str, maximum_width: f32, font_size: u16) -> Vec<String> {
    wrap_plain(text, maximum_width, |value| measure_width(value, font_size))
}

pub(super) fn draw_text_block(text: &str, rect: Rect, mut size: u16, tint: Color) {
    let mut lines = wrap_lines(text, rect.w, size);
    while size > 12 && lines.len() as f32 * f32::from(size + 4) > rect.h {
        size -= 1;
        lines = wrap_lines(text, rect.w, size);
    }
    let rows = (rect.h / f32::from(size + 4)).floor().max(1.0) as usize;
    for (index, line) in lines.iter().take(rows).enumerate() {
        let text = if index + 1 == rows && lines.len() > rows {
            ellipsize(&format!("{line} ..."), rect.w, size)
        } else {
            line.clone()
        };
        draw_text(
            text,
            rect.x,
            rect.y + f32::from(size) + index as f32 * f32::from(size + 4),
            f32::from(size),
            tint,
        );
    }
}

pub(super) fn ellipsize(text: &str, width: f32, size: u16) -> String {
    if measure_width(text, size) <= width {
        return text.to_owned();
    }
    let mut result = text.to_owned();
    while !result.is_empty() && measure_width(format!("{result}..."), size) > width {
        result.pop();
    }
    result.push_str("...");
    result
}

pub(super) fn draw_centered(
    text: &str,
    center_x: f32,
    baseline_y: f32,
    font_size: u16,
    tint: Color,
) {
    let width = measure_width(text, font_size);
    draw_text(
        text,
        center_x - width / 2.0,
        baseline_y,
        f32::from(font_size),
        tint,
    );
}

pub(super) fn draw_fitted_centered(rect: Rect, text: &str, mut size: u16, tint: Color) {
    let width = (rect.w - 24.0).max(1.0);
    let height = (rect.h - 8.0).max(1.0);
    let mut lines = wrap_lines(text, width, size);
    while size > 12 && lines.len() as f32 * f32::from(size + 4) > height {
        size -= 1;
        lines = wrap_lines(text, width, size);
    }
    let rows = (height / f32::from(size + 4)).floor().max(1.0) as usize;
    let total = lines.len().min(rows) as f32 * f32::from(size + 4);
    for (index, line) in lines.iter().take(rows).enumerate() {
        let text = if index + 1 == rows && lines.len() > rows {
            ellipsize(&format!("{line} ..."), width, size)
        } else {
            ellipsize(line, width, size)
        };
        draw_centered(
            &text,
            rect.x + rect.w / 2.0,
            rect.y + (rect.h - total) / 2.0 + f32::from(size) + index as f32 * f32::from(size + 4),
            size,
            tint,
        );
    }
}

pub(super) fn color(hex: &str) -> Color {
    let [red, green, blue, alpha] = parse_hex_color(hex).unwrap_or([255, 0, 255, 255]);
    Color::from_rgba(red, green, blue, alpha)
}

pub(super) fn color_alpha(hex: &str, alpha: f32) -> Color {
    let mut output = color(hex);
    output.a = alpha;
    output
}
