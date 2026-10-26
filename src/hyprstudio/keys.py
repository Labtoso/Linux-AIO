"""Tastenkürzel-Editor: Zeilen aus Modifikatoren, aufgenommener Taste, Aktion und Ziel."""

import sys

from . import model, theme
from . import dialogs
from .i18n import TEXT, t
from .qt import QtCore, QtGui, QtWidgets, Qt, Signal
from .widgets import SearchCombo, draw_icon

K = Qt.Key
KEYNAMES = {
    K.Key_Return: "Return", K.Key_Enter: "Return", K.Key_Space: "space", K.Key_Tab: "Tab",
    K.Key_Backspace: "BackSpace", K.Key_Delete: "Delete", K.Key_Insert: "Insert", K.Key_Print: "Print",
    K.Key_Left: "left", K.Key_Right: "right", K.Key_Up: "up", K.Key_Down: "down",
    K.Key_Home: "Home", K.Key_End: "End", K.Key_PageUp: "Prior", K.Key_PageDown: "Next",
    K.Key_Comma: "comma", K.Key_Period: "period", K.Key_Minus: "minus", K.Key_Plus: "plus",
    K.Key_Slash: "slash", K.Key_Semicolon: "semicolon", K.Key_QuoteLeft: "grave",
    K.Key_VolumeUp: "XF86AudioRaiseVolume", K.Key_VolumeDown: "XF86AudioLowerVolume",
    K.Key_VolumeMute: "XF86AudioMute", K.Key_MediaPlay: "XF86AudioPlay", K.Key_MediaTogglePlayPause: "XF86AudioPlay",
    K.Key_MediaNext: "XF86AudioNext", K.Key_MediaPrevious: "XF86AudioPrev",
    K.Key_MonBrightnessUp: "XF86MonBrightnessUp", K.Key_MonBrightnessDown: "XF86MonBrightnessDown",
}
MOD_KEYS = (K.Key_Shift, K.Key_Control, K.Key_Alt, K.Key_Meta, K.Key_Super_L, K.Key_Super_R, K.Key_AltGr,
            K.Key_CapsLock)


def key_name(e: QtGui.QKeyEvent) -> str | None:
    k = e.key()
    sc = e.nativeScanCode()
    # Zahlenreihe über die Position, damit SHIFT + 1 nicht zu „!“ wird
    if sys.platform.startswith("linux") and 10 <= sc <= 19:
        return str((sc - 9) % 10)
    if sys.platform == "win32" and 2 <= sc <= 11:
        return str((sc - 1) % 10)
    if K.Key_A.value <= _v(k) <= K.Key_Z.value or K.Key_0.value <= _v(k) <= K.Key_9.value:
        return chr(_v(k))
    if K.Key_F1.value <= _v(k) <= K.Key_F24.value:
        return f"F{_v(k) - K.Key_F1.value + 1}"
    for qk, name in KEYNAMES.items():
        if _v(qk) == _v(k):
            return name
    txt = QtGui.QKeySequence(_v(k)).toString()
    return txt or None


def _v(k) -> int:
    return k.value if hasattr(k, "value") else int(k)


MAX_KEYS = 4
MOD_FLAGS = (("SUPER", Qt.KeyboardModifier.MetaModifier), ("CTRL", Qt.KeyboardModifier.ControlModifier),
             ("ALT", Qt.KeyboardModifier.AltModifier), ("SHIFT", Qt.KeyboardModifier.ShiftModifier))


def mods_of(m) -> list:
    on = [mod for mod, flag in MOD_FLAGS if m & flag]
    return [x for x in model.MODS if x in on]


def combo_label(mods: list, key: str) -> str:
    if key in model.SUPER_ALONE:
        return t("key.super_alone")
    keyl = t("key." + key) if key and ("key." + key) in TEXT["de"] else key
    order = [m for m in ("SUPER", "CTRL", "ALT", "SHIFT") if m in mods]
    return " + ".join([t("mod." + m) for m in order] + ([keyl] if key else []))


