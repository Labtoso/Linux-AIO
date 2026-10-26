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
              "max": "#3a424d", "icons": "always"},
    "square": {"titel": "Eckig", "shape": "square", "size": 24, "close": "#3a424d", "min": "#3a424d",
               "max": "#3a424d", "icons": "always"},
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


def _rules(sel_btn: str, sel_img: str, p: dict) -> list:
    size = int(p.get("size", 24))
    radius = {"circle": "100%", "square": "4px", "wide": "0", "flat": "6px"}[p["shape"]]
    width = size * 2 if p["shape"] == "wide" else size
    out = [f"{sel_btn} {{ min-width: {width}px; min-height: {size}px; padding: 0; margin: 0 3px;"
           f" border-radius: {radius}; box-shadow: none; border: none;"
           f" background-image: none; }}",
           f"{sel_btn} {sel_img} {{ background: none; box-shadow: none; }}"]
    for cls, key in (("close", "close"), ("minimize", "min"), ("maximize", "max")):
        col = p.get(key) or "transparent"
        out.append(f"{sel_btn}.{cls} {{ background-color: {col}; }}")
    if p.get("hover_close"):
        out.append(f"{sel_btn}.close:hover {{ background-color: {p['hover_close']}; color: #ffffff; }}")
    if p.get("icons") == "hover":
        out.append(f"{sel_btn} {sel_img} {{ opacity: 0; }}")
        out.append(f"{sel_btn}:hover {sel_img} {{ opacity: 1; }}")
        out.append(f"{sel_btn} {sel_img} {{ color: rgba(0,0,0,0.6); }}")
    elif p.get("icons") == "never":
        out.append(f"{sel_btn} {sel_img} {{ opacity: 0; }}")
    return out


def css(name: str, gtk: int = 4) -> str:
    p = THEMES[name]
    if gtk == 4:
        rules = _rules("windowcontrols > button", "> image", p)
    else:
        rules = _rules("headerbar button.titlebutton", "image", p) + _rules(".titlebar button.titlebutton", "image", p)
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
