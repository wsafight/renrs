# Native Font Memory

The 2026-09-06 native font follow-up replaces eager project-font outline loading
with demand-driven glyph rasterization. Full Chinese coverage is retained; the
font file is unchanged and is not subsetted.

## Cause and Change

The previous native path loaded NotoSansSC through macroquad's fontdue backend.
fontdue 0.9.4 expands mapped glyph outlines when constructing a font. An isolated
probe using the actual release dependency measured RSS at 1.56 MiB before loading,
11.81 MiB after reading the 10,589,136-byte font and 254.14 MiB after parsing its
30,890 mapped characters. This identified a large allocation source, rather than
attributing process memory to the implementation language.

The new path in `src/player/text.rs` uses ab_glyph 0.2.32 for font parsing and
rasterization, and etagere 0.2.15 for atlas allocation:

- Width measurement reads glyph advances and kerning without allocating glyph
  outlines, bitmaps or a GPU atlas. Only glyphs actually drawn are rasterized.
- Project font bytes are moved into an owned face, with a 32 MiB input limit.
  The bundled fallback borrows static bytes. The entire font remains available.
- A single 1024x1024 RGBA atlas uses 4 MiB, allocated on first glyph draw. The
  glyph lookup table holds at most 4096 entries. Bitmaps are temporary and are
  released after upload; there is no second persistent CPU copy of the atlas.
- The atlas is recycled when full. Pending draws are flushed through the safe
  camera API before their texels are reused, keeping the active render target.
  Recycling retains the same texture instead of accumulating retired textures.
- Unusually large glyphs are downsampled to at most 1020x1020 pixels and drawn
  at their original scale. Temporary bitmap/rasterizer buffers are separate
  from the 4 MiB atlas limit.
- Font reload parses and validates before replacing state. Cache entries are
  invalidated after a successful swap; invalid fonts leave the runtime intact.
  Textures are released before the native graphics context shuts down.

All native UI, dialogue, ruby, underline and history text use this path. Existing
layout, Unicode reveal and save behavior remain in place. Macroquad still has
its small built-in font internally; project CJK fonts no longer use that path.
Web continues to use browser text rendering and is outside this native change.

## Measurements

Same Apple M3 Pro, macOS 26.5.2, assets, font, fixtures and sampling method as
[the earlier comparison](PERFORMANCE_COMPARISON.md). Each row is a median of
three runs; figures are peak sampled process RSS in the measurement window.
Before values are the preserved earlier runs, not interleaved with this follow-up.

| Scenario | RenRS before | RenRS after | Reduction | Ren'Py this run |
| --- | ---: | ---: | ---: | ---: |
| Idle dialogue | 359.3 MiB | 127.5 MiB | 64.5% | 239.7 MiB |
| Moving character | 353.5 MiB | 127.2 MiB | 64.0% | 237.0 MiB |

After-change RSS ranges were 126.8-127.9 MiB idle and 127.0-127.2 MiB in motion.
Ren'Py ranges were 238.6-241.6 MiB idle and 180.1-238.8 MiB in motion; the lower
motion run is retained. These figures describe this small native workload, not
all projects, total GPU memory or cross-platform behavior.

Idle RenRS rasterized 66 glyph bitmaps and retained 68 lookup entries including
empty glyphs; motion rasterized 41 and retained 42 entries. Both used one 4 MiB
atlas. The report's single reset is the initial font install, not capacity churn.
The earlier scene image textures still account for 3,115,200 bytes separately.

CPU medians were 2.43% idle and 7.98% in motion for RenRS, versus 2.83% and 12.39%
for Ren'Py (100% means one logical core). Idle CPU varied from 1.71% to 4.38% for
RenRS; this iteration does not establish a CPU improvement. RenRS drew 0 scenes
while idle and 525/481/480 during the three approximately eight-second motion
windows. Ren'Py drew 957/960/958 in motion. The differing update rates, toolbar
and internal raster resolution remain limitations of cross-engine comparisons.

All twelve runs passed nonblank and motion-pixel checks. Captures occur after
the CPU/RSS measurement window. New artifacts are in:

```text
target/engine-comparison-verified/results-1788697453495/
```

Binary SHA-256 values:

```text
before: 39ed6c6c1f99abe3cceb6e92f0ed021c94a2fcdf1f8ee7d82f797b8da37a45a8
after:  f6237c2f7c4ae7fda7ce5acedea14fb6e2ed8ca97b4940e3181e8d3c594355f2
```

The earlier player is retained at `target/renrs-before-lazy-fonts`. The workspace
has no committed baseline; these identify binaries, not reproducible git revisions.

## Verification

- Full Rust workspace/all-target tests, formatting and strict Clippy pass.
- Tests cover CJK em sizing, nonblank glyph rasterization, spaces, invalid fonts,
  oversized glyph bounds and measuring thousands of unseen CJK characters
  without populating a glyph cache. Reload tests reject a bad font transactionally.
- The native atlas stress example forces nine resets, checking identical pixels
  before/after reuse and font replacement. It passed on both the default drawable
  and the offscreen camera path used by the player. Latest artifacts:
  `target/text-cache-canvas-verified/`.
- Product and NVL/ruby native smoke runs passed, including Chinese UI and save/load,
  with 17 nonblank captures each. Artifacts: `target/smoke-lazy-font-product/` and
  `target/smoke-lazy-font-reading/`. Chinese and ruby screenshots were inspected.

Reproduce from the repository root with an interactive desktop and the earlier
comparison fixtures/official SDK available:

```sh
cargo build --offline --release --bin renrs --example text_cache_smoke --example inspect_frames
target/release/examples/text_cache_smoke target/my-text-cache-check
node scripts/compare-engines.mjs target/engine-comparison-verified target/renpy-sdk/renpy-8.5.3-sdk 3
```

The renderer remains a simple glyph renderer, not a full text-shaping engine.
RTL, complex-script shaping and color-emoji support are not added here. Atlas
recycling can add rasterization work for screens with many unique large glyphs;
the budget remains fixed. Physical-device and Windows/Linux acceptance remain open.
