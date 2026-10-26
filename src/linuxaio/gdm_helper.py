"""Root-Teil für den GNOME-Anmeldebildschirm (GDM): Logo, Banner, Benutzerliste über dconf,
Hintergrund über eine neu gebaute gnome-shell-theme.gresource (Original wird einmal gesichert).

Aufrufe:  gdm-logo <png|--clear>   gdm-banner <text|--clear>   gdm-userlist <on|off>
          gdm-bg <bild|#rrggbb>    gdm-restore
"""

import json
import os
import re
import shutil
import subprocess
import tempfile
from pathlib import Path
from xml.sax.saxutils import escape

GRES = Path("/usr/share/gnome-shell/gnome-shell-theme.gresource")
STATE_DIR = Path("/var/lib/labtoso-linuxaio")
BACKUP = STATE_DIR / "gnome-shell-theme.gresource.original"
DCONF_FILE = Path("/etc/dconf/db/gdm.d/90-linuxaio")
DCONF_PROFILE = Path("/etc/dconf/profile/gdm")
LOGO = Path("/usr/share/pixmaps/linuxaio-gdm-logo.png")
PREFIX = "/org/gnome/shell/theme"
BG_NAME = "linuxaio-background"
MARK = "/* Linux AIO: Hintergrund */"


def _say(tag, text=""):
    print(f"@@{tag} {text}".rstrip(), flush=True)


def _dry() -> bool:
    return os.environ.get("DEMANAGER_DRYRUN") == "1"


def state_file() -> Path:
    if _dry():
        cache = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache"))
        return cache / "LabtosoStudios" / "linuxaio" / "testmodus" / "gdm.json"
    return STATE_DIR / "gdm.json"


def load() -> dict:
    try:
        return json.loads(state_file().read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return {}


def save(st: dict) -> None:
    p = state_file()
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(st, indent=2, ensure_ascii=False), encoding="utf-8")
    try:
        os.chmod(p, 0o644)
    except OSError:
        pass


def _run(cmd: list, **kw) -> subprocess.CompletedProcess | None:
    print("$ " + " ".join(cmd), flush=True)
    if _dry():
        return None
    return subprocess.run(cmd, capture_output=True, text=True, **kw)


def _image_ok(path: Path) -> bool:
    from .sddm_helper import is_image, user_owned
    try:
        st = path.stat()
    except OSError:
        return False
    return path.is_file() and user_owned(st) and is_image(path) and st.st_size < 40 * 1024 * 1024


def _gv(s: str) -> str:
    """Text als GVariant-String für die dconf-Schlüsseldatei."""
    return "'" + s.replace("\\", "\\\\").replace("'", "\\'").replace("\n", "\\n") + "'"


def write_dconf(st: dict) -> None:
    lines = ["# Linux AIO (LabtosoStudios)", "[org/gnome/login-screen]"]
    if st.get("logo"):
        lines.append(f"logo={_gv(st['logo'])}")
    if st.get("banner"):
        lines.append("banner-message-enable=true")
        lines.append(f"banner-message-text={_gv(st['banner'])}")
    else:
        lines.append("banner-message-enable=false")
    if "userlist" in st:
        lines.append("disable-user-list=" + ("false" if st["userlist"] else "true"))
    text = "\n".join(lines) + "\n"
    print(f"→ {DCONF_FILE}", flush=True)
    for ln in lines[1:]:
        print("  " + ln, flush=True)
    if _dry():
        return
    if not DCONF_PROFILE.exists():
        DCONF_PROFILE.parent.mkdir(parents=True, exist_ok=True)
        DCONF_PROFILE.write_text("user-db:user\nsystem-db:gdm\nfile-db:/usr/share/gdm/greeter-dconf-defaults\n",
                                 encoding="utf-8")
        print(f"→ {DCONF_PROFILE}", flush=True)
    DCONF_FILE.parent.mkdir(parents=True, exist_ok=True)
    DCONF_FILE.write_text(text, encoding="utf-8")
    _run(["dconf", "update"])


def do_logo(arg: str) -> int:
    st = load()
    _say("STEP", "gdm.logo")
    if arg == "--clear":
        st.pop("logo", None)
    else:
        src = Path(arg)
        if not _image_ok(src):
            _say("FAIL", "gdm.image")
            return 2
        print(f"{src} → {LOGO}", flush=True)
        if not _dry():
            LOGO.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(src, LOGO)
            os.chmod(LOGO, 0o644)
        st["logo"] = str(LOGO)
    write_dconf(st)
    save(st)
    _say("OK")
    return 0


def do_banner(arg: str) -> int:
    st = load()
    _say("STEP", "gdm.banner")
    if arg == "--clear":
        st.pop("banner", None)
    else:
        st["banner"] = arg[:500]
    write_dconf(st)
    save(st)
    _say("OK")
    return 0


