//! Tastenkürzel-Editor: Zeilen aus Modifikatoren, aufgenommener Taste, Aktion und Ziel.

use super::dialogs::{self, MenuEntry};
use super::draw;
use super::shell;
use super::theme;
use super::widgets::{cursor_hand, hbox, label, spacer, tool_button, set_tool_icon, button, vbox, Cb, ComboItem,
                     SearchCombo};
use crate::i18n::{has, t, tf};
use crate::jsonx::{self, Get, Obj};
use crate::{kw, model};
use gtk::prelude::*;
use gtk::{gdk, glib, pango};
use glib::translate::{FromGlib, IntoGlib};
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};

const MAX_KEYS: usize = 4;

fn key_label(key: &str) -> String {
    let k = format!("key.{key}");
    if has(&k) { t(&k) } else { key.to_string() }
}

pub fn combo_label(mods: &[String], key: &str) -> String {
    if model::SUPER_ALONE.contains(&key) {
        return t("key.super_alone");
    }
    let mut parts: Vec<String> = ["SUPER", "CTRL", "ALT", "SHIFT"].iter().filter(|m| mods.iter().any(|x| x == *m))
        .map(|m| t(&format!("mod.{m}"))).collect();
    if !key.is_empty() {
        parts.push(key_label(key));
    }
    parts.join(" + ")
}

/// „Strg + Shift + S“ oder „SUPER SHIFT, S“ → (mods, key).
pub fn parse_combo(text: &str) -> (Vec<String>, String) {
    let norm = text.replace([',', ' '], "+");
    let parts: Vec<&str> = norm.split('+').map(str::trim).filter(|x| !x.is_empty()).collect();
    let Some((last, rest)) = parts.split_last() else { return (vec![], String::new()) };
    let alias = |x: &str| -> Option<&'static str> {
        match x {
            "STRG" | "STEUERUNG" => Some("CTRL"),
            "UMSCHALT" => Some("SHIFT"),
            _ => model::mod_alias(x),
        }
    };
    let mods: Vec<&str> = rest.iter().filter_map(|x| alias(&x.to_uppercase())).collect();
    (model::MODS.iter().filter(|m| mods.contains(m)).map(|m| m.to_string()).collect(), last.to_string())
}

fn is_mod_key(k: gdk::Key) -> bool {
    use gdk::Key as K;
    matches!(k, K::Shift_L | K::Shift_R | K::Control_L | K::Control_R | K::Alt_L | K::Alt_R | K::Meta_L | K::Meta_R
        | K::Super_L | K::Super_R | K::ISO_Level3_Shift | K::Caps_Lock | K::Hyper_L | K::Hyper_R)
}

fn is_super_key(k: gdk::Key) -> bool {
    matches!(k, gdk::Key::Super_L | gdk::Key::Super_R | gdk::Key::Meta_L | gdk::Key::Meta_R)
}

/// Zusatztasten aus dem Zustand (+ der gerade gedrückten Zusatztaste selbst, die GTK noch nicht mitzählt).
fn mods_of(state: gdk::ModifierType, extra: Option<gdk::Key>) -> Vec<String> {
    use gdk::Key as K;
    use gdk::ModifierType as M;
    let mut on: Vec<&str> = vec![];
    if state.contains(M::SUPER_MASK) || state.contains(M::META_MASK) || matches!(extra, Some(k) if is_super_key(k)) {
        on.push("SUPER");
    }
    if state.contains(M::CONTROL_MASK) || matches!(extra, Some(K::Control_L | K::Control_R)) {
        on.push("CTRL");
    }
    if state.contains(M::ALT_MASK) || matches!(extra, Some(K::Alt_L | K::Alt_R)) {
        on.push("ALT");
    }
    if state.contains(M::SHIFT_MASK) || matches!(extra, Some(K::Shift_L | K::Shift_R)) {
        on.push("SHIFT");
    }
    model::MODS.iter().filter(|m| on.contains(m)).map(|m| m.to_string()).collect()
}

