//! Hyprland › Arbeitsflächen: eine Zeile pro Fläche 1–10 (Name, Monitor, Lücken, Rahmen, immer da, Start-Fläche).

use super::dialogs::{self, MenuEntry};
use super::keys::EditCombo;
use super::shell;
use super::theme;
use super::widgets::{hbox, label, set_tool_icon, spacer, tool_button, vbox, Cb};
use crate::i18n::t;
use crate::jsonx::{self, Get, Obj};
use crate::model;
use gtk::prelude::*;
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub fn monitors() -> Vec<String> {
    if model::dry() {
        return vec!["DP-1".into(), "HDMI-A-1".into(), "eDP-1".into()];
    }
    if !crate::util::has("hyprctl") {
        return vec![];
    }
    let out = crate::util::output(&["hyprctl", "-j", "monitors"], 5);
    let v = jsonx::parse(&out);
    v.as_array().map(|a| a.iter().filter_map(|m| m.get("name").and_then(|n| n.as_str()))
        .filter(|n| !n.is_empty()).map(String::from).collect()).unwrap_or_default()
}

/// Zahlenfeld: leer = Standard (-1).
fn num_field(hi: i64) -> gtk::Entry {
    let e = gtk::Entry::new();
    e.set_placeholder_text(Some(&t("ws.std")));
    e.set_width_chars(4);
    e.set_size_request(84, -1);
    EditableExt::set_alignment(&e, 0.5);
    e.set_input_purpose(gtk::InputPurpose::Digits);
    e.connect_changed(move |e| {
        let txt = e.text().to_string();
        let mut digits: String = txt.chars().filter(|c| c.is_ascii_digit()).collect();
        while !digits.is_empty() && digits.parse::<i64>().map(|v| v > hi).unwrap_or(true) {
            digits.pop();
        }
        if digits != txt {
            e.set_text(&digits);
        }
    });
    e
}

fn num_value(e: &gtk::Entry) -> i64 {
    e.text().trim().parse::<i64>().unwrap_or(-1)
}

fn set_num(e: &gtk::Entry, v: i64) {
    e.set_text(&if v < 0 { String::new() } else { v.to_string() });
}

pub struct WsRow {
    pub widget: gtk::Box,
    pub w: RefCell<Obj>,
    name: gtk::Entry,
    mon: Rc<EditCombo>,
    gi: gtk::Entry,
    go: gtk::Entry,
    bs: gtk::Entry,
    pers: gtk::CheckButton,
    dflt: gtk::CheckButton,
    more: gtk::Button,
    quiet: Cell<bool>,
    on_changed: Cb<()>,
}

