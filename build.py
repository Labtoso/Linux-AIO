"""Baut alles: Icons, portable .pyz, Quell-Tarball fürs Arch-Paket, AppImage, Release-ZIP und den installer/-Ordner.

Start:  python3 build.py

Regeln (Sebastian, 2026-10-26):
- Vorherige Version wird vorher nach old/<version>/ verschoben (nur lokal, nie ins Repo/Release)
- Jede Version: dist/Linux-AIO-<version>.zip mit Install.sh, Uninstall.sh, AppImage, README.md, LICENSE
- AppImage mit appimagetool (PATH oder ~/.local/bin); fehlt es, wird die AppImage übersprungen
"""

import io
import os
import shutil
import subprocess
import sys
import tarfile
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent
SRC = ROOT / "src"
sys.path.insert(0, str(SRC))
import linuxaio  # noqa: E402

VER = linuxaio.__version__
NAME = f"labtoso-linuxaio-{VER}"
INST = ROOT / "installer"
DIST = ROOT / "dist"
OLD = ROOT / "old"
# AppDir auf der lokalen Platte: Cloud-Ordner (pCloud) speichern kein Ausführen-Recht → AppRun liefe nicht
BUILD = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "LabtosoStudios" / "linuxaio" / "build"
SIZES = (16, 32, 48, 64, 128, 256)
APPIMAGE = f"Linux-AIO-{VER}-x86_64.AppImage"
ZIPNAME = f"Linux-AIO-{VER}.zip"
PACKAGES = ("linuxaio", "hyprstudio")   # Hypr Studio steckt komplett drin (Einstellungen › Hyprland)


def lf(data: bytes) -> bytes:
    return data.replace(b"\r\n", b"\n")


def write_lf(path: Path, text: str, mode: int = 0o644) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(lf(text.encode("utf-8")))
    os.chmod(path, mode)


# ---------- alte Version wegräumen ----------

def previous_version() -> str | None:
    f = DIST / "VERSION"
    if f.is_file():
        v = f.read_text(encoding="utf-8").strip()
        return v if v and v != VER else None
    for tb in sorted(DIST.glob("labtoso-linuxaio-*.tar.gz")):
        v = tb.name.removeprefix("labtoso-linuxaio-").removesuffix(".tar.gz")
        if v != VER:
            return v
    return None


def archive_previous() -> None:
    """dist/ und installer/ der vorigen Version nach old/<version>/ verschieben (nichts wird gelöscht)."""
    prev = previous_version()
    if not prev:
        return
    target = OLD / prev
    n = 1
    while target.exists():
        target = OLD / f"{prev}-{n}"
        n += 1
    for folder in (DIST, INST):
        if folder.is_dir() and any(folder.iterdir()):
            dest = target / folder.name
            dest.mkdir(parents=True, exist_ok=True)
            for item in folder.iterdir():
                shutil.move(str(item), str(dest / item.name))
    print(f"Vorige Version {prev} → {target.relative_to(ROOT)}")


# ---------- Bausteine ----------

def render_icons() -> None:
    from PySide6 import QtCore, QtGui, QtSvg, QtWidgets  # noqa: F401
    _ = QtWidgets.QApplication.instance() or QtWidgets.QApplication(["build"])
    r = QtSvg.QSvgRenderer(str(SRC / "linuxaio/assets/linux-aio.svg"))
    out = ROOT / "packaging/icons"
    out.mkdir(parents=True, exist_ok=True)
    for s in SIZES:
        img = QtGui.QImage(s, s, QtGui.QImage.Format.Format_ARGB32)
        img.fill(0)
        p = QtGui.QPainter(img)
        p.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
        r.render(p, QtCore.QRectF(0, 0, s, s))
        p.end()
        img.save(str(out / f"{s}.png"))
    print("Icons:", ", ".join(map(str, SIZES)))


def src_files():
    for pkg in PACKAGES:
        for f in sorted((SRC / pkg).rglob("*")):
            if f.is_file() and "__pycache__" not in f.parts:
                yield f


