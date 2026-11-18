//! Tab „Einstellungen“ (vorläufig).

use super::win::AioPage;
use gtk::prelude::*;
use hyprstudio::ui::widgets::vbox;
use std::rc::Rc;

pub struct SettingsPage {
    root: gtk::Box,
}

impl SettingsPage {
    pub fn new() -> Rc<SettingsPage> {
        Rc::new(SettingsPage { root: vbox(0) })
    }
    pub fn apply_accent(&self) {}
    pub fn select(&self, _de: &str) {}
    pub fn select_adv(&self, _key: &str) {}
    pub fn show_section(&self, _sec: &str, _sub: Option<&str>) {}
}

impl AioPage for SettingsPage {
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }
}
