//! Zeichnen mit Cairo/Pango (statt QPainter): Farben, abgerundete Rechtecke, Text, Symbole, Texturen.

use crate::model;
use gtk::{cairo, gdk, glib, pango};
use std::f64::consts::PI;

/// Farbe aus "#rrggbb[aa]" (wie `qcolor`), Alpha zusätzlich multiplizierbar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba(pub f64, pub f64, pub f64, pub f64);

impl Rgba {
    pub fn hex(c: &str) -> Rgba {
        let h = model::norm(c);
        let h = h.trim_start_matches('#');
        let p = |i: usize| u8::from_str_radix(h.get(i..i + 2).unwrap_or("ff"), 16).unwrap_or(255) as f64 / 255.0;
        Rgba(p(0), p(2), p(4), p(6))
    }
    pub fn rgb8(r: u8, g: u8, b: u8, a: u8) -> Rgba {
        Rgba(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0, a as f64 / 255.0)
    }
    pub fn alpha(self, a: f64) -> Rgba {
        Rgba(self.0, self.1, self.2, a.clamp(0.0, 1.0))
    }
    pub fn mul_alpha(self, m: f64) -> Rgba {
        Rgba(self.0, self.1, self.2, (self.3 * m).clamp(0.0, 1.0))
    }
    pub fn a8(self) -> u8 {
        (self.3 * 255.0).round() as u8
    }
    /// QColor.fromHsl(h, s, l) mit Werten 0..359 / 0..255.
    pub fn hsl(h: f64, s: f64, l: f64) -> Rgba {
        let (s, l) = (s / 255.0, l / 255.0);
        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let hp = (h % 360.0) / 60.0;
        let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
        let (r, g, b) = match hp as i32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        let m = l - c / 2.0;
        Rgba(r + m, g + m, b + m, 1.0)
    }
    pub fn set(self, cr: &cairo::Context) {
        cr.set_source_rgba(self.0, self.1, self.2, self.3);
    }
    pub fn to_hex(self) -> String {
        let b = |x: f64| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
        format!("#{:02x}{:02x}{:02x}{:02x}", b(self.0), b(self.1), b(self.2), b(self.3))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct R {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl R {
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> R {
        R { x, y, w, h }
    }
    pub fn right(&self) -> f64 {
        self.x + self.w
    }
    pub fn bottom(&self) -> f64 {
        self.y + self.h
    }
    pub fn cx(&self) -> f64 {
        self.x + self.w / 2.0
    }
    pub fn cy(&self) -> f64 {
        self.y + self.h / 2.0
    }
    /// QRectF.adjusted(dx1, dy1, dx2, dy2)
    pub fn adj(&self, a: f64, b: f64, c: f64, d: f64) -> R {
        R::new(self.x + a, self.y + b, self.w - a + c, self.h - b + d)
    }
    pub fn translate(&self, dx: f64, dy: f64) -> R {
        R::new(self.x + dx, self.y + dy, self.w, self.h)
    }
}

/// Abgerundetes Rechteck als Pfad (wie QPainterPath.addRoundedRect).
pub fn rounded(cr: &cairo::Context, r: R, rad: f64) {
    let rad = rad.max(0.0).min(r.w / 2.0).min(r.h / 2.0);
    cr.new_sub_path();
    if rad <= 0.0 {
        cr.rectangle(r.x, r.y, r.w, r.h);
        return;
    }
    cr.arc(r.right() - rad, r.y + rad, rad, -PI / 2.0, 0.0);
    cr.arc(r.right() - rad, r.bottom() - rad, rad, 0.0, PI / 2.0);
    cr.arc(r.x + rad, r.bottom() - rad, rad, PI / 2.0, PI);
    cr.arc(r.x + rad, r.y + rad, rad, PI, 1.5 * PI);
    cr.close_path();
}

pub fn fill_rounded(cr: &cairo::Context, r: R, rad: f64, c: Rgba) {
    rounded(cr, r, rad);
    c.set(cr);
    let _ = cr.fill();
}

pub fn fill_rect(cr: &cairo::Context, r: R, c: Rgba) {
    cr.rectangle(r.x, r.y, r.w, r.h);
    c.set(cr);
    let _ = cr.fill();
}

pub fn ellipse(cr: &cairo::Context, cx: f64, cy: f64, rx: f64, ry: f64) {
    if rx <= 0.0 || ry <= 0.0 {
        return;
    }
    cr.save().ok();
    cr.translate(cx, cy);
    cr.scale(rx, ry);
    cr.new_sub_path();
    cr.arc(0.0, 0.0, 1.0, 0.0, 2.0 * PI);
    cr.restore().ok();
}

pub fn fill_ellipse(cr: &cairo::Context, cx: f64, cy: f64, rx: f64, ry: f64, c: Rgba) {
    ellipse(cr, cx, cy, rx, ry);
    c.set(cr);
    let _ = cr.fill();
}

// ---------- Text ----------

/// Schrift der Oberfläche (wie die Standardschrift der Qt-App), für Text ohne eigene Schriftart.
pub fn ui_family() -> String {
    thread_local! {
        static FAM: std::cell::OnceCell<String> = const { std::cell::OnceCell::new() };
    }
    FAM.with(|f| f.get_or_init(|| {
        let name = gtk::Settings::default().and_then(|s| s.gtk_font_name()).map(|s| s.to_string()).unwrap_or_default();
        let fd = pango::FontDescription::from_string(&name);
        fd.family().map(|f| f.to_string()).filter(|f| !f.is_empty()).unwrap_or_else(|| "Sans".into())
    }).clone())
}

pub struct Font {
    pub family: String,
    pub px: f64,
    pub weight: pango::Weight,
}

impl Font {
    pub fn new(px: f64, weight: pango::Weight) -> Font {
        Font { family: String::new(), px, weight }
    }
    pub fn layout(&self, cr: &cairo::Context, text: &str) -> pango::Layout {
        let l = pangocairo::functions::create_layout(cr);
        let mut fd = pango::FontDescription::new();
        if !self.family.is_empty() {
            fd.set_family(&self.family);
        } else {
            fd.set_family(&ui_family());
        }
        fd.set_absolute_size(self.px.max(1.0) * pango::SCALE as f64);
        fd.set_weight(self.weight);
        l.set_font_description(Some(&fd));
        l.set_text(text);
        l
    }
    /// Zeilenhöhe (wie QFontMetricsF.height()).
    pub fn height(&self, cr: &cairo::Context) -> f64 {
        let l = self.layout(cr, "Hg");
        l.pixel_size().1 as f64
    }
    pub fn advance(&self, cr: &cairo::Context, text: &str) -> f64 {
        if text.is_empty() {
            return 0.0;
        }
        let l = self.layout(cr, text);
        let (_, log) = l.extents();
        log.width() as f64 / pango::SCALE as f64
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum HAlign {
    Left,
    Center,
    Right,
}

/// Text in ein Rechteck zeichnen (senkrecht mittig wie AlignVCenter), optional abgeschnitten mit „…“.
pub fn text(cr: &cairo::Context, font: &Font, r: R, s: &str, c: Rgba, h: HAlign, elide: bool) {
    if s.is_empty() {
        return;
    }
    let l = font.layout(cr, s);
    if elide {
        l.set_width((r.w.max(1.0) * pango::SCALE as f64) as i32);
        l.set_ellipsize(pango::EllipsizeMode::End);
    }
    let (tw, th) = l.pixel_size();
    let x = match h {
        HAlign::Left => r.x,
        HAlign::Center => r.x + (r.w - tw as f64) / 2.0,
        HAlign::Right => r.right() - tw as f64,
    };
    let y = r.y + (r.h - th as f64) / 2.0;
    cr.save().ok();
    c.set(cr);
    cr.move_to(x, y);
    pangocairo::functions::show_layout(cr, &l);
    cr.restore().ok();
}

/// Text oben ausgerichtet (wie AlignTop).
pub fn text_top(cr: &cairo::Context, font: &Font, r: R, s: &str, c: Rgba, h: HAlign) {
    let l = font.layout(cr, s);
    l.set_width((r.w.max(1.0) * pango::SCALE as f64) as i32);
    l.set_ellipsize(pango::EllipsizeMode::End);
    l.set_alignment(match h {
        HAlign::Left => pango::Alignment::Left,
        HAlign::Center => pango::Alignment::Center,
        HAlign::Right => pango::Alignment::Right,
    });
    cr.save().ok();
    c.set(cr);
    cr.move_to(r.x, r.y);
    pangocairo::functions::show_layout(cr, &l);
    cr.restore().ok();
}

// ---------- Texturen ----------

/// In eine Textur zeichnen (Pixelgröße w×h · scale), z. B. für Preset-Bilder und Symbole.
pub fn texture(w: i32, h: i32, scale: f64, f: impl FnOnce(&cairo::Context)) -> gdk::Texture {
    let pw = ((w as f64) * scale).round().max(1.0) as i32;
    let ph = ((h as f64) * scale).round().max(1.0) as i32;
    let mut surf = cairo::ImageSurface::create(cairo::Format::ARgb32, pw, ph).expect("surface");
    {
        let cr = cairo::Context::new(&surf).expect("cairo");
        cr.scale(scale, scale);
        f(&cr);
    }
    surf.flush();
    let stride = surf.stride() as usize;
    let data = surf.data().expect("data").to_vec();
    let bytes = glib::Bytes::from_owned(data);
    let fmt = if cfg!(target_endian = "little") {
        gdk::MemoryFormat::B8g8r8a8Premultiplied
    } else {
        gdk::MemoryFormat::A8r8g8b8Premultiplied
    };
    gdk::MemoryTexture::new(pw, ph, fmt, &bytes, stride).upcast()
}

use gtk::prelude::*;

// ---------- gezeichnete Symbole (wie draw_icon) ----------

fn pen(cr: &cairo::Context, c: Rgba, w: f64) {
    c.set(cr);
    cr.set_line_width(w);
    cr.set_line_cap(cairo::LineCap::Round);
    cr.set_line_join(cairo::LineJoin::Round);
}

fn line(cr: &cairo::Context, x1: f64, y1: f64, x2: f64, y2: f64) {
    cr.move_to(x1, y1);
    cr.line_to(x2, y2);
    let _ = cr.stroke();
}

pub fn paint_icon(cr: &cairo::Context, kind: &str, color: &str, size: f64) {
    let col = Rgba::hex(color);
    pen(cr, col, 1.8);
    let c = size / 2.0;
    match kind {
        "gear" => {
            let (teeth, ro, ri) = (8, c * 0.9, c * 0.66);
            let n = teeth * 4;
            for i in 0..n {
                let a = (i as f64 / n as f64) * 2.0 * PI - PI / 2.0;
                let rad = if matches!(i % 4, 1 | 2) { ro } else { ri };
                let (x, y) = (c + rad * a.cos(), c + rad * a.sin());
                if i == 0 { cr.move_to(x, y) } else { cr.line_to(x, y) }
            }
            cr.close_path();
            cr.new_sub_path();
            cr.arc_negative(c, c, c * 0.28, 0.0, -2.0 * PI);
            cr.set_fill_rule(cairo::FillRule::EvenOdd);
            col.set(cr);
            let _ = cr.fill();
        }
        "close" => {
            let m = size * 0.3;
            line(cr, m, m, size - m, size - m);
            line(cr, size - m, m, m, size - m);
        }
        "min" => {
            let m = size * 0.3;
            line(cr, m, c, size - m, c);
        }
        "max" => {
            let m = size * 0.3;
            rounded(cr, R::new(m, m, size - 2.0 * m, size - 2.0 * m), 1.5);
            let _ = cr.stroke();
        }
        "dots" => {
            for x in [0.25, 0.5, 0.75] {
                fill_ellipse(cr, size * x, c, size * 0.08, size * 0.08, col);
            }
        }
        "play" => {
            cr.move_to(size * 0.32, size * 0.22);
            cr.line_to(size * 0.8, c);
            cr.line_to(size * 0.32, size * 0.78);
            cr.close_path();
            let _ = cr.fill();
        }
        "reset" => {
            let rad = size * 0.31;
            let start = 70f64.to_radians();
            cr.new_sub_path();
            cr.arc_negative(c, c, rad, -start, -start - 300f64.to_radians());
            let _ = cr.stroke();
            let tip = (c + rad * start.cos(), c - rad * start.sin());
            let t = (start.sin(), start.cos());
            let nd = (start.cos(), -start.sin());
            let h = size * 0.2;
            let base = (tip.0 - t.0 * h * 0.2, tip.1 - t.1 * h * 0.2);
            cr.move_to(base.0 + t.0 * h, base.1 + t.1 * h);
            cr.line_to(base.0 + nd.0 * h * 0.62, base.1 + nd.1 * h * 0.62);
            cr.line_to(base.0 - nd.0 * h * 0.62, base.1 - nd.1 * h * 0.62);
            cr.close_path();
            cr.set_line_width(1.0);
            let _ = cr.fill_preserve();
            let _ = cr.stroke();
        }
        "chev" => {
            let m = size * 0.3;
            line(cr, m, c - size * 0.1, c, c + size * 0.12);
            line(cr, c, c + size * 0.12, size - m, c - size * 0.1);
        }
        "chev_r" => {
            let m = size * 0.3;
            line(cr, c - size * 0.1, m, c + size * 0.12, c);
            line(cr, c + size * 0.12, c, c - size * 0.1, size - m);
        }
        "upload" => {
            line(cr, c, size * 0.18, c, size * 0.64);
            line(cr, c - size * 0.2, size * 0.37, c, size * 0.18);
            line(cr, c + size * 0.2, size * 0.37, c, size * 0.18);
            line(cr, size * 0.22, size * 0.82, size * 0.78, size * 0.82);
        }
        "download" => {
            line(cr, c, size * 0.18, c, size * 0.64);
            line(cr, c - size * 0.2, size * 0.45, c, size * 0.64);
            line(cr, c + size * 0.2, size * 0.45, c, size * 0.64);
            line(cr, size * 0.22, size * 0.82, size * 0.78, size * 0.82);
        }
        "check" => {
            line(cr, size * 0.22, c + size * 0.02, size * 0.42, size * 0.7);
            line(cr, size * 0.42, size * 0.7, size * 0.8, size * 0.3);
        }
        "search" => {
            cr.new_sub_path();
            cr.arc(c - 1.5, c - 1.5, c * 0.42, 0.0, 2.0 * PI);
            let _ = cr.stroke();
            line(cr, c + c * 0.25, c + c * 0.25, size * 0.8, size * 0.8);
        }
        "plus" => {
            let m = size * 0.28;
            line(cr, m, c, size - m, c);
            line(cr, c, m, c, size - m);
        }
        "trash" => {
            let m = size * 0.26;
            line(cr, m - 1.0, size * 0.3, size - m + 1.0, size * 0.3);
            line(cr, c - size * 0.08, size * 0.2, c + size * 0.08, size * 0.2);
            rounded(cr, R::new(m + 1.0, size * 0.3, size - 2.0 * m - 2.0, size * 0.5), 1.5);
            let _ = cr.stroke();
        }
        _ => {}
    }
}

/// Symbol als Textur (doppelte Auflösung wie im Original).
pub fn icon(kind: &str, color: &str, size: i32) -> gdk::Texture {
    let kind = kind.to_string();
    let color = color.to_string();
    texture(size, size, 2.0, move |cr| paint_icon(cr, &kind, &color, size as f64))
}

/// Bild-Widget mit gezeichnetem Symbol.
pub fn icon_image(kind: &str, color: &str, size: i32) -> gtk::Image {
    let img = gtk::Image::from_paintable(Some(&icon(kind, color, size)));
    img.set_pixel_size(size);
    img
}

// ---------- Textur in logischer Größe ----------

mod imp {
    use gtk::subclass::prelude::*;
    use gtk::{gdk, glib};
    use std::cell::{Cell, RefCell};

    #[derive(Default)]
    pub struct Logical {
        pub tex: RefCell<Option<gdk::Texture>>,
        pub w: Cell<i32>,
        pub h: Cell<i32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Logical {
        const NAME: &'static str = "HsLogicalPaintable";
        type Type = super::Logical;
        type Interfaces = (gdk::Paintable,);
    }

    impl ObjectImpl for Logical {}

    impl gdk::subclass::paintable::PaintableImpl for Logical {
        fn intrinsic_width(&self) -> i32 {
            self.w.get()
        }
        fn intrinsic_height(&self) -> i32 {
            self.h.get()
        }
        fn snapshot(&self, snapshot: &gdk::Snapshot, width: f64, height: f64) {
            if let Some(t) = self.tex.borrow().as_ref() {
                use gtk::prelude::*;
                t.snapshot(snapshot, width, height);
            }
        }
    }
}

glib::wrapper! {
    pub struct Logical(ObjectSubclass<imp::Logical>) @implements gdk::Paintable;
}

/// Hochaufgelöste Textur (z. B. ×2) mit logischer Größe w×h – damit `gtk::Picture` nicht doppelt so groß wird.
pub fn logical(tex: &gdk::Texture, w: i32, h: i32) -> Logical {
    use gtk::subclass::prelude::ObjectSubclassIsExt;
    let p: Logical = glib::Object::new();
    *p.imp().tex.borrow_mut() = Some(tex.clone());
    p.imp().w.set(w);
    p.imp().h.set(h);
    p
}
