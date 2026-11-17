//! L Launcher („l-launcher“ bzw. „hypr-studio --launcher“): vergleicht die Eingabe mit den Programmnamen und sucht
//! bei jedem neuen Buchstaben neu – es bleiben nur Programme, deren Name mit der Eingabe ANFÄNGT. Enter startet den
//! obersten Treffer, oft Gestartetes steht weiter oben. Aussehen aus dem Launcher-Tab
//! (~/.config/LabtosoStudios/l-launcher.json). Hier die Logik; das Fenster liegt in `ui::llauncher`.

use crate::jsonx::{self, Obj};
use crate::{model, util};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub const APP_ID: &str = "l-launcher";

#[derive(Clone, Debug, PartialEq)]
pub struct App {
    pub id: String,
    pub name: String,
    pub exec: String,
    pub icon: String,
    pub terminal: bool,
    pub path: String,
}

pub fn settings_file() -> PathBuf {
    model::config_home().join("LabtosoStudios").join("l-launcher.json")
}

pub fn history_file() -> PathBuf {
    model::cache().join("l-launcher-history.json")
}

pub fn app_dirs() -> Vec<PathBuf> {
    let home = util::home();
    let mut data = vec![util::xdg_data()];
    let dirs = std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    data.extend(dirs.split(':').filter(|p| !p.is_empty()).map(PathBuf::from));
    data.push(home.join(".local/share/flatpak/exports/share"));
    data.push(PathBuf::from("/var/lib/flatpak/exports/share"));
    let mut out: Vec<PathBuf> = vec![];
    for d in data {
        let a = d.join("applications");
        if !out.contains(&a) {
            out.push(a);
        }
    }
    out
}

fn lang_keys() -> Vec<String> {
    let lang = std::env::var("LC_MESSAGES").ok().filter(|s| !s.is_empty())
        .or_else(|| std::env::var("LANG").ok()).unwrap_or_default();
    let lang = lang.split('.').next().unwrap_or("").to_string();
    let mut keys = vec![];
    if !lang.is_empty() {
        keys.push(format!("Name[{}]", lang));
        if let Some((l, _)) = lang.split_once('_') {
            keys.push(format!("Name[{}]", l));
        }
    }
    keys.push("Name".into());
    keys
}

/// [Desktop Entry] lesen; None für Ausgeblendetes, Nicht-Anwendungen und Einträge ohne Exec.
pub fn parse_desktop(text: &str, lang_keys: &[String]) -> Option<App> {
    let mut entry: Vec<(String, String)> = vec![];
    let mut sect = "";
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            sect = line;
            continue;
        }
        if sect != "[Desktop Entry]" {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        let k = k.trim();
        if !entry.iter().any(|e| e.0 == k) {
            entry.push((k.into(), v.trim().into()));
        }
    }
    let get = |k: &str| entry.iter().find(|e| e.0 == k).map(|e| e.1.as_str()).unwrap_or("");
    if entry.iter().any(|e| e.0 == "Type") && get("Type") != "Application" {
        return None;
    }
    if get("NoDisplay").eq_ignore_ascii_case("true") || get("Hidden").eq_ignore_ascii_case("true") {
        return None;
    }
    let name = lang_keys.iter().map(|k| get(k)).find(|v| !v.is_empty()).unwrap_or("");
    if name.is_empty() || get("Exec").is_empty() {
        return None;
    }
    Some(App { id: String::new(), name: name.into(), exec: get("Exec").into(), icon: get("Icon").into(),
        terminal: get("Terminal").eq_ignore_ascii_case("true"), path: get("Path").into() })
}

fn desktop_files(d: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = crate::fonts::walk(d).into_iter()
        .filter(|p| p.extension().map(|e| e == "desktop").unwrap_or(false)).collect();
    v.sort();
    v
}

/// Alle Programme; gleiche Desktop-ID nur einmal (die erste gewinnt, wie bei XDG üblich).
pub fn load_apps(dirs: Option<Vec<PathBuf>>) -> Vec<App> {
    let keys = lang_keys();
    let mut seen = HashSet::new();
    let mut apps = vec![];
    for d in dirs.unwrap_or_else(app_dirs) {
        if !d.is_dir() {
            continue;
        }
        for f in desktop_files(&d) {
            let Ok(rel) = f.strip_prefix(&d) else { continue };
            let did = rel.to_string_lossy().replace('/', "-");
            if !seen.insert(did.clone()) {
                continue;
            }
            let Ok(bytes) = std::fs::read(&f) else { continue };
            if let Some(mut a) = parse_desktop(&String::from_utf8_lossy(&bytes), &keys) {
                a.id = did;
                apps.push(a);
            }
        }
    }
    apps
}

