# Upstream Ren'Py research

## Research copy and license bounds

- Local path: `references/renpy`
- Current commit: `9ed7dd3`
- Commit date: 2026-09-04
- Upstream: the official Ren'Py source repository

The directory is local research only. It is not part of RenRS builds or distribution.
Ren'Py itself is mostly MIT; some components and dependencies have their own licenses.
See upstream `sphinx/source/license.rst`. RenRS does not copy the Python/Cython
implementation. It records architecture ideas, behavior bounds, user workflow, and
test inspiration, then reimplements with independent Rust types.

## Reading focus

- `renpy/lexer.py`, `parser.py`, `ast.py`, `script.py`, `execution.py`: indent syntax, AST, node identity, execution context.
- `renpy/display/scenelists.py`, `display/core.py`, `display/render.pyx`: display lists, interaction loop, render bounds.
- `renpy/display/transform.py`, `atl.py`, `display/transition.py`: transform, ATL, and transition lifetime.
- `renpy/loadsave.py`, `rollback.py`, `persistent.py`: save container, rollback log, cross-save persistent state.
- `renpy/translation/`: translation identity, language catalogs, string replacement.
- `renpy/audio/`, `display/video.py`: media channels, queues, video lifetime.
- `renpy/display/predict.py`, `loader.py`: predicted load and asset access.
- `renpy/screenlang.py`, `renpy/sl2/`, `style.pyx`: screen language, UI AST, style system.
- `renpy/lint.py`, `editor.py`, `scriptedit.py`: author diagnostics and editor workflow.

## Independent design after that reading

### Script and execution

Ren'Py script nodes both express semantics and take part in jump and save restore.
RenRS keeps parse-then-execute, but compiles the AST to flat instructions. Runtime
uses indexes; persistence uses SHA-256 stable instruction IDs. CFG analysis, headless
tests, and version work can sit on a compact `Program`.

### Interaction loop

Ren'Py switches between execution context and UI interaction. RenRS exposes dialogue,
menus, pause, and end through `WaitState`. The player only consumes effects and input;
it does not own story control flow. Audio is also events from Runtime, adapted by the
player onto a real backend.

### Display state

Ren'Py supports named layers, displayable trees, camera, ATL, and complex transitions.
RenRS saves one background, named sprite layers ordered explicitly and then by z-order,
and a transform per sprite; each sprite layer can be cleared independently. It does not
provide layer cameras or arbitrary displayables and is not described as full ATL or screen support.

### Saves and rollback

Ren'Py saves an object graph, rollback log, screenshots, and JSON metadata. RenRS
saves a small explicit snapshot and does not use pickle. v8 snapshots record execution
position, variables, an interned stage table, dynamic call frames, language, history,
and bounded full checkpoints; v7 snapshots remain readable. Older snapshot formats are rejected. The outer save adds
project/content identity, play time, chapter, and SHA-256.

Read state and player settings are not in a single snapshot. They are project-level
data. That is close to Ren'Py persistent/preferences in intent, with an independent
data structure and compatibility format.

### Localization

Ren'Py folds translation nodes and languages into the script system. RenRS uses an
independent `TranslationId` and JSON catalogs, splitting translation identity from
execution IDs and read IDs. Language choice does explicit and structural fallback
before interpolation and markup parse, so translations can move placeholders.

### Project and shipping

Ren'Py's loader gives directories and archives a unified asset view. RenRS matches
that with `ProjectSource`: directories for authoring and hot reload, checked `.renrs`
for read-only shipping. Settings, read state, and saves go to the system user data
directory, so a shipping archive does not need to be writable.

## Explicit differences

- RenRS does not execute Python, and is not compatible with `.rpyc`, Ren'Py saves, or the Python plugin ecosystem.
- RenRS uses `.rns`. The migrator is only an offline static-subset converter.
- RenRS currently has built-in themeable UI, not screen language, full style, or a displayable system.
- RenRS transform is constrained property animation, not ATL. Named sprite layers do not include Ren'Py's layer-camera/displayable model.
- RenRS supports serializable fade/dissolve, predictive loading, streamed video, a Web player, and Capacitor mobile builds. Combined transitions, Live2D, shaders, and native Rust mobile rendering remain unimplemented.
- RenRS rollback stores bounded full checkpoints. It does not implement Ren'Py's rollback object-diff log and full fixed-rollback semantics.
- RenRS audio covers music/sound/voice, basic queue/fade, and static music/sound relative gain. Arbitrary mixers are unsupported. Player defaults are audible; automated tests stay muted.

Full status and later priorities are in [Gap with Ren’Py](RENPY_GAP_ANALYSIS.md) and
[Roadmap](ROADMAP.md).

## Later research rules

1. Pin and record the upstream commit before updating a comparison, so “current Ren'Py behavior” stays reproducible.
2. For each borrowed idea, record the source module, observed behavior, RenRS independent design, and incompatibility bound.
3. Do not copy implementation. If upstream assets or code must be introduced later, review license and distribution duties first.
4. New migration rules need real input samples, a structured report, and a failure path. Do not replace grammar with string guessing.
5. Heavy media such as video must pick a reliable backend and verify three platforms before it enters product support.
