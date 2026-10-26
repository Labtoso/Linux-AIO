"""Mauszeiger- und Symbol-Themes: aus einem Ordner installieren (nach ~/.local/share/icons), auflisten,
pro Desktop anwenden und den aktiven Stand lesen. Ohne Qt (Vorschau-Bilder macht look_page.py)."""

import os
import re
import shutil
import struct
from pathlib import Path

from . import usercmd

DEFAULT_CURSOR_DIR = Path.home() / "Cursors"     # Quelle „Ordner“, im Programm änderbar
DEFAULT_ICON_DIR = Path.home() / "Icons"

CURSOR_PREVIEW = ("left_ptr", "hand2", "xterm", "watch")
ICON_PREVIEW = ("folder", "user-home", "utilities-terminal", "text-x-generic", "internet-web-browser",
                "system-file-manager", "accessories-text-editor", "preferences-system")
_ALIASES = {"hand2": ("hand2", "pointer", "hand1"), "watch": ("watch", "wait", "left_ptr_watch"),
            "xterm": ("xterm", "text"), "left_ptr": ("left_ptr", "default", "arrow")}


def user_icons() -> Path:
    """Ziel der Installation (ohne root)."""
    return usercmd.data_dir() / "icons"


def icon_dirs() -> list:
    dirs = [user_icons(), Path.home() / ".local" / "share" / "icons", Path.home() / ".icons"]
    for d in os.environ.get("XDG_DATA_DIRS", "/usr/local/share:/usr/share").split(":"):
        if d:
            dirs.append(Path(d) / "icons")
    out = []
    for d in dirs:
        if d not in out:
            out.append(d)
    return out


def _index_name(folder: Path) -> str:
    try:
        for ln in (folder / "index.theme").read_text(encoding="utf-8", errors="replace").splitlines():
            if ln.startswith("Name="):
                return ln.split("=", 1)[1].strip()
    except OSError:
        pass
    return folder.name


def is_cursor_theme(folder: Path) -> bool:
    return (folder / "cursors").is_dir()


def is_icon_theme(folder: Path) -> bool:
    """Symbol-Theme = index.theme mit Directories= (reine Mauszeiger-Themes haben keins)."""
    try:
        text = (folder / "index.theme").read_text(encoding="utf-8", errors="replace")
    except OSError:
        return False
    return bool(re.search(r"^Directories\s*=\s*\S", text, re.M)) and folder.name not in ("default", "hicolor")


def _check(kind: str):
    return is_cursor_theme if kind == "cursor" else is_icon_theme


def installed_path(name: str, kind: str = "cursor") -> Path | None:
    for d in icon_dirs():
        p = d / name
        if p.is_dir() and _check(kind)(p):
            return p
    return None


def list_installed(kind: str = "cursor") -> dict:
    """name → Ordner (erste Fundstelle gewinnt, wie bei XDG)."""
    found = {}
    for d in icon_dirs():
        try:
            subs = sorted(d.iterdir())
        except OSError:
            continue
        for p in subs:
            if p.name not in found and p.is_dir() and _check(kind)(p):
                found[p.name] = p
    return found


def list_folder(folder: Path, kind: str = "cursor") -> dict:
    """Themes im Quellordner (ein Ordner pro Theme, eine Ebene tief)."""
    out = {}
    try:
        subs = sorted(Path(folder).iterdir())
    except OSError:
        return out
    for p in subs:
        if p.is_dir() and _check(kind)(p):
            out[p.name] = p
        elif p.is_dir():                 # Cloud-Ordner ohne Symlinks: Theme als Archiv <Name>/<Name>.tar.xz
            arc = next(iter(sorted(p.glob("*.tar.xz"))), None)
            if arc is not None:
                out[p.name] = arc
    return out


def items(folder: Path, kind: str = "cursor") -> list:
    """Kacheln: Themes aus dem Ordner (installierbar) und alles schon Installierte."""
    src = list_folder(folder, kind)
    inst = list_installed(kind)
    result = []
    for name in list(src) + [n for n in inst if n not in src]:
        path = inst.get(name)
        s_ = src.get(name)
        label_from = path or (s_ if s_ is not None and s_.is_dir() else None)
        result.append({"name": name, "label": _index_name(label_from) if label_from else name, "src": s_,
                       "installed": path, "color": color_of(name)})
    return result


def color_of(name: str) -> str | None:
    """Bibata-Material-<Farbe> → Farbname (für die Suche)."""
    m = re.match(r"Bibata-Material-(.+)$", name)
    return m.group(1).replace("-", " ") if m else None


