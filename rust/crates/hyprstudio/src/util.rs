//! Kleine Helfer: Prozesse starten, Programme finden, Pfade.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

pub fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"))
}

fn env_dir(var: &str, default: &str) -> PathBuf {
    match std::env::var_os(var) {
        Some(v) if !v.is_empty() => PathBuf::from(v),
        _ => home().join(default),
    }
}

pub fn xdg_config() -> PathBuf {
    env_dir("XDG_CONFIG_HOME", ".config")
}

pub fn xdg_cache() -> PathBuf {
    env_dir("XDG_CACHE_HOME", ".cache")
}

pub fn xdg_data() -> PathBuf {
    env_dir("XDG_DATA_HOME", ".local/share")
}

/// `shutil.which`
pub fn which(name: &str) -> Option<PathBuf> {
    if name.contains('/') {
        let p = PathBuf::from(name);
        return is_exec(&p).then_some(p);
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).map(|d| d.join(name)).find(|p| is_exec(p))
}

pub fn has(name: &str) -> bool {
    which(name).is_some()
}

fn is_exec(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    p.metadata().map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false)
}

/// Ergebnis eines Befehls (wie `subprocess.run(..., capture_output=True, text=True)`).
pub struct Out {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Out {
    pub fn ok(&self) -> bool {
        self.code == 0
    }
}

/// Befehl ausführen und Ausgabe sammeln; Zeitlimit in Sekunden (None = unbegrenzt).
pub fn capture(cmd: &[&str], timeout: Option<u64>) -> std::io::Result<Out> {
    capture_env(cmd, timeout, &[], None)
}

pub fn capture_env(cmd: &[&str], timeout: Option<u64>, env: &[(&str, &str)], input: Option<&str>)
    -> std::io::Result<Out> {
    let mut c = Command::new(cmd[0]);
    c.args(&cmd[1..]).stdout(Stdio::piped()).stderr(Stdio::piped());
    c.stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() });
    for (k, v) in env {
        c.env(k, v);
    }
    let mut child = c.spawn()?;
    if let Some(inp) = input {
        use std::io::Write;
        if let Some(mut si) = child.stdin.take() {
            let _ = si.write_all(inp.as_bytes());
        }
    }
    let out_h = child.stdout.take();
    let err_h = child.stderr.take();
    let t_out = std::thread::spawn(move || read_all(out_h));
    let t_err = std::thread::spawn(move || read_all(err_h));
    let status = match timeout {
        None => child.wait()?,
        Some(secs) => {
            let start = std::time::Instant::now();
            loop {
                if let Some(s) = child.try_wait()? {
                    break s;
                }
                if start.elapsed() > Duration::from_secs(secs) {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(std::io::Error::new(std::io::ErrorKind::TimedOut,
                        format!("Command '{:?}' timed out after {} seconds", cmd, secs)));
                }
                std::thread::sleep(Duration::from_millis(15));
            }
        }
    };
    Ok(Out {
        code: status.code().unwrap_or(-1),
        stdout: t_out.join().unwrap_or_default(),
        stderr: t_err.join().unwrap_or_default(),
    })
}

fn read_all<R: std::io::Read>(r: Option<R>) -> String {
    let mut buf = Vec::new();
    if let Some(mut r) = r {
        let _ = r.read_to_end(&mut buf);
    }
    String::from_utf8_lossy(&buf).into_owned()
}

/// Ausgabe eines Befehls oder "" (Fehler werden verschluckt).
pub fn output(cmd: &[&str], timeout: u64) -> String {
    capture(cmd, Some(timeout)).map(|o| o.stdout).unwrap_or_default()
}

/// Programm im Hintergrund starten (eigene Sitzung, keine Ausgabe), wie `Popen(..., start_new_session=True)`.
pub fn spawn_detached(cmd: &[&str]) -> std::io::Result<()> {
    use std::os::unix::process::CommandExt;
    let mut c = Command::new(cmd[0]);
    c.args(&cmd[1..]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    c.process_group(0);
    for k in ["DESKTOP_STARTUP_ID", "XDG_ACTIVATION_TOKEN"] {
        c.env_remove(k);
    }
    let mut child = c.spawn()?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

pub fn read_text(p: &Path) -> String {
    std::fs::read(p).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default()
}

pub fn read_opt(p: &Path) -> Option<String> {
    std::fs::read(p).ok().map(|b| String::from_utf8_lossy(&b).into_owned())
}

pub fn write_text(p: &Path, s: &str) -> std::io::Result<()> {
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d)?;
    }
    std::fs::write(p, s)
}

