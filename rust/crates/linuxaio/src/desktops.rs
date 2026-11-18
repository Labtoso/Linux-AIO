//! Desktops für den Tab „Einstellungen“: welche gibt es, welche sind installiert, welcher läuft.

use crate::{catalog, system};

#[derive(Debug, Clone)]
pub struct Entry {
    pub id: String,
    pub name: String,
    pub sessions: Vec<String>,
    pub detect: Vec<String>,
}

fn e(id: &str, name: &str, sessions: &[&str], detect: &[&str]) -> Entry {
    Entry { id: id.into(), name: name.into(), sessions: sessions.iter().map(|s| s.to_string()).collect(),
            detect: detect.iter().map(|s| s.to_string()).collect() }
}

/// Zusätzlich zum Katalog (dort nur, was sich installieren lässt)
fn extra() -> Vec<Entry> {
    vec![
        e("niri", "niri", &["niri"], &["niri"]),
        e("pantheon", "Pantheon", &["pantheon"], &["Pantheon"]),
        e("deepin", "Deepin", &["deepin"], &["Deepin", "DDE"]),
        e("enlightenment", "Enlightenment", &["enlightenment"], &["Enlightenment"]),
    ]
}

pub const ORDER: [&str; 15] = ["plasma", "gnome", "hyprland", "xfce", "cinnamon", "mate", "budgie", "lxqt", "cosmic",
    "sway", "i3", "niri", "pantheon", "deepin", "enlightenment"];

pub fn entries() -> Vec<Entry> {
    let mut items: Vec<Entry> = catalog::DESKTOPS.iter()
        .map(|d| Entry { id: d.id.clone(), name: d.name.clone(), sessions: d.sessions.clone(), detect: d.detect.clone() })
        .collect();
    for x in extra() {
        match items.iter_mut().find(|i| i.id == x.id) {
            Some(i) => *i = x,
            None => items.push(x),
        }
    }
    let mut out: Vec<Entry> = ORDER.iter().filter_map(|k| items.iter().find(|i| i.id == *k).cloned()).collect();
    out.extend(items.into_iter().filter(|i| !ORDER.contains(&i.id.as_str())));
    out
}

pub fn get(de_id: &str) -> Option<Entry> {
    entries().into_iter().find(|e| e.id == de_id)
}

pub fn installed(e: &Entry) -> bool {
    if std::env::var("DEMANAGER_DRYRUN").as_deref() == Ok("1") && std::env::var("LINUXAIO_ALL_DE").as_deref() == Ok("1") {
        return true;
    }
    e.sessions.iter().any(|s| system::session_file(s).is_some())
}

/// Laufender Desktop aus XDG_CURRENT_DESKTOP (bzw. XDG_SESSION_DESKTOP).
pub fn current_from(xcd: &str, xsd: &str) -> Option<String> {
    let cur = format!("{}:{}", xcd, xsd);
    let all = entries();
    for p in cur.split(':').map(|p| p.trim().to_lowercase()).filter(|p| !p.is_empty()) {
        // erster Eintrag zählt (Budgie:GNOME → Budgie)
        for e in &all {
            if e.detect.iter().chain(e.sessions.iter()).chain(std::iter::once(&e.id)).any(|x| x.to_lowercase() == p) {
                return Some(e.id.clone());
            }
        }
    }
    None
}

pub fn current() -> Option<String> {
    current_from(&std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default(),
                 &std::env::var("XDG_SESSION_DESKTOP").unwrap_or_default())
}

/// Desktops ohne eigene Einstellungen für GTK-Themes: hier gelten gsettings/settings.ini.
pub fn gtk_based(de_id: &str) -> bool {
    ["hyprland", "sway", "i3", "niri", "enlightenment", "cosmic"].contains(&de_id)
}

#[cfg(test)]
mod tests {
    #[test]
    fn current() {
        assert_eq!(super::current_from("Budgie:GNOME", "").as_deref(), Some("budgie"));
        assert_eq!(super::current_from("KDE", "").as_deref(), Some("plasma"));
        assert_eq!(super::entries().len(), 15);
    }
}
