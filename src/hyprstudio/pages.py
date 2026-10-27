"""Seiten Hyprland und Waybar: Seitenleiste links, oben Live-Vorschau, darunter Presets bzw. Regler-Karten."""

import json

from . import launcher, model, packages, sysactions, theme
from .i18n import t
from .preview import DesktopPreview, render_pixmap
from .qt import QtCore, QtWidgets, Qt
from .keys import BindEditor
from .bar_editor import BarEditor
from .workspaces import WorkspaceEditor
from .userpresets import PREFIX, UserPresetsMixin, store
from .widgets import Card, FlowGrid, ModuleColumns, PresetCard, Switch, draw_icon, nav_groups, set_chip



class EditorPage(UserPresetsMixin, QtWidgets.QWidget):
    SECTIONS: tuple = ()
    NAV_GROUPS: tuple = ()  # Kategorien der Seitenleiste: (Schlüssel, [Bereiche])
    NO_PREVIEW: tuple = ()
    HAS_PLAY = True
    KIND = ""
    LOOK_KEYS: tuple = ()

    def __init__(self, win):
        super().__init__()
        self.win = win
        self.state = self.load_state()
        self.cards, self.switches, self.segs, self.presets = [], [], [], {}
        self._presets_dirty = True

        lay = QtWidgets.QHBoxLayout(self)
        lay.setContentsMargins(0, 0, 0, 0)
        lay.setSpacing(14)

        side = QtWidgets.QFrame()
        side.setObjectName("Side")
        side.setFixedWidth(190)
        sl = QtWidgets.QVBoxLayout(side)
        sl.setContentsMargins(10, 12, 10, 12)
        sl.setSpacing(4)
        self.nav = {}
        grp = QtWidgets.QButtonGroup(self)
        for key in self.SECTIONS:
            b = QtWidgets.QPushButton()
            b.setObjectName("Nav")
            b.setCheckable(True)
            b.setCursor(Qt.CursorShape.PointingHandCursor)
            b.clicked.connect(lambda _=False, k=key: self.show_section(k))
            grp.addButton(b)
            self.nav[key] = b
        self.nav_heads = nav_groups(sl, self.nav, self.NAV_GROUPS, f"{self.KIND}", win)
        sl.addStretch()
        self.info = QtWidgets.QLabel()
        self.info.setObjectName("Label")
        self.info.setWordWrap(True)
        sl.addWidget(self.info)
        lay.addWidget(side)

        right = QtWidgets.QVBoxLayout()
        right.setSpacing(14)
        top = QtWidgets.QHBoxLayout()
        top.setSpacing(14)
        self.preview = DesktopPreview()
        self.preview.setFixedHeight(290)
        top.addWidget(self.preview, 1)
        acts = QtWidgets.QBoxLayout(QtWidgets.QBoxLayout.Direction.TopToBottom)
        acts.setSpacing(8)
        self.acts = acts
        self.page_title = QtWidgets.QLabel()
        self.page_title.setObjectName("PageTitle")
        trow = QtWidgets.QHBoxLayout()               # Titel + Zurücksetzen (wie überall in Linux AIO)
        trow.setSpacing(6)
        trow.addWidget(self.page_title)
        from . import section_reset
        self.b_reset = section_reset.button(self.reset_section)
        trow.addWidget(self.b_reset, 0, Qt.AlignmentFlag.AlignVCenter)
        trow.addStretch()
        acts.addLayout(trow)
        self.applied = QtWidgets.QLabel()
        acts.addWidget(self.applied, 0, Qt.AlignmentFlag.AlignLeft)
        acts.addStretch()
        self.btn_play = QtWidgets.QPushButton()
        self.btn_play.setCursor(Qt.CursorShape.PointingHandCursor)
        self.btn_play.clicked.connect(self.preview.play)
        self.btn_undo = QtWidgets.QPushButton()
        self.btn_undo.setCursor(Qt.CursorShape.PointingHandCursor)
        self.btn_undo.clicked.connect(self.undo)
        self.btn_apply = QtWidgets.QPushButton()
        self.btn_apply.setObjectName("Primary")
        self.btn_apply.setCursor(Qt.CursorShape.PointingHandCursor)
        self.btn_apply.clicked.connect(self.apply)
        for b in (self.btn_play, self.btn_undo, self.btn_apply):
            b.setFixedSize(170, 40)
            acts.addWidget(b)
        top.addLayout(acts)
        self.top_box = QtWidgets.QWidget()
        top.setContentsMargins(0, 0, 0, 0)
        self.top_box.setLayout(top)
        right.addWidget(self.top_box)
        # Kopfzeile für eingehängte Bereiche (Hintergrundbild, Mauszeiger): Titel links, Knöpfe rechts
        self.ext_head = QtWidgets.QWidget()
        eh = QtWidgets.QHBoxLayout(self.ext_head)
        eh.setContentsMargins(0, 0, 0, 0)
        eh.setSpacing(8)
        self.ext_title = QtWidgets.QLabel()
        self.ext_title.setObjectName("PageTitle")
        self.ext_title.setMinimumHeight(40)
        eh.addWidget(self.ext_title)
        eh.addStretch()
        self.ext_btns = eh
        self.ext_head.hide()
        right.addWidget(self.ext_head)

        self.stack = QtWidgets.QStackedWidget()
        self.pages = {}
        for key in self.SECTIONS:
            sa = QtWidgets.QScrollArea()
            sa.setWidgetResizable(True)
            sa.setHorizontalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
            inner = QtWidgets.QWidget()
            inner.setObjectName("ScrollInner")
            il = QtWidgets.QVBoxLayout(inner)
            il.setContentsMargins(0, 0, 6, 0)
            il.setSpacing(14)
            self.build_section(key, il)
            il.addStretch()
            sa.setWidget(inner)
            self.stack.addWidget(sa)
            self.pages[key] = sa
        right.addWidget(self.stack, 1)
        lay.addLayout(right, 1)

        self.externals = {}
        self.refresh_controls()
        self.show_section(win.cfg.get(f"{self.KIND}_section", self.SECTIONS[0]))

    def add_external(self, key, widget):
        """Fremden Bereich (eigene Knöpfe, ohne Vorschau) in die Seitenleiste hängen, z. B. Hintergrundbild."""
        old = self.pages[key]
        self.stack.removeWidget(old)
        old.deleteLater()
        self.stack.addWidget(widget)
        self.pages[key] = widget
        self.externals[key] = widget
        for b in getattr(getattr(widget, "content", widget), "actions", []):
            self.ext_btns.addWidget(b)
            b.hide()

    def sync_external(self):
        """Kopfzeile passend zum offenen eingehängten Bereich; ohne installiertes Programm keine Knöpfe."""
        key = self.win.cfg.get(f"{self.KIND}_section")
        ext = self.externals.get(key)
        self.ext_head.setVisible(ext is not None)
        if ext is None:
            return
        self.ext_title.setText(t("sec." + key))
        ready = not hasattr(ext, "currentIndex") or ext.currentIndex() == 0
        for k, w in self.externals.items():
            for b in getattr(getattr(w, "content", w), "actions", []):
                b.setVisible(k == key and ready)

    def reset_section(self):
        """Nur den offenen Bereich auf Standard (Sicherung vorher, wirkt mit Anwenden)."""
        from . import section_reset
        key = self.win.cfg.get(f"{self.KIND}_section") or self.SECTIONS[0]
        pg = self.pages.get(key)
        if pg is not None:
            section_reset.reset(self, pg, self.sanitize(None), t("sec." + key), f"{self.KIND}-{key}")

    # ----- Aufbau-Helfer -----
    def card(self, layout, title_key) -> Card:
        c = Card(title_key, self)
        layout.addWidget(c)
        self.cards.append(c)
        return c

    def preset_grid(self, layout, names, prefix):
        self.build_user_presets(layout)
        grid = FlowGrid(212)
        for n in names:
            pc = PresetCard(n)
            pc.setProperty("tkey", prefix + n)
            pc.clicked.connect(self.use_preset)
            grid.add(pc)
            self.presets[n] = pc
        layout.addWidget(grid)

    # ----- Zustand -----
    def set(self, key, value):
        if self.state.get(key) == value:
            return
        self.state[key] = value
        if key in self.LOOK_KEYS:  # nur Aussehens-Änderungen lösen das Preset
            self.win.cfg[f"{self.KIND}_preset"] = ""
        self.changed()

    def changed(self):
        self.win.cfg[self.KIND] = self.state
        self.win.save_later()
        self.win.update_previews()
        self._presets_dirty = True
        self.refresh_presets_selection()
        self.update_applied()

    # ----- „Angewendet“-Anzeige -----
    def _sig(self) -> str:
        return json.dumps(self.state, sort_keys=True)

    def is_applied(self) -> bool:
        return self.win.cfg.get(f"{self.KIND}_applied_sig") == self._sig()

    def update_applied(self):
        ok = self.is_applied()
        set_chip(self.applied, ok, t("applied") if ok else t("not_applied"))
        self.refresh_presets_selection()

    def apply(self):
        if not self.do_apply():
            return
        self.win.cfg[f"{self.KIND}_applied_sig"] = self._sig()
        self.win.cfg[f"{self.KIND}_applied_preset"] = self.win.cfg.get(f"{self.KIND}_preset", "")
        self.win.save_later()
        name = self.win.cfg[f"{self.KIND}_applied_preset"]
        pre = self.presets.get(name)
        self.win.set_status(t("applied") + (f": {pre.name.text()}" if pre else ""))
        self.update_applied()

    def after_undo(self, ok: bool):
        self.win.set_status(t("undone") if ok else t("nothing"))
        if ok:
            self.win.cfg[f"{self.KIND}_applied_sig"] = ""
            self.win.cfg[f"{self.KIND}_applied_preset"] = ""
            self.update_applied()

    def refresh_controls(self):
        for c in self.cards:
            c.refresh(self.state)

    def show_section(self, key):
        if key not in self.pages:
            key = self.SECTIONS[0]
        self.nav[key].setChecked(True)
        self.stack.setCurrentWidget(self.pages[key])
        self.win.cfg[f"{self.KIND}_section"] = key
        self.win.save_later()
        ext = getattr(self, "externals", {}).get(key)
        self.top_box.setVisible(ext is None)
        if ext is not None and hasattr(ext, "refresh"):
            ext.refresh()
            content = getattr(ext, "content", None)
            if ext.currentIndex() == 0 and hasattr(content, "refresh"):
                content.refresh()
        if hasattr(self, "ext_head"):
            self.sync_external()
        if ext is not None:
            return
        # Bereiche ohne sichtbare Wirkung: Vorschau weg, Knöpfe in eine Zeile → mehr Platz für Einstellungen
        compact = key in self.NO_PREVIEW
        self.preview.setVisible(not compact)
        self.btn_play.setVisible(self.HAS_PLAY and not compact)
        self.acts.setDirection(QtWidgets.QBoxLayout.Direction.LeftToRight if compact
                               else QtWidgets.QBoxLayout.Direction.TopToBottom)
        if key == "presets":
            self.render_presets()

    def render_presets(self, force=False):
        if not (self._presets_dirty or force) or not self.presets:
            return
        self._presets_dirty = False
        for name, pc in self.presets.items():
            pc.set_pixmap(self.state_pixmap(self.user_state(name)) if name.startswith(PREFIX)
                          else self.preset_pixmap(name))
        self.refresh_presets_selection()

    def refresh_presets_selection(self):
        sel = self.win.cfg.get(f"{self.KIND}_preset", "")
        applied = self.win.cfg.get(f"{self.KIND}_applied_preset", "") if self.is_applied() else ""
        for name, pc in self.presets.items():
            pc.refresh(name == sel, self.win.accent(), name == applied)

    def preset_pixmap(self, name):
        hs, ws = self.preset_states(name)
        return render_pixmap(hs, ws, 196, 110, 9)

    def use_preset(self, name):
        self.state = self.user_state(name) if name.startswith(PREFIX) else self.preset_value(name)
        self.refresh_controls()
        self.changed()
        self.win.cfg[f"{self.KIND}_preset"] = name
        self.update_applied()

    def apply_theme(self, accent):
        for sw in self.switches:
            sw.accent = accent
            sw.update()
        self.btn_play.setIcon(draw_icon("play", theme.TEXT, 16))
        self.refresh_presets_selection()

    def retranslate(self):
        for key, b in self.nav.items():
            b.setText(t("sec." + key).replace("&", "&&"))
        for h in self.nav_heads:
            h.retranslate()
        for c in self.cards:
            c.retranslate()
        for s in self.segs:
            s.retranslate()
        for pc in self.presets.values():
            if pc.property("tkey"):
                pc.name.setText(t(pc.property("tkey")))
        self.retranslate_user_presets()
        self.page_title.setText(t("tab." + self.KIND))
        self.btn_play.setText(t("play"))
        self.btn_undo.setText(t("undo"))
        self.btn_undo.setToolTip(t("tip.undo"))
        self.btn_apply.setText(t("apply"))
        self.update_info()
        self.update_applied()

    # von Unterklassen
    def load_state(self) -> dict: ...
    def build_section(self, key, layout): ...
    def preset_states(self, name): ...
    def preset_value(self, name) -> dict: ...
    def do_apply(self) -> bool: ...
    def undo(self): ...
    def update_info(self): ...


