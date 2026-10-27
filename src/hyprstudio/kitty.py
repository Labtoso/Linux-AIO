"""Kitty-Terminal: Zustand, Presets und ~/.config/kitty/hyprstudio.conf (per include in kitty.conf). Ohne Qt."""

import json
import shutil
from pathlib import Path

from . import model, palettes
from .model import MARK, norm

INCLUDE = "include hyprstudio.conf"

PALETTES = {k: p["ansi"] for k, p in palettes.PALETTES.items()}  # 16 Farben: color0 … color15

KITTY_DEFAULT = {
    # Farben
    "palette": PALETTES["tokyo"], "fg": "#c0caf5", "bg": "#1a1b26", "cursor": "#c0caf5", "sel_fg": "#1a1b26",
    "sel_bg": "#7aa2f7", "url": "#7dcfff", "tab_active_fg": "#1a1b26", "tab_active_bg": "#7aa2f7",
    "tab_inactive_fg": "#787c99", "tab_inactive_bg": "#16161e",
    # Aussehen
    "opacity": 0.85, "dynamic_opacity": True, "blur": 32, "padding": 14, "decorations": "no",
    "display_server": "auto",
    # Schrift
    "font": "JetBrainsMono Nerd Font", "font_size": 11.5, "ligatures": "never",
    # Cursor & Animation
    "cursor_shape": "beam", "blink": 0.0, "blink_curve": "ease-in-out", "trail": 3, "trail_decay_fast": 0.1,
    "trail_decay_slow": 0.4, "bell_flash": False,
    # Tabs
    "tab_edge": "bottom", "tab_style": "powerline", "tab_powerline": "slanted", "tab_title": "{index}: {title}",
    # Verhalten
    "confirm_close": 0, "audio_bell": False, "scrollback": 10000, "url_style": "curly", "copy_on_select": False,
    # Beim Öffnen
    "fastfetch": False, "start_cmds": [],
}
START = "hyprstudio-start.sh"
LOOK = ("palette", "fg", "bg", "cursor", "sel_fg", "sel_bg", "url", "tab_active_fg", "tab_active_bg",
        "tab_inactive_fg", "tab_inactive_bg", "opacity", "blur", "padding", "cursor_shape", "trail", "tab_style",
        "tab_powerline")


def _theme(p: dict) -> dict:
    return {"palette": p["ansi"], "fg": p["fg"], "bg": p["bg"], "cursor": p["fg"], "sel_fg": p["bg"],
            "sel_bg": p["accent"], "url": p["ansi"][6], "tab_active_fg": p["bg"], "tab_active_bg": p["accent"],
            "tab_inactive_fg": p["muted"], "tab_inactive_bg": p["bg2"], "font": palettes.FONT,
            "font_size": palettes.FONT_SIZE}


_TOKYO_TABS = {"tab_inactive_fg": "#787c99", "tab_inactive_bg": "#16161e"}   # Kittys Tokyo-Night-Tabs wie bisher
PRESETS = palettes.themed(_theme, extras={
    # „01“ = Sebastians Kitty-Config (Vault: Skripte/Kitty Config), 2026-10-20: Deckkraft 0.7, ohne Blur
    "01": _TOKYO_TABS | {"display_server": "x11", "opacity": 0.7, "blur": 0},
    "tokyo": _TOKYO_TABS,
    "nord": {"cursor_shape": "block"},
    "gruvbox": {"cursor_shape": "block", "tab_style": "slant"},
    "rosepine": {"tab_powerline": "round"},
}, rest={
    "glass": {"opacity": 0.6, "blur": 48, "padding": 18, "trail": 3},
    "solid": {"opacity": 1.0, "blur": 0, "trail": 0, "tab_style": "fade"},
    "retro": {"palette": ["#000000", "#ff5555", "#33ff66", "#ffcc00", "#3399ff", "#cc66ff", "#33cccc", "#b0ffb0",
                          "#335533", "#ff7777", "#66ff99", "#ffe066", "#66b3ff", "#dd99ff", "#66e0e0", "#e0ffe0"],
              "fg": "#33ff66", "bg": "#000000", "cursor": "#33ff66", "sel_fg": "#000000", "sel_bg": "#33ff66",
              "url": "#33cccc", "tab_active_fg": "#000000", "tab_active_bg": "#33ff66", "tab_inactive_fg": "#227733",
              "tab_inactive_bg": "#001100", "opacity": 0.95, "blur": 0, "cursor_shape": "block", "trail": 0,
              "tab_style": "separator", "padding": 8},
})
PRESETS["mine"] = PRESETS["01"]                  # alter Name aus früheren Versionen


