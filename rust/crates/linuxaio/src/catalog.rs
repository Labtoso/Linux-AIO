//! Alle Desktop-Umgebungen mit je 3 Varianten (minimal, standard, komplett).
//!
//! Paket-Schreibweise pro Paketmanager:
//!   "a|b"          – erstes verfügbares nehmen
//!   "?a"           – optional, wird übersprungen wenn nicht verfügbar
//!   "@gruppe"      – dnf-Gruppe
//!   "pattern:name" – zypper-Pattern

use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;

pub const VARIANTS: [&str; 3] = ["minimal", "standard", "full"];
pub const BACKENDS: [&str; 4] = ["pacman", "apt", "dnf", "zypper"];

#[derive(Debug, Clone, Deserialize)]
pub struct Desktop {
    pub id: String,
    pub name: String,
    /// Namen der .desktop-Sessiondateien, bevorzugte zuerst
    pub sessions: Vec<String>,
    /// Werte von XDG_CURRENT_DESKTOP
    pub detect: Vec<String>,
    pub wayland: bool,
    pub x11: bool,
    pub tiling: bool,
    /// Wallpaper-Farbe der Vorschau
    pub color: String,
    /// Vorschau-Layout
    pub layout: String,
    pub desc: HashMap<String, String>,
    /// Variante → Paketmanager → Pakete
    pub packages: HashMap<String, HashMap<String, Vec<String>>>,
}

impl Desktop {
    pub fn specs(&self, variant: &str, backend: &str) -> Vec<String> {
        self.packages.get(variant).and_then(|m| m.get(backend)).cloned().unwrap_or_default()
    }

    pub fn supports(&self, variant: &str, backend: &str) -> bool {
        !self.specs(variant, backend).is_empty()
    }

    pub fn desc(&self, lang: &str) -> String {
        self.desc.get(lang).or_else(|| self.desc.get("de")).cloned().unwrap_or_default()
    }
}

pub static DESKTOPS: Lazy<Vec<Desktop>> =
    Lazy::new(|| serde_json::from_str(include_str!("../data/catalog.json")).expect("catalog"));

pub fn get(de_id: &str) -> Option<&'static Desktop> {
    DESKTOPS.iter().find(|d| d.id == de_id)
}
