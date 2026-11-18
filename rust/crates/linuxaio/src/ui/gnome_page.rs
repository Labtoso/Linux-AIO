//! GNOME-Bereiche im Tab „Einstellungen“: wie GNOME Tweaks plus Titelleisten-Knöpfe, obere Leiste,
//! Startprogramme und Anmeldebildschirm (GDM).

use super::form::{backup_files, combo, flow_grid, missing_button, remove_button, restore_files, tbtn, Page, Row};
use super::img;
use super::widgets::{chip, search_entry, seg, set_applied, set_chip, ChipRow};
use super::win::{win, AioPage};
use crate::i18n::{t, tf};
use crate::{buttons, corners, gnome, installers, usercmd};
use gtk::prelude::*;
use gtk::{cairo, glib, pango};
use hyprstudio::kw;
use hyprstudio::ui::draw::{self, Font, HAlign, Rgba, R};
use hyprstudio::ui::shell;
use hyprstudio::ui::theme;
use hyprstudio::ui::widgets::{cursor_hand, hbox, label, vbox, ComboItem, SearchCombo, Segmented, Switch};
use serde_json::{json, Map, Value};
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};

fn ext_name(uuid: &str) -> String {
    installers::ext_name(uuid).unwrap_or(uuid).to_string()
}

fn truthy(v: &Value) -> bool {
    hyprstudio::jsonx::truthy(v)
}

// ---------- einfache Seite (nur gsettings) ----------

pub struct SimplePage {
    pub page: Rc<Page>,
}

pub fn simple_page(key: &str, section: &str) -> Rc<SimplePage> {
    let page = Page::new(&format!("gn.{key}"), true);
    let card = page.card("");
    page.gsection(&card, gnome::section(section));
    page.finish();
    Rc::new(SimplePage { page })
}

impl AioPage for SimplePage {
    fn widget(&self) -> gtk::Widget {
        self.page.root.clone().upcast()
    }
    fn retranslate(&self) {
        self.page.retranslate();
    }
    fn refresh(&self) {
        self.page.refresh();
    }
}

// ---------- fehlende Erweiterung per Klick ----------

/// Erweiterung von extensions.gnome.org holen, sonst Paket per pkexec (Plan B).
fn install_ext(uuid: &str, me: Rc<dyn AioPage>, pending: Rc<RefCell<Option<String>>>) {
    let w = win();
    let name = ext_name(uuid);
    w.log("");
    w.task_begin(&tf("miss.installing", kw!("name" => name.clone())));
    let u = uuid.to_string();
    let u2 = u.clone();
    shell::run_bg(move || installers::install_extension(&u).map_err(|e| format!("{}: {}", e.key, e.detail)).map(|r| (u, r)),
        move |res| {
            let w = win();
            w.task_end(true, "");
            match res {
                Ok((_, r)) => {
                    let key = match r.as_str() {
                        "relogin" => "miss.relogin",
                        "incompatible" => "miss.incompatible",
                        _ => "miss.done",
                    };
                    w.set_status(&tf(key, kw!("name" => name)));
                    me.refresh();
                }
                Err(e) => {
                    w.log(&e);
                    match installers::ext_package(&u2) {
                        Some(key) => {
                            *pending.borrow_mut() = Some(u2.clone());
                            w.run(vec!["pkg-install".into(), key.into()], "pkg-install", Some(me.clone()));
                        }
                        None => w.set_status(&format!("{}: {}", t("failed"), e)),
                    }
                }
            }
        });
}


fn uninstall_ext(uuid: &str, me: Rc<dyn AioPage>) {
    let name = ext_name(uuid);
    let u = uuid.to_string();
    win().confirm(&t("ext.uninstall"), &tf("ext.uninstall.ask", kw!("name" => name.clone())), move |ok| {
        if !ok {
            return;
        }
        let w = win();
        w.log("");
        gnome::ext_uninstall(&u);
        w.set_status(&tf("ext.uninstalled", kw!("name" => name)));
        me.refresh();
    });
}

fn ext_finished(ok: bool, action: &str, pending: &RefCell<Option<String>>) {
    let uuid = pending.borrow_mut().take();
    if action == "pkg-install" && ok {
        if let Some(u) = uuid {
            usercmd::run(&["gnome-extensions", "enable", &u]);
            win().set_status(&tf("miss.relogin", kw!("name" => ext_name(&u))));
        }
    }
}

fn combo_fill(c: &SearchCombo, names: &[String], cur: &str) {
    let items: Vec<ComboItem> = names.iter().map(|n| ComboItem::new(n, n)).collect();
    c.set_items(items);
    c.set_current(c.find_data(cur).max(0));
}

// ---------- Erscheinungsbild ----------

pub struct LookGnome {
    page: Rc<Page>,
    gtk: Rc<SearchCombo>,
    r_gtk: Rc<Row>,
    shell_chip: gtk::Label,
    shell_get: gtk::Button,
    shell_rm: gtk::Button,
    shell: Rc<SearchCombo>,
    r_shell: Rc<Row>,
    pending: Rc<RefCell<Option<String>>>,
    me: RefCell<Weak<LookGnome>>,
}

impl LookGnome {
    pub fn new() -> Rc<LookGnome> {
        let page = Page::new("gn.look", true);
        let card = page.card("gn.look.style");
        page.gsection(&card, gnome::section("look"));
        let themes = page.card("gn.look.themes");
        let gtkc = combo();
        let r_gtk = Row::new("gn.gtk", &gtkc.button);
        themes.append(&r_gtk.root);
        let sw = hbox(8);
        let shell_chip = chip("", "Chip");
        let shell_get = missing_button();
        let shell_rm = remove_button();
        let shellc = combo();
        sw.append(&shell_chip);
        sw.append(&shell_get);
        sw.append(&shell_rm);
        sw.append(&shellc.button);
        let r_shell = Row::new("gn.shell", &sw);
        themes.append(&r_shell.root);
        *page.keys_override.borrow_mut() = Some({
            let mut k = page.reset_keys();
            k.push((gnome::IFACE.into(), "gtk-theme".into()));
            k.push(("org.gnome.shell.extensions.user-theme".into(), "name".into()));
            k
        });
        page.finish();
        let p = Rc::new(LookGnome { page, gtk: gtkc, r_gtk, shell_chip, shell_get, shell_rm, shell: shellc, r_shell,
            pending: Rc::new(RefCell::new(None)), me: RefCell::new(Weak::new()) });
        *p.me.borrow_mut() = Rc::downgrade(&p);
        let g2 = Rc::downgrade(&p.gtk);
        p.gtk.connect(move |_| {
            if let Some(c) = g2.upgrade() {
                usercmd::gset(gnome::IFACE, "gtk-theme", json!(c.current_data()));
                win().set_status(&t("done"));
            }
        });
        let w = Rc::downgrade(&p);
        p.shell.connect(move |_| {
            if let Some(p) = w.upgrade() {
                let name = p.shell.current_data();
                gnome::set_shell_theme(&name);
                win().set_status(&t("done"));
                p.refresh();
            }
        });
        let w = Rc::downgrade(&p);
        p.shell_get.connect_clicked(move |_| {
            if let Some(p) = w.upgrade() {
                install_ext(gnome::USER_THEME, p.clone(), p.pending.clone());
            }
        });
        let w = Rc::downgrade(&p);
        p.shell_rm.connect_clicked(move |_| {
            if let Some(p) = w.upgrade() {
                uninstall_ext(gnome::USER_THEME, p.clone());
            }
        });
        p
    }
}

impl AioPage for LookGnome {
    fn widget(&self) -> gtk::Widget {
        self.page.root.clone().upcast()
    }

    fn retranslate(&self) {
        self.page.retranslate();
        self.r_gtk.retranslate();
        self.r_shell.retranslate();
    }

    fn refresh(&self) {
        self.page.load_controls();
        let mut cur = usercmd::gget_str(gnome::IFACE, "gtk-theme");
        if cur.is_empty() {
            cur = "Adwaita".into();
        }
        let mut names = gnome::list_themes("gtk");
        if !names.contains(&cur) {
            names.push(cur.clone());
        }
        combo_fill(&self.gtk, &names, &cur);
        let ext_ok = gnome::ext_installed(gnome::USER_THEME);
        let on = gnome::ext_enabled().iter().any(|x| x == gnome::USER_THEME);
        set_chip(&self.shell_chip, if on { "ChipOk" } else { "ChipWarn" },
            &if on { t("gn.ext.on") } else if ext_ok { t("gn.ext.off") } else { t("gn.ext.missing") });
        self.shell_chip.set_tooltip_text(Some(&t("gn.shell.tip")));
        self.shell_get.set_visible(!ext_ok);
        self.shell_rm.set_visible(ext_ok && gnome::ext_removable(gnome::USER_THEME));
        self.shell_get.set_sensitive(!win().busy());
        self.shell_get.set_tooltip_text(Some(&format!("{}: User Themes", t("miss.install"))));
        let mut items = vec![ComboItem::new(&t("gn.default"), "")];
        items.extend(gnome::list_themes("shell").iter().map(|n| ComboItem::new(n, n)));
        self.shell.set_items(items);
        let cs = gnome::shell_theme();
        self.shell.set_current(self.shell.find_data(&cs).max(0));
        self.shell.set_sensitive(ext_ok);
    }

