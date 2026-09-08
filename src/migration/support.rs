use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::path::Path;

use renrs_project::screens::Screens;
use renrs_project::theme::Theme;

use super::assets::AssetCatalog;
use super::conversion::ConvertedScript;
use super::expressions::escape_string;
use super::static_values::{StaticValue, parse_assignment, resolve};
use super::{MigrationIssueKind, MigrationSupportFile, MigrationSupportStatus, coded_issue};

const BUILT_IN_SCREENS: &[&str] = &[
    "say",
    "input",
    "choice",
    "quick_menu",
    "navigation",
    "main_menu",
    "game_menu",
    "about",
    "save",
    "load",
    "file_slots",
    "preferences",
    "history",
    "help",
    "keyboard_help",
    "mouse_help",
    "gamepad_help",
    "confirm",
    "skip_indicator",
    "notify",
    "nvl",
    "nvl_dialogue",
];

#[derive(Debug, Default)]
pub(super) struct SupportAccumulator {
    theme: Theme,
    write_theme: bool,
    write_screens: bool,
    pub(super) files: Vec<MigrationSupportFile>,
}

pub(super) struct GeneratedSupport {
    pub(super) theme: Option<Vec<u8>>,
    pub(super) screens: Option<Vec<u8>>,
}

impl SupportAccumulator {
    pub(super) fn convert(
        &mut self,
        file: &str,
        source: &str,
        catalog: &AssetCatalog,
    ) -> Option<ConvertedScript> {
        match Path::new(file)
            .file_name()?
            .to_str()?
            .to_ascii_lowercase()
            .as_str()
        {
            "options.rpy" => Some(self.convert_options(file, source)),
            "gui.rpy" => Some(self.convert_gui(file, source, catalog)),
            "screens.rpy" => Some(self.convert_screens(file, source)),
            _ => None,
        }
    }

    pub(super) fn finish(self) -> Result<GeneratedSupport, serde_json::Error> {
        Ok(GeneratedSupport {
            theme: self
                .write_theme
                .then(|| serde_json::to_vec_pretty(&self.theme))
                .transpose()?,
            screens: self
                .write_screens
                .then(|| serde_json::to_vec_pretty(&Screens::default()))
                .transpose()?,
        })
    }

    fn convert_options(&mut self, file: &str, source: &str) -> ConvertedScript {
        let values = assignments(source, &["config.", "build.", "preferences."]);
        let mut mapped = BTreeSet::new();
        let mut output = "# Migrated from Ren'Py options.rpy.\n".to_owned();
        if let Some(StaticValue::String(title)) = resolve(&values, "config.name") {
            writeln!(output, "config title \"{}\"", escape_string(title))
                .expect("writing to a String cannot fail");
            mapped.insert("config.name".to_owned());
        }
        if let Some(StaticValue::String(id)) = resolve(&values, "build.name") {
            let id = safe_project_id(id);
            writeln!(output, "config id \"{}\"", escape_string(&id))
                .expect("writing to a String cannot fail");
            mapped.insert("build.name".to_owned());
        }
        for (source_key, volume) in [
            ("config.default_music_volume", &mut self.theme.music_volume),
            ("config.default_sfx_volume", &mut self.theme.sound_volume),
            ("config.default_voice_volume", &mut self.theme.voice_volume),
        ] {
            if let Some(StaticValue::Number(value)) = resolve(&values, source_key)
                && (0.0..=1.0).contains(value)
            {
                *volume = *value;
                self.write_theme = true;
                mapped.insert(source_key.to_owned());
            }
        }
        self.support_result(file, "options", output, &values, mapped)
    }

