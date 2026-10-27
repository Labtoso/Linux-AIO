"""GNOME-Bereiche im Tab „Einstellungen“: wie GNOME Tweaks plus Titelleisten-Knöpfe, obere Leiste,
Startprogramme und Anmeldebildschirm (GDM)."""

from pathlib import Path

from hyprstudio.widgets import Switch

from . import corners, gnome, installers, theme, usercmd
from .form import Card, MissingButton, Page, RemoveButton, Row, combo, gsection, retranslate_buttons, set_chip, start_job, tool_button
from .i18n import t
from .qt import QtCore, QtGui, QtWidgets, Qt
from .widgets import CACHE, draw_icon


def simple_page(key: str, section: str) -> Page:
    pg = Page("gn." + key)
    card = Card()
    pg.body.addWidget(card)
    gsection(pg, card, gnome.SECTIONS[section])
    pg.finish()
    return pg


# ---------- fehlende Erweiterung per Klick ----------

class ExtInstall:
    """Mixin: Erweiterung von extensions.gnome.org holen, sonst Paket per pkexec (Plan B)."""

    def install_ext(self, uuid: str):
        host = self.host
        name = installers.EXT_NAMES.get(uuid, uuid)
        host.log("")
        host.task_begin(t("miss.installing", name=name))

        def done(ok, res):
            host.task_end(True, "")
            if ok:
                key = {"relogin": "miss.relogin", "incompatible": "miss.incompatible"}.get(res, "miss.done")
                host.set_status(t(key, name=name))
                self.refresh()
                return
            host.log(str(res))
            key = installers.EXT_PACKAGE.get(uuid)
            if key:
                self._pending_ext = uuid
                host.run(["pkg-install", key], "pkg-install", self)
            else:
                host.set_status(f"{t('failed')}: {res}")
        start_job(self, lambda: installers.install_extension(uuid), done)

    def uninstall_ext(self, uuid: str):
        from hyprstudio import dialogs
        name = installers.EXT_NAMES.get(uuid, uuid)
        if not dialogs.confirm(self, t("ext.uninstall"), t("ext.uninstall.ask", name=name)):
            return
        self.host.log("")
        gnome.ext_uninstall(uuid)
        self.host.set_status(t("ext.uninstalled", name=name))
        self.refresh()

    def finished(self, ok, action, result):
        uuid = getattr(self, "_pending_ext", None)
        if action == "pkg-install" and ok and uuid:
            usercmd.run(["gnome-extensions", "enable", uuid])
            self.host.set_status(t("miss.relogin", name=installers.EXT_NAMES.get(uuid, uuid)))
        self._pending_ext = None
        self.refresh()


# ---------- Erscheinungsbild ----------

class LookGnome(ExtInstall, Page):
    def __init__(self, host):
        super().__init__("gn.look")
        self.host = host
        card = Card("gn.look.style")
        self.body.addWidget(card)
        gsection(self, card, gnome.SECTIONS["look"])
        themes = Card("gn.look.themes")
        self.body.addWidget(themes)
        self.gtk = combo()
        self.gtk.textActivated.connect(lambda n: self._set_gtk(n))
        self.r_gtk = Row("gn.gtk", self.gtk)
        themes.lay.addWidget(self.r_gtk)
        shell_w = QtWidgets.QWidget()
        sl = QtWidgets.QHBoxLayout(shell_w)
        sl.setContentsMargins(0, 0, 0, 0)
        sl.setSpacing(8)
        self.shell_chip = QtWidgets.QLabel()
        self.shell_get = MissingButton()
        self.shell_get.clicked.connect(lambda: self.install_ext(gnome.USER_THEME))
        self.shell_rm = RemoveButton()
        self.shell_rm.clicked.connect(lambda: self.uninstall_ext(gnome.USER_THEME))
        self.shell = combo()
        self.shell.textActivated.connect(lambda n: self._set_shell(n))
        sl.addWidget(self.shell_chip)
        sl.addWidget(self.shell_get)
        sl.addWidget(self.shell_rm)
        sl.addWidget(self.shell)
        self.r_shell = Row("gn.shell", shell_w)
        themes.lay.addWidget(self.r_shell)
        self.finish()

    def reset_keys(self):
        return super().reset_keys() + [(gnome.IFACE, "gtk-theme"), ("org.gnome.shell.extensions.user-theme", "name")]

    def _set_gtk(self, name):
        usercmd.gset(gnome.IFACE, "gtk-theme", name)
        self.host.set_status(t("done"))

    def _set_shell(self, name):
        gnome.set_shell_theme("" if name == t("gn.default") else name)
        self.host.set_status(t("done"))
        self.refresh()

    def retranslate(self):
        super().retranslate()
        self.r_gtk.retranslate()
        self.r_shell.retranslate()

    def refresh(self):
        super().refresh()
        cur = usercmd.gget(gnome.IFACE, "gtk-theme", "Adwaita") or "Adwaita"
        self.gtk.blockSignals(True)
        self.gtk.clear()
        names = gnome.list_themes("gtk")
        self.gtk.addItems(names if cur in names else names + [cur])
        self.gtk.setCurrentText(cur)
        self.gtk.blockSignals(False)
        ext_ok = gnome.ext_installed(gnome.USER_THEME)
        on = gnome.USER_THEME in gnome.ext_enabled()
        set_chip(self.shell_chip, "ChipOk" if on else "ChipWarn",
                 t("gn.ext.on") if on else (t("gn.ext.off") if ext_ok else t("gn.ext.missing")))
        self.shell_chip.setToolTip(t("gn.shell.tip"))
        self.shell_get.setVisible(not ext_ok)
        self.shell_rm.setVisible(ext_ok and gnome.ext_removable(gnome.USER_THEME))
        self.shell_get.setEnabled(not self.host.busy())
        self.shell_get.setToolTip(t("miss.install") + ": User Themes")
        self.shell.blockSignals(True)
        self.shell.clear()
        self.shell.addItem(t("gn.default"))
        self.shell.addItems(gnome.list_themes("shell"))
        cur_s = gnome.shell_theme()
        self.shell.setCurrentText(cur_s or t("gn.default"))
        self.shell.setEnabled(ext_ok)
        self.shell.blockSignals(False)


