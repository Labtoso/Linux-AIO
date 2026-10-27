"""Start check: remove dangerous leftovers of older versions (with backup) – global GTK_THEME/QT overrides in
environment.d, the systemd user session and Flatpak. They break libadwaita apps (GNOME Settings without spacing)
and can force apps light/dark. No Qt."""

import shutil
import time

from . import usercmd

ENV_FILE = "environment.d/60-hyprstudio-look.conf"
VARS = ("GTK_THEME", "QT_QPA_PLATFORMTHEME", "QT_STYLE_OVERRIDE")
MARKS = ("Hypr Studio", "LabtosoStudios", "Linux AIO")


def cleanup(log=usercmd.log) -> list:
    """Returns what was removed (empty = nothing found)."""
    done = []
    f = usercmd.config_dir() / ENV_FILE
    src = usercmd.readable(f)
    if src.is_file() and any(m in src.read_text(encoding="utf-8", errors="replace") for m in MARKS):
        bak = usercmd.config_dir() / "LabtosoStudios/backups" / (time.strftime("%Y-%m-%d_%H%M%S") + "-env")
        bak.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, bak / f.name)
        f.unlink(missing_ok=True)
        log(f"− {f} (backup: {bak})")
        done.append(str(f))
    env = usercmd.query(["systemctl", "--user", "show-environment"]) or ""
    if done and any(line.split("=", 1)[0] in VARS for line in env.splitlines()):
        usercmd.run(["systemctl", "--user", "unset-environment", *VARS])
        done.append("systemd user environment")
    over = usercmd.query(["flatpak", "override", "--user", "--show"]) or ""
    if any(line.startswith("GTK_THEME=") for line in over.splitlines()):
        usercmd.run(["flatpak", "override", "--user", "--unset-env=GTK_THEME"])
        done.append("Flatpak GTK_THEME")
    return done

