//! settings.json model: load, watch (mtime), nested edit, atomic write.
//!
//! Mirrors the Quickshell contract: the file is the single source of truth,
//! every write replaces the whole file atomically (`tmp + mv`), keeping
//! unknown keys untouched and key order stable (serde_json preserve_order).

use anyhow::{Context, Result};
use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

pub fn default_settings_path() -> PathBuf {
    if let Ok(p) = std::env::var("XTURING_SETTINGS") {
        return PathBuf::from(p);
    }
    home().join(".config/hypr/settings.json")
}

pub struct Settings {
    pub path: PathBuf,
    pub data: Map<String, Value>,
    mtime: Option<SystemTime>,
}

impl Settings {
    pub fn load() -> Result<Self> {
        Self::load_from(default_settings_path())
    }

    pub fn load_from(path: PathBuf) -> Result<Self> {
        let data = read_json_object(&path)?;
        let mtime = fs::metadata(&path).ok().and_then(|m| m.modified().ok());
        Ok(Self { path, data, mtime })
    }

    /// Re-read the file when it changed on disk (the shell also writes it).
    /// Returns true when new content was loaded.
    pub fn reload_if_changed(&mut self) -> Result<bool> {
        let mtime = fs::metadata(&self.path)
            .ok()
            .and_then(|m| m.modified().ok());
        if mtime == self.mtime {
            return Ok(false);
        }
        self.data = read_json_object(&self.path)?;
        self.mtime = mtime;
        Ok(true)
    }

    pub fn reload(&mut self) -> Result<()> {
        self.data = read_json_object(&self.path)?;
        self.mtime = fs::metadata(&self.path)
            .ok()
            .and_then(|m| m.modified().ok());
        Ok(())
    }

    pub fn get(&self, path: &[&str]) -> Option<&Value> {
        get_at(&self.data, path)
    }

    pub fn get_str(&self, path: &[&str]) -> Option<String> {
        self.get(path).and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            _ => None,
        })
    }

    pub fn get_bool(&self, path: &[&str]) -> Option<bool> {
        self.get(path).and_then(Value::as_bool)
    }

    pub fn get_f64(&self, path: &[&str]) -> Option<f64> {
        self.get(path).and_then(|v| {
            v.as_f64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
    }

    /// Set a nested path and persist atomically. Returns true when it changed.
    pub fn set(&mut self, path: &[&str], value: Value) -> Result<bool> {
        if path.is_empty() {
            return Ok(false);
        }
        let before = self.get(path).cloned();
        if before.as_ref() == Some(&value) {
            return Ok(false);
        }
        set_at(&mut self.data, path, value);
        self.write()?;
        Ok(true)
    }

    pub fn write(&mut self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).ok();
        }
        let json = serde_json::to_string_pretty(&Value::Object(self.data.clone()))
            .context("serialize settings.json")?;
        write_atomic(&self.path, json.as_bytes())?;
        self.mtime = fs::metadata(&self.path)
            .ok()
            .and_then(|m| m.modified().ok());
        Ok(())
    }
}

pub fn read_json_object(path: &Path) -> Result<Map<String, Value>> {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Map::new()),
        Err(e) => return Err(e).with_context(|| format!("read {}", path.display())),
    };
    let text = text.trim();
    if text.is_empty() {
        return Ok(Map::new());
    }
    match serde_json::from_str::<Value>(text) {
        Ok(Value::Object(m)) => Ok(m),
        Ok(_) => Ok(Map::new()),
        Err(e) => Err(e).with_context(|| format!("parse {}", path.display())),
    }
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let dir = path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(dir).ok();
    let base = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("settings.json");
    let tmp = dir.join(format!(".{}.tmp.{}", base, std::process::id()));
    fs::write(&tmp, bytes).with_context(|| format!("write {}", tmp.display()))?;
    fs::rename(&tmp, path).with_context(|| format!("rename into {}", path.display()))?;
    Ok(())
}

pub fn get_at<'a>(data: &'a Map<String, Value>, path: &[&str]) -> Option<&'a Value> {
    let (first, rest) = path.split_first()?;
    let mut cur = data.get(*first)?;
    for key in rest {
        cur = cur.as_object()?.get(*key)?;
    }
    Some(cur)
}

pub fn set_at(data: &mut Map<String, Value>, path: &[&str], value: Value) {
    fn rec(obj: &mut Map<String, Value>, path: &[&str], value: Value) {
        let (first, rest) = path.split_first().unwrap();
        if rest.is_empty() {
            obj.insert((*first).to_string(), value);
            return;
        }
        let entry = obj
            .entry((*first).to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        if !entry.is_object() {
            *entry = Value::Object(Map::new());
        }
        rec(entry.as_object_mut().unwrap(), rest, value);
    }
    rec(data, path, value);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tmp_dir() -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "xturing-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn nested_set_preserves_unknown_keys_and_order() {
        let dir = tmp_dir();
        let path = dir.join("settings.json");
        fs::write(
            &path,
            r#"{
  "bar": { "thickness": 48, "unknownKey": {"a": 1}, "modules": {"time": {"size": 0}} },
  "zzz": 1,
  "aaa": 2
}"#,
        )
        .unwrap();
        let mut s = Settings::load_from(path.clone()).unwrap();
        s.set(&["bar", "thickness"], json!(56)).unwrap();
        s.set(&["bar", "modules", "time", "size"], json!(18))
            .unwrap();
        let text = fs::read_to_string(&path).unwrap();
        let v: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["bar"]["thickness"], json!(56));
        assert_eq!(v["bar"]["modules"]["time"]["size"], json!(18));
        assert_eq!(v["bar"]["unknownKey"]["a"], json!(1));
        let keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(keys, vec!["bar", "zzz", "aaa"]);
        // No leftover temp files
        let leftovers: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp."))
            .collect();
        assert!(leftovers.is_empty());
    }

    #[test]
    fn missing_file_is_empty_object() {
        let dir = tmp_dir();
        let s = Settings::load_from(dir.join("nope.json")).unwrap();
        assert!(s.data.is_empty());
        assert_eq!(s.get(&["bar"]), None);
    }

    #[test]
    fn set_is_noop_when_value_equal() {
        let dir = tmp_dir();
        let path = dir.join("settings.json");
        fs::write(&path, r#"{"uiScale": 1.0}"#).unwrap();
        let mut s = Settings::load_from(path.clone()).unwrap();
        let changed = s.set(&["uiScale"], json!(1.0)).unwrap();
        assert!(!changed);
        let changed = s.set(&["uiScale"], json!(1.25)).unwrap();
        assert!(changed);
        assert_eq!(s.get_f64(&["uiScale"]), Some(1.25));
    }

    #[test]
    fn reload_if_changed_picks_up_external_write() {
        let dir = tmp_dir();
        let path = dir.join("settings.json");
        fs::write(&path, r#"{"barEngine": "bar"}"#).unwrap();
        let mut s = Settings::load_from(path.clone()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        fs::write(&path, r#"{"barEngine": "classic"}"#).unwrap();
        let changed = s.reload_if_changed().unwrap();
        assert!(changed);
        assert_eq!(s.get_str(&["barEngine"]).as_deref(), Some("classic"));
    }
}
