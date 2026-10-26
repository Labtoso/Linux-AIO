#!/bin/sh
# Linux AIO deinstallieren (Programm selbst; installierte Desktops und Themes bleiben)
set -e
cd /tmp   # root darf in Cloud-Ordnern nicht lesen

if command -v pacman >/dev/null 2>&1 && pacman -Qq labtoso-linuxaio >/dev/null 2>&1; then
  echo "==> Entferne pacman-Paket labtoso-linuxaio"
  exec pkexec pacman -R --noconfirm labtoso-linuxaio
fi

if [ "$(id -u)" -ne 0 ]; then
  T="$(mktemp /tmp/linuxaio-uninstall.XXXXXX)"; cp "$0" "$T"; chmod 755 "$T"
  exec pkexec sh "$T" "$@"
fi

echo "==> Entferne Programmdateien aus /usr"
rm -f /usr/bin/linux-aio
rm -rf /usr/lib/labtoso-linuxaio
rm -f /usr/share/applications/de.labtosostudios.linuxaio.desktop
rm -f /usr/share/polkit-1/actions/de.labtosostudios.linuxaio.policy
rm -f /usr/share/icons/hicolor/scalable/apps/de.labtosostudios.linuxaio.svg
for s in 16 32 48 64 128 256; do
  rm -f "/usr/share/icons/hicolor/${s}x${s}/apps/de.labtosostudios.linuxaio.png"
done
gtk-update-icon-cache -q /usr/share/icons/hicolor 2>/dev/null || true
echo "==> Fertig. Einstellungen und Status bleiben in /var/lib/labtoso-linuxaio und ~/.config/LabtosoStudios"
