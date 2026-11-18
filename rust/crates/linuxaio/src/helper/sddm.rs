//! Root-Teil für SDDM-Themes: installieren (Archiv sicher entpacken), setzen, entfernen.

use super::{arg, dm_restore, dm_snapshot, dry, run, say, sv, write_conf, write_json_atomic, ArgErr};
use crate::{sddm, system};
use hyprstudio::util;
use regex::Regex;
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
use std::io::Read;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};

pub const MAX_ARCHIVE: u64 = 300 * 1024 * 1024;
pub const MAX_UNPACKED: u64 = 800 * 1024 * 1024;

/// QML-Import → Arch-Paket (Qt6-Greeter)
pub const QML_DEPS_QT6: [(&str, &str); 5] = [("Qt5Compat.GraphicalEffects", "qt6-5compat"),
    ("QtMultimedia", "qt6-multimedia"), ("QtQuick.VirtualKeyboard", "qt6-virtualkeyboard"),
    ("QtQuick.Controls", "qt6-declarative"), ("QtSvg", "qt6-svg")];
/// QML-Import → Arch-Paket (Qt5-Greeter)
pub const QML_DEPS_QT5: [(&str, &str); 6] = [("QtGraphicalEffects", "qt5-graphicaleffects"),
    ("QtQuick.Controls 2", "qt5-quickcontrols2"), ("QtQuick.Controls 1", "qt5-quickcontrols"),
    ("QtMultimedia", "qt5-multimedia"), ("QtQuick.VirtualKeyboard", "qt5-virtualkeyboard"), ("QtSvg", "qt5-svg")];

pub fn save_store_state(st: &Map<String, Value>) {
    let p = sddm::store_state_file();
    let _ = write_json_atomic(&p, st);
    let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o644));
}

/// Datei muss dem Benutzer gehören, der pkexec aufgerufen hat (kein Auslesen fremder Root-Dateien).
pub fn user_owned(m: &std::fs::Metadata) -> bool {
    if dry() {
        return true;
    }
    let uid = std::env::var("PKEXEC_UID").ok().filter(|s| !s.is_empty()).or_else(|| std::env::var("SUDO_UID").ok());
    match uid {
        Some(u) if super::digits(&u) => u.parse::<u32>().map(|n| n == m.uid()).unwrap_or(false),
        _ => false,
    }
}

pub fn is_image(path: &Path) -> bool {
    let mut head = [0u8; 8];
    let Ok(mut f) = std::fs::File::open(path) else { return false };
    let n = f.read(&mut head).unwrap_or(0);
    let h = &head[..n];
    h.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) || h.starts_with(&[0xFF, 0xD8, 0xFF])
}

// ---------- sicher entpacken ----------

fn safe_member(name: &str) -> bool {
    let p = Path::new(name);
    !p.is_absolute() && !p.components().any(|c| c == Component::ParentDir) && !name.starts_with('/')
        && !name.starts_with('\\')
}

fn decompressed(archive: &Path) -> Result<Box<dyn Read>, String> {
    let mut f = std::fs::File::open(archive).map_err(|e| e.to_string())?;
    let mut magic = [0u8; 6];
    let n = f.read(&mut magic).map_err(|e| e.to_string())?;
    let f = std::fs::File::open(archive).map_err(|e| e.to_string())?;
    let m = &magic[..n];
    Ok(if m.starts_with(&[0x1f, 0x8b]) {
        Box::new(flate2::read::GzDecoder::new(f))
    } else if m.starts_with(&[0xfd, b'7', b'z', b'X', b'Z', 0]) {
        Box::new(xz2::read::XzDecoder::new(f))
    } else if m.starts_with(b"BZh") {
        Box::new(bzip2::read::BzDecoder::new(f))
    } else if m.starts_with(&[0x28, 0xb5, 0x2f, 0xfd]) {
        return Err("zstd".into());
    } else {
        Box::new(f)
    })
}

