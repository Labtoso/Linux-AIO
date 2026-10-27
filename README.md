# Linux AIO

Linux settings in one place: install and switch desktop environments, SDDM login themes, Plymouth boot splash and settings for GNOME, KDE Plasma, Hyprland and more. Dark design, English by default – **German available** (gear menu › language). By LabtosoStudios, Python + PySide6.

## Features

- **Desktops:** install GNOME, Plasma, Hyprland, Xfce, Cinnamon and more, set the default, remove
- **SDDM / Plymouth:** apply, test and install themes from the KDE Store; full animated Plymouth previews
- **Settings:** all desktops on the left (the running one is preselected), matching options on the right
  - **Cursors and icons** for every desktop: tiles with previews, downloaded from this repository (checksums verified) or from your own folder – 29 Material Bibata cursors and 20 icon themes (Papirus, Numix, Kora, Flat-Remix, Candy, MoreWaita, Gruvbox Plus, Zafiro, Obsidian)
  - **GNOME:** GNOME Tweaks options, title bar buttons and button themes (GTK and Firefox/Zen), window corner radius, top bar editor and power menu, fonts from Google Fonts and Nerd Fonts, startup apps, login screen (GDM), missing extensions with one click
  - **Plasma:** colour scheme and Plasma style
  - **Hyprland:** the complete [Hypr Studio](https://github.com/Labtoso/HyprStudio), embedded or as its own window
- **Advanced:** install yay with one click, configure Kitty and Alacritty (presets, font, padding, opacity, cursor)
- **Search:** one search field for pages, settings and themes (Ctrl+F)
- **App:** interface size 100 / 90 / 80 %

## Install

Download `Linux-AIO-<version>.zip` from the [releases](https://github.com/Labtoso/Linux-AIO/releases), unzip it and run:

```bash
sh Install.sh
```

It installs everything that is needed (Python 3, PySide6, polkit – via pacman, apt, dnf or zypper), puts the app into `~/.local/bin/linux-aio` and adds a start menu entry. Remove it with `sh Uninstall.sh`; your settings and installed cursors stay if you want.

The AppImage also runs on its own (it registers its start menu entry on first start).

## Build

```bash
python3 build.py
python3 -m unittest discover -s tests
```

Test mode without changing the system: `DEMANAGER_DRYRUN=1 python3 -m linuxaio` (inside `src`).

## Licence

Linux AIO is free software under the **GNU General Public License 3.0** (see `LICENSE`).

- Cursors in the releases: **Bibata / Material Bibata Cursor**, GPL-3.0 (https://github.com/SakibShahariar/material-bibata-cursor).
- Icon themes in the releases: each under GPL-3.0 from its authors (Papirus, Numix, Kora, Flat-Remix, Candy, MoreWaita, Gruvbox Plus, Zafiro, Obsidian); the licence file is included in every archive.
- Fonts are downloaded directly from Google Fonts and Nerd Fonts under their own licences.
- The interface uses Qt for Python (PySide6) under the LGPL-3.0.
