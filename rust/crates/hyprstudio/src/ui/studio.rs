//! Hauptfenster: Tabs Hyprland / Waybar / Launcher / Sonstiges oben, Design-Vorlage Nr. 1.
//! Läuft als eigenes Fenster (Hypr Studio) oder eingebettet in Linux AIO (ohne Titel, Fensterknöpfe, Fußzeile).

use super::draw;
use super::editor::{EditorPage, Kind};
use super::gate::{Gate, LauncherGate};
use super::other::OtherPage;
use super::settings::{reset_dialog, SettingsPanel};
use super::shell::{self, Conf, Shell};
use super::theme;
use super::widgets::{self, hbox, label, set_tool_icon, spacer, tool_button, vbox, Segmented};
use crate::i18n::{self, t, tf};
use crate::jsonx::{Get, Obj};
use crate::{appearance, config, hardware, kw, laptopkeys, launcher, model, packages, palettes, reset, sysactions};
use gtk::prelude::*;
use gtk::{gdk, glib};
use once_cell::unsync::OnceCell;
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

pub const TABS: &[&str] = &["hypr", "waybar", "launcher", "other"];

/// Wie das Fenster läuft.
#[derive(Clone, Default)]
pub struct Opts {
    /// in Linux AIO eingebettet (ohne Rahmen, Titel, Fensterknöpfe, eigene Fußzeile)
    pub embedded: bool,
    /// eigenes Fenster aus Linux AIO heraus (verschiebbar, maximierbar, ohne Fußzeile)
    pub popout: bool,
    /// Linux AIO › Launcher: nur die Launcher-Seite
    pub launcher_only: bool,
    /// Linux AIO: Zahnrad weg, Launcher-Tab weg, Kitty weg, „Erweitert“ unter Sonstiges
    pub aio: bool,
    /// Status und Protokoll zusätzlich an Linux AIO
    pub host: Option<Rc<dyn Shell>>,
    /// Automatiken nur, wenn wirklich Hyprland läuft (Linux AIO)
    pub only_on_hyprland: bool,
    /// Testmodus von Linux AIO
    pub host_dry: bool,
}

pub struct Studio {
    pub opts: Opts,
    pub cfg: Rc<RefCell<Obj>>,
    pub window: Option<gtk::Window>,
    /// alles im Fenster (zum Einbetten)
    pub root: gtk::Box,
    overlay: gtk::Overlay,
    content: gtk::Box,
    gear: gtk::Button,
    title: gtk::Label,
    pub tabs: Rc<Segmented>,
    btn_min: gtk::Button,
    btn_max: gtk::Button,
    btn_close: gtk::Button,
    stack: gtk::Stack,
    status: gtk::Label,
    busy: gtk::ProgressBar,
    pub btn_log: gtk::ToggleButton,
    logview: gtk::TextView,
    log_box: gtk::ScrolledWindow,
    dim: gtk::Box,
    reveal: gtk::Revealer,
    panel: Rc<SettingsPanel>,
    save_id: RefCell<Option<glib::SourceId>>,
    pub fake_missing: Cell<bool>,
    hypr: OnceCell<Rc<EditorPage>>,
    waybar: OnceCell<Rc<EditorPage>>,
    launcher: OnceCell<Rc<EditorPage>>,
    other: OnceCell<Rc<OtherPage>>,
    waybar_gate: OnceCell<Rc<Gate>>,
    launcher_gate: OnceCell<Rc<LauncherGate>>,
    me: RefCell<Weak<Studio>>,
}

struct PageContent(gtk::Widget);
impl super::gate::GateContent for PageContent {
    fn widget(&self) -> gtk::Widget {
        self.0.clone()
    }
}

