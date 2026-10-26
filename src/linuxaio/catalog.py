"""Alle Desktop-Umgebungen mit je 3 Varianten (minimal, standard, komplett).

Paket-Schreibweise pro Paketmanager:
  "a|b"          – erstes verfügbares nehmen
  "?a"           – optional, wird übersprungen wenn nicht verfügbar
  "@gruppe"      – dnf-Gruppe
  "pattern:name" – zypper-Pattern
"""

from dataclasses import dataclass, field

VARIANTS = ("minimal", "standard", "full")
BACKENDS = ("pacman", "apt", "dnf", "zypper")


@dataclass(frozen=True)
class Desktop:
    id: str
    name: str
    sessions: tuple          # Namen der .desktop-Sessiondateien, bevorzugte zuerst
    detect: tuple            # Werte von XDG_CURRENT_DESKTOP
    wayland: bool
    x11: bool
    tiling: bool
    color: str               # Wallpaper-Farbe der Vorschau
    layout: str              # Vorschau-Layout
    desc: dict
    packages: dict = field(default_factory=dict)   # variant -> backend -> [specs]

    def specs(self, variant: str, backend: str) -> list:
        return list(self.packages.get(variant, {}).get(backend, []))

    def supports(self, variant: str, backend: str) -> bool:
        return bool(self.specs(variant, backend))


def _pk(minimal: dict, standard: dict, full: dict) -> dict:
    return {"minimal": minimal, "standard": standard, "full": full}


XORG = "xorg-server"

