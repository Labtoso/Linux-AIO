//! Einstellungen (links einfahrendes Panel): Sprache, Akzentfarbe, Alles zurücksetzen, Erweitert (Config-Format),
//! ganz unten Version und der Link zum Impressum (öffnet im Browser).
//! Dazu die Bestätigung für „Alles zurücksetzen“.

use super::dialogs::Frame;
use super::widgets::{button, hbox, label, vbox, vspacer, Cb, Segmented};
use crate::i18n::{t, tf};
use crate::jsonx::{Get, Obj};
use crate::{kw, model, palettes};
use gtk::prelude::*;
use std::rc::Rc;

pub struct SettingsPanel {
    pub root: gtk::Box,
    pub pages: gtk::Stack,
    title: gtk::Label,
    l_lang: gtk::Label,
    lang_btns: Vec<(String, gtk::ToggleButton)>,
    l_theme: gtk::Label,
    theme_btns: Vec<(String, gtk::Button)>,
    l_reset: gtk::Label,
    btn_reset: gtk::Button,
    btn_adv: gtk::Button,
    info: gtk::Label,
    btn_back: gtk::Button,
    adv_title: gtk::Label,
    l_fmt: gtk::Label,
    pub seg_fmt: Rc<Segmented>,
    fmt_state: gtk::Label,
    pub on_lang: Cb<String>,
    pub on_theme: Cb<String>,
    pub on_reset: Cb<()>,
    pub on_format: Cb<String>,
    css: gtk::CssProvider,
}

