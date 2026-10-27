"""yay from the AUR: clone yay-bin and build it as the user (makepkg must not run as root);
the root helper only installs the finished package. No Qt."""

import os
import subprocess
from pathlib import Path

from . import system, usercmd

AUR = "https://aur.archlinux.org/yay-bin.git"


def work_dir() -> Path:
    return usercmd.real_home() / ".cache/LabtosoStudios/linuxaio/yay"


def supported() -> bool:
    return system.detect_backend() == "pacman"


def installed() -> bool:
    return usercmd.have("yay")


def deps_missing() -> list:
    return [t for t in ("git", "makepkg", "fakeroot", "strip") if not usercmd.have(t)]


def _run(cmd: list, cwd: Path, log) -> None:
    log("$ " + " ".join(cmd))
    if os.environ.get("DEMANAGER_DRYRUN"):
        return
    proc = subprocess.Popen(cmd, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
                            stdin=subprocess.DEVNULL, env=dict(os.environ, LC_ALL="C"))
    for line in proc.stdout:
        log(line.rstrip())
    if proc.wait() != 0:
        raise RuntimeError(f"{cmd[0]} failed ({proc.returncode})")


def build(log=usercmd.log) -> Path:
    """Fetch (or update) yay-bin and build it; returns the package file."""
    base = work_dir()
    base.mkdir(parents=True, exist_ok=True)
    src = base / "yay-bin"
    if (src / ".git").is_dir():
        _run(["git", "pull", "--ff-only"], src, log)
    else:
        _run(["git", "clone", "--depth", "1", AUR, str(src)], base, log)
    for old in src.glob("yay-bin-*.pkg.tar.*"):
        old.unlink()
    _run(["makepkg", "-f", "--noconfirm"], src, log)
    if os.environ.get("DEMANAGER_DRYRUN"):
        return src / "yay-bin-dry.pkg.tar.zst"
    pkgs = sorted(p for p in src.glob("yay-bin-*.pkg.tar.*") if "-debug-" not in p.name)
    if not pkgs:
        raise RuntimeError("makepkg produced no package")
    return pkgs[-1]
