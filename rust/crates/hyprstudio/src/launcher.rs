//! App-Launcher Rofi, Wofi, Fuzzel, Walker, Tofi und L Launcher: ein gemeinsamer Zustand, eigene Config-Formate.

use crate::jsonx::{self, Get, Obj};
use crate::model::{backup_foreign, muted_color, norm, rgba_css, undo_file, Log, MARK};
use crate::{model, util};
use once_cell::sync::Lazy;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub const APPS: &[&str] = &["rofi", "wofi", "fuzzel", "walker", "tofi", "llauncher"];

pub fn command(app: &str) -> &'static str {
    match app {
        "rofi" => "rofi -show drun", "wofi" => "wofi --show drun", "fuzzel" => "fuzzel", "walker" => "walker",
        "tofi" => "tofi-drun --drun-launch=true", _ => "l-launcher",
    }
}

static DEFAULT: Lazy<Obj> =
    Lazy::new(|| jsonx::obj(jsonx::parse(include_str!("../data/launcher.LAUNCHER_DEFAULT.json"))));
static PRESETS: Lazy<Obj> = Lazy::new(|| jsonx::obj(jsonx::parse(include_str!("../data/launcher.PRESETS.json"))));

pub const LOOK: &[&str] = &["bg", "fg", "accent", "sel", "sel_fg", "input_bg", "border", "border_width", "radius",
    "el_radius", "width", "lines", "columns", "padding", "spacing", "position", "offset", "grid", "icon_size"];

pub fn default() -> &'static Obj {
    &DEFAULT
}

pub fn presets() -> &'static Obj {
    &PRESETS
}

fn ini_value(f: &Path, section: &str, key: &str) -> String {
    let Some(text) = util::read_opt(f) else { return String::new() };
    let mut sect = String::new();
    for ln in text.lines() {
        let ln = ln.trim();
        if ln.starts_with('[') {
            sect = ln.to_string();
        } else if sect == section && ln.contains('=') && ln.split('=').next().unwrap().trim() == key {
            return ln.split_once('=').unwrap().1.trim().to_string();
        }
    }
    String::new()
}

/// Symbolthema des Systems: KDE (kdeglobals [Icons] Theme, z. B. Sweet), sonst GTK, sonst gsettings.
pub fn system_icon_theme() -> String {
    if model::dry() {
        return String::new();
    }
    let cfg = util::xdg_config();
    let mut v = ini_value(&cfg.join("kdeglobals"), "[Icons]", "Theme");
    for f in [cfg.join("gtk-3.0/settings.ini"), cfg.join("gtk-4.0/settings.ini")] {
        if v.is_empty() {
            v = ini_value(&f, "[Settings]", "gtk-icon-theme-name").trim_matches('"').to_string();
        }
    }
    if v.is_empty() {
        if let Ok(o) = util::capture(&["gsettings", "get", "org.gnome.desktop.interface", "icon-theme"], Some(3)) {
            let out = o.stdout.trim().trim_matches('\'').to_string();
            v = if out.is_empty() || out == "hicolor" { String::new() } else { out };
        }
    }
    v
}

/// Im Launcher-Tab eingetragenes Thema, sonst das des Systems (KDE/GTK).
pub fn icon_theme(s: &Obj) -> String {
    let t = s.s("icon_theme");
    if t.is_empty() { system_icon_theme() } else { t.to_string() }
}

pub fn state(saved: Option<&Obj>) -> Obj {
    let mut s = DEFAULT.clone();
    if let Some(saved) = saved {
        for (k, v) in saved {
            if let Some(cur) = s.get(k) {
                if jsonx::same_type(v, cur) {
                    s.insert(k.clone(), v.clone());
                }
            }
        }
    }
    if !APPS.contains(&s.s("app")) {
        s.insert("app".into(), json!("rofi"));
    }
    s
}

pub fn preset(name: &str, base: Option<&Obj>) -> Obj {
    let mut s = state(base);
    for k in LOOK {
        s.insert(k.to_string(), DEFAULT[*k].clone());
    }
    if let Some(p) = PRESETS.get(name).and_then(|v| v.as_object()) {
        for (k, v) in p {
            s.insert(k.clone(), v.clone());
        }
    }
    s
}

