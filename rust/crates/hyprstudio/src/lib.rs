//! Hypr Studio – Hyprland, Waybar und App-Launcher per Klick gestalten (LabtosoStudios).
//!
//! Rust-Neufassung der früheren Python/PySide6-Version: gleiche Dateien, gleiche Pfade, gleiches Verhalten.
//! Die Logik (ohne Oberfläche) liegt in den Modulen oben, die GTK-Oberfläche unter [`ui`].

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_ID: &str = "de.labtosostudios.hyprstudio";
pub const APP_NAME: &str = "Hypr Studio";

pub mod jsonx;
pub mod i18n;
pub mod palettes;
pub mod model;
pub mod util;
pub mod net;
pub mod config;
pub mod fixer;
pub mod hardware;
pub mod packages;
pub mod fonts;
pub mod appearance;
pub mod laptopkeys;
pub mod sysactions;
pub mod kitty;
pub mod launcher;
pub mod hyprlock; pub mod paper; pub mod perf; pub mod reset;
pub mod online;
pub mod applauncher;
