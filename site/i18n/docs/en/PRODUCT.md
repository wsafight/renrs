# RenRS product document

## 1. Product definition

RenRS is a visual novel engine written independently in Rust and inspired by Ren'Py.
It is for authors and developers who want declarative scripts for story, plus native
desktop shipping, deterministic playback, static analysis, and auditable saves.

RenRS uses its own `.rns` language. It does not execute Python, run `.rpy` files, or
read Ren'Py saves. The offline migrator converts only the supported static subset and
writes ambiguous content into a report.

## 2. Core users

- Independent authors making visual novels, interactive narrative, and dialogue prototypes.
- Small teams that need headless checks, automated shipping, and stable data formats.
- Engineering teams that want to embed the runtime in Rust or build authoring tools.

## 3. Core value

- **Low writing floor**: describe characters, dialogue, assets, variables, and branches in screenplay-like syntax.
- **Deterministic playback**: parse, validate, compile, and analyze before runtime. No arbitrary host-language code.
- **Recoverable state**: execution, stage, animation, audio, call stack, and rollback checkpoints serialize.
- **Native delivery**: one command turns a directory project into a player plus a read-only `.renrs` archive.
- **Diagnosable toolchain**: check, format, LSP, story graph, localization, and migration all run headless.

### 3.1 Current positioning

RenRS is differentiated by making visual-novel development checkable, testable, and
reproducible, not by being "Ren'Py rewritten in Rust." External descriptions should lead
with the author outcomes of static checks, route tests, deterministic state, and Git
collaboration rather than the implementation language.

The project is still prerelease. A shipped feature means its code, tests, and documentation
have landed; it does not mean long-term validation in real productions or low-risk Ren'Py
replacement. A more accurate current position is:

> A Git-friendly visual novel engine for technical authors and small teams, with static
> story checks and automated route testing.

### 3.2 Current fit

RenRS currently fits technical authors willing to validate an early product, experiments
with complex branching, and teams that need headless checks, route replay, and auditable
state. It is not yet low-risk infrastructure for:

- existing projects that rely heavily on Ren'Py Python, plugins, full Screen Language, or ATL;
- projects close to commercial release that require mature store, signing, notarization, and multi-platform support;
- work that requires Live2D, complex shaders, particles, 3D, platform services, or cloud saves;
- long projects that require durable script and old-save compatibility but cannot absorb prerelease format changes.

The launcher and VS Code extension provide authoring entry points, but first install,
asset management, instant preview, tutorials, and shipping for non-programmers still need
validation with real users.

## 4. Current user flow

1. Create a playable template with `renrs-init`, then write story in one or more `.rns` files with a stable `config id`.
2. Put images, fonts, audio, `theme.json`, optional `screens.json`, and `locales/*.json` in the project directory.
3. Run `renrs-check <project>` for script, asset, and control-flow checks.
4. During development run `renrs <project-directory>`; script, asset, and support-file changes can hot-reload. The VS Code extension is a unified entry.
5. Use `renrs-i18n` to extract, update, and check translation catalogs.
6. Verify routes with `renrs-debug test/explore`, then `renrs-build <project> <output-directory>` for a shipping folder.
7. Player settings, read state, and saves go to the system user data directory. They do not pollute the project or a read-only archive.

## 5. Shipped scope

### 5.1 Script and static analysis

- `config title` / `config id`, `define`, `default`, and static `image` declarations.
- Backgrounds, global sprite aliases, ordered/clearable named sprite layers, dialogue, interpolation, basic rich text, and conditional menus.
- `set`, deterministic expressions, lists/records and built-in collection ops, `if` / `elif` / `else`.
- `jump`, positional/named/default `call` arguments, `return <expr>`, and `_return`.
- `move`, transform, easing, serial/parallel timelines, fade/dissolve, and character precomposition.
- Music, sound, voice, music queue, fade commands, and static music/sound relative gain.
- Multi-file declaration checks, asset checks, unreachable code, definite assignment, and immediate-loop diagnostics.

### 5.2 Playback and UI

