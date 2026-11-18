//! Root-Teil für den GNOME-Anmeldebildschirm (GDM): Logo, Banner, Benutzerliste über dconf,
//! Hintergrund über eine neu gebaute gnome-shell-theme.gresource (Original wird einmal gesichert).
//!
//! Aufrufe:  gdm-logo <png|--clear>   gdm-banner <text|--clear>   gdm-userlist <on|off>
//!           gdm-bg <bild|#rrggbb>    gdm-restore

use super::sddm::{is_image, user_owned, TempDir};
use super::{arg, dry, say, ArgErr};
use crate::pyjson;
use hyprstudio::util;
use serde_json::{Map, Value};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const GRES: &str = "/usr/share/gnome-shell/gnome-shell-theme.gresource";
pub const STATE_DIR: &str = "/var/lib/labtoso-linuxaio";
pub const BACKUP: &str = "/var/lib/labtoso-linuxaio/gnome-shell-theme.gresource.original";
pub const DCONF_FILE: &str = "/etc/dconf/db/gdm.d/90-linuxaio";
pub const DCONF_PROFILE: &str = "/etc/dconf/profile/gdm";
pub const LOGO: &str = "/usr/share/pixmaps/linuxaio-gdm-logo.png";
pub const PREFIX: &str = "/org/gnome/shell/theme";
pub const BG_NAME: &str = "linuxaio-background";
pub const MARK: &str = "/* Linux AIO: Hintergrund */";

pub fn state_file() -> PathBuf {
    if dry() {
        return util::xdg_cache().join("LabtosoStudios").join("linuxaio").join("testmodus").join("gdm.json");
    }
    Path::new(STATE_DIR).join("gdm.json")
}

pub fn load() -> Map<String, Value> {
    std::fs::read_to_string(state_file()).ok().and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v.as_object().cloned()).unwrap_or_default()
}

fn save(st: &Map<String, Value>) {
    let p = state_file();
    let _ = std::fs::create_dir_all(p.parent().unwrap());
    let _ = std::fs::write(&p, pyjson::dumps(&Value::Object(st.clone()), Some(2), false, false));
    let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o644));
}

fn run(cmd: &[&str]) -> Option<std::process::Output> {
    println!("$ {}", cmd.join(" "));
    if dry() {
        return None;
    }
    Command::new(cmd[0]).args(&cmd[1..]).output().ok()
}

fn image_ok(path: &Path) -> bool {
    let Ok(st) = path.metadata() else { return false };
    path.is_file() && user_owned(&st) && is_image(path) && st.len() < 40 * 1024 * 1024
}

/// Text als GVariant-String für die dconf-Schlüsseldatei.
pub fn gv(s: &str) -> String {
    format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'").replace('\n', "\\n"))
}

fn truthy(v: Option<&Value>) -> bool {
    v.map(hyprstudio::jsonx::truthy).unwrap_or(false)
}

pub fn dconf_lines(st: &Map<String, Value>) -> Vec<String> {
    let mut lines = vec!["# Linux AIO (LabtosoStudios)".to_string(), "[org/gnome/login-screen]".to_string()];
    let s = |k: &str| st.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
    if truthy(st.get("logo")) {
        lines.push(format!("logo={}", gv(&s("logo"))));
    }
    if truthy(st.get("banner")) {
        lines.push("banner-message-enable=true".into());
        lines.push(format!("banner-message-text={}", gv(&s("banner"))));
    } else {
        lines.push("banner-message-enable=false".into());
    }
    if st.contains_key("userlist") {
        lines.push(format!("disable-user-list={}", if truthy(st.get("userlist")) { "false" } else { "true" }));
    }
    lines
}

