"""Installiert-Prüfung: fehlt ein Programm, zeigt der Bereich statt der Einstellungen einen großen
Installieren-Knopf. Nach der Installation erscheinen die Einstellungen automatisch."""

from . import launcher, packages
from .i18n import t
from .qt import QtWidgets, Qt


def _big_button(primary=True) -> QtWidgets.QPushButton:
    b = QtWidgets.QPushButton()
    b.setObjectName("Primary" if primary else "")
    b.setCursor(Qt.CursorShape.PointingHandCursor)
    b.setMinimumSize(300, 56)
    f = b.font()
    f.setPointSizeF(f.pointSizeF() + 2)
    f.setBold(True)
    b.setFont(f)
    return b


class Gate(QtWidgets.QStackedWidget):
    """Seite 0 = Inhalt, Seite 1 = „nicht installiert“ mit großem Knopf."""

    def __init__(self, content: QtWidgets.QWidget, key: str, win, parent=None):
        super().__init__(parent)
        self.content, self.key, self.win = content, key, win
        self.addWidget(content)
        box = QtWidgets.QFrame()
        box.setObjectName("Card")
        lay = QtWidgets.QVBoxLayout(box)
        lay.setContentsMargins(40, 40, 40, 40)
        lay.setSpacing(14)
        lay.addStretch()
        self.title = QtWidgets.QLabel()
        self.title.setObjectName("PageTitle")
        self.title.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.sub = QtWidgets.QLabel()
        self.sub.setObjectName("RowLabel")
        self.sub.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.btn = _big_button()
        self.btn.clicked.connect(lambda: self.win.other_page.install_keys([self.key]))
        lay.addWidget(self.title)
        lay.addWidget(self.sub)
        lay.addSpacing(10)
        lay.addWidget(self.btn, 0, Qt.AlignmentFlag.AlignHCenter)
        lay.addStretch()
        self.addWidget(box)

    def refresh(self):
        ok = packages.installed(self.key)
        self.setCurrentIndex(0 if ok else 1)
        name = t("pkg." + self.key)
        self.title.setText(t("gate.missing", name=name))
        self.sub.setText(t("pkgd." + self.key) + "  ·  " + " ".join(
            packages.PACKAGES[self.key][2].get(packages.manager() or "pacman", [])))
        self.btn.setText(t("gate.install", name=name))
        self.btn.setEnabled(packages.available(self.key, packages.manager()))


def _arrow_icon(color: str, size: int = 18):
    """Download arrow (same as Linux AIO's install arrow)."""
    from .qt import QtCore, QtGui
    pm = QtGui.QPixmap(size * 2, size * 2)
    pm.fill(Qt.GlobalColor.transparent)
    p = QtGui.QPainter(pm)
    p.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
    p.scale(2, 2)
    p.setPen(QtGui.QPen(QtGui.QColor(color), 1.8, Qt.PenStyle.SolidLine, Qt.PenCapStyle.RoundCap))
    c = size / 2
    p.drawLine(QtCore.QPointF(c, size * 0.18), QtCore.QPointF(c, size * 0.64))
    p.drawLine(QtCore.QPointF(c - size * 0.2, size * 0.45), QtCore.QPointF(c, size * 0.64))
    p.drawLine(QtCore.QPointF(c + size * 0.2, size * 0.45), QtCore.QPointF(c, size * 0.64))
    p.drawLine(QtCore.QPointF(size * 0.22, size * 0.82), QtCore.QPointF(size * 0.78, size * 0.82))
    p.end()
    return QtGui.QIcon(pm)


