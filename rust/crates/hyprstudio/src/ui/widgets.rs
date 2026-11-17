//! Bausteine: Segmente, Schalter, Farbfeld, Karten mit Zeilen, Preset-Karten, Raster, Seitenleisten-Kategorien,
//! Auswahl mit Suche, Schriftwahl, Statuspunkt.

use super::draw::{self, Rgba, R};
use super::shell::{self, shell};
use super::theme;
use crate::i18n::{t, tf};
use crate::jsonx::{self, Get, Obj};
use crate::{fonts, kw};
use gtk::prelude::*;
use gtk::{gdk, glib, pango};
use serde_json::Value;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

pub type Cb<T> = RefCell<Option<Box<dyn Fn(T)>>>;

pub fn cursor_hand<W: IsA<gtk::Widget>>(w: &W) {
    w.set_cursor_from_name(Some("pointer"));
}

pub fn label(text: &str, class: &str) -> gtk::Label {
    let l = gtk::Label::new(Some(text));
    l.set_xalign(0.0);
    if !class.is_empty() {
        l.add_css_class(class);
    }
    l
}

pub fn button(text: &str, class: &str) -> gtk::Button {
    let b = gtk::Button::with_label(text);
    if !class.is_empty() {
        b.add_css_class(class);
    }
    cursor_hand(&b);
    b
}

/// Knopf mit Symbol + Text (wie QPushButton mit Icon).
pub fn icon_button(kind: &str, color: &str, size: i32, text: &str, class: &str) -> gtk::Button {
    let b = gtk::Button::new();
    let bx = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    bx.set_halign(gtk::Align::Center);
    bx.append(&draw::icon_image(kind, color, size));
    let l = gtk::Label::new(Some(text));
    bx.append(&l);
    b.set_child(Some(&bx));
    if !class.is_empty() {
        b.add_css_class(class);
    }
    cursor_hand(&b);
    b
}

/// Text eines Knopfs setzen – auch wenn er ein Symbol + Text enthält.
pub fn set_button_text(b: &gtk::Button, text: &str) {
    if let Some(child) = b.child() {
        if let Ok(l) = child.clone().downcast::<gtk::Label>() {
            l.set_text(text);
            return;
        }
        if let Ok(bx) = child.downcast::<gtk::Box>() {
            let mut c = bx.first_child();
            while let Some(w) = c {
                if let Ok(l) = w.clone().downcast::<gtk::Label>() {
                    l.set_text(text);
                    return;
                }
                c = w.next_sibling();
            }
        }
    }
    b.set_label(text);
}

/// Werkzeugknopf nur mit Symbol (Fensterknöpfe, Zurücksetzen).
pub fn tool_button(kind: &str, color: &str, size: i32, class: &str) -> gtk::Button {
    let b = gtk::Button::new();
    b.set_child(Some(&draw::icon_image(kind, color, size)));
    b.add_css_class("Tool");
    if !class.is_empty() {
        b.add_css_class(class);
    }
    cursor_hand(&b);
    b
}

pub fn set_tool_icon(b: &gtk::Button, kind: &str, color: &str, size: i32) {
    b.set_child(Some(&draw::icon_image(kind, color, size)));
}

pub fn hbox(spacing: i32) -> gtk::Box {
    gtk::Box::new(gtk::Orientation::Horizontal, spacing)
}

pub fn vbox(spacing: i32) -> gtk::Box {
    gtk::Box::new(gtk::Orientation::Vertical, spacing)
}

pub fn spacer() -> gtk::Box {
    let b = hbox(0);
    b.set_hexpand(true);
    b
}

pub fn vspacer() -> gtk::Box {
    let b = vbox(0);
    b.set_vexpand(true);
    b
}

pub fn margins<W: IsA<gtk::Widget>>(w: &W, l: i32, t: i32, r: i32, b: i32) {
    w.set_margin_start(l);
    w.set_margin_top(t);
    w.set_margin_end(r);
    w.set_margin_bottom(b);
}

/// Alle Kinder eines Containers entfernen.
pub fn clear_box(b: &gtk::Box) {
    while let Some(c) = b.first_child() {
        b.remove(&c);
    }
}

/// Status als schlichter Text mit farbigem Punkt (grün = ok, gelb = Hinweis).
pub fn set_chip(l: &gtk::Label, ok: bool, text: &str) {
    let col = if ok { theme::OK } else { theme::WARN };
    l.set_markup(&format!("<span foreground=\"{}\">●</span>  {}", col, glib::markup_escape_text(text)));
    l.add_css_class("chip");
}

pub fn chip() -> gtk::Label {
    let l = gtk::Label::new(None);
    l.set_xalign(0.0);
    l.add_css_class("chip");
    l
}

// ---------- Segmente ----------

pub struct Segmented {
    pub root: gtk::Box,
    pub btns: Vec<(String, gtk::ToggleButton)>,
    prefix: String,
    on_change: Cb<String>,
}

impl Segmented {
    pub fn new(keys: &[&str], prefix: &str) -> Rc<Segmented> {
        let root = hbox(2);
        root.add_css_class("SegBox");
        root.set_halign(gtk::Align::Start);
        let mut btns = vec![];
        let mut first: Option<gtk::ToggleButton> = None;
        for k in keys {
            let b = gtk::ToggleButton::new();
            b.add_css_class("Seg");
            cursor_hand(&b);
            if let Some(f) = &first {
                b.set_group(Some(f));
            } else {
                first = Some(b.clone());
            }
            root.append(&b);
            btns.push((k.to_string(), b));
        }
        let s = Rc::new(Segmented { root, btns, prefix: prefix.into(), on_change: RefCell::new(None) });
        for (k, b) in &s.btns {
            let w = Rc::downgrade(&s);
            let k = k.clone();
            b.connect_clicked(move |b| {
                if !b.is_active() {
                    b.set_active(true);
                }
                if let Some(s) = w.upgrade() {
                    if let Some(f) = s.on_change.borrow().as_ref() {
                        f(k.clone());
                    }
                }
            });
        }
        s.retranslate();
        s
    }

    pub fn connect(&self, f: impl Fn(String) + 'static) {
        *self.on_change.borrow_mut() = Some(Box::new(f));
    }

    pub fn select(&self, key: &str) {
        for (k, b) in &self.btns {
            if k == key {
                b.set_active(true);
            }
        }
    }

    pub fn btn(&self, key: &str) -> Option<&gtk::ToggleButton> {
        self.btns.iter().find(|b| b.0 == key).map(|b| &b.1)
    }

    pub fn retranslate(&self) {
        for (k, b) in &self.btns {
            b.set_label(&t(&format!("{}{}", self.prefix, k)));
        }
    }
}

// ---------- Schalter ----------

pub struct Switch {
    pub area: gtk::DrawingArea,
    on: Cell<bool>,
    pos: Cell<f64>,
    pub accent: RefCell<String>,
    on_toggle: Cb<bool>,
    tick: RefCell<Option<gtk::TickCallbackId>>,
}

