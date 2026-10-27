"""Daten, Presets und Config-Erzeugung für Hyprland (Lua ab 0.55 oder hyprlang) und Waybar. Ohne Qt."""

import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

from . import palettes

MARK = "Hypr Studio (LabtosoStudios)"
DRY = os.environ.get("HYPRSTUDIO_DRYRUN") == "1" or sys.platform == "win32"
FORMAT = ""  # Erweiterte Einstellungen: "lua" (neu, 0.55+), "conf" (alt, hyprlang bis 0.56) oder "" = erkennen
CACHE = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "LabtosoStudios" / "hyprstudio"


def config_home() -> Path:
    if DRY:
        return CACHE / "testmodus" / ".config"
    return Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config"))


# ---------- Farben ----------

def norm(c: str) -> str:
    """'#rgb', '#rrggbb' oder '#rrggbbaa' → '#rrggbbaa' (klein)."""
    h = c.strip().lstrip("#").lower()
    if len(h) == 3:
        h = "".join(ch * 2 for ch in h)
    if len(h) == 6:
        h += "ff"
    if len(h) != 8 or any(ch not in "0123456789abcdef" for ch in h):
        return "#000000ff"
    return "#" + h


def muted_color(c: str) -> str:
    return norm(c)[:7] + "88"


def rgba_hypr(c: str) -> str:
    return f"rgba({norm(c)[1:]})"


def rgba_css(c: str, alpha: float | None = None) -> str:
    h = norm(c)[1:]
    r, g, b, a = (int(h[i:i + 2], 16) for i in (0, 2, 4, 6))
    a = a / 255 if alpha is None else alpha
    return f"rgba({r}, {g}, {b}, {a:.2f})"


# ---------- Hyprland ----------

HYPR_DEFAULT = {
    # Aussehen (wird von Presets gesetzt)
    "gaps_in": 5, "gaps_out": 20, "border_size": 2,
    "gradient": True, "active1": "#33ccffee", "active2": "#00ff99ee", "angle": 45,
    "inactive": "#595959aa",
    "rounding": 10, "rounding_power": 2.0,
    "active_opacity": 1.0, "inactive_opacity": 1.0, "dim_inactive": False, "dim_strength": 0.3,
    "blur": True, "blur_size": 3, "blur_passes": 1, "blur_vibrancy": 0.17,
    "blur_noise": 0.0117, "blur_contrast": 0.8916, "blur_brightness": 0.8172, "blur_xray": False,
    "blur_ignore_opacity": True,
    "shadow": True, "shadow_range": 4, "shadow_power": 3, "shadow_color": "#1a1a1aee", "shadow_sharp": False,
    "anim": True, "anim_curve": "smooth", "anim_tempo": 1.0, "win_style": "popin", "ws_style": "slide",
    "border_rotate": False, "border_rotate_speed": 60,
    # Verhalten (bleibt beim Preset-Wechsel)
    "layout": "dwindle", "resize_on_border": False, "smart_gaps": False,
    "dwindle_preserve": True, "dwindle_force": 0, "dwindle_smart": False,
    "master_new": "master", "master_mfact": 0.55, "master_orient": "left",
    "anim_windows": True, "anim_fade": True, "anim_ws": True, "anim_border": True, "anim_layers": True,
    "kb_layout": "de", "kb_variant": "", "kb_options": "", "numlock": False,
    "repeat_rate": 25, "repeat_delay": 600, "sensitivity": 0.0, "accel": "default", "follow_mouse": 1,
    "left_handed": False, "mouse_natural": False, "natural_scroll": False, "dwt": True, "scroll_factor": 1.0, "gesture": 0,
    "cursor_size": 24, "cursor_theme": "", "cursor_timeout": 0.0, "cursor_hide_keys": False,
    "qt_theme": "",  # QT_QPA_PLATFORMTHEME (kde/qt6ct) – setzt Sonstiges › Hell / Dunkel
    "no_logo": True, "bg_color": "#111111ff", "focus_on_activate": False, "swallow": False,
    "swallow_regex": "^(kitty|foot|Alacritty)$", "mmb_paste": True, "dpms_mouse": True, "dpms_key": True,
    "monitor_on": False, "monitor_mode": "preferred", "monitor_scale": "auto",
    "autostart": [], "autostart_waybar": False, "waybar_managed": False,
    "polkit_agent": False, "polkit_managed": False,  # hyprpolkitagent beim Start (Sonstiges › Polkit-Agent)
    "laptop_keys": False, "laptop_managed": False, "vol_tool": "wpctl",  # Sonstiges › Laptop-Tasten
    "workspaces_on": False, "workspaces": [],  # Arbeitsflächen-Regeln (Name, Monitor, Lücken …)
    "skip_opts": [],  # vom Fehler-Fixer abgeschaltete Optionen, z. B. "decoration.blur.xray"
    "binds": [],  # leer = Hypr Studio schreibt keine Tastenkürzel
}
LOOK_KEYS = tuple(list(HYPR_DEFAULT)[:list(HYPR_DEFAULT).index("layout")])

_HYPR_PRESETS_OWN = {
    "hyprland": {},
    "tokyo": {"active1": "#7aa2f7ff", "active2": "#bb9af7ff", "inactive": "#414868aa", "gaps_in": 5,
              "gaps_out": 12, "rounding": 10, "inactive_opacity": 0.92, "blur_size": 6, "blur_passes": 2},
    "catppuccin": {"active1": "#cba6f7ff", "active2": "#89b4faff", "inactive": "#45475aaa", "rounding": 12,
                   "gaps_out": 14, "inactive_opacity": 0.94},
    "nord": {"active1": "#88c0d0ff", "active2": "#81a1c1ff", "inactive": "#4c566aaa", "rounding": 8,
             "gaps_in": 4, "gaps_out": 10, "anim_curve": "fast"},
    "gruvbox": {"active1": "#fabd2fff", "active2": "#fe8019ff", "inactive": "#504945aa", "rounding": 4,
                "gaps_in": 4, "gaps_out": 8, "border_size": 3, "angle": 90},
    "rosepine": {"active1": "#ebbcbaff", "active2": "#c4a7e7ff", "inactive": "#26233aaa", "rounding": 14,
                 "gaps_out": 16, "anim_curve": "bouncy"},
    "dracula": {"active1": "#bd93f9ff", "active2": "#ff79c6ff", "inactive": "#44475aaa", "rounding": 10,
                "angle": 135},
    "glass": {"gradient": False, "active1": "#ffffff66", "inactive": "#ffffff22", "rounding": 18, "gaps_in": 8,
              "gaps_out": 22, "active_opacity": 0.9, "inactive_opacity": 0.78, "blur_size": 8, "blur_passes": 3,
              "blur_vibrancy": 0.3, "shadow_range": 18, "anim_curve": "bouncy"},
    "everforest": {"active1": "#a7c080ff", "active2": "#83c092ff", "inactive": "#475258aa", "rounding": 10,
                   "gaps_out": 14},
    "kanagawa": {"active1": "#7e9cd8ff", "active2": "#957fb8ff", "inactive": "#363646aa", "rounding": 8, "angle": 90},
    "onedark": {"active1": "#61afefff", "active2": "#c678ddff", "inactive": "#3e4451aa", "rounding": 10},
    "solarized": {"active1": "#268bd2ff", "active2": "#2aa198ff", "inactive": "#073642aa", "rounding": 6,
                  "gaps_in": 4, "gaps_out": 10},
    "monokai": {"active1": "#a6e22eff", "active2": "#f92672ff", "inactive": "#49483eaa", "rounding": 6, "angle": 135},
    "neon": {"active1": "#ff2bd6ff", "active2": "#00e5ffff", "inactive": "#1a1a2eaa", "border_size": 3, "rounding": 12,
             "border_rotate": True, "shadow_color": "#ff2bd666", "shadow_range": 20, "anim_curve": "bouncy"},
    "sharp": {"gradient": False, "active1": "#e6edf5ff", "inactive": "#30363dff", "border_size": 3, "rounding": 0,
              "gaps_in": 3, "gaps_out": 6, "shadow": False, "anim_curve": "fast"},
    "bubbly": {"active1": "#ffb4a2ff", "active2": "#b5838dff", "inactive": "#4a3f4baa", "rounding": 24, "gaps_in": 10,
               "gaps_out": 24, "rounding_power": 4.0, "anim_curve": "bouncy", "shadow_range": 16},
    "mono": {"gradient": False, "active1": "#ffffffcc", "inactive": "#ffffff1a", "border_size": 1, "rounding": 8,
             "inactive_opacity": 0.85, "dim_inactive": True, "dim_strength": 0.2},
    "minimal": {"gradient": False, "active1": "#3d8bfdff", "inactive": "#2a2f38ff", "border_size": 1,
                "gaps_in": 0, "gaps_out": 0, "rounding": 0, "blur": False, "shadow": False, "anim": False},
}
HYPR_PRESETS = palettes.unify(_HYPR_PRESETS_OWN, lambda p: {"active1": p["accent"] + "ff", "active2": p["accent2"] + "ff", "inactive": p["muted"] + "aa"})  # gemeinsame Namen/Farben

CURVES = {
    "smooth": ((0.23, 1.0), (0.32, 1.0)),
    "fast": ((0.15, 0.0), (0.1, 1.0)),
    "bouncy": ((0.34, 1.56), (0.64, 1.0)),
    "linear": ((0.0, 0.0), (1.0, 1.0)),
}
# Grund-Dauer in Dezisekunden (wie im Hyprland-Beispiel)
ANIMS = (
    ("windowsIn", 4.1, "win"), ("windowsOut", 1.5, "win"), ("windowsMove", 4.8, None),
    ("border", 5.4, None), ("fade", 3.0, None), ("layers", 3.8, None), ("workspaces", 3.0, "ws"),
)
WIN_STYLES = {"popin": "popin 80%", "slide": "slide", "gnomed": "gnomed"}
WS_STYLES = {"slide": "slide", "slidevert": "slidevert", "fade": "fade", "slidefade": "slidefade 20%"}


def hypr_state(saved: dict | None) -> dict:
    s = dict(HYPR_DEFAULT)
    s["binds"], s["autostart"], s["skip_opts"], s["workspaces"] = [], [], [], []
    for k, v in (saved or {}).items():
        if k not in s:
            continue
        if k == "binds":
            s[k] = clean_binds(v)
        elif k == "workspaces":
            s[k] = clean_workspaces(v)
        elif k in ("autostart", "skip_opts"):
            s[k] = [x for x in v if isinstance(x, str)] if isinstance(v, list) else []
        elif type(v) is type(s[k]):
            s[k] = v
        elif isinstance(s[k], float) and type(v) is int:
            s[k] = float(v)
    return s


