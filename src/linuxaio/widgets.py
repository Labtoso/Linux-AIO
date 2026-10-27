"""Gemeinsame Bausteine: Einstellungen, Symbole, Root-Prozess, Chips, Segment-Umschalter, Kacheln, Bildfläche."""

import json
import math
import os
import sys
from pathlib import Path

import linuxaio
from . import system, theme
from .i18n import t
from .qt import QtCore, QtGui, QtWidgets, Qt, Signal

INSTALLED_HELPER = Path("/usr/lib/labtoso-linuxaio/linuxaio-helper")
CONFIG = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config")) / "LabtosoStudios" / "linuxaio.json"
CACHE = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "LabtosoStudios" / "linuxaio"
DRY = os.environ.get("DEMANAGER_DRYRUN") == "1" or sys.platform == "win32"
if DRY:
    os.environ["DEMANAGER_DRYRUN"] = "1"


def load_cfg() -> dict:
    try:
        return json.loads(CONFIG.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return {"lang": "en", "theme": "blue"}      # English by default, German in the settings


def save_cfg(cfg: dict) -> None:
    try:
        CONFIG.parent.mkdir(parents=True, exist_ok=True)
        CONFIG.write_text(json.dumps(cfg), encoding="utf-8")
    except OSError:
        pass


# ---------- gezeichnete Symbole ----------

def draw_icon(kind: str, color: str, size: int = 20) -> QtGui.QIcon:
    pm = QtGui.QPixmap(size * 2, size * 2)
    pm.fill(Qt.GlobalColor.transparent)
    p = QtGui.QPainter(pm)
    p.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
    p.scale(2, 2)
    p.setPen(QtGui.QPen(QtGui.QColor(color), 1.8, Qt.PenStyle.SolidLine, Qt.PenCapStyle.RoundCap))
    c = size / 2
    if kind == "gear":
        teeth, ro, ri = 8, c * 0.9, c * 0.66
        path = QtGui.QPainterPath()
        for i in range(teeth * 4):
            a = (i / (teeth * 4)) * 2 * math.pi - math.pi / 2
            rad = ro if i % 4 in (1, 2) else ri
            pt = QtCore.QPointF(c + rad * math.cos(a), c + rad * math.sin(a))
            path.moveTo(pt) if i == 0 else path.lineTo(pt)
        path.closeSubpath()
        hole = QtGui.QPainterPath()
        hole.addEllipse(QtCore.QPointF(c, c), c * 0.28, c * 0.28)
        p.setPen(Qt.PenStyle.NoPen)
        p.setBrush(QtGui.QColor(color))
        p.drawPath(path.subtracted(hole))
    elif kind == "close":
        m = size * 0.3
        p.drawLine(QtCore.QPointF(m, m), QtCore.QPointF(size - m, size - m))
        p.drawLine(QtCore.QPointF(size - m, m), QtCore.QPointF(m, size - m))
    elif kind == "min":
        m = size * 0.3
        p.drawLine(QtCore.QPointF(m, c), QtCore.QPointF(size - m, c))
    elif kind == "reset":
        p.setBrush(Qt.BrushStyle.NoBrush)
        r = QtCore.QRectF(size * 0.2, size * 0.2, size * 0.6, size * 0.6)
        p.drawArc(r, 60 * 16, 290 * 16)
        tip = QtCore.QPointF(size * 0.65, size * 0.24)
        p.drawLine(tip, QtCore.QPointF(size * 0.80, size * 0.20))
        p.drawLine(tip, QtCore.QPointF(size * 0.70, size * 0.38))
    elif kind == "max":
        m = size * 0.3
        p.drawRoundedRect(QtCore.QRectF(m, m, size - 2 * m, size - 2 * m), 1.5, 1.5)
    elif kind == "chev":
        m = size * 0.3
        p.drawLine(QtCore.QPointF(m, c - size * 0.1), QtCore.QPointF(c, c + size * 0.12))
        p.drawLine(QtCore.QPointF(c, c + size * 0.12), QtCore.QPointF(size - m, c - size * 0.1))
    elif kind == "check":
        p.drawLine(QtCore.QPointF(size * 0.22, c + size * 0.02), QtCore.QPointF(size * 0.42, size * 0.7))
        p.drawLine(QtCore.QPointF(size * 0.42, size * 0.7), QtCore.QPointF(size * 0.8, size * 0.3))
    elif kind == "download":
        p.drawLine(QtCore.QPointF(c, size * 0.18), QtCore.QPointF(c, size * 0.64))
        p.drawLine(QtCore.QPointF(c - size * 0.2, size * 0.45), QtCore.QPointF(c, size * 0.64))
        p.drawLine(QtCore.QPointF(c + size * 0.2, size * 0.45), QtCore.QPointF(c, size * 0.64))
        p.drawLine(QtCore.QPointF(size * 0.22, size * 0.82), QtCore.QPointF(size * 0.78, size * 0.82))
    elif kind == "search":
        p.setBrush(Qt.BrushStyle.NoBrush)
        p.drawEllipse(QtCore.QPointF(c - 1.5, c - 1.5), c * 0.42, c * 0.42)
        p.drawLine(QtCore.QPointF(c + c * 0.25, c + c * 0.25), QtCore.QPointF(size * 0.8, size * 0.8))
    p.end()
    return QtGui.QIcon(pm)


def set_applied(btn: QtWidgets.QPushButton, applied: bool, text: str = "") -> None:
    """Knopf zeigt „Angewendet“ (grüner Rand + Häkchen) statt ausgegraut, wenn der Zustand schon aktiv ist."""
    if bool(btn.property("applied")) != applied:
        btn.setProperty("applied", applied)
        btn.style().unpolish(btn)
        btn.style().polish(btn)
    if applied:
        btn.setText(text)
        ic = draw_icon("check", theme.OK, 16)
        ic.addPixmap(ic.pixmap(32, 32), QtGui.QIcon.Mode.Disabled)   # Häkchen auch bei deaktiviertem Knopf grün
        btn.setIcon(ic)
        btn.setEnabled(False)
    else:
        btn.setIcon(QtGui.QIcon())


def chevron_file(color: str) -> str:
    """Pfeil für Auswahlfelder als Datei (Qt-Stylesheets brauchen eine Bild-URL)."""
    try:
        CACHE.mkdir(parents=True, exist_ok=True)
        f = CACHE / "chevron.png"
        draw_icon("chev", color, 16).pixmap(32, 32).save(str(f))
        return f.as_posix()
    except OSError:
        return ""


def app_icon() -> QtGui.QIcon:
    ic = QtGui.QIcon.fromTheme(linuxaio.APP_ID)
    if not ic.isNull():
        return ic
    f = Path(__file__).parent / "assets" / "linux-aio.svg"
    if f.is_file():
        return QtGui.QIcon(str(f))
    try:  # im .pyz
        import importlib.resources as res
        data = res.files("linuxaio").joinpath("assets/linux-aio.svg").read_bytes()
        pm = QtGui.QPixmap()
        pm.loadFromData(data, "SVG")
        return QtGui.QIcon(pm)
    except Exception:
        return QtGui.QIcon()


# ---------- Root-Prozess (pkexec) ----------

def stage_helper(root: str) -> str:
    """Paket linuxaio (aus Ordner oder .pyz) nach /tmp kopieren und den neuen Suchpfad zurückgeben."""
    import shutil
    import tempfile
    import zipfile
    dest = Path(tempfile.mkdtemp(prefix="linuxaio-helper-"))
    src = Path(root)
    if src.is_dir():
        shutil.copytree(src / "linuxaio", dest / "linuxaio", ignore=shutil.ignore_patterns("__pycache__", "assets"))
    else:
        with zipfile.ZipFile(src) as z:
            for n in z.namelist():
                if n.startswith("linuxaio/") and not n.endswith("/") and "/assets/" not in n:
                    z.extract(n, dest)
    os.chmod(dest, 0o755)
    return str(dest)


class Runner(QtCore.QObject):
    line = Signal(str)
    step = Signal(str)
    progress = Signal(int)
    finished = Signal(bool, str)

    def __init__(self, parent=None):
        super().__init__(parent)
        self.proc = None
        self._err = ""
        self._buf = ""
        self.result = ""

    def busy(self) -> bool:
        return self.proc is not None

    def start(self, args: list) -> None:
        root = str(Path(linuxaio.__file__).resolve().parent.parent)
        self.proc = QtCore.QProcess(self)
        self.proc.setProcessChannelMode(QtCore.QProcess.ProcessChannelMode.MergedChannels)
        if DRY:
            env = QtCore.QProcessEnvironment.systemEnvironment()
            env.insert("PYTHONPATH", root)
            env.insert("DEMANAGER_DRYRUN", "1")
            env.insert("PYTHONIOENCODING", "utf-8")
            if sys.platform == "win32":
                env.insert("DEMANAGER_BACKEND", os.environ.get("DEMANAGER_BACKEND", "pacman"))
            self.proc.setProcessEnvironment(env)
            prog, argv = sys.executable, ["-u", "-m", "linuxaio", "--helper", *args]
        elif INSTALLED_HELPER.is_file():
            prog, argv = "pkexec", [str(INSTALLED_HELPER), *args]
        else:
            root = stage_helper(root)      # root darf FUSE (Cloud, AppImage) nicht lesen → Kopie in /tmp
            py = sys.executable
            if os.environ.get("APPDIR") and py.startswith(os.environ["APPDIR"]) and Path("/usr/bin/python3").is_file():
                py = "/usr/bin/python3"     # eingebautes Python liegt im AppImage-Mount; der Helfer braucht kein Qt
            prog, argv = "pkexec", ["/usr/bin/env", f"PYTHONPATH={root}", py, "-u", "-m", "linuxaio",
                                    "--helper", *args]
        self._err, self._buf, self.result = "", "", ""
        self.proc.readyReadStandardOutput.connect(self._read)
        self.proc.finished.connect(self._done)
        self.line.emit("$ " + " ".join([prog, *argv]))
        self.proc.start(prog, argv)

    def _read(self):
        self._buf += bytes(self.proc.readAllStandardOutput()).decode("utf-8", "replace")
        *lines, self._buf = self._buf.split("\n")
        for ln in lines:
            self._line(ln.rstrip("\r"))

    def _line(self, ln: str):
        if ln.startswith("@@STEP "):
            self.step.emit(ln[7:])
        elif ln.startswith("@@PROGRESS "):
            try:
                self.progress.emit(int(ln[11:]))
            except ValueError:
                pass
        elif ln.startswith("@@FAIL"):
            self._err = ln[7:]
        elif ln.startswith("@@THEME "):
            self.result = ln[8:].strip()
        elif ln.startswith("@@OK"):
            pass
        elif ln:
            self.line.emit(ln)

    def _done(self, code, _status=None):
        self._read()
        if self._buf:
            self._line(self._buf)
        err = self._err or ("cancel" if code in (126, 127) else "")
        self.proc.deleteLater()
        self.proc = None
        self.finished.emit(code == 0, err)


# ---------- Bausteine ----------

def chip(text: str, kind: str = "Chip") -> QtWidgets.QLabel:
    lb = QtWidgets.QLabel(text)
    lb.setObjectName(kind)
    return lb


class ChipRow(QtWidgets.QWidget):
    """Schilder nebeneinander; reicht die Breite nicht, brechen sie in die nächste Zeile um."""

    def __init__(self, parent=None):
        super().__init__(parent)
        self.lay = QtWidgets.QVBoxLayout(self)
        self.lay.setContentsMargins(0, 0, 0, 0)
        self.lay.setSpacing(6)
        self.labels = []
        self._w = 0

    tip_target = None      # set → no badges, status only as tooltip of this widget

    def set(self, chips: list) -> None:
        if self.tip_target is not None:
            lines = [text + (f" – {tip}" if tip else "") for text, _kind, tip in chips]
            base = self.tip_target.property("base_tip") or ""
            self.tip_target.setToolTip("\n".join(([base] if base else []) + lines))
            self.hide()
            return
        for lb in self.labels:
            lb.hide()
            lb.setParent(None)
            lb.deleteLater()
        self.labels = []
        for text, kind, tip in chips:
            lb = chip(text, kind)
            if tip:
                lb.setToolTip(tip)
            self.labels.append(lb)
        self._flow()

    def _flow(self):
        while self.lay.count():
            item = self.lay.takeAt(0)
            if item.layout():
                while item.layout().count():
                    item.layout().takeAt(0)
        width = max(120, self.width() or 280)
        row, used = None, 0
        for lb in self.labels:
            lb.ensurePolished()
            w = lb.sizeHint().width()
            if row is None or (used and used + 6 + w > width):
                row = QtWidgets.QHBoxLayout()
                row.setSpacing(6)
                self.lay.addLayout(row)
                used = 0
            row.addWidget(lb)
            lb.show()
            used += w + (6 if used else 0)
        for i in range(self.lay.count()):
            self.lay.itemAt(i).layout().addStretch()

    def resizeEvent(self, e):
        super().resizeEvent(e)
        if abs(self.width() - self._w) > 4:
            self._w = self.width()
            QtCore.QTimer.singleShot(0, self._flow)


class Segmented(QtWidgets.QFrame):
    changed = Signal(str)

    def __init__(self, keys, parent=None, prefix: str = ""):
        super().__init__(parent)
        self.setObjectName("SegBox")
        self.prefix = prefix
        lay = QtWidgets.QHBoxLayout(self)
        lay.setContentsMargins(3, 3, 3, 3)
        lay.setSpacing(2)
        self.group = QtWidgets.QButtonGroup(self)
        self.btns = {}
        for k in keys:
            b = QtWidgets.QPushButton()
            b.setObjectName("Seg")
            b.setCheckable(True)
            b.setCursor(Qt.CursorShape.PointingHandCursor)
            b.clicked.connect(lambda _=False, k=k: self.changed.emit(k))
            self.group.addButton(b)
            self.btns[k] = b
            lay.addWidget(b)

    def select(self, key: str) -> None:
        if key in self.btns:
            self.btns[key].setChecked(True)

    def retranslate(self):
        for k, b in self.btns.items():
            b.setText(t(self.prefix + k))


class Tile(QtWidgets.QFrame):
    """Kachel mit Bildfläche oben, Name und Status-Punkt."""
    clicked = Signal(str)

    def __init__(self, key: str, title: str, picture: QtWidgets.QWidget, parent=None):
        super().__init__(parent)
        self.key = key
        self.setObjectName("Tile")
        self.setCursor(Qt.CursorShape.PointingHandCursor)
        self.setFixedSize(176, 150)
        lay = QtWidgets.QVBoxLayout(self)
        lay.setContentsMargins(10, 10, 10, 10)
        lay.setSpacing(8)
        self.pic = picture
        self.pic.setParent(self)
        self.pic.setFixedHeight(96)
        self.pic.setAttribute(Qt.WidgetAttribute.WA_TransparentForMouseEvents)
        row = QtWidgets.QHBoxLayout()
        self.name = QtWidgets.QLabel()
        self.name.setStyleSheet("font-weight:600; background:transparent;")
        self.name.setText(QtGui.QFontMetrics(self.name.font()).elidedText(title, Qt.TextElideMode.ElideRight, 134))
        self.dot = QtWidgets.QLabel()
        self.dot.setFixedSize(8, 8)
        row.addWidget(self.name)
        row.addStretch()
        row.addWidget(self.dot)
        lay.addWidget(self.pic)
        lay.addLayout(row)

    def refresh(self, selected: bool, dot: str | None, accent: str) -> None:
        border = accent if selected else theme.BORDER
        bg = theme.SURFACE2 if selected else theme.SURFACE
        self.setStyleSheet(f"#Tile {{ background:{bg}; border:{2 if selected else 1}px solid {border};"
                           f" border-radius:14px; }}")
        self.dot.setStyleSheet(f"background:{dot or 'transparent'}; border-radius:4px;")

    def mouseReleaseEvent(self, e):
        if e.button() == Qt.MouseButton.LeftButton:
            self.clicked.emit(self.key)


class TileStrip(QtWidgets.QScrollArea):
    """Horizontale Kachel-Leiste; meldet, wenn das Ende erreicht ist (zum Nachladen)."""
    reached_end = Signal()

    def __init__(self, parent=None):
        super().__init__(parent)
        self.inner = QtWidgets.QWidget()
        self.inner.setObjectName("TileStrip")
        self.lay = QtWidgets.QHBoxLayout(self.inner)
        self.lay.setContentsMargins(0, 0, 0, 6)
        self.lay.setSpacing(12)
        self.lay.addStretch()
        self.setWidget(self.inner)
        self.setWidgetResizable(True)
        self.setVerticalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
        self.setFixedHeight(172)
        self.viewport().setStyleSheet("background: transparent;")
        self.horizontalScrollBar().valueChanged.connect(self._scrolled)

    def _scrolled(self, v):
        sb = self.horizontalScrollBar()
        if sb.maximum() > 0 and v >= sb.maximum() - 200:
            self.reached_end.emit()

    def clear(self) -> None:
        while self.lay.count() > 1:
            w = self.lay.takeAt(0).widget()
            if w:
                w.hide()
                w.setParent(None)
                w.deleteLater()

    def add(self, w: QtWidgets.QWidget) -> None:
        self.lay.insertWidget(self.lay.count() - 1, w)
        w.show()

    def wheelEvent(self, e):
        # Mausrad scrollt waagerecht (wie in Plasma-Leisten)
        d = e.angleDelta().y() or e.angleDelta().x()
        self.horizontalScrollBar().setValue(self.horizontalScrollBar().value() - d)


class Picture(QtWidgets.QWidget):
    """Bild mit runden Ecken, füllend zugeschnitten; ohne Bild ein gezeichneter Anmeldebildschirm."""

    def __init__(self, parent=None, radius: float = 12.0):
        super().__init__(parent)
        self.radius = radius
        self.pix = None
        self.seed = ""
        self.accent = theme.THEMES["blue"]
        self.loading = False
        self.anim = None          # Bootsplash aus Einzelbildern
        self.movie = None         # animiertes GIF
        self.timer = QtCore.QTimer(self)
        self.timer.timeout.connect(self._tick)

    def _stop(self) -> None:
        self.timer.stop()
        self.anim = None
        if self.movie is not None:
            self.movie.stop()
            self.movie.deleteLater()
            self.movie = None

    def set_pixmap(self, pm) -> None:
        self._stop()
        self.pix = pm if pm is not None and not pm.isNull() else None
        self.loading = False
        self.update()

    def set_placeholder(self, seed: str, accent: str, loading: bool = False, style: str = "login") -> None:
        self._stop()
        self.pix, self.seed, self.accent, self.loading, self.style = None, seed, accent, loading, style
        self.update()

    def set_image_data(self, data: bytes, animate: bool = True) -> None:
        """PNG/JPG als Bild, animiertes GIF wird abgespielt."""
        if data[:4] == b"GIF8" and animate:
            self._stop()
            self.loading = False
            buf = QtCore.QBuffer(self)
            buf.setData(QtCore.QByteArray(data))
            buf.open(QtCore.QIODevice.OpenModeFlag.ReadOnly)
            self.movie = QtGui.QMovie(buf, QtCore.QByteArray(), self)
            self.movie.setCacheMode(QtGui.QMovie.CacheMode.CacheAll)
            self.movie.frameChanged.connect(self._movie_frame)
            self.movie.start()
            return
        pm = QtGui.QPixmap()
        pm.loadFromData(data)
        self.set_pixmap(pm)

    def _movie_frame(self, _n):
        if self.movie is not None and self.isVisible():
            self.pix = self.movie.currentPixmap()
            self.update()

    def set_animation(self, a: dict, animate: bool = True) -> None:
        """a: frames [QPixmap], logo, bgimg (QPixmap|None), bg, bg2 (Farbe), ha, va, wva (0..1)."""
        self._stop()
        self.pix, self.loading = None, False
        self.anim = dict(a, idx=0)
        if animate and len(a["frames"]) > 1:
            self.timer.start(30)          # ~33 Bilder/s wie beim Booten, Schleife über alle Bilder
        self.update()

    def _tick(self):
        if self.anim and self.isVisible():
            self.anim["idx"] = (self.anim["idx"] + 1) % len(self.anim["frames"])
            self.update()

    def _paint_anim(self, p, r):
        a = self.anim
        bg = QtGui.QColor(a["bg"])
        if a.get("bg2"):
            grad = QtGui.QLinearGradient(r.topLeft(), r.bottomLeft())
            grad.setColorAt(0, bg)
            grad.setColorAt(1, QtGui.QColor(a["bg2"]))
            p.fillRect(r, grad)
        else:
            p.fillRect(r, bg)
        if a.get("bgimg") is not None and not a["bgimg"].isNull():
            b = a["bgimg"]
            scale = max(r.width() / b.width(), r.height() / b.height())
            sw, sh = r.width() / scale, r.height() / scale
            p.drawPixmap(r, b, QtCore.QRectF((b.width() - sw) / 2, (b.height() - sh) / 2, sw, sh))
        k = r.width() / 1366.0         # Bilder in Originalgröße auf einem 1366 px breiten Bildschirm

        def place(pm, ha, va, min_h=0.0):
            if pm is None or pm.isNull():
                return
            w, h = pm.width() * k, pm.height() * k
            if min_h and h < r.height() * min_h:                    # zu klein für die Vorschau: vergrößern
                f = min(r.height() * min_h / h, 4.0)
                w, h = w * f, h * f
            if w > r.width() * 0.9 or h > r.height() * 0.6:      # zu groß: einpassen
                f = min(r.width() * 0.9 / w, r.height() * 0.6 / h)
                w, h = w * f, h * f
            x = r.left() + (r.width() - w) * ha
            y = r.top() + (r.height() - h) * va
            p.drawPixmap(QtCore.QRectF(x, y, w, h), pm, QtCore.QRectF(pm.rect()))

        place(a.get("logo"), 0.5, a["wva"], 0.06)
        if a["frames"]:
            place(a["frames"][a["idx"]], a["ha"], a["va"], 0.11)

    def paintEvent(self, _):
        p = QtGui.QPainter(self)
        p.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
        p.setRenderHint(QtGui.QPainter.RenderHint.SmoothPixmapTransform)
        r = QtCore.QRectF(self.rect())
        target_h = r.width() * 10 / 16
        if target_h > r.height():
            tw = r.height() * 16 / 10
            r = QtCore.QRectF(r.left() + (r.width() - tw) / 2, r.top(), tw, r.height())
        else:
            r = QtCore.QRectF(r.left(), r.top() + (r.height() - target_h) / 2, r.width(), target_h)
        clip = QtGui.QPainterPath()
        clip.addRoundedRect(r, self.radius, self.radius)
        p.setClipPath(clip)
        if self.pix:
            pw, ph = self.pix.width(), self.pix.height()
            scale = max(r.width() / pw, r.height() / ph)
            sw, sh = r.width() / scale, r.height() / scale
            src = QtCore.QRectF((pw - sw) / 2, (ph - sh) / 2, sw, sh)
            p.drawPixmap(r, self.pix, src)
        elif self.anim:
            self._paint_anim(p, r)
        elif getattr(self, "style", "login") == "boot":
            self._boot(p, r)
        else:
            self._login(p, r)
        p.end()

    def _boot(self, p, r):
        """Bootsplash: schwarzer Grund, Logo-Ring und Lade-Punkte."""
        p.fillRect(r, QtGui.QColor("#05070a"))
        s = r.height() / 100
        cx, cy = r.center().x(), r.top() + r.height() * 0.44
        hue = sum(map(ord, self.seed or "x")) % 360
        ring = QtGui.QColor.fromHsl(hue, 60, 60)
        p.setBrush(Qt.BrushStyle.NoBrush)
        p.setPen(QtGui.QPen(ring, 2.2 * s))
        p.drawEllipse(QtCore.QPointF(cx, cy), 11 * s, 11 * s)
        p.setPen(QtGui.QPen(QtGui.QColor(self.accent), 2.2 * s, Qt.PenStyle.SolidLine, Qt.PenCapStyle.RoundCap))
        p.drawArc(QtCore.QRectF(cx - 11 * s, cy - 11 * s, 22 * s, 22 * s), 90 * 16, -110 * 16)
        p.setPen(Qt.PenStyle.NoPen)
        for i in range(5):
            a = 230 - i * 40
            p.setBrush(QtGui.QColor(255, 255, 255, max(40, a)))
            p.drawEllipse(QtCore.QPointF(cx - 8 * s + i * 4 * s, cy + 24 * s), 1.3 * s, 1.3 * s)
        if self.loading:
            p.setPen(QtGui.QColor(255, 255, 255, 120))
            f = p.font()
            f.setPointSizeF(max(7.0, 4 * s))
            p.setFont(f)
            p.drawText(r.adjusted(0, 0, 0, -6 * s), Qt.AlignmentFlag.AlignHCenter | Qt.AlignmentFlag.AlignBottom,
                       t("sddm.loading"))

    def _login(self, p, r):
        hue = sum(map(ord, self.seed or "x")) % 360
        base = QtGui.QColor.fromHsl(hue, 70, 46)
        p.fillRect(r, base.darker(260))
        s = r.height() / 100
        p.setPen(Qt.PenStyle.NoPen)
        p.setBrush(base.darker(190))
        p.drawEllipse(QtCore.QPointF(r.left() + r.width() * 0.2, r.bottom()), r.width() * 0.45, r.height() * 0.4)
        p.setBrush(base.darker(160))
        p.drawEllipse(QtCore.QPointF(r.right() - r.width() * 0.1, r.bottom()), r.width() * 0.4, r.height() * 0.32)
        cx, cy = r.center().x(), r.top() + r.height() * 0.42
        p.setBrush(QtGui.QColor(255, 255, 255, 50))
        p.drawEllipse(QtCore.QPointF(cx, cy - 8 * s), 9 * s, 9 * s)
        p.setBrush(QtGui.QColor(0, 0, 0, 110))
        p.drawRoundedRect(QtCore.QRectF(cx - 24 * s, cy + 6 * s, 48 * s, 7 * s), 3.5 * s, 3.5 * s)
        p.setBrush(QtGui.QColor(self.accent))
        p.drawRoundedRect(QtCore.QRectF(cx - 24 * s, cy + 16 * s, 48 * s, 7 * s), 3.5 * s, 3.5 * s)
        p.setBrush(QtGui.QColor(255, 255, 255, 140))
        p.drawRoundedRect(QtCore.QRectF(r.left() + 4 * s, r.top() + 4 * s, 14 * s, 3 * s), 1.5 * s, 1.5 * s)
        if self.loading:
            p.setPen(QtGui.QColor(255, 255, 255, 150))
            f = p.font()
            f.setPointSizeF(max(7.0, 4 * s))
            p.setFont(f)
            p.drawText(r.adjusted(0, 0, 0, -6 * s), Qt.AlignmentFlag.AlignHCenter | Qt.AlignmentFlag.AlignBottom,
                       t("sddm.loading"))


class SettingsPanel(QtWidgets.QFrame):
    lang_changed = Signal(str)
    theme_changed = Signal(str)
    about_requested = Signal()

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setObjectName("Panel")
        self.setFixedWidth(300)
        lay = QtWidgets.QVBoxLayout(self)
        lay.setContentsMargins(22, 22, 22, 22)
        lay.setSpacing(12)
        self.title = QtWidgets.QLabel()
        self.title.setObjectName("PanelTitle")
        lay.addWidget(self.title)
        lay.addSpacing(10)
        self.l_lang = QtWidgets.QLabel()
        self.l_lang.setObjectName("Label")
        lay.addWidget(self.l_lang)
        row = QtWidgets.QHBoxLayout()
        self.lang_btns = {}
        grp = QtWidgets.QButtonGroup(self)
        for code, label in (("de", "Deutsch"), ("en", "English")):
            b = QtWidgets.QPushButton(label)
            b.setObjectName("Lang")
            b.setCheckable(True)
            b.setCursor(Qt.CursorShape.PointingHandCursor)
            b.clicked.connect(lambda _=False, c=code: self.lang_changed.emit(c))
            grp.addButton(b)
            self.lang_btns[code] = b
            row.addWidget(b)
        lay.addLayout(row)
        lay.addSpacing(10)
        self.l_theme = QtWidgets.QLabel()
        self.l_theme.setObjectName("Label")
        lay.addWidget(self.l_theme)
        row2 = QtWidgets.QHBoxLayout()
        row2.setSpacing(12)
        self.theme_btns = {}
        for name in theme.THEMES:
            b = QtWidgets.QPushButton()
            b.setFixedSize(38, 38)
            b.setCursor(Qt.CursorShape.PointingHandCursor)
            b.setToolTip(name.capitalize())
            b.clicked.connect(lambda _=False, n=name: self.theme_changed.emit(n))
            self.theme_btns[name] = b
            row2.addWidget(b)
        row2.addStretch()
        lay.addLayout(row2)
        lay.addStretch()
        self.b_about = QtWidgets.QPushButton()
        self.b_about.setCursor(Qt.CursorShape.PointingHandCursor)
        self.b_about.clicked.connect(self.about_requested.emit)
        lay.addWidget(self.b_about)
        self.info = QtWidgets.QLabel()
        self.info.setObjectName("Label")
        self.info.setWordWrap(True)
        lay.addWidget(self.info)

    def sync(self, cfg: dict, backend: str | None) -> None:
        self.title.setText(t("settings"))
        self.l_lang.setText(t("language"))
        self.l_theme.setText(t("theme"))
        self.b_about.setText(t("about"))
        self.lang_btns[cfg["lang"]].setChecked(True)
        for name, b in self.theme_btns.items():
            ring = "#ffffff" if name == cfg["theme"] else theme.SURFACE
            b.setStyleSheet(f"QPushButton {{ background:{theme.THEMES[name]}; border:3px solid {ring};"
                            f" border-radius:19px; padding:0; }}")
        self.info.setText(f"{system.distro_name()} · {backend or '–'} · v{linuxaio.__version__}\nLabtosoStudios · GPL-3.0")
