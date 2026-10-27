"""Bausteine für die Einstellungs-Seiten: Seite mit Titel, Karten mit Zeilen, gsettings-Steuerelemente,
Hintergrund-Aufgaben."""

from pathlib import Path

from hyprstudio.widgets import Switch

from . import theme, usercmd
from .i18n import t
from .qt import QtCore, QtGui, QtWidgets, Qt, Signal


class Job(QtCore.QThread):
    """Funktion im Hintergrund; done(ok, ergebnis oder Fehlertext)."""
    done = Signal(bool, object)
    running = set()

    def __init__(self, fn, parent=None):
        super().__init__(parent)
        self.fn = fn
        Job.running.add(self)
        self.finished.connect(lambda: Job.running.discard(self))

    @classmethod
    def wait_all(cls):
        """Beim Beenden: laufende Aufgaben abwarten (sonst stürzt Qt beim Aufräumen ab)."""
        for j in list(cls.running):
            j.wait(5000)

    def run(self):
        try:
            self.done.emit(True, self.fn())
        except Exception as e:  # noqa: BLE001  (Fehler landen im Protokoll)
            key = getattr(e, "key", None)       # remote.SourceError: „schlüssel: detail“ für klare Meldungen
            self.done.emit(False, f"{key}: {getattr(e, 'detail', '')}" if key else f"{type(e).__name__}: {e}")


def start_job(owner, fn, on_done) -> Job:
    job = Job(fn, owner)
    jobs = owner.__dict__.setdefault("_jobs", set())
    jobs.add(job)

    def finish(ok, res, j=job):
        jobs.discard(j)
        on_done(ok, res)
        j.deleteLater()
    job.done.connect(finish)
    job.start()
    return job


class Page(QtWidgets.QWidget):
    """Seite mit Titel oben (+ Platz für Knöpfe rechts) und scrollbarem Inhalt."""

    def __init__(self, title_key: str, parent=None, scroll: bool = True):
        super().__init__(parent)
        self.title_key = title_key
        lay = QtWidgets.QVBoxLayout(self)
        lay.setContentsMargins(0, 0, 0, 0)
        lay.setSpacing(12)
        # Kopf: Titel + Schilder | Werkzeuge; bei wenig Platz rutschen die Werkzeuge in eine zweite Zeile
        self.head = QtWidgets.QBoxLayout(QtWidgets.QBoxLayout.Direction.LeftToRight)
        self.head.setSpacing(8)
        left = QtWidgets.QHBoxLayout()
        self.title = QtWidgets.QLabel()
        self.title.setObjectName("PageTitle")
        left.addWidget(self.title)
        self.b_reset = QtWidgets.QToolButton()          # „Reset to default“ für genau diese Seite
        self.b_reset.setObjectName("ResetBtn")
        self.b_reset.setFixedSize(30, 30)
        self.b_reset.setIconSize(QtCore.QSize(16, 16))
        self.b_reset.setCursor(Qt.CursorShape.PointingHandCursor)
        self.b_reset.clicked.connect(self._reset_clicked)
        self.b_reset.hide()
        left.addWidget(self.b_reset, 0, Qt.AlignmentFlag.AlignVCenter)
        self.chips = QtWidgets.QHBoxLayout()
        self.chips.setSpacing(6)
        left.addLayout(self.chips)
        left.addStretch()
        self.head.addLayout(left, 1)
        tools_row = QtWidgets.QHBoxLayout()
        tools_row.addStretch()
        self.tools = QtWidgets.QHBoxLayout()
        self.tools.setSpacing(8)
        tools_row.addLayout(self.tools)
        self.head.addLayout(tools_row)
        lay.addLayout(self.head)
        self.inner = QtWidgets.QWidget()
        self.inner.setObjectName("PageInner")
        self.body = QtWidgets.QVBoxLayout(self.inner)
        self.body.setContentsMargins(0, 2, 12, 12)   # Abstand zum Scrollbalken, Kartenecken bleiben sichtbar
        self.body.setSpacing(12)
        if scroll:
            sa = QtWidgets.QScrollArea()
            sa.setWidgetResizable(True)
            sa.setHorizontalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
            sa.setWidget(self.inner)
            sa.viewport().setObjectName("Viewport")      # mit Selektor, sonst erben alle Knöpfe „transparent“
            sa.viewport().setStyleSheet("#Viewport { background: transparent; }")
            lay.addWidget(sa, 1)
        else:
            lay.addWidget(self.inner, 1)
        self.controls = []

    def finish(self):
        self.body.addStretch()
        if self.reset_keys() or getattr(self, "has_reset", False):
            from .widgets import draw_icon
            self.b_reset.setIcon(draw_icon("reset", theme.MUTED, 16))
            self.b_reset.show()

    # ----- Zurücksetzen -----
    def reset_keys(self) -> list:
        """gsettings keys of this page (schema, key); pages add their own via reset_extra."""
        return [(c.schema, c.gkey) for c in self.controls if getattr(c, "schema", None)]

    def reset_extra(self, backup: Path) -> None:
        """Extra things to reset (files etc.); copy what is changed into backup first."""

    def _reset_clicked(self):
        from hyprstudio import dialogs
        host = getattr(self, "host", None)
        if host is None or not dialogs.confirm(self, t("reset.title"), t("reset.ask", page=self.title.text())):
            return
        import json
        bak = usercmd.backup_dir()
        keys = self.reset_keys()
        values = {f"{s} {k}": usercmd.gget(s, k) for s, k in keys}
        (bak / f"{self.title_key}.json").write_text(json.dumps(values, ensure_ascii=False, indent=1), encoding="utf-8")
        host.log("")
        for s, k in keys:
            usercmd.greset(s, k)
        self.reset_extra(bak)
        self.refresh()

        def undo(values=values, bak=bak):
            for sk, v in values.items():
                s, k = sk.split(" ", 1)
                if v is not None:
                    usercmd.gset(s, k, v)
            self.undo_extra(bak)
            self.refresh()
            host.set_status(t("reset.undone"))
        host.offer_undo(t("reset.done", page=self.title.text(), path=str(bak)), undo)

    def undo_extra(self, backup: Path) -> None:
        """Restore what reset_extra changed."""

    def resizeEvent(self, e):
        super().resizeEvent(e)
        need = self.title.sizeHint().width() + 180 + sum(
            self.tools.itemAt(i).widget().sizeHint().width() + 8
            for i in range(self.tools.count()) if self.tools.itemAt(i).widget())
        d = (QtWidgets.QBoxLayout.Direction.TopToBottom if self.width() < need
             else QtWidgets.QBoxLayout.Direction.LeftToRight)
        if self.head.direction() != d:
            self.head.setDirection(d)

    def retranslate(self):
        self.title.setText(t(self.title_key))
        self.b_reset.setToolTip(t("reset.tip"))
        for card in self.findChildren(Card):
            card.retranslate()
        for c in self.controls:
            c.retranslate()

    def refresh(self):
        for c in self.controls:
            c.load()


