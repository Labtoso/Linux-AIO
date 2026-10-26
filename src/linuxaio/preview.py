"""Gezeichnete Mini-Desktops als Vorschau (keine Screenshots nötig, skaliert scharf)."""

from .qt import QtCore, QtGui, QtWidgets, Qt

QRectF, QPointF = QtCore.QRectF, QtCore.QPointF
QColor = QtGui.QColor


def _c(hex_: str, alpha: int = 255) -> QColor:
    c = QColor(hex_)
    c.setAlpha(alpha)
    return c


def paint_desktop(p: QtGui.QPainter, r: QRectF, de, variant: str, radius: float = 12.0) -> None:
    p.save()
    p.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
    clip = QtGui.QPainterPath()
    clip.addRoundedRect(r, radius, radius)
    p.setClipPath(clip)

    # Wallpaper: Fläche + zwei Hügel
    base = QColor(de.color)
    p.fillRect(r, base)
    w, h = r.width(), r.height()
    for i, (k, dy) in enumerate(((115, 0.62), (135, 0.74))):
        path = QtGui.QPainterPath(QPointF(r.left(), r.bottom()))
        path.lineTo(r.left(), r.top() + h * dy)
        path.cubicTo(r.left() + w * (0.25 + i * 0.1), r.top() + h * (dy - 0.18),
                     r.left() + w * (0.55 - i * 0.05), r.top() + h * (dy + 0.08),
                     r.right(), r.top() + h * (dy - 0.1))
        path.lineTo(r.right(), r.bottom())
        p.fillPath(path, base.darker(k))
    p.setPen(Qt.PenStyle.NoPen)
    p.setBrush(_c("#ffffff", 40))
    p.drawEllipse(QPointF(r.left() + w * 0.78, r.top() + h * 0.24), h * 0.06, h * 0.06)

    n = {"minimal": 1, "standard": 2, "full": 3}[variant]
    s = h / 100.0  # Maßstab
    lay = de.layout
    top = bottom = 0.0

    def bar(y, height, color="#0b0f14", alpha=235):
        p.setBrush(_c(color, alpha))
        p.drawRect(QRectF(r.left(), y, w, height))

    def dots(x, y, count, size, gap, color="#ffffff", alpha=170):
        p.setBrush(_c(color, alpha))
        for i in range(count):
            p.drawRoundedRect(QRectF(x + i * (size + gap), y, size, size), size * 0.25, size * 0.25)

    def float_dock(count, y, size, rad=0.3):
        width = count * size * 1.35 + size * 0.5
        x = r.left() + (w - width) / 2
        p.setBrush(_c("#0b0f14", 200))
        p.drawRoundedRect(QRectF(x, y, width, size * 1.5), size * rad * 1.5, size * rad * 1.5)
        dots(x + size * 0.45, y + size * 0.25, count, size, size * 0.35)

    if lay == "gnome":
        top = 4.5 * s
        bar(r.top(), top, "#000000", 245)
        p.setBrush(_c("#ffffff", 200))
        p.drawRoundedRect(QRectF(r.left() + 2 * s, r.top() + 1.4 * s, 6 * s, 1.8 * s), s, s)
        p.drawRoundedRect(QRectF(r.center().x() - 4 * s, r.top() + 1.4 * s, 8 * s, 1.8 * s), s, s)
        dots(r.right() - 10 * s, r.top() + 1.3 * s, 3, 2 * s, 1 * s)
        if variant == "full":
            float_dock(7, r.bottom() - 9 * s, 4.5 * s, 0.35)
            bottom = 10 * s
    elif lay == "plasma":
        ph = 6.5 * s
        if variant == "minimal":
            bar(r.bottom() - ph, ph, "#10151c", 240)
            y = r.bottom() - ph
            x0 = r.left()
        else:
            pw = w * (0.62 if variant == "standard" else 0.8)
            x0 = r.left() + (w - pw) / 2
            y = r.bottom() - ph - 1.5 * s
            p.setBrush(_c("#10151c", 235))
            p.drawRoundedRect(QRectF(x0, y, pw, ph), 2.2 * s, 2.2 * s)
        p.setBrush(_c("#3d8bfd", 230))
        p.drawEllipse(QRectF(x0 + 1.6 * s, y + 1.4 * s, 3.7 * s, 3.7 * s))
        dots(x0 + 7.5 * s, y + 1.6 * s, 4 + n, 3.3 * s, 1.4 * s)
        bottom = ph + 2 * s
    elif lay == "xfce":
        top = 4.5 * s
        bar(r.top(), top, "#1c2420")
        dots(r.left() + 1.5 * s, r.top() + 1.1 * s, 5, 2.3 * s, 1.5 * s)
        float_dock(4 + n, r.bottom() - 8 * s, 3.8 * s, 0.2)
        bottom = 9 * s
    elif lay == "cinnamon":
        bh = 6 * s
        bar(r.bottom() - bh, bh, "#1b1712", 245)
        p.setBrush(_c("#8fd16a", 220))
        p.drawRoundedRect(QRectF(r.left() + 1.5 * s, r.bottom() - bh + 1.3 * s, 3.4 * s, 3.4 * s), s, s)
        dots(r.left() + 7 * s, r.bottom() - bh + 1.5 * s, 3 + n, 3 * s, 1.5 * s)
        dots(r.right() - 14 * s, r.bottom() - bh + 2 * s, 4, 2 * s, 1.2 * s)
        bottom = bh
    elif lay == "mate":
        top = 4 * s
        bar(r.top(), top, "#2c3a2e")
        bar(r.bottom() - 4 * s, 4 * s, "#2c3a2e")
        dots(r.left() + 1.5 * s, r.top() + 0.9 * s, 3, 2.2 * s, 2 * s)
        dots(r.left() + 1.5 * s, r.bottom() - 3.1 * s, 2 + n, 6 * s, 1 * s, alpha=60)
        bottom = 4 * s
    elif lay == "budgie":
        top = 4 * s
        bar(r.top(), top, "#100c18", 240)
        dots(r.left() + 1.5 * s, r.top() + 1 * s, 2, 2 * s, 1.5 * s)
        float_dock(3 + n, r.bottom() - 8 * s, 3.8 * s, 0.3)
        bottom = 9 * s
        if variant == "full":
            p.setBrush(_c("#100c18", 220))
            p.drawRoundedRect(QRectF(r.right() - 22 * s, r.top() + top + 2 * s, 20 * s, h - top - 14 * s), 2 * s, 2 * s)
    elif lay == "lxqt":
        bh = 5 * s
        bar(r.bottom() - bh, bh, "#131a24", 245)
        p.setBrush(_c("#3d8bfd", 220))
        p.drawRoundedRect(QRectF(r.left() + 1.2 * s, r.bottom() - bh + 1 * s, 3 * s, 3 * s), 0.6 * s, 0.6 * s)
        dots(r.left() + 6 * s, r.bottom() - bh + 1.2 * s, 2 + n, 2.6 * s, 1.2 * s)
        bottom = bh
    elif lay == "cosmic":
        top = 4 * s
        bar(r.top(), top, "#120d1c", 230)
        dots(r.left() + 2 * s, r.top() + 1 * s, 2, 2 * s, 2 * s)
        float_dock(4 + n, r.bottom() - 9 * s, 4.2 * s, 0.45)
        bottom = 10 * s

    # Fenster
    if de.tiling:
        gap = {"hyprland": 2.2, "sway": 1.0, "i3": 0.0}[lay] * s
        rad = 2.2 * s if lay == "hyprland" else 0.0
        bh = 4 * s
        if lay == "hyprland":
            p.setBrush(_c("#0b0f14", 210))
            p.drawRoundedRect(QRectF(r.left() + gap, r.top() + gap, w - 2 * gap, bh), 2 * s, 2 * s)
            dots(r.left() + gap + 2 * s, r.top() + gap + 1 * s, 5, 2 * s, 1.2 * s)
            area = QRectF(r.left() + gap, r.top() + bh + 2 * gap, w - 2 * gap, h - bh - 3 * gap)
        else:
            bar(r.bottom() - bh, bh, "#000000", 245)
            dots(r.left() + 1 * s, r.bottom() - bh + 1 * s, 5, 2 * s, 0.8 * s, color="#3d8bfd" if lay == "i3" else "#8bd5a0")
            area = QRectF(r.left() + gap, r.top() + gap, w - 2 * gap, h - bh - 2 * gap)
        cells = []
        if n == 1:
            cells = [area]
        elif n == 2:
            half = (area.width() - gap) / 2
            cells = [QRectF(area.left(), area.top(), half, area.height()),
                     QRectF(area.left() + half + gap, area.top(), half, area.height())]
        else:
            half = (area.width() - gap) / 2
            hh = (area.height() - gap) / 2
            cells = [QRectF(area.left(), area.top(), half, area.height()),
                     QRectF(area.left() + half + gap, area.top(), half, hh),
                     QRectF(area.left() + half + gap, area.top() + hh + gap, half, hh)]
        border = {"hyprland": "#33ccff", "sway": "#4c7899", "i3": "#285577"}[lay]
        for i, c in enumerate(cells):
            p.setBrush(_c("#0d1117", 235 if lay != "hyprland" else 215))
            p.setPen(QtGui.QPen(_c(border if i == 0 else "#30363d"), max(1.0, 0.5 * s)))
            p.drawRoundedRect(c, rad, rad)
            p.setPen(Qt.PenStyle.NoPen)
            _lines(p, c.adjusted(2 * s, 2 * s, -2 * s, -2 * s), s, 4 + i)
    else:
        area = QRectF(r.left(), r.top() + top, w, h - top - bottom)
        rects = [QRectF(0.10, 0.14, 0.52, 0.66), QRectF(0.42, 0.08, 0.48, 0.58), QRectF(0.26, 0.38, 0.44, 0.52)]
        for i in range(n):
            q = rects[i]
            c = QRectF(area.left() + area.width() * q.left(), area.top() + area.height() * q.top(),
                       area.width() * q.width(), area.height() * q.height())
            _window(p, c, s, lay, active=(i == n - 1))
    p.restore()


