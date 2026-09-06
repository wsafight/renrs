# Quick start

RenRS is a visual novel engine written in Rust. It uses its own `.rns` story format.
Run the bundled project first, then create yours. The current version is `0.1.0`.
It is not a stable release; older scripts and saves are not guaranteed.

## Set up the environment

Install Rust 1.88 or later, then enter this repository. The native player needs a
working display and audio output. Linux builds also need ALSA development libraries;
on Debian / Ubuntu install `libasound2-dev`.

```sh
rustc --version
cargo build --bins
```

The web player and workspace need Node.js. The official docs site needs Node.js 22.12
or later. Streaming video conversion and native playback also need FFmpeg. Image-only
stories do not.

## Run the sample

```sh
cargo run --bin renrs-check -- demo
cargo run --bin renrs -- demo
```

The first command checks syntax, asset refs, and control flow. The second opens the
player. Backgrounds, characters, audio, and translations ship with the repository.
A development directory hot-reloads after a successful check. Saves and preferences
go to the system user data directory.

## Create a story

Generate a project in a directory that does not exist yet:

```sh
cargo run --bin renrs-init -- my-story --title "My Story" --id org.example.my-story
cargo run --bin renrs -- my-story
```

`config id` is the isolation key for game data. Use a different ID for each project.
The template includes images, Chinese translations, branching endings, and route
tests. The data-interaction template is `--template inventory`.

## Write the first line

Project `.rns` files use four-space indentation. This is a minimal story that can
run on its own:

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
cargo run --bin renrs-debug -- test my-story my-story/routes.json
cargo run --bin renrs-build -- my-story dist/my-story
```

The build output directory must not already exist. The shipping folder contains the
player and a `game.renrs` archive; the player must be built for the target system.
Web and mobile have separate flows, see [Command-line tools](TOOLING.md) and
[Web and mobile](MOBILE.md).

RenRS does not execute `.rpy`, Python, or Ren’Py saves. For existing works, start
with [Migrate from Ren’Py](MIGRATION.md).
