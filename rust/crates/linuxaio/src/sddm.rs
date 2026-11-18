//! SDDM-Themes: lokal lesen, aktuelles Theme bestimmen, KDE-Store (OCS, wie die SDDM-KCM über KNewStuff).
//!
//! Die KCM nutzt sddmtheme.knsrc: Kategorie "SDDM Theme" (OCS-ID 101) auf store.kde.org,
//! lädt das Archiv herunter und entpackt es als root nach /usr/share/sddm/themes.

use crate::i18n::t;
use crate::ini::Ini;
use crate::system;
use hyprstudio::util;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

pub const OCS: &str = "https://api.kde-look.org/ocs/v1";
pub const CATEGORY: &str = "101";
pub const STORE_PAGE: &str = "https://store.kde.org/p/{id}";
/// neu, beliebt (Downloads), Bewertung
pub const SORTS: [&str; 3] = ["new", "down", "high"];
pub const PAGE_SIZE: u32 = 24;

pub static NAME_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[A-Za-z0-9][A-Za-z0-9._+-]{0,80}$").unwrap());

pub fn dry() -> bool {
    std::env::var("DEMANAGER_DRYRUN").as_deref() == Ok("1")
}

pub fn dry_base() -> PathBuf {
    std::env::temp_dir().join("desddm-dry")
}

pub fn themes_dir() -> PathBuf {
    if dry() { dry_base().join("themes") } else { PathBuf::from("/usr/share/sddm/themes") }
}

fn sorted_conf(d: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(d).map(|r| r.flatten().map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "conf").unwrap_or(false)).collect()).unwrap_or_default();
    v.sort();
    v
}

pub fn conf_files() -> Vec<PathBuf> {
    if dry() {
        let base = dry_base();
        let mut v = sorted_conf(&base.join("sddm.conf.d"));
        v.push(base.join("sddm.conf"));
        return v;
    }
    let mut out = Vec::new();
    for d in ["/usr/lib/sddm/sddm.conf.d", "/etc/sddm.conf.d"] {
        if Path::new(d).is_dir() {
            out.extend(sorted_conf(Path::new(d)));
        }
    }
    out.push(PathBuf::from("/etc/sddm.conf"));
    out
}

pub fn kde_settings_file() -> PathBuf {
    if dry() { dry_base().join("sddm.conf.d").join("kde_settings.conf") }
    else { PathBuf::from("/etc/sddm.conf.d/kde_settings.conf") }
}

pub fn main_conf_file() -> PathBuf {
    conf_files().pop().unwrap()
}

pub fn store_state_file() -> PathBuf {
    if dry() { dry_base().join("sddm-state.json") } else { PathBuf::from("/var/lib/labtoso-linuxaio/sddm.json") }
}

pub fn load_store_state() -> Map<String, Value> {
    system::load_state_file(&store_state_file())
}

pub fn read_ini(path: &Path) -> Ini {
    Ini::read(path)
}

pub fn current_theme() -> String {
    let mut cur = String::new();
    for f in conf_files() {
        if let Some(v) = read_ini(&f).get("Theme", "Current") {
            cur = v.trim().to_string();
        }
    }
    cur
}

pub fn is_installed_sddm() -> bool {
    dry() || Path::new("/usr/share/sddm").is_dir() || util::has("sddm")
}

/// Ein lokal installiertes Theme (SDDM oder Plymouth).
#[derive(Debug, Clone, Default)]
pub struct ThemeInfo {
    pub id: String,
    pub name: String,
    pub author: String,
    pub description: String,
    pub version: String,
    pub screenshot: String,
    pub qt6: bool,
    pub path: String,
    /// nur Plymouth
    pub module: String,
    pub broken: bool,
}

