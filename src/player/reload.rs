use super::app::{App, first_background};
use super::reload_worker::ReloadBundle;
use super::text::Face;

impl App {
    pub(super) fn reload_project(&mut self, mut bundle: ReloadBundle) -> Result<(), String> {
        if bundle.program.project_id != self.program.project_id {
            return Err("changing config id requires restarting the player".to_owned());
        }
        bundle
            .localizer
            .set_language(self.localizer.language().map(ToOwned::to_owned))
            .map_err(|error| error.to_string())?;
        let restored = self
            .runtime
            .as_ref()
            .map(|runtime| {
                let mut restored = runtime
                    .reloaded(bundle.program.clone())
                    .map_err(|error| error.to_string())?;
                restored
                    .set_localizer(bundle.localizer.clone())
                    .map_err(|error| error.to_string())?;
                Ok::<_, String>(restored)
            })
            .transpose()?;
        let font = bundle.font.take().map(Face::family).transpose()?;
        self.runtime = restored;
        self.redraw = true;
        self.image_hints = self
            .runtime
            .as_ref()
            .map_or_else(Vec::new, |runtime| runtime.upcoming_images(4));
        self.storage.profile_revision = None;
        self.title_background = first_background(&bundle.program);
        self.program = bundle.program;
        self.localizer = bundle.localizer;
        self.project_theme = bundle.theme;
        self.theme = super::ui_accessibility::accessible_theme(&self.project_theme, &self.settings);
        super::ui_text::set_localizer(self.localizer.clone());
        self.screens = bundle.screens;
        self.screen_layouts = super::ui_layouts::ScreenLayouts::new(&self.screens);
        self.storage.screen_lists.clear();
        self.screen_scroll.clear();
        if let Some(font) = font {
            super::text::install_family(font);
        }
        self.history_view.invalidate();
        self.dialogue_view.invalidate();
        self.dialogue_cue_remaining = None;
        self.selected_choice = 0;
        self.clips.clear();
        self.video = None;
        self.assets.invalidate(&bundle.paths);
        self.audio.invalidate(&bundle.paths);
        self.notice = Some(("Project reloaded".to_owned(), 2.0));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reload_preserves_profile_and_rejects_unrestorable_results_transactionally() {
        let root = tempfile::tempdir().unwrap();
        let script = root.path().join("script.rns");
        std::fs::write(&script, "config id \"org.reload-test\"\ndefault persistent_count = 0\nlabel start:\n    @id \"checkpoint\" \"Before\"").unwrap();
        let source = renrs::ProjectSource::Directory(root.path().to_owned());
        let mut app = App::new(
            source.clone(),
            source.compile().unwrap(),
            renrs::theme::Theme::default(),
            &root.path().join(".data"),
        );
        app.start_new_game();
        app.runtime
            .as_mut()
            .unwrap()
            .set_screen_variable("persistent_count", renrs::syntax::Value::Integer(7))
            .unwrap();
        let bundle = |id: &str| {
            std::fs::write(&script, format!("config id \"org.reload-test\"\ndefault persistent_count = 0\nlabel start:\n    @id \"{id}\" \"Edited\"")).unwrap();
            ReloadBundle {
                generation: 1,
                paths: vec!["script.rns".to_owned()],
                program: std::sync::Arc::new(source.compile().unwrap()),
                theme: renrs::theme::Theme::default(),
                screens: renrs::screens::Screens::default(),
                localizer: renrs::Localizer::default(),
                font: None,
            }
        };
        app.reload_project(bundle("checkpoint")).unwrap();
        assert_eq!(
            app.runtime.as_ref().unwrap().profile().variables["persistent_count"],
            renrs::syntax::Value::Integer(7)
        );
        let fingerprint = app.program.fingerprint.clone();
        let mut invalid_font = bundle("checkpoint");
        invalid_font.font = Some(vec![vec![0; 32]]);
        let previous_state =
            serde_json::to_value(app.runtime.as_ref().unwrap().snapshot()).unwrap();
        assert!(app.reload_project(invalid_font).is_err());
        assert_eq!(
            serde_json::to_value(app.runtime.as_ref().unwrap().snapshot()).unwrap(),
            previous_state
        );
        assert!(app.reload_project(bundle("missing")).is_err());
        assert_eq!(app.program.fingerprint, fingerprint);
        assert_eq!(
            app.runtime.as_ref().unwrap().variables()["persistent_count"],
            renrs::syntax::Value::Integer(7)
        );
    }
}