def preset_state(name: str, base: dict | None = None) -> dict:
    """Preset-Look; Verhalten (Layout, Eingabe, Kürzel …) kommt aus base."""
    s = hypr_state(base)
    for k in LOOK_KEYS:
        s[k] = HYPR_DEFAULT[k]
    s.update(HYPR_PRESETS.get(name, {}))
    return s


# ---------- Arbeitsflächen ----------

WS_FIELDS = {"id": 1, "name": "", "monitor": "", "gaps_in": -1, "gaps_out": -1, "border": -1, "persistent": False,
             "default": False, "no_rounding": False, "layout": "", "on_empty": ""}


def clean_workspaces(v) -> list:
    out, seen = [], set()
    for w in v if isinstance(v, list) else []:
        if not isinstance(w, dict):
            continue
        d = dict(WS_FIELDS)
        for k, default in WS_FIELDS.items():
            x = w.get(k, default)
            if type(x) is type(default):
                d[k] = x.strip() if isinstance(x, str) else x
        if 1 <= d["id"] <= 10 and d["id"] not in seen:
            seen.add(d["id"])
            out.append(d)
    return sorted(out, key=lambda x: x["id"])


def default_workspaces() -> list:
    return [dict(WS_FIELDS, id=i) for i in range(1, 11)]


def ws_used(w: dict) -> bool:
    """Hat die Arbeitsfläche irgendeine eigene Einstellung?"""
    return any(w[k] != WS_FIELDS[k] for k in WS_FIELDS if k != "id")


def ws_lua(w: dict) -> str:
    parts = [f'workspace = "{w["id"]}"']
    if w["name"]:
        parts.append(f"default_name = {_lua_str(w['name'])}")
    if w["monitor"]:
        parts.append(f"monitor = {_lua_str(w['monitor'])}")
    if w["default"]:
        parts.append("default = true")
    if w["persistent"]:
        parts.append("persistent = true")
    for k, lk in (("gaps_in", "gaps_in"), ("gaps_out", "gaps_out"), ("border", "border_size")):
        if w[k] >= 0:
            parts.append(f"{lk} = {w[k]}")
    if w["no_rounding"]:
        parts.append("no_rounding = true")
    if w["layout"]:
        parts.append(f"layout = {_lua_str(w['layout'])}")
    if w["on_empty"]:
        parts.append(f"on_created_empty = {_lua_str(w['on_empty'])}")
    return "hl.workspace_rule({ " + ", ".join(parts) + " })"


def ws_conf(w: dict) -> str:
    parts = [str(w["id"])]
    if w["name"]:
        parts.append(f"defaultName:{w['name']}")
    if w["monitor"]:
        parts.append(f"monitor:{w['monitor']}")
    if w["default"]:
        parts.append("default:true")
    if w["persistent"]:
        parts.append("persistent:true")
    for k, ck in (("gaps_in", "gapsin"), ("gaps_out", "gapsout"), ("border", "bordersize")):
        if w[k] >= 0:
            parts.append(f"{ck}:{w[k]}")
    if w["no_rounding"]:
        parts.append("rounding:false")
    if w["layout"]:
        parts.append(f"layout:{w['layout']}")
    if w["on_empty"]:
        parts.append(f"on-created-empty:{w['on_empty']}")
    return "workspace = " + ", ".join(parts)


# ---------- Tastenkürzel ----------

MODS = ("SUPER", "SHIFT", "CTRL", "ALT")
MOD_ALIAS = {"SUPER": "SUPER", "WIN": "SUPER", "MOD4": "SUPER", "META": "SUPER", "LOGO": "SUPER",
             "SHIFT": "SHIFT", "CTRL": "CTRL", "CONTROL": "CTRL", "ALT": "ALT", "MOD1": "ALT"}
# Aktion → Art des Arguments (None = keins)
ACTIONS = {
    "exec": "cmd", "launcher": "launcher", "close": None, "kill": None, "fullscreen": None, "float": None, "pseudo": None,
    "togglesplit": None, "focus": "dir", "movewin": "dir", "workspace": "ws", "movetows": "ws",
    "wsnext": None, "wsprev": None, "special": None, "movespecial": None, "drag": None, "resize": None,
    "exit": None,
}
MOUSE_ACTIONS = ("drag", "resize")
# Aktion „Launcher öffnen“: Auswahl im Kürzel → Befehl
LAUNCHER_CMDS = {"llauncher": "l-launcher", "rofi": "rofi -show drun", "wofi": "wofi --show drun", "fuzzel": "fuzzel",
                 "walker": "walker", "tofi": "tofi-drun --drun-launch=true"}


LAUNCHER_TOGGLE = True  # zweites Drücken der Launcher-Taste schließt ihn (Launcher-Tab › Verhalten)


def launcher_exec(key: str) -> str:
    """Befehl fürs Kürzel; mit Umschalten: läuft der Launcher schon, wird er geschlossen.
    Der L Launcher schaltet selbst um (merkt sich seine Prozess-ID)."""
    cmd = LAUNCHER_CMDS.get(key, "")
    if LAUNCHER_TOGGLE and cmd and key != "llauncher":
        return f"pkill -x {cmd.split()[0]} || {cmd}"
    return cmd


def _strip_toggle(cmd: str) -> str:
    """„pkill -x rofi || rofi -show drun“ → „rofi -show drun“."""
    c = (cmd or "").strip()
    if c.startswith("pkill -x ") and "||" in c:
        return c.split("||", 1)[1].strip()
    return c


def bind_cmd(b: dict) -> str:
    """Befehl, den ein Kürzel startet („Launcher öffnen“ → Befehl des gewählten Launchers)."""
    if b.get("action") == "launcher":
        return LAUNCHER_CMDS.get(b.get("arg", ""), "")
    return b.get("arg", "") if b.get("action") == "exec" else ""


def as_launcher(action: str, arg: str) -> tuple:
    """Exec eines bekannten Launchers → Aktion „Launcher öffnen“ (für Import aus der Config)."""
    if action == "exec":
        for k, c in LAUNCHER_CMDS.items():
            if _strip_toggle(arg) == c:
                return "launcher", k
    return action, arg
FLAGS = ("locked", "repeating", "release")
DIRECTIONS = ("left", "right", "up", "down")
EXEC_SUGGEST = ("kitty", "foot", "alacritty", "dolphin", "nautilus", "thunar", "firefox", "chromium",
                "rofi -show drun", "wofi --show drun", "fuzzel", "hyprlauncher", "hyprlock", "wlogout",
                'grim -g "$(slurp)" - | wl-copy', "grim - | wl-copy", "hyprpicker -a", "pavucontrol", "code",
                "cliphist list | rofi -dmenu | cliphist decode | wl-copy", "swaync-client -t")
SPECIAL_KEYS = ("SUPER_L", "mouse:272", "mouse:273", "mouse_down", "mouse_up", "Print", "space", "Return", "Tab", "Escape",
                "XF86AudioRaiseVolume", "XF86AudioLowerVolume", "XF86AudioMute", "XF86AudioMicMute",
                "XF86AudioPlay", "XF86AudioNext", "XF86AudioPrev", "XF86MonBrightnessUp", "XF86MonBrightnessDown",
                "XF86PowerOff", "XF86Calculator")


def _bind(mods, key, action, arg="", flags=()) -> dict:
    return {"mods": list(mods), "key": key, "action": action, "arg": arg, "flags": list(flags)}


def clean_binds(v) -> list:
    out = []
    if not isinstance(v, list):
        return out
    for b in v:
        if not isinstance(b, dict) or b.get("action") not in ACTIONS:
            continue
        mods = [m for m in MODS if m in (b.get("mods") or [])]
        flags = [f for f in FLAGS if f in (b.get("flags") or [])]
        out.append(_bind(mods, str(b.get("key") or "").strip(), b["action"], str(b.get("arg") or "").strip(), flags))
    return out


LR = ("locked", "repeating")
TEMPLATES = {
    "basics": [
        _bind(["SUPER"], "Q", "exec", "kitty"), _bind(["SUPER"], "C", "close"), _bind(["SUPER"], "E", "exec", "dolphin"),
        _bind(["SUPER"], "R", "launcher", "rofi"), _bind(["SUPER"], "V", "float"),
        _bind(["SUPER"], "F", "fullscreen"), _bind(["SUPER"], "J", "togglesplit"), _bind(["SUPER"], "P", "pseudo"),
        _bind(["SUPER"], "L", "exec", "hyprlock"), _bind(["SUPER"], "M", "exit"),
    ],
    "focus": [_bind(["SUPER"], d, "focus", d) for d in DIRECTIONS],
    "move": [_bind(["SUPER", "SHIFT"], d, "movewin", d) for d in DIRECTIONS],
    "workspaces": [_bind(["SUPER"], str(i % 10), "workspace", str(i)) for i in range(1, 11)]
    + [_bind(["SUPER", "SHIFT"], str(i % 10), "movetows", str(i)) for i in range(1, 11)],
    "special": [_bind(["SUPER"], "S", "special"), _bind(["SUPER", "SHIFT"], "S", "movespecial")],
    "mouse": [_bind(["SUPER"], "mouse:272", "drag"), _bind(["SUPER"], "mouse:273", "resize"),
              _bind(["SUPER"], "mouse_down", "wsnext"), _bind(["SUPER"], "mouse_up", "wsprev")],
    "media": [
        _bind([], "XF86AudioRaiseVolume", "exec", "wpctl set-volume -l 1 @DEFAULT_AUDIO_SINK@ 5%+", LR),
        _bind([], "XF86AudioLowerVolume", "exec", "wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-", LR),
        _bind([], "XF86AudioMute", "exec", "wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle", LR),
        _bind([], "XF86AudioMicMute", "exec", "wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle", LR),
        _bind([], "XF86MonBrightnessUp", "exec", "brightnessctl -e4 -n2 set 5%+", LR),
        _bind([], "XF86MonBrightnessDown", "exec", "brightnessctl -e4 -n2 set 5%-", LR),
        _bind([], "XF86AudioNext", "exec", "playerctl next", ["locked"]),
        _bind([], "XF86AudioPlay", "exec", "playerctl play-pause", ["locked"]),
        _bind([], "XF86AudioPrev", "exec", "playerctl previous", ["locked"]),
    ],
    "screenshot": [_bind([], "Print", "exec", 'grim -g "$(slurp)" - | wl-copy'),
                   _bind(["SHIFT"], "Print", "exec", "grim - | wl-copy"),
                   _bind(["SUPER", "SHIFT"], "C", "exec", "hyprpicker -a")],
}