impl WsRow {
    pub fn new(w: &Obj, mons: &[String]) -> Rc<WsRow> {
        let widget = hbox(8);
        widget.add_css_class("BindRow");
        widget.add_css_class("WsRow");
        let num = label(&w.i("id").to_string(), "Mono");
        num.set_size_request(22, -1);
        widget.append(&num);
        let name = gtk::Entry::new();
        name.set_text(w.s("name"));
        name.set_placeholder_text(Some(&t("ws.name_ph")));
        name.set_size_request(150, -1);
        let mon = EditCombo::new();
        let mut items = vec![String::new()];
        items.extend(mons.iter().cloned());
        mon.set_items(items);
        mon.entry.set_text(w.s("monitor"));
        mon.entry.set_placeholder_text(Some(&t("ws.any")));
        mon.widget.set_size_request(140, -1);
        mon.entry.set_width_chars(6);
        let (gi, go, bs) = (num_field(60), num_field(60), num_field(10));
        set_num(&gi, w.i("gaps_in"));
        set_num(&go, w.i("gaps_out"));
        set_num(&bs, w.i("border"));
        let pers = gtk::CheckButton::with_label(&t("ws.persistent"));
        pers.set_active(w.b("persistent"));
        pers.set_tooltip_text(Some(&t("tip.ws.persistent")));
        let dflt = gtk::CheckButton::with_label(&t("ws.default"));
        dflt.set_active(w.b("default"));
        dflt.set_tooltip_text(Some(&t("tip.ws.default")));
        name.set_tooltip_text(Some(&t("ws.name")));
        mon.widget.set_tooltip_text(Some(&t("ws.monitor")));
        gi.set_tooltip_text(Some(&t("gaps_in")));
        go.set_tooltip_text(Some(&t("gaps_out")));
        bs.set_tooltip_text(Some(&t("border_size")));
        widget.append(&name);
        widget.append(&mon.widget);
        widget.append(&gi);
        widget.append(&go);
        widget.append(&bs);
        widget.append(&pers);
        widget.append(&dflt);
        widget.append(&spacer());
        let more = tool_button("dots", theme::MUTED, 16, "");
        widget.append(&more);
        let r = Rc::new(WsRow { widget, w: RefCell::new(w.clone()), name, mon, gi, go, bs, pers, dflt, more,
            quiet: Cell::new(false), on_changed: RefCell::new(None) });
        let emit = {
            let wk = Rc::downgrade(&r);
            move || {
                if let Some(r) = wk.upgrade() {
                    r.emit();
                }
            }
        };
        let e = emit.clone();
        r.name.connect_changed(move |_| e());
        let e = emit.clone();
        r.mon.entry.connect_changed(move |_| e());
        for f in [&r.gi, &r.go, &r.bs] {
            let e = emit.clone();
            f.connect_changed(move |_| e());
        }
        let e = emit.clone();
        r.pers.connect_toggled(move |_| e());
        let e = emit;
        r.dflt.connect_toggled(move |_| e());
        let wk = Rc::downgrade(&r);
        r.more.connect_clicked(move |_| {
            if let Some(r) = wk.upgrade() {
                r.menu();
            }
        });
        r.dots();
        r
    }

    fn menu(self: &Rc<Self>) {
        let (round, layout) = {
            let w = self.w.borrow();
            (w.b("no_rounding"), w.s("layout").to_string())
        };
        let entries = vec![
            MenuEntry::Check(t("ws.no_rounding"), round),
            MenuEntry::Header(t("ws.layout")),
            MenuEntry::Check(t("ws.layout_std"), layout.is_empty()),
            MenuEntry::Check(t("layout.dwindle"), layout == "dwindle"),
            MenuEntry::Check(t("layout.master"), layout == "master"),
            MenuEntry::Sep,
            MenuEntry::Item(t("ws.on_empty")),
        ];
        let wk = Rc::downgrade(self);
        dialogs::popup_menu(&self.more, None, entries, move |i| {
            let Some(r) = wk.upgrade() else { return };
            match i {
                0 => {
                    r.w.borrow_mut().insert("no_rounding".into(), json!(!round));
                    r.emit();
                }
                1..=3 => {
                    let k = ["", "dwindle", "master"][i - 1];
                    r.w.borrow_mut().insert("layout".into(), json!(k));
                    r.emit();
                }
                _ => {
                    let cur = r.w.borrow().s("on_empty").to_string();
                    let wk2 = Rc::downgrade(&r);
                    dialogs::ask_text(shell::toplevel_of(&r.widget).as_ref(), &t("ws.on_empty"), &t("tip.ws.on_empty"),
                        &cur, move |txt| {
                            if let (Some(r), Some(txt)) = (wk2.upgrade(), txt) {
                                r.w.borrow_mut().insert("on_empty".into(), json!(txt.trim()));
                                r.emit();
                            }
                        });
                }
            }
        });
    }

    fn dots(&self) {
        let w = self.w.borrow();
        let extra = w.b("no_rounding") || !w.s("layout").is_empty() || !w.s("on_empty").is_empty();
        let color = if extra { theme::accent_of("tokyo") } else { theme::MUTED.to_string() };
        set_tool_icon(&self.more, "dots", &color, 16);
        let mut tips = vec![];
        if w.b("no_rounding") {
            tips.push(t("ws.no_rounding"));
        }
        if !w.s("layout").is_empty() {
            tips.push(t(&format!("layout.{}", w.s("layout"))));
        }
        if !w.s("on_empty").is_empty() {
            tips.push(w.s("on_empty").to_string());
        }
        let tip = if tips.is_empty() { t("ws.more") } else { tips.join(", ") };
        self.more.set_tooltip_text(Some(&tip));
    }

