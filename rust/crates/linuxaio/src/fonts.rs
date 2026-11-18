//! Schriften direkt von der Quelle installieren: Google Fonts (OFL/Apache/UFL) und Nerd Fonts (GitHub-Releases).
//! Nichts davon liegt in unserem Repo. Ziel: ~/.local/share/fonts/LinuxAIO/<Familie>/, danach fc-cache.

use crate::remote::{self, SourceError};
use crate::usercmd;
use hyprstudio::util;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;
use std::io::Read;
use std::path::{Path, PathBuf};

pub const GOOGLE_META: &str = "https://fonts.google.com/metadata/fonts";
pub const NERD_API: &str = "https://api.github.com/repos/ryanoasis/nerd-fonts/releases/latest";
pub const CATEGORIES: [&str; 6] = ["Sans Serif", "Serif", "Monospace", "Display", "Handwriting", "Nerd Font"];
pub const MAX_FONT_ARCHIVE: usize = 400 * 1024 * 1024;
pub const CACHE_TTL: u64 = 24 * 3600;

pub fn fonts_dir() -> PathBuf {
    usercmd::data_dir().join("fonts").join("LinuxAIO")
}

pub fn cache_dir() -> PathBuf {
    usercmd::cache().join("fonts")
}

/// Ordnername aus dem Familiennamen (keine Pfadzeichen).
pub fn safe_family(name: &str) -> String {
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"[^A-Za-z0-9 ._+-]").unwrap());
    let s = RE.replace_all(name, "").trim().trim_matches('.').to_string();
    if s.is_empty() { "Font".into() } else { s }
}

fn json_of(data: &[u8]) -> Result<Value, String> {
    let text = String::from_utf8_lossy(data);
    let i = text.find('{').ok_or("substring not found")?; // Google setzt )]}' davor
    serde_json::from_str(&text[i..]).map_err(|e| e.to_string())
}

