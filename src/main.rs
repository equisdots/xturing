//! xturing — ratatui front-end for the equisdots BarEditor settings.
//!
//! The name: X (execute/command) + Turing — driving the system from the
//! terminal by running the same instructions as the panel.

#![recursion_limit = "256"]

mod actions;
mod app;
mod catalog;
mod classic;
mod monitors;
mod palette;
mod settings;
mod ui;

use anyhow::Result;
use app::{App, EditTarget, Mode};
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::SetTitle;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::time::Duration;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Default)]
struct Options {
    settings: Option<PathBuf>,
    palettes: Option<PathBuf>,
    dry: bool,
    page: Option<String>,
    search: Option<String>,
}

fn main() -> Result<()> {
    let options = parse_args();
    if !std::io::stdout().is_terminal() {
        eprintln!("xturing: an interactive terminal is required (do not pipe it).");
        std::process::exit(1);
    }
    if options.dry {
        actions::set_dry(true);
    }
    if let Some(dir) = options.palettes.clone() {
        palette::set_palettes_dir(dir);
    }
    let settings = match &options.settings {
        Some(path) => settings::Settings::load_from(path.clone())?,
        None => settings::Settings::load()?,
    };
    let mut app = App::with_settings(settings)?;
    if let Some(page) = &options.page
        && !app.goto_page_id(page)
    {
        eprintln!("xturing: unknown or hidden page '{}'", page);
    }
    if let Some(query) = &options.search {
        app.open_search_with(query);
    }
    let mut terminal = ratatui::init();
    let _ = execute!(
        std::io::stdout(),
        EnableMouseCapture,
        SetTitle("xturing — equisdots settings")
    );
    let result = run(&mut terminal, &mut app);
    let _ = execute!(std::io::stdout(), DisableMouseCapture);
    app.persist_hypr();
    ratatui::restore();
    result
}

fn parse_args() -> Options {
    let mut opts = Options::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            "-V" | "--version" => {
                println!("xturing {}", VERSION);
                std::process::exit(0);
            }
            "--dry-run" => opts.dry = true,
            "--settings" | "--palettes" | "--page" | "--search" => {
                let Some(value) = args.next() else {
                    eprintln!("xturing: {} requires a value", arg);
                    std::process::exit(2);
                };
                match arg.as_str() {
                    "--settings" => opts.settings = Some(PathBuf::from(value)),
                    "--palettes" => opts.palettes = Some(PathBuf::from(value)),
                    "--page" => opts.page = Some(value),
                    _ => opts.search = Some(value),
                }
            }
            other => {
                eprintln!("xturing: unknown option '{}'\n", other);
                print_help();
                std::process::exit(2);
            }
        }
    }
    opts
}

fn print_help() {
    println!(
        "xturing {VERSION} — terminal UI for the equisdots settings panel

Usage: xturing [options]

Options:
  --settings <path>   settings.json to edit (default ~/.config/hypr/settings.json)
  --palettes <dir>    palettes directory (default .../quickshell/dock/palettes)
  --page <id>         open a page directly (e.g. d_style, s_general)
  --search <query>    open the search palette pre-filled
  --dry-run           do not run external commands (logs to /tmp/xturing-actions.log)
  -h, --help          show this help
  -V, --version       show the version

Environment:
  XTURING_SETTINGS      alternative settings.json path
  XTURING_PALETTES_DIR  alternative palettes directory
  XTURING_DRY=1         same as --dry-run

Keys:
  ↑↓ move (wraps across pages)   ←→ adjust values   Enter select/edit
  / search   Tab page   1-6 group   r reload   ? help   q quit"
    );
}

fn run(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui::render(f, app))?;
        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => handle_key(app, key),
                Event::Mouse(mouse) => handle_mouse(app, mouse),
                _ => {}
            }
        }
        app.tick();
        if app.quit {
            return Ok(());
        }
    }
}

fn handle_key(app: &mut App, key: KeyEvent) {
    if matches!(app.mode, Mode::Edit { .. }) {
        handle_edit(app, key);
    } else if matches!(app.mode, Mode::Chooser { .. }) {
        handle_chooser(app, key);
    } else if matches!(app.mode, Mode::Confirm { .. }) {
        handle_confirm(app, key);
    } else if matches!(app.mode, Mode::Form { .. }) {
        handle_form(app, key);
    } else if matches!(app.mode, Mode::Record { .. }) {
        handle_record(app, key);
    } else if matches!(app.mode, Mode::Search(_)) {
        handle_search(app, key);
    } else if matches!(app.mode, Mode::Help) {
        app.mode = Mode::Browse;
    } else {
        handle_browse(app, key);
    }
}

