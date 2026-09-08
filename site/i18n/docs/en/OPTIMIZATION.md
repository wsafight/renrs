# Sequential optimization notes

This round shipped in the following order. Each item is accepted on code, tests, and
docs.

- [x] Player experience: full history scrolling, immediate language choice,
  manual/quick/auto save browsing, long-dialogue paging. 97 automated tests passed.
  Window screenshots are accepted together at the end.
- [x] Performance: save-list cache, 64 KiB buffered streaming pack, background image
  decode, texture/audio cache budgets, asset hot reload. 100 automated tests passed.
- [x] Authoring entry: `renrs-init` and VS Code LSP/diagnostics/preview/run/build.
  A local VSIX was rebuilt with the license; it has not been accepted in a VS Code
  window.
- [x] Story debugging: `renrs-debug inspect/record/replay/test/explore`. Dual
  endings, missing/extra choices, state assertions, explore caps, and post-ending
  save restore tests passed.
- [x] Screen customization: `screens.json` declarative text/image/button/slider/list
  and row/column layout, plus controlled actions. 106 automated tests passed.
- [x] Scale and shipping: 100-chapter, 10,000-line release benchmark; 109 automated
  tests, strict Clippy, fmt/check; macOS at two window sizes; directory/archive and
  release package startup accepted.

Full commands, measurements, and limits are in [Local validation](VALIDATION.md).
Local output:

- `target/validation/benchmark.json`: current release-mode baseline. Compile about
  56–84 ms; 10,102 story interactions about 52 ms.
- `target/validation/demo-*`, `visual-*`: 1280x720/800x600 window captures for built-in
  and custom screens.
- `target/validation/package-800`, `package-release-800`: shipping-package startup
  captures and snapshot restore from `/private/tmp`.
- `target/validation/release-distribution/`: a startable release-mode sample with
  engine and font licenses.
- `editors/vscode-renrs/renrs-0.1.0-rc.1.vsix`: an installable editor extension.

Six native runs produced 81 PNGs, all reported pass. Captures include the second
page of long dialogue, 12 long choices, history ends, Chinese switch, custom
slider/list, and manual/quick/auto saves. Later work added custom history cache,
very long label layout, container style misuse checks, save-cache Send/Sync, and
shipping-package lookup that does not depend on the working directory.

Authoring entry is in [Command-line tools](TOOLING.md), route assertions in
[Story debugging](DEBUGGING.md), screen format in [Screens and interaction](SCREENS.md).
Template/route, VSIX, and window checks now run through the layered local gates; GitHub
Actions retains only documentation deployment. Remote three-platform acceptance and
human audio listening were not run in this round.

Performance bounds: texture cache target 256 MiB; audio cache estimated from decoded
size, target 64 MiB. The current stage and playing audio are not force-evicted, so
active resources themselves may still exceed the target. The image background queue
is bounded. GPU upload and the current audio backend still decode on the main thread.
Save UI no longer reads disk every frame; external file changes appear within one
second; explicit load always rechecks. Local large-save write is about 144 ms and
first six-slot read about 263 ms, still synchronous. Dialogue page numbers are not
in the save yet.
