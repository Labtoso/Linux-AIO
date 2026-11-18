//! Tab „Einstellungen“: links alle Desktops (installierte wählbar, andere ausgegraut, laufender vorgewählt),
//! rechts die Einstellungen des gewählten Desktops.

use super::advanced_page::{self, SUBS};
use super::font_page::FontsPage;
use super::gnome_page as g;
use super::hypr_embed::{HyprEmbed, HyprLauncher};
use super::look_page::LookPage;
use super::plasma_page::PlasmaLook;
use super::win::{win, AioPage};
use crate::desktops::{self, Entry};
use crate::i18n::t;
use gtk::prelude::*;
use hyprstudio::jsonx::Get;
use hyprstudio::ui::draw::Rgba;
use hyprstudio::ui::widgets::{hbox, label, vbox, NavHeader};
use serde_json::json;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

pub fn sections(de: &str) -> &'static [&'static str] {
    match de {
        "gnome" => &["look", "cursor", "icon", "fonts", "windows", "titlebar", "topbar", "input", "autostart", "login"],
        "plasma" => &["plasma", "cursor", "icon"],
        "hyprland" => &["hyprstudio", "launcher", "cursor", "icon"],
        _ => &["cursor", "icon"],
    }
}

fn make_section(de: &str, key: &str) -> Rc<dyn AioPage> {
    match key {
        "cursor" | "icon" => LookPage::new(key, de),
        "plasma" => PlasmaLook::new(),
        "hyprstudio" => {
            if win().cfg.borrow().s("hypr_mode") == "window" {
                HyprLauncher::new()
            } else {
                HyprEmbed::new(false)
            }
        }
        "launcher" => HyprEmbed::new(true),
        "look" => g::LookGnome::new(),
        "titlebar" => g::TitlebarPage::new(),
        "windows" => g::WindowsPage::new(),
        "fonts" => FontsPage::new(),
        "topbar" => g::TopbarPage::new(),
        "autostart" => g::AutostartPage::new(),
        "login" => g::LoginPage::new(),
        _ => g::simple_page(key, key),
    }
}

fn nav_row(text: &str, height: i32, dot: Option<&gtk::DrawingArea>) -> (gtk::ListBoxRow, gtk::Label) {
    let row = gtk::ListBoxRow::new();
    let b = hbox(8);
    b.set_size_request(-1, height - 8);
    if let Some(d) = dot {
        b.append(d);
    }
    let l = label(text, "");
    l.set_xalign(0.0);
    l.set_hexpand(true);
    l.set_ellipsize(gtk::pango::EllipsizeMode::End);
    b.append(&l);
    row.set_child(Some(&b));
    (row, l)
}

// ---------- Einstellungen eines Desktops ----------

pub struct DePanel {
    pub de: String,
    root: gtk::Box,
    keys: Vec<&'static str>,
    nav: gtk::ListBox,
    nav_labels: Vec<gtk::Label>,
    stack: gtk::Stack,
    pub pages: RefCell<HashMap<String, Rc<dyn AioPage>>>,
    pub current: RefCell<String>,
    block: Cell<bool>,
}

impl DePanel {
    pub fn new(de: &str) -> Rc<DePanel> {
        let keys: Vec<&'static str> = sections(de).to_vec();
        let root = hbox(14);
        let nav = gtk::ListBox::new();
        nav.add_css_class("NavList");
        nav.set_selection_mode(gtk::SelectionMode::Single);
        let mut nav_labels = vec![];
        for _ in &keys {
            let (row, l) = nav_row("", 38, None);
            nav.append(&row);
            nav_labels.push(l);
        }
        let nav_sc = gtk::ScrolledWindow::new();
        nav_sc.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        nav_sc.set_child(Some(&nav));
        nav_sc.set_size_request(176, -1);
        nav_sc.set_propagate_natural_height(true);
        nav_sc.set_valign(gtk::Align::Start);
        nav_sc.set_visible(keys.len() > 1);
        root.append(&nav_sc);
        let stack = gtk::Stack::new();
        stack.set_hexpand(true);
        stack.set_vexpand(true);
        stack.set_transition_type(gtk::StackTransitionType::None);
        root.append(&stack);
        let p = Rc::new(DePanel { de: de.into(), root, keys, nav, nav_labels, stack, pages: RefCell::new(HashMap::new()),
            current: RefCell::new(String::new()), block: Cell::new(false) });
        let me = Rc::downgrade(&p);
        p.nav.connect_row_selected(move |_, row| {
            let (Some(p), Some(row)) = (me.upgrade(), row) else { return };
            if p.block.get() {
                return;
            }
            let k = p.keys[row.index() as usize];
            p.show_section(k);
        });
        p.retranslate();
        let start = win().cfg.borrow().s(&format!("section_{de}")).to_string();
        let start = p.keys.iter().find(|k| **k == start).copied().unwrap_or(p.keys[0]);
        p.show_section(start);
        p
    }

