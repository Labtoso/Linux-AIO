//! GNOME-Einstellungen wie in GNOME Tweaks (über gsettings), Titelleisten-Knöpfe, Startprogramme,
//! Shell-/GTK-Themes und Erweiterungen.

use crate::{installers, pyjson, usercmd};
use hyprstudio::util;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

pub const IFACE: &str = "org.gnome.desktop.interface";
pub const WM: &str = "org.gnome.desktop.wm.preferences";
pub const MUTTER: &str = "org.gnome.mutter";
pub const USER_THEME: &str = "user-theme@gnome-shell-extensions.gcampax.github.com";
pub const WS_INDICATOR: &str = "workspace-indicator@gnome-shell-extensions.gcampax.github.com";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Bool,
    Choice,
    Int,
    Double,
    Font,
}

#[derive(Debug, Clone, Copy)]
pub enum Choices {
    None,
    List(&'static [&'static str]),
    Range(f64, f64, f64),
}

/// (Schlüssel für Texte, Schema, gsettings-Key, Art, Auswahl)
#[derive(Debug, Clone, Copy)]
pub struct Opt {
    pub label: &'static str,
    pub schema: &'static str,
    pub key: &'static str,
    pub kind: Kind,
    pub choices: Choices,
}

const fn o(label: &'static str, schema: &'static str, key: &'static str, kind: Kind, choices: Choices) -> Opt {
    Opt { label, schema, key, kind, choices }
}

use Choices::{List, None as No, Range};
use Kind::*;

pub const SECTIONS: &[(&str, &[Opt])] = &[
    ("fonts", &[
        o("font.ui", IFACE, "font-name", Font, No),
        o("font.doc", IFACE, "document-font-name", Font, No),
        o("font.mono", IFACE, "monospace-font-name", Font, No),
        o("font.title", WM, "titlebar-font", Font, No),
        o("font.hint", IFACE, "font-hinting", Choice, List(&["none", "slight", "medium", "full"])),
        o("font.aa", IFACE, "font-antialiasing", Choice, List(&["none", "grayscale", "rgba"])),
        o("font.scale", IFACE, "text-scaling-factor", Double, Range(0.5, 3.0, 0.05)),
    ]),
    ("look", &[
        o("look.scheme", IFACE, "color-scheme", Choice, List(&["default", "prefer-dark", "prefer-light"])),
        o("look.accent", IFACE, "accent-color", Choice,
          List(&["blue", "teal", "green", "yellow", "orange", "red", "pink", "purple", "slate"])),
        o("look.anim", IFACE, "enable-animations", Bool, No),
    ]),
    ("windows", &[
        o("win.dbl", WM, "action-double-click-titlebar", Choice,
          List(&["toggle-maximize", "toggle-maximize-horizontally", "toggle-maximize-vertically", "minimize",
                 "lower", "menu", "none"])),
        o("win.mid", WM, "action-middle-click-titlebar", Choice, List(&["none", "lower", "minimize", "toggle-maximize", "menu"])),
        o("win.right", WM, "action-right-click-titlebar", Choice, List(&["menu", "none", "lower", "minimize", "toggle-maximize"])),
        o("win.focus", WM, "focus-mode", Choice, List(&["click", "sloppy", "mouse"])),
        o("win.raise", WM, "raise-on-click", Bool, No),
        o("win.modifier", WM, "mouse-button-modifier", Choice, List(&["<Super>", "<Alt>"])),
        o("win.resize", WM, "resize-with-right-button", Bool, No),
        o("win.attach", MUTTER, "attach-modal-dialogs", Bool, No),
        o("win.center", MUTTER, "center-new-windows", Bool, No),
        o("win.edge", MUTTER, "edge-tiling", Bool, No),
    ]),
    ("topbar", &[
        o("top.date", IFACE, "clock-show-date", Bool, No),
        o("top.weekday", IFACE, "clock-show-weekday", Bool, No),
        o("top.seconds", IFACE, "clock-show-seconds", Bool, No),
        o("top.format", IFACE, "clock-format", Choice, List(&["24h", "12h"])),
        o("top.weeknum", "org.gnome.desktop.calendar", "show-weekdate", Bool, No),
        o("top.battery", IFACE, "show-battery-percentage", Bool, No),
        o("top.hotcorner", IFACE, "enable-hot-corners", Bool, No),
        o("top.dynamic", MUTTER, "dynamic-workspaces", Bool, No),
        o("top.numws", WM, "num-workspaces", Int, Range(1.0, 36.0, 1.0)),
        o("top.primary", MUTTER, "workspaces-only-on-primary", Bool, No),
        o("top.wsswitch", "org.gnome.shell.app-switcher", "current-workspace-only", Bool, No),
    ]),
    ("input", &[
        o("in.accel", "org.gnome.desktop.peripherals.mouse", "accel-profile", Choice, List(&["default", "flat", "adaptive"])),
        o("in.natural", "org.gnome.desktop.peripherals.mouse", "natural-scroll", Bool, No),
        o("in.tap", "org.gnome.desktop.peripherals.touchpad", "tap-to-click", Bool, No),
        o("in.tpnatural", "org.gnome.desktop.peripherals.touchpad", "natural-scroll", Bool, No),
        o("in.typing", "org.gnome.desktop.peripherals.touchpad", "disable-while-typing", Bool, No),
        o("in.paste", IFACE, "gtk-enable-primary-paste", Bool, No),
        o("in.overlay", MUTTER, "overlay-key", Choice, List(&["Super_L", "Super_R"])),
    ]),
];

