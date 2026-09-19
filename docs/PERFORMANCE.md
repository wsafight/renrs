# Performance work

Implementation order and verification log for the P0-P2 performance review.

The latest iteration is recorded in [the Ren'Py comparison](PERFORMANCE_COMPARISON.md):
native scene caching, Web reading behavior, shared rollback state and repeated
same-asset measurements. The figures below are historical results from the earlier
iteration, not the latest build's benchmark or test totals.

The [native font follow-up](FONT_MEMORY.md) replaces eager CJK outline loading
with on-demand glyphs and a fixed 4 MiB atlas, reducing measured native RSS by
about 65% in the comparison fixtures.

## Velin 0.4 Extension Boundary

Measured on 2026-09-16 on an Apple M3 Pro, arm64, macOS 26.6.2, with
`rustc 1.98.0` and Criterion 0.5.1. The stored Rhai 1.26.0 baseline was
collected on the same machine on 2026-09-15 at RenRS `fe4b03c`; the Velin
measurement uses the final 0.4.0 implementation based on `45af585`. Both use
the same benchmark source, inputs, result assertions, release profile,
three-second warmup and 100 samples. The runs were not interleaved, so transient
system load can still move a repeat measurement slightly.

Times are Criterion 95% confidence intervals with the point estimate in bold.
Changes come directly from Criterion's saved `rhai` distribution; all seven
have `p < 0.05`.

| Benchmark | Rhai 1.26.0 | Velin 0.4.0 | Time change | Point-estimate speedup |
| --- | ---: | ---: | ---: | ---: |
| `extensions/compile/scalar` | 42.699 - **43.028** - 43.571 us | 3.2349 - **3.2584** - 3.2988 us | **-92.449%** `[-92.556%, -92.366%]` | 13.20x |
| `extensions/compile/suite_4` | 48.993 - **49.274** - 49.674 us | 17.083 - **17.186** - 17.304 us | **-65.568%** `[-66.062%, -65.124%]` | 2.87x |
| `extensions/invoke/scalar_arithmetic` | 291.16 - **295.63** - 301.12 ns | 98.040 - **98.912** - 100.43 ns | **-66.845%** `[-67.576%, -66.252%]` | 2.99x |
| `extensions/invoke/list_identity_1024` | 31.949 - **32.299** - 32.757 us | 2.9364 - **2.9518** - 2.9816 us | **-90.857%** `[-90.936%, -90.778%]` | 10.94x |
| `extensions/invoke/record_identity_512` | 184.46 - **186.07** - 188.31 us | 4.3740 - **4.3840** - 4.3938 us | **-97.657%** `[-97.675%, -97.642%]` | 42.44x |
| `extensions/invoke/list_append_1024` | 38.876 - **39.112** - 39.465 us | 4.1854 - **4.2086** - 4.2544 us | **-89.270%** `[-89.363%, -89.181%]` | 9.29x |
| `extensions/invoke/sum_loop_256` | 17.633 - **17.744** - 17.917 us | 16.267 - **16.290** - 16.313 us | **-8.3338%** `[-9.1934%, -7.6869%]` | 1.09x |

The identity cases isolate dynamic-value conversion and collection boundaries.
Sharing Velin's `Value` representation removes the recursive
`rhai::Dynamic` conversion, producing the largest gains for lists and
records. A bounded reusable `MachineInvoker` pool also reduces short scalar
calls from about 296 ns to 99 ns. The loop improves by only 8.3%, so
compute-heavy scripts should not expect the order-of-magnitude gain seen at
structured-data boundaries. Compilation happens only while creating or
restoring a runtime and is not frame-time throughput.

Reproduce from the repository root:

```sh
cargo bench -p renrs-extensions --bench extensions -- --save-baseline rhai
cargo bench -p renrs-extensions --bench extensions -- --baseline rhai
```

The saved distributions live under
`target/criterion/extensions_*/*/rhai/` and are removed by `cargo clean`.
The workload covers scalar compilation, a four-module suite, scalar invocation,
1024-item list identity and append, 512-field record identity, and summing the
integers 0 through 255.

Upgrade verification passed the full workspace/all-target test suite, including
repeated invocation, failure reuse, constant modules and eight-way concurrent
state isolation. The Web crate builds for `wasm32-unknown-unknown`; affected
libraries pass Clippy with `-D warnings`, and formatting and diff checks pass.
The recorded benchmark build's dependency tree contained only Velin 0.4.0 packages and no Rhai package.

## Story Expression Migration

The 2026-09-15 migration measurement compared RenRS's former expression parser
and evaluator with the shared Velin expression path. It predates the dependency
bump to 0.4.0. These figures are retained as migration history, not presented
as a fresh 0.4.0 or 0.5.1 run.

The expression was:

```text
trust + 2 * 3 >= 6 and contains(list("signal", "key"), "key")
```

| Benchmark | Former RenRS implementation | Shared Velin expression path | Criterion change |
| --- | ---: | ---: | ---: |
| `parse_story_expression` | 1.626 us | 2.172 us | +34.41% |
| `evaluate_story_expression` | 310.62 ns | 273.46 ns | -6.28% |

Parsing now includes conservative static checking and precise diagnostics, so
the 0.55 us increase is not an equal-feature parser comparison. The cost occurs
while loading, hot reloading or submitting an interactive expression, not on
every rendered frame.

The generated 100-chapter, 100-line-per-chapter scenario showed no statistically
significant end-to-end change:

| Benchmark | Before shared expressions | After migration | Point change | Criterion |
| --- | ---: | ---: | ---: | --- |
| `parse_script_chapter` | 124.48 us | 125.87 us | +2.08% | No significant change |
| `compile_generated_project` | 27.666 ms | 28.013 ms | +1.25% | No significant change |
| `runtime_play_generated_route` | 29.320 ms | 29.909 ms | +2.01% | No significant change |

The expression migration therefore removed duplicate implementation and unified
semantics; it was not an end-to-end performance optimization.

## Runtime Optimization History

- [x] P0: save-only 240x135 GPU thumbnails; encode on the storage worker.
- [x] P0: compact Web state, paginated history, on-demand debugger/profile data.
- [x] P0: bounded Web sound channels and deterministic media cleanup.
- [x] P1: owned save payloads and shared immutable compiled programs.
- [x] P1: byte-accounted decode queues, visible collection/save previews.
- [x] P1: revision-based profile updates and allocation-free dialogue reveal.
- [x] P2: background file watching/compilation with incremental parsing.
- [x] P2: bounded streaming playback for long audio/video.

Baseline (release, medium fixture, 10,805 instructions): compile 44.139-77.095 ms;
10,102 interactions 43.348 ms; snapshot+JSON 4.355 ms / 5,029,836 bytes;
restore 4.160 ms; durable save 50.586 ms; watch scan 1.083 ms.
Frame intervals include display waiting and must not be interpreted as CPU time.

P0 verification: workspace tests, two audio lifecycle tests, six Playwright
flows (desktop/mobile), native smoke including save-time thumbnail inspection.
Readback is now 129,600 bytes per explicit save instead of 3,686,400 bytes per
ordinary click (96.5% smaller when a capture is needed). Autosaves skip capture.
The entry JS bundle is 23.90 kB; the 446.92 kB debugger loads only when opened.

P1 intermediate release measurements on the same medium fixture: snapshot+JSON
3.299 ms (baseline 4.355), restore 2.782 ms (4.160), durable save 45.630 ms
(50.586). Snapshot JSON remains 5,029,836 bytes. These are individual runs,
not a statistical claim about frame rate or disk latency.

Final standalone medium-fixture run (macOS arm64, release): compile
65.913 / 44.202 / 47.541 ms, 10,102 interactions 43.630 ms, snapshot+JSON
3.450 ms, restore 2.596 ms, durable save 45.280 ms, cached listing 0.000357 ms.
A concurrent GUI/benchmark run showed large transient latency and was excluded
from this comparison; full compile and interpreter speed are essentially unchanged.

Final native observations (800x600, 60 warmup + 300 measured frames):

| Project | p50 / p95 / p99 (ms) | Max (ms) | Peak process RSS (bytes) |
| --- | --- | --- | --- |
| Demo before | 8.295 / 9.362 / 16.646 | 18.076 | 746,012,672 |
| Demo after | 8.319 / 9.293 / 10.216 | 11.782 | 389,545,984 |
| Medium before | 8.332 / 9.344 / 10.624 | 14.094 | 411,402,240 |
| Medium after | 8.310 / 9.371 / 12.811 | 19.131 | 377,012,224 |

RSS fell in these runs, but frame intervals do not show a consistent improvement.
The medium route's tail latency needs repeated profiling before attributing a
change to engine CPU work. Reports: `target/perf-final-*-frames.json`.
Native streaming playback was also rendered and inspected; its separate report
is `target/perf-final-stream-check.json`. Process RSS excludes the FFmpeg child.

Implementation details and limits:

- Runtime and player share `Arc<Program>`. Snapshots share history until a
  writer changes it. Runtime, snapshots and the bounded 256 rollback entries now
  share variable maps and stage state; nested lists and records also use
  copy-on-write sharing. JSON serialization retains checkpoint metadata while
  values and repeated stages are interned.
- Storage accepts owned snapshots and interns stage state once before hashing and
  encoding. File replacement and checksums remain validated. Loading accepts only
  the current container v2 and snapshot v7/v8. A changed compiled-script fingerprint
  is accepted only when active positions resolve through explicit IDs or aliases;
  snapshots older than v7 and save migration are not supported.
- GPU story textures have a 256 MiB cap. Image workers separately reserve up
  to 256 MiB for encoded inputs, decoder scratch and queued RGBA buffers.
  Encoded images are limited to 32 MiB. Oversized images report a load error.
  These application budgets do not include fonts, drivers or codec internals.
- Gallery and save preview textures are prepared for the visible page;
  dialogue reveal uses cached Unicode boundaries. Prefetch hints update only
  when the execution position changes. Profile clones/writes use revisions.
- `notify` triggers background scans with a 5-second polling fallback; parse
  results are reused by source hash, with full merged-project validation.
  Compilation and resource reads stay off the renderer. Transactional editor
  session remapping and font-face installation still occur during the validated
  result swap; glyph uploads happen only when text is drawn. Session remapping
  is not a save-upgrade API.
- Rodio streams WAV/Ogg through seekable 64 KiB buffered readers. Archive
  checksums are verified on the worker with a fixed buffer. There is no full
  decoded audio cache, and simultaneous effects are capped at 16 channels.
- Streaming video v2 uses MP4/WebM via FFmpeg natively and HTML video on Web.
  Native output is limited to 1920x1080, with two queued frames, one in-flight
  frame, one pending frame and one reusable GPU texture. FFmpeg's own process
  memory is separate. Archive video uses a temporary file, removed on close.
- Video supports legacy optional WAV audio or bounded localized WAV audio/subtitle
  tracks synchronized with playback, pause, seek and save restoration. Per-track gain
  multiplies the sound-channel volume. Native streaming requires `ffmpeg` or
  `RENRS_FFMPEG`. PNG-frame v1 clips remain supported without a runtime FFmpeg
  dependency.

Create a streaming clip with:

```sh
renrs-video input.mp4 my-project clips/intro --stream
```

The conversion also requires `ffprobe` or `RENRS_FFPROBE`. The command prints
the exact script statement and duration. Web deployments must serve media
with the correct content type and byte-range support (the preview server does).

Verification includes 134 passing workspace tests and strict Clippy,
Web lifecycle tests, seven desktop/mobile browser flows with canvas-pixel
checks, native save-preview smoke (`target/perf-final-smoke`), and one explicitly
run FFmpeg seek/decode test (135 Rust tests total; the FFmpeg case is ignored by
the ordinary suite because it requires an external executable):

```sh
cargo test --bin renrs player::video_worker::tests::streams_frames_from_a_saved_offset -- --ignored
```
