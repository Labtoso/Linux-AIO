//! Root-Teil für Plymouth: einrichten (Paket, Hook, splash), Themes installieren/setzen/entfernen.

use super::sddm::{archive_valid, copytree, extract, fix_perms, is_image, user_owned, walk, TempDir};
use super::{arg, dm_restore, dm_snapshot, dry, run, say, sv, write_conf, write_json_atomic, ArgErr};
use crate::{plymouth, sddm, system};
use hyprstudio::util;
use regex::Regex;
use serde_json::{json, Map, Value};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

pub const BOOT_ENTRY_DIRS: [&str; 3] = ["/boot/loader/entries", "/efi/loader/entries", "/boot/efi/loader/entries"];
pub const LIMINE_FILES: [&str; 5] = ["/boot/limine.conf", "/boot/limine/limine.conf", "/efi/limine.conf",
    "/boot/EFI/limine/limine.conf", "/efi/EFI/limine/limine.conf"];
pub const REAL_THEMES: &str = "/usr/share/plymouth/themes";

pub fn save_state(st: &Map<String, Value>) {
    let p = plymouth::store_state_file();
    let _ = write_json_atomic(&p, st);
    let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o644));
}

/// Datei ändern, vorher einmalig eine Sicherung *.desddm-bak anlegen.
pub fn edit_file(path: &Path, new_text: &str) {
    let old = std::fs::read_to_string(path).unwrap_or_default();
    if old == new_text {
        return;
    }
    println!("Ändere {}", path.display());
    if dry() {
        return;
    }
    let bak = util::with_name(path, &format!("{}.desddm-bak", util::file_name(path)));
    if !bak.exists() {
        let _ = util::copy2(path, &bak);
    }
    let mode = path.metadata().map(|m| m.permissions().mode() & 0o7777).ok();
    let _ = std::fs::write(path, new_text);
    if let Some(m) = mode {
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(m));
    }
}

pub fn regen_initramfs() -> i32 {
    say("STEP", "ply.initramfs");
    if util::has("mkinitcpio") {
        return run(&sv(&["mkinitcpio", "-P"]), &[]);
    }
    if util::has("update-initramfs") {
        return run(&sv(&["update-initramfs", "-u", "-k", "all"]), &[]);
    }
    if util::has("dracut") {
        return run(&sv(&["dracut", "-f", "--regenerate-all"]), &[]);
    }
    if dry() {
        return run(&sv(&["mkinitcpio", "-P"]), &[]);
    }
    println!("Kein Initramfs-Werkzeug gefunden");
    0
}

// ---------- Einrichtung ----------

pub fn setup_hook() {
    if let Some(f) = plymouth::hooks_file() {
        let text = std::fs::read_to_string(&f).unwrap_or_default();
        edit_file(&f, &plymouth::add_hook(&text));
    }
}

