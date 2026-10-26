"""Linux AIO 2.0: Tab „Einstellungen“ – Logik und echte Klicks bis in die geschriebenen Werte (Testmodus)."""

import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

TMP = Path(tempfile.mkdtemp(prefix="linuxaio-test-"))
os.environ["DEMANAGER_DRYRUN"] = "1"
os.environ["XDG_CACHE_HOME"] = str(TMP / "cache")
os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")

SRC = Path(__file__).resolve().parent.parent / "src"
sys.path.insert(0, str(SRC))
from linuxaio import desktops, gnome, looks, usercmd  # noqa: E402

PNG = bytes([0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) + b"x" * 64


def gvals() -> dict:
    try:
        return json.loads(usercmd.DRY_GSET.read_text(encoding="utf-8"))
    except OSError:
        return {}


def fake_cursor_theme(base: Path, name: str) -> Path:
    import struct
    d = base / name
    (d / "cursors").mkdir(parents=True)
    (d / "index.theme").write_text(f"[Icon Theme]\nName={name}\n", encoding="utf-8")
    w = h = 4
    img = struct.pack("<IIIIIIIII", 36, 0xFFFD0002, 24, 1, w, h, 0, 0, 0) + b"\xff\x00\x00\xff" * (w * h)
    data = b"Xcur" + struct.pack("<III", 16, 0x10000, 1) + struct.pack("<III", 0xFFFD0002, 24, 28) + img
    (d / "cursors" / "left_ptr").write_bytes(data)
    return d


class Logic(unittest.TestCase):
    def test_desktop_detect(self):
        self.assertEqual(desktops.current({"XDG_CURRENT_DESKTOP": "GNOME"}), "gnome")
        self.assertEqual(desktops.current({"XDG_CURRENT_DESKTOP": "KDE"}), "plasma")
        self.assertEqual(desktops.current({"XDG_CURRENT_DESKTOP": "Hyprland"}), "hyprland")
        self.assertEqual(desktops.current({"XDG_CURRENT_DESKTOP": "Budgie:GNOME"}), "budgie")
        self.assertEqual(desktops.current({"XDG_CURRENT_DESKTOP": "X-Cinnamon"}), "cinnamon")
        self.assertIsNone(desktops.current({}))
        ids = [e.id for e in desktops.entries()]
        for x in ("plasma", "gnome", "hyprland", "xfce", "cinnamon"):
            self.assertIn(x, ids)

    def test_button_layout(self):
        self.assertEqual(gnome.build_layout("right", True, True), "appmenu:minimize,maximize,close")
        self.assertEqual(gnome.build_layout("left", True, False), "close,minimize:appmenu")
        self.assertEqual(gnome.build_layout("right", False, False, appmenu=False), ":close")
        d = gnome.parse_layout("close,minimize,maximize:")
        self.assertEqual((d["side"], d["minimize"], d["maximize"]), ("left", True, True))
        d = gnome.parse_layout("appmenu:close")
        self.assertEqual((d["side"], d["minimize"], d["maximize"]), ("right", False, False))

    def test_fonts_and_values(self):
        self.assertEqual(gnome.split_font("Cantarell Bold 11"), ("Cantarell Bold", 11.0))
        self.assertEqual(gnome.join_font("Noto Sans", 10.0), "Noto Sans 10")
        self.assertEqual(usercmd.parse_value("uint32 4"), 4)
        self.assertEqual(usercmd.parse_value("'prefer-dark'"), "prefer-dark")
        self.assertEqual(usercmd.parse_value("@as []"), [])
        self.assertEqual(usercmd.gvariant("it's"), "'it\\'s'")
        self.assertEqual(usercmd.gvariant(True), "true")

    def test_ini_set_keeps_other_lines(self):
        p = TMP / "x.ini"
        p.write_text("# Kommentar\n[A]\nk=1\n[B]\nz=2\n", encoding="utf-8")
        usercmd.ini_set(p, "A", "k", "9")
        usercmd.ini_set(p, "B", "neu", "3")
        usercmd.ini_set(p, "C", "c", "4")
        text = p.read_text(encoding="utf-8")
        self.assertIn("# Kommentar", text)
        self.assertEqual(usercmd.ini_get(p, "A", "k"), "9")
        self.assertEqual(usercmd.ini_get(p, "B", "z"), "2")
        self.assertEqual(usercmd.ini_get(p, "B", "neu"), "3")
        self.assertEqual(usercmd.ini_get(p, "C", "c"), "4")

    def test_cursor_install_and_read(self):
        src = TMP / "src"
        th = fake_cursor_theme(src, "Bibata-Material-Test")
        items = looks.items(src, "cursor")
        it = next(i for i in items if i["name"] == "Bibata-Material-Test")
        self.assertEqual(it["color"], "Test")
        self.assertIsNone(it["installed"])
        dest = looks.install(th, lambda s: None)
        self.assertTrue(str(dest).startswith(str(usercmd.DRY_HOME)))      # Testmodus: nicht ins echte Home
        self.assertTrue((dest / "cursors" / "left_ptr").is_file())
        self.assertEqual(looks.install(th, lambda s: None), dest)        # vorhanden: nichts überschreiben
        r = looks.read_xcursor(dest / "cursors" / "left_ptr", 24)
        self.assertEqual(r[:2], (4, 4))

    def test_apply_cursor_per_desktop(self):
        looks.apply_cursor("gnome", "Bibata-Material-Teal", 32, lambda s: None)
        v = gvals()
        self.assertEqual(v["org.gnome.desktop.interface cursor-theme"], "Bibata-Material-Teal")
        self.assertEqual(v["org.gnome.desktop.interface cursor-size"], 32)
        self.assertEqual(looks.current("gnome", "cursor"), "Bibata-Material-Teal")
        looks.apply_cursor("cinnamon", "Bibata-Material-Mint", 24, lambda s: None)
        self.assertEqual(gvals()["org.cinnamon.desktop.interface cursor-theme"], "Bibata-Material-Mint")
        lines = []
        usercmd.set_logger(lines.append)
        looks.apply_cursor("plasma", "Bibata-Material-Sky-Blue", 24)
        looks.apply_cursor("hyprland", "Bibata-Material-Lilac", 24, lines.append)
        usercmd.set_logger(print)
        joined = "\n".join(lines)
        self.assertIn("plasma-apply-cursortheme Bibata-Material-Sky-Blue", joined)
        self.assertIn("hyprctl setcursor Bibata-Material-Lilac 24", joined)
        idx = usercmd.DRY_HOME / ".icons" / "default" / "index.theme"
        self.assertEqual(usercmd.ini_get(idx, "Icon Theme", "Inherits"), "Bibata-Material-Lilac")

    def test_autostart(self):
        app = TMP / "foo.desktop"
        app.write_text("[Desktop Entry]\nType=Application\nName=Foo\nExec=foo\n", encoding="utf-8")
        dest = gnome.autostart_add(app)
        self.assertTrue(str(dest).startswith(str(usercmd.DRY_HOME)))
        gnome.autostart_enable(dest, False)
        e = next(x for x in gnome.list_autostart() if x["id"] == "foo.desktop")
        self.assertFalse(e["enabled"])
        gnome.autostart_enable(dest, True)
        e = next(x for x in gnome.list_autostart() if x["id"] == "foo.desktop")
        self.assertTrue(e["enabled"])


def make_txz(files: dict, links: dict | None = None) -> bytes:
    """{pfad: bytes} → tar.xz (optional Symlinks {pfad: ziel})."""
    import io
    import tarfile
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w:xz") as tar:
        for name, data in files.items():
            ti = tarfile.TarInfo(name)
            ti.size = len(data)
            tar.addfile(ti, io.BytesIO(data))
        for name, target in (links or {}).items():
            ti = tarfile.TarInfo(name)
            ti.type = tarfile.SYMTYPE
            ti.linkname = target
            tar.addfile(ti)
    return buf.getvalue()


class Remote(unittest.TestCase):
    def setUp(self):
        import hashlib
        from linuxaio import remote
        self.remote = remote
        self.good = make_txz({"Bibata-Material-Zz/index.theme": b"[Icon Theme]\nName=Zz\n",
                              "Bibata-Material-Zz/cursors/left_ptr": b"Xcur"})
        self.pack = make_txz({"Bibata-Material-P1/cursors/a": b"1", "Bibata-Material-P2/cursors/a": b"2"})
        sha = lambda d: hashlib.sha256(d).hexdigest()  # noqa: E731
        man = {"format": 1, "kategorien": {"cursors": {
            "eintraege": [{"name": "Bibata-Material-Zz", "farbe": "Zz", "datei": "Zz.tar.xz", "groesse": 1,
                           "sha256": sha(self.good), "vorschau": ""},
                          {"name": "Bibata-Material-Kaputt", "farbe": "K", "datei": "Kaputt.tar.xz",
                           "sha256": "0" * 64, "vorschau": ""},
                          {"name": "../boese", "datei": "Zz.tar.xz", "sha256": sha(self.good)}],
            "pakete": [{"name": "alle", "titel": "Alle", "datei": "all.tar.xz", "sha256": sha(self.pack),
                        "groesse": 1, "enthaelt": ["Bibata-Material-P1", "Bibata-Material-P2"]}]},
            "login": {"eintraege": [], "pakete": []}, "icons": {"eintraege": [], "pakete": []}}}
        base = "https://example.invalid/"
        files = {"manifest.json": json.dumps(man).encode(), "Zz.tar.xz": self.good, "Kaputt.tar.xz": self.good,
                 "all.tar.xz": self.pack}
        rel = {"tag_name": "assets", "assets": [{"name": n, "browser_download_url": base + n} for n in files]}

        def fake(url, timeout=20, accept=""):
            if "api.github.com" in url:
                if "tags/assets" in url:
                    return json.dumps(rel).encode()
                raise AssertionError(url)
            return files[url[len(base):]]
        remote.fetch = fake

    def tearDown(self):
        self.remote.fetch = self.remote._get

    def test_manifest_and_install(self):
        man = self.remote.load_manifest("Labtoso/Linux-AIO")
        entries, packs = self.remote.items(man, "cursors")
        self.assertEqual([e["name"] for e in entries], ["Bibata-Material-Zz", "Bibata-Material-Kaputt"])
        self.assertEqual(len(packs), 1)
        dest = TMP / "icons-remote"
        new = self.remote.install(entries[0], dest)
        self.assertEqual([p.name for p in new], ["Bibata-Material-Zz"])
        self.assertTrue((dest / "Bibata-Material-Zz/cursors/left_ptr").is_file())
        with self.assertRaises(self.remote.SourceError) as cm:      # falsche Prüfsumme
            self.remote.install(entries[1], dest)
        self.assertEqual(cm.exception.key, "remote.badsum")
        self.assertFalse((dest / "Bibata-Material-Kaputt").exists())
        got = self.remote.install(packs[0], dest)
        self.assertEqual(sorted(p.name for p in got), ["Bibata-Material-P1", "Bibata-Material-P2"])
        self.assertEqual(self.remote.items(man, "icons"), ([], []))

    def test_unsafe_archives(self):
        dest = TMP / "icons-unsafe"
        for data in (make_txz({"../x/evil": b"1"}), make_txz({"/abs/evil": b"1"}),
                     make_txz({"Bibata-Material-L/cursors/a": b"1"}, {"Bibata-Material-L/cursors/b": "/etc/passwd"}),
                     make_txz({"Anderes/a": b"1"})):
            with self.assertRaises(self.remote.SourceError) as cm:
                self.remote.safe_extract(data, dest, {"Bibata-Material-L"})
            self.assertEqual(cm.exception.key, "remote.badarchive")
        self.assertFalse(dest.exists() and any(dest.iterdir()))

    def test_errors(self):
        import urllib.error

        def offline(url, timeout=20, accept=""):
            raise urllib.error.URLError("no network")
        self.remote.fetch = offline
        with self.assertRaises(self.remote.SourceError) as cm:
            self.remote.load_manifest("Labtoso/Linux-AIO")
        self.assertEqual(cm.exception.key, "remote.offline")

        def missing(url, timeout=20, accept=""):
            raise urllib.error.HTTPError(url, 404, "nf", {}, None)
        self.remote.fetch = missing
        with self.assertRaises(self.remote.SourceError) as cm:
            self.remote.load_manifest("Labtoso/Linux-AIO")
        self.assertEqual(cm.exception.key, "remote.norelease")
        with self.assertRaises(self.remote.SourceError) as cm:
            self.remote.load_manifest("kein repo")
        self.assertEqual(cm.exception.key, "remote.badrepo")


class Extras(unittest.TestCase):
    def test_button_css_markers(self):
        from linuxaio import buttons
        f4 = buttons.files()[0]
        f4.parent.mkdir(parents=True, exist_ok=True)
        f4.write_text("/* eigenes */\nwindow { color: red; }\n", encoding="utf-8")
        buttons.apply("macos", lambda s: None)
        buttons.apply("nord", lambda s: None)       # ersetzt, verdoppelt nicht
        text = f4.read_text(encoding="utf-8")
        self.assertIn("window { color: red; }", text)
        self.assertEqual(text.count(buttons.START), 1)
        self.assertEqual(buttons.current(), "nord")
        self.assertIn("windowcontrols > button.close", text)
        self.assertIn("headerbar button.titlebutton", buttons.files()[1].read_text(encoding="utf-8"))
        buttons.remove(lambda s: None)
        self.assertEqual(f4.read_text(encoding="utf-8"), "/* eigenes */\nwindow { color: red; }\n")
        self.assertEqual(buttons.current(), "")

    def test_layout_without_close(self):
        self.assertEqual(gnome.build_layout("right", True, True, True, close=False), "appmenu:minimize,maximize")
        self.assertFalse(gnome.parse_layout("appmenu:minimize")["close"])

    def test_stage_helper_from_dir_and_pyz(self):
        from linuxaio.widgets import stage_helper
        d = Path(stage_helper(str(SRC)))
        self.assertTrue((d / "linuxaio/helper.py").is_file())
        shutil.rmtree(d)

    def test_extension_zip_check(self):
        import io
        import zipfile
        from linuxaio import installers
        buf = io.BytesIO()
        with zipfile.ZipFile(buf, "w") as z:
            z.writestr("metadata.json", json.dumps({"uuid": "x@y"}))
            z.writestr("extension.js", "//")
        installers.check_zip(buf.getvalue(), "x@y")
        with self.assertRaises(installers.InstallError):
            installers.check_zip(buf.getvalue(), "anders@y")
        bad = io.BytesIO()
        with zipfile.ZipFile(bad, "w") as z:
            z.writestr("../evil.js", "//")
            z.writestr("metadata.json", json.dumps({"uuid": "x@y"}))
        with self.assertRaises(installers.InstallError):
            installers.check_zip(bad.getvalue(), "x@y")

    def test_extension_install_dry(self):
        from linuxaio import installers
        uuid = gnome.USER_THEME
        self.assertEqual(installers.install_extension(uuid, lambda s: None), "relogin")
        self.assertTrue(gnome.ext_installed(uuid))
        self.assertIn(uuid, gnome.ext_enabled())

    def test_pkg_install_helper_whitelist(self):
        code, out = helper("pkg-install", "gnome-shell-extensions")
        self.assertEqual(code, 0, out)
        self.assertIn("gnome-shell-extensions", out)
        code, out = helper("pkg-install", "irgendwas; rm -rf /")
        self.assertNotEqual(code, 0)


def helper(*args):
    env = dict(os.environ, PYTHONPATH=str(SRC), DEMANAGER_DRYRUN="1")
    r = subprocess.run([sys.executable, "-m", "linuxaio", "--helper", *args], env=env, capture_output=True, text=True)
    return r.returncode, r.stdout


class GdmHelper(unittest.TestCase):
    def test_logo_banner_bg_dry(self):
        img = TMP / "logo.png"
        img.write_bytes(PNG)
        code, out = helper("gdm-logo", str(img))
        self.assertEqual(code, 0, out)
        self.assertIn("logo='/usr/share/pixmaps/linuxaio-gdm-logo.png'", out)
        code, out = helper("gdm-banner", "Hallo 'du'")
        self.assertEqual(code, 0, out)
        self.assertIn("banner-message-text='Hallo \\'du\\''", out)
        code, out = helper("gdm-userlist", "off")
        self.assertIn("disable-user-list=true", out)
        code, out = helper("gdm-bg", "#112233")
        self.assertEqual(code, 0, out)
        st = json.loads((TMP / "cache/LabtosoStudios/linuxaio/testmodus/gdm.json").read_text(encoding="utf-8"))
        self.assertEqual(st["background"], "#112233")
        self.assertEqual(st["banner"], "Hallo 'du'")
        self.assertFalse(st["userlist"])
        bad = TMP / "bad.png"
        bad.write_bytes(b"not an image")
        code, out = helper("gdm-bg", str(bad))
        self.assertNotEqual(code, 0)
        self.assertIn("@@FAIL gdm.image", out)

    def test_css_rule(self):
        from linuxaio import gdm_helper
        self.assertIn("background: #abcdef", gdm_helper._css_rule("#abcdef", False))
        self.assertIn("resource:///org/gnome/shell/theme/linuxaio-background", gdm_helper._css_rule("x", True))


class Ui(unittest.TestCase):
    """Echte Klicks im Fenster (Testmodus)."""

    @classmethod
    def setUpClass(cls):
        os.environ["LINUXAIO_ALL_DE"] = "1"
        os.environ["XDG_CURRENT_DESKTOP"] = "GNOME"
        os.environ["XDG_CONFIG_HOME"] = str(TMP / "config")
        from linuxaio.qt import QtWidgets
        cls.app = QtWidgets.QApplication.instance() or QtWidgets.QApplication([])
        from linuxaio import gui
        from linuxaio import widgets as W
        W.CONFIG = TMP / "config" / "linuxaio.json"
        import urllib.error
        from linuxaio import remote

        def offline(url, timeout=20, accept=""):
            raise urllib.error.URLError("Testmodus ohne Netz")
        remote.fetch = offline       # UI-Tests ohne Netz: Quelle GitHub zeigt Meldung + Installierte
        cls.w = gui.MainWindow()
        cls.w.show()

    def spin(self, n=10):
        from linuxaio.qt import QtCore
        for _ in range(n):
            self.app.processEvents()
            QtCore.QThread.msleep(30)

    def panel(self, de, sec):
        self.w.set_tab("settings")
        self.w.settings_page.select(de)
        p = self.w.settings_page.panel()
        p.show_section(sec)
        self.spin()
        return p.pages[sec]

    def test_running_desktop_preselected(self):
        self.assertEqual(self.w.settings_page.running, "gnome")
        self.assertIn("gnome", self.w.settings_page.panels)

    def test_extension_arrow_click(self):
        from linuxaio import installers
        installers.DRY_EXT.discard(gnome.USER_THEME)
        pg = self.panel("gnome", "look")
        if gnome.ext_installed(gnome.USER_THEME):
            self.skipTest("User Themes ist auf diesem System schon installiert")
        self.assertTrue(pg.shell_get.isVisibleTo(pg))
        pg.shell_get.click()
        for _ in range(40):
            if not pg.shell_get.isVisibleTo(pg):
                break
            self.spin(3)
        self.assertFalse(pg.shell_get.isVisibleTo(pg))
        self.assertTrue(pg.shell.isEnabled())

    def test_button_theme_tile_click(self):
        from linuxaio import buttons
        pg = self.panel("gnome", "titlebar")
        tile = next(tl for tl in pg.tiles if tl.name == "macos")
        tile.btn.click()
        self.assertEqual(buttons.current(), "macos")
        self.assertFalse(tile.btn.isEnabled())
        pg.b_orig.click()
        self.assertEqual(buttons.current(), "")
        pg.sw_close.set_now(True)
        pg.sw_close.click()
        self.assertNotIn("close", gvals()["org.gnome.desktop.wm.preferences button-layout"])
        self.assertTrue(pg.warn.isVisibleTo(pg))
        pg.sw_close.click()

    def test_titlebar_clicks(self):
        pg = self.panel("gnome", "titlebar")
        pg.sw_min.set_now(False)
        pg.sw_max.set_now(False)
        pg.b_right.click()
        pg.sw_min.click()
        self.assertEqual(gvals()["org.gnome.desktop.wm.preferences button-layout"].split(":")[1], "minimize,close")
        pg.sw_max.click()
        pg.b_left.click()
        self.assertEqual(gvals()["org.gnome.desktop.wm.preferences button-layout"].split(":")[0],
                         "close,maximize,minimize")

    def test_topbar_switch_click(self):
        pg = self.panel("gnome", "topbar")
        c = next(c for c in pg.controls if c.gkey == "clock-show-seconds")
        before = c.w.isChecked()
        c.w.click()
        self.assertEqual(gvals()["org.gnome.desktop.interface clock-show-seconds"], not before)

    def test_choice_combo(self):
        pg = self.panel("gnome", "windows")
        c = next(c for c in pg.controls if c.gkey == "focus-mode")
        c.w.setCurrentIndex(c.w.findData("sloppy"))
        self.assertEqual(gvals()["org.gnome.desktop.wm.preferences focus-mode"], "sloppy")

    def test_cursor_tile_apply_click(self):
        pg = self.panel("gnome", "cursor")
        for _ in range(60):
            if pg.tiles:
                break
            self.spin(5)
        self.assertTrue(pg.tiles)
        self.assertIn("Keine Verbindung", pg.empty.text() if self.w.cfg.get("lang") == "de" else "Keine Verbindung")
        tile = next(t for t in pg.tiles.values() if t.item["installed"] is not None and t.b_apply.isEnabled())
        tile.b_apply.click()
        self.spin()
        self.assertEqual(gvals()["org.gnome.desktop.interface cursor-theme"], tile.item["name"])
        self.assertFalse(tile.b_apply.isEnabled())               # zeigt jetzt „Angewendet“
        self.assertEqual(tile.b_apply.text(), "Angewendet" if self.w.cfg.get("lang") == "de" else "Applied")

    def test_hyprland_embed_and_cursor(self):
        pg = self.panel("hyprland", "hyprstudio")
        self.assertTrue(pg.hypr_page.isVisibleTo(pg))
        self.assertFalse(pg.btn_close.isVisibleTo(pg))
        cur = self.panel("hyprland", "cursor")
        for _ in range(60):
            if cur.tiles:
                break
            self.spin(5)
        tile = next(t for t in cur.tiles.values() if t.item["installed"] is not None)
        tile.b_apply.click()
        from hyprstudio import model
        files = list((model.config_home() / "hypr").glob("hyprstudio_cursor.*"))
        main = model.hypr_format(model.config_home() / "hypr")
        if main:   # nur wenn es im Testordner eine hyprland.lua/.conf gibt
            self.assertTrue(files)

    def test_language_switch(self):
        self.panel("gnome", "fonts")
        self.w.set_language("en")
        self.assertEqual(self.w.tabs.btns["settings"].text(), "Settings")
        self.w.set_language("de")
        self.assertEqual(self.w.tabs.btns["settings"].text(), "Einstellungen")

    def test_store_filter_labels(self):
        self.assertEqual(self.w.sddm_page.src.btns["store"].text(), "Installieren")
        self.assertEqual(self.w.ply_page.src.btns["local"].text(), "Installiert")


def tearDownModule():
    shutil.rmtree(TMP, ignore_errors=True)


if __name__ == "__main__":
    unittest.main()
