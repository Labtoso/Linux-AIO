//! Hypr Studio komplett im Tab „Einstellungen › Hyprland“ (eingebettet) oder als eigenes Fenster.

use super::win::{win, AioPage};
use crate::i18n::t;
use crate::usercmd;
use gtk::prelude::*;
use hyprstudio::jsonx::Get;
use hyprstudio::ui::shell::Shell;
use hyprstudio::ui::studio::{Opts, Studio};
use hyprstudio::ui::widgets::{cursor_hand, vbox, vspacer};
use std::rc::Rc;

fn opts(embedded: bool, popout: bool, launcher_only: bool) -> Opts {
    Opts { embedded, popout, launcher_only, aio: true, host: Some(win() as Rc<dyn Shell>), only_on_hyprland: true,
        host_dry: usercmd::dry() }
}

fn look() -> (String, String) {
    let w = win();
    let c = w.cfg.borrow();
    let lang = if c.s("lang").is_empty() { "en" } else { c.s("lang") }.to_string();
    let th = if c.s("theme").is_empty() { "01" } else { c.s("theme") }.to_string();
    (lang, th)
}

/// Hypr Studio als eingebettetes Widget (ohne Fensterrahmen, Titel, Minimieren/Schließen).
pub struct HyprEmbed {
    pub studio: Rc<Studio>,
}

impl HyprEmbed {
    pub fn new(launcher_only: bool) -> Rc<HyprEmbed> {
        let studio = Studio::new(None, opts(true, false, launcher_only));
        Rc::new(HyprEmbed { studio })
    }
}

impl AioPage for HyprEmbed {
    fn widget(&self) -> gtk::Widget {
        self.studio.root.clone().upcast()
    }
    fn refresh(&self) {
        let (l, th) = look();
        self.studio.sync_look(&l, &th);
    }
    fn retranslate(&self) {
        self.refresh();
    }
}

/// Einstellung „eigenes Fenster“: kleine Seite mit Knopf; Hypr Studio öffnet als eigenes Fenster.
pub struct HyprLauncher {
    root: gtk::Box,
    btn: gtk::Button,
}

impl HyprLauncher {
    pub fn new() -> Rc<HyprLauncher> {
        let root = vbox(0);
        root.append(&vspacer());
        let btn = gtk::Button::with_label(&t("app.hypr.open"));
        btn.add_css_class("Primary");
        btn.set_size_request(240, 44);
        btn.set_halign(gtk::Align::Center);
        cursor_hand(&btn);
        btn.connect_clicked(|_| open());
        root.append(&btn);
        root.append(&vspacer());
        Rc::new(HyprLauncher { root, btn })
    }
}

pub fn open() {
    let w = win();
    let existing = w.hypr_window.borrow().clone();
    let s = match existing {
        Some(s) => s,
        None => {
            let app = w.window.application();
            let s = Studio::new(app.as_ref(), opts(false, true, false));
            *w.hypr_window.borrow_mut() = Some(s.clone());
            s
        }
    };
    let (l, th) = look();
    s.sync_look(&l, &th);
    s.present();
}

impl AioPage for HyprLauncher {
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }
    fn retranslate(&self) {
        self.btn.set_label(&t("app.hypr.open"));
    }
    fn refresh(&self) {
        self.retranslate();
    }
}