pub fn setup_cmdline() {
    let mut done = false;
    let grub = Path::new("/etc/default/grub");
    // Fedora & Co.
    if util::has("grubby") && !grub.is_file() {
        run(&sv(&["grubby", "--update-kernel=ALL", "--args=quiet splash rhgb"]), &[]);
        return;
    }
    if grub.is_file() {
        let mut text = std::fs::read_to_string(grub).unwrap_or_default();
        let rx = fancy_rx();
        if let Some(new) = grub_sub(&rx, &text) {
            text = new;
        } else {
            text.push_str("\nGRUB_CMDLINE_LINUX_DEFAULT=\"quiet splash\"\n");
        }
        edit_file(grub, &text);
        if util::has("update-grub") {
            run(&sv(&["update-grub"]), &[]);
        } else if util::has("grub-mkconfig") && Path::new("/boot/grub").is_dir() {
            run(&sv(&["grub-mkconfig", "-o", "/boot/grub/grub.cfg"]), &[]);
        } else if util::has("grub2-mkconfig") && Path::new("/boot/grub2").is_dir() {
            run(&sv(&["grub2-mkconfig", "-o", "/boot/grub2/grub.cfg"]), &[]);
        }
        done = true;
    }
    let kc = Path::new("/etc/kernel/cmdline"); // Unified Kernel Images
    if kc.is_file() {
        let t = std::fs::read_to_string(kc).unwrap_or_default();
        edit_file(kc, &(plymouth::add_splash(t.trim()) + "\n"));
        done = true;
    }
    for d in BOOT_ENTRY_DIRS {
        // systemd-boot
        let d = Path::new(d);
        if d.is_dir() {
            let mut es: Vec<PathBuf> = std::fs::read_dir(d).map(|r| r.flatten().map(|e| e.path())
                .filter(|p| util::file_name(p).ends_with(".conf")).collect()).unwrap_or_default();
            es.sort();
            for e in es {
                let t = std::fs::read_to_string(&e).unwrap_or_default();
                let new: Vec<String> = util::splitlines(&t).into_iter().map(|l| {
                    if l.trim().starts_with("options") {
                        let rest = l.trim_start().split_once(char::is_whitespace).map(|x| x.1).unwrap_or("");
                        format!("options {}", plymouth::add_splash(rest))
                    } else {
                        l.to_string()
                    }
                }).collect();
                edit_file(&e, &(new.join("\n") + "\n"));
                done = true;
            }
        }
    }
    for lf in LIMINE_FILES {
        // Limine
        let lf = Path::new(lf);
        if lf.is_file() {
            let t = std::fs::read_to_string(lf).unwrap_or_default();
            let new: Vec<String> = util::splitlines(&t).into_iter().map(|l| {
                let key = l.trim().split(':').next().unwrap_or("").to_lowercase();
                if (key == "cmdline" || key == "kernel_cmdline") && l.contains(':') {
                    let (pre, val) = l.split_once(':').unwrap();
                    format!("{}: {}", pre, plymouth::add_splash(val.trim()))
                } else {
                    l.to_string()
                }
            }).collect();
            edit_file(lf, &(new.join("\n") + "\n"));
            done = true;
        }
    }
    if !done {
        println!("Bootloader nicht erkannt: Kernel-Parameter 'quiet splash' bitte selbst ergänzen");
    }
}

/// `^(GRUB_CMDLINE_LINUX_DEFAULT=)(["\']?)(.*?)\2\s*$` (mit Rückverweis, daher von Hand)
fn fancy_rx() -> Regex {
    Regex::new(r"(?m)^GRUB_CMDLINE_LINUX_DEFAULT=(.*)$").unwrap()
}

fn grub_sub(rx: &Regex, text: &str) -> Option<String> {
    for m in rx.captures_iter(text) {
        let all = m.get(0).unwrap();
        let raw = m[1].trim_end();
        let (q, inner) = if let Some(r) = raw.strip_prefix('"').and_then(|r| r.strip_suffix('"')) { ('"', r) }
            else if let Some(r) = raw.strip_prefix('\'').and_then(|r| r.strip_suffix('\'')) { ('\'', r) }
            else if raw.starts_with('"') || raw.starts_with('\'') {
                // öffnendes Anführungszeichen ohne schließendes: Python nimmt dann die leere Variante
                ('-', raw)
            } else { ('-', raw) };
        let _ = q;
        let mut out = String::with_capacity(text.len() + 16);
        out.push_str(&text[..all.start()]);
        out.push_str(&format!("GRUB_CMDLINE_LINUX_DEFAULT=\"{}\"", plymouth::add_splash(inner)));
        out.push_str(&text[all.end()..]);
        return Some(out);
    }
    None
}