class Card(QtWidgets.QFrame):
    def __init__(self, title_key: str = "", parent=None):
        super().__init__(parent)
        self.setObjectName("Card")
        self.title_key = title_key
        self.lay = QtWidgets.QVBoxLayout(self)
        self.lay.setContentsMargins(16, 12, 16, 12)
        self.lay.setSpacing(4)
        self.head = QtWidgets.QLabel()
        self.head.setObjectName("CardTitle")
        self.head.setVisible(bool(title_key))
        self.lay.addWidget(self.head)

    def retranslate(self):
        if self.title_key:
            self.head.setText(t(self.title_key))


class Section(Card):
    """Like a card, but without a frame – for galleries (no cards inside cards)."""

    def __init__(self, title_key: str = "", parent=None):
        super().__init__(title_key, parent)
        self.setObjectName("Section")
        self.lay.setContentsMargins(0, 8, 0, 0)


class Row(QtWidgets.QWidget):
    """Eine Zeile: Text links, Steuerelement rechts; Tooltip nur auf dem Text."""

    def __init__(self, key: str, control: QtWidgets.QWidget, parent=None):
        super().__init__(parent)
        self.key = key
        lay = QtWidgets.QHBoxLayout(self)
        lay.setContentsMargins(0, 6, 0, 6)
        self.label = QtWidgets.QLabel()
        self.label.setWordWrap(True)        # schmale Fenster: Text bricht um statt abzuschneiden
        self.label.setMinimumWidth(80)
        self.label.setSizePolicy(QtWidgets.QSizePolicy.Policy.Ignored, QtWidgets.QSizePolicy.Policy.Preferred)
        lay.addWidget(self.label, 1)
        lay.addWidget(control)

    def retranslate(self):
        self.label.setText(t(self.key))
        tip = t(self.key + ".tip")
        self.label.setToolTip("" if tip == self.key + ".tip" else tip)


