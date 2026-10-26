"""Hyprland › Hintergrundbild (Hyprpaper) und › Mauszeiger (Hyprcursor/XCursor).
Beide haben eine Bibliothek bzw. Suche und können Neues aus dem Netz installieren (online.py)."""

import os
from pathlib import Path

from . import dialogs, model, online, paper
from .i18n import t
from .qt import QtCore, QtGui, QtWidgets, Qt, Signal
from .widgets import Card, FlowGrid, Segmented, Switch, set_chip
from .workspaces import monitors


def _load(path: str, w: int, h: int) -> QtGui.QImage:
    """Bild verkleinert laden (schnell auch bei großen Dateien)."""
    r = QtGui.QImageReader(path)
    r.setAutoTransform(True)
    size = r.size()
    if size.isValid() and size.width() > 0:
        scale = max(w / size.width(), h / size.height())
        r.setScaledSize(QtCore.QSize(max(1, int(size.width() * scale)), max(1, int(size.height() * scale))))
    return r.read()


def _thumb(path: str, w: int, h: int, radius: int = 12) -> QtGui.QPixmap:
    pm = QtGui.QPixmap(w, h)
    pm.fill(Qt.GlobalColor.transparent)
    p = QtGui.QPainter(pm)
    p.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
    p.setRenderHint(QtGui.QPainter.RenderHint.SmoothPixmapTransform)
    clip = QtGui.QPainterPath()
    clip.addRoundedRect(QtCore.QRectF(0, 0, w, h), radius, radius)
    p.setClipPath(clip)
    full = os.path.expanduser(path) if path else ""
    img = QtGui.QImage()
    if full and Path(full).is_dir():  # Ordner: erstes Bild zeigen
        for f in sorted(Path(full).iterdir()):
            if f.suffix.lower() in paper.IMAGE_EXT:
                img = _load(str(f), w, h)
                if not img.isNull():
                    break
    elif full:
        img = _load(full, w, h)
    if img.isNull():
        p.fillRect(QtCore.QRectF(0, 0, w, h), QtGui.QColor("#1b2430"))
        p.setPen(QtGui.QColor("#8a98ab"))
        p.drawText(QtCore.QRectF(0, 0, w, h), Qt.AlignmentFlag.AlignCenter, t("paper.none"))
    else:
        scale = max(w / img.width(), h / img.height())
        sw, sh = w / scale, h / scale
        p.drawImage(QtCore.QRectF(0, 0, w, h), img, QtCore.QRectF((img.width() - sw) / 2, (img.height() - sh) / 2, sw, sh))
    p.end()
    return pm


_thumbs: dict = {}


def _thumb_cached(path: str) -> QtGui.QPixmap:
    """Vorschaubild der Bibliothek merken (nach Pfad und Änderungszeit) – Wiederöffnen ohne neu zu laden."""
    try:
        key = (path, os.path.getmtime(path))
    except OSError:
        key = (path, 0)
    if key not in _thumbs:
        _thumbs[key] = _thumb(path, 172, 95, 10)
    return _thumbs[key]


def _pick(parent, folder: bool, start: str) -> str:
    start = os.path.expanduser(start) if start else str(online.wall_dir() if online.wall_dir().is_dir() else Path.home())
    if folder:
        return dialogs.open_dir(parent, t("paper.folder"), start)
    return dialogs.open_file(parent, t("paper.image"), start, "Bilder (*.png *.jpg *.jpeg *.webp *.jxl *.bmp)")


def _short(path: str) -> str:
    home = str(Path.home())
    return "~" + path[len(home):] if path.startswith(home) else path


def _btn(primary: bool = False) -> QtWidgets.QPushButton:
    b = QtWidgets.QPushButton()
    b.setCursor(Qt.CursorShape.PointingHandCursor)
    if primary:
        b.setObjectName("Primary")
    return b


def _action(primary: bool = False) -> QtWidgets.QPushButton:
    """Knopf für die Kopfzeile (Anwenden/Rückgängig) – überall gleich groß."""
    b = _btn(primary)
    b.setFixedSize(170, 40)
    return b


def card_header(card: Card, *widgets) -> None:
    """Titel der Karte links, Knöpfe rechts in derselben Zeile."""
    lay = card.layout()
    lay.removeWidget(card.title)
    row = QtWidgets.QHBoxLayout()
    row.setSpacing(8)
    row.addWidget(card.title)
    row.addStretch()
    for w in widgets:
        row.addWidget(w)
    lay.insertLayout(0, row)


def _scroll(owner: QtWidgets.QWidget) -> QtWidgets.QVBoxLayout:
    outer = QtWidgets.QVBoxLayout(owner)
    outer.setContentsMargins(0, 0, 0, 0)
    sa = QtWidgets.QScrollArea()
    sa.setWidgetResizable(True)
    sa.setHorizontalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
    inner = QtWidgets.QWidget()
    inner.setObjectName("ScrollInner")
    lay = QtWidgets.QVBoxLayout(inner)
    lay.setContentsMargins(0, 0, 6, 0)
    lay.setSpacing(14)
    sa.setWidget(inner)
    outer.addWidget(sa)
    return lay


