"""KDE Plasma im Tab „Einstellungen“: Farbschema und Plasma-Design (plasma-apply-*)."""

import re

from . import usercmd
from .form import Card, Page, Row, combo, retranslate_buttons, tool_button
from .i18n import t
from .qt import QtWidgets


def _list(cmd: list) -> tuple[list, str]:
    """plasma-apply-colorscheme --list-schemes: ' * BreezeDark (current color scheme)'."""
    out = usercmd.query(cmd) or ""
    names, cur = [], ""
    for ln in out.splitlines():
        m = re.match(r"^\s*\*\s+(.+?)(\s+\((current|aktuell).*\))?\s*$", ln)
        if m:
            names.append(m.group(1))
            if m.group(2):
                cur = m.group(1)
    return names, cur


class PlasmaLook(Page):
    def __init__(self, host):
        super().__init__("pl.look")
        self.host = host
        card = Card()
        self.body.addWidget(card)
        self.scheme = combo()
        self.desktop = combo()
        self.b_scheme = tool_button("apply", primary=True)
        self.b_desktop = tool_button("apply", primary=True)
        self.b_scheme.clicked.connect(lambda: self._apply("plasma-apply-colorscheme", self.scheme))
        self.b_desktop.clicked.connect(lambda: self._apply("plasma-apply-desktoptheme", self.desktop))
        self.rows = []
        for key, cb, b in (("pl.scheme", self.scheme, self.b_scheme), ("pl.desktop", self.desktop, self.b_desktop)):
            w = QtWidgets.QWidget()
            hl = QtWidgets.QHBoxLayout(w)
            hl.setContentsMargins(0, 0, 0, 0)
            hl.addWidget(cb)
            hl.addWidget(b)
            r = Row(key, w)
            card.lay.addWidget(r)
            self.rows.append(r)
        self.finish()

    def _apply(self, tool, cb):
        name = cb.currentText()
        if name:
            self.host.log("")
            ok, _ = usercmd.run([tool, name])
            self.host.set_status(t("done") if ok else t("failed"))

    def retranslate(self):
        super().retranslate()
        for r in self.rows:
            r.retranslate()
        retranslate_buttons(self)

    def refresh(self):
        for cb, cmd in ((self.scheme, ["plasma-apply-colorscheme", "--list-schemes"]),
                        (self.desktop, ["plasma-apply-desktoptheme", "--list-themes"])):
            names, cur = _list(cmd)
            cb.clear()
            cb.addItems(names)
            if cur:
                cb.setCurrentText(cur)
            cb.setEnabled(bool(names))
        self.b_scheme.setEnabled(self.scheme.count() > 0)
        self.b_desktop.setEnabled(self.desktop.count() > 0)
