"""Dunkles Design mit 3 Akzentfarben (kein Light-Theme)."""

THEMES = {
    "blue": "#3d8bfd",
    "teal": "#22b8cf",
    "violet": "#8b7cf6",
}

BG = "#0e131a"
SURFACE = "#151c26"
SURFACE2 = "#1b2430"
BORDER = "#253142"
TEXT = "#e6edf5"
MUTED = "#8a98ab"
DANGER = "#e5534b"
OK = "#3fb950"


def qss(accent: str) -> str:
    return f"""
    QWidget {{ color: {TEXT}; font-size: 10pt; }}
    #Root {{ background: {BG}; border: 1px solid {BORDER}; border-radius: 16px; }}
    #Title {{ font-size: 12pt; font-weight: 600; }}
    #Hero {{ background: {SURFACE}; border: 1px solid {BORDER}; border-radius: 16px; }}
    #DeName {{ font-size: 22pt; font-weight: 700; }}
    #Chip {{ background: {SURFACE2}; border: 1px solid {BORDER}; border-radius: 9px; padding: 2px 9px;
             color: {MUTED}; font-size: 9pt; }}
    #ChipAccent {{ background: transparent; border: 1px solid {accent}; border-radius: 9px; padding: 2px 9px;
                   color: {accent}; font-size: 9pt; }}
    #ChipOk {{ background: transparent; border: 1px solid {OK}; border-radius: 9px; padding: 2px 9px;
               color: {OK}; font-size: 9pt; }}
    #ChipWarn {{ background: transparent; border: 1px solid #d29922; border-radius: 9px; padding: 2px 9px;
                 color: #d29922; font-size: 9pt; }}
    QComboBox#Combo {{ background: {SURFACE2}; border: 1px solid {BORDER}; border-radius: 10px; padding: 7px 12px; }}
    QComboBox#Combo:hover {{ border-color: {accent}; }}
    QComboBox#Combo::drop-down {{ border: none; width: 22px; }}
    QComboBox QAbstractItemView {{ background: {SURFACE2}; border: 1px solid {BORDER}; border-radius: 8px;
                                   selection-background-color: {accent}; outline: none; padding: 4px; }}
    QLineEdit#Search {{ background: {SURFACE2}; border: 1px solid {BORDER}; border-radius: 10px; padding: 7px 10px; }}
    QLineEdit#Search:focus {{ border-color: {accent}; }}
    #DmBar {{ background: {SURFACE}; border: 1px solid {BORDER}; border-radius: 14px; }}
    #TabBox {{ background: {SURFACE}; border: 1px solid {BORDER}; border-radius: 12px; }}
    #TabBox QPushButton#Seg {{ padding: 7px 22px; font-weight: 600; }}
    QPushButton {{ background: {SURFACE2}; border: 1px solid {BORDER}; border-radius: 10px; padding: 8px 16px; }}
    QPushButton:hover {{ border-color: {accent}; }}
    QPushButton:disabled {{ color: #4d5a6b; border-color: {SURFACE2}; }}
    QPushButton#Primary {{ background: {accent}; border: 1px solid {accent}; color: #ffffff; font-weight: 600; }}
    QPushButton#Primary:hover {{ background: {accent}; border-color: #ffffff; }}
    QPushButton#Primary:disabled {{ background: {SURFACE2}; border-color: {SURFACE2}; color: #4d5a6b; }}
    QPushButton#Danger:hover {{ border-color: {DANGER}; color: {DANGER}; }}
    QPushButton[applied="true"], QPushButton[applied="true"]:disabled,
    QPushButton#Primary[applied="true"], QPushButton#Primary[applied="true"]:disabled,
    QPushButton#Danger[applied="true"]:disabled {{ background: transparent;
        border: 1px solid {OK}; color: {OK}; font-weight: 600; }}
    QPushButton#Seg {{ background: transparent; border: none; border-radius: 8px; padding: 6px 14px; color: {MUTED}; }}
    QPushButton#Seg:checked {{ background: {accent}; color: #ffffff; font-weight: 600; }}
    QPushButton#Seg:disabled {{ color: #3c4757; }}
    #SegBox {{ background: {SURFACE2}; border: 1px solid {BORDER}; border-radius: 10px; }}
    QToolButton {{ background: transparent; border: none; border-radius: 8px; padding: 4px; }}
    QToolButton:hover {{ background: {SURFACE2}; }}
    QToolButton#CloseBtn:hover {{ background: {DANGER}; }}
    QScrollArea, #TileStrip {{ background: transparent; border: none; }}
    QScrollBar:horizontal {{ height: 8px; background: transparent; margin: 0 16px; }}
    QScrollBar::handle:horizontal {{ background: {BORDER}; border-radius: 4px; min-width: 40px; }}
    QScrollBar::handle:horizontal:hover {{ background: {accent}; }}
    QScrollBar::add-line, QScrollBar::sub-line, QScrollBar::add-page, QScrollBar::sub-page {{ width: 0; background: none; }}
    QScrollBar:vertical {{ width: 8px; background: transparent; }}
    QScrollBar::handle:vertical {{ background: {BORDER}; border-radius: 4px; min-height: 30px; }}
    QProgressBar {{ background: {SURFACE2}; border: none; border-radius: 3px; max-height: 6px; }}
    QProgressBar::chunk {{ background: {accent}; border-radius: 3px; }}
    #Status {{ color: {MUTED}; }}
    QPlainTextEdit {{ background: #0a0e13; border: 1px solid {BORDER}; border-radius: 12px; padding: 8px;
                      font-family: "Hack", "JetBrains Mono", "DejaVu Sans Mono", monospace; font-size: 9pt;
                      color: #b8c4d4; }}
    QPlainTextEdit#LogDrawer {{ background: #0a0e13; border: 1px solid {accent}; border-radius: 14px; padding: 10px; }}
    #Panel {{ background: {SURFACE}; border-right: 1px solid {BORDER}; border-top-left-radius: 16px;
              border-bottom-left-radius: 16px; }}
    #PanelTitle {{ font-size: 13pt; font-weight: 600; }}
    #Label {{ color: {MUTED}; font-size: 9pt; }}
    QPushButton#Lang {{ padding: 6px 14px; }}
    QPushButton#Lang:checked {{ border-color: {accent}; color: {accent}; font-weight: 600; }}
    QToolTip {{ background: {SURFACE2}; color: {TEXT}; border: 1px solid {BORDER}; border-radius: 6px; padding: 6px; }}
    QMessageBox {{ background: {SURFACE}; }}
    #SideBox {{ background: {SURFACE}; border: 1px solid {BORDER}; border-radius: 14px; }}
    QListWidget#DeList, QListWidget#NavList {{ background: transparent; border: none; outline: none; }}
    QListWidget#NavList {{ background: {SURFACE}; border: 1px solid {BORDER}; border-radius: 14px; padding: 8px; }}
    QListWidget#DeList::item, QListWidget#NavList::item {{ border-radius: 8px; padding: 4px 10px; color: {TEXT}; }}
    QListWidget#DeList::item:hover:enabled, QListWidget#NavList::item:hover {{ background: {SURFACE2}; }}
    QListWidget#DeList::item:selected, QListWidget#NavList::item:selected {{ background: {SURFACE2};
        border: 1px solid {accent}; color: {TEXT}; font-weight: 600; }}
    QListWidget#DeList::item:disabled {{ color: #4d5a6b; }}
    #PageTitle {{ font-size: 15pt; font-weight: 700; }}
    #Card {{ background: {SURFACE}; border: 1px solid {BORDER}; border-radius: 14px; }}
    #CardTitle {{ color: {MUTED}; font-size: 9pt; font-weight: 600; padding-bottom: 2px; }}
    #Mono {{ color: {MUTED}; font-family: "JetBrains Mono", "Hack", monospace; font-size: 9pt; }}
    #PageInner {{ background: transparent; }}
    QSpinBox#Spin, QDoubleSpinBox#Spin {{ background: {SURFACE2}; border: 1px solid {BORDER}; border-radius: 8px;
        padding: 5px 8px; }}
    QSpinBox#Spin:focus, QDoubleSpinBox#Spin:focus {{ border-color: {accent}; }}
    QFontComboBox#Combo {{ background: {SURFACE2}; border: 1px solid {BORDER}; border-radius: 10px; padding: 6px 10px; }}
    QListWidget {{ background: {SURFACE2}; border: 1px solid {BORDER}; border-radius: 10px; }}
    QDialog {{ background: {SURFACE}; }}
    """
