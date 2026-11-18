//! Gemeinsame Theme-Seite (SDDM, Plymouth): installierte Themes + KDE Store, mit Installieren-Aufforderung.
//! Dazu die Leisten oben (Login-Manager bzw. Bootsplash-Einrichtung).

use super::img;
use super::widgets::{num, reset_button, search_entry, seg, set_applied, Anim, ChipRow, Picture, Tile, TileStrip};
use super::win::{win, AioPage};
use crate::i18n::{lang, t, tf};
use crate::sddm::{StoreItem, ThemeInfo};
use crate::{plymouth, remote, sddm, system, usercmd};
use gtk::cairo::ImageSurface;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::glib;
use gtk::prelude::*;
use hyprstudio::kw;
use hyprstudio::ui::shell;
use hyprstudio::ui::theme;
use hyprstudio::ui::widgets::{cursor_hand, hbox, label, vbox, vspacer, Segmented};
use serde_json::{Map, Value};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Sddm,
    Ply,
}

// ---------- Netz (mit Platten-Cache für Vorschaubilder) ----------

type ImgCb = Box<dyn Fn(Option<Rc<ImageSurface>>)>;

fn http_cache(url: &str) -> PathBuf {
    usercmd::cache().join("http").join(remote::sha256_hex(url.as_bytes()))
}

fn net_get(url: &str) -> Result<Vec<u8>, String> {
    remote::get(url, 30, "").map_err(|e| e.to_string())
}


pub struct ThemePage {
    kind: Kind,
    act: &'static str,
    style: &'static str,
    prompt_title: &'static str,
    prompt_button: &'static str,
    prompt_args: &'static [&'static str],
    has_test: bool,
    default_theme: &'static str,
    root: gtk::Box,
    // Leiste oben
    b_reset: gtk::Button,
    bar_label: gtk::Label,
    bar_chips: Rc<ChipRow>,
    bar_btn: gtk::Button,
    // Seiten
    body: gtk::Stack,
    prompt_pic: Rc<Picture>,
    prompt_label: gtk::Label,
    prompt_btn: gtk::Button,
    prompt_anchor: gtk::Box,
    big: Rc<Picture>,
    name: gtk::Label,
    chips: Rc<ChipRow>,
    info: gtk::Label,
    files: gtk::DropDown,
    files_model: gtk::StringList,
    files_data: RefCell<(String, Vec<u32>)>,
    btn1: gtk::Button,
    btn2: gtk::Button,
    btn3: gtk::Button,
    src: Rc<Segmented>,
    search_edit: gtk::Entry,
    search_timer: RefCell<Option<glib::SourceId>>,
    sort_seg: Rc<Segmented>,
    strip: Rc<TileStrip>,
    // Zustand
    mode: RefCell<String>,
    sort: RefCell<String>,
    search: RefCell<String>,
    local: RefCell<Vec<ThemeInfo>>,
    store: RefCell<Vec<StoreItem>>,
    page: Cell<u32>,
    total: Cell<i64>,
    loading: Cell<bool>,
    gen: Cell<u32>,
    sel_local: RefCell<Option<String>>,
    sel_store: RefCell<Option<String>>,
    local_pix: RefCell<HashMap<String, Option<Rc<ImageSurface>>>>,
    current: RefCell<String>,
    mine: RefCell<Map<String, Value>>,
    mine_store: RefCell<HashSet<String>>,
    tiles: RefCell<Vec<(String, Rc<Tile>, Rc<Picture>)>>,
    anim_cache: RefCell<HashMap<(String, bool), Option<Anim>>>,
    net: Rc<NetImg>,
}

/// Netz-Bilder: Daten im Hintergrund holen, im Hauptthread dekodieren.
#[derive(Default)]
pub struct NetImg {
    pix: RefCell<HashMap<String, Option<Rc<ImageSurface>>>>,
    raw: RefCell<HashMap<String, Vec<u8>>>,
    waiting: RefCell<HashMap<String, Vec<ImgCb>>>,
}

impl NetImg {
    fn image(self: &Rc<Self>, url: &str, cb: impl Fn(Option<Rc<ImageSurface>>) + 'static) {
        if url.is_empty() {
            return;
        }
        if let Some(p) = self.pix.borrow().get(url) {
            cb(p.clone());
            return;
        }
        if let Some(w) = self.waiting.borrow_mut().get_mut(url) {
            w.push(Box::new(cb));
            return;
        }
        self.waiting.borrow_mut().insert(url.to_string(), vec![Box::new(cb)]);
        let u = url.to_string();
        let me = self.clone();
        shell::run_bg(move || {
            let cp = http_cache(&u);
            if let Ok(d) = std::fs::read(&cp) {
                if !d.is_empty() {
                    return (u, Some(d));
                }
            }
            let d = net_get(&u).ok();
            if let Some(d) = &d {
                if let Some(dir) = cp.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                let _ = std::fs::write(&cp, d);
            }
            (u, d)
        }, move |(u, data)| {
            let mut surf = None;
            if let Some(d) = data {
                if let Some(pb) = img::from_bytes(&d) {
                    surf = img::surface(&img::to_width(pb, 960)).map(Rc::new);
                }
                me.raw.borrow_mut().insert(u.clone(), d);
            }
            me.pix.borrow_mut().insert(u.clone(), surf.clone());
            let cbs = me.waiting.borrow_mut().remove(&u).unwrap_or_default();
            for cb in cbs {
                cb(surf.clone());
            }
        });
    }
}

