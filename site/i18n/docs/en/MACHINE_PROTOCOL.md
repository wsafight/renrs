# Machine protocol

RenRS headless tools use a versioned JSON envelope shared by the launcher, editors,
agents, and a future thin MCP layer. The current protocol is v1. Its schema is
[`schemas/machine-protocol-v1.schema.json`](../schemas/machine-protocol-v1.schema.json).

## Top-level shape

```json
{
  "protocol_version": 1,
  "command": "inspect",
  "ok": true,
  "data": {},
  "diagnostics": [],
  "error": null
}
```

- `protocol_version`: version of the envelope and common diagnostic shape.
- `command`: stable producer ID such as `check`, `debug.test`, `accept`, `inspect`, or `impact`.
- `ok`: whether the command met its completion condition. Failed checks, routes, and incomplete exploration are false.
- `data`: command-specific result; a failure may retain useful partial results.
- `diagnostics`: project diagnostics with a code, severity, and source location.
- `error`: stable command error code and human-readable message; null on success.

Exit codes are fixed: `0` success, `1` project/validation/runtime failure, and `2`
invalid arguments. Consumers must check `protocol_version` first, then interpret `data`
by `command`; they must not depend on field order or human-readable messages. v1 may add
optional object fields, but does not remove or change existing field semantics. Breaking
changes require a new `protocol_version`.

## Current commands

```sh
renrs-check --json <project|archive>
renrs-debug inspect <project|archive>
renrs-debug replay <project|archive> <route.json>
renrs-debug test <project|archive> <routes.json>
renrs-debug explore <project|archive>
renrs-accept <project|archive> [--saves <directory>]
renrs-inspect <project|archive>
renrs-impact <baseline> <candidate>
renrs-impact --git <candidate-directory> [base-ref]
```

`renrs-debug record` is an interactive human command and does not use the machine
envelope. Its `state` output is also terminal-oriented.

## Project inspection

`renrs-inspect` returns project ID, script fingerprint, format versions, scripts,
characters, variables, labels, endings, resources, localization status, and route
coverage. Static reachability and route coverage are separate: the former comes from the
control-flow graph, the latter from actual `routes.json` replay. `referenced_by_story`
only means an asset is referenced by story or progress configuration, not visually accepted.

## Impact analysis

`renrs-impact` compiles the baseline and candidate, then compares labels, route outcomes,
ending reachability and coverage, localization, and save-structure risk. `--git` reads the
candidate directory's committed files from a Git revision, defaulting to `HEAD`; untracked
files are absent from the baseline.

Save risk is a conservative static report, not a compatibility guarantee. After a content
fingerprint change, continue to test representative real saves:

```sh
renrs-accept <candidate> --saves <save-fixtures>
```
