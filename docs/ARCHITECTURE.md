# Cargo Workspace

The workspace separates compilation, execution, project storage and editor integration.
The root `renrs` package is the library and CLI tools. The native player is `renrs-player`
and still exposes the `renrs` binary.

| Crate | Responsibility | Direct Internal Dependencies |
| --- | --- | --- |
| `renrs-syntax` | AST, values, spans, diagnostics, text markup and localization data | None |
| `renrs-model` | Compiled instructions/IDs and the runtime-facing project bundle | syntax |
| `renrs-compiler` | Parsing, lowering, static analysis and catalog extraction | syntax, model |
| `renrs-runtime` | Interpreter, current-format snapshots, hot reload, rollback, profiles, debugger and route exploration | syntax, model, extensions |
| `renrs-project` | Project assembly, resources, archives, themes, screens and video manifests | syntax, model, compiler, extensions |
| `renrs-editor` | Symbols, references, formatting, story graphs and LSP server | syntax, compiler, project |
| `renrs-web` | WASM bindings and expression parsing at the browser boundary | syntax, compiler, runtime |
| `renrs` (root) | Library facade, save repository, distributions and CLI entry points | syntax, model, compiler, runtime, project, editor |
| `renrs-player` | Native presentation, audio, fonts and the `renrs` player binary | root `renrs` |

`CompiledProgram` is the pure script output. `ProjectBundle` adds extensions, compiled
layered images and progress configuration; `Program` remains its compatibility name.
Runtime depends on this model directly. It does not load files, initialize graphics or
invoke the parser during execution. Native and Web adapters parse interactive expressions
into `Expr` before calling runtime APIs.
Editor integration can be built and tested without the native player or audio stack.
Project resource rules are shared by desktop loading, archive creation, watching,
Web distribution and editor disk indexing.

The VS Code extension source stays in `editors/vscode-renrs/src` and compiles to a
CommonJS bundle; its Rust LSP is in `crates/editor`. The strict TypeScript browser UI
stays in `web`; only its WASM interface is Rust. The Launcher uses a separate Vite shell
with Zod-validated local API responses while its Node server remains an `.mjs` script.
The root library is the public facade used by the player and CLI. Graphics and audio
backends stay in `renrs-player`, so `renrs-check` and other CLI tools do not compile
macroquad or rodio. Leaf crates do not re-export parser and syntax modules from their
upstream dependencies.
Pre-release APIs and storage formats may change without backward compatibility.
There is no catch-all core crate and no cross-crate source-path inclusion.

```sh
cargo test --workspace --exclude renrs-player --all-targets
cargo test -p renrs-player --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -p renrs-compiler
cargo test -p renrs-runtime
cargo test -p renrs-editor
cargo build --bins
cargo build -p renrs-player --bin renrs
cargo build -p renrs-web --target wasm32-unknown-unknown --release
```

All workspace packages share one root `Cargo.lock`. The source-size check covers
every Rust crate, not generated dependencies or build artifacts.
