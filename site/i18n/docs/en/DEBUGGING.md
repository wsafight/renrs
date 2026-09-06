# Story debugging and route tests

`renrs-debug` uses a headless Runtime. It can run without a GPU, audio device, or
player settings. Every command accepts a project directory or a `.renrs` archive
and runs the unified project check first.

## Inspect and record

```sh
cargo run --bin renrs-debug -- inspect demo
cargo run --bin renrs-debug -- record demo first-route.json
cargo run --bin renrs-debug -- replay demo first-route.json
```

`inspect` advances to the first interaction and prints the current source location,
chapter, variables, call stack, wait state, and history length. `record` shows
dialogue and choices in the terminal. Type `next` or press Enter to advance, a
1-based choice number to pick, `state` to inspect, and `quit` to cancel. A file is
written only when an ending is reached. Existing files are not overwritten.

A recording stores the project ID, content fingerprint, 0-based choice indexes,
stable choice IDs, ending label, and final variables. Replay checks those
constraints. Re-record after content changes. A handwritten route may omit
`fingerprint` and use state assertions for cross-version regression. If
`choice_ids` is present, its length must match `choices`.

## Route assertions

The `renrs-init` template includes a `routes.json` that can run as-is:

```json
{
  "routes": [
    {
      "name": "answer",
      "choices": [0],
      "expect_label": "answer",
      "expect_variables": { "trust": 1 },
      "expect_dialogue": "Someone answers"
    },
    {
      "name": "wait",
      "choices": [1],
      "expect_label": "wait",
      "expect_variables": { "trust": 0 }
    }
  ]
}
```

```sh
cargo run --bin renrs-debug -- test my-story my-story/routes.json
```

`expect_variables` checks final values of named variables. `expect_dialogue` checks
whether history contains the given text. The ending label is the last instruction
that actually ran; a `return` that crosses a label boundary does not misreport.
Missing choices, extra choices, failed assertions, fingerprint mismatch, or more
than 10,000 interactions return a non-zero exit. The library API `run_route` can
set a larger interaction cap.

## Bounded branch exploration

```sh
cargo run --bin renrs-debug -- explore my-story
cargo run --bin renrs-debug -- explore my-story --max-runs 256 --max-steps 20000 --max-depth 48
```

Defaults are 128 routes, 10,000 interactions each, and choice depth 32. The report
includes reached endings, instruction coverage, run errors, and truncated branches.
When a cap is hit, `complete` is `false` and the command fails. That is not the
same as a story bug, and it is not the same as every branch being verified. Use
explicit route assertions for loops and combinatorial explosions.

These tools verify story state. They do not verify glyphs, image display, animation
smoothness, or how audio sounds. Native window acceptance is in
[Local validation](VALIDATION.md).
