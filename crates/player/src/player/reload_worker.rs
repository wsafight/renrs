use super::app::load_localizer;
use super::bootstrap::load_theme;
use notify::Watcher;
use renrs::{Localizer, Program, ProjectSource};
use std::collections::BTreeMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
    mpsc,
};
use std::time::Duration;

pub(super) struct ReloadBundle {
    pub(super) generation: u64,
    pub(super) paths: Vec<String>,
    pub(super) program: Arc<Program>,
    pub(super) theme: renrs::theme::Theme,
    pub(super) screens: renrs::screens::Screens,
    pub(super) localizer: Localizer,
    pub(super) font: Option<Vec<Vec<u8>>>,
}

type ReloadResult = (u64, Result<ReloadBundle, String>);

pub(super) struct ReloadWorker {
    latest: Arc<Mutex<Option<ReloadResult>>>,
    revision: Arc<AtomicU64>,
    applied: Arc<AtomicU64>,
    stop: mpsc::Sender<()>,
}

impl ReloadWorker {
    pub(super) fn new(source: ProjectSource) -> Self {
        let latest = Arc::new(Mutex::new(None));
        let output = latest.clone();
        let (stop, stopped) = mpsc::channel();
        let revision = Arc::new(AtomicU64::new(1));
        let edits = revision.clone();
        let applied = Arc::new(AtomicU64::new(0));
        let acknowledged = applied.clone();
        std::thread::spawn(move || {
            let Some(root) = source.watch_root() else {
                return;
            };
            let changed = edits.clone();
            let mut watcher =
                notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                    if event.is_err()
                        || event
                            .is_ok_and(|event| !matches!(event.kind, notify::EventKind::Access(_)))
                    {
                        changed.fetch_add(1, Ordering::AcqRel);
                    }
                })
                .ok();
            if let Some(watcher) = &mut watcher {
                let _ = watcher.watch(root, notify::RecursiveMode::Recursive);
            }
            let mut scan = match renrs::watch::ProjectWatcher::new(root, Duration::ZERO) {
                Ok(scan) => scan,
                Err(error) => {
                    *output.lock().unwrap() =
                        Some((edits.load(Ordering::Acquire), Err(error.to_string())));
                    return;
                }
            };
            let mut cache = renrs_project::compile_cache::CompileCache::default();
            let _ = cache.compile(&source);
            let mut pending = BTreeMap::new();
            let mut generation = 0;
            let mut ticks = 0;
            let mut scanned = 0;
            let mut retry = false;
            while let Err(mpsc::RecvTimeoutError::Timeout) =
                stopped.recv_timeout(Duration::from_millis(250))
            {
                ticks += 1;
                pending.retain(|_, changed| *changed > acknowledged.load(Ordering::Acquire));
                let revision = edits.load(Ordering::Acquire);
                if revision == scanned && !retry && ticks % 20 != 0 {
                    continue;
                }
                let paths = match scan.poll_changes() {
                    Ok(paths) => paths,
                    Err(error) => {
                        *output.lock().unwrap() = Some((revision, Err(error.to_string())));
                        continue;
                    }
                };
                scanned = revision;
                let outdated = output
                    .lock()
                    .unwrap()
                    .as_ref()
                    .is_some_and(|(old, _)| *old != revision);
                if paths.is_empty() && !retry && !outdated {
                    continue;
                }
                if !paths.is_empty() {
                    generation += 1;
                }
                pending.extend(paths.into_iter().map(|path| (path, generation)));
                let result = prepare(
                    &source,
                    &mut cache,
                    pending.keys().cloned().collect(),
                    generation,
                );
                // A newer edit supersedes a completed compile before it reaches the renderer.
                if edits.load(Ordering::Acquire) != revision {
                    retry = true;
                    continue;
                }
                retry = false;
                *output.lock().unwrap() = Some((revision, result));
            }
        });
        Self {
            latest,
            revision,
            applied,
            stop,
        }
    }

    pub(super) fn poll(&self) -> Option<Result<ReloadBundle, String>> {
        let mut latest = self.latest.lock().unwrap();
        if latest.as_ref()?.0 != self.revision.load(Ordering::Acquire) {
            return None;
        }
        latest.take().map(|(_, result)| result)
    }

    pub(super) fn acknowledge(&self, generation: u64) {
        self.applied.store(generation, Ordering::Release);
    }
}

impl Drop for ReloadWorker {
    fn drop(&mut self) {
        let _ = self.stop.send(());
    }
}

fn prepare(
    source: &ProjectSource,
    cache: &mut renrs_project::compile_cache::CompileCache,
    paths: Vec<String>,
    generation: u64,
) -> Result<ReloadBundle, String> {
    let program = Arc::new(cache.compile(source).map_err(|errors| {
        errors
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    })?);
    let theme = load_theme(source).map_err(|error| error.to_string())?;
    let screens = super::ui_declarative::load_screens(source)?;
    let (localizer, notice) = load_localizer(source, None, false);
    if let Some(notice) = notice {
        return Err(notice);
    }
    let font = if paths
        .iter()
        .any(|path| path == "theme.json" || theme.font_paths().any(|font| font == path))
    {
        Some(
            theme
                .font_paths()
                .map(|path| source.read(path).map_err(|error| error.to_string()))
                .collect::<Result<Vec<_>, _>>()?,
        )
    } else {
        None
    };
    Ok(ReloadBundle {
        generation,
        paths,
        program,
        theme,
        screens,
        localizer,
        font,
    })
}
