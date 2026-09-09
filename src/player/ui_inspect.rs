use super::app::App;
use super::text::draw_text;
use super::ui_common::{color, color_alpha};
use macroquad::prelude::*;
use renrs::runtime::DebugState;

impl App {
    pub(super) fn toggle_inspect(&mut self) {
        self.inspect = !self.inspect;
        self.redraw = true;
    }

    pub(super) fn draw_inspect(&self) {
        if !self.inspect {
            return;
        }
        draw_rectangle(16.0, 72.0, 420.0, 560.0, color_alpha("#111827", 0.92));
        draw_rectangle_lines(
            16.0,
            72.0,
            420.0,
            560.0,
            2.0,
            color(&self.theme.focus_color),
        );
        draw_text(
            "Inspect  F3",
            28.0,
            96.0,
            18.0,
            color(&self.theme.focus_color),
        );
        let Some(runtime) = &self.runtime else {
            draw_text(
                "No runtime",
                28.0,
                128.0,
                16.0,
                color(&self.theme.text_color),
            );
            return;
        };
        let state: DebugState = runtime.debug_state();
        let waiting = format!("{:?}", state.waiting);
        let lines = [
            format!("label  {}", state.label.as_deref().unwrap_or("-")),
            format!(
                "inst   {}",
                state.instruction.as_ref().map_or("-", |id| id.as_str())
            ),
            format!(
                "line   {}",
                state.location.as_ref().map_or(0, |span| span.line)
            ),
            format!("wait   {waiting}"),
            format!(
                "window {}",
                if runtime.stage().window {
                    "show"
                } else {
                    "hide"
                }
            ),
            format!("sprites {}", runtime.stage().sprites.len()),
        ];
        for (index, line) in lines.iter().enumerate() {
            draw_text(
                line,
                28.0,
                128.0 + index as f32 * 22.0,
                16.0,
                color(&self.theme.text_color),
            );
        }
        draw_text(
            "variables",
            28.0,
            280.0,
            16.0,
            color(&self.theme.focus_color),
        );
        for (index, (name, value)) in state.variables.iter().take(12).enumerate() {
            let text = format!("{name} = {value:?}");
            draw_text(
                text.chars().take(42).collect::<String>(),
                28.0,
                304.0 + index as f32 * 18.0,
                14.0,
                color(&self.theme.text_color),
            );
        }
        draw_text("stage", 28.0, 530.0, 16.0, color(&self.theme.focus_color));
        for (index, sprite) in runtime.stage().sprites.iter().take(4).enumerate() {
            draw_text(
                format!("{} {:?}", sprite.alias, sprite.position),
                28.0,
                552.0 + index as f32 * 16.0,
                14.0,
                color(&self.theme.text_color),
            );
        }
    }
}