def restart_files():
    """Files (Nautilus) keeps running in the background and only reads gtk.css at start."""
    usercmd.run(["nautilus", "-q"])


# ---------- Fenster (mit Eckenrundung) ----------

class CornerPreview(QtWidgets.QWidget):
    def __init__(self, parent=None):
        super().__init__(parent)
        self.radius = 15
        self.setFixedHeight(120)

    def set_radius(self, r):
        self.radius = r
        self.update()

    def paintEvent(self, _):
        p = QtGui.QPainter(self)
        p.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
        bg = QtCore.QRectF(self.rect()).adjusted(1, 1, -1, -1)
        p.setPen(Qt.PenStyle.NoPen)
        p.setBrush(QtGui.QColor("#3b5470"))
        p.drawRoundedRect(bg, 12, 12)
        w = QtCore.QRectF(bg.center().x() - 160, bg.top() + 16, 320, bg.height() - 32)
        path = QtGui.QPainterPath()
        path.addRoundedRect(w, self.radius, self.radius)
        p.fillPath(path, QtGui.QColor("#242a32"))
        p.save()
        p.setClipPath(path)
        p.fillRect(QtCore.QRectF(w.left(), w.top(), w.width(), 30), QtGui.QColor("#2e3540"))
        p.restore()
        p.setPen(QtGui.QColor(theme.TEXT))
        p.drawText(QtCore.QRectF(w.left(), w.top(), w.width(), 30), Qt.AlignmentFlag.AlignCenter, f"{self.radius} px")
        p.end()


class WindowsPage(ExtInstall, Page):
    def __init__(self, host):
        super().__init__("gn.windows")
        self.host = host
        card = Card()
        self.body.addWidget(card)
        gsection(self, card, gnome.SECTIONS["windows"])
        rc = Card("gn.corners")
        self.body.addWidget(rc)
        self.preview = CornerPreview()
        rc.lay.addWidget(self.preview)
        sw = QtWidgets.QWidget()
        hl = QtWidgets.QHBoxLayout(sw)
        hl.setContentsMargins(0, 0, 0, 0)
        self.slider = QtWidgets.QSlider(Qt.Orientation.Horizontal)
        self.slider.setRange(0, 24)
        self.slider.setMinimumWidth(120)
        self.value = QtWidgets.QLabel()
        self.value.setFixedWidth(48)
        self.b_orig = tool_button("gn.login.restore")
        self.b_orig.clicked.connect(self.reset_corners)
        self.b_files = tool_button("gn.files.restart")
        self.b_files.clicked.connect(restart_files)
        hl.addWidget(self.slider)
        hl.addWidget(self.value)
        hl.addWidget(self.b_orig)
        hl.addWidget(self.b_files)
        self.r_radius = Row("gn.corners.radius", sw)
        rc.lay.addWidget(self.r_radius)
        ew = QtWidgets.QWidget()
        el = QtWidgets.QHBoxLayout(ew)
        el.setContentsMargins(0, 0, 0, 0)
        el.setSpacing(8)
        self.ext_chip = QtWidgets.QLabel()
        self.ext_get = MissingButton()
        self.ext_get.clicked.connect(lambda: self.install_ext(corners.EXT_UUID))
        self.ext_rm = RemoveButton()
        self.ext_rm.clicked.connect(lambda: self.uninstall_ext(corners.EXT_UUID))
        self.sw_ext = Switch()
        self.sw_ext.toggled.connect(self._ext_toggle)
        self.b_force = tool_button("gn.ext.force")
        self.b_force.clicked.connect(self._force)
        el.addWidget(self.ext_chip)
        el.addWidget(self.ext_get)
        el.addWidget(self.ext_rm)
        el.addWidget(self.b_force)
        el.addWidget(self.sw_ext)
        self.r_ext = Row("gn.corners.all", ew)
        rc.lay.addWidget(self.r_ext)

        self.timer = QtCore.QTimer(self)
        self.timer.setSingleShot(True)
        self.timer.setInterval(400)
        self.timer.timeout.connect(self._apply_corners)
        self.slider.valueChanged.connect(self._slide)
        self.finish()

    def reset_extra(self, bak):
        from .form import backup_files
        backup_files(corners.files(), bak)
        corners.remove()

    def undo_extra(self, bak):
        from .form import restore_files
        restore_files(corners.files(), bak)

    def _slide(self, v):
        self.preview.set_radius(v)
        self.value.setText(f"{v} px")
        self.timer.start()

    def _apply_corners(self):
        v = self.slider.value()
        corners.apply(v)
        if gnome.ext_installed(corners.EXT_UUID) and corners.EXT_UUID in gnome.ext_enabled():
            corners.set_extension_radius(v)
        self.b_orig.setEnabled(True)
        self.host.set_status(t("gn.corners.done", r=v))

    def reset_corners(self):
        corners.remove()
        self.slider.blockSignals(True)
        self.slider.setValue(15)
        self.slider.blockSignals(False)
        self.preview.set_radius(15)
        self.value.setText("15 px")
        self.b_orig.setEnabled(False)
        self.host.set_status(t("gn.corners.reset"))

    def _force(self):
        from hyprstudio import dialogs
        if not dialogs.confirm(self, t("gn.ext.force"), t("gn.ext.force.ask")):
            return
        usercmd.gset("org.gnome.shell", "disable-extension-version-validation", True)
        gnome.ext_set(corners.EXT_UUID, True)
        corners.set_extension_radius(self.slider.value())
        self.host.set_status(t("miss.relogin", name="Rounded Window Corners Reborn"))
        self.refresh()

    def _ext_toggle(self, on):
        gnome.ext_set(corners.EXT_UUID, bool(on))
        if on:
            corners.set_extension_radius(self.slider.value())
        self.host.set_status(t("done"))

    def retranslate(self):
        super().retranslate()
        self.r_radius.retranslate()
        self.r_ext.retranslate()
        self.r_radius.label.setToolTip(t("gn.corners.note"))      # Hinweise nur als Tooltip
        retranslate_buttons(self)

    def refresh(self):
        super().refresh()
        cur = corners.current()
        self.slider.blockSignals(True)
        self.slider.setValue(15 if cur is None else cur)
        self.slider.blockSignals(False)
        self.preview.set_radius(self.slider.value())
        self.value.setText(f"{self.slider.value()} px")
        self.b_orig.setEnabled(cur is not None)
        have = gnome.ext_installed(corners.EXT_UUID)
        on = corners.EXT_UUID in gnome.ext_enabled()
        compat = gnome.ext_compatible(corners.EXT_UUID)
        blocked = have and compat is False and not gnome.version_check_off()
        if blocked:
            set_chip(self.ext_chip, "ChipWarn", t("gn.ext.incompatible"))
        else:
            set_chip(self.ext_chip, "ChipOk" if on else ("Chip" if have else "ChipWarn"),
                     t("gn.ext.on") if on else (t("gn.ext.off") if have else t("gn.ext.missing")))
        self.b_force.setVisible(blocked)
        self.b_force.setToolTip(t("gn.ext.force.ask"))
        self.ext_chip.setToolTip("Rounded Window Corners Reborn")
        self.ext_get.setVisible(not have)
        self.ext_rm.setVisible(have and gnome.ext_removable(corners.EXT_UUID))
        self.ext_get.setToolTip(t("miss.install") + ": Rounded Window Corners Reborn")
        self.sw_ext.setEnabled(have and not blocked)
        self.sw_ext.set_now(on)


