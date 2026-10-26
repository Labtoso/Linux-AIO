"""Tab „Plymouth“: Bootsplash-Themes wie plymouth-kcm (lokal und KDE Store), Einrichtung per Klick."""

from pathlib import Path

from . import plymouth
from .i18n import t
from .qt import QtGui, QtWidgets, Qt
from .theme_page import ThemePage
from .widgets import ChipRow


class PlymouthPage(ThemePage):
    mod = plymouth
    act = "ply"
    style = "boot"
    prompt_title = "ply.prompt"
    prompt_button = "ply.install"
    prompt_args = ("ply-setup",)
    has_test = False

    def local_animation(self, th):
        """Animierte Vorschau aus den Bildern des installierten Themes (wird pro Theme zwischengespeichert)."""
        cache = self.__dict__.setdefault("_anim_cache", {})
        if th["path"] in cache:
            return cache[th["path"]]
        a = plymouth.preview_assets(Path(th["path"]))
        result = None
        if a["frames"] or a["logo"]:
            def load(f, limit=700):
                pm = QtGui.QPixmap(f) if f else None
                if pm is not None and not pm.isNull() and max(pm.width(), pm.height()) > limit:
                    pm = pm.scaled(limit, limit, Qt.AspectRatioMode.KeepAspectRatio,
                                   Qt.TransformationMode.SmoothTransformation)
                return pm if pm is not None and not pm.isNull() else None
            frames = [pm for pm in (load(f) for f in a["frames"]) if pm is not None]
            result = {"frames": frames, "logo": load(a["logo"]), "bgimg": load(a["background"], 1600),
                      "bg": a["bg"], "bg2": a["bg2"], "ha": a["ha"], "va": a["va"], "wva": a["wva"]}
        cache[th["path"]] = result
        return result

    def reload_local(self):
        self.__dict__["_anim_cache"] = {}
        super().reload_local()

    def build_bar(self, layout):
        bar = QtWidgets.QFrame()
        bar.setObjectName("DmBar")
        bar.setFixedHeight(54)
        bl = QtWidgets.QHBoxLayout(bar)
        bl.setContentsMargins(16, 8, 10, 8)
        bl.setSpacing(10)
        self.ply_label = QtWidgets.QLabel()
        self.ply_label.setObjectName("Label")
        self.ply_chips = ChipRow()
        self.btn_setup = QtWidgets.QPushButton()
        self.btn_setup.setObjectName("Primary")
        self.btn_setup.setCursor(Qt.CursorShape.PointingHandCursor)
        self.btn_setup.setMinimumHeight(34)
        self.btn_setup.clicked.connect(lambda: self.host.run(["ply-setup"], "ply-setup", self))
        bl.addWidget(self.ply_label)
        bl.addWidget(self.ply_chips, 1)
        bl.addWidget(self.btn_setup)
        layout.addWidget(bar)

    def refresh_bar(self):
        have = plymouth.is_installed()
        hook = plymouth.has_hook()
        splash = plymouth.splash_active()
        self.ply_label.setText(t("ply.label"))
        chips = [(t("installed") if have else t("ply.missing"), "ChipOk" if have else "ChipWarn", "")]
        if have:
            if hook is not None:
                chips.append(("initramfs-Hook", "ChipOk" if hook else "ChipWarn",
                              "" if hook else t("ply.nohook")))
            chips.append(("splash", "ChipOk" if splash else "ChipWarn", "" if splash else t("ply.nosplash")))
        self.ply_chips.set(chips)
        self.btn_setup.setText(t("ply.setup"))
        self.btn_setup.setVisible(have and (hook is False or not splash))
        self.btn_setup.setEnabled(not self.host.busy())
