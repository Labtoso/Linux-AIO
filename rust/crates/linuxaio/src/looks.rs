//! Mauszeiger- und Symbol-Themes: aus einem Ordner installieren (nach ~/.local/share/icons), auflisten,
//! pro Desktop anwenden und den aktiven Stand lesen (Vorschau-Bilder macht die Look-Seite).

use crate::{gnome, remote, usercmd};
use hyprstudio::util;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;
use std::path::{Path, PathBuf};

pub fn default_cursor_dir() -> PathBuf {
    util::home().join("Cursors") // Quelle „Ordner“, im Programm änderbar
}

pub fn default_icon_dir() -> PathBuf {
    util::home().join("Icons")
}

pub const CURSOR_PREVIEW: [&str; 4] = ["left_ptr", "hand2", "xterm", "watch"];
pub const ICON_PREVIEW: [&str; 8] = ["folder", "user-home", "utilities-terminal", "text-x-generic",
    "internet-web-browser", "system-file-manager", "accessories-text-editor", "preferences-system"];

fn aliases(name: &str) -> Vec<&str> {
    match name {
        "hand2" => vec!["hand2", "pointer", "hand1"],
        "watch" => vec!["watch", "wait", "left_ptr_watch"],
        "xterm" => vec!["xterm", "text"],
        "left_ptr" => vec!["left_ptr", "default", "arrow"],
        n => vec![n],
    }
}

/// Ziel der Installation (ohne root).
pub fn user_icons() -> PathBuf {
    usercmd::data_dir().join("icons")
}

pub fn icon_dirs() -> Vec<PathBuf> {
    let h = util::home();
    let mut dirs = vec![user_icons(), h.join(".local/share/icons"), h.join(".icons")];
    dirs.extend(gnome::xdg_data_dirs().into_iter().map(|d| PathBuf::from(d).join("icons")));
    let mut out: Vec<PathBuf> = Vec::new();
    for d in dirs {
        if !out.contains(&d) {
            out.push(d);
        }
    }
    out
}

pub fn index_name(folder: &Path) -> String {
    if let Some(s) = util::read_opt(&folder.join("index.theme")) {
        for ln in util::splitlines(&s) {
            if let Some(v) = ln.strip_prefix("Name=") {
                return v.trim().to_string();
            }
        }
    }
    util::file_name(folder)
}

pub fn is_cursor_theme(folder: &Path) -> bool {
    folder.join("cursors").is_dir()
}

/// Symbol-Theme = index.theme mit Directories= (reine Mauszeiger-Themes haben keins).
pub fn is_icon_theme(folder: &Path) -> bool {
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^Directories\s*=\s*\S").unwrap());
    let Some(text) = util::read_opt(&folder.join("index.theme")) else { return false };
    let n = util::file_name(folder);
    RE.is_match(&text) && n != "default" && n != "hicolor"
}

fn check(kind: &str, p: &Path) -> bool {
    if kind == "cursor" { is_cursor_theme(p) } else { is_icon_theme(p) }
}

pub fn installed_path(name: &str, kind: &str) -> Option<PathBuf> {
    icon_dirs().into_iter().map(|d| d.join(name)).find(|p| p.is_dir() && check(kind, p))
}

fn sorted_iter(d: &Path) -> Option<Vec<PathBuf>> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(d).ok()?.flatten().map(|e| e.path()).collect();
    v.sort();
    Some(v)
}

/// Name → Ordner (erste Fundstelle gewinnt, wie bei XDG).
pub fn list_installed(kind: &str) -> Vec<(String, PathBuf)> {
    let mut found: Vec<(String, PathBuf)> = Vec::new();
    for d in icon_dirs() {
        let Some(subs) = sorted_iter(&d) else { continue };
        for p in subs {
            let n = util::file_name(&p);
            if !found.iter().any(|(k, _)| *k == n) && p.is_dir() && check(kind, &p) {
                found.push((n, p));
            }
        }
    }
    found
}

