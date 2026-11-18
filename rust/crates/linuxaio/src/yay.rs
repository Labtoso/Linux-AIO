//! yay aus dem AUR: yay-bin klonen und als Benutzer bauen (makepkg darf nicht als root laufen);
//! der Root-Helfer installiert nur das fertige Paket.

use crate::{system, usercmd};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const AUR: &str = "https://aur.archlinux.org/yay-bin.git";

pub fn work_dir() -> PathBuf {
    usercmd::real_home().join(".cache/LabtosoStudios/linuxaio/yay")
}

pub fn supported() -> bool {
    system::detect_backend().as_deref() == Some("pacman")
}

pub fn installed() -> bool {
    usercmd::have("yay")
}

pub fn deps_missing() -> Vec<String> {
    ["git", "makepkg", "fakeroot", "strip"].iter().filter(|t| !usercmd::have(t)).map(|s| s.to_string()).collect()
}

fn dry() -> bool {
    std::env::var("DEMANAGER_DRYRUN").map(|v| !v.is_empty()).unwrap_or(false)
}

fn run(cmd: &[&str], cwd: &Path, log: &dyn Fn(&str)) -> Result<(), String> {
    log(&format!("$ {}", cmd.join(" ")));
    if dry() {
        return Ok(());
    }
    let mut child = Command::new("sh").arg("-c").arg("exec \"$@\" 2>&1").arg("sh").args(cmd).current_dir(cwd)
        .stdin(Stdio::null()).stdout(Stdio::piped()).env("LC_ALL", "C").spawn().map_err(|e| e.to_string())?;
    if let Some(out) = child.stdout.take() {
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            log(line.trim_end());
        }
    }
    let code = child.wait().map(|s| s.code().unwrap_or(1)).unwrap_or(1);
    if code != 0 {
        return Err(format!("{} failed ({})", cmd[0], code));
    }
    Ok(())
}

fn pkgs(src: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(src).map(|r| r.flatten().map(|e| e.path())
        .filter(|p| { let n = hyprstudio::util::file_name(p); n.starts_with("yay-bin-") && n.contains(".pkg.tar.") })
        .collect()).unwrap_or_default();
    v.sort();
    v
}

/// yay-bin holen (oder aktualisieren) und bauen; gibt die Paketdatei zurück.
pub fn build(log: &dyn Fn(&str)) -> Result<PathBuf, String> {
    let base = work_dir();
    std::fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    let src = base.join("yay-bin");
    if src.join(".git").is_dir() {
        run(&["git", "pull", "--ff-only"], &src, log)?;
    } else {
        run(&["git", "clone", "--depth", "1", AUR, &src.display().to_string()], &base, log)?;
    }
    for old in pkgs(&src) {
        let _ = std::fs::remove_file(old);
    }
    run(&["makepkg", "-f", "--noconfirm"], &src, log)?;
    if dry() {
        return Ok(src.join("yay-bin-dry.pkg.tar.zst"));
    }
    pkgs(&src).into_iter().filter(|p| !hyprstudio::util::file_name(p).contains("-debug-")).last()
        .ok_or_else(|| "makepkg produced no package".to_string())
}
