# Performance work

Implementation order and verification log for the P0-P2 performance review.

The latest iteration is recorded in [the Ren'Py comparison](PERFORMANCE_COMPARISON.md):
native scene caching, Web reading behavior, shared rollback state and repeated
same-asset measurements. The figures below are historical results from the earlier
iteration, not the latest build's benchmark or test totals.

The [native font follow-up](FONT_MEMORY.md) replaces eager CJK outline loading
with on-demand glyphs and a fixed 4 MiB atlas, reducing measured native RSS by
about 65% in the comparison fixtures.

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