class HyprPage(EditorPage):
    SECTIONS = ("presets", "windows", "workspaces", "colors", "effects", "anim", "input", "keys", "system",
                "wallpaper", "cursor", "lock")
    NAV_GROUPS = (("look", ["presets", "windows", "colors", "effects", "anim"]),
                  ("control", ["workspaces", "input", "keys", "system"]),
                  ("desktop", ["wallpaper", "cursor", "lock"]))
    NO_PREVIEW = ("workspaces", "input", "keys", "system")
    LOOK_KEYS = model.LOOK_KEYS
    KIND = "hypr"

    def load_state(self):
        return model.hypr_state(self.win.cfg.get("hypr"))

    def sanitize(self, d):
        return model.hypr_state(d)

    def state_pixmap(self, s):
        wb = self.win.waybar_page.state if hasattr(self.win, "waybar_page") else None
        return render_pixmap(s, wb, 196, 110, 9)

    def build_section(self, key, lay):
        if key == "presets":
            self.preset_grid(lay, model.HYPR_PRESETS, "p.")
        elif key == "windows":
            c = self.card(lay, "card.gaps")
            c.slider("gaps_in", 0, 40)
            c.slider("gaps_out", 0, 60)
            c.toggle("smart_gaps", tip="tip.smart_gaps")
            c = self.card(lay, "card.border")
            c.slider("border_size", 0, 8)
            c.slider("rounding", 0, 30)
            c.slider("rounding_power", 2, 10, 0.5, "{:.1f}", tip="tip.rounding_power")
            c.toggle("resize_on_border")
            c = self.card(lay, "card.layout")
            c.choice("layout", ("dwindle", "master"), "layout.")
            c = self.card(lay, "card.dwindle")
            c.toggle("dwindle_preserve")
            c.choice("dwindle_force", ("0", "1", "2"), "force.")
            c.toggle("dwindle_smart", tip="tip.dwindle_smart")
            c = self.card(lay, "card.master")
            c.choice("master_new", ("master", "slave", "inherit"), "mnew.")
            c.slider("master_mfact", 0.1, 0.9, 0.05, "{:.2f}")
            c.choice("master_orient", ("left", "right", "top", "bottom", "center"), "orient.")
        elif key == "colors":
            c = self.card(lay, "card.border_col")
            c.color("active1", label_key="active")
            c.toggle("gradient")
            c.color("active2")
            c.slider("angle", 0, 360, 5, "{:.0f}°")
            c.color("inactive")
            c.toggle("border_rotate", tip="tip.border_rotate")
            c.slider("border_rotate_speed", 10, 200, 5, "{:.0f}", tip="tip.border_rotate_speed")
            c = self.card(lay, "card.shadow")
            c.color("shadow_color")
            c = self.card(lay, "card.background")
            c.color("bg_color")
        elif key == "effects":
            c = self.card(lay, "card.opacity")
            c.slider("active_opacity", 0.3, 1.0, 0.01, "{:.2f}")
            c.slider("inactive_opacity", 0.3, 1.0, 0.01, "{:.2f}")
            c.toggle("dim_inactive")
            c.slider("dim_strength", 0.0, 1.0, 0.05, "{:.2f}")
            c = self.card(lay, "card.blur")
            c.toggle("blur")
            c.slider("blur_size", 1, 20)
            c.slider("blur_passes", 1, 6)
            c.slider("blur_vibrancy", 0.0, 1.0, 0.01, "{:.2f}")
            c.slider("blur_noise", 0.0, 0.2, 0.0025, "{:.4f}")
            c.slider("blur_contrast", 0.0, 2.0, 0.02, "{:.2f}")
            c.slider("blur_brightness", 0.0, 2.0, 0.02, "{:.2f}")
            c.toggle("blur_xray", tip="tip.blur_xray")
            c.toggle("blur_ignore_opacity")
            c = self.card(lay, "card.shadow")
            c.toggle("shadow")
            c.slider("shadow_range", 0, 40)
            c.slider("shadow_power", 1, 4)
            c.toggle("shadow_sharp")
        elif key == "anim":
            c = self.card(lay, "card.anim")
            c.toggle("anim")
            c.choice("anim_curve", tuple(model.CURVES), "curve.")
            c.slider("anim_tempo", 0.5, 2.0, 0.1, "{:.1f}×", tip="tip.anim_tempo")
            c.choice("win_style", tuple(model.WIN_STYLES), "win.")
            c.choice("ws_style", tuple(model.WS_STYLES), "ws.")
            c = self.card(lay, "card.anim_parts")
            for k in ("anim_windows", "anim_fade", "anim_ws", "anim_border", "anim_layers"):
                c.toggle(k)
        elif key == "input":
            c = self.card(lay, "card.kb")
            c.text("kb_layout", tip="tip.kb_layout", width=160)
            c.text("kb_variant", tip="tip.kb_variant", width=160)
            c.text("kb_options", tip="tip.kb_options", width=260)
            c.toggle("numlock")
            c.slider("repeat_rate", 10, 100, 1, "{:.0f}/s")
            c.slider("repeat_delay", 100, 1500, 50, "{:.0f} ms")
            c = self.card(lay, "card.mouse")
            c.slider("sensitivity", -1.0, 1.0, 0.05, "{:+.2f}")
            c.choice("accel", ("default", "flat", "adaptive"), "accel.")
            c.choice("follow_mouse", ("0", "1", "2"), "fm.")
            c.toggle("left_handed")
            c.toggle("mouse_natural", tip="tip.scroll")
            c = self.card(lay, "card.touchpad")
            c.toggle("natural_scroll")
            c.toggle("dwt")
            c.slider("scroll_factor", 0.1, 3.0, 0.1, "{:.1f}×")
            c.choice("gesture", ("0", "3", "4"), "gest.", tip="tip.gesture")
            c = self.card(lay, "card.cursor")
            c.slider("cursor_size", 16, 64, 2, "{:.0f}")
            c.text("cursor_theme", tip="tip.cursor_theme", width=220)
            c.slider("cursor_timeout", 0.0, 30.0, 1.0, "{:.0f} s", tip="tip.cursor_timeout")
            c.toggle("cursor_hide_keys")
        elif key == "system":
            c = self.card(lay, "card.autostart")
            c.toggle("autostart_waybar")
            c.lines("autostart", tip="tip.autostart")
            c = self.card(lay, "card.monitor")
            c.toggle("monitor_on", tip="tip.monitor_on")
            c.choice("monitor_mode", ("preferred", "highrr", "highres"), "mmode.")
            c.choice("monitor_scale", ("auto", "1", "1.25", "1.5", "2"), "scale.")
            c = self.card(lay, "card.misc")
            c.toggle("no_logo")
            c.toggle("focus_on_activate")
            c.toggle("swallow", tip="tip.swallow")
            c.text("swallow_regex", width=260)
            c.toggle("mmb_paste")
            c.toggle("dpms_mouse")
            c.toggle("dpms_key")
            c = self.card(lay, "card.skip")
            c.lines("skip_opts", tip="tip.skip_opts")
        elif key == "workspaces":
            c = self.card(lay, "card.workspaces")
            self.ws_toggle = c.toggle("workspaces_on", tip="tip.workspaces_on")
            self.wsed = WorkspaceEditor()
            self.wsed.changed.connect(self._ws_changed)
            c.layout().addWidget(self.wsed)
            c.rows.append((QtWidgets.QLabel(), lambda s: self.wsed.load(s["workspaces"])))
        elif key == "keys":
            c = self.card(lay, "card.keys")
            self.binds = BindEditor()
            self.binds.changed.connect(self._binds_changed)
            self.binds.fix_requested.connect(self._fix_conflicts)
            self.binds.import_requested.connect(self._import_binds)
            self.binds.user_store = lambda: store(self.win.cfg, "binds")
            self.binds.saved.connect(self.win.save_later)
            c.layout().addWidget(self.binds)
            c.rows.append((QtWidgets.QLabel(), lambda s: (self.binds.load(s["binds"]), self._mark())))

    def _ws_changed(self):
        self.state["workspaces"] = self.wsed.read()
        if self.state["workspaces"] and not self.state["workspaces_on"]:
            self.state["workspaces_on"] = True
            self.ws_toggle.set_now(True)  # nicht alles neu laden, sonst verliert das Eingabefeld den Fokus
        self.changed()

    def _binds_changed(self):
        self.state["binds"] = self.binds.read()
        self._mark()
        self.changed()

    def _conflicts(self) -> set:
        main = self.hdir() / "hyprland.lua"
        if not main.is_file():
            return set()
        text = main.read_text(encoding="utf-8", errors="replace")
        mine = {model.combo(b["mods"], b["key"]) for b in self.state["binds"] if model.bind_ok(b)}
        return {c for _, c in model.lua_bind_combos(text) if c in mine}

    def _mark(self):
        if hasattr(self, "binds"):
            self.binds.mark(self._conflicts())

    def _import_binds(self):
        self.win.log("")
        got = model.import_binds(self.hdir(), self.win.log)
        if not got:
            self.win.set_status(t("keys.imported", n=0))
            return
        self.binds.merge(got)
        # Originale sind jetzt auskommentiert → sofort anwenden, sonst fehlen die Kürzel bis zum nächsten Klick
        self.apply()
        self.win.set_status(t("keys.imported", n=len(got)))

    def _fix_conflicts(self):
        self.win.log("")
        n = model.comment_conflicts(self.hdir(), self.state["binds"], self.win.log)
        self.win.set_status(t("keys.fixed", n=n))
        self._mark()

    def retranslate(self):
        super().retranslate()
        if hasattr(self, "binds"):
            self.binds.retranslate()
        if hasattr(self, "wsed"):
            self.wsed.retranslate(self.state["workspaces"])
            self._mark()

    def set(self, key, value):
        if key == "autostart_waybar":  # ab jetzt steuert nur noch dieser Schalter den Waybar-Autostart
            self.state["waybar_managed"] = True
        super().set(key, value)
        if key in ("anim_curve", "anim_tempo", "win_style") and self.state["anim"]:
            self.preview.play()

    def preset_states(self, name):
        return model.preset_state(name, self.state), self.win.waybar_page.state if hasattr(self.win, "waybar_page") else None

    def preset_value(self, name):
        return model.preset_state(name, self.state)

    def hdir(self):
        return model.config_home() / "hypr"

    def do_apply(self):
        ok, info = model.apply_hypr(self.state, self.hdir(), self.win.log)
        self.win.report(ok, info)
        self.update_info()
        self._mark()
        return ok

    def undo(self):
        self.win.log("")
        self.after_undo(model.undo_hypr(self.hdir(), self.win.log))

    def update_info(self):
        fmt = model.hypr_format(self.hdir())
        self.info.setText("Hyprland · " + t(f"fmt.{fmt or 'none'}"))