# ---------- Titelleiste ----------

class TitlebarPreview(QtWidgets.QWidget):
    """Gezeichnete Titelleiste: Knöpfe je nach Layout und Knopf-Theme."""

    def __init__(self, parent=None, height: int = 64, title: bool = True):
        super().__init__(parent)
        self.layout_ = {"side": "right", "minimize": False, "maximize": False, "close": True}
        self.style_ = None
        self.title = title
        self.setFixedHeight(height)

    def set_layout(self, d):
        self.layout_ = d
        self.update()

    def set_style(self, name):
        self.style_ = name
        self.update()

    def paintEvent(self, _):
        from . import buttons
        p = QtGui.QPainter(self)
        p.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
        r = QtCore.QRectF(self.rect()).adjusted(1, 1, -1, -1)
        p.setPen(QtGui.QPen(QtGui.QColor(theme.BORDER), 1))
        p.setBrush(QtGui.QColor("#242a32"))
        p.drawRoundedRect(r, 12, 12)
        if self.title:
            p.setPen(QtGui.QColor(theme.TEXT))
            f = p.font()
            f.setBold(True)
            p.setFont(f)
            p.drawText(r, Qt.AlignmentFlag.AlignCenter, t("gn.title.window"))
        d = self.layout_
        btns = (["close"] if d.get("close", True) else []) + (["maximize"] if d["maximize"] else []) \
            + (["minimize"] if d["minimize"] else [])
        if d["side"] == "right":
            btns = list(reversed(btns))
        st = buttons.THEMES.get(self.style_) or {"shape": "circle", "size": 26, "close": "#3a424d",
                                                 "min": "#3a424d", "max": "#3a424d", "icons": "always"}
        k = r.height() / 64
        size = max(10.0, st["size"] * 1.1 * k)
        bw = size * 2 if st["shape"] == "wide" else size
        gap = 8 * k if st["shape"] != "wide" else 0
        total = len(btns) * bw + max(0, len(btns) - 1) * gap
        x = r.left() + 16 * k if d["side"] == "left" else r.right() - 16 * k - total
        if st["shape"] == "wide":
            x = r.left() + 4 if d["side"] == "left" else r.right() - 4 - total
        y = r.center().y() - size / 2
        for b in btns:
            c = QtCore.QRectF(x, y, bw, size)
            col = st.get({"close": "close", "maximize": "max", "minimize": "min"}[b]) or ""
            p.setPen(Qt.PenStyle.NoPen)
            if col:
                p.setBrush(QtGui.QColor(col))
                if st["shape"] == "circle":
                    p.drawEllipse(c)
                else:
                    p.drawRoundedRect(c, 4 if st["shape"] == "square" else 0, 4 if st["shape"] == "square" else 0)
            if st.get("icons") != "never" and (st.get("icons") == "always" or not col):
                light = not col or QtGui.QColor(col).lightness() < 128
                p.setPen(QtGui.QPen(QtGui.QColor(theme.TEXT if light else "#1b1b1b"), 1.6,
                                    Qt.PenStyle.SolidLine, Qt.PenCapStyle.RoundCap))
                p.setBrush(Qt.BrushStyle.NoBrush)
                m = min(bw, size) * 0.33
                ic = QtCore.QRectF(c.center().x() - m / 2, c.center().y() - m / 2, m, m)
                if b == "close":
                    p.drawLine(ic.topLeft(), ic.bottomRight())
                    p.drawLine(ic.topRight(), ic.bottomLeft())
                elif b == "maximize":
                    p.drawRect(ic)
                else:
                    p.drawLine(QtCore.QPointF(ic.left(), ic.bottom()), QtCore.QPointF(ic.right(), ic.bottom()))
            x += bw + gap
        p.end()


class ButtonTile(QtWidgets.QFrame):
    """Kachel eines Knopf-Themes: Vorschau + Installieren/Angewendet."""

    def __init__(self, name: str, page, parent=None):
        super().__init__(parent)
        from . import buttons
        self.name, self.page = name, page
        self.setObjectName("Tile")
        self.setFixedSize(262, 150)
        lay = QtWidgets.QVBoxLayout(self)
        lay.setContentsMargins(10, 10, 10, 10)
        lay.setSpacing(8)
        self.pre = TitlebarPreview(height=56, title=False)
        self.pre.set_style(name)
        lay.addWidget(self.pre)
        lb = QtWidgets.QLabel(buttons.THEMES[name]["titel"])
        lb.setStyleSheet("font-weight:600; background:transparent;")
        lay.addWidget(lb)
        self.btn = QtWidgets.QPushButton()
        self.btn.setObjectName("Primary")
        self.btn.setCursor(Qt.CursorShape.PointingHandCursor)
        self.btn.setMinimumHeight(30)
        self.btn.clicked.connect(lambda: page.apply_buttons(self.name))
        lay.addWidget(self.btn)

    def refresh(self, layout: dict, current: str, accent: str):
        from .widgets import set_applied
        self.pre.set_layout(dict(layout, minimize=True, maximize=True, close=True))
        active = current == self.name
        set_applied(self.btn, active, t("applied"))
        if not active:
            self.btn.setText(t("install"))
            self.btn.setEnabled(True)
        border = accent if active else theme.BORDER
        self.setStyleSheet(f"#Tile {{ background:{theme.SURFACE}; border:{2 if active else 1}px solid {border};"
                           f" border-radius:14px; }}")


