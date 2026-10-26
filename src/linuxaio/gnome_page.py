"""GNOME-Bereiche im Tab „Einstellungen“: wie GNOME Tweaks plus Titelleisten-Knöpfe, obere Leiste,
Startprogramme und Anmeldebildschirm (GDM)."""

from pathlib import Path

from hyprstudio.widgets import Switch

from . import gnome, installers, theme, usercmd
from .form import Card, MissingButton, Page, Row, combo, gsection, retranslate_buttons, set_chip, start_job, tool_button
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
                host.set_status(t("miss.relogin" if res == "relogin" else "miss.done", name=name))
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
        self.shell = combo()
        self.shell.textActivated.connect(lambda n: self._set_shell(n))
        sl.addWidget(self.shell_chip)
        sl.addWidget(self.shell_get)
        sl.addWidget(self.shell)
        self.r_shell = Row("gn.shell", shell_w)
        themes.lay.addWidget(self.r_shell)
        self.finish()

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
        from hyprstudio.widgets import FlowGrid
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
        head.addWidget(self.b_orig)
        self.body.addSpacing(6)
        self.body.addLayout(head)
        self.note = QtWidgets.QLabel()
        self.note.setObjectName("Label")
        self.body.addWidget(self.note)
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
        self.host.set_status(t("gn.btns.restart"))
        self._tiles()

    def remove_buttons(self):
        from . import buttons
        self.host.log("")
        buttons.remove()
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
        self.note.setText(t("gn.btns.note"))
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
        hl.addWidget(self.ws_chip)
        hl.addWidget(self.ws_get)
        hl.addWidget(self.sw_ws)
        self.r_ws = Row("gn.top.wsind", w)
        bar.lay.addWidget(self.r_ws)
        self.sw_ws.toggled.connect(self._ws)
        ws = Card("gn.top.ws")
        self.body.addWidget(ws)
        gsection(self, ws, specs[7:])
        self.finish()

    def _ws(self, on):
        gnome.ext_set(gnome.WS_INDICATOR, bool(on))
        self.host.set_status(t("done"))

    def retranslate(self):
        super().retranslate()
        self.r_ws.retranslate()

    def refresh(self):
        super().refresh()
        have = gnome.ext_installed(gnome.WS_INDICATOR)
        on = gnome.WS_INDICATOR in gnome.ext_enabled()
        self.sw_ws.setEnabled(have)
        self.sw_ws.set_now(on)
        set_chip(self.ws_chip, "Chip" if have else "ChipWarn", t("gn.ext.name") if have else t("gn.ext.missing"))
        self.ws_chip.setToolTip(t("gn.top.wsind.tip"))
        self.ws_get.setVisible(not have)
        self.ws_get.setToolTip(t("miss.install") + ": Workspace Indicator")


# ---------- Startprogramme ----------

class AppPicker(QtWidgets.QDialog):
    def __init__(self, parent, accent):
        super().__init__(parent)
        self.setWindowTitle(t("gn.auto.add"))
        self.setStyleSheet(theme.qss(accent) + f"QDialog {{ background: {theme.SURFACE}; }}")
        self.resize(460, 560)
        lay = QtWidgets.QVBoxLayout(self)
        lay.setContentsMargins(16, 16, 16, 16)
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

    def pick_file(self) -> str:
        f, _ = QtWidgets.QFileDialog.getOpenFileName(self, t("gn.login.pick"), str(Path.home()),
                                                     "Bilder (*.png *.jpg *.jpeg *.webp *.svg)", "",
                                                     QtWidgets.QFileDialog.Option.DontUseNativeDialog)
        return f

    def pick_bg(self):
        f = self.pick_file()
        out = stage_image(f, "background") if f else None
        if out:
            self.host.run(["gdm-bg", str(out)], "gdm-bg", self)

    def pick_color(self):
        c = QtWidgets.QColorDialog.getColor(QtGui.QColor("#1b2430"), self, t("gn.login.color"),
                                            QtWidgets.QColorDialog.ColorDialogOption.DontUseNativeDialog)
        if c.isValid():
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
