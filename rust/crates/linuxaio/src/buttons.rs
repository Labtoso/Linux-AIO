//! Knopf-Themes für die Titelleiste (GTK 3 + GTK 4/libadwaita): eigener CSS-Block in ~/.config/gtk-*/gtk.css
//! zwischen festen Markern – der Rest der Datei bleibt unangetastet. Dazu Firefox/Zen (userChrome.css).

use crate::usercmd;
use hyprstudio::jsonx::{Get, Obj};
use hyprstudio::{palettes, util};
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub const START: &str = "/* Linux AIO buttons start";
pub const END: &str = "/* Linux AIO buttons end */";

fn own() -> Obj {
    let v = json!({
        "macos": {"titel": "macOS", "shape": "circle", "size": 14, "close": "#ff5f57", "min": "#febc2e",
                  "max": "#28c840", "icons": "hover"},
        "round": {"titel": "Rund", "shape": "circle", "size": 24, "close": "#3a424d", "min": "#3a424d",
                  "max": "#3a424d", "icons": "always", "icon_color": "#e6edf5"},
        "square": {"titel": "Eckig", "shape": "square", "size": 24, "close": "#3a424d", "min": "#3a424d",
                   "max": "#3a424d", "icons": "always", "icon_color": "#e6edf5"},
        "minimal": {"titel": "Minimal", "shape": "flat", "size": 24, "close": "", "min": "", "max": "",
                    "icons": "always"},
        "windows": {"titel": "Windows", "shape": "wide", "size": 30, "close": "", "min": "", "max": "",
                    "hover_close": "#c42b1c", "icons": "always"},
        "material": {"titel": "Material", "shape": "circle", "size": 22, "close": "#ef5350", "min": "#ffca28",
                     "max": "#66bb6a", "icons": "always"},
        "catppuccin": {"titel": "Catppuccin", "shape": "circle", "size": 16, "close": "#f38ba8", "min": "#f9e2af",
                       "max": "#a6e3a1", "icons": "hover"},
        "nord": {"titel": "Nord", "shape": "circle", "size": 16, "close": "#bf616a", "min": "#ebcb8b",
                 "max": "#a3be8c", "icons": "hover"}
    });
    v.as_object().cloned().unwrap_or_default()
}

/// palettes.unify: gemeinsame Namen/Farben zuerst, eigene Nicht-Farb-Werte bleiben, dann die übrigen.
fn unify(old: &Obj, make: impl Fn(&Obj) -> Obj) -> Obj {
    let aliases = [("01", "tokyo")];
    let mut out = Obj::new();
    for key in palettes::order() {
        let colours = make(palettes::palette(key));
        let alias = aliases.iter().find(|(a, _)| a == key).map(|(_, b)| *b).unwrap_or("");
        let src = old.get(key.as_str()).or_else(|| old.get(alias)).and_then(|v| v.as_object()).cloned().unwrap_or_default();
        let mut o = Obj::new();
        for (k, v) in src {
            if !colours.contains_key(&k) {
                o.insert(k, v);
            }
        }
        for (k, v) in colours {
            o.insert(k, v);
        }
        out.insert(key.clone(), Value::Object(o));
    }
    let gone: Vec<String> = palettes::order().iter().cloned().chain(aliases.iter().map(|(_, b)| b.to_string())).collect();
    for (k, v) in old {
        if !gone.contains(k) {
            out.insert(k.clone(), v.clone());
        }
    }
    out
}

pub static THEMES: Lazy<Obj> = Lazy::new(|| {
    unify(&own(), |p| {
        let mut o = Obj::new();
        o.insert("titel".into(), json!(p.s("title")));
        o.insert("shape".into(), json!("circle"));
        o.insert("size".into(), json!(16));
        o.insert("close".into(), json!(p.s("red")));
        o.insert("min".into(), json!(p.s("yellow")));
        o.insert("max".into(), json!(p.s("green")));
        o.insert("icons".into(), json!("hover"));
        o
    })
});

