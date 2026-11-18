//! Gezeichnete Mini-Desktops als Vorschau (keine Screenshots nötig, skaliert scharf).

use super::img::darker;
use crate::catalog::Desktop;
use gtk::cairo;
use gtk::prelude::*;
use hyprstudio::ui::draw::{self, Rgba, R};
use std::cell::RefCell;
use std::rc::Rc;

fn c(hex: &str, alpha: u8) -> Rgba {
    Rgba::hex(hex).alpha(alpha as f64 / 255.0)
}

fn rrect(cr: &cairo::Context, r: R, rad: f64, col: Rgba) {
    if rad <= 0.0 {
        draw::fill_rect(cr, r, col);
    } else {
        draw::fill_rounded(cr, r, rad, col);
    }
}

pub fn paint_desktop(cr: &cairo::Context, r: R, de: &Desktop, variant: &str, radius: f64) {
    cr.save().ok();
    draw::rounded(cr, r, radius);
    cr.clip();

    // Wallpaper: Fläche + zwei Hügel
    let base = Rgba::hex(&de.color);
    draw::fill_rect(cr, r, base);
    let (w, h) = (r.w, r.h);
    for (i, (k, dy)) in [(115.0, 0.62), (135.0, 0.74)].iter().enumerate() {
        let i = i as f64;
        cr.move_to(r.x, r.bottom());
        cr.line_to(r.x, r.y + h * dy);
        cr.curve_to(r.x + w * (0.25 + i * 0.1), r.y + h * (dy - 0.18), r.x + w * (0.55 - i * 0.05),
            r.y + h * (dy + 0.08), r.right(), r.y + h * (dy - 0.1));
        cr.line_to(r.right(), r.bottom());
        cr.close_path();
        darker(base, *k).set(cr);
        let _ = cr.fill();
    }
    draw::fill_ellipse(cr, r.x + w * 0.78, r.y + h * 0.24, h * 0.06, h * 0.06, c("#ffffff", 40));

    let n = match variant {
        "minimal" => 1,
        "full" => 3,
        _ => 2,
    };
    let s = h / 100.0;
    let lay = de.layout.as_str();
    let (mut top, mut bottom) = (0.0, 0.0);

    let bar = |y: f64, height: f64, color: &str, alpha: u8| {
        draw::fill_rect(cr, R::new(r.x, y, w, height), c(color, alpha));
    };
    let dots = |x: f64, y: f64, count: i32, size: f64, gap: f64, color: &str, alpha: u8| {
        for i in 0..count {
            rrect(cr, R::new(x + i as f64 * (size + gap), y, size, size), size * 0.25, c(color, alpha));
        }
    };
    let float_dock = |count: i32, y: f64, size: f64, rad: f64| {
        let width = count as f64 * size * 1.35 + size * 0.5;
        let x = r.x + (w - width) / 2.0;
        rrect(cr, R::new(x, y, width, size * 1.5), size * rad * 1.5, c("#0b0f14", 200));
        dots(x + size * 0.45, y + size * 0.25, count, size, size * 0.35, "#ffffff", 170);
    };

    match lay {
        "gnome" => {
            top = 4.5 * s;
            bar(r.y, top, "#000000", 245);
            rrect(cr, R::new(r.x + 2.0 * s, r.y + 1.4 * s, 6.0 * s, 1.8 * s), s, c("#ffffff", 200));
            rrect(cr, R::new(r.cx() - 4.0 * s, r.y + 1.4 * s, 8.0 * s, 1.8 * s), s, c("#ffffff", 200));
            dots(r.right() - 10.0 * s, r.y + 1.3 * s, 3, 2.0 * s, 1.0 * s, "#ffffff", 170);
            if variant == "full" {
                float_dock(7, r.bottom() - 9.0 * s, 4.5 * s, 0.35);
                bottom = 10.0 * s;
            }
        }
        "plasma" => {
            let ph = 6.5 * s;
            let (x0, y);
            if variant == "minimal" {
                bar(r.bottom() - ph, ph, "#10151c", 240);
                y = r.bottom() - ph;
                x0 = r.x;
            } else {
                let pw = w * if variant == "standard" { 0.62 } else { 0.8 };
                x0 = r.x + (w - pw) / 2.0;
                y = r.bottom() - ph - 1.5 * s;
                rrect(cr, R::new(x0, y, pw, ph), 2.2 * s, c("#10151c", 235));
            }
            draw::fill_ellipse(cr, x0 + 1.6 * s + 3.7 * s / 2.0, y + 1.4 * s + 3.7 * s / 2.0, 3.7 * s / 2.0,
                3.7 * s / 2.0, c("#3d8bfd", 230));
            dots(x0 + 7.5 * s, y + 1.6 * s, 4 + n, 3.3 * s, 1.4 * s, "#ffffff", 170);
            bottom = ph + 2.0 * s;
        }
        "xfce" => {
            top = 4.5 * s;
            bar(r.y, top, "#1c2420", 235);
            dots(r.x + 1.5 * s, r.y + 1.1 * s, 5, 2.3 * s, 1.5 * s, "#ffffff", 170);
            float_dock(4 + n, r.bottom() - 8.0 * s, 3.8 * s, 0.2);
            bottom = 9.0 * s;
        }
        "cinnamon" => {
            let bh = 6.0 * s;
            bar(r.bottom() - bh, bh, "#1b1712", 245);
            rrect(cr, R::new(r.x + 1.5 * s, r.bottom() - bh + 1.3 * s, 3.4 * s, 3.4 * s), s, c("#8fd16a", 220));
            dots(r.x + 7.0 * s, r.bottom() - bh + 1.5 * s, 3 + n, 3.0 * s, 1.5 * s, "#ffffff", 170);
            dots(r.right() - 14.0 * s, r.bottom() - bh + 2.0 * s, 4, 2.0 * s, 1.2 * s, "#ffffff", 170);
            bottom = bh;
        }
        "mate" => {
            top = 4.0 * s;
            bar(r.y, top, "#2c3a2e", 235);
            bar(r.bottom() - 4.0 * s, 4.0 * s, "#2c3a2e", 235);
            dots(r.x + 1.5 * s, r.y + 0.9 * s, 3, 2.2 * s, 2.0 * s, "#ffffff", 170);
            dots(r.x + 1.5 * s, r.bottom() - 3.1 * s, 2 + n, 6.0 * s, 1.0 * s, "#ffffff", 60);
            bottom = 4.0 * s;
        }
        "budgie" => {
            top = 4.0 * s;
            bar(r.y, top, "#100c18", 240);
            dots(r.x + 1.5 * s, r.y + 1.0 * s, 2, 2.0 * s, 1.5 * s, "#ffffff", 170);
            float_dock(3 + n, r.bottom() - 8.0 * s, 3.8 * s, 0.3);
            bottom = 9.0 * s;
            if variant == "full" {
                rrect(cr, R::new(r.right() - 22.0 * s, r.y + top + 2.0 * s, 20.0 * s, h - top - 14.0 * s), 2.0 * s,
                    c("#100c18", 220));
            }
        }
        "lxqt" => {
            let bh = 5.0 * s;
            bar(r.bottom() - bh, bh, "#131a24", 245);
            rrect(cr, R::new(r.x + 1.2 * s, r.bottom() - bh + 1.0 * s, 3.0 * s, 3.0 * s), 0.6 * s, c("#3d8bfd", 220));
            dots(r.x + 6.0 * s, r.bottom() - bh + 1.2 * s, 2 + n, 2.6 * s, 1.2 * s, "#ffffff", 170);
            bottom = bh;
        }
        "cosmic" => {
            top = 4.0 * s;
            bar(r.y, top, "#120d1c", 230);
            dots(r.x + 2.0 * s, r.y + 1.0 * s, 2, 2.0 * s, 2.0 * s, "#ffffff", 170);
            float_dock(4 + n, r.bottom() - 9.0 * s, 4.2 * s, 0.45);
            bottom = 10.0 * s;
        }
        _ => {}
    }

    // Fenster
    if de.tiling {
        let gap = match lay {
            "hyprland" => 2.2,
            "sway" => 1.0,
            _ => 0.0,
        } * s;
        let rad = if lay == "hyprland" { 2.2 * s } else { 0.0 };
        let bh = 4.0 * s;
        let area;
        if lay == "hyprland" {
            rrect(cr, R::new(r.x + gap, r.y + gap, w - 2.0 * gap, bh), 2.0 * s, c("#0b0f14", 210));
            dots(r.x + gap + 2.0 * s, r.y + gap + 1.0 * s, 5, 2.0 * s, 1.2 * s, "#ffffff", 170);
            area = R::new(r.x + gap, r.y + bh + 2.0 * gap, w - 2.0 * gap, h - bh - 3.0 * gap);
        } else {
            bar(r.bottom() - bh, bh, "#000000", 245);
            dots(r.x + 1.0 * s, r.bottom() - bh + 1.0 * s, 5, 2.0 * s, 0.8 * s,
                if lay == "i3" { "#3d8bfd" } else { "#8bd5a0" }, 170);
            area = R::new(r.x + gap, r.y + gap, w - 2.0 * gap, h - bh - 2.0 * gap);
        }
        let cells: Vec<R> = if n == 1 {
            vec![area]
        } else if n == 2 {
            let half = (area.w - gap) / 2.0;
            vec![R::new(area.x, area.y, half, area.h), R::new(area.x + half + gap, area.y, half, area.h)]
        } else {
            let half = (area.w - gap) / 2.0;
            let hh = (area.h - gap) / 2.0;
            vec![R::new(area.x, area.y, half, area.h), R::new(area.x + half + gap, area.y, half, hh),
                 R::new(area.x + half + gap, area.y + hh + gap, half, hh)]
        };
        let border = match lay {
            "hyprland" => "#33ccff",
            "sway" => "#4c7899",
            _ => "#285577",
        };
        for (i, cell) in cells.iter().enumerate() {
            let fill = c("#0d1117", if lay != "hyprland" { 235 } else { 215 });
            if rad > 0.0 {
                draw::rounded(cr, *cell, rad);
            } else {
                cr.rectangle(cell.x, cell.y, cell.w, cell.h);
            }
            fill.set(cr);
            let _ = cr.fill_preserve();
            c(if i == 0 { border } else { "#30363d" }, 255).set(cr);
            cr.set_line_width((0.5 * s).max(1.0));
            let _ = cr.stroke();
            lines(cr, cell.adj(2.0 * s, 2.0 * s, -2.0 * s, -2.0 * s), s, 4 + i as i32);
        }
    } else {
        let area = R::new(r.x, r.y + top, w, h - top - bottom);
        let rects = [(0.10, 0.14, 0.52, 0.66), (0.42, 0.08, 0.48, 0.58), (0.26, 0.38, 0.44, 0.52)];
        for i in 0..n as usize {
            let q = rects[i];
            let cell = R::new(area.x + area.w * q.0, area.y + area.h * q.1, area.w * q.2, area.h * q.3);
            window(cr, cell, s, lay, i == n as usize - 1);
        }
    }
    cr.restore().ok();
}