/// Name der Taste wie Hyprland ihn erwartet. Die Zahlenreihe geht über die Position, damit SHIFT + 1 nicht
/// zu „!“ wird.
fn key_name(key: gdk::Key, code: u32) -> Option<String> {
    use gdk::Key as K;
    if (10..=19).contains(&code) {
        return Some(((code - 9) % 10).to_string());
    }
    let named = match key {
        K::Return | K::KP_Enter => "Return",
        K::space => "space",
        K::Tab | K::ISO_Left_Tab => "Tab",
        K::BackSpace => "BackSpace",
        K::Delete => "Delete",
        K::Insert => "Insert",
        K::Print => "Print",
        K::Left => "left",
        K::Right => "right",
        K::Up => "up",
        K::Down => "down",
        K::Home => "Home",
        K::End => "End",
        K::Page_Up => "Prior",
        K::Page_Down => "Next",
        K::comma => "comma",
        K::period => "period",
        K::minus => "minus",
        K::plus => "plus",
        K::slash => "slash",
        K::semicolon => "semicolon",
        K::grave | K::dead_grave => "grave",
        K::AudioRaiseVolume => "XF86AudioRaiseVolume",
        K::AudioLowerVolume => "XF86AudioLowerVolume",
        K::AudioMute => "XF86AudioMute",
        K::AudioPlay | K::AudioPause => "XF86AudioPlay",
        K::AudioNext => "XF86AudioNext",
        K::AudioPrev => "XF86AudioPrev",
        K::MonBrightnessUp => "XF86MonBrightnessUp",
        K::MonBrightnessDown => "XF86MonBrightnessDown",
        _ => "",
    };
    if !named.is_empty() {
        return Some(named.into());
    }
    let low = key.to_lower();
    if let Some(c) = low.to_unicode() {
        if c.is_ascii_alphanumeric() {
            return Some(c.to_ascii_uppercase().to_string());
        }
    }
    let n = key.name()?.to_string();
    if let Some(num) = n.strip_prefix('F').and_then(|x| x.parse::<u32>().ok()) {
        if (1..=24).contains(&num) {
            return Some(format!("F{num}"));
        }
    }
    if let Some(c) = key.to_unicode().filter(|c| !c.is_control()) {
        if !c.is_ascii() {
            return Some(c.to_uppercase().to_string());
        }
    }
    Some(n)
}

// ---------- Taste aufnehmen ----------

/// Klicken, dann die ganze Kombination drücken (z. B. Strg + Shift + Alt + A, bis zu 4 Tasten).
/// Esc bricht ab. Rechtsklick: Maus-/Medientasten oder von Hand eingeben.
pub struct KeyButton {
    pub button: gtk::Button,
    text: gtk::Label,
    pub mods: RefCell<Vec<String>>,
    pub key: RefCell<String>,
    listening: Cell<bool>,
    warn: Cell<bool>,
    mods_down: RefCell<HashSet<u32>>,
    other_down: Cell<bool>,
    on_captured: Cb<()>,
}

impl KeyButton {
    pub fn new() -> Rc<KeyButton> {
        let button = gtk::Button::new();
        button.add_css_class("KeyBtn");
        cursor_hand(&button);
        button.set_size_request(260, -1);
        let text = gtk::Label::new(None);
        text.set_ellipsize(pango::EllipsizeMode::Middle);
        text.set_max_width_chars(28);
        button.set_child(Some(&text));
        let k = Rc::new(KeyButton { button, text, mods: RefCell::new(vec![]), key: RefCell::new(String::new()),
            listening: Cell::new(false), warn: Cell::new(false), mods_down: RefCell::new(HashSet::new()),
            other_down: Cell::new(false), on_captured: RefCell::new(None) });
        let w = Rc::downgrade(&k);
        k.button.connect_clicked(move |_| {
            if let Some(k) = w.upgrade() {
                k.start();
            }
        });
        let keys = gtk::EventControllerKey::new();
        keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        let w = Rc::downgrade(&k);
        keys.connect_key_pressed(move |c, key, code, state| {
            let Some(k) = w.upgrade() else { return glib::Propagation::Proceed };
            if !k.listening.get() {
                return glib::Propagation::Proceed;
            }
            k.press(key, code, state, c);
            glib::Propagation::Stop
        });
        let w = Rc::downgrade(&k);
        keys.connect_key_released(move |_, key, _, state| {
            if let Some(k) = w.upgrade() {
                k.release(key, state);
            }
        });
        k.button.add_controller(keys);
        let focus = gtk::EventControllerFocus::new();
        let w = Rc::downgrade(&k);
        focus.connect_leave(move |_| {
            if let Some(k) = w.upgrade() {
                k.stop();
            }
        });
        k.button.add_controller(focus);
        let right = gtk::GestureClick::new();
        right.set_button(3);
        let w = Rc::downgrade(&k);
        right.connect_pressed(move |_, _, x, y| {
            if let Some(k) = w.upgrade() {
                k.menu(x, y);
            }
        });
        k.button.add_controller(right);
        k
    }

    pub fn connect(&self, f: impl Fn(()) + 'static) {
        *self.on_captured.borrow_mut() = Some(Box::new(f));
    }

    fn emit(&self) {
        if let Some(f) = self.on_captured.borrow().as_ref() {
            f(());
        }
    }

