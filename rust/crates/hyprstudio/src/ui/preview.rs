//! Live-Vorschau: gezeichneter Hyprland-Desktop mit Waybar, Lücken, Rahmen, Unschärfe, Schatten und Animation.

use super::draw::{self, Font, HAlign, Rgba, R};
use crate::jsonx::{Get, Obj};
use crate::model;
use gtk::prelude::*;
use gtk::{cairo, gdk, glib, pango};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::f64::consts::PI;
use std::rc::Rc;

pub const VW: f64 = 1280.0;
pub const VH: f64 = 720.0;

/// Kubische Bezierkurve (0,0)-p1-p2-(1,1): y zu gegebenem x (Bisektion).
pub fn bezier_y(p1: (f64, f64), p2: (f64, f64), x: f64) -> f64 {
    let bx = |t: f64| 3.0 * (1.0 - t).powi(2) * t * p1.0 + 3.0 * (1.0 - t) * t * t * p2.0 + t.powi(3);
    let by = |t: f64| 3.0 * (1.0 - t).powi(2) * t * p1.1 + 3.0 * (1.0 - t) * t * t * p2.1 + t.powi(3);
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..30 {
        let mid = (lo + hi) / 2.0;
        if bx(mid) < x { lo = mid } else { hi = mid }
    }
    by((lo + hi) / 2.0)
}

thread_local! {
    static WALL: RefCell<HashMap<(i32, i32), (cairo::ImageSurface, cairo::ImageSurface)>> = RefCell::new(HashMap::new());
}

fn make_wall(w: i32, h: i32) -> cairo::ImageSurface {
    let s = cairo::ImageSurface::create(cairo::Format::ARgb32, w, h).unwrap();
    {
        let cr = cairo::Context::new(&s).unwrap();
        let (wf, hf) = (w as f64, h as f64);
        draw::fill_rect(&cr, R::new(0.0, 0.0, wf, hf), Rgba::hex("#24345e"));
        for (i, col) in ["#2b3f70", "#344a80", "#40558c", "#4d5f94"].iter().enumerate() {
            draw::fill_rect(&cr, R::new(0.0, hf * (0.12 + i as f64 * 0.1), wf, hf), Rgba::hex(col));
        }
        draw::fill_ellipse(&cr, wf * 0.78, hf * 0.24, hf * 0.08, hf * 0.08, Rgba::rgb8(240, 236, 220, 240));
        let waves = [("#5a5d9e", 0.55, 0.10, 2.2), ("#47508c", 0.66, 0.08, 3.1), ("#333d70", 0.78, 0.07, 4.3),
            ("#1f2849", 0.9, 0.05, 5.7)];
        for (i, (col, base, amp, freq)) in waves.iter().enumerate() {
            let i = i as f64;
            cr.move_to(0.0, hf);
            let mut x = 0;
            while x < w + 8 {
                let xf = x as f64;
                let y = hf * (base - amp * (xf / wf * freq * PI + i * 1.7).sin() * (0.6 + 0.4 * (xf / wf * 7.0 + i).cos()));
                cr.line_to(xf, y);
                x += 8;
            }
            cr.line_to(wf, hf);
            cr.close_path();
            Rgba::hex(col).set(&cr);
            let _ = cr.fill();
        }
        let star = Rgba::rgb8(255, 255, 255, 140);
        for i in 0..40 {
            let x = (i * 97 % w.max(1)) as f64;
            let y = (i * 53 % ((h as f64 * 0.4) as i32).max(1)) as f64;
            draw::fill_ellipse(&cr, x, y, 0.9, 0.9, star);
        }
    }
    s
}

