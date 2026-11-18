//! Eigene Presets: aktuellen Stand unter einem Namen speichern, laden, umbenennen, entfernen, als Datei
//! exportieren/importieren. Gespeichert in der App-Config (cfg["user_presets"][bereich]).

use super::dialogs::{self, MenuEntry};
use super::shell::{self, Conf};
use super::widgets::{button, flow_grid, hbox, label, spacer, PresetCard};
use crate::i18n::{t, tf};
use crate::jsonx::{self, Get, Obj};
use crate::{kw, util};
use gtk::prelude::*;
use gtk::gdk;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::path::Path;
use std::rc::{Rc, Weak};

pub const PREFIX: &str = "user:";

/// Was eine Seite mit Presets können muss (Editor-Seiten, Kitty).
pub trait PresetHost {
    fn kind(&self) -> String;
    fn conf(&self) -> Conf;
    fn state(&self) -> Obj;
    fn sanitize(&self, v: Option<&Obj>) -> Obj;
    fn state_texture(&self, s: &Obj) -> gdk::Texture;
    fn use_preset(&self, name: &str);
    fn update_applied(&self);
    /// Preset-Karten der Seite (eingebaute + eigene); eigene werden hier ein- und ausgetragen.
    fn presets(&self) -> &RefCell<Vec<(String, Rc<PresetCard>)>>;
    fn widget(&self) -> gtk::Widget;
}

/// Bereich der eigenen Presets in der Konfiguration (wird angelegt).
pub fn with_store<R>(cfg: &mut Obj, kind: &str, f: impl FnOnce(&mut Obj) -> R) -> R {
    let up = cfg.entry("user_presets").or_insert_with(|| json!({}));
    if !up.is_object() {
        *up = json!({});
    }
    let st = up.as_object_mut().unwrap().entry(kind).or_insert_with(|| json!({}));
    if !st.is_object() {
        *st = json!({});
    }
    f(st.as_object_mut().unwrap())
}

pub fn store_copy(cfg: &Obj, kind: &str) -> Obj {
    cfg.obj("user_presets").and_then(|u| u.obj(kind)).cloned().unwrap_or_default()
}

fn sorted_names(m: &Obj) -> Vec<String> {
    let mut v: Vec<String> = m.keys().cloned().collect();
    v.sort_by_key(|x| x.to_lowercase());
    v
}

pub struct UserPresets {
    pub btn_save: gtk::Button,
    pub btn_import: gtk::Button,
    pub lbl_user: gtk::Label,
    pub user_grid: gtk::FlowBox,
    pub lbl_builtin: gtk::Label,
    host: RefCell<Option<Weak<dyn PresetHost>>>,
}

impl UserPresets {
    /// Knöpfe, „Meine Presets“ und die Überschrift der eingebauten in `layout` legen.
    pub fn build(layout: &gtk::Box) -> Rc<UserPresets> {
        let row = hbox(6);
        let btn_save = button("", "");
        let btn_import = button("", "");
        row.append(&btn_save);
        row.append(&btn_import);
        row.append(&spacer());
        layout.append(&row);
        let lbl_user = label("", "CardTitle");
        layout.append(&lbl_user);
        let user_grid = flow_grid();
        layout.append(&user_grid);
        let lbl_builtin = label("", "CardTitle");
        layout.append(&lbl_builtin);
        let u = Rc::new(UserPresets { btn_save, btn_import, lbl_user, user_grid, lbl_builtin, host: RefCell::new(None) });
        let w = Rc::downgrade(&u);
        u.btn_save.connect_clicked(move |_| {
            if let Some(u) = w.upgrade() {
                u.save();
            }
        });
        let w = Rc::downgrade(&u);
        u.btn_import.connect_clicked(move |_| {
            if let Some(u) = w.upgrade() {
                u.import();
            }
        });
        u
    }

    pub fn set_host(&self, h: Weak<dyn PresetHost>) {
        *self.host.borrow_mut() = Some(h);
    }

    fn host(&self) -> Option<Rc<dyn PresetHost>> {
        self.host.borrow().as_ref().and_then(|w| w.upgrade())
    }

    fn parent(&self) -> Option<gtk::Window> {
        shell::toplevel_of(&self.btn_save)
    }

