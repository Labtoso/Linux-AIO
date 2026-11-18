#!/bin/sh
# Programm deinstallieren (nur das Programm; installierte Desktops, Themes und Einstellungen bleiben)
set -e
HERE="$(cd "$(dirname "$0")" && pwd)"
. "$HERE/app.conf"
cd /tmp   # root darf in Cloud-Ordnern nicht lesen

as_root() {
  if command -v pkexec >/dev/null 2>&1; then pkexec "$@"; else sudo "$@"; fi
}

if command -v pacman >/dev/null 2>&1 && pacman -Qq "$PKG" >/dev/null 2>&1; then
  echo "==> Entferne pacman-Paket $PKG"
  as_root pacman -R --noconfirm "$PKG"
  exit 0
fi

if [ "$(id -u)" -ne 0 ]; then
  T="$(mktemp -d /tmp/labtoso-uninstall.XXXXXX)"; cp "$0" "$HERE/app.conf" "$T/"; chmod -R a+rX "$T"
  as_root sh "$T/$(basename "$0")" "$@"
  exit $?
fi

echo "==> Entferne $NAME aus /usr"
rm -f "/usr/bin/$BIN"
for x in $EXTRA; do rm -f "/usr/bin/$x"; done
if [ -n "$HELPER" ]; then
  rm -f "$HELPER"; rmdir "$(dirname "$HELPER")" 2>/dev/null || true
  rm -f "/usr/share/polkit-1/actions/$ID.policy"
fi
rm -f "/usr/share/applications/$ID.desktop"
rm -f "/usr/share/icons/hicolor/scalable/apps/$ID.svg"
for s in 16 32 48 64 128 256; do rm -f "/usr/share/icons/hicolor/${s}x${s}/apps/$ID.png"; done
rm -rf "/usr/share/licenses/$PKG"
gtk-update-icon-cache -q /usr/share/icons/hicolor 2>/dev/null || true
echo "==> Fertig. Einstellungen bleiben in ~/.config/LabtosoStudios"