/// Hintergrund und seine unscharfe Fassung (klein gerechnet und wieder vergrößert).
pub fn wallpaper(w: i32, h: i32) -> (cairo::ImageSurface, cairo::ImageSurface) {
    WALL.with(|c| {
        if let Some(x) = c.borrow().get(&(w, h)) {
            return x.clone();
        }
        let wall = make_wall(w, h);
        let sw = (w / 14).max(1);
        let sh = (h / 14).max(1);
        let small = cairo::ImageSurface::create(cairo::Format::ARgb32, sw, sh).unwrap();
        {
            let cr = cairo::Context::new(&small).unwrap();
            cr.scale(sw as f64 / w as f64, sh as f64 / h as f64);
            cr.set_source_surface(&wall, 0.0, 0.0).ok();
            cr.source().set_filter(cairo::Filter::Good);
            let _ = cr.paint();
        }
        let blur = cairo::ImageSurface::create(cairo::Format::ARgb32, w, h).unwrap();
        {
            let cr = cairo::Context::new(&blur).unwrap();
            cr.scale(w as f64 / sw as f64, h as f64 / sh as f64);
            cr.set_source_surface(&small, 0.0, 0.0).ok();
            cr.source().set_filter(cairo::Filter::Bilinear);
            cr.source().set_extend(cairo::Extend::Pad);
            let _ = cr.paint();
        }
        let mut m = c.borrow_mut();
        if m.len() > 12 {
            m.clear();
        }
        m.insert((w, h), (wall.clone(), blur.clone()));
        (wall, blur)
    })
}

pub fn mod_text(m: &str, ws: &Obj) -> String {
    match m {
        "clock" => {
            let h = if ws.b("clock24") { "14:32" } else { "2:32 PM" };
            match ws.s("clock_fmt") {
                "hms" => h.replacen("32", "32:05", 1),
                "dhm" => format!("Mo {}", h),
                "full" => format!("18.10.2026  {}", h),
                _ => h.into(),
            }
        }
        "network" => match ws.s("net_fmt") {
            "ip" => "192.168.1.20".into(),
            "signal" => "78%".into(),
            _ => "WLAN".into(),
        },
        "hyprland/window" => "kitty".into(),
        "pulseaudio" => "60%".into(),
        "battery" => "87%".into(),
        "backlight" => "70%".into(),
        "bluetooth" => "BT".into(),
        "cpu" => "12%".into(),
        "memory" => "41%".into(),
        "temperature" => "48°".into(),
        "disk" => "37%".into(),
        "mpris" => "Artist – Song".into(),
        "hyprland/language" => "DE".into(),
        _ => String::new(),
    }
}

fn weight(ws: &Obj) -> pango::Weight {
    match ws.s("font_weight") {
        "bold" => pango::Weight::Bold,
        "600" => pango::Weight::Semibold,
        _ => pango::Weight::Normal,
    }
}

