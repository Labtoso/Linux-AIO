"""Live-Vorschau: gezeichneter Hyprland-Desktop mit Waybar, Lücken, Rahmen, Unschärfe, Schatten und Animation."""

import math

from . import model
from .qt import QtCore, QtGui, QtWidgets, Qt

VW, VH = 1280, 720  # virtueller Bildschirm (etwas kleiner als echt, damit Lücken sichtbar bleiben)
_wall_cache: dict = {}


def qcolor(c: str, alpha_mul: float = 1.0) -> QtGui.QColor:
    h = model.norm(c)[1:]
    r, g, b, a = (int(h[i:i + 2], 16) for i in (0, 2, 4, 6))
    return QtGui.QColor(r, g, b, max(0, min(255, int(a * alpha_mul))))


def bezier_y(p1, p2, x: float) -> float:
    """Kubische Bezierkurve (0,0)-p1-p2-(1,1): y zu gegebenem x (Bisektion)."""
    def bx(t):
        return 3 * (1 - t) ** 2 * t * p1[0] + 3 * (1 - t) * t * t * p2[0] + t ** 3

    def by(t):
        return 3 * (1 - t) ** 2 * t * p1[1] + 3 * (1 - t) * t * t * p2[1] + t ** 3
    lo, hi = 0.0, 1.0
    for _ in range(30):
        mid = (lo + hi) / 2
        if bx(mid) < x:
            lo = mid
        else:
            hi = mid
    return by((lo + hi) / 2)


