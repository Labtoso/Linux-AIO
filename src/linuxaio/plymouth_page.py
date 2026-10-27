"""Tab „Plymouth“: Bootsplash-Themes wie plymouth-kcm (lokal und KDE Store), Einrichtung per Klick."""

from pathlib import Path

from . import plymouth
from .i18n import t
from .qt import QtGui, QtWidgets, Qt
from .theme_page import ThemePage
from .widgets import ChipRow


def _pm(img):
    return QtGui.QPixmap.fromImage(img) if img is not None else None


class PlymouthPage(ThemePage):
    mod = plymouth
    act = "ply"
    style = "boot"
    prompt_title = "ply.prompt"
    prompt_button = "ply.install"
    prompt_args = ("ply-setup",)
    has_test = False
    default_theme = "bgrt"

    def local_animation(self, th, tile: bool = False):
        """Vorschau aus den Bildern des Themes. Kachel: ein fertiges Standbild. Große Vorschau: alle Einzelbilder
        in voller Länge als Schleife – im Hintergrund geladen, damit der Tab-Wechsel nicht hängt."""
        cache = self.__dict__.setdefault("_anim_cache", {})
        key = (th["path"], tile)
        if key in cache:
            return cache[key]
        a = plymouth.preview_assets(Path(th["path"]))
        if not (a["frames"] or a["logo"]):
            cache[key] = None
            return None

        def load(f, limit=700):
            img = QtGui.QImage(f) if f else QtGui.QImage()
            if not img.isNull() and max(img.width(), img.height()) > limit:
                img = img.scaled(limit, limit, Qt.AspectRatioMode.KeepAspectRatio,
                                 Qt.TransformationMode.SmoothTransformation)
            return None if img.isNull() else img

        def build(frames):
            return {"frames": [QtGui.QPixmap.fromImage(i) for i in frames if i is not None],
                    "logo": _pm(load(a["logo"])), "bgimg": _pm(load(a["background"], 1600)),
                    "bg": a["bg"], "bg2": a["bg2"], "ha": a["ha"], "va": a["va"], "wva": a["wva"]}
        if tile:      # Standbild: letztes Bild der Animation (dort ist das Logo komplett)
            still = a["frames"][-1:] if a["frames"] else []
            result = build([load(f) for f in still])
            cache[key] = result
            return result
        first = build([load(a["frames"][-1])] if a["frames"] else [])
        cache[key] = first
        if len(a["frames"]) > 1:
            from .form import start_job
            files = list(a["frames"])
            start_job(self, lambda: [load(f) for f in files],
                      lambda ok, imgs, k=key, p=th["path"]: self._frames_done(ok, imgs, k, p, build))
        return first

    def _frames_done(self, ok, imgs, key, path, build):
        if not ok:
            return
        full = build(imgs)
        self.__dict__.setdefault("_anim_cache", {})[key] = full
        it = self.item() if self.mode == "local" else None
        if it and it["path"] == path:
            self.big.set_animation(full)

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
        self.ply_chips.tip_target = self.ply_label
        self.btn_setup = QtWidgets.QPushButton()
        self.btn_setup.setObjectName("Primary")
        self.btn_setup.setCursor(Qt.CursorShape.PointingHandCursor)
        self.btn_setup.setMinimumHeight(34)
        self.btn_setup.clicked.connect(lambda: self.host.run(["ply-setup"], "ply-setup", self))
        bl.addWidget(self.reset_button())
        bl.addWidget(self.ply_label)
        bl.addWidget(self.ply_chips, 1)
        bl.addWidget(self.btn_setup)
        layout.addWidget(bar)

    def refresh_bar(self):
        have = plymouth.is_installed()
        hook = plymouth.has_hook()
        splash = plymouth.splash_active()
        self.ply_label.setText(t("ply.label"))
        self._b_reset.setToolTip(t("reset.tip"))
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
