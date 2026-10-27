"""Hyprland › Arbeitsflächen: eine Zeile pro Fläche 1–10 (Name, Monitor, Lücken, Rahmen, immer da, Start-Fläche)."""

import json
import shutil
import subprocess

from . import model, theme
from . import dialogs
from .i18n import t
from .qt import QtGui, QtWidgets, Qt, Signal
from .widgets import draw_icon


def monitors() -> list:
    if model.DRY:
        return ["DP-1", "HDMI-A-1", "eDP-1"]
    if not shutil.which("hyprctl"):
        return []
    try:
        out = subprocess.run(["hyprctl", "-j", "monitors"], capture_output=True, text=True, timeout=5).stdout
        return [m.get("name", "") for m in json.loads(out) if m.get("name")]
    except (OSError, ValueError, subprocess.TimeoutExpired):
        return []


class NumField(QtWidgets.QLineEdit):
    """Zahlenfeld: leer = Standard (-1)."""
    valueChanged = Signal(int)

    def __init__(self, hi=60):
        super().__init__()
        self.setValidator(QtGui.QIntValidator(0, hi, self))
        self.setPlaceholderText(t("ws.std"))
        self.setFixedWidth(84)
        self.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.textEdited.connect(lambda _: self.valueChanged.emit(self.value()))

    def value(self) -> int:
        return int(self.text()) if self.text().strip().isdigit() else -1

    def setValue(self, v: int):
        self.setText("" if v < 0 else str(v))


def _spin(lo=-1, hi=60) -> NumField:
    return NumField(hi)


class WsRow(QtWidgets.QFrame):
    changed = Signal()

    def __init__(self, w: dict, mons: list, parent=None):
        super().__init__(parent)
        self.setObjectName("BindRow")
        self.w = dict(w)
        lay = QtWidgets.QHBoxLayout(self)
        lay.setContentsMargins(12, 6, 8, 6)
        lay.setSpacing(8)
        num = QtWidgets.QLabel(str(w["id"]))
        num.setObjectName("Mono")
        num.setFixedWidth(22)
        lay.addWidget(num)
        self.name = QtWidgets.QLineEdit(w["name"])
        self.name.setPlaceholderText(t("ws.name_ph"))
        self.name.setFixedWidth(150)
        self.mon = QtWidgets.QComboBox()
        self.mon.setEditable(True)
        self.mon.addItem("")
        self.mon.addItems(mons)
        self.mon.setEditText(w["monitor"])
        self.mon.lineEdit().setPlaceholderText(t("ws.any"))
        self.mon.setFixedWidth(140)
        self.gi, self.go, self.bs = _spin(), _spin(), _spin(-1, 10)
        self.gi.setValue(w["gaps_in"])
        self.go.setValue(w["gaps_out"])
        self.bs.setValue(w["border"])
        self.pers = QtWidgets.QCheckBox(t("ws.persistent"))
        self.pers.setChecked(w["persistent"])
        self.dflt = QtWidgets.QCheckBox(t("ws.default"))
        self.dflt.setChecked(w["default"])
        self.pers.setToolTip(t("tip.ws.persistent"))
        self.dflt.setToolTip(t("tip.ws.default"))
        for wid, tip in ((self.name, "ws.name"), (self.mon, "ws.monitor"), (self.gi, "gaps_in"), (self.go, "gaps_out"),
                         (self.bs, "border_size")):
            wid.setToolTip(t(tip))
            lay.addWidget(wid)
        lay.addWidget(self.pers)
        lay.addWidget(self.dflt)
        lay.addStretch()
        # Seltener gebraucht: im Menü
        self.more = QtWidgets.QToolButton()
        self.more.setPopupMode(QtWidgets.QToolButton.ToolButtonPopupMode.InstantPopup)
        self.more.setCursor(Qt.CursorShape.PointingHandCursor)
        m = QtWidgets.QMenu(self.more)
        self.a_round = m.addAction(t("ws.no_rounding"))
        self.a_round.setCheckable(True)
        self.a_round.setChecked(w["no_rounding"])
        lm = m.addMenu(t("ws.layout"))
        self.layout_acts = {}
        for key in ("", "dwindle", "master"):
            a = lm.addAction(t("ws.layout_std") if not key else t("layout." + key))
            a.setCheckable(True)
            a.setChecked(w["layout"] == key)
            a.triggered.connect(lambda _=False, k=key: self._layout(k))
            self.layout_acts[key] = a
        self.a_empty = m.addAction(t("ws.on_empty"))
        self.a_empty.triggered.connect(self._on_empty)
        self.more.setMenu(m)
        lay.addWidget(self.more)
        self.name.textEdited.connect(self._emit)
        self.mon.currentTextChanged.connect(self._emit)
        for sp in (self.gi, self.go, self.bs):
            sp.valueChanged.connect(self._emit)
        self.pers.toggled.connect(self._emit)
        self.dflt.toggled.connect(self._emit)
        self.a_round.toggled.connect(self._emit)
        self._dots()

    def _layout(self, key):
        for k, a in self.layout_acts.items():
            a.setChecked(k == key)
        self.w["layout"] = key
        self._emit()

    def _on_empty(self):
        txt, ok = dialogs.ask_text(self, t("ws.on_empty"), t("tip.ws.on_empty"), self.w["on_empty"])
        if ok:
            self.w["on_empty"] = txt.strip()
            self._emit()

    def _dots(self):
        extra = self.w["no_rounding"] or self.w["layout"] or self.w["on_empty"]
        self.more.setIcon(draw_icon("dots", theme.THEMES["tokyo"] if extra else theme.MUTED, 16))
        tips = [t("ws.no_rounding")] if self.w["no_rounding"] else []
        if self.w["layout"]:
            tips.append(t("layout." + self.w["layout"]))
        if self.w["on_empty"]:
            tips.append(self.w["on_empty"])
        self.more.setToolTip(", ".join(tips) or t("ws.more"))

    def _emit(self, *_):
        self.w.update(name=self.name.text().strip(), monitor=self.mon.currentText().strip(), gaps_in=self.gi.value(),
                      gaps_out=self.go.value(), border=self.bs.value(), persistent=self.pers.isChecked(),
                      default=self.dflt.isChecked(), no_rounding=self.a_round.isChecked())
        self._dots()
        self.changed.emit()


