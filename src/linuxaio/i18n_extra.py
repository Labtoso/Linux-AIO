"""Texte für Linux AIO 2.0 (Tab „Einstellungen“), werden in i18n.TEXT eingemischt."""

_TITLE_ACTIONS_DE = {
    "toggle-maximize": "Maximieren umschalten", "toggle-maximize-horizontally": "Waagerecht maximieren",
    "toggle-maximize-vertically": "Senkrecht maximieren", "minimize": "Minimieren", "lower": "Nach hinten",
    "menu": "Menü", "none": "Nichts",
}
_TITLE_ACTIONS_EN = {
    "toggle-maximize": "Toggle maximise", "toggle-maximize-horizontally": "Maximise horizontally",
    "toggle-maximize-vertically": "Maximise vertically", "minimize": "Minimise", "lower": "Lower",
    "menu": "Menu", "none": "Nothing",
}
_ACCENTS_DE = {"blue": "Blau", "teal": "Türkis", "green": "Grün", "yellow": "Gelb", "orange": "Orange",
               "red": "Rot", "pink": "Pink", "purple": "Lila", "slate": "Schiefer"}
_ACCENTS_EN = {"blue": "Blue", "teal": "Teal", "green": "Green", "yellow": "Yellow", "orange": "Orange",
               "red": "Red", "pink": "Pink", "purple": "Purple", "slate": "Slate"}


def _expand(prefix: str, mapping: dict) -> dict:
    return {f"{prefix}.{k}": v for k, v in mapping.items()}


