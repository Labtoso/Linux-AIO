"""Alles zurücksetzen: Hypr Studio aus Hyprland, Waybar und Launchern herausnehmen, optional die
offizielle Standard-Config (Hyprland 0.55+, example/hyprland.lua) schreiben. Nichts wird gelöscht:
jede Datei, die ersetzt oder weggenommen wird, bleibt als Sicherung liegen. Ohne Qt."""

import re
import shutil
from pathlib import Path

from . import fixer, kitty, model

# Offizielle Beispiel-Config von Hyprland (example/hyprland.lua), gekürzt um die Kommentar-Blöcke
DEFAULT_LUA = '''-- This is an example Hyprland Lua config file.
-- Refer to the wiki for more information.
-- https://wiki.hypr.land/configuring/

-- See https://wiki.hypr.land/configuring/core/monitors/
hl.monitor({
    output   = "",
    mode     = "preferred",
    position = "auto",
    scale    = "auto",
})

local terminal    = "kitty"
local fileManager = "dolphin"
local menu        = "hyprlauncher"

-- See https://wiki.hypr.land/configuring/core/environment-variables/
hl.env("XCURSOR_SIZE", "24")
hl.env("HYPRCURSOR_SIZE", "24")

hl.config({
    general = {
        gaps_in  = 5,
        gaps_out = 20,

        border_size = 2,

        col = {
            active_border   = { colors = {"rgba(33ccffee)", "rgba(00ff99ee)"}, angle = 45 },
            inactive_border = "rgba(595959aa)",
        },

        resize_on_border = false,

        allow_tearing = false,

        layout = "dwindle",
    },

    decoration = {
        rounding       = 10,
        rounding_power = 2,

        active_opacity   = 1.0,
        inactive_opacity = 1.0,

        shadow = {
            enabled      = true,
            range        = 4,
            render_power = 3,
            color        = 0xee1a1a1a,
        },

        blur = {
            enabled   = true,
            size      = 3,
            passes    = 1,
            vibrancy  = 0.1696,
        },
    },

    animations = {
        enabled = true,
    },
})

hl.curve("easeOutQuint",   { type = "bezier", points = { {0.23, 1},    {0.32, 1}    } })
hl.curve("easeInOutCubic", { type = "bezier", points = { {0.65, 0.05}, {0.36, 1}    } })
hl.curve("linear",         { type = "bezier", points = { {0, 0},       {1, 1}       } })
hl.curve("almostLinear",   { type = "bezier", points = { {0.5, 0.5},   {0.75, 1}    } })
hl.curve("quick",          { type = "bezier", points = { {0.15, 0},    {0.1, 1}     } })

hl.curve("easy",           { type = "spring", mass = 1, stiffness = 238.1191, damping = 24.21279333 })

hl.animation({ leaf = "global",        enabled = true,  speed = 10,   bezier = "default" })
hl.animation({ leaf = "border",        enabled = true,  speed = 5.39, bezier = "easeOutQuint" })
hl.animation({ leaf = "windows",       enabled = true,  speed = 4.79, spring = "easy" })
hl.animation({ leaf = "windowsIn",     enabled = true,  speed = 4.1,  spring = "easy",         style = "popin 87%" })
hl.animation({ leaf = "windowsOut",    enabled = true,  speed = 1.49, bezier = "linear",       style = "popin 87%" })
hl.animation({ leaf = "fadeIn",        enabled = true,  speed = 1.73, bezier = "almostLinear" })
hl.animation({ leaf = "fadeOut",       enabled = true,  speed = 1.46, bezier = "almostLinear" })
hl.animation({ leaf = "fade",          enabled = true,  speed = 3.03, bezier = "quick" })
hl.animation({ leaf = "layers",        enabled = true,  speed = 3.81, bezier = "easeOutQuint" })
hl.animation({ leaf = "layersIn",      enabled = true,  speed = 4,    bezier = "easeOutQuint", style = "fade" })
hl.animation({ leaf = "layersOut",     enabled = true,  speed = 1.5,  bezier = "linear",       style = "fade" })
hl.animation({ leaf = "fadeLayersIn",  enabled = true,  speed = 1.79, bezier = "almostLinear" })
hl.animation({ leaf = "fadeLayersOut", enabled = true,  speed = 1.39, bezier = "almostLinear" })
hl.animation({ leaf = "workspaces",    enabled = true,  speed = 1.94, bezier = "almostLinear", style = "fade" })
hl.animation({ leaf = "workspacesIn",  enabled = true,  speed = 1.21, bezier = "almostLinear", style = "fade" })
hl.animation({ leaf = "workspacesOut", enabled = true,  speed = 1.94, bezier = "almostLinear", style = "fade" })
hl.animation({ leaf = "zoomFactor",    enabled = true,  speed = 7,    bezier = "quick" })

hl.config({
    dwindle = {
        preserve_split = true,
    },
})

hl.config({
    master = {
        new_status = "master",
    },
})

hl.config({
    scrolling = {
        fullscreen_on_one_column = true,
    },
})

hl.config({
    misc = {
        force_default_wallpaper = -1,
        disable_hyprland_logo   = false,
    },
})

hl.config({
    input = {
        kb_layout  = "us",
        kb_variant = "",
        kb_model   = "",
        kb_options = "",
        kb_rules   = "",

        follow_mouse = 1,

        sensitivity = 0,

        touchpad = {
            natural_scroll = false,
        },
    },
})

hl.gesture({
    fingers = 3,
    direction = "horizontal",
    action = "workspace"
})

local mainMod = "SUPER"

hl.bind(mainMod .. " + Q", hl.dsp.exec_cmd(terminal))
hl.bind(mainMod .. " + C", hl.dsp.window.close())
hl.bind(mainMod .. " + M", hl.dsp.exec_cmd("command -v hyprshutdown >/dev/null 2>&1 && hyprshutdown || hyprctl dispatch 'hl.dsp.exit()'"))
hl.bind(mainMod .. " + E", hl.dsp.exec_cmd(fileManager))
hl.bind(mainMod .. " + V", hl.dsp.window.float({ action = "toggle" }))
hl.bind(mainMod .. " + R", hl.dsp.exec_cmd(menu))
hl.bind(mainMod .. " + P", hl.dsp.window.pseudo())
hl.bind(mainMod .. " + J", hl.dsp.layout("togglesplit"))

hl.bind(mainMod .. " + left",  hl.dsp.focus({ direction = "left" }))
hl.bind(mainMod .. " + right", hl.dsp.focus({ direction = "right" }))
hl.bind(mainMod .. " + up",    hl.dsp.focus({ direction = "up" }))
hl.bind(mainMod .. " + down",  hl.dsp.focus({ direction = "down" }))

for i = 1, 10 do
    local key = i % 10 -- 10 maps to key 0
    hl.bind(mainMod .. " + " .. key,             hl.dsp.focus({ workspace = i}))
    hl.bind(mainMod .. " + SHIFT + " .. key,     hl.dsp.window.move({ workspace = i }))
end

hl.bind(mainMod .. " + S",         hl.dsp.workspace.toggle_special("magic"))
hl.bind(mainMod .. " + SHIFT + S", hl.dsp.window.move({ workspace = "special:magic" }))

hl.bind(mainMod .. " + mouse_down", hl.dsp.focus({ workspace = "e+1" }))
hl.bind(mainMod .. " + mouse_up",   hl.dsp.focus({ workspace = "e-1" }))

hl.bind(mainMod .. " + mouse:272", hl.dsp.window.drag(),   { mouse = true })
hl.bind(mainMod .. " + mouse:273", hl.dsp.window.resize(), { mouse = true })

hl.bind("XF86AudioRaiseVolume", hl.dsp.exec_cmd("wpctl set-volume -l 1 @DEFAULT_AUDIO_SINK@ 5%+"), { locked = true, repeating = true })
hl.bind("XF86AudioLowerVolume", hl.dsp.exec_cmd("wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-"),      { locked = true, repeating = true })
hl.bind("XF86AudioMute",        hl.dsp.exec_cmd("wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle"),     { locked = true, repeating = true })
hl.bind("XF86AudioMicMute",     hl.dsp.exec_cmd("wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle"),   { locked = true, repeating = true })
hl.bind("XF86MonBrightnessUp",  hl.dsp.exec_cmd("brightnessctl -e4 -n2 set 5%+"),                  { locked = true, repeating = true })
hl.bind("XF86MonBrightnessDown",hl.dsp.exec_cmd("brightnessctl -e4 -n2 set 5%-"),                  { locked = true, repeating = true })

hl.bind("XF86AudioNext",  hl.dsp.exec_cmd("playerctl next"),       { locked = true })
hl.bind("XF86AudioPause", hl.dsp.exec_cmd("playerctl play-pause"), { locked = true })
hl.bind("XF86AudioPlay",  hl.dsp.exec_cmd("playerctl play-pause"), { locked = true })
hl.bind("XF86AudioPrev",  hl.dsp.exec_cmd("playerctl previous"),   { locked = true })

hl.window_rule({
    name  = "suppress-maximize-events",
    match = { class = ".*" },

    suppress_event = "maximize",
})

hl.window_rule({
    name  = "fix-xwayland-drags",
    match = {
        class      = "^$",
        title      = "^$",
        xwayland   = true,
        float      = true,
        fullscreen = false,
        pin        = false,
    },

    no_focus = true,
})

hl.window_rule({
    name  = "move-hyprland-run",
    match = { class = "hyprland-run" },

    move  = "20 monitor_h-120",
    float = true,
})
'''


