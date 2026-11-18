//! Root-Helfer (läuft per pkexec). Nimmt nur DE-/Varianten-IDs an, nie Paketnamen.
//!
//! Ausgabe-Protokoll für die GUI:
//!   @@STEP <text>   @@PROGRESS <0-100>   @@OK   @@FAIL <text>

pub mod gdm;
pub mod plymouth;
pub mod sddm;

use crate::catalog::{self, Desktop};
use crate::{ini::Ini, installers, pyjson, system};
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const DM_LINK: &str = "/etc/systemd/system/display-manager.service";
pub const DEBIAN_DM: &str = "/etc/X11/default-display-manager";

/// Fehler „falsche Argumente“ (Python: IndexError/KeyError → @@FAIL args …)
pub struct ArgErr(pub String);

pub fn dry() -> bool {
    std::env::var("DEMANAGER_DRYRUN").as_deref() == Ok("1")
}

pub fn say(tag: &str, text: &str) {
    println!("{}", format!("@@{} {}", tag, text).trim_end());
}

pub fn arg(argv: &[String], i: usize) -> Result<String, ArgErr> {
    argv.get(i).cloned().ok_or_else(|| ArgErr("list index out of range".into()))
}

pub fn state_path() -> PathBuf {
    if dry() { std::env::temp_dir().join("linuxaio-dry-state.json") } else { PathBuf::from(system::STATE_FILE) }
}

pub fn load_state() -> Map<String, Value> {
    system::load_state_file(&state_path())
}

/// Zustand atomar schreiben (wie `json.dumps(st, indent=2, sort_keys=True)` + `os.replace`).
pub fn write_json_atomic(p: &Path, st: &Map<String, Value>) -> std::io::Result<()> {
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d)?;
    }
    let tmp = p.with_extension("tmp");
    std::fs::write(&tmp, pyjson::dumps(&Value::Object(st.clone()), Some(2), true, true))?;
    std::fs::rename(&tmp, p)
}

pub fn save_state(st: &Map<String, Value>) {
    let _ = write_json_atomic(&state_path(), st);
}

/// Befehl ausführen, Ausgabe (stdout+stderr) zeilenweise weitergeben.
pub fn run(cmd: &[String], env: &[(&str, &str)]) -> i32 {
    println!("$ {}", cmd.join(" "));
    if dry() {
        return 0;
    }
    let mut c = Command::new("sh");
    c.arg("-c").arg("exec \"$@\" 2>&1").arg("sh").args(cmd);
    c.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    for (k, v) in env {
        c.env(k, v);
    }
    let Ok(mut child) = c.spawn() else { return 127 };
    if let Some(out) = child.stdout.take() {
        let mut r = BufReader::new(out);
        let mut buf = Vec::new();
        while r.read_until(b'\n', &mut buf).map(|n| n > 0).unwrap_or(false) {
            println!("{}", String::from_utf8_lossy(&buf).trim_end());
            buf.clear();
        }
    }
    child.wait().map(|s| s.code().unwrap_or(1)).unwrap_or(1)
}

