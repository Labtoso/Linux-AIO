//! Daten, Presets und Config-Erzeugung für Hyprland (Lua ab 0.55 oder hyprlang) und Waybar. Ohne Oberfläche.

use crate::jsonx::{self, f, Get, Obj};
use crate::util::{self, add_suffix, copy2, read_text};
use crate::{fre, re};
use once_cell::sync::Lazy;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;

pub const MARK: &str = "Hypr Studio (LabtosoStudios)";

/// Protokoll-Rückruf (Zeile für Zeile).
pub type Log<'a> = &'a dyn Fn(&str);

static DRY_FLAG: Lazy<AtomicBool> =
    Lazy::new(|| AtomicBool::new(std::env::var("HYPRSTUDIO_DRYRUN").map(|v| v == "1").unwrap_or(false)));

/// Testmodus: schreibt nur in den Cache, führt nichts aus.
pub fn dry() -> bool {
    DRY_FLAG.load(Ordering::Relaxed)
}

pub fn set_dry(on: bool) {
    DRY_FLAG.store(on, Ordering::Relaxed)
}

/// Erweiterte Einstellungen: "lua" (neu, 0.55+), "conf" (alt, hyprlang bis 0.56) oder "" = erkennen
pub static FORMAT: RwLock<String> = RwLock::new(String::new());

pub fn set_format(fmt: &str) {
    *FORMAT.write().unwrap() = fmt.to_string();
}

pub fn cache() -> PathBuf {
    util::xdg_cache().join("LabtosoStudios").join("hyprstudio")
}

pub fn config_home() -> PathBuf {
    if dry() {
        return cache().join("testmodus").join(".config");
    }
    util::xdg_config()
}

// ---------- Farben ----------

/// '#rgb', '#rrggbb' oder '#rrggbbaa' → '#rrggbbaa' (klein).
pub fn norm(c: &str) -> String {
    let mut h = c.trim().trim_start_matches('#').to_lowercase();
    if h.chars().count() == 3 {
        h = h.chars().flat_map(|ch| [ch, ch]).collect();
    }
    if h.len() == 6 {
        h.push_str("ff");
    }
    if h.len() != 8 || !h.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return "#000000ff".into();
    }
    format!("#{}", h)
}

pub fn muted_color(c: &str) -> String {
    format!("{}88", &norm(c)[..7])
}

pub fn rgba_hypr(c: &str) -> String {
    format!("rgba({})", &norm(c)[1..])
}

/// (r, g, b, a 0..255)
pub fn rgba_parts(c: &str) -> (u8, u8, u8, u8) {
    let h = norm(c);
    let p = |i: usize| u8::from_str_radix(&h[1 + i..3 + i], 16).unwrap_or(0);
    (p(0), p(2), p(4), p(6))
}

pub fn rgba_css(c: &str, alpha: Option<f64>) -> String {
    let (r, g, b, a) = rgba_parts(c);
    let a = alpha.unwrap_or(a as f64 / 255.0);
    format!("rgba({}, {}, {}, {:.2})", r, g, b, a)
}

// ---------- Hyprland ----------

static DATA_HYPR_DEFAULT: Lazy<Obj> =
    Lazy::new(|| jsonx::obj(jsonx::parse(include_str!("../data/model.HYPR_DEFAULT.json"))));
static DATA_HYPR_PRESETS: Lazy<Obj> =
    Lazy::new(|| jsonx::obj(jsonx::parse(include_str!("../data/model.HYPR_PRESETS.json"))));
static DATA_WAYBAR_DEFAULT: Lazy<Obj> =
    Lazy::new(|| jsonx::obj(jsonx::parse(include_str!("../data/model.WAYBAR_DEFAULT.json"))));
static DATA_WAYBAR_PRESETS: Lazy<Obj> =
    Lazy::new(|| jsonx::obj(jsonx::parse(include_str!("../data/model.WAYBAR_PRESETS.json"))));
static DATA_TEMPLATES: Lazy<Obj> =
    Lazy::new(|| jsonx::obj(jsonx::parse(include_str!("../data/model.TEMPLATES.json"))));
static DATA_G: Lazy<Obj> = Lazy::new(|| jsonx::obj(jsonx::parse(include_str!("../data/model.G.json"))));

pub fn hypr_default() -> &'static Obj {
    &DATA_HYPR_DEFAULT
}

/// Aussehen (wird von Presets gesetzt): alle Schlüssel vor „layout“.
pub fn look_keys() -> Vec<String> {
    DATA_HYPR_DEFAULT.keys().take_while(|k| k.as_str() != "layout").cloned().collect()
}

pub fn hypr_presets() -> &'static Obj {
    &DATA_HYPR_PRESETS
}

pub const CURVES: &[(&str, [(f64, f64); 2])] = &[
    ("smooth", [(0.23, 1.0), (0.32, 1.0)]),
    ("fast", [(0.15, 0.0), (0.1, 1.0)]),
    ("bouncy", [(0.34, 1.56), (0.64, 1.0)]),
    ("linear", [(0.0, 0.0), (1.0, 1.0)]),
];

pub fn curve(name: &str) -> [(f64, f64); 2] {
    CURVES.iter().find(|(n, _)| *n == name).map(|(_, c)| *c).unwrap_or(CURVES[0].1)
}

/// Grund-Dauer in Dezisekunden (wie im Hyprland-Beispiel)
pub const ANIMS: &[(&str, f64, Option<&str>)] = &[
    ("windowsIn", 4.1, Some("win")), ("windowsOut", 1.5, Some("win")), ("windowsMove", 4.8, None),
    ("border", 5.4, None), ("fade", 3.0, None), ("layers", 3.8, None), ("workspaces", 3.0, Some("ws")),
];
pub const WIN_STYLES: &[(&str, &str)] = &[("popin", "popin 80%"), ("slide", "slide"), ("gnomed", "gnomed")];
pub const WS_STYLES: &[(&str, &str)] =
    &[("slide", "slide"), ("slidevert", "slidevert"), ("fade", "fade"), ("slidefade", "slidefade 20%")];

fn lookup<'a>(tbl: &'a [(&'a str, &'a str)], k: &str) -> Option<&'a str> {
    tbl.iter().find(|(a, _)| *a == k).map(|(_, b)| *b)
}

pub fn hypr_state(saved: Option<&Obj>) -> Obj {
    let mut s = DATA_HYPR_DEFAULT.clone();
    for k in ["binds", "autostart", "skip_opts", "workspaces"] {
        s.insert(k.into(), json!([]));
    }
    if let Some(saved) = saved {
        for (k, v) in saved {
            let Some(cur) = s.get(k) else { continue };
            let nv = match k.as_str() {
                "binds" => Value::Array(clean_binds(v)),
                "workspaces" => Value::Array(clean_workspaces(v)),
                "autostart" | "skip_opts" => Value::Array(
                    v.as_array().map(|a| a.iter().filter(|x| x.is_string()).cloned().collect()).unwrap_or_default()),
                _ => {
                    if jsonx::same_type(v, cur) {
                        v.clone()
                    } else if jsonx::is_float(cur) && jsonx::is_int(v) {
                        f(v.as_f64().unwrap_or(0.0))
                    } else {
                        continue;
                    }
                }
            };
            s.insert(k.clone(), nv);
        }
    }
    s
}

/// Preset-Look; Verhalten (Layout, Eingabe, Kürzel …) kommt aus base.
pub fn preset_state(name: &str, base: Option<&Obj>) -> Obj {
    let mut s = hypr_state(base);
    for k in look_keys() {
        s.insert(k.clone(), DATA_HYPR_DEFAULT[&k].clone());
    }
    if let Some(p) = DATA_HYPR_PRESETS.get(name).and_then(|v| v.as_object()) {
        for (k, v) in p {
            s.insert(k.clone(), v.clone());
        }
    }
    s
}

// ---------- Arbeitsflächen ----------

pub fn ws_fields() -> Obj {
    jsonx::obj(json!({"id": 1, "name": "", "monitor": "", "gaps_in": -1, "gaps_out": -1, "border": -1,
        "persistent": false, "default": false, "no_rounding": false, "layout": "", "on_empty": ""}))
}

pub fn clean_workspaces(v: &Value) -> Vec<Value> {
    let fields = ws_fields();
    let mut out: Vec<Obj> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for w in v.as_array().map(|a| a.as_slice()).unwrap_or(&[]) {
        let Some(w) = w.as_object() else { continue };
        let mut d = fields.clone();
        for (k, default) in &fields {
            let x = w.get(k).unwrap_or(default);
            if jsonx::same_type(x, default) {
                let nv = match x.as_str() {
                    Some(s) => Value::String(s.trim().to_string()),
                    None => x.clone(),
                };
                d.insert(k.clone(), nv);
            }
        }
        let id = d.i("id");
        if (1..=10).contains(&id) && seen.insert(id) {
            out.push(d);
        }
    }
    out.sort_by_key(|x| x.i("id"));
    out.into_iter().map(Value::Object).collect()
}

pub fn default_workspaces() -> Vec<Value> {
    (1..=10).map(|i| {
        let mut d = ws_fields();
        d.insert("id".into(), json!(i));
        Value::Object(d)
    }).collect()
}

/// Hat die Arbeitsfläche irgendeine eigene Einstellung?
pub fn ws_used(w: &Obj) -> bool {
    ws_fields().iter().any(|(k, d)| k != "id" && w.val(k) != d)
}

pub fn ws_lua(w: &Obj) -> String {
    let mut parts = vec![format!("workspace = \"{}\"", w.i("id"))];
    if !w.s("name").is_empty() {
        parts.push(format!("default_name = {}", lua_str(w.s("name"))));
    }
    if !w.s("monitor").is_empty() {
        parts.push(format!("monitor = {}", lua_str(w.s("monitor"))));
    }
    if w.b("default") {
        parts.push("default = true".into());
    }
    if w.b("persistent") {
        parts.push("persistent = true".into());
    }
    for (k, lk) in [("gaps_in", "gaps_in"), ("gaps_out", "gaps_out"), ("border", "border_size")] {
        if w.i(k) >= 0 {
            parts.push(format!("{} = {}", lk, w.i(k)));
        }
    }
    if w.b("no_rounding") {
        parts.push("no_rounding = true".into());
    }
    if !w.s("layout").is_empty() {
        parts.push(format!("layout = {}", lua_str(w.s("layout"))));
    }
    if !w.s("on_empty").is_empty() {
        parts.push(format!("on_created_empty = {}", lua_str(w.s("on_empty"))));
    }
    format!("hl.workspace_rule({{ {} }})", parts.join(", "))
}

pub fn ws_conf(w: &Obj) -> String {
    let mut parts = vec![w.i("id").to_string()];
    if !w.s("name").is_empty() {
        parts.push(format!("defaultName:{}", w.s("name")));
    }
    if !w.s("monitor").is_empty() {
        parts.push(format!("monitor:{}", w.s("monitor")));
    }
    if w.b("default") {
        parts.push("default:true".into());
    }
    if w.b("persistent") {
        parts.push("persistent:true".into());
    }
    for (k, ck) in [("gaps_in", "gapsin"), ("gaps_out", "gapsout"), ("border", "bordersize")] {
        if w.i(k) >= 0 {
            parts.push(format!("{}:{}", ck, w.i(k)));
        }
    }
    if w.b("no_rounding") {
        parts.push("rounding:false".into());
    }
    if !w.s("layout").is_empty() {
        parts.push(format!("layout:{}", w.s("layout")));
    }
    if !w.s("on_empty").is_empty() {
        parts.push(format!("on-created-empty:{}", w.s("on_empty")));
    }
    format!("workspace = {}", parts.join(", "))
}

// ---------- Tastenkürzel ----------

