//! Globale Suche im Kopf: Seiten, Einstellungs-Zeilen, Desktops und Themes. Der Index entsteht beim Start im
//! Hintergrund (Dateisystem), das Filtern ist sofort (fertige Liste).

use super::settings_page::sections;
use super::widgets::search_entry;
use super::win::{tabs, win, AioPage};
use crate::i18n::{self, t};
use crate::{catalog, desktops, looks, plymouth, sddm};
use gtk::prelude::*;
use gtk::{gdk, glib};
use hyprstudio::ui::shell;
use hyprstudio::ui::widgets::{hbox, label};
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;
use std::time::Duration;

/// i18n-Präfix → (Desktop, Bereich[, Unterseite]); der erste Treffer gilt.
const PLACES: &[(&[&str], (&str, &str, &str))] = &[
    (&["ala."], ("*", "advanced", "alacritty")),
    (&["adv.yay"], ("*", "advanced", "yay")),
    (&["gn.look", "gn.gtk", "gn.shell", "look.scheme", "look.accent", "look.anim"], ("gnome", "look", "")),
    (&["font.ui", "font.doc", "font.mono", "font.title", "font.hint", "font.aa", "font.scale"], ("gnome", "fonts", "")),
    (&["win.", "gn.windows", "gn.corners", "gn.files"], ("gnome", "windows", "")),
    (&["gn.titlebar", "gn.title", "gn.btn", "gn.left", "gn.right", "gn.sup"], ("gnome", "titlebar", "")),
    (&["gn.topbar", "gn.top.", "top.", "bar.", "pw.", "qs."], ("gnome", "topbar", "")),
    (&["in.", "gn.input"], ("gnome", "input", "")),
    (&["gn.autostart", "gn.auto.add"], ("gnome", "autostart", "")),
    (&["gn.login"], ("gnome", "login", "")),
    (&["pl."], ("plasma", "plasma", "")),
];

/// Werte und Meldungen, keine eigenen Einstellungen.
const SKIP: &[&str] = &["win.dbl.", "win.mid.", "win.right.", "win.focus.", "win.modifier.", "look.accent.", "look.scheme.",
    "in.accel.", "in.overlay.", "font.hint.", "font.aa.", "top.format.", "font.cat.", "font.f",
    "gn.login.nogdm", "gn.login.banner.ph", "adv.yay.ok", "adv.yay.no", "adv.yay.arch", "adv.yay.s",
    "adv.yay.done", "adv.kitty.missing", "adv.alacritty.missing", "ala.applied", "gn.btns.note",
    "gn.btns.restart", "gn.btn.noclose", "gn.sup."];

fn place(key: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if key.ends_with(".tip") || SKIP.iter().any(|s| key.starts_with(s)) {
        return None;
    }
    PLACES.iter().find(|(pre, _)| pre.iter().any(|p| key.starts_with(p))).map(|(_, p)| *p)
}

/// Alles, was die Platte anfasst (läuft im Hintergrund).
#[derive(Default, Clone)]
pub struct Data {
    de: Vec<(String, String, bool)>,
    sddm: Vec<String>,
    ply: Vec<String>,
    cursor: Vec<String>,
    icon: Vec<String>,
}

pub fn scan() -> Data {
    let mut out = Data::default();
    for d in catalog::DESKTOPS.iter() {
        let ok = desktops::get(&d.id).map(|e| desktops::installed(&e)).unwrap_or(false);
        out.de.push((d.id.clone(), d.name.clone(), ok));
    }
    out.sddm = sddm::list_themes().into_iter().map(|t| t.name).collect();
    out.ply = plymouth::list_themes().into_iter().map(|t| t.name).collect();
    for kind in ["cursor", "icon"] {
        let mut v: Vec<String> = looks::list_installed(kind).into_iter().map(|(n, _)| n).collect();
        v.sort();
        if kind == "cursor" {
            out.cursor = v;
        } else {
            out.icon = v;
        }
    }
    out
}

#[derive(Clone, Debug)]
pub enum Target {
    Tab(String),
    De(String),
    /// Desktop, Bereich, Zeilen-Schlüssel, angezeigter Text
    Sec(String, String, String, String),
    /// Unterseite, Zeilen-Schlüssel, angezeigter Text
    Adv(String, String, String),
    Theme(String, String),
}

pub struct Hit {
    text: String,
    place: String,
    target: Target,
    hay: String,
}

