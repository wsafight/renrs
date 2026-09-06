# Cargo Workspace

The workspace separates compilation, execution, project storage and editor integration.
The root `renrs` package assembles the native player and command-line tools.

| Crate | Responsibility | Direct Internal Dependencies |
| --- | --- | --- |
| `renrs-syntax` | AST, values, spans, diagnostics, text markup and localization data | None |
| `renrs-compiler` | Parsing, lowering, compiled Program/IDs, static analysis and catalog extraction | syntax |
| `renrs-runtime` | Interpreter, current-format snapshots, hot reload, rollback, profiles, debugger and route exploration | compiler |
| `renrs-project` | Project assembly, resources, archives, themes, screens and video manifests | compiler |
| `renrs-editor` | Symbols, references, formatting, story graphs and LSP server | compiler, project |
| `renrs-web` | WASM bindings to the shared runtime | runtime |
| `renrs` (root) | Native presentation, audio, save repository, distributions and CLI entry points | compiler, runtime, project, editor |

`Program` is the compiler/runtime contract. Runtime depends on its compiled model;
it does not load files, initialize graphics or invoke the parser during execution.
Editor integration can be built and tested without the native player or audio stack.
Project resource rules are shared by desktop loading, archive creation, watching,
Web distribution and editor disk indexing.

The VS Code JavaScript extension stays in `editors/vscode-renrs`; its Rust LSP is
in `crates/editor`. The browser UI stays in `web`; only its WASM interface is Rust.
The root library re-exports the module paths used by the player and CLI.
Pre-release APIs and storage formats may change without backward compatibility.
There is no catch-all core crate and no cross-crate source-path inclusion.

```sh
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -p renrs-compiler
cargo test -p renrs-runtime
cargo test -p renrs-editor
cargo build --bins
cargo build -p renrs-web --target wasm32-unknown-unknown --release
```

All workspace packages share one root `Cargo.lock`. The source-size check covers
every Rust crate, not generated dependencies or build artifacts.