def _stamp(f: Path) -> Path:
    return model.stamp(f)


def _backup(f: Path, log) -> Path:
    b = _stamp(f)
    shutil.copy2(f, b)
    log(f"Sicherung: {b}")
    return b


def remove_include(hdir: Path, log) -> int:
    """Nimmt die Hypr-Studio-Einbindung aus hyprland.lua/.conf (mit Sicherung)."""
    n = 0
    for name in ("hyprland.lua", "hyprland.conf"):
        f = hdir / name
        if not f.is_file():
            continue
        text = f.read_text(encoding="utf-8", errors="replace")
        keep = [ln for ln in text.splitlines(keepends=True)
                if not (ln.strip() in (f"-- {model.MARK}", f"# {model.MARK}", 'require("hyprstudio")',
                                       "source = ~/.config/hypr/hyprstudio.conf", 'require("hyprstudio_laptop")',
                                       "source = ~/.config/hypr/hyprstudio_laptop.conf",
                                       'require("hyprstudio_perf")', "source = ~/.config/hypr/hyprstudio_perf.conf",
                                       'require("hyprstudio_look")', "source = ~/.config/hypr/hyprstudio_look.conf",
                                       f"-- {model.MARK} – Hell/Dunkel", f"# {model.MARK} – Hell/Dunkel",
                                       f"-- {model.MARK} – Leistung", f"# {model.MARK} – Leistung",
                                       f"-- {model.MARK} – Laptop-Tasten", f"# {model.MARK} – Laptop-Tasten"))]
        if len(keep) != len(text.splitlines(keepends=True)):
            _backup(f, log)
            f.write_text("".join(keep), encoding="utf-8")
            log(f"Einbindung entfernt: {f}")
            n += 1
    return n


