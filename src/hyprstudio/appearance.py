"""Hell/Dunkel für alle Programme. Ohne Qt.

Unter Hyprland (ohne Plasma-Sitzung) reicht das KDE-Farbschema allein nicht: Qt-/KDE-Apps wie Kate und KWrite
lesen kdeglobals nur mit dem KDE-Plattform-Theme (Paket plasma-integration, QT_QPA_PLATFORMTHEME=kde). Darum:
- GTK 3/4: settings.ini (prefer-dark + Adwaita-dark), gsettings color-scheme/gtk-theme (Portal → libadwaita, Electron)
- KDE/Qt: Farbschema BreezeDark/BreezeLight in kdeglobals (aus /usr/share/color-schemes, sonst eingebaut),
  plasma-apply-colorscheme falls vorhanden, Farbschema zusätzlich in den rc-Dateien der KDE-Apps ([UiSettings])
- qt5ct/qt6ct (falls statt plasma-integration genutzt): dunkle/helle Palette
- Hyprland-Umgebung QT_QPA_PLATFORMTHEME (schreibt die Hyprland-Seite, siehe qt_platform_theme)
"""

import glob
import shutil
from pathlib import Path

from . import model
from .model import MARK

MODES = ("dark", "light")
DEFAULT = "dark"
SCHEME = {"dark": ("BreezeDark", "Breeze Dark"), "light": ("BreezeLight", "Breeze Light")}

# KDE-Apps mit eigenem Farbschema-Menü (Einstellungen › Farbschema) → <app>rc [UiSettings] ColorScheme
KDE_APPS = ("kate", "kwrite", "dolphin", "okular", "gwenview", "ark", "spectacle", "kcalc", "filelight", "elisa",
            "kdenlive", "krita", "partitionmanager", "kfind", "kcharselect", "skanlite", "kolourpaint", "k3b",
            "konversation", "kmail2", "kontact", "korganizer", "akregator", "ktorrent", "kdevelop", "haruna",
            "kwalletmanager5", "kinfocenter", "systemsettings", "discover", "plasma-discover")

KDE_PLUGIN_GLOBS = ("/usr/lib/qt6/plugins/platformthemes/KDEPlasmaPlatformTheme6.so",
                    "/usr/lib/*/qt6/plugins/platformthemes/KDEPlasmaPlatformTheme6.so",
                    "/usr/lib64/qt6/plugins/platformthemes/KDEPlasmaPlatformTheme6.so")

# Eingebaute Notfall-Farben (Breeze), falls /usr/share/color-schemes fehlt
_FALLBACK = {
    "dark": {
        "Colors:Window": ("32,35,38", "252,252,252"), "Colors:View": ("20,22,24", "252,252,252"),
        "Colors:Button": ("41,44,48", "252,252,252"), "Colors:Selection": ("61,174,233", "252,252,252"),
        "Colors:Tooltip": ("41,44,48", "252,252,252"), "Colors:Complementary": ("20,22,24", "252,252,252"),
        "Colors:Header": ("32,35,38", "252,252,252"),
    },
    "light": {
        "Colors:Window": ("239,240,241", "35,38,41"), "Colors:View": ("255,255,255", "35,38,41"),
        "Colors:Button": ("252,252,252", "35,38,41"), "Colors:Selection": ("61,174,233", "255,255,255"),
        "Colors:Tooltip": ("247,247,247", "35,38,41"), "Colors:Complementary": ("42,46,50", "252,252,252"),
        "Colors:Header": ("222,224,226", "35,38,41"),
    },
}


# ---------- INI-Hilfen ----------

def read_ini(text: str) -> list:
    """[(Abschnitt, [(Schlüssel, Wert)])] – Reihenfolge bleibt erhalten; Zeilen vor dem 1. Abschnitt unter ""."""
    out = [("", [])]
    for ln in text.splitlines():
        s = ln.strip()
        if s.startswith("[") and s.endswith("]"):
            out.append((s[1:-1], []))
        elif "=" in s and not s.startswith(("#", ";")):
            k, v = s.split("=", 1)
            out[-1][1].append((k.strip(), v.strip()))
        elif s:
            out[-1][1].append((s, None))
    return out


