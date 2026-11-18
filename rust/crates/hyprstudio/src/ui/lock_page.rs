//! Hyprland › Sperrbildschirm (Hyprlock): Presets, Live-Vorschau, Hintergrund, Uhr & Text, Passwortfeld, Verhalten.

use super::dialogs;
use super::draw::{self, Font, HAlign, Rgba, R};
use super::gate::GateContent;
use super::paper_page::{action, scroll_page};
use super::studio::Studio;
use super::widgets::{button, flow_grid, hbox, label, Card, Host, PresetCard};
use crate::i18n::{t, tf};
use crate::jsonx::{Get, Obj};
use crate::{hyprlock, kw, model, online, util};
use gtk::gdk::prelude::GdkCairoContextExt;
use gtk::prelude::*;
use gtk::{cairo, gdk, gdk_pixbuf::{InterpType, Pixbuf}, glib, pango};
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::Path;
use std::rc::{Rc, Weak};

thread_local! {
    static DECODED: RefCell<HashMap<(String, i32, i32), Option<Pixbuf>>> = RefCell::new(HashMap::new());
    static BG: RefCell<HashMap<(String, i32, i32, i64), Option<Pixbuf>>> = RefCell::new(HashMap::new());
}

/// Bild nur einmal je Größe von der Platte lesen (alle Presets nutzen dasselbe Hintergrundbild).
fn decoded(path: &str, w: i32, h: i32) -> Option<Pixbuf> {
    let key = (path.to_string(), w, h);
    if let Some(v) = DECODED.with(|c| c.borrow().get(&key).cloned()) {
        return v;
    }
    let pb = if path.is_empty() {
        None
    } else {
        (|| {
            let p = Path::new(path);
            let (_, iw, ih) = Pixbuf::file_info(p)?;
            let pb = if iw > 0 && ih > 0 {
                let s = (w as f64 / iw as f64).max(h as f64 / ih as f64);
                Pixbuf::from_file_at_scale(p, ((iw as f64 * s) as i32).max(1), ((ih as f64 * s) as i32).max(1), false)
                    .ok()?
            } else {
                Pixbuf::from_file(p).ok()?
            };
            Some(pb.apply_embedded_orientation().unwrap_or(pb))
        })()
    };
    DECODED.with(|c| {
        let mut c = c.borrow_mut();
        if c.len() > 8 {
            c.clear();
        }
        c.insert(key, pb.clone());
    });
    pb
}

/// Bild füllend zuschneiden und grob weichzeichnen (kleiner rechnen und wieder hochziehen).
fn background(path: &str, w: i32, h: i32, blur: i64) -> Option<Pixbuf> {
    let key = (path.to_string(), w, h, blur);
    if let Some(v) = BG.with(|c| c.borrow().get(&key).cloned()) {
        return v;
    }
    let out = (|| {
        let img = decoded(path, w, h)?;
        let scale = (w as f64 / img.width() as f64).max(h as f64 / img.height() as f64);
        let sw = (img.width() as f64 * scale) as i32 + 1;
        let sh = (img.height() as f64 * scale) as i32 + 1;
        let img = img.scale_simple(sw, sh, InterpType::Bilinear)?;
        let img = img.new_subpixbuf((sw - w) / 2, (sh - h) / 2, w.min(sw), h.min(sh)).copy()?;
        if blur > 0 {
            let f = (blur as f64 * w as f64 / 1920.0 * 1.6).max(1.0);
            let small = img.scale_simple(((w as f64 / f) as i32).max(1), ((h as f64 / f) as i32).max(1),
                InterpType::Bilinear)?;
            return small.scale_simple(w, h, InterpType::Bilinear);
        }
        Some(img)
    })();
    BG.with(|c| {
        let mut c = c.borrow_mut();
        if c.len() > 40 {
            c.clear();
        }
        c.insert(key, out.clone());
    });
    out
}

fn user() -> String {
    std::env::var("USER").ok().filter(|s| !s.is_empty()).or_else(|| std::env::var("USERNAME").ok())
        .filter(|s| !s.is_empty()).unwrap_or_else(|| "user".into())
}

