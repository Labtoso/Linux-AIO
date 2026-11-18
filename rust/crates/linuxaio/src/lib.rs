//! Linux AIO – Desktops, SDDM, Plymouth und Desktop-Einstellungen an einem Ort (LabtosoStudios).
//!
//! Rust-Neufassung der früheren Python/Qt-Version: gleiche Dateien, gleiche Pfade, gleiches Verhalten.

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_ID: &str = "de.labtosostudios.linuxaio";
pub const APP_NAME: &str = "Linux AIO";
pub const REPO_URL: &str = "https://github.com/Labtoso/Linux-AIO";
pub const LICENSE: &str = "GPL-3.0-only";

pub mod i18n;
pub mod ini;
pub mod usercmd;
pub mod catalog;
pub mod system;
pub mod desktops;
pub mod safety;
pub mod sddm;
pub mod plymouth;
pub mod pyjson;
pub mod helper;
pub mod remote;
pub mod installers;
pub mod gnome;
pub mod looks;
pub mod fonts;
pub mod updater;
pub mod corners;
pub mod integrate;
pub mod alacritty;
pub mod yay;
pub mod presetfile;
pub mod buttons;
pub mod ui;
