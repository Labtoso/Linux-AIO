"""Schriften direkt von der Quelle installieren: Google Fonts (OFL/Apache/UFL) und Nerd Fonts (GitHub-Releases).
Nichts davon liegt in unserem Repo. Ziel: ~/.local/share/fonts/LinuxAIO/<Familie>/, danach fc-cache. Ohne Qt."""

import io
import json
import re
import shutil
import tarfile
import time
import urllib.parse
from pathlib import Path

from . import remote, usercmd

GOOGLE_META = "https://fonts.google.com/metadata/fonts"
GOOGLE_LIST = "https://fonts.google.com/download/list?family={family}"
NERD_API = "https://api.github.com/repos/ryanoasis/nerd-fonts/releases/latest"
NERD_URL = "https://github.com/ryanoasis/nerd-fonts/releases/download/{tag}/{name}.tar.xz"
CATEGORIES = ("Sans Serif", "Serif", "Monospace", "Display", "Handwriting", "Nerd Font")
FONT_EXT = (".ttf", ".otf")
MAX_FONT_ARCHIVE = 400 * 1024 * 1024
CACHE_TTL = 24 * 3600


def fonts_dir() -> Path:
    return usercmd.data_dir() / "fonts" / "LinuxAIO"


def cache_dir() -> Path:
    return usercmd.CACHE / "fonts"


def safe_family(name: str) -> str:
    """Ordnername aus dem Familiennamen (keine Pfadzeichen)."""
    s = re.sub(r"[^A-Za-z0-9 ._+-]", "", name).strip().strip(".")
    return s or "Font"


def _json(data: bytes):
    text = data.decode("utf-8", "replace")
    return json.loads(text[text.index("{"):])          # Google setzt )]}' davor


def _cached(name: str, url: str, accept: str = ""):
    f = cache_dir() / name
    if f.is_file() and time.time() - f.stat().st_mtime < CACHE_TTL:
        try:
            return _json(f.read_bytes())
        except ValueError:
            pass
    try:
        data = remote.fetch(url, timeout=30, accept=accept) if accept else remote.fetch(url)
    except Exception as e:  # noqa: BLE001  (offline → klare Meldung)
        if f.is_file():
            return _json(f.read_bytes())
        raise remote.SourceError("remote.offline", str(getattr(e, "reason", e))) from e
    f.parent.mkdir(parents=True, exist_ok=True)
    f.write_bytes(data)
    return _json(data)


# ---------- Kataloge ----------

def google_catalog() -> list:
    d = _cached("google.json", GOOGLE_META)
    out = []
    for e in d.get("familyMetadataList", []):
        if not e.get("isOpenSource", True) or e.get("isBrandFont"):
            continue
        out.append({"family": e["family"], "category": e.get("category", "Sans Serif"), "source": "google",
                    "popularity": int(e.get("popularity") or 9999), "noto": bool(e.get("isNoto"))})
    return sorted(out, key=lambda x: x["popularity"])


def nerd_catalog() -> list:
    d = _cached("nerd.json", NERD_API, accept="application/vnd.github+json")
    tag = d.get("tag_name", "")
    out = []
    for a in d.get("assets", []):
        n = a.get("name", "")
        if n.endswith(".tar.xz"):
            base = n[:-7]
            out.append({"family": f"{base} Nerd Font", "archive": base, "category": "Nerd Font", "source": "nerd",
                        "popularity": 0, "tag": tag, "size": int(a.get("size") or 0),
                        "url": a.get("browser_download_url") or NERD_URL.format(tag=tag, name=base)})
    return sorted(out, key=lambda x: x["family"].lower())


def catalog() -> tuple[list, str]:
    """Beide Quellen; Fehler einer Quelle → die andere bleibt, Meldung zurück."""
    items, errors = [], []
    for fn in (google_catalog, nerd_catalog):
        try:
            items += fn()
        except remote.SourceError as e:
            errors.append(e.key)
    return items, (errors[0] if errors and not items else "")


# ---------- Installieren ----------

def installed(family: str) -> Path | None:
    p = fonts_dir() / safe_family(family)
    return p if p.is_dir() and any(p.glob("*.[ot]tf")) else None


def list_installed() -> set:
    try:
        return {p.name for p in fonts_dir().iterdir() if p.is_dir()}
    except OSError:
        return set()


def google_files(family: str) -> tuple[list, dict]:
    """(Schrift-URLs, {Lizenzdatei: Text}). Variable Schriften bevorzugt, sonst statische."""
    data = remote.fetch(GOOGLE_LIST.format(family=urllib.parse.quote(family)))
    man = _json(data).get("manifest", {})
    refs = [r for r in man.get("fileRefs", []) if r.get("filename", "").lower().endswith(FONT_EXT)]
    variable = [r for r in refs if not r["filename"].startswith("static/")]
    use = variable or refs
    licenses = {f["filename"]: f.get("contents", "") for f in man.get("files", [])
                if re.match(r"^(OFL|LICENSE|UFL)", f.get("filename", ""), re.I)}
    urls = []
    for r in use:
        u = r.get("url", "")
        if urllib.parse.urlparse(u).netloc.endswith("fonts.gstatic.com"):
            urls.append((Path(r["filename"]).name, u))
    return urls, licenses