fn now_fmt(fmt: &str) -> String {
    glib::DateTime::now_local().ok().and_then(|d| d.format(fmt).ok()).map(|s| s.to_string()).unwrap_or_default()
}

/// Sperrbildschirm zeichnen (w×h logisch, `px` = Pixel pro Einheit für scharfe Bilder).
pub fn paint_lock(cr: &cairo::Context, w: f64, h: f64, s: &Obj, wallpaper: &str, radius: f64, px: f64) {
    cr.save().ok();
    draw::rounded(cr, R::new(0.0, 0.0, w, h), radius);
    cr.clip();
    let k = h / 1080.0;
    let full = R::new(0.0, 0.0, w, h);

    // Hintergrund (Bildschirmfoto wird mit dem Hintergrundbild angedeutet)
    let bg = s.s("bg");
    let path = match bg {
        "screenshot" | "wallpaper" => wallpaper.to_string(),
        "image" => s.s("bg_path").to_string(),
        _ => String::new(),
    };
    let path = if path.is_empty() {
        String::new()
    } else {
        util::expanduser(&hyprlock::first_image(&path)).to_string_lossy().to_string()
    };
    draw::fill_rect(cr, full, Rgba::hex(s.s("bg_color")));
    let (pw, ph) = ((w * px).round() as i32, (h * px).round() as i32);
    let img = if path.is_empty() {
        None
    } else {
        background(&path, pw.max(1), ph.max(1), s.i("blur_size") * s.i("blur_passes"))
    };
    match &img {
        None if bg != "color" => {
            let g = cairo::LinearGradient::new(0.0, 0.0, w, h);
            g.add_color_stop_rgb(0.0, 0x24 as f64 / 255.0, 0x30 as f64 / 255.0, 0x4a as f64 / 255.0);
            g.add_color_stop_rgb(1.0, 0x3a as f64 / 255.0, 0x24 as f64 / 255.0, 0x48 as f64 / 255.0);
            cr.set_source(&g).ok();
            cr.rectangle(0.0, 0.0, w, h);
            let _ = cr.fill();
        }
        Some(pb) => {
            cr.save().ok();
            cr.scale(1.0 / px, 1.0 / px);
            cr.set_source_pixbuf(pb, 0.0, 0.0);
            let _ = cr.paint();
            cr.restore().ok();
        }
        None => {}
    }
    if bg != "color" {
        let b = s.n("brightness");
        let shade = if b < 1.0 {
            Rgba::rgb8(0, 0, 0, (((1.0 - b).min(1.0)) * 255.0) as u8)
        } else {
            Rgba::rgb8(255, 255, 255, (((b - 1.0).min(1.0)) * 255.0) as u8)
        };
        draw::fill_rect(cr, full, shade);
    }

    let (ww, hh) = (w / k, 1080.0); // Bildschirm in 1080p-Koordinaten (wie in hyprlock.conf)
    let pos = hyprlock::place(s);
    let hs = hyprlock::heights(s);
    let text_c = Rgba::hex(s.s("text_color"));
    let fam = if s.s("font").is_empty() { "Sans".to_string() } else { s.s("font").to_string() };

    // Rechteck in Vorschau-Pixeln – gleiche Rechnung wie hyprlock: Abstand von der Ankerkante.
    let boxr = |e: &str, bw: f64| -> (R, &'static str) {
        let (x, y, ha, va) = pos[e];
        let (x, y) = (x as f64, y as f64);
        let bh = hs[e] as f64;
        let left = match ha {
            "left" => x,
            "right" => ww + x - bw,
            _ => ww / 2.0 + x - bw / 2.0,
        };
        let top = match va {
            "center" => hh / 2.0 - y - bh / 2.0,
            "bottom" => hh - y - bh,
            _ => -y,
        };
        (R::new(left * k, top * k, bw * k, bh * k), ha)
    };
    let lbl = |e: &str, txt: &str, size: f64, bold: bool| {
        let f = Font { family: fam.clone(), px: ((size * k * 1.33) as i64).max(4) as f64,
            weight: if bold { pango::Weight::Bold } else { pango::Weight::Normal } };
        let (r, ha) = boxr(e, f.advance(cr, txt) / k + 4.0);
        let al = match ha {
            "left" => HAlign::Left,
            "right" => HAlign::Right,
            _ => HAlign::Center,
        };
        draw::text(cr, &f, r, txt, text_c, al, false);
    };

    if pos.contains_key("clock") {
        let fmt = match s.s("clock_fmt") {
            "12" => "%I:%M %p",
            "sec" => "%H:%M:%S",
            _ => "%H:%M",
        };
        lbl("clock", &now_fmt(fmt), s.n("clock_size"), true);
    }
    if pos.contains_key("date") {
        lbl("date", &now_fmt(hyprlock::date_fmt(s.s("date_fmt"))), s.n("date_size"), false);
    }
    if pos.contains_key("avatar") {
        let (r, _) = boxr("avatar", s.n("avatar_size"));
        let d = r.w;
        let ap = s.s("avatar_path");
        let av = if ap.is_empty() { None } else { Pixbuf::from_file(util::expanduser(ap)).ok() };
        cr.save().ok();
        draw::ellipse(cr, r.cx(), r.cy(), r.w / 2.0, r.h / 2.0);
        cr.clip();
        match av {
            None => {
                draw::fill_rect(cr, r, Rgba::rgb8(255, 255, 255, 40));
                let c = Rgba::rgb8(255, 255, 255, 170);
                draw::fill_ellipse(cr, r.cx(), r.y + d * 0.38, d * 0.18, d * 0.18, c);
                draw::fill_ellipse(cr, r.cx(), r.y + d * 0.92, d * 0.34, d * 0.3, c);
            }
            Some(pb) => {
                cr.translate(r.x, r.y);
                cr.scale(r.w / pb.width() as f64, r.h / pb.height() as f64);
                cr.set_source_pixbuf(&pb, 0.0, 0.0);
                let _ = cr.paint();
            }
        }
        cr.restore().ok();
        Rgba::hex(s.s("accent")).set(cr);
        cr.set_line_width((s.n("outline") * k).max(1.0));
        draw::ellipse(cr, r.cx(), r.cy(), r.w / 2.0, r.h / 2.0);
        let _ = cr.stroke();
    }
    if pos.contains_key("greet") {
        lbl("greet", &s.s("greet_text").replace("$USER", &user()), 16.0, false);
    }

    // Passwortfeld
    let (r, _) = boxr("input", s.n("input_w"));
    let ih = r.h;
    let rad = if s.n("rounding") < 0.0 { ih / 2.0 } else { (ih / 2.0).min(s.n("rounding") * k) };
    draw::rounded(cr, r, rad);
    Rgba::hex(s.s("inner_color")).set(cr);
    if s.n("outline") != 0.0 {
        let _ = cr.fill_preserve();
        Rgba::hex(s.s("accent")).set(cr);
        cr.set_line_width(s.n("outline") * k);
        let _ = cr.stroke();
    } else {
        let _ = cr.fill();
    }
    let l = pangocairo::functions::create_layout(cr);
    let mut fd = pango::FontDescription::new();
    fd.set_family(&fam);
    fd.set_absolute_size(((ih * 0.36) as i64).max(4) as f64 * pango::SCALE as f64);
    fd.set_style(pango::Style::Italic);
    l.set_font_description(Some(&fd));
    l.set_text(s.s("placeholder"));
    let (tw, th) = l.pixel_size();
    Rgba::hex(s.s("input_text")).alpha(150.0 / 255.0).set(cr);
    cr.move_to(r.x + (r.w - tw as f64) / 2.0, r.y + (r.h - th as f64) / 2.0);
    pangocairo::functions::show_layout(cr, &l);
    cr.restore().ok();
}