fn lines(cr: &cairo::Context, cl: R, s: f64, count: i32) {
    for i in 0..count {
        let y = cl.y + i as f64 * 3.0 * s;
        if y + 1.4 * s > cl.bottom() {
            break;
        }
        let f = 0.4 + 0.5 * ((i * 37) % 10) as f64 / 10.0;
        rrect(cr, R::new(cl.x, y, cl.w * f, 1.4 * s), 0.7 * s, c("#ffffff", 45));
    }
}

fn window(cr: &cairo::Context, cl: R, s: f64, lay: &str, active: bool) {
    let rad = match lay {
        "gnome" | "cosmic" => 2.6,
        "plasma" | "budgie" => 1.8,
        _ => 1.0,
    } * s;
    rrect(cr, cl.translate(0.0, 0.8 * s), rad, c("#000000", 70));
    rrect(cr, cl, rad, c(if active { "#1e252e" } else { "#171d24" }, 255));
    let tb = 4.2 * s;
    let head_col = c(if active { "#262f3a" } else { "#1d242c" }, 255);
    cr.save().ok();
    draw::rounded(cr, R::new(cl.x, cl.y, cl.w, tb), rad);
    cr.rectangle(cl.x, cl.y + tb / 2.0, cl.w, tb / 2.0);
    cr.set_fill_rule(cairo::FillRule::Winding);
    head_col.set(cr);
    let _ = cr.fill();
    cr.restore().ok();
    // Fensterknöpfe
    let d = 1.6 * s;
    let y = cl.y + (tb - d) / 2.0;
    if matches!(lay, "gnome" | "cosmic" | "budgie") {
        draw::fill_ellipse(cr, cl.right() - 3.0 * s + d / 2.0, y + d / 2.0, d / 2.0, d / 2.0, c("#ffffff", 90));
    } else {
        let cols = if lay != "plasma" { ["#ffffff", "#ffffff", "#e5534b"] } else { ["#ffffff", "#ffffff", "#ffffff"] };
        for (i, col) in cols.iter().enumerate() {
            let x = cl.right() - (3.0 + i as f64 * 2.8) * s;
            draw::fill_ellipse(cr, x + d / 2.0, y + d / 2.0, d / 2.0, d / 2.0, c(col, 110));
        }
    }
    lines(cr, cl.adj(2.5 * s, tb + 2.0 * s, -2.5 * s, -2.0 * s), s, 6);
}

