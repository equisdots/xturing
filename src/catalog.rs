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
    pub body: Body,
}

pub enum Body {
    Controls(Vec<Section>),
    Keybinds,
    Startup,
    Zones,
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
    ZoneAdd,
    ZonesCenterAll,
    ZonesDefault,
    PaletteReset,
    PaletteCreate,
    PaletteDelete,
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
fn stepper(
    label: &str,
    path: &[&str],
    step: f64,
    min: f64,
    max: f64,
    decimals: u32,
    unit: &'static str,
    help: &str,
) -> Control {
    control(
        label,
        Kind::Stepper {
            step,
            min,
            max,
            decimals,
            unit,
        },
        p(path),
        help,
    )
}

fn options(label: &str, path: &[&str], opts: &[(&str, &str)], help: &str) -> Control {
    control(
        label,
        Kind::Options(
            opts.iter()
                .map(|(l, v)| Opt {
                    label: l.to_string(),
                    value: v.to_string(),
                })
                .collect(),
        ),
        p(path),
        help,
    )
}

fn text(label: &str, path: &[&str], placeholder: &str, max: usize, help: &str) -> Control {
    control(
        label,
        Kind::Text {
            placeholder: placeholder.to_string(),
            max,
        },
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
    Section {
        title: title.to_string(),
        controls,
    }
}

// ───────────────────────────── catalog ─────────────────────────────

fn bar_general_controls() -> Vec<Section> {
    vec![
        section(
            "Bar",
            vec![
                toggle(
                    "Bar enabled",
                    &["bar", "enabled"],
                    "Master switch: off unmaps the bar on every screen",
                ),
                toggle(
                    "Drag modules",
                    &["bar", "dragModules"],
                    "Allow dragging modules between zones",
                ),
                stepper(
                    "Opacity",
                    &["bar", "barOpacity"],
                    0.05,
                    0.2,
                    1.0,
                    2,
                    "×",
                    "Bar background opacity",
                ),
                stepper(
                    "Corner roundness",
                    &["bar", "roundness"],
                    0.05,
                    0.0,
                    1.0,
                    2,
                    "",
                    "Corner rounding of the bar surfaces",
                ),
            ],
        ),
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
                    "auto = the widget's own layout; the rest anchor with a 20px scaled margin",
                ));
            }
            out
        }),
    ]
}

fn appearance_controls() -> Vec<Section> {
    vec![section(
        "Structure tokens",
        vec![
            stepper(
                "Shape scale",
                &["appearance", "shapeScale"],
                0.1,
                0.5,
                2.0,
                2,
                "×",
                "Radius/shape scale for every surface",
            ),
            stepper(
                "Density scale",
                &["appearance", "densityScale"],
                0.1,
                0.5,
                2.0,
                2,
                "×",
                "Spacing/padding scale",
            ),
            stepper(
                "Motion scale",
                &["appearance", "motionScale"],
                0.1,
                0.5,
                2.0,
                2,
                "×",
                "Animation duration scale",
            ),
        ],
    )]
}

fn frame_controls() -> Vec<Section> {
    vec![section(
        "Frame",
        vec![
            toggle("Enabled", &["frame", "enabled"], "Draw the screen frame"),
            toggle(
                "Border line",
                &["frame", "line"],
                "Draw the inner line that hugs windows",
            ),
            stepper(
                "Band width (top/bottom)",
                &["frame", "thickness"],
                1.0,
                0.0,
                64.0,
                0,
                "px",
                "Band width on top/bottom",
            ),
            stepper(
                "Band width (left/right)",
                &["frame", "sideThickness"],
                1.0,
                0.0,
                64.0,
                0,
                "px",
                "Band width on left/right",
            ),
            stepper(
                "Line width",
                &["frame", "lineWidth"],
                1.0,
                0.0,
                16.0,
                0,
                "px",
                "Inner line stroke width",
            ),
            stepper(
                "Corner radius",
                &["frame", "radius"],
                1.0,
                0.0,
                128.0,
                0,
                "px",
                "Screen corner radius",
            ),
            stepper(
                "Inset",
                &["frame", "inset"],
                1.0,
                0.0,
                128.0,
                0,
                "px",
                "Outer edge offset from the screen edge",
            ),
            stepper(
                "Gap",
                &["frame", "gap"],
                1.0,
                0.0,
                64.0,
                0,
                "px",
                "Gap between the frame and an obstacle",
            ),
            stepper(
                "Line opacity",
                &["frame", "alpha"],
                0.05,
                0.0,
                1.0,
                2,
                "",
                "Border alpha",
            ),
            options(
                "Color",
                &["frame", "color"],
                &[
                    ("Overlay 0", "overlay0"),
                    ("Overlay 1", "overlay1"),
                    ("Overlay 2", "overlay2"),
                    ("Mauve", "mauve"),
                    ("Blue", "blue"),
                    ("Green", "green"),
                    ("Red", "red"),
                    ("Yellow", "yellow"),
                ],
                "Border colour (palette role or #rrggbb)",
            ),
            stepper(
                "Notch radius",
                &["frame", "notchRadius"],
                1.0,
                0.0,
                64.0,
                0,
                "px",
                "Corner radius of the notches",
            ),
            toggle(
                "Reserve space",
                &["frame", "reserve"],
                "Reserve space so windows never cross the frame",
            ),
            stepper(
                "Reserve extra",
                &["frame", "reserveExtra"],
                1.0,
                0.0,
                64.0,
                0,
                "px",
                "Extra reserved px",
            ),
        ],
    )]
}

