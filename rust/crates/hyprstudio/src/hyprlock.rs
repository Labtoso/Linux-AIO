//! Hyprlock (Sperrbildschirm): Zustand, Presets und ~/.config/hypr/hyprlock.conf.

use crate::jsonx::{self, Get, Obj};
use crate::model::{self, rgba_hypr, Log, MARK};
use crate::util;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const BGS: &[&str] = &["screenshot", "wallpaper", "image", "color"];
pub const CLOCKS: &[&str] = &["24", "12", "sec"];
pub const LAYOUTS: &[&str] = &["center", "bottom_left", "bottom_right", "top_left", "left", "right", "top", "bottom",
    "split"];
pub const DATES: &[&str] = &["long", "short", "iso"];
const MARGIN_X: i64 = 90;
const MARGIN_Y: i64 = 70;
const GAP: i64 = 14;

type Group = (&'static str, &'static str, &'static [&'static str]);
const ALL5: &[&str] = &["avatar", "clock", "date", "greet", "input"];
const CD: &[&str] = &["clock", "date"];
const AGI: &[&str] = &["avatar", "greet", "input"];

fn groups(layout: &str) -> &'static [Group] {
    match layout {
        "bottom_left" => &[("left", "bottom", ALL5)],
        "bottom_right" => &[("right", "bottom", ALL5)],
        "top_left" => &[("left", "top", CD), ("left", "bottom", AGI)],
        "left" => &[("left", "center", ALL5)],
        "right" => &[("right", "center", ALL5)],
        "top" => &[("center", "top", CD), ("center", "bottom", AGI)],
        "bottom" => &[("center", "bottom", ALL5)],
        "split" => &[("left", "top", CD), ("right", "bottom", AGI)],
        _ => &[],
    }
}

pub fn date_fmt(k: &str) -> &'static str {
    match k {
        "short" => "%d.%m.%Y",
        "iso" => "%Y-%m-%d",
        _ => "%A, %d %B",
    }
}

static DEFAULT: Lazy<Obj> = Lazy::new(|| jsonx::obj(jsonx::parse(include_str!("../data/hyprlock.LOCK_DEFAULT.json"))));
static PRESETS: Lazy<Obj> = Lazy::new(|| jsonx::obj(jsonx::parse(include_str!("../data/hyprlock.PRESETS.json"))));

pub fn default() -> &'static Obj {
    &DEFAULT
}

pub fn presets() -> &'static Obj {
    &PRESETS
}

/// Schlüssel, die ein Preset festlegt (alles außer Verhalten und eigenen Texten/Pfaden).
pub fn look_keys() -> Vec<String> {
    const SKIP: &[&str] = &["lock_bind", "hide_cursor", "grace", "ignore_empty", "greet_text", "avatar_path", "bg_path",
        "placeholder"];
    DEFAULT.keys().filter(|k| !SKIP.contains(&k.as_str())).cloned().collect()
}

pub fn lock_state(saved: Option<&Obj>) -> Obj {
    let mut s = DEFAULT.clone();
    if let Some(saved) = saved {
        for (k, v) in saved {
            let Some(cur) = s.get(k) else { continue };
            if jsonx::same_type(v, cur) {
                s.insert(k.clone(), v.clone());
            } else if jsonx::is_float(cur) && jsonx::is_int(v) {
                s.insert(k.clone(), jsonx::f(v.as_f64().unwrap_or(0.0)));
            }
        }
    }
    for (k, opts) in [("bg", BGS), ("clock_fmt", CLOCKS), ("date_fmt", DATES), ("layout", LAYOUTS)] {
        if !opts.contains(&s.s(k)) {
            s.insert(k.into(), DEFAULT[k].clone());
        }
    }
    s
}

pub fn preset_state(name: &str, base: Option<&Obj>) -> Obj {
    let mut s = lock_state(base);
    for k in look_keys() {
        s.insert(k.clone(), DEFAULT[&k].clone());
    }
    if let Some(p) = PRESETS.get(name).and_then(|v| v.as_object()) {
        for (k, v) in p {
            s.insert(k.clone(), v.clone());
        }
    }
    s
}

/// Bild für background { path }: screenshot, das Hyprpaper-Bild oder ein eigenes Bild; leer = nur Farbe.
pub fn bg_path(s: &Obj, wallpaper: &str) -> String {
    match s.s("bg") {
        "screenshot" => "screenshot".into(),
        "wallpaper" => first_image(wallpaper),
        "image" => s.s("bg_path").into(),
        _ => String::new(),
    }
}

/// Hyprlock kann keinen Ordner: bei einer Diashow das erste Bild nehmen.
pub fn first_image(path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }
    let p = util::expanduser(path);
    if p.is_dir() {
        let mut imgs: Vec<PathBuf> = std::fs::read_dir(&p).map(|rd| {
            rd.flatten().map(|e| e.path()).filter(|f| crate::paper::is_image(f)).collect()
        }).unwrap_or_default();
        imgs.sort();
        return imgs.first().map(|f| f.display().to_string()).unwrap_or_default();
    }
    path.into()
}

