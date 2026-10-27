"""Sonstiges › Kitty: Terminal-Vorschau, Presets und Einstellungen in Unterseiten."""

import json

from . import kitty, model, theme
from .i18n import t
from .preview import _wallpaper, qcolor
from .qt import QtCore, QtGui, QtWidgets, Qt
from .userpresets import PREFIX, UserPresetsMixin
from .widgets import Card, ColorButton, FlowGrid, PresetCard, Segmented, set_chip

DEMO = (
    [("user", 4), ("@", 7), ("arch ", 4), ("~/projects ", 6), ("❯ ", 5), ("ls", 15)],
    [("Pictures/  ", 4), ("Music/  ", 4), ("hypr-studio.pyz  ", 2), ("notes.md  ", 15), ("start.sh", 2)],
    [("user", 4), ("@", 7), ("arch ", 4), ("~/projects ", 6), ("❯ ", 5), ("git status", 15)],
    [("On branch ", 7), ("main", 2)],
    [("modified:  ", 1), ("src/gui.py", 3)],
    [("user", 4), ("@", 7), ("arch ", 4), ("~/projects ", 6), ("❯ ", 5)],
)
TABS = ("1: zsh", "2: nvim", "3: htop")


def _mono_font(s: dict, px: float) -> QtGui.QFont:
    f = QtGui.QFont(s["font"])
    f.setStyleHint(QtGui.QFont.StyleHint.Monospace)
    if not QtGui.QFontInfo(f).fixedPitch():
        f = QtGui.QFont("JetBrains Mono")
        f.setStyleHint(QtGui.QFont.StyleHint.Monospace)
        if not QtGui.QFontInfo(f).fixedPitch():
            f = QtGui.QFont("Consolas")
    f.setPixelSize(max(4, int(round(px))))
    return f