EXTRA = {
    "de": {
        "tab.settings": "Einstellungen",
        "src.store": "Installieren", "sddm.download": "Installieren",
        "apply": "Anwenden", "cancel": "Abbrechen", "na": "Auf diesem System nicht verfügbar",
        "set.desktops": "Desktops", "set.none": "Kein Desktop installiert",
        "set.running": "Läuft gerade", "set.notinstalled": "Nicht installiert",
        "sec.look": "Erscheinungsbild", "sec.cursor": "Mauszeiger", "sec.icon": "Symbole", "sec.fonts": "Schriften",
        "sec.windows": "Fenster", "sec.titlebar": "Titelleiste", "sec.topbar": "Obere Leiste",
        "sec.input": "Maus & Tastatur", "sec.autostart": "Startprogramme", "sec.login": "Anmeldebildschirm",
        "sec.plasma": "Erscheinungsbild", "sec.hyprstudio": "Hypr Studio",
        "look.cursor": "Mauszeiger", "look.icon": "Symbole", "look.folder": "Ordner …", "look.reload": "Neu laden",
        "look.source": "Quelle", "look.nofolder": "Ordner nicht gefunden – nur installierte Themes",
        "look.nothemes": "Keine Themes im Ordner – nur installierte", "look.active": "Aktiv",
        "look.unknown": "Aktives Theme unbekannt", "look.installing": "{name} wird installiert …",
        "look.installed": "{name} installiert",
        "gn.look": "Erscheinungsbild", "gn.look.style": "Stil", "gn.look.themes": "Themes",
        "gn.gtk": "Ältere Anwendungen (GTK-Theme)", "gn.shell": "Shell-Theme",
        "gn.shell.tip": "Braucht die Erweiterung „User Themes“", "gn.default": "Standard",
        "gn.ext.on": "Erweiterung an", "gn.ext.off": "Erweiterung aus", "gn.ext.missing": "Erweiterung fehlt",
        "gn.ext.name": "Erweiterung",
        "gn.fonts": "Schriften", "gn.windows": "Fenster", "gn.input": "Maus & Tastatur",
        "gn.titlebar": "Titelleiste", "gn.title.window": "Fenster", "gn.btn.min": "Minimieren",
        "gn.btn.max": "Maximieren", "gn.btn.side": "Knöpfe", "gn.left": "Links", "gn.right": "Rechts",
        "gn.topbar": "Obere Leiste", "gn.top.clock": "Uhr", "gn.top.bar": "Leiste", "gn.top.ws": "Arbeitsflächen",
        "gn.top.wsind": "Arbeitsflächen-Anzeige",
        "gn.top.wsind.tip": "Erweiterung „Workspace Indicator“ zeigt die Arbeitsfläche in der Leiste",
        "gn.autostart": "Startprogramme", "gn.auto.add": "Programm hinzufügen", "gn.auto.none": "Keine Startprogramme",
        "gn.login": "Anmeldebildschirm", "gn.login.bg": "Hintergrund", "gn.login.pick": "Bild wählen …",
        "gn.login.color": "Farbe …", "gn.login.restore": "Original",
        "gn.login.restore.tip": "Original-Design des Anmeldebildschirms zurückholen",
        "gn.login.bg.tip": "Baut gnome-shell-theme.gresource neu (Original wird gesichert). Nach GNOME-Updates neu setzen.",
        "gn.login.logo": "Logo", "gn.login.more": "Mehr", "gn.login.banner": "Text auf dem Anmeldebildschirm",
        "gn.login.banner.ph": "leer = kein Text", "gn.login.users": "Benutzerliste zeigen",
        "gn.login.nogdm": "GDM fehlt",
        "pl.look": "Erscheinungsbild", "pl.scheme": "Farbschema", "pl.desktop": "Plasma-Design",
        "font.ui": "Oberfläche", "font.doc": "Dokumente", "font.mono": "Festbreite", "font.title": "Titelleiste",
        "font.hint": "Hinting", "font.hint.none": "Keins", "font.hint.slight": "Leicht", "font.hint.medium": "Mittel",
        "font.hint.full": "Voll", "font.aa": "Kantenglättung", "font.aa.none": "Keine",
        "font.aa.grayscale": "Graustufen", "font.aa.rgba": "Subpixel (LCD)", "font.scale": "Skalierung",
        "look.scheme": "Stil", "look.scheme.default": "Standard", "look.scheme.prefer-dark": "Dunkel",
        "look.scheme.prefer-light": "Hell", "look.accent": "Akzentfarbe", "look.anim": "Animationen",
        "win.dbl": "Doppelklick", "win.mid": "Mittelklick", "win.right": "Rechtsklick",
        "win.focus": "Fokus", "win.focus.click": "Klick", "win.focus.sloppy": "Maus darüber",
        "win.focus.mouse": "Maus (streng)", "win.raise": "Beim Klick nach vorne",
        "win.modifier": "Taste zum Verschieben", "win.modifier.<Super>": "Super", "win.modifier.<Alt>": "Alt",
        "win.resize": "Größe mit rechter Maustaste", "win.attach": "Dialoge anhängen",
        "win.center": "Neue Fenster mittig", "win.edge": "Am Rand einrasten",
        "top.date": "Datum", "top.weekday": "Wochentag", "top.seconds": "Sekunden", "top.format": "Format",
        "top.format.24h": "24 Stunden", "top.format.12h": "12 Stunden (AM/PM)", "top.weeknum": "Kalenderwochen",
        "top.battery": "Akku in Prozent", "top.hotcorner": "Aktive Ecke oben links",
        "top.dynamic": "Dynamische Arbeitsflächen", "top.numws": "Anzahl (wenn fest)",
        "top.primary": "Nur auf dem Hauptbildschirm", "top.wsswitch": "Alt+Tab nur aktuelle Arbeitsfläche",
        "in.accel": "Mausbeschleunigung", "in.accel.default": "Standard", "in.accel.flat": "Aus (flach)",
        "in.accel.adaptive": "An (adaptiv)", "in.natural": "Maus: natürliches Scrollen",
        "in.tap": "Touchpad: Tippen zum Klicken", "in.tpnatural": "Touchpad: natürliches Scrollen",
        "in.typing": "Touchpad beim Tippen aus", "in.paste": "Einfügen mit Mittelklick",
        "in.overlay": "Taste für die Übersicht", "in.overlay.Super_L": "Super links", "in.overlay.Super_R": "Super rechts",
        "step.gdm.logo": "Logo wird gesetzt", "step.gdm.banner": "Text wird gesetzt",
        "step.gdm.bg": "Anmeldebildschirm wird neu gebaut", "step.gdm.restore": "Original wird zurückgeholt",
        "err.gdm.image": "Bild ungültig", "err.gdm.missing": "gnome-shell-theme.gresource fehlt",
        "err.gdm.gresource": "gresource konnte nicht gebaut werden", "err.gdm.nobackup": "Keine Sicherung vorhanden",
        **_expand("win.dbl", _TITLE_ACTIONS_DE), **_expand("win.mid", _TITLE_ACTIONS_DE),
        **_expand("win.right", _TITLE_ACTIONS_DE), **_expand("look.accent", _ACCENTS_DE),
    },
    "en": {
        "tab.settings": "Settings",
        "src.store": "Install", "sddm.download": "Install",
        "apply": "Apply", "cancel": "Cancel", "na": "Not available on this system",
        "set.desktops": "Desktops", "set.none": "No desktop installed",
        "set.running": "Running now", "set.notinstalled": "Not installed",
        "sec.look": "Appearance", "sec.cursor": "Cursor", "sec.icon": "Icons", "sec.fonts": "Fonts",
        "sec.windows": "Windows", "sec.titlebar": "Title bar", "sec.topbar": "Top bar",
        "sec.input": "Mouse & keyboard", "sec.autostart": "Startup apps", "sec.login": "Login screen",
        "sec.plasma": "Appearance", "sec.hyprstudio": "Hypr Studio",
        "look.cursor": "Cursor", "look.icon": "Icons", "look.folder": "Folder …", "look.reload": "Reload",
        "look.source": "Source", "look.nofolder": "Folder not found – installed themes only",
        "look.nothemes": "No themes in the folder – installed only", "look.active": "Active",
        "look.unknown": "Active theme unknown", "look.installing": "Installing {name} …",
        "look.installed": "{name} installed",
        "gn.look": "Appearance", "gn.look.style": "Style", "gn.look.themes": "Themes",
        "gn.gtk": "Legacy applications (GTK theme)", "gn.shell": "Shell theme",
        "gn.shell.tip": "Needs the “User Themes” extension", "gn.default": "Default",
        "gn.ext.on": "Extension on", "gn.ext.off": "Extension off", "gn.ext.missing": "Extension missing",
        "gn.ext.name": "Extension",
        "gn.fonts": "Fonts", "gn.windows": "Windows", "gn.input": "Mouse & keyboard",
        "gn.titlebar": "Title bar", "gn.title.window": "Window", "gn.btn.min": "Minimise",
        "gn.btn.max": "Maximise", "gn.btn.side": "Buttons", "gn.left": "Left", "gn.right": "Right",
        "gn.topbar": "Top bar", "gn.top.clock": "Clock", "gn.top.bar": "Bar", "gn.top.ws": "Workspaces",
        "gn.top.wsind": "Workspace indicator",
        "gn.top.wsind.tip": "The “Workspace Indicator” extension shows the workspace in the bar",
        "gn.autostart": "Startup apps", "gn.auto.add": "Add app", "gn.auto.none": "No startup apps",
        "gn.login": "Login screen", "gn.login.bg": "Background", "gn.login.pick": "Choose image …",
        "gn.login.color": "Colour …", "gn.login.restore": "Original",
        "gn.login.restore.tip": "Restore the original login screen design",
        "gn.login.bg.tip": "Rebuilds gnome-shell-theme.gresource (original is backed up). Set again after GNOME updates.",
        "gn.login.logo": "Logo", "gn.login.more": "More", "gn.login.banner": "Text on the login screen",
        "gn.login.banner.ph": "empty = no text", "gn.login.users": "Show user list",
        "gn.login.nogdm": "GDM missing",
        "pl.look": "Appearance", "pl.scheme": "Colour scheme", "pl.desktop": "Plasma style",
        "font.ui": "Interface", "font.doc": "Documents", "font.mono": "Monospace", "font.title": "Title bar",
        "font.hint": "Hinting", "font.hint.none": "None", "font.hint.slight": "Slight", "font.hint.medium": "Medium",
        "font.hint.full": "Full", "font.aa": "Antialiasing", "font.aa.none": "None",
        "font.aa.grayscale": "Greyscale", "font.aa.rgba": "Subpixel (LCD)", "font.scale": "Scaling",
        "look.scheme": "Style", "look.scheme.default": "Default", "look.scheme.prefer-dark": "Dark",
        "look.scheme.prefer-light": "Light", "look.accent": "Accent colour", "look.anim": "Animations",
        "win.dbl": "Double click", "win.mid": "Middle click", "win.right": "Right click",
        "win.focus": "Focus", "win.focus.click": "Click", "win.focus.sloppy": "Mouse over",
        "win.focus.mouse": "Mouse (strict)", "win.raise": "Raise on click",
        "win.modifier": "Key to move windows", "win.modifier.<Super>": "Super", "win.modifier.<Alt>": "Alt",
        "win.resize": "Resize with right button", "win.attach": "Attach modal dialogs",
        "win.center": "Centre new windows", "win.edge": "Edge tiling",
        "top.date": "Date", "top.weekday": "Weekday", "top.seconds": "Seconds", "top.format": "Format",
        "top.format.24h": "24 hours", "top.format.12h": "12 hours (AM/PM)", "top.weeknum": "Week numbers",
        "top.battery": "Battery percentage", "top.hotcorner": "Hot corner top left",
        "top.dynamic": "Dynamic workspaces", "top.numws": "Number (when fixed)",
        "top.primary": "Primary display only", "top.wsswitch": "Alt+Tab current workspace only",
        "in.accel": "Mouse acceleration", "in.accel.default": "Default", "in.accel.flat": "Off (flat)",
        "in.accel.adaptive": "On (adaptive)", "in.natural": "Mouse: natural scrolling",
        "in.tap": "Touchpad: tap to click", "in.tpnatural": "Touchpad: natural scrolling",
        "in.typing": "Disable touchpad while typing", "in.paste": "Middle click paste",
        "in.overlay": "Overview key", "in.overlay.Super_L": "Left Super", "in.overlay.Super_R": "Right Super",
        "step.gdm.logo": "Setting logo", "step.gdm.banner": "Setting text",
        "step.gdm.bg": "Rebuilding login screen", "step.gdm.restore": "Restoring original",
        "err.gdm.image": "Invalid image", "err.gdm.missing": "gnome-shell-theme.gresource missing",
        "err.gdm.gresource": "Could not build gresource", "err.gdm.nobackup": "No backup found",
        **_expand("win.dbl", _TITLE_ACTIONS_EN), **_expand("win.mid", _TITLE_ACTIONS_EN),
        **_expand("win.right", _TITLE_ACTIONS_EN), **_expand("look.accent", _ACCENTS_EN),
    },
}

