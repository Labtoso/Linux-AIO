//! Bausteine für die Einstellungs-Seiten: Seite mit Titel, Karten mit Zeilen, gsettings-Steuerelemente.

use super::widgets::reset_button;
use super::win::win;
use crate::gnome::{self, Choices, Kind, Opt};
use crate::i18n::{t, tf};
use crate::usercmd;
use gtk::prelude::*;
use hyprstudio::kw;
use hyprstudio::ui::draw;
use hyprstudio::ui::theme;
use hyprstudio::ui::widgets::{cursor_hand, font_families, hbox, label, scroller, vbox, ComboItem, SearchCombo, Switch};
use serde_json::{json, Map, Value};
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;

type PathFn = Rc<dyn Fn(&Path)>;

/// Seite mit Titel oben (+ Platz für Knöpfe rechts) und scrollbarem Inhalt.
pub struct Page {
    pub root: gtk::Box,
    pub title_key: String,
    pub title: gtk::Label,
    pub b_reset: gtk::Button,
    pub chips: gtk::Box,
    pub tools: gtk::Box,
    pub body: gtk::Box,
    pub controls: RefCell<Vec<Rc<GControl>>>,
    cards: RefCell<Vec<Rc<Card>>>,
    pub has_reset: Cell<bool>,
    /// eigene gsettings-Schlüssel statt der Steuerelemente
    pub keys_override: RefCell<Option<Vec<(String, String)>>>,
    /// Seiten mit eigener Anzeige: statt nur die Steuerelemente neu zu laden
    pub on_refresh: RefCell<Option<Rc<dyn Fn()>>>,
    pub reset_extra: RefCell<Option<PathFn>>,
    pub undo_extra: RefCell<Option<PathFn>>,
}

impl Page {
    pub fn new(title_key: &str, scroll: bool) -> Rc<Page> {
        let root = vbox(12);
        let head = hbox(8);
        let title = label("", "APageTitle");
        head.append(&title);
        let b_reset = reset_button();
        b_reset.set_visible(false);
        head.append(&b_reset);
        let chips = hbox(6);
        chips.set_valign(gtk::Align::Center);
        head.append(&chips);
        let fill = hbox(0);
        fill.set_hexpand(true);
        head.append(&fill);
        let tools = hbox(8);
        tools.set_valign(gtk::Align::Center);
        head.append(&tools);
        root.append(&head);
        let body = vbox(12);
        hyprstudio::ui::widgets::margins(&body, 0, 2, 12, 12);
        if scroll {
            root.append(&scroller(&body));
        } else {
            body.set_vexpand(true);
            root.append(&body);
        }
        let p = Rc::new(Page { root, title_key: title_key.into(), title, b_reset, chips, tools, body,
            controls: RefCell::new(vec![]), cards: RefCell::new(vec![]), has_reset: Cell::new(false), keys_override: RefCell::new(None),
            on_refresh: RefCell::new(None), reset_extra: RefCell::new(None), undo_extra: RefCell::new(None) });
        let w = Rc::downgrade(&p);
        p.b_reset.connect_clicked(move |_| {
            if let Some(p) = w.upgrade() {
                p.reset_clicked();
            }
        });
        p
    }

    pub fn card(&self, title_key: &str) -> Rc<Card> {
        let c = Card::new(title_key);
        self.body.append(&c.root);
        self.cards.borrow_mut().push(c.clone());
        c
    }

    pub fn section(&self, title_key: &str) -> Rc<Card> {
        let c = Card::section(title_key);
        self.body.append(&c.root);
        self.cards.borrow_mut().push(c.clone());
        c
    }

    /// Karte merken, die woanders eingehängt wird (für Übersetzungen).
    pub fn track(&self, c: &Rc<Card>) {
        self.cards.borrow_mut().push(c.clone());
    }

    pub fn finish(&self) {
        // Zurücksetzen überall an derselben Stelle; ohne Rücksetzbares sichtbar, aber ausgegraut
        self.b_reset.set_sensitive(!self.reset_keys().is_empty() || self.has_reset.get());
        self.b_reset.set_visible(true);
    }