- 1280x720 logical canvas, uniform scale, background/sprite/UI layers.
- Stable typewriter text, long-dialogue paging, full history scrolling, menu focus, auto-forward, skip-read, and interaction rollback.
- Title, settings, 60 manual save slots, quick save/load, and grouped manual/quick/auto browsing.
- `theme.json`, project fonts, high contrast, and reduced motion.
- Desktop/Web `screens.json` for title, dialogue, choices, save/load, preferences, and history, with controlled variable updates and a read-only HUD.
- NVL, multi-segment reading, ruby, underline, and system self-voicing.
- Independent music, sound, and voice volumes, defaults `0.6`, `0.8`, `1.0`; music/sound statements can set relative gain; tests mute.
- Directory mode watches scripts, media, theme, fonts, translations, and screens. Archive mode stays read-only and deterministic.

### 5.3 Localization

- Dialogue and menus use independent `TranslationId`, with explicit `@id`, menu `id`, and old-ID aliases.
- JSON per-language catalogs, explicit fallback, and `zh-Hans-CN -> zh-Hans -> zh` chain fallback.
- Missing or empty translations fall back to source text. Translation happens before interpolation and rich-text parse.
- The player reads `locales/*.json`. Language comes from player settings, or first-use `RENRS_LANGUAGE`.
- The settings page can switch current dialogue/menu language immediately and keeps the previous state on failure. An explicit source-text choice is not overridden by the environment variable.
- `renrs-i18n extract/update/check` extracts, adds empty items, keeps obsolete items, and checks completeness in CI.

### 5.4 Project and shipping

- `ProjectSource` gives directories and `.renrs` the same script and asset API.
- `.renrs` uses a versioned manifest, canonical relative paths, bound checks, and per-asset SHA-256.
- The player can open `.renrs` directly. With no arguments it prefers `game.renrs` next to the player, independent of cwd.
- `renrs-build` validates the project and writes the player, `game.renrs`, a build manifest, run notes, and engine/font licenses.
- Local release scripts assemble the current-platform toolkit, Web shell, editor extension, and checksummed SDK.

### 5.5 Current-version saves

- The project is not released. Breaking format changes are allowed. Old container and snapshot formats are not migrated.
- The current runtime snapshot is v7. An identical fingerprint restores directly. After a content update, only explicit `@id`/`alias` positions are mapped; automatic, removed, or inconsistent positions are rejected.
- v7 records stable execution positions, call-site return addresses, shared value nodes, stage, transforms, audio queues with relative gain, language, history, and bounded rollback checkpoints.
- The current save container is v2: engine version, project ID, content fingerprint, play time, chapter, and SHA-256 integrity. Other container versions and missing or mismatched checksums are rejected.
- Corrupt slots stay visible in the list. The store API supports import/export. The player rotates 3 quick and 5 auto slots.
- Read state is not part of a single snapshot. It is stored in project-level `read.json`.
- Hot reload and current-format persisted saves share ID/alias mapping. Failure preserves the live session or original save respectively.

### 5.6 Engineering constraints

- The core runtime does not need a window. Tests can run with no GPU and no audio device.
- Parser, compiler, analysis, runtime, player, migration, and LSP are split by duty.
- Tests reject any Rust source file over 500 lines in the root package and all workspace crates.
- Video is converted through FFmpeg/FFprobe to frames or a streaming manifest, optionally with a WAV bed, and plays on the audio clock.

Async saves, achievements, web, debugger, and shipping tools added in this round are in [Capability upgrades](UPGRADES.md).

### 5.7 Authoring and scale verification

- `renrs-init` creates a project with art, Chinese translations, two endings, a theme, and route assertions.
- The VS Code extension provides LSP, save-time project diagnostics, image/audio preview, run, shipping build, and route tests.
- `renrs-debug` provides inspect, record, replay, route assertions, and bounded branch exploration, with a clear report on truncation.
- Background image decode, a little next-image prediction, texture/audio cache budgets, save-list cache, streaming archive pack.
- `renrs-bench` generates a multi-file synthetic project and measures compile, run, snapshot, save, watch, and archive work.
- Player `--smoke-test` drives real rendering and save/load in an isolated data directory and writes PNG and JSON reports.

## 6. Data location

`config id` is the persistence identity. Use a stable reverse domain such as
`org.example.my-story` and do not change it after release.

- macOS: `~/Library/Application Support/RenRS/<project-id>/`
- Windows: `%APPDATA%/RenRS/<project-id>/`, falling back to `%LOCALAPPDATA%`
- Linux: `$XDG_DATA_HOME/renrs/<project-id>/`, or `~/.local/share/renrs/<project-id>/`

