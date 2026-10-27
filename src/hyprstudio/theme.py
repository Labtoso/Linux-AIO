"""Dunkles Design (Design-Vorlage Nr. 1) mit den Akzentfarben der gemeinsamen Paletten, kein Light-Theme."""

from .palettes import LEGACY_ACCENT, PALETTES  # noqa: E402

# Akzentfarben der Oberfläche = gemeinsame Paletten (gleiche Namen/Reihenfolge wie alle Presets)
THEMES = {k: p["ui"] for k, p in PALETTES.items()}


def on_accent(accent: str) -> str:
    """Schrift auf der Akzentfarbe: dunkel auf hellen Farben (Weiß, Mint, Orange), sonst weiß."""
    h = accent.lstrip("#")[:6]
    r, g, b = (int(h[i:i + 2], 16) / 255 for i in (0, 2, 4))
    return BG if 0.2126 * r + 0.7152 * g + 0.0722 * b > 0.55 else "#ffffff"

BG = "#0e131a"
SURFACE = "#151c26"
SURFACE2 = "#1b2430"
BORDER = "#253142"
TEXT = "#e6edf5"
MUTED = "#8a98ab"
DANGER = "#e5534b"
OK = "#3fb950"
RAISED = "#2b3747"  # gewählter Zustand (Segmente, Tabs, Sprache) – neutral statt Akzentfarbe
LINE = "#3a4a5f"    # Rahmen beim Darüberfahren
HOVER = "#1f2a38"   # Hintergrund beim Darüberfahren


def palette(accent: str):
    from .qt import QtGui
    P = QtGui.QPalette
    pal = QtGui.QPalette()
    c = QtGui.QColor
    for role, col in ((P.ColorRole.Window, SURFACE), (P.ColorRole.WindowText, TEXT), (P.ColorRole.Base, BG),
                      (P.ColorRole.AlternateBase, SURFACE2), (P.ColorRole.Text, TEXT), (P.ColorRole.Button, SURFACE2),
                      (P.ColorRole.ButtonText, TEXT), (P.ColorRole.ToolTipBase, SURFACE2),
                      (P.ColorRole.ToolTipText, TEXT), (P.ColorRole.Highlight, accent),
                      (P.ColorRole.HighlightedText, on_accent(accent)), (P.ColorRole.PlaceholderText, MUTED),
                      (P.ColorRole.Link, accent), (P.ColorRole.Mid, BORDER), (P.ColorRole.Dark, BG),
                      (P.ColorRole.Light, SURFACE2), (P.ColorRole.Midlight, SURFACE2), (P.ColorRole.Shadow, "#000000")):
        pal.setColor(role, c(col))
    for role in (P.ColorRole.WindowText, P.ColorRole.Text, P.ColorRole.ButtonText):
        pal.setColor(P.ColorGroup.Disabled, role, c(MUTED))
    return pal