    fn convert_gui(&mut self, file: &str, source: &str, catalog: &AssetCatalog) -> ConvertedScript {
        let values = assignments(source, &["gui.", "config."]);
        let mut mapped = BTreeSet::new();
        map_color(
            &values,
            "gui.text_color",
            &mut self.theme.text_color,
            &mut mapped,
        );
        map_color(
            &values,
            "gui.idle_small_color",
            &mut self.theme.muted_text_color,
            &mut mapped,
        );
        map_color(
            &values,
            "gui.accent_color",
            &mut self.theme.accent_color,
            &mut mapped,
        );
        map_color(
            &values,
            "gui.hover_color",
            &mut self.theme.focus_color,
            &mut mapped,
        );
        map_color(
            &values,
            "gui.muted_color",
            &mut self.theme.panel_color,
            &mut mapped,
        );
        map_color(
            &values,
            "gui.idle_color",
            &mut self.theme.surface_color,
            &mut mapped,
        );
        map_size(
            &values,
            "gui.text_size",
            &mut self.theme.dialogue_font_size,
            18..=52,
            &mut mapped,
        );
        map_size(
            &values,
            "gui.interface_text_size",
            &mut self.theme.ui_font_size,
            14..=36,
            &mut mapped,
        );
        map_size(
            &values,
            "gui.label_text_size",
            &mut self.theme.heading_font_size,
            20..=64,
            &mut mapped,
        );
        map_size(
            &values,
            "gui.title_text_size",
            &mut self.theme.title_font_size,
            32..=96,
            &mut mapped,
        );
        self.theme.dialogue_line_height = (f32::from(self.theme.dialogue_font_size) * 1.35)
            .max(f32::from(self.theme.dialogue_font_size) + 2.0)
            .min(80.0);
        if let Some(StaticValue::String(font)) = resolve(&values, "gui.text_font")
            && let Some(path) = catalog.find_resource(font)
        {
            self.theme.font_path = Some(path.to_owned());
            mapped.insert("gui.text_font".to_owned());
        }
        map_dialogue_layout(&values, &mut self.theme, &mut mapped);
        self.write_theme = true;
        self.support_result(
            file,
            "gui",
            "# Ren'Py GUI values were migrated to theme.json.\n".to_owned(),
            &values,
            mapped,
        )
    }

    fn convert_screens(&mut self, file: &str, source: &str) -> ConvertedScript {
        let screen_names = source
            .lines()
            .filter_map(|line| screen_name(line.trim_start_matches('\u{feff}').trim()))
            .collect::<Vec<_>>();
        let standard = ["say", "choice", "navigation", "main_menu", "save", "load"];
        let is_default = standard
            .iter()
            .all(|expected| screen_names.contains(expected));
        let mut issues = Vec::new();
        let mut mapped = Vec::new();
        let mut unmapped = Vec::new();
        if is_default {
            self.write_screens = true;
            mapped.extend(
                screen_names
                    .iter()
                    .filter(|name| BUILT_IN_SCREENS.contains(name))
                    .map(|name| format!("screen {name}")),
            );
            issues.push(coded_issue(
                file,
                1,
                MigrationIssueKind::Assumption,
                "default_screens_replaced",
                "replaced the Ren'Py default screen template with RenRS built-in screens"
                    .to_owned(),
            ));
            for (line, name) in source.lines().enumerate().filter_map(|(line, source)| {
                screen_name(source.trim_start_matches('\u{feff}').trim())
                    .filter(|name| !BUILT_IN_SCREENS.contains(name))
                    .map(|name| (line + 1, name))
            }) {
                unmapped.push(format!("screen {name}"));
                issues.push(coded_issue(
                    file,
                    line,
                    MigrationIssueKind::Unsupported,
                    "custom_screen_unsupported",
                    format!("custom screen `{name}` requires manual migration"),
                ));
            }
        } else {
            for (line, name) in source.lines().enumerate().filter_map(|(line, source)| {
                screen_name(source.trim_start_matches('\u{feff}').trim())
                    .map(|name| (line + 1, name))
            }) {
                unmapped.push(format!("screen {name}"));
                issues.push(coded_issue(
                    file,
                    line,
                    MigrationIssueKind::Unsupported,
                    "custom_screen_unsupported",
                    format!("custom screen `{name}` requires manual migration"),
                ));
            }
        }
        mapped.sort();
        mapped.dedup();
        unmapped.sort();
        unmapped.dedup();
        self.files.push(MigrationSupportFile {
            file: file.to_owned(),
            kind: "screens".to_owned(),
            status: if is_default && unmapped.is_empty() {
                MigrationSupportStatus::BuiltInReplacement
            } else if is_default {
                MigrationSupportStatus::Partial
            } else {
                MigrationSupportStatus::ManualReview
            },
            mapped_keys: mapped,
            unmapped_keys: unmapped,
        });
        ConvertedScript {
            output: if is_default {
                "# Ren'Py default screens are provided by RenRS screens.json.\n".to_owned()
            } else {
                "# TODO migration: custom Ren'Py screens require manual migration.\n".to_owned()
            },
            issues,
            generated_assets: Vec::new(),
        }
    }