impl Studio {
    pub fn new(app: Option<&gtk::Application>, opts: Opts) -> Rc<Studio> {
        let mut c = config::load_cfg();
        i18n::set_lang(if c.s("lang").is_empty() { "en" } else { c.s("lang") });
        let fmt = c.s("hypr_format").to_string();
        model::set_format(if fmt == "lua" || fmt == "conf" { &fmt } else { "" });
        let ls = launcher::state(c.obj("launcher"));
        model::LAUNCHER_TOGGLE.store(ls.get("toggle").map(crate::jsonx::truthy).unwrap_or(true),
            std::sync::atomic::Ordering::Relaxed);
        let th = c.s("theme").to_string();
        let th = palettes::legacy(&th).unwrap_or(th);
        let th = if palettes::theme_color(&th).is_some() { th } else { "01".into() };
        c.insert("theme".into(), json!(th));
        if !TABS.contains(&c.s("tab")) {
            c.insert("tab".into(), json!("hypr"));
        }
        let cfg = Rc::new(RefCell::new(c));

        let standalone = !opts.embedded;
        let window = if standalone {
            let w = match app {
                Some(a) => gtk::ApplicationWindow::new(a).upcast::<gtk::Window>(),
                None => gtk::Window::new(),
            };
            w.set_title(Some(if opts.popout { "Hypr Studio – Linux AIO" } else { crate::APP_NAME }));
            w.set_icon_name(Some(crate::APP_ID));
            w.set_decorated(false);
            w.add_css_class("hs-window");
            w.set_default_size(1220, 860);
            w.set_size_request(1000, 720);
            // aus Linux AIO geöffnet: Schließen versteckt nur (erneut öffnen zeigt dasselbe Fenster)
            w.set_hide_on_close(opts.popout);
            Some(w)
        } else {
            None
        };
        let root = vbox(0);
        if standalone {
            root.add_css_class("Root");
            root.set_overflow(gtk::Overflow::Hidden);
        }
        let overlay = gtk::Overlay::new();
        overlay.set_vexpand(true);
        root.append(&overlay);
        let content = vbox(14);
        if opts.embedded {
            widgets::margins(&content, 14, 10, 14, 12);
        } else {
            widgets::margins(&content, 20, 12, 20, 16);
        }
        overlay.set_child(Some(&content));

        // Kopfzeile
        let gear = tool_button("gear", theme::TEXT, 22, "");
        let title = label(crate::APP_NAME, "Title");
        let left = hbox(10);
        left.append(&gear);
        left.append(&title);
        let tabs = Segmented::new(TABS, "tab.");
        tabs.root.add_css_class("TabBox");
        let right = hbox(0);
        right.set_halign(gtk::Align::End);
        let btn_min = tool_button("min", theme::MUTED, 18, "");
        let btn_max = tool_button("max", theme::MUTED, 18, "");
        let btn_close = tool_button("close", theme::MUTED, 18, "CloseBtn");
        right.append(&btn_min);
        right.append(&btn_max);
        right.append(&btn_close);
        btn_max.set_visible(opts.popout);
        let head = gtk::CenterBox::new();
        head.set_start_widget(Some(&left));
        head.set_center_widget(Some(&tabs.root));
        head.set_end_widget(Some(&right));
        if standalone {
            let handle = gtk::WindowHandle::new();
            handle.set_child(Some(&head));
            content.append(&handle);
        } else {
            content.append(&head);
        }
        let stack = gtk::Stack::new();
        stack.set_vexpand(true);
        stack.set_hhomogeneous(false);
        stack.set_vhomogeneous(false);
        content.append(&stack);
        // Fußzeile
        let foot = hbox(0);
        let status = label("", "Status");
        status.set_hexpand(true);
        status.set_ellipsize(gtk::pango::EllipsizeMode::End);
        let busy = widgets::busy_bar(200);
        let btn_log = gtk::ToggleButton::new();
        widgets::cursor_hand(&btn_log);
        btn_log.set_margin_start(12);
        foot.append(&status);
        foot.append(&busy);
        foot.append(&btn_log);
        content.append(&foot);
        let logview = gtk::TextView::new();
        logview.set_editable(false);
        logview.set_cursor_visible(false);
        logview.add_css_class("Log");
        logview.set_wrap_mode(gtk::WrapMode::WordChar);
        let log_box = gtk::ScrolledWindow::new();
        log_box.add_css_class("EditFrame");
        log_box.set_child(Some(&logview));
        log_box.set_size_request(-1, 140);
        log_box.set_visible(false);
        content.append(&log_box);
        if opts.embedded || opts.popout {
            // eigene Fußzeile weg: Status und Protokoll laufen über Linux AIO
            status.set_visible(false);
            btn_log.set_visible(false);
        }
        if opts.embedded {
            title.set_visible(false);
            btn_min.set_visible(false);
            btn_close.set_visible(false);
        }

        // Einstellungen: abdunkeln + Panel von links
        let dim = vbox(0);
        dim.add_css_class("Dim");
        dim.set_visible(false);
        overlay.add_overlay(&dim);
        let panel = SettingsPanel::new();
        let reveal = gtk::Revealer::new();
        reveal.set_transition_type(gtk::RevealerTransitionType::SlideRight);
        reveal.set_transition_duration(220);
        reveal.set_halign(gtk::Align::Start);
        reveal.set_valign(gtk::Align::Fill);
        reveal.set_child(Some(&panel.root));
        reveal.set_visible(false);
        overlay.add_overlay(&reveal);
        if standalone {
            // Größe ändern unten rechts (wie QSizeGrip)
            let grip = gtk::Box::new(gtk::Orientation::Vertical, 0);
            grip.set_size_request(16, 16);
            grip.set_halign(gtk::Align::End);
            grip.set_valign(gtk::Align::End);
            grip.set_cursor_from_name(Some("se-resize"));
            let g = gtk::GestureClick::new();
            g.connect_pressed(|g, _, x, y| {
                let Some(w) = g.widget() else { return };
                let Some(native) = w.native() else { return };
                let Some(surf) = native.surface() else { return };
                let Ok(top) = surf.downcast::<gdk::Toplevel>() else { return };
                let Some(dev) = g.device() else { return };
                let (sx, sy) = w.translate_coordinates(&native, x, y).unwrap_or((x, y));
                top.begin_resize(gdk::SurfaceEdge::SouthEast, Some(&dev), g.current_button() as i32, sx, sy,
                    g.current_event_time());
            });
            grip.add_controller(g);
            // eigene Zeile unter dem Inhalt wie QSizeGrip (gleiche Höhe)
            let grip_row = hbox(0);
            grip_row.set_size_request(-1, 26);
            grip_row.append(&spacer());
            grip.set_margin_end(4);
            grip.set_margin_bottom(4);
            grip_row.append(&grip);
            root.append(&grip_row);
        }
        if let Some(w) = &window {
            w.set_child(Some(&root));
        }

        let s = Rc::new(Studio { opts, cfg, window, root, overlay, content, gear, title, tabs, btn_min, btn_max, btn_close,
            stack, status, busy, btn_log, logview, log_box, dim, reveal, panel, save_id: RefCell::new(None),
            fake_missing: Cell::new(false), hypr: OnceCell::new(), waybar: OnceCell::new(), launcher: OnceCell::new(),
            other: OnceCell::new(), waybar_gate: OnceCell::new(), launcher_gate: OnceCell::new(),
            me: RefCell::new(Weak::new()) });
        *s.me.borrow_mut() = Rc::downgrade(&s);
        if !s.opts.embedded && s.opts.host.is_none() {
            shell::set_shell(s.clone());
        }
        theme::apply(&s.accent());
        s.connect();
        s.build_pages();
        s.apply_theme();
        s.retranslate();
        s.update_previews();
        for pg in s.all_pages() {
            pg.render_presets(true);
        }
        let tab = s.cfg.borrow().s("tab").to_string();
        s.set_tab(&tab);
        if model::dry() {
            s.set_status(&t("testmode"));
        }
        // beim Start nach Config-Fehlern schauen
        let w = Rc::downgrade(&s);
        shell::later(300, move || {
            if let Some(s) = w.upgrade() {
                s.startup_check();
            }
        });
        s
    }

