//! Installiert-Prüfung: fehlt ein Programm, zeigt der Bereich statt der Einstellungen einen großen
//! Installieren-Knopf. Nach der Installation erscheinen die Einstellungen automatisch.

use super::draw;
use super::editor::EditorPage;
use super::studio::Studio;
use super::widgets::{button, flow_grid, icon_button, label, scroller, set_button_text, vbox, vspacer};
use crate::i18n::{t, tf};
use crate::{kw, launcher, packages};
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

fn big_button(primary: bool) -> gtk::Button {
    let b = button("", if primary { "Primary" } else { "" });
    b.add_css_class("Big");
    b.set_size_request(300, 56);
    b
}

/// Was hinter einer Gate steckt (Kitty, Hintergrundbild, Mauszeiger, Sperrbildschirm, Waybar-Seite).
pub trait GateContent {
    fn widget(&self) -> gtk::Widget;
    fn actions(&self) -> Vec<gtk::Button> {
        vec![]
    }
    fn refresh(&self) {}
}

/// Seite „content“ = Inhalt, Seite „missing“ = „nicht installiert“ mit großem Knopf.
pub struct Gate {
    pub stack: gtk::Stack,
    pub content: Rc<dyn GateContent>,
    pub key: String,
    title: gtk::Label,
    sub: gtk::Label,
    btn: gtk::Button,
}

impl Gate {
    pub fn new(content: Rc<dyn GateContent>, key: &str, win: &Rc<Studio>) -> Rc<Gate> {
        let stack = gtk::Stack::new();
        stack.set_vexpand(true);
        stack.add_named(&content.widget(), Some("content"));
        let bx = vbox(14);
        bx.add_css_class("Card");
        super::widgets::margins(&bx, 40, 40, 40, 40);
        bx.append(&vspacer());
        let title = label("", "PageTitle");
        title.set_xalign(0.5);
        let sub = label("", "RowLabel");
        sub.set_xalign(0.5);
        sub.set_wrap(true);
        sub.set_justify(gtk::Justification::Center);
        let btn = big_button(true);
        btn.set_halign(gtk::Align::Center);
        btn.set_margin_top(10);
        bx.append(&title);
        bx.append(&sub);
        bx.append(&btn);
        bx.append(&vspacer());
        // Rahmen-Box mit Innenabstand: die Karte selbst füllt den Bereich
        let card = vbox(0);
        card.add_css_class("Card");
        bx.remove_css_class("Card");
        card.append(&bx);
        stack.add_named(&card, Some("missing"));
        let g = Rc::new(Gate { stack, content, key: key.into(), title, sub, btn });
        let w: Weak<Studio> = Rc::downgrade(win);
        let k = key.to_string();
        g.btn.connect_clicked(move |_| {
            if let Some(win) = w.upgrade() {
                win.install_keys(&[&k]);
            }
        });
        g
    }

    pub fn ready(&self) -> bool {
        self.stack.visible_child_name().as_deref() == Some("content")
    }

    pub fn refresh(&self) {
        let ok = packages::installed(&self.key);
        self.stack.set_visible_child_name(if ok { "content" } else { "missing" });
        let name = t(&format!("pkg.{}", self.key));
        self.title.set_text(&tf("gate.missing", kw!("name" => name)));
        let pm = packages::manager();
        self.sub.set_text(&format!("{}  ·  {}", t(&format!("pkgd.{}", self.key)),
            packages::pkgs_for(&self.key, pm.unwrap_or("pacman")).join(" ")));
        set_button_text(&self.btn, &tf("gate.install", kw!("name" => name)));
        self.btn.set_sensitive(packages::available(&self.key, pm));
        if ok {
            self.content.refresh();
        }
    }
}

impl super::editor::External for Gate {
    fn widget(&self) -> gtk::Widget {
        self.stack.clone().upcast()
    }
    fn actions(&self) -> Vec<gtk::Button> {
        self.content.actions()
    }
    fn ready(&self) -> bool {
        Gate::ready(self)
    }
    fn refresh(&self) {
        Gate::refresh(self)
    }
}

/// Launcher-Tab: Ist der gewählte Launcher nicht installiert, einen wählen oder installieren (gleiche Kacheln).
pub struct LauncherGate {
    pub stack: gtk::Stack,
    page: Rc<EditorPage>,
    win: Weak<Studio>,
    pub installing: RefCell<String>,
    title: gtk::Label,
    cards: Vec<(String, gtk::Label, gtk::Button, gtk::ProgressBar)>,
}

