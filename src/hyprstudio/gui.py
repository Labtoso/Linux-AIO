"""Hauptfenster: Tabs Hyprland / Waybar oben, Design-Vorlage Nr. 1."""

import os
import sys

import hyprstudio
from . import appearance, hardware, laptopkeys, launcher, model, packages, reset, sysactions, theme
from .i18n import t, set_lang
from .gate import Gate, LauncherGate
from .other import OtherPage
from .pages import HyprPage, LauncherPage, WaybarPage
from .qt import QtCore, QtGui, QtWidgets, Qt
from .widgets import (ResetDialog, RoundPopups, Segmented, SettingsPanel, Switch, app_icon, draw_icon, load_cfg,
                      save_cfg)


TABS = ("hypr", "waybar", "launcher", "other")


_QT_TR = None


def _qt_lang(code: str) -> None:
    """Qt-eigene Texte (Dateibrowser, Farbwahl) in der App-Sprache; fehlt die Übersetzung, bleibt Englisch."""
    global _QT_TR
    app = QtWidgets.QApplication.instance()
    if _QT_TR is not None:
        app.removeTranslator(_QT_TR)
        _QT_TR = None
    if code == "en":
        return
    tr = QtCore.QTranslator(app)
    path = QtCore.QLibraryInfo.path(QtCore.QLibraryInfo.LibraryPath.TranslationsPath)
    if tr.load(f"qtbase_{code}", path):
        app.installTranslator(tr)
        _QT_TR = tr


