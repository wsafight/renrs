# RenRS roadmap

This page preserves early priorities and milestones as planning context. It is not the
onboarding path or the current status checklist. Start with [Quick start](QUICKSTART.md),
use [Current status](NEXT_PRODUCT_WORK.md) for completed, paused, and deferred scope,
and read [Product upgrades](PRODUCT_UPGRADES.md) for detailed delivery history.

The project is not released. Starting at `0.1.0-rc.1`, v1 machine, archive, and screen
contracts are frozen. Pre-RC compatibility and save migration remain outside the current
scope; see the [release contract](RELEASE.md).

## Priority

- **P0**: blockers for reliable authoring, run, save, and shipping.
- **P1**: important capability for mid-size visual novel production, expression, and migration quality.
- **P2**: extensions that need their own design or heavier dependencies. Not a completion condition for the current version.

Done means code, headless tests, and docs landed. It does not mean full Ren'Py
equivalence. Comparison is against `references/renpy` at `9ed7dd3` (2026-09-04). The
detailed matrix is [Gap with Ren’Py](RENPY_GAP_ANALYSIS.md).

## P0: reliable authoring and shipping (done)

- [x] Unified `ProjectSource`: directory projects and `.renrs` archives use the same script, image, font, theme, translation, and audio read bounds.
- [x] `.renrs` can be checked and played directly. Archive mode disables hot reload; directory mode keeps transactional hot reload.
- [x] Added `config id`, and put settings, read state, and saves in a system user data directory isolated by project ID.
- [x] Added `default`, static `image`, label parameters, `call` arguments, `return <expr>`, and `_return`.
- [x] Added conditional menus and fixed definite-assignment analysis across `call` / `return` edges.
- [x] Split execution IDs, translation IDs, and read IDs. Support `@id` and old-ID `alias`.
- [x] JSON translation catalogs, explicit and language-tag fallback, empty translations falling back to source, translation before interpolation and rich-text parse.
- [x] Saves only accept current snapshot format v7 and the current script fingerprint. Development hot reload keeps stable ID mapping.
- [x] Saves include project ID, content fingerprint, play time, and chapter, with SHA-256 integrity. Corrupt slots stay visible.
- [x] Save container reads and writes v2 only, with mandatory checksums. Archive unpack rejects symlink escapes and overwrite.
- [x] 3 quick and 5 auto rotating slots, plus import/export store API.
- [x] `renrs-build` emits a shipping directory of player, `game.renrs`, build manifest, and run notes.
- [x] `renrs-migrate --strict` re-runs parse, validate, compile, and control-flow analysis after migration.

## P1: production and expression (done)

- [x] General `transform` that saves, loads, and rolls back: x/y, scale, rotate, alpha, anchor, crop, uncrop.
- [x] Transform `over` plus `linear`, `in`, `out`, `in_out` easing. Keep basic position tween and fade transitions.
- [x] Independent music, sound, and voice channels. Music fade-in, queue, fade-out, and static music/sound relative gain.
- [x] WAV/Ogg duration advances a non-looping music queue. Current defaults 0.6/0.8/1.0, tests muted.
- [x] Project `theme.json`, custom fonts, high contrast, and reduced motion.
- [x] `renrs-i18n extract/update/check`. The player reads catalogs from `locales/*.json`.
- [x] LSP workspace symbols, definition, references, rename, and completion, keeping diagnostics and format.
- [x] Offline migrator covers the supported Ren'Py static subset. Uncertain or dynamic syntax goes into a structured report, not silent guesses.
- [x] Parser, runtime, compiler, analysis, migrator, player, and LSP split by duty.
- [x] Tests force every Rust file under `src/` and `tests/` to stay under 500 lines.
- [x] Split core/Web/media/editor/release local gates and test Launcher protocol and Web semantic boundaries.
- [x] Add a 10-chapter, 500-dialogue, two-route, 30-60 minute first-party reference fixture; external author acceptance remains open.
- [x] Use shaped-cluster boundaries for native wrapping; expose stable Web dialogue status and scene, choice, and control semantics.
- [x] Migrate static `easein`/`easeout`, ATL pause, and master camera; report dynamic, looping, and layer-camera cases.
- [x] Freeze the release-format matrix, prepare `0.1.0-rc.1`, and include mobile wrapper validation in the release gate.

