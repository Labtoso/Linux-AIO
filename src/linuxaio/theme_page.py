"""Gemeinsame Theme-Seite (SDDM, Plymouth): installierte Themes + KDE Store, mit Installieren-Aufforderung."""

import re

from . import theme
from .i18n import t, lang
from .qt import QtCore, QtGui, QtNetwork, QtWidgets, Qt
from .widgets import CACHE, ChipRow, Picture, Segmented, Tile, TileStrip, draw_icon, set_applied

UA = b"Linux-AIO/1.2 (LabtosoStudios)"


def _num(n: int) -> str:
    s = f"{n:,}"
    return s.replace(",", ".") if lang() == "de" else s


class Net(QtCore.QObject):
    """HTTP über Qt (mit Platten-Cache für Vorschaubilder)."""

    def __init__(self, parent=None):
        super().__init__(parent)
        self.nam = QtNetwork.QNetworkAccessManager(self)
        cache = QtNetwork.QNetworkDiskCache(self)
        cache.setCacheDirectory(str(CACHE / "http"))
        cache.setMaximumCacheSize(200 * 1024 * 1024)
        self.nam.setCache(cache)
        self.pix = {}
        self.raw = {}
        self.waiting = {}

    def request(self, url: str, cache: bool = False) -> QtNetwork.QNetworkRequest:
        req = QtNetwork.QNetworkRequest(QtCore.QUrl(url))
        req.setRawHeader(b"User-Agent", UA)
        req.setAttribute(QtNetwork.QNetworkRequest.Attribute.RedirectPolicyAttribute,
                         QtNetwork.QNetworkRequest.RedirectPolicy.NoLessSafeRedirectPolicy)
        if cache:
            req.setAttribute(QtNetwork.QNetworkRequest.Attribute.CacheLoadControlAttribute,
                             QtNetwork.QNetworkRequest.CacheLoadControl.PreferCache)
        return req

    def get(self, url: str, cache: bool = False) -> QtNetwork.QNetworkReply:
        return self.nam.get(self.request(url, cache))

    def image(self, url: str, callback) -> None:
        if not url:
            return
        if url in self.pix:
            callback(self.pix[url])
            return
        if url in self.waiting:
            self.waiting[url].append(callback)
            return
        self.waiting[url] = [callback]
        reply = self.get(url, cache=True)
        reply.finished.connect(lambda r=reply, u=url: self._image_done(r, u))

    def _image_done(self, reply, url):
        pm = QtGui.QPixmap()
        if reply.error() == QtNetwork.QNetworkReply.NetworkError.NoError:
            data = bytes(reply.readAll())
            self.raw[url] = data
            pm.loadFromData(data)
            if not pm.isNull() and pm.width() > 960:
                pm = pm.scaledToWidth(960, Qt.TransformationMode.SmoothTransformation)
        reply.deleteLater()
        self.pix[url] = pm
        for cb in self.waiting.pop(url, []):
            try:
                cb(pm)
            except RuntimeError:      # Widget inzwischen gelöscht
                pass


