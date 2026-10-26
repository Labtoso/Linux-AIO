"""Build the icon theme downloads for the release "assets": one .tar.xz per theme (relative symlinks kept – icon
themes need them), preview PNG, licence, and the "icons" category in manifest.json.

Sources are the authors' official GitHub repositories (all GPL-3.0); only themes that ship ready-to-use folders
(no install/build scripts are run).

Start:  python3 tools/icon_assets.py <folder with the downloaded repo .tar.gz files> <release-assets folder>
"""

import hashlib
import io
import json
import shutil
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

# (theme name, archive file, folder in the archive, extra themes it needs, repo)
THEMES = [
    ("Papirus", "PapirusDevelopmentTeam_papirus-icon-theme", "Papirus", (), "PapirusDevelopmentTeam/papirus-icon-theme"),
    ("Papirus-Dark", "PapirusDevelopmentTeam_papirus-icon-theme", "Papirus-Dark", ("Papirus",), "PapirusDevelopmentTeam/papirus-icon-theme"),
    ("Papirus-Light", "PapirusDevelopmentTeam_papirus-icon-theme", "Papirus-Light", ("Papirus",), "PapirusDevelopmentTeam/papirus-icon-theme"),
    ("Numix", "numixproject_numix-icon-theme", "Numix", (), "numixproject/numix-icon-theme"),
    ("Numix-Light", "numixproject_numix-icon-theme", "Numix-Light", ("Numix",), "numixproject/numix-icon-theme"),
    ("Numix-Circle", "numixproject_numix-icon-theme-circle", "Numix-Circle", ("Numix",), "numixproject/numix-icon-theme-circle"),
    ("Numix-Circle-Light", "numixproject_numix-icon-theme-circle", "Numix-Circle-Light", ("Numix", "Numix-Light"), "numixproject/numix-icon-theme-circle"),
    ("kora", "bikass_kora", "kora", (), "bikass/kora"),
    ("kora-pgrey", "bikass_kora", "kora-pgrey", ("kora",), "bikass/kora"),
    ("Flat-Remix-Blue-Dark", "daniruiz_flat-remix", "Flat-Remix-Blue-Dark", (), "daniruiz/flat-remix"),
    ("Flat-Remix-Blue-Light", "daniruiz_flat-remix", "Flat-Remix-Blue-Light", (), "daniruiz/flat-remix"),
    ("Flat-Remix-Green-Dark", "daniruiz_flat-remix", "Flat-Remix-Green-Dark", (), "daniruiz/flat-remix"),
    ("candy-icons", "EliverLara_candy-icons", "", (), "EliverLara/candy-icons"),
    ("MoreWaita", "somepaulo_MoreWaita", "", (), "somepaulo/MoreWaita"),
    ("Gruvbox-Plus-Dark", "SylEleuth_gruvbox-plus-icon-pack", "Gruvbox-Plus-Dark", (), "SylEleuth/gruvbox-plus-icon-pack"),
    ("Gruvbox-Plus-Light", "SylEleuth_gruvbox-plus-icon-pack", "Gruvbox-Plus-Light", (), "SylEleuth/gruvbox-plus-icon-pack"),
    ("Zafiro-Icons-Dark", "zayronxio_Zafiro-icons", "Dark", (), "zayronxio/Zafiro-icons"),
    ("Zafiro-Icons-Light", "zayronxio_Zafiro-icons", "Light", (), "zayronxio/Zafiro-icons"),
    ("Obsidian", "madmaxms_iconpack-obsidian", "Obsidian", (), "madmaxms/iconpack-obsidian"),
    ("Obsidian-Aqua", "madmaxms_iconpack-obsidian", "Obsidian-Aqua", ("Obsidian",), "madmaxms/iconpack-obsidian"),
]


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for b in iter(lambda: f.read(1 << 20), b""):
            h.update(b)
    return h.hexdigest()