    fn finished(&self, ok: bool, action: &str, _r: &str) {
        ext_finished(ok, action, &self.pending);
        self.refresh();
    }
}

/// Dateien (Nautilus) läuft im Hintergrund weiter und liest gtk.css nur beim Start.
fn restart_files() {
    usercmd::run(&["nautilus", "-q"]);
}

// ---------- Fenster (mit Eckenrundung) ----------

fn corner_preview() -> (gtk::DrawingArea, Rc<Cell<i32>>) {
    let area = gtk::DrawingArea::new();
    area.set_content_height(120);
    area.set_hexpand(true);
    let radius = Rc::new(Cell::new(15));
    let r2 = radius.clone();
    area.set_draw_func(move |_, cr, wd, ht| {
        let bg = R::new(1.0, 1.0, wd as f64 - 2.0, ht as f64 - 2.0);
        draw::fill_rounded(cr, bg, 12.0, Rgba::hex("#3b5470"));
        let rad = r2.get() as f64;
        let w = R::new(bg.cx() - 160.0, bg.y + 16.0, 320.0, bg.h - 32.0);
        draw::fill_rounded(cr, w, rad, Rgba::hex("#242a32"));
        cr.save().ok();
        draw::rounded(cr, w, rad);
        cr.clip();
        draw::fill_rect(cr, R::new(w.x, w.y, w.w, 30.0), Rgba::hex("#2e3540"));
        cr.restore().ok();
        let f = Font::new(img::pt(10.0), pango::Weight::Normal);
        draw::text(cr, &f, R::new(w.x, w.y, w.w, 30.0), &format!("{} px", r2.get()), Rgba::hex(theme::TEXT),
            HAlign::Center, false);
    });
    (area, radius)
}

pub struct WindowsPage {
    page: Rc<Page>,
    preview: gtk::DrawingArea,
    radius: Rc<Cell<i32>>,
    slider: gtk::Scale,
    value: gtk::Label,
    b_orig: gtk::Button,
    r_radius: Rc<Row>,
    ext_chip: gtk::Label,
    ext_get: gtk::Button,
    ext_rm: gtk::Button,
    sw_ext: Rc<Switch>,
    b_force: gtk::Button,
    r_ext: Rc<Row>,
    timer: RefCell<Option<glib::SourceId>>,
    loading: Cell<bool>,
    pending: Rc<RefCell<Option<String>>>,
    me: RefCell<Weak<WindowsPage>>,
}

impl WindowsPage {
    pub fn new() -> Rc<WindowsPage> {
        let page = Page::new("gn.windows", true);
        let card = page.card("");
        page.gsection(&card, gnome::section("windows"));
        let rc = page.card("gn.corners");
        let (preview, radius) = corner_preview();
        rc.append(&preview);
        let sw = hbox(8);
        let slider = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 24.0, 1.0);
        slider.set_draw_value(false);
        slider.set_size_request(120, -1);
        slider.set_hexpand(true);
        hyprstudio::ui::widgets::no_wheel(&slider);
        let value = label("", "");
        value.set_size_request(48, -1);
        let b_orig = tbtn("gn.login.restore", false);
        let b_files = tbtn("gn.files.restart", false);
        b_files.connect_clicked(|_| restart_files());
        sw.append(&slider);
        sw.append(&value);
        sw.append(&b_orig);
        sw.append(&b_files);
        sw.set_hexpand(true);
        let r_radius = Row::new("gn.corners.radius", &sw);
        rc.append(&r_radius.root);
        let ew = hbox(8);
        let ext_chip = chip("", "Chip");
        let ext_get = missing_button();
        let ext_rm = remove_button();
        let sw_ext = Switch::new();
        let b_force = tbtn("gn.ext.force", false);
        ew.append(&ext_chip);
        ew.append(&ext_get);
        ew.append(&ext_rm);
        ew.append(&b_force);
        ew.append(&sw_ext.area);
        let r_ext = Row::new("gn.corners.all", &ew);
        rc.append(&r_ext.root);
        page.has_reset.set(true);
        page.finish();
        let p = Rc::new(WindowsPage { page, preview, radius, slider, value, b_orig, r_radius, ext_chip, ext_get, ext_rm,
            sw_ext, b_force, r_ext, timer: RefCell::new(None), loading: Cell::new(false),
            pending: Rc::new(RefCell::new(None)), me: RefCell::new(Weak::new()) });
        *p.me.borrow_mut() = Rc::downgrade(&p);
        *p.page.reset_extra.borrow_mut() = Some(Rc::new(|bak: &Path| {
            backup_files(&corners::files(), bak);
            corners::remove(&|s| usercmd::log(s));
        }));
        *p.page.undo_extra.borrow_mut() = Some(Rc::new(|bak: &Path| restore_files(&corners::files(), bak)));
        let w = Rc::downgrade(&p);
        *p.page.on_refresh.borrow_mut() = Some(Rc::new(move || {
            if let Some(p) = w.upgrade() {
                p.refresh();
            }
        }));
        let w = Rc::downgrade(&p);
        p.slider.connect_value_changed(move |s| {
            let Some(p) = w.upgrade() else { return };
            if p.loading.get() {
                return;
            }
            let v = s.value().round() as i32;
            p.radius.set(v);
            p.preview.queue_draw();
            p.value.set_text(&format!("{v} px"));
            if let Some(id) = p.timer.borrow_mut().take() {
                id.remove();
            }
            let w2 = Rc::downgrade(&p);
            let id = glib::timeout_add_local_once(std::time::Duration::from_millis(400), move || {
                if let Some(p) = w2.upgrade() {
                    p.timer.borrow_mut().take();
                    p.apply_corners();
                }
            });
            *p.timer.borrow_mut() = Some(id);
        });
        let w = Rc::downgrade(&p);
        p.b_orig.connect_clicked(move |_| {
            if let Some(p) = w.upgrade() {
                p.reset_corners();
            }
        });
        let w = Rc::downgrade(&p);
        p.ext_get.connect_clicked(move |_| {
            if let Some(p) = w.upgrade() {
                install_ext(corners::EXT_UUID, p.clone(), p.pending.clone());
            }
        });
        let w = Rc::downgrade(&p);
        p.ext_rm.connect_clicked(move |_| {
            if let Some(p) = w.upgrade() {
                uninstall_ext(corners::EXT_UUID, p.clone());
            }
        });
        let w = Rc::downgrade(&p);
        p.b_force.connect_clicked(move |_| {
            let Some(p) = w.upgrade() else { return };
            win().confirm(&t("gn.ext.force"), &t("gn.ext.force.ask"), move |ok| {
                if !ok {
                    return;
                }
                usercmd::gset("org.gnome.shell", "disable-extension-version-validation", json!(true));
                gnome::ext_set(corners::EXT_UUID, true);
                corners::set_extension_radius(p.slider.value().round() as i64);
                win().set_status(&tf("miss.relogin", kw!("name" => "Rounded Window Corners Reborn")));
                p.refresh();
            });
        });
        let w = Rc::downgrade(&p);
        p.sw_ext.connect(move |on| {
            let Some(p) = w.upgrade() else { return };
            gnome::ext_set(corners::EXT_UUID, on);
            if on {
                corners::set_extension_radius(p.slider.value().round() as i64);
            }
            win().set_status(&t("done"));
        });
        p
    }

    fn apply_corners(&self) {
        let v = self.slider.value().round() as i64;
        corners::apply(v, &|s| usercmd::log(s));
        if gnome::ext_installed(corners::EXT_UUID) && gnome::ext_enabled().iter().any(|x| x == corners::EXT_UUID) {
            corners::set_extension_radius(v);
        }
        self.b_orig.set_sensitive(true);
        win().set_status(&tf("gn.corners.done", kw!("r" => v)));
    }

    fn set_slider(&self, v: i32) {
        self.loading.set(true);
        self.slider.set_value(v as f64);
        self.loading.set(false);
        self.radius.set(v);
        self.preview.queue_draw();
        self.value.set_text(&format!("{v} px"));
    }

    fn reset_corners(&self) {
        corners::remove(&|s| usercmd::log(s));
        self.set_slider(15);
        self.b_orig.set_sensitive(false);
        win().set_status(&t("gn.corners.reset"));
    }
}

