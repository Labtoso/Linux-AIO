//! Sonstiges › Leistung: prüfen, warum Programme unter Hyprland langsam starten, ewig laden oder abstürzen –
//! und beheben (Portal, zram, Energieprofil, Effekte, Speicherplatz, Abstürze).

use crate::jsonx::{self, Get, Obj};
use crate::model::{self, Log, MARK};
use crate::util;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const PORTAL_FILES: &[(&str, [&str; 3])] = &[
    ("hyprland", ["/usr/lib/xdg-desktop-portal-hyprland", "/usr/libexec/xdg-desktop-portal-hyprland",
        "/usr/share/xdg-desktop-portal/portals/hyprland.portal"]),
    ("gtk", ["/usr/lib/xdg-desktop-portal-gtk", "/usr/libexec/xdg-desktop-portal-gtk",
        "/usr/share/xdg-desktop-portal/portals/gtk.portal"]),
];
const PORTAL_PKGS: &[&str] = &["xdg-desktop-portal-hyprland", "xdg-desktop-portal-gtk"];
pub const ZRAM_CONF: &str = "/etc/systemd/zram-generator.conf";
pub const DBUS_ENV: &str = "dbus-update-activation-environment --systemd --all";

pub fn zram_text() -> String {
    format!("# {}\n[zram0]\nzram-size = ram / 2\ncompression-algorithm = zstd\n", MARK)
}

fn zram_pkg(pm: &str) -> Option<&'static str> {
    match pm {
        "pacman" | "dnf" | "zypper" => Some("zram-generator"),
        "apt" => Some("systemd-zram-generator"),
        _ => None,
    }
}

pub fn default() -> Obj {
    jsonx::obj(json!({"portal_env": false, "light": false}))
}

pub fn perf_state(saved: Option<&Obj>) -> Obj {
    let mut s = default();
    if let Some(saved) = saved {
        for (k, v) in saved {
            if let Some(cur) = s.get(k) {
                if jsonx::same_type(v, cur) {
                    s.insert(k.clone(), v.clone());
                }
            }
        }
    }
    s
}

// ---------- Prüfen ----------

fn meminfo() -> HashMap<String, u64> {
    let mut out = HashMap::new();
    for ln in util::read_text(Path::new("/proc/meminfo")).lines() {
        if let Some((k, v)) = ln.split_once(':') {
            if let Some(n) = v.split_whitespace().next().and_then(|x| x.parse::<u64>().ok()) {
                out.insert(k.to_string(), n * 1024);
            }
        }
    }
    out
}

fn swaps() -> Vec<String> {
    util::read_text(Path::new("/proc/swaps")).lines().skip(1).filter(|l| !l.trim().is_empty())
        .filter_map(|l| l.split_whitespace().next().map(String::from)).collect()
}

fn run(cmd: &[&str], timeout: u64) -> String {
    util::output(cmd, timeout)
}

fn files_with(hdir: &Path, ext: &str) -> Vec<PathBuf> {
    std::fs::read_dir(hdir).map(|rd| rd.flatten().map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == ext).unwrap_or(false)).collect()).unwrap_or_default()
}

/// Bekommt der D-Bus die Wayland-Umgebung (nötig für Portale)? uwsm macht das selbst.
fn dbus_env_started(hdir: &Path, hypr: &Obj) -> bool {
    if hypr.strs("autostart").iter().any(|c| c.contains("dbus-update-activation-environment")) {
        return true;
    }
    for ext in ["lua", "conf"] {
        for f in files_with(hdir, ext) {
            if util::read_text(&f).contains("dbus-update-activation-environment") {
                return true;
            }
        }
    }
    run(&["ps", "-eo", "comm"], 5).contains("uwsm")
}