def default_binds() -> list:
    return [dict(b) for k in ("basics", "focus", "workspaces", "special", "mouse") for b in TEMPLATES[k]]


SUPER_ALONE = ("SUPER_L", "SUPER_R")


def bind_flags(b: dict) -> list:
    """Flags des Kürzels; Super allein braucht immer „release“, sonst würde jedes Super + X es auslösen."""
    fl = [f for f in FLAGS if f in b.get("flags", [])]
    if b.get("key") in SUPER_ALONE and "release" not in fl:
        fl.append("release")
    return fl


def combo(mods, key: str) -> tuple:
    return tuple(sorted({MOD_ALIAS.get(m.upper(), m.upper()) for m in mods})), key.strip().lower()


def combo_text(b: dict) -> str:
    return " + ".join([*b["mods"], b["key"]])


def bind_ok(b: dict) -> bool:
    kind = ACTIONS[b["action"]]
    if not b["key"] or (kind and not b["arg"]):
        return False
    if kind == "dir" and b["arg"] not in DIRECTIONS:
        return False
    if kind == "launcher" and b["arg"] not in LAUNCHER_CMDS:
        return False
    return True


def _ws(arg: str) -> str:
    return arg if arg.lstrip("-").isdigit() else _lua_str(arg)


def bind_lua(b: dict) -> str:
    a, g = b["action"], b["arg"]
    dsp = {
        "exec": lambda: f"hl.dsp.exec_cmd({_lua_str(g)})",
        "launcher": lambda: f"hl.dsp.exec_cmd({_lua_str(launcher_exec(g))})",
        "close": lambda: "hl.dsp.window.close()",
        "kill": lambda: "hl.dsp.window.kill()",
        "fullscreen": lambda: 'hl.dsp.window.fullscreen({ action = "toggle" })',
        "float": lambda: 'hl.dsp.window.float({ action = "toggle" })',
        "pseudo": lambda: "hl.dsp.window.pseudo()",
        "togglesplit": lambda: 'hl.dsp.layout("togglesplit")',
        "focus": lambda: f'hl.dsp.focus({{ direction = "{g}" }})',
        "movewin": lambda: f'hl.dsp.window.move({{ direction = "{g}" }})',
        "workspace": lambda: f"hl.dsp.focus({{ workspace = {_ws(g)} }})",
        "movetows": lambda: f"hl.dsp.window.move({{ workspace = {_ws(g)} }})",
        "wsnext": lambda: 'hl.dsp.focus({ workspace = "e+1" })',
        "wsprev": lambda: 'hl.dsp.focus({ workspace = "e-1" })',
        "special": lambda: 'hl.dsp.workspace.toggle_special("magic")',
        "movespecial": lambda: 'hl.dsp.window.move({ workspace = "special:magic" })',
        "drag": lambda: "hl.dsp.window.drag()",
        "resize": lambda: "hl.dsp.window.resize()",
        "exit": lambda: "hl.dsp.exit()",
    }[a]()
    flags = bind_flags(b) + (["mouse"] if a in MOUSE_ACTIONS else [])
    extra = (", { " + ", ".join(f"{f} = true" for f in flags) + " }") if flags else ""
    return f"hl.bind({_lua_str(combo_text(b))}, {dsp}{extra})"


CONF_DSP = {"close": "killactive,", "kill": "forcekillactive,", "fullscreen": "fullscreen, 0",
            "float": "togglefloating,", "pseudo": "pseudo,", "togglesplit": "togglesplit,",
            "wsnext": "workspace, e+1", "wsprev": "workspace, e-1", "special": "togglespecialworkspace, magic",
            "movespecial": "movetoworkspace, special:magic", "drag": "movewindow", "resize": "resizewindow",
            "exit": "exit,"}


def bind_conf(b: dict) -> list:
    a, g = b["action"], b["arg"]
    d = CONF_DSP.get(a) or {"exec": f"exec, {g}", "launcher": f"exec, {launcher_exec(g)}", "focus": f"movefocus, {g[:1]}", "movewin": f"movewindow, {g[:1]}",
                            "workspace": f"workspace, {g}", "movetows": f"movetoworkspace, {g}"}[a]
    mods = " ".join(b["mods"])
    letters = "".join(c for f, c in (("locked", "l"), ("repeating", "e"), ("release", "r")) if f in bind_flags(b))
    kind = "bindm" if a in MOUSE_ACTIONS else "bind" + letters
    # unbind entfernt vorher gesetzte Kürzel mit derselben Taste (aus hyprland.conf)
    return [f"unbind = {mods}, {b['key']}", f"{kind} = {mods}, {b['key']}, {d}"]


def _call_args(s: str) -> list:
    """Argumente eines Lua-Aufrufs auf oberster Ebene (s beginnt nach der öffnenden Klammer)."""
    depth, q, cur, out = 0, None, [], []
    for ch in s:
        if q:
            cur.append(ch)
            if ch == q:
                q = None
            continue
        if ch in "\"'":
            q = ch
        elif ch in "({[":
            depth += 1
        elif ch in ")}]":
            if depth == 0:
                out.append("".join(cur).strip())
                return out
            depth -= 1
        elif ch == "," and depth == 0:
            out.append("".join(cur).strip())
            cur = []
            continue
        cur.append(ch)
    out.append("".join(cur).strip())
    return out


def _split_args(s: str) -> str:
    return _call_args(s)[0]


def _lua_concat(expr: str, local: dict) -> str | None:
    val = ""
    for part in expr.split(".."):
        part = part.strip()
        if len(part) >= 2 and part[0] == part[-1] and part[0] in "\"'":
            val += part[1:-1]
        elif part in local:
            val += local[part]
        else:
            return None
    return val


def _lua_locals(text: str) -> dict:
    import re
    return dict(re.findall(r'^\s*local\s+(\w+)\s*=\s*"([^"]*)"', text, re.M))


def lua_bind_combos(text: str) -> list:
    """[(zeilenindex, combo)] aller einfach lesbaren hl.bind-Zeilen (Variablen wie mainMod werden aufgelöst)."""
    import re
    local = _lua_locals(text)
    out = []
    for i, ln in enumerate(text.splitlines()):
        s = ln.strip()
        if s.startswith("--"):
            continue
        m = re.match(r'(?:local\s+\w+\s*=\s*)?hl\.bind\((.*)$', s)
        if not m:
            continue
        val = _lua_concat(_split_args(m.group(1)), local)
        if val is None:
            continue
        keys = [x.strip() for x in val.split("+") if x.strip()]
        if keys:
            out.append((i, combo(keys[:-1], keys[-1])))
    return out


def _parse_dsp(dsp: str, local: dict) -> tuple | None:
    """Lua-Dispatcher → (aktion, argument) oder None, wenn unbekannt."""
    import re
    d = " ".join(dsp.split())
    m = re.fullmatch(r"hl\.dsp\.exec_cmd\((.*)\)", d)
    if m:
        cmd = _lua_concat(_call_args(m.group(1))[0], local)
        if cmd is None:
            return None
        try:  # Lua-Escapes wie \" auflösen
            cmd = json.loads(f'"{cmd}"')
        except ValueError:
            pass
        return as_launcher("exec", cmd)
    simple = {"hl.dsp.window.close()": "close", "hl.dsp.window.kill()": "kill", "hl.dsp.window.pseudo()": "pseudo",
              "hl.dsp.window.drag()": "drag", "hl.dsp.window.resize()": "resize", "hl.dsp.exit()": "exit",
              'hl.dsp.layout("togglesplit")': "togglesplit"}
    if d in simple:
        return simple[d], ""
    if d.startswith("hl.dsp.window.fullscreen("):
        return "fullscreen", ""
    if d.startswith("hl.dsp.window.float("):
        return "float", ""
    if d.startswith("hl.dsp.workspace.toggle_special("):
        return "special", ""
    m = re.fullmatch(r'hl\.dsp\.(focus|window\.move)\(\{\s*(direction|workspace)\s*=\s*("?)([^"}\s]+)\3\s*\}\)', d)
    if m:
        kind, field, _, val = m.groups()
        if field == "direction" and val in DIRECTIONS:
            return ("focus" if kind == "focus" else "movewin"), val
        if field == "workspace":
            if val in ("e+1", "e-1") and kind == "focus":
                return ("wsnext" if val == "e+1" else "wsprev"), ""
            if val == "special:magic" and kind != "focus":
                return "movespecial", ""
            if val.lstrip("-").isdigit() or val.startswith("name:"):
                return ("workspace" if kind == "focus" else "movetows"), val
    return None


def _flags(tbl: str | None) -> list:
    import re
    if not tbl:
        return []
    on = set(re.findall(r"(\w+)\s*=\s*true", tbl))
    return [f for f in FLAGS if f in on]


def import_lua(text: str) -> tuple:
    """Liest hl.bind-Zeilen (auch die übliche for-Schleife für Arbeitsflächen).
    Gibt (binds, zeilen_zum_auskommentieren) zurück; Unbekanntes bleibt unangetastet."""
    import re
    local = _lua_locals(text)
    lines = text.splitlines()
    binds, idx = [], set()

    def one(line: str, loc: dict):
        s = line.strip()
        m = re.match(r'(?:local\s+\w+\s*=\s*)?hl\.bind\((.*)$', s)
        if not m or s.startswith("--"):
            return None
        args = _call_args(m.group(1))
        if len(args) < 2:
            return None
        val = _lua_concat(args[0], loc)
        dsp = _parse_dsp(args[1], loc)
        if val is None or dsp is None:
            return None
        keys = [x.strip() for x in val.split("+") if x.strip()]
        if not keys:
            return None
        mods = [MOD_ALIAS[k.upper()] for k in keys[:-1] if k.upper() in MOD_ALIAS]
        if len(mods) != len(keys) - 1:
            return None
        return _bind([m for m in MODS if m in mods], keys[-1], dsp[0], dsp[1], _flags(args[2] if len(args) > 2 else None))

    i = 0
    while i < len(lines):
        ln = lines[i]
        loop = re.match(r"^(\s*)for\s+(\w+)\s*=\s*1\s*,\s*(\d+)\s+do\s*$", ln)
        if loop:
            ind, var, n = loop.group(1), loop.group(2), int(loop.group(3))
            j = i + 1
            while j < len(lines) and not re.match(rf"^{re.escape(ind)}end\b", lines[j]):
                j += 1
            body = lines[i + 1:j]
            found, ok = [], j < len(lines)
            for k in range(1, n + 1):
                loc = dict(local)
                loc[var] = str(k)
                for bl in body:
                    km = re.match(r"^\s*local\s+(\w+)\s*=\s*" + re.escape(var) + r"\s*%\s*10\s*(--.*)?$", bl)
                    if km:
                        loc[km.group(1)] = str(k % 10)
                for bl in body:
                    if "hl.bind(" not in bl:
                        continue
                    b = one(re.sub(rf"\b{re.escape(var)}\b(?=\s*[}}])", str(k), bl), loc)
                    if b is None:
                        ok = False
                    else:
                        found.append(b)
            if ok and found:
                binds += found
                idx.update(range(i, j + 1))
            i = j + 1
            continue
        b = one(ln, local)
        if b:
            binds.append(b)
            idx.add(i)
        i += 1
    return binds, sorted(idx)