def build_pyz() -> Path:
    DIST.mkdir(exist_ok=True)
    target = DIST / "Linux-AIO.pyz"
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w", zipfile.ZIP_DEFLATED) as z:
        for f in src_files():
            data = f.read_bytes()
            if f.suffix == ".py":
                data = lf(data)
            z.writestr(str(f.relative_to(SRC)).replace("\\", "/"), data)
        z.writestr("__main__.py", "import sys\nfrom linuxaio.__main__ import main\nsys.exit(main())\n")
    with open(target, "wb") as fh:
        fh.write(b"#!/usr/bin/env python3\n")
        fh.write(buf.getvalue())
    os.chmod(target, 0o755)
    print("Portable:", target.name, f"{target.stat().st_size // 1024} KB")
    return target


def build_tarball() -> Path:
    DIST.mkdir(exist_ok=True)
    target = DIST / f"{NAME}.tar.gz"
    files = list(src_files()) + [f for f in sorted((ROOT / "packaging").rglob("*")) if f.is_file()]
    files += [ROOT / "LICENSE", ROOT / "README.md"]
    with tarfile.open(target, "w:gz") as tar:
        for f in files:
            rel = f.relative_to(ROOT).as_posix()
            data = f.read_bytes()
            if f.suffix in (".py", ".sh", ".desktop", ".policy", ".md", "") or f.name == "PKGBUILD":
                data = lf(data)
            ti = tarfile.TarInfo(f"{NAME}/{rel}")
            ti.size = len(data)
            ti.mode = 0o755 if f.name in ("linux-aio", "linuxaio-helper", "install.sh", "uninstall.sh") else 0o644
            ti.mtime = int(f.stat().st_mtime)
            tar.addfile(ti, io.BytesIO(data))
    print("Quellen:", target.name)
    return target


def find_appimagetool() -> str | None:
    for c in (shutil.which("appimagetool"), str(Path.home() / ".local/bin/appimagetool")):
        if c and Path(c).is_file() and os.access(c, os.X_OK):
            return c
    return None


def build_appimage(pyz: Path) -> Path | None:
    tool = find_appimagetool()
    if not tool:
        print("AppImage übersprungen: appimagetool fehlt (nach ~/.local/bin legen)")
        return None
    appdir = BUILD / "AppDir"
    if appdir.exists():
        shutil.rmtree(appdir)       # eigener Bau-Ordner, wird jedes Mal neu erzeugt
    (appdir / "usr/lib/linux-aio").mkdir(parents=True)
    shutil.copy2(pyz, appdir / "usr/lib/linux-aio/Linux-AIO.pyz")
    base = BUILD.parent / "appimage" / "base" / "python"       # tools/appimage_base.sh
    if (base / "bin" / "python3").is_file():
        shutil.copytree(base, appdir / "usr/python", symlinks=True)
        write_lf(appdir / "AppRun", APPRUN_BUNDLED, 0o755)
        print("AppImage: mit eigenem Python + PySide6")
    else:
        write_lf(appdir / "AppRun", APPRUN, 0o755)
        print("AppImage: nutzt Python/PySide6 des Systems (tools/appimage_base.sh fehlt)")
    write_lf(appdir / "linux-aio.desktop", APPIMAGE_DESKTOP)
    shutil.copy2(ROOT / "packaging/icons/256.png", appdir / "linux-aio.png")
    shutil.copy2(ROOT / "packaging/icons/256.png", appdir / ".DirIcon")
    target = DIST / APPIMAGE
    env = dict(os.environ, ARCH="x86_64", APPIMAGE_EXTRACT_AND_RUN="1")
    r = subprocess.run([tool, "--no-appstream", str(appdir), str(target)], env=env, capture_output=True, text=True)
    if r.returncode != 0 or not target.is_file():
        print("AppImage fehlgeschlagen:\n" + (r.stdout + r.stderr)[-2000:])
        return None
    os.chmod(target, 0o755)
    print("AppImage:", target.name, f"{target.stat().st_size // 1024} KB")
    return target


