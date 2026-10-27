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

    def test_icon_links(self):
        dest = TMP / "icons-links"
        good = make_txz({"Ico/index.theme": b"[Icon Theme]", "Ico/48/a.svg": b"<svg/>"},
                        {"Ico/48/b.svg": "a.svg", "Ico/32": "48"})
        new = self.remote.safe_extract(good, dest, {"Ico"}, allow_links=True)
        self.assertTrue((new[0] / "48/b.svg").is_symlink())
        self.assertEqual((new[0] / "32/a.svg").read_bytes(), b"<svg/>")
        for links in ({"Evil/x": "../../etc/passwd"}, {"Evil/x": "/etc/passwd"}, {"Evil/48": "../Other"}):
            bad = make_txz({"Evil/index.theme": b"x"}, links)
            with self.assertRaises(self.remote.SourceError):
                self.remote.safe_extract(bad, TMP / "icons-evil", {"Evil"}, allow_links=True)

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


class Fonts(unittest.TestCase):
    def setUp(self):
        from linuxaio import fonts, remote
        self.fonts, self.remote = fonts, remote
        ttf = b"\x00\x01\x00\x00" + b"x" * 40
        meta = {"familyMetadataList": [
            {"family": "Caveat", "category": "Handwriting", "popularity": 50, "isOpenSource": True},
            {"family": "Brand", "category": "Serif", "popularity": 1, "isBrandFont": True}]}
        lst = {"manifest": {"files": [{"filename": "OFL.txt", "contents": "SIL OFL"}],
                            "fileRefs": [{"filename": "static/Caveat-Regular.ttf", "url": "https://fonts.gstatic.com/s/a.ttf"},
                                         {"filename": "Caveat[wght].ttf", "url": "https://fonts.gstatic.com/s/v.ttf"},
                                         {"filename": "evil.ttf", "url": "https://evil.example/x.ttf"}]}}
        nerd = make_txz({"HackNerdFont-Regular.ttf": ttf, "LICENSE.md": b"MIT", "readme.md": b"x"},
                        {"link.ttf": "/etc/passwd"})
        rel = {"tag_name": "v3.5.1", "assets": [{"name": "Hack.tar.xz", "size": len(nerd),
                                                 "browser_download_url": "https://github.com/n/Hack.tar.xz"}]}

        def fake(url, timeout=20, accept=""):
            if url.startswith(fonts.GOOGLE_META):
                return b")]}'\n" + json.dumps(meta).encode()
            if "download/list" in url:
                return b")]}'\n" + json.dumps(lst).encode()
            if "api.github.com" in url:
                return json.dumps(rel).encode()
            if url.endswith("Hack.tar.xz"):
                return nerd
            if "gstatic" in url:
                return ttf
            raise AssertionError(url)
        remote.fetch = fake
        shutil.rmtree(fonts.cache_dir(), ignore_errors=True)

    def tearDown(self):
        self.remote.fetch = self.remote._get

    def test_catalog_and_install(self):
        items, err = self.fonts.catalog()
        self.assertEqual(err, "")
        self.assertEqual([i["family"] for i in items], ["Caveat", "Hack Nerd Font"])   # Markenschrift raus
        d = self.fonts.install(items[0], lambda s: None)
        self.assertTrue(str(d).startswith(str(usercmd.DRY_HOME)))
        self.assertEqual(sorted(x.name for x in d.iterdir()), ["Caveat[wght].ttf", "OFL.txt"])  # variabel bevorzugt
        n = self.fonts.install(items[1], lambda s: None)
        self.assertEqual(sorted(x.name for x in n.iterdir()), ["HackNerdFont-Regular.ttf", "LICENSE.md"])  # kein Link
        self.assertTrue(self.fonts.remove("Caveat", lambda s: None))
        self.assertIsNone(self.fonts.installed("Caveat"))
        self.assertFalse(self.fonts.remove("../../etc", lambda s: None))

    def test_bad_font_rejected(self):
        self.remote.fetch, old = (lambda url, timeout=20, accept="": b"<html>" if "gstatic" in url
                                  else old(url, timeout, accept)), self.remote.fetch
        with self.assertRaises(self.remote.SourceError) as cm:
            self.fonts.install_google("Caveat", lambda s: None)
        self.assertEqual(cm.exception.key, "fonts.bad")
        self.assertIsNone(self.fonts.installed("Caveat"))

    def test_offline(self):
        import urllib.error

        def offline(url, timeout=20, accept=""):
            raise urllib.error.URLError("kein Netz")
        self.remote.fetch = offline
        items, err = self.fonts.catalog()
        self.assertEqual((items, err), ([], "remote.offline"))


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

    def test_button_css_draws_on_icon(self):
        from linuxaio import buttons
        css = buttons.css("macos", 4)
        self.assertIn("windowcontrols > button.close > image { background-color: #ff5f57; }", css)
        self.assertIn("windowcontrols > button { background: none;", css)   # Knopf selbst durchsichtig
        self.assertIn("-gtk-icon-size", css)
        self.assertNotIn("-gtk-icon-size", buttons.css("macos", 3))         # GTK3 kennt das nicht
        self.assertNotIn("filter:", css + buttons.css("macos", 3))

    def test_corners_block(self):
        from linuxaio import buttons, corners
        f4, f3 = corners.files()
        f4.parent.mkdir(parents=True, exist_ok=True)
        f4.write_text("/* eigenes */\n", encoding="utf-8")
        buttons.apply("nord", lambda s: None)
        corners.apply(6, lambda s: None)
        corners.apply(30, lambda s: None)                     # auf 24 begrenzt, ersetzt den Block
        text = f4.read_text(encoding="utf-8")
        self.assertEqual(text.count(corners.START), 1)
        self.assertIn(":root { --window-radius: 24px; }", text)
        self.assertIn("decoration { border-radius: 24px; }", f3.read_text(encoding="utf-8"))
        self.assertEqual(corners.current(), 24)
        corners.remove(lambda s: None)
        self.assertIsNone(corners.current())
        self.assertEqual(buttons.current(), "nord")           # Knopf-Theme bleibt unberührt
        buttons.remove(lambda s: None)
        self.assertEqual(f4.read_text(encoding="utf-8"), "/* eigenes */\n")

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

    def test_corner_slider(self):
        from linuxaio import corners
        pg = self.panel("gnome", "windows")
        pg.slider.setValue(8)
        pg._apply_corners()
        self.assertEqual(corners.current(), 8)
        pg.b_orig.click()
        self.assertIsNone(corners.current())

    def test_reset_and_undo(self):
        from hyprstudio import dialogs
        from linuxaio import buttons
        old = dialogs.confirm
        dialogs.confirm = lambda *a, **k: True
        try:
            pg = self.panel("gnome", "titlebar")
            self.assertTrue(pg.b_reset.isVisibleTo(pg))
            next(tl for tl in pg.tiles if tl.name == "nord").btn.click()
            usercmd.gset("org.gnome.desktop.wm.preferences", "button-layout", "close:")
            pg.b_reset.click()
            self.assertNotIn("org.gnome.desktop.wm.preferences button-layout", gvals())   # zurück auf Standard
            self.assertEqual(buttons.current(), "")
            self.assertTrue(self.w.btn_undo.isVisibleTo(self.w))
            bak = list((usercmd.config_dir() / "LabtosoStudios/linuxaio/backups").iterdir())
            self.assertTrue(bak)
            self.w.btn_undo.click()
            self.assertEqual(gvals()["org.gnome.desktop.wm.preferences button-layout"], "close:")
            self.assertEqual(buttons.current(), "nord")
            buttons.remove(lambda s: None)
            fonts = self.panel("gnome", "fonts")
            self.assertTrue(fonts.b_reset.isVisibleTo(fonts))
            auto = self.panel("gnome", "autostart")
            self.assertTrue(auto.b_reset.isVisibleTo(auto))            # überall an derselben Stelle …
            self.assertFalse(auto.b_reset.isEnabled())                 # … aber nichts zum Zurücksetzen
        finally:
            dialogs.confirm = old

    def test_ui_scale_stable(self):
        app = self.app
        base_qss = self.w.styleSheet()
        base_pt = app.font().pointSizeF()
        for _ in range(10):
            for v in ("90", "80", "100"):
                self.w.set_ui_scale(v)
        self.assertEqual(self.w.styleSheet(), base_qss)              # wieder genau die Basis
        self.assertAlmostEqual(app.font().pointSizeF(), base_pt, places=2)
        self.w.set_ui_scale("80")
        self.assertAlmostEqual(app.font().pointSizeF(), base_pt * 0.8, places=2)
        self.assertIn("font-size: 8.0pt", self.w.styleSheet())
        self.assertNotIn("QT_SCALE_FACTOR", os.environ)               # kein Neustart-Faktor mehr
        self.w.set_ui_scale("100")

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


