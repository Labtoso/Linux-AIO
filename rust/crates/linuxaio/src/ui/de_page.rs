//! Tab „Desktops“: Desktop-Umgebungen installieren, als Standard setzen, entfernen.

use super::preview::Preview;
use super::widgets::{seg, set_applied, ChipRow, Tile, TileStrip};
use super::win::{win, AioPage};
use crate::catalog::{self, VARIANTS};
use crate::i18n::{lang, t, tf};
use crate::{system, usercmd};
use gtk::prelude::*;
use hyprstudio::kw;
use hyprstudio::ui::widgets::{cursor_hand, hbox, label, vbox, vspacer, Segmented};
use serde_json::{Map, Value};
use std::cell::RefCell;
use std::rc::Rc;

pub fn read_state() -> Map<String, Value> {
    let p = if usercmd::dry() {
        std::env::temp_dir().join("linuxaio-dry-state.json")
    } else {
        std::path::PathBuf::from(system::STATE_FILE)
    };
    system::load_state_file(&p)
}

pub struct DePage {
    root: gtk::Box,
    state: RefCell<Map<String, Value>>,
    default_de: RefCell<Option<String>>,
    current: RefCell<String>,
    variant: RefCell<String>,
    big: Rc<Preview>,
    de_name: gtk::Label,
    chips: Rc<ChipRow>,
    seg: Rc<Segmented>,
    pkg_label: gtk::Label,
    btn_install: gtk::Button,
    btn_default: gtk::Button,
    btn_remove: gtk::Button,
    strip: Rc<TileStrip>,
    tiles: Vec<(String, Rc<Tile>, Rc<Preview>)>,
}

impl DePage {
    pub fn new() -> Rc<DePage> {
        let w = win();
        let current = match &w.running {
            Some(r) if r != "plasma" && catalog::get(r).is_some() => r.clone(),
            _ => "gnome".to_string(),
        };
        let root = vbox(14);
        let hero = hbox(24);
        hero.add_css_class("AHero");
        hero.set_size_request(-1, 280);
        hero.set_vexpand(true);
        hyprstudio::ui::widgets::margins(&hero, 0, 0, 0, 0);
        let inner = hbox(24);
        hyprstudio::ui::widgets::margins(&inner, 18, 18, 18, 18);
        inner.set_hexpand(true);
        hero.append(&inner);
        let big = Preview::new(12.0);
        big.area.set_hexpand(true);
        big.area.set_vexpand(true);
        inner.append(&big.area);
        let side = vbox(12);
        side.set_size_request(300, -1);
        side.set_hexpand(false);
        let de_name = label("", "DeName");
        de_name.set_xalign(0.0);
        side.append(&de_name);
        let chips = ChipRow::new();
        chips.tip_for(&de_name);
        side.append(&chips.root);
        let sg = seg(&VARIANTS, "");
        sg.root.set_halign(gtk::Align::Start);
        sg.root.set_margin_top(8);
        side.append(&sg.root);
        let pkg_label = label("", "Label");
        pkg_label.set_xalign(0.0);
        side.append(&pkg_label);
        side.append(&vspacer());
        let btn_install = gtk::Button::with_label("");
        btn_install.add_css_class("Primary");
        let btn_default = gtk::Button::with_label("");
        let btn_remove = gtk::Button::with_label("");
        btn_remove.add_css_class("Danger");
        for b in [&btn_install, &btn_default, &btn_remove] {
            cursor_hand(b);
            b.set_size_request(-1, 40);
            side.append(b);
        }
        inner.append(&side);
        root.append(&hero);
        let strip = TileStrip::new();
        let mut tiles = vec![];
        for de in catalog::DESKTOPS.iter() {
            let pv = Preview::new(8.0);
            pv.set(de, "standard");
            let tile = Tile::new(&de.id, &de.name, pv.area.upcast_ref());
            strip.add(&tile.root);
            tiles.push((de.id.clone(), tile, pv));
        }
        root.append(&strip.root);
        let p = Rc::new(DePage { root, state: RefCell::new(Map::new()), default_de: RefCell::new(None),
            current: RefCell::new(current), variant: RefCell::new("standard".into()), big, de_name, chips, seg: sg,
            pkg_label, btn_install, btn_default, btn_remove, strip, tiles });
        for (_, tile, _) in &p.tiles {
            let wp = Rc::downgrade(&p);
            tile.connect(move |id| {
                if let Some(p) = wp.upgrade() {
                    p.select(&id);
                }
            });
        }
        let wp = Rc::downgrade(&p);
        p.seg.connect(move |v| {
            if let Some(p) = wp.upgrade() {
                *p.variant.borrow_mut() = v;
                p.refresh();
            }
        });
        let wp = Rc::downgrade(&p);
        p.btn_install.connect_clicked(move |_| {
            if let Some(p) = wp.upgrade() {
                let args = vec!["install".into(), p.current.borrow().clone(), p.variant.borrow().clone()];
                win().run(args, "install", Some(p.clone() as Rc<dyn AioPage>));
            }
        });
        let wp = Rc::downgrade(&p);
        p.btn_default.connect_clicked(move |_| {
            if let Some(p) = wp.upgrade() {
                let args = vec!["default".into(), p.current.borrow().clone()];
                win().run(args, "default", Some(p.clone() as Rc<dyn AioPage>));
            }
        });
        let wp = Rc::downgrade(&p);
        p.btn_remove.connect_clicked(move |_| {
            let Some(p) = wp.upgrade() else { return };
            let cur = p.current.borrow().clone();
            let name = catalog::get(&cur).map(|d| d.name.clone()).unwrap_or_default();
            let p2 = p.clone();
            win().confirm(&t("remove"), &tf("confirm.remove", kw!("name" => name)), move |ok| {
                if ok {
                    let w = win();
                    let run = w.running.clone().unwrap_or_default();
                    let args = vec!["remove".into(), cur, format!("--running={run}")];
                    w.run(args, "remove", Some(p2.clone() as Rc<dyn AioPage>));
                }
            });
        });
        p
    }