pub fn section(name: &str) -> &'static [Opt] {
    SECTIONS.iter().find(|(n, _)| *n == name).map(|(_, v)| *v).unwrap_or(&[])
}

pub fn available(schema: &str, key: &str) -> bool {
    usercmd::dry() || usercmd::has_key(schema, key)
}

// ---------- Schrift-Werte: „Cantarell Bold 11“ ----------

pub fn split_font(value: &str) -> (String, f64) {
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(.*?)\s+(\d+(?:\.\d+)?)$").unwrap());
    let v = value.trim();
    match RE.captures(v) {
        Some(m) => (m[1].to_string(), m[2].parse().unwrap_or(11.0)),
        None => (v.to_string(), 11.0),
    }
}

/// Python-`f"{x:g}"`
pub fn fmt_g(x: f64) -> String {
    if x == 0.0 {
        return "0".into();
    }
    if !x.is_finite() {
        return if x.is_nan() { "nan".into() } else if x > 0.0 { "inf".into() } else { "-inf".into() };
    }
    let exp = x.abs().log10().floor() as i32;
    let s = format!("{:.5e}", x);
    let (m, e) = s.split_once('e').unwrap();
    let e: i32 = e.parse().unwrap_or(exp);
    if !(-4..6).contains(&e) {
        let m = m.trim_end_matches('0').trim_end_matches('.');
        return format!("{}e{}{:02}", m, if e < 0 { '-' } else { '+' }, e.abs());
    }
    let prec = (5 - e).max(0) as usize;
    let s = format!("{:.*}", prec, x);
    if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s }
}

pub fn join_font(family: &str, size: f64) -> String {
    format!("{} {}", family.trim(), fmt_g(size))
}

// ---------- Titelleisten-Knöpfe ----------

#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub side: String,
    pub minimize: bool,
    pub maximize: bool,
    pub close: bool,
    pub appmenu: bool,
}

/// 'appmenu:minimize,maximize,close' → side right, minimize, maximize …
pub fn parse_layout(value: &str) -> Layout {
    let v = if value.is_empty() { "appmenu:close" } else { value };
    let (left, right) = v.split_once(':').unwrap_or((v, ""));
    let lb: Vec<&str> = left.split(',').filter(|x| !x.is_empty()).collect();
    let rb: Vec<&str> = right.split(',').filter(|x| !x.is_empty()).collect();
    let mut side = if lb.contains(&"close") { "left" } else { "right" };
    let all: Vec<&str> = lb.iter().chain(rb.iter()).copied().collect();
    if !all.contains(&"close") {
        // ohne Schließen: Seite nach den übrigen Knöpfen
        side = if !lb.is_empty() && rb.is_empty() { "left" } else { "right" };
    }
    Layout { side: side.into(), minimize: all.contains(&"minimize"), maximize: all.contains(&"maximize"),
             close: all.contains(&"close"), appmenu: all.contains(&"appmenu") }
}

