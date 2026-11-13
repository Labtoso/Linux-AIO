//! Config-Fehler finden und beheben: liest `hyprctl configerrors`, ordnet jede Meldung einer Datei-Zeile zu
//! und repariert sie. Fehler in Hypr Studios eigener Datei werden über den Zustand behoben (Option weglassen
//! bzw. Funktion abschalten), damit der Fix beim nächsten Anwenden nicht wieder verschwindet.
//! Fehler in fremden Dateien werden mit Sicherung auskommentiert.

use crate::jsonx::{Get, Obj};
use crate::model::{self, Log};
use crate::util::{self, copy2, read_text};
use crate::re;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Testmodus: Fehlermeldungen kommen aus dieser Datei
pub const TESTFILE: &str = "configerrors.txt";

/// Rohe Fehlermeldungen (eine pro Eintrag).
pub fn read_errors(hdir: &Path, log: Log) -> Vec<String> {
    if model::dry() {
        let f = hdir.parent().unwrap_or(hdir).join(TESTFILE);
        if !f.is_file() {
            return vec![];
        }
        return read_text(&f).lines().filter(|l| !l.trim().is_empty()).map(String::from).collect();
    }
    let p = match util::capture(&["hyprctl", "configerrors", "-j"], Some(8)) {
        Ok(p) => p,
        Err(e) => {
            log(&format!("hyprctl: {}", e));
            return vec![];
        }
    };
    let out = p.stdout.trim().to_string();
    let items: Vec<String> = match serde_json::from_str::<Value>(&out) {
        Ok(Value::Array(a)) => a.iter().map(crate::jsonx::py_str).collect(),
        Ok(other) => vec![crate::jsonx::py_str(&other)],
        Err(_) => out.lines().map(String::from).collect(),
    };
    let mut lines = vec![];
    for it in items {
        lines.extend(it.lines().filter(|x| !x.trim().is_empty()).map(String::from));
    }
    lines.into_iter().filter(|x| !["", "no errors", "ok"].contains(&x.trim().to_lowercase().as_str())).collect()
}

#[derive(Clone, Debug, Default)]
pub struct ErrorEntry {
    pub raw: String,
    pub file: String,
    pub line: usize,
    pub msg: String,
}

pub fn parse(raw: &[String]) -> Vec<ErrorEntry> {
    let pats = [
        re!(r"(?i)(?:config error in )?file\s+(?P<file>\S+?)\s+at line\s+(?P<line>\d+):?\s*(?P<msg>.*)"),
        re!(r"(?P<file>[A-Za-z]:[/\\]\S+?\.(?:lua|conf)):(?P<line>\d+):\s*(?P<msg>.*)"),
        re!(r"(?P<file>/\S+?\.(?:lua|conf)):(?P<line>\d+):\s*(?P<msg>.*)"),
    ];
    raw.iter().map(|r| {
        let mut e = ErrorEntry { raw: r.clone(), file: String::new(), line: 0, msg: r.trim().to_string() };
        for pat in pats {
            if let Some(m) = pat.captures(r) {
                e.file = m["file"].trim_matches(|c| c == '\'' || c == '"').to_string();
                e.line = m["line"].parse().unwrap_or(0);
                let msg = m["msg"].trim();
                e.msg = if msg.is_empty() { r.clone() } else { msg.to_string() };
                break;
            }
        }
        e
    }).collect()
}

pub fn line_text(e: &ErrorEntry) -> String {
    match util::read_opt(Path::new(&e.file)) {
        Some(t) => {
            let lines: Vec<&str> = t.lines().collect();
            if e.line > 0 && e.line <= lines.len() { lines[e.line - 1].trim().to_string() } else { String::new() }
        }
        None => String::new(),
    }
}

pub fn is_ours(e: &ErrorEntry) -> bool {
    util::file_name(Path::new(&e.file)).starts_with("hyprstudio.")
}