## P2: remaining extensions

This round first shipped independently verifiable P2 performance, screen subset, and
authoring tools. See [Optimization notes](OPTIMIZATION.md): background image decode
and prediction, resource budgets, save-list cache, streaming pack, templates and
editor entry, story-route debug, declarative JSON screens, synthetic benches, and
native captures. The items below are remaining extensions. They do not relist shipped
subsets as unimplemented.

These are still clearly missing versus Ren'Py. They cannot be marked supported without
a real backend.

1. **Custom screen extensions**: nested viewports, drag/drop, and Web semantics are shipped; arbitrary displayables and a native OS AT tree remain.
2. **Advanced text layout**: build on the current `rustybuzz` shaping, BiDi, font fallback,
   CJK wrapping, ruby, and shaped-cluster-aware wrapping. Vertical layout, color emoji,
   and native screen-reader semantics remain.
3. **Advanced presentation**: named sprite layers are shipped; full ATL, layer cameras, arbitrary displayables, composite transitions, shaders, particles, and Live2D remain.
4. **Video**: localized audio tracks, relative volume, and subtitle tracks are shipped;
   more device and long-form sync acceptance remains.
5. **Performance**: real project samples and font-cache budgets. Incremental compile is shipped.
   Execution already uses a linear `Program` instruction IR; only evaluate compact opcodes
   or expression bytecode when profiling identifies expression evaluation or dispatch as a bottleneck.
6. **Shipping platforms**: native Rust mobile rendering, Capacitor device matrix, signing/notarization external acceptance, store SDKs, and a network auto-update client.
7. **Advanced narrative state**: fixed rollback, finer preference sync, and cloud saves.
8. **Migration coverage**: default screens, common static ATL/master camera, and a first-party
   reference baseline are shipped. Complex image expressions, parameterized/looping
   ATL, dynamic jump/call, custom statements, and a broader real-project corpus remain.
9. **Extension mechanism**: do not embed Python. If needed, evaluate a least-privilege WASM plugin API separately.

## Recommended order

1. The first-party 30-to-60-minute fixture now covers regression. An external author's real
   mid-size work is still required for authoring, migration, and shipping baselines. Record frame
   time, resource peaks, save I/O, production time, and platform issues.
2. Address real-work blockers in advanced text layout, JSON screen controls,
   accessibility, ATL, and migration coverage. Every addition needs shared desktop/Web
   semantics, structured diagnostics, and a fallback policy.
3. Maintain the completed Web toolchain typing: the Web player, VS Code extension, and
   Launcher view all use strict TypeScript. Explicit parsers protect WASM boundaries,
   Zod validates Launcher machine-protocol responses, and Biome formats stable Node
   operations scripts that remain `.mjs` or `.cjs`.
4. Validate the FFmpeg/Rodio/HTML media clock with real long-form video and complete
   desktop, browser, and Capacitor device matrices.
5. Compatibility windows for `.rns`, machine protocols, screen schemas, archives, and saves are
   defined and `0.1.0-rc.1` is prepared. Publish `0.1.0` only after external work, target-platform,
   and publisher-credential gates pass.
6. Optimize expression IR or introduce bytecode only when real profiling identifies
   execution IR as a user-visible bottleneck. Do not presume a full VM rewrite is the
   player's performance foundation.

## Technical evolution boundaries

- **Execution model**: the compiler already lowers scripts to linear `Program`
  instructions with stable IDs and jump targets. Runtime advances an instruction cursor;
  only expressions inside instructions are currently evaluated recursively. Do not
  describe the whole runtime as tree-walking.
- **Text system**: keep the current `rustybuzz`, BiDi, Unicode segmentation, and font
  cache. Before adopting a heavier dependency such as `cosmic-text`, compare complex
  scripts, ruby, vertical text, memory, and cross-platform rendering in an isolated prototype.
