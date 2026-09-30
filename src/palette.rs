//! Palette files (equisdots/palettes deployed under the shell dir):
//! index.json listing, active palette editing with backup/reset, create/delete.

use crate::actions;
use crate::settings::{home, write_atomic};
use anyhow::{Result, bail};
use serde_json::{Map, Value, json};
use std::fs;
use std::path::PathBuf;

static PALETTES_OVERRIDE: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// CLI override (`--palettes`); wins over the environment variable.
pub fn set_palettes_dir(path: PathBuf) {
    let _ = PALETTES_OVERRIDE.set(path);
}

pub fn palettes_dir() -> PathBuf {
    if let Some(dir) = PALETTES_OVERRIDE.get() {
        return dir.clone();
    }
    if let Ok(p) = std::env::var("XTURING_PALETTES_DIR") {
        return PathBuf::from(p);
    }
    home().join(".config/hypr/scripts/quickshell/dock/palettes")
}

fn backup_dir() -> PathBuf {
    home().join(".local/state/quickshell/palette_backup")
}

#[derive(Clone)]
pub struct PaletteEntry {
    pub slug: String,
    pub name: String,
    pub category: String,
    pub path: Option<String>,
}

pub fn load_index() -> Vec<PaletteEntry> {
    let path = palettes_dir().join("index.json");
    let Ok(text) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    let Ok(Value::Array(items)) = serde_json::from_str::<Value>(&text) else {
        return Vec::new();
    };
    items
        .into_iter()
        .filter_map(|v| {
            let slug = v.get("slug")?.as_str()?.to_string();
            let name = v
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or(&slug)
                .to_string();
            let category = v
                .get("category")
                .and_then(Value::as_str)
                .unwrap_or("custom")
                .to_string();
            let path = v.get("path").and_then(Value::as_str).map(str::to_string);
            Some(PaletteEntry {
                slug,
                name,
                category,
                path,
            })
        })
        .collect()
}

pub fn entry_path(entry: &PaletteEntry) -> PathBuf {
    match &entry.path {
        Some(rel) => palettes_dir().join(rel),
        None => palettes_dir().join(format!("{}.json", entry.slug)),
    }
}

fn parse_hex(s: &str) -> Option<(u8, u8, u8)> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some((r, g, b))
}

fn to_hex(rgb: (f64, f64, f64)) -> String {
    format!(
        "#{:02x}{:02x}{:02x}",
        rgb.0.round().clamp(0.0, 255.0) as u8,
        rgb.1.round().clamp(0.0, 255.0) as u8,
        rgb.2.round().clamp(0.0, 255.0) as u8
    )
}

pub fn mix(a: &str, b: &str, t: f64) -> String {
    let (Some(a), Some(b)) = (parse_hex(a), parse_hex(b)) else {
        return a.to_string();
    };
    let lerp = |x: u8, y: u8| x as f64 + (y as f64 - x as f64) * t;
    to_hex((lerp(a.0, b.0), lerp(a.1, b.1), lerp(a.2, b.2)))
}

pub fn valid_hex(s: &str) -> bool {
    parse_hex(s).is_some()
}

/// The 18 editable slots of the active palette.
#[derive(Clone, Default)]
pub struct PaletteFile {
    pub slug: String,
    pub path: PathBuf,
    pub name: String,
    /// color0..color15
    pub base16: [String; 16],
    pub background: String,
    pub foreground: String,
    pub raw: Map<String, Value>,
}

pub fn load_palette(entry: &PaletteEntry) -> Result<PaletteFile> {
    let path = entry_path(entry);
    let text = fs::read_to_string(&path)?;
    let raw: Map<String, Value> = serde_json::from_str(&text)?;
    let base16obj = raw.get("base16").and_then(Value::as_object);
    let mut base16: [String; 16] = Default::default();
    for (i, slot) in base16.iter_mut().enumerate() {
        let key = format!("color{}", i);
        *slot = base16obj
            .and_then(|o| o.get(&key))
            .and_then(Value::as_str)
            .unwrap_or("#000000")
            .to_string();
    }
    Ok(PaletteFile {
        slug: entry.slug.clone(),
        path,
        name: raw
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or(&entry.name)
            .to_string(),
        background: raw
            .get("background")
            .and_then(Value::as_str)
            .unwrap_or(&base16[0])
            .to_string(),
        foreground: raw
            .get("foreground")
            .and_then(Value::as_str)
            .unwrap_or(&base16[15])
            .to_string(),
        base16,
        raw,
    })
}

impl PaletteFile {
    pub fn slot(&self, idx: usize) -> &str {
        match idx {
            0..=15 => &self.base16[idx],
            16 => &self.background,
            _ => &self.foreground,
        }
    }

    pub fn slot_label(idx: usize) -> String {
        match idx {
            0..=15 => format!("color{}", idx),
            16 => "background".into(),
            _ => "foreground".into(),
        }
    }

    fn backup_path(&self) -> PathBuf {
        backup_dir().join(format!("{}.json", self.slug))
    }

