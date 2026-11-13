//! Hell/Dunkel für alle Programme.
//!
//! Unter Hyprland (ohne Plasma-Sitzung) reicht das KDE-Farbschema allein nicht: Qt-/KDE-Apps wie Kate und KWrite
//! lesen kdeglobals nur mit dem KDE-Plattform-Theme (Paket plasma-integration, QT_QPA_PLATFORMTHEME=kde). Darum:
//! - GTK 3/4: settings.ini (prefer-dark + Adwaita-dark), gsettings color-scheme/gtk-theme (Portal → libadwaita, Electron)
//! - KDE/Qt: Farbschema BreezeDark/BreezeLight in kdeglobals (aus /usr/share/color-schemes, sonst eingebaut),
//!   plasma-apply-colorscheme falls vorhanden, Farbschema zusätzlich in den rc-Dateien der KDE-Apps ([UiSettings])
//! - qt5ct/qt6ct (falls statt plasma-integration genutzt): dunkle/helle Palette
//! - Hyprland-Umgebung QT_QPA_PLATFORMTHEME (schreibt die Hyprland-Seite, siehe qt_platform_theme)

use crate::model::{self, Log, MARK};
use crate::{laptopkeys, packages, util};
use std::path::{Path, PathBuf};

pub const MODES: &[&str] = &["dark", "light"];
pub const DEFAULT: &str = "dark";

pub fn scheme(mode: &str) -> (&'static str, &'static str) {
    if mode == "light" { ("BreezeLight", "Breeze Light") } else { ("BreezeDark", "Breeze Dark") }
}

/// KDE-Apps mit eigenem Farbschema-Menü (Einstellungen › Farbschema) → <app>rc [UiSettings] ColorScheme
pub const KDE_APPS: &[&str] = &["kate", "kwrite", "dolphin", "okular", "gwenview", "ark", "spectacle", "kcalc",
    "filelight", "elisa", "kdenlive", "krita", "partitionmanager", "kfind", "kcharselect", "skanlite", "kolourpaint",
    "k3b", "konversation", "kmail2", "kontact", "korganizer", "akregator", "ktorrent", "kdevelop", "haruna",
    "kwalletmanager5", "kinfocenter", "systemsettings", "discover", "plasma-discover"];

pub const KDE_PLUGIN_GLOBS: &[&str] = &["/usr/lib/qt6/plugins/platformthemes/KDEPlasmaPlatformTheme6.so",
    "/usr/lib/*/qt6/plugins/platformthemes/KDEPlasmaPlatformTheme6.so",
    "/usr/lib64/qt6/plugins/platformthemes/KDEPlasmaPlatformTheme6.so"];

/// Eingebaute Notfall-Farben (Breeze), falls /usr/share/color-schemes fehlt
fn fallback(mode: &str) -> Vec<(&'static str, (&'static str, &'static str))> {
    if mode == "light" {
        vec![("Colors:Window", ("239,240,241", "35,38,41")), ("Colors:View", ("255,255,255", "35,38,41")),
            ("Colors:Button", ("252,252,252", "35,38,41")), ("Colors:Selection", ("61,174,233", "255,255,255")),
            ("Colors:Tooltip", ("247,247,247", "35,38,41")), ("Colors:Complementary", ("42,46,50", "252,252,252")),
            ("Colors:Header", ("222,224,226", "35,38,41"))]
    } else {
        vec![("Colors:Window", ("32,35,38", "252,252,252")), ("Colors:View", ("20,22,24", "252,252,252")),
            ("Colors:Button", ("41,44,48", "252,252,252")), ("Colors:Selection", ("61,174,233", "252,252,252")),
            ("Colors:Tooltip", ("41,44,48", "252,252,252")), ("Colors:Complementary", ("20,22,24", "252,252,252")),
            ("Colors:Header", ("32,35,38", "252,252,252"))]
    }
}

// ---------- INI-Hilfen ----------

pub type Section = (String, Vec<(String, Option<String>)>);
/// {Abschnitt: {Schlüssel: Wert}} mit Reihenfolge
pub type Updates = Vec<(String, Vec<(String, String)>)>;