The directory contains `settings.json`, `read.json`, and `saves/*.json`. Without
`config id` it is derived from the title, which is fine for prototypes, not as a
long-term identity for a shipped project.

## 7. Current non-goals

- Python, arbitrary host code, Ren'Py plugins, or Ren'Py save compatibility.
- Ren'Py screen language, a full style/displayable system, and full ATL; only a constrained JSON UI subset.
- Live2D, complex particles, 3D, custom shaders, layer cameras, and arbitrary displayables.
- Native Rust mobile rendering, cloud saves, multiplayer, and network auto-update. Current mobile output uses a Capacitor WebView.
- Fetching shipping credentials or completing store review for the publisher. Existing signing, notarization, and mobile pack tools need the matching SDKs and accounts.
- A stable native plugin ABI. If extensions are needed later, prefer least-privilege WASM.

## 8. Acceptance

- The same project loads consistent scripts and assets from a directory and an archive, and passes headless checks.
- Dual-path samples run end to end. Save/load, rollback, hot reload, and translation do not break stable execution position.
- Invalid syntax, assets, control flow, archives, translation catalogs, and save checks return explicit errors.
- The P0/P1 local gate passes; target desktop systems receive local machine or VM acceptance before release, strict Clippy is warning-free, and Rust files stay under 500 lines.
- Audio state is covered by automated tests, muted in test mode. Actual listening is left for final human acceptance.

## 9. Next

Sequential optimization and evidence are in [Optimization notes](OPTIMIZATION.md) and
[Local validation](VALIDATION.md). Later results are in [Product upgrades](PRODUCT_UPGRADES.md).
Real projects, broader migration, and platform shipping still need more verification.
Full capability bounds are in [Roadmap](ROADMAP.md) and [Gap with Ren’Py](RENPY_GAP_ANALYSIS.md).

### 9.1 Product priorities

1. **Close a real-work loop**: the owner or an external author should validate the full
   workflow with a 30-to-60-minute work containing multiple endings, saves, localization,
   audio/video, and custom screens, then record authoring time, build results, and platform
   issues. Commercial projects, public release, and external cross-platform acceptance are
   outside the current local-development scope and cannot be replaced by synthetic fixtures.
2. **Lower first-use friction**: launcher and SDK assembly plus local end-to-end acceptance
   are complete. Next steps are downloadable per-platform builds, fewer host dependencies,
   and validated signing, notarization, and installation.
3. **Productize the differentiators**: show source locations, uncovered routes, unreachable
   endings, and localization gaps in the launcher or editor. Before/after route impact
   analysis is not implemented and must not be presented as current capability.
4. **Establish a stability window**: release candidates should state compatibility bounds
   for scripts, structured protocols, and saves, publish upgrade notes, and use the docs
   workflow plus versioned local full verification as release evidence.
5. **Keep scope controlled**: do not prioritize Live2D, 3D, complex shaders, cloud accounts,
   build farms, or a large online dashboard before real productions require them.

### 9.2 Agent collaboration

The deterministic runtime, headless checks, story graph, and route tests are suitable for
agent use. Agent support extends the authoring toolchain; it does not replace basic authoring
experience, engine reliability, or real-production validation. RenRS remains a visual novel
engine first. The useful agent role is a verifiable technical production assistant, not an
engine-level story generator.

Existing machine-interface foundations are `renrs-check --json`, structured results from
`renrs-debug inspect/test/explore`, JSON acceptance reports from `renrs-accept`, and Web
engine `inspect()`. The next step is consistent exit codes, error codes, and versioned JSON
Schemas across those existing binaries. Combining them into one `renrs <subcommand>` binary
is not a prerequisite. Unimplemented commands and interfaces must be labeled as proposals.

The ownership boundary is:

- the agent interprets intent, proposes a change plan, and edits files;
- RenRS returns real project state and analyzes an explicit project version, candidate patch, or Git diff;
- checks, route tests, captures, and builds reuse core libraries or stable CLIs rather than diverging in editor or MCP layers;
- mutating interfaces identify target files, return reviewable diffs, and preserve user approval boundaries.

The target loop is "read real state -> plan and edit -> static check -> route tests -> captures
and coverage report -> user review -> build." See the [roadmap](ROADMAP.md#agent-collaboration-proposal)
for implementation order and capability bounds.
