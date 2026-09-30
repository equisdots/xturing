//! Catalog: every page and control of the BarEditor (SUPER+SHIFT+D),
//! extracted 1:1 from the Quickshell sources. Data-driven: the UI renders
//! whatever lives here; app.rs knows how to read/write the paths.

use serde_json::Value;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Shell,
    Bar,
    Theme,
    Behavior,
    Widgets,
    System,
}

impl Group {
    pub const ALL: [Group; 6] = [
        Group::Shell,
        Group::Bar,
        Group::Theme,
        Group::Behavior,
        Group::Widgets,
        Group::System,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Group::Shell => "Shell",
            Group::Bar => "Bar",
            Group::Theme => "Theme",
            Group::Behavior => "Behavior",
            Group::Widgets => "Widgets",
            Group::System => "System",
        }
    }
}

pub struct Page {
    pub id: &'static str,
    pub group: Group,
    pub label: &'static str,
    /// Only listed in the rail when settings.barEngine matches.
    pub engine: Option<&'static str>,
    pub body: Body,
}

pub enum Body {
    Controls(Vec<Section>),
    Keybinds,
    Startup,
    Zones,
    ClassicBar(Vec<Section>),
    Palette,
    HyprEffects,
    Monitors,
    Guide,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BodyKind {
    Controls,
    Keybinds,
    Startup,
    Zones,
    ClassicBar,
    Palette,
    HyprEffects,
    Monitors,
    Guide,
}

impl Body {
    pub fn kind(&self) -> BodyKind {
        match self {
            Body::Controls(_) => BodyKind::Controls,
            Body::Keybinds => BodyKind::Keybinds,
            Body::Startup => BodyKind::Startup,
            Body::Zones => BodyKind::Zones,
            Body::ClassicBar(_) => BodyKind::ClassicBar,
            Body::Palette => BodyKind::Palette,
            Body::HyprEffects => BodyKind::HyprEffects,
            Body::Monitors => BodyKind::Monitors,
            Body::Guide => BodyKind::Guide,
        }
    }
}

#[derive(Clone)]
pub struct Section {
    pub title: String,
    pub controls: Vec<Control>,
}

#[derive(Clone)]
pub struct Control {
    pub label: String,
    pub kind: Kind,
    pub path: Vec<String>,
    pub help: String,
    pub visible: Option<Cond>,
}

#[derive(Clone)]
pub enum Cond {
    Eq(Vec<String>, Value),
    Engine(&'static str),
}

#[derive(Clone)]
pub enum Kind {
    Toggle,
    Stepper {
        step: f64,
        min: f64,
        max: f64,
        decimals: u32,
        unit: &'static str,
    },
    Options(Vec<Opt>),
    /// Option list whose value lives outside settings.json (GPU/Idle modes,
    /// engine switch...). app.rs applies `action` and refreshes `state_key`.
    StateOptions {
        state_key: String,
        options: Vec<Opt>,
        action: Action,
    },
    Text {
        placeholder: String,
        max: usize,
    },
    Action(Action),
    Info(String),
}

#[derive(Clone)]
pub struct Opt {
    pub label: String,
    pub value: String,
}

#[derive(Clone)]
pub enum Action {
    Shell(String),
    Prompt { prompt: String, prefix: String },
    SetEngine,
    MirrorBar,
    ClassicApplyDefaults,
    ZoneAdd,
    ZonesCenterAll,
    ZonesDefault,
    ClassicMirror,
    ClassicDefaults,
    PaletteReset,
    PaletteCreate,
    PaletteDelete,
    RotateForecastOrder,
    HyprReset,
    HyprRefresh,
    GpuMode,
    GpuRefresh,
    IdleMode,
    LockNow,
    Suspend,
    DndToggle,
    OpenWidget(String),
    CopyDebug,
    RunDoctor,
    OpenUrl(String),
    RefreshPage,
    MonitorsApply,
    MonitorsReset,
    PaletteFilterEdit,
}

// ───────────────────────────── helpers ─────────────────────────────

fn p(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| s.to_string()).collect()
}

fn control(label: &str, kind: Kind, path: Vec<String>, help: &str) -> Control {
    Control {
        label: label.to_string(),
        kind,
        path,
        help: help.to_string(),
        visible: None,
    }
}

fn toggle(label: &str, path: &[&str], help: &str) -> Control {
    control(label, Kind::Toggle, p(path), help)
}

#[allow(clippy::too_many_arguments)]
fn stepper(label: &str, path: &[&str], step: f64, min: f64, max: f64, decimals: u32, unit: &'static str, help: &str) -> Control {
    control(
        label,
        Kind::Stepper { step, min, max, decimals, unit },
        p(path),
        help,
    )
}

fn options(label: &str, path: &[&str], opts: &[(&str, &str)], help: &str) -> Control {
    control(
        label,
        Kind::Options(opts.iter().map(|(l, v)| Opt { label: l.to_string(), value: v.to_string() }).collect()),
        p(path),
        help,
    )
}

fn text(label: &str, path: &[&str], placeholder: &str, max: usize, help: &str) -> Control {
    control(
        label,
        Kind::Text { placeholder: placeholder.to_string(), max },
        p(path),
        help,
    )
}

fn action(label: &str, act: Action, help: &str) -> Control {
    control(label, Kind::Action(act), Vec::new(), help)
}

fn info(label: &str, key: &str) -> Control {
    control(label, Kind::Info(key.to_string()), Vec::new(), "")
}

fn visible(mut c: Control, cond: Cond) -> Control {
    c.visible = Some(cond);
    c
}

fn section(title: &str, controls: Vec<Control>) -> Section {
    Section { title: title.to_string(), controls }
}

// ───────────────────────────── catalog ─────────────────────────────

pub fn build() -> Vec<Page> {
    vec![
        // ═══════════════ SHELL ═══════════════
        Page {
            id: "s_general",
            group: Group::Shell,
            label: "General",
            engine: None,
            body: Body::Controls(vec![
                section("Display", vec![
                    stepper("UI Scale", &["uiScale"], 0.1, 0.5, 2.0, 1, "x", "Escala global del shell (0.5 – 2.0)"),
                    stepper("App Scale", &["appScale"], 0.25, 0.75, 2.0, 2, "x", "Escala de aplicaciones: dispara scale-menu.sh al cambiar"),
                    stepper("Workspaces", &["workspaceCount"], 1.0, 2.0, 10.0, 0, "", "2 – 10; recarga el topbar al cambiar"),
                ]),
                section("Keyboard", vec![
                    text("Keyboard layouts", &["language"], "us,es", 128, "Códigos xkb separados por coma (us, gb, es, latam, dvorak, us-intl...)"),
                    options("Layout shortcut", &["kbOptions"], &[
                        ("Alt + Shift", "grp:alt_shift_toggle"),
                        ("Win + Space", "grp:win_space_toggle"),
                        ("Caps Lock", "grp:caps_toggle"),
                        ("Ctrl + Shift", "grp:ctrl_shift_toggle"),
                        ("Ctrl + Alt", "grp:ctrl_alt_toggle"),
                        ("Right Alt", "grp:toggle"),
                        ("No Toggle", ""),
                    ], "Atajo para cambiar de layout"),
                ]),
                section("Wallpaper", vec![
                    text("Wallpaper directory", &["wallpaperDir"], "~/.config/hypr/wallpapers", 512, "Directorio de wallpapers"),
                    action("Change wallpaper", Action::OpenWidget("wallpaper".into()), "Abre el selector davincix (widget wallpaper)"),
                ]),
            ]),
        },
        Page {
            id: "s_timex",
            group: Group::Shell,
            label: "Timex",
            engine: None,
            body: Body::Controls(timex_controls()),
        },
        Page {
            id: "s_keyboard",
            group: Group::Shell,
            label: "Keyboard",
            engine: None,
            body: Body::Keybinds,
        },
        Page {
            id: "s_monitors",
            group: Group::Shell,
            label: "Monitors",
            engine: None,
            body: Body::Monitors,
        },
        Page {
            id: "s_startup",
            group: Group::Shell,
            label: "Startup",
            engine: None,
            body: Body::Startup,
        },
        // ═══════════════ BAR ═══════════════
        Page {
            id: "d_engine",
            group: Group::Bar,
            label: "Engine",
            engine: None,
            body: Body::Controls(vec![
                section("Engine", vec![
                    control(
                        "Engine",
                        Kind::StateOptions {
                            state_key: "engine".into(),
                            options: vec![
                                Opt { label: "Bar (zones · hot switch)".into(), value: "bar".into() },
                                Opt { label: "ClassicBar (autohide · pills)".into(), value: "classic".into() },
                            ],
                            action: Action::SetEngine,
                        },
                        Vec::new(),
                        "Cambia entre el motor de zonas y ClassicBar (hot switch)",
                    ),
                    visible(
                        action("Mirror bar layout", Action::MirrorBar, "Importa los módulos activos del bar a classicbar"),
                        Cond::Engine("classic"),
                    ),
                    visible(
                        action("Classic defaults", Action::ClassicApplyDefaults, "Resetea classicbar a sus valores por defecto"),
                        Cond::Engine("classic"),
                    ),
                ]),
                section("Popup positions", {
                    let mut out = Vec::new();
                    for (id, label) in POPUP_WIDGETS {
                        out.push(options(
                            label,
                            &["widgets", id, "position"],
                            &[
                                ("Auto", "default"),
                                ("Top Left", "top-left"),
                                ("Top Center", "top-center"),
                                ("Top Right", "top-right"),
                                ("Center Left", "center-left"),
                                ("Center", "center"),
                                ("Center Right", "center-right"),
                                ("Bottom Left", "bottom-left"),
                                ("Bottom Center", "bottom-center"),
                                ("Bottom Right", "bottom-right"),
                            ],
                            "auto = layout propio del widget; el resto ancla con margen de 20px escalado",
                        ));
                    }
                    out
                }),
            ]),
        },
        Page {
            id: "d_position",
            group: Group::Bar,
            label: "Position",
            engine: None,
            body: Body::Controls(vec![section("Bar position", vec![
                visible(
                    options("Position", &["bar", "position"], &[
                        ("Top", "top"), ("Bottom", "bottom"), ("Left", "left"), ("Right", "right"),
                    ], "Posición del bar de zonas"),
                    Cond::Engine("bar"),
                ),
                visible(
                    options("Position", &["classicbar", "position"], &[
                        ("Top", "top"), ("Bottom", "bottom"), ("Left", "left"), ("Right", "right"),
                    ], "Posición de ClassicBar"),
                    Cond::Engine("classic"),
                ),
            ])]),
        },
        Page {
            id: "d_style",
            group: Group::Bar,
            label: "Style",
            engine: Some("bar"),
            body: Body::Controls(bar_style_controls()),
        },
        Page {
            id: "d_zones",
            group: Group::Bar,
            label: "Zones",
            engine: Some("bar"),
            body: Body::Zones,
        },
        Page {
            id: "d_classic",
            group: Group::Bar,
            label: "Classic Bar",
            engine: Some("classic"),
            body: Body::ClassicBar(classic_controls()),
        },
        Page {
            id: "d_modules",
            group: Group::Bar,
            label: "Modules",
            engine: None,
            body: Body::Controls(modules_controls()),
        },
        Page {
            id: "d_workspaces",
            group: Group::Bar,
            label: "Workspaces",
            engine: None,
            body: Body::Controls(vec![section("Workspaces marker", vec![
                options("Marker", &["bar", "workspacesMarker"], &[
                    ("Numbers", "number"), ("Dots", "dot"), ("Letters", "letter"), ("Custom", "custom"),
                ], "Cómo se dibujan los workspaces vacíos"),
                visible(
                    text("Marker character", &["bar", "workspacesMarkerText"], "•", 4, "Hasta 4 caracteres; vacío usa •"),
                    Cond::Eq(p(&["bar", "workspacesMarker"]), Value::String("custom".into())),
                ),
                info("Workspace count", "workspaceCount"),
            ])]),
        },
        // ═══════════════ THEME ═══════════════
        Page {
            id: "d_palette",
            group: Group::Theme,
            label: "Palette",
            engine: None,
            body: Body::Palette,
        },
        Page {
            id: "d_animations",
            group: Group::Theme,
            label: "Animations",
            engine: None,
            body: Body::Controls(vec![
                section("General", vec![
                    toggle("Enabled", &["animations", "enabled"], "Animaciones de Hyprland (user-animations.lua)"),
                ]),
                section("Speed", vec![
                    options("Preset", &["animations", "speed"], &[
                        ("Snappy", "0.6"), ("Fast", "0.8"), ("Normal", "1.0"), ("Slow", "1.5"),
                    ], "Presets de velocidad; mayor valor = más lento"),
                    stepper("Speed", &["animations", "speed"], 0.1, 0.5, 2.0, 1, "x", "0.5 – 2.0"),
                ]),
            ]),
        },
        Page {
            id: "d_shadows",
            group: Group::Theme,
            label: "Shadows",
            engine: None,
            body: Body::Controls(vec![
                section("Shape", vec![
                    toggle("Enabled", &["shadows", "enabled"], "Sombras del shell (paneles, barra)"),
                    stepper("Blur", &["shadows", "blur"], 2.0, 0.0, 60.0, 0, "px", "0 – 60"),
                    stepper("Spread", &["shadows", "spread"], 1.0, -10.0, 20.0, 0, "px", "-10 – 20"),
                    stepper("Corner radius", &["shadows", "radius"], 1.0, 0.0, 30.0, 0, "px", "0 – 30"),
                ]),
                section("Placement", vec![
                    stepper("Offset X", &["shadows", "offsetX"], 2.0, -40.0, 40.0, 0, "px", "-40 – 40"),
                    stepper("Offset Y", &["shadows", "offsetY"], 2.0, -40.0, 40.0, 0, "px", "-40 – 40"),
                    stepper("Opacity", &["shadows", "opacity"], 0.05, 0.0, 1.0, 2, "", "0 – 1"),
                ]),
            ]),
        },
        Page {
            id: "d_glass",
            group: Group::Theme,
            label: "Glass",
            engine: None,
            body: Body::Controls(vec![section("Glassmorphism", vec![
                toggle("Enabled", &["glass", "enabled"], "Fondo translúcido en las superficies del shell"),
                stepper("Background opacity", &["glass", "opacity"], 0.05, 0.4, 1.0, 2, "", "0.4 – 1.0"),
            ])]),
        },
        Page {
            id: "d_mascots",
            group: Group::Theme,
            label: "Mascots",
            engine: None,
            body: Body::Controls(vec![
                section("General", vec![
                    toggle("Enabled", &["mascots", "enabled"], "Mascotas del escritorio"),
                ]),
                section("Species", vec![
                    options("Species", &["mascots", "species"], &[
                        ("Flame", "flame"), ("Cats", "cat"), ("Dogs", "dog"), ("Eyes", "eyes"), ("Mixed", "mixed"), ("Dots", "dots"),
                    ], "Especie de la mascota"),
                ]),
                section("Position", vec![
                    options("Position", &["mascots", "position"], &[
                        ("Top Left", "top-left"), ("Top Center", "top-center"), ("Top Right", "top-right"),
                        ("Center Left", "center-left"), ("Center", "center"), ("Center Right", "center-right"),
                        ("Bottom Left", "bottom-left"), ("Bottom Center", "bottom-center"), ("Bottom Right", "bottom-right"),
                    ], "Ancla de la mascota"),
                ]),
                section("How many", vec![
                    options("Count", &["mascots", "count"], &[
                        ("One", "1"), ("Two", "2"), ("Three", "3"),
                    ], "1 – 3 mascotas"),
                    stepper("Size", &["mascots", "size"], 0.1, 0.6, 1.6, 1, "x", "0.6 – 1.6"),
                ]),
            ]),
        },
        // ═══════════════ BEHAVIOR ═══════════════
        Page {
            id: "d_launcher",
            group: Group::Behavior,
            label: "Launcher",
            engine: None,
            body: Body::Controls(launcher_controls()),
        },
        Page {
            id: "d_notifications",
            group: Group::Behavior,
            label: "Notifications",
            engine: None,
            body: Body::Controls(notification_controls()),
        },
        // ═══════════════ WIDGETS ═══════════════
        Page {
            id: "d_widgets",
            group: Group::Widgets,
            label: "Popups",
            engine: None,
            body: Body::Controls(popup_controls()),
        },
        // ═══════════════ SYSTEM ═══════════════
        Page {
            id: "d_hyprland",
            group: Group::System,
            label: "Hyprland",
            engine: None,
            body: Body::HyprEffects,
        },
        Page {
            id: "d_input",
            group: Group::System,
            label: "Input",
            engine: None,
            body: Body::Controls(vec![
                section("Mouse", vec![
                    stepper("Sensitivity", &["input", "sensitivity"], 0.1, -1.0, 1.0, 1, "", "-1.0 – 1.0"),
                    options("Accel profile", &["input", "accelProfile"], &[
                        ("Adaptive", "adaptive"), ("Flat", "flat"),
                    ], "Perfil de aceleración del puntero"),
                ]),
                section("Touchpad", vec![
                    toggle("Tap to click", &["input", "tapToClick"], "Tap para hacer clic"),
                    toggle("Natural scroll", &["input", "naturalScroll"], "Scroll natural"),
                    toggle("Disable while typing", &["input", "disableWhileTyping"], "Desactiva el touchpad al escribir"),
                ]),
            ]),
        },
        Page {
            id: "d_gpu",
            group: Group::System,
            label: "GPU",
            engine: None,
            body: Body::Controls(vec![
                section("Optimus mode", vec![
                    control(
                        "Mode",
                        Kind::StateOptions {
                            state_key: "gpu".into(),
                            options: vec![
                                Opt { label: "Integrated".into(), value: "integrated".into() },
                                Opt { label: "Hybrid".into(), value: "hybrid".into() },
                                Opt { label: "NVIDIA".into(), value: "nvidia".into() },
                            ],
                            action: Action::GpuMode,
                        },
                        Vec::new(),
                        "Cambia con envycontrol; puede requerir logout/reboot",
                    ),
                    info("Current mode", "gpu.mode"),
                    action("Refresh", Action::GpuRefresh, "Relee envycontrol --query"),
                ]),
                section("Environment (static)", vec![
                    info("GBM_BACKEND", "gpu.env.gbm"),
                    info("LIBVA_DRIVER_NAME", "gpu.env.libva"),
                ]),
            ]),
        },
        Page {
            id: "d_idle",
            group: Group::System,
            label: "Idle",
            engine: None,
            body: Body::Controls(vec![
                section("Idle mode", vec![
                    control(
                        "Mode",
                        Kind::StateOptions {
                            state_key: "idle".into(),
                            options: vec![
                                Opt { label: "Auto".into(), value: "normal".into() },
                                Opt { label: "Awake".into(), value: "awake".into() },
                            ],
                            action: Action::IdleMode,
                        },
                        Vec::new(),
                        "Awake detiene hypridle: nada auto-dim, lock ni suspend",
                    ),
                    info("Current mode", "idle.mode"),
                ]),
                section("Manual actions", vec![
                    action("Lock now", Action::LockNow, "Bloquea la sesión (SUPER+L)"),
                    action("Suspend", Action::Suspend, "systemctl suspend"),
                ]),
            ]),
        },
        Page {
            id: "d_guide",
            group: Group::System,
            label: "About",
            engine: None,
            body: Body::Guide,
        },
    ]
}

// ───────────────────────────── popups / personalization ─────────────────────────────

const POPUP_WIDGETS: &[(&str, &str)] = &[
    ("network", "Wifi / Bluetooth"),
    ("volume", "Sound"),
    ("battery", "Battery"),
    ("system-monitor", "System"),
    ("applauncher", "Launcher"),
    ("clipboard", "Clipboard"),
    ("calendar", "Timex"),
    ("music", "Music"),
    ("updater", "Updater"),
    ("guide", "About (guide)"),
    ("quicknotes", "Notepad"),
    ("rss-reader", "RSS"),
    ("scale", "Scale"),
    ("window-controls", "Window controls"),
];

enum PKind {
    Bool,
    Int { min: f64, max: f64, step: f64 },
    Float { min: f64, max: f64, step: f64, decimals: u32 },
}

struct PKey {
    key: &'static str,
    kind: PKind,
}

struct PSec {
    id: &'static str,
    label: &'static str,
    keys: &'static [PKey],
}

macro_rules! pk {
    ($key:literal, bool) => {
        PKey { key: $key, kind: PKind::Bool }
    };
    ($key:literal, int, $min:expr, $max:expr, $step:expr) => {
        PKey { key: $key, kind: PKind::Int { min: $min, max: $max, step: $step } }
    };
    ($key:literal, float, $min:expr, $max:expr, $step:expr, $dec:expr) => {
        PKey { key: $key, kind: PKind::Float { min: $min, max: $max, step: $step, decimals: $dec } }
    };
}

static PERSONALIZATION: &[PSec] = &[
    PSec { id: "network", label: "Wifi / Bluetooth", keys: &[
        pk!("popupWidth", int, 200.0, 4000.0, 10.0),
        pk!("popupHeight", int, 200.0, 4000.0, 10.0),
        pk!("powerAnimMs", int, 0.0, 5000.0, 50.0),
        pk!("busyTimeoutMs", int, 0.0, 60000.0, 500.0),
        pk!("failClearMs", int, 0.0, 30000.0, 500.0),
    ]},
    PSec { id: "volume", label: "Sound", keys: &[
        pk!("popupWidth", int, 200.0, 2000.0, 10.0),
        pk!("step", int, 1.0, 50.0, 1.0),
        pk!("syncDelayMs", int, 0.0, 5000.0, 50.0),
        pk!("pollMs", int, 100.0, 10000.0, 100.0),
    ]},
    PSec { id: "battery", label: "Battery", keys: &[
        pk!("popupWidth", int, 200.0, 2000.0, 10.0),
        pk!("lowThreshold", int, 0.0, 100.0, 5.0),
        pk!("pollMs", int, 100.0, 60000.0, 500.0),
    ]},
    PSec { id: "system-monitor", label: "System monitor", keys: &[
        pk!("popupWidth", int, 200.0, 2000.0, 10.0),
        pk!("popupHeight", int, 200.0, 2000.0, 10.0),
        pk!("pollMs", int, 100.0, 60000.0, 500.0),
        pk!("historyLength", int, 1.0, 600.0, 5.0),
    ]},
    PSec { id: "calendar", label: "Calendar", keys: &[
        pk!("popupWidth", int, 200.0, 4000.0, 10.0),
        pk!("popupHeight", int, 200.0, 4000.0, 10.0),
    ]},
    PSec { id: "clipboard", label: "Clipboard", keys: &[
        pk!("popupWidth", int, 200.0, 2000.0, 10.0),
        pk!("popupHeight", int, 200.0, 2000.0, 10.0),
        pk!("fetchLimit", int, 1.0, 500.0, 1.0),
    ]},
    PSec { id: "quicknotes", label: "Notepad", keys: &[
        pk!("popupWidth", int, 200.0, 2000.0, 10.0),
        pk!("popupHeight", int, 200.0, 2000.0, 10.0),
        pk!("saveDebounceMs", int, 0.0, 10000.0, 100.0),
    ]},
    PSec { id: "rss-reader", label: "RSS reader", keys: &[
        pk!("popupWidth", int, 200.0, 2000.0, 10.0),
        pk!("popupHeight", int, 200.0, 2000.0, 10.0),
        pk!("refreshMs", int, 10000.0, 3600000.0, 30000.0),
    ]},
    PSec { id: "file-search", label: "File search", keys: &[
        pk!("popupWidth", int, 200.0, 2000.0, 10.0),
        pk!("popupHeight", int, 200.0, 2000.0, 10.0),
        pk!("debounceMs", int, 0.0, 5000.0, 50.0),
    ]},
    PSec { id: "scale", label: "Scale picker", keys: &[
        pk!("popupWidth", int, 200.0, 2000.0, 10.0),
        pk!("popupHeight", int, 200.0, 2000.0, 10.0),
        pk!("pollMs", int, 50.0, 5000.0, 50.0),
    ]},
    PSec { id: "window-controls", label: "Window controls", keys: &[
        pk!("popupWidth", int, 200.0, 2000.0, 10.0),
        pk!("popupHeight", int, 200.0, 2000.0, 10.0),
        pk!("activeOpacity", float, 0.0, 1.0, 0.05, 2),
        pk!("inactiveOpacity", float, 0.0, 1.0, 0.05, 2),
        pk!("blurSize", int, 0.0, 24.0, 1.0),
        pk!("blurPasses", int, 0.0, 10.0, 1.0),
        pk!("roundness", int, 0.0, 40.0, 1.0),
    ]},
    PSec { id: "lock", label: "Lock screen", keys: &[
        pk!("revealDurationMs", int, 0.0, 5000.0, 50.0),
        pk!("clockPollMs", int, 100.0, 10000.0, 100.0),
        pk!("batteryPollMs", int, 100.0, 60000.0, 500.0),
        pk!("wallpaperBlur", float, 0.0, 1.0, 0.05, 2),
        pk!("wallpaperDim", float, 0.0, 0.85, 0.05, 2),
        pk!("showClock", bool),
        pk!("showBattery", bool),
        pk!("showPower", bool),
        pk!("clockScale", float, 0.6, 1.6, 0.05, 2),
    ]},
    PSec { id: "updater", label: "Updater", keys: &[
        pk!("popupWidth", int, 200.0, 4000.0, 10.0),
        pk!("popupHeight", int, 200.0, 4000.0, 10.0),
    ]},
    PSec { id: "idle", label: "Idle", keys: &[
        pk!("popupWidth", int, 200.0, 2000.0, 10.0),
        pk!("popupHeight", int, 200.0, 2000.0, 10.0),
    ]},
    PSec { id: "tray", label: "System tray", keys: &[
        pk!("tint", bool),
        pk!("useAccent", bool),
        pk!("size", int, 8.0, 48.0, 2.0),
    ]},
    PSec { id: "widgets", label: "Desktop widgets", keys: &[
        pk!("redactorWidth", int, 200.0, 8000.0, 10.0),
        pk!("redactorHeight", int, 200.0, 8000.0, 10.0),
    ]},
];

fn pretty(key: &str) -> String {
    let mut out = String::new();
    for (i, ch) in key.chars().enumerate() {
        if i > 0 && ch.is_uppercase() {
            out.push(' ');
        }
        if i == 0 {
            out.extend(ch.to_uppercase());
        } else {
            out.push(ch);
        }
    }
    out
}

fn popup_controls() -> Vec<Section> {
    let mut sections = Vec::new();
    for sec in PERSONALIZATION {
        let mut controls = Vec::new();
        for key in sec.keys {
            let label = pretty(key.key);
            let path = ["", "", ""];
            let _ = path;
            let full = p(&[sec.id, key.key]);
            match &key.kind {
                PKind::Bool => controls.push(control(&label, Kind::Toggle, full, "")),
                PKind::Int { min, max, step } => controls.push(control(
                    &label,
                    Kind::Stepper { step: *step, min: *min, max: *max, decimals: 0, unit: "" },
                    full,
                    "",
                )),
                PKind::Float { min, max, step, decimals } => controls.push(control(
                    &label,
                    Kind::Stepper { step: *step, min: *min, max: *max, decimals: *decimals, unit: "" },
                    full,
                    "",
                )),
            }
        }
        sections.push(section(sec.label, controls));
    }
    sections
}

// ───────────────────────────── timex ─────────────────────────────

fn timex_controls() -> Vec<Section> {
    let timex = |key: &str| p(&["timex", key]);
    let t = |label: &str, key: &str| control(label, Kind::Toggle, timex(key), "");
    vec![
        section("Provider", vec![
            control("Provider", Kind::Options(vec![
                Opt { label: "Open-Meteo (free, city or lat,lon)".into(), value: "open-meteo".into() },
                Opt { label: "wttr.in (free, city)".into(), value: "wttr".into() },
                Opt { label: "OpenWeatherMap (needs key)".into(), value: "openweather".into() },
            ]), timex("provider"), "Fuente de datos meteorológicos"),
            control("Temperature unit", Kind::Options(vec![
                Opt { label: "Celsius".into(), value: "metric".into() },
                Opt { label: "Fahrenheit".into(), value: "imperial".into() },
            ]), timex("unit"), ""),
        ]),
        section("City", vec![
            control("City", Kind::Text { placeholder: "Madrid o 40.4,-3.7".into(), max: 128 }, timex("city"), "Enter guarda y refresca; Open-Meteo acepta lat,lon"),
            action("OpenWeather API key", Action::Prompt {
                prompt: "API key".into(),
                prefix: "TIMEX=\"${TIMEX_CLI:-$HOME/.local/bin/timex}\"; [ -x \"$TIMEX\" ] || TIMEX=timex; \"$TIMEX\" keys set OPENWEATHER_KEY".into(),
            }, "Guarda la key en el state de timex (chmod 600)"),
            action("Test key", Action::Shell("TIMEX=\"${TIMEX_CLI:-$HOME/.local/bin/timex}\"; [ -x \"$TIMEX\" ] || TIMEX=timex; \"$TIMEX\" test".into()), "Prueba la API key"),
            action("Refresh data", Action::Shell("TIMEX=\"${TIMEX_CLI:-$HOME/.local/bin/timex}\"; [ -x \"$TIMEX\" ] || TIMEX=timex; \"$TIMEX\" --invalidate >/dev/null 2>&1; \"$TIMEX\" --getdata >/dev/null 2>&1".into()), "Invalida y vuelve a pedir datos"),
            info("Engine status", "timex.status"),
        ]),
        section("Timex layout", vec![
            t("Hourly row", "forecastEnabled"),
            control("Forecast position", Kind::Options(vec![
                Opt { label: "Below".into(), value: "below".into() },
                Opt { label: "Above".into(), value: "above".into() },
            ]), timex("forecastPosition"), ""),
            t("Show time", "forecastShowTime"),
            t("Show icon", "forecastShowIcon"),
            t("Show temp", "forecastShowTemp"),
            action("Rotate order", Action::RotateForecastOrder, "Rota el orden time · icon · temp"),
            t("Clock seconds", "clockShowSeconds"),
            t("Clock date", "clockShowDate"),
            stepper("Forecast size", &["timex", "forecastSize"], 0.1, 0.8, 1.4, 1, "x", "0.8 – 1.4"),
            stepper("Gap", &["timex", "forecastGap"], 2.0, 2.0, 40.0, 0, "px", "2 – 40"),
            stepper("Hours shown", &["timex", "forecastHours"], 1.0, 3.0, 8.0, 0, "", "3 – 8"),
            stepper("Clock scale", &["timex", "clockScale"], 0.05, 0.85, 1.25, 2, "x", "0.85 – 1.25"),
        ]),
        section("Calendar & day panel", vec![
            t("Calendar", "calendarEnabled"),
            control("Week starts", Kind::Options(vec![
                Opt { label: "Monday".into(), value: "monday".into() },
                Opt { label: "Sunday".into(), value: "sunday".into() },
            ]), timex("calendarWeekStart"), ""),
            stepper("Calendar size", &["timex", "calendarSize"], 0.05, 0.8, 1.2, 2, "x", "0.8 – 1.2"),
            t("Day panel", "panelEnabled"),
            t("Wind", "panelShowWind"),
            t("Humidity", "panelShowHumidity"),
            t("Rain", "panelShowPop"),
            t("Feels like", "panelShowFeels"),
            stepper("Day panel size", &["timex", "panelSize"], 0.05, 0.8, 1.2, 2, "x", "0.8 – 1.2"),
        ]),
        section("Order", vec![
            info("Order value", "timex.order"),
        ]),
    ]
}

// ───────────────────────────── bar style / borders ─────────────────────────────

pub fn border_sections(prefix_help: &str) -> Vec<Section> {
    vec![border_controls(prefix_help)]
}

fn border_controls(prefix_help: &str) -> Section {
    section(&format!("Window borders — {}", prefix_help), vec![
        toggle("Follow palette", &["bar", "borderFollowPalette"], "Bordes de ventana siguen los acentos de la paleta"),
        options("Active color", &["bar", "borderActive"], &[("Empty (palette)", "")], "Hex #rrggbb; vacío = derivado de la paleta"),
        options("Active gradient", &["bar", "borderGradientActive"], &[("Off", "false"), ("On", "true")], "Gradiente en el borde activo"),
        options("Active 2nd color", &["bar", "borderActive2"], &[("Empty", "")], "Segundo color del gradiente activo"),
        stepper("Active angle", &["bar", "borderAngleActive"], 15.0, 0.0, 360.0, 0, "°", "0 – 360"),
        options("Inactive color", &["bar", "borderInactive"], &[("Empty (palette)", "")], "Hex #rrggbb; vacío = derivado de la paleta"),
        options("Inactive gradient", &["bar", "borderGradientInactive"], &[("Off", "false"), ("On", "true")], "Gradiente en el borde inactivo"),
        options("Inactive 2nd color", &["bar", "borderInactive2"], &[("Empty", "")], "Segundo color del gradiente inactivo"),
        stepper("Inactive angle", &["bar", "borderAngleInactive"], 15.0, 0.0, 360.0, 0, "°", "0 – 360"),
    ])
}

fn bar_style_controls() -> Vec<Section> {
    vec![
        section("Presets", vec![
            options("Style preset", &["bar", "stylePreset"], &[
                ("Modular", "modular"), ("Solid", "solid"), ("Fill", "fill"),
            ], "Aplica un bundle de flags (pillBg/pillSolid/barBg/edgeGap)"),
        ]),
        section("Shape", vec![
            stepper("Roundness", &["bar", "roundness"], 0.1, 0.0, 1.0, 1, "", "0 – 1"),
            stepper("Thickness", &["bar", "thickness"], 4.0, 24.0, 96.0, 0, "px", "24 – 96"),
            stepper("Edge margin", &["bar", "edgeGap"], 2.0, 0.0, 24.0, 0, "px", "0 – 24"),
            stepper("Bar opacity", &["bar", "barOpacity"], 0.05, 0.2, 1.0, 2, "", "0.2 – 1.0"),
        ]),
        section("Fill", vec![
            toggle("Island fill", &["bar", "pillBg"], "Fondo de las islas/pills"),
            toggle("Solid fill", &["bar", "pillSolid"], "Relleno sólido de las islas"),
            toggle("Unified bar", &["bar", "barBg"], "Fondo unificado de barra completa"),
            toggle("Drag modules", &["bar", "dragModules"], "Permite arrastrar módulos en la barra"),
        ]),
        section("Font", vec![
            text("Font", &["bar", "font"], "Hack Nerd Font", 128, "Fuente del shell; vacío resetea a Hack Nerd Font"),
            text("Time format", &["bar", "timeFormat"], "HH:mm:ss", 64, "Formato Qt (QML) del reloj"),
            text("Date format", &["bar", "dateFormat"], "dddd, MMMM dd", 64, "Formato Qt (QML) de la fecha"),
        ]),
        section("Borders", vec![
            stepper("Border width", &["bar", "borderWidth"], 1.0, 0.0, 8.0, 0, "px", "0 – 8"),
            options("Border color", &["bar", "borderColor"], &[
                ("surface1", "surface1"), ("surface0", "surface0"), ("text", "text"), ("red", "red"),
                ("blue", "blue"), ("green", "green"), ("yellow", "yellow"), ("mauve", "mauve"), ("teal", "teal"),
            ], "Rol de color del borde del bar"),
        ]),
        border_controls("bordes de ventana"),
    ]
}

fn classic_controls() -> Vec<Section> {
    vec![
        section("Style & size", vec![
            options("Style", &["classicbar", "style"], &[
                ("Modular", "modular"), ("Solid", "solid"), ("Fill", "fill"),
            ], ""),
            options("Time format", &["classicbar", "timeFormat"], &[
                ("24h :ss", "HH:mm:ss"), ("24h :mm", "HH:mm"), ("12h", "h:mm a"),
            ], ""),
            toggle("Distinct pills", &["classicbar", "distinctPills"], "Pills separadas por módulo"),
            toggle("Autohide", &["classicbar", "autohide"], "Oculta la barra hasta acercar el cursor"),
            stepper("Roundness", &["classicbar", "roundness"], 0.1, 0.0, 1.0, 1, "", "0 – 1"),
            stepper("Thickness", &["classicbar", "thickness"], 4.0, 24.0, 120.0, 0, "px", "24 – 120"),
            stepper("Bar opacity", &["classicbar", "opacity"], 5.0, 20.0, 100.0, 0, "%", "20 – 100"),
            stepper("Width", &["classicbar", "widthPercent"], 5.0, 40.0, 100.0, 0, "%", "40 – 100"),
            stepper("Hide delay", &["classicbar", "autohideTimeout"], 100.0, 200.0, 5000.0, 0, "ms", "200 – 5000"),
        ]),
        section("Actions", vec![
            action("Mirror bar layout", Action::ClassicMirror, "Importa módulos activos del bar de zonas"),
            action("Classic defaults", Action::ClassicDefaults, "Resetea classicbar (mantiene posición)"),
        ]),
    ]
}

// ───────────────────────────── modules ─────────────────────────────

const MODULES: &[(&str, &str, bool)] = &[
    ("help", "Help", true),
    ("search", "Search", true),
    ("settings", "Settings", true),
    ("update", "Updates", true),
    ("time", "Clock", false),
    ("date", "Date", false),
    ("media", "Media", false),
    ("workspaces", "Workspaces", false),
    ("tray", "System tray", false),
    ("keyboard", "Keyboard", true),
    ("wifi", "Network", true),
    ("bluetooth", "Bluetooth", true),
    ("sysmon", "Resources", false),
    ("volume", "Volume", true),
    ("battery", "Battery", true),
    ("recording", "Recording", true),
    ("weather", "Timex", true),
    ("focus", "Focus", true),
];

fn fill_control(label: &str, path: Vec<String>) -> Control {
    Control {
        label: label.to_string(),
        kind: Kind::Options(vec![
            Opt { label: "Default".into(), value: "default".into() },
            Opt { label: "Filled".into(), value: "on".into() },
            Opt { label: "None".into(), value: "off".into() },
        ]),
        path,
        help: String::new(),
        visible: None,
    }
}

fn modules_controls() -> Vec<Section> {
    let mut sections = vec![section("Global", vec![
        text("Icon color", &["bar", "iconColor"], "default (module roles)", 64, "Rol colors.* o #hex; aplica a módulos sin color propio"),
    ])];
    for (id, label, icon_ok) in MODULES {
        let m = |key: &str| p(&["bar", "modules", id, key]);
        let mut controls = vec![];
        if *icon_ok {
            controls.push(control("Icon", Kind::Text { placeholder: "default".into(), max: 4 }, m("icon"), "Glyph de Nerd Font (máx 4)"))
        }
        controls.push(control("Color", Kind::Text { placeholder: "default".into(), max: 64 }, m("color"), "Rol colors.* o #hex"));
        controls.push(control("Accent", Kind::Text { placeholder: "default".into(), max: 64 }, m("accent"), "Rol de acento"));
        controls.push(fill_control("Fill", m("fill")));
        match *id {
            "time" => {
                controls.push(text("Clock format", &["bar", "timeFormat"], "HH:mm:ss", 64, "Formato Qt"));
                controls.push(control("Size", Kind::Stepper { step: 1.0, min: 0.0, max: 48.0, decimals: 0, unit: "px" }, m("size"), "0 = default del módulo"));
                controls.push(control("Effect", Kind::Options(vec![
                    Opt { label: "No effect".into(), value: String::new() },
                    Opt { label: "Typewriter".into(), value: "typewriter".into() },
                ]), m("effect"), ""));
                controls.push(control("Cursor", Kind::Toggle, m("cursor"), "Cursor parpadeante"));
            }
            "date" => {
                controls.push(text("Date format", &["bar", "dateFormat"], "dddd, MMMM dd", 64, "Formato Qt"));
                controls.push(control("Size", Kind::Stepper { step: 1.0, min: 0.0, max: 48.0, decimals: 0, unit: "px" }, m("size"), "0 = default del módulo"));
            }
            "workspaces" => {
                controls.push(control("Marker", Kind::Options(vec![
                    Opt { label: "Numbers".into(), value: "number".into() },
                    Opt { label: "Dots".into(), value: "dot".into() },
                    Opt { label: "Letters".into(), value: "letter".into() },
                    Opt { label: "Custom".into(), value: "custom".into() },
                ]), m("marker"), ""));
                let slots = ["active", "activeText", "occupied", "empty", "hover", "marker", "markerEmpty"];
                for slot in slots {
                    controls.push(control(
                        &format!("Color: {}", pretty(slot)),
                        Kind::Text { placeholder: "default".into(), max: 64 },
                        m(&format!("colors.{}", slot)),
                        "Rol colors.* o #hex",
                    ));
                }
            }
            _ => {}
        }
        sections.push(section(label, controls));
    }
    sections
}

// ───────────────────────────── launcher ─────────────────────────────

fn launcher_controls() -> Vec<Section> {
    vec![
        section("Position", vec![
            options("Position", &["launcher", "position"], &[
                ("Center", "center"), ("Top", "top"), ("Bottom", "bottom"), ("Left", "left"), ("Right", "right"),
            ], ""),
        ]),
        section("Size", vec![
            stepper("Width", &["launcher", "width"], 40.0, 320.0, 1280.0, 0, "px", "320 – 1280"),
            stepper("Visible apps", &["launcher", "maxApps"], 1.0, 4.0, 20.0, 0, "", "4 – 20"),
            stepper("Margin", &["launcher", "margin"], 8.0, -200.0, 200.0, 0, "px", "-200 – 200"),
            stepper("Row height", &["launcher", "rowHeight"], 4.0, 28.0, 80.0, 0, "px", "28 – 80"),
        ]),
        section("Borders", vec![
            stepper("Border width", &["launcher", "borderWidth"], 1.0, 0.0, 4.0, 0, "px", "0 oculta el borde"),
            stepper("Radius", &["launcher", "radius"], 2.0, 0.0, 28.0, 0, "px", "0 – 28"),
            options("Border color", &["launcher", "borderColor"], &[
                ("surface1", "surface1"), ("surface0", "surface0"), ("text", "text"), ("red", "red"),
                ("blue", "blue"), ("green", "green"), ("yellow", "yellow"), ("mauve", "mauve"), ("teal", "teal"),
            ], "Rol de color"),
        ]),
        section("Behavior", vec![
            toggle("Avoid bar", &["launcher", "avoidBar"], "Evita solaparse con el bar"),
            toggle("Show icons", &["launcher", "showIcons"], "Muestra iconos de aplicaciones"),
        ]),
        section("Content alignment", vec![
            options("Align", &["launcher", "align"], &[
                ("Left", "left"), ("Center", "center"), ("Right", "right"),
            ], ""),
        ]),
    ]
}

// ───────────────────────────── notifications ─────────────────────────────

fn notification_controls() -> Vec<Section> {
    vec![
        section("Notifications layout", vec![
            stepper("Width", &["notifications", "width"], 5.0, 0.0, 1200.0, 0, "px", ""),
            stepper("Max height (0 = auto)", &["notifications", "maxHeight"], 5.0, 0.0, 2000.0, 0, "px", ""),
            stepper("Shadow blur", &["notifications", "shadowBlur"], 1.0, 0.0, 80.0, 0, "px", ""),
            stepper("Shadow offset", &["notifications", "shadowOffset"], 1.0, 0.0, 40.0, 0, "px", ""),
            options("Window shadow", &["notifications", "shadow"], &[
                ("Off", "0"), ("On", "1"),
            ], ""),
            options("Position", &["notifications", "position"], &[
                ("Top Left", "0"), ("Top Center", "1"), ("Top Right", "2"),
                ("Bottom Left", "3"), ("Bottom Center", "4"), ("Bottom Right", "5"),
            ], ""),
            control("Do Not Disturb", Kind::Action(Action::DndToggle), Vec::new(), "Silencia los popups (archivo de estado)"),
        ]),
    ]
}


pub const HYPR_SLIDERS: &[(&str, &str, f64, f64, f64, f64, &str)] = &[
    ("active_opacity", "Active Opacity", 0.85, 0.30, 1.0, 0.05, "2"),
    ("inactive_opacity", "Inactive Opacity", 0.80, 0.30, 1.0, 0.05, "2"),
    ("rounding", "Rounding", 20.0, 0.0, 35.0, 1.0, "0"),
    ("blur_size", "Blur Size", 8.0, 0.0, 24.0, 1.0, "0"),
    ("blur_passes", "Blur Passes", 3.0, 0.0, 10.0, 1.0, "0"),
    ("gaps_in", "Gaps In", 16.0, 0.0, 50.0, 2.0, "0"),
    ("gaps_out", "Gaps Out", 25.0, 0.0, 50.0, 2.0, "0"),
    ("border_size", "Border Width", 2.0, 0.0, 20.0, 1.0, "0"),
    ("shadow_range", "Shadow Range", 35.0, 0.0, 50.0, 1.0, "0"),
    ("shadow_render_power", "Shadow Power", 5.0, 0.0, 10.0, 1.0, "0"),
    ("shadow_offset_x", "Shadow Offset X", 0.0, -30.0, 30.0, 1.0, "0"),
    ("shadow_offset_y", "Shadow Offset Y", 10.0, -30.0, 30.0, 1.0, "0"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_all_editor_pages_and_unique_ids() {
        let pages = build();
        let mut ids: Vec<&str> = pages.iter().map(|p| p.id).collect();
        let count = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), count, "duplicate page ids");
        let expect = [
            "s_general", "s_timex", "s_keyboard", "s_monitors", "s_startup",
            "d_engine", "d_position", "d_style", "d_zones", "d_classic",
            "d_modules", "d_workspaces",
            "d_palette", "d_animations", "d_shadows", "d_glass", "d_mascots",
            "d_launcher", "d_notifications",
            "d_widgets",
            "d_hyprland", "d_input", "d_gpu", "d_idle", "d_guide",
        ];
        for id in expect {
            assert!(pages.iter().any(|p| p.id == id), "missing page {id}");
        }
        assert_eq!(pages.len(), expect.len());
    }

