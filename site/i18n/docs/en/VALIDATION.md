# Scale and runtime validation

## Performance-optimization acceptance

After P0-P2 performance work, 134 ordinary Rust tests, a standalone FFmpeg
decode/seek test, 2 web sound-lifecycle tests, and 7 Playwright flows passed, plus
fmt and strict Clippy. Native smoke output is `target/perf-final-smoke`. Native
streaming video also had a window check. Latest numbers, budget definitions, and
feature bounds are in [Performance](PERFORMANCE.md).

## Workspace-split acceptance

On 2026-09-06 the same machine ran local verification after the workspace split:
125 Rust tests, fmt, and strict Clippy passed. VS Code 1.136.1 actually hosted the
extension and verified go-to-definition, disk indexing after closing a file, and
missing-asset diagnostics. The VSIX is about 465 KB and does not include a test
runtime.

Six Chromium regressions cover 1280x800 and 390x844 layout, save notes and restore,
settings, real breakpoints and stepping, route-coverage export, collection
persistence across restart, video frame advance, and overlay pause/load restore.
First playback blocked by the browser can resume audio on a player click. The story
graph had a canvas pixel check. Screenshots are in `web/test-results/`.

The native demo passed 17 capture checks. A long-text custom toggle/input project
passed all 18 capture checks and quick-load restore in release. Output is
`target/p2-native-demo/` and `target/p2-native-visual-release/`. The debug long-text
test exceeded 45 seconds at capture time and is not a performance baseline.

Release `--profile` uses an 800x600 window, mute, auto-forward, 60-frame warmup, and
300-frame sampling, and does not read screenshots:

| Project | p50 | p95 | p99 | Max frame | Sampled peak RSS | Texture peak |
| --- | --- | --- | --- | --- | --- | --- |
| demo | 8.29 ms | 9.36 ms | 16.65 ms | 18.08 ms | 711.5 MiB | 4.95 MiB |
| 10,000-line synthetic | 8.33 ms | 9.34 ms | 10.62 ms | 14.09 ms | 392.3 MiB | 1.98 MiB |

Raw reports are `target/p2-demo-frames.json` and `target/p2-medium-frames.json`.
This is a short-window frame-loop observation and did not play all 10,000 lines. RSS
includes fonts, audio, and the graphics backend, well above texture counts. Memory
attribution and real-art projects are still needed. There is no same-machine older
build to compare, so no speedup factor is claimed.

A local shipping directory and an unsigned `target/Signal-P2.app` were generated.
plist checks passed. Started from `/private/tmp` with no project argument, it passed
17 captures and save/load. The report is `target/p2-app-smoke/`. Steam config uses
`preview=1`. An itch launch config was generated. Notarization, store upload, and
remote CI were not submitted. Signed delta-patch tests cover rebuild, content
tamper, and a wrong public key.

The following is the historical baseline from before this round. Current features and
limits are in [Capability upgrades](UPGRADES.md).

## Measurement environment

Local measurement on 2026-09-06: Apple M3 Pro, 36 GiB RAM, macOS 26.5.2 (arm64),
Rust 1.98.0, Cargo release with thin LTO. Numbers establish a reproducible current
baseline. There is no same-machine older build, so they are not an overall speedup
claim.

```sh
cargo build --release --bins
target/release/renrs-bench generate target/benchmark-story 100 100
target/release/renrs-bench target/benchmark-story 5
```

The workload is 100 chapters and 10,000 generated dialogue lines, plus ending
dialogue, long text, and two branches per chapter: 10,805 instructions, 209 project
files. Images use independent paths but reuse two template illustrations. This is
synthetic. It does not stand in for real high-resolution art, a large music library,
or a complex condition graph.

## Release-mode results

