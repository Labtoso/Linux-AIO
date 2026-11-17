//! App-Einstellungen in ~/.config/LabtosoStudios/hyprstudio.json (gleiches Format wie vorher).

use crate::jsonx::{self, Obj};
use crate::util;
use serde_json::{json, Value};
use std::path::PathBuf;

static PATH: std::sync::RwLock<Option<PathBuf>> = std::sync::RwLock::new(None);

/// Anderer Ort (Linux AIO im Testmodus: nicht im echten Home).
pub fn set_path(p: PathBuf) {
    *PATH.write().unwrap() = Some(p);
}

pub fn path() -> PathBuf {
    if let Some(p) = PATH.read().unwrap().clone() {
        return p;
    }
    util::xdg_config().join("LabtosoStudios").join("hyprstudio.json")
}

pub fn load_raw() -> Option<Obj> {
    let text = std::fs::read_to_string(path()).ok()?;
    match serde_json::from_str::<Value>(&text).ok()? {
        Value::Object(o) => Some(o),
        _ => None,
    }
}

/// neu installiert: Englisch (Deutsch im Zahnrad-Menü)
pub fn load_cfg() -> Obj {
    load_raw().unwrap_or_else(|| jsonx::obj(json!({"lang": "en", "theme": "01"})))
}

pub fn save_raw(cfg: &Obj) -> std::io::Result<()> {
    util::write_text(&path(), &jsonx::dumps(&Value::Object(cfg.clone())))
}

pub fn save_cfg(cfg: &Obj) {
    let _ = save_raw(cfg);
}
