# Product Gap Work

This is the second product iteration following the Ren'Py comparison. It does not
reuse the completion status of the earlier engineering or performance iterations.

## Pre-release Policy

The project has not shipped. `0.1.0-rc.1` freezes the documented v1 machine,
archive and screen contracts; pre-RC migration is not a release criterion. Save loading
accepts only the current container and snapshot formats. When content changes,
active positions must resolve through explicit `@id`/`alias` mappings; automatic
or removed positions are rejected. Desktop/Web exchange, checksums, rollback and
transactional editor hot reload remain supported.

## P0

- [x] Shared customizable dialogue and choice screens.
- [x] Web custom screens, theme, styled text, sprite crop and anchors.
- [x] Portable desktop/Web save exchange and current-build save acceptance.
- [x] Audible first-run defaults, localized player controls, keyboard save tools.
- [x] Distribution acceptance report and packaged regression fixture.

## P1

- [x] Parallel presentation and named, precomposed character variants.
- [x] Video soundtrack playback and synchronization.
- [x] NVL, ruby, underline, keyboard controls and optional self voicing.
- [x] Integrated VS Code project and release workflow.
- [x] Shaped-cluster native wrapping and stable Web assistive semantics.
- [x] Common static ATL/master-camera migration with explicit dynamic fallbacks.

## P2

- [x] Capacitor Android/iOS projects, local native builds and platform adapters.
- [x] Lists/records, deterministic built-ins and controlled screen expressions.
- [x] Named/default label arguments, dynamic parameter scope and static migration.
- [x] Ordered named sprite layers, layer clearing, save/rollback and transactional reload.
- [x] Static per-track music/sound gain across native/Web playback, queues, saves and migration.
- [x] Localized video audio tracks, bounded relative gain, subtitle cues and native/Web fallback.
- [ ] Physical-device acceptance, platform services and store publication.

## External Acceptance

- [ ] Owner-supplied real production project.
- [ ] Local packaged-player results on the publisher's target desktop systems.
- [ ] Publisher signing, notarization and store credentials.

Unchecked work remains open. Locally generated examples are regression fixtures,
not evidence of independent author adoption or store approval.

## Character Composition

`renrs-compose <project> <character.json> <images/variants>` creates PNG presets
and an `images.rns` declaration file. Layers are blended in listed order using
the image crate. Output must be new; asset paths and decoded-memory limits are
validated. This is build-time composition, not dynamic layeredimage, lip sync or
Live2D. Every image declaration shares the project's global namespace.

```json
{
  "version": 1, "width": 800, "height": 1000,
  "layers": {
    "body": {"path":"characters/body.png"},
    "smile": {"path":"characters/smile.png","x":0,"y":0}
  },
  "presets": {"mira_smile":["body","smile"]}
}
```

## Release Acceptance

`renrs-accept game --saves saved-games` emits JSON for route assertions and each
current-build saved slot. Failures exit nonzero. No previous project is required.
The shared native/Web save
container is checksum-protected, not a cryptographic authenticity guarantee.
External acceptance fields deliberately remain false until independently verified.
The previous 12/14 cross-version migration report is no longer an acceptance gate.
Existing development saves from another script build can be discarded and replaced
by starting a new game. Explicit end anchors are not required for current-build saves.
The CLI and VS Code no longer expose previous-version comparison.

## Local Evidence

- Rust workspace tests and strict Clippy; focused native audio seek/pause/restart
  test without an audio device; explicit FFmpeg worker test.
- Nine Chromium workflows at 1280 and 390 pixels: custom UI, portable saves including
  integers beyond JS precision, variable actions, animation motion, NVL and both
  soundtrack formats with pause/save/restore.
- Native custom-screen smoke captures: `target/product-native-v2`.
- Native NVL/ruby smoke and save restoration: `target/reading-native-captures`.
- VS Code host regression and packaged `editors/vscode-renrs/renrs-0.1.0-rc.1.vsix`.
- Android APK: `target/mobile-product-v1/android/app/build/outputs/apk/debug/app-debug.apk`.
- iOS simulator app: `target/mobile-ios-build/Build/Products/Debug-iphonesimulator/App.app`;
  installed/launched on iPhone 17 simulator, screenshot `target/mobile-ios-screen.png`.
- Acceptance tests cover current-build dialogue and finished saves, compatible
  content edits at explicit anchors, and rejection of removed anchors and malformed saves.
- Current product fixture acceptance passes both routes and all four native slots
  using `renrs-accept target/product-story-v3 --saves target/product-native-v2/data/saves`.

The latest preview at http://127.0.0.1:4183/product/ demonstrates custom screens
and data; http://127.0.0.1:4183/reading/ demonstrates animation, NVL, typewriter
progress and synchronized video. Reading now pauses with dialogs and background
tabs, restores visible-character progress from saves, and batches read-history
writes. See [the performance iteration](PERFORMANCE_COMPARISON.md) for its
verification and measured comparison with Ren'Py.

## Remaining Boundaries

No Python/Ren'Py plugin compatibility, complete ATL, non-master layer cameras, arbitrary displayables,
dynamic runtime layeredimage, cloud service or store integration has been added.
Desktop pages and Web scroll areas differ;
save transfer preserves runtime progress, not identical text wrapping. Speech depends on installed
OS/browser voices. Web exposes structured dialogue, choice, scene and custom-control semantics;
the native Macroquad player still lacks an OS accessibility tree.
Mobile builds run Rust/WASM in WebView; physical devices and native share workflows
still need acceptance. See [mobile distribution](MOBILE.md).
