//! Einstellungen › Erweitert: yay mit einem Klick, der Kitty-Konfigurator (aus Hypr Studio hierher umgezogen) und
//! ein Alacritty-Konfigurator im selben Stil.

use super::form::Page;
use super::widgets::{reset_button, set_applied};
use super::win::{win, AioPage};
use crate::i18n::{t, tf};
use crate::{alacritty, usercmd, yay};
use gtk::prelude::*;
use hyprstudio::jsonx::Obj;
use hyprstudio::kw;
use hyprstudio::ui::kitty_page::{kitty_program, KittyEditor, Program, TermWin};
use hyprstudio::ui::shell::{self, Conf};
use hyprstudio::ui::widgets::{busy_bar, cursor_hand, hbox, label, vbox};
use serde_json::json;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

pub const SUBS: [&str; 3] = ["kitty", "alacritty", "yay"];

/// Texte der Alacritty-Karten auch in Hypr Studios Wörterbuch (seine Karten übersetzen sich selbst).
fn hs_texts() {
    let src = crate::i18n::TEXT.text.read().unwrap().clone();
    let mut dst = hyprstudio::i18n::TEXT.text.write().unwrap();
    for (lang, d) in src {
        let e = dst.entry(lang).or_default();
        for (k, v) in d {
            if k.starts_with("ala.") {
                e.insert(k, v);
            }
        }
    }
}

/// „nicht installiert“ + Installieren-Knopf (Root-Helfer, fester Paket-Schlüssel).
struct InstallRow {
    root: gtk::Box,
    lb: gtk::Label,
    btn: gtk::Button,
    key: String,
    pkg: String,
}

impl InstallRow {
    fn new(key: &str, pkg: &str) -> InstallRow {
        let root = hbox(8);
        root.add_css_class("ACard");
        root.add_css_class("AInst");
        let lb = label("", "");
        lb.set_wrap(true);
        lb.set_xalign(0.0);
        lb.set_hexpand(true);
        let btn = gtk::Button::with_label("");
        btn.add_css_class("Primary");
        btn.set_valign(gtk::Align::Center);
        cursor_hand(&btn);
        root.append(&lb);
        root.append(&btn);
        InstallRow { root, lb, btn, key: key.into(), pkg: pkg.into() }
    }

    fn retr(&self) {
        self.lb.set_text(&t(&format!("{}.missing", self.key)));
        self.btn.set_label(&t("install"));
    }

    fn check(&self) {
        self.root.set_visible(!usercmd::have(&self.pkg));
    }
}

// ---------- yay ----------

pub struct YayPage {
    page: Rc<Page>,
    desc: gtk::Label,
    state: gtk::Label,
    btn: gtk::Button,
    bar: gtk::ProgressBar,
    step: gtk::Label,
    busy: Cell<bool>,
    next: RefCell<String>,
    me: RefCell<Weak<YayPage>>,
}

impl YayPage {
    pub fn new() -> Rc<YayPage> {
        let page = Page::new("adv.yay", true);
        let card = page.card("adv.yay.card");
        let desc = label("", "RowLabel");
        desc.set_wrap(true);
        desc.set_xalign(0.0);
        card.append(&desc);
        let row = hbox(8);
        row.set_margin_top(10);
        let state = label("", "");
        state.set_hexpand(true);
        state.set_xalign(0.0);
        let btn = gtk::Button::with_label("");
        btn.add_css_class("Primary");
        btn.set_size_request(170, -1);
        cursor_hand(&btn);
        row.append(&state);
        row.append(&btn);
        card.append(&row);
        let bar = busy_bar(-1);
        bar.set_hexpand(true);
        card.append(&bar);
        let step = label("", "Label");
        step.set_xalign(0.0);
        card.append(&step);
        page.finish();
        let p = Rc::new(YayPage { page, desc, state, btn, bar, step, busy: Cell::new(false),
            next: RefCell::new(String::new()), me: RefCell::new(Weak::new()) });
        *p.me.borrow_mut() = Rc::downgrade(&p);
        let w = Rc::downgrade(&p);
        p.btn.connect_clicked(move |_| {
            if let Some(p) = w.upgrade() {
                p.install();
            }
        });
        p
    }

    fn me_dyn(&self) -> Option<Rc<dyn AioPage>> {
        self.me.borrow().upgrade().map(|p| p as Rc<dyn AioPage>)
    }

    fn set_busy(&self, on: bool, step: &str) {
        self.busy.set(on);
        self.bar.set_visible(on);
        self.step.set_text(step);
        self.btn.set_sensitive(!on);
    }