pub fn do_setup() -> i32 {
    let Some(backend) = system::detect_backend() else {
        say("FAIL", "nobackend");
        return 11;
    };
    let pkgs: Vec<String> = match backend.as_str() {
        "pacman" => sv(&["plymouth"]),
        "apt" => sv(&["plymouth", "plymouth-themes"]),
        "dnf" => sv(&["plymouth", "plymouth-system-theme"]),
        _ => sv(&["plymouth"]),
    };
    say("STEP", "ply.install");
    say("PROGRESS", "5");
    let env: Vec<(&str, &str)> = if backend == "apt" { vec![("DEBIAN_FRONTEND", "noninteractive")] } else { vec![] };
    let before = dm_snapshot();
    for cmd in system::install_commands(&backend, &pkgs) {
        if run(&cmd, &env) != 0 {
            dm_restore(&before);
            say("FAIL", "install");
            return 1;
        }
    }
    dm_restore(&before);
    say("STEP", "ply.setup");
    say("PROGRESS", "50");
    setup_hook();
    setup_cmdline();
    say("PROGRESS", "70");
    if regen_initramfs() != 0 {
        say("FAIL", "ply.initramfs");
        return 1;
    }
    if dry() {
        let f = plymouth::dry_base().join("installed");
        let _ = std::fs::create_dir_all(f.parent().unwrap());
        let _ = std::fs::write(&f, "1");
        for n in ["spinner", "bgrt", "tribar"] {
            let d = plymouth::themes_dir().join(n);
            let _ = std::fs::create_dir_all(&d);
            let cap = n[..1].to_uppercase() + &n[1..];
            let _ = std::fs::write(d.join(format!("{}.plymouth", n)),
                format!("[Plymouth Theme]\nName={}\nModuleName=two-step\n", cap));
        }
        let _ = std::fs::write(plymouth::defaults_file(), "[Daemon]\nTheme=bgrt\n");
    }
    say("PROGRESS", "100");
    say("OK", "");
    0
}

// ---------- Themes ----------

fn plyfiles(d: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(d).map(|r| r.flatten().map(|e| e.path())
        .filter(|p| util::file_name(p).ends_with(".plymouth")).collect()).unwrap_or_default();
    v.sort();
    v
}

/// Theme-Ordner im Archiv. Der Name kommt aus der .plymouth-Datei, denn Plymouth lädt
/// immer themes/<Name>/<Name>.plymouth (Archive ohne eigenen Ordner, z. B. dotLock).
pub fn find_roots(base: &Path, _archive_name: &str) -> Vec<(String, PathBuf, String)> {
    let mut roots: Vec<PathBuf> = Vec::new();
    for tf in walk(base).into_iter().filter(|p| util::file_name(p).ends_with(".plymouth")) {
        let d = tf.parent().unwrap().to_path_buf();
        if !roots.contains(&d) && !roots.iter().any(|r| d.starts_with(r) && d != *r) {
            roots.push(d);
        }
    }
    roots.into_iter().map(|d| {
        let files = plyfiles(&d);
        let dn = util::file_name(&d);
        let tf = files.iter().find(|f| f.file_stem().map(|s| s.to_string_lossy() == dn).unwrap_or(false))
            .unwrap_or(&files[0]).clone();
        let stem = tf.file_stem().unwrap().to_string_lossy().into_owned();
        (sddm::safe_name(&stem), d, util::file_name(&tf))
    }).collect()
}

/// <Name>.plymouth sicherstellen und eigene Pfade (ImageDir/ScriptFile) auf den echten Ordner zeigen lassen.
pub fn fix_theme(target: &Path, name: &str, plyfile: &str) {
    let want = target.join(format!("{}.plymouth", name));
    let src = if !plyfile.is_empty() { Some(target.join(plyfile)) } else { plymouth::theme_file(target) };
    if let Some(src) = src {
        if src.is_file() && src != want {
            if want.exists() {
                let _ = std::fs::remove_file(&want);
            }
            let _ = std::fs::rename(&src, &want);
        }
    }
    if !want.is_file() {
        return;
    }
    let text = util::read_text(&want);
    let rx = Regex::new(&format!(r"(?m)^(\s*(?:ImageDir|ScriptFile)\s*=\s*){}/+([^/\s]+)(.*)$",
                                 regex::escape(REAL_THEMES))).unwrap();
    let new = rx.replace_all(&text, |m: &regex::Captures| {
        let r = &m[2];
        let rest = &m[3];
        let other = plymouth::themes_dir().join(r).is_dir() && !target.join(rest.trim_start_matches('/')).exists();
        if r == name || other {
            return m[0].to_string(); // zeigt schon richtig oder bewusst auf ein anderes Theme (z. B. bgrt → spinner)
        }
        format!("{}{}/{}{}", &m[1], REAL_THEMES, name, rest)
    }).into_owned();
    if new != text {
        let _ = std::fs::write(&want, &new);
        println!("Pfade in {} auf {}/{} korrigiert", util::file_name(&want), REAL_THEMES, name);
    }
}