fn surf_from_pb(pb: Option<Pixbuf>) -> Option<Rc<ImageSurface>> {
    pb.and_then(|p| img::surface(&p)).map(Rc::new)
}

impl ThemePage {
    pub fn new_sddm() -> Rc<ThemePage> {
        Self::build(Kind::Sddm)
    }

    pub fn new_ply() -> Rc<ThemePage> {
        Self::build(Kind::Ply)
    }

    fn build(kind: Kind) -> Rc<ThemePage> {
        let (act, style, prompt_title, prompt_button, prompt_args, has_test, default_theme): (_, _, _, _, &'static [&'static str], _, _) =
            match kind {
                Kind::Sddm => ("sddm", "login", "sddm.prompt", "dm.install", &["dm-install"], true, "breeze"),
                Kind::Ply => ("ply", "boot", "ply.prompt", "ply.install", &["ply-setup"], false, "bgrt"),
            };
        let root = vbox(14);
        // Leiste: Login-Manager (SDDM) bzw. Bootsplash-Einrichtung (Plymouth)
        let bar = hbox(10);
        bar.add_css_class("DmBar");
        bar.set_size_request(-1, 54);
        hyprstudio::ui::widgets::margins(&bar, 0, 0, 0, 0);
        let bar_in = hbox(10);
        hyprstudio::ui::widgets::margins(&bar_in, 16, 8, 10, 8);
        bar_in.set_hexpand(true);
        bar.append(&bar_in);
        let b_reset = reset_button();
        b_reset.set_valign(gtk::Align::Center);
        let bar_label = label("", "Label");
        let bar_chips = ChipRow::new();
        bar_chips.tip_for(&bar_label);
        let bar_btn = gtk::Button::with_label("");
        bar_btn.add_css_class("Primary");
        cursor_hand(&bar_btn);
        bar_btn.set_size_request(-1, 34);
        bar_btn.set_valign(gtk::Align::Center);
        bar_in.append(&b_reset);
        bar_in.append(&bar_label);
        bar_chips.root.set_hexpand(true);
        bar_in.append(&bar_chips.root);
        let fill = hbox(0);
        fill.set_hexpand(true);
        bar_in.append(&fill);
        bar_in.append(&bar_btn);
        root.append(&bar);

        let body = gtk::Stack::new();
        body.set_vexpand(true);
        body.set_hhomogeneous(false);
        body.set_vhomogeneous(false);
        root.append(&body);

        // Seite 0: Aufforderung zum Installieren
        let prompt = vbox(0);
        prompt.add_css_class("AHero");
        let pin = vbox(0);
        hyprstudio::ui::widgets::margins(&pin, 24, 24, 24, 24);
        pin.set_vexpand(true);
        prompt.append(&pin);
        let top = vspacer();
        let prompt_pic = Picture::new(14.0);
        prompt_pic.area.set_size_request(320, 200);
        prompt_pic.area.set_halign(gtk::Align::Center);
        let prompt_label = label("", "DeName");
        prompt_label.set_justify(gtk::Justification::Center);
        prompt_label.set_xalign(0.5);
        prompt_label.set_halign(gtk::Align::Center);
        prompt_label.set_margin_top(18);
        let prompt_btn = gtk::Button::with_label("");
        prompt_btn.add_css_class("Primary");
        prompt_btn.set_size_request(260, 44);
        prompt_btn.set_halign(gtk::Align::Center);
        prompt_btn.set_margin_top(10);
        cursor_hand(&prompt_btn);
        let prompt_anchor = hbox(0);
        prompt_anchor.set_size_request(-1, 1);
        prompt_anchor.set_margin_top(16);
        let bottom = vspacer();
        // Verhältnis 2:3 der Freiräume über/unter dem Inhalt
        pin.append(&top);
        pin.append(&prompt_pic.area);
        pin.append(&prompt_label);
        pin.append(&prompt_btn);
        pin.append(&prompt_anchor);
        pin.append(&bottom);
        body.add_named(&prompt, Some("prompt"));

        // Seite 1: Themes
        let main = vbox(14);
        let hero = hbox(0);
        hero.add_css_class("AHero");
        hero.set_size_request(-1, 280);
        hero.set_vexpand(true);
        let hin = hbox(24);
        hyprstudio::ui::widgets::margins(&hin, 18, 18, 18, 18);
        hin.set_hexpand(true);
        hero.append(&hin);
        let big = Picture::new(12.0);
        big.area.set_hexpand(true);
        big.area.set_vexpand(true);
        hin.append(&big.area);
        let side = vbox(12);
        side.set_size_request(300, -1);
        let name = label("", "DeName");
        name.set_wrap(true);
        name.set_xalign(0.0);
        name.set_max_width_chars(14);
        side.append(&name);
        let chips = ChipRow::new();
        chips.tip_for(&name);
        side.append(&chips.root);
        let info = label("", "Label");
        info.set_wrap(true);
        info.set_xalign(0.0);
        info.set_max_width_chars(30);
        side.append(&info);
        let files_model = gtk::StringList::new(&[]);
        let files = gtk::DropDown::new(Some(files_model.clone()), gtk::Expression::NONE);
        files.add_css_class("Combo");
        files.set_margin_top(4);
        cursor_hand(&files);
        side.append(&files);
        side.append(&vspacer());
        let btn1 = gtk::Button::with_label("");
        btn1.add_css_class("Primary");
        let btn2 = gtk::Button::with_label("");
        let btn3 = gtk::Button::with_label("");
        btn3.add_css_class("Danger");
        for b in [&btn1, &btn2, &btn3] {
            cursor_hand(b);
            b.set_size_request(-1, 40);
            side.append(b);
        }
        hin.append(&side);
        main.append(&hero);
        let bar2 = hbox(10);
        let src = seg(&["local", "store"], "src.");
        let search_edit = search_entry(260);
        let sort_seg = seg(&["down", "new", "high"], "sort.");
        bar2.append(&src.root);
        let sp = hbox(0);
        sp.set_hexpand(true);
        bar2.append(&sp);
        bar2.append(&search_edit);
        bar2.append(&sort_seg.root);
        main.append(&bar2);
        let strip = TileStrip::new();
        main.append(&strip.root);
        body.add_named(&main, Some("main"));
        body.set_visible_child_name("prompt");

