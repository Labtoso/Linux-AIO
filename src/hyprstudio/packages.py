"""Optionale Programme selbst auswählen und installieren (nichts davon wird automatisch installiert). Ohne Qt."""

import glob
import shutil
import subprocess

from . import appearance, model

# key: (Gruppe, Prüf-Befehl oder Font-Name, {Paketmanager: [Pakete]})
PACKAGES = {
    "rofi": ("launcher", "rofi", {"pacman": ["rofi"], "apt": ["rofi"], "dnf": ["rofi-wayland"], "zypper": ["rofi"]}),
    "wofi": ("launcher", "wofi", {"pacman": ["wofi"], "apt": ["wofi"], "dnf": ["wofi"], "zypper": ["wofi"]}),
    "fuzzel": ("launcher", "fuzzel", {"pacman": ["fuzzel"], "apt": ["fuzzel"], "dnf": ["fuzzel"], "zypper": ["fuzzel"]}),
    "waybar": ("bar", "waybar", {"pacman": ["waybar"], "apt": ["waybar"], "dnf": ["waybar"], "zypper": ["waybar"]}),
    "nerdfont": ("bar", "font:JetBrainsMono Nerd Font", {"pacman": ["ttf-jetbrains-mono-nerd", "ttf-nerd-fonts-symbols-mono"]}),
    "kitty": ("tools", "kitty", {"pacman": ["kitty"], "apt": ["kitty"], "dnf": ["kitty"], "zypper": ["kitty"]}),
    "pavucontrol": ("bar", "pavucontrol", {"pacman": ["pavucontrol"], "apt": ["pavucontrol"], "dnf": ["pavucontrol"],
                                           "zypper": ["pavucontrol"]}),
    "screenshot": ("tools", "grim", {"pacman": ["grim", "slurp", "wl-clipboard"], "apt": ["grim", "slurp", "wl-clipboard"],
                                     "dnf": ["grim", "slurp", "wl-clipboard"], "zypper": ["grim", "slurp", "wl-clipboard"]}),
    "media": ("tools", "playerctl", {"pacman": ["playerctl", "brightnessctl"], "apt": ["playerctl", "brightnessctl"],
                                     "dnf": ["playerctl", "brightnessctl"], "zypper": ["playerctl", "brightnessctl"]}),
    "hyprpicker": ("tools", "hyprpicker", {"pacman": ["hyprpicker"], "dnf": ["hyprpicker"]}),
    "hyprlock": ("tools", "hyprlock", {"pacman": ["hyprlock"], "dnf": ["hyprlock"]}),
    "hyprcursor": ("tools", "hyprcursor-util", {"pacman": ["hyprcursor"], "dnf": ["hyprcursor"]}),
    "hyprpaper": ("tools", "hyprpaper", {"pacman": ["hyprpaper"], "dnf": ["hyprpaper"]}),
    "cliphist": ("tools", "cliphist", {"pacman": ["cliphist"]}),
    "hyprpolkitagent": ("tools", "file:/usr/lib/hyprpolkitagent/hyprpolkitagent|/usr/libexec/hyprpolkitagent"
                                 "|/usr/lib/systemd/user/hyprpolkitagent.service",
                        {"pacman": ["hyprpolkitagent"], "dnf": ["hyprpolkitagent"]}),
    "qtdark": ("tools", "file:" + "|".join(appearance.KDE_PLUGIN_GLOBS),
               {"pacman": ["plasma-integration"], "apt": ["plasma-integration"], "dnf": ["plasma-integration"],
                "zypper": ["plasma6-integration-plugin"]}),
    "fastfetch": ("tools", "fastfetch", {"pacman": ["fastfetch"], "apt": ["fastfetch"], "dnf": ["fastfetch"],
                                         "zypper": ["fastfetch"]}),
}
GROUPS = ("launcher", "bar", "tools")
DRY_INSTALLED: set = set()
DRY_REMOVED: set = set()


def manager() -> str | None:
    if model.DRY:
        return "pacman"
    for pm, exe in (("pacman", "pacman"), ("apt", "apt-get"), ("dnf", "dnf"), ("zypper", "zypper")):
        if shutil.which(exe):
            return pm
    return None