/// Zeichnet die Waybar, gibt die belegte Höhe (virtuell) zurück.
fn bar(cr: &cairo::Context, r: R, k: f64, ws: &Obj) -> f64 {
    let h = ws.n("height");
    let style = ws.s("style");
    let m = if style != "bar" { ws.n("margin") } else { 0.0 };
    let top = ws.s("position") == "top";
    let y = if top { r.y + m * k } else { r.bottom() - (m + h) * k };
    let barr = R::new(r.x + (m * 2.0) * k, y, r.w - (m * 4.0) * k, h * k);
    let rad = ws.n("radius") * k;
    let (bg, fg, acc) = (Rgba::hex(ws.s("bg")), Rgba::hex(ws.s("fg")), Rgba::hex(ws.s("accent")));
    let bw = ws.n("border_width");
    let border = Rgba::hex(ws.s("border"));
    let pen_w = (bw * k).max(1.0);
    let draw_box = |rr: R, radius: f64| {
        draw::rounded(cr, rr, radius);
        bg.set(cr);
        let _ = cr.fill_preserve();
        if bw > 0.0 {
            border.set(cr);
            cr.set_line_width(pen_w);
            let _ = cr.stroke();
        } else {
            cr.new_path();
        }
    };
    if style != "islands" {
        draw_box(barr, if style == "float" { rad } else { 0.0 });
    }
    let mut font = Font::new((ws.n("font_size") * k).round().max(3.0), weight(ws));
    font.family = String::new();
    let fh = font.height(cr);
    let pad = (ws.n("pad") + if style == "islands" { 2.0 } else { 0.0 }) * k;
    let n_ws = ws.i("ws_persist").max(4);
    let gap = if style == "islands" { 6.0 } else { ws.n("spacing") } * k;
    let icons = ws.b("icons");
    let four = font.advance(cr, "4");
    let width = |m: &str| -> f64 {
        match m {
            "hyprland/workspaces" => n_ws as f64 * (four + 14.0 * k),
            "custom/launcher" | "custom/power" | "idle_inhibitor" | "power-profiles-daemon" => fh * 0.9,
            "tray" => 2.0 * fh * 0.5 + 6.0 * k,
            _ => {
                let dot = if icons { fh * 0.45 + 5.0 * k } else { 0.0 };
                dot + font.advance(cr, &mod_text(m, ws))
            }
        }
    };
    let draw_mod = |m: &str, x: f64| -> f64 {
        let wdt = width(m);
        let full = if style == "islands" { wdt + 2.0 * pad } else { wdt + pad };
        let cx = if style == "islands" {
            draw_box(R::new(x, barr.y, full, barr.h), rad);
            x + pad
        } else {
            x + pad / 2.0
        };
        let cy = barr.cy();
        match m {
            "hyprland/workspaces" => {
                let cw = four + 14.0 * k;
                let look = ws.s("ws_look");
                for i in 0..n_ws {
                    let rr = R::new(cx + i as f64 * cw, cy - fh * 0.6, cw, fh * 1.2);
                    let mut c = fg;
                    if i == 0 {
                        c = if look == "box" { Rgba::hex("#0e131a") } else if look == "underline" { fg } else { acc };
                        if look == "pill" || look == "box" {
                            let a = acc.alpha(if look == "pill" { 46.0 / 255.0 } else { 1.0 });
                            draw::fill_rounded(cr, rr, (rad / 2.0).max(2.0), a);
                        } else if look == "underline" {
                            draw::fill_rect(cr, R::new(rr.x, barr.bottom() - 3.0 * k, rr.w, 3.0 * k), acc);
                        }
                    } else {
                        c = c.alpha(if i < 3 { 140.0 / 255.0 } else { 80.0 / 255.0 });
                    }
                    if ws.s("ws_format") == "dot" && icons {
                        let d = fh * if i == 0 { 0.42 } else { 0.3 };
                        draw::fill_ellipse(cr, rr.cx(), rr.cy(), d / 2.0, d / 2.0, c);
                    } else {
                        draw::text(cr, &font, rr, &(i + 1).to_string(), c, HAlign::Center, false);
                    }
                }
            }
            "custom/launcher" => {
                let d = fh * 0.62;
                cr.move_to(cx + wdt / 2.0, cy - d / 2.0);
                cr.line_to(cx + wdt / 2.0 + d / 2.0, cy + d / 2.0);
                cr.line_to(cx + wdt / 2.0 - d / 2.0, cy + d / 2.0);
                cr.close_path();
                acc.set(cr);
                let _ = cr.fill();
            }
            "custom/power" | "idle_inhibitor" | "power-profiles-daemon" => {
                let col = if m == "custom/power" { Rgba::hex("#e5534b") } else { Rgba::hex(&model::module_color(m, ws)) };
                let d = fh * 0.55;
                draw::ellipse(cr, cx + wdt / 2.0, cy, d / 2.0, d / 2.0);
                col.set(cr);
                cr.set_line_width((1.6 * k * 2.0).max(1.0));
                let _ = cr.stroke();
            }
            "tray" => {
                let d = fh * 0.5;
                for i in 0..2 {
                    draw::fill_rounded(cr, R::new(cx + i as f64 * (d + 6.0 * k), cy - d / 2.0, d, d), 2.0,
                        fg.alpha(150.0 / 255.0));
                }
            }
            _ => {
                let mut tx = cx;
                let colorful = ws.b("colorful");
                let mc = if colorful { Rgba::hex(&model::module_color(m, ws)) } else { acc };
                if icons {
                    let d = fh * 0.45;
                    draw::fill_ellipse(cr, cx + d / 2.0, cy, d / 2.0, d / 2.0, mc);
                    tx += d + 5.0 * k;
                }
                draw::text(cr, &font, R::new(tx, barr.y, wdt + 4.0, barr.h), &mod_text(m, ws),
                    if colorful { mc } else { fg }, HAlign::Left, false);
            }
        }
        full
    };
    let group_w = |mods: &[String]| -> f64 {
        if mods.is_empty() {
            return 0.0;
        }
        let each: f64 = mods.iter().map(|m| width(m) + if style == "islands" { 2.0 * pad } else { pad }).sum();
        each + gap * (mods.len() as f64 - 1.0)
    };
    let edge = if style == "islands" { 0.0 } else { 6.0 * k };
    let mut x = barr.x + edge;
    for m in ws.strs("left") {
        x += draw_mod(&m, x) + gap;
    }
    let center = ws.strs("center");
    x = barr.cx() - group_w(&center) / 2.0;
    for m in &center {
        x += draw_mod(m, x) + gap;
    }
    let right = ws.strs("right");
    x = barr.right() - group_w(&right) - edge;
    for m in &right {
        x += draw_mod(m, x) + gap;
    }
    m + h
}

