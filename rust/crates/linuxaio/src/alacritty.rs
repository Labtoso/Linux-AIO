//! Alacritty-Einstellungen: Zustand des Kitty-Editors (gleiche Presets, gleiche Optionen) → ~/.config/alacritty/alacritty.toml.
//! Alles, was Alacritty kann, wird übertragen; der Rest steht in UNSUPPORTED und ist im Editor gesperrt.
//! Eine fremde Datei wird einmal als alacritty.toml.linuxaio-original gesichert.

use crate::{pyjson, usercmd};
use hyprstudio::jsonx::{Get, Obj};
use hyprstudio::kitty;
use serde_json::Value;
use std::path::PathBuf;

pub const MARK: &str = "# Linux AIO – generated, overwritten when applied";
pub const NAMES: [&str; 8] = ["black", "red", "green", "yellow", "blue", "magenta", "cyan", "white"];
/// nur Alacritty (nicht im Kitty-Zustand): Cursor-Dicke (Anteil einer Zelle), Blinken endet nach n Sekunden
pub const EXTRA: [(&str, f64); 2] = [("thickness", 0.15), ("blink_timeout", 5.0)];
/// Kitty-Optionen ohne Gegenstück in Alacritty
pub const UNSUPPORTED: [&str; 18] = ["dynamic_opacity", "display_server", "ligatures", "blink_curve", "trail",
    "trail_decay_fast", "trail_decay_slow", "tab_edge", "tab_style", "tab_powerline", "tab_title", "url",
    "tab_active_bg", "tab_active_fg", "tab_inactive_bg", "tab_inactive_fg", "url_style", "audio_bell"];
pub const UNSUPPORTED2: [&str; 1] = ["confirm_close"];

pub fn unsupported(k: &str) -> bool {
    UNSUPPORTED.contains(&k) || UNSUPPORTED2.contains(&k)
}

/// Kitty-Zustand plus Alacritty-Extras; ohne gespeicherten Zustand Preset „01“.
pub fn state(saved: Option<&Obj>) -> Obj {
    let saved = saved.filter(|s| !s.is_empty());
    let mut s = match saved {
        Some(sv) => kitty::state(Some(sv)),
        None => kitty::preset("01", None),
    };
    for (k, v) in EXTRA {
        let x = saved.and_then(|sv| sv.get(k)).cloned().unwrap_or(hyprstudio::jsonx::f(v));
        let f = match &x {
            Value::Number(n) => n.as_f64().unwrap_or(v),
            _ => v,
        };
        s.insert(k.into(), hyprstudio::jsonx::f(f));
    }
    s
}

fn q(s: &str) -> String {
    pyjson::dumps(&Value::String(s.into()), None, false, true)
}

fn c(x: &str) -> String {
    kitty::c(x)
}

pub fn toml(s: &Obj) -> String {
    let pal: Vec<String> = s.strs("palette").iter().map(|x| c(x)).collect();
    let blink = s.n("blink") > 0.0;
    let shape = match s.s("cursor_shape") { "block" => "Block", "underline" => "Underline", _ => "Beam" };
    let pad = s.n("padding").trunc() as i64;
    let mut lines: Vec<String> = vec![
        MARK.into(), "".into(),
        "[window]".into(), format!("padding = {{ x = {}, y = {} }}", pad, pad),
        format!("opacity = {:.2}", s.n("opacity")), format!("blur = {}", if s.b("blur") { "true" } else { "false" }),
        format!("decorations = {}", q(if s.s("decorations") == "no" { "Full" } else { "None" })), "".into(),
        "[font]".into(), format!("size = {:.1}", s.n("font_size")), format!("normal = {{ family = {} }}", q(s.s("font"))), "".into(),
        "[cursor]".into(), format!("style = {{ shape = {}, blinking = {} }}", q(shape), q(if blink { "On" } else { "Off" })),
        format!("thickness = {:.2}", s.n("thickness").clamp(0.05, 1.0)), "unfocused_hollow = true".into(),
    ];
    if blink {
        lines.push(format!("blink_interval = {}", ((s.n("blink") * 1000.0).trunc() as i64).max(10)));
        lines.push(format!("blink_timeout = {}", s.n("blink_timeout").trunc() as i64));
    }
    lines.extend([
        "".into(), "[scrolling]".into(), format!("history = {}", (s.n("scrollback").trunc() as i64).min(100000)), "".into(),
        "[selection]".into(), format!("save_to_clipboard = {}", if s.b("copy_on_select") { "true" } else { "false" }), "".into(),
        "[bell]".into(), format!("duration = {}", if s.b("bell_flash") { 100 } else { 0 }), format!("color = {}", q(&c(s.s("fg")))), "".into(),
        "[colors.primary]".into(), format!("background = {}", q(&c(s.s("bg")))), format!("foreground = {}", q(&c(s.s("fg")))), "".into(),
        "[colors.cursor]".into(), format!("cursor = {}", q(&c(s.s("cursor")))), format!("text = {}", q(&c(s.s("bg")))), "".into(),
        "[colors.selection]".into(), format!("background = {}", q(&c(s.s("sel_bg")))), format!("text = {}", q(&c(s.s("sel_fg")))), "".into(),
        "[colors.normal]".to_string(),
    ]);
    lines.extend(NAMES.iter().zip(pal.iter().take(8)).map(|(n, v)| format!("{} = {}", n, q(v))));
    lines.push("".into());
    lines.push("[colors.bright]".into());
    lines.extend(NAMES.iter().zip(pal.iter().skip(8).take(8)).map(|(n, v)| format!("{} = {}", n, q(v))));
    let mut cmds: Vec<String> = if s.b("fastfetch") { vec!["fastfetch".into()] } else { vec![] };
    cmds.extend(s.strs("start_cmds"));
    if !cmds.is_empty() {
        // „Beim Öffnen“: Befehle, danach die normale Shell
        let line = cmds.join("; ") + "; exec \"${SHELL:-/bin/bash}\"";
        lines.push("".into());
        lines.push("[terminal]".into());
        lines.push(format!("shell = {{ program = \"/bin/sh\", args = [\"-c\", {}] }}", q(&line)));
    }
    lines.join("\n") + "\n"
}

pub fn target() -> PathBuf {
    usercmd::config_dir().join("alacritty").join("alacritty.toml")
}

pub fn backup() -> PathBuf {
    target().with_file_name("alacritty.toml.linuxaio-original")
}

pub fn apply(s: &Obj, log: &dyn Fn(&str)) -> PathBuf {
    let f = target();
    let _ = std::fs::create_dir_all(f.parent().unwrap());
    let src = usercmd::readable(&f);
    if src.is_file() {
        let old = hyprstudio::util::read_text(&src);
        if !old.starts_with(MARK) && !backup().exists() {
            let _ = hyprstudio::util::copy2(&src, &backup()); // fremde Config einmal sichern
            log(&format!("Backup: {}", backup().display()));
        }
    }
    let _ = std::fs::write(&f, toml(s));
    log(&format!("→ {}", f.display()));
    f
}

pub fn undo(log: &dyn Fn(&str)) -> bool {
    let (f, bak) = (target(), backup());
    if bak.is_file() {
        let _ = std::fs::copy(&bak, &f);
        log(&format!("{} → {}", bak.display(), f.display()));
        return true;
    }
    if f.is_file() && hyprstudio::util::read_text(&f).starts_with(MARK) {
        let _ = std::fs::remove_file(&f);
        log(&format!("− {}", f.display()));
        return true;
    }
    false
}