/// [(Programm, Anzahl)] aus systemd-coredump der letzten Tage, häufigste zuerst.
pub fn crashes(days: u32) -> Vec<(String, i64)> {
    if model::dry() {
        return vec![("firefox".into(), 3), ("electron".into(), 2), ("waybar".into(), 1)];
    }
    if !util::has("coredumpctl") {
        return vec![];
    }
    let since = format!("--since=-{}d", days);
    let out = run(&["coredumpctl", "list", &since, "--no-pager", "--json=short"], 20);
    let mut counts: Vec<(String, i64)> = Vec::new();
    let mut add = |name: String| match counts.iter_mut().find(|x| x.0 == name) {
        Some(x) => x.1 += 1,
        None => counts.push((name, 1)),
    };
    let base = |p: &str| util::file_name(Path::new(p));
    let text = if out.trim().is_empty() { "[]" } else { out.as_str() };
    match serde_json::from_str::<Value>(text) {
        Ok(Value::Array(a)) => {
            for e in a {
                let exe = [e.s("exe"), e.s("comm")].into_iter().find(|x| !x.is_empty()).unwrap_or("?").to_string();
                add(base(&exe));
            }
        }
        Ok(_) => {}
        Err(_) => {
            for ln in out.lines().skip(1) {
                if let Some(last) = ln.split_whitespace().last() {
                    add(base(last));
                }
            }
        }
    }
    counts.sort_by_key(|x| -x.1);
    counts
}

pub fn hypr_crash_reports(days: u64) -> usize {
    let d = util::xdg_cache().join("hyprland");
    let limit = std::time::SystemTime::now() - std::time::Duration::from_secs(days * 86400);
    std::fs::read_dir(&d).map(|rd| rd.flatten().filter(|e| {
        let n = e.file_name().to_string_lossy().to_string();
        n.starts_with("hyprlandCrashReport") && n.ends_with(".txt")
            && e.metadata().and_then(|m| m.modified()).map(|t| t > limit).unwrap_or(false)
    }).count()).unwrap_or(0)
}

fn disk_usage(path: &str) -> (u64, u64) {
    let c = std::ffi::CString::new(path).unwrap();
    let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut st) } != 0 {
        return (0, 1);
    }
    (st.f_bavail as u64 * st.f_frsize as u64, (st.f_blocks as u64 * st.f_frsize as u64).max(1))
}

/// Alle Prüfungen – läuft im Hintergrund (coredumpctl kann dauern). Je Punkt: ok, Wert, Details.
pub fn check(hdir: &Path, hypr: &Obj, perf: &Obj) -> Obj {
    let passes = hypr.get("blur_passes").cloned().unwrap_or(json!(1));
    let gb = 1u64 << 30;
    let crash_list = |c: Vec<(String, i64)>| Value::Array(c.into_iter().map(|(a, b)| json!([a, b])).collect());
    if model::dry() {
        return jsonx::obj(json!({
            "portal": {"ok": false, "missing": ["gtk"], "env": false},
            "swap": {"ok": false, "ram": 8 * gb, "avail": 2 * gb, "swap": 0},
            "power": {"ok": false, "profile": "power-saver", "ac": true},
            "effects": {"ok": perf.b("light") || passes.as_f64().unwrap_or(1.0) <= 2.0, "passes": passes},
            "disk": {"ok": true, "free": 120 * gb, "total": 500 * gb},
            "crashes": crash_list(crashes(7)), "hypr_crashes": 0}));
    }
    let missing: Vec<&str> = PORTAL_FILES.iter().filter(|(_, files)| !files.iter().any(|f| Path::new(f).exists()))
        .map(|(k, _)| *k).collect();
    let env = perf.b("portal_env") || dbus_env_started(hdir, hypr);
    let mem = meminfo();
    let sw = swaps();
    let prof = if util::has("powerprofilesctl") { run(&["powerprofilesctl", "get"], 5).trim().to_string() } else { String::new() };
    let online: Vec<PathBuf> = glob::glob("/sys/class/power_supply/A*/online").map(|g| g.flatten().collect())
        .unwrap_or_default();
    let ac = online.is_empty() || online.iter().any(|p| util::read_text(p).trim() == "1");
    let (free, total) = disk_usage("/");
    let effects_ok = perf.b("light") || (passes.as_f64().unwrap_or(1.0) <= 2.0 && !hypr.b("blur_xray"));
    jsonx::obj(json!({
        "portal": {"ok": missing.is_empty() && env, "missing": missing, "env": env},
        "swap": {"ok": !sw.is_empty(), "ram": mem.get("MemTotal").copied().unwrap_or(0),
            "avail": mem.get("MemAvailable").copied().unwrap_or(0), "swap": mem.get("SwapTotal").copied().unwrap_or(0)},
        "power": {"ok": prof.is_empty() || prof == "performance" || (prof == "balanced" && !ac), "profile": prof, "ac": ac},
        "effects": {"ok": effects_ok, "passes": passes},
        "disk": {"ok": free as f64 / total as f64 > 0.1, "free": free, "total": total},
        "crashes": crash_list(crashes(7)), "hypr_crashes": hypr_crash_reports(7),
    }))
}

