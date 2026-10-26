#!/bin/sh
# One-time setup for a self-contained AppImage: portable Python (python-build-standalone) + PySide6 wheels
# from PyPI in ~/.cache/LabtosoStudios/linuxaio/appimage/base. build.py bundles it when it exists.
set -e
B="${XDG_CACHE_HOME:-$HOME/.cache}/LabtosoStudios/linuxaio/appimage"
mkdir -p "$B"
cd "$B"
if [ ! -s py.tar.gz ]; then
  URL="$(curl -fsS https://api.github.com/repos/astral-sh/python-build-standalone/releases/latest | python3 -c "import json,sys; r=json.load(sys.stdin); print(next(a['browser_download_url'] for a in r['assets'] if a['name'].startswith('cpython-3.13.') and a['name'].endswith('x86_64-unknown-linux-gnu-install_only_stripped.tar.gz')))")"
  curl -fL -o py.tar.gz "$URL"
fi
rm -rf base
mkdir base
tar -xzf py.tar.gz -C base
base/python/bin/python3 -m pip install --no-warn-script-location --disable-pip-version-check PySide6-Essentials
base/python/bin/python3 -c "import PySide6.QtWidgets, PySide6.QtSvg, PySide6.QtNetwork; print('ok: PySide6', __import__('PySide6').__version__)"
