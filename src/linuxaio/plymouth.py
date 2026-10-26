"""Plymouth-Bootsplash: Themes lesen, aktives Theme, Einrichtungsstatus, KDE Store (wie plymouth-kcm).

plymouth-kcm: KNewStuff-Kategorie "Plymouth Themes" (OCS-ID 108), Themes nach /usr/share/plymouth/themes,
Setzen über /etc/plymouth/plymouthd.conf [Daemon] Theme=, auf Debian/Ubuntu zusätzlich update-alternatives,
danach Initramfs neu bauen (update-initramfs -u bzw. mkinitcpio -P).
"""

import os
import re
import shutil
import tempfile
from pathlib import Path
from urllib.parse import urlencode

from . import sddm

DRY = os.environ.get("DEMANAGER_DRYRUN") == "1" or os.name == "nt"
CATEGORY = "108"
STORE_PAGE = sddm.STORE_PAGE
parse_items = sddm.parse_items
parse_item = sddm.parse_item
item_url = sddm.item_url
archive_ok = sddm.archive_ok
NAME_RE = sddm.NAME_RE


def _dry_base() -> Path:
    return Path(tempfile.gettempdir()) / "desddm-dry" / "plymouth"


def themes_dir() -> Path:
    return _dry_base() / "themes" if DRY else Path("/usr/share/plymouth/themes")


def conf_file() -> Path:
    return _dry_base() / "plymouthd.conf" if DRY else Path("/etc/plymouth/plymouthd.conf")


def defaults_file() -> Path:
    return _dry_base() / "plymouthd.defaults" if DRY else Path("/usr/share/plymouth/plymouthd.defaults")


def store_state_file() -> Path:
    return _dry_base() / "state.json" if DRY else Path("/var/lib/labtoso-linuxaio/plymouth.json")


def load_store_state() -> dict:
    from .system import load_state_file
    return load_state_file(store_state_file())


def is_installed() -> bool:
    if DRY:
        return (_dry_base() / "installed").exists()
    return shutil.which("plymouth") is not None or shutil.which("plymouthd") is not None


def current_theme() -> str:
    for f in (conf_file(), defaults_file()):
        v = sddm.read_ini(f).get("Daemon", "Theme", fallback="").strip()
        if v:
            return v
    return ""


def theme_file(path: Path) -> Path | None:
    own = path / f"{path.name}.plymouth"
    if own.is_file():
        return own
    return next(iter(sorted(path.glob("*.plymouth"))), None)


def theme_info(path: Path) -> dict | None:
    tf = theme_file(path)
    if not tf:
        return None
    cp = sddm.read_ini(tf)
    g = lambda k: cp.get("Plymouth Theme", k, fallback="").strip()   # noqa: E731
    shot = next((p for p in (path / "preview.png", path.parent / f"{path.name}.png", path / "preview.jpg")
                 if p.is_file()), None)
    return {
        "id": path.name,
        "name": g("Name") or path.name,
        "author": "",
        "description": g("Description"),
        "version": "",
        "module": g("ModuleName"),
        "broken": not (path / f"{path.name}.plymouth").is_file(),
        "screenshot": str(shot) if shot else "",
        "path": str(path),
    }


def list_themes() -> list:
    d = themes_dir()
    out = []
    if d.is_dir():
        for p in sorted(d.iterdir(), key=lambda x: x.name.lower()):
            if p.is_dir():
                info = theme_info(p)
                if info:
                    out.append(info)
    return out


FRAME_RE = re.compile(r"^(?P<base>.*?)[-_]?(?P<num>\d+)\.png$", re.I)
SKIP = ("box", "bullet", "entry", "lock", "capslock", "keyboard", "keymap", "password", "prompt", "dialog",
        "progress_bar", "progress-bar", "bar", "background", "bg", "wallpaper", "corner", "suspend", "header")


def _color(v: str) -> str:
    v = (v or "").strip().lower().replace("0x", "").replace("#", "")
    return f"#{v[-6:]:0>6}" if re.fullmatch(r"[0-9a-f]{1,8}", v or "x") else ""