        let p = Rc::new(ThemePage { kind, act, style, prompt_title, prompt_button, prompt_args, has_test, default_theme,
            root, b_reset, bar_label, bar_chips, bar_btn, body, prompt_pic, prompt_label, prompt_btn, prompt_anchor, big,
            name, chips, info, files, files_model, files_data: RefCell::new((String::new(), vec![])), btn1, btn2, btn3,
            src, search_edit, search_timer: RefCell::new(None), sort_seg, strip, mode: RefCell::new("local".into()),
            sort: RefCell::new("down".into()), search: RefCell::new(String::new()), local: RefCell::new(vec![]),
            store: RefCell::new(vec![]), page: Cell::new(0), total: Cell::new(-1), loading: Cell::new(false),
            gen: Cell::new(0), sel_local: RefCell::new(None), sel_store: RefCell::new(None),
            local_pix: RefCell::new(HashMap::new()), current: RefCell::new(String::new()),
            mine: RefCell::new(Map::new()), mine_store: RefCell::new(HashSet::new()), tiles: RefCell::new(vec![]),
            anim_cache: RefCell::new(HashMap::new()), net: Rc::new(NetImg::default()) });
        p.src.select("local");
        p.sort_seg.select("down");
        p.connect();
        p
    }

    fn me(&self) -> Option<Rc<dyn AioPage>> {
        let w = win();
        let r = match self.kind {
            Kind::Sddm => w.sddm_page.get().cloned(),
            Kind::Ply => w.ply_page.get().cloned(),
        };
        r.map(|p| p as Rc<dyn AioPage>)
    }

    fn run(&self, args: Vec<String>, action: &str) {
        win().run(args, action, self.me());
    }

    fn connect(self: &Rc<Self>) {
        let w = Rc::downgrade(self);
        self.b_reset.connect_clicked(move |_| {
            if let Some(p) = w.upgrade() {
                p.reset_default();
            }
        });
        let w = Rc::downgrade(self);
        self.bar_btn.connect_clicked(move |_| {
            let Some(p) = w.upgrade() else { return };
            match p.kind {
                Kind::Sddm => p.run(vec!["dm-enable".into(), "sddm".into()], "dm-enable"),
                Kind::Ply => p.run(vec!["ply-setup".into()], "ply-setup"),
            }
        });
        let w = Rc::downgrade(self);
        self.prompt_btn.connect_clicked(move |_| {
            let Some(p) = w.upgrade() else { return };
            let args: Vec<String> = p.prompt_args.iter().map(|s| s.to_string()).collect();
            p.run(args, p.prompt_args[0]);
        });
        let w = Rc::downgrade(self);
        self.src.connect(move |m| {
            if let Some(p) = w.upgrade() {
                p.set_mode(&m);
            }
        });
        let w = Rc::downgrade(self);
        self.sort_seg.connect(move |s| {
            if let Some(p) = w.upgrade() {
                *p.sort.borrow_mut() = s;
                p.fetch_store(true);
            }
        });
        let w = Rc::downgrade(self);
        self.search_edit.connect_changed(move |_| {
            let Some(p) = w.upgrade() else { return };
            if let Some(id) = p.search_timer.borrow_mut().take() {
                id.remove();
            }
            let w2 = Rc::downgrade(&p);
            let id = glib::timeout_add_local_once(std::time::Duration::from_millis(450), move || {
                if let Some(p) = w2.upgrade() {
                    p.search_timer.borrow_mut().take();
                    p.search_now();
                }
            });
            *p.search_timer.borrow_mut() = Some(id);
        });
        let w = Rc::downgrade(self);
        self.strip.connect_end(move |_| {
            if let Some(p) = w.upgrade() {
                if *p.mode.borrow() == "store" {
                    p.fetch_store(false);
                }
            }
        });
        let w = Rc::downgrade(self);
        self.btn1.connect_clicked(move |_| {
            if let Some(p) = w.upgrade() {
                p.primary();
            }
        });
        let w = Rc::downgrade(self);
        self.btn2.connect_clicked(move |_| {
            if let Some(p) = w.upgrade() {
                p.secondary();
            }
        });
        let w = Rc::downgrade(self);
        self.btn3.connect_clicked(move |_| {
            if let Some(p) = w.upgrade() {
                p.remove();
            }
        });
    }

    // ----- Modul-Aufrufe -----

    fn list_themes(&self) -> Vec<ThemeInfo> {
        match self.kind {
            Kind::Sddm => sddm::list_themes(),
            Kind::Ply => plymouth::list_themes(),
        }
    }

    fn current_theme(&self) -> String {
        match self.kind {
            Kind::Sddm => sddm::current_theme(),
            Kind::Ply => plymouth::current_theme(),
        }
    }

    fn load_store_state(&self) -> Map<String, Value> {
        match self.kind {
            Kind::Sddm => sddm::load_store_state(),
            Kind::Ply => plymouth::load_store_state(),
        }
    }

    fn installed(&self) -> bool {
        match self.kind {
            Kind::Sddm => sddm::is_installed(),
            Kind::Ply => plymouth::is_installed(),
        }
    }

    fn list_url(&self, page: u32, sort: &str, search: &str) -> String {
        match self.kind {
            Kind::Sddm => sddm::list_url(page, sort, search),
            Kind::Ply => plymouth::list_url(page, sort, search),
        }
    }

    fn theme_chips(&self, th: &ThemeInfo) -> Vec<(String, String, String)> {
        match self.kind {
            Kind::Sddm => sddm::theme_chips(th),
            Kind::Ply => plymouth::theme_chips(th),
        }
    }

    fn test_command(&self, th: &ThemeInfo) -> Option<Vec<String>> {
        match self.kind {
            Kind::Sddm => sddm::test_command(th),
            Kind::Ply => plymouth::test_command(th),
        }
    }

    // ----- Zurücksetzen auf den Systemstandard -----

    fn reset_area(&self) -> String {
        if self.kind == Kind::Sddm { t("tab.sddm") } else { t("tab.ply") }
    }

    fn reset_default(self: &Rc<Self>) {
        let me = self.clone();
        // immer zuerst fragen (auch wenn nichts zu tun ist), erst danach prüfen
        hyprstudio::ui::dialogs::confirm(win().parent().as_ref(), &t("reset.title"),
            &tf("reset.ask", kw!("page" => self.reset_area())), Some(&t("reset.btn")), true, move |ok| {
                if !ok {
                    return;
                }
                let p = me;
                p.reload_local();
                let has = p.local.borrow().iter().any(|x| x.id == p.default_theme);
                let w = win();
                if !has {
                    w.set_status(&t("reset.nodefault"));
                    return;
                }
                let target = p.default_theme.to_string();
                let prev = p.current.borrow().clone();
                if prev == target {
                    w.set_status(&t("reset.already"));
                    return;
                }
                let action = format!("{}-set", p.act);
                p.run(vec![action.clone(), target.clone()], &action);
                if !prev.is_empty() {
                    let p2 = p.clone();
                    w.offer_undo(&tf("reset.sysdone", kw!("name" => target, "prev" => prev)), Box::new(move || {
                        p2.run(vec![action.clone(), prev], &action);
                    }));
                }
            });
    }

    // ----- Leisten -----

    fn refresh_bar(&self) {
        let busy = win().busy();
        self.b_reset.set_tooltip_text(Some(&t("reset.tip")));
        match self.kind {
            Kind::Sddm => {
                let active = system::active_display_manager();
                let have = system::sddm_installed();
                self.bar_label.set_text(&t("dm.label"));
                let kind = if active == "sddm" { "ChipOk" } else { "ChipAccent" };
                let tip = if active.is_empty() { String::new() } else { format!("{active}.service") };
                self.bar_chips.set(&[(system::dm_pretty(&active), kind.into(), tip)]);
                self.bar_btn.set_label(&t("dm.enable"));
                self.bar_btn.set_visible(have && active != "sddm");
                self.bar_btn.set_sensitive(!busy);
            }
            Kind::Ply => {
                let have = plymouth::is_installed();
                let hook = plymouth::has_hook();
                let splash = plymouth::splash_active();
                self.bar_label.set_text(&t("ply.label"));
                let mut chips = vec![(if have { t("installed") } else { t("ply.missing") },
                    if have { "ChipOk" } else { "ChipWarn" }.to_string(), String::new())];
                if have {
                    if let Some(h) = hook {
                        chips.push(("initramfs-Hook".into(), if h { "ChipOk" } else { "ChipWarn" }.into(),
                            if h { String::new() } else { t("ply.nohook") }));
                    }
                    chips.push(("splash".into(), if splash { "ChipOk" } else { "ChipWarn" }.into(),
                        if splash { String::new() } else { t("ply.nosplash") }));
                }
                self.bar_chips.set(&chips);
                self.bar_btn.set_label(&t("ply.setup"));
                self.bar_btn.set_visible(have && (hook == Some(false) || !splash));
                self.bar_btn.set_sensitive(!busy);
            }
        }
    }

    // ----- Daten -----

    fn reload_local(&self) {
        if self.kind == Kind::Ply {
            self.anim_cache.borrow_mut().clear();
        }
        *self.local.borrow_mut() = self.list_themes();
        *self.current.borrow_mut() = self.current_theme();
        let mine = self.load_store_state();
        *self.mine_store.borrow_mut() = mine.values()
            .filter_map(|v| v.get("store").map(hyprstudio::jsonx::py_str))
            .filter(|s| !s.is_empty() && s != "None").collect();
        *self.mine.borrow_mut() = mine;
        let local = self.local.borrow();
        let ok = self.sel_local.borrow().as_ref().map(|s| local.iter().any(|x| &x.id == s)).unwrap_or(false);
        if !ok {
            let cur = self.current.borrow().clone();
            let pick = local.iter().find(|x| x.id == cur).map(|x| x.id.clone())
                .or_else(|| local.first().map(|x| x.id.clone()));
            *self.sel_local.borrow_mut() = pick;
        }
    }

    fn fetch_store(self: &Rc<Self>, reset: bool) {
        if reset {
            self.gen.set(self.gen.get() + 1);
            self.store.borrow_mut().clear();
            self.page.set(0);
            self.total.set(-1);
            *self.sel_store.borrow_mut() = None;
        }
        if self.loading.get() || (self.total.get() >= 0 && self.store.borrow().len() as i64 >= self.total.get()) {
            return;
        }
        self.loading.set(true);
        let gen = self.gen.get();
        let url = self.list_url(self.page.get(), &self.sort.borrow(), &self.search.borrow());
        let w = Rc::downgrade(self);
        shell::run_bg(move || net_get(&url), move |res| {
            if let Some(p) = w.upgrade() {
                p.store_done(res, gen);
            }
        });
        if reset {
            self.rebuild();
        }
    }

    fn store_done(self: &Rc<Self>, res: Result<Vec<u8>, String>, gen: u32) {
        self.loading.set(false);
        if gen != self.gen.get() {
            return;
        }
        let data = match res {
            Err(e) => {
                win().log(&e);
                win().set_status(&t("sddm.neterr"));
                self.total.set(self.store.borrow().len() as i64);
                self.rebuild();
                return;
            }
            Ok(d) => d,
        };
        let (items, total) = sddm::parse_items(&data);
        self.total.set(total);
        self.page.set(self.page.get() + 1);
        let known: HashSet<String> = self.store.borrow().iter().map(|x| x.store_id.clone()).collect();
        let new: Vec<StoreItem> = items.iter().filter(|x| !known.contains(&x.store_id)).cloned().collect();
        self.store.borrow_mut().extend(new.iter().cloned());
        if items.is_empty() {
            self.total.set(self.store.borrow().len() as i64);
        }
        if self.sel_store.borrow().is_none() {
            let first = self.store.borrow().first().map(|x| x.store_id.clone());
            *self.sel_store.borrow_mut() = first;
        }
        if *self.mode.borrow() == "store" {
            if self.store.borrow().len() == new.len() {
                self.rebuild();
            } else {
                for it in &new {
                    self.add_store_tile(it);
                }
                self.refresh();
            }
        }
    }

    // ----- Umschalten -----

    fn set_mode(self: &Rc<Self>, mode: &str) {
        *self.mode.borrow_mut() = mode.to_string();
        if mode == "store" && self.store.borrow().is_empty() && !self.loading.get() {
            self.fetch_store(true);
        }
        self.rebuild();
    }

    /// Globale Suche: Theme-Namen ins Suchfeld.
    pub fn set_search(&self, text: &str) {
        self.search_edit.set_text(text);
    }

    fn search_now(self: &Rc<Self>) {
        let q = self.search_edit.text().trim().to_string();
        if q == *self.search.borrow() {
            return;
        }
        *self.search.borrow_mut() = q;
        if *self.mode.borrow() == "local" {
            self.rebuild();
        } else {
            self.fetch_store(true);
        }
    }

    fn select(&self, key: &str) {
        if *self.mode.borrow() == "local" {
            *self.sel_local.borrow_mut() = Some(key.to_string());
        } else {
            *self.sel_store.borrow_mut() = Some(key.to_string());
        }
        self.refresh();
    }

    // ----- Kacheln -----

    fn visible_local(&self) -> Vec<ThemeInfo> {
        let q = self.search.borrow().to_lowercase();
        self.local.borrow().iter()
            .filter(|x| q.is_empty() || x.name.to_lowercase().contains(&q) || x.id.to_lowercase().contains(&q))
            .cloned().collect()
    }

    pub fn rebuild(self: &Rc<Self>) {
        let local = *self.mode.borrow() == "local";
        if local {
            self.reload_local();
        }
        self.strip.clear();
        self.tiles.borrow_mut().clear();
        if local {
            for th in self.visible_local() {
                let pic = Picture::new(8.0);
                self.local_picture(&th, &pic, true);
                let tile = Tile::new(&th.id, &th.name, &pic.widget());
                self.hook_tile(&tile);
                let tip = if th.description.is_empty() { th.name.clone() } else { th.description.clone() };
                tile.root.set_tooltip_text(Some(&tip));
                self.strip.add(&tile.root);
                self.tiles.borrow_mut().push((th.id.clone(), tile, pic));
            }
        } else {
            let items = self.store.borrow().clone();
            for it in &items {
                self.add_store_tile(it);
            }
        }
        self.strip.to_start();
        self.refresh();
    }

    fn hook_tile(self: &Rc<Self>, tile: &Rc<Tile>) {
        let w = Rc::downgrade(self);
        tile.connect(move |k| {
            if let Some(p) = w.upgrade() {
                p.select(&k);
            }
        });
    }

    fn add_store_tile(self: &Rc<Self>, it: &StoreItem) {
        let pic = Picture::new(8.0);
        pic.set_placeholder(&it.name, &win().accent(), true, self.style);
        let pw = Rc::downgrade(&pic);
        self.net.image(&it.small, move |s| {
            if let Some(p) = pw.upgrade() {
                p.set_surface(s);
            }
        });
        let tile = Tile::new(&it.store_id, &it.name, &pic.widget());
        self.hook_tile(&tile);
        let tip = if it.summary.is_empty() { it.name.clone() } else { it.summary.clone() };
        tile.root.set_tooltip_text(Some(&tip));
        self.strip.add(&tile.root);
        self.tiles.borrow_mut().push((it.store_id.clone(), tile, pic));
    }

    fn local_picture(self: &Rc<Self>, th: &ThemeInfo, pic: &Rc<Picture>, tile: bool) {
        if let Some(a) = self.local_animation(th, tile) {
            pic.set_animation(a, true);
            return;
        }
        let path = th.screenshot.clone();
        if !path.is_empty() {
            let s = self.local_pix.borrow_mut().entry(path.clone())
                .or_insert_with(|| surf_from_pb(img::load(Path::new(&path), 0).map(|pb| img::to_width(pb, 960))))
                .clone();
            if s.is_some() {
                pic.set_surface(s);
                return;
            }
        }
        pic.set_placeholder(&th.id, &win().accent(), false, self.style);
    }

    /// Plymouth: Vorschau aus den Bildern des Themes. Kachel: ein fertiges Standbild. Große Vorschau: alle
    /// Einzelbilder in voller Länge als Schleife – im Hintergrund geladen, damit der Tab-Wechsel nicht hängt.
    fn local_animation(self: &Rc<Self>, th: &ThemeInfo, tile: bool) -> Option<Anim> {
        if self.kind != Kind::Ply {
            return None;
        }
        let key = (th.path.clone(), tile);
        if let Some(a) = self.anim_cache.borrow().get(&key) {
            return a.clone();
        }
        let a = plymouth::preview_assets(Path::new(&th.path));
        if a.frames.is_empty() && a.logo.is_empty() {
            self.anim_cache.borrow_mut().insert(key, None);
            return None;
        }
        let load = |f: &str, limit: i32| -> Option<Pixbuf> {
            if f.is_empty() { None } else { img::load(Path::new(f), limit) }
        };
        let logo = surf_from_pb(load(&a.logo, 700));
        let bgimg = surf_from_pb(load(&a.background, 1600));
        let make = |frames: Vec<Option<Pixbuf>>| Anim {
            frames: frames.into_iter().flatten().filter_map(|p| img::surface(&p)).map(Rc::new).collect(),
            logo: logo.clone(),
            bgimg: bgimg.clone(),
            bg: a.bg.clone(),
            bg2: a.bg2.clone(),
            ha: a.ha,
            va: a.va,
            wva: a.wva,
        };
        // Standbild: letztes Bild der Animation (dort ist das Logo komplett)
        let still = a.frames.last().map(|f| vec![load(f, 700)]).unwrap_or_default();
        let first = make(still);
        self.anim_cache.borrow_mut().insert(key.clone(), Some(first.clone()));
        if tile || a.frames.len() <= 1 {
            return Some(first);
        }
        let files = a.frames.clone();
        let w = Rc::downgrade(self);
        let path = th.path.clone();
        let base = first.clone();
        shell::run_bg(move || {
            files.iter().map(|f| img::load_raw(Path::new(f), 700)).collect::<Vec<Option<img::Raw>>>()
        }, move |pbs| {
            let Some(p) = w.upgrade() else { return };
            let mut full = base;
            full.frames = pbs.into_iter().flatten().filter_map(|r| img::surface(&r.pixbuf())).map(Rc::new).collect();
            p.anim_cache.borrow_mut().insert(key, Some(full.clone()));
            if *p.mode.borrow() == "local" {
                if let Some(LocalOrStore::Local(it)) = p.item() {
                    if it.path == path {
                        p.big.set_animation(full, true);
                    }
                }
            }
        });
        Some(first)
    }

    // ----- Anzeige -----

    fn item(&self) -> Option<LocalOrStore> {
        if *self.mode.borrow() == "local" {
            let key = self.sel_local.borrow().clone()?;
            self.local.borrow().iter().find(|x| x.id == key).cloned().map(LocalOrStore::Local)
        } else {
            let key = self.sel_store.borrow().clone()?;
            self.store.borrow().iter().find(|x| x.store_id == key).cloned().map(LocalOrStore::Store)
        }
    }

    fn me_rc(&self) -> Option<Rc<ThemePage>> {
        let w = win();
        match self.kind {
            Kind::Sddm => w.sddm_page.get().cloned(),
            Kind::Ply => w.ply_page.get().cloned(),
        }
    }

    fn files_current(&self) -> Option<u32> {
        let i = self.files.selected();
        self.files_data.borrow().1.get(i as usize).copied()
    }

    fn primary(self: &Rc<Self>) {
        match self.item() {
            Some(LocalOrStore::Local(it)) => {
                let a = format!("{}-set", self.act);
                self.run(vec![a.clone(), it.id], &a);
            }
            Some(LocalOrStore::Store(it)) => self.download(it),
            None => {}
        }
    }

    fn secondary(&self) {
        match self.item() {
            Some(LocalOrStore::Local(it)) => {
                if let Some(cmd) = self.test_command(&it) {
                    win().log(&format!("$ {}", cmd.join(" ")));
                    let refs: Vec<&str> = cmd.iter().map(|s| s.as_str()).collect();
                    let _ = hyprstudio::util::spawn_detached(&refs);
                }
            }
            Some(LocalOrStore::Store(it)) => {
                let url = sddm::STORE_PAGE.replace("{id}", &it.store_id);
                let _ = gtk::gio::AppInfo::launch_default_for_uri(&url, gtk::gio::AppLaunchContext::NONE);
            }
            None => {}
        }
    }

    fn remove(self: &Rc<Self>) {
        let Some(LocalOrStore::Local(it)) = self.item() else { return };
        let me = self.clone();
        win().confirm(&t("remove"), &tf("confirm.remove", kw!("name" => it.name)), move |ok| {
            if ok {
                let a = format!("{}-remove", me.act);
                me.run(vec![a.clone(), it.id], &a);
            }
        });
    }

    fn download(self: &Rc<Self>, it: StoreItem) {
        let index = if self.files.is_visible() {
            self.files_current().unwrap_or(1)
        } else {
            it.files.first().map(|f| f.index).unwrap_or(1)
        };
        let w = win();
        w.task_begin(&format!("{} · {}", t("sddm.downloading"), it.name));
        let (ptx, prx) = async_channel::unbounded::<u32>();
        glib::spawn_future_local(async move {
            while let Ok(v) = prx.recv().await {
                win().task_progress(v as i32);
            }
        });
        let item = it.clone();
        let me = self.clone();
        shell::run_job(move |log| {
            // frische Download-Links holen
            let fresh = net_get(&sddm::item_url(&item.store_id)).ok().and_then(|d| sddm::parse_item(&d));
            let files = fresh.map(|f| f.files).filter(|f| !f.is_empty()).unwrap_or_else(|| item.files.clone());
            let f = files.iter().find(|x| x.index == index).or(files.first()).cloned();
            let Some(f) = f else { return Err(String::new()) };
            log(&format!("GET {}", f.name));
            let data = download_with_progress(&f.link, &ptx).map_err(|e| {
                log(&e);
                e
            })?;
            if data.is_empty() {
                return Err(String::new());
            }
            // Originaler Dateiname (ohne Store-Nummer davor), eigener Ordner pro Store-Eintrag
            let folder = usercmd::cache().join("downloads").join(&item.store_id);
            let _ = std::fs::create_dir_all(&folder);
            let re = regex::Regex::new(r"[^A-Za-z0-9._+-]+").unwrap();
            let mut name = re.replace_all(&f.name, "-").to_string();
            if name.is_empty() {
                name = "theme.tar.gz".into();
            }
            let path = folder.join(name);
            std::fs::write(&path, &data).map_err(|e| e.to_string())?;
            log(&format!("{} ({} KB)", path.display(), data.len() / 1024));
            Ok((path, folder))
        }, move |res| {
            let w = win();
            match res {
                Err(_) => w.task_end(false, &t("sddm.neterr")),
                Ok((path, folder)) => {
                    let a = format!("{}-install", me.act);
                    let mut args = vec![a.clone(), path.to_string_lossy().into_owned(), format!("--store={}", it.store_id)];
                    if let Some(raw) = me.net.raw.borrow().get(&it.preview) {
                        let pv = folder.join("preview.png");
                        if let Some(pb) = img::from_bytes(raw) {
                            if pb.savev(&pv, "png", &[]).is_ok() {
                                args.push(format!("--preview={}", pv.display()));
                            }
                        }
                    }
                    w.task_end(true, "");
                    me.run(args, &a);
                }
            }
        });
    }

    fn hero_pix(&self, key: &str, s: Option<Rc<ImageSurface>>) {
        if *self.mode.borrow() != "store" || self.sel_store.borrow().as_deref() != Some(key) || s.is_none() {
            return;
        }
        let raw = match self.item() {
            Some(LocalOrStore::Store(it)) => self.net.raw.borrow().get(&it.preview).cloned(),
            _ => None,
        };
        match raw {
            Some(r) if r.starts_with(b"GIF8") => self.big.set_image_data(&r), // animierte Store-Vorschau abspielen
            _ => self.big.set_surface(s),
        }
    }
}