/// Vorschau nur, wenn sie dem Aufrufer gehört und ein echtes PNG/JPEG ist.
pub fn safe_preview(preview: &str) -> Option<PathBuf> {
    if preview.is_empty() {
        return None;
    }
    let pv = PathBuf::from(preview);
    let st = std::fs::symlink_metadata(&pv).ok()?;
    (st.file_type().is_file() && st.len() < 20 * 1024 * 1024 && user_owned(&st) && is_image(&pv)).then_some(pv)
}

pub fn do_install(archive_arg: &str, store_id: &str, preview: &str) -> i32 {
    let archive = PathBuf::from(archive_arg);
    say("STEP", "sddm.unpack");
    say("PROGRESS", "10");
    if archive_valid(&archive) != Some(true) {
        say("FAIL", "sddm.archive");
        return 2;
    }
    let mut state = plymouth::load_store_state();
    let dest_dir = plymouth::themes_dir();
    let _ = std::fs::create_dir_all(&dest_dir);
    let mut installed: Vec<String> = Vec::new();
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
        let roots = find_roots(&work, &util::file_name(&archive));
        if roots.is_empty() {
            say("FAIL", "ply.notheme");
            return 4;
        }
        say("STEP", "sddm.copy");
        say("PROGRESS", "50");
        let pv = safe_preview(preview);
        for (name, src, plyfile) in roots {
            if !sddm::NAME_RE.is_match(&name) {
                continue;
            }
            let target = dest_dir.join(&name);
            if target.exists() {
                if !state.contains_key(&name) {
                    println!("{}: existiert schon (nicht von hier installiert) – wird genutzt", name);
                    installed.push(name);
                    continue;
                }
                let _ = std::fs::remove_dir_all(&target);
            }
            if copytree(&src, &target).is_err() {
                continue;
            }
            fix_theme(&target, &name, &plyfile);
            // Store-Vorschau als preview.png ablegen (wie plymouth-kcm)
            if let Some(p) = &pv {
                if !target.join("preview.png").exists() {
                    let _ = std::fs::copy(p, target.join("preview.png"));
                }
            }
            if !dry() {
                fix_perms(&target);
            }
            state.insert(name.clone(), json!({"store": store_id, "archive": util::file_name(&archive)}));
            println!("Installiert: {}", target.display());
            installed.push(name);
        }
    }
    if installed.is_empty() {
        say("FAIL", "sddm.exists");
        return 5;
    }
    save_state(&state);
    println!("@@THEME {}", installed[0]);
    say("PROGRESS", "100");
    say("OK", "");
    0
}

/// Ältere Installationen reparieren: Ordnername muss zur .plymouth-Datei passen (z. B. 1635081-dotLock → dotLock).
pub fn repair(name: &str) -> String {
    let d = plymouth::themes_dir().join(name);
    if d.join(format!("{}.plymouth", name)).is_file() {
        return name.into();
    }
    let Some(tf) = plymouth::theme_file(&d) else { return name.into() };
    let good = sddm::safe_name(&tf.file_stem().unwrap().to_string_lossy());
    let target = plymouth::themes_dir().join(&good);
    let mut state = plymouth::load_store_state();
    if target.is_dir() && target.join(format!("{}.plymouth", good)).is_file() {
        println!("{}: Ordnername passt nicht zur Datei {} – nehme vorhandenes Theme {}", name, util::file_name(&tf), good);
        return good;
    }
    if state.contains_key(name) && !dry() {
        let _ = std::fs::rename(&d, &target);
    } else if state.contains_key(name) {
        let _ = copytree(&d, &target);
        let _ = std::fs::remove_dir_all(&d);
    } else {
        println!("{}: Ordnername passt nicht zu {} (fremdes Theme, nicht geändert)", name, util::file_name(&tf));
        return name.into();
    }
    fix_theme(&target, &good, &util::file_name(&tf));
    if let Some(v) = state.shift_remove(name) {
        state.insert(good.clone(), v);
        save_state(&state);
    }
    println!("Repariert: {} → {}", name, good);
    good
}