def preview_assets(path: Path) -> dict:
    """Bilder für eine animierte Vorschau: Einzelbilder, Logo, Hintergrund, Farben, Positionen."""
    tf = theme_file(path)
    cp = sddm.read_ini(tf) if tf else None
    module = (cp.get("Plymouth Theme", "ModuleName", fallback="") if cp else "").strip().lower()
    sec = module if cp is not None and cp.has_section(module) else None
    g = (lambda k, d="": cp.get(sec, k, fallback=d).strip()) if sec else (lambda k, d="": d)
    raw_dir = g("ImageDir")
    if not raw_dir:
        img_dir = path
    elif raw_dir.startswith("/"):
        # absolute Plymouth-Pfade (/usr/share/plymouth/themes/…); im Testmodus auf den Testordner umbiegen
        img_dir = themes_dir() / Path(raw_dir).name if DRY else Path(raw_dir)
    else:
        img_dir = path / raw_dir
    if not img_dir.is_dir():
        img_dir = path
    pngs = sorted(p for p in img_dir.glob("*.png") if p.is_file())
    groups = {}
    for p in pngs:
        m = FRAME_RE.match(p.name)
        if m:
            groups.setdefault(m.group("base").lower(), []).append((int(m.group("num")), p))
    frames = []
    if groups:
        # die lange Hauptanimation zuerst (two-step: animation-* = das eigentliche Bild, throbber-* nur Ladepunkte)
        def rank(item):
            base, lst = item
            pref = next((i for i, k in enumerate(("animation", "throbber", "spinner", "loading", "frame", "progress"))
                         if k in base), 9)
            return (-len(lst), pref)
        base, lst = min(((b, l) for b, l in groups.items() if len(l) >= 3), key=rank, default=(None, []))
        frames = [str(p) for _, p in sorted(lst)][:600]
    in_groups = {str(p) for lst in groups.values() for _, p in lst}
    named = {p.stem.lower(): p for p in pngs}
    logo = next((named[n] for n in ("watermark", "logo", "header-image", "splash", "distro-logo") if n in named), None)
    if logo is None:
        rest = [p for p in pngs if str(p) not in in_groups and not any(s in p.stem.lower() for s in SKIP)]
        logo = max(rest, key=lambda p: p.stat().st_size, default=None)
    bgimg = next((named[n] for n in ("background", "bg", "wallpaper", "background-tile") if n in named), None)
    bg = _color(g("BackgroundStartColor")) or _color(g("BackgroundColor")) or "#000000"
    bg2 = _color(g("BackgroundEndColor"))
    try:
        ha = float(g("HorizontalAlignment", ".5"))
        va = float(g("VerticalAlignment", ".75" if module == "two-step" else ".5"))
        wva = float(g("WatermarkVerticalAlignment", ".96" if module == "two-step" else ".4"))
    except ValueError:
        ha, va, wva = .5, .6, .4
    if module != "two-step" and logo and frames:
        va, wva = .66, .42
    return {"frames": frames, "logo": str(logo) if logo else "", "background": str(bgimg) if bgimg else "",
            "bg": bg, "bg2": bg2 if bg2 and bg2 != bg else "", "ha": ha, "va": va, "wva": wva}


def theme_chips(th: dict) -> list:
    from .i18n import t
    chips = [(th["module"], "Chip", "")] if th.get("module") else []
    if th.get("broken"):
        chips.append((t("ply.broken"), "ChipWarn", t("ply.broken.tip")))
    return chips


def test_command(th: dict):
    return None


def list_url(page: int = 0, sort: str = "down", search: str = "") -> str:
    q = {"categories": CATEGORY, "format": "json", "pagesize": sddm.PAGE_SIZE, "page": page,
         "sortmode": sort if sort in sddm.SORTS else "down"}
    if search.strip():
        q["search"] = search.strip()
    return f"{sddm.OCS}/content/data?{urlencode(q)}"


# ---------- Einrichtung (Hook + Kernel-Parameter) ----------

def mkinitcpio_files() -> list:
    files = [Path("/etc/mkinitcpio.conf")]
    d = Path("/etc/mkinitcpio.conf.d")
    if d.is_dir():
        files += sorted(d.glob("*.conf"))
    return [f for f in files if f.is_file()]


HOOKS_RE = re.compile(r"^(\s*HOOKS\s*=\s*\()([^)]*)(\).*)$", re.M)


def hooks_file() -> Path | None:
    """Datei, deren HOOKS=-Zeile zuletzt gilt."""
    last = None
    for f in mkinitcpio_files():
        try:
            if HOOKS_RE.search(f.read_text(encoding="utf-8")):
                last = f
        except OSError:
            pass
    return last


def add_hook(text: str) -> str:
    """Fügt 'plymouth' nach systemd/udev/base in HOOKS=(...) ein (vor encrypt)."""
    def fix(m):
        hooks = m.group(2).split()
        if "plymouth" in hooks or "sd-plymouth" in hooks:
            return m.group(0)
        pos = 0
        for anchor in ("systemd", "udev", "base"):
            if anchor in hooks:
                pos = hooks.index(anchor) + 1
                break
        for blocker in ("encrypt", "sd-encrypt"):
            if blocker in hooks:
                pos = min(pos, hooks.index(blocker))
        hooks.insert(pos, "plymouth")
        return m.group(1) + " ".join(hooks) + m.group(3)
    return HOOKS_RE.sub(fix, text)


def has_hook() -> bool | None:
    if DRY:
        return is_installed()
    f = hooks_file()
    if not f:
        return None          # kein mkinitcpio (dracut/initramfs-tools bringen Plymouth selbst mit)
    m = HOOKS_RE.search(f.read_text(encoding="utf-8"))
    return bool(m and ("plymouth" in m.group(2).split() or "sd-plymouth" in m.group(2).split()))


def add_params(cmdline: str, params=("quiet", "splash")) -> str:
    parts = cmdline.split()
    for p in params:
        if p not in parts:
            parts.append(p)
    return " ".join(parts)


def splash_active() -> bool:
    if DRY:
        return is_installed()
    try:
        return "splash" in Path("/proc/cmdline").read_text().split()
    except OSError:
        return False


def is_ready() -> bool:
    return is_installed() and splash_active() and has_hook() is not False