pub fn build_layout(side: &str, minimize: bool, maximize: bool, appmenu: bool, close: bool) -> String {
    let mut btns: Vec<&str> = Vec::new();
    if side == "left" {
        if close { btns.push("close"); }
        if maximize { btns.push("maximize"); }
        if minimize { btns.push("minimize"); }
        return format!("{}:{}", btns.join(","), if appmenu { "appmenu" } else { "" });
    }
    if minimize { btns.push("minimize"); }
    if maximize { btns.push("maximize"); }
    if close { btns.push("close"); }
    format!("{}:{}", if appmenu { "appmenu" } else { "" }, btns.join(","))
}

// ---------- Themes ----------

pub fn xdg_data_dirs() -> Vec<String> {
    std::env::var("XDG_DATA_DIRS").ok().filter(|s| !s.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".into())
        .split(':').filter(|d| !d.is_empty()).map(String::from).collect()
}

fn dedup(v: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for p in v {
        if !out.contains(&p) {
            out.push(p);
        }
    }
    out
}

pub fn theme_dirs() -> Vec<PathBuf> {
    let h = util::home();
    let mut dirs = vec![usercmd::data_dir().join("themes"), h.join(".local/share/themes"), h.join(".themes")];
    dirs.extend(xdg_data_dirs().into_iter().map(|d| PathBuf::from(d).join("themes")));
    dedup(dirs)
}

fn sorted_entries(d: &Path) -> Option<Vec<PathBuf>> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(d).ok()?.flatten().map(|e| e.path()).collect();
    v.sort();
    Some(v)
}

/// kind: gtk (gtk-3.0) oder shell (gnome-shell).
pub fn list_themes(kind: &str) -> Vec<String> {
    let sub = if kind == "gtk" { "gtk-3.0" } else { "gnome-shell" };
    let mut names: Vec<String> = Vec::new();
    for d in theme_dirs() {
        let Some(ents) = sorted_entries(&d) else { continue };
        for p in ents {
            let n = util::file_name(&p);
            if p.join(sub).is_dir() && !names.contains(&n) {
                names.push(n);
            }
        }
    }
    if kind == "gtk" {
        for n in ["Adwaita", "Adwaita-dark", "HighContrast"] {
            // eingebaut in GTK
            if !names.iter().any(|x| x == n) {
                names.insert(0, n.into());
            }
        }
    }
    names
}

// ---------- Erweiterungen ----------

pub fn ext_enabled() -> Vec<String> {
    match usercmd::gget("org.gnome.shell", "enabled-extensions", Value::Array(vec![])) {
        Value::Array(a) => a.into_iter().map(|v| match v { Value::String(s) => s, o => hyprstudio::jsonx::py_str(&o) }).collect(),
        _ => vec![],
    }
}

fn ext_roots() -> [PathBuf; 2] {
    [util::home().join(".local/share/gnome-shell/extensions"), PathBuf::from("/usr/share/gnome-shell/extensions")]
}

pub fn ext_installed(uuid: &str) -> bool {
    installers::dry_ext_has(uuid) || ext_roots().iter().any(|d| d.join(uuid).is_dir())
}

pub fn ext_dir(uuid: &str) -> Option<PathBuf> {
    ext_roots().into_iter().map(|d| d.join(uuid)).find(|d| d.join("metadata.json").is_file())
}

/// Unterstützt die laufende GNOME Shell die Erweiterung? None = unbekannt.
pub fn ext_compatible(uuid: &str) -> Option<bool> {
    let d = ext_dir(uuid);
    let out = usercmd::query(&["gnome-shell", "--version"]).unwrap_or_default();
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\d+)").unwrap());
    let major = RE.captures(&out).map(|m| m[1].to_string());
    let (d, major) = (d?, major?);
    let meta: Value = serde_json::from_str(&std::fs::read_to_string(d.join("metadata.json")).ok()?).ok()?;
    let versions = meta.get("shell-version").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    Some(versions.iter().any(|v| {
        let s = match v { Value::String(s) => s.clone(), o => hyprstudio::jsonx::py_str(o) };
        s.split('.').next() == Some(major.as_str())
    }))
}

