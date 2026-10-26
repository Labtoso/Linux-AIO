"""L Launcher („l-launcher“ bzw. „hypr-studio --launcher“): vergleicht die Eingabe mit den Programmnamen und
sucht bei jedem neuen Buchstaben neu – es bleiben nur Programme, deren Name mit der Eingabe ANFÄNGT.
„h“ → alles mit H, „hy“ → nur Hy…, Enter startet den obersten Treffer. Oft Gestartetes steht weiter oben.
Aussehen kommt aus dem Launcher-Tab (beim Anwenden nach ~/.config/LabtosoStudios/l-launcher.json)."""

import json
import os
import re
import shlex
import subprocess
import sys
from pathlib import Path

APP_ID = "l-launcher"
FIELD = re.compile(r"%[fFuUdDnNickvm]")


def _cfg_home() -> Path:
    from . import model
    return model.config_home()


def settings_file() -> Path:
    return _cfg_home() / "LabtosoStudios" / "l-launcher.json"


def history_file() -> Path:
    from . import model
    return model.CACHE / "l-launcher-history.json"


def app_dirs() -> list:
    home = Path.home()
    data = [Path(os.environ.get("XDG_DATA_HOME", home / ".local/share"))]
    data += [Path(p) for p in os.environ.get("XDG_DATA_DIRS", "/usr/local/share:/usr/share").split(":") if p]
    data += [home / ".local/share/flatpak/exports/share", Path("/var/lib/flatpak/exports/share")]
    out = []
    for d in data:
        a = d / "applications"
        if a not in out:
            out.append(a)
    return out


def _lang_keys() -> list:
    lang = (os.environ.get("LC_MESSAGES") or os.environ.get("LANG") or "").split(".")[0]
    keys = []
    if lang:
        keys.append(f"Name[{lang}]")
        if "_" in lang:
            keys.append(f"Name[{lang.split('_')[0]}]")
    return keys + ["Name"]


def parse_desktop(text: str, lang_keys: list | None = None) -> dict | None:
    """[Desktop Entry] lesen; None für Ausgeblendetes, Nicht-Anwendungen und Einträge ohne Exec."""
    lang_keys = lang_keys or ["Name"]
    entry, sect = {}, None
    for raw in text.splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("["):
            sect = line
            continue
        if sect != "[Desktop Entry]" or "=" not in line:
            continue
        k, v = line.split("=", 1)
        entry.setdefault(k.strip(), v.strip())
    if entry.get("Type", "Application") != "Application":
        return None
    if entry.get("NoDisplay", "").lower() == "true" or entry.get("Hidden", "").lower() == "true":
        return None
    name = next((entry[k] for k in lang_keys if entry.get(k)), "")
    if not name or not entry.get("Exec"):
        return None
    return {"name": name, "exec": entry["Exec"], "icon": entry.get("Icon", ""),
            "terminal": entry.get("Terminal", "").lower() == "true", "path": entry.get("Path", "")}


def load_apps(dirs: list | None = None) -> list:
    """Alle Programme; gleiche Desktop-ID nur einmal (die erste gewinnt, wie bei XDG üblich)."""
    keys, seen, apps = _lang_keys(), set(), []
    for d in dirs or app_dirs():
        if not d.is_dir():
            continue
        for f in sorted(d.rglob("*.desktop")):
            did = str(f.relative_to(d)).replace(os.sep, "-")
            if did in seen:
                continue
            seen.add(did)
            try:
                a = parse_desktop(f.read_text(encoding="utf-8", errors="replace"), keys)
            except OSError:
                continue
            if a:
                a["id"] = did
                apps.append(a)
    return apps


def filter_apps(apps: list, query: str, history: dict, case_insensitive: bool = True) -> list:
    """Nur Namen, die mit der Eingabe ANFANGEN; oft Gestartetes zuerst, dann alphabetisch."""
    q = query.strip()
    if case_insensitive:
        q = q.lower()
    hits = [a for a in apps if (a["name"].lower() if case_insensitive else a["name"]).startswith(q)]
    return sorted(hits, key=lambda a: (-int(history.get(a["id"], 0)), a["name"].lower()))