def import_conf(text: str) -> tuple:
    import re
    var = dict(re.findall(r"^\s*\$(\w+)\s*=\s*(.*?)\s*$", text, re.M))
    rev = {"killactive": "close", "forcekillactive": "kill", "togglefloating": "float", "fullscreen": "fullscreen",
           "pseudo": "pseudo", "togglesplit": "togglesplit", "exit": "exit",
           "togglespecialworkspace": "special"}
    dmap = {"l": "left", "r": "right", "u": "up", "d": "down"}
    binds, idx = [], []
    for i, ln in enumerate(text.splitlines()):
        m = re.match(r"^\s*bind([a-z]*)\s*=\s*(.*)$", ln)
        if not m:
            continue
        flags, rest = m.group(1), re.sub(r"\$(\w+)", lambda x: var.get(x.group(1), x.group(0)), m.group(2))
        parts = [p.strip() for p in rest.split(",", 3)]
        if len(parts) < 3:
            continue
        mods_raw, key, d = parts[0], parts[1], parts[2]
        arg = parts[3] if len(parts) > 3 else ""
        mods = [MOD_ALIAS.get(x.upper()) for x in mods_raw.replace("_", " ").split()]
        if None in mods or not key:
            continue
        act = None
        if flags == "m":
            act, arg = {"movewindow": ("drag", ""), "resizewindow": ("resize", "")}.get(d, (None, ""))
        elif d == "exec" and arg:
            act, arg = as_launcher("exec", arg)
        elif d in rev and (d != "togglespecialworkspace" or arg in ("", "magic")):
            act, arg = rev[d], ""
        elif d == "layoutmsg" and arg == "togglesplit":
            act, arg = "togglesplit", ""
        elif d in ("movefocus", "movewindow") and arg in dmap:
            act, arg = ("focus" if d == "movefocus" else "movewin"), dmap[arg]
        elif d == "workspace" and arg in ("e+1", "e-1"):
            act, arg = ("wsnext" if arg == "e+1" else "wsprev"), ""
        elif d in ("workspace", "movetoworkspace") and arg == "special:magic" and d == "movetoworkspace":
            act, arg = "movespecial", ""
        elif d in ("workspace", "movetoworkspace") and arg.lstrip("-").isdigit():
            act = "workspace" if d == "workspace" else "movetows"
        if not act or any(c not in "ler" for c in flags.replace("m", "")):
            continue
        fl = [f for f, c in (("locked", "l"), ("repeating", "e"), ("release", "r")) if c in flags]
        binds.append(_bind([x for x in MODS if x in mods], key, act, arg, fl))
        idx.append(i)
    return binds, idx


def _comment_lines(main: Path, idx, prefix: str, log) -> int:
    idx = sorted(set(idx))
    if not idx:
        return 0
    bak = stamp(main)
    shutil.copy2(main, bak)
    log(f"Sicherung: {bak}")
    lines = main.read_text(encoding="utf-8", errors="replace").splitlines(keepends=True)
    for i in idx:
        lines[i] = prefix + lines[i]
        log(f"  auskommentiert: {lines[i].strip()}")
    main.write_text("".join(lines), encoding="utf-8")
    return len(idx)


def import_binds(hdir: Path, log) -> list:
    """Kürzel aus hyprland.lua/.conf übernehmen und dort auskommentieren (mit Sicherung)."""
    fmt = hypr_format(hdir)
    if not fmt:
        return []
    main = hdir / f"hyprland.{fmt}"
    if not main.is_file():
        return []
    text = main.read_text(encoding="utf-8", errors="replace")
    binds, idx = import_lua(text) if fmt == "lua" else import_conf(text)
    _comment_lines(main, idx, "-- [Hypr Studio] " if fmt == "lua" else "# [Hypr Studio] ", log)
    return binds


def lua_conflicts(main_text: str, binds: list) -> list:
    mine = {combo(b["mods"], b["key"]) for b in binds if bind_ok(b)}
    return [i for i, c in lua_bind_combos(main_text) if c in mine]


LAUNCHERS = ("l-launcher", "rofi", "wofi", "fuzzel", "hyprlauncher", "tofi", "anyrun", "walker", "bemenu-run", "dmenu_run",
             "ulauncher", "nwg-drawer", "albert", "sherlock", "vicinae", "hyprshell")


def is_launcher_cmd(cmd: str) -> bool:
    cmd = _strip_toggle(cmd)
    if (cmd or "").strip().startswith("hypr-studio --launcher"):
        return True
    first = (cmd or "").strip().split(" ", 1)[0]
    return Path(first).name in LAUNCHERS


def launcher_binds(hdir: Path, own: list) -> list:
    """Kürzel, die einen Launcher starten: [(bind, "own"|"config")].
    "config" = steht in hyprland.lua/.conf und wird nicht schon von einem eigenen Kürzel überdeckt."""
    out = [(b, "own") for b in own if is_launcher_cmd(bind_cmd(b))]
    mine = {combo(b["mods"], b["key"]) for b in own if bind_ok(b)}
    fmt = hypr_format(hdir)
    main = hdir / f"hyprland.{fmt}" if fmt else None
    if main and main.is_file():
        text = main.read_text(encoding="utf-8", errors="replace")
        found, _ = import_lua(text) if fmt == "lua" else import_conf(text)
        for b in found:
            if is_launcher_cmd(bind_cmd(b)) and combo(b["mods"], b["key"]) not in mine:
                out.append((b, "config"))
    return out


def comment_conflicts(hdir: Path, binds: list, log) -> int:
    main = hdir / "hyprland.lua"
    if not main.is_file():
        return 0
    text = main.read_text(encoding="utf-8", errors="replace")
    return _comment_lines(main, lua_conflicts(text, binds), "-- [Hypr Studio] ", log)


def anim_list(s: dict) -> list:
    """[(leaf, speed, style)] für die aktuelle Einstellung."""
    tempo = max(0.25, float(s["anim_tempo"]))
    fast = 0.6 if s["anim_curve"] == "fast" else 1.0
    out = []
    for leaf, base, kind in ANIMS:
        speed = round(max(0.5, base * fast / tempo), 2)
        style = WIN_STYLES.get(s["win_style"]) if kind == "win" else WS_STYLES.get(s["ws_style"]) if kind == "ws" else None
        out.append((leaf, speed, style))
    return out


def _lua_str(v: str) -> str:
    return json.dumps(v, ensure_ascii=False)


def _num(v) -> str:
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, float):
        return f"{v:.4f}".rstrip("0").rstrip(".") if v != int(v) else f"{v:.1f}"
    return str(v)


def _lua_val(v) -> str:
    if isinstance(v, str):
        return _lua_str(v)
    return _num(v)


class _Raw(str):
    """Wert, der unverändert in Lua landet (z. B. eine Verlaufs-Tabelle)."""


def _lua_table(d: dict, ind: int = 1) -> list:
    """Python-Dict → Lua-Tabellenzeilen (verschachtelt, Reihenfolge bleibt)."""
    pad = "    " * ind
    out = []
    w = max((len(k) for k, v in d.items() if not isinstance(v, dict)), default=0)
    for k, v in d.items():
        if isinstance(v, dict):
            out.append(f"{pad}{k} = {{")
            out += _lua_table(v, ind + 1)
            out.append(f"{pad}}},")
        elif isinstance(v, _Raw):
            out.append(f"{pad}{k.ljust(w)} = {v},")
        else:
            out.append(f"{pad}{k.ljust(w)} = {_lua_val(v)},")
    return out


def _conf_block(name: str, d: dict, ind: int = 0) -> list:
    pad = "    " * ind
    out = [f"{pad}{name} {{"]
    for k, v in d.items():
        if isinstance(v, dict):
            out += _conf_block(k, v, ind + 1)
        else:
            out.append(f"{pad}    {k} = {v if isinstance(v, str) else _num(v)}")
    out.append(f"{pad}}}")
    return out


def _drop(d: dict, parts: list) -> None:
    if not parts:
        return
    d.pop(".".join(parts), None)  # hyprlang: col.active_border steht als ein Schlüssel
    if len(parts) > 1 and isinstance(d.get(parts[0]), dict):
        _drop(d[parts[0]], parts[1:])


def _sections(s: dict, lua: bool) -> dict:
    sec = _sections_all(s, lua)
    for path in s["skip_opts"]:
        _drop(sec, path.split("."))
    return sec


