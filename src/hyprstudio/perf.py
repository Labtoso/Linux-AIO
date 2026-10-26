"""Sonstiges › Leistung: prüfen, warum Programme unter Hyprland langsam starten, ewig laden oder abstürzen – und
beheben. Ohne Qt.

Häufigste Ursachen unter Hyprland:
- Desktop-Portal fehlt oder bekommt die Wayland-Umgebung nicht → GTK/Electron-Apps warten bis zu 25 s beim Start,
  Datei-Dialoge hängen (Fix: xdg-desktop-portal-hyprland + -gtk, dbus-update-activation-environment beim Start)
- kein Swap/zram → bei vollem Speicher beendet der Kernel Programme („Absturz“) (Fix: zram-generator)
- Energieprofil „Stromsparen“ → alles träge (Fix: powerprofilesctl)
- viel Unschärfe/Schatten → ruckelt, besonders auf Laptops (Fix: Effekte sparen)
- volle Festplatte → Programme können nichts speichern und brechen ab (nur Hinweis)
"""

import glob
import json
import os
import shutil
import subprocess
import time
from pathlib import Path

from . import laptopkeys, model
from .model import MARK

PORTAL_FILES = {
    "hyprland": ("/usr/lib/xdg-desktop-portal-hyprland", "/usr/libexec/xdg-desktop-portal-hyprland",
                 "/usr/share/xdg-desktop-portal/portals/hyprland.portal"),
    "gtk": ("/usr/lib/xdg-desktop-portal-gtk", "/usr/libexec/xdg-desktop-portal-gtk",
            "/usr/share/xdg-desktop-portal/portals/gtk.portal"),
}
PORTAL_PKGS = {"pacman": ["xdg-desktop-portal-hyprland", "xdg-desktop-portal-gtk"],
               "apt": ["xdg-desktop-portal-hyprland", "xdg-desktop-portal-gtk"],
               "dnf": ["xdg-desktop-portal-hyprland", "xdg-desktop-portal-gtk"],
               "zypper": ["xdg-desktop-portal-hyprland", "xdg-desktop-portal-gtk"]}
ZRAM_PKG = {"pacman": "zram-generator", "apt": "systemd-zram-generator", "dnf": "zram-generator",
            "zypper": "zram-generator"}
ZRAM_CONF = "/etc/systemd/zram-generator.conf"
ZRAM_TEXT = f"# {MARK}\n[zram0]\nzram-size = ram / 2\ncompression-algorithm = zstd\n"
DBUS_ENV = "dbus-update-activation-environment --systemd --all"
PERF_DEFAULT = {"portal_env": False, "light": False}


def perf_state(saved: dict | None) -> dict:
    s = dict(PERF_DEFAULT)
    for k, v in (saved or {}).items():
        if k in s and type(v) is type(s[k]):
            s[k] = v
    return s


# ---------- Prüfen ----------

def _meminfo() -> dict:
    out = {}
    try:
        for ln in Path("/proc/meminfo").read_text().splitlines():
            k, v = ln.split(":", 1)
            out[k] = int(v.split()[0]) * 1024
    except (OSError, ValueError):
        pass
    return out


def _swaps() -> list:
    try:
        return [ln.split()[0] for ln in Path("/proc/swaps").read_text().splitlines()[1:] if ln.strip()]
    except OSError:
        return []


def _run(cmd: list, timeout: int = 15) -> str:
    try:
        return subprocess.run(cmd, capture_output=True, text=True, timeout=timeout).stdout
    except (OSError, subprocess.TimeoutExpired):
        return ""


def _dbus_env_started(hdir: Path, hypr_state: dict) -> bool:
    """Bekommt der D-Bus die Wayland-Umgebung (nötig für Portale)? uwsm macht das selbst."""
    if any("dbus-update-activation-environment" in c for c in hypr_state.get("autostart", [])):
        return True
    for f in hdir.glob("*.lua"):
        if "dbus-update-activation-environment" in f.read_text(encoding="utf-8", errors="replace"):
            return True
    for f in hdir.glob("*.conf"):
        if "dbus-update-activation-environment" in f.read_text(encoding="utf-8", errors="replace"):
            return True
    return "uwsm" in _run(["ps", "-eo", "comm"], 5)


def crashes(days: int = 7) -> list:
    """[(Programm, Anzahl)] aus systemd-coredump der letzten Tage, häufigste zuerst."""
    if model.DRY:
        return [("firefox", 3), ("electron", 2), ("waybar", 1)]
    if not shutil.which("coredumpctl"):
        return []
    out = _run(["coredumpctl", "list", f"--since=-{days}d", "--no-pager", "--json=short"], 20)
    counts = {}
    try:
        for e in json.loads(out or "[]"):
            name = Path(e.get("exe") or e.get("comm") or "?").name
            counts[name] = counts.get(name, 0) + 1
    except ValueError:
        for ln in out.splitlines()[1:]:  # ältere Versionen ohne JSON: letzte Spalte = Programm
            parts = ln.split()
            if parts:
                name = Path(parts[-1]).name
                counts[name] = counts.get(name, 0) + 1
    return sorted(counts.items(), key=lambda x: -x[1])


def hypr_crash_reports(days: int = 7) -> int:
    d = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "hyprland"
    limit = time.time() - days * 86400
    return len([f for f in d.glob("hyprlandCrashReport*.txt") if f.stat().st_mtime > limit]) if d.is_dir() else 0


