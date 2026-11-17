//! Seiten Hyprland, Waybar und Launcher: Seitenleiste links, oben Live-Vorschau, darunter Presets bzw.
//! Regler-Karten. Ein gemeinsamer Aufbau, die Unterschiede je Seite stehen bei `Kind`.

use super::bar_editor::{BarEditor, ModuleColumns};
use super::dialogs;
use super::draw;
use super::keys::{combo_label, BindEditor};
use super::preview::{render_texture, DesktopPreview};
use super::shell::{self, Conf};
use super::studio::Studio;
use super::theme;
use super::userpresets::{PresetHost, UserPresets, PREFIX};
use super::widgets::{button, chip, flow_grid, hbox, icon_button, label, nav_button, nav_groups, nav_set_text,
                     scroller, set_chip, spacer, tool_button, vbox, vspacer, Card, Host, NavHeader, PresetCard,
                     Switch};
use super::workspaces::WorkspaceEditor;
use crate::i18n::{t, tf};
use crate::jsonx::{self, Get, Obj};
use crate::{kw, launcher, model, packages, sysactions, util};
use gtk::prelude::*;
use gtk::gdk;
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::{Rc, Weak};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Hypr,
    Waybar,
    Launcher,
}

impl Kind {
    pub fn key(self) -> &'static str {
        match self {
            Kind::Hypr => "hypr",
            Kind::Waybar => "waybar",
            Kind::Launcher => "launcher",
        }
    }
    fn sections(self) -> &'static [&'static str] {
        match self {
            Kind::Hypr => &["presets", "windows", "workspaces", "colors", "effects", "anim", "input", "keys", "system",
                "wallpaper", "cursor", "lock"],
            Kind::Waybar => &["presets", "editor", "bar", "modules", "options", "style"],
            Kind::Launcher => &["presets", "app", "window", "colors", "text"],
        }
    }
    fn nav_groups(self) -> Vec<(&'static str, Vec<&'static str>)> {
        match self {
            Kind::Hypr => vec![("look", vec!["presets", "windows", "colors", "effects", "anim"]),
                ("control", vec!["workspaces", "input", "keys", "system"]),
                ("desktop", vec!["wallpaper", "cursor", "lock"])],
            Kind::Waybar => vec![("look", vec!["presets", "bar", "style"]), ("content", vec!["editor", "modules", "options"])],
            Kind::Launcher => vec![("look", vec!["presets", "window", "colors", "text"]), ("program", vec!["app"])],
        }
    }
    fn no_preview(self) -> &'static [&'static str] {
        match self {
            Kind::Hypr => &["workspaces", "input", "keys", "system"],
            Kind::Waybar => &["editor"],
            Kind::Launcher => &[],
        }
    }
    fn has_play(self) -> bool {
        self == Kind::Hypr
    }
    fn look_keys(self) -> Vec<String> {
        match self {
            Kind::Hypr => model::look_keys(),
            Kind::Waybar => model::WB_LOOK.iter().map(|s| s.to_string()).collect(),
            Kind::Launcher => launcher::LOOK.iter().map(|s| s.to_string()).collect(),
        }
    }
    pub fn sanitize(self, d: Option<&Obj>) -> Obj {
        match self {
            Kind::Hypr => model::hypr_state(d),
            Kind::Waybar => model::waybar_state(d),
            Kind::Launcher => launcher::state(d),
        }
    }
}

/// Python-Gleichheit (1 == 1.0).
pub fn py_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(p, q)| py_eq(p, q)),
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).map(|w| py_eq(v, w)).unwrap_or(false))
        }
        (Value::Bool(x), Value::Number(y)) | (Value::Number(y), Value::Bool(x)) => {
            y.as_f64() == Some(if *x { 1.0 } else { 0.0 })
        }
        _ => a == b,
    }
}

/// Eingehängter Bereich (Hintergrundbild, Mauszeiger, Sperrbildschirm): eigene Knöpfe, ohne Vorschau.
pub trait External {
    fn widget(&self) -> gtk::Widget;
    fn actions(&self) -> Vec<gtk::Button>;
    /// Seite 0 = Inhalt, sonst „nicht installiert“.
    fn ready(&self) -> bool;
    fn refresh(&self);
}

// ---------- Kartenbeschreibung ----------