pub fn version_check_off() -> bool {
    hyprstudio::jsonx::truthy(&usercmd::gget("org.gnome.shell", "disable-extension-version-validation", Value::Bool(false)))
}

/// Immer auch über enabled-extensions: gnome-extensions scheitert, solange die laufende Shell eine frisch
/// installierte Erweiterung noch nicht kennt (Wayland: aktiv nach dem nächsten Anmelden).
pub fn ext_set(uuid: &str, on: bool) -> bool {
    if usercmd::have("gnome-extensions") || usercmd::dry() {
        usercmd::run(&["gnome-extensions", if on { "enable" } else { "disable" }, uuid]);
    }
    let cur = ext_enabled();
    let mut new: Vec<String> = cur.iter().filter(|x| *x != uuid).cloned().collect();
    if on {
        new.push(uuid.into());
    }
    if new == cur {
        return true;
    }
    usercmd::gset("org.gnome.shell", "enabled-extensions", serde_json::json!(new))
}

/// Nur Erweiterungen im Benutzerordner lassen sich ohne Root entfernen.
pub fn ext_removable(uuid: &str) -> bool {
    installers::dry_ext_has(uuid) || util::home().join(".local/share/gnome-shell/extensions").join(uuid).is_dir()
}

pub fn ext_uninstall(uuid: &str) -> bool {
    ext_set(uuid, false);
    let ok = usercmd::run(&["gnome-extensions", "uninstall", uuid]).0;
    installers::DRY_EXT.lock().unwrap().remove(uuid);
    ok
}

pub fn shell_theme() -> String {
    match usercmd::gget("org.gnome.shell.extensions.user-theme", "name", Value::String(String::new())) {
        Value::String(s) => s,
        Value::Null => String::new(),
        v if !hyprstudio::jsonx::truthy(&v) => String::new(),
        v => hyprstudio::jsonx::py_str(&v),
    }
}

pub fn set_shell_theme(name: &str) -> bool {
    if !ext_enabled().iter().any(|x| x == USER_THEME) && ext_installed(USER_THEME) {
        ext_set(USER_THEME, true);
    }
    usercmd::gset("org.gnome.shell.extensions.user-theme", "name", Value::String(name.into()))
}

// ---------- Startprogramme (~/.config/autostart) ----------

pub fn app_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![util::xdg_data().join("applications")];
    dirs.extend(xdg_data_dirs().into_iter().map(|d| PathBuf::from(d).join("applications")));
    dirs.push(PathBuf::from("/var/lib/flatpak/exports/share/applications"));
    dirs.push(util::home().join(".local/share/flatpak/exports/share/applications"));
    dedup(dirs)
}

pub fn read_desktop(path: &Path) -> Map<String, Value> {
    let mut d = Map::new();
    let mut sec = String::new();
    if let Ok(b) = std::fs::read(path) {
        let text = String::from_utf8_lossy(&b).into_owned();
        for ln in util::splitlines(&text) {
            let s = ln.trim();
            if s.starts_with('[') {
                sec = s.to_string();
            } else if sec == "[Desktop Entry]" && s.contains('=') && !s.starts_with('#') {
                let (k, v) = s.split_once('=').unwrap();
                d.entry(k.trim().to_string()).or_insert(Value::String(v.trim().to_string()));
            }
        }
    }
    d
}