/// Archiv nach `dest` entpacken: keine absoluten Pfade, kein „..“, keine Links/Geräte, Größenlimit.
pub fn extract(archive: &Path, dest: &Path) -> Result<(), String> {
    let mut total: u64 = 0;
    if let Ok(mut z) = std::fs::File::open(archive).map_err(|e| e.to_string())
        .and_then(|f| zip::ZipArchive::new(f).map_err(|e| e.to_string())) {
        for i in 0..z.len() {
            let mut m = z.by_index(i).map_err(|e| e.to_string())?;
            let name = m.name().to_string();
            if !safe_member(&name) {
                return Err("unsafe".into());
            }
            if m.unix_mode().map(|md| md & 0o170000 == 0o120000).unwrap_or(false) {
                continue;
            }
            total += m.size();
            if total > MAX_UNPACKED {
                return Err("toolarge".into());
            }
            let target = dest.join(&name);
            if m.is_dir() {
                std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
                continue;
            }
            if let Some(p) = target.parent() {
                std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }
            let mut out = std::fs::File::create(&target).map_err(|e| e.to_string())?;
            std::io::copy(&mut m, &mut out).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    // erst prüfen (wie getmembers), dann entpacken
    let mut ok_names = Vec::new();
    {
        let mut ar = tar::Archive::new(decompressed(archive)?);
        for e in ar.entries().map_err(|e| e.to_string())? {
            let e = e.map_err(|e| e.to_string())?;
            let name = String::from_utf8_lossy(&e.path_bytes()).into_owned();
            if !safe_member(&name) {
                return Err("unsafe".into());
            }
            let kind = e.header().entry_type();
            if !(kind.is_file() || kind.is_dir()) {
                continue; // keine Links, Geräte, FIFOs
            }
            total += e.size();
            if total > MAX_UNPACKED {
                return Err("toolarge".into());
            }
            ok_names.push(name);
        }
    }
    let set: BTreeSet<String> = ok_names.into_iter().collect();
    let mut ar = tar::Archive::new(decompressed(archive)?);
    ar.set_preserve_permissions(false);
    for e in ar.entries().map_err(|e| e.to_string())? {
        let mut e = e.map_err(|e| e.to_string())?;
        let name = String::from_utf8_lossy(&e.path_bytes()).into_owned();
        if !set.contains(&name) {
            continue;
        }
        let target = dest.join(name.trim_start_matches("./"));
        if e.header().entry_type().is_dir() {
            std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(p) = target.parent() {
            std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        let mut out = std::fs::File::create(&target).map_err(|e| e.to_string())?;
        std::io::copy(&mut e, &mut out).map_err(|e| e.to_string())?;
        // wie filter="data": ausführbar bleibt ausführbar, sonst 644
        let mode = e.header().mode().unwrap_or(0o644);
        let m = if mode & 0o100 != 0 { 0o755 } else { 0o644 };
        let _ = std::fs::set_permissions(&target, std::fs::Permissions::from_mode(m));
    }
    Ok(())
}

/// Alle Dateien unterhalb (rekursiv, Links werden nicht verfolgt), wie `Path.rglob("*")`.
pub fn walk(base: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![base.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        let mut subs = Vec::new();
        for e in rd.flatten() {
            let p = e.path();
            out.push(p.clone());
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                subs.push(p);
            }
        }
        subs.reverse();
        stack.extend(subs);
    }
    out
}

fn has_ext(d: &Path, ext: &str) -> bool {
    std::fs::read_dir(d).map(|r| r.flatten().any(|e| util::file_name(&e.path()).ends_with(ext))).unwrap_or(false)
}

pub fn find_theme_roots(base: &Path, archive_name: &str) -> Vec<(String, PathBuf)> {
    let mut roots: Vec<PathBuf> = Vec::new();
    for meta in walk(base).into_iter().filter(|p| util::file_name(p) == "metadata.desktop") {
        let d = meta.parent().unwrap().to_path_buf();
        if has_ext(&d, ".qml") && !roots.iter().any(|r| d.starts_with(r) && d != *r) {
            roots.push(d);
        }
    }
    let split = Regex::new(r"\.(tar|zip|tgz|txz|tbz2)").unwrap();
    roots.into_iter().map(|d| {
        let name = if d != base { util::file_name(&d) } else { split.split(archive_name).next().unwrap_or("").to_string() };
        (sddm::safe_name(&name), d)
    }).collect()
}

pub fn qml_deps(theme_dir: &Path, qt6: bool) -> Vec<String> {
    let table: &[(&str, &str)] = if qt6 { &QML_DEPS_QT6 } else { &QML_DEPS_QT5 };
    let mut found: BTreeSet<String> = BTreeSet::new();
    if !qt6 {
        found.insert("qt5-declarative".into()); // Qt5-Greeter braucht QtQuick 2 aus Qt5
    }
    for q in walk(theme_dir).into_iter().filter(|p| util::file_name(p).ends_with(".qml")) {
        let Ok(b) = std::fs::read(&q) else { continue };
        let text = String::from_utf8_lossy(&b);
        for (imp, pkg) in table {
            let re = Regex::new(&format!(r"(?m)^\s*import\s+{}\b", regex::escape(imp))).unwrap();
            if re.is_match(&text) {
                found.insert(pkg.to_string());
            }
        }
    }
    found.into_iter().collect()
}

pub fn install_deps(pkgs: &[String]) {
    if pkgs.is_empty() || system::detect_backend().as_deref() != Some("pacman") {
        return;
    }
    let have = if dry() { BTreeSet::new() } else { system::installed_packages("pacman") };
    let missing: Vec<String> = pkgs.iter().filter(|p| !have.contains(*p)).cloned()
        .collect::<BTreeSet<_>>().into_iter().collect();
    if missing.is_empty() {
        return;
    }
    println!("Abhängigkeiten: {}", missing.join(" "));
    let mut cmd = sv(&["pacman", "-S", "--needed", "--noconfirm"]);
    cmd.extend(missing);
    println!("$ {}", cmd.join(" "));
    if !dry() {
        let _ = std::process::Command::new(&cmd[0]).args(&cmd[1..]).stdin(std::process::Stdio::null()).status();
    }
}

pub fn fix_perms(path: &Path) {
    for p in walk(path) {
        let m = if p.is_dir() { 0o755 } else { 0o644 };
        let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(m));
    }
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755));
}

