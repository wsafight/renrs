# Migrate from Ren'Py

The RenRS migrator is an offline source converter. It does not load Ren'Py, execute
Python, read `.rpyc` or Ren'Py saves, or promise identical behavior after conversion.

```sh
cargo run --bin renrs-migrate -- path/to/renpy/game migrated-game
cargo run --bin renrs-migrate -- --strict path/to/renpy/game migrated-game
```

Input may be a `.rpy` file or a project directory. The output directory must be
empty or missing. Directory migration recursively converts `.rpy`, copies ordinary
assets, ignores hidden directories, `cache`, `saves`, `tl`, and `.rpyc`, and always
writes `migration-report.json`.

## Automatic conversion

- `define config.name` and static `Character(...)` declarations.
- Parameterless labels, narration, character dialogue, and unconditional basic menus.
- Static `scene`, `show ... at left/center/right`, `show ... as alias`, and simple `hide`.
- Simple `$ variable = expression` and `default variable = expression`.
- `if`, `elif`, `else`, static `jump`, static `call`, and `return` with no value.
- Basic `play music`, `play sound`, `stop music`, and `pause`.
- `with fade` / `with dissolve` become `transition fade 0.5` under documented assumptions.
- Simple `[variable]` in dialogue becomes `{variable}`.

Scene and sprite names are matched against source image filenames. For example
`scene bg room` may match `images/bg_room.jpg` or `images/bg room.png`. If nothing
is found, an assumed path `images/bg_room.png` is generated and recorded as an
`assumption` in the report.

## Needs a human

- Python blocks, `init python`, ordinary `init`, and arbitrary Python expressions.
- Screen language, styles, custom displayables, and UI actions.
- ATL/transform blocks, dynamic image expressions, custom transitions.
- Label parameters, call arguments, return values, dynamic jump/call.
- Conditional/dynamic menus, complex Character, complex interpolation.
- Custom layers, zorder, behind, camera, video, and plugin statements.
- Implicit fallthrough of Ren'Py labels; RenRS implicitly returns/ends at a label.

Unsupported source lines are kept as `# TODO migration:` comments and recorded as
`unsupported` in the report. They are not executed or silently dropped. Some of
those capabilities (label parameters and RenRS transforms) can be written by hand
in RenRS; the migrator does not guess Ren'Py semantics.

## Report and strict mode

`migration-report.json` contains:

- `converted_files`: converted script count.
- `copied_resources`: copied ordinary asset count.
- `issues`: conversion issues with file, line, and `assumption` / `unsupported`.
- `post_validation_diagnostics`: diagnostics from re-parse, asset check, compile,
  and CFG analysis after conversion.

Default mode still writes the result and report when issues exist, so you can fix
them in steps. `--strict` still writes first, then fails if `issues` or post
validation diagnostics are non-empty. Use it in CI and batch migration checks.

## Recommended review

1. Migrate a copy into a new empty directory. Do not modify the original Ren'Py project.
2. Handle every `unsupported` and `assumption` in the report.
3. Add a stable `config id`, and give key dialogue and menus explicit translation IDs.
4. Run `renrs-fmt`, `renrs-check`, and `renrs-migrate --strict` side by side.
5. Accept every entry, menu branch, return path, and save/load point in the story.
6. Listen to audio separately. Default volume is `0`; automated tests do not judge sound.

The migrator only maps syntax that is provably safe. Full capability gaps are in
[Gap with Ren’Py](RENPY_GAP_ANALYSIS.md).
