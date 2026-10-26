"""SDDM-Themes: lokal lesen, aktuelles Theme bestimmen, KDE-Store (OCS, wie die SDDM-KCM über KNewStuff).

Die KCM nutzt sddmtheme.knsrc: Kategorie "SDDM Theme" (OCS-ID 101) auf store.kde.org,
lädt das Archiv herunter und entpackt es als root nach /usr/share/sddm/themes.
"""

import configparser
import json
import os
import re
import shutil
import tempfile
from pathlib import Path
from urllib.parse import urlencode

DRY = os.environ.get("DEMANAGER_DRYRUN") == "1" or os.name == "nt"

OCS = "https://api.kde-look.org/ocs/v1"
CATEGORY = "101"
STORE_PAGE = "https://store.kde.org/p/{id}"
SORTS = ("new", "down", "high")          # neu, beliebt (Downloads), Bewertung
PAGE_SIZE = 24

NAME_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._+-]{0,80}$")


def themes_dir() -> Path:
    if DRY:
        return Path(tempfile.gettempdir()) / "desddm-dry" / "themes"
    return Path("/usr/share/sddm/themes")


def conf_files() -> list:
    if DRY:
        base = Path(tempfile.gettempdir()) / "desddm-dry"
        return sorted((base / "sddm.conf.d").glob("*.conf")) + [base / "sddm.conf"]
    out = []
    for d in (Path("/usr/lib/sddm/sddm.conf.d"), Path("/etc/sddm.conf.d")):
        out += sorted(d.glob("*.conf")) if d.is_dir() else []
    return out + [Path("/etc/sddm.conf")]


def kde_settings_file() -> Path:
    if DRY:
        return Path(tempfile.gettempdir()) / "desddm-dry" / "sddm.conf.d" / "kde_settings.conf"
    return Path("/etc/sddm.conf.d/kde_settings.conf")


def main_conf_file() -> Path:
    return conf_files()[-1]


def store_state_file() -> Path:
    if DRY:
        return Path(tempfile.gettempdir()) / "desddm-dry" / "sddm-state.json"
    return Path("/var/lib/labtoso-linuxaio/sddm.json")


def load_store_state() -> dict:
    from .system import load_state_file
    return load_state_file(store_state_file())


def read_ini(path: Path) -> configparser.ConfigParser:
    cp = configparser.ConfigParser(interpolation=None, strict=False)
    cp.optionxform = str
    try:
        cp.read(path, encoding="utf-8")
    except (OSError, configparser.Error, UnicodeDecodeError):
        pass
    return cp


def current_theme() -> str:
    cur = ""
    for f in conf_files():
        cp = read_ini(f)
        v = cp.get("Theme", "Current", fallback=None)
        if v is not None:
            cur = v.strip()
    return cur


def is_installed_sddm() -> bool:
    return DRY or Path("/usr/share/sddm").is_dir() or shutil.which("sddm") is not None


def theme_info(path: Path) -> dict | None:
    meta = path / "metadata.desktop"
    if not meta.is_file():
        return None
    cp = read_ini(meta)
    sec = "SddmGreeterTheme" if cp.has_section("SddmGreeterTheme") else (cp.sections()[0] if cp.sections() else None)
    g = (lambda k, d="": cp.get(sec, k, fallback=d).strip()) if sec else (lambda k, d="": d)
    shot = g("Screenshot")
    shot_path = (path / shot) if shot else None
    if not shot_path or not shot_path.is_file():
        shot_path = next((p for n in ("preview.png", "screenshot.png", "preview.jpg", "screenshot.jpg")
                          for p in [path / n] if p.is_file()), None)
    qt = g("QtVersion")
    return {
        "id": path.name,
        "name": g("Name", path.name) or path.name,
        "author": g("Author"),
        "description": g("Description"),
        "version": g("Version"),
        "screenshot": str(shot_path) if shot_path else "",
        "qt6": qt.startswith("6"),
        "path": str(path),
    }


def list_themes() -> list:
    d = themes_dir()
    out = []
    if d.is_dir():
        for p in sorted(d.iterdir(), key=lambda x: x.name.lower()):
            if p.is_dir():
                info = theme_info(p)
                if info:
                    out.append(info)
    return out


def greeter_command(theme: dict) -> list | None:
    if theme.get("qt6"):
        exe = shutil.which("sddm-greeter-qt6") or shutil.which("sddm-greeter")
    else:
        exe = shutil.which("sddm-greeter") or shutil.which("sddm-greeter-qt6")
    return [exe, "--test-mode", "--theme", theme["path"]] if exe else None


# ---------- KDE Store (OCS) ----------

def list_url(page: int = 0, sort: str = "down", search: str = "") -> str:
    q = {"categories": CATEGORY, "format": "json", "pagesize": PAGE_SIZE, "page": page,
         "sortmode": sort if sort in SORTS else "down"}
    if search.strip():
        q["search"] = search.strip()
    return f"{OCS}/content/data?{urlencode(q)}"


def item_url(item_id: str) -> str:
    return f"{OCS}/content/data/{item_id}?format=json"


def _files(c: dict) -> list:
    files = []
    i = 1
    while f"downloadlink{i}" in c or f"downloadname{i}" in c:
        name = c.get(f"downloadname{i}") or ""
        link = c.get(f"downloadlink{i}") or ""
        if name and link and archive_ok(name):
            try:
                size = int(c.get(f"downloadsize{i}") or 0)
            except ValueError:
                size = 0
            files.append({"index": i, "name": name, "size_kb": size, "link": link})
        i += 1
    return files


def archive_ok(name: str) -> bool:
    n = name.lower()
    return n.endswith((".tar.gz", ".tgz", ".tar.xz", ".txz", ".tar.bz2", ".tbz2", ".tar.zst", ".tar", ".zip"))


def parse_items(data: bytes) -> tuple[list, int]:
    try:
        d = json.loads(data.decode("utf-8", "replace"))
    except ValueError:
        return [], 0
    items = []
    for c in d.get("data") or []:
        if not isinstance(c, dict) or not c.get("id"):
            continue
        tags = (c.get("tags") or "").lower()
        items.append({
            "store_id": str(c["id"]),
            "name": (c.get("name") or "").strip(),
            "author": c.get("personid") or "",
            "downloads": int(c.get("downloads") or 0),
            "score": int(c.get("score") or 0),
            "preview": c.get("previewpic1") or c.get("smallpreviewpic1") or "",
            "small": c.get("smallpreviewpic1") or c.get("previewpic1") or "",
            "summary": re.sub(r"<[^>]+>", "", c.get("summary") or c.get("description") or "")[:400],
            "qt6": "qt6" in tags or "plasma6" in tags or "plasma 6" in tags,
            "changed": (c.get("changed") or "")[:10],
            "files": _files(c),
        })
    try:
        total = int(d.get("totalitems") or 0)
    except ValueError:
        total = 0
    return items, total


def parse_item(data: bytes) -> dict | None:
    items, _ = parse_items(data)
    return items[0] if items else None


def safe_name(name: str) -> str:
    n = re.sub(r"[^A-Za-z0-9._+-]+", "-", name).strip("-.")
    return n[:80] or "theme"


# ---------- gemeinsame Schnittstelle für die Theme-Seite ----------

def is_installed() -> bool:
    from . import system
    return system.sddm_installed()


def theme_chips(th: dict) -> list:
    from .i18n import t
    return [("Qt6", "Chip", "")] if th.get("qt6") else [("Qt5", "ChipWarn", t("sddm.qt5"))]


def test_command(th: dict):
    return greeter_command(th)