pub fn muted(s: &Obj) -> String {
    muted_color(s.s("fg"))
}

fn p(s: &Obj, k: &str) -> String {
    jsonx::py_str(s.val(k))
}

/// Python `round(x)` (zur geraden Zahl bei .5)
pub fn py_round(x: f64) -> i64 {
    let r = x.round();
    if (x - x.trunc()).abs() == 0.5 { (2.0 * (x / 2.0).round()) as i64 } else { r as i64 }
}

// ---------- Rofi ----------

fn rofi_pos(pos: &str) -> (&'static str, &'static str) {
    match pos { "top" => ("north", "north"), "bottom" => ("south", "south"), _ => ("center", "center") }
}

/// Suche ab Namensanfang: nur im Namen, Eingabe als Ganzes ab Wortanfang (Rofi kennt kein reines
/// „beginnt mit“), fzf-Sortierung setzt Namen, die mit der Eingabe anfangen, nach oben → Enter öffnet sie.
fn rofi_match(s: &Obj) -> String {
    if s.b("name_search") {
        return "    matching: \"prefix\";\n    tokenize: false;\n    drun-match-fields: \"name\";\n    sorting-method: \"fzf\";\n"
            .into();
    }
    format!("    matching: \"{}\";\n", s.s("matching"))
}

pub fn rofi_config(s: &Obj, theme_path: &str) -> String {
    let modi = if s.s("mode") == "run" { "run,drun,window" } else { "drun,run,window" };
    let it = icon_theme(s);
    let icon = if it.is_empty() { String::new() } else { format!("    icon-theme: \"{}\";\n", it) };
    format!("/* {mark} – automatisch erzeugt, wird beim Anwenden überschrieben */
configuration {{
    modi: \"{modi}\";
    show-icons: {icons};
{icon}    font: \"{font} {fs}\";
    terminal: \"{term}\";
    display-drun: \"{prompt}\";
    display-run: \"{prompt}\";
    display-window: \"{prompt}\";
    drun-display-format: \"{{name}}\";
{matching}    case-sensitive: {cs};
    sort: true;
    hover-select: true;
}}

@theme \"{theme}\"
", mark = MARK, modi = modi, icons = if s.b("icons") { "true" } else { "false" }, icon = icon, font = s.s("font"),
        fs = p(s, "font_size"), term = s.s("terminal"), prompt = s.s("prompt"), matching = rofi_match(s),
        cs = if s.b("insensitive") { "false" } else { "true" }, theme = theme_path)
}

pub fn rofi_theme(s: &Obj) -> String {
    let (loc, anchor) = rofi_pos(s.s("position"));
    let off = match s.s("position") { "top" => s.i("offset"), "bottom" => -s.i("offset"), _ => 0 };
    let grid = s.b("grid");
    let cols = if grid { s.i("columns") } else { s.i("columns").max(1) };
    let prompt = if s.b("show_prompt") { "prompt, " } else { "" };
    let ha = if grid { "horizontal-align: 0.5;" } else { "" };
    format!("/* {mark} – automatisch erzeugt, wird beim Anwenden überschrieben */
* {{
    bg:      {bg};
    fg:      {fg};
    accent:  {accent};
    sel:     {sel};
    sel-fg:  {selfg};
    input:   {input};
    bordercol: {border};
    muted:   {muted};
    background-color: transparent;
    text-color: @fg;
}}

window {{
    location: {loc};
    anchor: {anchor};
    y-offset: {off}px;
    width: {width}px;
    background-color: @bg;
    border: {bw}px solid;
    border-color: @bordercol;
    border-radius: {radius}px;
    padding: {padding}px;
}}

mainbox {{
    spacing: 12px;
    children: [ inputbar, listview ];
}}

inputbar {{
    children: [ {prompt}entry ];
    spacing: 10px;
    padding: 10px 14px;
    border-radius: {r4}px;
    background-color: @input;
}}

prompt {{
    text-color: @accent;
}}

entry {{
    placeholder: \"{placeholder}\";
    placeholder-color: @muted;
    cursor: text;
}}

listview {{
    lines: {lines};
    columns: {cols};
    spacing: {spacing}px;
    fixed-height: true;
    scrollbar: {scroll};
}}

scrollbar {{
    handle-color: @accent;
    handle-width: 4px;
    border-radius: 4px;
}}

element {{
    orientation: {orient};
    padding: {epad};
    spacing: {esp};
    border-radius: {elr}px;
}}

element selected.normal, element selected.active {{
    background-color: @sel;
    text-color: @sel-fg;
}}

element-icon {{
    size: {isz}px;
    {ha}
}}

element-text {{
    vertical-align: 0.5;
    {ha}
    text-color: inherit;
}}
", mark = MARK, bg = norm(s.s("bg")), fg = norm(s.s("fg")), accent = norm(s.s("accent")), sel = norm(s.s("sel")),
        selfg = norm(s.s("sel_fg")), input = norm(s.s("input_bg")), border = norm(s.s("border")), muted = norm(&muted(s)),
        loc = loc, anchor = anchor, off = off, width = p(s, "width"), bw = p(s, "border_width"), radius = p(s, "radius"),
        padding = p(s, "padding"), prompt = prompt, r4 = (s.i("radius") - 4).max(0), placeholder = s.s("placeholder"),
        lines = p(s, "lines"), cols = cols, spacing = p(s, "spacing"),
        scroll = if s.b("show_scroll") { "true" } else { "false" },
        orient = if grid { "vertical" } else { "horizontal" }, epad = if grid { "12px 8px" } else { "8px 10px" },
        esp = if grid { "8px" } else { "12px" }, elr = p(s, "el_radius"), isz = p(s, "icon_size"), ha = ha)
}