pub fn theme_info(path: &Path) -> Option<ThemeInfo> {
    let meta = path.join("metadata.desktop");
    if !meta.is_file() {
        return None;
    }
    let cp = read_ini(&meta);
    let sec = if cp.has_section("SddmGreeterTheme") { Some("SddmGreeterTheme".to_string()) }
              else { cp.section_names().into_iter().next() };
    let g = |k: &str, d: &str| -> String {
        match &sec {
            Some(s) => cp.get_or(s, k, d).trim().to_string(),
            None => d.to_string(),
        }
    };
    let name0 = util::file_name(path);
    let shot = g("Screenshot", "");
    let mut shot_path = (!shot.is_empty()).then(|| path.join(&shot));
    if !shot_path.as_ref().map(|p| p.is_file()).unwrap_or(false) {
        shot_path = ["preview.png", "screenshot.png", "preview.jpg", "screenshot.jpg"].iter()
            .map(|n| path.join(n)).find(|p| p.is_file());
    }
    let qt = g("QtVersion", "");
    let n = g("Name", &name0);
    Some(ThemeInfo {
        id: name0.clone(),
        name: if n.is_empty() { name0 } else { n },
        author: g("Author", ""),
        description: g("Description", ""),
        version: g("Version", ""),
        screenshot: shot_path.map(|p| p.display().to_string()).unwrap_or_default(),
        qt6: qt.starts_with('6'),
        path: path.display().to_string(),
        ..Default::default()
    })
}

/// Unterordner sortiert nach Namen (ohne Groß/klein).
pub fn sorted_dirs(d: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(d).map(|r| r.flatten().map(|e| e.path()).collect()).unwrap_or_default();
    v.sort_by_key(|p| util::file_name(p).to_lowercase());
    v.into_iter().filter(|p| p.is_dir()).collect()
}

pub fn list_themes() -> Vec<ThemeInfo> {
    sorted_dirs(&themes_dir()).iter().filter_map(|p| theme_info(p)).collect()
}

pub fn greeter_command(theme: &ThemeInfo) -> Option<Vec<String>> {
    let exe = if theme.qt6 { util::which("sddm-greeter-qt6").or_else(|| util::which("sddm-greeter")) }
              else { util::which("sddm-greeter").or_else(|| util::which("sddm-greeter-qt6")) };
    exe.map(|e| vec![e.display().to_string(), "--test-mode".into(), "--theme".into(), theme.path.clone()])
}

// ---------- KDE Store (OCS) ----------

/// `urllib.parse.quote_plus`
pub fn quote_plus(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'.' | b'-' | b'~' => out.push(b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

pub fn urlencode(q: &[(&str, String)]) -> String {
    q.iter().map(|(k, v)| format!("{}={}", quote_plus(k), quote_plus(v))).collect::<Vec<_>>().join("&")
}

pub fn list_url_cat(category: &str, page: u32, sort: &str, search: &str) -> String {
    let mut q = vec![("categories", category.to_string()), ("format", "json".into()),
                     ("pagesize", PAGE_SIZE.to_string()), ("page", page.to_string()),
                     ("sortmode", if SORTS.contains(&sort) { sort.into() } else { "down".into() })];
    if !search.trim().is_empty() {
        q.push(("search", search.trim().to_string()));
    }
    format!("{}/content/data?{}", OCS, urlencode(&q))
}

pub fn list_url(page: u32, sort: &str, search: &str) -> String {
    list_url_cat(CATEGORY, page, sort, search)
}

pub fn item_url(item_id: &str) -> String {
    format!("{}/content/data/{}?format=json", OCS, item_id)
}

#[derive(Debug, Clone, Default)]
pub struct StoreFile {
    pub index: u32,
    pub name: String,
    pub size_kb: i64,
    pub link: String,
}

#[derive(Debug, Clone, Default)]
pub struct StoreItem {
    pub store_id: String,
    pub name: String,
    pub author: String,
    pub downloads: i64,
    pub score: i64,
    pub preview: String,
    pub small: String,
    pub summary: String,
    pub qt6: bool,
    pub changed: String,
    pub files: Vec<StoreFile>,
}

fn sget(c: &Map<String, Value>, k: &str) -> String {
    match c.get(k) {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(false)) => String::new(),
        Some(v) => hyprstudio::jsonx::py_str(v),
    }
}

fn iget(c: &Map<String, Value>, k: &str) -> Option<i64> {
    match c.get(k) {
        None | Some(Value::Null) => Some(0),
        Some(Value::Number(n)) => n.as_i64().or_else(|| n.as_f64().map(|f| f as i64)),
        Some(Value::String(s)) if s.is_empty() => Some(0),
        Some(Value::String(s)) => s.trim().parse::<i64>().ok(),
        Some(Value::Bool(b)) => Some(*b as i64),
        _ => None,
    }
}

fn files_of(c: &Map<String, Value>) -> Vec<StoreFile> {
    let mut files = Vec::new();
    let mut i = 1;
    while c.contains_key(&format!("downloadlink{}", i)) || c.contains_key(&format!("downloadname{}", i)) {
        let name = sget(c, &format!("downloadname{}", i));
        let link = sget(c, &format!("downloadlink{}", i));
        if !name.is_empty() && !link.is_empty() && archive_ok(&name) {
            let size = iget(c, &format!("downloadsize{}", i)).unwrap_or(0);
            files.push(StoreFile { index: i, name, size_kb: size, link });
        }
        i += 1;
    }
    files
}

pub fn archive_ok(name: &str) -> bool {
    let n = name.to_lowercase();
    [".tar.gz", ".tgz", ".tar.xz", ".txz", ".tar.bz2", ".tbz2", ".tar.zst", ".tar", ".zip"].iter().any(|e| n.ends_with(e))
}

static TAG_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"<[^>]+>").unwrap());

