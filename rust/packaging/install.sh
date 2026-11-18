#!/bin/sh
# Programm systemweit installieren (Arch: als pacman-Paket, sonst nach /usr).
# Welches Programm, steht in app.conf daneben (Linux AIO, SDDM Studio oder Hypr Studio).
# root darf in Cloud-Ordnern (FUSE) nicht lesen → alles erst nach /tmp kopieren und dort arbeiten.
set -e
if [ -z "$LA_WORK" ]; then
  HERE="$(cd "$(dirname "$0")" && pwd)"
  LA_WORK="$(mktemp -d /tmp/labtoso-install.XXXXXX)"
  cp -r "$HERE"/. "$LA_WORK/"
  chmod -R a+rX "$LA_WORK"
  export LA_WORK
fi
cd "$LA_WORK"
. ./app.conf

as_root() {
  if command -v pkexec >/dev/null 2>&1; then pkexec env LA_WORK="$LA_WORK" "$@"
  else sudo env LA_WORK="$LA_WORK" "$@"
  fi
}

# nach der Installation als normaler Benutzer (Hypr Studio: Laptop-Tasten sofort aktiv)
after() {
  if [ "$BIN" = hypr-studio ]; then /usr/bin/hypr-studio --laptop-keys || true; fi
  echo "==> Fertig: $NAME steht im Startmenü"
}

if [ "$1" != "--root" ]; then
  if command -v pacman >/dev/null 2>&1 && command -v makepkg >/dev/null 2>&1 && [ "$(id -u)" -ne 0 ]; then
    echo "==> Arch erkannt: baue pacman-Paket"
    makepkg -sf --noconfirm
    PKGFILE="$(ls "$LA_WORK"/"$PKG"-*.pkg.tar.zst | sort -V | tail -n1)"
    as_root pacman -U --noconfirm "$PKGFILE"
    after
    exit 0
  fi
  if [ "$(id -u)" -ne 0 ]; then
    as_root sh "$LA_WORK/Install.sh" --root
    after
    exit 0
  fi
fi

echo "==> Installiere $NAME nach /usr"
# Laufzeit: GTK 4 und polkit (pkexec)
if command -v apt-get >/dev/null; then apt-get install -y libgtk-4-1 && (apt-get install -y pkexec || apt-get install -y policykit-1 || true)
elif command -v dnf >/dev/null; then dnf install -y gtk4 polkit
elif command -v zypper >/dev/null; then zypper -n in libgtk-4-1 polkit
elif command -v pacman >/dev/null; then pacman -S --needed --noconfirm gtk4 polkit
fi
if [ "$BIN" = hypr-studio ]; then   # Helligkeits- und Medientasten
  if command -v apt-get >/dev/null; then apt-get install -y brightnessctl playerctl || true
  elif command -v dnf >/dev/null; then dnf install -y brightnessctl playerctl || true
  elif command -v zypper >/dev/null; then zypper -n in brightnessctl playerctl || true
  elif command -v pacman >/dev/null; then pacman -S --needed --noconfirm brightnessctl playerctl || true
  fi
fi
sh ./files.sh ""
gtk-update-icon-cache -q /usr/share/icons/hicolor 2>/dev/null || true
U="$(id -nu "${PKEXEC_UID:-${SUDO_UID:-0}}" 2>/dev/null || echo root)"
command -v kbuildsycoca6 >/dev/null && su "$U" -c kbuildsycoca6 >/dev/null 2>&1 || true
