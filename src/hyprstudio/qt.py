"""Qt-Weiche: PySide6 bevorzugt, sonst PyQt6."""

try:
    from PySide6 import QtCore, QtGui, QtWidgets  # noqa: F401
    from PySide6.QtCore import Signal  # noqa: F401
    API = "PySide6"
except ImportError:  # pragma: no cover
    from PyQt6 import QtCore, QtGui, QtWidgets  # noqa: F401
    from PyQt6.QtCore import pyqtSignal as Signal  # noqa: F401
    API = "PyQt6"

Qt = QtCore.Qt
