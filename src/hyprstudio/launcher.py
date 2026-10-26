"""App-Launcher Rofi, Wofi und Fuzzel: ein gemeinsamer Zustand, drei Config-Formate. Ohne Qt."""

import json
import os
import subprocess
from pathlib import Path

from .model import MARK, _backup_foreign, muted_color, norm, rgba_css, undo_file

APPS = ("rofi", "wofi", "fuzzel", "llauncher")
COMMANDS = {"rofi": "rofi -show drun", "wofi": "wofi --show drun", "fuzzel": "fuzzel", "llauncher": "l-launcher"}

LAUNCHER_DEFAULT = {
    "app": "rofi",
    # Aussehen (Presets)
    "bg": "#0e131aee", "fg": "#e6edf5ff", "accent": "#3d8bfdff", "sel": "#3d8bfd33", "sel_fg": "#ffffffff",
    "input_bg": "#1b2430ff", "border": "#253142ff", "border_width": 2, "radius": 14, "el_radius": 8,
    "width": 620, "lines": 8, "columns": 1, "padding": 16, "spacing": 4, "position": "center", "offset": 0,
    "grid": False, "icon_size": 24,
    # Verhalten
    "font": "JetBrainsMono Nerd Font", "font_size": 12, "icons": True, "icon_theme": "",
    "show_prompt": True, "prompt": "Apps", "placeholder": "Suchen …", "mode": "drun", "matching": "fuzzy",
    "insensitive": True, "terminal": "kitty", "show_scroll": False, "name_search": True, "toggle": True,
}
LOOK = ("bg", "fg", "accent", "sel", "sel_fg", "input_bg", "border", "border_width", "radius", "el_radius",
        "width", "lines", "columns", "padding", "spacing", "position", "offset", "grid", "icon_size")

PRESETS = {
    "spotlight": {},
    "grid": {"grid": True, "columns": 5, "lines": 3, "icon_size": 48, "width": 760, "radius": 18, "el_radius": 12},
    "compact": {"position": "top", "offset": 60, "width": 480, "lines": 6, "radius": 10, "padding": 10,
                "icon_size": 20, "border_width": 1},
    "glass": {"bg": "#ffffff1a", "input_bg": "#ffffff14", "border": "#ffffff33", "sel": "#ffffff26",
              "radius": 20, "border_width": 1},
    "tokyo": {"bg": "#1a1b26f0", "fg": "#c0caf5ff", "accent": "#7aa2f7ff", "sel": "#7aa2f733",
              "input_bg": "#24283bff", "border": "#7aa2f7ff"},
    "catppuccin": {"bg": "#1e1e2ef0", "fg": "#cdd6f4ff", "accent": "#cba6f7ff", "sel": "#cba6f733",
                   "input_bg": "#313244ff", "border": "#cba6f7ff", "radius": 16},
    "nord": {"bg": "#2e3440f0", "fg": "#eceff4ff", "accent": "#88c0d0ff", "sel": "#88c0d033",
             "input_bg": "#3b4252ff", "border": "#4c566aff", "radius": 8},
    "dracula": {"bg": "#282a36f0", "fg": "#f8f8f2ff", "accent": "#bd93f9ff", "sel": "#bd93f933",
                "input_bg": "#44475aff", "border": "#bd93f9ff"},
    "rosepine": {"bg": "#191724f0", "fg": "#e0def4ff", "accent": "#ebbcbaff", "sel": "#ebbcba33",
                 "input_bg": "#26233aff", "border": "#ebbcbaff", "radius": 18},
    "everforest": {"bg": "#2d353bf0", "fg": "#d3c6aaff", "accent": "#a7c080ff", "sel": "#a7c08033",
                   "input_bg": "#343f44ff", "border": "#a7c080ff"},
    "kanagawa": {"bg": "#1f1f28f0", "fg": "#dcd7baff", "accent": "#7e9cd8ff", "sel": "#7e9cd833",
                 "input_bg": "#2a2a37ff", "border": "#7e9cd8ff", "radius": 10},
    "neon": {"bg": "#0b0b14f0", "fg": "#f8f8ffff", "accent": "#ff2bd6ff", "sel": "#ff2bd633", "input_bg": "#16162aff",
             "border": "#00e5ffff", "border_width": 2},
    "topbar": {"position": "top", "offset": 0, "width": 1100, "lines": 6, "radius": 0, "el_radius": 4,
               "border_width": 0, "padding": 10},
    "bigicons": {"grid": True, "columns": 4, "lines": 2, "icon_size": 64, "width": 700, "radius": 22, "el_radius": 16},
    "sharp": {"radius": 0, "el_radius": 0, "border_width": 2, "border": "#e6edf5ff", "sel": "#e6edf533"},
    "solid": {"bg": "#0e131aff"},
    "solid_tokyo": {"bg": "#1a1b26ff", "fg": "#c0caf5ff", "accent": "#7aa2f7ff", "sel": "#7aa2f733",
                    "input_bg": "#24283bff", "border": "#7aa2f7ff"},
    "solid_catppuccin": {"bg": "#1e1e2eff", "fg": "#cdd6f4ff", "accent": "#cba6f7ff", "sel": "#cba6f733",
                         "input_bg": "#313244ff", "border": "#cba6f7ff", "radius": 16},
    "solid_nord": {"bg": "#2e3440ff", "fg": "#eceff4ff", "accent": "#88c0d0ff", "sel": "#88c0d033",
                   "input_bg": "#3b4252ff", "border": "#4c566aff", "radius": 8},
    "gruvbox": {"bg": "#282828f0", "fg": "#ebdbb2ff", "accent": "#fabd2fff", "sel": "#fabd2f33",
                "input_bg": "#3c3836ff", "border": "#fabd2fff", "radius": 4, "el_radius": 2},
}