class AdvancedTests(unittest.TestCase):
    def test_alacritty_toml_parses(self):
        import tomllib
        from hyprstudio import kitty, palettes
        from linuxaio import alacritty
        for name in palettes.ORDER:
            s = alacritty.state(dict(kitty.preset(name), font='A "B"', fastfetch=True, blink=0.5))
            d = tomllib.loads(alacritty.toml(s))
            self.assertEqual(d["font"]["normal"]["family"], 'A "B"')
            self.assertEqual(len(d["colors"]["normal"]), 8)
            self.assertEqual(d["cursor"]["blink_interval"], 500)
            self.assertIn("fastfetch", d["terminal"]["shell"]["args"][1])

    def test_preset_01_matches_kitty(self):
        """Alacritty „01“ = Kitty „01“ 1:1 (same state, same colours, font, padding, opacity, cursor)."""
        import tomllib
        from hyprstudio import kitty
        from linuxaio import alacritty
        k = kitty.preset("01")
        d = tomllib.loads(alacritty.toml(alacritty.state(None)))
        self.assertEqual(d["window"]["opacity"], k["opacity"])
        self.assertEqual(d["window"]["padding"]["x"], k["padding"])
        self.assertEqual(d["font"]["normal"]["family"], k["font"])
        self.assertEqual(d["font"]["size"], k["font_size"])
        self.assertEqual(d["colors"]["primary"]["background"], kitty._c(k["bg"]))
        self.assertEqual(d["cursor"]["style"]["shape"], "Beam")

    def test_all_preset_sets_share_names_and_order(self):
        from hyprstudio import hyprlock, kitty, launcher, model, palettes, theme
        from linuxaio import buttons
        for d in (kitty.PRESETS, launcher.PRESETS, model.HYPR_PRESETS, model.WAYBAR_PRESETS, hyprlock.PRESETS,
                  buttons.THEMES, theme.THEMES):
            self.assertEqual(tuple(list(d)[:len(palettes.ORDER)]), palettes.ORDER)
        self.assertEqual(launcher.PRESETS["nord"]["accent"], palettes.PALETTES["nord"]["accent"] + "ff")
        self.assertEqual(model.HYPR_PRESETS["dracula"]["active1"], palettes.PALETTES["dracula"]["accent"] + "ff")

    def test_search_places(self):
        from linuxaio import search
        self.assertEqual(search.place("top.hotcorner"), ("gnome", "topbar"))
        self.assertIsNone(search.place("win.dbl.minimize"))
        self.assertIsNone(search.place("top.hotcorner.tip"))
        self.assertEqual(search.place("ala.opacity")[2], "alacritty")



