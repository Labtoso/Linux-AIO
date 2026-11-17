//! Waybar › Editor: Module per Drag & Drop direkt auf der Leiste anordnen.
//! Ziehen innerhalb der Leiste = verschieben (links/Mitte/rechts), aus der Palette auf die Leiste = hinzufügen,
//! von der Leiste weg = entfernen. Rechtsklick auf ein Modul: entfernen oder in einen anderen Bereich.
//! Außerdem: vier Listen (Verfügbar, Links, Mitte, Rechts) zum Verschieben per Ziehen (ModuleColumns).

use super::dialogs::{self, MenuEntry};
use super::draw::{self, Font, HAlign, Rgba, R};
use super::theme;
use super::widgets::{hbox, label, vbox, Cb};
use crate::i18n::t;
use crate::jsonx::{Get, Obj};
use crate::model;
use gtk::prelude::*;
use gtk::{cairo, gdk, glib, pango};
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;

pub const SIDES: &[&str] = &["left", "center", "right"];

pub type Lists = Vec<(String, Vec<String>)>;

pub struct BarEditor {
    pub area: gtk::DrawingArea,
    ws: RefCell<Obj>,
    drag: RefCell<Option<(String, String)>>,
    drag_pos: Cell<Option<(f64, f64)>>,
    hover: RefCell<Option<String>>,
    on_changed: Cb<Lists>,
}

fn font() -> Font {
    Font::new(13.0, pango::Weight::Semibold)
}

fn mod_w(cr: &cairo::Context, m: &str) -> f64 {
    font().advance(cr, &t(&format!("m.{m}"))) + 30.0
}

fn side_list(ws: &Obj, side: &str) -> Vec<String> {
    ws.strs(side)
}

/// Messen ohne sichtbares Zeichnen.
fn measure_cr() -> cairo::Context {
    let s = cairo::ImageSurface::create(cairo::Format::ARgb32, 1, 1).unwrap();
    cairo::Context::new(&s).unwrap()
}

impl BarEditor {
    pub fn new() -> Rc<BarEditor> {
        let area = gtk::DrawingArea::new();
        area.set_size_request(-1, 330);
        area.set_hexpand(true);
        let e = Rc::new(BarEditor { area, ws: RefCell::new(model::waybar_state(None)), drag: RefCell::new(None),
            drag_pos: Cell::new(None), hover: RefCell::new(None), on_changed: RefCell::new(None) });
        let w = Rc::downgrade(&e);
        e.area.set_draw_func(move |_, cr, width, _| {
            if let Some(e) = w.upgrade() {
                e.paint(cr, width as f64);
            }
        });
        let motion = gtk::EventControllerMotion::new();
        let w = Rc::downgrade(&e);
        motion.connect_motion(move |_, x, y| {
            let Some(e) = w.upgrade() else { return };
            if e.drag.borrow().is_some() {
                e.drag_pos.set(Some((x, y)));
            } else {
                let hit = e.hit(x, y);
                e.area.set_cursor_from_name(Some(if hit.is_some() { "grab" } else { "default" }));
                *e.hover.borrow_mut() = hit.map(|h| h.0);
            }
            e.area.queue_draw();
        });
        let w = Rc::downgrade(&e);
        motion.connect_leave(move |_| {
            if let Some(e) = w.upgrade() {
                *e.hover.borrow_mut() = None;
                e.area.queue_draw();
            }
        });
        e.area.add_controller(motion);
        let drag = gtk::GestureDrag::new();
        drag.set_button(1);
        let w = Rc::downgrade(&e);
        drag.connect_drag_begin(move |_, x, y| {
            let Some(e) = w.upgrade() else { return };
            if let Some((m, side, _)) = e.hit(x, y) {
                *e.drag.borrow_mut() = Some((m, side));
                e.drag_pos.set(Some((x, y)));
                e.area.set_cursor_from_name(Some("grabbing"));
                e.area.queue_draw();
            }
        });
        let w = Rc::downgrade(&e);
        drag.connect_drag_update(move |g, dx, dy| {
            let Some(e) = w.upgrade() else { return };
            if e.drag.borrow().is_some() {
                if let Some((sx, sy)) = g.start_point() {
                    e.drag_pos.set(Some((sx + dx, sy + dy)));
                    e.area.queue_draw();
                }
            }
        });
        let w = Rc::downgrade(&e);
        drag.connect_drag_end(move |g, dx, dy| {
            let Some(e) = w.upgrade() else { return };
            let Some((sx, sy)) = g.start_point() else { return };
            e.release(sx + dx, sy + dy);
        });
        e.area.add_controller(drag);
        let right = gtk::GestureClick::new();
        right.set_button(3);
        let w = Rc::downgrade(&e);
        right.connect_pressed(move |_, _, x, y| {
            let Some(e) = w.upgrade() else { return };
            if let Some((m, side, _)) = e.hit(x, y) {
                if side != "palette" {
                    e.menu(&m, &side, x, y);
                }
            }
        });
        e.area.add_controller(right);
        e
    }

