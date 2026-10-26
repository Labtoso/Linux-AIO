"""Root-Helfer (läuft per pkexec). Nimmt nur DE-/Varianten-IDs an, nie Paketnamen.

Ausgabe-Protokoll für die GUI:
  @@STEP <text>   @@PROGRESS <0-100>   @@OK   @@FAIL <text>
"""

import configparser
import json
import os

import subprocess
import sys
import tempfile
from pathlib import Path

from . import catalog, system

DRY = os.environ.get("DEMANAGER_DRYRUN") == "1"
DM_LINK = Path("/etc/systemd/system/display-manager.service")
DEBIAN_DM = Path("/etc/X11/default-display-manager")


def say(tag: str, text: str = "") -> None:
    print(f"@@{tag} {text}".rstrip(), flush=True)


def state_path() -> Path:
    if DRY:
        return Path(tempfile.gettempdir()) / "linuxaio-dry-state.json"
    return system.STATE_FILE


def load_state() -> dict:
    try:
        return json.loads(state_path().read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return {}


def save_state(st: dict) -> None:
    p = state_path()
    p.parent.mkdir(parents=True, exist_ok=True)
    tmp = p.with_suffix(".tmp")
    tmp.write_text(json.dumps(st, indent=2, sort_keys=True), encoding="utf-8")
    os.replace(tmp, p)


def run(cmd: list, env: dict | None = None) -> int:
    print("$ " + " ".join(cmd), flush=True)
    if DRY:
        return 0
    e = dict(os.environ, **(env or {}))
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, env=e,
                            stdin=subprocess.DEVNULL, bufsize=1)
    for line in proc.stdout:
        print(line.rstrip(), flush=True)
    return proc.wait()


def check(backend: str, name: str) -> bool:
    return True if DRY else system.available(backend, name)


# ---------- Display-Manager unverändert lassen (SDDM bleibt bei KDE) ----------

def dm_snapshot() -> dict:
    snap = {"link": None, "debian": None}
    try:
        snap["link"] = os.readlink(DM_LINK)
    except OSError:
        pass
    try:
        snap["debian"] = DEBIAN_DM.read_text(encoding="utf-8")
    except OSError:
        pass
    return snap


def dm_restore(before: dict) -> None:
    if DRY:
        return
    after = dm_snapshot()
    if before["debian"] is not None and after["debian"] != before["debian"]:
        DEBIAN_DM.write_text(before["debian"], encoding="utf-8")
        print("Display-Manager-Auswahl wiederhergestellt", flush=True)
    if before["link"] and after["link"] != before["link"]:
        unit = Path(before["link"]).name
        run(["systemctl", "enable", "--force", unit])


# ---------- Aktionen ----------

def do_install(de: catalog.Desktop, variant: str, backend: str) -> int:
    say("STEP", "resolve")
    say("PROGRESS", "5")
    pkgs, missing = system.resolve(backend, de.specs(variant, backend), check)
    if missing:
        say("FAIL", "missing " + " ".join(missing))
        return 2
    print("Pakete: " + " ".join(pkgs), flush=True)
    before = set() if DRY else system.installed_packages(backend)
    dm_before = dm_snapshot()
    cmds = system.install_commands(backend, pkgs)
    env = {"DEBIAN_FRONTEND": "noninteractive"} if backend == "apt" else None
    for i, cmd in enumerate(cmds):
        say("STEP", "install")
        say("PROGRESS", str(10 + int(80 * i / len(cmds))))
        if run(cmd, env) != 0:
            dm_restore(dm_before)
            say("FAIL", "install")
            return 1
    dm_restore(dm_before)
    after = set(pkgs) if DRY else system.installed_packages(backend)
    st = load_state()
    prev = st.get(de.id, {})
    added = sorted((after - before) | set(prev.get("added", [])))
    requested = sorted(set(pkgs) | set(prev.get("requested", [])))
    st[de.id] = {"variant": variant, "backend": backend, "added": added, "requested": requested}
    save_state(st)
    say("PROGRESS", "100")
    say("OK")
    return 0


def do_remove(de: catalog.Desktop, backend: str, running: str = "") -> int:
    if running == de.id:
        say("FAIL", "running")
        return 3
    st = load_state()
    entry = st.get(de.id)
    if entry:
        others = set()
        for k, v in st.items():
            if k != de.id:
                others |= set(v.get("added", []))
        added = set(entry.get("added", [])) - others
        targets = sorted(added) if backend == "pacman" else sorted(set(entry.get("requested", [])) - others)
        allowed = added | set(entry.get("requested", []))
    else:
        # Nicht über Linux AIO installiert: nur die Hauptpakete der Standard-Variante, die nichts anderes braucht
        targets, _ = system.resolve(backend, de.specs("standard", backend), check)
        targets = [p for p in targets if p != catalog.XORG]
        allowed = None
    if not targets:
        say("FAIL", "nothing")
        return 4
    say("STEP", "check")
    say("PROGRESS", "10")
    if not DRY and allowed is not None:
        sim = system.simulate_remove(backend, targets)
        if sim is not None:
            extra = sorted(sim - allowed)
            if extra:
                say("FAIL", "foreign " + " ".join(extra[:15]))
                return 5
    say("STEP", "remove")
    say("PROGRESS", "30")
    if run(system.remove_command(backend, targets)) != 0:
        say("FAIL", "remove")
        return 1
    st.pop(de.id, None)
    save_state(st)
    say("PROGRESS", "100")
    say("OK")
    return 0


