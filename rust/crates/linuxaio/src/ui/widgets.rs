//! Gemeinsame Bausteine: Symbole, „Angewendet“-Knopf, Schilder, Segmente, Kacheln, Kachel-Leiste, Bildfläche.

use super::img;
use crate::i18n::t;
use gtk::gdk_pixbuf::{Pixbuf, PixbufAnimation, PixbufAnimationIter};
use gtk::prelude::*;
use gtk::{cairo, glib, pango};
use hyprstudio::ui::draw::{self, Font, HAlign, Rgba, R};
use hyprstudio::ui::theme;
use hyprstudio::ui::widgets::{cursor_hand, hbox, label, vbox, Segmented};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub type Cb<T> = RefCell<Option<Box<dyn Fn(T)>>>;


/// Segment-Umschalter mit Linux-AIO-Texten.
pub fn seg(keys: &[&str], prefix: &str) -> Rc<Segmented> {
    Segmented::new_tr(keys, prefix, t)
}

/// Programmsymbol (Tux) in Pixelgröße.
pub fn app_icon(size: i32) -> gtk::Image {
    let theme_has = gtk::gdk::Display::default()
        .map(|d| gtk::IconTheme::for_display(&d).has_icon(crate::edition::ed().id)).unwrap_or(false);
    let img = if theme_has {
        gtk::Image::from_icon_name(crate::edition::ed().id)
    } else if let Some(pb) = img::from_bytes_at(crate::edition::ed().icon_svg, size * 2, size * 2) {
        gtk::Image::from_paintable(Some(&img::texture(&pb)))
    } else {
        gtk::Image::new()
    };
    img.set_pixel_size(size);
    img
}

/// Kreispfeil-Knopf „Auf Standard zurücksetzen“ (30×30, Symbol 16).
pub fn reset_button() -> gtk::Button {
    let b = gtk::Button::new();
    b.add_css_class("ResetBtn");
    b.set_child(Some(&draw::icon_image("reset", theme::MUTED, 16)));
    b.set_size_request(30, 30);
    b.set_valign(gtk::Align::Center);
    cursor_hand(&b);
    b
}

/// Knopf zeigt „Angewendet“ (grüner Rand + Häkchen) statt ausgegraut, wenn der Zustand schon aktiv ist.
pub fn set_applied(b: &gtk::Button, applied: bool, text: &str) {
    if applied {
        b.add_css_class("applied");
        let bx = hbox(6);
        bx.set_halign(gtk::Align::Center);
        bx.append(&draw::icon_image("check", theme::OK, 16));
        let l = gtk::Label::new(Some(text));
        l.set_ellipsize(pango::EllipsizeMode::End);
        bx.append(&l);
        b.set_child(Some(&bx));
        b.set_sensitive(false);
    } else if b.has_css_class("applied") {
        b.remove_css_class("applied");
        b.set_label("");
    }
}

/// Text eines Knopfs setzen (ohne „Angewendet“-Zustand zu zerstören, der ein eigenes Kind hat).
pub fn set_text(b: &gtk::Button, text: &str) {
    if !b.has_css_class("applied") {
        b.set_label(text);
    }
}

/// Knopf mit Linux-AIO-Text (Höhe wie tool_button: 34).
pub fn tool_button(text: &str, primary: bool) -> gtk::Button {
    let b = gtk::Button::with_label(text);
    if primary {
        b.add_css_class("Primary");
    }
    b.set_size_request(-1, 34);
    b.set_valign(gtk::Align::Center);
    cursor_hand(&b);
    b
}

/// Suchfeld mit Lupe links und Löschen-Knopf (wie QLineEdit#Search).
pub fn search_entry(width: i32) -> gtk::Entry {
    let e = gtk::Entry::new();
    e.add_css_class("Search");
    e.set_primary_icon_paintable(Some(&draw::icon("search", theme::MUTED, 16)));
    e.connect_changed(|e| {
        let icon = if e.text().is_empty() { None } else { Some("edit-clear-symbolic") };
        e.set_secondary_icon_name(icon);
    });
    e.connect_icon_release(|e, pos| {
        if pos == gtk::EntryIconPosition::Secondary {
            e.set_text("");
        }
    });
    if width > 0 {
        e.set_size_request(width, -1);
    }
    e.set_valign(gtk::Align::Center);
    e
}