impl LauncherGate {
    pub fn new(page: Rc<EditorPage>, win: &Rc<Studio>) -> Rc<LauncherGate> {
        let stack = gtk::Stack::new();
        stack.set_vexpand(true);
        stack.add_named(&page.root, Some("content"));
        let bx = vbox(16);
        super::widgets::margins(&bx, 8, 24, 8, 24);
        let title = label("", "PageTitle");
        title.set_wrap(true);
        bx.append(&title);
        let grid = flow_grid();
        grid.set_column_spacing(14);
        grid.set_row_spacing(14);
        bx.append(&grid);
        let mut cards = vec![];
        for app in launcher::APPS {
            let card = vbox(8);
            card.add_css_class("Card");
            card.set_size_request(230, 170);
            super::widgets::margins(&card, 16, 14, 16, 14);
            let name = label(&t(&format!("app.{app}")), "CardTitle");
            let desc = label("", "RowLabel");
            desc.set_wrap(true);
            desc.set_vexpand(true);
            desc.set_valign(gtk::Align::Start);
            desc.set_max_width_chars(24);
            let bar = super::widgets::busy_bar(-1);
            bar.add_css_class("thin");
            bar.set_visible(false);
            let btn = icon_button("download", "#ffffff", 18, "", "");
            btn.set_size_request(-1, 36);
            card.append(&name);
            card.append(&desc);
            card.append(&bar);
            card.append(&btn);
            let child = gtk::FlowBoxChild::new();
            child.set_child(Some(&card));
            child.set_focusable(false);
            grid.append(&child);
            cards.push((app.to_string(), desc, btn, bar));
        }
        bx.append(&vspacer());
        let sa = scroller(&bx);
        stack.add_named(&sa, Some("pick"));
        let g = Rc::new(LauncherGate { stack, page, win: Rc::downgrade(win), installing: RefCell::new(String::new()),
            title, cards });
        for (app, _, btn, _) in &g.cards {
            let w = Rc::downgrade(&g);
            let a = app.clone();
            btn.connect_clicked(move |_| {
                if let Some(g) = w.upgrade() {
                    g.click(&a);
                }
            });
        }
        g
    }

    fn click(self: &Rc<Self>, app: &str) {
        if packages::installed(app) {
            self.page.set("app", serde_json::json!(app));
            self.page.refresh_controls();
            self.installing.borrow_mut().clear();
            self.refresh();
        } else {
            *self.installing.borrow_mut() = app.to_string();
            self.refresh();
            if let Some(win) = self.win.upgrade() {
                win.install_keys(&[app]);
            }
        }
    }

    pub fn refresh(&self) {
        let app = self.page.state.borrow().s("app").to_string();
        self.stack.set_visible_child_name(if packages::installed(&app) { "content" } else { "pick" });
        self.title.set_text(&tf("gate.launcher", kw!("name" => t(&format!("app.{app}")))));
        let pm = packages::manager();
        for (a, desc, btn, bar) in &self.cards {
            let inst = packages::installed(a);
            let avail = a == "llauncher" || pm.map(|pm| packages::available(a, Some(pm))).unwrap_or(false);
            desc.set_text(&t(&format!("pkgd.{a}")));
            let img = btn.child().and_then(|c| c.first_child());
            if inst {
                set_button_text(btn, &t("gate.use"));
                if let Some(i) = &img {
                    i.set_visible(false);
                }
                btn.remove_css_class("Primary");
                btn.set_sensitive(true);
            } else {
                set_button_text(btn, &t("gate.install_short"));
                if let Some(i) = &img {
                    i.set_visible(true);
                }
                btn.add_css_class("Primary");
                btn.set_sensitive(avail);
                if !avail {
                    set_button_text(btn, &t("pkg.unavailable"));
                }
            }
            if inst && *self.installing.borrow() == *a {
                self.installing.borrow_mut().clear();
            }
            bar.set_visible(*self.installing.borrow() == *a && !inst);
        }
    }
}

use crate::jsonx::Get;

#[allow(dead_code)]
fn _u() {
    let _ = draw::Rgba(0.0, 0.0, 0.0, 0.0);
}