fn ds(d: &Map<String, Value>, k: &str) -> String {
    d.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

pub fn local_name(d: &Map<String, Value>) -> String {
    let lang_env = std::env::var("LANG").unwrap_or_default();
    let lang = lang_env.split('.').next().unwrap_or("").to_string();
    let short: String = lang.chars().take(2).collect();
    for k in [format!("Name[{}]", lang), format!("Name[{}]", short), "Name".into()] {
        let v = ds(d, &k);
        if !v.is_empty() {
            return v;
        }
    }
    String::new()
}

#[derive(Debug, Clone)]
pub struct App {
    pub file: PathBuf,
    pub id: String,
    pub name: String,
    pub icon: String,
    pub exec: String,
    pub enabled: bool,
}

fn desktop_files(d: &Path) -> Option<Vec<PathBuf>> {
    Some(sorted_entries(d)?.into_iter().filter(|p| util::file_name(p).ends_with(".desktop")).collect())
}

fn stem(p: &Path) -> String {
    p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Alle startbaren Programme (für „Hinzufügen“).
pub fn list_apps() -> Vec<App> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for d in app_dirs() {
        let Some(files) = desktop_files(&d) else { continue };
        for f in files {
            let id = util::file_name(&f);
            if !seen.insert(id.clone()) {
                continue;
            }
            let e = read_desktop(&f);
            let typ = e.get("Type").and_then(|v| v.as_str()).unwrap_or("Application");
            if typ != "Application" || ds(&e, "NoDisplay") == "true" || ds(&e, "Hidden") == "true" {
                continue;
            }
            let n = local_name(&e);
            out.push(App { name: if n.is_empty() { stem(&f) } else { n }, icon: ds(&e, "Icon"), exec: ds(&e, "Exec"),
                           id, file: f, enabled: true });
        }
    }
    out.sort_by_key(|a| a.name.to_lowercase());
    out
}

pub fn autostart_dir() -> PathBuf {
    usercmd::config_dir().join("autostart")
}

pub fn list_autostart() -> Vec<App> {
    let d = autostart_dir();
    let dirs = if usercmd::dry() { vec![d, util::home().join(".config/autostart")] } else { vec![d] };
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for folder in dirs {
        let Some(files) = desktop_files(&folder) else { continue };
        for f in files {
            let id = util::file_name(&f);
            if !seen.insert(id.clone()) {
                continue;
            }
            let e = read_desktop(&f);
            let en = e.get("X-GNOME-Autostart-enabled").and_then(|v| v.as_str()).unwrap_or("true");
            let on = ds(&e, "Hidden") != "true" && en != "false";
            let n = local_name(&e);
            out.push(App { name: if n.is_empty() { stem(&f) } else { n }, icon: ds(&e, "Icon"), exec: String::new(),
                           id, file: f, enabled: on });
        }
    }
    out.sort_by_key(|a| a.name.to_lowercase());
    out
}

pub fn autostart_add(app_file: &Path) -> PathBuf {
    let dest = autostart_dir().join(util::file_name(app_file));
    let _ = std::fs::create_dir_all(dest.parent().unwrap());
    if !dest.exists() {
        let _ = std::fs::write(&dest, util::read_text(app_file));
        usercmd::log(&format!("→ {}", dest.display()));
    }
    autostart_enable(&dest, true);
    dest
}

/// An/aus ohne zu löschen: Hidden + X-GNOME-Autostart-enabled in der eigenen Kopie.
pub fn autostart_enable(path: &Path, on: bool) {
    let dest = autostart_dir().join(util::file_name(path));
    if !dest.exists() {
        let _ = std::fs::create_dir_all(dest.parent().unwrap());
        let _ = std::fs::write(&dest, util::read_text(&usercmd::readable(path)));
    }
    usercmd::ini_set(&dest, "Desktop Entry", "Hidden", if on { "false" } else { "true" });
    usercmd::ini_set(&dest, "Desktop Entry", "X-GNOME-Autostart-enabled", if on { "true" } else { "false" });
}

// ---------- Anmeldebildschirm (GDM) ----------

pub fn gdm_installed() -> bool {
    usercmd::dry() || Path::new("/usr/share/gnome-shell/gnome-shell-theme.gresource").is_file()
}

/// Was Linux AIO beim GDM gesetzt hat (vom Root-Helfer geschrieben).
pub fn gdm_state() -> Map<String, Value> {
    let p = if usercmd::dry() { usercmd::cache().join("testmodus").join("gdm.json") }
            else { PathBuf::from("/var/lib/labtoso-linuxaio/gdm.json") };
    std::fs::read_to_string(p).ok().and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v.as_object().cloned()).unwrap_or_default()
}