impl Switch {
    pub fn new() -> Rc<Switch> {
        let area = gtk::DrawingArea::new();
        area.set_content_width(42);
        area.set_content_height(24);
        area.set_halign(gtk::Align::Start);
        area.set_valign(gtk::Align::Center);
        cursor_hand(&area);
        let accent = if shell::has_shell() { shell().accent() } else { theme::accent_of("tokyo") };
        let s = Rc::new(Switch { area, on: Cell::new(false), pos: Cell::new(0.0), accent: RefCell::new(accent),
            on_toggle: RefCell::new(None), tick: RefCell::new(None) });
        let w = Rc::downgrade(&s);
        s.area.set_draw_func(move |_, cr, wd, ht| {
            if let Some(s) = w.upgrade() {
                s.paint(cr, wd as f64, ht as f64);
            }
        });
        let click = gtk::GestureClick::new();
        let w = Rc::downgrade(&s);
        click.connect_released(move |_, _, _, _| {
            if let Some(s) = w.upgrade() {
                if s.area.is_sensitive() {
                    s.set_checked(!s.on.get());
                }
            }
        });
        s.area.add_controller(click);
        SWITCHES.with(|l| l.borrow_mut().push(Rc::downgrade(&s)));
        s
    }

    pub fn is_on(&self) -> bool {
        self.on.get()
    }

    pub fn connect(&self, f: impl Fn(bool) + 'static) {
        *self.on_toggle.borrow_mut() = Some(Box::new(f));
    }

    /// Wie ein Klick: umschalten, animieren, melden.
    pub fn set_checked(self: &Rc<Self>, on: bool) {
        if on == self.on.get() {
            return;
        }
        self.on.set(on);
        self.animate();
        if let Some(f) = self.on_toggle.borrow().as_ref() {
            f(on);
        }
    }

    /// Ohne Animation und ohne Meldung setzen.
    pub fn set_now(&self, on: bool) {
        if let Some(id) = self.tick.borrow_mut().take() {
            id.remove();
        }
        self.on.set(on);
        self.pos.set(if on { 1.0 } else { 0.0 });
        self.area.queue_draw();
    }