pub const MODS: &[&str] = &["SUPER", "SHIFT", "CTRL", "ALT"];

pub fn mod_alias(m: &str) -> Option<&'static str> {
    Some(match m {
        "SUPER" | "WIN" | "MOD4" | "META" | "LOGO" => "SUPER",
        "SHIFT" => "SHIFT",
        "CTRL" | "CONTROL" => "CTRL",
        "ALT" | "MOD1" => "ALT",
        _ => return None,
    })
}

/// Aktion → Art des Arguments (None = keins)
pub const ACTIONS: &[(&str, Option<&str>)] = &[
    ("exec", Some("cmd")), ("launcher", Some("launcher")), ("close", None), ("kill", None), ("fullscreen", None),
    ("float", None), ("pseudo", None), ("togglesplit", None), ("focus", Some("dir")), ("movewin", Some("dir")),
    ("workspace", Some("ws")), ("movetows", Some("ws")), ("wsnext", None), ("wsprev", None), ("special", None),
    ("movespecial", None), ("drag", None), ("resize", None), ("exit", None),
];

pub fn action_kind(a: &str) -> Option<Option<&'static str>> {
    ACTIONS.iter().find(|(n, _)| *n == a).map(|(_, k)| *k)
}

pub const MOUSE_ACTIONS: &[&str] = &["drag", "resize"];
/// Aktion „Launcher öffnen“: Auswahl im Kürzel → Befehl
pub const LAUNCHER_CMDS: &[(&str, &str)] = &[
    ("llauncher", "l-launcher"), ("rofi", "rofi -show drun"), ("wofi", "wofi --show drun"), ("fuzzel", "fuzzel"),
    ("walker", "walker"), ("tofi", "tofi-drun --drun-launch=true"),
];

pub fn launcher_cmd(key: &str) -> &'static str {
    LAUNCHER_CMDS.iter().find(|(k, _)| *k == key).map(|(_, c)| *c).unwrap_or("")
}

/// zweites Drücken der Launcher-Taste schließt ihn (Launcher-Tab › Verhalten)
pub static LAUNCHER_TOGGLE: AtomicBool = AtomicBool::new(true);

/// Befehl fürs Kürzel; mit Umschalten: läuft der Launcher schon, wird er geschlossen.
/// Der L Launcher schaltet selbst um (merkt sich seine Prozess-ID).
pub fn launcher_exec(key: &str) -> String {
    let cmd = launcher_cmd(key);
    if LAUNCHER_TOGGLE.load(Ordering::Relaxed) && !cmd.is_empty() && key != "llauncher" {
        return format!("pkill -x {} || {}", cmd.split(' ').next().unwrap_or(""), cmd);
    }
    cmd.to_string()
}

/// „pkill -x rofi || rofi -show drun“ → „rofi -show drun“.
pub fn strip_toggle(cmd: &str) -> String {
    let c = cmd.trim();
    if c.starts_with("pkill -x ") && c.contains("||") {
        return c.split_once("||").unwrap().1.trim().to_string();
    }
    c.to_string()
}

/// Befehl, den ein Kürzel startet („Launcher öffnen“ → Befehl des gewählten Launchers).
pub fn bind_cmd(b: &Obj) -> String {
    match b.s("action") {
        "launcher" => launcher_cmd(b.s("arg")).to_string(),
        "exec" => b.s("arg").to_string(),
        _ => String::new(),
    }
}

/// Exec eines bekannten Launchers → Aktion „Launcher öffnen“ (für Import aus der Config).
pub fn as_launcher(action: &str, arg: &str) -> (String, String) {
    if action == "exec" {
        for (k, c) in LAUNCHER_CMDS {
            if strip_toggle(arg) == *c {
                return ("launcher".into(), k.to_string());
            }
        }
    }
    (action.into(), arg.into())
}

pub const FLAGS: &[&str] = &["locked", "repeating", "release"];
pub const DIRECTIONS: &[&str] = &["left", "right", "up", "down"];
pub const EXEC_SUGGEST: &[&str] = &[
    "kitty", "foot", "alacritty", "dolphin", "nautilus", "thunar", "firefox", "chromium",
    "rofi -show drun", "wofi --show drun", "fuzzel", "hyprlauncher", "hyprlock", "wlogout",
    "grim -g \"$(slurp)\" - | wl-copy", "grim - | wl-copy", "hyprpicker -a", "pavucontrol", "code",
    "cliphist list | rofi -dmenu | cliphist decode | wl-copy", "swaync-client -t",
];
pub const SPECIAL_KEYS: &[&str] = &[
    "SUPER_L", "mouse:272", "mouse:273", "mouse_down", "mouse_up", "Print", "space", "Return", "Tab", "Escape",
    "XF86AudioRaiseVolume", "XF86AudioLowerVolume", "XF86AudioMute", "XF86AudioMicMute",
    "XF86AudioPlay", "XF86AudioNext", "XF86AudioPrev", "XF86MonBrightnessUp", "XF86MonBrightnessDown",
    "XF86PowerOff", "XF86Calculator",
];

pub fn mk_bind(mods: &[&str], key: &str, action: &str, arg: &str, flags: &[&str]) -> Obj {
    jsonx::obj(json!({"mods": mods, "key": key, "action": action, "arg": arg, "flags": flags}))
}

pub fn clean_binds(v: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    let Some(a) = v.as_array() else { return out };
    for b in a {
        let Some(b) = b.as_object() else { continue };
        let act = b.s("action");
        if action_kind(act).is_none() || !b.val("action").is_string() {
            continue;
        }
        let have = |k: &str| -> Vec<String> { b.strs(k) };
        let bm = have("mods");
        let bf = have("flags");
        let mods: Vec<&str> = MODS.iter().copied().filter(|m| bm.iter().any(|x| x == m)).collect();
        let flags: Vec<&str> = FLAGS.iter().copied().filter(|m| bf.iter().any(|x| x == m)).collect();
        let key = jsonx::py_str(b.val("key"));
        let key = if jsonx::truthy(b.val("key")) { key } else { String::new() };
        let arg = if jsonx::truthy(b.val("arg")) { jsonx::py_str(b.val("arg")) } else { String::new() };
        out.push(Value::Object(mk_bind(&mods, key.trim(), act, arg.trim(), &flags)));
    }
    out
}

pub const LR: &[&str] = &["locked", "repeating"];

pub fn templates() -> &'static Obj {
    &DATA_TEMPLATES
}

pub fn default_binds() -> Vec<Value> {
    ["basics", "focus", "workspaces", "special", "mouse"].iter()
        .flat_map(|k| DATA_TEMPLATES.arr(k).to_vec()).collect()
}

pub const SUPER_ALONE: &[&str] = &["SUPER_L", "SUPER_R"];

/// Flags des Kürzels; Super allein braucht immer „release“, sonst würde jedes Super + X es auslösen.
pub fn bind_flags(b: &Obj) -> Vec<&'static str> {
    let have = b.strs("flags");
    let mut fl: Vec<&'static str> = FLAGS.iter().copied().filter(|f| have.iter().any(|x| x == f)).collect();
    if SUPER_ALONE.contains(&b.s("key")) && !fl.contains(&"release") {
        fl.push("release");
    }
    fl
}

pub type Combo = (Vec<String>, String);

pub fn combo<S: AsRef<str>>(mods: &[S], key: &str) -> Combo {
    let mut set: Vec<String> = mods.iter().map(|m| {
        let u = m.as_ref().to_uppercase();
        mod_alias(&u).map(String::from).unwrap_or(u)
    }).collect();
    set.sort();
    set.dedup();
    (set, key.trim().to_lowercase())
}

pub fn combo_text(b: &Obj) -> String {
    let mut parts = b.strs("mods");
    parts.push(b.s("key").to_string());
    parts.join(" + ")
}

pub fn bind_ok(b: &Obj) -> bool {
    let Some(kind) = action_kind(b.s("action")) else { return false };
    if b.s("key").is_empty() || (kind.is_some() && b.s("arg").is_empty()) {
        return false;
    }
    if kind == Some("dir") && !DIRECTIONS.contains(&b.s("arg")) {
        return false;
    }
    if kind == Some("launcher") && launcher_cmd(b.s("arg")).is_empty() {
        return false;
    }
    true
}

fn ws_arg(arg: &str) -> String {
    if util::isdigit(arg.trim_start_matches('-')) { arg.to_string() } else { lua_str(arg) }
}

pub fn bind_lua(b: &Obj) -> String {
    let (a, g) = (b.s("action"), b.s("arg"));
    let dsp = match a {
        "exec" => format!("hl.dsp.exec_cmd({})", lua_str(g)),
        "launcher" => format!("hl.dsp.exec_cmd({})", lua_str(&launcher_exec(g))),
        "close" => "hl.dsp.window.close()".into(),
        "kill" => "hl.dsp.window.kill()".into(),
        "fullscreen" => "hl.dsp.window.fullscreen({ action = \"toggle\" })".into(),
        "float" => "hl.dsp.window.float({ action = \"toggle\" })".into(),
        "pseudo" => "hl.dsp.window.pseudo()".into(),
        "togglesplit" => "hl.dsp.layout(\"togglesplit\")".into(),
        "focus" => format!("hl.dsp.focus({{ direction = \"{}\" }})", g),
        "movewin" => format!("hl.dsp.window.move({{ direction = \"{}\" }})", g),
        "workspace" => format!("hl.dsp.focus({{ workspace = {} }})", ws_arg(g)),
        "movetows" => format!("hl.dsp.window.move({{ workspace = {} }})", ws_arg(g)),
        "wsnext" => "hl.dsp.focus({ workspace = \"e+1\" })".into(),
        "wsprev" => "hl.dsp.focus({ workspace = \"e-1\" })".into(),
        "special" => "hl.dsp.workspace.toggle_special(\"magic\")".into(),
        "movespecial" => "hl.dsp.window.move({ workspace = \"special:magic\" })".into(),
        "drag" => "hl.dsp.window.drag()".into(),
        "resize" => "hl.dsp.window.resize()".into(),
        _ => "hl.dsp.exit()".into(),
    };
    let mut flags: Vec<&str> = bind_flags(b);
    if MOUSE_ACTIONS.contains(&a) {
        flags.push("mouse");
    }
    let extra = if flags.is_empty() {
        String::new()
    } else {
        format!(", {{ {} }}", flags.iter().map(|f| format!("{} = true", f)).collect::<Vec<_>>().join(", "))
    };
    format!("hl.bind({}, {}{})", lua_str(&combo_text(b)), dsp, extra)
}

fn conf_dsp(a: &str) -> Option<&'static str> {
    Some(match a {
        "close" => "killactive,", "kill" => "forcekillactive,", "fullscreen" => "fullscreen, 0",
        "float" => "togglefloating,", "pseudo" => "pseudo,", "togglesplit" => "togglesplit,",
        "wsnext" => "workspace, e+1", "wsprev" => "workspace, e-1", "special" => "togglespecialworkspace, magic",
        "movespecial" => "movetoworkspace, special:magic", "drag" => "movewindow", "resize" => "resizewindow",
        "exit" => "exit,",
        _ => return None,
    })
}

