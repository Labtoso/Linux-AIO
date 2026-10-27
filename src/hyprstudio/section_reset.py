"""Reset button per section (same icon, place and confirmation as everywhere in Linux AIO): only the settings of
the visible section go back to their defaults; the current values are saved as JSON first. Takes effect with Apply,
like every other change in the editor."""

import json
import time
from pathlib import Path

from . import theme
from .i18n import TEXT, t
from .qt import QtCore, QtWidgets, Qt

TEXT["de"].update({"reset.tip": "Auf Standard zurücksetzen", "reset.title": "Auf Standard zurücksetzen",
                   "reset.ask": "„{page}“ auf den Standard zurücksetzen? Vorher wird eine Sicherung angelegt.",
                   "reset.btn": "Zurücksetzen", "reset.section": "„{page}“ zurückgesetzt – Sicherung: {path} · "
                   "Anwenden übernimmt es"})
TEXT["en"].update({"reset.tip": "Reset to default", "reset.title": "Reset to default",
                   "reset.ask": "Reset “{page}” to default? A backup is created.", "reset.btn": "Reset",
                   "reset.section": "“{page}” reset – backup: {path} · Apply to use it"})


def backup_dir() -> Path:
    d = Path.home() / ".config/LabtosoStudios/backups" / time.strftime("%Y-%m-%d_%H%M%S")
    d.mkdir(parents=True, exist_ok=True)
    return d


def button(on_click) -> QtWidgets.QToolButton:
    try:
        from linuxaio.widgets import draw_icon    # dasselbe Kreispfeil-Symbol wie in Linux AIO
    except ImportError:
        from .widgets import draw_icon
    b = QtWidgets.QToolButton()
    b.setObjectName("ResetBtn")
    b.setFixedSize(30, 30)
    b.setIconSize(QtCore.QSize(16, 16))
    b.setIcon(draw_icon("reset", theme.MUTED, 16))
    b.setCursor(Qt.CursorShape.PointingHandCursor)
    b.setToolTip(t("reset.tip"))
    b.clicked.connect(on_click)
    return b


def section_keys(widget: QtWidgets.QWidget) -> set:
    """State keys of all rows in a section (Card rows carry them as property „skey“)."""
    return {lb.property("skey") for lb in widget.findChildren(QtWidgets.QLabel) if lb.property("skey")}


def reset(editor, section_widget, fresh: dict, title: str, kind: str) -> bool:
    """Ask, back up, reset only the keys of this section (none found = whole state, e.g. presets)."""
    from . import dialogs
    if not dialogs.confirm(editor, t("reset.title"), t("reset.ask", page=title), t("reset.btn"), danger=True):
        return False
    path = backup_dir() / f"{kind}.json"
    path.write_text(json.dumps(editor.state, ensure_ascii=False, indent=1), encoding="utf-8")
    keys = section_keys(section_widget) & set(fresh)
    for k in keys or fresh:
        editor.state[k] = json.loads(json.dumps(fresh[k]))
    editor.refresh_controls()
    editor.changed()
    editor.win.set_status(t("reset.section", page=title, path=str(path)))
    return True
