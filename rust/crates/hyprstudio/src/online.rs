//! Mauszeiger und Hintergrundbilder aus dem Netz holen (GitHub, KDE Store, wallhaven; ohne root).
//! Mauszeiger → ~/.local/share/icons/<Thema>, Hintergründe → ~/Bilder/Wallpapers/<Sammlung>. Vorhandenes bleibt.

use crate::jsonx::Get;
use crate::model::{self, Log};
use crate::{fonts, net, paper, util};
use serde_json::Value;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

fn gh(repo: &str, file: &str) -> String {
    format!("https://github.com/{}/releases/latest/download/{}", repo, file)
}

pub struct Cursor {
    pub id: &'static str,
    pub label: &'static str,
    pub repo: &'static str,
    pub file: &'static str,
    pub names: &'static [&'static str],
}

impl Cursor {
    pub fn url(&self) -> String {
        gh(self.repo, self.file)
    }
}

macro_rules! cur {
    ($id:literal, $label:literal, $repo:literal, $file:literal, [$($n:literal),*]) => {
        Cursor { id: $id, label: $label, repo: $repo, file: $file, names: &[$($n),*] }
    };
}

pub const CURSORS: &[Cursor] = &[
    cur!("bibata-classic", "Bibata Modern Classic", "ful1e5/Bibata_Cursor", "Bibata-Modern-Classic.tar.xz", ["Bibata-Modern-Classic"]),
    cur!("bibata-ice", "Bibata Modern Ice", "ful1e5/Bibata_Cursor", "Bibata-Modern-Ice.tar.xz", ["Bibata-Modern-Ice"]),
    cur!("bibata-amber", "Bibata Modern Amber", "ful1e5/Bibata_Cursor", "Bibata-Modern-Amber.tar.xz", ["Bibata-Modern-Amber"]),
    cur!("bibata-original", "Bibata Original Classic", "ful1e5/Bibata_Cursor", "Bibata-Original-Classic.tar.xz", ["Bibata-Original-Classic"]),
    cur!("catppuccin-dark", "Catppuccin Mocha Dark", "catppuccin/cursors", "catppuccin-mocha-dark-cursors.zip", ["catppuccin-mocha-dark-cursors"]),
    cur!("catppuccin-lavender", "Catppuccin Mocha Lavender", "catppuccin/cursors", "catppuccin-mocha-lavender-cursors.zip", ["catppuccin-mocha-lavender-cursors"]),
    cur!("phinger", "Phinger", "phisch/phinger-cursors", "phinger-cursors-variants.tar.bz2", ["phinger-cursors-dark", "phinger-cursors-light"]),
    cur!("nordzy", "Nordzy", "guillaumeboehm/Nordzy-cursors", "Nordzy-cursors.tar.gz", ["Nordzy-cursors"]),
    cur!("googledot-blue", "Google Dot Blue", "ful1e5/Google_Cursor", "GoogleDot-Blue.tar.gz", ["GoogleDot-Blue"]),
    cur!("googledot-black", "Google Dot Black", "ful1e5/Google_Cursor", "GoogleDot-Black.tar.gz", ["GoogleDot-Black"]),
    cur!("macos", "macOS", "ful1e5/apple_cursor", "macOS.tar.xz", ["macOS", "macOS-White"]),
];

pub struct WallPack {
    pub id: &'static str,
    pub label: &'static str,
    pub repo: &'static str,
    pub folder: &'static str,
}

macro_rules! wp {
    ($id:literal, $label:literal, $repo:literal, $folder:literal) => {
        WallPack { id: $id, label: $label, repo: $repo, folder: $folder }
    };
}

