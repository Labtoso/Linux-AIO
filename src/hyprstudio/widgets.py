"""Bausteine: Einstellungen speichern, Symbole, Schalter, Regler-Zeilen, Farbwahl, Preset-Karten, Modul-Spalten."""

import json
import math
import os
import weakref
from pathlib import Path

import hyprstudio
from . import fonts, model, theme
from .i18n import t
from .qt import QtCore, QtGui, QtWidgets, Qt, Signal

CONFIG = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config")) / "LabtosoStudios" / "hyprstudio.json"


def load_cfg() -> dict:
    try:
        return json.loads(CONFIG.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return {"lang": "en", "theme": "blue"}  # neu installiert: Englisch (Deutsch im Zahnrad-Menü)


def save_cfg(cfg: dict) -> None:
    try:
        CONFIG.parent.mkdir(parents=True, exist_ok=True)
        CONFIG.write_text(json.dumps(cfg, ensure_ascii=False), encoding="utf-8")
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
    elif kind == "dots":
        p.setPen(Qt.PenStyle.NoPen)
        p.setBrush(QtGui.QColor(color))
        for x in (0.25, 0.5, 0.75):
            p.drawEllipse(QtCore.QPointF(size * x, c), size * 0.08, size * 0.08)
    elif kind == "play":
        p.setPen(Qt.PenStyle.NoPen)
        p.setBrush(QtGui.QColor(color))
        p.drawPolygon(QtGui.QPolygonF([QtCore.QPointF(size * 0.32, size * 0.22), QtCore.QPointF(size * 0.8, c),
                                       QtCore.QPointF(size * 0.32, size * 0.78)]))
    p.end()
    return QtGui.QIcon(pm)


def app_icon() -> QtGui.QIcon:
    ic = QtGui.QIcon.fromTheme(hyprstudio.APP_ID)
    if not ic.isNull():
        return ic
    f = Path(__file__).parent / "assets" / "hyprstudio.svg"
    if f.is_file():
        return QtGui.QIcon(str(f))
    try:  # im .pyz
        import importlib.resources as res
        data = res.files("hyprstudio").joinpath("assets/hyprstudio.svg").read_bytes()
        pm = QtGui.QPixmap()
        pm.loadFromData(data, "SVG")
        return QtGui.QIcon(pm)
    except Exception:
        return QtGui.QIcon()


# ---------- Grundbausteine ----------

def set_chip(lb: QtWidgets.QLabel, ok: bool, text: str) -> None:
    """Status als schlichter Text mit farbigem Punkt (grün = ok, gelb = Hinweis); Stil direkt am Widget."""
    col = theme.OK if ok else "#d29922"
    lb.setTextFormat(Qt.TextFormat.RichText)
    lb.setText(f'<span style="color:{col}">●</span>&nbsp;&nbsp;{text}')
    lb.setStyleSheet(f"QLabel {{ background: transparent; border: none; padding: 2px 4px 2px 0;"
                     f" color: {theme.MUTED}; font-size: 9pt; }}")
    lb.setMinimumSize(0, 0)
    lb.setMaximumSize(16777215, 16777215)
    lb.adjustSize()
    lb.setFixedSize(lb.sizeHint())

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

    def select(self, key) -> None:
        if key in self.btns:
            self.btns[key].setChecked(True)

    def retranslate(self):
        for k, b in self.btns.items():
            b.setText(t(f"{self.prefix}{k}").replace("&", "&&"))
            f = QtGui.QFont(b.font())
            f.setWeight(QtGui.QFont.Weight.DemiBold)
            b.setMinimumWidth(int(QtGui.QFontMetrics(f).horizontalAdvance(b.text())) + 30)


class Switch(QtWidgets.QAbstractButton):
    def __init__(self, parent=None):
        super().__init__(parent)
        self.setCheckable(True)
        self.setCursor(Qt.CursorShape.PointingHandCursor)
        self.setFixedSize(42, 24)
        self.accent = theme.THEMES["blue"]
        self._pos = 0.0
        self._anim = QtCore.QVariantAnimation(self)
        self._anim.setDuration(140)
        self._anim.valueChanged.connect(self._move)
        self.toggled.connect(self._animate)

    def _move(self, v):
        self._pos = float(v)
        self.update()

    def _animate(self, on):
        self._anim.stop()
        self._anim.setStartValue(self._pos)
        self._anim.setEndValue(1.0 if on else 0.0)
        self._anim.start()

    def set_now(self, on: bool):
        self._anim.stop()
        self.blockSignals(True)
        self.setChecked(on)
        self.blockSignals(False)
        self._pos = 1.0 if on else 0.0
        self.update()

    def paintEvent(self, _):
        p = QtGui.QPainter(self)
        p.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
        r = QtCore.QRectF(self.rect()).adjusted(1, 1, -1, -1)
        off, on = QtGui.QColor(theme.SURFACE2), QtGui.QColor(self.accent)
        c = QtGui.QColor(int(off.red() + (on.red() - off.red()) * self._pos),
                         int(off.green() + (on.green() - off.green()) * self._pos),
                         int(off.blue() + (on.blue() - off.blue()) * self._pos))
        p.setPen(QtGui.QPen(QtGui.QColor(theme.BORDER) if self._pos < 0.5 else c, 1))
        p.setBrush(c)
        p.drawRoundedRect(r, r.height() / 2, r.height() / 2)
        d = r.height() - 6
        x = r.left() + 3 + (r.width() - d - 6) * self._pos
        p.setPen(Qt.PenStyle.NoPen)
        # Knopf: auf heller Akzentfarbe (Weiß, Mint) dunkel, sonst weiß – sonst ist er nicht zu sehen
        p.setBrush(QtGui.QColor(theme.on_accent(self.accent) if self._pos >= 0.5 else "#ffffff"))
        p.drawEllipse(QtCore.QRectF(x, r.top() + 3, d, d))
        p.end()


def _checker(p: QtGui.QPainter, r: QtCore.QRectF, s: float = 5):
    p.fillRect(r, QtGui.QColor("#d0d6de"))
    y, row = r.top(), 0
    while y < r.bottom():
        x = r.left() + (s if row % 2 else 0)
        while x < r.right():
            p.fillRect(QtCore.QRectF(x, y, s, s).intersected(r), QtGui.QColor("#9aa4b1"))
            x += 2 * s
        y += s
        row += 1


class ColorButton(QtWidgets.QPushButton):
    """Farbfeld mit Alpha (Schachbrett dahinter), öffnet einen Farbwähler mit Live-Vorschau."""
    color_changed = Signal(str)

    def __init__(self, parent=None):
        super().__init__(parent)
        self.color = "#3d8bfdff"
        self.setFixedSize(64, 30)
        self.setCursor(Qt.CursorShape.PointingHandCursor)
        self.clicked.connect(self.pick)

    def set_color(self, c: str):
        self.color = model.norm(c)
        self.setToolTip(self.color)
        self.update()

    def pick(self):
        from .preview import qcolor
        old = self.color
        from . import dialogs
        c = dialogs.pick_color(self, qcolor(old), t("color.pick"), live=self._live)
        if c is not None:
            self._live(c)
        else:
            self.set_color(old)
            self.color_changed.emit(old)

    def _live(self, c: QtGui.QColor):
        self.set_color(f"#{c.red():02x}{c.green():02x}{c.blue():02x}{c.alpha():02x}")
        self.color_changed.emit(self.color)

    def paintEvent(self, _):
        from .preview import qcolor
        p = QtGui.QPainter(self)
        p.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
        r = QtCore.QRectF(self.rect()).adjusted(1, 1, -1, -1)
        path = QtGui.QPainterPath()
        path.addRoundedRect(r, 9, 9)
        p.setClipPath(path)
        _checker(p, r)
        p.fillRect(r, qcolor(self.color))
        p.setClipping(False)
        p.setPen(QtGui.QPen(QtGui.QColor(theme.TEXT if self.underMouse() else theme.BORDER), 1))
        p.setBrush(Qt.BrushStyle.NoBrush)
        p.drawRoundedRect(r, 9, 9)
        p.end()


# ---------- Karten mit Zeilen ----------

class Card(QtWidgets.QFrame):
    """Karte mit Titel und Zeilen (Beschriftung | Steuerelement | Wert)."""

    def __init__(self, title_key: str, page, parent=None):
        super().__init__(parent)
        self.setObjectName("Card")
        self.page = page
        self.title_key = title_key
        self.rows = []
        lay = QtWidgets.QVBoxLayout(self)
        lay.setContentsMargins(18, 14, 18, 16)
        lay.setSpacing(10)
        self.title = QtWidgets.QLabel()
        self.title.setObjectName("CardTitle")
        lay.addWidget(self.title)
        self.grid = QtWidgets.QGridLayout()
        self.grid.setHorizontalSpacing(14)
        self.grid.setVerticalSpacing(10)
        self.grid.setColumnStretch(1, 1)
        lay.addLayout(self.grid)

    def _label(self, key: str, tip: str | None = None) -> QtWidgets.QLabel:
        lb = QtWidgets.QLabel()
        lb.setObjectName("RowLabel")
        lb.setProperty("tkey", key)
        if tip:
            lb.setProperty("tipkey", tip)
        lb.setMinimumWidth(130)
        return lb

    def _add(self, key, widget, value=None, tip=None, refresh=None, label_key=None):
        r = self.grid.rowCount()
        lb = self._label(label_key or key, tip)
        self.grid.addWidget(lb, r, 0)
        if value is None:
            self.grid.addWidget(widget, r, 1, 1, 2, Qt.AlignmentFlag.AlignLeft)
        else:
            self.grid.addWidget(widget, r, 1)
            self.grid.addWidget(value, r, 2)
        self.rows.append((lb, refresh))
        return lb

    def slider(self, key, lo, hi, step=1.0, fmt="{:.0f}", tip=None, label_key=None):
        sl = QtWidgets.QSlider(Qt.Orientation.Horizontal)
        sl.setRange(int(round(lo / step)), int(round(hi / step)))
        sl.setCursor(Qt.CursorShape.PointingHandCursor)
        val = QtWidgets.QLabel()
        val.setObjectName("Value")
        val.setMinimumWidth(44)
        val.setAlignment(Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignVCenter)
        is_float = isinstance(self.page.state[key], float)

        def changed(i):
            v = i * step
            v = round(v, 4) if is_float else int(round(v))
            val.setText(fmt.format(v))
            self.page.set(key, v)

        def refresh(s):
            sl.blockSignals(True)
            sl.setValue(int(round(s[key] / step)))
            sl.blockSignals(False)
            val.setText(fmt.format(s[key]))
        sl.valueChanged.connect(changed)
        self._add(key, sl, val, tip, refresh, label_key=label_key)
        return sl

    def toggle(self, key, tip=None, label_key=None):
        sw = Switch()
        sw.toggled.connect(lambda on: self.page.set(key, bool(on)))
        self._add(key, sw, None, tip, lambda s: sw.set_now(bool(s[key])), label_key=label_key)
        self.page.switches.append(sw)
        return sw

    def color(self, key, label_key=None, tip=None):
        cb = ColorButton()
        cb.color_changed.connect(lambda c: self.page.set(key, c))
        self._add(key, cb, None, tip, lambda s: cb.set_color(s[key]), label_key=label_key)
        return cb

    def choice(self, key, options, prefix, tip=None, label_key=None):
        seg = Segmented(options, prefix=prefix)
        seg.changed.connect(lambda v: self.page.set(key, int(v) if isinstance(self.page.state[key], int) else v))
        self._add(key, seg, None, tip, lambda s: seg.select(str(s[key])), label_key=label_key)
        self.page.segs.append(seg)
        return seg

    def text(self, key, tip=None, width=220, label_key=None):
        le = QtWidgets.QLineEdit()
        le.setFixedWidth(width)
        le.textEdited.connect(lambda v: self.page.set(key, v.strip()))

        def refresh(s):
            if le.text().strip() != s[key]:
                le.setText(s[key])
        self._add(key, le, None, tip, refresh, label_key=label_key)
        return le

    def font(self, key, tip=None, label_key=None):
        """Schriftart: Auswahl mit Suche, fehlende Nerd Fonts direkt installierbar."""
        fc = FontCombo()
        fc.chosen.connect(lambda f: self.page.set(key, f))
        self._add(key, fc, None, tip, lambda s: fc.fill(s[key]), label_key=label_key)
        return fc

    def lines(self, key, tip=None):
        """Mehrzeiliges Feld: eine Zeile = ein Listeneintrag."""
        ed = QtWidgets.QPlainTextEdit()
        ed.setFixedHeight(110)
        ed.setMinimumWidth(380)
        ed.textChanged.connect(lambda: self.page.set(key, [x.strip() for x in ed.toPlainText().splitlines()
                                                           if x.strip()]))

        def refresh(s):
            if [x.strip() for x in ed.toPlainText().splitlines() if x.strip()] != s[key]:
                ed.blockSignals(True)
                ed.setPlainText("\n".join(s[key]))
                ed.blockSignals(False)
        self._add(key, ed, None, tip, refresh)
        return ed

    def refresh(self, s):
        for _, fn in self.rows:
            if fn:
                fn(s)

    def retranslate(self):
        self.title.setText(t(self.title_key))
        for lb, _ in self.rows:
            if not lb.property("tkey"):
                continue
            lb.setText(t(lb.property("tkey")))
            tip = lb.property("tipkey")
            if tip:
                lb.setToolTip(t(tip))


class PresetCard(QtWidgets.QFrame):
    clicked = Signal(str)

    def __init__(self, key: str, parent=None):
        super().__init__(parent)
        self.key = key
        self.setObjectName("Preset")
        self.setCursor(Qt.CursorShape.PointingHandCursor)
        self.setFixedSize(212, 160)
        lay = QtWidgets.QVBoxLayout(self)
        lay.setContentsMargins(8, 8, 8, 10)
        lay.setSpacing(8)
        self.pic = QtWidgets.QLabel()
        self.pic.setFixedSize(196, 110)
        row = QtWidgets.QHBoxLayout()
        row.setContentsMargins(0, 0, 0, 0)
        self.name = QtWidgets.QLabel()
        self.name.setStyleSheet("font-weight:600; background:transparent; padding-left:4px;")
        self.chip = QtWidgets.QLabel()
        self.chip.setObjectName("ChipOk")
        self.chip.hide()
        row.addWidget(self.name)
        row.addStretch()
        row.addWidget(self.chip)
        lay.addWidget(self.pic)
        lay.addLayout(row)

    def set_pixmap(self, pm):
        self.pic.setPixmap(pm)

    def refresh(self, selected: bool, accent: str, applied: bool = False):
        self.chip.setText(t("applied"))
        self.chip.setVisible(applied)
        border = accent if selected else theme.BORDER
        bg = theme.SURFACE2 if selected else theme.SURFACE
        self.setStyleSheet(f"#Preset {{ background:{bg}; border:{2 if selected else 1}px solid {border};"
                           f" border-radius:14px; }}")

    def mouseReleaseEvent(self, e):
        if e.button() == Qt.MouseButton.LeftButton:
            self.clicked.emit(self.key)


class FlowGrid(QtWidgets.QWidget):
    """Einfaches Raster, das die Spaltenzahl der Breite anpasst."""

    def __init__(self, cell_w: int, parent=None):
        super().__init__(parent)
        self.cell_w = cell_w
        self.items = []
        self.grid = QtWidgets.QGridLayout(self)
        self.grid.setContentsMargins(0, 0, 0, 0)
        self.grid.setSpacing(14)
        self._cols = 0

    def add(self, w):
        self.items.append(w)
        self._relayout(force=True)

    def _relayout(self, force=False):
        cols = max(1, (self.width() + 14) // (self.cell_w + 14))
        if cols == self._cols and not force:
            return
        self._cols = cols
        for w in self.items:
            self.grid.removeWidget(w)
        for i, w in enumerate(self.items):
            self.grid.addWidget(w, i // cols, i % cols, Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignTop)
        for c in range(cols + 1):
            self.grid.setColumnStretch(c, 1 if c == cols else 0)

    def resizeEvent(self, e):
        super().resizeEvent(e)
        self._relayout()


class ModuleColumns(QtWidgets.QWidget):
    """Vier Listen (Verfügbar, Links, Mitte, Rechts), Module per Ziehen verschieben."""
    changed = Signal()

    SIDES = ("avail", "left", "center", "right")

    def __init__(self, parent=None):
        super().__init__(parent)
        lay = QtWidgets.QHBoxLayout(self)
        lay.setContentsMargins(0, 0, 0, 0)
        lay.setSpacing(10)
        self.lists, self.heads = {}, {}
        self._loading = False
        for side in self.SIDES:
            col = QtWidgets.QVBoxLayout()
            col.setSpacing(6)
            h = QtWidgets.QLabel()
            h.setObjectName("RowLabel")
            lw = QtWidgets.QListWidget()
            lw.setObjectName("ModList")
            lw.setDragDropMode(QtWidgets.QAbstractItemView.DragDropMode.DragDrop)
            lw.setDefaultDropAction(Qt.DropAction.MoveAction)
            lw.setSelectionMode(QtWidgets.QAbstractItemView.SelectionMode.SingleSelection)
            lw.setMinimumHeight(250)
            m = lw.model()
            m.rowsInserted.connect(self._later)
            m.rowsRemoved.connect(self._later)
            m.rowsMoved.connect(self._later)
            col.addWidget(h)
            col.addWidget(lw, 1)
            lay.addLayout(col)
            self.lists[side], self.heads[side] = lw, h
        self._timer = QtCore.QTimer(self)
        self._timer.setSingleShot(True)
        self._timer.setInterval(0)
        self._timer.timeout.connect(self.changed.emit)

    def _later(self, *_):
        if not self._loading:
            self._timer.start()

    def load(self, s: dict):
        self._loading = True
        used = set(s["left"] + s["center"] + s["right"])
        data = {"avail": [m for m in model.MODULES if m not in used],
                "left": s["left"], "center": s["center"], "right": s["right"]}
        for side, lw in self.lists.items():
            lw.clear()
            for m in data[side]:
                it = QtWidgets.QListWidgetItem(t("m." + m))
                it.setData(Qt.ItemDataRole.UserRole, m)
                lw.addItem(it)
        self._loading = False

    def read(self) -> dict:
        out = {}
        for side in ("left", "center", "right"):
            lw = self.lists[side]
            seen = []
            for i in range(lw.count()):
                m = lw.item(i).data(Qt.ItemDataRole.UserRole)
                if m in model.MODULES and m not in seen:
                    seen.append(m)
            out[side] = seen
        return out

    def retranslate(self):
        for side, h in self.heads.items():
            h.setText(t("col." + side))
        for lw in self.lists.values():
            for i in range(lw.count()):
                it = lw.item(i)
                it.setText(t("m." + it.data(Qt.ItemDataRole.UserRole)))
        self.setToolTip(t("tip.modules"))


class SettingsPanel(QtWidgets.QFrame):
    lang_changed = Signal(str)
    theme_changed = Signal(str)
    reset_requested = Signal()
    format_changed = Signal(str)

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setObjectName("Panel")
        self.setFixedWidth(300)
        outer = QtWidgets.QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        self.pages = QtWidgets.QStackedWidget()
        outer.addWidget(self.pages)
        main = QtWidgets.QWidget()
        self.pages.addWidget(main)
        lay = QtWidgets.QVBoxLayout(main)
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
            b.setProperty("tipkey", "accent." + name)
            b.clicked.connect(lambda _=False, n=name: self.theme_changed.emit(n))
            self.theme_btns[name] = b
            row2.addWidget(b)
        row2.addStretch()
        lay.addLayout(row2)
        lay.addStretch()
        self.l_reset = QtWidgets.QLabel()
        self.l_reset.setObjectName("Label")
        lay.addWidget(self.l_reset)
        self.btn_reset = QtWidgets.QPushButton()
        self.btn_reset.setObjectName("DangerFill")
        self.btn_reset.setCursor(Qt.CursorShape.PointingHandCursor)
        self.btn_reset.setMinimumHeight(40)
        self.btn_reset.clicked.connect(self.reset_requested.emit)
        lay.addWidget(self.btn_reset)
        lay.addSpacing(8)
        self.btn_adv = QtWidgets.QPushButton()
        self.btn_adv.setObjectName("Outline")
        self.btn_adv.setCursor(Qt.CursorShape.PointingHandCursor)
        self.btn_adv.setMinimumHeight(40)
        self.btn_adv.clicked.connect(lambda: self.pages.setCurrentIndex(1))
        lay.addWidget(self.btn_adv)
        lay.addSpacing(14)
        self.info = QtWidgets.QLabel()
        self.info.setObjectName("Label")
        self.info.setWordWrap(True)
        lay.addWidget(self.info)
        self._build_advanced()

    def _build_advanced(self):
        """Erweiterte Einstellungen: zweite Seite im selben Panel."""
        page = QtWidgets.QWidget()
        self.pages.addWidget(page)
        lay = QtWidgets.QVBoxLayout(page)
        lay.setContentsMargins(22, 22, 22, 22)
        lay.setSpacing(12)
        self.btn_back = QtWidgets.QPushButton()
        self.btn_back.setCursor(Qt.CursorShape.PointingHandCursor)
        self.btn_back.clicked.connect(lambda: self.pages.setCurrentIndex(0))
        lay.addWidget(self.btn_back, 0, Qt.AlignmentFlag.AlignLeft)
        self.adv_title = QtWidgets.QLabel()
        self.adv_title.setObjectName("PanelTitle")
        self.adv_title.setWordWrap(True)
        lay.addWidget(self.adv_title)
        lay.addSpacing(10)
        self.l_fmt = QtWidgets.QLabel()
        self.l_fmt.setObjectName("Label")
        lay.addWidget(self.l_fmt)
        self.seg_fmt = Segmented(("conf", "lua"), prefix="fmtsw.")
        self.seg_fmt.changed.connect(self.format_changed.emit)
        lay.addWidget(self.seg_fmt)
        self.fmt_state = QtWidgets.QLabel()
        self.fmt_state.setObjectName("Label")
        self.fmt_state.setWordWrap(True)
        lay.addWidget(self.fmt_state)
        lay.addStretch()

    def sync(self, cfg: dict, info: str) -> None:
        self.title.setText(t("settings"))
        self.l_lang.setText(t("language"))
        self.l_theme.setText(t("theme"))
        self.l_reset.setText(t("reset.section"))
        self.btn_reset.setText(t("reset.button"))
        self.btn_reset.setToolTip(t("reset.tip"))
        self.btn_adv.setText(t("adv.button"))
        self.btn_back.setText("‹  " + t("adv.back"))
        self.adv_title.setText(t("adv.button"))
        self.l_fmt.setText(t("adv.format"))
        self.seg_fmt.retranslate()
        for k, b in self.seg_fmt.btns.items():
            b.setToolTip(t("tip.fmtsw." + k))
        hdir = model.config_home() / "hypr"
        self.seg_fmt.select(model.hypr_format(hdir) or "lua")
        found = model.detect_format(hdir)
        self.fmt_state.setText(t("adv.found", name=t("fmt." + (found or "none"))))
        self.lang_btns[cfg["lang"]].setChecked(True)
        for name, b in self.theme_btns.items():
            b.setToolTip(t("accent." + name))
            ring = (theme.THEMES["blue"] if name == "white" else "#ffffff") if name == cfg["theme"] else theme.SURFACE
            b.setStyleSheet(f"QPushButton {{ background:{theme.THEMES[name]}; border:3px solid {ring};"
                            f" border-radius:19px; padding:0; }}")
        self.info.setText(f"{info}\nv{hyprstudio.__version__} · LabtosoStudios")


class ResetDialog(QtWidgets.QDialog):
    """Bestätigung für „Alles zurücksetzen“: Knopf erst aktiv, wenn der Haken gesetzt ist."""

    def __init__(self, parent, lua_ok: bool):
        super().__init__(parent)
        self.setWindowTitle(t("reset.title"))
        self.setModal(True)
        self.setMinimumWidth(520)
        self.setWindowFlags(Qt.WindowType.Dialog | Qt.WindowType.FramelessWindowHint)
        self.setAttribute(Qt.WidgetAttribute.WA_TranslucentBackground)
        outer = QtWidgets.QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        frame = QtWidgets.QFrame()
        frame.setObjectName("DialogFrame")
        outer.addWidget(frame)
        lay = QtWidgets.QVBoxLayout(frame)
        lay.setContentsMargins(24, 22, 24, 20)
        lay.setSpacing(12)
        title = QtWidgets.QLabel(t("reset.title"))
        title.setObjectName("PanelTitle")
        lay.addWidget(title)
        text = QtWidgets.QLabel(t("reset.text"))
        text.setWordWrap(True)
        lay.addWidget(text)
        self.factory = QtWidgets.QCheckBox(t("reset.factory"))
        self.factory.setEnabled(lua_ok)
        self.factory.setToolTip(t("reset.factory_tip") if lua_ok else t("reset.factory_off"))
        lay.addWidget(self.factory)
        lay.addSpacing(6)
        self.sure = QtWidgets.QCheckBox(t("reset.sure"))
        self.sure.setObjectName("Sure")
        lay.addWidget(self.sure)
        row = QtWidgets.QHBoxLayout()
        row.addStretch()
        cancel = QtWidgets.QPushButton(t("cancel"))
        cancel.setCursor(Qt.CursorShape.PointingHandCursor)
        cancel.clicked.connect(self.reject)
        cancel.setDefault(True)
        self.ok = QtWidgets.QPushButton(t("reset.button"))
        self.ok.setObjectName("DangerFill")
        self.ok.setCursor(Qt.CursorShape.PointingHandCursor)
        self.ok.setEnabled(False)
        self.ok.clicked.connect(self.accept)
        self.sure.toggled.connect(self.ok.setEnabled)
        row.addWidget(cancel)
        row.addWidget(self.ok)
        lay.addLayout(row)


# ---------- Auswahlmenüs ----------

def round_popup(combo: QtWidgets.QComboBox) -> None:
    """Rundes Aufklappmenü ohne helles Rechteck dahinter (Fenster des Menüs durchsichtig)."""
    win = combo.view().window()
    if win is combo or win.property("hs_round"):
        return
    win.setProperty("hs_round", True)
    win.setWindowFlags(win.windowFlags() | Qt.WindowType.FramelessWindowHint | Qt.WindowType.NoDropShadowWindowHint)
    win.setAttribute(Qt.WidgetAttribute.WA_TranslucentBackground)
    win.setStyleSheet("QComboBoxPrivateContainer { background: transparent; border: none; }")


class RoundPopups(QtCore.QObject):
    """App-weit: jedes Auswahlmenü bekommt runde Ecken ohne helle Box."""

    def eventFilter(self, obj, e):
        if e.type() == QtCore.QEvent.Type.Polish:
            if isinstance(obj, QtWidgets.QComboBox) and not isinstance(obj, SearchCombo):
                round_popup(obj)
            elif isinstance(obj, QtWidgets.QMenu) and not obj.property("hs_round"):
                obj.setProperty("hs_round", True)
                obj.setWindowFlags(obj.windowFlags() | Qt.WindowType.FramelessWindowHint
                                   | Qt.WindowType.NoDropShadowWindowHint)
                obj.setAttribute(Qt.WidgetAttribute.WA_TranslucentBackground)
        return False


class NavHeader(QtWidgets.QPushButton):
    """Kategorie in der Seitenleiste: Akzentfarbe, kleine Großbuchstaben, per Klick ein-/ausklappbar.
    Der gerade offene Bereich bleibt auch eingeklappt sichtbar."""

    def __init__(self, gid: str, key: str, win, parent=None):
        super().__init__(parent)
        self.setObjectName("NavHead")
        self.setCursor(Qt.CursorShape.PointingHandCursor)
        self.gid, self.key, self.win, self.buttons = gid, key, win, []
        self.clicked.connect(self.toggle)

    def collapsed(self) -> bool:
        return self.gid in self.win.cfg.get("nav_collapsed", [])

    def toggle(self):
        lst = self.win.cfg.setdefault("nav_collapsed", [])
        if self.gid in lst:
            lst.remove(self.gid)
        else:
            lst.append(self.gid)
        self.win.save_later()
        self.sync()

    def sync(self, *_):
        c = self.collapsed()
        for b in self.buttons:
            b.setVisible(not c or b.isChecked())
        self.retranslate()
        self.update()

    def retranslate(self):
        self.setText(t("nav." + self.key))
        self.setToolTip(t("nav.fold") if not self.collapsed() else t("nav.unfold"))

    def enterEvent(self, e):
        super().enterEvent(e)
        self.update()

    def leaveEvent(self, e):
        super().leaveEvent(e)
        self.update()

    def paintEvent(self, e):
        """Kleiner Pfeil rechts – nur beim Darüberfahren oder wenn eingeklappt (wie in Dateimanagern)."""
        super().paintEvent(e)
        if not (self.underMouse() or self.collapsed()):
            return
        p = QtGui.QPainter(self)
        p.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
        p.setPen(QtGui.QPen(QtGui.QColor(theme.MUTED), 1.4, Qt.PenStyle.SolidLine, Qt.PenCapStyle.RoundCap,
                            Qt.PenJoinStyle.RoundJoin))
        cx, cy, d = self.width() - 16, self.height() - 12, 3.5
        pts = ([QtCore.QPointF(cx - d / 2, cy - d), QtCore.QPointF(cx + d / 2, cy), QtCore.QPointF(cx - d / 2, cy + d)]
               if self.collapsed() else
               [QtCore.QPointF(cx - d, cy - d / 2), QtCore.QPointF(cx, cy + d / 2), QtCore.QPointF(cx + d, cy - d / 2)])
        p.drawPolyline(pts)
        p.end()


def nav_groups(layout, nav: dict, groups, kind: str, win) -> list:
    """Seitenleiste nach Kategorien ordnen: Überschrift, darunter ihre Bereiche; ohne Gruppen wie bisher."""
    heads, placed = [], set()
    for key, items in groups:
        h = NavHeader(f"{kind}:{key}", key, win)
        layout.addWidget(h)
        for k in items:
            if k in nav:
                layout.addWidget(nav[k])
                h.buttons.append(nav[k])
                nav[k].toggled.connect(h.sync)
                placed.add(k)
        heads.append(h)
    for k, b in nav.items():  # Bereiche ohne Kategorie ans Ende
        if k not in placed:
            layout.addWidget(b)
    for h in heads:
        h.sync()
    return heads


class SearchPopup(QtWidgets.QFrame):
    """Aufklappliste mit Suchfeld oben – Tippen sucht sofort, Pfeile/Enter/Esc funktionieren.
    Schnell auch bei Hunderten Einträgen: Liste nur neu aufbauen, wenn sich die Einträge geändert haben,
    gleich hohe Zeilen (keine Messung pro Eintrag), Filter gebündelt, Fenstergröße nur beim Öffnen setzen."""
    picked = Signal(int)
    ROW = 30
    ROWS = 12

    def __init__(self, combo):
        super().__init__(combo, Qt.WindowType.Popup | Qt.WindowType.FramelessWindowHint
                         | Qt.WindowType.NoDropShadowWindowHint)
        self.combo = combo
        self._texts = None
        self._low = []
        self.setAttribute(Qt.WidgetAttribute.WA_TranslucentBackground)
        outer = QtWidgets.QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        card = QtWidgets.QFrame()
        card.setObjectName("PopupCard")
        outer.addWidget(card)
        lay = QtWidgets.QVBoxLayout(card)
        lay.setContentsMargins(8, 8, 8, 8)
        lay.setSpacing(6)
        self.search = QtWidgets.QLineEdit()
        self.search.setClearButtonEnabled(True)
        self.search.installEventFilter(self)
        lay.addWidget(self.search)
        self.list = QtWidgets.QListWidget()
        self.list.setObjectName("PopList")
        self.list.setUniformItemSizes(True)
        self.list.setHorizontalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
        self.list.itemClicked.connect(self._choose)
        lay.addWidget(self.list)
        # bei langen Listen kurz warten, bis das Tippen pausiert (sonst ruckelt jeder Buchstabe)
        self._delay = QtCore.QTimer(self)
        self._delay.setSingleShot(True)
        self._delay.setInterval(70)
        self._delay.timeout.connect(self._filter)
        self.search.textChanged.connect(self._typed)

    def _build(self):
        texts = [self.combo.itemText(i) for i in range(self.combo.count())]
        if texts == self._texts:
            return
        self._texts = texts
        self._low = [x.lower() for x in texts]
        self.list.setUpdatesEnabled(False)
        self.list.clear()
        size = QtCore.QSize(0, self.ROW)
        for i, text in enumerate(texts):
            it = QtWidgets.QListWidgetItem(text)
            it.setData(Qt.ItemDataRole.UserRole, i)
            it.setSizeHint(size)
            for role in (Qt.ItemDataRole.FontRole, Qt.ItemDataRole.ToolTipRole, Qt.ItemDataRole.ForegroundRole):
                v = self.combo.itemData(i, role)
                if v is not None:
                    it.setData(role, v)
            self.list.addItem(it)
        self.list.setUpdatesEnabled(True)

    def open(self):
        self.search.setPlaceholderText(t("search"))
        self._build()
        self.search.blockSignals(True)
        self.search.clear()
        self.search.blockSignals(False)
        self._filter()
        self._place()
        cur = self.list.item(self.combo.currentIndex())
        if cur:
            self.list.setCurrentItem(cur)
        self.show()
        if cur:
            self.list.scrollToItem(cur, QtWidgets.QAbstractItemView.ScrollHint.PositionAtCenter)
        self.activateWindow()
        self.search.setFocus()

    def _visible(self) -> list:
        return [self.list.item(i) for i in range(self.list.count()) if not self.list.item(i).isHidden()]

    def _typed(self, *_):
        if len(self._low) > 150:
            self._delay.start()
        else:
            self._filter()

    def _filter(self, *_):
        self._delay.stop()
        words = self.search.text().strip().lower().split()
        self.list.setUpdatesEnabled(False)
        first = None
        for i, low in enumerate(self._low):
            hit = all(w in low for w in words)
            it = self.list.item(i)
            if it.isHidden() == hit:
                it.setHidden(not hit)
            if hit and first is None:
                first = it
        self.list.setUpdatesEnabled(True)
        if words and first is not None:
            self.list.setCurrentItem(first)
            self.list.scrollToItem(first)

    def _place(self):
        """Größe einmal beim Öffnen (nach der Eintragszahl); ist unten kein Platz, geht das Menü nach oben auf."""
        n = len(self._low)
        h_list = max(1, min(n, self.ROWS)) * self.ROW + 8
        w = max(self.combo.width(), 280)
        h = h_list + self.search.sizeHint().height() + 6 + 16
        self.list.setFixedHeight(h_list)
        self.setFixedSize(w, h)
        g = self.combo.mapToGlobal(QtCore.QPoint(0, self.combo.height() + 4))
        scr = (self.combo.screen() or QtGui.QGuiApplication.primaryScreen()).availableGeometry()
        if g.y() + h > scr.bottom():
            g = self.combo.mapToGlobal(QtCore.QPoint(0, -h - 4))
        g.setX(min(max(scr.left(), g.x()), scr.right() - w))
        self.move(g)

    def _choose(self, it=None):
        if self._delay.isActive():
            self._filter()
        it = it or self.list.currentItem()
        if it is not None and not it.isHidden():
            self.picked.emit(it.data(Qt.ItemDataRole.UserRole))
        self.hide()

    def eventFilter(self, obj, e):
        if obj is self.search and e.type() == QtCore.QEvent.Type.KeyPress:
            k = e.key()
            steps = {Qt.Key.Key_Down: 1, Qt.Key.Key_Up: -1, Qt.Key.Key_PageDown: 8, Qt.Key.Key_PageUp: -8}
            if k in steps:
                vis = self._visible()
                if vis:
                    cur = self.list.currentItem()
                    i = vis.index(cur) if cur in vis else -1
                    self.list.setCurrentItem(vis[max(0, min(len(vis) - 1, i + steps[k]))])
                return True
            if k in (Qt.Key.Key_Return, Qt.Key.Key_Enter):
                self._choose()
                return True
            if k == Qt.Key.Key_Escape:
                self.hide()
                return True
        return False


class SearchCombo(QtWidgets.QComboBox):
    """Auswahlmenü mit Suchfeld (nicht editierbar); editierbare Felder nutzen das normale runde Menü."""

    def __init__(self, parent=None):
        super().__init__(parent)
        self._pop = None

    def showPopup(self):
        if self.isEditable():
            round_popup(self)
            super().showPopup()
            return
        if self._pop is None:
            self._pop = SearchPopup(self)
            self._pop.picked.connect(self._choose)
        self._pop.open()

    def _choose(self, i: int):
        """Wie eine echte Auswahl im Qt-Menü: Index setzen UND activated melden (sonst merkt die Seite nichts)."""
        self.setCurrentIndex(i)
        self.activated.emit(i)
        self.textActivated.emit(self.itemText(i))

    def hidePopup(self):
        if self._pop is not None:
            self._pop.hide()
        super().hidePopup()


class _FontJob(QtCore.QThread):
    line = Signal(str)
    result = Signal(object)

    def __init__(self, archive, parent=None):
        super().__init__(parent)
        self.archive = archive

    def run(self):
        try:
            r = fonts.install(self.archive, self.line.emit)
        except Exception as e:  # noqa: BLE001 – Netzfehler sauber melden
            r = e
        self.result.emit(r)


class FontCombo(SearchCombo):
    """Schriftart wählen: installierte Schriften in ihrer eigenen Schrift, darunter Nerd Fonts zum Installieren."""
    chosen = Signal(str)
    _all = weakref.WeakSet()
    _loaded = False

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setMinimumWidth(320)
        self.value = None
        self.job = None
        FontCombo._all.add(self)
        self.activated.connect(self._picked)
        if not FontCombo._loaded:  # über Hypr Studio installierte Schriften sofort in Qt verfügbar machen
            FontCombo._loaded = True
            for f in fonts.local_files():
                QtGui.QFontDatabase.addApplicationFont(str(f))

    def fill(self, value: str, force: bool = False):
        if not force and self.count() and value == self.value:
            return
        self.value = value
        fams = sorted({f for f in QtGui.QFontDatabase.families() if not f.startswith(".")}, key=str.lower)
        self.blockSignals(True)
        self.clear()
        if value and value not in fams:  # gewählt, aber (noch) nicht installiert
            self.addItem(f"{value}  ·  {t('font.missing')}", ("font", value))
        for f in fams:
            self.addItem(f, ("font", f))
            self.setItemData(self.count() - 1, QtGui.QFont(f), Qt.ItemDataRole.FontRole)
        get = t("font.get")
        for archive, fam in fonts.CATALOG:
            if fam not in fams:
                self.addItem(f"{fam}  –  {get}", ("get", archive))
                self.setItemData(self.count() - 1, t("font.get_tip"), Qt.ItemDataRole.ToolTipRole)
                self.setItemData(self.count() - 1, QtGui.QColor(theme.MUTED), Qt.ItemDataRole.ForegroundRole)
        self.setCurrentIndex(max(0, self.findData(("font", value))))
        self.blockSignals(False)

    def _win(self):
        w = self.window()
        return w if hasattr(w, "set_status") else None

    def _picked(self, i: int):
        kind, val = self.itemData(i) or ("", "")
        if kind == "font":
            self.value = val
            self.chosen.emit(val)
            return
        self.setCurrentIndex(max(0, self.findData(("font", self.value))))
        if kind == "get" and self.job is None:
            self._install(val)

    def _install(self, archive: str):
        win = self._win()
        self.setEnabled(False)
        if win:
            win.set_status(t("font.loading", name=fonts.family_of(archive)))
            win.bar_busy(True)
        self.job = _FontJob(archive, self)
        if win:
            self.job.line.connect(win.log)
        self.job.result.connect(lambda r, a=archive: self._installed(a, r))
        self.job.start()

    def _installed(self, archive: str, r):
        win = self._win()
        self.job = None
        self.setEnabled(True)
        if win:
            win.bar_busy(False)
        if isinstance(r, Exception) or not r:
            if win:
                win.log(f"{archive}: {r}")
                win.set_status(t("font.fail"))
            return
        found = []
        for f in r:
            fid = QtGui.QFontDatabase.addApplicationFont(str(f))
            if fid >= 0:
                found += QtGui.QFontDatabase.applicationFontFamilies(fid)
        fam = fonts.family_of(archive)
        if fam not in found and found:
            fam = min(set(found), key=len)
        for c in list(FontCombo._all):
            c.fill(c.value or "", force=True)
        self.fill(fam, force=True)
        self.chosen.emit(fam)
        if win:
            win.set_status(t("font.done", name=fam))
