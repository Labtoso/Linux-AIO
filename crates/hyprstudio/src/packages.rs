//! Optionale Programme selbst auswählen und installieren (nichts davon wird automatisch installiert).

use crate::{appearance, model, util};
use once_cell::sync::Lazy;
use std::collections::HashSet;
use std::sync::Mutex;
use std::time::Instant;

pub struct Package {
    pub key: &'static str,
    pub group: &'static str,
    pub check: String,
    pub pkgs: Vec<(&'static str, Vec<&'static str>)>,
}

const ALL4: &[&str] = &["pacman", "apt", "dnf", "zypper"];

fn same(p: &[&'static str]) -> Vec<(&'static str, Vec<&'static str>)> {
    // gleiche Pakete für alle vier Paketmanager
    ALL4.iter().map(|pm| (*pm, p.to_vec())).collect()
}

fn pk(v: &[(&'static str, &[&'static str])]) -> Vec<(&'static str, Vec<&'static str>)> {
    v.iter().map(|(a, b)| (*a, b.to_vec())).collect()
}

/// key: (Gruppe, Prüf-Befehl oder Font-Name, {Paketmanager: [Pakete]})
pub static PACKAGES: Lazy<Vec<Package>> = Lazy::new(|| {
    let p = |key, group, check: &str, pkgs: Vec<(&'static str, Vec<&'static str>)>| Package { key, group, check: check.to_string(), pkgs };
    vec![
        p("rofi", "launcher", "rofi", pk(&[("pacman", &["rofi"]), ("apt", &["rofi"]), ("dnf", &["rofi-wayland"]),
            ("zypper", &["rofi"])])),
        p("wofi", "launcher", "wofi", same(&["wofi"])),
        p("fuzzel", "launcher", "fuzzel", same(&["fuzzel"])),
        p("walker", "launcher", "walker", pk(&[("pacman", &["walker"])])),
        p("tofi", "launcher", "tofi", pk(&[("apt", &["tofi"])])),
        p("waybar", "bar", "waybar", same(&["waybar"])),
        p("nerdfont", "bar", "font:JetBrainsMono Nerd Font",
            pk(&[("pacman", &["ttf-jetbrains-mono-nerd", "ttf-nerd-fonts-symbols-mono"])])),
        p("kitty", "tools", "kitty", same(&["kitty"])),
        p("pavucontrol", "bar", "pavucontrol", same(&["pavucontrol"])),
        p("screenshot", "tools", "grim", same(&["grim", "slurp", "wl-clipboard"])),
        p("media", "tools", "playerctl", same(&["playerctl", "brightnessctl"])),
        p("hyprpicker", "tools", "hyprpicker", pk(&[("pacman", &["hyprpicker"]), ("dnf", &["hyprpicker"])])),
        p("hyprlock", "tools", "hyprlock", pk(&[("pacman", &["hyprlock"]), ("dnf", &["hyprlock"])])),
        p("hyprcursor", "tools", "hyprcursor-util", pk(&[("pacman", &["hyprcursor"]), ("dnf", &["hyprcursor"])])),
        p("hyprpaper", "tools", "hyprpaper", pk(&[("pacman", &["hyprpaper"]), ("dnf", &["hyprpaper"])])),
        p("cliphist", "tools", "cliphist", pk(&[("pacman", &["cliphist"])])),
        p("hyprpolkitagent", "tools", "file:/usr/lib/hyprpolkitagent/hyprpolkitagent|/usr/libexec/hyprpolkitagent\
|/usr/lib/systemd/user/hyprpolkitagent.service", pk(&[("pacman", &["hyprpolkitagent"]), ("dnf", &["hyprpolkitagent"])])),
        p("qtdark", "tools", &format!("file:{}", appearance::KDE_PLUGIN_GLOBS.join("|")),
            pk(&[("pacman", &["plasma-integration"]), ("apt", &["plasma-integration"]), ("dnf", &["plasma-integration"]),
              ("zypper", &["plasma6-integration-plugin"])])),
        p("fastfetch", "tools", "fastfetch", same(&["fastfetch"])),
    ]
});

pub const GROUPS: &[&str] = &["launcher", "bar", "tools"];

pub static DRY_INSTALLED: Lazy<Mutex<HashSet<String>>> = Lazy::new(Default::default);
pub static DRY_REMOVED: Lazy<Mutex<HashSet<String>>> = Lazy::new(Default::default);

pub fn get(key: &str) -> Option<&'static Package> {
    PACKAGES.iter().find(|p| p.key == key)
}

pub fn manager() -> Option<&'static str> {
    if model::dry() {
        return Some("pacman");
    }
    for (pm, exe) in [("pacman", "pacman"), ("apt", "apt-get"), ("dnf", "dnf"), ("zypper", "zypper")] {
        if util::has(exe) {
            return Some(pm);
        }
    }
    None
}

/// fc-list ist bei vielen Schriften langsam → kurz zwischenspeichern
static FONTS: Lazy<Mutex<(Option<Instant>, String)>> = Lazy::new(|| Mutex::new((None, String::new())));