    pub fn reset_keys(&self) -> Vec<(String, String)> {
        if let Some(k) = self.keys_override.borrow().as_ref() {
            return k.clone();
        }
        self.controls.borrow().iter().map(|c| (c.opt.schema.to_string(), c.opt.key.to_string())).collect()
    }

    pub fn refresh(&self) {
        let f = self.on_refresh.borrow().clone();
        match f {
            Some(f) => f(),
            None => self.load_controls(),
        }
    }

    pub fn load_controls(&self) {
        for c in self.controls.borrow().iter() {
            c.load();
        }
    }

    fn reset_clicked(self: &Rc<Self>) {
        let me = self.clone();
        let name = self.title.text().to_string();
        hyprstudio::ui::dialogs::confirm(win().parent().as_ref(), &t("reset.title"),
            &tf("reset.ask", kw!("page" => name.clone())), Some(&t("reset.btn")), true, move |ok| {
                if !ok {
                    return;
                }
                let p = me;
                let w = win();
                let bak = usercmd::backup_dir();
                let keys = p.reset_keys();
                let mut values = Map::new();
                for (s, k) in &keys {
                    values.insert(format!("{s} {k}"), usercmd::gget(s, k, Value::Null));
                }
                let _ = std::fs::write(bak.join(format!("{}.json", p.title_key)),
                    crate::pyjson::dumps(&Value::Object(values.clone()), Some(1), false, false));
                w.log("");
                for (s, k) in &keys {
                    usercmd::greset(s, k);
                }
                if let Some(f) = p.reset_extra.borrow().clone() {
                    f(&bak);
                }
                p.refresh();
                let p2 = p.clone();
                let bak2 = bak.clone();
                w.offer_undo(&tf("reset.done", kw!("page" => name, "path" => bak.to_string_lossy())), Box::new(move || {
                    for (sk, v) in &values {
                        if let Some((s, k)) = sk.split_once(' ') {
                            if !v.is_null() {
                                usercmd::gset(s, k, v.clone());
                            }
                        }
                    }
                    if let Some(f) = p2.undo_extra.borrow().clone() {
                        f(&bak2);
                    }
                    p2.refresh();
                    win().set_status(&t("reset.undone"));
                }));
            });
    }

    pub fn retranslate(&self) {
        self.title.set_text(&t(&self.title_key));
        self.b_reset.set_tooltip_text(Some(&if self.b_reset.is_sensitive() { t("reset.tip") } else { t("reset.none") }));
        for c in self.cards.borrow().iter() {
            c.retranslate();
        }
        for c in self.controls.borrow().iter() {
            c.retranslate();
        }
        retranslate_buttons(&self.root);
    }

    /// gsettings-Zeilen in eine Karte.
    pub fn gsection(&self, card: &Card, specs: &[Opt]) -> Vec<Rc<GControl>> {
        let mut out = vec![];
        for o in specs {
            let c = GControl::new(*o);
            card.append(&c.row.root);
            self.controls.borrow_mut().push(c.clone());
            out.push(c);
        }
        out
    }
}

pub struct Card {
    pub root: gtk::Box,
    pub head: gtk::Label,
    title_key: String,
}

impl Card {
    pub fn new(title_key: &str) -> Rc<Card> {
        let root = vbox(4);
        root.add_css_class("ACard");
        let head = label("", "ACardTitle");
        head.set_xalign(0.0);
        head.set_visible(!title_key.is_empty());
        head.set_widget_name(&format!("ckey:{title_key}"));
        let c = Rc::new(Card { root, head, title_key: title_key.into() });
        c.root.append(&c.head);
        c
    }

    /// Wie eine Karte, aber ohne Rahmen – für Galerien (keine Karten in Karten).
    pub fn section(title_key: &str) -> Rc<Card> {
        let c = Card::new(title_key);
        c.root.remove_css_class("ACard");
        c.root.add_css_class("ASection");
        c
    }

    pub fn append<W: IsA<gtk::Widget>>(&self, w: &W) {
        self.root.append(w);
    }