def _copy(d):
    return json.loads(json.dumps(d))


def state(saved: dict | None) -> dict:
    s = _copy(KITTY_DEFAULT)
    for k, v in (saved or {}).items():
        if k not in s:
            continue
        if k == "palette":
            if isinstance(v, list) and len(v) == 16 and all(isinstance(x, str) for x in v):
                s[k] = v
        elif k == "start_cmds":
            if isinstance(v, list):
                s[k] = [x.strip() for x in v if isinstance(x, str) and x.strip()]
        elif type(v) is type(s[k]):
            s[k] = v
        elif isinstance(s[k], float) and type(v) is int:
            s[k] = float(v)
    return s


def preset(name: str, base: dict | None = None) -> dict:
    s = state(base)
    for k in LOOK + ("display_server",):
        s[k] = _copy(KITTY_DEFAULT[k])
    s.update(_copy(PRESETS.get(name, {})))
    return s


def _c(x: str) -> str:
    return norm(x)[:7]


def _num(v) -> str:
    return model._num(v) if not isinstance(v, bool) else ("yes" if v else "no")


def has_start(s: dict) -> bool:
    return bool(s["fastfetch"] or s["start_cmds"])


def start_script(s: dict) -> str:
    """Läuft bei jedem neuen Kitty-Fenster und startet danach die normale Shell."""
    out = ["#!/bin/sh", f"# {MARK} – automatisch erzeugt, wird beim Anwenden überschrieben"]
    if s["fastfetch"]:
        out.append("command -v fastfetch >/dev/null 2>&1 && fastfetch")
    out += s["start_cmds"]
    out += ['sh_="${SHELL:-}"',
            f'case "$sh_" in ""|*{START}) sh_=$(getent passwd "$(id -un)" | cut -d: -f7) ;; esac',
            'exec "${sh_:-/bin/sh}"']
    return "\n".join(out) + "\n"