class TitlebarPage(Page):
    def __init__(self, host):
        from .form import FlowGrid
        from . import buttons
        super().__init__("gn.titlebar")
        self.host = host
        card = Card()
        self.body.addWidget(card)
        self.preview = TitlebarPreview()
        card.lay.addWidget(self.preview)
        card.lay.addSpacing(6)
        self.sw_min = Switch()
        self.sw_max = Switch()
        self.sw_close = Switch()
        self.r_min = Row("gn.btn.min", self.sw_min)
        self.r_max = Row("gn.btn.max", self.sw_max)
        self.r_close = Row("gn.btn.close", self.sw_close)
        side = QtWidgets.QWidget()
        hl = QtWidgets.QHBoxLayout(side)
        hl.setContentsMargins(0, 0, 0, 0)
        self.b_left = QtWidgets.QPushButton()
        self.b_right = QtWidgets.QPushButton()
        grp = QtWidgets.QButtonGroup(self)
        for b in (self.b_left, self.b_right):
            b.setObjectName("Lang")
            b.setCheckable(True)
            b.setCursor(Qt.CursorShape.PointingHandCursor)
            grp.addButton(b)
            hl.addWidget(b)
        self.r_side = Row("gn.btn.side", side)
        for r in (self.r_min, self.r_max, self.r_close, self.r_side):
            card.lay.addWidget(r)
        self.warn = QtWidgets.QLabel()
        self.warn.setStyleSheet("color: #d29922; background: transparent;")
        self.warn.setWordWrap(True)
        card.lay.addWidget(self.warn)
        for sw in (self.sw_min, self.sw_max, self.sw_close):
            sw.toggled.connect(lambda _v: self._write())
        self.b_left.clicked.connect(lambda: self._write())
        self.b_right.clicked.connect(lambda: self._write())
        # Knopf-Themes
        head = QtWidgets.QHBoxLayout()
        self.l_themes = QtWidgets.QLabel()
        self.l_themes.setObjectName("CardTitle")
        self.b_orig = tool_button("gn.login.restore")
        self.b_orig.clicked.connect(self.remove_buttons)
        head.addWidget(self.l_themes)
        head.addStretch()
        self.b_files = tool_button("gn.files.restart")
        self.b_files.clicked.connect(restart_files)
        head.addWidget(self.b_files)
        head.addWidget(self.b_orig)
        self.body.addSpacing(6)
        self.body.addLayout(head)
        from . import buttons as _b
        from .widgets import ChipRow
        self.support = ChipRow()
        self.body.addWidget(self.support)
        self.sw_moz = Switch()
        self.sw_moz.set_now(bool(host.cfg.get("moz_buttons", True)) and bool(_b.moz_profiles()))
        self.sw_moz.setEnabled(bool(_b.moz_profiles()))
        self.sw_moz.toggled.connect(self._moz_toggle)
        self.r_moz = Row("gn.btns.moz", self.sw_moz)
        self.body.addWidget(self.r_moz)

        self.grid = FlowGrid(262)
        self.tiles = []
        for name in buttons.THEMES:
            tile = ButtonTile(name, self)
            self.tiles.append(tile)
            self.grid.add(tile)
        self.body.addWidget(self.grid)
        self.finish()
        self._appmenu = True

    def _state(self):
        return {"side": "left" if self.b_left.isChecked() else "right", "minimize": self.sw_min.isChecked(),
                "maximize": self.sw_max.isChecked(), "close": self.sw_close.isChecked()}

    def _write(self):
        d = self._state()
        self.preview.set_layout(d)
        value = gnome.build_layout(d["side"], d["minimize"], d["maximize"], self._appmenu, d["close"])
        usercmd.gset(gnome.WM, "button-layout", value)
        self.warn.setVisible(not d["close"])
        self.host.set_status(t("done"))
        self._tiles()

    def apply_buttons(self, name):
        from . import buttons
        self.host.log("")
        buttons.apply(name)
        if self.sw_moz.isChecked():
            buttons.apply_moz(name)
        self.host.set_status(t("gn.btns.restart"))
        self._tiles()

    def _css_files(self):
        from . import buttons
        files = list(buttons.files())
        for prof in buttons.moz_profiles():
            files += list(buttons._moz_targets(prof))
        return files

    def reset_keys(self):
        return [(gnome.WM, "button-layout")]

    def reset_extra(self, bak):
        from . import buttons
        from .form import backup_files
        backup_files(self._css_files(), bak)
        buttons.remove()
        buttons.remove_moz()

    def undo_extra(self, bak):
        from .form import restore_files
        restore_files(self._css_files(), bak)

    def _moz_toggle(self, on):
        from . import buttons
        self.host.cfg["moz_buttons"] = bool(on)
        self.host.save()
        cur = buttons.current()
        if on and cur:
            buttons.apply_moz(cur)
        elif not on:
            buttons.remove_moz()
        self.host.set_status(t("gn.btns.restart"))

    def remove_buttons(self):
        from . import buttons
        self.host.log("")
        buttons.remove()
        buttons.remove_moz()
        self.host.set_status(t("gn.btns.restart"))
        self._tiles()

    def _tiles(self):
        from . import buttons
        cur = buttons.current()
        self.preview.set_style(cur or None)
        for tl in self.tiles:
            tl.refresh(self._state(), cur, self.host.accent())
        self.b_orig.setEnabled(bool(cur))

    def retranslate(self):
        super().retranslate()
        for r in (self.r_min, self.r_max, self.r_close, self.r_side):
            r.retranslate()
        self.b_left.setText(t("gn.left"))
        self.b_right.setText(t("gn.right"))
        self.warn.setText(t("gn.btn.noclose"))
        self.l_themes.setText(t("gn.btns"))
        self.l_themes.setToolTip(t("gn.btns.note"))
        self.r_moz.retranslate()
        self.support.set([(t("gn.sup.gtk"), "ChipOk", ""), (t("gn.sup.moz"), "ChipOk", t("gn.sup.moz.tip")),
                          (t("gn.sup.chromium"), "ChipWarn", t("gn.sup.chromium.tip")),
                          (t("gn.sup.electron"), "Chip", t("gn.sup.electron.tip")),
                          (t("gn.sup.qt"), "Chip", t("gn.sup.qt.tip"))])
        retranslate_buttons(self)
        self.preview.update()

    def refresh(self):
        d = gnome.parse_layout(usercmd.gget(gnome.WM, "button-layout", "appmenu:close") or "appmenu:close")
        self._appmenu = d["appmenu"]
        self.sw_min.set_now(d["minimize"])
        self.sw_max.set_now(d["maximize"])
        self.sw_close.set_now(d["close"])
        (self.b_left if d["side"] == "left" else self.b_right).setChecked(True)
        self.preview.set_layout(d)
        self.warn.setVisible(not d["close"])
        self._tiles()


