"""Tab „SDDM“: Login-Manager-Leiste + Login-Themes (lokal und KDE Store)."""

from . import sddm, system
from .i18n import t
from .qt import QtWidgets, Qt
from .theme_page import ThemePage
from .widgets import ChipRow


class SDDMPage(ThemePage):
    mod = sddm
    act = "sddm"
    style = "login"
    prompt_title = "sddm.prompt"
    prompt_button = "dm.install"
    prompt_args = ("dm-install",)
    has_test = True
    default_theme = "breeze"

    def build_bar(self, layout):
        # Login-Manager-Leiste: SDDM installieren / als Login-Manager setzen
        dmbar = QtWidgets.QFrame()
        dmbar.setObjectName("DmBar")
        dmbar.setFixedHeight(54)
        dl = QtWidgets.QHBoxLayout(dmbar)
        dl.setContentsMargins(16, 8, 10, 8)
        dl.setSpacing(10)
        self.dm_label = QtWidgets.QLabel()
        self.dm_label.setObjectName("Label")
        self.dm_chips = ChipRow()
        self.btn_dm_enable = QtWidgets.QPushButton()
        self.btn_dm_enable.setObjectName("Primary")
        self.btn_dm_enable.setCursor(Qt.CursorShape.PointingHandCursor)
        self.btn_dm_enable.setMinimumHeight(34)
        self.btn_dm_enable.clicked.connect(lambda: self.host.run(["dm-enable", "sddm"], "dm-enable", self))
        dl.addWidget(self.reset_button())
        dl.addWidget(self.dm_label)
        dl.addWidget(self.dm_chips, 1)
        dl.addWidget(self.btn_dm_enable)
        layout.addWidget(dmbar)

    def refresh_bar(self):
        active = system.active_display_manager()
        have = system.sddm_installed()
        self.dm_label.setText(t("dm.label"))
        self._b_reset.setToolTip(t("reset.tip"))
        kind = "ChipOk" if active == "sddm" else "ChipAccent"
        self.dm_chips.set([(system.dm_pretty(active), kind, f"{active}.service" if active else "")])
        self.btn_dm_enable.setText(t("dm.enable"))
        self.btn_dm_enable.setVisible(have and active != "sddm")
        self.btn_dm_enable.setEnabled(not self.host.busy())