def _ini_value(f: Path, section: str, key: str) -> str:
    try:
        lines = f.read_text(encoding="utf-8", errors="replace").splitlines()
    except OSError:
        return ""
    sect = ""
    for ln in lines:
        ln = ln.strip()
        if ln.startswith("["):
            sect = ln
        elif sect == section and "=" in ln and ln.split("=", 1)[0].strip() == key:
            return ln.split("=", 1)[1].strip()
    return ""


def system_icon_theme() -> str:
    """Symbolthema des Systems: KDE (kdeglobals [Icons] Theme, z. B. Sweet), sonst GTK, sonst gsettings."""
    from . import model
    if model.DRY:
        return ""
    cfg = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config"))
    v = _ini_value(cfg / "kdeglobals", "[Icons]", "Theme")
    for f in (cfg / "gtk-3.0" / "settings.ini", cfg / "gtk-4.0" / "settings.ini"):
        v = v or _ini_value(f, "[Settings]", "gtk-icon-theme-name").strip('"')
    if not v:
        try:
            out = subprocess.run(["gsettings", "get", "org.gnome.desktop.interface", "icon-theme"],
                                 capture_output=True, text=True, timeout=3).stdout.strip().strip("'")
            v = "" if out in ("", "hicolor") else out
        except (OSError, subprocess.TimeoutExpired):
            pass
    return v


def icon_theme(s: dict) -> str:
    """Im Launcher-Tab eingetragenes Thema, sonst das des Systems (KDE/GTK)."""
    return s["icon_theme"] or system_icon_theme()


def _copy(d):
    return json.loads(json.dumps(d))


def state(saved: dict | None) -> dict:
    s = _copy(LAUNCHER_DEFAULT)
    for k, v in (saved or {}).items():
        if k in s and type(v) is type(s[k]):
            s[k] = v
    if s["app"] not in APPS:
        s["app"] = "rofi"
    return s


def preset(name: str, base: dict | None = None) -> dict:
    s = state(base)
    for k in LOOK:
        s[k] = _copy(LAUNCHER_DEFAULT[k])
    s.update(_copy(PRESETS.get(name, {})))
    return s


def muted(s: dict) -> str:
    return muted_color(s["fg"])


# ---------- Rofi ----------

