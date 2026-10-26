"""Root-Teil für SDDM-Themes: installieren (Archiv sicher entpacken), setzen, entfernen."""

import configparser
import json
import os
import re
import shutil
import stat
import subprocess
import tarfile
import tempfile
import zipfile
from pathlib import Path

from . import sddm, system

MAX_ARCHIVE = 300 * 1024 * 1024
MAX_UNPACKED = 800 * 1024 * 1024

# QML-Import -> Arch-Paket (Qt6-Greeter bzw. Qt5-Greeter)
QML_DEPS_QT6 = {
    "Qt5Compat.GraphicalEffects": "qt6-5compat",
    "QtMultimedia": "qt6-multimedia",
    "QtQuick.VirtualKeyboard": "qt6-virtualkeyboard",
    "QtQuick.Controls": "qt6-declarative",
    "QtSvg": "qt6-svg",
}
QML_DEPS_QT5 = {
    "QtGraphicalEffects": "qt5-graphicaleffects",
    "QtQuick.Controls 2": "qt5-quickcontrols2",
    "QtQuick.Controls 1": "qt5-quickcontrols",
    "QtMultimedia": "qt5-multimedia",
    "QtQuick.VirtualKeyboard": "qt5-virtualkeyboard",
    "QtSvg": "qt5-svg",
}


def _say(tag, text=""):
    print(f"@@{tag} {text}".rstrip(), flush=True)


def _dry() -> bool:
    return os.environ.get("DEMANAGER_DRYRUN") == "1"


def save_store_state(st: dict) -> None:
    p = sddm.store_state_file()
    p.parent.mkdir(parents=True, exist_ok=True)
    tmp = p.with_suffix(".tmp")
    tmp.write_text(json.dumps(st, indent=2, sort_keys=True), encoding="utf-8")
    os.replace(tmp, p)
    try:
        os.chmod(p, 0o644)
    except OSError:
        pass


def user_owned(st) -> bool:
    """Datei muss dem Benutzer gehören, der pkexec aufgerufen hat (kein Auslesen fremder Root-Dateien)."""
    if _dry():
        return True
    uid = os.environ.get("PKEXEC_UID") or os.environ.get("SUDO_UID")
    return uid is not None and uid.isdigit() and st.st_uid == int(uid)


def is_image(path: Path) -> bool:
    try:
        with open(path, "rb") as f:
            head = f.read(8)
    except OSError:
        return False
    return head.startswith(bytes([0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A])) or head[:3] == bytes([0xFF, 0xD8, 0xFF])


# ---------- sicher entpacken ----------

def _safe_member(name: str) -> bool:
    p = Path(name)
    return not p.is_absolute() and ".." not in p.parts and not name.startswith(("/", "\\"))


def extract(archive: Path, dest: Path) -> None:
    total = 0
    if zipfile.is_zipfile(archive):
        with zipfile.ZipFile(archive) as z:
            for m in z.infolist():
                if not _safe_member(m.filename):
                    raise ValueError("unsafe")
                mode = (m.external_attr >> 16) & 0o170000
                if mode == stat.S_IFLNK:
                    continue
                total += m.file_size
                if total > MAX_UNPACKED:
                    raise ValueError("toolarge")
                target = dest / m.filename
                if m.is_dir():
                    target.mkdir(parents=True, exist_ok=True)
                    continue
                target.parent.mkdir(parents=True, exist_ok=True)
                with z.open(m) as src, open(target, "wb") as out:
                    shutil.copyfileobj(src, out)
        return
    with tarfile.open(archive, "r:*") as tar:
        members = []
        for m in tar.getmembers():
            if not _safe_member(m.name):
                raise ValueError("unsafe")
            if not (m.isfile() or m.isdir()):
                continue          # keine Links, Geräte, FIFOs
            total += m.size
            if total > MAX_UNPACKED:
                raise ValueError("toolarge")
            members.append(m)
        try:
            tar.extractall(dest, members=members, filter="data")
        except TypeError:        # Python ohne filter-Parameter
            tar.extractall(dest, members=members)


