//! Gerät erkennen (Hersteller, Modell, Laptop?) und die Befehle für Helligkeit/Lautstärke wählen.
//!
//! Quelle: /sys/class/dmi/id – Lenovo schreibt den echten Modellnamen in product_version (product_name ist nur
//! eine Nummer wie „82LM“). Laptop = Gehäusetyp Notebook/Laptop/Convertible … oder es gibt einen Akku/Backlight.

use crate::model;
use std::path::Path;

const DMI: &str = "/sys/class/dmi/id";
/// Portable, Laptop, Notebook, Hand Held, Sub Notebook, Tablet …
const LAPTOP_CHASSIS: &[i64] = &[8, 9, 10, 11, 14, 30, 31, 32];
const GENERIC: &[&str] = &["", "to be filled by o.e.m.", "default string", "system product name", "not applicable",
    "none", "0"];

fn vendor_name(raw: &str) -> Option<&'static str> {
    Some(match raw {
        "LENOVO" => "Lenovo", "HP" | "HEWLETT-PACKARD" => "HP", "DELL INC." => "Dell",
        "ASUSTEK COMPUTER INC." => "ASUS", "ACER" => "Acer", "MICRO-STAR INTERNATIONAL CO., LTD." => "MSI",
        "SAMSUNG ELECTRONICS CO., LTD." => "Samsung", "MICROSOFT CORPORATION" => "Microsoft",
        "FRAMEWORK" => "Framework", "HUAWEI" => "Huawei", "XIAOMI" => "Xiaomi", "APPLE INC." => "Apple",
        "TUXEDO" => "TUXEDO", "SCHENKER" => "Schenker", "CLEVO" => "Clevo", "GIGABYTE" => "Gigabyte",
        _ => return None,
    })
}

fn read(name: &str) -> String {
    crate::util::read_opt(&Path::new(DMI).join(name)).map(|s| s.trim().to_string()).unwrap_or_default()
}

#[derive(Clone, Debug)]
pub struct Device {
    pub vendor: String,
    pub model: String,
    pub laptop: bool,
    pub backlight: Vec<String>,
    pub battery: bool,
}

/// Python `str.title()`
fn title(s: &str) -> String {
    let mut out = String::new();
    let mut prev_alpha = false;
    for c in s.chars() {
        if c.is_alphabetic() {
            if prev_alpha { out.extend(c.to_lowercase()) } else { out.extend(c.to_uppercase()) }
            prev_alpha = true;
        } else {
            out.push(c);
            prev_alpha = false;
        }
    }
    out
}

fn is_upper(s: &str) -> bool {
    s.chars().any(|c| c.is_alphabetic()) && !s.chars().any(|c| c.is_lowercase())
}

/// Hersteller, Modell, Laptop? – im Testmodus ein Beispiel-Lenovo.
pub fn detect() -> Device {
    if model::dry() {
        return Device { vendor: "Lenovo".into(), model: "IdeaPad 5 14ALC05".into(), laptop: true,
            backlight: vec!["amdgpu_bl1".into()], battery: true };
    }
    let raw = read("sys_vendor");
    let vendor = vendor_name(&raw.to_uppercase()).map(String::from)
        .unwrap_or_else(|| if is_upper(&raw) { title(&raw) } else { raw.clone() });
    let (name, version) = (read("product_name"), read("product_version"));
    let dev = if raw.to_uppercase() == "LENOVO" && !GENERIC.contains(&version.to_lowercase().as_str()) {
        version
    } else if !GENERIC.contains(&name.to_lowercase().as_str()) {
        name
    } else {
        version
    };
    let chassis: i64 = read("chassis_type").parse().unwrap_or(0);
    let mut backlight: Vec<String> = std::fs::read_dir("/sys/class/backlight").map(|rd| rd.flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
    backlight.sort();
    let battery = std::fs::read_dir("/sys/class/power_supply").map(|rd| rd.flatten()
        .any(|e| e.file_name().to_string_lossy().starts_with("BAT"))).unwrap_or(false);
    Device { vendor, model: dev, laptop: LAPTOP_CHASSIS.contains(&chassis) || battery || !backlight.is_empty(),
        backlight, battery }
}

pub fn label(d: &Device) -> String {
    let parts: Vec<&str> = [d.vendor.as_str(), d.model.as_str()].into_iter()
        .filter(|x| !x.is_empty() && !GENERIC.contains(&x.to_lowercase().as_str())).collect();
    if parts.is_empty() { "?".into() } else { parts.join(" ") }
}

/// wpctl (PipeWire/WirePlumber, Standard auf Arch) oder pactl (PulseAudio).
pub fn volume_tool() -> &'static str {
    if model::dry() || model::which("wpctl") {
        return "wpctl";
    }
    if model::which("pactl") { "pactl" } else { "wpctl" }
}

/// Welche Befehle für die Tasten da sind: (brightnessctl, volume, playerctl)
pub fn tools() -> (bool, bool, bool) {
    if model::dry() {
        return (true, true, true);
    }
    (model::which("brightnessctl"), model::which(volume_tool()), model::which("playerctl"))
}
