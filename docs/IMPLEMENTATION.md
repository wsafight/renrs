# Implementation Status

This tracks the P0 to P2 work requested on 2026-09-06. A checked item includes
implementation and relevant verification; external acceptance remains explicit.

## P0

- [x] Reject ambiguous cross-version save positions; preserve anchored calls and rollback.
- [x] Initialize missing defaults when restoring saves and rollback checkpoints.
- [x] Share project resource inclusion rules across loading, packing and watching.

P0 verification: 114 tests, formatting and strict Clippy passed locally.
The current snapshot v7 uses call-site IDs for return addresses, preserves dynamic
parameter scopes, and interns shared values. Older snapshot formats are rejected;
content updates restore only through
explicit positions, while automatic positions are rejected when the fingerprint changes.

## P1

- [x] Protect progress on overwrite, load and exit; periodic and chapter autosaves.
- [x] Background save operations, lightweight metadata, pages, screenshots and notes.
- [x] Editor disk/buffer indexing, references, diagnostics and host regression tests.
- [x] Audio completion, errors, replay and configurable defaults.
- [x] Common player UI localization and accessibility preferences.
- [x] Tool discovery, project setup and local desktop distribution validation.
- [x] Frame timing and memory measurements on demo and a synthetic medium workload.

## P2

- [x] Interactive story graph, state inspection, breakpoints and route coverage.
- [x] Persistent achievements, gallery, endings and rollback barriers.
- [x] Sequential timeline, dissolve, silent frame clips and desktop toggle/input controls.
- [x] Signing/notarization commands, signed resource deltas and local store-file generation.
- [x] Web player with the shared Rust runtime; desktop/mobile Chromium acceptance.
- [x] Ordered named sprite layers across compiler, save/rollback, native/Web rendering and LSP.

This historical iteration delivered a bounded subset. Subsequent product work
added synchronized video, parallel tracks, shared Web screens, keyboard save tools,
NVL and Capacitor mobile builds. Current status and limits are tracked separately
in [PRODUCT_UPGRADES.md](PRODUCT_UPGRADES.md). Full ATL, complete UI translation,
native Rust mobile rendering, store SDKs and cloud saves remain outside that subset.
External signing/notarization and remote platform acceptance remain unchecked below.

Subsequent performance work added streaming audio/video, background compilation,
bounded decoding and compact Web state. See [PERFORMANCE.md](PERFORMANCE.md).

Shared Rust implementation verification: 125 tests passed after splitting the
workspace into syntax, compiler, runtime, project, editor and web crates. The native
player and CLI entry points remain in the root package. Common UI labels are
localized; some diagnostics and collection text still use English.

Local acceptance also passed six Chromium workflows, the VS Code extension host,
native demo and long-text/custom-control captures, packaged macOS app startup,
and two 300-frame release measurements. See [VALIDATION.md](VALIDATION.md).
The current Web preview is served at http://127.0.0.1:4174/ from `target/web-final`.

## External Acceptance

- [ ] A real medium-size project supplied by its owner.
- [ ] Local packaged-player acceptance on the publisher's target desktop platforms.
- [ ] Signing identities, notarization and store credentials provided by the publisher.
