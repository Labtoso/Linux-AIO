//! Kitty-Terminal: Zustand, Presets und ~/.config/kitty/hyprstudio.conf (per include in kitty.conf).

use crate::jsonx::{self, Get, Obj};
use crate::model::{self, Log, MARK};
use crate::util;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::path::{Path, PathBuf};

pub const INCLUDE: &str = "include hyprstudio.conf";
pub const START: &str = "hyprstudio-start.sh";

static DEFAULT: Lazy<Obj> = Lazy::new(|| jsonx::obj(jsonx::parse(include_str!("../data/kitty.KITTY_DEFAULT.json"))));
static PRESETS: Lazy<Obj> = Lazy::new(|| jsonx::obj(jsonx::parse(include_str!("../data/kitty.PRESETS.json"))));

pub const LOOK: &[&str] = &["palette", "fg", "bg", "cursor", "sel_fg", "sel_bg", "url", "tab_active_fg", "tab_active_bg",
    "tab_inactive_fg", "tab_inactive_bg", "opacity", "blur", "padding", "cursor_shape", "trail", "tab_style",
    "tab_powerline"];

pub fn default() -> &'static Obj {
    &DEFAULT
}

/// Presets (gemeinsame Paletten zuerst, dann eigene; „mine“ = alter Name von „01“)
pub fn presets() -> &'static Obj {
    &PRESETS
}

/// 16 Farben einer gemeinsamen Palette: color0 … color15
pub fn palette_colors(key: &str) -> Vec<String> {
    crate::palettes::palette(key).strs("ansi")
}

pub fn state(saved: Option<&Obj>) -> Obj {
    let mut s = DEFAULT.clone();
    if let Some(saved) = saved {
        for (k, v) in saved {
            let Some(cur) = s.get(k) else { continue };
            if k == "palette" {
                if let Some(a) = v.as_array() {
                    if a.len() == 16 && a.iter().all(|x| x.is_string()) {
                        s.insert(k.clone(), v.clone());
                    }
                }
            } else if k == "start_cmds" {
                if let Some(a) = v.as_array() {
                    let l: Vec<Value> = a.iter().filter_map(|x| x.as_str()).map(|x| x.trim())
                        .filter(|x| !x.is_empty()).map(|x| Value::String(x.into())).collect();
                    s.insert(k.clone(), Value::Array(l));
                }
            } else if jsonx::same_type(v, cur) {
                s.insert(k.clone(), v.clone());
            } else if jsonx::is_float(cur) && jsonx::is_int(v) {
                s.insert(k.clone(), jsonx::f(v.as_f64().unwrap_or(0.0)));
            }
        }
    }
    s
}

pub fn preset(name: &str, base: Option<&Obj>) -> Obj {
    let mut s = state(base);
    for k in LOOK.iter().chain(["display_server"].iter()) {
        s.insert(k.to_string(), DEFAULT[*k].clone());
    }
    if let Some(p) = PRESETS.get(name).and_then(|v| v.as_object()) {
        for (k, v) in p {
            s.insert(k.clone(), v.clone());
        }
    }
    s
}

fn c(x: &str) -> String {
    model::norm(x)[..7].to_string()
}

fn yn(b: bool) -> &'static str {
    if b { "yes" } else { "no" }
}

pub fn has_start(s: &Obj) -> bool {
    s.b("fastfetch") || !s.arr("start_cmds").is_empty()
}

/// Läuft bei jedem neuen Kitty-Fenster und startet danach die normale Shell.
pub fn start_script(s: &Obj) -> String {
    let mut out = vec!["#!/bin/sh".to_string(), format!("# {} – automatisch erzeugt, wird beim Anwenden überschrieben", MARK)];
    if s.b("fastfetch") {
        out.push("command -v fastfetch >/dev/null 2>&1 && fastfetch".into());
    }
    out.extend(s.strs("start_cmds"));
    out.push("sh_=\"${SHELL:-}\"".into());
    out.push(format!("case \"$sh_\" in \"\"|*{}) sh_=$(getent passwd \"$(id -un)\" | cut -d: -f7) ;; esac", START));
    out.push("exec \"${sh_:-/bin/sh}\"".into());
    out.join("\n") + "\n"
}