def command(app: dict, terminal: str = "kitty") -> str:
    cmd = FIELD.sub("", app["exec"]).replace("%%", "%").strip()
    if app["terminal"]:
        cmd = f"{terminal} -e {cmd}"
    return cmd


def launch(app: dict, terminal: str = "kitty") -> None:
    cwd = os.path.expanduser(app["path"]) if app.get("path") else str(Path.home())
    subprocess.Popen(["sh", "-c", command(app, terminal)], cwd=cwd if os.path.isdir(cwd) else None,
                     stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                     start_new_session=True)


def load_json(f: Path) -> dict:
    try:
        d = json.loads(f.read_text(encoding="utf-8"))
        return d if isinstance(d, dict) else {}
    except (OSError, ValueError):
        return {}


def remember(app_id: str) -> None:
    f = history_file()
    h = load_json(f)
    h[app_id] = int(h.get(app_id, 0)) + 1
    try:
        f.parent.mkdir(parents=True, exist_ok=True)
        f.write_text(json.dumps(h), encoding="utf-8")
    except OSError:
        pass


# ---------- Fenster ----------

def pid_file() -> Path:
    run = os.environ.get("XDG_RUNTIME_DIR")
    return Path(run) / "l-launcher.pid" if run else history_file().with_name("l-launcher.pid")


def _alive(pid: int) -> bool:
    try:
        os.kill(pid, 0)
    except (OSError, ValueError):
        return False
    try:  # nur echte L-Launcher-Prozesse (PID kann neu vergeben sein)
        return b"--launcher" in Path(f"/proc/{pid}/cmdline").read_bytes()
    except OSError:
        return sys.platform == "win32"


def toggle_running() -> bool:
    """Läuft schon ein L Launcher? Dann schließen und True zurückgeben (zweites Drücken)."""
    if sys.platform == "win32":  # os.kill würde unter Windows beenden statt prüfen (nur Testmodus)
        return False
    f = pid_file()
    try:
        pid = int(f.read_text().strip())
    except (OSError, ValueError):
        return False
    if pid != os.getpid() and _alive(pid):
        import signal
        try:
            os.kill(pid, signal.SIGTERM)
        except OSError:
            return False
        return True
    return False


