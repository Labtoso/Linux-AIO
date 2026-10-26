"""Mauszeiger und Hintergrundbilder aus dem Netz holen (GitHub, ohne root). Ohne Qt.
Mauszeiger → ~/.local/share/icons/<Thema>, Hintergründe → ~/Bilder/Wallpapers/<Sammlung>. Vorhandenes bleibt."""

import io
import json
import os
import shutil
import subprocess
import tarfile
import tempfile
import urllib.request
import zipfile
from pathlib import Path

import hyprstudio
from . import model, paper

GH = "https://github.com/{}/releases/latest/download/{}"

# (id, Anzeigename, URL, Themen-Ordner im Archiv)
CURSORS = [
    ("bibata-classic", "Bibata Modern Classic", GH.format("ful1e5/Bibata_Cursor", "Bibata-Modern-Classic.tar.xz"),
     ("Bibata-Modern-Classic",)),
    ("bibata-ice", "Bibata Modern Ice", GH.format("ful1e5/Bibata_Cursor", "Bibata-Modern-Ice.tar.xz"),
     ("Bibata-Modern-Ice",)),
    ("bibata-amber", "Bibata Modern Amber", GH.format("ful1e5/Bibata_Cursor", "Bibata-Modern-Amber.tar.xz"),
     ("Bibata-Modern-Amber",)),
    ("bibata-original", "Bibata Original Classic", GH.format("ful1e5/Bibata_Cursor", "Bibata-Original-Classic.tar.xz"),
     ("Bibata-Original-Classic",)),
    ("catppuccin-dark", "Catppuccin Mocha Dark", GH.format("catppuccin/cursors", "catppuccin-mocha-dark-cursors.zip"),
     ("catppuccin-mocha-dark-cursors",)),
    ("catppuccin-lavender", "Catppuccin Mocha Lavender",
     GH.format("catppuccin/cursors", "catppuccin-mocha-lavender-cursors.zip"), ("catppuccin-mocha-lavender-cursors",)),
    ("phinger", "Phinger", GH.format("phisch/phinger-cursors", "phinger-cursors-variants.tar.bz2"),
     ("phinger-cursors-dark", "phinger-cursors-light")),
    ("nordzy", "Nordzy", GH.format("guillaumeboehm/Nordzy-cursors", "Nordzy-cursors.tar.gz"), ("Nordzy-cursors",)),
    ("googledot-blue", "Google Dot Blue", GH.format("ful1e5/Google_Cursor", "GoogleDot-Blue.tar.gz"), ("GoogleDot-Blue",)),
    ("googledot-black", "Google Dot Black", GH.format("ful1e5/Google_Cursor", "GoogleDot-Black.tar.gz"),
     ("GoogleDot-Black",)),
    ("macos", "macOS", GH.format("ful1e5/apple_cursor", "macOS.tar.xz"), ("macOS", "macOS-White")),
]

# (id, Anzeigename, GitHub-Repo, Ordner)
WALL_PACKS = [
    ("catppuccin-minimal", "Catppuccin Minimal", "zhichaoh/catppuccin-wallpapers", "minimalistic"),
    ("catppuccin-landscapes", "Catppuccin Landschaften", "zhichaoh/catppuccin-wallpapers", "landscapes"),
    ("nordic", "Nordic", "linuxdotexe/nordic-wallpapers", "wallpapers"),
    ("rosepine", "Rosé Pine", "rose-pine/wallpapers", "illustration"),
    ("mountain", "Berge", "dharmx/walls", "mountain"),
    ("minimal", "Minimal", "dharmx/walls", "minimal"),
    ("digital", "Digital", "dharmx/walls", "digital"),
    ("nord", "Nord", "dharmx/walls", "nord"),
    ("anime", "Anime", "dharmx/walls", "anime"),
]
MAX_WALLS = 12


def _get(url: str, timeout: int = 90) -> bytes:
    req = urllib.request.Request(url, headers={"User-Agent": f"HyprStudio/{hyprstudio.__version__}"})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return r.read()


# ---------- Mauszeiger ----------

def icons_dir() -> Path:
    return paper.icon_dirs()[0]


def installed_cursor(entry) -> bool:
    names = {n for n, _ in paper.cursor_themes()}
    return any(n in names for n in entry[3])


def _inside(base: Path, name: str) -> bool:
    target = (base / name).resolve()
    return target == base.resolve() or base.resolve() in target.parents


def _link(dest: Path, name: str, target: str) -> None:
    """Relativen Symlink anlegen (Mauszeiger-Aliase); nur wenn er innerhalb von dest bleibt."""
    link = dest / name
    if target.startswith("/") or not _inside(dest, str(Path(name).parent / target)):
        return
    link.parent.mkdir(parents=True, exist_ok=True)
    try:
        os.symlink(target, link)
    except OSError:  # z. B. Windows im Testmodus: Datei kopieren
        src = link.parent / target
        if src.is_file():
            shutil.copy2(src, link)


