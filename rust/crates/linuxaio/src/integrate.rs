//! Desktop-Einbindung: Läuft das Programm ohne installiertes Paket (einzelne Datei), wird ein Startmenü-Eintrag mit
//! derselben ID wie das Fenster (Wayland app_id = Name der .desktop-Datei) und das Tux-Symbol in hicolor angelegt,
//! damit Docks das richtige Symbol zeigen. Das PNG zeichnet der Aufrufer.

use crate::usercmd;
use hyprstudio::util;
use std::path::{Path, PathBuf};

pub fn system_entry() -> PathBuf {
    PathBuf::from(format!("/usr/share/applications/{}.desktop", crate::APP_ID))
}

/// Befehl, der diese Kopie des Programms startet (nur außerhalb von /usr).
pub fn launcher() -> Option<String> {
    if let Ok(a) = std::env::var("APPIMAGE") {
        if Path::new(&a).is_file() {
            return Some(format!("\"{}\"", a));
        }
    }
    let exe = std::env::current_exe().ok()?;
    if exe.starts_with("/usr") || exe.to_string_lossy().contains("/target/") {
        return None;
    }
    Some(format!("\"{}\"", exe.display()))
}

pub fn entry_text(exec_cmd: &str) -> String {
    format!("[Desktop Entry]\nType=Application\nName={}\n\
             Comment=Desktops, SDDM, Plymouth and desktop settings in one place\n\
             Comment[de]=Desktops, SDDM, Plymouth und Desktop-Einstellungen an einem Ort\n\
             Exec={}\nIcon={}\nTerminal=false\nCategories=System;Settings;\n\
             StartupWMClass={}\nStartupNotify=false\n\
             Keywords=desktop;gnome;plasma;hyprland;sddm;plymouth;cursor;fonts;tweaks;\n",
            crate::APP_NAME, exec_cmd, crate::APP_ID, crate::APP_ID)
}

/// Eintrag schreiben/auffrischen. `render_png(size, path)` zeichnet das Symbol. True, wenn sich etwas geändert hat.
pub fn ensure(render_png: Option<&dyn Fn(i32, &Path)>, log: &dyn Fn(&str)) -> bool {
    if usercmd::dry() || system_entry().is_file() {
        return false;
    }
    let Some(cmd) = launcher() else { return false };
    let data = util::xdg_data();
    let entry = data.join("applications").join(format!("{}.desktop", crate::APP_ID));
    let text = entry_text(&cmd);
    let mut changed = false;
    let old = if entry.is_file() { util::read_text(&entry) } else { String::new() };
    if old != text {
        let _ = std::fs::create_dir_all(entry.parent().unwrap());
        if let Err(e) = std::fs::write(&entry, &text) {
            log(&format!("Startmenü-Eintrag: {}", e));
            return false;
        }
        changed = true;
        log(&format!("→ {}", entry.display()));
    }
    for size in [48, 128, 256] {
        let icon = data.join(format!("icons/hicolor/{s}x{s}/apps/{}.png", crate::APP_ID, s = size));
        if let (false, Some(r)) = (icon.is_file(), render_png) {
            let _ = std::fs::create_dir_all(icon.parent().unwrap());
            r(size, &icon);
            changed = true;
        }
    }
    if changed {
        usercmd::query(&["update-desktop-database", &entry.parent().unwrap().display().to_string()]);
        usercmd::query(&["gtk-update-icon-cache", "-q", "-t", &data.join("icons/hicolor").display().to_string()]);
    }
    changed
}