pub const WALL_PACKS: &[WallPack] = &[
    wp!("catppuccin-minimal", "Catppuccin Minimal", "zhichaoh/catppuccin-wallpapers", "minimalistic"),
    wp!("catppuccin-landscapes", "Catppuccin Landschaften", "zhichaoh/catppuccin-wallpapers", "landscapes"),
    wp!("nordic", "Nordic", "linuxdotexe/nordic-wallpapers", "wallpapers"),
    wp!("rosepine", "Rosé Pine", "rose-pine/wallpapers", "illustration"),
    wp!("mountain", "Berge", "dharmx/walls", "mountain"),
    wp!("minimal", "Minimal", "dharmx/walls", "minimal"),
    wp!("digital", "Digital", "dharmx/walls", "digital"),
    wp!("nord", "Nord", "dharmx/walls", "nord"),
    wp!("anime", "Anime", "dharmx/walls", "anime"),
];
pub const MAX_WALLS: usize = 12;

fn get(url: &str, timeout: u64) -> Result<Vec<u8>, String> {
    net::get_bytes(url, timeout, &net::user_agent())
}

fn get_json(url: &str, timeout: u64) -> Result<Value, String> {
    net::get_json(url, timeout, &net::user_agent())
}

// ---------- Mauszeiger ----------

pub fn icons_dir() -> PathBuf {
    paper::icon_dirs().remove(0)
}

pub fn installed_cursor(c: &Cursor) -> bool {
    let names: Vec<String> = paper::cursor_themes().into_iter().map(|x| x.0).collect();
    c.names.iter().any(|n| names.iter().any(|x| x == n))
}

/// Liegt `name` (relativ) innerhalb des Zielordners? Rein über die Pfadteile, ohne das Dateisystem.
fn inside(name: &str) -> bool {
    let mut depth: i32 = 0;
    for c in Path::new(name).components() {
        match c {
            Component::Normal(_) => depth += 1,
            Component::CurDir => {}
            Component::ParentDir => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => return false,
        }
    }
    true
}

/// Relativen Symlink anlegen (Mauszeiger-Aliase); nur wenn er innerhalb von dest bleibt.
fn link(dest: &Path, name: &str, target: &str) {
    let parent = Path::new(name).parent().unwrap_or(Path::new(""));
    if target.starts_with('/') || !inside(&parent.join(target).to_string_lossy()) {
        return;
    }
    let l = dest.join(name);
    if let Some(p) = l.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    let _ = std::os::unix::fs::symlink(target, &l);
}

fn write_entry(dest: &Path, name: &str, data: &[u8]) {
    let f = dest.join(name);
    if let Some(p) = f.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    let _ = std::fs::write(f, data);
}

/// Archiv entpacken; nichts außerhalb von dest, Symlinks nur relativ und nach innen.
pub fn safe_extract(data: &[u8], name: &str, dest: &Path) -> Result<(), String> {
    let mut links: Vec<(String, String)> = vec![];
    if name.ends_with(".zip") {
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(data)).map_err(|e| e.to_string())?;
        for i in 0..z.len() {
            let mut f = z.by_index(i).map_err(|e| e.to_string())?;
            let n = f.name().to_string();
            if !inside(&n) {
                continue;
            }
            let mode = f.unix_mode().unwrap_or(0);
            let mut buf = vec![];
            f.read_to_end(&mut buf).map_err(|e| e.to_string())?;
            if mode & 0o170000 == 0o120000 {
                links.push((n, String::from_utf8_lossy(&buf).into_owned()));
            } else if f.is_dir() {
                let _ = std::fs::create_dir_all(dest.join(&n));
            } else {
                write_entry(dest, &n, &buf);
            }
        }
    } else {
        let reader: Box<dyn Read> = if data.starts_with(&[0xfd, b'7', b'z', b'X', b'Z', 0]) {
            Box::new(xz2::read::XzDecoder::new(data))
        } else if data.starts_with(&[0x1f, 0x8b]) {
            Box::new(flate2::read::GzDecoder::new(data))
        } else if data.starts_with(b"BZh") {
            Box::new(bzip2::read::BzDecoder::new(data))
        } else {
            Box::new(data)
        };
        let mut ar = tar::Archive::new(reader);
        for e in ar.entries().map_err(|e| e.to_string())? {
            let mut e = e.map_err(|e| e.to_string())?;
            let n = e.path().map_err(|e| e.to_string())?.to_string_lossy().into_owned();
            if !inside(&n) {
                continue;
            }
            let t = e.header().entry_type();
            if t.is_symlink() {
                if let Ok(Some(target)) = e.link_name() {
                    links.push((n, target.to_string_lossy().into_owned()));
                }
            } else if t.is_dir() {
                let _ = std::fs::create_dir_all(dest.join(&n));
            } else if t.is_file() {
                let mut buf = vec![];
                e.read_to_end(&mut buf).map_err(|e| e.to_string())?;
                write_entry(dest, &n, &buf);
            }
        }
    }
    for (n, target) in links {
        link(dest, &n, &target);
    }
    Ok(())
}

