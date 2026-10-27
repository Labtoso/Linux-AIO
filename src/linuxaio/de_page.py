"""Tab „Desktops“: Desktop-Umgebungen installieren, als Standard setzen, entfernen."""

import json
import tempfile
from pathlib import Path

from . import catalog, system
from .i18n import t, lang
from .preview import Preview
from .qt import QtWidgets, Qt
from .widgets import DRY, ChipRow, Segmented, Tile, TileStrip, set_applied


def read_state() -> dict:
    p = Path(tempfile.gettempdir()) / "linuxaio-dry-state.json" if DRY else system.STATE_FILE
    return system.load_state_file(p)


class DEPage(QtWidgets.QWidget):
    def __init__(self, host):
        super().__init__()
        self.host = host
        self.state = {}
        self.default_de = None
        self.current = host.running if host.running and host.running != "plasma" else "gnome"
        self.variant = "standard"

        cl = QtWidgets.QVBoxLayout(self)
        cl.setContentsMargins(0, 0, 0, 0)
        cl.setSpacing(14)

        hero = QtWidgets.QFrame()
        hero.setObjectName("Hero")
        hero.setMinimumHeight(280)
        hl = QtWidgets.QHBoxLayout(hero)
        hl.setContentsMargins(18, 18, 18, 18)
        hl.setSpacing(24)
        self.big = Preview(hero, radius=12)
        self.big.setSizePolicy(QtWidgets.QSizePolicy.Policy.Expanding, QtWidgets.QSizePolicy.Policy.Expanding)
        hl.addWidget(self.big, 3)

        side = QtWidgets.QVBoxLayout()
        side.setSpacing(12)
        self.de_name = QtWidgets.QLabel()
        self.de_name.setObjectName("DeName")
        side.addWidget(self.de_name)
        self.chips = ChipRow()
        self.chips.tip_target = self.de_name
        side.addWidget(self.chips)
        side.addSpacing(8)
        self.seg = Segmented(catalog.VARIANTS)
        self.seg.changed.connect(self.set_variant)
        side.addWidget(self.seg, 0, Qt.AlignmentFlag.AlignLeft)
        self.pkg_label = QtWidgets.QLabel()
        self.pkg_label.setObjectName("Label")
        side.addWidget(self.pkg_label)
        side.addStretch()
        self.btn_install = QtWidgets.QPushButton()
        self.btn_install.setObjectName("Primary")
        self.btn_install.clicked.connect(self.do_install)
        self.btn_default = QtWidgets.QPushButton()
        self.btn_default.clicked.connect(self.do_default)
        self.btn_remove = QtWidgets.QPushButton()
        self.btn_remove.setObjectName("Danger")
        self.btn_remove.clicked.connect(self.do_remove)
        for b in (self.btn_install, self.btn_default, self.btn_remove):
            b.setCursor(Qt.CursorShape.PointingHandCursor)
            b.setMinimumHeight(40)
            side.addWidget(b)
        side_w = QtWidgets.QWidget()
        side_w.setLayout(side)
        side_w.setFixedWidth(300)
        hl.addWidget(side_w)
        cl.addWidget(hero, 1)

        self.strip = TileStrip()
        self.tiles = {}
        for de in catalog.DESKTOPS:
            pv = Preview(radius=8)
            pv.set(de, "standard")
            tile = Tile(de.id, de.name, pv)
            tile.clicked.connect(self.select)
            self.tiles[de.id] = tile
            self.strip.add(tile)
        cl.addWidget(self.strip)

    # ----- Anzeige -----
    def drawer_anchor(self):
        return self.strip

    def retranslate(self):
        self.btn_default.setText(t("default"))
        self.btn_remove.setText(t("remove"))
        self.seg.retranslate()
        self.refresh()

    def select(self, de_id: str):
        self.current = de_id
        st = self.state.get(de_id)
        self.variant = st["variant"] if st else "standard"
        self.refresh()

    def set_variant(self, v: str):
        self.variant = v
        self.refresh()

    def refresh(self):
        backend, accent, running = self.host.backend, self.host.accent(), self.host.running
        self.state = read_state()
        self.default_de = system.default_desktop()
        de = catalog.get(self.current)
        installed = {d.id: (system.is_installed(d) or d.id in self.state) for d in catalog.DESKTOPS}
        for d_id, tile in self.tiles.items():
            tile.refresh(d_id == self.current, "#3fb950" if installed[d_id] else None, accent)
            tile.setToolTip(catalog.get(d_id).desc[lang()])
        if not backend or not de.supports(self.variant, backend):
            for v in catalog.VARIANTS:
                if backend and de.supports(v, backend):
                    self.variant = v
                    break
        self.big.set(de, self.variant)
        self.de_name.setText(de.name)
        self.de_name.setProperty("base_tip", de.desc[lang()])

        chips = []
        if de.wayland:
            chips.append((t("wayland"), "Chip", ""))
        if de.x11:
            chips.append((t("x11"), "Chip", ""))
        if de.tiling:
            chips.append((t("tiling"), "Chip", ""))
        if installed[de.id]:
            chips.append((t("installed"), "ChipOk", ""))
        if running == de.id:
            chips.append((t("running"), "ChipAccent", ""))
        elif self.default_de == de.id:
            chips.append((t("isdefault"), "ChipAccent", ""))
        self.chips.set(chips)

        for v, b in self.seg.btns.items():
            ok = bool(backend) and de.supports(v, backend)
            b.setEnabled(ok)
            b.setChecked(v == self.variant)
            specs = de.specs(v, backend) if backend else []
            b.setToolTip(" ".join(s.lstrip("?") for s in specs) if ok else t("unsupported"))
        specs = de.specs(self.variant, backend) if backend else []
        self.pkg_label.setText(f"{t('packages')}: {len(specs)}  ·  {backend or '–'}")
        self.pkg_label.setToolTip("\n".join(specs))

        busy = self.host.busy()
        st = self.state.get(de.id)
        same = bool(st) and st.get("variant") == self.variant
        set_applied(self.btn_install, same, t("installed"))
        if not same:
            self.btn_install.setText(t("reinstall") if installed[de.id] else t("install"))
            self.btn_install.setEnabled(not busy and bool(backend) and de.supports(self.variant, backend))
        is_default = installed[de.id] and self.default_de == de.id
        set_applied(self.btn_default, is_default, t("applied"))
        if not is_default:
            self.btn_default.setText(t("default"))
            self.btn_default.setEnabled(not busy and installed[de.id])
        self.btn_remove.setEnabled(not busy and installed[de.id] and running != de.id)
        self.btn_remove.setToolTip(t("err.running") if running == de.id else "")

    # ----- Aktionen -----
    def do_install(self):
        self.host.run(["install", self.current, self.variant], "install", self)

    def do_default(self):
        self.host.run(["default", self.current], "default", self)

    def do_remove(self):
        if self.host.confirm(t("remove"), t("confirm.remove", name=catalog.get(self.current).name)):
            self.host.run(["remove", self.current, f"--running={self.host.running or ''}"], "remove", self)

    def finished(self, ok: bool, action: str, result: str):
        self.refresh()
