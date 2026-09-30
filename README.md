# xturing

TUI en Rust/ratatui que ofrece **todas las opciones del panel de settings de
equisdots** (el que se abre con `SUPER+SHIFT+D`), leyendo y escribiendo el
mismo `~/.config/hypr/settings.json` y llamando a los mismos scripts.

**El nombre**: `x` como metáfora de ejecutar/controlar desde la terminal +
**Alan Turing**: la máquina que ejecuta tus instrucciones. Segunda vía de
interacción con el sistema: no modifica ni depende de cambios en `shell/`,
`hyprland/`, `palettes/` ni ningún otro repo. El panel QML sigue funcionando
igual y ambos pueden convivir (mismo contrato de escritura atómica).

## Estado

- **25 páginas** de los 6 grupos del rail: Shell (General, Timex, Keyboard,
  Monitors, Startup), Bar (Engine, Position, Style, Zones, Classic Bar,
  Modules, Workspaces), Theme (Palette, Animations, Shadows, Glass, Mascots),
  Behavior (Launcher, Notifications), Widgets (Popups) y System (Hyprland,
  Input, GPU, Idle, About).
- **Paleta de búsqueda** (`/` o `Ctrl+P`): fuzzy match sobre todas las páginas,
  opciones, paletas, zonas, keybinds, comandos de startup y comandos rápidos
  (recargar, cambiar motor, ayuda, salir). Enter salta directo a la opción.
- **Ratón y táctil**: click en el rail para cambiar de página, click en una
  fila para seleccionar, doble-tap para abrir/ejecutar, click o arrastre en
  `[-]`/`[+]` de los steppers, rueda para moverse.
- **22 tests** (`cargo test`) cubren escritura atómica, merge sin perder
  claves desconocidas, catálogo completo, engine switch con seed de
  `classicbar`, zonas, keybinds, agrupado de ClassicBar y la búsqueda.
- Smoke tests reales por PTY: navegar, ajustar, elegir paleta, buscar y
  clickar en el rail; todo escribe/pinta correctamente.

### Cobertura por página

| Página | Qué incluye |
|---|---|
| Shell · General | uiScale, appScale (dispara `scale-menu.sh`), workspaceCount (+`queueReload`), idiomas xkb, atajo de layout, wallpaper dir, abrir selector de fondo |
| Shell · Timex | provider (open-meteo/wttr/openweather), unidad, ciudad (+refresh), API key (`timex keys set`), test de key, los ~21 knobs de layout de forecast/clock/calendar/day-panel |
| Shell · Keyboard | lista completa de keybinds (añadir/editar/borrar, validación de duplicados) y **regenera `config/user-keybinds.lua` + `hyprctl reload`** igual que el panel |
| Shell · Monitors | lectura `hyprctl -j monitors`, edición de resolución/refresh/rotación/VRR/bitdepth/cm/mirror/posición, **Apply** (settings.monitors + display-config + `hyprctl eval`) y Reset to auto |
| Shell · Startup | lista de comandos y **regenera `config/user-startup.lua` + reload** |
| Bar · Engine | selector bar/classic con seed de `classicbar` desde las zonas, mirror, classic defaults, matriz de posiciones de los 14 popups |
| Bar · Position | `bar.position` / `classicbar.position` según motor |
| Bar · Style | presets (aplican bundle de flags), roundness, thickness, edgeGap, opacity, fills, dragModules, font, formatos de hora/fecha, borders + **WindowBordersSection** completo (palette-follow, hex, gradientes, ángulos) |
| Bar · Zones | zonas (añadir/borrar), align, unify, container bg + color + solid, border width/color, chips de módulos con enable/disable y movimiento (Shift+←/→) dentro y entre zonas, Center all, Default |
| Bar · Classic Bar | style/timeFormat/distinctPills/autohide/roundness/thickness/opacity/width/hide-delay, secciones left/center/right/available con reordenar (`↑↓`), mover de sección (`m`), agrupar (`g`), desagrupar (`u`), mirror y defaults |
| Bar · Modules | iconColor global + los 18 módulos con icon/color/accent/fill y extras (formato y tamaño de reloj, efecto typewriter, cursor, formato de fecha, marker y 7 color-slots de workspaces) |
| Bar · Workspaces | marker (numbers/dots/letters/custom) y carácter custom |
| Theme · Palette | lista completa desde `index.json` (x/custom/user) con filtro, selección (`bar.palette`), **edición de los 18 slots** del archivo de paleta con backup y reset, crear (a partir de la activa) y borrar |
| Theme · Animations | enabled + presets/stepper de speed; escribe `user-animations.lua` con `luac -p` + reload vía `persist-hypr.sh` |
| Theme · Shadows/Glass/Mascots | todos los knobs (sombras 7, glass 2, mascots especie/posición/cantidad/tamaño) |
| Behavior · Launcher | posición, tamaños, márgenes, bordes, avoidBar, showIcons, alineación |
| Behavior · Notifications | width/maxHeight/shadowBlur/shadowOffset/shadow/posición 6 y DND |
| Widgets · Popups | las 16 secciones de `Personalization.js` con label `pretty()` y tipos respetados (bool/toggle, floats con step decimal, etc.) |
| System · Hyprland | los 12 sliders de `hypr-effects.sh` (preview en vivo con `hyprctl eval`, persist con `apply` al salir), Reset, Refresh, borders de ventana |
| System · Input | sensitivity, accel profile, tap-to-click, natural scroll, disable-while-typing; `persist-hypr.sh input` + reload |
| System · GPU | modo integrated/hybrid/nvidia vía `gpu-mode.sh`, modo actual vía `envycontrol --query` |
| System · Idle | modo Auto/Awake vía `idle-mode.sh`, estado, lock/suspend manuales |
| System · About | sysinfo, copiar debug, `dots doctor`, updater, docs, report issue |