def write_ini(sections: list) -> str:
    lines = []
    for name, items in sections:
        if name:
            lines += ["", f"[{name}]"] if lines else [f"[{name}]"]
        for k, v in items:
            lines.append(k if v is None else f"{k}={v}")
    return "\n".join(lines).lstrip("\n") + "\n"


def merge_ini(text: str, updates: dict) -> str:
    """updates = {Abschnitt: {Schlüssel: Wert}} in vorhandenen Text einsetzen, Rest behalten."""
    secs = read_ini(text)
    for sec, kv in updates.items():
        hit = next((items for name, items in secs if name == sec), None)
        if hit is None:
            hit = []
            secs.append((sec, hit))
        for k, v in kv.items():
            i = next((j for j, (kk, _) in enumerate(hit) if kk == k), None)
            if i is None:
                hit.append((k, v))
            else:
                hit[i] = (k, v)
    return write_ini(secs)


def _update(f: Path, updates: dict, log, backup: bool = True) -> None:
    f.parent.mkdir(parents=True, exist_ok=True)
    if backup and f.is_file() and not f.with_name(f.name + ".hyprstudio-original").exists():
        bak = model._backup_foreign(f)  # nur einmal: das Original vor der ersten Änderung
        if bak:
            log(f"Sicherung: {bak}")
    old = f.read_text(encoding="utf-8", errors="replace") if f.is_file() else ""
    new = merge_ini(old, updates)
    if new != old:
        with open(f, "w", encoding="utf-8", newline="\n") as fh:
            fh.write(new)
        log(f"→ {f}")


def set_ini(f: Path, key: str, value: str, log) -> None:
    """Schlüssel in [Settings] setzen (GTK), Rest der Datei behalten; fremde Datei vorher sichern."""
    old = f.read_text(encoding="utf-8", errors="replace") if f.is_file() else ""
    if MARK not in old[:300]:
        f.parent.mkdir(parents=True, exist_ok=True)
        bak = model._backup_foreign(f)
        if bak:
            log(f"Sicherung: {bak}")
        old = f"# {MARK}\n" + old
        f.write_text(old, encoding="utf-8")
    _update(f, {"Settings": {key: value}}, log, backup=False)


# ---------- KDE-Farbschema ----------

def scheme_file(mode: str) -> Path | None:
    name = SCHEME[mode][0]
    for d in (Path.home() / ".local/share/color-schemes", Path("/usr/share/color-schemes")):
        f = d / f"{name}.colors"
        if f.is_file():
            return f
    return None


def scheme_colors(mode: str) -> dict:
    """Alle Farb-Abschnitte des Schemas (Colors:*, WM) als {Abschnitt: {Schlüssel: Wert}}."""
    f = scheme_file(mode)
    if f:
        secs = read_ini(f.read_text(encoding="utf-8", errors="replace"))
        out = {n: {k: v for k, v in items if v is not None} for n, items in secs
               if n.startswith("Colors:") or n == "WM"}
        if out:
            return out
    out = {}
    for sec, (bg, fg) in _FALLBACK[mode].items():
        out[sec] = {"BackgroundNormal": bg, "BackgroundAlternate": bg, "ForegroundNormal": fg,
                    "ForegroundInactive": fg, "DecorationFocus": "61,174,233", "DecorationHover": "61,174,233"}
    return out


def kde_plugin() -> bool:
    if model.DRY:
        return True
    return any(glob.glob(g) for g in KDE_PLUGIN_GLOBS)


def qt_platform_theme() -> str:
    """Welches Qt-Plattform-Theme in Hyprland gesetzt werden soll: kde (plasma-integration), qt6ct oder keins."""
    if kde_plugin():
        return "kde"
    if shutil.which("qt6ct"):
        return "qt6ct"
    return ""