    pub fn reload(self: &Rc<Self>) {
        let Some(h) = self.host() else { return };
        h.presets().borrow_mut().retain(|(k, _)| !k.starts_with(PREFIX));
        while let Some(c) = self.user_grid.first_child() {
            self.user_grid.remove(&c);
        }
        let conf = h.conf();
        let data = store_copy(&conf.cfg.borrow(), &h.kind());
        for name in sorted_names(&data) {
            let key = format!("{PREFIX}{name}");
            let pc = PresetCard::new(&key);
            pc.name.set_text(&name);
            pc.root.set_tooltip_text(Some(&t("up.tip")));
            let wh = Rc::downgrade(&h);
            pc.connect(move |k| {
                if let Some(h) = wh.upgrade() {
                    h.use_preset(&k);
                }
            });
            let w = Rc::downgrade(self);
            let n = name.clone();
            let root = pc.root.clone();
            pc.connect_menu(move |(x, y)| {
                if let Some(u) = w.upgrade() {
                    u.menu(&n, &root, x, y);
                }
            });
            let st = h.sanitize(data.obj(&name));
            pc.set_texture(&h.state_texture(&st));
            self.user_grid.append(&pc.root);
            h.presets().borrow_mut().push((key, pc));
        }
        self.lbl_user.set_visible(!data.is_empty());
        self.user_grid.set_visible(!data.is_empty());
        h.update_applied();
    }

    pub fn user_state(&self, key: &str) -> Obj {
        let Some(h) = self.host() else { return Obj::new() };
        let data = store_copy(&h.conf().cfg.borrow(), &h.kind());
        h.sanitize(data.obj(key.strip_prefix(PREFIX).unwrap_or(key)))
    }

    fn ask_name(&self, title: &str, text: &str, done: impl FnOnce(String) + 'static) {
        dialogs::ask_text(self.parent().as_ref(), title, &t("up.name"), text, move |r| {
            if let Some(n) = r.map(|x| x.trim().to_string()).filter(|x| !x.is_empty()) {
                done(n);
            }
        });
    }

    fn confirm(&self, text: &str, done: impl FnOnce() + 'static) {
        dialogs::confirm(self.parent().as_ref(), &t("up.title"), text, None, false, move |ok| {
            if ok {
                done();
            }
        });
    }

    fn save(self: &Rc<Self>) {
        let w = Rc::downgrade(self);
        self.ask_name(&t("up.save"), "", move |name| {
            let Some(u) = w.upgrade() else { return };
            let Some(h) = u.host() else { return };
            let exists = store_copy(&h.conf().cfg.borrow(), &h.kind()).contains_key(&name);
            let name2 = name.clone();
            let w2 = Rc::downgrade(&u);
            let go = move || {
                let Some(u) = w2.upgrade() else { return };
                let Some(h) = u.host() else { return };
                let conf = h.conf();
                let kind = h.kind();
                {
                    let mut c = conf.cfg.borrow_mut();
                    with_store(&mut c, &kind, |m| {
                        m.insert(name.clone(), Value::Object(h.state()));
                    });
                    c.insert(format!("{kind}_preset"), json!(format!("{PREFIX}{name}")));
                }
                conf.save_later();
                u.reload();
                shell::status(&tf("up.saved", kw!("name" => name)));
            };
            if exists {
                u.confirm(&tf("up.overwrite", kw!("name" => name2)), go);
            } else {
                go();
            }
        });
    }