pub fn render_lock(s: &Obj, w: i32, h: i32, wallpaper: &str, radius: f64) -> gdk::Texture {
    let s = s.clone();
    let wp = wallpaper.to_string();
    draw::texture(w, h, 2.0, move |cr| paint_lock(cr, w as f64, h as f64, &s, &wp, radius, 2.0))
}

pub struct LockEditor {
    pub root: gtk::ScrolledWindow,
    win: Weak<Studio>,
    pub state: RefCell<Obj>,
    cards: RefCell<Vec<Rc<Card>>>,
    presets: RefCell<Vec<(String, Rc<PresetCard>)>>,
    btn_test: gtk::Button,
    btn_undo: gtk::Button,
    btn_apply: gtk::Button,
    preview: gtk::DrawingArea,
    wp: RefCell<String>,
    btn_img: gtk::Button,
    img_lbl: gtk::Label,
    img_row: gtk::Box,
    thumb_sig: RefCell<Option<(String, String)>>,
    building: Cell<bool>,
}

thread_local! {
    static LOCKS: RefCell<Vec<Weak<LockEditor>>> = const { RefCell::new(Vec::new()) };
}

impl Host for LockEditor {
    fn value(&self, key: &str) -> Value {
        self.state.borrow().val(key).clone()
    }
    fn set_value(&self, key: &str, v: Value) {
        self.set(key, v);
    }
}

