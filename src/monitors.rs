//! Monitors: read `hyprctl -j monitors`, edit in memory, apply like the
//! MonitorsTab (settings.monitors + display-config + hyprctl eval per output).

use crate::actions;
use crate::settings::home;
use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::fs;

#[derive(Clone)]
pub struct Mode {
    pub label: String,
    pub w: u32,
    pub h: u32,
    pub rate: f64,
}

#[derive(Clone)]
pub struct Monitor {
    pub name: String,
    pub description: String,
    pub w: u32,
    pub h: u32,
    pub rate: f64,
    pub x: i64,
    pub y: i64,
    pub scale: f64,
    pub transform: u32,
    pub vrr: u32,
    pub disabled: bool,
    pub mirror: String,
    pub cm: String,
    pub bitdepth: u32,
    pub modes: Vec<Mode>,
}

pub fn read() -> Result<Vec<Monitor>> {
    let text = actions::capture("hyprctl -j monitors");
    let data: Vec<Value> = serde_json::from_str(&text).context("parse hyprctl monitors")?;
    let mut out = Vec::new();
    for m in data {
        let modes = m
            .get("availableModes")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .filter_map(parse_mode)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let bitdepth = if m
            .get("currentFormat")
            .and_then(Value::as_str)
            .map(|s| s.contains("2101010"))
            .unwrap_or(false)
        {
            10
        } else {
            8
        };
        out.push(Monitor {
            name: m
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("?")
                .to_string(),
            description: m
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            w: m.get("width").and_then(Value::as_u64).unwrap_or(0) as u32,
            h: m.get("height").and_then(Value::as_u64).unwrap_or(0) as u32,
            rate: m.get("refreshRate").and_then(Value::as_f64).unwrap_or(0.0),
            x: m.get("x").and_then(Value::as_i64).unwrap_or(0),
            y: m.get("y").and_then(Value::as_i64).unwrap_or(0),
            scale: m.get("scale").and_then(Value::as_f64).unwrap_or(1.0),
            transform: m.get("transform").and_then(Value::as_u64).unwrap_or(0) as u32,
            vrr: if m.get("vrr").and_then(Value::as_bool).unwrap_or(false) {
                1
            } else {
                0
            },
            disabled: m.get("disabled").and_then(Value::as_bool).unwrap_or(false),
            mirror: m
                .get("mirrorOf")
                .and_then(Value::as_str)
                .unwrap_or("none")
                .to_string(),
            cm: m
                .get("colorManagementPreset")
                .and_then(Value::as_str)
                .unwrap_or("auto")
                .to_string(),
            bitdepth,
            modes,
        });
    }
    Ok(out)
}

fn parse_mode(s: &str) -> Option<Mode> {
    let (res, rest) = s.split_once('@')?;
    let (w, h) = res.split_once('x')?;
    let rate: f64 = rest.trim_end_matches("Hz").parse().ok()?;
    let w: u32 = w.parse().ok()?;
    let h: u32 = h.parse().ok()?;
    Some(Mode {
        label: format!("{}x{}@{}", w, h, rate.round() as u32),
        w,
        h,
        rate,
    })
}

impl Monitor {
    pub fn output(&self) -> String {
        if self.description.is_empty() {
            self.name.clone()
        } else {
            format!("desc:{}", self.description)
        }
    }