pub fn entries(data: &Data, current_de: &str) -> Vec<Hit> {
    let names: std::collections::HashMap<&str, &str> = data.de.iter().map(|(d, n, _)| (d.as_str(), n.as_str())).collect();
    let inst: Vec<&str> = data.de.iter().filter(|(_, _, ok)| *ok).map(|(d, _, _)| d.as_str()).collect();
    let set = t("tab.settings");
    let mut res: Vec<(String, String, Target)> = tabs().iter().map(|k| (t(&format!("tab.{k}")), String::new(), Target::Tab(k.to_string()))).collect();
    for (d, n, _) in &data.de {
        res.push((n.clone(), t("tab.de"), Target::De(d.clone())));
    }
    for de in &inst {
        for key in sections(de) {
            res.push((t(&format!("sec.{key}")), format!("{set} › {}", names.get(de).unwrap_or(de)),
                Target::Sec(de.to_string(), key.to_string(), String::new(), String::new())));
        }
    }
    let adv_de = if inst.contains(&current_de) { current_de } else { inst.first().copied().unwrap_or("") };
    let adv = format!("{set} › {}", t("sec.advanced"));
    for sub in ["kitty", "alacritty", "yay"] {
        res.push((t(&format!("adv.{sub}")), adv.clone(), Target::Adv(sub.into(), String::new(), String::new())));
    }
    // Kitty-/Alacritty-Optionen (gleiche Beschriftung in beiden)
    {
        let text = hyprstudio::i18n::TEXT.text.read().unwrap();
        let mut keys: Vec<(&String, &String)> = text.get(&i18n::lang()).map(|m| m.iter().collect()).unwrap_or_default();
        keys.sort();
        for (key, txt) in keys {
            if key.starts_with("k.") && !key.starts_with("k.missing") && !key.starts_with("k.reload") && !txt.contains('{')
                && txt.chars().count() <= 48 {
                for sub in ["kitty", "alacritty"] {
                    res.push((txt.clone(), format!("{adv} › {}", t(&format!("adv.{sub}"))),
                        Target::Adv(sub.into(), key.clone(), txt.clone())));
                }
            }
        }
    }
    {
        let text = i18n::TEXT.text.read().unwrap();
        let mut keys: Vec<(&String, &String)> = text.get(&i18n::lang()).map(|m| m.iter().collect()).unwrap_or_default();
        keys.sort();
        for (key, txt) in keys {
            let Some(p) = place(key) else { continue };
            if txt.contains('{') || txt.chars().count() > 48 {
                continue;
            }
            if p.0 == "*" {
                res.push((txt.clone(), format!("{adv} › {}", t(&format!("adv.{}", p.2))), Target::Adv(p.2.into(), key.clone(), txt.clone())));
                continue;
            }
            if !inst.contains(&p.0) {
                continue;
            }
            let where_ = format!("{set} › {} › {}", names.get(p.0).unwrap_or(&p.0), t(&format!("sec.{}", p.1)));
            res.push((txt.clone(), where_, Target::Sec(p.0.into(), p.1.into(), key.clone(), txt.clone())));
        }
    }
    for (tab, list) in [("sddm", &data.sddm), ("ply", &data.ply)] {
        let lbl = t(&format!("tab.{tab}"));
        for n in list {
            res.push((n.clone(), lbl.clone(), Target::Theme(tab.into(), n.clone())));
        }
    }
    let look_de = if inst.contains(&current_de) { current_de } else { adv_de };
    if !look_de.is_empty() {
        for (kind, list) in [("cursor", &data.cursor), ("icon", &data.icon)] {
            for n in list {
                res.push((n.clone(), format!("{set} › {}", t(&format!("sec.{kind}"))),
                    Target::Sec(look_de.into(), kind.into(), String::new(), String::new())));
            }
        }
    }
    // doppelte Treffer (gleicher Text, gleicher Ort) nur einmal
    let mut seen = HashSet::new();
    let mut out = vec![];
    for (text, place, target) in res {
        let k = (text.to_lowercase(), format!("{:?}", match &target {
            Target::Sec(a, b, c, _) => format!("sec{a}{b}{c}"),
            Target::Adv(a, b, _) => format!("adv{a}{b}"),
            o => format!("{o:?}"),
        }));
        if seen.insert(k) {
            let hay = format!("{text} {place}").to_lowercase();
            out.push(Hit { text, place, target, hay });
        }
    }
    out
}

const MAX_ROWS: usize = 12;

pub struct SearchBox {
    pub entry: gtk::Entry,
    pop: gtk::Popover,
    list: gtk::ListBox,
    data: RefCell<Option<Data>>,
    hits: RefCell<Vec<Hit>>,
    shown: RefCell<Vec<Target>>,
    quiet: Cell<bool>,
}

