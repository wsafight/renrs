# Workspace and SDK

The launcher is a local project workspace. It reuses the Rust CLI for check, run,
story graphs, and builds. It provides a project list, script editing, quality reports,
job logs, and SDK path configuration.

## Start from source

```sh
cargo build --bins
npm ci --prefix web
node scripts/launcher.mjs --port 4185
```

Open `http://127.0.0.1:4185`. The server listens on loopback only. Tools default to
`target/debug`. Project and SDK paths are stored in `.renrs/launcher.json` in the
repository. Use `--state` for another config file.

## Project management

Existing projects are registered by directory path. New projects use the standard
story template or the data-interaction template. The create directory must not
already exist. Removing a project from the list only drops the workspace record;
project files stay on disk.

The script view reads `.rns` files. Save checks the fingerprint from load time: if
the file was edited elsewhere, reload first so those edits are not overwritten.
A successful save starts a project check.

The Quality view calls `renrs-inspect` and shows route instruction coverage, passed
routes, statically unreachable labels, missing translations, source diagnostics, and
per-route results. Static reachability and actual route coverage remain separate. Refresh
reads the current content from disk again.

## Build jobs

The workspace can start a native package, a web package, and a `.renrs` archive
build. Each job uses the matching Rust CLI. The output path must not already exist.
Only one job runs at a time. Logs have a size cap, and a running job can be
cancelled. Web builds need the web shell first:

```sh
npm ci --prefix web
node scripts/build-web.mjs
```

The `wasm32-unknown-unknown` target and a matching `wasm-bindgen-cli` must be
installed first. Full commands are in [Command-line tools](TOOLING.md#web-and-mobile).

## Package an SDK

```sh
cargo build --release --bins
node scripts/package-sdk.mjs dist/renrs-sdk target/release
```

The web shell must already exist. The SDK includes tools, the launcher, the web
shell, editor extension sources, and docs. `sdk.json` records version, target
platform, and file checksums. It still requires Node.js 22 or later on the host.
Native binaries match the machine that built them; this is not a universal SDK.

The launcher and SDK are implemented and pass packaged local end-to-end acceptance.
Public downloads, per-platform builds, signing, and notarization still require external
shipping validation; see [Current status](NEXT_PRODUCT_WORK.md).
