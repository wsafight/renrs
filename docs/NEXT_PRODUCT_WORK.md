# Product Development Follow-up

Scope: develop the remaining product and performance gaps in priority order.
Commercial-project acceptance, cross-platform acceptance and publication are
excluded by the owner. Previous save formats need not remain compatible.

Current scope decision (2026-09-08): release P0 work, physical-device and real-project
acceptance are paused. Repository-closed P1/P2 work uses `node scripts/verify-local.mjs`
as the acceptance gate. An Ubuntu GitHub Actions workflow now runs fmt, Clippy,
Rust workspace/player tests, and Web checks. Agent-facing `SKILL.md`, `llms.txt`,
a read-only MCP server, and related integration work are also paused.

The native player lives in `crates/player` (`renrs` binary) so CLI tools no longer
link macroquad or rodio. Web runtime state omits an unchanged stage snapshot, and
parallel animations are sampled in WASM.

## At a glance

| State | Scope |
| --- | --- |
| Complete in the repository | P1/P2 authoring, bounded media, Launcher/SDK, machine protocol, site, and local verification |
| Paused | P0 publication work, real-project acceptance, physical-device checks, and Agent integration |
| Deferred P2 | Live2D, custom shaders, cloud/store SDKs, native mobile rendering, and other heavy backends |

For using RenRS, start with [Quick start](QUICKSTART.md). For release readiness and the
remaining external checks, use the [release contract](RELEASE.md). The detailed list below
is an implementation record rather than a required reading sequence.

- [x] P0: shared snapshot encoding and lightweight native/Web save listing.
- [x] P1: composable UI, scroll containers, data-driven controls and drag/drop.
- [x] P1: camera, runtime character layers and richer presentation.
- [x] P1: deterministic extension APIs with persistence/rollback semantics.
- [x] P1: complex text layout, fallback fonts and localization rules.
- [x] P0: split local acceptance gates, test Launcher/Web protocol boundaries, and add a fixed 30-60 minute first-party reference fixture.
- [x] P1: shaped-cluster line breaking, Web assistive semantics, static ATL/master-camera and bounded standard layer-camera migration, media gates, mobile wrapper validation, and the `0.1.0-rc.1` contract.
- [x] P1 migration subset: static ATL blocks with terminal `repeat 1..16` are expanded to recompilable RenRS transforms; unbounded, parameterized and dynamic loops remain structured diagnostics.
- [x] P1 migration subset: existing-resource `show/scene expression "..."`, `Image("...")`, and `im.Image("...")` are converted to ordinary static images; dynamic expressions remain unsupported.
- [x] P1 migration subset: bounded existing-resource `Transform`/`im.Transform` expressions with static numeric
  options expand to recompilable RenRS transforms; dynamic displayables and unsupported options remain diagnosed.
- [x] P1 migration subset: static `Composite`/`im.Composite` expressions with bounded canvas coordinates and
  existing image resources generate deterministic `.layers.json` compositions; scene composites, dynamic
  displayables and `LiveComposite` remain diagnosed.
- [x] P1 migration diagnostics: unknown identifier-led statements receive stable `custom_statement_unsupported`
  issues with their source fragment; the official `testsuite`/`testcase` baseline remains generic.
- [x] P1 migration diagnostics: parameterized ATL declarations, image uses and camera uses receive the stable
  `atl_parameters_unsupported` code instead of a generic statement failure.
- [x] P1 migration subset: finite numeric positional parameterized ATL calls specialize at static `show` and
  camera call sites; parameter expressions, defaults, loops and parameterized `scene` remain explicit diagnostics.
- [x] P2: project launcher, SDK workflow, templates and author documentation; local end-to-end verification passes.
- [x] P2: v1 machine protocol, read-only project inspection, baseline/candidate impact analysis and Launcher quality reporting.
- [x] P2: bounded media configuration: localized video audio, relative gain, subtitle cues and shared native/Web fallback.
- [ ] P2: heavier graphics and platform-service backends (Live2D, custom shaders, cloud/store SDKs and native mobile rendering).

Each item requires implementation and focused verification before checking it.
External SDKs and services must remain explicitly identified when unavailable.

The launcher is available with `node scripts/launcher.mjs`; SDK bundles are
assembled with `node scripts/package-sdk.mjs <new-directory> <binary-directory>`.
The launcher delegates inspect, check, run, graph, pack and build tasks to existing
Rust tools, keeps bounded logs, detects stale script edits, and stores local paths
only. Its quality view separates static reachability from actual route coverage and
shows diagnostics, route results and localization gaps. Machine-facing check,
debug, acceptance, inspection and impact commands share the versioned v1 envelope.
Theme files support fallback font families. Independent music, sound and voice
volume defaults now combine with bounded per-track music/sound gain. Video manifests
support localized WAV audio, relative gain and bounded subtitle cues on native and Web.
Conditional `.layers.json` layers refresh when story or screen variables change, and validated
mutually exclusive variant groups select the last matching layer. Static `show/scene ... with`
clauses migrate to recompilable `transition` statements; dynamic and custom transitions remain
manual migration work.
Static ATL `repeat 1..16` blocks are expanded with Ren'Py's total-cycle semantics and verified
through migration, recompilation and runtime-effect tests; bare `repeat` remains unsupported to
avoid introducing an unbounded runtime loop.
Static image expressions are only accepted when a single- or double-quoted path, a no-argument
`Image`/`im.Image` constructor, or a one-transform `At`/`im.At` wrapper resolves to a project resource;
`show expression` additionally requires an explicit alias so the resulting RenRS stage identity
is stable. Dynamic paths, extra constructor arguments, missing resources, and missing aliases have
separate report codes.
Dynamic `jump` / `call` targets remain manual migration work, but now use stable
`jump_target_dynamic`, `call_target_dynamic`, and `call_clause_unsupported` report codes while
retaining the source target or invocation text.
Layer cameras use explicit `transform camera onlayer <layer> ...` statements and are sampled in
parallel animations and rendered by native/Web frontends; arbitrary Ren'Py camera/displayable
semantics remain outside the supported subset.
This does not complete arbitrary mixers, long-form/device media validation, graphics,
or platform-service extensions. Live2D,
custom shaders and cloud/store services remain unimplemented. Commercial-project
and cross-platform acceptance are excluded from this development scope.

The reference fixture is generated by `renrs-bench generate-reference`; it is not
an owner-supplied production project. Compatibility versions and remaining physical-device,
signing and store gates are tracked in [the release contract](RELEASE.md).

## Save Evidence

Snapshot v8 interns scalar/collection nodes and repeated stage states, restores shared
collections, and preserves dynamic label-parameter scopes.
The save container remains v2, writing v8 and accepting v7/v8 snapshots. Native summaries
are used by both listing APIs and copied during rotation; Web IndexedDB summaries
are separate from payloads and indexed by project. Loading still checks payloads.

Same 818-instruction/401-interaction large-state fixture, release build, one local
run before/after: snapshot 34,773,179 -> 348,528 bytes; first six-save listing
3274.05 -> 0.66 ms; save 197.13 -> 23.20 ms. New full disk load/restore measurement
was 3.89 ms. These are workload-specific observations, not cross-engine claims.

Shared snapshot/rollback tests, native save tests and both 1280/390px Web data and
save workflows pass. UI native clipping and layered-image verification are ongoing.