# ---------- Obere Leiste ----------

from .widgets import Segmented  # noqa: E402


class TopbarPage(ExtInstall, Page):
    def __init__(self, host):
        super().__init__("gn.topbar")
        self.host = host
        specs = gnome.SECTIONS["topbar"]
        clock = Card("gn.top.clock")
        self.body.addWidget(clock)
        gsection(self, clock, specs[:5])
        bar = Card("gn.top.bar")
        self.body.addWidget(bar)
        gsection(self, bar, specs[5:7])
        self.sw_ws = Switch()
        self.ws_chip = QtWidgets.QLabel()
        w = QtWidgets.QWidget()
        hl = QtWidgets.QHBoxLayout(w)
        hl.setContentsMargins(0, 0, 0, 0)
        hl.setSpacing(8)
        self.ws_get = MissingButton()
        self.ws_get.clicked.connect(lambda: self.install_ext(gnome.WS_INDICATOR))
        self.ws_rm = RemoveButton()
        self.ws_rm.clicked.connect(lambda: self.uninstall_ext(gnome.WS_INDICATOR))
        hl.addWidget(self.ws_chip)
        hl.addWidget(self.ws_get)
        hl.addWidget(self.ws_rm)
        hl.addWidget(self.sw_ws)
        self.r_ws = Row("gn.top.wsind", w)
        bar.lay.addWidget(self.r_ws)
        self.sw_ws.toggled.connect(self._ws)
        ws = Card("gn.top.ws")
        self.body.addWidget(ws)
        gsection(self, ws, specs[7:])
        ed = Card("bar.editor")
        self.body.addWidget(ed)
        self.editor = BarEditor(self)
        ed.lay.addWidget(self.editor)
        pw = Card("pw.menu")
        self.body.addWidget(pw)
        self.power = []
        for key, schema, gk, inverted in gnome.POWER_ITEMS:
            sw = Switch()
            sw.toggled.connect(lambda on, s=schema, k=gk, inv=inverted: (usercmd.gset(s, k, (not on) if inv else bool(on)),
                                                                          self.host.set_status(t("done"))))
            r = Row(key, sw)
            pw.lay.addWidget(r)
            self.power.append((sw, r, schema, gk, inverted))
        self.finish()

    def reset_keys(self):
        return super().reset_keys() + [(s, k) for _n, s, k, _i in gnome.POWER_ITEMS]

    def _jp_keys(self):
        return [k for _n, k, _z in gnome.BAR_ITEMS] + [k for _n, k in gnome.QUICK_ITEMS] + \
            ["clock-menu-position", "top-panel-position"]

    def reset_extra(self, bak):
        import json
        if not gnome.jp_ready():
            return
        vals = {k: gnome.jp_get(k) for k in self._jp_keys()}
        (bak / "just-perfection.json").write_text(json.dumps(vals), encoding="utf-8")
        for k in self._jp_keys():
            usercmd.run(gnome._jp_cmd() + ["reset", gnome.JP_SCHEMA, k])

    def undo_extra(self, bak):
        import json
        try:
            vals = json.loads((bak / "just-perfection.json").read_text(encoding="utf-8"))
        except (OSError, ValueError):
            return
        for k, v in vals.items():
            if v is not None:
                gnome.jp_set(k, v)

    def _ws(self, on):
        gnome.ext_set(gnome.WS_INDICATOR, bool(on))
        self.host.set_status(t("done"))

    def retranslate(self):
        super().retranslate()
        self.r_ws.retranslate()
        self.editor.retranslate()
        for sw, r, *_ in self.power:
            r.retranslate()

    def refresh(self):
        super().refresh()
        have = gnome.ext_installed(gnome.WS_INDICATOR)
        on = gnome.WS_INDICATOR in gnome.ext_enabled()
        self.sw_ws.setEnabled(have)
        self.sw_ws.set_now(on)
        set_chip(self.ws_chip, "Chip" if have else "ChipWarn", t("gn.ext.name") if have else t("gn.ext.missing"))
        self.ws_chip.setToolTip(t("gn.top.wsind.tip"))
        self.ws_get.setVisible(not have)
        self.ws_rm.setVisible(have and gnome.ext_removable(gnome.WS_INDICATOR))
        self.editor.refresh()
        for sw, r, schema, gk, inverted in self.power:
            v = bool(usercmd.gget(schema, gk, False))
            sw.set_now((not v) if inverted else v)
        self.ws_get.setToolTip(t("miss.install") + ": Workspace Indicator")