def do_userlist(arg: str) -> int:
    st = load()
    st["userlist"] = arg == "on"
    write_dconf(st)
    save(st)
    _say("OK")
    return 0


def _css_rule(value: str, has_image: bool) -> str:
    if has_image:
        bg = (f'background: #000 url("resource://{PREFIX}/{BG_NAME}"); background-size: cover; '
              "background-position: center; background-repeat: no-repeat;")
    else:
        bg = f"background: {value}; background-image: none;"
    return f"\n{MARK}\n#lockDialogGroup {{ {bg} }}\n"


def do_bg(arg: str) -> int:
    is_color = bool(re.fullmatch(r"#[0-9a-fA-F]{6}", arg))
    src = None if is_color else Path(arg)
    if src is not None and not _image_ok(src):
        _say("FAIL", "gdm.image")
        return 2
    if not GRES.is_file() and not _dry():
        _say("FAIL", "gdm.missing")
        return 2
    _say("STEP", "gdm.bg")
    _say("PROGRESS", "10")
    if not _dry():
        STATE_DIR.mkdir(parents=True, exist_ok=True)
        if not BACKUP.exists():
            shutil.copy2(GRES, BACKUP)                      # Original genau einmal sichern
            print(f"Sicherung: {BACKUP}", flush=True)
    print(f"Quelle: {BACKUP} (Original)", flush=True)
    if _dry():
        for c in (["gresource", "list", str(BACKUP)], ["gresource", "extract", str(BACKUP), "…"],
                  ["glib-compile-resources", "…"], ["install", "-m644", "…", str(GRES)]):
            print("$ " + " ".join(c), flush=True)
        st = load()
        st["background"] = arg
        save(st)
        _say("PROGRESS", "100")
        _say("OK")
        return 0
    work = Path(tempfile.mkdtemp(prefix="linuxaio-gdm-"))
    try:
        res = subprocess.run(["gresource", "list", str(BACKUP)], capture_output=True, text=True)
        if res.returncode != 0:
            print(res.stderr, flush=True)
            _say("FAIL", "gdm.gresource")
            return 1
        names = [n.strip() for n in res.stdout.splitlines() if n.strip().startswith(PREFIX + "/")]
        files = []
        for n in names:
            rel = n[len(PREFIX) + 1:]
            out = work / rel
            out.parent.mkdir(parents=True, exist_ok=True)
            data = subprocess.run(["gresource", "extract", str(BACKUP), n], capture_output=True).stdout
            if rel.endswith(".css") and (rel.startswith(("gnome-shell", "gdm"))):
                text = data.decode("utf-8", "replace")
                text = text.split("\n" + MARK)[0] + _css_rule(arg, src is not None)
                data = text.encode("utf-8")
            out.write_bytes(data)
            files.append(rel)
        _say("PROGRESS", "50")
        if src is not None:
            shutil.copyfile(src, work / BG_NAME)
            files.append(BG_NAME)
        xml = ['<?xml version="1.0" encoding="UTF-8"?>', "<gresources>", f'  <gresource prefix="{PREFIX}">']
        xml += [f"    <file>{escape(f)}</file>" for f in files]
        xml += ["  </gresource>", "</gresources>"]
        (work / "theme.gresource.xml").write_text("\n".join(xml) + "\n", encoding="utf-8")
        target = work / "gnome-shell-theme.gresource"
        r = subprocess.run(["glib-compile-resources", "--sourcedir", str(work), "--target", str(target),
                            str(work / "theme.gresource.xml")], capture_output=True, text=True)
        if r.returncode != 0 or not target.is_file():
            print(r.stderr, flush=True)
            _say("FAIL", "gdm.gresource")
            return 1
        _say("PROGRESS", "85")
        shutil.copyfile(target, GRES)
        os.chmod(GRES, 0o644)
        print(f"→ {GRES}", flush=True)
    finally:
        shutil.rmtree(work, ignore_errors=True)
    st = load()
    st["background"] = arg
    save(st)
    _say("PROGRESS", "100")
    _say("OK")
    return 0


def do_restore() -> int:
    _say("STEP", "gdm.restore")
    if not _dry():
        if not BACKUP.exists():
            _say("FAIL", "gdm.nobackup")
            return 2
        shutil.copyfile(BACKUP, GRES)
        os.chmod(GRES, 0o644)
    print(f"{BACKUP} → {GRES}", flush=True)
    st = load()
    st.pop("background", None)
    save(st)
    _say("OK")
    return 0


def main(argv: list) -> int:
    action = argv[0]
    if action == "gdm-logo":
        return do_logo(argv[1])
    if action == "gdm-banner":
        return do_banner(argv[1])
    if action == "gdm-userlist":
        return do_userlist(argv[1])
    if action == "gdm-bg":
        return do_bg(argv[1])
    if action == "gdm-restore":
        return do_restore()
    raise KeyError(action)
