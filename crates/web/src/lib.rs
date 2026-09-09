#![allow(clippy::missing_errors_doc)] // Exported errors are JavaScript values, not Rust API errors.

use renrs_runtime::{Program, Runtime, RuntimeError};
use std::cell::RefCell;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

const PARALLEL_SAMPLES: u16 = 12;

#[wasm_bindgen]
pub struct Engine {
    runtime: Runtime,
    last_stage: RefCell<Option<Arc<renrs_runtime::runtime::StageState>>>,
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
        Ok(Self {
            runtime,
            last_stage: RefCell::new(None),
        })
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
        let stage = self.runtime.shared_stage();
        let unchanged = self
            .last_stage
            .borrow()
            .as_ref()
            .is_some_and(|previous| Arc::ptr_eq(previous, &stage));
        let encoded = serde_json::to_string(&serde_json::json!({
            "stage": if unchanged { serde_json::Value::Null } else { serde_json::to_value(stage.as_ref()).map_err(js_error)? },
            "waiting": self.runtime.waiting(),
            "debug": {"label": self.runtime.current_label(), "paused": self.runtime.debug_paused()},
            "profile_revision": self.runtime.profile_revision(),
            "history_count": self.runtime.history().len(),
            "can_rollback": self.runtime.can_rollback(),
            "nvl": self.runtime.nvl_dialogue()
        }))
        .map_err(js_error)?;
        *self.last_stage.borrow_mut() = Some(stage);
        Ok(encoded)
    }

    pub fn animation_frame(&self, progress: f32) -> Result<String, JsValue> {
        match self.try_sample_parallel(progress) {
            Some(stage) => serde_json::to_string(&stage.sprites).map_err(js_error),
            None => Ok("[]".to_owned()),
        }
    }

    pub fn camera_frame(&self, progress: f32) -> Result<String, JsValue> {
        match self.try_sample_parallel(progress) {
            Some(stage) => serde_json::to_string(&stage.camera).map_err(js_error),
            None => Ok("null".to_owned()),
        }
    }

    pub fn animation_frames(&self) -> Result<String, JsValue> {
        self.parallel_sprite_frames()
    }

    pub fn camera_frames(&self) -> Result<String, JsValue> {
        self.parallel_camera_frames()
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

    pub fn screen_visible(&self, source: &str) -> Result<bool, JsValue> {
        let expression = renrs_compiler::expression::parse_expression(source, "screens.json", 1, 1)
            .map_err(js_error)?;
        match self
            .runtime
            .evaluate_condition(&expression)
            .map_err(js_error)?
        {
            renrs_syntax::syntax::Value::Boolean(value) => Ok(value),
            _ => Err(JsValue::from_str(
                "screen visibility expression must return a boolean",
            )),
        }
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
        *self.last_stage.borrow_mut() = None;
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

    pub fn sound_ended(&mut self) {
        self.runtime.complete_sound_track();
    }
}

impl Engine {
    fn try_sample_parallel(&self, progress: f32) -> Option<renrs_runtime::runtime::StageState> {
        let renrs_runtime::WaitState::Effect {
            effect:
                renrs_runtime::runtime::VisualEffect::Parallel {
                    from,
                    tracks,
                    seconds,
                },
        } = self.runtime.waiting()?
        else {
            return None;
        };
        Some(renrs_runtime::animation::sample(
            from,
            tracks,
            progress.clamp(0.0, 1.0) * seconds,
        ))
    }

    fn parallel_sprite_frames(&self) -> Result<String, JsValue> {
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
        let frames: Vec<_> = (0..=PARALLEL_SAMPLES)
            .map(|index| {
                renrs_runtime::animation::sample(
                    from,
                    tracks,
                    f32::from(index) * seconds / f32::from(PARALLEL_SAMPLES),
                )
                .sprites
            })
            .collect();
        serde_json::to_string(&frames).map_err(js_error)
    }

    fn parallel_camera_frames(&self) -> Result<String, JsValue> {
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
        let frames: Vec<_> = (0..=PARALLEL_SAMPLES)
            .map(|index| {
                renrs_runtime::animation::sample(
                    from,
                    tracks,
                    f32::from(index) * seconds / f32::from(PARALLEL_SAMPLES),
                )
                .camera
            })
            .collect();
        serde_json::to_string(&frames).map_err(js_error)
    }
}
