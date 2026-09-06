# Product Development Follow-up

Scope: develop the remaining product and performance gaps in priority order.
Commercial-project acceptance, cross-platform acceptance and publication are
excluded by the owner. Previous save formats need not remain compatible.

- [x] P0: shared snapshot encoding and lightweight native/Web save listing.
- [x] P1: composable UI, scroll containers, data-driven controls and drag/drop.
- [x] P1: camera, runtime character layers and richer presentation.
- [x] P1: deterministic extension APIs with persistence/rollback semantics.
- [x] P1: complex text layout, fallback fonts and localization rules.
- [ ] P2: project launcher, SDK workflow, templates and author documentation (implemented; end-to-end verification pending).
- [ ] P2: media configuration, graphics extensions and platform-service adapters (new work pending).

Each item requires implementation and focused verification before checking it.
External SDKs and services must remain explicitly identified when unavailable.

The launcher is available with `node scripts/launcher.mjs`; SDK bundles are
assembled with `node scripts/package-sdk.mjs <new-directory> <binary-directory>`.
The launcher delegates check, run, graph, pack and build tasks to existing Rust
tools, keeps bounded logs, detects stale script edits, and stores local paths only.
Theme files support fallback font families. Independent music, sound and voice
volume defaults and local platform adapters predate this follow-up; they do not
complete the planned media, graphics or platform-service extensions. Live2D,
custom shaders and cloud/store services remain unimplemented. Commercial-project
and cross-platform acceptance are excluded from this development scope.

## Save Evidence

Snapshot v5 interns scalar/collection nodes and restores shared collections.
The save container remains v2, accepting only current snapshots. Native summaries
are used by both listing APIs and copied during rotation; Web IndexedDB summaries
are separate from payloads and indexed by project. Loading still checks payloads.

Same 818-instruction/401-interaction large-state fixture, release build, one local
run before/after: snapshot 34,773,179 -> 348,528 bytes; first six-save listing
3274.05 -> 0.66 ms; save 197.13 -> 23.20 ms. New full disk load/restore measurement
was 3.89 ms. These are workload-specific observations, not cross-engine claims.

Shared snapshot/rollback tests, native save tests and both 1280/390px Web data and
save workflows pass. UI native clipping and layered-image verification are ongoing.
