//! Plymouth-Bootsplash: Themes lesen, aktives Theme, Einrichtungsstatus, KDE Store (wie plymouth-kcm).
//!
//! plymouth-kcm: KNewStuff-Kategorie "Plymouth Themes" (OCS-ID 108), Themes nach /usr/share/plymouth/themes,
//! Setzen über /etc/plymouth/plymouthd.conf [Daemon] Theme=, auf Debian/Ubuntu zusätzlich update-alternatives,
//! danach Initramfs neu bauen (update-initramfs -u bzw. mkinitcpio -P).

use crate::i18n::t;
use crate::sddm::{self, ThemeInfo};
use crate::system;
use hyprstudio::util;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub const CATEGORY: &str = "108";
pub use sddm::{archive_ok, item_url, parse_item, parse_items, NAME_RE, STORE_PAGE};

fn dry() -> bool {
    sddm::dry()
}

pub fn dry_base() -> PathBuf {
    sddm::dry_base().join("plymouth")
}

pub fn themes_dir() -> PathBuf {
    if dry() { dry_base().join("themes") } else { PathBuf::from("/usr/share/plymouth/themes") }
}

pub fn conf_file() -> PathBuf {
    if dry() { dry_base().join("plymouthd.conf") } else { PathBuf::from("/etc/plymouth/plymouthd.conf") }
}

pub fn defaults_file() -> PathBuf {
    if dry() { dry_base().join("plymouthd.defaults") } else { PathBuf::from("/usr/share/plymouth/plymouthd.defaults") }
}

pub fn store_state_file() -> PathBuf {
    if dry() { dry_base().join("state.json") } else { PathBuf::from("/var/lib/labtoso-linuxaio/plymouth.json") }
}

pub fn load_store_state() -> Map<String, Value> {
    system::load_state_file(&store_state_file())
}

pub fn is_installed() -> bool {
    if dry() {
        return dry_base().join("installed").exists();
    }
    util::has("plymouth") || util::has("plymouthd")
}

pub fn current_theme() -> String {
    for f in [conf_file(), defaults_file()] {
        let v = sddm::read_ini(&f).get_or("Daemon", "Theme", "").trim().to_string();
        if !v.is_empty() {
            return v;
        }
    }
    String::new()
}

fn glob_ext(d: &Path, ext: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(d).map(|r| r.flatten().map(|e| e.path())
        .filter(|p| util::file_name(p).ends_with(ext) && !util::file_name(p).starts_with('.')).collect())
        .unwrap_or_default();
    v.sort();
    v
}

pub fn theme_file(path: &Path) -> Option<PathBuf> {
    let own = path.join(format!("{}.plymouth", util::file_name(path)));
    if own.is_file() {
        return Some(own);
    }
    glob_ext(path, ".plymouth").into_iter().next()
}

pub fn theme_info(path: &Path) -> Option<ThemeInfo> {
    let tf = theme_file(path)?;
    let cp = sddm::read_ini(&tf);
    let g = |k: &str| cp.get_or("Plymouth Theme", k, "").trim().to_string();
    let name0 = util::file_name(path);
    let parent = path.parent().unwrap_or(Path::new("/"));
    let shot = [path.join("preview.png"), parent.join(format!("{}.png", name0)), path.join("preview.jpg")]
        .into_iter().find(|p| p.is_file());
    let n = g("Name");
    Some(ThemeInfo {
        id: name0.clone(),
        name: if n.is_empty() { name0.clone() } else { n },
        description: g("Description"),
        module: g("ModuleName"),
        broken: !path.join(format!("{}.plymouth", name0)).is_file(),
        screenshot: shot.map(|p| p.display().to_string()).unwrap_or_default(),
        path: path.display().to_string(),
        ..Default::default()
    })
}

pub fn list_themes() -> Vec<ThemeInfo> {
    sddm::sorted_dirs(&themes_dir()).iter().filter_map(|p| theme_info(p)).collect()
}

static FRAME_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)^(?P<base>.*?)[-_]?(?P<num>\d+)\.png$").unwrap());
pub const SKIP: [&str; 19] = ["box", "bullet", "entry", "lock", "capslock", "keyboard", "keymap", "password", "prompt",
    "dialog", "progress_bar", "progress-bar", "bar", "background", "bg", "wallpaper", "corner", "suspend", "header"];

fn color(v: &str) -> String {
    let v = v.trim().to_lowercase().replace("0x", "").replace('#', "");
    let ok = !v.is_empty() && v.len() <= 8 && v.chars().all(|c| c.is_ascii_hexdigit());
    if !ok {
        return String::new();
    }
    let tail = if v.len() > 6 { &v[v.len() - 6..] } else { &v[..] };
    format!("#{:0>6}", tail)
}

/// Bilder für eine animierte Vorschau.
#[derive(Debug, Clone, Default)]
pub struct Assets {
    pub frames: Vec<String>,
    pub logo: String,
    pub background: String,
    pub bg: String,
    pub bg2: String,
    pub ha: f64,
    pub va: f64,
    pub wva: f64,
}

