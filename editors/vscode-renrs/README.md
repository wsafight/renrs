# RenRS for VS Code

This local extension connects to the existing RenRS command-line tools. It has
syntax highlighting, LSP navigation/formatting/completion, project diagnostics,
image/audio previews, and commands to initialize, run, build, and test stories.
The RenRS Project view in Explorer also migrates Ren'Py projects, publishes migration
issues at original `.rpy` locations, inspects variables/reachability/coverage, compares
project impact, emits the story graph, opens screens and themes, builds Web distributions,
runs release acceptance, and checks current-build saved games.
Acceptance results appear in the RenRS output channel.

Build the tools with `cargo build --bins`. Set `renrs.toolsPath` to the absolute
`target/debug` directory, or place the release tools on PATH. Open the game folder;
when opening the engine repository instead, set `renrs.projectPath` to `demo`.
For Web builds, set `renrs.webShellPath` to the built `web/dist` directory or the
release package's `web-shell` directory. If unset, the extension looks for
`web-shell` beside the resolved tool binary.

Run `npm install` and `npm run build` here to type-check and generate
`dist/extension.js`. Run `npm run package` to produce a local VSIX, then use
VS Code's Install from VSIX command. Reload VS Code after changing the tool or
project path if the language server does not restart with the new configuration.

Project checks use on-disk files; save changes before checking. Route tests use
`routes.json` at the project root. Resource preview does not play audio automatically.
Run, build, route tests, and acceptance commands save existing documents first.
This project is pre-release and does not preserve old saves across script builds.
Explicit story IDs support localization and live preview reloads; save files must
match the current compiled script.
This extension requires a trusted workspace and has not been published to a marketplace.
