"""Hyprpaper (Hintergrundbild) und Hyprcursor (Mauszeiger). Ohne Qt.

hyprpaper.conf (aktuelle Syntax, Wiki Aug. 2026):
    wallpaper {
        monitor = DP-1          # leer = für alle (Fallback)
        path = ~/bild.jpg       # Bild oder Ordner (Ordner = Diashow)
        fit_mode = cover        # contain | cover | tile | fill
        timeout = 30            # nur bei Ordner
        order = random          # nur bei Ordner
    }
    splash = false
    ipc = true
"""

import json
import os
import shutil
from pathlib import Path

from . import model
from .model import MARK

FITS = ("cover", "contain", "fill", "tile")
IMAGE_EXT = (".png", ".jpg", ".jpeg", ".webp", ".jxl", ".bmp")

PAPER_DEFAULT = {"path": "", "fit": "cover", "monitors": {}, "timeout": 30, "random": False, "recursive": False,
                 "splash": False}


def paper_state(saved: dict | None) -> dict:
    s = json.loads(json.dumps(PAPER_DEFAULT))
    for k, v in (saved or {}).items():
        if k in s and type(v) is type(s[k]):
            s[k] = v
    if s["fit"] not in FITS:
        s["fit"] = "cover"
    mons = {}
    for name, m in s["monitors"].items():
        if isinstance(m, dict) and isinstance(m.get("path"), str) and m["path"]:
            mons[str(name)] = {"path": m["path"], "fit": m.get("fit") if m.get("fit") in FITS else s["fit"]}
    s["monitors"] = mons
    return s


def _block(monitor: str, path: str, fit: str, s: dict) -> list:
    out = ["wallpaper {", f"    monitor = {monitor}", f"    path = {path}", f"    fit_mode = {fit}"]
    if path and not Path(os.path.expanduser(path)).suffix.lower() in IMAGE_EXT:  # Ordner → Diashow
        out.append(f"    timeout = {s['timeout']}")
        if s["random"]:
            out.append("    order = random")
        if s["recursive"]:
            out.append("    recursive = true")
    out.append("}")
    return out


def paper_conf(s: dict) -> str:
    lines = [f"# {MARK} – automatisch erzeugt, wird beim Anwenden überschrieben", ""]
    for name, m in sorted(s["monitors"].items()):
        lines += _block(name, m["path"], m["fit"], s) + [""]
    if s["path"]:
        lines += _block("", s["path"], s["fit"], s) + [""]
    lines += [f"splash = {'true' if s['splash'] else 'false'}", "ipc = true"]
    return "\n".join(lines) + "\n"


def apply_paper(s: dict, hdir: Path, log) -> tuple[bool, str]:
    if not s["path"] and not s["monitors"]:
        return False, "paper.nopath"
    hdir.mkdir(parents=True, exist_ok=True)
    f = hdir / "hyprpaper.conf"
    bak = model._backup_foreign(f)
    if bak:
        log(f"Sicherung: {bak}")
    f.write_text(paper_conf(s), encoding="utf-8")
    log(f"→ {f}")
    restart_paper(log)
    return True, ""


def restart_paper(log) -> None:
    model.run(["pkill", "-x", "hyprpaper"], log, quiet=True)
    if model.DRY:
        log("$ hyprpaper &  (Testmodus)")
        return
    import subprocess
    import time
    time.sleep(0.3)
    try:
        subprocess.Popen(["hyprpaper"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True,
                         cwd=str(Path.home()))
        log("$ hyprpaper &")
    except OSError as e:
        log(f"hyprpaper: {e}")


# ---------- Hyprcursor / XCursor ----------

def icon_dirs() -> list:
    home = Path.home()
    dirs = [home / ".local/share/icons", home / ".icons", Path("/usr/share/icons")]
    if model.DRY:
        dirs = [model.CACHE / "testmodus" / "icons"]
    return dirs


def cursor_themes() -> list:
    """[(name, art)] – art = "hyprcursor" (manifest.hl/.toml) oder "xcursor" (Ordner cursors/)."""
    found = {}
    for d in icon_dirs():
        if not d.is_dir():
            continue
        try:
            entries = sorted(d.iterdir())
        except OSError:
            continue
        for e in entries:
            if not e.is_dir():
                continue
            if (e / "manifest.hl").is_file() or (e / "manifest.toml").is_file():
                found[e.name] = "hyprcursor"
            elif (e / "cursors").is_dir() and e.name not in found:
                found[e.name] = "xcursor"
    return sorted(found.items(), key=lambda x: x[0].lower())


def set_cursor_live(theme: str, size: int, gtk: bool, log) -> None:
    if theme:
        model.run(["hyprctl", "setcursor", theme, str(size)], log)
        if gtk and (model.DRY or shutil.which("gsettings")):
            model.run(["gsettings", "set", "org.gnome.desktop.interface", "cursor-theme", theme], log, quiet=True)
            model.run(["gsettings", "set", "org.gnome.desktop.interface", "cursor-size", str(size)], log, quiet=True)