class Job(QtCore.QThread):
    """Download im Hintergrund; Protokollzeilen und Ergebnis kommen als Signal in den GUI-Thread."""
    line = Signal(str)
    result = Signal(object)

    def __init__(self, fn, parent=None):
        super().__init__(parent)
        self.fn = fn

    def run(self):
        try:
            r = self.fn(self.line.emit)
        except Exception as e:  # noqa: BLE001 – Netzfehler sauber melden
            r = e
        self.result.emit(r)


class Thumb(QtWidgets.QLabel):
    clicked = Signal(str)

    def __init__(self, path: Path, parent=None):
        super().__init__(parent)
        self.path = str(path)
        self.setFixedSize(176, 99)
        self.setCursor(Qt.CursorShape.PointingHandCursor)
        self.setToolTip(online.short(path))

    def mark(self, on: bool, accent: str):
        self.setStyleSheet(f"border: 2px solid {accent}; border-radius: 12px;" if on
                           else "border: 2px solid transparent; border-radius: 12px;")

    def mouseReleaseEvent(self, e):
        if e.button() == Qt.MouseButton.LeftButton:
            self.clicked.emit(self.path)


class WallTile(QtWidgets.QFrame):
    """Online-Bild: Vorschau + Auflösung/Favoriten; Klick = laden und als Hintergrund setzen."""
    clicked = Signal(object)

    def __init__(self, item: dict, parent=None):
        super().__init__(parent)
        self.item = item
        self.setObjectName("BindRow")
        self.setCursor(Qt.CursorShape.PointingHandCursor)
        self.setFixedSize(196, 146)
        lay = QtWidgets.QVBoxLayout(self)
        lay.setContentsMargins(8, 8, 8, 8)
        lay.setSpacing(6)
        self.pic = QtWidgets.QLabel()
        self.pic.setFixedSize(180, 101)
        self.pic.setStyleSheet("background: #1b2430; border-radius: 10px;")
        info = QtWidgets.QLabel(f"{item['res'].replace('x', '×')}  ·  {t('paper.favs', n=item['favs'])}")
        info.setObjectName("RowLabel")
        lay.addWidget(self.pic)
        lay.addWidget(info)
        self.setToolTip(t("tip.wall_tile"))

    def mouseReleaseEvent(self, e):
        if e.button() == Qt.MouseButton.LeftButton:
            self.clicked.emit(self.item)


WALL_SORTS = ("toplist", "date_added", "favorites", "random")