// ---------- Wofi ----------

pub fn wofi_config(s: &Obj) -> String {
    let per = if s.b("grid") { s.i("icon_size") + 24 } else { s.i("icon_size").max(s.i("font_size") * 2) + 18 };
    let height = s.i("lines") * per + 2 * s.i("padding") + 60;
    let pos = s.s("position");
    let mut lines = vec![
        format!("# {} – automatisch erzeugt, wird beim Anwenden überschrieben", MARK),
        format!("show={}", s.s("mode")),
        format!("width={}", p(s, "width")),
        format!("height={}", height),
        format!("location={}", match pos { "top" => "top", "bottom" => "bottom", _ => "center" }),
        format!("prompt={}", s.s("placeholder")),
        format!("allow_images={}", if s.b("icons") { "true" } else { "false" }),
        format!("image_size={}", p(s, "icon_size")),
        format!("insensitive={}", if s.b("insensitive") { "true" } else { "false" }),
        format!("matching={}", if s.s("matching") == "fuzzy" && !s.b("name_search") { "fuzzy" } else { "contains" }),
        format!("columns={}", if s.b("grid") { s.i("columns") } else { s.i("columns").max(1) }),
        format!("hide_scroll={}", if s.b("show_scroll") { "false" } else { "true" }),
        "no_actions=true".into(),
        format!("term={}", s.s("terminal")),
        "gtk_dark=true".into(),
    ];
    // Achtung wofi(7): JEDER x/y-Offset bei location=center schaltet auf top_left um → nur oben/unten, nur ≠ 0
    if (pos == "top" || pos == "bottom") && s.i("offset") != 0 {
        lines.insert(5, format!("yoffset={}", if pos == "top" { s.i("offset") } else { -s.i("offset") }));
    }
    lines.join("\n") + "\n"
}

pub fn wofi_css(s: &Obj) -> String {
    let grid = s.b("grid");
    format!("/* {mark} – automatisch erzeugt, wird beim Anwenden überschrieben */
* {{
    font-family: \"{font}\", sans-serif;
    font-size: {fs}px;
}}

window {{
    margin: 0;
    border: {bw}px solid {border};
    border-radius: {radius}px;
    background-color: {bg};
}}

#outer-box {{
    padding: {padding}px;
}}

#input {{
    margin-bottom: 12px;
    padding: 10px 14px;
    border: none;
    border-radius: {r4}px;
    background-color: {input};
    color: {fg};
    box-shadow: none;
    outline: none;
}}

#inner-box, #scroll {{
    background: transparent;
}}

