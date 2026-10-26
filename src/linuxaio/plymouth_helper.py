"""Root-Teil für Plymouth: einrichten (Paket, Hook, splash), Themes installieren/setzen/entfernen."""

import json
import os
import shutil
import stat
import tarfile
import tempfile
import zipfile
from pathlib import Path

from . import plymouth, sddm, sddm_helper, system

BOOT_ENTRY_DIRS = (Path("/boot/loader/entries"), Path("/efi/loader/entries"), Path("/boot/efi/loader/entries"))
LIMINE_FILES = (Path("/boot/limine.conf"), Path("/boot/limine/limine.conf"), Path("/efi/limine.conf"),
                Path("/boot/EFI/limine/limine.conf"), Path("/efi/EFI/limine/limine.conf"))

_say = sddm_helper._say
_dry = sddm_helper._dry


def _run(cmd: list, env=None) -> int:
    from . import helper as H
    return H.run(cmd, env)


def save_state(st: dict) -> None:
    p = plymouth.store_state_file()
    p.parent.mkdir(parents=True, exist_ok=True)
    tmp = p.with_suffix(".tmp")
    tmp.write_text(json.dumps(st, indent=2, sort_keys=True), encoding="utf-8")
    os.replace(tmp, p)
    try:
        os.chmod(p, 0o644)
    except OSError:
        pass


def edit_file(path: Path, new_text: str) -> None:
    """Datei ändern, vorher einmalig eine Sicherung *.desddm-bak anlegen."""
    old = path.read_text(encoding="utf-8")
    if old == new_text:
        return
    print(f"Ändere {path}", flush=True)
    if _dry():
        return
    bak = path.with_name(path.name + ".desddm-bak")
    if not bak.exists():
        shutil.copy2(path, bak)
    st = path.stat()
    path.write_text(new_text, encoding="utf-8")
    os.chmod(path, stat.S_IMODE(st.st_mode))


def regen_initramfs() -> int:
    _say("STEP", "ply.initramfs")
    if shutil.which("mkinitcpio"):
        return _run(["mkinitcpio", "-P"])
    if shutil.which("update-initramfs"):
        return _run(["update-initramfs", "-u", "-k", "all"])
    if shutil.which("dracut"):
        return _run(["dracut", "-f", "--regenerate-all"])
    if _dry():
        return _run(["mkinitcpio", "-P"])
    print("Kein Initramfs-Werkzeug gefunden", flush=True)
    return 0


# ---------- Einrichtung ----------

def setup_hook() -> None:
    f = plymouth.hooks_file()
    if not f:
        return
    edit_file(f, plymouth.add_hook(f.read_text(encoding="utf-8")))


def setup_cmdline() -> None:
    import re
    done = False
    # Fedora & Co.
    if shutil.which("grubby") and not Path("/etc/default/grub").is_file():
        _run(["grubby", "--update-kernel=ALL", "--args=quiet splash rhgb"])
        return
    grub = Path("/etc/default/grub")
    if grub.is_file():
        text = grub.read_text(encoding="utf-8")
        rx = re.compile(r'^(GRUB_CMDLINE_LINUX_DEFAULT=)(["\']?)(.*?)\2\s*$', re.M)
        if rx.search(text):
            text = rx.sub(lambda m: f'{m.group(1)}"{plymouth.add_params(m.group(3))}"', text, count=1)
        else:
            text += '\nGRUB_CMDLINE_LINUX_DEFAULT="quiet splash"\n'
        edit_file(grub, text)
        if shutil.which("update-grub"):
            _run(["update-grub"])
        elif shutil.which("grub-mkconfig") and Path("/boot/grub").is_dir():
            _run(["grub-mkconfig", "-o", "/boot/grub/grub.cfg"])
        elif shutil.which("grub2-mkconfig") and Path("/boot/grub2").is_dir():
            _run(["grub2-mkconfig", "-o", "/boot/grub2/grub.cfg"])
        done = True
    kc = Path("/etc/kernel/cmdline")              # Unified Kernel Images
    if kc.is_file():
        edit_file(kc, plymouth.add_params(kc.read_text(encoding="utf-8").strip()) + "\n")
        done = True
    for d in BOOT_ENTRY_DIRS:                     # systemd-boot
        if d.is_dir():
            for e in sorted(d.glob("*.conf")):
                lines = e.read_text(encoding="utf-8").splitlines()
                new = [("options " + plymouth.add_params(l.split(None, 1)[1] if len(l.split(None, 1)) > 1 else ""))
                       if l.strip().startswith("options") else l for l in lines]
                edit_file(e, "\n".join(new) + "\n")
                done = True
    for lf in LIMINE_FILES:                       # Limine
        if lf.is_file():
            lines = lf.read_text(encoding="utf-8").splitlines()
            new = []
            for l in lines:
                key = l.strip().split(":", 1)[0].lower()
                if key in ("cmdline", "kernel_cmdline") and ":" in l:
                    pre, val = l.split(":", 1)
                    l = f"{pre}: {plymouth.add_params(val.strip())}"
                new.append(l)
            edit_file(lf, "\n".join(new) + "\n")
            done = True
    if not done:
        print("Bootloader nicht erkannt: Kernel-Parameter 'quiet splash' bitte selbst ergänzen", flush=True)


