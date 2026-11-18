//! GNOME › Schriften: Schrift-Einstellungen wie GNOME Tweaks + Schriften installieren (Google Fonts, Nerd Fonts)
//! mit Live-Vorschau in der echten Schrift; danach direkt als Oberflächen-/Dokument-/Monospace-Schrift anwendbar.

use super::form::{flow_grid, tbtn, Page};
use super::widgets::{search_entry, seg, set_applied};
use super::win::{win, AioPage};
use crate::fonts::{self, FontItem};
use crate::gnome;
use crate::i18n::{lang, t, tf};
use crate::usercmd;
use gtk::glib;
use gtk::prelude::*;
use hyprstudio::kw;
use hyprstudio::ui::draw;
use hyprstudio::ui::shell;
use hyprstudio::ui::theme;
use hyprstudio::ui::widgets::{cursor_hand, hbox, label, vbox, Segmented};
use serde_json::Value;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub const TILE_W: i32 = 262;
pub const TILE_H: i32 = 200;
const PAGE: usize = 24;
const FILTERS: [&str; 7] = ["all", "Sans Serif", "Serif", "Monospace", "Display", "Handwriting", "Nerd Font"];

fn sample() -> &'static str {
    if lang() == "de" { "Franz jagt im komplett verwahrlosten Taxi" } else { "The quick brown fox jumps over the lazy dog" }
}

// ---------- Schriftdatei für die Vorschau laden (fontconfig) ----------

mod ffi {
    use std::os::raw::{c_char, c_int, c_void};
    #[link(name = "fontconfig")]
    extern "C" {
        pub fn FcConfigAppFontAddFile(config: *mut c_void, file: *const c_char) -> c_int;
    }
    #[link(name = "pangoft2-1.0")]
    extern "C" {
        pub fn pango_fc_font_map_config_changed(fcfontmap: *mut c_void);
    }
}

/// Datei für diese Sitzung bekannt machen; liefert den Familiennamen aus der Datei.
fn add_app_font(path: &Path, family: Option<String>) -> Option<String> {
    use gtk::glib::translate::ToGlibPtr;
    let c = std::ffi::CString::new(path.to_string_lossy().as_bytes()).ok()?;
    let ok = unsafe { ffi::FcConfigAppFontAddFile(std::ptr::null_mut(), c.as_ptr()) };
    if ok == 0 {
        return None;
    }
    let fm = pangocairo::FontMap::default();
    if let Some(t) = glib::Type::from_name("PangoFcFontMap") {
        if fm.type_().is_a(t) {
            let p: *mut gtk::pango::ffi::PangoFontMap = fm.to_glib_none().0;
            unsafe { ffi::pango_fc_font_map_config_changed(p as *mut _) };
        }
    }
    family
}