def find_theme_roots(base: Path, archive_name: str) -> list:
    roots = []
    for meta in base.rglob("metadata.desktop"):
        d = meta.parent
        if any(d.glob("*.qml")) and not any(r in d.parents for r in roots):
            roots.append(d)
    out = []
    for d in roots:
        name = d.name if d != base else re.split(r"\.(tar|zip|tgz|txz|tbz2)", archive_name)[0]
        out.append((sddm.safe_name(name), d))
    return out


def qml_deps(theme_dir: Path, qt6: bool) -> list:
    table = QML_DEPS_QT6 if qt6 else QML_DEPS_QT5
    found = set() if qt6 else {"qt5-declarative"}   # Qt5-Greeter braucht QtQuick 2 aus Qt5
    for q in theme_dir.rglob("*.qml"):
        try:
            text = q.read_text(encoding="utf-8", errors="ignore")
        except OSError:
            continue
        for imp, pkg in table.items():
            if re.search(r"^\s*import\s+" + re.escape(imp) + r"\b", text, re.M):
                found.add(pkg)
    return sorted(found)


def install_deps(pkgs: list) -> None:
    backend = system.detect_backend()
    if not pkgs or backend != "pacman":
        return
    missing = sorted(set(pkgs) - (set() if _dry() else system.installed_packages("pacman")))
    if not missing:
        return
    print("Abhängigkeiten: " + " ".join(missing), flush=True)
    cmd = ["pacman", "-S", "--needed", "--noconfirm", *missing]
    print("$ " + " ".join(cmd), flush=True)
    if not _dry():
        subprocess.run(cmd, stdin=subprocess.DEVNULL)


def _fix_perms(path: Path) -> None:
    for root, dirs, files in os.walk(path):
        for d in dirs:
            os.chmod(os.path.join(root, d), 0o755)
        for f in files:
            os.chmod(os.path.join(root, f), 0o644)
    os.chmod(path, 0o755)


def do_install(archive_arg: str, store_id: str = "") -> int:
    archive = Path(archive_arg)
    _say("STEP", "sddm.unpack")
    _say("PROGRESS", "10")
    try:
        st = os.lstat(archive)
    except OSError:
        _say("FAIL", "sddm.archive")
        return 2
    if not stat.S_ISREG(st.st_mode) or st.st_size > MAX_ARCHIVE or not sddm.archive_ok(archive.name)             or not user_owned(st):
        _say("FAIL", "sddm.archive")
        return 2
    state = sddm.load_store_state()
    dest_dir = sddm.themes_dir()
    dest_dir.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="desddm-") as tmp:
        work = Path(tmp)
        try:
            extract(archive, work)
        except (ValueError, tarfile.TarError, zipfile.BadZipFile, OSError) as e:
            print(f"Entpacken fehlgeschlagen: {e}", flush=True)
            _say("FAIL", "sddm.archive")
            return 3
        roots = find_theme_roots(work, archive.name)
        if not roots:
            _say("FAIL", "sddm.notheme")
            return 4
        _say("STEP", "sddm.copy")
        _say("PROGRESS", "50")
        installed = []
        for name, src in roots:
            if not sddm.NAME_RE.match(name):
                continue
            target = dest_dir / name
            if target.exists():
                if name not in state:
                    print(f"{name}: existiert schon (nicht von hier installiert) – übersprungen", flush=True)
                    continue
                shutil.rmtree(target)
            shutil.copytree(src, target, symlinks=False,
                            ignore=lambda d, names: [n for n in names if os.path.islink(os.path.join(d, n))])
            if not _dry():
                _fix_perms(target)
            info = sddm.theme_info(target) or {"qt6": False}
            install_deps(qml_deps(target, info.get("qt6", False)))
            state[name] = {"store": store_id, "archive": archive.name}
            installed.append(name)
            print(f"Installiert: {target}", flush=True)
    if not installed:
        _say("FAIL", "sddm.exists")
        return 5
    save_store_state(state)
    print("@@THEME " + installed[0], flush=True)
    _say("PROGRESS", "100")
    _say("OK")
    return 0


def write_conf(path: Path, section: str, key: str, value: str) -> None:
    cp = configparser.ConfigParser(interpolation=None, strict=False)
    cp.optionxform = str
    owner = None
    if path.exists():
        owner = path.stat()
        cp.read(path, encoding="utf-8")
    if not cp.has_section(section):
        cp.add_section(section)
    cp.set(section, key, value)
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        cp.write(f, space_around_delimiters=False)
    if owner and hasattr(os, "chown"):
        os.chown(path, owner.st_uid, owner.st_gid)