/// Schild (Chip) mit Art: AChip, AChipOk, AChipWarn, AChipAccent.
pub fn chip(text: &str, kind: &str) -> gtk::Label {
    let l = gtk::Label::new(Some(text));
    l.add_css_class(&a_kind(kind));
    l.set_valign(gtk::Align::Center);
    l
}

/// Python-Namen (Chip, ChipOk …) → Linux-AIO-Klassen.
pub fn a_kind(kind: &str) -> String {
    if kind.starts_with('A') { kind.to_string() } else { format!("A{kind}") }
}

pub fn set_chip(l: &gtk::Label, kind: &str, text: &str) {
    for c in ["AChip", "AChipOk", "AChipWarn", "AChipAccent"] {
        l.remove_css_class(c);
    }
    l.add_css_class(&a_kind(kind));
    l.set_text(text);
}

pub type Chips = Vec<(String, String, String)>;

/// Schilder nebeneinander (brechen um) – oder nur als Tooltip eines anderen Widgets.
pub struct ChipRow {
    pub root: gtk::FlowBox,
    pub tip_target: RefCell<Option<gtk::Widget>>,
    pub base_tip: RefCell<String>,
}

impl ChipRow {
    pub fn new() -> Rc<ChipRow> {
        let fb = gtk::FlowBox::new();
        fb.set_selection_mode(gtk::SelectionMode::None);
        fb.set_column_spacing(6);
        fb.set_row_spacing(6);
        fb.set_max_children_per_line(20);
        fb.set_homogeneous(false);
        fb.set_halign(gtk::Align::Start);
        Rc::new(ChipRow { root: fb, tip_target: RefCell::new(None), base_tip: RefCell::new(String::new()) })
    }

    pub fn tip_for<W: IsA<gtk::Widget>>(&self, w: &W) {
        *self.tip_target.borrow_mut() = Some(w.clone().upcast());
        self.root.set_visible(false);
    }

    pub fn set(&self, chips: &[(String, String, String)]) {
        if let Some(tt) = self.tip_target.borrow().as_ref() {
            let mut lines: Vec<String> = vec![];
            let base = self.base_tip.borrow().clone();
            if !base.is_empty() {
                lines.push(base);
            }
            for (text, _k, tip) in chips {
                lines.push(if tip.is_empty() { text.clone() } else { format!("{text} – {tip}") });
            }
            let s = lines.join("\n");
            tt.set_tooltip_text(if s.is_empty() { None } else { Some(&s) });
            self.root.set_visible(false);
            return;
        }
        while let Some(c) = self.root.first_child() {
            self.root.remove(&c);
        }
        for (text, kind, tip) in chips {
            let l = chip(text, kind);
            if !tip.is_empty() {
                l.set_tooltip_text(Some(tip));
            }
            self.root.insert(&l, -1);
        }
        // eine Zeile wie in Qt (sonst stapelt die FlowBox bei halign=Start untereinander)
        self.root.set_min_children_per_line(chips.len().max(1) as u32);
        self.root.set_visible(true);
    }
}

pub fn chips(v: &[(&str, &str, &str)]) -> Chips {
    v.iter().map(|(a, b, c)| (a.to_string(), b.to_string(), c.to_string())).collect()
}

/// Kachel mit Bildfläche oben, Name und Status-Punkt.
pub struct Tile {
    pub root: gtk::Box,
    pub key: String,
    pub name: gtk::Label,
    dot: gtk::DrawingArea,
    dot_color: RefCell<Option<String>>,
    on_click: Cb<String>,
}