def build_zip(appimage: Path | None, pyz: Path) -> Path:
    target = DIST / ZIPNAME
    with zipfile.ZipFile(target, "w", zipfile.ZIP_DEFLATED) as z:
        def add(name, data, mode=0o644):
            zi = zipfile.ZipInfo(f"Linux-AIO-{VER}/{name}")
            zi.external_attr = (0o100000 | mode) << 16
            zi.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(zi, data)
        bundled = "1" if appimage and (BUILD / "AppDir" / "usr" / "python").is_dir() else "0"
        add("Install.sh", lf(ZIP_INSTALL.replace("@VER@", VER).replace("@BUNDLED@", bundled).encode()), 0o755)
        add("Uninstall.sh", lf(ZIP_UNINSTALL.encode()), 0o755)
        add("README.md", lf((ROOT / "README.md").read_bytes()))
        add("LICENSE", (ROOT / "LICENSE").read_bytes())
        add("linux-aio.png", (ROOT / "packaging/icons/256.png").read_bytes())
        if appimage:
            add(appimage.name, appimage.read_bytes(), 0o755)
        else:
            add(pyz.name, pyz.read_bytes(), 0o755)      # ohne appimagetool: .pyz statt AppImage
    print("ZIP:", target.name, f"{target.stat().st_size // 1024} KB", "" if appimage else "(ohne AppImage)")
    return target


def build_installer(pyz: Path, tarball: Path) -> None:
    (INST / "portable").mkdir(parents=True, exist_ok=True)
    (INST / "arch").mkdir(parents=True, exist_ok=True)
    shutil.copy2(pyz, INST / "portable" / pyz.name)
    shutil.copy2(ROOT / "packaging/icons/256.png", INST / "portable" / "linux-aio.png")
    shutil.copy2(tarball, INST / "arch" / tarball.name)
    write_lf(INST / "arch/PKGBUILD", (ROOT / "packaging/PKGBUILD").read_text(encoding="utf-8"))
    write_lf(INST / "arch/install.sh", (ROOT / "packaging/install.sh").read_text(encoding="utf-8"))
    write_lf(INST / "Installieren.sh", INSTALL_SH)
    write_lf(INST / "Portable.sh", PORTABLE_SH)
    write_lf(INST / "uninstall.sh", (ROOT / "packaging/uninstall.sh").read_text(encoding="utf-8"))
    write_lf(INST / "Linux AIO.desktop", PORTABLE_DESKTOP)
    print("installer/ aktualisiert")


# ---------- Skripte ----------

INSTALL_SH = """#!/bin/sh
# Linux AIO installieren (Startmenü-Eintrag, polkit-Regel, Icon)
cd "$(dirname "$0")" || exit 1
exec sh arch/install.sh "$@"
"""

PORTABLE_SH = """#!/bin/sh
# Linux AIO ohne Installation starten
HERE="$(cd "$(dirname "$0")" && pwd)"
cd /tmp   # pkexec/sudo nie aus Cloud-Ordnern starten (root darf dort nicht lesen)
if ! python3 -c "import PySide6" 2>/dev/null && ! python3 -c "import PyQt6" 2>/dev/null; then
  echo "Qt für Python fehlt – wird installiert …"
  if command -v pacman >/dev/null; then pkexec pacman -S --needed --noconfirm pyside6
  elif command -v apt-get >/dev/null; then pkexec apt-get install -y python3-pyqt6
  elif command -v dnf >/dev/null; then pkexec dnf install -y python3-pyside6
  elif command -v zypper >/dev/null; then pkexec zypper -n in python3-pyside6
  fi
fi
ICON="$HOME/.local/share/icons/hicolor/256x256/apps/de.labtosostudios.linuxaio.png"
[ -f "$ICON" ] || install -Dm644 "$HERE/portable/linux-aio.png" "$ICON" 2>/dev/null || true
exec python3 "$HERE/portable/Linux-AIO.pyz" "$@"
"""

