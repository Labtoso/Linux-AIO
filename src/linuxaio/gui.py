"""Hauptfenster: Tabs Desktops / SDDM / Plymouth / Einstellungen im Kachel-Galerie-Design."""

import os
import sys

import linuxaio
from . import system, theme, usercmd
from .de_page import DEPage
from .i18n import TEXT, t, set_lang
from .qt import QtCore, QtGui, QtWidgets, Qt, Signal
from .plymouth_page import PlymouthPage
from .sddm_page import SDDMPage
from .settings_page import SettingsPage
from .widgets import DRY, Runner, Segmented, SettingsPanel, app_icon, chevron_file, draw_icon, load_cfg, save_cfg


TABS = ("de", "sddm", "ply", "settings")


class AboutDialog(QtWidgets.QDialog):
    """Über-Dialog im App-Look: rahmenlos, rund, eigene Titelzeile (sieht auf allen Desktops gleich aus)."""

    def __init__(self, win):
        from hyprstudio.dialogs import Frame
        super().__init__(win)
        self.frame = Frame(win, t("about"), 460)
        self.frame.title.hide()
        b = self.frame.body
        b.setSpacing(10)
        top = QtWidgets.QHBoxLayout()
        top.setSpacing(16)
        ic = QtWidgets.QLabel()
        dpr = win.devicePixelRatioF()
        pm = app_icon().pixmap(QtCore.QSize(72, 72), dpr)
        ic.setPixmap(pm)
        top.addWidget(ic)
        col = QtWidgets.QVBoxLayout()
        name = QtWidgets.QLabel(linuxaio.APP_NAME)
        name.setObjectName("DeName")
        ver = QtWidgets.QLabel(f"{t('about.version')} {linuxaio.__version__} · LabtosoStudios")
        ver.setObjectName("Label")
        col.addWidget(name)
        col.addWidget(ver)
        col.addStretch()
        top.addLayout(col, 1)
        b.addLayout(top)
        txt = QtWidgets.QLabel(t("about.text", url=linuxaio.REPO_URL))
        txt.setTextFormat(Qt.TextFormat.RichText)
        txt.setWordWrap(True)
        txt.setOpenExternalLinks(True)
        txt.setStyleSheet(f"a {{ color: {win.accent()}; }}")
        txt.setFixedWidth(410)
        txt.setMinimumHeight(txt.heightForWidth(410) + 6)      # Umbruch-Höhe vorher festlegen (Wayland schnitt ab)
        b.addWidget(txt)
        row = QtWidgets.QHBoxLayout()
        gh = QtWidgets.QPushButton("GitHub")
        gh.setCursor(Qt.CursorShape.PointingHandCursor)
        gh.clicked.connect(lambda: QtGui.QDesktopServices.openUrl(QtCore.QUrl(linuxaio.REPO_URL)))
        ok = QtWidgets.QPushButton(t("close"))
        ok.setObjectName("Primary")
        ok.setCursor(Qt.CursorShape.PointingHandCursor)
        ok.setDefault(True)
        ok.clicked.connect(self.frame.accept)
        row.addWidget(gh)
        row.addStretch()
        row.addWidget(ok)
        b.addLayout(row)

    def exec(self):
        self.frame.adjustSize()
        return self.frame.exec()