enum LocalOrStore {
    Local(ThemeInfo),
    Store(StoreItem),
}

/// Datei laden, Fortschritt in Prozent melden.
fn download_with_progress(url: &str, ptx: &async_channel::Sender<u32>) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let agent = hyprstudio::net::builder().timeout_connect(std::time::Duration::from_secs(20)).build();
    let resp = agent.get(url).set("User-Agent", &remote::ua()).call().map_err(|e| e.to_string())?;
    let total: u64 = resp.header("Content-Length").and_then(|s| s.parse().ok()).unwrap_or(0);
    let mut r = resp.into_reader();
    let mut out = Vec::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = r.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        out.extend_from_slice(&buf[..n]);
        let pct = if total > 0 { (out.len() as u64 * 100 / total) as u32 } else { 0 };
        let _ = ptx.send_blocking(pct);
    }
    Ok(out)
}

impl AioPage for ThemePage {
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }

    fn drawer_anchor(&self) -> gtk::Widget {
        if self.body.visible_child_name().as_deref() == Some("main") {
            self.src.root.clone().upcast()
        } else {
            self.prompt_anchor.clone().upcast()
        }
    }

    fn retranslate(&self) {
        self.src.retranslate();
        self.sort_seg.retranslate();
        self.search_edit.set_placeholder_text(Some(&t("search")));
        self.refresh();
    }

    fn finished(&self, ok: bool, action: &str, result: &str) {
        let Some(me) = self.me_rc() else { return };
        me.reload_local();
        if ok && (action == format!("{}-install", self.act) || action == format!("{}-set", self.act)) && !result.is_empty() {
            *me.sel_local.borrow_mut() = Some(result.to_string());
            *me.mode.borrow_mut() = "local".into();
            me.src.select("local");
        }
        me.rebuild();
    }

    fn refresh(&self) {
        self.refresh_bar();
        let w = win();
        let accent = w.accent();
        let busy = w.busy();
        if !self.installed() {
            self.body.set_visible_child_name("prompt");
            self.prompt_label.set_text(&t(self.prompt_title));
            self.prompt_btn.set_label(&t(self.prompt_button));
            self.prompt_btn.set_sensitive(!busy);
            self.prompt_pic.set_placeholder(self.act, &accent, false, self.style);
            return;
        }
        if self.body.visible_child_name().as_deref() != Some("main") {
            self.body.set_visible_child_name("main");
            self.reload_local();
            if let Some(me) = self.me_rc() {
                glib::idle_add_local_once(move || me.rebuild());
            }
        }
        let store = *self.mode.borrow() == "store";
        self.sort_seg.root.set_visible(store);
        let sel = if store { self.sel_store.borrow().clone() } else { self.sel_local.borrow().clone() };
        let cur = self.current.borrow().clone();
        for (key, tile, _) in self.tiles.borrow().iter() {
            let dot: Option<String> = if store {
                self.mine_store.borrow().contains(key).then(|| theme::OK.to_string())
            } else if *key == cur {
                Some(accent.clone())
            } else {
                self.mine.borrow().contains_key(key).then(|| theme::OK.to_string())
            };
            tile.refresh(sel.as_deref() == Some(key.as_str()), dot.as_deref());
        }

        let it = self.item();
        self.files.set_visible(false);
        for b in [&self.btn1, &self.btn2, &self.btn3] {
            b.set_visible(true);
            b.set_tooltip_text(None);
        }
        let Some(it) = it else {
            let empty = self.loading.get() && store;
            self.name.set_text(&if empty { t("sddm.loading") } else { t("sddm.none") });
            *self.chips.base_tip.borrow_mut() = String::new();
            self.chips.set(&[]);
            self.info.set_text("");
            self.big.set_placeholder(self.act, &accent, empty, self.style);
            for b in [&self.btn1, &self.btn2, &self.btn3] {
                b.set_visible(false);
            }
            return;
        };
        match it {
            LocalOrStore::Store(it) => {
                self.name.set_text(&it.name);
                *self.chips.base_tip.borrow_mut() = it.summary.clone();
                let mut chips = vec![];
                if self.mine_store.borrow().contains(&it.store_id) {
                    chips.push((t("installed"), "ChipOk".to_string(), String::new()));
                }
                if it.qt6 && self.kind == Kind::Sddm {
                    chips.push(("Qt6".into(), "Chip".into(), String::new()));
                }
                self.chips.set(&chips);
                let mut score = format!("{:.1}", it.score as f64 / 10.0);
                if lang() == "de" {
                    score = score.replace('.', ",");
                }
                self.info.set_text(&format!("{}  ·  {} Downloads  ·  {}/10", it.author, num(it.downloads), score));
                self.big.set_placeholder(&it.name, &accent, true, self.style);
                let key = it.store_id.clone();
                if let Some(me) = self.me_rc() {
                    let wm = Rc::downgrade(&me);
                    self.net.image(&it.preview, move |s| {
                        if let Some(p) = wm.upgrade() {
                            p.hero_pix(&key, s);
                        }
                    });
                }
                if it.files.len() > 1 {
                    let keep = if self.files_data.borrow().0 == it.store_id { self.files_current() } else { None };
                    let names: Vec<String> = it.files.iter().map(|f| format!("{}  ({} KB)", f.name, f.size_kb)).collect();
                    let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
                    self.files_model.splice(0, self.files_model.n_items(), &refs);
                    let idx: Vec<u32> = it.files.iter().map(|f| f.index).collect();
                    let pos = keep.and_then(|k| idx.iter().position(|x| *x == k)).unwrap_or(0);
                    *self.files_data.borrow_mut() = (it.store_id.clone(), idx);
                    self.files.set_selected(pos as u32);
                    self.files.set_tooltip_text(Some(&t("sddm.files")));
                    self.files.set_visible(true);
                }
                set_applied(&self.btn1, false, "");
                self.btn1.set_label(&t("sddm.download"));
                self.btn1.set_sensitive(!busy && !it.files.is_empty());
                self.btn2.set_label(&t("sddm.open"));
                self.btn2.set_sensitive(true);
                self.btn3.set_visible(false);
            }
            LocalOrStore::Local(it) => {
                self.name.set_text(&it.name);
                *self.chips.base_tip.borrow_mut() = it.description.clone();
                let mut chips = vec![];
                if it.id == cur {
                    chips.push((t("sddm.active"), "ChipAccent".to_string(), String::new()));
                }
                chips.extend(self.theme_chips(&it));
                let mine = self.mine.borrow().contains_key(&it.id);
                if mine {
                    chips.push((t("sddm.mine"), "ChipOk".into(), String::new()));
                }
                self.chips.set(&chips);
                let mut bits: Vec<String> = vec![];
                let ver = if it.version.is_empty() { String::new() } else { format!("v{}", it.version) };
                for b in [it.author.clone(), ver, it.id.clone()] {
                    if !b.is_empty() && !bits.contains(&b) {
                        bits.push(b);
                    }
                }
                self.info.set_text(&bits.join("  ·  "));
                if let Some(me) = self.me_rc() {
                    me.local_picture(&it, &me.big, false);
                }
                let active = it.id == cur;
                set_applied(&self.btn1, active, &t("applied"));
                if !active {
                    self.btn1.set_label(&t("sddm.set"));
                    self.btn1.set_sensitive(!busy);
                }
                if self.has_test {
                    self.btn2.set_label(&t("sddm.test"));
                    let can = self.test_command(&it).is_some();
                    self.btn2.set_sensitive(can);
                    self.btn2.set_tooltip_text(if can { None } else { Some(t("sddm.nogreeter")) }.as_deref());
                } else {
                    self.btn2.set_visible(false);
                }
                self.btn3.set_label(&t("remove"));
                self.btn3.set_sensitive(!busy && mine && it.id != cur);
                self.btn3.set_tooltip_text(if mine { None } else { Some(t("err.sddm.foreign")) }.as_deref());
            }
        }
    }
}