class CursorRuleTests(unittest.TestCase):
    def test_no_wait_cursor_anywhere(self):
        """The full loading cursor must never appear (at most BusyCursor, always reset in finally)."""
        root = Path(__file__).resolve().parent.parent / "src"
        bad = [str(f.relative_to(root)) for f in root.rglob("*.py") if "WaitCursor" in f.read_text(encoding="utf-8")]
        self.assertEqual(bad, [])

    def test_desktop_entries_no_startup_notify(self):
        base = Path(__file__).resolve().parent.parent
        texts = [(base / "packaging/de.labtosostudios.linuxaio.desktop").read_text(encoding="utf-8"),
                 (base / "build.py").read_text(encoding="utf-8"),
                 (base / "src/linuxaio/integrate.py").read_text(encoding="utf-8")]
        for txt in texts:
            self.assertNotIn("StartupNotify=true", txt)
        self.assertIn("StartupNotify=false", texts[0])



class NoSystemChangeTests(unittest.TestCase):
    """Rule since 2.5.1: opening pages, hover, wheel and focus never change the system (only explicit clicks),
    and nothing sets GTK_THEME/QT overrides globally."""

    FORBIDDEN = ("set-environment", "--env=GTK_THEME", "--env=QT_", 'hl.env("GTK_THEME"', "env = GTK_THEME",
                 'f"GTK_THEME=', "GTK_THEME={", "QT_STYLE_OVERRIDE=")

    def test_static_no_global_theme_env(self):
        bad = []
        for f in SRC.rglob("*.py"):
            for n, line in enumerate(f.read_text(encoding="utf-8").splitlines(), 1):
                code = line.split("#", 1)[0]
                if any(x in code for x in self.FORBIDDEN) and "unset" not in code:
                    bad.append(f"{f.relative_to(SRC)}:{n}: {line.strip()}")
        self.assertEqual(bad, [])

    def test_hypr_look_env_has_no_gtk_theme(self):
        from hyprstudio import appearance
        for fmt in ("lua", "conf"):
            self.assertNotIn("GTK_THEME", appearance.env_content("dark", "kde", fmt))

    def test_gtk_css_only_allowed_rules(self):
        """Whitelist: button themes only style window controls, corners only set radii."""
        import re
        from linuxaio import buttons, corners
        css = "".join(buttons.css(n) for n in buttons.THEMES) + corners.css(12, 4) + corners.css(12, 3)
        for sel, body in re.findall(r"([^{}]+)\{([^}]*)\}", css):
            sel = sel.strip()
            if not sel or sel.startswith("/*") or sel.startswith("@"):
                continue
            props = {p.split(":", 1)[0].strip() for p in body.split(";") if ":" in p}
            if "windowcontrols" in sel or "titlebutton" in sel:
                continue
            self.assertTrue(props <= {"border-radius", "--window-radius", "border-top-left-radius",
                                      "border-top-right-radius"}, f"{sel}: {props}")

    def test_browsing_every_page_writes_nothing(self):
        from linuxaio.qt import QtCore, QtGui, QtWidgets
        app = QtWidgets.QApplication.instance() or QtWidgets.QApplication([])
        from hyprstudio import appearance, model
        from linuxaio import gui, usercmd as U
        from linuxaio import widgets as W
        W.CONFIG = TMP / "config" / "linuxaio-nowrite.json"
        writes = []
        saved = (U.gset, U.greset, U.run, model.run, appearance.apply, W.Runner.start)
        U.gset = lambda *a, **k: writes.append(("gset", a)) or True
        U.greset = lambda *a, **k: writes.append(("greset", a)) or True
        U.run = lambda cmd, *a, **k: writes.append(("run", tuple(cmd))) or (True, "")
        model.run = lambda cmd, *a, **k: writes.append(("hs-run", tuple(cmd))) or True
        appearance.apply = lambda *a, **k: writes.append(("appearance", a)) or True
        W.Runner.start = lambda self, args: writes.append(("root", tuple(args)))
        os.environ["LINUXAIO_ALL_DE"] = "1"
        try:
            w = gui.MainWindow()
            w.show()

            def pump(n=4):
                for _ in range(n):
                    app.processEvents()
                    QtCore.QThread.msleep(10)

            from hyprstudio.widgets import NoWheel
            no_hand = set()

            def poke(root):
                """Hover, wheel and focus on every widget of the visible page; clickable = pointing hand."""
                for wid in root.findChildren(QtWidgets.QWidget):
                    if not wid.isVisible():
                        continue
                    if NoWheel.clickable(wid) and wid.isEnabled() and \
                            wid.cursor().shape() != QtCore.Qt.CursorShape.PointingHandCursor and \
                            wid.cursor().shape() in (QtCore.Qt.CursorShape.ArrowCursor,):
                        no_hand.add(f"{type(wid).__name__} {wid.objectName()} {getattr(wid, 'text', lambda: '')()}"[:80])
                    pos = QtCore.QPointF(wid.rect().center())
                    app.sendEvent(wid, QtGui.QEnterEvent(pos, pos, pos))
                    app.sendEvent(wid, QtGui.QWheelEvent(pos, pos, QtCore.QPoint(0, 0), QtCore.QPoint(0, -120),
                                                         QtCore.Qt.MouseButton.NoButton,
                                                         QtCore.Qt.KeyboardModifier.NoModifier,
                                                         QtCore.Qt.ScrollPhase.NoScrollPhase, False))
                    app.sendEvent(wid, QtGui.QFocusEvent(QtCore.QEvent.Type.FocusIn))
                pump(2)

            for tab in ("de", "sddm", "ply", "settings"):
                w.set_tab(tab)
                pump()
                poke(w.stack.currentWidget())
            sp = w.settings_page
            for de in ("gnome", "plasma", "hyprland"):
                sp.select(de)
                panel = sp.panel()
                if panel is None:
                    continue
                for key in panel.keys:
                    panel.show_section(key)
                    pump()
                    pg = panel.pages[key]
                    poke(pg)
                    other = getattr(pg, "other_page", None)       # eingebettetes Hypr Studio: alle Bereiche
                    if other is not None:
                        for sec in other.SECTIONS:
                            other.show_section(sec)
                            pump()
                            poke(other)
                    for ed in (getattr(pg, "hypr_page", None), getattr(pg, "waybar_page", None),
                               getattr(pg, "launcher_page", None)):
                        if ed is not None:
                            for sec in ed.SECTIONS:
                                ed.show_section(sec)
                                pump()
                                poke(ed)
            for key in ("kitty", "alacritty", "yay"):
                poke(sp.select_adv(key))
            pump(10)
            w.hide()
        finally:
            U.gset, U.greset, U.run, model.run, appearance.apply, W.Runner.start = saved
            os.environ.pop("LINUXAIO_ALL_DE", None)
        self.maxDiff = None
        self.assertEqual([str(x)[:160] for x in writes], [])
        self.assertEqual(sorted(no_hand), [])



