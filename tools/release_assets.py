"""Release-Dateien für die Download-Quelle bauen: ein .tar.xz pro Mauszeiger-Theme, ein Gesamtpaket,
Vorschaubilder, LICENSE und manifest.json.

Start:  python3 tools/release_assets.py <Ordner mit Bibata-Material-*> [Zielordner]
        (Ziel Standard: release-assets/ im Projekt)

Alias-Links der Mauszeiger (pointer, text, default …) werden als echte Dateien ins Archiv gelegt:
keine Symlinks im Archiv, trotzdem vollständige Themes. Quelle daher ein Ordner MIT Links
(z. B. ~/.local/share/icons), nicht eine Cloud-Kopie, die Links verliert.
"""

import hashlib
import io
import json
import shutil
import subprocess
import sys
import tarfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
LICENSE_SRC = Path("/usr/share/licenses/spdx/GPL-3.0-only.txt")
SOURCE_URL = "https://github.com/SakibShahariar/material-bibata-cursor"
PACK = "bibata-material-all"


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def _filter(ti: tarfile.TarInfo) -> tarfile.TarInfo | None:
    if not (ti.isfile() or ti.isdir()):
        return None                     # keine Links/Geräte in den Archiven
    ti.uid = ti.gid = 0
    ti.uname = ti.gname = ""
    ti.mode = 0o755 if ti.isdir() else 0o644
    return ti


def make_archive(folders: list, target: Path) -> None:
    """tar (Links aufgelöst, Besitzer 0) durch xz – schneller als Pythons lzma."""
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w", format=tarfile.PAX_FORMAT, dereference=True) as tar:
        for d in folders:
            tar.add(d, arcname=d.name, filter=_filter)
    with open(target, "wb") as out:
        # -9 (64-MB-Fenster): die Alias-Kopien kosten so fast keinen Platz
        subprocess.run(["xz", "-T1", "-9", "-c"], input=buf.getvalue(), stdout=out, check=True)


def preview(theme: Path, target: Path) -> bool:
    from linuxaio.look_page import cursor_preview
    img = cursor_preview(theme)
    return img is not None and img.save(str(target), "PNG")


def main(argv) -> int:
    if len(argv) < 2:
        print(__doc__)
        return 2
    src = Path(argv[1]).expanduser()
    out = Path(argv[2]).expanduser() if len(argv) > 2 else ROOT / "release-assets"
    out.mkdir(parents=True, exist_ok=True)
    from linuxaio.qt import QtWidgets
    _app = QtWidgets.QApplication.instance() or QtWidgets.QApplication(["release"])
    themes = sorted(p for p in src.iterdir() if p.is_dir() and (p / "cursors").is_dir()
                    and p.name.startswith("Bibata-Material-"))
    if not themes:
        print("Keine Mauszeiger-Themes gefunden:", src)
        return 1
    entries = []
    for th in themes:
        arc = out / f"{th.name}.tar.xz"
        png = out / f"{th.name}.png"
        if not arc.is_file():
            make_archive([th], arc)
        if not png.is_file():
            preview(th, png)
        farbe = th.name.removeprefix("Bibata-Material-").replace("-", " ")
        entries.append({"name": th.name, "farbe": farbe, "datei": arc.name, "groesse": arc.stat().st_size,
                        "sha256": sha256(arc), "vorschau": png.name if png.is_file() else ""})
        print(f"{arc.name}  {arc.stat().st_size // 1024} KB")
    pack = out / f"{PACK}.tar.xz"
    if not pack.is_file():
        make_archive(themes, pack)
    print(f"{pack.name}  {pack.stat().st_size // (1024 * 1024)} MB")
    if LICENSE_SRC.is_file():
        shutil.copyfile(LICENSE_SRC, out / "LICENSE")
    manifest = {
        "format": 1,
        "kategorien": {
            "cursors": {
                "lizenz": "GPL-3.0 (Bibata, Material Bibata Cursor)", "quelle": SOURCE_URL,
                "eintraege": entries,
                "pakete": [{"name": PACK, "titel": "Bibata Material – alle Farben", "datei": pack.name,
                            "groesse": pack.stat().st_size, "sha256": sha256(pack),
                            "enthaelt": [e["name"] for e in entries]}],
            },
            "login": {"eintraege": [], "pakete": []},
            "icons": {"eintraege": [], "pakete": []},
            "buttons": {"eintraege": [], "pakete": []},
        },
    }
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print("manifest.json:", len(entries), "Mauszeiger + 1 Paket")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