    fn animate(self: &Rc<Self>) {
        if let Some(id) = self.tick.borrow_mut().take() {
            id.remove();
        }
        let start = self.pos.get();
        let end = if self.on.get() { 1.0 } else { 0.0 };
        let t0 = Rc::new(Cell::new(None::<i64>));
        let w = Rc::downgrade(self);
        let id = self.area.add_tick_callback(move |_, clock| {
            let Some(s) = w.upgrade() else { return glib::ControlFlow::Break };
            let now = clock.frame_time();
            let t = t0.get().unwrap_or_else(|| {
                t0.set(Some(now));
                now
            });
            let f = ((now - t) as f64 / 140_000.0).min(1.0);
            s.pos.set(start + (end - start) * f);
            s.area.queue_draw();
            if f >= 1.0 {
                s.tick.borrow_mut().take();
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
        *self.tick.borrow_mut() = Some(id);
    }

    fn paint(&self, cr: &gtk::cairo::Context, w: f64, h: f64) {
        let r = R::new(1.0, 1.0, w - 2.0, h - 2.0);
        let pos = self.pos.get();
        let off = Rgba::hex(theme::SURFACE2);
        let on = Rgba::hex(&self.accent.borrow());
        let c = Rgba(off.0 + (on.0 - off.0) * pos, off.1 + (on.1 - off.1) * pos, off.2 + (on.2 - off.2) * pos, 1.0);
        draw::rounded(cr, r.adj(0.5, 0.5, -0.5, -0.5), r.h / 2.0);
        c.set(cr);
        let _ = cr.fill_preserve();
        if pos < 0.5 { Rgba::hex(theme::BORDER) } else { c }.set(cr);
        cr.set_line_width(1.0);
        let _ = cr.stroke();
        let d = r.h - 6.0;
        let x = r.x + 3.0 + (r.w - d - 6.0) * pos;
        let knob = if pos >= 0.5 { theme::on_accent(&self.accent.borrow()) } else { "#ffffff" };
        draw::fill_ellipse(cr, x + d / 2.0, r.y + 3.0 + d / 2.0, d / 2.0, d / 2.0, Rgba::hex(knob));
        if !self.area.is_sensitive() {
            draw::fill_rounded(cr, r, r.h / 2.0, Rgba::hex(theme::BG).alpha(0.5));
        }
    }
}

thread_local! {
    static SWITCHES: RefCell<Vec<Weak<Switch>>> = const { RefCell::new(Vec::new()) };
}

/// Alle Schalter in der neuen Akzentfarbe.
pub fn recolor_switches(accent: &str) {
    SWITCHES.with(|l| {
        let mut l = l.borrow_mut();
        l.retain(|w| w.strong_count() > 0);
        for s in l.iter().filter_map(|w| w.upgrade()) {
            *s.accent.borrow_mut() = accent.to_string();
            s.area.queue_draw();
        }
    });
}

// ---------- Farbfeld ----------

pub fn checker(cr: &gtk::cairo::Context, r: R, s: f64) {
    draw::fill_rect(cr, r, Rgba::hex("#d0d6de"));
    let mut y = r.y;
    let mut row = 0;
    while y < r.bottom() {
        let mut x = r.x + if row % 2 == 1 { s } else { 0.0 };
        while x < r.right() {
            let cell = R::new(x, y, s.min(r.right() - x), s.min(r.bottom() - y));
            draw::fill_rect(cr, cell, Rgba::hex("#9aa4b1"));
            x += 2.0 * s;
        }
        y += s;
        row += 1;
    }
}

pub struct ColorButton {
    pub area: gtk::DrawingArea,
    pub color: RefCell<String>,
    hover: Cell<bool>,
    on_change: Cb<String>,
}

impl ColorButton {
    pub fn new() -> Rc<ColorButton> {
        let area = gtk::DrawingArea::new();
        area.set_content_width(64);
        area.set_content_height(30);
        area.set_halign(gtk::Align::Start);
        cursor_hand(&area);
        let c = Rc::new(ColorButton { area, color: RefCell::new("#3d8bfdff".into()), hover: Cell::new(false),
            on_change: RefCell::new(None) });
        let w = Rc::downgrade(&c);
        c.area.set_draw_func(move |_, cr, wd, ht| {
            if let Some(c) = w.upgrade() {
                let r = R::new(1.0, 1.0, wd as f64 - 2.0, ht as f64 - 2.0);
                cr.save().ok();
                draw::rounded(cr, r, 9.0);
                cr.clip();
                checker(cr, r, 5.0);
                draw::fill_rect(cr, r, Rgba::hex(&c.color.borrow()));
                cr.restore().ok();
                draw::rounded(cr, r.adj(0.5, 0.5, -0.5, -0.5), 9.0);
                Rgba::hex(if c.hover.get() { theme::TEXT } else { theme::BORDER }).set(cr);
                cr.set_line_width(1.0);
                let _ = cr.stroke();
            }
        });
        let motion = gtk::EventControllerMotion::new();
        let w = Rc::downgrade(&c);
        motion.connect_enter(move |_, _, _| {
            if let Some(c) = w.upgrade() {
                c.hover.set(true);
                c.area.queue_draw();
            }
        });
        let w = Rc::downgrade(&c);
        motion.connect_leave(move |_| {
            if let Some(c) = w.upgrade() {
                c.hover.set(false);
                c.area.queue_draw();
            }
        });
        c.area.add_controller(motion);
        let click = gtk::GestureClick::new();
        let w = Rc::downgrade(&c);
        click.connect_released(move |_, _, _, _| {
            if let Some(c) = w.upgrade() {
                c.pick();
            }
        });
        c.area.add_controller(click);
        c
    }

    pub fn connect(&self, f: impl Fn(String) + 'static) {
        *self.on_change.borrow_mut() = Some(Box::new(f));
    }

    pub fn set_color(&self, c: &str) {
        let n = crate::model::norm(c);
        self.area.set_tooltip_text(Some(&n));
        *self.color.borrow_mut() = n;
        self.area.queue_draw();
    }

    fn emit(&self) {
        let c = self.color.borrow().clone();
        if let Some(f) = self.on_change.borrow().as_ref() {
            f(c);
        }
    }

    pub fn pick(self: &Rc<Self>) {
        let old = self.color.borrow().clone();
        let w = Rc::downgrade(self);
        let live = move |c: String| {
            if let Some(s) = w.upgrade() {
                s.set_color(&c);
                s.emit();
            }
        };
        let w2 = Rc::downgrade(self);
        let parent = shell::toplevel_of(&self.area);
        let start = old.clone();
        super::dialogs::pick_color(parent.as_ref(), &start, &t("color.pick"), Box::new(live), Box::new(move |res| {
            if let Some(s) = w2.upgrade() {
                match res {
                    Some(c) => s.set_color(&c),
                    None => s.set_color(&old),
                }
                s.emit();
            }
        }));
    }
}

// ---------- Karten mit Zeilen ----------

/// Wer die Werte einer Karte hält (Editor-Seite, Kitty, Sperrbildschirm …).
pub trait Host {
    fn value(&self, key: &str) -> Value;
    fn set_value(&self, key: &str, v: Value);
}

pub struct Row {
    pub label: gtk::Label,
    pub tkey: String,
    pub tip: Option<String>,
    pub skey: String,
    pub refresh: Option<Box<dyn Fn(&Obj)>>,
    pub widgets: Vec<gtk::Widget>,
}

pub struct Card {
    pub root: gtk::Box,
    pub title: gtk::Label,
    pub title_key: String,
    pub grid: gtk::Grid,
    pub rows: RefCell<Vec<Row>>,
    pub segs: RefCell<Vec<Rc<Segmented>>>,
    pub switches: RefCell<Vec<Rc<Switch>>>,
    pub fonts: RefCell<Vec<Rc<FontCombo>>>,
    host: Weak<dyn Host>,
    loading: Rc<Cell<bool>>,
}

/// Python-Formate wie "{:.0f}", "{:+.2f}", "{:.1f}×", "{:.0f} px".
pub fn fmt_num(fmt: &str, v: f64) -> String {
    let Some(a) = fmt.find('{') else { return fmt.into() };
    let Some(b) = fmt[a..].find('}').map(|x| x + a) else { return fmt.into() };
    let spec = &fmt[a + 1..b];
    let spec = spec.strip_prefix(':').unwrap_or(spec);
    let plus = spec.starts_with('+');
    let spec = spec.trim_start_matches('+');
    let digits: usize = spec.strip_prefix('.').and_then(|s| s.trim_end_matches('f').parse().ok()).unwrap_or(0);
    let mut s = format!("{:.*}", digits, v);
    if s.starts_with('-') && s[1..].chars().all(|c| c == '0' || c == '.') {
        s = s[1..].to_string();
    }
    if plus && !s.starts_with('-') {
        s = format!("+{}", s);
    }
    format!("{}{}{}", &fmt[..a], s, &fmt[b + 1..])
}

impl Card {
    pub fn new(title_key: &str, host: Weak<dyn Host>) -> Rc<Card> {
        let root = vbox(10);
        root.add_css_class("Card");
        margins(&root, 0, 0, 0, 0);
        let inner_pad = gtk::Box::new(gtk::Orientation::Vertical, 10);
        let title = label("", "CardTitle");
        let grid = gtk::Grid::new();
        grid.set_column_spacing(14);
        grid.set_row_spacing(10);
        root.append(&title);
        root.append(&grid);
        drop(inner_pad);
        root.set_margin_start(0);
        // Innenabstand 18/14/18/16 über die Kinder
        title.set_margin_start(18);
        title.set_margin_end(18);
        title.set_margin_top(14);
        grid.set_margin_start(18);
        grid.set_margin_end(18);
        grid.set_margin_bottom(16);
        let c = Rc::new(Card { root, title, title_key: title_key.into(), grid, rows: RefCell::new(vec![]),
            segs: RefCell::new(vec![]), switches: RefCell::new(vec![]), fonts: RefCell::new(vec![]), host,
            loading: Rc::new(Cell::new(false)) });
        c.retranslate();
        c
    }

    /// Zusätzliches Widget unten in die Karte (wie card.layout().addWidget).
    pub fn append<W: IsA<gtk::Widget>>(&self, w: &W) {
        w.set_margin_start(18);
        w.set_margin_end(18);
        self.root.remove(&self.grid);
        self.root.append(&self.grid);
        // Grid behält den unteren Abstand, das neue Widget kommt darunter
        self.grid.set_margin_bottom(if self.rows.borrow().is_empty() { 0 } else { 6 });
        self.root.append(w);
        w.set_margin_bottom(16);
        self.fix_bottom_margins();
    }

    fn fix_bottom_margins(&self) {
        // nur das letzte Kind bekommt den unteren Innenabstand
        let mut c = self.root.first_child();
        let last = self.root.last_child();
        while let Some(w) = c {
            if w != self.title.clone().upcast::<gtk::Widget>() {
                w.set_margin_bottom(if Some(&w) == last.as_ref() { 16 } else { 0 });
            }
            c = w.next_sibling();
        }
    }

    pub fn host(&self) -> Option<Rc<dyn Host>> {
        self.host.upgrade()
    }

    pub fn add_row(&self, key: &str, widget: &gtk::Widget, value: Option<&gtk::Widget>, tip: Option<&str>,
        refresh: Option<Box<dyn Fn(&Obj)>>, label_key: Option<&str>) -> gtk::Label {
        let r = self.rows.borrow().len() as i32;
        let lb = label("", "RowLabel");
        lb.set_size_request(130, -1);
        lb.set_valign(gtk::Align::Center);
        self.grid.attach(&lb, 0, r, 1, 1);
        let mut widgets = vec![widget.clone()];
        match value {
            None => {
                widget.set_halign(gtk::Align::Start);
                self.grid.attach(widget, 1, r, 2, 1);
            }
            Some(v) => {
                widget.set_hexpand(true);
                self.grid.attach(widget, 1, r, 1, 1);
                self.grid.attach(v, 2, r, 1, 1);
                widgets.push(v.clone());
            }
        }
        self.rows.borrow_mut().push(Row { label: lb.clone(), tkey: label_key.unwrap_or(key).into(),
            tip: tip.map(String::from), skey: key.into(), refresh, widgets });
        self.retranslate_row(self.rows.borrow().len() - 1);
        lb
    }

    fn retranslate_row(&self, i: usize) {
        let rows = self.rows.borrow();
        let r = &rows[i];
        if r.tkey.is_empty() {
            return;
        }
        r.label.set_text(&t(&r.tkey));
        if let Some(tip) = &r.tip {
            r.label.set_tooltip_text(Some(&t(tip)));
        }
    }

    pub fn set_row_visible(&self, key: &str, on: bool) {
        for r in self.rows.borrow().iter().filter(|r| r.skey == key) {
            r.label.set_visible(on);
            for w in &r.widgets {
                w.set_visible(on);
            }
        }
    }

    fn set(&self, key: &str, v: Value) {
        if self.loading.get() {
            return;
        }
        if let Some(h) = self.host.upgrade() {
            h.set_value(key, v);
        }
    }

    pub fn slider(self: &Rc<Self>, key: &str, lo: f64, hi: f64, step: f64, fmt: &str, tip: Option<&str>,
        label_key: Option<&str>) -> gtk::Scale {
        let adj = gtk::Adjustment::new(0.0, (lo / step).round(), (hi / step).round(), 1.0, 1.0, 0.0);
        let sl = gtk::Scale::new(gtk::Orientation::Horizontal, Some(&adj));
        sl.set_draw_value(false);
        sl.set_digits(0);
        sl.set_round_digits(0);
        sl.set_hexpand(true);
        cursor_hand(&sl);
        no_wheel(&sl);
        let val = label("", "Value");
        val.set_xalign(1.0);
        val.set_size_request(44, -1);
        let is_float = self.host.upgrade().map(|h| jsonx::is_float(&h.value(key))).unwrap_or(false);
        let w = Rc::downgrade(self);
        let k = key.to_string();
        let f = fmt.to_string();
        let v2 = val.clone();
        sl.connect_value_changed(move |s| {
            let Some(c) = w.upgrade() else { return };
            let i = s.value().round();
            let v = i * step;
            let jv = if is_float {
                let r = (v * 10000.0).round() / 10000.0;
                jsonx::f(r)
            } else {
                Value::from(v.round() as i64)
            };
            v2.set_text(&fmt_num(&f, jv.as_f64().unwrap_or(0.0)));
            c.set(&k, jv);
        });
        let k = key.to_string();
        let f = fmt.to_string();
        let sl2 = sl.clone();
        let v3 = val.clone();
        let loading = self.loading.clone();
        let refresh = Box::new(move |s: &Obj| {
            let x = s.n(&k);
            loading.set(true);
            sl2.set_value((x / step).round());
            loading.set(false);
            v3.set_text(&fmt_num(&f, x));
        });
        self.add_row(key, sl.upcast_ref(), Some(val.upcast_ref()), tip, Some(refresh), label_key);
        sl
    }

    pub fn toggle(self: &Rc<Self>, key: &str, tip: Option<&str>, label_key: Option<&str>) -> Rc<Switch> {
        let sw = Switch::new();
        let w = Rc::downgrade(self);
        let k = key.to_string();
        sw.connect(move |on| {
            if let Some(c) = w.upgrade() {
                c.set(&k, Value::Bool(on));
            }
        });
        let k = key.to_string();
        let s2 = sw.clone();
        self.add_row(key, sw.area.upcast_ref(), None, tip, Some(Box::new(move |s: &Obj| s2.set_now(s.b(&k)))), label_key);
        self.switches.borrow_mut().push(sw.clone());
        sw
    }

    pub fn color(self: &Rc<Self>, key: &str, label_key: Option<&str>, tip: Option<&str>) -> Rc<ColorButton> {
        let cb = ColorButton::new();
        let w = Rc::downgrade(self);
        let k = key.to_string();
        cb.connect(move |c| {
            if let Some(card) = w.upgrade() {
                card.set(&k, Value::String(c));
            }
        });
        let k = key.to_string();
        let c2 = cb.clone();
        self.add_row(key, cb.area.upcast_ref(), None, tip, Some(Box::new(move |s: &Obj| c2.set_color(s.s(&k)))),
            label_key);
        cb
    }

    pub fn choice(self: &Rc<Self>, key: &str, options: &[&str], prefix: &str, tip: Option<&str>,
        label_key: Option<&str>) -> Rc<Segmented> {
        let seg = Segmented::new(options, prefix);
        let w = Rc::downgrade(self);
        let k = key.to_string();
        seg.connect(move |v| {
            let Some(c) = w.upgrade() else { return };
            let is_int = c.host.upgrade().map(|h| jsonx::is_int(&h.value(&k))).unwrap_or(false);
            let jv = if is_int { Value::from(v.parse::<i64>().unwrap_or(0)) } else { Value::String(v) };
            c.set(&k, jv);
        });
        let k = key.to_string();
        let s2 = seg.clone();
        self.add_row(key, seg.root.upcast_ref(), None, tip,
            Some(Box::new(move |s: &Obj| s2.select(&jsonx::py_str(s.val(&k))))), label_key);
        self.segs.borrow_mut().push(seg.clone());
        seg
    }

    pub fn text(self: &Rc<Self>, key: &str, tip: Option<&str>, width: i32, label_key: Option<&str>) -> gtk::Entry {
        let le = gtk::Entry::new();
        le.set_size_request(width, -1);
        le.set_hexpand(false);
        let w = Rc::downgrade(self);
        let k = key.to_string();
        le.connect_changed(move |e| {
            if let Some(c) = w.upgrade() {
                c.set(&k, Value::String(e.text().trim().to_string()));
            }
        });
        let k = key.to_string();
        let l2 = le.clone();
        let loading = self.loading.clone();
        self.add_row(key, le.upcast_ref(), None, tip, Some(Box::new(move |s: &Obj| {
            if l2.text().trim() != s.s(&k) {
                loading.set(true);
                l2.set_text(s.s(&k));
                loading.set(false);
            }
        })), label_key);
        le
    }

    pub fn font(self: &Rc<Self>, key: &str, tip: Option<&str>, label_key: Option<&str>) -> Rc<FontCombo> {
        let fc = FontCombo::new();
        let w = Rc::downgrade(self);
        let k = key.to_string();
        fc.connect(move |f| {
            if let Some(c) = w.upgrade() {
                c.set(&k, Value::String(f));
            }
        });
        let k = key.to_string();
        let f2 = fc.clone();
        self.add_row(key, fc.combo.button.upcast_ref(), None, tip,
            Some(Box::new(move |s: &Obj| f2.fill(s.s(&k), false))), label_key);
        self.fonts.borrow_mut().push(fc.clone());
        fc
    }

    /// Mehrzeiliges Feld: eine Zeile = ein Listeneintrag.
    pub fn lines(self: &Rc<Self>, key: &str, tip: Option<&str>) -> gtk::TextView {
        let (frame, tv) = text_edit(110, 380);
        let w = Rc::downgrade(self);
        let k = key.to_string();
        tv.buffer().connect_changed(move |b| {
            if let Some(c) = w.upgrade() {
                let text = b.text(&b.start_iter(), &b.end_iter(), false);
                let v: Vec<Value> = text.lines().map(|x| x.trim()).filter(|x| !x.is_empty())
                    .map(|x| Value::String(x.into())).collect();
                c.set(&k, Value::Array(v));
            }
        });
        let k = key.to_string();
        let tv2 = tv.clone();
        let loading = self.loading.clone();
        self.add_row(key, frame.upcast_ref(), None, tip, Some(Box::new(move |s: &Obj| {
            let b = tv2.buffer();
            let cur: Vec<String> = b.text(&b.start_iter(), &b.end_iter(), false).lines().map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty()).collect();
            if cur != s.strs(&k) {
                loading.set(true);
                b.set_text(&s.strs(&k).join("\n"));
                loading.set(false);
            }
        })), None);
        tv
    }

    pub fn refresh(&self, s: &Obj) {
        for r in self.rows.borrow().iter() {
            if let Some(f) = &r.refresh {
                f(s);
            }
        }
    }

    pub fn retranslate(&self) {
        self.title.set_text(&t(&self.title_key));
        self.title.set_visible(!self.title_key.is_empty());
        for i in 0..self.rows.borrow().len() {
            self.retranslate_row(i);
        }
        for s in self.segs.borrow().iter() {
            s.retranslate();
        }
    }

    /// Zustands-Schlüssel aller Zeilen (für „Bereich zurücksetzen“).
    pub fn keys(&self) -> Vec<String> {
        self.rows.borrow().iter().map(|r| r.skey.clone()).filter(|k| !k.is_empty()).collect()
    }
}

/// Mehrzeiliges Eingabefeld im Rahmen (wie QPlainTextEdit, editierbar).
pub fn text_edit(height: i32, min_width: i32) -> (gtk::ScrolledWindow, gtk::TextView) {
    let tv = gtk::TextView::new();
    tv.add_css_class("Edit");
    tv.set_wrap_mode(gtk::WrapMode::None);
    let sw = gtk::ScrolledWindow::new();
    sw.add_css_class("EditFrame");
    sw.set_child(Some(&tv));
    sw.set_size_request(min_width, height);
    sw.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
    (sw, tv)
}

/// Mausrad ohne Fokus ändert keine Werte (die Seite scrollt stattdessen).
pub fn no_wheel<W: IsA<gtk::Widget>>(w: &W) {
    let sc = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::BOTH_AXES);
    sc.set_propagation_phase(gtk::PropagationPhase::Capture);
    let ww = w.clone().upcast::<gtk::Widget>();
    sc.connect_scroll(move |c, _dx, dy| {
        if ww.has_focus() {
            return glib::Propagation::Proceed;
        }
        // Seite scrollen
        let mut p = ww.parent();
        while let Some(x) = p {
            if let Ok(sw) = x.clone().downcast::<gtk::ScrolledWindow>() {
                let adj = sw.vadjustment();
                adj.set_value(adj.value() + dy * 40.0);
                break;
            }
            p = x.parent();
        }
        let _ = c;
        glib::Propagation::Stop
    });
    w.add_controller(sc);
}

// ---------- Preset-Karten ----------

pub struct PresetCard {
    pub root: gtk::Box,
    pub key: String,
    pub tkey: RefCell<String>,
    pub pic: gtk::Picture,
    pub name: gtk::Label,
    pub chip: gtk::Label,
    on_click: Cb<String>,
    on_menu: Cb<(f64, f64)>,
}

impl PresetCard {
    pub fn new(key: &str) -> Rc<PresetCard> {
        let root = vbox(8);
        root.add_css_class("Preset");
        root.set_size_request(212, 160);
        cursor_hand(&root);
        let pic = gtk::Picture::new();
        pic.set_size_request(196, 110);
        pic.set_can_shrink(true);
        pic.set_margin_start(8);
        pic.set_margin_end(8);
        pic.set_margin_top(8);
        let row = hbox(0);
        let name = label("", "name");
        let chip = label("", "ChipOk");
        chip.set_visible(false);
        row.append(&name);
        row.append(&spacer());
        row.append(&chip);
        margins(&row, 8, 0, 12, 10);
        root.append(&pic);
        root.append(&row);
        let c = Rc::new(PresetCard { root, key: key.into(), tkey: RefCell::new(String::new()), pic, name, chip,
            on_click: RefCell::new(None), on_menu: RefCell::new(None) });
        let click = gtk::GestureClick::new();
        click.set_button(0);
        let w = Rc::downgrade(&c);
        click.connect_released(move |g, _, x, y| {
            let Some(c) = w.upgrade() else { return };
            if g.current_button() == 3 {
                if let Some(f) = c.on_menu.borrow().as_ref() {
                    f((x, y));
                }
            } else if g.current_button() == 1 {
                if let Some(f) = c.on_click.borrow().as_ref() {
                    f(c.key.clone());
                }
            }
        });
        c.root.add_controller(click);
        c
    }