    pub fn set_slot(&mut self, idx: usize, hex: &str) -> Result<()> {
        let hex = hex.to_lowercase();
        if !valid_hex(&hex) {
            bail!("invalid hex");
        }
        let backup = self.backup_path();
        if !backup.exists() {
            fs::create_dir_all(backup_dir()).ok();
            fs::copy(&self.path, &backup).ok();
        }
        match idx {
            0..=15 => {
                self.base16[idx] = hex.clone();
                if let Some(base16) = self.raw.get_mut("base16").and_then(Value::as_object_mut) {
                    base16.insert(format!("color{}", idx), Value::String(hex));
                }
            }
            16 => {
                self.background = hex.clone();
                self.raw.insert("background".into(), Value::String(hex));
            }
            _ => {
                self.foreground = hex.clone();
                self.raw.insert("foreground".into(), Value::String(hex));
            }
        }
        let text = serde_json::to_string_pretty(&Value::Object(self.raw.clone()))?;
        write_atomic(&self.path, format!("{}\n", text).as_bytes())?;
        Ok(())
    }

    pub fn reset(&mut self) -> Result<()> {
        let backup = self.backup_path();
        if !backup.exists() {
            bail!("no backup");
        }
        let bytes = fs::read(&backup)?;
        write_atomic(&self.path, &bytes)?;
        fs::remove_file(&backup).ok();
        Ok(())
    }
}

pub fn slugify(name: &str) -> String {
    let mut out = String::new();
    for ch in name.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if (ch.is_whitespace() || ch == '-' || ch == '_')
            && !out.ends_with('-')
            && !out.is_empty()
        {
            out.push('-');
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

/// Create user palette from 8 base colors (background, red, green, yellow,
/// blue, purple, cyan, foreground) + register in index.json.
pub fn create(name: &str, c8: &[String; 8], active: &PaletteFile) -> Result<String> {
    let slug = slugify(name);
    if slug.is_empty() {
        bail!("Enter a palette name");
    }
    let dir = palettes_dir();
    let path = dir.join(format!("{}.json", slug));
    if path.exists() {
        bail!("A palette with slug '{}' already exists", slug);
    }
    for c in c8 {
        if !valid_hex(c) {
            bail!("Invalid color in palette draft");
        }
    }
    let color8 = mix(&c8[7], &c8[0], 0.45);
    let mut base16 = Map::new();
    for (i, c) in c8.iter().enumerate() {
        base16.insert(format!("color{}", i), json!(c.to_lowercase()));
    }
    base16.insert("color8".into(), json!(color8));
    for (i, c) in c8.iter().enumerate().skip(1) {
        base16.insert(format!("color{}", i + 8), json!(c.to_lowercase()));
    }
    let obj = json!({
        "name": name,
        "slug": slug,
        "author": "xturing",
        "base16": Value::Object(base16),
        "background": c8[0],
        "foreground": c8[7],
        "roles": { "workspaceActive": mix(&c8[5], &c8[7], 0.35) }
    });
    let _ = active;
    let text = serde_json::to_string_pretty(&obj)?;
    write_atomic(&path, format!("{}\n", text).as_bytes())?;

    let index_path = dir.join("index.json");
    let text = fs::read_to_string(&index_path)?;
    let mut index: Value = serde_json::from_str(&text)?;
    if let Value::Array(items) = &mut index {
        items.push(json!({
            "slug": slug,
            "name": name,
            "category": "user",
            "colors": c8.iter().map(|c| c.to_lowercase()).collect::<Vec<_>>(),
        }));
    }
    let text = serde_json::to_string_pretty(&index)?;
    write_atomic(&index_path, format!("{}\n", text).as_bytes())?;
    Ok(slug)
}

pub fn delete(slug: &str) -> Result<()> {
    if slug == "x" {
        bail!("'x' is the fallback palette and cannot be deleted");
    }
    let dir = palettes_dir();
    let index_path = dir.join("index.json");
    let text = fs::read_to_string(&index_path)?;
    let mut index: Value = serde_json::from_str(&text)?;
    let mut removed = false;
    let mut rel: Option<String> = None;
    if let Value::Array(items) = &mut index {
        items.retain(|v| {
            let keep = v.get("slug").and_then(Value::as_str) != Some(slug);
            if !keep {
                removed = true;
                rel = v.get("path").and_then(Value::as_str).map(str::to_string);
            }
            keep
        });
    }
    if !removed {
        bail!("Palette not found in index.json");
    }
    let file = match rel {
        Some(r) => dir.join(r),
        None => dir.join(format!("{}.json", slug)),
    };
    fs::remove_file(file).ok();
    fs::remove_file(backup_dir().join(format!("{}.json", slug))).ok();
    let text = serde_json::to_string_pretty(&index)?;
    write_atomic(&index_path, format!("{}\n", text).as_bytes())?;
    Ok(())
}

pub fn sync_theme() {
    actions::spawn(
        "ENGINE=\"$HOME/.local/share/equisdots/theme-sync/theme-sync.sh\"; [ -x \"$ENGINE\" ] || ENGINE=\"$HOME/.local/bin/theme-sync\"; bash \"$ENGINE\" >/dev/null 2>&1",
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_matches_index_conventions() {
        assert_eq!(slugify("My Palette"), "my-palette");
        assert_eq!(slugify("  Hello   World!! "), "hello-world");
        assert_eq!(slugify("a-b_c"), "a-b-c");
    }

    #[test]
    fn mix_lerps_hex() {
        assert_eq!(mix("#000000", "#ffffff", 0.5), "#808080");
        assert_eq!(mix("#ff0000", "#00ff00", 1.0), "#00ff00");
    }

    #[test]
    fn valid_hex_accepts_hash_and_plain() {
        assert!(valid_hex("#aAbBcC"));
        assert!(valid_hex("aabbcc"));
        assert!(!valid_hex("#aabbc"));
        assert!(!valid_hex("zzzzzz"));
    }
}