impl SettingsPanel {
    pub fn new() -> Rc<SettingsPanel> {
        let root = vbox(0);
        root.add_css_class("Panel");
        root.set_size_request(300, -1);
        root.set_halign(gtk::Align::Start);
        let pages = gtk::Stack::new();
        pages.set_vexpand(true);
        root.append(&pages);
        let main = vbox(12);
        super::widgets::margins(&main, 22, 22, 22, 22);
        let title = label("", "PanelTitle");
        title.set_margin_bottom(10);
        main.append(&title);
        let l_lang = label("", "Label");
        main.append(&l_lang);
        let row = hbox(6);
        let mut lang_btns = vec![];
        let mut first: Option<gtk::ToggleButton> = None;
        for (code, name) in [("de", "Deutsch"), ("en", "English")] {
            let b = gtk::ToggleButton::with_label(name);
            b.add_css_class("Lang");
            super::widgets::cursor_hand(&b);
            if let Some(f) = &first {
                b.set_group(Some(f));
            } else {
                first = Some(b.clone());
            }
            b.set_hexpand(true);
            row.append(&b);
            lang_btns.push((code.to_string(), b));
        }
        row.set_margin_bottom(10);
        main.append(&row);
        let l_theme = label("", "Label");
        main.append(&l_theme);
        let grid = gtk::Grid::new();
        grid.set_row_spacing(10);
        grid.set_column_spacing(10);
        let mut theme_btns = vec![];
        for (i, (name, _)) in palettes::themes().into_iter().enumerate() {
            let b = gtk::Button::new();
            b.add_css_class("Accent");
            b.add_css_class(&format!("acc-{}", css_name(&name)));
            b.set_size_request(38, 38);
            super::widgets::cursor_hand(&b);
            b.set_tooltip_text(Some(&palettes::title(&name)));
            grid.attach(&b, (i % 5) as i32, (i / 5) as i32, 1, 1);
            theme_btns.push((name, b));
        }
        main.append(&grid);
        main.append(&vspacer());
        let l_reset = label("", "Label");
        main.append(&l_reset);
        let btn_reset = button("", "DangerFill");
        btn_reset.set_size_request(-1, 40);
        main.append(&btn_reset);
        let btn_adv = button("", "Outline");
        btn_adv.set_size_request(-1, 40);
        btn_adv.set_margin_top(8);
        main.append(&btn_adv);
        let info = label("", "Label");
        info.set_wrap(true);
        info.set_margin_top(14);
        main.append(&info);
        let imprint = super::widgets::ext_link("Impressum", crate::IMPRINT_URL);
        imprint.set_margin_top(4);
        main.append(&imprint);
        pages.add_named(&main, Some("main"));
        // Erweiterte Einstellungen: zweite Seite im selben Panel
        let adv = vbox(12);
        super::widgets::margins(&adv, 22, 22, 22, 22);
        let btn_back = button("", "");
        btn_back.set_halign(gtk::Align::Start);
        adv.append(&btn_back);
        let adv_title = label("", "PanelTitle");
        adv_title.set_wrap(true);
        adv_title.set_margin_bottom(10);
        adv.append(&adv_title);
        let l_fmt = label("", "Label");
        adv.append(&l_fmt);
        let seg_fmt = Segmented::new(&["conf", "lua"], "fmtsw.");
        adv.append(&seg_fmt.root);
        let fmt_state = label("", "Label");
        fmt_state.set_wrap(true);
        adv.append(&fmt_state);
        adv.append(&vspacer());
        pages.add_named(&adv, Some("adv"));
        let css = gtk::CssProvider::new();
        if let Some(d) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(&d, &css, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 2);
        }
        let p = Rc::new(SettingsPanel { root, pages, title, l_lang, lang_btns, l_theme, theme_btns, l_reset, btn_reset,
            btn_adv, info, btn_back, adv_title, l_fmt, seg_fmt, fmt_state, on_lang: Default::default(),
            on_theme: Default::default(), on_reset: Default::default(), on_format: Default::default(), css });
        for (code, b) in &p.lang_btns {
            let w = Rc::downgrade(&p);
            let c = code.clone();
            b.connect_clicked(move |_| {
                if let Some(p) = w.upgrade() {
                    if let Some(f) = p.on_lang.borrow().as_ref() {
                        f(c.clone());
                    }
                }
            });
        }
        for (name, b) in &p.theme_btns {
            let w = Rc::downgrade(&p);
            let n = name.clone();
            b.connect_clicked(move |_| {
                if let Some(p) = w.upgrade() {
                    if let Some(f) = p.on_theme.borrow().as_ref() {
                        f(n.clone());
                    }
                }
            });
        }
        let w = Rc::downgrade(&p);
        p.btn_reset.connect_clicked(move |_| {
            if let Some(p) = w.upgrade() {
                if let Some(f) = p.on_reset.borrow().as_ref() {
                    f(());
                }
            }
        });
        let pg = p.pages.clone();
        p.btn_adv.connect_clicked(move |_| pg.set_visible_child_name("adv"));
        let pg = p.pages.clone();
        p.btn_back.connect_clicked(move |_| pg.set_visible_child_name("main"));
        let w = Rc::downgrade(&p);
        p.seg_fmt.connect(move |v| {
            if let Some(p) = w.upgrade() {
                if let Some(f) = p.on_format.borrow().as_ref() {
                    f(v);
                }
            }
        });
        p
    }

    pub fn sync(&self, cfg: &Obj, info: &str) {
        self.title.set_text(&t("settings"));
        self.l_lang.set_text(&t("language"));
        self.l_theme.set_text(&t("theme"));
        self.l_reset.set_text(&t("reset.btn"));
        self.btn_reset.set_label(&t("reset.button"));
        self.btn_reset.set_tooltip_text(Some(&t("reset.tip")));
        self.btn_adv.set_label(&t("adv.button"));
        self.btn_back.set_label(&format!("‹  {}", t("adv.back")));
        self.adv_title.set_text(&t("adv.button"));
        self.l_fmt.set_text(&t("adv.format"));
        self.seg_fmt.retranslate();
        for k in ["conf", "lua"] {
            if let Some(b) = self.seg_fmt.btn(k) {
                b.set_tooltip_text(Some(&t(&format!("tip.fmtsw.{k}"))));
            }
        }
        let hdir = model::config_home().join("hypr");
        self.seg_fmt.select(&model::hypr_format(&hdir).unwrap_or_else(|| "lua".into()));
        let found = model::detect_format(&hdir).unwrap_or_else(|| "none".into());
        self.fmt_state.set_text(&tf("adv.found", kw!("name" => t(&format!("fmt.{found}")))));
        if let Some((_, b)) = self.lang_btns.iter().find(|(c, _)| c == cfg.s("lang")) {
            b.set_active(true);
        }
        let cur = cfg.s("theme");
        let mut css = String::new();
        for (name, b) in &self.theme_btns {
            b.set_tooltip_text(Some(&t(&format!("accent.{name}"))));
            let ring = if name == cur {
                if name == "01" { super::theme::accent_of("tokyo") } else { "#ffffff".into() }
            } else {
                super::theme::SURFACE.to_string()
            };
            css.push_str(&format!("button.Accent.acc-{} {{ background: {}; border: 3px solid {}; border-radius: 19px; \
                padding: 0; min-width: 0; min-height: 0; }}\n", css_name(name), super::theme::accent_of(name), ring));
        }
        self.css.load_from_string(&css);
        self.info.set_text(&format!("{info}\nv{} · LabtosoStudios", crate::VERSION));
    }
}

fn css_name(n: &str) -> String {
    n.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect()
}

/// Bestätigung für „Alles zurücksetzen“: Knopf erst aktiv, wenn der Haken gesetzt ist.
/// `done(Some(factory))` bei Bestätigung.
pub fn reset_dialog(parent: Option<&gtk::Window>, lua_ok: bool, done: impl FnOnce(Option<bool>) + 'static) {
    let d = Frame::new(parent, &t("reset.title"), 520);
    let text = label(&t("reset.text"), "");
    text.set_wrap(true);
    text.set_max_width_chars(60);
    d.body.append(&text);
    let factory = gtk::CheckButton::with_label(&t("reset.factory"));
    factory.set_sensitive(lua_ok);
    factory.set_tooltip_text(Some(&if lua_ok { t("reset.factory_tip") } else { t("reset.factory_off") }));
    d.body.append(&factory);
    let sure = gtk::CheckButton::with_label(&t("reset.sure"));
    sure.add_css_class("Sure");
    sure.set_margin_top(6);
    d.body.append(&sure);
    let ok = d.buttons(&t("reset.button"), "DangerFill", true);
    ok.set_sensitive(false);
    let ok2 = ok.clone();
    sure.connect_toggled(move |s| ok2.set_sensitive(s.is_active()));
    let f2 = factory.clone();
    d.run(move |yes| done(yes.then(|| f2.is_active())));
}
