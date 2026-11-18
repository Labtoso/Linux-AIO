//! Globale Suche (vorläufig).

use super::widgets::search_entry;
use crate::i18n::t;
use std::rc::Rc;

pub struct SearchBox {
    pub entry: gtk::Entry,
}

impl SearchBox {
    pub fn new() -> Rc<SearchBox> {
        Rc::new(SearchBox { entry: search_entry(240) })
    }
    pub fn retranslate(&self) {
        use gtk::prelude::*;
        self.entry.set_placeholder_text(Some(&t("search")));
    }
}