/// [(Abschnitt, [(Schlüssel, Wert)])] – Reihenfolge bleibt erhalten; Zeilen vor dem 1. Abschnitt unter "".
pub fn read_ini(text: &str) -> Vec<Section> {
    let mut out: Vec<Section> = vec![(String::new(), vec![])];
    for ln in text.lines() {
        let s = ln.trim();
        if s.starts_with('[') && s.ends_with(']') && s.len() >= 2 {
            out.push((s[1..s.len() - 1].to_string(), vec![]));
        } else if s.contains('=') && !s.starts_with('#') && !s.starts_with(';') {
            let (k, v) = s.split_once('=').unwrap();
            out.last_mut().unwrap().1.push((k.trim().to_string(), Some(v.trim().to_string())));
        } else if !s.is_empty() {
            out.last_mut().unwrap().1.push((s.to_string(), None));
        }
    }
    out
}

pub fn write_ini(sections: &[Section]) -> String {
    let mut lines: Vec<String> = vec![];
    for (name, items) in sections {
        if !name.is_empty() {
            if !lines.is_empty() {
                lines.push(String::new());
            }
            lines.push(format!("[{}]", name));
        }
        for (k, v) in items {
            lines.push(match v { None => k.clone(), Some(v) => format!("{}={}", k, v) });
        }
    }
    lines.join("\n").trim_start_matches('\n').to_string() + "\n"
}

/// updates = {Abschnitt: {Schlüssel: Wert}} in vorhandenen Text einsetzen, Rest behalten.
pub fn merge_ini(text: &str, updates: &Updates) -> String {
    let mut secs = read_ini(text);
    for (sec, kv) in updates {
        let pos = match secs.iter().position(|(n, _)| n == sec) {
            Some(p) => p,
            None => {
                secs.push((sec.clone(), vec![]));
                secs.len() - 1
            }
        };
        let hit = &mut secs[pos].1;
        for (k, v) in kv {
            match hit.iter().position(|(kk, _)| kk == k) {
                None => hit.push((k.clone(), Some(v.clone()))),
                Some(i) => hit[i] = (k.clone(), Some(v.clone())),
            }
        }
    }
    write_ini(&secs)
}

fn upd(sec: &str, kv: &[(&str, &str)]) -> Updates {
    vec![(sec.to_string(), kv.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect())]
}

pub fn update_ini(f: &Path, updates: &Updates, log: Log, backup: bool) {
    if let Some(d) = f.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    if backup && f.is_file() && !util::add_suffix(f, ".hyprstudio-original").exists() {
        if let Some(bak) = model::backup_foreign(f) {
            // nur einmal: das Original vor der ersten Änderung
            log(&format!("Sicherung: {}", bak.display()));
        }
    }
    let old = if f.is_file() { util::read_text(f) } else { String::new() };
    let new = merge_ini(&old, updates);
    if new != old {
        let _ = std::fs::write(f, new);
        log(&format!("→ {}", f.display()));
    }
}

/// Schlüssel in [Settings] setzen (GTK), Rest der Datei behalten; fremde Datei vorher sichern.
pub fn set_ini(f: &Path, key: &str, value: &str, log: Log) {
    let mut old = if f.is_file() { util::read_text(f) } else { String::new() };
    let head: String = old.chars().take(300).collect();
    if !head.contains(MARK) {
        if let Some(d) = f.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        if let Some(bak) = model::backup_foreign(f) {
            log(&format!("Sicherung: {}", bak.display()));
        }
        old = format!("# {}\n{}", MARK, old);
        let _ = std::fs::write(f, &old);
    }
    update_ini(f, &upd("Settings", &[(key, value)]), log, false);
}

// ---------- KDE-Farbschema ----------

pub fn scheme_file(mode: &str) -> Option<PathBuf> {
    let name = scheme(mode).0;
    for d in [util::home().join(".local/share/color-schemes"), PathBuf::from("/usr/share/color-schemes")] {
        let f = d.join(format!("{}.colors", name));
        if f.is_file() {
            return Some(f);
        }
    }
    None
}

/// Alle Farb-Abschnitte des Schemas (Colors:*, WM) als {Abschnitt: {Schlüssel: Wert}}.
pub fn scheme_colors(mode: &str) -> Updates {
    if let Some(f) = scheme_file(mode) {
        let out: Updates = read_ini(&util::read_text(&f)).into_iter()
            .filter(|(n, _)| n.starts_with("Colors:") || n == "WM")
            .map(|(n, items)| (n, items.into_iter().filter_map(|(k, v)| v.map(|v| (k, v))).collect()))
            .collect();
        if !out.is_empty() {
            return out;
        }
    }
    fallback(mode).into_iter().map(|(sec, (bg, fg))| (sec.to_string(), vec![
        ("BackgroundNormal".into(), bg.into()), ("BackgroundAlternate".into(), bg.into()),
        ("ForegroundNormal".into(), fg.into()), ("ForegroundInactive".into(), fg.into()),
        ("DecorationFocus".into(), "61,174,233".into()), ("DecorationHover".into(), "61,174,233".into())])).collect()
}