/// Vorschau-Fläche (16:10, mittig).
pub struct Preview {
    pub area: gtk::DrawingArea,
    de: RefCell<Option<&'static Desktop>>,
    variant: RefCell<String>,
    radius: f64,
}

impl Preview {
    pub fn new(radius: f64) -> Rc<Preview> {
        let area = gtk::DrawingArea::new();
        area.set_size_request(160, 100);
        let p = Rc::new(Preview { area, de: RefCell::new(None), variant: RefCell::new("standard".into()), radius });
        let w = Rc::downgrade(&p);
        p.area.set_draw_func(move |_, cr, wd, ht| {
            let Some(p) = w.upgrade() else { return };
            let Some(de) = *p.de.borrow() else { return };
            let mut r = R::new(0.0, 0.0, wd as f64, ht as f64);
            let target_h = r.w * 10.0 / 16.0;
            if target_h > r.h {
                let tw = r.h * 16.0 / 10.0;
                r = R::new(r.x + (r.w - tw) / 2.0, r.y, tw, r.h);
            } else {
                r = R::new(r.x, r.y + (r.h - target_h) / 2.0, r.w, target_h);
            }
            paint_desktop(cr, r, de, &p.variant.borrow(), p.radius);
        });
        p
    }

    pub fn set(&self, de: &'static Desktop, variant: &str) {
        *self.de.borrow_mut() = Some(de);
        *self.variant.borrow_mut() = variant.into();
        self.area.queue_draw();
    }
}
