//! ClassicBar model helpers (port of the BarLayout.js pieces the editor uses).

use serde_json::{json, Map, Value};

pub const SECTIONS: [&str; 4] = ["left", "center", "right", "available"];

pub fn defaults() -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("position".into(), json!("top"));
    m.insert("style".into(), json!("modular"));
    m.insert("widthPercent".into(), json!(100));
    m.insert("autohide".into(), json!(false));
    m.insert("autohideTimeout".into(), json!(800));
    m.insert("distinctPills".into(), json!(false));
    m.insert("roundness".into(), json!(0.6));
    m.insert("timeFormat".into(), json!("HH:mm:ss"));
    m.insert("thickness".into(), json!(40));
    m.insert("opacity".into(), json!(100));
    m.insert(
        "modules".into(),
        json!({
            "left": ["help", "search", "settings", "media"],
            "center": [["time", "date", "weather"]],
            "right": [["keyboard", "wifi", "bluetooth", "volume", "battery"], "tray", "update"],
            "available": []
        }),
    );
    m
}

pub const CATALOG_IDS: &[&str] = &[
    "help", "search", "settings", "update", "time", "date", "media", "workspaces", "tray",
    "keyboard", "wifi", "bluetooth", "sysmon", "volume", "battery", "recording", "weather", "focus",
];

/// Port of `barToClassicModules`: enabled modules per zone, align -> section,
/// center groups of 3+ collapse into a single pill.
pub fn bar_to_classic(bar: &Map<String, Value>) -> Value {
    let mut out: Map<String, Value> = Map::new();
    for section in SECTIONS {
        out.insert(section.into(), json!([]));
    }
    let Some(zones) = bar.get("zones").and_then(Value::as_array) else {
        return Value::Object(out);
    };
    let mut seen: Vec<String> = Vec::new();
    for zone in zones {
        let align = zone.get("align").and_then(Value::as_str).unwrap_or("start");
        let target = match align {
            "center" => "center",
            "end" => "right",
            _ => "left",
        };
        let mut enabled: Vec<String> = Vec::new();
        if let Some(mods) = zone.get("modules").and_then(Value::as_array) {
            for m in mods {
                let enabled_flag = m.get("enabled").and_then(Value::as_bool).unwrap_or(false);
                let Some(id) = m.get("id").and_then(Value::as_str) else { continue };
                if enabled_flag && !seen.contains(&id.to_string()) {
                    seen.push(id.to_string());
                    enabled.push(id.to_string());
                }
            }
        }
        if enabled.is_empty() {
            continue;
        }
        let collapse = target == "center" && enabled.len() >= 3;
        let entry = if collapse {
            json!([enabled])
        } else {
            Value::Array(enabled.into_iter().map(Value::String).collect())
        };
        let arr = out.get_mut(target).unwrap().as_array_mut().unwrap();
        if collapse {
            if let Value::Array(inner) = entry {
                for item in inner {
                    if let Value::Array(group) = item {
                        arr.push(Value::Array(group));
                    }
                }
            }
        } else if let Value::Array(items) = entry {
            for item in items {
                arr.push(item);
            }
        }
    }
    Value::Object(out)
}

/// Light normalize: keep known modules, dedupe across sections, flatten
/// 1-member groups, drop empty groups. Unknown ids are discarded.
pub fn normalize_modules(modules: &mut Value) {
    let obj = match modules.as_object_mut() {
        Some(o) => o,
        None => {
            *modules = defaults().remove("modules").unwrap();
            return;
        }
    };
    let mut seen: Vec<String> = Vec::new();
    for section in SECTIONS {
        let entry = obj.entry(section.to_string()).or_insert_with(|| json!([]));
        let arr = match entry.as_array_mut() {
            Some(a) => a,
            None => {
                *entry = json!([]);
                continue;
            }
        };
        let mut out: Vec<Value> = Vec::new();
        for item in arr.drain(..) {
            match item {
                Value::String(id) => {
                    if CATALOG_IDS.contains(&id.as_str()) && !seen.contains(&id) {
                        seen.push(id.clone());
                        out.push(Value::String(id));
                    }
                }
                Value::Array(group) => {
                    let mut members: Vec<Value> = Vec::new();
                    for m in group {
                        if let Value::String(id) = m
                            && CATALOG_IDS.contains(&id.as_str()) && !seen.contains(&id) {
                                seen.push(id.clone());
                                members.push(Value::String(id));
                            }
                    }
                    if members.len() >= 2 {
                        out.push(Value::Array(members));
                    } else if members.len() == 1 {
                        out.push(members.pop().unwrap());
                    }
                }
                _ => {}
            }
        }
        *arr = out;
    }
}

pub fn is_group(item: &Value) -> Option<Vec<String>> {
    item.as_array().map(|a| {
        a.iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect()
    })
}

pub fn item_label(item: &Value) -> String {
    match item {
        Value::String(s) => s.clone(),
        Value::Array(a) => {
            let parts: Vec<&str> = a.iter().filter_map(Value::as_str).collect();
            format!("[{}]", parts.join("+"))
        }
        _ => "?".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bar_zones_map_to_classic_sections() {
        let bar = serde_json::json!({
            "zones": [
                {"id": "start", "align": "start", "modules": [
                    {"id": "help", "enabled": true},
                    {"id": "search", "enabled": false}
                ]},
                {"id": "center", "align": "center", "modules": [
                    {"id": "time", "enabled": true},
                    {"id": "workspaces", "enabled": true}
                ]},
                {"id": "end", "align": "end", "modules": [
                    {"id": "volume", "enabled": true},
                    {"id": "battery", "enabled": true}
                ]}
            ]
        });
        let modules = bar_to_classic(bar.as_object().unwrap());
        assert_eq!(modules["left"], serde_json::json!(["help"]));
        assert_eq!(modules["right"], serde_json::json!(["volume", "battery"]));
        // center with fewer than 3 enabled modules stays loose
        assert_eq!(modules["center"], serde_json::json!(["time", "workspaces"]));
    }

    #[test]
    fn center_group_of_three_collapses() {
        let bar = serde_json::json!({
            "zones": [{"id": "center", "align": "center", "modules": [
                {"id": "time", "enabled": true},
                {"id": "date", "enabled": true},
                {"id": "weather", "enabled": true}
            ]}]
        });
        let modules = bar_to_classic(bar.as_object().unwrap());
        assert_eq!(
            modules["center"],
            serde_json::json!([["time", "date", "weather"]])
        );
    }

    #[test]
    fn normalize_dedupes_and_flattens() {
        let mut modules = serde_json::json!({
            "left": ["help", "help", ["time"], "unknown"],
            "center": ["time", ["date", "weather"]],
            "right": [],
            "available": []
        });
        normalize_modules(&mut modules);
        // first claim wins (a 1-member group flattens in place), duplicates drop
        assert_eq!(modules["left"], serde_json::json!(["help", "time"]));
        assert_eq!(modules["center"], serde_json::json!([["date", "weather"]]));
    }
}
