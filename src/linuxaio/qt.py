"""Qt-Weiche: PySide6 bevorzugt, sonst PyQt6 (beides läuft nativ auf KDE Plasma)."""

try:
    from PySide6 import QtCore, QtGui, QtNetwork, QtSvg, QtWidgets  # noqa: F401
    from PySide6.QtCore import Signal  # noqa: F401
    API = "PySide6"
except ImportError:  # pragma: no cover
    from PyQt6 import QtCore, QtGui, QtNetwork, QtSvg, QtWidgets  # noqa: F401
    from PyQt6.QtCore import pyqtSignal as Signal  # noqa: F401
    API = "PyQt6"

Qt = QtCore.Qt
