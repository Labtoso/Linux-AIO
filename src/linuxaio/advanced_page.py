"""Settings › Advanced: yay with one click, the Kitty configurator (moved here from Hypr Studio) and an
Alacritty configurator in the same style."""

import json

from hyprstudio.widgets import Switch

from . import alacritty, theme, usercmd, yay
from .form import Card, Page, Row, start_job
from .i18n import lang, t
from .qt import QtCore, QtGui, QtWidgets, Qt, Signal
from .widgets import Segmented, set_applied

SUBS = ("yay", "kitty", "alacritty")


def _install_row(page, key: str, pkg: str):
    """„not installed“ + install button (root helper, fixed package key)."""
    box = QtWidgets.QFrame()
    box.setObjectName("Card")
    lay = QtWidgets.QHBoxLayout(box)
    lay.setContentsMargins(16, 10, 16, 10)
    lb = QtWidgets.QLabel()
    lb.setWordWrap(True)
    b = QtWidgets.QPushButton()
    b.setObjectName("Primary")
    b.setCursor(Qt.CursorShape.PointingHandCursor)
    b.clicked.connect(lambda: page.host.run(["pkg-install", pkg], "pkg-install", page))
    lay.addWidget(lb, 1)
    lay.addWidget(b)
    box.retr = lambda: (lb.setText(t(key + ".missing")), b.setText(t("install")))
    box.check = lambda: box.setVisible(not usercmd.have(pkg))
    return box


# ---------- yay ----------

class YayPage(Page):
    line = Signal(str)                          # Protokollzeilen aus dem Bau-Thread

    def __init__(self, host):
        super().__init__("adv.yay")
        self.host = host
        self.line.connect(host.log)
        card = Card("adv.yay.card")
        self.desc = QtWidgets.QLabel()
        self.desc.setWordWrap(True)
        self.desc.setObjectName("RowLabel")
        card.lay.addWidget(self.desc)
        row = QtWidgets.QHBoxLayout()
        row.setContentsMargins(0, 10, 0, 0)
        self.state = QtWidgets.QLabel()
        self.btn = QtWidgets.QPushButton()
        self.btn.setObjectName("Primary")
        self.btn.setCursor(Qt.CursorShape.PointingHandCursor)
        self.btn.setMinimumWidth(170)
        self.btn.clicked.connect(self.install)
        row.addWidget(self.state, 1)
        row.addWidget(self.btn)
        card.lay.addLayout(row)
        self.bar = QtWidgets.QProgressBar()
        self.bar.setRange(0, 0)
        self.bar.setTextVisible(False)
        self.bar.hide()
        card.lay.addWidget(self.bar)
        self.step = QtWidgets.QLabel()
        self.step.setObjectName("Label")
        card.lay.addWidget(self.step)
        self.body.addWidget(card)
        self.busy = False
        self.finish()

    def retranslate(self):
        super().retranslate()
        self.desc.setText(t("adv.yay.desc"))
        self.refresh()

    def refresh(self):
        if not yay.supported():
            self.state.setText(t("adv.yay.arch"))
            self.btn.hide()
            return
        inst = yay.installed()
        self.state.setText(t("adv.yay.ok") if inst else t("adv.yay.no"))
        set_applied(self.btn, inst, t("installed"))
        if not inst:
            self.btn.setText(t("install"))
            self.btn.setEnabled(not self.busy)

    def _busy(self, on: bool, step: str = ""):
        self.busy = on
        self.bar.setVisible(on)
        self.step.setText(step)
        self.btn.setEnabled(not on)

    def install(self):
        if self.busy:
            return
        self.host.log("")
        if yay.deps_missing():                   # git/base-devel zuerst
            self._busy(True, t("adv.yay.s1"))
            self._next = "build"
            self.host.run(["pkg-install", "yay-deps"], "pkg-install", self)
            return
        self.build()

    def build(self):
        self._busy(True, t("adv.yay.s2"))

        def done(ok, res):
            if not ok:
                self._busy(False, "")
                self.host.log(str(res))
                self.host.set_status(f"{t('failed')}: {res}")
                self.refresh()
                return
            self._busy(True, t("adv.yay.s3"))
            self._next = "end"
            self.host.run(["yay-install", str(res)], "yay-install", self)
        start_job(self, lambda: yay.build(self.line.emit), done)

    def finished(self, ok, action, _result):
        nxt, self._next = getattr(self, "_next", ""), ""
        if ok and nxt == "build":
            self.build()
            return
        self._busy(False, "")
        if ok and action == "yay-install":
            self.host.set_status(t("adv.yay.done"))
        self.refresh()