// ---------- Beheben ----------

fn install_cmd(pm: &str) -> Option<Vec<&'static str>> {
    Some(match pm {
        "pacman" => vec!["pacman", "-S", "--needed", "--noconfirm"],
        "apt" => vec!["apt-get", "install", "-y"],
        "dnf" => vec!["dnf", "install", "-y"],
        "zypper" => vec!["zypper", "-n", "in"],
        _ => return None,
    })
}

pub fn portal_command(pm: Option<&str>) -> Vec<String> {
    let Some(mut c) = install_cmd(pm.unwrap_or("")) else { return vec![] };
    c.extend(PORTAL_PKGS);
    c.into_iter().map(String::from).collect()
}

/// zram einrichten: Paket, Einstellung (eine vorhandene bleibt), sofort starten – als ein root-Befehl.
pub fn zram_command(pm: Option<&str>) -> Vec<String> {
    let pm = pm.unwrap_or("");
    let Some(pkg) = zram_pkg(pm) else { return vec![] };
    let inst = format!("{} {}", install_cmd(pm).unwrap().join(" "), pkg);
    let text = zram_text().replace('\n', "\\n");
    vec!["sh".into(), "-c".into(), format!("{} && ( [ -f {} ] || printf '{}' > {} ) \
        && systemctl daemon-reload && systemctl start systemd-zram-setup@zram0.service", inst, ZRAM_CONF, text, ZRAM_CONF)]
}

pub fn power_command(profile: &str) -> Vec<String> {
    if util::has("powerprofilesctl") || model::dry() {
        vec!["powerprofilesctl".into(), "set".into(), profile.into()]
    } else {
        vec![]
    }
}

/// hyprstudio_perf.lua/.conf: D-Bus-Umgebung beim Start, leichtere Effekte.
pub fn content(perf: &Obj, fmt: &str) -> String {
    let c = if fmt == "lua" { "--" } else { "#" };
    let mut out = vec![format!("{} {} – Leistung, automatisch erzeugt (Sonstiges › Leistung)", c, MARK)];
    if perf.b("portal_env") {
        if fmt == "lua" {
            out.push("hl.on(\"hyprland.start\", function ()".into());
            out.push(format!("    hl.exec_cmd(\"{}\")", DBUS_ENV));
            out.push("end)".into());
        } else {
            out.push(format!("exec-once = {}", DBUS_ENV));
        }
    }
    if perf.b("light") {
        let lines: &[&str] = if fmt == "lua" {
            &["hl.config({", "    decoration = {",
                "        blur = { passes = 1, size = 3, new_optimizations = true, xray = false },",
                "        shadow = { enabled = false },", "    },", "    misc = { vfr = true },", "})"]
        } else {
            &["decoration {", "    blur {", "        passes = 1", "        size = 3", "        new_optimizations = true",
                "        xray = false", "    }", "    shadow {", "        enabled = false", "    }", "}", "misc {",
                "    vfr = true", "}"]
        };
        out.extend(lines.iter().map(|s| s.to_string()));
    }
    if out.len() == 1 {
        out.push(format!("{} nichts eingeschaltet", c));
    }
    out.join("\n") + "\n"
}

pub fn write(hdir: &Path, perf: &Obj, log: Log) -> bool {
    crate::laptopkeys::include_extra(hdir, "perf", &|fmt| content(perf, fmt), "Leistung", log).is_some()
}

pub fn size(n: u64) -> String {
    format!("{:.1} GB", n as f64 / (1u64 << 30) as f64)
}
