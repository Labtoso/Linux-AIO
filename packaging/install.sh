#!/bin/sh
# Linux AIO systemweit installieren (Arch: als pacman-Paket, sonst nach /usr)
# root darf in Cloud-Ordnern (FUSE) nicht lesen → alles erst nach /tmp kopieren und dort arbeiten
set -e
if [ -z "$LA_WORK" ]; then
  HERE="$(cd "$(dirname "$0")" && pwd)"
  LA_WORK="$(mktemp -d /tmp/linuxaio-install.XXXXXX)"
  TARBALL="$(ls "$HERE"/labtoso-linuxaio-*.tar.gz | sort -V | tail -n1)"
  cp "$HERE/PKGBUILD" "$TARBALL" "$0" "$LA_WORK/"
  chmod 755 "$LA_WORK"
  export LA_WORK
fi
cd "$LA_WORK"
TARBALL="$(ls "$LA_WORK"/labtoso-linuxaio-*.tar.gz | sort -V | tail -n1)"

need_qt() {
  python3 -c "import PySide6" 2>/dev/null || python3 -c "import PyQt6" 2>/dev/null
}

if command -v pacman >/dev/null 2>&1 && command -v makepkg >/dev/null 2>&1 && [ "$(id -u)" -ne 0 ]; then
  echo "==> Arch erkannt: baue pacman-Paket"
  makepkg -sf --noconfirm
  PKG="$(ls "$LA_WORK"/labtoso-linuxaio-*.pkg.tar.zst | sort -V | tail -n1)"
  pkexec pacman -U --noconfirm "$PKG"
  echo "==> Fertig: Linux AIO steht im Startmenü (System)"
  exit 0
fi

if [ "$(id -u)" -ne 0 ]; then
  exec pkexec env LA_WORK="$LA_WORK" sh "$LA_WORK/install.sh" "$@"
fi

echo "==> Installiere nach /usr"
if ! need_qt; then
  if command -v apt-get >/dev/null; then apt-get install -y python3-pyqt6 || apt-get install -y python3-pyside6.qtwidgets
  elif command -v dnf >/dev/null; then dnf install -y python3-pyside6
  elif command -v zypper >/dev/null; then zypper -n in python3-pyside6
  elif command -v pacman >/dev/null; then pacman -S --needed --noconfirm pyside6
  fi
fi
TMP="$(mktemp -d)"
tar -xzf "$TARBALL" -C "$TMP"
SRC="$(ls -d "$TMP"/labtoso-linuxaio-* | sort -V | tail -n1)"
install -d /usr/lib/labtoso-linuxaio
cp -r "$SRC/src/linuxaio" "$SRC/src/hyprstudio" /usr/lib/labtoso-linuxaio/
install -Dm755 "$SRC/packaging/linux-aio" /usr/bin/linux-aio
install -Dm755 "$SRC/packaging/linuxaio-helper" /usr/lib/labtoso-linuxaio/linuxaio-helper
install -Dm644 "$SRC/packaging/de.labtosostudios.linuxaio.desktop" /usr/share/applications/de.labtosostudios.linuxaio.desktop
install -Dm644 "$SRC/packaging/de.labtosostudios.linuxaio.policy" /usr/share/polkit-1/actions/de.labtosostudios.linuxaio.policy
install -Dm644 "$SRC/src/linuxaio/assets/linux-aio.svg" /usr/share/icons/hicolor/scalable/apps/de.labtosostudios.linuxaio.svg
for s in 16 32 48 64 128 256; do
  install -Dm644 "$SRC/packaging/icons/$s.png" "/usr/share/icons/hicolor/${s}x${s}/apps/de.labtosostudios.linuxaio.png"
done
install -dm755 /var/lib/labtoso-linuxaio
gtk-update-icon-cache -q /usr/share/icons/hicolor 2>/dev/null || true
command -v kbuildsycoca6 >/dev/null && su "$(logname 2>/dev/null || echo root)" -c kbuildsycoca6 >/dev/null 2>&1 || true
echo "==> Fertig: Linux AIO steht im Startmenü (System)"