pub fn conf(s: &Obj, start: Option<&Path>) -> String {
    let nf = |k: &str| model::num_f(s.n(k));
    let blink = if s.n("blink") == 0.0 { "0".to_string() } else { format!("{} {}", nf("blink"), s.s("blink_curve")) };
    let p = |k: &str| jsonx::py_str(s.val(k));
    let mut lines = vec![
        format!("# {} – automatisch erzeugt, wird beim Anwenden überschrieben", MARK),
        String::new(),
        "# Schrift".into(),
        format!("font_family        {}", s.s("font")),
        format!("font_size          {}", nf("font_size")),
        format!("disable_ligatures  {}", s.s("ligatures")),
        String::new(),
        "# Transparenz".into(),
        format!("background_opacity          {}", nf("opacity")),
        format!("dynamic_background_opacity  {}", yn(s.b("dynamic_opacity"))),
        format!("background_blur             {}", p("blur")),
    ];
    if s.s("display_server") != "auto" {
        lines.push(format!("linux_display_server        {}", s.s("display_server")));
    }
    lines.extend([
        String::new(),
        "# Fenster".into(),
        format!("hide_window_decorations  {}", s.s("decorations")),
        format!("window_padding_width     {}", p("padding")),
        format!("confirm_os_window_close  {}", p("confirm_close")),
        format!("enable_audio_bell        {}", yn(s.b("audio_bell"))),
        format!("visual_bell_duration     {}", if s.b("bell_flash") { "0.2 ease-out" } else { "0" }),
        format!("scrollback_lines         {}", p("scrollback")),
        format!("url_style                {}", s.s("url_style")),
        format!("copy_on_select           {}", if s.b("copy_on_select") { "clipboard" } else { "no" }),
        String::new(),
        "# Cursor & Animation".into(),
        format!("cursor_shape             {}", s.s("cursor_shape")),
        format!("cursor_blink_interval    {}", blink),
        format!("cursor_trail             {}", p("trail")),
        format!("cursor_trail_decay       {} {}", nf("trail_decay_fast"), nf("trail_decay_slow")),
        String::new(),
        "# Tabs".into(),
        format!("tab_bar_edge          {}", s.s("tab_edge")),
        format!("tab_bar_style         {}", s.s("tab_style")),
        format!("tab_powerline_style   {}", s.s("tab_powerline")),
        format!("tab_title_template    \"{}\"", s.s("tab_title")),
        String::new(),
        "# Farben".into(),
        format!("foreground            {}", c(s.s("fg"))),
        format!("background            {}", c(s.s("bg"))),
        format!("selection_foreground  {}", c(s.s("sel_fg"))),
        format!("selection_background  {}", c(s.s("sel_bg"))),
        format!("cursor                {}", c(s.s("cursor"))),
        format!("url_color             {}", c(s.s("url"))),
        format!("active_tab_foreground   {}", c(s.s("tab_active_fg"))),
        format!("active_tab_background   {}", c(s.s("tab_active_bg"))),
        format!("inactive_tab_foreground {}", c(s.s("tab_inactive_fg"))),
        format!("inactive_tab_background {}", c(s.s("tab_inactive_bg"))),
        String::new(),
    ]);
    for (i, col) in s.strs("palette").iter().enumerate() {
        lines.push(format!("color{:<2} {}", i, c(col)));
    }
    if let Some(st) = start {
        lines.push(String::new());
        lines.push("# Beim Öffnen (Fastfetch / eigene Befehle)".into());
        lines.push(format!("shell {}", st.display()));
    }
    lines.join("\n") + "\n"
}

pub fn kdir(cfg_home: &Path) -> PathBuf {
    cfg_home.join("kitty")
}

pub fn apply(s: &Obj, cfg_home: &Path, log: Log) -> (bool, String) {
    let d = kdir(cfg_home);
    let _ = std::fs::create_dir_all(&d);
    let own = d.join("hyprstudio.conf");
    model::keep_previous(&own);
    let mut start = None;
    if has_start(s) {
        let st = d.join(START);
        let _ = std::fs::write(&st, start_script(s));
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&st, std::fs::Permissions::from_mode(0o755));
        log(&format!("→ {}", st.display()));
        start = Some(st);
    }
    let _ = std::fs::write(&own, conf(s, start.as_deref()));
    log(&format!("→ {}", own.display()));
    let main = d.join("kitty.conf");
    let text = if main.is_file() { util::read_text(&main) } else { String::new() };
    if !text.lines().any(|l| l.trim() == INCLUDE) {
        if main.is_file() {
            let bak = main.with_file_name("kitty.conf.hyprstudio-original");
            if !bak.exists() && util::copy2(&main, &bak).is_ok() {
                log(&format!("Sicherung: {}", bak.display()));
            }
        }
        let sep = if !text.is_empty() && !text.ends_with('\n') { "\n" } else { "" };
        model::append(&main, &format!("{}\n# {}\n{}\n", sep, MARK, INCLUDE));
        log(&format!("+ {}: {}", main.display(), INCLUDE));
    }
    // laufende Kitty-Fenster laden die Config neu (SIGUSR1)
    model::run(&["pkill", "-USR1", "-x", "kitty"], log, true);
    (true, String::new())
}

pub fn undo(cfg_home: &Path, log: Log) -> bool {
    let ok = model::undo_file(&kdir(cfg_home).join("hyprstudio.conf"));
    if ok {
        model::run(&["pkill", "-USR1", "-x", "kitty"], log, true);
    }
    ok
}

pub fn remove_include(cfg_home: &Path, log: Log) -> bool {
    let main = kdir(cfg_home).join("kitty.conf");
    if !main.is_file() {
        return false;
    }
    let lines = util::splitlines_keep(&util::read_text(&main));
    let mark = format!("# {}", MARK);
    let keep: Vec<&String> = lines.iter().filter(|l| { let t = l.trim(); t != INCLUDE && t != mark }).collect();
    if keep.len() == lines.len() {
        return false;
    }
    let _ = util::copy2(&main, &model::stamp(&main));
    let _ = std::fs::write(&main, keep.iter().map(|s| s.as_str()).collect::<String>());
    log(&format!("Einbindung entfernt: {}", main.display()));
    true
}
