//! Path normalization and safety checks shared by the project, tooling and
//! migration code paths.

use std::path::{Component, Path};

/// Whether `path` is a non-empty relative path made only of normal components.
///
/// Rejects absolute paths, `..`, `.`, and platform-specific prefixes. This is
/// the policy used for archive entries, themes, resources and save slots.
#[must_use]
pub fn safe_relative_path(path: impl AsRef<Path>) -> bool {
    let path = path.as_ref();
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

/// Whether `path` is a portable relative resource path.
///
/// Stricter than [`safe_relative_path`]: also rejects Windows separators and
/// drive colons, and rejects empty, `.` or `..` segments explicitly so the same
/// string is safe on every platform.
#[must_use]
pub fn safe_resource_path(path: &str) -> bool {
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

/// Renders `path` relative to `root` with `/` separators.
///
/// Falls back to `path` when it is not under `root`. Non-UTF-8 components use
/// their lossy representation.
#[must_use]
pub fn relative_name(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