fn window_content(cr: &cairo::Context, rect: R, hs: &Obj, k: f64, active: bool) {
    let pad = 16.0 * k;
    let lh = (9.0 * k).max(1.5);
    let a1 = Rgba::hex(hs.s("active1"));
    let a2 = Rgba::hex(if hs.b("gradient") { hs.s("active2") } else { hs.s("active1") });
    let cols = [a1, a2, Rgba::hex("#8a98ab")];
    let mut y = rect.y + pad;
    let mut i = 0usize;
    while y + lh < rect.bottom() - pad && i < 9 {
        let c = cols[i % 3].alpha(if active { 200.0 } else { 120.0 } / 255.0);
        let w1 = rect.w * (0.12 + 0.05 * (i % 3) as f64);
        draw::fill_rounded(cr, R::new(rect.x + pad, y, w1, lh), lh / 2.0, c);
        let g = Rgba::hex("#c8d3e2").alpha(if active { 110.0 } else { 70.0 } / 255.0);
        let w2 = (rect.w - 2.0 * pad - w1 - 6.0 * k) * (0.35 + 0.5 * ((i * 37) % 10) as f64 / 10.0);
        draw::fill_rounded(cr, R::new(rect.x + pad + w1 + 6.0 * k, y, w2.max(0.0), lh), lh / 2.0, g);
        y += lh * 2.4;
        i += 1;
    }
}