/// `shutil.copytree(src, dst, symlinks=False, ignore=<Links>)`
pub fn copytree(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for e in std::fs::read_dir(src)?.flatten() {
        let p = e.path();
        let ft = e.file_type()?;
        if ft.is_symlink() {
            continue;
        }
        let t = dst.join(e.file_name());
        if ft.is_dir() {
            copytree(&p, &t)?;
        } else {
            util::copy2(&p, &t)?;
        }
    }
    Ok(())
}

/// Temporärer Ordner, der beim Verlassen gelöscht wird (`tempfile.TemporaryDirectory`).
pub struct TempDir(pub PathBuf);

impl TempDir {
    pub fn new(prefix: &str) -> std::io::Result<TempDir> {
        let base = std::env::temp_dir();
        for i in 0..1000u32 {
            let n = format!("{}{:x}{:x}", prefix, std::process::id(),
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0)
                    .wrapping_add(i));
            let p = base.join(n);
            if std::fs::create_dir(&p).is_ok() {
                let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o700));
                return Ok(TempDir(p));
            }
        }
        Err(std::io::Error::other("tempdir"))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Archiv prüfen: reguläre Datei, Größe, Endung, gehört dem Aufrufer.
pub fn archive_valid(archive: &Path) -> Option<bool> {
    let st = std::fs::symlink_metadata(archive).ok()?;
    Some(st.file_type().is_file() && st.len() <= MAX_ARCHIVE && sddm::archive_ok(&util::file_name(archive))
        && user_owned(&st))
}

pub fn do_install(archive_arg: &str, store_id: &str) -> i32 {
    let archive = PathBuf::from(archive_arg);
    say("STEP", "sddm.unpack");
    say("PROGRESS", "10");
    if archive_valid(&archive) != Some(true) {
        say("FAIL", "sddm.archive");
        return 2;
    }
    let mut state = sddm::load_store_state();
    let dest_dir = sddm::themes_dir();
    let _ = std::fs::create_dir_all(&dest_dir);
    let mut installed = Vec::new();
    {
        let Ok(tmp) = TempDir::new("desddm-") else {
            say("FAIL", "sddm.archive");
            return 3;
        };
        let work = tmp.0.clone();
        if let Err(e) = extract(&archive, &work) {
            println!("Entpacken fehlgeschlagen: {}", e);
            say("FAIL", "sddm.archive");
            return 3;
        }
        let roots = find_theme_roots(&work, &util::file_name(&archive));
        if roots.is_empty() {
            say("FAIL", "sddm.notheme");
            return 4;
        }
        say("STEP", "sddm.copy");
        say("PROGRESS", "50");
        for (name, src) in roots {
            if !sddm::NAME_RE.is_match(&name) {
                continue;
            }
            let target = dest_dir.join(&name);
            if target.exists() {
                if !state.contains_key(&name) {
                    println!("{}: existiert schon (nicht von hier installiert) – übersprungen", name);
                    continue;
                }
                let _ = std::fs::remove_dir_all(&target);
            }
            if copytree(&src, &target).is_err() {
                continue;
            }
            if !dry() {
                fix_perms(&target);
            }
            let qt6 = sddm::theme_info(&target).map(|i| i.qt6).unwrap_or(false);
            install_deps(&qml_deps(&target, qt6));
            state.insert(name.clone(), json!({"store": store_id, "archive": util::file_name(&archive)}));
            installed.push(name);
            println!("Installiert: {}", target.display());
        }
    }
    if installed.is_empty() {
        say("FAIL", "sddm.exists");
        return 5;
    }
    save_store_state(&state);
    println!("@@THEME {}", installed[0]);
    say("PROGRESS", "100");
    say("OK", "");
    0
}

