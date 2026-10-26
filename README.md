# Linux AIO

Linux-Einstellungen an einem Ort, per Klick: Desktop-Umgebungen installieren und wechseln, SDDM-Login-Themes, Plymouth-Bootsplash und Einstellungen für GNOME, KDE Plasma, Hyprland und weitere Desktops. Dunkles Design, Deutsch/Englisch. Von LabtosoStudios, Python + PySide6.

## Was es kann

- **Desktops:** GNOME, Plasma, Hyprland, Xfce, Cinnamon und mehr installieren, als Standard setzen, entfernen
- **SDDM / Plymouth:** Themes setzen, testen und aus dem KDE Store installieren
- **Einstellungen:** links alle Desktops (der laufende ist vorgewählt), rechts die passenden Optionen
  - **Mauszeiger und Symbole** für jeden Desktop: Kacheln mit Vorschau, aus diesem Repo herunterladen (Prüfsumme wird kontrolliert) oder aus einem eigenen Ordner, „Installieren“ und „Anwenden“
  - **GNOME:** die Optionen von GNOME Tweaks, Titelleisten-Knöpfe und Knopf-Themes, obere Leiste, Startprogramme, Anmeldebildschirm (GDM), fehlende Erweiterungen per Klick
  - **Plasma:** Farbschema und Plasma-Design
  - **Hyprland:** das komplette [Hypr Studio](https://github.com/Labtoso/HyprStudio) ist eingebaut

## Installation

Aus den [Releases](https://github.com/Labtoso/Linux-AIO/releases) `Linux-AIO-<Version>.zip` laden, entpacken und im Terminal ausführen:

```bash
sh Install.sh
```

Das installiert die AppImage nach `~/.local/bin/linux-aio`, einen Startmenü-Eintrag und das Icon (fehlt PySide6, wird es mit installiert). Entfernen mit `sh Uninstall.sh`; Einstellungen und installierte Mauszeiger bleiben, wenn du willst.

Alternativ als Arch-Paket: `installer/Installieren.sh` (baut mit `makepkg` und installiert mit `pacman`).

Braucht: Python 3, PySide6, polkit (für Systemänderungen wie Desktops oder Login-Themes).

## Selbst bauen

```bash
python3 build.py
python3 -m unittest discover -s tests
```

Testmodus ohne Änderungen am System: `DEMANAGER_DRYRUN=1 python3 -m linuxaio` (im Ordner `src`).

## Lizenz

Linux AIO ist freie Software unter der **GNU General Public License 3.0** (siehe `LICENSE`).

- Die Mauszeiger in den Releases sind **Bibata / Material Bibata Cursor** und stehen ebenfalls unter **GPL-3.0** (Quelle: https://github.com/SakibShahariar/material-bibata-cursor, ursprünglich https://github.com/ful1e5/Bibata_Cursor).
- Die Oberfläche nutzt Qt for Python (PySide6) unter der LGPL-3.0.
