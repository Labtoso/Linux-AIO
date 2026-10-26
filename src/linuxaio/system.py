"""Systemabfragen ohne Root: Paketmanager, installierte Sessions, laufender Desktop, Standard-Session."""

import configparser
import os
import shutil
import subprocess
from pathlib import Path

from . import catalog

SESSION_DIRS = (Path("/usr/share/wayland-sessions"), Path("/usr/share/xsessions"))
SDDM_STATE = Path("/var/lib/sddm/state.conf")
ACCOUNTS_DIR = Path("/var/lib/AccountsService/users")
STATE_FILE = Path("/var/lib/labtoso-linuxaio/state.json")


def detect_backend() -> str | None:
    forced = os.environ.get("DEMANAGER_BACKEND")
    if forced in catalog.BACKENDS:
        return forced
    for name, binary in (("pacman", "pacman"), ("apt", "apt-get"), ("dnf", "dnf"), ("zypper", "zypper")):
        if shutil.which(binary):
            return name
    return None


def distro_name() -> str:
    try:
        for line in Path("/etc/os-release").read_text(encoding="utf-8").splitlines():
            if line.startswith("PRETTY_NAME="):
                return line.split("=", 1)[1].strip().strip('"')
    except OSError:
        pass
    return "Linux"


def session_file(name: str) -> Path | None:
    for d in SESSION_DIRS:
        p = d / f"{name}.desktop"
        if p.is_file():
            return p
    return None


def installed_session(de: catalog.Desktop) -> Path | None:
    for s in de.sessions:
        p = session_file(s)
        if p:
            return p
    return None


def is_installed(de: catalog.Desktop) -> bool:
    return installed_session(de) is not None


def running_desktop() -> str | None:
    cur = os.environ.get("XDG_CURRENT_DESKTOP", "") + ":" + os.environ.get("XDG_SESSION_DESKTOP", "")
    parts = {p.strip().lower() for p in cur.split(":") if p.strip()}
    for de in catalog.DESKTOPS:
        if any(x.lower() in parts for x in de.detect + de.sessions):
            return de.id
    return None


def default_session() -> str | None:
    """Session-Name (ohne .desktop), die der Login-Manager als nächstes startet."""
    if os.environ.get("DEMANAGER_DRYRUN") == "1":
        try:
            return (_dry_dm_file().parent / "default-session").read_text(encoding="utf-8").strip() or None
        except OSError:
            return None
    user = os.environ.get("USER", "")
    acc = ACCOUNTS_DIR / user
    for path, section, keys in ((acc, "User", ("Session", "XSession")), (SDDM_STATE, "Last", ("Session",))):
        try:
            cp = configparser.ConfigParser(interpolation=None)
            cp.read(path, encoding="utf-8")
            for k in keys:
                v = cp.get(section, k, fallback="").strip()
                if v:
                    return Path(v).name.removesuffix(".desktop")
        except (OSError, configparser.Error):
            continue
    return None


def default_desktop() -> str | None:
    s = default_session()
    if not s:
        return None
    for de in catalog.DESKTOPS:
        if s in de.sessions:
            return de.id
    return None


def has_pkexec() -> bool:
    return shutil.which("pkexec") is not None


# ---------- Paketmanager-Befehle (vom Helfer genutzt) ----------

def _quiet(cmd: list) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)


def available(backend: str, name: str) -> bool:
    if backend == "pacman":
        return _quiet(["pacman", "-Si", name]).returncode == 0 or bool(_quiet(["pacman", "-Sg", name]).stdout.strip())
    if backend == "apt":
        r = _quiet(["apt-cache", "show", "--no-all-versions", name])
        return r.returncode == 0 and "Package:" in r.stdout
    if backend == "dnf":
        if name.startswith("@"):
            return _quiet(["dnf", "-q", "group", "info", name[1:]]).returncode == 0
        return bool(_quiet(["dnf", "-q", "repoquery", "--latest-limit=1", name]).stdout.strip())
    if backend == "zypper":
        kind, n = ("pattern", name[8:]) if name.startswith("pattern:") else ("package", name)
        return _quiet(["zypper", "-q", "-n", "se", "-x", "-t", kind, n]).returncode == 0
    return False


def resolve(backend: str, specs: list, check=available) -> tuple[list, list]:
    """Löst Alternativen/optionale Pakete auf. Gibt (pakete, fehlende_pflichtpakete) zurück."""
    out, missing = [], []
    for spec in specs:
        optional = spec.startswith("?")
        choices = spec.lstrip("?").split("|")
        hit = next((c for c in choices if check(backend, c)), None)
        if hit:
            if hit not in out:
                out.append(hit)
        elif not optional:
            missing.append(spec)
    return out, missing


