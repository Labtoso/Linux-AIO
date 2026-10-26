"""Tab „Einstellungen“: links alle Desktops (installierte wählbar, andere ausgegraut, laufender vorgewählt),
rechts die Einstellungen des gewählten Desktops."""

from . import desktops, theme
from .form import accent_switches
from .i18n import t
from .qt import QtCore, QtGui, QtWidgets, Qt
from .widgets import Segmented

SECTIONS = {
    "gnome": ("look", "cursor", "icon", "fonts", "windows", "titlebar", "topbar", "input", "autostart", "login"),
    "plasma": ("plasma", "cursor", "icon"),
    "hyprland": ("hyprstudio", "launcher", "cursor", "icon"),
    "app": ("app",),
}
DEFAULT_SECTIONS = ("cursor", "icon")


def dot_icon(color: str, size: int = 10) -> QtGui.QIcon:
    pm = QtGui.QPixmap(size * 2, size * 2)
    pm.fill(Qt.GlobalColor.transparent)
    p = QtGui.QPainter(pm)
    p.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
    p.setPen(Qt.PenStyle.NoPen)
    p.setBrush(QtGui.QColor(color))
    p.drawEllipse(QtCore.QRectF(size * 0.5, size * 0.5, size, size))
    p.end()
    return QtGui.QIcon(pm)


def make_section(host, de: str, key: str):
    from . import gnome_page as G
    from .look_page import LookPage
    if key in ("cursor", "icon"):
        return LookPage(host, key, de)
    if key == "plasma":
        from .plasma_page import PlasmaLook
        return PlasmaLook(host)
    if key == "hyprstudio":
        from .hypr_embed import HyprEmbed, HyprLauncher
        return HyprLauncher(host) if host.cfg.get("hypr_mode") == "window" else HyprEmbed(host)
    if key == "launcher":                     # Rofi, Wofi, Fuzzel, Walker, Tofi, L Launcher
        from .hypr_embed import HyprEmbed
        return HyprEmbed(host, launcher_only=True)
    if key == "app":
        from .app_page import AppPage
        return AppPage(host)
    if key == "look":
        return G.LookGnome(host)
    if key == "titlebar":
        return G.TitlebarPage(host)
    if key == "windows":
        return G.WindowsPage(host)
    if key == "fonts":
        from .font_page import FontsPage
        return FontsPage(host)
    if key == "topbar":
        return G.TopbarPage(host)
    if key == "autostart":
        return G.AutostartPage(host)
    if key == "login":
        return G.LoginPage(host)
    return G.simple_page(key, key)


class DePanel(QtWidgets.QWidget):
    """Einstellungen eines Desktops: Seitenleiste (bzw. Segmente bei Hyprland) + Bereiche."""

    def __init__(self, host, de: str, parent=None):
        super().__init__(parent)
        self.host, self.de = host, de
        self.keys = SECTIONS.get(de, DEFAULT_SECTIONS)
        self.pages = {}
        self.top = False                     # überall die Bereichsliste links (auch Hyprland)
        lay = (QtWidgets.QVBoxLayout if self.top else QtWidgets.QHBoxLayout)(self)
        lay.setContentsMargins(0, 0, 0, 0)
        lay.setSpacing(14)
        if self.top:
            row = QtWidgets.QHBoxLayout()
            self.seg = Segmented(self.keys, prefix="sec.")
            self.seg.changed.connect(self.show_section)
            row.addWidget(self.seg)
            row.addStretch()
            lay.addLayout(row)
            self.nav = None
        else:
            self.nav = QtWidgets.QListWidget()
            self.nav.setObjectName("NavList")
            self.nav.setFixedWidth(176)
            self.nav.setVerticalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAsNeeded)
            for k in self.keys:
                it = QtWidgets.QListWidgetItem()
                it.setData(Qt.ItemDataRole.UserRole, k)
                it.setSizeHint(QtCore.QSize(0, 38))
                self.nav.addItem(it)
            self.nav.currentRowChanged.connect(lambda r: self.show_section(self.keys[r]) if r >= 0 else None)
            lay.addWidget(self.nav)
            self.nav.setVisible(len(self.keys) > 1)      # ein Bereich: keine Seitenleiste
        self.stack = QtWidgets.QStackedWidget()
        lay.addWidget(self.stack, 1)
        self.current = None
        self.retranslate()
        start = host.cfg.get(f"section_{de}")
        self.show_section(start if start in self.keys else self.keys[0])

    def show_section(self, key: str):
        if key not in self.pages:
            pg = make_section(self.host, self.de, key)
            self.pages[key] = pg
            self.stack.addWidget(pg)
            if hasattr(pg, "retranslate"):
                pg.retranslate()
            accent_switches(pg, self.host.accent())
        pg = self.pages[key]
        self.stack.setCurrentWidget(pg)
        self.current = key
        if self.nav is not None:
            i = self.keys.index(key)
            if self.nav.currentRow() != i:
                self.nav.blockSignals(True)
                self.nav.setCurrentRow(i)
                self.nav.blockSignals(False)
        else:
            self.seg.select(key)
        if key in ("hyprstudio", "launcher"):
            pg.sync_look(self.host.cfg.get("lang", "en"), self.host.cfg.get("theme", "blue"))
        elif hasattr(pg, "refresh"):
            pg.refresh()
        self.host.cfg[f"section_{self.de}"] = key
        self.host.save()

    def retranslate(self):
        if self.nav is not None:
            for i, k in enumerate(self.keys):
                self.nav.item(i).setText(t("sec." + k))
        else:
            self.seg.retranslate()
        for k, pg in self.pages.items():
            if k in ("hyprstudio", "launcher"):
                pg.sync_look(self.host.cfg.get("lang", "de"), self.host.cfg.get("theme", "blue"))
            elif hasattr(pg, "retranslate"):
                pg.retranslate()
                if k == self.current and hasattr(pg, "refresh"):
                    pg.refresh()

    def apply_accent(self):
        for k, pg in self.pages.items():
            if k in ("hyprstudio", "launcher"):
                pg.sync_look(self.host.cfg.get("lang", "de"), self.host.cfg.get("theme", "blue"))
            else:
                accent_switches(pg, self.host.accent())
                if k == self.current and k in ("cursor", "icon"):
                    pg.refresh()

    def refresh_busy(self):
        pg = self.pages.get(self.current)
        if pg is not None and self.current in ("cursor", "icon", "login"):
            pg.refresh()