pub fn theme(name: &str) -> Option<&'static Obj> {
    THEMES.get(name).and_then(|v| v.as_object())
}

pub fn names() -> Vec<String> {
    THEMES.keys().cloned().collect()
}

fn size_of(p: &Obj) -> i64 {
    p.get("size").and_then(|v| v.as_i64()).unwrap_or(24)
}

fn rules(sel_btn: &str, sel_img: &str, p: &Obj, gtk: u8) -> Vec<String> {
    let size = size_of(p);
    let shape = p.s("shape");
    let img = format!("{sel_btn} {sel_img}");
    let mut out = vec![format!("{sel_btn} {{ background: none; box-shadow: none; border: none; outline: none; \
        min-width: 0; min-height: 0; padding: 0 {}px; margin: 0; }}", if shape != "wide" { 4 } else { 0 })];
    if shape == "circle" || shape == "square" {
        // Die Form wird als Hintergrundbild fester Größe gemalt (mittig, ohne Wiederholung). So bleibt der Knopf
        // ein Kreis, auch wenn eine App (z. B. libadwaita/Shelly) das Symbol in der Höhe streckt – vorher
        // wurden daraus dunkle Ovale ohne Symbol.
        let icon = if gtk == 4 { (size - 8).clamp(8, 16) } else { 16 };
        let r = size as f64 / 2.0;
        let size_rule = if gtk == 4 { format!(" -gtk-icon-size: {icon}px;") } else { String::new() };
        out.push(format!("{img} {{ background-color: transparent; background-repeat: no-repeat; \
            background-position: center; background-size: {size}px {size}px; border-radius: 0; padding: 0; \
            min-width: {size}px; min-height: {size}px; box-shadow: none; opacity: 1;{size_rule} }}"));
        for (cls, key) in [("close", "close"), ("minimize", "min"), ("maximize", "max")] {
            let c = p.s(key);
            let c = if c.is_empty() { "transparent" } else { c };
            let paint = if shape == "circle" {
                format!("radial-gradient(circle {r}px at center, {c} 0, {c} {:.1}px, transparent {r}px)", r - 0.6)
            } else {
                format!("linear-gradient({c}, {c})")
            };
            out.push(format!("{sel_btn}.{cls} {sel_img} {{ background-image: {paint}; }}"));
        }
        let dark = "rgba(0,0,0,0.65)";
        let light = p.s("icon_color");
        match p.s("icons") {
            "hover" => {
                out.push(format!("{img} {{ color: transparent; }}"));
                out.push(format!("{sel_btn}:hover {sel_img} {{ color: {}; }}", if light.is_empty() { dark } else { light }));
            }
            "never" => out.push(format!("{img} {{ color: transparent; }}")),
            _ => out.push(format!("{img} {{ color: {}; }}", if light.is_empty() { dark } else { light })),
        }
        let hover = if gtk == 4 { "filter: brightness(1.18);" } else { "opacity: 0.85;" };
        out.push(format!("{sel_btn}:hover {sel_img}, {sel_btn}:active {sel_img} {{ {hover} }}"));
    } else if shape == "wide" {
        // Windows: breite Flächen, Schließen wird rot
        out.push(format!("{sel_btn} {{ min-width: {}px; border-radius: 0; }}", size + 16));
        out.push(format!("{img} {{ background: none; box-shadow: none; }}"));
        out.push(format!("{sel_btn}:hover {{ background-color: rgba(127,127,127,0.18); }}"));
        let hc = p.s("hover_close");
        if !hc.is_empty() {
            out.push(format!("{sel_btn}.close:hover {{ background-color: {hc}; color: #ffffff; }}"));
        }
    } else {
        // flat: nur Symbole
        out.push(format!("{img} {{ background: none; box-shadow: none; }}"));
        out.push(format!("{sel_btn}:hover {sel_img} {{ background-color: rgba(127,127,127,0.18); border-radius: 6px; }}"));
    }
    out
}