    pub fn retranslate(&self) {
        if !self.title_key.is_empty() {
            self.head.set_text(&t(&self.title_key));
        }
    }
}

/// Eine Zeile: Text links, Steuerelement rechts; Tooltip nur auf dem Text.
pub struct Row {
    pub root: gtk::Box,
    pub label: gtk::Label,
    pub key: String,
}

impl Row {
    pub fn new<W: IsA<gtk::Widget>>(key: &str, control: &W) -> Rc<Row> {
        let root = hbox(8);
        root.set_margin_top(6);
        root.set_margin_bottom(6);
        let l = label("", "");
        l.set_wrap(true);
        l.set_xalign(0.0);
        l.set_hexpand(true);
        l.set_size_request(80, -1);
        l.set_widget_name(&format!("rkey:{key}"));
        root.append(&l);
        control.set_valign(gtk::Align::Center);
        root.append(control);
        Rc::new(Row { root, label: l, key: key.into() })
    }

    pub fn retranslate(&self) {
        self.label.set_text(&t(&self.key));
        let tk = format!("{}.tip", self.key);
        let tip = t(&tk);
        self.label.set_tooltip_text(if tip == tk { None } else { Some(&tip) });
    }
}

// ---------- Dateien sichern / zurückholen ----------

fn bak_name(f: &Path) -> String {
    f.to_string_lossy().trim_matches('/').replace('/', "__")
}

pub fn backup_files(files: &[PathBuf], bak: &Path) {
    for f in files {
        let src = usercmd::readable(f);
        if src.is_file() {
            let _ = std::fs::copy(&src, bak.join(bak_name(f)));
        }
    }
}

pub fn restore_files(files: &[PathBuf], bak: &Path) {
    for f in files {
        let b = bak.join(bak_name(f));
        if b.is_file() {
            if let Some(d) = f.parent() {
                let _ = std::fs::create_dir_all(d);
            }
            let _ = std::fs::copy(&b, f);
        }
    }
}

// ---------- Auswahl ----------

pub fn combo() -> Rc<SearchCombo> {
    let c = SearchCombo::new();
    c.button.set_size_request(150, -1);
    c
}

pub fn spin(lo: f64, hi: f64, step: f64, digits: u32, width: i32) -> gtk::SpinButton {
    let s = gtk::SpinButton::with_range(lo, hi, step);
    s.set_digits(digits);
    s.add_css_class("Spin");
    s.set_size_request(width, -1);
    s.set_update_policy(gtk::SpinButtonUpdatePolicy::IfValid);
    hyprstudio::ui::widgets::no_wheel(&s);
    s
}

// ---------- gsettings-Wert ↔ Steuerelement ----------

enum Ctl {
    Bool(Rc<Switch>),
    Choice(Rc<SearchCombo>),
    Num(gtk::SpinButton),
    Font(Rc<SearchCombo>, gtk::SpinButton),
}

/// Änderungen werden sofort gesetzt (wie GNOME Tweaks).
pub struct GControl {
    pub opt: Opt,
    ctl: Ctl,
    pub row: Rc<Row>,
    loading: Cell<bool>,
    pub available: Cell<bool>,
    pub on_changed: RefCell<Option<Rc<dyn Fn()>>>,
}