class BarPreview(QtWidgets.QWidget):
    """Top bar as a picture: click an element to show or hide it (Just Perfection)."""

    def __init__(self, parent=None):
        super().__init__(parent)
        self.state = {}
        self.clock_pos = 0
        self.rects = {}
        self.setFixedHeight(46)
        self.setCursor(Qt.CursorShape.PointingHandCursor)
        self.on_click = None

    def set_state(self, state: dict, clock_pos: int):
        self.state, self.clock_pos = state, clock_pos
        self.update()

    def paintEvent(self, _):
        p = QtGui.QPainter(self)
        p.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
        r = QtCore.QRectF(self.rect()).adjusted(1, 1, -1, -1)
        p.setPen(Qt.PenStyle.NoPen)
        p.setBrush(QtGui.QColor("#000000"))
        p.drawRoundedRect(r, 10, 10)
        zones = {"left": [], "center": [], "right": []}
        for key, _k, zone in gnome.BAR_ITEMS:
            z = zone
            if key == "clock":
                z = {0: "center", 1: "right", 2: "left"}.get(self.clock_pos, "center")
            zones[z].append(key)
        f = p.font()
        avail = r.width() - 40
        for pt in (9.0, 8.5, 8.0, 7.5, 7.0, 6.5, 6.0):    # kleiner, bis alles in die Leiste passt
            f.setPointSizeF(pt)
            p.setFont(f)
            fm = p.fontMetrics()                            # Maße des Painters (wie gezeichnet wird)
            need = sum(fm.horizontalAdvance(t("bar." + k)) + 18 + 6 for z in zones.values() for k in z) + 60
            if need <= avail:
                break
        self.rects = {}

        def chip(key, x):
            text = t("bar." + key)
            w = fm.horizontalAdvance(text) + 18
            rr = QtCore.QRectF(x, r.top() + 8, w, r.height() - 16)
            on = self.state.get(key, True)
            p.setBrush(QtGui.QColor("#2b3747" if on else "#151c26"))
            p.setPen(QtGui.QPen(QtGui.QColor("#3a4a5f" if on else "#253142"), 1, Qt.PenStyle.SolidLine if on else Qt.PenStyle.DashLine))
            p.drawRoundedRect(rr, 7, 7)
            p.setPen(QtGui.QColor("#e6edf5" if on else "#4d5a6b"))
            p.drawText(rr, Qt.AlignmentFlag.AlignCenter, text)
            self.rects[key] = rr
            return w
        x = r.left() + 10
        for key in zones["left"]:
            x += chip(key, x) + 6
        rw = [fm.horizontalAdvance(t("bar." + k)) + 18 for k in zones["right"]]
        rx = r.right() - 10 - sum(rw) - 6 * max(0, len(rw) - 1)
        cw = [fm.horizontalAdvance(t("bar." + k)) + 18 for k in zones["center"]]
        ctotal = sum(cw) + 6 * max(0, len(cw) - 1)
        cx = min(r.center().x() - ctotal / 2, rx - 12 - ctotal)     # Mitte, aber nie unter die rechte Seite
        cx = max(cx, x + 6)
        for key in zones["center"]:
            cx += chip(key, cx) + 6
        for key in zones["right"]:
            rx += chip(key, rx) + 6
        p.end()

    def mouseReleaseEvent(self, e):
        for key, rr in self.rects.items():
            if rr.contains(e.position()) and self.on_click:
                self.on_click(key)
                return


class BarEditor(QtWidgets.QWidget):
    """Card content: bar preview, clock position, bar position, quick settings, power menu."""

    def __init__(self, page):
        super().__init__()
        self.page = page
        lay = QtWidgets.QVBoxLayout(self)
        lay.setContentsMargins(0, 0, 0, 0)
        lay.setSpacing(4)
        head = QtWidgets.QHBoxLayout()
        self.chip = QtWidgets.QLabel()
        self.get = MissingButton()
        self.get.clicked.connect(lambda: page.install_ext(gnome.JP_UUID))
        self.rm = RemoveButton()
        self.rm.clicked.connect(lambda: page.uninstall_ext(gnome.JP_UUID))
        head.addWidget(self.chip)
        head.addWidget(self.get)
        head.addWidget(self.rm)
        head.addStretch()
        lay.addLayout(head)
        self.bar = BarPreview()
        self.bar.on_click = self.toggle
        lay.addWidget(self.bar)
        self.clock = Segmented(("2", "0", "1"), prefix="bar.pos.")
        self.clock.changed.connect(lambda v: self._set("clock-menu-position", int(v)))
        self.r_clock = Row("bar.clockpos", self.clock)
        lay.addWidget(self.r_clock)
        self.place = Segmented(("0", "1"), prefix="bar.place.")
        self.place.changed.connect(lambda v: self._set("top-panel-position", int(v)))
        self.r_place = Row("bar.place", self.place)
        lay.addWidget(self.r_place)
        self.qs = {}
        for key, jk in gnome.QUICK_ITEMS:
            sw = Switch()
            sw.toggled.connect(lambda on, k=jk: self._set(k, bool(on)))
            r = Row(key, sw)
            self.qs[jk] = (sw, r)
            lay.addWidget(r)

    def toggle(self, key):
        jk = next(k for name, k, _z in gnome.BAR_ITEMS if name == key)
        cur = bool(gnome.jp_get(jk, True))
        self._set(jk, not cur)

    def _set(self, key, value):
        if not gnome.jp_ready():
            return
        gnome.jp_set(key, value)
        self.page.host.set_status(t("done"))
        self.refresh()

    def retranslate(self):
        for r in (self.r_clock, self.r_place):
            r.retranslate()
        self.clock.retranslate()
        self.place.retranslate()
        for sw, r in self.qs.values():
            r.retranslate()
        self.bar.setToolTip(t("bar.tip"))

    def refresh(self):
        have = gnome.ext_installed(gnome.JP_UUID)
        ready = gnome.jp_ready()
        set_chip(self.chip, "ChipOk" if ready else "ChipWarn",
                 "Just Perfection" if ready else (t("gn.ext.off") if have else t("gn.ext.missing")))
        self.chip.setToolTip(t("bar.needs"))
        self.get.setVisible(not have)
        self.rm.setVisible(have and gnome.ext_removable(gnome.JP_UUID))
        state = {name: bool(gnome.jp_get(k, True)) for name, k, _z in gnome.BAR_ITEMS} if ready else {}
        self.bar.set_state(state, int(gnome.jp_get("clock-menu-position", 0) or 0) if ready else 0)
        self.clock.select(str(int(gnome.jp_get("clock-menu-position", 0) or 0)) if ready else "0")
        self.place.select(str(int(gnome.jp_get("top-panel-position", 0) or 0)) if ready else "0")
        for jk, (sw, r) in self.qs.items():
            sw.set_now(bool(gnome.jp_get(jk, True)) if ready else True)
        for w in [self.bar, self.clock, self.place] + [sw for sw, _r in self.qs.values()]:
            w.setEnabled(ready)


