"""Settings › Linux AIO: interface scale and how Hypr Studio opens."""

import os
import sys

from .form import Card, Page, Row, retranslate_buttons, tool_button
from .i18n import t
from .qt import QtCore, QtWidgets
from .widgets import Segmented

SCALES = ("100", "90", "80")


class AppPage(Page):
    def __init__(self, host):
        super().__init__("app.title")
        self.host = host
        card = Card()
        self.body.addWidget(card)
        self.scale = Segmented(SCALES, prefix="app.scale.")
        self.scale.select(str(host.cfg.get("ui_scale", "100")))
        self.scale.changed.connect(self._scale)
        self.r_scale = Row("app.scale", self.scale)
        card.lay.addWidget(self.r_scale)
        self.hmode = Segmented(("embed", "window"), prefix="app.hypr.")
        self.hmode.select(host.cfg.get("hypr_mode", "embed"))
        self.hmode.changed.connect(self._hmode)
        self.r_hypr = Row("app.hypr", self.hmode)
        card.lay.addWidget(self.r_hypr)
        self.b_restart = tool_button("app.restart", primary=True)
        self.b_restart.clicked.connect(restart)
        self.b_restart.hide()
        card.lay.addWidget(self.b_restart, 0, QtCore.Qt.AlignmentFlag.AlignRight)
        self.finish()

    def _scale(self, v):
        self.host.cfg["ui_scale"] = v
        self.host.save()
        self.b_restart.show()
        self.host.set_status(t("app.restart.need"))

    def _hmode(self, v):
        self.host.cfg["hypr_mode"] = v
        self.host.save()
        self.b_restart.show()
        self.host.set_status(t("app.restart.need"))

    def retranslate(self):
        super().retranslate()
        self.r_scale.retranslate()
        self.r_hypr.retranslate()
        self.scale.retranslate()
        self.hmode.retranslate()
        retranslate_buttons(self)


def restart():
    """Start the same copy again (AppImage, .pyz or package) and quit this one."""
    app = os.environ.get("APPIMAGE")
    if app:
        QtCore.QProcess.startDetached(app, sys.argv[1:])
    else:
        QtCore.QProcess.startDetached(sys.executable, sys.argv if sys.argv[0].endswith(".pyz") else ["-m", "linuxaio"])
    QtWidgets.QApplication.instance().quit()