/// Zu einer Zeile in hyprstudio.lua/.conf den Options-Pfad bestimmen, z. B. decoration.blur.xray.
pub fn option_path(text: &str, line: usize) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    if !(line > 0 && line <= lines.len()) {
        return None;
    }
    let m = re!(r"^\s*([\w.\-]+)\s*=").captures(lines[line - 1])?;
    if lines[line - 1].trim_end().ends_with('{') {
        return None;
    }
    let mut stack: Vec<String> = vec![];
    for ln in &lines[..line - 1] {
        let s = ln.trim();
        if let Some(o) = re!(r"^([\w.]+)\s*(?:=\s*)?\{$").captures(s) {
            if &o[1] != "hl.config(" {
                stack.push(o[1].to_string());
            }
        } else if s == "}" || s == "}," || s == "})" {
            stack.pop();
        } else if s == "hl.config({" {
            stack.clear();
        }
    }
    if stack.is_empty() {
        return None;
    }
    stack.push(m[1].to_string());
    Some(stack.join("."))
}

/// Zeilenanfang in der eigenen Datei → Funktion, die abgeschaltet wird
fn features() -> Vec<(Vec<&'static str>, Value)> {
    let mut ws: Vec<&'static str> = vec![
        "hl.workspace_rule({ workspace = \"1", "hl.workspace_rule({ workspace = \"2", "hl.workspace_rule({ workspace = \"3",
        "hl.workspace_rule({ workspace = \"4", "hl.workspace_rule({ workspace = \"5", "hl.workspace_rule({ workspace = \"6",
        "hl.workspace_rule({ workspace = \"7", "hl.workspace_rule({ workspace = \"8", "hl.workspace_rule({ workspace = \"9"];
    ws.extend(["workspace = 1,", "workspace = 2,", "workspace = 3,", "workspace = 4,", "workspace = 5,", "workspace = 6,",
        "workspace = 7,", "workspace = 8,", "workspace = 9,", "workspace = 10,"]);
    vec![
        (ws, json!({"workspaces_on": false})),
        (vec!["hl.gesture", "gesture ="], json!({"gesture": 0})),
        (vec!["hl.workspace_rule", "hl.window_rule", "workspace = w[", "workspace = f[", "name  = \"hs-no-gaps",
            "match = {", "border_size = 0", "rounding    = 0"], json!({"smart_gaps": false})),
        (vec!["hl.monitor", "monitor =", "output   =", "mode     =", "position =", "scale    ="],
            json!({"monitor_on": false})),
        (vec!["hl.on(\"hyprland.start\"", "hl.exec_cmd", "exec-once"],
            json!({"autostart": [], "autostart_waybar": false, "polkit_agent": false})),
        (vec!["hl.env", "env ="], json!({"cursor_theme": "", "qt_theme": ""})),
        (vec!["hl.curve(\"hs_linear\"", "leaf = \"borderangle\"", "animation = borderangle", "bezier = hs_linear"],
            json!({"border_rotate": false})),
        (vec!["hl.curve", "hl.animation", "bezier =", "animation ="], json!({"anim": false})),
    ]
}

#[derive(Clone, Debug)]
pub enum Plan {
    None,
    Comment,
    Skip(String),
    State(Obj),
    Bind(String),
}

impl Plan {
    pub fn kind(&self) -> &'static str {
        match self {
            Plan::None => "none",
            Plan::Comment => "comment",
            Plan::Skip(_) => "skip",
            Plan::State(_) => "state",
            Plan::Bind(_) => "bind",
        }
    }
}

/// Was der Fix tun würde.
pub fn plan(e: &ErrorEntry) -> Plan {
    if e.file.is_empty() || e.line == 0 {
        return Plan::None;
    }
    if !is_ours(e) {
        return Plan::Comment;
    }
    let Some(text) = util::read_opt(Path::new(&e.file)) else { return Plan::None };
    if let Some(path) = option_path(&text, e.line) {
        return Plan::Skip(path);
    }
    let lines: Vec<&str> = text.lines().collect();
    let src = if e.line <= lines.len() { lines[e.line - 1].trim() } else { "" };
    if src.starts_with("hl.bind(") || src.starts_with("bind") || src.starts_with("unbind") {
        return Plan::Bind(src.to_string());
    }
    for (prefixes, change) in features() {
        if prefixes.iter().any(|p| src.starts_with(p)) {
            return Plan::State(crate::jsonx::obj(change));
        }
    }
    Plan::Comment
}

