"""Alacritty configurator: presets, font, padding, opacity, cursor → ~/.config/alacritty/alacritty.toml.
A foreign file is backed up once as alacritty.toml.linuxaio-original. No Qt."""

import json
import shutil
from pathlib import Path

from . import usercmd

MARK = "# Linux AIO – generated, overwritten when applied"



def _from_kitty(name: str, title: str) -> dict:
    """A Hypr Studio Kitty preset 1:1 as Alacritty preset: colours plus font, padding, opacity, cursor."""
    from hyprstudio import kitty
    k = kitty.preset(name)
    pal = [kitty._c(c) for c in k["palette"]]
    return {"title": title, "bg": kitty._c(k["bg"]), "fg": kitty._c(k["fg"]), "cursor": kitty._c(k["cursor"]),
            "sel": kitty._c(k["sel_bg"]), "sel_fg": kitty._c(k["sel_fg"]), "normal": pal[:8], "bright": pal[8:16],
            "look": {"font": k["font"], "size": float(k["font_size"]), "padding": int(k["padding"]),
                     "opacity": float(k["opacity"]),
                     "cursor": {"block": "Block", "underline": "Underline"}.get(k["cursor_shape"], "Beam"),
                     "blink": float(k["blink"]) > 0}}


PRESETS = {
    "01": _from_kitty("mine", "01"),          # Sebastians Kitty-Preset „01“
    "tokyo": {"title": "Tokyo Night", "bg": "#1a1b26", "fg": "#c0caf5", "cursor": "#c0caf5", "sel": "#33467c",
              "normal": ["#15161e", "#f7768e", "#9ece6a", "#e0af68", "#7aa2f7", "#bb9af7", "#7dcfff", "#a9b1d6"],
              "bright": ["#414868", "#f7768e", "#9ece6a", "#e0af68", "#7aa2f7", "#bb9af7", "#7dcfff", "#c0caf5"]},
    "catppuccin": {"title": "Catppuccin Mocha", "bg": "#1e1e2e", "fg": "#cdd6f4", "cursor": "#f5e0dc", "sel": "#585b70",
                   "normal": ["#45475a", "#f38ba8", "#a6e3a1", "#f9e2af", "#89b4fa", "#f5c2e7", "#94e2d5", "#bac2de"],
                   "bright": ["#585b70", "#f38ba8", "#a6e3a1", "#f9e2af", "#89b4fa", "#f5c2e7", "#94e2d5", "#a6adc8"]},
    "nord": {"title": "Nord", "bg": "#2e3440", "fg": "#d8dee9", "cursor": "#d8dee9", "sel": "#4c566a",
             "normal": ["#3b4252", "#bf616a", "#a3be8c", "#ebcb8b", "#81a1c1", "#b48ead", "#88c0d0", "#e5e9f0"],
             "bright": ["#4c566a", "#bf616a", "#a3be8c", "#ebcb8b", "#81a1c1", "#b48ead", "#8fbcbb", "#eceff4"]},
    "dracula": {"title": "Dracula", "bg": "#282a36", "fg": "#f8f8f2", "cursor": "#f8f8f2", "sel": "#44475a",
                "normal": ["#21222c", "#ff5555", "#50fa7b", "#f1fa8c", "#bd93f9", "#ff79c6", "#8be9fd", "#f8f8f2"],
                "bright": ["#6272a4", "#ff6e6e", "#69ff94", "#ffffa5", "#d6acff", "#ff92df", "#a4ffff", "#ffffff"]},
    "gruvbox": {"title": "Gruvbox", "bg": "#282828", "fg": "#ebdbb2", "cursor": "#ebdbb2", "sel": "#504945",
                "normal": ["#282828", "#cc241d", "#98971a", "#d79921", "#458588", "#b16286", "#689d6a", "#a89984"],
                "bright": ["#928374", "#fb4934", "#b8bb26", "#fabd2f", "#83a598", "#d3869b", "#8ec07c", "#ebdbb2"]},
    "onedark": {"title": "One Dark", "bg": "#282c34", "fg": "#abb2bf", "cursor": "#528bff", "sel": "#3e4451",
                "normal": ["#282c34", "#e06c75", "#98c379", "#e5c07b", "#61afef", "#c678dd", "#56b6c2", "#abb2bf"],
                "bright": ["#5c6370", "#e06c75", "#98c379", "#e5c07b", "#61afef", "#c678dd", "#56b6c2", "#ffffff"]},
}
DEFAULT = {"preset": "01", **PRESETS["01"]["look"]}
NAMES = ("black", "red", "green", "yellow", "blue", "magenta", "cyan", "white")


def state(saved: dict | None) -> dict:
    s = dict(DEFAULT)
    s.update({k: v for k, v in (saved or {}).items() if k in DEFAULT})
    if s["preset"] not in PRESETS:
        s["preset"] = DEFAULT["preset"]
    return s


def toml(s: dict) -> str:
    c = PRESETS[s["preset"]]
    q = json.dumps                                   # TOML strings like JSON strings
    lines = [MARK, "", "[window]", f"padding = {{ x = {int(s['padding'])}, y = {int(s['padding'])} }}",
             f"opacity = {float(s['opacity']):.2f}", 'decorations = "Full"', "",
             "[font]", f"size = {float(s['size']):.1f}", f"normal = {{ family = {q(s['font'])} }}", "",
             "[cursor]", f"style = {{ shape = {q(s['cursor'])}, blinking = {q('On' if s['blink'] else 'Off')} }}", "",
             "[colors.primary]", f"background = {q(c['bg'])}", f"foreground = {q(c['fg'])}", "",
             "[colors.cursor]", f"cursor = {q(c['cursor'])}", f"text = {q(c['bg'])}", "",
             "[colors.selection]", f"background = {q(c['sel'])}", f"text = {q(c.get('sel_fg', c['fg']))}", "",
             "[colors.normal]"] + [f"{n} = {q(v)}" for n, v in zip(NAMES, c["normal"])] + ["", "[colors.bright]"] + \
            [f"{n} = {q(v)}" for n, v in zip(NAMES, c["bright"])]
    return "\n".join(lines) + "\n"


def target() -> Path:
    return usercmd.config_dir() / "alacritty" / "alacritty.toml"


def apply(s: dict, log=usercmd.log) -> Path:
    f = target()
    f.parent.mkdir(parents=True, exist_ok=True)
    src = usercmd.readable(f)
    if src.is_file():
        old = src.read_text(encoding="utf-8", errors="replace")
        bak = f.with_name(f.name + ".linuxaio-original")
        if not old.startswith(MARK) and not bak.exists():
            shutil.copy2(src, bak)                    # fremde Config einmal sichern
            log(f"Backup: {bak}")
    f.write_text(toml(s), encoding="utf-8")
    log(f"→ {f}")
    return f
