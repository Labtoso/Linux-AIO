//! Die eine Farbquelle für alle eingebauten Presets (Kitty, Alacritty, Hyprland, Waybar, Launcher, Sperrbildschirm,
//! Titelleisten-Knöpfe, App-Akzente): gleiche Namen, gleiche Reihenfolge, gleiche Farben überall.

use crate::jsonx::{Get, Obj};
use once_cell::sync::Lazy;
use serde_json::Value;

pub struct Palettes {
    pub order: Vec<String>,
    pub font: String,
    pub font_size: f64,
    pub palettes: Obj,
    pub legacy: Obj,
}

pub static P: Lazy<Palettes> = Lazy::new(|| {
    let v = crate::jsonx::parse(include_str!("../data/palettes.json"));
    Palettes {
        order: v.strs("order"),
        font: v.s("font").to_string(),
        font_size: v.n("font_size"),
        palettes: v.obj("palettes").cloned().unwrap_or_default(),
        legacy: v.obj("legacy").cloned().unwrap_or_default(),
    }
});

pub fn order() -> &'static [String] {
    &P.order
}

pub fn palette(key: &str) -> &'static Obj {
    P.palettes.get(key).and_then(|v| v.as_object()).unwrap_or_else(|| P.palettes["tokyo"].as_object().unwrap())
}

pub fn title(key: &str) -> String {
    palette(key).s("title").to_string()
}

/// Akzentfarben der Oberfläche (THEMES): Name → Farbe, in Reihenfolge.
pub fn themes() -> Vec<(String, String)> {
    P.order.iter().map(|k| (k.clone(), palette(k).s("ui").to_string())).collect()
}

pub fn theme_color(name: &str) -> Option<String> {
    P.palettes.get(name).map(|p| p.s("ui").to_string())
}

/// Alte Akzent-Namen → Palette.
pub fn legacy(name: &str) -> Option<String> {
    P.legacy.get(name).and_then(|v| v.as_str()).map(String::from)
}

/// Presets: erst die gemeinsamen Paletten (Farben aus make(palette) plus Extras je Name), dann die übrigen.
pub fn themed(make: impl Fn(&Obj) -> Obj, extras: &Obj, rest: &Obj) -> Obj {
    let mut out = Obj::new();
    for key in &P.order {
        let mut p = make(palette(key));
        if let Some(e) = extras.get(key).and_then(|v| v.as_object()) {
            for (k, v) in e {
                p.insert(k.clone(), v.clone());
            }
        }
        out.insert(key.clone(), Value::Object(p));
    }
    for (k, v) in rest {
        if !out.contains_key(k) {
            out.insert(k.clone(), v.clone());
        }
    }
    out
}