def safe_extract(data: bytes, name: str, dest: Path) -> None:
    """Archiv entpacken; nichts außerhalb von dest, Symlinks nur relativ und nach innen."""
    links = []
    if name.endswith(".zip"):
        with zipfile.ZipFile(io.BytesIO(data)) as z:
            for m in z.infolist():
                if not _inside(dest, m.filename):
                    continue
                if (m.external_attr >> 16) & 0o170000 == 0o120000:
                    links.append((m.filename, z.read(m).decode("utf-8", "replace")))
                else:
                    z.extract(m, dest)
    else:
        with tarfile.open(fileobj=io.BytesIO(data)) as tf:
            ok = []
            for m in tf.getmembers():
                if not _inside(dest, m.name):
                    continue
                if m.issym():
                    links.append((m.name, m.linkname))
                elif m.isfile() or m.isdir():
                    ok.append(m)
            tf.extractall(dest, members=ok)
    for n, target in links:
        _link(dest, n, target)


def find_themes(root: Path) -> list:
    """Ordner mit index.theme und cursors/ bzw. Hyprcursor-Manifest (bis 3 Ebenen tief)."""
    out = []
    for p in sorted(root.rglob("index.theme")):
        d = p.parent
        if len(d.relative_to(root).parts) <= 3 and ((d / "cursors").is_dir() or (d / "manifest.hl").is_file()
                                                   or (d / "manifest.toml").is_file()):
            out.append(d)
    return out


def install_cursor(entry, log) -> list:
    """Lädt das Archiv und legt die Themen nach ~/.local/share/icons. Rückgabe: neu installierte Namen."""
    _, label, url, names = entry
    dest = icons_dir()
    dest.mkdir(parents=True, exist_ok=True)
    log(f"↓ {label}: {url}")
    if model.DRY:  # Testmodus: kein Netz, nur Platzhalter-Ordner
        for n in names:
            (dest / n / "cursors").mkdir(parents=True, exist_ok=True)
            (dest / n / "index.theme").write_text(f"[Icon Theme]\nName={n}\n", encoding="utf-8")
            log(f"→ {dest / n}  (Testmodus)")
        return list(names)
    return _install_archive(_get(url), url.rsplit("/", 1)[-1], log)


def _install_archive(data: bytes, filename: str, log) -> list:
    dest = icons_dir()
    dest.mkdir(parents=True, exist_ok=True)
    new = []
    with tempfile.TemporaryDirectory(prefix="hyprstudio-") as tmp:
        safe_extract(data, filename, Path(tmp))
        themes = find_themes(Path(tmp))
        if not themes:
            log(f"Kein Mauszeiger-Thema in {filename} gefunden")
        for d in themes:
            target = dest / d.name
            if target.exists():
                log(f"= {target} (schon da)")
                continue
            shutil.move(str(d), str(target))
            new.append(d.name)
            log(f"→ {target}")
    return new


# ---------- Mauszeiger-Katalog (KDE Store / pling, wie „Neue Mauszeiger holen“ in KDE) ----------

STORE = "https://api.kde-look.org/ocs/v1/content/data"
STORE_CAT = 107  # „Cursors“
STORE_SORTS = ("high", "new", "down")  # beste Bewertung, neueste, meiste Downloads
ARCHIVE_EXT = (".tar.gz", ".tgz", ".tar.xz", ".txz", ".tar.bz2", ".tbz2", ".tar", ".zip")


def _store_item(c: dict) -> dict:
    files = []
    for i in range(1, 60):
        name = c.get(f"downloadname{i}") or ""
        if not c.get(f"downloadlink{i}"):
            continue
        if name.lower().endswith(ARCHIVE_EXT):
            files.append((i, name, int(c.get(f"downloadsize{i}") or 0)))
    try:
        dl = int(c.get("downloads") or 0)
    except ValueError:
        dl = 0
    return {"id": str(c.get("id", "")), "name": str(c.get("name", "")).strip(), "score": int(c.get("score") or 50),
            "downloads": dl, "changed": str(c.get("changed") or c.get("created") or "")[:10],
            "preview": c.get("smallpreviewpic1") or c.get("previewpic1") or "", "files": files,
            "page": c.get("detailpage") or ""}


def store_search(query: str = "", sort: str = "high", page: int = 0) -> tuple:
    """([Eintrag], Gesamtzahl) – 10 pro Seite; nur Einträge mit Archiv zum Herunterladen."""
    from urllib.parse import urlencode
    params = {"categories": STORE_CAT, "sortmode": sort if sort in STORE_SORTS else "high", "page": page,
              "pagesize": 10, "format": "json"}
    if query.strip():
        params["search"] = query.strip()
    d = json.loads(_get(f"{STORE}?{urlencode(params)}", 30))
    items = [_store_item(c) for c in d.get("data") or [] if isinstance(c, dict)]
    return [x for x in items if x["files"]], int(d.get("totalitems") or 0)