    pub fn connect(&self, f: impl Fn(String) + 'static) {
        *self.on_click.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_menu(&self, f: impl Fn((f64, f64)) + 'static) {
        *self.on_menu.borrow_mut() = Some(Box::new(f));
    }

    pub fn set_texture(&self, tex: &gdk::Texture) {
        self.pic.set_paintable(Some(tex));
    }

    pub fn refresh(&self, selected: bool, applied: bool) {
        self.chip.set_text(&t("applied"));
        self.chip.set_visible(applied);
        if selected {
            self.root.add_css_class("selected");
        } else {
            self.root.remove_css_class("selected");
        }
    }
}

/// Raster, das die Spaltenzahl der Breite anpasst (Zellen gleich breit, links bündig).
pub fn flow_grid() -> gtk::FlowBox {
    let fb = gtk::FlowBox::new();
    fb.set_selection_mode(gtk::SelectionMode::None);
    fb.set_column_spacing(14);
    fb.set_row_spacing(14);
    fb.set_homogeneous(true);
    fb.set_min_children_per_line(1);
    fb.set_max_children_per_line(30);
    fb.set_halign(gtk::Align::Start);
    fb.set_valign(gtk::Align::Start);
    fb.set_activate_on_single_click(false);
    fb
}

// ---------- Seitenleiste ----------

pub struct NavHeader {
    pub button: gtk::Button,
    gid: String,
    key: String,
    arrow: gtk::DrawingArea,
    text: gtk::Label,
    pub buttons: RefCell<Vec<gtk::ToggleButton>>,
}

impl NavHeader {
    pub fn new(gid: &str, key: &str) -> Rc<NavHeader> {
        let button = gtk::Button::new();
        button.add_css_class("NavHead");
        cursor_hand(&button);
        let row = hbox(4);
        let arrow = gtk::DrawingArea::new();
        arrow.set_content_width(16);
        arrow.set_content_height(14);
        arrow.set_valign(gtk::Align::Center);
        let text = label("", "");
        row.append(&arrow);
        row.append(&text);
        button.set_child(Some(&row));
        let h = Rc::new(NavHeader { button, gid: gid.into(), key: key.into(), arrow, text,
            buttons: RefCell::new(vec![]) });
        let w = Rc::downgrade(&h);
        h.arrow.set_draw_func(move |a, cr, wd, ht| {
            let Some(h) = w.upgrade() else { return };
            let hover = a.parent().and_then(|p| p.parent()).map(|b| b.state_flags().contains(gtk::StateFlags::PRELIGHT))
                .unwrap_or(false);
            let col = Rgba::hex(if hover { theme::TEXT } else { theme::MUTED });
            col.set(cr);
            cr.set_line_width(1.5);
            cr.set_line_cap(gtk::cairo::LineCap::Round);
            cr.set_line_join(gtk::cairo::LineJoin::Round);
            let (cx, cy, d) = (wd as f64 / 2.0, ht as f64 / 2.0, 3.2);
            if h.collapsed() {
                cr.move_to(cx - d / 2.0, cy - d);
                cr.line_to(cx + d / 2.0, cy);
                cr.line_to(cx - d / 2.0, cy + d);
            } else {
                cr.move_to(cx - d, cy - d / 2.0);
                cr.line_to(cx, cy + d / 2.0);
                cr.line_to(cx + d, cy - d / 2.0);
            }
            let _ = cr.stroke();
        });
        let w = Rc::downgrade(&h);
        h.button.connect_clicked(move |_| {
            if let Some(h) = w.upgrade() {
                h.toggle();
            }
        });
        let motion = gtk::EventControllerMotion::new();
        let a = h.arrow.clone();
        motion.connect_enter(move |_, _, _| a.queue_draw());
        let a = h.arrow.clone();
        motion.connect_leave(move |_| a.queue_draw());
        h.button.add_controller(motion);
        h
    }