/// Ordner mit index.theme und cursors/ bzw. Hyprcursor-Manifest (bis 3 Ebenen tief).
pub fn find_themes(root: &Path) -> Vec<PathBuf> {
    let mut idx: Vec<PathBuf> = fonts::walk(root).into_iter()
        .filter(|p| util::file_name(p) == "index.theme").collect();
    idx.sort();
    idx.into_iter().filter_map(|p| {
        let d = p.parent()?.to_path_buf();
        let depth = d.strip_prefix(root).ok()?.components().count();
        (depth <= 3 && (d.join("cursors").is_dir() || d.join("manifest.hl").is_file()
            || d.join("manifest.toml").is_file())).then_some(d)
    }).collect()
}

fn placeholder(dest: &Path, n: &str, log: Log) {
    let _ = std::fs::create_dir_all(dest.join(n).join("cursors"));
    let _ = std::fs::write(dest.join(n).join("index.theme"), format!("[Icon Theme]\nName={}\n", n));
    log(&format!("→ {}  (Testmodus)", dest.join(n).display()));
}

/// Lädt das Archiv und legt die Themen nach ~/.local/share/icons. Rückgabe: neu installierte Namen.
pub fn install_cursor(c: &Cursor, log: Log) -> Result<Vec<String>, String> {
    let dest = icons_dir();
    let _ = std::fs::create_dir_all(&dest);
    let url = c.url();
    log(&format!("↓ {}: {}", c.label, url));
    if model::dry() {
        for n in c.names {
            placeholder(&dest, n, log);
        }
        return Ok(c.names.iter().map(|s| s.to_string()).collect());
    }
    install_archive(&get(&url, 90)?, c.file, log)
}

fn move_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    if std::fs::rename(src, dst).is_ok() {
        return Ok(());
    }
    let st = util::capture(&["cp", "-a", &src.to_string_lossy(), &dst.to_string_lossy()], Some(120))?;
    if st.ok() { Ok(()) } else { Err(std::io::Error::other(st.stderr)) }
}

