# Quick start

RenRS is a visual novel engine written in Rust. It uses its own `.rns` story format.
Run the bundled project first, then create yours. The current version is `0.1.0-rc.1`.
It is not a stable release; older scripts and saves are not guaranteed.

By the end of this page, you will have a project that passes checks, opens in the
player, and builds into a native shipping directory. Follow the sections in order on
your first visit. Use [Scripting language](SCRIPTING.md) later as a reference rather
than reading every document up front.

## Set up the environment

Install Rust 1.88 or later, then enter this repository. The native player needs a
working display and audio output. Linux builds also need ALSA development libraries;
on Debian / Ubuntu install `libasound2-dev`.

```sh
rustc --version
cargo build --bins
cargo build -p renrs-player --bin renrs
```

The web player and workspace need Node.js. The official docs site needs Node.js 22.12
or later. Streaming video conversion and native playback also need FFmpeg. Image-only
stories do not.

## Run the sample

```sh
cargo run --bin renrs-check -- demo
cargo run -p renrs-player --bin renrs -- demo
```

The first command checks syntax, asset refs, and control flow. The second opens the
player. Backgrounds, characters, audio, and translations ship with the repository.
A development directory hot-reloads after a successful check. Saves and preferences
go to the system user data directory.

## Create a story

Generate a project in a directory that does not exist yet:

```sh
cargo run --bin renrs-init -- my-story --title "My Story" --id org.example.my-story
cargo run -p renrs-player --bin renrs -- my-story
```

`config id` is the isolation key for game data. Use a different ID for each project.
The template includes images, Chinese translations, branching endings, and route
tests. The data-interaction template is `--template inventory`.

## Write the first line

Open the generated `my-story/script.rns` first. Project `.rns` files use four-space
indentation. This is a minimal structure that can run on its own. If you replace the
template story with it, update the route assertions in the bundled `routes.json` too.

```text
config title "My Story"
config id "org.example.my-story"

define mira = character "Mira" color "#4CC9A0"
default visits = 0

label start:
    set visits = visits + 1
    mira "Welcome. This is visit {visits}."
    menu:
        "Stay a little longer":
            mira "There is another story to tell."
        "Leave":
            "Until next time."
    return
```

More dialogue, variables, camera, and audio syntax is in [Scripting language](SCRIPTING.md).
Layouts and controls are in [Screens and interaction](SCREENS.md).

## Check and build

```sh
cargo run --bin renrs-check -- my-story
cargo run -p renrs-player --bin renrs -- my-story
cargo run --bin renrs-debug -- test my-story my-story/routes.json
cargo run --bin renrs-build -- my-story dist/my-story
```

Check the project first, then use the player to confirm the actual interaction.
`renrs-debug test` runs the assertions in `routes.json`; keep them in sync when story
branches change. The build output directory must not already exist. The shipping folder
contains the player and a `game.renrs` archive; the player must be built for the target system.

## Where to go next

| Goal | Read |
| --- | --- |
| Continue with dialogue, branches, variables, camera, and audio | [Scripting language](SCRIPTING.md) |
| Customize menus, save screens, HUDs, and data controls | [Screens and interaction](SCREENS.md) |
| Use the visual workspace or package an SDK | [Workspace and SDK](LAUNCHER.md) |
| Build for native, web, or mobile | [Command-line tools](TOOLING.md) and [Release contract](RELEASE.md) |
| Convert an existing Ren'Py project | [Migrate from Ren'Py](MIGRATION.md) |

RenRS does not execute `.rpy`, Python, or Ren’Py saves. For existing works, start
with [Migrate from Ren’Py](MIGRATION.md).
