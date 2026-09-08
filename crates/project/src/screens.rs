use crate::theme::ThemeRect;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod layout;
pub use layout::{PlacedElement, ViewportFrame, layout};

pub const SCREENS_FILE: &str = "screens.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenKind {
    MainMenu,
    Hud,
    Save,
    Load,
    Settings,
    History,
    Dialogue,
    Choices,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Screens {
    pub version: u32,
    pub styles: BTreeMap<String, Style>,
    pub main_menu: Option<Screen>,
    pub hud: Option<Screen>,
    pub save: Option<Screen>,
    pub load: Option<Screen>,
    pub settings: Option<Screen>,
    pub history: Option<Screen>,
    pub dialogue: Option<Screen>,
    pub choices: Option<Screen>,
}

impl Default for Screens {
    fn default() -> Self {
        Self {
            version: 1,
            styles: BTreeMap::new(),
            main_menu: None,
            hud: None,
            save: None,
            load: None,
            settings: None,
            history: None,
            dialogue: None,
            choices: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Style {
    pub font_size: Option<u16>,
    pub text_color: Option<String>,
    pub background_color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Screen {
    pub bounds: ThemeRect,
    pub root: Element,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Element {
    #[serde(default)]
    pub bounds: Option<ThemeRect>,
    #[serde(default)]
    pub size: Option<f32>,
    #[serde(default)]
    pub style: Option<String>,
    #[serde(flatten)]
    pub widget: Widget,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Widget {
    Extension {
        text: String,
        name: String,
        variable: String,
        input: String,
    },
    Stack {
        children: Vec<Element>,
    },
    Viewport {
        id: String,
        content_height: f32,
        child: Box<Element>,
    },
    DataList {
        variable: String,
        selected: String,
        #[serde(default)]
        label: Option<String>,
        #[serde(default = "item_height")]
        item_height: f32,
    },
    Drag {
        text: String,
        expression: String,
    },
    Drop {
        text: String,
        variable: String,
    },
    Set {
        text: String,
        variable: String,
        expression: String,
    },
    Dialogue,
    Choices,
    Toggle {
        text: String,
        setting: TogglePreference,
    },
    Input {
        text: String,
        variable: String,
        #[serde(default = "input_limit")]
        max_length: usize,
    },
    Text {
        text: String,
    },
    Image {
        path: String,
    },
    Button {
        text: String,
        action: Action,
    },
    Slider {
        text: String,
        setting: Preference,
    },
    List {
        source: ListSource,
        #[serde(default = "item_height")]
        item_height: f32,
        #[serde(default = "list_gap")]
        gap: f32,
    },
    Row {
        #[serde(default)]
        gap: f32,
        #[serde(default)]
        padding: f32,
        children: Vec<Element>,
    },
    Column {
        #[serde(default)]
        gap: f32,
        #[serde(default)]
        padding: f32,
        children: Vec<Element>,
    },
}

const fn item_height() -> f32 {
    52.0
}
const fn input_limit() -> usize {
    128
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TogglePreference {
    HighContrast,
    ReducedMotion,
    WaitVoice,
    SelfVoicing,
}
const fn list_gap() -> f32 {
    8.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Collection,
    NewGame,
    Continue,
    Save,
    Load,
    Settings,
    History,
    QuickSave,
    QuickLoad,
    ManualSaves,
    QuickSaves,
    AutoSaves,
    Rollback,
    Auto,
    Skip,
    Close,
    Quit,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListSource {
    Saves,
    ManualSaves,
    QuickSaves,
    AutoSaves,
    History,
    Languages,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Preference {
    TextSpeed,
    AutoDelay,
    MusicVolume,
    SoundVolume,
    VoiceVolume,
}

impl Screens {
    /// Parses and validates the bounded declarative interface format.
    ///
    /// # Errors
    /// Rejects unknown versions/widgets/styles, invalid bounds, and overflowing layouts.
    pub fn from_slice(bytes: &[u8]) -> Result<Self, String> {
        let screens: Self = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        if screens.version != 1 {
            return Err("unsupported screens version".to_owned());
        }
        for style in screens.styles.values() {
            if style
                .font_size
                .is_some_and(|size| !(12..=72).contains(&size))
            {
                return Err("screen font_size must be 12..72".to_owned());
            }
            for color in [&style.text_color, &style.background_color]
                .into_iter()
                .flatten()
            {
                if crate::validator::parse_hex_color(color).is_none() {
                    return Err(format!("invalid screen color {color}"));
                }
            }
        }
        for kind in [
            ScreenKind::MainMenu,
            ScreenKind::Hud,
            ScreenKind::Save,
            ScreenKind::Load,
            ScreenKind::Settings,
            ScreenKind::History,
            ScreenKind::Dialogue,
            ScreenKind::Choices,
        ] {
            if let Some(screen) = screens.get(kind) {
                let elements = layout(screen)?;
                for element in &elements {
                    validate_data_widget(kind, &element.widget)?;
                    validate_set_widget(kind, &element.widget)?;
                    if matches!(element.widget, Widget::Dialogue) && kind != ScreenKind::Dialogue
                        || matches!(element.widget, Widget::Choices) && kind != ScreenKind::Choices
                    {
                        return Err(
                            "dialogue/choices widgets require the corresponding story screen"
                                .to_owned(),
                        );
                    }
                    if element
                        .style
                        .as_ref()
                        .is_some_and(|name| !screens.styles.contains_key(name))
                    {
                        return Err("unknown screen style".to_owned());
                    }
                    if let Widget::List { source, .. } = &element.widget
                        && kind == ScreenKind::Hud
                        && !matches!(source, ListSource::History)
                    {
                        return Err("HUD lists must be read-only history".to_owned());
                    }
                    if kind == ScreenKind::Hud
                        && matches!(
                            element.widget,
                            Widget::Button { .. }
                                | Widget::Slider { .. }
                                | Widget::Toggle { .. }
                                | Widget::Input { .. }
                        )
                    {
                        return Err(
                            "HUD is read-only; interactive widgets belong in menus".to_owned()
                        );
                    }
                }
                validate_story_widget(kind, &elements)?;
                if kind == ScreenKind::MainMenu
                    && !elements.iter().any(|element| {
                        matches!(
                            element.widget,
                            Widget::Button {
                                action: Action::NewGame,
                                ..
                            }
                        )
                    })
                {
                    return Err("main_menu must contain a new_game button".to_owned());
                }
            }
        }
        Ok(screens)
    }

    #[must_use]
    pub const fn get(&self, kind: ScreenKind) -> Option<&Screen> {
        match kind {
            ScreenKind::MainMenu => self.main_menu.as_ref(),
            ScreenKind::Hud => self.hud.as_ref(),
            ScreenKind::Save => self.save.as_ref(),
            ScreenKind::Load => self.load.as_ref(),
            ScreenKind::Settings => self.settings.as_ref(),
            ScreenKind::History => self.history.as_ref(),
            ScreenKind::Dialogue => self.dialogue.as_ref(),
            ScreenKind::Choices => self.choices.as_ref(),
        }
    }

    /// Returns images referenced by all custom screens.
    #[must_use]
    pub fn images(&self) -> Vec<String> {
        [
            ScreenKind::Dialogue,
            ScreenKind::Choices,
            ScreenKind::MainMenu,
            ScreenKind::Hud,
            ScreenKind::Save,
            ScreenKind::Load,
            ScreenKind::Settings,
            ScreenKind::History,
        ]
        .into_iter()
        .filter_map(|kind| self.get(kind))
        .flat_map(|screen| layout(screen).unwrap_or_default())
        .filter_map(|element| {
            if let Widget::Image { path } = element.widget {
                Some(path)
            } else {
                None
            }
        })
        .collect()
    }

    #[must_use]
    pub fn images_for(&self, kind: ScreenKind) -> Vec<String> {
        self.get(kind)
            .into_iter()
            .flat_map(|screen| layout(screen).unwrap_or_default())
            .filter_map(|element| {
                if let Widget::Image { path } = element.widget {
                    Some(path)
                } else {
                    None
                }
            })
            .collect()
    }
}

fn validate_data_widget(kind: ScreenKind, widget: &Widget) -> Result<(), String> {
    let variables = match widget {
        Widget::Extension {
            variable, input, ..
        } => {
            renrs_compiler::expression::parse_expression(input, SCREENS_FILE, 1, 1)
                .map_err(|error| error.to_string())?;
            vec![variable]
        }
        Widget::DataList {
            variable, selected, ..
        } => vec![variable, selected],
        Widget::Drop { variable, .. } => vec![variable],
        Widget::Drag { expression, .. } => {
            renrs_compiler::expression::parse_expression(expression, SCREENS_FILE, 1, 1)
                .map_err(|error| error.to_string())?;
            Vec::new()
        }
        _ => return Ok(()),
    };
    if !matches!(
        kind,
        ScreenKind::Dialogue | ScreenKind::Choices | ScreenKind::Settings
    ) {
        return Err("data controls require a dialogue, choices or settings screen".to_owned());
    }
    for variable in variables {
        if !matches!(
            renrs_compiler::expression::parse_expression(variable, SCREENS_FILE, 1, 1),
            Ok(renrs_syntax::syntax::Expr::Variable(_))
        ) {
            return Err("data control target must be a variable name".to_owned());
        }
    }
    Ok(())
}

fn validate_story_widget(kind: ScreenKind, elements: &[PlacedElement]) -> Result<(), String> {
    if matches!(kind, ScreenKind::Dialogue | ScreenKind::Choices) {
        let count = elements
            .iter()
            .filter(|element| {
                matches!(
                    (kind, &element.widget),
                    (ScreenKind::Dialogue, Widget::Dialogue)
                        | (ScreenKind::Choices, Widget::Choices)
                )
            })
            .count();
        if count != 1 {
            return Err("story screen requires exactly one dialogue/choices widget".to_owned());
        }
    }
    Ok(())
}

fn validate_set_widget(kind: ScreenKind, widget: &Widget) -> Result<(), String> {
    let Widget::Set {
        variable,
        expression,
        ..
    } = widget
    else {
        return Ok(());
    };
    if !matches!(
        kind,
        ScreenKind::Dialogue | ScreenKind::Choices | ScreenKind::Settings
    ) {
        return Err("set widgets require a dialogue, choices or settings screen".to_owned());
    }
    if !matches!(
        renrs_compiler::expression::parse_expression(variable, SCREENS_FILE, 1, 1),
        Ok(renrs_syntax::syntax::Expr::Variable(_))
    ) {
        return Err("set target must be a variable name".to_owned());
    }
    renrs_compiler::expression::parse_expression(expression, SCREENS_FILE, 1, 1)
        .map_err(|error| error.to_string())?;
    Ok(())
}