#entry {{
    padding: {epad};
    margin: {sp2}px 0;
    border-radius: {elr}px;
}}

#entry:selected {{
    background-color: {sel};
    outline: none;
}}

#text {{
    color: {fg};
    margin-left: 10px;
}}

#entry:selected #text {{
    color: {selfg};
}}
", mark = MARK, font = s.s("font"), fs = s.i("font_size") + 2, bw = p(s, "border_width"),
        border = rgba_css(s.s("border"), None), radius = p(s, "radius"), bg = rgba_css(s.s("bg"), None),
        padding = p(s, "padding"), r4 = (s.i("radius") - 4).max(0), input = rgba_css(s.s("input_bg"), None),
        fg = rgba_css(s.s("fg"), None), epad = if grid { "12px 8px" } else { "8px 10px" },
        sp2 = s.i("spacing").div_euclid(2), elr = p(s, "el_radius"), sel = rgba_css(s.s("sel"), None),
        selfg = rgba_css(s.s("sel_fg"), None))
}

// ---------- Fuzzel ----------

fn fz(c: &str) -> String {
    norm(c)[1..].to_string()
}

pub fn fuzzel_ini(s: &Obj) -> String {
    let chars = py_round(s.n("width") / (s.n("font_size") * 0.75)).max(20);
    let prompt = if s.b("show_prompt") { format!("{} ", s.s("prompt")) } else { String::new() };
    let mut main = vec![
        "[main]".to_string(),
        format!("font={}:size={}", s.s("font"), p(s, "font_size")),
        format!("prompt=\"{}\"", prompt),
        format!("placeholder={}", s.s("placeholder")),
        format!("icons-enabled={}", if s.b("icons") { "yes" } else { "no" }),
    ];
    let it = icon_theme(s);
    if !it.is_empty() {
        main.push(format!("icon-theme={}", it));
    }
    main.push(format!("terminal={} -e", s.s("terminal")));
    if s.b("name_search") {
        // nur Name/Dateiname, kein Teil-Treffer in Beschreibung oder Kategorie
        main.push("match-mode=exact".into());
        main.push("fields=name".into());
    }
    let pos = s.s("position");
    main.extend([
        format!("lines={}", s.i("lines") * if s.b("grid") { s.i("columns") } else { 1 }),
        format!("width={}", chars),
        format!("horizontal-pad={}", s.i("padding") + 4),
        format!("vertical-pad={}", p(s, "padding")),
        format!("inner-pad={}", s.i("spacing") + 4),
        format!("anchor={}", match pos { "top" => "top", "bottom" => "bottom", _ => "center" }),
        format!("y-margin={}", if pos != "center" { s.i("offset") } else { 0 }),
        "layer=overlay".into(),
        String::new(),
        "[colors]".into(),
        format!("background={}", fz(s.s("bg"))),
        format!("text={}", fz(s.s("fg"))),
        format!("prompt={}", fz(s.s("accent"))),
        format!("input={}", fz(s.s("fg"))),
        format!("match={}", fz(s.s("accent"))),
        format!("selection={}", fz(s.s("sel"))),
        format!("selection-text={}", fz(s.s("sel_fg"))),
        format!("selection-match={}", fz(s.s("sel_fg"))),
        format!("border={}", fz(s.s("border"))),
        String::new(),
        "[border]".into(),
        format!("width={}", p(s, "border_width")),
        format!("radius={}", p(s, "radius")),
    ]);
    format!("# {} – automatisch erzeugt, wird beim Anwenden überschrieben\n{}\n", MARK, main.join("\n"))
}

/// #rrggbbaa → rgba() for GTK CSS.
fn rgba(c: &str) -> String {
    let h = c.trim_start_matches('#');
    let h = if h.len() == 6 || h.len() == 8 { format!("{}ff", h)[..8].to_string() } else { "000000ff".into() };
    let q = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0);
    format!("rgba({},{},{},{:.2})", q(0), q(2), q(4), q(6) as f64 / 255.0)
}