pub fn sv(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

fn check(backend: &str, name: &str) -> bool {
    dry() || system::available(backend, name)
}

// ---------- Display-Manager unverändert lassen (SDDM bleibt bei KDE) ----------

#[derive(Clone, Default)]
pub struct DmSnap {
    pub link: Option<String>,
    pub debian: Option<String>,
}

pub fn dm_snapshot() -> DmSnap {
    DmSnap {
        link: std::fs::read_link(DM_LINK).ok().map(|p| p.display().to_string()),
        debian: std::fs::read_to_string(DEBIAN_DM).ok(),
    }
}

pub fn dm_restore(before: &DmSnap) {
    if dry() {
        return;
    }
    let after = dm_snapshot();
    if let Some(d) = &before.debian {
        if after.debian.as_ref() != Some(d) {
            let _ = std::fs::write(DEBIAN_DM, d);
            println!("Display-Manager-Auswahl wiederhergestellt");
        }
    }
    if let Some(l) = &before.link {
        if !l.is_empty() && after.link.as_ref() != Some(l) {
            let unit = hyprstudio::util::file_name(Path::new(l));
            run(&sv(&["systemctl", "enable", "--force", &unit]), &[]);
        }
    }
}

// ---------- Aktionen ----------

fn strs(v: Option<&Value>) -> BTreeSet<String> {
    v.and_then(|x| x.as_array()).map(|a| a.iter().filter_map(|s| s.as_str().map(String::from)).collect())
        .unwrap_or_default()
}

pub fn do_install(de: &Desktop, variant: &str, backend: &str) -> i32 {
    say("STEP", "resolve");
    say("PROGRESS", "5");
    let (pkgs, missing) = system::resolve(backend, &de.specs(variant, backend), &check);
    if !missing.is_empty() {
        say("FAIL", &format!("missing {}", missing.join(" ")));
        return 2;
    }
    println!("Pakete: {}", pkgs.join(" "));
    let before = if dry() { BTreeSet::new() } else { system::installed_packages(backend) };
    let dm_before = dm_snapshot();
    let cmds = system::install_commands(backend, &pkgs);
    let env: Vec<(&str, &str)> = if backend == "apt" { vec![("DEBIAN_FRONTEND", "noninteractive")] } else { vec![] };
    for (i, cmd) in cmds.iter().enumerate() {
        say("STEP", "install");
        say("PROGRESS", &(10 + 80 * i / cmds.len()).to_string());
        if run(cmd, &env) != 0 {
            dm_restore(&dm_before);
            say("FAIL", "install");
            return 1;
        }
    }
    dm_restore(&dm_before);
    let after: BTreeSet<String> = if dry() { pkgs.iter().cloned().collect() } else { system::installed_packages(backend) };
    let mut st = load_state();
    let prev = st.get(&de.id).cloned().unwrap_or(json!({}));
    let added: BTreeSet<String> = after.difference(&before).cloned().chain(strs(prev.get("added"))).collect();
    let requested: BTreeSet<String> = pkgs.iter().cloned().chain(strs(prev.get("requested"))).collect();
    st.insert(de.id.clone(), json!({"variant": variant, "backend": backend,
        "added": added.into_iter().collect::<Vec<_>>(), "requested": requested.into_iter().collect::<Vec<_>>()}));
    save_state(&st);
    say("PROGRESS", "100");
    say("OK", "");
    0
}

pub fn do_remove(de: &Desktop, backend: &str, running: &str) -> i32 {
    if running == de.id {
        say("FAIL", "running");
        return 3;
    }
    let mut st = load_state();
    let entry = st.get(&de.id).filter(|e| e.as_object().map(|o| !o.is_empty()).unwrap_or(false)).cloned();
    let (targets, allowed): (Vec<String>, Option<BTreeSet<String>>) = match entry {
        Some(entry) => {
            let mut others = BTreeSet::new();
            for (k, v) in &st {
                if *k != de.id {
                    others.extend(strs(v.get("added")));
                }
            }
            let added: BTreeSet<String> = strs(entry.get("added")).difference(&others).cloned().collect();
            let req = strs(entry.get("requested"));
            let targets: Vec<String> = if backend == "pacman" { added.iter().cloned().collect() }
                else { req.difference(&others).cloned().collect() };
            let allowed: BTreeSet<String> = added.union(&req).cloned().collect();
            (targets, Some(allowed))
        }
        None => {
            // Nicht über Linux AIO installiert: nur die Hauptpakete der Standard-Variante, die nichts anderes braucht
            let (t, _) = system::resolve(backend, &de.specs("standard", backend), &check);
            (t.into_iter().filter(|p| p != "xorg-server").collect(), None)
        }
    };
    if targets.is_empty() {
        say("FAIL", "nothing");
        return 4;
    }
    say("STEP", "check");
    say("PROGRESS", "10");
    if !dry() {
        if let Some(allowed) = &allowed {
            if let Some(sim) = system::simulate_remove(backend, &targets) {
                let extra: Vec<String> = sim.difference(allowed).cloned().collect();
                if !extra.is_empty() {
                    say("FAIL", &format!("foreign {}", extra.iter().take(15).cloned().collect::<Vec<_>>().join(" ")));
                    return 5;
                }
            }
        }
    }
    say("STEP", "remove");
    say("PROGRESS", "30");
    if run(&system::remove_command(backend, &targets), &[]) != 0 {
        say("FAIL", "remove");
        return 1;
    }
    st.shift_remove(&de.id);
    save_state(&st);
    say("PROGRESS", "100");
    say("OK", "");
    0
}

/// (Name, UID) des Benutzers, der pkexec/sudo aufgerufen hat.
pub fn invoking_user() -> Option<(String, u32)> {
    for var in ["PKEXEC_UID", "SUDO_UID"] {
        if let Ok(uid) = std::env::var(var) {
            if !uid.is_empty() && uid.chars().all(|c| c.is_ascii_digit()) {
                let Ok(n) = uid.parse::<u32>() else { continue };
                if let Some((name, _)) = pw_entry(n) {
                    return Some((name, n));
                }
            }
        }
    }
    None
}

/// (Name, Home) aus der Benutzerdatenbank.
pub fn pw_entry(uid: u32) -> Option<(String, PathBuf)> {
    unsafe {
        let pw = libc::getpwuid(uid);
        if pw.is_null() {
            return None;
        }
        let name = std::ffi::CStr::from_ptr((*pw).pw_name).to_string_lossy().into_owned();
        let dir = std::ffi::CStr::from_ptr((*pw).pw_dir).to_string_lossy().into_owned();
        Some((name, PathBuf::from(dir)))
    }
}

fn quiet_status(cmd: &[&str]) -> bool {
    Command::new(cmd[0]).args(&cmd[1..]).stdout(Stdio::null()).stderr(Stdio::null()).status()
        .map(|s| s.success()).unwrap_or(false)
}

pub fn do_default(de: &Desktop) -> i32 {
    let sess: Option<PathBuf> = if dry() {
        let s = PathBuf::from(format!("/usr/share/wayland-sessions/{}.desktop", de.sessions[0]));
        let f = system::dry_dm_file().parent().unwrap().join("default-session");
        let _ = std::fs::create_dir_all(f.parent().unwrap());
        let _ = std::fs::write(&f, s.file_stem().unwrap().to_string_lossy().as_bytes());
        Some(s)
    } else {
        system::installed_session(de)
    };
    let Some(sess) = sess else {
        say("FAIL", "notinstalled");
        return 2;
    };
    let name = sess.file_stem().unwrap().to_string_lossy().into_owned();
    say("STEP", "default");
    say("PROGRESS", "30");
    let user = if dry() { None } else { invoking_user() };
    if let Some((uname, uid)) = user {
        let obj = format!("/org/freedesktop/Accounts/User{}", uid);
        let ok = quiet_status(&["busctl", "call", "org.freedesktop.Accounts", &obj, "org.freedesktop.Accounts.User",
                                "SetSession", "s", &name]);
        if !ok {
            quiet_status(&["busctl", "call", "org.freedesktop.Accounts", &obj, "org.freedesktop.Accounts.User",
                           "SetXSession", "s", &name]);
        }
        println!("AccountsService: {} -> {}", uname, name);
    }
    if sddm_present() {
        write_ini(Path::new(system::SDDM_STATE), "Last", "Session", &sess.display().to_string());
        println!("SDDM: Last.Session = {}", sess.display());
    }
    say("PROGRESS", "100");
    say("OK", "");
    0
}

fn sddm_present() -> bool {
    dry() || Path::new(system::SDDM_STATE).parent().unwrap().is_dir()
}

/// configparser-Datei ändern; Besitzer bleibt erhalten.
pub fn write_conf(path: &Path, section: &str, key: &str, value: &str) -> std::io::Result<()> {
    use std::os::unix::fs::MetadataExt;
    let mut cp = Ini::new();
    let owner = path.metadata().ok().filter(|_| path.exists());
    if owner.is_some() {
        cp.read_file(path);
    }
    cp.set(section, key, value);
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    std::fs::write(path, cp.write_string(false))?;
    if let Some(m) = owner {
        let _ = std::os::unix::fs::chown(path, Some(m.uid()), Some(m.gid()));
    }
    Ok(())
}

fn write_ini(path: &Path, section: &str, key: &str, value: &str) {
    if dry() {
        return;
    }
    let _ = write_conf(path, section, key, value);
}

fn is_digits(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

pub fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

pub fn main(argv: &[String]) -> i32 {
    if !dry() && !is_root() {
        say("FAIL", "root");
        return 10;
    }
    let first = argv.first().map(|s| s.as_str()).unwrap_or("");
    if first == "pkg-install" {
        // nur feste Schlüssel aus installers::PACKAGES, nie freie Paketnamen
        let backend = system::detect_backend();
        let pkgs = installers::packages_for(argv.get(1).map(|s| s.as_str()).unwrap_or(""),
                                            backend.as_deref().unwrap_or(""));
        if pkgs.is_empty() {
            say("FAIL", "args pkg");
            return 12;
        }
        say("STEP", "install");
        for cmd in system::install_commands(backend.as_deref().unwrap_or(""), &pkgs) {
            if run(&cmd, &[]) != 0 {
                say("FAIL", "missing");
                return 1;
            }
        }
        say("PROGRESS", "100");
        say("OK", "");
        return 0;
    }
    if first == "yay-install" {
        // nur das selbst gebaute yay-bin-Paket aus dem Cache des aufrufenden Benutzers
        let f = PathBuf::from(argv.get(1).cloned().unwrap_or_default());
        let home = invoking_user().and_then(|(_, uid)| pw_entry(uid)).map(|(_, h)| h);
        let ok_dir = home.map(|h| {
            let want = std::fs::canonicalize(h.join(".cache/LabtosoStudios/linuxaio/yay/yay-bin"))
                .unwrap_or_else(|_| h.join(".cache/LabtosoStudios/linuxaio/yay/yay-bin"));
            let par = std::fs::canonicalize(&f).ok().and_then(|p| p.parent().map(|x| x.to_path_buf()));
            par.as_ref() == Some(&want)
        }).unwrap_or(false);
        let n = hyprstudio::util::file_name(&f);
        if !(dry() || (ok_dir && n.starts_with("yay-bin-") && n.contains(".pkg.tar.") && f.is_file())) {
            say("FAIL", "args pkg");
            return 12;
        }
        say("STEP", "install");
        if run(&sv(&["pacman", "-U", "--noconfirm", "--needed", &f.display().to_string()]), &[]) != 0 {
            say("FAIL", "missing");
            return 1;
        }
        say("PROGRESS", "100");
        say("OK", "");
        return 0;
    }
    let sub = if first.starts_with("gdm-") { Some(gdm::main(argv)) }
        else if first.starts_with("ply-") { Some(plymouth::main(argv)) }
        else if first.starts_with("sddm-") || first.starts_with("dm-") { Some(sddm::main(argv)) }
        else { None };
    if let Some(r) = sub {
        return match r {
            Ok(c) => c,
            Err(ArgErr(e)) => {
                say("FAIL", &format!("args {}", e));
                12
            }
        };
    }
    let Some(backend) = system::detect_backend() else {
        say("FAIL", "nobackend");
        return 11;
    };
    let res = (|| -> Result<i32, ArgErr> {
        let action = arg(argv, 0)?;
        let de_id = arg(argv, 1)?;
        let de = catalog::get(&de_id).ok_or_else(|| ArgErr(format!("'{}'", de_id)))?;
        match action.as_str() {
            "install" => {
                let variant = arg(argv, 2)?;
                if !catalog::VARIANTS.contains(&variant.as_str()) || !de.supports(&variant, &backend) {
                    return Err(ArgErr(format!("'{}'", variant)));
                }
                Ok(do_install(de, &variant, &backend))
            }
            "remove" => {
                let running = argv[2..].iter().find_map(|a| a.strip_prefix("--running=")).unwrap_or("");
                Ok(do_remove(de, &backend, running))
            }
            "default" => Ok(do_default(de)),
            _ => Err(ArgErr(format!("'{}'", action))),
        }
    })();
    match res {
        Ok(c) => c,
        Err(ArgErr(e)) => {
            say("FAIL", &format!("args {}", e));
            12
        }
    }
}

pub(crate) fn digits(s: &str) -> bool {
    is_digits(s)
}