class MainWindow(QtWidgets.QWidget):
    def __init__(self):
        super().__init__()
        self.cfg = load_cfg()
        set_lang(self.cfg.get("lang", "en"))
        _qt_lang(self.cfg.get("lang", "en"))
        model.FORMAT = self.cfg.get("hypr_format", "") if self.cfg.get("hypr_format") in ("lua", "conf") else ""
        model.LAUNCHER_TOGGLE = launcher.state(self.cfg.get("launcher")).get("toggle", True)
        if self.cfg.get("theme") not in theme.THEMES:
            self.cfg["theme"] = "blue"
        if self.cfg.get("tab") not in TABS:
            self.cfg["tab"] = "hypr"
        self._save = QtCore.QTimer(self)
        self._save.setSingleShot(True)
        self._save.setInterval(400)
        self._save.timeout.connect(lambda: save_cfg(self.cfg))

        self.setWindowTitle(hyprstudio.APP_NAME)
        self.setWindowIcon(app_icon())
        self.setWindowFlags(Qt.WindowType.FramelessWindowHint | Qt.WindowType.Window)
        self.setAttribute(Qt.WidgetAttribute.WA_TranslucentBackground)
        self.resize(1220, 860)
        self.setMinimumSize(1000, 720)

        outer = QtWidgets.QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        self.root = QtWidgets.QFrame(self)
        self.root.setObjectName("Root")
        outer.addWidget(self.root)
        rl = QtWidgets.QVBoxLayout(self.root)
        rl.setContentsMargins(0, 0, 0, 0)
        self.content = QtWidgets.QWidget(self.root)
        rl.addWidget(self.content)
        cl = QtWidgets.QVBoxLayout(self.content)
        cl.setContentsMargins(20, 12, 20, 16)
        cl.setSpacing(14)

        head = QtWidgets.QHBoxLayout()
        left = QtWidgets.QHBoxLayout()
        left.setSpacing(10)
        self.gear = QtWidgets.QToolButton()
        self.gear.setIconSize(QtCore.QSize(22, 22))
        self.gear.setCursor(Qt.CursorShape.PointingHandCursor)
        self.gear.clicked.connect(self.toggle_settings)
        self.title = QtWidgets.QLabel(hyprstudio.APP_NAME)
        self.title.setObjectName("Title")
        left.addWidget(self.gear)
        left.addWidget(self.title)
        left.addStretch()
        self.tabs = Segmented(TABS, prefix="tab.")
        self.tabs.setObjectName("TabBox")
        self.tabs.changed.connect(self.set_tab)
        right = QtWidgets.QHBoxLayout()
        right.addStretch()
        self.btn_min = QtWidgets.QToolButton()
        self.btn_min.clicked.connect(self.showMinimized)
        self.btn_close = QtWidgets.QToolButton()
        self.btn_close.setObjectName("CloseBtn")
        self.btn_close.clicked.connect(self.close)
        for b in (self.btn_min, self.btn_close):
            b.setIconSize(QtCore.QSize(18, 18))
            right.addWidget(b)
        head.addLayout(left, 1)
        head.addWidget(self.tabs)
        head.addLayout(right, 1)
        cl.addLayout(head)

        self.hypr_page = HyprPage(self)
        self.waybar_page = WaybarPage(self)
        self.launcher_page = LauncherPage(self)
        self.stack = QtWidgets.QStackedWidget()
        self.stack.addWidget(self.hypr_page)
        self.waybar_gate = Gate(self.waybar_page, "waybar", self)
        self.launcher_gate = LauncherGate(self.launcher_page, self)
        self.stack.addWidget(self.waybar_gate)
        self.stack.addWidget(self.launcher_gate)
        self.other_page = OtherPage(self)
        self.stack.addWidget(self.other_page)
        for k in ("wallpaper", "cursor", "lock"):  # gehören zu Hyprland
            self.hypr_page.add_external(k, self.other_page.gates[k])
        self.hypr_page.show_section(self.cfg.get("hypr_section", "presets"))
        QtCore.QTimer.singleShot(0, self._first_look)
        QtCore.QTimer.singleShot(0, self._auto_laptop)
        QtCore.QTimer.singleShot(0, self._auto_sysactions)
        QtCore.QTimer.singleShot(0, self._auto_look)
        cl.addWidget(self.stack, 1)

        foot = QtWidgets.QHBoxLayout()
        self.status = QtWidgets.QLabel()
        self.status.setObjectName("Status")
        self.btn_log = QtWidgets.QPushButton()
        self.btn_log.setCheckable(True)
        self.btn_log.setCursor(Qt.CursorShape.PointingHandCursor)
        self.btn_log.toggled.connect(self.logview_toggle)
        foot.addWidget(self.status, 1)
        self.busy_bar = QtWidgets.QProgressBar()
        self.busy_bar.setRange(0, 0)
        self.busy_bar.setTextVisible(False)
        self.busy_bar.setFixedWidth(200)
        self.busy_bar.hide()
        foot.addWidget(self.busy_bar)
        foot.addSpacing(12)
        foot.addWidget(self.btn_log)
        cl.addLayout(foot)
        self.logview = QtWidgets.QPlainTextEdit()
        self.logview.setReadOnly(True)
        self.logview.setMaximumBlockCount(5000)
        self.logview.setFixedHeight(140)
        self.logview.hide()
        cl.addWidget(self.logview)

        grip_row = QtWidgets.QHBoxLayout()
        grip_row.setContentsMargins(0, 0, 4, 4)
        grip_row.addStretch()
        grip = QtWidgets.QSizeGrip(self.root)
        grip.setStyleSheet("background: transparent;")
        grip_row.addWidget(grip)
        rl.addLayout(grip_row)

        self.dim = QtWidgets.QWidget(self.root)
        self.dim.setStyleSheet("background: rgba(5,8,12,110); border-radius: 16px;")
        self.dim.hide()
        self.dim.mousePressEvent = lambda e: self.toggle_settings()
        self.panel = SettingsPanel(self.root)
        self.panel.hide()
        self.panel.lang_changed.connect(self.set_language)
        self.panel.theme_changed.connect(self.set_theme)
        self.panel.reset_requested.connect(self.reset_all)
        self.panel.format_changed.connect(self.set_format)
        self.anim = QtCore.QPropertyAnimation(self.panel, b"pos", self)
        self.anim.setDuration(220)
        self.anim.setEasingCurve(QtCore.QEasingCurve.Type.OutCubic)
        self.anim.finished.connect(self._anim_done)
        self._closing = False

        self.apply_theme()
        self.retranslate()
        self.update_previews()
        for pg in self.all_pages():
            pg.render_presets(force=True)
        self.set_tab(self.cfg["tab"])
        if model.DRY:
            self.set_status(t("testmode"))
        # beim Start nach Config-Fehlern schauen
        QtCore.QTimer.singleShot(300, self._startup_check)

    def _startup_check(self):
        n = self.other_page.scan(quiet=True)
        if n:
            self.set_status(t("fix.startup", n=n))

    # ----- für die Seiten -----
    def all_pages(self):
        return [pg for pg in (getattr(self, "hypr_page", None), getattr(self, "waybar_page", None),
                              getattr(self, "launcher_page", None)) if pg is not None]

    def accent(self) -> str:
        return theme.THEMES[self.cfg["theme"]]

    def save_later(self):
        self._save.start()

    def update_previews(self):
        hs, ws = self.hypr_page.state, getattr(self, "waybar_page", None)
        ws = ws.state if ws else model.waybar_state(None)
        for pg in self.all_pages():
            pg.preview.set_states(hs, ws)
            pg._presets_dirty = True

    def refresh_gates(self):
        """Ist ein Zusatzprogramm installiert? Sonst großer Installieren-Bildschirm."""
        if not hasattr(self, "other_page"):
            return
        self.waybar_gate.refresh()
        self.launcher_gate.refresh()
        for g in self.other_page.gates.values():
            g.refresh()
        self.hypr_page.sync_external()
        for pg in self.all_pages():
            pg.update_info()
        if self.cfg.get("tab") == "other" and self.cfg.get("other_section") == "install":
            self.other_page.refresh_packages()  # Paketliste nur, wenn sie zu sehen ist
        if self.cfg.get("other_section") == "look":
            self.other_page.refresh_look()
        if self.cfg.get("other_section") == "polkit":
            self.other_page.refresh_polkit()
        if self.cfg.get("other_section") == "laptop":
            self.other_page.refresh_laptop()

    def bar_busy(self, on: bool):
        """Fortschrittsbalken ohne Prozent (läuft hin und her) für Installationen."""
        self.busy_bar.setVisible(on)

    def set_status(self, text: str):
        self.status.setText(text)

    def log(self, text: str):
        self.logview.appendPlainText(text)

    def report(self, ok: bool, info: str):
        if ok:
            self.set_status(t("done"))
        else:
            self.set_status(f"{t('failed')}: {t(info)}")
            if not self.logview.isVisible():
                self.btn_log.setChecked(True)

    # ----- Fenster verschieben (Wayland + X11) -----
    def mousePressEvent(self, e):
        if e.button() == Qt.MouseButton.LeftButton and e.position().y() < 56:
            h = self.windowHandle()
            if h is not None:
                h.startSystemMove()

    def mouseDoubleClickEvent(self, e):
        if e.position().y() < 56:
            self.showNormal() if self.isMaximized() else self.showMaximized()

    def resizeEvent(self, e):
        super().resizeEvent(e)
        self.dim.setGeometry(self.root.rect())
        self.panel.setFixedHeight(self.root.height())
        if self.panel.isVisible() and self.anim.state() != QtCore.QAbstractAnimation.State.Running:
            self.panel.move(0, 0)

    def logview_toggle(self, on: bool):
        self.logview.setVisible(on)

    def closeEvent(self, e):
        save_cfg(self.cfg)
        super().closeEvent(e)

    # ----- Tabs -----
    def set_tab(self, tab: str):
        self.cfg["tab"] = tab
        self.save_later()
        self.tabs.select(tab)
        page = {"hypr": self.hypr_page, "waybar": self.waybar_page, "launcher": self.launcher_page,
                "other": self.other_page}[tab]
        self.stack.setCurrentWidget({"waybar": self.waybar_gate, "launcher": self.launcher_gate}.get(tab, page))
        if tab == "launcher":  # aktiven Launcher aus den Kürzeln erkennen
            self.launcher_page.autodetect()
        self.refresh_gates()
        if tab == "other":
            if self.cfg.get("other_section", "fix") == "fix":
                page.scan()
        elif self.cfg.get(f"{tab}_section") == "presets":
            page.render_presets()

    # ----- Einstellungen -----
    def toggle_settings(self):
        opening = not self.panel.isVisible() or self.panel.x() < 0
        w = self.panel.width()
        self.anim.stop()
        self._closing = not opening
        if opening:
            blur = QtWidgets.QGraphicsBlurEffect(self.content)
            blur.setBlurRadius(14)
            self.content.setGraphicsEffect(blur)
            self.dim.setGeometry(self.root.rect())
            self.dim.show()
            self.dim.raise_()
            self.panel.pages.setCurrentIndex(0)
            self.panel.sync(self.cfg, self._info())
            self.panel.setFixedHeight(self.root.height())
            self.panel.move(-w, 0)
            self.panel.show()
            self.panel.raise_()
            self.anim.setStartValue(QtCore.QPoint(-w, 0))
            self.anim.setEndValue(QtCore.QPoint(0, 0))
        else:
            self.anim.setStartValue(self.panel.pos())
            self.anim.setEndValue(QtCore.QPoint(-w, 0))
        self.anim.start()

    def _info(self) -> str:
        fmt = model.hypr_format(model.config_home() / "hypr")
        return f"Hyprland: {t('fmt.' + (fmt or 'none'))}" + (f"\n{t('testmode')}" if model.DRY else "")

    def _anim_done(self):
        if not self._closing:
            return
        self.panel.hide()
        self.dim.hide()
        self.content.setGraphicsEffect(None)

    def reset_all(self):
        hdir = model.config_home() / "hypr"
        dlg = ResetDialog(self, model.hypr_format(hdir) in ("lua", None))
        dlg.setStyleSheet(theme.qss(self.accent()))
        if dlg.exec() != QtWidgets.QDialog.DialogCode.Accepted:
            return
        if self.panel.isVisible():
            self.toggle_settings()
        self.log("")
        self.log("== " + t("reset.title") + " ==")
        reset.reset_all(model.config_home(), dlg.factory.isChecked(), self.log)
        defaults = {"hypr": model.hypr_state(None), "waybar": model.waybar_state(None),
                    "launcher": launcher.state(None)}
        for pg in self.all_pages():
            pg.state = defaults[pg.KIND]
            for k in ("preset", "applied_sig", "applied_preset"):
                self.cfg[f"{pg.KIND}_{k}"] = ""
            pg.refresh_controls()
            pg.changed()
        self.launcher_page.preview.ls = self.launcher_page.state
        self.other_page.kitty.reset_state()
        self.save_later()
        self.set_status(t("reset.done"))
        if not self.logview.isVisible():
            self.btn_log.setChecked(True)
        QtCore.QTimer.singleShot(700, lambda: self.other_page.scan(quiet=True))

    def set_language(self, code: str):
        self.cfg["lang"] = code
        set_lang(code)
        _qt_lang(code)
        self.save_later()
        self.retranslate()
        self.panel.sync(self.cfg, self._info())

    def set_format(self, fmt: str):
        """Erweiterte Einstellungen: alte (.conf) oder neue (Lua) Hyprland-Config schreiben."""
        self.cfg["hypr_format"] = fmt
        model.FORMAT = fmt
        self.save_later()
        self.panel.sync(self.cfg, self._info())
        self.hypr_page.update_info()
        self.set_status(t("adv.set", name=t("fmtsw." + fmt)))
        self.log(t("adv.set", name=t("fmtsw." + fmt)))

    def _auto_look(self):
        """Hell/Dunkel bei jedem Start auffrischen: Portal-Datei und Umgebung bleiben aktuell (z. B. nach Updates)."""
        mode = self.cfg.get("appearance")
        if mode in appearance.MODES:
            appearance.write_portals_conf(model.config_home(), self.log)
            appearance.apply_env(mode, model.config_home(), self.log)
            appearance.apply_services(mode, model.config_home(), self.log)

    def _auto_sysactions(self):
        """Herunterfahren/Neustart … in allen Launchern (an, solange der Schalter nicht aus ist)."""
        if self.cfg.get("sys_actions", True):
            sysactions.apply(True, self.log, packages.installed("hyprlock"))

    def _auto_laptop(self):
        """Wie KDE: auf Laptops funktionieren Helligkeit, Lautstärke und Stumm von selbst."""
        hp = self.hypr_page
        was = hp.state["laptop_keys"]
        if laptopkeys.auto(hp.state, self.log):
            hp.changed()
            if hp.state["laptop_keys"] and not was:
                self.log(t("laptop.auto", name=hardware.label(hardware.detect())))

    def _first_look(self):
        """Erster Start: dunkles Farbschema für alle Programme (unter Sonstiges umstellbar)."""
        if "appearance" in self.cfg:
            return
        self.cfg["appearance"] = appearance.DEFAULT
        appearance.apply(appearance.DEFAULT, model.config_home(), self.log)
        self.hypr_page.state["qt_theme"] = appearance.qt_platform_theme()  # kommt beim nächsten Anwenden mit
        self.hypr_page.changed()
        self.log(t("look.first"))
        self.save_later()

    def set_theme(self, name: str):
        self.cfg["theme"] = name
        self.save_later()
        self.apply_theme()
        self.panel.sync(self.cfg, self._info())

    def apply_theme(self):
        QtWidgets.QApplication.instance().setPalette(theme.palette(self.accent()))
        self.setStyleSheet(theme.qss(self.accent()))
        self.gear.setIcon(draw_icon("gear", theme.TEXT, 22))
        self.btn_min.setIcon(draw_icon("min", theme.MUTED, 18))
        self.btn_close.setIcon(draw_icon("close", theme.MUTED, 18))
        for pg in self.all_pages():
            pg.apply_theme(self.accent())
        if hasattr(self, "other_page"):
            self.other_page.apply_theme(self.accent())
        for sw in self.findChildren(Switch):  # wirklich jeder Schalter in der Akzentfarbe
            sw.accent = self.accent()
            sw.update()

    def retranslate(self):
        self.gear.setToolTip(t("settings"))
        self.btn_min.setToolTip(t("minimize"))
        self.btn_close.setToolTip(t("close"))
        self.btn_log.setText(t("log"))
        self.tabs.retranslate()
        for pg in self.all_pages():
            pg.retranslate()
        self.other_page.retranslate()
        self.refresh_gates()
        self.set_status(t("testmode") if model.DRY else t("ready"))