impl AioPage for WindowsPage {
    fn widget(&self) -> gtk::Widget {
        self.page.root.clone().upcast()
    }

    fn retranslate(&self) {
        self.page.retranslate();
        self.r_radius.retranslate();
        self.r_ext.retranslate();
        self.r_radius.label.set_tooltip_text(Some(&t("gn.corners.note"))); // Hinweise nur als Tooltip
    }

    fn refresh(&self) {
        self.page.load_controls();
        let cur = corners::current();
        self.set_slider(cur.unwrap_or(15) as i32);
        self.b_orig.set_sensitive(cur.is_some());
        let have = gnome::ext_installed(corners::EXT_UUID);
        let on = gnome::ext_enabled().iter().any(|x| x == corners::EXT_UUID);
        let compat = gnome::ext_compatible(corners::EXT_UUID);
        let blocked = have && compat == Some(false) && !gnome::version_check_off();
        if blocked {
            set_chip(&self.ext_chip, "ChipWarn", &t("gn.ext.incompatible"));
        } else {
            set_chip(&self.ext_chip, if on { "ChipOk" } else if have { "Chip" } else { "ChipWarn" },
                &if on { t("gn.ext.on") } else if have { t("gn.ext.off") } else { t("gn.ext.missing") });
        }
        self.b_force.set_visible(blocked);
        self.b_force.set_tooltip_text(Some(&t("gn.ext.force.ask")));
        self.ext_chip.set_tooltip_text(Some("Rounded Window Corners Reborn"));
        self.ext_get.set_visible(!have);
        self.ext_rm.set_visible(have && gnome::ext_removable(corners::EXT_UUID));
        self.ext_get.set_tooltip_text(Some(&format!("{}: Rounded Window Corners Reborn", t("miss.install"))));
        self.sw_ext.area.set_sensitive(have && !blocked);
        self.sw_ext.set_now(on);
    }

    fn finished(&self, ok: bool, action: &str, _r: &str) {
        ext_finished(ok, action, &self.pending);
        self.refresh();
    }
}

// ---------- Titelleiste ----------

/// Gezeichnete Titelleiste: Knöpfe je nach Layout und Knopf-Theme.
pub struct TitlebarPreview {
    pub area: gtk::DrawingArea,
    pub layout: RefCell<gnome::Layout>,
    pub style: RefCell<Option<String>>,
}

fn lightness(c: &str) -> f64 {
    let (r, g, b) = theme::rgb(c);
    let mx = r.max(g).max(b);
    let mn = r.min(g).min(b);
    (mx + mn) / 2.0 * 255.0
}

impl TitlebarPreview {
    pub fn new(height: i32, title: bool) -> Rc<TitlebarPreview> {
        let area = gtk::DrawingArea::new();
        area.set_content_height(height);
        area.set_hexpand(true);
        let p = Rc::new(TitlebarPreview { area,
            layout: RefCell::new(gnome::Layout { side: "right".into(), minimize: false, maximize: false, close: true,
                appmenu: true }),
            style: RefCell::new(None) });
        let w = Rc::downgrade(&p);
        p.area.set_draw_func(move |_, cr, wd, ht| {
            if let Some(p) = w.upgrade() {
                p.paint(cr, wd as f64, ht as f64, title);
            }
        });
        p
    }

    pub fn set_layout(&self, l: gnome::Layout) {
        *self.layout.borrow_mut() = l;
        self.area.queue_draw();
    }

    pub fn set_style(&self, s: Option<String>) {
        *self.style.borrow_mut() = s;
        self.area.queue_draw();
    }

    fn paint(&self, cr: &cairo::Context, w: f64, h: f64, title: bool) {
        let r = R::new(1.0, 1.0, w - 2.0, h - 2.0);
        draw::fill_rounded(cr, r, 12.0, Rgba::hex("#242a32"));
        draw::rounded(cr, R::new(r.x + 0.5, r.y + 0.5, r.w - 1.0, r.h - 1.0), 12.0);
        Rgba::hex(theme::BORDER).set(cr);
        cr.set_line_width(1.0);
        let _ = cr.stroke();
        if title {
            let f = Font::new(img::pt(10.0), pango::Weight::Bold);
            draw::text(cr, &f, r, &t("gn.title.window"), Rgba::hex(theme::TEXT), HAlign::Center, false);
        }
        let d = self.layout.borrow().clone();
        let mut btns: Vec<&str> = vec![];
        if d.close {
            btns.push("close");
        }
        if d.maximize {
            btns.push("maximize");
        }
        if d.minimize {
            btns.push("minimize");
        }
        if d.side == "right" {
            btns.reverse();
        }
        let style = self.style.borrow().clone();
        let fallback: Map<String, Value> = json!({"shape": "circle", "size": 26, "close": "#3a424d", "min": "#3a424d",
            "max": "#3a424d", "icons": "always"}).as_object().cloned().unwrap_or_default();
        let st = style.as_deref().and_then(buttons::theme).cloned().unwrap_or(fallback);
        let s = |k: &str| st.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let shape = s("shape");
        let k = r.h / 64.0;
        let size = (st.get("size").and_then(|v| v.as_f64()).unwrap_or(24.0) * 1.1 * k).max(10.0);
        let bw = if shape == "wide" { size * 2.0 } else { size };
        let gap = if shape != "wide" { 8.0 * k } else { 0.0 };
        let n = btns.len() as f64;
        let total = n * bw + (n - 1.0).max(0.0) * gap;
        let mut x = if d.side == "left" { r.x + 16.0 * k } else { r.right() - 16.0 * k - total };
        if shape == "wide" {
            x = if d.side == "left" { r.x + 4.0 } else { r.right() - 4.0 - total };
        }
        let y = r.cy() - size / 2.0;
        let icons = s("icons");
        for b in btns {
            let c = R::new(x, y, bw, size);
            let col = s(match b { "close" => "close", "maximize" => "max", _ => "min" });
            if !col.is_empty() {
                let rgba = Rgba::hex(&col);
                if shape == "circle" {
                    draw::fill_ellipse(cr, c.cx(), c.cy(), c.w / 2.0, c.h / 2.0, rgba);
                } else {
                    draw::fill_rounded(cr, c, if shape == "square" { 4.0 } else { 0.0 }, rgba);
                }
            }
            if icons != "never" && (icons == "always" || col.is_empty()) {
                let light = col.is_empty() || lightness(&col) < 128.0;
                Rgba::hex(if light { theme::TEXT } else { "#1b1b1b" }).set(cr);
                cr.set_line_width(1.6);
                cr.set_line_cap(cairo::LineCap::Round);
                let m = bw.min(size) * 0.33;
                let ic = R::new(c.cx() - m / 2.0, c.cy() - m / 2.0, m, m);
                match b {
                    "close" => {
                        cr.move_to(ic.x, ic.y);
                        cr.line_to(ic.right(), ic.bottom());
                        cr.move_to(ic.right(), ic.y);
                        cr.line_to(ic.x, ic.bottom());
                    }
                    "maximize" => cr.rectangle(ic.x, ic.y, ic.w, ic.h),
                    _ => {
                        cr.move_to(ic.x, ic.bottom());
                        cr.line_to(ic.right(), ic.bottom());
                    }
                }
                let _ = cr.stroke();
            }
            x += bw + gap;
        }
    }
}

/// Kachel eines Knopf-Themes: Vorschau + Installieren/Angewendet.
pub struct ButtonTile {
    root: gtk::Box,
    name: String,
    pre: Rc<TitlebarPreview>,
    btn: gtk::Button,
}

impl ButtonTile {
    fn new(name: &str) -> Rc<ButtonTile> {
        let root = vbox(8);
        root.add_css_class("ATile");
        root.set_size_request(262, 150);
        root.add_css_class("Pad10");
        let pre = TitlebarPreview::new(56, false);
        pre.set_style(Some(name.into()));
        root.append(&pre.area);
        let titel = buttons::theme(name).and_then(|p| p.get("titel")).and_then(|v| v.as_str()).unwrap_or(name).to_string();
        let lb = label(&titel, "name");
        lb.set_xalign(0.0);
        root.append(&lb);
        let btn = gtk::Button::with_label("");
        btn.add_css_class("Primary");
        cursor_hand(&btn);
        btn.set_size_request(-1, 30);
        root.append(&btn);
        Rc::new(ButtonTile { root, name: name.into(), pre, btn })
    }

