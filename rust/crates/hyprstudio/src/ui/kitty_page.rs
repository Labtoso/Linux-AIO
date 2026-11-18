//! Sonstiges › Kitty: Terminal-Vorschau, Presets und Einstellungen in Unterseiten.
//! Derselbe Editor schreibt in Linux AIO auch Alacritty (anderes `Program`).

use super::draw::{self, Font, HAlign, Rgba, R};
use super::preview::wallpaper;
use super::shell::Conf;
use super::userpresets::{PresetHost, UserPresets, PREFIX};
use super::widgets::{button, chip, flow_grid, hbox, icon_button, scroller, set_button_text, set_chip, vbox,
                     vspacer, Card, ColorButton, Host, PresetCard, Segmented, Switch};
use crate::i18n::t;
use crate::jsonx::{self, Get, Obj};
use crate::model::{self, Log};
use crate::kitty;
use gtk::prelude::*;
use gtk::{cairo, gdk, glib, pango};
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

const DEMO: &[&[(&str, usize)]] = &[
    &[("user", 4), ("@", 7), ("arch ", 4), ("~/projects ", 6), ("❯ ", 5), ("ls", 15)],
    &[("Pictures/  ", 4), ("Music/  ", 4), ("hypr-studio.pyz  ", 2), ("notes.md  ", 15), ("start.sh", 2)],
    &[("user", 4), ("@", 7), ("arch ", 4), ("~/projects ", 6), ("❯ ", 5), ("git status", 15)],
    &[("On branch ", 7), ("main", 2)],
    &[("modified:  ", 1), ("src/gui.py", 3)],
    &[("user", 4), ("@", 7), ("arch ", 4), ("~/projects ", 6), ("❯ ", 5)],
];
const TABS: &[&str] = &["1: zsh", "2: nvim", "3: htop"];