/// Python-`float()` für Texte aus Theme-Dateien.
fn pyfloat(s: &str) -> Option<f64> {
    let s = s.trim().replace('_', "");
    let l = s.to_lowercase();
    if l.contains("inf") || l.contains("nan") {
        return None;
    }
    s.parse::<f64>().ok()
}

/// Bilder für eine animierte Vorschau: Einzelbilder, Logo, Hintergrund, Farben, Positionen.
pub fn preview_assets(path: &Path) -> Assets {
    let tf = theme_file(path);
    let cp = tf.as_ref().map(|f| sddm::read_ini(f));
    let module = cp.as_ref().map(|c| c.get_or("Plymouth Theme", "ModuleName", "")).unwrap_or_default()
        .trim().to_lowercase();
    let sec = cp.as_ref().filter(|c| c.has_section(&module)).map(|_| module.clone());
    let g = |k: &str, d: &str| -> String {
        match (&sec, &cp) {
            (Some(s), Some(c)) => c.get_or(s, k, d).trim().to_string(),
            _ => d.to_string(),
        }
    };
    let raw_dir = g("ImageDir", "");
    let mut img_dir = if raw_dir.is_empty() {
        path.to_path_buf()
    } else if raw_dir.starts_with('/') {
        // absolute Plymouth-Pfade (/usr/share/plymouth/themes/…); im Testmodus auf den Testordner umbiegen
        if dry() { themes_dir().join(util::file_name(Path::new(&raw_dir))) } else { PathBuf::from(&raw_dir) }
    } else {
        path.join(&raw_dir)
    };
    if !img_dir.is_dir() {
        img_dir = path.to_path_buf();
    }
    let pngs: Vec<PathBuf> = glob_ext(&img_dir, ".png").into_iter().filter(|p| p.is_file()).collect();
    let mut groups: Vec<(String, Vec<(u64, PathBuf)>)> = Vec::new();
    for p in &pngs {
        if let Some(m) = FRAME_RE.captures(&util::file_name(p)) {
            let base = m["base"].to_lowercase();
            let num: u64 = m["num"].parse().unwrap_or(0);
            match groups.iter_mut().find(|(b, _)| *b == base) {
                Some((_, l)) => l.push((num, p.clone())),
                None => groups.push((base, vec![(num, p.clone())])),
            }
        }
    }
    let mut frames = Vec::new();
    if !groups.is_empty() {
        // die lange Hauptanimation zuerst (two-step: animation-* = das eigentliche Bild, throbber-* nur Ladepunkte)
        let rank = |base: &str, n: usize| -> (i64, usize) {
            let pref = ["animation", "throbber", "spinner", "loading", "frame", "progress"].iter()
                .position(|k| base.contains(k)).unwrap_or(9);
            (-(n as i64), pref)
        };
        let mut best: Option<&(String, Vec<(u64, PathBuf)>)> = None;
        for gr in groups.iter().filter(|(_, l)| l.len() >= 3) {
            if best.map(|b| rank(&gr.0, gr.1.len()) < rank(&b.0, b.1.len())).unwrap_or(true) {
                best = Some(gr);
            }
        }
        if let Some((_, lst)) = best {
            let mut l = lst.clone();
            l.sort();
            frames = l.into_iter().take(600).map(|(_, p)| p.display().to_string()).collect();
        }
    }
    let in_groups: BTreeSet<PathBuf> = groups.iter().flat_map(|(_, l)| l.iter().map(|(_, p)| p.clone())).collect();
    let stem = |p: &Path| p.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
    // wie das Python-Dict {stem: pfad}: der letzte gewinnt
    let named = |n: &str| pngs.iter().rev().find(|p| stem(p) == n).cloned();
    let mut logo = ["watermark", "logo", "header-image", "splash", "distro-logo"].iter().find_map(|n| named(n));
    if logo.is_none() {
        let rest: Vec<&PathBuf> = pngs.iter()
            .filter(|p| !in_groups.contains(*p) && !SKIP.iter().any(|s| stem(p).contains(s))).collect();
        let mut best: Option<(u64, &PathBuf)> = None;
        for p in rest {
            let sz = p.metadata().map(|m| m.len()).unwrap_or(0);
            if best.map(|(b, _)| sz > b).unwrap_or(true) {
                best = Some((sz, p));
            }
        }
        logo = best.map(|(_, p)| p.clone());
    }
    let bgimg = ["background", "bg", "wallpaper", "background-tile"].iter().find_map(|n| named(n));
    let bg = [color(&g("BackgroundStartColor", "")), color(&g("BackgroundColor", ""))].into_iter()
        .find(|c| !c.is_empty()).unwrap_or_else(|| "#000000".into());
    let bg2 = color(&g("BackgroundEndColor", ""));
    let two = module == "two-step";
    let (mut ha, mut va, mut wva) = match (pyfloat(&g("HorizontalAlignment", ".5")),
        pyfloat(&g("VerticalAlignment", if two { ".75" } else { ".5" })),
        pyfloat(&g("WatermarkVerticalAlignment", if two { ".96" } else { ".4" }))) {
        (Some(a), Some(b), Some(c)) => (a, b, c),
        _ => (0.5, 0.6, 0.4),
    };
    let _ = &mut ha;
    if !two && logo.is_some() && !frames.is_empty() {
        va = 0.66;
        wva = 0.42;
    }
    Assets {
        frames,
        logo: logo.map(|p| p.display().to_string()).unwrap_or_default(),
        background: bgimg.map(|p| p.display().to_string()).unwrap_or_default(),
        bg2: if !bg2.is_empty() && bg2 != bg { bg2 } else { String::new() },
        bg,
        ha,
        va,
        wva,
    }
}