    fn refresh(&self, layout: &gnome::Layout, current: &str) {
        let mut l = layout.clone();
        l.minimize = true;
        l.maximize = true;
        l.close = true;
        self.pre.set_layout(l);
        let active = current == self.name;
        set_applied(&self.btn, active, &t("applied"));
        if !active {
            self.btn.set_label(&t("install"));
            self.btn.set_sensitive(true);
        }
        if active {
            self.root.add_css_class("act");
        } else {
            self.root.remove_css_class("act");
        }
    }
}

pub struct TitlebarPage {
    page: Rc<Page>,
    preview: Rc<TitlebarPreview>,
    sw_min: Rc<Switch>,
    sw_max: Rc<Switch>,
    sw_close: Rc<Switch>,
    rows: Vec<Rc<Row>>,
    b_left: gtk::ToggleButton,
    b_right: gtk::ToggleButton,
    warn: gtk::Label,
    l_themes: gtk::Label,
    b_orig: gtk::Button,
    support: Rc<ChipRow>,
    sw_moz: Rc<Switch>,
    r_moz: Rc<Row>,
    tiles: Vec<Rc<ButtonTile>>,
    appmenu: Cell<bool>,
    loading: Cell<bool>,
}

fn css_files() -> Vec<PathBuf> {
    let mut f = buttons::files();
    for prof in buttons::moz_profiles() {
        let (a, b) = buttons::moz_targets(&prof);
        f.push(a);
        f.push(b);
    }
    f
}

impl TitlebarPage {
    pub fn new() -> Rc<TitlebarPage> {
        let w = win();
        let page = Page::new("gn.titlebar", true);
        let card = page.card("");
        let preview = TitlebarPreview::new(64, true);
        card.append(&preview.area);
        preview.area.set_margin_bottom(6);
        let sw_min = Switch::new();
        let sw_max = Switch::new();
        let sw_close = Switch::new();
        let r_min = Row::new("gn.btn.min", &sw_min.area);
        let r_max = Row::new("gn.btn.max", &sw_max.area);
        let r_close = Row::new("gn.btn.close", &sw_close.area);
        let side = hbox(6);
        let b_left = gtk::ToggleButton::new();
        let b_right = gtk::ToggleButton::new();
        b_right.set_group(Some(&b_left));
        for b in [&b_left, &b_right] {
            b.add_css_class("Lang");
            cursor_hand(b);
            side.append(b);
        }
        let r_side = Row::new("gn.btn.side", &side);
        for r in [&r_min, &r_max, &r_close, &r_side] {
            card.append(&r.root);
        }
        let warn = label("", "WarnText");
        warn.set_wrap(true);
        warn.set_xalign(0.0);
        card.append(&warn);
        // Knopf-Themes
        let head = hbox(8);
        head.set_margin_top(6);
        let l_themes = label("", "ACardTitle");
        let b_orig = tbtn("gn.login.restore", false);
        let b_files = tbtn("gn.files.restart", false);
        b_files.connect_clicked(|_| restart_files());
        head.append(&l_themes);
        let fill = hbox(0);
        fill.set_hexpand(true);
        head.append(&fill);
        head.append(&b_files);
        head.append(&b_orig);
        page.body.append(&head);
        let support = ChipRow::new();
        page.body.append(&support.root);
        let sw_moz = Switch::new();
        let profs = !buttons::moz_profiles().is_empty();
        let moz_on = w.cfg.borrow().get("moz_buttons").map(truthy).unwrap_or(true);
        sw_moz.set_now(moz_on && profs);
        sw_moz.area.set_sensitive(profs);
        let r_moz = Row::new("gn.btns.moz", &sw_moz.area);
        page.body.append(&r_moz.root);
        let grid = flow_grid();
        let mut tiles = vec![];
        for name in buttons::names() {
            let tile = ButtonTile::new(&name);
            grid.insert(&tile.root, -1);
            tiles.push(tile);
        }
        page.body.append(&grid);
        *page.keys_override.borrow_mut() = Some(vec![(gnome::WM.into(), "button-layout".into())]);
        page.finish();
        let p = Rc::new(TitlebarPage { page, preview, sw_min, sw_max, sw_close, rows: vec![r_min, r_max, r_close, r_side],
            b_left, b_right, warn, l_themes, b_orig, support, sw_moz, r_moz, tiles, appmenu: Cell::new(true),
            loading: Cell::new(false) });
        *p.page.reset_extra.borrow_mut() = Some(Rc::new(|bak: &Path| {
            backup_files(&css_files(), bak);
            buttons::remove(&|s| usercmd::log(s));
            buttons::remove_moz(&|s| usercmd::log(s));
        }));
        *p.page.undo_extra.borrow_mut() = Some(Rc::new(|bak: &Path| restore_files(&css_files(), bak)));
        let wp = Rc::downgrade(&p);
        *p.page.on_refresh.borrow_mut() = Some(Rc::new(move || {
            if let Some(p) = wp.upgrade() {
                p.refresh();
            }
        }));
        for sw in [&p.sw_min, &p.sw_max, &p.sw_close] {
            let wp = Rc::downgrade(&p);
            sw.connect(move |_| {
                if let Some(p) = wp.upgrade() {
                    p.write();
                }
            });
        }
        for b in [&p.b_left, &p.b_right] {
            let wp = Rc::downgrade(&p);
            b.connect_toggled(move |b| {
                if let Some(p) = wp.upgrade() {
                    if b.is_active() && !p.loading.get() {
                        p.write();
                    }
                }
            });
        }
        for tile in &p.tiles {
            let wp = Rc::downgrade(&p);
            let name = tile.name.clone();
            tile.btn.connect_clicked(move |_| {
                if let Some(p) = wp.upgrade() {
                    p.apply_buttons(&name);
                }
            });
        }
        let wp = Rc::downgrade(&p);
        p.b_orig.connect_clicked(move |_| {
            if let Some(p) = wp.upgrade() {
                let w = win();
                w.log("");
                buttons::remove(&|s| win().log(s));
                buttons::remove_moz(&|s| win().log(s));
                w.set_status(&t("gn.btns.restart"));
                p.tiles_refresh();
            }
        });
        p.sw_moz.connect(|on| {
            let w = win();
            w.cfg.borrow_mut().insert("moz_buttons".into(), json!(on));
            w.save();
            let cur = buttons::current();
            if on && !cur.is_empty() {
                buttons::apply_moz(&cur, &|s| win().log(s));
            } else if !on {
                buttons::remove_moz(&|s| win().log(s));
            }
            w.set_status(&t("gn.btns.restart"));
        });
        p
    }

    fn state(&self) -> gnome::Layout {
        gnome::Layout { side: if self.b_left.is_active() { "left".into() } else { "right".into() },
            minimize: self.sw_min.is_on(), maximize: self.sw_max.is_on(), close: self.sw_close.is_on(),
            appmenu: self.appmenu.get() }
    }

    fn write(&self) {
        let d = self.state();
        self.preview.set_layout(d.clone());
        let value = gnome::build_layout(&d.side, d.minimize, d.maximize, self.appmenu.get(), d.close);
        usercmd::gset(gnome::WM, "button-layout", json!(value));
        self.warn.set_visible(!d.close);
        win().set_status(&t("done"));
        self.tiles_refresh();
    }

    fn apply_buttons(&self, name: &str) {
        let w = win();
        w.log("");
        buttons::apply(name, &|s| win().log(s));
        if self.sw_moz.is_on() {
            buttons::apply_moz(name, &|s| win().log(s));
        }
        w.set_status(&t("gn.btns.restart"));
        self.tiles_refresh();
    }

    fn tiles_refresh(&self) {
        let cur = buttons::current();
        self.preview.set_style(if cur.is_empty() { None } else { Some(cur.clone()) });
        let st = self.state();
        for tl in &self.tiles {
            tl.refresh(&st, &cur);
        }
        self.b_orig.set_sensitive(!cur.is_empty());
    }
}

impl AioPage for TitlebarPage {
    fn widget(&self) -> gtk::Widget {
        self.page.root.clone().upcast()
    }

    fn retranslate(&self) {
        self.page.retranslate();
        for r in &self.rows {
            r.retranslate();
        }
        self.b_left.set_label(&t("gn.left"));
        self.b_right.set_label(&t("gn.right"));
        self.warn.set_text(&t("gn.btn.noclose"));
        self.l_themes.set_text(&t("gn.btns"));
        self.l_themes.set_tooltip_text(Some(&t("gn.btns.note")));
        self.r_moz.retranslate();
        self.support.set(&[(t("gn.sup.gtk"), "ChipOk".into(), String::new()),
            (t("gn.sup.moz"), "ChipOk".into(), t("gn.sup.moz.tip")),
            (t("gn.sup.chromium"), "ChipWarn".into(), t("gn.sup.chromium.tip")),
            (t("gn.sup.electron"), "Chip".into(), t("gn.sup.electron.tip")),
            (t("gn.sup.qt"), "Chip".into(), t("gn.sup.qt.tip"))]);
        self.preview.area.queue_draw();
    }