def _wallpaper(w: int, h: int) -> tuple:
    key = (w, h)
    if key in _wall_cache:
        return _wall_cache[key]
    img = QtGui.QImage(w, h, QtGui.QImage.Format.Format_ARGB32_Premultiplied)
    img.fill(QtGui.QColor("#24345e"))
    p = QtGui.QPainter(img)
    p.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
    p.setPen(Qt.PenStyle.NoPen)
    for i, col in enumerate(("#2b3f70", "#344a80", "#40558c", "#4d5f94")):
        p.setBrush(QtGui.QColor(col))
        p.drawRect(QtCore.QRectF(0, h * (0.12 + i * 0.1), w, h))
    p.setBrush(QtGui.QColor(240, 236, 220, 240))
    p.drawEllipse(QtCore.QPointF(w * 0.78, h * 0.24), h * 0.08, h * 0.08)
    for i, (col, base, amp, freq) in enumerate((("#5a5d9e", 0.55, 0.10, 2.2), ("#47508c", 0.66, 0.08, 3.1),
                                                 ("#333d70", 0.78, 0.07, 4.3), ("#1f2849", 0.9, 0.05, 5.7))):
        path = QtGui.QPainterPath(QtCore.QPointF(0, h))
        for x in range(0, w + 8, 8):
            y = h * (base - amp * math.sin(x / w * freq * math.pi + i * 1.7) * (0.6 + 0.4 * math.cos(x / w * 7 + i)))
            path.lineTo(x, y)
        path.lineTo(w, h)
        p.setBrush(QtGui.QColor(col))
        p.drawPath(path)
    p.setBrush(QtGui.QColor(255, 255, 255, 140))
    for i in range(40):
        x, y = (i * 97 % w), (i * 53 % int(h * 0.4))
        p.drawEllipse(QtCore.QPointF(x, y), 0.9, 0.9)
    p.end()
    pm = QtGui.QPixmap.fromImage(img)
    small = img.scaled(max(1, w // 14), max(1, h // 14), Qt.AspectRatioMode.IgnoreAspectRatio,
                       Qt.TransformationMode.SmoothTransformation)
    blur = QtGui.QPixmap.fromImage(small.scaled(w, h, Qt.AspectRatioMode.IgnoreAspectRatio,
                                                Qt.TransformationMode.SmoothTransformation))
    if len(_wall_cache) > 12:
        _wall_cache.clear()
    _wall_cache[key] = (pm, blur)
    return pm, blur


MOD_TEXT = {
    "custom/launcher": "", "hyprland/window": "kitty", "clock": "14:32", "pulseaudio": "60%",
    "network": "WLAN", "battery": "87%", "backlight": "70%", "bluetooth": "BT", "cpu": "12%",
    "memory": "41%", "temperature": "48°", "tray": "", "disk": "37%", "mpris": "Artist – Song",
    "hyprland/language": "DE", "idle_inhibitor": "", "power-profiles-daemon": "", "custom/power": "",
}


def mod_text(mod: str, ws: dict) -> str:
    if mod == "clock":
        h = "14:32" if ws["clock24"] else "2:32 PM"
        return {"hm": h, "hms": h.replace("32", "32:05", 1), "dhm": "Mo " + h, "full": "18.10.2026  " + h}[ws["clock_fmt"]]
    if mod == "network":
        return {"essid": "WLAN", "ip": "192.168.1.20", "signal": "78%"}[ws["net_fmt"]]
    return MOD_TEXT.get(mod, "")


def _bar(p: QtGui.QPainter, r: QtCore.QRectF, k: float, ws: dict) -> float:
    """Zeichnet die Waybar, gibt die belegte Höhe (virtuell) zurück."""
    h = ws["height"]
    m = ws["margin"] if ws["style"] != "bar" else 0
    top = ws["position"] == "top"
    y = r.top() + m * k if top else r.bottom() - (m + h) * k
    bar = QtCore.QRectF(r.left() + (m * 2) * k, y, r.width() - (m * 4) * k, h * k)
    rad = ws["radius"] * k
    bg, fg, acc = qcolor(ws["bg"]), qcolor(ws["fg"]), qcolor(ws["accent"])
    pen = QtGui.QPen(qcolor(ws["border"]), max(1.0, ws["border_width"] * k)) if ws["border_width"] else Qt.PenStyle.NoPen
    if ws["style"] != "islands":
        p.setPen(pen)
        p.setBrush(bg)
        p.drawRoundedRect(bar, rad if ws["style"] == "float" else 0, rad if ws["style"] == "float" else 0)
    f = p.font()
    f.setPixelSize(max(3, int(round(ws["font_size"] * k))))
    f.setWeight(QtGui.QFont.Weight.DemiBold)
    p.setFont(f)
    fm = QtGui.QFontMetricsF(f)
    pad = (ws["pad"] + (2 if ws["style"] == "islands" else 0)) * k
    f.setWeight(QtGui.QFont.Weight.Bold if ws["font_weight"] == "bold" else
                QtGui.QFont.Weight.DemiBold if ws["font_weight"] == "600" else QtGui.QFont.Weight.Normal)
    p.setFont(f)
    fm = QtGui.QFontMetricsF(f)
    n_ws = max(4, ws["ws_persist"])
    gap = (6 if ws["style"] == "islands" else ws["spacing"]) * k

    def width(mod):
        if mod == "hyprland/workspaces":
            return n_ws * (fm.horizontalAdvance("4") + 14 * k)
        if mod in ("custom/launcher", "custom/power", "idle_inhibitor", "power-profiles-daemon"):
            return fm.height() * 0.9
        if mod == "tray":
            return 2 * fm.height() * 0.5 + 6 * k
        dot = fm.height() * 0.45 + 5 * k if ws["icons"] else 0
        return dot + fm.horizontalAdvance(mod_text(mod, ws))

    def draw(mod, x):
        wdt = width(mod)
        full = wdt + 2 * pad if ws["style"] == "islands" else wdt + pad
        if ws["style"] == "islands":
            p.setPen(pen)
            p.setBrush(bg)
            p.drawRoundedRect(QtCore.QRectF(x, bar.top(), full, bar.height()), rad, rad)
            cx = x + pad
        else:
            cx = x + pad / 2
        cy = bar.center().y()
        p.setPen(Qt.PenStyle.NoPen)
        if mod == "hyprland/workspaces":
            cw = fm.horizontalAdvance("4") + 14 * k
            look = ws["ws_look"]
            for i in range(n_ws):
                rr = QtCore.QRectF(cx + i * cw, cy - fm.height() * 0.6, cw, fm.height() * 1.2)
                c = QtGui.QColor(fg)
                if i == 0:
                    c = QtGui.QColor("#0e131a") if look == "box" else QtGui.QColor(fg if look == "underline" else acc)
                    if look in ("pill", "box"):
                        a = QtGui.QColor(acc)
                        a.setAlpha(46 if look == "pill" else 255)
                        p.setBrush(a)
                        p.drawRoundedRect(rr, max(2, rad / 2), max(2, rad / 2))
                    elif look == "underline":
                        p.setBrush(acc)
                        p.drawRect(QtCore.QRectF(rr.left(), bar.bottom() - 3 * k, rr.width(), 3 * k))
                else:
                    c.setAlpha(140 if i < 3 else 80)
                if ws["ws_format"] == "dot" and ws["icons"]:
                    p.setBrush(c)
                    d = fm.height() * (0.42 if i == 0 else 0.3)
                    p.drawEllipse(rr.center(), d / 2, d / 2)
                else:
                    p.setPen(c)
                    p.drawText(rr, Qt.AlignmentFlag.AlignCenter, str(i + 1))
                p.setPen(Qt.PenStyle.NoPen)
        elif mod == "custom/launcher":
            p.setBrush(acc)
            d = fm.height() * 0.62
            tri = QtGui.QPolygonF([QtCore.QPointF(cx + wdt / 2, cy - d / 2), QtCore.QPointF(cx + wdt / 2 + d / 2, cy + d / 2),
                                   QtCore.QPointF(cx + wdt / 2 - d / 2, cy + d / 2)])
            p.drawPolygon(tri)
        elif mod in ("custom/power", "idle_inhibitor", "power-profiles-daemon"):
            col = QtGui.QColor("#e5534b") if mod == "custom/power" else qcolor(model.module_color(mod, ws))
            p.setPen(QtGui.QPen(col, max(1.0, 1.6 * k * 2)))
            p.setBrush(Qt.BrushStyle.NoBrush)
            d = fm.height() * 0.55
            p.drawEllipse(QtCore.QPointF(cx + wdt / 2, cy), d / 2, d / 2)
            p.setPen(Qt.PenStyle.NoPen)
        elif mod == "tray":
            p.setBrush(QtGui.QColor(fg.red(), fg.green(), fg.blue(), 150))
            d = fm.height() * 0.5
            for i in range(2):
                p.drawRoundedRect(QtCore.QRectF(cx + i * (d + 6 * k), cy - d / 2, d, d), 2, 2)
        else:
            tx = cx
            mc = qcolor(model.module_color(mod, ws)) if ws["colorful"] else acc
            if ws["icons"]:
                d = fm.height() * 0.45
                p.setBrush(mc)
                p.drawEllipse(QtCore.QRectF(cx, cy - d / 2, d, d))
                tx += d + 5 * k
            p.setPen(mc if ws["colorful"] else fg)
            p.drawText(QtCore.QRectF(tx, bar.top(), wdt, bar.height()), Qt.AlignmentFlag.AlignVCenter, mod_text(mod, ws))
        return full

    def group_w(mods):
        if not mods:
            return 0
        each = [width(m) + (2 * pad if ws["style"] == "islands" else pad) for m in mods]
        return sum(each) + gap * (len(mods) - 1)

    x = bar.left() + (0 if ws["style"] == "islands" else 6 * k)
    for mod in ws["left"]:
        x += draw(mod, x) + gap
    x = bar.center().x() - group_w(ws["center"]) / 2
    for mod in ws["center"]:
        x += draw(mod, x) + gap
    x = bar.right() - group_w(ws["right"]) - (0 if ws["style"] == "islands" else 6 * k)
    for mod in ws["right"]:
        x += draw(mod, x) + gap
    return m + h


def _window_content(p, rect, hs, k, active):
    pad = 16 * k
    lh = max(1.5, 9 * k)
    cols = [qcolor(hs["active1"]), qcolor(hs["active2"] if hs["gradient"] else hs["active1"]), QtGui.QColor("#8a98ab")]
    p.setPen(Qt.PenStyle.NoPen)
    y = rect.top() + pad
    i = 0
    while y + lh < rect.bottom() - pad and i < 9:
        c = QtGui.QColor(cols[i % 3])
        c.setAlpha(200 if active else 120)
        p.setBrush(c)
        w1 = rect.width() * (0.12 + 0.05 * (i % 3))
        p.drawRoundedRect(QtCore.QRectF(rect.left() + pad, y, w1, lh), lh / 2, lh / 2)
        g = QtGui.QColor("#c8d3e2")
        g.setAlpha(110 if active else 70)
        p.setBrush(g)
        w2 = (rect.width() - 2 * pad - w1 - 6 * k) * (0.35 + 0.5 * ((i * 37) % 10) / 10)
        p.drawRoundedRect(QtCore.QRectF(rect.left() + pad + w1 + 6 * k, y, max(0, w2), lh), lh / 2, lh / 2)
        y += lh * 2.4
        i += 1


def paint_desktop(p: QtGui.QPainter, r: QtCore.QRectF, hs: dict, ws: dict | None, anim: float = 1.0,
                  radius: float = 12.0, bar_scale: float = 1.0):
    p.save()
    p.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
    p.setRenderHint(QtGui.QPainter.RenderHint.SmoothPixmapTransform)
    clip = QtGui.QPainterPath()
    clip.addRoundedRect(r, radius, radius)
    p.setClipPath(clip)
    wall, blur = _wallpaper(max(1, int(r.width())), max(1, int(r.height())))
    p.drawPixmap(r.topLeft(), wall)
    k = r.width() / VW

    reserved_top = reserved_bottom = 0.0
    if ws is not None:
        used = _bar(p, r, k * bar_scale, ws) * bar_scale  # bar_scale > 1: Leiste größer (Preset-Kärtchen)
        if ws["position"] == "top":
            reserved_top = used
        else:
            reserved_bottom = used

    go, gi = hs["gaps_out"], hs["gaps_in"]
    ax, ay = go, reserved_top + go
    aw, ah = VW - 2 * go, VH - reserved_top - reserved_bottom - 2 * go
    if hs["layout"] == "master":
        lw = (aw - 2 * gi) * 0.58
    else:
        lw = (aw - 2 * gi) / 2
    rh = (ah - 2 * gi) / 2
    wins = [
        (ax, ay, lw, ah),
        (ax + lw + 2 * gi, ay, aw - lw - 2 * gi, rh),
        (ax + lw + 2 * gi, ay + rh + 2 * gi, aw - lw - 2 * gi, rh),
    ]
    rad = hs["rounding"] * k
    bs = hs["border_size"] * k
    for idx, (x, y, w, h) in enumerate(wins):
        active = idx == 0
        rect = QtCore.QRectF(r.left() + x * k, r.top() + y * k, w * k, h * k)
        alpha = 1.0
        if idx == 2 and anim < 1.0:
            style = hs["win_style"]
            if style == "slide":
                rect.translate((1 - anim) * (r.right() - rect.left()), 0)
            elif style == "gnomed":
                s = max(0.01, anim)
                c = rect.center()
                rect = QtCore.QRectF(c.x() - rect.width() / 2, c.y() - rect.height() * s / 2, rect.width(),
                                     rect.height() * s)
            else:
                s = 0.8 + 0.2 * anim
                c = rect.center()
                rect = QtCore.QRectF(c.x() - rect.width() * s / 2, c.y() - rect.height() * s / 2,
                                     rect.width() * s, rect.height() * s)
                alpha = max(0.0, min(1.0, anim * 1.4))
            if anim <= 0:
                continue
        p.save()
        p.setOpacity(max(0.0, min(1.0, alpha)))
        path = QtGui.QPainterPath()
        path.addRoundedRect(rect, rad, rad)
        if hs["shadow"] and hs["shadow_range"] > 0:
            sc = qcolor(hs["shadow_color"])
            steps = max(1, min(14, hs["shadow_range"]))
            spread = hs["shadow_range"] * k
            p.setPen(Qt.PenStyle.NoPen)
            for s in range(steps):
                f = (s + 1) / steps
                c = QtGui.QColor(sc)
                c.setAlpha(int(sc.alpha() * (1 - f) ** (hs["shadow_power"] * 0.7) * 0.35))
                p.setBrush(c)
                g = spread * f
                p.drawRoundedRect(rect.adjusted(-g, -g, g, g), rad + g, rad + g)
        op = hs["active_opacity"] if active else hs["inactive_opacity"]
        p.save()
        p.setClipPath(path, Qt.ClipOperation.IntersectClip)
        if op < 1.0 and hs["blur"]:
            p.drawPixmap(r.topLeft(), blur)
        bgc = QtGui.QColor("#141a24")
        bgc.setAlphaF(op if (op < 1.0) else 1.0)
        p.fillRect(rect, bgc)
        _window_content(p, rect, hs, k, active)
        if hs["dim_inactive"] and not active:
            p.fillRect(rect, QtGui.QColor(0, 0, 0, int(160 * hs["dim_strength"])))
        p.restore()
        if bs > 0:
            pw = max(1.0, bs)
            br = rect.adjusted(pw / 2, pw / 2, -pw / 2, -pw / 2)
            if active and hs["gradient"]:
                a = math.radians(hs["angle"])
                c = br.center()
                d = max(br.width(), br.height()) / 2
                g = QtGui.QLinearGradient(c.x() - math.cos(a) * d, c.y() + math.sin(a) * d,
                                          c.x() + math.cos(a) * d, c.y() - math.sin(a) * d)
                g.setColorAt(0, qcolor(hs["active1"]))
                g.setColorAt(1, qcolor(hs["active2"]))
                pen = QtGui.QPen(QtGui.QBrush(g), pw)
            else:
                pen = QtGui.QPen(qcolor(hs["active1"] if active else hs["inactive"]), pw)
            p.setPen(pen)
            p.setBrush(Qt.BrushStyle.NoBrush)
            p.drawRoundedRect(br, max(0.0, rad - pw / 2), max(0.0, rad - pw / 2))
        p.restore()
    p.restore()


def _overlay(p, r, ls, radius):
    p.save()
    clip = QtGui.QPainterPath()
    clip.addRoundedRect(r, radius, radius)
    p.setClipPath(clip)
    p.fillRect(r, QtGui.QColor(0, 0, 0, 70))
    paint_launcher(p, r, ls)
    p.restore()


def render_pixmap(hs: dict, ws: dict | None, w: int, h: int, radius: float = 10.0, ls: dict | None = None,
                  bar_scale: float = 1.0) -> QtGui.QPixmap:
    dpr = 2
    pm = QtGui.QPixmap(w * dpr, h * dpr)
    pm.setDevicePixelRatio(dpr)
    pm.fill(Qt.GlobalColor.transparent)
    p = QtGui.QPainter(pm)
    r = QtCore.QRectF(0, 0, w, h)
    paint_desktop(p, r, hs, ws, radius=radius, bar_scale=bar_scale)
    if ls:
        _overlay(p, r, ls, radius)
    p.end()
    return pm


class DesktopPreview(QtWidgets.QWidget):
    def __init__(self, parent=None):
        super().__init__(parent)
        self.hs = model.hypr_state(None)
        self.ws = model.waybar_state(None)
        self.anim = 1.0
        self.zoom_bar = False
        self.ls = None  # Launcher-Zustand → Launcher über dem Desktop zeichnen
        self.setMinimumHeight(220)
        self.setSizePolicy(QtWidgets.QSizePolicy.Policy.Expanding, QtWidgets.QSizePolicy.Policy.Expanding)
        self._a = QtCore.QVariantAnimation(self)
        self._a.setStartValue(0.0)
        self._a.setEndValue(1.0)
        self._a.valueChanged.connect(self._tick)

    def set_states(self, hs: dict, ws: dict) -> None:
        self.hs, self.ws = hs, ws
        self.update()

    def play(self) -> None:
        if not self.hs["anim"]:
            return
        speed = dict((leaf, sp) for leaf, sp, _ in model.anim_list(self.hs))["windowsIn"]
        self._a.stop()
        self._a.setDuration(int(speed * 100 * 1.6))  # etwas langsamer, damit man es sieht
        self._a.start()

    def _tick(self, v):
        p1, p2 = model.CURVES[self.hs["anim_curve"]]
        self.anim = bezier_y(p1, p2, float(v)) if float(v) < 1 else 1.0
        self.update()

    def paintEvent(self, _):
        r = QtCore.QRectF(self.rect())
        p = QtGui.QPainter(self)
        if self.zoom_bar:
            # volle Breite, nur die Seite mit der Leiste sichtbar → Waybar größer
            th = r.width() * 9 / 16
            top = r.top() if self.ws["position"] == "top" else r.bottom() - th
            clip = QtGui.QPainterPath()
            clip.addRoundedRect(r, 14, 14)
            p.setClipPath(clip)
            paint_desktop(p, QtCore.QRectF(r.left(), top, r.width(), th), self.hs, self.ws, self.anim, radius=0)
        else:
            tw, th = r.width(), r.width() * 9 / 16
            if th > r.height():
                th = r.height()
                tw = th * 16 / 9
            rect = QtCore.QRectF(r.left() + (r.width() - tw) / 2, r.top() + (r.height() - th) / 2, tw, th)
            paint_desktop(p, rect, self.hs, self.ws, self.anim, radius=14)
            if self.ls:
                _overlay(p, rect, self.ls, 14)
        p.end()


APPS_DEMO = ("Firefox", "Kitty", "Dolphin", "Spotify", "Discord", "Steam", "VS Code", "GIMP", "Obsidian",
             "Blender", "OBS Studio", "Krita", "VLC", "Thunderbird", "Einstellungen", "Rechner", "Bilder", "Karten")


def paint_launcher(p: QtGui.QPainter, r: QtCore.QRectF, ls: dict):
    """Zeichnet das Launcher-Fenster (Rofi/Wofi/Fuzzel sehen mit diesen Einstellungen ähnlich aus)."""
    k = r.width() / VW
    grid = ls["grid"]
    cols = ls["columns"] if grid else 1
    f = p.font()
    f.setPixelSize(max(4, int(round(ls["font_size"] * 1.25 * k))))
    f.setWeight(QtGui.QFont.Weight.Medium)
    p.setFont(f)
    fm = QtGui.QFontMetricsF(f)
    pad, sp = ls["padding"] * k, ls["spacing"] * k
    ic = ls["icon_size"] * k if ls["icons"] else 0
    row_h = (ic + fm.height() + 28 * k) if grid else max(ic, fm.height()) + 16 * k
    input_h = fm.height() + 20 * k
    w = min(ls["width"] * k, r.width() - 20 * k)
    h = 2 * pad + input_h + 12 * k + ls["lines"] * (row_h + sp)
    x = r.center().x() - w / 2
    y = {"center": r.center().y() - h / 2, "top": r.top() + ls["offset"] * k + 10 * k,
         "bottom": r.bottom() - h - ls["offset"] * k - 10 * k}[ls["position"]]
    box = QtCore.QRectF(x, max(r.top() + 4, y), w, min(h, r.height() - 8))
    p.save()
    p.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
    rad = ls["radius"] * k
    # weicher Schatten
    p.setPen(Qt.PenStyle.NoPen)
    for i in range(6):
        p.setBrush(QtGui.QColor(0, 0, 0, 18))
        g = (i + 1) * 3 * k
        p.drawRoundedRect(box.adjusted(-g, -g + 2 * k, g, g + 2 * k), rad + g, rad + g)
    p.setBrush(qcolor(ls["bg"]))
    if ls["border_width"]:
        p.setPen(QtGui.QPen(qcolor(ls["border"]), max(1.0, ls["border_width"] * k)))
    p.drawRoundedRect(box, rad, rad)
    p.setClipRect(box.adjusted(1, 1, -1, -1))
    inner = box.adjusted(pad, pad, -pad, -pad)
    ib = QtCore.QRectF(inner.left(), inner.top(), inner.width(), input_h)
    p.setPen(Qt.PenStyle.NoPen)
    p.setBrush(qcolor(ls["input_bg"]))
    p.drawRoundedRect(ib, max(0.0, rad - 4 * k), max(0.0, rad - 4 * k))
    tx = ib.left() + 14 * k
    if ls["show_prompt"] and ls["app"] != "wofi":
        p.setPen(qcolor(ls["accent"]))
        p.drawText(QtCore.QRectF(tx, ib.top(), ib.width(), ib.height()), Qt.AlignmentFlag.AlignVCenter, ls["prompt"])
        tx += fm.horizontalAdvance(ls["prompt"]) + 10 * k
    p.setPen(qcolor(model.muted_color(ls["fg"])))
    p.drawText(QtCore.QRectF(tx, ib.top(), ib.right() - tx, ib.height()), Qt.AlignmentFlag.AlignVCenter,
               ls["placeholder"])
    y0 = ib.bottom() + 12 * k
    cw = (inner.width() - sp * (cols - 1)) / cols
    n = ls["lines"] * cols
    for i in range(n):
        cx = inner.left() + (i % cols) * (cw + sp)
        cy = y0 + (i // cols) * (row_h + sp)
        cell = QtCore.QRectF(cx, cy, cw, row_h)
        selected = i == 0
        if selected:
            p.setPen(Qt.PenStyle.NoPen)
            p.setBrush(qcolor(ls["sel"]))
            p.drawRoundedRect(cell, ls["el_radius"] * k, ls["el_radius"] * k)
        name = APPS_DEMO[i % len(APPS_DEMO)]
        hue = (i * 47 + 200) % 360
        if grid:
            if ic:
                p.setPen(Qt.PenStyle.NoPen)
                p.setBrush(QtGui.QColor.fromHsl(hue, 150, 140))
                p.drawRoundedRect(QtCore.QRectF(cell.center().x() - ic / 2, cell.top() + 10 * k, ic, ic), ic * 0.25,
                                  ic * 0.25)
            p.setPen(qcolor(ls["sel_fg"] if selected else ls["fg"]))
            p.drawText(QtCore.QRectF(cell.left(), cell.bottom() - fm.height() - 8 * k, cell.width(), fm.height()),
                       Qt.AlignmentFlag.AlignHCenter, fm.elidedText(name, Qt.TextElideMode.ElideRight, cell.width()))
        else:
            ix = cell.left() + 10 * k
            if ic:
                p.setPen(Qt.PenStyle.NoPen)
                p.setBrush(QtGui.QColor.fromHsl(hue, 150, 140))
                p.drawRoundedRect(QtCore.QRectF(ix, cell.center().y() - ic / 2, ic, ic), ic * 0.25, ic * 0.25)
                ix += ic + 12 * k
            p.setPen(qcolor(ls["sel_fg"] if selected else ls["fg"]))
            p.drawText(QtCore.QRectF(ix, cell.top(), cell.right() - ix, cell.height()), Qt.AlignmentFlag.AlignVCenter,
                       name)
    p.restore()