def main(argv) -> int:
    from . import launcher as lconf
    s0 = lconf.state(load_json(settings_file()))
    if s0["toggle"] and toggle_running():
        return 0
    try:
        pid_file().parent.mkdir(parents=True, exist_ok=True)
        pid_file().write_text(str(os.getpid()))
    except OSError:
        pass
    from .qt import QtCore, QtGui, QtWidgets, Qt
    from .preview import qcolor

    s = s0
    QtGui.QGuiApplication.setDesktopFileName(APP_ID)
    app = QtWidgets.QApplication(argv)
    app.setApplicationName(APP_ID)
    # Symbolthema wie im System (KDE z. B. Sweet), auch aus ~/.local/share/icons und ~/.icons
    home = Path.home()
    paths = [str(p) for p in (home / ".local/share/icons", home / ".icons") if p.is_dir()]
    QtGui.QIcon.setThemeSearchPaths(paths + [p for p in QtGui.QIcon.themeSearchPaths() if p not in paths]
                                    + [str(d.parent / "icons") for d in app_dirs() if (d.parent / "icons").is_dir()])
    theme = lconf.icon_theme(s)
    if theme:
        QtGui.QIcon.setThemeName(theme)
    elif not QtGui.QIcon.themeName() or QtGui.QIcon.themeName() == "hicolor":
        for name in ("Papirus-Dark", "Papirus", "breeze-dark", "breeze", "Adwaita"):
            if any((d.parent / "icons" / name).is_dir() for d in app_dirs()):
                QtGui.QIcon.setThemeName(name)
                break
    QtGui.QIcon.setFallbackThemeName("hicolor")

    apps = load_apps()
    history = load_json(history_file())

    def rgba(c: str) -> str:
        q = qcolor(c)
        return f"rgba({q.red()},{q.green()},{q.blue()},{q.alpha()})"

    row_h = max(s["icon_size"], int(s["font_size"] * 2)) + 12
    rows = max(3, s["lines"])
    w = QtWidgets.QWidget()
    w.setWindowTitle("L Launcher")
    w.setWindowFlags(Qt.WindowType.FramelessWindowHint)
    w.setAttribute(Qt.WidgetAttribute.WA_TranslucentBackground)
    outer = QtWidgets.QVBoxLayout(w)
    outer.setContentsMargins(0, 0, 0, 0)
    card = QtWidgets.QFrame()
    card.setObjectName("Card")
    outer.addWidget(card)
    lay = QtWidgets.QVBoxLayout(card)
    pad = s["padding"]
    lay.setContentsMargins(pad, pad, pad, pad)
    lay.setSpacing(max(6, s["spacing"] + 4))
    edit = QtWidgets.QLineEdit()
    edit.setPlaceholderText(s["placeholder"])
    if s["show_prompt"] and s["prompt"]:
        lbl = QtWidgets.QLabel(s["prompt"])
        lbl.setObjectName("Prompt")
        top = QtWidgets.QHBoxLayout()
        top.addWidget(lbl)
        top.addWidget(edit, 1)
        lay.addLayout(top)
    else:
        lay.addWidget(edit)
    lst = QtWidgets.QListWidget()
    lst.setUniformItemSizes(True)
    lst.setIconSize(QtCore.QSize(s["icon_size"], s["icon_size"]))
    lst.setHorizontalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
    lst.setVerticalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAsNeeded if s["show_scroll"]
                                   else Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
    # wie bei Rofi/Wofi: Raster (Kacheln) oder Liste mit mehreren Spalten
    cols = max(1, s["columns"]) if (s["grid"] or s["columns"] > 1) else 1
    inner_w = s["width"] - 2 * pad - 2 * s["border_width"] - 8
    cell_w = inner_w // cols
    if s["grid"]:
        row_h = s["icon_size"] + int(s["font_size"] * 2.2) + 22
        lst.setViewMode(QtWidgets.QListView.ViewMode.IconMode)
        lst.setMovement(QtWidgets.QListView.Movement.Static)
    if cols > 1:
        lst.setFlow(QtWidgets.QListView.Flow.LeftToRight)
        lst.setWrapping(True)
        lst.setResizeMode(QtWidgets.QListView.ResizeMode.Adjust)
        lst.setGridSize(QtCore.QSize(cell_w, row_h + s["spacing"]))
        lst.setWordWrap(True)
    lst.setFixedHeight(rows * (row_h + s["spacing"]) + 6)
    lay.addWidget(lst)
    font = f'"{s["font"]}"'
    w.setStyleSheet(f"""
        * {{ font-family: {font}; font-size: {s['font_size']}pt; color: {rgba(s['fg'])}; }}
        #Card {{ background: {rgba(s['bg'])}; border: {s['border_width']}px solid {rgba(s['border'])};
                 border-radius: {s['radius']}px; }}
        QLineEdit {{ background: {rgba(s['input_bg'])}; border: none; border-radius: {s['el_radius']}px;
                     padding: 8px 12px; selection-background-color: {rgba(s['accent'])}; }}
        #Prompt {{ color: {rgba(s['accent'])}; font-weight: 600; padding-right: 6px; }}
        QListWidget {{ background: transparent; border: none; outline: none; }}
        QListWidget::item {{ {"" if cols > 1 else f"height: {row_h}px;"} border-radius: {s['el_radius']}px;
                             padding: {"6px 4px" if s["grid"] else "0 8px"};
                             {"" if cols > 1 else f"margin-bottom: {s['spacing']}px;"} }}
        QListWidget::item:selected {{ background: {rgba(s['sel'])}; color: {rgba(s['sel_fg'])}; }}
        QScrollBar:vertical {{ width: 6px; background: transparent; }}
        QScrollBar::handle:vertical {{ background: {rgba(s['border'])}; border-radius: 3px; }}
        QScrollBar::add-line, QScrollBar::sub-line {{ height: 0; }}
    """)
    # feste Größe → Hyprland lässt das Fenster schweben und zentriert es
    w.setFixedSize(s["width"], lst.height() + 2 * pad + edit.sizeHint().height() + lay.spacing() + 4)

    shown = []
    icons = {}  # Symbole nur einmal suchen (QIcon.fromTheme pro Buchstabe und App war der Bremsklotz)

    def icon(a):
        if a["id"] not in icons:
            icons[a["id"]] = QtGui.QIcon(a["icon"]) if os.path.isabs(a["icon"]) else QtGui.QIcon.fromTheme(a["icon"])
        return icons[a["id"]]

    def refresh(*_):
        shown[:] = filter_apps(apps, edit.text(), history, s["insensitive"])
        lst.setUpdatesEnabled(False)
        lst.clear()
        for a in shown:
            it = QtWidgets.QListWidgetItem(a["name"])
            if cols > 1:  # ganze Zelle als Kachel (Auswahl füllt sie, Name hat Platz)
                it.setSizeHint(QtCore.QSize(cell_w - 6, row_h))
                if s["grid"]:
                    it.setTextAlignment(Qt.AlignmentFlag.AlignHCenter | Qt.AlignmentFlag.AlignBottom)
            if s["icons"] and a["icon"]:
                it.setIcon(icon(a))
            lst.addItem(it)
        lst.setUpdatesEnabled(True)
        if shown:
            lst.setCurrentRow(0)

    def start(row: int | None = None):
        row = lst.currentRow() if row is None else row
        if 0 <= row < len(shown):
            a = shown[row]
            remember(a["id"])
            launch(a, s["terminal"])
        app.quit()

    class Keys(QtCore.QObject):
        def eventFilter(self, obj, e):
            if e.type() == QtCore.QEvent.Type.KeyPress:
                k = e.key()
                n = lst.count()
                if k == Qt.Key.Key_Escape:
                    app.quit()
                    return True
                if k in (Qt.Key.Key_Return, Qt.Key.Key_Enter):
                    start()
                    return True
                step = {Qt.Key.Key_Down: cols, Qt.Key.Key_Up: -cols, Qt.Key.Key_Tab: 1, Qt.Key.Key_Backtab: -1,
                        Qt.Key.Key_PageDown: rows * cols, Qt.Key.Key_PageUp: -rows * cols}.get(k)
                if cols > 1 and not edit.text() and k in (Qt.Key.Key_Left, Qt.Key.Key_Right):
                    step = 1 if k == Qt.Key.Key_Right else -1  # im Raster links/rechts, solange nichts getippt ist
                if step and n:
                    lst.setCurrentRow(max(0, min(n - 1, lst.currentRow() + step)))
                    return True
            if e.type() == QtCore.QEvent.Type.WindowDeactivate and obj is w:
                app.quit()
            return False

    keys = Keys(w)
    edit.installEventFilter(keys)
    w.installEventFilter(keys)
    edit.textChanged.connect(refresh)
    lst.itemActivated.connect(lambda it: start(lst.row(it)))
    lst.itemClicked.connect(lambda it: start(lst.row(it)))
    refresh()
    w.show()
    edit.setFocus()
    import signal
    signal.signal(signal.SIGTERM, lambda *_: app.quit())  # zweites Drücken → schließen
    tick = QtCore.QTimer(w)  # Python-Signale auch während Qt läuft verarbeiten
    tick.start(200)
    try:
        return app.exec()
    finally:
        try:
            if pid_file().read_text().strip() == str(os.getpid()):
                pid_file().unlink()
        except OSError:
            pass


if __name__ == "__main__":
    sys.exit(main(sys.argv))