impl LockEditor {
    fn me(&self) -> Option<Rc<LockEditor>> {
        LOCKS.with(|p| p.borrow().iter().filter_map(|w| w.upgrade()).find(|x| std::ptr::eq(x.as_ref(), self)))
    }

    fn win(&self) -> Rc<Studio> {
        self.win.upgrade().expect("Fenster")
    }

    pub fn new(win: &Rc<Studio>) -> Rc<LockEditor> {
        let state = hyprlock::lock_state(win.cfg.borrow().obj("lock"));
        let (root, lay) = scroll_page();
        let preview = gtk::DrawingArea::new();
        preview.set_content_height(300);
        preview.set_size_request(-1, 300);
        preview.set_hexpand(true);
        let img_row = hbox(8);
        let e = Rc::new(LockEditor { root, win: Rc::downgrade(win), state: RefCell::new(state),
            cards: RefCell::new(vec![]), presets: RefCell::new(vec![]), btn_test: action(false),
            btn_undo: action(false), btn_apply: action(true), preview, wp: RefCell::new(String::new()),
            btn_img: button("", ""), img_lbl: label("", "Mono"), img_row, thumb_sig: RefCell::new(None),
            building: Cell::new(true) });
        LOCKS.with(|x| x.borrow_mut().push(Rc::downgrade(&e)));
        let hw: Weak<LockEditor> = Rc::downgrade(&e);
        for (b, f) in [(&e.btn_test, LockEditor::test as fn(&LockEditor)), (&e.btn_undo, LockEditor::undo),
            (&e.btn_apply, LockEditor::apply)] {
            let w = Rc::downgrade(&e);
            b.connect_clicked(move |_| {
                if let Some(e) = w.upgrade() {
                    f(&e);
                }
            });
        }
        let w = Rc::downgrade(&e);
        e.preview.set_draw_func(move |_, cr, wd, ht| {
            let Some(e) = w.upgrade() else { return };
            if wd < 50 {
                return;
            }
            let h = ht as f64;
            let pw = (wd as f64).min((h * 16.0 / 9.0).floor());
            cr.translate(((wd as f64 - pw) / 2.0).floor(), 0.0);
            let sf = e.preview.scale_factor() as f64;
            paint_lock(cr, pw, h, &e.state.borrow(), &e.wp.borrow(), 14.0, sf);
        });
        lay.append(&e.preview);
        let w = Rc::downgrade(&e);
        glib::timeout_add_seconds_local(1, move || {
            let Some(e) = w.upgrade() else { return glib::ControlFlow::Break };
            if e.preview.is_mapped() && e.state.borrow().b("clock") {
                e.preview.queue_draw();
            }
            glib::ControlFlow::Continue
        });

        let card = |title: &str| {
            let c = Card::new(title, hw.clone());
            e.cards.borrow_mut().push(c.clone());
            lay.append(&c.root);
            c
        };

        // Presets
        let c = card("lock.presets");
        let grid = flow_grid();
        for name in hyprlock::presets().keys() {
            let pc = PresetCard::new(name);
            let w = Rc::downgrade(&e);
            pc.connect(move |n| {
                if let Some(e) = w.upgrade() {
                    e.use_preset(&n);
                }
            });
            grid.insert(&pc.root, -1);
            e.presets.borrow_mut().push((name.clone(), pc));
        }
        c.append(&grid);

        // Hintergrund
        let c = card("lock.layout");
        c.choice("layout", hyprlock::LAYOUTS, "llay.", None, Some("lock.layout"));

        let c = card("lock.bg");
        c.choice("bg", hyprlock::BGS, "lbg.", None, Some("lock.bg_src"));
        let w = Rc::downgrade(&e);
        e.btn_img.connect_clicked(move |_| {
            if let Some(e) = w.upgrade() {
                e.choose_image();
            }
        });
        e.img_lbl.set_hexpand(true);
        e.img_lbl.set_xalign(0.0);
        e.img_lbl.set_ellipsize(pango::EllipsizeMode::Middle);
        e.img_row.append(&e.btn_img);
        e.img_row.append(&e.img_lbl);
        c.add_row("lock.image", e.img_row.upcast_ref(), None, None, None, Some("lock.image"));
        c.color("bg_color", Some("lock.bg_color"), None);
        c.slider("blur_passes", 0.0, 4.0, 1.0, "{:.0f}", None, Some("lock.blur_passes"));
        c.slider("blur_size", 1.0, 15.0, 1.0, "{:.0f}", None, Some("lock.blur_size"));
        c.slider("brightness", 0.2, 1.2, 0.05, "{:.2f}", None, Some("lock.brightness"));
        c.slider("contrast", 0.5, 1.5, 0.05, "{:.2f}", None, Some("lock.contrast"));
        c.slider("vibrancy", 0.0, 1.0, 0.05, "{:.2f}", None, Some("lock.vibrancy"));

        // Uhr & Text
        let c = card("lock.text");
        c.toggle("clock", None, Some("lock.clock"));
        c.choice("clock_fmt", hyprlock::CLOCKS, "lclk.", None, Some("lock.clock_fmt"));
        c.slider("clock_size", 30.0, 200.0, 1.0, "{:.0f}", None, Some("lock.clock_size"));
        c.toggle("date", None, Some("lock.date"));
        c.choice("date_fmt", hyprlock::DATES, "ldate.", None, Some("lock.date_fmt"));
        c.slider("date_size", 10.0, 40.0, 1.0, "{:.0f}", None, Some("lock.date_size"));
        c.toggle("greet", None, Some("lock.greet"));
        c.text("greet_text", Some("tip.lock.greet"), 260, Some("lock.greet_text"));
        c.toggle("avatar", None, Some("lock.avatar"));
        c.text("avatar_path", None, 260, Some("lock.avatar_path"));
        c.slider("avatar_size", 60.0, 220.0, 1.0, "{:.0f}", None, Some("lock.avatar_size"));
        c.font("font", None, Some("font"));
        c.color("text_color", Some("lock.text_color"), None);

        // Passwortfeld
        let c = card("lock.input");
        c.slider("input_w", 150.0, 700.0, 10.0, "{:.0f}", None, Some("lock.input_w"));
        c.slider("input_h", 30.0, 100.0, 1.0, "{:.0f}", None, Some("lock.input_h"));
        c.slider("rounding", -1.0, 50.0, 1.0, "{:.0f}", Some("tip.lock.rounding"), Some("lock.rounding"));
        c.slider("outline", 0.0, 8.0, 1.0, "{:.0f}", None, Some("lock.outline"));
        c.slider("input_y", -450.0, 150.0, 10.0, "{:.0f}", None, Some("lock.input_y"));
        c.text("placeholder", None, 260, Some("lock.placeholder"));
        c.color("accent", Some("lock.accent"), None);
        c.color("inner_color", Some("lock.inner"), None);
        c.color("input_text", Some("lock.input_text"), None);
        c.color("check_color", Some("lock.check"), None);
        c.color("fail_color", Some("lock.fail"), None);
        c.toggle("fade_empty", None, Some("lock.fade_empty"));
        c.toggle("dots_center", None, Some("lock.dots_center"));

        // Verhalten
        let c = card("lock.behavior");
        c.toggle("lock_bind", Some("tip.lock.bind"), Some("lock.bind"));
        c.toggle("hide_cursor", None, Some("lock.hide_cursor"));
        c.toggle("ignore_empty", None, Some("lock.ignore_empty"));
        c.slider("grace", 0.0, 30.0, 1.0, "{:.0f} s", Some("tip.lock.grace"), Some("lock.grace"));
        e.building.set(false);
        e.refresh_controls();
        e
    }