    pub fn show_section(self: &Rc<Self>, key: &str) {
        let Some(i) = self.keys.iter().position(|k| *k == key) else { return };
        let key = self.keys[i];
        let created = !self.pages.borrow().contains_key(key);
        if created {
            let pg = make_section(&self.de, key);
            self.stack.add_named(&pg.widget(), Some(key));
            pg.retranslate();
            self.pages.borrow_mut().insert(key.into(), pg);
        }
        let pg = self.pages.borrow().get(key).cloned().unwrap();
        self.stack.set_visible_child_name(key);
        *self.current.borrow_mut() = key.into();
        if self.nav.selected_row().map(|r| r.index()) != Some(i as i32) {
            self.block.set(true);
            self.nav.select_row(self.nav.row_at_index(i as i32).as_ref());
            self.block.set(false);
        }
        pg.refresh();
        let w = win();
        w.cfg.borrow_mut().insert(format!("section_{}", self.de), json!(key));
        w.save();
    }

    pub fn page(&self, key: &str) -> Option<Rc<dyn AioPage>> {
        self.pages.borrow().get(key).cloned()
    }

    pub fn retranslate(&self) {
        for (l, k) in self.nav_labels.iter().zip(&self.keys) {
            l.set_text(&t(&format!("sec.{k}")));
        }
        let cur = self.current.borrow().clone();
        let pages: Vec<(String, Rc<dyn AioPage>)> = self.pages.borrow().iter().map(|(k, p)| (k.clone(), p.clone())).collect();
        for (k, pg) in pages {
            pg.retranslate();
            if k == cur && !matches!(k.as_str(), "hyprstudio" | "launcher") {
                pg.refresh();
            }
        }
    }

    pub fn apply_accent(&self) {
        let cur = self.current.borrow().clone();
        let pages: Vec<(String, Rc<dyn AioPage>)> = self.pages.borrow().iter().map(|(k, p)| (k.clone(), p.clone())).collect();
        for (k, pg) in pages {
            if matches!(k.as_str(), "hyprstudio" | "launcher") || (k == cur && matches!(k.as_str(), "cursor" | "icon")) {
                pg.refresh();
            }
        }
    }

    pub fn refresh_busy(&self) {
        let cur = self.current.borrow().clone();
        if matches!(cur.as_str(), "cursor" | "icon" | "login") {
            if let Some(pg) = self.page(&cur) {
                pg.refresh();
            }
        }
    }
}

// ---------- Seitenleiste + Inhalt ----------

/// Überschrift „Desktops“ / „Advanced“ (gleiches Widget wie in Hypr Studio).
fn nav_tr(key: &str) -> String {
    match key {
        "nav.desktops" => t("set.desktops"),
        "nav.advanced" => t("sec.advanced"),
        k => t(k),
    }
}

pub struct SettingsPage {
    root: gtk::Box,
    entries: Vec<Entry>,
    running: Option<String>,
    list: gtk::ListBox,
    rows: Vec<(gtk::ListBoxRow, gtk::Label, gtk::DrawingArea)>,
    adv: gtk::ListBox,
    adv_labels: Vec<gtk::Label>,
    heads: Vec<Rc<NavHeader>>,
    stack: gtk::Stack,
    empty: gtk::Label,
    panels: RefCell<HashMap<String, Rc<DePanel>>>,
    adv_pages: RefCell<HashMap<String, Rc<dyn AioPage>>>,
    block: Cell<bool>,
}