def extract(src_tgz: Path, folder: str, dest: Path, name: str) -> Path:
    """Folder from the repo archive into dest/<name>; relative links inside are kept ("data" filter)."""
    out = dest / name
    if out.exists():
        return out
    with tarfile.open(src_tgz, "r:gz") as tar:
        top = tar.getmembers()[0].name.split("/")[0]
        prefix = f"{top}/{folder}/" if folder else f"{top}/"
        members = []
        for m in tar.getmembers():
            if not m.name.startswith(prefix) or m.name == prefix.rstrip("/"):
                continue
            rel = m.name[len(prefix):]
            if not folder and "/" not in rel and not (rel == "index.theme" or m.isdir()):
                continue                    # repo-root themes: no README/scripts at the top
            if not folder and rel.split("/")[0].startswith((".", "_")):
                continue                    # .github, _dev, _extras … are not part of the theme
            m.name = f"{name}/{rel}"
            if m.issym() or m.islnk():
                if m.linkname.startswith("/") or (m.islnk()):
                    continue
            members.append(m)
        tmp = Path(tempfile.mkdtemp(dir=dest))
        tar.extractall(tmp, members=members, filter="data")
        (tmp / name).rename(out)
        shutil.rmtree(tmp)
    return out


def licence(src_tgz: Path) -> str:
    with tarfile.open(src_tgz, "r:gz") as tar:
        for m in tar.getmembers():
            parts = m.name.split("/")
            if len(parts) == 2 and parts[1].lower().startswith(("license", "copying")) and m.isfile():
                return tar.extractfile(m).read().decode("utf-8", "replace")
    return ""


def pack(dirs: list, target: Path) -> None:
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w", format=tarfile.PAX_FORMAT) as tar:
        for d in dirs:
            def flt(ti):
                if not (ti.isfile() or ti.isdir() or ti.issym()):
                    return None
                ti.uid = ti.gid = 0
                ti.uname = ti.gname = ""
                ti.mode = 0o755 if ti.isdir() else 0o644
                return ti
            tar.add(d, arcname=d.name, filter=flt)
    with open(target, "wb") as out:
        subprocess.run(["xz", "-T0", "-6", "-c"], input=buf.getvalue(), stdout=out, check=True)


def main(argv) -> int:
    if len(argv) < 3:
        print(__doc__)
        return 2
    src, out = Path(argv[1]).expanduser(), Path(argv[2]).expanduser()
    work = out.parent / "icons-work"
    work.mkdir(parents=True, exist_ok=True)
    from linuxaio.look_page import icon_preview
    from linuxaio.qt import QtWidgets
    _app = QtWidgets.QApplication.instance() or QtWidgets.QApplication(["icons"])
    themes = {t[0]: t for t in THEMES}
    entries = []
    for name, arc, folder, needs, repo in THEMES:
        tgz = src / f"{arc}.tar.gz"
        d = extract(tgz, folder, work, name)
        deps = [extract(src / f"{themes[n][1]}.tar.gz", themes[n][2], work, n) for n in needs]
        lic = licence(tgz)
        if lic:
            (d / "LICENSE").write_text(lic, encoding="utf-8")
        target = out / f"icons-{name}.tar.xz"
        if not target.is_file():
            pack([d, *deps], target)
        png = out / f"icons-{name}.png"
        img = icon_preview(d)
        if img is not None and not png.is_file():
            img.save(str(png), "PNG")
        entries.append({"name": name, "farbe": name.replace("-", " "), "datei": target.name,
                        "groesse": target.stat().st_size, "sha256": sha256(target),
                        "vorschau": png.name if png.is_file() else "", "enthaelt": [name, *needs],
                        "links": True, "lizenz": "GPL-3.0", "quelle": f"https://github.com/{repo}"})
        print(f"{target.name}  {target.stat().st_size // 1024} KB")
    man_path = out / "manifest.json"
    man = json.loads(man_path.read_text(encoding="utf-8")) if man_path.is_file() else {"format": 1, "kategorien": {}}
    man["kategorien"]["icons"] = {"lizenz": "GPL-3.0 (je Theme, siehe LICENSE im Archiv)", "eintraege": entries, "pakete": []}
    man_path.write_text(json.dumps(man, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print("manifest.json: icons =", len(entries))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