    // ----- Zustand -----

    pub fn wallpaper(&self) -> String {
        let ps = self.win.upgrade().and_then(|w| w.op()).map(|o| o.paper.state.borrow().clone()).unwrap_or_default();
        let mut path = ps.s("path").to_string();
        if path.is_empty() {
            path = ps.obj("monitors").and_then(|m| m.values().next().and_then(|v| v.as_object()))
                .map(|m| m.s("path").to_string()).unwrap_or_default();
        }
        hyprlock::first_image(&path)
    }

    pub fn set(&self, key: &str, value: Value) {
        if self.state.borrow().get(key).map(|v| super::editor::py_eq(v, &value)).unwrap_or(false) {
            return;
        }
        self.state.borrow_mut().insert(key.into(), value);
        self.changed();
    }

    pub fn changed(&self) {
        if self.building.get() {
            return;
        }
        let win = self.win();
        win.cfg.borrow_mut().insert("lock".into(), Value::Object(self.state.borrow().clone()));
        win.save_later();
        let (bg, bp) = {
            let s = self.state.borrow();
            (s.s("bg").to_string(), s.s("bg_path").to_string())
        };
        self.img_row.set_sensitive(bg == "image");
        let short = if bp.is_empty() { String::new() } else { online::short(Path::new(&bp)) };
        self.img_lbl.set_text(if short.is_empty() { "–" } else { &short });
        *self.wp.borrow_mut() = self.wallpaper();
        self.preview.queue_draw();
        self.mark();
    }