pub fn write_dconf(st: &Map<String, Value>) {
    let lines = dconf_lines(st);
    let text = lines.join("\n") + "\n";
    println!("→ {}", DCONF_FILE);
    for ln in &lines[1..] {
        println!("  {}", ln);
    }
    if dry() {
        return;
    }
    let prof = Path::new(DCONF_PROFILE);
    if !prof.exists() {
        let _ = std::fs::create_dir_all(prof.parent().unwrap());
        let _ = std::fs::write(prof, "user-db:user\nsystem-db:gdm\nfile-db:/usr/share/gdm/greeter-dconf-defaults\n");
        println!("→ {}", DCONF_PROFILE);
    }
    let _ = std::fs::create_dir_all(Path::new(DCONF_FILE).parent().unwrap());
    let _ = std::fs::write(DCONF_FILE, text);
    run(&["dconf", "update"]);
}

pub fn do_logo(a: &str) -> i32 {
    let mut st = load();
    say("STEP", "gdm.logo");
    if a == "--clear" {
        st.shift_remove("logo");
    } else {
        let src = PathBuf::from(a);
        if !image_ok(&src) {
            say("FAIL", "gdm.image");
            return 2;
        }
        println!("{} → {}", src.display(), LOGO);
        if !dry() {
            let _ = std::fs::create_dir_all(Path::new(LOGO).parent().unwrap());
            let _ = std::fs::copy(&src, LOGO);
            let _ = std::fs::set_permissions(LOGO, std::fs::Permissions::from_mode(0o644));
        }
        st.insert("logo".into(), Value::String(LOGO.into()));
    }
    write_dconf(&st);
    save(&st);
    say("OK", "");
    0
}

pub fn do_banner(a: &str) -> i32 {
    let mut st = load();
    say("STEP", "gdm.banner");
    if a == "--clear" {
        st.shift_remove("banner");
    } else {
        st.insert("banner".into(), Value::String(a.chars().take(500).collect()));
    }
    write_dconf(&st);
    save(&st);
    say("OK", "");
    0
}

pub fn do_userlist(a: &str) -> i32 {
    let mut st = load();
    st.insert("userlist".into(), Value::Bool(a == "on"));
    write_dconf(&st);
    save(&st);
    say("OK", "");
    0
}

pub fn css_rule(value: &str, has_image: bool) -> String {
    let bg = if has_image {
        format!("background: #000 url(\"resource://{}/{}\"); background-size: cover; background-position: center; \
                 background-repeat: no-repeat;", PREFIX, BG_NAME)
    } else {
        format!("background: {}; background-image: none;", value)
    };
    format!("\n{}\n#lockDialogGroup {{ {} }}\n", MARK, bg)
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('>', "&gt;").replace('<', "&lt;")
}

fn is_color(a: &str) -> bool {
    a.len() == 7 && a.starts_with('#') && a[1..].chars().all(|c| c.is_ascii_hexdigit())
}