def qss(accent: str) -> str:
    on = on_accent(accent)
    return f"""
    QWidget {{ color: {TEXT}; font-size: 10pt; }}
    #Root {{ background: {BG}; border: 1px solid {BORDER}; border-radius: 16px; }}
    #Title {{ font-size: 12pt; font-weight: 600; }}
    #Side {{ background: {SURFACE}; border: 1px solid {BORDER}; border-radius: 12px; }}
    QPushButton#Nav {{ background: transparent; border: none; border-radius: 6px; padding: 8px 14px;
                       text-align: left; color: {MUTED}; }}
    QPushButton#NavHead {{ background: transparent; border: none; color: {MUTED}; font-size: 9pt; font-weight: 600;
                           text-align: left; padding: 12px 8px 4px 24px; }}
    QPushButton#NavHead:hover {{ color: {TEXT}; }}
    QPushButton#Nav:hover {{ background: {SURFACE2}; color: {TEXT}; }}
    QPushButton#Nav:checked {{ background: {SURFACE2}; color: {TEXT}; }}
    #Card {{ background: {SURFACE}; border: 1px solid {BORDER}; border-radius: 12px; }}
    #CardTitle {{ font-size: 10.5pt; font-weight: 600; }}
    #RowLabel {{ color: {MUTED}; }}
    #Value {{ color: {TEXT}; font-family: "Hack", "JetBrains Mono", "DejaVu Sans Mono", monospace; }}
    #PageTitle {{ font-size: 15pt; font-weight: 600; }}
    QSlider:horizontal {{ min-height: 24px; background: transparent; }}
    QSlider::groove:horizontal {{ height: 6px; background: {SURFACE2}; border-radius: 3px; }}
    QSlider::sub-page:horizontal {{ background: {accent}; border-radius: 3px; }}
    QSlider::handle:horizontal {{ background: {TEXT}; width: 16px; height: 16px; margin: -5px 0; border-radius: 8px; }}
    QSlider::handle:horizontal:hover {{ background: #ffffff; }}
    QSpinBox {{ background: {SURFACE2}; border: 1px solid {BORDER}; border-radius: 6px; padding: 0 6px; min-height: 30px; }}
    QSpinBox:focus {{ border-color: {accent}; }}
    QLineEdit, QComboBox {{ background: {SURFACE2}; border: 1px solid {BORDER}; border-radius: 6px; padding: 6px 10px; }}
    QLineEdit:focus {{ border-color: {accent}; }}
    QComboBox:hover {{ border-color: {LINE}; }}
    QComboBox::drop-down {{ border: none; width: 22px; }}
    QComboBox QAbstractItemView {{ background: {SURFACE2}; border: 1px solid {BORDER}; border-radius: 8px;
                                   selection-background-color: {accent}; outline: none; padding: 4px; }}
    QMenu {{ background: {SURFACE}; border: 1px solid {BORDER}; border-radius: 10px; padding: 5px; color: {TEXT}; }}
    QMenu::item {{ padding: 6px 22px 6px 12px; border-radius: 5px; background: transparent; }}
    QMenu::item:selected {{ background: {accent}; color: {on}; }}
    QMenu::item:disabled {{ color: {MUTED}; }}
    QMenu::separator {{ height: 1px; background: {BORDER}; margin: 5px 8px; }}
    QMenu::section {{ color: {MUTED}; padding: 6px 14px 2px 14px; font-weight: 600; }}
    QMenu::indicator {{ width: 14px; height: 14px; margin-left: 6px; }}
    QFileDialog QTreeView, QFileDialog QListView, QColorDialog QListView {{ background: {BG};
        border: 1px solid {BORDER}; border-radius: 10px; outline: none; }}
    QFileDialog QTreeView::item, QFileDialog QListView::item {{ padding: 3px; }}
    QFileDialog QTreeView::item:selected, QFileDialog QListView::item:selected {{ background: {accent}; color: {on}; }}
    QHeaderView::section {{ background: {SURFACE2}; color: {MUTED}; border: none; border-right: 1px solid {BORDER};
        padding: 6px 8px; }}
    QFileDialog QToolButton {{ background: {SURFACE2}; border: 1px solid {BORDER}; border-radius: 8px; padding: 4px; }}
    QFileDialog QToolButton:hover {{ border-color: {LINE}; }}
    QFileDialog QSplitter::handle {{ background: transparent; }}
    #PopupCard {{ background: {SURFACE}; border: 1px solid {BORDER}; border-radius: 10px; }}
    QListWidget#PopList {{ background: transparent; border: none; outline: none; }}
    QListWidget#PopList::item {{ border-radius: 5px; padding: 5px 8px; }}
    QListWidget#PopList::item:hover {{ background: {SURFACE2}; }}
    QListWidget#PopList::item:selected {{ background: {accent}; color: {on}; }}
    QComboBox QAbstractItemView::item {{ border-radius: 6px; padding: 4px 6px; }}
    QListWidget#ModList {{ background: {BG}; border: 1px solid {BORDER}; border-radius: 12px; padding: 6px; outline: none; }}
    QListWidget#ModList::item {{ background: {SURFACE2}; border: 1px solid {BORDER}; border-radius: 8px;
                                 padding: 6px 8px; margin: 2px 0; }}
    QListWidget#ModList::item:selected {{ border-color: {accent}; color: {TEXT}; }}
    #TabBox {{ background: {SURFACE}; border: 1px solid {BORDER}; border-radius: 9px; }}
    #TabBox QPushButton#Seg {{ padding: 7px 22px; font-weight: 600; }}
    QPushButton {{ background: {SURFACE2}; border: 1px solid {BORDER}; border-radius: 7px; padding: 8px 16px; }}
    QPushButton:hover {{ border-color: {LINE}; background: {HOVER}; }}
    QPushButton:disabled {{ color: #4d5a6b; border-color: {SURFACE2}; }}
    QPushButton#Primary {{ background: {accent}; border: 1px solid {accent}; color: {on}; font-weight: 600; }}
    QPushButton#Primary:hover {{ background: {accent}; border-color: {accent}; }}
    QPushButton#Primary:disabled {{ background: {SURFACE2}; border-color: {SURFACE2}; color: #4d5a6b; }}
    QPushButton#Danger:hover {{ border-color: {DANGER}; color: {DANGER}; }}
    QPushButton#Outline {{ background: transparent; border: 1px solid {LINE}; color: {TEXT}; }}
    QPushButton#Outline:hover {{ background: {HOVER}; }}
    QPushButton#DangerFill {{ background: transparent; border: 1px solid {DANGER}; color: {DANGER}; font-weight: 600; }}
    QPushButton#DangerFill:hover {{ background: {DANGER}; color: #ffffff; }}
    QPushButton#DangerFill:disabled {{ border-color: {SURFACE2}; color: #4d5a6b; background: transparent; }}
    #DialogFrame {{ background: {SURFACE}; border: 1px solid {BORDER}; border-radius: 16px; }}
    QCheckBox {{ spacing: 10px; }}
    QCheckBox::indicator {{ width: 18px; height: 18px; border-radius: 6px; border: 1px solid {BORDER}; background: {SURFACE2}; }}
    QCheckBox::indicator:checked {{ background: {accent}; border-color: {accent}; }}
    QCheckBox::indicator:disabled {{ background: {BG}; }}
    QCheckBox#Sure {{ color: {DANGER}; font-weight: 600; }}
    QCheckBox#Sure::indicator:checked {{ background: {DANGER}; border-color: {DANGER}; }}
    QPushButton#Seg {{ background: transparent; border: none; border-radius: 5px; padding: 6px 14px; color: {MUTED}; }}
    QPushButton#Seg:hover {{ color: {TEXT}; }}
    QPushButton#Seg:checked {{ background: {RAISED}; color: {TEXT}; font-weight: 600; }}
    #SegBox {{ background: {SURFACE2}; border: 1px solid {BORDER}; border-radius: 7px; }}
    QToolButton {{ background: transparent; border: none; border-radius: 8px; padding: 4px; }}
    QToolButton:hover {{ background: {SURFACE2}; }}
    QToolButton#CloseBtn:hover {{ background: {DANGER}; }}
    QToolButton#ResetBtn {{ background: transparent; border: none; border-radius: 7px; }}
    QToolButton#ResetBtn:hover {{ background: {SURFACE2}; }}
    QScrollArea {{ background: transparent; border: none; }}
    QScrollArea > QWidget#qt_scrollarea_viewport {{ background: transparent; }}
    #ScrollInner {{ background: transparent; }}
    QScrollBar:vertical {{ width: 8px; background: transparent; margin: 4px 0; }}
    QScrollBar::handle:vertical {{ background: {BORDER}; border-radius: 4px; min-height: 30px; }}
    QScrollBar::handle:vertical:hover {{ background: {LINE}; }}
    QScrollBar:horizontal {{ height: 8px; background: transparent; }}
    QScrollBar::handle:horizontal {{ background: {BORDER}; border-radius: 4px; min-width: 30px; }}
    QScrollBar::add-line, QScrollBar::sub-line, QScrollBar::add-page, QScrollBar::sub-page {{ width: 0; height: 0; background: none; }}
    #BindRow {{ background: {SURFACE2}; border: 1px solid {BORDER}; border-radius: 8px; }}
    #BindRow QComboBox {{ background: {BG}; }}
    QPushButton#Mod {{ background: transparent; padding: 4px 9px; border-radius: 5px; color: {MUTED}; font-size: 9pt; }}
    QPushButton#Mod:checked {{ background: {RAISED}; border-color: {LINE}; color: {TEXT}; font-weight: 600; }}
    QPushButton#KeyBtn {{ background: {BG}; font-family: "JetBrains Mono", "Hack", "Consolas", "JetBrains Mono", "DejaVu Sans Mono", monospace;
                          font-weight: 600; }}
    #Warn {{ background: transparent; border: 1px solid {DANGER}; border-radius: 11px; color: {DANGER}; font-weight: 700; }}
    #ChipOk {{ background: transparent; border: none; padding: 0; color: {OK}; font-size: 9pt; }}
    #ChipWarn {{ background: transparent; border: none; padding: 0; color: #d29922; font-size: 9pt; }}
    #Mono {{ font-family: "JetBrains Mono", "Hack", "Consolas", monospace; color: {TEXT}; }}
    #Code {{ font-family: "JetBrains Mono", "Hack", "Consolas", monospace; font-size: 9pt; color: {MUTED};
             background: {BG}; border-radius: 6px; padding: 4px 8px; }}
    QProgressBar {{ background: {SURFACE2}; border: none; border-radius: 3px; max-height: 6px; }}
    QProgressBar::chunk {{ background: {accent}; border-radius: 3px; }}
    #Status {{ color: {MUTED}; }}
    QPlainTextEdit {{ background: #0a0e13; border: 1px solid {BORDER}; border-radius: 8px; padding: 8px;
                      font-family: "Hack", "JetBrains Mono", "DejaVu Sans Mono", monospace; font-size: 9pt;
                      color: #b8c4d4; }}
    QPlainTextEdit[readOnly="false"] {{ background: {SURFACE2}; border-radius: 6px; color: {TEXT}; }}
    QPlainTextEdit[readOnly="false"]:focus {{ border-color: {accent}; }}
    #Panel {{ background: {SURFACE}; border-right: 1px solid {BORDER}; border-top-left-radius: 16px;
              border-bottom-left-radius: 16px; }}
    #PanelTitle {{ font-size: 13pt; font-weight: 600; }}
    #Label {{ color: {MUTED}; font-size: 9pt; }}
    QPushButton#Lang {{ padding: 6px 14px; }}
    QPushButton#Lang:checked {{ background: {RAISED}; border-color: {LINE}; color: {TEXT}; font-weight: 600; }}
    QToolTip {{ background: {SURFACE2}; color: {TEXT}; border: 1px solid {BORDER}; border-radius: 6px; padding: 6px; }}
    QColorDialog, QColorDialog QWidget {{ background: {SURFACE}; }}
    QSpinBox QLineEdit {{ background: transparent; border: none; padding: 0; }}
    QSpinBox::up-button, QSpinBox::down-button {{ width: 0; height: 0; border: none; }}
    """
