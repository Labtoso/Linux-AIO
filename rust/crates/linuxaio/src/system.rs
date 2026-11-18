//! Systemabfragen ohne Root: Paketmanager, installierte Sessions, laufender Desktop, Standard-Session.

use crate::catalog::{self, Desktop};
use hyprstudio::util;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const SESSION_DIRS: [&str; 2] = ["/usr/share/wayland-sessions", "/usr/share/xsessions"];
pub const SDDM_STATE: &str = "/var/lib/sddm/state.conf";
pub const ACCOUNTS_DIR: &str = "/var/lib/AccountsService/users";
pub const STATE_FILE: &str = "/var/lib/labtoso-linuxaio/state.json";
/// Vorgänger DE SDDM Manager
pub const LEGACY: (&str, &str) = ("labtoso-linuxaio", "labtoso-desddmmanager");

fn dry() -> bool {
    std::env::var("DEMANAGER_DRYRUN").as_deref() == Ok("1")
}

/// Zustand aus /var/lib (übersteht Updates); Einträge der Vorgänger-App werden eingemischt.
pub fn load_state_file(path: &Path) -> Map<String, Value> {
    let mut data = Map::new();
    let old = PathBuf::from(path.to_string_lossy().replace(LEGACY.0, LEGACY.1));
    for p in [old, path.to_path_buf()] {
        let Some(Value::Object(d)) = std::fs::read_to_string(&p).ok().and_then(|s| serde_json::from_str(&s).ok())
        else { continue };
        for (k, v) in d {
            match (v, data.get_mut(&k)) {
                (Value::Object(nv), Some(Value::Object(ov))) => {
                    for (a, b) in nv {
                        ov.insert(a, b);
                    }
                }
                (v, _) => {
                    data.insert(k, v);
                }
            }
        }
    }
    data
}

pub fn detect_backend() -> Option<String> {
    if let Ok(f) = std::env::var("DEMANAGER_BACKEND") {
        if catalog::BACKENDS.contains(&f.as_str()) {
            return Some(f);
        }
    }
    for (name, bin) in [("pacman", "pacman"), ("apt", "apt-get"), ("dnf", "dnf"), ("zypper", "zypper")] {
        if util::has(bin) {
            return Some(name.into());
        }
    }
    None
}

pub fn distro_name() -> String {
    if let Ok(s) = std::fs::read_to_string("/etc/os-release") {
        for line in s.lines() {
            if let Some(v) = line.strip_prefix("PRETTY_NAME=") {
                return v.trim().trim_matches('"').to_string();
            }
        }
    }
    "Linux".into()
}

pub fn session_file(name: &str) -> Option<PathBuf> {
    SESSION_DIRS.iter().map(|d| Path::new(d).join(format!("{}.desktop", name))).find(|p| p.is_file())
}

pub fn installed_session(de: &Desktop) -> Option<PathBuf> {
    de.sessions.iter().find_map(|s| session_file(s))
}

pub fn is_installed(de: &Desktop) -> bool {
    installed_session(de).is_some()
}

pub fn running_desktop() -> Option<String> {
    let cur = format!("{}:{}", std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default(),
                      std::env::var("XDG_SESSION_DESKTOP").unwrap_or_default());
    let parts: BTreeSet<String> = cur.split(':').map(|p| p.trim().to_lowercase()).filter(|p| !p.is_empty()).collect();
    catalog::DESKTOPS.iter()
        .find(|de| de.detect.iter().chain(de.sessions.iter()).any(|x| parts.contains(&x.to_lowercase())))
        .map(|de| de.id.clone())
}

/// configparser-artig: Wert aus [section] key (Schlüssel ohne Groß/klein, Fortsetzungszeilen ignoriert).
fn cp_get(path: &Path, section: &str, key: &str) -> Option<String> {
    let s = std::fs::read_to_string(path).ok()?;
    let mut sec = String::new();
    for ln in s.lines() {
        let t = ln.trim();
        if t.starts_with('[') && t.ends_with(']') {
            sec = t[1..t.len() - 1].to_string();
        } else if sec == section && !t.starts_with('#') && !t.starts_with(';') {
            let pos = t.find(['=', ':']);
            if let Some(i) = pos {
                if t[..i].trim().eq_ignore_ascii_case(key) {
                    return Some(t[i + 1..].trim().to_string());
                }
            }
        }
    }
    None
}