def _lines(p, c: QRectF, s: float, count: int) -> None:
    p.setBrush(_c("#ffffff", 45))
    for i in range(count):
        y = c.top() + i * 3 * s
        if y + 1.4 * s > c.bottom():
            break
        p.drawRoundedRect(QRectF(c.left(), y, c.width() * (0.4 + 0.5 * ((i * 37) % 10) / 10), 1.4 * s), 0.7 * s, 0.7 * s)


def _window(p, c: QRectF, s: float, lay: str, active: bool) -> None:
    rad = {"gnome": 2.6, "cosmic": 2.6, "plasma": 1.8, "budgie": 1.8}.get(lay, 1.0) * s
    p.setBrush(_c("#000000", 70))
    p.drawRoundedRect(c.translated(0, 0.8 * s), rad, rad)
    p.setBrush(_c("#1e252e" if active else "#171d24"))
    p.drawRoundedRect(c, rad, rad)
    tb = 4.2 * s
    head = QRectF(c.left(), c.top(), c.width(), tb)
    p.setBrush(_c("#262f3a" if active else "#1d242c"))
    path = QtGui.QPainterPath()
    path.addRoundedRect(head, rad, rad)
    path.addRect(QRectF(c.left(), c.top() + tb / 2, c.width(), tb / 2))
    p.drawPath(path.simplified())
    # Fensterknöpfe
    d = 1.6 * s
    y = c.top() + (tb - d) / 2
    if lay in ("gnome", "cosmic", "budgie"):
        p.setBrush(_c("#ffffff", 90))
        p.drawEllipse(QRectF(c.right() - 3 * s, y, d, d))
    else:
        cols = ("#ffffff", "#ffffff", "#e5534b") if lay != "plasma" else ("#ffffff", "#ffffff", "#ffffff")
        for i, col in enumerate(cols):
            p.setBrush(_c(col, 110))
            p.drawEllipse(QRectF(c.right() - (3 + i * 2.8) * s, y, d, d))
    _lines(p, c.adjusted(2.5 * s, tb + 2 * s, -2.5 * s, -2 * s), s, 6)


class Preview(QtWidgets.QWidget):
    def __init__(self, parent=None, radius: float = 14.0):
        super().__init__(parent)
        self.de = None
        self.variant = "standard"
        self.radius = radius
        self.setMinimumSize(160, 100)

    def set(self, de, variant: str) -> None:
        self.de, self.variant = de, variant
        self.update()

    def hasHeightForWidth(self) -> bool:
        return True

    def heightForWidth(self, w: int) -> int:
        return int(w * 10 / 16)

    def paintEvent(self, _):
        if not self.de:
            return
        p = QtGui.QPainter(self)
        r = QRectF(self.rect())
        target_h = r.width() * 10 / 16
        if target_h > r.height():
            tw = r.height() * 16 / 10
            r = QRectF(r.left() + (r.width() - tw) / 2, r.top(), tw, r.height())
        else:
            r = QRectF(r.left(), r.top() + (r.height() - target_h) / 2, r.width(), target_h)
        paint_desktop(p, r, self.de, self.variant, self.radius)
        p.end()