# ---------- Kitty (Hypr Studio's editor) ----------

class _KittyHost:
    """What Hypr Studio's KittyEditor expects from its window, served by Linux AIO."""

    def __init__(self, host):
        self.host = host
        self.cfg = host.cfg
        self.hypr_page = None

    def accent(self):
        return self.host.accent()

    def save_later(self):
        from .widgets import save_cfg
        save_cfg(self.cfg)

    def log(self, text):
        self.host.log(text)

    def set_status(self, text):
        self.host.set_status(text)

    def report(self, ok, info):
        from hyprstudio.i18n import t as ht
        self.host.set_status(t("done") if ok else f"{t('failed')}: {ht(info)}")


class KittyPage(QtWidgets.QWidget):
    def __init__(self, host):
        super().__init__()
        from hyprstudio import i18n as hi
        from hyprstudio.kitty_page import KittyEditor
        hi.set_lang(lang())
        self.host = host
        lay = QtWidgets.QVBoxLayout(self)
        lay.setContentsMargins(0, 0, 0, 0)
        lay.setSpacing(12)
        self.title = QtWidgets.QLabel()
        self.title.setObjectName("PageTitle")
        lay.addWidget(self.title)
        self.missing = _install_row(self, "adv.kitty", "kitty")
        lay.addWidget(self.missing)
        self.frame = QtWidgets.QFrame()          # Hypr Studio's look for its own widgets
        fl = QtWidgets.QVBoxLayout(self.frame)
        fl.setContentsMargins(0, 0, 0, 0)
        self.editor = KittyEditor(_KittyHost(host))
        if hasattr(self.editor, "btn_from"):     # „Farbe aus Hyprland“ gibt es hier nicht
            self.editor.btn_from.hide()
        fl.addWidget(self.editor)
        lay.addWidget(self.frame, 1)
        self.apply_theme()

    def apply_theme(self):
        from hyprstudio import theme as htheme
        acc = self.host.accent()
        self.frame.setStyleSheet(theme.scale_qss(htheme.qss(acc), self.host.ui_factor()) +
                                 " QFrame#Card { background: %s; }" % theme.SURFACE)
        self.editor.apply_theme(acc)

    def retranslate(self):
        from hyprstudio import i18n as hi
        hi.set_lang(lang())
        self.title.setText(t("adv.kitty"))
        self.missing.retr()
        self.missing.check()
        self.editor.retranslate()
        if hasattr(self.editor, "btn_from"):
            self.editor.btn_from.hide()

    def refresh(self):
        self.missing.check()

    def finished(self, ok, action, _result):
        self.refresh()


# ---------- Alacritty ----------

