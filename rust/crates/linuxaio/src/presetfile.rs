//! Export / Import von allem, was Linux AIO gesetzt hat: App-Einstellungen (mit Kitty, Alacritty, Launcher,
//! Hyprland-Zuständen), Hypr-Studio-Einstellungen, GNOME-Einstellungen unserer Seiten, Mauszeiger/Symbole,
//! von Linux AIO installierte Schriften. Datei: *.linuxaio (JSON). Import: Vorschau → Sicherung → Anwenden.

use crate::{desktops, fonts, gnome, looks, pyjson, remote, usercmd};
use hyprstudio::util;
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub const FORMAT: &str = "linuxaio-preset";
pub const SKIP_CFG: [&str; 8] = ["version", "lang", "tab", "geometry", "settings_adv", "settings_de", "side_collapsed",
    "nav_collapsed"];

fn skip(k: &str) -> bool {
    SKIP_CFG.contains(&k)
}

pub fn gkeys() -> Vec<(String, String)> {
    let mut out: BTreeSet<(String, String)> = BTreeSet::new();
    for (_, specs) in gnome::SECTIONS {
        for o in specs.iter() {
            if !o.schema.is_empty() && !o.key.is_empty() {
                out.insert((o.schema.into(), o.key.into()));
            }
        }
    }
    out.into_iter().collect()
}

fn hs_cfg_path() -> PathBuf {
    usercmd::config_dir().join("LabtosoStudios").join("hyprstudio.json")
}

fn read_json(p: &Path) -> Map<String, Value> {
    std::fs::read_to_string(usercmd::readable(p)).ok().and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v.as_object().cloned()).unwrap_or_default()
}

fn filtered(m: &Map<String, Value>) -> Map<String, Value> {
    m.iter().filter(|(k, _)| !skip(k)).map(|(k, v)| (k.clone(), v.clone())).collect()
}

pub fn collect(cfg: &Map<String, Value>) -> Map<String, Value> {
    let de = desktops::current().unwrap_or_else(|| "gnome".into());
    let mut g = Map::new();
    for (schema, key) in gkeys() {
        let v = usercmd::gget(&schema, &key, Value::Null);
        if !v.is_null() {
            g.insert(format!("{} {}", schema, key), v);
        }
    }
    let f: Vec<String> = fonts::list_installed().into_iter().collect();
    json!({"format": FORMAT, "version": crate::VERSION, "created": util::strftime("%Y-%m-%d %H:%M"),
           "app": filtered(cfg), "hyprstudio": filtered(&read_json(&hs_cfg_path())), "gsettings": g,
           "cursor": looks::current(&de, "cursor"), "icons": looks::current(&de, "icon"), "fonts": f})
        .as_object().cloned().unwrap()
}

pub fn export(cfg: &Map<String, Value>, path: &Path) -> std::io::Result<PathBuf> {
    let mut p = path.to_path_buf();
    if p.extension().map(|e| e != "linuxaio").unwrap_or(true) {
        p = p.with_extension("linuxaio");
    }
    std::fs::write(&p, pyjson::dumps(&Value::Object(collect(cfg)), Some(1), false, false))?;
    Ok(p)
}

pub fn load(path: &Path) -> Result<Map<String, Value>, String> {
    let s = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let d: Value = serde_json::from_str(&s).map_err(|e| e.to_string())?;
    let Some(d) = d.as_object().cloned().filter(|o| o.get("format").and_then(|v| v.as_str()) == Some(FORMAT)) else {
        return Err("not a Linux AIO preset".into());
    };
    for (k, list) in [("app", false), ("hyprstudio", false), ("gsettings", false), ("fonts", true)] {
        if let Some(v) = d.get(k) {
            if (list && !v.is_array()) || (!list && !v.is_object()) {
                return Err(k.into());
            }
        }
    }
    Ok(d)
}

