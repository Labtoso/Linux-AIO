//! Eckenrundung der App-Fenster: eigener Block in ~/.config/gtk-4.0/gtk.css und gtk-3.0/gtk.css zwischen Markern.
//! Für alle Fenster (auch Nicht-GTK) optional die GNOME-Erweiterung „Rounded Window Corners Reborn“.

use crate::{gnome, usercmd};
use regex::Regex;
use std::path::PathBuf;

pub const START: &str = "/* Linux AIO corners start";
pub const END: &str = "/* Linux AIO corners end */";
pub const EXT_UUID: &str = "rounded-window-corners@fxgn";
pub const EXT_SCHEMA: &str = "org.gnome.shell.extensions.rounded-window-corners-reborn";
pub const EXT_KEY: &str = "global-rounded-corner-settings";

pub fn css(radius: i64, gtk: u8) -> String {
    let r = radius.clamp(0, 24);
    let rules: Vec<String> = if gtk == 4 {
        vec![
            format!(":root {{ --window-radius: {}px; }}", r), // libadwaita 1.6+
            format!("window.csd, window.csd.background, window.solid-csd {{ border-radius: {}px; }}", r),
            format!("window.csd headerbar, window.csd .titlebar {{ border-top-left-radius: {r}px; border-top-right-radius: {r}px; }}"),
        ]
    } else {
        vec![
            format!("decoration {{ border-radius: {}px; }}", r),
            // Inhalt nur unten rund, sonst entsteht unter der Titelleiste eine Lücke
            format!("window.csd, window.csd.background {{ border-radius: 0 0 {r}px {r}px; }}"),
            format!("headerbar.titlebar, .titlebar:not(headerbar) {{ border-top-left-radius: {r}px; border-top-right-radius: {r}px; }}"),
        ]
    };
    let mut all = vec![format!("{}: {} */", START, r)];
    all.extend(rules);
    all.push(END.into());
    all.join("\n")
}

pub fn files() -> Vec<PathBuf> {
    vec![usercmd::config_dir().join("gtk-4.0/gtk.css"), usercmd::config_dir().join("gtk-3.0/gtk.css")]
}

fn strip(text: &str) -> String {
    let re = Regex::new(&format!(r"(?s){}.*?{}\n?", regex::escape(START), regex::escape(END))).unwrap();
    re.replace_all(text, "").into_owned()
}

pub fn apply(radius: i64, log: &dyn Fn(&str)) {
    for (f, gtk) in files().into_iter().zip([4u8, 3]) {
        let src = usercmd::readable(&f);
        let old = if src.is_file() { hyprstudio::util::read_text(&src) } else { String::new() };
        let body = strip(&old).trim_end_matches('\n').to_string();
        let _ = std::fs::create_dir_all(f.parent().unwrap());
        let text = format!("{}{}\n", if body.is_empty() { String::new() } else { body + "\n\n" }, css(radius, gtk));
        let _ = std::fs::write(&f, text);
        log(&format!("→ {}: Eckenrundung {} px", f.display(), radius));
    }
}

pub fn remove(log: &dyn Fn(&str)) {
    for f in files() {
        let src = usercmd::readable(&f);
        if !src.is_file() {
            continue;
        }
        let old = hyprstudio::util::read_text(&src);
        let n = strip(&old);
        let new = if n.trim().is_empty() { String::new() } else { n.trim_end_matches('\n').to_string() + "\n" };
        if new != old {
            let _ = std::fs::create_dir_all(f.parent().unwrap());
            let _ = std::fs::write(&f, new);
            log(&format!("→ {}: Eckenrundung entfernt", f.display()));
        }
    }
}

pub fn current() -> Option<i64> {
    let text = std::fs::read(usercmd::readable(&files()[0])).ok()?;
    let text = String::from_utf8_lossy(&text);
    let re = Regex::new(&format!(r"{}: (\d+) \*/", regex::escape(START))).unwrap();
    re.captures(&text).and_then(|m| m[1].parse().ok())
}

/// gsettings mit dem eigenen Schema-Ordner der Erweiterung (ihr Schema ist nicht systemweit installiert).
fn gs(ext_dir: &Option<PathBuf>) -> Vec<String> {
    match ext_dir {
        Some(d) => vec!["gsettings".into(), "--schemadir".into(), d.join("schemas").display().to_string()],
        None => vec!["gsettings".into()],
    }
}

/// Radius der Erweiterung im a{sv}-Wörterbuch ändern (Rest bleibt, wie er ist).
pub fn set_extension_radius(radius: i64) -> bool {
    let d = gnome::ext_dir(EXT_UUID);
    if d.is_none() && !usercmd::dry() {
        return false;
    }
    let mut c = gs(&d);
    c.extend(["get".into(), EXT_SCHEMA.into(), EXT_KEY.into()]);
    let mut raw = usercmd::query_t(&c, 8).unwrap_or_default();
    if usercmd::dry() && raw.is_empty() {
        raw = "{'borderRadius': <uint32 15>}".into();
    }
    let re = Regex::new(r"('borderRadius':\s*<(?:uint32\s+)?)(\d+)(>)").unwrap();
    if !re.is_match(&raw) {
        return false;
    }
    let new = re.replace_all(&raw, |m: &regex::Captures| format!("{}{}{}", &m[1], radius, &m[3])).into_owned();
    let mut c = gs(&d);
    c.extend(["set".into(), EXT_SCHEMA.into(), EXT_KEY.into(), new]);
    usercmd::run_v(&c).0
}