def check(hdir: Path, hypr_state: dict, perf: dict) -> dict:
    """Alle Prüfungen – läuft im Hintergrund (coredumpctl kann dauern). Je Punkt: ok, Wert, Details."""
    if model.DRY:
        return {"portal": {"ok": False, "missing": ["gtk"], "env": False},
                "swap": {"ok": False, "ram": 8 * 2 ** 30, "avail": 2 * 2 ** 30, "swap": 0},
                "power": {"ok": False, "profile": "power-saver", "ac": True},
                "effects": {"ok": perf["light"] or hypr_state.get("blur_passes", 1) <= 2, "passes": hypr_state.get("blur_passes", 1)},
                "disk": {"ok": True, "free": 120 * 2 ** 30, "total": 500 * 2 ** 30},
                "crashes": crashes(), "hypr_crashes": 0}
    missing = [k for k, files in PORTAL_FILES.items() if not any(Path(f).exists() for f in files)]
    env = perf["portal_env"] or _dbus_env_started(hdir, hypr_state)
    mem, swaps = _meminfo(), _swaps()
    prof = _run(["powerprofilesctl", "get"], 5).strip() if shutil.which("powerprofilesctl") else ""
    ac = any(Path(p).read_text().strip() == "1" for p in glob.glob("/sys/class/power_supply/A*/online")) \
        if glob.glob("/sys/class/power_supply/A*/online") else True
    du = shutil.disk_usage("/")
    return {
        "portal": {"ok": not missing and env, "missing": missing, "env": env},
        "swap": {"ok": bool(swaps), "ram": mem.get("MemTotal", 0), "avail": mem.get("MemAvailable", 0),
                 "swap": mem.get("SwapTotal", 0)},
        "power": {"ok": prof in ("", "performance") or (prof == "balanced" and not ac), "profile": prof, "ac": ac},
        "effects": {"ok": perf["light"] or hypr_state.get("blur_passes", 1) <= 2 and not hypr_state.get("blur_xray"),
                    "passes": hypr_state.get("blur_passes", 1)},
        "disk": {"ok": du.free / du.total > 0.1, "free": du.free, "total": du.total},
        "crashes": crashes(), "hypr_crashes": hypr_crash_reports(),
    }


# ---------- Beheben ----------

def portal_command(pm: str | None) -> list:
    pkgs = PORTAL_PKGS.get(pm or "", [])
    if not pkgs:
        return []
    return {"pacman": ["pacman", "-S", "--needed", "--noconfirm", *pkgs], "apt": ["apt-get", "install", "-y", *pkgs],
            "dnf": ["dnf", "install", "-y", *pkgs], "zypper": ["zypper", "-n", "in", *pkgs]}[pm]


def zram_command(pm: str | None) -> list:
    """zram einrichten: Paket, Einstellung (eine vorhandene bleibt), sofort starten – als ein root-Befehl."""
    pkg = ZRAM_PKG.get(pm or "")
    if not pkg:
        return []
    inst = {"pacman": f"pacman -S --needed --noconfirm {pkg}", "apt": f"apt-get install -y {pkg}",
            "dnf": f"dnf install -y {pkg}", "zypper": f"zypper -n in {pkg}"}[pm]
    text = ZRAM_TEXT.replace("\n", "\\n")
    return ["sh", "-c", f"{inst} && ( [ -f {ZRAM_CONF} ] || printf '{text}' > {ZRAM_CONF} ) "
                        f"&& systemctl daemon-reload && systemctl start systemd-zram-setup@zram0.service"]


def power_command(profile: str = "performance") -> list:
    return ["powerprofilesctl", "set", profile] if shutil.which("powerprofilesctl") or model.DRY else []


def content(perf: dict, fmt: str) -> str:
    """hyprstudio_perf.lua/.conf: D-Bus-Umgebung beim Start, leichtere Effekte."""
    c = "--" if fmt == "lua" else "#"
    out = [f"{c} {MARK} – Leistung, automatisch erzeugt (Sonstiges › Leistung)"]
    if perf["portal_env"]:
        if fmt == "lua":
            out += ['hl.on("hyprland.start", function ()', f'    hl.exec_cmd("{DBUS_ENV}")', "end)"]
        else:
            out.append(f"exec-once = {DBUS_ENV}")
    if perf["light"]:
        if fmt == "lua":
            out += ["hl.config({", "    decoration = {",
                    "        blur = { passes = 1, size = 3, new_optimizations = true, xray = false },",
                    "        shadow = { enabled = false },", "    },", "    misc = { vfr = true },", "})"]
        else:
            out += ["decoration {", "    blur {", "        passes = 1", "        size = 3",
                    "        new_optimizations = true", "        xray = false", "    }",
                    "    shadow {", "        enabled = false", "    }", "}", "misc {", "    vfr = true", "}"]
    if len(out) == 1:
        out.append(f"{c} nichts eingeschaltet")
    return "\n".join(out) + "\n"


def write(hdir: Path, perf: dict, log) -> bool:
    return laptopkeys.include_extra(hdir, "perf", lambda fmt: content(perf, fmt), "Leistung", log) is not None


def size(n: int) -> str:
    return f"{n / 2 ** 30:.1f} GB"