pub fn font_list() -> String {
    let mut f = FONTS.lock().unwrap();
    if f.0.map(|t| t.elapsed().as_secs_f64() > 60.0).unwrap_or(true) {
        f.1 = util::capture(&["fc-list", ":", "family"], Some(8)).map(|o| o.stdout.to_lowercase()).unwrap_or_default();
        f.0 = Some(Instant::now());
    }
    f.1.clone()
}

/// Nach Installieren/Entfernen: Zwischenspeicher verwerfen.
pub fn forget() {
    FONTS.lock().unwrap().0 = None;
}

pub fn any_glob(patterns: &str) -> bool {
    patterns.split('|').any(|g| glob::glob(g).map(|mut it| it.next().is_some()).unwrap_or(false))
}

pub fn installed(key: &str) -> bool {
    let Some(p) = get(key) else { return true }; // z. B. L Launcher: gehört zu Hypr Studio
    if model::dry() {
        // Testmodus: etwas gemischt anzeigen, „Installiertes“ merken
        return (key == "waybar" || key == "rofi" || DRY_INSTALLED.lock().unwrap().contains(key))
            && !DRY_REMOVED.lock().unwrap().contains(key);
    }
    if let Some(g) = p.check.strip_prefix("file:") {
        return any_glob(g);
    }
    if let Some(f) = p.check.strip_prefix("font:") {
        return font_list().contains(&f.to_lowercase());
    }
    util::has(&p.check)
}

pub const POLKIT_AGENTS: &[&str] = &["hyprpolkitagent", "polkit-kde-authentication-agent-1",
    "polkit-gnome-authentication-agent-1", "polkit-mate-authentication-agent-1", "lxpolkit", "lxqt-policykit-agent",
    "xfce-polkit", "polkit-agent-helper", "soteria"];

/// Läuft ein Polkit-Agent (Passwortfenster für pkexec)? Unter Hyprland fehlt er oft → dann Terminal + sudo.
pub fn polkit_agent() -> bool {
    if model::dry() {
        return true;
    }
    match util::capture(&["ps", "-eo", "args"], Some(5)) {
        Ok(o) => POLKIT_AGENTS.iter().any(|a| o.stdout.contains(a)),
        Err(_) => false,
    }
}

/// Häufige Paketmanager-Fehler → i18n-Schlüssel mit verständlichem Hinweis.
pub fn fail_reason(output: &str) -> Option<&'static str> {
    let o = output.to_lowercase();
    let any = |v: &[&str]| v.iter().any(|x| o.contains(x));
    if any(&["failed retrieving file", "404", "could not resolve", "failed to synchronize"]) {
        return Some("pkg.err_sync");
    }
    if any(&["target not found", "unable to locate package", "no match for argument"]) {
        return Some("pkg.err_notfound");
    }
    if any(&["unable to lock database", "could not get lock"]) {
        return Some("pkg.err_lock");
    }
    if any(&["not authorized", "authentication", "no authentication agent"]) {
        return Some("pkg.err_auth");
    }
    None
}

fn pkgs_for(key: &str, pm: &str) -> Vec<&'static str> {
    get(key).and_then(|p| p.pkgs.iter().find(|(m, _)| *m == pm)).map(|(_, v)| v.to_vec()).unwrap_or_default()
}

pub fn available(key: &str, pm: Option<&str>) -> bool {
    pm.map(|pm| !pkgs_for(key, pm).is_empty()).unwrap_or(false)
}

pub fn pkg_names(keys: &[&str], pm: &str) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for k in keys {
        for p in pkgs_for(k, pm) {
            if !out.iter().any(|x| x == p) {
                out.push(p.to_string());
            }
        }
    }
    out
}

/// Nur Pakete, die wirklich installiert sind (sonst bricht das Entfernen ab).
fn present(pkgs: Vec<String>, pm: &str) -> Vec<String> {
    if model::dry() {
        return pkgs;
    }
    let q: &[&str] = match pm { "pacman" => &["pacman", "-Q"], "apt" => &["dpkg", "-s"], _ => &["rpm", "-q"] };
    pkgs.into_iter().filter(|p| {
        let mut cmd = q.to_vec();
        cmd.push(p);
        util::capture(&cmd, Some(8)).map(|o| o.ok()).unwrap_or(true)
    }).collect()
}

fn with(base: &[&str], pkgs: &[String]) -> Vec<String> {
    base.iter().map(|s| s.to_string()).chain(pkgs.iter().cloned()).collect()
}

/// Entfernen per pkexec; pacman -Rs nimmt nicht mehr gebrauchte Abhängigkeiten mit.
pub fn remove_command(keys: &[&str], pm: &str) -> Vec<String> {
    let pkgs = present(pkg_names(keys, pm), pm);
    if pkgs.is_empty() {
        return vec![];
    }
    match pm {
        "pacman" => with(&["pacman", "-Rs", "--noconfirm"], &pkgs),
        "apt" => with(&["apt-get", "remove", "-y"], &pkgs),
        "dnf" => with(&["dnf", "remove", "-y"], &pkgs),
        _ => with(&["zypper", "-n", "rm"], &pkgs),
    }
}

