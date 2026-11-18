//! Bilder laden und mit Cairo zeichnen (statt QPixmap/QImage): füllend zuschneiden, einpassen, verkleinern.

use gtk::gdk_pixbuf::{InterpType, Pixbuf, PixbufLoader};
use gtk::prelude::*;
use gtk::{cairo, gdk};
use hyprstudio::ui::draw::R;
use std::path::Path;

/// Bild aus Datei (verkleinert, wenn die längere Seite größer als `limit` ist; 0 = nie).
pub fn load(path: &Path, limit: i32) -> Option<Pixbuf> {
    let pb = Pixbuf::from_file(path).ok()?;
    Some(shrink(pb, limit))
}

/// Bild aus Bytes (PNG, JPG, GIF – erstes Bild, SVG …).
pub fn from_bytes(data: &[u8]) -> Option<Pixbuf> {
    let l = PixbufLoader::new();
    l.write(data).ok()?;
    l.close().ok()?;
    l.pixbuf()
}

/// SVG/Bild in fester Größe (z. B. Programmsymbol).
pub fn from_bytes_at(data: &[u8], w: i32, h: i32) -> Option<Pixbuf> {
    let l = PixbufLoader::new();
    l.set_size(w, h);
    l.write(data).ok()?;
    l.close().ok()?;
    l.pixbuf()
}

pub fn shrink(pb: Pixbuf, limit: i32) -> Pixbuf {
    let (w, h) = (pb.width(), pb.height());
    if limit <= 0 || w.max(h) <= limit {
        return pb;
    }
    let f = limit as f64 / w.max(h) as f64;
    let (nw, nh) = (((w as f64) * f).round().max(1.0) as i32, ((h as f64) * f).round().max(1.0) as i32);
    pb.scale_simple(nw, nh, InterpType::Bilinear).unwrap_or(pb)
}

/// Auf Breite verkleinern (wie scaledToWidth).
pub fn to_width(pb: Pixbuf, width: i32) -> Pixbuf {
    if pb.width() <= width {
        return pb;
    }
    let nh = ((pb.height() as f64) * width as f64 / pb.width() as f64).round().max(1.0) as i32;
    pb.scale_simple(width, nh, InterpType::Bilinear).unwrap_or(pb)
}

/// Pixbuf → Cairo-Fläche (einmal umwandeln, dann schnell zeichnen).
pub fn surface(pb: &Pixbuf) -> Option<cairo::ImageSurface> {
    let s = cairo::ImageSurface::create(cairo::Format::ARgb32, pb.width(), pb.height()).ok()?;
    {
        let cr = cairo::Context::new(&s).ok()?;
        cr.set_source_pixbuf(pb, 0.0, 0.0);
        cr.paint().ok()?;
    }
    s.flush();
    Some(s)
}

/// Quellrechteck `src` (Bildpixel) in Zielrechteck `dst` zeichnen.
pub fn draw_part(cr: &cairo::Context, s: &cairo::ImageSurface, dst: R, src: R) {
    if src.w <= 0.0 || src.h <= 0.0 {
        return;
    }
    cr.save().ok();
    cr.rectangle(dst.x, dst.y, dst.w, dst.h);
    cr.clip();
    cr.translate(dst.x, dst.y);
    cr.scale(dst.w / src.w, dst.h / src.h);
    let _ = cr.set_source_surface(s, -src.x, -src.y);
    cr.source().set_filter(cairo::Filter::Good);
    let _ = cr.paint();
    cr.restore().ok();
}

/// Ganzes Bild in `dst` (verzerrt nicht, wenn `dst` das Seitenverhältnis hat).
pub fn draw_into(cr: &cairo::Context, s: &cairo::ImageSurface, dst: R) {
    draw_part(cr, s, dst, R::new(0.0, 0.0, s.width() as f64, s.height() as f64));
}

/// Füllend zuschneiden (wie scale = max(...), Mitte).
pub fn draw_cover(cr: &cairo::Context, s: &cairo::ImageSurface, r: R) {
    let (pw, ph) = (s.width() as f64, s.height() as f64);
    if pw <= 0.0 || ph <= 0.0 {
        return;
    }
    let scale = (r.w / pw).max(r.h / ph);
    let (sw, sh) = (r.w / scale, r.h / scale);
    draw_part(cr, s, r, R::new((pw - sw) / 2.0, (ph - sh) / 2.0, sw, sh));
}

/// Einpassen (KeepAspectRatio) und mittig zeichnen.
pub fn draw_fit(cr: &cairo::Context, s: &cairo::ImageSurface, r: R) {
    let (pw, ph) = (s.width() as f64, s.height() as f64);
    if pw <= 0.0 || ph <= 0.0 {
        return;
    }
    let f = (r.w / pw).min(r.h / ph);
    let (w, h) = (pw * f, ph * f);
    draw_into(cr, s, R::new(r.x + (r.w - w) / 2.0, r.y + (r.h - h) / 2.0, w, h));
}

/// Pixbuf als Textur (für gtk::Picture/Image).
pub fn texture(pb: &Pixbuf) -> gdk::Texture {
    gdk::Texture::for_pixbuf(pb)
}

/// QColor.darker(k): Helligkeit (HSV-V) durch k/100 teilen.
pub fn darker(c: hyprstudio::ui::draw::Rgba, k: f64) -> hyprstudio::ui::draw::Rgba {
    let f = 100.0 / k;
    hyprstudio::ui::draw::Rgba(c.0 * f, c.1 * f, c.2 * f, c.3)
}

/// Qt-Punktgröße → Pixel (96 dpi).
pub fn pt(p: f64) -> f64 {
    p * 96.0 / 72.0
}

/// Bilddaten, die zwischen Threads wandern dürfen (Pixbuf selbst darf das nicht).
pub struct Raw {
    data: gtk::glib::Bytes,
    alpha: bool,
    w: i32,
    h: i32,
    stride: i32,
}

impl Raw {
    pub fn from(pb: &Pixbuf) -> Raw {
        Raw { data: pb.read_pixel_bytes(), alpha: pb.has_alpha(), w: pb.width(), h: pb.height(), stride: pb.rowstride() }
    }

    pub fn pixbuf(&self) -> Pixbuf {
        Pixbuf::from_bytes(&self.data, gtk::gdk_pixbuf::Colorspace::Rgb, self.alpha, 8, self.w, self.h, self.stride)
    }
}

/// Im Hintergrund laden (verkleinert) → `Raw`.
pub fn load_raw(path: &Path, limit: i32) -> Option<Raw> {
    load(path, limit).map(|pb| Raw::from(&pb))
}
