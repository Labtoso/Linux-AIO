"""The one colour source for every built-in preset (Kitty, Alacritty, Hyprland, Waybar, launchers, lock screen,
title bar buttons, app accents): same names, same order, same colours and font everywhere. No Qt.

"01" is Sebastian's own setup (Tokyo Night colours, see the Kitty preset); the others are the usual schemes."""

ORDER = ("01", "tokyo", "catppuccin", "nord", "dracula", "gruvbox", "onedark", "rosepine", "everforest")
FONT, FONT_SIZE = "JetBrainsMono Nerd Font", 11.5

_TOKYO = {"bg": "#1a1b26", "bg2": "#24283b", "fg": "#c0caf5", "muted": "#414868", "accent": "#7aa2f7",
          "accent2": "#bb9af7", "red": "#f7768e", "yellow": "#e0af68", "green": "#9ece6a",
          "ansi": ["#15161e", "#f7768e", "#9ece6a", "#e0af68", "#7aa2f7", "#bb9af7", "#7dcfff", "#a9b1d6",
                   "#414868", "#f7768e", "#9ece6a", "#e0af68", "#7aa2f7", "#bb9af7", "#7dcfff", "#c0caf5"]}

PALETTES = {
    # ui = accent of the Linux AIO / Hypr Studio interface (01: the white Sebastian uses)
    "01": dict(_TOKYO, title="01", ui="#eef2f7"),
    "tokyo": dict(_TOKYO, title="Tokyo Night", ui="#7aa2f7"),
    "catppuccin": {"title": "Catppuccin", "bg": "#1e1e2e", "bg2": "#313244", "fg": "#cdd6f4", "muted": "#45475a",
                   "accent": "#cba6f7", "accent2": "#89b4fa", "red": "#f38ba8", "yellow": "#f9e2af",
                   "green": "#a6e3a1", "ui": "#cba6f7",
                   "ansi": ["#45475a", "#f38ba8", "#a6e3a1", "#f9e2af", "#89b4fa", "#f5c2e7", "#94e2d5", "#bac2de",
                            "#585b70", "#f38ba8", "#a6e3a1", "#f9e2af", "#89b4fa", "#f5c2e7", "#94e2d5", "#a6adc8"]},
    "nord": {"title": "Nord", "bg": "#2e3440", "bg2": "#3b4252", "fg": "#d8dee9", "muted": "#4c566a",
             "accent": "#88c0d0", "accent2": "#81a1c1", "red": "#bf616a", "yellow": "#ebcb8b", "green": "#a3be8c",
             "ui": "#88c0d0",
             "ansi": ["#3b4252", "#bf616a", "#a3be8c", "#ebcb8b", "#81a1c1", "#b48ead", "#88c0d0", "#e5e9f0",
                      "#4c566a", "#bf616a", "#a3be8c", "#ebcb8b", "#81a1c1", "#b48ead", "#8fbcbb", "#eceff4"]},
    "dracula": {"title": "Dracula", "bg": "#282a36", "bg2": "#44475a", "fg": "#f8f8f2", "muted": "#6272a4",
                "accent": "#bd93f9", "accent2": "#ff79c6", "red": "#ff5555", "yellow": "#f1fa8c", "green": "#50fa7b",
                "ui": "#bd93f9",
                "ansi": ["#21222c", "#ff5555", "#50fa7b", "#f1fa8c", "#bd93f9", "#ff79c6", "#8be9fd", "#f8f8f2",
                         "#6272a4", "#ff6e6e", "#69ff94", "#ffffa5", "#d6acff", "#ff92df", "#a4ffff", "#ffffff"]},
    "gruvbox": {"title": "Gruvbox", "bg": "#282828", "bg2": "#3c3836", "fg": "#ebdbb2", "muted": "#504945",
                "accent": "#fabd2f", "accent2": "#fe8019", "red": "#fb4934", "yellow": "#d79921", "green": "#b8bb26",
                "ui": "#fe8019",
                "ansi": ["#282828", "#cc241d", "#98971a", "#d79921", "#458588", "#b16286", "#689d6a", "#a89984",
                         "#928374", "#fb4934", "#b8bb26", "#fabd2f", "#83a598", "#d3869b", "#8ec07c", "#ebdbb2"]},
    "onedark": {"title": "One Dark", "bg": "#282c34", "bg2": "#3e4451", "fg": "#abb2bf", "muted": "#5c6370",
                "accent": "#61afef", "accent2": "#c678dd", "red": "#e06c75", "yellow": "#e5c07b", "green": "#98c379",
                "ui": "#61afef",
                "ansi": ["#282c34", "#e06c75", "#98c379", "#e5c07b", "#61afef", "#c678dd", "#56b6c2", "#abb2bf",
                         "#5c6370", "#e06c75", "#98c379", "#e5c07b", "#61afef", "#c678dd", "#56b6c2", "#ffffff"]},
    "rosepine": {"title": "Rosé Pine", "bg": "#191724", "bg2": "#26233a", "fg": "#e0def4", "muted": "#6e6a86",
                 "accent": "#ebbcba", "accent2": "#c4a7e7", "red": "#eb6f92", "yellow": "#f6c177", "green": "#9ccfd8",
                 "ui": "#ebbcba",
                 "ansi": ["#26233a", "#eb6f92", "#31748f", "#f6c177", "#9ccfd8", "#c4a7e7", "#ebbcba", "#e0def4",
                          "#6e6a86", "#eb6f92", "#31748f", "#f6c177", "#9ccfd8", "#c4a7e7", "#ebbcba", "#e0def4"]},
    "everforest": {"title": "Everforest", "bg": "#2d353b", "bg2": "#343f44", "fg": "#d3c6aa", "muted": "#475258",
                   "accent": "#a7c080", "accent2": "#83c092", "red": "#e67e80", "yellow": "#dbbc7f",
                   "green": "#a7c080", "ui": "#a7c080",
                   "ansi": ["#475258", "#e67e80", "#a7c080", "#dbbc7f", "#7fbbb3", "#d699b6", "#83c092", "#d3c6aa",
                            "#475258", "#e67e80", "#a7c080", "#dbbc7f", "#7fbbb3", "#d699b6", "#83c092", "#d3c6aa"]},
}

