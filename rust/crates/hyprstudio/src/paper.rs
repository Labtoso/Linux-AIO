//! Hyprpaper (Hintergrundbild) und Hyprcursor (Mauszeiger).
//!
//! hyprpaper.conf: `wallpaper { monitor, path (Bild oder Ordner = Diashow), fit_mode, timeout, order }`,
//! `splash = false`, `ipc = true`.

use crate::jsonx::{self, Get, Obj};
use crate::model::{self, Log, MARK};
use crate::util;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub const FITS: &[&str] = &["cover", "contain", "fill", "tile"];
pub const IMAGE_EXT: &[&str] = &[".png", ".jpg", ".jpeg", ".webp", ".jxl", ".bmp"];

pub fn default() -> Obj {
    jsonx::obj(json!({"path": "", "fit": "cover", "monitors": {}, "timeout": 30, "random": false,
        "recursive": false, "splash": false}))
}

/// Endung wie Pythons `Path.suffix.lower()` (mit Punkt, leer ohne Endung).
pub fn suffix(p: &Path) -> String {
    let name = util::file_name(p);
    match name.rfind('.') {
        Some(i) if i > 0 && i < name.len() - 1 => name[i..].to_lowercase(),
        _ => String::new(),
    }
}

pub fn is_image(p: &Path) -> bool {
    IMAGE_EXT.contains(&suffix(p).as_str())
}

pub fn paper_state(saved: Option<&Obj>) -> Obj {
    let mut s = default();
    if let Some(saved) = saved {
        for (k, v) in saved {
            if let Some(cur) = s.get(k) {
                if jsonx::same_type(v, cur) {
                    s.insert(k.clone(), v.clone());
                }
            }
        }
    }
    if !FITS.contains(&s.s("fit")) {
        s.insert("fit".into(), "cover".into());
    }
    let fit = s.s("fit").to_string();
    let mut mons = Obj::new();
    if let Some(m) = s.obj("monitors") {
        for (name, m) in m {
            let Some(mo) = m.as_object() else { continue };
            let Some(path) = mo.get("path").and_then(|p| p.as_str()).filter(|p| !p.is_empty()) else { continue };
            let f = mo.get("fit").and_then(|f| f.as_str()).filter(|f| FITS.contains(f)).unwrap_or(&fit);
            mons.insert(name.clone(), json!({"path": path, "fit": f}));
        }
    }
    s.insert("monitors".into(), Value::Object(mons));
    s
}

fn block(monitor: &str, path: &str, fit: &str, s: &Obj) -> Vec<String> {
    let mut out = vec!["wallpaper {".to_string(), format!("    monitor = {}", monitor), format!("    path = {}", path),
        format!("    fit_mode = {}", fit)];
    if !path.is_empty() && !is_image(&util::expanduser(path)) {
        out.push(format!("    timeout = {}", jsonx::py_str(s.val("timeout"))));
        if s.b("random") {
            out.push("    order = random".into());
        }
        if s.b("recursive") {
            out.push("    recursive = true".into());
        }
    }
    out.push("}".into());
    out
}

pub fn paper_conf(s: &Obj) -> String {
    let mut lines = vec![format!("# {} – automatisch erzeugt, wird beim Anwenden überschrieben", MARK), String::new()];
    if let Some(m) = s.obj("monitors") {
        let mut names: Vec<&String> = m.keys().collect();
        names.sort();
        for name in names {
            let mo = &m[name];
            lines.extend(block(name, mo.s("path"), mo.s("fit"), s));
            lines.push(String::new());
        }
    }
    if !s.s("path").is_empty() {
        lines.extend(block("", s.s("path"), s.s("fit"), s));
        lines.push(String::new());
    }
    lines.push(format!("splash = {}", if s.b("splash") { "true" } else { "false" }));
    lines.push("ipc = true".into());
    lines.join("\n") + "\n"
}

pub fn apply_paper(s: &Obj, hdir: &Path, log: Log) -> (bool, String) {
    if s.s("path").is_empty() && s.obj("monitors").map(|m| m.is_empty()).unwrap_or(true) {
        return (false, "paper.nopath".into());
    }
    let _ = std::fs::create_dir_all(hdir);
    let f = hdir.join("hyprpaper.conf");
    if let Some(bak) = model::backup_foreign(&f) {
        log(&format!("Sicherung: {}", bak.display()));
    }
    let _ = std::fs::write(&f, paper_conf(s));
    log(&format!("→ {}", f.display()));
    restart_paper(log);
    (true, String::new())
}

pub fn restart_paper(log: Log) {
    model::run(&["pkill", "-x", "hyprpaper"], log, true);
    if model::dry() {
        log("$ hyprpaper &  (Testmodus)");
        return;
    }
    std::thread::sleep(std::time::Duration::from_millis(300));
    match util::spawn_detached_in(&["hyprpaper"], Some(&util::home())) {
        Ok(()) => log("$ hyprpaper &"),
        Err(e) => log(&format!("hyprpaper: {}", e)),
    }
}

// ---------- Hyprcursor / XCursor ----------

pub fn icon_dirs() -> Vec<PathBuf> {
    if model::dry() {
        return vec![model::cache().join("testmodus").join("icons")];
    }
    let home = util::home();
    vec![home.join(".local/share/icons"), home.join(".icons"), PathBuf::from("/usr/share/icons")]
}

/// [(Name, Art)] – Art = "hyprcursor" (manifest.hl/.toml) oder "xcursor" (Ordner cursors/).
pub fn cursor_themes() -> Vec<(String, &'static str)> {
    let mut found: Vec<(String, &'static str)> = Vec::new();
    for d in icon_dirs() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
        entries.sort();
        for e in entries {
            if !e.is_dir() {
                continue;
            }
            let name = util::file_name(&e);
            let pos = found.iter().position(|x| x.0 == name);
            if e.join("manifest.hl").is_file() || e.join("manifest.toml").is_file() {
                match pos {
                    Some(i) => found[i].1 = "hyprcursor",
                    None => found.push((name, "hyprcursor")),
                }
            } else if e.join("cursors").is_dir() && pos.is_none() {
                found.push((name, "xcursor"));
            }
        }
    }
    found.sort_by_key(|x| x.0.to_lowercase());
    found
}

pub fn set_cursor_live(theme: &str, size: i64, gtk: bool, log: Log) {
    if theme.is_empty() {
        return;
    }
    let sz = size.to_string();
    model::run(&["hyprctl", "setcursor", theme, &sz], log, false);
    if gtk && (model::dry() || util::has("gsettings")) {
        model::run(&["gsettings", "set", "org.gnome.desktop.interface", "cursor-theme", theme], log, true);
        model::run(&["gsettings", "set", "org.gnome.desktop.interface", "cursor-size", &sz], log, true);
    }
}
