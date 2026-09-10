//! Microbenchmarks for parser, compiler, runtime, snapshot, checksum, and markup.
//!
//! Full-project scale uses the same generator as `renrs-bench` (100 chapters,
//! 100 lines). Run with `cargo bench --bench engine`. These do not replace the
//! release-mode `renrs-bench` acceptance report.

use std::hint::black_box;
use std::sync::{Arc, LazyLock};

use criterion::{Criterion, criterion_group, criterion_main};
use renrs::runtime::RuntimeSnapshot;
use renrs::save_format::{SaveFile, checksum};
use renrs::syntax::Script;
use renrs::text::parse_text_markup;
use renrs::{Program, Runtime, WaitState, compile, load_project, parse_script};

const MARKUP: &str = "{b}Chapter {n}{/b}{br}{color=#ff0080}signal{/color}{w=0.5} {ruby=note}word{/ruby}{p=0.25}next{nw}";

struct Fixture {
    chapter_source: String,
    script: Script,
    program: Arc<Program>,
    snapshot: RuntimeSnapshot,
    save: SaveFile,
}

static FIXTURE: LazyLock<Fixture> = LazyLock::new(build_fixture);

fn build_fixture() -> Fixture {
    let root = tempfile::tempdir().expect("bench fixture directory");
    let game = root.path().join("game");
    renrs::benchmark::generate(&game, 100, 100).expect("generate renrs-bench workload");
    let chapter_source = format!(
        "label start:\n    return\n{}",
        std::fs::read_to_string(game.join("chapters/000.rns")).expect("generated chapter source")
    );
    let script = load_project(&game).expect("load generated project");
    let program = Arc::new(compile(&script).expect("compile generated project"));
    let mut runtime = Runtime::new(Arc::clone(&program)).expect("start generated project");
    play(&mut runtime);
    let snapshot = runtime.snapshot();
    let mut save = SaveFile {
        container_version: SaveFile::CONTAINER_VERSION,
        engine_version: env!("CARGO_PKG_VERSION").to_owned(),
        saved_at_unix: 1,
        project_id: program.project_id.clone(),
        content_version: program.fingerprint.clone(),
        play_time_seconds: 0,
        chapter: Some("ending".to_owned()),
        snapshot: snapshot.clone(),
        checksum_sha256: String::new(),
        presentation: None,
    };
    save.checksum_sha256 = checksum(&save).expect("checksum generated save");
    drop(root);
    Fixture {
        chapter_source,
        script,
        program,
        snapshot,
        save,
    }
}

fn play(runtime: &mut Runtime) {
    let mut state = runtime.advance().expect("generated route should start");
    let mut steps = 0;
    while state != WaitState::Finished {
        steps += 1;
        assert!(
            steps < 100_000,
            "generated route exceeded 100000 interactions"
        );
        state = if matches!(state, WaitState::Choice { .. }) {
            runtime.choose(0)
        } else {
            runtime.continue_story()
        }
        .expect("generated route should execute");
        runtime.drain_audio_events().for_each(drop);
    }
}

fn parse_and_compile(criterion: &mut Criterion) {
    let fixture = &*FIXTURE;
    criterion.bench_function("parse_script_chapter", |bencher| {
        bencher.iter(|| {
            parse_script(
                black_box(fixture.chapter_source.as_str()),
                "chapters/000.rns",
            )
            .expect("chapter should parse")
        });
    });
    criterion.bench_function("compile_generated_project", |bencher| {
        bencher.iter(|| compile(black_box(&fixture.script)).expect("project should compile"));
    });
}

fn play_route(criterion: &mut Criterion) {
    let program = Arc::clone(&FIXTURE.program);
    criterion.bench_function("runtime_play_generated_route", |bencher| {
        bencher.iter(|| {
            let mut runtime =
                Runtime::new(Arc::clone(black_box(&program))).expect("runtime should start");
            play(&mut runtime);
            black_box(runtime.history().len())
        });
    });
}

fn persist(criterion: &mut Criterion) {
    let fixture = &*FIXTURE;
    criterion.bench_function("snapshot_serialize_json", |bencher| {
        bencher.iter(|| {
            serde_json::to_vec(black_box(&fixture.snapshot)).expect("snapshot should serialize")
        });
    });
    criterion.bench_function("snapshot_restore", |bencher| {
        bencher.iter(|| {
            Runtime::restore(
                Arc::clone(&fixture.program),
                black_box(fixture.snapshot.clone()),
            )
            .expect("snapshot should restore")
        });
    });
    criterion.bench_function("save_checksum", |bencher| {
        bencher.iter(|| checksum(black_box(&fixture.save)).expect("save should checksum"));
    });
}

fn markup(criterion: &mut Criterion) {
    let sample = MARKUP.repeat(40);
    criterion.bench_function("parse_text_markup", |bencher| {
        bencher.iter(|| parse_text_markup(black_box(&sample)).expect("markup should parse"));
    });
}

criterion_group!(benches, parse_and_compile, play_route, persist, markup);
criterion_main!(benches);