_FONTS = {"t": 0.0, "out": ""}  # fc-list ist bei vielen Schriften langsam → kurz zwischenspeichern


def font_list() -> str:
    import time
    if time.monotonic() - _FONTS["t"] > 60:
        try:
            _FONTS["out"] = subprocess.run(["fc-list", ":", "family"], capture_output=True, text=True,
                                           timeout=8).stdout.lower()
        except (OSError, subprocess.TimeoutExpired):
            _FONTS["out"] = ""
        _FONTS["t"] = time.monotonic()
    return _FONTS["out"]


def forget() -> None:
    """Nach Installieren/Entfernen: Zwischenspeicher verwerfen."""
    _FONTS["t"] = 0.0


def installed(key: str) -> bool:
    if key not in PACKAGES:  # z. B. L Launcher: gehört zu Hypr Studio
        return True
    check = PACKAGES[key][1]
    if model.DRY:  # Testmodus: etwas gemischt anzeigen, „Installiertes“ merken
        return (key in ("waybar", "rofi") or key in DRY_INSTALLED) and key not in DRY_REMOVED
    if check.startswith("file:"):  # z. B. Qt-Plugin ohne eigenen Befehl
        return any(glob.glob(g) for g in check[5:].split("|"))
    if check.startswith("font:"):
        return check[5:].lower() in font_list()
    return shutil.which(check) is not None


POLKIT_AGENTS = ("hyprpolkitagent", "polkit-kde-authentication-agent-1", "polkit-gnome-authentication-agent-1",
                 "polkit-mate-authentication-agent-1", "lxpolkit", "lxqt-policykit-agent", "xfce-polkit",
                 "polkit-agent-helper", "soteria")


def polkit_agent() -> bool:
    """Läuft ein Polkit-Agent (Passwortfenster für pkexec)? Unter Hyprland fehlt er oft → dann Terminal + sudo."""
    if model.DRY:
        return True
    try:
        out = subprocess.run(["ps", "-eo", "args"], capture_output=True, text=True, timeout=5).stdout
    except (OSError, subprocess.TimeoutExpired):
        return False
    return any(a in out for a in POLKIT_AGENTS)


def fail_reason(output: str) -> str | None:
    """Häufige Paketmanager-Fehler → i18n-Schlüssel mit verständlichem Hinweis."""
    o = output.lower()
    if "failed retrieving file" in o or "404" in o or "could not resolve" in o or "failed to synchronize" in o:
        return "pkg.err_sync"
    if "target not found" in o or "unable to locate package" in o or "no match for argument" in o:
        return "pkg.err_notfound"
    if "unable to lock database" in o or "could not get lock" in o:
        return "pkg.err_lock"
    if "not authorized" in o or "authentication" in o or "no authentication agent" in o:
        return "pkg.err_auth"
    return None


def available(key: str, pm: str | None) -> bool:
    return bool(pm and key in PACKAGES and PACKAGES[key][2].get(pm))


def pkg_names(keys: list, pm: str) -> list:
    pkgs = []
    for k in keys:
        pkgs += [p for p in PACKAGES[k][2].get(pm, []) if p not in pkgs]
    return pkgs


def _present(pkgs: list, pm: str) -> list:
    """Nur Pakete, die wirklich installiert sind (sonst bricht das Entfernen ab)."""
    if model.DRY:
        return pkgs
    q = {"pacman": ["pacman", "-Q"], "apt": ["dpkg", "-s"], "dnf": ["rpm", "-q"], "zypper": ["rpm", "-q"]}[pm]
    out = []
    for p in pkgs:
        try:
            if subprocess.run(q + [p], capture_output=True, timeout=8).returncode == 0:
                out.append(p)
        except (OSError, subprocess.TimeoutExpired):
            out.append(p)
    return out


def remove_command(keys: list, pm: str) -> list:
    """Entfernen per pkexec; pacman -Rs nimmt nicht mehr gebrauchte Abhängigkeiten mit."""
    pkgs = _present(pkg_names(keys, pm), pm)
    if not pkgs:
        return []
    return {
        "pacman": ["pacman", "-Rs", "--noconfirm", *pkgs],
        "apt": ["apt-get", "remove", "-y", *pkgs],
        "dnf": ["dnf", "remove", "-y", *pkgs],
        "zypper": ["zypper", "-n", "rm", *pkgs],
    }[pm]