impl SettingsPage {
    pub fn new() -> Rc<SettingsPage> {
        let entries = desktops::entries();
        let running = desktops::current();
        let root = hbox(16);
        let side = vbox(8);
        side.add_css_class("SideBox");
        side.set_size_request(180, -1);
        side.add_css_class("PadSide");
        let conf = win().conf();

        // Desktops
        let head_de = NavHeader::new("settings:desktops", "desktops", win().conf());
        head_de.tr.set(nav_tr);
        let list = gtk::ListBox::new();
        list.add_css_class("DeList");
        let mut rows = vec![];
        for e in &entries {
            let dot = gtk::DrawingArea::new();
            dot.set_content_width(10);
            dot.set_content_height(10);
            dot.set_valign(gtk::Align::Center);
            let (row, l) = nav_row(&e.name, 36, Some(&dot));
            list.append(&row);
            rows.push((row, l, dot));
        }
        let list_sc = gtk::ScrolledWindow::new();
        list_sc.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        list_sc.set_child(Some(&list));
        list_sc.set_vexpand(true);
        head_de.widgets.borrow_mut().push(list_sc.clone().upcast());
        side.append(&head_de.button);
        side.append(&list_sc);

        // Advanced
        let head_adv = NavHeader::new("settings:advanced", "advanced", conf);
        head_adv.tr.set(nav_tr);
        let adv = gtk::ListBox::new();
        adv.add_css_class("DeList");
        let mut adv_labels = vec![];
        for _ in SUBS {
            let dot = gtk::DrawingArea::new();
            dot.set_content_width(10);
            dot.set_content_height(10);
            let (row, l) = nav_row("", 36, Some(&dot));
            adv.append(&row);
            adv_labels.push(l);
        }
        head_adv.widgets.borrow_mut().push(adv.clone().upcast());
        side.append(&head_adv.button);
        side.append(&adv);
        // zugeklappte Desktops: Advanced rutscht nach oben
        let spacer = vbox(0);
        side.append(&spacer);
        let sp = spacer.clone();
        head_de.on_sync.replace(Some(Box::new(move |collapsed| sp.set_vexpand(collapsed))));
        root.append(&side);

        let stack = gtk::Stack::new();
        stack.set_hexpand(true);
        stack.set_vexpand(true);
        let empty = label("", "ALabel");
        empty.set_halign(gtk::Align::Center);
        empty.set_valign(gtk::Align::Center);
        stack.add_named(&empty, Some("empty"));
        root.append(&stack);

        let p = Rc::new(SettingsPage { root, entries, running, list, rows, adv, adv_labels, heads: vec![head_de, head_adv],
            stack, empty, panels: RefCell::new(HashMap::new()), adv_pages: RefCell::new(HashMap::new()),
            block: Cell::new(false) });
        for h in &p.heads {
            h.sync();
        }
        for (i, (_, _, dot)) in p.rows.iter().enumerate() {
            let me = Rc::downgrade(&p);
            dot.set_draw_func(move |_, cr, w, h| {
                let Some(p) = me.upgrade() else { return };
                if p.running.as_deref() != Some(p.entries[i].id.as_str()) {
                    return;
                }
                Rgba::hex(&win().accent()).set(cr);
                cr.arc(w as f64 / 2.0, h as f64 / 2.0, 5.0, 0.0, std::f64::consts::TAU);
                let _ = cr.fill();
            });
        }
        let me = Rc::downgrade(&p);
        p.list.connect_row_selected(move |_, row| {
            let (Some(p), Some(row)) = (me.upgrade(), row) else { return };
            if !p.block.get() {
                let id = p.entries[row.index() as usize].id.clone();
                p.select(&id);
            }
        });
        let me = Rc::downgrade(&p);
        p.adv.connect_row_selected(move |_, row| {
            let (Some(p), Some(row)) = (me.upgrade(), row) else { return };
            if !p.block.get() {
                p.select_adv(SUBS[row.index() as usize]);
            }
        });
        p.mark();

        let w = win();
        let (cfg_de, cfg_adv) = {
            let c = w.cfg.borrow();
            (c.s("settings_de").to_string(), c.s("settings_adv").to_string())
        };
        let mut start = match &p.running {
            Some(r) if p.installed(r) => Some(r.clone()),
            _ => Some(cfg_de),
        };
        if !start.as_deref().map(|s| !s.is_empty() && p.installed(s)).unwrap_or(false) {
            start = p.entries.iter().find(|e| desktops::installed(e)).map(|e| e.id.clone());
        }
        if SUBS.contains(&cfg_adv.as_str()) {
            p.select_adv(&cfg_adv);
        } else if let Some(s) = start {
            p.select(&s);
        }
        p
    }

    fn installed(&self, de_id: &str) -> bool {
        desktops::get(de_id).map(|e| desktops::installed(&e)).unwrap_or(false)
    }