def uncomment(hdir: Path, log) -> int:
    """Hebt das Auskommentieren durch Hypr Studio auf (übernommene Kürzel, Fixer) – die Config ist wieder wie vorher."""
    n = 0
    for name in ("hyprland.lua", "hyprland.conf"):
        f = hdir / name
        if not f.is_file():
            continue
        text = f.read_text(encoding="utf-8", errors="replace")
        new = re.sub(r"^(?:-- |# )\[Hypr Studio(?: Fix)?\] ", "", text, flags=re.M)
        if new != text:
            _backup(f, log)
            f.write_text(new, encoding="utf-8")
            n += text.count("[Hypr Studio")
            log(f"Wieder aktiv: {n} Zeilen in {f}")
    return n


def restore_originals(cfg_home: Path, log) -> int:
    """Waybar und Launcher: Original zurück; gab es keins, wird die erzeugte Datei zur Sicherung umbenannt."""
    n = 0
    for rel in ("waybar/config.jsonc", "waybar/config.json", "waybar/config", "waybar/style.css",
                "rofi/config.rasi", "rofi/hyprstudio.rasi", "wofi/config", "wofi/style.css", "fuzzel/fuzzel.ini",
                "hypr/hyprlock.conf", "kdeglobals"):
        f = cfg_home / rel
        orig = f.with_name(f.name + ".hyprstudio-original")
        if orig.is_file():
            fixer.restore(orig, f, log)
            n += 1
        elif f.is_file() and model.MARK in f.read_text(encoding="utf-8", errors="replace")[:300]:
            f.rename(_stamp(f))  # nicht löschen, nur beiseitelegen → Programm nutzt wieder seine Vorgabe
            log(f"Beiseitegelegt: {f}")
            n += 1
    return n


def write_default(hdir: Path, log) -> bool:
    """Offizielle Standard-Config als hyprland.lua (vorhandene Datei wird gesichert)."""
    hdir.mkdir(parents=True, exist_ok=True)
    f = hdir / "hyprland.lua"
    if f.is_file():
        _backup(f, log)
    f.write_text(DEFAULT_LUA, encoding="utf-8")
    log(f"Standard-Config geschrieben: {f}")
    return True


def reset_all(cfg_home: Path, factory: bool, log) -> None:
    hdir = cfg_home / "hypr"
    remove_include(hdir, log)
    uncomment(hdir, log)
    restore_originals(cfg_home, log)
    if kitty.remove_include(cfg_home, log):
        model.run(["pkill", "-USR1", "-x", "kitty"], log, quiet=True)
    if factory:
        write_default(hdir, log)
    model.run(["hyprctl", "reload"], log)
    model.restart_waybar(log)