# ---------- Updates (Sonstiges › Updates) ----------

UPDATE_GROUPS = {  # Gruppe → Pakete (nur installierte werden aktualisiert)
    "hyprland": ["hyprland", "hyprlock", "hypridle", "hyprpaper", "hyprcursor", "hyprpicker", "hyprpolkitagent",
                 "hyprsunset", "xdg-desktop-portal-hyprland", "hyprutils", "hyprlang", "hyprgraphics", "aquamarine"],
    "waybar": ["waybar"],
    "launcher": ["rofi", "rofi-wayland", "wofi", "fuzzel"],
}


def versions(pkgs: list, pm: str | None) -> dict:
    """Installierte Versionen {Paket: Version}; nicht installierte fehlen."""
    if model.DRY:
        demo = {"hyprland": "0.55.1-1", "hyprlock": "0.9.1-1", "hyprpaper": "0.7.5-1", "waybar": "0.13.0-1",
                "rofi": "1.7.9-1"}
        return {p: demo[p] for p in pkgs if p in demo}
    cmd = {"pacman": ["pacman", "-Q", *pkgs], "apt": ["dpkg-query", "-W", "-f=${Package} ${Version}\n", *pkgs],
           "dnf": ["rpm", "-q", "--qf", "%{NAME} %{VERSION}-%{RELEASE}\n", *pkgs],
           "zypper": ["rpm", "-q", "--qf", "%{NAME} %{VERSION}-%{RELEASE}\n", *pkgs]}.get(pm)
    if not cmd:
        return {}
    try:
        out = subprocess.run(cmd, capture_output=True, text=True, timeout=15).stdout
    except (OSError, subprocess.TimeoutExpired):
        return {}
    res = {}
    for ln in out.splitlines():
        parts = ln.split()
        if len(parts) == 2 and parts[0] in pkgs:
            res[parts[0]] = parts[1]
    return res


def newer(pm: str | None) -> dict | None:
    """Verfügbare neuere Versionen {Paket: neu} – Arch über checkupdates (pacman-contrib, ändert nichts am
    System); None = lässt sich nicht prüfen."""
    if model.DRY:
        return {"hyprland": "0.56.0-1", "waybar": "0.13.1-1"}
    if pm == "pacman" and shutil.which("checkupdates"):
        try:
            p = subprocess.run(["checkupdates"], capture_output=True, text=True, timeout=90)
        except (OSError, subprocess.TimeoutExpired):
            return None
        res = {}
        for ln in p.stdout.splitlines():  # „paket alt -> neu“
            parts = ln.split()
            if len(parts) >= 4 and parts[2] == "->":
                res[parts[0]] = parts[3]
        return res
    return None


def update_command(pkgs: list, pm: str | None, everything: bool = False) -> list:
    """Befehl für pkexec. Arch: immer mit -Syu (Teil-Updates sind dort nicht unterstützt)."""
    if not pm or (not everything and not pkgs):
        return []
    if pm == "pacman":
        return ["pacman", "-Syu", "--noconfirm"] + ([] if everything else ["--needed", *pkgs])
    if pm == "apt":
        up = "apt-get -y upgrade" if everything else "apt-get install -y --only-upgrade " + " ".join(pkgs)
        return ["sh", "-c", "apt-get update && " + up]
    if pm == "dnf":
        return ["dnf", "upgrade", "-y", *([] if everything else pkgs)]
    if pm == "zypper":
        return ["zypper", "-n", "up", *([] if everything else pkgs)]
    return []


def command(keys: list, pm: str) -> list:
    """Befehl für pkexec (läuft als root, ohne Rückfrage im Terminal)."""
    pkgs = pkg_names(keys, pm)
    if not pkgs:
        return []
    return {
        "pacman": ["pacman", "-S", "--needed", "--noconfirm", *pkgs],
        "apt": ["apt-get", "install", "-y", *pkgs],
        "dnf": ["dnf", "install", "-y", *pkgs],
        "zypper": ["zypper", "-n", "in", *pkgs],
    }[pm]
