//! Application state: navigation, editing, special pages, side effects.

use crate::actions;
use crate::catalog::{self, Action, Body, BodyKind, Cond, Control, Group, Kind, Opt, Page};
use crate::classic;
use crate::monitors::{self, Monitor};
use crate::palette::{self, PaletteEntry, PaletteFile};
use crate::settings::Settings;
use anyhow::Result;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

#[derive(Clone, PartialEq, Debug)]
pub enum Row {
    Section(String),
    Control(usize),
    Keybind(usize),
    Startup(usize),
    ZoneHeader(usize),
    ZoneField { zone: usize, field: usize },
    ZoneModule { zone: usize, idx: usize },
    ClassicItem { sec: usize, idx: usize },
    PaletteItem(usize),
    PaletteSlot(usize),
    MonitorField { mon: usize, field: usize },
    Info(String, String),
}

impl Row {
    pub fn selectable(&self) -> bool {
        !matches!(self, Row::Section(_) | Row::Info(..))
    }
}

#[derive(Clone)]
pub enum EditTarget {
    Control {
        path: Vec<String>,
        numeric: bool,
        label: String,
    },
    Keybind {
        kb: usize,
        field: usize,
    },
    Startup(usize),
    PaletteSlot {
        slot: usize,
        slug: String,
    },
    PaletteFilter,
    Prompt {
        prefix: String,
        prompt: String,
    },
}

pub enum Mode {
    Browse,
    Edit {
        target: EditTarget,
        buf: String,
        cursor: usize,
    },
    Confirm {
        action: Action,
        msg: String,
    },
    Chooser {
        target: ChooserTarget,
        idx: usize,
    },
    Form {
        kb: usize,
        field: usize,
    },
    Record {
        kb: usize,
    },
    Help,
    Search(SearchState),
}

pub struct SearchState {
    pub query: String,
    pub cursor: usize,
    pub results: Vec<SearchEntry>,
    pub sel: usize,
}

#[derive(Clone)]
pub struct SearchEntry {
    pub text: String,
    pub context: String,
    pub target: SearchTarget,
}

#[derive(Clone)]
pub enum SearchTarget {
    Page(usize),
    Control {
        page: usize,
        index: usize,
    },
    Zone {
        page: usize,
        zone: usize,
    },
    ZoneModule {
        page: usize,
        zone: usize,
        index: usize,
    },
    PaletteItem {
        page: usize,
        index: usize,
    },
    PaletteSlot {
        page: usize,
        slot: usize,
    },
    Monitor {
        page: usize,
        monitor: usize,
    },
    Keybind {
        page: usize,
        index: usize,
    },
    Startup {
        page: usize,
        index: usize,
    },
    Command(SearchCommand),
}

#[derive(Clone, Copy, PartialEq)]
pub enum SearchCommand {
    Reload,
    Quit,
    Help,
    ToggleEngine,
}

/// Clickable screen region registered every frame by the renderer.
#[derive(Clone)]
pub struct Hit {
    pub y: u16,
    pub x0: u16,
    pub x1: u16,
    pub action: HitAction,
}

#[derive(Clone)]
pub enum HitAction {
    SelectRow(usize),
    Adjust { ctrl: usize, delta: i32 },
    GotoPage(usize),
    SearchSelect(usize),
    ChooserSelect(usize),
}

#[derive(Clone, Copy)]
pub struct DragState {
    pub ctrl: usize,
    pub last_x: u16,
}

pub enum ChooserTarget {
    Control {
        path: Vec<String>,
        state_action: Option<Action>,
        opts: Vec<Opt>,
    },
    Keybind {
        kb: usize,
        field: usize,
    },
    Monitor {
        mon: usize,
        field: usize,
    },
}

#[derive(Clone)]
enum PendingKind {
    WorkspaceReload,
    TimexRefresh,
    PersistInput,
    PersistAnimations,
    HyprPreview,
}

struct Pending {
    at: Instant,
    kind: PendingKind,
}

pub const MONITOR_FIELDS: &[(&str, &str)] = &[
    ("Disabled", "toggle"),
    ("Resolution", "options"),
    ("Refresh rate", "options"),
    ("Transform", "options"),
    ("VRR", "options"),
    ("Bit depth", "options"),
    ("Color mode", "options"),
    ("Mirror", "options"),
    ("Position X", "stepper"),
    ("Position Y", "stepper"),
];

pub struct App {
    pub settings: Settings,
    pub pages: Vec<Page>,
    pub engine: String,
    pub page: usize,
    pub sel: usize,
    pub sel_memory: Vec<usize>,
    pub rows: Vec<Row>,
    pub controls: Vec<Control>,
    pub mode: Mode,
    pub status: String,
    pub quit: bool,
    pub infos: HashMap<String, String>,
    pub vstate: HashMap<String, String>,
    pub keybinds: Vec<Value>,
    pub startup: Vec<Value>,
    pub keybinds_dirty: bool,
    pub startup_dirty: bool,
    pub hits: Vec<Hit>,
    pub last_tap: Option<(usize, Instant)>,
    pub drag: Option<DragState>,
    search_index: Vec<SearchEntry>,
    pub palettes: Vec<PaletteEntry>,
    pub palette: Option<PaletteFile>,
    pub palette_filter: String,
    pub palette_delete_slug: Option<String>,
    pub monitors: Vec<Monitor>,
    pub hypr: HashMap<String, f64>,
    pub hypr_dirty: HashSet<String>,
    pending: Vec<Pending>,
    last_check: Instant,
    pub last_status_clear: Instant,
}

impl App {
    pub fn with_settings(settings: Settings) -> Result<Self> {
        let settings = settings;
        let engine = settings
            .get_str(&["barEngine"])
            .unwrap_or_else(|| "bar".into());
        let keybinds = settings
            .get(&["keybinds"])
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let startup = settings
            .get(&["startup"])
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let pages = catalog::build();
        let page_count = pages.len();
        let mut app = Self {
            settings,
            pages,
            engine,
            page: 0,
            sel: 0,
            sel_memory: vec![0; page_count],
            rows: Vec::new(),
            controls: Vec::new(),
            mode: Mode::Browse,
            status: String::new(),
            quit: false,
            infos: HashMap::new(),
            vstate: HashMap::new(),
            keybinds,
            startup,
            keybinds_dirty: false,
            startup_dirty: false,
            hits: Vec::new(),
            last_tap: None,
            drag: None,
            search_index: Vec::new(),
            palettes: Vec::new(),
            palette: None,
            palette_filter: String::new(),
            palette_delete_slug: None,
            monitors: Vec::new(),
            hypr: HashMap::new(),
            hypr_dirty: HashSet::new(),
            pending: Vec::new(),
            last_check: Instant::now(),
            last_status_clear: Instant::now(),
        };
        app.refresh_page_state();
        Ok(app)
    }

    // ───────────────────────── pages ─────────────────────────

    pub fn visible_page_indices(&self) -> Vec<usize> {
        self.pages
            .iter()
            .enumerate()
            .filter(|(_, p)| p.engine.map(|e| e == self.engine).unwrap_or(true))
            .map(|(i, _)| i)
            .collect()
    }

    pub fn current_page(&self) -> &Page {
        &self.pages[self.page]
    }

    fn page_position(&self) -> usize {
        self.visible_page_indices()
            .iter()
            .position(|&i| i == self.page)
            .unwrap_or(0)
    }

    pub fn next_page(&mut self, delta: i32) {
        self.remember_selection();
        self.step_page(delta);
        self.restore_selection();
    }

    /// Page change without touching the per-page selection memory.
    fn step_page(&mut self, delta: i32) {
        let vis = self.visible_page_indices();
        if vis.is_empty() {
            return;
        }
        let pos = self.page_position() as i32;
        let len = vis.len() as i32;
        let new = ((pos + delta).rem_euclid(len)) as usize;
        self.page = vis[new];
        self.sel = 0;
        self.refresh_page_state();
    }

    pub fn goto_group(&mut self, group: Group) {
        let candidates: Vec<usize> = self
            .visible_page_indices()
            .into_iter()
            .filter(|&i| self.pages[i].group == group)
            .collect();
        if let Some(&first) = candidates.first() {
            self.remember_selection();
            self.page = first;
            self.sel = 0;
            self.refresh_page_state();
            self.restore_selection();
        }
    }

    pub fn goto_page_id(&mut self, id: &str) -> bool {
        let Some(page) = self
            .visible_page_indices()
            .into_iter()
            .find(|&i| self.pages[i].id == id)
        else {
            return false;
        };
        self.remember_selection();
        self.page = page;
        self.sel = 0;
        self.refresh_page_state();
        self.restore_selection();
        true
    }

    pub fn remember_selection(&mut self) {
        if let Some(slot) = self.sel_memory.get_mut(self.page) {
            *slot = self.sel;
        }
    }

    pub fn restore_selection(&mut self) {
        let remembered = self.sel_memory.get(self.page).copied().unwrap_or(0);
        self.sel = remembered.min(self.rows.len().saturating_sub(1));
        if !self.rows.is_empty()
            && !self.rows[self.sel].selectable()
            && let Some(i) = self.first_selectable()
        {
            self.sel = i;
        }
    }

    fn first_selectable(&self) -> Option<usize> {
        self.rows.iter().position(|r| r.selectable())
    }

    fn last_selectable(&self) -> Option<usize> {
        self.rows.iter().rposition(|r| r.selectable())
    }

    fn cond_ok(&self, cond: &Cond) -> bool {
        match cond {
            Cond::Engine(e) => self.engine == *e,
            Cond::Eq(path, want) => {
                let got = self
                    .settings
                    .get(&path.iter().map(String::as_str).collect::<Vec<_>>());
                json_eq_loose(got, want)
            }
        }
    }

    pub fn control_visible(&self, c: &Control) -> bool {
        c.visible.as_ref().map(|c| self.cond_ok(c)).unwrap_or(true)
    }