    pub fn connect(&self, f: impl Fn(Lists) + 'static) {
        *self.on_changed.borrow_mut() = Some(Box::new(f));
    }

    fn emit(&self, l: Lists) {
        if let Some(f) = self.on_changed.borrow().as_ref() {
            f(l);
        }
    }

    pub fn load(&self, ws: &Obj) {
        *self.ws.borrow_mut() = ws.clone();
        self.area.queue_draw();
    }

    fn width(&self) -> f64 {
        self.area.width().max(1) as f64
    }

    fn bar_rect(&self, width: f64) -> R {
        R::new(18.0, 46.0, width - 36.0, 52.0)
    }

    /// {seite: [(modul, rect)]} – das gerade gezogene Modul wird ausgelassen.
    fn layout_mods(&self, cr: &cairo::Context, width: f64, skip: Option<&str>) -> Vec<(String, Vec<(String, R)>)> {
        let bar = self.bar_rect(width);
        let (gap, h) = (6.0, bar.h - 16.0);
        let y = bar.y + 8.0;
        let ws = self.ws.borrow();
        let mut out = vec![];
        for side in SIDES {
            let mods: Vec<String> = side_list(&ws, side).into_iter().filter(|m| Some(m.as_str()) != skip).collect();
            let widths: Vec<f64> = mods.iter().map(|m| mod_w(cr, m)).collect();
            let total = widths.iter().sum::<f64>() + gap * (mods.len().max(1) - 1) as f64;
            let mut x = match *side {
                "left" => bar.x + 10.0,
                "center" => bar.cx() - total / 2.0,
                _ => bar.right() - 10.0 - total,
            };
            let mut items = vec![];
            for (m, w) in mods.into_iter().zip(widths) {
                items.push((m, R::new(x, y, w, h)));
                x += w + gap;
            }
            out.push((side.to_string(), items));
        }
        out
    }

    fn palette_items(&self, cr: &cairo::Context, width: f64) -> Vec<(String, R)> {
        let ws = self.ws.borrow();
        let used: HashSet<String> = SIDES.iter().flat_map(|s| side_list(&ws, s)).collect();
        let (mut x, mut y) = (18.0, self.bar_rect(width).bottom() + 70.0);
        let mut items = vec![];
        for m in model::MODULES {
            if used.contains(*m) {
                continue;
            }
            let w = mod_w(cr, m);
            if x + w > width - 18.0 {
                x = 18.0;
                y += 44.0;
            }
            items.push((m.to_string(), R::new(x, y, w, 34.0)));
            x += w + 8.0;
        }
        items
    }

    fn hit(&self, x: f64, y: f64) -> Option<(String, String, R)> {
        let cr = measure_cr();
        let w = self.width();
        let inside = |r: &R| x >= r.x && x < r.right() && y >= r.y && y < r.bottom();
        for (side, items) in self.layout_mods(&cr, w, None) {
            for (m, r) in items {
                if inside(&r) {
                    return Some((m, side, r));
                }
            }
        }
        for (m, r) in self.palette_items(&cr, w) {
            if inside(&r) {
                return Some((m, "palette".into(), r));
            }
        }
        None
    }