fn install_archive(data: &[u8], filename: &str, log: Log) -> Result<Vec<String>, String> {
    let dest = icons_dir();
    let _ = std::fs::create_dir_all(&dest);
    let tmp = fonts::tempdir()?;
    let res = (|| {
        safe_extract(data, filename, &tmp)?;
        let themes = find_themes(&tmp);
        if themes.is_empty() {
            log(&format!("Kein Mauszeiger-Thema in {} gefunden", filename));
        }
        let mut new = vec![];
        for d in themes {
            let name = util::file_name(&d);
            let target = dest.join(&name);
            if target.exists() || target.is_symlink() {
                log(&format!("= {} (schon da)", target.display()));
                continue;
            }
            move_dir(&d, &target).map_err(|e| e.to_string())?;
            log(&format!("→ {}", target.display()));
            new.push(name);
        }
        Ok(new)
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    res
}

// ---------- Mauszeiger-Katalog (KDE Store / pling, wie „Neue Mauszeiger holen“ in KDE) ----------

pub const STORE: &str = "https://api.kde-look.org/ocs/v1/content/data";
pub const STORE_CAT: i64 = 107;
pub const STORE_SORTS: &[&str] = &["high", "new", "down"];
const ARCHIVE_EXT: &[&str] = &[".tar.gz", ".tgz", ".tar.xz", ".txz", ".tar.bz2", ".tbz2", ".tar", ".zip"];

#[derive(Clone, Debug, Default)]
pub struct StoreItem {
    pub id: String,
    pub name: String,
    pub score: i64,
    pub downloads: i64,
    pub changed: String,
    pub preview: String,
    pub files: Vec<(i64, String, i64)>,
    pub page: String,
}

/// Wert als Text wie Pythons `str()` (Zahlen ohne Anführungszeichen).
fn vs(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        other => crate::jsonx::py_str(other),
    }
}

fn vi(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) => n.as_i64().or_else(|| n.as_f64().map(|f| f as i64)),
        Value::String(s) if s.is_empty() => Some(0),
        Value::String(s) => s.trim().parse().ok(),
        Value::Null | Value::Bool(false) => Some(0),
        _ => None,
    }
}

fn store_item(c: &Value) -> StoreItem {
    let mut files = vec![];
    for i in 1..60 {
        let name = vs(c.val(&format!("downloadname{}", i)));
        if !crate::jsonx::truthy(c.val(&format!("downloadlink{}", i))) {
            continue;
        }
        if ARCHIVE_EXT.iter().any(|e| name.to_lowercase().ends_with(e)) {
            files.push((i, name, vi(c.val(&format!("downloadsize{}", i))).unwrap_or(0)));
        }
    }
    let first = |keys: &[&str]| keys.iter().map(|k| vs(c.val(k))).find(|s| !s.is_empty()).unwrap_or_default();
    let score = vi(c.val("score")).filter(|x| *x != 0).unwrap_or(50);
    StoreItem {
        id: vs(c.val("id")),
        name: vs(c.val("name")).trim().to_string(),
        score,
        downloads: vi(c.val("downloads")).unwrap_or(0),
        changed: first(&["changed", "created"]).chars().take(10).collect(),
        preview: first(&["smallpreviewpic1", "previewpic1"]),
        files,
        page: first(&["detailpage"]),
    }
}

fn urlencode(params: &[(&str, String)]) -> String {
    fn enc(s: &str) -> String {
        let mut o = String::new();
        for b in s.bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'.' | b'-' | b'~' => o.push(b as char),
                b' ' => o.push('+'),
                _ => o.push_str(&format!("%{:02X}", b)),
            }
        }
        o
    }
    params.iter().map(|(k, v)| format!("{}={}", enc(k), enc(v))).collect::<Vec<_>>().join("&")
}

/// ([Eintrag], Gesamtzahl) – 10 pro Seite; nur Einträge mit Archiv zum Herunterladen.
pub fn store_search(query: &str, sort: &str, page: i64) -> Result<(Vec<StoreItem>, i64), String> {
    let sort = if STORE_SORTS.contains(&sort) { sort } else { "high" };
    let mut params = vec![("categories", STORE_CAT.to_string()), ("sortmode", sort.into()), ("page", page.to_string()),
        ("pagesize", "10".into()), ("format", "json".into())];
    if !query.trim().is_empty() {
        params.push(("search", query.trim().into()));
    }
    let d = get_json(&format!("{}?{}", STORE, urlencode(&params)), 30)?;
    let items: Vec<StoreItem> = d.arr("data").iter().filter(|c| c.is_object()).map(store_item)
        .filter(|x| !x.files.is_empty()).collect();
    Ok((items, vi(d.val("totalitems")).unwrap_or(0)))
}