class PaperEditor(QtWidgets.QWidget):
    KIND = "paper"

    def __init__(self, win):
        super().__init__()
        self.win = win
        self.state = paper.paper_state(win.cfg.get("paper"))
        self.cards, self.switches, self.segs = [], [], []
        self.job = None
        self.thumbs = []
        self._todo = []
        self.btn_undo, self.btn_apply = _action(), _action(True)
        self.btn_apply.clicked.connect(self.apply)
        self.btn_undo.clicked.connect(self.undo)
        self.actions = [self.btn_undo, self.btn_apply]
        lay = _scroll(self)

        # Alle Monitore
        c = Card("paper.all", self)
        self.cards.append(c)
        row = QtWidgets.QHBoxLayout()
        row.setSpacing(16)
        self.thumb = QtWidgets.QLabel()
        self.thumb.setFixedSize(320, 180)
        row.addWidget(self.thumb)
        col = QtWidgets.QVBoxLayout()
        col.setSpacing(8)
        self.path_lbl = QtWidgets.QLabel()
        self.path_lbl.setObjectName("Mono")
        self.path_lbl.setWordWrap(True)
        col.addWidget(self.path_lbl)
        self.btn_img, self.btn_dir = _btn(), _btn()
        for b in (self.btn_img, self.btn_dir):
            b.setMinimumWidth(220)
            col.addWidget(b, 0, Qt.AlignmentFlag.AlignLeft)
        self.btn_img.clicked.connect(lambda: self._choose("", False))
        self.btn_dir.clicked.connect(lambda: self._choose("", True))
        col.addStretch()
        row.addLayout(col, 1)
        c.layout().addLayout(row)
        c.choice("fit", paper.FITS, "fit.", label_key="paper.fit")
        lay.addWidget(c)

        # Bibliothek: installierte Bilder, Klick = für alle Monitore
        self.lib_card = Card("paper.lib", self)
        self.cards.append(self.lib_card)
        self.btn_get = _btn(True)
        self.btn_get.clicked.connect(self._get_menu)
        self.btn_lib_open = _btn()
        self.btn_lib_open.clicked.connect(lambda: online.open_dir(online.wall_dir()))
        card_header(self.lib_card, self.btn_lib_open, self.btn_get)
        self.lib_empty = QtWidgets.QLabel()
        self.lib_empty.setObjectName("RowLabel")
        self.lib_card.layout().addWidget(self.lib_empty)
        self.grid = FlowGrid(176)
        self.lib_card.layout().addWidget(self.grid)
        lib_at = lay.count()

        # Online suchen (wallhaven.cc, nur jugendfrei)
        self.web_card = Card("paper.web", self)
        self.cards.append(self.web_card)
        self.wsearch = QtWidgets.QLineEdit()
        self.wsearch.setClearButtonEnabled(True)
        self.wsearch.setMinimumHeight(40)
        self.web_card.layout().addWidget(self.wsearch)
        frow = QtWidgets.QHBoxLayout()
        frow.setSpacing(10)
        self.wf = {}
        for key, opts, pre in (("wall_sort", WALL_SORTS, "wsort."), ("wall_cat", ("all", "general", "anime"), "wcat."),
                               ("wall_res", ("all", "fhd", "qhd", "uhd"), "wres.")):
            sg = Segmented(opts, prefix=pre)
            sg.changed.connect(lambda v, k=key: self._wall_filter(k, v))
            self.segs.append(sg)
            frow.addWidget(sg)
            self.wf[key] = (sg, opts)
        frow.addStretch()
        self.web_card.layout().addLayout(frow)
        self.web_state = QtWidgets.QLabel()
        self.web_state.setObjectName("RowLabel")
        self.web_card.layout().addWidget(self.web_state)
        self.web_grid = FlowGrid(196)
        self.web_card.layout().addWidget(self.web_grid)
        self.btn_wmore = _btn()
        self.btn_wmore.clicked.connect(lambda: self.wall_search(more=True))
        self.web_card.layout().addWidget(self.btn_wmore, 0, Qt.AlignmentFlag.AlignHCenter)
        lay.addWidget(self.web_card)
        lay.addWidget(self.lib_card)  # Bibliothek unter der Suche
        self.wgen, self.wpage, self.wlast, self.wtiles = 0, 1, 1, {}
        self.wfetch = self.wpics = None
        self._wtimer = QtCore.QTimer(self)
        self._wtimer.setSingleShot(True)
        self._wtimer.setInterval(500)
        self._wtimer.timeout.connect(self.wall_search)
        self.wsearch.textChanged.connect(lambda _: self._wtimer.start())
        self._wloaded = False

        # Pro Monitor
        self.mon_card = Card("paper.per_monitor", self)
        self.cards.append(self.mon_card)
        self.mon_lay = QtWidgets.QVBoxLayout()
        self.mon_lay.setSpacing(8)
        self.mon_card.layout().addLayout(self.mon_lay)
        lay.addWidget(self.mon_card)

        # Diashow + Sonstiges
        c = Card("paper.extra", self)
        self.cards.append(c)
        c.slider("timeout", 5, 600, 5, "{:.0f} s", label_key="paper.timeout", tip="tip.paper.timeout")
        c.toggle("random", label_key="paper.random")
        c.toggle("recursive", label_key="paper.recursive")
        c.toggle("splash", label_key="paper.splash")
        self.auto = Switch()
        self.auto.toggled.connect(self._autostart)
        c._add("paper.autostart", self.auto, None, "tip.paper.autostart", None, label_key="paper.autostart")
        self.switches.append(self.auto)
        lay.addWidget(c)
        lay.addStretch()
        self._timer = QtCore.QTimer(self)
        self._timer.timeout.connect(self._thumb_step)
        self.refresh_controls()

    # ----- Zustand -----
    def set(self, key, value):
        self.state[key] = value
        self.changed()

    def changed(self):
        self.win.cfg["paper"] = self.state
        self.win.save_later()
        self.thumb.setPixmap(_thumb(self.state["path"], 320, 180))
        self.path_lbl.setText(_short(self.state["path"]))
        self.path_lbl.setVisible(bool(self.state["path"]))
        self._mark()

    def refresh_controls(self):
        for c in self.cards:
            c.refresh(self.state)
        self.auto.set_now("hyprpaper" in self.win.hypr_page.state["autostart"])
        self._monitors()
        self.changed()

    def refresh(self):
        """Beim Öffnen des Bereichs: Bibliothek neu einlesen, Online-Suche einmal starten."""
        self.load_library()
        for key, (sg, opts) in self.wf.items():
            sg.select(self._wf(key))
        if not self._wloaded:
            self._wloaded = True
            self.wall_search()

    # ----- Online suchen -----
    def _wf(self, key) -> str:
        opts = self.wf[key][1]
        v = self.win.cfg.get(key)
        return v if v in opts else opts[0]

    def _wall_filter(self, key, value):
        self.win.cfg[key] = value
        self.win.save_later()
        self.wall_search()

    def wall_search(self, more: bool = False):
        if more:
            self.wpage += 1
        else:
            self.wgen += 1
            self.wpage = 1
            for w in self.web_grid.items:
                w.hide()
                w.setParent(None)
                w.deleteLater()
            self.web_grid.items, self.wtiles = [], {}
        gen, page = self.wgen, self.wpage
        q, sort, cat, res = self.wsearch.text(), self._wf("wall_sort"), self._wf("wall_cat"), self._wf("wall_res")
        self.web_state.setText(t("paper.web_loading"))
        self.web_state.show()
        self.btn_wmore.hide()
        self.wfetch = Job(lambda _log: online.wall_search(q, sort, cat, res, page), self)
        self.wfetch.result.connect(lambda r, g=gen: self._wall_loaded(g, r))
        self.wfetch.start()

    def _wall_loaded(self, gen, r):
        if gen != self.wgen:
            return
        if isinstance(r, Exception):
            self.web_state.setText(t("cur.offline"))
            return
        items, self.wlast = r
        pics = []
        for it in items:
            tile = WallTile(it)
            tile.clicked.connect(self._wall_pick)
            self.web_grid.add(tile)
            self.wtiles[it["id"]] = tile
            pics.append((it["id"], it["thumb"]))
        self.web_state.setText(t("cur.nomatch"))
        self.web_state.setVisible(not self.web_grid.items)
        self.btn_wmore.setText(t("cur.more"))
        self.btn_wmore.setVisible(self.wpage < self.wlast and bool(items))
        if pics:
            self.wpics = PicJob(pics, self)
            self.wpics.got.connect(lambda k, data, g=gen: self._wall_pic(g, k, data))
            self.wpics.start()

    def _wall_pic(self, gen, key, data):
        tile = self.wtiles.get(key)
        if gen == self.wgen and tile is not None:
            img = QtGui.QImage()
            img.loadFromData(data)
            if not img.isNull():
                pm = QtGui.QPixmap(180, 101)
                pm.fill(Qt.GlobalColor.transparent)
                p = QtGui.QPainter(pm)
                p.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
                p.setRenderHint(QtGui.QPainter.RenderHint.SmoothPixmapTransform)
                clip = QtGui.QPainterPath()
                clip.addRoundedRect(QtCore.QRectF(0, 0, 180, 101), 10, 10)
                p.setClipPath(clip)
                s = max(180 / img.width(), 101 / img.height())
                sw, sh = 180 / s, 101 / s
                p.drawImage(QtCore.QRectF(0, 0, 180, 101), img,
                            QtCore.QRectF((img.width() - sw) / 2, (img.height() - sh) / 2, sw, sh))
                p.end()
                tile.pic.setPixmap(pm)

    def _wall_pick(self, item):
        if self.job:
            return
        self.win.log("")
        self.win.set_status(t("dl.loading", name=item["res"]))
        self.job = Job(lambda log: online.wall_install(item, log), self)
        self.job.line.connect(self.win.log)
        self.job.result.connect(self._wall_got)
        self.job.start()

    def _wall_got(self, r):
        self.job = None
        if isinstance(r, Exception):
            self.win.log(f"Fehler: {r}")
            self.win.set_status(t("dl.fail", err=r))
            return
        self.state["path"] = str(r)
        self.changed()
        self._monitors()
        self.load_library()
        self.win.set_status(t("paper.picked", name=r.name))

    # ----- Bibliothek -----
    def load_library(self):
        files = online.library_images(60)
        if files == getattr(self, "_lib_files", None):  # nichts Neues → Vorschaubilder behalten
            self._mark()
            return
        self._lib_files = files
        self._timer.stop()
        for w in self.grid.items:
            w.hide()
            w.setParent(None)
            w.deleteLater()
        self.grid.items, self.thumbs = [], []
        for f in files:
            th = Thumb(f)
            th.clicked.connect(self._lib_pick)
            self.grid.add(th)
            self.thumbs.append(th)
        self._todo = list(self.thumbs)
        self.lib_empty.setText(t("paper.lib_empty", dir=online.short(online.wall_dir())))
        self.lib_empty.setVisible(not files)
        self._mark()
        self._timer.start(15)

    def _thumb_step(self):
        for _ in range(3):  # ein paar pro Takt, damit die Oberfläche flüssig bleibt
            if not self._todo:
                self._timer.stop()
                return
            th = self._todo.pop(0)
            th.setPixmap(_thumb_cached(th.path))

    def _mark(self):
        acc = self.win.accent()
        for th in self.thumbs:
            th.mark(th.path == os.path.expanduser(self.state["path"]), acc)

    def _lib_pick(self, path: str):
        self.state["path"] = path
        self.changed()
        self._monitors()
        self.win.set_status(t("paper.picked", name=Path(path).name))

    def _get_menu(self):
        m = QtWidgets.QMenu(self)
        for pack in online.WALL_PACKS:
            a = m.addAction(t("paper.pack", name=pack[1], n=online.MAX_WALLS))
            a.triggered.connect(lambda _=False, p=pack: self._install(p))
        m.exec(self.btn_get.mapToGlobal(QtCore.QPoint(0, self.btn_get.height() + 4)))

    def _install(self, pack):
        if self.job:
            return
        self.win.log("")
        self.win.set_status(t("dl.loading", name=pack[1]))
        self.btn_get.setEnabled(False)
        self.job = Job(lambda log: online.install_walls(pack, log), self)
        self.job.line.connect(self.win.log)
        self.job.result.connect(lambda r, p=pack: self._installed(p, r))
        self.job.start()

    def _installed(self, pack, r):
        self.job = None
        self.btn_get.setEnabled(True)
        if isinstance(r, Exception):
            self.win.log(f"Fehler: {r}")
            self.win.set_status(t("dl.fail", err=r))
            return
        self.win.set_status(t("paper.got", name=pack[1], n=r))
        self.load_library()

    # ----- Auswahl -----
    def _choose(self, mon: str, folder: bool):
        cur = self.state["monitors"].get(mon, {}).get("path", "") if mon else self.state["path"]
        path = _pick(self, folder, cur)
        if not path:
            return
        if mon:
            self.state["monitors"][mon] = {"path": path, "fit": self.state["fit"]}
            self._monitors()
        else:
            self.state["path"] = path
        self.changed()

    def _clear(self, mon: str):
        self.state["monitors"].pop(mon, None)
        self._monitors()
        self.changed()

    def _monitors(self):
        while self.mon_lay.count():
            w = self.mon_lay.takeAt(0).widget()
            if w:
                w.hide()
                w.setParent(None)
                w.deleteLater()
        for mon in sorted(set(monitors()) | set(self.state["monitors"])):
            m = self.state["monitors"].get(mon)
            row = QtWidgets.QFrame()
            row.setObjectName("BindRow")
            rl = QtWidgets.QHBoxLayout(row)
            rl.setContentsMargins(12, 8, 10, 8)
            rl.setSpacing(10)
            th = QtWidgets.QLabel()
            th.setPixmap(_thumb(m["path"] if m else self.state["path"], 96, 54, 8))
            name = QtWidgets.QLabel(mon)
            name.setObjectName("Mono")
            name.setFixedWidth(110)
            path = QtWidgets.QLabel(_short(m["path"]) if m else t("paper.same"))
            path.setObjectName("RowLabel")
            rl.addWidget(th)
            rl.addWidget(name)
            rl.addWidget(path, 1)
            btns = [(t("paper.image_btn"), lambda _=False, x=mon: self._choose(x, False)),
                    (t("paper.folder_btn"), lambda _=False, x=mon: self._choose(x, True))]
            if m:
                btns.append((t("paper.reset"), lambda _=False, x=mon: self._clear(x)))
            for txt, fn in btns:
                b = _btn()
                b.setText(txt)
                b.setMinimumWidth(96)
                b.clicked.connect(fn)
                rl.addWidget(b)
            self.mon_lay.addWidget(row)
        self.mon_card.setVisible(self.mon_lay.count() > 0)

    def _autostart(self, on: bool):
        hp = self.win.hypr_page
        cmds = [c for c in hp.state["autostart"] if c.strip() != "hyprpaper"]
        if on:
            cmds.insert(0, "hyprpaper")
        hp.state["autostart"] = cmds
        hp.refresh_controls()
        hp.changed()
        self.win.set_status(t("paper.auto_hint"))

    def apply(self):
        self.win.log("")
        ok, info = paper.apply_paper(self.state, model.config_home() / "hypr", self.win.log)
        self.win.set_status(t("done") if ok else t(info))

    def undo(self):
        ok = model.undo_file(model.config_home() / "hypr" / "hyprpaper.conf")
        if ok:
            paper.restart_paper(self.win.log)
        self.win.set_status(t("undone") if ok else t("nothing"))

    def retranslate(self):
        for c in self.cards:
            c.retranslate()
        for sg in self.segs:
            sg.retranslate()
        self.btn_img.setText(t("paper.image"))
        self.btn_dir.setText(t("paper.folder"))
        self.btn_get.setText(t("paper.get"))
        self.wsearch.setPlaceholderText(t("paper.web_search"))
        self.btn_wmore.setText(t("cur.more"))
        self.btn_lib_open.setText(t("paper.open"))
        self.btn_undo.setText(t("undo"))
        self.btn_apply.setText(t("apply"))
        self.lib_empty.setText(t("paper.lib_empty", dir=online.short(online.wall_dir())))
        self._monitors()
        self.changed()

    def apply_theme(self, accent):
        for sw in self.switches:
            sw.accent = accent
            sw.update()
        self._mark()