pub fn css(name: &str, gtk: u8) -> String {
    let Some(p) = theme(name) else { return String::new() };
    let r = if gtk == 4 {
        rules("windowcontrols > button", "> image", p, 4)
    } else {
        let mut a = rules("headerbar button.titlebutton", "image", p, 3);
        a.extend(rules(".titlebar button.titlebutton", "image", p, 3));
        a
    };
    let mut lines = vec![format!("{START}: {name} */")];
    lines.extend(r);
    lines.push(END.into());
    lines.join("\n")
}

pub fn files() -> Vec<PathBuf> {
    vec![usercmd::config_dir().join("gtk-4.0").join("gtk.css"), usercmd::config_dir().join("gtk-3.0").join("gtk.css")]
}

static BLOCK: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&format!(r"(?s){}.*?{}\n?", regex::escape(START), regex::escape(END))).unwrap()
});

fn strip(text: &str) -> String {
    BLOCK.replace_all(text, "").into_owned()
}

fn read(p: &Path) -> String {
    if p.is_file() { util::read_text(p) } else { String::new() }
}

fn write(p: &Path, text: &str) {
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::write(p, text);
}

pub fn apply(name: &str, log: &dyn Fn(&str)) {
    for (f, gtk) in files().iter().zip([4u8, 3]) {
        let old = read(&usercmd::readable(f));
        let body = strip(&old).trim_end_matches('\n').to_string();
        let new = format!("{}{}\n", if body.is_empty() { String::new() } else { format!("{body}\n\n") }, css(name, gtk));
        write(f, &new);
        log(&format!("→ {}: Knopf-Theme {}", f.display(), name));
    }
}

fn cleaned(old: &str) -> String {
    let new = strip(old);
    if new.trim().is_empty() { String::new() } else { format!("{}\n", new.trim_end_matches('\n')) }
}

pub fn remove(log: &dyn Fn(&str)) {
    for f in files() {
        let src = usercmd::readable(&f);
        if !src.is_file() {
            continue;
        }
        let old = util::read_text(&src);
        let new = cleaned(&old);
        if new != old {
            write(&f, &new);
            log(&format!("→ {}: Knopf-Theme entfernt", f.display()));
        }
    }
}

pub fn current() -> String {
    let f = usercmd::readable(&files()[0]);
    let Ok(text) = std::fs::read(&f) else { return String::new() };
    let text = String::from_utf8_lossy(&text);
    let re = Regex::new(&format!(r"{}: (\w+) \*/", regex::escape(START))).unwrap();
    re.captures(&text).map(|c| c[1].to_string()).unwrap_or_default()
}

// ---------- Firefox / Zen (userChrome.css) ----------

pub const MOZ_DIRS: [&str; 4] = [".config/mozilla/firefox", ".mozilla/firefox", ".config/zen", ".zen"];
pub const PREF: &str = "user_pref(\"toolkit.legacyUserProfileCustomizations.stylesheets\", true); // Linux AIO";

/// Profilordner von Firefox und Zen aus ihrer profiles.ini.
pub fn moz_profiles() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = vec![];
    let re = Regex::new(r"(?m)^Path=(.+)$").unwrap();
    for rel in MOZ_DIRS {
        let base = usercmd::real_home().join(rel);
        let Ok(text) = std::fs::read(base.join("profiles.ini")) else { continue };
        let text = String::from_utf8_lossy(&text);
        for c in re.captures_iter(&text) {
            let v = c[1].trim();
            let p = if v.starts_with('/') { PathBuf::from(v) } else { base.join(v) };
            if p.is_dir() && !out.contains(&p) {
                out.push(p);
            }
        }
    }
    out
}