    pub fn collapsed(&self) -> bool {
        let cfg = shell().cfg();
        let c = cfg.borrow();
        c.strs("nav_collapsed").contains(&self.gid)
    }

    pub fn toggle(&self) {
        {
            let cfg = shell().cfg();
            let mut c = cfg.borrow_mut();
            let mut lst = c.strs("nav_collapsed");
            if let Some(i) = lst.iter().position(|x| x == &self.gid) {
                lst.remove(i);
            } else {
                lst.push(self.gid.clone());
            }
            c.insert("nav_collapsed".into(), Value::Array(lst.into_iter().map(Value::String).collect()));
        }
        shell().save_later();
        self.sync();
    }

    pub fn sync(&self) {
        let c = self.collapsed();
        for b in self.buttons.borrow().iter() {
            b.set_visible(!c || b.is_active());
        }
        self.retranslate();
        self.arrow.queue_draw();
    }

    pub fn retranslate(&self) {
        self.text.set_text(&t(&format!("nav.{}", self.key)));
        self.button.set_tooltip_text(Some(&t(if self.collapsed() { "nav.unfold" } else { "nav.fold" })));
    }
}

/// Seitenleiste nach Kategorien ordnen: Überschrift, darunter ihre Bereiche; Bereiche ohne Kategorie ans Ende.
pub fn nav_groups(layout: &gtk::Box, nav: &[(String, gtk::ToggleButton)], groups: &[(&str, Vec<&str>)], kind: &str)
    -> Vec<Rc<NavHeader>> {
    let mut heads = vec![];
    let mut placed = vec![];
    for (key, items) in groups {
        let h = NavHeader::new(&format!("{}:{}", kind, key), key);
        layout.append(&h.button);
        for k in items {
            if let Some((_, b)) = nav.iter().find(|(n, _)| n == k) {
                layout.append(b);
                h.buttons.borrow_mut().push(b.clone());
                let w = Rc::downgrade(&h);
                b.connect_toggled(move |_| {
                    if let Some(h) = w.upgrade() {
                        h.sync();
                    }
                });
                placed.push(k.to_string());
            }
        }
        heads.push(h);
    }
    for (k, b) in nav {
        if !placed.contains(k) {
            layout.append(b);
        }
    }
    for h in &heads {
        h.sync();
    }
    heads
}

/// Knopf der Seitenleiste.
pub fn nav_button(group: Option<&gtk::ToggleButton>) -> gtk::ToggleButton {
    let b = gtk::ToggleButton::new();
    b.add_css_class("Nav");
    cursor_hand(&b);
    if let Some(g) = group {
        b.set_group(Some(g));
    }
    if let Some(l) = b.child().and_then(|c| c.downcast::<gtk::Label>().ok()) {
        l.set_xalign(0.0);
    }
    b
}

pub fn nav_set_text(b: &gtk::ToggleButton, text: &str) {
    b.set_label(text);
    if let Some(l) = b.child().and_then(|c| c.downcast::<gtk::Label>().ok()) {
        l.set_xalign(0.0);
    }
}

// ---------- Auswahl mit Suche ----------

#[derive(Clone, Default)]
pub struct ComboItem {
    pub text: String,
    pub data: String,
    pub font: Option<String>,
    pub tooltip: Option<String>,
    pub muted: bool,
}

impl ComboItem {
    pub fn new(text: &str, data: &str) -> ComboItem {
        ComboItem { text: text.into(), data: data.into(), ..Default::default() }
    }
}

/// Auswahlmenü mit Suchfeld oben – Tippen sucht sofort, Pfeile/Enter/Esc funktionieren.
pub struct SearchCombo {
    pub button: gtk::Button,
    shown: gtk::Label,
    pub items: RefCell<Vec<ComboItem>>,
    pub current: Cell<i32>,
    popover: gtk::Popover,
    search: gtk::SearchEntry,
    list: gtk::ListBox,
    built: Cell<bool>,
    on_activate: Cb<i32>,
    pub editable: Option<gtk::Entry>,
}

impl SearchCombo {
    pub fn new() -> Rc<SearchCombo> {
        let button = gtk::Button::new();
        button.add_css_class("combo");
        cursor_hand(&button);
        let row = hbox(8);
        let shown = label("", "");
        shown.set_hexpand(true);
        shown.set_ellipsize(pango::EllipsizeMode::End);
        row.append(&shown);
        row.append(&draw::icon_image("chev", theme::MUTED, 14));
        button.set_child(Some(&row));
        no_wheel(&button);
        let popover = gtk::Popover::new();
        popover.set_has_arrow(false);
        popover.set_position(gtk::PositionType::Bottom);
        popover.set_parent(&button);
        let card = vbox(6);
        let search = gtk::SearchEntry::new();
        let list = gtk::ListBox::new();
        list.add_css_class("PopList");
        list.set_selection_mode(gtk::SelectionMode::Single);
        list.set_activate_on_single_click(true);
        let sw = gtk::ScrolledWindow::new();
        sw.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        sw.set_child(Some(&list));
        sw.set_propagate_natural_height(true);
        sw.set_max_content_height(12 * 30 + 8);
        card.append(&search);
        card.append(&sw);
        popover.set_child(Some(&card));
        let c = Rc::new(SearchCombo { button, shown, items: RefCell::new(vec![]), current: Cell::new(-1), popover,
            search, list, built: Cell::new(false), on_activate: RefCell::new(None), editable: None });
        let w = Rc::downgrade(&c);
        c.button.connect_clicked(move |_| {
            if let Some(c) = w.upgrade() {
                c.open();
            }
        });
        let w = Rc::downgrade(&c);
        c.search.connect_search_changed(move |_| {
            if let Some(c) = w.upgrade() {
                c.filter();
            }
        });
        let w = Rc::downgrade(&c);
        c.search.connect_activate(move |_| {
            if let Some(c) = w.upgrade() {
                if let Some(r) = c.list.selected_row().filter(|r| r.is_child_visible()) {
                    c.choose(r.index());
                }
            }
        });
        let w = Rc::downgrade(&c);
        c.search.connect_stop_search(move |_| {
            if let Some(c) = w.upgrade() {
                c.popover.popdown();
            }
        });
        let keys = gtk::EventControllerKey::new();
        let w = Rc::downgrade(&c);
        keys.connect_key_pressed(move |_, key, _, _| {
            let Some(c) = w.upgrade() else { return glib::Propagation::Proceed };
            let step = match key {
                gdk::Key::Down => 1,
                gdk::Key::Up => -1,
                gdk::Key::Page_Down => 8,
                gdk::Key::Page_Up => -8,
                _ => return glib::Propagation::Proceed,
            };
            c.move_sel(step);
            glib::Propagation::Stop
        });
        c.search.add_controller(keys);
        let w = Rc::downgrade(&c);
        c.list.connect_row_activated(move |_, r| {
            if let Some(c) = w.upgrade() {
                c.choose(r.index());
            }
        });
        c
    }

