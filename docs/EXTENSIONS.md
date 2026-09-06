# Project Extensions

Extensions run in the shared native/WASM Rust runtime using Rhai. A module is a
pure function of immutable `input`; its last expression is returned as story data.
Each call gets a fresh scope. Imports, eval, floating point, clocks and filesystem
access are unavailable. Budgets: 100,000 operations, 32 call levels, 4096 collection
entries and 1 MiB strings; returned data also passes the story value budget.

```json
{"version":1,"modules":{"inventory.reward":"extensions/reward.rhai"}}
```

```rhai
let next = input;
next.push("map");
next
```

```text
extend bag = "inventory.reward" bag
```

```json
{"type":"extension","text":"Collect reward","name":"inventory.reward","variable":"bag","input":"bag"}
```

Module source is compiled once per runtime, included in the program fingerprint
and packaged in directory/archive/Web builds. Editing a module changes the build
identity. Results participate in normal saves/rollback. Failed screen calls commit
no changes. Host integrations can call `Runtime::invoke_extension` or
`Runtime::apply_extension_expression`; external side effects belong in platform
adapters, not deterministic story functions.