class PicJob(QtCore.QThread):
    """Vorschaubilder nacheinander laden."""
    got = Signal(str, bytes)

    def __init__(self, items, parent=None):
        super().__init__(parent)
        self.items = items

    def run(self):
        for key, url in self.items:
            if self.isInterruptionRequested():
                return
            try:
                self.got.emit(key, online._get(url, 20))
            except Exception:  # noqa: BLE001 – fehlendes Vorschaubild ist egal
                pass


def _pic(data: bytes, w: int, h: int) -> QtGui.QPixmap:
    img = QtGui.QImage()
    img.loadFromData(data)
    pm = QtGui.QPixmap(w, h)
    pm.fill(QtGui.QColor(0, 0, 0, 0))
    if img.isNull():
        return pm
    p = QtGui.QPainter(pm)
    p.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
    p.setRenderHint(QtGui.QPainter.RenderHint.SmoothPixmapTransform)
    clip = QtGui.QPainterPath()
    clip.addRoundedRect(QtCore.QRectF(0, 0, w, h), 8, 8)
    p.setClipPath(clip)
    p.fillRect(QtCore.QRectF(0, 0, w, h), QtGui.QColor("#e9edf2"))
    s = min(w / img.width(), h / img.height())
    dw, dh = img.width() * s, img.height() * s
    p.drawImage(QtCore.QRectF((w - dw) / 2, (h - dh) / 2, dw, dh), img)
    p.end()
    return pm