# 2.1: Download-Quelle GitHub, Ein-Klick-Installation, Über
EXTRA["de"].update({
    "remote.src.online": "GitHub", "remote.src.folder": "Ordner", "remote.repo": "Repo",
    "remote.repo.change": "Repo …", "remote.repo.ask": "GitHub-Repo (Besitzer/Name):",
    "remote.loading": "Lade Liste aus {repo} …", "remote.loaded": "Liste geladen: {repo} ({tag})",
    "remote.empty": "In {repo} gibt es für diese Kategorie noch nichts.",
    "remote.pack.info": "{n} Themes · {have} installiert · {mb} MB",
    "remote.pack.install": "Alles installieren", "remote.pack.done": "Alles installiert",
    "remote.pack.ok": "{n} Themes installiert",
    "remote.badrepo": "Ungültiges Repo „{repo}“ (Form: Besitzer/Name)",
    "remote.norelease": "In {repo} gibt es noch kein Release mit Downloads.",
    "remote.nomanifest": "Im Release fehlt manifest.json.",
    "remote.badmanifest": "manifest.json ist beschädigt.",
    "remote.offline": "Keine Verbindung zu GitHub – Internet prüfen oder Quelle „Ordner“ nutzen.",
    "remote.limit": "GitHub-Abfragelimit erreicht – in einer Stunde erneut versuchen.",
    "remote.http": "GitHub antwortet mit einem Fehler.",
    "remote.toobig": "Datei zu groß – abgebrochen.",
    "remote.badsum": "Prüfsumme stimmt nicht – Datei verworfen, nichts installiert.",
    "remote.badarchive": "Archiv unsicher oder beschädigt – nichts installiert.",
    "miss.install": "Installieren", "miss.installing": "{name} wird installiert …",
    "miss.done": "{name} installiert", "miss.relogin": "{name} installiert – unter Wayland einmal ab- und wieder anmelden",
    "about": "Über Linux AIO", "about.license": "Lizenz",
})
EXTRA["en"].update({
    "remote.src.online": "GitHub", "remote.src.folder": "Folder", "remote.repo": "Repo",
    "remote.repo.change": "Repo …", "remote.repo.ask": "GitHub repo (owner/name):",
    "remote.loading": "Loading list from {repo} …", "remote.loaded": "List loaded: {repo} ({tag})",
    "remote.empty": "{repo} has nothing in this category yet.",
    "remote.pack.info": "{n} themes · {have} installed · {mb} MB",
    "remote.pack.install": "Install all", "remote.pack.done": "All installed",
    "remote.pack.ok": "{n} themes installed",
    "remote.badrepo": "Invalid repo “{repo}” (format: owner/name)",
    "remote.norelease": "{repo} has no release with downloads yet.",
    "remote.nomanifest": "manifest.json is missing in the release.",
    "remote.badmanifest": "manifest.json is damaged.",
    "remote.offline": "No connection to GitHub – check the internet or use the “Folder” source.",
    "remote.limit": "GitHub rate limit reached – try again in an hour.",
    "remote.http": "GitHub answered with an error.",
    "remote.toobig": "File too large – aborted.",
    "remote.badsum": "Checksum mismatch – file discarded, nothing installed.",
    "remote.badarchive": "Archive unsafe or damaged – nothing installed.",
    "miss.install": "Install", "miss.installing": "Installing {name} …",
    "miss.done": "{name} installed", "miss.relogin": "{name} installed – on Wayland log out and back in once",
    "about": "About Linux AIO", "about.license": "Licence",
})