    fn connect(self: &Rc<Self>) {
        let w = Rc::downgrade(self);
        self.gear.connect_clicked(move |_| {
            if let Some(s) = w.upgrade() {
                s.toggle_settings();
            }
        });
        let w = Rc::downgrade(self);
        self.tabs.connect(move |tab| {
            if let Some(s) = w.upgrade() {
                s.set_tab(&tab);
            }
        });
        let w = Rc::downgrade(self);
        self.btn_min.connect_clicked(move |_| {
            if let Some(win) = w.upgrade().and_then(|s| s.window.clone()) {
                win.minimize();
            }
        });
        let w = Rc::downgrade(self);
        self.btn_max.connect_clicked(move |_| {
            if let Some(win) = w.upgrade().and_then(|s| s.window.clone()) {
                if win.is_maximized() { win.unmaximize() } else { win.maximize() }
            }
        });
        let w = Rc::downgrade(self);
        self.btn_close.connect_clicked(move |_| {
            if let Some(win) = w.upgrade().and_then(|s| s.window.clone()) {
                win.close();
            }
        });
        let w = Rc::downgrade(self);
        self.btn_log.connect_toggled(move |b| {
            if let Some(s) = w.upgrade() {
                s.logview_toggle(b.is_active());
            }
        });
        let click = gtk::GestureClick::new();
        let w = Rc::downgrade(self);
        click.connect_pressed(move |_, _, _, _| {
            if let Some(s) = w.upgrade() {
                s.toggle_settings();
            }
        });
        self.dim.add_controller(click);
        let w = Rc::downgrade(self);
        self.reveal.connect_child_revealed_notify(move |r| {
            if !r.reveals_child() {
                if let Some(s) = w.upgrade() {
                    s.reveal.set_visible(false);
                    s.dim.set_visible(false);
                    s.content.remove_css_class("blurred");
                }
            }
        });
        let w = Rc::downgrade(self);
        *self.panel.on_lang.borrow_mut() = Some(Box::new(move |c| {
            if let Some(s) = w.upgrade() {
                s.set_language(&c);
            }
        }));
        let w = Rc::downgrade(self);
        *self.panel.on_theme.borrow_mut() = Some(Box::new(move |c| {
            if let Some(s) = w.upgrade() {
                s.set_theme(&c);
            }
        }));
        let w = Rc::downgrade(self);
        *self.panel.on_reset.borrow_mut() = Some(Box::new(move |_| {
            if let Some(s) = w.upgrade() {
                s.reset_all();
            }
        }));
        let w = Rc::downgrade(self);
        *self.panel.on_format.borrow_mut() = Some(Box::new(move |f| {
            if let Some(s) = w.upgrade() {
                s.set_format(&f);
            }
        }));
        if let Some(win) = &self.window {
            let w = Rc::downgrade(self);
            win.connect_close_request(move |_| {
                if let Some(s) = w.upgrade() {
                    config::save_cfg(&s.cfg.borrow());
                }
                glib::Propagation::Proceed
            });
        }
    }

