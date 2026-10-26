"""Knopf-Themes für die Titelleiste (GTK 3 + GTK 4/libadwaita): eigener CSS-Block in ~/.config/gtk-*/gtk.css
zwischen festen Markern – der Rest der Datei bleibt unangetastet. Ohne Qt."""

import re

from . import usercmd

START = "/* Linux AIO buttons start"
END = "/* Linux AIO buttons end */"

# shape: circle | square | wide | flat;  icons: Symbole immer / nur beim Darüberfahren / nie
THEMES = {
    "macos": {"titel": "macOS", "shape": "circle", "size": 14, "close": "#ff5f57", "min": "#febc2e",
              "max": "#28c840", "icons": "hover"},
    "round": {"titel": "Rund", "shape": "circle", "size": 24, "close": "#3a424d", "min": "#3a424d",
              "max": "#3a424d", "icons": "always", "icon_color": "#e6edf5"},
    "square": {"titel": "Eckig", "shape": "square", "size": 24, "close": "#3a424d", "min": "#3a424d",
               "max": "#3a424d", "icons": "always", "icon_color": "#e6edf5"},
    "minimal": {"titel": "Minimal", "shape": "flat", "size": 24, "close": "", "min": "", "max": "",
                "icons": "always"},
    "windows": {"titel": "Windows", "shape": "wide", "size": 30, "close": "", "min": "", "max": "",
                "hover_close": "#c42b1c", "icons": "always"},
    "material": {"titel": "Material", "shape": "circle", "size": 22, "close": "#ef5350", "min": "#ffca28",
                 "max": "#66bb6a", "icons": "always"},
    "catppuccin": {"titel": "Catppuccin", "shape": "circle", "size": 16, "close": "#f38ba8", "min": "#f9e2af",
                   "max": "#a6e3a1", "icons": "hover"},
    "nord": {"titel": "Nord", "shape": "circle", "size": 16, "close": "#bf616a", "min": "#ebcb8b",
             "max": "#a3be8c", "icons": "hover"},
}


def _rules(sel_btn: str, sel_img: str, p: dict, gtk: int = 4) -> list:
    """libadwaita streckt die Knöpfe auf die Leistenhöhe → Punkt/Kachel wird auf das Symbol (image) gemalt,
    der Knopf selbst bleibt durchsichtig (so macht es libadwaita auch)."""
    size = int(p.get("size", 24))
    shape = p["shape"]
    img = f"{sel_btn} {sel_img}"
    out = [f"{sel_btn} {{ background: none; box-shadow: none; border: none; outline: none;"
           f" min-width: 0; min-height: 0; padding: 0 {4 if shape != 'wide' else 0}px; margin: 0; }}"]
    if shape in ("circle", "square"):
        icon = max(8, size - 6) if gtk == 4 else 16
        pad = max(0, (size - icon) // 2) if gtk == 4 else max(0, (size - 16) // 2)
        radius = "100%" if shape == "circle" else "4px"
        size_rule = f" -gtk-icon-size: {icon}px;" if gtk == 4 else ""
        out.append(f"{img} {{ border-radius: {radius}; padding: {pad}px; min-width: 0; min-height: 0;"
                   f" box-shadow: none;{size_rule} }}")
        for cls, key in (("close", "close"), ("minimize", "min"), ("maximize", "max")):
            out.append(f"{sel_btn}.{cls} {sel_img} {{ background-color: {p.get(key) or 'transparent'}; }}")
        dark = "rgba(0,0,0,0.65)"
        light = p.get("icon_color")
        if p.get("icons") == "hover":
            out.append(f"{img} {{ color: transparent; }}")
            out.append(f"{sel_btn}:hover {sel_img} {{ color: {light or dark}; }}")
        elif p.get("icons") == "never":
            out.append(f"{img} {{ color: transparent; }}")
        elif light:
            out.append(f"{img} {{ color: {light}; }}")
        out.append(f"{sel_btn}:hover {sel_img}, {sel_btn}:active {sel_img} {{ box-shadow: inset 0 0 0 20px rgba(255,255,255,0.15); }}")
    elif shape == "wide":     # Windows: breite Flächen, Schließen wird rot
        out.append(f"{sel_btn} {{ min-width: {size + 16}px; border-radius: 0; }}")
        out.append(f"{img} {{ background: none; box-shadow: none; }}")
        out.append(f"{sel_btn}:hover {{ background-color: rgba(127,127,127,0.18); }}")
        if p.get("hover_close"):
            out.append(f"{sel_btn}.close:hover {{ background-color: {p['hover_close']}; color: #ffffff; }}")
    else:                     # flat: nur Symbole
        out.append(f"{img} {{ background: none; box-shadow: none; }}")
        out.append(f"{sel_btn}:hover {sel_img} {{ background-color: rgba(127,127,127,0.18); border-radius: 6px; }}")
    return out


def css(name: str, gtk: int = 4) -> str:
    p = THEMES[name]
    if gtk == 4:
        rules = _rules("windowcontrols > button", "> image", p, 4)
    else:
        rules = _rules("headerbar button.titlebutton", "image", p, 3) + _rules(".titlebar button.titlebutton", "image", p, 3)
    return "\n".join([f"{START}: {name} */", *rules, END])


def files() -> list:
    return [usercmd.config_dir() / "gtk-4.0" / "gtk.css", usercmd.config_dir() / "gtk-3.0" / "gtk.css"]


def _strip(text: str) -> str:
    return re.sub(re.escape(START) + r".*?" + re.escape(END) + r"\n?", "", text, flags=re.S)


def apply(name: str, log=usercmd.log) -> None:
    for f, gtk in zip(files(), (4, 3)):
        old = usercmd.readable(f).read_text(encoding="utf-8", errors="replace") if usercmd.readable(f).is_file() else ""
        body = _strip(old).rstrip("\n")
        new = (body + "\n\n" if body else "") + css(name, gtk) + "\n"
        f.parent.mkdir(parents=True, exist_ok=True)
        f.write_text(new, encoding="utf-8")
        log(f"→ {f}: Knopf-Theme {name}")


def remove(log=usercmd.log) -> None:
    for f in files():
        src = usercmd.readable(f)
        if not src.is_file():
            continue
        old = src.read_text(encoding="utf-8", errors="replace")
        new = _strip(old)
        new = new.rstrip("\n") + "\n" if new.strip() else ""     # die beim Anwenden eingefügte Leerzeile mit weg
        if new != old:
            f.parent.mkdir(parents=True, exist_ok=True)
            f.write_text(new, encoding="utf-8")
            log(f"→ {f}: Knopf-Theme entfernt")


def current() -> str:
    f = usercmd.readable(files()[0])
    try:
        m = re.search(re.escape(START) + r": (\w+) \*/", f.read_text(encoding="utf-8", errors="replace"))
    except OSError:
        return ""
    return m.group(1) if m else ""