ROFI_POS = {"center": ("center", "center"), "top": ("north", "north"), "bottom": ("south", "south")}


def _hex(c: str) -> str:
    return norm(c)


def _rofi_match(s: dict) -> str:
    """Suche ab Namensanfang: nur im Namen, Eingabe als Ganzes ab Wortanfang (Rofi kennt kein reines
    „beginnt mit“), fzf-Sortierung setzt Namen, die mit der Eingabe anfangen, nach oben → Enter öffnet sie."""
    if s["name_search"]:
        return ('    matching: "prefix";\n    tokenize: false;\n    drun-match-fields: "name";\n'
                '    sorting-method: "fzf";\n')
    return f'    matching: "{s["matching"]}";\n'


def rofi_config(s: dict, theme_path: str) -> str:
    modi = {"drun": "drun,run,window", "run": "run,drun,window"}[s["mode"]]
    return f"""/* {MARK} – automatisch erzeugt, wird beim Anwenden überschrieben */
configuration {{
    modi: "{modi}";
    show-icons: {"true" if s["icons"] else "false"};
{f'    icon-theme: "{icon_theme(s)}";' + chr(10) if icon_theme(s) else ""}    font: "{s["font"]} {s["font_size"]}";
    terminal: "{s["terminal"]}";
    display-drun: "{s["prompt"]}";
    display-run: "{s["prompt"]}";
    display-window: "{s["prompt"]}";
    drun-display-format: "{{name}}";
{_rofi_match(s)}    case-sensitive: {"false" if s["insensitive"] else "true"};
    sort: true;
    hover-select: true;
}}

@theme "{theme_path}"
"""


def rofi_theme(s: dict) -> str:
    loc, anchor = ROFI_POS[s["position"]]
    off = {"center": 0, "top": s["offset"], "bottom": -s["offset"]}[s["position"]]
    grid = s["grid"]
    cols = s["columns"] if grid else max(1, s["columns"])
    prompt = "prompt, " if s["show_prompt"] else ""
    return f"""/* {MARK} – automatisch erzeugt, wird beim Anwenden überschrieben */
* {{
    bg:      {_hex(s["bg"])};
    fg:      {_hex(s["fg"])};
    accent:  {_hex(s["accent"])};
    sel:     {_hex(s["sel"])};
    sel-fg:  {_hex(s["sel_fg"])};
    input:   {_hex(s["input_bg"])};
    bordercol: {_hex(s["border"])};
    muted:   {_hex(muted(s))};
    background-color: transparent;
    text-color: @fg;
}}

window {{
    location: {loc};
    anchor: {anchor};
    y-offset: {off}px;
    width: {s["width"]}px;
    background-color: @bg;
    border: {s["border_width"]}px solid;
    border-color: @bordercol;
    border-radius: {s["radius"]}px;
    padding: {s["padding"]}px;
}}

mainbox {{
    spacing: 12px;
    children: [ inputbar, listview ];
}}

inputbar {{
    children: [ {prompt}entry ];
    spacing: 10px;
    padding: 10px 14px;
    border-radius: {max(0, s["radius"] - 4)}px;
    background-color: @input;
}}

prompt {{
    text-color: @accent;
}}

entry {{
    placeholder: "{s["placeholder"]}";
    placeholder-color: @muted;
    cursor: text;
}}

listview {{
    lines: {s["lines"]};
    columns: {cols};
    spacing: {s["spacing"]}px;
    fixed-height: true;
    scrollbar: {"true" if s["show_scroll"] else "false"};
}}

scrollbar {{
    handle-color: @accent;
    handle-width: 4px;
    border-radius: 4px;
}}

element {{
    orientation: {"vertical" if grid else "horizontal"};
    padding: {"12px 8px" if grid else "8px 10px"};
    spacing: {"8px" if grid else "12px"};
    border-radius: {s["el_radius"]}px;
}}

element selected.normal, element selected.active {{
    background-color: @sel;
    text-color: @sel-fg;
}}

element-icon {{
    size: {s["icon_size"]}px;
    {"horizontal-align: 0.5;" if grid else ""}
}}

element-text {{
    vertical-align: 0.5;
    {"horizontal-align: 0.5;" if grid else ""}
    text-color: inherit;
}}
"""