def store_install(item: dict, index: int, log) -> list:
    """Download-Link ist nur kurz gültig → frisch holen, dann wie ein Release-Archiv installieren."""
    name = next((n for i, n, _ in item["files"] if i == index), "")
    log(f"↓ {item['name']}: {name}")
    if model.DRY:
        n = Path(name).name.split(".")[0] or item["name"]
        d = icons_dir() / n
        (d / "cursors").mkdir(parents=True, exist_ok=True)
        (d / "index.theme").write_text(f"[Icon Theme]\nName={n}\n", encoding="utf-8")
        log(f"→ {d}  (Testmodus)")
        return [n]
    d = json.loads(_get(f"{STORE}/{item['id']}?format=json", 30))["data"]
    c = d[0] if isinstance(d, list) else d
    return _install_archive(_get(c[f"downloadlink{index}"]), name, log)


# ---------- Hintergrundbilder ----------

def pictures_dir() -> Path:
    if model.DRY:
        return model.CACHE / "testmodus" / "Bilder"
    try:
        p = subprocess.run(["xdg-user-dir", "PICTURES"], capture_output=True, text=True, timeout=5).stdout.strip()
        if p and p != str(Path.home()):
            return Path(p)
    except (OSError, subprocess.TimeoutExpired):
        pass
    return Path.home() / "Pictures"


def wall_dir() -> Path:
    return pictures_dir() / "Wallpapers"


def library_dirs() -> list:
    dirs = [wall_dir()]
    if not model.DRY:
        dirs += [Path.home() / ".local/share/backgrounds", Path("/usr/share/hypr"), Path("/usr/share/backgrounds")]
    return dirs


def library_images(limit: int = 120) -> list:
    out = []
    for d in library_dirs():
        if not d.is_dir():
            continue
        try:
            files = sorted(p for p in d.rglob("*") if p.suffix.lower() in paper.IMAGE_EXT and p.is_file()
                           and len(p.relative_to(d).parts) <= 3)
        except OSError:
            continue
        out += files
        if len(out) >= limit:
            break
    return out[:limit]


def pack_files(pack) -> list:
    """[(name, download_url)] – die ersten MAX_WALLS Bilder der Sammlung (GitHub-API)."""
    _, _, repo, folder = pack
    data = json.loads(_get(f"https://api.github.com/repos/{repo}/contents/{folder}", 30))
    imgs = [(x["name"], x["download_url"]) for x in data
            if x.get("type") == "file" and x["name"].lower().endswith(paper.IMAGE_EXT)]
    return sorted(imgs)[:MAX_WALLS]


def install_walls(pack, log) -> int:
    dest = wall_dir() / pack[0]
    dest.mkdir(parents=True, exist_ok=True)
    log(f"↓ {pack[1]}: github.com/{pack[2]}/{pack[3]}")
    if model.DRY:
        log(f"→ {dest}  (Testmodus, nichts geladen)")
        return 0
    n = 0
    for name, url in pack_files(pack):
        f = dest / name
        if f.exists():
            continue
        f.write_bytes(_get(url))
        n += 1
        log(f"→ {f}")
    return n


def short(p: Path) -> str:
    s, home = str(p), str(Path.home())
    return "~" + s[len(home):] if s.startswith(home) else s


def open_dir(p: Path) -> None:
    p.mkdir(parents=True, exist_ok=True)
    if not model.DRY and shutil.which("xdg-open"):
        subprocess.Popen(["xdg-open", str(p)], start_new_session=True)
    elif os.name == "nt":
        os.startfile(str(p))  # noqa: S606 – nur Testmodus unter Windows


# ---------- Hintergrundbilder suchen (wallhaven.cc, nur jugendfrei) ----------

WALLHAVEN = "https://wallhaven.cc/api/v1/search"
WALL_SORTS = ("toplist", "date_added", "favorites", "random")
WALL_CATS = {"all": "110", "general": "100", "anime": "010"}
WALL_RES = {"all": "", "fhd": "1920x1080", "qhd": "2560x1440", "uhd": "3840x2160"}


def wall_search(query: str = "", sort: str = "toplist", cat: str = "all", res: str = "all", page: int = 1) -> tuple:
    """([Bild], letzte Seite) – 24 pro Seite."""
    from urllib.parse import urlencode
    params = {"q": query.strip(), "sorting": sort if sort in WALL_SORTS else "toplist", "purity": "100",
              "categories": WALL_CATS.get(cat, "110"), "page": page}
    if params["sorting"] == "toplist":
        params["topRange"] = "1y"
    if WALL_RES.get(res):
        params["atleast"] = WALL_RES[res]
    d = json.loads(_get(f"{WALLHAVEN}?{urlencode(params)}", 30))
    items = [{"id": x["id"], "thumb": x["thumbs"]["small"], "full": x["path"], "res": x.get("resolution", ""),
              "favs": int(x.get("favorites") or 0), "size": int(x.get("file_size") or 0)}
             for x in d.get("data") or [] if x.get("purity") == "sfw"]
    return items, int((d.get("meta") or {}).get("last_page") or 1)


def wall_install(item: dict, log) -> Path:
    dest = wall_dir() / "wallhaven"
    dest.mkdir(parents=True, exist_ok=True)
    f = dest / item["full"].rsplit("/", 1)[-1]
    if not f.exists():
        log(f"↓ {item['full']}")
        f.write_bytes(_get(item["full"]))
    log(f"→ {f}")
    return f