    fn select(&self, de_id: &str) {
        *self.current.borrow_mut() = de_id.to_string();
        let v = self.state.borrow().get(de_id).and_then(|s| s.get("variant")).and_then(|v| v.as_str())
            .map(String::from).unwrap_or_else(|| "standard".into());
        *self.variant.borrow_mut() = v;
        self.refresh();
    }
}

impl AioPage for DePage {
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }

    fn drawer_anchor(&self) -> gtk::Widget {
        self.strip.root.clone().upcast()
    }

    fn retranslate(&self) {
        self.btn_default.set_label(&t("default"));
        self.btn_remove.set_label(&t("remove"));
        self.seg.retranslate();
        self.refresh();
    }

    fn finished(&self, _ok: bool, _action: &str, _result: &str) {
        self.refresh();
    }

    fn refresh(&self) {
        let w = win();
        let backend = w.backend.clone();
        let running = w.running.clone();
        *self.state.borrow_mut() = read_state();
        *self.default_de.borrow_mut() = system::default_desktop();
        let cur = self.current.borrow().clone();
        let Some(de) = catalog::get(&cur) else { return };
        let state = self.state.borrow();
        let installed = |id: &str| -> bool {
            catalog::get(id).map(system::is_installed).unwrap_or(false) || state.contains_key(id)
        };
        for (id, tile, _) in &self.tiles {
            tile.refresh(*id == cur, installed(id).then_some("#3fb950"));
            tile.root.set_tooltip_text(catalog::get(id).map(|d| d.desc(&lang())).as_deref());
        }
        let be = backend.as_deref();
        if be.map(|b| !de.supports(&self.variant.borrow(), b)).unwrap_or(true) {
            for v in VARIANTS {
                if let Some(b) = be {
                    if de.supports(v, b) {
                        *self.variant.borrow_mut() = v.to_string();
                        break;
                    }
                }
            }
        }
        let variant = self.variant.borrow().clone();
        self.big.set(de, &variant);
        self.de_name.set_text(&de.name);
        *self.chips.base_tip.borrow_mut() = de.desc(&lang());
        let inst = installed(&de.id);
        let mut chips: Vec<(String, String, String)> = vec![];
        let mut add = |text: String, kind: &str| chips.push((text, kind.to_string(), String::new()));
        if de.wayland {
            add(t("wayland"), "Chip");
        }
        if de.x11 {
            add(t("x11"), "Chip");
        }
        if de.tiling {
            add(t("tiling"), "Chip");
        }
        if inst {
            add(t("installed"), "ChipOk");
        }
        let default_de = self.default_de.borrow().clone();
        if running.as_deref() == Some(de.id.as_str()) {
            add(t("running"), "ChipAccent");
        } else if default_de.as_deref() == Some(de.id.as_str()) {
            add(t("isdefault"), "ChipAccent");
        }
        self.chips.set(&chips);
        for v in VARIANTS {
            if let Some(b) = self.seg.btn(v) {
                let ok = be.map(|x| de.supports(v, x)).unwrap_or(false);
                b.set_sensitive(ok);
                b.set_active(v == variant);
                let specs = be.map(|x| de.specs(v, x)).unwrap_or_default();
                let tip = if ok {
                    specs.iter().map(|s| s.trim_start_matches('?')).collect::<Vec<_>>().join(" ")
                } else {
                    t("unsupported")
                };
                b.set_tooltip_text(Some(&tip));
            }
        }
        let specs = be.map(|x| de.specs(&variant, x)).unwrap_or_default();
        self.pkg_label.set_text(&format!("{}: {}  ·  {}", t("packages"), specs.len(), be.unwrap_or("–")));
        self.pkg_label.set_tooltip_text(Some(&specs.join("\n")));

        let busy = w.busy();
        let same = state.get(&de.id).and_then(|s| s.get("variant")).and_then(|v| v.as_str()) == Some(variant.as_str());
        set_applied(&self.btn_install, same, &t("installed"));
        if !same {
            self.btn_install.set_label(&if inst { t("reinstall") } else { t("install") });
            self.btn_install.set_sensitive(!busy && be.map(|x| de.supports(&variant, x)).unwrap_or(false));
        }
        let is_default = inst && default_de.as_deref() == Some(de.id.as_str());
        set_applied(&self.btn_default, is_default, &t("applied"));
        if !is_default {
            self.btn_default.set_label(&t("default"));
            self.btn_default.set_sensitive(!busy && inst);
        }
        let is_running = running.as_deref() == Some(de.id.as_str());
        self.btn_remove.set_sensitive(!busy && inst && !is_running);
        self.btn_remove.set_tooltip_text(if is_running { Some(t("err.running")) } else { None }.as_deref());
    }
}