    fn refresh(&self) {
        let mut v = usercmd::gget_str(gnome::WM, "button-layout");
        if v.is_empty() {
            v = "appmenu:close".into();
        }
        let d = gnome::parse_layout(&v);
        self.appmenu.set(d.appmenu);
        self.sw_min.set_now(d.minimize);
        self.sw_max.set_now(d.maximize);
        self.sw_close.set_now(d.close);
        self.loading.set(true);
        if d.side == "left" { self.b_left.set_active(true) } else { self.b_right.set_active(true) }
        self.loading.set(false);
        self.warn.set_visible(!d.close);
        self.preview.set_layout(d);
        self.tiles_refresh();
    }
}

// ---------- Obere Leiste ----------

/// Obere Leiste als Bild: Klick auf ein Element blendet es ein oder aus (Just Perfection).
pub struct BarPreview {
    pub area: gtk::DrawingArea,
    state: RefCell<Map<String, Value>>,
    clock_pos: Cell<i64>,
    rects: RefCell<Vec<(String, R)>>,
    pub on_click: RefCell<Option<Box<dyn Fn(&str)>>>,
}

impl BarPreview {
    fn new() -> Rc<BarPreview> {
        let area = gtk::DrawingArea::new();
        area.set_content_height(46);
        area.set_hexpand(true);
        cursor_hand(&area);
        let p = Rc::new(BarPreview { area, state: RefCell::new(Map::new()), clock_pos: Cell::new(0),
            rects: RefCell::new(vec![]), on_click: RefCell::new(None) });
        let w = Rc::downgrade(&p);
        p.area.set_draw_func(move |_, cr, wd, ht| {
            if let Some(p) = w.upgrade() {
                p.paint(cr, wd as f64, ht as f64);
            }
        });
        let click = gtk::GestureClick::new();
        let w = Rc::downgrade(&p);
        click.connect_released(move |_, _, x, y| {
            let Some(p) = w.upgrade() else { return };
            if !p.area.is_sensitive() {
                return;
            }
            let hit = p.rects.borrow().iter().find(|(_, r)| x >= r.x && x <= r.right() && y >= r.y && y <= r.bottom())
                .map(|(k, _)| k.clone());
            if let Some(k) = hit {
                if let Some(f) = p.on_click.borrow().as_ref() {
                    f(&k);
                }
            }
        });
        p.area.add_controller(click);
        p
    }

    fn set_state(&self, state: Map<String, Value>, clock_pos: i64) {
        *self.state.borrow_mut() = state;
        self.clock_pos.set(clock_pos);
        self.area.queue_draw();
    }

    fn paint(&self, cr: &cairo::Context, w: f64, h: f64) {
        let r = R::new(1.0, 1.0, w - 2.0, h - 2.0);
        draw::fill_rounded(cr, r, 10.0, Rgba::hex("#000000"));
        let mut zones: Vec<(&str, Vec<&str>)> = vec![("left", vec![]), ("center", vec![]), ("right", vec![])];
        for (key, _k, zone) in gnome::BAR_ITEMS {
            let mut z = zone;
            if key == "clock" {
                z = match self.clock_pos.get() { 1 => "right", 2 => "left", _ => "center" };
            }
            if let Some((_, v)) = zones.iter_mut().find(|(n, _)| *n == z) {
                v.push(key);
            }
        }
        let avail = r.w - 40.0;
        let mut font = Font::new(img::pt(9.0), pango::Weight::Normal);
        for pt in [9.0, 8.5, 8.0, 7.5, 7.0, 6.5, 6.0] {
            // kleiner, bis alles in die Leiste passt
            font = Font::new(img::pt(pt), pango::Weight::Normal);
            let need: f64 = zones.iter().flat_map(|(_, v)| v.iter())
                .map(|k| font.advance(cr, &t(&format!("bar.{k}"))) + 18.0 + 6.0).sum::<f64>() + 60.0;
            if need <= avail {
                break;
            }
        }
        let mut rects = vec![];
        let state = self.state.borrow();
        let mut chip_at = |key: &str, x: f64, rects: &mut Vec<(String, R)>| -> f64 {
            let text = t(&format!("bar.{key}"));
            let cw = font.advance(cr, &text) + 18.0;
            let rr = R::new(x, r.y + 8.0, cw, r.h - 16.0);
            let on = state.get(key).map(truthy).unwrap_or(true);
            draw::fill_rounded(cr, rr, 7.0, Rgba::hex(if on { "#2b3747" } else { "#151c26" }));
            draw::rounded(cr, R::new(rr.x + 0.5, rr.y + 0.5, rr.w - 1.0, rr.h - 1.0), 7.0);
            Rgba::hex(if on { "#3a4a5f" } else { "#253142" }).set(cr);
            cr.set_line_width(1.0);
            if !on {
                cr.set_dash(&[3.0, 2.0], 0.0);
            }
            let _ = cr.stroke();
            cr.set_dash(&[], 0.0);
            draw::text(cr, &font, rr, &text, Rgba::hex(if on { "#e6edf5" } else { "#4d5a6b" }), HAlign::Center, false);
            rects.push((key.to_string(), rr));
            cw
        };
        let mut x = r.x + 10.0;
        for key in &zones[0].1 {
            x += chip_at(key, x, &mut rects) + 6.0;
        }
        let rw: Vec<f64> = zones[2].1.iter().map(|k| font.advance(cr, &t(&format!("bar.{k}"))) + 18.0).collect();
        let mut rx = r.right() - 10.0 - rw.iter().sum::<f64>() - 6.0 * (rw.len() as f64 - 1.0).max(0.0);
        let cwv: Vec<f64> = zones[1].1.iter().map(|k| font.advance(cr, &t(&format!("bar.{k}"))) + 18.0).collect();
        let ctotal = cwv.iter().sum::<f64>() + 6.0 * (cwv.len() as f64 - 1.0).max(0.0);
        let mut cx = (r.cx() - ctotal / 2.0).min(rx - 12.0 - ctotal); // Mitte, aber nie unter die rechte Seite
        cx = cx.max(x + 6.0);
        for key in &zones[1].1 {
            cx += chip_at(key, cx, &mut rects) + 6.0;
        }
        for key in &zones[2].1 {
            rx += chip_at(key, rx, &mut rects) + 6.0;
        }
        drop(state);
        *self.rects.borrow_mut() = rects;
    }
}

/// Karteninhalt: Leisten-Vorschau, Uhr-Position, Leisten-Position, Schnelleinstellungen.
pub struct BarEditor {
    root: gtk::Box,
    chip: gtk::Label,
    get: gtk::Button,
    rm: gtk::Button,
    bar: Rc<BarPreview>,
    clock: Rc<Segmented>,
    r_clock: Rc<Row>,
    place: Rc<Segmented>,
    r_place: Rc<Row>,
    qs: Vec<(String, Rc<Switch>, Rc<Row>)>,
    loading: Rc<Cell<bool>>,
}

impl BarEditor {
    fn new() -> Rc<BarEditor> {
        let root = vbox(4);
        let head = hbox(8);
        let chip_l = chip("", "Chip");
        let get = missing_button();
        let rm = remove_button();
        head.append(&chip_l);
        head.append(&get);
        head.append(&rm);
        root.append(&head);
        let bar = BarPreview::new();
        root.append(&bar.area);
        let clock = seg(&["2", "0", "1"], "bar.pos.");
        let r_clock = Row::new("bar.clockpos", &clock.root);
        root.append(&r_clock.root);
        let place = seg(&["0", "1"], "bar.place.");
        let r_place = Row::new("bar.place", &place.root);
        root.append(&r_place.root);
        let mut qs = vec![];
        for (key, jk) in gnome::QUICK_ITEMS {
            let sw = Switch::new();
            let r = Row::new(key, &sw.area);
            root.append(&r.root);
            qs.push((jk.to_string(), sw, r));
        }
        let e = Rc::new(BarEditor { root, chip: chip_l, get, rm, bar, clock, r_clock, place, r_place, qs,
            loading: Rc::new(Cell::new(false)) });
        let w = Rc::downgrade(&e);
        *e.bar.on_click.borrow_mut() = Some(Box::new(move |key| {
            let Some(e) = w.upgrade() else { return };
            if let Some((_, jk, _)) = gnome::BAR_ITEMS.iter().find(|(n, _, _)| *n == key) {
                let cur = truthy(&gnome::jp_get(jk, json!(true)));
                e.set(jk, json!(!cur));
            }
        }));
        let w = Rc::downgrade(&e);
        e.clock.connect(move |v| {
            if let Some(e) = w.upgrade() {
                e.set("clock-menu-position", json!(v.parse::<i64>().unwrap_or(0)));
            }
        });
        let w = Rc::downgrade(&e);
        e.place.connect(move |v| {
            if let Some(e) = w.upgrade() {
                e.set("top-panel-position", json!(v.parse::<i64>().unwrap_or(0)));
            }
        });
        for (jk, sw, _) in &e.qs {
            let w = Rc::downgrade(&e);
            let k = jk.clone();
            sw.connect(move |on| {
                if let Some(e) = w.upgrade() {
                    e.set(&k, json!(on));
                }
            });
        }
        e
    }

