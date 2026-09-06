use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

pub const RESOURCE_RULES_FILE: &str = "resources.json";

/// Optional exact paths or directory prefixes ending in `/`. Exclusions win.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ResourceRules {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

impl ResourceRules {
    /// Reads and validates the project's resource rules.
    ///
    /// # Errors
    /// Returns invalid JSON, unsafe rules or filesystem errors.
    pub fn load(root: &Path) -> io::Result<Self> {
        let path = root.join(RESOURCE_RULES_FILE);
        let rules: Self = match fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(io::Error::other)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => Self::default(),
            Err(error) => return Err(error),
        };
        for rule in rules.include.iter().chain(&rules.exclude) {
            if !safe_path(rule.trim_end_matches('/')) || rule.contains(['*', '?']) {
                return Err(io::Error::other(format!(
                    "invalid resource rule `{rule}`; use a relative file or directory/ prefix"
                )));
            }
        }
        Ok(rules)
    }

    #[must_use]
    pub fn includes(&self, path: &str) -> bool {
        if !visible_path(path) {
            return false;
        }
        if path == RESOURCE_RULES_FILE {
            return true;
        }
        let matches = |rule: &String| path == rule || rule.ends_with('/') && path.starts_with(rule);
        !self.exclude.iter().any(matches)
            && (self.include.is_empty() || self.include.iter().any(matches))
    }
}

#[must_use]
pub fn safe_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(['\\', ':'])
        && !Path::new(path).is_absolute()
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
        && Path::new(path)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

#[must_use]
pub fn visible_path(path: &str) -> bool {
    safe_path(path)
        && path.split('/').all(|part| {
            !part.starts_with('.')
                && !matches!(
                    part.to_ascii_lowercase().as_str(),
                    "dist" | "target" | "node_modules"
                )
        })
        && !path.to_ascii_lowercase().ends_with(".renrs")
        && !path.ends_with(['~'])
        && !matches!(
            Path::new(path).extension().and_then(|x| x.to_str()),
            Some("tmp" | "swp" | "swo")
        )
}

/// Returns sorted, regular project files according to the shared resource policy.
///
/// # Errors
/// Returns invalid configuration or filesystem errors. Symlinks are not traversed.
pub fn collect_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    let rules = ResourceRules::load(root)?;
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();
            let relative = relative_name(root, &path);
            if !visible_path(&relative) {
                continue;
            }
            let kind = entry.file_type()?;
            if kind.is_dir() {
                pending.push(path);
            } else if kind.is_file() && rules.includes(&relative) {
                files.push(path);
            }
        }
    }
    files.sort_by_key(|path| relative_name(root, path));
    Ok(files)
}

#[must_use]
pub fn relative_name(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Validates a resource's inclusion and canonical filesystem boundary.
///
/// # Errors
/// Returns an error for excluded resources, escaping symlinks, or absent files.
pub fn resource_path(root: &Path, relative: &str) -> io::Result<PathBuf> {
    if !ResourceRules::load(root)?.includes(relative) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "resource excluded by project rules",
        ));
    }
    let path = root.join(relative);
    let canonical = path.canonicalize()?;
    if !canonical.starts_with(root.canonicalize()?) || !canonical.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "resource is outside the project",
        ));
    }
    // Directory and archive projects must expose the same files.
    let mut current = root.to_path_buf();
    for component in Path::new(relative).components() {
        current.push(component);
        if fs::symlink_metadata(&current)?.file_type().is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "symbolic-link resources are unsupported",
            ));
        }
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn packing_loading_and_watching_share_exclusions() {
        let root = tempfile::tempdir().unwrap();
        for directory in ["dist/previous", "target", "drafts", "story"] {
            fs::create_dir_all(root.path().join(directory)).unwrap();
        }
        for path in [
            ".env",
            "dist/previous/game.renrs",
            "target/copy.rns",
            "drafts/no.rns",
            "story/yes.rns",
            "editor.swp",
        ] {
            fs::write(root.path().join(path), "test").unwrap();
        }
        fs::write(
            root.path().join(RESOURCE_RULES_FILE),
            r#"{"exclude":["drafts/"]}"#,
        )
        .unwrap();
        let names: Vec<_> = collect_files(root.path())
            .unwrap()
            .iter()
            .map(|path| relative_name(root.path(), path))
            .collect();
        assert_eq!(names, [RESOURCE_RULES_FILE, "story/yes.rns"]);
        let archive_path = root.path().join("packed.renrs");
        crate::archive::pack_project(root.path(), &archive_path).unwrap();
        let archive = crate::archive::ResourceArchive::open(archive_path).unwrap();
        assert_eq!(
            archive
                .entries()
                .iter()
                .map(|entry| &entry.path)
                .collect::<Vec<_>>(),
            names.iter().collect::<Vec<_>>()
        );
    }
}