    fn build_pages(self: &Rc<Self>) {
        let hp = EditorPage::new(self, Kind::Hypr);
        let _ = self.hypr.set(hp.clone());
        let wp = EditorPage::new(self, Kind::Waybar);
        let _ = self.waybar.set(wp.clone());
        let lp = EditorPage::new(self, Kind::Launcher);
        let _ = self.launcher.set(lp.clone());
        self.stack.add_named(&hp.root, Some("hypr"));
        let wg = Gate::new(Rc::new(PageContent(wp.root.clone().upcast())), "waybar", self);
        self.stack.add_named(&wg.stack, Some("waybar"));
        let _ = self.waybar_gate.set(wg);
        let lg = LauncherGate::new(lp.clone(), self);
        self.stack.add_named(&lg.stack, Some("launcher"));
        let _ = self.launcher_gate.set(lg);
        let op = OtherPage::new(self);
        self.stack.add_named(&op.root, Some("other"));
        let _ = self.other.set(op.clone());
        for k in ["wallpaper", "cursor", "lock"] {
            // gehören zu Hyprland
            if let Some(g) = op.gate(k) {
                hp.add_external(k, g);
            }
        }
        let sec = self.cfg.borrow().s("hypr_section").to_string();
        hp.show_section(if sec.is_empty() { "presets" } else { &sec });
        if self.opts.aio {
            op.add_advanced();
            self.gear.set_visible(false); // Einstellungen stecken in Sonstiges › Erweitert
            op.hide_kitty();
        }
        if self.opts.launcher_only {
            self.tabs.root.set_visible(false);
            self.cfg.borrow_mut().insert("tab".into(), json!("launcher"));
        } else if self.opts.aio {
            // der Launcher hat in Linux AIO einen eigenen Tab
            if let Some(b) = self.tabs.btn("launcher") {
                b.set_visible(false);
            }
            if self.cfg.borrow().s("tab") == "launcher" {
                self.cfg.borrow_mut().insert("tab".into(), json!("hypr"));
            }
        }
        for f in [Self::first_look as fn(&Rc<Self>), Self::auto_laptop, Self::auto_sysactions, Self::auto_look] {
            let w = Rc::downgrade(self);
            glib::idle_add_local_once(move || {
                if let Some(s) = w.upgrade() {
                    f(&s);
                }
            });
        }
    }

    // ----- Zugriff für die Seiten -----

    pub fn hp(&self) -> Option<Rc<EditorPage>> {
        self.hypr.get().cloned()
    }
    pub fn wp(&self) -> Option<Rc<EditorPage>> {
        self.waybar.get().cloned()
    }
    pub fn lp(&self) -> Option<Rc<EditorPage>> {
        self.launcher.get().cloned()
    }
    pub fn op(&self) -> Option<Rc<OtherPage>> {
        self.other.get().cloned()
    }

    pub fn conf(self: &Rc<Self>) -> Conf {
        let w = Rc::downgrade(self);
        Conf { cfg: self.cfg.clone(), save: Rc::new(move || {
            if let Some(s) = w.upgrade() {
                s.save_later();
            }
        }) }
    }