    fn emit(&self) {
        {
            let mut w = self.w.borrow_mut();
            w.insert("name".into(), json!(self.name.text().trim()));
            w.insert("monitor".into(), json!(self.mon.entry.text().trim()));
            w.insert("gaps_in".into(), json!(num_value(&self.gi)));
            w.insert("gaps_out".into(), json!(num_value(&self.go)));
            w.insert("border".into(), json!(num_value(&self.bs)));
            w.insert("persistent".into(), json!(self.pers.is_active()));
            w.insert("default".into(), json!(self.dflt.is_active()));
        }
        self.dots();
        if self.quiet.get() {
            return;
        }
        if let Some(f) = self.on_changed.borrow().as_ref() {
            f(());
        }
    }
}

pub struct WorkspaceEditor {
    pub widget: gtk::Box,
    heads: Vec<(gtk::Label, &'static str)>,
    rows: RefCell<Vec<Rc<WsRow>>>,
    loading: Cell<bool>,
    on_changed: Cb<()>,
}

impl WorkspaceEditor {
    pub fn new() -> Rc<WorkspaceEditor> {
        let widget = vbox(6);
        let head = hbox(8);
        head.set_margin_start(12);
        head.set_margin_end(8);
        let mut heads = vec![];
        for (key, w) in [("", 22), ("ws.h_name", 150), ("ws.h_monitor", 140), ("gaps_in", 84), ("gaps_out", 84),
                         ("border_size", 84)] {
            let lb = label(&if key.is_empty() { String::new() } else { t(key) }, "RowLabel");
            lb.set_size_request(w, -1);
            lb.set_ellipsize(gtk::pango::EllipsizeMode::End);
            head.append(&lb);
            heads.push((lb, key));
        }
        widget.append(&head);
        Rc::new(WorkspaceEditor { widget, heads, rows: RefCell::new(vec![]), loading: Cell::new(false),
            on_changed: RefCell::new(None) })
    }

    pub fn connect_changed(&self, f: impl Fn(()) + 'static) {
        *self.on_changed.borrow_mut() = Some(Box::new(f));
    }

    pub fn retranslate(self: &Rc<Self>, items: &[Value]) {
        for (lb, key) in &self.heads {
            if !key.is_empty() {
                lb.set_text(&t(key));
            }
        }
        self.load(items);
    }

    pub fn load(self: &Rc<Self>, items: &[Value]) {
        self.loading.set(true);
        for r in self.rows.borrow_mut().drain(..) {
            self.widget.remove(&r.widget);
        }
        let mons = monitors();
        for w in model::default_workspaces() {
            let id = w.get("id").and_then(|x| x.as_i64()).unwrap_or(0);
            let have = items.iter().filter_map(|x| x.as_object()).find(|x| x.i("id") == id);
            let r = WsRow::new(have.unwrap_or(w.as_object().unwrap()), &mons);
            let wk = Rc::downgrade(self);
            r.on_changed.replace(Some(Box::new(move |_| {
                if let Some(e) = wk.upgrade() {
                    if !e.loading.get() {
                        if let Some(f) = e.on_changed.borrow().as_ref() {
                            f(());
                        }
                    }
                }
            })));
            self.widget.append(&r.widget);
            self.rows.borrow_mut().push(r);
        }
        self.loading.set(false);
    }

    pub fn read(&self) -> Vec<Value> {
        let used: Vec<Value> = self.rows.borrow().iter().map(|r| r.w.borrow().clone())
            .filter(model::ws_used).map(Value::Object).collect();
        model::clean_workspaces(&Value::Array(used))
    }
}
