//! Tux › Einstellungen (links einfahrendes Panel): Sprache, Farbe, Oberflächengröße, Hypr Studio,
//! Update-Hinweis, Preset-Datei, Updates, Über.

use super::widgets::seg;
use super::win::win;
use crate::i18n::t;
use crate::system;
use gtk::prelude::*;
use hyprstudio::jsonx::{truthy, Get, Obj};
use hyprstudio::palettes;
use hyprstudio::ui::theme;
use hyprstudio::ui::widgets::{cursor_hand, hbox, icon_button, label, set_button_text, vbox, vspacer, Segmented};
use serde_json::json;
use std::rc::Rc;

pub struct Panel {
    pub root: gtk::Box,
    title: gtk::Label,
    l_lang: gtk::Label,
    lang_btns: Vec<(String, gtk::ToggleButton)>,
    l_theme: gtk::Label,
    theme_btns: Vec<(String, gtk::Button)>,
    l_scale: gtk::Label,
    seg_scale: Rc<Segmented>,
    l_hypr: gtk::Label,
    seg_hypr: Rc<Segmented>,
    l_upd: gtk::Label,
    seg_upd: Rc<Segmented>,
    pub b_restart: gtk::Button,
    b_export: gtk::Button,
    b_import: gtk::Button,
    b_update: gtk::Button,
    b_about: gtk::Button,
    info: gtk::Label,
    css: gtk::CssProvider,
}

fn css_name(n: &str) -> String {
    n.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect()
}

