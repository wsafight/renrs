use std::env;
use std::path::{Path, PathBuf};

use super::text::draw_text;
use macroquad::prelude::*;
use renrs::compiler::{InstructionKind, Program};
use renrs::save::SaveRepository;
use renrs::theme::Theme as ProjectTheme;
use renrs::{Localizer, ProjectSource, Runtime, TranslationCatalog};

use crate::frontend::{FocusState, UiActions};

use super::CANVAS_WIDTH;
use super::assets::AssetCache;
use super::audio::AudioManager;
use super::dialogue::DialogueView;
use super::settings::{PlaybackModes, ReadState, Settings};
use super::ui_common::{color, color_alpha, draw_fitted_centered, wrap_lines};
use super::ui_history::HistoryView;
use super::ui_slots::SlotGroup;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Screen {
    MainMenu,
    Playing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Overlay {
    History,
    Settings,
    Languages,
    SaveSlots,
    LoadSlots,
    Confirm,
    SaveTools,
    Accessibility,
    Collection,
}

#[allow(clippy::struct_excessive_bools)]
pub(super) struct App {
    pub(super) canvas_target: Option<RenderTarget>,
    pub(super) transition_target: Option<RenderTarget>,
    pub(super) drag_expression: Option<String>,
    pub(super) redraw: bool,
    pub(super) clips: std::collections::HashMap<String, renrs::video::VideoClip>,
    pub(super) video: Option<super::media::StreamingVideo>,
    pub(super) image_hints: Vec<String>,
    /// Scratch buffer reused by [`App::prepare_assets`] so collecting the
    /// visible image paths does not allocate a fresh vector every wake.
    pub(super) asset_paths: Vec<String>,
    pub(super) screens: renrs::screens::Screens,
    pub(super) screen_layouts: super::ui_layouts::ScreenLayouts,
    pub(super) screen_scroll: std::collections::HashMap<String, usize>,
    pub(super) source: ProjectSource,
    pub(super) program: std::sync::Arc<Program>,
    pub(super) theme: ProjectTheme,
    pub(super) project_theme: ProjectTheme,
    pub(super) focus: FocusState,
    pub(super) runtime: Option<Runtime>,
    pub(super) saves: SaveRepository,
    pub(super) settings_path: PathBuf,
    pub(super) read_path: PathBuf,
    pub(super) assets: AssetCache,
    pub(super) audio: AudioManager,
    pub(super) localizer: Localizer,
    pub(super) settings: Settings,
    pub(super) settings_dirty: bool,
    pub(super) read_state: ReadState,
    pub(super) read_dirty: bool,
    pub(super) screen: Screen,
    pub(super) overlay: Option<Overlay>,
    pub(super) title_background: Option<String>,
    pub(super) visible_characters: f32,
    pub(super) pause_remaining: f32,
    pub(super) effect_remaining: f32,
    pub(super) selected_choice: usize,
    pub(super) playback: PlaybackModes,
    pub(super) auto_remaining: f32,
    pub(super) dialogue_cue_remaining: Option<f32>,
    pub(super) skip_remaining: f32,
    pub(super) play_time_seconds: f64,
    pub(super) notice: Option<(String, f32)>,
    pub(super) fatal_error: Option<String>,
    pub(super) quit: bool,
    pub(super) history_view: HistoryView,
    pub(super) dialogue_view: DialogueView,
    pub(super) inspect: bool,
    pub(super) language_offset: usize,
    pub(super) slot_group: SlotGroup,
    pub(super) storage: super::saving::PlayerStorage,
    pub(super) metrics: super::metrics::Metrics,
}

impl App {
    pub(super) fn new(
        source: ProjectSource,
        program: Program,
        theme: ProjectTheme,
        data_root: &Path,
    ) -> Self {
        let title_background = first_background(&program);
        let saves = SaveRepository::new(data_root.join("saves"));
        let storage = super::saving::PlayerStorage::new(saves.clone());
        let settings_path = data_root.join("settings.json");
        let read_path = data_root.join("read.json");
        let mut settings = Settings::load(&settings_path);
        if !settings_path.exists() {
            settings.music_volume = theme.music_volume;
            settings.sound_volume = theme.sound_volume;
            settings.voice_volume = theme.voice_volume;
        }
        let (localizer, localization_notice) = load_localizer(
            &source,
            settings.language.clone(),
            !settings.language_selected,
        );
        let auto_delay = settings.auto_delay;
        super::ui_text::set_localizer(localizer.clone());
        let project_theme = theme;
        let theme = super::ui_accessibility::accessible_theme(&project_theme, &settings);
        let read_state = ReadState::load(&read_path);
        let screens = super::ui_declarative::load_screens(&source).unwrap_or_default();
        Self {
            canvas_target: None,
            transition_target: None,
            drag_expression: None,
            redraw: true,
            clips: std::collections::HashMap::new(),
            video: None,
            image_hints: Vec::new(),
            asset_paths: Vec::new(),
            screen_layouts: super::ui_layouts::ScreenLayouts::new(&screens),
            screens,
            screen_scroll: std::collections::HashMap::default(),
            source,
            program: std::sync::Arc::new(program),
            theme,
            project_theme,
            focus: FocusState::default(),
            runtime: None,
            saves,
            settings_path,
            read_path,
            assets: AssetCache::default(),
            audio: AudioManager::default(),
            localizer,
            settings,
            settings_dirty: false,
            read_state,
            read_dirty: false,
            screen: Screen::MainMenu,
            overlay: None,
            title_background,
            visible_characters: 0.0,
            pause_remaining: 0.0,
            effect_remaining: 0.0,
            selected_choice: 0,
            playback: PlaybackModes::default(),
            auto_remaining: auto_delay,
            dialogue_cue_remaining: None,
            skip_remaining: 0.0,
            play_time_seconds: 0.0,
            notice: localization_notice.map(|notice| (notice, 6.0)),
            fatal_error: None,
            quit: false,
            history_view: HistoryView::default(),
            dialogue_view: DialogueView::default(),
            inspect: false,
            language_offset: 0,
            slot_group: SlotGroup::Manual,
            storage,
            metrics: super::metrics::Metrics::default(),
        }
    }

    pub(super) fn draw(&mut self, mouse: Vec2, actions: &UiActions) {
        if is_key_pressed(KeyCode::F3) {
            self.toggle_inspect();
        }
        if is_key_pressed(KeyCode::F8) {
            self.settings.self_voicing = !self.settings.self_voicing;
            self.settings_dirty = true;
        }
        super::speech::begin(self.settings.self_voicing);
        match self.screen {
            Screen::MainMenu => self.draw_main_menu(mouse, actions),
            Screen::Playing => self.draw_game(mouse, actions),
        }
        if let Some(overlay) = self.overlay {
            match overlay {
                Overlay::History => self.draw_history(mouse, actions),
                Overlay::Settings => self.draw_settings(mouse, actions),
                Overlay::Languages => self.draw_languages(mouse, actions),
                Overlay::SaveSlots => self.draw_slots(mouse, actions, true),
                Overlay::LoadSlots => self.draw_slots(mouse, actions, false),
                Overlay::Confirm => self.draw_confirmation(mouse, actions),
                Overlay::SaveTools => self.draw_save_tools(mouse, actions),
                Overlay::Accessibility => self.draw_accessibility(mouse, actions),
                Overlay::Collection => self.draw_collection(mouse, actions),
            }
        }
        let spoken = self
            .runtime
            .as_ref()
            .and_then(|runtime| runtime.stage().dialogue.as_ref())
            .map_or("", |dialogue| dialogue.text.as_str());
        if let Some(error) = super::speech::finish(spoken) {
            self.notice = Some((error, 6.0));
        }
        self.draw_inspect();
        if let Some((message, _)) = &self.notice {
            draw_rectangle(
                360.0,
                18.0,
                560.0,
                46.0,
                color_alpha(&self.theme.panel_color, 0.97),
            );
            draw_fitted_centered(
                Rect::new(360.0, 18.0, 560.0, 46.0),
                message,
                self.theme.ui_font_size.saturating_sub(2),
                color(&self.theme.text_color),
            );
        }
        if let Some(error) = &self.fatal_error {
            draw_rectangle(
                0.0,
                616.0,
                CANVAS_WIDTH,
                104.0,
                color(&self.theme.danger_color),
            );
            for (index, line) in wrap_lines(error, 1200.0, self.theme.ui_font_size)
                .iter()
                .take(2)
                .enumerate()
            {
                draw_text(
                    line,
                    40.0,
                    656.0 + index as f32 * 32.0,
                    f32::from(self.theme.ui_font_size),
                    WHITE,
                );
            }
        }
    }
}

pub(super) fn first_background(program: &Program) -> Option<String> {
    program.instructions.iter().find_map(|instruction| {
        if let InstructionKind::Scene { path } = &instruction.kind {
            Some(path.clone())
        } else {
            None
        }
    })
}

pub(super) fn load_localizer(
    source: &ProjectSource,
    configured_language: Option<String>,
    use_environment: bool,
) -> (Localizer, Option<String>) {
    let language = configured_language.or_else(|| {
        use_environment
            .then(|| env::var("RENRS_LANGUAGE").ok())
            .flatten()
    });
    let mut localizer = Localizer::default();
    let mut warnings = Vec::new();
    if let Err(error) = localizer.set_language(language) {
        warnings.push(error.to_string());
    }
    match source.resource_names() {
        Ok(names) => {
            for path in names.into_iter().filter(|path| {
                path.starts_with("locales/")
                    && Path::new(path)
                        .extension()
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
            }) {
                match source.read(&path).and_then(|bytes| {
                    TranslationCatalog::from_reader(std::io::Cursor::new(bytes)).map_err(|error| {
                        renrs::ProjectSourceError::Io(std::io::Error::other(error))
                    })
                }) {
                    Ok(catalog) => {
                        if let Err(error) = localizer.insert(catalog) {
                            warnings.push(format!("{path}: {error}"));
                        }
                    }
                    Err(error) => warnings.push(format!("{path}: {error}")),
                }
            }
        }
        Err(error) => warnings.push(format!("Could not scan translations: {error}")),
    }
    let notice = (!warnings.is_empty()).then(|| format!("Translations: {}", warnings.join("; ")));
    (localizer, notice)
}
