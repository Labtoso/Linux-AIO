//! Herunterfahren, Neustart, Abmelden, Sperren als App-Einträge – damit Rofi, Wofi, Fuzzel und L Launcher sie finden.
//! Je Aktion ein englischer und ein deutscher Eintrag, so wirkt „shutdown“ genauso wie „herunterfahren“, egal welche
//! Sprache eingestellt ist. Enter führt den Befehl sofort aus. Ausschalten = Einträge verstecken (nichts wird gelöscht).

use crate::model::{self, Log, MARK};
use crate::util;
use std::path::PathBuf;

/// (Schlüssel, Befehl, Symbol, [(Sprache, Name)], Suchwörter)
pub const ACTIONS: &[(&str, &str, &str, &[(&str, &str)], &str)] = &[
    ("poweroff", "systemctl poweroff", "system-shutdown", &[("en", "Shutdown"), ("de", "Herunterfahren")],
     "shutdown;power off;poweroff;turn off;herunterfahren;ausschalten;abschalten;"),
    ("reboot", "systemctl reboot", "system-reboot", &[("en", "Restart"), ("de", "Neustart")],
     "restart;reboot;neustart;neu starten;"),
    // ganz abmelden bis zum Anmeldebildschirm (SDDM): die ganze Sitzung beenden, nicht nur sperren
    ("logout", "sh -c \"loginctl terminate-session self || hyprctl dispatch exit\"", "system-log-out",
     &[("en", "Logout"), ("de", "Abmelden"), ("de2", "Ausloggen")],
     "logout;log out;sign out;abmelden;ausloggen;abmeldung;"),
    ("lock", "hyprlock", "system-lock-screen", &[("en", "Lock screen"), ("de", "Sperren")],
     "lock;lock screen;sperren;bildschirm sperren;"),
];

pub fn apps_dir() -> PathBuf {
    if model::dry() {
        return model::cache().join("testmodus").join("applications");
    }
    util::xdg_data().join("applications")
}

pub fn entry(key: &str, lang: &str, name: &str, cmd: &str, icon: &str, words: &str, shown: bool) -> String {
    [format!("# {} – Systembefehl für alle Launcher (Sonstiges › Launcher › Systembefehle)", MARK),
        "[Desktop Entry]".into(), "Type=Application".into(), format!("Name={}", name), format!("Exec={}", cmd),
        format!("Icon={}", icon), format!("Keywords={}", words), "Categories=System;".into(), "Terminal=false".into(),
        format!("NoDisplay={}", if shown { "false" } else { "true" }),
        format!("Hidden={}", if shown { "false" } else { "true" }), format!("X-HyprStudio={}-{}", key, lang),
        String::new()].join("\n")
}

pub fn files(on: bool, lock_ok: bool) -> Vec<(PathBuf, String)> {
    let mut out = vec![];
    for (key, cmd, icon, names, words) in ACTIONS {
        let shown = on && (*key != "lock" || lock_ok);
        for (lang, name) in *names {
            out.push((apps_dir().join(format!("hyprstudio-{}-{}.desktop", key, lang)),
                entry(key, lang, name, cmd, icon, words, shown)));
        }
    }
    out
}

/// Einträge schreiben (an) bzw. verstecken (aus). Gibt die Zahl der geänderten Dateien zurück.
pub fn apply(on: bool, log: Log, lock_ok: bool) -> usize {
    let mut n = 0;
    for (f, text) in files(on, lock_ok) {
        if f.is_file() && util::read_text(&f) == text {
            continue;
        }
        let _ = util::write_text(&f, &text);
        n += 1;
    }
    if n > 0 {
        log(&format!("→ {}: {} Systembefehle {}", apps_dir().display(), n, if on { "an" } else { "aus" }));
    }
    n
}
