use crate::{ProjectSource, normalize_project_id};
use serde_json::json;
use std::fs;
use std::path::Path;

/// Creates a validated, playable project in a new directory.
///
/// # Errors
/// Refuses an existing destination, invalid identity, or any filesystem/validation failure.
pub fn create_project(destination: &Path, title: &str, project_id: &str) -> Result<(), String> {
    create_from_template(destination, title, project_id, "story")
}

/// Creates a playable project from a bundled authoring template.
/// # Errors
/// Rejects unknown templates, existing destinations or invalid project metadata.
pub fn create_from_template(
    destination: &Path,
    title: &str,
    project_id: &str,
    template: &str,
) -> Result<(), String> {
    if !matches!(template, "story" | "inventory") {
        return Err("unknown template".to_owned());
    }
    if title.trim().is_empty() || title.contains(['\n', '\r']) {
        return Err("title must be a nonempty single line".to_owned());
    }
    if project_id.is_empty() || normalize_project_id(project_id) != project_id {
        return Err(
            "project id must use lowercase ASCII letters, digits, dots, hyphens or underscores"
                .to_owned(),
        );
    }
    if destination.exists() {
        return Err(format!(
            "destination already exists: {}",
            destination.display()
        ));
    }
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let name = destination
        .file_name()
        .ok_or("destination must name a new directory")?
        .to_string_lossy();
    let staging = parent.join(format!(".{name}.{}.init", std::process::id()));
    fs::create_dir(&staging).map_err(|error| error.to_string())?;
    let result = write_project(&staging, title, project_id, template).and_then(|()| {
        if destination.exists() {
            return Err("destination appeared during initialization".to_owned());
        }
        fs::rename(&staging, destination).map_err(|error| error.to_string())
    });
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

fn write_project(root: &Path, title: &str, project_id: &str, template: &str) -> Result<(), String> {
    for directory in ["images", "locales", ".vscode"] {
        fs::create_dir(root.join(directory)).map_err(|error| error.to_string())?;
    }
    let header = format!(
        "config title {}\nconfig id {}\n\n",
        json!(title),
        json!(project_id)
    );
    let script = format!("{header}{}", include_str!("scaffold/story.rns"));
    let editor = json!({"renrs.projectPath": ".", "renrs.checkOnSave": true});
    for (path, bytes) in [
        ("script.rns", script.as_bytes()),
        (
            "theme.json",
            include_bytes!("../demo/theme.json").as_slice(),
        ),
        (
            "images/studio.png",
            include_bytes!("../demo/images/studio.png").as_slice(),
        ),
        (
            "images/rooftop.png",
            include_bytes!("../demo/images/rooftop.png").as_slice(),
        ),
        (
            "images/mira.png",
            include_bytes!("../demo/images/mira.png").as_slice(),
        ),
        (
            "locales/zh-Hans.json",
            include_bytes!("scaffold/zh-Hans.json").as_slice(),
        ),
        ("README.md", include_bytes!("scaffold/README.md").as_slice()),
        (
            "screens.json",
            include_bytes!("scaffold/screens.json").as_slice(),
        ),
        (
            "routes.json",
            include_bytes!("scaffold/routes.json").as_slice(),
        ),
        (".gitignore", b"dist/\n.renrs/\n*.renrs\n".as_slice()),
        (
            ".vscode/settings.json",
            serde_json::to_string_pretty(&editor)
                .map_err(|error| error.to_string())?
                .as_bytes(),
        ),
    ] {
        fs::write(root.join(path), bytes).map_err(|error| error.to_string())?;
    }
    if template == "inventory" {
        fs::write(
            root.join("script.rns"),
            format!("{header}{}", include_str!("scaffold/inventory.rns")),
        )
        .map_err(|error| error.to_string())?;
        fs::write(
            root.join("screens.json"),
            include_bytes!("../examples/composable_screens.json"),
        )
        .map_err(|error| error.to_string())?;
        fs::write(root.join("routes.json"), br#"{"routes":[]}"#)
            .map_err(|error| error.to_string())?;
        fs::create_dir(root.join("extensions")).map_err(|error| error.to_string())?;
        fs::write(root.join("extensions/reward.rhai"), "input + 1")
            .map_err(|error| error.to_string())?;
        fs::write(
            root.join("extensions.json"),
            br#"{"version":1,"modules":{"reward":"extensions/reward.rhai"}}"#,
        )
        .map_err(|error| error.to_string())?;
    }
    ProjectSource::Directory(root.to_owned())
        .compile()
        .map_err(|errors| {
            errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unknown_templates_blank_titles_and_invalid_ids() {
        let root = tempfile::tempdir().unwrap();
        assert!(
            create_from_template(&root.path().join("a"), "Title", "org.ok", "missing")
                .unwrap_err()
                .contains("unknown template")
        );
        assert!(create_project(&root.path().join("b"), "", "org.ok").is_err());
        assert!(create_project(&root.path().join("c"), "Title", "Org.Bad").is_err());
    }

    #[test]
    fn inventory_template_writes_extensions_and_compiles() {
        let root = tempfile::tempdir().unwrap();
        let game = root.path().join("bag");
        create_from_template(&game, "Bag", "org.renrs.bag", "inventory").unwrap();
        assert!(game.join("extensions.json").is_file());
        assert!(game.join("extensions/reward.rhai").is_file());
    }
}