class WaybarPage(EditorPage):
    SECTIONS = ("presets", "editor", "bar", "modules", "options", "style")
    NAV_GROUPS = (("look", ["presets", "bar", "style"]), ("content", ["editor", "modules", "options"]))
    NO_PREVIEW = ("editor",)
    HAS_PLAY = False
    LOOK_KEYS = model.WB_LOOK
    KIND = "waybar"

    def __init__(self, win):
        super().__init__(win)
        self.preview.zoom_bar = True
        # Ein/Aus-Schalter und Installieren direkt unter dem Titel
        row = QtWidgets.QHBoxLayout()
        row.setSpacing(8)
        self.on_switch = Switch()
        self.on_switch.set_now(self.state["enabled"])
        self.on_switch.toggled.connect(self.set_enabled)
        self.on_label = QtWidgets.QLabel()
        self.on_label.setObjectName("RowLabel")
        self.btn_install = QtWidgets.QPushButton()
        self.btn_install.setCursor(Qt.CursorShape.PointingHandCursor)
        self.btn_install.clicked.connect(lambda: self.win.other_page.install_keys(["waybar"]))
        row.addWidget(self.on_switch)
        row.addWidget(self.on_label)
        row.addSpacing(8)
        row.addWidget(self.btn_install)
        row.addStretch()
        self.acts.insertLayout(2, row)
        self.switches.append(self.on_switch)
        self.preview.setFixedHeight(190)

    def load_state(self):
        return model.waybar_state(self.win.cfg.get("waybar"))

    def sanitize(self, d):
        return model.waybar_state(d)

    def state_pixmap(self, s):
        return render_pixmap(self.win.hypr_page.state, s, 196, 110, 9, bar_scale=2.0)

    def build_section(self, key, lay):
        if key == "presets":
            self.preset_grid(lay, model.WAYBAR_PRESETS, "wp.")
        elif key == "bar":
            c = self.card(lay, "card.shape")
            c.choice("position", ("top", "bottom"), "pos.")
            c.choice("style", ("bar", "float", "islands"), "st.")
            c.choice("layer", ("top", "bottom"), "layer.", tip="tip.layer")
            c.toggle("exclusive", tip="tip.exclusive")
            c = self.card(lay, "card.size")
            c.slider("height", 20, 60)
            c.slider("margin", 0, 30)
            c.slider("radius", 0, 30)
            c.slider("spacing", 0, 20)
            c.slider("pad", 0, 24)
            c.slider("border_width", 0, 4)
        elif key == "editor":
            c = self.card(lay, "card.editor")
            self.bared = BarEditor()
            self.bared.setToolTip(t("tip.editor"))
            self.bared.changed.connect(self._editor_changed)
            c.layout().addWidget(self.bared)
            c.rows.append((QtWidgets.QLabel(), lambda s: self.bared.load(s)))
        elif key == "modules":
            c = self.card(lay, "sec.modules")
            self.mods = ModuleColumns()
            self.mods.changed.connect(self._mods_changed)
            c.layout().addWidget(self.mods)
            c.rows.append((QtWidgets.QLabel(), lambda s: self.mods.load(s)))
            c2 = self.card(lay, "card.extra")
            c2.text("launcher", width=260)
            c2.text("power_cmd", width=260)
            c2.text("vol_click", width=260)
        elif key == "options":
            c = self.card(lay, "card.ws")
            c.choice("ws_format", ("number", "dot"), "wsf.")
            c.choice("ws_look", ("pill", "box", "underline", "text"), "wsl.")
            c.slider("ws_persist", 0, 10, 1, "{:.0f}", tip="tip.ws_persist")
            c = self.card(lay, "card.clock")
            c.choice("clock_fmt", ("hm", "hms", "dhm", "full"), "cf.")
            c.toggle("clock24")
            c = self.card(lay, "card.modopts")
            c.slider("win_len", 10, 120)
            c.slider("mpris_len", 10, 80)
            c.choice("net_fmt", ("essid", "ip", "signal"), "net.")
            c.slider("vol_step", 1, 20, 1, "{:.0f} %")
            c.slider("bat_warn", 5, 60, 1, "{:.0f} %")
            c.slider("bat_crit", 2, 40, 1, "{:.0f} %")
            c.slider("interval", 1, 30, 1, "{:.0f} s")
            c.slider("tray_size", 12, 32)
            c.toggle("tooltips")
        elif key == "style":
            c = self.card(lay, "card.colors")
            c.color("bg")
            c.color("fg")
            c.color("accent")
            c.color("border")
            c.toggle("colorful", tip="tip.colorful")
            self.btn_from = QtWidgets.QPushButton()
            self.btn_from.setCursor(Qt.CursorShape.PointingHandCursor)
            self.btn_from.clicked.connect(self.from_hypr)
            c.layout().addWidget(self.btn_from, 0, Qt.AlignmentFlag.AlignLeft)
            c = self.card(lay, "card.font")
            c.font("font")
            c.slider("font_size", 9, 24)
            c.choice("font_weight", ("normal", "600", "bold"), "fw.")
            c.toggle("icons")

    def _editor_changed(self, lists):
        self.state.update(lists)
        self.win.cfg["waybar_preset"] = ""
        self.refresh_controls()
        self.changed()

    def set_enabled(self, on: bool):
        self.state["enabled"] = bool(on)
        self.win.log("")
        if on:
            model.restart_waybar(self.win.log)
            self.win.set_status(t("wb.on"))
        else:
            model.run(["pkill", "-x", "waybar"], self.win.log, quiet=True)
            hp = self.win.hypr_page
            if hp.state["autostart_waybar"]:
                hp.state["waybar_managed"] = True
                hp.state["autostart_waybar"] = False
                hp.refresh_controls()
                hp.changed()
                hp.apply()
            self.win.set_status(t("wb.off"))
        self.changed()

    def _mods_changed(self):
        new = self.mods.read()
        if any(new[k] != self.state[k] for k in new):
            self.state.update(new)
            self.win.cfg["waybar_preset"] = ""
            self.changed()

    def from_hypr(self):
        hs = self.win.hypr_page.state
        self.state["accent"] = model.norm(hs["active1"])[:7] + "ff"
        self.state["border"] = model.norm(hs["inactive"])
        self.win.cfg["waybar_preset"] = ""
        self.refresh_controls()
        self.changed()

    def preset_states(self, name):
        return self.win.hypr_page.state, model.waybar_preset(name, self.state)

    def preset_pixmap(self, name):
        hs, ws = self.preset_states(name)
        return render_pixmap(hs, ws, 196, 110, 9, bar_scale=2.0)

    def preset_value(self, name):
        return model.waybar_preset(name, self.state)

    def wdir(self):
        return model.config_home() / "waybar"

    def do_apply(self):
        if not model.which("waybar"):
            self.win.report(False, "nowaybar")
            return False
        ok, info = model.apply_waybar(self.state, self.wdir(), self.win.log, restart=self.state["enabled"])
        self.win.report(ok, info)
        return ok

    def undo(self):
        self.win.log("")
        self.after_undo(model.undo_waybar(self.wdir(), self.win.log))

    def retranslate(self):
        super().retranslate()
        if hasattr(self, "mods"):
            self.mods.retranslate()
        if hasattr(self, "btn_from"):
            self.btn_from.setText(t("from_hypr"))

    def update_info(self):
        ok = model.which("waybar") and not getattr(self.win, "_fake_missing", False)
        self.info.setText("Waybar · " + (t("pkg.installed") if ok else t("fmt.none")))
        if hasattr(self, "btn_install"):
            self.btn_install.setVisible(not ok)
            self.btn_install.setText(t("wb.install"))
            self.on_label.setText(t("wb.enabled"))
            self.on_switch.set_now(self.state["enabled"])


