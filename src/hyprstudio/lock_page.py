"""Hyprland › Sperrbildschirm (Hyprlock): Presets, Live-Vorschau, Hintergrund, Uhr & Text, Passwortfeld, Verhalten."""

import datetime
import os
import subprocess
from pathlib import Path

from . import dialogs, hyprlock, model
from .i18n import t
from .paper_page import _action, _btn, _load, _scroll, _short
from .qt import QtCore, QtGui, QtWidgets, Qt
from .widgets import Card, FlowGrid, PresetCard


def _qcolor(c: str) -> QtGui.QColor:
    h = model.norm(c)[1:]
    r, g, b, a = (int(h[i:i + 2], 16) for i in (0, 2, 4, 6))
    return QtGui.QColor(r, g, b, a)


_cache: dict = {}


_decoded_cache: dict = {}


def _decoded(path: str, w: int, h: int) -> QtGui.QImage:
    """Bild nur einmal je Größe von der Platte lesen (alle Presets nutzen dasselbe Hintergrundbild)."""
    key = (path, w, h)
    if key not in _decoded_cache:
        if len(_decoded_cache) > 8:
            _decoded_cache.clear()
        _decoded_cache[key] = _load(path, w, h) if path else QtGui.QImage()
    return _decoded_cache[key]


def _background(path: str, w: int, h: int, blur: int) -> QtGui.QImage:
    """Bild füllend zuschneiden und grob weichzeichnen (kleiner rechnen und wieder hochziehen)."""
    key = (path, w, h, blur)
    if key in _cache:
        return _cache[key]
    img = _decoded(path, w, h)
    if img.isNull():
        return img
    scale = max(w / img.width(), h / img.height())
    img = img.scaled(int(img.width() * scale) + 1, int(img.height() * scale) + 1,
                     Qt.AspectRatioMode.IgnoreAspectRatio, Qt.TransformationMode.SmoothTransformation)
    img = img.copy((img.width() - w) // 2, (img.height() - h) // 2, w, h)
    if blur > 0:
        f = max(1.0, blur * w / 1920 * 1.6)
        small = img.scaled(max(1, int(w / f)), max(1, int(h / f)), Qt.AspectRatioMode.IgnoreAspectRatio,
                           Qt.TransformationMode.SmoothTransformation)
        img = small.scaled(w, h, Qt.AspectRatioMode.IgnoreAspectRatio, Qt.TransformationMode.SmoothTransformation)
    if len(_cache) > 40:
        _cache.clear()
    _cache[key] = img
    return img


def render_lock(s: dict, w: int, h: int, wallpaper: str, radius: int = 12) -> QtGui.QPixmap:
    pm = QtGui.QPixmap(w, h)
    pm.fill(Qt.GlobalColor.transparent)
    p = QtGui.QPainter(pm)
    p.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
    p.setRenderHint(QtGui.QPainter.RenderHint.TextAntialiasing)
    clip = QtGui.QPainterPath()
    clip.addRoundedRect(QtCore.QRectF(0, 0, w, h), radius, radius)
    p.setClipPath(clip)
    k = h / 1080

    # Hintergrund (Bildschirmfoto wird mit dem Hintergrundbild angedeutet)
    path = wallpaper if s["bg"] in ("screenshot", "wallpaper") else (s["bg_path"] if s["bg"] == "image" else "")
    path = os.path.expanduser(hyprlock.first_image(path)) if path else ""
    p.fillRect(QtCore.QRectF(0, 0, w, h), _qcolor(s["bg_color"]))
    img = _background(path, w, h, s["blur_size"] * s["blur_passes"]) if path else QtGui.QImage()
    if img.isNull() and s["bg"] != "color":
        g = QtGui.QLinearGradient(0, 0, w, h)
        g.setColorAt(0, QtGui.QColor("#24304a"))
        g.setColorAt(1, QtGui.QColor("#3a2448"))
        p.fillRect(QtCore.QRectF(0, 0, w, h), g)
    elif not img.isNull():
        p.drawImage(0, 0, img)
    if s["bg"] != "color":
        b = s["brightness"]
        shade = QtGui.QColor(0, 0, 0, int(min(1.0, 1 - b) * 255)) if b < 1 else QtGui.QColor(255, 255, 255, int(min(1.0, b - 1) * 255))
        p.fillRect(QtCore.QRectF(0, 0, w, h), shade)

    W, H = w / k, 1080.0  # Bildschirm in 1080p-Koordinaten (wie in hyprlock.conf)
    pos, hs = hyprlock.place(s), hyprlock.heights(s)
    text = _qcolor(s["text_color"])

    def box(e: str, bw: float) -> tuple:
        """Rechteck in Vorschau-Pixeln – gleiche Rechnung wie hyprlock: Abstand von der Ankerkante."""
        x, y, ha, va = pos[e]
        bh = hs[e]
        left = x if ha == "left" else W + x - bw if ha == "right" else W / 2 + x - bw / 2
        top = H / 2 - y - bh / 2 if va == "center" else H - y - bh if va == "bottom" else -y
        return QtCore.QRectF(left * k, top * k, bw * k, bh * k), ha

    def label(e: str, txt: str, size: float, bold=False):
        f = QtGui.QFont(s["font"] or "Sans")
        f.setPixelSize(max(4, int(size * k * 1.33)))
        f.setBold(bold)
        p.setFont(f)
        p.setPen(text)
        r, ha = box(e, QtGui.QFontMetricsF(f).horizontalAdvance(txt) / k + 4)
        align = {"left": Qt.AlignmentFlag.AlignLeft, "right": Qt.AlignmentFlag.AlignRight}.get(
            ha, Qt.AlignmentFlag.AlignHCenter)
        p.drawText(r, align | Qt.AlignmentFlag.AlignVCenter, txt)

    now = datetime.datetime.now()
    if "clock" in pos:
        fmt = {"24": "%H:%M", "12": "%I:%M %p", "sec": "%H:%M:%S"}[s["clock_fmt"]]
        label("clock", now.strftime(fmt), s["clock_size"], True)
    if "date" in pos:
        label("date", now.strftime(hyprlock.DATE_FMT[s["date_fmt"]]), s["date_size"])
    if "avatar" in pos:
        r, _ = box("avatar", s["avatar_size"])
        d = r.width()
        av = QtGui.QImage(os.path.expanduser(s["avatar_path"])) if s["avatar_path"] else QtGui.QImage()
        circ = QtGui.QPainterPath()
        circ.addEllipse(r)
        p.save()
        p.setClipPath(circ, Qt.ClipOperation.IntersectClip)
        if av.isNull():
            p.fillRect(r, QtGui.QColor(255, 255, 255, 40))
            p.setPen(Qt.PenStyle.NoPen)
            p.setBrush(QtGui.QColor(255, 255, 255, 170))
            p.drawEllipse(QtCore.QPointF(r.center().x(), r.top() + d * 0.38), d * 0.18, d * 0.18)
            p.drawEllipse(QtCore.QPointF(r.center().x(), r.top() + d * 0.92), d * 0.34, d * 0.3)
        else:
            p.drawImage(r, av)
        p.restore()
        p.setBrush(Qt.BrushStyle.NoBrush)
        p.setPen(QtGui.QPen(_qcolor(s["accent"]), max(1.0, s["outline"] * k)))
        p.drawEllipse(r)
    if "greet" in pos:
        label("greet", s["greet_text"].replace("$USER", os.environ.get("USER") or os.environ.get("USERNAME") or "user"),
              16)

    # Passwortfeld
    r, _ = box("input", s["input_w"])
    ih = r.height()
    rad = ih / 2 if s["rounding"] < 0 else min(ih / 2, s["rounding"] * k)
    p.setBrush(_qcolor(s["inner_color"]))
    pen = QtGui.QPen(_qcolor(s["accent"]), s["outline"] * k) if s["outline"] else QtGui.QPen(Qt.PenStyle.NoPen)
    p.setPen(pen)
    p.drawRoundedRect(r, rad, rad)
    f = QtGui.QFont(s["font"] or "Sans")
    f.setPixelSize(max(4, int(ih * 0.36)))
    f.setItalic(True)
    p.setFont(f)
    ph = _qcolor(s["input_text"])
    ph.setAlpha(150)
    p.setPen(ph)
    p.drawText(r, Qt.AlignmentFlag.AlignCenter, s["placeholder"])
    p.end()
    return pm


class LockPreview(QtWidgets.QLabel):
    def __init__(self, parent=None):
        super().__init__(parent)
        self.setMinimumHeight(260)
        self.setSizePolicy(QtWidgets.QSizePolicy.Policy.Expanding, QtWidgets.QSizePolicy.Policy.Fixed)
        self.setFixedHeight(300)
        self.args = None

    def show_state(self, s, wallpaper):
        self.args = (s, wallpaper)
        self._draw()

    def _draw(self):
        if not self.args or self.width() < 50:
            return
        h = self.height()
        w = min(self.width(), int(h * 16 / 9))
        self.setPixmap(render_lock(self.args[0], w, h, self.args[1], 14))

    def resizeEvent(self, e):
        super().resizeEvent(e)
        self._draw()


class LockEditor(QtWidgets.QWidget):
    KIND = "lock"

    def __init__(self, win):
        super().__init__()
        self.win = win
        self.state = hyprlock.lock_state(win.cfg.get("lock"))
        self.cards, self.switches, self.segs = [], [], []
        self.presets = {}
        self.btn_test, self.btn_undo, self.btn_apply = _action(), _action(), _action(True)
        self.btn_test.clicked.connect(self.test)
        self.btn_undo.clicked.connect(self.undo)
        self.btn_apply.clicked.connect(self.apply)
        self.actions = [self.btn_test, self.btn_undo, self.btn_apply]
        lay = _scroll(self)

        self.preview = LockPreview()
        self.preview.setAlignment(Qt.AlignmentFlag.AlignCenter)
        lay.addWidget(self.preview)
        self._clock = QtCore.QTimer(self)
        self._clock.timeout.connect(self._tick)
        self._clock.start(1000)

        # Presets
        c = Card("lock.presets", self)
        self.cards.append(c)
        self.grid = FlowGrid(212)
        for name in hyprlock.PRESETS:
            pc = PresetCard(name)
            pc.clicked.connect(self.use_preset)
            self.grid.add(pc)
            self.presets[name] = pc
        c.layout().addWidget(self.grid)
        lay.addWidget(c)

        # Hintergrund
        c = Card("lock.layout", self)
        self.cards.append(c)
        c.choice("layout", hyprlock.LAYOUTS, "llay.", label_key="lock.layout")
        lay.addWidget(c)

        c = Card("lock.bg", self)
        self.cards.append(c)
        c.choice("bg", hyprlock.BGS, "lbg.", label_key="lock.bg_src")
        row = QtWidgets.QHBoxLayout()
        self.btn_img = _btn()
        self.btn_img.clicked.connect(self._choose_image)
        self.img_lbl = QtWidgets.QLabel()
        self.img_lbl.setObjectName("Mono")
        row.addWidget(self.btn_img)
        row.addWidget(self.img_lbl, 1)
        self.img_row = QtWidgets.QWidget()
        self.img_row.setLayout(row)
        row.setContentsMargins(0, 0, 0, 0)
        c._add("lock.image", self.img_row, label_key="lock.image")
        c.color("bg_color", label_key="lock.bg_color")
        c.slider("blur_passes", 0, 4, label_key="lock.blur_passes")
        c.slider("blur_size", 1, 15, label_key="lock.blur_size")
        c.slider("brightness", 0.2, 1.2, 0.05, "{:.2f}", label_key="lock.brightness")
        c.slider("contrast", 0.5, 1.5, 0.05, "{:.2f}", label_key="lock.contrast")
        c.slider("vibrancy", 0.0, 1.0, 0.05, "{:.2f}", label_key="lock.vibrancy")
        lay.addWidget(c)

        # Uhr & Text
        c = Card("lock.text", self)
        self.cards.append(c)
        c.toggle("clock", label_key="lock.clock")
        c.choice("clock_fmt", hyprlock.CLOCKS, "lclk.", label_key="lock.clock_fmt")
        c.slider("clock_size", 30, 200, label_key="lock.clock_size")
        c.toggle("date", label_key="lock.date")
        c.choice("date_fmt", hyprlock.DATES, "ldate.", label_key="lock.date_fmt")
        c.slider("date_size", 10, 40, label_key="lock.date_size")
        c.toggle("greet", label_key="lock.greet")
        c.text("greet_text", tip="tip.lock.greet", width=260, label_key="lock.greet_text")
        c.toggle("avatar", label_key="lock.avatar")
        c.text("avatar_path", width=260, label_key="lock.avatar_path")
        c.slider("avatar_size", 60, 220, label_key="lock.avatar_size")
        c.font("font", label_key="font")
        c.color("text_color", label_key="lock.text_color")
        lay.addWidget(c)

        # Passwortfeld
        c = Card("lock.input", self)
        self.cards.append(c)
        c.slider("input_w", 150, 700, 10, label_key="lock.input_w")
        c.slider("input_h", 30, 100, label_key="lock.input_h")
        c.slider("rounding", -1, 50, tip="tip.lock.rounding", label_key="lock.rounding")
        c.slider("outline", 0, 8, label_key="lock.outline")
        c.slider("input_y", -450, 150, 10, label_key="lock.input_y")
        c.text("placeholder", width=260, label_key="lock.placeholder")
        c.color("accent", label_key="lock.accent")
        c.color("inner_color", label_key="lock.inner")
        c.color("input_text", label_key="lock.input_text")
        c.color("check_color", label_key="lock.check")
        c.color("fail_color", label_key="lock.fail")
        c.toggle("fade_empty", label_key="lock.fade_empty")
        c.toggle("dots_center", label_key="lock.dots_center")
        lay.addWidget(c)

        # Verhalten
        c = Card("lock.behavior", self)
        self.cards.append(c)
        c.toggle("lock_bind", tip="tip.lock.bind", label_key="lock.bind")
        c.toggle("hide_cursor", label_key="lock.hide_cursor")
        c.toggle("ignore_empty", label_key="lock.ignore_empty")
        c.slider("grace", 0, 30, fmt="{:.0f} s", tip="tip.lock.grace", label_key="lock.grace")
        lay.addWidget(c)
        lay.addStretch()
        self.refresh_controls()

    # ----- Zustand -----
    def wallpaper(self) -> str:
        ps = self.win.other_page.paper.state if hasattr(self.win, "other_page") else {}
        path = ps.get("path") or next((m["path"] for m in ps.get("monitors", {}).values()), "")
        return hyprlock.first_image(path)

    def set(self, key, value):
        if self.state.get(key) == value:
            return
        self.state[key] = value
        self.changed()

    def changed(self):
        self.win.cfg["lock"] = self.state
        self.win.save_later()
        self.img_row.setEnabled(self.state["bg"] == "image")
        self.img_lbl.setText(_short(self.state["bg_path"]) or "–")
        self.preview.show_state(self.state, self.wallpaper())
        self._mark()

    def _tick(self):
        if self.isVisible() and self.state["clock"]:
            self.preview.show_state(self.state, self.wallpaper())

    def refresh_controls(self):
        if hasattr(self.win, "hypr_page"):
            self.state["lock_bind"] = hyprlock.has_lock_bind(self.win.hypr_page.state["binds"])
        for c in self.cards:
            c.refresh(self.state)
        self.changed()

    def refresh(self):
        """Beim Öffnen: Kürzel-Stand übernehmen, Preset-Bilder mit aktuellem Hintergrundbild zeichnen."""
        self.refresh_controls()
        wp = self.wallpaper()
        sig = (wp, self.state["font"])  # Preset-Bilder hängen nur vom Hintergrund und der Schrift ab
        if getattr(self, "_thumb_sig", None) == sig:
            return
        self._thumb_sig = sig
        for name, pc in self.presets.items():
            pc.set_pixmap(render_lock(hyprlock.preset_state(name, self.state), 196, 110, wp, 10))

    def use_preset(self, name):
        self.state = hyprlock.preset_state(name, self.state)
        self.refresh_controls()
        self.win.set_status(t("lock.preset", name=t("lp." + name)))

    def _choose_image(self):
        start = os.path.dirname(os.path.expanduser(self.state["bg_path"])) if self.state["bg_path"] else str(Path.home())
        f = dialogs.open_file(self, t("lock.image"), start, "Bilder (*.png *.jpg *.jpeg *.webp *.jxl *.bmp)")
        if f:
            self.state["bg"] = "image"
            self.set("bg_path", f)
            self.refresh_controls()

    # ----- Aktionen -----
    def apply(self):
        self.win.log("")
        hdir = model.config_home() / "hypr"
        ok, info = hyprlock.apply_lock(self.state, hdir, self.win.log, self.wallpaper())
        if ok:
            hp = self.win.hypr_page
            if hyprlock.has_lock_bind(hp.state["binds"]) != self.state["lock_bind"]:
                hp.state["binds"] = hyprlock.set_lock_bind(hp.state["binds"], self.state["lock_bind"])
                hp.refresh_controls()
                hp.changed()
                hp.apply()
        self.win.set_status(t("lock.done") if ok else t(info))

    def undo(self):
        ok = model.undo_file(hyprlock.lock_file(model.config_home() / "hypr"))
        self.win.set_status(t("undone") if ok else t("nothing"))

    def test(self):
        """Jetzt sperren (mit der angewendeten Datei) – entsperren mit dem eigenen Passwort."""
        if model.DRY:
            self.win.log("$ hyprlock  (Testmodus)")
            return
        try:
            subprocess.Popen(["hyprlock"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                             start_new_session=True, cwd=str(Path.home()))
            self.win.log("$ hyprlock")
        except OSError as e:
            self.win.log(f"hyprlock: {e}")

    def retranslate(self):
        for c in self.cards:
            c.retranslate()
        for sg in self.segs:
            sg.retranslate()
        for name, pc in self.presets.items():
            pc.name.setText(t("lp." + name))
        self.btn_img.setText(t("paper.image"))
        self.btn_test.setText(t("lock.test"))
        self.btn_undo.setText(t("undo"))
        self.btn_apply.setText(t("apply"))
        self.btn_test.setToolTip(t("tip.lock.test"))
        self.changed()

    def apply_theme(self, accent):
        self.accent = accent
        for sw in self.switches:
            sw.accent = accent
            sw.update()
        for name, pc in self.presets.items():
            sel = all(self.state[k] == v for k, v in hyprlock.preset_state(name, self.state).items()
                      if k in hyprlock.LOOK_KEYS)
            pc.refresh(sel, accent)

    def _mark(self):
        """Preset-Auswahl mit der Akzentfarbe markieren."""
        acc = getattr(self, "accent", "#3b82f6")
        for name, pc in self.presets.items():
            sel = all(self.state[k] == v for k, v in hyprlock.preset_state(name, self.state).items()
                      if k in hyprlock.LOOK_KEYS)
            pc.refresh(sel, acc)
