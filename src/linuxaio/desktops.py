"""Desktops für den Tab „Einstellungen“: welche gibt es, welche sind installiert, welcher läuft. Ohne Qt."""

import os
from dataclasses import dataclass

from . import catalog, system


@dataclass(frozen=True)
class Entry:
    id: str
    name: str
    sessions: tuple
    detect: tuple


# Zusätzlich zum Katalog (dort nur, was sich installieren lässt)
EXTRA = (
    Entry("niri", "niri", ("niri",), ("niri",)),
    Entry("pantheon", "Pantheon", ("pantheon",), ("Pantheon",)),
    Entry("deepin", "Deepin", ("deepin",), ("Deepin", "DDE")),
    Entry("enlightenment", "Enlightenment", ("enlightenment",), ("Enlightenment",)),
)

ORDER = ("plasma", "gnome", "hyprland", "xfce", "cinnamon", "mate", "budgie", "lxqt", "cosmic", "sway", "i3",
         "niri", "pantheon", "deepin", "enlightenment")


def entries() -> list:
    items = {de.id: Entry(de.id, de.name, de.sessions, de.detect) for de in catalog.DESKTOPS}
    items.update({e.id: e for e in EXTRA})
    return [items[k] for k in ORDER if k in items] + [v for k, v in items.items() if k not in ORDER]


def get(de_id: str) -> Entry | None:
    return next((e for e in entries() if e.id == de_id), None)


def installed(e: Entry) -> bool:
    if os.environ.get("DEMANAGER_DRYRUN") == "1" and os.environ.get("LINUXAIO_ALL_DE") == "1":
        return True
    return any(system.session_file(s) for s in e.sessions)


def current(env: dict | None = None) -> str | None:
    """Laufender Desktop aus XDG_CURRENT_DESKTOP (bzw. XDG_SESSION_DESKTOP)."""
    env = os.environ if env is None else env
    cur = env.get("XDG_CURRENT_DESKTOP", "") + ":" + env.get("XDG_SESSION_DESKTOP", "")
    parts = [p.strip().lower() for p in cur.split(":") if p.strip()]
    for p in parts:   # erster Eintrag zählt (Budgie:GNOME → Budgie)
        for e in entries():
            if p in (x.lower() for x in e.detect + e.sessions + (e.id,)):
                return e.id
    return None


def gtk_based(de_id: str) -> bool:
    """Desktops ohne eigene Einstellungen für GTK-Themes: hier gelten gsettings/settings.ini."""
    return de_id in ("hyprland", "sway", "i3", "niri", "enlightenment", "cosmic")