pub fn bind_conf(b: &Obj) -> Vec<String> {
    let (a, g) = (b.s("action"), b.s("arg"));
    let first = |s: &str| s.chars().next().map(String::from).unwrap_or_default();
    let d = match conf_dsp(a) {
        Some(d) => d.to_string(),
        None => match a {
            "exec" => format!("exec, {}", g),
            "launcher" => format!("exec, {}", launcher_exec(g)),
            "focus" => format!("movefocus, {}", first(g)),
            "movewin" => format!("movewindow, {}", first(g)),
            "workspace" => format!("workspace, {}", g),
            _ => format!("movetoworkspace, {}", g),
        },
    };
    let mods = b.strs("mods").join(" ");
    let fl = bind_flags(b);
    let letters: String = [("locked", "l"), ("repeating", "e"), ("release", "r")].iter()
        .filter(|(f, _)| fl.contains(f)).map(|(_, c)| *c).collect();
    let kind = if MOUSE_ACTIONS.contains(&a) { "bindm".to_string() } else { format!("bind{}", letters) };
    // unbind entfernt vorher gesetzte Kürzel mit derselben Taste (aus hyprland.conf)
    vec![format!("unbind = {}, {}", mods, b.s("key")), format!("{} = {}, {}, {}", kind, mods, b.s("key"), d)]
}

/// Argumente eines Lua-Aufrufs auf oberster Ebene (s beginnt nach der öffnenden Klammer).
pub fn call_args(s: &str) -> Vec<String> {
    let (mut depth, mut q, mut cur, mut out) = (0i32, None::<char>, String::new(), Vec::new());
    for ch in s.chars() {
        if let Some(qc) = q {
            cur.push(ch);
            if ch == qc {
                q = None;
            }
            continue;
        }
        if ch == '"' || ch == '\'' {
            q = Some(ch);
        } else if "({[".contains(ch) {
            depth += 1;
        } else if ")}]".contains(ch) {
            if depth == 0 {
                out.push(cur.trim().to_string());
                return out;
            }
            depth -= 1;
        } else if ch == ',' && depth == 0 {
            out.push(cur.trim().to_string());
            cur.clear();
            continue;
        }
        cur.push(ch);
    }
    out.push(cur.trim().to_string());
    out
}

fn split_args(s: &str) -> String {
    call_args(s).into_iter().next().unwrap_or_default()
}

type Locals = std::collections::HashMap<String, String>;

fn lua_concat(expr: &str, local: &Locals) -> Option<String> {
    let mut val = String::new();
    for part in expr.split("..") {
        let part = part.trim();
        let c: Vec<char> = part.chars().collect();
        if c.len() >= 2 && c[0] == c[c.len() - 1] && (c[0] == '"' || c[0] == '\'') {
            val.push_str(&part[1..part.len() - 1]);
        } else if let Some(v) = local.get(part) {
            val.push_str(v);
        } else {
            return None;
        }
    }
    Some(val)
}

fn lua_locals(text: &str) -> Locals {
    re!(r#"(?m)^\s*local\s+(\w+)\s*=\s*"([^"]*)""#).captures_iter(text)
        .map(|c| (c[1].to_string(), c[2].to_string())).collect()
}

/// [(zeilenindex, combo)] aller einfach lesbaren hl.bind-Zeilen (Variablen wie mainMod werden aufgelöst).
pub fn lua_bind_combos(text: &str) -> Vec<(usize, Combo)> {
    let local = lua_locals(text);
    let mut out = Vec::new();
    for (i, ln) in text.lines().enumerate() {
        let s = ln.trim();
        if s.starts_with("--") {
            continue;
        }
        let Some(m) = re!(r"^(?:local\s+\w+\s*=\s*)?hl\.bind\((.*)$").captures(s) else { continue };
        let Some(val) = lua_concat(&split_args(&m[1]), &local) else { continue };
        let keys: Vec<&str> = val.split('+').map(|x| x.trim()).filter(|x| !x.is_empty()).collect();
        if let Some((last, mods)) = keys.split_last() {
            out.push((i, combo(mods, last)));
        }
    }
    out
}

/// Lua-Dispatcher → (aktion, argument) oder None, wenn unbekannt.
fn parse_dsp(dsp: &str, local: &Locals) -> Option<(String, String)> {
    let d = dsp.split_whitespace().collect::<Vec<_>>().join(" ");
    if let Some(m) = re!(r"^hl\.dsp\.exec_cmd\((.*)\)$").captures(&d) {
        let mut cmd = lua_concat(&call_args(&m[1]).into_iter().next().unwrap_or_default(), local)?;
        if let Ok(Value::String(s)) = serde_json::from_str::<Value>(&format!("\"{}\"", cmd)) {
            cmd = s; // Lua-Escapes wie \" auflösen
        }
        return Some(as_launcher("exec", &cmd));
    }
    let simple = [("hl.dsp.window.close()", "close"), ("hl.dsp.window.kill()", "kill"),
        ("hl.dsp.window.pseudo()", "pseudo"), ("hl.dsp.window.drag()", "drag"), ("hl.dsp.window.resize()", "resize"),
        ("hl.dsp.exit()", "exit"), ("hl.dsp.layout(\"togglesplit\")", "togglesplit")];
    if let Some((_, a)) = simple.iter().find(|(k, _)| *k == d) {
        return Some((a.to_string(), String::new()));
    }
    if d.starts_with("hl.dsp.window.fullscreen(") {
        return Some(("fullscreen".into(), String::new()));
    }
    if d.starts_with("hl.dsp.window.float(") {
        return Some(("float".into(), String::new()));
    }
    if d.starts_with("hl.dsp.workspace.toggle_special(") {
        return Some(("special".into(), String::new()));
    }
    if let Ok(Some(m)) = fre!(r#"^hl\.dsp\.(focus|window\.move)\(\{\s*(direction|workspace)\s*=\s*("?)([^"}\s]+)\3\s*\}\)$"#)
        .captures(&d) {
        let (kind, field, val) = (&m[1], &m[2], m[4].to_string());
        if field == "direction" && DIRECTIONS.contains(&val.as_str()) {
            return Some((if kind == "focus" { "focus" } else { "movewin" }.into(), val));
        }
        if field == "workspace" {
            if (val == "e+1" || val == "e-1") && kind == "focus" {
                return Some((if val == "e+1" { "wsnext" } else { "wsprev" }.into(), String::new()));
            }
            if val == "special:magic" && kind != "focus" {
                return Some(("movespecial".into(), String::new()));
            }
            if util::isdigit(val.trim_start_matches('-')) || val.starts_with("name:") {
                return Some((if kind == "focus" { "workspace" } else { "movetows" }.into(), val));
            }
        }
    }
    None
}

fn lua_flags(tbl: Option<&str>) -> Vec<&'static str> {
    let Some(tbl) = tbl.filter(|t| !t.is_empty()) else { return vec![] };
    let on: Vec<String> = re!(r"(\w+)\s*=\s*true").captures_iter(tbl).map(|c| c[1].to_string()).collect();
    FLAGS.iter().copied().filter(|f| on.iter().any(|x| x == f)).collect()
}

/// Ersetzt `var` (als Wort), wenn danach nur Leerzeichen und `}` folgen.
fn sub_loop_var(line: &str, var: &str, val: &str) -> String {
    let pat = format!(r"\b{}\b(?=\s*[}}])", fancy_regex::escape(var));
    let r = fancy_regex::Regex::new(&pat).unwrap();
    r.replace_all(line, val).into_owned()
}

fn lua_one(line: &str, loc: &Locals) -> Option<Obj> {
    let s = line.trim();
    let m = re!(r"^(?:local\s+\w+\s*=\s*)?hl\.bind\((.*)$").captures(s)?;
    if s.starts_with("--") {
        return None;
    }
    let args = call_args(&m[1]);
    if args.len() < 2 {
        return None;
    }
    let val = lua_concat(&args[0], loc)?;
    let dsp = parse_dsp(&args[1], loc)?;
    let keys: Vec<&str> = val.split('+').map(|x| x.trim()).filter(|x| !x.is_empty()).collect();
    let (last, rest) = keys.split_last()?;
    let mods: Vec<&str> = rest.iter().filter_map(|k| mod_alias(&k.to_uppercase())).collect();
    if mods.len() != rest.len() {
        return None;
    }
    let ordered: Vec<&str> = MODS.iter().copied().filter(|m| mods.contains(m)).collect();
    Some(mk_bind(&ordered, last, &dsp.0, &dsp.1, &lua_flags(args.get(2).map(|s| s.as_str()))))
}

/// Liest hl.bind-Zeilen (auch die übliche for-Schleife für Arbeitsflächen).
/// Gibt (binds, zeilen_zum_auskommentieren) zurück; Unbekanntes bleibt unangetastet.
pub fn import_lua(text: &str) -> (Vec<Obj>, Vec<usize>) {
    let local = lua_locals(text);
    let lines: Vec<&str> = text.lines().collect();
    let mut binds = Vec::new();
    let mut idx = std::collections::BTreeSet::new();
    let mut i = 0;
    while i < lines.len() {
        let ln = lines[i];
        if let Some(lp) = re!(r"^(\s*)for\s+(\w+)\s*=\s*1\s*,\s*(\d+)\s+do\s*$").captures(ln) {
            let (ind, var) = (lp[1].to_string(), lp[2].to_string());
            let n: usize = lp[3].parse().unwrap_or(0);
            let end_re = regex::Regex::new(&format!(r"^{}end\b", regex::escape(&ind))).unwrap();
            let mut j = i + 1;
            while j < lines.len() && !end_re.is_match(lines[j]) {
                j += 1;
            }
            let body = &lines[i + 1..j.min(lines.len())];
            let mut found = Vec::new();
            let mut ok = j < lines.len();
            let km_re = regex::Regex::new(&format!(r"^\s*local\s+(\w+)\s*=\s*{}\s*%\s*10\s*(--.*)?$",
                regex::escape(&var))).unwrap();
            for k in 1..=n {
                let mut loc = local.clone();
                loc.insert(var.clone(), k.to_string());
                for bl in body {
                    if let Some(km) = km_re.captures(bl) {
                        loc.insert(km[1].to_string(), (k % 10).to_string());
                    }
                }
                for bl in body {
                    if !bl.contains("hl.bind(") {
                        continue;
                    }
                    match lua_one(&sub_loop_var(bl, &var, &k.to_string()), &loc) {
                        None => ok = false,
                        Some(b) => found.push(b),
                    }
                }
            }
            if ok && !found.is_empty() {
                binds.extend(found);
                idx.extend(i..=j);
            }
            i = j + 1;
            continue;
        }
        if let Some(b) = lua_one(ln, &local) {
            binds.push(b);
            idx.insert(i);
        }
        i += 1;
    }
    (binds, idx.into_iter().collect())
}

pub fn import_conf(text: &str) -> (Vec<Obj>, Vec<usize>) {
    let var: Locals = re!(r"(?m)^\s*\$(\w+)\s*=\s*(.*?)\s*$").captures_iter(text)
        .map(|c| (c[1].to_string(), c[2].to_string())).collect();
    let rev = |d: &str| -> Option<&'static str> {
        Some(match d {
            "killactive" => "close", "forcekillactive" => "kill", "togglefloating" => "float",
            "fullscreen" => "fullscreen", "pseudo" => "pseudo", "togglesplit" => "togglesplit", "exit" => "exit",
            "togglespecialworkspace" => "special",
            _ => return None,
        })
    };
    let dmap = |a: &str| -> Option<&'static str> {
        Some(match a { "l" => "left", "r" => "right", "u" => "up", "d" => "down", _ => return None })
    };
    let (mut binds, mut idx) = (Vec::new(), Vec::new());
    for (i, ln) in text.lines().enumerate() {
        let Some(m) = re!(r"^\s*bind([a-z]*)\s*=\s*(.*)$").captures(ln) else { continue };
        let flags = m[1].to_string();
        let rest = re!(r"\$(\w+)").replace_all(&m[2], |c: &regex::Captures| {
            var.get(&c[1]).cloned().unwrap_or_else(|| c[0].to_string())
        }).into_owned();
        let parts: Vec<String> = rest.splitn(4, ',').map(|p| p.trim().to_string()).collect();
        if parts.len() < 3 {
            continue;
        }
        let (mods_raw, key, d) = (&parts[0], &parts[1], parts[2].as_str());
        let mut arg = parts.get(3).cloned().unwrap_or_default();
        let mods: Vec<Option<&str>> = mods_raw.replace('_', " ").split_whitespace()
            .map(|x| mod_alias(&x.to_uppercase())).collect();
        if mods.contains(&None) || key.is_empty() {
            continue;
        }
        let mods: Vec<&str> = mods.into_iter().flatten().collect();
        let mut act: Option<String> = None;
        if flags == "m" {
            match d {
                "movewindow" => { act = Some("drag".into()); arg = String::new(); }
                "resizewindow" => { act = Some("resize".into()); arg = String::new(); }
                _ => { arg = String::new(); }
            }
        } else if d == "exec" && !arg.is_empty() {
            let (a, g) = as_launcher("exec", &arg);
            act = Some(a);
            arg = g;
        } else if rev(d).is_some() && (d != "togglespecialworkspace" || arg.is_empty() || arg == "magic") {
            act = rev(d).map(String::from);
            arg = String::new();
        } else if d == "layoutmsg" && arg == "togglesplit" {
            act = Some("togglesplit".into());
            arg = String::new();
        } else if (d == "movefocus" || d == "movewindow") && dmap(&arg).is_some() {
            act = Some(if d == "movefocus" { "focus" } else { "movewin" }.into());
            arg = dmap(&arg).unwrap().into();
        } else if d == "workspace" && (arg == "e+1" || arg == "e-1") {
            act = Some(if arg == "e+1" { "wsnext" } else { "wsprev" }.into());
            arg = String::new();
        } else if d == "movetoworkspace" && arg == "special:magic" {
            act = Some("movespecial".into());
            arg = String::new();
        } else if (d == "workspace" || d == "movetoworkspace") && util::isdigit(arg.trim_start_matches('-')) {
            act = Some(if d == "workspace" { "workspace" } else { "movetows" }.into());
        }
        let Some(act) = act else { continue };
        if flags.replace('m', "").chars().any(|c| !"ler".contains(c)) {
            continue;
        }
        let fl: Vec<&str> = [("locked", 'l'), ("repeating", 'e'), ("release", 'r')].iter()
            .filter(|(_, c)| flags.contains(*c)).map(|(f, _)| *f).collect();
        let ordered: Vec<&str> = MODS.iter().copied().filter(|x| mods.contains(x)).collect();
        binds.push(mk_bind(&ordered, key, &act, &arg, &fl));
        idx.push(i);
    }
    (binds, idx)
}