def backup_files(files, bak: Path) -> None:
    """Copy existing files into the backup folder (path kept as name)."""
    import shutil
    for f in files:
        src = usercmd.readable(Path(f))
        if src.is_file():
            shutil.copy2(src, bak / str(f).strip("/").replace("/", "__"))


def restore_files(files, bak: Path) -> None:
    import shutil
    for f in files:
        b = bak / str(f).strip("/").replace("/", "__")
        if b.is_file():
            Path(f).parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(b, f)


def combo() -> QtWidgets.QComboBox:
    c = QtWidgets.QComboBox()
    c.setObjectName("Combo")
    c.setCursor(Qt.CursorShape.PointingHandCursor)
    c.setMinimumWidth(150)
    c.setSizeAdjustPolicy(QtWidgets.QComboBox.SizeAdjustPolicy.AdjustToMinimumContentsLengthWithIcon)
    c.setMinimumContentsLength(14)
    c.setMaxVisibleItems(16)
    return c


class GControl(QtCore.QObject):
    """gsettings-Wert ↔ Steuerelement. Änderungen werden sofort gesetzt (wie GNOME Tweaks)."""
    changed = Signal()

    def __init__(self, key, schema, gkey, kind, extra, parent=None):
        super().__init__(parent)
        self.key, self.schema, self.gkey, self.kind, self.extra = key, schema, gkey, kind, extra
        self._loading = False
        if kind == "bool":
            self.w = Switch()
            self.w.toggled.connect(lambda v: self._set(bool(v)))
        elif kind == "choice":
            self.w = combo()
            self.w.currentIndexChanged.connect(lambda i: self._set(self.w.itemData(i)))
        elif kind in ("int", "double"):
            lo, hi, step = extra
            self.w = QtWidgets.QSpinBox() if kind == "int" else QtWidgets.QDoubleSpinBox()
            self.w.setObjectName("Spin")
            self.w.setRange(lo, hi)
            self.w.setSingleStep(step)
            self.w.setMinimumWidth(110)
            if kind == "double":
                self.w.setDecimals(2)
            self.w.setKeyboardTracking(False)
            self.w.valueChanged.connect(lambda v: self._set(int(v) if kind == "int" else round(float(v), 2)))
        elif kind == "font":
            self.w = QtWidgets.QWidget()
            hl = QtWidgets.QHBoxLayout(self.w)
            hl.setContentsMargins(0, 0, 0, 0)
            hl.setSpacing(6)
            self.fam = QtWidgets.QFontComboBox()
            self.fam.setObjectName("Combo")
            self.fam.setMinimumWidth(220)
            self.size = QtWidgets.QDoubleSpinBox()
            self.size.setObjectName("Spin")
            self.size.setRange(5, 72)
            self.size.setDecimals(0)
            self.size.setMinimumWidth(80)
            self.size.setKeyboardTracking(False)
            hl.addWidget(self.fam)
            hl.addWidget(self.size)
            self.fam.currentFontChanged.connect(lambda _f: self._font())
            self.size.valueChanged.connect(lambda _v: self._font())
        self.row = Row(key, self.w)
        self.available = True

    def _font(self):
        from .gnome import join_font
        self._set(join_font(self.fam.currentFont().family(), self.size.value()))

    def _set(self, value):
        if self._loading or value is None:
            return
        usercmd.gset(self.schema, self.gkey, value)
        self.changed.emit()

    def load(self):
        from .gnome import available, split_font
        self.available = available(self.schema, self.gkey)
        self.row.setEnabled(self.available)
        self.row.setToolTip("" if self.available else t("na"))
        v = usercmd.gget(self.schema, self.gkey)
        self._loading = True
        try:
            if self.kind == "bool":
                self.w.set_now(bool(v))
            elif self.kind == "choice":
                i = self.w.findData(v)
                if i < 0 and v is not None:
                    self.w.addItem(str(v), v)
                    i = self.w.count() - 1
                self.w.setCurrentIndex(max(0, i))
            elif self.kind in ("int", "double") and isinstance(v, (int, float)):
                self.w.setValue(v)
            elif self.kind == "font" and isinstance(v, str):
                fam, size = split_font(v)
                self.fam.setCurrentFont(QtGui.QFont(fam))
                self.size.setValue(size)
        finally:
            self._loading = False

    def retranslate(self):
        self.row.retranslate()
        if self.kind == "choice":
            self._loading = True
            cur = self.w.currentData()
            self.w.clear()
            for val in self.extra:
                key = f"{self.key}.{val}"
                txt = t(key)
                self.w.addItem(val if txt == key else txt, val)
            i = self.w.findData(cur)
            self.w.setCurrentIndex(max(0, i))
            self._loading = False


