# Gap with Ren'Py

## Comparison scope

This analysis uses the local research copy `references/renpy` at commit `9ed7dd3`
(2026-09-04). It focused on script/AST, execution context, display lists, screen
language, ATL, save/rollback, localization, audio, video, predicted load, and shipping
modules. RenRS borrows architecture bounds and user workflow. It does not copy
upstream code.

The project is not released. Breaking changes are allowed. Old-version compatibility
and save migration are not product gaps or acceptance conditions.

Status: **done** means the current P0/P1 acceptance scope is implemented. **partial**
means only a static or constrained subset. **not implemented** means docs and the
migration report must reject it or list it as todo.

| Capability | RenRS | Current bound | Later priority |
| --- | --- | --- | --- |
| Script parse and static diagnostics | done | Multi-file `.rns`, source positions, cross-file checks, CFG | Expand diagnostics from real projects |
| Dialogue, menus, conditions, jumps | done | Conditional menus, interpolation, basic rich text | More text tags and input |
| Label parameters and returns | done | Positional/named/default args, call-time defaults, dynamic parameter scope, `return expr`, `_return` | `*args`, `**kwargs`, and keyword-only parameters are outside the static subset |
| Python/store semantics | not implemented | Integers, booleans, strings, lists, records, deterministic built-ins | Python ecosystem remains a non-goal |
| Image declarations and stage | partial | Static image, background, global aliases, ordered/clearable named sprite layers, camera, constrained layered image | Layer cameras, arbitrary displayables, Live2D, and particles still missing |
| ATL and transform | partial | Transform, easing, serial timeline, independent parallel tracks | Full ATL, dynamic layeredimage, blink/lipsync still missing |
| Transition | partial | Serializable fade, dissolve | Composite transitions still missing |
| Screen/style/UI | partial | Shared desktop/web JSON screens, scroll, data controls, drag/drop, extension buttons | Arbitrary displayables and full screen language still missing |
| Audio | partial | music/sound/voice, music queue, fades, static music/sound relative gain | Arbitrary mixers, sync, and more formats are P2 |
| Video | partial | Image frames or streaming video, optional WAV bed, pause/resume on the audio clock | Extra audio/subtitle tracks and more platform measurement still pending |
| Reading and a11y | partial | NVL, underline, ruby, RTL shaping, keyboard controls, self-voicing | Shaped-cluster wrapping, vertical text, color emoji, and full screen-reader trees still missing |
| Localization | done | JSON catalog, stable IDs, alias, fallback, CLDR plurals, RTL shaping, font families | Full ICU rich text and screen-reader trees still missing |
| Saves | done | Snapshot v7, async saves, explicit-ID content-update restore, desktop/web container exchange and load acceptance | Old-format migration and cloud sync still missing |
| Rollback/read/auto/skip | done | Bounded checkpoints, independent profile, rollback barrier | Fixed rollback and advanced preference sync still missing |
| Archives and shipping | partial | Signed delta patches, local app and store config tools | Shipping credentials and external acceptance still pending |
| Editor tools | partial | Separate editor crate, LSP, VS Code grammar, launcher, templates, SDK pack | Visual story authoring and a full debugger still missing |
| Predicted load and large-project work | partial | Background media, incremental compile, shared Program/history, bounded decode, frame/RSS measurement | External large works still need verification |
| Web/mobile | partial | Rust/WASM, shared screens, Capacitor Android/iOS build and system share | Mobile uses WebView; native Rust mobile rendering, device matrix, and store acceptance still missing |
| Live2D/particles/3D/shader | not implemented | No matching runtime | P2, evaluate per project |

## P0/P1 conclusion

P0 covers the loop “project directory or archive -> check/run -> persist -> build and
ship”. Scripts, theme, fonts, images, audio, and translations in an archive are read
through `ProjectSource`. Saves and settings do not write back into a read-only game
directory.

P1 covers parameterized calls, stable localization identity, recoverable transform,
three-channel audio state, workspace LSP, and strict migration reports for a mid-size
static visual novel. Defaults are audible; automated tests stay muted. Listening still
needs a human.

These two priorities complete a reliable independent Rust engine baseline, not a
Ren'Py replica. Screen language, full ATL, the Python ecosystem, visual authoring,
platform services, and external shipping acceptance remain the main gaps. Latest
implementation and tests are in [Product upgrades](PRODUCT_UPGRADES.md).

## Entry rules for new features

- New script syntax must update parser, validator, compiler, runtime, migrator, LSP, and docs together.
- Persistable state must define current-version read/write, hot reload, and rollback. Old-format migration is not required.
- New media formats must use a reliable decoder and be verified on target-platform CI. Fail clearly when decode is impossible.
- Uncertain Ren'Py migration must emit `assumption` or `unsupported`. Do not silently change semantics.
- New Rust modules split by a single duty. No `.rs` file in `src/` or `tests/` may exceed 500 lines.
