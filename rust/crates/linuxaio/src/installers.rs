//! Ein-Klick-Installation für Fehlendes (Download-Pfeil in der App).
//!
//! GNOME-Erweiterungen: passende Version von extensions.gnome.org (Info-Endpunkt mit der Shell-Version),
//! Zip prüfen (metadata.json mit richtiger UUID, keine Pfade außerhalb), dann `gnome-extensions install --force`
//! und `gnome-extensions enable`. Klappt das nicht: Paket über den Root-Helfer (`pkg-install <schlüssel>`).

use crate::{gnome, remote, sddm, usercmd};
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;
use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Component, Path};
use std::sync::Mutex;

pub const EGO: &str = "https://extensions.gnome.org";

/// Schlüssel → Pakete je Paketmanager (der Root-Helfer nimmt nur diese Schlüssel an)
pub fn packages_table(key: &str, backend: &str) -> Option<&'static [&'static str]> {
    let v: &[(&str, &[&str])] = match key {
        "yay-deps" => &[("pacman", &["git", "base-devel"])],
        "alacritty" => &[("pacman", &["alacritty"]), ("apt", &["alacritty"]), ("dnf", &["alacritty"]),
                         ("zypper", &["alacritty"])],
        "kitty" => &[("pacman", &["kitty"]), ("apt", &["kitty"]), ("dnf", &["kitty"]), ("zypper", &["kitty"])],
        "gnome-shell-extensions" => &[("pacman", &["gnome-shell-extensions"]), ("apt", &["gnome-shell-extensions"]),
            ("dnf", &["gnome-shell-extension-user-theme"]), ("zypper", &["gnome-shell-extensions-common"])],
        "gnome-tweaks" => &[("pacman", &["gnome-tweaks"]), ("apt", &["gnome-tweaks"]), ("dnf", &["gnome-tweaks"]),
                            ("zypper", &["gnome-tweaks"])],
        "squashfs-tools" => &[("pacman", &["squashfs-tools"]), ("apt", &["squashfs-tools"]),
                              ("dnf", &["squashfs-tools"]), ("zypper", &["squashfs"])],
        _ => return None,
    };
    v.iter().find(|(b, _)| *b == backend).map(|(_, p)| *p)
}

pub const PACKAGE_KEYS: [&str; 6] = ["yay-deps", "alacritty", "kitty", "gnome-shell-extensions", "gnome-tweaks",
    "squashfs-tools"];

/// Erweiterung → Paket, das sie mitbringt (Plan B)
pub fn ext_package(uuid: &str) -> Option<&'static str> {
    match uuid {
        "user-theme@gnome-shell-extensions.gcampax.github.com"
        | "workspace-indicator@gnome-shell-extensions.gcampax.github.com" => Some("gnome-shell-extensions"),
        _ => None,
    }
}

pub fn ext_name(uuid: &str) -> Option<&'static str> {
    match uuid {
        "just-perfection-desktop@just-perfection" => Some("Just Perfection"),
        "rounded-window-corners@fxgn" => Some("Rounded Window Corners Reborn"),
        "user-theme@gnome-shell-extensions.gcampax.github.com" => Some("User Themes"),
        "workspace-indicator@gnome-shell-extensions.gcampax.github.com" => Some("Workspace Indicator"),
        _ => None,
    }
}

/// Testmodus: „installierte“ Erweiterungen merken
pub static DRY_EXT: Lazy<Mutex<BTreeSet<String>>> = Lazy::new(|| Mutex::new(BTreeSet::new()));

pub fn dry_ext_has(uuid: &str) -> bool {
    DRY_EXT.lock().unwrap().contains(uuid)
}

#[derive(Debug, Clone)]
pub struct InstallError {
    pub key: String,
    pub detail: String,
}

impl InstallError {
    pub fn new(key: &str, detail: &str) -> InstallError {
        InstallError { key: key.into(), detail: detail.into() }
    }
}

pub fn shell_version() -> String {
    let out = usercmd::query(&["gnome-shell", "--version"]).unwrap_or_default();
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\d+)(?:\.(\d+))?").unwrap());
    RE.captures(&out).map(|m| m[1].to_string()).unwrap_or_default()
}