pub fn do_bg(a: &str) -> i32 {
    let src = if is_color(a) { None } else { Some(PathBuf::from(a)) };
    if let Some(s) = &src {
        if !image_ok(s) {
            say("FAIL", "gdm.image");
            return 2;
        }
    }
    if !Path::new(GRES).is_file() && !dry() {
        say("FAIL", "gdm.missing");
        return 2;
    }
    say("STEP", "gdm.bg");
    say("PROGRESS", "10");
    if !dry() {
        let _ = std::fs::create_dir_all(STATE_DIR);
        if !Path::new(BACKUP).exists() {
            let _ = util::copy2(Path::new(GRES), Path::new(BACKUP)); // Original genau einmal sichern
            println!("Sicherung: {}", BACKUP);
        }
    }
    println!("Quelle: {} (Original)", BACKUP);
    if dry() {
        for c in [vec!["gresource", "list", BACKUP], vec!["gresource", "extract", BACKUP, "…"],
                  vec!["glib-compile-resources", "…"], vec!["install", "-m644", "…", GRES]] {
            println!("$ {}", c.join(" "));
        }
        let mut st = load();
        st.insert("background".into(), Value::String(a.into()));
        save(&st);
        say("PROGRESS", "100");
        say("OK", "");
        return 0;
    }
    let Ok(tmp) = TempDir::new("linuxaio-gdm-") else {
        say("FAIL", "gdm.gresource");
        return 1;
    };
    let work = tmp.0.clone();
    let res = Command::new("gresource").args(["list", BACKUP]).output();
    let Ok(res) = res else {
        say("FAIL", "gdm.gresource");
        return 1;
    };
    if !res.status.success() {
        println!("{}", String::from_utf8_lossy(&res.stderr));
        say("FAIL", "gdm.gresource");
        return 1;
    }
    let listing = String::from_utf8_lossy(&res.stdout).into_owned();
    let names: Vec<String> = listing.lines().map(|n| n.trim().to_string())
        .filter(|n| n.starts_with(&format!("{}/", PREFIX))).collect();
    let mut files = Vec::new();
    for n in names {
        let rel = n[PREFIX.len() + 1..].to_string();
        let out = work.join(&rel);
        let _ = std::fs::create_dir_all(out.parent().unwrap());
        let mut data = Command::new("gresource").args(["extract", BACKUP, &n]).output().map(|o| o.stdout)
            .unwrap_or_default();
        if rel.ends_with(".css") && (rel.starts_with("gnome-shell") || rel.starts_with("gdm")) {
            let text = String::from_utf8_lossy(&data).into_owned();
            let head = text.split(&format!("\n{}", MARK)).next().unwrap_or("").to_string();
            data = (head + &css_rule(a, src.is_some())).into_bytes();
        }
        let _ = std::fs::write(&out, data);
        files.push(rel);
    }
    say("PROGRESS", "50");
    if let Some(s) = &src {
        let _ = std::fs::copy(s, work.join(BG_NAME));
        files.push(BG_NAME.into());
    }
    let mut xml = vec!["<?xml version=\"1.0\" encoding=\"UTF-8\"?>".to_string(), "<gresources>".into(),
                       format!("  <gresource prefix=\"{}\">", PREFIX)];
    xml.extend(files.iter().map(|f| format!("    <file>{}</file>", xml_escape(f))));
    xml.push("  </gresource>".into());
    xml.push("</gresources>".into());
    let _ = std::fs::write(work.join("theme.gresource.xml"), xml.join("\n") + "\n");
    let target = work.join("gnome-shell-theme.gresource");
    let r = Command::new("glib-compile-resources").arg("--sourcedir").arg(&work).arg("--target").arg(&target)
        .arg(work.join("theme.gresource.xml")).output();
    let ok = r.as_ref().map(|o| o.status.success()).unwrap_or(false);
    if !ok || !target.is_file() {
        if let Ok(o) = &r {
            println!("{}", String::from_utf8_lossy(&o.stderr));
        }
        say("FAIL", "gdm.gresource");
        return 1;
    }
    say("PROGRESS", "85");
    let _ = std::fs::copy(&target, GRES);
    let _ = std::fs::set_permissions(GRES, std::fs::Permissions::from_mode(0o644));
    println!("→ {}", GRES);
    drop(tmp);
    let mut st = load();
    st.insert("background".into(), Value::String(a.into()));
    save(&st);
    say("PROGRESS", "100");
    say("OK", "");
    0
}

pub fn do_restore() -> i32 {
    say("STEP", "gdm.restore");
    if !dry() {
        if !Path::new(BACKUP).exists() {
            say("FAIL", "gdm.nobackup");
            return 2;
        }
        let _ = std::fs::copy(BACKUP, GRES);
        let _ = std::fs::set_permissions(GRES, std::fs::Permissions::from_mode(0o644));
    }
    println!("{} → {}", BACKUP, GRES);
    let mut st = load();
    st.shift_remove("background");
    save(&st);
    say("OK", "");
    0
}

pub fn main(argv: &[String]) -> Result<i32, ArgErr> {
    let action = arg(argv, 0)?;
    match action.as_str() {
        "gdm-logo" => Ok(do_logo(&arg(argv, 1)?)),
        "gdm-banner" => Ok(do_banner(&arg(argv, 1)?)),
        "gdm-userlist" => Ok(do_userlist(&arg(argv, 1)?)),
        "gdm-bg" => Ok(do_bg(&arg(argv, 1)?)),
        "gdm-restore" => Ok(do_restore()),
        _ => Err(ArgErr(format!("'{}'", action))),
    }
}