class MainWindow(QtWidgets.QWidget):
    logline = Signal(str)       # Protokoll aus Hintergrund-Aufgaben (thread-sicher)

    def __init__(self):
        super().__init__()
        self.cfg = load_cfg()
        if self.cfg.get("version") != linuxaio.__version__:   # each new version starts in English
            self.cfg["lang"] = "en"
            self.cfg["version"] = linuxaio.__version__
            save_cfg(self.cfg)
        set_lang(self.cfg.get("lang", "en"))
        if self.cfg.get("theme") not in theme.THEMES:
            self.cfg["theme"] = "blue"
        if self.cfg.get("tab") not in TABS:
            self.cfg["tab"] = "de"
        self.backend = system.detect_backend() or ("pacman" if DRY else None)
        self.running = system.running_desktop()
        self.runner = Runner(self)
        self.runner.line.connect(self.log)
        self.runner.step.connect(lambda s: self.set_status(t("step." + s)))
        self.runner.progress.connect(self.task_progress)
        self.runner.finished.connect(self._finished)
        self._job = None
        self._task = False

        self.setWindowTitle(linuxaio.APP_NAME)
        self.setWindowIcon(app_icon())
        self.setWindowFlags(Qt.WindowType.FramelessWindowHint | Qt.WindowType.Window)
        self.setAttribute(Qt.WidgetAttribute.WA_TranslucentBackground)
        self.resize(1360, 880)
        self.setMinimumSize(900, 600)   # Seiten mit viel Inhalt scrollen selbst (kein zweiter Scrollbalken außen)

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

        # Kopfzeile: Zahnrad + Titel | Tabs mittig | Fensterknöpfe
        head = QtWidgets.QHBoxLayout()
        left = QtWidgets.QHBoxLayout()
        left.setSpacing(10)
        self.gear = QtWidgets.QToolButton()        # Tux-Logo, öffnet die App-Einstellungen
        self.gear.setIconSize(QtCore.QSize(26, 26))
        self.gear.setFixedSize(36, 36)
        self.gear.setCursor(Qt.CursorShape.PointingHandCursor)
        self.gear.clicked.connect(self.toggle_settings)
        self.title = QtWidgets.QLabel(linuxaio.APP_NAME)
        self.title.setObjectName("Title")
        left.addWidget(self.gear, 0, Qt.AlignmentFlag.AlignVCenter)
        left.addWidget(self.title, 0, Qt.AlignmentFlag.AlignVCenter)
        left.addStretch()
        self.tabs = Segmented(TABS, prefix="tab.")
        self.tabs.setObjectName("TabBox")
        self.tabs.changed.connect(self.set_tab)
        right = QtWidgets.QHBoxLayout()
        right.addStretch()
        self.btn_min = QtWidgets.QToolButton()
        self.btn_min.clicked.connect(self.showMinimized)
        self.btn_max = QtWidgets.QToolButton()
        self.btn_max.clicked.connect(lambda: self.showNormal() if self.isMaximized() else self.showMaximized())
        self.btn_close = QtWidgets.QToolButton()
        self.btn_close.setObjectName("CloseBtn")
        self.btn_close.clicked.connect(self.close)
        right.setSpacing(6)
        for b in (self.btn_min, self.btn_max, self.btn_close):
            b.setIconSize(QtCore.QSize(16, 16))
            b.setFixedSize(32, 32)          # Hover-Fläche quadratisch und mittig
            right.addWidget(b, 0, Qt.AlignmentFlag.AlignVCenter)
        head.addLayout(left, 1)
        head.addWidget(self.tabs)
        head.addLayout(right, 1)
        cl.addLayout(head)

        self.de_page = DEPage(self)
        self.sddm_page = SDDMPage(self)
        self.ply_page = PlymouthPage(self)
        self.stack = QtWidgets.QStackedWidget()
        self.stack.addWidget(self.de_page)
        self.stack.addWidget(self.sddm_page)
        self.stack.addWidget(self.ply_page)
        self.logline.connect(self.log)
        usercmd.set_logger(self.logline.emit)
        self.settings_page = SettingsPage(self)
        self.stack.addWidget(self.settings_page)

        cl.addWidget(self.stack, 1)

        self.footer = QtWidgets.QWidget()
        foot = QtWidgets.QHBoxLayout(self.footer)
        foot.setContentsMargins(0, 0, 0, 0)
        self.status = QtWidgets.QLabel()
        self.status.setObjectName("Status")
        self.bar = QtWidgets.QProgressBar()
        self.bar.setTextVisible(False)
        self.bar.setFixedWidth(220)
        self.bar.setRange(0, 100)
        self.bar.hide()
        self.btn_log = QtWidgets.QPushButton()
        self.btn_log.setCheckable(True)
        self.btn_log.setCursor(Qt.CursorShape.PointingHandCursor)
        self.btn_log.toggled.connect(self._toggle_log)
        foot.addWidget(self.status, 1)
        self.btn_undo = QtWidgets.QPushButton()               # „Rückgängig“ nach Zurücksetzen
        self.btn_undo.setCursor(Qt.CursorShape.PointingHandCursor)
        self.btn_undo.hide()
        self.btn_undo.clicked.connect(self._undo)
        self._undo_fn = None
        foot.addWidget(self.btn_undo)
        foot.addWidget(self.bar)
        foot.addSpacing(12)
        foot.addWidget(self.btn_log)
        cl.addWidget(self.footer)
        # Protokoll als Schublade über den Kacheln (nicht im Layout, verschiebt nichts)
        self.logview = QtWidgets.QPlainTextEdit(self.root)
        self.logview.setObjectName("LogDrawer")
        self.logview.setReadOnly(True)
        self.logview.setMaximumBlockCount(5000)
        self.logview.hide()
        self.log_anim = QtCore.QPropertyAnimation(self.logview, b"geometry", self)
        self.log_anim.setDuration(200)
        self.log_anim.setEasingCurve(QtCore.QEasingCurve.Type.OutCubic)
        self.log_anim.finished.connect(lambda: self.logview.setVisible(self.btn_log.isChecked()))

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
        self.panel.about_requested.connect(self.show_about)
        self._add_app_settings()
        self.anim = QtCore.QPropertyAnimation(self.panel, b"pos", self)
        self.anim.setDuration(220)
        self.anim.setEasingCurve(QtCore.QEasingCurve.Type.OutCubic)
        self.anim.finished.connect(self._anim_done)
        self._closing = False

        self.apply_theme()
        self.retranslate()
        self.sddm_page.rebuild()
        self.ply_page.rebuild()
        self.set_tab(self.cfg["tab"])
        if not self.backend:
            self.set_status(t("nobackend"))
        elif not DRY and not system.has_pkexec():
            self.set_status(t("nopkexec"))

    # ----- Schnittstelle für die Seiten -----
    def accent(self) -> str:
        return theme.THEMES[self.cfg["theme"]]

    def save(self):
        save_cfg(self.cfg)

    def offer_undo(self, text: str, fn):
        self._undo_fn = fn
        self.set_status(text)
        self.btn_undo.setText(t("reset.undo"))
        self.btn_undo.show()

    def _undo(self):
        fn, self._undo_fn = self._undo_fn, None
        self.btn_undo.hide()
        if fn:
            fn()

    def busy(self) -> bool:
        return self.runner.busy() or self._task

    def set_status(self, text: str):
        self.status.setText(text)

    def log(self, text: str):
        self.logview.appendPlainText(text)

    def confirm(self, title: str, text: str) -> bool:
        from hyprstudio import dialogs
        return dialogs.confirm(self, title, text)

    def task_begin(self, text: str):
        self._task = True
        self.bar.setValue(0)
        self.bar.show()
        self.set_status(text)
        self.log("")
        self._refresh_pages()

    def task_progress(self, v: int):
        self.bar.setValue(v)

    def task_end(self, ok: bool, msg: str):
        self._task = False
        self.bar.hide()
        if not ok:
            self.set_status(f"{t('failed')}: {msg}")
            if not self.logview.isVisible():
                self.btn_log.setChecked(True)
        self._refresh_pages()

    def run(self, args: list, action: str, page):
        if self.runner.busy():
            return
        self._job = (action, page)
        self.bar.setValue(0)
        self.bar.show()
        self.set_status(t("busy"))
        self.log("")
        self.runner.start(args)
        self._refresh_pages()

    def _finished(self, ok: bool, err: str):
        self.bar.hide()
        action, page = self._job or ("", None)
        self._job = None
        if ok:
            msg = t("done")
            if action in ("default", "sddm-set"):
                msg += " · " + t("logout")
            elif action in ("dm-enable", "ply-setup", "ply-set"):
                msg += " · " + t("reboot")
            self.set_status(msg)
        else:
            key = err.split(" ", 1)[0] if err else ""
            detail = err.split(" ", 1)[1] if " " in err else ""
            text = t("err." + key) if ("err." + key) in TEXT["de"] else t("failed")
            self.set_status(f"{t('failed')}: {text}" + (f" ({detail})" if detail else ""))
            if not self.logview.isVisible():
                self.btn_log.setChecked(True)
        if page:
            page.finished(ok, action, self.runner.result)
        self._refresh_pages()

    def _refresh_pages(self):
        self.de_page.refresh()
        self.sddm_page.refresh()
        self.ply_page.refresh()
        self.settings_page.refresh()

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
        if self.btn_log.isChecked() and self.log_anim.state() != QtCore.QAbstractAnimation.State.Running:
            QtCore.QTimer.singleShot(0, lambda: self.logview.setGeometry(self._log_rect(True)))

    def _log_rect(self, open_: bool) -> QtCore.QRect:
        foot = self.footer.geometry().translated(self.content.pos())
        bottom = foot.top() - 8
        page = self.stack.currentWidget()
        anchor = page.drawer_anchor()            # erstes Element unter der großen Karte
        top = anchor.mapTo(self.root, QtCore.QPoint(0, 0)).y()
        x, w = foot.left(), foot.width()
        return QtCore.QRect(x, top, w, bottom - top) if open_ else QtCore.QRect(x, bottom, w, 0)

    def _toggle_log(self, on: bool):
        self.log_anim.stop()
        self.log_anim.setStartValue(self.logview.geometry() if self.logview.isVisible() else self._log_rect(False))
        self.log_anim.setEndValue(self._log_rect(on))
        if on:
            self.logview.show()
            self.logview.raise_()
            if self.panel.isVisible():
                self.dim.raise_()
                self.panel.raise_()
        self.log_anim.start()

    # ----- Tabs -----
    def set_tab(self, tab: str):
        self.cfg["tab"] = tab
        save_cfg(self.cfg)
        self.tabs.select(tab)
        self.stack.setCurrentWidget({"de": self.de_page, "sddm": self.sddm_page, "ply": self.ply_page,
                                     "settings": self.settings_page}[tab])
        if self.btn_log.isChecked():
            QtCore.QTimer.singleShot(0, lambda: self.logview.setGeometry(self._log_rect(True)))

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
            self.panel.sync(self.cfg, self.backend)
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

    def _anim_done(self):
        if not self._closing:
            return
        self.panel.hide()
        self.dim.hide()
        self.content.setGraphicsEffect(None)

    def _add_app_settings(self):
        """Tux › app settings: interface size and Hypr Studio mode (besides language, colour, About)."""
        from .app_page import restart
        lay = self.panel.layout()
        pos = lay.indexOf(self.panel.b_about) - 1      # direkt unter den Farben, vor dem Abstand
        self.l_scale = QtWidgets.QLabel()
        self.l_scale.setObjectName("Label")
        self.seg_scale = Segmented(("100", "90", "80"), prefix="app.scale.")
        self.seg_scale.select(str(self.cfg.get("ui_scale", "100")))
        self.seg_scale.changed.connect(self.set_ui_scale)
        self.l_hypr = QtWidgets.QLabel()
        self.l_hypr.setObjectName("Label")
        self.seg_hypr = Segmented(("embed", "window"), prefix="app.hypr.")
        self.seg_hypr.select(self.cfg.get("hypr_mode", "embed"))
        self.seg_hypr.changed.connect(lambda v: self._app_opt("hypr_mode", v))
        self.b_restart = QtWidgets.QPushButton()
        self.b_restart.setObjectName("Primary")
        self.b_restart.setCursor(Qt.CursorShape.PointingHandCursor)
        self.b_restart.clicked.connect(restart)
        self.b_restart.hide()
        lay.insertSpacing(pos, 10)
        for i, w in enumerate((self.l_scale, self.seg_scale, self.l_hypr, self.seg_hypr, self.b_restart)):
            lay.insertWidget(pos + 1 + i, w)

    def _app_opt(self, key, value):
        self.cfg[key] = value
        self.save()
        self.b_restart.show()
        self.set_status(t("app.restart.need"))

    def show_about(self):
        AboutDialog(self).exec()

    def set_language(self, code: str):
        self.cfg["lang"] = code
        set_lang(code)
        save_cfg(self.cfg)
        self.retranslate()
        self.panel.sync(self.cfg, self.backend)

    def set_theme(self, name: str):
        self.cfg["theme"] = name
        save_cfg(self.cfg)
        self.apply_theme()
        self._refresh_pages()
        self.settings_page.apply_accent()
        self.panel.sync(self.cfg, self.backend)

    def ui_factor(self) -> float:
        try:
            return int(self.cfg.get("ui_scale", "100")) / 100
        except ValueError:
            return 1.0

    def set_ui_scale(self, value: str):
        """Live, both directions, all open windows – always computed from the base sizes."""
        self.cfg["ui_scale"] = value
        self.save()
        self.apply_theme()
        for w in self._hypr_windows():
            w.apply_theme()
        self.set_status(t("app.scale") + f": {value} %")

    def _hypr_windows(self):
        from .hypr_embed import HyprEmbed
        out = list(self.findChildren(HyprEmbed))
        pop = getattr(self, "_hypr_window", None)
        if pop is not None:
            out.append(pop)
        return out

    def apply_theme(self):
        from hyprstudio import theme as hs_theme
        app = QtWidgets.QApplication.instance()
        if not hasattr(app, "_base_pt"):
            app._base_pt = app.font().pointSizeF()
        f = app.font()
        f.setPointSizeF(app._base_pt * self.ui_factor())
        app.setFont(f)
        QtWidgets.QApplication.instance().setPalette(hs_theme.palette(self.accent()))   # dunkle Palette überall
        chev = chevron_file(theme.MUTED)
        extra = f"QComboBox#Combo::down-arrow {{ image: url({chev}); width: 14px; height: 14px; }}" if chev else ""
        self.setStyleSheet(theme.scale_qss(theme.qss(self.accent()) + extra, self.ui_factor()))
        self.gear.setIcon(app_icon())
        self.btn_min.setIcon(draw_icon("min", theme.MUTED, 16))
        self.btn_max.setIcon(draw_icon("max", theme.MUTED, 16))
        self.btn_close.setIcon(draw_icon("close", theme.MUTED, 16))

    def retranslate(self):
        self.gear.setToolTip(t("app.settings"))
        if hasattr(self, "seg_scale"):
            self.l_scale.setText(t("app.scale"))
            self.l_hypr.setText(t("app.hypr"))
            self.b_restart.setText(t("app.restart"))
            self.seg_scale.retranslate()
            self.seg_hypr.retranslate()
        self.btn_min.setToolTip(t("minimize"))
        self.btn_max.setToolTip(t("maximize"))
        self.btn_close.setToolTip(t("close"))
        self.btn_log.setText(t("log"))
        self.tabs.retranslate()
        self.de_page.retranslate()
        self.sddm_page.retranslate()
        self.ply_page.retranslate()
        self.settings_page.retranslate()
        if not self.busy():
            self.set_status(t("ready"))