pub fn build() -> Vec<Page> {
    vec![
        // ═══════════════ SHELL ═══════════════
        Page {
            id: "s_general",
            group: Group::Shell,
            label: "General",
            body: Body::Controls(vec![
                section(
                    "Display",
                    vec![
                        stepper(
                            "UI Scale",
                            &["uiScale"],
                            0.1,
                            0.5,
                            2.0,
                            1,
                            "x",
                            "Global shell scale (0.5 – 2.0)",
                        ),
                        stepper(
                            "App Scale",
                            &["appScale"],
                            0.25,
                            0.75,
                            2.0,
                            2,
                            "x",
                            "App scale: runs scale-menu.sh on change",
                        ),
                        stepper(
                            "Workspaces",
                            &["workspaceCount"],
                            1.0,
                            2.0,
                            10.0,
                            0,
                            "",
                            "2 – 10; reloads the topbar on change",
                        ),
                    ],
                ),
                section(
                    "Keyboard",
                    vec![
                        text(
                            "Keyboard layouts",
                            &["language"],
                            "us,es",
                            128,
                            "Comma-separated xkb codes (us, gb, es, latam, dvorak, us-intl...)",
                        ),
                        options(
                            "Layout shortcut",
                            &["kbOptions"],
                            &[
                                ("Alt + Shift", "grp:alt_shift_toggle"),
                                ("Win + Space", "grp:win_space_toggle"),
                                ("Caps Lock", "grp:caps_toggle"),
                                ("Ctrl + Shift", "grp:ctrl_shift_toggle"),
                                ("Ctrl + Alt", "grp:ctrl_alt_toggle"),
                                ("Right Alt", "grp:toggle"),
                                ("No Toggle", ""),
                            ],
                            "Shortcut to switch keyboard layouts",
                        ),
                    ],
                ),
                section(
                    "Wallpaper",
                    vec![
                        text(
                            "Wallpaper directory",
                            &["wallpaperDir"],
                            "~/.config/hypr/wallpapers",
                            512,
                            "Wallpaper directory",
                        ),
                        action(
                            "Change wallpaper",
                            Action::OpenWidget("wallpaper".into()),
                            "Open the davincix picker (wallpaper widget)",
                        ),
                    ],
                ),
            ]),
        },
        Page {
            id: "s_keyboard",
            group: Group::Shell,
            label: "Keyboard",
            body: Body::Keybinds,
        },
        Page {
            id: "s_monitors",
            group: Group::Shell,
            label: "Monitors",
            body: Body::Monitors,
        },
        Page {
            id: "s_startup",
            group: Group::Shell,
            label: "Startup",
            body: Body::Startup,
        },
        // ═══════════════ BAR ═══════════════
        Page {
            id: "d_general",
            group: Group::Bar,
            label: "General",
            body: Body::Controls(bar_general_controls()),
        },
        Page {
            id: "d_position",
            group: Group::Bar,
            label: "Position",
            body: Body::Controls(vec![section(
                "Bar position",
                vec![options(
                    "Position",
                    &["bar", "position"],
                    &[
                        ("Top", "top"),
                        ("Bottom", "bottom"),
                        ("Left", "left"),
                        ("Right", "right"),
                    ],
                    "Bar position",
                )],
            )]),
        },
        Page {
            id: "d_style",
            group: Group::Bar,
            label: "Style",
            body: Body::Controls(bar_style_controls()),
        },
        Page {
            id: "d_zones",
            group: Group::Bar,
            label: "Zones",
            body: Body::Zones,
        },
        Page {
            id: "d_modules",
            group: Group::Bar,
            label: "Modules",
            body: Body::Controls(modules_controls()),
        },
        Page {
            id: "d_workspaces",
            group: Group::Bar,
            label: "Workspaces",
            body: Body::Controls(vec![section(
                "Workspaces marker",
                vec![
                    options(
                        "Marker",
                        &["bar", "workspacesMarker"],
                        &[
                            ("Numbers", "number"),
                            ("Dots", "dot"),
                            ("Letters", "letter"),
                            ("Custom", "custom"),
                        ],
                        "How empty workspaces are drawn",
                    ),
                    visible(
                        text(
                            "Marker character",
                            &["bar", "workspacesMarkerText"],
                            "•",
                            4,
                            "Up to 4 characters; empty uses •",
                        ),
                        Cond::Eq(
                            p(&["bar", "workspacesMarker"]),
                            Value::String("custom".into()),
                        ),
                    ),
                    info("Workspace count", "workspaceCount"),
                ],
            )]),
        },
        // ═══════════════ THEME ═══════════════
        Page {
            id: "d_appearance",
            group: Group::Theme,
            label: "Appearance",
            body: Body::Controls(appearance_controls()),
        },
        Page {
            id: "d_palette",
            group: Group::Theme,
            label: "Palette",
            body: Body::Palette,
        },
        Page {
            id: "d_animations",
            group: Group::Theme,
            label: "Animations",
            body: Body::Controls(vec![
                section(
                    "General",
                    vec![toggle(
                        "Enabled",
                        &["animations", "enabled"],
                        "Hyprland animations (user-animations.lua)",
                    )],
                ),
                section(
                    "Speed",
                    vec![
                        options(
                            "Preset",
                            &["animations", "speed"],
                            &[
                                ("Snappy", "0.6"),
                                ("Fast", "0.8"),
                                ("Normal", "1.0"),
                                ("Slow", "1.5"),
                            ],
                            "Speed presets; higher value = slower",
                        ),
                        stepper(
                            "Speed",
                            &["animations", "speed"],
                            0.1,
                            0.5,
                            2.0,
                            1,
                            "x",
                            "0.5 – 2.0",
                        ),
                    ],
                ),
            ]),
        },
        Page {
            id: "d_shadows",
            group: Group::Theme,
            label: "Shadows",
            body: Body::Controls(vec![
                section(
                    "Shape",
                    vec![
                        toggle(
                            "Enabled",
                            &["shadows", "enabled"],
                            "Shell shadows (panels, bar)",
                        ),
                        stepper(
                            "Blur",
                            &["shadows", "blur"],
                            2.0,
                            0.0,
                            60.0,
                            0,
                            "px",
                            "0 – 60",
                        ),
                        stepper(
                            "Spread",
                            &["shadows", "spread"],
                            1.0,
                            -10.0,
                            20.0,
                            0,
                            "px",
                            "-10 – 20",
                        ),
                        stepper(
                            "Corner radius",
                            &["shadows", "radius"],
                            1.0,
                            0.0,
                            30.0,
                            0,
                            "px",
                            "0 – 30",
                        ),
                    ],
                ),
                section(
                    "Placement",
                    vec![
                        stepper(
                            "Offset X",
                            &["shadows", "offsetX"],
                            2.0,
                            -40.0,
                            40.0,
                            0,
                            "px",
                            "-40 – 40",
                        ),
                        stepper(
                            "Offset Y",
                            &["shadows", "offsetY"],
                            2.0,
                            -40.0,
                            40.0,
                            0,
                            "px",
                            "-40 – 40",
                        ),
                        stepper(
                            "Opacity",
                            &["shadows", "opacity"],
                            0.05,
                            0.0,
                            1.0,
                            2,
                            "",
                            "0 – 1",
                        ),
                    ],
                ),
            ]),
        },
        Page {
            id: "d_frame",
            group: Group::Theme,
            label: "Frame",
            body: Body::Controls(frame_controls()),
        },
        Page {
            id: "d_glass",
            group: Group::Theme,
            label: "Glass",
            body: Body::Controls(vec![section(
                "Glassmorphism",
                vec![
                    toggle(
                        "Enabled",
                        &["glass", "enabled"],
                        "Translucent background on shell surfaces",
                    ),
                    stepper(
                        "Background opacity",
                        &["glass", "opacity"],
                        0.05,
                        0.4,
                        1.0,
                        2,
                        "",
                        "0.4 – 1.0",
                    ),
                ],
            )]),
        },
        // ═══════════════ BEHAVIOR ═══════════════
        Page {
            id: "d_launcher",
            group: Group::Behavior,
            label: "Launcher",
            body: Body::Controls(launcher_controls()),
        },
        Page {
            id: "d_notifications",
            group: Group::Behavior,
            label: "Notifications",
            body: Body::Controls(notification_controls()),
        },
        // ═══════════════ WIDGETS ═══════════════
        Page {
            id: "d_widgets",
            group: Group::Widgets,
            label: "Popups",
            body: Body::Controls(popup_controls()),
        },
        // ═══════════════ SYSTEM ═══════════════
        Page {
            id: "d_hyprland",
            group: Group::System,
            label: "Hyprland",
            body: Body::HyprEffects,
        },
        Page {
            id: "d_input",
            group: Group::System,
            label: "Input",
            body: Body::Controls(vec![
                section(
                    "Mouse",
                    vec![
                        stepper(
                            "Sensitivity",
                            &["input", "sensitivity"],
                            0.1,
                            -1.0,
                            1.0,
                            1,
                            "",
                            "-1.0 – 1.0",
                        ),
                        options(
                            "Accel profile",
                            &["input", "accelProfile"],
                            &[("Adaptive", "adaptive"), ("Flat", "flat")],
                            "Pointer acceleration profile",
                        ),
                    ],
                ),
                section(
                    "Touchpad",
                    vec![
                        toggle("Tap to click", &["input", "tapToClick"], "Tap to click"),
                        toggle(
                            "Natural scroll",
                            &["input", "naturalScroll"],
                            "Scroll natural",
                        ),
                        toggle(
                            "Disable while typing",
                            &["input", "disableWhileTyping"],
                            "Disable the touchpad while typing",
                        ),
                    ],
                ),
            ]),
        },
        Page {
            id: "d_gpu",
            group: Group::System,
            label: "GPU",
            body: Body::Controls(vec![
                section(
                    "Optimus mode",
                    vec![
                        control(
                            "Mode",
                            Kind::StateOptions {
                                state_key: "gpu".into(),
                                options: vec![
                                    Opt {
                                        label: "Integrated".into(),
                                        value: "integrated".into(),
                                    },
                                    Opt {
                                        label: "Hybrid".into(),
                                        value: "hybrid".into(),
                                    },
                                    Opt {
                                        label: "NVIDIA".into(),
                                        value: "nvidia".into(),
                                    },
                                ],
                                action: Action::GpuMode,
                            },
                            Vec::new(),
                            "Switches with envycontrol; may require logout/reboot",
                        ),
                        info("Current mode", "gpu.mode"),
                        action("Refresh", Action::GpuRefresh, "Relee envycontrol --query"),
                    ],
                ),
                section(
                    "Environment (static)",
                    vec![
                        info("GBM_BACKEND", "gpu.env.gbm"),
                        info("LIBVA_DRIVER_NAME", "gpu.env.libva"),
                    ],
                ),
            ]),
        },
        Page {
            id: "d_idle",
            group: Group::System,
            label: "Idle",
            body: Body::Controls(vec![
                section(
                    "Idle mode",
                    vec![
                        control(
                            "Mode",
                            Kind::StateOptions {
                                state_key: "idle".into(),
                                options: vec![
                                    Opt {
                                        label: "Auto".into(),
                                        value: "normal".into(),
                                    },
                                    Opt {
                                        label: "Awake".into(),
                                        value: "awake".into(),
                                    },
                                ],
                                action: Action::IdleMode,
                            },
                            Vec::new(),
                            "Awake detiene hypridle: nada auto-dim, lock ni suspend",
                        ),
                        info("Current mode", "idle.mode"),
                    ],
                ),
                section(
                    "Manual actions",
                    vec![
                        action("Lock now", Action::LockNow, "Lock the session (SUPER+L)"),
                        action("Suspend", Action::Suspend, "systemctl suspend"),
                    ],
                ),
            ]),
        },
        Page {
            id: "d_guide",
            group: Group::System,
            label: "About",
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
    Int {
        min: f64,
        max: f64,
        step: f64,
    },
    Float {
        min: f64,
        max: f64,
        step: f64,
        decimals: u32,
    },
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
        PKey {
            key: $key,
            kind: PKind::Bool,
        }
    };
    ($key:literal, int, $min:expr, $max:expr, $step:expr) => {
        PKey {
            key: $key,
            kind: PKind::Int {
                min: $min,
                max: $max,
                step: $step,
            },
        }
    };
    ($key:literal, float, $min:expr, $max:expr, $step:expr, $dec:expr) => {
        PKey {
            key: $key,
            kind: PKind::Float {
                min: $min,
                max: $max,
                step: $step,
                decimals: $dec,
            },
        }
    };
}

static PERSONALIZATION: &[PSec] = &[
    PSec {
        id: "network",
        label: "Wifi / Bluetooth",
        keys: &[
            pk!("popupWidth", int, 200.0, 4000.0, 10.0),
            pk!("popupHeight", int, 200.0, 4000.0, 10.0),
            pk!("powerAnimMs", int, 0.0, 5000.0, 50.0),
            pk!("busyTimeoutMs", int, 0.0, 60000.0, 500.0),
            pk!("failClearMs", int, 0.0, 30000.0, 500.0),
        ],
    },
    PSec {
        id: "volume",
        label: "Sound",
        keys: &[
            pk!("popupWidth", int, 200.0, 2000.0, 10.0),
            pk!("step", int, 1.0, 50.0, 1.0),
            pk!("syncDelayMs", int, 0.0, 5000.0, 50.0),
            pk!("pollMs", int, 100.0, 10000.0, 100.0),
        ],
    },
    PSec {
        id: "battery",
        label: "Battery",
        keys: &[
            pk!("popupWidth", int, 200.0, 2000.0, 10.0),
            pk!("lowThreshold", int, 0.0, 100.0, 5.0),
            pk!("pollMs", int, 100.0, 60000.0, 500.0),
        ],
    },
    PSec {
        id: "system-monitor",
        label: "System monitor",
        keys: &[
            pk!("popupWidth", int, 200.0, 2000.0, 10.0),
            pk!("popupHeight", int, 200.0, 2000.0, 10.0),
            pk!("pollMs", int, 100.0, 60000.0, 500.0),
            pk!("historyLength", int, 1.0, 600.0, 5.0),
        ],
    },
    PSec {
        id: "calendar",
        label: "Calendar",
        keys: &[
            pk!("popupWidth", int, 200.0, 4000.0, 10.0),
            pk!("popupHeight", int, 200.0, 4000.0, 10.0),
        ],
    },
    PSec {
        id: "clipboard",
        label: "Clipboard",
        keys: &[
            pk!("popupWidth", int, 200.0, 2000.0, 10.0),
            pk!("popupHeight", int, 200.0, 2000.0, 10.0),
            pk!("fetchLimit", int, 1.0, 500.0, 1.0),
        ],
    },
    PSec {
        id: "quicknotes",
        label: "Notepad",
        keys: &[
            pk!("popupWidth", int, 200.0, 2000.0, 10.0),
            pk!("popupHeight", int, 200.0, 2000.0, 10.0),
            pk!("saveDebounceMs", int, 0.0, 10000.0, 100.0),
        ],
    },
    PSec {
        id: "rss-reader",
        label: "RSS reader",
        keys: &[
            pk!("popupWidth", int, 200.0, 2000.0, 10.0),
            pk!("popupHeight", int, 200.0, 2000.0, 10.0),
            pk!("refreshMs", int, 10000.0, 3600000.0, 30000.0),
        ],
    },
    PSec {
        id: "file-search",
        label: "File search",
        keys: &[
            pk!("popupWidth", int, 200.0, 2000.0, 10.0),
            pk!("popupHeight", int, 200.0, 2000.0, 10.0),
            pk!("debounceMs", int, 0.0, 5000.0, 50.0),
        ],
    },
    PSec {
        id: "scale",
        label: "Scale picker",
        keys: &[
            pk!("popupWidth", int, 200.0, 2000.0, 10.0),
            pk!("popupHeight", int, 200.0, 2000.0, 10.0),
            pk!("pollMs", int, 50.0, 5000.0, 50.0),
        ],
    },
    PSec {
        id: "window-controls",
        label: "Window controls",
        keys: &[
            pk!("popupWidth", int, 200.0, 2000.0, 10.0),
            pk!("popupHeight", int, 200.0, 2000.0, 10.0),
            pk!("activeOpacity", float, 0.0, 1.0, 0.05, 2),
            pk!("inactiveOpacity", float, 0.0, 1.0, 0.05, 2),
            pk!("blurSize", int, 0.0, 24.0, 1.0),
            pk!("blurPasses", int, 0.0, 10.0, 1.0),
            pk!("roundness", int, 0.0, 40.0, 1.0),
        ],
    },
    PSec {
        id: "lock",
        label: "Lock screen",
        keys: &[
            pk!("revealDurationMs", int, 0.0, 5000.0, 50.0),
            pk!("clockPollMs", int, 100.0, 10000.0, 100.0),
            pk!("batteryPollMs", int, 100.0, 60000.0, 500.0),
            pk!("wallpaperBlur", float, 0.0, 1.0, 0.05, 2),
            pk!("wallpaperDim", float, 0.0, 0.85, 0.05, 2),
            pk!("showClock", bool),
            pk!("showBattery", bool),
            pk!("showPower", bool),
            pk!("clockScale", float, 0.6, 1.6, 0.05, 2),
        ],
    },
    PSec {
        id: "updater",
        label: "Updater",
        keys: &[
            pk!("popupWidth", int, 200.0, 4000.0, 10.0),
            pk!("popupHeight", int, 200.0, 4000.0, 10.0),
        ],
    },
    PSec {
        id: "idle",
        label: "Idle",
        keys: &[
            pk!("popupWidth", int, 200.0, 2000.0, 10.0),
            pk!("popupHeight", int, 200.0, 2000.0, 10.0),
        ],
    },
    PSec {
        id: "tray",
        label: "System tray",
        keys: &[
            pk!("tint", bool),
            pk!("useAccent", bool),
            pk!("size", int, 8.0, 48.0, 2.0),
        ],
    },
    PSec {
        id: "widgets",
        label: "Desktop widgets",
        keys: &[
            pk!("redactorWidth", int, 200.0, 8000.0, 10.0),
            pk!("redactorHeight", int, 200.0, 8000.0, 10.0),
        ],
    },
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
                    Kind::Stepper {
                        step: *step,
                        min: *min,
                        max: *max,
                        decimals: 0,
                        unit: "",
                    },
                    full,
                    "",
                )),
                PKind::Float {
                    min,
                    max,
                    step,
                    decimals,
                } => controls.push(control(
                    &label,
                    Kind::Stepper {
                        step: *step,
                        min: *min,
                        max: *max,
                        decimals: *decimals,
                        unit: "",
                    },
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

// ───────────────────────────── bar style / borders ─────────────────────────────

pub fn border_sections() -> Vec<Section> {
    vec![border_controls()]
}

fn border_controls() -> Section {
    section(
        "Window borders",
        vec![
            toggle(
                "Follow palette",
                &["bar", "borderFollowPalette"],
                "Window borders follow the palette accents",
            ),
            options(
                "Active color",
                &["bar", "borderActive"],
                &[("Empty (palette)", "")],
                "Hex #rrggbb; empty = derived from the palette",
            ),
            options(
                "Active gradient",
                &["bar", "borderGradientActive"],
                &[("Off", "false"), ("On", "true")],
                "Gradient on the active border",
            ),
            options(
                "Active 2nd color",
                &["bar", "borderActive2"],
                &[("Empty", "")],
                "Second color of the active gradient",
            ),
            stepper(
                "Active angle",
                &["bar", "borderAngleActive"],
                15.0,
                0.0,
                360.0,
                0,
                "°",
                "0 – 360",
            ),
            options(
                "Inactive color",
                &["bar", "borderInactive"],
                &[("Empty (palette)", "")],
                "Hex #rrggbb; empty = derived from the palette",
            ),
            options(
                "Inactive gradient",
                &["bar", "borderGradientInactive"],
                &[("Off", "false"), ("On", "true")],
                "Gradient on the inactive border",
            ),
            options(
                "Inactive 2nd color",
                &["bar", "borderInactive2"],
                &[("Empty", "")],
                "Second color of the inactive gradient",
            ),
            stepper(
                "Inactive angle",
                &["bar", "borderAngleInactive"],
                15.0,
                0.0,
                360.0,
                0,
                "°",
                "0 – 360",
            ),
        ],
    )
}

fn bar_style_controls() -> Vec<Section> {
    vec![
        section(
            "Presets",
            vec![options(
                "Style preset",
                &["bar", "stylePreset"],
                &[("Modular", "modular"), ("Solid", "solid"), ("Fill", "fill")],
                "Applies a bundle of flags (pillBg/pillSolid/barBg/edgeGap)",
            )],
        ),
        section(
            "Shape",
            vec![
                stepper(
                    "Roundness",
                    &["bar", "roundness"],
                    0.1,
                    0.0,
                    1.0,
                    1,
                    "",
                    "0 – 1",
                ),
                stepper(
                    "Thickness",
                    &["bar", "thickness"],
                    4.0,
                    24.0,
                    96.0,
                    0,
                    "px",
                    "24 – 96",
                ),
                stepper(
                    "Edge margin",
                    &["bar", "edgeGap"],
                    2.0,
                    0.0,
                    24.0,
                    0,
                    "px",
                    "0 – 24",
                ),
                stepper(
                    "Bar opacity",
                    &["bar", "barOpacity"],
                    0.05,
                    0.2,
                    1.0,
                    2,
                    "",
                    "0.2 – 1.0",
                ),
            ],
        ),
        section(
            "Fill",
            vec![
                toggle("Island fill", &["bar", "pillBg"], "Island/pill background"),
                toggle(
                    "Solid fill",
                    &["bar", "pillSolid"],
                    "Solid fill for islands",
                ),
                toggle(
                    "Unified bar",
                    &["bar", "barBg"],
                    "Unified full-bar background",
                ),
                toggle(
                    "Drag modules",
                    &["bar", "dragModules"],
                    "Allow dragging modules on the bar",
                ),
            ],
        ),
        section(
            "Font",
            vec![
                text(
                    "Font",
                    &["bar", "font"],
                    "Hack Nerd Font",
                    128,
                    "Shell font; empty resets to Hack Nerd Font",
                ),
                text(
                    "Time format",
                    &["bar", "timeFormat"],
                    "HH:mm:ss",
                    64,
                    "Qt (QML) clock format",
                ),
                text(
                    "Date format",
                    &["bar", "dateFormat"],
                    "dddd, MMMM dd",
                    64,
                    "Qt (QML) date format",
                ),
            ],
        ),
        section(
            "Borders",
            vec![
                stepper(
                    "Border width",
                    &["bar", "borderWidth"],
                    1.0,
                    0.0,
                    8.0,
                    0,
                    "px",
                    "0 – 8",
                ),
                options(
                    "Border color",
                    &["bar", "borderColor"],
                    &[
                        ("surface1", "surface1"),
                        ("surface0", "surface0"),
                        ("text", "text"),
                        ("red", "red"),
                        ("blue", "blue"),
                        ("green", "green"),
                        ("yellow", "yellow"),
                        ("mauve", "mauve"),
                        ("teal", "teal"),
                    ],
                    "Bar border color role",
                ),
            ],
        ),
        border_controls(),
    ]
}

// ───────────────────────────── modules ─────────────────────────────

pub(crate) const MODULES: &[(&str, &str, bool)] = &[
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
            Opt {
                label: "Default".into(),
                value: "default".into(),
            },
            Opt {
                label: "Filled".into(),
                value: "on".into(),
            },
            Opt {
                label: "None".into(),
                value: "off".into(),
            },
        ]),
        path,
        help: String::new(),
        visible: None,
    }
}