class _TileGrid(QtWidgets.QWidget):
    """Equal tiles that wrap with the width (never pushed into each other)."""

    def __init__(self, cell_w: int, cell_h: int, parent=None):
        super().__init__(parent)
        self.cell_w, self.cell_h, self.items, self._cols = cell_w, cell_h, [], 0
        self.grid = QtWidgets.QGridLayout(self)
        self.grid.setContentsMargins(0, 0, 0, 0)
        self.grid.setSpacing(14)
        self.grid.setSizeConstraint(QtWidgets.QLayout.SizeConstraint.SetNoConstraint)

    def add(self, w):
        w.setFixedSize(self.cell_w, self.cell_h)
        self.items.append(w)
        self._relayout(True)

    def _relayout(self, force=False):
        cols = max(1, min(len(self.items), (self.width() + 14) // (self.cell_w + 14)))
        if cols == self._cols and not force:
            return
        self._cols = cols
        for w in self.items:
            self.grid.removeWidget(w)
        for n, w in enumerate(self.items):
            self.grid.addWidget(w, n // cols, n % cols)
        rows = (len(self.items) + cols - 1) // cols
        self.setMinimumHeight(rows * (self.cell_h + 14))
        self.setMinimumWidth(self.cell_w)

    def resizeEvent(self, e):
        super().resizeEvent(e)
        self._relayout()


class LauncherGate(QtWidgets.QStackedWidget):
    """Launcher tab: if the chosen launcher is not installed, pick or install one (equal tiles, wrap)."""

    def __init__(self, page, win, parent=None):
        super().__init__(parent)
        self.page, self.win = page, win
        self.installing = ""
        self.addWidget(page)
        sa = QtWidgets.QScrollArea()
        sa.setWidgetResizable(True)
        sa.setFrameShape(QtWidgets.QFrame.Shape.NoFrame)
        sa.setHorizontalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
        box = QtWidgets.QWidget()
        box.setObjectName("GateBox")
        box.setStyleSheet("#GateBox { background: transparent; }")
        sa.viewport().setObjectName("GateView")
        sa.viewport().setStyleSheet("#GateView { background: transparent; }")
        lay = QtWidgets.QVBoxLayout(box)
        lay.setContentsMargins(8, 24, 8, 24)
        lay.setSpacing(16)
        self.title = QtWidgets.QLabel()
        self.title.setObjectName("PageTitle")
        self.title.setWordWrap(True)
        lay.addWidget(self.title)
        self.grid = _TileGrid(230, 170)
        lay.addWidget(self.grid)
        lay.addStretch()
        sa.setWidget(box)
        self.cards = {}
        for app in launcher.APPS:
            card = QtWidgets.QFrame()
            card.setObjectName("Card")
            cl = QtWidgets.QVBoxLayout(card)
            cl.setContentsMargins(16, 14, 16, 14)
            cl.setSpacing(8)
            name = QtWidgets.QLabel(t("app." + app))
            name.setObjectName("CardTitle")
            desc = QtWidgets.QLabel()
            desc.setObjectName("RowLabel")
            desc.setWordWrap(True)
            desc.setAlignment(Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignTop)
            desc.setSizePolicy(QtWidgets.QSizePolicy.Policy.Ignored, QtWidgets.QSizePolicy.Policy.Expanding)
            bar = QtWidgets.QProgressBar()
            bar.setRange(0, 0)
            bar.setTextVisible(False)
            bar.setFixedHeight(4)
            bar.hide()
            btn = QtWidgets.QPushButton()
            btn.setCursor(Qt.CursorShape.PointingHandCursor)
            btn.setMinimumHeight(36)
            btn.clicked.connect(lambda _=False, a=app: self._click(a))
            cl.addWidget(name)
            cl.addWidget(desc, 1)
            cl.addWidget(bar)
            cl.addWidget(btn)
            self.grid.add(card)
            self.cards[app] = (desc, btn, bar)
        self.addWidget(sa)

    def _click(self, app):
        if packages.installed(app):
            self.page.set("app", app)
            self.page.refresh_controls()
            self.installing = ""
            self.refresh()
        else:
            self.installing = app
            self.refresh()
            self.win.other_page.install_keys([app])

    def refresh(self):
        from .qt import QtGui
        app = self.page.state["app"]
        self.setCurrentIndex(0 if packages.installed(app) else 1)
        self.title.setText(t("gate.launcher", name=t("app." + app)))
        for a, (desc, btn, bar) in self.cards.items():
            inst = packages.installed(a)
            pm = packages.manager()
            avail = a == "llauncher" or bool(pm and packages.available(a, pm))
            desc.setText(t("pkgd." + a))
            if inst:
                btn.setText(t("gate.use"))
                btn.setIcon(QtGui.QIcon())
                btn.setObjectName("")
            else:
                btn.setText(t("gate.install_short"))
                btn.setIcon(_arrow_icon("#ffffff"))
                btn.setObjectName("Primary")
                btn.setEnabled(bool(avail))
                if not avail:
                    btn.setText(t("pkg.unavailable"))
            if inst and self.installing == a:
                self.installing = ""
            bar.setVisible(self.installing == a and not inst)
            btn.style().unpolish(btn)
            btn.style().polish(btn)