/// Session-Name (ohne .desktop), die der Login-Manager als nächstes startet.
pub fn default_session() -> Option<String> {
    if dry() {
        let s = std::fs::read_to_string(dry_dm_file().parent().unwrap().join("default-session")).ok()?;
        let s = s.trim().to_string();
        return (!s.is_empty()).then_some(s);
    }
    let user = std::env::var("USER").unwrap_or_default();
    let acc = Path::new(ACCOUNTS_DIR).join(user);
    let tries: [(PathBuf, &str, &[&str]); 2] =
        [(acc, "User", &["Session", "XSession"]), (PathBuf::from(SDDM_STATE), "Last", &["Session"])];
    for (path, section, keys) in tries {
        for k in keys {
            if let Some(v) = cp_get(&path, section, k) {
                if !v.is_empty() {
                    let n = util::file_name(Path::new(&v));
                    return Some(n.strip_suffix(".desktop").map(String::from).unwrap_or(n));
                }
            }
        }
    }
    None
}

pub fn default_desktop() -> Option<String> {
    let s = default_session()?;
    catalog::DESKTOPS.iter().find(|de| de.sessions.contains(&s)).map(|de| de.id.clone())
}

pub fn has_pkexec() -> bool {
    util::has("pkexec")
}

// ---------- Paketmanager-Befehle (vom Helfer genutzt) ----------

fn quiet(cmd: &[&str]) -> (i32, String) {
    match Command::new(cmd[0]).args(&cmd[1..]).stdin(Stdio::null()).stdout(Stdio::piped())
        .stderr(Stdio::null()).output() {
        Ok(o) => (o.status.code().unwrap_or(-1), String::from_utf8_lossy(&o.stdout).into_owned()),
        Err(_) => (-1, String::new()),
    }
}

pub fn available(backend: &str, name: &str) -> bool {
    match backend {
        "pacman" => quiet(&["pacman", "-Si", name]).0 == 0 || !quiet(&["pacman", "-Sg", name]).1.trim().is_empty(),
        "apt" => {
            let (c, o) = quiet(&["apt-cache", "show", "--no-all-versions", name]);
            c == 0 && o.contains("Package:")
        }
        "dnf" => {
            if let Some(g) = name.strip_prefix('@') {
                return quiet(&["dnf", "-q", "group", "info", g]).0 == 0;
            }
            !quiet(&["dnf", "-q", "repoquery", "--latest-limit=1", name]).1.trim().is_empty()
        }
        "zypper" => {
            let (kind, n) = match name.strip_prefix("pattern:") {
                Some(p) => ("pattern", p),
                None => ("package", name),
            };
            quiet(&["zypper", "-q", "-n", "se", "-x", "-t", kind, n]).0 == 0
        }
        _ => false,
    }
}

/// Löst Alternativen/optionale Pakete auf. Gibt (Pakete, fehlende Pflichtpakete) zurück.
pub fn resolve(backend: &str, specs: &[String], check: &dyn Fn(&str, &str) -> bool) -> (Vec<String>, Vec<String>) {
    let (mut out, mut missing) = (Vec::new(), Vec::new());
    for spec in specs {
        let optional = spec.starts_with('?');
        let choices = spec.trim_start_matches('?').split('|');
        let hit = choices.into_iter().find(|c| check(backend, c));
        match hit {
            Some(h) => {
                if !out.iter().any(|x: &String| x == h) {
                    out.push(h.to_string());
                }
            }
            None if !optional => missing.push(spec.clone()),
            None => {}
        }
    }
    (out, missing)
}

pub fn installed_packages(backend: &str) -> BTreeSet<String> {
    let cmd: Vec<&str> = match backend {
        "pacman" => vec!["pacman", "-Qq"],
        "apt" => vec!["dpkg-query", "-W", "-f=${db:Status-Abbrev} ${Package}\n"],
        _ => vec!["rpm", "-qa", "--qf", "%{NAME}\n"],
    };
    let (_, out) = quiet(&cmd);
    let mut pk = BTreeSet::new();
    for line in out.lines() {
        if backend == "apt" {
            let (st, name) = line.split_once(' ').unwrap_or((line, ""));
            if st.starts_with("ii") {
                pk.insert(name.split(':').next().unwrap_or("").to_string());
            }
        } else if !line.trim().is_empty() {
            pk.insert(line.trim().to_string());
        }
    }
    pk
}

fn cat(pre: &[&str], pkgs: &[String]) -> Vec<String> {
    pre.iter().map(|s| s.to_string()).chain(pkgs.iter().cloned()).collect()
}