pub fn paint_desktop(cr: &cairo::Context, r: R, hs: &Obj, ws: Option<&Obj>, anim: f64, radius: f64, bar_scale: f64) {
    cr.save().ok();
    draw::rounded(cr, r, radius);
    cr.clip();
    let (wall, blur) = wallpaper((r.w as i32).max(1), (r.h as i32).max(1));
    cr.set_source_surface(&wall, r.x, r.y).ok();
    let _ = cr.paint();
    let k = r.w / VW;
    let (mut reserved_top, mut reserved_bottom) = (0.0, 0.0);
    if let Some(ws) = ws {
        let used = bar(cr, r, k * bar_scale, ws) * bar_scale;
        if ws.s("position") == "top" { reserved_top = used } else { reserved_bottom = used }
    }
    let (go, gi) = (hs.n("gaps_out"), hs.n("gaps_in"));
    let (ax, ay) = (go, reserved_top + go);
    let (aw, ah) = (VW - 2.0 * go, VH - reserved_top - reserved_bottom - 2.0 * go);
    let lw = if hs.s("layout") == "master" { (aw - 2.0 * gi) * 0.58 } else { (aw - 2.0 * gi) / 2.0 };
    let rh = (ah - 2.0 * gi) / 2.0;
    let wins = [(ax, ay, lw, ah), (ax + lw + 2.0 * gi, ay, aw - lw - 2.0 * gi, rh),
        (ax + lw + 2.0 * gi, ay + rh + 2.0 * gi, aw - lw - 2.0 * gi, rh)];
    let rad = hs.n("rounding") * k;
    let bs = hs.n("border_size") * k;
    for (idx, (x, y, w, h)) in wins.iter().enumerate() {
        let active = idx == 0;
        let mut rect = R::new(r.x + x * k, r.y + y * k, w * k, h * k);
        let mut alpha = 1.0;
        if idx == 2 && anim < 1.0 {
            match hs.s("win_style") {
                "slide" => rect = rect.translate((1.0 - anim) * (r.right() - rect.x), 0.0),
                "gnomed" => {
                    let s = anim.max(0.01);
                    rect = R::new(rect.x, rect.cy() - rect.h * s / 2.0, rect.w, rect.h * s);
                }
                _ => {
                    let s = 0.8 + 0.2 * anim;
                    rect = R::new(rect.cx() - rect.w * s / 2.0, rect.cy() - rect.h * s / 2.0, rect.w * s, rect.h * s);
                    alpha = (anim * 1.4).clamp(0.0, 1.0);
                }
            }
            if anim <= 0.0 {
                continue;
            }
        }
        cr.push_group();
        if hs.b("shadow") && hs.n("shadow_range") > 0.0 {
            let sc = Rgba::hex(hs.s("shadow_color"));
            let steps = (hs.i("shadow_range")).clamp(1, 14);
            let spread = hs.n("shadow_range") * k;
            for s in 0..steps {
                let f = (s + 1) as f64 / steps as f64;
                let a = ((sc.a8() as f64 * (1.0 - f).powf(hs.n("shadow_power") * 0.7) * 0.35) as i64) as f64 / 255.0;
                let g = spread * f;
                draw::fill_rounded(cr, rect.adj(-g, -g, g, g), rad + g, sc.alpha(a));
            }
        }
        let op = if active { hs.n("active_opacity") } else { hs.n("inactive_opacity") };
        cr.save().ok();
        draw::rounded(cr, rect, rad);
        cr.clip();
        if op < 1.0 && hs.b("blur") {
            cr.set_source_surface(&blur, r.x, r.y).ok();
            let _ = cr.paint();
        }
        draw::fill_rect(cr, rect, Rgba::hex("#141a24").alpha(if op < 1.0 { op } else { 1.0 }));
        window_content(cr, rect, hs, k, active);
        if hs.b("dim_inactive") && !active {
            draw::fill_rect(cr, rect, Rgba(0.0, 0.0, 0.0, ((160.0 * hs.n("dim_strength")) as i64) as f64 / 255.0));
        }
        cr.restore().ok();
        if bs > 0.0 {
            let pw = bs.max(1.0);
            let br = rect.adj(pw / 2.0, pw / 2.0, -pw / 2.0, -pw / 2.0);
            draw::rounded(cr, br, (rad - pw / 2.0).max(0.0));
            cr.set_line_width(pw);
            if active && hs.b("gradient") {
                let a = hs.n("angle").to_radians();
                let d = br.w.max(br.h) / 2.0;
                let (cx, cy) = (br.cx(), br.cy());
                let g = cairo::LinearGradient::new(cx - a.cos() * d, cy + a.sin() * d, cx + a.cos() * d, cy - a.sin() * d);
                let c1 = Rgba::hex(hs.s("active1"));
                let c2 = Rgba::hex(hs.s("active2"));
                g.add_color_stop_rgba(0.0, c1.0, c1.1, c1.2, c1.3);
                g.add_color_stop_rgba(1.0, c2.0, c2.1, c2.2, c2.3);
                cr.set_source(&g).ok();
            } else {
                Rgba::hex(if active { hs.s("active1") } else { hs.s("inactive") }).set(cr);
            }
            let _ = cr.stroke();
        }
        cr.pop_group_to_source().ok();
        let _ = cr.paint_with_alpha(alpha);
    }
    cr.restore().ok();
}

fn overlay(cr: &cairo::Context, r: R, ls: &Obj, radius: f64) {
    cr.save().ok();
    draw::rounded(cr, r, radius);
    cr.clip();
    draw::fill_rect(cr, r, Rgba(0.0, 0.0, 0.0, 70.0 / 255.0));
    paint_launcher(cr, r, ls);
    cr.restore().ok();
}

pub const APPS_DEMO: &[&str] = &["Firefox", "Kitty", "Dolphin", "Spotify", "Discord", "Steam", "VS Code", "GIMP",
    "Obsidian", "Blender", "OBS Studio", "Krita", "VLC", "Thunderbird", "Einstellungen", "Rechner", "Bilder", "Karten"];