    fn install(&self) {
        if self.busy.get() {
            return;
        }
        let w = win();
        w.log("");
        if !yay::deps_missing().is_empty() {
            // git/base-devel zuerst
            self.set_busy(true, &t("adv.yay.s1"));
            *self.next.borrow_mut() = "build".into();
            w.run(vec!["pkg-install".into(), "yay-deps".into()], "pkg-install", self.me_dyn());
            return;
        }
        self.build();
    }

    fn build(&self) {
        self.set_busy(true, &t("adv.yay.s2"));
        let wm = self.me.borrow().clone();
        shell::run_job(|log| yay::build(log), move |res| {
            let Some(p) = wm.upgrade() else { return };
            let w = win();
            match res {
                Err(e) => {
                    p.set_busy(false, "");
                    w.log(&e);
                    w.set_status(&format!("{}: {}", t("failed"), e));
                    p.refresh();
                }
                Ok(path) => {
                    p.set_busy(true, &t("adv.yay.s3"));
                    *p.next.borrow_mut() = "end".into();
                    w.run(vec!["yay-install".into(), path.to_string_lossy().into_owned()], "yay-install", p.me_dyn());
                }
            }
        });
    }
}

impl AioPage for YayPage {
    fn widget(&self) -> gtk::Widget {
        self.page.root.clone().upcast()
    }

    fn retranslate(&self) {
        self.page.retranslate();
        self.desc.set_text(&t("adv.yay.desc"));
        self.refresh();
    }

    fn refresh(&self) {
        if !yay::supported() {
            self.state.set_text(&t("adv.yay.arch"));
            self.btn.set_visible(false);
            return;
        }
        let inst = yay::installed();
        self.state.set_text(&if inst { t("adv.yay.ok") } else { t("adv.yay.no") });
        set_applied(&self.btn, inst, &t("installed"));
        if !inst {
            self.btn.set_label(&t("install"));
            self.btn.set_sensitive(!self.busy.get());
        }
    }

    fn finished(&self, ok: bool, action: &str, _result: &str) {
        let nxt = std::mem::take(&mut *self.next.borrow_mut());
        if ok && nxt == "build" {
            self.build();
            return;
        }
        self.set_busy(false, "");
        if ok && action == "yay-install" {
            win().set_status(&t("adv.yay.done"));
        }
        self.refresh();
    }
}

// ---------- Kitty / Alacritty (Hypr Studios Editor) ----------

/// Was Hypr Studios Editor vom Fenster braucht – von Linux AIO bedient.
struct KittyHost;

impl TermWin for KittyHost {
    fn conf(&self) -> Conf {
        win().conf()
    }
    fn log(&self, text: &str) {
        win().log(text)
    }
    fn set_status(&self, text: &str) {
        win().set_status(text)
    }
    fn report(&self, ok: bool, info: &str) {
        win().set_status(&if ok { t("done") } else { format!("{}: {}", t("failed"), hyprstudio::i18n::t(info)) });
    }
    fn hypr_state(&self) -> Option<Obj> {
        None
    }
}

fn ala_view(s: &Obj) -> Obj {
    let mut v = s.clone();
    v.insert("tab_style".into(), json!("hidden"));
    v.insert("trail".into(), json!(0));
    v.insert("_title".into(), json!("alacritty"));
    v
}

fn ala_extra(e: &Rc<KittyEditor>, key: &str, lay: &gtk::Box) {
    if key == "cursor" {
        // nur Alacritty: Strichstärke, Blinken endet nach …
        let c = e.card(lay, "ala.cursor_more");
        c.slider("thickness", 0.05, 0.5, 0.05, "{:.2f}", None, Some("ala.thickness"));
        c.slider("blink_timeout", 0.0, 30.0, 1.0, "{:.0f} s", Some("ala.blink_timeout.tip"), Some("ala.blink_timeout"));
    }
}

fn ala_after(_e: &KittyEditor) {
    win().set_status(&tf("ala.applied", kw!("path" => alacritty::target().to_string_lossy())));
}

/// Kittys Editor 1:1 (gleiche Presets, Bereiche, Vorschau, Kacheln), geschrieben als alacritty.toml.
/// Was Alacritty nicht kann, bleibt sichtbar, aber ausgegraut: „Von Alacritty nicht unterstützt“.
fn alacritty_program() -> Program {
    Program { cfg: "alacritty_k", program: "alacritty", make_state: alacritty::state,
        write: Box::new(|s, log| {
            alacritty::apply(s, log);
            (true, String::new())
        }),
        restore: Box::new(|log| alacritty::undo(log)), view: ala_view, extra: Some(ala_extra), after_apply: Some(ala_after) }
}