/// Lesbare Liste dessen, was der Import ändert: (Bereich, Name, alt, neu).
pub fn changes(data: &Map<String, Value>, cfg: &Map<String, Value>) -> Vec<(String, String, Value, Value)> {
    let cur = collect(cfg);
    let mut out = Vec::new();
    let allowed: BTreeSet<String> = gkeys().into_iter().map(|(s, k)| format!("{} {}", s, k)).collect();
    let empty = Map::new();
    let obj = |m: &Map<String, Value>, k: &str| m.get(k).and_then(|v| v.as_object()).cloned().unwrap_or_else(|| empty.clone());
    let cg = obj(&cur, "gsettings");
    for (k, v) in obj(data, "gsettings") {
        let old = cg.get(&k).cloned().unwrap_or(Value::Null);
        if allowed.contains(&k) && old != v {
            out.push(("GNOME".into(), k.split_once(' ').map(|x| x.1).unwrap_or(&k).to_string(), old, v));
        }
    }
    for kind in ["cursor", "icons"] {
        let new = data.get(kind).cloned().unwrap_or(Value::Null);
        if hyprstudio::jsonx::truthy(&new) && Some(&new) != cur.get(kind) {
            out.push(("Theme".into(), kind.into(), cur.get(kind).cloned().unwrap_or(Value::Null), new));
        }
    }
    let have: BTreeSet<String> = cur.get("fonts").and_then(|v| v.as_array()).map(|a| a.iter()
        .filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
    let want: BTreeSet<String> = data.get("fonts").and_then(|v| v.as_array()).map(|a| a.iter()
        .filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
    for f in want.difference(&have) {
        out.push(("Font".into(), f.clone(), Value::Null, Value::String("install".into())));
    }
    for area in ["app", "hyprstudio"] {
        let c = obj(&cur, area);
        for (k, v) in obj(data, area) {
            if !skip(&k) && c.get(&k) != Some(&v) {
                out.push((if area == "app" { "Linux AIO" } else { "Hypr Studio" }.into(), k,
                          Value::String("…".into()), Value::String("…".into())));
            }
        }
    }
    out
}

pub fn backup_current(cfg: &Map<String, Value>) -> std::io::Result<PathBuf> {
    let d = usercmd::config_dir().join("LabtosoStudios").join("backups")
        .join(util::strftime("%Y-%m-%d_%H%M%S") + "-before-import");
    std::fs::create_dir_all(&d)?;
    export(cfg, &d.join("before-import.linuxaio"))
}

#[derive(Debug, Clone, Default)]
pub struct Summary {
    pub gsettings: usize,
    pub themes: Vec<String>,
    pub fonts: Vec<String>,
    pub errors: Vec<String>,
}

/// Alles schreiben (vorher backup_current). Fehlende Themes/Schriften werden geladen.
pub fn apply(data: &Map<String, Value>, cfg: &mut Map<String, Value>, log: &dyn Fn(&str)) -> Summary {
    let mut res = Summary::default();
    let allowed: BTreeSet<String> = gkeys().into_iter().map(|(s, k)| format!("{} {}", s, k)).collect();
    if let Some(g) = data.get("gsettings").and_then(|v| v.as_object()) {
        for (k, v) in g {
            if allowed.contains(k) {
                let (schema, key) = k.split_once(' ').unwrap();
                if usercmd::gset(schema, key, v.clone()) {
                    res.gsettings += 1;
                }
            }
        }
    }
    if let Some(a) = data.get("app").and_then(|v| v.as_object()) {
        for (k, v) in a {
            if !skip(k) {
                cfg.insert(k.clone(), v.clone());
            }
        }
    }
    let mut hs = read_json(&hs_cfg_path());
    if let Some(h) = data.get("hyprstudio").and_then(|v| v.as_object()) {
        for (k, v) in h {
            if !skip(k) {
                hs.insert(k.clone(), v.clone());
            }
        }
    }
    let p = hs_cfg_path();
    let _ = std::fs::create_dir_all(p.parent().unwrap());
    let _ = std::fs::write(&p, pyjson::dumps(&Value::Object(hs), Some(1), false, false));
    let de = desktops::current().unwrap_or_else(|| "gnome".into());
    for kind in ["cursor", "icons"] {
        let name = data.get(kind).and_then(|v| v.as_str()).unwrap_or("").to_string();
        if name.is_empty() {
            continue;
        }
        let k2 = if kind == "cursor" { "cursor" } else { "icon" };
        if looks::installed_path(&name, k2).is_none() {
            if let Err(e) = download_theme(&name, k2, log) {
                res.errors.push(format!("{}: {}", name, e));
                continue;
            }
        }
        let ok = if kind == "cursor" { looks::apply_cursor(&de, &name, 24, log) } else { looks::apply_icons(&de, &name) };
        if ok {
            res.themes.push(name);
        }
    }
    let want: BTreeSet<String> = data.get("fonts").and_then(|v| v.as_array()).map(|a| a.iter()
        .filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
    let want: BTreeSet<String> = want.difference(&fonts::list_installed()).cloned().collect();
    if !want.is_empty() {
        let (catalog, _err) = fonts::catalog();
        for item in catalog {
            if want.contains(&fonts::safe_family(&item.family)) {
                match fonts::install(&item, log) {
                    Ok(_) => res.fonts.push(item.family.clone()),
                    Err(e) => res.errors.push(format!("{}: {}", item.family, e)),
                }
            }
        }
    }
    res
}

fn download_theme(name: &str, kind: &str, log: &dyn Fn(&str)) -> Result<(), remote::SourceError> {
    let man = remote::load_manifest(remote::DEFAULT_REPO)?;
    let (items, _) = remote::items(&man, if kind == "cursor" { "cursors" } else { "icons" });
    let entry = items.into_iter().find(|e| e.get("name").and_then(|v| v.as_str()) == Some(name))
        .ok_or_else(|| remote::SourceError::new("remote.empty", name))?;
    remote::install(&entry, &looks::user_icons(), None)?;
    log(&format!("↓ {}", name));
    Ok(())
}