def parse_combo(text: str) -> tuple:
    """„Strg + Shift + S“ oder „SUPER SHIFT, S“ → (mods, key)."""
    parts = [x.strip() for x in text.replace(",", "+").replace(" ", "+").split("+") if x.strip()]
    if not parts:
        return [], ""
    alias = dict(model.MOD_ALIAS, STRG="CTRL", STEUERUNG="CTRL", UMSCHALT="SHIFT")
    mods = [alias[x.upper()] for x in parts[:-1] if x.upper() in alias]
    return [m for m in model.MODS if m in mods], parts[-1]


class KeyButton(QtWidgets.QPushButton):
    """Klicken, dann die ganze Kombination drücken (z. B. Strg + Shift + Alt + A, bis zu 4 Tasten).
    Esc bricht ab. Rechtsklick: Maus-/Medientasten oder von Hand eingeben."""
    captured = Signal(list, str)

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setObjectName("KeyBtn")
        self.setCursor(Qt.CursorShape.PointingHandCursor)
        self.setFixedWidth(260)
        self.mods, self.key = [], ""
        self.listening = False
        self.clicked.connect(self.start)
        self.setContextMenuPolicy(Qt.ContextMenuPolicy.CustomContextMenu)
        self.customContextMenuRequested.connect(self._menu)

    def set_combo(self, mods: list, key: str):
        self.mods, self.key = list(mods), key
        self.listening = False
        txt = combo_label(mods, key) if key else t("keys.none")
        self.setText(QtGui.QFontMetrics(self.font()).elidedText(txt, Qt.TextElideMode.ElideMiddle, 240))
        self.setToolTip(t("tip.keybtn") + (f"\n{model.combo_text({'mods': mods, 'key': key})}" if key else ""))

    def start(self):
        self.listening = True
        self._warn = False
        self._mods_down = set()  # in dieser Aufnahme gedrückte Zusatztasten
        self.setText(t("keys.press"))
        self.setFocus()
        self.grabKeyboard()

    def _stop(self):
        if self.listening:
            self.releaseKeyboard()
        self.set_combo(self.mods, self.key)

    def _live(self, m):
        if getattr(self, "_warn", False):
            return
        mods = mods_of(m)
        self.setText(combo_label(mods, "") + " + …" if mods else t("keys.press"))

    def _menu(self, pos):
        m = QtWidgets.QMenu(self)
        manual = m.addAction(t("keys.manual"))
        m.addSeparator()
        for k in model.SPECIAL_KEYS:
            m.addAction(t("key." + k) if ("key." + k) in TEXT["de"] else k).setData(k)
        a = m.exec(self.mapToGlobal(pos))
        if a is None:
            return
        if a is manual:
            txt, ok = dialogs.ask_text(self, t("keys.manual"), t("keys.manual_q"),
                                       model.combo_text({"mods": self.mods, "key": self.key}))
            mods, key = parse_combo(txt) if ok else ([], "")
            if key:
                self.set_combo(mods, key)
                self.captured.emit(mods, key)
        elif a.data() in model.SUPER_ALONE:
            self.set_combo(["SUPER"], a.data())
            self.captured.emit(self.mods, self.key)
        elif a.data():  # Sondertaste: gedrückte Zusatztasten bleiben
            self.set_combo(self.mods, a.data())
            self.captured.emit(self.mods, self.key)

    def keyPressEvent(self, e):
        if not self.listening:
            return super().keyPressEvent(e)
        if _v(e.key()) in {_v(k) for k in MOD_KEYS}:
            self._mods_down.add(_v(e.key()))
            self._live(e.modifiers())
            return
        self._mods_down.add(-1)  # normale Taste dabei → kein „Super allein“
        if _v(e.key()) == _v(K.Key_Escape) and not e.modifiers():
            self._stop()
            return
        name = key_name(e)
        if not name:
            return
        self._warn = False
        mods = mods_of(e.modifiers())
        if len(mods) + 1 > MAX_KEYS:
            self.setText(t("keys.max4"))
            self._warn = True
            return
        self.releaseKeyboard()
        self.set_combo(mods, name)
        self.captured.emit(mods, name)

    def keyReleaseEvent(self, e):
        supers = {_v(K.Key_Super_L), _v(K.Key_Super_R), _v(K.Key_Meta)}
        if self.listening and _v(e.key()) in supers and getattr(self, "_mods_down", set()) <= supers:
            # nur Super gedrückt und wieder losgelassen → Super allein
            key = "SUPER_R" if _v(e.key()) == _v(K.Key_Super_R) else "SUPER_L"
            self.releaseKeyboard()
            self.set_combo(["SUPER"], key)
            self.captured.emit(["SUPER"], key)
            return
        if self.listening and _v(e.key()) in {_v(k) for k in MOD_KEYS}:
            self._live(e.modifiers())
            return
        super().keyReleaseEvent(e)

    def focusOutEvent(self, e):
        self._stop()
        super().focusOutEvent(e)