def main(argv) -> int:
    if hasattr(QtGui.QGuiApplication, "setHighDpiScaleFactorRoundingPolicy"):
        QtGui.QGuiApplication.setHighDpiScaleFactorRoundingPolicy(
            Qt.HighDpiScaleFactorRoundingPolicy.PassThrough)
    # Plattform-Themes (kde, gtk3, qt6ct) würden Dialoge/Farben verändern → aus, eigenes Design überall
    for var in ("QT_QPA_PLATFORMTHEME", "QT_STYLE_OVERRIDE"):
        os.environ.pop(var, None)
    os.environ.setdefault("QT_SCALE_FACTOR_ROUNDING_POLICY", "PassThrough")

    app = QtWidgets.QApplication(argv)
    app.setApplicationName(linuxaio.APP_NAME)
    app.setOrganizationName("LabtosoStudios")
    app.setApplicationVersion(linuxaio.__version__)
    app.setApplicationDisplayName(linuxaio.APP_NAME)
    app.setDesktopFileName(linuxaio.APP_ID)   # Wayland: richtiges Icon in der Plasma-Taskleiste
    app.setWindowIcon(app_icon())
    app.setStyle("Fusion")   # überall gleich (kein Breeze/GTK-Stil vom System)
    from hyprstudio.widgets import RoundPopups
    app._round = RoundPopups(app)       # Auswahlmenüs rund, ohne helle Box (wie Hypr Studio)
    app.installEventFilter(app._round)
    from .form import Job
    app.aboutToQuit.connect(Job.wait_all)
    w = MainWindow()
    w.show()
    QtCore.QTimer.singleShot(800, _integrate)
    shot = os.environ.get("DEMANAGER_SHOT")
    if shot:
        QtCore.QTimer.singleShot(600, lambda: _shots(w, app, shot, os.environ.get("LINUXAIO_STEPS", "")))
    code = app.exec()
    if any(j.isRunning() for j in Job.running):
        os._exit(code)          # Download hängt noch: hart beenden statt Qt-Absturz beim Aufräumen
    return code