    pub fn connect(&self, f: impl Fn(i32) + 'static) {
        *self.on_activate.borrow_mut() = Some(Box::new(f));
    }

    pub fn set_items(&self, items: Vec<ComboItem>) {
        *self.items.borrow_mut() = items;
        self.built.set(false);
        if self.current.get() >= self.items.borrow().len() as i32 {
            self.current.set(-1);
        }
        self.update_shown();
    }

    pub fn set_current(&self, i: i32) {
        self.current.set(i);
        self.update_shown();
    }

    pub fn find_data(&self, data: &str) -> i32 {
        self.items.borrow().iter().position(|x| x.data == data).map(|x| x as i32).unwrap_or(-1)
    }

    pub fn current_data(&self) -> String {
        let i = self.current.get();
        self.items.borrow().get(i.max(0) as usize).filter(|_| i >= 0).map(|x| x.data.clone()).unwrap_or_default()
    }

    pub fn current_text(&self) -> String {
        let i = self.current.get();
        self.items.borrow().get(i.max(0) as usize).filter(|_| i >= 0).map(|x| x.text.clone()).unwrap_or_default()
    }

    fn update_shown(&self) {
        self.shown.set_text(&self.current_text());
    }

    fn build(&self) {
        if self.built.get() {
            return;
        }
        self.built.set(true);
        while let Some(r) = self.list.first_child() {
            self.list.remove(&r);
        }
        for it in self.items.borrow().iter() {
            let l = label(&it.text, if it.muted { "muted" } else { "" });
            l.set_ellipsize(pango::EllipsizeMode::End);
            if let Some(f) = &it.font {
                let attrs = pango::AttrList::new();
                attrs.insert(pango::AttrString::new_family(f));
                l.set_attributes(Some(&attrs));
            }
            if let Some(tip) = &it.tooltip {
                l.set_tooltip_text(Some(tip));
            }
            let row = gtk::ListBoxRow::new();
            row.set_child(Some(&l));
            row.set_size_request(-1, 30);
            self.list.append(&row);
        }
    }