class LauncherPage(EditorPage):
    SECTIONS = ("presets", "app", "window", "colors", "text")
    NAV_GROUPS = (("look", ["presets", "window", "colors", "text"]), ("program", ["app"]))
    HAS_PLAY = False
    KIND = "launcher"
    LOOK_KEYS = launcher.LOOK

    def __init__(self, win):
        super().__init__(win)
        self.preview.ls = self.state

    def load_state(self):
        return launcher.state(self.win.cfg.get("launcher"))

    def sanitize(self, d):
        return launcher.state(d)

    def state_pixmap(self, s):
        return render_pixmap(self.win.hypr_page.state, self.win.waybar_page.state, 196, 110, 9, ls=s)

    def changed(self):
        self.preview.ls = self.state
        self.win.cfg.setdefault("launcher_apps", {})[self.state["app"]] = self.state  # eigene Werte pro Launcher
        super().changed()

    # ----- jeder Launcher mit eigenen Einstellungen -----
    def switch_app(self, app: str, push: bool):
        """Auf einen anderen Launcher wechseln: dessen eigene Einstellungen laden (beim ersten Mal den
        aktuellen Look übernehmen). push = Kürzel und Waybar sofort auf ihn umstellen und anwenden."""
        if app not in launcher.APPS:
            return
        store = self.win.cfg.setdefault("launcher_apps", {})
        store[self.state["app"]] = self.state
        if app != self.state["app"]:
            ns = launcher.state(store.get(app) or dict(self.state))
            ns["app"] = app
            self.state = ns
            self.win.cfg["launcher_preset"] = ""
            self.refresh_controls()
            self.changed()
        self.update_info()
        if hasattr(self.win, "launcher_gate"):
            self.win.launcher_gate.refresh()
        if push:
            if packages.installed(app):
                self.use_everywhere()
            else:  # erst nach der Installation umstellen, sonst öffnet das Kürzel nichts
                self._pending_push = app
                self.win.set_status(t("launcher.after_install", app=t("app." + app)))

    def use_preset(self, name):
        app = self.state["app"]  # Presets ändern nur den Look, nicht den Launcher
        super().use_preset(name)
        if self.state["app"] != app:
            self.state["app"] = app
            self.changed()

    def detect_app(self) -> str | None:
        """Welchen Launcher starten die Kürzel wirklich? (eigene zuerst, dann hyprland.lua/.conf)"""
        hp = self.win.hypr_page
        for b, _ in model.launcher_binds(hp.hdir(), hp.state["binds"]):
            app = launcher.app_of_cmd(model.bind_cmd(b))
            if app:
                return app
        return None

    def autodetect(self):
        app = self.detect_app()
        if app and app != self.state["app"]:
            self.switch_app(app, push=False)
            self.win.set_status(t("launcher.detected", app=t("app." + app)))

    def build_section(self, key, lay):
        if key == "presets":
            self.preset_grid(lay, launcher.PRESETS, "lp.")
        elif key == "app":
            c = self.card(lay, "card.app")
            c.choice("app", launcher.APPS, "app.")
            self.app_info = QtWidgets.QLabel()
            self.app_info.setObjectName("RowLabel")
            c.layout().addWidget(self.app_info)
            self.btn_inst_app = QtWidgets.QPushButton()
            self.btn_inst_app.setObjectName("Primary")
            self.btn_inst_app.setCursor(Qt.CursorShape.PointingHandCursor)
            self.btn_inst_app.clicked.connect(lambda: self.win.other_page.install_keys([self.state["app"]]))
            c.layout().addWidget(self.btn_inst_app, 0, Qt.AlignmentFlag.AlignLeft)
            brow = QtWidgets.QHBoxLayout()
            brow.setSpacing(10)
            self.bound_info = QtWidgets.QLabel()
            self.bound_info.setObjectName("RowLabel")
            self.bound_info.setWordWrap(True)
            self.bound_chip = QtWidgets.QLabel()
            brow.addWidget(self.bound_chip)
            brow.addWidget(self.bound_info, 1)
            c.layout().addLayout(brow)
            self.btn_use = QtWidgets.QPushButton()
            self.btn_use.setCursor(Qt.CursorShape.PointingHandCursor)
            self.btn_use.clicked.connect(self.use_everywhere)
            c.layout().addWidget(self.btn_use, 0, Qt.AlignmentFlag.AlignLeft)
            # Herunterfahren, Neustart … als Einträge für alle Launcher (sysactions.py)
            c = self.card(lay, "card.sys")
            self.sys_sw = Switch()
            self.sys_sw.set_now(self.win.cfg.get("sys_actions", True))
            self.sys_sw.toggled.connect(self.set_sys_actions)
            c._add("sys.on", self.sys_sw, None, "tip.sys.on", None, label_key="sys.on")
            self.switches.append(self.sys_sw)
            c = self.card(lay, "card.lmode")
            c.choice("mode", ("drun", "run"), "lmode.")
            c.toggle("name_search", tip="tip.name_search")
            c.toggle("toggle", tip="tip.ltoggle", label_key="ltoggle")
            c.choice("matching", ("fuzzy", "normal"), "match.")
            c.toggle("insensitive")
            c.text("terminal", width=200)
        elif key == "window":
            c = self.card(lay, "card.lsize")
            c.slider("width", 300, 1200, 10, "{:.0f} px")
            c.slider("lines", 3, 15)
            c.toggle("grid", tip="tip.grid")
            c.slider("columns", 1, 8, tip="tip.columns")
            c.slider("icon_size", 16, 64, 2, "{:.0f} px")
            c.slider("padding", 0, 40, label_key="pad")
            c.slider("spacing", 0, 16, label_key="lspacing")
            c = self.card(lay, "card.lpos")
            c.choice("position", ("center", "top", "bottom"), "lpos.")
            c.slider("offset", 0, 300, 5, "{:.0f} px")
            c = self.card(lay, "card.lshape")
            c.slider("border_width", 0, 6)
            c.slider("radius", 0, 30)
            c.slider("el_radius", 0, 20)
        elif key == "colors":
            c = self.card(lay, "card.colors")
            c.color("bg")
            c.color("input_bg")
            c.color("fg")
            c.color("accent")
            c.color("sel")
            c.color("sel_fg")
            c.color("border")
            self.btn_from = QtWidgets.QPushButton()
            self.btn_from.setCursor(Qt.CursorShape.PointingHandCursor)
            self.btn_from.clicked.connect(self.from_hypr)
            c.layout().addWidget(self.btn_from, 0, Qt.AlignmentFlag.AlignLeft)
        elif key == "text":
            c = self.card(lay, "card.font")
            c.font("font")
            c.slider("font_size", 8, 24)
            c = self.card(lay, "card.ltext")
            c.toggle("show_prompt")
            c.text("prompt", width=200)
            c.text("placeholder", width=260)
            c.toggle("icons", label_key="app_icons")
            c.text("icon_theme", width=220, tip="tip.icon_theme")
            c.toggle("show_scroll")

    def set(self, key, value):
        if key == "app":  # Launcher wechseln = eigene Einstellungen laden + System umstellen
            if value != self.state["app"]:
                self.switch_app(value, push=True)
            return
        super().set(key, value)
        if key == "toggle":  # steckt in den Hyprland-Kürzeln → dort neu schreiben
            model.LAUNCHER_TOGGLE = bool(value)
            self.win.hypr_page.changed()
        if key == "app":
            self.update_info()
            if hasattr(self.win, "launcher_gate"):
                self.win.launcher_gate.refresh()

    def from_hypr(self):
        hs = self.win.hypr_page.state
        a = model.norm(hs["active1"])[:7]
        self.state.update({"accent": a + "ff", "border": a + "ff", "sel": a + "33"})
        self.win.cfg["launcher_preset"] = ""
        self.refresh_controls()
        self.changed()

    def use_everywhere(self):
        """Launcher-Befehl in Waybar und in Kürzeln übernehmen, die einen Launcher starten."""
        cmd = launcher.COMMANDS[self.state["app"]]
        wb = self.win.waybar_page
        wb.state["launcher"] = cmd
        wb.refresh_controls()
        wb.changed()
        hp = self.win.hypr_page
        n = 0
        found = model.launcher_binds(hp.hdir(), hp.state["binds"])
        app = self.state["app"]
        for b, where in found:
            if model.bind_cmd(b) == cmd:
                continue
            if where == "own":
                b["action"], b["arg"] = "launcher", app
            else:  # Kürzel aus hyprland.lua übernehmen; das Original wird beim Anwenden auskommentiert
                hp.state["binds"].append(dict(b, action="launcher", arg=app))
            n += 1
        if not found:  # gar kein Launcher-Kürzel → Super + Leertaste anlegen
            hp.state["binds"].append({"mods": ["SUPER"], "key": "space", "action": "launcher", "arg": app, "flags": []})
            n += 1
        hp.refresh_controls()
        hp.changed()
        self.win.log("")
        self.apply()     # Launcher-Config (Suche) schreiben
        model.comment_conflicts(hp.hdir(), hp.state["binds"], self.win.log)  # Original-Kürzel aus
        hp.apply()       # Kürzel in Hyprland übernehmen
        wb.apply()
        self.update_info()
        self.win.set_status(t("launcher.used", cmd=cmd, n=n))

    def preset_pixmap(self, name):
        return render_pixmap(self.win.hypr_page.state, self.win.waybar_page.state, 196, 110, 9,
                             ls=launcher.preset(name, self.state))

    def preset_value(self, name):
        return launcher.preset(name, self.state)

    def set_sys_actions(self, on: bool):
        self.win.cfg["sys_actions"] = bool(on)
        self.win.save_later()
        self.win.log("")
        sysactions.apply(bool(on), self.win.log, packages.installed("hyprlock"))
        self.win.set_status(t("sys.done_on") if on else t("sys.done_off"))

    def do_apply(self):
        app = self.state["app"]
        if app != "llauncher" and not model.which(app):
            self.win.log(t("launcher.missing", app=app))
        ok, info = launcher.apply(self.state, model.config_home(), self.win.log,
                                  per_app=self.win.cfg.get("launcher_apps"))
        self.win.report(ok, info)
        hp = self.win.hypr_page
        if ok and any(b["action"] == "launcher" for b in hp.state["binds"]):
            hp.apply()  # Umschalten (zweites Drücken schließt) steckt im Kürzel
        return ok

    def undo(self):
        self.win.log("")
        self.after_undo(launcher.undo(self.state, model.config_home()))

    def retranslate(self):
        super().retranslate()
        if hasattr(self, "btn_use"):
            self.btn_use.setText(t("launcher.use"))
            self.btn_use.setToolTip(t("tip.launcher.use"))
        if hasattr(self, "btn_from"):
            self.btn_from.setText(t("from_hypr"))

    def update_info(self):
        app = self.state["app"]
        if getattr(self, "_pending_push", None) == app and packages.installed(app):
            self._pending_push = None
            self.use_everywhere()  # Installation fertig → Kürzel und Waybar jetzt umstellen
        ok = app == "llauncher" or model.which(app)
        self.info.setText(f"{t('app.' + app)} · " + (t("pkg.installed") if ok else t("fmt.none")))
        if hasattr(self, "btn_inst_app"):
            self.btn_inst_app.setVisible(not ok)
            self.btn_inst_app.setText(t("launcher.install", app=app.capitalize()))
        if hasattr(self, "app_info"):
            self.app_info.setText(t("launcher.cmd", cmd=launcher.COMMANDS[app]) +
                                  ("" if ok else "  ·  " + t("launcher.missing", app=app)))
        if hasattr(self, "bound_info") and hasattr(self.win, "hypr_page"):
            from .keys import combo_label
            hp = self.win.hypr_page
            found = model.launcher_binds(hp.hdir(), hp.state["binds"])
            cmd = launcher.COMMANDS[app]
            parts = [f"{combo_label(b['mods'], b['key'])} → {model.bind_cmd(b).split(' ', 1)[0]}" for b, _ in found]
            wrong = [b for b, _ in found if model.bind_cmd(b) != cmd]
            self.bound_info.setText(t("launcher.bound", list="   ·   ".join(parts)) if parts else t("launcher.unbound"))
            set_chip(self.bound_chip, not wrong and bool(found), t("launcher.ok") if not wrong and found
                     else t("launcher.other"))
            self.btn_use.setObjectName("Primary" if wrong or not found else "")
            self.btn_use.style().unpolish(self.btn_use)
            self.btn_use.style().polish(self.btn_use)