    fn menu(self: &Rc<Self>, name: &str, anchor: &gtk::Box, x: f64, y: f64) {
        let entries = vec![MenuEntry::Item(t("up.update")), MenuEntry::Item(t("up.rename")),
            MenuEntry::Item(t("up.export")), MenuEntry::Sep, MenuEntry::Item(t("up.remove"))];
        let w = Rc::downgrade(self);
        let name = name.to_string();
        dialogs::popup_menu(anchor, Some((x, y)), entries, move |i| {
            let Some(u) = w.upgrade() else { return };
            let Some(h) = u.host() else { return };
            let conf = h.conf();
            let kind = h.kind();
            let finish = {
                let w = Rc::downgrade(&u);
                let conf = conf.clone();
                move || {
                    conf.save_later();
                    if let Some(u) = w.upgrade() {
                        u.reload();
                    }
                }
            };
            match i {
                0 => {
                    with_store(&mut conf.cfg.borrow_mut(), &kind, |m| {
                        m.insert(name.clone(), Value::Object(h.state()));
                    });
                    shell::status(&tf("up.saved", kw!("name" => name)));
                    finish();
                }
                1 => {
                    let w2 = Rc::downgrade(&u);
                    let old = name.clone();
                    u.ask_name(&t("up.rename"), &name, move |new| {
                        let Some(u) = w2.upgrade() else { return };
                        if new == old {
                            return;
                        }
                        let exists = store_copy(&conf.cfg.borrow(), &kind).contains_key(&new);
                        let new2 = new.clone();
                        let conf2 = conf.clone();
                        let kind2 = kind.clone();
                        let go = move || {
                            let mut c = conf2.cfg.borrow_mut();
                            with_store(&mut c, &kind2, |m| {
                                if let Some(v) = m.shift_remove(&old) {
                                    m.insert(new.clone(), v);
                                }
                            });
                            for k in [format!("{kind2}_preset"), format!("{kind2}_applied_preset")] {
                                if c.s(&k) == format!("{PREFIX}{old}") {
                                    c.insert(k, json!(format!("{PREFIX}{new}")));
                                }
                            }
                            drop(c);
                            finish();
                        };
                        if exists {
                            u.confirm(&tf("up.overwrite", kw!("name" => new2)), go);
                        } else {
                            go();
                        }
                    });
                }
                2 => u.export(&name),
                _ => {
                    let n = name.clone();
                    u.confirm(&tf("up.remove_q", kw!("name" => name)), move || {
                        with_store(&mut conf.cfg.borrow_mut(), &kind, |m| {
                            m.shift_remove(&n);
                        });
                        finish();
                    });
                }
            }
        });
    }

    fn export(&self, name: &str) {
        let Some(h) = self.host() else { return };
        let start = util::home().join(format!("{name}.hyprstudio.json"));
        let kind = h.kind();
        let state = store_copy(&h.conf().cfg.borrow(), &kind).val(name).clone();
        let name = name.to_string();
        dialogs::save_file(self.parent().as_ref(), &t("up.export"), &start.to_string_lossy(),
            &[("Hypr Studio", &["*.json"])], move |p| {
                let Some(p) = p else { return };
                let payload = json!({"hyprstudio": kind, "name": name, "state": state});
                let _ = util::write_text(&p, &jsonx::dumps_indent(&payload, 2));
                shell::status(&tf("up.exported", kw!("path" => p.to_string_lossy())));
            });
    }

    fn import(self: &Rc<Self>) {
        let w = Rc::downgrade(self);
        dialogs::open_file(self.parent().as_ref(), &t("up.import"), &util::home().to_string_lossy(),
            &[("Hypr Studio", &["*.json"])], move |p| {
                let (Some(u), Some(p)) = (w.upgrade(), p) else { return };
                let Some(h) = u.host() else { return };
                let kind = h.kind();
                let payload = util::read_opt(&p).map(|x| jsonx::parse(&x)).unwrap_or(Value::Null);
                let ok = payload.get("hyprstudio").and_then(|x| x.as_str()) == Some(kind.as_str())
                    && payload.get("state").map(|x| x.is_object()).unwrap_or(false);
                if !ok {
                    shell::status(&t("up.badfile"));
                    return;
                }
                let name = payload.get("name").filter(|x| jsonx::truthy(x)).map(jsonx::py_str)
                    .unwrap_or_else(|| stem(&p));
                let state = h.sanitize(payload["state"].as_object());
                let exists = store_copy(&h.conf().cfg.borrow(), &kind).contains_key(&name);
                let w2 = Rc::downgrade(&u);
                let go = move || {
                    let Some(u) = w2.upgrade() else { return };
                    let Some(h) = u.host() else { return };
                    let conf = h.conf();
                    with_store(&mut conf.cfg.borrow_mut(), &kind, |m| {
                        m.insert(name.clone(), Value::Object(state));
                    });
                    conf.save_later();
                    u.reload();
                    shell::status(&tf("up.imported", kw!("name" => name)));
                };
                if exists {
                    u.confirm(&tf("up.overwrite", kw!("name" => name_of(&payload, &p))), go);
                } else {
                    go();
                }
            });
    }

    pub fn retranslate(&self) {
        self.btn_save.set_label(&t("up.save"));
        self.btn_import.set_label(&t("up.import"));
        self.lbl_user.set_text(&t("up.mine"));
        self.lbl_builtin.set_text(&t("up.builtin"));
    }
}

fn stem(p: &Path) -> String {
    p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()
}

fn name_of(payload: &Value, p: &Path) -> String {
    payload.get("name").filter(|x| jsonx::truthy(x)).map(jsonx::py_str).unwrap_or_else(|| stem(p))
}