def _sections_all(s: dict, lua: bool) -> dict:
    """Gemeinsamer Inhalt für Lua und hyprlang: {kategorie: {option: wert}}."""
    f = float
    if s["gradient"]:
        if lua:
            active = _Raw(f'{{ colors = {{{_lua_str(rgba_hypr(s["active1"]))}, {_lua_str(rgba_hypr(s["active2"]))}}}, '
                          f'angle = {int(s["angle"])} }}')
        else:
            active = f"{rgba_hypr(s['active1'])} {rgba_hypr(s['active2'])} {int(s['angle'])}deg"
    else:
        active = rgba_hypr(s["active1"])
    general = {"gaps_in": s["gaps_in"], "gaps_out": s["gaps_out"], "border_size": s["border_size"]}
    if lua:
        general["col"] = {"active_border": active, "inactive_border": rgba_hypr(s["inactive"])}
    else:
        general["col.active_border"] = active
        general["col.inactive_border"] = rgba_hypr(s["inactive"])
    general["layout"] = s["layout"]
    general["resize_on_border"] = s["resize_on_border"]
    return {
        "general": general,
        "decoration": {
            "rounding": s["rounding"], "rounding_power": f(s["rounding_power"]),
            "active_opacity": f(s["active_opacity"]), "inactive_opacity": f(s["inactive_opacity"]),
            "dim_inactive": s["dim_inactive"], "dim_strength": f(s["dim_strength"]),
            "blur": {"enabled": s["blur"], "size": s["blur_size"], "passes": s["blur_passes"],
                     "vibrancy": f(s["blur_vibrancy"]), "noise": f(s["blur_noise"]),
                     "contrast": f(s["blur_contrast"]), "brightness": f(s["blur_brightness"]),
                     "xray": s["blur_xray"], "ignore_opacity": s["blur_ignore_opacity"]},
            "shadow": {"enabled": s["shadow"], "range": s["shadow_range"], "render_power": s["shadow_power"],
                       "sharp": s["shadow_sharp"], "color": rgba_hypr(s["shadow_color"])},
        },
        "animations": {"enabled": s["anim"]},
        "dwindle": {"preserve_split": s["dwindle_preserve"], "force_split": s["dwindle_force"],
                    "smart_split": s["dwindle_smart"]},
        "master": {"new_status": s["master_new"], "mfact": f(s["master_mfact"]), "orientation": s["master_orient"]},
        "input": {
            "kb_layout": s["kb_layout"], "kb_variant": s["kb_variant"], "kb_options": s["kb_options"],
            "numlock_by_default": s["numlock"], "repeat_rate": s["repeat_rate"], "repeat_delay": s["repeat_delay"],
            "follow_mouse": s["follow_mouse"], "sensitivity": f(s["sensitivity"]),
            "accel_profile": "" if s["accel"] == "default" else s["accel"], "left_handed": s["left_handed"],
            "natural_scroll": s["mouse_natural"],
            "touchpad": {"natural_scroll": s["natural_scroll"], "disable_while_typing": s["dwt"],
                         "scroll_factor": f(s["scroll_factor"])},
        },
        "cursor": {"inactive_timeout": f(s["cursor_timeout"]), "hide_on_key_press": s["cursor_hide_keys"]},
        "misc": {
            "disable_hyprland_logo": s["no_logo"], "disable_splash_rendering": s["no_logo"],
            "force_default_wallpaper": 0 if s["no_logo"] else -1,
            "background_color": rgba_hypr(s["bg_color"]), "focus_on_activate": s["focus_on_activate"],
            "enable_swallow": s["swallow"], "swallow_regex": s["swallow_regex"],
            "middle_click_paste": s["mmb_paste"], "mouse_move_enables_dpms": s["dpms_mouse"],
            "key_press_enables_dpms": s["dpms_key"],
        },
    }


def _anim_plan(s: dict) -> list:
    """[(leaf, an?, speed, kurve, stil)]"""
    off = {"windows": not s["anim_windows"], "fade": not s["anim_fade"], "workspaces": not s["anim_ws"],
           "border": not s["anim_border"], "layers": not s["anim_layers"]}
    out = []
    for leaf, sp, st in anim_list(s):
        root = next(r for r in off if leaf.startswith(r))
        if not off[root]:
            out.append((leaf, True, sp, "hs_curve", st))
    out += [(root, False, 1.0, None, None) for root, is_off in off.items() if is_off]
    if s["border_rotate"] and s["anim_border"]:
        out.append(("borderangle", True, float(s["border_rotate_speed"]), "hs_linear", "loop"))
    return out


WAYBAR_ONCE = "pgrep -x waybar >/dev/null || waybar"


def _is_waybar_cmd(cmd: str) -> bool:
    """Startet der Befehl NUR Waybar (ggf. mit Optionen)? Ketten wie „waybar & hyprpaper“ zählen nicht."""
    import re
    c = cmd.strip()
    return c == WAYBAR_ONCE or bool(re.fullmatch(r"waybar(\s+[^&;|]*)?", c))


def _autostart(s: dict) -> list:
    """Waybar genau einmal: nur über den Schalter, und nur wenn noch keins läuft. Polkit-Agent ebenso über seinen Schalter."""
    cmds = [c.strip() for c in s["autostart"] if c.strip() and not _is_waybar_cmd(c)
            and not (s["polkit_managed"] and _is_polkit_cmd(c))]
    if s["autostart_waybar"]:
        cmds.insert(0, WAYBAR_ONCE)
    if s["polkit_agent"]:
        cmds.insert(0, POLKIT_CMD)
    return cmds


POLKIT_CMD = "systemctl --user start hyprpolkitagent"


def _is_polkit_cmd(cmd: str) -> bool:
    """Startet der Befehl nur den Polkit-Agent (systemd-Dienst oder direkt)?"""
    import re
    c = cmd.strip()
    return bool(re.fullmatch(r"(systemctl\s+--user\s+(re)?start\s+hyprpolkitagent(\.service)?"
                             r"|\S*hyprpolkitagent)(\s*&)?", c))


def foreign_polkit_lines(text: str, fmt: str) -> list:
    """Zeilen in der Hauptdatei, die hyprpolkitagent selbst starten (exec_cmd bzw. exec-once)."""
    import re
    out = []
    for i, ln in enumerate(text.splitlines()):
        st = ln.strip()
        if "hyprpolkitagent" not in st or st.startswith("--" if fmt == "lua" else "#"):
            continue
        if fmt == "lua":
            m = re.search(r"exec_cmd\(\s*([\"'])(.*?)\1", st)
        else:
            m = re.match(r"exec(?:-once)?\s*=\s*()(.*)$", st)
        if m and _is_polkit_cmd(m.group(2)):
            out.append(i)
    return out


def detect_polkit(hdir: Path, s: dict) -> bool:
    """Startet die vorhandene Config den Polkit-Agent schon (Hauptdatei oder Autostart-Liste)?"""
    if any(_is_polkit_cmd(c) for c in s["autostart"]):
        return True
    fmt = hypr_format(hdir)
    main = hdir / f"hyprland.{fmt}" if fmt else None
    if not (main and main.is_file()):
        return False
    return bool(foreign_polkit_lines(main.read_text(encoding="utf-8", errors="replace"), fmt))


def manage_polkit_autostart(hdir: Path, s: dict, log) -> int:
    """Schalter an: eigene Starts in hyprland.lua/.conf auskommentieren, damit der Agent nicht doppelt startet."""
    if not (s.get("polkit_managed") and s.get("polkit_agent")):
        return 0
    fmt = hypr_format(hdir)
    main = hdir / f"hyprland.{fmt}" if fmt else None
    if not (main and main.is_file()):
        return 0
    idx = foreign_polkit_lines(main.read_text(encoding="utf-8", errors="replace"), fmt)
    return _comment_lines(main, idx, "-- [Hypr Studio] " if fmt == "lua" else "# [Hypr Studio] ", log)


def foreign_waybar_lines(text: str, fmt: str) -> tuple:
    """(zeilen_nur_waybar, zeilen_mit_waybar_in_kette) im Autostart der Hauptdatei."""
    import re
    only, chained = [], []
    for i, ln in enumerate(text.splitlines()):
        s = ln.strip()
        if fmt == "lua":
            if s.startswith("--") or "exec_cmd" not in s or "waybar" not in s:
                continue
            m = re.search(r"exec_cmd\(\s*([\"'])(.*?)\1", s)
        else:
            if s.startswith("#") or not re.match(r"exec(-once)?\s*=", s) or "waybar" not in s:
                continue
            m = re.match(r"exec(?:-once)?\s*=\s*()(.*)$", s)
        cmd = (m.group(2) if m else "").strip()
        if not re.search(r"\bwaybar\b", cmd):
            continue
        (only if _is_waybar_cmd(cmd) else chained).append(i)
    return only, chained


def manage_waybar_autostart(hdir: Path, s: dict, log) -> int:
    """Hypr Studio steuert den Waybar-Autostart allein (sobald der Schalter benutzt wurde):
    eigene Waybar-Starts in hyprland.lua/.conf auskommentieren, waybar.service (systemd) abschalten."""
    if not s.get("waybar_managed"):
        return 0
    n = 0
    fmt = hypr_format(hdir)
    main = hdir / f"hyprland.{fmt}" if fmt else None
    if main and main.is_file():
        only, chained = foreign_waybar_lines(main.read_text(encoding="utf-8", errors="replace"), fmt)
        n = _comment_lines(main, only, "-- [Hypr Studio] " if fmt == "lua" else "# [Hypr Studio] ", log)
        lines = main.read_text(encoding="utf-8", errors="replace").splitlines()
        for i in chained:
            log(f"Hinweis: startet Waybar zusätzlich (bitte selbst anpassen): {main.name}:{i + 1}: {lines[i].strip()}")
    if which("systemctl") and run(["systemctl", "--user", "is-enabled", "--quiet", "waybar.service"], log, quiet=True):
        run(["systemctl", "--user", "disable", "waybar.service"], log)
        n += 1
    return n


# ---------- Laptop-Tasten (Helligkeit, Lautstärke, Stumm, Mikrofon, Medien) ----------

LAPTOP_KEYS = ("XF86MonBrightnessUp", "XF86MonBrightnessDown", "XF86AudioRaiseVolume", "XF86AudioLowerVolume",
               "XF86AudioMute", "XF86AudioMicMute", "XF86AudioPlay", "XF86AudioPause", "XF86AudioNext",
               "XF86AudioPrev")
VOLUME_CMDS = {  # (lauter, leiser, stumm, Mikrofon stumm)
    "wpctl": ("wpctl set-volume -l 1 @DEFAULT_AUDIO_SINK@ 5%+", "wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-",
              "wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle", "wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle"),
    "pactl": ("pactl set-sink-volume @DEFAULT_SINK@ +5%", "pactl set-sink-volume @DEFAULT_SINK@ -5%",
              "pactl set-sink-mute @DEFAULT_SINK@ toggle", "pactl set-source-mute @DEFAULT_SOURCE@ toggle"),
}
BRIGHT_UP, BRIGHT_DOWN = "brightnessctl -e4 -n2 set 5%+", "brightnessctl -e4 -n2 set 5%-"


