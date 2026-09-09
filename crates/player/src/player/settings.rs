use std::collections::HashSet;
use std::fs;
use std::path::Path;

use renrs::StatementId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub(super) struct Settings {
    pub(super) text_speed: f32,
    #[serde(default = "default_auto_delay")]
    pub(super) auto_delay: f32,
    pub(super) music_volume: f32,
    pub(super) sound_volume: f32,
    #[serde(default)]
    pub(super) voice_volume: f32,
    #[serde(default)]
    pub(super) language: Option<String>,
    #[serde(default)]
    pub(super) language_selected: bool,
    #[serde(default = "default_font_scale")]
    pub(super) font_scale: f32,
    #[serde(default)]
    pub(super) high_contrast: bool,
    #[serde(default)]
    pub(super) reduced_motion: bool,
    #[serde(default = "default_wait_voice")]
    pub(super) wait_voice: bool,
    #[serde(default)]
    pub(super) self_voicing: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            text_speed: 42.0,
            auto_delay: default_auto_delay(),
            music_volume: 0.6,
            sound_volume: 0.8,
            voice_volume: 1.0,
            language: None,
            language_selected: false,
            font_scale: 1.0,
            high_contrast: false,
            reduced_motion: false,
            wait_voice: true,
            self_voicing: false,
        }
    }
}

impl Settings {
    pub(super) fn load(path: &Path) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|source| serde_json::from_str(&source).ok())
            .filter(Self::is_valid)
            .unwrap_or_default()
    }

    pub(super) fn save(&self, path: &Path) -> Result<(), String> {
        renrs::storage::write_json(path, self).map_err(|error| error.to_string())
    }

    pub(super) fn is_valid(&self) -> bool {
        (10.0..=100.0).contains(&self.text_speed)
            && (0.5..=10.0).contains(&self.auto_delay)
            && (0.0..=1.0).contains(&self.music_volume)
            && (0.0..=1.0).contains(&self.sound_volume)
            && (0.0..=1.0).contains(&self.voice_volume)
            && (0.8..=1.5).contains(&self.font_scale)
    }
}

const fn default_auto_delay() -> f32 {
    2.0
}
const fn default_font_scale() -> f32 {
    1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_saves_and_rejects_out_of_range_values() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("settings.json");
        assert_eq!(Settings::load(&path).text_speed, 42.0);
        let settings = Settings::default();
        settings.save(&path).unwrap();
        assert!((Settings::load(&path).music_volume - 0.6).abs() < f32::EPSILON);
        let mut invalid = Settings::default();
        invalid.text_speed = 1.0;
        assert!(!invalid.is_valid());
        fs::write(
            &path,
            r#"{"text_speed":1,"auto_delay":2,"music_volume":0.6,"sound_volume":0.8}"#,
        )
        .unwrap();
        assert_eq!(Settings::load(&path).text_speed, 42.0);
    }
}
const fn default_wait_voice() -> bool {
    true
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub(super) struct ReadState {
    pub(super) statements: HashSet<StatementId>,
}

#[derive(Debug, Default)]
pub(super) struct PlaybackModes {
    pub(super) auto: bool,
    pub(super) skip_read: bool,
    pub(super) current_dialogue_was_read: bool,
}

impl ReadState {
    pub(super) fn load(path: &Path) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|source| serde_json::from_str(&source).ok())
            .unwrap_or_default()
    }

    pub(super) fn save(&self, path: &Path) -> Result<(), String> {
        renrs::storage::write_json(path, self).map_err(|error| error.to_string())
    }
}
