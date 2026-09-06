# Declarative screens

A project root may include `screens.json` to replace `main_menu`, `save`, `load`,
`settings`, `history`, `dialogue`, and `choices`, and to overlay a read-only `hud`.
Undeclared screens keep the built-in versions. Directory hot reload and `.renrs`
archives both support the file. This is RenRS's constrained JSON format. It does not
execute Ren'Py screen language or arbitrary code.

## Minimal example

```json
{
  "version": 1,
  "styles": {
    "title": { "font_size": 48, "text_color": "#FFFFFF" }
  },
  "main_menu": {
    "bounds": { "x": 70, "y": 100, "width": 430, "height": 520 },
    "root": {
      "type": "column",
      "gap": 20,
      "children": [
        { "size": 160, "type": "text", "style": "title", "text": "{title}" },
        { "size": 64, "type": "button", "text": "New game", "action": "new_game" },
        { "size": 64, "type": "button", "text": "Settings", "action": "settings" },
        { "size": 64, "type": "button", "text": "Quit", "action": "quit" }
      ]
    }
  }
}
```

## Layout and style

Screen coordinates use a 1280x720 logical canvas. The window scales uniformly and
keeps letterboxing. `bounds` is the whole screen region. `row` and `column` use
`gap`, `padding`, and `children`. A child's `size` is the main-axis length; omitted
sizes split remaining space. Overflow of fixed lengths, non-finite numbers, empty
containers, more than 64 direct children, or leaving the canvas is rejected. Each
screen has at most 256 elements and nesting depth 16.

`style` names a top-level `styles` entry. It may set `font_size` (12–72),
`text_color`, and `background_color`. Styles apply to leaf controls only and do not
inherit. Declaring a style on a row or column is an error. Text wraps and shrinks in
the constrained region; tails that still do not fit at the minimum size are ellipsized.
Dialogue body uses separate paging.

## Controls

| `type` | Fields | Behavior |
| --- | --- | --- |
| `text` | `text` | Text with current story variables, `{title}`, `{chapter}` |
| `image` | `path` | Project-relative path, shown whole at aspect |
| `button` | `text`, `action` | Calls a constrained player action |
| `dialogue` | none | Dialogue region; only on the dialogue screen, exactly one; at least 240x140 |
| `choices` | none | Choice region; only on the choices screen, exactly one; at least 200x64 |
| `set` | `text`, `variable`, `expression` | Button runs a deterministic variable update; dialogue/choices/settings only |
| `slider` | `text`, `setting` | Edits a preference; needs at least 200x64 |
| `toggle` | `text`, `setting` | Toggles `high_contrast`, `reduced_motion`, `wait_voice`, or `self_voicing` |
| `input` | `text`, `variable`, `max_length` | Edits a declared string variable; a game must have started |
| `list` | `source`, `item_height`, `gap` | Bounded display, wheel, and paging; at most one list per screen |
| `viewport` | `id`, `content_height`, `child` | Nested scroll region; native and web keep independent scroll and clip |
| `data_list` | `variable`, `selected`, `item_height`, `label` | Bounded choices from a list variable, written to a string variable |
| `drag` / `drop` | `expression` / `variable` | Constrained drag-and-drop; touch, mouse, and keyboard share assignment |
| `extension` | `name`, `input`, `variable` | Calls a pure Rhai module and writes back transactionally |
| `row` / `column` | `children`, `gap`, `padding` | Row and column layout |

`setting` supports `text_speed`, `auto_delay`, `music_volume`, `sound_volume`,
`voice_volume`. Initial volumes are 0.6, 0.8, and 1.0. Settings save to the player
data directory.

List sources: `manual_saves`, `quick_saves`, `auto_saves`, `history`, `languages`,
and `saves`. `saves` follows the current group on load screens and uses manual slots
on save screens. `manual_saves` can write on the save screen; other save lists are
read-only. Corrupt saves or saves that are not for the current project cannot load.
History layout is cached by width and font size, and invalidates on story, language,
or theme change. Only visible rows are drawn.

Actions:

- `new_game`, `continue`, `save`, `load`, `settings`, `history`, `collection`.
- `quick_save`, `quick_load`, `rollback`, `auto`, `skip`.
- `manual_saves`, `quick_saves`, `auto_saves`: open and switch the load group, used
  with the `saves` list.
- `close`: save preferences and close the current screen. `quit`: exit the player.

The title screen must provide a `new_game` button. HUD cannot have buttons, sliders,
toggles, inputs, or writable lists; only text, images, and read-only history.
Screens use keyboard focus and the mouse. History and long lists support
Home/End/PageUp/PageDown.

## Full example and limits

[`examples/visual_screens.json`](../examples/visual_screens.json) combines six screens
and can generate a playable fixture:

```sh
cargo run --example generate_visual_fixture -- target/visual-story
cargo run -- target/visual-story
```

Desktop and web share the Rust-validated layout. Web reflows leaf controls in order
on a phone-sized viewport. Native dialogue paginates; web body can scroll. NVL uses a
dedicated reading page. History lists on web load 50 rows per page so long works do
not create an unbounded DOM. Screen copy may use project `ui.*` translations, with
built-in fallbacks for common Chinese labels. Diagnostic text is not fully localized.

```json
{"type":"set","text":"Pack map","variable":"bag","expression":"push(bag, \"map\")"}
```

The variable must already be declared, and types must match before and after the
update. It can run only while stopped on dialogue or a menu. If the expression or
menu condition fails, the original variable, profile, and checkpoint stay. A
successful update enters the current checkpoint and can restore from later saves and
rollback. There are no arbitrary host callbacks, nested viewports beyond the
supported widget, or Ren'Py displayables.