    fn set(&self, key: &str, value: Value) {
        if self.loading.get() || !gnome::jp_ready() {
            return;
        }
        gnome::jp_set(key, value);
        win().set_status(&t("done"));
        self.refresh();
    }

    fn retranslate(&self) {
        self.r_clock.retranslate();
        self.r_place.retranslate();
        self.clock.retranslate();
        self.place.retranslate();
        for (_, _, r) in &self.qs {
            r.retranslate();
        }
        self.bar.area.set_tooltip_text(Some(&t("bar.tip")));
        self.bar.area.queue_draw();
    }

    fn refresh(&self) {
        let have = gnome::ext_installed(gnome::JP_UUID);
        let ready = gnome::jp_ready();
        set_chip(&self.chip, if ready { "ChipOk" } else { "ChipWarn" },
            &if ready { "Just Perfection".to_string() } else if have { t("gn.ext.off") } else { t("gn.ext.missing") });
        self.chip.set_tooltip_text(Some(&t("bar.needs")));
        self.get.set_visible(!have);
        self.rm.set_visible(have && gnome::ext_removable(gnome::JP_UUID));
        let mut state = Map::new();
        if ready {
            for (name, k, _) in gnome::BAR_ITEMS {
                state.insert(name.into(), json!(truthy(&gnome::jp_get(k, json!(true)))));
            }
        }
        let num = |k: &str| gnome::jp_get(k, json!(0)).as_i64().unwrap_or(0);
        self.loading.set(true);
        self.bar.set_state(state, if ready { num("clock-menu-position") } else { 0 });
        self.clock.select(&if ready { num("clock-menu-position").to_string() } else { "0".into() });
        self.place.select(&if ready { num("top-panel-position").to_string() } else { "0".into() });
        for (jk, sw, _) in &self.qs {
            sw.set_now(if ready { truthy(&gnome::jp_get(jk, json!(true))) } else { true });
            sw.area.set_sensitive(ready);
        }
        self.loading.set(false);
        self.bar.area.set_sensitive(ready);
        self.clock.root.set_sensitive(ready);
        self.place.root.set_sensitive(ready);
    }
}

fn jp_keys() -> Vec<String> {
    let mut v: Vec<String> = gnome::BAR_ITEMS.iter().map(|(_, k, _)| k.to_string()).collect();
    v.extend(gnome::QUICK_ITEMS.iter().map(|(_, k)| k.to_string()));
    v.push("clock-menu-position".into());
    v.push("top-panel-position".into());
    v
}

pub struct TopbarPage {
    page: Rc<Page>,
    sw_ws: Rc<Switch>,
    ws_chip: gtk::Label,
    ws_get: gtk::Button,
    ws_rm: gtk::Button,
    r_ws: Rc<Row>,
    editor: Rc<BarEditor>,
    power: Vec<(Rc<Switch>, Rc<Row>, &'static str, &'static str, bool)>,
    pending: Rc<RefCell<Option<String>>>,
}

impl TopbarPage {
    pub fn new() -> Rc<TopbarPage> {
        let page = Page::new("gn.topbar", true);
        let specs = gnome::section("topbar");
        let clock = page.card("gn.top.clock");
        page.gsection(&clock, &specs[..5]);
        let bar = page.card("gn.top.bar");
        page.gsection(&bar, &specs[5..7]);
        let sw_ws = Switch::new();
        let ws_chip = chip("", "Chip");
        let wb = hbox(8);
        let ws_get = missing_button();
        let ws_rm = remove_button();
        wb.append(&ws_chip);
        wb.append(&ws_get);
        wb.append(&ws_rm);
        wb.append(&sw_ws.area);
        let r_ws = Row::new("gn.top.wsind", &wb);
        bar.append(&r_ws.root);
        let ws = page.card("gn.top.ws");
        page.gsection(&ws, &specs[7..]);
        let ed = page.card("bar.editor");
        let editor = BarEditor::new();
        ed.append(&editor.root);
        let pw = page.card("pw.menu");
        let mut power = vec![];
        for (key, schema, gk, inv) in gnome::POWER_ITEMS {
            let sw = Switch::new();
            sw.connect(move |on| {
                usercmd::gset(schema, gk, json!(if inv { !on } else { on }));
                win().set_status(&t("done"));
            });
            let r = Row::new(key, &sw.area);
            pw.append(&r.root);
            power.push((sw, r, schema, gk, inv));
        }
        let mut keys = page.reset_keys();
        keys.extend(gnome::POWER_ITEMS.iter().map(|(_, s, k, _)| (s.to_string(), k.to_string())));
        *page.keys_override.borrow_mut() = Some(keys);
        page.finish();
        let p = Rc::new(TopbarPage { page, sw_ws, ws_chip, ws_get, ws_rm, r_ws, editor, power,
            pending: Rc::new(RefCell::new(None)) });
        *p.page.reset_extra.borrow_mut() = Some(Rc::new(|bak: &Path| {
            if !gnome::jp_ready() {
                return;
            }
            let mut vals = Map::new();
            for k in jp_keys() {
                vals.insert(k.clone(), gnome::jp_get(&k, Value::Null));
            }
            let _ = std::fs::write(bak.join("just-perfection.json"), serde_json::to_string(&Value::Object(vals)).unwrap_or_default());
            for k in jp_keys() {
                let mut c = gnome::jp_cmd();
                c.extend(["reset".to_string(), gnome::JP_SCHEMA.into(), k]);
                usercmd::run_v(&c);
            }
        }));
        *p.page.undo_extra.borrow_mut() = Some(Rc::new(|bak: &Path| {
            let Ok(text) = std::fs::read_to_string(bak.join("just-perfection.json")) else { return };
            let Ok(Value::Object(vals)) = serde_json::from_str::<Value>(&text) else { return };
            for (k, v) in vals {
                if !v.is_null() {
                    gnome::jp_set(&k, v);
                }
            }
        }));
        let wp = Rc::downgrade(&p);
        *p.page.on_refresh.borrow_mut() = Some(Rc::new(move || {
            if let Some(p) = wp.upgrade() {
                p.refresh();
            }
        }));
        p.sw_ws.connect(|on| {
            gnome::ext_set(gnome::WS_INDICATOR, on);
            win().set_status(&t("done"));
        });
        let wp = Rc::downgrade(&p);
        p.ws_get.connect_clicked(move |_| {
            if let Some(p) = wp.upgrade() {
                install_ext(gnome::WS_INDICATOR, p.clone(), p.pending.clone());
            }
        });
        let wp = Rc::downgrade(&p);
        p.ws_rm.connect_clicked(move |_| {
            if let Some(p) = wp.upgrade() {
                uninstall_ext(gnome::WS_INDICATOR, p.clone());
            }
        });
        let wp = Rc::downgrade(&p);
        p.editor.get.connect_clicked(move |_| {
            if let Some(p) = wp.upgrade() {
                install_ext(gnome::JP_UUID, p.clone(), p.pending.clone());
            }
        });
        let wp = Rc::downgrade(&p);
        p.editor.rm.connect_clicked(move |_| {
            if let Some(p) = wp.upgrade() {
                uninstall_ext(gnome::JP_UUID, p.clone());
            }
        });
        p
    }
}

impl AioPage for TopbarPage {
    fn widget(&self) -> gtk::Widget {
        self.page.root.clone().upcast()
    }

    fn retranslate(&self) {
        self.page.retranslate();
        self.r_ws.retranslate();
        self.editor.retranslate();
        for (_, r, ..) in &self.power {
            r.retranslate();
        }
    }