    /// (seite, index) für eine Ablage an pos – None = außerhalb der Leiste (entfernen).
    fn target(&self, cr: &cairo::Context, width: f64, x: f64, y: f64) -> Option<(String, usize)> {
        let bar = self.bar_rect(width);
        let a = bar.adj(-10.0, -30.0, 10.0, 30.0);
        if !(x >= a.x && x < a.right() && y >= a.y && y < a.bottom()) {
            return None;
        }
        let third = bar.w / 3.0;
        let i = (((x - bar.x) / third).floor() as i64).clamp(0, 2) as usize;
        let side = SIDES[i];
        let skip = self.drag.borrow().as_ref().map(|d| d.0.clone());
        let lay = self.layout_mods(cr, width, skip.as_deref());
        let items = &lay.iter().find(|(s, _)| s == side).unwrap().1;
        let idx = items.iter().filter(|(_, r)| r.cx() < x).count();
        Some((side.to_string(), idx))
    }

    fn lists_without(&self, m: &str) -> Lists {
        let ws = self.ws.borrow();
        SIDES.iter().map(|s| (s.to_string(), side_list(&ws, s).into_iter().filter(|x| x != m).collect())).collect()
    }

    fn release(&self, x: f64, y: f64) {
        let Some((m, src)) = self.drag.borrow_mut().take() else { return };
        let cr = measure_cr();
        let tgt = self.target(&cr, self.width(), x, y);
        self.drag_pos.set(None);
        self.area.set_cursor_from_name(Some("grab"));
        let mut lists = self.lists_without(&m);
        match tgt {
            Some((side, idx)) => {
                let l = &mut lists.iter_mut().find(|(s, _)| *s == side).unwrap().1;
                l.insert(idx.min(l.len()), m);
            }
            None if src == "palette" => {
                self.area.queue_draw();
                return;
            }
            None => {}
        }
        self.area.queue_draw();
        self.emit(lists);
    }

    fn menu(self: &Rc<Self>, m: &str, side: &str, x: f64, y: f64) {
        let others: Vec<&str> = SIDES.iter().copied().filter(|s| *s != side).collect();
        let mut entries: Vec<MenuEntry> = others.iter().map(|s| MenuEntry::Item(t(&format!("ed.move_{s}")))).collect();
        entries.push(MenuEntry::Sep);
        entries.push(MenuEntry::Item(t("keys.remove")));
        let w = Rc::downgrade(self);
        let m = m.to_string();
        let others: Vec<String> = others.iter().map(|s| s.to_string()).collect();
        dialogs::popup_menu(&self.area, Some((x, y)), entries, move |i| {
            let Some(e) = w.upgrade() else { return };
            let mut lists = e.lists_without(&m);
            if let Some(dst) = others.get(i) {
                lists.iter_mut().find(|(s, _)| s == dst).unwrap().1.push(m.clone());
            }
            e.emit(lists);
        });
    }