    pub fn rebuild_rows(&mut self) {
        let kind = self.current_page().body.kind();
        let mut rows: Vec<Row> = Vec::new();
        let controls = match kind {
            BodyKind::Controls => self.control_sections_flat(),
            BodyKind::ClassicBar => self.control_sections_flat(),
            BodyKind::HyprEffects => hypr_controls(),
            BodyKind::Zones => zones_actions(),
            BodyKind::Monitors => monitors_actions(),
            BodyKind::Guide => guide_actions(),
            BodyKind::Palette => palette_actions(),
            _ => Vec::new(),
        };
        self.controls = controls;

        match kind {
            BodyKind::Controls | BodyKind::HyprEffects => {
                for (title, indices) in self.control_sections() {
                    let visible: Vec<usize> = indices
                        .into_iter()
                        .filter(|&i| self.control_visible(&self.controls[i]))
                        .collect();
                    if visible.is_empty() {
                        continue;
                    }
                    rows.push(Row::Section(title));
                    for i in visible {
                        rows.push(Row::Control(i));
                    }
                }
            }
            BodyKind::Keybinds => {
                rows.push(Row::Section("Keybindings".into()));
                for (i, _) in self.keybinds.iter().enumerate() {
                    rows.push(Row::Keybind(i));
                }
            }
            BodyKind::Startup => {
                rows.push(Row::Section("Startup commands".into()));
                for (i, _) in self.startup.iter().enumerate() {
                    rows.push(Row::Startup(i));
                }
            }
            BodyKind::Zones => {
                rows.push(Row::Section("Actions".into()));
                for i in 0..self.controls.len() {
                    rows.push(Row::Control(i));
                }
                let zones = self.zones();
                for (z, _) in zones.iter().enumerate() {
                    rows.push(Row::ZoneHeader(z));
                    for f in 0..ZONE_FIELD_COUNT {
                        rows.push(Row::ZoneField { zone: z, field: f });
                    }
                    let modules = zones[z]
                        .get("modules")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default();
                    for (i, _) in modules.iter().enumerate() {
                        rows.push(Row::ZoneModule { zone: z, idx: i });
                    }
                }
            }
            BodyKind::ClassicBar => {
                for (title, indices) in self.control_sections() {
                    let visible: Vec<usize> = indices
                        .into_iter()
                        .filter(|&i| self.control_visible(&self.controls[i]))
                        .collect();
                    if visible.is_empty() {
                        continue;
                    }
                    rows.push(Row::Section(title));
                    for i in visible {
                        rows.push(Row::Control(i));
                    }
                }
                let labels = ["Left", "Center", "Right", "Available"];
                for (sec, label) in labels.iter().enumerate() {
                    rows.push(Row::Section(format!("Modules — {}", label)));
                    let items = self.classic_items(sec);
                    for (i, _) in items.iter().enumerate() {
                        rows.push(Row::ClassicItem { sec, idx: i });
                    }
                }
            }
            BodyKind::Palette => {
                rows.push(Row::Section("Palettes".into()));
                rows.push(Row::Control(0)); // filter action row (label shows filter)
                let filter = self.palette_filter.to_lowercase();
                let list: Vec<usize> = self
                    .palettes
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| {
                        filter.is_empty()
                            || e.name.to_lowercase().contains(&filter)
                            || e.slug.to_lowercase().contains(&filter)
                    })
                    .map(|(i, _)| i)
                    .collect();
                for i in list {
                    rows.push(Row::PaletteItem(i));
                }
                if let Some(p) = &self.palette {
                    rows.push(Row::Section(format!(
                        "Active palette — {} ({})",
                        p.name, p.slug
                    )));
                    for slot in 0..18 {
                        rows.push(Row::PaletteSlot(slot));
                    }
                }
                rows.push(Row::Section("Actions".into()));
                for i in 1..self.controls.len() {
                    rows.push(Row::Control(i));
                }
            }
            BodyKind::Monitors => {
                rows.push(Row::Section("Actions".into()));
                for i in 0..self.controls.len() {
                    rows.push(Row::Control(i));
                }
                for (m, mon) in self.monitors.iter().enumerate() {
                    let title = if mon.description.is_empty() {
                        mon.name.clone()
                    } else {
                        format!("{} — {}", mon.name, mon.description)
                    };
                    rows.push(Row::Section(title));
                    for f in 0..MONITOR_FIELDS.len() {
                        rows.push(Row::MonitorField { mon: m, field: f });
                    }
                }
            }
            BodyKind::Guide => {
                rows.push(Row::Section("System".into()));
                let info_keys: Vec<String> = self
                    .infos
                    .keys()
                    .filter(|k| k.starts_with("sys."))
                    .cloned()
                    .collect();
                let mut info_keys = info_keys;
                info_keys.sort();
                for k in info_keys {
                    let label = k.trim_start_matches("sys.").to_string();
                    rows.push(Row::Info(label, k));
                }
                rows.push(Row::Section("Actions".into()));
                for i in 0..self.controls.len() {
                    rows.push(Row::Control(i));
                }
            }
        }
        self.rows = rows;
        self.clamp_selection();
    }

    /// Section titles + indices into `self.controls` for the current page.
    fn control_sections(&self) -> Vec<(String, Vec<usize>)> {
        match &self.current_page().body {
            Body::Controls(sections) | Body::ClassicBar(sections) => {
                let mut out = Vec::new();
                let mut i = 0usize;
                for s in sections {
                    let idxs = (i..i + s.controls.len()).collect();
                    i += s.controls.len();
                    out.push((s.title.clone(), idxs));
                }
                out
            }
            Body::HyprEffects => {
                let n = catalog::HYPR_SLIDERS.len();
                vec![
                    ("Window effects".to_string(), (0..n).collect()),
                    ("Actions".to_string(), (n..n + 2).collect()),
                    (
                        "Window borders".to_string(),
                        (n + 2..self.controls.len()).collect(),
                    ),
                ]
            }
            Body::Zones => vec![("Actions".to_string(), (0..self.controls.len()).collect())],
            Body::Monitors => vec![("Actions".to_string(), (0..self.controls.len()).collect())],
            Body::Guide => vec![("Actions".to_string(), (0..self.controls.len()).collect())],
            Body::Palette => vec![
                ("Filter".to_string(), vec![0]),
                ("Actions".to_string(), (1..self.controls.len()).collect()),
            ],
            _ => Vec::new(),
        }
    }

    fn control_sections_flat(&self) -> Vec<Control> {
        match &self.current_page().body {
            Body::Controls(sections) | Body::ClassicBar(sections) => flatten(sections),
            _ => Vec::new(),
        }
    }

    fn clamp_selection(&mut self) {
        if self.rows.is_empty() {
            self.sel = 0;
            return;
        }
        if self.sel >= self.rows.len() {
            self.sel = self.rows.len() - 1;
        }
        // Do not use move_selection here: clamping is a layout fix, not a
        // user movement, and must not touch the per-page selection memory.
        if !self.rows[self.sel].selectable()
            && let Some(i) = self.first_selectable()
        {
            self.sel = i;
        }
    }

    /// Move the selection; at the edges of a page it crosses to the
    /// neighbouring page, so the whole menu is reachable with arrows alone.
    pub fn move_selection(&mut self, delta: i32) {
        if self.rows.is_empty() {
            return;
        }
        let len = self.rows.len() as i32;
        let mut idx = self.sel as i32 + delta;
        while idx >= 0 && idx < len {
            if self.rows[idx as usize].selectable() {
                self.sel = idx as usize;
                self.remember_selection();
                return;
            }
            idx += delta;
        }
        // No selectable row in that direction inside this page: cross pages.
        self.remember_selection();
        let before = self.page;
        self.step_page(delta.signum());
        if self.page != before {
            let target = if delta > 0 {
                self.first_selectable()
            } else {
                self.last_selectable()
            };
            if let Some(i) = target {
                self.sel = i;
            }
            self.remember_selection();
        }
    }

    pub fn selected_row(&self) -> Option<Row> {
        self.rows.get(self.sel).cloned()
    }

    // ───────────────────────── data helpers ─────────────────────────

    fn zones(&self) -> Vec<Value> {
        self.settings
            .get(&["bar", "zones"])
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    }

    fn set_zones(&mut self, zones: Vec<Value>) {
        let _ = self.settings.set(&["bar", "zones"], Value::Array(zones));
    }

    fn classic_items(&self, sec: usize) -> Vec<Value> {
        self.settings
            .get(&["classicbar", "modules", classic::SECTIONS[sec]])
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    }

    fn set_classic_items(&mut self, sec: usize, items: Vec<Value>) {
        let mut modules = self
            .settings
            .get(&["classicbar", "modules"])
            .cloned()
            .unwrap_or_else(|| json!({}));
        if !modules.is_object() {
            modules = json!({});
        }
        classic::normalize_modules(&mut modules);
        modules
            .as_object_mut()
            .unwrap()
            .insert(classic::SECTIONS[sec].to_string(), Value::Array(items));
        classic::normalize_modules(&mut modules);
        let _ = self.settings.set(&["classicbar", "modules"], modules);
    }

    fn classic_modules_root(&self) -> Value {
        self.settings
            .get(&["classicbar", "modules"])
            .cloned()
            .unwrap_or_else(|| json!({}))
    }

    pub fn info_value(&self, key: &str) -> String {
        if let Some(v) = self.infos.get(key) {
            return v.clone();
        }
        if let Some(v) = self.vstate.get(key) {
            return v.clone();
        }
        match key {
            "workspaceCount" => self
                .settings
                .get_f64(&["workspaceCount"])
                .map(|v| format!("{}", v as i64))
                .unwrap_or_else(|| "8".into()),
            "timex.order" => self
                .settings
                .get_str(&["timex", "forecastOrder"])
                .unwrap_or_else(|| "time,icon,temp".into()),
            "gpu.env.gbm" => "nvidia-drm".into(),
            "gpu.env.libva" => "nvidia".into(),
            _ => "—".into(),
        }
    }

    // ───────────────────────── value get/set ─────────────────────────

    pub fn control_value(&self, c: &Control) -> Option<Value> {
        if c.path.first().map(String::as_str) == Some("__hypr__") {
            return c
                .path
                .get(1)
                .and_then(|k| self.hypr.get(k))
                .map(|v| json!(v));
        }
        if c.path.is_empty() {
            return None;
        }
        let path: Vec<&str> = c.path.iter().map(String::as_str).collect();
        self.settings.get(&path).cloned()
    }

    fn write_path(&mut self, path: &[String], value: Value) {
        if path.is_empty() {
            return;
        }
        let path_ref: Vec<&str> = path.iter().map(String::as_str).collect();
        match self.settings.set(&path_ref, value) {
            Ok(true) => self.after_write(path),
            Ok(false) => {}
            Err(e) => self.set_status(format!("write error: {e}")),
        }
    }

    fn after_write(&mut self, path: &[String]) {
        let page = self.current_page().id;
        let key = path.last().map(String::as_str).unwrap_or("");
        let top = path.first().map(String::as_str).unwrap_or("");
        match (page, top, key) {
            ("s_general", "appScale", _) => {
                if let Some(v) = self.settings.get_f64(&["appScale"]) {
                    actions::spawn(&format!("bash {} {}", actions::script("scale-menu.sh"), v));
                }
            }
            ("s_general", "workspaceCount", _) => {
                self.pending.push(Pending {
                    at: Instant::now() + Duration::from_millis(300),
                    kind: PendingKind::WorkspaceReload,
                });
            }
            ("s_timex", _, _) => {
                self.pending.push(Pending {
                    at: Instant::now() + Duration::from_millis(500),
                    kind: PendingKind::TimexRefresh,
                });
            }
            ("d_input", "input", _) => {
                self.pending.push(Pending {
                    at: Instant::now() + Duration::from_millis(600),
                    kind: PendingKind::PersistInput,
                });
            }
            ("d_animations", "animations", _) => {
                self.pending.push(Pending {
                    at: Instant::now() + Duration::from_millis(600),
                    kind: PendingKind::PersistAnimations,
                });
            }
            ("d_palette", "bar", "palette") => {
                palette::sync_theme();
                self.reload_palette();
            }
            _ => {}
        }
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status = msg.into();
        self.last_status_clear = Instant::now();
    }

    // ───────────────────────── generic control ops ─────────────────────────

    pub fn adjust_control(&mut self, idx: usize, delta: i32) {
        let Some(c) = self.controls.get(idx).cloned() else {
            return;
        };
        if !self.control_visible(&c) {
            return;
        }
        let is_hypr = c.path.first().map(String::as_str) == Some("__hypr__");
        match &c.kind {
            Kind::Toggle => {
                let cur = self
                    .control_value(&c)
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let new = !cur;
                if is_hypr {
                    self.set_hypr(&c, if new { 1.0 } else { 0.0 });
                } else {
                    self.write_path(&c.path, Value::Bool(new));
                }
            }
            Kind::Stepper {
                step,
                min,
                max,
                decimals,
                ..
            } => {
                let cur = self
                    .control_value(&c)
                    .and_then(|v| {
                        v.as_f64()
                            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                    })
                    .unwrap_or(0.0);
                let mut new = cur + (*step * delta as f64);
                new = new.clamp(*min, *max);
                let factor = 10f64.powi(*decimals as i32);
                new = (new * factor).round() / factor;
                if is_hypr {
                    self.set_hypr(&c, new);
                } else {
                    let val = if *decimals == 0 && new.fract() == 0.0 {
                        json!(new as i64)
                    } else {
                        json!(new)
                    };
                    self.write_path(&c.path, val);
                }
            }
            Kind::Options(opts) => {
                if opts.is_empty() {
                    return;
                }
                let cur = self
                    .control_value(&c)
                    .map(value_to_string)
                    .unwrap_or_else(|| opts[0].value.clone());
                let pos = opts.iter().position(|o| o.value == cur).unwrap_or(0) as i32;
                let new = (pos + delta).rem_euclid(opts.len() as i32) as usize;
                self.write_path(&c.path, Value::String(opts[new].value.clone()));
            }
            Kind::StateOptions { .. } => {
                self.open_chooser_for_control(idx);
            }
            Kind::Text { .. } | Kind::Action(_) | Kind::Info(_) => {}
        }
    }

    pub fn activate_control(&mut self, idx: usize) {
        let Some(c) = self.controls.get(idx).cloned() else {
            return;
        };
        if !self.control_visible(&c) {
            return;
        }
        match &c.kind {
            Kind::Toggle => self.adjust_control(idx, 1),
            Kind::Stepper { .. } => self.begin_edit_control(idx),
            Kind::Options(opts) => {
                let opts = opts.clone();
                let cur = self
                    .control_value(&c)
                    .map(value_to_string)
                    .unwrap_or_default();
                let pos = opts.iter().position(|o| o.value == cur).unwrap_or(0);
                self.mode = Mode::Chooser {
                    target: ChooserTarget::Control {
                        path: c.path.clone(),
                        state_action: None,
                        opts,
                    },
                    idx: pos,
                };
            }
            Kind::StateOptions {
                options,
                action,
                state_key,
            } => {
                let cur = self.vstate.get(state_key).cloned().unwrap_or_default();
                let pos = options.iter().position(|o| o.value == cur).unwrap_or(0);
                self.mode = Mode::Chooser {
                    target: ChooserTarget::Control {
                        path: Vec::new(),
                        state_action: Some(action.clone()),
                        opts: options.clone(),
                    },
                    idx: pos,
                };
            }
            Kind::Text { .. } => self.begin_edit_control(idx),
            Kind::Action(act) => {
                let act = act.clone();
                self.run_action(act, None);
            }
            Kind::Info(_) => {}
        }
    }

    // ───────────────────────── row dispatch ─────────────────────────

    pub fn adjust_row(&mut self, delta: i32) {
        match self.selected_row() {
            Some(Row::Control(i)) => self.adjust_control(i, delta),
            Some(Row::ZoneField { zone, field }) => self.zone_field(zone, field, delta),
            Some(Row::ClassicItem { sec, idx }) => self.classic_move(sec, idx, delta),
            Some(Row::MonitorField { mon, field }) => self.monitor_adjust(mon, field, delta),
            Some(Row::PaletteSlot(_)) | Some(Row::Keybind(_)) | Some(Row::Startup(_)) => {}
            Some(Row::ZoneModule { .. }) | Some(Row::ZoneHeader(_)) => {}
            Some(Row::PaletteItem(_)) => {}
            Some(Row::Section(_)) | Some(Row::Info(..)) | None => {}
        }
    }

    pub fn activate_row(&mut self) {
        match self.selected_row() {
            Some(Row::Control(i)) => self.activate_control(i),
            Some(Row::Keybind(i)) => {
                self.mode = Mode::Form { kb: i, field: 0 };
            }
            Some(Row::Startup(i)) => {
                let cmd = self
                    .startup
                    .get(i)
                    .and_then(|s| s.get("command"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                self.mode = Mode::Edit {
                    target: EditTarget::Startup(i),
                    cursor: cmd.chars().count(),
                    buf: cmd,
                };
            }
            Some(Row::ZoneField { zone, field }) => {
                if field == 7 {
                    self.zone_delete(zone);
                } else {
                    self.zone_field(zone, field, 1);
                }
            }
            Some(Row::ZoneModule { zone, idx }) => self.zone_toggle_module(zone, idx),
            Some(Row::ClassicItem { sec, idx }) => {
                let _ = (sec, idx);
            }
            Some(Row::PaletteItem(i)) => self.palette_select(i),
            Some(Row::PaletteSlot(slot)) => {
                if let Some(p) = &self.palette {
                    let buf = p.slot(slot).to_string();
                    let slug = p.slug.clone();
                    self.mode = Mode::Edit {
                        target: EditTarget::PaletteSlot { slot, slug },
                        cursor: buf.chars().count(),
                        buf,
                    };
                }
            }
            Some(Row::MonitorField { mon, field }) => self.activate_monitor_field(mon, field),
            Some(Row::ZoneHeader(_)) => {}
            Some(Row::Section(_)) | Some(Row::Info(..)) | None => {}
        }
    }

    fn activate_monitor_field(&mut self, mon: usize, field: usize) {
        match field {
            0 => self.monitor_adjust(mon, field, 1),
            8 | 9 => {}
            _ => {
                let opts = self.monitor_field_options(mon, field);
                if opts.is_empty() {
                    return;
                }
                self.mode = Mode::Chooser {
                    target: ChooserTarget::Monitor { mon, field },
                    idx: 0,
                };
            }
        }
    }

    pub fn begin_keybind_field_edit(&mut self, kb: usize, field: usize) {
        let opts = KEYBIND_OPTIONS
            .get(field)
            .map(|o| {
                o.iter()
                    .map(|(l, v)| Opt {
                        label: (*l).into(),
                        value: (*v).into(),
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if !opts.is_empty() {
            let cur = self
                .keybinds
                .get(kb)
                .and_then(|b| b.get(KEYBIND_FIELDS[field].0))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let idx = opts.iter().position(|o| o.value == cur).unwrap_or(0);
            self.mode = Mode::Chooser {
                target: ChooserTarget::Keybind { kb, field },
                idx,
            };
        } else {
            let buf = self
                .keybinds
                .get(kb)
                .and_then(|b| b.get(KEYBIND_FIELDS[field].0))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            self.mode = Mode::Edit {
                target: EditTarget::Keybind { kb, field },
                cursor: buf.chars().count(),
                buf,
            };
        }
    }

    pub fn form_move(&mut self, delta: i32) {
        if let Mode::Form { field, .. } = &mut self.mode {
            let n = KEYBIND_FIELDS.len() as i32;
            *field = (*field as i32 + delta).rem_euclid(n) as usize;
        }
    }

    pub fn form_activate(&mut self) {
        if let Mode::Form { kb, field } = self.mode {
            self.begin_keybind_field_edit(kb, field);
        }
    }

    // ───────────────────────── special actions used by main ─────────────────────────

    pub fn move_zone_module(&mut self, delta: i32) {
        if let Some(Row::ZoneModule { zone, idx }) = self.selected_row() {
            self.zone_move_module(zone, idx, delta);
        }
    }

    pub fn classic_action_join(&mut self) {
        if let Some(Row::ClassicItem { sec, idx }) = self.selected_row() {
            self.classic_join(sec, idx);
        }
    }

    pub fn classic_action_ungroup(&mut self) {
        if let Some(Row::ClassicItem { sec, idx }) = self.selected_row() {
            self.classic_ungroup(sec, idx);
        }
    }

    pub fn classic_action_move_section(&mut self) {
        if let Some(Row::ClassicItem { sec, idx }) = self.selected_row() {
            self.classic_move_section(sec, idx);
        }
    }

    pub fn selected_is_zone_module(&self) -> bool {
        matches!(self.selected_row(), Some(Row::ZoneModule { .. }))
    }

    pub fn monitor_field_options_pub(&self, mon: usize, field: usize) -> Vec<Opt> {
        self.monitor_field_options(mon, field)
    }

    pub fn chooser_move(&mut self, delta: i32) {
        let len = match &self.mode {
            Mode::Chooser { target, .. } => match target {
                ChooserTarget::Control { opts, .. } => opts.len(),
                ChooserTarget::Keybind { field, .. } => {
                    KEYBIND_OPTIONS.get(*field).map(|o| o.len()).unwrap_or(0)
                }
                ChooserTarget::Monitor { mon, field } => {
                    self.monitor_field_options(*mon, *field).len()
                }
            },
            _ => return,
        };
        if len == 0 {
            return;
        }
        if let Mode::Chooser { idx, .. } = &mut self.mode {
            *idx = (*idx as i32 + delta).rem_euclid(len as i32) as usize;
        }
    }

    pub fn hard_reload(&mut self) {
        if let Err(e) = self.settings.reload() {
            self.set_status(format!("reload error: {e}"));
            return;
        }
        self.engine = self
            .settings
            .get_str(&["barEngine"])
            .unwrap_or_else(|| "bar".into());
        self.keybinds = self
            .settings
            .get(&["keybinds"])
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        self.startup = self
            .settings
            .get(&["startup"])
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        self.refresh_page_state();
        self.set_status("Reloaded settings.json");
    }

    pub fn add_selected_list(&mut self) {
        match self.current_page().body.kind() {
            BodyKind::Keybinds => self.keybind_add(),
            BodyKind::Startup => self.startup_add(),
            _ => {}
        }
    }

    pub fn delete_selected(&mut self) {
        match self.selected_row() {
            Some(Row::Keybind(i)) => self.keybind_delete(i),
            Some(Row::Startup(i)) => self.startup_delete(i),
            Some(Row::PaletteItem(i)) => {
                let slug = self
                    .palettes
                    .get(i)
                    .map(|e| e.slug.clone())
                    .unwrap_or_default();
                if slug == "x" {
                    self.set_status("'x' cannot be deleted");
                } else if !slug.is_empty() {
                    self.palette_delete_slug = Some(slug.clone());
                    self.begin_confirm(
                        Action::PaletteDelete,
                        &format!("Delete palette '{}'?", slug),
                    );
                }
            }
            _ => {}
        }
    }

    pub fn save_selected_list(&mut self) {
        match self.current_page().body.kind() {
            BodyKind::Keybinds => self.save_keybinds(),
            BodyKind::Startup => self.save_startup(),
            _ => {}
        }
    }

    fn open_chooser_for_control(&mut self, idx: usize) {
        if let Some(c) = self.controls.get(idx).cloned()
            && let Kind::StateOptions {
                options,
                action,
                state_key,
            } = &c.kind
        {
            let cur = self.vstate.get(state_key).cloned().unwrap_or_default();
            let pos = options.iter().position(|o| o.value == cur).unwrap_or(0);
            self.mode = Mode::Chooser {
                target: ChooserTarget::Control {
                    path: Vec::new(),
                    state_action: Some(action.clone()),
                    opts: options.clone(),
                },
                idx: pos,
            };
        }
    }

    fn begin_edit_control(&mut self, idx: usize) {
        let Some(c) = self.controls.get(idx).cloned() else {
            return;
        };
        let current = self
            .control_value(&c)
            .map(value_to_string)
            .unwrap_or_default();
        let numeric = matches!(c.kind, Kind::Stepper { .. });
        self.mode = Mode::Edit {
            target: EditTarget::Control {
                path: c.path.clone(),
                numeric,
                label: c.label.clone(),
            },
            cursor: current.chars().count(),
            buf: current,
        };
    }

    // ───────────────────────── hypr effects ─────────────────────────

    pub fn load_hypr(&mut self) {
        // Defaults first so the UI is usable even if hyprctl is unavailable.
        for (key, _, default, _, _, _, _) in catalog::HYPR_SLIDERS {
            self.hypr.insert((*key).to_string(), *default);
        }
        // `hypr-effects.sh read` prints one `key=value` line per option.
        let text = actions::capture(&format!(
            "bash {} read",
            actions::quickshell_dir() + "/core/scripts/hypr-effects.sh"
        ));
        for line in text.lines() {
            if let Some((k, v)) = line.split_once('=')
                && let Ok(n) = v.trim().parse::<f64>()
            {
                self.hypr.insert(k.trim().to_string(), n);
            }
        }
    }

    fn set_hypr(&mut self, c: &Control, value: f64) {
        let Some(key) = c.path.get(1).cloned() else {
            return;
        };
        self.hypr.insert(key.clone(), value);
        self.hypr_dirty.insert(key);
        self.pending.push(Pending {
            at: Instant::now() + Duration::from_millis(200),
            kind: PendingKind::HyprPreview,
        });
    }

    pub fn flush_hypr(&mut self) {
        if self.hypr.is_empty() {
            return;
        }
        let dirty: Vec<String> = HYPR_ORDER
            .iter()
            .filter(|k| self.hypr_dirty.contains(**k))
            .map(|k| (*k).to_string())
            .collect();
        if dirty.is_empty() {
            return;
        }
        let args: Vec<String> = dirty
            .iter()
            .map(|k| {
                format!(
                    "{}={}",
                    k,
                    fmt_num(self.hypr.get(k).copied().unwrap_or(0.0))
                )
            })
            .collect();
        let script = actions::quickshell_dir() + "/core/scripts/hypr-effects.sh";
        actions::spawn(&format!("bash {} preview {}", script, args.join(" ")));
    }

    pub fn persist_hypr(&mut self) {
        if self.hypr_dirty.is_empty() {
            return;
        }
        let args: Vec<String> = HYPR_ORDER
            .iter()
            .filter(|k| self.hypr_dirty.contains(**k))
            .map(|k| {
                format!(
                    "{}={}",
                    k,
                    fmt_num(self.hypr.get(*k).copied().unwrap_or(0.0))
                )
            })
            .collect();
        let script = actions::quickshell_dir() + "/core/scripts/hypr-effects.sh";
        actions::spawn(&format!("bash {} apply {}", script, args.join(" ")));
        self.hypr_dirty.clear();
    }

    // ───────────────────────── actions ─────────────────────────

    pub fn run_action(&mut self, act: Action, arg: Option<String>) {
        match act {
            Action::Shell(cmd) => actions::spawn(&cmd),
            Action::Prompt { prompt, prefix } => {
                self.mode = Mode::Edit {
                    target: EditTarget::Prompt { prefix, prompt },
                    buf: String::new(),
                    cursor: 0,
                };
            }
            Action::SetEngine => {
                let value = arg
                    .or_else(|| self.vstate.get("engine").cloned())
                    .unwrap_or_else(|| "bar".into());
                self.set_engine(&value);
            }
            Action::MirrorBar => self.mirror_bar(),
            Action::ClassicApplyDefaults => self.classic_defaults(),
            Action::ZoneAdd => self.zone_add(),
            Action::ZonesCenterAll => self.zones_center_all(),
            Action::ZonesDefault => {
                self.mode = Mode::Confirm {
                    action: Action::ZonesDefault,
                    msg: "Reset bar to factory defaults?".into(),
                };
            }
            Action::ClassicMirror => self.mirror_bar(),
            Action::ClassicDefaults => self.classic_defaults(),
            Action::PaletteReset => {
                self.mode = Mode::Confirm {
                    action: Action::PaletteReset,
                    msg: "Restore the active palette from its backup?".into(),
                };
            }
            Action::PaletteCreate => {
                self.mode = Mode::Edit {
                    target: EditTarget::Prompt {
                        prefix: "__palette_create__".into(),
                        prompt: "New palette name".into(),
                    },
                    buf: String::new(),
                    cursor: 0,
                };
            }
            Action::PaletteDelete => {
                let slug = self
                    .palette
                    .as_ref()
                    .map(|p| p.slug.clone())
                    .unwrap_or_default();
                if slug == "x" {
                    self.set_status("'x' is the fallback palette and cannot be deleted");
                } else if !slug.is_empty() {
                    self.mode = Mode::Confirm {
                        action: Action::PaletteDelete,
                        msg: format!("Delete palette '{}'?", slug),
                    };
                }
            }
            Action::RotateForecastOrder => {
                let cur = self
                    .settings
                    .get_str(&["timex", "forecastOrder"])
                    .unwrap_or_else(|| "time,icon,temp".into());
                let parts: Vec<&str> = cur.split(',').collect();
                let rotated = if parts.len() == 3 {
                    format!("{},{},{}", parts[1], parts[2], parts[0])
                } else {
                    "time,icon,temp".into()
                };
                self.write_path(&p("timex.forecastOrder"), Value::String(rotated));
            }
            Action::HyprReset => {
                for (key, _, default, _, _, _, _) in catalog::HYPR_SLIDERS {
                    self.hypr.insert((*key).to_string(), *default);
                    self.hypr_dirty.insert((*key).to_string());
                }
                self.flush_hypr();
                self.set_status("Hyprland effects reset to defaults");
            }
            Action::HyprRefresh => {
                self.load_hypr();
                self.set_status("Reloaded live values from hyprctl");
            }
            Action::GpuMode => {
                let value = arg.unwrap_or_default();
                actions::spawn(&format!(
                    "bash {} {}",
                    actions::script("gpu-mode.sh"),
                    value
                ));
                self.vstate.insert("gpu".into(), value);
            }
            Action::GpuRefresh => self.refresh_gpu(),
            Action::IdleMode => {
                let value = arg.unwrap_or_default();
                actions::spawn(&format!(
                    "bash {} {}",
                    actions::script("idle-mode.sh"),
                    value
                ));
                self.vstate.insert("idle".into(), value);
            }
            Action::LockNow => actions::spawn(&format!("bash {}", actions::script("lock.sh"))),
            Action::Suspend => actions::spawn("systemctl suspend"),
            Action::DndToggle => {
                let on = self.dnd_on();
                let dir = dnd_dir();
                let val = if on { "0" } else { "1" };
                actions::spawn(&format!(
                    "mkdir -p '{}' && echo '{}' > '{}/state'",
                    dir, val, dir
                ));
                self.infos
                    .insert("dnd".into(), if on { "off".into() } else { "on".into() });
            }
            Action::OpenWidget(id) => {
                actions::spawn(&format!(
                    "bash {} toggle {}",
                    actions::script("qs_manager.sh"),
                    id
                ));
            }
            Action::CopyDebug => {
                actions::spawn(&format!(
                    "bash '{}' | wl-copy",
                    actions::quickshell_dir() + "/ui/bar/editor/sysinfo.sh"
                ));
                self.set_status("Debug info copied to clipboard");
            }
            Action::RunDoctor => {
                let out = actions::capture(
                    "D=\"$HOME/.local/share/equisdots/dots/dots\"; if [ -x \"$D\" ]; then bash \"$D\" doctor 2>&1; else echo \"dots is not installed (run: dots system && dots install)\"; fi",
                );
                let first: String = out.chars().take(400).collect();
                self.infos
                    .insert("guide.doctor".into(), first.replace('\n', " | "));
                self.set_status("Doctor finished");
            }
            Action::OpenUrl(url) => actions::spawn(&format!("xdg-open '{}'", url)),
            Action::MonitorsApply => self.monitors_apply(),
            Action::MonitorsReset => {
                monitors::reset();
                self.monitors = Vec::new();
                self.set_status("Monitors reset to auto");
                self.refresh_page_state();
            }
            Action::PaletteFilterEdit => {
                self.mode = Mode::Edit {
                    target: EditTarget::PaletteFilter,
                    cursor: self.palette_filter.chars().count(),
                    buf: self.palette_filter.clone(),
                };
            }
            Action::RefreshPage => self.refresh_page_state(),
        }
    }

    fn apply_chooser_value(&mut self, target: ChooserTarget, idx: usize) {
        match target {
            ChooserTarget::Control {
                path,
                state_action,
                opts,
                ..
            } => {
                let Some(opt) = opts.get(idx) else { return };
                if let Some(act) = state_action {
                    self.run_action(act, Some(opt.value.clone()));
                } else {
                    self.write_path(&path, Value::String(opt.value.clone()));
                }
            }
            ChooserTarget::Keybind { kb, field } => {
                self.keybinds[kb][KEYBIND_FIELDS[field].0] = Value::String(
                    KEYBIND_OPTIONS[field]
                        .get(idx)
                        .map(|(_, v)| v.to_string())
                        .unwrap_or_default(),
                );
                self.keybinds_dirty = true;
            }
            ChooserTarget::Monitor { mon, field } => {
                self.apply_monitor_option(mon, field, idx);
            }
        }
    }

    // ───────────────────────── keybinds / startup ─────────────────────────

    pub fn keybind_add(&mut self) {
        self.keybinds.push(json!({
            "type": "bind", "mods": "", "key": "", "dispatcher": "exec", "command": ""
        }));
        self.keybinds_dirty = true;
        self.rebuild_rows();
        self.select_last_of(|r| matches!(r, Row::Keybind(_)));
    }

    pub fn keybind_delete(&mut self, idx: usize) {
        if idx < self.keybinds.len() {
            self.keybinds.remove(idx);
            self.keybinds_dirty = true;
            self.rebuild_rows();
        }
    }

    pub fn startup_add(&mut self) {
        self.startup.push(json!({"command": ""}));
        self.startup_dirty = true;
        self.rebuild_rows();
        self.select_last_of(|r| matches!(r, Row::Startup(_)));
    }

    pub fn startup_delete(&mut self, idx: usize) {
        if idx < self.startup.len() {
            self.startup.remove(idx);
            self.startup_dirty = true;
            self.rebuild_rows();
        }
    }

    pub fn save_keybinds(&mut self) {
        let mut rows: Vec<Value> = Vec::new();
        for b in &self.keybinds {
            let key = b
                .get("key")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_string();
            let command = b
                .get("command")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_string();
            if key.is_empty() && command.is_empty() {
                continue;
            }
            rows.push(json!({
                "type": b.get("type").and_then(Value::as_str).unwrap_or("bind"),
                "mods": b.get("mods").and_then(Value::as_str).unwrap_or(""),
                "key": key,
                "dispatcher": b.get("dispatcher").and_then(Value::as_str).unwrap_or("exec"),
                "command": command,
            }));
        }
        // duplicate check
        let mut seen: HashSet<String> = HashSet::new();
        for b in &rows {
            let combo = format!(
                "{} {}",
                b.get("mods")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .replace('&', " ")
                    .trim(),
                b.get("key").and_then(Value::as_str).unwrap_or("")
            );
            if !seen.insert(combo.clone()) {
                actions::notify("Keybind Error", &format!("Duplicate key combo: {}", combo));
                self.set_status(format!("Duplicate key combo: {}", combo));
                return;
            }
        }
        let _ = self.settings.set(&["keybinds"], Value::Array(rows.clone()));
        self.keybinds = rows
            .iter()
            .map(|b| {
                json!({
                    "type": b["type"].clone(),
                    "mods": b["mods"].clone(),
                    "key": b["key"].clone(),
                    "dispatcher": b["dispatcher"].clone(),
                    "command": b["command"].clone(),
                })
            })
            .collect();
        self.keybinds_dirty = false;

        let mut lines = vec![
            "-- Auto-generated by the QuickShell settings panel.".to_string(),
            "-- Bound to SUPER+SHIFT+S -> Keybindings. Do not edit manually.".to_string(),
            String::new(),
        ];
        for b in &rows {
            let key = b.get("key").and_then(Value::as_str).unwrap_or("");
            let dispatcher = b.get("dispatcher").and_then(Value::as_str).unwrap_or("");
            if key.is_empty() || dispatcher.is_empty() {
                continue;
            }
            let btype = b.get("type").and_then(Value::as_str).unwrap_or("bind");
            let mut flags: Vec<&str> = Vec::new();
            match btype {
                "bindl" => flags.push("locked = true"),
                "bindel" => {
                    flags.push("locked = true");
                    flags.push("repeating = true");
                }
                "bindm" => flags.push("mouse = true"),
                _ => {}
            }
            let mods = b
                .get("mods")
                .and_then(Value::as_str)
                .unwrap_or("")
                .replace('&', " ");
            let mods = mods.trim();
            let keys = if mods.is_empty() {
                key.to_string()
            } else {
                format!("{} + {}", mods, key)
            };
            let command = b.get("command").and_then(Value::as_str).unwrap_or("");
            match lua_dispatcher(dispatcher, command) {
                Some(dsp) => {
                    if flags.is_empty() {
                        lines.push(format!("hl.bind(\"{}\", {})", keys, dsp));
                    } else {
                        lines.push(format!(
                            "hl.bind(\"{}\", {}, {{ {} }})",
                            keys,
                            dsp,
                            flags.join(", ")
                        ));
                    }
                }
                None => lines.push(format!(
                    "-- [skipped] {} {} (no Lua equivalent)",
                    dispatcher, command
                )),
            }
        }
        lines.push(String::new());
        let lua = lines.join("\n");
        let quoted = lua.replace('\'', "'\\''");
        actions::spawn(&format!(
            "mkdir -p ~/.config/hypr/config && cat > ~/.config/hypr/config/user-keybinds.lua << 'LUAEOF'\n{}\nLUAEOF\nhyprctl reload",
            lua
        ));
        let _ = quoted;
        actions::notify("QuickShell", "Keybinds Saved Successfully!");
        self.set_status("Keybinds saved and reloaded");
        self.rebuild_rows();
    }

    pub fn save_startup(&mut self) {
        let rows: Vec<Value> = self
            .startup
            .iter()
            .filter_map(|s| {
                let cmd = s
                    .get("command")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim();
                if cmd.is_empty() {
                    None
                } else {
                    Some(json!({"command": cmd}))
                }
            })
            .collect();
        let _ = self.settings.set(&["startup"], Value::Array(rows.clone()));
        self.startup = rows;
        self.startup_dirty = false;
        let mut lines = vec![
            "-- Auto-generated by the QuickShell settings panel.".to_string(),
            "-- Extra startup commands run once at session start.".to_string(),
            String::new(),
            "hl.on(\"hyprland.start\", function()".to_string(),
        ];
        for s in &self.startup {
            if let Some(cmd) = s.get("command").and_then(Value::as_str) {
                lines.push(format!(
                    "    hl.exec_cmd({})",
                    serde_json::to_string(cmd).unwrap()
                ));
            }
        }
        lines.push("end)".to_string());
        lines.push(String::new());
        let lua = lines.join("\n");
        actions::spawn(&format!(
            "mkdir -p ~/.config/hypr/config && cat > ~/.config/hypr/config/user-startup.lua << 'LUAEOF'\n{}\nLUAEOF\nhyprctl reload",
            lua
        ));
        actions::notify("QuickShell", "Startup entries saved!");
        self.set_status("Startup saved and reloaded");
        self.rebuild_rows();
    }

    // ───────────────────────── engine / bar / classic ─────────────────────────

    fn set_engine(&mut self, engine: &str) {
        if engine == self.engine {
            return;
        }
        if engine == "classic" {
            let has = self
                .settings
                .get(&["classicbar"])
                .map(Value::is_object)
                .unwrap_or(false);
            if !has {
                let bar_position = self
                    .settings
                    .get_str(&["bar", "position"])
                    .unwrap_or_else(|| "top".into());
                let mut cfg = classic::defaults();
                cfg.insert("position".into(), Value::String(bar_position));
                let bar = self
                    .settings
                    .get(&["bar"])
                    .and_then(Value::as_object)
                    .cloned()
                    .unwrap_or_default();
                cfg.insert("modules".into(), classic::bar_to_classic(&bar));
                let _ = self.settings.set(&["classicbar"], Value::Object(cfg));
            }
        }
        let _ = self
            .settings
            .set(&["barEngine"], Value::String(engine.into()));
        self.engine = engine.to_string();
        self.vstate.insert("engine".into(), engine.to_string());
        // If current page is engine-gated and no longer visible, fall back.
        if let Some(gate) = self.current_page().engine
            && gate != self.engine
        {
            self.page = self
                .pages
                .iter()
                .position(|p| p.id == "d_engine")
                .unwrap_or(0);
        }
        self.rebuild_rows();
        self.set_status(format!("Engine: {}", engine));
    }

    fn mirror_bar(&mut self) {
        let bar = self
            .settings
            .get(&["bar"])
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let modules = classic::bar_to_classic(&bar);
        let _ = self.settings.set(&["classicbar", "modules"], modules);
        self.set_status("Mirrored bar layout into classicbar");
        self.refresh_page_state();
    }

    fn classic_defaults(&mut self) {
        let position = self
            .settings
            .get_str(&["classicbar", "position"])
            .unwrap_or_else(|| "top".into());
        let mut cfg = classic::defaults();
        cfg.insert("position".into(), Value::String(position));
        let _ = self.settings.set(&["classicbar"], Value::Object(cfg));
        self.set_status("ClassicBar reset to defaults");
        self.refresh_page_state();
    }

    // ───────────────────────── zones ─────────────────────────

    fn zone_add(&mut self) {
        let mut zones = self.zones();
        let id = format!(
            "zone{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() % 100000)
                .unwrap_or(0)
        );
        zones.push(json!({
            "id": id, "align": "start", "unify": false,
            "zoneBg": "", "zoneBgSolid": false,
            "borderWidth": 0, "borderColor": "surface1",
            "modules": []
        }));
        self.set_zones(zones);
        self.rebuild_rows();
        self.set_status("Zone added");
    }

    fn zones_center_all(&mut self) {
        let zones = self.zones();
        let mut enabled_ids: Vec<String> = Vec::new();
        for z in &zones {
            if let Some(mods) = z.get("modules").and_then(Value::as_array) {
                for m in mods {
                    if m.get("enabled").and_then(Value::as_bool).unwrap_or(false)
                        && let Some(id) = m.get("id").and_then(Value::as_str)
                    {
                        enabled_ids.push(id.to_string());
                    }
                }
            }
        }
        let order: Vec<String> = classic::CATALOG_IDS
            .iter()
            .filter(|id| enabled_ids.iter().any(|e| e == *id))
            .map(|s| s.to_string())
            .collect();
        let mut new_zones: Vec<Value> = zones
            .iter()
            .map(|z| {
                let mut z = z.clone();
                if let Some(mods) = z.get_mut("modules").and_then(Value::as_array_mut) {
                    mods.retain(|m| {
                        !(m.get("enabled").and_then(Value::as_bool).unwrap_or(false)
                            && m.get("id")
                                .and_then(Value::as_str)
                                .map(|id| order.iter().any(|o| o == id))
                                .unwrap_or(false))
                    });
                }
                z
            })
            .collect();
        let center_idx = new_zones
            .iter()
            .position(|z| z.get("align").and_then(Value::as_str) == Some("center"));
        let center_idx = match center_idx {
            Some(i) => i,
            None => {
                let id = format!("center{}", new_zones.len() + 1);
                new_zones.push(json!({
                    "id": id, "align": "center", "unify": false,
                    "zoneBg": "", "zoneBgSolid": false,
                    "borderWidth": 0, "borderColor": "surface1",
                    "modules": []
                }));
                new_zones.len() - 1
            }
        };
        let block: Vec<Value> = order
            .iter()
            .rev()
            .map(|id| json!({"id": id, "enabled": true}))
            .collect();
        if let Some(mods) = new_zones[center_idx]
            .get_mut("modules")
            .and_then(Value::as_array_mut)
        {
            for m in block {
                mods.insert(0, m);
            }
        }
        self.set_zones(new_zones);
        self.rebuild_rows();
        self.set_status("Enabled modules centered");
    }

    fn zones_default(&mut self) {
        let default = json!({
            "position": "top", "palette": "x", "thickness": 48, "edgeGap": 8,
            "roundness": 1.0, "pillBg": true, "pillSolid": false, "barBg": false,
            "stylePreset": "modular", "barOpacity": 0.85, "dragModules": true,
            "borderWidth": 0, "borderColor": "surface1", "borderFollowPalette": true,
            "borderActive": "", "borderInactive": "", "borderActive2": "", "borderInactive2": "",
            "borderGradientActive": false, "borderGradientInactive": false,
            "borderAngleActive": 45, "borderAngleInactive": 45,
            "font": "Hack Nerd Font", "timeFormat": "HH:mm:ss", "dateFormat": "dddd, MMMM dd",
            "modules": {},
            "workspacesMarker": "number", "workspacesMarkerText": "",
            "zones": [
                {"id": "start", "align": "start", "unify": false, "zoneBg": "", "zoneBgSolid": false,
                 "borderWidth": 0, "borderColor": "surface1",
                 "modules": [{"id": "help", "enabled": true}, {"id": "search", "enabled": true},
                             {"id": "settings", "enabled": true}, {"id": "update", "enabled": true},
                             {"id": "time", "enabled": true}, {"id": "date", "enabled": true},
                             {"id": "media", "enabled": true}]},
                {"id": "center", "align": "center", "unify": false, "zoneBg": "", "zoneBgSolid": false,
                 "borderWidth": 0, "borderColor": "surface1",
                 "modules": [{"id": "workspaces", "enabled": true}]},
                {"id": "end", "align": "end", "unify": false, "zoneBg": "", "zoneBgSolid": false,
                 "borderWidth": 0, "borderColor": "surface1",
                 "modules": [{"id": "tray", "enabled": true}, {"id": "keyboard", "enabled": true},
                             {"id": "wifi", "enabled": true}, {"id": "bluetooth", "enabled": true},
                             {"id": "sysmon", "enabled": true}, {"id": "volume", "enabled": true},
                             {"id": "battery", "enabled": true}, {"id": "recording", "enabled": true},
                             {"id": "weather", "enabled": false}, {"id": "focus", "enabled": false}]}
            ]
        });
        if let (Some(dst), Value::Object(src)) = (self.settings.data.get_mut("bar"), default)
            && let Some(dst) = dst.as_object_mut()
        {
            for (k, v) in src {
                dst.insert(k, v);
            }
        }
        let _ = self.settings.write();
        self.set_status("Bar reset to defaults");
        self.refresh_page_state();
    }

    fn zone_field(&mut self, zone: usize, field: usize, delta: i32) {
        let mut zones = self.zones();
        let Some(z) = zones.get_mut(zone) else { return };
        const ALIGNS: [&str; 3] = ["start", "center", "end"];
        const ROLES: [&str; 9] = [
            "surface1", "surface0", "text", "red", "blue", "green", "yellow", "mauve", "teal",
        ];
        let get_str =
            |z: &Value, k: &str| z.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        match field {
            0 => {
                let cur = get_str(z, "align");
                let pos = ALIGNS.iter().position(|a| *a == cur).unwrap_or(0) as i32;
                let new = ALIGNS[((pos + delta).rem_euclid(3)) as usize];
                z["align"] = json!(new);
            }
            1 => {
                let cur = z.get("unify").and_then(Value::as_bool).unwrap_or(false);
                z["unify"] = json!(!cur);
            }
            2 => {
                let cur = get_str(z, "zoneBg");
                z["zoneBg"] = json!(if cur.is_empty() { "surface0" } else { "" });
            }
            3 => {
                let cur = get_str(z, "zoneBg");
                let pos = ROLES.iter().position(|r| *r == cur).unwrap_or(0) as i32;
                let new = ROLES[((pos + delta).rem_euclid(9)) as usize];
                z["zoneBg"] = json!(new);
            }
            4 => {
                let cur = z
                    .get("zoneBgSolid")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                z["zoneBgSolid"] = json!(!cur);
            }
            5 => {
                let cur = z.get("borderWidth").and_then(Value::as_f64).unwrap_or(0.0);
                let new = (cur + delta as f64).clamp(0.0, 8.0);
                z["borderWidth"] = json!(new as i64);
            }
            6 => {
                let cur = get_str(z, "borderColor");
                let pos = ROLES.iter().position(|r| *r == cur).unwrap_or(0) as i32;
                let new = ROLES[((pos + delta).rem_euclid(9)) as usize];
                z["borderColor"] = json!(new);
            }
            _ => return,
        }
        self.set_zones(zones);
        self.rebuild_rows();
    }

    fn zone_delete(&mut self, zone: usize) {
        let mut zones = self.zones();
        if zones.len() <= 1 {
            self.set_status("At least one zone is required");
            return;
        }
        if zone >= zones.len() {
            return;
        }
        let removed = zones.remove(zone);
        let orphans: Vec<Value> = removed
            .get("modules")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|m| m.get("enabled").and_then(Value::as_bool).unwrap_or(false))
            .collect();
        if let Some(first) = zones
            .first_mut()
            .and_then(|z| z.get_mut("modules"))
            .and_then(Value::as_array_mut)
        {
            for m in orphans {
                first.push(m);
            }
        }
        self.set_zones(zones);
        self.rebuild_rows();
        self.set_status("Zone removed");
    }

    fn zone_toggle_module(&mut self, zone: usize, idx: usize) {
        let mut zones = self.zones();
        if let Some(m) = zones
            .get_mut(zone)
            .and_then(|z| z.get_mut("modules"))
            .and_then(Value::as_array_mut)
            .and_then(|a| a.get_mut(idx))
        {
            let cur = m.get("enabled").and_then(Value::as_bool).unwrap_or(false);
            m["enabled"] = json!(!cur);
        }
        self.set_zones(zones);
        self.rebuild_rows();
    }

    /// Port of `moduleMove`: swap with the neighbor inside the zone; at the
    /// edges hop to the neighboring zone.
    fn zone_move_module(&mut self, zone: usize, idx: usize, delta: i32) {
        let mut zones = self.zones();
        let module = match zones
            .get(zone)
            .and_then(|z| z.get("modules"))
            .and_then(Value::as_array)
            .and_then(|a| a.get(idx))
        {
            Some(m) => m.clone(),
            None => return,
        };
        let zone_count = zones.len();
        let list_len = zones[zone]
            .get("modules")
            .and_then(Value::as_array)
            .map(|a| a.len())
            .unwrap_or(0);
        if list_len == 0 {
            return;
        }
        let new_idx = idx as i32 + delta;
        if new_idx >= 0 && (new_idx as usize) < list_len {
            let list = zones[zone]
                .get_mut("modules")
                .unwrap()
                .as_array_mut()
                .unwrap();
            list.swap(idx, new_idx as usize);
        } else if zone_count > 1 {
            zones[zone]
                .get_mut("modules")
                .unwrap()
                .as_array_mut()
                .unwrap()
                .remove(idx);
            let target = if delta > 0 {
                (zone + 1) % zone_count
            } else {
                (zone + zone_count - 1) % zone_count
            };
            let insert_at = if delta > 0 { 0 } else { usize::MAX };
            let list = zones[target]
                .get_mut("modules")
                .unwrap()
                .as_array_mut()
                .unwrap();
            let pos = if insert_at == usize::MAX {
                list.len()
            } else {
                insert_at
            };
            list.insert(pos, module);
        }
        self.set_zones(zones);
        self.rebuild_rows();
    }

    fn classic_move(&mut self, sec: usize, idx: usize, delta: i32) {
        let items = self.classic_items(sec);
        if items.is_empty() {
            return;
        }
        let new_idx = idx as i32 + delta;
        if new_idx < 0 || new_idx as usize >= items.len() {
            return;
        }
        let mut items = items;
        items.swap(idx, new_idx as usize);
        self.set_classic_items(sec, items);
        self.rebuild_rows();
    }

    /// 'm': move item to the next classic section (left -> center -> right -> available).
    fn classic_move_section(&mut self, sec: usize, idx: usize) {
        let items = self.classic_items(sec);
        if idx >= items.len() {
            return;
        }
        let target = (sec + 1) % classic::SECTIONS.len();
        let mut src = items.clone();
        let item = src.remove(idx);
        let mut dst = self.classic_items(target);
        dst.push(item);
        // write both (normalize once at the end)
        self.set_classic_items_raw(target, dst);
        self.set_classic_items_raw(sec, src);
        self.set_status(format!("Moved to {}", classic::SECTIONS[target]));
        self.rebuild_rows();
    }

    fn set_classic_items_raw(&mut self, sec: usize, items: Vec<Value>) {
        let mut modules = self.classic_modules_root();
        if !modules.is_object() {
            modules = json!({});
        }
        modules
            .as_object_mut()
            .unwrap()
            .insert(classic::SECTIONS[sec].to_string(), Value::Array(items));
        let _ = self.settings.set(&["classicbar", "modules"], modules);
    }

    /// 'g': join selected item with the next one into a group.
    fn classic_join(&mut self, sec: usize, idx: usize) {
        let items = self.classic_items(sec);
        if idx + 1 >= items.len() {
            self.set_status("Nothing to join with");
            return;
        }
        let members_of = |v: &Value| -> Vec<String> {
            match v {
                Value::String(s) => vec![s.clone()],
                Value::Array(a) => a
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect(),
                _ => Vec::new(),
            }
        };
        let mut members = members_of(&items[idx]);
        members.extend(members_of(&items[idx + 1]));
        if members.len() < 2 {
            self.set_status("Not enough modules to group");
            return;
        }
        let mut items = items;
        items.splice(
            idx..=idx + 1,
            [Value::Array(
                members.into_iter().map(Value::String).collect(),
            )],
        );
        self.set_classic_items(sec, items);
        self.rebuild_rows();
    }

    /// 'u': ungroup the selected group into loose items.
    fn classic_ungroup(&mut self, sec: usize, idx: usize) {
        let items = self.classic_items(sec);
        let Some(group) = items.get(idx).and_then(classic::is_group) else {
            self.set_status("Selected item is not a group");
            return;
        };
        let mut items = items;
        items.splice(idx..=idx, group.into_iter().map(Value::String));
        self.set_classic_items(sec, items);
        self.rebuild_rows();
    }

    // ───────────────────────── palette ─────────────────────────

    pub fn reload_palette(&mut self) {
        self.palettes = palette::load_index();
        let slug = self
            .settings
            .get_str(&["bar", "palette"])
            .unwrap_or_else(|| "x".into());
        if let Some(entry) = self.palettes.iter().find(|e| e.slug == slug) {
            self.palette = palette::load_palette(entry).ok();
        } else {
            self.palette = None;
        }
    }

    fn palette_select(&mut self, idx: usize) {
        let Some(entry) = self.palettes.get(idx).cloned() else {
            return;
        };
        self.write_path(&p("bar.palette"), Value::String(entry.slug.clone()));
        self.set_status(format!("Palette: {}", entry.name));
    }

    fn palette_prompt_create(&mut self, name: &str) {
        // Build draft from the active palette's first 8 slots.
        let Some(active) = self.palette.clone() else {
            self.set_status("No active palette loaded");
            return;
        };
        let c8: [String; 8] = [
            active.base16[0].clone(),
            active.base16[1].clone(),
            active.base16[2].clone(),
            active.base16[3].clone(),
            active.base16[4].clone(),
            active.base16[5].clone(),
            active.base16[6].clone(),
            active.base16[7].clone(),
        ];
        match palette::create(name, &c8, &active) {
            Ok(slug) => {
                self.reload_palette();
                self.write_path(&p("bar.palette"), Value::String(slug.clone()));
                self.set_status(format!("Created palette '{}'", slug));
            }
            Err(e) => self.set_status(format!("Create failed: {}", e)),
        }
    }

    fn palette_delete(&mut self) {
        let Some(slug) = self.palette.as_ref().map(|p| p.slug.clone()) else {
            return;
        };
        match palette::delete(&slug) {
            Ok(()) => {
                let _ = self
                    .settings
                    .set(&["bar", "palette"], Value::String("x".into()));
                self.reload_palette();
                self.set_status("Palette deleted");
            }
            Err(e) => self.set_status(format!("Delete failed: {}", e)),
        }
    }

    // ───────────────────────── monitors ─────────────────────────

    pub fn refresh_monitors(&mut self) {
        self.monitors = monitors::read().unwrap_or_default();
    }

    fn mirror_options(&self, mon: usize) -> Vec<String> {
        let name = self
            .monitors
            .get(mon)
            .map(|m| m.name.clone())
            .unwrap_or_default();
        std::iter::once("none".to_string())
            .chain(
                self.monitors
                    .iter()
                    .filter(|o| o.name != name)
                    .map(|o| o.name.clone()),
            )
            .collect()
    }

    fn monitor_adjust(&mut self, mon: usize, field: usize, delta: i32) {
        if field == 7 {
            let options = self.mirror_options(mon);
            let Some(m) = self.monitors.get_mut(mon) else {
                return;
            };
            let pos = options.iter().position(|o| o == &m.mirror).unwrap_or(0) as i32;
            let new = (pos + delta).rem_euclid(options.len() as i32) as usize;
            m.mirror = options[new].clone();
            return;
        }
        let Some(m) = self.monitors.get_mut(mon) else {
            return;
        };
        match field {
            0 => m.disabled = !m.disabled,
            1 => {
                let modes = m.modes.clone();
                if modes.is_empty() {
                    return;
                }
                let pos = modes
                    .iter()
                    .position(|x| x.w == m.w && x.h == m.h)
                    .unwrap_or(0) as i32;
                let new = (pos + delta).rem_euclid(modes.len() as i32) as usize;
                m.w = modes[new].w;
                m.h = modes[new].h;
                m.rate = modes[new].rate;
            }
            2 => {
                let base = (m.w, m.h);
                let rates: Vec<f64> = m
                    .modes
                    .iter()
                    .filter(|x| (x.w, x.h) == base)
                    .map(|x| x.rate)
                    .collect();
                if rates.is_empty() {
                    return;
                }
                let pos = rates
                    .iter()
                    .position(|r| (r - m.rate).abs() < 0.01)
                    .unwrap_or(0) as i32;
                let new = (pos + delta).rem_euclid(rates.len() as i32) as usize;
                m.rate = rates[new];
            }
            3 => {
                const T: [u32; 4] = [0, 90, 180, 270];
                let pos = T.iter().position(|t| *t == m.transform).unwrap_or(0) as i32;
                m.transform = T[((pos + delta).rem_euclid(4)) as usize];
            }
            4 => m.vrr = ((m.vrr as i32 + delta).rem_euclid(3)) as u32,
            5 => m.bitdepth = if m.bitdepth == 8 { 10 } else { 8 },
            6 => {
                const CMS: [&str; 5] = ["auto", "srgb", "wide", "hdr", "edid"];
                let pos = CMS.iter().position(|c| *c == m.cm).unwrap_or(0) as i32;
                m.cm = CMS[((pos + delta).rem_euclid(5)) as usize].to_string();
            }
            7 => {
                // handled above (needs other monitors)
            }
            8 => m.x += (delta * 10) as i64,
            9 => m.y += (delta * 10) as i64,
            _ => {}
        }
    }

    fn apply_monitor_option(&mut self, mon: usize, field: usize, idx: usize) {
        if field == 7 {
            let options = self.mirror_options(mon);
            if let (Some(m), Some(o)) = (self.monitors.get_mut(mon), options.get(idx)) {
                m.mirror = o.clone();
            }
            return;
        }
        let Some(m) = self.monitors.get_mut(mon) else {
            return;
        };
        match field {
            1 => {
                if let Some(mode) = m.modes.get(idx) {
                    m.w = mode.w;
                    m.h = mode.h;
                    m.rate = mode.rate;
                }
            }
            2 => {
                let base = (m.w, m.h);
                let rates: Vec<f64> = m
                    .modes
                    .iter()
                    .filter(|x| (x.w, x.h) == base)
                    .map(|x| x.rate)
                    .collect();
                if let Some(r) = rates.get(idx) {
                    m.rate = *r;
                }
            }
            3 => {
                const T: [u32; 4] = [0, 90, 180, 270];
                if let Some(t) = T.get(idx) {
                    m.transform = *t;
                }
            }
            4 => m.vrr = idx as u32,
            5 => m.bitdepth = if idx == 0 { 8 } else { 10 },
            6 => {
                const CMS: [&str; 5] = ["auto", "srgb", "wide", "hdr", "edid"];
                if let Some(c) = CMS.get(idx) {
                    m.cm = (*c).to_string();
                }
            }
            7 => {
                // handled above (needs other monitors)
            }
            _ => {}
        }
    }

    fn monitor_field_options(&self, mon: usize, field: usize) -> Vec<Opt> {
        let Some(m) = self.monitors.get(mon) else {
            return Vec::new();
        };
        match field {
            1 => m
                .modes
                .iter()
                .filter(|x| (x.w, x.h) != (0, 0))
                .map(|x| Opt {
                    label: x.label.clone(),
                    value: String::new(),
                })
                .collect(),
            2 => {
                let base = (m.w, m.h);
                m.modes
                    .iter()
                    .filter(|x| (x.w, x.h) == base)
                    .map(|x| Opt {
                        label: format!("{} Hz", x.rate.round() as u32),
                        value: String::new(),
                    })
                    .collect()
            }
            3 => ["0°", "90°", "180°", "270°"]
                .iter()
                .map(|l| Opt {
                    label: (*l).into(),
                    value: String::new(),
                })
                .collect(),
            4 => ["Off", "On", "Fullscreen"]
                .iter()
                .map(|l| Opt {
                    label: (*l).into(),
                    value: String::new(),
                })
                .collect(),
            5 => ["8-bit", "10-bit"]
                .iter()
                .map(|l| Opt {
                    label: (*l).into(),
                    value: String::new(),
                })
                .collect(),
            6 => ["Auto", "sRGB", "Wide", "HDR", "EDID"]
                .iter()
                .map(|l| Opt {
                    label: (*l).into(),
                    value: String::new(),
                })
                .collect(),
            7 => std::iter::once(Opt {
                label: "None".into(),
                value: String::new(),
            })
            .chain(
                self.monitors
                    .iter()
                    .filter(|o| o.name != m.name)
                    .map(|o| Opt {
                        label: o.name.clone(),
                        value: String::new(),
                    }),
            )
            .collect(),
            _ => Vec::new(),
        }
    }

    fn monitors_apply(&mut self) {
        match monitors::apply(&mut self.monitors) {
            Ok(arr) => {
                let _ = self.settings.set(&["monitors"], Value::Array(arr));
                actions::notify("Display Update", "Applied layout");
                self.set_status("Monitors applied");
            }
            Err(e) => self.set_status(format!("Apply failed: {}", e)),
        }
    }

    fn refresh_gpu(&mut self) {
        let out = actions::capture("envycontrol --query 2>/dev/null || true");
        let lower = out.to_lowercase();
        let mode = if lower.contains("integrated") {
            "integrated"
        } else if lower.contains("hybrid") {
            "hybrid"
        } else if lower.contains("nvidia") {
            "nvidia"
        } else {
            ""
        };
        if mode.is_empty() {
            self.vstate.insert("gpu".into(), String::new());
            self.infos
                .insert("gpu.mode".into(), "envycontrol not available".into());
        } else {
            self.vstate.insert("gpu".into(), mode.to_string());
            self.infos.insert("gpu.mode".into(), mode.to_string());
        }
    }

    fn dnd_on(&self) -> bool {
        std::fs::read_to_string(std::path::Path::new(&dnd_dir()).join("state"))
            .map(|s| s.trim() == "1")
            .unwrap_or(false)
    }

    // ───────────────────────── page state ─────────────────────────

    pub fn refresh_page_state(&mut self) {
        match &self.current_page().body {
            Body::Palette => self.reload_palette(),
            Body::Monitors => self.refresh_monitors(),
            Body::HyprEffects => self.load_hypr(),
            Body::Guide => self.load_sysinfo(),
            Body::Controls(_) => match self.current_page().id {
                "d_gpu" => self.refresh_gpu(),
                "d_idle" => self.refresh_idle(),
                "d_notifications" => {
                    let on = self.dnd_on();
                    self.infos
                        .insert("dnd".into(), if on { "on".into() } else { "off".into() });
                }
                _ => {}
            },
            _ => {}
        }
        self.vstate.insert("engine".into(), self.engine.clone());
        self.rebuild_rows();
    }

    fn refresh_idle(&mut self) {
        let path = crate::settings::home().join(".config/hypr/idle-settings.json");
        let mode = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .and_then(|v| {
                v.get("idleMode")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .unwrap_or_else(|| "normal".into());
        self.vstate.insert("idle".into(), mode.clone());
        self.infos.insert("idle.mode".into(), mode);
    }

    fn load_sysinfo(&mut self) {
        let out = actions::capture(&format!(
            "bash {}",
            actions::quickshell_dir() + "/ui/bar/editor/sysinfo.sh"
        ));
        for line in out.lines() {
            if let Some((k, v)) = line.split_once('=') {
                self.infos
                    .insert(format!("sys.{}", k.trim()), v.trim().to_string());
            }
        }
    }

    // ───────────────────────── edit commit ─────────────────────────

    pub fn commit_edit(&mut self, target: EditTarget, buf: String) {
        match target {
            EditTarget::Control {
                path,
                numeric,
                label,
            } => {
                if numeric {
                    match buf.trim().parse::<f64>() {
                        Ok(n) => {
                            let val = if n.fract() == 0.0 && !buf.contains('.') {
                                json!(n as i64)
                            } else {
                                json!(n)
                            };
                            self.write_path(&path, val);
                            self.set_status(format!("{} = {}", label, buf.trim()));
                        }
                        Err(_) => self.set_status("Invalid number"),
                    }
                } else {
                    self.write_path(&path, Value::String(buf));
                    self.set_status(format!("{} saved", label));
                }
            }
            EditTarget::Keybind { kb, field } => {
                if let Some(b) = self.keybinds.get_mut(kb) {
                    b[KEYBIND_FIELDS[field].0] = Value::String(buf);
                    self.keybinds_dirty = true;
                }
            }
            EditTarget::Startup(idx) => {
                if let Some(s) = self.startup.get_mut(idx) {
                    s["command"] = Value::String(buf);
                    self.startup_dirty = true;
                }
            }
            EditTarget::PaletteSlot { slot, slug } => {
                let Some(p) = self.palette.as_mut() else {
                    return;
                };
                if p.slug != slug {
                    return;
                }
                match p.set_slot(slot, buf.trim()) {
                    Ok(()) => {
                        self.set_status(format!(
                            "{} = {}",
                            PaletteFile::slot_label(slot),
                            buf.trim()
                        ));
                        palette::sync_theme();
                    }
                    Err(_) => self.set_status("Invalid hex (#rrggbb)"),
                }
            }
            EditTarget::PaletteFilter => {
                self.palette_filter = buf;
            }
            EditTarget::Prompt { prefix, prompt } => {
                if prefix == "__palette_create__" {
                    if !buf.trim().is_empty() {
                        self.palette_prompt_create(buf.trim());
                    }
                } else if !buf.trim().is_empty() {
                    let escaped = buf.replace('\'', "'\\''");
                    actions::spawn(&format!("{} '{}'", prefix, escaped));
                    self.set_status(format!("{} saved", prompt));
                }
            }
        }
        self.rebuild_rows();
    }

    // ───────────────────────── input mode handlers ─────────────────────────

    pub fn begin_confirm(&mut self, action: Action, msg: &str) {
        self.mode = Mode::Confirm {
            action,
            msg: msg.to_string(),
        };
    }

    pub fn confirm_yes(&mut self) {
        let Mode::Confirm { action, .. } = std::mem::replace(&mut self.mode, Mode::Browse) else {
            return;
        };
        match action {
            Action::ZonesDefault => self.zones_default(),
            Action::PaletteReset => {
                if let Some(p) = self.palette.as_mut() {
                    match p.reset() {
                        Ok(()) => {
                            self.set_status("Palette restored from backup");
                            palette::sync_theme();
                        }
                        Err(e) => self.set_status(format!("Reset failed: {}", e)),
                    }
                }
            }
            Action::PaletteDelete => {
                if let Some(slug) = self.palette_delete_slug.take() {
                    match palette::delete(&slug) {
                        Ok(()) => {
                            let active = self
                                .settings
                                .get_str(&["bar", "palette"])
                                .unwrap_or_default();
                            if active == slug {
                                let _ = self
                                    .settings
                                    .set(&["bar", "palette"], Value::String("x".into()));
                            }
                            self.reload_palette();
                            self.set_status("Palette deleted");
                        }
                        Err(e) => self.set_status(format!("Delete failed: {}", e)),
                    }
                } else {
                    self.palette_delete();
                }
            }
            other => self.run_action(other, None),
        }
        self.rebuild_rows();
    }

    pub fn chooser_accept(&mut self) {
        let Mode::Chooser { target, idx } = std::mem::replace(&mut self.mode, Mode::Browse) else {
            return;
        };
        self.apply_chooser_value(target, idx);
        self.rebuild_rows();
    }

    // ───────────────────────── ticks / pending ─────────────────────────

    pub fn tick(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_status_clear) > Duration::from_secs(5)
            && !self.status.is_empty()
        {
            self.status.clear();
        }
        if now.duration_since(self.last_check) > Duration::from_millis(800) {
            self.last_check = now;
            match self.settings.reload_if_changed() {
                Ok(true) => {
                    self.engine = self
                        .settings
                        .get_str(&["barEngine"])
                        .unwrap_or_else(|| "bar".into());
                    if let Some(gate) = self.current_page().engine
                        && gate != self.engine
                    {
                        self.page = self
                            .pages
                            .iter()
                            .position(|p| p.id == "d_engine")
                            .unwrap_or(0);
                    }
                    self.rebuild_rows();
                }
                Ok(false) => {}
                Err(e) => self.set_status(format!("reload error: {e}")),
            }
        }
        let due: Vec<PendingKind> = {
            let mut due = Vec::new();
            let mut rest = Vec::new();
            for pending in self.pending.drain(..) {
                if pending.at <= now {
                    due.push(pending.kind);
                } else {
                    rest.push(pending);
                }
            }
            self.pending = rest;
            due
        };
        for kind in due {
            match kind {
                PendingKind::WorkspaceReload => {
                    actions::spawn(&format!(
                        "qs -p {}/Shell.qml ipc call topbar queueReload",
                        actions::quickshell_dir()
                    ));
                }
                PendingKind::TimexRefresh => {
                    actions::spawn(
                        "TIMEX=\"${TIMEX_CLI:-$HOME/.local/bin/timex}\"; [ -x \"$TIMEX\" ] || TIMEX=timex; \"$TIMEX\" --invalidate >/dev/null 2>&1; \"$TIMEX\" --getdata >/dev/null 2>&1",
                    );
                }
                PendingKind::PersistInput => self.persist_input(),
                PendingKind::PersistAnimations => self.persist_animations(),
                PendingKind::HyprPreview => self.flush_hypr(),
            }
        }
    }

    fn persist_input(&mut self) {
        let sens = self
            .settings
            .get_f64(&["input", "sensitivity"])
            .unwrap_or(0.0);
        let profile = self
            .settings
            .get_str(&["input", "accelProfile"])
            .unwrap_or_else(|| "flat".into());
        let tap = self
            .settings
            .get_bool(&["input", "tapToClick"])
            .unwrap_or(true);
        let natural = self
            .settings
            .get_bool(&["input", "naturalScroll"])
            .unwrap_or(true);
        let dwt = self
            .settings
            .get_bool(&["input", "disableWhileTyping"])
            .unwrap_or(true);
        let script = format!(
            "{}/ui/bar/editor/persist-hypr.sh",
            actions::quickshell_dir()
        );
        actions::spawn(&format!(
            "bash {} input {} {} {} {} {}",
            script,
            fmt_num(sens),
            profile,
            if tap { 1 } else { 0 },
            if natural { 1 } else { 0 },
            if dwt { 1 } else { 0 }
        ));
    }

    fn persist_animations(&mut self) {
        let enabled = self
            .settings
            .get_bool(&["animations", "enabled"])
            .unwrap_or(true);
        let speed = self
            .settings
            .get_f64(&["animations", "speed"])
            .unwrap_or(1.0);
        let script = format!(
            "{}/ui/bar/editor/persist-hypr.sh",
            actions::quickshell_dir()
        );
        actions::spawn(&format!(
            "bash {} animations {} {:.1}",
            script,
            if enabled { 1 } else { 0 },
            speed
        ));
    }

    fn select_last_of(&mut self, pred: impl Fn(&Row) -> bool) {
        if let Some(idx) = self.rows.iter().rposition(pred) {
            self.sel = idx;
        }
    }

    // ───────────────────────── search palette ─────────────────────────

    pub fn open_search(&mut self) {
        self.open_search_with("");
    }

    pub fn open_search_with(&mut self, query: &str) {
        self.search_index = self.build_search_index();
        self.mode = Mode::Search(SearchState {
            query: query.to_string(),
            cursor: query.chars().count(),
            results: self.search_index.clone(),
            sel: 0,
        });
        if !query.is_empty() {
            self.search_refresh();
        }
    }

    fn build_search_index(&self) -> Vec<SearchEntry> {
        let mut out: Vec<SearchEntry> = Vec::new();
        let mut push = |text: String, context: String, target: SearchTarget| {
            out.push(SearchEntry {
                text,
                context,
                target,
            });
        };
        push(
            "Reload settings.json".into(),
            "Command".into(),
            SearchTarget::Command(SearchCommand::Reload),
        );
        push(
            "Quit xturing".into(),
            "Command".into(),
            SearchTarget::Command(SearchCommand::Quit),
        );
        push(
            "Help".into(),
            "Command".into(),
            SearchTarget::Command(SearchCommand::Help),
        );
        push(
            format!(
                "Switch engine ({})",
                if self.engine == "bar" {
                    "bar -> classic"
                } else {
                    "classic -> bar"
                }
            ),
            "Command".into(),
            SearchTarget::Command(SearchCommand::ToggleEngine),
        );
        for &pi in &self.visible_page_indices() {
            let page = &self.pages[pi];
            let group = page.group.label();
            push(
                page.label.to_string(),
                format!("{} › page", group),
                SearchTarget::Page(pi),
            );
            for (section, index, control) in self.page_controls_with_sections(page) {
                if !self.control_visible(&control) {
                    continue;
                }
                let ctx = if section.is_empty() {
                    format!("{} › {}", group, page.label)
                } else {
                    format!("{} › {} · {}", group, page.label, section)
                };
                push(
                    control.label.clone(),
                    ctx,
                    SearchTarget::Control { page: pi, index },
                );
            }
        }
        if let Some(zones_page) = self.pages.iter().position(|p| p.id == "d_zones") {
            for (z, zone) in self.zones().iter().enumerate() {
                let id = zone.get("id").and_then(Value::as_str).unwrap_or("zone");
                push(
                    format!("Zone {}", id),
                    "Bar › Zones".into(),
                    SearchTarget::Zone {
                        page: zones_page,
                        zone: z,
                    },
                );
                if let Some(modules) = zone.get("modules").and_then(Value::as_array) {
                    for (i, module) in modules.iter().enumerate() {
                        let Some(module_id) = module.get("id").and_then(Value::as_str) else {
                            continue;
                        };
                        let enabled = module
                            .get("enabled")
                            .and_then(Value::as_bool)
                            .unwrap_or(false);
                        push(
                            format!("Module {}", module_id),
                            format!(
                                "Bar › Zones · {} ({})",
                                id,
                                if enabled { "on" } else { "off" }
                            ),
                            SearchTarget::ZoneModule {
                                page: zones_page,
                                zone: z,
                                index: i,
                            },
                        );
                    }
                }
            }
        }
        if let Some(pal_page) = self.pages.iter().position(|p| p.id == "d_palette") {
            for (i, entry) in self.palettes.iter().enumerate() {
                push(
                    entry.name.clone(),
                    format!("Theme › Palette · {}", entry.category),
                    SearchTarget::PaletteItem {
                        page: pal_page,
                        index: i,
                    },
                );
            }
            if let Some(active) = &self.palette {
                for slot in 0..18 {
                    push(
                        format!("{} {}", PaletteFile::slot_label(slot), active.slot(slot)),
                        format!("Theme › Palette · {} (active)", active.slug),
                        SearchTarget::PaletteSlot {
                            page: pal_page,
                            slot,
                        },
                    );
                }
            }
        }
        if let Some(kb_page) = self.pages.iter().position(|p| p.id == "s_keyboard") {
            for (i, b) in self.keybinds.iter().enumerate() {
                let combo = format!(
                    "{} {}",
                    b.get("mods").and_then(Value::as_str).unwrap_or(""),
                    b.get("key").and_then(Value::as_str).unwrap_or("")
                );
                let detail = b.get("command").and_then(Value::as_str).unwrap_or("");
                push(
                    format!("{}  {}", combo.trim(), detail),
                    "Shell › Keyboard".into(),
                    SearchTarget::Keybind {
                        page: kb_page,
                        index: i,
                    },
                );
            }
        }
        if let Some(mon_page) = self.pages.iter().position(|p| p.id == "s_monitors") {
            for (m, monitor) in self.monitors.iter().enumerate() {
                push(
                    monitor.name.clone(),
                    format!("Shell › Monitors · {}", monitor.description),
                    SearchTarget::Monitor {
                        page: mon_page,
                        monitor: m,
                    },
                );
            }
        }
        if let Some(st_page) = self.pages.iter().position(|p| p.id == "s_startup") {
            for (i, s) in self.startup.iter().enumerate() {
                let cmd = s.get("command").and_then(Value::as_str).unwrap_or("");
                if cmd.trim().is_empty() {
                    continue;
                }
                push(
                    cmd.to_string(),
                    "Shell › Startup".into(),
                    SearchTarget::Startup {
                        page: st_page,
                        index: i,
                    },
                );
            }
        }
        out
    }

    /// (section title, flat index, control) for every control of a page,
    /// regardless of which page is currently open. Indexes match the ones
    /// produced by `rebuild_rows` for that page.
    fn page_controls_with_sections(&self, page: &Page) -> Vec<(String, usize, Control)> {
        let mut out = Vec::new();
        match &page.body {
            Body::Controls(sections) | Body::ClassicBar(sections) => {
                let mut i = 0usize;
                for section in sections {
                    for c in &section.controls {
                        out.push((section.title.clone(), i, c.clone()));
                        i += 1;
                    }
                }
            }
            Body::HyprEffects => {
                let controls = hypr_controls();
                let n = catalog::HYPR_SLIDERS.len();
                for (i, c) in controls.iter().enumerate() {
                    let section = if i < n {
                        "Window effects"
                    } else if i < n + 2 {
                        "Actions"
                    } else {
                        "Window borders"
                    };
                    out.push((section.to_string(), i, c.clone()));
                }
            }
            Body::Zones => {
                for (i, c) in zones_actions().iter().enumerate() {
                    out.push(("Actions".to_string(), i, c.clone()));
                }
            }
            Body::Monitors => {
                for (i, c) in monitors_actions().iter().enumerate() {
                    out.push(("Actions".to_string(), i, c.clone()));
                }
            }
            Body::Palette => {
                for (i, c) in palette_actions().iter().enumerate() {
                    let section = if i == 0 { "Filter" } else { "Actions" };
                    out.push((section.to_string(), i, c.clone()));
                }
            }
            Body::Guide => {
                for (i, c) in guide_actions().iter().enumerate() {
                    out.push(("Actions".to_string(), i, c.clone()));
                }
            }
            _ => {}
        }
        out
    }

    pub fn search_refresh(&mut self) {
        let Mode::Search(state) = &mut self.mode else {
            return;
        };
        let query = state.query.clone();
        let mut scored: Vec<(i32, SearchEntry)> = self
            .search_index
            .iter()
            .filter_map(|entry| {
                let haystack = format!("{} {}", entry.text, entry.context);
                fuzzy_score(&query, &haystack).map(|score| (score, entry.clone()))
            })
            .collect();
        scored.sort_by_key(|entry| std::cmp::Reverse(entry.0));
        state.results = scored.into_iter().map(|(_, e)| e).collect();
        state.sel = 0;
    }

    pub fn search_type_char(&mut self, c: char) {
        if let Mode::Search(state) = &mut self.mode {
            let byte = state
                .query
                .char_indices()
                .nth(state.cursor)
                .map(|(i, _)| i)
                .unwrap_or(state.query.len());
            state.query.insert(byte, c);
            state.cursor += 1;
        }
        self.search_refresh();
    }

    pub fn search_backspace(&mut self) {
        if let Mode::Search(state) = &mut self.mode
            && state.cursor > 0
            && let Some((byte, _)) = state.query.char_indices().nth(state.cursor - 1)
        {
            state.query.remove(byte);
            state.cursor -= 1;
        }
        self.search_refresh();
    }

    pub fn search_move(&mut self, delta: i32) {
        if let Mode::Search(state) = &mut self.mode {
            let len = state.results.len() as i32;
            if len == 0 {
                return;
            }
            state.sel = (state.sel as i32 + delta).rem_euclid(len) as usize;
        }
    }

    pub fn search_cancel(&mut self) {
        self.mode = Mode::Browse;
    }

    pub fn search_accept(&mut self) {
        let Mode::Search(state) = std::mem::replace(&mut self.mode, Mode::Browse) else {
            return;
        };
        if let Some(entry) = state.results.get(state.sel).cloned() {
            self.jump_target(entry.target);
        }
    }

    pub fn jump_target(&mut self, target: SearchTarget) {
        self.mode = Mode::Browse;
        match target {
            SearchTarget::Command(SearchCommand::Reload) => self.hard_reload(),
            SearchTarget::Command(SearchCommand::Quit) => self.quit = true,
            SearchTarget::Command(SearchCommand::Help) => self.mode = Mode::Help,
            SearchTarget::Command(SearchCommand::ToggleEngine) => {
                let next = if self.engine == "bar" {
                    "classic"
                } else {
                    "bar"
                };
                self.set_engine(next);
            }
            SearchTarget::Page(page) => {
                self.page = page;
                self.sel = 0;
                self.refresh_page_state();
            }
            SearchTarget::Control { page, index } => {
                self.goto_row(page, |row| matches!(row, Row::Control(i) if *i == index));
            }
            SearchTarget::Zone { page, zone } => {
                self.goto_row(page, |row| matches!(row, Row::ZoneHeader(z) if *z == zone));
            }
            SearchTarget::ZoneModule { page, zone, index } => {
                self.goto_row(page, |row| {
                    matches!(row, Row::ZoneModule { zone: z, idx: i } if *z == zone && *i == index)
                });
            }
            SearchTarget::PaletteSlot { page, slot } => {
                self.goto_row(page, |row| matches!(row, Row::PaletteSlot(s) if *s == slot));
            }
            SearchTarget::Monitor { page, monitor } => {
                self.goto_row(
                    page,
                    |row| matches!(row, Row::MonitorField { mon: m, field: 0 } if *m == monitor),
                );
            }
            SearchTarget::PaletteItem { page, index } => {
                self.goto_row(
                    page,
                    |row| matches!(row, Row::PaletteItem(i) if *i == index),
                );
            }
            SearchTarget::Keybind { page, index } => {
                self.goto_row(page, |row| matches!(row, Row::Keybind(i) if *i == index));
            }
            SearchTarget::Startup { page, index } => {
                self.goto_row(page, |row| matches!(row, Row::Startup(i) if *i == index));
            }
        }
    }

    fn goto_row(&mut self, page: usize, predicate: impl Fn(&Row) -> bool) {
        self.remember_selection();
        self.page = page;
        self.refresh_page_state();
        if let Some(idx) = self.rows.iter().position(predicate) {
            self.sel = idx;
        }
        self.remember_selection();
    }

    // ───────────────────────── mouse / touch ─────────────────────────

    pub fn apply_hit(&mut self, action: &HitAction) {
        match action {
            HitAction::SelectRow(i) => {
                let now = Instant::now();
                let double = matches!(self.last_tap, Some((row, at)) if row == *i && now.duration_since(at) < Duration::from_millis(450));
                self.sel = (*i).min(self.rows.len().saturating_sub(1));
                self.remember_selection();
                if double {
                    self.last_tap = None;
                    self.activate_row();
                } else {
                    self.last_tap = Some((*i, now));
                }
            }
            HitAction::Adjust { ctrl, delta } => {
                self.adjust_control(*ctrl, *delta);
            }
            HitAction::GotoPage(page) => {
                self.remember_selection();
                self.page = *page;
                self.sel = 0;
                self.refresh_page_state();
                self.restore_selection();
            }
            HitAction::SearchSelect(i) => {
                if let Mode::Search(state) = &mut self.mode {
                    state.sel = (*i).min(state.results.len().saturating_sub(1));
                }
                self.search_accept();
            }
            HitAction::ChooserSelect(i) => {
                if let Mode::Chooser { idx, .. } = &mut self.mode {
                    *idx = *i;
                }
                self.chooser_accept();
            }
        }
    }

    pub fn selected_stepper_control(&self) -> Option<usize> {
        match self.selected_row()? {
            Row::Control(i) => match self.controls.get(i)?.kind {
                Kind::Stepper { .. } => Some(i),
                _ => None,
            },
            _ => None,
        }
    }
}

pub const KEYBIND_FIELDS: [(&str, &str); 5] = [
    ("type", "Type"),
    ("mods", "Mods"),
    ("key", "Key"),
    ("dispatcher", "Dispatcher"),
    ("command", "Command"),
];

pub const KEYBIND_OPTIONS: [&[(&str, &str)]; 5] = [
    &[
        ("bind", "bind"),
        ("binde", "binde"),
        ("bindl", "bindl"),
        ("bindel", "bindel"),
        ("bindm", "bindm"),
    ],
    &[],
    &[],
    &[
        ("exec", "exec"),
        ("exec-once", "exec-once"),
        ("dispatch", "dispatch"),
        ("workspace", "workspace"),
        ("movetoworkspace", "movetoworkspace"),
        ("movewindow", "movewindow"),
        ("resizeactive", "resizeactive"),
        ("movefocus", "movefocus"),
        ("togglefloating", "togglefloating"),
        ("killactive", "killactive"),
    ],
    &[],
];

pub const ZONE_FIELD_COUNT: usize = 8;
pub const ZONE_FIELD_LABELS: [&str; 8] = [
    "Align",
    "Unify",
    "Container bg",
    "Container bg color",
    "Container bg solid",
    "Zone border",
    "Zone border color",
    "Delete zone (d)",
];

pub const HYPR_ORDER: [&str; 12] = [
    "active_opacity",
    "inactive_opacity",
    "rounding",
    "blur_size",
    "blur_passes",
    "gaps_in",
    "gaps_out",
    "border_size",
    "shadow_range",
    "shadow_render_power",
    "shadow_offset_x",
    "shadow_offset_y",
];

fn hypr_controls() -> Vec<Control> {
    let mut controls = Vec::new();
    for (key, label, _, min, max, step, decimals) in catalog::HYPR_SLIDERS {
        let dec: u32 = decimals.parse().unwrap_or(0);
        controls.push(Control {
            label: (*label).to_string(),
            kind: Kind::Stepper {
                step: *step,
                min: *min,
                max: *max,
                decimals: dec,
                unit: "",
            },
            path: vec!["__hypr__".to_string(), (*key).to_string()],
            help: "Live preview with hyprctl eval; persists to config/window-effects.lua on exit"
                .to_string(),
            visible: None,
        });
    }
    controls.push(Control {
        label: "Reset effects".into(),
        kind: Kind::Action(Action::HyprReset),
        path: Vec::new(),
        help: "Back to defaults (does not touch gaps or border width)".into(),
        visible: None,
    });
    controls.push(Control {
        label: "Refresh".into(),
        kind: Kind::Action(Action::HyprRefresh),
        path: Vec::new(),
        help: "Re-read the live values with hyprctl".into(),
        visible: None,
    });
    // Window borders (same block as Bar > Style)
    let borders = catalog::border_sections();
    for c in flatten(&borders) {
        controls.push(c);
    }
    controls
}

fn zones_actions() -> Vec<Control> {
    vec![
        Control {
            label: "Add zone".into(),
            kind: Kind::Action(Action::ZoneAdd),
            path: Vec::new(),
            help: "Add a new zone (align start)".into(),
            visible: None,
        },
        Control {
            label: "Center all".into(),
            kind: Kind::Action(Action::ZonesCenterAll),
            path: Vec::new(),
            help: "Move every enabled module to the front of the first center zone".into(),
            visible: None,
        },
        Control {
            label: "Default".into(),
            kind: Kind::Action(Action::ZonesDefault),
            path: Vec::new(),
            help: "Reset the whole bar to factory values".into(),
            visible: None,
        },
    ]
}

fn monitors_actions() -> Vec<Control> {
    vec![
        Control {
            label: "Apply & save permanently".into(),
            kind: Kind::Action(Action::MonitorsApply),
            path: Vec::new(),
            help: "Apply and save the layout to display-config".into(),
            visible: None,
        },
        Control {
            label: "Reset to auto".into(),
            kind: Kind::Action(Action::MonitorsReset),
            path: Vec::new(),
            help: "Delete display-config and go back to auto".into(),
            visible: None,
        },
        Control {
            label: "Refresh".into(),
            kind: Kind::Action(Action::RefreshPage),
            path: Vec::new(),
            help: "Relee hyprctl monitors -j".into(),
            visible: None,
        },
    ]
}

fn guide_actions() -> Vec<Control> {
    vec![
        Control {
            label: "Copy debug info".into(),
            kind: Kind::Action(Action::CopyDebug),
            path: Vec::new(),
            help: "sysinfo.sh | wl-copy".into(),
            visible: None,
        },
        Control {
            label: "Run doctor".into(),
            kind: Kind::Action(Action::RunDoctor),
            path: Vec::new(),
            help: "dots doctor".into(),
            visible: None,
        },
        Control {
            label: "Updates".into(),
            kind: Kind::Action(Action::OpenWidget("updater".into())),
            path: Vec::new(),
            help: "Open the updater popup".into(),
            visible: None,
        },
        Control {
            label: "Docs".into(),
            kind: Kind::Action(Action::OpenUrl("https://github.com/equisdots".into())),
            path: Vec::new(),
            help: String::new(),
            visible: None,
        },
        Control {
            label: "Report issue".into(),
            kind: Kind::Action(Action::OpenUrl(
                "https://github.com/equisdots/shell/issues/new".into(),
            )),
            path: Vec::new(),
            help: String::new(),
            visible: None,
        },
    ]
}

fn palette_actions() -> Vec<Control> {
    vec![
        Control {
            label: "Filter (press Enter to edit)".into(),
            kind: Kind::Action(Action::PaletteFilterEdit),
            path: Vec::new(),
            help: "Type to filter; empty = all".into(),
            visible: None,
        },
        Control {
            label: "Reset active palette".into(),
            kind: Kind::Action(Action::PaletteReset),
            path: Vec::new(),
            help: "Restore from the session backup if present".into(),
            visible: None,
        },
        Control {
            label: "New palette".into(),
            kind: Kind::Action(Action::PaletteCreate),
            path: Vec::new(),
            help: "Create a user palette from the active one".into(),
            visible: None,
        },
        Control {
            label: "Delete active palette".into(),
            kind: Kind::Action(Action::PaletteDelete),
            path: Vec::new(),
            help: "'x' cannot be deleted".into(),
            visible: None,
        },
    ]
}

fn flatten(sections: &[catalog::Section]) -> Vec<Control> {
    let mut out = Vec::new();
    for s in sections {
        for c in &s.controls {
            out.push(Control {
                label: c.label.clone(),
                kind: clone_kind(&c.kind),
                path: c.path.clone(),
                help: c.help.clone(),
                visible: c.visible.clone(),
            });
        }
    }
    out
}

fn clone_kind(kind: &Kind) -> Kind {
    match kind {
        Kind::Toggle => Kind::Toggle,
        Kind::Stepper {
            step,
            min,
            max,
            decimals,
            unit,
        } => Kind::Stepper {
            step: *step,
            min: *min,
            max: *max,
            decimals: *decimals,
            unit,
        },
        Kind::Options(opts) => Kind::Options(opts.clone()),
        Kind::StateOptions {
            state_key,
            options,
            action,
        } => Kind::StateOptions {
            state_key: state_key.clone(),
            options: options.clone(),
            action: action.clone(),
        },
        Kind::Text { placeholder, max } => Kind::Text {
            placeholder: placeholder.clone(),
            max: *max,
        },
        Kind::Action(a) => Kind::Action(a.clone()),
        Kind::Info(k) => Kind::Info(k.clone()),
    }
}

fn p(path: &str) -> Vec<String> {
    path.split('.').map(str::to_string).collect()
}

fn value_to_string(v: Value) -> String {
    match v {
        Value::String(s) => s,
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_string()
            } else {
                n.as_f64().map(fmt_num).unwrap_or_default()
            }
        }
        other => other.to_string(),
    }
}

/// Map a crossterm key press to Hyprland (mods, key) strings for the
/// keybind recorder. Returns None for modifier-only presses.
pub fn shortcut_fields(
    code: crossterm::event::KeyCode,
    modifiers: crossterm::event::KeyModifiers,
) -> Option<(String, String)> {
    use crossterm::event::{KeyCode, KeyModifiers};
    if matches!(code, KeyCode::Modifier(_)) {
        return None;
    }
    let key = match code {
        KeyCode::Char(' ') => "Space".to_string(),
        KeyCode::Char(c) => match c {
            '.' => "period".to_string(),
            ',' => "comma".to_string(),
            '\'' => "apostrophe".to_string(),
            ';' => "semicolon".to_string(),
            '/' => "slash".to_string(),
            '\\' => "backslash".to_string(),
            '`' => "grave".to_string(),
            '-' => "minus".to_string(),
            '=' => "equal".to_string(),
            '[' => "bracketleft".to_string(),
            ']' => "bracketright".to_string(),
            other => other.to_uppercase().to_string(),
        },
        KeyCode::Enter => "Return".to_string(),
        KeyCode::Esc => "Escape".to_string(),
        KeyCode::Backspace => "BackSpace".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        KeyCode::Tab | KeyCode::BackTab => "Tab".to_string(),
        KeyCode::Left => "left".to_string(),
        KeyCode::Right => "right".to_string(),
        KeyCode::Up => "up".to_string(),
        KeyCode::Down => "down".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "Page_Up".to_string(),
        KeyCode::PageDown => "Page_Down".to_string(),
        KeyCode::Insert => "Insert".to_string(),
        KeyCode::PrintScreen => "Print".to_string(),
        KeyCode::F(n) => format!("F{}", n),
        _ => return None,
    };
    let mut mods: Vec<&str> = Vec::new();
    if modifiers.contains(KeyModifiers::SUPER) {
        mods.push("SUPER");
    }
    if modifiers.contains(KeyModifiers::CONTROL) {
        mods.push("CTRL");
    }
    if modifiers.contains(KeyModifiers::ALT) {
        mods.push("ALT");
    }
    if modifiers.contains(KeyModifiers::SHIFT) {
        mods.push("SHIFT");
    }
    Some((mods.join(" "), key))
}

pub fn fmt_num(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{}", v as i64)
    } else {
        let s = format!("{:.2}", v);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

fn fuzzy_score(query: &str, text: &str) -> Option<i32> {
    if query.trim().is_empty() {
        return Some(0);
    }
    let needle: Vec<char> = query
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let hay: Vec<char> = text.to_lowercase().chars().collect();
    let mut qi = 0usize;
    let mut score = 0i32;
    let mut last: i32 = -2;
    for (i, &c) in hay.iter().enumerate() {
        if qi < needle.len() && c == needle[qi] {
            score += 10;
            if i as i32 == last + 1 {
                score += 15;
            }
            if i == 0 || !hay[i - 1].is_alphanumeric() {
                score += 8;
            }
            last = i as i32;
            qi += 1;
        }
    }
    if qi == needle.len() {
        Some(score - (hay.len() as i32 / 4))
    } else {
        None
    }
}

fn json_eq_loose(got: Option<&Value>, want: &Value) -> bool {
    match (got, want) {
        (Some(Value::String(a)), Value::String(b)) => a == b,
        (Some(Value::Bool(a)), Value::Bool(b)) => a == b,
        (Some(Value::String(a)), Value::Number(n)) => a.parse::<f64>().ok() == n.as_f64(),
        (Some(a), Value::Number(_)) => a.as_f64() == want.as_f64(),
        (Some(a), b) => a == b,
        _ => false,
    }
}

fn lua_dispatcher(dispatcher: &str, command: &str) -> Option<String> {
    let command = command.trim();
    let json_str = |s: &str| serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into());
    Some(match dispatcher {
        "exec" | "exec-once" => format!("hl.dsp.exec_cmd({})", json_str(command)),
        "workspace" => format!("hl.dsp.focus({{ workspace = {} }})", json_str(command)),
        "movetoworkspace" => format!(
            "hl.dsp.window.move({{ workspace = {} }})",
            json_str(command)
        ),
        "movewindow" => match command {
            "l" | "r" | "u" | "d" => {
                format!("hl.dsp.window.move({{ direction = \"{}\" }})", command)
            }
            _ => format!("hl.dsp.window.move({{ monitor = {} }})", json_str(command)),
        },
        "movefocus" => {
            let dir = match command {
                "l" | "r" | "u" | "d" => command,
                _ => "l",
            };
            format!("hl.dsp.focus({{ direction = \"{}\" }})", dir)
        }
        "resizeactive" => {
            let parts: Vec<&str> = command
                .split(|c: char| c.is_whitespace() || c == ',')
                .collect();
            let x: i64 = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
            let y: i64 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
            format!(
                "hl.dsp.window.resize({{ x = {}, y = {}, relative = true }})",
                x, y
            )
        }
        "togglefloating" => "hl.dsp.window.float({ action = \"toggle\" })".to_string(),
        "killactive" => "hl.dsp.window.kill()".to_string(),
        _ => return None,
    })
}

pub fn dnd_dir() -> String {
    if let Ok(dir) = std::env::var("QS_CACHE_DND") {
        return dir;
    }
    format!(
        "{}/.cache/quickshell/dnd",
        crate::settings::home().display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn sandbox(json: &str) -> (App, std::path::PathBuf) {
        crate::actions::set_dry(true);
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir =
            std::env::temp_dir().join(format!("xturing-app-{}-{}", std::process::id(), unique));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        fs::write(&path, json).unwrap();
        let settings = Settings::load_from(path.clone()).unwrap();
        let app = App::with_settings(settings).unwrap();
        (app, path)
    }

    #[test]
    fn fuzzy_search_ranks_and_jumps() {
        let (mut app, _) = sandbox(r#"{"barEngine":"bar","bar":{"thickness":48}}"#);
        app.open_search();
        app.search_type_char('t');
        app.search_type_char('h');
        app.search_type_char('i');
        app.search_type_char('c');
        app.search_type_char('k');
        let results = match &app.mode {
            Mode::Search(state) => state.results.clone(),
            _ => panic!("search mode"),
        };
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.text == "Thickness"));
        // select the first Thickness result and jump
        let idx = results.iter().position(|r| r.text == "Thickness").unwrap();
        app.search_move(idx as i32);
        app.search_accept();
        assert_eq!(app.current_page().id, "d_style");
        match app.selected_row() {
            Some(Row::Control(i)) => {
                let c = &app.controls[i];
                assert_eq!(c.path, ["bar", "thickness"]);
            }
            other => panic!("expected thickness row, got {other:?}"),
        }
    }

    #[test]
    fn search_commands_and_dynamic_entries() {
        let (mut app, _) = sandbox(
            r#"{"keybinds":[{"mods":"SUPER","key":"Q","dispatcher":"exec","command":"kitty"}],"bar":{"zones":[{"id":"start","align":"start","modules":[]}]}}"#,
        );
        app.open_search();
        let has = |app: &App, text: &str| match &app.mode {
            Mode::Search(s) => s.results.iter().any(|r| r.text.contains(text)),
            _ => false,
        };
        assert!(has(&app, "Reload settings.json"));
        assert!(has(&app, "SUPER Q"));
        assert!(has(&app, "Zone start"));
    }

    #[test]
    fn mouse_hits_adjust_and_double_tap_select() {
        let (mut app, _) = sandbox(r#"{"barEngine":"bar","uiScale":1.0}"#);
        app.page = 0;
        app.refresh_page_state();
        // hit map is built by the renderer; emulate one Adjust hit here
        app.apply_hit(&HitAction::Adjust { ctrl: 0, delta: 1 });
        assert_eq!(app.settings.get_f64(&["uiScale"]), Some(1.1));
        // double tap on a row activates it (select row then activate via hit twice)
        let row = app
            .rows
            .iter()
            .position(|r| matches!(r, Row::Control(0)))
            .unwrap();
        app.apply_hit(&HitAction::SelectRow(row));
        app.apply_hit(&HitAction::SelectRow(row));
        // UI Scale is a stepper: activation opens the edit mode
        assert!(matches!(app.mode, Mode::Edit { .. }));
    }

    #[test]
    fn selection_memory_survives_page_round_trip() {
        let (mut app, _) = sandbox(r#"{"barEngine":"bar"}"#);
        app.page = app.pages.iter().position(|p| p.id == "s_general").unwrap();
        app.refresh_page_state();
        // pick a selectable row beyond the first
        app.sel = app.rows.iter().rposition(|r| r.selectable()).unwrap();
        let remembered = app.sel;
        app.next_page(1);
        assert_ne!(app.current_page().id, "s_general");
        app.next_page(-1);
        assert_eq!(app.current_page().id, "s_general");
        assert_eq!(app.sel, remembered);
    }

    #[test]
    fn arrows_wrap_across_pages() {
        let (mut app, _) = sandbox(r#"{"barEngine":"bar"}"#);
        app.page = 0;
        app.refresh_page_state();
        let first_page = app.current_page().id;
        // keep pressing Down until the page changes
        let mut guard = 0;
        while app.current_page().id == first_page && guard < 500 {
            app.move_selection(1);
            guard += 1;
        }
        assert_ne!(
            app.current_page().id,
            first_page,
            "Down did not cross to the next page"
        );
        assert_eq!(
            Some(app.sel),
            app.rows.iter().position(|r| r.selectable()),
            "crossing down must land on the first selectable row"
        );
        // keep pressing Up until we are back on the first page
        guard = 0;
        while app.current_page().id != first_page && guard < 500 {
            app.move_selection(-1);
            guard += 1;
        }
        assert_eq!(app.current_page().id, first_page, "Up did not cross back");
        assert_eq!(
            Some(app.sel),
            app.rows.iter().rposition(|r| r.selectable()),
            "crossing up must land on the last selectable row"
        );
    }

    #[test]
    fn goto_page_id_opens_visible_pages() {
        let (mut app, _) = sandbox(r#"{"barEngine":"bar"}"#);
        assert!(app.goto_page_id("d_style"));
        assert_eq!(app.current_page().id, "d_style");
        // engine-gated page hidden while engine is bar? d_style is bar-only and visible
        assert!(!app.goto_page_id("d_classic"));
        app.run_action(Action::SetEngine, Some("classic".into()));
        assert!(app.goto_page_id("d_classic"));
    }

    #[test]
    fn search_reaches_zone_modules_and_pages() {
        let (mut app, _) = sandbox(
            r#"{"bar":{"zones":[{"id":"start","align":"start","modules":[{"id":"help","enabled":true},{"id":"volume","enabled":false}]}]}}"#,
        );
        app.open_search_with("module vol");
        let results = match &app.mode {
            Mode::Search(state) => state.results.clone(),
            _ => panic!("search mode"),
        };
        assert!(results.iter().any(|r| r.text == "Module volume"));
        assert!(results.iter().any(|r| r.context.contains("start")));
    }

    #[test]
    fn shortcut_recorder_maps_hyprland_names() {
        use crossterm::event::{KeyCode, KeyModifiers};
        assert_eq!(
            shortcut_fields(
                KeyCode::Char('x'),
                KeyModifiers::SUPER | KeyModifiers::SHIFT
            ),
            Some(("SUPER SHIFT".into(), "X".into()))
        );
        assert_eq!(
            shortcut_fields(KeyCode::Char('.'), KeyModifiers::SUPER),
            Some(("SUPER".into(), "period".into()))
        );
        assert_eq!(
            shortcut_fields(KeyCode::Enter, KeyModifiers::empty()),
            Some((String::new(), "Return".into()))
        );
        assert_eq!(
            shortcut_fields(KeyCode::F(5), KeyModifiers::ALT),
            Some(("ALT".into(), "F5".into()))
        );
        assert_eq!(
            shortcut_fields(
                KeyCode::Modifier(crossterm::event::ModifierKeyCode::LeftShift),
                KeyModifiers::SHIFT
            ),
            None
        );
    }

    #[test]
    fn every_page_and_row_rebuilds_without_panic() {
        let (mut app, _) = sandbox(r#"{"barEngine":"bar"}"#);
        let pages = app.pages.len();
        for i in 0..pages {
            app.page = i;
            app.sel = 0;
            app.refresh_page_state();
            assert!(
                !app.rows.is_empty(),
                "page {} has no rows",
                app.current_page().id
            );
            for _ in 0..app.rows.len() + 2 {
                app.move_selection(1);
            }
        }
    }

    #[test]
    fn stepper_control_writes_settings_atomically() {
        let (mut app, path) = sandbox(r#"{"barEngine":"bar","bar":{"thickness":48}}"#);
        app.page = app.pages.iter().position(|p| p.id == "d_style").unwrap();
        app.refresh_page_state();
        let idx = app
            .controls
            .iter()
            .position(|c| c.path == ["bar", "thickness"])
            .unwrap();
        let before = app.settings.get_f64(&["bar", "thickness"]).unwrap();
        app.adjust_control(idx, 1);
        let after = app.settings.get_f64(&["bar", "thickness"]).unwrap();
        assert!(after > before);
        let disk: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(disk["bar"]["thickness"].as_f64().unwrap(), after);
    }

    #[test]
    fn engine_switch_seeds_classicbar_and_filters_pages() {
        let (mut app, _) = sandbox(
            r#"{"barEngine":"bar","bar":{"position":"bottom","zones":[{"id":"start","align":"start","modules":[{"id":"help","enabled":true}]}]}}"#,
        );
        app.run_action(Action::SetEngine, Some("classic".into()));
        assert_eq!(
            app.settings.get_str(&["barEngine"]).as_deref(),
            Some("classic")
        );
        assert_eq!(
            app.settings.get_str(&["classicbar", "position"]).as_deref(),
            Some("bottom")
        );
        assert!(
            app.settings
                .get(&["classicbar", "modules", "left"])
                .is_some()
        );
        let visible = app.visible_page_indices();
        assert!(visible.iter().any(|&i| app.pages[i].id == "d_classic"));
        assert!(!visible.iter().any(|&i| app.pages[i].id == "d_style"));
        app.next_page(1);
        app.next_page(-1);
    }

    #[test]
    fn zones_add_move_and_delete() {
        let (mut app, _) = sandbox(
            r#"{"bar": {"zones":[{"id":"start","align":"start","modules":[{"id":"help","enabled":true},{"id":"search","enabled":true}]},{"id":"end","align":"end","modules":[]}]}}"#,
        );
        app.page = app.pages.iter().position(|p| p.id == "d_zones").unwrap();
        app.refresh_page_state();
        app.zone_add();
        assert_eq!(app.zones().len(), 3);
        app.zone_move_module(0, 0, 1);
        let zones = app.zones();
        assert_eq!(zones[0]["modules"][0]["id"], "search");
        assert_eq!(zones[0]["modules"][1]["id"], "help");
        app.zone_move_module(0, 1, 1);
        let zones = app.zones();
        assert_eq!(zones[0]["modules"].as_array().unwrap().len(), 1);
        assert_eq!(zones[1]["modules"][0]["id"], "help");
        app.zone_delete(0);
        assert_eq!(app.zones().len(), 2);
    }

    #[test]
    fn keybind_add_delete_and_classic_group_ops() {
        let (mut app, _) = sandbox(
            r#"{"keybinds":[],"classicbar":{"modules":{"left":["help","search"],"center":[],"right":[],"available":[]}}}"#,
        );
        app.page = app.pages.iter().position(|p| p.id == "s_keyboard").unwrap();
        app.refresh_page_state();
        app.keybind_add();
        assert_eq!(app.keybinds.len(), 1);
        app.commit_edit(EditTarget::Keybind { kb: 0, field: 2 }, "T".into());
        assert_eq!(app.keybinds[0]["key"], "T");
        app.keybind_delete(0);
        assert!(app.keybinds.is_empty());

        app.page = app.pages.iter().position(|p| p.id == "d_classic").unwrap();
        app.refresh_page_state();
        app.classic_join(0, 0);
        let left = app.classic_items(0);
        assert_eq!(left.len(), 1);
        assert!(crate::classic::is_group(&left[0]).is_some());
        app.classic_ungroup(0, 0);
        let left = app.classic_items(0);
        assert_eq!(left.len(), 2);
    }
}