def laptop_binds(s: dict) -> list:
    """Wie im offiziellen Beispiel: gesperrt (geht auch auf dem Sperrbildschirm) und mit Wiederholung."""
    up, down, mute, mic = VOLUME_CMDS.get(s["vol_tool"], VOLUME_CMDS["wpctl"])
    return [_bind([], "XF86MonBrightnessUp", "exec", BRIGHT_UP, LR),
            _bind([], "XF86MonBrightnessDown", "exec", BRIGHT_DOWN, LR),
            _bind([], "XF86AudioRaiseVolume", "exec", up, LR), _bind([], "XF86AudioLowerVolume", "exec", down, LR),
            _bind([], "XF86AudioMute", "exec", mute, ["locked"]), _bind([], "XF86AudioMicMute", "exec", mic, ["locked"]),
            _bind([], "XF86AudioPlay", "exec", "playerctl play-pause", ["locked"]),
            _bind([], "XF86AudioPause", "exec", "playerctl play-pause", ["locked"]),
            _bind([], "XF86AudioNext", "exec", "playerctl next", ["locked"]),
            _bind([], "XF86AudioPrev", "exec", "playerctl previous", ["locked"])]


def is_laptop_bind(b: dict) -> bool:
    return not b.get("mods") and b.get("key") in LAPTOP_KEYS


def all_binds(s: dict) -> list:
    """Eigene Kürzel; Laptop-Tasten stehen in hyprstudio_laptop.* (laptopkeys.py) – gleiche Tasten fallen hier weg."""
    if not s.get("laptop_keys"):
        return s["binds"]
    return [b for b in s["binds"] if not is_laptop_bind(b)]


def foreign_key_lines(text: str, fmt: str) -> list:
    """Zeilen in der Hauptdatei, die eine der Laptop-Tasten ohne Modifikator selbst belegen."""
    import re
    out = []
    for i, ln in enumerate(text.splitlines()):
        st = ln.strip()
        if fmt == "lua":
            m = re.match(r'hl\.bind\(\s*"(XF86\w+)"', st)
        else:
            m = re.match(r"bind\w*\s*=\s*,\s*(XF86\w+)\s*,", st)
        if m and m.group(1) in LAPTOP_KEYS:
            out.append(i)
    return out


def manage_laptop_keys(hdir: Path, s: dict, log) -> int:
    """Schalter an: gleiche Tasten in hyprland.lua/.conf auskommentieren, sonst lösen sie doppelt aus."""
    if not (s.get("laptop_managed") and s.get("laptop_keys")):
        return 0
    fmt = hypr_format(hdir)
    main = hdir / f"hyprland.{fmt}" if fmt else None
    if not (main and main.is_file()):
        return 0
    idx = foreign_key_lines(main.read_text(encoding="utf-8", errors="replace"), fmt)
    return _comment_lines(main, idx, "-- [Hypr Studio] " if fmt == "lua" else "# [Hypr Studio] ", log)


SMART_GAPS_LUA = '''
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
})'''


L_LAUNCHER_LUA = '''
-- L Launcher (Hypr Studio): schwebt (feste Größe), eigener runder Rahmen
hl.window_rule({
    name  = "hs-l-launcher",
    match = { class = "l-launcher" },
    border_size = 0,
    rounding    = 0,
})'''


def hypr_lua(s: dict) -> str:
    b = _num
    # ein hl.config pro Bereich (wie im offiziellen Beispiel): lehnt Hyprland eine Option ab, fällt nur ihr Bereich weg
    lines = [f"-- {MARK} – automatisch erzeugt, wird beim Anwenden überschrieben"]
    for name, d in _sections(s, True).items():
        lines += ["hl.config({", f"    {name} = {{"] + _lua_table(d, 2) + ["    },", "})"]
    if s["anim"]:
        (x0, y0), (x1, y1) = CURVES[s["anim_curve"]]
        lines += ["", f'hl.curve("hs_curve", {{ type = "bezier", points = {{ {{{b(x0)}, {b(y0)}}}, '
                      f'{{{b(x1)}, {b(y1)}}} }} }})']
        if s["border_rotate"]:
            lines.append('hl.curve("hs_linear", { type = "bezier", points = { {0.0, 0.0}, {1.0, 1.0} } })')
        for leaf, on, speed, curve, style in _anim_plan(s):
            if not on:
                lines.append(f'hl.animation({{ leaf = "{leaf}", enabled = false }})')
                continue
            st = f", style = {_lua_str(style)}" if style else ""
            lines.append(f'hl.animation({{ leaf = "{leaf}", enabled = true, speed = {b(float(speed))}, '
                         f'bezier = "{curve}"{st} }})')
    if s["smart_gaps"]:
        lines += SMART_GAPS_LUA.split("\n")
    wss = [w for w in s["workspaces"] if ws_used(w)] if s["workspaces_on"] else []
    if wss:
        lines += ["", "-- Arbeitsflächen"] + [ws_lua(w) for w in wss]
    if s["gesture"]:
        lines += ["", f'hl.gesture({{ fingers = {s["gesture"]}, direction = "horizontal", action = "workspace" }})']
    lines += ["", f'hl.env("XCURSOR_SIZE", "{s["cursor_size"]}")', f'hl.env("HYPRCURSOR_SIZE", "{s["cursor_size"]}")']
    if s["cursor_theme"]:
        lines += [f'hl.env("XCURSOR_THEME", {_lua_str(s["cursor_theme"])})',
                  f'hl.env("HYPRCURSOR_THEME", {_lua_str(s["cursor_theme"])})']
    if s["qt_theme"]:
        lines.append(f'hl.env("QT_QPA_PLATFORMTHEME", {_lua_str(s["qt_theme"])})')
    if s["monitor_on"]:
        sc = s["monitor_scale"]
        sc = _lua_str(sc) if sc == "auto" else sc
        lines += ["", "hl.monitor({", '    output   = "",', f'    mode     = {_lua_str(s["monitor_mode"])},',
                  '    position = "auto",', f"    scale    = {sc},", "})"]
    cmds = _autostart(s)
    if cmds:
        lines += ["", 'hl.on("hyprland.start", function ()']
        lines += [f"    hl.exec_cmd({_lua_str(c)})" for c in cmds] + ["end)"]
    binds = [x for x in all_binds(s) if bind_ok(x)]
    if binds:
        lines += ["", "-- Tastenkürzel"] + [bind_lua(x) for x in binds]
    lines += L_LAUNCHER_LUA.splitlines()
    return "\n".join(lines) + "\n"


def hypr_conf(s: dict) -> str:
    b = _num
    out = [f"# {MARK} – automatisch erzeugt, wird beim Anwenden überschrieben"]
    sec = _sections(s, False)
    anim = sec.pop("animations")
    for name, d in sec.items():
        out += _conf_block(name, d)
    out += ["animations {", f"    enabled = {b(anim['enabled'])}"]
    if s["anim"]:
        (x0, y0), (x1, y1) = CURVES[s["anim_curve"]]
        out.append(f"    bezier = hs_curve, {b(x0)}, {b(y0)}, {b(x1)}, {b(y1)}")
        if s["border_rotate"]:
            out.append("    bezier = hs_linear, 0.0, 0.0, 1.0, 1.0")
        for leaf, on, speed, curve, style in _anim_plan(s):
            if not on:
                out.append(f"    animation = {leaf}, 0")
            else:
                out.append(f"    animation = {leaf}, 1, {b(float(speed))}, {curve}" + (f", {style}" if style else ""))
    out.append("}")
    if s["smart_gaps"]:
        out += ["workspace = w[tv1], gapsout:0, gapsin:0", "workspace = f[1], gapsout:0, gapsin:0"]
    if s["workspaces_on"]:
        out += [ws_conf(w) for w in s["workspaces"] if ws_used(w)]
    if s["gesture"]:
        out.append(f"gesture = {s['gesture']}, horizontal, workspace")
    out += [f"env = XCURSOR_SIZE,{s['cursor_size']}", f"env = HYPRCURSOR_SIZE,{s['cursor_size']}"]
    if s["cursor_theme"]:
        out += [f"env = XCURSOR_THEME,{s['cursor_theme']}", f"env = HYPRCURSOR_THEME,{s['cursor_theme']}"]
    if s["qt_theme"]:
        out.append(f"env = QT_QPA_PLATFORMTHEME,{s['qt_theme']}")
    if s["monitor_on"]:
        out.append(f"monitor = , {s['monitor_mode']}, auto, {s['monitor_scale']}")
    out += [f"exec-once = {c}" for c in _autostart(s)]
    binds = [x for x in all_binds(s) if bind_ok(x)]
    if binds:
        out += ["", "# Tastenkürzel"] + [ln for x in binds for ln in bind_conf(x)]
    return "\n".join(out) + "\n"


def detect_format(hdir: Path) -> str | None:
    """Lua hat ab 0.55 Vorrang (hyprland.lua wird statt hyprland.conf geladen); ab 0.57 gibt es nur noch Lua."""
    if (hdir / "hyprland.lua").is_file():
        return "lua"
    if (hdir / "hyprland.conf").is_file():
        return "conf"
    return None


def hypr_format(hdir: Path) -> str | None:
    """Gewähltes Format (Erweiterte Einstellungen) oder erkanntes."""
    return FORMAT if FORMAT in ("lua", "conf") else detect_format(hdir)


INCLUDE = {
    "lua": ('require("hyprstudio")', "--"),
    "conf": ("source = ~/.config/hypr/hyprstudio.conf", "#"),
}


def _has_include(text: str, fmt: str) -> bool:
    """Genau die Einbindung von hyprstudio.* (die der Laptop-Tasten zählt nicht)."""
    return INCLUDE[fmt][0] in (ln.strip() for ln in text.splitlines())


def stamp(f: Path) -> Path:
    """Freier Sicherungsname mit Zeitstempel (nie eine vorhandene Sicherung überschreiben)."""
    base = f"{f.name}.hyprstudio-{time.strftime('%Y%m%d-%H%M%S')}"
    b, i = f.with_name(base), 1
    while b.exists():
        b, i = f.with_name(f"{base}-{i}"), i + 1
    return b


def _keep_previous(target: Path) -> None:
    """Vorherige eigene Version als .bak behalten (für Rückgängig)."""
    if target.is_file():
        shutil.copy2(target, target.with_name(target.name + ".bak"))


def _backup_foreign(target: Path) -> Path | None:
    """Fremde Datei vor dem ersten Überschreiben sichern; nie etwas löschen."""
    if not target.is_file():
        return None
    if MARK in target.read_text(encoding="utf-8", errors="replace")[:300]:
        _keep_previous(target)
        return None
    dst = target.with_name(target.name + ".hyprstudio-original")
    if dst.exists():
        dst = stamp(target)
    shutil.copy2(target, dst)
    return dst