/// Themes im Quellordner (ein Ordner pro Theme, eine Ebene tief).
pub fn list_folder(folder: &Path, kind: &str) -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    let Some(subs) = sorted_iter(folder) else { return out };
    for p in subs {
        if p.is_dir() && check(kind, &p) {
            out.push((util::file_name(&p), p));
        } else if p.is_dir() {
            // Cloud-Ordner ohne Symlinks: Theme als Archiv <Name>/<Name>.tar.xz
            if let Some(arc) = sorted_iter(&p).unwrap_or_default().into_iter()
                .find(|x| util::file_name(x).ends_with(".tar.xz")) {
                out.push((util::file_name(&p), arc));
            }
        }
    }
    out
}

#[derive(Debug, Clone)]
pub struct Item {
    pub name: String,
    pub label: String,
    pub src: Option<PathBuf>,
    pub installed: Option<PathBuf>,
    pub color: Option<String>,
}

/// Kacheln: Themes aus dem Ordner (installierbar) und alles schon Installierte.
pub fn items(folder: &Path, kind: &str) -> Vec<Item> {
    let src = list_folder(folder, kind);
    let inst = list_installed(kind);
    let get = |v: &Vec<(String, PathBuf)>, n: &str| v.iter().find(|(k, _)| k == n).map(|(_, p)| p.clone());
    let mut names: Vec<String> = src.iter().map(|(n, _)| n.clone()).collect();
    names.extend(inst.iter().filter(|(n, _)| get(&src, n).is_none()).map(|(n, _)| n.clone()));
    names.into_iter().map(|name| {
        let path = get(&inst, &name);
        let s = get(&src, &name);
        let label_from = path.clone().or_else(|| s.clone().filter(|x| x.is_dir()));
        Item { label: label_from.map(|p| index_name(&p)).unwrap_or_else(|| name.clone()), src: s, installed: path,
               color: color_of(&name), name }
    }).collect()
}

/// Bibata-Material-<Farbe> → Farbname (für die Suche).
pub fn color_of(name: &str) -> Option<String> {
    name.strip_prefix("Bibata-Material-").filter(|r| !r.is_empty()).map(|r| r.replace('-', " "))
}

fn copytree_links(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for e in std::fs::read_dir(src)?.flatten() {
        let p = e.path();
        let ft = e.file_type()?;
        let t = dst.join(e.file_name());
        if ft.is_symlink() {
            std::os::unix::fs::symlink(std::fs::read_link(&p)?, &t)?;
        } else if ft.is_dir() {
            copytree_links(&p, &t)?;
        } else {
            util::copy2(&p, &t)?;
        }
    }
    Ok(())
}

/// Theme-Ordner (oder .tar.xz-Archiv) nach ~/.local/share/icons. Vorhandenes wird nie überschrieben.
pub fn install(src: &Path, log: &dyn Fn(&str)) -> Result<PathBuf, String> {
    if src.is_file() {
        let data = std::fs::read(src).map_err(|e| e.to_string())?;
        let new = remote::safe_extract(&data, &user_icons(), None, true).map_err(|e| e.to_string())?;
        let name = util::file_name(src.parent().unwrap_or(Path::new("")));
        log(&format!("→ {}", user_icons().join(&name).display()));
        let p = user_icons().join(&name);
        return Ok(if p.exists() { p } else { new.into_iter().next().unwrap_or(p) });
    }
    let dest = user_icons().join(util::file_name(src));
    if dest.exists() {
        log(&format!("schon da: {}", dest.display()));
        return Ok(dest);
    }
    let _ = std::fs::create_dir_all(dest.parent().unwrap());
    let tmp = util::with_name(&dest, &format!("{}.linuxaio-tmp", util::file_name(&dest)));
    if tmp.exists() {
        let _ = std::fs::remove_dir_all(&tmp); // eigener halbfertiger Kopier-Rest
    }
    copytree_links(src, &tmp).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &dest).map_err(|e| e.to_string())?;
    log(&format!("→ {}{}", dest.display(), if usercmd::dry() { "   [Testmodus]" } else { "" }));
    Ok(dest)
}