## Uso

```sh
xturing              # instalado en ~/.local/bin
```

Para desarrollo: `cargo run` dentro de este repo.

### Probar sin tocar nada (recomendado antes de subir)

```sh
# 1) modo dry: ningún comando externo se ejecuta, se registran en
#    /tmp/xturing-actions.log
XTURING_DRY=1 xturing

# 2) settings sandbox: ni siquiera toca tu settings.json real
cp ~/.config/hypr/settings.json /tmp/settings-test.json
XTURING_DRY=1 XTURING_SETTINGS=/tmp/settings-test.json xturing

# 3) paletas sandbox (para la página Palette)
XTURING_DRY=1 \
XTURING_SETTINGS=/tmp/settings-test.json \
XTURING_PALETTES_DIR=/tmp/palettes-test \
xturing
```

Variables de entorno:

| Variable | Efecto |
|---|---|
| `XTURING_SETTINGS` | ruta alternativa de `settings.json` |
| `XTURING_PALETTES_DIR` | ruta alternativa del directorio de paletas (`index.json` + archivos) |
| `XTURING_DRY=1` | no ejecuta comandos; los escribe en `/tmp/xturing-actions.log` |

## Teclas

```
↑↓ / j k      mover selección          Tab / Shift+Tab  página siguiente/anterior
←→ / h l      ajustar / ciclar        1..6             ir a grupo (Shell, Bar, Theme, Behavior, Widgets, System)
Enter/Space   editar, togglear, elegir opción, ejecutar acción
/ o Ctrl+P    paleta de búsqueda       r                recargar settings.json
?             ayuda                    q / Ctrl+C       salir (persiste efectos pendientes)
a / d / w     añadir / borrar / guardar listas (keybinds, startup)
Shift+←/→     mover módulo entre zonas (Zones)
m / g / u     mover de sección / agrupar / desagrupar (Classic Bar)
```

Ratón / táctil (la terminal debe reportar eventos de ratón, p. ej. foot,
kitty o wezterm; en tablet el tap equivale al click):

```
click         seleccionar fila (en el rail: cambiar de página)
doble-tap     abrir / ejecutar la fila
[-] / [+]     pulsar o arrastrar para ajustar steppers
rueda         subir / bajar selección
```

## Contrato de escritura

- `settings.json` se escribe con `tmp + rename` atómico y `serde_json`
  preservando orden y **claves desconocidas**.
- Las escrituras son inmediatas (el panel QML usa un debounce de 220 ms;
  el resultado final es el mismo).
- Los caracteres de control en `bar.modules` (iconos glyph) se leen/escriben
  tal cual, sin sanear.
- El shell detecta cambios externos por sus watchers (`FileView`/`inotify`),
  así que los cambios se reflejan en vivo sin tocarlo.

## Estructura

```
src/
  settings.rs   lectura/escritura atómica + tests
  catalog.rs    definición data-driven de las 25 páginas y sus controles
  app.rs        estado, navegación, edición, páginas especiales, side effects
  ui.rs         render ratatui (rail, contenido, chooser, formularios, ayuda)
  palette.rs    index.json, edición de paletas con backup, crear/borrar
  classic.rs    defaults/mirrorBar/normalize de ClassicBar
  monitors.rs   hyprctl monitors, apply/reset con lua + display-config
  actions.rs    spawn/capture (con modo dry), notify-send
```

## Diferencias conocidas vs. el panel QML

- **Zones**: no hay drag & drop; el movimiento equivalente es
  `Shift+←/→` (dentro y entre zonas) y Enter para activar/desactivar.
- **ClassicBar**: agrupar/desagrupar con `g`/`u` en vez de arrastrar.
- **Monitors**: el canvas de arrastre se sustituye por campos Position X/Y.
- **Palette**: los colores se editan como hex (`#rrggbb`) en vez de swatch
  picker; "New palette" clona los 8 colores base de la paleta activa (el
  panel permite editar el draft antes de crear).
- **General**: la lista de idiomas xkb es campo de texto (se acepta cualquier
  código válido) en vez de sugerencias.
- El botón *Refresh* de Hyprland aquí funciona (`hypr-effects.sh read`); en el
  panel QML actual llama a un método inexistente.

Nada de esto cambia el archivo de settings ni el formato: son diferencias de
interacción, no de contrato.