def apply_hypr(s: dict, hdir: Path, log) -> tuple[bool, str]:
    fmt = hypr_format(hdir)
    if DRY and not (hdir / f"hyprland.{fmt or 'lua'}").is_file():
        fmt = fmt or "lua"
        hdir.mkdir(parents=True, exist_ok=True)
        (hdir / f"hyprland.{fmt}").write_text(f"{INCLUDE[fmt][1]} Testmodus: Platzhalter für die echte "
                                              f"hyprland.{fmt}\n", encoding="utf-8")
    if fmt is None:
        return False, "nohypr"
    if not (hdir / f"hyprland.{fmt}").is_file():
        return False, f"nomain.{fmt}"
    target = hdir / f"hyprstudio.{fmt}"
    _keep_previous(target)
    target.write_text(hypr_lua(s) if fmt == "lua" else hypr_conf(s), encoding="utf-8")
    log(f"→ {target}")
    main = hdir / f"hyprland.{fmt}"
    text = main.read_text(encoding="utf-8", errors="replace")
    if not _has_include(text, fmt):
        bak = main.with_name(main.name + ".hyprstudio-original")
        if not bak.exists():
            shutil.copy2(main, bak)
            log(f"Sicherung: {bak}")
        line, c = INCLUDE[fmt]
        with open(main, "a", encoding="utf-8") as fh:
            fh.write(("" if text.endswith("\n") or not text else "\n") + f"\n{c} {MARK}\n{line}\n")
        log(f"+ {main}: {line}")
    manage_waybar_autostart(hdir, s, log)
    manage_polkit_autostart(hdir, s, log)
    manage_laptop_keys(hdir, s, log)
    run(["hyprctl", "reload"], log)
    return True, fmt


def undo_file(target: Path) -> bool:
    bak = target.with_name(target.name + ".bak")
    if not (bak.is_file() and target.is_file()):
        return False
    cur, old = target.read_bytes(), bak.read_bytes()
    target.write_bytes(old)
    bak.write_bytes(cur)
    return True


def undo_hypr(hdir: Path, log) -> bool:
    fmt = hypr_format(hdir)
    if not fmt or not undo_file(hdir / f"hyprstudio.{fmt}"):
        return False
    run(["hyprctl", "reload"], log)
    return True


# ---------- Waybar ----------

WAYBAR_DEFAULT = {
    # Aussehen (Presets)
    "position": "top", "style": "float", "height": 34, "margin": 8, "radius": 12, "spacing": 4,
    "bg": "#0e131add", "fg": "#e6edf5ff", "accent": "#3d8bfdff", "border_width": 0, "border": "#253142ff",
    "pad": 10, "ws_look": "pill", "colorful": False,
    "left": ["custom/launcher", "hyprland/workspaces"], "center": ["clock"],
    "right": ["pulseaudio", "network", "battery", "tray"],
    # Verhalten und Module (bleiben beim Preset-Wechsel)
    "enabled": True, "layer": "top", "exclusive": True, "tooltips": True,
    "font": "JetBrainsMono Nerd Font", "font_size": 13, "font_weight": "bold", "icons": True,
    "clock24": True, "clock_fmt": "hm", "ws_format": "number", "ws_persist": 0, "win_len": 48,
    "bat_warn": 30, "bat_crit": 15, "net_fmt": "essid", "vol_step": 5, "vol_click": "pavucontrol",
    "interval": 5, "tray_size": 16, "mpris_len": 32,
    "launcher": "rofi -show drun", "power_cmd": "wlogout",
}
WB_LOOK = ("position", "style", "height", "margin", "radius", "spacing", "bg", "fg", "accent", "border_width",
           "border", "pad", "ws_look", "colorful", "left", "center", "right")

MODULES = ("custom/launcher", "hyprland/workspaces", "hyprland/window", "clock", "pulseaudio", "network",
           "battery", "backlight", "bluetooth", "cpu", "memory", "temperature", "disk", "mpris",
           "hyprland/language", "idle_inhibitor", "power-profiles-daemon", "tray", "custom/power")

_WAYBAR_PRESETS_OWN = {
    "float": {},
    "islands": {"style": "islands", "bg": "#151c26ee", "radius": 14, "height": 32},
    "classic": {"style": "bar", "margin": 0, "radius": 0, "height": 30, "bg": "#0e131aff", "ws_look": "underline",
                "left": ["hyprland/workspaces", "hyprland/window"], "center": ["clock"],
                "right": ["cpu", "memory", "pulseaudio", "network", "battery", "tray"]},
    "minimal": {"style": "float", "height": 28, "margin": 6, "radius": 10, "bg": "#0e131aaa", "ws_look": "text",
                "left": ["hyprland/workspaces"], "center": [], "right": ["clock"]},
    "bottom": {"position": "bottom", "style": "islands", "bg": "#151c26ee",
               "left": ["custom/launcher", "hyprland/workspaces"], "center": ["hyprland/window"],
               "right": ["pulseaudio", "network", "battery", "clock", "tray"]},
    "glass": {"style": "float", "bg": "#ffffff1f", "fg": "#ffffffff", "border_width": 1, "border": "#ffffff33",
              "radius": 16, "height": 36, "margin": 10},
    "rainbow": {"style": "islands", "bg": "#1a1b26ee", "colorful": True, "ws_look": "box", "radius": 10,
                "left": ["custom/launcher", "hyprland/workspaces", "mpris"], "center": ["clock"],
                "right": ["cpu", "memory", "pulseaudio", "network", "battery", "tray", "custom/power"]},
    "dock": {"position": "bottom", "style": "islands", "height": 40, "radius": 20, "margin": 10, "bg": "#151c26f0",
             "ws_look": "box", "left": [], "center": ["custom/launcher", "hyprland/workspaces"],
             "right": ["clock", "tray"]},
    "neon": {"style": "float", "bg": "#0b0b14f0", "fg": "#f8f8ffff", "accent": "#ff2bd6ff", "border_width": 2,
             "border": "#ff2bd6ff", "radius": 12, "colorful": True, "ws_look": "underline"},
    "pills": {"style": "islands", "bg": "#1b2430ff", "radius": 18, "height": 34, "ws_look": "box", "pad": 14},
    "underline": {"style": "bar", "margin": 0, "radius": 0, "height": 30, "bg": "#11111bff", "ws_look": "underline",
                  "left": ["hyprland/workspaces", "hyprland/window"], "center": [],
                  "right": ["pulseaudio", "network", "battery", "clock", "tray"]},
    "clear": {"style": "float", "bg": "#00000000", "fg": "#ffffffff", "ws_look": "text", "border_width": 0},
    "catppuccin": {"style": "islands", "bg": "#1e1e2ef0", "fg": "#cdd6f4ff", "accent": "#cba6f7ff", "radius": 14,
                   "border_width": 1, "border": "#313244ff", "colorful": True},
    "nord": {"style": "float", "bg": "#2e3440ee", "fg": "#eceff4ff", "accent": "#88c0d0ff", "radius": 8,
             "border_width": 1, "border": "#4c566aff"},
    "gruvbox": {"style": "bar", "margin": 0, "radius": 0, "bg": "#282828ff", "fg": "#ebdbb2ff", "accent": "#fabd2fff",
                "ws_look": "box", "height": 30},
    "full": {"style": "bar", "margin": 0, "radius": 0, "height": 32, "bg": "#0e131af0", "colorful": True,
             "left": ["custom/launcher", "hyprland/workspaces", "hyprland/window"], "center": ["clock", "mpris"],
             "right": ["idle_inhibitor", "hyprland/language", "cpu", "memory", "disk", "temperature", "pulseaudio",
                       "backlight", "network", "bluetooth", "battery", "tray", "custom/power"]},
}
WAYBAR_PRESETS = palettes.unify(_WAYBAR_PRESETS_OWN, lambda p: {"bg": p["bg"] + "ee", "fg": p["fg"] + "ff", "accent": p["accent"] + "ff"})  # gemeinsame Namen/Farben

PALETTE = ("#7aa2f7ff", "#9ece6aff", "#e0af68ff", "#bb9af7ff", "#7dcfffff", "#f7768eff", "#ff9e64ff", "#73dacaff")

G = {  # Nerd-Font-Symbole
    "clock": "", "vol": ["", "", ""], "mute": "\U000f075f", "wifi": "",
    "eth": "\U000f0200", "nonet": "\U000f05aa", "bat": ["", "", "", "", ""],
    "charge": "", "cpu": "", "mem": "\U000f035b", "temp": "", "light": [""],
    "bt": "", "arch": "", "disk": "", "play": "", "pause": "", "kb": "",
    "eye": "", "eyeoff": "", "power": "", "dot": "", "ring": "",
    "perf": "", "bal": "", "save": "",
}
CLOCK_FMT = {"hm": ("%H:%M", "%I:%M %p"), "hms": ("%H:%M:%S", "%I:%M:%S %p"), "dhm": ("%a %H:%M", "%a %I:%M %p"),
             "full": ("%d.%m.%Y  %H:%M", "%m/%d/%Y  %I:%M %p")}


def _copy(d):
    return json.loads(json.dumps(d))


def waybar_state(saved: dict | None) -> dict:
    s = _copy(WAYBAR_DEFAULT)
    for k, v in (saved or {}).items():
        if k in s and type(v) is type(s[k]):
            s[k] = v
    for side in ("left", "center", "right"):
        s[side] = [m for m in s[side] if m in MODULES]
    return s


def waybar_preset(name: str, base: dict | None = None) -> dict:
    """Preset-Look; Schrift, Befehle und Modul-Optionen kommen aus base."""
    s = waybar_state(base)
    for k in WB_LOOK:
        s[k] = _copy(WAYBAR_DEFAULT[k])
    s.update(_copy(WAYBAR_PRESETS.get(name, {})))
    return s


def module_color(name: str, s: dict) -> str:
    return PALETTE[MODULES.index(name) % len(PALETTE)] if s["colorful"] else s["fg"]