def _integrate():
    """AppImage/.pyz: start menu entry + icon with the window's app ID (right icon in the GNOME dock)."""
    from . import integrate
    from .qt import QtSvg

    def render(size, path):
        img = QtGui.QImage(size, size, QtGui.QImage.Format.Format_ARGB32)
        img.fill(0)
        p = QtGui.QPainter(img)
        p.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
        data = _icon_svg()
        if data:
            QtSvg.QSvgRenderer(QtCore.QByteArray(data)).render(p, QtCore.QRectF(0, 0, size, size))
        p.end()
        img.save(str(path), "PNG")
    integrate.ensure(render)


def _icon_svg() -> bytes:
    from pathlib import Path
    f = Path(__file__).parent / "assets" / "linux-aio.svg"
    if f.is_file():
        return f.read_bytes()
    try:
        import importlib.resources as res
        return res.files("linuxaio").joinpath("assets/linux-aio.svg").read_bytes()
    except Exception:  # noqa: BLE001
        return b""


def _shots(w, app, shot, steps):
    """Testhilfe: LINUXAIO_STEPS='settings:gnome:cursor,sddm' → je ein Bild (Tab[:Desktop[:Bereich]])."""
    items = [s for s in steps.split(",") if s] or [""]
    for i, st in enumerate(items):
        if st:
            parts = st.split(":")
            w.set_tab(parts[0])
            if parts[0] == "settings" and len(parts) > 1:
                w.settings_page.select(parts[1])
                if len(parts) > 2 and w.settings_page.panel():
                    w.settings_page.panel().show_section(parts[2])
        for _ in range(int(os.environ.get("LINUXAIO_WAIT", "12"))):   # Hintergrund-Aufgaben (Vorschauen) abwarten
            app.processEvents()
            QtCore.QThread.msleep(100)
        app.processEvents()
        w.grab().save(shot if len(items) == 1 else shot.replace(".png", f"_{i}.png"))
    app.quit()
