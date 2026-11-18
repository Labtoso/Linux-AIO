//! Nach Updates suchen: neueste Programmdatei dieser App in den v-Releases von Labtoso/Linux-AIO (das Release „assets“ wird ignoriert), Änderungen,
//! Download des Programms mit sha256-Prüfung (SHA256SUMS im Release), atomarer Austausch von ~/.local/bin/Linux-AIO
//! (das alte kommt nach ~/.local/share/LabtosoStudios/linuxaio/old/<Version>/). Einstellungen bleiben.

use crate::remote::{self, NetErr, SourceError};
use crate::usercmd;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::Duration;

pub const API: &str = "https://api.github.com/repos/Labtoso/Linux-AIO/releases?per_page=20";
/// Endung der Programmdatei im Release (Linux-AIO-<Version>-x86_64)
pub const ASSET_SUFFIX: &str = "-x86_64";

/// Programmdatei dieser App: Linux-AIO-<Version>-x86_64 bzw. SDDM-Studio-<Version>-x86_64
fn prefix() -> &'static str {
    crate::edition::ed().asset_prefix
}

/// Version aus dem Dateinamen (ein Release enthält alle Apps mit eigener Version).
fn asset_version(name: &str) -> Option<String> {
    let v = name.strip_prefix(prefix())?.strip_suffix(ASSET_SUFFIX)?;
    let ok = v.split('.').count() == 3 && v.split('.').all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));
    ok.then(|| v.to_string())
}

pub fn vtuple(v: &str) -> Vec<u64> {
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\d+").unwrap());
    let t: Vec<u64> = RE.find_iter(v).take(3).filter_map(|m| m.as_str().parse().ok()).collect();
    if t.is_empty() { vec![0] } else { t }
}

fn tag_ok(tag: &str) -> bool {
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^v\d+\.\d+\.\d+$").unwrap());
    RE.is_match(tag)
}

#[derive(Debug, Clone, Default)]
pub struct Info {
    pub version: String,
    pub notes: String,
    pub app_url: String,
    pub app_name: String,
    pub sums_url: String,
    pub size: u64,
}

fn get(url: &str) -> Result<Vec<u8>, SourceError> {
    remote::get(url, 20, "application/vnd.github+json").map_err(|e| SourceError::new("remote.offline", &e.to_string()))
}

/// Neueste echte Version oder None.
pub fn latest(releases: Option<Vec<Value>>) -> Result<Option<Info>, SourceError> {
    let releases = match releases {
        Some(r) => r,
        None => match remote::get(API, 20, "application/vnd.github+json") {
            Ok(b) => serde_json::from_slice::<Vec<Value>>(&b).unwrap_or_default(),
            Err(NetErr::Http(403)) | Err(NetErr::Http(429)) => return latest_without_api(), // API-Limit erreicht
            Err(e) => return Err(SourceError::new("remote.offline", &e.to_string())),
        },
    };
    let mut best: Option<Info> = None;
    for r in releases {
        let tag = r.get("tag_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let flag = |k: &str| r.get(k).map(hyprstudio::jsonx::truthy).unwrap_or(false);
        if flag("draft") || flag("prerelease") || !tag_ok(&tag) {
            continue; // „assets“ und Entwürfe ignorieren
        }
        let assets = r.get("assets").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let name_of = |a: &Value| a.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let app = assets.iter().find(|a| asset_version(&name_of(a)).is_some());
        let sums = assets.iter().find(|a| name_of(a) == "SHA256SUMS");
        let (Some(app), Some(sums)) = (app, sums) else { continue };
        let url = |a: &Value| a.get("browser_download_url").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let info = Info { version: asset_version(&name_of(app)).unwrap_or_default(), notes: r.get("body").and_then(|v| v.as_str()).unwrap_or("").into(),
                          app_url: url(app), app_name: name_of(app), sums_url: url(sums),
                          size: app.get("size").and_then(|v| v.as_u64()).unwrap_or(0) };
        if best.as_ref().map(|b| vtuple(&info.version) > vtuple(&b.version)).unwrap_or(true) {
            best = Some(info);
        }
    }
    Ok(best)
}

/// Ohne GitHub-API: …/releases/latest leitet auf die neueste Version um.
fn latest_without_api() -> Result<Option<Info>, SourceError> {
    let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(20)).redirects(0).build();
    let resp = agent.head("https://github.com/Labtoso/Linux-AIO/releases/latest").set("User-Agent", &remote::ua()).call();
    let loc = match resp {
        Ok(r) => r.header("Location").map(String::from).unwrap_or_else(|| r.get_url().to_string()),
        Err(ureq::Error::Status(_, r)) => r.header("Location").map(String::from).unwrap_or_default(),
        Err(e) => return Err(SourceError::new("remote.offline", &e.to_string())),
    };
    let tag = loc.trim_end_matches('/').rsplit('/').next().unwrap_or("").to_string();
    if !tag_ok(&tag) {
        return Ok(None);
    }
    let base = format!("https://github.com/Labtoso/Linux-AIO/releases/download/{}/", tag);
    // Name der Programmdatei steht in SHA256SUMS
    let sums = String::from_utf8_lossy(&get(&format!("{}SHA256SUMS", base))?).into_owned();
    let Some(name) = sums.lines().filter_map(|l| l.split_whitespace().last()).find(|n| asset_version(n).is_some()) else {
        return Ok(None);
    };
    let name = name.trim_start_matches('*').to_string();
    Ok(Some(Info { version: asset_version(&name).unwrap_or_default(), notes: String::new(), app_url: format!("{}{}", base, name),
                   app_name: name, sums_url: format!("{}SHA256SUMS", base), size: 0 }))
}

