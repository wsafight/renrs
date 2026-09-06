use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProjectState(BTreeMap<String, (u64, Option<SystemTime>)>);

/// Polls metadata of visible project files without coupling filesystem access
/// to the interpreter.
#[derive(Debug)]
pub struct ProjectWatcher {
    root: PathBuf,
    interval: Duration,
    next_scan: Instant,
    state: ProjectState,
}

impl ProjectWatcher {
    /// Creates a watcher and records the initial project state.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the project tree cannot be read.
    pub fn new(root: impl Into<PathBuf>, interval: Duration) -> io::Result<Self> {
        let root = root.into();
        let state = scan(&root)?;
        Ok(Self {
            root,
            interval,
            next_scan: Instant::now() + interval,
            state,
        })
    }

    /// Returns `true` once after one or more script files change.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the project tree cannot be read. The previous
    /// state is retained so a later poll can retry.
    pub fn poll_changed(&mut self) -> io::Result<bool> {
        self.poll_changes().map(|paths| !paths.is_empty())
    }

    /// Returns changed, added, and deleted resource paths. Unchanged files are not read.
    ///
    /// # Errors
    /// Returns an I/O error without replacing the previous metadata snapshot.
    pub fn poll_changes(&mut self) -> io::Result<Vec<String>> {
        let now = Instant::now();
        if now < self.next_scan {
            return Ok(Vec::new());
        }
        self.next_scan = now + self.interval;
        let state = scan(&self.root)?;
        let mut paths: Vec<_> = state
            .0
            .keys()
            .chain(self.state.0.keys())
            .filter(|path| state.0.get(*path) != self.state.0.get(*path))
            .cloned()
            .collect();
        paths.sort();
        paths.dedup();
        self.state = state;
        Ok(paths)
    }
}

fn scan(root: &Path) -> io::Result<ProjectState> {
    let files = crate::resources::collect_files(root)?;

    let mut state = BTreeMap::new();
    for path in files {
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        let metadata = path.metadata()?;
        state.insert(relative, (metadata.len(), metadata.modified().ok()));
    }
    Ok(ProjectState(state))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn reports_support_file_additions_changes_and_removals() {
        let root = tempfile::tempdir().unwrap();
        let mut watcher = ProjectWatcher::new(root.path(), Duration::ZERO).unwrap();
        fs::write(root.path().join("theme.json"), b"{}").unwrap();
        assert_eq!(watcher.poll_changes().unwrap(), ["theme.json"]);
        assert!(watcher.poll_changes().unwrap().is_empty());
        fs::remove_file(root.path().join("theme.json")).unwrap();
        assert_eq!(watcher.poll_changes().unwrap(), ["theme.json"]);
    }

    #[test]
    fn detects_script_changes_and_ignores_hidden_state() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("script.rns"), "label start:\n    return").unwrap();
        let mut watcher = ProjectWatcher::new(root.path(), Duration::ZERO).unwrap();
        assert!(!watcher.poll_changed().unwrap());

        fs::write(
            root.path().join("script.rns"),
            "label start:\n    \"Changed\"",
        )
        .unwrap();
        assert!(watcher.poll_changed().unwrap());
        assert!(!watcher.poll_changed().unwrap());

        fs::create_dir(root.path().join(".renrs")).unwrap();
        fs::write(root.path().join(".renrs/ignored.rns"), "not a script").unwrap();
        assert!(!watcher.poll_changed().unwrap());
    }
}
