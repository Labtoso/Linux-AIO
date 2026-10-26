"""Desktop integration: when run as AppImage or .pyz (no package installed), register a start menu entry with the
same ID as the window (Wayland app_id = desktop file name) plus the Tux icon in hicolor, so docks show the right
icon and the launch finishes cleanly. No Qt needed for the files; the PNG is rendered by the caller."""

import os
import sys
from pathlib import Path

import linuxaio
from . import usercmd

SYSTEM_ENTRY = Path("/usr/share/applications") / f"{linuxaio.APP_ID}.desktop"


def launcher() -> str | None:
    """Command that starts this copy of the app (AppImage path or python + .pyz/package)."""
    appimage = os.environ.get("APPIMAGE")
    if appimage and Path(appimage).is_file():
        return f'"{appimage}"'
    main = Path(sys.argv[0]).resolve() if sys.argv and sys.argv[0] else None
    if main and main.suffix == ".pyz" and main.is_file():
        return f'python3 "{main}"'
    return None


def entry_text(exec_cmd: str) -> str:
    return (f"[Desktop Entry]\nType=Application\nName={linuxaio.APP_NAME}\n"
            "Comment=Desktops, SDDM, Plymouth and desktop settings in one place\n"
            "Comment[de]=Desktops, SDDM, Plymouth und Desktop-Einstellungen an einem Ort\n"
            f"Exec={exec_cmd}\nIcon={linuxaio.APP_ID}\nTerminal=false\nCategories=System;Settings;\n"
            f"StartupWMClass={linuxaio.APP_ID}\nStartupNotify=false\n"
            "Keywords=desktop;gnome;plasma;hyprland;sddm;plymouth;cursor;fonts;tweaks;\n")


def ensure(render_png=None, log=usercmd.log) -> bool:
    """Write/refresh the user entry. render_png(size, path) draws the icon. Returns True if something changed."""
    if usercmd.DRY or SYSTEM_ENTRY.is_file():
        return False
    cmd = launcher()
    if not cmd:
        return False
    data = Path(os.environ.get("XDG_DATA_HOME", Path.home() / ".local" / "share"))
    entry = data / "applications" / f"{linuxaio.APP_ID}.desktop"
    text = entry_text(cmd)
    changed = False
    try:
        old = entry.read_text(encoding="utf-8") if entry.is_file() else ""
        if old != text:
            entry.parent.mkdir(parents=True, exist_ok=True)
            entry.write_text(text, encoding="utf-8")
            changed = True
            log(f"→ {entry}")
        for size in (48, 128, 256):
            icon = data / "icons" / "hicolor" / f"{size}x{size}" / "apps" / f"{linuxaio.APP_ID}.png"
            if not icon.is_file() and render_png:
                icon.parent.mkdir(parents=True, exist_ok=True)
                render_png(size, icon)
                changed = True
    except OSError as e:
        log(f"Startmenü-Eintrag: {e}")
        return False
    if changed:
        usercmd.query(["update-desktop-database", str(entry.parent)])
        usercmd.query(["gtk-update-icon-cache", "-q", "-t", str(data / "icons" / "hicolor")])
    return changed


def clean_env(env: dict) -> dict:
    """Launch/startup tokens must not leak into child processes (otherwise the desktop may keep a busy cursor)."""
    for k in ("DESKTOP_STARTUP_ID", "XDG_ACTIVATION_TOKEN"):
        env.pop(k, None)
    return env