class SettingsPage(QtWidgets.QWidget):
    def __init__(self, host):
        super().__init__()
        self.host = host
        self.panels = {}
        self.entries = desktops.entries()
        self.running = desktops.current()
        lay = QtWidgets.QHBoxLayout(self)
        lay.setContentsMargins(0, 0, 0, 0)
        lay.setSpacing(16)
        side = QtWidgets.QFrame()
        side.setObjectName("SideBox")
        side.setFixedWidth(180)
        sl = QtWidgets.QVBoxLayout(side)
        sl.setContentsMargins(10, 12, 10, 10)
        sl.setSpacing(8)
        self.side_title = QtWidgets.QLabel()
        self.side_title.setObjectName("Label")
        sl.addWidget(self.side_title)
        self.list = QtWidgets.QListWidget()
        self.list.setObjectName("DeList")
        self.list.setIconSize(QtCore.QSize(10, 10))
        for e in self.entries:
            it = QtWidgets.QListWidgetItem(e.name)
            it.setData(Qt.ItemDataRole.UserRole, e.id)
            it.setSizeHint(QtCore.QSize(0, 36))
            self.list.addItem(it)
        self.list.currentRowChanged.connect(self._row)
        sl.addWidget(self.list, 1)
        lay.addWidget(side)
        self.stack = QtWidgets.QStackedWidget()
        lay.addWidget(self.stack, 1)
        self.empty = QtWidgets.QLabel()
        self.empty.setObjectName("Label")
        self.empty.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.stack.addWidget(self.empty)
        self.mark()
        start = self.running if self.running and self._installed(self.running) else host.cfg.get("settings_de")
        if not (start and self._installed(start)):
            start = next((e.id for e in self.entries if desktops.installed(e)), None)
        if start == "app":
            start = None
        if start:
            self.select(start)

    def _installed(self, de_id: str) -> bool:
        if de_id == "app":
            return True
        e = desktops.get(de_id)
        return bool(e and desktops.installed(e))

    def mark(self):
        """Installierte wählbar, nicht installierte ausgegraut; Punkt = läuft gerade."""
        acc = self.host.accent()
        for i, e in enumerate(self.entries):
            it = self.list.item(i)
            inst = e.id == "app" or desktops.installed(e)
            flags = Qt.ItemFlag.ItemIsEnabled | Qt.ItemFlag.ItemIsSelectable if inst else Qt.ItemFlag.NoItemFlags
            it.setFlags(flags)
            it.setIcon(dot_icon(acc if e.id == self.running else ("#00000000" if inst else "#00000000")))
            tip = t("set.running") if e.id == self.running else (t("installed") if inst else t("set.notinstalled"))
            it.setToolTip(tip)
            it.setForeground(QtGui.QColor(theme.TEXT if inst else "#4d5a6b"))

    def _row(self, r: int):
        if r < 0:
            return
        self.select(self.entries[r].id)

    def select(self, de_id: str):
        if not self._installed(de_id):
            return
        if de_id not in self.panels:
            p = DePanel(self.host, de_id)
            self.panels[de_id] = p
            self.stack.addWidget(p)
        self.stack.setCurrentWidget(self.panels[de_id])
        i = next(i for i, e in enumerate(self.entries) if e.id == de_id)
        if self.list.currentRow() != i:
            self.list.blockSignals(True)
            self.list.setCurrentRow(i)
            self.list.blockSignals(False)
        self.host.cfg["settings_de"] = de_id
        self.host.save()

    def panel(self) -> DePanel | None:
        w = self.stack.currentWidget()
        return w if isinstance(w, DePanel) else None

    def drawer_anchor(self):
        return self.stack

    def retranslate(self):
        self.side_title.setText(t("set.desktops"))
        self.empty.setText(t("set.none"))
        self.mark()
        for p in self.panels.values():
            p.retranslate()

    def apply_accent(self):
        self.mark()
        for p in self.panels.values():
            p.apply_accent()

    def refresh(self):
        self.mark()          # frisch installierte Desktops sofort wählbar (läuft nach jeder Aktion)
        p = self.panel()
        if p is not None:
            p.refresh_busy()