EXTRA["de"].update({
    "gn.btn.close": "Schließen",
    "gn.btn.noclose": "Ohne Schließen-Knopf: Fenster mit Alt+F4 (oder deinem Kürzel, z. B. Super+Q) schließen.",
    "gn.btns": "Knopf-Themes", "gn.btns.note": "Wirkt in Programmen nach deren Neustart. „Dateien“ läuft im Hintergrund weiter – „Dateien neu starten“ beendet es kurz.",
    "gn.btns.restart": "Knopf-Theme gesetzt – Programme neu starten",
})
EXTRA["en"].update({
    "gn.btn.close": "Close",
    "gn.btn.noclose": "Without a close button: close windows with Alt+F4 (or your shortcut, e.g. Super+Q).",
    "gn.btns": "Button themes", "gn.btns.note": "Takes effect in apps after restarting them. Files keeps running in the background – “Restart Files” quits it briefly.",
    "gn.btns.restart": "Button theme set – restart apps",
})

EXTRA["de"].update({
    "about.text": ("Freie Software unter der <b>GNU GPL 3.0</b> – Quellcode: <a href=\"{url}\">{url}</a><br><br>"
                   "Enthält Hypr Studio (LabtosoStudios, GPL-3.0).<br>"
                   "Mauszeiger: Bibata / Material Bibata Cursor (GPL-3.0).<br>"
                   "Oberfläche: Qt for Python / PySide6 (LGPL-3.0)."),
})
EXTRA["en"].update({
    "about.text": ("Free software under the <b>GNU GPL 3.0</b> – source: <a href=\"{url}\">{url}</a><br><br>"
                   "Includes Hypr Studio (LabtosoStudios, GPL-3.0).<br>"
                   "Cursors: Bibata / Material Bibata Cursor (GPL-3.0).<br>"
                   "Interface: Qt for Python / PySide6 (LGPL-3.0)."),
})