fn q(text: &str) -> String {
    text.replace('\n', " ").trim().to_string()
}

pub type Pos = (i64, i64, &'static str, &'static str);

fn where_(p: &Pos) -> Vec<String> {
    vec![format!("    position = {}, {}", p.0, p.1), format!("    halign = {}", p.2), format!("    valign = {}", p.3)]
}

fn label(text: &str, color: &str, size: i64, font: &str, p: &Pos) -> Vec<String> {
    let mut out = vec!["label {".to_string(), "    monitor =".into(), format!("    text = {}", text),
        format!("    color = {}", rgba_hypr(color)), format!("    font_size = {}", size),
        format!("    font_family = {}", font)];
    if p.2 != "center" {
        out.push(format!("    text_align = {}", p.2));
    }
    out.extend(where_(p));
    out.push("}".into());
    out.push(String::new());
    out
}

/// Ungefähre Höhe jedes Elements in Pixeln (1080p) – zum Stapeln.
pub fn heights(s: &Obj) -> BTreeMap<&'static str, i64> {
    BTreeMap::from([
        ("clock", (s.n("clock_size") * 1.25) as i64),
        ("date", (s.n("date_size") * 1.5) as i64),
        ("greet", 24),
        ("avatar", s.i("avatar_size")),
        ("input", s.i("input_h")),
    ])
}

pub fn shown(s: &Obj) -> Vec<&'static str> {
    [("avatar", s.b("avatar")), ("clock", s.b("clock")), ("date", s.b("date")),
        ("greet", s.b("greet") && !s.s("greet_text").trim().is_empty()), ("input", true)]
        .into_iter().filter(|x| x.1).map(|x| x.0).collect()
}

/// Element → (x, y, halign, valign) wie in hyprlock: Abstand von der Ankerkante, y nach oben positiv.
pub fn place(s: &Obj) -> BTreeMap<&'static str, Pos> {
    let mut out = BTreeMap::new();
    if s.s("layout") == "center" {
        for (e, y) in layout(s) {
            out.insert(e, (0, y, "center", "center"));
        }
        return out;
    }
    let hs = heights(s);
    let on = shown(s);
    for (halign, valign, items) in groups(s.s("layout")) {
        let items: Vec<&'static str> = items.iter().copied().filter(|e| on.contains(e)).collect();
        let x = match *halign {
            "left" => MARGIN_X,
            "right" => -MARGIN_X,
            _ => 0,
        };
        match *valign {
            "bottom" => {
                let mut cur = MARGIN_Y;
                for e in items.iter().rev() {
                    out.insert(*e, (x, cur, *halign, *valign));
                    cur += hs[e] + GAP;
                }
            }
            "top" => {
                let mut cur = MARGIN_Y;
                for e in &items {
                    out.insert(*e, (x, -cur, *halign, *valign));
                    cur += hs[e] + GAP;
                }
            }
            _ => {
                let n = items.len() as i64;
                let mut top = (items.iter().map(|e| hs[e]).sum::<i64>() + GAP * (n - 1)) as f64 / 2.0;
                for e in &items {
                    out.insert(*e, (x, (top - hs[e] as f64 / 2.0) as i64, *halign, *valign));
                    top -= (hs[e] + GAP) as f64;
                }
            }
        }
    }
    out
}

/// y-Positionen (Mitte = 0, oben positiv) – gleich für Datei und Vorschau.
pub fn layout(s: &Obj) -> BTreeMap<&'static str, i64> {
    let mut y = BTreeMap::new();
    y.insert("input", s.i("input_y"));
    let mut top = s.i("input_y") + s.i("input_h").div_euclid(2) + 30;
    if s.b("greet") {
        y.insert("greet", top + 14);
        top += 50;
    }
    if s.b("avatar") {
        y.insert("avatar", top + s.i("avatar_size").div_euclid(2));
        top += s.i("avatar_size") + 30;
    }
    if s.b("date") {
        y.insert("date", top + s.i("date_size"));
        top += s.i("date_size") * 2 + 10;
    }
    if s.b("clock") {
        y.insert("clock", top + (s.n("clock_size") * 0.6) as i64);
    }
    y
}

pub fn clock_text(s: &Obj) -> String {
    match s.s("clock_fmt") {
        "12" => "$TIME12".into(),
        "sec" => "cmd[update:1000] date +\"%H:%M:%S\"".into(),
        _ => "$TIME".into(),
    }
}

fn tf(b: bool) -> &'static str {
    if b { "true" } else { "false" }
}

