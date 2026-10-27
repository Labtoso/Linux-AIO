"""Ein-Klick-Installation für Fehlendes (Download-Pfeil in der App). Ohne Qt.

GNOME-Erweiterungen: passende Version von extensions.gnome.org (Info-Endpunkt mit der Shell-Version),
Zip prüfen (metadata.json mit richtiger UUID, keine Pfade außerhalb), dann `gnome-extensions install --force`
und `gnome-extensions enable`. Klappt das nicht: Paket über den Root-Helfer (`pkg-install <schlüssel>`).
"""

import io
import json
import re
import tempfile
import urllib.parse
import urllib.request
import zipfile
from pathlib import Path

from . import remote, usercmd

EGO = "https://extensions.gnome.org"

# Schlüssel → Pakete je Paketmanager (der Root-Helfer nimmt nur diese Schlüssel an)
PACKAGES = {
    "yay-deps": {"pacman": ["git", "base-devel"]},
    "alacritty": {"pacman": ["alacritty"], "apt": ["alacritty"], "dnf": ["alacritty"], "zypper": ["alacritty"]},
    "kitty": {"pacman": ["kitty"], "apt": ["kitty"], "dnf": ["kitty"], "zypper": ["kitty"]},
    "gnome-shell-extensions": {"pacman": ["gnome-shell-extensions"], "apt": ["gnome-shell-extensions"],
                               "dnf": ["gnome-shell-extension-user-theme"], "zypper": ["gnome-shell-extensions-common"]},
    "gnome-tweaks": {"pacman": ["gnome-tweaks"], "apt": ["gnome-tweaks"], "dnf": ["gnome-tweaks"],
                     "zypper": ["gnome-tweaks"]},
    "squashfs-tools": {"pacman": ["squashfs-tools"], "apt": ["squashfs-tools"], "dnf": ["squashfs-tools"],
                       "zypper": ["squashfs"]},
}
# Erweiterung → Paket, das sie mitbringt (Plan B)
EXT_PACKAGE = {
    "user-theme@gnome-shell-extensions.gcampax.github.com": "gnome-shell-extensions",
    "workspace-indicator@gnome-shell-extensions.gcampax.github.com": "gnome-shell-extensions",
}
EXT_NAMES = {
    "just-perfection-desktop@just-perfection": "Just Perfection",
    "rounded-window-corners@fxgn": "Rounded Window Corners Reborn",
    "user-theme@gnome-shell-extensions.gcampax.github.com": "User Themes",
    "workspace-indicator@gnome-shell-extensions.gcampax.github.com": "Workspace Indicator",
}

DRY_EXT = set()   # Testmodus: „installierte“ Erweiterungen merken


class InstallError(Exception):
    def __init__(self, key: str, detail: str = ""):
        super().__init__(detail or key)
        self.key, self.detail = key, detail


def shell_version() -> str:
    out = usercmd.query(["gnome-shell", "--version"]) or ""
    m = re.search(r"(\d+)(?:\.(\d+))?", out)
    return m.group(1) if m else ""


def check_zip(data: bytes, uuid: str) -> None:
    try:
        z = zipfile.ZipFile(io.BytesIO(data))
        names = z.namelist()
        bad = z.testzip()
    except (zipfile.BadZipFile, OSError) as e:
        raise InstallError("miss.badzip", str(e)) from e
    if bad:
        raise InstallError("miss.badzip", bad)
    for n in names:
        p = Path(n)
        if n.startswith(("/", "\\")) or p.is_absolute() or ".." in p.parts:
            raise InstallError("miss.badzip", n)
    try:
        meta = json.loads(z.read("metadata.json"))
    except (KeyError, ValueError) as e:
        raise InstallError("miss.badzip", "metadata.json") from e
    if meta.get("uuid") != uuid:
        raise InstallError("miss.badzip", f"uuid {meta.get('uuid')}")


def ego_download(uuid: str, version: str) -> bytes:
    q = urllib.parse.urlencode({"uuid": uuid, "shell_version": version})
    try:
        info = json.loads(remote.fetch(f"{EGO}/extension-info/?{q}"))
    except Exception as e:  # noqa: BLE001  (Netz, 404 = keine passende Version)
        raise InstallError("miss.noversion", str(e)) from e
    url = info.get("download_url")
    if not url or not url.startswith("/"):
        raise InstallError("miss.noversion", version)
    try:
        return remote.fetch(EGO + url)
    except Exception as e:  # noqa: BLE001
        raise InstallError("remote.offline", str(e)) from e


def install_extension(uuid: str, log=usercmd.log) -> str:
    """Erweiterung holen, prüfen, installieren, einschalten. Gibt 'relogin' oder 'ok' zurück."""
    if usercmd.DRY:
        log(f"[Testmodus] {EGO}/extension-info/?uuid={uuid}&shell_version={shell_version() or '?'}")
        usercmd.run(["gnome-extensions", "install", "--force", f"{uuid}.shell-extension.zip"])
        usercmd.run(["gnome-extensions", "enable", uuid])
        DRY_EXT.add(uuid)
        from .gnome import ext_enabled
        usercmd.gset("org.gnome.shell", "enabled-extensions", ext_enabled() + [uuid])
        return "relogin"
    ver = shell_version()
    if not ver:
        raise InstallError("miss.noshell")
    data = ego_download(uuid, ver)
    check_zip(data, uuid)
    with tempfile.TemporaryDirectory(prefix="linuxaio-ext-") as d:
        z = Path(d) / f"{uuid}.shell-extension.zip"
        z.write_bytes(data)
        ok, out = usercmd.run(["gnome-extensions", "install", "--force", str(z)])
        if not ok:
            raise InstallError("miss.failed", out)
    from .gnome import ext_compatible, ext_set
    ext_set(uuid, True)
    if ext_compatible(uuid) is False:
        return "incompatible"
    # Unter Wayland kennt die laufende Shell neu installierte Erweiterungen erst nach dem Neuanmelden
    return "ok" if uuid in (usercmd.query(["gnome-extensions", "list", "--enabled"]) or "") else "relogin"


def packages_for(key: str, backend: str) -> list:
    return list(PACKAGES.get(key, {}).get(backend, []))