fn first_chars(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

pub fn parse_items(data: &[u8]) -> (Vec<StoreItem>, i64) {
    let text = String::from_utf8_lossy(data);
    let Ok(d) = serde_json::from_str::<Value>(&text) else { return (vec![], 0) };
    let mut items = Vec::new();
    let list = d.get("data").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    for c in list {
        let Some(c) = c.as_object() else { continue };
        let id = sget(c, "id");
        if id.is_empty() || id == "0" && matches!(c.get("id"), Some(Value::Number(_))) {
            continue;
        }
        let tags = sget(c, "tags").to_lowercase();
        let or = |a: &str, b: &str| { let x = sget(c, a); if x.is_empty() { sget(c, b) } else { x } };
        items.push(StoreItem {
            store_id: id,
            name: sget(c, "name").trim().to_string(),
            author: sget(c, "personid"),
            downloads: iget(c, "downloads").unwrap_or(0),
            score: iget(c, "score").unwrap_or(0),
            preview: or("previewpic1", "smallpreviewpic1"),
            small: or("smallpreviewpic1", "previewpic1"),
            summary: first_chars(&TAG_RE.replace_all(&or("summary", "description"), ""), 400),
            qt6: tags.contains("qt6") || tags.contains("plasma6") || tags.contains("plasma 6"),
            changed: first_chars(&sget(c, "changed"), 10),
            files: files_of(c),
        });
    }
    let total = d.as_object().and_then(|o| iget(o, "totalitems")).unwrap_or(0);
    (items, total)
}

pub fn parse_item(data: &[u8]) -> Option<StoreItem> {
    parse_items(data).0.into_iter().next()
}

pub fn safe_name(name: &str) -> String {
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"[^A-Za-z0-9._+-]+").unwrap());
    let n = RE.replace_all(name, "-").to_string();
    let n = n.trim_matches(|c| c == '-' || c == '.');
    let n = first_chars(n, 80);
    if n.is_empty() { "theme".into() } else { n }
}

// ---------- gemeinsame Schnittstelle für die Theme-Seite ----------

pub fn is_installed() -> bool {
    system::sddm_installed()
}

/// (Text, CSS-Klasse, Tooltip)
pub fn theme_chips(th: &ThemeInfo) -> Vec<(String, String, String)> {
    if th.qt6 {
        vec![("Qt6".into(), "Chip".into(), String::new())]
    } else {
        vec![("Qt5".into(), "ChipWarn".into(), t("sddm.qt5"))]
    }
}

pub fn test_command(th: &ThemeInfo) -> Option<Vec<String>> {
    greeter_command(th)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls() {
        assert_eq!(list_url(2, "high", " a b&c "),
            "https://api.kde-look.org/ocs/v1/content/data?categories=101&format=json&pagesize=24&page=2&sortmode=high&search=a+b%26c");
        assert_eq!(safe_name("..My Theme!!"), "My-Theme");
        let (items, total) = parse_items(br#"{"totalitems":"5","data":[{"id":7,"name":" X ","downloads":"3","tags":"qt6,x","summary":"<b>hi</b>","downloadname1":"a.tar.gz","downloadlink1":"http://x","downloadsize1":"12","changed":"2024-01-01T00"}]}"#);
        assert_eq!(total, 5);
        assert_eq!(items[0].name, "X");
        assert!(items[0].qt6);
        assert_eq!(items[0].summary, "hi");
        assert_eq!(items[0].files[0].size_kb, 12);
        assert_eq!(items[0].changed, "2024-01-01");
    }
}