    fn refresh(&self) {
        self.page.load_controls();
        let have = gnome::ext_installed(gnome::WS_INDICATOR);
        let on = gnome::ext_enabled().iter().any(|x| x == gnome::WS_INDICATOR);
        self.sw_ws.area.set_sensitive(have);
        self.sw_ws.set_now(on);
        set_chip(&self.ws_chip, if have { "Chip" } else { "ChipWarn" }, &if have { t("gn.ext.name") } else { t("gn.ext.missing") });
        self.ws_chip.set_tooltip_text(Some(&t("gn.top.wsind.tip")));
        self.ws_get.set_visible(!have);
        self.ws_rm.set_visible(have && gnome::ext_removable(gnome::WS_INDICATOR));
        self.editor.refresh();
        for (sw, _, schema, gk, inv) in &self.power {
            let v = truthy(&usercmd::gget(schema, gk, json!(false)));
            sw.set_now(if *inv { !v } else { v });
        }
        self.ws_get.set_tooltip_text(Some(&format!("{}: Workspace Indicator", t("miss.install"))));
    }

    fn finished(&self, ok: bool, action: &str, _r: &str) {
        ext_finished(ok, action, &self.pending);
        self.refresh();
    }
}

// ---------- Startprogramme ----------

/// Programm auswählen (Suchfeld + Liste).
fn app_picker(done: impl FnOnce(Option<PathBuf>) + 'static) {
    let f = hyprstudio::ui::dialogs::Frame::new(win().parent().as_ref(), &t("gn.auto.add"), 460);
    f.win.set_default_size(460, 560);
    let search = search_entry(-1);
    search.set_placeholder_text(Some(&t("search")));
    f.body.append(&search);
    let list = gtk::ListBox::new();
    list.add_css_class("NavList");
    let sc = gtk::ScrolledWindow::new();
    sc.set_child(Some(&list));
    sc.set_vexpand(true);
    sc.set_size_request(-1, 400);
    f.body.append(&sc);
    let apps = gnome::list_apps();
    let files: Rc<Vec<PathBuf>> = Rc::new(apps.iter().map(|a| a.file.clone()).collect());
    let names: Rc<Vec<String>> = Rc::new(apps.iter().map(|a| a.name.clone()).collect());
    for a in &apps {
        let row = hbox(10);
        hyprstudio::ui::widgets::margins(&row, 4, 4, 4, 4);
        let ic = gtk::Image::from_icon_name(if a.icon.is_empty() { "application-x-executable" } else { &a.icon });
        ic.set_pixel_size(24);
        row.append(&ic);
        row.append(&label(&a.name, ""));
        list.append(&row);
    }
    let n2 = names.clone();
    let l2 = list.clone();
    search.connect_changed(move |e| {
        let q = e.text().trim().to_lowercase();
        let mut i = 0;
        while let Some(r) = l2.row_at_index(i) {
            r.set_visible(q.is_empty() || n2.get(i as usize).map(|n| n.to_lowercase().contains(&q)).unwrap_or(false));
            i += 1;
        }
    });
    let ok = f.buttons(&t("gn.auto.add"), "Primary", true);
    let _ = ok;
    let fw = Rc::downgrade(&f);
    list.connect_row_activated(move |_, _| {
        if let Some(f) = fw.upgrade() {
            f.finish(true);
        }
    });
    let l3 = list.clone();
    f.run(move |yes| {
        let chosen = if yes { l3.selected_row().and_then(|r| files.get(r.index() as usize).cloned()) } else { None };
        done(chosen);
    });
}

pub struct AutostartPage {
    page: Rc<Page>,
    card: Rc<super::form::Card>,
    empty: gtk::Label,
    rows: RefCell<Vec<gtk::Box>>,
    switches: RefCell<Vec<Rc<Switch>>>,
}

impl AutostartPage {
    pub fn new() -> Rc<AutostartPage> {
        let page = Page::new("gn.autostart", true);
        let b_add = tbtn("gn.auto.add", true);
        page.tools.append(&b_add);
        let card = page.card("");
        let empty = label("", "Label");
        empty.set_xalign(0.0);
        card.append(&empty);
        page.finish();
        let p = Rc::new(AutostartPage { page, card, empty, rows: RefCell::new(vec![]), switches: RefCell::new(vec![]) });
        let wp = Rc::downgrade(&p);
        b_add.connect_clicked(move |_| {
            let wp2 = wp.clone();
            app_picker(move |f| {
                let (Some(p), Some(f)) = (wp2.upgrade(), f) else { return };
                let w = win();
                w.log("");
                gnome::autostart_add(&f);
                p.refresh();
                w.set_status(&t("done"));
            });
        });
        let wp = Rc::downgrade(&p);
        *p.page.on_refresh.borrow_mut() = Some(Rc::new(move || {
            if let Some(p) = wp.upgrade() {
                p.refresh();
            }
        }));
        p
    }
}

impl AioPage for AutostartPage {
    fn widget(&self) -> gtk::Widget {
        self.page.root.clone().upcast()
    }

    fn retranslate(&self) {
        self.page.retranslate();
        self.empty.set_text(&t("gn.auto.none"));
    }

    fn refresh(&self) {
        for r in self.rows.borrow_mut().drain(..) {
            self.card.root.remove(&r);
        }
        self.switches.borrow_mut().clear();
        let entries = gnome::list_autostart();
        self.empty.set_visible(entries.is_empty());
        for e in entries {
            let row = hbox(8);
            row.set_margin_top(6);
            row.set_margin_bottom(6);
            let ic = gtk::Image::from_icon_name(if e.icon.is_empty() { "application-x-executable" } else { &e.icon });
            ic.set_pixel_size(24);
            let name = label(&e.name, "");
            name.set_xalign(0.0);
            name.set_hexpand(true);
            name.set_tooltip_text(Some(&e.file.display().to_string()));
            let sw = Switch::new();
            sw.set_now(e.enabled);
            let path = e.file.clone();
            sw.connect(move |on| {
                gnome::autostart_enable(&path, on);
                win().set_status(&t("done"));
            });
            row.append(&ic);
            row.append(&name);
            row.append(&sw.area);
            self.card.append(&row);
            self.rows.borrow_mut().push(row);
            self.switches.borrow_mut().push(sw);
        }
    }
}

// ---------- Anmeldebildschirm (GDM) ----------

/// Bildfläche mit Rahmen: Bild eingepasst, Farbe vollflächig oder „–“.
pub struct ImageBox {
    pub area: gtk::DrawingArea,
    surf: RefCell<Option<Rc<cairo::ImageSurface>>>,
    color: RefCell<Option<String>>,
}

impl ImageBox {
    pub fn new(w: i32, h: i32) -> Rc<ImageBox> {
        let area = gtk::DrawingArea::new();
        area.set_content_width(w);
        area.set_content_height(h);
        area.set_hexpand(false);
        area.set_halign(gtk::Align::Start);
        let b = Rc::new(ImageBox { area, surf: RefCell::new(None), color: RefCell::new(None) });
        let wb = Rc::downgrade(&b);
        b.area.set_draw_func(move |_, cr, wd, ht| {
            let Some(b) = wb.upgrade() else { return };
            let r = R::new(0.5, 0.5, wd as f64 - 1.0, ht as f64 - 1.0);
            let col = b.color.borrow().clone();
            draw::fill_rounded(cr, r, 10.0, Rgba::hex(col.as_deref().unwrap_or(theme::SURFACE2)));
            draw::rounded(cr, r, 10.0);
            Rgba::hex(theme::BORDER).set(cr);
            cr.set_line_width(1.0);
            let _ = cr.stroke();
            if col.is_some() {
                return;
            }
            let s = b.surf.borrow().clone();
            match s {
                Some(s) => {
                    let (iw, ih) = (s.width() as f64, s.height() as f64);
                    let f = ((r.w - 8.0) / iw).min((r.h - 8.0) / ih);
                    let (w, h) = (iw * f, ih * f);
                    img::draw_into(cr, &s, R::new(r.x + (r.w - w) / 2.0, r.y + (r.h - h) / 2.0, w, h));
                }
                None => {
                    let f = Font::new(img::pt(10.0), pango::Weight::Normal);
                    draw::text(cr, &f, r, "–", Rgba::hex(theme::MUTED), HAlign::Center, false);
                }
            }
        });
        b
    }

    pub fn show_file(&self, path: Option<&Path>, color: Option<&str>) {
        *self.color.borrow_mut() = color.map(String::from);
        *self.surf.borrow_mut() = path.and_then(|p| img::load(p, 1600)).and_then(|pb| img::surface(&pb)).map(Rc::new);
        self.area.queue_draw();
    }
}

