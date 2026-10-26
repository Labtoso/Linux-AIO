"""GNOME-Einstellungen wie in GNOME Tweaks (über gsettings), Titelleisten-Knöpfe, Startprogramme,
Shell-/GTK-Themes und Erweiterungen. Ohne Qt."""

import os
import re
from pathlib import Path

from . import usercmd

IFACE = "org.gnome.desktop.interface"
WM = "org.gnome.desktop.wm.preferences"
MUTTER = "org.gnome.mutter"
USER_THEME = "user-theme@gnome-shell-extensions.gcampax.github.com"
WS_INDICATOR = "workspace-indicator@gnome-shell-extensions.gcampax.github.com"

# Optionen: (Schlüssel für Texte, Schema, gsettings-Key, Art, Auswahl)
# Art: bool | choice | int | double | font
SECTIONS = {
    "fonts": [
        ("font.ui", IFACE, "font-name", "font", None),
        ("font.doc", IFACE, "document-font-name", "font", None),
        ("font.mono", IFACE, "monospace-font-name", "font", None),
        ("font.title", WM, "titlebar-font", "font", None),
        ("font.hint", IFACE, "font-hinting", "choice", ("none", "slight", "medium", "full")),
        ("font.aa", IFACE, "font-antialiasing", "choice", ("none", "grayscale", "rgba")),
        ("font.scale", IFACE, "text-scaling-factor", "double", (0.5, 3.0, 0.05)),
    ],
    "look": [
        ("look.scheme", IFACE, "color-scheme", "choice", ("default", "prefer-dark", "prefer-light")),
        ("look.accent", IFACE, "accent-color", "choice",
         ("blue", "teal", "green", "yellow", "orange", "red", "pink", "purple", "slate")),
        ("look.anim", IFACE, "enable-animations", "bool", None),
    ],
    "windows": [
        ("win.dbl", WM, "action-double-click-titlebar", "choice",
         ("toggle-maximize", "toggle-maximize-horizontally", "toggle-maximize-vertically", "minimize",
          "lower", "menu", "none")),
        ("win.mid", WM, "action-middle-click-titlebar", "choice",
         ("none", "lower", "minimize", "toggle-maximize", "menu")),
        ("win.right", WM, "action-right-click-titlebar", "choice",
         ("menu", "none", "lower", "minimize", "toggle-maximize")),
        ("win.focus", WM, "focus-mode", "choice", ("click", "sloppy", "mouse")),
        ("win.raise", WM, "raise-on-click", "bool", None),
        ("win.modifier", WM, "mouse-button-modifier", "choice", ("<Super>", "<Alt>")),
        ("win.resize", WM, "resize-with-right-button", "bool", None),
        ("win.attach", MUTTER, "attach-modal-dialogs", "bool", None),
        ("win.center", MUTTER, "center-new-windows", "bool", None),
        ("win.edge", MUTTER, "edge-tiling", "bool", None),
    ],
    "topbar": [
        ("top.date", IFACE, "clock-show-date", "bool", None),
        ("top.weekday", IFACE, "clock-show-weekday", "bool", None),
        ("top.seconds", IFACE, "clock-show-seconds", "bool", None),
        ("top.format", IFACE, "clock-format", "choice", ("24h", "12h")),
        ("top.weeknum", "org.gnome.desktop.calendar", "show-weekdate", "bool", None),
        ("top.battery", IFACE, "show-battery-percentage", "bool", None),
        ("top.hotcorner", IFACE, "enable-hot-corners", "bool", None),
        ("top.dynamic", MUTTER, "dynamic-workspaces", "bool", None),
        ("top.numws", WM, "num-workspaces", "int", (1, 36, 1)),
        ("top.primary", MUTTER, "workspaces-only-on-primary", "bool", None),
        ("top.wsswitch", "org.gnome.shell.app-switcher", "current-workspace-only", "bool", None),
    ],
    "input": [
        ("in.accel", "org.gnome.desktop.peripherals.mouse", "accel-profile", "choice", ("default", "flat", "adaptive")),
        ("in.natural", "org.gnome.desktop.peripherals.mouse", "natural-scroll", "bool", None),
        ("in.tap", "org.gnome.desktop.peripherals.touchpad", "tap-to-click", "bool", None),
        ("in.tpnatural", "org.gnome.desktop.peripherals.touchpad", "natural-scroll", "bool", None),
        ("in.typing", "org.gnome.desktop.peripherals.touchpad", "disable-while-typing", "bool", None),
        ("in.paste", IFACE, "gtk-enable-primary-paste", "bool", None),
        ("in.overlay", MUTTER, "overlay-key", "choice", ("Super_L", "Super_R")),
    ],
}