    fn paint(&self, cr: &cairo::Context, width: f64) {
        let f = font();
        let ws = self.ws.borrow().clone();
        let bar = self.bar_rect(width);
        let accent = Rgba::hex(&theme::accent_of("tokyo"));
        // Hintergrund-Streifen (Wallpaper) hinter der Leiste
        draw::fill_rounded(cr, R::new(0.0, 0.0, width, bar.bottom() + 30.0), 14.0, Rgba::hex("#22325a"));
        let bg = Rgba::hex(ws.s("bg"));
        let fg = Rgba::hex(ws.s("fg"));
        let acc = Rgba::hex(ws.s("accent"));
        let bw = ws.n("border_width");
        let border = Rgba::hex(ws.s("border"));
        let rad = ws.n("radius").min(bar.h / 2.0);
        let stroke = |cr: &cairo::Context| {
            if bw > 0.0 {
                border.set(cr);
                cr.set_line_width(bw.max(1.0));
                let _ = cr.stroke();
            } else {
                cr.new_path();
            }
        };
        let style = ws.s("style").to_string();
        if style != "islands" {
            let r = if style == "float" { rad } else { 0.0 };
            draw::rounded(cr, bar, r);
            bg.set(cr);
            let _ = cr.fill_preserve();
            stroke(cr);
        }
        let dpos = self.drag_pos.get();
        let drag = self.drag.borrow().clone();
        let dragging = drag.is_some() && dpos.is_some();
        let tgt = if dragging { self.target(cr, width, dpos.unwrap().0, dpos.unwrap().1) } else { None };
        if dragging {
            // Ablagebereiche zeigen
            let third = bar.w / 3.0;
            for (i, side) in SIDES.iter().enumerate() {
                let zr = R::new(bar.x + i as f64 * third + 3.0, bar.y - 26.0, third - 6.0, bar.h + 52.0);
                let on = tgt.as_ref().map(|x| x.0 == *side).unwrap_or(false);
                if on {
                    draw::rounded(cr, zr, 12.0);
                    Rgba::rgb8(61, 139, 253, 28).set(cr);
                    let _ = cr.fill();
                }
                draw::rounded(cr, zr, 12.0);
                if on { accent } else { Rgba::hex(theme::MUTED) }.set(cr);
                cr.set_line_width(1.5);
                cr.set_dash(&[4.5, 3.0], 0.0);
                let _ = cr.stroke();
                cr.set_dash(&[], 0.0);
                draw::text(cr, &f, R::new(zr.x, zr.y + 2.0, zr.w, 20.0), &t(&format!("col.{side}")),
                    Rgba::hex(if on { theme::TEXT } else { theme::MUTED }), HAlign::Center, false);
            }
        }
        let skip = if dragging { drag.as_ref().map(|d| d.0.clone()) } else { None };
        let lay = self.layout_mods(cr, width, skip.as_deref());
        let hover = self.hover.borrow().clone();
        for (_, items) in &lay {
            for (m, r) in items {
                if style == "islands" {
                    let rr = r.adj(-2.0, -4.0, 2.0, 4.0);
                    let rd = ws.n("radius").min(r.h / 2.0);
                    draw::rounded(cr, rr, rd);
                    bg.set(cr);
                    let _ = cr.fill_preserve();
                    stroke(cr);
                }
                if !dragging && hover.as_deref() == Some(m.as_str()) {
                    draw::fill_rounded(cr, *r, 8.0, Rgba::rgb8(255, 255, 255, 26));
                }
                let col = if m == "custom/launcher" {
                    acc
                } else if m == "custom/power" {
                    Rgba::hex("#e5534b")
                } else if ws.b("colorful") {
                    Rgba::hex(&model::module_color(m, &ws))
                } else {
                    fg
                };
                draw::text(cr, &f, *r, &t(&format!("m.{m}")), col, HAlign::Center, false);
            }
        }
        if let Some((side, idx)) = &tgt {
            let items = &lay.iter().find(|(s, _)| s == side).unwrap().1;
            let x = if !items.is_empty() {
                if *idx > 0 { items[idx - 1].1.right() + 3.0 } else { items[0].1.x - 3.0 }
            } else {
                match side.as_str() {
                    "left" => bar.x + 10.0,
                    "center" => bar.cx(),
                    _ => bar.right() - 10.0,
                }
            };
            accent.set(cr);
            cr.set_line_width(3.0);
            cr.set_line_cap(cairo::LineCap::Round);
            cr.move_to(x, bar.y + 6.0);
            cr.line_to(x, bar.bottom() - 6.0);
            let _ = cr.stroke();
        }
        // Palette
        let muted = Rgba::hex(theme::MUTED);
        draw::text(cr, &f, R::new(18.0, bar.bottom() + 44.0, width - 36.0, 18.0), &t("ed.palette"), muted,
            HAlign::Left, false);
        let pal = self.palette_items(cr, width);
        for (m, r) in &pal {
            if dragging && drag.as_ref().map(|d| &d.0) == Some(m) {
                continue;
            }
            draw::rounded(cr, r.adj(0.5, 0.5, -0.5, -0.5), 10.0);
            Rgba::hex(theme::SURFACE2).set(cr);
            let _ = cr.fill_preserve();
            if hover.as_deref() == Some(m.as_str()) { accent } else { Rgba::hex(theme::BORDER) }.set(cr);
            cr.set_line_width(1.0);
            let _ = cr.stroke();
            draw::text(cr, &f, *r, &t(&format!("m.{m}")), Rgba::hex(theme::TEXT), HAlign::Center, false);
        }
        if pal.is_empty() {
            draw::text(cr, &f, R::new(18.0, bar.bottom() + 76.0, width - 36.0, 18.0), &t("ed.all_used"), muted,
                HAlign::Left, false);
        }
        // gezogenes Modul am Mauszeiger
        if let (true, Some((m, _)), Some((px, py))) = (dragging, drag, dpos) {
            let w = mod_w(cr, &m);
            let r = R::new(px - w / 2.0, py - 18.0, w, 36.0);
            cr.push_group();
            draw::rounded(cr, r, 10.0);
            Rgba::hex(theme::SURFACE).set(cr);
            let _ = cr.fill_preserve();
            accent.set(cr);
            cr.set_line_width(2.0);
            let _ = cr.stroke();
            draw::text(cr, &f, r, &t(&format!("m.{m}")), Rgba::hex(theme::TEXT), HAlign::Center, false);
            let _ = cr.pop_group_to_source();
            let _ = cr.paint_with_alpha(0.92);
        }
    }
}