impl Tile {
    pub fn new(key: &str, title: &str, pic: &gtk::Widget) -> Rc<Tile> {
        let root = vbox(8);
        root.add_css_class("ATile");
        root.set_size_request(176, 150);
        root.set_valign(gtk::Align::Start);
        cursor_hand(&root);
        pic.set_size_request(-1, 96);
        pic.set_can_target(false);
        pic.set_margin_start(10);
        pic.set_margin_end(10);
        pic.set_margin_top(10);
        let row = hbox(0);
        row.set_margin_start(10);
        row.set_margin_end(10);
        row.set_margin_bottom(10);
        let name = label(title, "name");
        name.set_ellipsize(pango::EllipsizeMode::End);
        name.set_max_width_chars(1);
        name.set_hexpand(true);
        name.set_size_request(134, -1);
        let dot = gtk::DrawingArea::new();
        dot.set_content_width(8);
        dot.set_content_height(8);
        dot.set_valign(gtk::Align::Center);
        row.append(&name);
        row.append(&dot);
        root.append(pic);
        root.append(&row);
        let t = Rc::new(Tile { root, key: key.into(), name, dot, dot_color: RefCell::new(None),
            on_click: RefCell::new(None) });
        let w = Rc::downgrade(&t);
        t.dot.set_draw_func(move |_, cr, wd, ht| {
            let Some(t) = w.upgrade() else { return };
            let c = t.dot_color.borrow().clone();
            if let Some(c) = c {
                draw::fill_ellipse(cr, wd as f64 / 2.0, ht as f64 / 2.0, 4.0, 4.0, Rgba::hex(&c));
            }
        });
        let click = gtk::GestureClick::new();
        let w = Rc::downgrade(&t);
        click.connect_released(move |_, _, _, _| {
            if let Some(t) = w.upgrade() {
                if let Some(f) = t.on_click.borrow().as_ref() {
                    f(t.key.clone());
                }
            }
        });
        t.root.add_controller(click);
        t
    }

    pub fn connect(&self, f: impl Fn(String) + 'static) {
        *self.on_click.borrow_mut() = Some(Box::new(f));
    }

    pub fn refresh(&self, selected: bool, dot: Option<&str>) {
        if selected {
            self.root.add_css_class("sel");
        } else {
            self.root.remove_css_class("sel");
        }
        *self.dot_color.borrow_mut() = dot.map(String::from);
        self.dot.queue_draw();
    }
}

/// Waagerechte Kachel-Leiste; meldet, wenn das Ende erreicht ist (zum Nachladen). Mausrad scrollt waagerecht.
pub struct TileStrip {
    pub root: gtk::ScrolledWindow,
    pub inner: gtk::Box,
    on_end: Cb<()>,
}

impl TileStrip {
    pub fn new() -> Rc<TileStrip> {
        let root = gtk::ScrolledWindow::new();
        root.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Never);
        root.set_size_request(-1, 172);
        root.set_vexpand(false);
        let inner = hbox(12);
        inner.set_margin_bottom(6);
        inner.set_valign(gtk::Align::Start);
        root.set_child(Some(&inner));
        let s = Rc::new(TileStrip { root, inner, on_end: RefCell::new(None) });
        let w = Rc::downgrade(&s);
        s.root.hadjustment().connect_value_changed(move |a| {
            let max = a.upper() - a.page_size();
            if max > 0.0 && a.value() >= max - 200.0 {
                if let Some(s) = w.upgrade() {
                    if let Some(f) = s.on_end.borrow().as_ref() {
                        f(());
                    }
                }
            }
        });
        let sc = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::BOTH_AXES);
        sc.set_propagation_phase(gtk::PropagationPhase::Capture);
        let sw = s.root.clone();
        sc.connect_scroll(move |c, dx, dy| {
            let d = if dy != 0.0 { dy } else { dx };
            let unit = if c.unit() == gtk::gdk::ScrollUnit::Wheel { 120.0 } else { 1.0 };
            let a = sw.hadjustment();
            a.set_value((a.value() + d * unit).clamp(0.0, (a.upper() - a.page_size()).max(0.0)));
            glib::Propagation::Stop
        });
        s.root.add_controller(sc);
        s
    }

    pub fn connect_end(&self, f: impl Fn(()) + 'static) {
        *self.on_end.borrow_mut() = Some(Box::new(f));
    }

    pub fn clear(&self) {
        while let Some(c) = self.inner.first_child() {
            self.inner.remove(&c);
        }
    }

    pub fn add<W: IsA<gtk::Widget>>(&self, w: &W) {
        self.inner.append(w);
    }

    pub fn to_start(&self) {
        self.root.hadjustment().set_value(0.0);
    }
}

// ---------- Bildfläche ----------

/// Bootsplash aus Einzelbildern.
#[derive(Clone)]
pub struct Anim {
    pub frames: Vec<Rc<cairo::ImageSurface>>,
    pub logo: Option<Rc<cairo::ImageSurface>>,
    pub bgimg: Option<Rc<cairo::ImageSurface>>,
    pub bg: String,
    pub bg2: String,
    pub ha: f64,
    pub va: f64,
    pub wva: f64,
}

enum Mode {
    Empty,
    Pix(Rc<cairo::ImageSurface>),
    Anim(Anim, Cell<usize>),
    Gif(PixbufAnimationIter, RefCell<Option<Rc<cairo::ImageSurface>>>),
    Place,
}