EXTRA["de"].update({
    "gn.corners": "Eckenrundung", "gn.corners.radius": "Radius der App-Fenster",
    "gn.corners.all": "Auch andere Programme (Erweiterung)",
    "gn.corners.note": "Gilt für GTK-3- und GTK-4-Programme nach deren Neustart (Standard bei GNOME: 15 px). Firefox/Zen, Chrome/Electron und Qt-Programme folgen nur mit der Erweiterung „Rounded Window Corners Reborn“.",
    "gn.corners.done": "Eckenrundung {r} px – Programme neu starten", "gn.corners.reset": "Eckenrundung wieder Standard",
})
EXTRA["en"].update({
    "gn.corners": "Corner radius", "gn.corners.radius": "App window radius",
    "gn.corners.all": "Other apps too (extension)",
    "gn.corners.note": "Applies to GTK 3 and GTK 4 apps after restarting them (GNOME default: 15 px). Firefox/Zen, Chrome/Electron and Qt apps only follow with the “Rounded Window Corners Reborn” extension.",
    "gn.corners.done": "Corner radius {r} px – restart apps", "gn.corners.reset": "Corner radius back to default",
})

EXTRA["de"]["about.version"] = "Version"
EXTRA["en"]["about.version"] = "Version"

EXTRA["de"].update({
    "font.store": "Schriften installieren", "font.search": "Schrift suchen …", "font.onlyinst": "Nur installierte",
    "font.f.all": "Alle", "font.f.Sans Serif": "Sans", "font.f.Serif": "Serif", "font.f.Monospace": "Mono",
    "font.f.Display": "Display", "font.f.Handwriting": "Handschrift", "font.f.Nerd Font": "Nerd Fonts",
    "font.cat.Sans Serif": "Sans Serif · Google Fonts", "font.cat.Serif": "Serif · Google Fonts",
    "font.cat.Monospace": "Monospace · Google Fonts", "font.cat.Display": "Display · Google Fonts",
    "font.cat.Handwriting": "Handschrift · Google Fonts", "font.cat.Nerd Font": "Nerd Font · mit Symbolen",
    "font.use": "Verwenden", "font.more": "Mehr laden", "font.loading": "Schriften werden geladen …",
    "font.none": "Keine Schrift gefunden", "font.nopreview": "Vorschau nach dem Installieren",
    "font.applied": "{name} ist jetzt Schrift für: {where}",
    "fonts.nofiles": "Keine Schriftdateien gefunden", "fonts.bad": "Datei ist keine Schrift – abgebrochen",
})
EXTRA["en"].update({
    "font.store": "Install fonts", "font.search": "Search fonts …", "font.onlyinst": "Installed only",
    "font.f.all": "All", "font.f.Sans Serif": "Sans", "font.f.Serif": "Serif", "font.f.Monospace": "Mono",
    "font.f.Display": "Display", "font.f.Handwriting": "Handwriting", "font.f.Nerd Font": "Nerd Fonts",
    "font.cat.Sans Serif": "Sans serif · Google Fonts", "font.cat.Serif": "Serif · Google Fonts",
    "font.cat.Monospace": "Monospace · Google Fonts", "font.cat.Display": "Display · Google Fonts",
    "font.cat.Handwriting": "Handwriting · Google Fonts", "font.cat.Nerd Font": "Nerd Font · with icons",
    "font.use": "Use", "font.more": "Load more", "font.loading": "Loading fonts …",
    "font.none": "No font found", "font.nopreview": "Preview after installing",
    "font.applied": "{name} is now the font for: {where}",
    "fonts.nofiles": "No font files found", "fonts.bad": "File is not a font – aborted",
})

