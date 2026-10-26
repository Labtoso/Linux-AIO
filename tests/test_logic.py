import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SRC = Path(__file__).resolve().parent.parent / "src"
sys.path.insert(0, str(SRC))
from linuxaio import catalog, system  # noqa: E402

STATE = Path(tempfile.gettempdir()) / "linuxaio-dry-state.json"


def helper(*args):
    env = dict(os.environ, PYTHONPATH=str(SRC), DEMANAGER_DRYRUN="1", DEMANAGER_BACKEND="pacman")
    r = subprocess.run([sys.executable, "-m", "linuxaio", "--helper", *args], env=env,
                       capture_output=True, text=True)
    return r.returncode, r.stdout


class Catalog(unittest.TestCase):
    def test_three_variants_everywhere(self):
        for de in catalog.DESKTOPS:
            self.assertEqual(set(de.packages), set(catalog.VARIANTS), de.id)
            for v in catalog.VARIANTS:
                self.assertTrue(de.supports(v, "pacman"), f"{de.id}/{v}")
            self.assertTrue(de.sessions and de.detect and de.desc["de"] and de.desc["en"])

    def test_gnome_vanilla_minimal(self):
        g = catalog.get("gnome")
        self.assertIn("vanilla-gnome-desktop|gnome-core", g.specs("minimal", "apt"))
        self.assertNotIn("gnome-extra", g.specs("minimal", "pacman"))

    def test_resolve(self):
        have = {"b", "c"}
        pk, missing = system.resolve("pacman", ["a|b", "?x", "c", "z"], lambda _b, n: n in have)
        self.assertEqual(pk, ["b", "c"])
        self.assertEqual(missing, ["z"])


class Helper(unittest.TestCase):
    def setUp(self):
        if STATE.exists():
            STATE.unlink()

    def test_install_remove_default(self):
        code, out = helper("install", "gnome", "minimal")
        self.assertEqual(code, 0, out)
        self.assertIn("pacman -Syu --needed --noconfirm", out)
        st = json.loads(STATE.read_text())
        self.assertEqual(st["gnome"]["variant"], "minimal")
        code, out = helper("default", "gnome")
        self.assertEqual(code, 0, out)
        code, out = helper("remove", "gnome", "--running=plasma")
        self.assertEqual(code, 0, out)
        self.assertIn("pacman -Rnsu", out)
        self.assertNotIn("gnome", json.loads(STATE.read_text()))

    def test_refuse_running_and_bad_ids(self):
        self.assertEqual(helper("remove", "plasma", "--running=plasma")[0], 3)
        self.assertNotEqual(helper("install", "gnome; rm -rf /", "minimal")[0], 0)
        self.assertNotEqual(helper("install", "gnome", "evil")[0], 0)


class SddmSafety(unittest.TestCase):
    def _tar(self, members):
        import io, tarfile
        path = Path(tempfile.mkdtemp()) / "t.tar.gz"
        with tarfile.open(path, "w:gz") as tar:
            for name, data in members:
                ti = tarfile.TarInfo(name)
                ti.size = len(data)
                tar.addfile(ti, io.BytesIO(data))
        return path

    def test_rejects_path_traversal(self):
        from linuxaio import sddm_helper
        bad = self._tar([("../evil/metadata.desktop", b"x")])
        with self.assertRaises(ValueError):
            sddm_helper.extract(bad, Path(tempfile.mkdtemp()))

    def test_finds_theme_root_and_names(self):
        from linuxaio import sddm_helper
        arc = self._tar([("pkg/My Theme/metadata.desktop", b"[SddmGreeterTheme]\nName=My\nQtVersion=6\n"),
                         ("pkg/My Theme/Main.qml", b"import Qt5Compat.GraphicalEffects\n")])
        dest = Path(tempfile.mkdtemp())
        sddm_helper.extract(arc, dest)
        roots = sddm_helper.find_theme_roots(dest, arc.name)
        self.assertEqual([n for n, _ in roots], ["My-Theme"])
        self.assertEqual(sddm_helper.qml_deps(roots[0][1], True), ["qt6-5compat"])

    def test_dm_enable_whitelist(self):
        self.assertEqual(helper("dm-enable", "evil;reboot")[0], 2)
        code, out = helper("dm-enable", "sddm")
        self.assertEqual(code, 0, out)
        self.assertIn("systemctl enable --force sddm.service", out)

    def test_remove_only_own_themes(self):
        code, out = helper("sddm-remove", "breeze")
        self.assertEqual(code, 2)
        self.assertIn("sddm.foreign", out)