/// Kitty und Alacritty: dieselbe Seite (Titel, Installieren-Hinweis, Hypr Studios Terminal-Editor).
pub struct TerminalPage {
    root: gtk::Box,
    program: String,
    title: gtk::Label,
    b_reset: gtk::Button,
    missing: InstallRow,
    editor: Rc<KittyEditor>,
    me: RefCell<Weak<TerminalPage>>,
}

impl TerminalPage {
    pub fn new(program: &str) -> Rc<TerminalPage> {
        hyprstudio::i18n::set_lang(&crate::i18n::lang());
        if program != "kitty" {
            hs_texts();
        }
        let root = vbox(12);
        let trow = hbox(6);
        let title = label("", "APageTitle");
        trow.append(&title);
        let b_reset = reset_button();
        trow.append(&b_reset);
        root.append(&trow);
        let missing = InstallRow::new(&format!("adv.{program}"), program);
        root.append(&missing.root);
        let prog = if program == "kitty" { kitty_program() } else { alacritty_program() };
        let editor = KittyEditor::new(Rc::new(KittyHost), prog);
        editor.btn_from.set_visible(false); // „Farbe aus Hyprland“ gibt es hier nicht
        editor.root.set_vexpand(true);
        root.append(&editor.root);
        let p = Rc::new(TerminalPage { root, program: program.into(), title, b_reset, missing, editor,
            me: RefCell::new(Weak::new()) });
        *p.me.borrow_mut() = Rc::downgrade(&p);
        let w = Rc::downgrade(&p);
        p.missing.btn.connect_clicked(move |_| {
            if let Some(p) = w.upgrade() {
                let pkg = p.missing.pkg.clone();
                win().run(vec!["pkg-install".into(), pkg], "pkg-install", Some(p.clone() as Rc<dyn AioPage>));
            }
        });
        let w = Rc::downgrade(&p);
        p.b_reset.connect_clicked(move |_| {
            if let Some(p) = w.upgrade() {
                p.reset_section();
            }
        });
        p
    }

    /// Nur die offene Unterseite (Presets, Aussehen, Schrift …) auf Standard.
    fn reset_section(self: &Rc<Self>) {
        let sub = self.editor.current_sub();
        let title = format!("{} › {}", t(&format!("adv.{}", self.program)), hyprstudio::i18n::t(&format!("ks.{sub}")));
        let me = self.clone();
        let kind = format!("{}-{}", self.program, sub);
        hyprstudio::ui::dialogs::confirm(win().parent().as_ref(), &t("reset.title"),
            &tf("reset.ask", kw!("page" => title.clone())), Some(&t("reset.btn")), true, move |ok| {
                if !ok {
                    return;
                }
                let dir = usercmd::real_home().join(".config/LabtosoStudios/backups")
                    .join(hyprstudio::util::strftime("%Y-%m-%d_%H%M%S"));
                let _ = std::fs::create_dir_all(&dir);
                let path = dir.join(format!("{kind}.json"));
                let st = serde_json::Value::Object(me.editor.state.borrow().clone());
                let _ = std::fs::write(&path, crate::pyjson::dumps(&st, Some(1), false, false));
                me.editor.reset_keys(&sub);
                win().set_status(&hyprstudio::i18n::tf("reset.section",
                    kw!("page" => title, "path" => path.to_string_lossy())));
            });
    }

    fn mark_unsupported(&self) {
        if self.program != "alacritty" {
            return;
        }
        for card in self.editor.cards.borrow().iter() {
            for r in card.rows.borrow().iter() {
                if !alacritty::unsupported(&r.skey) {
                    continue;
                }
                r.label.set_text(&format!("{}  ·  {}", r.label.text(), t("ala.unsupported")));
                r.label.set_tooltip_text(Some(&t("ala.unsupported")));
                for w in &r.widgets {
                    w.set_sensitive(false);
                }
            }
        }
    }
}

impl AioPage for TerminalPage {
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }

    fn retranslate(&self) {
        hyprstudio::i18n::set_lang(&crate::i18n::lang());
        self.title.set_text(&t(&format!("adv.{}", self.program)));
        self.b_reset.set_tooltip_text(Some(&t("reset.tip")));
        self.missing.retr();
        self.missing.check();
        self.editor.retranslate();
        self.mark_unsupported();
        self.editor.btn_from.set_visible(false);
    }

    fn refresh(&self) {
        self.missing.check();
    }

    fn finished(&self, _ok: bool, _action: &str, _result: &str) {
        self.refresh();
    }
}

pub fn make(key: &str) -> Rc<dyn AioPage> {
    if key == "yay" {
        YayPage::new()
    } else {
        TerminalPage::new(key)
    }
}