/// Zeichnet das Launcher-Fenster (Rofi/Wofi/Fuzzel sehen mit diesen Einstellungen ähnlich aus).
pub fn paint_launcher(cr: &cairo::Context, r: R, ls: &Obj) {
    let k = r.w / VW;
    let grid = ls.b("grid");
    let cols = if grid { ls.i("columns").max(1) } else { 1 };
    let font = Font::new((ls.n("font_size") * 1.25 * k).round().max(4.0), pango::Weight::Medium);
    let fh = font.height(cr);
    let (pad, sp) = (ls.n("padding") * k, ls.n("spacing") * k);
    let ic = if ls.b("icons") { ls.n("icon_size") * k } else { 0.0 };
    let row_h = if grid { ic + fh + 28.0 * k } else { ic.max(fh) + 16.0 * k };
    let input_h = fh + 20.0 * k;
    let w = (ls.n("width") * k).min(r.w - 20.0 * k);
    let lines = ls.i("lines");
    let h = 2.0 * pad + input_h + 12.0 * k + lines as f64 * (row_h + sp);
    let x = r.cx() - w / 2.0;
    let y = match ls.s("position") {
        "top" => r.y + ls.n("offset") * k + 10.0 * k,
        "bottom" => r.bottom() - h - ls.n("offset") * k - 10.0 * k,
        _ => r.cy() - h / 2.0,
    };
    let bx = R::new(x, (r.y + 4.0).max(y), w, h.min(r.h - 8.0));
    cr.save().ok();
    let rad = ls.n("radius") * k;
    for i in 0..6 {
        let g = (i + 1) as f64 * 3.0 * k;
        draw::fill_rounded(cr, bx.adj(-g, -g + 2.0 * k, g, g + 2.0 * k), rad + g, Rgba(0.0, 0.0, 0.0, 18.0 / 255.0));
    }
    draw::rounded(cr, bx, rad);
    Rgba::hex(ls.s("bg")).set(cr);
    let _ = cr.fill_preserve();
    if ls.n("border_width") > 0.0 {
        Rgba::hex(ls.s("border")).set(cr);
        cr.set_line_width((ls.n("border_width") * k).max(1.0));
        let _ = cr.stroke();
    } else {
        cr.new_path();
    }
    let clip = bx.adj(1.0, 1.0, -1.0, -1.0);
    cr.rectangle(clip.x, clip.y, clip.w, clip.h);
    cr.clip();
    let inner = bx.adj(pad, pad, -pad, -pad);
    let ib = R::new(inner.x, inner.y, inner.w, input_h);
    draw::fill_rounded(cr, ib, (rad - 4.0 * k).max(0.0), Rgba::hex(ls.s("input_bg")));
    let mut tx = ib.x + 14.0 * k;
    if ls.b("show_prompt") && ls.s("app") != "wofi" {
        draw::text(cr, &font, R::new(tx, ib.y, ib.w, ib.h), ls.s("prompt"), Rgba::hex(ls.s("accent")), HAlign::Left, false);
        tx += font.advance(cr, ls.s("prompt")) + 10.0 * k;
    }
    draw::text(cr, &font, R::new(tx, ib.y, ib.right() - tx, ib.h), ls.s("placeholder"),
        Rgba::hex(&model::muted_color(ls.s("fg"))), HAlign::Left, true);
    let y0 = ib.bottom() + 12.0 * k;
    let cw = (inner.w - sp * (cols - 1) as f64) / cols as f64;
    for i in 0..(lines * cols) {
        let cx = inner.x + (i % cols) as f64 * (cw + sp);
        let cy = y0 + (i / cols) as f64 * (row_h + sp);
        let cell = R::new(cx, cy, cw, row_h);
        let selected = i == 0;
        if selected {
            draw::fill_rounded(cr, cell, ls.n("el_radius") * k, Rgba::hex(ls.s("sel")));
        }
        let name = APPS_DEMO[i as usize % APPS_DEMO.len()];
        let hue = ((i * 47 + 200) % 360) as f64;
        let tc = Rgba::hex(if selected { ls.s("sel_fg") } else { ls.s("fg") });
        if grid {
            if ic > 0.0 {
                draw::fill_rounded(cr, R::new(cell.cx() - ic / 2.0, cell.y + 10.0 * k, ic, ic), ic * 0.25,
                    Rgba::hsl(hue, 150.0, 140.0));
            }
            draw::text(cr, &font, R::new(cell.x, cell.bottom() - fh - 8.0 * k, cell.w, fh), name, tc, HAlign::Center, true);
        } else {
            let mut ix = cell.x + 10.0 * k;
            if ic > 0.0 {
                draw::fill_rounded(cr, R::new(ix, cell.cy() - ic / 2.0, ic, ic), ic * 0.25, Rgba::hsl(hue, 150.0, 140.0));
                ix += ic + 12.0 * k;
            }
            draw::text(cr, &font, R::new(ix, cell.y, cell.right() - ix, cell.h), name, tc, HAlign::Left, false);
        }
    }
    cr.restore().ok();
}