class ThemePage(QtWidgets.QWidget):
    """Unterklassen setzen: mod (Backend-Modul), act (Aktions-Präfix), style (Platzhalter),
    prompt_title/prompt_button/prompt_args und bauen optional eine Leiste (build_bar/refresh_bar)."""

    mod = None
    act = ""
    style = "login"
    prompt_title = ""
    prompt_button = ""
    prompt_args = ()
    has_test = False

    def __init__(self, host):
        super().__init__()
        self.host = host
        self.net = Net(self)
        self.mode = "local"
        self.sort = "down"
        self.search = ""
        self.local = []
        self.store = []
        self.page = 0
        self.total = -1
        self.loading = False
        self.gen = 0
        self.sel = {"local": None, "store": None}
        self.local_pix = {}
        self.current = ""
        self.mine = {}
        self.mine_store = set()

        cl = QtWidgets.QVBoxLayout(self)
        cl.setContentsMargins(0, 0, 0, 0)
        cl.setSpacing(14)
        self.build_bar(cl)

        self.body = QtWidgets.QStackedWidget()
        cl.addWidget(self.body, 1)

        # Seite 0: Aufforderung zum Installieren
        prompt = QtWidgets.QFrame()
        prompt.setObjectName("Hero")
        pl = QtWidgets.QVBoxLayout(prompt)
        pl.setContentsMargins(24, 24, 24, 24)
        pl.addStretch(2)
        self.prompt_pic = Picture(radius=14)
        self.prompt_pic.setFixedSize(320, 200)
        pl.addWidget(self.prompt_pic, 0, Qt.AlignmentFlag.AlignHCenter)
        pl.addSpacing(18)
        self.prompt_label = QtWidgets.QLabel()
        self.prompt_label.setObjectName("DeName")
        self.prompt_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        pl.addWidget(self.prompt_label)
        pl.addSpacing(10)
        self.prompt_btn = QtWidgets.QPushButton()
        self.prompt_btn.setObjectName("Primary")
        self.prompt_btn.setMinimumSize(260, 44)
        self.prompt_btn.setCursor(Qt.CursorShape.PointingHandCursor)
        self.prompt_btn.clicked.connect(lambda: self.host.run(list(self.prompt_args), self.prompt_args[0], self))
        pl.addWidget(self.prompt_btn, 0, Qt.AlignmentFlag.AlignHCenter)
        self.prompt_anchor = QtWidgets.QWidget()
        self.prompt_anchor.setFixedHeight(1)
        pl.addSpacing(16)
        pl.addWidget(self.prompt_anchor)
        pl.addStretch(3)
        self.body.addWidget(prompt)

        # Seite 1: Themes
        main = QtWidgets.QWidget()
        ml = QtWidgets.QVBoxLayout(main)
        ml.setContentsMargins(0, 0, 0, 0)
        ml.setSpacing(14)
        hero = QtWidgets.QFrame()
        hero.setObjectName("Hero")
        hero.setMinimumHeight(280)
        hl = QtWidgets.QHBoxLayout(hero)
        hl.setContentsMargins(18, 18, 18, 18)
        hl.setSpacing(24)
        self.big = Picture(hero, radius=12)
        self.big.setSizePolicy(QtWidgets.QSizePolicy.Policy.Expanding, QtWidgets.QSizePolicy.Policy.Expanding)
        hl.addWidget(self.big, 3)
        side = QtWidgets.QVBoxLayout()
        side.setSpacing(12)
        self.name = QtWidgets.QLabel()
        self.name.setObjectName("DeName")
        self.name.setWordWrap(True)
        side.addWidget(self.name)
        self.chips = ChipRow()
        self.chips.tip_target = self.name
        side.addWidget(self.chips)
        self.info = QtWidgets.QLabel()
        self.info.setObjectName("Label")
        self.info.setWordWrap(True)
        side.addWidget(self.info)
        side.addSpacing(4)
        self.files = QtWidgets.QComboBox()
        self.files.setObjectName("Combo")
        self.files.setCursor(Qt.CursorShape.PointingHandCursor)
        side.addWidget(self.files)
        side.addStretch()
        self.btn1 = QtWidgets.QPushButton()
        self.btn1.setObjectName("Primary")
        self.btn2 = QtWidgets.QPushButton()
        self.btn3 = QtWidgets.QPushButton()
        self.btn3.setObjectName("Danger")
        self.btn1.clicked.connect(self.primary)
        self.btn2.clicked.connect(self.secondary)
        self.btn3.clicked.connect(self.remove)
        for b in (self.btn1, self.btn2, self.btn3):
            b.setCursor(Qt.CursorShape.PointingHandCursor)
            b.setMinimumHeight(40)
            side.addWidget(b)
        side_w = QtWidgets.QWidget()
        side_w.setLayout(side)
        side_w.setFixedWidth(300)
        hl.addWidget(side_w)
        ml.addWidget(hero, 1)

        bar = QtWidgets.QHBoxLayout()
        bar.setSpacing(10)
        self.src = Segmented(("local", "store"), prefix="src.")
        self.src.changed.connect(self.set_mode)
        self.search_edit = QtWidgets.QLineEdit()
        self.search_edit.setObjectName("Search")
        self.search_edit.setClearButtonEnabled(True)
        self.search_edit.setFixedWidth(260)
        self.search_edit.addAction(draw_icon("search", theme.MUTED, 16), QtWidgets.QLineEdit.ActionPosition.LeadingPosition)
        self.search_timer = QtCore.QTimer(self)
        self.search_timer.setSingleShot(True)
        self.search_timer.setInterval(450)
        self.search_timer.timeout.connect(self._search_now)
        self.search_edit.textChanged.connect(lambda _: self.search_timer.start())
        self.sort_seg = Segmented(("down", "new", "high"), prefix="sort.")
        self.sort_seg.changed.connect(self.set_sort)
        bar.addWidget(self.src)
        bar.addStretch()
        bar.addWidget(self.search_edit)
        bar.addWidget(self.sort_seg)
        ml.addLayout(bar)
        self.strip = TileStrip()
        self.strip.reached_end.connect(self.load_more)
        self.tiles = {}
        ml.addWidget(self.strip)
        self.body.addWidget(main)

        self.src.select("local")
        self.sort_seg.select("down")

    default_theme = ""       # Systemstandard für „Reset to default“

    def reset_button(self) -> QtWidgets.QToolButton:
        """Round reset button: back to the system default theme (nothing is deleted)."""
        b = QtWidgets.QToolButton()
        b.setObjectName("ResetBtn")
        b.setFixedSize(30, 30)
        b.setIconSize(QtCore.QSize(16, 16))
        b.setIcon(draw_icon("reset", theme.MUTED, 16))
        b.setCursor(Qt.CursorShape.PointingHandCursor)
        b.clicked.connect(self._reset_default)
        self._b_reset = b
        return b

    def _reset_default(self):
        from hyprstudio import dialogs
        self.reload_local()
        ids = [x["id"] for x in self.local]
        target = self.default_theme if self.default_theme in ids else ""
        if not target:
            self.host.set_status(t("reset.nodefault"))
            return
        prev = self.current
        if prev == target:
            self.host.set_status(t("reset.already"))
            return
        if not dialogs.confirm(self, t("reset.title"), t("reset.sys", name=target)):
            return
        self.host.run([f"{self.act}-set", target], f"{self.act}-set", self)
        if prev:
            self.host.offer_undo(t("reset.sysdone", name=target, prev=prev),
                                 lambda: self.host.run([f"{self.act}-set", prev], f"{self.act}-set", self))

    # ----- Haken für Unterklassen -----
    def build_bar(self, layout):
        pass

    def refresh_bar(self):
        pass

    def installed(self) -> bool:
        return self.mod.is_installed()

    def local_animation(self, th, tile: bool = False):
        return None

    # ----- Daten -----
    def reload_local(self):
        self.local = self.mod.list_themes()
        self.current = self.mod.current_theme()
        self.mine = self.mod.load_store_state()
        self.mine_store = {v.get("store") for v in self.mine.values() if v.get("store")}
        if self.sel["local"] not in {x["id"] for x in self.local}:
            cur = next((x["id"] for x in self.local if x["id"] == self.current), None)
            self.sel["local"] = cur or (self.local[0]["id"] if self.local else None)

    def fetch_store(self, reset: bool):
        if reset:
            self.gen += 1
            self.store, self.page, self.total = [], 0, -1
            self.sel["store"] = None
        if self.loading or (self.total >= 0 and len(self.store) >= self.total):
            return
        self.loading = True
        gen = self.gen
        reply = self.net.get(self.mod.list_url(self.page, self.sort, self.search))
        reply.finished.connect(lambda r=reply, g=gen: self._store_done(r, g))
        if reset:
            self.rebuild()

    def _store_done(self, reply, gen):
        self.loading = False
        data = bytes(reply.readAll())
        err = reply.error() != QtNetwork.QNetworkReply.NetworkError.NoError
        reply.deleteLater()
        if gen != self.gen:
            return
        if err:
            self.host.set_status(t("sddm.neterr"))
            self.total = len(self.store)
            self.rebuild()
            return
        items, total = self.mod.parse_items(data)
        self.total = total
        self.page += 1
        known = {x["store_id"] for x in self.store}
        new = [x for x in items if x["store_id"] not in known]
        self.store += new
        if not items:
            self.total = len(self.store)
        if self.sel["store"] is None and self.store:
            self.sel["store"] = self.store[0]["store_id"]
        if self.mode == "store":
            if len(self.store) == len(new):
                self.rebuild()
            else:
                for it in new:
                    self._add_store_tile(it)
                self.refresh()

    def load_more(self):
        if self.mode == "store":
            self.fetch_store(False)

    # ----- Umschalten -----
    def set_mode(self, mode: str):
        self.mode = mode
        if mode == "store" and not self.store and not self.loading:
            self.fetch_store(True)
        self.rebuild()

    def set_sort(self, s: str):
        self.sort = s
        self.fetch_store(True)

    def _search_now(self):
        q = self.search_edit.text().strip()
        if q == self.search:
            return
        self.search = q
        if self.mode == "local":
            self.rebuild()
        else:
            self.fetch_store(True)

    def select(self, key: str):
        self.sel[self.mode] = key
        self.refresh()

    # ----- Kacheln -----
    def _visible_local(self):
        q = self.search.lower()
        return [x for x in self.local if not q or q in x["name"].lower() or q in x["id"].lower()]

    def rebuild(self):
        if self.mode == "local":
            self.reload_local()
        self.strip.clear()
        self.tiles = {}
        if self.mode == "local":
            for th in self._visible_local():
                pic = Picture(radius=8)
                self._local_picture(th, pic)
                tile = Tile(th["id"], th["name"], pic)
                tile.clicked.connect(self.select)
                tile.setToolTip(th["description"] or th["name"])
                self.tiles[th["id"]] = tile
                self.strip.add(tile)
        else:
            for it in self.store:
                self._add_store_tile(it)
        self.strip.horizontalScrollBar().setValue(0)
        self.refresh()

    def _add_store_tile(self, it):
        pic = Picture(radius=8)
        pic.set_placeholder(it["name"], self.host.accent(), loading=True, style=self.style)
        self.net.image(it["small"], pic.set_pixmap)
        tile = Tile(it["store_id"], it["name"], pic)
        tile.clicked.connect(self.select)
        tile.setToolTip(it["summary"] or it["name"])
        self.tiles[it["store_id"]] = tile
        self.strip.add(tile)

    def _local_picture(self, th, pic):
        anim = self.local_animation(th, tile=pic is not self.big)
        if anim:
            pic.set_animation(anim)
            return
        path = th["screenshot"]
        if path:
            if path not in self.local_pix:
                pm = QtGui.QPixmap(path)
                if not pm.isNull() and pm.width() > 960:
                    pm = pm.scaledToWidth(960, Qt.TransformationMode.SmoothTransformation)
                self.local_pix[path] = pm
            pic.set_pixmap(self.local_pix[path])
            if pic.pix:
                return
        pic.set_placeholder(th["id"], self.host.accent(), style=self.style)

    # ----- Anzeige -----
    def item(self):
        key = self.sel[self.mode]
        if self.mode == "local":
            return next((x for x in self.local if x["id"] == key), None)
        return next((x for x in self.store if x["store_id"] == key), None)

    def drawer_anchor(self):
        return self.src if self.body.currentIndex() == 1 else self.prompt_anchor

    def retranslate(self):
        self.src.retranslate()
        self.sort_seg.retranslate()
        self.search_edit.setPlaceholderText(t("search"))
        self.refresh()

    def refresh(self):
        self.refresh_bar()
        accent = self.host.accent()
        busy = self.host.busy()
        if not self.installed():
            self.body.setCurrentIndex(0)
            self.prompt_label.setText(t(self.prompt_title))
            self.prompt_btn.setText(t(self.prompt_button))
            self.prompt_btn.setEnabled(not busy)
            self.prompt_pic.set_placeholder(self.act, accent, style=self.style)
            return
        if self.body.currentIndex() == 0:
            self.body.setCurrentIndex(1)
            self.reload_local()
            QtCore.QTimer.singleShot(0, self.rebuild)
        store = self.mode == "store"
        self.sort_seg.setVisible(store)
        for key, tile in self.tiles.items():
            if store:
                dot = theme.OK if key in self.mine_store else None
            else:
                dot = accent if key == self.current else (theme.OK if key in self.mine else None)
            tile.refresh(key == self.sel[self.mode], dot, accent)

        it = self.item()
        self.files.hide()
        for b in (self.btn1, self.btn2, self.btn3):
            b.show()
            b.setToolTip("")
        if not it:
            empty = self.loading and store
            self.name.setText(t("sddm.loading") if empty else t("sddm.none"))
            self.chips.set([])
            self.info.setText("")
            self.big.set_placeholder(self.act, accent, loading=empty, style=self.style)
            for b in (self.btn1, self.btn2, self.btn3):
                b.hide()
            return

        if store:
            self.name.setText(it["name"])
            self.name.setProperty("base_tip", it["summary"])
            chips = []
            if it["store_id"] in self.mine_store:
                chips.append((t("installed"), "ChipOk", ""))
            if it.get("qt6") and self.act == "sddm":
                chips.append(("Qt6", "Chip", ""))
            self.chips.set(chips)
            score = f"{it['score'] / 10:.1f}".replace(".", "," if lang() == "de" else ".")
            self.info.setText(f"{it['author']}  ·  {_num(it['downloads'])} Downloads  ·  {score}/10")
            self.big.set_placeholder(it["name"], accent, loading=True, style=self.style)
            key = it["store_id"]
            self.net.image(it["preview"], lambda pm, k=key: self._hero_pix(k, pm))
            if len(it["files"]) > 1:
                cur = self.files.currentData() if self.files.property("item") == key else None
                self.files.blockSignals(True)
                self.files.clear()
                for f in it["files"]:
                    self.files.addItem(f"{f['name']}  ({f['size_kb']} KB)", f["index"])
                self.files.setProperty("item", key)
                if cur is not None:
                    self.files.setCurrentIndex(max(0, self.files.findData(cur)))
                self.files.blockSignals(False)
                self.files.setToolTip(t("sddm.files"))
                self.files.show()
            set_applied(self.btn1, False)
            self.btn1.setText(t("sddm.download"))
            self.btn1.setEnabled(not busy and bool(it["files"]))
            self.btn2.setText(t("sddm.open"))
            self.btn2.setEnabled(True)
            self.btn3.hide()
        else:
            self.name.setText(it["name"])
            self.name.setProperty("base_tip", it["description"])
            chips = []
            if it["id"] == self.current:
                chips.append((t("sddm.active"), "ChipAccent", ""))
            chips += self.mod.theme_chips(it)
            if it["id"] in self.mine:
                chips.append((t("sddm.mine"), "ChipOk", ""))
            self.chips.set(chips)
            bits = [b for b in (it["author"], it["version"] and f"v{it['version']}", it["id"]) if b]
            self.info.setText("  ·  ".join(dict.fromkeys(bits)))
            self._local_picture(it, self.big)
            active = it["id"] == self.current
            set_applied(self.btn1, active, t("applied"))
            if not active:
                self.btn1.setText(t("sddm.set"))
                self.btn1.setEnabled(not busy)
            if self.has_test:
                self.btn2.setText(t("sddm.test"))
                can = self.mod.test_command(it) is not None
                self.btn2.setEnabled(can)
                self.btn2.setToolTip("" if can else t("sddm.nogreeter"))
            else:
                self.btn2.hide()
            self.btn3.setText(t("remove"))
            mine = it["id"] in self.mine
            self.btn3.setEnabled(not busy and mine and it["id"] != self.current)
            self.btn3.setToolTip("" if mine else t("err.sddm.foreign"))

    def _hero_pix(self, key, pm):
        if self.mode == "store" and self.sel["store"] == key and not pm.isNull():
            it = self.item()
            raw = self.net.raw.get(it["preview"]) if it else None
            if raw and raw[:4] == b"GIF8":
                self.big.set_image_data(raw)      # animierte Store-Vorschau abspielen
            else:
                self.big.set_pixmap(pm)

    # ----- Aktionen -----
    def primary(self):
        it = self.item()
        if not it:
            return
        if self.mode == "local":
            self.host.run([f"{self.act}-set", it["id"]], f"{self.act}-set", self)
        else:
            self.download(it)

    def secondary(self):
        it = self.item()
        if not it:
            return
        if self.mode == "local":
            cmd = self.mod.test_command(it)
            if cmd:
                self.host.log("$ " + " ".join(cmd))
                QtCore.QProcess.startDetached(cmd[0], cmd[1:])
        else:
            QtGui.QDesktopServices.openUrl(QtCore.QUrl(self.mod.STORE_PAGE.format(id=it["store_id"])))

    def remove(self):
        it = self.item()
        if it and self.mode == "local" and self.host.confirm(t("remove"), t("confirm.remove", name=it["name"])):
            self.host.run([f"{self.act}-remove", it["id"]], f"{self.act}-remove", self)

    def download(self, it):
        index = self.files.currentData() if self.files.isVisible() else (it["files"][0]["index"] if it["files"] else 1)
        self.host.task_begin(t("sddm.downloading") + f" · {it['name']}")
        reply = self.net.get(self.mod.item_url(it["store_id"]))     # frische Download-Links holen
        reply.finished.connect(lambda r=reply, i=it, x=index: self._details_done(r, i, x))

    def _details_done(self, reply, it, index):
        data = bytes(reply.readAll())
        ok = reply.error() == QtNetwork.QNetworkReply.NetworkError.NoError
        reply.deleteLater()
        fresh = self.mod.parse_item(data) if ok else None
        files = (fresh or {}).get("files") or it["files"]
        f = next((x for x in files if x["index"] == index), files[0] if files else None)
        if not f:
            self.host.task_end(False, t("sddm.neterr"))
            return
        self.host.log(f"GET {f['name']}")
        r = self.net.get(f["link"])
        r.downloadProgress.connect(lambda done, total: self.host.task_progress(int(done * 100 / total) if total > 0 else 0))
        r.finished.connect(lambda r=r, i=it, fn=f["name"]: self._file_done(r, i, fn))

    def _file_done(self, reply, it, filename):
        data = bytes(reply.readAll())
        ok = reply.error() == QtNetwork.QNetworkReply.NetworkError.NoError and len(data) > 0
        err = reply.errorString()
        reply.deleteLater()
        if not ok:
            self.host.log(err)
            self.host.task_end(False, t("sddm.neterr"))
            return
        # Originaler Dateiname (ohne Store-Nummer davor), eigener Ordner pro Store-Eintrag
        folder = CACHE / "downloads" / it["store_id"]
        folder.mkdir(parents=True, exist_ok=True)
        name = re.sub(r"[^A-Za-z0-9._+-]+", "-", filename) or "theme.tar.gz"
        path = folder / name
        path.write_bytes(data)
        self.host.log(f"{path} ({len(data) // 1024} KB)")
        args = [f"{self.act}-install", str(path), f"--store={it['store_id']}"]
        raw = self.net.raw.get(it["preview"])
        if raw:
            pv = folder / "preview.png"
            pm = QtGui.QPixmap()
            if pm.loadFromData(raw) and pm.save(str(pv), "PNG"):
                args.append(f"--preview={pv}")
        self.host.task_end(True, "")
        self.host.run(args, f"{self.act}-install", self)

    def finished(self, ok: bool, action: str, result: str):
        self.reload_local()
        if ok and action in (f"{self.act}-install", f"{self.act}-set") and result:
            self.sel["local"] = result
            self.mode = "local"
            self.src.select("local")
        self.rebuild()