class AlacrittyPreview(QtWidgets.QWidget):
    def __init__(self):
        super().__init__()
        self.s = alacritty.state(None)
        self.setMinimumHeight(190)

    def set_state(self, s):
        self.s = dict(s)
        self.update()

    def paintEvent(self, _e):
        s, c = self.s, alacritty.PRESETS[self.s["preset"]]
        p = QtGui.QPainter(self)
        p.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
        r = QtCore.QRectF(self.rect()).adjusted(1, 1, -1, -1)
        p.setPen(QtGui.QPen(QtGui.QColor(theme.BORDER), 1))
        p.setBrush(QtGui.QColor(theme.SURFACE2))
        p.drawRoundedRect(r, 12, 12)
        bg = QtGui.QColor(c["bg"])
        bg.setAlphaF(float(s["opacity"]))
        p.setBrush(bg)
        p.setPen(Qt.PenStyle.NoPen)
        win = r.adjusted(14, 14, -14, -14)
        p.drawRoundedRect(win, 10, 10)
        f = QtGui.QFont(s["font"])
        f.setStyleHint(QtGui.QFont.StyleHint.Monospace)
        f.setPointSizeF(max(6.0, float(s["size"])))
        p.setFont(f)
        fm = QtGui.QFontMetricsF(f)
        pad = int(s["padding"])
        x0, y = win.left() + pad, win.top() + pad + fm.ascent()
        n, b = c["normal"], c["bright"]
        lines = [[(c["fg"], "~ "), (n[4], "❯ "), (c["fg"], "ls --color")],
                 [(n[4], "Documents  "), (n[2], "build.sh  "), (n[5], "Music  "), (n[6], "notes.md")],
                 [(c["fg"], "~ "), (n[4], "❯ "), (c["fg"], "git status")],
                 [(n[1], "modified:  "), (c["fg"], "alacritty.toml")],
                 [(b[0], "# " + c["title"])],
                 [(c["fg"], "~ "), (n[4], "❯ ")]]
        for i, ln in enumerate(lines):
            if y > win.bottom() - 4:
                break
            x = x0
            for col, text in ln:
                p.setPen(QtGui.QColor(col))
                p.drawText(QtCore.QPointF(x, y), text)
                x += fm.horizontalAdvance(text)
            if i == len(lines) - 1:              # Cursor
                cw, ch = fm.horizontalAdvance("M"), fm.height()
                top = y - fm.ascent()
                rect = {"Block": QtCore.QRectF(x, top, cw, ch), "Underline": QtCore.QRectF(x, y + 2, cw, 2)}.get(
                    s["cursor"], QtCore.QRectF(x, top, 2, ch))
                p.fillRect(rect, QtGui.QColor(c["cursor"]))
            y += fm.lineSpacing()
        p.end()


class PresetButton(QtWidgets.QPushButton):
    def __init__(self, key):
        super().__init__()
        self.key = key
        c = alacritty.PRESETS[key]
        self.setCheckable(True)
        self.setCursor(Qt.CursorShape.PointingHandCursor)
        self.setMinimumHeight(40)
        pm = QtGui.QPixmap(52, 14)
        pm.fill(Qt.GlobalColor.transparent)
        q = QtGui.QPainter(pm)
        q.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
        for i, col in enumerate([c["bg"]] + c["normal"][1:6]):
            q.setBrush(QtGui.QColor(col))
            q.setPen(QtGui.QPen(QtGui.QColor(theme.BORDER), 1))
            q.drawEllipse(QtCore.QRectF(i * 8 + 1, 1, 12, 12))
        q.end()
        self.setIcon(QtGui.QIcon(pm))
        self.setIconSize(pm.size())
        self.setText("  " + c["title"])
        self.setStyleSheet(f"QPushButton:checked {{ background: {theme.RAISED}; border-color: {theme.LINE};"
                           " font-weight: 600; }")