enum Spec {
    Card(&'static str),
    Sl(&'static str, f64, f64, f64, &'static str, Option<&'static str>, Option<&'static str>),
    Tg(&'static str, Option<&'static str>, Option<&'static str>),
    Col(&'static str, Option<&'static str>),
    Ch(&'static str, Vec<&'static str>, &'static str, Option<&'static str>),
    Tx(&'static str, Option<&'static str>, i32, Option<&'static str>),
    Fo(&'static str),
    Li(&'static str, Option<&'static str>),
}
use Spec::*;

fn sl(k: &'static str, lo: f64, hi: f64) -> Spec {
    Sl(k, lo, hi, 1.0, "{:.0f}", None, None)
}
fn slf(k: &'static str, lo: f64, hi: f64, step: f64, fmt: &'static str) -> Spec {
    Sl(k, lo, hi, step, fmt, None, None)
}
fn slt(k: &'static str, lo: f64, hi: f64, step: f64, fmt: &'static str, tip: &'static str) -> Spec {
    Sl(k, lo, hi, step, fmt, Some(tip), None)
}
fn tg(k: &'static str) -> Spec {
    Tg(k, None, None)
}
fn tgt(k: &'static str, tip: &'static str) -> Spec {
    Tg(k, Some(tip), None)
}
fn col(k: &'static str) -> Spec {
    Col(k, None)
}
fn ch(k: &'static str, o: &[&'static str], p: &'static str) -> Spec {
    Ch(k, o.to_vec(), p, None)
}
fn tx(k: &'static str, w: i32) -> Spec {
    Tx(k, None, w, None)
}
fn txt(k: &'static str, tip: &'static str, w: i32) -> Spec {
    Tx(k, Some(tip), w, None)
}

fn specs(kind: Kind, key: &str) -> Vec<Spec> {
    match (kind, key) {
        (Kind::Hypr, "windows") => vec![
            Card("card.gaps"), sl("gaps_in", 0.0, 40.0), sl("gaps_out", 0.0, 60.0), tgt("smart_gaps", "tip.smart_gaps"),
            Card("card.border"), sl("border_size", 0.0, 8.0), sl("rounding", 0.0, 30.0),
            slt("rounding_power", 2.0, 10.0, 0.5, "{:.1f}", "tip.rounding_power"), tg("resize_on_border"),
            Card("card.layout"), ch("layout", &["dwindle", "master"], "layout."),
            Card("card.dwindle"), tg("dwindle_preserve"), ch("dwindle_force", &["0", "1", "2"], "force."),
            tgt("dwindle_smart", "tip.dwindle_smart"),
            Card("card.master"), ch("master_new", &["master", "slave", "inherit"], "mnew."),
            slf("master_mfact", 0.1, 0.9, 0.05, "{:.2f}"),
            ch("master_orient", &["left", "right", "top", "bottom", "center"], "orient."),
        ],
        (Kind::Hypr, "colors") => vec![
            Card("card.border_col"), Col("active1", Some("active")), tg("gradient"), col("active2"),
            slf("angle", 0.0, 360.0, 5.0, "{:.0f}°"), col("inactive"), tgt("border_rotate", "tip.border_rotate"),
            slt("border_rotate_speed", 10.0, 200.0, 5.0, "{:.0f}", "tip.border_rotate_speed"),
            Card("card.shadow"), col("shadow_color"),
            Card("card.background"), col("bg_color"),
        ],
        (Kind::Hypr, "effects") => vec![
            Card("card.opacity"), slf("active_opacity", 0.3, 1.0, 0.01, "{:.2f}"),
            slf("inactive_opacity", 0.3, 1.0, 0.01, "{:.2f}"), tg("dim_inactive"),
            slf("dim_strength", 0.0, 1.0, 0.05, "{:.2f}"),
            Card("card.blur"), tg("blur"), sl("blur_size", 1.0, 20.0), sl("blur_passes", 1.0, 6.0),
            slf("blur_vibrancy", 0.0, 1.0, 0.01, "{:.2f}"), slf("blur_noise", 0.0, 0.2, 0.0025, "{:.4f}"),
            slf("blur_contrast", 0.0, 2.0, 0.02, "{:.2f}"), slf("blur_brightness", 0.0, 2.0, 0.02, "{:.2f}"),
            tgt("blur_xray", "tip.blur_xray"), tg("blur_ignore_opacity"),
            Card("card.shadow"), tg("shadow"), sl("shadow_range", 0.0, 40.0), sl("shadow_power", 1.0, 4.0),
            tg("shadow_sharp"),
        ],
        (Kind::Hypr, "anim") => vec![
            Card("card.anim"), tg("anim"), Ch("anim_curve", model::CURVES.iter().map(|c| c.0).collect(), "curve.", None),
            slt("anim_tempo", 0.5, 2.0, 0.1, "{:.1f}×", "tip.anim_tempo"),
            Ch("win_style", model::WIN_STYLES.iter().map(|c| c.0).collect(), "win.", None),
            Ch("ws_style", model::WS_STYLES.iter().map(|c| c.0).collect(), "ws.", None),
            Card("card.anim_parts"), tg("anim_windows"), tg("anim_fade"), tg("anim_ws"), tg("anim_border"),
            tg("anim_layers"),
        ],
        (Kind::Hypr, "input") => vec![
            Card("card.kb"), txt("kb_layout", "tip.kb_layout", 160), txt("kb_variant", "tip.kb_variant", 160),
            txt("kb_options", "tip.kb_options", 260), tg("numlock"), slf("repeat_rate", 10.0, 100.0, 1.0, "{:.0f}/s"),
            slf("repeat_delay", 100.0, 1500.0, 50.0, "{:.0f} ms"),
            Card("card.mouse"), slf("sensitivity", -1.0, 1.0, 0.05, "{:+.2f}"),
            ch("accel", &["default", "flat", "adaptive"], "accel."), ch("follow_mouse", &["0", "1", "2"], "fm."),
            tg("left_handed"), tgt("mouse_natural", "tip.scroll"),
            Card("card.touchpad"), tg("natural_scroll"), tg("dwt"), slf("scroll_factor", 0.1, 3.0, 0.1, "{:.1f}×"),
            Ch("gesture", vec!["0", "3", "4"], "gest.", Some("tip.gesture")),
            Card("card.cursor"), slf("cursor_size", 16.0, 64.0, 2.0, "{:.0f}"),
            txt("cursor_theme", "tip.cursor_theme", 220),
            slt("cursor_timeout", 0.0, 30.0, 1.0, "{:.0f} s", "tip.cursor_timeout"), tg("cursor_hide_keys"),
        ],
        (Kind::Hypr, "system") => vec![
            Card("card.autostart"), tg("autostart_waybar"), Li("autostart", Some("tip.autostart")),
            Card("card.monitor"), tgt("monitor_on", "tip.monitor_on"),
            ch("monitor_mode", &["preferred", "highrr", "highres"], "mmode."),
            ch("monitor_scale", &["auto", "1", "1.25", "1.5", "2"], "scale."),
            Card("card.misc"), tg("no_logo"), tg("focus_on_activate"), tgt("swallow", "tip.swallow"),
            tx("swallow_regex", 260), tg("mmb_paste"), tg("dpms_mouse"), tg("dpms_key"),
            Card("card.skip"), Li("skip_opts", Some("tip.skip_opts")),
        ],
        (Kind::Waybar, "bar") => vec![
            Card("card.shape"), ch("position", &["top", "bottom"], "pos."), ch("style", &["bar", "float", "islands"], "st."),
            Ch("layer", vec!["top", "bottom"], "layer.", Some("tip.layer")), tgt("exclusive", "tip.exclusive"),
            Card("card.size"), sl("height", 20.0, 60.0), sl("margin", 0.0, 30.0), sl("radius", 0.0, 30.0),
            sl("spacing", 0.0, 20.0), sl("pad", 0.0, 24.0), sl("border_width", 0.0, 4.0),
        ],
        (Kind::Waybar, "options") => vec![
            Card("card.ws"), ch("ws_format", &["number", "dot"], "wsf."),
            ch("ws_look", &["pill", "box", "underline", "text"], "wsl."),
            slt("ws_persist", 0.0, 10.0, 1.0, "{:.0f}", "tip.ws_persist"),
            Card("card.clock"), ch("clock_fmt", &["hm", "hms", "dhm", "full"], "cf."), tg("clock24"),
            Card("card.modopts"), sl("win_len", 10.0, 120.0), sl("mpris_len", 10.0, 80.0),
            ch("net_fmt", &["essid", "ip", "signal"], "net."), slf("vol_step", 1.0, 20.0, 1.0, "{:.0f} %"),
            slf("bat_warn", 5.0, 60.0, 1.0, "{:.0f} %"), slf("bat_crit", 2.0, 40.0, 1.0, "{:.0f} %"),
            slf("interval", 1.0, 30.0, 1.0, "{:.0f} s"), sl("tray_size", 12.0, 32.0), tg("tooltips"),
        ],
        (Kind::Waybar, "style") => vec![
            Card("card.colors"), col("bg"), col("fg"), col("accent"), col("border"), tgt("colorful", "tip.colorful"),
        ],
        (Kind::Waybar, "style2") => vec![
            Card("card.font"), Fo("font"), sl("font_size", 9.0, 24.0), ch("font_weight", &["normal", "600", "bold"], "fw."),
            tg("icons"),
        ],
        (Kind::Launcher, "lmode") => vec![
            Card("card.lmode"), ch("mode", &["drun", "run"], "lmode."), tgt("name_search", "tip.name_search"),
            Tg("toggle", Some("tip.ltoggle"), Some("ltoggle")), ch("matching", &["fuzzy", "normal"], "match."),
            tg("insensitive"), tx("terminal", 200),
        ],
        (Kind::Launcher, "window") => vec![
            Card("card.lsize"), slf("width", 300.0, 1200.0, 10.0, "{:.0f} px"), sl("lines", 3.0, 15.0),
            tgt("grid", "tip.grid"), Sl("columns", 1.0, 8.0, 1.0, "{:.0f}", Some("tip.columns"), None),
            slf("icon_size", 16.0, 64.0, 2.0, "{:.0f} px"), Sl("padding", 0.0, 40.0, 1.0, "{:.0f}", None, Some("pad")),
            Sl("spacing", 0.0, 16.0, 1.0, "{:.0f}", None, Some("lspacing")),
            Card("card.lpos"), ch("position", &["center", "top", "bottom"], "lpos."),
            slf("offset", 0.0, 300.0, 5.0, "{:.0f} px"),
            Card("card.lshape"), sl("border_width", 0.0, 6.0), sl("radius", 0.0, 30.0), sl("el_radius", 0.0, 20.0),
        ],
        (Kind::Launcher, "colors") => vec![
            Card("card.colors"), col("bg"), col("input_bg"), col("fg"), col("accent"), col("sel"), col("sel_fg"),
            col("border"),
        ],
        (Kind::Launcher, "text") => vec![
            Card("card.font"), Fo("font"), sl("font_size", 8.0, 24.0),
            Card("card.ltext"), tg("show_prompt"), tx("prompt", 200), tx("placeholder", 260),
            Tg("icons", None, Some("app_icons")), txt("icon_theme", "tip.icon_theme", 220), tg("show_scroll"),
        ],
        _ => vec![],
    }
}

// ---------- Seite ----------

pub struct EditorPage {
    pub kind: Kind,
    pub root: gtk::Box,
    win: Weak<Studio>,
    conf: Conf,
    pub state: RefCell<Obj>,
    cards: RefCell<Vec<Rc<Card>>>,
    section_cards: RefCell<Vec<(String, Vec<Rc<Card>>)>>,
    extra_refresh: RefCell<Vec<Box<dyn Fn(&Obj)>>>,
    pub switches: RefCell<Vec<Rc<Switch>>>,
    presets: RefCell<Vec<(String, Rc<PresetCard>)>>,
    presets_dirty: Cell<bool>,
    up: RefCell<Option<Rc<UserPresets>>>,
    nav: Vec<(String, gtk::ToggleButton)>,
    nav_heads: RefCell<Vec<Rc<NavHeader>>>,
    info: gtk::Label,
    pub preview: Rc<DesktopPreview>,
    page_title: gtk::Label,
    b_reset: gtk::Button,
    applied: gtk::Label,
    acts: gtk::Box,
    acts_spacer: gtk::Box,
    btn_play: gtk::Button,
    btn_undo: gtk::Button,
    btn_apply: gtk::Button,
    top_box: gtk::Box,
    ext_head: gtk::Box,
    ext_title: gtk::Label,
    ext_btns: gtk::Box,
    stack: gtk::Stack,
    pages: RefCell<Vec<(String, gtk::Widget)>>,
    externals: RefCell<Vec<(String, Rc<dyn External>)>>,
    // Hyprland
    ws_toggle: RefCell<Option<Rc<Switch>>>,
    wsed: RefCell<Option<Rc<WorkspaceEditor>>>,
    pub binds: RefCell<Option<Rc<BindEditor>>>,
    // Waybar
    on_switch: RefCell<Option<Rc<Switch>>>,
    on_label: gtk::Label,
    btn_install: gtk::Button,
    bared: RefCell<Option<Rc<BarEditor>>>,
    mods: RefCell<Option<Rc<ModuleColumns>>>,
    btn_from: RefCell<Option<gtk::Button>>,
    // Launcher
    app_info: gtk::Label,
    btn_inst_app: gtk::Button,
    bound_info: gtk::Label,
    bound_chip: gtk::Label,
    btn_use: gtk::Button,
    sys_sw: RefCell<Option<Rc<Switch>>>,
    pending_push: RefCell<Option<String>>,
}

impl Host for EditorPage {
    fn value(&self, key: &str) -> Value {
        self.state.borrow().val(key).clone()
    }
    fn set_value(&self, key: &str, v: Value) {
        if let Some(me) = self.me() {
            me.set(key, v);
        }
    }
}

thread_local! {
    static PAGES: RefCell<Vec<Weak<EditorPage>>> = const { RefCell::new(Vec::new()) };
}

impl EditorPage {
    fn me(&self) -> Option<Rc<EditorPage>> {
        PAGES.with(|p| p.borrow().iter().filter_map(|w| w.upgrade()).find(|x| std::ptr::eq(x.as_ref(), self)))
    }

    pub fn win(&self) -> Rc<Studio> {
        self.win.upgrade().expect("Fenster")
    }

    pub fn new(win: &Rc<Studio>, kind: Kind) -> Rc<EditorPage> {
        let conf = win.conf();
        let saved = conf.cfg.borrow().obj(kind.key()).cloned();
        let state = kind.sanitize(saved.as_ref());
        let root = hbox(14);
        let side = vbox(4);
        side.add_css_class("Side");
        side.set_size_request(190, -1);
        super::widgets::margins(&side, 10, 12, 10, 12);
        let mut nav = vec![];
        let mut first: Option<gtk::ToggleButton> = None;
        for key in kind.sections() {
            let b = nav_button(first.as_ref());
            if first.is_none() {
                first = Some(b.clone());
            }
            nav.push((key.to_string(), b));
        }
        let info = label("", "Label");
        info.set_wrap(true);
        let right = vbox(14);
        right.set_hexpand(true);
        let top = hbox(14);
        let preview = DesktopPreview::new();
        preview.area.set_content_height(290);
        preview.area.set_size_request(-1, 290);
        top.append(&preview.area);
        let acts = vbox(8);
        let page_title = label("", "PageTitle");
        let trow = hbox(6);
        trow.append(&page_title);
        let b_reset = tool_button("reset", theme::MUTED, 16, "ResetBtn");
        b_reset.set_size_request(30, 30);
        b_reset.set_valign(gtk::Align::Center);
        trow.append(&b_reset);
        acts.append(&trow);
        let applied = chip();
        applied.set_halign(gtk::Align::Start);
        acts.append(&applied);
        let acts_spacer = vspacer();
        acts.append(&acts_spacer);
        let btn_play = icon_button("play", theme::TEXT, 16, "", "");
        let btn_undo = button("", "");
        let btn_apply = button("", "Primary");
        for b in [&btn_play, &btn_undo, &btn_apply] {
            b.set_size_request(170, 40);
            acts.append(b);
        }
        top.append(&acts);
        let top_box = top;
        right.append(&top_box);
        let ext_head = hbox(8);
        let ext_title = label("", "PageTitle");
        ext_title.set_size_request(-1, 40);
        ext_head.append(&ext_title);
        ext_head.append(&spacer());
        let ext_btns = hbox(8);
        ext_head.append(&ext_btns);
        ext_head.set_visible(false);
        right.append(&ext_head);
        let stack = gtk::Stack::new();
        stack.set_vexpand(true);
        stack.set_hhomogeneous(false);
        stack.set_vhomogeneous(false);
        right.append(&stack);
        root.append(&side);
        root.append(&right);
        let p = Rc::new(EditorPage { kind, root, win: Rc::downgrade(win), conf: conf.clone(), state: RefCell::new(state),
            cards: RefCell::new(vec![]), section_cards: RefCell::new(vec![]), extra_refresh: RefCell::new(vec![]),
            switches: RefCell::new(vec![]), presets: RefCell::new(vec![]), presets_dirty: Cell::new(true),
            up: RefCell::new(None), nav, nav_heads: RefCell::new(vec![]), info, preview, page_title, b_reset, applied,
            acts, acts_spacer, btn_play, btn_undo, btn_apply, top_box, ext_head, ext_title, ext_btns, stack,
            pages: RefCell::new(vec![]), externals: RefCell::new(vec![]), ws_toggle: RefCell::new(None),
            wsed: RefCell::new(None), binds: RefCell::new(None), on_switch: RefCell::new(None),
            on_label: label("", "RowLabel"), btn_install: button("", ""), bared: RefCell::new(None),
            mods: RefCell::new(None), btn_from: RefCell::new(None), app_info: label("", "RowLabel"),
            btn_inst_app: button("", "Primary"), bound_info: label("", "RowLabel"), bound_chip: chip(),
            btn_use: button("", ""), sys_sw: RefCell::new(None), pending_push: RefCell::new(None) });
        PAGES.with(|x| x.borrow_mut().push(Rc::downgrade(&p)));
        for (key, b) in &p.nav {
            let w = Rc::downgrade(&p);
            let k = key.clone();
            b.connect_clicked(move |_| {
                if let Some(p) = w.upgrade() {
                    p.show_section(&k);
                }
            });
        }
        let heads = nav_groups(&side, &p.nav, &kind.nav_groups(), kind.key(), &conf);
        *p.nav_heads.borrow_mut() = heads;
        side.append(&vspacer());
        side.append(&p.info);
        let w = Rc::downgrade(&p);
        p.b_reset.connect_clicked(move |_| {
            if let Some(p) = w.upgrade() {
                p.reset_section();
            }
        });
        let w = Rc::downgrade(&p);
        p.btn_play.connect_clicked(move |_| {
            if let Some(p) = w.upgrade() {
                p.preview.play();
            }
        });
        let w = Rc::downgrade(&p);
        p.btn_undo.connect_clicked(move |_| {
            if let Some(p) = w.upgrade() {
                p.undo();
            }
        });
        let w = Rc::downgrade(&p);
        p.btn_apply.connect_clicked(move |_| {
            if let Some(p) = w.upgrade() {
                p.apply();
            }
        });
        for key in kind.sections() {
            let inner = vbox(14);
            inner.add_css_class("ScrollInner");
            inner.set_margin_end(6);
            let before = p.cards.borrow().len();
            p.build_section(key, &inner);
            let new: Vec<Rc<Card>> = p.cards.borrow()[before..].to_vec();
            p.section_cards.borrow_mut().push((key.to_string(), new));
            let sa = scroller(&inner);
            p.stack.add_named(&sa, Some(key));
            p.pages.borrow_mut().push((key.to_string(), sa.upcast()));
        }
        match kind {
            Kind::Waybar => {
                p.preview.zoom_bar.set(true);
                let row = hbox(8);
                let sw = Switch::new();
                sw.set_now(p.state.borrow().b("enabled"));
                let w = Rc::downgrade(&p);
                sw.connect(move |on| {
                    if let Some(p) = w.upgrade() {
                        p.set_enabled(on);
                    }
                });
                row.append(&sw.area);
                row.append(&p.on_label);
                p.btn_install.set_margin_start(8);
                row.append(&p.btn_install);
                let w = Rc::downgrade(&p);
                p.btn_install.connect_clicked(move |_| {
                    if let Some(p) = w.upgrade() {
                        p.win().install_keys(&["waybar"]);
                    }
                });
                p.acts.insert_child_after(&row, Some(&p.applied));
                p.switches.borrow_mut().push(sw.clone());
                *p.on_switch.borrow_mut() = Some(sw);
                p.preview.area.set_content_height(190);
                p.preview.area.set_size_request(-1, 190);
            }
            Kind::Launcher => {
                p.preview.set_launcher(Some(&p.state.borrow()));
            }
            Kind::Hypr => {}
        }
        p.refresh_controls();
        let sec = conf.cfg.borrow().s(&format!("{}_section", kind.key())).to_string();
        p.show_section(if sec.is_empty() { kind.sections()[0] } else { &sec });
        p
    }

    fn cfg_get(&self, k: &str) -> Value {
        self.conf.cfg.borrow().val(k).clone()
    }

    fn cfg_set(&self, k: &str, v: Value) {
        self.conf.cfg.borrow_mut().insert(k.to_string(), v);
    }

    fn kk(&self, suffix: &str) -> String {
        format!("{}_{}", self.kind.key(), suffix)
    }

    fn host_weak(self: &Rc<Self>) -> Weak<dyn Host> {
        let w: Weak<EditorPage> = Rc::downgrade(self);
        w
    }

    fn card(self: &Rc<Self>, layout: &gtk::Box, title: &str) -> Rc<Card> {
        let c = Card::new(title, self.host_weak());
        layout.append(&c.root);
        self.cards.borrow_mut().push(c.clone());
        c
    }

    fn build_specs(self: &Rc<Self>, layout: &gtk::Box, specs: Vec<Spec>) -> Option<Rc<Card>> {
        let mut c: Option<Rc<Card>> = None;
        for s in specs {
            match s {
                Card(title) => c = Some(self.card(layout, title)),
                Sl(k, lo, hi, step, fmt, tip, lk) => {
                    c.as_ref().unwrap().slider(k, lo, hi, step, fmt, tip, lk);
                }
                Tg(k, tip, lk) => {
                    let sw = c.as_ref().unwrap().toggle(k, tip, lk);
                    self.switches.borrow_mut().push(sw);
                }
                Col(k, lk) => {
                    c.as_ref().unwrap().color(k, lk, None);
                }
                Ch(k, o, p, tip) => {
                    c.as_ref().unwrap().choice(k, &o, p, tip, None);
                }
                Tx(k, tip, w, lk) => {
                    c.as_ref().unwrap().text(k, tip, w, lk);
                }
                Fo(k) => {
                    c.as_ref().unwrap().font(k, None, None);
                }
                Li(k, tip) => {
                    c.as_ref().unwrap().lines(k, tip);
                }
            }
        }
        c
    }

    fn preset_names(&self) -> (Vec<String>, &'static str) {
        match self.kind {
            Kind::Hypr => (model::hypr_presets().keys().cloned().collect(), "p."),
            Kind::Waybar => (model::waybar_presets().keys().cloned().collect(), "wp."),
            Kind::Launcher => (launcher::presets().keys().cloned().collect(), "lp."),
        }
    }

    fn preset_grid(self: &Rc<Self>, layout: &gtk::Box) {
        let up = UserPresets::build(layout);
        let w: Weak<dyn PresetHost> = {
            let x: Weak<EditorPage> = Rc::downgrade(self);
            x
        };
        up.set_host(w);
        *self.up.borrow_mut() = Some(up.clone());
        up.reload();
        let grid = flow_grid();
        let (names, prefix) = self.preset_names();
        for n in names {
            let pc = PresetCard::new(&n);
            *pc.tkey.borrow_mut() = format!("{prefix}{n}");
            let w = Rc::downgrade(self);
            pc.connect(move |k| {
                if let Some(p) = w.upgrade() {
                    p.use_preset(&k);
                }
            });
            grid.append(&pc.root);
            self.presets.borrow_mut().push((n.clone(), pc));
        }
        layout.append(&grid);
    }

    fn build_section(self: &Rc<Self>, key: &str, lay: &gtk::Box) {
        if key == "presets" {
            self.preset_grid(lay);
            return;
        }
        match (self.kind, key) {
            (Kind::Hypr, "workspaces") => {
                let c = self.card(lay, "card.workspaces");
                let sw = c.toggle("workspaces_on", Some("tip.workspaces_on"), None);
                self.switches.borrow_mut().push(sw.clone());
                *self.ws_toggle.borrow_mut() = Some(sw);
                let ed = WorkspaceEditor::new();
                let w = Rc::downgrade(self);
                ed.connect_changed(move |_| {
                    if let Some(p) = w.upgrade() {
                        p.ws_changed();
                    }
                });
                c.append(&ed.widget);
                let e2 = ed.clone();
                self.extra_refresh.borrow_mut().push(Box::new(move |s| e2.load(s.arr("workspaces"))));
                *self.wsed.borrow_mut() = Some(ed);
            }
            (Kind::Hypr, "keys") => {
                let c = self.card(lay, "card.keys");
                let be = BindEditor::new();
                let w = Rc::downgrade(self);
                be.connect_changed(move |_| {
                    if let Some(p) = w.upgrade() {
                        p.binds_changed();
                    }
                });
                let w = Rc::downgrade(self);
                be.connect_fix(move |_| {
                    if let Some(p) = w.upgrade() {
                        p.fix_conflicts();
                    }
                });
                let w = Rc::downgrade(self);
                be.connect_import(move |_| {
                    if let Some(p) = w.upgrade() {
                        p.import_binds();
                    }
                });
                let conf = self.conf.clone();
                *be.user_store.borrow_mut() = Some((conf.cfg.clone(), conf.save.clone()));
                c.append(&be.widget);
                let b2 = be.clone();
                let w = Rc::downgrade(self);
                self.extra_refresh.borrow_mut().push(Box::new(move |s| {
                    b2.load(s.arr("binds"));
                    if let Some(p) = w.upgrade() {
                        p.mark();
                    }
                }));
                *self.binds.borrow_mut() = Some(be);
            }
            (Kind::Waybar, "editor") => {
                let c = self.card(lay, "card.editor");
                let be = BarEditor::new();
                be.area.set_tooltip_text(Some(&t("tip.editor")));
                let w = Rc::downgrade(self);
                be.connect(move |lists| {
                    if let Some(p) = w.upgrade() {
                        p.editor_changed(lists);
                    }
                });
                c.append(&be.area);
                let b2 = be.clone();
                self.extra_refresh.borrow_mut().push(Box::new(move |s| b2.load(s)));
                *self.bared.borrow_mut() = Some(be);
            }
            (Kind::Waybar, "modules") => {
                let c = self.card(lay, "sec.modules");
                let mc = ModuleColumns::new();
                let w = Rc::downgrade(self);
                mc.connect_changed(move |_| {
                    if let Some(p) = w.upgrade() {
                        p.mods_changed();
                    }
                });
                c.append(&mc.widget);
                let m2 = mc.clone();
                self.extra_refresh.borrow_mut().push(Box::new(move |s| m2.load(s)));
                *self.mods.borrow_mut() = Some(mc);
                let c2 = self.card(lay, "card.extra");
                c2.text("launcher", None, 260, None);
                c2.text("power_cmd", None, 260, None);
                c2.text("vol_click", None, 260, None);
            }
            (Kind::Waybar, "style") => {
                let c = self.build_specs(lay, specs(Kind::Waybar, "style")).unwrap();
                let b = button("", "");
                b.set_halign(gtk::Align::Start);
                let w = Rc::downgrade(self);
                b.connect_clicked(move |_| {
                    if let Some(p) = w.upgrade() {
                        p.from_hypr();
                    }
                });
                c.append(&b);
                *self.btn_from.borrow_mut() = Some(b);
                self.build_specs(lay, specs(Kind::Waybar, "style2"));
            }
            (Kind::Launcher, "app") => {
                let c = self.card(lay, "card.app");
                c.choice("app", launcher::APPS, "app.", None, None);
                c.append(&self.app_info);
                self.btn_inst_app.set_halign(gtk::Align::Start);
                let w = Rc::downgrade(self);
                self.btn_inst_app.connect_clicked(move |_| {
                    if let Some(p) = w.upgrade() {
                        let app = p.state.borrow().s("app").to_string();
                        p.win().install_keys(&[&app]);
                    }
                });
                c.append(&self.btn_inst_app);
                let brow = hbox(10);
                self.bound_info.set_wrap(true);
                self.bound_info.set_hexpand(true);
                brow.append(&self.bound_chip);
                brow.append(&self.bound_info);
                c.append(&brow);
                self.btn_use.set_halign(gtk::Align::Start);
                let w = Rc::downgrade(self);
                self.btn_use.connect_clicked(move |_| {
                    if let Some(p) = w.upgrade() {
                        p.use_everywhere();
                    }
                });
                c.append(&self.btn_use);
                // Herunterfahren, Neustart … als Einträge für alle Launcher (sysactions)
                let c = self.card(lay, "card.sys");
                let sw = Switch::new();
                sw.set_now(self.conf.cfg.borrow().get("sys_actions").map(jsonx::truthy).unwrap_or(true));
                let w = Rc::downgrade(self);
                sw.connect(move |on| {
                    if let Some(p) = w.upgrade() {
                        p.set_sys_actions(on);
                    }
                });
                c.add_row("sys.on", sw.area.upcast_ref(), None, Some("tip.sys.on"), None, Some("sys.on"));
                self.switches.borrow_mut().push(sw.clone());
                *self.sys_sw.borrow_mut() = Some(sw);
                self.build_specs(lay, specs(Kind::Launcher, "lmode"));
            }
            (Kind::Launcher, "colors") => {
                let c = self.build_specs(lay, specs(Kind::Launcher, "colors")).unwrap();
                let b = button("", "");
                b.set_halign(gtk::Align::Start);
                let w = Rc::downgrade(self);
                b.connect_clicked(move |_| {
                    if let Some(p) = w.upgrade() {
                        p.from_hypr();
                    }
                });
                c.append(&b);
                *self.btn_from.borrow_mut() = Some(b);
            }
            (kind, key) => {
                self.build_specs(lay, specs(kind, key));
            }
        }
    }

    // ----- Eingehängte Bereiche -----

    /// Fremden Bereich (eigene Knöpfe, ohne Vorschau) in die Seitenleiste hängen, z. B. Hintergrundbild.
    pub fn add_external(self: &Rc<Self>, key: &str, ext: Rc<dyn External>) {
        let mut pages = self.pages.borrow_mut();
        if let Some(i) = pages.iter().position(|(k, _)| k == key) {
            self.stack.remove(&pages[i].1);
            let w = ext.widget();
            self.stack.add_named(&w, Some(key));
            pages[i].1 = w;
        }
        for b in ext.actions() {
            self.ext_btns.append(&b);
            b.set_visible(false);
        }
        self.externals.borrow_mut().push((key.to_string(), ext));
    }

    /// Kopfzeile passend zum offenen eingehängten Bereich; ohne installiertes Programm keine Knöpfe.
    pub fn sync_external(&self) {
        let key = self.conf.cfg.borrow().s(&self.kk("section")).to_string();
        let exts = self.externals.borrow();
        let ext = exts.iter().find(|(k, _)| *k == key).map(|x| x.1.clone());
        self.ext_head.set_visible(ext.is_some());
        let Some(ext) = ext else { return };
        self.ext_title.set_text(&t(&format!("sec.{key}")));
        let ready = ext.ready();
        for (k, e) in exts.iter() {
            for b in e.actions() {
                b.set_visible(*k == key && ready);
            }
        }
    }

    fn reset_section(self: &Rc<Self>) {
        let mut key = self.conf.cfg.borrow().s(&self.kk("section")).to_string();
        if key.is_empty() {
            key = self.kind.sections()[0].to_string();
        }
        let cards = self.section_cards.borrow().iter().find(|(k, _)| *k == key).map(|x| x.1.clone());
        let Some(cards) = cards else { return };
        let title = t(&format!("sec.{key}"));
        let w = Rc::downgrade(self);
        let kind = format!("{}-{}", self.kind.key(), key);
        dialogs::confirm(shell::toplevel_of(&self.root).as_ref(), &t("reset.title"),
            &tf("reset.ask", kw!("page" => title)), Some(&t("reset.btn")), true, move |ok| {
                let (true, Some(p)) = (ok, w.upgrade()) else { return };
                let dir = backup_dir();
                let path = dir.join(format!("{kind}.json"));
                let _ = util::write_text(&path, &jsonx::dumps_indent(&Value::Object(p.state.borrow().clone()), 1));
                let fresh = p.kind.sanitize(None);
                let mut keys: Vec<String> = cards.iter().flat_map(|c| c.keys()).filter(|k| fresh.contains_key(k))
                    .collect();
                keys.dedup();
                if keys.is_empty() {
                    keys = fresh.keys().cloned().collect();
                }
                {
                    let mut st = p.state.borrow_mut();
                    for k in &keys {
                        st.insert(k.clone(), fresh[k].clone());
                    }
                }
                p.refresh_controls();
                p.changed();
                p.win().set_status(&tf("reset.section", kw!("page" => title, "path" => path.to_string_lossy())));
            });
    }

    // ----- Zustand -----

    pub fn set(self: &Rc<Self>, key: &str, value: Value) {
        if self.kind == Kind::Launcher && key == "app" {
            // Launcher wechseln = eigene Einstellungen laden + System umstellen
            let cur = self.state.borrow().s("app").to_string();
            if value.as_str() != Some(cur.as_str()) {
                self.switch_app(value.as_str().unwrap_or(""), true);
            }
            return;
        }
        if self.kind == Kind::Hypr && key == "autostart_waybar" {
            // ab jetzt steuert nur noch dieser Schalter den Waybar-Autostart
            self.state.borrow_mut().insert("waybar_managed".into(), json!(true));
        }
        if self.state.borrow().get(key).map(|v| py_eq(v, &value)).unwrap_or(false) {
            return;
        }
        self.state.borrow_mut().insert(key.to_string(), value.clone());
        if self.kind.look_keys().iter().any(|k| k == key) {
            // nur Aussehens-Änderungen lösen das Preset
            self.cfg_set(&self.kk("preset"), json!(""));
        }
        self.changed();
        match self.kind {
            Kind::Hypr => {
                if ["anim_curve", "anim_tempo", "win_style"].contains(&key) && self.state.borrow().b("anim") {
                    self.preview.play();
                }
            }
            Kind::Launcher if key == "toggle" => {
                // steckt in den Hyprland-Kürzeln → dort neu schreiben
                model::LAUNCHER_TOGGLE.store(jsonx::truthy(&value), std::sync::atomic::Ordering::Relaxed);
                if let Some(hp) = self.win().hp() {
                    hp.changed();
                }
            }
            _ => {}
        }
    }

    pub fn changed(self: &Rc<Self>) {
        if self.kind == Kind::Launcher {
            let st = self.state.borrow().clone();
            self.preview.set_launcher(Some(&st));
            let app = st.s("app").to_string();
            let mut c = self.conf.cfg.borrow_mut();
            let la = c.entry("launcher_apps").or_insert_with(|| json!({}));
            if !la.is_object() {
                *la = json!({});
            }
            la.as_object_mut().unwrap().insert(app, Value::Object(st));
        }
        let st = Value::Object(self.state.borrow().clone());
        self.cfg_set(self.kind.key(), st);
        self.conf.save_later();
        self.win().update_previews();
        self.presets_dirty.set(true);
        self.refresh_presets_selection();
        self.update_applied_impl();
    }

    fn sig(&self) -> String {
        jsonx::dumps_sorted(&Value::Object(self.state.borrow().clone()))
    }

    pub fn is_applied(&self) -> bool {
        self.cfg_get(&self.kk("applied_sig")).as_str() == Some(self.sig().as_str())
    }

    fn update_applied_impl(&self) {
        let ok = self.is_applied();
        set_chip(&self.applied, ok, &if ok { t("applied") } else { t("not_applied") });
        self.refresh_presets_selection();
    }

    pub fn apply(self: &Rc<Self>) {
        if !self.do_apply() {
            return;
        }
        self.cfg_set(&self.kk("applied_sig"), json!(self.sig()));
        let pre = self.cfg_get(&self.kk("preset"));
        let pre = pre.as_str().unwrap_or("").to_string();
        self.cfg_set(&self.kk("applied_preset"), json!(pre));
        self.conf.save_later();
        let name = self.presets.borrow().iter().find(|(k, _)| *k == pre).map(|(_, pc)| pc.name.text().to_string());
        self.win().set_status(&(t("applied") + &name.map(|n| format!(": {n}")).unwrap_or_default()));
        self.update_applied_impl();
    }

    fn after_undo(&self, ok: bool) {
        self.win().set_status(&if ok { t("undone") } else { t("nothing") });
        if ok {
            self.cfg_set(&self.kk("applied_sig"), json!(""));
            self.cfg_set(&self.kk("applied_preset"), json!(""));
            self.update_applied_impl();
        }
    }

    pub fn refresh_controls(&self) {
        let s = self.state.borrow().clone();
        for c in self.cards.borrow().iter() {
            c.refresh(&s);
        }
        for f in self.extra_refresh.borrow().iter() {
            f(&s);
        }
    }

    pub fn current_section(&self) -> String {
        self.conf.cfg.borrow().s(&self.kk("section")).to_string()
    }

    pub fn show_section(self: &Rc<Self>, key: &str) {
        let key = if self.kind.sections().contains(&key) { key } else { self.kind.sections()[0] };
        if let Some((_, b)) = self.nav.iter().find(|(k, _)| k == key) {
            b.set_active(true);
        }
        self.stack.set_visible_child_name(key);
        self.cfg_set(&self.kk("section"), json!(key));
        self.conf.save_later();
        let ext = self.externals.borrow().iter().find(|(k, _)| k == key).map(|x| x.1.clone());
        self.top_box.set_visible(ext.is_none());
        if let Some(e) = &ext {
            e.refresh();
        }
        self.sync_external();
        if ext.is_some() {
            return;
        }
        // Bereiche ohne sichtbare Wirkung: Vorschau weg, Knöpfe in eine Zeile → mehr Platz für Einstellungen
        let compact = self.kind.no_preview().contains(&key);
        self.preview.area.set_visible(!compact);
        self.btn_play.set_visible(self.kind.has_play() && !compact);
        self.acts.set_orientation(if compact { gtk::Orientation::Horizontal } else { gtk::Orientation::Vertical });
        self.acts_spacer.set_hexpand(compact);
        self.acts_spacer.set_vexpand(!compact);
        self.acts.set_hexpand(compact);
        if key == "presets" {
            self.render_presets(false);
        }
    }

    pub fn render_presets(&self, force: bool) {
        if !(self.presets_dirty.get() || force) || self.presets.borrow().is_empty() {
            return;
        }
        self.presets_dirty.set(false);
        let items: Vec<(String, Rc<PresetCard>)> = self.presets.borrow().clone();
        for (name, pc) in items {
            let tex = if name.starts_with(PREFIX) {
                let st = self.user_state(&name);
                self.state_texture_impl(&st)
            } else {
                self.preset_texture(&name)
            };
            pc.set_texture(&tex);
        }
        self.refresh_presets_selection();
    }

    pub fn mark_presets_dirty(&self) {
        self.presets_dirty.set(true);
    }

    fn refresh_presets_selection(&self) {
        let sel = self.cfg_get(&self.kk("preset")).as_str().unwrap_or("").to_string();
        let applied = if self.is_applied() {
            self.cfg_get(&self.kk("applied_preset")).as_str().unwrap_or("").to_string()
        } else {
            String::new()
        };
        for (name, pc) in self.presets.borrow().iter() {
            pc.refresh(*name == sel, !applied.is_empty() && *name == applied);
        }
    }

    fn user_state(&self, key: &str) -> Obj {
        self.up.borrow().as_ref().map(|u| u.user_state(key)).unwrap_or_default()
    }

    fn other_states(&self) -> (Obj, Obj) {
        let win = self.win();
        let hs = win.hp().map(|p| p.state.borrow().clone()).unwrap_or_else(|| model::hypr_state(None));
        let ws = win.wp().map(|p| p.state.borrow().clone()).unwrap_or_else(|| model::waybar_state(None));
        (hs, ws)
    }

    fn state_texture_impl(&self, s: &Obj) -> gdk::Texture {
        let (hs, ws) = self.other_states();
        match self.kind {
            Kind::Hypr => {
                let ws = self.win().wp().map(|p| p.state.borrow().clone());
                render_texture(s, ws.as_ref(), 196, 110, 9.0, None, 1.0)
            }
            Kind::Waybar => render_texture(&hs, Some(s), 196, 110, 9.0, None, 2.0),
            Kind::Launcher => render_texture(&hs, Some(&ws), 196, 110, 9.0, Some(s), 1.0),
        }
    }

    fn preset_texture(&self, name: &str) -> gdk::Texture {
        let (hs, ws) = self.other_states();
        let st = self.state.borrow().clone();
        match self.kind {
            Kind::Hypr => {
                let ws = self.win().wp().map(|p| p.state.borrow().clone());
                render_texture(&model::preset_state(name, Some(&st)), ws.as_ref(), 196, 110, 9.0, None, 1.0)
            }
            Kind::Waybar => render_texture(&hs, Some(&model::waybar_preset(name, Some(&st))), 196, 110, 9.0, None, 2.0),
            Kind::Launcher => render_texture(&hs, Some(&ws), 196, 110, 9.0, Some(&launcher::preset(name, Some(&st))), 1.0),
        }
    }

    fn preset_value(&self, name: &str) -> Obj {
        let st = self.state.borrow().clone();
        match self.kind {
            Kind::Hypr => model::preset_state(name, Some(&st)),
            Kind::Waybar => model::waybar_preset(name, Some(&st)),
            Kind::Launcher => launcher::preset(name, Some(&st)),
        }
    }

    pub fn use_preset_impl(self: &Rc<Self>, name: &str) {
        let app = self.state.borrow().s("app").to_string();
        let ns = if name.starts_with(PREFIX) { self.user_state(name) } else { self.preset_value(name) };
        *self.state.borrow_mut() = ns;
        self.refresh_controls();
        self.changed();
        self.cfg_set(&self.kk("preset"), json!(name));
        self.update_applied_impl();
        if self.kind == Kind::Launcher && self.state.borrow().s("app") != app {
            // Presets ändern nur den Look, nicht den Launcher
            self.state.borrow_mut().insert("app".into(), json!(app));
            self.changed();
        }
    }

    pub fn apply_theme(&self, accent: &str) {
        for sw in self.switches.borrow().iter() {
            sw.area.queue_draw();
        }
        let _ = accent;
        self.refresh_presets_selection();
    }

    pub fn retranslate(self: &Rc<Self>) {
        for (key, b) in &self.nav {
            nav_set_text(b, &t(&format!("sec.{key}")));
        }
        for h in self.nav_heads.borrow().iter() {
            h.retranslate();
        }
        for c in self.cards.borrow().iter() {
            c.retranslate();
        }
        for (_, pc) in self.presets.borrow().iter() {
            let tk = pc.tkey.borrow().clone();
            if !tk.is_empty() {
                pc.name.set_text(&t(&tk));
            }
        }
        if let Some(u) = self.up.borrow().as_ref() {
            u.retranslate();
        }
        self.page_title.set_text(&t(&format!("tab.{}", self.kind.key())));
        self.b_reset.set_tooltip_text(Some(&t("reset.tip")));
        super::widgets::set_button_text(&self.btn_play, &t("play"));
        self.btn_undo.set_label(&t("undo"));
        self.btn_undo.set_tooltip_text(Some(&t("tip.undo")));
        self.btn_apply.set_label(&t("apply"));
        self.update_info();
        self.update_applied_impl();
        match self.kind {
            Kind::Hypr => {
                if let Some(b) = self.binds.borrow().clone() {
                    b.retranslate();
                }
                if let Some(e) = self.wsed.borrow().clone() {
                    let items = self.state.borrow().arr("workspaces").to_vec();
                    e.retranslate(&items);
                    self.mark();
                }
            }
            Kind::Waybar => {
                if let Some(m) = self.mods.borrow().clone() {
                    m.retranslate();
                }
                if let Some(b) = self.btn_from.borrow().as_ref() {
                    b.set_label(&t("from_hypr"));
                }
                if let Some(b) = self.bared.borrow().as_ref() {
                    b.area.set_tooltip_text(Some(&t("tip.editor")));
                    b.area.queue_draw();
                }
            }
            Kind::Launcher => {
                self.btn_use.set_label(&t("launcher.use"));
                self.btn_use.set_tooltip_text(Some(&t("tip.launcher.use")));
                if let Some(b) = self.btn_from.borrow().as_ref() {
                    b.set_label(&t("from_hypr"));
                }
            }
        }
    }

    // ----- je Seite -----

    pub fn hdir(&self) -> PathBuf {
        model::config_home().join("hypr")
    }

    fn wdir(&self) -> PathBuf {
        model::config_home().join("waybar")
    }

    fn do_apply(self: &Rc<Self>) -> bool {
        let win = self.win();
        let st = self.state.borrow().clone();
        let log = |s: &str| win.log(s);
        match self.kind {
            Kind::Hypr => {
                let (ok, info) = model::apply_hypr(&st, &self.hdir(), &log);
                win.report(ok, &info);
                self.update_info();
                self.mark();
                ok
            }
            Kind::Waybar => {
                if !model::which("waybar") {
                    win.report(false, "nowaybar");
                    return false;
                }
                let (ok, info) = model::apply_waybar(&st, &self.wdir(), &log, st.b("enabled"));
                win.report(ok, &info);
                ok
            }
            Kind::Launcher => {
                let app = st.s("app").to_string();
                if app != "llauncher" && !model::which(&app) {
                    win.log(&tf("launcher.missing", kw!("app" => app)));
                }
                let per_app = self.conf.cfg.borrow().obj("launcher_apps").cloned();
                let (ok, info) = launcher::apply(&st, &model::config_home(), &log, per_app.as_ref());
                win.report(ok, &info);
                if let Some(hp) = win.hp() {
                    let any = hp.state.borrow().arr("binds").iter().any(|b| b.get("action").and_then(|a| a.as_str())
                        == Some("launcher"));
                    if ok && any {
                        hp.apply(); // Umschalten (zweites Drücken schließt) steckt im Kürzel
                    }
                }
                ok
            }
        }
    }

    fn undo(&self) {
        let win = self.win();
        win.log("");
        let log = |s: &str| win.log(s);
        let ok = match self.kind {
            Kind::Hypr => model::undo_hypr(&self.hdir(), &log),
            Kind::Waybar => model::undo_waybar(&self.wdir(), &log),
            Kind::Launcher => launcher::undo(&self.state.borrow(), &model::config_home()),
        };
        self.after_undo(ok);
    }

    pub fn update_info(self: &Rc<Self>) {
        match self.kind {
            Kind::Hypr => {
                let fmt = model::hypr_format(&self.hdir()).unwrap_or_else(|| "none".into());
                self.info.set_text(&format!("Hyprland · {}", t(&format!("fmt.{fmt}"))));
            }
            Kind::Waybar => {
                let ok = model::which("waybar") && !self.win().fake_missing.get();
                self.info.set_text(&format!("Waybar · {}", if ok { t("pkg.installed") } else { t("fmt.none") }));
                self.btn_install.set_visible(!ok);
                self.btn_install.set_label(&t("wb.install"));
                self.on_label.set_text(&t("wb.enabled"));
                if let Some(sw) = self.on_switch.borrow().as_ref() {
                    sw.set_now(self.state.borrow().b("enabled"));
                }
            }
            Kind::Launcher => self.update_info_launcher(),
        }
    }

    // ----- Hyprland -----

    fn ws_changed(self: &Rc<Self>) {
        let Some(ed) = self.wsed.borrow().clone() else { return };
        let items = ed.read();
        let turn_on = !items.is_empty() && !self.state.borrow().b("workspaces_on");
        self.state.borrow_mut().insert("workspaces".into(), Value::Array(items));
        if turn_on {
            self.state.borrow_mut().insert("workspaces_on".into(), json!(true));
            if let Some(sw) = self.ws_toggle.borrow().as_ref() {
                sw.set_now(true); // nicht alles neu laden, sonst verliert das Eingabefeld den Fokus
            }
        }
        self.changed();
    }

    fn binds_changed(self: &Rc<Self>) {
        let Some(be) = self.binds.borrow().clone() else { return };
        self.state.borrow_mut().insert("binds".into(), Value::Array(be.read()));
        self.mark();
        self.changed();
    }

    fn conflicts(&self) -> HashSet<model::Combo> {
        let main = self.hdir().join("hyprland.lua");
        if !main.is_file() {
            return HashSet::new();
        }
        let text = String::from_utf8_lossy(&std::fs::read(&main).unwrap_or_default()).to_string();
        let mine: HashSet<model::Combo> = self.state.borrow().arr("binds").iter().filter_map(|b| b.as_object())
            .filter(|b| model::bind_ok(b)).map(|b| model::combo(&b.strs("mods"), b.s("key"))).collect();
        model::lua_bind_combos(&text).into_iter().map(|(_, c)| c).filter(|c| mine.contains(c)).collect()
    }

    pub fn mark(&self) {
        if let Some(b) = self.binds.borrow().clone() {
            b.mark(&self.conflicts());
        }
    }

    fn import_binds(self: &Rc<Self>) {
        let win = self.win();
        win.log("");
        let got = model::import_binds(&self.hdir(), &|s| win.log(s));
        if got.is_empty() {
            win.set_status(&tf("keys.imported", kw!("n" => 0)));
            return;
        }
        let n = got.len();
        if let Some(b) = self.binds.borrow().clone() {
            b.merge(got.into_iter().map(Value::Object).collect());
        }
        // Originale sind jetzt auskommentiert → sofort anwenden, sonst fehlen die Kürzel bis zum nächsten Klick
        self.apply();
        win.set_status(&tf("keys.imported", kw!("n" => n)));
    }

    fn fix_conflicts(&self) {
        let win = self.win();
        win.log("");
        let binds = self.state.borrow().arr("binds").to_vec();
        let n = model::comment_conflicts(&self.hdir(), &binds, &|s| win.log(s));
        win.set_status(&tf("keys.fixed", kw!("n" => n)));
        self.mark();
    }

    // ----- Waybar -----

    fn editor_changed(self: &Rc<Self>, lists: super::bar_editor::Lists) {
        {
            let mut st = self.state.borrow_mut();
            for (side, l) in lists {
                st.insert(side, json!(l));
            }
        }
        self.cfg_set("waybar_preset", json!(""));
        self.refresh_controls();
        self.changed();
    }

    fn set_enabled(self: &Rc<Self>, on: bool) {
        self.state.borrow_mut().insert("enabled".into(), json!(on));
        let win = self.win();
        win.log("");
        if on {
            model::restart_waybar(&|s| win.log(s));
            win.set_status(&t("wb.on"));
        } else {
            model::run(&["pkill", "-x", "waybar"], &|s| win.log(s), true);
            if let Some(hp) = win.hp() {
                if hp.state.borrow().b("autostart_waybar") {
                    {
                        let mut st = hp.state.borrow_mut();
                        st.insert("waybar_managed".into(), json!(true));
                        st.insert("autostart_waybar".into(), json!(false));
                    }
                    hp.refresh_controls();
                    hp.changed();
                    hp.apply();
                }
            }
            win.set_status(&t("wb.off"));
        }
        self.changed();
    }

    fn mods_changed(self: &Rc<Self>) {
        let Some(m) = self.mods.borrow().clone() else { return };
        let new = m.read();
        let diff = new.iter().any(|(k, v)| !py_eq(v, self.state.borrow().val(k)));
        if diff {
            {
                let mut st = self.state.borrow_mut();
                for (k, v) in new {
                    st.insert(k, v);
                }
            }
            self.cfg_set("waybar_preset", json!(""));
            self.changed();
        }
    }

    fn from_hypr(self: &Rc<Self>) {
        let Some(hp) = self.win().hp() else { return };
        let hs = hp.state.borrow().clone();
        {
            let mut st = self.state.borrow_mut();
            match self.kind {
                Kind::Waybar => {
                    let a = model::norm(hs.s("active1"));
                    st.insert("accent".into(), json!(format!("{}ff", &a[..7.min(a.len())])));
                    st.insert("border".into(), json!(model::norm(hs.s("inactive"))));
                }
                _ => {
                    let a = model::norm(hs.s("active1"));
                    let a = &a[..7.min(a.len())];
                    st.insert("accent".into(), json!(format!("{a}ff")));
                    st.insert("border".into(), json!(format!("{a}ff")));
                    st.insert("sel".into(), json!(format!("{a}33")));
                }
            }
        }
        self.cfg_set(&self.kk("preset"), json!(""));
        self.refresh_controls();
        self.changed();
    }

    // ----- Launcher -----

    /// Auf einen anderen Launcher wechseln: dessen eigene Einstellungen laden (beim ersten Mal den aktuellen
    /// Look übernehmen). push = Kürzel und Waybar sofort auf ihn umstellen und anwenden.
    pub fn switch_app(self: &Rc<Self>, app: &str, push: bool) {
        if !launcher::APPS.contains(&app) {
            return;
        }
        let cur = self.state.borrow().clone();
        let cur_app = cur.s("app").to_string();
        let stored = {
            let mut c = self.conf.cfg.borrow_mut();
            let la = c.entry("launcher_apps").or_insert_with(|| json!({}));
            if !la.is_object() {
                *la = json!({});
            }
            let m = la.as_object_mut().unwrap();
            m.insert(cur_app.clone(), Value::Object(cur.clone()));
            m.get(app).and_then(|v| v.as_object()).filter(|o| !o.is_empty()).cloned()
        };
        if app != cur_app {
            let mut ns = launcher::state(Some(stored.as_ref().unwrap_or(&cur)));
            ns.insert("app".into(), json!(app));
            *self.state.borrow_mut() = ns;
            self.cfg_set("launcher_preset", json!(""));
            self.refresh_controls();
            self.changed();
        }
        self.update_info();
        let win = self.win();
        win.launcher_gate_refresh();
        if push {
            if packages::installed(app) {
                self.use_everywhere();
            } else {
                // erst nach der Installation umstellen, sonst öffnet das Kürzel nichts
                *self.pending_push.borrow_mut() = Some(app.to_string());
                win.set_status(&tf("launcher.after_install", kw!("app" => t(&format!("app.{app}")))));
            }
        }
    }

    /// Welchen Launcher starten die Kürzel wirklich? (eigene zuerst, dann hyprland.lua/.conf)
    fn detect_app(&self) -> Option<&'static str> {
        let hp = self.win().hp()?;
        let binds = hp.state.borrow().arr("binds").to_vec();
        for (b, _) in model::launcher_binds(&hp.hdir(), &binds) {
            if let Some(app) = launcher::app_of_cmd(&model::bind_cmd(&b)) {
                return Some(app);
            }
        }
        None
    }

    pub fn autodetect(self: &Rc<Self>) {
        if let Some(app) = self.detect_app() {
            if app != self.state.borrow().s("app") {
                self.switch_app(app, false);
                self.win().set_status(&tf("launcher.detected", kw!("app" => t(&format!("app.{app}")))));
            }
        }
    }

    /// Launcher-Befehl in Waybar und in Kürzeln übernehmen, die einen Launcher starten.
    pub fn use_everywhere(self: &Rc<Self>) {
        let win = self.win();
        let app = self.state.borrow().s("app").to_string();
        let cmd = launcher::command(&app).to_string();
        let (Some(wb), Some(hp)) = (win.wp(), win.hp()) else { return };
        wb.state.borrow_mut().insert("launcher".into(), json!(cmd));
        wb.refresh_controls();
        wb.changed();
        let mut n = 0;
        let binds = hp.state.borrow().arr("binds").to_vec();
        let found = model::launcher_binds(&hp.hdir(), &binds);
        {
            let mut st = hp.state.borrow_mut();
            let mut list = st.arr("binds").to_vec();
            for (b, where_) in &found {
                if model::bind_cmd(b) == cmd {
                    continue;
                }
                if *where_ == "own" {
                    // gleiche Kombination in der eigenen Liste umstellen
                    for x in list.iter_mut().filter_map(|x| x.as_object_mut()) {
                        if x == b {
                            x.insert("action".into(), json!("launcher"));
                            x.insert("arg".into(), json!(app));
                            break;
                        }
                    }
                } else {
                    // Kürzel aus hyprland.lua übernehmen; das Original wird beim Anwenden auskommentiert
                    let mut nb = b.clone();
                    nb.insert("action".into(), json!("launcher"));
                    nb.insert("arg".into(), json!(app));
                    list.push(Value::Object(nb));
                }
                n += 1;
            }
            if found.is_empty() {
                // gar kein Launcher-Kürzel → Super + Leertaste anlegen
                list.push(json!({"mods": ["SUPER"], "key": "space", "action": "launcher", "arg": app, "flags": []}));
                n += 1;
            }
            st.insert("binds".into(), Value::Array(list));
        }
        hp.refresh_controls();
        hp.changed();
        win.log("");
        self.apply(); // Launcher-Config (Suche) schreiben
        let binds = hp.state.borrow().arr("binds").to_vec();
        model::comment_conflicts(&hp.hdir(), &binds, &|s| win.log(s)); // Original-Kürzel aus
        hp.apply(); // Kürzel in Hyprland übernehmen
        wb.apply();
        self.update_info();
        win.set_status(&tf("launcher.used", kw!("cmd" => cmd, "n" => n)));
    }

    fn set_sys_actions(&self, on: bool) {
        let win = self.win();
        self.cfg_set("sys_actions", json!(on));
        self.conf.save_later();
        win.log("");
        sysactions::apply(on, &|s| win.log(s), packages::installed("hyprlock"));
        win.set_status(&if on { t("sys.done_on") } else { t("sys.done_off") });
    }

    fn update_info_launcher(self: &Rc<Self>) {
        let app = self.state.borrow().s("app").to_string();
        let pend = self.pending_push.borrow().clone();
        if pend.as_deref() == Some(app.as_str()) && packages::installed(&app) {
            *self.pending_push.borrow_mut() = None;
            self.use_everywhere(); // Installation fertig → Kürzel und Waybar jetzt umstellen
        }
        let ok = app == "llauncher" || model::which(&app);
        self.info.set_text(&format!("{} · {}", t(&format!("app.{app}")),
            if ok { t("pkg.installed") } else { t("fmt.none") }));
        self.btn_inst_app.set_visible(!ok);
        let mut cap = app.clone();
        if let Some(f) = cap.get_mut(0..1) {
            f.make_ascii_uppercase();
        }
        self.btn_inst_app.set_label(&tf("launcher.install", kw!("app" => cap)));
        let cmd = launcher::command(&app).to_string();
        let mut info = tf("launcher.cmd", kw!("cmd" => cmd));
        if !ok {
            info.push_str("  ·  ");
            info.push_str(&tf("launcher.missing", kw!("app" => app)));
        }
        self.app_info.set_text(&info);
        let Some(hp) = self.win().hp() else { return };
        let binds = hp.state.borrow().arr("binds").to_vec();
        let found = model::launcher_binds(&hp.hdir(), &binds);
        let parts: Vec<String> = found.iter().map(|(b, _)| format!("{} → {}", combo_label(&b.strs("mods"), b.s("key")),
            model::bind_cmd(b).split(' ').next().unwrap_or(""))).collect();
        let wrong = found.iter().any(|(b, _)| model::bind_cmd(b) != cmd);
        self.bound_info.set_text(&if parts.is_empty() { t("launcher.unbound") } else {
            tf("launcher.bound", kw!("list" => parts.join("   ·   ")))
        });
        let good = !wrong && !found.is_empty();
        set_chip(&self.bound_chip, good, &if good { t("launcher.ok") } else { t("launcher.other") });
        if wrong || found.is_empty() {
            self.btn_use.add_css_class("Primary");
        } else {
            self.btn_use.remove_css_class("Primary");
        }
    }
}

impl PresetHost for EditorPage {
    fn kind(&self) -> String {
        self.kind.key().to_string()
    }
    fn conf(&self) -> Conf {
        self.conf.clone()
    }
    fn state(&self) -> Obj {
        self.state.borrow().clone()
    }
    fn sanitize(&self, v: Option<&Obj>) -> Obj {
        self.kind.sanitize(v)
    }
    fn state_texture(&self, s: &Obj) -> gdk::Texture {
        self.state_texture_impl(s)
    }
    fn use_preset(&self, name: &str) {
        if let Some(me) = self.me() {
            me.use_preset_impl(name);
        }
    }
    fn update_applied(&self) {
        self.update_applied_impl();
    }
    fn presets(&self) -> &RefCell<Vec<(String, Rc<PresetCard>)>> {
        &self.presets
    }
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }
}

/// Sicherungsordner für „Bereich zurücksetzen“.
pub fn backup_dir() -> PathBuf {
    let d = util::home().join(".config/LabtosoStudios/backups").join(util::strftime("%Y-%m-%d_%H%M%S"));
    let _ = std::fs::create_dir_all(&d);
    d
}

#[allow(dead_code)]
fn _unused() {
    let _ = draw::Rgba(0.0, 0.0, 0.0, 0.0);
}
