"""Hypr Studio komplett im Tab „Einstellungen › Hyprland“ (Tabs Hyprland/Waybar/Launcher/Sonstiges)."""

from hyprstudio import gui as hgui
from hyprstudio import widgets as hwidgets

from . import desktops, usercmd
from .qt import QtWidgets, Qt


def add_advanced(win):
    """Section „Advanced“ in Hypr Studio's sidebar (More): config format + reset everything."""
    from hyprstudio import i18n as hi
    from hyprstudio import model
    from hyprstudio.widgets import Segmented as HSeg
    hi.TEXT["de"].update({"sec.advanced": "Erweitert", "adv.reset": "Alles zurücksetzen",
                          "adv.reset.tip": "Nimmt alle Änderungen von Hypr Studio an Hyprland, Waybar und Launchern zurück"})
    hi.TEXT["en"].update({"sec.advanced": "Advanced", "adv.reset": "Reset everything",
                          "adv.reset.tip": "Undoes all Hypr Studio changes to Hyprland, Waybar and launchers"})
    op = win.other_page
    page = QtWidgets.QWidget()
    lay = QtWidgets.QVBoxLayout(page)
    lay.setContentsMargins(0, 0, 0, 0)
    lay.setSpacing(12)
    row = QtWidgets.QHBoxLayout()
    lbl = QtWidgets.QLabel()
    seg = HSeg(("conf", "lua"), prefix="fmtsw.")
    seg.changed.connect(win.set_format)
    row.addWidget(lbl)
    row.addStretch()
    row.addWidget(seg)
    lay.addLayout(row)
    reset = QtWidgets.QPushButton()
    reset.setObjectName("DangerFill")
    reset.setCursor(Qt.CursorShape.PointingHandCursor)
    reset.clicked.connect(win.reset_all)
    lay.addWidget(reset, 0, Qt.AlignmentFlag.AlignLeft)
    lay.addStretch()
    op.stack.addWidget(page)
    op.pages["advanced"] = page
    op.SECTIONS = tuple(op.SECTIONS) + ("advanced",)
    b = QtWidgets.QPushButton()
    b.setObjectName("Nav")
    b.setCheckable(True)
    b.setCursor(Qt.CursorShape.PointingHandCursor)
    b.clicked.connect(lambda: op.show_section("advanced"))
    group = next((x.group() for x in op.nav.values() if x.group()), None)
    if group:
        group.addButton(b)
    op.nav["advanced"] = b
    side = op.findChild(QtWidgets.QFrame, "Side")
    sl = side.layout()
    sl.insertWidget(sl.count() - 1, b)

    def retr():
        b.setText(hi.t("sec.advanced"))
        lbl.setText(hi.t("adv.format"))
        reset.setText(hi.t("adv.reset"))
        reset.setToolTip(hi.t("adv.reset.tip"))
        seg.retranslate()
        cur = win.cfg.get("hypr_format") or model.hypr_format(model.config_home() / "hypr") or "lua"
        seg.select(cur)
    win._adv_retr = retr
    retr()


class HyprEmbed(hgui.MainWindow):
    """Hypr-Studio-Fenster als eingebettetes Widget: ohne Fensterrahmen, Titel, Minimieren/Schließen."""

    def __init__(self, host, popout: bool = False, launcher_only: bool = False):
        if usercmd.DRY:   # Testmodus: auch Hypr Studios eigene Einstellungen nicht im echten Home
            hwidgets.CONFIG = usercmd.CACHE / "testmodus" / "hyprstudio.json"
        self.host = host
        self.popout = popout
        self.launcher_only = launcher_only
        super().__init__()
        if launcher_only:            # Linux AIO › Launcher: nur Hypr Studios Launcher-Seite
            self.tabs.hide()
            hgui.MainWindow.set_tab(self, "launcher")
        else:                        # der Launcher hat in Linux AIO einen eigenen Tab
            self.tabs.btns["launcher"].hide()
            if self.cfg.get("tab") == "launcher":
                hgui.MainWindow.set_tab(self, "hypr")
        self.gear.hide()                 # Einstellungen stecken in Sonstiges › Erweitert
        add_advanced(self)
        if popout:                       # eigenes Fenster: frei verschiebbar, maximierbar
            self.setWindowTitle("Hypr Studio – Linux AIO")
            self.status.hide()
            self.btn_log.hide()
            from hyprstudio import theme as htheme
            from .widgets import draw_icon
            self.btn_max = QtWidgets.QToolButton()
            self.btn_max.setIconSize(self.btn_min.iconSize())
            self.btn_max.setIcon(draw_icon("max", htheme.MUTED, 18))
            self.btn_max.clicked.connect(lambda: self.showNormal() if self.isMaximized() else self.showMaximized())
            row = next((lo for lo in self.findChildren(QtWidgets.QHBoxLayout) if lo.indexOf(self.btn_min) >= 0), None)
            if row is not None:
                row.insertWidget(row.indexOf(self.btn_min) + 1, self.btn_max)
            return
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

    def set_tab(self, tab: str):
        if getattr(self, "launcher_only", False):
            tab = "launcher"
        elif tab == "launcher":
            tab = "hypr"
        super().set_tab(tab)

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
        if self.popout:
            super().mousePressEvent(e)

    def mouseDoubleClickEvent(self, e):
        if self.popout:
            super().mouseDoubleClickEvent(e)

    def apply_theme(self):
        """Hypr Studio's stylesheet, scaled like Linux AIO (from its base text, never cumulative)."""
        super().apply_theme()
        from . import theme as aio_theme
        try:
            factor = self.host.ui_factor()
        except (AttributeError, RuntimeError):
            factor = 1.0
        if abs(factor - 1.0) > 0.001:
            from hyprstudio import theme as htheme
            self.setStyleSheet(aio_theme.scale_qss(htheme.qss(self.accent()), factor))

    def retranslate(self):
        super().retranslate()
        if hasattr(self, "_adv_retr"):
            self._adv_retr()

    def sync_look(self, lang: str, accent_name: str):
        if self.cfg.get("lang") != lang:
            self.set_language(lang)
        if accent_name in hgui.theme.THEMES and self.cfg.get("theme") != accent_name:
            self.set_theme(accent_name)


class HyprLauncher(QtWidgets.QWidget):
    """Mode „separate window“: small page with a button; Hypr Studio opens as its own window."""

    def __init__(self, host):
        super().__init__()
        from .i18n import t
        self.host = host
        lay = QtWidgets.QVBoxLayout(self)
        lay.addStretch()
        self.btn = QtWidgets.QPushButton(t("app.hypr.open"))
        self.btn.setObjectName("Primary")
        self.btn.setMinimumSize(240, 44)
        self.btn.setCursor(Qt.CursorShape.PointingHandCursor)
        self.btn.clicked.connect(self.open)
        lay.addWidget(self.btn, 0, Qt.AlignmentFlag.AlignHCenter)
        lay.addStretch()

    def open(self):
        win = getattr(self.host, "_hypr_window", None)
        if win is None:
            win = HyprEmbed(self.host, popout=True)
            self.host._hypr_window = win
        win.sync_look(self.host.cfg.get("lang", "en"), self.host.cfg.get("theme", "blue"))
        win.show()
        win.raise_()
        win.activateWindow()

    def sync_look(self, lang, accent):
        from .i18n import t
        self.btn.setText(t("app.hypr.open"))
