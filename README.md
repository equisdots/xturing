# xturing

Rust/ratatui TUI that exposes **every option of the equisdots settings panel**
(the one bound to `SUPER+SHIFT+D`), reading and writing the same
`~/.config/hypr/settings.json` and calling the same scripts.

**The name**: `x` as in execute/control from the terminal + **Alan Turing**,
the machine that runs your instructions. It is a second way to drive the
system: it does not modify or depend on changes in `shell/`, `hyprland/`,
`palettes/` or any other repo. The QML panel keeps working as before and both
can coexist (same atomic write contract).

## Status

- **25 pages** across the 6 rail groups: Shell (General, Timex, Keyboard,
  Monitors, Startup), Bar (Engine, Position, Style, Zones, Classic Bar,
  Modules, Workspaces), Theme (Palette, Animations, Shadows, Glass, Mascots),
  Behavior (Launcher, Notifications), Widgets (Popups) and System (Hyprland,
  Input, GPU, Idle, About).
- **Search palette** (`/` or `Ctrl+P`): fuzzy match across all pages, options,
  palettes, zones, keybinds, startup commands and quick commands (reload,
  switch engine, help, quit). Enter jumps straight to the option.
- **Mouse and touch**: click the rail to switch pages, click a row to select,
  double-tap to open/run, click or drag the stepper `[-]`/`[+]` zones, wheel
  to move the selection.
- **Arrow navigation**: `↑`/`↓` walk the whole menu, crossing into the
  previous/next page at the edges; `Ctrl+←`/`→` (or `[`/`]`) change page
  directly. The per-page selection is remembered when you come back.
- **Shortcut recorder**: in the keybind editor press `r` and then the actual
  key combination to fill mods + key.
- **CLI flags**: `--help`, `--version`, `--dry-run`, `--settings`, `--palettes`,
  `--page`, `--search` (see below).
- **22 tests** (`cargo test`) covering atomic writes, merges that preserve
  unknown keys, the full catalog, engine switching with `classicbar` seeding,
  zones, keybinds, ClassicBar grouping and search.
- Real PTY smoke tests: navigate, adjust, pick a palette, search and click the
  rail; everything writes/renders correctly.

### Page coverage

| Page | What it includes |
|---|---|
| Shell · General | uiScale, appScale (runs `scale-menu.sh`), workspaceCount (+`queueReload`), xkb layouts, layout shortcut, wallpaper dir, open wallpaper picker |
| Shell · Timex | provider (open-meteo/wttr/openweather), unit, city (+refresh), API key (`timex keys set`), key test, all ~21 forecast/clock/calendar/day-panel layout knobs |
| Shell · Keyboard | full keybind list (add/edit/delete, duplicate validation) and **regenerates `config/user-keybinds.lua` + `hyprctl reload`** like the panel |
| Shell · Monitors | reads `hyprctl -j monitors`, edits resolution/rate/rotation/VRR/bitdepth/cm/mirror/position, **Apply** (settings.monitors + display-config + `hyprctl eval`) and Reset to auto |
| Shell · Startup | command list and **regenerates `config/user-startup.lua` + reload** |
| Bar · Engine | bar/classic selector with `classicbar` seeding from the zones, mirror, classic defaults, 14-popup position matrix |
| Bar · Position | `bar.position` / `classicbar.position` depending on the engine |
| Bar · Style | presets (apply a flag bundle), roundness, thickness, edgeGap, opacity, fills, dragModules, font, time/date formats, borders + full **WindowBordersSection** (palette-follow, hex, gradients, angles) |
| Bar · Zones | zones (add/delete), align, unify, container bg + color + solid, border width/color, module chips with enable/disable and movement (Shift+←/→) within and across zones, Center all, Default |
| Bar · Classic Bar | style/timeFormat/distinctPills/autohide/roundness/thickness/opacity/width/hide-delay, left/center/right/available sections with reorder (`↑↓`), move section (`m`), group (`g`), ungroup (`u`), mirror and defaults |
| Bar · Modules | global iconColor + the 18 modules with icon/color/accent/fill and extras (clock format and size, typewriter effect, cursor, date format, workspaces marker and 7 color slots) |
| Bar · Workspaces | marker (numbers/dots/letters/custom) and custom character |
| Theme · Palette | full list from `index.json` (x/custom/user) with filter, selection (`bar.palette`), **editing of the 18 slots** of the palette file with backup and reset, create (from the active palette) and delete |
| Theme · Animations | enabled + speed presets/stepper; writes `user-animations.lua` with `luac -p` + reload via `persist-hypr.sh` |
| Theme · Shadows/Glass/Mascots | every knob (7 shadows, 2 glass, mascots species/position/count/size) |
| Behavior · Launcher | position, sizes, margins, borders, avoidBar, showIcons, alignment |
| Behavior · Notifications | width/maxHeight/shadowBlur/shadowOffset/shadow/6 positions and DND |
| Widgets · Popups | the 16 `Personalization.js` sections with `pretty()` labels and respected types (bool/toggle, floats with decimal step, etc.) |
| System · Hyprland | the 12 `hypr-effects.sh` sliders (live preview with `hyprctl eval`, `apply` persist on exit), Reset, Refresh, window borders |
| System · Input | sensitivity, accel profile, tap-to-click, natural scroll, disable-while-typing; `persist-hypr.sh input` + reload |
| System · GPU | integrated/hybrid/nvidia mode via `gpu-mode.sh`, current mode via `envycontrol --query` |
| System · Idle | Auto/Awake mode via `idle-mode.sh`, status, manual lock/suspend |
| System · About | sysinfo, copy debug info, `dots doctor`, updater, docs, report issue |