fn handle_browse(app: &mut App, key: KeyEvent) {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    match key.code {
        KeyCode::Char('q') => app.quit = true,
        KeyCode::Esc => app.quit = true,
        KeyCode::Char('c') if ctrl => app.quit = true,
        KeyCode::Char('p') if ctrl => app.open_search(),
        KeyCode::Char('/') => app.open_search(),
        KeyCode::Char('?') => app.mode = Mode::Help,
        KeyCode::Tab => app.next_page(1),
        KeyCode::BackTab => app.next_page(-1),
        KeyCode::Char(c) if ('1'..='6').contains(&c) => {
            let group = match c {
                '1' => catalog::Group::Shell,
                '2' => catalog::Group::Bar,
                '3' => catalog::Group::Theme,
                '4' => catalog::Group::Behavior,
                '5' => catalog::Group::Widgets,
                _ => catalog::Group::System,
            };
            app.goto_group(group);
        }
        KeyCode::Up | KeyCode::Char('k') => app.move_selection(-1),
        KeyCode::Down | KeyCode::Char('j') => app.move_selection(1),
        KeyCode::PageUp => app.move_selection(-10),
        KeyCode::PageDown => app.move_selection(10),
        KeyCode::Left if ctrl => app.next_page(-1),
        KeyCode::Right if ctrl => app.next_page(1),
        KeyCode::Left | KeyCode::Char('h') => {
            if shift && app.selected_is_zone_module() {
                app.move_zone_module(-1);
            } else {
                app.adjust_row(-1);
            }
        }
        KeyCode::Right | KeyCode::Char('l') => {
            if shift && app.selected_is_zone_module() {
                app.move_zone_module(1);
            } else {
                app.adjust_row(1);
            }
        }
        KeyCode::Char('[') => app.next_page(-1),
        KeyCode::Char(']') => app.next_page(1),
        KeyCode::Enter | KeyCode::Char(' ') => app.activate_row(),
        KeyCode::Char('r') => app.hard_reload(),
        KeyCode::Char('a') => app.add_selected_list(),
        KeyCode::Char('d') => app.delete_selected(),
        KeyCode::Char('w') => app.save_selected_list(),
        KeyCode::Char('m') => app.classic_action_move_section(),
        KeyCode::Char('g') => app.classic_action_join(),
        KeyCode::Char('u') => app.classic_action_ungroup(),
        _ => {}
    }
}

enum EditAction {
    None,
    Cancel,
    Commit(EditTarget, String),
}

fn handle_edit(app: &mut App, key: KeyEvent) {
    let mut action = EditAction::None;
    if let Mode::Edit {
        target,
        buf,
        cursor,
    } = &mut app.mode
    {
        match key.code {
            KeyCode::Esc => action = EditAction::Cancel,
            KeyCode::Enter => action = EditAction::Commit(target.clone(), buf.clone()),
            KeyCode::Backspace => {
                if *cursor > 0
                    && let Some((byte, _)) = buf.char_indices().nth(*cursor - 1)
                {
                    buf.remove(byte);
                    *cursor -= 1;
                }
            }
            KeyCode::Delete => {
                if let Some((byte, _)) = buf.char_indices().nth(*cursor) {
                    buf.remove(byte);
                }
            }
            KeyCode::Left => *cursor = cursor.saturating_sub(1),
            KeyCode::Right => {
                if *cursor < buf.chars().count() {
                    *cursor += 1;
                }
            }
            KeyCode::Home => *cursor = 0,
            KeyCode::End => *cursor = buf.chars().count(),
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                let byte = buf
                    .char_indices()
                    .nth(*cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(buf.len());
                buf.insert(byte, c);
                *cursor += 1;
            }
            _ => {}
        }
    }
    match action {
        EditAction::None => {}
        EditAction::Cancel => app.mode = Mode::Browse,
        EditAction::Commit(target, buf) => {
            app.mode = Mode::Browse;
            app.commit_edit(target, buf);
        }
    }
}

fn handle_search(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.search_cancel(),
        KeyCode::Enter => app.search_accept(),
        KeyCode::Up => app.search_move(-1),
        KeyCode::Down => app.search_move(1),
        KeyCode::PageUp => app.search_move(-8),
        KeyCode::PageDown => app.search_move(8),
        KeyCode::Backspace => app.search_backspace(),
        KeyCode::Char(c) => {
            if !key.modifiers.contains(KeyModifiers::CONTROL) {
                app.search_type_char(c);
            } else if c == 'p' || c == 'c' {
                app.search_cancel();
            }
        }
        _ => {}
    }
}

