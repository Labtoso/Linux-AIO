"""Hypr Studio komplett im Tab „Einstellungen › Hyprland“ (Tabs Hyprland/Waybar/Launcher/Sonstiges)."""

from hyprstudio import gui as hgui
from hyprstudio import widgets as hwidgets

from . import desktops, usercmd
from .qt import QtWidgets, Qt


class HyprEmbed(hgui.MainWindow):
    """Hypr-Studio-Fenster als eingebettetes Widget: ohne Fensterrahmen, Titel, Minimieren/Schließen."""

    def __init__(self, host):
        if usercmd.DRY:   # Testmodus: auch Hypr Studios eigene Einstellungen nicht im echten Home
            hwidgets.CONFIG = usercmd.CACHE / "testmodus" / "hyprstudio.json"
        self.host = host
        super().__init__()
        self.setWindowFlags(Qt.WindowType.Widget)
        self.setAttribute(Qt.WidgetAttribute.WA_TranslucentBackground, False)
        self.setMinimumSize(0, 0)
        self.title.hide()
        self.btn_min.hide()
        self.btn_close.hide()
        self.content.layout().setContentsMargins(14, 10, 14, 12)
        for g in self.findChildren(QtWidgets.QSizeGrip):
            g.hide()
        self.gear.setToolTip("Hypr Studio")
        # eigene Fußzeile weg: Status und Protokoll laufen über Linux AIO
        self.status.hide()
        self.btn_log.hide()

    def logview_toggle(self, on: bool):
        try:
            self.host.btn_log.setChecked(on)
        except RuntimeError:
            pass

    def _on_hyprland(self) -> bool:
        return desktops.current() == "hyprland"

    # Automatiken von Hypr Studio nur, wenn wirklich Hyprland läuft (sonst nichts im System ändern)
    def _first_look(self):
        if self._on_hyprland() or usercmd.DRY:
            super()._first_look()

    def _auto_laptop(self):
        if self._on_hyprland():
            super()._auto_laptop()

    def _auto_sysactions(self):
        if self._on_hyprland():
            super()._auto_sysactions()

    def _auto_look(self):
        if self._on_hyprland():
            super()._auto_look()

    def _startup_check(self):
        if self._on_hyprland() or usercmd.DRY:
            super()._startup_check()

    def set_status(self, text: str):
        super().set_status(text)
        try:
            self.host.set_status(text)
        except RuntimeError:
            pass

    def log(self, text: str):
        super().log(text)
        try:
            self.host.log(text)
        except RuntimeError:
            pass

    def mousePressEvent(self, e):
        pass

    def mouseDoubleClickEvent(self, e):
        pass

    def sync_look(self, lang: str, accent_name: str):
        if self.cfg.get("lang") != lang:
            self.set_language(lang)
        if accent_name in hgui.theme.THEMES and self.cfg.get("theme") != accent_name:
            self.set_theme(accent_name)