/// Download-Link ist nur kurz gültig → frisch holen, dann wie ein Release-Archiv installieren.
pub fn store_install(item: &StoreItem, index: i64, log: Log) -> Result<Vec<String>, String> {
    let name = item.files.iter().find(|f| f.0 == index).map(|f| f.1.clone()).unwrap_or_default();
    log(&format!("↓ {}: {}", item.name, name));
    if model::dry() {
        let base = util::file_name(Path::new(&name));
        let mut n = base.split('.').next().unwrap_or("").to_string();
        if n.is_empty() {
            n = item.name.clone();
        }
        placeholder(&icons_dir(), &n, log);
        return Ok(vec![n]);
    }
    let d = get_json(&format!("{}/{}?format=json", STORE, item.id), 30)?;
    let data = d.val("data");
    let c = match data {
        Value::Array(a) => a.first().cloned().unwrap_or(Value::Null),
        other => other.clone(),
    };
    let url = c.s(&format!("downloadlink{}", index)).to_string();
    install_archive(&get(&url, 90)?, &name, log)
}

// ---------- Hintergrundbilder ----------

pub fn pictures_dir() -> PathBuf {
    if model::dry() {
        return model::cache().join("testmodus").join("Bilder");
    }
    if let Ok(o) = util::capture(&["xdg-user-dir", "PICTURES"], Some(5)) {
        let p = o.stdout.trim().to_string();
        if !p.is_empty() && p != util::home().to_string_lossy() {
            return PathBuf::from(p);
        }
    }
    util::home().join("Pictures")
}

pub fn wall_dir() -> PathBuf {
    pictures_dir().join("Wallpapers")
}

pub fn library_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![wall_dir()];
    if !model::dry() {
        dirs.push(util::home().join(".local/share/backgrounds"));
        dirs.push(PathBuf::from("/usr/share/hypr"));
        dirs.push(PathBuf::from("/usr/share/backgrounds"));
    }
    dirs
}

pub fn library_images(limit: usize) -> Vec<PathBuf> {
    let mut out = vec![];
    for d in library_dirs() {
        if !d.is_dir() {
            continue;
        }
        let mut files: Vec<PathBuf> = fonts::walk(&d).into_iter().filter(|p| {
            paper::is_image(p) && p.is_file()
                && p.strip_prefix(&d).map(|r| r.components().count() <= 3).unwrap_or(false)
        }).collect();
        files.sort();
        out.extend(files);
        if out.len() >= limit {
            break;
        }
    }
    out.truncate(limit);
    out
}

/// [(Name, Download-URL)] – die ersten MAX_WALLS Bilder der Sammlung (GitHub-API).
pub fn pack_files(p: &WallPack) -> Result<Vec<(String, String)>, String> {
    let data = get_json(&format!("https://api.github.com/repos/{}/contents/{}", p.repo, p.folder), 30)?;
    let mut imgs: Vec<(String, String)> = data.as_array().map(|a| a.iter().filter(|x| {
        x.s("type") == "file" && paper::IMAGE_EXT.iter().any(|e| x.s("name").to_lowercase().ends_with(e))
    }).map(|x| (x.s("name").to_string(), x.s("download_url").to_string())).collect()).unwrap_or_default();
    imgs.sort();
    imgs.truncate(MAX_WALLS);
    Ok(imgs)
}

pub fn install_walls(p: &WallPack, log: Log) -> Result<usize, String> {
    let dest = wall_dir().join(p.id);
    let _ = std::fs::create_dir_all(&dest);
    log(&format!("↓ {}: github.com/{}/{}", p.label, p.repo, p.folder));
    if model::dry() {
        log(&format!("→ {}  (Testmodus, nichts geladen)", dest.display()));
        return Ok(0);
    }
    let mut n = 0;
    for (name, url) in pack_files(p)? {
        let f = dest.join(&name);
        if f.exists() {
            continue;
        }
        std::fs::write(&f, get(&url, 90)?).map_err(|e| e.to_string())?;
        n += 1;
        log(&format!("→ {}", f.display()));
    }
    Ok(n)
}