    pub fn set_combo(&self, mods: &[String], key: &str) {
        *self.mods.borrow_mut() = mods.to_vec();
        *self.key.borrow_mut() = key.to_string();
        self.listening.set(false);
        self.text.set_text(&if key.is_empty() { t("keys.none") } else { combo_label(mods, key) });
        let mut tip = t("tip.keybtn");
        if !key.is_empty() {
            let b = model::mk_bind(&mods.iter().map(String::as_str).collect::<Vec<_>>(), key, "exec", "", &[]);
            tip.push('\n');
            tip.push_str(&model::combo_text(&b));
        }
        self.button.set_tooltip_text(Some(&tip));
    }

    pub fn start(&self) {
        self.listening.set(true);
        self.warn.set(false);
        self.mods_down.borrow_mut().clear();
        self.other_down.set(false);
        self.text.set_text(&t("keys.press"));
        self.button.grab_focus();
    }

    fn stop(&self) {
        let (m, k) = (self.mods.borrow().clone(), self.key.borrow().clone());
        self.set_combo(&m, &k);
    }

    fn live(&self, mods: &[String]) {
        if self.warn.get() {
            return;
        }
        self.text.set_text(&if mods.is_empty() { t("keys.press") } else { combo_label(mods, "") + " + …" });
    }

    fn captured(&self, mods: Vec<String>, key: &str) {
        self.set_combo(&mods, key);
        self.emit();
    }

    fn press(&self, key: gdk::Key, code: u32, state: gdk::ModifierType, _c: &gtk::EventControllerKey) {
        if is_mod_key(key) {
            self.mods_down.borrow_mut().insert(key.into_glib());
            self.live(&mods_of(state, Some(key)));
            return;
        }
        self.other_down.set(true); // normale Taste dabei → kein „Super allein“
        let mods = mods_of(state, None);
        if key == gdk::Key::Escape && mods.is_empty() {
            self.stop();
            return;
        }
        let Some(name) = key_name(key, code) else { return };
        self.warn.set(false);
        if mods.len() + 1 > MAX_KEYS {
            self.text.set_text(&t("keys.max4"));
            self.warn.set(true);
            return;
        }
        self.captured(mods, &name);
    }

    fn release(&self, key: gdk::Key, state: gdk::ModifierType) {
        if !self.listening.get() {
            return;
        }
        let only_super = !self.other_down.get() && self.mods_down.borrow().iter().all(|k| {
            is_super_key(unsafe { gdk::Key::from_glib(*k) })
        });
        if is_super_key(key) && only_super {
            // nur Super gedrückt und wieder losgelassen → Super allein
            let k = if key == gdk::Key::Super_R { "SUPER_R" } else { "SUPER_L" };
            self.captured(vec!["SUPER".into()], k);
            return;
        }
        if is_mod_key(key) {
            let mut m = mods_of(state, None);
            // GTK meldet beim Loslassen die Taste selbst noch als gedrückt
            let gone = match key {
                k if is_super_key(k) => "SUPER",
                gdk::Key::Control_L | gdk::Key::Control_R => "CTRL",
                gdk::Key::Alt_L | gdk::Key::Alt_R => "ALT",
                gdk::Key::Shift_L | gdk::Key::Shift_R => "SHIFT",
                _ => "",
            };
            m.retain(|x| x != gone);
            self.live(&m);
        }
    }

    fn menu(self: &Rc<Self>, x: f64, y: f64) {
        let mut entries = vec![MenuEntry::Item(t("keys.manual")), MenuEntry::Sep];
        for k in model::SPECIAL_KEYS {
            entries.push(MenuEntry::Item(key_label(k)));
        }
        let w = Rc::downgrade(self);
        dialogs::popup_menu(&self.button, Some((x, y)), entries, move |i| {
            let Some(k) = w.upgrade() else { return };
            if i == 0 {
                let cur = model::combo_text(&model::mk_bind(
                    &k.mods.borrow().iter().map(String::as_str).collect::<Vec<_>>(), &k.key.borrow(), "exec", "", &[]));
                let w2 = Rc::downgrade(&k);
                dialogs::ask_text(shell::toplevel_of(&k.button).as_ref(), &t("keys.manual"), &t("keys.manual_q"), &cur,
                    move |txt| {
                        let (Some(k), Some(txt)) = (w2.upgrade(), txt) else { return };
                        let (mods, key) = parse_combo(&txt);
                        if !key.is_empty() {
                            k.captured(mods, &key);
                        }
                    });
                return;
            }
            let sk = model::SPECIAL_KEYS[i - 1];
            if model::SUPER_ALONE.contains(&sk) {
                k.captured(vec!["SUPER".into()], sk);
            } else {
                // Sondertaste: gedrückte Zusatztasten bleiben
                let m = k.mods.borrow().clone();
                k.captured(m, sk);
            }
        });
    }
}