fn cached(name: &str, url: &str, accept: &str) -> Result<Value, SourceError> {
    let f = cache_dir().join(name);
    if let Ok(m) = f.metadata() {
        let age = m.modified().ok().and_then(|t| t.elapsed().ok()).map(|d| d.as_secs()).unwrap_or(u64::MAX);
        if f.is_file() && age < CACHE_TTL {
            if let Ok(v) = json_of(&std::fs::read(&f).unwrap_or_default()) {
                return Ok(v);
            }
        }
    }
    let data = if accept.is_empty() { remote::fetch(url) } else { remote::get(url, 30, accept) };
    match data {
        Err(e) => {
            if f.is_file() {
                return json_of(&std::fs::read(&f).unwrap_or_default())
                    .map_err(|x| SourceError::new("remote.badmanifest", &x));
            }
            Err(SourceError::new("remote.offline", &e.to_string()))
        }
        Ok(d) => {
            let _ = std::fs::create_dir_all(f.parent().unwrap());
            let _ = std::fs::write(&f, &d);
            json_of(&d).map_err(|x| SourceError::new("remote.badmanifest", &x))
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct FontItem {
    pub family: String,
    pub category: String,
    pub source: String,
    pub popularity: i64,
    pub noto: bool,
    pub archive: String,
    pub tag: String,
    pub size: i64,
    pub url: String,
}

fn vs(v: &Value, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

fn vi(v: &Value, k: &str, d: i64) -> i64 {
    match v.get(k) {
        Some(Value::Number(n)) => n.as_i64().or_else(|| n.as_f64().map(|f| f as i64)).unwrap_or(d),
        Some(Value::String(s)) if !s.is_empty() => s.parse().unwrap_or(d),
        _ => d,
    }
}

// ---------- Kataloge ----------

pub fn google_catalog() -> Result<Vec<FontItem>, SourceError> {
    let d = cached("google.json", GOOGLE_META, "")?;
    let mut out = Vec::new();
    for e in d.get("familyMetadataList").and_then(|v| v.as_array()).cloned().unwrap_or_default() {
        let open = e.get("isOpenSource").map(hyprstudio::jsonx::truthy).unwrap_or(true);
        let brand = e.get("isBrandFont").map(hyprstudio::jsonx::truthy).unwrap_or(false);
        if !open || brand {
            continue;
        }
        let cat = e.get("category").and_then(|v| v.as_str()).unwrap_or("Sans Serif").to_string();
        let pop = match vi(&e, "popularity", 0) { 0 => 9999, p => p };
        out.push(FontItem { family: vs(&e, "family"), category: cat, source: "google".into(), popularity: pop,
                            noto: e.get("isNoto").map(hyprstudio::jsonx::truthy).unwrap_or(false), ..Default::default() });
    }
    out.sort_by_key(|x| x.popularity);
    Ok(out)
}

pub fn nerd_catalog() -> Result<Vec<FontItem>, SourceError> {
    let d = cached("nerd.json", NERD_API, "application/vnd.github+json")?;
    let tag = vs(&d, "tag_name");
    let mut out = Vec::new();
    for a in d.get("assets").and_then(|v| v.as_array()).cloned().unwrap_or_default() {
        let n = vs(&a, "name");
        if let Some(base) = n.strip_suffix(".tar.xz") {
            let url = match vs(&a, "browser_download_url") {
                u if !u.is_empty() => u,
                _ => format!("https://github.com/ryanoasis/nerd-fonts/releases/download/{}/{}.tar.xz", tag, base),
            };
            out.push(FontItem { family: format!("{} Nerd Font", base), archive: base.into(), category: "Nerd Font".into(),
                                source: "nerd".into(), popularity: 0, tag: tag.clone(), size: vi(&a, "size", 0), url,
                                ..Default::default() });
        }
    }
    out.sort_by_key(|x| x.family.to_lowercase());
    Ok(out)
}

/// Beide Quellen; Fehler einer Quelle → die andere bleibt, Meldung zurück.
pub fn catalog() -> (Vec<FontItem>, String) {
    let mut items = Vec::new();
    let mut errors = Vec::new();
    for r in [google_catalog(), nerd_catalog()] {
        match r {
            Ok(v) => items.extend(v),
            Err(e) => errors.push(e.key),
        }
    }
    let err = if !errors.is_empty() && items.is_empty() { errors[0].clone() } else { String::new() };
    (items, err)
}

// ---------- Installieren ----------

fn is_font(p: &Path) -> bool {
    let n = util::file_name(p);
    n.ends_with(".ttf") || n.ends_with(".otf")
}

fn fonts_in(d: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(d).map(|r| r.flatten().map(|e| e.path()).filter(|p| is_font(p)).collect())
        .unwrap_or_default();
    v.sort();
    v
}

pub fn installed(family: &str) -> Option<PathBuf> {
    let p = fonts_dir().join(safe_family(family));
    (p.is_dir() && !fonts_in(&p).is_empty()).then_some(p)
}

pub fn list_installed() -> std::collections::BTreeSet<String> {
    std::fs::read_dir(fonts_dir()).map(|r| r.flatten().map(|e| e.path()).filter(|p| p.is_dir())
        .map(|p| util::file_name(&p)).collect()).unwrap_or_default()
}

/// `urllib.parse.quote` (safe="/")
pub fn quote(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'.' | b'-' | b'~' | b'/' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn netloc(u: &str) -> String {
    let rest = u.split_once("://").map(|x| x.1).unwrap_or("");
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    host.to_string()
}

/// (Schrift-URLs, Lizenzdateien). Variable Schriften bevorzugt, sonst statische.
pub fn google_files(family: &str) -> Result<(Vec<(String, String)>, Vec<(String, String)>), SourceError> {
    let data = remote::fetch(&format!("https://fonts.google.com/download/list?family={}", quote(family)))
        .map_err(|e| SourceError::new("remote.offline", &e.to_string()))?;
    let man = json_of(&data).map_err(|e| SourceError::new("remote.badmanifest", &e))?
        .get("manifest").cloned().unwrap_or(Value::Null);
    let refs: Vec<Value> = man.get("fileRefs").and_then(|v| v.as_array()).cloned().unwrap_or_default().into_iter()
        .filter(|r| { let f = vs(r, "filename").to_lowercase(); f.ends_with(".ttf") || f.ends_with(".otf") }).collect();
    let variable: Vec<Value> = refs.iter().filter(|r| !vs(r, "filename").starts_with("static/")).cloned().collect();
    let using = if variable.is_empty() { refs } else { variable };
    static LIC: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)^(OFL|LICENSE|UFL)").unwrap());
    let mut licenses: Vec<(String, String)> = Vec::new();
    for f in man.get("files").and_then(|v| v.as_array()).cloned().unwrap_or_default() {
        let n = vs(&f, "filename");
        if LIC.is_match(&n) {
            match licenses.iter_mut().find(|(k, _)| *k == n) {
                Some(e) => e.1 = vs(&f, "contents"),
                None => licenses.push((n, vs(&f, "contents"))),
            }
        }
    }
    let mut urls = Vec::new();
    for r in using {
        let u = vs(&r, "url");
        if netloc(&u).ends_with("fonts.gstatic.com") {
            urls.push((util::file_name(Path::new(&vs(&r, "filename"))), u));
        }
    }
    Ok((urls, licenses))
}

pub fn ok_font(data: &[u8]) -> bool {
    data.len() >= 4 && [&b"\x00\x01\x00\x00"[..], b"OTTO", b"true", b"ttcf"].contains(&&data[..4])
}

fn finish(tmp: &Path, dest: &Path, log: &dyn Fn(&str)) -> PathBuf {
    if dest.exists() {
        let _ = std::fs::remove_dir_all(tmp);
        return dest.to_path_buf();
    }
    let _ = std::fs::rename(tmp, dest);
    log(&format!("→ {}", dest.display()));
    usercmd::run_t(&usercmd::sv(&["fc-cache", "-f", &dest.parent().unwrap().display().to_string()]), 120);
    dest.to_path_buf()
}

fn file_safe(name: &str) -> String {
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"[^A-Za-z0-9._+\[\],-]").unwrap());
    RE.replace_all(name, "_").into_owned()
}

fn fresh_tmp(dest: &Path) -> PathBuf {
    let tmp = util::with_name(dest, &format!(".{}.linuxaio-tmp", util::file_name(dest)));
    if tmp.exists() {
        let _ = std::fs::remove_dir_all(&tmp);
    }
    let _ = std::fs::create_dir_all(&tmp);
    tmp
}

pub fn install_google(family: &str, log: &dyn Fn(&str)) -> Result<PathBuf, SourceError> {
    let dest = fonts_dir().join(safe_family(family));
    if installed(family).is_some() {
        return Ok(dest);
    }
    let (urls, licenses) = google_files(family)?;
    if urls.is_empty() {
        return Err(SourceError::new("fonts.nofiles", family));
    }
    let tmp = fresh_tmp(&dest);
    for (name, url) in urls {
        let data = remote::fetch(&url).map_err(|e| {
            let _ = std::fs::remove_dir_all(&tmp);
            SourceError::new("remote.offline", &e.to_string())
        })?;
        if !ok_font(&data) {
            let _ = std::fs::remove_dir_all(&tmp);
            return Err(SourceError::new("fonts.bad", &name));
        }
        let _ = std::fs::write(tmp.join(file_safe(&name)), &data);
    }
    for (name, text) in licenses {
        let _ = std::fs::write(tmp.join(util::file_name(Path::new(&name))), text);
    }
    Ok(finish(&tmp, &dest, log))
}

pub fn install_nerd(item: &FontItem, log: &dyn Fn(&str)) -> Result<PathBuf, SourceError> {
    let dest = fonts_dir().join(safe_family(&item.family));
    if installed(&item.family).is_some() {
        return Ok(dest);
    }
    let data = remote::fetch(&item.url).map_err(|e| SourceError::new("remote.offline", &e.to_string()))?;
    if data.len() > MAX_FONT_ARCHIVE {
        return Err(SourceError::new("remote.toobig", ""));
    }
    let mut ar = tar::Archive::new(xz2::read::XzDecoder::new(&data[..]));
    let entries = ar.entries().map_err(|e| SourceError::new("remote.badarchive", &e.to_string()))?;
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    for e in entries {
        let mut e = e.map_err(|e| SourceError::new("remote.badarchive", &e.to_string()))?;
        // nur Schriften und Lizenzen, flach, keine Links/Pfade
        let name = String::from_utf8_lossy(&e.path_bytes()).into_owned();
        let base = util::file_name(Path::new(&name));
        if !e.header().entry_type().is_file() || base.is_empty() || base.starts_with('.') {
            continue;
        }
        let low = base.to_lowercase();
        if low.ends_with(".ttf") || low.ends_with(".otf") || ["license", "license.txt", "ofl.txt", "license.md"].contains(&low.as_str()) {
            let mut b = Vec::new();
            e.read_to_end(&mut b).map_err(|e| SourceError::new("remote.badarchive", &e.to_string()))?;
            files.push((base, b));
        }
    }
    let tmp = fresh_tmp(&dest);
    let mut count = 0;
    for (base, b) in files {
        let _ = std::fs::write(tmp.join(&base), b);
        let low = base.to_lowercase();
        if low.ends_with(".ttf") || low.ends_with(".otf") {
            count += 1;
        }
    }
    if count == 0 {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(SourceError::new("fonts.nofiles", &item.family));
    }
    Ok(finish(&tmp, &dest, log))
}

pub fn install(item: &FontItem, log: &dyn Fn(&str)) -> Result<PathBuf, SourceError> {
    if item.source == "nerd" { install_nerd(item, log) } else { install_google(&item.family, log) }
}

/// Nur unsere eigenen Ordner unter fonts/LinuxAIO entfernen (vom Nutzer per Knopf gewünscht).
pub fn remove(family: &str, log: &dyn Fn(&str)) -> bool {
    let p = fonts_dir().join(safe_family(family));
    if !p.is_dir() || p.parent() != Some(fonts_dir().as_path()) {
        return false;
    }
    let _ = std::fs::remove_dir_all(&p);
    log(&format!("entfernt: {}", p.display()));
    usercmd::run_t(&usercmd::sv(&["fc-cache", "-f", &fonts_dir().display().to_string()]), 120);
    true
}

// ---------- Vorschau ----------

/// Eine Datei für die Live-Vorschau: installierte Schrift, sonst (Google) eine Datei in den Cache laden.
pub fn preview_file(item: &FontItem) -> Option<PathBuf> {
    if let Some(inst) = installed(&item.family) {
        let mut files = fonts_in(&inst);
        files.sort_by_key(|p| { let n = util::file_name(p); (!n.to_lowercase().contains("regular"), n.chars().count()) });
        return files.into_iter().next();
    }
    if item.source != "google" {
        return None;
    }
    let d = cache_dir().join("preview").join(safe_family(&item.family));
    if let Some(f) = fonts_in(&d).into_iter().next() {
        return Some(f);
    }
    let (mut urls, _) = google_files(&item.family).ok()?;
    if urls.is_empty() {
        return None;
    }
    urls.sort_by_key(|(n, _)| (!n.to_lowercase().contains("regular") && !n.contains('['), n.chars().count()));
    let (name, url) = urls.swap_remove(0);
    let data = remote::fetch(&url).ok()?;
    if !ok_font(&data) {
        return None;
    }
    let _ = std::fs::create_dir_all(&d);
    let f = d.join(file_safe(&name));
    let _ = std::fs::write(&f, data);
    Some(f)
}

#[cfg(test)]
mod tests {
    #[test]
    fn names() {
        assert_eq!(super::safe_family("Fira/Code!"), "FiraCode");
        assert_eq!(super::quote("Noto Sans"), "Noto%20Sans");
        assert_eq!(super::netloc("https://fonts.gstatic.com/s/x.ttf"), "fonts.gstatic.com");
    }
}