    fn support_result(
        &mut self,
        file: &str,
        kind: &str,
        output: String,
        values: &BTreeMap<String, StaticValue>,
        mapped: BTreeSet<String>,
    ) -> ConvertedScript {
        let unmapped = values
            .keys()
            .filter(|key| !mapped.contains(*key))
            .cloned()
            .collect::<Vec<_>>();
        let status = if unmapped.is_empty() {
            MigrationSupportStatus::Converted
        } else {
            MigrationSupportStatus::Partial
        };
        let issues = (!unmapped.is_empty())
            .then(|| {
                coded_issue(
                    file,
                    1,
                    MigrationIssueKind::Assumption,
                    "support_file_partial",
                    format!(
                        "mapped {} supported {kind} value(s); {} value(s) remain documented as unmapped",
                        mapped.len(),
                        unmapped.len()
                    ),
                )
            })
            .into_iter()
            .collect();
        self.files.push(MigrationSupportFile {
            file: file.to_owned(),
            kind: kind.to_owned(),
            status,
            mapped_keys: mapped.into_iter().collect(),
            unmapped_keys: unmapped,
        });
        ConvertedScript {
            output,
            issues,
            generated_assets: Vec::new(),
        }
    }
}

fn assignments(source: &str, prefixes: &[&str]) -> BTreeMap<String, StaticValue> {
    source
        .lines()
        .filter_map(parse_assignment)
        .filter(|assignment| {
            prefixes
                .iter()
                .any(|prefix| assignment.target.starts_with(prefix))
        })
        .map(|assignment| (assignment.target, assignment.value))
        .collect()
}

fn map_color(
    values: &BTreeMap<String, StaticValue>,
    key: &str,
    target: &mut String,
    mapped: &mut BTreeSet<String>,
) {
    if let Some(StaticValue::String(value)) = resolve(values, key)
        && valid_color(value)
    {
        target.clone_from(value);
        mapped.insert(key.to_owned());
    }
}

fn map_size(
    values: &BTreeMap<String, StaticValue>,
    key: &str,
    target: &mut u16,
    range: std::ops::RangeInclusive<u16>,
    mapped: &mut BTreeSet<String>,
) {
    if let Some(StaticValue::Number(value)) = resolve(values, key)
        && value.fract() == 0.0
        && *value >= 0.0
        && *value <= f32::from(u16::MAX)
        && let Ok(value) = value.to_string().parse::<u16>()
        && range.contains(&value)
    {
        *target = value;
        mapped.insert(key.to_owned());
    }
}

fn map_dialogue_layout(
    values: &BTreeMap<String, StaticValue>,
    theme: &mut Theme,
    mapped: &mut BTreeSet<String>,
) {
    let number = |key| match resolve(values, key) {
        Some(StaticValue::Number(value)) => Some(*value),
        _ => None,
    };
    let (Some(x), Some(y), Some(width), Some(box_height)) = (
        number("gui.dialogue_xpos"),
        number("gui.dialogue_ypos"),
        number("gui.dialogue_width"),
        number("gui.textbox_height"),
    ) else {
        return;
    };
    let height = (box_height - y).max(40.0);
    let top = 720.0 - box_height + y;
    if x >= 0.0 && top >= 0.0 && width >= 40.0 && x + width <= 1280.0 && top + height <= 720.0 {
        theme.layout.dialogue_rect = renrs_project::theme::ThemeRect {
            x,
            y: top,
            width,
            height,
        };
        mapped.extend(
            [
                "gui.dialogue_xpos",
                "gui.dialogue_ypos",
                "gui.dialogue_width",
                "gui.textbox_height",
            ]
            .into_iter()
            .map(str::to_owned),
        );
    }
}

fn valid_color(value: &str) -> bool {
    value.strip_prefix('#').is_some_and(|hex| {
        matches!(hex.len(), 6 | 8) && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

fn safe_project_id(value: &str) -> String {
    let normalized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    if normalized.bytes().any(|byte| byte.is_ascii_alphanumeric()) {
        normalized
    } else {
        "migrated-game".to_owned()
    }
}

fn screen_name(source: &str) -> Option<&str> {
    let header = source
        .strip_prefix("screen ")?
        .split(['(', ':'])
        .next()?
        .trim();
    (!header.is_empty()).then_some(header)
}

#[cfg(test)]
#[path = "support_tests.rs"]
mod tests;
