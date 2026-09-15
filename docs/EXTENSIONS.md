# Project Extensions

Extensions run in the shared native/WASM Rust runtime using Velin 0.4.0. A module
is a pure function of immutable `input` and must finish with `perform return(value)`
or may stop deliberately with `perform fail(message)`. Each call resets an isolated VM;
idle VM workspaces are reused across calls and concurrent calls never share mutable state.
Other host commands and the `random`/`chance` built-ins are rejected at compile
time; floating point, clocks and filesystem access are unavailable. Each call has
a fixed 10,000 immediate-step budget. Input and output are also limited to 4096
values, 16 collection levels and 1 MiB of text.

```json
{"version":2,"modules":{"inventory.reward":"extensions/reward.velin"}}
```

```velin
set next = push(input, "map")
perform return(next)
```

```text
extend bag = "inventory.reward" bag
```

```json
{"type":"extension","text":"Collect reward","name":"inventory.reward","variable":"bag","input":"bag"}
```

## Runtime Model

Manifest version 2 accepts only `.velin` modules. Module source is compiled once
per runtime, included in the program fingerprint and packaged in directory,
archive and Web builds. Editing a module changes the build identity. Results
participate in normal saves and rollback. Failed screen calls, including an
explicit `fail`, commit no changes. Host integrations can call
`Runtime::invoke_extension` or `Runtime::apply_extension_expression`; external
side effects belong in platform adapters, not deterministic story functions.

Each compiled module lazily keeps up to four idle `MachineInvoker` instances.
An invocation briefly locks the pool to take and return an invoker, but runs the
VM without holding that lock. If concurrent callers exhaust the pool, each gets
an independent machine. Every machine restarts from the validated initial frame
before binding `input`, so successful, failed and concurrent calls cannot leak
state into one another.

RenRS stores the owned `MachineInvoker` rather than the borrowing
`PureModuleInvoker<'_>`. This avoids a self-referential structure while
preserving `Extensions::invoke(&self)`, cloneability and concurrent use. Input
slots and the `return` / `fail` host IDs are resolved once; the single-argument
host path avoids per-call input maps and one-element host vectors. The measured
Rhai-to-Velin results and reproduction commands are in
[Performance](PERFORMANCE.md#velin-04-extension-boundary).