pub fn newer(info: Option<&Info>) -> bool {
    info.map(|i| vtuple(&i.version) > vtuple(crate::edition::ed().version)).unwrap_or(false)
}

pub fn short_notes(notes: &str, lines: usize) -> String {
    notes.replace('\r', "").lines().filter(|l| !l.trim().is_empty() && !l.starts_with("## Install"))
        .take(lines).collect::<Vec<_>>().join("\n")
}

pub fn target() -> PathBuf {
    let bin = usercmd::real_home().join(".local/bin");
    // läuft schon eine Kopie in ~/.local/bin (auch die frühere Linux-AIO.AppImage): genau die ersetzen
    if let Ok(me) = std::env::current_exe() {
        if me.parent() == Some(bin.as_path()) {
            return me;
        }
    }
    bin.join(crate::edition::ed().local_name)
}

/// Laden → sha256 prüfen → altes Programm nach old/ → neues atomar an seinen Platz.
pub fn install(info: &Info, progress: &mut dyn FnMut(u32), log: &dyn Fn(&str)) -> Result<PathBuf, SourceError> {
    let sums = String::from_utf8_lossy(&get(&info.sums_url)?).into_owned();
    let want = sums.lines().find(|l| l.trim().ends_with(&info.app_name))
        .and_then(|l| l.split_whitespace().next()).unwrap_or("").to_string();
    if want.len() != 64 || !want.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()) {
        return Err(SourceError::new("remote.badsum", "SHA256SUMS"));
    }
    let dst = target();
    let _ = std::fs::create_dir_all(dst.parent().unwrap());
    let part = dst.with_file_name(format!("{}.part", crate::edition::ed().local_name));
    let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(60)).build();
    let off = |e: String| {
        let _ = std::fs::remove_file(&part);
        SourceError::new("remote.offline", &e)
    };
    let resp = agent.get(&info.app_url).set("User-Agent", &remote::ua()).call().map_err(|e| off(e.to_string()))?;
    let total: u64 = resp.header("Content-Length").and_then(|v| v.parse().ok()).filter(|n| *n > 0).unwrap_or(info.size);
    let mut r = resp.into_reader();
    let mut f = std::fs::File::create(&part).map_err(|e| off(e.to_string()))?;
    let mut data = Vec::new();
    let mut chunk = vec![0u8; 1 << 16];
    loop {
        let n = r.read(&mut chunk).map_err(|e| off(e.to_string()))?;
        if n == 0 {
            break;
        }
        f.write_all(&chunk[..n]).map_err(|e| off(e.to_string()))?;
        data.extend_from_slice(&chunk[..n]);
        if total > 0 {
            progress((data.len() as u64 * 100 / total) as u32);
        }
    }
    drop(f);
    if remote::sha256_hex(&data) != want {
        let _ = std::fs::remove_file(&part);
        return Err(SourceError::new("remote.badsum", ""));
    }
    let _ = std::fs::set_permissions(&part, std::fs::Permissions::from_mode(0o755));
    if dst.is_file() {
        let old = usercmd::real_home().join(".local/share/LabtosoStudios").join(crate::edition::ed().key).join("old").join(crate::edition::ed().version);
        let _ = std::fs::create_dir_all(&old);
        let _ = hyprstudio::util::copy2(&dst, &old.join(crate::edition::ed().local_name));
        log(&format!("{} → {}", dst.display(), old.display()));
    }
    std::fs::rename(&part, &dst).map_err(|e| SourceError::new("remote.offline", &e.to_string()))?; // atomar
    log(&format!("→ {} ({})", dst.display(), info.version));
    Ok(dst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_latest() {
        let rel: Vec<Value> = serde_json::from_str(r#"[
          {"tag_name": "assets", "assets": []},
          {"tag_name": "v3.0.1", "assets": [{"name": "Linux-AIO-3.0.1-x86_64", "browser_download_url": "a", "size": 5},
                                             {"name": "SHA256SUMS", "browser_download_url": "s"}]},
          {"tag_name": "v3.0.10", "assets": [{"name": "Linux-AIO-3.0.10-x86_64", "browser_download_url": "b"},
                                              {"name": "SDDM-Studio-1.0.4-x86_64", "browser_download_url": "c"},
                                              {"name": "SHA256SUMS", "browser_download_url": "s"}]},
          {"tag_name": "v9.0.0", "draft": true, "assets": []}]"#).unwrap();
        let i = latest(Some(rel)).unwrap().unwrap();
        assert_eq!(i.version, "3.0.10");
        assert_eq!(vtuple("2.5.3"), vec![2, 5, 3]);
        assert_eq!(short_notes("## Install\nA\n\nB", 14), "A\nB");
    }
}