# old app accent names → palette (keeps the colour someone picked before 2.5.1)
LEGACY_ACCENT = {"white": "01", "blue": "tokyo", "teal": "nord", "violet": "dracula", "orange": "gruvbox",
                 "mint": "everforest"}


def title(key: str) -> str:
    return PALETTES[key]["title"]


def themed(make, extras: dict | None = None, rest: dict | None = None) -> dict:
    """Presets dict: first the shared palettes in ORDER (colours from make(palette), plus non-colour extras per
    name), then the other presets of that program (shapes, layouts …)."""
    out = {}
    for key in ORDER:
        p = dict(make(PALETTES[key]))
        p.update((extras or {}).get(key, {}))
        out[key] = p
    for key, value in (rest or {}).items():
        if key not in out:
            out[key] = value
    return out


def register_titles(text: dict, *prefixes: str) -> None:
    """Same names in every language and program: prefix + key → title."""
    for lang in text.values():
        for pre in prefixes:
            for key in ORDER:
                lang[pre + key] = PALETTES[key]["title"]


def unify(old: dict, make, aliases: dict | None = None) -> dict:
    """Turn a program's preset dict into the shared set: colour keys from make(palette), the program's own
    non-colour settings of that name (radius, gaps …) are kept; then all other presets unchanged."""
    aliases = {"01": "tokyo", **(aliases or {})}
    out = {}
    for key in ORDER:
        colours = make(PALETTES[key])
        src = old.get(key) or old.get(aliases.get(key, ""), {})
        out[key] = {k: v for k, v in src.items() if k not in colours} | colours
    gone = set(ORDER) | set(aliases.values())          # alte Namen (z. B. „rose“) gehen im neuen auf
    out.update((k, v) for k, v in old.items() if k not in gone)
    return out