def available(schema: str, key: str) -> bool:
    return usercmd.DRY or usercmd.has_key(schema, key)


# ---------- Schrift-Werte: „Cantarell Bold 11“ ----------

def split_font(value: str) -> tuple[str, float]:
    m = re.match(r"^(.*?)\s+(\d+(?:\.\d+)?)$", (value or "").strip())
    if not m:
        return (value or "").strip(), 11.0
    return m.group(1), float(m.group(2))


def join_font(family: str, size: float) -> str:
    s = f"{size:g}"
    return f"{family.strip()} {s}"


# ---------- Titelleisten-Knöpfe ----------

def parse_layout(value: str) -> dict:
    """'appmenu:minimize,maximize,close' → {'side': 'right', 'minimize': True, 'maximize': True}."""
    v = value or "appmenu:close"
    left, _, right = v.partition(":")
    lb = [x for x in left.split(",") if x]
    rb = [x for x in right.split(",") if x]
    side = "left" if "close" in lb else "right"
    allb = lb + rb
    if "close" not in allb:      # ohne Schließen: Seite nach den übrigen Knöpfen
        side = "left" if lb and not rb else "right"
    return {"side": side, "minimize": "minimize" in allb, "maximize": "maximize" in allb,
            "close": "close" in allb, "appmenu": "appmenu" in allb}


def build_layout(side: str, minimize: bool, maximize: bool, appmenu: bool = True, close: bool = True) -> str:
    c = ["close"] if close else []
    if side == "left":
        btns = c + (["maximize"] if maximize else []) + (["minimize"] if minimize else [])
        return ",".join(btns) + ":" + ("appmenu" if appmenu else "")
    btns = (["minimize"] if minimize else []) + (["maximize"] if maximize else []) + c
    return ("appmenu" if appmenu else "") + ":" + ",".join(btns)


# ---------- Themes ----------

def theme_dirs() -> list:
    dirs = [usercmd.data_dir() / "themes", Path.home() / ".local" / "share" / "themes", Path.home() / ".themes"]
    for d in os.environ.get("XDG_DATA_DIRS", "/usr/local/share:/usr/share").split(":"):
        if d:
            dirs.append(Path(d) / "themes")
    return list(dict.fromkeys(dirs))


def list_themes(kind: str) -> list:
    """kind: gtk (gtk-3.0) oder shell (gnome-shell)."""
    sub = "gtk-3.0" if kind == "gtk" else "gnome-shell"
    names = []
    for d in theme_dirs():
        try:
            for p in sorted(d.iterdir()):
                if (p / sub).is_dir() and p.name not in names:
                    names.append(p.name)
        except OSError:
            continue
    if kind == "gtk":
        for n in ("Adwaita", "Adwaita-dark", "HighContrast"):   # eingebaut in GTK
            if n not in names:
                names.insert(0, n)
    return names


# ---------- Erweiterungen ----------

def ext_enabled() -> list:
    v = usercmd.gget("org.gnome.shell", "enabled-extensions", [])
    return list(v) if isinstance(v, (list, tuple)) else []


def ext_installed(uuid: str) -> bool:
    from .installers import DRY_EXT
    if uuid in DRY_EXT:
        return True
    for d in (Path.home() / ".local/share/gnome-shell/extensions", Path("/usr/share/gnome-shell/extensions")):
        if (d / uuid).is_dir():
            return True
    return False


def ext_dir(uuid: str) -> Path | None:
    for d in (Path.home() / ".local/share/gnome-shell/extensions", Path("/usr/share/gnome-shell/extensions")):
        if (d / uuid / "metadata.json").is_file():
            return d / uuid
    return None


