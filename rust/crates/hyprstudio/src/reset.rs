//! Alles zurücksetzen: Hypr Studio aus Hyprland, Waybar und Launchern herausnehmen, optional die offizielle
//! Standard-Config (Hyprland 0.55+, example/hyprland.lua) schreiben. Nichts wird gelöscht: jede Datei, die ersetzt
//! oder weggenommen wird, bleibt als Sicherung liegen.

use crate::model::{self, Log, MARK};
use crate::{fixer, kitty, util};
use std::path::{Path, PathBuf};

/// Offizielle Beispiel-Config von Hyprland (example/hyprland.lua), gekürzt um die Kommentar-Blöcke.
pub const DEFAULT_LUA: &str = include_str!("../data/reset.default.lua");

fn backup(f: &Path, log: Log) -> PathBuf {
    let b = model::stamp(f);
    let _ = util::copy2(f, &b);
    log(&format!("Sicherung: {}", b.display()));
    b
}

fn own_lines() -> Vec<String> {
    let m = MARK;
    let mut v = vec![format!("-- {}", m), format!("# {}", m)];
    v.extend(["require(\"hyprstudio\")", "source = ~/.config/hypr/hyprstudio.conf", "require(\"hyprstudio_laptop\")",
        "source = ~/.config/hypr/hyprstudio_laptop.conf", "require(\"hyprstudio_perf\")",
        "source = ~/.config/hypr/hyprstudio_perf.conf", "require(\"hyprstudio_look\")",
        "source = ~/.config/hypr/hyprstudio_look.conf"].iter().map(|s| s.to_string()));
    for what in ["Hell/Dunkel", "Leistung", "Laptop-Tasten"] {
        v.push(format!("-- {} – {}", m, what));
        v.push(format!("# {} – {}", m, what));
    }
    v
}

/// Nimmt die Hypr-Studio-Einbindung aus hyprland.lua/.conf (mit Sicherung).
pub fn remove_include(hdir: &Path, log: Log) -> usize {
    let own = own_lines();
    let mut n = 0;
    for name in ["hyprland.lua", "hyprland.conf"] {
        let f = hdir.join(name);
        if !f.is_file() {
            continue;
        }
        let text = util::read_text(&f);
        let lines = util::splitlines_keep(&text);
        let keep: Vec<&String> = lines.iter().filter(|l| !own.iter().any(|o| o == l.trim())).collect();
        if keep.len() != lines.len() {
            backup(&f, log);
            let _ = std::fs::write(&f, keep.into_iter().map(|s| s.as_str()).collect::<String>());
            log(&format!("Einbindung entfernt: {}", f.display()));
            n += 1;
        }
    }
    n
}

/// Hebt das Auskommentieren durch Hypr Studio auf (übernommene Kürzel, Fixer) – die Config ist wieder wie vorher.
pub fn uncomment(hdir: &Path, log: Log) -> usize {
    let re = regex::Regex::new(r"(?m)^(?:-- |# )\[Hypr Studio(?: Fix)?\] ").unwrap();
    let mut n = 0;
    for name in ["hyprland.lua", "hyprland.conf"] {
        let f = hdir.join(name);
        if !f.is_file() {
            continue;
        }
        let text = util::read_text(&f);
        let new = re.replace_all(&text, "").to_string();
        if new != text {
            backup(&f, log);
            let _ = std::fs::write(&f, &new);
            n += text.matches("[Hypr Studio").count();
            log(&format!("Wieder aktiv: {} Zeilen in {}", n, f.display()));
        }
    }
    n
}

/// Waybar und Launcher: Original zurück; gab es keins, wird die erzeugte Datei zur Sicherung umbenannt.
pub fn restore_originals(cfg_home: &Path, log: Log) -> usize {
    let mut n = 0;
    for rel in ["waybar/config.jsonc", "waybar/config.json", "waybar/config", "waybar/style.css", "rofi/config.rasi",
        "rofi/hyprstudio.rasi", "wofi/config", "wofi/style.css", "fuzzel/fuzzel.ini", "hypr/hyprlock.conf", "kdeglobals"] {
        let f = cfg_home.join(rel);
        let orig = util::add_suffix(&f, ".hyprstudio-original");
        if orig.is_file() {
            fixer::restore(&orig, &f, log);
            n += 1;
        } else if f.is_file() && util::read_text(&f).chars().take(300).collect::<String>().contains(MARK) {
            let _ = std::fs::rename(&f, model::stamp(&f));
            log(&format!("Beiseitegelegt: {}", f.display()));
            n += 1;
        }
    }
    n
}

/// Offizielle Standard-Config als hyprland.lua (vorhandene Datei wird gesichert).
pub fn write_default(hdir: &Path, log: Log) -> bool {
    let _ = std::fs::create_dir_all(hdir);
    let f = hdir.join("hyprland.lua");
    if f.is_file() {
        backup(&f, log);
    }
    let _ = std::fs::write(&f, DEFAULT_LUA);
    log(&format!("Standard-Config geschrieben: {}", f.display()));
    true
}

pub fn reset_all(cfg_home: &Path, factory: bool, log: Log) {
    let hdir = cfg_home.join("hypr");
    remove_include(&hdir, log);
    uncomment(&hdir, log);
    restore_originals(cfg_home, log);
    if kitty::remove_include(cfg_home, log) {
        model::run(&["pkill", "-USR1", "-x", "kitty"], log, true);
    }
    if factory {
        write_default(&hdir, log);
    }
    model::run(&["hyprctl", "reload"], log, false);
    model::restart_waybar(log);
}
