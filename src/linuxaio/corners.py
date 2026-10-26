"""Eckenrundung der App-Fenster: eigener Block in ~/.config/gtk-4.0/gtk.css und gtk-3.0/gtk.css zwischen Markern.
Für alle Fenster (auch Nicht-GTK) optional die GNOME-Erweiterung „Rounded Window Corners Reborn“. Ohne Qt."""

import re

from . import usercmd

START = "/* Linux AIO corners start"
END = "/* Linux AIO corners end */"
EXT_UUID = "rounded-window-corners@fxgn"
EXT_SCHEMA = "org.gnome.shell.extensions.rounded-window-corners-reborn"
EXT_KEY = "global-rounded-corner-settings"


def css(radius: int, gtk: int = 4) -> str:
    r = max(0, min(24, int(radius)))
    if gtk == 4:
        rules = [
            f":root {{ --window-radius: {r}px; }}",                                  # libadwaita 1.6+
            f"window.csd, window.csd.background, window.solid-csd {{ border-radius: {r}px; }}",
            f"window.csd headerbar, window.csd .titlebar {{ border-top-left-radius: {r}px;"
            f" border-top-right-radius: {r}px; }}",
        ]
    else:
        rules = [
            f"decoration {{ border-radius: {r}px; }}",
            f"window.csd, window.csd.background {{ border-radius: {r}px; }}",
            f"headerbar.titlebar, .titlebar:not(headerbar) {{ border-top-left-radius: {r}px;"
            f" border-top-right-radius: {r}px; }}",
        ]
    return "\n".join([f"{START}: {r} */", *rules, END])


def files() -> list:
    return [usercmd.config_dir() / "gtk-4.0" / "gtk.css", usercmd.config_dir() / "gtk-3.0" / "gtk.css"]


def _strip(text: str) -> str:
    return re.sub(re.escape(START) + r".*?" + re.escape(END) + r"\n?", "", text, flags=re.S)


def apply(radius: int, log=usercmd.log) -> None:
    for f, gtk in zip(files(), (4, 3)):
        src = usercmd.readable(f)
        old = src.read_text(encoding="utf-8", errors="replace") if src.is_file() else ""
        body = _strip(old).rstrip("\n")
        f.parent.mkdir(parents=True, exist_ok=True)
        f.write_text((body + "\n\n" if body else "") + css(radius, gtk) + "\n", encoding="utf-8")
        log(f"→ {f}: Eckenrundung {radius} px")


def remove(log=usercmd.log) -> None:
    for f in files():
        src = usercmd.readable(f)
        if not src.is_file():
            continue
        old = src.read_text(encoding="utf-8", errors="replace")
        new = _strip(old)
        new = new.rstrip("\n") + "\n" if new.strip() else ""
        if new != old:
            f.parent.mkdir(parents=True, exist_ok=True)
            f.write_text(new, encoding="utf-8")
            log(f"→ {f}: Eckenrundung entfernt")


def current() -> int | None:
    try:
        text = usercmd.readable(files()[0]).read_text(encoding="utf-8", errors="replace")
    except OSError:
        return None
    m = re.search(re.escape(START) + r": (\d+) \*/", text)
    return int(m.group(1)) if m else None


def set_extension_radius(radius: int) -> bool:
    """Radius der Erweiterung im a{sv}-Wörterbuch ändern (Rest bleibt, wie er ist)."""
    if not usercmd.has_key(EXT_SCHEMA, EXT_KEY) and not usercmd.DRY:
        return False
    raw = usercmd.query(["gsettings", "get", EXT_SCHEMA, EXT_KEY]) or ""
    new, n = re.subn(r"('borderRadius':\s*<(?:uint32\s+)?)(\d+)(>)", lambda m: f"{m.group(1)}{int(radius)}{m.group(3)}", raw)
    if not n:
        return False
    return usercmd.run(["gsettings", "set", EXT_SCHEMA, EXT_KEY, new])[0]