    pub fn refresh_controls(&self) {
        if let Some(hp) = self.win().hp() {
            let on = hyprlock::has_lock_bind(hp.state.borrow().arr("binds"));
            self.state.borrow_mut().insert("lock_bind".into(), json!(on));
        }
        let s = self.state.borrow().clone();
        for c in self.cards.borrow().iter() {
            c.refresh(&s);
        }
        self.changed();
    }

    /// Beim Öffnen: Kürzel-Stand übernehmen, Preset-Bilder mit aktuellem Hintergrundbild zeichnen.
    pub fn refresh_impl(&self) {
        self.refresh_controls();
        let wp = self.wallpaper();
        // Preset-Bilder hängen nur vom Hintergrund und der Schrift ab
        let sig = (wp.clone(), self.state.borrow().s("font").to_string());
        if self.thumb_sig.borrow().as_ref() == Some(&sig) {
            return;
        }
        *self.thumb_sig.borrow_mut() = Some(sig);
        let st = self.state.borrow().clone();
        for (name, pc) in self.presets.borrow().iter() {
            pc.set_texture(&render_lock(&hyprlock::preset_state(name, Some(&st)), 196, 110, &wp, 10.0));
        }
    }

    pub fn use_preset(&self, name: &str) {
        let st = hyprlock::preset_state(name, Some(&self.state.borrow()));
        *self.state.borrow_mut() = st;
        self.refresh_controls();
        self.win().set_status(&tf("lock.preset", kw!("name" => t(&format!("lp.{name}")))));
    }