def _ok_font(data: bytes) -> bool:
    return data[:4] in (b"\x00\x01\x00\x00", b"OTTO", b"true", b"ttcf")


def _finish(tmp: Path, dest: Path, log) -> Path:
    if dest.exists():
        shutil.rmtree(tmp)
        return dest
    tmp.replace(dest)
    log(f"→ {dest}")
    usercmd.run(["fc-cache", "-f", str(dest.parent)], timeout=120)
    return dest


def install_google(family: str, log=usercmd.log) -> Path:
    dest = fonts_dir() / safe_family(family)
    if installed(family):
        return dest
    urls, licenses = google_files(family)
    if not urls:
        raise remote.SourceError("fonts.nofiles", family)
    tmp = dest.with_name(f".{dest.name}.linuxaio-tmp")
    if tmp.exists():
        shutil.rmtree(tmp)
    tmp.mkdir(parents=True)
    for name, url in urls:
        data = remote.fetch(url)
        if not _ok_font(data):
            shutil.rmtree(tmp)
            raise remote.SourceError("fonts.bad", name)
        (tmp / re.sub(r"[^A-Za-z0-9._+\[\],-]", "_", name)).write_bytes(data)
    for name, text in licenses.items():
        (tmp / Path(name).name).write_text(text, encoding="utf-8")
    return _finish(tmp, dest, log)


def install_nerd(item: dict, log=usercmd.log) -> Path:
    dest = fonts_dir() / safe_family(item["family"])
    if installed(item["family"]):
        return dest
    try:
        data = remote.fetch(item["url"])
    except Exception as e:  # noqa: BLE001
        raise remote.SourceError("remote.offline", str(e)) from e
    if len(data) > MAX_FONT_ARCHIVE:
        raise remote.SourceError("remote.toobig")
    try:
        tar = tarfile.open(fileobj=io.BytesIO(data), mode="r:xz")
        members = tar.getmembers()
    except (tarfile.TarError, EOFError, OSError) as e:
        raise remote.SourceError("remote.badarchive", str(e)) from e
    tmp = dest.with_name(f".{dest.name}.linuxaio-tmp")
    if tmp.exists():
        shutil.rmtree(tmp)
    tmp.mkdir(parents=True)
    count = 0
    for m in members:            # nur Schriften und Lizenzen, flach, keine Links/Pfade
        base = Path(m.name).name
        if not m.isfile() or not base or base.startswith("."):
            continue
        low = base.lower()
        if low.endswith(FONT_EXT) or low in ("license", "license.txt", "ofl.txt", "license.md"):
            fh = tar.extractfile(m)
            if fh is None:
                continue
            (tmp / base).write_bytes(fh.read())
            count += low.endswith(FONT_EXT)
    if not count:
        shutil.rmtree(tmp)
        raise remote.SourceError("fonts.nofiles", item["family"])
    return _finish(tmp, dest, log)


def install(item: dict, log=usercmd.log) -> Path:
    return install_nerd(item, log) if item.get("source") == "nerd" else install_google(item["family"], log)


def remove(family: str, log=usercmd.log) -> bool:
    """Nur unsere eigenen Ordner unter fonts/LinuxAIO entfernen (vom Nutzer per Knopf gewünscht)."""
    p = fonts_dir() / safe_family(family)
    if not p.is_dir() or p.parent != fonts_dir():
        return False
    shutil.rmtree(p)
    log(f"entfernt: {p}")
    usercmd.run(["fc-cache", "-f", str(fonts_dir())], timeout=120)
    return True


# ---------- Vorschau ----------

def preview_file(item: dict) -> Path | None:
    """Eine Datei für die Live-Vorschau: installierte Schrift, sonst (Google) eine Datei in den Cache laden."""
    inst = installed(item["family"])
    if inst:
        files = sorted(inst.glob("*.[ot]tf"), key=lambda p: ("regular" not in p.name.lower(), len(p.name)))
        return files[0] if files else None
    if item.get("source") != "google":
        return None
    d = cache_dir() / "preview" / safe_family(item["family"])
    have = sorted(d.glob("*.[ot]tf")) if d.is_dir() else []
    if have:
        return have[0]
    urls, _ = google_files(item["family"])
    if not urls:
        return None
    urls.sort(key=lambda x: ("regular" not in x[0].lower() and "[" not in x[0], len(x[0])))
    name, url = urls[0]
    data = remote.fetch(url)
    if not _ok_font(data):
        return None
    d.mkdir(parents=True, exist_ok=True)
    f = d / re.sub(r"[^A-Za-z0-9._+\[\],-]", "_", name)
    f.write_bytes(data)
    return f