class PlymouthSetup(unittest.TestCase):
    def test_hook_after_systemd_before_encrypt(self):
        from linuxaio import plymouth
        txt = "MODULES=()\nHOOKS=(base systemd autodetect microcode modconf kms keyboard sd-vconsole block sd-encrypt filesystems fsck)\n"
        out = plymouth.add_hook(txt)
        self.assertIn("HOOKS=(base systemd plymouth autodetect", out)
        self.assertEqual(plymouth.add_hook(out), out)       # nur einmal

    def test_hook_udev_and_comment_untouched(self):
        from linuxaio import plymouth
        txt = "#HOOKS=(base udev)\nHOOKS=(base udev autodetect encrypt filesystems)\n"
        out = plymouth.add_hook(txt)
        self.assertTrue(out.startswith("#HOOKS=(base udev)\n"))
        self.assertIn("HOOKS=(base udev plymouth autodetect encrypt", out)

    def test_params(self):
        from linuxaio import plymouth
        self.assertEqual(plymouth.add_params("root=UUID=x rw quiet"), "root=UUID=x rw quiet splash")
        self.assertEqual(plymouth.add_params("quiet splash"), "quiet splash")

    def test_root_level_archive_named_after_plymouth_file(self):
        # wie dotLock: Dateien direkt im Archiv, kein eigener Ordner, Archivname mit Store-Nummer
        import io, tarfile
        from linuxaio import plymouth, plymouth_helper
        work = Path(tempfile.mkdtemp())
        arc = work / "1635081-dotLock.tar.gz"
        ply = b"[Plymouth Theme]\nName=dotLock\nModuleName=two-step\n[two-step]\nImageDir=/usr/share/plymouth/themes/dotLock\n"
        with tarfile.open(arc, "w:gz") as tar:
            for n, data in (("dotLock.plymouth", ply), ("animation-001.png", b"x")):
                ti = tarfile.TarInfo(n)
                ti.size = len(data)
                tar.addfile(ti, io.BytesIO(data))
        dest = Path(tempfile.mkdtemp())
        plymouth_helper.sddm_helper.extract(arc, dest)
        roots = plymouth_helper.find_roots(dest, arc.name)
        self.assertEqual([r[0] for r in roots], ["dotLock"])

    def test_fix_theme_paths_and_filename(self):
        from linuxaio import plymouth_helper
        d = Path(tempfile.mkdtemp()) / "Neon"
        d.mkdir()
        (d / "neon theme.plymouth").write_text(
            "[Plymouth Theme]\nModuleName=script\n[script]\nImageDir=/usr/share/plymouth/themes/neon-old\n"
            "ScriptFile=/usr/share/plymouth/themes/neon-old/neon.script\n", encoding="utf-8")
        (d / "neon.script").write_text("x", encoding="utf-8")
        plymouth_helper.fix_theme(d, "Neon", "neon theme.plymouth")
        text = (d / "Neon.plymouth").read_text(encoding="utf-8")
        self.assertIn("ImageDir=/usr/share/plymouth/themes/Neon", text)
        self.assertIn("ScriptFile=/usr/share/plymouth/themes/Neon/neon.script", text)

    def test_ply_actions_whitelisted(self):
        self.assertEqual(helper("ply-set", "../../etc")[0], 2)
        self.assertEqual(helper("ply-remove", "bgrt")[0], 2)


if __name__ == "__main__":
    unittest.main()