pub fn do_set(name: &str) -> i32 {
    if !sddm::NAME_RE.is_match(name) || !sddm::themes_dir().join(name).join("metadata.desktop").is_file() {
        say("FAIL", "sddm.notheme");
        return 2;
    }
    say("STEP", "sddm.set");
    say("PROGRESS", "40");
    // wie die KCM: /etc/sddm.conf.d/kde_settings.conf
    let _ = write_conf(&sddm::kde_settings_file(), "Theme", "Current", name);
    println!("{}: [Theme] Current={}", sddm::kde_settings_file().display(), name);
    let main_conf = sddm::main_conf_file();
    if main_conf.is_file() && sddm::read_ini(&main_conf).has_option("Theme", "Current") {
        let _ = write_conf(&main_conf, "Theme", "Current", name); // /etc/sddm.conf hat Vorrang
        println!("{}: [Theme] Current={}", main_conf.display(), name);
    }
    say("PROGRESS", "100");
    say("OK", "");
    0
}

pub fn do_remove(name: &str) -> i32 {
    let mut state = sddm::load_store_state();
    if !sddm::NAME_RE.is_match(name) || !state.contains_key(name) {
        say("FAIL", "sddm.foreign");
        return 2;
    }
    if sddm::current_theme() == name {
        say("FAIL", "sddm.active");
        return 3;
    }
    say("STEP", "sddm.remove");
    let target = sddm::themes_dir().join(name);
    if target.is_dir() && !target.is_symlink() {
        let _ = std::fs::remove_dir_all(&target);
        println!("Entfernt: {}", target.display());
    }
    state.shift_remove(name);
    save_store_state(&state);
    say("PROGRESS", "100");
    say("OK", "");
    0
}

pub fn do_dm_install() -> i32 {
    let Some(backend) = system::detect_backend() else {
        say("FAIL", "nobackend");
        return 11;
    };
    say("STEP", "dm.install");
    say("PROGRESS", "10");
    let before = dm_snapshot();
    let env: Vec<(&str, &str)> = if backend == "apt" { vec![("DEBIAN_FRONTEND", "noninteractive")] } else { vec![] };
    let cmds = system::install_commands(&backend, &["sddm".to_string()]);
    for (i, cmd) in cmds.iter().enumerate() {
        say("PROGRESS", &(10 + 80 * i / cmds.len()).to_string());
        if run(cmd, &env) != 0 {
            dm_restore(&before);
            say("FAIL", "install");
            return 1;
        }
    }
    dm_restore(&before); // nur installieren, nicht umschalten
    if dry() {
        let f = system::dry_dm_file().parent().unwrap().join("sddm-installed");
        let _ = std::fs::create_dir_all(f.parent().unwrap());
        let _ = std::fs::write(f, "1");
    }
    say("PROGRESS", "100");
    say("OK", "");
    0
}

pub fn do_dm_enable(name: &str) -> i32 {
    if !system::DM_SWITCHABLE.contains(&name) || !system::dm_unit_exists(name) {
        say("FAIL", "dm.unit");
        return 2;
    }
    say("STEP", "dm.enable");
    say("PROGRESS", "30");
    if run(&sv(&["systemctl", "enable", "--force", &format!("{}.service", name)]), &[]) != 0 {
        say("FAIL", "dm.unit");
        return 1;
    }
    let debian = Path::new("/etc/X11/default-display-manager");
    if let (false, true, Some(exe)) = (dry(), debian.is_file(), util::which(name)) {
        let _ = std::fs::write(debian, format!("{}\n", exe.display()));
        println!("{}: {}", debian.display(), exe.display());
    }
    if dry() {
        let f = system::dry_dm_file();
        let _ = std::fs::create_dir_all(f.parent().unwrap());
        let _ = std::fs::write(f, name);
    }
    say("PROGRESS", "100");
    say("OK", "");
    0
}

pub fn main(argv: &[String]) -> Result<i32, ArgErr> {
    let action = arg(argv, 0)?;
    match action.as_str() {
        "dm-install" => Ok(do_dm_install()),
        "dm-enable" => Ok(do_dm_enable(&arg(argv, 1)?)),
        "sddm-install" => {
            let a = arg(argv, 1)?;
            let store = argv[2..].iter().find_map(|a| a.strip_prefix("--store=")).unwrap_or("");
            Ok(do_install(&a, if super::digits(store) { store } else { "" }))
        }
        "sddm-set" => Ok(do_set(&arg(argv, 1)?)),
        "sddm-remove" => Ok(do_remove(&arg(argv, 1)?)),
        _ => Err(ArgErr(format!("'{}'", action))),
    }
}