def invoking_user() -> tuple[str, int] | None:
    import pwd
    for var in ("PKEXEC_UID", "SUDO_UID"):
        uid = os.environ.get(var)
        if uid and uid.isdigit():
            try:
                return pwd.getpwuid(int(uid)).pw_name, int(uid)
            except KeyError:
                pass
    return None


def do_default(de: catalog.Desktop) -> int:
    sess = None if DRY else system.installed_session(de)
    if DRY:
        sess = Path(f"/usr/share/wayland-sessions/{de.sessions[0]}.desktop")
        f = system._dry_dm_file().parent / "default-session"
        f.parent.mkdir(parents=True, exist_ok=True)
        f.write_text(sess.stem, encoding="utf-8")
    if not sess:
        say("FAIL", "notinstalled")
        return 2
    name = sess.stem
    is_x = sess.parent.name == "xsessions"
    say("STEP", "default")
    say("PROGRESS", "30")
    user = None if DRY else invoking_user()
    if user and not DRY:
        uname, uid = user
        ok = subprocess.run(["busctl", "call", "org.freedesktop.Accounts", f"/org/freedesktop/Accounts/User{uid}",
                             "org.freedesktop.Accounts.User", "SetSession", "s", name],
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0
        if not ok:
            subprocess.run(["busctl", "call", "org.freedesktop.Accounts", f"/org/freedesktop/Accounts/User{uid}",
                            "org.freedesktop.Accounts.User", "SetXSession", "s", name],
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        print(f"AccountsService: {uname} -> {name}", flush=True)
    if SDDM_present():
        write_ini(system.SDDM_STATE, "Last", "Session", str(sess))
        print(f"SDDM: Last.Session = {sess}", flush=True)
    say("PROGRESS", "100")
    say("OK")
    return 0


def SDDM_present() -> bool:
    return DRY or system.SDDM_STATE.parent.is_dir()


def write_ini(path: Path, section: str, key: str, value: str) -> None:
    if DRY:
        return
    cp = configparser.ConfigParser(interpolation=None)
    cp.optionxform = str
    st = None
    if path.exists():
        st = path.stat()
        cp.read(path, encoding="utf-8")
    if not cp.has_section(section):
        cp.add_section(section)
    cp.set(section, key, value)
    with open(path, "w", encoding="utf-8") as f:
        cp.write(f, space_around_delimiters=False)
    if st:
        os.chown(path, st.st_uid, st.st_gid)


def main(argv: list) -> int:
    try:
        sys.stdout.reconfigure(encoding="utf-8")
    except (AttributeError, ValueError):
        pass
    if not DRY and getattr(os, "geteuid", lambda: 1)() != 0:
        say("FAIL", "root")
        return 10
    if argv and argv[0] == "pkg-install":
        # nur feste Schlüssel aus installers.PACKAGES, nie freie Paketnamen
        from . import installers
        backend = system.detect_backend()
        pkgs = installers.packages_for(argv[1] if len(argv) > 1 else "", backend or "")
        if not pkgs:
            say("FAIL", "args pkg")
            return 12
        say("STEP", "install")
        for cmd in system.install_commands(backend, pkgs):
            if run(cmd) != 0:
                say("FAIL", "missing")
                return 1
        say("PROGRESS", "100")
        say("OK")
        return 0
    if argv and argv[0].startswith("gdm-"):
        from . import gdm_helper
        try:
            return gdm_helper.main(argv)
        except (IndexError, KeyError) as e:
            say("FAIL", f"args {e}")
            return 12
    if argv and argv[0].startswith(("sddm-", "dm-", "ply-")):
        from . import plymouth_helper, sddm_helper
        try:
            return (plymouth_helper if argv[0].startswith("ply-") else sddm_helper).main(argv)
        except (IndexError, KeyError) as e:
            say("FAIL", f"args {e}")
            return 12
    backend = system.detect_backend()
    if not backend:
        say("FAIL", "nobackend")
        return 11
    try:
        action, de_id = argv[0], argv[1]
        de = catalog.get(de_id)
        if action == "install":
            variant = argv[2]
            if variant not in catalog.VARIANTS or not de.supports(variant, backend):
                raise KeyError(variant)
            return do_install(de, variant, backend)
        if action == "remove":
            running = next((a.split("=", 1)[1] for a in argv[2:] if a.startswith("--running=")), "")
            return do_remove(de, backend, running)
        if action == "default":
            return do_default(de)
        raise KeyError(action)
    except (IndexError, KeyError) as e:
        say("FAIL", f"args {e}")
        return 12


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