# Doppelklick-Starter für Dolphin (relativ zum Ordner über %k)
PORTABLE_DESKTOP = """[Desktop Entry]
Type=Application
Name=Linux AIO (portabel)
Exec=sh -c 'cd "$(dirname "%k")" && sh Portable.sh'
Icon=de.labtosostudios.linuxaio
Terminal=false
Categories=System;
"""

APPRUN = """#!/bin/sh
# Linux AIO (AppImage): nutzt das Python 3 + PySide6 des Systems
HERE="$(dirname "$(readlink -f "$0")")"
if ! python3 -c "import PySide6" 2>/dev/null && ! python3 -c "import PyQt6" 2>/dev/null; then
  MSG="Linux AIO braucht Python 3 mit PySide6 (Arch: sudo pacman -S pyside6) – oder Install.sh aus der ZIP nutzen."
  command -v notify-send >/dev/null 2>&1 && notify-send "Linux AIO" "$MSG"
  echo "$MSG" >&2
  exit 1
fi
exec python3 "$HERE/usr/lib/linux-aio/Linux-AIO.pyz" "$@"
"""

APPRUN_BUNDLED = """#!/bin/sh
# Linux AIO (AppImage): brings its own Python 3 + PySide6 – nothing needed on the system
HERE="$(dirname "$(readlink -f "$0")")"
export PYTHONNOUSERSITE=1
unset PYTHONPATH PYTHONHOME
exec "$HERE/usr/python/bin/python3" "$HERE/usr/lib/linux-aio/Linux-AIO.pyz" "$@"
"""

APPIMAGE_DESKTOP = """[Desktop Entry]
Type=Application
Name=Linux AIO
Comment=Desktops, SDDM, Plymouth and desktop settings in one place
Exec=linux-aio
Icon=linux-aio
Terminal=false
Categories=System;Settings;
"""

