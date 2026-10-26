"""Eigene Dialoge im App-Look (dunkel, rund, ohne System-Rahmen) statt System-/KDE-Dialogen:
Text eingeben, Rückfrage, Datei/Ordner wählen, speichern, Farbe wählen."""

from .i18n import t
from .qt import QtCore, QtGui, QtWidgets, Qt


class Frame(QtWidgets.QDialog):
    """Rahmenloses rundes Fenster mit Titel; an der Titelzeile verschiebbar."""

    def __init__(self, parent, title: str, width: int = 460):
        super().__init__(parent)
        self.setWindowTitle(title)
        self.setModal(True)
        self.setMinimumWidth(width)
        self.setWindowFlags(Qt.WindowType.Dialog | Qt.WindowType.FramelessWindowHint)
        self.setAttribute(Qt.WidgetAttribute.WA_TranslucentBackground)
        win = parent.window() if parent is not None else None
        if win is not None and win.styleSheet():
            self.setStyleSheet(win.styleSheet())
        outer = QtWidgets.QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        frame = QtWidgets.QFrame()
        frame.setObjectName("DialogFrame")
        outer.addWidget(frame)
        self.body = QtWidgets.QVBoxLayout(frame)
        self.body.setContentsMargins(24, 20, 24, 20)
        self.body.setSpacing(12)
        self.title = QtWidgets.QLabel(title)
        self.title.setObjectName("PanelTitle")
        self.body.addWidget(self.title)
        self._drag = None

    def buttons(self, ok_text: str, primary: str = "Primary", cancel: bool = True) -> QtWidgets.QPushButton:
        row = QtWidgets.QHBoxLayout()
        row.addStretch()
        if cancel:
            c = QtWidgets.QPushButton(t("cancel"))
            c.setCursor(Qt.CursorShape.PointingHandCursor)
            c.setMinimumWidth(120)
            c.clicked.connect(self.reject)
            row.addWidget(c)
        ok = QtWidgets.QPushButton(ok_text)
        ok.setObjectName(primary)
        ok.setCursor(Qt.CursorShape.PointingHandCursor)
        ok.setMinimumWidth(120)
        ok.setDefault(True)
        ok.clicked.connect(self.accept)
        row.addWidget(ok)
        self.body.addLayout(row)
        return ok

    # verschieben an der Titelzeile
    def mousePressEvent(self, e):
        if e.button() == Qt.MouseButton.LeftButton and e.position().y() < 56:
            self._drag = e.globalPosition().toPoint() - self.frameGeometry().topLeft()

    def mouseMoveEvent(self, e):
        if self._drag is not None:
            self.move(e.globalPosition().toPoint() - self._drag)

    def mouseReleaseEvent(self, e):
        self._drag = None


def ask_text(parent, title: str, label: str, text: str = "") -> tuple:
    d = Frame(parent, title)
    if label:
        lb = QtWidgets.QLabel(label)
        lb.setObjectName("RowLabel")
        lb.setWordWrap(True)
        d.body.addWidget(lb)
    le = QtWidgets.QLineEdit(text)
    le.setMinimumHeight(38)
    le.selectAll()
    d.body.addWidget(le)
    d.buttons(t("ok"))
    le.returnPressed.connect(d.accept)
    le.setFocus()
    ok = d.exec() == QtWidgets.QDialog.DialogCode.Accepted
    return le.text(), ok


def confirm(parent, title: str, text: str, ok_text: str | None = None, danger: bool = False) -> bool:
    d = Frame(parent, title)
    lb = QtWidgets.QLabel(text)
    lb.setWordWrap(True)
    d.body.addWidget(lb)
    d.buttons(ok_text or t("yes"), "DangerFill" if danger else "Primary")
    return d.exec() == QtWidgets.QDialog.DialogCode.Accepted


def _file(parent, title: str, start: str, filt: str = "", mode: str = "open") -> str:
    """Qt-Dateibrowser eingebettet im eigenen Fenster (kein Dolphin/Portal-Dialog im hellen System-Look)."""
    d = Frame(parent, title, 900)
    d.resize(920, 600)
    fd = QtWidgets.QFileDialog(d, title, start, filt)
    fd.setOption(QtWidgets.QFileDialog.Option.DontUseNativeDialog, True)
    fd.setWindowFlags(Qt.WindowType.Widget)
    fd.setSizeGripEnabled(False)
    if mode == "dir":
        fd.setFileMode(QtWidgets.QFileDialog.FileMode.Directory)
        fd.setOption(QtWidgets.QFileDialog.Option.ShowDirsOnly, True)
    elif mode == "save":
        fd.setAcceptMode(QtWidgets.QFileDialog.AcceptMode.AcceptSave)
        fd.setFileMode(QtWidgets.QFileDialog.FileMode.AnyFile)
    else:
        fd.setFileMode(QtWidgets.QFileDialog.FileMode.ExistingFile)
    if mode == "save" and start and not QtCore.QFileInfo(start).isDir():
        fd.selectFile(start)
    d.body.addWidget(fd, 1)
    out = []
    fd.accepted.connect(lambda: (out.extend(fd.selectedFiles()), d.accept()))
    fd.rejected.connect(d.reject)
    d.exec()
    return out[0] if out else ""


def open_file(parent, title: str, start: str, filt: str = "") -> str:
    return _file(parent, title, start, filt, "open")


def open_dir(parent, title: str, start: str) -> str:
    return _file(parent, title, start, "", "dir")


def save_file(parent, title: str, start: str, filt: str = "") -> str:
    return _file(parent, title, start, filt, "save")


def pick_color(parent, color: QtGui.QColor, title: str, live=None) -> QtGui.QColor | None:
    d = Frame(parent, title, 560)
    cd = QtWidgets.QColorDialog(color, d)
    cd.setOption(QtWidgets.QColorDialog.ColorDialogOption.ShowAlphaChannel, True)
    cd.setOption(QtWidgets.QColorDialog.ColorDialogOption.DontUseNativeDialog, True)
    cd.setOption(QtWidgets.QColorDialog.ColorDialogOption.NoButtons, True)
    cd.setWindowFlags(Qt.WindowType.Widget)
    if live:
        cd.currentColorChanged.connect(live)
    d.body.addWidget(cd)
    d.buttons(t("ok"))
    if d.exec() == QtWidgets.QDialog.DialogCode.Accepted:
        return cd.currentColor()
    return None
