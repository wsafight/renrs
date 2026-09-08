use super::*;

#[test]
fn maps_options_and_gui_to_project_support_files() {
    let mut support = SupportAccumulator::default();
    let options = support
        .convert(
            "options.rpy",
            "define config.name = _(\"Story\")\ndefine build.name = \"studio.story\"\n",
            &AssetCatalog::empty(),
        )
        .unwrap();
    assert!(options.output.contains("config title \"Story\""));
    assert!(options.output.contains("config id \"studio.story\""));
    let gui = support
        .convert(
            "gui.rpy",
            "define gui.accent_color = '#123456'\ndefine gui.text_size = 24\n",
            &AssetCatalog::empty(),
        )
        .unwrap();
    assert!(gui.issues.is_empty());
    let generated = support.finish().unwrap();
    let theme: Theme = serde_json::from_slice(&generated.theme.unwrap()).unwrap();
    assert_eq!(theme.accent_color, "#123456");
    assert_eq!(theme.dialogue_font_size, 24);
}

#[test]
fn recognizes_default_screen_template() {
    let source = default_screens_source();
    let mut support = SupportAccumulator::default();
    let converted = support
        .convert("screens.rpy", source, &AssetCatalog::empty())
        .unwrap();
    assert_eq!(converted.issues[0].code, "default_screens_replaced");
    assert!(support.finish().unwrap().screens.is_some());
}

#[test]
fn keeps_default_replacement_but_reports_custom_screens() {
    let source = format!("{}screen inventory():\n", default_screens_source());
    let mut support = SupportAccumulator::default();
    let converted = support
        .convert("screens.rpy", &source, &AssetCatalog::empty())
        .unwrap();

    assert_eq!(converted.issues.len(), 2);
    assert_eq!(converted.issues[0].code, "default_screens_replaced");
    assert_eq!(converted.issues[1].code, "custom_screen_unsupported");
    assert_eq!(converted.issues[1].line, 7);
    assert_eq!(support.files[0].status, MigrationSupportStatus::Partial);
    assert_eq!(support.files[0].unmapped_keys, ["screen inventory"]);
    assert!(support.finish().unwrap().screens.is_some());
}

fn default_screens_source() -> &'static str {
    "screen say(who, what):\nscreen choice(items):\nscreen navigation():\nscreen main_menu():\nscreen save():\nscreen load():\n"
}
