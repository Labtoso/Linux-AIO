//! Befehle ohne Root (gsettings, plasma-apply-*, hyprctl …) und Dateien im Home.
//!
//! Im Testmodus (DEMANAGER_DRYRUN=1) wird nichts ausgeführt und nichts im echten Home geschrieben:
//! Befehle landen nur im Protokoll, gsettings-Werte in einer Testmodus-Datei, Dateien unter dem Testmodus-Ordner.

use hyprstudio::{jsonx, util};
use once_cell::sync::Lazy;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub static DRY: Lazy<bool> = Lazy::new(|| {
    let d = std::env::var("DEMANAGER_DRYRUN").as_deref() == Ok("1");
    if d {
        std::env::set_var("DEMANAGER_DRYRUN", "1");
        std::env::set_var("HYPRSTUDIO_DRYRUN", "1"); // eingebautes Hypr Studio schreibt dann auch nur in den Testordner
    }
    d
});

pub fn dry() -> bool {
    *DRY
}

pub fn cache() -> PathBuf {
    util::xdg_cache().join("LabtosoStudios").join("linuxaio")
}

pub fn dry_home() -> PathBuf {
    cache().join("testmodus").join("home")
}

pub fn dry_gset() -> PathBuf {
    cache().join("testmodus").join("gsettings.json")
}

type Logger = Box<dyn Fn(&str) + Send + Sync>;
static LOG: Lazy<Mutex<Option<Logger>>> = Lazy::new(|| Mutex::new(None));

pub fn set_logger(f: impl Fn(&str) + Send + Sync + 'static) {
    *LOG.lock().unwrap() = Some(Box::new(f));
}

pub fn log(text: &str) {
    match LOG.lock().unwrap().as_ref() {
        Some(f) => f(text),
        None => println!("{}", text),
    }
}

/// Echtes Home bzw. im Testmodus ein eigener Ordner (für alles, was geschrieben wird).
pub fn home() -> PathBuf {
    if dry() { dry_home() } else { util::home() }
}

pub fn real_home() -> PathBuf {
    util::home()
}

pub fn config_dir() -> PathBuf {
    if dry() { dry_home().join(".config") } else { util::xdg_config() }
}

pub fn data_dir() -> PathBuf {
    if dry() { dry_home().join(".local").join("share") } else { util::xdg_data() }
}

/// Zum Lesen: im Testmodus die Testdatei, solange es sie nicht gibt die echte.
pub fn readable(path: &Path) -> PathBuf {
    if dry() && !path.exists() {
        if let Ok(rel) = path.strip_prefix(dry_home()) {
            return util::home().join(rel);
        }
    }
    path.to_path_buf()
}

pub fn have(tool: &str) -> bool {
    util::has(tool)
}

fn env_clean() -> Vec<(String, String)> {
    std::env::vars().filter(|(k, _)| k != "DESKTOP_STARTUP_ID" && k != "XDG_ACTIVATION_TOKEN").collect()
}

fn exec(cmd: &[String], timeout: u64) -> std::io::Result<util::Out> {
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    if cmd.is_empty() {
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "empty"));
    }
    let mut c = Command::new(&cmd[0]);
    c.args(&cmd[1..]).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    c.env_clear().envs(env_clean());
    let mut child = c.spawn()?;
    let mut out_h = child.stdout.take();
    let mut err_h = child.stderr.take();
    let t1 = std::thread::spawn(move || {
        let mut s = Vec::new();
        if let Some(h) = out_h.as_mut() {
            let _ = std::io::Read::read_to_end(h, &mut s);
        }
        String::from_utf8_lossy(&s).into_owned()
    });
    let t2 = std::thread::spawn(move || {
        let mut s = Vec::new();
        if let Some(h) = err_h.as_mut() {
            let _ = std::io::Read::read_to_end(h, &mut s);
        }
        String::from_utf8_lossy(&s).into_owned()
    });
    let start = Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait()? {
            break s;
        }
        if start.elapsed() > Duration::from_secs(timeout) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(std::io::Error::new(std::io::ErrorKind::TimedOut,
                format!("Command '{:?}' timed out after {} seconds", cmd, timeout)));
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    Ok(util::Out { code: status.code().unwrap_or(-1), stdout: t1.join().unwrap_or_default(),
                   stderr: t2.join().unwrap_or_default() })
}

pub fn sv(cmd: &[&str]) -> Vec<String> {
    cmd.iter().map(|s| s.to_string()).collect()
}

/// Befehl ausführen (Testmodus: nur protokollieren). Gibt (ok, Ausgabe) zurück.
pub fn run(cmd: &[&str]) -> (bool, String) {
    run_t(&sv(cmd), 20)
}

pub fn run_v(cmd: &[String]) -> (bool, String) {
    run_t(cmd, 20)
}