// ---------- Eingabe mit Vorschlägen ----------

/// Bearbeitbares Feld mit Vorschlagsliste (wie eine editierbare QComboBox).
pub struct EditCombo {
    pub widget: gtk::Box,
    pub entry: gtk::Entry,
    items: RefCell<Vec<String>>,
    chev: gtk::Button,
}

impl EditCombo {
    pub fn new() -> Rc<EditCombo> {
        let widget = hbox(0);
        widget.add_css_class("EditCombo");
        let entry = gtk::Entry::new();
        entry.set_hexpand(true);
        let chev = tool_button("chev", theme::MUTED, 14, "ComboChev");
        widget.append(&entry);
        widget.append(&chev);
        let c = Rc::new(EditCombo { widget, entry, items: RefCell::new(vec![]), chev });
        let w = Rc::downgrade(&c);
        c.chev.connect_clicked(move |_| {
            let Some(c) = w.upgrade() else { return };
            let items = c.items.borrow().clone();
            let entries = items.iter().map(|x| MenuEntry::Item(x.clone())).collect();
            let w2 = Rc::downgrade(&c);
            dialogs::popup_menu(&c.widget, None, entries, move |i| {
                if let Some(c) = w2.upgrade() {
                    if let Some(x) = c.items.borrow().get(i) {
                        c.entry.set_text(x);
                    }
                }
            });
        });
        c
    }

    pub fn set_items(&self, items: Vec<String>) {
        *self.items.borrow_mut() = items;
    }
}

// ---------- Zeile ----------

pub struct BindRow {
    pub widget: gtk::Box,
    pub keybtn: Rc<KeyButton>,
    action: Rc<SearchCombo>,
    arg_pick: Rc<SearchCombo>,
    arg_edit: Rc<EditCombo>,
    warn: gtk::Label,
    more: gtk::Button,
    flags: RefCell<Vec<String>>,
    quiet: Cell<bool>,
    on_changed: Cb<()>,
    on_removed: Cb<()>,
    on_duplicated: Cb<()>,
}

fn action_items() -> Vec<ComboItem> {
    model::ACTIONS.iter().map(|(a, _)| ComboItem::new(&t(&format!("act.{a}")), a)).collect()
}

impl BindRow {
    pub fn new(b: &Obj) -> Rc<BindRow> {
        let widget = hbox(6);
        widget.add_css_class("BindRow");
        super::widgets::margins(&widget, 0, 0, 0, 0);
        let keybtn = KeyButton::new();
        keybtn.set_combo(&b.strs("mods"), b.s("key"));
        keybtn.button.set_margin_end(6);
        let action = SearchCombo::new();
        action.button.set_size_request(220, -1);
        action.set_items(action_items());
        action.set_current(model::ACTIONS.iter().position(|(a, _)| *a == b.s("action")).unwrap_or(0) as i32);
        let arg_pick = SearchCombo::new();
        arg_pick.button.set_size_request(200, -1);
        arg_pick.button.set_hexpand(true);
        let arg_edit = EditCombo::new();
        arg_edit.widget.set_size_request(200, -1);
        arg_edit.widget.set_hexpand(true);
        let warn = gtk::Label::new(Some("!"));
        warn.add_css_class("Warn");
        warn.set_size_request(22, 22);
        warn.set_valign(gtk::Align::Center);
        warn.set_visible(false);
        let more = tool_button("dots", theme::MUTED, 16, "");
        let rm = tool_button("close", theme::MUTED, 16, "");
        rm.set_tooltip_text(Some(&t("keys.remove")));
        widget.append(&keybtn.button);
        widget.append(&action.button);
        widget.append(&arg_pick.button);
        widget.append(&arg_edit.widget);
        widget.append(&warn);
        widget.append(&more);
        widget.append(&rm);
        let r = Rc::new(BindRow { widget, keybtn, action, arg_pick, arg_edit, warn, more,
            flags: RefCell::new(model::FLAGS.iter().filter(|f| b.strs("flags").iter().any(|x| x == *f))
                .map(|f| f.to_string()).collect()),
            quiet: Cell::new(false), on_changed: RefCell::new(None), on_removed: RefCell::new(None),
            on_duplicated: RefCell::new(None) });
        let w = Rc::downgrade(&r);
        r.keybtn.connect(move |_| {
            if let Some(r) = w.upgrade() {
                r.emit_changed();
            }
        });
        let w = Rc::downgrade(&r);
        r.action.connect(move |_| {
            if let Some(r) = w.upgrade() {
                r.fill_arg("");
                r.emit_changed();
            }
        });
        let w = Rc::downgrade(&r);
        r.arg_pick.connect(move |_| {
            if let Some(r) = w.upgrade() {
                r.emit_changed();
            }
        });
        let w = Rc::downgrade(&r);
        r.arg_edit.entry.connect_changed(move |_| {
            if let Some(r) = w.upgrade() {
                r.emit_changed();
            }
        });
        let w = Rc::downgrade(&r);
        r.more.connect_clicked(move |_| {
            if let Some(r) = w.upgrade() {
                r.flags_menu();
            }
        });
        let w = Rc::downgrade(&r);
        rm.connect_clicked(move |_| {
            if let Some(r) = w.upgrade() {
                if let Some(f) = r.on_removed.borrow().as_ref() {
                    f(());
                }
            }
        });
        r.fill_arg(b.s("arg"));
        r.flags_changed();
        r
    }