    pub fn all_pages(&self) -> Vec<Rc<EditorPage>> {
        [self.hp(), self.wp(), self.lp()].into_iter().flatten().collect()
    }

    pub fn accent(&self) -> String {
        theme::accent_of(self.cfg.borrow().s("theme"))
    }

    pub fn save_later(&self) {
        if let Some(id) = self.save_id.borrow_mut().take() {
            id.remove();
        }
        let cfg = self.cfg.clone();
        let me = self.me.borrow().clone();
        let id = glib::timeout_add_local_once(std::time::Duration::from_millis(400), move || {
            config::save_cfg(&cfg.borrow());
            if let Some(s) = me.upgrade() {
                s.save_id.borrow_mut().take();
            }
        });
        *self.save_id.borrow_mut() = Some(id);
    }

    pub fn save_now(&self) {
        config::save_cfg(&self.cfg.borrow());
    }

    pub fn update_previews(&self) {
        let Some(hp) = self.hp() else { return };
        let hs = hp.state.borrow().clone();
        let ws = self.wp().map(|p| p.state.borrow().clone()).unwrap_or_else(|| model::waybar_state(None));
        for pg in self.all_pages() {
            pg.preview.set_states(&hs, &ws);
            pg.mark_presets_dirty();
        }
    }

    pub fn launcher_gate_refresh(&self) {
        if let Some(g) = self.launcher_gate.get() {
            g.refresh();
        }
    }

    pub fn install_keys(&self, keys: &[&str]) {
        if let Some(op) = self.op() {
            op.install_keys(&keys.iter().map(|s| s.to_string()).collect::<Vec<_>>(), false);
        }
    }

    /// Ist ein Zusatzprogramm installiert? Sonst großer Installieren-Bildschirm.
    pub fn refresh_gates(&self) {
        let Some(op) = self.op() else { return };
        if let Some(g) = self.waybar_gate.get() {
            g.refresh();
        }
        self.launcher_gate_refresh();
        op.refresh_gates();
        if let Some(hp) = self.hp() {
            hp.sync_external();
        }
        for pg in self.all_pages() {
            pg.update_info();
        }
        let (tab, sec) = {
            let c = self.cfg.borrow();
            (c.s("tab").to_string(), c.s("other_section").to_string())
        };
        if tab == "other" && sec == "install" {
            op.refresh_packages(); // Paketliste nur, wenn sie zu sehen ist
        }
        match sec.as_str() {
            "look" => op.refresh_look(),
            "polkit" => op.refresh_polkit(),
            "laptop" => op.refresh_laptop(),
            _ => {}
        }
    }

    pub fn bar_busy(&self, on: bool) {
        self.busy.set_visible(on);
    }

    pub fn set_status(&self, text: &str) {
        self.status.set_text(text);
        if let Some(h) = &self.opts.host {
            h.set_status(text);
        }
    }

    pub fn log(&self, text: &str) {
        let buf = self.logview.buffer();
        let mut end = buf.end_iter();
        if buf.char_count() > 0 {
            buf.insert(&mut end, "\n");
        }
        buf.insert(&mut end, text);
        if buf.line_count() > 5000 {
            let mut a = buf.start_iter();
            let mut b = buf.iter_at_line(buf.line_count() - 5000).unwrap_or(buf.start_iter());
            buf.delete(&mut a, &mut b);
        }
        let mark = buf.create_mark(None, &buf.end_iter(), false);
        self.logview.scroll_mark_onscreen(&mark);
        buf.delete_mark(&mark);
        if let Some(h) = &self.opts.host {
            h.log(text);
        }
    }

    pub fn log_visible(&self) -> bool {
        if let Some(h) = &self.opts.host {
            let _ = h;
            return false;
        }
        self.log_box.is_visible()
    }

    pub fn show_log(&self) {
        if let Some(h) = &self.opts.host {
            h.show_log();
            return;
        }
        if !self.log_box.is_visible() {
            self.btn_log.set_active(true);
        }
    }

    pub fn report(&self, ok: bool, info: &str) {
        if ok {
            self.set_status(&t("done"));
        } else {
            self.set_status(&format!("{}: {}", t("failed"), t(info)));
            self.show_log();
        }
    }

    fn logview_toggle(&self, on: bool) {
        self.log_box.set_visible(on);
    }

    // ----- Tabs -----

