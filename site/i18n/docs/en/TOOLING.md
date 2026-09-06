# RenRS toolchain

Except the `renrs` player, every tool is a headless command for editors, CI, and
shipping. Examples below use `cargo run` in the source repository. A shipping package
can call the same binary names directly.

## Check and format

```sh
cargo run --bin renrs-check -- game
cargo run --bin renrs-check -- game.renrs
cargo run --bin renrs-fmt -- game
cargo run --bin renrs-fmt -- --check game
```

`renrs-check` accepts a directory or `.renrs` and runs load, asset validation, compile,
and control-flow analysis. `renrs-fmt` processes `.rns` in a directory: trailing
whitespace, consecutive blank lines, and a final newline. It does not change
four-space semantic indent or delete comments. `--check` writes nothing.

## Create a project and the editor

```sh
cargo run --bin renrs-init -- my-story --title "My Story" --id org.example.my-story
cargo run --bin renrs-debug -- test my-story my-story/routes.json
```

The target directory must not exist. The template includes a background, sprite, two
routes, a Chinese catalog, a theme, and title/HUD screens. After installing the local
[VS Code extension](../editors/vscode-renrs/README.md), you get LSP, project
diagnostics, asset preview, run, build, and route-test commands. Set `renrs.toolsPath`
to the tools directory. The RenRS Project panel in the explorer also opens screens and
theme, web build, release acceptance, and current-version save checks.

Story inspect, record, replay, and bounded exploration are in [Story debugging](DEBUGGING.md).
Medium-project generation and `renrs-bench` coverage are in [Local validation](VALIDATION.md).

## LSP

```sh
cargo run --bin renrs-lsp
```

`renrs-lsp` uses stdio Language Server Protocol. On initialize it loads `.rns` files in
non-hidden workspace directories. It supports:

- Full document sync and live syntax diagnostics.
- Document symbols for characters, labels, static images, and variables.
- Full-document format.
- Workspace definition, references, and rename.
- Keyword and workspace-symbol completion.

Still run `renrs-check` before shipping; it loads the full project and assets and runs
CFG analysis.

## Story graph

```sh
cargo run --bin renrs-graph -- game story.dot
```

Output is Graphviz DOT. Nodes are labels. Edges distinguish static `jump` and `call`.
Omitting the output path writes stdout.

## Localization

```sh
cargo run --bin renrs-i18n -- extract game zh-Hans game/locales/zh-Hans.json
cargo run --bin renrs-i18n -- update game game/locales/zh-Hans.json
cargo run --bin renrs-i18n -- check game game/locales/zh-Hans.json
```

- `extract` creates or rewrites the catalog for a language. Protect existing
  translations with version control before a rewrite.
- `update` adds empty translations for new IDs and keeps obsolete entries so human
  copy is not silently dropped.
- `check` lists missing/empty translations and obsolete entries. Untranslated items
  fail the command.

All three accept a directory or an archive. The player loads `locales/*.json` automatically.

## Asset archives

```sh
cargo run --bin renrs-pack -- game game.renrs
cargo run --bin renrs-unpack -- game.renrs extracted-game
cargo run --bin renrs-check -- game.renrs
```

`.renrs` contains a versioned JSON manifest and a contiguous asset payload. Each entry
records a canonical relative path, offset, length, and SHA-256. Read and unpack reject
escaping paths, duplicate paths, out-of-range data, and checksum failure. Hidden
directories are not packed. Unpack will not overwrite existing files, and it rejects
symlinks in the target directory, parent directory, or output files.

An archive is not unpack-only. The player, checker, localization tools, and builder
can read it directly. Player archive mode does not hot-reload.

## Build a shipping directory

```sh
cargo build --bin renrs --bin renrs-build
cargo run --bin renrs-build -- game dist/my-game
```

If `renrs-build` is not next to the player, pass it explicitly:

```sh
cargo run --bin renrs-build -- game dist/my-game --player target/debug/renrs
```

The builder validates, compiles, and analyzes control flow, then writes atomically:

```text
dist/my-game/
  renrs              # renrs.exe on Windows
  game.renrs
  renrs-build.json
  README.txt
  LICENSE-renrs.txt
  LICENSE-font-OFL.txt
```

The target directory must not exist. Put it outside the project. A generated player
started with no arguments from any working directory prefers `game.renrs` next to the
player. An explicit project path overrides that default.

## Web and mobile

```sh
node scripts/build-web.mjs
cargo run --bin renrs-web-build -- game dist/web --shell web/dist
node scripts/mobile.mjs dist/web dist/mobile
```

The web shell build needs Node.js, installed `web/` dependencies, the Rust WASM target,
and a matching `wasm-bindgen`. Shipping packages already include `web-shell/` and can
pass it to `--shell`. In VS Code, `renrs.webShellPath` points at that directory. During
source development, point at built `web/dist`.

The mobile script writes a Capacitor project config, then installs dependencies and
adds Android/iOS platforms in the output directory. That directory must not exist.
Mobile runs the Rust/WASM WebView. Environment, steps, and acceptance limits are in
[Web and mobile](MOBILE.md).

## Current-version acceptance

```sh
cargo run --bin renrs-accept -- game
cargo run --bin renrs-accept -- game --saves saved-games
```

The acceptor prints JSON, checks route assertions, and restores real saves for the
current script version. Failure returns non-zero. The project is not released, so old
versions are not required and `--baseline` is not accepted. Test saves should come from
the current build. After script changes you can start a new game. Current policy is in
[Product upgrades](PRODUCT_UPGRADES.md).

## Character precomposition

```sh
cargo run --bin renrs-compose -- game character.json images/variants
```

It writes named PNGs and `images.rns` in layer order from the config. The target is an
in-project asset directory that does not exist yet. This is build-time precomposition.
Examples and limits are in [Product upgrades](PRODUCT_UPGRADES.md#character-composition).

## Ren'Py migration

```sh
cargo run --bin renrs-migrate -- path/to/renpy/game migrated-game
cargo run --bin renrs-migrate -- --strict path/to/renpy/game migrated-game
```

The migrator does not load Ren'Py or execute Python. After conversion it re-parses,
validates, compiles, and analyzes, and writes issues to `migration-report.json`.
`--strict` fails when assumptions, unsupported items, or post-validation diagnostics
exist. Use it in CI. Full scope is in [Migrate from Ren’Py](MIGRATION.md).

## Cross-platform release packages

`.github/workflows/release.yml` builds Linux x86-64, macOS arm64, and Windows x86-64
artifacts on a `v*` tag or a manual run. A toolkit includes:

- `renrs`, `renrs-check`, `renrs-fmt`, `renrs-graph`, and `renrs-lsp`.
- `renrs-i18n`, `renrs-migrate`, `renrs-pack`, `renrs-unpack`, and `renrs-build`.
- `renrs-init`, `renrs-debug`, and `renrs-bench`.
- `renrs-accept`, `renrs-web-build`, `renrs-video`, `renrs-compose`, and `renrs-update`.
- `web-shell/`, `mobile.mjs`, and the VS Code extension VSIX.
- `demo/`, `demo.renrs`, docs, README, engine and built-in font licenses.

The workflow emits unsigned artifacts. It does not include macOS notarization, Windows
signing, an installer, or auto-update.

## Engineering gates

```sh
cargo fmt --all -- --check
cargo check --offline --workspace --all-targets
cargo clippy --offline --workspace --all-targets --all-features -- -D warnings
cargo test --offline --workspace --all-targets
```

`tests/source_size.rs` recursively checks `src/`, `tests/`, and every workspace crate.
Any Rust file over 500 lines fails the test. Modules that outgrow a responsibility
should split into submodules in a same-named directory and keep the public API.

CI also configures template creation, route assertions, branch exploration, VSIX
packaging, Linux Xvfb/Mesa native captures, and starting a shipping package from
outside the project. Window automated acceptance and the current local validation
scope are in [Local validation](VALIDATION.md).