pub fn comment_lines(main: &Path, idx: &[usize], prefix: &str, log: Log) -> usize {
    let mut idx: Vec<usize> = idx.to_vec();
    idx.sort();
    idx.dedup();
    if idx.is_empty() {
        return 0;
    }
    let bak = stamp(main);
    if copy2(main, &bak).is_ok() {
        log(&format!("Sicherung: {}", bak.display()));
    }
    let mut lines = util::splitlines_keep(&read_text(main));
    for &i in &idx {
        if i < lines.len() {
            lines[i] = format!("{}{}", prefix, lines[i]);
            log(&format!("  auskommentiert: {}", lines[i].trim()));
        }
    }
    let _ = std::fs::write(main, lines.concat());
    idx.len()
}

fn comment_prefix(fmt: &str) -> &'static str {
    if fmt == "lua" { "-- [Hypr Studio] " } else { "# [Hypr Studio] " }
}

fn main_file(hdir: &Path) -> Option<(String, PathBuf)> {
    let fmt = hypr_format(hdir)?;
    let main = hdir.join(format!("hyprland.{}", fmt));
    main.is_file().then_some((fmt, main))
}

fn import_any(fmt: &str, text: &str) -> (Vec<Obj>, Vec<usize>) {
    if fmt == "lua" { import_lua(text) } else { import_conf(text) }
}

/// Kürzel aus hyprland.lua/.conf übernehmen und dort auskommentieren (mit Sicherung).
pub fn import_binds(hdir: &Path, log: Log) -> Vec<Obj> {
    let Some((fmt, main)) = main_file(hdir) else { return vec![] };
    let text = read_text(&main);
    let (binds, idx) = import_any(&fmt, &text);
    comment_lines(&main, &idx, comment_prefix(&fmt), log);
    binds
}

pub fn lua_conflicts(main_text: &str, binds: &[Value]) -> Vec<usize> {
    let mine: Vec<Combo> = binds.iter().filter_map(|b| b.as_object()).filter(|b| bind_ok(b))
        .map(|b| combo(&b.strs("mods"), b.s("key"))).collect();
    lua_bind_combos(main_text).into_iter().filter(|(_, c)| mine.contains(c)).map(|(i, _)| i).collect()
}

pub const LAUNCHERS: &[&str] = &["l-launcher", "rofi", "wofi", "fuzzel", "hyprlauncher", "tofi", "anyrun", "walker",
    "bemenu-run", "dmenu_run", "ulauncher", "nwg-drawer", "albert", "sherlock", "vicinae", "hyprshell"];

pub fn is_launcher_cmd(cmd: &str) -> bool {
    let cmd = strip_toggle(cmd);
    if cmd.trim().starts_with("hypr-studio --launcher") {
        return true;
    }
    let first = cmd.trim().split(' ').next().unwrap_or("");
    let name = Path::new(first).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    LAUNCHERS.contains(&name.as_str())
}

/// Kürzel, die einen Launcher starten: [(bind, "own"|"config")].
/// "config" = steht in hyprland.lua/.conf und wird nicht schon von einem eigenen Kürzel überdeckt.
pub fn launcher_binds(hdir: &Path, own: &[Value]) -> Vec<(Obj, &'static str)> {
    let own: Vec<&Obj> = own.iter().filter_map(|b| b.as_object()).collect();
    let mut out: Vec<(Obj, &str)> = own.iter().filter(|b| is_launcher_cmd(&bind_cmd(b)))
        .map(|b| ((*b).clone(), "own")).collect();
    let mine: Vec<Combo> = own.iter().filter(|b| bind_ok(b)).map(|b| combo(&b.strs("mods"), b.s("key"))).collect();
    if let Some((fmt, main)) = main_file(hdir) {
        let (found, _) = import_any(&fmt, &read_text(&main));
        for b in found {
            if is_launcher_cmd(&bind_cmd(&b)) && !mine.contains(&combo(&b.strs("mods"), b.s("key"))) {
                out.push((b, "config"));
            }
        }
    }
    out
}

pub fn comment_conflicts(hdir: &Path, binds: &[Value], log: Log) -> usize {
    let main = hdir.join("hyprland.lua");
    if !main.is_file() {
        return 0;
    }
    let text = read_text(&main);
    comment_lines(&main, &lua_conflicts(&text, binds), "-- [Hypr Studio] ", log)
}

/// [(leaf, speed, style)] für die aktuelle Einstellung.
pub fn anim_list(s: &Obj) -> Vec<(&'static str, f64, Option<&'static str>)> {
    let tempo = s.n("anim_tempo").max(0.25);
    let fast = if s.s("anim_curve") == "fast" { 0.6 } else { 1.0 };
    ANIMS.iter().map(|(leaf, base, kind)| {
        let speed = round2((base * fast / tempo).max(0.5));
        let style = match *kind {
            Some("win") => lookup(WIN_STYLES, s.s("win_style")),
            Some("ws") => lookup(WS_STYLES, s.s("ws_style")),
            _ => None,
        };
        (*leaf, speed, style)
    }).collect()
}

/// Python `round(x, n)` (Banker's rounding auf der Dezimaldarstellung, hier ausreichend genau).
pub fn round_n(x: f64, n: i32) -> f64 {
    let m = 10f64.powi(n);
    let y = x * m;
    let r = y.round();
    // halbe Werte wie Python zur geraden Zahl runden
    let r = if (y - y.trunc()).abs() == 0.5 { 2.0 * (y / 2.0).round() } else { r };
    r / m
}

fn round2(x: f64) -> f64 {
    // Python round(x, 2) arbeitet auf der exakten Binärzahl: über die Textdarstellung nachbilden
    let s = format!("{:.2}", x);
    s.parse().unwrap_or(x)
}

pub fn lua_str(v: &str) -> String {
    serde_json::to_string(v).unwrap()
}

/// Zahl wie im Python-Original (`_num`) für eine Kommazahl.
pub fn num_f(v: f64) -> String {
    if v != v.trunc() {
        let s = format!("{:.4}", v);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        format!("{:.1}", v)
    }
}

/// `_num` für beliebige JSON-Werte (bool → true/false, Kommazahl, Ganzzahl).
pub fn num(v: &Value) -> String {
    match v {
        Value::Bool(b) => if *b { "true".into() } else { "false".into() },
        Value::Number(n) if n.is_f64() => num_f(n.as_f64().unwrap()),
        Value::Number(n) => n.to_string(),
        other => jsonx::py_str(other),
    }
}

/// Wert in einer erzeugten Lua/hyprlang-Tabelle.
#[derive(Clone, Debug)]
pub enum LV {
    Map(Vec<(String, LV)>),
    /// Wert, der unverändert in Lua landet (z. B. eine Verlaufs-Tabelle)
    Raw(String),
    Str(String),
    Bool(bool),
    Int(i64),
    Float(f64),
}

impl LV {
    fn lua(&self) -> String {
        match self {
            LV::Str(s) => lua_str(s),
            LV::Raw(s) => s.clone(),
            _ => self.num(),
        }
    }
    fn num(&self) -> String {
        match self {
            LV::Bool(b) => if *b { "true".into() } else { "false".into() },
            LV::Int(i) => i.to_string(),
            LV::Float(x) => num_f(*x),
            LV::Str(s) | LV::Raw(s) => s.clone(),
            LV::Map(_) => String::new(),
        }
    }
    fn from_value(v: &Value) -> LV {
        match v {
            Value::Bool(b) => LV::Bool(*b),
            Value::Number(n) if n.is_f64() => LV::Float(n.as_f64().unwrap()),
            Value::Number(n) => LV::Int(n.as_i64().unwrap_or(0)),
            Value::String(s) => LV::Str(s.clone()),
            other => LV::Str(jsonx::py_str(other)),
        }
    }
}

/// Dict → Lua-Tabellenzeilen (verschachtelt, Reihenfolge bleibt).
pub fn lua_table(d: &[(String, LV)], ind: usize) -> Vec<String> {
    let pad = "    ".repeat(ind);
    let w = d.iter().filter(|(_, v)| !matches!(v, LV::Map(_))).map(|(k, _)| k.chars().count()).max().unwrap_or(0);
    let mut out = Vec::new();
    for (k, v) in d {
        match v {
            LV::Map(m) => {
                out.push(format!("{}{} = {{", pad, k));
                out.extend(lua_table(m, ind + 1));
                out.push(format!("{}}},", pad));
            }
            _ => out.push(format!("{}{} = {},", pad, util::ljust(k, w), v.lua())),
        }
    }
    out
}

fn conf_block(name: &str, d: &[(String, LV)], ind: usize) -> Vec<String> {
    let pad = "    ".repeat(ind);
    let mut out = vec![format!("{}{} {{", pad, name)];
    for (k, v) in d {
        match v {
            LV::Map(m) => out.extend(conf_block(k, m, ind + 1)),
            _ => out.push(format!("{}    {} = {}", pad, k, v.num())),
        }
    }
    out.push(format!("{}}}", pad));
    out
}