// ---------- Updates (Sonstiges › Updates) ----------

/// Gruppe → Pakete (nur installierte werden aktualisiert)
pub const UPDATE_GROUPS: &[(&str, &[&str])] = &[
    ("hyprland", &["hyprland", "hyprlock", "hypridle", "hyprpaper", "hyprcursor", "hyprpicker", "hyprpolkitagent",
        "hyprsunset", "xdg-desktop-portal-hyprland", "hyprutils", "hyprlang", "hyprgraphics", "aquamarine"]),
    ("waybar", &["waybar"]),
    ("launcher", &["rofi", "rofi-wayland", "wofi", "fuzzel", "walker"]),
];

/// Installierte Versionen {Paket: Version}; nicht installierte fehlen.
pub fn versions(pkgs: &[String], pm: Option<&str>) -> Vec<(String, String)> {
    if model::dry() {
        let demo = [("hyprland", "0.55.1-1"), ("hyprlock", "0.9.1-1"), ("hyprpaper", "0.7.5-1"), ("waybar", "0.13.0-1"),
            ("rofi", "1.7.9-1")];
        return pkgs.iter().filter_map(|p| demo.iter().find(|(d, _)| d == p).map(|(_, v)| (p.clone(), v.to_string())))
            .collect();
    }
    let cmd: Vec<String> = match pm {
        Some("pacman") => with(&["pacman", "-Q"], pkgs),
        Some("apt") => with(&["dpkg-query", "-W", "-f=${Package} ${Version}\n"], pkgs),
        Some("dnf") | Some("zypper") => with(&["rpm", "-q", "--qf", "%{NAME} %{VERSION}-%{RELEASE}\n"], pkgs),
        _ => return vec![],
    };
    let refs: Vec<&str> = cmd.iter().map(|s| s.as_str()).collect();
    let Ok(o) = util::capture(&refs, Some(15)) else { return vec![] };
    let mut res = vec![];
    for ln in o.stdout.lines() {
        let parts: Vec<&str> = ln.split_whitespace().collect();
        if parts.len() == 2 && pkgs.iter().any(|p| p == parts[0]) {
            res.push((parts[0].to_string(), parts[1].to_string()));
        }
    }
    res
}

/// Verfügbare neuere Versionen {Paket: neu} – Arch über checkupdates (pacman-contrib, ändert nichts am
/// System); None = lässt sich nicht prüfen.
pub fn newer(pm: Option<&str>) -> Option<Vec<(String, String)>> {
    if model::dry() {
        return Some(vec![("hyprland".into(), "0.56.0-1".into()), ("waybar".into(), "0.13.1-1".into())]);
    }
    if pm == Some("pacman") && util::has("checkupdates") {
        let o = util::capture(&["checkupdates"], Some(90)).ok()?;
        let mut res = vec![];
        for ln in o.stdout.lines() {
            // „paket alt -> neu“
            let parts: Vec<&str> = ln.split_whitespace().collect();
            if parts.len() >= 4 && parts[2] == "->" {
                res.push((parts[0].to_string(), parts[3].to_string()));
            }
        }
        return Some(res);
    }
    None
}

/// Befehl für pkexec. Arch: immer mit -Syu (Teil-Updates sind dort nicht unterstützt).
pub fn update_command(pkgs: &[String], pm: Option<&str>, everything: bool) -> Vec<String> {
    let Some(pm) = pm else { return vec![] };
    if !everything && pkgs.is_empty() {
        return vec![];
    }
    let none: Vec<String> = vec![];
    let sel = if everything { &none } else { pkgs };
    match pm {
        "pacman" => {
            let mut v = with(&["pacman", "-Syu", "--noconfirm"], &[]);
            if !everything {
                v.push("--needed".into());
                v.extend(pkgs.iter().cloned());
            }
            v
        }
        "apt" => {
            let up = if everything { "apt-get -y upgrade".to_string() } else {
                format!("apt-get install -y --only-upgrade {}", pkgs.join(" "))
            };
            vec!["sh".into(), "-c".into(), format!("apt-get update && {}", up)]
        }
        "dnf" => with(&["dnf", "upgrade", "-y"], sel),
        "zypper" => with(&["zypper", "-n", "up"], sel),
        _ => vec![],
    }
}

/// Befehl für pkexec (läuft als root, ohne Rückfrage im Terminal).
pub fn command(keys: &[&str], pm: &str) -> Vec<String> {
    let pkgs = pkg_names(keys, pm);
    if pkgs.is_empty() {
        return vec![];
    }
    match pm {
        "pacman" => with(&["pacman", "-S", "--needed", "--noconfirm"], &pkgs),
        "apt" => with(&["apt-get", "install", "-y"], &pkgs),
        "dnf" => with(&["dnf", "install", "-y"], &pkgs),
        _ => with(&["zypper", "-n", "in"], &pkgs),
    }
}
