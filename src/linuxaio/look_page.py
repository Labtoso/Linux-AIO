"""Mauszeiger bzw. Symbole: Kacheln mit Vorschau, „Installieren“/„Installiert“ und „Anwenden“/„Angewendet“.
Quellordner einstellbar; installiert wird nach ~/.local/share/icons (ohne root)."""

from pathlib import Path



from . import looks, remote, theme, usercmd
from .form import FlowGrid, Page, combo, set_chip, start_job, tool_button, retranslate_buttons
from .i18n import t
from .qt import QtCore, QtGui, QtWidgets, Qt
from .widgets import Segmented, draw_icon, set_applied

CATEGORY = {"cursor": "cursors", "icon": "icons", "login": "login"}

TILE_W, TILE_H = 262, 206


def cursor_preview(theme_dir: Path) -> QtGui.QImage | None:
    """Vier Zeiger (Pfeil, Hand, Text, Warten) nebeneinander aus den Xcursor-Dateien."""
    imgs = []
    for name in looks.CURSOR_PREVIEW:
        f = looks.cursor_file(theme_dir, name)
        r = looks.read_xcursor(f, 48) if f else None
        if r:
            w, h, data = r
            img = QtGui.QImage(data, w, h, w * 4, QtGui.QImage.Format.Format_ARGB32_Premultiplied).copy()
            imgs.append(img)
    if not imgs:
        return None
    cell = 44
    out = QtGui.QImage(cell * len(imgs), cell, QtGui.QImage.Format.Format_ARGB32_Premultiplied)
    out.fill(0)
    p = QtGui.QPainter(out)
    p.setRenderHint(QtGui.QPainter.RenderHint.SmoothPixmapTransform)
    for i, img in enumerate(imgs):
        s = img.scaled(cell, cell, Qt.AspectRatioMode.KeepAspectRatio, Qt.TransformationMode.SmoothTransformation)
        p.drawImage(i * cell + (cell - s.width()) // 2, (cell - s.height()) // 2, s)
    p.end()
    return out


def icon_preview(theme_dir: Path) -> QtGui.QImage | None:
    from .qt import QtSvg
    files = [f for f in (looks.icon_file(theme_dir, n) for n in looks.ICON_PREVIEW) if f][:4]
    if not files:
        return None
    cell = 44
    out = QtGui.QImage(cell * len(files), cell, QtGui.QImage.Format.Format_ARGB32_Premultiplied)
    out.fill(0)
    p = QtGui.QPainter(out)
    p.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
    p.setRenderHint(QtGui.QPainter.RenderHint.SmoothPixmapTransform)
    for i, f in enumerate(files):
        r = QtCore.QRectF(i * cell + 4, 4, cell - 8, cell - 8)
        if f.suffix == ".svg":
            QtSvg.QSvgRenderer(str(f)).render(p, r)
        else:
            img = QtGui.QImage(str(f))
            if not img.isNull():
                p.drawImage(r, img)
    p.end()
    return out


class Preview(QtWidgets.QWidget):
    def __init__(self, parent=None):
        super().__init__(parent)
        self.img = None
        self.setFixedHeight(92)

    def set_image(self, img):
        self.img = img
        self.update()

    def paintEvent(self, _):
        p = QtGui.QPainter(self)
        p.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
        r = QtCore.QRectF(self.rect())
        path = QtGui.QPainterPath()
        path.addRoundedRect(r, 10, 10)
        p.fillPath(path, QtGui.QColor("#dfe5ec"))           # heller Grund: dunkle und helle Zeiger sichtbar
        if self.img is not None and not self.img.isNull():
            s = self.img.scaled(int(r.width() - 16), int(r.height() - 16), Qt.AspectRatioMode.KeepAspectRatio,
                                Qt.TransformationMode.SmoothTransformation)
            p.drawImage(int((r.width() - s.width()) / 2), int((r.height() - s.height()) / 2), s)
        else:
            p.setPen(QtGui.QColor("#7a8796"))
            p.drawText(r, Qt.AlignmentFlag.AlignCenter, "…")
        p.end()


class LookTile(QtWidgets.QFrame):
    def __init__(self, item: dict, page, parent=None):
        super().__init__(parent)
        self.item = item
        self.page = page
        self.setObjectName("Tile")
        self.setFixedSize(TILE_W, TILE_H)
        lay = QtWidgets.QVBoxLayout(self)
        lay.setContentsMargins(10, 10, 10, 10)
        lay.setSpacing(8)
        self.pre = Preview()
        lay.addWidget(self.pre)
        self.name = QtWidgets.QLabel()
        self.name.setStyleSheet("font-weight:600; background:transparent;")
        label = item["color"] or item["label"]
        self.name.setText(QtGui.QFontMetrics(self.name.font()).elidedText(label, Qt.TextElideMode.ElideRight, 238))
        self.name.setToolTip(item["name"])
        lay.addWidget(self.name)
        row = QtWidgets.QHBoxLayout()
        row.setSpacing(6)
        self.b_inst = QtWidgets.QPushButton()
        self.b_apply = QtWidgets.QPushButton()
        self.b_apply.setObjectName("Primary")
        for b in (self.b_inst, self.b_apply):
            b.setCursor(Qt.CursorShape.PointingHandCursor)
            b.setMinimumHeight(30)
            b.setStyleSheet("padding: 4px 8px;")
            row.addWidget(b)
        self.b_inst.clicked.connect(lambda: page.install(self.item))
        self.b_apply.clicked.connect(lambda: page.apply(self.item))
        lay.addLayout(row)

    def refresh(self, current: str, busy: bool, accent: str):
        it = self.item
        installed = it["installed"] is not None
        set_applied(self.b_inst, installed, t("installed"))
        if not installed:
            self.b_inst.setText(t("install"))
            self.b_inst.setEnabled(not busy and (it["src"] is not None or bool(it.get("remote"))))
        active = installed and it["name"] == current
        set_applied(self.b_apply, active, t("applied"))
        if not active:
            self.b_apply.setText(t("apply"))
            self.b_apply.setEnabled(installed and not busy)
        border = accent if active else theme.BORDER
        self.setStyleSheet(f"#Tile {{ background:{theme.SURFACE}; border:{2 if active else 1}px solid {border};"
                           f" border-radius:14px; }}")


class PackCard(QtWidgets.QFrame):
    """Gesamtpaket aus dem Release: nur „Alles installieren“."""

    def __init__(self, pack: dict, page, parent=None):
        super().__init__(parent)
        self.pack, self.page = pack, page
        self.setObjectName("Card")
        lay = QtWidgets.QHBoxLayout(self)
        lay.setContentsMargins(16, 10, 10, 10)
        col = QtWidgets.QVBoxLayout()
        self.title = QtWidgets.QLabel(pack.get("titel") or pack["name"])
        self.title.setStyleSheet("font-weight:600; background:transparent;")
        self.info = QtWidgets.QLabel()
        self.info.setObjectName("Label")
        col.addWidget(self.title)
        col.addWidget(self.info)
        lay.addLayout(col, 1)
        self.btn = QtWidgets.QPushButton()
        self.btn.setObjectName("Primary")
        self.btn.setCursor(Qt.CursorShape.PointingHandCursor)
        self.btn.setMinimumHeight(34)
        self.btn.clicked.connect(lambda: page.install_pack(self.pack))
        lay.addWidget(self.btn)

    def refresh(self, busy: bool):
        names = self.pack.get("enthaelt") or []
        have = sum(1 for n in names if looks.installed_path(n, self.page.kind))
        mb = int(self.pack.get("groesse") or 0) / (1024 * 1024)
        self.info.setText(t("remote.pack.info", n=len(names), have=have, mb=f"{mb:.0f}"))
        done = bool(names) and have == len(names)
        set_applied(self.btn, done, t("remote.pack.done"))
        if not done:
            self.btn.setText(t("remote.pack.install"))
            self.btn.setEnabled(not busy)


class LookPage(Page):
    """kind: cursor | icon"""

    def __init__(self, host, kind: str, de: str, parent=None):
        super().__init__("look." + kind, parent)
        self.host, self.kind, self.de = host, kind, de
        self.items = []
        self.tiles = {}
        self.busy_name = ""
        self.previews = {}
        self.dry_applied = ""
        key = f"{kind}_dir"
        default = looks.DEFAULT_CURSOR_DIR if kind == "cursor" else looks.DEFAULT_ICON_DIR
        self.folder = Path(host.cfg.get(key) or default)

        self.search = QtWidgets.QLineEdit()
        self.search.setObjectName("Search")
        self.search.setClearButtonEnabled(True)
        self.search.setFixedWidth(170)
        self.search.addAction(draw_icon("search", theme.MUTED, 16), QtWidgets.QLineEdit.ActionPosition.LeadingPosition)
        self.search.textChanged.connect(lambda _: self._filter())
        self.source = host.cfg.get(f"{kind}_source", "online")
        if self.source not in ("online", "folder"):
            self.source = "online"
        self.src_seg = Segmented(("online", "folder"), prefix="remote.src.")
        self.src_seg.select(self.source)
        self.src_seg.changed.connect(self.set_source)
        self.tools.addWidget(self.src_seg)
        self.tools.addWidget(self.search)
        self.repo = host.cfg.get("remote_repo") or remote.DEFAULT_REPO
        self.manifest = None
        self.pack_cards = []
        if kind == "cursor":
            self.size = combo()
            self.size.setMinimumWidth(90)
            for s in (24, 32, 40, 48, 64):
                self.size.addItem(f"{s} px", s)
            self.size.setCurrentIndex(max(0, self.size.findData(int(host.cfg.get("cursor_size", 24)))))
            self.size.currentIndexChanged.connect(self._size_changed)
            self.tools.addWidget(self.size)

        bar = QtWidgets.QFrame()
        bar.setObjectName("DmBar")
        bl = QtWidgets.QHBoxLayout(bar)
        bl.setContentsMargins(14, 8, 10, 8)
        bl.setSpacing(10)
        self.l_folder = QtWidgets.QLabel()
        self.l_folder.setObjectName("Label")
        self.path = QtWidgets.QLabel()
        self.path.setObjectName("Mono")
        self.path.setTextInteractionFlags(Qt.TextInteractionFlag.TextSelectableByMouse)
        self.b_folder = tool_button("look.folder")
        self.b_folder.clicked.connect(self.choose_folder)
        self.b_reload = tool_button("look.reload")
        self.b_reload.clicked.connect(lambda: self.reload(force=True))
        self.b_repo = tool_button("remote.repo.change")
        self.b_repo.clicked.connect(self.choose_repo)
        bl.addWidget(self.l_folder)
        bl.addWidget(self.path, 1)
        bl.addWidget(self.b_reload)
        bl.addWidget(self.b_folder)
        bl.addWidget(self.b_repo)
        self.body.addWidget(bar)
        self.empty = QtWidgets.QLabel()
        self.empty.setObjectName("Label")
        self.empty.setWordWrap(True)
        self.empty.hide()
        self.body.addWidget(self.empty)
        self.packs_box = QtWidgets.QVBoxLayout()
        self.packs_box.setSpacing(8)
        self.body.addLayout(self.packs_box)
        self.grid = FlowGrid(TILE_W)
        self.body.addWidget(self.grid)
        self.finish()
        self._loaded = False

    # ----- Daten -----
    def showEvent(self, e):
        super().showEvent(e)
        if not self._loaded:
            self._loaded = True
            QtCore.QTimer.singleShot(0, self.reload)

    def set_source(self, src: str):
        self.source = src
        self.host.cfg[f"{self.kind}_source"] = src
        self.host.save()
        self.reload()

    def _source_widgets(self):
        online = self.source == "online"
        self.b_folder.setVisible(not online)
        self.b_repo.setVisible(online)
        self.l_folder.setText(t("remote.repo") if online else t("look.source"))
        self.path.setText(self.repo if online else str(self.folder))

    def reload(self, force: bool = False):
        self._source_widgets()
        self.empty.hide()
        if self.source == "online":
            if force or self.manifest is None or self.manifest.get("repo") != self.repo:
                repo = self.repo
                self.host.set_status(t("remote.loading", repo=repo))
                start_job(self, lambda: remote.load_manifest(repo), self._manifest_done)
            else:
                self._show_remote()
            return
        folder = self.folder
        kind = self.kind
        start_job(self, lambda: looks.items(folder, kind), self._items_done)

    def _manifest_done(self, ok, res):
        if self.source != "online":
            return
        if not ok:   # Meldung zeigen, darunter wenigstens die schon installierten Themes
            self.manifest = None
            msg = self._message(str(res))
            self.host.set_status(msg)
            kind = self.kind
            start_job(self, lambda: looks.items(Path("/nonexistent"), kind),
                      lambda ok2, items: self._items_done(ok2, items, message=msg, raw=True))
            return
        self.manifest = res
        self.host.set_status(t("remote.loaded", tag=res.get("tag", ""), repo=res.get("repo", "")))
        self._show_remote()

    def _show_remote(self):
        entries, packs = remote.items(self.manifest, CATEGORY[self.kind])
        result = []
        for e in entries:
            result.append({"name": e["name"], "label": e.get("farbe") or e["name"], "src": None,
                           "installed": looks.installed_path(e["name"], self.kind), "color": e.get("farbe"),
                           "remote": e})
        msg = "" if (entries or packs) else t("remote.empty", repo=self.repo)
        self._items_done(True, result, packs=packs, message=msg)

    def _items_done(self, ok, res, packs=(), message=None, raw=False):
        if not ok:
            self.host.log(str(res))
            return
        self.items = res
        for c in self.pack_cards:
            c.hide()
            c.setParent(None)
            c.deleteLater()
        self.pack_cards = []
        for pk in packs:
            card = PackCard(pk, self)
            self.packs_box.addWidget(card)
            self.pack_cards.append(card)
        for w in self.grid.items:
            w.hide()
            w.setParent(None)
            w.deleteLater()
        self.grid.items = []
        self.tiles = {}
        for it in self.items:
            tile = LookTile(it, self)
            self.tiles[it["name"]] = tile
            self.grid.add(tile)
            if it["name"] in self.previews:
                tile.pre.set_image(self.previews[it["name"]])
        if message is not None:
            self.empty.setText(message if raw else self._message(message))
            self.empty.setVisible(bool(message))
        else:
            has_src = any(it["src"] for it in self.items)
            self.empty.setVisible(not has_src)
            self.empty.setText(t("look.nofolder") if not Path(self.folder).is_dir() else t("look.nothemes"))
        self._filter()
        self.refresh_tiles()
        todo = [(it["name"], it["src"] or it["installed"], (it.get("remote") or {}).get("preview_url"))
                for it in self.items if it["name"] not in self.previews]
        fn = cursor_preview if self.kind == "cursor" else icon_preview

        def work(part, fn=fn):
            out = []
            for n, path, url in part:
                img = None
                if url:
                    data = remote.preview(url)
                    img = QtGui.QImage.fromData(data) if data else None
                if (img is None or img.isNull()) and path:
                    img = fn(path)
                out.append((n, img))
            return out
        # in Häppchen parallel laden: Vorschauen erscheinen nach und nach (Downloads von GitHub dauern)
        for i in range(0, len(todo), 4):
            part = todo[i:i + 4]
            start_job(self, lambda part=part, work=work: work(part), self._previews_done)

    def _message(self, text: str) -> str:
        """SourceError-Text „remote.xyz: detail“ → verständliche Meldung."""
        key, _, detail = text.partition(":")
        key = key.strip()
        if key.startswith("SourceError"):
            key = ""
        msg = t(key, repo=self.repo) if key.startswith("remote.") else text
        detail = detail.strip()
        return msg + (f" ({detail})" if detail and detail != self.repo and key.startswith("remote.") else "")

    def _previews_done(self, ok, res):
        if not ok:
            self.host.log(str(res))
            return
        for name, img in res:
            self.previews[name] = img
            tile = self.tiles.get(name)
            if tile:
                tile.pre.set_image(img)

    def _filter(self):
        q = self.search.text().strip().lower()
        for name, tile in self.tiles.items():
            it = tile.item
            hay = f"{name} {it['label']} {it['color'] or ''}".lower()
            tile.setVisible(not q or q in hay)

    def current(self) -> str:
        cur = looks.current(self.de, self.kind)
        return self.dry_applied if usercmd.DRY and self.dry_applied else cur

    def refresh_tiles(self):
        cur = self.current()
        busy = bool(self.busy_name) or self.host.busy()
        acc = self.host.accent()
        for tile in self.tiles.values():
            tile.refresh(cur, busy, acc)
        for c in self.pack_cards:
            c.refresh(busy)
        self.title_chips(cur)

    def title_chips(self, cur: str):
        while self.chips.count():
            w = self.chips.takeAt(0).widget()
            if w:
                w.deleteLater()
        lb = QtWidgets.QLabel()
        set_chip(lb, "ChipAccent" if cur else "Chip", (t("look.active") + ": " + cur) if cur else t("look.unknown"),
                 self.host.accent())
        self.chips.addWidget(lb)

    # ----- Aktionen -----
    def install(self, it):
        if self.busy_name or not (it["src"] or it.get("remote")):
            return
        self.busy_name = it["name"]
        self.host.set_status(t("look.installing", name=it["name"]))
        self.refresh_tiles()
        if it.get("remote"):
            entry, dest = it["remote"], looks.user_icons()
            fn = lambda: remote.install(entry, dest) and dest / entry["name"]  # noqa: E731
            if (dest / entry["name"]).exists():
                fn = lambda: dest / entry["name"]  # noqa: E731
        else:
            src = it["src"]
            fn = lambda: looks.install(src, lambda s: None)  # noqa: E731
        start_job(self, fn, lambda ok, res, i=it: self._installed(ok, res, i))

    def install_pack(self, pack):
        if self.busy_name:
            return
        self.busy_name = pack["name"]
        self.host.set_status(t("look.installing", name=pack.get("titel") or pack["name"]))
        self.refresh_tiles()
        dest = looks.user_icons()
        start_job(self, lambda: remote.install(pack, dest), self._pack_done)

    def _pack_done(self, ok, res):
        self.busy_name = ""
        if ok:
            for p in res:
                self.host.log(f"→ {p}")
            for it in self.items:
                it["installed"] = it["installed"] or looks.installed_path(it["name"], self.kind)
            self.host.set_status(t("remote.pack.ok", n=len(res)))
        else:
            self.host.log(str(res))
            self.host.set_status(f"{t('failed')}: {self._message(str(res))}")
        self.refresh_tiles()

    def _installed(self, ok, res, it):
        self.busy_name = ""
        if ok and res:
            it["installed"] = res
            self.host.log(f"→ {res}")
            self.host.set_status(t("look.installed", name=it["name"]))
        else:
            self.host.log(str(res))
            self.host.set_status(f"{t('failed')}: {self._message(str(res))}")
        self.refresh_tiles()

    def choose_repo(self):
        from hyprstudio import dialogs
        text, ok = dialogs.ask_text(self, t("remote.repo.change"), t("remote.repo.ask"), self.repo)
        text = (text or "").strip()
        if ok and text:
            if not remote.valid_repo(text):
                self.host.set_status(t("remote.badrepo", repo=text))
                return
            self.repo = text
            self.host.cfg["remote_repo"] = text
            self.host.save()
            self.reload(force=True)

    def apply(self, it):
        self.host.log("")
        if self.kind == "cursor":
            size = int(self.size.currentData() or 24)
            ok = looks.apply_cursor(self.de, it["name"], size, self.host.log)
        else:
            ok = looks.apply_icons(self.de, it["name"], self.host.log)
        if usercmd.DRY:
            self.dry_applied = it["name"]
        self.host.set_status(t("done") if ok else t("failed"))
        self.refresh_tiles()

    def _size_changed(self, _i):
        self.host.cfg["cursor_size"] = int(self.size.currentData())
        self.host.save()

    def choose_folder(self):
        from hyprstudio import dialogs
        d = dialogs.open_dir(self, t("look.folder"), str(self.folder))
        if d:
            self.folder = Path(d)
            self.host.cfg[f"{self.kind}_dir"] = d
            self.host.save()
            self.reload()

    def retranslate(self):
        super().retranslate()
        self.src_seg.retranslate()
        self._source_widgets()
        self.search.setPlaceholderText(t("search"))
        retranslate_buttons(self)
        if self.tiles:
            self.refresh_tiles()

    def refresh(self):
        if self.tiles:
            self.refresh_tiles()
