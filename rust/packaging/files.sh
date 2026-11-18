#!/bin/sh
# Dateien an ihren Platz legen. $1 = Wurzel (leer = System, beim Arch-Paket "$pkgdir").
set -e
D="$1"
cd "$(dirname "$0")"
. ./app.conf
install -Dm755 "$BIN" "$D/usr/bin/$BIN"
for x in $EXTRA; do install -Dm755 "$x" "$D/usr/bin/$x"; done
if [ -n "$HELPER" ]; then
  install -Dm755 helper "$D$HELPER"
  install -Dm644 "$ID.policy" "$D/usr/share/polkit-1/actions/$ID.policy"
  install -dm755 "$D/var/lib/labtoso-linuxaio"
fi
install -Dm644 "$ID.desktop" "$D/usr/share/applications/$ID.desktop"
install -Dm644 "icons/$ID.svg" "$D/usr/share/icons/hicolor/scalable/apps/$ID.svg"
for s in 16 32 48 64 128 256; do
  [ -f "icons/$s.png" ] && install -Dm644 "icons/$s.png" "$D/usr/share/icons/hicolor/${s}x${s}/apps/$ID.png"
done
install -Dm644 LICENSE "$D/usr/share/licenses/$PKG/LICENSE"
