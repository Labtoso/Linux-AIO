"""Download-Quelle GitHub: manifest.json aus dem festen Release „assets“ lesen (sonst dem neuesten), Archive laden, sha256 prüfen und
sicher nach ~/.local/share/icons entpacken. Ohne Qt (läuft in Hintergrund-Aufgaben).

manifest.json (Release-Asset):
  {"format": 1, "kategorien": {"cursors": {"eintraege": [{name, farbe, datei, groesse, sha256, vorschau}],
                                           "pakete": [{name, titel, datei, groesse, sha256, enthaelt}]},
                               "login": {...}, "icons": {...}}}
"""

import hashlib
import io
import json
import os
import shutil
import tarfile
import urllib.error
import urllib.request
from pathlib import Path

import linuxaio
from . import usercmd

DEFAULT_REPO = "Labtoso/Linux-AIO"
CATEGORIES = ("cursors", "login", "icons", "buttons")
API = "https://api.github.com/repos/{repo}/releases/{which}"
ASSET_TAG = "assets"   # festes Release mit den Downloads, unabhängig von der App-Version
UA = f"Linux-AIO/{linuxaio.__version__} (LabtosoStudios)"
MAX_ARCHIVE = 600 * 1024 * 1024
MAX_UNPACKED = 3 * 1024 * 1024 * 1024
CACHE = usercmd.CACHE / "remote"


class SourceError(Exception):
    """Klare Meldung für die Oberfläche (Schlüssel für i18n + Detail)."""

    def __init__(self, key: str, detail: str = ""):
        super().__init__(detail or key)
        self.key, self.detail = key, detail


def _get(url: str, timeout: int = 20, accept: str = "") -> bytes:
    req = urllib.request.Request(url, headers={"User-Agent": UA, **({"Accept": accept} if accept else {})})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return r.read()


fetch = _get   # Tests ersetzen das


def valid_repo(repo: str) -> bool:
    parts = repo.strip().split("/")
    ok = lambda s: bool(s) and all(c.isalnum() or c in "-_." for c in s) and s not in (".", "..")  # noqa: E731
    return len(parts) == 2 and ok(parts[0]) and ok(parts[1])


def load_manifest(repo: str = DEFAULT_REPO) -> dict:
    """{'tag', 'repo', 'assets': {name: url}, 'kategorien': {...}} oder SourceError."""
    if not valid_repo(repo):
        raise SourceError("remote.badrepo", repo)
    rel = None
    for which in (f"tags/{ASSET_TAG}", "latest"):
        try:
            rel = json.loads(fetch(API.format(repo=repo, which=which), accept="application/vnd.github+json"))
            break
        except urllib.error.HTTPError as e:
            if e.code == 404:
                continue
            if e.code == 403:
                raise SourceError("remote.limit", repo) from e
            raise SourceError("remote.http", str(e.code)) from e
        except (urllib.error.URLError, OSError, TimeoutError) as e:
            raise SourceError("remote.offline", str(getattr(e, "reason", e))) from e
        except ValueError as e:
            raise SourceError("remote.badmanifest", str(e)) from e
    if rel is None:
        raise SourceError("remote.norelease", repo)
    assets = {a.get("name"): a.get("browser_download_url") for a in rel.get("assets", []) if a.get("name")}
    if "manifest.json" not in assets:
        raise SourceError("remote.nomanifest", rel.get("tag_name", ""))
    try:
        man = json.loads(fetch(assets["manifest.json"]))
    except (urllib.error.URLError, OSError, TimeoutError) as e:
        raise SourceError("remote.offline", str(e)) from e
    except ValueError as e:
        raise SourceError("remote.badmanifest", str(e)) from e
    if not isinstance(man, dict) or not isinstance(man.get("kategorien"), dict):
        raise SourceError("remote.badmanifest", "kategorien")
    return {"tag": rel.get("tag_name", ""), "repo": repo, "assets": assets, "kategorien": man["kategorien"]}


def items(man: dict, category: str) -> tuple[list, list]:
    """Einträge und Pakete einer Kategorie, mit Download-Adressen (nur was es im Release wirklich gibt)."""
    cat = man.get("kategorien", {}).get(category) or {}
    assets = man.get("assets", {})

    def ok(e):
        return isinstance(e, dict) and _safe_name(str(e.get("name", ""))) and e.get("datei") in assets \
            and isinstance(e.get("sha256"), str) and len(e["sha256"]) == 64
    entries = [dict(e, url=assets[e["datei"]], preview_url=assets.get(e.get("vorschau") or ""))
               for e in cat.get("eintraege", []) if ok(e)]
    packs = [dict(p, url=assets[p["datei"]]) for p in cat.get("pakete", []) if ok(p)]
    return entries, packs