// ---------- Top bar editor (Just Perfection) ----------

pub const JP_UUID: &str = "just-perfection-desktop@just-perfection";
pub const JP_SCHEMA: &str = "org.gnome.shell.extensions.just-perfection";
/// Element → (Just-Perfection-Schlüssel, Bereich der Leiste)
pub const BAR_ITEMS: [(&str, &str, &str); 8] = [
    ("activities", "activities-button", "left"),
    ("clock", "clock-menu", "center"),
    ("notify", "panel-notification-icon", "right"),
    ("keyboard", "keyboard-layout", "right"),
    ("a11y", "accessibility-menu", "right"),
    ("screenshare", "screen-sharing-indicator", "right"),
    ("quick", "quick-settings", "right"),
    ("power", "power-icon", "right"),
];
pub const QUICK_ITEMS: [(&str, &str); 5] = [("qs.dark", "quick-settings-dark-mode"),
    ("qs.night", "quick-settings-night-light"), ("qs.dnd", "quick-settings-do-not-disturb"),
    ("qs.airplane", "quick-settings-airplane-mode"), ("qs.backlight", "quick-settings-backlight")];
pub const POWER_ITEMS: [(&str, &str, &str, bool); 3] = [("pw.logout", "org.gnome.shell", "always-show-log-out", false),
    ("pw.switch", "org.gnome.desktop.lockdown", "disable-user-switching", true),
    ("pw.lock", "org.gnome.desktop.lockdown", "disable-lock-screen", true)];

pub fn jp_cmd() -> Vec<String> {
    match ext_dir(JP_UUID) {
        Some(d) => vec!["gsettings".into(), "--schemadir".into(), d.join("schemas").display().to_string()],
        None => vec!["gsettings".into()],
    }
}

pub fn jp_get(key: &str, default: Value) -> Value {
    if usercmd::dry() {
        if let Some(v) = usercmd::dry_values().get(&format!("{} {}", JP_SCHEMA, key)) {
            if !v.is_null() {
                return v.clone();
            }
        }
    }
    let mut c = jp_cmd();
    c.extend(["get".to_string(), JP_SCHEMA.into(), key.into()]);
    match usercmd::query_t(&c, 8) {
        None => default,
        Some(o) => usercmd::parse_value(&o),
    }
}

pub fn jp_set(key: &str, value: Value) -> bool {
    if usercmd::dry() {
        let mut vals = usercmd::dry_values();
        vals.insert(format!("{} {}", JP_SCHEMA, key), value.clone());
        let p = usercmd::dry_gset();
        let _ = std::fs::create_dir_all(p.parent().unwrap());
        let _ = std::fs::write(&p, pyjson::dumps(&Value::Object(vals), None, false, false));
    }
    let mut c = jp_cmd();
    c.extend(["set".to_string(), JP_SCHEMA.into(), key.into(), usercmd::gvariant(&value)]);
    usercmd::run_v(&c).0
}

pub fn jp_ready() -> bool {
    usercmd::dry() || (ext_installed(JP_UUID) && ext_enabled().iter().any(|x| x == JP_UUID))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fonts_layout() {
        assert_eq!(split_font("Cantarell Bold 11"), ("Cantarell Bold".into(), 11.0));
        assert_eq!(join_font("Noto Sans", 10.5), "Noto Sans 10.5");
        assert_eq!(join_font("Noto Sans", 11.0), "Noto Sans 11");
        assert_eq!(fmt_g(1234567.0), "1.23457e+06");
        assert_eq!(fmt_g(0.0001), "0.0001");
        let l = parse_layout("close,minimize:appmenu");
        assert_eq!(l.side, "left");
        assert!(l.minimize && !l.maximize);
        assert_eq!(build_layout("right", true, true, true, true), "appmenu:minimize,maximize,close");
        assert_eq!(build_layout("left", false, true, false, true), "close,maximize:");
    }
}