EXTRA["de"].update({
    "maximize": "Maximieren",
    "miss.incompatible": "{name} installiert, unterstützt diese GNOME-Version aber noch nicht",
    "gn.ext.incompatible": "Nicht für diese GNOME-Version",
    "gn.ext.force": "Trotzdem aktivieren",
    "gn.ext.force.ask": "Die Erweiterung gibt diese GNOME-Version nicht an. GNOME kann die Versionsprüfung für alle Erweiterungen abschalten – dann läuft sie meist trotzdem, im schlimmsten Fall wird sie beim Anmelden deaktiviert. Danach ab- und wieder anmelden. Fortfahren?",
})
EXTRA["en"].update({
    "maximize": "Maximise",
    "miss.incompatible": "{name} installed, but it does not support this GNOME version yet",
    "gn.ext.incompatible": "Not for this GNOME version",
    "gn.ext.force": "Enable anyway",
    "gn.ext.force.ask": "The extension does not list this GNOME version. GNOME can turn off the version check for all extensions – it usually works then; at worst it is disabled again at login. Log out and back in afterwards. Continue?",
})

EXTRA["de"].update({
    "gn.btns.moz": "Auch in Firefox und Zen (userChrome.css)",
    "gn.sup.gtk": "GTK 3/4: ja", "gn.sup.moz": "Firefox/Zen: ja", "gn.sup.chromium": "Chromium: teilweise",
    "gn.sup.electron": "Electron: nein", "gn.sup.qt": "Qt/KDE: nein",
    "gn.sup.moz.tip": "Über userChrome.css im Profil; nur runde und eckige Themes",
    "gn.sup.chromium.tip": "Im Browser „Systemtitelleiste und -rahmen verwenden“ einschalten – dann zeichnet GTK die Knöpfe",
    "gn.sup.electron.tip": "Electron-Programme (z. B. Discord, VS Code, Slack) zeichnen eigene Knöpfe",
    "gn.sup.qt.tip": "Qt-Programme folgen unter GNOME nicht; unter Plasma regelt das das Fensterdekorations-Design",
})
EXTRA["en"].update({
    "gn.btns.moz": "Also in Firefox and Zen (userChrome.css)",
    "gn.sup.gtk": "GTK 3/4: yes", "gn.sup.moz": "Firefox/Zen: yes", "gn.sup.chromium": "Chromium: partly",
    "gn.sup.electron": "Electron: no", "gn.sup.qt": "Qt/KDE: no",
    "gn.sup.moz.tip": "Via userChrome.css in the profile; round and square themes only",
    "gn.sup.chromium.tip": "Turn on “Use system title bar and borders” in the browser – GTK then draws the buttons",
    "gn.sup.electron.tip": "Electron apps (e.g. Discord, VS Code, Slack) draw their own buttons",
    "gn.sup.qt.tip": "Qt apps do not follow under GNOME; on Plasma the window decoration theme controls this",
})