impl GControl {
    pub fn new(opt: Opt) -> Rc<GControl> {
        let (ctl, w): (Ctl, gtk::Widget) = match opt.kind {
            Kind::Bool => {
                let s = Switch::new();
                let a = s.area.clone().upcast();
                (Ctl::Bool(s), a)
            }
            Kind::Choice => {
                let c = combo();
                let b = c.button.clone().upcast();
                (Ctl::Choice(c), b)
            }
            Kind::Int | Kind::Double => {
                let (lo, hi, step) = match opt.choices {
                    Choices::Range(a, b, c) => (a, b, c),
                    _ => (0.0, 100.0, 1.0),
                };
                let s = spin(lo, hi, step, if opt.kind == Kind::Int { 0 } else { 2 }, 110);
                (Ctl::Num(s.clone()), s.upcast())
            }
            Kind::Font => {
                let row = hbox(6);
                let fam = combo();
                fam.button.set_size_request(220, -1);
                let size = spin(5.0, 72.0, 1.0, 0, 80);
                row.append(&fam.button);
                row.append(&size);
                (Ctl::Font(fam, size), row.upcast())
            }
        };
        let row = Row::new(opt.label, &w);
        let c = Rc::new(GControl { opt, ctl, row, loading: Cell::new(false), available: Cell::new(true),
            on_changed: RefCell::new(None) });
        let wc = Rc::downgrade(&c);
        match &c.ctl {
            Ctl::Bool(s) => s.connect(move |v| {
                if let Some(c) = wc.upgrade() {
                    c.set(json!(v));
                }
            }),
            Ctl::Choice(cb) => {
                let cb2 = Rc::downgrade(cb);
                cb.connect(move |_| {
                    if let (Some(c), Some(cb)) = (wc.upgrade(), cb2.upgrade()) {
                        c.set(json!(cb.current_data()));
                    }
                })
            }
            Ctl::Num(s) => {
                let int = c.opt.kind == Kind::Int;
                s.connect_value_changed(move |s| {
                    if let Some(c) = wc.upgrade() {
                        let v = s.value();
                        c.set(if int { json!(v.round() as i64) } else { json!((v * 100.0).round() / 100.0) });
                    }
                });
            }
            Ctl::Font(fam, size) => {
                let w2 = wc.clone();
                fam.connect(move |_| {
                    if let Some(c) = w2.upgrade() {
                        c.font_changed();
                    }
                });
                size.connect_value_changed(move |_| {
                    if let Some(c) = wc.upgrade() {
                        c.font_changed();
                    }
                });
            }
        }
        c
    }

    fn font_changed(&self) {
        if let Ctl::Font(fam, size) = &self.ctl {
            let f = fam.current_data();
            if f.is_empty() {
                return;
            }
            self.set(json!(gnome::join_font(&f, size.value())));
        }
    }

    fn set(&self, value: Value) {
        if self.loading.get() || value.is_null() {
            return;
        }
        usercmd::gset(self.opt.schema, self.opt.key, value);
        let f = self.on_changed.borrow().clone();
        if let Some(f) = f {
            f();
        }
    }

    pub fn load(&self) {
        let av = gnome::available(self.opt.schema, self.opt.key);
        self.available.set(av);
        self.row.root.set_sensitive(av);
        self.row.root.set_tooltip_text(if av { None } else { Some(t("na")) }.as_deref());
        let v = usercmd::gget(self.opt.schema, self.opt.key, Value::Null);
        self.loading.set(true);
        match &self.ctl {
            Ctl::Bool(s) => s.set_now(hyprstudio::jsonx::truthy(&v)),
            Ctl::Choice(cb) => {
                if cb.items.borrow().is_empty() {
                    self.fill_choice();
                }
                let vs = match &v {
                    Value::String(s) => s.clone(),
                    Value::Null => String::new(),
                    o => hyprstudio::jsonx::py_str(o),
                };
                let mut i = cb.find_data(&vs);
                if i < 0 && !v.is_null() {
                    let mut items: Vec<ComboItem> = cb.items.borrow().iter().map(clone_item).collect();
                    items.push(ComboItem::new(&vs, &vs));
                    i = items.len() as i32 - 1;
                    cb.set_items(items);
                }
                cb.set_current(i.max(0));
            }
            Ctl::Num(s) => {
                if let Some(x) = v.as_f64() {
                    s.set_value(x);
                }
            }
            Ctl::Font(fam, size) => {
                if let Value::String(s) = &v {
                    let (f, sz) = gnome::split_font(s);
                    if fam.items.borrow().is_empty() {
                        let items: Vec<ComboItem> = font_families(false).iter().map(|x| {
                            let mut it = ComboItem::new(x, x);
                            it.font = Some(x.clone());
                            it
                        }).collect();
                        fam.set_items(items);
                    }
                    let mut i = fam.find_data(&f);
                    if i < 0 {
                        let mut items: Vec<ComboItem> = fam.items.borrow().iter().map(clone_item).collect();
                        items.insert(0, ComboItem::new(&f, &f));
                        fam.set_items(items);
                        i = 0;
                    }
                    fam.set_current(i);
                    size.set_value(sz);
                }
            }
        }
        self.loading.set(false);
    }