    fn emit_changed(&self) {
        if self.quiet.get() {
            return;
        }
        if let Some(f) = self.on_changed.borrow().as_ref() {
            f(());
        }
    }

    fn flags_menu(self: &Rc<Self>) {
        let on = self.flags.borrow().clone();
        let mut entries: Vec<MenuEntry> = model::FLAGS.iter()
            .map(|f| MenuEntry::Check(t(&format!("flag.{f}")), on.iter().any(|x| x == f))).collect();
        entries.push(MenuEntry::Sep);
        entries.push(MenuEntry::Item(t("keys.duplicate")));
        let w = Rc::downgrade(self);
        dialogs::popup_menu(&self.more, None, entries, move |i| {
            let Some(r) = w.upgrade() else { return };
            if i < model::FLAGS.len() {
                let f = model::FLAGS[i];
                {
                    let mut fl = r.flags.borrow_mut();
                    if let Some(p) = fl.iter().position(|x| x == f) {
                        fl.remove(p);
                    } else {
                        fl.push(f.into());
                    }
                    let cur = fl.clone();
                    *fl = model::FLAGS.iter().filter(|x| cur.iter().any(|c| c == *x)).map(|x| x.to_string()).collect();
                }
                r.flags_changed();
            } else if let Some(f) = r.on_duplicated.borrow().as_ref() {
                f(());
            }
        });
    }

    fn flags_changed(&self) {
        let on = self.flags.borrow().clone();
        let color = if on.is_empty() { theme::MUTED.to_string() } else { theme::accent_of("tokyo") };
        set_tool_icon(&self.more, "dots", &color, 16);
        let mut tip = t("keys.flags");
        if !on.is_empty() {
            tip.push_str(": ");
            tip.push_str(&on.iter().map(|f| t(&format!("flag.{f}"))).collect::<Vec<_>>().join(", "));
        }
        self.more.set_tooltip_text(Some(&tip));
        self.emit_changed();
    }

    fn kind(&self) -> Option<&'static str> {
        model::action_kind(&self.action.current_data()).flatten()
    }

    fn fill_arg(&self, value: &str) {
        self.quiet.set(true);
        let kind = self.kind();
        let pick = matches!(kind, Some("dir") | Some("launcher"));
        self.arg_pick.button.set_visible(pick);
        self.arg_edit.widget.set_visible(kind.is_some() && !pick);
        match kind {
            Some("dir") => {
                self.arg_pick.set_items(model::DIRECTIONS.iter()
                    .map(|d| ComboItem::new(&t(&format!("dir.{d}")), d)).collect());
                self.arg_pick.set_current(self.arg_pick.find_data(value).max(0));
            }
            Some("launcher") => {
                self.arg_pick.set_items(model::LAUNCHER_CMDS.iter()
                    .map(|(k, _)| ComboItem::new(&t(&format!("app.{k}")), k)).collect());
                self.arg_pick.set_current(self.arg_pick.find_data(value).max(0));
            }
            Some(k) => {
                let items: Vec<String> = if k == "cmd" {
                    model::EXEC_SUGGEST.iter().map(|x| x.to_string()).collect()
                } else {
                    (1..=10).map(|i| i.to_string()).collect()
                };
                self.arg_edit.set_items(items);
                let v = if !value.is_empty() { value } else if k == "cmd" { "" } else { "1" };
                self.arg_edit.entry.set_text(v);
            }
            None => {}
        }
        self.quiet.set(false);
    }

    pub fn value(&self) -> Obj {
        let a = self.action.current_data();
        let arg = match self.kind() {
            Some("dir") | Some("launcher") => self.arg_pick.current_data(),
            Some(_) => self.arg_edit.entry.text().trim().to_string(),
            None => String::new(),
        };
        let mods = self.keybtn.mods.borrow().clone();
        let flags = self.flags.borrow().clone();
        jsonx::obj(json!({"mods": mods, "key": self.keybtn.key.borrow().clone(), "action": a, "arg": arg,
            "flags": flags}))
    }

    pub fn text(&self) -> String {
        let v = self.value();
        [model::combo_text(&v), t(&format!("act.{}", v.s("action"))), v.s("arg").to_string()].join(" ").to_lowercase()
    }

    pub fn set_warning(&self, text: Option<&str>) {
        self.warn.set_visible(text.is_some());
        self.warn.set_tooltip_text(text);
    }
}

