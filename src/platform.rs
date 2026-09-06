use std::env;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DataDirectoryError {
    #[error("could not determine the operating-system user data directory")]
    Unavailable,
}

/// Resolves a bundled game independently of the launcher's working directory.
/// Explicit project arguments should bypass this fallback lookup.
#[must_use]
pub fn default_project_path(executable: &Path, working_directory: &Path) -> PathBuf {
    let executable_directory = executable.parent().unwrap_or(working_directory);
    for directory in [executable_directory, working_directory] {
        let archive = directory.join("game.renrs");
        if archive.is_file() {
            return archive;
        }
    }
    let bundled_demo = executable_directory.join("demo");
    if bundled_demo.is_dir() {
        bundled_demo
    } else {
        working_directory.join("demo")
    }
}

/// Returns a stable, writable per-game user data directory.
///
/// The directory is not created by this function. Callers retain control over
/// when persistent state first touches the filesystem.
///
/// # Errors
///
/// Returns [`DataDirectoryError::Unavailable`] if the platform does not expose
/// a user data or home directory.
pub fn user_data_directory(project_id: &str) -> Result<PathBuf, DataDirectoryError> {
    user_data_directory_with(project_id, |name| env::var_os(name))
}

#[must_use]
pub fn normalize_project_id(project_id: &str) -> String {
    let normalized = project_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    let normalized = normalized
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if normalized.is_empty() {
        let digest = format!("{:x}", Sha256::digest(project_id.as_bytes()));
        format!("game-{}", &digest[..12])
    } else {
        normalized
    }
}

fn user_data_directory_with(
    project_id: &str,
    environment: impl Fn(&str) -> Option<std::ffi::OsString>,
) -> Result<PathBuf, DataDirectoryError> {
    let project_id = normalize_project_id(project_id);

    #[cfg(target_os = "windows")]
    {
        let base = environment("APPDATA")
            .or_else(|| environment("LOCALAPPDATA"))
            .map(PathBuf::from)
            .ok_or(DataDirectoryError::Unavailable)?;
        Ok(base.join("RenRS").join(project_id))
    }

    #[cfg(target_os = "macos")]
    {
        let home = environment("HOME")
            .map(PathBuf::from)
            .ok_or(DataDirectoryError::Unavailable)?;
        Ok(home
            .join("Library")
            .join("Application Support")
            .join("RenRS")
            .join(project_id))
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(base) = environment("XDG_DATA_HOME").map(PathBuf::from) {
            return Ok(base.join("renrs").join(project_id));
        }
        let home = environment("HOME")
            .map(PathBuf::from)
            .ok_or(DataDirectoryError::Unavailable)?;
        Ok(home
            .join(".local")
            .join("share")
            .join("renrs")
            .join(project_id))
    }

    #[cfg(not(any(unix, target_os = "windows")))]
    {
        let _ = environment;
        Err(DataDirectoryError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn prefers_the_executables_game_over_the_working_directory() {
        let temporary = tempfile::tempdir().unwrap();
        let package = temporary.path().join("package");
        std::fs::create_dir(&package).unwrap();
        let executable = package.join("renrs");
        let working = temporary.path();
        std::fs::write(working.join("game.renrs"), b"unrelated").unwrap();
        assert_eq!(
            default_project_path(&executable, working),
            working.join("game.renrs")
        );
        std::fs::write(package.join("game.renrs"), b"bundled").unwrap();
        assert_eq!(
            default_project_path(&executable, working),
            package.join("game.renrs")
        );
        std::fs::remove_file(package.join("game.renrs")).unwrap();
        std::fs::remove_file(working.join("game.renrs")).unwrap();
        std::fs::create_dir(package.join("demo")).unwrap();
        assert_eq!(
            default_project_path(&executable, working),
            package.join("demo")
        );
    }

    #[test]
    fn normalizes_project_ids_for_directory_names() {
        assert_eq!(
            normalize_project_id("Studio.Example_Game"),
            "studio.example_game"
        );
        assert_eq!(normalize_project_id("My Visual Novel"), "my-visual-novel");
        assert!(normalize_project_id("中文").starts_with("game-"));
    }

    #[test]
    fn resolves_a_platform_data_directory() {
        let directory = user_data_directory_with("example.game", |name| match name {
            "HOME" => Some(OsString::from("/users/test")),
            "APPDATA" => Some(OsString::from("C:/Users/test/AppData/Roaming")),
            "LOCALAPPDATA" => Some(OsString::from("C:/Users/test/AppData/Local")),
            "XDG_DATA_HOME" => Some(OsString::from("/users/test/.data")),
            _ => None,
        })
        .unwrap();
        assert!(directory.ends_with("example.game"));
        assert!(!directory.to_string_lossy().contains("game/.renrs"));
    }
}