# ---------- Startprogramme ----------

from hyprstudio.dialogs import Frame as _Frame


class AppPicker(_Frame):
    def __init__(self, parent, accent):
        super().__init__(parent, t("gn.auto.add"), 460)
        self.resize(460, 560)
        lay = self.body
        self.search = QtWidgets.QLineEdit()
        self.search.setObjectName("Search")
        self.search.setPlaceholderText(t("search"))
        self.search.addAction(draw_icon("search", theme.MUTED, 16), QtWidgets.QLineEdit.ActionPosition.LeadingPosition)
        lay.addWidget(self.search)
        self.list = QtWidgets.QListWidget()
        self.list.setObjectName("NavList")
        self.list.setIconSize(QtCore.QSize(24, 24))
        lay.addWidget(self.list, 1)
        row = QtWidgets.QHBoxLayout()
        row.addStretch()
        cancel = QtWidgets.QPushButton(t("cancel"))
        ok = QtWidgets.QPushButton(t("gn.auto.add"))
        ok.setObjectName("Primary")
        cancel.clicked.connect(self.reject)
        ok.clicked.connect(self.accept)
        row.addWidget(cancel)
        row.addWidget(ok)
        lay.addLayout(row)
        for a in gnome.list_apps():
            it = QtWidgets.QListWidgetItem(QtGui.QIcon.fromTheme(a["icon"]), a["name"])
            it.setData(Qt.ItemDataRole.UserRole, str(a["file"]))
            self.list.addItem(it)
        self.search.textChanged.connect(self._filter)
        self.list.itemDoubleClicked.connect(lambda _i: self.accept())

    def _filter(self, q):
        q = q.strip().lower()
        for i in range(self.list.count()):
            it = self.list.item(i)
            it.setHidden(bool(q) and q not in it.text().lower())

    def chosen(self) -> Path | None:
        it = self.list.currentItem()
        return Path(it.data(Qt.ItemDataRole.UserRole)) if it else None


class AutostartPage(Page):
    def __init__(self, host):
        super().__init__("gn.autostart")
        self.host = host
        self.b_add = tool_button("gn.auto.add", primary=True)
        self.b_add.clicked.connect(self.add)
        self.tools.addWidget(self.b_add)
        self.card = Card()
        self.body.addWidget(self.card)
        self.empty = QtWidgets.QLabel()
        self.empty.setObjectName("Label")
        self.card.lay.addWidget(self.empty)
        self.rows = []
        self.finish()

    def add(self):
        dlg = AppPicker(self, self.host.accent())
        if dlg.exec() == QtWidgets.QDialog.DialogCode.Accepted and dlg.chosen():
            self.host.log("")
            gnome.autostart_add(dlg.chosen())
            self.refresh()
            self.host.set_status(t("done"))

    def _toggle(self, path, on):
        gnome.autostart_enable(path, bool(on))
        self.host.set_status(t("done"))

    def retranslate(self):
        super().retranslate()
        retranslate_buttons(self)
        self.empty.setText(t("gn.auto.none"))

    def refresh(self):
        for r in self.rows:
            r.hide()
            r.setParent(None)
            r.deleteLater()
        self.rows = []
        entries = gnome.list_autostart()
        self.empty.setVisible(not entries)
        for e in entries:
            w = QtWidgets.QWidget()
            hl = QtWidgets.QHBoxLayout(w)
            hl.setContentsMargins(0, 6, 0, 6)
            ic = QtWidgets.QLabel()
            ic.setPixmap(QtGui.QIcon.fromTheme(e["icon"] or "application-x-executable").pixmap(24, 24))
            name = QtWidgets.QLabel(e["name"])
            name.setToolTip(str(e["file"]))
            sw = Switch()
            sw.accent = self.host.accent()
            sw.set_now(e["enabled"])
            sw.toggled.connect(lambda on, p=e["file"]: self._toggle(p, on))
            hl.addWidget(ic)
            hl.addWidget(name, 1)
            hl.addWidget(sw)
            self.card.lay.addWidget(w)
            self.rows.append(w)


# ---------- Anmeldebildschirm (GDM) ----------

class ImageBox(QtWidgets.QLabel):
    def __init__(self, w, h, parent=None):
        super().__init__(parent)
        self.setFixedSize(w, h)
        self.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.setStyleSheet(f"background:{theme.SURFACE2}; border:1px solid {theme.BORDER}; border-radius:10px;"
                           f" color:{theme.MUTED};")

    def show_file(self, path: str | None, color: str | None = None):
        if color:
            self.setPixmap(QtGui.QPixmap())
            self.setText("")
            self.setStyleSheet(f"background:{color}; border:1px solid {theme.BORDER}; border-radius:10px;")
            return
        self.setStyleSheet(f"background:{theme.SURFACE2}; border:1px solid {theme.BORDER}; border-radius:10px;"
                           f" color:{theme.MUTED};")
        pm = QtGui.QPixmap(path) if path else QtGui.QPixmap()
        if pm.isNull():
            self.setPixmap(QtGui.QPixmap())
            self.setText("–")
            return
        self.setPixmap(pm.scaled(self.width() - 8, self.height() - 8, Qt.AspectRatioMode.KeepAspectRatio,
                                 Qt.TransformationMode.SmoothTransformation))