// ---------- Xcursor lesen (für die Vorschau) ----------

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    d.get(at..at + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Erstes Bild der passenden Größe: (Breite, Höhe, BGRA-Bytes) oder None.
pub fn read_xcursor(path: &Path, want: u32) -> Option<(u32, u32, Vec<u8>)> {
    let data = std::fs::read(path).ok()?;
    if data.len() < 16 || &data[..4] != b"Xcur" {
        return None;
    }
    let hsize = u32le(&data, 4)? as usize;
    let ntoc = u32le(&data, 12)?;
    let mut best: Option<(u32, usize)> = None;
    for i in 0..ntoc.min(512) as usize {
        let at = hsize + i * 12;
        let typ = u32le(&data, at)?;
        let sub = u32le(&data, at + 4)?;
        let pos = u32le(&data, at + 8)? as usize;
        if typ != 0xFFFD0002 {
            continue;
        }
        if best.map(|(b, _)| (sub as i64 - want as i64).abs() < (b as i64 - want as i64).abs()).unwrap_or(true) {
            best = Some((sub, pos));
        }
    }
    let (_, pos) = best?;
    let w = u32le(&data, pos + 16)?;
    let h = u32le(&data, pos + 20)?;
    let start = pos + 36;
    if w > 512 || h > 512 || start + (w * h * 4) as usize > data.len() {
        return None;
    }
    Some((w, h, data[start..start + (w * h * 4) as usize].to_vec()))
}

pub fn cursor_file(theme: &Path, name: &str) -> Option<PathBuf> {
    aliases(name).into_iter().map(|n| theme.join("cursors").join(n)).find(|p| p.is_file())
}

/// Ein Symbol aus dem Theme finden (übliche Ordner-Schemata, ohne alles zu durchsuchen).
pub fn icon_file(theme: &Path, name: &str) -> Option<PathBuf> {
    let sizes = ["scalable", "48x48", "48", "64x64", "64", "32x32", "32", "256x256", "128x128", "96"];
    let ctxs = ["places", "apps", "mimetypes", "categories", "devices", "actions"];
    for ctx in ctxs {
        for s in sizes {
            for base in [theme.join(s).join(ctx), theme.join(ctx).join(s)] {
                for ext in [".svg", ".png"] {
                    let p = base.join(format!("{}{}", name, ext));
                    if p.is_file() {
                        return Some(p);
                    }
                }
            }
        }
    }
    None
}

// ---------- Anwenden pro Desktop ----------

/// Hyprland: Zusatzdatei hyprstudio_cursor.lua/.conf (über Hypr Studios Einbindung).
fn hypr_include(name: &str, size: i64, log: &dyn Fn(&str)) {
    let hdir = hyprstudio::model::config_home().join("hypr");
    let (n, s) = (name.to_string(), size);
    let content = move |fmt: &str| -> String {
        if fmt == "lua" {
            format!("-- Linux AIO: Mauszeiger\nhl.env(\"XCURSOR_THEME\", \"{n}\")\nhl.env(\"XCURSOR_SIZE\", \"{s}\")\n\
                     hl.env(\"HYPRCURSOR_THEME\", \"{n}\")\nhl.env(\"HYPRCURSOR_SIZE\", \"{s}\")\n")
        } else {
            format!("# Linux AIO: Mauszeiger\nenv = XCURSOR_THEME,{n}\nenv = XCURSOR_SIZE,{s}\n\
                     env = HYPRCURSOR_THEME,{n}\nenv = HYPRCURSOR_SIZE,{s}\n")
        }
    };
    hyprstudio::laptopkeys::include_extra(&hdir, "cursor", &content, "Mauszeiger (Linux AIO)", log);
}

fn gtk_settings(key: &str, value: &str) {
    for v in ["gtk-3.0", "gtk-4.0"] {
        usercmd::ini_set(&usercmd::config_dir().join(v).join("settings.ini"), "Settings", key, value);
    }
}

/// X11/XWayland-Programme ohne Desktop-Einstellung: ~/.icons/default/index.theme erbt das Theme.
fn default_index(name: &str) {
    usercmd::ini_set(&usercmd::home().join(".icons/default/index.theme"), "Icon Theme", "Inherits", name);
}

fn s(v: &str) -> Value {
    Value::String(v.into())
}

/// KDE Plasma: Größe zuerst in kcminputrc (plasma-apply-cursortheme übernimmt sie von dort),
/// dann das Theme über das Plasma-Werkzeug. Klappt das nicht (fehlt, findet das Theme nicht, ältere
/// Plasma-Version), selbst eintragen und Plasma/KWin über D-Bus Bescheid geben.
fn plasma_cursor(name: &str, sz: &str, log: &dyn Fn(&str)) -> bool {
    let rc = usercmd::config_dir().join("kcminputrc");
    usercmd::ini_set(&rc, "Mouse", "cursorSize", sz);
    let (ok, out) = usercmd::run(&["plasma-apply-cursortheme", name]);
    if ok || out.contains("already") || out.contains("bereits") {
        return true;
    }
    usercmd::ini_set(&rc, "Mouse", "cursorTheme", name);
    // KGlobalSettings::ChangeCursor = 5 – laufende KDE-Programme laden den Zeiger neu
    usercmd::run(&["dbus-send", "--session", "--type=signal", "/KGlobalSettings",
                   "org.kde.KGlobalSettings.notifyChange", "int32:5", "int32:0"]);
    usercmd::run(&["dbus-send", "--session", "--type=signal", "/KWin", "org.kde.KWin.reloadConfig"]);
    log("Plasma: Mauszeiger in kcminputrc eingetragen – falls er sich noch nicht ändert, einmal neu anmelden.");
    true
}

pub fn apply_cursor(de: &str, name: &str, size: i64, log: &dyn Fn(&str)) -> bool {
    let gs = usercmd::gset;
    let mut ok = true;
    let sz = size.to_string();
    match de {
        "gnome" | "budgie" | "pantheon" | "cosmic" => {
            ok = gs("org.gnome.desktop.interface", "cursor-theme", s(name));
            gs("org.gnome.desktop.interface", "cursor-size", Value::from(size));
        }
        "cinnamon" => {
            ok = gs("org.cinnamon.desktop.interface", "cursor-theme", s(name));
            gs("org.cinnamon.desktop.interface", "cursor-size", Value::from(size));
        }
        "mate" => {
            ok = gs("org.mate.peripherals-mouse", "cursor-theme", s(name));
            gs("org.mate.peripherals-mouse", "cursor-size", Value::from(size));
        }
        "xfce" => {
            ok = usercmd::run(&["xfconf-query", "-c", "xsettings", "-p", "/Gtk/CursorThemeName", "-s", name]).0;
            usercmd::run(&["xfconf-query", "-c", "xsettings", "-p", "/Gtk/CursorThemeSize", "-s", &sz]);
        }
        "plasma" => ok = plasma_cursor(name, &sz, log),
        "lxqt" => {
            let p = usercmd::config_dir().join("lxqt/session.conf");
            usercmd::ini_set(&p, "Mouse", "cursor_theme", name);
            usercmd::ini_set(&p, "Mouse", "cursor_size", &sz);
        }
        "hyprland" => {
            ok = usercmd::run(&["hyprctl", "setcursor", name, &sz]).0;
            hypr_include(name, size, log);
            if usercmd::has_schema("org.gnome.desktop.interface") || usercmd::dry() {
                gs("org.gnome.desktop.interface", "cursor-theme", s(name)); // GTK-Programme
            }
        }
        "sway" => {
            ok = usercmd::run(&["swaymsg", "seat", "*", "xcursor_theme", name, &sz]).0;
        }
        "niri" => log("niri: in ~/.config/niri/config.kdl unter cursor { xcursor-theme \"…\" } eintragen"),
        _ => {}
    }
    if !["plasma", "gnome", "budgie", "cinnamon", "mate", "pantheon"].contains(&de) {
        gtk_settings("gtk-cursor-theme-name", name);
        gtk_settings("gtk-cursor-theme-size", &sz);
    }
    default_index(name);
    ok
}

pub fn apply_icons(de: &str, name: &str) -> bool {
    let gs = usercmd::gset;
    let mut ok = true;
    match de {
        "gnome" | "budgie" | "pantheon" | "cosmic" => ok = gs("org.gnome.desktop.interface", "icon-theme", s(name)),
        "cinnamon" => ok = gs("org.cinnamon.desktop.interface", "icon-theme", s(name)),
        "mate" => ok = gs("org.mate.interface", "icon-theme", s(name)),
        "xfce" => ok = usercmd::run(&["xfconf-query", "-c", "xsettings", "-p", "/Net/IconThemeName", "-s", name]).0,
        "plasma" => {
            let tool = ["/usr/lib/plasma-changeicons", "/usr/libexec/plasma-changeicons",
                        "/usr/lib64/libexec/plasma-changeicons"].into_iter().find(|t| Path::new(t).is_file());
            if tool.is_some() || usercmd::dry() {
                ok = usercmd::run(&[tool.unwrap_or("/usr/lib/plasma-changeicons"), name]).0;
            } else {
                usercmd::ini_set(&usercmd::config_dir().join("kdeglobals"), "Icons", "Theme", name);
            }
        }
        "lxqt" => usercmd::ini_set(&usercmd::config_dir().join("lxqt/lxqt.conf"), "General", "icon_theme", name),
        _ => {
            if usercmd::has_schema("org.gnome.desktop.interface") || usercmd::dry() {
                gs("org.gnome.desktop.interface", "icon-theme", s(name));
            }
        }
    }
    if !["plasma", "gnome", "budgie", "cinnamon", "mate", "pantheon"].contains(&de) {
        gtk_settings("gtk-icon-theme-name", name);
    }
    ok
}

fn as_str(v: Value) -> String {
    match v {
        Value::String(s) => s,
        Value::Null => String::new(),
        v if !hyprstudio::jsonx::truthy(&v) => String::new(),
        v => hyprstudio::jsonx::py_str(&v),
    }
}

/// Aktives Theme lesen (für das Schild „Angewendet“).
pub fn current(de: &str, kind: &str) -> String {
    let key = if kind == "cursor" { "cursor-theme" } else { "icon-theme" };
    let g = |schema: &str| as_str(usercmd::gget(schema, key, s("")));
    let cfg = usercmd::config_dir();
    match de {
        "gnome" | "budgie" | "pantheon" | "cosmic" | "hyprland" | "sway" | "i3" | "niri" => {
            let v = g("org.gnome.desktop.interface");
            if !v.is_empty() && ["gnome", "budgie", "pantheon", "cosmic"].contains(&de) {
                return v;
            }
            let ini = usercmd::readable(&cfg.join("gtk-3.0/settings.ini"));
            usercmd::ini_get(&ini, "Settings", if kind == "cursor" { "gtk-cursor-theme-name" } else { "gtk-icon-theme-name" }, &v)
        }
        "cinnamon" => g("org.cinnamon.desktop.interface"),
        "mate" => g(if kind == "cursor" { "org.mate.peripherals-mouse" } else { "org.mate.interface" }),
        "xfce" => {
            let prop = if kind == "cursor" { "/Gtk/CursorThemeName" } else { "/Net/IconThemeName" };
            usercmd::query(&["xfconf-query", "-c", "xsettings", "-p", prop]).unwrap_or_default()
        }
        "plasma" => {
            if kind == "cursor" {
                usercmd::ini_get(&usercmd::readable(&cfg.join("kcminputrc")), "Mouse", "cursorTheme", "breeze_cursors")
            } else {
                usercmd::ini_get(&usercmd::readable(&cfg.join("kdeglobals")), "Icons", "Theme", "breeze")
            }
        }
        "lxqt" => {
            if kind == "cursor" {
                usercmd::ini_get(&usercmd::readable(&cfg.join("lxqt/session.conf")), "Mouse", "cursor_theme", "")
            } else {
                usercmd::ini_get(&usercmd::readable(&cfg.join("lxqt/lxqt.conf")), "General", "icon_theme", "")
            }
        }
        _ => String::new(),
    }
}