def apply_kde(mode: str, cfg_home: Path, log) -> None:
    file_name, display = SCHEME[mode]
    updates = scheme_colors(mode)
    updates.setdefault("General", {})["ColorScheme"] = file_name
    _update(cfg_home / "kdeglobals", updates, log)
    if model.DRY or shutil.which("plasma-apply-colorscheme"):
        model.run(["plasma-apply-colorscheme", file_name], log, quiet=True)
    # jede KDE-App einzeln (gilt auch ohne Plattform-Theme): nur vorhandene rc-Dateien + Kate/KWrite immer
    n = 0
    for app in KDE_APPS:
        f = cfg_home / f"{app}rc"
        if f.is_file() or app in ("kate", "kwrite"):
            _update(f, {"UiSettings": {"ColorScheme": display}}, log, backup=False)
            n += 1
    log(f"KDE-Apps: {n} × ColorScheme={display}")


def apply_qtct(mode: str, cfg_home: Path, log) -> None:
    """qt5ct/qt6ct: eigene Palette (darker = dunkel), nur wenn das Programm da ist."""
    for tool in ("qt6ct", "qt5ct"):
        if not (model.DRY or shutil.which(tool)):
            continue
        colors = Path(f"/usr/share/{tool}/colors/darker.conf")
        upd = {"Appearance": {"custom_palette": "true" if mode == "dark" else "false",
                              "color_scheme_path": str(colors).replace("\\", "/")}}
        if mode == "dark" and scheme_file("dark") and tool == "qt6ct":
            upd["Appearance"]["color_scheme_path"] = str(scheme_file("dark")).replace("\\", "/")
        _update(cfg_home / tool / f"{tool}.conf", upd, log)


# ---------- Portal, Umgebung, Flatpak (damit wirklich jede App dunkel wird) ----------
# GTK4/libadwaita, Chromium, Electron und Firefox lesen Hell/Dunkel über das Desktop-Portal (Settings-Schnittstelle).
# xdg-desktop-portal-hyprland bietet die nicht an → xdg-desktop-portal-gtk muss mitlaufen, und seit
# xdg-desktop-portal 1.18 braucht es dafür eine Datei <XDG_CURRENT_DESKTOP>-portals.conf.
PORTALS_CONF = f"""# {MARK} – Hell/Dunkel für alle Apps (Sonstiges › Hell / Dunkel)
[preferred]
default=hyprland;gtk
org.freedesktop.impl.portal.Settings=gtk
"""
DBUS_ENV = "dbus-update-activation-environment --systemd --all"


def write_portals_conf(cfg_home: Path, log) -> bool:
    """hyprland-portals.conf anlegen; eine eigene, fremde Datei bleibt unangetastet."""
    f = cfg_home / "xdg-desktop-portal" / "hyprland-portals.conf"
    if f.is_file() and MARK not in f.read_text(encoding="utf-8", errors="replace"):
        log(f"Eigene Portal-Einstellung bleibt: {f}")
        return False
    if not f.is_file() or f.read_text(encoding="utf-8") != PORTALS_CONF:
        f.parent.mkdir(parents=True, exist_ok=True)
        f.write_text(PORTALS_CONF, encoding="utf-8")
        log(f"→ {f}")
    return True


def gtk_portal() -> bool:
    if model.DRY:
        return True
    return any(Path(p).exists() for p in ("/usr/lib/xdg-desktop-portal-gtk", "/usr/libexec/xdg-desktop-portal-gtk",
                                          "/usr/share/xdg-desktop-portal/portals/gtk.portal"))