/// Familienname aus der Datei (fc-query), im Hintergrund.
fn family_of(path: &Path) -> Option<String> {
    let p = path.to_string_lossy().into_owned();
    usercmd::query(&["fc-query", "-f", "%{family[0]}", &p]).map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

pub struct FontTile {
    root: gtk::Box,
    item: FontItem,
    sample: gtk::Label,
    b_inst: gtk::Button,
    b_use: gtk::Button,
}

impl FontTile {
    fn new(item: FontItem, page: &Rc<FontsPage>) -> Rc<FontTile> {
        let root = vbox(6);
        root.add_css_class("ATile");
        root.set_size_request(TILE_W, TILE_H);
        root.add_css_class("PadFont");
        let sample = label("Aa", "Sample");
        sample.set_wrap(true);
        sample.set_xalign(0.0);
        sample.set_size_request(-1, 66);
        sample.set_valign(gtk::Align::Fill);
        sample.set_max_width_chars(1);
        sample.set_ellipsize(gtk::pango::EllipsizeMode::End);
        sample.set_lines(2);
        root.append(&sample);
        let name = label(&item.family, "name");
        name.set_xalign(0.0);
        name.set_ellipsize(gtk::pango::EllipsizeMode::End);
        name.set_max_width_chars(1);
        name.set_tooltip_text(Some(&item.family));
        root.append(&name);
        let cat = label(&t(&format!("font.cat.{}", item.category)), "Label");
        cat.set_xalign(0.0);
        root.append(&cat);
        let row = hbox(6);
        let b_inst = gtk::Button::with_label("");
        b_inst.add_css_class("Small");
        b_inst.set_hexpand(true);
        let b_use = gtk::Button::with_label(&t("font.use"));
        b_use.add_css_class("UseBtn");
        for b in [&b_inst, &b_use] {
            cursor_hand(b);
            b.set_size_request(-1, 30);
            row.append(b);
        }
        root.append(&row);
        let tile = Rc::new(FontTile { root, item, sample, b_inst, b_use });
        let (wp, wt) = (Rc::downgrade(page), Rc::downgrade(&tile));
        tile.b_inst.connect_clicked(move |_| {
            if let (Some(p), Some(t)) = (wp.upgrade(), wt.upgrade()) {
                p.install_font(&t.item);
            }
        });
        let (wp, wt) = (Rc::downgrade(page), Rc::downgrade(&tile));
        tile.b_use.connect_clicked(move |b| {
            let (Some(p), Some(tl)) = (wp.upgrade(), wt.upgrade()) else { return };
            let mut items: Vec<(String, bool)> = ["ui", "doc", "mono", "title"].iter()
                .map(|k| (t(&format!("font.{k}")), false)).collect();
            items.push((t("remove"), true));
            let inst = fonts::installed(&tl.item.family).is_some();
            let item = tl.item.clone();
            let p2 = p.clone();
            hyprstudio::ui::dialogs::menu(b, 0.0, b.height() as f64, items, move |i| {
                match i {
                    0..=3 => p2.use_font(&item, ["ui", "doc", "mono", "title"][i]),
                    _ if inst => p2.remove_font(&item),
                    _ => {}
                }
            });
        });
        tile
    }

    fn set_font_family(&self, family: Option<&str>, text: &str) {
        match family {
            Some(f) => {
                let attrs = gtk::pango::AttrList::new();
                let mut fd = gtk::pango::FontDescription::new();
                fd.set_family(&f.replace('"', ""));
                fd.set_size(15 * gtk::pango::SCALE);
                attrs.insert(gtk::pango::AttrFontDesc::new(&fd));
                self.sample.set_attributes(Some(&attrs));
                self.sample.remove_css_class("muted");
                self.sample.set_text(text);
            }
            None => {
                self.sample.set_attributes(None);
                self.sample.add_css_class("muted");
                self.sample.set_text(&t("font.nopreview"));
            }
        }
    }

    fn refresh(&self, busy: bool) {
        let inst = fonts::installed(&self.item.family).is_some();
        set_applied(&self.b_inst, inst, &t("installed"));
        if !inst {
            self.b_inst.set_label(&t("install"));
            self.b_inst.set_sensitive(!busy);
        }
        self.b_use.set_sensitive(inst && !busy);
    }
}

pub struct FontsPage {
    page: Rc<Page>,
    search: gtk::Entry,
    only_inst: gtk::CheckButton,
    filter: Rc<Segmented>,
    cur_filter: RefCell<String>,
    msg: gtk::Label,
    grid: gtk::FlowBox,
    b_more: gtk::Button,
    timer: RefCell<Option<glib::SourceId>>,
    items: RefCell<Vec<FontItem>>,
    shown: Cell<usize>,
    tiles: RefCell<Vec<Rc<FontTile>>>,
    /// Familie → geladener Name für die Vorschau
    families: RefCell<HashMap<String, Option<String>>>,
    busy: RefCell<String>,
    loaded: Cell<bool>,
}

impl FontsPage {
    pub fn new() -> Rc<FontsPage> {
        let page = Page::new("gn.fonts", true);
        let card = page.card("");
        page.gsection(&card, gnome::section("fonts"));
        // Schriften installieren
        let store = page.section("font.store");
        let bar = hbox(8);
        let search = search_entry(-1);
        search.set_hexpand(true);
        let only_inst = gtk::CheckButton::new();
        bar.append(&search);
        bar.append(&only_inst);
        store.append(&bar);
        let filter = seg(&FILTERS, "font.f.");
        filter.select("all");
        let fs = gtk::ScrolledWindow::new();
        fs.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Never);
        fs.set_child(Some(&filter.root));
        filter.root.set_halign(gtk::Align::Start);
        store.append(&fs);
        let msg = label("", "Label");
        msg.set_wrap(true);
        msg.set_xalign(0.0);
        store.append(&msg);
        let grid = flow_grid();
        store.append(&grid);
        let b_more = tbtn("font.more", false);
        b_more.set_halign(gtk::Align::Center);
        store.append(&b_more);
        page.finish();
        let p = Rc::new(FontsPage { page, search, only_inst, filter, cur_filter: RefCell::new("all".into()), msg, grid,
            b_more, timer: RefCell::new(None), items: RefCell::new(vec![]), shown: Cell::new(0),
            tiles: RefCell::new(vec![]), families: RefCell::new(HashMap::new()), busy: RefCell::new(String::new()),
            loaded: Cell::new(false) });
        let w = Rc::downgrade(&p);
        p.search.connect_changed(move |_| {
            let Some(p) = w.upgrade() else { return };
            if let Some(id) = p.timer.borrow_mut().take() {
                id.remove();
            }
            let w2 = Rc::downgrade(&p);
            let id = glib::timeout_add_local_once(std::time::Duration::from_millis(300), move || {
                if let Some(p) = w2.upgrade() {
                    p.timer.borrow_mut().take();
                    p.rebuild();
                }
            });
            *p.timer.borrow_mut() = Some(id);
        });
        let w = Rc::downgrade(&p);
        p.only_inst.connect_toggled(move |_| {
            if let Some(p) = w.upgrade() {
                p.rebuild();
            }
        });
        let w = Rc::downgrade(&p);
        p.filter.connect(move |k| {
            if let Some(p) = w.upgrade() {
                *p.cur_filter.borrow_mut() = k;
                p.rebuild();
            }
        });
        let w = Rc::downgrade(&p);
        p.b_more.connect_clicked(move |_| {
            if let Some(p) = w.upgrade() {
                p.more();
            }
        });
        let w = Rc::downgrade(&p);
        p.page.root.connect_map(move |_| {
            let Some(p) = w.upgrade() else { return };
            if p.loaded.get() {
                return;
            }
            p.loaded.set(true);
            p.msg.set_text(&t("font.loading"));
            p.msg.set_visible(true);
            let w2 = Rc::downgrade(&p);
            shell::run_bg(fonts::catalog, move |(items, err)| {
                let Some(p) = w2.upgrade() else { return };
                *p.items.borrow_mut() = items;
                p.msg.set_visible(!err.is_empty());
                if !err.is_empty() {
                    p.msg.set_text(&t(&err));
                }
                p.rebuild();
            });
        });
        let w = Rc::downgrade(&p);
        *p.page.on_refresh.borrow_mut() = Some(Rc::new(move || {
            if let Some(p) = w.upgrade() {
                p.page.load_controls();
                p.refresh_tiles();
            }
        }));
        p
    }

    fn visible(&self) -> Vec<FontItem> {
        let q = self.search.text().trim().to_lowercase();
        let inst = fonts::list_installed();
        let f = self.cur_filter.borrow().clone();
        let only = self.only_inst.is_active();
        self.items.borrow().iter().filter(|it| {
            (f == "all" || it.category == f) && (q.is_empty() || it.family.to_lowercase().contains(&q))
                && (!only || inst.contains(&fonts::safe_family(&it.family)))
        }).cloned().collect()
    }

    fn rebuild(self: &Rc<Self>) {
        while let Some(c) = self.grid.first_child() {
            self.grid.remove(&c);
        }
        self.tiles.borrow_mut().clear();
        self.shown.set(0);
        self.more();
    }

    fn more(self: &Rc<Self>) {
        let vis = self.visible();
        let start = self.shown.get().min(vis.len());
        let new: Vec<FontItem> = vis[start..(start + PAGE).min(vis.len())].to_vec();
        self.shown.set(start + new.len());
        for it in &new {
            let tile = FontTile::new(it.clone(), self);
            self.grid.insert(&tile.root, -1);
            if let Some(f) = self.families.borrow().get(&it.family) {
                tile.set_font_family(f.as_deref(), sample());
            }
            self.tiles.borrow_mut().push(tile);
        }
        self.b_more.set_visible(self.shown.get() < vis.len());
        let has = !self.items.borrow().is_empty();
        if has && vis.is_empty() {
            self.msg.set_text(&t("font.none"));
            self.msg.set_visible(true);
        } else if has {
            self.msg.set_visible(false);
        }
        self.refresh_tiles();
        let todo: Vec<FontItem> = new.into_iter().filter(|it| !self.families.borrow().contains_key(&it.family)).collect();
        for part in todo.chunks(6) {
            let part = part.to_vec();
            let w = Rc::downgrade(self);
            shell::run_bg(move || {
                part.iter().map(|it| {
                    let path = fonts::preview_file(it);
                    let fam = path.as_ref().and_then(|p| family_of(p));
                    (it.family.clone(), path, fam)
                }).collect::<Vec<(String, Option<PathBuf>, Option<String>)>>()
            }, move |res| {
                if let Some(p) = w.upgrade() {
                    p.previews(res);
                }
            });
        }
    }

    fn previews(&self, res: Vec<(String, Option<PathBuf>, Option<String>)>) {
        for (family, path, fam) in res {
            let f = path.and_then(|p| add_app_font(&p, fam));
            self.families.borrow_mut().insert(family.clone(), f.clone());
            if let Some(tile) = self.tiles.borrow().iter().find(|t| t.item.family == family) {
                tile.set_font_family(f.as_deref(), sample());
            }
        }
    }

    fn refresh_tiles(&self) {
        let busy = !self.busy.borrow().is_empty() || win().busy();
        for tile in self.tiles.borrow().iter() {
            tile.refresh(busy);
        }
    }

    fn install_font(self: &Rc<Self>, it: &FontItem) {
        if !self.busy.borrow().is_empty() {
            return;
        }
        *self.busy.borrow_mut() = it.family.clone();
        let w = win();
        w.log("");
        w.task_begin(&tf("look.installing", kw!("name" => it.family.clone())));
        self.refresh_tiles();
        let item = it.clone();
        let wp = Rc::downgrade(self);
        shell::run_job(move |log| fonts::install(&item, log).map_err(|e| format!("{}: {}", e.key, e.detail)).map(|_| item),
            move |res| {
                let Some(p) = wp.upgrade() else { return };
                p.busy.borrow_mut().clear();
                let w = win();
                w.task_end(true, "");
                match res {
                    Ok(item) => {
                        w.set_status(&tf("look.installed", kw!("name" => item.family.clone())));
                        p.families.borrow_mut().remove(&item.family);
                        let wp2 = Rc::downgrade(&p);
                        shell::run_bg(move || {
                            let path = fonts::preview_file(&item);
                            let fam = path.as_ref().and_then(|x| family_of(x));
                            vec![(item.family.clone(), path, fam)]
                        }, move |res| {
                            if let Some(p) = wp2.upgrade() {
                                p.previews(res);
                            }
                        });
                    }
                    Err(e) => {
                        w.log(&e);
                        let key = e.split(':').next().unwrap_or("").to_string();
                        let shown = if key.starts_with("remote.") || key.starts_with("fonts.") { t(&key) } else { e };
                        w.set_status(&format!("{}: {}", t("failed"), shown));
                    }
                }
                p.refresh_tiles();
            });
    }

    fn remove_font(self: &Rc<Self>, it: &FontItem) {
        let fam = it.family.clone();
        let wp = Rc::downgrade(self);
        win().confirm(&t("remove"), &tf("confirm.remove", kw!("name" => fam.clone())), move |ok| {
            if !ok {
                return;
            }
            let w = win();
            w.log("");
            fonts::remove(&fam, &|s| win().log(s));
            w.set_status(&t("done"));
            if let Some(p) = wp.upgrade() {
                p.refresh_tiles();
            }
        });
    }

    fn use_font(&self, it: &FontItem, key: &str) {
        let (schema, gkey) = match key {
            "ui" => (gnome::IFACE, "font-name"),
            "doc" => (gnome::IFACE, "document-font-name"),
            "mono" => (gnome::IFACE, "monospace-font-name"),
            _ => (gnome::WM, "titlebar-font"),
        };
        let cur = usercmd::gget_str(schema, gkey);
        let size = if cur.is_empty() { 11.0 } else { gnome::split_font(&cur).1 };
        let family = self.families.borrow().get(&it.family).cloned().flatten().unwrap_or_else(|| it.family.clone());
        usercmd::gset(schema, gkey, Value::String(gnome::join_font(&family, size)));
        win().set_status(&tf("font.applied", kw!("name" => family, "where" => t(&format!("font.{key}")))));
        self.page.load_controls();
    }
}

impl AioPage for FontsPage {
    fn widget(&self) -> gtk::Widget {
        self.page.root.clone().upcast()
    }

    fn retranslate(&self) {
        self.page.retranslate();
        self.search.set_placeholder_text(Some(&t("font.search")));
        self.only_inst.set_label(Some(&t("font.onlyinst")));
        self.filter.retranslate();
    }

    fn refresh(&self) {
        self.page.load_controls();
        self.refresh_tiles();
    }
}

#[allow(dead_code)]
fn _unused() {
    let _ = (draw::Rgba::hex("#000"), theme::MUTED);
}