impl SearchBox {
    pub fn new() -> Rc<SearchBox> {
        let entry = search_entry(240);
        let list = gtk::ListBox::new();
        list.add_css_class("SearchPop");
        list.set_selection_mode(gtk::SelectionMode::Single);
        list.set_activate_on_single_click(true);
        let sc = gtk::ScrolledWindow::new();
        sc.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        sc.set_propagate_natural_height(true);
        sc.set_max_content_height(MAX_ROWS as i32 * 32);
        sc.set_child(Some(&list));
        let pop = gtk::Popover::new();
        pop.set_has_arrow(false);
        pop.set_autohide(false);
        pop.set_can_focus(false);
        pop.set_position(gtk::PositionType::Bottom);
        pop.set_halign(gtk::Align::Start);
        pop.set_size_request(420, -1);
        pop.set_child(Some(&sc));
        pop.set_parent(&entry);
        let s = Rc::new(SearchBox { entry, pop, list, data: RefCell::new(None), hits: RefCell::new(vec![]),
            shown: RefCell::new(vec![]), quiet: Cell::new(false) });

        let me = Rc::downgrade(&s);
        s.entry.connect_changed(move |_| {
            if let Some(s) = me.upgrade() {
                if !s.quiet.get() {
                    s.filter();
                }
            }
        });
        let me = Rc::downgrade(&s);
        s.entry.connect_activate(move |_| {
            if let Some(s) = me.upgrade() {
                let i = s.list.selected_row().map(|r| r.index()).unwrap_or(0);
                s.go_index(i);
            }
        });
        let me = Rc::downgrade(&s);
        s.list.connect_row_activated(move |_, row| {
            if let Some(s) = me.upgrade() {
                s.go_index(row.index());
            }
        });
        // Pfeiltasten wählen in der Liste, Esc schließt
        let keys = gtk::EventControllerKey::new();
        let me = Rc::downgrade(&s);
        keys.connect_key_pressed(move |_, key, _, _| {
            let Some(s) = me.upgrade() else { return glib::Propagation::Proceed };
            if !s.pop.is_visible() {
                return glib::Propagation::Proceed;
            }
            let n = s.shown.borrow().len() as i32;
            let cur = s.list.selected_row().map(|r| r.index()).unwrap_or(-1);
            let next = match key {
                gdk::Key::Down => (cur + 1).min(n - 1),
                gdk::Key::Up => (cur - 1).max(0),
                gdk::Key::Escape => {
                    s.pop.popdown();
                    return glib::Propagation::Stop;
                }
                _ => return glib::Propagation::Proceed,
            };
            if let Some(r) = s.list.row_at_index(next) {
                s.list.select_row(Some(&r));
                r.grab_focus();
                s.entry.grab_focus_without_selecting();
            }
            glib::Propagation::Stop
        });
        s.entry.add_controller(keys);
        let focus = gtk::EventControllerFocus::new();
        let me = Rc::downgrade(&s);
        focus.connect_leave(move |_| {
            let me = me.clone();
            // Klick in die Liste erst ankommen lassen
            glib::timeout_add_local_once(Duration::from_millis(200), move || {
                if let Some(s) = me.upgrade() {
                    if !s.entry.has_focus() {
                        s.pop.popdown();
                    }
                }
            });
        });
        s.entry.add_controller(focus);

        let me = Rc::downgrade(&s);
        shell::run_bg(scan, move |data| {
            if let Some(s) = me.upgrade() {
                *s.data.borrow_mut() = Some(data);
                s.rebuild();
            }
        });
        s
    }

    fn rebuild(&self) {
        let Some(data) = self.data.borrow().clone() else { return };
        let cur = desktops::current().unwrap_or_default();
        *self.hits.borrow_mut() = entries(&data, if cur.is_empty() { "gnome" } else { &cur });
        if self.pop.is_visible() {
            self.filter();
        }
    }

    pub fn retranslate(&self) {
        self.entry.set_placeholder_text(Some(&t("search.all")));
        self.rebuild();
    }

    fn filter(&self) {
        while let Some(c) = self.list.first_child() {
            self.list.remove(&c);
        }
        self.shown.borrow_mut().clear();
        let q = self.entry.text().trim().to_lowercase();
        if q.is_empty() {
            self.pop.popdown();
            return;
        }
        let hits = self.hits.borrow();
        for h in hits.iter().filter(|h| h.hay.contains(&q)).take(60) {
            let row = gtk::ListBoxRow::new();
            let b = hbox(0);
            let l = label(&h.text, "");
            l.set_xalign(0.0);
            l.set_ellipsize(gtk::pango::EllipsizeMode::End);
            b.append(&l);
            if !h.place.is_empty() {
                let w = label(&format!("    ·  {}", h.place), "where");
                w.set_xalign(0.0);
                w.set_ellipsize(gtk::pango::EllipsizeMode::End);
                w.set_hexpand(true);
                b.append(&w);
            }
            row.set_child(Some(&b));
            row.set_focusable(false);
            self.list.append(&row);
            self.shown.borrow_mut().push(h.target.clone());
        }
        if self.shown.borrow().is_empty() {
            self.pop.popdown();
            return;
        }
        if let Some(r) = self.list.row_at_index(0) {
            self.list.select_row(Some(&r));
        }
        self.pop.popup();
    }