def installed_packages(backend: str) -> set:
    if backend == "pacman":
        cmd = ["pacman", "-Qq"]
    elif backend == "apt":
        cmd = ["dpkg-query", "-W", "-f=${db:Status-Abbrev} ${Package}\n"]
    else:
        cmd = ["rpm", "-qa", "--qf", "%{NAME}\n"]
    r = _quiet(cmd)
    pk = set()
    for line in r.stdout.splitlines():
        if backend == "apt":
            st, _, name = line.partition(" ")
            if st.startswith("ii"):
                pk.add(name.split(":")[0])
        elif line.strip():
            pk.add(line.strip())
    return pk


def install_commands(backend: str, pkgs: list) -> list:
    if backend == "pacman":
        return [["pacman", "-Syu", "--needed", "--noconfirm", *pkgs]]
    if backend == "apt":
        return [["apt-get", "update"], ["apt-get", "install", "-y", *pkgs]]
    if backend == "dnf":
        return [["dnf", "install", "-y", *pkgs]]
    if backend == "zypper":
        pats = [p[8:] for p in pkgs if p.startswith("pattern:")]
        plain = [p for p in pkgs if not p.startswith("pattern:")]
        cmds = [["zypper", "-n", "refresh"]]
        if pats:
            cmds.append(["zypper", "-n", "in", "-t", "pattern", *pats])
        if plain:
            cmds.append(["zypper", "-n", "in", *plain])
        return cmds
    raise ValueError(backend)


def remove_command(backend: str, pkgs: list) -> list:
    if backend == "pacman":
        return ["pacman", "-Rnsu", "--noconfirm", *pkgs]
    if backend == "apt":
        return ["apt-get", "remove", "-y", "--autoremove", *pkgs]
    if backend == "dnf":
        return ["dnf", "remove", "-y", *pkgs]
    if backend == "zypper":
        return ["zypper", "-n", "rm", "-u", *pkgs]
    raise ValueError(backend)


def simulate_remove(backend: str, pkgs: list) -> set | None:
    """Pakete, die ein Entfernen tatsächlich mitnehmen würde (None = nicht prüfbar)."""
    if backend == "pacman":
        r = _quiet(["pacman", "-Rnsu", "--print", "--print-format", "%n", *pkgs])
        return {l.strip() for l in r.stdout.splitlines() if l.strip()} if r.returncode == 0 else None
    if backend == "apt":
        r = _quiet(["apt-get", "-s", "remove", "--autoremove", *pkgs])
        if r.returncode != 0:
            return None
        return {l.split()[1].split(":")[0] for l in r.stdout.splitlines() if l.startswith("Remv ")}
    return None


# ---------- Login-Manager (Display-Manager) ----------

DM_NAMES = {"sddm": "SDDM", "plasmalogin": "Plasma Login Manager", "gdm": "GDM", "lightdm": "LightDM",
            "cosmic-greeter": "COSMIC Greeter", "ly": "Ly", "greetd": "greetd", "lxdm": "LXDM"}
DM_SWITCHABLE = ("sddm", "plasmalogin", "gdm", "lightdm")
UNIT_DIRS = (Path("/usr/lib/systemd/system"), Path("/etc/systemd/system"), Path("/lib/systemd/system"))


def _dry_dm_file() -> Path:
    import tempfile
    return Path(tempfile.gettempdir()) / "desddm-dry" / "display-manager"


def active_display_manager() -> str:
    if os.environ.get("DEMANAGER_DRYRUN") == "1":
        try:
            return _dry_dm_file().read_text(encoding="utf-8").strip()
        except OSError:
            return "plasmalogin"
    try:
        return Path(os.readlink("/etc/systemd/system/display-manager.service")).name.removesuffix(".service")
    except OSError:
        return ""


def dm_unit_exists(name: str) -> bool:
    if os.environ.get("DEMANAGER_DRYRUN") == "1":
        return True
    return any((d / f"{name}.service").is_file() for d in UNIT_DIRS)


def sddm_installed() -> bool:
    if os.environ.get("DEMANAGER_DRYRUN") == "1":
        return (_dry_dm_file().parent / "sddm-installed").exists()
    return shutil.which("sddm") is not None or dm_unit_exists("sddm")


def dm_pretty(name: str) -> str:
    return DM_NAMES.get(name, name or "–")