fn drop_path(d: &mut Vec<(String, LV)>, parts: &[&str]) {
    if parts.is_empty() {
        return;
    }
    let joined = parts.join(".");
    d.retain(|(k, _)| *k != joined); // hyprlang: col.active_border steht als ein Schlüssel
    if parts.len() > 1 {
        if let Some((_, LV::Map(m))) = d.iter_mut().find(|(k, _)| k == parts[0]) {
            drop_path(m, &parts[1..]);
        }
    }
}

fn sections(s: &Obj, lua: bool) -> Vec<(String, LV)> {
    let mut sec = sections_all(s, lua);
    for path in s.strs("skip_opts") {
        let parts: Vec<&str> = path.split('.').collect();
        drop_path(&mut sec, &parts);
    }
    sec
}

fn m(items: Vec<(&str, LV)>) -> LV {
    LV::Map(items.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

/// Gemeinsamer Inhalt für Lua und hyprlang: {kategorie: {option: wert}}.
fn sections_all(s: &Obj, lua: bool) -> Vec<(String, LV)> {
    let v = |k: &str| LV::from_value(s.val(k));
    let fl = |k: &str| LV::Float(s.n(k));
    let active = if s.b("gradient") {
        if lua {
            LV::Raw(format!("{{ colors = {{{}, {}}}, angle = {} }}", lua_str(&rgba_hypr(s.s("active1"))),
                lua_str(&rgba_hypr(s.s("active2"))), s.n("angle") as i64))
        } else {
            LV::Str(format!("{} {} {}deg", rgba_hypr(s.s("active1")), rgba_hypr(s.s("active2")), s.n("angle") as i64))
        }
    } else {
        LV::Str(rgba_hypr(s.s("active1")))
    };
    let mut general = vec![("gaps_in", v("gaps_in")), ("gaps_out", v("gaps_out")), ("border_size", v("border_size"))];
    if lua {
        general.push(("col", m(vec![("active_border", active), ("inactive_border", LV::Str(rgba_hypr(s.s("inactive"))))])));
    } else {
        general.push(("col.active_border", active));
        general.push(("col.inactive_border", LV::Str(rgba_hypr(s.s("inactive")))));
    }
    general.push(("layout", v("layout")));
    general.push(("resize_on_border", v("resize_on_border")));
    let no_logo = s.b("no_logo");
    let accel = if s.s("accel") == "default" { String::new() } else { s.s("accel").to_string() };
    let out = vec![
        ("general", m(general)),
        ("decoration", m(vec![
            ("rounding", v("rounding")), ("rounding_power", fl("rounding_power")),
            ("active_opacity", fl("active_opacity")), ("inactive_opacity", fl("inactive_opacity")),
            ("dim_inactive", v("dim_inactive")), ("dim_strength", fl("dim_strength")),
            ("blur", m(vec![("enabled", v("blur")), ("size", v("blur_size")), ("passes", v("blur_passes")),
                ("vibrancy", fl("blur_vibrancy")), ("noise", fl("blur_noise")),
                ("contrast", fl("blur_contrast")), ("brightness", fl("blur_brightness")),
                ("xray", v("blur_xray")), ("ignore_opacity", v("blur_ignore_opacity"))])),
            ("shadow", m(vec![("enabled", v("shadow")), ("range", v("shadow_range")), ("render_power", v("shadow_power")),
                ("sharp", v("shadow_sharp")), ("color", LV::Str(rgba_hypr(s.s("shadow_color"))))])),
        ])),
        ("animations", m(vec![("enabled", v("anim"))])),
        ("dwindle", m(vec![("preserve_split", v("dwindle_preserve")), ("force_split", v("dwindle_force")),
            ("smart_split", v("dwindle_smart"))])),
        ("master", m(vec![("new_status", v("master_new")), ("mfact", fl("master_mfact")),
            ("orientation", v("master_orient"))])),
        ("input", m(vec![
            ("kb_layout", v("kb_layout")), ("kb_variant", v("kb_variant")), ("kb_options", v("kb_options")),
            ("numlock_by_default", v("numlock")), ("repeat_rate", v("repeat_rate")), ("repeat_delay", v("repeat_delay")),
            ("follow_mouse", v("follow_mouse")), ("sensitivity", fl("sensitivity")),
            ("accel_profile", LV::Str(accel)), ("left_handed", v("left_handed")),
            ("natural_scroll", v("mouse_natural")),
            ("touchpad", m(vec![("natural_scroll", v("natural_scroll")), ("disable_while_typing", v("dwt")),
                ("scroll_factor", fl("scroll_factor"))])),
        ])),
        ("cursor", m(vec![("inactive_timeout", fl("cursor_timeout")), ("hide_on_key_press", v("cursor_hide_keys"))])),
        ("misc", m(vec![
            ("disable_hyprland_logo", LV::Bool(no_logo)), ("disable_splash_rendering", LV::Bool(no_logo)),
            ("force_default_wallpaper", LV::Int(if no_logo { 0 } else { -1 })),
            ("background_color", LV::Str(rgba_hypr(s.s("bg_color")))), ("focus_on_activate", v("focus_on_activate")),
            ("enable_swallow", v("swallow")), ("swallow_regex", v("swallow_regex")),
            ("middle_click_paste", v("mmb_paste")), ("mouse_move_enables_dpms", v("dpms_mouse")),
            ("key_press_enables_dpms", v("dpms_key")),
        ])),
    ];
    out.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

/// [(leaf, an?, speed, kurve, stil)]
fn anim_plan(s: &Obj) -> Vec<(String, bool, f64, Option<&'static str>, Option<String>)> {
    let off = [("windows", !s.b("anim_windows")), ("fade", !s.b("anim_fade")), ("workspaces", !s.b("anim_ws")),
        ("border", !s.b("anim_border")), ("layers", !s.b("anim_layers"))];
    let mut out = Vec::new();
    for (leaf, sp, st) in anim_list(s) {
        let root = off.iter().find(|(r, _)| leaf.starts_with(r)).unwrap();
        if !root.1 {
            out.push((leaf.to_string(), true, sp, Some("hs_curve"), st.map(String::from)));
        }
    }
    for (root, is_off) in off {
        if is_off {
            out.push((root.to_string(), false, 1.0, None, None));
        }
    }
    if s.b("border_rotate") && s.b("anim_border") {
        out.push(("borderangle".into(), true, s.n("border_rotate_speed"), Some("hs_linear"), Some("loop".into())));
    }
    out
}

pub const WAYBAR_ONCE: &str = "pgrep -x waybar >/dev/null || waybar";

/// Startet der Befehl NUR Waybar (ggf. mit Optionen)? Ketten wie „waybar & hyprpaper“ zählen nicht.
pub fn is_waybar_cmd(cmd: &str) -> bool {
    let c = cmd.trim();
    c == WAYBAR_ONCE || re!(r"^waybar(\s+[^&;|]*)?$").is_match(c)
}

/// Waybar genau einmal: nur über den Schalter, und nur wenn noch keins läuft. Polkit-Agent ebenso über seinen Schalter.
pub fn autostart(s: &Obj) -> Vec<String> {
    let mut cmds: Vec<String> = s.strs("autostart").iter().map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty() && !is_waybar_cmd(c) && !(s.b("polkit_managed") && is_polkit_cmd(c))).collect();
    if s.b("autostart_waybar") {
        cmds.insert(0, WAYBAR_ONCE.into());
    }
    if s.b("polkit_agent") {
        cmds.insert(0, POLKIT_CMD.into());
    }
    cmds
}

pub const POLKIT_CMD: &str = "systemctl --user start hyprpolkitagent";

/// Startet der Befehl nur den Polkit-Agent (systemd-Dienst oder direkt)?
pub fn is_polkit_cmd(cmd: &str) -> bool {
    re!(r"^(systemctl\s+--user\s+(re)?start\s+hyprpolkitagent(\.service)?|\S*hyprpolkitagent)(\s*&)?$")
        .is_match(cmd.trim())
}

fn exec_cmd_of(st: &str, fmt: &str) -> Option<String> {
    if fmt == "lua" {
        fre!(r#"exec_cmd\(\s*(["'])(.*?)\1"#).captures(st).ok().flatten().map(|m| m[2].to_string())
    } else {
        re!(r"^exec(?:-once)?\s*=\s*()(.*)$").captures(st).map(|m| m[2].to_string())
    }
}

/// Zeilen in der Hauptdatei, die hyprpolkitagent selbst starten (exec_cmd bzw. exec-once).
pub fn foreign_polkit_lines(text: &str, fmt: &str) -> Vec<usize> {
    let mut out = Vec::new();
    for (i, ln) in text.lines().enumerate() {
        let st = ln.trim();
        if !st.contains("hyprpolkitagent") || st.starts_with(if fmt == "lua" { "--" } else { "#" }) {
            continue;
        }
        if let Some(c) = exec_cmd_of(st, fmt) {
            if is_polkit_cmd(&c) {
                out.push(i);
            }
        }
    }
    out
}

/// Startet die vorhandene Config den Polkit-Agent schon (Hauptdatei oder Autostart-Liste)?
pub fn detect_polkit(hdir: &Path, s: &Obj) -> bool {
    if s.strs("autostart").iter().any(|c| is_polkit_cmd(c)) {
        return true;
    }
    match main_file(hdir) {
        Some((fmt, main)) => !foreign_polkit_lines(&read_text(&main), &fmt).is_empty(),
        None => false,
    }
}

/// Schalter an: eigene Starts in hyprland.lua/.conf auskommentieren, damit der Agent nicht doppelt startet.
pub fn manage_polkit_autostart(hdir: &Path, s: &Obj, log: Log) -> usize {
    if !(s.b("polkit_managed") && s.b("polkit_agent")) {
        return 0;
    }
    let Some((fmt, main)) = main_file(hdir) else { return 0 };
    let idx = foreign_polkit_lines(&read_text(&main), &fmt);
    comment_lines(&main, &idx, comment_prefix(&fmt), log)
}

/// (zeilen_nur_waybar, zeilen_mit_waybar_in_kette) im Autostart der Hauptdatei.
pub fn foreign_waybar_lines(text: &str, fmt: &str) -> (Vec<usize>, Vec<usize>) {
    let (mut only, mut chained) = (Vec::new(), Vec::new());
    for (i, ln) in text.lines().enumerate() {
        let s = ln.trim();
        if fmt == "lua" {
            if s.starts_with("--") || !s.contains("exec_cmd") || !s.contains("waybar") {
                continue;
            }
        } else if s.starts_with('#') || !re!(r"^exec(-once)?\s*=").is_match(s) || !s.contains("waybar") {
            continue;
        }
        let cmd = exec_cmd_of(s, fmt).unwrap_or_default();
        let cmd = cmd.trim();
        if !re!(r"\bwaybar\b").is_match(cmd) {
            continue;
        }
        if is_waybar_cmd(cmd) { only.push(i) } else { chained.push(i) }
    }
    (only, chained)
}

/// Hypr Studio steuert den Waybar-Autostart allein (sobald der Schalter benutzt wurde):
/// eigene Waybar-Starts in hyprland.lua/.conf auskommentieren, waybar.service (systemd) abschalten.
pub fn manage_waybar_autostart(hdir: &Path, s: &Obj, log: Log) -> usize {
    if !s.b("waybar_managed") {
        return 0;
    }
    let mut n = 0;
    if let Some((fmt, main)) = main_file(hdir) {
        let (only, chained) = foreign_waybar_lines(&read_text(&main), &fmt);
        n = comment_lines(&main, &only, comment_prefix(&fmt), log);
        let text = read_text(&main);
        let lines: Vec<&str> = text.lines().collect();
        for i in chained {
            log(&format!("Hinweis: startet Waybar zusätzlich (bitte selbst anpassen): {}:{}: {}",
                util::file_name(&main), i + 1, lines.get(i).map(|l| l.trim()).unwrap_or("")));
        }
    }
    if which("systemctl") && run(&["systemctl", "--user", "is-enabled", "--quiet", "waybar.service"], log, true) {
        run(&["systemctl", "--user", "disable", "waybar.service"], log, false);
        n += 1;
    }
    n
}

// ---------- Laptop-Tasten (Helligkeit, Lautstärke, Stumm, Mikrofon, Medien) ----------

pub const LAPTOP_KEYS: &[&str] = &["XF86MonBrightnessUp", "XF86MonBrightnessDown", "XF86AudioRaiseVolume",
    "XF86AudioLowerVolume", "XF86AudioMute", "XF86AudioMicMute", "XF86AudioPlay", "XF86AudioPause", "XF86AudioNext",
    "XF86AudioPrev"];

/// (lauter, leiser, stumm, Mikrofon stumm)
pub fn volume_cmds(tool: &str) -> [&'static str; 4] {
    match tool {
        "pactl" => ["pactl set-sink-volume @DEFAULT_SINK@ +5%", "pactl set-sink-volume @DEFAULT_SINK@ -5%",
            "pactl set-sink-mute @DEFAULT_SINK@ toggle", "pactl set-source-mute @DEFAULT_SOURCE@ toggle"],
        _ => ["wpctl set-volume -l 1 @DEFAULT_AUDIO_SINK@ 5%+", "wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-",
            "wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle", "wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle"],
    }
}

pub const BRIGHT_UP: &str = "brightnessctl -e4 -n2 set 5%+";
pub const BRIGHT_DOWN: &str = "brightnessctl -e4 -n2 set 5%-";

/// Wie im offiziellen Beispiel: gesperrt (geht auch auf dem Sperrbildschirm) und mit Wiederholung.
pub fn laptop_binds(s: &Obj) -> Vec<Obj> {
    let [up, down, mute, mic] = volume_cmds(s.s("vol_tool"));
    let lk = &["locked"][..];
    vec![mk_bind(&[], "XF86MonBrightnessUp", "exec", BRIGHT_UP, LR),
        mk_bind(&[], "XF86MonBrightnessDown", "exec", BRIGHT_DOWN, LR),
        mk_bind(&[], "XF86AudioRaiseVolume", "exec", up, LR), mk_bind(&[], "XF86AudioLowerVolume", "exec", down, LR),
        mk_bind(&[], "XF86AudioMute", "exec", mute, lk), mk_bind(&[], "XF86AudioMicMute", "exec", mic, lk),
        mk_bind(&[], "XF86AudioPlay", "exec", "playerctl play-pause", lk),
        mk_bind(&[], "XF86AudioPause", "exec", "playerctl play-pause", lk),
        mk_bind(&[], "XF86AudioNext", "exec", "playerctl next", lk),
        mk_bind(&[], "XF86AudioPrev", "exec", "playerctl previous", lk)]
}

pub fn is_laptop_bind(b: &Obj) -> bool {
    !b.b("mods") && LAPTOP_KEYS.contains(&b.s("key"))
}

/// Eigene Kürzel; Laptop-Tasten stehen in hyprstudio_laptop.* (laptopkeys) – gleiche Tasten fallen hier weg.
pub fn all_binds(s: &Obj) -> Vec<Obj> {
    let binds = s.arr("binds").iter().filter_map(|b| b.as_object().cloned());
    if !s.b("laptop_keys") {
        return binds.collect();
    }
    binds.filter(|b| !is_laptop_bind(b)).collect()
}

/// Zeilen in der Hauptdatei, die eine der Laptop-Tasten ohne Modifikator selbst belegen.
pub fn foreign_key_lines(text: &str, fmt: &str) -> Vec<usize> {
    let mut out = Vec::new();
    for (i, ln) in text.lines().enumerate() {
        let st = ln.trim();
        let m = if fmt == "lua" {
            re!(r#"^hl\.bind\(\s*"(XF86\w+)""#).captures(st)
        } else {
            re!(r"^bind\w*\s*=\s*,\s*(XF86\w+)\s*,").captures(st)
        };
        if let Some(m) = m {
            if LAPTOP_KEYS.contains(&&m[1]) {
                out.push(i);
            }
        }
    }
    out
}

/// Schalter an: gleiche Tasten in hyprland.lua/.conf auskommentieren, sonst lösen sie doppelt aus.
pub fn manage_laptop_keys(hdir: &Path, s: &Obj, log: Log) -> usize {
    if !(s.b("laptop_managed") && s.b("laptop_keys")) {
        return 0;
    }
    let Some((fmt, main)) = main_file(hdir) else { return 0 };
    let idx = foreign_key_lines(&read_text(&main), &fmt);
    comment_lines(&main, &idx, comment_prefix(&fmt), log)
}

pub const SMART_GAPS_LUA: &str = r#"
-- Keine Lücken bei nur einem Fenster
hl.workspace_rule({ workspace = "w[tv1]", gaps_out = 0, gaps_in = 0 })
hl.workspace_rule({ workspace = "f[1]",   gaps_out = 0, gaps_in = 0 })
hl.window_rule({
    name  = "hs-no-gaps-wtv1",
    match = { float = false, workspace = "w[tv1]" },
    border_size = 0,
    rounding    = 0,
})
hl.window_rule({
    name  = "hs-no-gaps-f1",
    match = { float = false, workspace = "f[1]" },
    border_size = 0,
    rounding    = 0,
})"#;

pub const L_LAUNCHER_LUA: &str = r#"
-- L Launcher (Hypr Studio): schwebt (feste Größe), eigener runder Rahmen
hl.window_rule({
    name  = "hs-l-launcher",
    match = { class = "l-launcher" },
    border_size = 0,
    rounding    = 0,
})"#;

pub fn hypr_lua(s: &Obj) -> String {
    // ein hl.config pro Bereich (wie im offiziellen Beispiel): lehnt Hyprland eine Option ab, fällt nur ihr Bereich weg
    let mut lines = vec![format!("-- {} – automatisch erzeugt, wird beim Anwenden überschrieben", MARK)];
    for (name, d) in sections(s, true) {
        let LV::Map(d) = d else { continue };
        lines.push("hl.config({".into());
        lines.push(format!("    {} = {{", name));
        lines.extend(lua_table(&d, 2));
        lines.push("    },".into());
        lines.push("})".into());
    }
    if s.b("anim") {
        let [(x0, y0), (x1, y1)] = curve(s.s("anim_curve"));
        lines.push(String::new());
        lines.push(format!("hl.curve(\"hs_curve\", {{ type = \"bezier\", points = {{ {{{}, {}}}, {{{}, {}}} }} }})",
            num_f(x0), num_f(y0), num_f(x1), num_f(y1)));
        if s.b("border_rotate") {
            lines.push("hl.curve(\"hs_linear\", { type = \"bezier\", points = { {0.0, 0.0}, {1.0, 1.0} } })".into());
        }
        for (leaf, on, speed, curve, style) in anim_plan(s) {
            if !on {
                lines.push(format!("hl.animation({{ leaf = \"{}\", enabled = false }})", leaf));
                continue;
            }
            let st = style.map(|x| format!(", style = {}", lua_str(&x))).unwrap_or_default();
            lines.push(format!("hl.animation({{ leaf = \"{}\", enabled = true, speed = {}, bezier = \"{}\"{} }})",
                leaf, num_f(speed), curve.unwrap_or(""), st));
        }
    }
    if s.b("smart_gaps") {
        lines.extend(SMART_GAPS_LUA.split('\n').map(String::from));
    }
    let wss: Vec<Obj> = if s.b("workspaces_on") {
        s.arr("workspaces").iter().filter_map(|w| w.as_object()).filter(|w| ws_used(w)).cloned().collect()
    } else {
        vec![]
    };
    if !wss.is_empty() {
        lines.push(String::new());
        lines.push("-- Arbeitsflächen".into());
        lines.extend(wss.iter().map(ws_lua));
    }
    if s.i("gesture") != 0 {
        lines.push(String::new());
        lines.push(format!("hl.gesture({{ fingers = {}, direction = \"horizontal\", action = \"workspace\" }})",
            jsonx::py_str(s.val("gesture"))));
    }
    let cs = jsonx::py_str(s.val("cursor_size"));
    lines.push(String::new());
    lines.push(format!("hl.env(\"XCURSOR_SIZE\", \"{}\")", cs));
    lines.push(format!("hl.env(\"HYPRCURSOR_SIZE\", \"{}\")", cs));
    if !s.s("cursor_theme").is_empty() {
        lines.push(format!("hl.env(\"XCURSOR_THEME\", {})", lua_str(s.s("cursor_theme"))));
        lines.push(format!("hl.env(\"HYPRCURSOR_THEME\", {})", lua_str(s.s("cursor_theme"))));
    }
    if !s.s("qt_theme").is_empty() {
        lines.push(format!("hl.env(\"QT_QPA_PLATFORMTHEME\", {})", lua_str(s.s("qt_theme"))));
    }
    if s.b("monitor_on") {
        let sc = s.s("monitor_scale");
        let sc = if sc == "auto" { lua_str(sc) } else { sc.to_string() };
        lines.extend([String::new(), "hl.monitor({".into(), "    output   = \"\",".into(),
            format!("    mode     = {},", lua_str(s.s("monitor_mode"))), "    position = \"auto\",".into(),
            format!("    scale    = {},", sc), "})".into()]);
    }
    let cmds = autostart(s);
    if !cmds.is_empty() {
        lines.push(String::new());
        lines.push("hl.on(\"hyprland.start\", function ()".into());
        lines.extend(cmds.iter().map(|c| format!("    hl.exec_cmd({})", lua_str(c))));
        lines.push("end)".into());
    }
    let binds: Vec<Obj> = all_binds(s).into_iter().filter(|b| bind_ok(b)).collect();
    if !binds.is_empty() {
        lines.push(String::new());
        lines.push("-- Tastenkürzel".into());
        lines.extend(binds.iter().map(bind_lua));
    }
    lines.extend(L_LAUNCHER_LUA.lines().map(String::from));
    lines.join("\n") + "\n"
}

pub fn hypr_conf(s: &Obj) -> String {
    let mut out = vec![format!("# {} – automatisch erzeugt, wird beim Anwenden überschrieben", MARK)];
    let mut sec = sections(s, false);
    let anim_pos = sec.iter().position(|(k, _)| k == "animations");
    let anim = anim_pos.map(|p| sec.remove(p).1);
    for (name, d) in &sec {
        if let LV::Map(d) = d {
            out.extend(conf_block(name, d, 0));
        }
    }
    let enabled = match &anim {
        Some(LV::Map(m)) => m.iter().find(|(k, _)| k == "enabled").map(|(_, v)| v.num()).unwrap_or_default(),
        _ => String::new(),
    };
    out.push("animations {".into());
    out.push(format!("    enabled = {}", enabled));
    if s.b("anim") {
        let [(x0, y0), (x1, y1)] = curve(s.s("anim_curve"));
        out.push(format!("    bezier = hs_curve, {}, {}, {}, {}", num_f(x0), num_f(y0), num_f(x1), num_f(y1)));
        if s.b("border_rotate") {
            out.push("    bezier = hs_linear, 0.0, 0.0, 1.0, 1.0".into());
        }
        for (leaf, on, speed, curve, style) in anim_plan(s) {
            if !on {
                out.push(format!("    animation = {}, 0", leaf));
            } else {
                out.push(format!("    animation = {}, 1, {}, {}{}", leaf, num_f(speed), curve.unwrap_or(""),
                    style.map(|x| format!(", {}", x)).unwrap_or_default()));
            }
        }
    }
    out.push("}".into());
    if s.b("smart_gaps") {
        out.push("workspace = w[tv1], gapsout:0, gapsin:0".into());
        out.push("workspace = f[1], gapsout:0, gapsin:0".into());
    }
    if s.b("workspaces_on") {
        out.extend(s.arr("workspaces").iter().filter_map(|w| w.as_object()).filter(|w| ws_used(w)).map(ws_conf));
    }
    if s.i("gesture") != 0 {
        out.push(format!("gesture = {}, horizontal, workspace", jsonx::py_str(s.val("gesture"))));
    }
    let cs = jsonx::py_str(s.val("cursor_size"));
    out.push(format!("env = XCURSOR_SIZE,{}", cs));
    out.push(format!("env = HYPRCURSOR_SIZE,{}", cs));
    if !s.s("cursor_theme").is_empty() {
        out.push(format!("env = XCURSOR_THEME,{}", s.s("cursor_theme")));
        out.push(format!("env = HYPRCURSOR_THEME,{}", s.s("cursor_theme")));
    }
    if !s.s("qt_theme").is_empty() {
        out.push(format!("env = QT_QPA_PLATFORMTHEME,{}", s.s("qt_theme")));
    }
    if s.b("monitor_on") {
        out.push(format!("monitor = , {}, auto, {}", s.s("monitor_mode"), s.s("monitor_scale")));
    }
    out.extend(autostart(s).iter().map(|c| format!("exec-once = {}", c)));
    let binds: Vec<Obj> = all_binds(s).into_iter().filter(|b| bind_ok(b)).collect();
    if !binds.is_empty() {
        out.push(String::new());
        out.push("# Tastenkürzel".into());
        for b in &binds {
            out.extend(bind_conf(b));
        }
    }
    out.join("\n") + "\n"
}

/// Lua hat ab 0.55 Vorrang (hyprland.lua wird statt hyprland.conf geladen); ab 0.57 gibt es nur noch Lua.
pub fn detect_format(hdir: &Path) -> Option<String> {
    if hdir.join("hyprland.lua").is_file() {
        return Some("lua".into());
    }
    if hdir.join("hyprland.conf").is_file() {
        return Some("conf".into());
    }
    None
}

/// Gewähltes Format (Erweiterte Einstellungen) oder erkanntes.
pub fn hypr_format(hdir: &Path) -> Option<String> {
    let f = FORMAT.read().unwrap().clone();
    if f == "lua" || f == "conf" { Some(f) } else { detect_format(hdir) }
}

pub fn include(fmt: &str) -> (&'static str, &'static str) {
    if fmt == "lua" { ("require(\"hyprstudio\")", "--") } else { ("source = ~/.config/hypr/hyprstudio.conf", "#") }
}

/// Genau die Einbindung von hyprstudio.* (die der Laptop-Tasten zählt nicht).
pub fn has_include(text: &str, fmt: &str) -> bool {
    let line = include(fmt).0;
    text.lines().any(|l| l.trim() == line)
}

/// Freier Sicherungsname mit Zeitstempel (nie eine vorhandene Sicherung überschreiben).
pub fn stamp(f: &Path) -> PathBuf {
    let base = format!("{}.hyprstudio-{}", util::file_name(f), util::strftime("%Y%m%d-%H%M%S"));
    let mut b = f.with_file_name(&base);
    let mut i = 1;
    while b.exists() {
        b = f.with_file_name(format!("{}-{}", base, i));
        i += 1;
    }
    b
}

/// Vorherige eigene Version als .bak behalten (für Rückgängig).
pub fn keep_previous(target: &Path) {
    if target.is_file() {
        let _ = copy2(target, &add_suffix(target, ".bak"));
    }
}

/// Fremde Datei vor dem ersten Überschreiben sichern; nie etwas löschen.
pub fn backup_foreign(target: &Path) -> Option<PathBuf> {
    if !target.is_file() {
        return None;
    }
    let head: String = read_text(target).chars().take(300).collect();
    if head.contains(MARK) {
        keep_previous(target);
        return None;
    }
    let mut dst = add_suffix(target, ".hyprstudio-original");
    if dst.exists() {
        dst = stamp(target);
    }
    copy2(target, &dst).ok()?;
    Some(dst)
}

pub fn apply_hypr(s: &Obj, hdir: &Path, log: Log) -> (bool, String) {
    let mut fmt = hypr_format(hdir);
    if dry() && !hdir.join(format!("hyprland.{}", fmt.as_deref().unwrap_or("lua"))).is_file() {
        let fm = fmt.clone().unwrap_or_else(|| "lua".into());
        let _ = std::fs::create_dir_all(hdir);
        let _ = std::fs::write(hdir.join(format!("hyprland.{}", fm)),
            format!("{} Testmodus: Platzhalter für die echte hyprland.{}\n", include(&fm).1, fm));
        fmt = Some(fm);
    }
    let Some(fmt) = fmt else { return (false, "nohypr".into()) };
    if !hdir.join(format!("hyprland.{}", fmt)).is_file() {
        return (false, format!("nomain.{}", fmt));
    }
    let target = hdir.join(format!("hyprstudio.{}", fmt));
    keep_previous(&target);
    let _ = std::fs::write(&target, if fmt == "lua" { hypr_lua(s) } else { hypr_conf(s) });
    log(&format!("→ {}", target.display()));
    let main = hdir.join(format!("hyprland.{}", fmt));
    let text = read_text(&main);
    if !has_include(&text, &fmt) {
        let bak = add_suffix(&main, ".hyprstudio-original");
        if !bak.exists() && copy2(&main, &bak).is_ok() {
            log(&format!("Sicherung: {}", bak.display()));
        }
        let (line, c) = include(&fmt);
        let sep = if text.ends_with('\n') || text.is_empty() { "" } else { "\n" };
        append(&main, &format!("{}\n{} {}\n{}\n", sep, c, MARK, line));
        log(&format!("+ {}: {}", main.display(), line));
    }
    manage_waybar_autostart(hdir, s, log);
    manage_polkit_autostart(hdir, s, log);
    manage_laptop_keys(hdir, s, log);
    run(&["hyprctl", "reload"], log, false);
    (true, fmt)
}

pub fn append(p: &Path, text: &str) {
    use std::io::Write;
    if let Ok(mut fh) = std::fs::OpenOptions::new().append(true).create(true).open(p) {
        let _ = fh.write_all(text.as_bytes());
    }
}

pub fn undo_file(target: &Path) -> bool {
    let bak = add_suffix(target, ".bak");
    if !(bak.is_file() && target.is_file()) {
        return false;
    }
    let (Ok(cur), Ok(old)) = (std::fs::read(target), std::fs::read(&bak)) else { return false };
    let _ = std::fs::write(target, old);
    let _ = std::fs::write(&bak, cur);
    true
}

pub fn undo_hypr(hdir: &Path, log: Log) -> bool {
    let Some(fmt) = hypr_format(hdir) else { return false };
    if !undo_file(&hdir.join(format!("hyprstudio.{}", fmt))) {
        return false;
    }
    run(&["hyprctl", "reload"], log, false);
    true
}

// ---------- Waybar ----------

pub fn waybar_default() -> &'static Obj {
    &DATA_WAYBAR_DEFAULT
}

pub const WB_LOOK: &[&str] = &["position", "style", "height", "margin", "radius", "spacing", "bg", "fg", "accent",
    "border_width", "border", "pad", "ws_look", "colorful", "left", "center", "right"];

pub const MODULES: &[&str] = &["custom/launcher", "hyprland/workspaces", "hyprland/window", "clock", "pulseaudio",
    "network", "battery", "backlight", "bluetooth", "cpu", "memory", "temperature", "disk", "mpris",
    "hyprland/language", "idle_inhibitor", "power-profiles-daemon", "tray", "custom/power"];

pub fn waybar_presets() -> &'static Obj {
    &DATA_WAYBAR_PRESETS
}

pub const PALETTE: &[&str] = &["#7aa2f7ff", "#9ece6aff", "#e0af68ff", "#bb9af7ff", "#7dcfffff", "#f7768eff",
    "#ff9e64ff", "#73dacaff"];

/// Nerd-Font-Symbole
pub fn glyph(k: &str) -> String {
    DATA_G.s(k).to_string()
}

pub fn glyphs(k: &str) -> Value {
    DATA_G.val(k).clone()
}

pub fn clock_fmt(k: &str) -> (&'static str, &'static str) {
    match k {
        "hms" => ("%H:%M:%S", "%I:%M:%S %p"),
        "dhm" => ("%a %H:%M", "%a %I:%M %p"),
        "full" => ("%d.%m.%Y  %H:%M", "%m/%d/%Y  %I:%M %p"),
        _ => ("%H:%M", "%I:%M %p"),
    }
}

pub fn waybar_state(saved: Option<&Obj>) -> Obj {
    let mut s = DATA_WAYBAR_DEFAULT.clone();
    if let Some(saved) = saved {
        for (k, v) in saved {
            if let Some(cur) = s.get(k) {
                if jsonx::same_type(v, cur) {
                    s.insert(k.clone(), v.clone());
                }
            }
        }
    }
    for side in ["left", "center", "right"] {
        let l: Vec<Value> = s.arr(side).iter().filter(|m| m.as_str().map(|x| MODULES.contains(&x)).unwrap_or(false))
            .cloned().collect();
        s.insert(side.into(), Value::Array(l));
    }
    s
}

/// Preset-Look; Schrift, Befehle und Modul-Optionen kommen aus base.
pub fn waybar_preset(name: &str, base: Option<&Obj>) -> Obj {
    let mut s = waybar_state(base);
    for k in WB_LOOK {
        s.insert(k.to_string(), DATA_WAYBAR_DEFAULT[*k].clone());
    }
    if let Some(p) = DATA_WAYBAR_PRESETS.get(name).and_then(|v| v.as_object()) {
        for (k, v) in p {
            s.insert(k.clone(), v.clone());
        }
    }
    s
}

pub fn module_color(name: &str, s: &Obj) -> String {
    if s.b("colorful") {
        let i = MODULES.iter().position(|m| *m == name).unwrap_or(0);
        PALETTE[i % PALETTE.len()].to_string()
    } else {
        s.s("fg").to_string()
    }
}

pub fn module_cfg(name: &str, s: &Obj) -> Value {
    let ic = s.b("icons");
    let g = |k: &str| if ic { format!("{}  ", glyph(k)) } else { String::new() };
    let gi = |k: &str, alt: &str| if ic { glyph(k) } else { alt.to_string() };
    let (c24, c12) = clock_fmt(s.s("clock_fmt"));
    let clock = if s.b("clock24") { c24 } else { c12 };
    let mut ws = json!({"format": "{name}", "on-click": "activate", "sort-by-number": true});
    if s.s("ws_format") == "dot" && ic {
        ws["format"] = json!("{icon}");
        ws["format-icons"] = json!({"active": glyph("dot"), "default": glyph("ring")});
    }
    if s.i("ws_persist") != 0 {
        ws["persistent-workspaces"] = json!({"*": s.val("ws_persist")});
    }
    let net = match s.s("net_fmt") { "ip" => "{ipaddr}", "signal" => "{signalStrength}%", _ => "{essid}" };
    let mut cfg = match name {
        "custom/launcher" => json!({"format": gi("arch", "Apps"), "on-click": s.val("launcher"), "tooltip": false,
            "justify": "center"}),
        "hyprland/workspaces" => ws,
        "hyprland/window" => json!({"max-length": s.val("win_len"), "separate-outputs": true}),
        "clock" => {
            let mut c = json!({"format": format!("{}{{:{}}}", g("clock"), clock), "format-alt": "{:%A, %d.%m.%Y}",
                "tooltip-format": "<tt>{calendar}</tt>"});
            if s.s("clock_fmt") == "hms" {
                c["interval"] = json!(1);
            }
            c
        }
        "pulseaudio" => json!({"format": if ic { "{icon}  {volume}%" } else { "Vol {volume}%" },
            "format-muted": gi("mute", "Stumm"), "format-icons": {"default": glyphs("vol")},
            "scroll-step": s.val("vol_step"), "on-click": s.val("vol_click")}),
        "network" => json!({"format-wifi": format!("{}{}", g("wifi"), net),
            "format-ethernet": format!("{}{}", g("eth"), if s.s("net_fmt") != "ip" { "LAN" } else { "{ipaddr}" }),
            "format-disconnected": gi("nonet", "Offline"), "tooltip-format": "{ifname} · {ipaddr}"}),
        "battery" => json!({"format": if ic { "{icon}  {capacity}%" } else { "Bat {capacity}%" },
            "format-icons": glyphs("bat"), "format-charging": format!("{}{{capacity}}%", g("charge")),
            "states": {"warning": s.val("bat_warn"), "critical": s.val("bat_crit")}}),
        "backlight" => json!({"format": if ic { "{icon}  {percent}%" } else { "Hell {percent}%" },
            "format-icons": glyphs("light")}),
        "bluetooth" => json!({"format": gi("bt", "BT"), "format-connected": format!("{} {{num_connections}}", gi("bt", "BT"))}),
        "cpu" => json!({"format": format!("{}{{usage}}%", or(g("cpu"), "CPU ")), "interval": s.val("interval")}),
        "memory" => json!({"format": format!("{}{{percentage}}%", or(g("mem"), "RAM ")), "interval": s.val("interval")}),
        "temperature" => json!({"format": format!("{}{{temperatureC}}°C", g("temp")), "interval": s.val("interval")}),
        "disk" => json!({"format": format!("{}{{percentage_used}}%", or(g("disk"), "Disk ")), "path": "/",
            "interval": s.i("interval") * 6}),
        "mpris" => json!({"format": format!("{}{{artist}} – {{title}}", g("play")),
            "format-paused": format!("{}{{artist}} – {{title}}", g("pause")), "max-length": s.val("mpris_len")}),
        "hyprland/language" => json!({"format": format!("{}{{short}}", g("kb"))}),
        "idle_inhibitor" => json!({"format": "{icon}", "format-icons": {"activated": gi("eye", "Wach"),
            "deactivated": gi("eyeoff", "Auto")}}),
        "power-profiles-daemon" => json!({"format": if ic { "{icon}" } else { "{profile}" },
            "format-icons": {"default": glyph("perf"), "performance": glyph("perf"), "balanced": glyph("bal"),
            "power-saver": glyph("save")}}),
        "tray" => json!({"spacing": 8, "icon-size": s.val("tray_size")}),
        _ => json!({"format": gi("power", "Power"), "on-click": s.val("power_cmd"), "tooltip": false, "justify": "center"}),
    };
    if !s.b("tooltips") {
        cfg["tooltip"] = json!(false);
    }
    cfg
}

fn or(a: String, b: &str) -> String {
    if a.is_empty() { b.to_string() } else { a }
}

pub fn waybar_json(s: &Obj) -> String {
    let mut cfg = json!({"layer": s.val("layer"), "position": s.val("position"), "height": s.val("height"),
        "spacing": s.val("spacing"), "exclusive": s.val("exclusive")});
    if s.s("style") != "bar" && s.i("margin") != 0 {
        let m = s.i("margin");
        let top = s.s("position") == "top";
        cfg["margin-top"] = json!(if top { m } else { 0 });
        cfg["margin-bottom"] = json!(if s.s("position") == "bottom" { m } else { 0 });
        cfg["margin-left"] = json!(m * 2);
        cfg["margin-right"] = json!(m * 2);
    }
    cfg["modules-left"] = s.val("left").clone();
    cfg["modules-center"] = s.val("center").clone();
    cfg["modules-right"] = s.val("right").clone();
    for side in ["left", "center", "right"] {
        for m in s.strs(side) {
            cfg[m.as_str()] = module_cfg(&m, s);
        }
    }
    format!("// {} – automatisch erzeugt, wird beim Anwenden überschrieben\n{}\n", MARK, jsonx::dumps_indent(&cfg, 4))
}

pub fn css_id(m: &str) -> &'static str {
    match m {
        "custom/launcher" => "#custom-launcher", "hyprland/workspaces" => "#workspaces", "hyprland/window" => "#window",
        "clock" => "#clock", "pulseaudio" => "#pulseaudio", "network" => "#network", "battery" => "#battery",
        "backlight" => "#backlight", "bluetooth" => "#bluetooth", "cpu" => "#cpu", "memory" => "#memory",
        "temperature" => "#temperature", "disk" => "#disk", "mpris" => "#mpris", "hyprland/language" => "#language",
        "idle_inhibitor" => "#idle_inhibitor", "power-profiles-daemon" => "#power-profiles-daemon", "tray" => "#tray",
        _ => "#custom-power",
    }
}

fn ws_css(look: &str, fg: &str, acc: &str) -> String {
    match look {
        "box" => format!("color: #0e131a;\n    background: {};", rgba_css(acc, None)),
        "underline" => format!("color: {};\n    box-shadow: inset 0 -3px {};\n    border-radius: 0;",
            rgba_css(fg, None), rgba_css(acc, None)),
        "text" => format!("color: {};\n    background: transparent;", rgba_css(acc, None)),
        _ => format!("color: {};\n    background: {};", rgba_css(acc, None), rgba_css(acc, Some(0.18))),
    }
}

pub fn waybar_css(s: &Obj) -> String {
    let r = s.i("radius");
    let (fg, acc, bg) = (s.s("fg"), s.s("accent"), s.s("bg"));
    let border = if s.i("border_width") != 0 {
        format!("{}px solid {}", s.i("border_width"), rgba_css(s.s("border"), None))
    } else {
        "none".into()
    };
    let ids = MODULES.iter().map(|m| css_id(m)).collect::<Vec<_>>().join(", ");
    let pad = s.i("pad");
    let (win, module) = if s.s("style") == "islands" {
        ("background: transparent;".to_string(),
         format!("background: {}; border-radius: {}px; border: {}; padding: 0 {}px; margin: 0 3px;",
            rgba_css(bg, None), r, border, pad + 2))
    } else {
        (format!("background: {}; border-radius: {}px; border: {};", rgba_css(bg, None),
            if s.s("style") == "float" { r } else { 0 }, border),
         format!("padding: 0 {}px;", pad))
    };
    let colors = if s.b("colorful") {
        MODULES.iter().filter(|m| !["hyprland/workspaces", "tray", "custom/launcher"].contains(m))
            .map(|m| format!("{} {{ color: {}; }}", css_id(m), rgba_css(&module_color(m, s), None)))
            .collect::<Vec<_>>().join("\n") + "\n"
    } else {
        String::new()
    };
    let font = s.s("font");
    let fs = s.i("font_size");
    format!(r#"/* {mark} – automatisch erzeugt, wird beim Anwenden überschrieben */
* {{
    font-family: "{font}", "Symbols Nerd Font", sans-serif;
    font-size: {fs}px;
    font-weight: {fw};
    min-height: 0;
}}

window#waybar {{
    {win}
    color: {fgc};
}}

{ids} {{
    {module}
    color: {fgc};
}}

{colors}/* Symbole aus einer Symbol-Schrift: die normale Nerd Font zeichnet sie breiter als ihr Platz → wirken verschoben */
#custom-launcher, #custom-power {{
    font-family: "Symbols Nerd Font Mono", "JetBrainsMono Nerd Font Propo", "{font}", sans-serif;
    font-size: {fs3}px;
    padding: 0 {pad2}px;
    min-width: {fs3}px;
}}