pub fn run_t(cmd: &[String], timeout: u64) -> (bool, String) {
    let line: Vec<String> = cmd.iter().map(|c| q(c)).collect();
    log(&format!("$ {}{}", line.join(" "), if dry() { "   [Testmodus]" } else { "" }));
    if dry() {
        return (true, String::new());
    }
    match exec(cmd, timeout) {
        Err(e) => {
            let s = if e.kind() == std::io::ErrorKind::NotFound {
                format!("[Errno 2] No such file or directory: '{}'", cmd.first().cloned().unwrap_or_default())
            } else {
                e.to_string()
            };
            log(&s);
            (false, s)
        }
        Ok(r) => {
            let out = format!("{}{}", r.stdout, r.stderr).trim().to_string();
            if !out.is_empty() {
                log(&out);
            }
            (r.code == 0, out)
        }
    }
}

/// Nur lesen (läuft auch im Testmodus).
pub fn query(cmd: &[&str]) -> Option<String> {
    query_t(&sv(cmd), 8)
}

pub fn query_t(cmd: &[String], timeout: u64) -> Option<String> {
    let r = exec(cmd, timeout).ok()?;
    (r.code == 0).then(|| r.stdout.trim().to_string())
}

/// Wie `_q`: Argument für das Protokoll quoten.
pub fn q(s: &str) -> String {
    if !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || "-_./:=@,+".contains(c)) {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

// ---------- gsettings ----------

static SCHEMAS: Lazy<Mutex<Option<HashSet<String>>>> = Lazy::new(|| Mutex::new(None));
static KEYS: Lazy<Mutex<HashMap<String, HashSet<String>>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub fn schemas() -> HashSet<String> {
    let mut g = SCHEMAS.lock().unwrap();
    if g.is_none() {
        let out = if have("gsettings") { query(&["gsettings", "list-schemas"]) } else { None };
        *g = Some(out.map(|o| o.split_whitespace().map(String::from).collect()).unwrap_or_default());
    }
    g.clone().unwrap()
}

pub fn has_schema(schema: &str) -> bool {
    schemas().contains(schema)
}

pub fn has_key(schema: &str, key: &str) -> bool {
    if !has_schema(schema) {
        return false;
    }
    let mut k = KEYS.lock().unwrap();
    if !k.contains_key(schema) {
        let out = query(&["gsettings", "list-keys", schema]);
        k.insert(schema.into(), out.map(|o| o.split_whitespace().map(String::from).collect()).unwrap_or_default());
    }
    k[schema].contains(key)
}

pub fn dry_values() -> Map<String, Value> {
    std::fs::read_to_string(dry_gset()).ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

fn write_dry(vals: &Map<String, Value>, mkdir: bool) {
    let p = dry_gset();
    if mkdir {
        if let Some(d) = p.parent() {
            let _ = std::fs::create_dir_all(d);
        }
    }
    let _ = std::fs::write(&p, jsonx::dumps_indent(&Value::Object(vals.clone()), 1));
}

/// gsettings-Ausgabe → Wert: 'abc' → abc, true → true, uint32 4 → 4, @as [] → [].
pub fn parse_value(text: &str) -> Value {
    let mut t = text.trim();
    for pre in ["uint32 ", "int32 ", "int64 ", "uint64 ", "double ", "@as ", "@a{sv} "] {
        if let Some(r) = t.strip_prefix(pre) {
            t = r;
        }
    }
    if t == "true" || t == "false" {
        return Value::Bool(t == "true");
    }
    let mut p = Lit { s: t.as_bytes(), i: 0, src: t };
    match p.value() {
        Some(v) => {
            p.ws();
            if p.i == p.s.len() { v } else { Value::String(t.into()) }
        }
        None => Value::String(t.into()),
    }
}

/// Kleiner Parser für Python-Literale (wie `ast.literal_eval` für die Ausgaben von gsettings).
struct Lit<'a> {
    s: &'a [u8],
    i: usize,
    src: &'a str,
}

impl Lit<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && (self.s[self.i] as char).is_ascii_whitespace() {
            self.i += 1;
        }
    }
    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }
    fn value(&mut self) -> Option<Value> {
        self.ws();
        let mut v = self.atom()?;
        // Python verbindet nebeneinanderstehende Strings
        loop {
            let save = self.i;
            self.ws();
            if matches!(self.peek(), Some(b'\'') | Some(b'"')) && v.is_string() {
                if let Some(Value::String(b)) = self.atom() {
                    v = Value::String(format!("{}{}", v.as_str().unwrap(), b));
                    continue;
                }
                return None;
            }
            self.i = save;
            return Some(v);
        }
    }
    fn atom(&mut self) -> Option<Value> {
        match self.peek()? {
            b'\'' | b'"' => self.string(),
            b'[' => self.seq(b']'),
            b'(' => self.seq(b')'),
            b'{' => self.dict(),
            _ => self.number(),
        }
    }
    fn seq(&mut self, close: u8) -> Option<Value> {
        self.i += 1;
        let mut out = Vec::new();
        let mut had_comma = false;
        loop {
            self.ws();
            if self.peek()? == close {
                self.i += 1;
                if close == b')' && out.len() == 1 && !had_comma {
                    return Some(out.pop().unwrap()); // (x) ist kein Tupel
                }
                return Some(Value::Array(out));
            }
            out.push(self.value()?);
            self.ws();
            match self.peek()? {
                b',' => {
                    self.i += 1;
                    had_comma = true;
                }
                c if c == close => {}
                _ => return None,
            }
        }
    }
    fn dict(&mut self) -> Option<Value> {
        self.i += 1;
        let mut out = Map::new();
        loop {
            self.ws();
            if self.peek()? == b'}' {
                self.i += 1;
                return Some(Value::Object(out));
            }
            let k = self.value()?;
            self.ws();
            if self.peek()? != b':' {
                return None;
            }
            self.i += 1;
            let v = self.value()?;
            let key = match k {
                Value::String(s) => s,
                other => jsonx::py_str(&other),
            };
            out.insert(key, v);
            self.ws();
            match self.peek()? {
                b',' => self.i += 1,
                b'}' => {}
                _ => return None,
            }
        }
    }
    fn string(&mut self) -> Option<Value> {
        let qch = self.s[self.i];
        self.i += 1;
        let mut out = String::new();
        let rest = &self.src[self.i..];
        let mut it = rest.char_indices();
        while let Some((j, c)) = it.next() {
            if c as u32 == qch as u32 {
                self.i += j + 1;
                return Some(Value::String(out));
            }
            if c == '\\' {
                let (_, e) = it.next()?;
                match e {
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    'r' => out.push('\r'),
                    '0' => out.push('\0'),
                    '\\' => out.push('\\'),
                    '\'' => out.push('\''),
                    '"' => out.push('"'),
                    'x' => {
                        let h: String = (0..2).filter_map(|_| it.next().map(|x| x.1)).collect();
                        out.push(char::from_u32(u32::from_str_radix(&h, 16).ok()?)?);
                    }
                    'u' => {
                        let h: String = (0..4).filter_map(|_| it.next().map(|x| x.1)).collect();
                        out.push(char::from_u32(u32::from_str_radix(&h, 16).ok()?)?);
                    }
                    other => {
                        out.push('\\');
                        out.push(other);
                    }
                }
            } else if c == '\n' {
                return None;
            } else {
                out.push(c);
            }
        }
        None
    }
    fn number(&mut self) -> Option<Value> {
        let st = self.i;
        while self.i < self.s.len() && (self.s[self.i].is_ascii_alphanumeric() || b"+-._".contains(&self.s[self.i])) {
            self.i += 1;
        }
        let w = self.src[st..self.i].replace('_', "");
        match w.as_str() {
            "True" => return Some(Value::Bool(true)),
            "False" => return Some(Value::Bool(false)),
            "None" => return Some(Value::Null),
            _ => {}
        }
        if let Ok(i) = w.parse::<i64>() {
            return Some(Value::from(i));
        }
        if w.chars().any(|c| c.is_ascii_digit()) && !w.contains("inf") && !w.contains("nan") {
            if let Ok(f) = w.parse::<f64>() {
                return Some(jsonx::f(f));
            }
        }
        None
    }
}