pub fn check_zip(data: &[u8], uuid: &str) -> Result<(), InstallError> {
    let bad = |d: &str| InstallError::new("miss.badzip", d);
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(data)).map_err(|e| bad(&e.to_string()))?;
    let names: Vec<String> = z.file_names().map(String::from).collect();
    // wie testzip(): jede Datei lesen (CRC)
    for i in 0..z.len() {
        let mut f = z.by_index(i).map_err(|e| bad(&e.to_string()))?;
        let n = f.name().to_string();
        let mut sink = Vec::new();
        if f.read_to_end(&mut sink).is_err() {
            return Err(bad(&n));
        }
    }
    for n in &names {
        let p = Path::new(n);
        if n.starts_with('/') || n.starts_with('\\') || p.is_absolute() || p.components().any(|c| c == Component::ParentDir) {
            return Err(bad(n));
        }
    }
    let mut meta = String::new();
    z.by_name("metadata.json").map_err(|_| bad("metadata.json"))?.read_to_string(&mut meta)
        .map_err(|_| bad("metadata.json"))?;
    let m: Value = serde_json::from_str(&meta).map_err(|_| bad("metadata.json"))?;
    let got = m.get("uuid").cloned().unwrap_or(Value::Null);
    if got.as_str() != Some(uuid) {
        let shown = match &got { Value::Null => "None".to_string(), Value::String(s) => s.clone(), v => v.to_string() };
        return Err(bad(&format!("uuid {}", shown)));
    }
    Ok(())
}

pub fn ego_download(uuid: &str, version: &str) -> Result<Vec<u8>, InstallError> {
    let q = sddm::urlencode(&[("uuid", uuid.to_string()), ("shell_version", version.to_string())]);
    let info: Value = remote::fetch(&format!("{}/extension-info/?{}", EGO, q))
        .map_err(|e| InstallError::new("miss.noversion", &e.to_string()))
        .and_then(|b| serde_json::from_slice(&b).map_err(|e| InstallError::new("miss.noversion", &e.to_string())))?;
    let url = info.get("download_url").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if url.is_empty() || !url.starts_with('/') {
        return Err(InstallError::new("miss.noversion", version));
    }
    remote::fetch(&format!("{}{}", EGO, url)).map_err(|e| InstallError::new("remote.offline", &e.to_string()))
}

/// Erweiterung holen, prüfen, installieren, einschalten. Gibt "relogin", "incompatible" oder "ok" zurück.
pub fn install_extension(uuid: &str) -> Result<String, InstallError> {
    if usercmd::dry() {
        let v = shell_version();
        usercmd::log(&format!("[Testmodus] {}/extension-info/?uuid={}&shell_version={}", EGO, uuid,
                              if v.is_empty() { "?" } else { &v }));
        usercmd::run(&["gnome-extensions", "install", "--force", &format!("{}.shell-extension.zip", uuid)]);
        usercmd::run(&["gnome-extensions", "enable", uuid]);
        DRY_EXT.lock().unwrap().insert(uuid.into());
        let mut en = gnome::ext_enabled();
        en.push(uuid.into());
        usercmd::gset("org.gnome.shell", "enabled-extensions", serde_json::json!(en));
        return Ok("relogin".into());
    }
    let ver = shell_version();
    if ver.is_empty() {
        return Err(InstallError::new("miss.noshell", ""));
    }
    let data = ego_download(uuid, &ver)?;
    check_zip(&data, uuid)?;
    {
        let tmp = crate::helper::sddm::TempDir::new("linuxaio-ext-").map_err(|e| InstallError::new("miss.failed", &e.to_string()))?;
        let z = tmp.0.join(format!("{}.shell-extension.zip", uuid));
        std::fs::write(&z, &data).map_err(|e| InstallError::new("miss.failed", &e.to_string()))?;
        let (ok, out) = usercmd::run(&["gnome-extensions", "install", "--force", &z.display().to_string()]);
        if !ok {
            return Err(InstallError::new("miss.failed", &out));
        }
    }
    gnome::ext_set(uuid, true);
    if gnome::ext_compatible(uuid) == Some(false) {
        return Ok("incompatible".into());
    }
    // Unter Wayland kennt die laufende Shell neu installierte Erweiterungen erst nach dem Neuanmelden
    let en = usercmd::query(&["gnome-extensions", "list", "--enabled"]).unwrap_or_default();
    Ok(if en.contains(uuid) { "ok".into() } else { "relogin".into() })
}

pub fn packages_for(key: &str, backend: &str) -> Vec<String> {
    packages_table(key, backend).map(|p| p.iter().map(|s| s.to_string()).collect()).unwrap_or_default()
}