// ---------- Editor ----------

pub struct BindEditor {
    pub widget: gtk::Box,
    pub search: gtk::SearchEntry,
    btn_add: gtk::Button,
    btn_tpl: gtk::Button,
    btn_import: gtk::Button,
    btn_fix: gtk::Button,
    count: gtk::Label,
    rows_box: gtk::Box,
    pub rows: RefCell<Vec<Rc<BindRow>>>,
    /// eigene Kürzel-Sets: (cfg, speichern)
    pub user_store: RefCell<Option<(Rc<RefCell<Obj>>, Rc<dyn Fn()>)>>,
    on_changed: Cb<()>,
    on_fix: Cb<()>,
    on_import: Cb<()>,
}

impl BindEditor {
    pub fn new() -> Rc<BindEditor> {
        let widget = vbox(10);
        let top = hbox(8);
        let btn_add = button("", "");
        let btn_tpl = button("", "");
        let btn_import = button("", "");
        let btn_fix = button("", "Danger");
        for b in [&btn_add, &btn_tpl, &btn_import, &btn_fix] {
            top.append(b);
        }
        top.append(&spacer());
        let count = label("", "RowLabel");
        top.append(&count);
        let search = gtk::SearchEntry::new();
        search.add_css_class("Search");
        search.set_size_request(-1, 40);
        widget.append(&search);
        widget.append(&top);
        let rows_box = vbox(6);
        widget.append(&rows_box);
        btn_fix.set_visible(false);
        let e = Rc::new(BindEditor { widget, search, btn_add, btn_tpl, btn_import, btn_fix, count, rows_box,
            rows: RefCell::new(vec![]), user_store: RefCell::new(None), on_changed: RefCell::new(None),
            on_fix: RefCell::new(None), on_import: RefCell::new(None) });
        let w = Rc::downgrade(&e);
        e.search.connect_search_changed(move |_| {
            if let Some(e) = w.upgrade() {
                e.filter();
            }
        });
        let w = Rc::downgrade(&e);
        e.btn_add.connect_clicked(move |_| {
            if let Some(e) = w.upgrade() {
                e.add_empty();
            }
        });
        let w = Rc::downgrade(&e);
        e.btn_import.connect_clicked(move |_| {
            if let Some(e) = w.upgrade() {
                if let Some(f) = e.on_import.borrow().as_ref() {
                    f(());
                }
            }
        });
        let w = Rc::downgrade(&e);
        e.btn_fix.connect_clicked(move |_| {
            if let Some(e) = w.upgrade() {
                if let Some(f) = e.on_fix.borrow().as_ref() {
                    f(());
                }
            }
        });
        let w = Rc::downgrade(&e);
        e.btn_tpl.connect_clicked(move |_| {
            if let Some(e) = w.upgrade() {
                e.tpl_menu();
            }
        });
        e
    }

