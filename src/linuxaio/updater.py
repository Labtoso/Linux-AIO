"""Check for updates: newest v-release of Labtoso/Linux-AIO (the „assets“ release is ignored), changelog, AppImage
download with sha256 check (SHA256SUMS in the release), atomic replacement of ~/.local/bin/Linux-AIO.AppImage
(the old one goes to ~/.local/share/LabtosoStudios/linuxaio/old/<version>/). Settings stay. No Qt."""

import hashlib
import json
import os
import re
import shutil
import urllib.error
import urllib.request
from pathlib import Path

import linuxaio
from . import usercmd
from .remote import UA, SourceError

API = "https://api.github.com/repos/Labtoso/Linux-AIO/releases?per_page=20"


def vtuple(v: str) -> tuple:
    return tuple(int(x) for x in re.findall(r"\d+", v)[:3]) or (0,)


def _get(url: str, timeout: int = 20) -> bytes:
    req = urllib.request.Request(url, headers={"User-Agent": UA, "Accept": "application/vnd.github+json"})
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            return r.read()
    except (urllib.error.URLError, OSError, TimeoutError) as e:
        raise SourceError("remote.offline", str(getattr(e, "reason", e))) from e


fetch = _get   # Tests ersetzen das


def latest(releases: list | None = None) -> dict | None:
    """{version, notes, appimage_url, sums_url, size} of the newest real release, or None."""
    if releases is None:
        try:
            releases = json.loads(fetch(API))
        except SourceError as e:
            if "rate limit" not in str(e.detail).lower() and "403" not in str(e.detail):
                raise
            return _latest_without_api()             # API-Limit erreicht: über die Release-Seite
    best = None
    for r in releases:
        tag = str(r.get("tag_name", ""))
        if r.get("draft") or r.get("prerelease") or not re.fullmatch(r"v\d+\.\d+\.\d+", tag):
            continue                                   # „assets“ und Entwürfe ignorieren
        assets = {a.get("name", ""): a for a in r.get("assets", [])}
        app = next((a for n, a in assets.items() if n.endswith("-x86_64.AppImage")), None)
        sums = assets.get("SHA256SUMS")
        if not app or not sums:
            continue
        info = {"version": tag[1:], "notes": str(r.get("body") or ""), "appimage_url": app["browser_download_url"],
                "appimage_name": app["name"], "sums_url": sums["browser_download_url"], "size": int(app.get("size") or 0)}
        if best is None or vtuple(info["version"]) > vtuple(best["version"]):
            best = info
    return best


def _latest_without_api() -> dict | None:
    """Fallback without the GitHub API: …/releases/latest redirects to the newest tag."""
    req = urllib.request.Request("https://github.com/Labtoso/Linux-AIO/releases/latest", headers={"User-Agent": UA},
                                 method="HEAD")
    try:
        with urllib.request.urlopen(req, timeout=20) as r:
            tag = r.geturl().rstrip("/").rsplit("/", 1)[-1]
    except (urllib.error.URLError, OSError, TimeoutError) as e:
        raise SourceError("remote.offline", str(getattr(e, "reason", e))) from e
    if not re.fullmatch(r"v\d+\.\d+\.\d+", tag):
        return None
    base = f"https://github.com/Labtoso/Linux-AIO/releases/download/{tag}/"
    name = f"Linux-AIO-{tag[1:]}-x86_64.AppImage"
    return {"version": tag[1:], "notes": "", "appimage_url": base + name, "appimage_name": name,
            "sums_url": base + "SHA256SUMS", "size": 0}


def newer(info: dict | None) -> bool:
    return bool(info) and vtuple(info["version"]) > vtuple(linuxaio.__version__)


def short_notes(notes: str, lines: int = 14) -> str:
    out = [ln for ln in notes.replace("\r", "").splitlines() if ln.strip() and not ln.startswith("## Install")]
    return "\n".join(out[:lines])


def target() -> Path:
    return usercmd.real_home() / ".local/bin/Linux-AIO.AppImage"


def install(info: dict, progress=None, log=usercmd.log) -> Path:
    """Download → check sha256 → move the old AppImage to old/ → put the new one in place atomically."""
    sums = fetch(info["sums_url"]).decode("utf-8", "replace")
    want = next((ln.split()[0] for ln in sums.splitlines() if ln.strip().endswith(info["appimage_name"])), "")
    if not re.fullmatch(r"[0-9a-f]{64}", want):
        raise SourceError("remote.badsum", "SHA256SUMS")
    dst = target()
    dst.parent.mkdir(parents=True, exist_ok=True)
    part = dst.with_name(dst.name + ".part")
    h = hashlib.sha256()
    req = urllib.request.Request(info["appimage_url"], headers={"User-Agent": UA})
    try:
        with urllib.request.urlopen(req, timeout=60) as r, open(part, "wb") as f:
            total = int(r.headers.get("Content-Length") or info.get("size") or 0)
            done = 0
            while True:
                block = r.read(1 << 16)
                if not block:
                    break
                f.write(block)
                h.update(block)
                done += len(block)
                if progress and total:
                    progress(int(done * 100 / total))
    except (urllib.error.URLError, OSError, TimeoutError) as e:
        part.unlink(missing_ok=True)
        raise SourceError("remote.offline", str(getattr(e, "reason", e))) from e
    if h.hexdigest() != want:
        part.unlink(missing_ok=True)
        raise SourceError("remote.badsum")
    os.chmod(part, 0o755)
    if dst.is_file():
        old = usercmd.real_home() / ".local/share/LabtosoStudios/linuxaio/old" / linuxaio.__version__
        old.mkdir(parents=True, exist_ok=True)
        shutil.copy2(dst, old / dst.name)
        log(f"{dst} → {old}")
    os.replace(part, dst)                              # atomar: läuft die alte Version noch, stört das nicht
    usercmd.run(["gio", "set", str(dst), "metadata::custom-icon",
                 "file://" + str(usercmd.real_home() / ".local/share/icons/hicolor/256x256/apps/de.labtosostudios.linuxaio.png")])
    log(f"→ {dst} ({info['version']})")
    return dst