#custom-launcher {{
    color: {accc};
}}

#custom-power {{
    color: #e5534b;
}}

#workspaces button {{
    padding: 0 7px;
    color: {fg55};
    background: transparent;
    border: none;
    border-radius: {r2}px;
    box-shadow: none;
}}

#workspaces button:hover {{
    background: {fg08};
}}

#workspaces button.active {{
    {wscss}
}}

#workspaces button.urgent {{
    color: #e5534b;
}}

#idle_inhibitor.activated {{
    color: {accc};
}}

#battery.warning {{ color: #d29922; }}
#battery.critical {{ color: #e5534b; }}

tooltip {{
    background: {bg95};
    border: 1px solid {fg15};
    border-radius: {r6}px;
}}
"#, mark = MARK, font = font, fs = fs, fw = jsonx::py_str(s.val("font_weight")), win = win, fgc = rgba_css(fg, None),
        ids = ids, module = module, colors = colors, fs3 = fs + 3,
        pad2 = pad + if s.s("style") == "islands" { 4 } else { 2 }, accc = rgba_css(acc, None),
        fg55 = rgba_css(fg, Some(0.55)), r2 = (r.div_euclid(2)).max(4), fg08 = rgba_css(fg, Some(0.08)),
        wscss = ws_css(s.s("ws_look"), fg, acc), bg95 = rgba_css(bg, Some(0.95)), fg15 = rgba_css(fg, Some(0.15)),
        r6 = (r.div_euclid(2)).max(6))
}