    pub fn set_tab(self: &Rc<Self>, tab: &str) {
        let tab = if self.opts.launcher_only {
            "launcher"
        } else if self.opts.aio && tab == "launcher" {
            "hypr"
        } else {
            tab
        };
        self.cfg.borrow_mut().insert("tab".into(), json!(tab));
        self.save_later();
        self.tabs.select(tab);
        self.stack.set_visible_child_name(tab);
        if tab == "launcher" {
            // aktiven Launcher aus den Kürzeln erkennen
            if let Some(lp) = self.lp() {
                lp.autodetect();
            }
        }
        self.refresh_gates();
        if tab == "other" {
            let sec = self.cfg.borrow().s("other_section").to_string();
            if sec.is_empty() || sec == "fix" {
                if let Some(op) = self.op() {
                    op.scan(false);
                }
            }
        } else {
            let page = match tab {
                "hypr" => self.hp(),
                "waybar" => self.wp(),
                _ => self.lp(),
            };
            if let Some(pg) = page {
                if pg.current_section() == "presets" {
                    pg.render_presets(false);
                }
            }
        }
    }

    pub fn show_section(self: &Rc<Self>, tab: &str, sec: &str) {
        match tab {
            "hypr" => self.hp().map(|p| p.show_section(sec)),
            "waybar" => self.wp().map(|p| p.show_section(sec)),
            "launcher" => self.lp().map(|p| p.show_section(sec)),
            _ => self.op().map(|p| p.show_section(sec)),
        };
    }

    // ----- Einstellungen -----

    pub fn toggle_settings(self: &Rc<Self>) {
        let opening = !self.reveal.reveals_child();
        if opening {
            self.content.add_css_class("blurred");
            self.dim.set_visible(true);
            self.panel.pages.set_visible_child_name("main");
            self.panel.sync(&self.cfg.borrow(), &self.info());
            self.reveal.set_visible(true);
            self.reveal.set_reveal_child(true);
        } else {
            self.reveal.set_reveal_child(false);
        }
    }

    fn info(&self) -> String {
        let fmt = model::hypr_format(&model::config_home().join("hypr")).unwrap_or_else(|| "none".into());
        let mut s = format!("Hyprland: {}", t(&format!("fmt.{fmt}")));
        if model::dry() {
            s.push('\n');
            s.push_str(&t("testmode"));
        }
        s
    }

    pub fn reset_all(self: &Rc<Self>) {
        let hdir = model::config_home().join("hypr");
        let lua_ok = matches!(model::hypr_format(&hdir).as_deref(), Some("lua") | None);
        let w = Rc::downgrade(self);
        reset_dialog(self.parent_window().as_ref(), lua_ok, move |r| {
            let (Some(s), Some(factory)) = (w.upgrade(), r) else { return };
            if s.reveal.reveals_child() {
                s.toggle_settings();
            }
            s.log("");
            s.log(&format!("== {} ==", t("reset.title")));
            reset::reset_all(&model::config_home(), factory, &|x| s.log(x));
            for pg in s.all_pages() {
                *pg.state.borrow_mut() = pg.kind.sanitize(None);
                for k in ["preset", "applied_sig", "applied_preset"] {
                    s.cfg.borrow_mut().insert(format!("{}_{k}", pg.kind.key()), json!(""));
                }
                pg.refresh_controls();
                pg.changed();
            }
            if let Some(lp) = s.lp() {
                lp.preview.set_launcher(Some(&lp.state.borrow()));
            }
            if let Some(op) = s.op() {
                op.kitty_reset_state();
            }
            s.save_later();
            s.set_status(&t("reset.done"));
            s.show_log();
            let w2 = Rc::downgrade(&s);
            shell::later(700, move || {
                if let Some(op) = w2.upgrade().and_then(|s| s.op()) {
                    op.scan(true);
                }
            });
        });
    }

    pub fn parent_window(&self) -> Option<gtk::Window> {
        self.window.clone().or_else(|| shell::toplevel_of(&self.root))
    }

    pub fn set_language(self: &Rc<Self>, code: &str) {
        self.cfg.borrow_mut().insert("lang".into(), json!(code));
        i18n::set_lang(code);
        self.save_later();
        self.retranslate();
        self.panel.sync(&self.cfg.borrow(), &self.info());
    }

    /// Erweiterte Einstellungen: alte (.conf) oder neue (Lua) Hyprland-Config schreiben.
    pub fn set_format(self: &Rc<Self>, fmt: &str) {
        self.cfg.borrow_mut().insert("hypr_format".into(), json!(fmt));
        model::set_format(fmt);
        self.save_later();
        self.panel.sync(&self.cfg.borrow(), &self.info());
        if let Some(hp) = self.hp() {
            hp.update_info();
        }
        let msg = tf("adv.set", kw!("name" => t(&format!("fmtsw.{fmt}"))));
        self.set_status(&msg);
        self.log(&msg);
    }

