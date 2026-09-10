# Reading, Rendering and Rollback Optimization

Measured on 2026-09-06. This is the P0-P2 iteration following the Ren'Py
comparison. Historical results in [PERFORMANCE.md](PERFORMANCE.md) are separate
measurements. The project is pre-release: current saves require container v2 and
snapshot v7. The current code writes snapshot v8 and remains compatible with v7.
Content updates restore only when active positions resolve through explicit IDs or
aliases; this does not migrate an older snapshot format.

The subsequent [native font memory optimization](FONT_MEMORY.md) reduces the
measured RenRS idle RSS from 359.3 to 127.5 MiB. The native tables below describe
the earlier font implementation; the follow-up contains current memory results.

## Completed Work

- [x] P0: cache the native scene and invalidate it for input, story changes,
  animation, asset completion, reload and storage responses. A static scene is
  not redrawn on ordinary housekeeping wakes. Save-only thumbnails remain on
  the storage worker path.
- [x] P0: block the macOS event loop between wakes. A worker timer requests an
  update and posts a Cocoa event through safe objc2 APIs. Animation uses a
  16.667 ms cadence; housekeeping uses 100 ms. Absolute deadlines prevent timer
  oversleep from accumulating; long stalls reset the cadence without bursts.
- [x] P0: batch Web read-history writes every two seconds, deduplicate IDs and
  flush on save, backgrounding and page exit. Storage failures retain dirty
  state for retry and show a notice.
- [x] P0: Web Unicode typewriter, complete-before-advance, modal/background
  pause, saved reveal progress, post-reveal auto delay and skip-read behavior.
  Rich text and ruby reserve layout; existing NVL text stays visible.
- [x] P1: cache flattened custom-screen layouts and visible save lists;
  rebuild/invalidate after reload or slot changes. Rendering retains shared
  stage state instead of cloning it per draw.
- [x] P1: predict resources across jumps, conditional alternatives, choices,
  calls and returns without executing script expressions. Traversal is bounded
  to 256 positions, 16 call frames and 16 unique images.
- [x] P1: share variable maps, stage state and nested lists/records across
  runtime, snapshots and rollback checkpoints. Mutations use copy-on-write;
  failed screen edits restore the prior state transactionally.
- [x] P2: add repeatable Ren'Py/RenRS fixtures, native measurement mode, external
  process sampling, screenshot checks and a preserved-binary state comparison.

Scene caching works on all native targets, but blocking event-loop wakeup is
enabled and verified only on macOS. Windows/Linux keep their ordinary event
loop. Even on macOS, housekeeping/input wakes still submit the cached texture;
zero scene redraws does not mean zero GPU work or zero CPU activity.

## Method