pub fn lock_conf(s: &Obj, wallpaper: &str) -> String {
    let mut font = q(s.s("font"));
    if font.is_empty() {
        font = "Sans".into();
    }
    let y = place(s);
    let mut out: Vec<String> = vec![format!("# {} – automatisch erzeugt, wird beim Anwenden überschrieben", MARK),
        String::new(), "general {".into(), format!("    hide_cursor = {}", tf(s.b("hide_cursor"))),
        format!("    grace = {}", jsonx::py_str(s.val("grace"))),
        format!("    ignore_empty_input = {}", tf(s.b("ignore_empty"))), "}".into(), String::new(),
        "background {".into(), "    monitor =".into()];
    let path = bg_path(s, wallpaper);
    if !path.is_empty() {
        out.push(format!("    path = {}", path));
    }
    out.extend([format!("    color = {}", rgba_hypr(s.s("bg_color"))),
        format!("    blur_passes = {}", jsonx::py_str(s.val("blur_passes"))),
        format!("    blur_size = {}", jsonx::py_str(s.val("blur_size"))),
        format!("    brightness = {:.2}", s.n("brightness")), format!("    contrast = {:.2}", s.n("contrast")),
        format!("    vibrancy = {:.2}", s.n("vibrancy")), "}".into(), String::new()]);
    if s.b("avatar") && !s.s("avatar_path").is_empty() {
        out.extend(["image {".to_string(), "    monitor =".into(), format!("    path = {}", s.s("avatar_path")),
            format!("    size = {}", jsonx::py_str(s.val("avatar_size"))), "    rounding = -1".into(),
            format!("    border_size = {}", jsonx::py_str(s.val("outline"))),
            format!("    border_color = {}", rgba_hypr(s.s("accent")))]);
        out.extend(where_(&y["avatar"]));
        out.extend(["}".to_string(), String::new()]);
    }
    if s.b("clock") {
        out.extend(label(&clock_text(s), s.s("text_color"), s.i("clock_size"), &font, &y["clock"]));
    }
    if s.b("date") {
        out.extend(label(&format!("cmd[update:60000] date +\"{}\"", date_fmt(s.s("date_fmt"))), s.s("text_color"),
            s.i("date_size"), &font, &y["date"]));
    }
    let greet = q(s.s("greet_text"));
    if s.b("greet") && !greet.is_empty() {
        out.extend(label(&greet, s.s("text_color"), 16, &font, &y["greet"]));
    }
    out.extend(["input-field {".to_string(), "    monitor =".into(),
        format!("    size = {}, {}", jsonx::py_str(s.val("input_w")), jsonx::py_str(s.val("input_h"))),
        format!("    outline_thickness = {}", jsonx::py_str(s.val("outline"))),
        format!("    outer_color = {}", rgba_hypr(s.s("accent"))),
        format!("    inner_color = {}", rgba_hypr(s.s("inner_color"))),
        format!("    font_color = {}", rgba_hypr(s.s("input_text"))), format!("    font_family = {}", font),
        format!("    check_color = {}", rgba_hypr(s.s("check_color"))),
        format!("    fail_color = {}", rgba_hypr(s.s("fail_color"))),
        format!("    rounding = {}", jsonx::py_str(s.val("rounding"))),
        format!("    placeholder_text = {}", q(s.s("placeholder"))), "    fail_text = $FAIL ($ATTEMPTS)".into(),
        format!("    fade_on_empty = {}", tf(s.b("fade_empty"))),
        format!("    dots_center = {}", tf(s.b("dots_center"))), "    dots_size = 0.25".into(),
        "    dots_spacing = 0.3".into()]);
    out.extend(where_(&y["input"]));
    out.push("}".into());
    out.join("\n") + "\n"
}

pub fn lock_file(hdir: &Path) -> PathBuf {
    hdir.join("hyprlock.conf")
}

pub fn apply_lock(s: &Obj, hdir: &Path, log: Log, wallpaper: &str) -> (bool, String) {
    if s.s("bg") == "image" && s.s("bg_path").is_empty() {
        return (false, "lock.nopath".into());
    }
    let _ = std::fs::create_dir_all(hdir);
    let f = lock_file(hdir);
    if let Some(bak) = model::backup_foreign(&f) {
        log(&format!("Sicherung: {}", bak.display()));
    }
    let _ = std::fs::write(&f, lock_conf(s, wallpaper));
    log(&format!("→ {}", f.display()));
    (true, String::new())
}

pub fn lock_bind() -> Obj {
    model::mk_bind(&["SUPER"], "L", "exec", "hyprlock", &[])
}

fn is_lock(b: &Value) -> bool {
    b.s("action") == "exec" && b.s("arg").trim() == "hyprlock"
}

pub fn has_lock_bind(binds: &[Value]) -> bool {
    binds.iter().any(is_lock)
}

pub fn set_lock_bind(binds: &[Value], on: bool) -> Vec<Value> {
    let mut rest: Vec<Value> = binds.iter().filter(|b| !is_lock(b)).cloned().collect();
    if on {
        rest.push(Value::Object(lock_bind()));
    }
    rest
}