class BindRow(QtWidgets.QFrame):
    changed = Signal()
    removed = Signal(object)
    duplicated = Signal(object)

    def __init__(self, b: dict, parent=None):
        super().__init__(parent)
        self.setObjectName("BindRow")
        lay = QtWidgets.QHBoxLayout(self)
        lay.setContentsMargins(10, 8, 8, 8)
        lay.setSpacing(6)
        self.keybtn = KeyButton()
        self.keybtn.set_combo(b["mods"], b["key"])
        self.keybtn.captured.connect(lambda *_: self.changed.emit())
        lay.addWidget(self.keybtn)
        lay.addSpacing(6)
        self.action = SearchCombo()
        self.action.setFixedWidth(220)
        self.action.setMaxVisibleItems(20)
        for a in model.ACTIONS:
            self.action.addItem(t("act." + a), a)
        self.action.setCurrentIndex(list(model.ACTIONS).index(b["action"]))
        self.action.currentIndexChanged.connect(self._action_changed)
        lay.addWidget(self.action)
        self.arg = SearchCombo()
        self.arg.setMinimumWidth(200)
        self.arg.setMaxVisibleItems(20)
        self.arg.currentTextChanged.connect(lambda _: self.changed.emit())
        lay.addWidget(self.arg, 1)
        lay.addStretch(0)
        self.warn = QtWidgets.QLabel("!")
        self.warn.setObjectName("Warn")
        self.warn.setFixedSize(22, 22)
        self.warn.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.warn.hide()
        lay.addWidget(self.warn)
        # Flags + Duplizieren im Menü
        self.more = QtWidgets.QToolButton()
        self.more.setPopupMode(QtWidgets.QToolButton.ToolButtonPopupMode.InstantPopup)
        self.more.setCursor(Qt.CursorShape.PointingHandCursor)
        menu = QtWidgets.QMenu(self.more)
        self.flag_acts = {}
        for f in model.FLAGS:
            a = menu.addAction(t("flag." + f))
            a.setCheckable(True)
            a.setChecked(f in b.get("flags", []))
            a.toggled.connect(self._flags_changed)
            self.flag_acts[f] = a
        menu.addSeparator()
        menu.addAction(t("keys.duplicate")).triggered.connect(lambda: self.duplicated.emit(self))
        self.more.setMenu(menu)
        lay.addWidget(self.more)
        self.rm = QtWidgets.QToolButton()
        self.rm.setIcon(draw_icon("close", theme.MUTED, 16))
        self.rm.setCursor(Qt.CursorShape.PointingHandCursor)
        self.rm.setToolTip(t("keys.remove"))
        self.rm.clicked.connect(lambda: self.removed.emit(self))
        lay.addWidget(self.rm)
        self._fill_arg(b["arg"])
        self._flags_changed()

    def _flags_changed(self, *_):
        on = [f for f, a in self.flag_acts.items() if a.isChecked()]
        self.more.setIcon(draw_icon("dots", theme.THEMES["blue"] if on else theme.MUTED, 16))
        self.more.setToolTip(t("keys.flags") + (": " + ", ".join(t("flag." + f) for f in on) if on else ""))
        self.changed.emit()

    def _fill_arg(self, value: str):
        a = self.action.currentData()
        kind = model.ACTIONS[a]
        self.arg.blockSignals(True)
        self.arg.clear()
        self.arg.setVisible(kind is not None)
        if kind == "dir":
            self.arg.setEditable(False)
            for d in model.DIRECTIONS:
                self.arg.addItem(t("dir." + d), d)
            self.arg.setCurrentIndex(max(0, self.arg.findData(value)))
        elif kind == "launcher":  # Auswahl: L Launcher, Rofi, Wofi, Fuzzel
            self.arg.setEditable(False)
            for k in model.LAUNCHER_CMDS:
                self.arg.addItem(t("app." + k), k)
            self.arg.setCurrentIndex(max(0, self.arg.findData(value)))
        elif kind:
            self.arg.setEditable(True)
            items = list(model.EXEC_SUGGEST) if kind == "cmd" else [str(i) for i in range(1, 11)]
            for it in items:
                self.arg.addItem(it, it)
            self.arg.setEditText(value or ("" if kind == "cmd" else "1"))
        self.arg.blockSignals(False)

    def _action_changed(self):
        self._fill_arg("")
        self.changed.emit()

    def value(self) -> dict:
        a = self.action.currentData()
        kind = model.ACTIONS[a]
        if kind in ("dir", "launcher"):
            arg = self.arg.currentData() or ""
        elif kind:
            arg = self.arg.currentText().strip()
        else:
            arg = ""
        return {"mods": list(self.keybtn.mods), "key": self.keybtn.key,
                "action": a, "arg": arg, "flags": [f for f, x in self.flag_acts.items() if x.isChecked()]}

    def text(self) -> str:
        v = self.value()
        return " ".join([model.combo_text(v), t("act." + v["action"]), v["arg"]]).lower()

    def set_warning(self, text: str | None):
        self.warn.setVisible(bool(text))
        self.warn.setToolTip(text or "")