| Operation | Local measurement |
| --- | --- |
| Directory check and compile, 5 runs | 83.79 / 56.22 / 74.74 / 57.15 / 56.64 ms |
| First-choice route, 10,102 interactions | 52.32 ms |
| Snapshot create and JSON | 5.62 ms, 5,029,836 bytes |
| Snapshot restore | 6.07 ms |
| Write one full save (checksum to disk) | 143.84 ms |
| First full read of six saves | 263.18 ms |
| Cached list, mean of 100 | 0.000448 ms/call |
| Watcher idle scan, mean of 10 | 0.758 ms/call |
| Directory archive pack | 72.23 ms, 3,292,092 bytes |
| Archive check and compile | 57.94 ms |

Raw local output is `target/validation/benchmark.json`. Compile measurements include
asset and support-file checks. The first call did not drop the OS file cache. Run
measurements are headless Runtime only: no GPU, image decode, audio decode, frame
loop, or per-character animation. Snapshots include full history and up to 256
rollback points.

That is the historical baseline. The current player reads and writes saves through a
background storage queue, bounds image-decode queue bytes, and uses streaming audio
instead of a fully decoded cache. Latest frame-interval and RSS observations are in
the performance notes. `renrs-bench` CLI times still include synchronous disk write
and are not the player main-thread stall.

`renrs-bench` also accepts an archive. Then `pack_ms` is archive copy and
`watcher_idle_ms` is 0. The generator allows 1–200 chapters and 1–1000 lines each.
The measurer advances at most 100,000 interactions. Oversized or looping projects
error. The generate target must not exist.

## Native window acceptance

```sh
cargo run --example generate_visual_fixture -- target/visual-story
cargo run --bin renrs-debug -- test target/visual-story target/visual-story/routes.json
cargo run -- demo --smoke-test target/captures-demo --window-size 1280x720
cargo run -- target/visual-story --smoke-test target/captures-visual --window-size 800x600
```

The test output directory must not exist. The mode uses `data/` under the output
directory, mutes all three audio channels, drives Macroquad from real App state and
keyboard actions, and reads the framebuffer after the current image is available.
Each capture must have at least 32 sampled colors. Run errors, asset warnings,
snapshot-restore mismatch, or timeout fail. Success writes `report.json` and PNGs,
then closes the window. Existing player settings and saves are not modified.

At most 14 states: title, dialogue, second page, first/last choices, history ends,
settings, language, translated dialogue, manual/quick/auto saves, and quick load.
Projects without long dialogue skip the second page explicitly. A project used in
this mode must show dialogue and a later choice within 2,000 advances. Total limit
45 seconds.

This round covered the bundled demo and a reproducible custom-screen project at
1280x720 and 800x600. The custom project has six dialogue pages, 36 extra history
lines, 12 long choices, a Chinese catalog, five preference sliders, and every
replaceable screen. Captures confirm long text and controls do not overlap, Chinese
and images display, and a small window keeps letterboxing. On a local Retina
display, PNG pixels are twice the requested window size.

A shipping directory contains the player, game archive, manifest, and two licenses.
Started from `/private/tmp` with no project path, both debug and release players
finished captures and quick-load restore, verifying `game.renrs` lookup next to the
player. Captures and reports are in `target/validation/`, grouped as `demo-*`,
`visual-*`, and `package-*`.

## Regression and delivery

- Local fmt, all-target check, strict Clippy, and 109 automated tests passed. Every Rust file in `src/` and `tests/` is under 500 lines.
- Tests cover old snapshots/containers, stable ID aliases, rollback, directory/archive consistency, corrupt saves, the migrator, language transactions, and route assertions.
- The VS Code extension passed JavaScript syntax checks and regenerated a VSIX with LSP deps, grammar, and license.
- CI configures three-desktop Rust checks, template and route tests, VSIX pack, Linux Xvfb/Mesa captures, and shipping startup.

This historical stage did not actually run remote three-platform CI, VS Code
extension-host acceptance, or human audio listening. Window automated acceptance
does not simulate full OS mouse events and does not replace real usability tests.
Font families/RTL, background audio decode, save screenshots, exact dialogue page
restore, mobile, code signing, and notarization were still unimplemented at that
stage.
