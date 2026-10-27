"""Settings › Advanced: yay with one click, the Kitty configurator (moved here from Hypr Studio) and an
Alacritty configurator in the same style."""

from . import alacritty, theme, usercmd, yay
from .form import Card, Page, start_job
from .i18n import lang, t
from .qt import QtWidgets, Qt, Signal
from .widgets import set_applied


def _hs_texts():
    """Texte der Alacritty-Karten auch in Hypr Studios Wörterbuch (seine Karten übersetzen sich selbst)."""
    from hyprstudio import i18n as hi
    from .i18n_extra import _ALA
    for lang_, d in _ALA.items():
        hi.TEXT[lang_].update(d)


SUBS = ("kitty", "alacritty", "yay")      # Settings › Advanced in der linken Spalte


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


def _alacritty_editor(win):
    from hyprstudio.kitty_page import KittyEditor
    _hs_texts()

    class AlacrittyEditor(KittyEditor):
        """Kitty's editor 1:1 (same presets, sections, preview, tiles), written as alacritty.toml.
        Options Alacritty doesn't have stay visible but disabled: „Not supported by Alacritty“."""
        CFG = "alacritty_k"
        PROGRAM = "alacritty"

        def make_state(self, saved):
            return alacritty.state(saved)

        def write(self):
            try:
                alacritty.apply(self.state, self.win.log)
                return True, ""
            except OSError as e:
                return False, str(e)

        def restore(self):
            return alacritty.undo(self.win.log)

        def view(self, s):
            return dict(s, tab_style="hidden", trail=0, _title="alacritty")

        def build_extra(self, key, lay):
            if key == "cursor":                      # nur Alacritty: Strichstärke, Blinken endet nach …
                c = self.card(lay, "ala.cursor_more")
                c.slider("thickness", 0.05, 0.5, 0.05, "{:.2f}", label_key="ala.thickness")
                c.slider("blink_timeout", 0, 30, 1, "{:.0f} s", label_key="ala.blink_timeout",
                         tip="ala.blink_timeout.tip")

        def apply(self):
            super().apply()
            if self.win.cfg.get(self.CFG + "_applied_sig") == self._sig():
                self.win.set_status(t("ala.applied", path=str(alacritty.target())))

        def retranslate(self):
            super().retranslate()
            for card in self.cards:
                g = card.grid
                for r in range(g.rowCount()):
                    item = g.itemAtPosition(r, 0)
                    lb = item.widget() if item else None
                    if lb is None or lb.property("skey") not in alacritty.UNSUPPORTED:
                        continue
                    lb.setText(f"{lb.text()}  ·  {t('ala.unsupported')}")
                    lb.setToolTip(t("ala.unsupported"))
                    for c in range(1, g.columnCount()):
                        w = g.itemAtPosition(r, c)
                        if w and w.widget():
                            w.widget().setEnabled(False)

    return AlacrittyEditor(win)


class TerminalPage(QtWidgets.QWidget):
    """Kitty and Alacritty: the same page (title, install hint, Hypr Studio's terminal editor)."""

    def __init__(self, host, program: str = "kitty"):
        super().__init__()
        from hyprstudio import i18n as hi
        from hyprstudio.kitty_page import KittyEditor
        hi.set_lang(lang())
        self.host, self.program = host, program
        lay = QtWidgets.QVBoxLayout(self)
        lay.setContentsMargins(0, 0, 0, 0)
        lay.setSpacing(12)
        self.title = QtWidgets.QLabel()
        self.title.setObjectName("PageTitle")
        trow = QtWidgets.QHBoxLayout()               # Titel + Zurücksetzen an derselben Stelle wie überall
        trow.setSpacing(6)
        trow.addWidget(self.title)
        from hyprstudio import section_reset
        self.b_reset = section_reset.button(self.reset_section)
        trow.addWidget(self.b_reset, 0, Qt.AlignmentFlag.AlignVCenter)
        trow.addStretch()
        lay.addLayout(trow)
        self.missing = _install_row(self, "adv." + program, program)
        lay.addWidget(self.missing)
        self.frame = QtWidgets.QFrame()          # Hypr Studio's look for its own widgets
        fl = QtWidgets.QVBoxLayout(self.frame)
        fl.setContentsMargins(0, 0, 0, 0)
        win = _KittyHost(host)
        self.editor = KittyEditor(win) if program == "kitty" else _alacritty_editor(win)
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
        self.title.setText(t("adv." + self.program))
        self.missing.retr()
        self.missing.check()
        self.editor.retranslate()
        if hasattr(self.editor, "btn_from"):
            self.editor.btn_from.hide()

    def reset_section(self):
        """Nur die offene Unterseite (Presets, Aussehen, Schrift …) auf Standard."""
        from hyprstudio import section_reset
        from hyprstudio.i18n import t as ht
        ed = self.editor
        sub = ed.win.cfg.get(ed.CFG + "_sub", "presets")
        page = ed.pages.get(sub)
        if page is not None:
            section_reset.reset(ed, page, ed.make_state(None), f"{t('adv.' + self.program)} › {ht('ks.' + sub)}",
                                f"{self.program}-{sub}")

    def refresh(self):
        self.missing.check()

    def finished(self, ok, action, _result):
        self.refresh()


def make(key: str, host):
    if key == "yay":
        return YayPage(host)
    return TerminalPage(host, key)
