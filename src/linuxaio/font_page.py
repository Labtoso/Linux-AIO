"""GNOME › Schriften: Schrift-Einstellungen wie GNOME Tweaks + Schriften installieren (Google Fonts, Nerd Fonts)
mit Live-Vorschau in der echten Schrift; danach direkt als Oberflächen-/Dokument-/Monospace-Schrift anwendbar."""

from . import fonts, gnome, theme, usercmd
from .form import Card, FlowGrid, Page, gsection, retranslate_buttons, set_chip, start_job, tool_button
from .i18n import t
from .qt import QtCore, QtGui, QtWidgets, Qt
from .widgets import Segmented, draw_icon, set_applied

TILE_W, TILE_H = 262, 200
PAGE = 24
FILTERS = ("all", "Sans Serif", "Serif", "Monospace", "Display", "Handwriting", "Nerd Font")
SAMPLE = {"de": "Franz jagt im komplett verwahrlosten Taxi", "en": "The quick brown fox jumps over the lazy dog"}


class FontTile(QtWidgets.QFrame):
    def __init__(self, item: dict, page, parent=None):
        super().__init__(parent)
        self.item, self.page = item, page
        self.setObjectName("Tile")
        self.setFixedSize(TILE_W, TILE_H)
        self.setStyleSheet(f"#Tile {{ background:{theme.SURFACE}; border:1px solid {theme.BORDER}; border-radius:14px; }}")
        lay = QtWidgets.QVBoxLayout(self)
        lay.setContentsMargins(12, 10, 12, 10)
        lay.setSpacing(6)
        self.sample = QtWidgets.QLabel("Aa")
        self.sample.setWordWrap(True)
        self.sample.setFixedHeight(66)
        self.sample.setAlignment(Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignVCenter)
        self.base_css = f"background: {theme.SURFACE2}; border-radius: 8px; padding: 6px 8px;"
        self.sample.setStyleSheet(self.base_css)
        lay.addWidget(self.sample)
        name = QtWidgets.QLabel(QtGui.QFontMetrics(self.font()).elidedText(item["family"], Qt.TextElideMode.ElideRight, 236))
        name.setStyleSheet("font-weight:600; background:transparent;")
        name.setToolTip(item["family"])
        lay.addWidget(name)
        self.cat = QtWidgets.QLabel(t("font.cat." + item["category"]))
        self.cat.setObjectName("Label")
        lay.addWidget(self.cat)
        row = QtWidgets.QHBoxLayout()
        row.setSpacing(6)
        self.b_inst = QtWidgets.QPushButton()
        self.b_use = QtWidgets.QToolButton()
        self.b_use.setText(t("font.use"))
        self.b_use.setPopupMode(QtWidgets.QToolButton.ToolButtonPopupMode.InstantPopup)
        self.b_use.setStyleSheet(f"QToolButton {{ background:{theme.SURFACE2}; border:1px solid {theme.BORDER};"
                                 " border-radius:8px; padding:4px 10px; }")
        menu = QtWidgets.QMenu(self.b_use)
        for key in ("ui", "doc", "mono", "title"):
            menu.addAction(t("font." + key), lambda k=key: page.use_font(self.item, k))
        menu.addSeparator()
        self.act_remove = menu.addAction(t("remove"), lambda: page.remove_font(self.item))
        self.b_use.setMenu(menu)
        for b in (self.b_inst, self.b_use):
            b.setCursor(Qt.CursorShape.PointingHandCursor)
            b.setMinimumHeight(30)
        self.b_inst.setStyleSheet("padding: 4px 8px;")
        self.b_inst.clicked.connect(lambda: page.install_font(self.item))
        row.addWidget(self.b_inst, 1)
        row.addWidget(self.b_use)
        lay.addLayout(row)

    def set_font_family(self, family: str | None, text: str):
        # Schrift im eigenen Stylesheet: die App-weite Regel (font-size) würde setFont() überschreiben
        if family:
            fam = family.replace('"', "")
            self.sample.setStyleSheet(self.base_css + f' font-family: "{fam}"; font-size: 15pt;')
            self.sample.setText(text)
        else:
            self.sample.setText(t("font.nopreview"))
            self.sample.setStyleSheet(self.base_css + f" color: {theme.MUTED};")

    def refresh(self, busy: bool):
        inst = fonts.installed(self.item["family"]) is not None
        set_applied(self.b_inst, inst, t("installed"))
        if not inst:
            self.b_inst.setText(t("install"))
            self.b_inst.setEnabled(not busy)
        self.b_use.setEnabled(inst and not busy)
        self.act_remove.setEnabled(inst)