def conf(s: dict, start: Path | None = None) -> str:
    yn = lambda b: "yes" if b else "no"  # noqa: E731
    blink = "0" if not s["blink"] else f"{_num(float(s['blink']))} {s['blink_curve']}"
    lines = [
        f"# {MARK} – automatisch erzeugt, wird beim Anwenden überschrieben",
        "",
        "# Schrift",
        f"font_family        {s['font']}",
        f"font_size          {_num(float(s['font_size']))}",
        f"disable_ligatures  {s['ligatures']}",
        "",
        "# Transparenz",
        f"background_opacity          {_num(float(s['opacity']))}",
        f"dynamic_background_opacity  {yn(s['dynamic_opacity'])}",
        f"background_blur             {s['blur']}",
    ]
    if s["display_server"] != "auto":
        lines.append(f"linux_display_server        {s['display_server']}")
    lines += [
        "",
        "# Fenster",
        f"hide_window_decorations  {s['decorations']}",
        f"window_padding_width     {s['padding']}",
        f"confirm_os_window_close  {s['confirm_close']}",
        f"enable_audio_bell        {yn(s['audio_bell'])}",
        f"visual_bell_duration     {'0.2 ease-out' if s['bell_flash'] else '0'}",
        f"scrollback_lines         {s['scrollback']}",
        f"url_style                {s['url_style']}",
        f"copy_on_select           {'clipboard' if s['copy_on_select'] else 'no'}",
        "",
        "# Cursor & Animation",
        f"cursor_shape             {s['cursor_shape']}",
        f"cursor_blink_interval    {blink}",
        f"cursor_trail             {s['trail']}",
        f"cursor_trail_decay       {_num(float(s['trail_decay_fast']))} {_num(float(s['trail_decay_slow']))}",
        "",
        "# Tabs",
        f"tab_bar_edge          {s['tab_edge']}",
        f"tab_bar_style         {s['tab_style']}",
        f"tab_powerline_style   {s['tab_powerline']}",
        f'tab_title_template    "{s["tab_title"]}"',
        "",
        "# Farben",
        f"foreground            {_c(s['fg'])}",
        f"background            {_c(s['bg'])}",
        f"selection_foreground  {_c(s['sel_fg'])}",
        f"selection_background  {_c(s['sel_bg'])}",
        f"cursor                {_c(s['cursor'])}",
        f"url_color             {_c(s['url'])}",
        f"active_tab_foreground   {_c(s['tab_active_fg'])}",
        f"active_tab_background   {_c(s['tab_active_bg'])}",
        f"inactive_tab_foreground {_c(s['tab_inactive_fg'])}",
        f"inactive_tab_background {_c(s['tab_inactive_bg'])}",
        "",
    ]
    lines += [f"color{i:<2} {_c(c)}" for i, c in enumerate(s["palette"])]
    if start:
        lines += ["", "# Beim Öffnen (Fastfetch / eigene Befehle)", f"shell {start}"]
    return "\n".join(lines) + "\n"


def kdir(cfg_home: Path) -> Path:
    return cfg_home / "kitty"


def apply(s: dict, cfg_home: Path, log) -> tuple[bool, str]:
    d = kdir(cfg_home)
    d.mkdir(parents=True, exist_ok=True)
    own = d / "hyprstudio.conf"
    model._keep_previous(own)
    start = None
    if has_start(s):
        start = d / START
        with open(start, "w", encoding="utf-8", newline="\n") as fh:
            fh.write(start_script(s))
        start.chmod(0o755)
        log(f"→ {start}")
    own.write_text(conf(s, start), encoding="utf-8")
    log(f"→ {own}")
    main = d / "kitty.conf"
    text = main.read_text(encoding="utf-8", errors="replace") if main.is_file() else ""
    if not any(ln.strip() == INCLUDE for ln in text.splitlines()):
        if main.is_file():
            bak = main.with_name("kitty.conf.hyprstudio-original")
            if not bak.exists():
                shutil.copy2(main, bak)
                log(f"Sicherung: {bak}")
        with open(main, "a", encoding="utf-8") as fh:
            fh.write(("\n" if text and not text.endswith("\n") else "") + f"\n# {MARK}\n{INCLUDE}\n")
        log(f"+ {main}: {INCLUDE}")
    # laufende Kitty-Fenster laden die Config neu (SIGUSR1)
    model.run(["pkill", "-USR1", "-x", "kitty"], log, quiet=True)
    return True, ""


def undo(cfg_home: Path, log) -> bool:
    ok = model.undo_file(kdir(cfg_home) / "hyprstudio.conf")
    if ok:
        model.run(["pkill", "-USR1", "-x", "kitty"], log, quiet=True)
    return ok


def remove_include(cfg_home: Path, log) -> bool:
    main = kdir(cfg_home) / "kitty.conf"
    if not main.is_file():
        return False
    lines = main.read_text(encoding="utf-8", errors="replace").splitlines(keepends=True)
    keep = [ln for ln in lines if ln.strip() not in (INCLUDE, f"# {MARK}")]
    if len(keep) == len(lines):
        return False
    shutil.copy2(main, model.stamp(main))
    main.write_text("".join(keep), encoding="utf-8")
    log(f"Einbindung entfernt: {main}")
    return True
