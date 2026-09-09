use crate::{
    Runtime,
    runtime::DialogueState,
    text::{TextRun, TextStyle},
};

impl Runtime {
    /// Builds the active NVL page, keeping its source history and rollback state.
    #[must_use]
    pub fn nvl_dialogue(&self) -> Option<DialogueState> {
        if !self.stage().nvl {
            return None;
        }
        let mut dialogue = self.stage().dialogue.clone()?;
        dialogue.runs.clear();
        dialogue.text.clear();
        dialogue.speaker_name = None;
        for entry in &self.history()[self.stage().nvl_start.min(self.history().len())..] {
            if !dialogue.runs.is_empty() {
                dialogue.runs.push(TextRun {
                    text: "\n\n".to_owned(),
                    style: TextStyle::default(),
                    cue: None,
                });
            }
            if let Some(name) = &entry.speaker_name {
                dialogue.runs.push(TextRun {
                    text: format!("{name}: "),
                    style: TextStyle {
                        bold: true,
                        color: Some(entry.speaker_color.clone()),
                        ..TextStyle::default()
                    },
                    cue: None,
                });
            }
            if entry.runs.is_empty() {
                dialogue.runs.push(TextRun {
                    text: entry.text.clone(),
                    style: TextStyle::default(),
                    cue: None,
                });
            } else {
                dialogue.runs.extend(entry.runs.clone());
            }
        }
        dialogue.text = dialogue.runs.iter().map(|run| run.text.as_str()).collect();
        Some(dialogue)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Runtime, compile, parse_script};
    #[test]
    fn nvl_clear_and_rollback_restore_page_boundaries() {
        let program = compile(&parse_script("label start:\n    nvl on\n    \"One\"\n    \"Two\"\n    nvl clear\n    \"Three\"\n", "nvl.rns").unwrap()).unwrap();
        let mut runtime = Runtime::new(program.clone()).unwrap();
        runtime.advance().unwrap();
        runtime.continue_story().unwrap();
        assert_eq!(runtime.nvl_dialogue().unwrap().text, "One\n\nTwo");
        runtime.continue_story().unwrap();
        assert_eq!(runtime.nvl_dialogue().unwrap().text, "Three");
        let mut restored = Runtime::restore(program, runtime.snapshot()).unwrap();
        restored.rollback().unwrap();
        assert_eq!(restored.nvl_dialogue().unwrap().text, "One\n\nTwo");
    }
}