    fn on_hyprland(&self) -> bool {
        !self.opts.only_on_hyprland || std::env::var("XDG_CURRENT_DESKTOP").map(|d| d.to_lowercase().contains("hyprland"))
            .unwrap_or(false) || std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok()
    }

    /// Hell/Dunkel bei jedem Start auffrischen: Portal-Datei und Umgebung bleiben aktuell (z. B. nach Updates).
    fn auto_look(self: &Rc<Self>) {
        if !self.on_hyprland() {
            return;
        }
        let mode = self.cfg.borrow().s("appearance").to_string();
        if appearance::MODES.contains(&mode.as_str()) {
            let log = |x: &str| self.log(x);
            appearance::write_portals_conf(&model::config_home(), &log);
            appearance::apply_env(&mode, &model::config_home(), &log);
            appearance::apply_services(&mode, &model::config_home(), &log);
        }
    }

    /// Herunterfahren/Neustart … in allen Launchern (an, solange der Schalter nicht aus ist).
    fn auto_sysactions(self: &Rc<Self>) {
        if !self.on_hyprland() {
            return;
        }
        let on = self.cfg.borrow().get("sys_actions").map(crate::jsonx::truthy).unwrap_or(true);
        if on {
            sysactions::apply(true, &|x| self.log(x), packages::installed("hyprlock"));
        }
    }

    /// Wie KDE: auf Laptops funktionieren Helligkeit, Lautstärke und Stumm von selbst.
    fn auto_laptop(self: &Rc<Self>) {
        if !self.on_hyprland() {
            return;
        }
        let Some(hp) = self.hp() else { return };
        let was = hp.state.borrow().b("laptop_keys");
        let changed = {
            let mut st = hp.state.borrow_mut();
            laptopkeys::auto(&mut st, &|x| self.log(x))
        };
        if changed {
            hp.changed();
            if hp.state.borrow().b("laptop_keys") && !was {
                self.log(&tf("laptop.auto", kw!("name" => hardware::label(&hardware::detect()))));
            }
        }
    }

    /// Erster Start: dunkles Farbschema für alle Programme (unter Sonstiges umstellbar).
    fn first_look(self: &Rc<Self>) {
        if !(self.on_hyprland() || self.opts.host_dry) {
            return;
        }
        if self.cfg.borrow().contains_key("appearance") {
            return;
        }
        // nur merken – das System ändert erst ein Klick
        self.cfg.borrow_mut().insert("appearance".into(), json!(appearance::DEFAULT));
        if let Some(hp) = self.hp() {
            hp.state.borrow_mut().insert("qt_theme".into(), json!(appearance::qt_platform_theme()));
            hp.changed();
        }
        self.log(&t("look.first"));
        self.save_later();
    }

    fn startup_check(self: &Rc<Self>) {
        if !(self.on_hyprland() || self.opts.host_dry) {
            return;
        }
        if let Some(op) = self.op() {
            let n = op.scan(true);
            if n > 0 {
                self.set_status(&tf("fix.startup", kw!("n" => n)));
            }
        }
    }

    pub fn set_theme(self: &Rc<Self>, name: &str) {
        self.cfg.borrow_mut().insert("theme".into(), json!(name));
        self.save_later();
        self.apply_theme();
        self.panel.sync(&self.cfg.borrow(), &self.info());
    }

    pub fn apply_theme(&self) {
        if !self.opts.embedded {
            theme::apply(&self.accent());
        }
        set_tool_icon(&self.gear, "gear", theme::TEXT, 22);
        set_tool_icon(&self.btn_min, "min", theme::MUTED, 18);
        set_tool_icon(&self.btn_max, "max", theme::MUTED, 18);
        set_tool_icon(&self.btn_close, "close", theme::MUTED, 18);
        let acc = self.accent();
        for pg in self.all_pages() {
            pg.apply_theme(&acc);
        }
        if let Some(op) = self.op() {
            op.apply_theme(&acc);
        }
        widgets::recolor_switches(&acc); // wirklich jeder Schalter in der Akzentfarbe
    }