pub fn moz_css(name: &str) -> Option<String> {
    let p = theme(name)?;
    let shape = p.s("shape");
    if shape != "circle" && shape != "square" {
        return None; // breite/flache Themes: Firefox behält seine Knöpfe
    }
    let size = size_of(p);
    let radius = if shape == "circle" { "50%" } else { "4px" };
    let mut rules = vec![
        ".titlebar-buttonbox { margin-inline: 4px !important; }".to_string(),
        ".titlebar-button { appearance: none !important; background: none !important; padding: 0 5px !important; }".to_string(),
        format!(".titlebar-button > .toolbarbutton-icon {{ appearance: none !important; width: {size}px !important; \
            height: {size}px !important; border-radius: {radius} !important; padding: 0 !important; \
            list-style-image: none !important; }}"),
    ];
    for (cls, key) in [("titlebar-close", "close"), ("titlebar-min", "min"), ("titlebar-max", "max"), ("titlebar-restore", "max")] {
        let c = p.s(key);
        rules.push(format!(".{cls} > .toolbarbutton-icon {{ background-color: {} !important; }}",
            if c.is_empty() { "transparent" } else { c }));
    }
    rules.push(".titlebar-button:hover > .toolbarbutton-icon { filter: brightness(1.15); }".into());
    let mut lines = vec![format!("{START}: {name} */")];
    lines.extend(rules);
    lines.push(END.into());
    Some(lines.join("\n"))
}

/// Ziel im Testmodus unter dem Testordner, sonst im echten Profil.
pub fn moz_targets(profile: &Path) -> (PathBuf, PathBuf) {
    if usercmd::dry() {
        let base = usercmd::dry_home().join("moz").join(util::file_name(profile));
        return (base.join("chrome").join("userChrome.css"), base.join("user.js"));
    }
    (profile.join("chrome").join("userChrome.css"), profile.join("user.js"))
}

pub fn apply_moz(name: &str, log: &dyn Fn(&str)) -> usize {
    let css = moz_css(name);
    let mut n = 0;
    for prof in moz_profiles() {
        let (chrome, userjs) = moz_targets(&prof);
        let Some(css) = &css else {
            remove_moz_one(&chrome, &userjs);
            continue;
        };
        let src = if chrome.is_file() { chrome.clone() } else { prof.join("chrome").join("userChrome.css") };
        let old = read(&src);
        let body = strip(&old).trim_end_matches('\n').to_string();
        write(&chrome, &format!("{}{}\n", if body.is_empty() { String::new() } else { format!("{body}\n\n") }, css));
        let js_src = if userjs.is_file() { userjs.clone() } else { prof.join("user.js") };
        let js = read(&js_src);
        if !js.contains(PREF) {
            let base = js.trim_end_matches('\n');
            write(&userjs, &format!("{}{}{}\n", base, if js.trim().is_empty() { "" } else { "\n" }, PREF));
        }
        log(&format!("→ {}", chrome.display()));
        n += 1;
    }
    n
}

fn remove_moz_one(chrome: &Path, userjs: &Path) {
    if chrome.is_file() {
        let old = util::read_text(chrome);
        let new = cleaned(&old);
        if new != old {
            let _ = std::fs::write(chrome, new);
        }
    }
    if userjs.is_file() {
        let js = util::read_text(userjs);
        if js.contains(PREF) {
            let kept: Vec<&str> = js.lines().filter(|l| l.trim() != PREF).collect();
            let _ = std::fs::write(userjs, format!("{}\n", kept.join("\n")));
        }
    }
}

pub fn remove_moz(_log: &dyn Fn(&str)) {
    for prof in moz_profiles() {
        let (c, u) = moz_targets(&prof);
        remove_moz_one(&c, &u);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn strip_and_css() {
        let c = super::css("macos", 4);
        assert!(c.starts_with("/* Linux AIO buttons start: macos */"));
        let text = format!("a {{}}\n\n{}\n", c);
        assert_eq!(super::strip(&text), "a {}\n\n");
        assert!(super::THEMES.contains_key("macos"));
        // feste Kreisgröße statt border-radius auf gestreckter Fläche
        let r = super::css("round", 4);
        assert!(r.contains("radial-gradient(circle 12px at center"), "{r}");
        assert!(r.contains("background-size: 24px 24px"));
    }
}