## Install

```sh
# part of the equisdots stack: clone + build + ~/.local/bin/xturing
dots install

# or standalone
cargo install --path . --root ~/.local
```

## Usage

```sh
xturing                        # installed in ~/.local/bin
xturing --page d_style         # open a page directly
xturing --search palette       # open the search palette pre-filled
xturing --dry-run              # no external commands (logs to /tmp/xturing-actions.log)
xturing --help                 # all options and keys
```

| Option | Effect |
|---|---|
| `--settings <path>` | alternative `settings.json` |
| `--palettes <dir>` | alternative palettes directory |
| `--page <id>` | open a page directly (`s_general`, `d_style`, `d_palette`...) |
| `--search <query>` | open the search palette pre-filled |
| `--dry-run` | same as `XTURING_DRY=1` |
| `-h, --help` / `-V, --version` | help / version |

For development: `cargo run` inside this repo. Local checks (no remote CI):
`scripts/check.sh` runs `cargo fmt --check`, `clippy -D warnings` and the tests.

### Testing without touching anything (recommended)

```sh
# 1) dry mode: no external command runs, they are logged instead
#    (/tmp/xturing-actions.log)
XTURING_DRY=1 xturing

# 2) settings sandbox: does not even touch your real settings.json
cp ~/.config/hypr/settings.json /tmp/settings-test.json
XTURING_DRY=1 XTURING_SETTINGS=/tmp/settings-test.json xturing

# 3) palette sandbox (for the Palette page)
XTURING_DRY=1 \
XTURING_SETTINGS=/tmp/settings-test.json \
XTURING_PALETTES_DIR=/tmp/palettes-test \
xturing
```

Environment variables:

| Variable | Effect |
|---|---|
| `XTURING_SETTINGS` | alternative `settings.json` path |
| `XTURING_PALETTES_DIR` | alternative palettes directory (`index.json` + files) |
| `XTURING_DRY=1` | runs no commands; logs them to `/tmp/xturing-actions.log` |

## Keys

```
↑↓ / j k      move selection (wraps across pages at the edges)
←→ / h l      adjust / cycle          Tab / Shift+Tab  next / previous page
Ctrl+←/→ [ ]  previous / next page    1..6             jump to group (Shell, Bar, Theme, Behavior, Widgets, System)
Enter/Space   edit, toggle, choose an option, run an action
/ or Ctrl+P   search palette          r                reload settings.json
?             help                    q / Ctrl+C       quit (flushes pending effects)
a / d / w     add / delete / save lists (keybinds, startup)
r (in form)   record a shortcut by pressing it
Shift+←/→     move a module across zones (Zones)
m / g / u     move section / group / ungroup (Classic Bar)
```

Mouse / touch (the terminal must report mouse events, e.g. foot, kitty or
wezterm; on a tablet a tap is a click):

```
click         select row (on the rail: switch page)
double-tap    open / run the row
[-] / [+]     click or drag to adjust steppers
wheel         move selection up / down
```

## Write contract

- `settings.json` is written with atomic `tmp + rename` and `serde_json`,
  preserving key order and **unknown keys**.
- Writes are immediate (the QML panel debounces 220 ms; the end result is the
  same).
- Control characters in `bar.modules` (glyph icons) are read/written as-is,
  without sanitizing.
- The shell notices external changes through its watchers
  (`FileView`/`inotify`), so edits show up live without touching it.

## Structure

```
src/
  settings.rs   atomic read/write + tests
  catalog.rs    data-driven definition of the 25 pages and their controls
  app.rs        state, navigation, editing, special pages, side effects
  ui.rs         ratatui rendering (rail, content, chooser, forms, help)
  palette.rs    index.json, palette editing with backup, create/delete
  classic.rs    ClassicBar defaults/mirrorBar/normalize
  monitors.rs   hyprctl monitors, apply/reset with lua + display-config
  actions.rs    spawn/capture (with dry mode), notify-send
scripts/check.sh  local fmt + clippy + test gate
```

## Known differences vs. the QML panel

- **Zones**: no drag & drop; the equivalent is `Shift+←/→` (within and across
  zones) and Enter to enable/disable.
- **ClassicBar**: group/ungroup with `g`/`u` instead of dragging.
- **Monitors**: the drag canvas is replaced by Position X/Y fields.
- **Palette**: colors are edited as hex (`#rrggbb`) instead of a swatch
  picker; "New palette" clones the 8 base colors of the active palette (the
  panel lets you edit the draft before creating).
- **General**: the xkb layout list is a text field (any valid code is
  accepted) instead of suggestions.
- The Hyprland *Refresh* button works here (`hypr-effects.sh read`); in the
  current QML panel it calls a method that does not exist.

None of this changes the settings file or its format: these are interaction
differences, not contract differences.