# ---------- Wofi ----------

WOFI_POS = {"center": "center", "top": "top", "bottom": "bottom"}


def wofi_config(s: dict) -> str:
    height = s["lines"] * (s["icon_size"] + 24 if s["grid"] else max(s["icon_size"], s["font_size"] * 2) + 18) \
        + 2 * s["padding"] + 60
    lines = [
        f"# {MARK} – automatisch erzeugt, wird beim Anwenden überschrieben",
        f"show={s['mode']}",
        f"width={s['width']}",
        f"height={height}",
        f"location={WOFI_POS[s['position']]}",
        f"prompt={s['placeholder']}",
        f"allow_images={'true' if s['icons'] else 'false'}",
        f"image_size={s['icon_size']}",
        f"insensitive={'true' if s['insensitive'] else 'false'}",
        f"matching={'fuzzy' if s['matching'] == 'fuzzy' and not s['name_search'] else 'contains'}",
        f"columns={s['columns'] if s['grid'] else max(1, s['columns'])}",
        f"hide_scroll={'false' if s['show_scroll'] else 'true'}",
        "no_actions=true",
        f"term={s['terminal']}",
        "gtk_dark=true",
    ]
    # Achtung wofi(7): JEDER x/y-Offset bei location=center schaltet auf top_left um → nur oben/unten, nur ≠ 0
    if s["position"] in ("top", "bottom") and s["offset"]:
        lines.insert(5, f"yoffset={s['offset'] if s['position'] == 'top' else -s['offset']}")
    return "\n".join(lines) + "\n"


def wofi_css(s: dict) -> str:
    grid = s["grid"]
    return f"""/* {MARK} – automatisch erzeugt, wird beim Anwenden überschrieben */
* {{
    font-family: "{s["font"]}", sans-serif;
    font-size: {s["font_size"] + 2}px;
}}

window {{
    margin: 0;
    border: {s["border_width"]}px solid {rgba_css(s["border"])};
    border-radius: {s["radius"]}px;
    background-color: {rgba_css(s["bg"])};
}}

#outer-box {{
    padding: {s["padding"]}px;
}}

#input {{
    margin-bottom: 12px;
    padding: 10px 14px;
    border: none;
    border-radius: {max(0, s["radius"] - 4)}px;
    background-color: {rgba_css(s["input_bg"])};
    color: {rgba_css(s["fg"])};
    box-shadow: none;
    outline: none;
}}

#inner-box, #scroll {{
    background: transparent;
}}

#entry {{
    padding: {"12px 8px" if grid else "8px 10px"};
    margin: {s["spacing"] // 2}px 0;
    border-radius: {s["el_radius"]}px;
}}

#entry:selected {{
    background-color: {rgba_css(s["sel"])};
    outline: none;
}}

#text {{
    color: {rgba_css(s["fg"])};
    margin-left: 10px;
}}

#entry:selected #text {{
    color: {rgba_css(s["sel_fg"])};
}}
"""


# ---------- Fuzzel ----------

FUZZEL_ANCHOR = {"center": "center", "top": "top", "bottom": "bottom"}


def _fz(c: str) -> str:
    return norm(c)[1:]


