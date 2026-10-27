"""Global search in the header: pages, settings rows, desktops and themes. The index is built in the background
at start (file system scans), filtering is instant (QCompleter on a ready model)."""

from . import catalog, desktops, i18n, looks, plymouth, sddm
from .form import Row, start_job
from .i18n import t
from .qt import QtCore, QtGui, QtWidgets, Qt

# i18n key prefix → (desktop, section[, sub page]); first match wins
PLACES = (
    (("ala.",), ("*", "advanced", "alacritty")), (("adv.yay",), ("*", "advanced", "yay")),
    (("adv.kitty",), ("*", "advanced", "kitty")),
    (("gn.look", "gn.gtk", "gn.shell", "look.scheme", "look.accent", "look.anim"), ("gnome", "look")),
    (("font.ui", "font.doc", "font.mono", "font.title", "font.hint", "font.aa", "font.scale"), ("gnome", "fonts")),
    (("win.", "gn.windows", "gn.corners", "gn.files"), ("gnome", "windows")),
    (("gn.titlebar", "gn.title", "gn.btn", "gn.left", "gn.right", "gn.sup"), ("gnome", "titlebar")),
    (("gn.topbar", "gn.top.", "top.", "bar.", "pw.", "qs."), ("gnome", "topbar")),
    (("in.", "gn.input"), ("gnome", "input")),
    (("gn.autostart", "gn.auto.add"), ("gnome", "autostart")),
    (("gn.login",), ("gnome", "login")),
    (("pl.",), ("plasma", "plasma")),
)
# option values and messages, no settings of their own
SKIP = ("win.dbl.", "win.mid.", "win.right.", "win.focus.", "win.modifier.", "look.accent.", "look.scheme.",
        "in.accel.", "in.overlay.", "font.hint.", "font.aa.", "top.format.", "font.cat.", "font.f",
        "gn.login.nogdm", "gn.login.banner.ph", "adv.yay.ok", "adv.yay.no", "adv.yay.arch", "adv.yay.s",
        "adv.yay.done", "adv.kitty.missing", "adv.alacritty.missing", "ala.applied", "gn.btns.note",
        "gn.btns.restart", "gn.btn.noclose", "gn.sup.")


def place(key: str):
    if key.endswith(".tip") or key.startswith(SKIP):
        return None
    return next((p for pre, p in PLACES if key.startswith(pre)), None)


def scan() -> dict:
    """Runs in the background: everything that touches the disk."""
    out = {"de": [], "sddm": [], "ply": [], "cursor": [], "icon": []}
    for d in catalog.DESKTOPS:
        e = desktops.get(d.id)
        out["de"].append((d.id, d.name, bool(e and desktops.installed(e))))
    for key, fn in (("sddm", sddm.list_themes), ("ply", plymouth.list_themes)):
        try:
            out[key] = [th["name"] for th in fn()]
        except Exception:  # noqa: BLE001  (Index ohne diese Themes)
            pass
    for kind in ("cursor", "icon"):
        try:
            out[kind] = sorted(looks.list_installed(kind))
        except Exception:  # noqa: BLE001
            pass
    return out


def entries(data: dict, sections: dict, current_de: str) -> list:
    """(text, where, target) – target: ("tab", t) | ("de", id) | ("sec", de, key, sub, rowkey) | ("theme", tab, name)."""
    from .gui import TABS
    names = {d: n for d, n, _ok in data["de"]}
    inst = [d for d, _n, ok in data["de"] if ok]
    res = [(t("tab." + k), "", ("tab", k)) for k in TABS]
    res += [(n, t("tab.de"), ("de", d)) for d, n, _ok in data["de"]]
    for de in inst:
        for key in sections.get(de, ()):
            res.append((t("sec." + key), f"{t('tab.settings')} › {names.get(de, de)}", ("sec", de, key, "", "")))
    adv_de = current_de if current_de in inst else (inst[0] if inst else "")
    for sub in ("yay", "kitty", "alacritty"):
        if adv_de:
            res.append((t("adv." + sub), f"{t('tab.settings')} › {names.get(adv_de, adv_de)} › {t('sec.advanced')}",
                        ("sec", adv_de, "advanced", sub, "")))
    for key, text in i18n.TEXT[i18n.lang()].items():
        p = place(key)
        if not p or "{" in text or len(text) > 48:
            continue
        de = adv_de if p[0] == "*" else p[0]
        if de not in inst:
            continue
        sub = p[2] if len(p) > 2 else ""
        where = f"{t('tab.settings')} › {names.get(de, de)} › {t('sec.' + p[1])}"
        res.append((text, where, ("sec", de, p[1], sub, key)))
    for tab, label in (("sddm", t("tab.sddm")), ("ply", t("tab.ply"))):
        res += [(n, label, ("theme", tab, n)) for n in data[tab]]
    look_de = current_de if current_de in inst else adv_de
    for kind in ("cursor", "icon"):
        if look_de:
            res += [(n, f"{t('tab.settings')} › {t('sec.' + kind)}", ("sec", look_de, kind, "", "")) for n in data[kind]]
    seen, out = set(), []
    for text, where, target in res:              # doppelte Treffer (gleicher Text, gleicher Ort) nur einmal
        k = (text.lower(), tuple(target[:4]))
        if k not in seen:
            seen.add(k)
            out.append((text, where, target))
    return out