    /// Testhilfe: aktuelle Treffer auf stderr.
    pub fn debug_dump(&self) {
        let hits = self.hits.borrow();
        let q = self.entry.text().to_lowercase();
        for h in hits.iter().filter(|h| h.hay.contains(&q)).take(MAX_ROWS) {
            eprintln!("search: {}  ·  {}", h.text, h.place);
        }
    }

    fn go_index(&self, i: i32) {
        let Some(target) = self.shown.borrow().get(i.max(0) as usize).cloned() else { return };
        self.pop.popdown();
        self.quiet.set(true);
        self.entry.set_text("");
        self.quiet.set(false);
        glib::idle_add_local_once(move || go(&target));
    }
}

pub fn go(target: &Target) {
    let w = win();
    match target {
        Target::Tab(t) => w.set_tab(t),
        Target::De(id) => {
            w.set_tab("de");
            if let Some(p) = w.de_page.get() {
                p.select(id);
            }
        }
        Target::Theme(tab, name) => {
            w.set_tab(tab);
            let p = if tab == "sddm" { w.sddm_page.get() } else { w.ply_page.get() };
            if let Some(p) = p {
                p.set_search(name);
            }
        }
        Target::Adv(sub, key, text) => {
            w.set_tab("settings");
            let Some(sp) = w.settings_page.get() else { return };
            if let Some(pg) = sp.select_adv(sub) {
                if !key.is_empty() {
                    reveal_later(pg, key.clone(), text.clone());
                }
            }
        }
        Target::Sec(de, key, rowkey, text) => {
            w.set_tab("settings");
            let Some(sp) = w.settings_page.get() else { return };
            sp.select(de);
            if let Some(pg) = sp.show_section(key, None) {
                if !rowkey.is_empty() {
                    reveal_later(pg, rowkey.clone(), text.clone());
                }
            }
        }
    }
}

fn reveal_later(page: Rc<dyn AioPage>, key: String, text: String) {
    glib::timeout_add_local_once(Duration::from_millis(60), move || reveal(&page.widget(), &key, &text));
}

fn find(w: &gtk::Widget, test: &dyn Fn(&gtk::Widget) -> bool) -> Option<gtk::Widget> {
    if test(w) {
        return Some(w.clone());
    }
    let mut c = w.first_child();
    while let Some(ch) = c {
        if let Some(f) = find(&ch, test) {
            return Some(f);
        }
        c = ch.next_sibling();
    }
    None
}

/// Zeile in Sicht scrollen und kurz aufleuchten lassen.
fn reveal(root: &gtk::Widget, key: &str, text: &str) {
    let (rk, ck) = (format!("rkey:{key}"), format!("ckey:{key}"));
    let mut hit = find(root, &|w| w.widget_name() == rk.as_str())
        .or_else(|| find(root, &|w| w.widget_name() == ck.as_str()));
    if hit.is_none() && !text.is_empty() {
        // Hypr-Studio-Karten: Beschriftung mit dem Text
        hit = find(root, &|w| w.downcast_ref::<gtk::Label>().map(|l| l.text() == text).unwrap_or(false));
    }
    let Some(w) = hit else { return };
    // passende Unterseite (gtk::Stack) zeigen
    let mut cur = Some(w.clone());
    while let Some(c) = cur {
        let parent = c.parent();
        if let Some(st) = parent.as_ref().and_then(|p| p.downcast_ref::<gtk::Stack>()) {
            st.set_visible_child(&c);
        }
        if &c == root {
            break;
        }
        cur = parent;
    }
    let w2 = w.clone();
    glib::timeout_add_local_once(Duration::from_millis(30), move || {
        let mut p = w2.parent();
        while let Some(x) = p {
            if let Some(sc) = x.downcast_ref::<gtk::ScrolledWindow>() {
                if let Some(child) = sc.child() {
                    if let Some(pt) = w2.compute_point(&child, &gtk::graphene::Point::new(0.0, 0.0)) {
                        let adj = sc.vadjustment();
                        let y = (pt.y() as f64 - 60.0).clamp(adj.lower(), (adj.upper() - adj.page_size()).max(adj.lower()));
                        adj.set_value(y);
                    }
                }
                break;
            }
            p = x.parent();
        }
    });
    w.add_css_class("Flash");
    glib::timeout_add_local_once(Duration::from_millis(1400), move || w.remove_css_class("Flash"));
}
