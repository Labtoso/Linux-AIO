//! Downloads (wie urllib in der Python-Fassung): eigener User-Agent, Zeitlimit.

use std::io::Read;
use std::time::Duration;

/// Wie urllib: Proxy aus der Umgebung, Zertifikate des Systems.
pub fn builder() -> ureq::AgentBuilder {
    ureq::AgentBuilder::new().try_proxy_from_env(true)
}

pub fn agent(timeout: u64) -> ureq::Agent {
    builder().timeout(Duration::from_secs(timeout)).build()
}

pub fn user_agent() -> String {
    format!("HyprStudio/{}", crate::VERSION)
}

/// Lädt eine Datei komplett (Fehlertext wie „HTTP Error 404: Not Found“ bzw. Netzfehler).
pub fn get_bytes(url: &str, timeout: u64, ua: &str) -> Result<Vec<u8>, String> {
    let resp = agent(timeout).get(url).set("User-Agent", ua).call().map_err(err_text)?;
    let mut buf = Vec::new();
    resp.into_reader().read_to_end(&mut buf).map_err(|e| e.to_string())?;
    Ok(buf)
}

pub fn get_text(url: &str, timeout: u64, ua: &str) -> Result<String, String> {
    get_bytes(url, timeout, ua).map(|b| String::from_utf8_lossy(&b).into_owned())
}

pub fn get_json(url: &str, timeout: u64, ua: &str) -> Result<serde_json::Value, String> {
    let t = get_text(url, timeout, ua)?;
    serde_json::from_str(&t).map_err(|e| e.to_string())
}

/// Lädt mit Fortschritt (geladen, gesamt) in Blöcken.
pub fn get_progress(url: &str, timeout: u64, ua: &str, mut progress: impl FnMut(u64, u64)) -> Result<Vec<u8>, String> {
    let resp = agent(timeout).get(url).set("User-Agent", ua).call().map_err(err_text)?;
    let total: u64 = resp.header("Content-Length").and_then(|v| v.parse().ok()).unwrap_or(0);
    let mut r = resp.into_reader();
    let mut buf = Vec::new();
    let mut chunk = vec![0u8; 1 << 16];
    loop {
        let n = r.read(&mut chunk).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        progress(buf.len() as u64, total);
    }
    Ok(buf)
}

pub fn err_text(e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(code, r) => format!("HTTP Error {}: {}", code, r.status_text()),
        ureq::Error::Transport(t) => format!("<urlopen error {}>", t),
    }
}

/// Offline-Fehler? (für verständliche Meldungen)
pub fn is_offline(err: &str) -> bool {
    err.starts_with("<urlopen error")
}