/// Bild mit runden Ecken, füllend zugeschnitten (16:10); ohne Bild ein gezeichneter Anmelde- bzw. Bootbildschirm.
pub struct Picture {
    pub area: gtk::DrawingArea,
    radius: f64,
    mode: RefCell<Mode>,
    seed: RefCell<String>,
    accent: RefCell<String>,
    loading: Cell<bool>,
    style: RefCell<String>,
    timer: RefCell<Option<glib::SourceId>>,
}

impl Picture {
    pub fn new(radius: f64) -> Rc<Picture> {
        let area = gtk::DrawingArea::new();
        let p = Rc::new(Picture { area, radius, mode: RefCell::new(Mode::Empty), seed: RefCell::new(String::new()),
            accent: RefCell::new(theme::accent_of("01")), loading: Cell::new(false), style: RefCell::new("login".into()),
            timer: RefCell::new(None) });
        let w = Rc::downgrade(&p);
        p.area.set_draw_func(move |_, cr, wd, ht| {
            if let Some(p) = w.upgrade() {
                p.paint(cr, wd as f64, ht as f64);
            }
        });
        p
    }

    pub fn widget(&self) -> gtk::Widget {
        self.area.clone().upcast()
    }

    fn stop(&self) {
        if let Some(id) = self.timer.borrow_mut().take() {
            id.remove();
        }
    }

    pub fn has_pix(&self) -> bool {
        matches!(&*self.mode.borrow(), Mode::Pix(_))
    }

    pub fn set_surface(&self, s: Option<Rc<cairo::ImageSurface>>) {
        self.stop();
        *self.mode.borrow_mut() = match s {
            Some(s) => Mode::Pix(s),
            None => Mode::Empty,
        };
        self.loading.set(false);
        self.area.queue_draw();
    }

    pub fn set_pixbuf(&self, pb: Option<&Pixbuf>) {
        self.set_surface(pb.and_then(img::surface).map(Rc::new));
    }

    pub fn set_placeholder(&self, seed: &str, accent: &str, loading: bool, style: &str) {
        self.stop();
        *self.mode.borrow_mut() = Mode::Place;
        *self.seed.borrow_mut() = seed.into();
        *self.accent.borrow_mut() = accent.into();
        self.loading.set(loading);
        *self.style.borrow_mut() = style.into();
        self.area.queue_draw();
    }

    /// PNG/JPG als Bild, animiertes GIF wird abgespielt.
    pub fn set_image_data(self: &Rc<Self>, data: &[u8]) {
        if data.starts_with(b"GIF8") {
            let stream = gtk::gio::MemoryInputStream::from_bytes(&glib::Bytes::from(data));
            if let Ok(an) = PixbufAnimation::from_stream(&stream, gtk::gio::Cancellable::NONE) {
                if !an.is_static_image() {
                    self.stop();
                    self.loading.set(false);
                    let it = an.iter(Some(std::time::SystemTime::now()));
                    let first = img::surface(&it.pixbuf()).map(Rc::new);
                    let delay = it.delay_time().map(|d| d.as_millis() as u64).unwrap_or(100).max(20);
                    *self.mode.borrow_mut() = Mode::Gif(it, RefCell::new(first));
                    self.schedule_gif(delay);
                    self.area.queue_draw();
                    return;
                }
            }
        }
        self.set_pixbuf(img::from_bytes(data).as_ref());
    }

    fn schedule_gif(self: &Rc<Self>, delay: u64) {
        let w = Rc::downgrade(self);
        let id = glib::timeout_add_local_once(std::time::Duration::from_millis(delay), move || {
            let Some(p) = w.upgrade() else { return };
            p.timer.borrow_mut().take();
            let mut next = 100;
            if let Mode::Gif(it, cur) = &*p.mode.borrow() {
                if p.area.is_mapped() {
                    it.advance(std::time::SystemTime::now());
                    *cur.borrow_mut() = img::surface(&it.pixbuf()).map(Rc::new);
                    p.area.queue_draw();
                }
                next = it.delay_time().map(|d| d.as_millis() as u64).unwrap_or(100).max(20);
            } else {
                return;
            }
            p.schedule_gif(next);
        });
        *self.timer.borrow_mut() = Some(id);
    }