    fn fill_choice(&self) {
        if let (Ctl::Choice(cb), Choices::List(vals)) = (&self.ctl, self.opt.choices) {
            let cur = cb.current_data();
            let items: Vec<ComboItem> = vals.iter().map(|val| {
                let key = format!("{}.{}", self.opt.label, val);
                let txt = t(&key);
                ComboItem::new(if txt == key { val } else { &txt }, val)
            }).collect();
            cb.set_items(items);
            let i = cb.find_data(&cur);
            cb.set_current(i.max(0));
        }
    }

    pub fn retranslate(&self) {
        self.row.retranslate();
        if matches!(self.ctl, Ctl::Choice(_)) {
            self.loading.set(true);
            self.fill_choice();
            self.loading.set(false);
        }
    }
}

fn clone_item(x: &ComboItem) -> ComboItem {
    ComboItem { text: x.text.clone(), data: x.data.clone(), font: x.font.clone(), tooltip: x.tooltip.clone(),
        muted: x.muted }
}

// ---------- Knöpfe ----------

/// Knopf mit Text-Schlüssel (wird bei Sprachwechsel über `retranslate_buttons` übersetzt).
pub fn tbtn(text_key: &str, primary: bool) -> gtk::Button {
    let b = gtk::Button::with_label(&t(text_key));
    b.set_widget_name(&format!("tkey:{text_key}"));
    if primary {
        b.add_css_class("Primary");
    }
    cursor_hand(&b);
    b.set_size_request(-1, 34);
    b
}

pub fn retranslate_buttons<W: IsA<gtk::Widget>>(w: &W) {
    let mut stack: Vec<gtk::Widget> = vec![w.clone().upcast()];
    while let Some(x) = stack.pop() {
        let n = x.widget_name();
        if let Some(k) = n.strip_prefix("tkey:") {
            if let Some(b) = x.downcast_ref::<gtk::Button>() {
                hyprstudio::ui::widgets::set_button_text(b, &t(k));
            } else if let Some(b) = x.downcast_ref::<gtk::ToggleButton>() {
                b.set_label(&t(k));
            } else if let Some(l) = x.downcast_ref::<gtk::Label>() {
                l.set_text(&t(k));
            }
        }
        let mut c = x.first_child();
        while let Some(ch) = c {
            c = ch.next_sibling();
            stack.push(ch);
        }
    }
}

/// Kleiner „Deinstallieren“-Knopf neben einer installierten Erweiterung.
pub fn remove_button() -> gtk::Button {
    let b = gtk::Button::new();
    b.add_css_class("ResetBtn");
    b.set_child(Some(&draw::icon_image("close", theme::MUTED, 14)));
    b.set_size_request(30, 30);
    b.set_valign(gtk::Align::Center);
    cursor_hand(&b);
    b.set_tooltip_text(Some(&t("ext.uninstall")));
    b
}

/// Kleiner Download-Pfeil neben „fehlt“: ein Klick installiert das Fehlende.
pub fn missing_button() -> gtk::Button {
    let b = gtk::Button::new();
    b.add_css_class("Missing");
    b.set_child(Some(&draw::icon_image("download", theme::TEXT, 18)));
    b.set_size_request(30, 30);
    b.set_valign(gtk::Align::Center);
    cursor_hand(&b);
    b.set_tooltip_text(Some(&t("miss.install")));
    b
}

/// Kachel-Raster, das beim Verkleinern wieder umbricht.
pub fn flow_grid() -> gtk::FlowBox {
    let fb = hyprstudio::ui::widgets::flow_grid();
    fb.set_halign(gtk::Align::Fill);
    fb.set_hexpand(true);
    fb
}
