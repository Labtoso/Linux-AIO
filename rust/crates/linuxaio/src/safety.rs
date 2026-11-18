//! Startprüfung: gefährliche Reste älterer Versionen entfernen (mit Sicherung) – globale GTK_THEME/QT-Overrides in
//! environment.d, der systemd-Benutzersitzung und Flatpak. Sie machen libadwaita-Apps kaputt (GNOME-Einstellungen
//! ohne Abstände) und können Apps hell/dunkel erzwingen.

use crate::usercmd;
use hyprstudio::util;

pub const ENV_FILE: &str = "environment.d/60-hyprstudio-look.conf";
pub const VARS: [&str; 3] = ["GTK_THEME", "QT_QPA_PLATFORMTHEME", "QT_STYLE_OVERRIDE"];
pub const MARKS: [&str; 3] = ["Hypr Studio", "LabtosoStudios", "Linux AIO"];

/// Gibt zurück, was entfernt wurde (leer = nichts gefunden).
pub fn cleanup() -> Vec<String> {
    let mut done = Vec::new();
    let f = usercmd::config_dir().join(ENV_FILE);
    let src = usercmd::readable(&f);
    if src.is_file() && MARKS.iter().any(|m| util::read_text(&src).contains(m)) {
        let bak = usercmd::config_dir().join("LabtosoStudios/backups")
            .join(util::strftime("%Y-%m-%d_%H%M%S") + "-env");
        let _ = std::fs::create_dir_all(&bak);
        let _ = util::copy2(&src, &bak.join(util::file_name(&f)));
        let _ = std::fs::remove_file(&f);
        usercmd::log(&format!("− {} (backup: {})", f.display(), bak.display()));
        done.push(f.display().to_string());
    }
    let env = usercmd::query(&["systemctl", "--user", "show-environment"]).unwrap_or_default();
    if !done.is_empty() && env.lines().any(|l| VARS.contains(&l.split('=').next().unwrap_or(""))) {
        let mut c = vec!["systemctl", "--user", "unset-environment"];
        c.extend(VARS);
        usercmd::run(&c);
        done.push("systemd user environment".into());
    }
    let over = usercmd::query(&["flatpak", "override", "--user", "--show"]).unwrap_or_default();
    if over.lines().any(|l| l.starts_with("GTK_THEME=") && l.trim() != "GTK_THEME=") {
        usercmd::run(&["flatpak", "override", "--user", "--unset-env=GTK_THEME"]);
        done.push("Flatpak GTK_THEME".into());
    }
    done
}