def do_set(name: str) -> int:
    if not sddm.NAME_RE.match(name) or not (sddm.themes_dir() / name / "metadata.desktop").is_file():
        _say("FAIL", "sddm.notheme")
        return 2
    _say("STEP", "sddm.set")
    _say("PROGRESS", "40")
    # wie die KCM: /etc/sddm.conf.d/kde_settings.conf
    write_conf(sddm.kde_settings_file(), "Theme", "Current", name)
    print(f"{sddm.kde_settings_file()}: [Theme] Current={name}", flush=True)
    main_conf = sddm.main_conf_file()
    if main_conf.is_file() and sddm.read_ini(main_conf).has_option("Theme", "Current"):
        write_conf(main_conf, "Theme", "Current", name)   # /etc/sddm.conf hat Vorrang
        print(f"{main_conf}: [Theme] Current={name}", flush=True)
    _say("PROGRESS", "100")
    _say("OK")
    return 0


def do_remove(name: str) -> int:
    state = sddm.load_store_state()
    if not sddm.NAME_RE.match(name) or name not in state:
        _say("FAIL", "sddm.foreign")
        return 2
    if sddm.current_theme() == name:
        _say("FAIL", "sddm.active")
        return 3
    _say("STEP", "sddm.remove")
    target = sddm.themes_dir() / name
    if target.is_dir() and not target.is_symlink():
        shutil.rmtree(target)
        print(f"Entfernt: {target}", flush=True)
    state.pop(name, None)
    save_store_state(state)
    _say("PROGRESS", "100")
    _say("OK")
    return 0


def do_dm_install() -> int:
    from . import helper as H
    backend = system.detect_backend()
    if not backend:
        _say("FAIL", "nobackend")
        return 11
    _say("STEP", "dm.install")
    _say("PROGRESS", "10")
    before = H.dm_snapshot()
    env = {"DEBIAN_FRONTEND": "noninteractive"} if backend == "apt" else None
    cmds = system.install_commands(backend, ["sddm"])
    for i, cmd in enumerate(cmds):
        _say("PROGRESS", str(10 + int(80 * i / len(cmds))))
        if H.run(cmd, env) != 0:
            H.dm_restore(before)
            _say("FAIL", "install")
            return 1
    H.dm_restore(before)                     # nur installieren, nicht umschalten
    if _dry():
        f = system._dry_dm_file().parent / "sddm-installed"
        f.parent.mkdir(parents=True, exist_ok=True)
        f.write_text("1", encoding="utf-8")
    _say("PROGRESS", "100")
    _say("OK")
    return 0


def do_dm_enable(name: str) -> int:
    from . import helper as H
    if name not in system.DM_SWITCHABLE or not system.dm_unit_exists(name):
        _say("FAIL", "dm.unit")
        return 2
    _say("STEP", "dm.enable")
    _say("PROGRESS", "30")
    if H.run(["systemctl", "enable", "--force", f"{name}.service"]) != 0:
        _say("FAIL", "dm.unit")
        return 1
    debian = Path("/etc/X11/default-display-manager")
    exe = shutil.which(name)
    if not _dry() and debian.is_file() and exe:
        debian.write_text(exe + "\n", encoding="utf-8")
        print(f"{debian}: {exe}", flush=True)
    if _dry():
        f = system._dry_dm_file()
        f.parent.mkdir(parents=True, exist_ok=True)
        f.write_text(name, encoding="utf-8")
    _say("PROGRESS", "100")
    _say("OK")
    return 0


def main(argv: list) -> int:
    action = argv[0]
    if action == "dm-install":
        return do_dm_install()
    if action == "dm-enable":
        return do_dm_enable(argv[1])
    if action == "sddm-install":
        store = next((a.split("=", 1)[1] for a in argv[2:] if a.startswith("--store=")), "")
        return do_install(argv[1], store if store.isdigit() else "")
    if action == "sddm-set":
        return do_set(argv[1])
    if action == "sddm-remove":
        return do_remove(argv[1])
    raise KeyError(action)