- **Storage boundary**: saves keep temporary-file writes, `fsync`, atomic replacement,
  versions, and checksums. A mini KV, WAL, or LSM tree belongs in an independent systems
  experiment until cloud sync or large incremental state creates a product requirement.
- **Ren'Py gap**: do not target feature-by-feature parity. Python, Ren'Py saves, and
  arbitrary host code remain non-goals. Add only capabilities that a real work proves
  necessary for authoring, presentation, migration, or shipping.
- **TypeScript and Web**: `web/src` now uses strict TypeScript under Vite with explicit
  types and runtime validation at WASM/JSON boundaries. The VS Code extension compiles
  from strict TypeScript to a distributable CommonJS bundle. Vite builds the Launcher
  view, whose API responses are validated with Zod. Keep Astro's TypeScript support and
  retain short, stable `.mjs`/`.cjs` build, release, and test scripts. Biome provides the
  shared formatting and static-analysis gate for maintained Web and Node sources.
- **Framework choice**: Vite, Astro, Playwright, and Capacitor already cover the current
  build and platform boundaries. The Web player is centered on a game stage and WASM
  state machine, so do not add a general SPA framework yet. If component lifecycle and
  state synchronization repeatedly cause defects in a real project, compare Lit or
  Svelte prototypes on bundle size, accessibility, tests, and mobile behavior.

## Agent collaboration proposal

Status: paused. `SKILL.md`, `llms.txt`, a read-only MCP server, and other agent
integration are outside the current P1/P2 round. The items below remain a future proposal.

This path builds on existing static checks, deterministic runtime, headless debugging, and
route tests. It does not claim the capabilities below are implemented. Agent layers must
reuse core libraries or stable CLIs: the engine supplies facts and verifies candidate
changes, while the agent plans and edits.

1. [x] Standardize exit codes, error codes, and versioned JSON Schemas across the existing
   binaries. `renrs-check --json`, `renrs-debug inspect/test/explore`, `renrs-accept`, and
   Web `inspect()` are the current foundation.
2. [x] Add read-only project state for configuration, characters, variables, assets, story
   graph, localization, script fingerprint, and capability version.
3. [x] Accept an explicit baseline and candidate project or Git revision, then report affected routes,
   endings, translations, and save structure. The engine must not guess an agent plan when
   no proposed change was supplied.
4. Publish `SKILL.md`, `llms.txt`, JSON Schemas, and a machine-interface version policy that
   defines editing steps and mandatory checks.
5. [x] The launcher displays checks, variables, route tests, coverage, localization,
   migration and impact reports, and a project capture preview.
6. Once the interface is stable, provide a thin MCP server focused on reads and validation.
   Mutations must return target files and diffs.
7. Have the owner or an external author validate human-agent collaboration on a real work.
   That external acceptance remains outside the current local-development scope.

Cloud accounts, online asset storage, complex team permissions, hosted builds, model
gateways, billing, and a large dashboard are not near-term parts of this path. Agent
interfaces also do not replace non-programmer authoring experience, basic product quality,
or real-work validation.

## Historical milestones

- **v0.1**: script AST, validation, interpreter, basic stage, audio front-end, settings, saves, and a sample.
- **v0.2**: multi-file projects, stable IDs, control-flow analysis, transactional hot reload.
- **v0.3**: rollback, auto-forward, skip-read, independent voice channel.
- **v0.4**: fade, position tween, integer layer, basic rich text.
- **v0.5**: format, LSP, story graph, asset archives, three-platform release template.
- **v0.6**: Ren'Py static-subset migration and structured compatibility reports.

## Tradeoffs

- Nearby but independent `.rns` syntax. No promise to execute `.rpy` or load Ren'Py saves.
- Core runtime executes no Python or arbitrary host code, staying deterministic, analyzable, and headless-testable.
- Execution position uses stable instruction IDs. Translation and read state have their own identity bounds.
- Directories are for authoring and hot reload. Checked `.renrs` is for read-only shipping.
- Human audio listening is last. Automated verification only checks the state machine, formats, and queue advance.
