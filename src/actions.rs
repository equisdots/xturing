//! External command helpers.
//!
//! Everything that would normally touch the system (scripts, hyprctl,
//! qs ipc, notify-send) goes through here. With `XTURING_DRY=1` commands are
//! only logged to `/tmp/xturing-actions.log`, which makes the TUI safe to
//! test outside a Hyprland session.

use std::fs::OpenOptions;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};

static FORCE_DRY: AtomicBool = AtomicBool::new(false);

/// Force dry-run (tests and the `--dry-run` flag): never runs commands.
pub fn set_dry(v: bool) {
    FORCE_DRY.store(v, Ordering::Relaxed);
}

pub fn scripts_dir() -> String {
    format!("{}/.config/hypr/scripts", crate::settings::home().display())
}

pub fn script(name: &str) -> String {
    format!("{}/{}", scripts_dir(), name)
}

pub fn quickshell_dir() -> String {
    format!(
        "{}/.config/hypr/scripts/quickshell",
        crate::settings::home().display()
    )
}

fn dry_run() -> bool {
    FORCE_DRY.load(Ordering::Relaxed)
        || std::env::var("XTURING_DRY")
            .map(|v| v == "1")
            .unwrap_or(false)
}

pub fn log(cmd: &str) {
    let line = format!("{}\n", cmd);
    if let Ok(mut f) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/xturing-actions.log")
    {
        let _ = f.write_all(line.as_bytes());
    }
}

/// Fire-and-forget shell command (`Quickshell.execDetached` equivalent).
pub fn spawn(cmd: &str) {
    log(cmd);
    if dry_run() {
        return;
    }
    let _ = Command::new("bash")
        .arg("-c")
        .arg(cmd)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

/// Run and capture stdout (for reads: hyprctl, envycontrol, timex status...).
pub fn capture(cmd: &str) -> String {
    log(cmd);
    if dry_run() {
        return String::new();
    }
    match Command::new("bash").arg("-c").arg(cmd).output() {
        Ok(out) => String::from_utf8_lossy(&out.stdout).into_owned(),
        Err(_) => String::new(),
    }
}

pub fn notify(summary: &str, body: &str) {
    let escaped = body.replace('\'', "'\\''");
    spawn(&format!("notify-send -t 1500 '{}' '{}'", summary, escaped));
}