pub fn install_commands(backend: &str, pkgs: &[String]) -> Vec<Vec<String>> {
    match backend {
        "pacman" => vec![cat(&["pacman", "-Syu", "--needed", "--noconfirm"], pkgs)],
        "apt" => vec![cat(&["apt-get", "update"], &[]), cat(&["apt-get", "install", "-y"], pkgs)],
        "dnf" => vec![cat(&["dnf", "install", "-y"], pkgs)],
        "zypper" => {
            let pats: Vec<String> = pkgs.iter().filter_map(|p| p.strip_prefix("pattern:").map(String::from)).collect();
            let plain: Vec<String> = pkgs.iter().filter(|p| !p.starts_with("pattern:")).cloned().collect();
            let mut cmds = vec![cat(&["zypper", "-n", "refresh"], &[])];
            if !pats.is_empty() {
                cmds.push(cat(&["zypper", "-n", "in", "-t", "pattern"], &pats));
            }
            if !plain.is_empty() {
                cmds.push(cat(&["zypper", "-n", "in"], &plain));
            }
            cmds
        }
        _ => vec![],
    }
}

pub fn remove_command(backend: &str, pkgs: &[String]) -> Vec<String> {
    match backend {
        "pacman" => cat(&["pacman", "-Rnsu", "--noconfirm"], pkgs),
        "apt" => cat(&["apt-get", "remove", "-y", "--autoremove"], pkgs),
        "dnf" => cat(&["dnf", "remove", "-y"], pkgs),
        "zypper" => cat(&["zypper", "-n", "rm", "-u"], pkgs),
        _ => vec![],
    }
}

/// Pakete, die ein Entfernen tatsächlich mitnehmen würde (None = nicht prüfbar).
pub fn simulate_remove(backend: &str, pkgs: &[String]) -> Option<BTreeSet<String>> {
    let to_ref = |v: &Vec<String>| v.iter().map(|s| s.as_str()).collect::<Vec<&str>>().join("\u{0}");
    match backend {
        "pacman" => {
            let c = cat(&["pacman", "-Rnsu", "--print", "--print-format", "%n"], pkgs);
            let j = to_ref(&c);
            let (code, out) = quiet(&j.split('\u{0}').collect::<Vec<_>>());
            (code == 0).then(|| out.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
        }
        "apt" => {
            let c = cat(&["apt-get", "-s", "remove", "--autoremove"], pkgs);
            let j = to_ref(&c);
            let (code, out) = quiet(&j.split('\u{0}').collect::<Vec<_>>());
            if code != 0 {
                return None;
            }
            Some(out.lines().filter(|l| l.starts_with("Remv "))
                .filter_map(|l| l.split_whitespace().nth(1).map(|n| n.split(':').next().unwrap().to_string()))
                .collect())
        }
        _ => None,
    }
}

// ---------- Login-Manager (Display-Manager) ----------

pub const DM_NAMES: [(&str, &str); 8] = [("sddm", "SDDM"), ("plasmalogin", "Plasma Login Manager"), ("gdm", "GDM"),
    ("lightdm", "LightDM"), ("cosmic-greeter", "COSMIC Greeter"), ("ly", "Ly"), ("greetd", "greetd"), ("lxdm", "LXDM")];
pub const DM_SWITCHABLE: [&str; 4] = ["sddm", "plasmalogin", "gdm", "lightdm"];
pub const UNIT_DIRS: [&str; 3] = ["/usr/lib/systemd/system", "/etc/systemd/system", "/lib/systemd/system"];

pub fn dry_dm_file() -> PathBuf {
    std::env::temp_dir().join("desddm-dry").join("display-manager")
}

pub fn active_display_manager() -> String {
    if dry() {
        return std::fs::read_to_string(dry_dm_file()).map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "plasmalogin".into());
    }
    match std::fs::read_link("/etc/systemd/system/display-manager.service") {
        Ok(p) => {
            let n = util::file_name(&p);
            n.strip_suffix(".service").map(String::from).unwrap_or(n)
        }
        Err(_) => String::new(),
    }
}

pub fn dm_unit_exists(name: &str) -> bool {
    if dry() {
        return true;
    }
    UNIT_DIRS.iter().any(|d| Path::new(d).join(format!("{}.service", name)).is_file())
}

pub fn sddm_installed() -> bool {
    if dry() {
        return dry_dm_file().parent().unwrap().join("sddm-installed").exists();
    }
    util::has("sddm") || dm_unit_exists("sddm")
}

pub fn dm_pretty(name: &str) -> String {
    DM_NAMES.iter().find(|(k, _)| *k == name).map(|(_, v)| v.to_string())
        .unwrap_or_else(|| if name.is_empty() { "–".into() } else { name.into() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_specs() {
        let have = |_: &str, n: &str| ["b", "c", "x"].contains(&n);
        let specs: Vec<String> = ["a|b", "?z", "c", "b", "q"].iter().map(|s| s.to_string()).collect();
        let (o, m) = resolve("pacman", &specs, &have);
        assert_eq!(o, vec!["b", "c"]);
        assert_eq!(m, vec!["q"]);
        let p: Vec<String> = vec!["pattern:kde".into(), "foo".into()];
        assert_eq!(install_commands("zypper", &p).len(), 3);
    }
}