    pub fn open(&self) {
        self.search.set_placeholder_text(Some(&t("search")));
        self.build();
        self.search.set_text("");
        self.filter();
        let w = self.button.width().max(280);
        self.popover.set_size_request(w, -1);
        if let Some(r) = self.list.row_at_index(self.current.get()) {
            self.list.select_row(Some(&r));
        }
        self.popover.popup();
        self.search.grab_focus();
        if let Some(r) = self.list.row_at_index(self.current.get()) {
            let r2 = r.clone();
            glib::idle_add_local_once(move || {
                r2.grab_focus();
            });
            let s = self.search.clone();
            glib::idle_add_local_once(move || {
                s.grab_focus();
            });
        }
    }

    fn filter(&self) {
        let words: Vec<String> = self.search.text().to_lowercase().split_whitespace().map(String::from).collect();
        let items = self.items.borrow();
        let mut first = None;
        for (i, it) in items.iter().enumerate() {
            let low = it.text.to_lowercase();
            let hit = words.iter().all(|w| low.contains(w));
            if let Some(r) = self.list.row_at_index(i as i32) {
                r.set_visible(hit);
                if hit && first.is_none() {
                    first = Some(r);
                }
            }
        }
        if !words.is_empty() {
            if let Some(r) = first {
                self.list.select_row(Some(&r));
            }
        }
    }

    fn move_sel(&self, step: i32) {
        let n = self.items.borrow().len() as i32;
        let vis: Vec<i32> = (0..n).filter(|i| self.list.row_at_index(*i).map(|r| r.is_visible()).unwrap_or(false))
            .collect();
        if vis.is_empty() {
            return;
        }
        let cur = self.list.selected_row().map(|r| r.index()).unwrap_or(-1);
        let pos = vis.iter().position(|x| *x == cur).map(|x| x as i32).unwrap_or(-1);
        let np = (pos + step).clamp(0, vis.len() as i32 - 1);
        if let Some(r) = self.list.row_at_index(vis[np as usize]) {
            self.list.select_row(Some(&r));
            r.grab_focus();
            self.search.grab_focus();
        }
    }

