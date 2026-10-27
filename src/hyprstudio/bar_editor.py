"""Waybar › Editor: Module per Drag & Drop direkt auf der Leiste anordnen.
Ziehen innerhalb der Leiste = verschieben (links/Mitte/rechts), aus der Palette auf die Leiste = hinzufügen,
von der Leiste weg = entfernen. Rechtsklick auf ein Modul: entfernen oder in einen anderen Bereich."""

from . import model, theme
from .i18n import t
from .preview import qcolor
from .qt import QtCore, QtGui, QtWidgets, Qt, Signal

SIDES = ("left", "center", "right")


class BarEditor(QtWidgets.QWidget):
    changed = Signal(dict)  # {"left": [...], "center": [...], "right": [...]}

    def __init__(self, parent=None):
        super().__init__(parent)
        self.ws = model.waybar_state(None)
        self.setMinimumHeight(330)
        self.setMouseTracking(True)
        self.drag = None      # (modul, herkunft) – herkunft = Seite oder "palette"
        self.drag_pos = None
        self.hover = None
        self.accent = theme.THEMES["tokyo"]

    def load(self, ws: dict):
        self.ws = ws
        self.update()

    # ----- Geometrie -----
    def _font(self) -> QtGui.QFont:
        f = QtGui.QFont(self.font())
        f.setPixelSize(13)
        f.setWeight(QtGui.QFont.Weight.DemiBold)
        return f

    def bar_rect(self) -> QtCore.QRectF:
        return QtCore.QRectF(18, 46, self.width() - 36, 52)

    def _w(self, mod: str, fm) -> float:
        return fm.horizontalAdvance(t("m." + mod)) + 30

    def layout_mods(self, skip: str | None = None) -> dict:
        """{seite: [(modul, rect)]} – das gerade gezogene Modul wird ausgelassen."""
        fm = QtGui.QFontMetricsF(self._font())
        bar = self.bar_rect()
        gap, h = 6, bar.height() - 16
        y = bar.top() + 8
        out = {}
        for side in SIDES:
            mods = [m for m in self.ws[side] if m != skip]
            widths = [self._w(m, fm) for m in mods]
            total = sum(widths) + gap * max(0, len(mods) - 1)
            x = {"left": bar.left() + 10, "center": bar.center().x() - total / 2,
                 "right": bar.right() - 10 - total}[side]
            items = []
            for m, w in zip(mods, widths):
                items.append((m, QtCore.QRectF(x, y, w, h)))
                x += w + gap
            out[side] = items
        return out

    def palette_items(self) -> list:
        fm = QtGui.QFontMetricsF(self._font())
        used = set(self.ws["left"] + self.ws["center"] + self.ws["right"])
        x, y = 18.0, self.bar_rect().bottom() + 70
        items = []
        for m in model.MODULES:
            if m in used:
                continue
            w = self._w(m, fm)
            if x + w > self.width() - 18:
                x, y = 18.0, y + 44
            items.append((m, QtCore.QRectF(x, y, w, 34)))
            x += w + 8
        return items

    def _hit(self, pos) -> tuple | None:
        for side, items in self.layout_mods().items():
            for m, r in items:
                if r.contains(pos):
                    return m, side, r
        for m, r in self.palette_items():
            if r.contains(pos):
                return m, "palette", r
        return None

    def _target(self, pos) -> tuple | None:
        """(seite, index) für eine Ablage an pos – None = außerhalb der Leiste (entfernen)."""
        bar = self.bar_rect()
        if not bar.adjusted(-10, -30, 10, 30).contains(pos):
            return None
        third = bar.width() / 3
        side = SIDES[min(2, max(0, int((pos.x() - bar.left()) // third)))]
        items = self.layout_mods(skip=self.drag[0] if self.drag else None)[side]
        idx = sum(1 for _, r in items if r.center().x() < pos.x())
        return side, idx

    # ----- Zeichnen -----
    def paintEvent(self, _):
        p = QtGui.QPainter(self)
        p.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
        f = self._font()
        p.setFont(f)
        ws = self.ws
        bar = self.bar_rect()
        # Hintergrund-Streifen (Wallpaper) hinter der Leiste
        strip = QtCore.QRectF(0, 0, self.width(), bar.bottom() + 30)
        clip = QtGui.QPainterPath()
        clip.addRoundedRect(strip, 14, 14)
        p.fillPath(clip, QtGui.QColor("#22325a"))
        bg, fg, acc = qcolor(ws["bg"]), qcolor(ws["fg"]), qcolor(ws["accent"])
        rad = min(ws["radius"], bar.height() / 2)
        pen = QtGui.QPen(qcolor(ws["border"]), max(1, ws["border_width"])) if ws["border_width"] else Qt.PenStyle.NoPen
        if ws["style"] != "islands":
            p.setPen(pen)
            p.setBrush(bg)
            r = rad if ws["style"] == "float" else 0
            p.drawRoundedRect(bar, r, r)
        dragging = self.drag is not None and self.drag_pos is not None
        if dragging:  # Ablagebereiche zeigen
            third = bar.width() / 3
            tgt = self._target(self.drag_pos)
            for i, side in enumerate(SIDES):
                zr = QtCore.QRectF(bar.left() + i * third + 3, bar.top() - 26, third - 6, bar.height() + 52)
                on = tgt and tgt[0] == side
                p.setPen(QtGui.QPen(QtGui.QColor(self.accent if on else theme.MUTED), 1.5, Qt.PenStyle.DashLine))
                p.setBrush(QtGui.QColor(61, 139, 253, 28) if on else Qt.BrushStyle.NoBrush)
                p.drawRoundedRect(zr, 12, 12)
                p.setPen(QtGui.QColor(theme.TEXT if on else theme.MUTED))
                p.drawText(QtCore.QRectF(zr.left(), zr.top() + 2, zr.width(), 20), Qt.AlignmentFlag.AlignCenter,
                           t("col." + side))
        lay = self.layout_mods(skip=self.drag[0] if dragging else None)
        for side, items in lay.items():
            for m, r in items:
                self._module(p, m, r, ws, bg, fg, acc, pen, hover=(self.hover == m and not dragging))
        if dragging:
            tgt = self._target(self.drag_pos)
            if tgt:
                side, idx = tgt
                items = lay[side]
                if items:
                    x = items[idx - 1][1].right() + 3 if idx > 0 else items[0][1].left() - 3
                else:
                    x = {"left": bar.left() + 10, "center": bar.center().x(), "right": bar.right() - 10}[side]
                p.setPen(QtGui.QPen(QtGui.QColor(self.accent), 3, Qt.PenStyle.SolidLine, Qt.PenCapStyle.RoundCap))
                p.drawLine(QtCore.QPointF(x, bar.top() + 6), QtCore.QPointF(x, bar.bottom() - 6))
        # Palette
        p.setPen(QtGui.QColor(theme.MUTED))
        p.drawText(QtCore.QPointF(18, bar.bottom() + 58), t("ed.palette"))
        for m, r in self.palette_items():
            if dragging and self.drag[0] == m:
                continue
            p.setPen(QtGui.QPen(QtGui.QColor(self.accent if self.hover == m else theme.BORDER), 1))
            p.setBrush(QtGui.QColor(theme.SURFACE2))
            p.drawRoundedRect(r, 10, 10)
            p.setPen(QtGui.QColor(theme.TEXT))
            p.drawText(r, Qt.AlignmentFlag.AlignCenter, t("m." + m))
        if not any(True for _ in self.palette_items()):
            p.setPen(QtGui.QColor(theme.MUTED))
            p.drawText(QtCore.QPointF(18, bar.bottom() + 90), t("ed.all_used"))
        # gezogenes Modul am Mauszeiger
        if dragging:
            fm = QtGui.QFontMetricsF(f)
            w = self._w(self.drag[0], fm)
            r = QtCore.QRectF(self.drag_pos.x() - w / 2, self.drag_pos.y() - 18, w, 36)
            p.setOpacity(0.92)
            p.setPen(QtGui.QPen(QtGui.QColor(self.accent), 2))
            p.setBrush(QtGui.QColor(theme.SURFACE))
            p.drawRoundedRect(r, 10, 10)
            p.setPen(QtGui.QColor(theme.TEXT))
            p.drawText(r, Qt.AlignmentFlag.AlignCenter, t("m." + self.drag[0]))
            p.setOpacity(1)
        p.end()

    def _module(self, p, m, r, ws, bg, fg, acc, pen, hover=False):
        if ws["style"] == "islands":
            p.setPen(pen)
            p.setBrush(bg)
            p.drawRoundedRect(r.adjusted(-2, -4, 2, 4), min(ws["radius"], r.height() / 2), min(ws["radius"], r.height() / 2))
        if hover:
            p.setPen(Qt.PenStyle.NoPen)
            p.setBrush(QtGui.QColor(255, 255, 255, 26))
            p.drawRoundedRect(r, 8, 8)
        col = acc if m == "custom/launcher" else QtGui.QColor("#e5534b") if m == "custom/power" else \
            qcolor(model.module_color(m, ws)) if ws["colorful"] else fg
        p.setPen(col)
        p.drawText(r, Qt.AlignmentFlag.AlignCenter, t("m." + m))

    # ----- Maus -----
    def mousePressEvent(self, e):
        pos = e.position()
        hit = self._hit(pos)
        if e.button() == Qt.MouseButton.RightButton and hit and hit[1] != "palette":
            self._menu(hit[0], hit[1], e.globalPosition().toPoint())
            return
        if e.button() == Qt.MouseButton.LeftButton and hit:
            self.drag = (hit[0], hit[1])
            self.drag_pos = pos
            self.setCursor(Qt.CursorShape.ClosedHandCursor)
            self.update()

    def mouseMoveEvent(self, e):
        pos = e.position()
        if self.drag:
            self.drag_pos = pos
        else:
            hit = self._hit(pos)
            self.hover = hit[0] if hit else None
            self.setCursor(Qt.CursorShape.OpenHandCursor if hit else Qt.CursorShape.ArrowCursor)
        self.update()

    def mouseReleaseEvent(self, e):
        if not self.drag:
            return
        mod, src = self.drag
        tgt = self._target(e.position())
        self.drag, self.drag_pos = None, None
        self.setCursor(Qt.CursorShape.OpenHandCursor)
        lists = {s: [m for m in self.ws[s] if m != mod] for s in SIDES}
        if tgt:
            side, idx = tgt
            lists[side].insert(idx, mod)
        elif src == "palette":
            self.update()
            return
        self.changed.emit(lists)

    def leaveEvent(self, e):
        self.hover = None
        self.update()

    def _menu(self, mod, side, gpos):
        m = QtWidgets.QMenu(self)
        moves = {}
        for s in SIDES:
            if s != side:
                moves[m.addAction(t("ed.move_" + s))] = s
        m.addSeparator()
        rm = m.addAction(t("keys.remove"))
        a = m.exec(gpos)
        if a is None:
            return
        lists = {s: [x for x in self.ws[s] if x != mod] for s in SIDES}
        if a in moves:
            lists[moves[a]].append(mod)
        elif a is not rm:
            return
        self.changed.emit(lists)