pub fn short(p: &Path) -> String {
    let s = p.to_string_lossy().to_string();
    let home = util::home().to_string_lossy().to_string();
    match s.strip_prefix(&home) {
        Some(rest) => format!("~{}", rest),
        None => s,
    }
}

pub fn open_dir(p: &Path) {
    let _ = std::fs::create_dir_all(p);
    if !model::dry() && util::has("xdg-open") {
        let _ = util::spawn_detached(&["xdg-open", &p.to_string_lossy()]);
    }
}

// ---------- Hintergrundbilder suchen (wallhaven.cc, nur jugendfrei) ----------

pub const WALLHAVEN: &str = "https://wallhaven.cc/api/v1/search";
pub const WALL_SORTS: &[&str] = &["toplist", "date_added", "favorites", "random"];
pub const WALL_CATS: &[(&str, &str)] = &[("all", "110"), ("general", "100"), ("anime", "010")];
pub const WALL_RES: &[(&str, &str)] = &[("all", ""), ("fhd", "1920x1080"), ("qhd", "2560x1440"), ("uhd", "3840x2160")];

#[derive(Clone, Debug, Default)]
pub struct WallItem {
    pub id: String,
    pub thumb: String,
    pub full: String,
    pub res: String,
    pub favs: i64,
    pub size: i64,
}

/// ([Bild], letzte Seite) – 24 pro Seite.
pub fn wall_search(query: &str, sort: &str, cat: &str, res: &str, page: i64) -> Result<(Vec<WallItem>, i64), String> {
    let sort = if WALL_SORTS.contains(&sort) { sort } else { "toplist" };
    let cats = WALL_CATS.iter().find(|c| c.0 == cat).map(|c| c.1).unwrap_or("110");
    let mut params = vec![("q", query.trim().to_string()), ("sorting", sort.into()), ("purity", "100".into()),
        ("categories", cats.into()), ("page", page.to_string())];
    if sort == "toplist" {
        params.push(("topRange", "1y".into()));
    }
    if let Some((_, r)) = WALL_RES.iter().find(|r| r.0 == res && !r.1.is_empty()) {
        params.push(("atleast", r.to_string()));
    }
    let d = get_json(&format!("{}?{}", WALLHAVEN, urlencode(&params)), 30)?;
    let items = d.arr("data").iter().filter(|x| x.s("purity") == "sfw").map(|x| WallItem {
        id: vs(x.val("id")),
        thumb: x.val("thumbs").s("small").to_string(),
        full: x.s("path").to_string(),
        res: x.s("resolution").to_string(),
        favs: vi(x.val("favorites")).unwrap_or(0),
        size: vi(x.val("file_size")).unwrap_or(0),
    }).collect();
    let last = vi(d.val("meta").val("last_page")).filter(|x| *x != 0).unwrap_or(1);
    Ok((items, last))
}

pub fn wall_install(item: &WallItem, log: Log) -> Result<PathBuf, String> {
    let dest = wall_dir().join("wallhaven");
    let _ = std::fs::create_dir_all(&dest);
    let name = item.full.rsplit('/').next().unwrap_or("bild.jpg");
    let f = dest.join(name);
    if !f.exists() {
        log(&format!("↓ {}", item.full));
        std::fs::write(&f, get(&item.full, 90)?).map_err(|e| e.to_string())?;
    }
    log(&format!("→ {}", f.display()));
    Ok(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inside_works() {
        assert!(inside("a/b/c"));
        assert!(inside("a/../b"));
        assert!(!inside("../x"));
        assert!(!inside("/etc/passwd"));
        assert!(!inside("a/../../x"));
    }

    #[test]
    fn encode() {
        assert_eq!(urlencode(&[("q", "a b&c".into())]), "q=a+b%26c");
    }
}
