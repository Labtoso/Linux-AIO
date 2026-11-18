//! Download-Quelle GitHub: manifest.json aus dem festen Release „assets“ lesen (sonst dem neuesten), Archive laden,
//! sha256 prüfen und sicher nach ~/.local/share/icons entpacken. Läuft in Hintergrund-Aufgaben.
//!
//! manifest.json (Release-Asset):
//!   {"format": 1, "kategorien": {"cursors": {"eintraege": [{name, farbe, datei, groesse, sha256, vorschau}],
//!                                            "pakete": [{name, titel, datei, groesse, sha256, enthaelt}]},
//!                                "login": {...}, "icons": {...}}}

use crate::usercmd;
use once_cell::sync::Lazy;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

pub const DEFAULT_REPO: &str = "Labtoso/Linux-AIO";
pub const CATEGORIES: [&str; 4] = ["cursors", "login", "icons", "buttons"];
/// festes Release mit den Downloads, unabhängig von der App-Version
pub const ASSET_TAG: &str = "assets";
pub const MAX_ARCHIVE: usize = 600 * 1024 * 1024;
pub const MAX_UNPACKED: u64 = 3 * 1024 * 1024 * 1024;

pub fn ua() -> String {
    format!("{}/{} (LabtosoStudios)", crate::edition::ed().name.replace(' ', "-"), crate::edition::ed().version)
}

pub fn cache() -> PathBuf {
    usercmd::cache().join("remote")
}

/// Klare Meldung für die Oberfläche (Schlüssel für i18n + Detail).
#[derive(Debug, Clone)]
pub struct SourceError {
    pub key: String,
    pub detail: String,
}

impl SourceError {
    pub fn new(key: &str, detail: &str) -> SourceError {
        SourceError { key: key.into(), detail: detail.into() }
    }
}

impl std::fmt::Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", if self.detail.is_empty() { &self.key } else { &self.detail })
    }
}

/// Netzfehler wie urllib: HTTP-Code oder „offline“.
#[derive(Debug, Clone)]
pub enum NetErr {
    Http(u16),
    Offline(String),
}

impl std::fmt::Display for NetErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetErr::Http(c) => write!(f, "HTTP Error {}", c),
            NetErr::Offline(s) => write!(f, "{}", s),
        }
    }
}

type Fetcher = Box<dyn Fn(&str) -> Result<Vec<u8>, NetErr> + Send + Sync>;
/// Tests ersetzen das
static FETCH: Lazy<Mutex<Option<Fetcher>>> = Lazy::new(|| Mutex::new(None));

pub fn set_fetch(f: Option<Fetcher>) {
    *FETCH.lock().unwrap() = f;
}

fn net_err(e: ureq::Error) -> NetErr {
    match e {
        ureq::Error::Status(c, _) => NetErr::Http(c),
        ureq::Error::Transport(t) => NetErr::Offline(t.message().map(String::from).unwrap_or_else(|| t.to_string())),
    }
}

pub fn get(url: &str, timeout: u64, accept: &str) -> Result<Vec<u8>, NetErr> {
    if let Some(f) = FETCH.lock().unwrap().as_ref() {
        return f(url);
    }
    let agent = hyprstudio::net::builder().timeout(Duration::from_secs(timeout)).build();
    let mut req = agent.get(url).set("User-Agent", &ua());
    if !accept.is_empty() {
        req = req.set("Accept", accept);
    }
    let resp = req.call().map_err(net_err)?;
    let mut buf = Vec::new();
    resp.into_reader().read_to_end(&mut buf).map_err(|e| NetErr::Offline(e.to_string()))?;
    Ok(buf)
}

pub fn fetch(url: &str) -> Result<Vec<u8>, NetErr> {
    get(url, 20, "")
}

pub fn valid_repo(repo: &str) -> bool {
    let parts: Vec<&str> = repo.trim().split('/').collect();
    let ok = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || "-_.".contains(c)) && s != "." && s != "..";
    parts.len() == 2 && ok(parts[0]) && ok(parts[1])
}

#[derive(Debug, Clone, Default)]
pub struct Manifest {
    pub tag: String,
    pub repo: String,
    pub assets: Map<String, Value>,
    pub kategorien: Map<String, Value>,
}

