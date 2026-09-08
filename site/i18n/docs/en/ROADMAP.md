# RenRS roadmap

This page keeps early milestones. The latest implementation and acceptance as of
2026-09-06 is [Product upgrades](PRODUCT_UPGRADES.md). Shared screens/saves, parallel
animation, NVL, video with an audio bed, collection expressions, and Capacitor mobile
builds have been added. The project is not released. Breaking changes are allowed.
Old-version compatibility and save migration left the current implementation scope.

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

## P2: remaining extensions

This round first shipped independently verifiable P2 performance, screen subset, and
authoring tools. See [Optimization notes](OPTIMIZATION.md): background image decode
and prediction, resource budgets, save-list cache, streaming pack, templates and
editor entry, story-route debug, declarative JSON screens, synthetic benches, and
native captures. The items below are remaining extensions. They do not relist shipped
subsets as unimplemented.

These are still clearly missing versus Ren'Py. They cannot be marked supported without
a real backend.

1. **Custom screen extensions**: composable displayables, nested viewports, drag/drop, and full AT semantics.
2. **Advanced presentation**: named sprite layers are shipped; full ATL, layer cameras, arbitrary displayables, composite transitions, shaders, particles, and Live2D remain.
3. **Video**: extra audio and subtitle tracks, more device and long-form sync acceptance.
4. **Performance**: real project samples and font-cache budgets. Incremental compile is shipped.
5. **Shipping platforms**: native Rust mobile rendering, Capacitor device matrix, signing/notarization external acceptance, store SDKs, and a network auto-update client.
6. **Advanced narrative state**: fixed rollback, finer preference sync, and cloud saves.
7. **Migration coverage**: default screens, a common static ATL subset, and the first
   real-sample baseline are shipped. Complex image expressions, parameterized/looping
   ATL, dynamic jump/call, custom statements, and a broader real-project corpus remain.
8. **Extension mechanism**: do not embed Python. If needed, evaluate a least-privilege WASM plugin API separately.

## Recommended order

1. Add a real mid-size project on the existing synthetic bench. Measure frame time, resource peaks, first save read and write.
2. Accept the authoring flow in a VS Code host. Extend JSON screen controls and accessibility as the project needs.
3. Validate the current FFmpeg/Rodio/HTML media clock with real video and long clips, and fill the platform matrix.
4. Drive migrator expansion with real Ren'Py samples. Each new mapping needs a report, tests, and a fallback policy.
5. Handle mobile, stores, and updates last. They depend on stable input, lifecycle, and shipping format.

## Agent collaboration proposal

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
