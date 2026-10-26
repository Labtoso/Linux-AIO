"""Herunterfahren, Neustart, Abmelden, Sperren als App-Einträge – damit Rofi, Wofi, Fuzzel und L Launcher sie finden.
Je Aktion ein englischer und ein deutscher Eintrag, so wirkt „shutdown“ genauso wie „herunterfahren“, egal welche
Sprache eingestellt ist. Enter führt den Befehl sofort aus. Ausschalten = Einträge verstecken (nichts wird gelöscht).
Ohne Qt."""

import os
from pathlib import Path

from . import model
from .model import MARK

# (Schlüssel, Befehl, Symbol, [(Sprache, Name)], Suchwörter)
ACTIONS = [
    ("poweroff", "systemctl poweroff", "system-shutdown", [("en", "Shutdown"), ("de", "Herunterfahren")],
     "shutdown;power off;poweroff;turn off;herunterfahren;ausschalten;abschalten;"),
    ("reboot", "systemctl reboot", "system-reboot", [("en", "Restart"), ("de", "Neustart")],
     "restart;reboot;neustart;neu starten;"),
    # ganz abmelden bis zum Anmeldebildschirm (SDDM): die ganze Sitzung beenden, nicht nur sperren
    ("logout", 'sh -c "loginctl terminate-session self || hyprctl dispatch exit"', "system-log-out",
     [("en", "Logout"), ("de", "Abmelden"), ("de2", "Ausloggen")],
     "logout;log out;sign out;abmelden;ausloggen;abmeldung;"),
    ("lock", "hyprlock", "system-lock-screen", [("en", "Lock screen"), ("de", "Sperren")],
     "lock;lock screen;sperren;bildschirm sperren;"),
]


def apps_dir() -> Path:
    if model.DRY:
        return model.CACHE / "testmodus" / "applications"
    return Path(os.environ.get("XDG_DATA_HOME", Path.home() / ".local/share")) / "applications"


def entry(key: str, lang: str, name: str, cmd: str, icon: str, words: str, shown: bool) -> str:
    return "\n".join([
        f"# {MARK} – Systembefehl für alle Launcher (Sonstiges › Launcher › Systembefehle)",
        "[Desktop Entry]", "Type=Application", f"Name={name}", f"Exec={cmd}", f"Icon={icon}",
        f"Keywords={words}", "Categories=System;", "Terminal=false", f"NoDisplay={'false' if shown else 'true'}",
        f"Hidden={'false' if shown else 'true'}", f"X-HyprStudio={key}-{lang}", ""])


def files(on: bool, lock_ok: bool = True) -> list:
    out = []
    for key, cmd, icon, names, words in ACTIONS:
        shown = on and (key != "lock" or lock_ok)
        for lang, name in names:
            out.append((apps_dir() / f"hyprstudio-{key}-{lang}.desktop", entry(key, lang, name, cmd, icon, words, shown)))
    return out


def apply(on: bool, log, lock_ok: bool = True) -> int:
    """Einträge schreiben (an) bzw. verstecken (aus). Gibt die Zahl der geänderten Dateien zurück."""
    n = 0
    for f, text in files(on, lock_ok):
        if f.is_file() and f.read_text(encoding="utf-8", errors="replace") == text:
            continue
        f.parent.mkdir(parents=True, exist_ok=True)
        f.write_text(text, encoding="utf-8")
        n += 1
    if n:
        log(f"→ {apps_dir()}: {n} Systembefehle {'an' if on else 'aus'}")
    return n