    fn lua(&self, multi: bool, target: Option<&str>) -> String {
        if self.disabled {
            return format!(
                "hl.monitor({{ output = \"{}\", disabled = true }})",
                self.output()
            );
        }
        let rate = self
            .modes
            .iter()
            .filter(|m| m.w == self.w && m.h == self.h)
            .min_by(|a, b| {
                (a.rate - self.rate)
                    .abs()
                    .total_cmp(&(b.rate - self.rate).abs())
            })
            .map(|m| m.rate)
            .unwrap_or(self.rate);
        let pos = if multi {
            format!("{}x{}", self.x, self.y)
        } else {
            "0x0".to_string()
        };
        let mut parts = vec![
            format!("output = \"{}\"", self.output()),
            format!("mode = \"{}x{}@{:.2}\"", self.w, self.h, rate),
            format!("position = \"{}\"", pos),
        ];
        if (self.scale - 1.0).abs() > 0.001 {
            parts.push(format!("scale = {}", self.scale));
        }
        if self.transform != 0 {
            parts.push(format!("transform = {}", self.transform));
        }
        if self.vrr != 0 {
            parts.push(format!("vrr = {}", self.vrr));
        }
        if self.bitdepth != 8 {
            parts.push(format!("bitdepth = {}", self.bitdepth));
        }
        if self.cm != "auto" {
            parts.push(format!("cm = \"{}\"", self.cm));
        }
        if let Some(t) = target {
            parts.push(format!("mirror = \"desc:{}\"", t));
        }
        format!("hl.monitor({{ {} }})", parts.join(", "))
    }

    fn config_line(&self) -> String {
        let mode = self
            .modes
            .iter()
            .filter(|m| m.w == self.w && m.h == self.h)
            .min_by(|a, b| {
                (a.rate - self.rate)
                    .abs()
                    .total_cmp(&(b.rate - self.rate).abs())
            })
            .map(|m| m.rate)
            .unwrap_or(self.rate);
        format!(
            "{}|{}|{}|{}|{}x{}@{:.2}|{}|{}|{}|{}|{}|{}",
            self.output(),
            self.x,
            self.y,
            self.scale,
            self.w,
            self.h,
            mode,
            self.transform,
            self.vrr,
            self.bitdepth,
            self.cm,
            if self.mirror.is_empty() {
                "none"
            } else {
                &self.mirror
            },
            self.disabled
        )
    }
}

/// Returns the settings.monitors array to persist; performs the config file
/// write and hyprctl eval side effects.
pub fn apply(monitors: &mut [Monitor]) -> Result<Vec<Value>> {
    let multi = monitors.len() > 1;
    if multi {
        let min_x = monitors.iter().map(|m| m.x).min().unwrap_or(0);
        let min_y = monitors.iter().map(|m| m.y).min().unwrap_or(0);
        for m in monitors.iter_mut() {
            m.x -= min_x;
            m.y -= min_y;
        }
    }

    let mut settings_arr = Vec::new();
    let mut lines = Vec::new();
    let mut lua = Vec::new();
    for m in monitors.iter() {
        let target = if m.mirror.is_empty() || m.mirror == "none" {
            None
        } else {
            monitors
                .iter()
                .find(|o| o.name == m.mirror)
                .map(|o| o.description.clone())
        };
        lua.push(m.lua(multi, target.as_deref()));
        settings_arr.push(json!({
            "name": m.name,
            "resW": m.w,
            "resH": m.h,
            "rate": m.rate.round() as i64,
            "x": if multi { m.x } else { 0 },
            "y": if multi { m.y } else { 0 },
            "scale": m.scale,
            "transform": m.transform,
            "vrr": m.vrr,
            "disabled": m.disabled,
            "mirrorOf": m.mirror,
            "cm": m.cm,
            "bitdepth": m.bitdepth,
        }));
        lines.push(m.config_line());
    }

    let hypr = home().join(".config/hypr");
    let config_path = hypr.join("display-config");
    let text = format!("{}\n", lines.join("\n"));
    crate::settings::write_atomic(&config_path, text.as_bytes())?;
    let _ = fs::copy(&config_path, hypr.join("display-config.bak"));

    let eval = lua
        .iter()
        .map(|l| format!("hyprctl eval '{}'", l.replace('\'', "'\\''")))
        .collect::<Vec<_>>()
        .join("; ");
    actions::spawn(&format!(
        "{}; pkill xwww-daemon 2>/dev/null || true; xwww-daemon &",
        eval
    ));
    Ok(settings_arr)
}

pub fn reset() {
    actions::spawn(
        "rm -f ~/.config/hypr/display-config ~/.config/hypr/display-config.bak && hyprctl reload && hyprctl eval 'hl.monitor({ output = \"\", mode = \"highrr\", position = \"auto\", scale = \"auto\" })'",
    );
}