def install(src: Path, log=usercmd.log) -> Path:
    """Theme-Ordner (oder .tar.xz-Archiv) nach ~/.local/share/icons. Vorhandenes wird nie überschrieben."""
    if src.is_file():
        from . import remote
        new = remote.safe_extract(src.read_bytes(), user_icons(), None, allow_links=True)
        name = src.parent.name
        log(f"→ {user_icons() / name}")
        return user_icons() / name if (user_icons() / name).exists() else (new[0] if new else user_icons() / name)
    dest = user_icons() / src.name
    if dest.exists():
        log(f"schon da: {dest}")
        return dest
    dest.parent.mkdir(parents=True, exist_ok=True)
    tmp = dest.with_name(dest.name + ".linuxaio-tmp")
    if tmp.exists():
        shutil.rmtree(tmp)      # eigener halbfertiger Kopier-Rest
    shutil.copytree(src, tmp, symlinks=True)
    os.replace(tmp, dest)
    log(f"→ {dest}" + ("   [Testmodus]" if usercmd.DRY else ""))
    return dest


# ---------- Xcursor lesen (für die Vorschau) ----------

def read_xcursor(path: Path, want: int = 32):
    """Erstes Bild der passenden Größe: (breite, höhe, BGRA-Bytes) oder None."""
    try:
        data = path.read_bytes()
    except OSError:
        return None
    if data[:4] != b"Xcur" or len(data) < 16:
        return None
    try:
        hsize, _ver, ntoc = struct.unpack_from("<III", data, 4)
        best = None
        for i in range(min(ntoc, 512)):
            typ, sub, pos = struct.unpack_from("<III", data, hsize + i * 12)
            if typ != 0xFFFD0002:
                continue
            if best is None or abs(sub - want) < abs(best[0] - want):
                best = (sub, pos)
        if not best:
            return None
        pos = best[1]
        _h, _t, _s, _v, w, h = struct.unpack_from("<IIIIII", data, pos)
        start = pos + 36
        if w > 512 or h > 512 or start + w * h * 4 > len(data):
            return None
        return w, h, data[start:start + w * h * 4]
    except struct.error:
        return None


def cursor_file(theme: Path, name: str) -> Path | None:
    for n in _ALIASES.get(name, (name,)):
        p = theme / "cursors" / n
        if p.is_file():
            return p
    return None


def icon_file(theme: Path, name: str) -> Path | None:
    """Ein Symbol aus dem Theme finden (übliche Ordner-Schemata, ohne alles zu durchsuchen)."""
    sizes = ("scalable", "48x48", "48", "64x64", "64", "32x32", "32", "256x256", "128x128", "96")
    ctxs = ("places", "apps", "mimetypes", "categories", "devices", "actions")
    for ctx in ctxs:
        for s in sizes:
            for base in (theme / s / ctx, theme / ctx / s):
                for ext in (".svg", ".png"):
                    p = base / (name + ext)
                    if p.is_file():
                        return p
    return None


# ---------- Anwenden pro Desktop ----------

def _hypr_include(name: str, size: int, log):
    """Hyprland: Zusatzdatei hyprstudio_cursor.lua/.conf (über Hypr Studios Einbindung)."""
    try:
        from hyprstudio import laptopkeys, model
    except ImportError:
        return
    hdir = model.config_home() / "hypr"

    def content(fmt):
        if fmt == "lua":
            return ("-- Linux AIO: Mauszeiger\n"
                    f'hl.env("XCURSOR_THEME", "{name}")\nhl.env("XCURSOR_SIZE", "{size}")\n'
                    f'hl.env("HYPRCURSOR_THEME", "{name}")\nhl.env("HYPRCURSOR_SIZE", "{size}")\n')
        return ("# Linux AIO: Mauszeiger\n"
                f"env = XCURSOR_THEME,{name}\nenv = XCURSOR_SIZE,{size}\n"
                f"env = HYPRCURSOR_THEME,{name}\nenv = HYPRCURSOR_SIZE,{size}\n")
    laptopkeys.include_extra(hdir, "cursor", content, "Mauszeiger (Linux AIO)", log)


def _gtk_settings(key: str, value: str) -> None:
    for v in ("gtk-3.0", "gtk-4.0"):
        usercmd.ini_set(usercmd.config_dir() / v / "settings.ini", "Settings", key, value)


def _default_index(name: str) -> None:
    """X11/XWayland-Programme ohne Desktop-Einstellung: ~/.icons/default/index.theme erbt das Theme."""
    p = usercmd.home() / ".icons" / "default" / "index.theme"
    usercmd.ini_set(p, "Icon Theme", "Inherits", name)