def paint_terminal(p: QtGui.QPainter, r: QtCore.QRectF, s: dict, trail_phase: float = 1.0):
    p.save()
    p.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
    clip = QtGui.QPainterPath()
    clip.addRoundedRect(r, 12, 12)
    p.setClipPath(clip)
    wall, blur = _wallpaper(max(1, int(r.width())), max(1, int(r.height())))
    p.drawPixmap(r.topLeft(), wall)
    k = r.width() / 900
    win = r.adjusted(r.width() * 0.07, r.height() * 0.08, -r.width() * 0.07, -r.height() * 0.08)
    # Schatten
    p.setPen(Qt.PenStyle.NoPen)
    for i in range(5):
        p.setBrush(QtGui.QColor(0, 0, 0, 22))
        g = (i + 1) * 3 * k
        p.drawRoundedRect(win.adjusted(-g, -g + 3 * k, g, g + 3 * k), 10 + g, 10 + g)
    path = QtGui.QPainterPath()
    path.addRoundedRect(win, 10 * k + 2, 10 * k + 2)
    p.save()
    p.setClipPath(path, Qt.ClipOperation.IntersectClip)
    if s["blur"] and s["opacity"] < 1:
        p.drawPixmap(r.topLeft(), blur)
    bg = qcolor(s["bg"])
    bg.setAlphaF(max(0.0, min(1.0, float(s["opacity"]))))
    p.fillRect(win, bg)
    inner = QtCore.QRectF(win)
    if s["decorations"] == "no":
        tb = QtCore.QRectF(win.left(), win.top(), win.width(), 26 * k + 6)
        p.fillRect(tb, QtGui.QColor(20, 22, 30, 235))
        f = p.font()
        f.setPixelSize(max(4, int(11 * k + 3)))
        p.setFont(f)
        p.setPen(QtGui.QColor("#aab4c3"))
        p.drawText(tb, Qt.AlignmentFlag.AlignCenter, s.get("_title", "kitty"))
        for i, col in enumerate(("#e5534b", "#d29922", "#3fb950")):
            p.setBrush(QtGui.QColor(col))
            p.setPen(Qt.PenStyle.NoPen)
            d = 9 * k + 2
            p.drawEllipse(QtCore.QPointF(tb.right() - (14 + i * 18) * k - 4, tb.center().y()), d / 2, d / 2)
        inner.setTop(tb.bottom())
    font = _mono_font(s, float(s["font_size"]) * 1.45 * k + 2)
    p.setFont(font)
    fm = QtGui.QFontMetricsF(font)
    # Tab-Leiste
    th = fm.height() + 6 * k
    if s["tab_style"] != "hidden":
        bar = QtCore.QRectF(inner.left(), inner.top() if s["tab_edge"] == "top" else inner.bottom() - th,
                            inner.width(), th)
        p.fillRect(bar, qcolor(s["tab_inactive_bg"]))
        x = bar.left()
        for i, title in enumerate(TABS):
            w = fm.horizontalAdvance(title) + 22 * k
            active = i == 0
            bgc = qcolor(s["tab_active_bg"] if active else s["tab_inactive_bg"])
            fgc = qcolor(s["tab_active_fg"] if active else s["tab_inactive_fg"])
            rect = QtCore.QRectF(x, bar.top(), w, bar.height())
            p.setPen(Qt.PenStyle.NoPen)
            p.setBrush(bgc)
            if s["tab_style"] == "powerline":
                tip = bar.height() * 0.5
                poly = QtGui.QPainterPath(rect.topLeft())
                if s["tab_powerline"] == "round":
                    poly.addRoundedRect(rect.adjusted(0, 0, tip * 0.6, 0), tip, tip)
                else:
                    poly.lineTo(rect.topRight())
                    if s["tab_powerline"] == "slanted":
                        poly.lineTo(rect.right() + tip, rect.top())
                        poly.lineTo(rect.right(), rect.bottom())
                    else:
                        poly.lineTo(rect.right() + tip, rect.center().y())
                        poly.lineTo(rect.right(), rect.bottom())
                    poly.lineTo(rect.bottomLeft())
                    poly.closeSubpath()
                p.drawPath(poly)
                x += w + tip
            elif s["tab_style"] == "slant":
                sl = bar.height() * 0.45
                p.drawPolygon(QtGui.QPolygonF([QtCore.QPointF(rect.left() + sl, rect.top()),
                                               QtCore.QPointF(rect.right() + sl, rect.top()),
                                               QtCore.QPointF(rect.right(), rect.bottom()),
                                               QtCore.QPointF(rect.left(), rect.bottom())]))
                x += w + sl
            else:
                if active:
                    p.drawRect(rect)
                if s["tab_style"] == "separator" and i:
                    p.setPen(QtGui.QPen(fgc, 1))
                    p.drawLine(QtCore.QPointF(rect.left(), rect.top() + 4), QtCore.QPointF(rect.left(), rect.bottom() - 4))
                x += w
            p.setPen(fgc)
            p.drawText(rect, Qt.AlignmentFlag.AlignCenter, title)
        if s["tab_edge"] == "top":
            inner.setTop(bar.bottom())
        else:
            inner.setBottom(bar.top())
    # Text
    pad = float(s["padding"]) * k
    tx, ty = inner.left() + pad + 6 * k, inner.top() + pad + 4 * k
    pal = [qcolor(c) for c in s["palette"]]
    cur_x = cur_y = 0.0
    for line in DEMO:
        x = tx
        for text, ci in line:
            p.setPen(qcolor(s["fg"]) if ci == 15 else pal[ci])
            p.drawText(QtCore.QPointF(x, ty + fm.ascent()), text)
            x += fm.horizontalAdvance(text)
        cur_x, cur_y = x, ty
        ty += fm.height() * 1.15
    # Farbpalette als Kästchen
    sw = fm.height() * 0.9
    for i in range(16):
        p.fillRect(QtCore.QRectF(tx + (i % 8) * (sw + 3), ty + (i // 8) * (sw + 3) + 4, sw, sw), pal[i])
    # Cursor + Spur
    cw, ch = fm.horizontalAdvance("M"), fm.height()
    cc = qcolor(s["cursor"])
    if s["trail"] and trail_phase < 1.0:
        for j in range(4):
            g = QtGui.QColor(cc)
            g.setAlpha(int(110 * (1 - trail_phase) * (1 - j / 4)))
            p.fillRect(QtCore.QRectF(cur_x - (j + 1) * cw * 2.2 * (1 - trail_phase), cur_y, cw, ch), g)
    if s["cursor_shape"] == "beam":
        p.fillRect(QtCore.QRectF(cur_x, cur_y, max(1.5, 2 * k), ch), cc)
    elif s["cursor_shape"] == "underline":
        p.fillRect(QtCore.QRectF(cur_x, cur_y + ch - max(1.5, 2 * k), cw, max(1.5, 2 * k)), cc)
    else:
        p.fillRect(QtCore.QRectF(cur_x, cur_y, cw, ch), cc)
    p.restore()
    p.restore()


class TerminalPreview(QtWidgets.QWidget):
    def __init__(self, parent=None):
        super().__init__(parent)
        self.s = kitty.state(None)
        self.phase = 1.0
        self._a = QtCore.QVariantAnimation(self)
        self._a.setStartValue(0.0)
        self._a.setEndValue(1.0)
        self._a.setDuration(700)
        self._a.valueChanged.connect(lambda v: (setattr(self, "phase", float(v)), self.update()))

    def set_state(self, s):
        self.s = s
        self.update()

    def play(self):
        if self.s["trail"]:
            self._a.start()

    def paintEvent(self, _):
        r = QtCore.QRectF(self.rect())
        tw, th = r.width(), r.width() * 9 / 16
        if th > r.height():
            th, tw = r.height(), r.height() * 16 / 9
        p = QtGui.QPainter(self)
        paint_terminal(p, QtCore.QRectF(r.left() + (r.width() - tw) / 2, r.top(), tw, th), self.s, self.phase)
        p.end()


def render_card(s: dict) -> QtGui.QPixmap:
    pm = QtGui.QPixmap(196 * 2, 110 * 2)
    pm.setDevicePixelRatio(2)
    pm.fill(Qt.GlobalColor.transparent)
    p = QtGui.QPainter(pm)
    paint_terminal(p, QtCore.QRectF(0, 0, 196, 110), s)
    p.end()
    return pm


class KittyEditor(UserPresetsMixin, QtWidgets.QWidget):
    """Eigenständiger Editor mit derselben Schnittstelle wie die Seiten (state/set/changed) für die Karten."""
    SUBS = ("presets", "look", "font", "cursor", "tabs", "colors", "behavior", "start")
    KIND = "kitty"
    CFG = "kitty"               # Schlüssel im Config (Alacritty nutzt denselben Editor mit „alacritty_k“)
    PROGRAM = "kitty"

    def __init__(self, win):
        super().__init__()
        self.win = win
        self.state = self.make_state(win.cfg.get(self.CFG))
        for k in ("kitty_preset", "kitty_applied_preset"):     # „mine“ heißt jetzt „01“
            if win.cfg.get(k) == "mine":
                win.cfg[k] = "01"
        self.cards, self.switches, self.segs, self.presets = [], [], [], {}
        lay = QtWidgets.QVBoxLayout(self)
        lay.setContentsMargins(0, 0, 0, 0)
        lay.setSpacing(12)
        top = QtWidgets.QHBoxLayout()
        top.setSpacing(14)
        self.preview = TerminalPreview()
        self.preview.setFixedHeight(230)
        top.addWidget(self.preview, 1)
        col = QtWidgets.QVBoxLayout()
        col.setSpacing(8)
        self.applied = QtWidgets.QLabel()
        col.addWidget(self.applied, 0, Qt.AlignmentFlag.AlignLeft)
        col.addStretch()
        self.btn_play = QtWidgets.QPushButton()
        self.btn_play.clicked.connect(self.preview.play)
        self.btn_undo = QtWidgets.QPushButton()
        self.btn_undo.clicked.connect(self.undo)
        self.btn_apply = QtWidgets.QPushButton()
        self.btn_apply.setObjectName("Primary")
        self.btn_apply.clicked.connect(self.apply)
        for b in (self.btn_play, self.btn_undo, self.btn_apply):
            b.setFixedSize(170, 40)
            b.setCursor(Qt.CursorShape.PointingHandCursor)
            col.addWidget(b)
        top.addLayout(col)
        lay.addLayout(top)
        self.sub = Segmented(self.SUBS, prefix="ks.")
        self.sub.changed.connect(self.show_sub)
        lay.addWidget(self.sub, 0, Qt.AlignmentFlag.AlignLeft)
        self.stack = QtWidgets.QStackedWidget()
        lay.addWidget(self.stack, 1)
        self.pages = {}
        for key in self.SUBS:
            sa = QtWidgets.QScrollArea()
            sa.setWidgetResizable(True)
            sa.setHorizontalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
            inner = QtWidgets.QWidget()
            inner.setObjectName("ScrollInner")
            il = QtWidgets.QVBoxLayout(inner)
            il.setContentsMargins(0, 0, 6, 0)
            il.setSpacing(12)
            self.build(key, il)
            self.build_extra(key, il)
            il.addStretch()
            sa.setWidget(inner)
            self.stack.addWidget(sa)
            self.pages[key] = sa
        self.refresh_controls()
        self.preview.set_state(self.view(self.state))
        self.show_sub(win.cfg.get(self.CFG + "_sub", "presets"))

    # ----- Programm (Alacritty überschreibt diese vier) -----
    def make_state(self, saved):
        return kitty.state(saved)

    def write(self):
        return kitty.apply(self.state, model.config_home(), self.win.log)

    def restore(self):
        return kitty.undo(model.config_home(), self.win.log)

    def view(self, s):
        """State as the preview shows it (Alacritty: no tabs, no cursor trail, own title)."""
        return s

    def build_extra(self, key, lay):
        """Zusätzliche Karten eines anderen Programms pro Unterseite."""

    # ----- Aufbau -----
    def card(self, layout, title_key) -> Card:
        c = Card(title_key, self)
        layout.addWidget(c)
        self.cards.append(c)
        return c

    def build(self, key, lay):
        if key == "presets":
            self.build_user_presets(lay)
            grid = FlowGrid(212)
            for n in kitty.PRESETS:
                if n == "mine":          # alter Name von „01“
                    continue
                pc = PresetCard(n)
                pc.setProperty("tkey", "kp." + n)
                pc.clicked.connect(self.use_preset)
                pc.set_pixmap(render_card(self.view(kitty.preset(n, self.state))))
                grid.add(pc)
                self.presets[n] = pc
            lay.addWidget(grid)
        elif key == "look":
            c = self.card(lay, "kc.trans")
            c.slider("opacity", 0.2, 1.0, 0.01, "{:.2f}", label_key="k.opacity")
            c.toggle("dynamic_opacity", tip="tip.k.dynamic", label_key="k.dynamic")
            c.slider("blur", 0, 64, 1, label_key="k.blur", tip="tip.k.blur")
            c = self.card(lay, "kc.window")
            c.slider("padding", 0, 40, label_key="k.padding")
            c.choice("decorations", ("no", "titlebar-only", "yes"), "kdec.", label_key="k.decorations")
            c.choice("display_server", ("auto", "wayland", "x11"), "kds.", label_key="k.display", tip="tip.k.display")
        elif key == "font":
            c = self.card(lay, "card.font")
            c.font("font", label_key="font")
            c.slider("font_size", 6, 24, 0.5, "{:.1f}", label_key="font_size")
            c.choice("ligatures", ("never", "cursor", "always"), "klig.", label_key="k.ligatures")
        elif key == "cursor":
            c = self.card(lay, "kc.cursor")
            c.choice("cursor_shape", ("block", "beam", "underline"), "kcur.", label_key="k.cursor_shape")
            c.slider("blink", 0.0, 2.0, 0.1, "{:.1f} s", label_key="k.blink", tip="tip.k.blink")
            c.choice("blink_curve", ("linear", "ease-in-out", "ease-in", "ease-out", "step-end"), "kcurve.",
                     label_key="k.blink_curve")
            c = self.card(lay, "kc.anim")
            c.slider("trail", 0, 30, 1, "{:.0f}", label_key="k.trail", tip="tip.k.trail")
            c.slider("trail_decay_fast", 0.05, 1.0, 0.05, "{:.2f}", label_key="k.decay_fast")
            c.slider("trail_decay_slow", 0.05, 2.0, 0.05, "{:.2f}", label_key="k.decay_slow")
            c.toggle("bell_flash", label_key="k.bell_flash")
        elif key == "tabs":
            c = self.card(lay, "kc.tabs")
            c.choice("tab_edge", ("top", "bottom"), "pos.", label_key="position")
            c.choice("tab_style", ("powerline", "slant", "fade", "separator", "hidden"), "ktab.", label_key="k.tab_style")
            c.choice("tab_powerline", ("slanted", "angled", "round"), "kpl.", label_key="k.tab_powerline")
            c.text("tab_title", width=220, label_key="k.tab_title")
        elif key == "colors":
            c = self.card(lay, "card.colors")
            for k_ in ("bg", "fg", "cursor", "sel_bg", "sel_fg", "url", "tab_active_bg", "tab_active_fg",
                       "tab_inactive_bg", "tab_inactive_fg"):
                c.color(k_, label_key="k." + k_)
            pc = self.card(lay, "kc.palette")
            grid = QtWidgets.QGridLayout()
            grid.setSpacing(8)
            self.pal_btns = []
            for i in range(16):
                b = ColorButton()
                b.setFixedSize(52, 30)
                b.color_changed.connect(lambda col, i=i: self._pal(i, col))
                grid.addWidget(b, i // 8, i % 8)
                self.pal_btns.append(b)
            pc.layout().addLayout(grid)
            pc.rows.append((QtWidgets.QLabel(), lambda s: [b.set_color(s["palette"][i]) for i, b in enumerate(self.pal_btns)]))
            self.btn_from = QtWidgets.QPushButton()
            self.btn_from.setCursor(Qt.CursorShape.PointingHandCursor)
            self.btn_from.clicked.connect(self.from_hypr)
            c.layout().addWidget(self.btn_from, 0, Qt.AlignmentFlag.AlignLeft)
        elif key == "behavior":
            c = self.card(lay, "kc.behavior")
            c.slider("scrollback", 1000, 100000, 1000, "{:.0f}", label_key="k.scrollback")
            c.choice("url_style", ("curly", "straight", "double", "dotted", "dashed", "none"), "kurl.", label_key="k.url_style")
            c.toggle("copy_on_select", label_key="k.copy")
            c.toggle("audio_bell", label_key="k.audio_bell")
            c.choice("confirm_close", ("0", "1", "-1"), "kconf.", label_key="k.confirm")
        elif key == "start":
            c = self.card(lay, "kc.start")
            c.toggle("fastfetch", tip="tip.k.fastfetch", label_key="k.fastfetch")
            c.lines("start_cmds", tip="tip.k.start_cmds")

    # ----- Zustand -----
    def set(self, key, value):
        if self.state.get(key) == value:
            return
        self.state[key] = value
        if key in kitty.LOOK:
            self.win.cfg[self.CFG + "_preset"] = ""
        self.changed()
        if key in ("trail", "trail_decay_fast", "trail_decay_slow"):
            self.preview.play()

    def _pal(self, i, col):
        pal = list(self.state["palette"])
        pal[i] = col[:7]
        self.set("palette", pal)

    def changed(self):
        self.win.cfg[self.CFG] = self.state
        self.win.save_later()
        self.preview.set_state(self.view(self.state))
        self.update_applied()

    def refresh_controls(self):
        for c in self.cards:
            c.refresh(self.state)

    def sanitize(self, d):
        return self.make_state(d)

    def state_pixmap(self, s):
        return render_card(self.view(s))

    def use_preset(self, name):
        self.state = self.user_state(name) if name.startswith(PREFIX) else kitty.preset(name, self.state)
        self.win.cfg[self.CFG + "_preset"] = name
        self.refresh_controls()
        self.changed()
        self.preview.play()

    def from_hypr(self):
        a = model.norm(self.win.hypr_page.state["active1"])[:7]
        self.state.update({"sel_bg": a, "tab_active_bg": a, "cursor": a})
        self.win.cfg[self.CFG + "_preset"] = ""
        self.refresh_controls()
        self.changed()

    def show_sub(self, key):
        if key not in self.pages:
            key = "presets"
        self.sub.select(key)
        self.stack.setCurrentWidget(self.pages[key])
        self.win.cfg[self.CFG + "_sub"] = key
        self.win.save_later()

    def _sig(self):
        return json.dumps(self.state, sort_keys=True)

    def update_applied(self):
        if not hasattr(self, "applied"):
            return
        ok = self.win.cfg.get(self.CFG + "_applied_sig") == self._sig()
        set_chip(self.applied, ok, t("applied") if ok else t("not_applied"))
        applied = self.win.cfg.get(self.CFG + "_applied_preset", "") if ok else ""
        sel = self.win.cfg.get(self.CFG + "_preset", "")
        for n, pc in self.presets.items():
            pc.refresh(n == sel, self.win.accent(), n == applied)

    def apply(self):
        self.win.log("")
        if not model.which(self.PROGRAM):
            self.win.log(t("k.missing"))
        ok, info = self.write()
        self.win.report(ok, info)
        if ok:
            self.win.cfg[self.CFG + "_applied_sig"] = self._sig()
            self.win.cfg[self.CFG + "_applied_preset"] = self.win.cfg.get(self.CFG + "_preset", "")
            self.win.save_later()
            pre = self.presets.get(self.win.cfg[self.CFG + "_applied_preset"])
            self.win.set_status(t("applied") + (f": {pre.name.text()}" if pre else "") + " · " + t("k.reload"))
            self.update_applied()

    def undo(self):
        self.win.log("")
        ok = self.restore()
        self.win.set_status(t("undone") if ok else t("nothing"))
        if ok:
            self.win.cfg[self.CFG + "_applied_sig"] = ""
            self.update_applied()

    def reset_state(self):
        self.state = self.make_state(None)
        for k in ("preset", "applied_sig", "applied_preset"):
            self.win.cfg[f"{self.CFG}_{k}"] = ""
        self.refresh_controls()
        self.changed()

    def retranslate(self):
        self.sub.retranslate()
        for c in self.cards:
            c.retranslate()
        for s in self.segs:
            s.retranslate()
        for pc in self.presets.values():
            if pc.property("tkey"):
                pc.name.setText(t(pc.property("tkey")))
        self.retranslate_user_presets()
        self.btn_play.setText(t("play"))
        self.btn_undo.setText(t("undo"))
        self.btn_apply.setText(t("apply"))
        if hasattr(self, "btn_from"):
            self.btn_from.setText(t("from_hypr"))
        self.update_applied()

    def apply_theme(self, accent):
        for sw in self.switches:
            sw.accent = accent
            sw.update()
        self.update_applied()
