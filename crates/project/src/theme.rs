#[cfg(test)]
use std::fs;
#[cfg(test)]
use std::path::PathBuf;
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const THEME_FILE: &str = "theme.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Theme {
    pub font_path: Option<String>,
    pub font_fallbacks: Vec<String>,
    pub dialogue_font_size: u16,
    pub dialogue_line_height: f32,
    pub ui_font_size: u16,
    pub heading_font_size: u16,
    pub title_font_size: u16,
    pub text_color: String,
    pub muted_text_color: String,
    pub accent_color: String,
    pub focus_color: String,
    pub panel_color: String,
    pub surface_color: String,
    pub background_color: String,
    pub danger_color: String,
    pub high_contrast: bool,
    pub reduced_motion: bool,
    pub music_volume: f32,
    pub sound_volume: f32,
    pub voice_volume: f32,
    pub layout: ThemeLayout,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Default for ThemeRect {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ListLayout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub item_height: f32,
    pub gap: f32,
}

impl Default for ListLayout {
    fn default() -> Self {
        Self {
            x: 64.0,
            y: 120.0,
            width: 320.0,
            item_height: 52.0,
            gap: 14.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeLayout {
    pub main_menu_panel: ThemeRect,
    pub main_menu_buttons: ListLayout,
    pub toolbar: ThemeRect,
    pub toolbar_buttons_x: f32,
    #[serde(alias = "dialogue")]
    pub dialogue_rect: ThemeRect,
    pub slots: ListLayout,
}

impl Default for ThemeLayout {
    fn default() -> Self {
        Self {
            main_menu_panel: ThemeRect {
                x: 0.0,
                y: 0.0,
                width: 470.0,
                height: 720.0,
            },
            main_menu_buttons: ListLayout {
                x: 66.0,
                y: 252.0,
                width: 320.0,
                item_height: 52.0,
                gap: 14.0,
            },
            toolbar: ThemeRect {
                x: 0.0,
                y: 0.0,
                width: 1280.0,
                height: 54.0,
            },
            toolbar_buttons_x: 548.0,
            dialogue_rect: ThemeRect {
                x: 42.0,
                y: 488.0,
                width: 1196.0,
                height: 196.0,
            },
            slots: ListLayout {
                x: 180.0,
                y: 116.0,
                width: 920.0,
                item_height: 66.0,
                gap: 12.0,
            },
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            font_path: None,
            font_fallbacks: Vec::new(),
            dialogue_font_size: 30,
            dialogue_line_height: 40.0,
            ui_font_size: 22,
            heading_font_size: 36,
            title_font_size: 64,
            text_color: "#f8fafc".to_owned(),
            muted_text_color: "#cbd5e1".to_owned(),
            accent_color: "#e56f51".to_owned(),
            focus_color: "#f6c85f".to_owned(),
            panel_color: "#111318".to_owned(),
            surface_color: "#34383f".to_owned(),
            background_color: "#263238".to_owned(),
            danger_color: "#a94336".to_owned(),
            high_contrast: false,
            reduced_motion: false,
            music_volume: 0.6,
            sound_volume: 0.8,
            voice_volume: 1.0,
            layout: ThemeLayout::default(),
        }
    }
}

#[derive(Debug, Error)]
pub enum ThemeError {
    #[error("could not read {path}: {source}")]
    Read {
        path: String,
        source: std::io::Error,
    },
    #[error("could not parse {path}: {source}")]
    Parse {
        path: String,
        source: serde_json::Error,
    },
    #[error("invalid theme: {0}")]
    Invalid(String),
}

impl Theme {
    pub fn font_paths(&self) -> impl Iterator<Item = &str> {
        self.font_path
            .iter()
            .chain(&self.font_fallbacks)
            .map(String::as_str)
    }
    /// Loads an optional project theme, falling back to defaults when absent.
    ///
    /// # Errors
    ///
    /// Returns an error when the theme cannot be read, decoded, or validated.
    #[cfg(test)]
    pub fn load(game_root: &Path) -> Result<Self, ThemeError> {
        let path = game_root.join(THEME_FILE);
        if !path.exists() {
            return Ok(Self::default());
        }
        let source = fs::read_to_string(&path).map_err(|source| ThemeError::Read {
            path: path.display().to_string(),
            source,
        })?;
        Self::from_slice(source.as_bytes(), &path.display().to_string())
    }

    /// Decodes and validates a project theme from UTF-8 JSON bytes.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed JSON, unsafe font paths, invalid colors,
    /// unsupported metrics, or layouts outside the logical canvas.
    pub fn from_slice(source: &[u8], path: &str) -> Result<Self, ThemeError> {
        let mut theme =
            serde_json::from_slice::<Self>(source).map_err(|source| ThemeError::Parse {
                path: path.to_owned(),
                source,
            })?;
        theme.validate()?;
        theme.apply_accessibility_palette();
        Ok(theme)
    }

    #[cfg(test)]
    pub fn font_file(&self, game_root: &Path) -> Option<PathBuf> {
        self.font_path
            .as_deref()
            .map(Path::new)
            .filter(|path| safe_relative_path(path))
            .map(|path| game_root.join(path))
    }

    fn validate(&self) -> Result<(), ThemeError> {
        const TOOLBAR_BUTTONS_WIDTH: f32 = 620.0;
        if [self.music_volume, self.sound_volume, self.voice_volume]
            .iter()
            .any(|value| !(0.0..=1.0).contains(value))
        {
            return Err(ThemeError::Invalid(
                "audio defaults must be between 0 and 1".to_owned(),
            ));
        }

        if !(18..=52).contains(&self.dialogue_font_size) {
            return Err(ThemeError::Invalid(
                "dialogue_font_size must be between 18 and 52".to_owned(),
            ));
        }
        if !(f32::from(self.dialogue_font_size) + 2.0..=80.0).contains(&self.dialogue_line_height) {
            return Err(ThemeError::Invalid(
                "dialogue_line_height must exceed the dialogue font size and be at most 80"
                    .to_owned(),
            ));
        }
        if !(14..=36).contains(&self.ui_font_size)
            || !(20..=64).contains(&self.heading_font_size)
            || !(32..=96).contains(&self.title_font_size)
        {
            return Err(ThemeError::Invalid(
                "UI font sizes are outside their supported ranges".to_owned(),
            ));
        }
        for (name, value) in [
            ("text_color", &self.text_color),
            ("muted_text_color", &self.muted_text_color),
            ("accent_color", &self.accent_color),
            ("focus_color", &self.focus_color),
            ("panel_color", &self.panel_color),
            ("surface_color", &self.surface_color),
            ("background_color", &self.background_color),
            ("danger_color", &self.danger_color),
        ] {
            if !valid_color(value) {
                return Err(ThemeError::Invalid(format!(
                    "{name} must use #RRGGBB or #RRGGBBAA"
                )));
            }
        }
        if self.font_fallbacks.len() > 8
            || self
                .font_paths()
                .any(|path| !safe_relative_path(Path::new(path)))
        {
            return Err(ThemeError::Invalid(
                "font_path must be a safe relative path".to_owned(),
            ));
        }
        for (name, rect) in [
            ("layout.main_menu_panel", self.layout.main_menu_panel),
            ("layout.toolbar", self.layout.toolbar),
            ("layout.dialogue_rect", self.layout.dialogue_rect),
        ] {
            validate_rect(name, rect)?;
        }
        validate_list("layout.main_menu_buttons", self.layout.main_menu_buttons, 5)?;
        validate_list("layout.slots", self.layout.slots, 6)?;
        let toolbar_end = self.layout.toolbar.x + self.layout.toolbar.width;
        if self.layout.toolbar_buttons_x < self.layout.toolbar.x
            || self.layout.toolbar_buttons_x + TOOLBAR_BUTTONS_WIDTH > toolbar_end
        {
            return Err(ThemeError::Invalid(
                "layout.toolbar_buttons_x must leave 620px for controls inside layout.toolbar"
                    .to_owned(),
            ));
        }
        Ok(())
    }

    fn apply_accessibility_palette(&mut self) {
        if !self.high_contrast {
            return;
        }
        "#ffffff".clone_into(&mut self.text_color);
        "#e5e7eb".clone_into(&mut self.muted_text_color);
        "#ff765c".clone_into(&mut self.accent_color);
        "#ffe066".clone_into(&mut self.focus_color);
        "#000000".clone_into(&mut self.panel_color);
        "#202124".clone_into(&mut self.surface_color);
        "#101214".clone_into(&mut self.background_color);
        "#c43d2b".clone_into(&mut self.danger_color);
    }
}

fn valid_color(value: &str) -> bool {
    let Some(hex) = value.strip_prefix('#') else {
        return false;
    };
    matches!(hex.len(), 6 | 8) && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn safe_relative_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn validate_rect(name: &str, rect: ThemeRect) -> Result<(), ThemeError> {
    let valid = rect.x >= 0.0
        && rect.y >= 0.0
        && rect.width >= 40.0
        && rect.height >= 40.0
        && rect.x + rect.width <= 1280.0
        && rect.y + rect.height <= 720.0;
    if valid {
        Ok(())
    } else {
        Err(ThemeError::Invalid(format!(
            "{name} must be a positive rectangle inside the 1280x720 canvas"
        )))
    }
}

fn validate_list(name: &str, list: ListLayout, item_count: u16) -> Result<(), ThemeError> {
    let item_count = f32::from(item_count);
    let valid = list.x >= 0.0
        && list.y >= 0.0
        && list.width >= 80.0
        && list.item_height >= 32.0
        && list.gap >= 0.0
        && list.x + list.width <= 1280.0
        && list.y + item_count * list.item_height + (item_count - 1.0) * list.gap <= 720.0;
    if valid {
        Ok(())
    } else {
        Err(ThemeError::Invalid(format!(
            "{name} must fit {item_count} positive items inside the 1280x720 canvas"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_theme_uses_readable_defaults() {
        let root = tempfile::tempdir().unwrap();
        let theme = Theme::load(root.path()).unwrap();
        assert_eq!(theme.dialogue_font_size, 30);
        assert!((theme.dialogue_line_height - 40.0).abs() < f32::EPSILON);
        assert_eq!(theme.text_color, "#f8fafc");
    }

    #[test]
    fn loads_accessible_project_theme() {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join(THEME_FILE),
            r#"{"font_path":"fonts/ui.ttf","dialogue_font_size":32,"dialogue_line_height":44.0,"high_contrast":true,"reduced_motion":true}"#,
        )
        .unwrap();
        let theme = Theme::load(root.path()).unwrap();
        assert_eq!(
            theme.font_file(root.path()),
            Some(root.path().join("fonts/ui.ttf"))
        );
        assert_eq!(theme.text_color, "#ffffff");
        assert!(theme.reduced_motion);
    }

    #[test]
    fn rejects_unsafe_font_and_invalid_metrics() {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join(THEME_FILE),
            r#"{"font_path":"../font.ttf"}"#,
        )
        .unwrap();
        assert!(Theme::load(root.path()).is_err());

        fs::write(
            root.path().join(THEME_FILE),
            r#"{"dialogue_font_size":40,"dialogue_line_height":30.0}"#,
        )
        .unwrap();
        assert!(Theme::load(root.path()).is_err());
    }

    #[test]
    fn accepts_dialogue_alias_and_rejects_overflowing_toolbar_controls() {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join(THEME_FILE),
            r#"{"layout":{"dialogue":{"x":40.0,"y":480.0,"width":1200.0,"height":200.0}}}"#,
        )
        .unwrap();
        assert!(
            (Theme::load(root.path()).unwrap().layout.dialogue_rect.x - 40.0).abs() < f32::EPSILON
        );

        fs::write(
            root.path().join(THEME_FILE),
            r#"{"layout":{"toolbar_buttons_x":700.0}}"#,
        )
        .unwrap();
        assert!(Theme::load(root.path()).is_err());
    }
}
