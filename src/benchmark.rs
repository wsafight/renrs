use crate::save::SaveRepository;
use crate::{ProjectSource, Runtime, WaitState};
use serde::Serialize;
use std::fmt::Write;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

#[derive(Debug, Serialize)]
pub struct BenchmarkReport {
    pub profile: &'static str,
    pub platform: String,
    pub instructions: usize,
    pub resources: usize,
    pub compile_ms: Vec<f64>,
    pub play_ms: f64,
    pub interactions: usize,
    pub snapshot_bytes: usize,
    pub snapshot_serialize_ms: f64,
    pub restore_ms: f64,
    pub disk_load_ms: f64,
    pub save_bytes: u64,
    pub save_ms: f64,
    pub listing_ms: f64,
    pub cached_listing_ms: f64,
    pub watcher_idle_ms: f64,
    pub pack_ms: f64,
    pub archive_bytes: u64,
    pub archive_compile_ms: f64,
}

/// Measures real compile, execution, snapshot, save, archive, and watcher operations.
///
/// # Errors
/// Returns an error for invalid input, storage failures, or a story that cannot
/// finish by selecting the first visible option within 100,000 interactions.
#[allow(clippy::too_many_lines)]
pub fn measure(path: &Path, iterations: usize) -> Result<BenchmarkReport, String> {
    if !(1..=100).contains(&iterations) {
        return Err("iterations must be 1..100".to_owned());
    }
    let mut compile_ms = Vec::new();
    for _ in 0..iterations {
        let start = Instant::now();
        compile(path)?;
        compile_ms.push(elapsed_ms(start));
    }
    let source = ProjectSource::open(path).map_err(|error| error.to_string())?;
    let program = compile(path)?;
    let start = Instant::now();
    let mut runtime = Runtime::new(program.clone()).map_err(|error| error.to_string())?;
    let mut state = runtime.advance().map_err(|error| error.to_string())?;
    let mut interactions = 1;
    while state != WaitState::Finished {
        if interactions >= 100_000 {
            return Err("benchmark route exceeded 100000 interactions".to_owned());
        }
        state = if matches!(state, WaitState::Choice { .. }) {
            runtime.choose(0)
        } else {
            runtime.continue_story()
        }
        .map_err(|error| error.to_string())?;
        runtime.drain_audio_events().for_each(drop);
        interactions += 1;
    }
    let play_ms = elapsed_ms(start);
    let start = Instant::now();
    let snapshot = runtime.snapshot();
    let bytes = serde_json::to_vec(&snapshot).map_err(|error| error.to_string())?;
    let snapshot_serialize_ms = elapsed_ms(start);
    let start = Instant::now();
    Runtime::restore(program.clone(), snapshot.clone()).map_err(|error| error.to_string())?;
    let restore_ms = elapsed_ms(start);
    let temporary = tempfile::tempdir().map_err(|error| error.to_string())?;
    let saves = SaveRepository::new(temporary.path().join("saves"));
    let start = Instant::now();
    saves
        .save("slot-1", &snapshot)
        .map_err(|error| error.to_string())?;
    let save_ms = elapsed_ms(start);
    let save_bytes = fs::metadata(saves.root().join("slot-1.json"))
        .map_err(|error| error.to_string())?
        .len();
    let start = Instant::now();
    let loaded = saves.load("slot-1").map_err(|error| error.to_string())?;
    Runtime::restore(program.clone(), loaded.snapshot).map_err(|error| error.to_string())?;
    let disk_load_ms = elapsed_ms(start);
    for index in 2..=6 {
        saves
            .save(&format!("slot-{index}"), &snapshot)
            .map_err(|error| error.to_string())?;
    }
    let start = Instant::now();
    saves.list().map_err(|error| error.to_string())?;
    let listing_ms = elapsed_ms(start);
    saves.list_cached().map_err(|error| error.to_string())?;
    let start = Instant::now();
    for _ in 0..100 {
        std::hint::black_box(saves.list_cached().map_err(|error| error.to_string())?);
    }
    let cached_listing_ms = elapsed_ms(start) / 100.0;
    let mut watcher_idle_ms = 0.0;
    if let Some(root) = source.watch_root() {
        let mut watcher = crate::watch::ProjectWatcher::new(root, Duration::ZERO)
            .map_err(|error| error.to_string())?;
        let start = Instant::now();
        for _ in 0..10 {
            watcher.poll_changes().map_err(|error| error.to_string())?;
        }
        watcher_idle_ms = elapsed_ms(start) / 10.0;
    }
    let archive = temporary.path().join("game.renrs");
    let start = Instant::now();
    match &source {
        ProjectSource::Directory(root) => {
            crate::archive::pack_project(root, &archive).map_err(|error| error.to_string())?;
        }
        ProjectSource::Archive(_) => {
            fs::copy(path, &archive).map_err(|error| error.to_string())?;
        }
    }
    let pack_ms = elapsed_ms(start);
    let start = Instant::now();
    let archived = compile(&archive)?;
    let archive_compile_ms = elapsed_ms(start);
    if program.fingerprint != archived.fingerprint {
        return Err("directory and archive fingerprints differ".to_owned());
    }
    Ok(BenchmarkReport {
        profile: if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        platform: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        instructions: program.instructions.len(),
        resources: source
            .resource_names()
            .map_err(|error| error.to_string())?
            .len(),
        compile_ms,
        play_ms,
        interactions,
        snapshot_bytes: bytes.len(),
        snapshot_serialize_ms,
        restore_ms,
        disk_load_ms,
        save_bytes,
        save_ms,
        listing_ms,
        cached_listing_ms,
        watcher_idle_ms,
        pack_ms,
        archive_bytes: fs::metadata(archive)
            .map_err(|error| error.to_string())?
            .len(),
        archive_compile_ms,
    })
}