/// Manifest des Releases laden oder SourceError.
pub fn load_manifest(repo: &str) -> Result<Manifest, SourceError> {
    if !valid_repo(repo) {
        return Err(SourceError::new("remote.badrepo", repo));
    }
    let mut rel: Option<Value> = None;
    for which in [format!("tags/{}", ASSET_TAG), "latest".into()] {
        let url = format!("https://api.github.com/repos/{}/releases/{}", repo, which);
        match get(&url, 20, "application/vnd.github+json") {
            Err(NetErr::Http(404)) => continue,
            Err(NetErr::Http(403)) => return Err(SourceError::new("remote.limit", repo)),
            Err(NetErr::Http(c)) => return Err(SourceError::new("remote.http", &c.to_string())),
            Err(NetErr::Offline(s)) => return Err(SourceError::new("remote.offline", &s)),
            Ok(b) => match serde_json::from_slice::<Value>(&b) {
                Ok(v) => {
                    rel = Some(v);
                    break;
                }
                Err(e) => return Err(SourceError::new("remote.badmanifest", &e.to_string())),
            },
        }
    }
    let Some(rel) = rel else { return Err(SourceError::new("remote.norelease", repo)) };
    let mut assets = Map::new();
    for a in rel.get("assets").and_then(|v| v.as_array()).cloned().unwrap_or_default() {
        if let Some(n) = a.get("name").and_then(|v| v.as_str()).filter(|n| !n.is_empty()) {
            assets.insert(n.into(), a.get("browser_download_url").cloned().unwrap_or(Value::Null));
        }
    }
    let tag = rel.get("tag_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let Some(murl) = assets.get("manifest.json").and_then(|v| v.as_str()).map(String::from) else {
        return Err(SourceError::new("remote.nomanifest", &tag));
    };
    let man: Value = match fetch(&murl) {
        Err(e) => return Err(SourceError::new("remote.offline", &e.to_string())),
        Ok(b) => serde_json::from_slice(&b).map_err(|e| SourceError::new("remote.badmanifest", &e.to_string()))?,
    };
    let Some(kat) = man.get("kategorien").and_then(|v| v.as_object()).cloned() else {
        return Err(SourceError::new("remote.badmanifest", "kategorien"));
    };
    Ok(Manifest { tag, repo: repo.into(), assets, kategorien: kat })
}

fn safe_name(name: &str) -> bool {
    !name.is_empty() && !name.contains('/') && !name.contains('\\') && name != "." && name != ".." && !name.starts_with('.')
}

/// Einträge und Pakete einer Kategorie, mit Download-Adressen (nur was es im Release wirklich gibt).
pub fn items(man: &Manifest, category: &str) -> (Vec<Map<String, Value>>, Vec<Map<String, Value>>) {
    let cat = man.kategorien.get(category).and_then(|v| v.as_object()).cloned().unwrap_or_default();
    let asset = |k: &Value| k.as_str().and_then(|s| man.assets.get(s)).cloned();
    let ok = |e: &Value| -> bool {
        let Some(o) = e.as_object() else { return false };
        let name = match o.get("name") { Some(Value::String(s)) => s.clone(), Some(v) => hyprstudio::jsonx::py_str(v), None => String::new() };
        safe_name(&name) && o.get("datei").map(|d| asset(d).is_some()).unwrap_or(false)
            && o.get("sha256").and_then(|s| s.as_str()).map(|s| s.chars().count() == 64).unwrap_or(false)
    };
    let list = |k: &str| cat.get(k).and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let entries = list("eintraege").into_iter().filter(|e| ok(e)).map(|e| {
        let mut o = e.as_object().unwrap().clone();
        o.insert("url".into(), asset(&o["datei"]).unwrap());
        let pv = o.get("vorschau").filter(|v| hyprstudio::jsonx::truthy(v)).cloned().unwrap_or(Value::String(String::new()));
        o.insert("preview_url".into(), asset(&pv).unwrap_or(Value::Null));
        o
    }).collect();
    let packs = list("pakete").into_iter().filter(|e| ok(e)).map(|e| {
        let mut o = e.as_object().unwrap().clone();
        o.insert("url".into(), asset(&o["datei"]).unwrap());
        o
    }).collect();
    (entries, packs)
}

pub fn sha256_hex(data: &[u8]) -> String {
    glib::compute_checksum_for_data(glib::ChecksumType::Sha256, data).map(|s| s.to_string()).unwrap_or_default()
}

