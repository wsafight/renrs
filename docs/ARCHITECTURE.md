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

The expression boundary is shared with Velin 0.5.1: `renrs-syntax` re-exports its value,
expression, operator, built-in and diagnostic types; `renrs-compiler` uses its bounded parser
and conservative checker; `renrs-runtime` uses its reference evaluator. A compatibility
adapter removes expression source spans before storing the AST, preserves literal square
brackets in RenRS strings, and rejects `random` / `chance`. Story statements, instruction
IDs, waiting, reload, rollback and persistence remain RenRS-owned.

## Velin Ownership Boundary

| RenRS crate | Direct Velin dependency | Owned responsibility |
| --- | --- | --- |
| `renrs-syntax` | `velin-syntax` | Re-export `Value`, `Expr`, string parts, operators, built-ins and diagnostics |
| `renrs-compiler` | `velin-parse`, `velin-check` | Parse and conservatively check story, screen and layered-image expressions |
| `renrs-runtime` | `velin-eval` | Evaluate expressions and immutable list/record built-ins |
| `renrs-extensions` | `velin` | Compile and execute bounded deterministic `.velin` pure modules |

Only the extension crate depends on Velin's full facade and VM. The other crates
use the narrowest language sub-crate they need, avoiding an accidental VM
dependency in syntax, compilation or ordinary story evaluation.

The shared expression path covers `.rns` defaults, label parameter defaults,
`call` arguments, `return`, `set`, extension inputs, branch/menu conditions,
`screens.json` visibility and updates, layered-image conditions, project
validation and debugger evaluation. These consumers all traverse the same Velin
AST and share the same value semantics.

The boundary stops at pure language and computation. The `.rns` parser and
visual-novel statements, stable instruction IDs, waits, hot reload, rollback,
save containers, screen layout, native/Web UI, audio and rendering remain owned
by RenRS. Moving them into Velin would duplicate the host protocol and obscure
ownership of persistent and presentation state; they are not migration
candidates while these boundaries remain distinct.

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