/// Behebt einen Fehler. hs (Hyprland-Zustand) wird bei eigenen Fehlern angepasst; danach neu anwenden.
pub fn apply_fix(e: &ErrorEntry, hs: &mut Obj, log: Log) -> bool {
    match plan(e) {
        Plan::Skip(opt) => {
            let mut skip = hs.arr("skip_opts").to_vec();
            if !skip.iter().any(|x| x.as_str() == Some(&opt)) {
                skip.push(json!(opt));
            }
            hs.insert("skip_opts".into(), Value::Array(skip));
            log(&format!("Option wird nicht mehr geschrieben: {}", opt));
            true
        }
        Plan::State(change) => {
            let keys: Vec<String> = change.keys().cloned().collect();
            for (k, v) in change {
                hs.insert(k, v);
            }
            log(&format!("Abgeschaltet: {}", keys.join(", ")));
            true
        }
        Plan::Bind(src) => {
            let before = hs.arr("binds").len();
            let kept: Vec<Value> = hs.arr("binds").iter().filter(|b| {
                let Some(b) = b.as_object() else { return true };
                !(model::bind_ok(b) && (src == model::bind_lua(b) || model::bind_conf(b).contains(&src)))
            }).cloned().collect();
            let removed = kept.len() < before;
            hs.insert("binds".into(), Value::Array(kept));
            log(&if removed { format!("Kürzel entfernt: {}", src) } else { format!("Kürzel nicht gefunden: {}", src) });
            removed
        }
        Plan::Comment => {
            let f = PathBuf::from(&e.file);
            let prefix = if f.extension().map(|x| x == "lua").unwrap_or(false) {
                "-- [Hypr Studio Fix] "
            } else {
                "# [Hypr Studio Fix] "
            };
            comment_line(&f, e.line, prefix, log)
        }
        Plan::None => false,
    }
}

pub fn comment_line(f: &Path, line: usize, prefix: &str, log: Log) -> bool {
    let text = match std::fs::read(f) {
        Ok(b) => String::from_utf8_lossy(&b).into_owned(),
        Err(ex) => {
            log(&format!("{}: {}", f.display(), ex));
            return false;
        }
    };
    let mut lines = util::splitlines_keep(&text);
    if !(line > 0 && line <= lines.len()) || lines[line - 1].trim_start().starts_with(prefix.trim()) {
        return false;
    }
    let bak = model::stamp(f);
    let _ = copy2(f, &bak);
    log(&format!("Sicherung: {}", bak.display()));
    lines[line - 1] = format!("{}{}", prefix, lines[line - 1]);
    let _ = std::fs::write(f, lines.concat());
    log(&format!("Auskommentiert: {}:{}", util::file_name(f), line));
    true
}

// ---------- Sicherungen ----------

/// [(sicherung, original)] in den Ordnern, die Hypr Studio anfasst; neueste zuerst.
pub fn backups(cfg_home: &Path) -> Vec<(PathBuf, PathBuf)> {
    let r = re!(r"^(?P<orig>.+?)\.(?:hyprstudio-original|hyprstudio-\d{8}-\d{6}(?:-\d+)?|bak|vor-deinstallation)$");
    let mut out = vec![];
    for d in ["hypr", "waybar", "rofi", "wofi", "fuzzel"] {
        let folder = cfg_home.join(d);
        let Ok(rd) = std::fs::read_dir(&folder) else { continue };
        for f in rd.flatten() {
            let p = f.path();
            let name = util::file_name(&p);
            if let Some(m) = r.captures(&name) {
                if p.is_file() {
                    out.push((p.clone(), folder.join(&m["orig"])));
                }
            }
        }
    }
    let mtime = |p: &Path| p.metadata().and_then(|m| m.modified()).ok();
    out.sort_by(|a, b| mtime(&b.0).cmp(&mtime(&a.0)));
    out
}

/// Sicherung zurückkopieren; die aktuelle Datei wird vorher selbst gesichert (nichts geht verloren).
pub fn restore(backup: &Path, target: &Path, log: Log) -> bool {
    if target.is_file() {
        let keep = model::stamp(target);
        let _ = copy2(target, &keep);
        log(&format!("Sicherung: {}", keep.display()));
    }
    let _ = copy2(backup, target);
    log(&format!("Wiederhergestellt: {} ← {}", target.display(), util::file_name(backup)));
    true
}
