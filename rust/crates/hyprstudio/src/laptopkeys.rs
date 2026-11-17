//! Laptop-Tasten automatisch einrichten – wie unter KDE, ohne dass jemand etwas einstellen muss.
//!
//! Eigene kleine Datei ~/.config/hypr/hyprstudio_laptop.lua (bzw. .conf) nur mit Helligkeit, Lautstärke, Stumm,
//! Mikrofon und Medien-Tasten, eingebunden in hyprland.lua/.conf. Der übrige Look bleibt unangetastet.
//! Läuft direkt nach der Installation (hypr-studio --laptop-keys) und bei jedem Start der App.

use crate::jsonx::{Get, Obj};
use crate::model::{self, Log, MARK};
use crate::{config, hardware, packages, sysactions, util};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub fn include(fmt: &str) -> (&'static str, &'static str) {
    if fmt == "lua" {
        ("require(\"hyprstudio_laptop\")", "--")
    } else {
        ("source = ~/.config/hypr/hyprstudio_laptop.conf", "#")
    }
}

pub fn file_for(hdir: &Path, fmt: &str) -> PathBuf {
    hdir.join(format!("hyprstudio_laptop.{}", fmt))
}

pub fn content(s: &Obj, fmt: &str) -> String {
    let c = include(fmt).1;
    let mut lines = vec![format!("{} {} – Laptop-Tasten, automatisch erzeugt (Sonstiges › Laptop-Tasten)", c, MARK)];
    if s.b("laptop_keys") {
        for b in model::laptop_binds(s) {
            if fmt == "lua" {
                lines.push(model::bind_lua(&b));
            } else {
                lines.extend(model::bind_conf(&b));
            }
        }
    } else {
        lines.push(format!("{} ausgeschaltet", c));
    }
    lines.join("\n") + "\n"
}

/// 1.26.0 schrieb die Tasten in hyprstudio.lua – dort entfernen (eigene, erzeugte Zeilen), sonst doppelt.
fn own_old_binds(hdir: &Path, fmt: &str, log: Log) {
    let f = hdir.join(format!("hyprstudio.{}", fmt));
    if !f.is_file() {
        return;
    }
    let lines = util::splitlines_keep(&util::read_text(&f));
    let drop = model::foreign_key_lines(&lines.concat(), fmt);
    if !drop.is_empty() {
        let kept: String = lines.iter().enumerate().filter(|(i, _)| !drop.contains(i)).map(|(_, l)| l.as_str()).collect();
        let _ = std::fs::write(&f, kept);
        log(&format!("Laptop-Tasten aus {} in {} verschoben", util::file_name(&f),
            util::file_name(&file_for(hdir, fmt))));
    }
}

/// Kleine Zusatzdatei hyprstudio_<stem>.lua/.conf schreiben und einmal in hyprland.lua/.conf einbinden.
/// Gibt das Format zurück (oder None, wenn es keine Hauptdatei gibt). Der übrige Look bleibt unangetastet.
pub fn include_extra(hdir: &Path, stem: &str, content_fn: &dyn Fn(&str) -> String, label: &str, log: Log)
    -> Option<String> {
    let fmt = model::hypr_format(hdir);
    let main = fmt.as_ref().map(|f| hdir.join(format!("hyprland.{}", f)));
    let (Some(fmt), Some(main)) = (fmt, main.filter(|m| m.is_file())) else {
        log(&format!("{}: keine hyprland.lua/.conf gefunden", label));
        return None;
    };
    let target = hdir.join(format!("hyprstudio_{}.{}", stem, fmt));
    let new = content_fn(&fmt);
    if !target.is_file() || util::read_text(&target) != new {
        let _ = std::fs::write(&target, &new);
        log(&format!("→ {}", target.display()));
    }
    let text = util::read_text(&main);
    let c = if fmt == "lua" { "--" } else { "#" };
    let line = if fmt == "lua" {
        format!("require(\"hyprstudio_{}\")", stem)
    } else {
        format!("source = ~/.config/hypr/hyprstudio_{}.conf", stem)
    };
    if !text.lines().any(|l| l.trim() == line) {
        let bak = util::add_suffix(&main, ".hyprstudio-original");
        if !bak.exists() && util::copy2(&main, &bak).is_ok() {
            log(&format!("Sicherung: {}", bak.display()));
        }
        let sep = if text.ends_with('\n') || text.is_empty() { "" } else { "\n" };
        model::append(&main, &format!("{}\n{} {} – {}\n{}\n", sep, c, MARK, label, line));
        log(&format!("+ {}: {}", main.display(), line));
    }
    Some(fmt)
}

/// Datei schreiben, einmal einbinden, gleiche Tasten in der Hauptdatei auskommentieren.
pub fn write(hdir: &Path, s: &Obj, log: Log) -> bool {
    let Some(fmt) = include_extra(hdir, "laptop", &|f| content(s, f), "Laptop-Tasten", log) else { return false };
    model::manage_laptop_keys(hdir, s, log);
    own_old_binds(hdir, &fmt, log);
    true
}

/// Auf Laptops einschalten, solange niemand selbst geschaltet hat; Datei aktuell halten.
/// Gibt zurück, ob sich am Zustand etwas geändert hat.
pub fn auto(s: &mut Obj, log: Log) -> bool {
    let mut changed = false;
    if !s.b("laptop_managed") && hardware::detect().laptop {
        s.insert("laptop_keys".into(), json!(true));
        s.insert("laptop_managed".into(), json!(true));
        changed = true;
    }
    let tool = hardware::volume_tool();
    if s.s("vol_tool") != tool {
        s.insert("vol_tool".into(), json!(tool));
        changed = true;
    }
    if s.b("laptop_managed") {
        let hdir = model::config_home().join("hypr");
        let target = file_for(&hdir, &model::hypr_format(&hdir).unwrap_or_else(|| "lua".into()));
        let fresh = !target.is_file();
        if write(&hdir, s, log) && (changed || fresh) {
            model::run(&["hyprctl", "reload"], log, true);
        }
    }
    changed
}

/// hypr-studio --laptop-keys: ohne Fenster einrichten (läuft nach der Installation).
pub fn cli() -> i32 {
    let mut cfg = config::load_raw().unwrap_or_else(|| crate::jsonx::obj(json!({"lang": "en", "theme": "01"})));
    let fmt = cfg.s("hypr_format").to_string();
    if fmt == "lua" || fmt == "conf" {
        model::set_format(&fmt);
    }
    let mut s = model::hypr_state(cfg.obj("hypr"));
    let dev = hardware::detect();
    println!("Hypr Studio: {} ({})", hardware::label(&dev), if dev.laptop { "Laptop" } else { "kein Laptop" });
    let pr = |l: &str| println!("{}", l);
    auto(&mut s, &pr);
    if cfg.get("sys_actions").map(crate::jsonx::truthy).unwrap_or(true) {
        // Herunterfahren/Neustart … gleich nach der Installation in allen Launchern
        sysactions::apply(true, &pr, packages::installed("hyprlock"));
    }
    cfg.insert("hypr".into(), Value::Object(s));
    if let Err(e) = config::save_raw(&cfg) {
        println!("{}", e);
    }
    0
}