def do_setup() -> int:
    from . import helper as H
    backend = system.detect_backend()
    if not backend:
        _say("FAIL", "nobackend")
        return 11
    pkgs = {"pacman": ["plymouth"], "apt": ["plymouth", "plymouth-themes"],
            "dnf": ["plymouth", "plymouth-system-theme"], "zypper": ["plymouth"]}[backend]
    _say("STEP", "ply.install")
    _say("PROGRESS", "5")
    env = {"DEBIAN_FRONTEND": "noninteractive"} if backend == "apt" else None
    before = H.dm_snapshot()
    for cmd in system.install_commands(backend, pkgs):
        if _run(cmd, env) != 0:
            H.dm_restore(before)
            _say("FAIL", "install")
            return 1
    H.dm_restore(before)
    _say("STEP", "ply.setup")
    _say("PROGRESS", "50")
    setup_hook()
    setup_cmdline()
    _say("PROGRESS", "70")
    if regen_initramfs() != 0:
        _say("FAIL", "ply.initramfs")
        return 1
    if _dry():
        f = plymouth._dry_base() / "installed"
        f.parent.mkdir(parents=True, exist_ok=True)
        f.write_text("1", encoding="utf-8")
        for n in ("spinner", "bgrt", "tribar"):
            d = plymouth.themes_dir() / n
            d.mkdir(parents=True, exist_ok=True)
            (d / f"{n}.plymouth").write_text(f"[Plymouth Theme]\nName={n.capitalize()}\nModuleName=two-step\n",
                                              encoding="utf-8")
        plymouth.defaults_file().write_text("[Daemon]\nTheme=bgrt\n", encoding="utf-8")
    _say("PROGRESS", "100")
    _say("OK")
    return 0


# ---------- Themes ----------

REAL_THEMES = "/usr/share/plymouth/themes"


def find_roots(base: Path, archive_name: str) -> list:
    """Theme-Ordner im Archiv. Der Name kommt aus der .plymouth-Datei, denn Plymouth lädt
    immer themes/<Name>/<Name>.plymouth (Archive ohne eigenen Ordner, z. B. dotLock)."""
    roots = []
    for tf in base.rglob("*.plymouth"):
        d = tf.parent
        if d not in roots and not any(r in d.parents for r in roots):
            roots.append(d)
    out = []
    for d in roots:
        files = sorted(d.glob("*.plymouth"))
        tf = next((f for f in files if f.stem == d.name), files[0])
        out.append((sddm.safe_name(tf.stem), d, tf.name))
    return out