    fn choose(&self, i: i32) {
        self.popover.popdown();
        self.current.set(i);
        self.update_shown();
        if let Some(f) = self.on_activate.borrow().as_ref() {
            f(i);
        }
    }

    pub fn set_sensitive(&self, on: bool) {
        self.button.set_sensitive(on);
    }
}

// ---------- Schriftwahl ----------

thread_local! {
    static FONT_COMBOS: RefCell<Vec<Weak<FontCombo>>> = const { RefCell::new(Vec::new()) };
    static FAMILIES: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

/// Installierte Schriftfamilien (sortiert, ohne versteckte).
pub fn font_families(force: bool) -> Vec<String> {
    if !force {
        if let Some(f) = FAMILIES.with(|f| f.borrow().clone()) {
            return f;
        }
    }
    let ctx = gtk::Label::new(None).pango_context();
    if let Some(fm) = ctx.font_map() {
        if force {
            fm.changed();
        }
    }
    let mut v: Vec<String> = ctx.list_families().iter().map(|f| f.name().to_string())
        .filter(|f| !f.starts_with('.')).collect();
    v.sort_by_key(|a| a.to_lowercase());
    v.dedup();
    FAMILIES.with(|f| *f.borrow_mut() = Some(v.clone()));
    v
}

/// Schriftart wählen: installierte Schriften in ihrer eigenen Schrift, darunter Nerd Fonts zum Installieren.
pub struct FontCombo {
    pub combo: Rc<SearchCombo>,
    pub value: RefCell<Option<String>>,
    busy: Cell<bool>,
    on_chosen: Cb<String>,
}

impl FontCombo {
    pub fn new() -> Rc<FontCombo> {
        let combo = SearchCombo::new();
        combo.button.set_size_request(320, -1);
        let f = Rc::new(FontCombo { combo, value: RefCell::new(None), busy: Cell::new(false),
            on_chosen: RefCell::new(None) });
        let w = Rc::downgrade(&f);
        f.combo.connect(move |i| {
            if let Some(f) = w.upgrade() {
                f.picked(i);
            }
        });
        FONT_COMBOS.with(|l| l.borrow_mut().push(Rc::downgrade(&f)));
        f
    }

    pub fn connect(&self, f: impl Fn(String) + 'static) {
        *self.on_chosen.borrow_mut() = Some(Box::new(f));
    }

    pub fn fill(&self, value: &str, force: bool) {
        if !force && !self.combo.items.borrow().is_empty() && self.value.borrow().as_deref() == Some(value) {
            return;
        }
        *self.value.borrow_mut() = Some(value.to_string());
        let fams = font_families(false);
        let mut items = vec![];
        if !value.is_empty() && !fams.iter().any(|f| f == value) {
            items.push(ComboItem::new(&format!("{}  ·  {}", value, t("font.missing")), &format!("font:{}", value)));
        }
        for f in &fams {
            items.push(ComboItem { font: Some(f.clone()), ..ComboItem::new(f, &format!("font:{}", f)) });
        }
        let get = t("font.get");
        for (archive, fam) in fonts::CATALOG {
            if !fams.iter().any(|f| f == fam) {
                items.push(ComboItem { tooltip: Some(t("font.get_tip")), muted: true,
                    ..ComboItem::new(&format!("{}  –  {}", fam, get), &format!("get:{}", archive)) });
            }
        }
        self.combo.set_items(items);
        let i = self.combo.find_data(&format!("font:{}", value));
        self.combo.set_current(i.max(0));
    }

    fn picked(self: &Rc<Self>, i: i32) {
        let data = self.combo.items.borrow().get(i as usize).map(|x| x.data.clone()).unwrap_or_default();
        if let Some(f) = data.strip_prefix("font:") {
            *self.value.borrow_mut() = Some(f.to_string());
            if let Some(cb) = self.on_chosen.borrow().as_ref() {
                cb(f.to_string());
            }
            return;
        }
        let cur = self.value.borrow().clone().unwrap_or_default();
        self.combo.set_current(self.combo.find_data(&format!("font:{}", cur)).max(0));
        if let Some(archive) = data.strip_prefix("get:") {
            if !self.busy.get() {
                self.install(archive.to_string());
            }
        }
    }

    fn install(self: &Rc<Self>, archive: String) {
        self.busy.set(true);
        self.combo.set_sensitive(false);
        shell::status(&tf("font.loading", kw!("name" => fonts::family_of(&archive))));
        if shell::has_shell() {
            shell().bar_busy(true);
        }
        let w = Rc::downgrade(self);
        let a2 = archive.clone();
        shell::run_job(move |log| fonts::install(&a2, log), move |r| {
            let Some(f) = w.upgrade() else { return };
            f.busy.set(false);
            f.combo.set_sensitive(true);
            if shell::has_shell() {
                shell().bar_busy(false);
            }
            match r {
                Ok(files) if !files.is_empty() => {
                    let fams = font_families(true);
                    let mut fam = fonts::family_of(&archive);
                    if !fams.iter().any(|x| x == &fam) {
                        // neu installierte Familie mit dem kürzesten passenden Namen
                        let base = fam.split_whitespace().next().unwrap_or("").to_lowercase();
                        if let Some(x) = fams.iter().filter(|x| x.to_lowercase().starts_with(&base))
                            .min_by_key(|x| x.len()) {
                            fam = x.clone();
                        }
                    }
                    FONT_COMBOS.with(|l| {
                        for c in l.borrow().iter().filter_map(|w| w.upgrade()) {
                            let v = c.value.borrow().clone().unwrap_or_default();
                            c.fill(&v, true);
                        }
                    });
                    f.fill(&fam, true);
                    if let Some(cb) = f.on_chosen.borrow().as_ref() {
                        cb(fam.clone());
                    }
                    shell::status(&tf("font.done", kw!("name" => fam)));
                }
                other => {
                    let e = match other {
                        Err(e) => e,
                        _ => "[]".into(),
                    };
                    shell::log(&format!("{}: {}", archive, e));
                    shell::status(&t("font.fail"));
                }
            }
        });
    }
}

/// Fortschrittsbalken ohne Prozent (läuft hin und her).
pub fn busy_bar(width: i32) -> gtk::ProgressBar {
    let b = gtk::ProgressBar::new();
    b.set_size_request(width, -1);
    b.set_valign(gtk::Align::Center);
    b.set_visible(false);
    let b2 = b.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(80), move || {
        if b2.is_visible() {
            b2.pulse();
        }
        glib::ControlFlow::Continue
    });
    b.set_pulse_step(0.06);
    b
}

/// Seite im Bildlauf (wie QScrollArea mit Inhalt).
pub fn scroller<W: IsA<gtk::Widget>>(child: &W) -> gtk::ScrolledWindow {
    let sw = gtk::ScrolledWindow::new();
    sw.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    sw.set_child(Some(child));
    sw.set_vexpand(true);
    sw.set_hexpand(true);
    sw
}