class BindEditor(QtWidgets.QWidget):
    changed = Signal()
    saved = Signal()
    fix_requested = Signal()
    import_requested = Signal()

    def __init__(self, parent=None):
        super().__init__(parent)
        lay = QtWidgets.QVBoxLayout(self)
        lay.setContentsMargins(0, 0, 0, 0)
        lay.setSpacing(10)
        top = QtWidgets.QHBoxLayout()
        top.setSpacing(8)
        self.btn_add = QtWidgets.QPushButton()
        self.btn_tpl = QtWidgets.QPushButton()
        self.tpl_menu = QtWidgets.QMenu(self.btn_tpl)
        self.tpl_menu.aboutToShow.connect(self._fill_menu)
        self.btn_tpl.setMenu(self.tpl_menu)
        self.user_store = lambda: {}
        self.btn_import = QtWidgets.QPushButton()
        self.btn_fix = QtWidgets.QPushButton()
        self.btn_fix.setObjectName("Danger")
        for b in (self.btn_add, self.btn_tpl, self.btn_import, self.btn_fix):
            b.setCursor(Qt.CursorShape.PointingHandCursor)
            top.addWidget(b)
        top.addStretch()
        self.count = QtWidgets.QLabel()
        self.count.setObjectName("RowLabel")
        top.addWidget(self.count)
        # Suchleiste oben über die ganze Breite
        self.search = QtWidgets.QLineEdit()
        self.search.setObjectName("Search")
        self.search.setMinimumHeight(40)
        self.search.setClearButtonEnabled(True)
        self.search.textChanged.connect(self._filter)
        lay.addWidget(self.search)
        lay.addLayout(top)
        self.rows_lay = QtWidgets.QVBoxLayout()
        self.rows_lay.setSpacing(6)
        lay.addLayout(self.rows_lay)
        self.rows: list[BindRow] = []
        self.btn_add.clicked.connect(self.add_empty)
        self.btn_import.clicked.connect(self.import_requested.emit)
        self.btn_fix.clicked.connect(self.fix_requested.emit)
        self.btn_fix.hide()

    def load(self, binds: list):
        for r in self.rows:
            r.hide()
            r.setParent(None)
            r.deleteLater()
        self.rows = []
        seen = set()
        for b in binds:
            c = model.combo(b["mods"], b["key"])
            if b["key"] and c in seen:
                continue
            seen.add(c)
            self._add_row(b)
        self._filter()

    def merge(self, binds: list):
        """Neue Kürzel anhängen; schon vorhandene Tastenkombinationen bleiben unverändert."""
        self.load(self.read() + binds)
        self.changed.emit()

    def _add_row(self, b: dict, after: BindRow | None = None) -> BindRow:
        r = BindRow(b)
        r.changed.connect(self.changed.emit)
        r.removed.connect(self._remove)
        r.duplicated.connect(self._duplicate)
        if after is not None:
            i = self.rows.index(after) + 1
            self.rows_lay.insertWidget(i, r)
            self.rows.insert(i, r)
        else:
            self.rows_lay.addWidget(r)
            self.rows.append(r)
        return r

    def add_empty(self):
        self.search.clear()
        r = self._add_row({"mods": [], "key": "", "action": "exec", "arg": "", "flags": []})
        r.keybtn.start()
        self.changed.emit()

    def _duplicate(self, row):
        v = row.value()
        v["key"] = ""
        r = self._add_row(v, after=row)
        r.keybtn.start()
        self.changed.emit()

    def _remove(self, row):
        self.rows.remove(row)
        row.hide()
        row.setParent(None)
        row.deleteLater()
        self.changed.emit()

    def _filter(self, *_):
        words = self.search.text().strip().lower().split()
        self.setUpdatesEnabled(False)  # alle Zeilen auf einmal umschalten statt bei jeder neu zu zeichnen
        for r in self.rows:
            show = not words or all(w in r.text() for w in words)
            if r.isVisibleTo(self) != show:
                r.setVisible(show)
        self.setUpdatesEnabled(True)

    def read(self) -> list:
        return [r.value() for r in self.rows]

    def mark(self, conflicts: set):
        """Doppelte Kombinationen und Treffer in hyprland.lua markieren."""
        counts = {}
        for r in self.rows:
            v = r.value()
            if v["key"]:
                c = model.combo(v["mods"], v["key"])
                counts[c] = counts.get(c, 0) + 1
        for r in self.rows:
            v = r.value()
            c = model.combo(v["mods"], v["key"])
            if v["key"] and counts.get(c, 0) > 1:
                r.set_warning(t("keys.dup"))
            elif v["key"] and c in conflicts:
                r.set_warning(t("keys.conflict"))
            else:
                r.set_warning(None)
        self.btn_fix.setVisible(bool(conflicts))
        self.count.setText(t("keys.count", n=len([r for r in self.rows if model.bind_ok(r.value())])))

    def retranslate(self):
        self.btn_add.setText(t("keys.add"))
        self.btn_tpl.setText(t("keys.templates"))
        self.btn_import.setText(t("keys.import"))
        self.btn_import.setToolTip(t("tip.keys.import"))
        self.btn_fix.setText(t("keys.fix"))
        self.btn_fix.setToolTip(t("tip.keys.fix"))
        self.search.setPlaceholderText(t("search"))
        self.load(self.read())

    def _fill_menu(self):
        """Mitgelieferte Vorlagen (anhängen) + eigene Kürzel-Sets (laden, speichern, entfernen)."""
        m = self.tpl_menu
        m.clear()
        for name in model.TEMPLATES:
            m.addAction(t("tpl." + name)).triggered.connect(
                lambda _=False, n=name: self.merge([dict(b) for b in model.TEMPLATES[n]]))
        m.addAction(t("tpl.all")).triggered.connect(lambda: self.merge(
            [dict(b) for n in model.TEMPLATES for b in model.TEMPLATES[n]]))
        m.addSeparator()
        sets = self.user_store()
        if sets:
            m.addSection(t("up.mine"))
            for name in sorted(sets, key=str.lower):
                m.addAction(t("up.load", name=name)).triggered.connect(lambda _=False, n=name: self._load_set(n))
        m.addAction(t("up.save_binds")).triggered.connect(self._save_set)
        if sets:
            rm = m.addMenu(t("up.remove_set"))
            for name in sorted(sets, key=str.lower):
                rm.addAction(name).triggered.connect(lambda _=False, n=name: self._remove_set(n))

    def _save_set(self):
        name, ok = dialogs.ask_text(self, t("up.save_binds"), t("up.name"))
        if ok and name.strip():
            self.user_store()[name.strip()] = model.clean_binds(self.read())
            self.saved.emit()

    def _load_set(self, name: str):
        self.load(model.clean_binds(self.user_store().get(name, [])))
        self.changed.emit()

    def _remove_set(self, name: str):
        if dialogs.confirm(self, t("up.remove_set"), t("up.remove_q", name=name), t("up.remove"), danger=True):
            self.user_store().pop(name, None)
            self.saved.emit()