/// Nur Namen, die mit der Eingabe ANFANGEN; oft Gestartetes zuerst, dann alphabetisch.
pub fn filter_apps<'a>(apps: &'a [App], query: &str, history: &Obj, case_insensitive: bool) -> Vec<&'a App> {
    let mut q = query.trim().to_string();
    if case_insensitive {
        q = q.to_lowercase();
    }
    let mut hits: Vec<&App> = apps.iter().filter(|a| {
        if case_insensitive { a.name.to_lowercase().starts_with(&q) } else { a.name.starts_with(&q) }
    }).collect();
    let count = |a: &App| history.get(&a.id).and_then(|v| v.as_i64()).unwrap_or(0);
    hits.sort_by(|a, b| count(b).cmp(&count(a)).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    hits
}

pub fn command(app: &App, terminal: &str) -> String {
    let re = regex::Regex::new(r"%[fFuUdDnNickvm]").unwrap();
    let mut cmd = re.replace_all(&app.exec, "").replace("%%", "%").trim().to_string();
    if app.terminal {
        cmd = format!("{} -e {}", terminal, cmd);
    }
    cmd
}

pub fn launch(app: &App, terminal: &str) {
    let cwd = if app.path.is_empty() { util::home() } else { util::expanduser(&app.path) };
    let cwd = cwd.is_dir().then_some(cwd);
    let _ = util::spawn_detached_in(&["sh", "-c", &command(app, terminal)], cwd.as_deref());
}

pub fn load_json(f: &Path) -> Obj {
    util::read_opt(f).and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|v| v.as_object().cloned()).unwrap_or_default()
}

pub fn remember(app_id: &str) {
    let f = history_file();
    let mut h = load_json(&f);
    let n = h.get(app_id).and_then(|v| v.as_i64()).unwrap_or(0) + 1;
    h.insert(app_id.into(), n.into());
    if let Some(p) = f.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    let _ = std::fs::write(&f, jsonx::dumps(&serde_json::Value::Object(h)));
}

// ---------- Fenster-Steuerung (zweites Drücken schließt) ----------

pub fn pid_file() -> PathBuf {
    match std::env::var("XDG_RUNTIME_DIR") {
        Ok(r) if !r.is_empty() => PathBuf::from(r).join("l-launcher.pid"),
        _ => util::with_name(&history_file(), "l-launcher.pid"),
    }
}

fn alive(pid: i32) -> bool {
    if unsafe { libc::kill(pid, 0) } != 0 {
        return false;
    }
    std::fs::read(format!("/proc/{}/cmdline", pid))
        .map(|b| b.windows(10).any(|w| w == b"--launcher")).unwrap_or(false)
}

/// Läuft schon ein L Launcher? Dann schließen und true zurückgeben (zweites Drücken).
pub fn toggle_running() -> bool {
    let Some(pid) = util::read_opt(&pid_file()).and_then(|t| t.trim().parse::<i32>().ok()) else { return false };
    if pid != std::process::id() as i32 && alive(pid) {
        return unsafe { libc::kill(pid, libc::SIGTERM) } == 0;
    }
    false
}

pub fn write_pid() {
    let f = pid_file();
    if let Some(p) = f.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    let _ = std::fs::write(&f, std::process::id().to_string());
}

pub fn remove_pid() {
    let f = pid_file();
    if util::read_opt(&f).map(|t| t.trim() == std::process::id().to_string()).unwrap_or(false) {
        let _ = std::fs::remove_file(f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_filter() {
        let keys = vec!["Name[de]".to_string(), "Name".to_string()];
        let a = parse_desktop("[Desktop Entry]\nName=Firefox\nName[de]=Feuerfuchs\nExec=firefox %u\nIcon=firefox\n\
            [Desktop Action new]\nName=Neu\nExec=x", &keys).unwrap();
        assert_eq!(a.name, "Feuerfuchs");
        assert_eq!(command(&a, "kitty"), "firefox");
        assert!(parse_desktop("[Desktop Entry]\nName=X\nExec=x\nNoDisplay=true", &keys).is_none());
        assert!(parse_desktop("[Desktop Entry]\nType=Link\nName=X\nExec=x", &keys).is_none());
        let apps = vec![App { id: "b".into(), name: "Htop".into(), ..a.clone() },
            App { id: "a".into(), name: "Hyprland".into(), ..a.clone() }, App { id: "c".into(), ..a.clone() }];
        let mut h = Obj::new();
        h.insert("b".into(), 3.into());
        let r = filter_apps(&apps, "h", &h, true);
        assert_eq!(r.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(), vec!["b", "a"]);
        assert_eq!(filter_apps(&apps, "hy", &h, true).len(), 1);
    }
}