fn modules_controls() -> Vec<Section> {
    let mut sections = vec![section(
        "Global",
        vec![text(
            "Icon color",
            &["bar", "iconColor"],
            "default (module roles)",
            64,
            "colors.* role or #hex; applies to modules without their own color",
        )],
    )];
    for (id, label, icon_ok) in MODULES {
        let m = |key: &str| p(&["bar", "modules", id, key]);
        let mut controls = vec![];
        if *icon_ok {
            controls.push(control(
                "Icon",
                Kind::Text {
                    placeholder: "default".into(),
                    max: 4,
                },
                m("icon"),
                "Nerd Font glyph (max 4)",
            ))
        }
        controls.push(control(
            "Color",
            Kind::Text {
                placeholder: "default".into(),
                max: 64,
            },
            m("color"),
            "Rol colors.* o #hex",
        ));
        controls.push(control(
            "Accent",
            Kind::Text {
                placeholder: "default".into(),
                max: 64,
            },
            m("accent"),
            "Accent role",
        ));
        controls.push(fill_control("Fill", m("fill")));
        match *id {
            "time" => {
                controls.push(text(
                    "Clock format",
                    &["bar", "timeFormat"],
                    "HH:mm:ss",
                    64,
                    "Formato Qt",
                ));
                controls.push(control(
                    "Size",
                    Kind::Stepper {
                        step: 1.0,
                        min: 0.0,
                        max: 48.0,
                        decimals: 0,
                        unit: "px",
                    },
                    m("size"),
                    "0 = module default",
                ));
                controls.push(control(
                    "Effect",
                    Kind::Options(vec![
                        Opt {
                            label: "No effect".into(),
                            value: String::new(),
                        },
                        Opt {
                            label: "Typewriter".into(),
                            value: "typewriter".into(),
                        },
                    ]),
                    m("effect"),
                    "",
                ));
                controls.push(control(
                    "Cursor",
                    Kind::Toggle,
                    m("cursor"),
                    "Cursor parpadeante",
                ));
            }
            "date" => {
                controls.push(text(
                    "Date format",
                    &["bar", "dateFormat"],
                    "dddd, MMMM dd",
                    64,
                    "Formato Qt",
                ));
                controls.push(control(
                    "Size",
                    Kind::Stepper {
                        step: 1.0,
                        min: 0.0,
                        max: 48.0,
                        decimals: 0,
                        unit: "px",
                    },
                    m("size"),
                    "0 = module default",
                ));
            }
            "workspaces" => {
                controls.push(control(
                    "Marker",
                    Kind::Options(vec![
                        Opt {
                            label: "Numbers".into(),
                            value: "number".into(),
                        },
                        Opt {
                            label: "Dots".into(),
                            value: "dot".into(),
                        },
                        Opt {
                            label: "Letters".into(),
                            value: "letter".into(),
                        },
                        Opt {
                            label: "Custom".into(),
                            value: "custom".into(),
                        },
                    ]),
                    m("marker"),
                    "",
                ));
                let slots = [
                    "active",
                    "activeText",
                    "occupied",
                    "empty",
                    "hover",
                    "marker",
                    "markerEmpty",
                ];
                for slot in slots {
                    controls.push(control(
                        &format!("Color: {}", pretty(slot)),
                        Kind::Text {
                            placeholder: "default".into(),
                            max: 64,
                        },
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
        section(
            "Position",
            vec![options(
                "Position",
                &["launcher", "position"],
                &[
                    ("Center", "center"),
                    ("Top", "top"),
                    ("Bottom", "bottom"),
                    ("Left", "left"),
                    ("Right", "right"),
                ],
                "",
            )],
        ),
        section(
            "Size",
            vec![
                stepper(
                    "Width",
                    &["launcher", "width"],
                    40.0,
                    320.0,
                    1280.0,
                    0,
                    "px",
                    "320 – 1280",
                ),
                stepper(
                    "Visible apps",
                    &["launcher", "maxApps"],
                    1.0,
                    4.0,
                    20.0,
                    0,
                    "",
                    "4 – 20",
                ),
                stepper(
                    "Margin",
                    &["launcher", "margin"],
                    8.0,
                    -200.0,
                    200.0,
                    0,
                    "px",
                    "-200 – 200",
                ),
                stepper(
                    "Row height",
                    &["launcher", "rowHeight"],
                    4.0,
                    28.0,
                    80.0,
                    0,
                    "px",
                    "28 – 80",
                ),
            ],
        ),
        section(
            "Borders",
            vec![
                stepper(
                    "Border width",
                    &["launcher", "borderWidth"],
                    1.0,
                    0.0,
                    4.0,
                    0,
                    "px",
                    "0 hides the border",
                ),
                stepper(
                    "Radius",
                    &["launcher", "radius"],
                    2.0,
                    0.0,
                    28.0,
                    0,
                    "px",
                    "0 – 28",
                ),
                options(
                    "Border color",
                    &["launcher", "borderColor"],
                    &[
                        ("surface1", "surface1"),
                        ("surface0", "surface0"),
                        ("text", "text"),
                        ("red", "red"),
                        ("blue", "blue"),
                        ("green", "green"),
                        ("yellow", "yellow"),
                        ("mauve", "mauve"),
                        ("teal", "teal"),
                    ],
                    "Color role",
                ),
            ],
        ),
        section(
            "Behavior",
            vec![
                toggle(
                    "Avoid bar",
                    &["launcher", "avoidBar"],
                    "Avoid overlapping the bar",
                ),
                toggle(
                    "Show icons",
                    &["launcher", "showIcons"],
                    "Show application icons",
                ),
            ],
        ),
        section(
            "Content alignment",
            vec![options(
                "Align",
                &["launcher", "align"],
                &[("Left", "left"), ("Center", "center"), ("Right", "right")],
                "",
            )],
        ),
    ]
}

// ───────────────────────────── notifications ─────────────────────────────

fn notification_controls() -> Vec<Section> {
    vec![section(
        "Notifications layout",
        vec![
            stepper(
                "Width",
                &["notifications", "width"],
                5.0,
                0.0,
                1200.0,
                0,
                "px",
                "",
            ),
            stepper(
                "Max height (0 = auto)",
                &["notifications", "maxHeight"],
                5.0,
                0.0,
                2000.0,
                0,
                "px",
                "",
            ),
            stepper(
                "Shadow blur",
                &["notifications", "shadowBlur"],
                1.0,
                0.0,
                80.0,
                0,
                "px",
                "",
            ),
            stepper(
                "Shadow offset",
                &["notifications", "shadowOffset"],
                1.0,
                0.0,
                40.0,
                0,
                "px",
                "",
            ),
            options(
                "Window shadow",
                &["notifications", "shadow"],
                &[("Off", "0"), ("On", "1")],
                "",
            ),
            options(
                "Position",
                &["notifications", "position"],
                &[
                    ("Top Left", "0"),
                    ("Top Center", "1"),
                    ("Top Right", "2"),
                    ("Bottom Left", "3"),
                    ("Bottom Center", "4"),
                    ("Bottom Right", "5"),
                ],
                "",
            ),
            control(
                "Do Not Disturb",
                Kind::Action(Action::DndToggle),
                Vec::new(),
                "Silence popups (state file)",
            ),
        ],
    )]
}

pub const HYPR_SLIDERS: &[(&str, &str, f64, f64, f64, f64, &str)] = &[
    (
        "active_opacity",
        "Active Opacity",
        0.85,
        0.30,
        1.0,
        0.05,
        "2",
    ),
    (
        "inactive_opacity",
        "Inactive Opacity",
        0.80,
        0.30,
        1.0,
        0.05,
        "2",
    ),
    ("rounding", "Rounding", 20.0, 0.0, 35.0, 1.0, "0"),
    ("blur_size", "Blur Size", 8.0, 0.0, 24.0, 1.0, "0"),
    ("blur_passes", "Blur Passes", 3.0, 0.0, 10.0, 1.0, "0"),
    ("gaps_in", "Gaps In", 16.0, 0.0, 50.0, 2.0, "0"),
    ("gaps_out", "Gaps Out", 25.0, 0.0, 50.0, 2.0, "0"),
    ("border_size", "Border Width", 2.0, 0.0, 20.0, 1.0, "0"),
    ("shadow_range", "Shadow Range", 35.0, 0.0, 50.0, 1.0, "0"),
    (
        "shadow_render_power",
        "Shadow Power",
        5.0,
        0.0,
        10.0,
        1.0,
        "0",
    ),
    (
        "shadow_offset_x",
        "Shadow Offset X",
        0.0,
        -30.0,
        30.0,
        1.0,
        "0",
    ),
    (
        "shadow_offset_y",
        "Shadow Offset Y",
        10.0,
        -30.0,
        30.0,
        1.0,
        "0",
    ),
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
            "s_general",
            "s_keyboard",
            "s_monitors",
            "s_startup",
            "d_general",
            "d_position",
            "d_style",
            "d_zones",
            "d_modules",
            "d_workspaces",
            "d_appearance",
            "d_palette",
            "d_animations",
            "d_shadows",
            "d_frame",
            "d_glass",
            "d_launcher",
            "d_notifications",
            "d_widgets",
            "d_hyprland",
            "d_input",
            "d_gpu",
            "d_idle",
            "d_guide",
        ];
        for id in expect {
            assert!(pages.iter().any(|p| p.id == id), "missing page {id}");
        }
        assert_eq!(pages.len(), expect.len());
    }

    #[test]
    fn popup_positions_cover_all_registry_widgets() {
        let sections = match &build()
            .into_iter()
            .find(|p| p.id == "d_general")
            .unwrap()
            .body
        {
            Body::Controls(s) => s.clone(),
            _ => unreachable!(),
        };
        let positions = sections
            .iter()
            .find(|s| s.title == "Popup positions")
            .unwrap();
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
        let sections = match &build()
            .into_iter()
            .find(|p| p.id == "d_widgets")
            .unwrap()
            .body
        {
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