def main(argv) -> int:
    if hasattr(QtGui.QGuiApplication, "setHighDpiScaleFactorRoundingPolicy"):
        QtGui.QGuiApplication.setHighDpiScaleFactorRoundingPolicy(
            Qt.HighDpiScaleFactorRoundingPolicy.PassThrough)
    app = QtWidgets.QApplication(argv)
    app.setApplicationName(hyprstudio.APP_NAME)
    app.setOrganizationName("LabtosoStudios")
    app.setApplicationVersion(hyprstudio.__version__)
    app.setDesktopFileName(hyprstudio.APP_ID)
    app.setWindowIcon(app_icon())
    app.setStyle("Fusion")
    app._round = RoundPopups(app)  # Auswahlmenüs ohne helle Box
    app.installEventFilter(app._round)
    w = MainWindow()
    w.show()
    shot = os.environ.get("HYPRSTUDIO_SHOT")
    if shot:
        steps = os.environ.get("HYPRSTUDIO_STEPS", "")
        QtCore.QTimer.singleShot(300, lambda: _shots(w, app, shot, steps))
    return app.exec()


def _shots(w, app, shot, steps):
    """Testhilfe: HYPRSTUDIO_STEPS='hypr:presets,waybar:modules' → je ein Bild."""
    items = [s for s in steps.split(",") if s] or [""]
    for i, st in enumerate(items):
        if st:
            tab, sec = st.split(":")
            w.set_tab(tab)
            {"hypr": w.hypr_page, "waybar": w.waybar_page, "launcher": w.launcher_page,
             "other": w.other_page}[tab].show_section(sec)
        app.processEvents()
        QtCore.QThread.msleep(50)
        app.processEvents()
        w.grab().save(shot.replace(".png", f"_{i}.png"))
    app.quit()