/// Generates a reproducible multi-file workload with distinct scene textures.
///
/// # Errors
/// Refuses existing destinations and sizes outside 1..200 chapters or 1..1000 lines per chapter.
pub fn generate(destination: &Path, chapters: usize, lines: usize) -> Result<(), String> {
    if !(1..=200).contains(&chapters) || !(1..=1000).contains(&lines) {
        return Err("chapters must be 1..200 and lines must be 1..1000".to_owned());
    }
    crate::scaffold::create_project(destination, "Signal Study", "org.renrs.benchmark")?;
    let mut script = String::from(
        "config title \"Signal Study\"\nconfig id \"org.renrs.benchmark\"\ndefault trust = 0\nlabel start:\n    jump chapter_0\nlabel ending:\n    @id \"benchmark.end\" \"The last signal fades into the morning.\"\n    return\n",
    );
    fs::create_dir(destination.join("chapters")).map_err(|error| error.to_string())?;
    let paragraph = "The receiver carried a distant voice across the city. We noted the time, checked the frequency, and waited for the next reply. ".repeat(10);
    for chapter in 0..chapters {
        let path = format!("images/chapter-{chapter:03}.png");
        fs::copy(
            destination.join(if chapter % 2 == 0 {
                "images/studio.png"
            } else {
                "images/rooftop.png"
            }),
            destination.join(&path),
        )
        .map_err(|error| error.to_string())?;
        let mut text = format!(
            "label chapter_{chapter}:\n    scene {}\n",
            serde_json::json!(path)
        );
        for line in 0..lines {
            let dialogue = if line % 25 == 0 {
                paragraph.clone()
            } else {
                format!(
                    "Chapter {chapter}, record {line}: a new signal reaches the rooftop receiver."
                )
            };
            let _ = writeln!(
                text,
                "    @id \"chapter.{chapter}.{line}\" {}",
                serde_json::json!(dialogue)
            );
        }
        let next = if chapter + 1 == chapters {
            "ending".to_owned()
        } else {
            format!("chapter_{}", chapter + 1)
        };
        let _ = write!(
            text,
            "    menu:\n        \"Follow the next signal\" id \"chapter.{chapter}.next\":\n            set trust = trust + 1\n            jump {next}\n        \"End the watch\" id \"chapter.{chapter}.end\":\n            jump ending\n"
        );
        fs::write(destination.join(format!("chapters/{chapter:03}.rns")), text)
            .map_err(|error| error.to_string())?;
    }
    script.push('\n');
    fs::write(destination.join("script.rns"), script).map_err(|error| error.to_string())?;
    let routes = serde_json::json!({"routes":[{"name":"full","choices":vec![0; chapters],"expect_label":"ending","expect_variables":{"trust":chapters}},
        {"name":"early","choices":[1],"expect_label":"ending","expect_variables":{"trust":0}}]});
    fs::write(
        destination.join("routes.json"),
        serde_json::to_vec_pretty(&routes).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    compile(destination)?;
    Ok(())
}

fn compile(path: &Path) -> Result<crate::Program, String> {
    ProjectSource::open(path)
        .map_err(|error| error.to_string())?
        .compile()
        .map_err(|errors| {
            errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        })
}

fn elapsed_ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}