def module_cfg(name: str, s: dict) -> dict:
    ic = s["icons"]

    def g(k, sep="  "):
        return (G[k] + sep) if ic else ""
    clock = CLOCK_FMT[s["clock_fmt"]][0 if s["clock24"] else 1]
    ws = {"format": "{name}", "on-click": "activate", "sort-by-number": True}
    if s["ws_format"] == "dot" and ic:
        ws.update({"format": "{icon}", "format-icons": {"active": G["dot"], "default": G["ring"]}})
    if s["ws_persist"]:
        ws["persistent-workspaces"] = {"*": s["ws_persist"]}
    net = {"essid": "{essid}", "ip": "{ipaddr}", "signal": "{signalStrength}%"}[s["net_fmt"]]
    cfg = {
        "custom/launcher": {"format": G["arch"] if ic else "Apps", "on-click": s["launcher"], "tooltip": False,
                            "justify": "center"},
        "hyprland/workspaces": ws,
        "hyprland/window": {"max-length": s["win_len"], "separate-outputs": True},
        "clock": {"format": g("clock") + "{:" + clock + "}", "format-alt": "{:%A, %d.%m.%Y}",
                  "tooltip-format": "<tt>{calendar}</tt>", **({"interval": 1} if s["clock_fmt"] == "hms" else {})},
        "pulseaudio": {"format": "{icon}  {volume}%" if ic else "Vol {volume}%",
                       "format-muted": G["mute"] if ic else "Stumm", "format-icons": {"default": G["vol"]},
                       "scroll-step": s["vol_step"], "on-click": s["vol_click"]},
        "network": {"format-wifi": g("wifi") + net, "format-ethernet": g("eth") + ("LAN" if s["net_fmt"] != "ip" else "{ipaddr}"),
                    "format-disconnected": G["nonet"] if ic else "Offline", "tooltip-format": "{ifname} · {ipaddr}"},
        "battery": {"format": "{icon}  {capacity}%" if ic else "Bat {capacity}%", "format-icons": G["bat"],
                    "format-charging": g("charge") + "{capacity}%",
                    "states": {"warning": s["bat_warn"], "critical": s["bat_crit"]}},
        "backlight": {"format": "{icon}  {percent}%" if ic else "Hell {percent}%", "format-icons": G["light"]},
        "bluetooth": {"format": G["bt"] if ic else "BT", "format-connected": (G["bt"] if ic else "BT") + " {num_connections}"},
        "cpu": {"format": (g("cpu") or "CPU ") + "{usage}%", "interval": s["interval"]},
        "memory": {"format": (g("mem") or "RAM ") + "{percentage}%", "interval": s["interval"]},
        "temperature": {"format": g("temp") + "{temperatureC}°C", "interval": s["interval"]},
        "disk": {"format": (g("disk") or "Disk ") + "{percentage_used}%", "path": "/", "interval": s["interval"] * 6},
        "mpris": {"format": g("play") + "{artist} – {title}", "format-paused": g("pause") + "{artist} – {title}",
                  "max-length": s["mpris_len"]},
        "hyprland/language": {"format": g("kb") + "{short}"},
        "idle_inhibitor": {"format": "{icon}", "format-icons": {"activated": G["eye"] if ic else "Wach",
                                                                 "deactivated": G["eyeoff"] if ic else "Auto"}},
        "power-profiles-daemon": {"format": "{icon}" if ic else "{profile}",
                                  "format-icons": {"default": G["perf"], "performance": G["perf"],
                                                   "balanced": G["bal"], "power-saver": G["save"]}},
        "tray": {"spacing": 8, "icon-size": s["tray_size"]},
        "custom/power": {"format": G["power"] if ic else "Power", "on-click": s["power_cmd"], "tooltip": False,
                         "justify": "center"},
    }[name]
    if not s["tooltips"]:
        cfg["tooltip"] = False
    return cfg


def waybar_json(s: dict) -> str:
    cfg = {"layer": s["layer"], "position": s["position"], "height": s["height"], "spacing": s["spacing"],
           "exclusive": s["exclusive"]}
    if s["style"] != "bar" and s["margin"]:
        m = s["margin"]
        cfg.update({"margin-top": m if s["position"] == "top" else 0,
                    "margin-bottom": m if s["position"] == "bottom" else 0,
                    "margin-left": m * 2, "margin-right": m * 2})
    cfg["modules-left"] = s["left"]
    cfg["modules-center"] = s["center"]
    cfg["modules-right"] = s["right"]
    for m in s["left"] + s["center"] + s["right"]:
        cfg[m] = module_cfg(m, s)
    return f"// {MARK} – automatisch erzeugt, wird beim Anwenden überschrieben\n" + \
        json.dumps(cfg, indent=4, ensure_ascii=False) + "\n"


CSS_IDS = {"custom/launcher": "#custom-launcher", "hyprland/workspaces": "#workspaces",
           "hyprland/window": "#window", "clock": "#clock", "pulseaudio": "#pulseaudio", "network": "#network",
           "battery": "#battery", "backlight": "#backlight", "bluetooth": "#bluetooth", "cpu": "#cpu",
           "memory": "#memory", "temperature": "#temperature", "disk": "#disk", "mpris": "#mpris",
           "hyprland/language": "#language", "idle_inhibitor": "#idle_inhibitor",
           "power-profiles-daemon": "#power-profiles-daemon", "tray": "#tray", "custom/power": "#custom-power"}

WS_CSS = {
    "pill": lambda fg, acc, r: f"color: {rgba_css(acc)};\n    background: {rgba_css(acc, 0.18)};",
    "box": lambda fg, acc, r: f"color: #0e131a;\n    background: {rgba_css(acc)};",
    "underline": lambda fg, acc, r: f"color: {rgba_css(fg)};\n    box-shadow: inset 0 -3px {rgba_css(acc)};\n"
                                    f"    border-radius: 0;",
    "text": lambda fg, acc, r: f"color: {rgba_css(acc)};\n    background: transparent;",
}


def waybar_css(s: dict) -> str:
    r = s["radius"]
    fg, acc, bg = s["fg"], s["accent"], s["bg"]
    border = f"{s['border_width']}px solid {rgba_css(s['border'])}" if s["border_width"] else "none"
    ids = ", ".join(CSS_IDS[m] for m in MODULES)
    pad = s["pad"]
    if s["style"] == "islands":
        win = "background: transparent;"
        mod = (f"background: {rgba_css(bg)}; border-radius: {r}px; border: {border}; padding: 0 {pad + 2}px;"
               " margin: 0 3px;")
    else:
        win = f"background: {rgba_css(bg)}; border-radius: {r if s['style'] == 'float' else 0}px; border: {border};"
        mod = f"padding: 0 {pad}px;"
    colors = ""
    if s["colorful"]:
        colors = "\n".join(f"{CSS_IDS[m]} {{ color: {rgba_css(module_color(m, s))}; }}"
                           for m in MODULES if m not in ("hyprland/workspaces", "tray", "custom/launcher")) + "\n"
    return f"""/* {MARK} – automatisch erzeugt, wird beim Anwenden überschrieben */
* {{
    font-family: "{s['font']}", "Symbols Nerd Font", sans-serif;
    font-size: {s['font_size']}px;
    font-weight: {s['font_weight']};
    min-height: 0;
}}

window#waybar {{
    {win}
    color: {rgba_css(fg)};
}}

{ids} {{
    {mod}
    color: {rgba_css(fg)};
}}

{colors}/* Symbole aus einer Symbol-Schrift: die normale Nerd Font zeichnet sie breiter als ihr Platz → wirken verschoben */
#custom-launcher, #custom-power {{
    font-family: "Symbols Nerd Font Mono", "JetBrainsMono Nerd Font Propo", "{s['font']}", sans-serif;
    font-size: {s['font_size'] + 3}px;
    padding: 0 {pad + (4 if s['style'] == 'islands' else 2)}px;
    min-width: {s['font_size'] + 3}px;
}}

#custom-launcher {{
    color: {rgba_css(acc)};
}}

#custom-power {{
    color: #e5534b;
}}

#workspaces button {{
    padding: 0 7px;
    color: {rgba_css(fg, 0.55)};
    background: transparent;
    border: none;
    border-radius: {max(4, r // 2)}px;
    box-shadow: none;
}}

#workspaces button:hover {{
    background: {rgba_css(fg, 0.08)};
}}

#workspaces button.active {{
    {WS_CSS[s['ws_look']](fg, acc, r)}
}}

#workspaces button.urgent {{
    color: #e5534b;
}}

#idle_inhibitor.activated {{
    color: {rgba_css(acc)};
}}

#battery.warning {{ color: #d29922; }}
#battery.critical {{ color: #e5534b; }}

tooltip {{
    background: {rgba_css(bg, 0.95)};
    border: 1px solid {rgba_css(fg, 0.15)};
    border-radius: {max(6, r // 2)}px;
}}
"""


def waybar_files(wdir: Path) -> tuple[Path, Path]:
    for n in ("config.jsonc", "config.json", "config"):
        if (wdir / n).is_file():
            return wdir / n, wdir / "style.css"
    return wdir / "config.jsonc", wdir / "style.css"


def apply_waybar(s: dict, wdir: Path, log, restart: bool = True) -> tuple[bool, str]:
    wdir.mkdir(parents=True, exist_ok=True)
    cfg, css = waybar_files(wdir)
    for target, data in ((cfg, waybar_json(s)), (css, waybar_css(s))):
        bak = _backup_foreign(target)
        if bak:
            log(f"Sicherung: {bak}")
        target.write_text(data, encoding="utf-8")
        log(f"→ {target}")
    if restart:
        restart_waybar(log)
    return True, ""


def undo_waybar(wdir: Path, log) -> bool:
    cfg, css = waybar_files(wdir)
    done = [undo_file(f) for f in (cfg, css)]
    if any(done):
        restart_waybar(log)
    return any(done)


def restart_waybar(log) -> None:
    # SIGUSR2 lädt neu, stürzt aber bei manchen Versionen ab – Neustart ist zuverlässiger
    run(["pkill", "-x", "waybar"], log, quiet=True)
    if DRY:
        log("$ waybar &  (Testmodus)")
        return
    time.sleep(0.3)
    try:
        subprocess.Popen(["waybar"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True)
        log("$ waybar &")
    except OSError as e:
        log(f"waybar: {e}")


# ---------- Prozesse ----------

def run(cmd: list, log, quiet: bool = False) -> bool:
    if DRY:
        log("$ " + " ".join(cmd) + "  (Testmodus)")
        return True
    try:
        p = subprocess.run(cmd, capture_output=True, text=True, timeout=10)
    except (OSError, subprocess.TimeoutExpired) as e:
        log(f"$ {' '.join(cmd)}: {e}")
        return False
    if not quiet or p.returncode == 0:
        log("$ " + " ".join(cmd))
        for ln in (p.stdout + p.stderr).strip().splitlines():
            log("  " + ln)
    return p.returncode == 0


def which(name: str) -> bool:
    return DRY or shutil.which(name) is not None