def stage_image(src: str, name: str) -> Path | None:
    """Bild für den Root-Helfer bereitlegen: als PNG nach ~/.cache (root darf nicht in die Cloud lesen)."""
    img = QtGui.QImage(src)
    if img.isNull():
        return None
    if img.width() > 3840:
        img = img.scaledToWidth(3840, Qt.TransformationMode.SmoothTransformation)
    d = CACHE / "gdm"
    d.mkdir(parents=True, exist_ok=True)
    out = d / f"{name}.png"
    return out if img.save(str(out), "PNG") else None


class LoginPage(Page):
    has_reset = True

    def __init__(self, host):
        super().__init__("gn.login")
        self.host = host
        self.state_chip = QtWidgets.QLabel()
        self.chips.addWidget(self.state_chip)
        # Hintergrund
        bg = Card("gn.login.bg")
        self.body.addWidget(bg)
        row = QtWidgets.QHBoxLayout()
        row.setSpacing(16)
        self.bg_box = ImageBox(256, 144)
        row.addWidget(self.bg_box)
        col = QtWidgets.QVBoxLayout()
        self.b_bg = tool_button("gn.login.pick", primary=True)
        self.b_color = tool_button("gn.login.color")
        self.b_restore = tool_button("gn.login.restore")
        self.b_bg.clicked.connect(self.pick_bg)
        self.b_color.clicked.connect(self.pick_color)
        self.b_restore.clicked.connect(lambda: self.host.run(["gdm-restore"], "gdm-restore", self))
        for b in (self.b_bg, self.b_color, self.b_restore):
            col.addWidget(b)
        col.addStretch()
        row.addLayout(col)
        row.addStretch()
        bg.lay.addLayout(row)
        # Logo
        logo = Card("gn.login.logo")
        self.body.addWidget(logo)
        lrow = QtWidgets.QHBoxLayout()
        lrow.setSpacing(16)
        self.logo_box = ImageBox(144, 96)
        lrow.addWidget(self.logo_box)
        lcol = QtWidgets.QVBoxLayout()
        self.b_logo = tool_button("gn.login.pick", primary=True)
        self.b_logo_clear = tool_button("remove")
        self.b_logo.clicked.connect(self.pick_logo)
        self.b_logo_clear.clicked.connect(lambda: self.host.run(["gdm-logo", "--clear"], "gdm-logo", self))
        lcol.addWidget(self.b_logo)
        lcol.addWidget(self.b_logo_clear)
        lcol.addStretch()
        lrow.addLayout(lcol)
        lrow.addStretch()
        logo.lay.addLayout(lrow)
        # Text + Benutzerliste
        more = Card("gn.login.more")
        self.body.addWidget(more)
        bw = QtWidgets.QWidget()
        bl = QtWidgets.QHBoxLayout(bw)
        bl.setContentsMargins(0, 0, 0, 0)
        self.banner = QtWidgets.QLineEdit()
        self.banner.setObjectName("Search")
        self.banner.setMinimumWidth(280)
        self.b_banner = tool_button("apply")
        self.b_banner.clicked.connect(self.set_banner)
        bl.addWidget(self.banner)
        bl.addWidget(self.b_banner)
        self.r_banner = Row("gn.login.banner", bw)
        more.lay.addWidget(self.r_banner)
        self.sw_users = Switch()
        self.sw_users.toggled.connect(lambda on: self.host.run(["gdm-userlist", "on" if on else "off"],
                                                               "gdm-userlist", self))
        self.r_users = Row("gn.login.users", self.sw_users)
        more.lay.addWidget(self.r_users)
        self.finish()

    def reset_extra(self, bak):
        self.host.run(["gdm-restore"], "gdm-restore", self)     # Systemstandard; Sicherung bleibt im Helfer

    def pick_file(self) -> str:
        from hyprstudio import dialogs
        return dialogs.open_file(self, t("gn.login.pick"), str(Path.home()), "Bilder (*.png *.jpg *.jpeg *.webp *.svg)")

    def pick_bg(self):
        f = self.pick_file()
        out = stage_image(f, "background") if f else None
        if out:
            self.host.run(["gdm-bg", str(out)], "gdm-bg", self)

    def pick_color(self):
        from hyprstudio import dialogs
        c = dialogs.pick_color(self, QtGui.QColor("#1b2430"), t("gn.login.color"))
        if c is not None and c.isValid():
            self.host.run(["gdm-bg", c.name()], "gdm-bg", self)

    def pick_logo(self):
        f = self.pick_file()
        out = stage_image(f, "logo") if f else None
        if out:
            self.host.run(["gdm-logo", str(out)], "gdm-logo", self)

    def set_banner(self):
        text = self.banner.text().strip()
        self.host.run(["gdm-banner", text or "--clear"], "gdm-banner", self)

    def finished(self, ok, action, result):
        self.refresh()

    def retranslate(self):
        super().retranslate()
        retranslate_buttons(self)
        self.r_banner.retranslate()
        self.r_users.retranslate()
        self.banner.setPlaceholderText(t("gn.login.banner.ph"))

    def refresh(self):
        have = gnome.gdm_installed()
        st = gnome.gdm_state()
        set_chip(self.state_chip, "ChipOk" if have else "ChipWarn", "GDM" if have else t("gn.login.nogdm"))
        bgv = st.get("background", "")
        if bgv.startswith("#"):
            self.bg_box.show_file(None, bgv)
        else:
            self.bg_box.show_file(str(CACHE / "gdm" / "background.png") if bgv else None)
        self.logo_box.show_file(str(CACHE / "gdm" / "logo.png") if st.get("logo") else None)
        self.banner.setText(st.get("banner", ""))
        self.sw_users.set_now(st.get("userlist", True))
        busy = self.host.busy()
        for b in (self.b_bg, self.b_color, self.b_restore, self.b_logo, self.b_logo_clear, self.b_banner):
            b.setEnabled(have and not busy)
        self.sw_users.setEnabled(have and not busy)
        self.b_restore.setToolTip(t("gn.login.restore.tip"))
        self.b_bg.setToolTip(t("gn.login.bg.tip"))


__all__ = ["simple_page", "LookGnome", "TitlebarPage", "TopbarPage", "AutostartPage", "LoginPage"]