/// Wert → GVariant-Text für gsettings set.
pub fn gvariant(value: &Value) -> String {
    match value {
        Value::Bool(b) => if *b { "true".into() } else { "false".into() },
        Value::Number(n) => {
            if jsonx::is_float(value) {
                jsonx::py_float(n.as_f64().unwrap_or(0.0))
            } else {
                n.to_string()
            }
        }
        Value::Array(a) => format!("[{}]", a.iter().map(gvariant).collect::<Vec<_>>().join(", ")),
        Value::String(s) => format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'")),
        other => {
            let s = jsonx::py_str(other);
            format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'"))
        }
    }
}

pub fn gget(schema: &str, key: &str, default: Value) -> Value {
    if dry() {
        if let Some(v) = dry_values().get(&format!("{} {}", schema, key)) {
            if !v.is_null() {
                return v.clone();
            }
        }
    }
    if !has_key(schema, key) {
        return default;
    }
    match query(&["gsettings", "get", schema, key]) {
        None => default,
        Some(o) => parse_value(&o),
    }
}

pub fn gget_str(schema: &str, key: &str) -> String {
    match gget(schema, key, Value::Null) {
        Value::String(s) => s,
        Value::Null => String::new(),
        other => jsonx::py_str(&other),
    }
}

pub fn gset(schema: &str, key: &str, value: Value) -> bool {
    if dry() {
        let mut vals = dry_values();
        vals.insert(format!("{} {}", schema, key), value.clone());
        write_dry(&vals, true);
    }
    if !dry() && !has_key(schema, key) {
        log(&format!("gsettings: {} {} fehlt", schema, key));
        return false;
    }
    run_v(&sv(&["gsettings", "set", schema, key, &gvariant(&value)])).0
}