pub fn kde_plugin() -> bool {
    model::dry() || KDE_PLUGIN_GLOBS.iter().any(|g| packages::any_glob(g))
}

/// Welches Qt-Plattform-Theme in Hyprland gesetzt werden soll: kde (plasma-integration), qt6ct oder keins.
pub fn qt_platform_theme() -> &'static str {
    if kde_plugin() {
        return "kde";
    }
    if util::has("qt6ct") {
        return "qt6ct";
    }
    ""
}

pub fn apply_kde(mode: &str, cfg_home: &Path, log: Log) {
    let (file_name, display) = scheme(mode);
    let mut updates = scheme_colors(mode);
    match updates.iter_mut().find(|(n, _)| n == "General") {
        Some((_, kv)) => {
            match kv.iter_mut().find(|(k, _)| k == "ColorScheme") {
                Some(e) => e.1 = file_name.into(),
                None => kv.push(("ColorScheme".into(), file_name.into())),
            }
        }
        None => updates.push(("General".into(), vec![("ColorScheme".into(), file_name.into())])),
    }
    update_ini(&cfg_home.join("kdeglobals"), &updates, log, true);
    if model::dry() || util::has("plasma-apply-colorscheme") {
        model::run(&["plasma-apply-colorscheme", file_name], log, true);
    }
    // jede KDE-App einzeln (gilt auch ohne Plattform-Theme): nur vorhandene rc-Dateien + Kate/KWrite immer
    let mut n = 0;
    for app in KDE_APPS {
        let f = cfg_home.join(format!("{}rc", app));
        if f.is_file() || *app == "kate" || *app == "kwrite" {
            update_ini(&f, &upd("UiSettings", &[("ColorScheme", display)]), log, false);
            n += 1;
        }
    }
    log(&format!("KDE-Apps: {} × ColorScheme={}", n, display));
}

/// qt5ct/qt6ct: eigene Palette (darker = dunkel), nur wenn das Programm da ist.
pub fn apply_qtct(mode: &str, cfg_home: &Path, log: Log) {
    for tool in ["qt6ct", "qt5ct"] {
        if !(model::dry() || util::has(tool)) {
            continue;
        }
        let mut path = format!("/usr/share/{}/colors/darker.conf", tool);
        if mode == "dark" && tool == "qt6ct" {
            if let Some(f) = scheme_file("dark") {
                path = f.to_string_lossy().replace('\\', "/");
            }
        }
        let u = upd("Appearance", &[("custom_palette", if mode == "dark" { "true" } else { "false" }),
            ("color_scheme_path", &path)]);
        update_ini(&cfg_home.join(tool).join(format!("{}.conf", tool)), &u, log, true);
    }
}

// ---------- Portal, Umgebung, Flatpak (damit wirklich jede App dunkel wird) ----------

pub fn portals_conf() -> String {
    format!("# {} – Hell/Dunkel für alle Apps (Sonstiges › Hell / Dunkel)\n[preferred]\ndefault=hyprland;gtk\n\
org.freedesktop.impl.portal.Settings=gtk\n", MARK)
}

pub const DBUS_ENV: &str = "dbus-update-activation-environment --systemd --all";

/// hyprland-portals.conf anlegen; eine eigene, fremde Datei bleibt unangetastet.
pub fn write_portals_conf(cfg_home: &Path, log: Log) -> bool {
    let f = cfg_home.join("xdg-desktop-portal").join("hyprland-portals.conf");
    if f.is_file() && !util::read_text(&f).contains(MARK) {
        log(&format!("Eigene Portal-Einstellung bleibt: {}", f.display()));
        return false;
    }
    let want = portals_conf();
    if !f.is_file() || util::read_text(&f) != want {
        let _ = util::write_text(&f, &want);
        log(&format!("→ {}", f.display()));
    }
    true
}