def _clear(lay: QtWidgets.QLayout):
    while lay.count():
        w = lay.takeAt(0).widget()
        if w:
            w.hide()
            w.setParent(None)
            w.deleteLater()


SORTS = ("rec", "high", "new", "down")


class CursorEditor(QtWidgets.QWidget):
    """Mauszeiger: suchen, installierte wählen, neue aus dem Netz installieren (Empfohlen = GitHub-Releases,
    sonst KDE-Store-Katalog sortiert nach Bewertung, neueste oder Downloads), Größe, auch für GTK-Apps."""

    def __init__(self, win):
        super().__init__()
        self.win = win
        self.job = None
        self.fetch = None
        self.pics = None
        self.gen = 0
        self.page_no, self.total, self.store_items = 0, 0, []
        self.state = {}
        self.segs = []
        self.btn_apply = _action(True)
        self.btn_apply.clicked.connect(self.apply)
        self.actions = [self.btn_apply]
        lay = _scroll(self)

        self.search = QtWidgets.QLineEdit()
        self.search.setClearButtonEnabled(True)
        self.search.setMinimumHeight(40)
        self.search.textChanged.connect(self._search_changed)
        lay.addWidget(self.search)
        self._search_timer = QtCore.QTimer(self)
        self._search_timer.setSingleShot(True)
        self._search_timer.setInterval(500)
        self._search_timer.timeout.connect(self.reload_store)

        # Installiert
        self.inst_card = Card("cur.title", self)
        self.btn_refresh, self.btn_open = _btn(), _btn()
        self.btn_refresh.clicked.connect(self.load)
        self.btn_open.clicked.connect(lambda: online.open_dir(online.icons_dir()))
        card_header(self.inst_card, self.btn_refresh, self.btn_open)
        self.list = QtWidgets.QListWidget()
        self.list.setObjectName("ModList")
        self.list.setMinimumHeight(200)
        self.list.currentItemChanged.connect(self._picked)
        self.inst_card.layout().addWidget(self.list)
        grid = QtWidgets.QGridLayout()
        grid.setHorizontalSpacing(14)
        grid.setVerticalSpacing(10)
        self.l_size = QtWidgets.QLabel()
        self.l_size.setObjectName("RowLabel")
        self.l_size.setMinimumWidth(130)
        self.size = QtWidgets.QSlider(Qt.Orientation.Horizontal)
        self.size.setRange(8, 32)  # ×2 → 16 … 64
        self.size_val = QtWidgets.QLabel()
        self.size_val.setObjectName("Value")
        self.size_val.setMinimumWidth(44)
        self.size_val.setAlignment(Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignVCenter)
        self.size.valueChanged.connect(lambda v: self.size_val.setText(f"{v * 2} px"))
        self.l_gtk = QtWidgets.QLabel()
        self.l_gtk.setObjectName("RowLabel")
        self.gtk = Switch()
        self.gtk.set_now(bool(win.cfg.get("cursor_gtk", True)))
        grid.addWidget(self.l_size, 0, 0)
        grid.addWidget(self.size, 0, 1)
        grid.addWidget(self.size_val, 0, 2)
        grid.addWidget(self.l_gtk, 1, 0)
        grid.addWidget(self.gtk, 1, 1, Qt.AlignmentFlag.AlignLeft)
        grid.setColumnStretch(1, 1)
        self.inst_card.layout().addLayout(grid)
        lay.addWidget(self.inst_card)

        # Online installieren
        self.web_card = Card("cur.online", self)
        from .widgets import Segmented
        self.sort = Segmented(SORTS, prefix="cur.sort.")
        self.sort.changed.connect(self._sort_changed)
        self.segs.append(self.sort)
        card_header(self.web_card, self.sort)
        self.web_state = QtWidgets.QLabel()
        self.web_state.setObjectName("RowLabel")
        self.web_card.layout().addWidget(self.web_state)
        self.rows_lay = QtWidgets.QVBoxLayout()
        self.rows_lay.setSpacing(8)
        self.web_card.layout().addLayout(self.rows_lay)
        self.btn_more = _btn()
        self.btn_more.clicked.connect(lambda: self.reload_store(more=True))
        self.web_card.layout().addWidget(self.btn_more, 0, Qt.AlignmentFlag.AlignHCenter)
        lay.addWidget(self.web_card)
        lay.addStretch()
        self.sort_key = win.cfg.get("cursor_sort", "rec") if win.cfg.get("cursor_sort") in SORTS else "rec"

    # ----- Laden -----
    def refresh(self):
        self.load()
        self.show_online()

    def load(self):
        hs = self.win.hypr_page.state
        self.list.blockSignals(True)
        self.list.clear()
        items = paper.cursor_themes()
        sel = None
        for name, kind in items:
            it = QtWidgets.QListWidgetItem(f"{name}    ·  {t('cur.' + kind)}")
            it.setData(Qt.ItemDataRole.UserRole, name)
            self.list.addItem(it)
            if name == hs["cursor_theme"]:
                sel = it
        if not items:
            it = QtWidgets.QListWidgetItem(t("cur.none"))
            it.setFlags(Qt.ItemFlag.NoItemFlags)
            self.list.addItem(it)
        if sel:
            self.list.setCurrentItem(sel)
        self.list.blockSignals(False)
        self.size.setValue(max(8, min(32, hs["cursor_size"] // 2)))
        self.size_val.setText(f"{self.size.value() * 2} px")
        self._filter_installed()

    def _filter_installed(self):
        q = self.search.text().strip().lower()
        for i in range(self.list.count()):
            it = self.list.item(i)
            name = (it.data(Qt.ItemDataRole.UserRole) or "").lower()
            it.setHidden(bool(q) and bool(name) and q not in name)

    def _search_changed(self, *_):
        self._filter_installed()
        if self.sort_key == "rec":
            self.show_online()
        else:
            self._search_timer.start()

    def _sort_changed(self, key):
        self.sort_key = key
        self.win.cfg["cursor_sort"] = key
        self.win.save_later()
        self.show_online()

    # ----- Online -----
    def _row(self, title: str, meta: str, key: str) -> tuple:
        row = QtWidgets.QFrame()
        row.setObjectName("BindRow")
        rl = QtWidgets.QHBoxLayout(row)
        rl.setContentsMargins(10, 8, 10, 8)
        rl.setSpacing(12)
        pic = QtWidgets.QLabel()
        pic.setFixedSize(112, 64)
        rl.addWidget(pic)
        col = QtWidgets.QVBoxLayout()
        col.setSpacing(2)
        name = QtWidgets.QLabel(title)
        name.setStyleSheet("font-weight:600; background:transparent;")
        sub = QtWidgets.QLabel(meta)
        sub.setObjectName("RowLabel")
        col.addStretch()
        col.addWidget(name)
        col.addWidget(sub)
        col.addStretch()
        rl.addLayout(col, 1)
        chip = QtWidgets.QLabel()
        rl.addWidget(chip)
        btn = _btn(True)
        btn.setMinimumWidth(130)
        btn.setText(t("cur.install"))
        btn.setEnabled(self.job is None)
        rl.addWidget(btn)
        self.rows_lay.addWidget(row)
        self.row_pics[key] = pic
        return row, chip, btn

    def show_online(self):
        self.gen += 1
        if self.pics:
            self.pics.requestInterruption()
        self.sort.select(self.sort_key)
        _clear(self.rows_lay)
        self.row_pics = {}
        self.btn_more.hide()
        if self.sort_key != "rec":
            self.store_items, self.page_no, self.total = [], 0, 0
            self.reload_store()
            return
        q = self.search.text().strip().lower()
        hits = 0
        for e in online.CURSORS:
            if q and q not in e[1].lower() and not any(q in n.lower() for n in e[3]):
                continue
            hits += 1
            src = "github.com/" + e[2].split("github.com/")[1].split("/releases")[0]
            row, chip, btn = self._row(e[1], src, e[0])
            row.layout().itemAt(0).widget().hide()  # keine Vorschau für GitHub-Releases
            inst = online.installed_cursor(e)
            set_chip(chip, True, t("cur.installed"))
            chip.setVisible(inst)
            btn.setVisible(not inst)
            btn.clicked.connect(lambda _=False, x=e: self._install(x[1], lambda log, x=x: online.install_cursor(x, log)))
        self.web_state.setText(t("cur.nomatch"))
        self.web_state.setVisible(hits == 0)

    def reload_store(self, more: bool = False):
        if self.sort_key == "rec":
            return
        if not more:
            self.gen += 1
            _clear(self.rows_lay)
            self.row_pics, self.store_items, self.page_no = {}, [], 0
        else:
            self.page_no += 1
        gen, q, sort, page = self.gen, self.search.text(), self.sort_key, self.page_no
        self.web_state.setText(t("cur.loading_list"))
        self.web_state.show()
        self.btn_more.hide()
        self.fetch = Job(lambda _log: online.store_search(q, sort, page), self)
        self.fetch.result.connect(lambda r, g=gen: self._store_loaded(g, r))
        self.fetch.start()

    def _store_loaded(self, gen, r):
        if gen != self.gen:
            return
        if isinstance(r, Exception):
            self.web_state.setText(t("cur.offline"))
            return
        items, self.total = r
        self.web_state.setVisible(not items and not self.store_items)
        self.web_state.setText(t("cur.nomatch"))
        pics = []
        for it in items:
            date = ".".join(reversed(it["changed"].split("-"))) if it["changed"] else ""
            meta = t("cur.meta", score=f"{it['score'] / 10:.1f}", dl=f"{it['downloads']:,}".replace(",", "."),
                     date=date)
            row, chip, btn = self._row(it["name"], meta, it["id"])
            chip.hide()
            btn.clicked.connect(lambda _=False, x=it, b=btn: self._store_click(x, b))
            if it["preview"]:
                pics.append((it["id"], it["preview"]))
        self.store_items += items
        self.btn_more.setText(t("cur.more"))
        self.btn_more.setVisible((self.page_no + 1) * 10 < self.total and bool(items))
        if pics:
            self.pics = PicJob(pics, self)
            self.pics.got.connect(lambda k, data, g=gen: self._pic(g, k, data))
            self.pics.start()

    def _pic(self, gen, key, data):
        lb = self.row_pics.get(key)
        if gen == self.gen and lb is not None:
            lb.setPixmap(_pic(data, 112, 64))

    def _store_click(self, item, btn):
        if len(item["files"]) == 1:
            i = item["files"][0][0]
            self._install(item["name"], lambda log: online.store_install(item, i, log))
            return
        m = QtWidgets.QMenu(self)  # mehrere Varianten (Farben, Größen) → auswählen
        for i, name, size in item["files"]:
            a = m.addAction(f"{name}   ·  {size} KB" if size else name)
            a.triggered.connect(lambda _=False, i=i: self._install(item["name"],
                                                                   lambda log: online.store_install(item, i, log)))
        m.exec(btn.mapToGlobal(QtCore.QPoint(0, btn.height() + 4)))

    def _install(self, label, fn):
        if self.job:
            return
        self.win.log("")
        self.win.set_status(t("dl.loading", name=label))
        self.job = Job(fn, self)
        self._busy(True)
        self.job.line.connect(self.win.log)
        self.job.result.connect(lambda r, x=label: self._installed(x, r))
        self.job.start()

    def _busy(self, on: bool):
        for i in range(self.rows_lay.count()):
            w = self.rows_lay.itemAt(i).widget()
            for b in w.findChildren(QtWidgets.QPushButton) if w else []:
                b.setEnabled(not on)

    def _installed(self, label, r):
        self.job = None
        self._busy(False)
        if isinstance(r, Exception):
            self.win.log(f"Fehler: {r}")
            self.win.set_status(t("dl.fail", err=r))
            return
        self.load()
        if self.sort_key == "rec":
            self.show_online()
        for i in range(self.list.count()):  # neues Thema gleich auswählen
            if r and self.list.item(i).data(Qt.ItemDataRole.UserRole) == r[0]:
                self.list.setCurrentRow(i)
        self.win.set_status(t("cur.got", name=label) if r else t("cur.nothing_new", name=label))

    def _picked(self, cur, _prev):
        if cur and cur.data(Qt.ItemDataRole.UserRole):
            self.win.set_status(t("cur.picked", name=cur.data(Qt.ItemDataRole.UserRole)))

    def apply(self):
        it = self.list.currentItem()
        theme = it.data(Qt.ItemDataRole.UserRole) if it else ""
        size = self.size.value() * 2
        hp = self.win.hypr_page
        hp.state["cursor_theme"] = theme or hp.state["cursor_theme"]
        hp.state["cursor_size"] = size
        self.win.cfg["cursor_gtk"] = self.gtk.isChecked()
        hp.refresh_controls()
        hp.changed()
        hp.apply()
        paper.set_cursor_live(hp.state["cursor_theme"], size, self.gtk.isChecked(), self.win.log)
        self.win.set_status(t("cur.done", name=hp.state["cursor_theme"] or "–", size=size))

    def retranslate(self):
        self.inst_card.retranslate()
        self.web_card.retranslate()
        self.sort.retranslate()
        self.search.setPlaceholderText(t("cur.search"))
        self.btn_refresh.setText(t("bak.refresh"))
        self.btn_open.setText(t("cur.open"))
        self.btn_more.setText(t("cur.more"))
        self.l_size.setText(t("cursor_size"))
        self.l_gtk.setText(t("cur.gtk"))
        self.btn_apply.setText(t("apply"))
        self.load()
        self.sort.select(self.sort_key)

    def apply_theme(self, accent):
        self.gtk.accent = accent
        self.gtk.update()