/// Archiv laden und Prüfsumme kontrollieren (falsch → SourceError, nichts wird entpackt).
pub fn download(url: &str, sha: &str, size_hint: u64, mut progress: Option<&mut dyn FnMut(u32)>)
    -> Result<Vec<u8>, SourceError> {
    let data = if FETCH.lock().unwrap().is_some() {
        fetch(url).map_err(|e| SourceError::new("remote.offline", &e.to_string()))?
    } else {
        let agent = hyprstudio::net::builder().timeout(Duration::from_secs(60)).build();
        let resp = agent.get(url).set("User-Agent", &ua()).call()
            .map_err(|e| SourceError::new("remote.offline", &net_err(e).to_string()))?;
        let total: u64 = resp.header("Content-Length").and_then(|v| v.parse().ok()).filter(|n| *n > 0).unwrap_or(size_hint);
        let mut r = resp.into_reader();
        let mut buf = Vec::new();
        let mut chunk = vec![0u8; 1 << 16];
        loop {
            let n = r.read(&mut chunk).map_err(|e| SourceError::new("remote.offline", &e.to_string()))?;
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);
            if buf.len() > MAX_ARCHIVE {
                return Err(SourceError::new("remote.toobig", ""));
            }
            if let (Some(p), true) = (progress.as_mut(), total > 0) {
                p((buf.len() as u64 * 100 / total) as u32);
            }
        }
        buf
    };
    if sha256_hex(&data) != sha.to_lowercase() {
        return Err(SourceError::new("remote.badsum", ""));
    }
    Ok(data)
}

fn normpath(p: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for part in p.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if out.last().map(|l| *l != "..").unwrap_or(false) {
                    out.pop();
                } else {
                    out.push("..");
                }
            }
            x => out.push(x),
        }
    }
    if out.is_empty() { ".".into() } else { out.join("/") }
}

/// Relativer Link, der in einem Theme-Ordner desselben Archivs bleibt (Icon-Themes brauchen Tausende davon).
fn link_ok(name: &str, link: &str, tops: &BTreeSet<String>) -> bool {
    if link.starts_with('/') || link.contains('\\') {
        return false;
    }
    let dir = name.rsplit_once('/').map(|x| x.0).unwrap_or("");
    let target = normpath(&if dir.is_empty() { link.to_string() } else { format!("{}/{}", dir, link) });
    !target.starts_with("..") && tops.contains(target.split('/').next().unwrap_or(""))
}

fn parts(name: &str) -> Vec<String> {
    Path::new(name).components().filter_map(|c| match c {
        Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
        Component::ParentDir => Some("..".into()),
        Component::RootDir => Some("/".into()),
        _ => None,
    }).collect()
}

struct Member {
    name: String,
    kind: u8, // 0 Datei, 1 Ordner, 2 Link, 3 anderes
    link: String,
    size: u64,
    data: Vec<u8>,
}