class WorkspaceEditor(QtWidgets.QWidget):
    changed = Signal()

    def __init__(self, parent=None):
        super().__init__(parent)
        lay = QtWidgets.QVBoxLayout(self)
        lay.setContentsMargins(0, 0, 0, 0)
        lay.setSpacing(6)
        head = QtWidgets.QHBoxLayout()
        head.setContentsMargins(12, 0, 8, 0)
        head.setSpacing(8)
        self.heads = []
        for key, w in (("", 22), ("ws.h_name", 150), ("ws.h_monitor", 140), ("gaps_in", 84), ("gaps_out", 84),
                       ("border_size", 84)):
            lb = QtWidgets.QLabel()
            lb.setObjectName("RowLabel")
            lb.setFixedWidth(w)
            lb.setProperty("tkey", key)
            if key:
                lb.setText(t(key))
            head.addWidget(lb)
            self.heads.append(lb)
        head.addStretch()
        lay.addLayout(head)
        self.rows = []
        self.lay = lay
        self._loading = False

    def retranslate(self, items: list):
        for lb in self.heads:
            if lb.property("tkey"):
                lb.setText(t(lb.property("tkey")))
        self.load(items)

    def load(self, items: list):
        self._loading = True
        for r in self.rows:
            r.hide()
            r.setParent(None)
            r.deleteLater()
        self.rows = []
        have = {w["id"]: w for w in items}
        mons = monitors()
        for w in model.default_workspaces():
            r = WsRow(have.get(w["id"], w), mons)
            r.changed.connect(self._changed)
            self.lay.addWidget(r)
            self.rows.append(r)
        self._loading = False

    def _changed(self):
        if not self._loading:
            self.changed.emit()

    def read(self) -> list:
        return model.clean_workspaces([r.w for r in self.rows if model.ws_used(r.w)])