/// Walker 2.x theme (GTK4 CSS); missing layout files fall back to Walker's default theme.
pub fn walker_css(s: &Obj) -> String {
    let small = (s.i("font_size") - 2).max(8);
    format!("/* {mark} – generated, overwritten when applied */
* {{ all: unset; font-family: \"{font}\"; font-size: {fs}pt; }}
scrollbar {{ opacity: 0; }}
.normal-icons {{ -gtk-icon-size: {isz}px; }}
.large-icons {{ -gtk-icon-size: {isz8}px; }}
.box-wrapper {{ background: {bg}; padding: {padding}px; border-radius: {radius}px;
  border: {bw}px solid {border}; min-width: {width}px; }}
.input {{ caret-color: {accent}; background: {input}; padding: 10px;
  color: {fg}; border-radius: {elr}px; }}
.input placeholder {{ opacity: 0.5; }}
.list {{ color: {fg}; }}
.item-box {{ border-radius: {elr}px; padding: {sp6}px; }}
child:selected .item-box, row:selected .item-box {{ background: {sel}; color: {selfg}; }}
.item-subtext {{ font-size: {small}pt; opacity: 0.55; }}
.keybinds {{ padding-top: 8px; font-size: {small}pt; color: {fg}; opacity: 0.6; }}
.preview {{ border: 1px solid {border}; border-radius: {elr}px; color: {fg}; }}
", mark = MARK, font = s.s("font"), fs = p(s, "font_size"), isz = p(s, "icon_size"), isz8 = s.i("icon_size") + 8,
        bg = rgba(s.s("bg")), padding = p(s, "padding"), radius = p(s, "radius"), bw = p(s, "border_width"),
        border = rgba(s.s("border")), width = p(s, "width"), accent = rgba(s.s("accent")), input = rgba(s.s("input_bg")),
        fg = rgba(s.s("fg")), elr = p(s, "el_radius"), sp6 = s.i("spacing") + 6, sel = rgba(s.s("sel")),
        selfg = rgba(s.s("sel_fg")), small = small)
}

/// config.toml: only the theme line is set, everything else stays as it is.
pub fn walker_config(cfg_home: &Path) -> String {
    let text = std::fs::read_to_string(cfg_home.join("walker").join("config.toml")).unwrap_or_default();
    let r = crate::re!(r"(?m)^theme\s*=.*$");
    if crate::re!(r"(?m)^theme\s*=").is_match(&text) {
        return r.replacen(&text, 1, "theme = \"hyprstudio\"").into_owned();
    }
    format!("theme = \"hyprstudio\"\n{}", text)
}

pub fn tofi_config(s: &Obj) -> String {
    let prompt = if s.b("show_prompt") { format!("{} ", s.s("prompt")) } else { String::new() };
    let lines = [
        format!("# {} – generated, overwritten when applied", MARK),
        format!("font = {}", s.s("font")), format!("font-size = {}", p(s, "font_size")),
        format!("width = {}", p(s, "width")),
        format!("height = {}", (s.i("lines") * (s.i("font_size") * 2 + s.i("spacing") * 2) + 90).max(200)),
        format!("background-color = {}", s.s("bg")), format!("text-color = {}", s.s("fg")),
        format!("prompt-color = {}", s.s("accent")),
        format!("selection-color = {}", s.s("sel_fg")), format!("selection-background = {}", s.s("sel")),
        format!("input-background = {}", s.s("input_bg")),
        format!("border-width = {}", p(s, "border_width")), format!("border-color = {}", s.s("border")),
        "outline-width = 0".into(),
        format!("corner-radius = {}", p(s, "radius")), format!("padding-top = {}", p(s, "padding")),
        format!("padding-bottom = {}", p(s, "padding")),
        format!("padding-left = {}", p(s, "padding")), format!("padding-right = {}", p(s, "padding")),
        format!("prompt-text = \"{}\"", prompt),
        format!("placeholder-text = \"{}\"", s.s("placeholder")), format!("num-results = {}", p(s, "lines")),
        format!("result-spacing = {}", s.i("spacing") + 4),
        format!("fuzzy-match = {}", if s.s("matching") == "fuzzy" { "true" } else { "false" }),
        "drun-launch = true".into(),
        format!("anchor = {}", match s.s("position") { "top" => "top", "bottom" => "bottom", _ => "center" }),
    ];
    lines.join("\n") + "\n"
}

// ---------- Schreiben ----------

/// [(pfad, inhalt)] für ALLE Launcher (gewählter zuerst). Jeder nimmt seine eigenen Einstellungen
/// (per_app[app]); wer noch keine hat, bekommt die des gewählten – so ist jeder Launcher fertig eingerichtet,
/// auch wenn ein Kürzel einen anderen startet als den im Tab gewählten.
pub fn files(s: &Obj, cfg_home: &Path, per_app: Option<&Obj>) -> Vec<(PathBuf, String)> {
    let cur = s.s("app").to_string();
    let mut order = vec![cur.clone()];
    order.extend(APPS.iter().filter(|a| **a != cur).map(|a| a.to_string()));
    let mut out = vec![];
    for app in order {
        let mut own = if app == cur {
            s.clone()
        } else {
            let saved = per_app.and_then(|p| p.get(&app)).filter(|v| jsonx::truthy(v)).and_then(|v| v.as_object());
            state(Some(saved.unwrap_or(s)))
        };
        own.insert("app".into(), json!(app));
        out.extend(files_for(&app, &own, cfg_home));
    }
    out
}

/// Befehl → Launcher-Schlüssel (rofi/wofi/fuzzel/llauncher) oder None.
pub fn app_of_cmd(cmd: &str) -> Option<&'static str> {
    let mut c = cmd.trim().to_string();
    if c.starts_with("pkill -x ") && c.contains("||") {
        c = c.split_once("||").unwrap().1.trim().to_string();
    }
    if c.starts_with("hypr-studio --launcher") {
        return Some("llauncher");
    }
    let first = if c.is_empty() { String::new() } else {
        util::file_name(Path::new(c.split(' ').next().unwrap()))
    };
    Some(match first.as_str() {
        "rofi" => "rofi", "wofi" => "wofi", "fuzzel" => "fuzzel", "walker" => "walker", "tofi-drun" | "tofi" => "tofi",
        "l-launcher" => "llauncher",
        _ => return None,
    })
}

fn files_for(app: &str, s: &Obj, cfg_home: &Path) -> Vec<(PathBuf, String)> {
    match app {
        "llauncher" => {
            // Kennung vorn, damit die eigene Datei nicht bei jedem Anwenden als „fremd“ neu gesichert wird
            let mut data = Obj::new();
            data.insert("_generator".into(), json!(MARK));
            for (k, v) in s {
                data.insert(k.clone(), v.clone());
            }
            vec![(cfg_home.join("LabtosoStudios").join("l-launcher.json"), jsonx::dumps_indent(&Value::Object(data), 1))]
        }
        "rofi" => {
            let d = cfg_home.join("rofi");
            let theme = d.join("hyprstudio.rasi");
            vec![(d.join("config.rasi"), rofi_config(s, &theme.to_string_lossy().replace('\\', "/"))),
                (theme, rofi_theme(s))]
        }
        "wofi" => {
            let d = cfg_home.join("wofi");
            vec![(d.join("config"), wofi_config(s)), (d.join("style.css"), wofi_css(s))]
        }
        "walker" => {
            let d = cfg_home.join("walker");
            vec![(d.join("themes/hyprstudio/style.css"), walker_css(s)), (d.join("config.toml"), walker_config(cfg_home))]
        }
        "tofi" => vec![(cfg_home.join("tofi").join("config"), tofi_config(s))],
        _ => vec![(cfg_home.join("fuzzel").join("fuzzel.ini"), fuzzel_ini(s))],
    }
}

pub fn apply(s: &Obj, cfg_home: &Path, log: Log, per_app: Option<&Obj>) -> (bool, String) {
    for (target, data) in files(s, cfg_home, per_app) {
        if let Some(d) = target.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        if let Some(bak) = backup_foreign(&target) {
            log(&format!("Sicherung: {}", bak.display()));
        }
        let _ = std::fs::write(&target, data);
        log(&format!("→ {}", target.display()));
    }
    (true, String::new())
}

pub fn undo(s: &Obj, cfg_home: &Path) -> bool {
    files(s, cfg_home, None).iter().map(|(p, _)| undo_file(p)).fold(false, |a, b| a | b)
}
