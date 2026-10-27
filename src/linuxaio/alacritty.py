"""Alacritty configurator: the Kitty editor's state (same presets, same options) → ~/.config/alacritty/alacritty.toml.
Everything Alacritty can do is mapped; the rest is listed in UNSUPPORTED and shown disabled in the editor.
A foreign file is backed up once as alacritty.toml.linuxaio-original. No Qt."""

import json
import shutil
from pathlib import Path

from hyprstudio import kitty

from . import usercmd

MARK = "# Linux AIO – generated, overwritten when applied"
NAMES = ("black", "red", "green", "yellow", "blue", "magenta", "cyan", "white")
# only in Alacritty (not in Kitty's state): cursor thickness (share of a cell), blinking stops after n seconds
EXTRA = {"thickness": 0.15, "blink_timeout": 5.0}
# Kitty options without an Alacritty counterpart
UNSUPPORTED = {"dynamic_opacity", "display_server", "ligatures", "blink_curve", "trail", "trail_decay_fast",
               "trail_decay_slow", "tab_edge", "tab_style", "tab_powerline", "tab_title", "url", "tab_active_bg",
               "tab_active_fg", "tab_inactive_bg", "tab_inactive_fg", "url_style", "audio_bell", "confirm_close"}


def state(saved: dict | None) -> dict:
    """Kitty state plus Alacritty extras; without a saved state preset „01“."""
    s = kitty.state(saved) if saved else kitty.preset("01")
    for k, v in EXTRA.items():
        x = (saved or {}).get(k, v)
        s[k] = float(x) if isinstance(x, (int, float)) and not isinstance(x, bool) else v
    return s


def _c(x: str) -> str:
    return kitty._c(x)


def toml(s: dict) -> str:
    q = json.dumps                                   # TOML strings like JSON strings
    pal = [_c(c) for c in s["palette"]]
    blink = float(s["blink"]) > 0
    shape = {"block": "Block", "underline": "Underline"}.get(s["cursor_shape"], "Beam")
    lines = [MARK, "",
             "[window]", f"padding = {{ x = {int(s['padding'])}, y = {int(s['padding'])} }}",
             f"opacity = {float(s['opacity']):.2f}", f"blur = {'true' if s['blur'] else 'false'}",
             f'decorations = {q("Full" if s["decorations"] == "no" else "None")}', "",
             "[font]", f"size = {float(s['font_size']):.1f}", f"normal = {{ family = {q(s['font'])} }}", "",
             "[cursor]", f"style = {{ shape = {q(shape)}, blinking = {q('On' if blink else 'Off')} }}",
             f"thickness = {min(1.0, max(0.05, float(s['thickness']))):.2f}", "unfocused_hollow = true"]
    if blink:
        lines += [f"blink_interval = {max(10, int(float(s['blink']) * 1000))}",
                  f"blink_timeout = {int(s['blink_timeout'])}"]
    lines += ["", "[scrolling]", f"history = {min(100000, int(s['scrollback']))}", "",
              "[selection]", f"save_to_clipboard = {'true' if s['copy_on_select'] else 'false'}", "",
              "[bell]", f"duration = {100 if s['bell_flash'] else 0}", f"color = {q(_c(s['fg']))}", "",
              "[colors.primary]", f"background = {q(_c(s['bg']))}", f"foreground = {q(_c(s['fg']))}", "",
              "[colors.cursor]", f"cursor = {q(_c(s['cursor']))}", f"text = {q(_c(s['bg']))}", "",
              "[colors.selection]", f"background = {q(_c(s['sel_bg']))}", f"text = {q(_c(s['sel_fg']))}", "",
              "[colors.normal]"] + [f"{n} = {q(v)}" for n, v in zip(NAMES, pal[:8])] + \
             ["", "[colors.bright]"] + [f"{n} = {q(v)}" for n, v in zip(NAMES, pal[8:16])]
    cmds = (["fastfetch"] if s["fastfetch"] else []) + list(s["start_cmds"])
    if cmds:                                         # „Beim Öffnen“: Befehle, danach die normale Shell
        line = "; ".join(cmds) + '; exec "${SHELL:-/bin/bash}"'
        lines += ["", "[terminal]", f'shell = {{ program = "/bin/sh", args = ["-c", {q(line)}] }}']
    return "\n".join(lines) + "\n"


def target() -> Path:
    return usercmd.config_dir() / "alacritty" / "alacritty.toml"


def backup() -> Path:
    f = target()
    return f.with_name(f.name + ".linuxaio-original")


def apply(s: dict, log=usercmd.log) -> Path:
    f = target()
    f.parent.mkdir(parents=True, exist_ok=True)
    src = usercmd.readable(f)
    if src.is_file():
        old = src.read_text(encoding="utf-8", errors="replace")
        if not old.startswith(MARK) and not backup().exists():
            shutil.copy2(src, backup())               # fremde Config einmal sichern
            log(f"Backup: {backup()}")
    f.write_text(toml(s), encoding="utf-8")
    log(f"→ {f}")
    return f


def undo(log=usercmd.log) -> bool:
    f, bak = target(), backup()
    if bak.is_file():
        f.write_bytes(bak.read_bytes())
        log(f"{bak} → {f}")
        return True
    if f.is_file() and f.read_text(encoding="utf-8", errors="replace").startswith(MARK):
        f.unlink()
        log(f"− {f}")
        return True
    return False