    pub fn set_animation(self: &Rc<Self>, a: Anim, animate: bool) {
        self.stop();
        self.loading.set(false);
        let many = a.frames.len() > 1;
        *self.mode.borrow_mut() = Mode::Anim(a, Cell::new(0));
        if animate && many {
            let w = Rc::downgrade(self);
            let id = glib::timeout_add_local(std::time::Duration::from_millis(30), move || {
                let Some(p) = w.upgrade() else { return glib::ControlFlow::Break };
                if let Mode::Anim(a, idx) = &*p.mode.borrow() {
                    if p.area.is_mapped() {
                        idx.set((idx.get() + 1) % a.frames.len().max(1));
                        p.area.queue_draw();
                    }
                }
                glib::ControlFlow::Continue
            });
            *self.timer.borrow_mut() = Some(id);
        }
        self.area.queue_draw();
    }

    fn paint(&self, cr: &cairo::Context, w: f64, h: f64) {
        let mut r = R::new(0.0, 0.0, w, h);
        let target_h = r.w * 10.0 / 16.0;
        if target_h > r.h {
            let tw = r.h * 16.0 / 10.0;
            r = R::new(r.x + (r.w - tw) / 2.0, r.y, tw, r.h);
        } else {
            r = R::new(r.x, r.y + (r.h - target_h) / 2.0, r.w, target_h);
        }
        cr.save().ok();
        draw::rounded(cr, r, self.radius);
        cr.clip();
        match &*self.mode.borrow() {
            Mode::Pix(s) => img::draw_cover(cr, s, r),
            Mode::Gif(_, cur) => {
                if let Some(s) = cur.borrow().as_ref() {
                    img::draw_cover(cr, s, r);
                }
            }
            Mode::Anim(a, idx) => paint_anim(cr, r, a, idx.get()),
            Mode::Place | Mode::Empty => {
                if *self.style.borrow() == "boot" {
                    self.boot(cr, r);
                } else {
                    self.login(cr, r);
                }
            }
        }
        cr.restore().ok();
    }

    fn hue(&self) -> f64 {
        let s = self.seed.borrow();
        let s = if s.is_empty() { "x".to_string() } else { s.clone() };
        (s.chars().map(|c| c as u32 as u64).sum::<u64>() % 360) as f64
    }

    fn loading_text(&self, cr: &cairo::Context, r: R, s: f64, alpha: u8) {
        if !self.loading.get() {
            return;
        }
        let f = Font::new(img::pt((4.0 * s).max(7.0)), pango::Weight::Normal);
        let rr = R::new(r.x, r.y, r.w, r.h - 6.0 * s);
        let l = f.layout(cr, &t("sddm.loading"));
        let (_, lh) = l.pixel_size();
        draw::text(cr, &f, R::new(rr.x, rr.bottom() - lh as f64, rr.w, lh as f64), &t("sddm.loading"),
            Rgba::rgb8(255, 255, 255, alpha), HAlign::Center, false);
    }

    /// Bootsplash: schwarzer Grund, Logo-Ring und Lade-Punkte.
    fn boot(&self, cr: &cairo::Context, r: R) {
        draw::fill_rect(cr, r, Rgba::hex("#05070a"));
        let s = r.h / 100.0;
        let (cx, cy) = (r.cx(), r.y + r.h * 0.44);
        let ring = Rgba::hsl(self.hue(), 60.0, 60.0);
        ring.set(cr);
        cr.set_line_width(2.2 * s);
        cr.new_sub_path();
        cr.arc(cx, cy, 11.0 * s, 0.0, std::f64::consts::TAU);
        let _ = cr.stroke();
        Rgba::hex(&self.accent.borrow()).set(cr);
        cr.set_line_cap(cairo::LineCap::Round);
        // Qt: drawArc(start 90°, Spanne -110°) = von oben im Uhrzeigersinn
        let a0 = -std::f64::consts::FRAC_PI_2;
        cr.new_sub_path();
        cr.arc(cx, cy, 11.0 * s, a0, a0 + 110f64.to_radians());
        let _ = cr.stroke();
        for i in 0..5 {
            let a = (230 - i * 40).max(40) as u8;
            draw::fill_ellipse(cr, cx - 8.0 * s + i as f64 * 4.0 * s, cy + 24.0 * s, 1.3 * s, 1.3 * s,
                Rgba::rgb8(255, 255, 255, a));
        }
        self.loading_text(cr, r, s, 120);
    }