/// tar.xz sicher entpacken: nur normale Dateien/Ordner, nichts außerhalb, keine Links (außer erlaubt),
/// nur erlaubte Theme-Ordner oben, vorhandene Themes werden nie überschrieben. Gibt neue Themes zurück.
pub fn safe_extract(data: &[u8], dest_root: &Path, allowed: Option<&BTreeSet<String>>, allow_links: bool)
    -> Result<Vec<PathBuf>, SourceError> {
    let bad = |e: String| SourceError::new("remote.badarchive", &e);
    let mut members = Vec::new();
    {
        let mut ar = tar::Archive::new(xz2::read::XzDecoder::new(data));
        for e in ar.entries().map_err(|e| bad(e.to_string()))? {
            let mut e = e.map_err(|e| bad(e.to_string()))?;
            let name = String::from_utf8_lossy(&e.path_bytes()).into_owned();
            let t = e.header().entry_type();
            let kind = if t.is_file() { 0 } else if t.is_dir() { 1 } else if t.is_symlink() { 2 } else { 3 };
            let link = e.link_name_bytes().map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
            let size = e.size();
            let mut d = Vec::new();
            if kind == 0 {
                e.read_to_end(&mut d).map_err(|e| bad(e.to_string()))?;
            }
            members.push(Member { name, kind, link, size, data: d });
        }
    }
    let mut total = 0u64;
    let mut tops: BTreeSet<String> = BTreeSet::new();
    let link_tops: BTreeSet<String> = match allowed {
        Some(a) => a.clone(),
        None => members.iter().filter_map(|m| parts(&m.name).into_iter().next()).collect(),
    };
    for m in &members {
        let p = parts(&m.name);
        if m.name.starts_with('/') || m.name.starts_with('\\') || p.iter().any(|x| x == "..") || p.is_empty() {
            return Err(bad(m.name.clone()));
        }
        let top = p[0].clone();
        if m.kind == 2 && allow_links && link_ok(&m.name, &m.link, &link_tops) {
        } else if !(m.kind == 0 || m.kind == 1) {
            return Err(bad(format!("{} (Link/Gerät)", m.name)));
        }
        if !safe_name(&top) || allowed.map(|a| !a.contains(&top)).unwrap_or(false) {
            return Err(bad(top));
        }
        tops.insert(top);
        total += m.size;
        if total > MAX_UNPACKED {
            return Err(SourceError::new("remote.toobig", ""));
        }
    }
    let _ = std::fs::create_dir_all(dest_root);
    let mut new = Vec::new();
    for top in &tops {
        if dest_root.join(top).exists() {
            continue;
        }
        let tmp = dest_root.join(format!(".{}.linuxaio-tmp", top));
        if tmp.exists() {
            let _ = std::fs::remove_dir_all(&tmp);
        }
        std::fs::create_dir(&tmp).map_err(|e| bad(e.to_string()))?;
        for m in &members {
            let p = parts(&m.name);
            if p[0] != *top {
                continue;
            }
            let target = p[1..].iter().fold(tmp.clone(), |a, b| a.join(b));
            if m.kind == 2 {
                let _ = std::fs::create_dir_all(target.parent().unwrap());
                if !target.exists() && !target.is_symlink() {
                    let _ = std::os::unix::fs::symlink(&m.link, &target);
                }
                continue;
            }
            if m.kind == 1 {
                let _ = std::fs::create_dir_all(&target);
                continue;
            }
            let _ = std::fs::create_dir_all(target.parent().unwrap());
            std::fs::write(&target, &m.data).map_err(|e| bad(e.to_string()))?;
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o644));
        }
        std::fs::rename(&tmp, dest_root.join(top)).map_err(|e| bad(e.to_string()))?;
        new.push(dest_root.join(top));
    }
    Ok(new)
}

/// Eintrag oder Paket laden, prüfen, entpacken.
pub fn install(entry: &Map<String, Value>, dest_root: &Path, progress: Option<&mut dyn FnMut(u32)>)
    -> Result<Vec<PathBuf>, SourceError> {
    let s = |k: &str| entry.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let size = entry.get("groesse").and_then(|v| v.as_u64()).unwrap_or(0);
    let data = download(&s("url"), &s("sha256"), size, progress)?;
    let allowed: BTreeSet<String> = match entry.get("enthaelt").and_then(|v| v.as_array()).filter(|a| !a.is_empty()) {
        Some(a) => a.iter().filter_map(|x| x.as_str().map(String::from)).collect(),
        None => [s("name")].into_iter().collect(),
    };
    let links = entry.get("links").map(hyprstudio::jsonx::truthy).unwrap_or(false);
    safe_extract(&data, dest_root, Some(&allowed), links)
}

/// Vorschaubild (zwischengespeichert).
pub fn preview(url: &str) -> Option<Vec<u8>> {
    if url.is_empty() {
        return None;
    }
    let _ = std::fs::create_dir_all(cache());
    let name = glib::compute_checksum_for_data(glib::ChecksumType::Sha1, url.as_bytes())?.to_string();
    let f = cache().join(name);
    if f.is_file() {
        return std::fs::read(&f).ok();
    }
    let data = fetch(url).ok()?;
    if data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        let _ = std::fs::write(&f, &data);
        return Some(data);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_and_repo() {
        let tops: BTreeSet<String> = ["Papirus".to_string(), "Papirus-Dark".to_string()].into_iter().collect();
        assert!(link_ok("Papirus-Dark/16x16", "../Papirus/16x16", &tops));
        assert!(!link_ok("Papirus-Dark/16x16", "../../etc", &tops));
        assert!(valid_repo("Labtoso/Linux-AIO"));
        assert!(!valid_repo("a/b/c"));
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }
}