def gsection(page: Page, card: Card, specs) -> list:
    out = []
    for key, schema, gkey, kind, extra in specs:
        c = GControl(key, schema, gkey, kind, extra, page)
        card.lay.addWidget(c.row)
        page.controls.append(c)
        out.append(c)
    return out


def tool_button(text_key: str, primary: bool = False) -> QtWidgets.QPushButton:
    b = QtWidgets.QPushButton()
    b.setProperty("tkey", text_key)
    if primary:
        b.setObjectName("Primary")
    b.setCursor(Qt.CursorShape.PointingHandCursor)
    b.setMinimumHeight(34)
    return b


def retranslate_buttons(widget: QtWidgets.QWidget) -> None:
    for b in widget.findChildren(QtWidgets.QAbstractButton):
        k = b.property("tkey")
        if k:
            b.setText(t(k))


CHIP_COLORS = {"Chip": None, "ChipOk": theme.OK, "ChipWarn": "#d29922", "ChipAccent": "accent"}


def set_chip(lb: QtWidgets.QLabel, kind: str, text: str, accent: str = theme.THEMES["blue"]) -> None:
    """Schild mit Stil direkt am Widget (objectName-Wechsel + polish liefert eine zu kleine Größe)."""
    col = CHIP_COLORS.get(kind)
    col = accent if col == "accent" else col
    if col:
        css = f"background: transparent; border: 1px solid {col}; color: {col};"
    else:
        css = f"background: {theme.SURFACE2}; border: 1px solid {theme.BORDER}; color: {theme.MUTED};"
    lb.setStyleSheet(f"QLabel {{ {css} border-radius: 9px; padding: 2px 9px; font-size: 9pt; }}")
    lb.setText(text)
    lb.adjustSize()


def accent_switches(widget, accent: str) -> None:
    for sw in widget.findChildren(Switch):
        sw.accent = accent
        sw.update()


__all__ = ["Job", "start_job", "Page", "Card", "Row", "GControl", "gsection", "combo", "tool_button",
           "retranslate_buttons", "set_chip", "accent_switches", "theme"]


class RemoveButton(QtWidgets.QToolButton):
    """Small „uninstall“ button next to an installed extension."""

    def __init__(self, parent=None):
        super().__init__(parent)
        from .widgets import draw_icon
        self.setIcon(draw_icon("close", theme.MUTED, 14))
        self.setIconSize(QtCore.QSize(14, 14))
        self.setCursor(Qt.CursorShape.PointingHandCursor)
        self.setFixedSize(30, 30)
        self.setObjectName("ResetBtn")
        self.setToolTip(t("ext.uninstall"))


class MissingButton(QtWidgets.QToolButton):
    """Kleiner Download-Pfeil neben „fehlt“: ein Klick installiert das Fehlende."""

    def __init__(self, parent=None):
        super().__init__(parent)
        from .widgets import draw_icon
        self.setIcon(draw_icon("download", theme.TEXT, 18))
        self.setIconSize(QtCore.QSize(18, 18))
        self.setCursor(Qt.CursorShape.PointingHandCursor)
        self.setFixedSize(30, 30)
        self.setStyleSheet(f"QToolButton {{ background: {theme.SURFACE2}; border: 1px solid {theme.BORDER};"
                           f" border-radius: 8px; }} QToolButton:hover {{ border-color: #d29922; }}")
        self.setToolTip(t("miss.install"))


from hyprstudio.widgets import FlowGrid as _FlowGrid  # noqa: E402


class FlowGrid(_FlowGrid):
    """Kachel-Raster, das beim Verkleinern wieder umbricht (Mindestbreite = eine Kachel, nicht alle Spalten)."""

    def __init__(self, cell_w: int, parent=None):
        super().__init__(cell_w, parent)
        self.grid.setSizeConstraint(QtWidgets.QLayout.SizeConstraint.SetNoConstraint)
        self.setSizePolicy(QtWidgets.QSizePolicy.Policy.Preferred, QtWidgets.QSizePolicy.Policy.Minimum)

    def minimumSizeHint(self):
        return QtCore.QSize(self.cell_w, super().minimumSizeHint().height())

    def _relayout(self, force=False):
        super()._relayout(force)
        rows = (len(self.items) + max(1, self._cols) - 1) // max(1, self._cols)
        h = rows * (max((w.height() for w in self.items), default=0) + 14)
        self.setMinimumHeight(h)
