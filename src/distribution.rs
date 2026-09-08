use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ProjectSource;
use crate::archive::{ArchiveError, pack_project};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildManifest {
    pub engine_version: String,
    pub project_id: String,
    pub title: String,
    pub script_fingerprint: String,
    pub archive: String,
    pub executable: String,
}

#[derive(Debug, Error)]
pub enum BuildError {
    #[error("distribution destination `{0}` already exists")]
    DestinationExists(String),
    #[error("player executable `{0}` does not exist")]
    MissingPlayer(String),
    #[error("project is invalid:\n{0}")]
    InvalidProject(String),
    #[error("could not access distribution files: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not create project archive: {0}")]
    Archive(#[from] ArchiveError),
    #[error("could not encode build manifest: {0}")]
    Manifest(#[from] serde_json::Error),
}

/// Validates a project and assembles a directly playable distribution folder.
///
/// The copied player finds its sibling `game.renrs` from any working directory.
/// The destination must not already exist, avoiding partial
/// replacement of a previous build.
///
/// # Errors
///
/// Returns an error for an invalid project, missing player, existing output,
/// or filesystem/archive failure.
pub fn build_distribution(
    project_path: &Path,
    destination: &Path,
    player: &Path,
) -> Result<BuildManifest, BuildError> {
    if destination.exists() {
        return Err(BuildError::DestinationExists(
            destination.display().to_string(),
        ));
    }
    if !player.is_file() {
        return Err(BuildError::MissingPlayer(player.display().to_string()));
    }
    let source = ProjectSource::open(project_path.to_path_buf())
        .map_err(|error| BuildError::InvalidProject(error.to_string()))?;
    let program = source
        .compile()
        .map_err(|diagnostics| BuildError::InvalidProject(format_diagnostics(&diagnostics)))?;
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let temporary = tempfile::Builder::new()
        .prefix(".renrs-build-")
        .tempdir_in(parent)?;
    let staging = temporary.path();
    let archive_path = staging.join("game.renrs");
    match source {
        ProjectSource::Directory(root) => {
            pack_project(&root, &archive_path)?;
        }
        ProjectSource::Archive(archive) => {
            if let Some(entry) = archive
                .entries()
                .iter()
                .find(|entry| !crate::resources::visible_path(&entry.path))
            {
                return Err(BuildError::InvalidProject(format!(
                    "archive contains excluded resource {}; repack the source project",
                    entry.path
                )));
            }
            fs::copy(archive.path(), &archive_path)?;
        }
    }
    ProjectSource::open(&archive_path)
        .map_err(|error| BuildError::InvalidProject(error.to_string()))?
        .compile()
        .map_err(|errors| BuildError::InvalidProject(format_diagnostics(&errors)))?;
    let archive = crate::archive::ResourceArchive::open(&archive_path)?;
    fs::write(
        staging.join("resources.json"),
        serde_json::to_vec_pretty(archive.entries())?,
    )?;
    let executable_name = if cfg!(windows) { "renrs.exe" } else { "renrs" };
    fs::copy(player, staging.join(executable_name))?;
    let manifest = BuildManifest {
        engine_version: env!("CARGO_PKG_VERSION").to_owned(),
        project_id: program.project_id.clone(),
        title: program.title.clone(),
        script_fingerprint: program.fingerprint.clone(),
        archive: "game.renrs".to_owned(),
        executable: executable_name.to_owned(),
    };
    fs::write(
        staging.join("renrs-build.json"),
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    )?;
    fs::write(
        staging.join("README.txt"),
        "Run the renrs executable. It opens the adjacent game.renrs from any working directory.\n",
    )?;
    fs::write(
        staging.join("LICENSE-renrs.txt"),
        include_str!("../LICENSE"),
    )?;
    fs::write(
        staging.join("LICENSE-font-OFL.txt"),
        include_str!("../assets/fonts/OFL.txt"),
    )?;
    fs::rename(staging, destination)?;
    Ok(manifest)
}

fn format_diagnostics(diagnostics: &[crate::Diagnostic]) -> String {
    diagnostics
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assembles_a_validated_archive_and_player() {
        let temporary = tempfile::tempdir().unwrap();
        let project = temporary.path().join("game");
        fs::create_dir(&project).unwrap();
        fs::write(
            project.join("script.rns"),
            "config id \"example.build\"\nlabel start:\n    \"Hello\"\n",
        )
        .unwrap();
        let player = temporary.path().join(if cfg!(windows) {
            "player.exe"
        } else {
            "player"
        });
        fs::write(&player, b"player").unwrap();
        let output = temporary.path().join("dist");

        let manifest = build_distribution(&project, &output, &player).unwrap();

        assert_eq!(manifest.project_id, "example.build");
        assert!(output.join("game.renrs").is_file());
        assert!(output.join(&manifest.executable).is_file());
        assert!(output.join("renrs-build.json").is_file());
        assert!(output.join("LICENSE-renrs.txt").is_file());
        assert!(output.join("LICENSE-font-OFL.txt").is_file());
        assert!(build_distribution(&project, &output, &player).is_err());
    }
}
