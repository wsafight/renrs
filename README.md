# RenRS

**English** · [中文](README.zh-CN.md)

[Docs](https://wsafight.github.io/renrs/)
· [Quick start](https://wsafight.github.io/renrs/start/quickstart/)
· [中文文档](https://wsafight.github.io/renrs/?lang=zh)

RenRS is a visual novel engine written in Rust. Write story in `.rns`, check it,
then run a native player or ship a read-only archive. It is inspired by Ren'Py,
but it does not execute Python, run `.rpy`, or load Ren'Py saves.

> Release candidate `0.1.0-rc.1`. The v1 machine/archive/screen contracts are frozen;
> saves require the
> current container and snapshot formats; content updates additionally require
> explicit stable `@id`/`alias` positions wherever execution is active.

## Try the demo

Needs Rust 1.88+. Linux also needs ALSA (`libasound2-dev` on Debian / Ubuntu).

```sh
cargo run --bin renrs-check -- demo
cargo run -p renrs-player --bin renrs -- demo
```

The player takes a project directory or a `.renrs` archive. Directory mode
hot-reloads after a successful check. Archive mode is read-only. Saves and
preferences go to the system user data directory, keyed by `config id`.

With no project argument, the player looks next to itself and in the current
directory for `game.renrs` or `demo/`.

## Create a project

```sh
cargo run --bin renrs-init -- my-story --title "My Story" --id org.example.my-story
cargo run --bin renrs-debug -- test my-story my-story/routes.json
cargo run -p renrs-player --bin renrs -- my-story
```

The template includes art, Chinese translations, two endings, and route tests.
Use a different `config id` for each game. `--template inventory` is the
data-interaction starter.

Minimal script:

```text
config title "My Story"
config id "org.example.my-story"

define mira = character "Mira" color "#4CC9A0"

label start:
    mira "Welcome."
    menu:
        "Stay":
            mira "There is another story to tell."
        "Leave":
            "Until next time."
    return
```

## Ship

```sh
cargo run --bin renrs-pack -- demo demo.renrs
cargo build --bin renrs-build
cargo build -p renrs-player --bin renrs
cargo run --bin renrs-build -- demo dist/signal-at-dusk
```

`renrs-build` writes a player, `game.renrs`, a manifest, and licenses into a new
directory. Web and mobile flows are in the [toolchain guide](https://wsafight.github.io/renrs/reference/cli/).

## Documentation

The [docs site](https://wsafight.github.io/renrs/) is English by default. Append
`?lang=zh` or use the 中文 control to switch; the choice is stored in the browser.

| Topic | English | 中文 |
| --- | --- | --- |
| Quick start | [Site](https://wsafight.github.io/renrs/start/quickstart/) · [Source](docs/QUICKSTART.md) | [Site](https://wsafight.github.io/renrs/start/quickstart/?lang=zh) |
| Scripting | [Site](https://wsafight.github.io/renrs/guides/scripting/) · [Source](docs/SCRIPTING.md) | [Site](https://wsafight.github.io/renrs/guides/scripting/?lang=zh) |
| Screens | [Site](https://wsafight.github.io/renrs/guides/screens/) · [Source](docs/SCREENS.md) | [Site](https://wsafight.github.io/renrs/guides/screens/?lang=zh) |
| CLI | [Site](https://wsafight.github.io/renrs/reference/cli/) · [Source](docs/TOOLING.md) | [Site](https://wsafight.github.io/renrs/reference/cli/?lang=zh) |
| Web and mobile | [Site](https://wsafight.github.io/renrs/shipping/mobile/) · [Source](docs/MOBILE.md) | [Site](https://wsafight.github.io/renrs/shipping/mobile/?lang=zh) |
| Gap with Ren'Py | [Site](https://wsafight.github.io/renrs/project/comparison/) · [Source](docs/RENPY_GAP_ANALYSIS.md) | [Site](https://wsafight.github.io/renrs/project/comparison/?lang=zh) |

Also: [VS Code extension](editors/vscode-renrs/README.md),
[story debugging](https://wsafight.github.io/renrs/guides/debugging/),
[architecture](https://wsafight.github.io/renrs/engine/architecture/).
The checked compatibility matrix and RC gates are in [docs/RELEASE.md](docs/RELEASE.md).

## Capabilities

- Declarative `.rns` for characters, dialogue, menus, transforms, audio, and NVL
- Check before run: assets, control flow, definite assignment, unreachable story
- Recoverable state: snapshot v8 (v7 saves remain readable), rollback, checksummed saves, desktop/web exchange
- `screens.json` layouts, `theme.json`, JSON catalogs, `renrs-i18n`
- Headless tools: check, fmt, graph, LSP, debug, pack, build, migrate
- Web (WASM) and Capacitor Android/iOS packaging

First-run volumes are music `0.6`, sound `0.8`, voice `1.0`. Existing mute
preferences are kept.

## Layout

```text
crates/syntax         AST, values, diagnostics, text, localization
crates/model          Compiled program and project bundle contracts
crates/compiler       Parse, lower, static analysis
crates/runtime        Execute, snapshot, rollback, debugger
crates/project        Sources, assets, archives, theme, screens
crates/editor         LSP, symbols, format, story graph
crates/extensions     Sandboxed deterministic extension execution
crates/web            Shared runtime WASM bindings
crates/player          Native UI, render, audio (`renrs` binary)
src/migration/*       Supported Ren'Py static subset
src/bin/*             Headless CLI entry points
```

Every Rust file in source and tests must stay under 500 lines (`tests/source_size.rs`).
Crate boundaries: [architecture](docs/ARCHITECTURE.md).

## Develop

```sh
node scripts/verify-local.mjs

cargo fmt --all -- --check
cargo check --offline --workspace --all-targets
cargo clippy --offline --workspace --all-targets --all-features -- -D warnings
cargo test --offline --workspace --all-targets

# Native muted capture. The output directory must not exist.
cargo run -p renrs-player --bin renrs -- demo --smoke-test target/demo-captures --window-size 800x600
```

Use `node scripts/verify-local.mjs --full` for the Web browser, VS Code host,
release binary, and SDK packaging checks. It expects the Web and editor npm
dependencies, Rust WASM target, `wasm-bindgen`, and browser tooling to be installed.

The Ren'Py research copy lives in ignored `references/renpy`. See
[upstream research](docs/RENPY_RESEARCH.md). Bundled fonts use SIL OFL 1.1
(`assets/fonts/OFL.txt`).