def env_content(mode: str, qt: str, fmt: str) -> str:
    """hyprstudio_look.lua/.conf: Qt-Thema und D-Bus-Umgebung. Kein GTK_THEME: das schaltet libadwaita-Stile ab
    (kaputte GNOME-Apps); GTK/libadwaita folgen color-scheme übers Portal."""
    c = "--" if fmt == "lua" else "#"
    out = [f"{c} {MARK} – Hell/Dunkel, automatisch erzeugt (Sonstiges › Hell / Dunkel)"]
    if fmt == "lua":
        if qt:
            out.append(f'hl.env("QT_QPA_PLATFORMTHEME", "{qt}")')
        out += ['hl.on("hyprland.start", function ()', f'    hl.exec_cmd("{DBUS_ENV}")', "end)"]
    else:
        if qt:
            out.append(f"env = QT_QPA_PLATFORMTHEME,{qt}")
        out.append(f"exec-once = {DBUS_ENV}")
    return "\n".join(out) + "\n"


def apply_env(mode: str, cfg_home: Path, log) -> bool:
    from . import laptopkeys  # Helfer zum Einbinden kleiner Zusatzdateien
    qt = qt_platform_theme()
    return laptopkeys.include_extra(cfg_home / "hypr", "look", lambda fmt: env_content(mode, qt, fmt),
                                    "Hell/Dunkel", log) is not None


def apply_services(mode: str, cfg_home: Path, log) -> None:
    """Früher: GTK_THEME/QT_QPA_PLATFORMTHEME in ~/.config/environment.d – das galt für jede Sitzung (auch GNOME)
    und zerlegte dort libadwaita-Apps (GNOME-Einstellungen ohne Abstände). Jetzt nur noch aufräumen: unsere
    Datei entfernen und die Variablen aus der laufenden Sitzung nehmen. Hyprland bekommt sie über seine Config."""
    f = cfg_home / "environment.d" / "60-hyprstudio-look.conf"
    if f.is_file() and MARK in f.read_text(encoding="utf-8", errors="replace"):
        f.unlink()
        log(f"− {f}")
        if model.DRY or shutil.which("systemctl"):
            model.run(["systemctl", "--user", "unset-environment", "GTK_THEME", "QT_QPA_PLATFORMTHEME"], log,
                      quiet=True)


def apply_flatpak(mode: str, log) -> None:
    """Kein GTK_THEME für Flatpak-Apps mehr (bricht libadwaita); eine frühere Überschreibung wieder entfernen.
    Flatpak-Apps folgen dem Farbschema übers Portal."""
    if not (model.DRY or shutil.which("flatpak")):
        return
    model.run(["flatpak", "override", "--user", "--unset-env=GTK_THEME"], log, quiet=True)


def apply(mode: str, cfg_home: Path, log) -> bool:
    dark = mode == "dark"
    for d in ("gtk-3.0", "gtk-4.0"):
        set_ini(cfg_home / d / "settings.ini", "gtk-application-prefer-dark-theme", "1" if dark else "0", log)
    set_ini(cfg_home / "gtk-3.0" / "settings.ini", "gtk-theme-name", "Adwaita-dark" if dark else "Adwaita", log)
    if model.DRY or shutil.which("gsettings"):
        model.run(["gsettings", "set", "org.gnome.desktop.interface", "color-scheme",
                   "prefer-dark" if dark else "prefer-light"], log, quiet=True)
        model.run(["gsettings", "set", "org.gnome.desktop.interface", "gtk-theme",
                   "Adwaita-dark" if dark else "Adwaita"], log, quiet=True)
    apply_kde(mode, cfg_home, log)
    apply_qtct(mode, cfg_home, log)
    write_portals_conf(cfg_home, log)
    apply_env(mode, cfg_home, log)
    apply_flatpak(mode, log)
    apply_services(mode, cfg_home, log)
    if model.DRY or shutil.which("systemctl"):  # laufende Apps bekommen den Wechsel sofort übers Portal mit
        model.run(["systemctl", "--user", "try-restart", "xdg-desktop-portal.service",
                   "xdg-desktop-portal-gtk.service"], log, quiet=True)
    if model.DRY or shutil.which("hyprctl"):
        model.run(["hyprctl", "reload"], log, quiet=True)
    return True