def fuzzel_ini(s: dict) -> str:
    chars = max(20, round(s["width"] / (s["font_size"] * 0.75)))
    main = [
        "[main]",
        f"font={s['font']}:size={s['font_size']}",
        f'prompt="{s["prompt"] + " " if s["show_prompt"] else ""}"',
        f"placeholder={s['placeholder']}",
        f"icons-enabled={'yes' if s['icons'] else 'no'}",
    ]
    if icon_theme(s):
        main.append(f"icon-theme={icon_theme(s)}")
    main += [
        f"terminal={s['terminal']} -e",
    ]
    if s["name_search"]:  # nur Name/Dateiname, kein Teil-Treffer in Beschreibung oder Kategorie
        main += ["match-mode=exact", "fields=name"]
    main += [
        f"lines={s['lines'] * (s['columns'] if s['grid'] else 1)}",
        f"width={chars}",
        f"horizontal-pad={s['padding'] + 4}",
        f"vertical-pad={s['padding']}",
        f"inner-pad={s['spacing'] + 4}",
        f"anchor={FUZZEL_ANCHOR[s['position']]}",
        f"y-margin={s['offset'] if s['position'] != 'center' else 0}",
        "layer=overlay",
        "",
        "[colors]",
        f"background={_fz(s['bg'])}",
        f"text={_fz(s['fg'])}",
        f"prompt={_fz(s['accent'])}",
        f"input={_fz(s['fg'])}",
        f"match={_fz(s['accent'])}",
        f"selection={_fz(s['sel'])}",
        f"selection-text={_fz(s['sel_fg'])}",
        f"selection-match={_fz(s['sel_fg'])}",
        f"border={_fz(s['border'])}",
        "",
        "[border]",
        f"width={s['border_width']}",
        f"radius={s['radius']}",
    ]
    return f"# {MARK} – automatisch erzeugt, wird beim Anwenden überschrieben\n" + "\n".join(main) + "\n"


# ---------- Schreiben ----------

def files(s: dict, cfg_home: Path, home_for_theme: Path | None = None, per_app: dict | None = None) -> list:
    """[(pfad, inhalt)] für ALLE Launcher (gewählter zuerst). Jeder nimmt seine eigenen Einstellungen
    (per_app[app]); wer noch keine hat, bekommt die des gewählten – so ist jeder Launcher fertig eingerichtet,
    auch wenn ein Kürzel einen anderen startet als den im Tab gewählten."""
    out = []
    for app in [s["app"]] + [a for a in APPS if a != s["app"]]:
        own = s if app == s["app"] else state((per_app or {}).get(app) or s)
        out += _files_for(app, dict(own, app=app), cfg_home)
    return out


def app_of_cmd(cmd: str) -> str | None:
    """Befehl → Launcher-Schlüssel (rofi/wofi/fuzzel/llauncher) oder None."""
    c = (cmd or "").strip()
    if c.startswith("pkill -x ") and "||" in c:
        c = c.split("||", 1)[1].strip()
    if c.startswith("hypr-studio --launcher"):
        return "llauncher"
    first = Path(c.split(" ", 1)[0]).name if c else ""
    return {"rofi": "rofi", "wofi": "wofi", "fuzzel": "fuzzel", "l-launcher": "llauncher"}.get(first)


def _files_for(app: str, s: dict, cfg_home: Path) -> list:
    if app == "llauncher":  # L Launcher liest seine Einstellungen beim Start aus dieser Datei
        import json
        # Kennung vorn, damit die eigene Datei nicht bei jedem Anwenden als „fremd“ neu gesichert wird
        data = {"_generator": MARK, **s}
        return [(cfg_home / "LabtosoStudios" / "l-launcher.json", json.dumps(data, ensure_ascii=False, indent=1))]
    if app == "rofi":
        d = cfg_home / "rofi"
        theme = d / "hyprstudio.rasi"
        return [(d / "config.rasi", rofi_config(s, str(theme).replace("\\", "/"))), (theme, rofi_theme(s))]
    if app == "wofi":
        d = cfg_home / "wofi"
        return [(d / "config", wofi_config(s)), (d / "style.css", wofi_css(s))]
    return [(cfg_home / "fuzzel" / "fuzzel.ini", fuzzel_ini(s))]


def apply(s: dict, cfg_home: Path, log, per_app: dict | None = None) -> tuple[bool, str]:
    for target, data in files(s, cfg_home, per_app=per_app):
        target.parent.mkdir(parents=True, exist_ok=True)
        bak = _backup_foreign(target)
        if bak:
            log(f"Sicherung: {bak}")
        target.write_text(data, encoding="utf-8")
        log(f"→ {target}")
    return True, ""


def undo(s: dict, cfg_home: Path) -> bool:
    return any([undo_file(p) for p, _ in files(s, cfg_home)])
