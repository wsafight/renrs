# RenRS authoring guide

This is a task-oriented syntax reference, not a linear tutorial. Complete the
[Quick start](QUICKSTART.md) before your first project, then jump directly to the
section that matches the work at hand.

| Task | Section |
| --- | --- |
| Organize files and declare characters, images, and variables | [Project layout](#project-layout), [Config and declarations](#config-and-declarations) |
| Write dialogue, rich text, NVL, and paged text | [Dialogue, interpolation, and text tags](#dialogue-interpolation-and-text-tags) |
| Control sprites, animation, transitions, and video | [Stage and transform](#stage-and-transform) |
| Build variables, conditions, menus, and reusable labels | [Variables and expressions](#variables-and-expressions), [Conditions and menus](#conditions-and-menus) |
| Configure audio, localization, themes, and fonts | [Audio and pause](#audio-and-pause), [Stable IDs and localization](#stable-ids-and-localization) |
| Diagnose scripts and understand hot reload | [Check and hot reload](#check-and-hot-reload) |

## Project layout

```text
my-game/
  script.rns
  story/
    chapter-1.rns
  images/
  audio/
  fonts/
  locales/
    zh-Hans.json
  theme.json
```

RenRS recursively reads `.rns` files in non-hidden directories, sorts them by relative
path, and merges them. All files share config, characters, images, display layers, default variables,
and the label namespace. A project must define exactly one `start` label. Asset paths
are relative to the project root. Absolute paths, `..`, and other escapes are rejected.

The player and headless tools accept a directory or a `.renrs` archive. Only directory
mode watches script changes. An archive is a read-only shipping input.

## Config and declarations

```text
config title "My Story"
config id "org.example.my-story"

define e = character "Eileen" color "#ef6a6a"
default score = 0
default player_name = "Reader"
image room = "images/room.png"
image eileen = "images/eileen.png"
layer effects order 50
```

`config id` should be a stable ASCII reverse-domain or slug. It decides settings, read
state, and the save directory. Changing it after release makes the player treat the
project as a different game. If omitted, it is derived from the title; that is only
for prototypes.

`default` evaluates in declaration order at new-game start. Colors support `#RRGGBB`
and `#RRGGBBAA`. A static `image` lets later commands use a name instead of a path:
`scene room`, `show eileen`. `master`, `transient`, `screens`, and `overlay` are built-in
display layers. Declare a custom layer with `layer name order integer`; lower orders draw first.

## Dialogue, interpolation, and text tags

```text
label start:
    e "Hello, {player_name}. Score: {score}."
    "Narration has no speaker."
    e "A {b}bright{/b} {color=#ef8b72}signal{/color}.{br}It is moving."
```

Value types are integers, booleans, strings, lists, and records. `{name}` inserts a
variable in dialogue. Collections display as JSON. `{{` and `}}` emit literal braces.
Tags include `{b}...{/b}`, `{color=#RRGGBB}...{/color}`, `{color=#RRGGBBAA}...{/color}`,
`{br}`, `{u}...{/u}`, and `{ruby=annotation}...{/ruby}`. Ruby is 1 to 64 characters and
cannot nest. Tags do not count as typewriter characters. Native ruby shrinks to the
body fragment width; split long annotations into short words.

`nvl on` enables full-page multi-line narration, `nvl clear` clears the page, and
`nvl off` returns to ordinary dialogue. Page bounds restore with saves and rollback.
A page holds at most 256 segments; long pages can still paginate or scroll. Native
accessibility settings provide self-voicing, toggled with F8. macOS uses `say`,
Windows uses system speech, Linux needs `espeak-ng`. Web uses SpeechSynthesis; voices
depend on what the system installed.

## Stage and transform

```text
label start:
    scene room
    show eileen as hero at right onlayer effects zorder 10
    move hero to center over 0.4
    transform hero x 24 y -12 scale 1.1 rotate 5 alpha 0.9 over 0.5 ease in_out
    transform hero anchor 0.5 1 crop 0 0 600 900
    transform hero uncrop
    hide hero
    clear effects
```

`scene` replaces the background and clears sprites. `show` accepts a static image name
or a quoted path. Positions are `left`, `center`, `right`. `onlayer` selects a named
sprite layer and `zorder` is a 32-bit order within it; smaller values draw first. The
legacy `layer 10` spelling still means `zorder 10`. Sprite aliases remain unique across
the stage. `clear name` removes only that sprite layer. Layer identity, order, sprites,
and clears survive save and rollback. Hot reload refreshes changed layer orders and
transactionally rejects deleting a custom layer that is still visible.

`transform` can combine:

- `x` / `y`: pixel offset from the base position.
- `scale`: uniform scale in `0.01..=20`.
- `rotate`: degrees.
- `alpha`: `0..=1`.
- `anchor x y`: two normalized anchors in `0..=1`.
- `crop x y width height` / `uncrop`: texture-pixel crop or clear crop.
- `over seconds`: animation duration; omitted means apply immediately.
- `ease linear|in|out|in_out`: interpolation, default `linear`.

Transforms, position tweens, fade, and dissolve enter snapshots, load, and rollback.
Full Ren'Py ATL, per-layer cameras, split background layers, and arbitrary displayables
are not supported.

A `timeline:` block can serialize transform, move, and pause. `transition dissolve seconds`
blends the previous and next stage. `video "clips/name/clip.json" over seconds` accepts
image-frame manifests (v1) and MP4/WebM streaming manifests (v2). Duration must match
the manifest. Use `renrs-video input.mp4 my-project clips/name --stream` for a streaming
version. Conversion needs FFmpeg and FFprobe. Native streaming playback needs FFmpeg;
web uses the browser video player. The converter may extract `audio.wav`; the audio
clock drives play, pause, and load. Older manifests without audio still work. Video
manifests may also configure language-selected `audio_tracks`, per-track relative
volume, and embedded `subtitles`; native and Web players share selection fallback and
validation bounds. Video volume uses the sound channel. Commands, examples, and media limits are in
[Capability upgrades](UPGRADES.md#media-and-screen-controls).

Parallel staging uses 2 to 16 independent timelines. The same alias cannot be mutated
by more than one track. Each track has at most 256 steps:

```text
parallel:
    timeline:
        transform left_actor x 160 over 1.5
    timeline:
        pause 0.25
        transform right_actor alpha 0.5 over 1.25
```

Tracks only accept `transform` and `pause`. Total duration is the longest track.
A mid-timeline save restores from that progress on desktop and web. Character
precomposition is in [Product upgrades](PRODUCT_UPGRADES.md#character-composition).

## Variables and expressions

```text
set score = score + 1
set ready = score >= 1 and not false
set greeting = "Hello, " + player_name
```

Supported operators are `+ - * /`, comparison, equality, `and`, `or`, `not`, and
parentheses. `+` can concatenate two strings. Expressions cannot call files, network,
Python, or Rust.

```text
default bag = list("key")
default quest = record("done", false, "reward", 20)
set bag = push(bag, "map")
set quest = put(quest, "done", contains(bag, "map"))
set reward = get(quest, "reward")
```

Built-ins: `list(...)`, `record(key, value, ...)`, `get(data, key[, fallback])`,
`put(data, key, value)`, `push(list, value)`, `remove(data, key)`, `len(data)`, and
`contains(data, value)`. List indexes start at 0. Record keys are strings. `contains`
checks keys on records. Updates return a new value; catch it with `set`. Duplicate
keys, wrong types, and out-of-range access error. `get` may supply a missing value.
Built-in results and screen variable updates are capped at 4096 values, 16 collection
levels, and 1 MiB of text. Expressions are capped at 512 tokens and 32 parenthesis
levels. Collections enter saves and rollback like ordinary variables.

## Conditions and menus

```text
if score >= 2:
    e "High score."
elif score == 1:
    e "One point."
else:
    e "No points."

menu:
    "Continue" id "choice.continue" if ready:
        jump next_scene
    "Wait" id "choice.wait":
        e "Take your time."
```

Each `menu` declares at least two options. An option with a false `if` is hidden.
Even if only one option is visible at runtime, it is still a menu. Conditions must
produce booleans.

A menu may declare a narrated or spoken prompt in its header. The prompt remains
visible with the options and does not add another continue interaction:

```text
menu "Choose a route":
    "Left":
        jump left
    "Right":
        jump right

menu e "What should we do?":
    "Stay":
        return
    "Leave":
        jump outside
```

## Label parameters and return values

```text
label start:
    call add_score(score, amount=2)
    e "New score: {_return}."
    return

label add_score(current, amount=1):
    return current + amount
```

Required parameters must precede parameters with defaults, and positional arguments
must precede named arguments. Named arguments may be reordered. The compiler rejects
unknown, duplicate, missing, and excess arguments. A default expression is evaluated
only when that parameter was not supplied.

Every explicit argument and used default in one `call` is evaluated against the
caller's current variable table before any parameter is bound. A later argument or
default therefore cannot observe a parameter newly bound by the same call. Parameters
dynamically shadow variables with the same names. Returning restores the prior values
or their unassigned state. `start` cannot declare parameters, and `jump` cannot enter a
parameterized label; use `call`.

`return expr` is evaluated before parameters are restored, then writes the result to
`_return` and returns to the caller. `return` without an expression leaves `_return`
unchanged. An empty return stack ends the game. The end of a label also implicitly ends
or returns.

## Audio and pause

```text
play music "audio/theme.ogg" loop fadein 0.5 volume 0.7
queue music "audio/next.ogg" volume 0.6 fadein 0.25
play sound "audio/click.wav" volume 0.5
voice "audio/line-001.wav"
pause 0.5
stop music fadeout 0.8
```

Music, sound, and voice are independent channels. First-run defaults are `0.6`,
`0.8`, and `1.0`. `play music`, `queue music`, and `play sound` accept one static
`volume 0..1`; this relative gain multiplies the player's channel volume and music
state preserves it in saves. It is not an arbitrary mixer. Put `voice` before the matching dialogue; it stops when dialogue
advances. Non-looping WAV/Ogg music can advance the queue after the real duration is
parsed. Duration is never invented when it cannot be determined. Audio state restores
with saves and rollback. Current automated tests do not listen to sound.

## Stable IDs and localization

Dialogue can declare a stable translation ID before the statement, and register old
ID aliases:

```text
@id "intro.hello" alias "chapter1.old_hello" e "Hello, {player_name}."

menu:
    "Continue" id "intro.continue":
        jump next_scene
    "Wait" id "intro.wait":
        return
```

If none is declared, the compiler generates a structured ID. Explicit IDs are used
for localization and development hot reload. `alias` can map preview-session positions
when renaming. The project is not released; saves only accept the current script
fingerprint and do not require cross-version restore. IDs are 1 to 128 ASCII
characters: alphanumerics, `_`, `-`, `.`, `/`, and `:`.

Extract translations:

```sh
cargo run --bin renrs-i18n -- extract game zh-Hans game/locales/zh-Hans.json
cargo run --bin renrs-i18n -- update game game/locales/zh-Hans.json
cargo run --bin renrs-i18n -- check game game/locales/zh-Hans.json
```

Catalog example:

```json
{
  "language": "zh-Hans",
  "fallback": "zh",
  "messages": {
    "intro.continue": "继续",
    "intro.hello": "你好，{player_name}。"
  }
}
```

The player reads `locales/*.json`. Language prefers the project player setting, or
`RENRS_LANGUAGE=zh-Hans` for the initial language. Missing and empty translations
fall back to source text. Translation text is chosen first, then interpolation and
markup parse.

## Theme and fonts

The project root may provide `theme.json`. It supports font paths, size, colors, high
contrast, reduced motion, and fixed layouts for the main menu, toolbar, dialogue box,
and save list. `font_path` and `font_fallbacks` must be safe in-project relative
paths. Native picks fonts by glyph coverage, with built-in Arabic, Devanagari, and
CJK fallbacks. Web loads in the same order. `music_volume`, `sound_volume`, and
`voice_volume` set the three mix defaults. See `demo/theme.json` for fields.

A localization catalog may declare `{ "count": "items", "forms": { "one": "...", "other": "..." } }`
under `plurals`. Runtime picks zero/one/two/few/many/other by CLDR rules, falling
back to `other` when a form is missing.

## Check and hot reload

```sh
cargo run --bin renrs-check -- path/to/my-game
cargo run --bin renrs-check -- path/to/game.renrs
```

Checks cover indentation, syntax, colors, cross-file declarations, label and argument
refs, assets, translation identity, unreachable story, path-sensitive unassigned
variables, and immediate loops with no interaction.

The directory player checks `.rns` changes every 250 ms. A new script replaces the
running program only after parse, validation, compile, analysis, and stable ID mapping
all succeed. On failure the old program keeps running, and audio commands that already
passed are not replayed. Archives do not hot-reload.

Stable IDs relate to relative paths, labels, and structural position. Moving a file,
renaming a label, or reordering sibling statements may change automatic IDs. Add
explicit IDs on key interactions to keep preview position after edits. When hot reload
cannot map reliably, preview restarts. Persistent saves only support the current
script version; acceptance does not include old-save migration.
