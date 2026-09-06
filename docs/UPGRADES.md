# P0-P2 Upgrade Guide

The implementation checklist is in [IMPLEMENTATION.md](IMPLEMENTATION.md).
Cargo boundaries are in [ARCHITECTURE.md](ARCHITECTURE.md).

## Current Saves And Resources

Runtime snapshots use version 4 inside version 2 save containers. Other formats
and different compiled-script fingerprints are rejected. The project is pre-release;
old save migration and backward compatibility are not required. Current saves
retain call-site IDs, runtime state and rollback history with checksum validation.
Editor hot reload separately remaps a live session using explicit `@id` anchors
and initializes newly declared defaults. See [current policy](PRODUCT_UPGRADES.md).

`resources.json` accepts `include` and `exclude` lists of exact relative names
or directory prefixes ending in `/`. These are not globs. Hidden path components,
build/cache directories, temporary files and symbolic links are excluded.
The same rules govern directory loading, archives, watching and editor indexing.

## Desktop Player

The player offers 60 manual slots, three quick saves and five rotating autosaves.
Overwrite, load, delete and quit require confirmation. Quit writes a resume save;
dirty progress is also saved periodically and at chapter/choice/end boundaries.
Save operations use a bounded background queue and same-directory atomic writes.
Slot summaries cache metadata, notes and thumbnails. Invalid saves remain visible.

The optional save presentation records dialogue page, reveal position and remaining
pause/effect time. Import/export uses a path field in the save tools overlay.
Rodio streams WAV/Ogg from buffered files or archive entries. Resource opening and
archive verification run on a worker; no full PCM cache or render-thread sound
registration is needed. Voice replay and completion remain available in history/auto mode.

Preferences include font scale, high contrast, reduced motion and waiting for voice.
Project `ui.*` translations override common UI labels; Chinese has a bundled fallback.
Some diagnostic and collection labels still use English. This is not full UI translation.

`renrs-check --json <project>` emits structured diagnostics. The VS Code extension
adds tool discovery and project creation; the LSP indexes unopened files and reloads
the disk version when an editor buffer closes.

## Persistent Progress

Create `progress.json`:

```json
{
  "achievements": [{"id":"arrival","title":"Arrived","label":"ending"}],
  "gallery": [{"id":"view","title":"The view","label":"ending","image":"images/view.png"}],
  "endings": [{"id":"ending","title":"Completed","label":"ending"}],
  "rollback_barriers": ["ending"]
}
```

Entering a configured label unlocks the corresponding item. The profile is separate
from saves and survives new games, loads and rollback. Declared variables whose
names begin with `persistent_` also survive these operations. A barrier clears prior
rollback checkpoints when its label is entered. Desktop data uses `profile.json`;
Web uses localStorage. A malformed desktop profile disables profile writes for that
session and preserves the original file for recovery.

## Media And Screen Controls

```rns
timeline:
    transform mira x 100 over 0.4 ease in_out
    transform mira alpha 0.5 over 0.4
scene "images/roof.png"
transition dissolve 0.5
video "clips/intro/clip.json" over 2
```

Timeline is a sequential list of transform, move and pause operations. `parallel`
combines independent transform/pause timelines; this remains a subset of ATL.
Each operation uses the shared serializable wait model.
Dissolve retains the previous stage. Video plays a validated frame or stream manifest.

```sh
RENRS_FFMPEG=/path/to/ffmpeg target/debug/renrs-video intro.mp4 my-story clips/intro
```

The default conversion uses FFmpeg and FFprobe, 24 fps and width 640, with up to 7,199 frames.
Add `--stream` to generate a v2 MP4 clip using FFmpeg and FFprobe. Use the duration
printed by the converter. Native v2 playback requires FFmpeg; v1 remains portable
without it. Web v2 uses HTML video. Input soundtracks become seekable stereo WAV
files referenced by the manifest. The soundtrack clock controls video position;
pause and save restoration retain the offset. Music and voice stay independent.
See [performance work](PERFORMANCE.md) for budgets, compatibility and measurements.

Desktop `screens.json` adds `toggle` widgets for `high_contrast`, `reduced_motion`
and `wait_voice`, and `input` widgets for declared string variables:

```json
{"type":"input","text":"Name","variable":"player_name","max_length":24}
```

## Web And Debugging

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.121 --locked --root target/web-tools
npm ci --prefix web
node scripts/build-web.mjs
cargo run --bin renrs-web-build -- demo target/web-game
node scripts/serve-web.mjs target/web-game 4173
```

Open `http://127.0.0.1:4173/`. HTTP hosting is required for WASM and project assets;
IndexedDB checksums require localhost or HTTPS. The output directory must be new.
The browser uses the same Rust compiler output/runtime through `crates/web`.
Saves live in IndexedDB. Import/export uses the same checked desktop container;
Rust handles raw snapshots without passing large integers through JS numbers.

The debugger includes a Cytoscape story graph, variables, call stack, instruction
breakpoints, single step and route coverage export with project fingerprint and IDs.
Graph edges describe static label targets; they do not predict dynamic path feasibility.

Web supports custom screens, theme colors/fonts, rich text, crop/anchors, NVL,
parallel animation and synchronized video. Desktop pagination and Web scrolling
remain different reading presentations. See [mobile distribution](MOBILE.md)
for Capacitor Android/iOS packaging, lifecycle handling and native save sharing.

## Distribution And Updates

```sh
cargo build --release --bins
target/release/renrs-build demo target/distribution --player target/release/renrs
node scripts/release.mjs mac-app target/distribution target/Signal.app
node scripts/release.mjs store-files target/distribution target/store-files 12345 12346
target/release/renrs-update keygen target/update-signing
target/release/renrs-update create old.renrs new.renrs target/patch target/update-signing.key
target/release/renrs-update apply old.renrs target/patch patched.renrs target/update-signing.pub
```

Keep the private signing key outside game resources and distribute the trusted public
key independently of patches. Patch application verifies Ed25519 signatures, the base
archive, changed resources and the reconstructed archive. Existing output is refused;
failure leaves the old archive untouched. There is no network auto-update client.

`mac-sign` needs `RENRS_MAC_IDENTITY`; `mac-notarize` needs `RENRS_NOTARY_PROFILE`.
`windows-sign` needs `RENRS_WINDOWS_CERT_SHA1` and the Windows SDK. Store files include
a Steam preview-only VDF and an itch launcher that must be packaged at the game root.
Signing, notarization and upload require publisher credentials. Steam achievements,
cloud saves, remote store uploads and external CI acceptance have not been performed.

## Measuring Native Frames

```sh
target/release/renrs demo --profile target/demo-frames.json --window-size 800x600
```

This uses isolated save data, muted auto playback and first-option choices. It warms
up for 60 frames, records 300 frames, then exits with p50/p95/p99/max frame time,
sampled peak RSS and resident texture bytes. Screenshot smoke reports include capture
overhead and must not be compared with this mode as gameplay performance.
