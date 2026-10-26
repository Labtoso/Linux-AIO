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


class LauncherGate(QtWidgets.QStackedWidget):
    """Launcher-Tab: ist der gewählte Launcher nicht installiert, kann man ihn installieren oder wechseln."""

    def __init__(self, page, win, parent=None):
        super().__init__(parent)
        self.page, self.win = page, win
        self.addWidget(page)
        box = QtWidgets.QFrame()
        box.setObjectName("Card")
        lay = QtWidgets.QVBoxLayout(box)
        lay.setContentsMargins(40, 40, 40, 40)
        lay.setSpacing(14)
        lay.addStretch()
        self.title = QtWidgets.QLabel()
        self.title.setObjectName("PageTitle")
        self.title.setAlignment(Qt.AlignmentFlag.AlignCenter)
        lay.addWidget(self.title)
        lay.addSpacing(10)
        row = QtWidgets.QHBoxLayout()
        row.setSpacing(16)
        row.addStretch()
        self.cards = {}
        for app in launcher.APPS:
            card = QtWidgets.QFrame()
            card.setObjectName("BindRow")
            cl = QtWidgets.QVBoxLayout(card)
            cl.setContentsMargins(20, 18, 20, 18)
            cl.setSpacing(10)
            name = QtWidgets.QLabel(t("app." + app))
            name.setObjectName("CardTitle")
            name.setAlignment(Qt.AlignmentFlag.AlignCenter)
            desc = QtWidgets.QLabel()
            desc.setObjectName("RowLabel")
            desc.setAlignment(Qt.AlignmentFlag.AlignCenter)
            desc.setWordWrap(True)
            btn = _big_button()
            btn.setMinimumSize(200, 50)
            btn.clicked.connect(lambda _=False, a=app: self._click(a))
            cl.addWidget(name)
            cl.addWidget(desc)
            cl.addWidget(btn)
            row.addWidget(card)
            self.cards[app] = (desc, btn)
        row.addStretch()
        lay.addLayout(row)
        lay.addStretch()
        self.addWidget(box)

    def _click(self, app):
        if packages.installed(app):
            self.page.set("app", app)
            self.page.refresh_controls()
            self.refresh()
        else:
            self.win.other_page.install_keys([app])

    def refresh(self):
        app = self.page.state["app"]
        self.setCurrentIndex(0 if packages.installed(app) else 1)
        self.title.setText(t("gate.launcher", name=t("app." + app)))
        for a, (desc, btn) in self.cards.items():
            inst = packages.installed(a)
            desc.setText(t("pkgd." + a))
            btn.setText(t("gate.use") if inst else t("gate.install_short"))
            btn.setObjectName("" if inst else "Primary")
            btn.style().unpolish(btn)
            btn.style().polish(btn)