pub fn do_set(name: &str) -> i32 {
    let d = plymouth::themes_dir().join(name);
    let tf = if sddm::NAME_RE.is_match(name) && d.is_dir() { plymouth::theme_file(&d) } else { None };
    if tf.is_none() {
        say("FAIL", "ply.notheme");
        return 2;
    }
    let name = repair(name);
    let tf = plymouth::theme_file(&plymouth::themes_dir().join(&name));
    println!("@@THEME {}", name);
    say("STEP", "ply.set");
    say("PROGRESS", "20");
    let conf = plymouth::conf_file();
    let _ = std::fs::create_dir_all(conf.parent().unwrap());
    if !conf.exists() {
        let _ = std::fs::write(&conf, "[Daemon]\n");
    }
    let _ = write_conf(&conf, "Daemon", "Theme", &name);
    let _ = std::fs::set_permissions(&conf, std::fs::Permissions::from_mode(0o644));
    println!("{}: [Daemon] Theme={}", conf.display(), name);
    if util::has("update-alternatives") && !dry() {
        // Debian/Ubuntu wie plymouth-kcm
        let alt = "/usr/share/plymouth/themes/default.plymouth";
        let tfs = tf.map(|p| p.display().to_string()).unwrap_or_else(|| "None".into());
        run(&sv(&["update-alternatives", "--install", alt, "default.plymouth", &tfs, "100"]), &[]);
        run(&sv(&["update-alternatives", "--set", "default.plymouth", &tfs]), &[]);
    }
    say("PROGRESS", "40");
    if regen_initramfs() != 0 {
        say("FAIL", "ply.initramfs");
        return 1;
    }
    say("PROGRESS", "100");
    say("OK", "");
    0
}

pub fn do_remove(name: &str) -> i32 {
    let mut state = plymouth::load_store_state();
    if !sddm::NAME_RE.is_match(name) || !state.contains_key(name) {
        say("FAIL", "sddm.foreign");
        return 2;
    }
    if plymouth::current_theme() == name {
        say("FAIL", "sddm.active");
        return 3;
    }
    say("STEP", "sddm.remove");
    let target = plymouth::themes_dir().join(name);
    if target.is_dir() && !target.is_symlink() {
        let _ = std::fs::remove_dir_all(&target);
        println!("Entfernt: {}", target.display());
    }
    state.shift_remove(name);
    save_state(&state);
    say("PROGRESS", "100");
    say("OK", "");
    0
}

pub fn main(argv: &[String]) -> Result<i32, ArgErr> {
    let action = arg(argv, 0)?;
    match action.as_str() {
        "ply-setup" => Ok(do_setup()),
        "ply-install" => {
            let a = arg(argv, 1)?;
            let mut store = String::new();
            let mut preview = String::new();
            for x in &argv[2..] {
                if let Some((k, v)) = x.strip_prefix("--").and_then(|r| r.split_once('=')) {
                    match k {
                        "store" => store = v.into(),
                        "preview" => preview = v.into(),
                        _ => {}
                    }
                }
            }
            Ok(do_install(&a, if super::digits(&store) { &store } else { "" }, &preview))
        }
        "ply-set" => Ok(do_set(&arg(argv, 1)?)),
        "ply-remove" => Ok(do_remove(&arg(argv, 1)?)),
        _ => Err(ArgErr(format!("'{}'", action))),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn grub() {
        let rx = super::fancy_rx();
        assert_eq!(super::grub_sub(&rx, "A=1\nGRUB_CMDLINE_LINUX_DEFAULT=\"loglevel=3 quiet\"\nB=2\n").unwrap(),
                   "A=1\nGRUB_CMDLINE_LINUX_DEFAULT=\"loglevel=3 quiet splash\"\nB=2\n");
        assert_eq!(super::grub_sub(&rx, "GRUB_CMDLINE_LINUX_DEFAULT='x'\n").unwrap(),
                   "GRUB_CMDLINE_LINUX_DEFAULT=\"x quiet splash\"\n");
    }
}