    fn login(&self, cr: &cairo::Context, r: R) {
        let base = Rgba::hsl(self.hue(), 70.0, 46.0);
        draw::fill_rect(cr, r, img::darker(base, 260.0));
        let s = r.h / 100.0;
        draw::fill_ellipse(cr, r.x + r.w * 0.2, r.bottom(), r.w * 0.45, r.h * 0.4, img::darker(base, 190.0));
        draw::fill_ellipse(cr, r.right() - r.w * 0.1, r.bottom(), r.w * 0.4, r.h * 0.32, img::darker(base, 160.0));
        let (cx, cy) = (r.cx(), r.y + r.h * 0.42);
        draw::fill_ellipse(cr, cx, cy - 8.0 * s, 9.0 * s, 9.0 * s, Rgba::rgb8(255, 255, 255, 50));
        draw::fill_rounded(cr, R::new(cx - 24.0 * s, cy + 6.0 * s, 48.0 * s, 7.0 * s), 3.5 * s, Rgba::rgb8(0, 0, 0, 110));
        draw::fill_rounded(cr, R::new(cx - 24.0 * s, cy + 16.0 * s, 48.0 * s, 7.0 * s), 3.5 * s,
            Rgba::hex(&self.accent.borrow()));
        draw::fill_rounded(cr, R::new(r.x + 4.0 * s, r.y + 4.0 * s, 14.0 * s, 3.0 * s), 1.5 * s,
            Rgba::rgb8(255, 255, 255, 140));
        self.loading_text(cr, r, s, 150);
    }
}

fn paint_anim(cr: &cairo::Context, r: R, a: &Anim, idx: usize) {
    let bg = Rgba::hex(if a.bg.is_empty() { "#000000" } else { &a.bg });
    if !a.bg2.is_empty() {
        let g = cairo::LinearGradient::new(r.x, r.y, r.x, r.bottom());
        g.add_color_stop_rgba(0.0, bg.0, bg.1, bg.2, bg.3);
        let b2 = Rgba::hex(&a.bg2);
        g.add_color_stop_rgba(1.0, b2.0, b2.1, b2.2, b2.3);
        cr.rectangle(r.x, r.y, r.w, r.h);
        let _ = cr.set_source(&g);
        let _ = cr.fill();
    } else {
        draw::fill_rect(cr, r, bg);
    }
    if let Some(b) = &a.bgimg {
        img::draw_cover(cr, b, r);
    }
    let k = r.w / 1366.0; // Bilder in Originalgröße auf einem 1366 px breiten Bildschirm
    let place = |pm: &cairo::ImageSurface, ha: f64, va: f64, min_h: f64| {
        let (mut w, mut h) = (pm.width() as f64 * k, pm.height() as f64 * k);
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        if min_h > 0.0 && h < r.h * min_h {
            let f = (r.h * min_h / h).min(4.0);
            w *= f;
            h *= f;
        }
        if w > r.w * 0.9 || h > r.h * 0.6 {
            let f = (r.w * 0.9 / w).min(r.h * 0.6 / h);
            w *= f;
            h *= f;
        }
        let x = r.x + (r.w - w) * ha;
        let y = r.y + (r.h - h) * va;
        img::draw_into(cr, pm, R::new(x, y, w, h));
    };
    if let Some(l) = &a.logo {
        place(l, 0.5, a.wva, 0.06);
    }
    if let Some(f) = a.frames.get(idx) {
        place(f, a.ha, a.va, 0.11);
    }
}

/// Kreis als Symbol (Punkt „läuft gerade“ in Listen).
pub fn dot(color: &str, size: i32) -> gtk::DrawingArea {
    let d = gtk::DrawingArea::new();
    d.set_content_width(size);
    d.set_content_height(size);
    d.set_valign(gtk::Align::Center);
    let c = color.to_string();
    d.set_draw_func(move |_, cr, w, h| {
        draw::fill_ellipse(cr, w as f64 / 2.0, h as f64 / 2.0, w as f64 / 2.0, h as f64 / 2.0, Rgba::hex(&c));
    });
    d
}

/// Zahl mit Tausenderpunkten (de) bzw. -kommas (en).
pub fn num(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    let sep = if crate::i18n::lang() == "de" { '.' } else { ',' };
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(sep);
        }
        out.push(ch);
    }
    if n < 0 { format!("-{out}") } else { out }
}