// ---------- Vier Spalten ----------

/// Vier Listen (Verfügbar, Links, Mitte, Rechts), Module per Ziehen verschieben.
pub struct ModuleColumns {
    pub widget: gtk::Box,
    lists: Vec<(String, gtk::Box, gtk::Label)>,
    data: RefCell<Vec<(String, Vec<String>)>>,
    selected: RefCell<Option<String>>,
    on_changed: Cb<()>,
}

pub const COLS: &[&str] = &["avail", "left", "center", "right"];

impl ModuleColumns {
    pub fn new() -> Rc<ModuleColumns> {
        let widget = hbox(10);
        widget.set_homogeneous(true);
        let mut lists = vec![];
        for side in COLS {
            let col = vbox(6);
            let h = label("", "RowLabel");
            let lw = vbox(0);
            lw.add_css_class("ModList");
            lw.set_size_request(-1, 250);
            lw.set_vexpand(true);
            col.append(&h);
            col.append(&lw);
            widget.append(&col);
            lists.push((side.to_string(), lw, h));
        }
        let c = Rc::new(ModuleColumns { widget, lists, data: RefCell::new(vec![]), selected: RefCell::new(None),
            on_changed: RefCell::new(None) });
        for (side, lw, _) in &c.lists {
            let drop = gtk::DropTarget::new(glib::Type::STRING, gdk::DragAction::MOVE);
            let w = Rc::downgrade(&c);
            let side2 = side.clone();
            let lw2 = lw.clone();
            drop.connect_drop(move |_, v, _, y| {
                let Some(c) = w.upgrade() else { return false };
                let Ok(m) = v.get::<String>() else { return false };
                lw2.remove_css_class("drop");
                c.move_to(&m, &side2, c.index_at(&lw2, y));
                true
            });
            let lw3 = lw.clone();
            drop.connect_enter(move |_, _, _| {
                lw3.add_css_class("drop");
                gdk::DragAction::MOVE
            });
            let lw4 = lw.clone();
            drop.connect_leave(move |_| lw4.remove_css_class("drop"));
            lw.add_controller(drop);
        }
        c
    }

    pub fn connect_changed(&self, f: impl Fn(()) + 'static) {
        *self.on_changed.borrow_mut() = Some(Box::new(f));
    }