def fix_theme(target: Path, name: str, plyfile: str = "") -> None:
    """<Name>.plymouth sicherstellen und eigene Pfade (ImageDir/ScriptFile) auf den echten Ordner zeigen lassen."""
    import re
    want = target / f"{name}.plymouth"
    src = target / plyfile if plyfile else plymouth.theme_file(target)
    if src and src.is_file() and src != want:
        if want.exists():
            want.unlink()
        src.rename(want)
    if not want.is_file():
        return
    text = want.read_text(encoding="utf-8", errors="replace")
    rx = re.compile(r"^(\s*(?:ImageDir|ScriptFile)\s*=\s*)" + re.escape(REAL_THEMES) + r"/+([^/\s]+)(.*)$", re.M)

    def repl(m):
        ref = m.group(2)
        if ref == name or (plymouth.themes_dir() / ref).is_dir() and not (target / Path(m.group(3).lstrip("/"))).exists():
            return m.group(0)      # zeigt schon richtig oder bewusst auf ein anderes Theme (z. B. bgrt → spinner)
        return f"{m.group(1)}{REAL_THEMES}/{name}{m.group(3)}"
    new = rx.sub(repl, text)
    if new != text:
        want.write_text(new, encoding="utf-8")
        print(f"Pfade in {want.name} auf {REAL_THEMES}/{name} korrigiert", flush=True)


def safe_preview(preview: str):
    """Vorschau nur, wenn sie dem Aufrufer gehört und ein echtes PNG/JPEG ist."""
    if not preview:
        return None
    pv = Path(preview)
    try:
        pst = os.lstat(pv)
    except OSError:
        return None
    if stat.S_ISREG(pst.st_mode) and pst.st_size < 20 * 1024 * 1024 and sddm_helper.user_owned(pst) \
            and sddm_helper.is_image(pv):
        return pv
    return None


def do_install(archive_arg: str, store_id: str = "", preview: str = "") -> int:
    archive = Path(archive_arg)
    _say("STEP", "sddm.unpack")
    _say("PROGRESS", "10")
    try:
        st = os.lstat(archive)
    except OSError:
        _say("FAIL", "sddm.archive")
        return 2
    if not stat.S_ISREG(st.st_mode) or st.st_size > sddm_helper.MAX_ARCHIVE or not sddm.archive_ok(archive.name)             or not sddm_helper.user_owned(st):
        _say("FAIL", "sddm.archive")
        return 2
    state = plymouth.load_store_state()
    dest_dir = plymouth.themes_dir()
    dest_dir.mkdir(parents=True, exist_ok=True)
    installed = []
    with tempfile.TemporaryDirectory(prefix="desddm-") as tmp:
        work = Path(tmp)
        try:
            sddm_helper.extract(archive, work)
        except (ValueError, tarfile.TarError, zipfile.BadZipFile, OSError) as e:
            print(f"Entpacken fehlgeschlagen: {e}", flush=True)
            _say("FAIL", "sddm.archive")
            return 3
        roots = find_roots(work, archive.name)
        if not roots:
            _say("FAIL", "ply.notheme")
            return 4
        _say("STEP", "sddm.copy")
        _say("PROGRESS", "50")
        pv = safe_preview(preview)
        for name, src, plyfile in roots:
            if not sddm.NAME_RE.match(name):
                continue
            target = dest_dir / name
            if target.exists():
                if name not in state:
                    print(f"{name}: existiert schon (nicht von hier installiert) – wird genutzt", flush=True)
                    installed.append(name)
                    continue
                shutil.rmtree(target)
            shutil.copytree(src, target, symlinks=False,
                            ignore=lambda d, names: [n for n in names if os.path.islink(os.path.join(d, n))])
            fix_theme(target, name, plyfile)
            # Store-Vorschau als preview.png ablegen (wie plymouth-kcm)
            if pv and not (target / "preview.png").exists():
                shutil.copyfile(pv, target / "preview.png")
            if not _dry():
                sddm_helper._fix_perms(target)
            state[name] = {"store": store_id, "archive": archive.name}
            installed.append(name)
            print(f"Installiert: {target}", flush=True)
    if not installed:
        _say("FAIL", "sddm.exists")
        return 5
    save_state(state)
    print("@@THEME " + installed[0], flush=True)
    _say("PROGRESS", "100")
    _say("OK")
    return 0