def _safe_name(name: str) -> bool:
    return bool(name) and "/" not in name and "\\" not in name and name not in (".", "..") and not name.startswith(".")


def download(url: str, sha: str, size_hint: int = 0, progress=None) -> bytes:
    """Archiv laden und Prüfsumme kontrollieren (falsch → SourceError, nichts wird entpackt)."""
    try:
        req = urllib.request.Request(url, headers={"User-Agent": UA})
        if fetch is not _get:
            data = fetch(url)
        else:
            buf = io.BytesIO()
            with urllib.request.urlopen(req, timeout=60) as r:
                total = int(r.headers.get("Content-Length") or size_hint or 0)
                while True:
                    block = r.read(1 << 16)
                    if not block:
                        break
                    buf.write(block)
                    if buf.tell() > MAX_ARCHIVE:
                        raise SourceError("remote.toobig")
                    if progress and total:
                        progress(int(buf.tell() * 100 / total))
            data = buf.getvalue()
    except (urllib.error.URLError, OSError, TimeoutError) as e:
        raise SourceError("remote.offline", str(getattr(e, "reason", e))) from e
    if hashlib.sha256(data).hexdigest() != sha.lower():
        raise SourceError("remote.badsum")
    return data


def safe_extract(data: bytes, dest_root: Path, allowed: set | None = None) -> list:
    """tar.xz sicher entpacken: nur normale Dateien/Ordner, nichts außerhalb, keine Links,
    nur erlaubte Theme-Ordner oben, vorhandene Themes werden nie überschrieben. Gibt neue Themes zurück."""
    try:
        tar = tarfile.open(fileobj=io.BytesIO(data), mode="r:xz")
        members = tar.getmembers()
    except (tarfile.TarError, EOFError, OSError) as e:
        raise SourceError("remote.badarchive", str(e)) from e
    total, tops = 0, set()
    for m in members:
        p = Path(m.name)
        if m.name.startswith(("/", "\\")) or p.is_absolute() or ".." in p.parts or not p.parts:
            raise SourceError("remote.badarchive", m.name)
        if not (m.isfile() or m.isdir()):
            raise SourceError("remote.badarchive", f"{m.name} (Link/Gerät)")
        top = p.parts[0]
        if not _safe_name(top) or (allowed is not None and top not in allowed):
            raise SourceError("remote.badarchive", top)
        tops.add(top)
        total += max(0, m.size)
        if total > MAX_UNPACKED:
            raise SourceError("remote.toobig")
    dest_root.mkdir(parents=True, exist_ok=True)
    new = []
    for top in sorted(tops):
        if (dest_root / top).exists():
            continue
        tmp = dest_root / f".{top}.linuxaio-tmp"
        if tmp.exists():
            shutil.rmtree(tmp)
        tmp.mkdir()
        for m in members:
            p = Path(m.name)
            if p.parts[0] != top:
                continue
            target = tmp.joinpath(*p.parts[1:]) if len(p.parts) > 1 else tmp
            if m.isdir():
                target.mkdir(parents=True, exist_ok=True)
                continue
            target.parent.mkdir(parents=True, exist_ok=True)
            src = tar.extractfile(m)
            with open(target, "wb") as fh:
                shutil.copyfileobj(src, fh)
            os.chmod(target, 0o644)
        os.replace(tmp, dest_root / top)
        new.append(dest_root / top)
    return new


def install(entry: dict, dest_root: Path, progress=None) -> list:
    """Eintrag oder Paket laden, prüfen, entpacken."""
    data = download(entry["url"], entry["sha256"], int(entry.get("groesse") or 0), progress)
    allowed = set(entry.get("enthaelt") or [entry["name"]])
    return safe_extract(data, dest_root, allowed)


def preview(url: str) -> bytes | None:
    """Vorschaubild (zwischengespeichert)."""
    if not url:
        return None
    CACHE.mkdir(parents=True, exist_ok=True)
    f = CACHE / hashlib.sha1(url.encode()).hexdigest()
    if f.is_file():
        return f.read_bytes()
    try:
        data = fetch(url)
    except (urllib.error.URLError, OSError, TimeoutError):
        return None
    if data[:8] == bytes([0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]):
        f.write_bytes(data)
        return data
    return None