def apply_cursor(de: str, name: str, size: int = 24, log=usercmd.log) -> bool:
    gs = usercmd.gset
    ok = True
    if de in ("gnome", "budgie", "pantheon", "cosmic"):
        ok = gs("org.gnome.desktop.interface", "cursor-theme", name)
        gs("org.gnome.desktop.interface", "cursor-size", int(size))
    elif de == "cinnamon":
        ok = gs("org.cinnamon.desktop.interface", "cursor-theme", name)
        gs("org.cinnamon.desktop.interface", "cursor-size", int(size))
    elif de == "mate":
        ok = gs("org.mate.peripherals-mouse", "cursor-theme", name)
        gs("org.mate.peripherals-mouse", "cursor-size", int(size))
    elif de == "xfce":
        ok = usercmd.run(["xfconf-query", "-c", "xsettings", "-p", "/Gtk/CursorThemeName", "-s", name])[0]
        usercmd.run(["xfconf-query", "-c", "xsettings", "-p", "/Gtk/CursorThemeSize", "-s", str(size)])
    elif de == "plasma":
        ok = usercmd.run(["plasma-apply-cursortheme", name])[0]
        usercmd.ini_set(usercmd.config_dir() / "kcminputrc", "Mouse", "cursorSize", str(size))
    elif de == "lxqt":
        p = usercmd.config_dir() / "lxqt" / "session.conf"
        usercmd.ini_set(p, "Mouse", "cursor_theme", name)
        usercmd.ini_set(p, "Mouse", "cursor_size", str(size))
    elif de == "hyprland":
        ok = usercmd.run(["hyprctl", "setcursor", name, str(size)])[0]
        _hypr_include(name, size, log)
        if "org.gnome.desktop.interface" in usercmd.schemas() or usercmd.DRY:
            gs("org.gnome.desktop.interface", "cursor-theme", name)   # GTK-Programme
    elif de == "sway":
        ok = usercmd.run(["swaymsg", "seat", "*", "xcursor_theme", name, str(size)])[0]
    elif de == "niri":
        log("niri: in ~/.config/niri/config.kdl unter cursor { xcursor-theme \"…\" } eintragen")
    if de not in ("plasma", "gnome", "budgie", "cinnamon", "mate", "pantheon"):
        _gtk_settings("gtk-cursor-theme-name", name)
        _gtk_settings("gtk-cursor-theme-size", str(size))
    _default_index(name)
    return ok


def apply_icons(de: str, name: str, log=usercmd.log) -> bool:
    gs = usercmd.gset
    ok = True
    if de in ("gnome", "budgie", "pantheon", "cosmic"):
        ok = gs("org.gnome.desktop.interface", "icon-theme", name)
    elif de == "cinnamon":
        ok = gs("org.cinnamon.desktop.interface", "icon-theme", name)
    elif de == "mate":
        ok = gs("org.mate.interface", "icon-theme", name)
    elif de == "xfce":
        ok = usercmd.run(["xfconf-query", "-c", "xsettings", "-p", "/Net/IconThemeName", "-s", name])[0]
    elif de == "plasma":
        tool = next((t for t in ("/usr/lib/plasma-changeicons", "/usr/libexec/plasma-changeicons",
                                 "/usr/lib64/libexec/plasma-changeicons") if Path(t).is_file()), None)
        if tool or usercmd.DRY:
            ok = usercmd.run([tool or "/usr/lib/plasma-changeicons", name])[0]
        else:
            usercmd.ini_set(usercmd.config_dir() / "kdeglobals", "Icons", "Theme", name)
    elif de == "lxqt":
        usercmd.ini_set(usercmd.config_dir() / "lxqt" / "lxqt.conf", "General", "icon_theme", name)
    else:
        if "org.gnome.desktop.interface" in usercmd.schemas() or usercmd.DRY:
            gs("org.gnome.desktop.interface", "icon-theme", name)
    if de not in ("plasma", "gnome", "budgie", "cinnamon", "mate", "pantheon"):
        _gtk_settings("gtk-icon-theme-name", name)
    return ok


def current(de: str, kind: str = "cursor") -> str:
    """Aktives Theme lesen (für das Schild „Angewendet“)."""
    key = "cursor-theme" if kind == "cursor" else "icon-theme"
    g = usercmd.gget
    if de in ("gnome", "budgie", "pantheon", "cosmic", "hyprland", "sway", "i3", "niri"):
        v = g("org.gnome.desktop.interface", key, "")
        if v and de in ("gnome", "budgie", "pantheon", "cosmic"):
            return v
        ini = usercmd.readable(usercmd.config_dir() / "gtk-3.0" / "settings.ini")
        return usercmd.ini_get(ini, "Settings", "gtk-cursor-theme-name" if kind == "cursor"
                               else "gtk-icon-theme-name", v or "")
    if de == "cinnamon":
        return g("org.cinnamon.desktop.interface", key, "") or ""
    if de == "mate":
        return g("org.mate.peripherals-mouse" if kind == "cursor" else "org.mate.interface", key, "") or ""
    if de == "xfce":
        prop = "/Gtk/CursorThemeName" if kind == "cursor" else "/Net/IconThemeName"
        return usercmd.query(["xfconf-query", "-c", "xsettings", "-p", prop]) or ""
    if de == "plasma":
        if kind == "cursor":
            return usercmd.ini_get(usercmd.readable(usercmd.config_dir() / "kcminputrc"), "Mouse", "cursorTheme", "breeze_cursors")
        return usercmd.ini_get(usercmd.readable(usercmd.config_dir() / "kdeglobals"), "Icons", "Theme", "breeze")
    if de == "lxqt":
        if kind == "cursor":
            return usercmd.ini_get(usercmd.readable(usercmd.config_dir() / "lxqt" / "session.conf"), "Mouse", "cursor_theme", "")
        return usercmd.ini_get(usercmd.readable(usercmd.config_dir() / "lxqt" / "lxqt.conf"), "General", "icon_theme", "")
    return ""