fn handle_chooser(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.mode = Mode::Browse,
        KeyCode::Enter => app.chooser_accept(),
        KeyCode::Up | KeyCode::Char('k') => app.chooser_move(-1),
        KeyCode::Down | KeyCode::Char('j') => app.chooser_move(1),
        _ => {}
    }
}

fn handle_record(app: &mut App, key: KeyEvent) {
    let kb = if let Mode::Record { kb } = &app.mode {
        *kb
    } else {
        return;
    };
    if key.code == KeyCode::Esc {
        app.mode = Mode::Form { kb, field: 1 };
        return;
    }
    let Some((mods, name)) = app::shortcut_fields(key.code, key.modifiers) else {
        return;
    };
    if let Some(bind) = app.keybinds.get_mut(kb) {
        bind["mods"] = serde_json::Value::String(mods);
        bind["key"] = serde_json::Value::String(name);
        app.keybinds_dirty = true;
    }
    app.rebuild_rows();
    app.mode = Mode::Form { kb, field: 2 };
}

fn handle_confirm(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => app.confirm_yes(),
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.mode = Mode::Browse,
        _ => {}
    }
}

fn handle_form(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.mode = Mode::Browse,
        KeyCode::Up | KeyCode::Char('k') => app.form_move(-1),
        KeyCode::Down | KeyCode::Char('j') => app.form_move(1),
        KeyCode::Enter => app.form_activate(),
        KeyCode::Char('r') => {
            if let Mode::Form { kb, .. } = &app.mode {
                app.mode = Mode::Record { kb: *kb };
            }
        }
        KeyCode::Char('w') => app.save_keybinds(),
        KeyCode::Char('a') => app.keybind_add(),
        KeyCode::Char('d') => {
            if let Mode::Form { kb, .. } = app.mode {
                app.keybind_delete(kb);
                app.mode = Mode::Browse;
            }
        }
        _ => {}
    }
}

// ───────────────────────── mouse / touch ─────────────────────────

fn handle_mouse(app: &mut App, mouse: MouseEvent) {
    match mouse.kind {
        MouseEventKind::ScrollUp => {
            if matches!(app.mode, Mode::Browse) {
                app.move_selection(-1);
            } else if matches!(app.mode, Mode::Search(_)) {
                app.search_move(-1);
            } else if matches!(app.mode, Mode::Chooser { .. }) {
                app.chooser_move(-1);
            }
        }
        MouseEventKind::ScrollDown => {
            if matches!(app.mode, Mode::Browse) {
                app.move_selection(1);
            } else if matches!(app.mode, Mode::Search(_)) {
                app.search_move(1);
            } else if matches!(app.mode, Mode::Chooser { .. }) {
                app.chooser_move(1);
            }
        }
        MouseEventKind::Down(MouseButton::Left) => {
            let hit = app
                .hits
                .iter()
                .find(|h| h.y == mouse.row && mouse.column >= h.x0 && mouse.column < h.x1)
                .map(|h| h.action.clone());
            let Some(action) = hit else { return };
            match action {
                app::HitAction::SelectRow(_) => {
                    app.apply_hit(&action);
                    if let Some(ctrl) = app.selected_stepper_control() {
                        app.drag = Some(app::DragState {
                            ctrl,
                            last_x: mouse.column,
                        });
                    }
                }
                app::HitAction::Adjust { ctrl, .. } => {
                    app.apply_hit(&action);
                    app.drag = Some(app::DragState {
                        ctrl,
                        last_x: mouse.column,
                    });
                }
                _ => app.apply_hit(&action),
            }
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if let Some((ctrl, last_x)) = app.drag.as_ref().map(|d| (d.ctrl, d.last_x)) {
                let dx = mouse.column as i32 - last_x as i32;
                if dx.abs() >= 3 {
                    let steps = dx / 3;
                    let sign = steps.signum();
                    for _ in 0..steps.abs() {
                        app.adjust_control(ctrl, sign);
                    }
                    if let Some(drag) = app.drag.as_mut() {
                        drag.last_x = (drag.last_x as i32 + steps * 3) as u16;
                    }
                }
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            app.drag = None;
        }
        _ => {}
    }
}