Hardware: Apple M3 Pro, arm64, macOS 26.5.2, 120 Hz built-in display. The official
SDK actually executed is Ren'Py 8.5.3.26051504, from `renpy-8.5.3-sdk.zip`.
Its SHA-256 was checked against the [official checksums](https://www.renpy.org/dl/8.5.3/checksums.txt):

```text
ff57648f9c04f27e381c48af6d8e3ee3cdec296bed4d3831f47f09b0a71b505e
```

The local `references/renpy` source checkout is not the measured SDK version.
RenRS is a release build. Output records its binary SHA-256; the workspace has
no committed baseline, so these are binary-identified local measurements.

Both fixtures use identical background, character and NotoSansSC font files,
with asset hashes in `manifest.json`. The logical/window size is 1280x720 and
Retina captures are 2560x1440. A 440x650 character starts at x=84. Idle holds
the same fully revealed dialogue; motion moves to x=484 linearly over 30 seconds.
There is no audio. Each run warms up for two seconds after resource readiness,
then measures for eight seconds. Each scenario runs three times with alternating
engine order. Engines run sequentially, without builds or other test runs.

CPU is the external `ps` cumulative process CPU delta divided by sampled wall
time; 100% is one logical core. RSS is the highest observed process sample in
that window, not GPU memory or total system allocation. Polling requests a
250 ms interval; actual sample timestamps are retained. First/last samples lie
inside the eight-second window, so sampled duration is shorter. Samples crossing
report creation are discarded. Screenshots and PNG encoding happen after the
measurement report, followed by a second screenshot 0.5 seconds later.
The checker requires at least 32 sampled colors and 500 changed pixels for motion.

These are comparable assets and story scenarios, not identical render workloads:

- RenRS renders a 1280x720 scene target then scales it to the Retina drawable;
  Ren'Py renders to a 2560x1440 drawable. Text rasterization therefore differs.
- RenRS retains its normal toolbar; the Ren'Py fixture has a minimal say screen.
- Both request 60 Hz, but the macOS GL swap interval request does not reliably
  cap Ren'Py on this display. Actual draw rates are reported. RenRS wake counts
  and Ren'Py draw calls are different instrumentation points, not interchangeable
  CPU timings or end-to-end displayed-frame measurements.
- Three short runs do not establish cold-start, battery, GPU, large-commercial-
  project or cross-platform superiority. Readiness includes warmup and is not a
  cold-start benchmark.

## Native Results

Three-run medians, with full observed ranges in parentheses. All twelve runs
passed nonblank checks; idle verification frames were identical, and every
motion pair changed more than 112,000 pixels. Process samples covered
7.68-7.80 seconds per run.

| Scenario / engine | CPU (% of one core) | Peak sampled RSS (MiB) | Scene/draw calls in about 8 s |
| --- | ---: | ---: | ---: |
| Idle RenRS | 1.30 (1.29-1.30) | 359.3 (359.3-700.7) | 0 |
| Idle Ren'Py | 2.06 (1.95-2.20) | 241.2 (239.2-242.1) | 50 (49-50) |
| Motion RenRS | 7.61 (7.54-7.64) | 353.5 (353.4-353.6) | 481 (481-482) |
| Motion Ren'Py | 12.46 (12.19-13.71) | 237.2 (235.9-243.8) | 960 (952-960) |

RenRS avoids scene redraws while idle and sustains approximately 60 scene
updates/second during motion. Its motion wake p50 was 16.48-16.63 ms and p95
19.10-20.02 ms. Ren'Py actually drew about 119-120 times/second despite its
60 Hz request. This CPU table cannot establish rendering-efficiency or FPS
superiority because both the draw rate and internal raster resolution differ.

RenRS uses more process memory in both scenarios. One idle run had a 700.7 MiB
peak already present when sampling began; this outlier is retained, and its cause
has not been isolated. Two-second warmup does not guarantee settled allocation
behavior. The larger peak is not a screenshot-allocation sample, since captures
begin only after the report and boundary-crossing samples are discarded.

Artifacts: `target/engine-comparison-verified/results-1788694206504/summary.json`,
individual `*.process.json` samples and paired `*.start.png` / `*.end.png` files.
The screenshot names refer to post-measurement verification, not window bounds.
Earlier trial directories are excluded: they used prior timer scheduling or
sampling logic, including runs where screenshot allocation crossed the boundary.

```text
RenRS player SHA-256:
39ed6c6c1f99abe3cceb6e92f0ed021c94a2fcdf1f8ee7d82f797b8da37a45a8
```

## Shared-State Results

This compares RenRS before and after P1, not Ren'Py. The fixture builds a nested
inventory of 16 lists with 64 roughly 128-byte strings each, changes one scalar
on every dialogue and fills the 256-entry rollback window. Both binaries execute
818 instructions / 401 interactions. Results are medians of three separate runs,
alternating binary order; the command's `3` argument repeats compilation only.

| Operation | Before | After | Observation |
| --- | ---: | ---: | --- |
| Execute the route | 11.575 ms | 0.905 ms | 92.2% less time |
| Snapshot plus JSON serialization | 23.919 ms | 16.351 ms | 31.6% less time |
| Restore from an in-memory snapshot | 11.610 ms | 0.281 ms | 97.6% less time |
| Durable save | 174.763 ms | 157.905 ms | 9.6% less in these runs; disk timing varies |
| Uncached listing of six saves | 2384.392 ms | 2393.155 ms | No improvement |
| Snapshot JSON | 34,773,179 bytes | 34,773,179 bytes | No size reduction |

Restore timing excludes disk read and JSON parsing. Deserialization does not
reconstruct shared identities across checkpoints, and serialized JSON still
duplicates their contents. Large uncached save listings remain expensive; cached
visible lists reduce repeated work but do not solve the first full read.

Artifacts: `target/engine-comparison-verified/state-results-1788694170096/`.

```text
before: b6440b722b51d31de08590859a29f8385f519531157b0f23abe646a672b501a1
after:  7f50d976d33928df1ff847c0468a598306cbb47079b4c6cffb5c065eaa813687
```

## Reproduce

From the repository root, with the official SDK unpacked at the path below:

```sh
cargo build --offline --release --bin renrs-bench --example inspect_frames
cargo build --offline --release -p renrs-player --bin renrs
node scripts/generate-engine-comparison.mjs target/my-engine-comparison
node scripts/compare-engines.mjs target/my-engine-comparison target/renpy-sdk/renpy-8.5.3-sdk 3
node scripts/compare-state.mjs target/my-engine-comparison
```

Use a new fixture directory. Native measurements need an interactive desktop
session; avoid input to the benchmark windows. The process sampler uses POSIX
`ps` and was verified on macOS. State comparison additionally requires the
preserved `target/renrs-bench-before-p1` executable; it cannot be reconstructed
from a baseline commit in this currently untracked workspace. Each result
directory includes individual runs and a summary, with raw process samples and
verification screenshots for native measurements.

## Verification

- Full Rust workspace/all-target tests, source-size check, formatting and strict
  Clippy (`-D warnings`) pass. Coverage includes COW isolation, transactional
  failures, prediction traversal and timer drift/stall behavior.
- The normally ignored FFmpeg seek/decode test was explicitly run and passed.
- Four Web unit tests and 14 distinct Chromium workflows pass: 12 general
  desktop/mobile flows plus the separately enabled PNG/streaming media tests.
  These cover reading progress, auto delay, portable saves, Unicode/ruby,
  custom screens and synchronized video. Background reading is exercised via
  the visibility handler; physical mobile lifecycle acceptance remains open.
- Final native custom-screen/save/load smoke passes with 17 nonblank captures:
  `target/smoke-optimization-cadence/report.json`. Its screenshot-heavy frame
  timings and RSS are not used as performance evidence.
- Final Web bundle: entry JS 40.08 kB; debugger 446.94 kB loaded on demand.
  Preview: http://127.0.0.1:4183/ with `/product/` and `/reading/` fixtures.

Physical-device acceptance, remote platform CI and owner-supplied production
projects remain open in [PRODUCT_UPGRADES.md](PRODUCT_UPGRADES.md).