pub fn theme_chips(th: &ThemeInfo) -> Vec<(String, String, String)> {
    let mut chips = Vec::new();
    if !th.module.is_empty() {
        chips.push((th.module.clone(), "Chip".to_string(), String::new()));
    }
    if th.broken {
        chips.push((t("ply.broken"), "ChipWarn".into(), t("ply.broken.tip")));
    }
    chips
}

pub fn test_command(_th: &ThemeInfo) -> Option<Vec<String>> {
    None
}

pub fn list_url(page: u32, sort: &str, search: &str) -> String {
    sddm::list_url_cat(CATEGORY, page, sort, search)
}

// ---------- Einrichtung (Hook + Kernel-Parameter) ----------

pub fn mkinitcpio_files() -> Vec<PathBuf> {
    let mut files = vec![PathBuf::from("/etc/mkinitcpio.conf")];
    let d = Path::new("/etc/mkinitcpio.conf.d");
    if d.is_dir() {
        files.extend(glob_ext(d, ".conf"));
    }
    files.into_iter().filter(|f| f.is_file()).collect()
}

pub static HOOKS_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^(\s*HOOKS\s*=\s*\()([^)]*)(\).*)$").unwrap());

/// Datei, deren HOOKS=-Zeile zuletzt gilt.
pub fn hooks_file() -> Option<PathBuf> {
    let mut last = None;
    for f in mkinitcpio_files() {
        if let Ok(s) = std::fs::read_to_string(&f) {
            if HOOKS_RE.is_match(&s) {
                last = Some(f);
            }
        }
    }
    last
}

/// Fügt 'plymouth' nach systemd/udev/base in HOOKS=(...) ein (vor encrypt).
pub fn add_hook(text: &str) -> String {
    HOOKS_RE.replace_all(text, |m: &regex::Captures| {
        let mut hooks: Vec<&str> = m[2].split_whitespace().collect();
        if hooks.contains(&"plymouth") || hooks.contains(&"sd-plymouth") {
            return m[0].to_string();
        }
        let mut pos = 0;
        for anchor in ["systemd", "udev", "base"] {
            if let Some(i) = hooks.iter().position(|h| *h == anchor) {
                pos = i + 1;
                break;
            }
        }
        for blocker in ["encrypt", "sd-encrypt"] {
            if let Some(i) = hooks.iter().position(|h| *h == blocker) {
                pos = pos.min(i);
            }
        }
        hooks.insert(pos, "plymouth");
        format!("{}{}{}", &m[1], hooks.join(" "), &m[3])
    }).into_owned()
}

/// None = kein mkinitcpio (dracut/initramfs-tools bringen Plymouth selbst mit)
pub fn has_hook() -> Option<bool> {
    if dry() {
        return Some(is_installed());
    }
    let f = hooks_file()?;
    let s = std::fs::read_to_string(f).unwrap_or_default();
    Some(HOOKS_RE.captures(&s).map(|m| m[2].split_whitespace().any(|h| h == "plymouth" || h == "sd-plymouth"))
        .unwrap_or(false))
}

pub fn add_params(cmdline: &str, params: &[&str]) -> String {
    let mut parts: Vec<String> = cmdline.split_whitespace().map(String::from).collect();
    for p in params {
        if !parts.iter().any(|x| x == p) {
            parts.push(p.to_string());
        }
    }
    parts.join(" ")
}

pub fn add_splash(cmdline: &str) -> String {
    add_params(cmdline, &["quiet", "splash"])
}

pub fn splash_active() -> bool {
    if dry() {
        return is_installed();
    }
    std::fs::read_to_string("/proc/cmdline").map(|s| s.split_whitespace().any(|x| x == "splash")).unwrap_or(false)
}

pub fn is_ready() -> bool {
    is_installed() && splash_active() && has_hook() != Some(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hooks() {
        assert_eq!(add_hook("HOOKS=(base udev autodetect encrypt filesystems)\n"),
                   "HOOKS=(base udev plymouth autodetect encrypt filesystems)\n");
        assert_eq!(add_hook("HOOKS=(base systemd sd-encrypt)"), "HOOKS=(base systemd plymouth sd-encrypt)");
        assert_eq!(add_hook("HOOKS=(encrypt base udev)"), "HOOKS=(plymouth encrypt base udev)");
        assert_eq!(add_splash("rw quiet"), "rw quiet splash");
        assert_eq!(color("0xff112233"), "#112233");
        assert_eq!(color("abc"), "#000abc");
        assert_eq!(color("zz"), "");
    }
}
