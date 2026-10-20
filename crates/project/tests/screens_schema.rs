//! Keeps `schemas/screens-v1.schema.json` aligned with the `screens.json` types.

use renrs_project::screens::{
    Action, Element, ListSource, Preference, Screens, TogglePreference, Widget,
};
use renrs_project::theme::ThemeRect;
use serde_json::Value;
use std::collections::BTreeSet;

fn schema() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../schemas/screens-v1.schema.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn keys(value: &Value) -> BTreeSet<String> {
    value.as_object().unwrap().keys().cloned().collect()
}

fn names<T: serde::Serialize>(values: &[T]) -> BTreeSet<String> {
    values
        .iter()
        .map(|value| {
            serde_json::to_value(value)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect()
}

fn schema_enum(value: &Value) -> BTreeSet<String> {
    value["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_owned())
        .collect()
}

fn text() -> String {
    "x".to_owned()
}

fn child() -> Element {
    element(Widget::Text { text: text() })
}

fn element(widget: Widget) -> Element {
    Element {
        bounds: None,
        size: None,
        style: None,
        visible: None,
        widget,
    }
}

/// One instance of every widget with every optional field present. The match
/// is exhaustive so that a new widget fails to compile until it is listed here
/// and in the schema.
fn widgets() -> Vec<Widget> {
    let all = vec![
        Widget::Extension {
            text: text(),
            name: text(),
            variable: text(),
            input: text(),
        },
        Widget::Stack {
            children: vec![child()],
        },
        Widget::Viewport {
            id: text(),
            content_height: 1.0,
            child: Box::new(child()),
        },
        Widget::DataList {
            variable: text(),
            selected: text(),
            label: Some(text()),
            item_height: 1.0,
        },
        Widget::Drag {
            text: text(),
            expression: text(),
        },
        Widget::Drop {
            text: text(),
            variable: text(),
        },
        Widget::Set {
            text: text(),
            variable: text(),
            expression: text(),
        },
        Widget::Dialogue,
        Widget::Choices,
        Widget::Toggle {
            text: text(),
            setting: TogglePreference::WaitVoice,
        },
        Widget::Input {
            text: text(),
            variable: text(),
            max_length: 1,
        },
        Widget::Text { text: text() },
        Widget::Image { path: text() },
        Widget::Button {
            text: text(),
            action: Action::Close,
        },
        Widget::Hotspot {
            action: Some(Action::Close),
            variable: Some(text()),
            expression: Some(text()),
        },
        Widget::Slider {
            text: text(),
            setting: Preference::TextSpeed,
        },
        Widget::List {
            source: ListSource::History,
            item_height: 1.0,
            gap: 1.0,
        },
        Widget::Row {
            gap: 1.0,
            padding: 1.0,
            children: vec![child()],
        },
        Widget::Column {
            gap: 1.0,
            padding: 1.0,
            children: vec![child()],
        },
    ];
    for widget in &all {
        match widget {
            Widget::Extension { .. }
            | Widget::Stack { .. }
            | Widget::Viewport { .. }
            | Widget::DataList { .. }
            | Widget::Drag { .. }
            | Widget::Drop { .. }
            | Widget::Set { .. }
            | Widget::Dialogue
            | Widget::Choices
            | Widget::Toggle { .. }
            | Widget::Input { .. }
            | Widget::Text { .. }
            | Widget::Image { .. }
            | Widget::Button { .. }
            | Widget::Hotspot { .. }
            | Widget::Slider { .. }
            | Widget::List { .. }
            | Widget::Row { .. }
            | Widget::Column { .. } => {}
        }
    }
    all
}

#[test]
fn widget_types_and_fields_match_the_schema() {
    let schema = schema();
    let defs = &schema["$defs"];
    let mut types = BTreeSet::new();
    for widget in widgets() {
        let value = serde_json::to_value(&widget).unwrap();
        let kind = value["type"].as_str().unwrap().to_owned();
        let definition = &defs[format!("widget_{kind}")];
        assert_eq!(
            keys(&value),
            keys(&definition["properties"]),
            "fields of {kind}"
        );
        types.insert(kind);
    }
    assert_eq!(types, schema_enum(&defs["element"]["properties"]["type"]));
    let mut common = keys(&defs["element"]["properties"]);
    common.remove("type");
    let mut full = child();
    full.bounds = Some(ThemeRect::default());
    full.size = Some(1.0);
    full.style = Some(text());
    full.visible = Some(text());
    let mut serialized = keys(&serde_json::to_value(full).unwrap());
    serialized.remove("type");
    serialized.remove("text");
    assert_eq!(serialized, common);
}

#[test]
fn enumerations_match_the_schema() {
    use Action as A;
    let actions = [
        A::Collection,
        A::NewGame,
        A::Continue,
        A::Save,
        A::Load,
        A::Settings,
        A::History,
        A::QuickSave,
        A::QuickLoad,
        A::ManualSaves,
        A::QuickSaves,
        A::AutoSaves,
        A::Rollback,
        A::Auto,
        A::Skip,
        A::Close,
        A::Quit,
    ];
    for action in actions {
        match action {
            A::Collection
            | A::NewGame
            | A::Continue
            | A::Save
            | A::Load
            | A::Settings
            | A::History
            | A::QuickSave
            | A::QuickLoad
            | A::ManualSaves
            | A::QuickSaves
            | A::AutoSaves
            | A::Rollback
            | A::Auto
            | A::Skip
            | A::Close
            | A::Quit => {}
        }
    }
    let sources = [
        ListSource::Saves,
        ListSource::ManualSaves,
        ListSource::QuickSaves,
        ListSource::AutoSaves,
        ListSource::History,
        ListSource::Languages,
    ];
    let preferences = [
        Preference::TextSpeed,
        Preference::AutoDelay,
        Preference::MusicVolume,
        Preference::SoundVolume,
        Preference::VoiceVolume,
    ];
    let toggles = [
        TogglePreference::HighContrast,
        TogglePreference::ReducedMotion,
        TogglePreference::WaitVoice,
        TogglePreference::SelfVoicing,
        TogglePreference::SkipUnread,
        TogglePreference::SkipAfterChoices,
    ];
    let defs = &schema()["$defs"];
    assert_eq!(names(&actions), schema_enum(&defs["action"]));
    assert_eq!(
        names(&sources),
        schema_enum(&defs["widget_list"]["properties"]["source"])
    );
    assert_eq!(
        names(&preferences),
        schema_enum(&defs["widget_slider"]["properties"]["setting"])
    );
    assert_eq!(
        names(&toggles),
        schema_enum(&defs["widget_toggle"]["properties"]["setting"])
    );
}

#[test]
fn top_level_screens_match_the_schema_and_accept_a_schema_reference() {
    let screens = Screens {
        schema: Some(text()),
        ..Screens::default()
    };
    // Optional screens serialize as null, so every top-level field is present.
    assert_eq!(
        keys(&serde_json::to_value(&screens).unwrap()),
        keys(&schema()["properties"])
    );
    let parsed =
        Screens::from_slice(br#"{"$schema":"../schemas/screens-v1.schema.json","version":1}"#)
            .unwrap();
    assert!(parsed.schema.is_some());
}