EXTRA["de"].update({
    "sec.app": "Programm", "app.title": "Linux AIO", "app.scale": "Größe der Oberfläche",
    "app.scale.100": "100 %", "app.scale.90": "90 %", "app.scale.80": "80 %",
    "app.hypr": "Hypr Studio", "app.hypr.embed": "Eingebettet", "app.hypr.window": "Eigenes Fenster",
    "app.hypr.open": "Hypr Studio öffnen", "app.restart": "Neu starten", "app.restart.need": "Wirkt nach einem Neustart",
})
EXTRA["en"].update({
    "sec.app": "App", "app.title": "Linux AIO", "app.scale": "Interface size",
    "app.scale.100": "100 %", "app.scale.90": "90 %", "app.scale.80": "80 %",
    "app.hypr": "Hypr Studio", "app.hypr.embed": "Embedded", "app.hypr.window": "Separate window",
    "app.hypr.open": "Open Hypr Studio", "app.restart": "Restart", "app.restart.need": "Takes effect after a restart",
})

EXTRA["de"].update({
    "bar.editor": "Leisten-Editor", "bar.tip": "Auf ein Element klicken: zeigen oder ausblenden",
    "bar.needs": "Braucht die Erweiterung „Just Perfection“",
    "bar.activities": "Aktivitäten", "bar.clock": "Uhr", "bar.notify": "Benachrichtigungen", "bar.keyboard": "Tastatur",
    "bar.a11y": "Barrierefreiheit", "bar.screenshare": "Bildschirmfreigabe", "bar.quick": "Schnelleinstellungen",
    "bar.power": "Akku",
    "bar.clockpos": "Uhr", "bar.pos.0": "Mitte", "bar.pos.1": "Rechts", "bar.pos.2": "Links",
    "bar.place": "Leiste", "bar.place.0": "Oben", "bar.place.1": "Unten",
    "qs.dark": "Schnelleinstellung Dunkler Stil", "qs.night": "Schnelleinstellung Nachtlicht",
    "qs.dnd": "Schnelleinstellung Nicht stören", "qs.airplane": "Schnelleinstellung Flugmodus",
    "qs.backlight": "Schnelleinstellung Tastaturbeleuchtung",
    "pw.menu": "Power-Menü", "pw.logout": "„Abmelden“ immer zeigen", "pw.switch": "„Benutzer wechseln“",
    "pw.lock": "„Sperren“",
})
EXTRA["en"].update({
    "bar.editor": "Bar editor", "bar.tip": "Click an element to show or hide it",
    "bar.needs": "Needs the “Just Perfection” extension",
    "bar.activities": "Activities", "bar.clock": "Clock", "bar.notify": "Notifications", "bar.keyboard": "Keyboard",
    "bar.a11y": "Accessibility", "bar.screenshare": "Screen sharing", "bar.quick": "Quick settings",
    "bar.power": "Battery",
    "bar.clockpos": "Clock", "bar.pos.0": "Centre", "bar.pos.1": "Right", "bar.pos.2": "Left",
    "bar.place": "Bar", "bar.place.0": "Top", "bar.place.1": "Bottom",
    "qs.dark": "Quick setting Dark style", "qs.night": "Quick setting Night light",
    "qs.dnd": "Quick setting Do not disturb", "qs.airplane": "Quick setting Airplane mode",
    "qs.backlight": "Quick setting Keyboard backlight",
    "pw.menu": "Power menu", "pw.logout": "Always show “Log out”", "pw.switch": "“Switch user”",
    "pw.lock": "“Lock”",
})

EXTRA["de"].update({"gn.files.restart": "Dateien neu starten"})
EXTRA["en"].update({"gn.files.restart": "Restart Files"})

EXTRA["de"]["tab.launcher"] = "Launcher"
EXTRA["en"]["tab.launcher"] = "Launcher"

EXTRA["de"]["sec.launcher"] = "Launcher"
EXTRA["en"]["sec.launcher"] = "Launcher"

EXTRA["de"]["app.settings"] = "Linux-AIO-Einstellungen"
EXTRA["en"]["app.settings"] = "Linux AIO settings"