pub fn waybar_files(wdir: &Path) -> (PathBuf, PathBuf) {
    for n in ["config.jsonc", "config.json", "config"] {
        if wdir.join(n).is_file() {
            return (wdir.join(n), wdir.join("style.css"));
        }
    }
    (wdir.join("config.jsonc"), wdir.join("style.css"))
}

pub fn apply_waybar(s: &Obj, wdir: &Path, log: Log, restart: bool) -> (bool, String) {
    let _ = std::fs::create_dir_all(wdir);
    let (cfg, css) = waybar_files(wdir);
    for (target, data) in [(cfg, waybar_json(s)), (css, waybar_css(s))] {
        if let Some(bak) = backup_foreign(&target) {
            log(&format!("Sicherung: {}", bak.display()));
        }
        let _ = std::fs::write(&target, data);
        log(&format!("→ {}", target.display()));
    }
    if restart {
        restart_waybar(log);
    }
    (true, String::new())
}

pub fn undo_waybar(wdir: &Path, log: Log) -> bool {
    let (cfg, css) = waybar_files(wdir);
    let done: Vec<bool> = [cfg, css].iter().map(|f| undo_file(f)).collect();
    if done.iter().any(|d| *d) {
        restart_waybar(log);
    }
    done.iter().any(|d| *d)
}