/// `shutil.copy2`: Inhalt plus Zeitstempel und Rechte.
pub fn copy2(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::copy(src, dst)?;
    if let Ok(m) = src.metadata() {
        if let (Ok(a), Ok(mo)) = (m.accessed(), m.modified()) {
            let f = std::fs::File::options().write(true).open(dst)?;
            let _ = f.set_times(std::fs::FileTimes::new().set_accessed(a).set_modified(mo));
        }
    }
    Ok(())
}

/// Lokale Uhrzeit wie `time.strftime(fmt)` (über glib).
pub fn strftime(fmt: &str) -> String {
    glib::DateTime::now_local().ok().and_then(|d| d.format(fmt).ok()).map(|s| s.to_string()).unwrap_or_default()
}

pub fn file_name(p: &Path) -> String {
    p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

/// `Path.with_name`
pub fn with_name(p: &Path, name: &str) -> PathBuf {
    p.with_file_name(name)
}

/// Pfad mit angehängtem Text am Dateinamen (`target.name + ".bak"`).
pub fn add_suffix(p: &Path, suffix: &str) -> PathBuf {
    p.with_file_name(format!("{}{}", file_name(p), suffix))
}

/// Text wie Pythons `str.splitlines()` (ohne Zeilenende).
pub fn splitlines(s: &str) -> Vec<&str> {
    s.lines().collect()
}

/// `str.splitlines(keepends=True)`
pub fn splitlines_keep(s: &str) -> Vec<String> {
    s.split_inclusive('\n').map(String::from).collect()
}

/// Shell-Quoting wie `shlex.quote`.
pub fn shquote(s: &str) -> String {
    if !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || "@%+=:,./-_".contains(c)) {
        return s.to_string();
    }
    format!("'{}'", s.replace('\'', "'\"'\"'"))
}

/// `shlex.split` (einfach: Anführungszeichen und Backslash).
pub fn shsplit(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut have = false;
    let mut chars = s.chars().peekable();
    let mut q: Option<char> = None;
    while let Some(c) = chars.next() {
        match q {
            Some('\'') => {
                if c == '\'' { q = None } else { cur.push(c) }
            }
            Some(_) => {
                if c == '"' {
                    q = None
                } else if c == '\\' {
                    if let Some(&n) = chars.peek() {
                        if "\"\\$`\n".contains(n) {
                            chars.next();
                            cur.push(n);
                            continue;
                        }
                    }
                    cur.push(c)
                } else {
                    cur.push(c)
                }
            }
            None => {
                if c.is_whitespace() {
                    if have {
                        out.push(std::mem::take(&mut cur));
                        have = false;
                    }
                } else if c == '\'' || c == '"' {
                    q = Some(c);
                    have = true;
                } else if c == '\\' {
                    if let Some(n) = chars.next() {
                        cur.push(n);
                    }
                    have = true;
                } else {
                    cur.push(c);
                    have = true;
                }
            }
        }
    }
    if have {
        out.push(cur);
    }
    out
}

/// `os.path.expanduser`
pub fn expanduser(s: &str) -> PathBuf {
    if s == "~" {
        return home();
    }
    if let Some(r) = s.strip_prefix("~/") {
        return home().join(r);
    }
    PathBuf::from(s)
}

/// Einmal übersetzter regulärer Ausdruck: `re!(r"...")`.
#[macro_export]
macro_rules! re {
    ($p:expr) => {{
        static R: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| regex::Regex::new($p).unwrap());
        &*R
    }};
}

/// Wie `re!`, aber mit Rückbezügen/Lookahead (fancy-regex).
#[macro_export]
macro_rules! fre {
    ($p:expr) => {{
        static R: once_cell::sync::Lazy<fancy_regex::Regex> =
            once_cell::sync::Lazy::new(|| fancy_regex::Regex::new($p).unwrap());
        &*R
    }};
}

/// `str.isdigit()` für ASCII-Ziffern (nicht leer).
pub fn isdigit(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

/// Python-`str.ljust`
pub fn ljust(s: &str, w: usize) -> String {
    let n = s.chars().count();
    if n >= w { s.to_string() } else { format!("{}{}", s, " ".repeat(w - n)) }
}