/// Kleines Bild (Preset-Karten): Desktop, optional Launcher darüber.
pub fn render_texture(hs: &Obj, ws: Option<&Obj>, w: i32, h: i32, radius: f64, ls: Option<&Obj>, bar_scale: f64)
    -> gdk::Texture {
    let (hs, ws, ls) = (hs.clone(), ws.cloned(), ls.cloned());
    draw::texture(w, h, 2.0, move |cr| {
        let r = R::new(0.0, 0.0, w as f64, h as f64);
        paint_desktop(cr, r, &hs, ws.as_ref(), 1.0, radius, bar_scale);
        if let Some(ls) = &ls {
            overlay(cr, r, ls, radius);
        }
    })
}

pub struct DesktopPreview {
    pub area: gtk::DrawingArea,
    pub hs: RefCell<Obj>,
    pub ws: RefCell<Obj>,
    pub ls: RefCell<Option<Obj>>,
    pub zoom_bar: Cell<bool>,
    anim: Cell<f64>,
    tick: RefCell<Option<gtk::TickCallbackId>>,
}

impl DesktopPreview {
    pub fn new() -> Rc<DesktopPreview> {
        let area = gtk::DrawingArea::new();
        area.set_hexpand(true);
        area.set_vexpand(false);
        area.set_content_height(220);
        let p = Rc::new(DesktopPreview { area, hs: RefCell::new(model::hypr_state(None)),
            ws: RefCell::new(model::waybar_state(None)), ls: RefCell::new(None), zoom_bar: Cell::new(false),
            anim: Cell::new(1.0), tick: RefCell::new(None) });
        let w = Rc::downgrade(&p);
        p.area.set_draw_func(move |_, cr, wd, ht| {
            if let Some(p) = w.upgrade() {
                p.paint(cr, wd as f64, ht as f64);
            }
        });
        p
    }

    pub fn set_states(&self, hs: &Obj, ws: &Obj) {
        *self.hs.borrow_mut() = hs.clone();
        *self.ws.borrow_mut() = ws.clone();
        self.area.queue_draw();
    }

    pub fn set_launcher(&self, ls: Option<&Obj>) {
        *self.ls.borrow_mut() = ls.cloned();
        self.area.queue_draw();
    }

    pub fn play(self: &Rc<Self>) {
        let hs = self.hs.borrow().clone();
        if !hs.b("anim") {
            return;
        }
        let speed = model::anim_list(&hs).iter().find(|a| a.0 == "windowsIn").map(|a| a.1).unwrap_or(4.0);
        let dur = (speed * 100.0 * 1.6) as i64 * 1000;
        let curve = model::curve(hs.s("anim_curve"));
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
            let v = ((now - t0) as f64 / dur.max(1) as f64).min(1.0);
            p.anim.set(if v < 1.0 { bezier_y(curve[0], curve[1], v) } else { 1.0 });
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

    fn paint(&self, cr: &cairo::Context, w: f64, h: f64) {
        let r = R::new(0.0, 0.0, w, h);
        let hs = self.hs.borrow();
        let ws = self.ws.borrow();
        if self.zoom_bar.get() {
            let th = r.w * 9.0 / 16.0;
            let top = if ws.s("position") == "top" { r.y } else { r.bottom() - th };
            draw::rounded(cr, r, 14.0);
            cr.clip();
            paint_desktop(cr, R::new(r.x, top, r.w, th), &hs, Some(&ws), self.anim.get(), 0.0, 1.0);
        } else {
            let (mut tw, mut th) = (r.w, r.w * 9.0 / 16.0);
            if th > r.h {
                th = r.h;
                tw = th * 16.0 / 9.0;
            }
            let rect = R::new(r.x + (r.w - tw) / 2.0, r.y + (r.h - th) / 2.0, tw, th);
            paint_desktop(cr, rect, &hs, Some(&ws), self.anim.get(), 14.0, 1.0);
            if let Some(ls) = self.ls.borrow().as_ref() {
                overlay(cr, rect, ls, 14.0);
            }
        }
    }
}