class AlacrittyPage(Page):
    def __init__(self, host):
        super().__init__("adv.alacritty")
        self.host = host
        self.s = alacritty.state(host.cfg.get("alacritty"))
        self.missing = _install_row(self, "adv.alacritty", "alacritty")
        self.body.addWidget(self.missing)
        top = QtWidgets.QHBoxLayout()
        top.setSpacing(14)
        self.preview = AlacrittyPreview()
        top.addWidget(self.preview, 1)
        col = QtWidgets.QVBoxLayout()
        col.setSpacing(8)
        col.addStretch()
        self.btn_undo = QtWidgets.QPushButton()
        self.btn_undo.clicked.connect(self.undo)
        self.btn_apply = QtWidgets.QPushButton()
        self.btn_apply.setObjectName("Primary")
        self.btn_apply.clicked.connect(self.apply)
        for b in (self.btn_undo, self.btn_apply):
            b.setFixedSize(170, 40)
            b.setCursor(Qt.CursorShape.PointingHandCursor)
            col.addWidget(b)
        top.addLayout(col)
        self.body.addLayout(top)

        pc = Card("ala.presets")
        grid = QtWidgets.QGridLayout()
        grid.setSpacing(8)
        self.presets = QtWidgets.QButtonGroup(self)
        for i, key in enumerate(alacritty.PRESETS):
            b = PresetButton(key)
            b.clicked.connect(lambda _=False, k=key: self.set("preset", k))
            self.presets.addButton(b)
            grid.addWidget(b, i // 3, i % 3)
        pc.lay.addLayout(grid)
        self.body.addWidget(pc)

        fc = Card("ala.font")
        self.font = QtWidgets.QFontComboBox()
        self.font.setObjectName("Combo")
        self.font.setFontFilters(QtWidgets.QFontComboBox.FontFilter.MonospacedFonts)
        self.font.setMinimumWidth(240)
        self.font.currentFontChanged.connect(lambda f: self.set("font", f.family()))
        self.size = QtWidgets.QDoubleSpinBox()
        self.size.setObjectName("Spin")
        self.size.setRange(6, 32)
        self.size.setSingleStep(0.5)
        self.size.setDecimals(1)
        self.size.valueChanged.connect(lambda v: self.set("size", v))
        self.rows = [Row("ala.family", self.font), Row("ala.size", self.size)]

        wc = Card("ala.window")
        self.pad = QtWidgets.QSpinBox()
        self.pad.setObjectName("Spin")
        self.pad.setRange(0, 60)
        self.pad.valueChanged.connect(lambda v: self.set("padding", v))
        self.opac = QtWidgets.QSlider(Qt.Orientation.Horizontal)
        self.opac.setRange(30, 100)
        self.opac.setFixedWidth(200)
        self.opac.valueChanged.connect(lambda v: self.set("opacity", v / 100))
        self.rows += [Row("ala.padding", self.pad), Row("ala.opacity", self.opac)]

        cc = Card("ala.cursor")
        self.shape = QtWidgets.QComboBox()
        self.shape.setObjectName("Combo")
        for k in ("Block", "Beam", "Underline"):
            self.shape.addItem(k, k)
        self.shape.currentIndexChanged.connect(lambda _i: self.set("cursor", self.shape.currentData()))
        self.blink = Switch()
        self.blink.toggled.connect(lambda on: self.set("blink", bool(on)))
        self.rows += [Row("ala.shape", self.shape), Row("ala.blink", self.blink)]
        for card, rows in ((fc, self.rows[0:2]), (wc, self.rows[2:4]), (cc, self.rows[4:6])):
            for r in rows:
                card.lay.addWidget(r)
            self.body.addWidget(card)
        self.load()
        self.finish()

    def load(self):
        s = self.s
        for w in (self.font, self.size, self.pad, self.opac, self.shape, self.blink):
            w.blockSignals(True)
        self.font.setCurrentFont(QtGui.QFont(s["font"]))
        if self.font.currentFont().family() != s["font"]:      # noch nicht installiert: Namen trotzdem zeigen
            self.font.setEditText(s["font"])
        self.size.setValue(float(s["size"]))
        self.pad.setValue(int(s["padding"]))
        self.opac.setValue(round(float(s["opacity"]) * 100))
        self.shape.setCurrentIndex(max(0, self.shape.findData(s["cursor"])))
        self.blink.setChecked(bool(s["blink"]))
        for w in (self.font, self.size, self.pad, self.opac, self.shape, self.blink):
            w.blockSignals(False)
        for b in self.presets.buttons():
            b.setChecked(b.key == s["preset"])
        self.preview.set_state(s)
        self.update_applied()

    def set(self, key, value):
        self.s[key] = value
        self.host.cfg["alacritty"] = self.s
        if key == "preset":
            self.s.update(alacritty.PRESETS[value].get("look", {}))   # z. B. „01“: auch Schrift, Abstand, Deckkraft
            self.load()
            return
        self.preview.set_state(self.s)
        self.update_applied()

    def _sig(self):
        return json.dumps(self.s, sort_keys=True)

    def update_applied(self):
        on = self.host.cfg.get("alacritty_sig") == self._sig()
        set_applied(self.btn_apply, on, t("applied"))
        if not on:
            self.btn_apply.setText(t("apply"))
            self.btn_apply.setEnabled(True)

    def apply(self):
        self.host.log("")
        fam = self.s["font"]
        if fam.endswith(" Nerd Font") and fam not in QtGui.QFontDatabase.families():
            from . import fonts                    # fehlende Nerd Font zuerst holen (offizielles Release)
            self.btn_apply.setEnabled(False)
            self.host.set_status(t("miss.installing", name=fam))

            def get():
                item = next((i for i in fonts.nerd_catalog() if i["family"] == fam), None)
                if item is None:
                    raise RuntimeError(f"{fam}: not in the Nerd Fonts release")
                return fonts.install(item)

            def done(ok, res):
                if not ok:
                    self.host.log(str(res))
                QtGui.QFontDatabase.removeAllApplicationFonts()
                self._write()
            start_job(self, get, done)
            return
        self._write()

    def _write(self):
        f = alacritty.apply(self.s, self.host.log)
        self.host.cfg["alacritty_sig"] = self._sig()
        from .widgets import save_cfg
        save_cfg(self.host.cfg)
        self.host.set_status(t("ala.applied", path=str(f)))
        self.update_applied()

    def undo(self):
        f = alacritty.target()
        bak = f.with_name(f.name + ".linuxaio-original")
        self.host.log("")
        if bak.is_file():
            f.write_bytes(bak.read_bytes())
            self.host.log(f"{bak} → {f}")
        elif f.is_file() and f.read_text(encoding="utf-8", errors="replace").startswith(alacritty.MARK):
            f.unlink()
            self.host.log(f"− {f}")
        else:
            self.host.set_status(t("nothing"))
            return
        self.host.cfg["alacritty_sig"] = ""
        self.host.set_status(t("undone"))
        self.update_applied()

    def retranslate(self):
        super().retranslate()
        for r in self.rows:
            r.retranslate()
        self.missing.retr()
        self.missing.check()
        self.btn_undo.setText(t("ala.undo"))
        self.update_applied()

    def refresh(self):
        self.missing.check()

    def finished(self, ok, action, _result):
        self.refresh()


# ---------- Container ----------

class AdvancedPage(QtWidgets.QWidget):
    def __init__(self, host):
        super().__init__()
        self.host = host
        lay = QtWidgets.QVBoxLayout(self)
        lay.setContentsMargins(0, 0, 0, 0)
        lay.setSpacing(12)
        self.seg = Segmented(SUBS, prefix="adv.")
        self.seg.changed.connect(self.show_sub)
        lay.addWidget(self.seg, 0, Qt.AlignmentFlag.AlignLeft)
        self.stack = QtWidgets.QStackedWidget()
        lay.addWidget(self.stack, 1)
        self.pages = {}
        self.show_sub(host.cfg.get("adv_sub", "yay"))

    def show_sub(self, key):
        if key not in SUBS:
            key = "yay"
        if key not in self.pages:
            pg = {"yay": YayPage, "kitty": KittyPage, "alacritty": AlacrittyPage}[key](self.host)
            self.pages[key] = pg
            self.stack.addWidget(pg)
            pg.retranslate()
            from .form import accent_switches
            accent_switches(pg, self.host.accent())
        self.seg.select(key)
        self.stack.setCurrentWidget(self.pages[key])
        self.host.cfg["adv_sub"] = key
        self.seg.retranslate()

    def retranslate(self):
        self.seg.retranslate()
        for pg in self.pages.values():
            pg.retranslate()

    def refresh(self):
        for pg in self.pages.values():
            pg.refresh()

    def apply_theme(self, *_a):
        for pg in self.pages.values():
            if hasattr(pg, "apply_theme"):
                pg.apply_theme()
