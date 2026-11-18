//! KDE Plasma im Tab „Einstellungen“: Farbschema und Plasma-Design (plasma-apply-*).

use super::form::{combo, retranslate_buttons, tbtn, Page, Row};
use super::win::{win, AioPage};
use crate::i18n::t;
use crate::usercmd;
use gtk::prelude::*;
use hyprstudio::ui::widgets::{hbox, ComboItem, SearchCombo};
use once_cell::sync::Lazy;
use regex::Regex;
use std::rc::Rc;

static LINE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\s*\*\s+(.+?)(\s+\((current|aktuell).*\))?\s*$").unwrap());

/// `plasma-apply-colorscheme --list-schemes`: ` * BreezeDark (current color scheme)`.
fn list(cmd: &[&str]) -> (Vec<String>, String) {
    let out = usercmd::query(cmd).unwrap_or_default();
    let (mut names, mut cur) = (vec![], String::new());
    for ln in out.lines() {
        if let Some(m) = LINE.captures(ln) {
            let n = m[1].to_string();
            if m.get(2).is_some() {
                cur = n.clone();
            }
            names.push(n);
        }
    }
    (names, cur)
}

pub struct PlasmaLook {
    page: Rc<Page>,
    scheme: Rc<SearchCombo>,
    desktop: Rc<SearchCombo>,
    b_scheme: gtk::Button,
    b_desktop: gtk::Button,
    rows: Vec<Rc<Row>>,
}

impl PlasmaLook {
    pub fn new() -> Rc<PlasmaLook> {
        let page = Page::new("pl.look", true);
        let card = page.card("");
        let scheme = combo();
        let desktop = combo();
        let b_scheme = tbtn("apply", true);
        let b_desktop = tbtn("apply", true);
        let mut rows = vec![];
        for (key, cb, b) in [("pl.scheme", &scheme, &b_scheme), ("pl.desktop", &desktop, &b_desktop)] {
            let w = hbox(6);
            w.append(&cb.button);
            w.append(b);
            let r = Row::new(key, &w);
            card.append(&r.root);
            rows.push(r);
        }
        page.finish();
        let p = Rc::new(PlasmaLook { page, scheme, desktop, b_scheme, b_desktop, rows });
        let me = Rc::downgrade(&p);
        p.page.on_refresh.replace(Some(Rc::new(move || {
            if let Some(p) = me.upgrade() {
                p.load();
            }
        })));
        let c = p.scheme.clone();
        p.b_scheme.connect_clicked(move |_| apply("plasma-apply-colorscheme", &c));
        let c = p.desktop.clone();
        p.b_desktop.connect_clicked(move |_| apply("plasma-apply-desktoptheme", &c));
        p
    }

    fn load(&self) {
        for (cb, cmd) in [(&self.scheme, ["plasma-apply-colorscheme", "--list-schemes"]),
                          (&self.desktop, ["plasma-apply-desktoptheme", "--list-themes"])] {
            let (names, cur) = list(&cmd);
            cb.set_items(names.iter().map(|n| ComboItem::new(n, n)).collect());
            if !cur.is_empty() {
                cb.set_current(cb.find_data(&cur).max(0));
            }
            cb.set_sensitive(!names.is_empty());
        }
        self.b_scheme.set_sensitive(!self.scheme.current_text().is_empty());
        self.b_desktop.set_sensitive(!self.desktop.current_text().is_empty());
    }
}

fn apply(tool: &str, cb: &SearchCombo) {
    let name = cb.current_text();
    if name.is_empty() {
        return;
    }
    let w = win();
    w.log("");
    let (ok, _) = usercmd::run(&[tool, &name]);
    w.set_status(&t(if ok { "done" } else { "failed" }));
}

impl AioPage for PlasmaLook {
    fn widget(&self) -> gtk::Widget {
        self.page.root.clone().upcast()
    }
    fn retranslate(&self) {
        self.page.retranslate();
        for r in &self.rows {
            r.retranslate();
        }
        retranslate_buttons(&self.page.root);
    }
    fn refresh(&self) {
        self.page.refresh();
    }
}
