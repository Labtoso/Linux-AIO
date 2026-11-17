//! Eigene Dialoge im App-Look (dunkel, rund, ohne System-Rahmen) statt System-/KDE-Dialogen:
//! Text eingeben, Rückfrage, Datei/Ordner wählen, speichern, Farbe wählen.
#![allow(deprecated)]

use super::draw::{self, Rgba};
use super::theme;
use super::widgets::{button, cursor_hand, hbox, label, spacer, vbox};
use crate::i18n::t;
use gtk::prelude::*;
use gtk::{gdk, glib};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

/// Rahmenloses rundes Fenster mit Titel; an der Titelzeile verschiebbar.
pub struct Frame {
    pub win: gtk::Window,
    pub body: gtk::Box,
    pub title: gtk::Label,
    done: RefCell<Option<Box<dyn FnOnce(bool)>>>,
}

impl Frame {
    pub fn new(parent: Option<&gtk::Window>, title: &str, width: i32) -> Rc<Frame> {
        let win = gtk::Window::new();
        win.set_title(Some(title));
        win.set_modal(true);
        win.set_decorated(false);
        win.set_resizable(false);
        win.add_css_class("hs-window");
        if let Some(p) = parent {
            win.set_transient_for(Some(p));
        }
        let frame = vbox(12);
        frame.add_css_class("DialogFrame");
        frame.set_size_request(width, -1);
        let handle = gtk::WindowHandle::new();
        let tl = label(title, "PanelTitle");
        tl.set_wrap(true);
        handle.set_child(Some(&tl));
        frame.append(&handle);
        let body = vbox(12);
        frame.append(&body);
        frame.set_margin_start(0);
        handle.set_margin_start(24);
        handle.set_margin_end(24);
        handle.set_margin_top(20);
        body.set_margin_start(24);
        body.set_margin_end(24);
        body.set_margin_bottom(20);
        win.set_child(Some(&frame));
        let f = Rc::new(Frame { win, body, title: tl, done: RefCell::new(None) });
        let keys = gtk::EventControllerKey::new();
        let w = Rc::downgrade(&f);
        keys.connect_key_pressed(move |_, k, _, _| {
            if k == gdk::Key::Escape {
                if let Some(f) = w.upgrade() {
                    f.finish(false);
                }
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        f.win.add_controller(keys);
        let w = Rc::downgrade(&f);
        f.win.connect_close_request(move |_| {
            if let Some(f) = w.upgrade() {
                if let Some(cb) = f.done.borrow_mut().take() {
                    cb(false);
                }
            }
            glib::Propagation::Proceed
        });
        f
    }

    /// Knopfzeile unten rechts (Abbrechen + OK).
    pub fn buttons(self: &Rc<Self>, ok_text: &str, primary: &str, cancel: bool) -> gtk::Button {
        let row = hbox(8);
        row.append(&spacer());
        if cancel {
            let c = button(&t("cancel"), "");
            c.set_size_request(120, -1);
            let w = Rc::downgrade(self);
            c.connect_clicked(move |_| {
                if let Some(f) = w.upgrade() {
                    f.finish(false);
                }
            });
            row.append(&c);
        }
        let ok = button(ok_text, primary);
        ok.set_size_request(120, -1);
        let w = Rc::downgrade(self);
        ok.connect_clicked(move |_| {
            if let Some(f) = w.upgrade() {
                f.finish(true);
            }
        });
        row.append(&ok);
        self.body.append(&row);
        self.win.set_default_widget(Some(&ok));
        ok
    }

    pub fn finish(&self, ok: bool) {
        let cb = self.done.borrow_mut().take();
        self.win.close();
        if let Some(cb) = cb {
            cb(ok);
        }
    }

    pub fn run(self: &Rc<Self>, done: impl FnOnce(bool) + 'static) {
        // Frame lebt, bis der Dialog geschlossen ist
        let keep = self.clone();
        *self.done.borrow_mut() = Some(Box::new(move |ok| {
            let _k = &keep;
            done(ok);
        }));
        self.win.present();
    }
}

pub fn ask_text(parent: Option<&gtk::Window>, title: &str, lbl: &str, text: &str, done: impl FnOnce(Option<String>) + 'static) {
    let d = Frame::new(parent, title, 460);
    if !lbl.is_empty() {
        let l = label(lbl, "RowLabel");
        l.set_wrap(true);
        d.body.append(&l);
    }
    let le = gtk::Entry::new();
    le.set_text(text);
    le.set_size_request(-1, 38);
    d.body.append(&le);
    d.buttons(&t("ok"), "Primary", true);
    let w = Rc::downgrade(&d);
    le.connect_activate(move |_| {
        if let Some(d) = w.upgrade() {
            d.finish(true);
        }
    });
    let le2 = le.clone();
    d.run(move |ok| done(ok.then(|| le2.text().to_string())));
    le.grab_focus();
    le.select_region(0, -1);
}

pub fn confirm(parent: Option<&gtk::Window>, title: &str, text: &str, ok_text: Option<&str>, danger: bool,
    done: impl FnOnce(bool) + 'static) {
    let d = Frame::new(parent, title, 460);
    let l = label(text, "");
    l.set_wrap(true);
    l.set_max_width_chars(60);
    d.body.append(&l);
    let yes = t("yes");
    d.buttons(ok_text.unwrap_or(&yes), if danger { "DangerFill" } else { "Primary" }, true);
    d.run(done);
}

/// Nur ein Hinweis mit OK.
pub fn info(parent: Option<&gtk::Window>, title: &str, text: &str) {
    let d = Frame::new(parent, title, 460);
    let l = label(text, "");
    l.set_wrap(true);
    d.body.append(&l);
    d.buttons(&t("ok"), "Primary", false);
    d.run(|_| {});
}

#[derive(Clone, Copy, PartialEq)]
pub enum FileMode {
    Open,
    Dir,
    Save,
}

/// Dateibrowser eingebettet im eigenen Fenster (kein Portal-Dialog im hellen System-Look).
pub fn file(parent: Option<&gtk::Window>, title: &str, start: &str, filter: &[(&str, &[&str])], mode: FileMode,
    done: impl FnOnce(Option<PathBuf>) + 'static) {
    let d = Frame::new(parent, title, 900);
    d.win.set_default_size(920, 600);
    let action = match mode {
        FileMode::Open => gtk::FileChooserAction::Open,
        FileMode::Dir => gtk::FileChooserAction::SelectFolder,
        FileMode::Save => gtk::FileChooserAction::Save,
    };
    let fc = gtk::FileChooserWidget::new(action);
    fc.set_size_request(860, 480);
    fc.set_vexpand(true);
    for (name, pats) in filter {
        let f = gtk::FileFilter::new();
        f.set_name(Some(name));
        for p in *pats {
            f.add_pattern(p);
        }
        fc.add_filter(&f);
    }
    let sp = crate::util::expanduser(start);
    if mode == FileMode::Save && !sp.is_dir() {
        if let Some(parent_dir) = sp.parent() {
            let _ = fc.set_current_folder(Some(&gtk::gio::File::for_path(parent_dir)));
        }
        if let Some(n) = sp.file_name() {
            fc.set_current_name(&n.to_string_lossy());
        }
    } else if sp.is_dir() {
        let _ = fc.set_current_folder(Some(&gtk::gio::File::for_path(&sp)));
    } else if sp.is_file() {
        let _ = fc.set_file(&gtk::gio::File::for_path(&sp));
    }
    d.body.append(&fc);
    let ok_text = match mode {
        FileMode::Save => t("save"),
        _ => t("open"),
    };
    let ok_text = if ok_text == "save" || ok_text == "open" { t("ok") } else { ok_text };
    d.buttons(&ok_text, "Primary", true);
    let fc2 = fc.clone();
    let w = Rc::downgrade(&d);
    fc.connect_local("file-activated", false, move |_| {
        if let Some(d) = w.upgrade() {
            d.finish(true);
        }
        None
    });
    d.run(move |ok| {
        if !ok {
            return done(None);
        }
        let mut p = fc2.file().and_then(|f| f.path());
        if mode == FileMode::Dir && p.is_none() {
            p = fc2.current_folder().and_then(|f| f.path());
        }
        done(p);
    });
}

pub fn open_file(parent: Option<&gtk::Window>, title: &str, start: &str, filter: &[(&str, &[&str])],
    done: impl FnOnce(Option<PathBuf>) + 'static) {
    file(parent, title, start, filter, FileMode::Open, done)
}

pub fn open_dir(parent: Option<&gtk::Window>, title: &str, start: &str, done: impl FnOnce(Option<PathBuf>) + 'static) {
    file(parent, title, start, &[], FileMode::Dir, done)
}

pub fn save_file(parent: Option<&gtk::Window>, title: &str, start: &str, filter: &[(&str, &[&str])],
    done: impl FnOnce(Option<PathBuf>) + 'static) {
    file(parent, title, start, filter, FileMode::Save, done)
}

fn to_gdk(c: &str) -> gdk::RGBA {
    let r = Rgba::hex(c);
    gdk::RGBA::new(r.0 as f32, r.1 as f32, r.2 as f32, r.3 as f32)
}

fn from_gdk(c: &gdk::RGBA) -> String {
    Rgba(c.red() as f64, c.green() as f64, c.blue() as f64, c.alpha() as f64).to_hex()
}

/// Farbwähler mit Alpha und Live-Vorschau; `done(None)` = abgebrochen.
pub fn pick_color(parent: Option<&gtk::Window>, color: &str, title: &str, live: Box<dyn Fn(String)>,
    done: Box<dyn FnOnce(Option<String>)>) {
    let d = Frame::new(parent, title, 560);
    let cc = gtk::ColorChooserWidget::new();
    cc.set_use_alpha(true);
    cc.set_rgba(&to_gdk(color));
    cc.set_property("show-editor", true);
    let live = Rc::new(live);
    let l2 = live.clone();
    cc.connect_rgba_notify(move |c| l2(from_gdk(&c.rgba())));
    // Hex-Feld wie im Qt-Farbdialog: Farbe direkt eintippen (auch mit Alpha)
    let hexrow = hbox(8);
    let hl = label("HTML", "RowLabel");
    let he = gtk::Entry::new();
    he.set_text(&crate::model::norm(color));
    he.set_size_request(140, -1);
    let cc2 = cc.clone();
    he.connect_activate(move |e| {
        let txt = e.text().to_string();
        let ok = txt.trim_start_matches('#').len() >= 6 && txt.trim_start_matches('#').chars().all(|c| c.is_ascii_hexdigit());
        if ok {
            cc2.set_rgba(&to_gdk(&txt));
        }
    });
    let he2 = he.clone();
    cc.connect_rgba_notify(move |c| {
        if !he2.has_focus() {
            he2.set_text(&from_gdk(&c.rgba()));
        }
    });
    hexrow.append(&hl);
    hexrow.append(&he);
    cursor_hand(&cc);
    d.body.append(&cc);
    d.body.append(&hexrow);
    d.buttons(&t("ok"), "Primary", true);
    let cc3 = cc.clone();
    d.run(move |ok| done(ok.then(|| from_gdk(&cc3.rgba()))));
}

/// Eintrag eines Popup-Menüs (wie QMenu): Aktion, Haken, Trennlinie, Abschnittstitel.
pub enum MenuEntry {
    Item(String),
    Check(String, bool),
    Sep,
    Header(String),
}

/// Auswahl aus einer kurzen Liste (Kontextmenü-Ersatz): Einträge als Knöpfe in einem Popover.
pub fn menu<W: IsA<gtk::Widget>>(anchor: &W, x: f64, y: f64, items: Vec<(String, bool)>, done: impl Fn(usize) + 'static) {
    let mut entries = Vec::new();
    for (text, sep) in items {
        if sep {
            entries.push(MenuEntry::Sep);
        }
        entries.push(MenuEntry::Item(text));
    }
    popup_menu(anchor, Some((x, y)), entries, done);
}

/// Popup-Menü an `anchor` (an Punkt `at` oder unter dem Widget). `done(i)` bekommt den Index unter allen
/// anklickbaren Einträgen (Item/Check, in Reihenfolge).
pub fn popup_menu<W: IsA<gtk::Widget>>(anchor: &W, at: Option<(f64, f64)>, entries: Vec<MenuEntry>,
                                       done: impl Fn(usize) + 'static) {
    let pop = gtk::Popover::new();
    pop.set_has_arrow(false);
    pop.add_css_class("Menu");
    pop.set_parent(anchor);
    if let Some((x, y)) = at {
        pop.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
    }
    pop.set_position(gtk::PositionType::Bottom);
    let bx = vbox(0);
    let done = Rc::new(done);
    let mut idx = 0usize;
    for e in entries {
        let (text, check) = match e {
            MenuEntry::Sep => {
                let s = gtk::Separator::new(gtk::Orientation::Horizontal);
                s.add_css_class("MenuSep");
                bx.append(&s);
                continue;
            }
            MenuEntry::Header(h) => {
                let l = label(&h, "MenuHeader");
                bx.append(&l);
                continue;
            }
            MenuEntry::Item(t) => (t, None),
            MenuEntry::Check(t, on) => (t, Some(on)),
        };
        let b = gtk::Button::new();
        b.add_css_class("MenuItem");
        let row = hbox(8);
        if let Some(on) = check {
            let img = draw::icon_image("check", if on { theme::TEXT } else { "#00000000" }, 14);
            row.append(&img);
        }
        let l = label(&text, "");
        row.append(&l);
        b.set_child(Some(&row));
        cursor_hand(&b);
        let p = pop.clone();
        let d = done.clone();
        let i = idx;
        idx += 1;
        b.connect_clicked(move |_| {
            p.popdown();
            d(i);
        });
        bx.append(&b);
    }
    pop.set_child(Some(&bx));
    pop.connect_closed(|p| {
        let p = p.clone();
        glib::idle_add_local_once(move || p.unparent());
    });
    pop.popup();
}