class SearchBox(QtWidgets.QLineEdit):
    def __init__(self, host):
        super().__init__()
        self.host = host
        self.setObjectName("Search")
        self.setClearButtonEnabled(True)
        self.setFixedWidth(240)
        from .widgets import draw_icon
        from . import theme
        self.addAction(draw_icon("search", theme.MUTED, 16), QtWidgets.QLineEdit.ActionPosition.LeadingPosition)
        self.model = QtGui.QStandardItemModel(self)
        self.comp = QtWidgets.QCompleter(self.model, self)
        self.comp.setCaseSensitivity(Qt.CaseSensitivity.CaseInsensitive)
        self.comp.setFilterMode(Qt.MatchFlag.MatchContains)
        self.comp.setCompletionRole(Qt.ItemDataRole.UserRole + 2)   # Suchtext = Name + Ort
        self.comp.setMaxVisibleItems(12)
        self.comp.activated[QtCore.QModelIndex].connect(self._go)
        self.setCompleter(self.comp)
        pop = self.comp.popup()
        pop.setMinimumWidth(420)
        pop.setObjectName("SearchPop")
        self.data = None
        self.returnPressed.connect(self._first)
        sc = QtGui.QShortcut(QtGui.QKeySequence.StandardKey.Find, host)    # Strg+F
        sc.activated.connect(lambda: (self.setFocus(), self.selectAll()))
        start_job(self, scan, self._scanned)

    def _scanned(self, ok, data):
        if ok:
            self.data = data
            self.rebuild()

    def rebuild(self):
        if not self.data:
            return
        from .settings_page import DEFAULT_SECTIONS, SECTIONS
        secs = {d: SECTIONS.get(d, DEFAULT_SECTIONS) for d, _n, _ok in self.data["de"]}
        cur = desktops.current() or ""
        self.model.clear()
        for text, where, target in entries(self.data, secs, cur or "gnome"):
            it = QtGui.QStandardItem(f"{text}    ·  {where}" if where else text)
            it.setData(target, Qt.ItemDataRole.UserRole + 1)
            it.setData(f"{text} {where}", Qt.ItemDataRole.UserRole + 2)
            it.setData(f"{text}    ·  {where}" if where else text, Qt.ItemDataRole.DisplayRole)
            self.model.appendRow(it)

    def retranslate(self):
        self.setPlaceholderText(t("search.all"))
        self.rebuild()

    def _first(self):
        m = self.comp.completionModel()
        if m.rowCount():
            self._go(m.index(0, 0))

    def _go(self, index):
        target = index.data(Qt.ItemDataRole.UserRole + 1)
        if not target:
            return
        QtCore.QTimer.singleShot(0, self.clear)
        go(self.host, target)


def go(host, target):
    kind = target[0]
    if kind == "tab":
        host.set_tab(target[1])
    elif kind == "de":
        host.set_tab("de")
        host.de_page.select(target[1])
    elif kind == "theme":
        host.set_tab(target[1])
        page = host.sddm_page if target[1] == "sddm" else host.ply_page
        if hasattr(page, "search_edit"):
            page.search_edit.setText(target[2])
    elif kind == "sec":
        _, de, key, sub, rowkey = target
        host.set_tab("settings")
        host.settings_page.select(de)
        panel = host.settings_page.panel()
        if panel is None:
            return
        panel.show_section(key)
        pg = panel.pages.get(key)
        if sub and hasattr(pg, "show_sub"):
            pg.show_sub(sub)
            pg = pg.pages.get(sub, pg)
        if rowkey and pg is not None:
            QtCore.QTimer.singleShot(60, lambda: _reveal(pg, rowkey))


def _reveal(page, key):
    """Scroll the row into view and flash it briefly."""
    w = next((r for r in page.findChildren(Row) if r.key == key), None)
    if w is None:
        w = next((c for c in page.findChildren(QtWidgets.QFrame) if getattr(c, "title_key", None) == key), None)
    if w is None:
        return
    sa = w.parent()
    while sa is not None and not isinstance(sa, QtWidgets.QScrollArea):
        sa = sa.parent()
    if sa is not None:
        sa.ensureWidgetVisible(w, 0, 60)
    from . import theme
    lb = getattr(w, "label", None) or getattr(w, "head", None)
    if lb is None:
        return
    old = lb.styleSheet()
    lb.setStyleSheet(old + f" color: {theme.TEXT}; background: {theme.RAISED}; border-radius: 6px;")
    QtCore.QTimer.singleShot(1400, lambda: lb.setStyleSheet(old))