    fn index_at(&self, lw: &gtk::Box, y: f64) -> usize {
        let mut i = 0;
        let mut ch = lw.first_child();
        while let Some(c) = ch {
            let top = c.compute_point(lw, &gtk::graphene::Point::new(0.0, 0.0)).map(|p| p.y() as f64).unwrap_or(0.0);
            if y < top + c.height() as f64 / 2.0 {
                return i;
            }
            i += 1;
            ch = c.next_sibling();
        }
        i
    }

    fn move_to(self: &Rc<Self>, m: &str, side: &str, idx: usize) {
        let mut moved = false;
        {
            let mut data = self.data.borrow_mut();
            let mut old: Option<(usize, usize)> = None;
            for (si, (_, l)) in data.iter().enumerate() {
                if let Some(p) = l.iter().position(|x| x == m) {
                    old = Some((si, p));
                }
            }
            let Some((osi, op)) = old else { return };
            let dsi = data.iter().position(|(s, _)| s == side).unwrap();
            let mut idx = idx;
            if osi == dsi && op < idx {
                idx -= 1;
            }
            if osi != dsi || op != idx {
                data[osi].1.remove(op);
                let l = &mut data[dsi].1;
                l.insert(idx.min(l.len()), m.to_string());
                moved = true;
            }
        }
        if moved {
            self.rebuild();
            if let Some(f) = self.on_changed.borrow().as_ref() {
                f(());
            }
        }
    }

    pub fn load(self: &Rc<Self>, s: &Obj) {
        let used: HashSet<String> = SIDES.iter().flat_map(|x| s.strs(x)).collect();
        let avail: Vec<String> = model::MODULES.iter().filter(|m| !used.contains(**m)).map(|m| m.to_string()).collect();
        *self.data.borrow_mut() = vec![("avail".into(), avail), ("left".into(), s.strs("left")),
            ("center".into(), s.strs("center")), ("right".into(), s.strs("right"))];
        self.rebuild();
    }

    fn rebuild(self: &Rc<Self>) {
        let data = self.data.borrow().clone();
        let sel = self.selected.borrow().clone();
        for ((_, items), (_, lw, _)) in data.iter().zip(&self.lists) {
            while let Some(c) = lw.first_child() {
                lw.remove(&c);
            }
            for m in items {
                let l = label(&t(&format!("m.{m}")), "ModItem");
                if sel.as_deref() == Some(m.as_str()) {
                    l.add_css_class("sel");
                }
                l.set_cursor_from_name(Some("grab"));
                let src = gtk::DragSource::new();
                src.set_actions(gdk::DragAction::MOVE);
                let mm = m.clone();
                src.connect_prepare(move |_, _, _| Some(gdk::ContentProvider::for_value(&mm.to_value())));
                let l2 = l.clone();
                src.connect_drag_begin(move |s, _| {
                    let p = gtk::WidgetPaintable::new(Some(&l2));
                    s.set_icon(Some(&p), 10, 10);
                });
                l.add_controller(src);
                let click = gtk::GestureClick::new();
                let w = Rc::downgrade(self);
                let mm = m.clone();
                click.connect_released(move |_, _, _, _| {
                    if let Some(c) = w.upgrade() {
                        *c.selected.borrow_mut() = Some(mm.clone());
                        c.rebuild();
                    }
                });
                l.add_controller(click);
                lw.append(&l);
            }
        }
    }

    pub fn read(&self) -> Obj {
        let mut out = Obj::new();
        for (side, l) in self.data.borrow().iter().skip(1) {
            let mut seen: Vec<String> = vec![];
            for m in l {
                if model::MODULES.contains(&m.as_str()) && !seen.contains(m) {
                    seen.push(m.clone());
                }
            }
            out.insert(side.clone(), json!(seen));
        }
        out
    }

    pub fn retranslate(self: &Rc<Self>) {
        for (side, _, h) in &self.lists {
            h.set_text(&t(&format!("col.{side}")));
        }
        self.widget.set_tooltip_text(Some(&t("tip.modules")));
        self.rebuild();
    }
}

#[allow(dead_code)]
fn _v(_: Value) {}
