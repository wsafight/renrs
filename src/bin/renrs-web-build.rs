use renrs::screens::ScreenKind;
use std::path::{Path, PathBuf};

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let (project, destination, shell) = match args.as_slice() {
        [project, destination] => (
            PathBuf::from(project),
            PathBuf::from(destination),
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("web/dist"),
        ),
        [project, destination, flag, shell] if flag == "--shell" => {
            (project.into(), destination.into(), shell.into())
        }
        _ => return Err("usage: renrs-web-build <project> <output> [--shell web/dist]".into()),
    };
    if destination.exists() {
        return Err("output already exists".into());
    }
    if !shell.join("engine/renrs_web_bg.wasm").is_file() {
        return Err("Web shell missing; run node scripts/build-web.mjs first".into());
    }
    let source = renrs::ProjectSource::open(project)?;
    let program = source.compile().map_err(|errors| {
        errors
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)?;
    let staging = tempfile::tempdir_in(parent)?;
    copy_tree(&shell, staging.path())?;
    let mut catalogs = Vec::new();
    for name in source.resource_names()? {
        if !renrs::resources::visible_path(&name) {
            return Err(format!("excluded resource in archive: {name}").into());
        }
        let bytes = source.read(&name)?;
        let path = staging.path().join("game").join(&name);
        std::fs::create_dir_all(path.parent().ok_or("invalid resource path")?)?;
        std::fs::write(path, &bytes)?;
        if name.starts_with("locales/")
            && Path::new(&name)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
        {
            catalogs.push(serde_json::from_slice::<renrs::TranslationCatalog>(&bytes)?);
        }
    }
    let theme = if source.contains("theme.json") {
        renrs::theme::Theme::from_slice(&source.read("theme.json")?, "theme.json")?
    } else {
        renrs::theme::Theme::default()
    };
    let screens = if source.contains("screens.json") {
        renrs::screens::Screens::from_slice(&source.read("screens.json")?)?
    } else {
        renrs::screens::Screens::default()
    };
    let mut layouts =
        std::collections::BTreeMap::<String, Vec<renrs::screens::PlacedElement>>::new();
    for (name, kind) in [
        ("main_menu", ScreenKind::MainMenu),
        ("hud", ScreenKind::Hud),
        ("save", ScreenKind::Save),
        ("load", ScreenKind::Load),
        ("settings", ScreenKind::Settings),
        ("history", ScreenKind::History),
        ("dialogue", ScreenKind::Dialogue),
        ("choices", ScreenKind::Choices),
    ] {
        if let Some(screen) = screens.get(kind) {
            layouts.insert(name.to_owned(), renrs::screens::layout(screen)?);
        }
    }
    for (name, screen) in &screens.story {
        layouts.insert(format!("story:{name}"), renrs::screens::layout(screen)?);
    }
    std::fs::write(
        staging.path().join("project.json"),
        serde_json::to_vec(
            &serde_json::json!({"program": program, "program_json": serde_json::to_string(&program)?, "catalogs": catalogs, "theme": theme,
                "screens": {"styles": screens.styles, "layouts": layouts},
                "ui_zh": serde_json::from_str::<serde_json::Value>(include_str!("../../crates/player/src/player/ui_zh.json"))?}),
        )?,
    )?;
    std::fs::rename(staging.path(), &destination)?;
    println!("Web build: {}", destination.display());
    Ok(())
}

fn copy_tree(source: &Path, target: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let to = target.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &to)?;
        } else if entry.file_type()?.is_file() {
            std::fs::copy(entry.path(), to)?;
        }
    }
    Ok(())
}