DESKTOPS = (
    Desktop(
        "gnome", "GNOME", ("gnome", "gnome-xorg"), ("GNOME",), True, True, False,
        "#1d3557", "gnome",
        {"de": "GNOME wie vom Projekt gedacht: ohne Anpassungen", "en": "GNOME as intended upstream, unmodified"},
        _pk(
            {"pacman": ["gnome-shell", "gnome-session", "gnome-control-center", "gnome-settings-daemon",
                        "nautilus", "gnome-console", "gnome-keyring", "gnome-backgrounds", "xdg-desktop-portal-gnome"],
             "apt": ["vanilla-gnome-desktop|gnome-core"],
             "dnf": ["gnome-shell", "gnome-session-wayland-session|gnome-session", "gnome-control-center",
                     "nautilus", "ptyxis|gnome-terminal"],
             "zypper": ["pattern:gnome_basic"]},
            {"pacman": ["gnome"],
             "apt": ["vanilla-gnome-desktop|gnome", "gnome-tweaks"],
             "dnf": ["@gnome-desktop"],
             "zypper": ["pattern:gnome"]},
            {"pacman": ["gnome", "gnome-extra", "gnome-tweaks", "?extension-manager",
                        "?gnome-shell-extension-appindicator", "?gnome-shell-extension-dash-to-dock"],
             "apt": ["gnome", "gnome-tweaks", "?gnome-shell-extension-manager",
                     "?gnome-shell-extension-dashtodock", "?gnome-shell-extension-appindicator"],
             "dnf": ["@gnome-desktop", "gnome-tweaks", "?gnome-extensions-app",
                     "?gnome-shell-extension-dash-to-dock", "?gnome-shell-extension-appindicator"],
             "zypper": ["pattern:gnome", "?gnome-tweaks", "?gnome-shell-extension-dash-to-dock"]},
        ),
    ),
    Desktop(
        "plasma", "KDE Plasma", ("plasma", "plasmax11", "plasmawayland"), ("KDE",), True, True, False,
        "#14324a", "plasma",
        {"de": "Flexibler Desktop, frei anpassbar", "en": "Flexible, fully customisable desktop"},
        _pk(
            {"pacman": ["plasma-desktop", "konsole", "dolphin", "plasma-pa", "plasma-nm", "kscreen", "breeze-gtk"],
             "apt": ["kde-plasma-desktop"],
             "dnf": ["plasma-desktop", "konsole", "dolphin"],
             "zypper": ["pattern:kde_plasma"]},
            {"pacman": ["plasma", "konsole", "dolphin", "kate", "ark", "spectacle"],
             "apt": ["kde-standard"],
             "dnf": ["@kde-desktop"],
             "zypper": ["pattern:kde"]},
            {"pacman": ["plasma", "kde-applications-meta"],
             "apt": ["kde-full"],
             "dnf": ["@kde-desktop", "?@kde-apps"],
             "zypper": ["pattern:kde", "?pattern:kde_applications"]},
        ),
    ),
    Desktop(
        "xfce", "Xfce", ("xfce",), ("XFCE",), False, True, False,
        "#2b3a2f", "xfce",
        {"de": "Leicht und klassisch", "en": "Light and classic"},
        _pk(
            {"pacman": [XORG, "xfce4"], "apt": ["xfce4"],
             "dnf": ["xfce4-session", "xfwm4", "xfce4-panel", "xfdesktop", "Thunar", "xfce4-terminal"],
             "zypper": ["pattern:xfce_basic"]},
            {"pacman": [XORG, "xfce4", "xfce4-goodies"], "apt": ["xfce4", "xfce4-goodies"],
             "dnf": ["@xfce-desktop"], "zypper": ["pattern:xfce"]},
            {"pacman": [XORG, "xfce4", "xfce4-goodies", "network-manager-applet", "pavucontrol", "gvfs"],
             "apt": ["xubuntu-desktop|task-xfce-desktop"],
             "dnf": ["@xfce-desktop", "?@xfce-apps", "?@xfce-extra-plugins"],
             "zypper": ["pattern:xfce", "?pattern:xfce_office"]},
        ),
    ),
    Desktop(
        "cinnamon", "Cinnamon", ("cinnamon", "cinnamon-wayland"), ("X-Cinnamon", "Cinnamon"), False, True, False,
        "#3a2f22", "cinnamon",
        {"de": "Vertrautes Startmenü-Layout (Linux Mint)", "en": "Familiar start-menu layout (Linux Mint)"},
        _pk(
            {"pacman": [XORG, "cinnamon"], "apt": ["cinnamon-core"], "dnf": ["cinnamon"], "zypper": ["cinnamon"]},
            {"pacman": [XORG, "cinnamon", "nemo-fileroller", "gnome-terminal"], "apt": ["cinnamon"],
             "dnf": ["@cinnamon-desktop|cinnamon"], "zypper": ["pattern:cinnamon|cinnamon"]},
            {"pacman": [XORG, "cinnamon", "nemo-fileroller", "gnome-terminal", "xed", "xreader", "blueman"],
             "apt": ["cinnamon-desktop-environment"], "dnf": ["@cinnamon-desktop|cinnamon"],
             "zypper": ["pattern:cinnamon|cinnamon", "?nemo-extension-fileroller"]},
        ),
    ),
    Desktop(
        "mate", "MATE", ("mate",), ("MATE",), False, True, False,
        "#26402a", "mate",
        {"de": "Klassisches GNOME-2-Erbe", "en": "Classic GNOME 2 heritage"},
        _pk(
            {"pacman": [XORG, "mate"], "apt": ["mate-desktop-environment-core"], "dnf": ["@mate-desktop"],
             "zypper": ["pattern:mate_basic"]},
            {"pacman": [XORG, "mate", "mate-extra"], "apt": ["mate-desktop-environment"], "dnf": ["@mate-desktop"],
             "zypper": ["pattern:mate"]},
            {"pacman": [XORG, "mate", "mate-extra", "network-manager-applet"],
             "apt": ["mate-desktop-environment-extras"], "dnf": ["@mate-desktop", "?@mate-applications"],
             "zypper": ["pattern:mate", "?pattern:mate_utilities"]},
        ),
    ),
    Desktop(
        "budgie", "Budgie", ("budgie-desktop",), ("Budgie", "Budgie:GNOME"), False, True, False,
        "#2d2440", "budgie",
        {"de": "Modern, schlicht, mit Seitenleiste", "en": "Modern and simple with a sidebar"},
        _pk(
            {"pacman": [XORG, "budgie-desktop"], "apt": ["budgie-desktop"], "dnf": ["budgie-desktop"],
             "zypper": ["budgie"]},
            {"pacman": [XORG, "budgie-desktop", "budgie-control-center", "nemo", "gnome-terminal"],
             "apt": ["budgie-desktop", "?budgie-indicator-applet"], "dnf": ["@budgie-desktop|budgie-desktop"],
             "zypper": ["pattern:budgie|budgie"]},
            {"pacman": [XORG, "budgie", "?budgie-extras", "nemo", "gnome-terminal"],
             "apt": ["ubuntu-budgie-desktop|budgie-desktop"], "dnf": ["@budgie-desktop|budgie-desktop",
                                                                      "?@budgie-desktop-apps"],
             "zypper": ["pattern:budgie|budgie", "?budgie-extras"]},
        ),
    ),
    Desktop(
        "lxqt", "LXQt", ("lxqt", "lxqt-wayland"), ("LXQt",), False, True, False,
        "#1f2a3a", "lxqt",
        {"de": "Sehr leicht, Qt-basiert", "en": "Very light, Qt based"},
        _pk(
            {"pacman": [XORG, "lxqt"], "apt": ["lxqt-core"], "dnf": ["@lxqt-desktop"], "zypper": ["pattern:lxqt"]},
            {"pacman": [XORG, "lxqt", "breeze-icons", "xdg-utils"], "apt": ["lxqt"], "dnf": ["@lxqt-desktop"],
             "zypper": ["pattern:lxqt"]},
            {"pacman": [XORG, "lxqt", "breeze-icons", "xdg-utils", "featherpad", "lximage-qt", "qterminal"],
             "apt": ["lubuntu-desktop|task-lxqt-desktop"], "dnf": ["@lxqt-desktop", "?@lxqt-apps"],
             "zypper": ["pattern:lxqt"]},
        ),
    ),
    Desktop(
        "cosmic", "COSMIC", ("cosmic",), ("COSMIC",), True, False, False,
        "#2a1f3d", "cosmic",
        {"de": "Neuer Rust-Desktop von System76", "en": "New Rust desktop by System76"},
        _pk(
            {"pacman": ["cosmic-session", "cosmic-files", "cosmic-terminal"], "apt": ["cosmic-session"],
             "dnf": ["cosmic-session|@cosmic-desktop"]},
            {"pacman": ["cosmic"], "apt": ["cosmic-session", "?cosmic-files", "?cosmic-term"],
             "dnf": ["@cosmic-desktop|cosmic-session"]},
            {"pacman": ["cosmic", "?cosmic-store", "?cosmic-player"], "apt": ["pop-desktop|cosmic-session"],
             "dnf": ["@cosmic-desktop|cosmic-session", "?@cosmic-desktop-apps"]},
        ),
    ),
    Desktop(
        "hyprland", "Hyprland", ("hyprland",), ("Hyprland",), True, False, True,
        "#123040", "hyprland",
        {"de": "Kachel-Fenster mit Animationen (Wayland)", "en": "Animated tiling compositor (Wayland)"},
        _pk(
            {"pacman": ["hyprland", "kitty"], "apt": ["hyprland", "kitty"], "dnf": ["hyprland", "kitty"],
             "zypper": ["hyprland", "kitty"]},
            {"pacman": ["hyprland", "kitty", "waybar", "wofi", "xdg-desktop-portal-hyprland", "polkit-kde-agent"],
             "apt": ["hyprland", "kitty", "waybar", "wofi"], "dnf": ["hyprland", "kitty", "waybar", "wofi"],
             "zypper": ["hyprland", "kitty", "waybar", "wofi"]},
            {"pacman": ["hyprland", "kitty", "waybar", "wofi", "xdg-desktop-portal-hyprland", "polkit-kde-agent",
                        "hyprpaper", "hyprlock", "hypridle", "dunst", "grim", "slurp"],
             "apt": ["hyprland", "kitty", "waybar", "wofi", "?hyprpaper", "?hyprlock", "dunst", "grim", "slurp"],
             "dnf": ["hyprland", "kitty", "waybar", "wofi", "?hyprpaper", "?hyprlock", "dunst", "grim", "slurp"],
             "zypper": ["hyprland", "kitty", "waybar", "wofi", "?hyprpaper", "?hyprlock", "dunst", "grim", "slurp"]},
        ),
    ),
    Desktop(
        "sway", "Sway", ("sway",), ("sway",), True, False, True,
        "#20302a", "sway",
        {"de": "i3-kompatibles Tiling für Wayland", "en": "i3-compatible tiling for Wayland"},
        _pk(
            {"pacman": ["sway", "foot"], "apt": ["sway", "foot"], "dnf": ["sway", "foot"], "zypper": ["sway", "foot"]},
            {"pacman": ["sway", "foot", "waybar", "wofi", "swaybg", "swayidle", "swaylock"],
             "apt": ["sway", "foot", "waybar", "wofi", "swaybg", "swayidle", "swaylock"],
             "dnf": ["sway", "foot", "waybar", "wofi", "swaybg", "swayidle", "swaylock"],
             "zypper": ["sway", "foot", "waybar", "wofi", "swaybg", "swayidle", "swaylock"]},
            {"pacman": ["sway", "foot", "waybar", "wofi", "swaybg", "swayidle", "swaylock", "mako", "grim", "slurp"],
             "apt": ["sway", "foot", "waybar", "wofi", "swaybg", "swayidle", "swaylock", "mako-notifier", "grim",
                     "slurp"],
             "dnf": ["@sway-desktop-environment|sway", "waybar", "wofi", "mako", "grim", "slurp"],
             "zypper": ["?pattern:sway", "sway", "waybar", "wofi", "mako", "grim", "slurp"]},
        ),
    ),
    Desktop(
        "i3", "i3", ("i3",), ("i3",), False, True, True,
        "#2a2a2a", "i3",
        {"de": "Klassischer Tiling-Fenstermanager (X11)", "en": "Classic tiling window manager (X11)"},
        _pk(
            {"pacman": [XORG, "i3-wm", "xterm"], "apt": ["i3-wm", "xterm"], "dnf": ["i3", "xterm"],
             "zypper": ["i3", "xterm"]},
            {"pacman": [XORG, "i3-wm", "i3status", "i3lock", "dmenu", "xss-lock", "alacritty"],
             "apt": ["i3"], "dnf": ["i3", "i3status", "i3lock", "dmenu", "alacritty"],
             "zypper": ["i3", "i3status", "i3lock", "dmenu", "alacritty"]},
            {"pacman": [XORG, "i3-wm", "i3status", "i3lock", "dmenu", "xss-lock", "alacritty", "rofi", "picom",
                        "feh", "dunst"],
             "apt": ["i3", "rofi", "picom", "feh", "dunst", "alacritty"],
             "dnf": ["i3", "i3status", "i3lock", "dmenu", "alacritty", "rofi", "picom", "feh", "dunst"],
             "zypper": ["i3", "i3status", "i3lock", "dmenu", "alacritty", "rofi", "picom", "feh", "dunst"]},
        ),
    ),
)

BY_ID = {d.id: d for d in DESKTOPS}


def get(de_id: str) -> Desktop:
    if de_id not in BY_ID:
        raise KeyError(de_id)
    return BY_ID[de_id]