pub fn gtk_portal() -> bool {
    model::dry() || ["/usr/lib/xdg-desktop-portal-gtk", "/usr/libexec/xdg-desktop-portal-gtk",
        "/usr/share/xdg-desktop-portal/portals/gtk.portal"].iter().any(|p| Path::new(p).exists())
}

/// hyprstudio_look.lua/.conf: Qt-Thema und D-Bus-Umgebung. Kein GTK_THEME: das schaltet libadwaita-Stile ab
/// (kaputte GNOME-Apps); GTK/libadwaita folgen color-scheme übers Portal.
pub fn env_content(_mode: &str, qt: &str, fmt: &str) -> String {
    let c = if fmt == "lua" { "--" } else { "#" };
    let mut out = vec![format!("{} {} – Hell/Dunkel, automatisch erzeugt (Sonstiges › Hell / Dunkel)", c, MARK)];
    if fmt == "lua" {
        if !qt.is_empty() {
            out.push(format!("hl.env(\"QT_QPA_PLATFORMTHEME\", \"{}\")", qt));
        }
        out.push("hl.on(\"hyprland.start\", function ()".into());
        out.push(format!("    hl.exec_cmd(\"{}\")", DBUS_ENV));
        out.push("end)".into());
    } else {
        if !qt.is_empty() {
            out.push(format!("env = QT_QPA_PLATFORMTHEME,{}", qt));
        }
        out.push(format!("exec-once = {}", DBUS_ENV));
    }
    out.join("\n") + "\n"
}

pub fn apply_env(mode: &str, cfg_home: &Path, log: Log) -> bool {
    let qt = qt_platform_theme();
    laptopkeys::include_extra(&cfg_home.join("hypr"), "look", &|fmt| env_content(mode, qt, fmt), "Hell/Dunkel", log)
        .is_some()
}

/// Früher: GTK_THEME/QT_QPA_PLATFORMTHEME in ~/.config/environment.d – jetzt nur noch aufräumen.
pub fn apply_services(_mode: &str, cfg_home: &Path, log: Log) {
    let f = cfg_home.join("environment.d").join("60-hyprstudio-look.conf");
    if f.is_file() && util::read_text(&f).contains(MARK) {
        let _ = std::fs::remove_file(&f);
        log(&format!("− {}", f.display()));
        if model::dry() || util::has("systemctl") {
            model::run(&["systemctl", "--user", "unset-environment", "GTK_THEME", "QT_QPA_PLATFORMTHEME"], log, true);
        }
    }
}

/// Kein GTK_THEME für Flatpak-Apps mehr (bricht libadwaita); eine frühere Überschreibung wieder entfernen.
pub fn apply_flatpak(_mode: &str, log: Log) {
    if !(model::dry() || util::has("flatpak")) {
        return;
    }
    model::run(&["flatpak", "override", "--user", "--unset-env=GTK_THEME"], log, true);
}

pub fn apply(mode: &str, cfg_home: &Path, log: Log) -> bool {
    let dark = mode == "dark";
    for d in ["gtk-3.0", "gtk-4.0"] {
        set_ini(&cfg_home.join(d).join("settings.ini"), "gtk-application-prefer-dark-theme",
            if dark { "1" } else { "0" }, log);
    }
    set_ini(&cfg_home.join("gtk-3.0").join("settings.ini"), "gtk-theme-name", if dark { "Adwaita-dark" } else { "Adwaita" },
        log);
    if model::dry() || util::has("gsettings") {
        model::run(&["gsettings", "set", "org.gnome.desktop.interface", "color-scheme",
            if dark { "prefer-dark" } else { "prefer-light" }], log, true);
        model::run(&["gsettings", "set", "org.gnome.desktop.interface", "gtk-theme",
            if dark { "Adwaita-dark" } else { "Adwaita" }], log, true);
    }
    apply_kde(mode, cfg_home, log);
    apply_qtct(mode, cfg_home, log);
    write_portals_conf(cfg_home, log);
    apply_env(mode, cfg_home, log);
    apply_flatpak(mode, log);
    apply_services(mode, cfg_home, log);
    if model::dry() || util::has("systemctl") {
        // laufende Apps bekommen den Wechsel sofort übers Portal mit
        model::run(&["systemctl", "--user", "try-restart", "xdg-desktop-portal.service",
            "xdg-desktop-portal-gtk.service"], log, true);
    }
    if model::dry() || util::has("hyprctl") {
        model::run(&["hyprctl", "reload"], log, true);
    }
    true
}