/// Bild für den Root-Helfer bereitlegen: als PNG nach ~/.cache (root darf nicht in die Cloud lesen).
pub fn stage_image(src: &Path, name: &str) -> Option<PathBuf> {
    let pb = gtk::gdk_pixbuf::Pixbuf::from_file(src).ok()?;
    let pb = img::to_width(pb, 3840);
    let d = usercmd::cache().join("gdm");
    std::fs::create_dir_all(&d).ok()?;
    let out = d.join(format!("{name}.png"));
    pb.savev(&out, "png", &[]).ok()?;
    Some(out)
}

pub struct LoginPage {
    page: Rc<Page>,
    state_chip: gtk::Label,
    bg_box: Rc<ImageBox>,
    logo_box: Rc<ImageBox>,
    b_bg: gtk::Button,
    b_color: gtk::Button,
    b_restore: gtk::Button,
    b_logo: gtk::Button,
    b_logo_clear: gtk::Button,
    banner: gtk::Entry,
    b_banner: gtk::Button,
    r_banner: Rc<Row>,
    sw_users: Rc<Switch>,
    r_users: Rc<Row>,
    loading: Rc<Cell<bool>>,
    me: RefCell<Weak<LoginPage>>,
}

impl LoginPage {
    pub fn new() -> Rc<LoginPage> {
        let page = Page::new("gn.login", true);
        let state_chip = chip("", "Chip");
        page.chips.append(&state_chip);
        // Hintergrund
        let bg = page.card("gn.login.bg");
        let row = hbox(16);
        let bg_box = ImageBox::new(256, 144);
        row.append(&bg_box.area);
        let col = vbox(6);
        let b_bg = tbtn("gn.login.pick", true);
        let b_color = tbtn("gn.login.color", false);
        let b_restore = tbtn("gn.login.restore", false);
        col.append(&b_bg);
        col.append(&b_color);
        col.append(&b_restore);
        row.append(&col);
        bg.append(&row);
        // Logo
        let logo = page.card("gn.login.logo");
        let lrow = hbox(16);
        let logo_box = ImageBox::new(144, 96);
        lrow.append(&logo_box.area);
        let lcol = vbox(6);
        let b_logo = tbtn("gn.login.pick", true);
        let b_logo_clear = tbtn("remove", false);
        lcol.append(&b_logo);
        lcol.append(&b_logo_clear);
        lrow.append(&lcol);
        logo.append(&lrow);
        // Text + Benutzerliste
        let more = page.card("gn.login.more");
        let bw = hbox(6);
        let banner = gtk::Entry::new();
        banner.add_css_class("Search");
        banner.set_size_request(280, -1);
        let b_banner = tbtn("apply", false);
        bw.append(&banner);
        bw.append(&b_banner);
        let r_banner = Row::new("gn.login.banner", &bw);
        more.append(&r_banner.root);
        let sw_users = Switch::new();
        let r_users = Row::new("gn.login.users", &sw_users.area);
        more.append(&r_users.root);
        page.has_reset.set(true);
        page.finish();
        let p = Rc::new(LoginPage { page, state_chip, bg_box, logo_box, b_bg, b_color, b_restore, b_logo, b_logo_clear,
            banner, b_banner, r_banner, sw_users, r_users, loading: Rc::new(Cell::new(false)), me: RefCell::new(Weak::new()) });
        *p.me.borrow_mut() = Rc::downgrade(&p);
        let wp = Rc::downgrade(&p);
        *p.page.reset_extra.borrow_mut() = Some(Rc::new(move |_| {
            // Systemstandard; Sicherung bleibt im Helfer
            if let Some(p) = wp.upgrade() {
                p.run(&["gdm-restore"], "gdm-restore");
            }
        }));
        let wp = Rc::downgrade(&p);
        *p.page.on_refresh.borrow_mut() = Some(Rc::new(move || {
            if let Some(p) = wp.upgrade() {
                p.refresh();
            }
        }));
        let wp = Rc::downgrade(&p);
        p.b_restore.connect_clicked(move |_| {
            if let Some(p) = wp.upgrade() {
                p.run(&["gdm-restore"], "gdm-restore");
            }
        });
        let wp = Rc::downgrade(&p);
        p.b_logo_clear.connect_clicked(move |_| {
            if let Some(p) = wp.upgrade() {
                p.run(&["gdm-logo", "--clear"], "gdm-logo");
            }
        });
        let wp = Rc::downgrade(&p);
        p.b_bg.connect_clicked(move |_| {
            if let Some(p) = wp.upgrade() {
                p.pick("background", "gdm-bg");
            }
        });
        let wp = Rc::downgrade(&p);
        p.b_logo.connect_clicked(move |_| {
            if let Some(p) = wp.upgrade() {
                p.pick("logo", "gdm-logo");
            }
        });
        let wp = Rc::downgrade(&p);
        p.b_color.connect_clicked(move |_| {
            let wp2 = wp.clone();
            hyprstudio::ui::dialogs::pick_color(win().parent().as_ref(), "#1b2430", &t("gn.login.color"), Box::new(|_| {}),
                Box::new(move |c| {
                    if let (Some(p), Some(c)) = (wp2.upgrade(), c) {
                        let c = if c.len() == 9 { c[..7].to_string() } else { c };
                        p.run(&["gdm-bg", &c], "gdm-bg");
                    }
                }));
        });
        let wp = Rc::downgrade(&p);
        p.b_banner.connect_clicked(move |_| {
            if let Some(p) = wp.upgrade() {
                let text = p.banner.text().trim().to_string();
                let arg = if text.is_empty() { "--clear".to_string() } else { text };
                p.run(&["gdm-banner", &arg], "gdm-banner");
            }
        });
        let wp = Rc::downgrade(&p);
        p.sw_users.connect(move |on| {
            if let Some(p) = wp.upgrade() {
                if !p.loading.get() {
                    p.run(&["gdm-userlist", if on { "on" } else { "off" }], "gdm-userlist");
                }
            }
        });
        p
    }

    fn run(&self, args: &[&str], action: &str) {
        let me = self.me.borrow().upgrade().map(|p| p as Rc<dyn AioPage>);
        win().run(args.iter().map(|s| s.to_string()).collect(), action, me);
    }

    fn pick(&self, name: &'static str, action: &'static str) {
        let wp = self.me.borrow().clone();
        let home = usercmd::real_home().display().to_string();
        hyprstudio::ui::dialogs::open_file(win().parent().as_ref(), &t("gn.login.pick"), &home,
            &[("Bilder (*.png *.jpg *.jpeg *.webp *.svg)", &["*.png", "*.jpg", "*.jpeg", "*.webp", "*.svg"])], move |f| {
                let (Some(p), Some(f)) = (wp.upgrade(), f) else { return };
                if let Some(out) = stage_image(&f, name) {
                    p.run(&[action, &out.display().to_string()], action);
                }
            });
    }
}

impl AioPage for LoginPage {
    fn widget(&self) -> gtk::Widget {
        self.page.root.clone().upcast()
    }

    fn retranslate(&self) {
        self.page.retranslate();
        self.r_banner.retranslate();
        self.r_users.retranslate();
        self.banner.set_placeholder_text(Some(&t("gn.login.banner.ph")));
    }

    fn refresh(&self) {
        let have = gnome::gdm_installed();
        let st = gnome::gdm_state();
        set_chip(&self.state_chip, if have { "ChipOk" } else { "ChipWarn" }, &if have { "GDM".to_string() } else { t("gn.login.nogdm") });
        let sv = |k: &str| st.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let bgv = sv("background");
        let gdm = usercmd::cache().join("gdm");
        if bgv.starts_with('#') {
            self.bg_box.show_file(None, Some(&bgv));
        } else {
            let f = gdm.join("background.png");
            self.bg_box.show_file(if bgv.is_empty() { None } else { Some(f.as_path()) }, None);
        }
        let lf = gdm.join("logo.png");
        let has_logo = st.get("logo").map(truthy).unwrap_or(false);
        self.logo_box.show_file(if has_logo { Some(lf.as_path()) } else { None }, None);
        self.banner.set_text(&sv("banner"));
        self.loading.set(true);
        self.sw_users.set_now(st.get("userlist").map(truthy).unwrap_or(true));
        self.loading.set(false);
        let busy = win().busy();
        for b in [&self.b_bg, &self.b_color, &self.b_restore, &self.b_logo, &self.b_logo_clear, &self.b_banner] {
            b.set_sensitive(have && !busy);
        }
        self.sw_users.area.set_sensitive(have && !busy);
        self.b_restore.set_tooltip_text(Some(&t("gn.login.restore.tip")));
        self.b_bg.set_tooltip_text(Some(&t("gn.login.bg.tip")));
    }

    fn finished(&self, _ok: bool, _a: &str, _r: &str) {
        self.refresh();
    }
}