    /// Installierte wählbar, nicht installierte ausgegraut; Punkt = läuft gerade.
    pub fn mark(&self) {
        for (e, (row, _l, dot)) in self.entries.iter().zip(&self.rows) {
            let inst = desktops::installed(e);
            row.set_sensitive(inst);
            row.set_selectable(inst);
            let tip = if self.running.as_deref() == Some(e.id.as_str()) {
                t("set.running")
            } else if inst {
                t("installed")
            } else {
                t("set.notinstalled")
            };
            row.set_tooltip_text(Some(&tip));
            dot.queue_draw();
        }
    }

    fn unselect(&self, l: &gtk::ListBox) {
        self.block.set(true);
        l.unselect_all();
        self.block.set(false);
    }

    /// Advanced › Kitty / Alacritty / yay rechts zeigen.
    pub fn select_adv(&self, key: &str) -> Option<Rc<dyn AioPage>> {
        let i = SUBS.iter().position(|k| *k == key)?;
        if !self.adv_pages.borrow().contains_key(key) {
            let pg = advanced_page::make(key);
            self.stack.add_named(&pg.widget(), Some(&format!("adv:{key}")));
            pg.retranslate();
            self.adv_pages.borrow_mut().insert(key.into(), pg);
        }
        self.stack.set_visible_child_name(&format!("adv:{key}"));
        self.unselect(&self.list);
        if self.adv.selected_row().map(|r| r.index()) != Some(i as i32) {
            self.block.set(true);
            self.adv.select_row(self.adv.row_at_index(i as i32).as_ref());
            self.block.set(false);
        }
        let w = win();
        w.cfg.borrow_mut().insert("settings_adv".into(), json!(key));
        w.save();
        self.adv_pages.borrow().get(key).cloned()
    }

    pub fn select(&self, de_id: &str) {
        if !self.installed(de_id) {
            return;
        }
        if !self.panels.borrow().contains_key(de_id) {
            let p = DePanel::new(de_id);
            self.stack.add_named(&p.root, Some(&format!("de:{de_id}")));
            self.panels.borrow_mut().insert(de_id.into(), p);
        }
        self.stack.set_visible_child_name(&format!("de:{de_id}"));
        if let Some(i) = self.entries.iter().position(|e| e.id == de_id) {
            if self.list.selected_row().map(|r| r.index()) != Some(i as i32) {
                self.block.set(true);
                self.list.select_row(self.list.row_at_index(i as i32).as_ref());
                self.block.set(false);
            }
        }
        self.unselect(&self.adv);
        let w = win();
        {
            let mut c = w.cfg.borrow_mut();
            c.insert("settings_de".into(), json!(de_id));
            c.insert("settings_adv".into(), json!(""));
        }
        w.save();
    }

    /// Der gerade sichtbare Desktop-Bereich.
    pub fn panel(&self) -> Option<Rc<DePanel>> {
        let name = self.stack.visible_child_name()?.to_string();
        let id = name.strip_prefix("de:")?;
        self.panels.borrow().get(id).cloned()
    }

    /// Für Testläufe und die Suche: Bereich im gewählten Desktop zeigen.
    pub fn show_section(&self, sec: &str, _sub: Option<&str>) -> Option<Rc<dyn AioPage>> {
        let p = self.panel()?;
        p.show_section(sec);
        p.page(sec)
    }

    pub fn apply_accent(&self) {
        self.mark();
        for p in self.panels.borrow().values() {
            p.apply_accent();
        }
    }
}

impl AioPage for SettingsPage {
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }

    fn drawer_anchor(&self) -> gtk::Widget {
        self.stack.clone().upcast()
    }

    fn retranslate(&self) {
        for h in &self.heads {
            h.sync();
        }
        for (l, k) in self.adv_labels.iter().zip(SUBS) {
            l.set_text(&t(&format!("adv.{k}")));
        }
        let pages: Vec<Rc<dyn AioPage>> = self.adv_pages.borrow().values().cloned().collect();
        for pg in pages {
            pg.retranslate();
        }
        self.empty.set_text(&t("set.none"));
        self.mark();
        let panels: Vec<Rc<DePanel>> = self.panels.borrow().values().cloned().collect();
        for p in panels {
            p.retranslate();
        }
    }

    /// Frisch installierte Desktops sofort wählbar (läuft nach jeder Aktion).
    fn refresh(&self) {
        self.mark();
        if let Some(p) = self.panel() {
            p.refresh_busy();
        }
    }
}