class PresetUpdateTests(unittest.TestCase):
    def test_preset_roundtrip_and_preview(self):
        from linuxaio import presetfile, usercmd as U
        U.gset("org.gnome.desktop.interface", "clock-format", "24h")
        cfg = {"theme": "nord", "alacritty_k": {"opacity": 0.7}, "lang": "de"}
        f = presetfile.export(cfg, TMP / "mine")
        self.assertEqual(f.suffix, ".linuxaio")
        data = presetfile.load(f)
        self.assertNotIn("lang", data["app"])                        # Sprache bleibt beim Import wie sie ist
        U.gset("org.gnome.desktop.interface", "clock-format", "12h")
        ch = presetfile.changes(data, {"theme": "01"})
        self.assertTrue(any(n == "clock-format" for _a, n, _o, _v in ch))
        presetfile.backup_current({"theme": "01"})
        cfg2 = {"theme": "01"}
        res = presetfile.apply(data, cfg2, log=lambda s: None)
        self.assertEqual(cfg2["theme"], "nord")
        self.assertEqual(U.gget("org.gnome.desktop.interface", "clock-format"), "24h")
        self.assertGreater(res["gsettings"], 0)

    def test_preset_rejects_foreign_files(self):
        from linuxaio import presetfile
        bad = TMP / "x.linuxaio"
        bad.write_text('{"format": "other"}', encoding="utf-8")
        with self.assertRaises(presetfile.BadFile):
            presetfile.load(bad)

    def test_update_picks_newest_release_not_assets(self):
        from linuxaio import updater

        def rel(tag, sums=True):
            assets = [{"name": f"Linux-AIO-{tag[1:]}-x86_64.AppImage", "browser_download_url": "u", "size": 1}]
            if sums:
                assets.append({"name": "SHA256SUMS", "browser_download_url": "s"})
            return {"tag_name": tag, "assets": assets, "body": "## Added\n- x\n## Install\n- y"}
        info = updater.latest([rel("v2.5.0"), {"tag_name": "assets", "assets": []}, rel("v9.0.1"),
                               rel("v9.9.9", sums=False), dict(rel("v10.0.0"), prerelease=True)])
        self.assertEqual(info["version"], "9.0.1")
        self.assertTrue(updater.newer(info))
        self.assertNotIn("Install", updater.short_notes(info["notes"]))
        self.assertFalse(updater.newer({"version": "0.1.0"}))

    def test_update_rejects_bad_checksum(self):
        import hashlib
        from linuxaio import remote, updater
        tgt = TMP / "bin" / "Linux-AIO.AppImage"
        saved = updater.target, updater.fetch, updater.urllib.request.urlopen
        payload = b"new appimage"

        class R:
            headers = {"Content-Length": str(len(payload))}

            def __init__(self):
                self.d = [payload, b""]

            def read(self, n):
                return self.d.pop(0)

            def __enter__(self):
                return self

            def __exit__(self, *a):
                return False
        updater.target = lambda: tgt
        updater.urllib.request.urlopen = lambda req, timeout=60: R()
        info = {"version": "9.0.0", "appimage_url": "https://example.invalid/u", "appimage_name": "Linux-AIO-9.0.0-x86_64.AppImage",
                "sums_url": "s"}
        try:
            updater.fetch = lambda url, timeout=20: ("0" * 64 + "  Linux-AIO-9.0.0-x86_64.AppImage\n").encode()
            with self.assertRaises(remote.SourceError):
                updater.install(info, log=lambda s: None)
            self.assertFalse(tgt.exists())
            good = hashlib.sha256(payload).hexdigest()
            updater.fetch = lambda url, timeout=20: (good + "  Linux-AIO-9.0.0-x86_64.AppImage\n").encode()
            updater.install(info, log=lambda s: None)
            self.assertEqual(tgt.read_bytes(), payload)
        finally:
            updater.target, updater.fetch, updater.urllib.request.urlopen = saved