    pub fn retranslate(self: &Rc<Self>) {
        self.gear.set_tooltip_text(Some(&if self.opts.embedded { "Hypr Studio".to_string() } else { t("settings") }));
        self.btn_min.set_tooltip_text(Some(&t("minimize")));
        self.btn_close.set_tooltip_text(Some(&t("close")));
        self.btn_log.set_label(&t("log"));
        self.tabs.retranslate();
        for pg in self.all_pages() {
            pg.retranslate();
        }
        if let Some(op) = self.op() {
            op.retranslate();
        }
        self.refresh_gates();
        self.set_status(&if model::dry() { t("testmode") } else { t("ready") });
    }

    /// Sprache/Akzent von Linux AIO übernehmen.
    pub fn sync_look(self: &Rc<Self>, lang: &str, accent_name: &str) {
        if self.cfg.borrow().s("lang") != lang {
            self.set_language(lang);
        }
        if palettes::theme_color(accent_name).is_some() && self.cfg.borrow().s("theme") != accent_name {
            self.set_theme(accent_name);
        }
    }

    pub fn present(&self) {
        if let Some(w) = &self.window {
            w.present();
        }
    }
}

impl Shell for Studio {
    fn set_status(&self, text: &str) {
        Studio::set_status(self, text)
    }
    fn log(&self, text: &str) {
        Studio::log(self, text)
    }
    fn bar_busy(&self, on: bool) {
        Studio::bar_busy(self, on)
    }
    fn accent(&self) -> String {
        Studio::accent(self)
    }
    fn cfg(&self) -> Rc<RefCell<Obj>> {
        self.cfg.clone()
    }
    fn save_later(&self) {
        Studio::save_later(self)
    }
    fn window(&self) -> Option<gtk::Window> {
        self.parent_window()
    }
    fn show_log(&self) {
        Studio::show_log(self)
    }
}

impl super::kitty_page::TermWin for Studio {
    fn conf(&self) -> Conf {
        match self.me.borrow().upgrade() {
            Some(me) => Studio::conf(&me),
            None => Conf { cfg: self.cfg.clone(), save: Rc::new(|| {}) },
        }
    }
    fn log(&self, text: &str) {
        Studio::log(self, text)
    }
    fn set_status(&self, text: &str) {
        Studio::set_status(self, text)
    }
    fn report(&self, ok: bool, info: &str) {
        Studio::report(self, ok, info)
    }
    fn hypr_state(&self) -> Option<Obj> {
        self.hp().map(|hp| hp.state.borrow().clone())
    }
}

#[allow(dead_code)]
fn _u(_: Value, _: draw::Rgba) {}

/// Testhilfe: HYPRSTUDIO_SHOT=/pfad/bild.png HYPRSTUDIO_STEPS='hypr:presets,waybar:modules' → je ein Bild.
pub fn shots(s: Rc<Studio>, shot: String, steps: String) {
    let items: Vec<String> = steps.split(',').filter(|x| !x.is_empty()).map(String::from).collect();
    let items = if items.is_empty() { vec![String::new()] } else { items };
    let idx = Rc::new(Cell::new(0usize));
    fn step(s: Rc<Studio>, shot: String, items: Vec<String>, idx: Rc<Cell<usize>>) {
        let i = idx.get();
        if i >= items.len() {
            if let Some(w) = &s.window {
                if let Some(app) = w.application() {
                    app.quit();
                } else {
                    w.close();
                }
            }
            std::process::exit(0);
        }
        if let Some((tab, sec)) = items[i].split_once(':') {
            s.set_tab(tab);
            s.show_section(tab, sec);
        }
        let s2 = s.clone();
        shell::later(400, move || {
            if let Some(w) = &s2.window {
                let path = shot.replace(".png", &format!("_{i}.png"));
                save_png(w.upcast_ref(), &path);
            }
            idx.set(i + 1);
            step(s2, shot, items, idx);
        });
    }
    step(s, shot, items, idx);
}

/// Widget als PNG speichern (Bildschirmfoto ohne Fenstermanager).
pub fn save_png(w: &gtk::Widget, path: &str) {
    let (wd, ht) = (w.width(), w.height());
    if wd <= 0 || ht <= 0 {
        return;
    }
    let p = gtk::WidgetPaintable::new(Some(w));
    let snap = gtk::Snapshot::new();
    p.snapshot(&snap, wd as f64, ht as f64);
    let Some(node) = snap.to_node() else { return };
    let Some(native) = w.native() else { return };
    let Some(r) = native.renderer() else { return };
    let tex = r.render_texture(&node, Some(&gtk::graphene::Rect::new(0.0, 0.0, wd as f32, ht as f32)));
    let _ = tex.save_to_png(path);
}
