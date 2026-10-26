"""Laptop-Tasten automatisch einrichten – wie unter KDE, ohne dass jemand etwas einstellen muss. Ohne Qt.

Eigene kleine Datei ~/.config/hypr/hyprstudio_laptop.lua (bzw. .conf) nur mit Helligkeit, Lautstärke, Stumm,
Mikrofon und Medien-Tasten, eingebunden in hyprland.lua/.conf. Der übrige Look bleibt unangetastet.
Läuft direkt nach der Installation (hypr-studio --laptop-keys) und bei jedem Start der App.
"""

import json
import os
import shutil
from pathlib import Path

from . import hardware, model
from .model import MARK

INCLUDE = {
    "lua": ('require("hyprstudio_laptop")', "--"),
    "conf": ("source = ~/.config/hypr/hyprstudio_laptop.conf", "#"),
}
CONFIG = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config")) / "LabtosoStudios" / "hyprstudio.json"


def file_for(hdir: Path, fmt: str) -> Path:
    return hdir / f"hyprstudio_laptop.{fmt}"


def content(s: dict, fmt: str) -> str:
    c = INCLUDE[fmt][1]
    lines = [f"{c} {MARK} – Laptop-Tasten, automatisch erzeugt (Sonstiges › Laptop-Tasten)"]
    if s["laptop_keys"]:
        for b in model.laptop_binds(s):
            lines += [model.bind_lua(b)] if fmt == "lua" else model.bind_conf(b)
    else:
        lines.append(f"{c} ausgeschaltet")
    return "\n".join(lines) + "\n"


def _own_old_binds(hdir: Path, fmt: str, log) -> None:
    """1.26.0 schrieb die Tasten in hyprstudio.lua – dort entfernen (eigene, erzeugte Zeilen), sonst doppelt."""
    f = hdir / f"hyprstudio.{fmt}"
    if not f.is_file():
        return
    lines = f.read_text(encoding="utf-8", errors="replace").splitlines(keepends=True)
    drop = set(model.foreign_key_lines("".join(lines), fmt))
    if drop:
        f.write_text("".join(ln for i, ln in enumerate(lines) if i not in drop), encoding="utf-8")
        log(f"Laptop-Tasten aus {f.name} in {file_for(hdir, fmt).name} verschoben")


def include_extra(hdir: Path, stem: str, content_fn, label: str, log) -> str | None:
    """Kleine Zusatzdatei hyprstudio_<stem>.lua/.conf schreiben und einmal in hyprland.lua/.conf einbinden.
    Gibt das Format zurück (oder None, wenn es keine Hauptdatei gibt). Der übrige Look bleibt unangetastet."""
    fmt = model.hypr_format(hdir)
    main = hdir / f"hyprland.{fmt}" if fmt else None
    if not (main and main.is_file()):
        log(f"{label}: keine hyprland.lua/.conf gefunden")
        return None
    target = hdir / f"hyprstudio_{stem}.{fmt}"
    new = content_fn(fmt)
    if not target.is_file() or target.read_text(encoding="utf-8", errors="replace") != new:
        target.write_text(new, encoding="utf-8")
        log(f"→ {target}")
    text = main.read_text(encoding="utf-8", errors="replace")
    c = "--" if fmt == "lua" else "#"
    line = f'require("hyprstudio_{stem}")' if fmt == "lua" else f"source = ~/.config/hypr/hyprstudio_{stem}.conf"
    if line not in (ln.strip() for ln in text.splitlines()):
        bak = main.with_name(main.name + ".hyprstudio-original")
        if not bak.exists():
            shutil.copy2(main, bak)
            log(f"Sicherung: {bak}")
        with open(main, "a", encoding="utf-8") as fh:
            fh.write(("" if text.endswith("\n") or not text else "\n") + f"\n{c} {MARK} – {label}\n{line}\n")
        log(f"+ {main}: {line}")
    return fmt


def write(hdir: Path, s: dict, log) -> bool:
    """Datei schreiben, einmal einbinden, gleiche Tasten in der Hauptdatei auskommentieren."""
    fmt = include_extra(hdir, "laptop", lambda f: content(s, f), "Laptop-Tasten", log)
    if not fmt:
        return False
    model.manage_laptop_keys(hdir, s, log)
    _own_old_binds(hdir, fmt, log)
    return True


def auto(s: dict, log) -> bool:
    """Auf Laptops einschalten, solange niemand selbst geschaltet hat; Datei aktuell halten.
    Gibt zurück, ob sich am Zustand etwas geändert hat."""
    changed = False
    if not s["laptop_managed"] and hardware.detect()["laptop"]:
        s["laptop_keys"], s["laptop_managed"] = True, True
        changed = True
    tool = hardware.volume_tool()
    if s["vol_tool"] != tool:
        s["vol_tool"] = tool
        changed = True
    if s["laptop_managed"]:
        hdir = model.config_home() / "hypr"
        target = file_for(hdir, model.hypr_format(hdir) or "lua")
        fresh = not target.is_file()
        if write(hdir, s, log) and (changed or fresh):
            model.run(["hyprctl", "reload"], log, quiet=True)
    return changed


def cli() -> int:
    """hypr-studio --laptop-keys: ohne Fenster einrichten (läuft nach der Installation)."""
    try:
        cfg = json.loads(CONFIG.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        cfg = {"lang": "en", "theme": "blue"}
    if cfg.get("hypr_format") in ("lua", "conf"):
        model.FORMAT = cfg["hypr_format"]
    s = model.hypr_state(cfg.get("hypr"))
    dev = hardware.detect()
    print(f"Hypr Studio: {hardware.label(dev)} ({'Laptop' if dev['laptop'] else 'kein Laptop'})")
    auto(s, print)
    if cfg.get("sys_actions", True):  # Herunterfahren/Neustart … gleich nach der Installation in allen Launchern
        from . import packages, sysactions
        sysactions.apply(True, print, packages.installed("hyprlock"))
    cfg["hypr"] = s
    try:
        CONFIG.parent.mkdir(parents=True, exist_ok=True)
        CONFIG.write_text(json.dumps(cfg, ensure_ascii=False), encoding="utf-8")
    except OSError as e:
        print(e)
    return 0