pub fn restart_waybar(log: Log) {
    // SIGUSR2 lädt neu, stürzt aber bei manchen Versionen ab – Neustart ist zuverlässiger
    run(&["pkill", "-x", "waybar"], log, true);
    if dry() {
        log("$ waybar &  (Testmodus)");
        return;
    }
    std::thread::sleep(std::time::Duration::from_millis(300));
    match util::spawn_detached(&["waybar"]) {
        Ok(()) => log("$ waybar &"),
        Err(e) => log(&format!("waybar: {}", e)),
    }
}

// ---------- Prozesse ----------

pub fn run(cmd: &[&str], log: Log, quiet: bool) -> bool {
    if dry() {
        log(&format!("$ {}  (Testmodus)", cmd.join(" ")));
        return true;
    }
    let p = match util::capture(cmd, Some(10)) {
        Ok(p) => p,
        Err(e) => {
            log(&format!("$ {}: {}", cmd.join(" "), e));
            return false;
        }
    };
    if !quiet || p.ok() {
        log(&format!("$ {}", cmd.join(" ")));
        for ln in format!("{}{}", p.stdout, p.stderr).trim().lines() {
            log(&format!("  {}", ln));
        }
    }
    p.ok()
}

pub fn which(name: &str) -> bool {
    dry() || util::has(name)
}