    pub fn connect_changed(&self, f: impl Fn(()) + 'static) {
        *self.on_changed.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_fix(&self, f: impl Fn(()) + 'static) {
        *self.on_fix.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_import(&self, f: impl Fn(()) + 'static) {
        *self.on_import.borrow_mut() = Some(Box::new(f));
    }

    fn changed(&self) {
        if let Some(f) = self.on_changed.borrow().as_ref() {
            f(());
        }
    }

    pub fn load(self: &Rc<Self>, binds: &[Value]) {
        for r in self.rows.borrow_mut().drain(..) {
            self.rows_box.remove(&r.widget);
        }
        let mut seen = HashSet::new();
        for b in binds {
            let Some(b) = b.as_object() else { continue };
            let c = model::combo(&b.strs("mods"), b.s("key"));
            if !b.s("key").is_empty() && seen.contains(&c) {
                continue;
            }
            seen.insert(c);
            self.add_row(b, None);
        }
        self.filter();
    }

    /// Neue Kürzel anhängen; schon vorhandene Tastenkombinationen bleiben unverändert.
    pub fn merge(self: &Rc<Self>, binds: Vec<Value>) {
        let mut all = self.read();
        all.extend(binds);
        self.load(&all);
        self.changed();
    }

    fn add_row(self: &Rc<Self>, b: &Obj, after: Option<&Rc<BindRow>>) -> Rc<BindRow> {
        let r = BindRow::new(b);
        let w = Rc::downgrade(self);
        r.on_changed.replace(Some(Box::new(move |_| {
            if let Some(e) = w.upgrade() {
                e.changed();
            }
        })));
        let w = Rc::downgrade(self);
        let wr = Rc::downgrade(&r);
        r.on_removed.replace(Some(Box::new(move |_| {
            if let (Some(e), Some(r)) = (w.upgrade(), wr.upgrade()) {
                e.remove(&r);
            }
        })));
        let w = Rc::downgrade(self);
        let wr = Rc::downgrade(&r);
        r.on_duplicated.replace(Some(Box::new(move |_| {
            if let (Some(e), Some(r)) = (w.upgrade(), wr.upgrade()) {
                e.duplicate(&r);
            }
        })));
        let mut rows = self.rows.borrow_mut();
        match after.and_then(|a| rows.iter().position(|x| Rc::ptr_eq(x, a))) {
            Some(i) => {
                self.rows_box.insert_child_after(&r.widget, Some(&rows[i].widget));
                rows.insert(i + 1, r.clone());
            }
            None => {
                self.rows_box.append(&r.widget);
                rows.push(r.clone());
            }
        }
        r
    }

    pub fn add_empty(self: &Rc<Self>) {
        self.search.set_text("");
        let r = self.add_row(&model::mk_bind(&[], "", "exec", "", &[]), None);
        let k = r.keybtn.clone();
        glib::idle_add_local_once(move || k.start());
        self.changed();
    }

    fn duplicate(self: &Rc<Self>, row: &Rc<BindRow>) {
        let mut v = row.value();
        v.insert("key".into(), json!(""));
        let r = self.add_row(&v, Some(row));
        let k = r.keybtn.clone();
        glib::idle_add_local_once(move || k.start());
        self.changed();
    }

    fn remove(&self, row: &Rc<BindRow>) {
        self.rows.borrow_mut().retain(|x| !Rc::ptr_eq(x, row));
        self.rows_box.remove(&row.widget);
        self.changed();
    }

    fn filter(&self) {
        let q = self.search.text().trim().to_lowercase();
        let words: Vec<&str> = q.split_whitespace().collect();
        for r in self.rows.borrow().iter() {
            let txt = r.text();
            let show = words.is_empty() || words.iter().all(|w| txt.contains(w));
            if r.widget.is_visible() != show {
                r.widget.set_visible(show);
            }
        }
    }

    pub fn read(&self) -> Vec<Value> {
        self.rows.borrow().iter().map(|r| Value::Object(r.value())).collect()
    }

    /// Doppelte Kombinationen und Treffer in hyprland.lua markieren.
    pub fn mark(&self, conflicts: &HashSet<model::Combo>) {
        let rows = self.rows.borrow();
        let vals: Vec<Obj> = rows.iter().map(|r| r.value()).collect();
        let mut counts: HashMap<model::Combo, usize> = HashMap::new();
        for v in &vals {
            if !v.s("key").is_empty() {
                *counts.entry(model::combo(&v.strs("mods"), v.s("key"))).or_default() += 1;
            }
        }
        for (r, v) in rows.iter().zip(&vals) {
            let c = model::combo(&v.strs("mods"), v.s("key"));
            if !v.s("key").is_empty() && counts.get(&c).copied().unwrap_or(0) > 1 {
                r.set_warning(Some(&t("keys.dup")));
            } else if !v.s("key").is_empty() && conflicts.contains(&c) {
                r.set_warning(Some(&t("keys.conflict")));
            } else {
                r.set_warning(None);
            }
        }
        self.btn_fix.set_visible(!conflicts.is_empty());
        let n = vals.iter().filter(|v| model::bind_ok(v)).count();
        self.count.set_text(&tf("keys.count", kw!("n" => n)));
    }

    pub fn retranslate(self: &Rc<Self>) {
        self.btn_add.set_label(&t("keys.add"));
        self.btn_tpl.set_label(&t("keys.templates"));
        self.btn_import.set_label(&t("keys.import"));
        self.btn_import.set_tooltip_text(Some(&t("tip.keys.import")));
        self.btn_fix.set_label(&t("keys.fix"));
        self.btn_fix.set_tooltip_text(Some(&t("tip.keys.fix")));
        self.search.set_placeholder_text(Some(&t("search")));
        let cur = self.read();
        self.load(&cur);
    }

    fn sets(&self) -> Vec<String> {
        let Some((cfg, _)) = self.user_store.borrow().clone() else { return vec![] };
        let c = cfg.borrow();
        let mut names: Vec<String> = c.obj("user_presets").and_then(|u| u.obj("binds"))
            .map(|o| o.keys().cloned().collect()).unwrap_or_default();
        names.sort_by_key(|x| x.to_lowercase());
        names
    }

    fn with_store(&self, f: impl FnOnce(&mut Obj)) {
        let Some((cfg, save)) = self.user_store.borrow().clone() else { return };
        {
            let mut c = cfg.borrow_mut();
            let up = c.entry("user_presets").or_insert_with(|| json!({}));
            if !up.is_object() {
                *up = json!({});
            }
            let st = up.as_object_mut().unwrap().entry("binds").or_insert_with(|| json!({}));
            if !st.is_object() {
                *st = json!({});
            }
            f(st.as_object_mut().unwrap());
        }
        save();
    }

    /// Mitgelieferte Vorlagen (anhängen) + eigene Kürzel-Sets (laden, speichern, entfernen).
    fn tpl_menu(self: &Rc<Self>) {
        let tpls: Vec<String> = model::templates().keys().cloned().collect();
        let sets = self.sets();
        let mut entries: Vec<MenuEntry> = tpls.iter().map(|n| MenuEntry::Item(t(&format!("tpl.{n}")))).collect();
        entries.push(MenuEntry::Item(t("tpl.all")));
        entries.push(MenuEntry::Sep);
        if !sets.is_empty() {
            entries.push(MenuEntry::Header(t("up.mine")));
            for n in &sets {
                entries.push(MenuEntry::Item(tf("up.load", kw!("name" => n))));
            }
        }
        entries.push(MenuEntry::Item(t("up.save_binds")));
        if !sets.is_empty() {
            entries.push(MenuEntry::Item(t("up.remove_set") + " ›"));
        }
        let w = Rc::downgrade(self);
        dialogs::popup_menu(&self.btn_tpl, None, entries, move |i| {
            let Some(e) = w.upgrade() else { return };
            let tp = model::templates();
            if i < tpls.len() {
                e.merge(tp.arr(&tpls[i]).to_vec());
            } else if i == tpls.len() {
                e.merge(tpls.iter().flat_map(|n| tp.arr(n).to_vec()).collect());
            } else if i <= tpls.len() + sets.len() {
                e.load_set(&sets[i - tpls.len() - 1]);
            } else if i == tpls.len() + sets.len() + 1 {
                e.save_set();
            } else {
                e.remove_menu(sets.clone());
            }
        });
    }

    fn remove_menu(self: &Rc<Self>, sets: Vec<String>) {
        let entries = sets.iter().map(|n| MenuEntry::Item(n.clone())).collect();
        let w = Rc::downgrade(self);
        dialogs::popup_menu(&self.btn_tpl, None, entries, move |i| {
            let Some(e) = w.upgrade() else { return };
            let name = sets[i].clone();
            let w2 = Rc::downgrade(&e);
            dialogs::confirm(shell::toplevel_of(&e.widget).as_ref(), &t("up.remove_set"),
                &tf("up.remove_q", kw!("name" => name)), Some(&t("up.remove")), true, move |ok| {
                    if let (true, Some(e)) = (ok, w2.upgrade()) {
                        e.with_store(|m| {
                            m.shift_remove(&name);
                        });
                    }
                });
        });
    }

    fn save_set(self: &Rc<Self>) {
        let w = Rc::downgrade(self);
        dialogs::ask_text(shell::toplevel_of(&self.widget).as_ref(), &t("up.save_binds"), &t("up.name"), "",
            move |name| {
                let (Some(e), Some(name)) = (w.upgrade(), name) else { return };
                let name = name.trim().to_string();
                if name.is_empty() {
                    return;
                }
                let binds = model::clean_binds(&Value::Array(e.read()));
                e.with_store(|m| {
                    m.insert(name, Value::Array(binds));
                });
            });
    }

    fn load_set(self: &Rc<Self>, name: &str) {
        let Some((cfg, _)) = self.user_store.borrow().clone() else { return };
        let v = cfg.borrow().obj("user_presets").and_then(|u| u.obj("binds")).map(|o| o.val(name).clone())
            .unwrap_or(Value::Null);
        self.load(&model::clean_binds(&v));
        self.changed();
    }
}

#[allow(dead_code)]
fn _unused(_: Weak<()>, _: draw::Rgba) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        assert_eq!(parse_combo("Strg + Shift + S"), (vec!["SHIFT".to_string(), "CTRL".to_string()], "S".to_string()));
        assert_eq!(parse_combo("SUPER SHIFT, S"), (vec!["SUPER".to_string(), "SHIFT".to_string()], "S".to_string()));
        assert_eq!(parse_combo(""), (vec![], String::new()));
    }
}