class ResetConfirmTests(unittest.TestCase):
    def test_every_reset_asks_first(self):
        """Every reset button opens the confirmation (Cancel = nothing happens)."""
        from linuxaio.qt import QtWidgets
        app = QtWidgets.QApplication.instance() or QtWidgets.QApplication([])
        from hyprstudio import dialogs
        from linuxaio import gui
        from linuxaio import widgets as W
        W.CONFIG = TMP / "config" / "linuxaio-reset.json"
        asked = []
        saved = dialogs.confirm
        dialogs.confirm = lambda parent, title, text, *a, **k: asked.append(text) or False
        os.environ["LINUXAIO_ALL_DE"] = "1"
        try:
            w = gui.MainWindow()
            w.show()
            pages = [w.sddm_page, w.ply_page]
            sp = w.settings_page
            sp.select("gnome")
            p = sp.panel()
            for key in p.keys:
                p.show_section(key)
                pages.append(p.pages[key])
            for key in ("kitty", "alacritty"):
                pages.append(sp.select_adv(key))
            app.processEvents()
            buttons = []
            for pg in pages:
                buttons += [b for b in pg.findChildren(QtWidgets.QToolButton) if b.objectName() == "ResetBtn"
                            and b.toolTip() and not b.isHidden() and b.isEnabled()]
            self.assertGreater(len(buttons), 5)
            for b in buttons:
                n = len(asked)
                b.click()
                self.assertEqual(len(asked), n + 1, f"{b.toolTip()} in {type(b.window()).__name__} {getattr(b.parent(), "title_key", "")} {type(b.parent()).__name__} host={hasattr(b.parent(), "host")}")
            w.hide()
        finally:
            dialogs.confirm = saved
            os.environ.pop("LINUXAIO_ALL_DE", None)