thread_local! {
    static MONO: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

fn is_mono(family: &str) -> bool {
    MONO.with(|m| {
        if m.borrow().is_none() {
            let fm = pangocairo::FontMap::default();
            let v: Vec<String> = fm.list_families().iter().filter(|f| f.is_monospace())
                .map(|f| f.name().to_lowercase()).collect();
            *m.borrow_mut() = Some(v);
        }
        let low = family.to_lowercase();
        low == "monospace" || m.borrow().as_ref().unwrap().contains(&low)
    })
}

fn mono_font(s: &Obj, px: f64) -> Font {
    let mut fam = s.s("font").to_string();
    if !is_mono(&fam) {
        fam = if is_mono("JetBrains Mono") { "JetBrains Mono".into() } else { "monospace".into() };
    }
    Font { family: fam, px: px.round().max(4.0), weight: pango::Weight::Normal }
}

fn c(x: &str) -> Rgba {
    Rgba::hex(x)
}

pub fn paint_terminal(cr: &cairo::Context, r: R, s: &Obj, trail_phase: f64) {
    cr.save().ok();
    draw::rounded(cr, r, 12.0);
    cr.clip();
    let (wall, blur) = wallpaper((r.w as i32).max(1), (r.h as i32).max(1));
    cr.set_source_surface(&wall, r.x, r.y).ok();
    let _ = cr.paint();
    let k = r.w / 900.0;
    let win = r.adj(r.w * 0.07, r.h * 0.08, -r.w * 0.07, -r.h * 0.08);
    // Schatten
    for i in 0..5 {
        let g = (i + 1) as f64 * 3.0 * k;
        draw::fill_rounded(cr, win.adj(-g, -g + 3.0 * k, g, g + 3.0 * k), 10.0 + g, Rgba::rgb8(0, 0, 0, 22));
    }
    cr.save().ok();
    draw::rounded(cr, win, 10.0 * k + 2.0);
    cr.clip();
    let opacity = s.n("opacity");
    if jsonx::truthy(s.val("blur")) && opacity < 1.0 {
        cr.set_source_surface(&blur, r.x, r.y).ok();
        let _ = cr.paint();
    }
    draw::fill_rect(cr, win, c(s.s("bg")).alpha(opacity.clamp(0.0, 1.0)));
    let mut inner = win;
    if s.s("decorations") == "no" {
        let tb = R::new(win.x, win.y, win.w, 26.0 * k + 6.0);
        draw::fill_rect(cr, tb, Rgba::rgb8(20, 22, 30, 235));
        let f = Font::new((11.0 * k + 3.0).floor().max(4.0), pango::Weight::Normal);
        let title = if s.s("_title").is_empty() { "kitty" } else { s.s("_title") };
        draw::text(cr, &f, tb, title, c("#aab4c3"), HAlign::Center, false);
        for (i, col) in ["#e5534b", "#d29922", "#3fb950"].iter().enumerate() {
            let d = 9.0 * k + 2.0;
            draw::fill_ellipse(cr, tb.right() - (14.0 + i as f64 * 18.0) * k - 4.0, tb.cy(), d / 2.0, d / 2.0, c(col));
        }
        inner = R::new(inner.x, tb.bottom(), inner.w, inner.bottom() - tb.bottom());
    }
    let font = mono_font(s, s.n("font_size") * 1.45 * k + 2.0);
    let fh = font.height(cr);
    let asc = {
        let l = font.layout(cr, "Hg");
        l.baseline() as f64 / pango::SCALE as f64
    };
    // Tab-Leiste
    let th = fh + 6.0 * k;
    let style = s.s("tab_style");
    if style != "hidden" {
        let top = s.s("tab_edge") == "top";
        let bar = R::new(inner.x, if top { inner.y } else { inner.bottom() - th }, inner.w, th);
        draw::fill_rect(cr, bar, c(s.s("tab_inactive_bg")));
        let mut x = bar.x;
        for (i, title) in TABS.iter().enumerate() {
            let w = font.advance(cr, title) + 22.0 * k;
            let active = i == 0;
            let bgc = c(if active { s.s("tab_active_bg") } else { s.s("tab_inactive_bg") });
            let fgc = c(if active { s.s("tab_active_fg") } else { s.s("tab_inactive_fg") });
            let rect = R::new(x, bar.y, w, bar.h);
            match style {
                "powerline" => {
                    let tip = bar.h * 0.5;
                    match s.s("tab_powerline") {
                        "round" => draw::fill_rounded(cr, rect.adj(0.0, 0.0, tip * 0.6, 0.0), tip, bgc),
                        pl => {
                            cr.move_to(rect.x, rect.y);
                            cr.line_to(rect.right(), rect.y);
                            if pl == "slanted" {
                                cr.line_to(rect.right() + tip, rect.y);
                                cr.line_to(rect.right(), rect.bottom());
                            } else {
                                cr.line_to(rect.right() + tip, rect.cy());
                                cr.line_to(rect.right(), rect.bottom());
                            }
                            cr.line_to(rect.x, rect.bottom());
                            cr.close_path();
                            bgc.set(cr);
                            let _ = cr.fill();
                        }
                    }
                    x += w + tip;
                }
                "slant" => {
                    let sl = bar.h * 0.45;
                    cr.move_to(rect.x + sl, rect.y);
                    cr.line_to(rect.right() + sl, rect.y);
                    cr.line_to(rect.right(), rect.bottom());
                    cr.line_to(rect.x, rect.bottom());
                    cr.close_path();
                    bgc.set(cr);
                    let _ = cr.fill();
                    x += w + sl;
                }
                _ => {
                    if active {
                        draw::fill_rect(cr, rect, bgc);
                    }
                    if style == "separator" && i > 0 {
                        fgc.set(cr);
                        cr.set_line_width(1.0);
                        cr.move_to(rect.x, rect.y + 4.0);
                        cr.line_to(rect.x, rect.bottom() - 4.0);
                        let _ = cr.stroke();
                    }
                    x += w;
                }
            }
            draw::text(cr, &font, rect, title, fgc, HAlign::Center, false);
        }
        if top {
            inner = R::new(inner.x, bar.bottom(), inner.w, inner.bottom() - bar.bottom());
        } else {
            inner = R::new(inner.x, inner.y, inner.w, bar.y - inner.y);
        }
    }
    // Text
    let pad = s.n("padding") * k;
    let tx = inner.x + pad + 6.0 * k;
    let mut ty = inner.y + pad + 4.0 * k;
    let pal: Vec<Rgba> = s.strs("palette").iter().map(|x| c(x)).collect();
    let palc = |i: usize| pal.get(i).copied().unwrap_or(Rgba(1.0, 1.0, 1.0, 1.0));
    let (mut cur_x, mut cur_y) = (0.0, 0.0);
    for line in DEMO {
        let mut x = tx;
        for (text, ci) in line.iter() {
            let col = if *ci == 15 { c(s.s("fg")) } else { palc(*ci) };
            let l = font.layout(cr, text);
            col.set(cr);
            cr.move_to(x, ty + asc - l.baseline() as f64 / pango::SCALE as f64);
            pangocairo::functions::show_layout(cr, &l);
            x += font.advance(cr, text);
        }
        cur_x = x;
        cur_y = ty;
        ty += fh * 1.15;
    }
    // Farbpalette als Kästchen
    let sw = fh * 0.9;
    for i in 0..16 {
        draw::fill_rect(cr, R::new(tx + (i % 8) as f64 * (sw + 3.0), ty + (i / 8) as f64 * (sw + 3.0) + 4.0, sw, sw),
            palc(i));
    }
    // Cursor + Spur
    let (cw, ch) = (font.advance(cr, "M"), fh);
    let cc = c(s.s("cursor"));
    if jsonx::truthy(s.val("trail")) && trail_phase < 1.0 {
        for j in 0..4 {
            let a = (110.0 * (1.0 - trail_phase) * (1.0 - j as f64 / 4.0)).floor() / 255.0;
            draw::fill_rect(cr, R::new(cur_x - (j + 1) as f64 * cw * 2.2 * (1.0 - trail_phase), cur_y, cw, ch),
                cc.alpha(a));
        }
    }
    match s.s("cursor_shape") {
        "beam" => draw::fill_rect(cr, R::new(cur_x, cur_y, (2.0 * k).max(1.5), ch), cc),
        "underline" => draw::fill_rect(cr, R::new(cur_x, cur_y + ch - (2.0 * k).max(1.5), cw, (2.0 * k).max(1.5)), cc),
        _ => draw::fill_rect(cr, R::new(cur_x, cur_y, cw, ch), cc),
    }
    cr.restore().ok();
    cr.restore().ok();
}

pub fn render_card(s: &Obj) -> gdk::Texture {
    let s = s.clone();
    draw::texture(196, 110, 2.0, move |cr| paint_terminal(cr, R::new(0.0, 0.0, 196.0, 110.0), &s, 1.0))
}

pub struct TerminalPreview {
    pub area: gtk::DrawingArea,
    s: RefCell<Obj>,
    phase: Cell<f64>,
    tick: RefCell<Option<gtk::TickCallbackId>>,
}

impl TerminalPreview {
    pub fn new() -> Rc<TerminalPreview> {
        let area = gtk::DrawingArea::new();
        area.set_hexpand(true);
        let p = Rc::new(TerminalPreview { area, s: RefCell::new(kitty::state(None)), phase: Cell::new(1.0),
            tick: RefCell::new(None) });
        let w = Rc::downgrade(&p);
        p.area.set_draw_func(move |_, cr, wd, ht| {
            let Some(p) = w.upgrade() else { return };
            let r = R::new(0.0, 0.0, wd as f64, ht as f64);
            let (mut tw, mut th) = (r.w, r.w * 9.0 / 16.0);
            if th > r.h {
                th = r.h;
                tw = r.h * 16.0 / 9.0;
            }
            paint_terminal(cr, R::new(r.x + (r.w - tw) / 2.0, r.y, tw, th), &p.s.borrow(), p.phase.get());
        });
        p
    }

    pub fn set_state(&self, s: &Obj) {
        *self.s.borrow_mut() = s.clone();
        self.area.queue_draw();
    }

    pub fn play(self: &Rc<Self>) {
        if !jsonx::truthy(self.s.borrow().val("trail")) {
            return;
        }
        if let Some(id) = self.tick.borrow_mut().take() {
            id.remove();
        }
        let start = Rc::new(Cell::new(None::<i64>));
        let w = Rc::downgrade(self);
        let id = self.area.add_tick_callback(move |_, clock| {
            let Some(p) = w.upgrade() else { return glib::ControlFlow::Break };
            let now = clock.frame_time();
            let t0 = start.get().unwrap_or_else(|| {
                start.set(Some(now));
                now
            });
            let v = ((now - t0) as f64 / 700_000.0).min(1.0);
            p.phase.set(v);
            p.area.queue_draw();
            if v >= 1.0 {
                p.tick.borrow_mut().take();
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
        *self.tick.borrow_mut() = Some(id);
    }
}

/// Was der Editor vom Fenster braucht (Hypr Studio oder Linux AIO).
pub trait TermWin {
    fn conf(&self) -> Conf;
    fn log(&self, text: &str);
    fn set_status(&self, text: &str);
    fn report(&self, ok: bool, info: &str);
    fn hypr_state(&self) -> Option<Obj>;
}

/// Welches Programm der Editor schreibt (Kitty; Linux AIO: Alacritty).
pub struct Program {
    pub cfg: &'static str,
    pub program: &'static str,
    pub make_state: fn(Option<&Obj>) -> Obj,
    pub write: Box<dyn Fn(&Obj, Log) -> (bool, String)>,
    pub restore: Box<dyn Fn(Log) -> bool>,
    pub view: fn(&Obj) -> Obj,
    /// zusätzliche Karten je Unterseite
    pub extra: Option<fn(&Rc<KittyEditor>, &str, &gtk::Box)>,
    /// nach erfolgreichem Anwenden (z. B. eigener Statustext)
    pub after_apply: Option<fn(&KittyEditor)>,
}

pub fn kitty_program() -> Program {
    Program { cfg: "kitty", program: "kitty", make_state: kitty::state,
        write: Box::new(|s, log| kitty::apply(s, &model::config_home(), log)),
        restore: Box::new(|log| kitty::undo(&model::config_home(), log)), view: |s| s.clone(), extra: None,
        after_apply: None }
}

pub const SUBS: &[&str] = &["presets", "look", "font", "cursor", "tabs", "colors", "behavior", "start"];

pub struct KittyEditor {
    pub root: gtk::Box,
    pub prog: Program,
    win: Rc<dyn TermWin>,
    conf: Conf,
    pub state: RefCell<Obj>,
    pub cards: RefCell<Vec<Rc<Card>>>,
    pub sub_cards: RefCell<Vec<(String, Vec<Rc<Card>>)>>,
    extra_refresh: RefCell<Vec<Box<dyn Fn(&Obj)>>>,
    switches: RefCell<Vec<Rc<Switch>>>,
    presets: RefCell<Vec<(String, Rc<PresetCard>)>>,
    up: RefCell<Option<Rc<UserPresets>>>,
    preview: Rc<TerminalPreview>,
    applied: gtk::Label,
    btn_play: gtk::Button,
    btn_undo: gtk::Button,
    btn_apply: gtk::Button,
    sub: Rc<Segmented>,
    stack: gtk::Stack,
    pub btn_from: gtk::Button,
}

thread_local! {
    static EDITORS: RefCell<Vec<Weak<KittyEditor>>> = const { RefCell::new(Vec::new()) };
}

impl Host for KittyEditor {
    fn value(&self, key: &str) -> Value {
        self.state.borrow().val(key).clone()
    }
    fn set_value(&self, key: &str, v: Value) {
        if let Some(me) = self.me() {
            me.set(key, v);
        }
    }
}

impl KittyEditor {
    fn me(&self) -> Option<Rc<KittyEditor>> {
        EDITORS.with(|p| p.borrow().iter().filter_map(|w| w.upgrade()).find(|x| std::ptr::eq(x.as_ref(), self)))
    }

    fn ck(&self, suffix: &str) -> String {
        format!("{}_{}", self.prog.cfg, suffix)
    }

    pub fn new(win: Rc<dyn TermWin>, prog: Program) -> Rc<KittyEditor> {
        let conf = win.conf();
        let state = {
            let c = conf.cfg.borrow();
            (prog.make_state)(c.obj(prog.cfg))
        };
        {
            // „mine“ heißt jetzt „01“
            let mut c = conf.cfg.borrow_mut();
            for k in ["kitty_preset", "kitty_applied_preset"] {
                if c.s(k) == "mine" {
                    c.insert(k.into(), json!("01"));
                }
            }
        }
        let root = vbox(12);
        let top = hbox(14);
        let preview = TerminalPreview::new();
        preview.area.set_content_height(230);
        preview.area.set_size_request(-1, 230);
        top.append(&preview.area);
        let col = vbox(8);
        let applied = chip();
        applied.set_halign(gtk::Align::Start);
        col.append(&applied);
        col.append(&vspacer());
        let btn_play = button("", "");
        let btn_undo = button("", "");
        let btn_apply = button("", "Primary");
        for b in [&btn_play, &btn_undo, &btn_apply] {
            b.set_size_request(170, 40);
            col.append(b);
        }
        top.append(&col);
        top.set_vexpand(false);
        root.append(&top);
        let sub = Segmented::new(SUBS, "ks.");
        root.append(&sub.root);
        let stack = gtk::Stack::new();
        stack.set_vexpand(true);
        stack.set_vhomogeneous(false);
        root.append(&stack);
        let e = Rc::new(KittyEditor { root, prog, win, conf, state: RefCell::new(state), cards: RefCell::new(vec![]),
            sub_cards: RefCell::new(vec![]), extra_refresh: RefCell::new(vec![]), switches: RefCell::new(vec![]),
            presets: RefCell::new(vec![]), up: RefCell::new(None), preview, applied, btn_play, btn_undo, btn_apply, sub,
            stack, btn_from: button("", "") });
        EDITORS.with(|x| x.borrow_mut().push(Rc::downgrade(&e)));
        let w = Rc::downgrade(&e);
        e.btn_play.connect_clicked(move |_| {
            if let Some(e) = w.upgrade() {
                e.preview.play();
            }
        });
        let w = Rc::downgrade(&e);
        e.btn_undo.connect_clicked(move |_| {
            if let Some(e) = w.upgrade() {
                e.undo();
            }
        });
        let w = Rc::downgrade(&e);
        e.btn_apply.connect_clicked(move |_| {
            if let Some(e) = w.upgrade() {
                e.apply();
            }
        });
        let w = Rc::downgrade(&e);
        e.sub.connect(move |k| {
            if let Some(e) = w.upgrade() {
                e.show_sub(&k);
            }
        });
        for key in SUBS {
            let inner = vbox(12);
            inner.add_css_class("ScrollInner");
            inner.set_margin_end(6);
            let before = e.cards.borrow().len();
            e.build(key, &inner);
            if let Some(f) = e.prog.extra {
                f(&e, key, &inner);
            }
            let new = e.cards.borrow()[before..].to_vec();
            e.sub_cards.borrow_mut().push((key.to_string(), new));
            let sa = scroller(&inner);
            e.stack.add_named(&sa, Some(key));
        }
        e.refresh_controls();
        e.preview.set_state(&(e.prog.view)(&e.state.borrow()));
        let s = e.conf.cfg.borrow().s(&e.ck("sub")).to_string();
        e.show_sub(if s.is_empty() { "presets" } else { &s });
        e
    }

    pub fn card(self: &Rc<Self>, layout: &gtk::Box, title: &str) -> Rc<Card> {
        let w: Weak<KittyEditor> = Rc::downgrade(self);
        let c = Card::new(title, w);
        layout.append(&c.root);
        self.cards.borrow_mut().push(c.clone());
        c
    }

    fn build(self: &Rc<Self>, key: &str, lay: &gtk::Box) {
        let sw = |c: &Rc<Card>, k: &str, tip: Option<&str>, lk: &str| {
            let s = c.toggle(k, tip, Some(lk));
            self.switches.borrow_mut().push(s);
        };
        match key {
            "presets" => {
                let up = UserPresets::build(lay);
                let w: Weak<dyn PresetHost> = {
                    let x: Weak<KittyEditor> = Rc::downgrade(self);
                    x
                };
                up.set_host(w);
                *self.up.borrow_mut() = Some(up.clone());
                up.reload();
                let grid = flow_grid();
                let st = self.state.borrow().clone();
                for n in kitty::presets().keys() {
                    if n == "mine" {
                        continue; // alter Name von „01“
                    }
                    let pc = PresetCard::new(n);
                    *pc.tkey.borrow_mut() = format!("kp.{n}");
                    let w = Rc::downgrade(self);
                    pc.connect(move |k| {
                        if let Some(e) = w.upgrade() {
                            e.use_preset_impl(&k);
                        }
                    });
                    pc.set_texture(&render_card(&(self.prog.view)(&kitty::preset(n, Some(&st)))));
                    grid.append(&pc.root);
                    self.presets.borrow_mut().push((n.clone(), pc));
                }
                lay.append(&grid);
            }
            "look" => {
                let c = self.card(lay, "kc.trans");
                c.slider("opacity", 0.2, 1.0, 0.01, "{:.2f}", None, Some("k.opacity"));
                sw(&c, "dynamic_opacity", Some("tip.k.dynamic"), "k.dynamic");
                c.slider("blur", 0.0, 64.0, 1.0, "{:.0f}", Some("tip.k.blur"), Some("k.blur"));
                let c = self.card(lay, "kc.window");
                c.slider("padding", 0.0, 40.0, 1.0, "{:.0f}", None, Some("k.padding"));
                c.choice("decorations", &["no", "titlebar-only", "yes"], "kdec.", None, Some("k.decorations"));
                c.choice("display_server", &["auto", "wayland", "x11"], "kds.", Some("tip.k.display"), Some("k.display"));
            }
            "font" => {
                let c = self.card(lay, "card.font");
                c.font("font", None, Some("font"));
                c.slider("font_size", 6.0, 24.0, 0.5, "{:.1f}", None, Some("font_size"));
                c.choice("ligatures", &["never", "cursor", "always"], "klig.", None, Some("k.ligatures"));
            }
            "cursor" => {
                let c = self.card(lay, "kc.cursor");
                c.choice("cursor_shape", &["block", "beam", "underline"], "kcur.", None, Some("k.cursor_shape"));
                c.slider("blink", 0.0, 2.0, 0.1, "{:.1f} s", Some("tip.k.blink"), Some("k.blink"));
                c.choice("blink_curve", &["linear", "ease-in-out", "ease-in", "ease-out", "step-end"], "kcurve.", None,
                    Some("k.blink_curve"));
                let c = self.card(lay, "kc.anim");
                c.slider("trail", 0.0, 30.0, 1.0, "{:.0f}", Some("tip.k.trail"), Some("k.trail"));
                c.slider("trail_decay_fast", 0.05, 1.0, 0.05, "{:.2f}", None, Some("k.decay_fast"));
                c.slider("trail_decay_slow", 0.05, 2.0, 0.05, "{:.2f}", None, Some("k.decay_slow"));
                sw(&c, "bell_flash", None, "k.bell_flash");
            }
            "tabs" => {
                let c = self.card(lay, "kc.tabs");
                c.choice("tab_edge", &["top", "bottom"], "pos.", None, Some("position"));
                c.choice("tab_style", &["powerline", "slant", "fade", "separator", "hidden"], "ktab.", None,
                    Some("k.tab_style"));
                c.choice("tab_powerline", &["slanted", "angled", "round"], "kpl.", None, Some("k.tab_powerline"));
                c.text("tab_title", None, 220, Some("k.tab_title"));
            }
            "colors" => {
                let c = self.card(lay, "card.colors");
                for k in ["bg", "fg", "cursor", "sel_bg", "sel_fg", "url", "tab_active_bg", "tab_active_fg",
                    "tab_inactive_bg", "tab_inactive_fg"] {
                    c.color(k, Some(&format!("k.{k}")), None);
                }
                let pc = self.card(lay, "kc.palette");
                let grid = gtk::Grid::new();
                grid.set_row_spacing(8);
                grid.set_column_spacing(8);
                let mut btns = vec![];
                for i in 0..16 {
                    let b = ColorButton::new();
                    b.area.set_content_width(52);
                    b.area.set_content_height(30);
                    let w = Rc::downgrade(self);
                    b.connect(move |col| {
                        if let Some(e) = w.upgrade() {
                            e.pal(i, &col);
                        }
                    });
                    grid.attach(&b.area, (i % 8) as i32, (i / 8) as i32, 1, 1);
                    btns.push(b);
                }
                pc.append(&grid);
                self.extra_refresh.borrow_mut().push(Box::new(move |s| {
                    let pal = s.strs("palette");
                    for (i, b) in btns.iter().enumerate() {
                        if let Some(c) = pal.get(i) {
                            b.set_color(c);
                        }
                    }
                }));
                self.btn_from.set_halign(gtk::Align::Start);
                let w = Rc::downgrade(self);
                self.btn_from.connect_clicked(move |_| {
                    if let Some(e) = w.upgrade() {
                        e.from_hypr();
                    }
                });
                c.append(&self.btn_from);
            }
            "behavior" => {
                let c = self.card(lay, "kc.behavior");
                c.slider("scrollback", 1000.0, 100000.0, 1000.0, "{:.0f}", None, Some("k.scrollback"));
                c.choice("url_style", &["curly", "straight", "double", "dotted", "dashed", "none"], "kurl.", None,
                    Some("k.url_style"));
                sw(&c, "copy_on_select", None, "k.copy");
                sw(&c, "audio_bell", None, "k.audio_bell");
                c.choice("confirm_close", &["0", "1", "-1"], "kconf.", None, Some("k.confirm"));
            }
            "start" => {
                let c = self.card(lay, "kc.start");
                sw(&c, "fastfetch", Some("tip.k.fastfetch"), "k.fastfetch");
                c.lines("start_cmds", Some("tip.k.start_cmds"));
            }
            _ => {}
        }
    }

    // ----- Zustand -----

    pub fn set(self: &Rc<Self>, key: &str, value: Value) {
        if self.state.borrow().get(key).map(|v| super::editor::py_eq(v, &value)).unwrap_or(false) {
            return;
        }
        self.state.borrow_mut().insert(key.to_string(), value);
        if kitty::LOOK.contains(&key) {
            self.conf.cfg.borrow_mut().insert(self.ck("preset"), json!(""));
        }
        self.changed();
        if ["trail", "trail_decay_fast", "trail_decay_slow"].contains(&key) {
            self.preview.play();
        }
    }

    fn pal(self: &Rc<Self>, i: usize, col: &str) {
        let mut pal: Vec<Value> = self.state.borrow().arr("palette").to_vec();
        if i < pal.len() {
            pal[i] = json!(col.chars().take(7).collect::<String>());
        }
        self.set("palette", Value::Array(pal));
    }

    pub fn changed(&self) {
        let st = self.state.borrow().clone();
        self.conf.cfg.borrow_mut().insert(self.prog.cfg.to_string(), Value::Object(st.clone()));
        self.conf.save_later();
        self.preview.set_state(&(self.prog.view)(&st));
        self.update_applied_impl();
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

    pub fn use_preset_impl(self: &Rc<Self>, name: &str) {
        let ns = if name.starts_with(PREFIX) {
            self.up.borrow().as_ref().map(|u| u.user_state(name)).unwrap_or_default()
        } else {
            kitty::preset(name, Some(&self.state.borrow()))
        };
        *self.state.borrow_mut() = ns;
        self.conf.cfg.borrow_mut().insert(self.ck("preset"), json!(name));
        self.refresh_controls();
        self.changed();
        self.preview.play();
    }

    fn from_hypr(&self) {
        let Some(hs) = self.win.hypr_state() else { return };
        let a = model::norm(hs.s("active1"));
        let a = a[..7.min(a.len())].to_string();
        {
            let mut st = self.state.borrow_mut();
            for k in ["sel_bg", "tab_active_bg", "cursor"] {
                st.insert(k.into(), json!(a));
            }
        }
        self.conf.cfg.borrow_mut().insert(self.ck("preset"), json!(""));
        self.refresh_controls();
        self.changed();
    }

    pub fn current_sub(&self) -> String {
        let s = self.conf.cfg.borrow().s(&self.ck("sub")).to_string();
        if s.is_empty() { "presets".into() } else { s }
    }

    pub fn show_sub(&self, key: &str) {
        let key = if SUBS.contains(&key) { key } else { "presets" };
        self.sub.select(key);
        self.stack.set_visible_child_name(key);
        self.conf.cfg.borrow_mut().insert(self.ck("sub"), json!(key));
        self.conf.save_later();
    }

    pub fn sig(&self) -> String {
        jsonx::dumps_sorted(&Value::Object(self.state.borrow().clone()))
    }

    fn update_applied_impl(&self) {
        let (ok, applied, sel) = {
            let c = self.conf.cfg.borrow();
            let ok = c.s(&self.ck("applied_sig")) == self.sig();
            (ok, if ok { c.s(&self.ck("applied_preset")).to_string() } else { String::new() },
                c.s(&self.ck("preset")).to_string())
        };
        set_chip(&self.applied, ok, &if ok { t("applied") } else { t("not_applied") });
        for (n, pc) in self.presets.borrow().iter() {
            pc.refresh(*n == sel, !applied.is_empty() && *n == applied);
        }
    }

    pub fn apply(&self) {
        let win = self.win.clone();
        win.log("");
        if !model::which(self.prog.program) {
            win.log(&t("k.missing"));
        }
        let st = self.state.borrow().clone();
        let (ok, info) = (self.prog.write)(&st, &|s| win.log(s));
        win.report(ok, &info);
        if ok {
            let pre = {
                let mut c = self.conf.cfg.borrow_mut();
                c.insert(self.ck("applied_sig"), json!(self.sig()));
                let p = c.s(&self.ck("preset")).to_string();
                c.insert(self.ck("applied_preset"), json!(p));
                p
            };
            self.conf.save_later();
            let name = self.presets.borrow().iter().find(|(k, _)| *k == pre).map(|(_, pc)| pc.name.text().to_string());
            win.set_status(&format!("{}{} · {}", t("applied"), name.map(|n| format!(": {n}")).unwrap_or_default(),
                t("k.reload")));
            self.update_applied_impl();
            if let Some(f) = self.prog.after_apply {
                f(self);
            }
        }
    }

    fn undo(&self) {
        let win = self.win.clone();
        win.log("");
        let ok = (self.prog.restore)(&|s| win.log(s));
        win.set_status(&if ok { t("undone") } else { t("nothing") });
        if ok {
            self.conf.cfg.borrow_mut().insert(self.ck("applied_sig"), json!(""));
            self.update_applied_impl();
        }
    }

    pub fn reset_state(&self) {
        *self.state.borrow_mut() = (self.prog.make_state)(None);
        {
            let mut c = self.conf.cfg.borrow_mut();
            for k in ["preset", "applied_sig", "applied_preset"] {
                c.insert(self.ck(k), json!(""));
            }
        }
        self.refresh_controls();
        self.changed();
    }

    /// Nur die Werte einer Unterseite auf Standard (Linux AIO: Zurücksetzen-Knopf).
    pub fn reset_keys(&self, sub: &str) -> Vec<String> {
        let fresh = (self.prog.make_state)(None);
        let mut keys: Vec<String> = self.sub_cards.borrow().iter().filter(|(k, _)| k == sub)
            .flat_map(|(_, cs)| cs.iter().flat_map(|c| c.keys())).filter(|k| fresh.contains_key(k)).collect();
        if keys.is_empty() {
            keys = fresh.keys().cloned().collect();
        }
        {
            let mut st = self.state.borrow_mut();
            for k in &keys {
                st.insert(k.clone(), fresh[k].clone());
            }
        }
        self.refresh_controls();
        self.changed();
        keys
    }

    pub fn retranslate(&self) {
        self.sub.retranslate();
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
        self.btn_play.set_label(&t("play"));
        self.btn_undo.set_label(&t("undo"));
        self.btn_apply.set_label(&t("apply"));
        self.btn_from.set_label(&t("from_hypr"));
        self.update_applied_impl();
    }

    pub fn apply_theme(&self, _accent: &str) {
        for sw in self.switches.borrow().iter() {
            sw.area.queue_draw();
        }
        self.update_applied_impl();
    }
}

impl PresetHost for KittyEditor {
    fn kind(&self) -> String {
        "kitty".into()
    }
    fn conf(&self) -> Conf {
        self.conf.clone()
    }
    fn state(&self) -> Obj {
        self.state.borrow().clone()
    }
    fn sanitize(&self, v: Option<&Obj>) -> Obj {
        (self.prog.make_state)(v)
    }
    fn state_texture(&self, s: &Obj) -> gdk::Texture {
        render_card(&(self.prog.view)(s))
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

impl super::gate::GateContent for KittyEditor {
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }
}