def repair(name: str) -> str:
    """Ältere Installationen reparieren: Ordnername muss zur .plymouth-Datei passen (z. B. 1635081-dotLock → dotLock)."""
    d = plymouth.themes_dir() / name
    if (d / f"{name}.plymouth").is_file():
        return name
    tf = plymouth.theme_file(d)
    if not tf:
        return name
    good = sddm.safe_name(tf.stem)
    target = plymouth.themes_dir() / good
    state = plymouth.load_store_state()
    if target.is_dir() and (target / f"{good}.plymouth").is_file():
        print(f"{name}: Ordnername passt nicht zur Datei {tf.name} – nehme vorhandenes Theme {good}", flush=True)
        return good
    if name in state and not _dry():
        d.rename(target)
    elif name in state:
        shutil.copytree(d, target)
        shutil.rmtree(d)
    else:
        print(f"{name}: Ordnername passt nicht zu {tf.name} (fremdes Theme, nicht geändert)", flush=True)
        return name
    fix_theme(target, good, tf.name)
    if name in state:
        state[good] = state.pop(name)
        save_state(state)
    print(f"Repariert: {name} → {good}", flush=True)
    return good


def do_set(name: str) -> int:
    d = plymouth.themes_dir() / name
    tf = plymouth.theme_file(d) if sddm.NAME_RE.match(name) and d.is_dir() else None
    if not tf:
        _say("FAIL", "ply.notheme")
        return 2
    name = repair(name)
    tf = plymouth.theme_file(plymouth.themes_dir() / name)
    print(f"@@THEME {name}", flush=True)
    _say("STEP", "ply.set")
    _say("PROGRESS", "20")
    conf = plymouth.conf_file()
    conf.parent.mkdir(parents=True, exist_ok=True)
    if not conf.exists():
        conf.write_text("[Daemon]\n", encoding="utf-8")
    sddm_helper.write_conf(conf, "Daemon", "Theme", name)
    try:
        os.chmod(conf, 0o644)
    except OSError:
        pass
    print(f"{conf}: [Daemon] Theme={name}", flush=True)
    if shutil.which("update-alternatives") and not _dry():      # Debian/Ubuntu wie plymouth-kcm
        alt = "/usr/share/plymouth/themes/default.plymouth"
        _run(["update-alternatives", "--install", alt, "default.plymouth", str(tf), "100"])
        _run(["update-alternatives", "--set", "default.plymouth", str(tf)])
    _say("PROGRESS", "40")
    if regen_initramfs() != 0:
        _say("FAIL", "ply.initramfs")
        return 1
    _say("PROGRESS", "100")
    _say("OK")
    return 0


def do_remove(name: str) -> int:
    state = plymouth.load_store_state()
    if not sddm.NAME_RE.match(name) or name not in state:
        _say("FAIL", "sddm.foreign")
        return 2
    if plymouth.current_theme() == name:
        _say("FAIL", "sddm.active")
        return 3
    _say("STEP", "sddm.remove")
    target = plymouth.themes_dir() / name
    if target.is_dir() and not target.is_symlink():
        shutil.rmtree(target)
        print(f"Entfernt: {target}", flush=True)
    state.pop(name, None)
    save_state(state)
    _say("PROGRESS", "100")
    _say("OK")
    return 0


def main(argv: list) -> int:
    action = argv[0]
    if action == "ply-setup":
        return do_setup()
    if action == "ply-install":
        opt = {a.split("=", 1)[0]: a.split("=", 1)[1] for a in argv[2:] if a.startswith("--") and "=" in a}
        store = opt.get("--store", "")
        return do_install(argv[1], store if store.isdigit() else "", opt.get("--preview", ""))
    if action == "ply-set":
        return do_set(argv[1])
    if action == "ply-remove":
        return do_remove(argv[1])
    raise KeyError(action)