class SafetyCleanupTests(unittest.TestCase):
    def test_cleanup_only_when_something_is_there(self):
        from linuxaio import safety, usercmd as U
        saved = U.query, U.run
        calls = []
        U.run = lambda cmd, *a, **k: calls.append(cmd) or (True, "")
        try:
            U.query = lambda cmd, *a, **k: "[Environment]\nGTK_THEME=\n\n[Context]\nunset-environment=GTK_THEME;\n" \
                if cmd[0] == "flatpak" else ""
            self.assertEqual(safety.cleanup(lambda s: None), [])          # schon aufgeräumt: keine Meldung
            U.query = lambda cmd, *a, **k: "[Environment]\nGTK_THEME=Adwaita:dark\n" if cmd[0] == "flatpak" else ""
            self.assertEqual(safety.cleanup(lambda s: None), ["Flatpak GTK_THEME"])
            f = U.config_dir() / safety.ENV_FILE
            f.parent.mkdir(parents=True, exist_ok=True)
            f.write_text("# Hypr Studio (LabtosoStudios)\nGTK_THEME=Adwaita:dark\n", encoding="utf-8")
            U.query = lambda cmd, *a, **k: "GTK_THEME=Adwaita:dark" if cmd[0] == "systemctl" else ""
            done = safety.cleanup(lambda s: None)
            self.assertFalse(f.exists())
            self.assertIn("systemd user environment", done)
        finally:
            U.query, U.run = saved


if __name__ == "__main__":
    unittest.main()