def ext_compatible(uuid: str) -> bool | None:
    """Supports the extension the running GNOME Shell? None = unknown (not installed / no shell)."""
    import json
    d = ext_dir(uuid)
    out = usercmd.query(["gnome-shell", "--version"]) or ""
    m = re.search(r"(\d+)", out)
    if not d or not m:
        return None
    try:
        versions = json.loads((d / "metadata.json").read_text(encoding="utf-8")).get("shell-version", [])
    except (OSError, ValueError):
        return None
    return any(str(v).split(".")[0] == m.group(1) for v in versions)


def version_check_off() -> bool:
    return bool(usercmd.gget("org.gnome.shell", "disable-extension-version-validation", False))


def ext_set(uuid: str, on: bool) -> bool:
    """Always via the enabled-extensions key too: gnome-extensions fails while the running shell
    does not know a freshly installed extension yet (Wayland: active after the next login)."""
    if usercmd.have("gnome-extensions") or usercmd.DRY:
        usercmd.run(["gnome-extensions", "enable" if on else "disable", uuid])
    cur = ext_enabled()
    new = [x for x in cur if x != uuid] + ([uuid] if on else [])
    if new == cur:
        return True
    return usercmd.gset("org.gnome.shell", "enabled-extensions", new)


def shell_theme() -> str:
    return usercmd.gget("org.gnome.shell.extensions.user-theme", "name", "") or ""


def set_shell_theme(name: str) -> bool:
    if USER_THEME not in ext_enabled() and ext_installed(USER_THEME):
        ext_set(USER_THEME, True)
    return usercmd.gset("org.gnome.shell.extensions.user-theme", "name", name)


# ---------- Startprogramme (~/.config/autostart) ----------

def app_dirs() -> list:
    dirs = [Path(os.environ.get("XDG_DATA_HOME", Path.home() / ".local/share")) / "applications"]
    for d in os.environ.get("XDG_DATA_DIRS", "/usr/local/share:/usr/share").split(":"):
        if d:
            dirs.append(Path(d) / "applications")
    dirs += [Path("/var/lib/flatpak/exports/share/applications"),
             Path.home() / ".local/share/flatpak/exports/share/applications"]
    return list(dict.fromkeys(dirs))


def read_desktop(path: Path) -> dict:
    d, sec = {}, None
    try:
        for ln in path.read_text(encoding="utf-8", errors="replace").splitlines():
            s = ln.strip()
            if s.startswith("["):
                sec = s
            elif sec == "[Desktop Entry]" and "=" in s and not s.startswith("#"):
                k, v = s.split("=", 1)
                d.setdefault(k.strip(), v.strip())
    except OSError:
        pass
    return d


def local_name(d: dict) -> str:
    lang = (os.environ.get("LANG") or "").split(".")[0]
    for k in (f"Name[{lang}]", f"Name[{lang[:2]}]", "Name"):
        if d.get(k):
            return d[k]
    return ""


def list_apps() -> list:
    """Alle startbaren Programme (für „Hinzufügen“)."""
    seen, out = set(), []
    for d in app_dirs():
        try:
            files = sorted(d.glob("*.desktop"))
        except OSError:
            continue
        for f in files:
            if f.name in seen:
                continue
            seen.add(f.name)
            e = read_desktop(f)
            if e.get("Type", "Application") != "Application" or e.get("NoDisplay") == "true" or e.get("Hidden") == "true":
                continue
            out.append({"file": f, "id": f.name, "name": local_name(e) or f.stem, "icon": e.get("Icon", ""),
                        "exec": e.get("Exec", "")})
    return sorted(out, key=lambda a: a["name"].lower())


def autostart_dir() -> Path:
    return usercmd.config_dir() / "autostart"


def list_autostart() -> list:
    out = []
    d = autostart_dir()
    dirs = [d] if not usercmd.DRY else [d, Path.home() / ".config/autostart"]
    seen = set()
    for folder in dirs:
        try:
            files = sorted(folder.glob("*.desktop"))
        except OSError:
            continue
        for f in files:
            if f.name in seen:
                continue
            seen.add(f.name)
            e = read_desktop(f)
            on = e.get("Hidden") != "true" and e.get("X-GNOME-Autostart-enabled", "true") != "false"
            out.append({"file": f, "id": f.name, "name": local_name(e) or f.stem, "icon": e.get("Icon", ""),
                        "enabled": on})
    return sorted(out, key=lambda a: a["name"].lower())