impl Panel {
    pub fn new(cfg: &Obj) -> Rc<Panel> {
        let root = vbox(12);
        root.add_css_class("Panel");
        root.set_size_request(300, -1);
        root.set_halign(gtk::Align::Start);
        root.add_css_class("Pad22");
        let title = label("", "PanelTitle");
        title.set_margin_bottom(10);
        root.append(&title);
        let l_lang = label("", "Label");
        root.append(&l_lang);
        let row = hbox(6);
        let mut lang_btns = vec![];
        let mut first: Option<gtk::ToggleButton> = None;
        for (code, name) in [("de", "Deutsch"), ("en", "English")] {
            let b = gtk::ToggleButton::with_label(name);
            b.add_css_class("Lang");
            cursor_hand(&b);
            if let Some(f) = &first {
                b.set_group(Some(f));
            } else {
                first = Some(b.clone());
            }
            b.set_hexpand(true);
            let c = code.to_string();
            b.connect_clicked(move |_| win().set_language(&c));
            row.append(&b);
            lang_btns.push((code.to_string(), b));
        }
        row.set_margin_bottom(10);
        root.append(&row);
        let l_theme = label("", "Label");
        root.append(&l_theme);
        let grid = gtk::Grid::new();
        grid.set_row_spacing(10);
        grid.set_column_spacing(10);
        let mut theme_btns = vec![];
        for (i, (name, _)) in palettes::themes().into_iter().enumerate() {
            let b = gtk::Button::new();
            b.add_css_class("Accent");
            b.add_css_class(&format!("acc-{}", css_name(&name)));
            b.set_size_request(38, 38);
            cursor_hand(&b);
            b.set_tooltip_text(Some(&palettes::title(&name)));
            let n = name.clone();
            b.connect_clicked(move |_| win().set_theme(&n));
            grid.attach(&b, (i % 5) as i32, (i / 5) as i32, 1, 1);
            theme_btns.push((name, b));
        }
        root.append(&grid);
        // App-Einstellungen direkt unter den Farben
        let gap = vbox(0);
        gap.set_size_request(-1, 10);
        root.append(&gap);
        let l_scale = label("", "Label");
        let seg_scale = seg(&["100", "90", "80"], "app.scale.");
        let sc = match cfg.get("ui_scale") {
            Some(serde_json::Value::Number(n)) => n.to_string(),
            Some(serde_json::Value::String(s)) => s.clone(),
            _ => "100".into(),
        };
        seg_scale.select(&sc);
        seg_scale.connect(|v| win().set_ui_scale(&v));
        let l_hypr = label("", "Label");
        let seg_hypr = seg(&["embed", "window"], "app.hypr.");
        seg_hypr.select(if cfg.s("hypr_mode").is_empty() { "embed" } else { cfg.s("hypr_mode") });
        seg_hypr.connect(|v| win().app_opt("hypr_mode", &v));
        let l_upd = label("", "Label");
        let seg_upd = seg(&["on", "off"], "upd.hint.");
        let hint = cfg.get("update_hint").map(truthy).unwrap_or(true);
        seg_upd.select(if hint { "on" } else { "off" });
        seg_upd.connect(|v| {
            let w = win();
            w.cfg.borrow_mut().insert("update_hint".into(), json!(v == "on"));
            w.save();
        });
        let b_restart = gtk::Button::with_label("");
        b_restart.add_css_class("Primary");
        cursor_hand(&b_restart);
        b_restart.connect_clicked(|_| restart());
        b_restart.set_visible(false);
        root.append(&l_scale);
        root.append(&seg_scale.root);
        root.append(&l_hypr);
        root.append(&seg_hypr.root);
        root.append(&l_upd);
        root.append(&seg_upd.root);
        root.append(&b_restart);
        root.append(&vspacer());
        let b_export = icon_button("download", theme::MUTED, 16, "", "");
        b_export.connect_clicked(|_| win().export_preset());
        let b_import = icon_button("upload", theme::MUTED, 16, "", "");
        b_import.connect_clicked(|_| win().import_preset());
        let b_update = gtk::Button::with_label("");
        b_update.connect_clicked(|_| win().check_update(false));
        let b_about = gtk::Button::with_label("");
        b_about.connect_clicked(|_| win().show_about());
        for b in [&b_export, &b_import, &b_update, &b_about] {
            cursor_hand(b);
            root.append(b);
        }
        let info = label("", "Label");
        info.set_wrap(true);
        info.set_xalign(0.0);
        root.append(&info);
        let css = gtk::CssProvider::new();
        if let Some(d) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(&d, &css, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 30);
        }
        Rc::new(Panel { root, title, l_lang, lang_btns, l_theme, theme_btns, l_scale, seg_scale, l_hypr, seg_hypr,
            l_upd, seg_upd, b_restart, b_export, b_import, b_update, b_about, info, css })
    }

    pub fn retranslate(&self) {
        self.l_scale.set_text(&t("app.scale"));
        self.l_hypr.set_text(&t("app.hypr"));
        self.l_upd.set_text(&t("upd.hint"));
        self.seg_upd.retranslate();
        self.b_restart.set_label(&t("app.restart"));
        self.seg_scale.retranslate();
        self.seg_hypr.retranslate();
    }

    pub fn sync(&self, cfg: &Obj, backend: Option<&str>) {
        self.title.set_text(&t("settings"));
        self.l_lang.set_text(&t("language"));
        self.l_theme.set_text(&t("theme"));
        self.b_about.set_label(&t("about"));
        set_button_text(&self.b_export, &t("preset.export"));
        set_button_text(&self.b_import, &t("preset.import"));
        self.b_update.set_label(&t("upd.check"));
        if let Some((_, b)) = self.lang_btns.iter().find(|(c, _)| c == cfg.s("lang")) {
            b.set_active(true);
        }
        let cur = cfg.s("theme");
        let mut css = String::new();
        for (name, _) in &self.theme_btns {
            let ring = if name == cur { "#ffffff" } else { theme::SURFACE };
            css.push_str(&format!("button.Accent.acc-{} {{ background: {}; border: 3px solid {}; border-radius: 19px; \
                padding: 0; min-width: 0; min-height: 0; }}\n", css_name(name), theme::accent_of(name), ring));
        }
        self.css.load_from_string(&css);
        self.info.set_text(&format!("{} · {} · v{}\nLabtosoStudios · GPL-3.0", system::distro_name(),
            backend.unwrap_or("–"), crate::VERSION));
    }
}

/// Dieselbe Kopie neu starten (AppImage oder installiert) und diese beenden.
pub fn restart() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let prog = std::env::var("APPIMAGE").ok().filter(|s| !s.is_empty())
        .or_else(|| std::env::current_exe().ok().map(|p| p.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "linux-aio".into());
    let mut v = vec![prog];
    v.extend(args);
    let refs: Vec<&str> = v.iter().map(|s| s.as_str()).collect();
    let _ = hyprstudio::util::spawn_detached(&refs);
    win().quit();
}