    fn choose_image(self: &Rc<Self>) {
        let bp = self.state.borrow().s("bg_path").to_string();
        let start = if bp.is_empty() {
            util::home()
        } else {
            util::expanduser(&bp).parent().map(|p| p.to_path_buf()).unwrap_or_else(util::home)
        };
        let w = Rc::downgrade(self);
        dialogs::open_file(super::shell::toplevel_of(&self.root).as_ref(), &t("lock.image"),
            &start.to_string_lossy(), &[("Bilder", &["*.png", "*.jpg", "*.jpeg", "*.webp", "*.jxl", "*.bmp"])],
            move |f| {
                let (Some(e), Some(f)) = (w.upgrade(), f) else { return };
                e.state.borrow_mut().insert("bg".into(), json!("image"));
                e.set("bg_path", json!(f.to_string_lossy()));
                e.refresh_controls();
            });
    }

    // ----- Aktionen -----

    pub fn apply(&self) {
        let win = self.win();
        win.log("");
        let hdir = model::config_home().join("hypr");
        let st = self.state.borrow().clone();
        let (ok, info) = hyprlock::apply_lock(&st, &hdir, &|s| win.log(s), &self.wallpaper());
        if ok {
            if let Some(hp) = win.hp() {
                let want = st.b("lock_bind");
                let binds = hp.state.borrow().arr("binds").to_vec();
                if hyprlock::has_lock_bind(&binds) != want {
                    hp.state.borrow_mut().insert("binds".into(), Value::Array(hyprlock::set_lock_bind(&binds, want)));
                    hp.refresh_controls();
                    hp.changed();
                    hp.apply();
                }
            }
        }
        win.set_status(&if ok { t("lock.done") } else { t(&info) });
    }

    pub fn undo(&self) {
        let ok = model::undo_file(&hyprlock::lock_file(&model::config_home().join("hypr")));
        self.win().set_status(&if ok { t("undone") } else { t("nothing") });
    }

    /// Jetzt sperren (mit der angewendeten Datei) – entsperren mit dem eigenen Passwort.
    pub fn test(&self) {
        let win = self.win();
        if model::dry() {
            win.log("$ hyprlock  (Testmodus)");
            return;
        }
        match util::spawn_detached(&["hyprlock"]) {
            Ok(_) => win.log("$ hyprlock"),
            Err(e) => win.log(&format!("hyprlock: {e}")),
        }
    }

    pub fn retranslate(&self) {
        for c in self.cards.borrow().iter() {
            c.retranslate();
        }
        for (name, pc) in self.presets.borrow().iter() {
            pc.name.set_text(&t(&format!("lp.{name}")));
        }
        self.btn_img.set_label(&t("paper.image"));
        self.btn_test.set_label(&t("lock.test"));
        self.btn_undo.set_label(&t("undo"));
        self.btn_apply.set_label(&t("apply"));
        self.btn_test.set_tooltip_text(Some(&t("tip.lock.test")));
        self.changed();
    }

    pub fn apply_theme(&self, _accent: &str) {
        self.mark();
    }

    /// Preset-Auswahl markieren.
    fn mark(&self) {
        let st = self.state.borrow().clone();
        let look = hyprlock::look_keys();
        for (name, pc) in self.presets.borrow().iter() {
            let ps = hyprlock::preset_state(name, Some(&st));
            let sel = ps.iter().filter(|(k, _)| look.contains(k))
                .all(|(k, v)| st.get(k).map(|x| super::editor::py_eq(x, v)).unwrap_or(false));
            pc.refresh(sel, false);
        }
    }
}

impl GateContent for LockEditor {
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }
    fn actions(&self) -> Vec<gtk::Button> {
        vec![self.btn_test.clone(), self.btn_undo.clone(), self.btn_apply.clone()]
    }
    fn refresh(&self) {
        if let Some(me) = self.me() {
            me.refresh_impl();
        }
    }
}