def autostart_add(app_file: Path) -> Path:
    dest = autostart_dir() / app_file.name
    dest.parent.mkdir(parents=True, exist_ok=True)
    if not dest.exists():
        dest.write_text(app_file.read_text(encoding="utf-8", errors="replace"), encoding="utf-8")
        usercmd.log(f"→ {dest}")
    autostart_enable(dest, True)
    return dest


def autostart_enable(path: Path, on: bool) -> None:
    """An/aus ohne zu löschen: Hidden + X-GNOME-Autostart-enabled in der eigenen Kopie."""
    dest = autostart_dir() / path.name
    if not dest.exists():
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_text(usercmd.readable(path).read_text(encoding="utf-8", errors="replace"), encoding="utf-8")
    usercmd.ini_set(dest, "Desktop Entry", "Hidden", "false" if on else "true")
    usercmd.ini_set(dest, "Desktop Entry", "X-GNOME-Autostart-enabled", "true" if on else "false")


# ---------- Anmeldebildschirm (GDM) ----------

def gdm_installed() -> bool:
    return usercmd.DRY or Path("/usr/share/gnome-shell/gnome-shell-theme.gresource").is_file()


def gdm_state() -> dict:
    """Was Linux AIO beim GDM gesetzt hat (vom Root-Helfer geschrieben)."""
    import json
    p = Path("/var/lib/labtoso-linuxaio/gdm.json")
    if usercmd.DRY:
        p = usercmd.CACHE / "testmodus" / "gdm.json"
    try:
        return json.loads(p.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return {}


# ---------- Top bar editor (Just Perfection) ----------

JP_UUID = "just-perfection-desktop@just-perfection"
JP_SCHEMA = "org.gnome.shell.extensions.just-perfection"
# element → (Just-Perfection key, zone of the bar)
BAR_ITEMS = (
    ("activities", "activities-button", "left"),
    ("clock", "clock-menu", "center"),
    ("notify", "panel-notification-icon", "right"),
    ("keyboard", "keyboard-layout", "right"),
    ("a11y", "accessibility-menu", "right"),
    ("screenshare", "screen-sharing-indicator", "right"),
    ("quick", "quick-settings", "right"),
    ("power", "power-icon", "right"),
)
QUICK_ITEMS = (("qs.dark", "quick-settings-dark-mode"), ("qs.night", "quick-settings-night-light"),
               ("qs.dnd", "quick-settings-do-not-disturb"), ("qs.airplane", "quick-settings-airplane-mode"),
               ("qs.backlight", "quick-settings-backlight"))
POWER_ITEMS = (("pw.logout", "org.gnome.shell", "always-show-log-out", False),
               ("pw.switch", "org.gnome.desktop.lockdown", "disable-user-switching", True),
               ("pw.lock", "org.gnome.desktop.lockdown", "disable-lock-screen", True))


def _jp_cmd() -> list:
    d = ext_dir(JP_UUID)
    return ["gsettings", "--schemadir", str(d / "schemas")] if d else ["gsettings"]


def jp_get(key: str, default=None):
    if usercmd.DRY:
        v = usercmd._dry_values().get(f"{JP_SCHEMA} {key}")
        if v is not None:
            return v
    out = usercmd.query(_jp_cmd() + ["get", JP_SCHEMA, key])
    return default if out is None else usercmd.parse_value(out)


def jp_set(key: str, value) -> bool:
    if usercmd.DRY:
        vals = usercmd._dry_values()
        vals[f"{JP_SCHEMA} {key}"] = value
        usercmd.DRY_GSET.parent.mkdir(parents=True, exist_ok=True)
        usercmd.DRY_GSET.write_text(__import__("json").dumps(vals, ensure_ascii=False), encoding="utf-8")
    return usercmd.run(_jp_cmd() + ["set", JP_SCHEMA, key, usercmd.gvariant(value)])[0]


def jp_ready() -> bool:
    return usercmd.DRY or (ext_installed(JP_UUID) and JP_UUID in ext_enabled())
