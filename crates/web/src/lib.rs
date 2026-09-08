#![allow(clippy::missing_errors_doc)] // Exported errors are JavaScript values, not Rust API errors.

use renrs_runtime::{Program, Runtime, RuntimeError};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Engine {
    runtime: Runtime,
}

mod saves;
#[cfg(test)]
mod tests;

fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}

#[wasm_bindgen]
impl Engine {
    #[wasm_bindgen(constructor)]
    pub fn new(program: &str, profile: &str) -> Result<Engine, JsValue> {
        let program: Program = serde_json::from_str(program).map_err(js_error)?;
        let mut runtime = Runtime::new(program).map_err(js_error)?;
        if !profile.is_empty() {
            runtime
                .set_profile(serde_json::from_str(profile).map_err(js_error)?)
                .map_err(js_error)?;
        }
        runtime.enable_tracing();
        Ok(Self { runtime })
    }

    pub fn action(&mut self, command: &str, index: usize) -> Result<String, JsValue> {
        let result = match command {
            "start" => self.runtime.advance(),
            "next" => self.runtime.continue_story(),
            "choose" => self.runtime.choose(index),
            "rollback" => self.runtime.rollback(),
            "step" => self.runtime.debug_resume(true),
            "resume" => self.runtime.debug_resume(false),
            _ => return Err(JsValue::from_str("unknown engine action")),
        };
        match result {
            Ok(_) | Err(RuntimeError::DebugPaused) => self.state(),
            Err(error) => Err(js_error(error)),
        }
    }

    pub fn state(&self) -> Result<String, JsValue> {
        serde_json::to_string(&serde_json::json!({
            "stage": self.runtime.stage(), "waiting": self.runtime.waiting(),
            "debug": {"label": self.runtime.current_label(), "paused": self.runtime.debug_paused()},
            "profile_revision": self.runtime.profile_revision(),
            "history_count": self.runtime.history().len(),
            "can_rollback": self.runtime.can_rollback()
            ,"nvl": self.runtime.nvl_dialogue()
        }))
        .map_err(js_error)
    }

    pub fn animation_frames(&self) -> Result<String, JsValue> {
        let Some(renrs_runtime::WaitState::Effect {
            effect:
                renrs_runtime::runtime::VisualEffect::Parallel {
                    from,
                    tracks,
                    seconds,
                },
        }) = self.runtime.waiting()
        else {
            return Ok("[]".to_owned());
        };
        let frames: Vec<_> = (0..=60_u16)
            .map(|index| {
                renrs_runtime::animation::sample(from, tracks, f32::from(index) * seconds / 60.0)
                    .sprites
            })
            .collect();
        serde_json::to_string(&frames).map_err(js_error)
    }

    pub fn camera_frames(&self) -> Result<String, JsValue> {
        let Some(renrs_runtime::WaitState::Effect {
            effect:
                renrs_runtime::runtime::VisualEffect::Parallel {
                    from,
                    tracks,
                    seconds,
                },
        }) = self.runtime.waiting()
        else {
            return Ok("[]".to_owned());
        };
        let frames: Vec<_> = (0..=60_u16)
            .map(|index| {
                renrs_runtime::animation::sample(from, tracks, f32::from(index) * seconds / 60.0)
                    .camera
            })
            .collect();
        serde_json::to_string(&frames).map_err(js_error)
    }

    pub fn inspect(&self) -> Result<String, JsValue> {
        serde_json::to_string(&serde_json::json!({
            "debug": self.runtime.debug_state(),
            "coverage": self.runtime.visited_instructions().collect::<Vec<_>>()
        }))
        .map_err(js_error)
    }

    pub fn screen_text(&self, text: &str) -> Result<String, JsValue> {
        use renrs_syntax::syntax::Value;
        let mut variables = self.runtime.variables().clone();
        variables.insert(
            "title".to_owned(),
            Value::String(self.runtime.shared_program().title.clone()),
        );
        variables.insert(
            "chapter".to_owned(),
            Value::String(self.runtime.current_label().unwrap_or("").to_owned()),
        );
        renrs_runtime::runtime::format_text(text, &variables).map_err(js_error)
    }

    pub fn screen_value(&self, name: &str) -> Result<String, JsValue> {
        serde_json::to_string(&self.runtime.variables().get(name)).map_err(js_error)
    }

    pub fn set_variable(&mut self, name: &str, value: &str) -> Result<String, JsValue> {
        self.runtime
            .set_screen_variable(name, serde_json::from_str(value).map_err(js_error)?)
            .map_err(js_error)?;
        self.state()
    }

    pub fn apply_expression(&mut self, name: &str, expression: &str) -> Result<String, JsValue> {
        let expression =
            renrs_compiler::expression::parse_expression(expression, "screens.json", 1, 1)
                .map_err(js_error)?;
        self.runtime
            .apply_screen_expression(name, &expression)
            .map_err(js_error)?;
        self.state()
    }

    pub fn apply_extension(
        &mut self,
        target: &str,
        name: &str,
        input: &str,
    ) -> Result<String, JsValue> {
        let input = renrs_compiler::expression::parse_expression(input, "screens.json", 1, 1)
            .map_err(js_error)?;
        self.runtime
            .apply_extension_expression(target, name, &input)
            .map_err(js_error)?;
        self.state()
    }

    #[must_use]
    pub fn translate_ui(&self, text: &str) -> String {
        let key = format!(
            "ui.{}",
            text.to_ascii_lowercase()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join("_")
        );
        renrs_runtime::TranslationId::new(key).map_or_else(
            |_| text.to_owned(),
            |id| self.runtime.localizer().translate(&id, text).to_owned(),
        )
    }

    pub fn history(&self, offset: usize, limit: usize) -> Result<String, JsValue> {
        let history = self.runtime.history();
        let start = offset.min(history.len());
        let end = start.saturating_add(limit.min(100)).min(history.len());
        serde_json::to_string(&history[start..end]).map_err(js_error)
    }

    pub fn profile(&self) -> Result<String, JsValue> {
        serde_json::to_string(self.runtime.profile()).map_err(js_error)
    }

    pub fn snapshot(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.runtime.snapshot()).map_err(js_error)
    }

    pub fn restore(&mut self, snapshot: &str) -> Result<String, JsValue> {
        let (mut next, _) = Runtime::restore_compatible(
            self.runtime.shared_program(),
            serde_json::from_str(snapshot).map_err(js_error)?,
        )
        .map_err(js_error)?;
        next.set_profile(self.runtime.profile().clone())
            .map_err(js_error)?;
        next.set_localizer(self.runtime.localizer().clone())
            .map_err(js_error)?;
        next.enable_tracing();
        self.runtime = next;
        self.state()
    }

    pub fn breakpoints(&mut self, ids: &str) -> Result<(), JsValue> {
        self.runtime
            .set_breakpoints(serde_json::from_str(ids).map_err(js_error)?)
            .map_err(js_error)
    }

    pub fn catalog(&mut self, catalog: &str) -> Result<(), JsValue> {
        self.runtime
            .insert_translation_catalog(serde_json::from_str(catalog).map_err(js_error)?)
            .map_err(js_error)
    }

    pub fn language(&mut self, language: &str) -> Result<String, JsValue> {
        self.runtime
            .set_language((!language.is_empty()).then(|| language.to_owned()))
            .map_err(js_error)?;
        self.state()
    }

    pub fn audio_events(&mut self) -> Result<String, JsValue> {
        serde_json::to_string(&self.runtime.drain_audio_events().collect::<Vec<_>>())
            .map_err(js_error)
    }

    pub fn music_ended(&mut self) {
        self.runtime.complete_music_track();
    }
}