ZIP_INSTALL = """#!/bin/sh
# Linux AIO @VER@ – install for the current user (~/.local). Installs everything it needs:
# Python 3, PySide6 (Qt), polkit, gsettings, fontconfig, xz – via pacman, apt, dnf or zypper.
set -e
HERE="$(cd "$(dirname "$0")" && pwd)"
cd /tmp   # never run pkexec/sudo from cloud folders
BIN="$HOME/.local/bin"
APPS="$HOME/.local/share/applications"
ICON="$HOME/.local/share/icons/hicolor/256x256/apps/de.labtosostudios.linuxaio.png"

root() { if command -v pkexec >/dev/null 2>&1; then pkexec "$@"; else sudo "$@"; fi; }
have_qt() { python3 -c "import PySide6.QtWidgets" 2>/dev/null || python3 -c "import PyQt6.QtWidgets" 2>/dev/null; }

BUNDLED=@BUNDLED@   # 1 = the AppImage brings its own Python + PySide6
if [ "$BUNDLED" = 1 ]; then
  : # nothing to install
elif ! command -v python3 >/dev/null 2>&1 || ! have_qt; then
  echo "==> Installing Python 3 and Qt for Python (password needed)"
  if command -v pacman >/dev/null; then root pacman -S --needed --noconfirm python pyside6 polkit glib2 fontconfig xz
  elif command -v apt-get >/dev/null; then root apt-get update; root apt-get install -y python3 python3-pyside6.qtwidgets python3-pyside6.qtsvg python3-pyside6.qtnetwork policykit-1 libglib2.0-bin fontconfig xz-utils || root apt-get install -y python3 python3-pyqt6 python3-pyqt6.qtsvg pkexec libglib2.0-bin fontconfig xz-utils
  elif command -v dnf >/dev/null; then root dnf install -y python3 python3-pyside6 polkit glib2 fontconfig xz
  elif command -v zypper >/dev/null; then root zypper -n in python3 python3-pyside6 polkit glib2-tools fontconfig xz
  elif command -v python3 >/dev/null 2>&1; then python3 -m pip install --user PySide6 || true
  fi
fi
if [ "$BUNDLED" != 1 ] && ! have_qt; then
  echo "!! Qt for Python could not be installed automatically. Install PySide6 for your system and run this again."
  exit 1
fi

mkdir -p "$BIN" "$APPS"
APP="$(ls "$HERE"/Linux-AIO-*-x86_64.AppImage 2>/dev/null | sort -V | tail -n1)"
if [ -n "$APP" ]; then
  install -m755 "$APP" "$BIN/Linux-AIO.AppImage"
  cat > "$BIN/linux-aio" <<'EOS'
#!/bin/sh
# without FUSE the AppImage runs extracted
command -v fusermount3 >/dev/null 2>&1 || command -v fusermount >/dev/null 2>&1 || export APPIMAGE_EXTRACT_AND_RUN=1
exec "$HOME/.local/bin/Linux-AIO.AppImage" "$@"
EOS
else
  install -m755 "$HERE/Linux-AIO.pyz" "$BIN/Linux-AIO.pyz"
  printf '#!/bin/sh\\nexec python3 "$HOME/.local/bin/Linux-AIO.pyz" "$@"\\n' > "$BIN/linux-aio"
fi
chmod 755 "$BIN/linux-aio"
install -Dm644 "$HERE/linux-aio.png" "$ICON"
cat > "$APPS/de.labtosostudios.linuxaio.desktop" <<EOD
[Desktop Entry]
Type=Application
Name=Linux AIO
Comment=Desktops, SDDM, Plymouth and desktop settings in one place
Comment[de]=Desktops, SDDM, Plymouth und Desktop-Einstellungen an einem Ort
Exec=$BIN/linux-aio
Icon=de.labtosostudios.linuxaio
Terminal=false
Categories=System;Settings;
StartupWMClass=de.labtosostudios.linuxaio
StartupNotify=false
EOD
update-desktop-database "$APPS" >/dev/null 2>&1 || true
gtk-update-icon-cache -q "$HOME/.local/share/icons/hicolor" >/dev/null 2>&1 || true
echo "==> Done: Linux AIO @VER@ is in the app menu (command: linux-aio)"
"""

ZIP_UNINSTALL = """#!/bin/sh
# Remove Linux AIO (app, start menu entry, icon). Settings and cursors only after asking.
cd /tmp
BIN="$HOME/.local/bin"
rm -f "$BIN/linux-aio" "$BIN/Linux-AIO.AppImage" "$BIN/Linux-AIO.pyz"
rm -f "$HOME/.local/share/applications/de.labtosostudios.linuxaio.desktop"
rm -f "$HOME/.local/share/icons/hicolor/"*/apps/de.labtosostudios.linuxaio.png
update-desktop-database "$HOME/.local/share/applications" >/dev/null 2>&1 || true
echo "==> Linux AIO removed"

printf "Also delete the Linux AIO settings? [y/N] "
read -r a
case "$a" in
  y|Y|j|J) rm -f "$HOME/.config/LabtosoStudios/linuxaio.json"; rm -rf "$HOME/.cache/LabtosoStudios/linuxaio"
           echo "    settings deleted" ;;
  *) echo "    settings kept" ;;
esac
printf "Also delete the installed Bibata cursors (~/.local/share/icons/Bibata-Material-*)? [y/N] "
read -r b
case "$b" in
  y|Y|j|J) rm -rf "$HOME"/.local/share/icons/Bibata-Material-*; echo "    cursors deleted" ;;
  *) echo "    cursors kept" ;;
esac
"""


if __name__ == "__main__":
    archive_previous()
    render_icons()
    pyz = build_pyz()
    tb = build_tarball()
    app = build_appimage(pyz)
    build_zip(app, pyz)
    build_installer(pyz, tb)
    (DIST / "VERSION").write_text(VER + "\n", encoding="utf-8")
