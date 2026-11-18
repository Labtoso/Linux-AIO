#!/bin/sh
# Release-ZIP einer App bauen:  sh package.sh <linux-aio|sddm-studio|hypr-studio> <Programmdatei> <Version> <Zielordner>
# Ergebnis: <Ordner>/<Name>.zip (fester Name für den Download auf der Website) mit Install.sh, Uninstall.sh,
# dem Programm, Startmenü-Eintrag, Symbolen, polkit-Regel und PKGBUILD (Arch baut daraus ein pacman-Paket).
set -e
APP="$1"; EXE="$2"; VER="$3"; OUT="$4"
P="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$P/../.." && pwd)"
. "$P/$APP/app.conf"
case "$APP" in
  linux-aio) FOLDER="Linux-AIO"; SVG="$ROOT/rust/crates/linuxaio/data/linux-aio.svg"; LIC="$ROOT/LICENSE"; L="GPL-3.0-only" ;;
  sddm-studio) FOLDER="SDDM-Studio"; SVG="$ROOT/rust/crates/linuxaio/data/sddm-studio.svg"; LIC="$ROOT/LICENSE"; L="GPL-3.0-only" ;;
  hypr-studio) FOLDER="Hypr-Studio"; SVG="$ROOT/rust/crates/hyprstudio/data/hyprstudio.svg"; LIC="$P/hypr-studio/LICENSE"; L="MIT" ;;
  *) echo "unbekannte App: $APP"; exit 1 ;;
esac
STAGE="$(mktemp -d)"
S="$STAGE/$FOLDER"
mkdir -p "$S/icons"
install -m755 "$EXE" "$S/$BIN"
cp "$P/$APP/app.conf" "$P/$APP/$ID.desktop" "$S/"
[ -f "$P/$APP/$ID.policy" ] && cp "$P/$APP/$ID.policy" "$S/"
install -m755 "$P/install.sh" "$S/Install.sh"
install -m755 "$P/uninstall.sh" "$S/Uninstall.sh"
install -m644 "$P/files.sh" "$S/files.sh"
cp "$LIC" "$S/LICENSE"
if [ -n "$HELPER" ]; then
  printf '#!/bin/sh\n# Root-Helfer von %s (wird über pkexec mit eigener polkit-Aktion gestartet)\nexec /usr/bin/%s --helper "$@"\n' "$NAME" "$BIN" > "$S/helper"
  chmod 755 "$S/helper"
fi
for x in $EXTRA; do   # Hypr Studio: L Launcher
  printf '#!/bin/sh\nexec /usr/bin/%s --launcher "$@"\n' "$BIN" > "$S/$x"
  chmod 755 "$S/$x"
done
cp "$SVG" "$S/icons/$ID.svg"
if command -v rsvg-convert >/dev/null; then
  for s in 16 32 48 64 128 256; do rsvg-convert -w $s -h $s "$SVG" -o "$S/icons/$s.png"; done
fi
# Arch: pacman-Paket aus genau diesen Dateien
{
  echo "# Maintainer: LabtosoStudios"
  echo "pkgname=$PKG"
  echo "pkgver=$VER"
  echo "pkgrel=1"
  echo "pkgdesc=\"$DESC\""
  echo "arch=('x86_64')"
  echo "url=\"https://linux.labtoso.de\""
  echo "license=('$L')"
  if [ -n "$HELPER" ]; then echo "depends=('gtk4' 'polkit')"; else echo "depends=('gtk4')"; fi
  printf "optdepends=("
  echo "$OPT" | tr '|' '\n' | while IFS= read -r o; do [ -n "$o" ] && printf "'%s' " "$o"; done
  echo ")"
  [ "$BIN" = hypr-studio ] && echo "conflicts=('labtoso-hyprstudio-git')"
  echo "options=('!strip' '!debug')"
  echo ""
  echo "package() {"
  echo "  sh \"\$startdir/files.sh\" \"\$pkgdir\""
  echo "}"
} > "$S/PKGBUILD"
cat > "$S/README.md" <<EOT
# $NAME $VER

$DESC

## Install

\`\`\`bash
sh Install.sh
\`\`\`

Installs $NAME to /usr (on Arch as the pacman package \`$PKG\`) and adds a start menu entry.
Remove it with \`sh Uninstall.sh\` – your settings stay.

Website: https://linux.labtoso.de · Source: https://github.com/Labtoso/Linux-AIO
EOT
mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"
rm -f "$OUT/$FOLDER.zip"
(cd "$STAGE" && zip -qr "$OUT/$FOLDER.zip" "$FOLDER")
rm -rf "$STAGE"
echo "$OUT/$FOLDER.zip"