/// Zurück auf den Standardwert (gsettings reset).
pub fn greset(schema: &str, key: &str) -> bool {
    if dry() {
        let mut vals = dry_values();
        vals.shift_remove(&format!("{} {}", schema, key));
        write_dry(&vals, false);
    }
    run(&["gsettings", "reset", schema, key]).0
}

/// ~/.config/LabtosoStudios/linuxaio/backups/<Datum-Zeit>/ (Testmodus: unter dem Testordner).
pub fn backup_dir() -> PathBuf {
    let d = config_dir().join("LabtosoStudios").join("linuxaio").join("backups")
        .join(util::strftime("%Y-%m-%d_%H-%M-%S"));
    let _ = std::fs::create_dir_all(&d);
    d
}

// ---------- ini-Dateien (ohne configparser, Reihenfolge und Kommentare bleiben) ----------

pub fn ini_set(path: &Path, section: &str, key: &str, value: &str) {
    let text = if path.is_file() { util::read_text(path) } else { String::new() };
    let lines: Vec<&str> = util::splitlines(&text);
    let mut out: Vec<String> = Vec::new();
    let (mut sec, mut done, mut found_sec): (Option<String>, bool, bool) = (None, false, false);
    for ln in lines {
        let s = ln.trim();
        if s.starts_with('[') && s.ends_with(']') && s.len() >= 2 {
            if sec.as_deref() == Some(section) && !done {
                out.push(format!("{}={}", key, value));
                done = true;
            }
            sec = Some(s[1..s.len() - 1].to_string());
            found_sec = found_sec || sec.as_deref() == Some(section);
        } else if sec.as_deref() == Some(section) && !done && s.split('=').next().unwrap_or("").trim() == key {
            out.push(format!("{}={}", key, value));
            done = true;
            continue;
        }
        out.push(ln.to_string());
    }
    if !done {
        if !found_sec {
            if out.last().map(|l| !l.trim().is_empty()).unwrap_or(false) {
                out.push(String::new());
            }
            out.push(format!("[{}]", section));
        }
        out.push(format!("{}={}", key, value));
    }
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::write(path, out.join("\n") + "\n");
    log(&format!("→ {}: [{}] {}={}", path.display(), section, key, value));
}

pub fn ini_get(path: &Path, section: &str, key: &str, default: &str) -> String {
    let mut sec: Option<String> = None;
    if let Ok(b) = std::fs::read(path) {
        let text = String::from_utf8_lossy(&b).into_owned();
        for ln in util::splitlines(&text) {
            let s = ln.trim();
            if s.starts_with('[') && s.ends_with(']') && s.len() >= 2 {
                sec = Some(s[1..s.len() - 1].to_string());
            } else if sec.as_deref() == Some(section) && s.contains('=')
                && s.split('=').next().unwrap().trim() == key {
                return s.split_once('=').unwrap().1.trim().to_string();
            }
        }
    }
    default.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        assert_eq!(parse_value("'abc'"), Value::String("abc".into()));
        assert_eq!(parse_value("uint32 4"), Value::from(4));
        assert_eq!(parse_value("@as []"), Value::Array(vec![]));
        assert_eq!(parse_value("['a', 'b']"), serde_json::json!(["a", "b"]));
        assert_eq!(parse_value("true"), Value::Bool(true));
        assert_eq!(parse_value("1.5"), jsonx::f(1.5));
        assert_eq!(parse_value("appmenu:close"), Value::String("appmenu:close".into()));
        assert_eq!(parse_value("[('xkb', 'de')]"), serde_json::json!([["xkb", "de"]]));
        assert_eq!(gvariant(&serde_json::json!(["a'b", 1, true])), "['a\\'b', 1, true]");
        assert_eq!(gvariant(&jsonx::f(1.0)), "1.0");
        assert_eq!(q("a b"), "'a b'");
    }

    #[test]
    fn ini() {
        let d = std::env::temp_dir().join(format!("aio-ini-{}", std::process::id()));
        let p = d.join("x.ini");
        ini_set(&p, "A", "k", "1");
        ini_set(&p, "B", "x", "2");
        ini_set(&p, "A", "k", "3");
        ini_set(&p, "A", "j", "4");
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "[A]\nk=3\n\nj=4\n[B]\nx=2\n");
        assert_eq!(ini_get(&p, "B", "x", ""), "2");
        let _ = std::fs::remove_dir_all(d);
    }
}