class FontsPage(Page):
    def __init__(self, host):
        super().__init__("gn.fonts")
        self.host = host
        card = Card()
        self.body.addWidget(card)
        gsection(self, card, gnome.SECTIONS["fonts"])
        # Schriften installieren
        store = Card("font.store")
        self.body.addWidget(store)
        bar = QtWidgets.QHBoxLayout()
        self.search = QtWidgets.QLineEdit()
        self.search.setObjectName("Search")
        self.search.setClearButtonEnabled(True)
        self.search.addAction(draw_icon("search", theme.MUTED, 16), QtWidgets.QLineEdit.ActionPosition.LeadingPosition)
        self.search.textChanged.connect(lambda _: self.timer.start())
        self.only_inst = QtWidgets.QCheckBox()
        self.only_inst.toggled.connect(lambda _: self.rebuild())
        bar.addWidget(self.search, 1)
        bar.addWidget(self.only_inst)
        store.lay.addLayout(bar)
        self.filter = Segmented(FILTERS, prefix="font.f.")
        self.filter.select("all")
        self.cur_filter = "all"
        self.filter.changed.connect(self._set_filter)
        fs = QtWidgets.QScrollArea()          # Filter-Leiste scrollt, wenn das Fenster schmal ist
        fs.setWidget(self.filter)
        fs.setWidgetResizable(False)
        fs.setFrameShape(QtWidgets.QFrame.Shape.NoFrame)
        fs.setVerticalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
        self.filter_scroll = fs
        fs.viewport().setObjectName("Viewport")
        fs.viewport().setStyleSheet("#Viewport { background: transparent; }")
        store.lay.addWidget(fs)
        self.msg = QtWidgets.QLabel()
        self.msg.setObjectName("Label")
        self.msg.setWordWrap(True)
        store.lay.addWidget(self.msg)
        self.grid = FlowGrid(TILE_W)
        store.lay.addWidget(self.grid)
        self.b_more = tool_button("font.more")
        self.b_more.clicked.connect(self.more)
        store.lay.addWidget(self.b_more, 0, Qt.AlignmentFlag.AlignHCenter)
        self.finish()
        self.timer = QtCore.QTimer(self)
        self.timer.setSingleShot(True)
        self.timer.setInterval(300)
        self.timer.timeout.connect(self.rebuild)
        self.items, self.shown, self.tiles = [], 0, {}
        self.families = {}          # Familie → geladener Qt-Name für die Vorschau
        self.busy = ""
        self._loaded = False

    # ----- Daten -----
    def showEvent(self, e):
        super().showEvent(e)
        if not self._loaded:
            self._loaded = True
            self.msg.setText(t("font.loading"))
            self.msg.show()
            start_job(self, fonts.catalog, self._catalog_done)

    def _catalog_done(self, ok, res):
        if not ok:
            self.msg.setText(t("remote.offline"))
            return
        items, err = res
        self.items = items
        self.msg.setVisible(bool(err))
        if err:
            self.msg.setText(t(err))
        self.rebuild()

    def _set_filter(self, key):
        self.cur_filter = key
        self.rebuild()

    def _visible(self):
        q = self.search.text().strip().lower()
        inst = fonts.list_installed()
        out = []
        for it in self.items:
            if self.cur_filter != "all" and it["category"] != self.cur_filter:
                continue
            if q and q not in it["family"].lower():
                continue
            if self.only_inst.isChecked() and fonts.safe_family(it["family"]) not in inst:
                continue
            out.append(it)
        return out

    def rebuild(self):
        for w in self.grid.items:
            w.hide()
            w.setParent(None)
            w.deleteLater()
        self.grid.items, self.tiles, self.shown = [], {}, 0
        self.more()

    def more(self):
        vis = self._visible()
        new = vis[self.shown:self.shown + PAGE]
        self.shown += len(new)
        for it in new:
            tile = FontTile(it, self)
            self.tiles[it["family"]] = tile
            self.grid.add(tile)
            if it["family"] in self.families:
                tile.set_font_family(self.families[it["family"]], SAMPLE.get(self.host.cfg.get("lang"), SAMPLE["en"]))
        self.b_more.setVisible(self.shown < len(vis))
        if self.items and not vis:
            self.msg.setText(t("font.none"))
            self.msg.show()
        elif self.items:
            self.msg.hide()
        self.refresh_tiles()
        todo = [it for it in new if it["family"] not in self.families]
        for i in range(0, len(todo), 6):
            part = todo[i:i + 6]
            start_job(self, lambda part=part: [(it["family"], fonts.preview_file(it)) for it in part], self._previews)

    def _previews(self, ok, res):
        if not ok:
            return
        sample = SAMPLE.get(self.host.cfg.get("lang"), SAMPLE["en"])
        for family, path in res:
            fam = None
            if path:
                fid = QtGui.QFontDatabase.addApplicationFont(str(path))
                fams = QtGui.QFontDatabase.applicationFontFamilies(fid) if fid >= 0 else []
                fam = fams[0] if fams else None
            self.families[family] = fam
            tile = self.tiles.get(family)
            if tile:
                tile.set_font_family(fam, sample)

    def refresh_tiles(self):
        for tile in self.tiles.values():
            tile.refresh(bool(self.busy) or self.host.busy())

    # ----- Aktionen -----
    def install_font(self, it):
        if self.busy:
            return
        self.busy = it["family"]
        self.host.log("")
        self.host.task_begin(t("look.installing", name=it["family"]))
        self.refresh_tiles()
        start_job(self, lambda: fonts.install(it), lambda ok, res, i=it: self._installed(ok, res, i))

    def _installed(self, ok, res, it):
        self.busy = ""
        self.host.task_end(True, "")
        if ok:
            self.host.set_status(t("look.installed", name=it["family"]))
            self.families.pop(it["family"], None)
            path = fonts.preview_file(it)
            self._previews(True, [(it["family"], path)])
        else:
            self.host.log(str(res))
            key = str(res).split(":", 1)[0]
            self.host.set_status(f"{t('failed')}: {t(key) if key.startswith(('remote.', 'fonts.')) else res}")
        self.refresh_tiles()

    def remove_font(self, it):
        from hyprstudio import dialogs
        if not dialogs.confirm(self, t("remove"), t("confirm.remove", name=it["family"])):
            return
        self.host.log("")
        fonts.remove(it["family"])
        self.host.set_status(t("done"))
        self.refresh_tiles()

    def use_font(self, it, key):
        schema, gkey = {"ui": (gnome.IFACE, "font-name"), "doc": (gnome.IFACE, "document-font-name"),
                        "mono": (gnome.IFACE, "monospace-font-name"), "title": (gnome.WM, "titlebar-font")}[key]
        cur = usercmd.gget(schema, gkey, "") or ""
        _, size = gnome.split_font(cur) if cur else ("", 11.0)
        family = self.families.get(it["family"]) or it["family"]
        usercmd.gset(schema, gkey, gnome.join_font(family, size))
        self.host.set_status(t("font.applied", name=family, where=t("font." + key)))
        super().refresh()

    def retranslate(self):
        super().retranslate()
        self.search.setPlaceholderText(t("font.search"))
        self.only_inst.setText(t("font.onlyinst"))
        self.filter.retranslate()
        self.filter.adjustSize()                       # Größe nach den Texten (vorher leer → gequetscht)
        self.filter_scroll.setFixedHeight(self.filter.sizeHint().height() + 10)
        retranslate_buttons(self)

    def refresh(self):
        super().refresh()
        self.refresh_tiles()


__all__ = ["FontsPage", "set_chip"]