    #[test]
    fn engine_gated_pages_match_editor_nav() {
        let pages = build();
        let style = pages.iter().find(|p| p.id == "d_style").unwrap();
        assert_eq!(style.engine, Some("bar"));
        let zones = pages.iter().find(|p| p.id == "d_zones").unwrap();
        assert_eq!(zones.engine, Some("bar"));
        let classic = pages.iter().find(|p| p.id == "d_classic").unwrap();
        assert_eq!(classic.engine, Some("classic"));
        for id in ["d_engine", "d_position", "d_modules", "d_workspaces"] {
            assert_eq!(pages.iter().find(|p| p.id == id).unwrap().engine, None);
        }
    }

    #[test]
    fn popup_positions_cover_all_registry_widgets() {
        let sections = match &build().into_iter().find(|p| p.id == "d_engine").unwrap().body {
            Body::Controls(s) => s.clone(),
            _ => unreachable!(),
        };
        let positions = sections.iter().find(|s| s.title == "Popup positions").unwrap();
        assert_eq!(positions.controls.len(), POPUP_WIDGETS.len());
        let first = &positions.controls[0];
        assert_eq!(first.path, vec!["widgets", "network", "position"]);
        if let Kind::Options(opts) = &first.kind {
            assert_eq!(opts.len(), 10);
        } else {
            panic!("expected options");
        }
    }

    #[test]
    fn widgets_page_covers_personalization_sections() {
        let sections = match &build().into_iter().find(|p| p.id == "d_widgets").unwrap().body {
            Body::Controls(s) => s.clone(),
            _ => unreachable!(),
        };
        assert_eq!(sections.len(), PERSONALIZATION.len());
        let lock = sections.iter().find(|s| s.title == "Lock screen").unwrap();
        for key in ["wallpaperBlur", "showClock", "clockScale"] {
            assert!(lock.controls.iter().any(|c| c.path == ["lock", key]));
        }
    }
}
