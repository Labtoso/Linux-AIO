//! Hauptfenster: Tabs Desktops / SDDM / Plymouth / Einstellungen im Kachel-Galerie-Design.

use super::de_page::DePage;
use super::panel::Panel;
use super::search::SearchBox;
use super::settings_page::SettingsPage;
use super::style;
use super::theme_page::ThemePage;
use super::widgets::{app_icon, seg};
use crate::edition::ed;
use crate::i18n::{self, t, tf};
use crate::{presetfile, pyjson, safety, system, updater, usercmd};
use gtk::prelude::*;
use gtk::{gdk, glib};
use hyprstudio::jsonx::{Get, Obj};
use hyprstudio::kw;
use hyprstudio::ui::shell::{self, Shell};
use hyprstudio::ui::widgets::{cursor_hand, hbox, label, spacer, tool_button, vbox, Segmented};
use hyprstudio::ui::{dialogs, draw, theme};
use hyprstudio::{palettes, util};
use once_cell::unsync::OnceCell;
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};

/// Tabs dieser Ausgabe (SDDM Studio: nur „sddm“).
pub fn tabs() -> &'static [&'static str] {
    ed().tabs
}

/// Was jede Seite dem Fenster bietet.
pub trait AioPage {
    fn widget(&self) -> gtk::Widget;
    fn refresh(&self) {}
    fn retranslate(&self) {}
    /// Root-Aufgabe dieser Seite ist fertig.
    fn finished(&self, _ok: bool, _action: &str, _result: &str) {
        self.refresh();
    }
    /// Oberkante der Protokoll-Schublade (erstes Element unter der großen Karte).
    fn drawer_anchor(&self) -> gtk::Widget {
        self.widget()
    }
}

pub fn config_path() -> PathBuf {
    util::xdg_config().join("LabtosoStudios").join(ed().cfg_file)
}

pub fn load_cfg() -> Obj {
    match std::fs::read_to_string(config_path()).ok().and_then(|s| serde_json::from_str::<Value>(&s).ok()) {
        Some(Value::Object(m)) => m,
        _ => {
            let mut m = Obj::new();
            m.insert("lang".into(), json!("en"));
            m.insert("theme".into(), json!("01"));
            m
        }
    }
}

pub fn save_cfg(cfg: &Obj) {
    let p = config_path();
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::write(&p, pyjson::dumps(&Value::Object(cfg.clone()), None, false, true));
}

thread_local! {
    static WIN: RefCell<Option<Rc<Win>>> = const { RefCell::new(None) };
}

/// Das eine Hauptfenster.
pub fn win() -> Rc<Win> {
    WIN.with(|w| w.borrow().clone()).expect("Hauptfenster fehlt")
}

pub fn has_win() -> bool {
    WIN.with(|w| w.borrow().is_some())
}

enum RMsg {
    Line(String),
    Done(i32),
}

pub struct Win {
    pub cfg: Rc<RefCell<Obj>>,
    pub window: gtk::ApplicationWindow,
    pub backend: Option<String>,
    pub running: Option<String>,
    root: gtk::Box,
    content: gtk::Box,
    gear: gtk::Button,
    pub tabs: Rc<Segmented>,
    pub search: OnceCell<Rc<SearchBox>>,
    btn_min: gtk::Button,
    btn_max: gtk::Button,
    btn_close: gtk::Button,
    stack: gtk::Stack,
    footer: gtk::Box,
    status: gtk::Label,
    btn_undo: gtk::Button,
    bar: gtk::ProgressBar,
    pub btn_log: gtk::ToggleButton,
    logview: gtk::TextView,
    log_reveal: gtk::Revealer,
    log_box: gtk::ScrolledWindow,
    dim: gtk::Box,
    reveal: gtk::Revealer,
    pub panel: Rc<Panel>,
    undo_fn: RefCell<Option<Box<dyn FnOnce()>>>,
    job: RefCell<Option<(String, Option<Rc<dyn AioPage>>)>>,
    proc_busy: Cell<bool>,
    run_err: RefCell<String>,
    run_result: RefCell<String>,
    task: Cell<bool>,
    pub de_page: OnceCell<Rc<DePage>>,
    pub sddm_page: OnceCell<Rc<ThemePage>>,
    pub ply_page: OnceCell<Rc<ThemePage>>,
    pub settings_page: OnceCell<Rc<SettingsPage>>,
    /// Hypr Studio als eigenes Fenster (Einstellung „Fenster“)
    pub hypr_window: RefCell<Option<Rc<hyprstudio::ui::studio::Studio>>>,
}

impl Win {
    pub fn new(app: &gtk::Application) -> Rc<Win> {
        let mut c = load_cfg();
        if c.s("version") != ed().version {
            // jede neue Version startet auf Englisch
            c.insert("lang".into(), json!("en"));
            c.insert("version".into(), json!(ed().version));
            save_cfg(&c);
        }
        let lang = if c.s("lang").is_empty() { "en".to_string() } else { c.s("lang").to_string() };
        i18n::set_lang(&lang);
        let th = c.s("theme").to_string();
        let th = palettes::legacy(&th).unwrap_or(th);
        let th = if palettes::theme_color(&th).is_some() { th } else { "01".into() };
        c.insert("theme".into(), json!(th));
        if !tabs().contains(&c.s("tab")) {
            c.insert("tab".into(), json!(tabs()[0]));
        }
        let backend = system::detect_backend().or_else(|| usercmd::dry().then(|| "pacman".to_string()));
        let running = system::running_desktop();
        let cfg = Rc::new(RefCell::new(c));

        let window = gtk::ApplicationWindow::new(app);
        window.set_title(Some(ed().name));
        window.set_icon_name(Some(ed().id));
        window.set_decorated(false);
        window.add_css_class("hs-window");
        window.set_default_size(1360, 880);
        window.set_size_request(900, 600);

        let root = vbox(0);
        root.add_css_class("Root");
        root.set_overflow(gtk::Overflow::Hidden);
        let overlay = gtk::Overlay::new();
        overlay.set_vexpand(true);
        root.append(&overlay);
        let content = vbox(14);
        hyprstudio::ui::widgets::margins(&content, 20, 12, 20, 16);
        overlay.set_child(Some(&content));

        // Kopfzeile: Tux + Titel | Tabs mittig | Suche + Fensterknöpfe
        let gear = gtk::Button::new();
        gear.add_css_class("Tool");
        gear.set_size_request(36, 36);
        gear.set_valign(gtk::Align::Center);
        cursor_hand(&gear);
        let title = label(ed().name, "Title");
        let left = hbox(10);
        left.append(&gear);
        left.append(&title);
        let tabs = seg(ed().tabs, "tab.");
        tabs.root.set_visible(ed().tabs.len() > 1);
        tabs.root.remove_css_class("SegBox");
        tabs.root.add_css_class("ATabBox");
        let right = hbox(6);
        right.set_halign(gtk::Align::End);
        let search_slot = hbox(0);
        search_slot.set_margin_end(8);
        right.append(&search_slot);
        let btn_min = tool_button("min", theme::MUTED, 16, "");
        let btn_max = tool_button("max", theme::MUTED, 16, "");
        let btn_close = tool_button("close", theme::MUTED, 16, "CloseBtn");
        for b in [&btn_min, &btn_max, &btn_close] {
            b.set_size_request(32, 32);
            b.set_valign(gtk::Align::Center);
            right.append(b);
        }
        let head = gtk::CenterBox::new();
        head.set_start_widget(Some(&left));
        head.set_center_widget(Some(&tabs.root));
        head.set_end_widget(Some(&right));
        let handle = gtk::WindowHandle::new();
        handle.set_child(Some(&head));
        content.append(&handle);

        let stack = gtk::Stack::new();
        stack.set_vexpand(true);
        stack.set_hhomogeneous(false);
        stack.set_vhomogeneous(false);
        content.append(&stack);

        let footer = hbox(0);
        let status = label("", "Status");
        status.set_hexpand(true);
        status.set_ellipsize(gtk::pango::EllipsizeMode::End);
        let btn_undo = gtk::Button::with_label("");
        cursor_hand(&btn_undo);
        btn_undo.set_visible(false);
        let bar = gtk::ProgressBar::new();
        bar.set_size_request(220, -1);
        bar.set_valign(gtk::Align::Center);
        bar.set_visible(false);
        let btn_log = gtk::ToggleButton::with_label("");
        cursor_hand(&btn_log);
        btn_log.set_margin_start(12);
        footer.append(&status);
        footer.append(&btn_undo);
        footer.append(&bar);
        footer.append(&btn_log);
        content.append(&footer);

        // Protokoll als Schublade über den Kacheln (verschiebt nichts)
        let logview = gtk::TextView::new();
        logview.set_editable(false);
        logview.set_cursor_visible(false);
        logview.add_css_class("ALog");
        logview.set_wrap_mode(gtk::WrapMode::WordChar);
        let log_box = gtk::ScrolledWindow::new();
        log_box.add_css_class("LogDrawer");
        log_box.set_child(Some(&logview));
        let log_reveal = gtk::Revealer::new();
        log_reveal.set_transition_type(gtk::RevealerTransitionType::SlideUp);
        log_reveal.set_transition_duration(200);
        log_reveal.set_valign(gtk::Align::End);
        log_reveal.set_halign(gtk::Align::Fill);
        log_reveal.set_child(Some(&log_box));
        log_reveal.set_visible(false);
        overlay.add_overlay(&log_reveal);

        // Einstellungen: abdunkeln + Panel von links
        let dim = vbox(0);
        dim.add_css_class("Dim");
        dim.set_visible(false);
        overlay.add_overlay(&dim);
        let panel = Panel::new(&cfg.borrow());
        let reveal = gtk::Revealer::new();
        reveal.set_transition_type(gtk::RevealerTransitionType::SlideRight);
        reveal.set_transition_duration(220);
        reveal.set_halign(gtk::Align::Start);
        reveal.set_valign(gtk::Align::Fill);
        reveal.set_child(Some(&panel.root));
        reveal.set_visible(false);
        overlay.add_overlay(&reveal);

        // Größe ändern unten rechts
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
            let p = w.compute_point(&native, &gtk::graphene::Point::new(x as f32, y as f32))
                .map(|p| (p.x() as f64, p.y() as f64)).unwrap_or((x, y));
            top.begin_resize(gdk::SurfaceEdge::SouthEast, Some(&dev), g.current_button() as i32, p.0, p.1,
                g.current_event_time());
        });
        grip.add_controller(g);
        let grip_row = hbox(0);
        grip_row.set_size_request(-1, 20);
        grip_row.append(&spacer());
        grip.set_margin_end(4);
        grip.set_margin_bottom(4);
        grip_row.append(&grip);
        root.append(&grip_row);
        window.set_child(Some(&root));

        let w = Rc::new(Win { cfg, window, backend, running, root, content, gear, tabs, search: OnceCell::new(),
            btn_min, btn_max, btn_close, stack, footer, status, btn_undo, bar, btn_log, logview, log_reveal, log_box,
            dim, reveal, panel, undo_fn: RefCell::new(None), job: RefCell::new(None), proc_busy: Cell::new(false),
            run_err: RefCell::new(String::new()), run_result: RefCell::new(String::new()), task: Cell::new(false),
            de_page: OnceCell::new(), sddm_page: OnceCell::new(), ply_page: OnceCell::new(),
            settings_page: OnceCell::new(), hypr_window: RefCell::new(None) });
        WIN.with(|x| *x.borrow_mut() = Some(w.clone()));
        shell::set_shell(w.clone());
        // Protokoll aus Hintergrund-Aufgaben (thread-sicher) in den Hauptthread
        let (tx, rx) = async_channel::unbounded::<String>();
        usercmd::set_logger(move |s| {
            let _ = tx.send_blocking(s.to_string());
        });
        glib::spawn_future_local(async move {
            while let Ok(line) = rx.recv().await {
                if has_win() {
                    win().log(&line);
                }
            }
        });
        w.apply_theme();

        // SDDM Studio: nur der SDDM-Tab, ohne Suche
        let has = |k: &str| ed().tabs.contains(&k);
        if ed().tabs.len() > 1 {
            let sb = SearchBox::new();
            search_slot.append(&sb.entry);
            let _ = w.search.set(sb);
        }
        if has("de") {
            let de = DePage::new();
            w.stack.add_named(&de.widget(), Some("de"));
            let _ = w.de_page.set(de);
        }
        if has("sddm") {
            let sp = ThemePage::new_sddm();
            w.stack.add_named(&sp.widget(), Some("sddm"));
            let _ = w.sddm_page.set(sp);
        }
        if has("ply") {
            let pp = ThemePage::new_ply();
            w.stack.add_named(&pp.widget(), Some("ply"));
            let _ = w.ply_page.set(pp);
        }
        if has("settings") {
            let st = SettingsPage::new();
            w.stack.add_named(&st.widget(), Some("settings"));
            let _ = w.settings_page.set(st);
        }
        w.connect();
        w.retranslate();
        if let Some(p) = w.sddm_page.get() {
            p.rebuild();
        }
        if let Some(p) = w.ply_page.get() {
            p.rebuild();
        }
        let tab = w.cfg.borrow().s("tab").to_string();
        w.set_tab(&tab);
        if w.backend.is_none() {
            w.set_status(&t("nobackend"));
        } else if !usercmd::dry() && !system::has_pkexec() {
            w.set_status(&t("nopkexec"));
        }
        w
    }

    fn connect(self: &Rc<Self>) {
        self.gear.connect_clicked(|_| win().toggle_settings());
        self.tabs.connect(|tab| win().set_tab(&tab));
        self.btn_min.connect_clicked(|_| win().window.minimize());
        self.btn_max.connect_clicked(|_| {
            let w = win();
            if w.window.is_maximized() { w.window.unmaximize() } else { w.window.maximize() }
        });
        self.btn_close.connect_clicked(|_| win().window.close());
        self.btn_log.connect_toggled(|b| win().toggle_log(b.is_active()));
        self.btn_undo.connect_clicked(|_| win().undo());
        let click = gtk::GestureClick::new();
        click.connect_pressed(|_, _, _, _| win().toggle_settings());
        self.dim.add_controller(click);
        self.reveal.connect_child_revealed_notify(|r| {
            if !r.reveals_child() {
                let w = win();
                w.reveal.set_visible(false);
                w.dim.set_visible(false);
                w.content.remove_css_class("blurred");
            }
        });
        self.log_reveal.connect_child_revealed_notify(|r| {
            if !r.reveals_child() {
                r.set_visible(false);
            }
        });
        for prop in ["default-width", "default-height", "maximized"] {
            self.window.connect_notify_local(Some(prop), |_, _| {
                glib::idle_add_local_once(|| {
                    if has_win() {
                        win().place_log();
                    }
                });
            });
        }
        self.window.connect_close_request(|_| {
            if has_win() {
                save_cfg(&win().cfg.borrow());
            }
            glib::Propagation::Proceed
        });
        // Strg+F: Suche
        let keys = gtk::EventControllerKey::new();
        keys.connect_key_pressed(|_, key, _, mods| {
            if mods.contains(gdk::ModifierType::CONTROL_MASK) && matches!(key, gdk::Key::f | gdk::Key::F) {
                if let Some(s) = win().search.get() {
                    s.entry.grab_focus();
                    s.entry.select_region(0, -1);
                }
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        self.window.add_controller(keys);
    }

    // ----- Schnittstelle für die Seiten -----

    pub fn accent(&self) -> String {
        theme::accent_of(self.cfg.borrow().s("theme"))
    }

    pub fn save(&self) {
        save_cfg(&self.cfg.borrow());
    }

    pub fn conf(&self) -> shell::Conf {
        shell::Conf { cfg: self.cfg.clone(), save: Rc::new(|| win().save()) }
    }

    pub fn parent(&self) -> Option<gtk::Window> {
        Some(self.window.clone().upcast())
    }

    pub fn offer_undo(&self, text: &str, f: Box<dyn FnOnce()>) {
        *self.undo_fn.borrow_mut() = Some(f);
        self.set_status(text);
        self.btn_undo.set_label(&t("reset.undo"));
        self.btn_undo.set_visible(true);
    }

    fn undo(&self) {
        let f = self.undo_fn.borrow_mut().take();
        self.btn_undo.set_visible(false);
        if let Some(f) = f {
            f();
        }
    }

    pub fn busy(&self) -> bool {
        self.proc_busy.get() || self.task.get()
    }

    pub fn set_status(&self, text: &str) {
        self.status.set_text(text);
    }

    pub fn log(&self, text: &str) {
        let buf = self.logview.buffer();
        let mut end = buf.end_iter();
        if buf.char_count() > 0 || buf.line_count() > 1 {
            buf.insert(&mut end, "\n");
        } else if self.logview.has_css_class("started") {
            buf.insert(&mut end, "\n");
        }
        self.logview.add_css_class("started");
        buf.insert(&mut end, text);
        if buf.line_count() > 5000 {
            let mut a = buf.start_iter();
            let mut b = buf.iter_at_line(buf.line_count() - 5000).unwrap_or(buf.start_iter());
            buf.delete(&mut a, &mut b);
        }
        let mark = buf.create_mark(None, &buf.end_iter(), false);
        self.logview.scroll_mark_onscreen(&mark);
        buf.delete_mark(&mark);
    }

    pub fn show_log(&self) {
        if !self.btn_log.is_active() {
            self.btn_log.set_active(true);
        }
    }

    pub fn confirm(&self, title: &str, text: &str, done: impl FnOnce(bool) + 'static) {
        dialogs::confirm(self.parent().as_ref(), title, text, None, false, done);
    }

    pub fn task_begin(&self, text: &str) {
        self.task.set(true);
        self.bar.set_fraction(0.0);
        self.bar.set_visible(true);
        self.set_status(text);
        self.log("");
        self.refresh_pages();
    }

    pub fn task_progress(&self, v: i32) {
        self.bar.set_fraction((v as f64 / 100.0).clamp(0.0, 1.0));
    }

    pub fn task_end(&self, ok: bool, msg: &str) {
        self.task.set(false);
        self.bar.set_visible(false);
        if !ok {
            self.set_status(&format!("{}: {}", t("failed"), msg));
            self.show_log();
        }
        self.refresh_pages();
    }

    // ----- Root-Aufgaben (pkexec) -----

    pub fn run(self: &Rc<Self>, args: Vec<String>, action: &str, page: Option<Rc<dyn AioPage>>) {
        if self.proc_busy.get() {
            return;
        }
        *self.job.borrow_mut() = Some((action.to_string(), page));
        self.bar.set_fraction(0.0);
        self.bar.set_visible(true);
        self.set_status(&t("busy"));
        self.log("");
        self.start_runner(args);
        self.refresh_pages();
    }

    fn start_runner(self: &Rc<Self>, args: Vec<String>) {
        let me = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("linux-aio"));
        let mut env: Vec<(String, String)> = vec![];
        let (prog, argv): (String, Vec<String>) = if usercmd::dry() {
            env.push(("DEMANAGER_DRYRUN".into(), "1".into()));
            let mut a = vec!["--helper".to_string()];
            a.extend(args);
            (me.to_string_lossy().into_owned(), a)
        } else if Path::new(ed().helper).is_file() {
            let mut a = vec![ed().helper.to_string()];
            a.extend(args);
            ("pkexec".into(), a)
        } else {
            // root darf FUSE (Cloud, AppImage) nicht lesen → Kopie in /tmp
            let copy = stage_helper(&me).unwrap_or(me.clone());
            let mut a = vec![copy.to_string_lossy().into_owned(), "--helper".into()];
            a.extend(args);
            ("pkexec".into(), a)
        };
        self.run_err.borrow_mut().clear();
        self.run_result.borrow_mut().clear();
        self.proc_busy.set(true);
        let mut shown = vec![prog.clone()];
        shown.extend(argv.iter().cloned());
        self.log(&format!("$ {}", shown.join(" ")));
        let (tx, rx) = async_channel::unbounded::<RMsg>();
        std::thread::spawn(move || {
            let mut cmd = std::process::Command::new("sh");
            cmd.arg("-c").arg("exec \"$0\" \"$@\" 2>&1").arg(&prog).args(&argv)
                .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::piped())
                .env_remove("DESKTOP_STARTUP_ID").env_remove("XDG_ACTIVATION_TOKEN");
            for (k, v) in &env {
                cmd.env(k, v);
            }
            let code = match cmd.spawn() {
                Err(e) => {
                    let _ = tx.send_blocking(RMsg::Line(e.to_string()));
                    127
                }
                Ok(mut ch) => {
                    if let Some(out) = ch.stdout.take() {
                        let mut r = std::io::BufReader::new(out);
                        let mut buf = Vec::new();
                        loop {
                            buf.clear();
                            match r.read_until(b'\n', &mut buf) {
                                Ok(0) | Err(_) => break,
                                Ok(_) => {
                                    let s = String::from_utf8_lossy(&buf);
                                    let s = s.trim_end_matches('\n').trim_end_matches('\r').to_string();
                                    let _ = tx.send_blocking(RMsg::Line(s));
                                }
                            }
                        }
                    }
                    ch.wait().ok().and_then(|s| s.code()).unwrap_or(-1)
                }
            };
            let _ = tx.send_blocking(RMsg::Done(code));
        });
        glib::spawn_future_local(async move {
            while let Ok(m) = rx.recv().await {
                let w = win();
                match m {
                    RMsg::Line(ln) => w.runner_line(&ln),
                    RMsg::Done(code) => {
                        let err = {
                            let e = w.run_err.borrow().clone();
                            if !e.is_empty() { e } else if code == 126 || code == 127 { "cancel".into() } else { String::new() }
                        };
                        w.proc_busy.set(false);
                        w.finished(code == 0, &err);
                        break;
                    }
                }
            }
        });
    }

    fn runner_line(&self, ln: &str) {
        if let Some(s) = ln.strip_prefix("@@STEP ") {
            self.set_status(&t(&format!("step.{s}")));
        } else if let Some(p) = ln.strip_prefix("@@PROGRESS ") {
            if let Ok(v) = p.trim().parse::<i32>() {
                self.task_progress(v);
            }
        } else if ln.starts_with("@@FAIL") {
            *self.run_err.borrow_mut() = ln.get(7..).unwrap_or("").to_string();
        } else if let Some(r) = ln.strip_prefix("@@THEME ") {
            *self.run_result.borrow_mut() = r.trim().to_string();
        } else if ln.starts_with("@@OK") {
        } else if !ln.is_empty() {
            self.log(ln);
        }
    }

    fn finished(self: &Rc<Self>, ok: bool, err: &str) {
        self.bar.set_visible(false);
        let (action, page) = self.job.borrow_mut().take().unwrap_or((String::new(), None));
        if ok {
            let mut msg = t("done");
            if action == "default" || action == "sddm-set" {
                msg += &format!(" · {}", t("logout"));
            } else if matches!(action.as_str(), "dm-enable" | "ply-setup" | "ply-set") {
                msg += &format!(" · {}", t("reboot"));
            }
            self.set_status(&msg);
        } else {
            let (key, detail) = match err.split_once(' ') {
                Some((k, d)) => (k.to_string(), d.to_string()),
                None => (err.to_string(), String::new()),
            };
            let text = if i18n::TEXT.has("de", &format!("err.{key}")) { t(&format!("err.{key}")) } else { t("failed") };
            let det = if detail.is_empty() { String::new() } else { format!(" ({detail})") };
            self.set_status(&format!("{}: {}{}", t("failed"), text, det));
            self.show_log();
        }
        let result = self.run_result.borrow().clone();
        if let Some(p) = page {
            p.finished(ok, &action, &result);
        }
        self.refresh_pages();
    }

    pub fn refresh_pages(&self) {
        if let Some(p) = self.de_page.get() {
            p.refresh();
        }
        if let Some(p) = self.sddm_page.get() {
            p.refresh();
        }
        if let Some(p) = self.ply_page.get() {
            p.refresh();
        }
        if let Some(p) = self.settings_page.get() {
            p.refresh();
        }
    }

    // ----- Protokoll-Schublade -----

    fn current_page(&self) -> Option<Rc<dyn AioPage>> {
        let tab = self.cfg.borrow().s("tab").to_string();
        match tab.as_str() {
            "de" => self.de_page.get().map(|p| p.clone() as Rc<dyn AioPage>),
            "sddm" => self.sddm_page.get().map(|p| p.clone() as Rc<dyn AioPage>),
            "ply" => self.ply_page.get().map(|p| p.clone() as Rc<dyn AioPage>),
            _ => self.settings_page.get().map(|p| p.clone() as Rc<dyn AioPage>),
        }
    }

    fn place_log(&self) {
        let Some(foot) = self.footer.compute_bounds(&self.root) else { return };
        let bottom = foot.y() as f64 - 8.0;
        let top = self.current_page().and_then(|p| p.drawer_anchor().compute_bounds(&self.root))
            .map(|b| b.y() as f64).unwrap_or(60.0);
        let rh = self.root.height() as f64;
        self.log_reveal.set_margin_start(foot.x() as i32);
        self.log_reveal.set_margin_end((self.root.width() as f32 - foot.x() - foot.width()).max(0.0) as i32);
        self.log_reveal.set_margin_bottom((rh - bottom).max(0.0) as i32);
        self.log_box.set_size_request(-1, (bottom - top).max(60.0) as i32);
    }

    fn toggle_log(&self, on: bool) {
        self.place_log();
        if on {
            self.log_reveal.set_visible(true);
        }
        self.log_reveal.set_reveal_child(on);
    }

    // ----- Tabs -----

    pub fn set_tab(self: &Rc<Self>, tab: &str) {
        let tab = if tabs().contains(&tab) { tab } else { tabs()[0] };
        self.cfg.borrow_mut().insert("tab".into(), json!(tab));
        self.save();
        self.tabs.select(tab);
        self.stack.set_visible_child_name(tab);
        if self.btn_log.is_active() {
            glib::idle_add_local_once(|| win().place_log());
        }
    }

    // ----- Einstellungen -----

    pub fn toggle_settings(self: &Rc<Self>) {
        let opening = !self.reveal.reveals_child();
        if opening {
            self.content.add_css_class("blurred");
            self.dim.set_visible(true);
            self.panel.sync(&self.cfg.borrow(), self.backend.as_deref());
            self.reveal.set_visible(true);
            self.reveal.set_reveal_child(true);
        } else {
            self.reveal.set_reveal_child(false);
        }
    }

    pub fn app_opt(&self, key: &str, value: &str) {
        self.cfg.borrow_mut().insert(key.into(), json!(value));
        self.save();
        self.panel.b_restart.set_visible(true);
        self.set_status(&t("app.restart.need"));
    }

    pub fn show_about(&self) {
        about_dialog(self);
    }

    // ----- Preset-Datei (alles, was Linux AIO gesetzt hat) -----

    pub fn export_preset(self: &Rc<Self>) {
        let start = usercmd::real_home().join("my-setup.linuxaio");
        dialogs::save_file(self.parent().as_ref(), &t("preset.export"), &start.to_string_lossy(),
            &[("Linux AIO (*.linuxaio)", &["*.linuxaio"])], |p| {
                let Some(p) = p else { return };
                let w = win();
                let res = presetfile::export(&w.cfg.borrow(), &p);
                match res {
                    Ok(out) => w.set_status(&tf("preset.exported", kw!("path" => out.to_string_lossy()))),
                    Err(e) => w.set_status(&format!("{}: {}", t("failed"), e)),
                }
            });
    }

    pub fn import_preset(self: &Rc<Self>) {
        let start = usercmd::real_home();
        dialogs::open_file(self.parent().as_ref(), &t("preset.import"), &start.to_string_lossy(),
            &[("Linux AIO (*.linuxaio)", &["*.linuxaio"])], |p| {
                let Some(p) = p else { return };
                let w = win();
                let data = match presetfile::load(&p) {
                    Ok(d) => d,
                    Err(e) => {
                        w.set_status(&format!("{}: {}", t("preset.bad"), e));
                        return;
                    }
                };
                let ch = presetfile::changes(&data, &w.cfg.borrow());
                if ch.is_empty() {
                    w.set_status(&t("preset.same"));
                    return;
                }
                let mut lines: Vec<String> = ch.iter().take(30).map(|(a, n, o, v)| {
                    let show_old = !o.is_null() && o.as_str() != Some("…");
                    if show_old {
                        format!("• {a}: {n}  ({} → {})", hyprstudio::jsonx::py_str(o), hyprstudio::jsonx::py_str(v))
                    } else {
                        format!("• {a}: {n}")
                    }
                }).collect();
                if ch.len() > 30 {
                    lines.push(format!("… +{}", ch.len() - 30));
                }
                let text = format!("{}\n\n{}", tf("preset.preview", kw!("n" => ch.len())), lines.join("\n"));
                dialogs::confirm(w.parent().as_ref(), &t("preset.import"), &text, Some(&t("preset.apply")), false,
                    move |ok| {
                        if !ok {
                            return;
                        }
                        let w = win();
                        let bak = presetfile::backup_current(&w.cfg.borrow()).unwrap_or_default();
                        w.log("");
                        w.task_begin(&t("preset.applying"));
                        let mut cfg = w.cfg.borrow().clone();
                        shell::run_job(move |log| {
                            let s = presetfile::apply(&data, &mut cfg, &|x| log(x));
                            (s, cfg)
                        }, move |(sum, cfg)| {
                            let w = win();
                            w.task_end(true, "");
                            *w.cfg.borrow_mut() = cfg;
                            w.save();
                            let mut msg = tf("preset.done", kw!("n" => sum.gsettings, "path" => bak.to_string_lossy()));
                            if !sum.errors.is_empty() {
                                msg += &format!(" · {}", t("preset.someerr"));
                                for e in &sum.errors {
                                    w.log(e);
                                }
                            }
                            w.set_status(&msg);
                            w.retranslate();
                        });
                    });
            });
    }

    // ----- Updates -----

    pub fn check_update(self: &Rc<Self>, quiet: bool) {
        shell::run_bg(|| updater::latest(None), move |res| {
            let w = win();
            match res {
                Err(e) => {
                    if !quiet {
                        let s = e.to_string();
                        w.set_status(&if s.contains("offline") { t("upd.offline") } else { format!("{}: {}", t("failed"), s) });
                    }
                }
                Ok(info) => {
                    if !updater::newer(info.as_ref()) {
                        if !quiet {
                            w.set_status(&tf("upd.none", kw!("ver" => ed().version)));
                        }
                        return;
                    }
                    let Some(info) = info else { return };
                    if quiet {
                        w.set_status(&tf("upd.available", kw!("ver" => info.version)));
                        return;
                    }
                    w.offer_update(info);
                }
            }
        });
    }

    fn offer_update(self: &Rc<Self>, info: updater::Info) {
        let text = format!("{}\n\n{}", tf("upd.ask", kw!("ver" => info.version, "cur" => ed().version)),
            updater::short_notes(&info.notes, 12));
        dialogs::confirm(self.parent().as_ref(), &t("upd.check"), &text, Some(&t("upd.install")), false, move |ok| {
            if !ok {
                return;
            }
            let w = win();
            w.task_begin(&tf("upd.loading", kw!("ver" => info.version)));
            let (ptx, prx) = async_channel::unbounded::<u32>();
            glib::spawn_future_local(async move {
                while let Ok(v) = prx.recv().await {
                    win().task_progress(v as i32);
                }
            });
            let inf = info.clone();
            shell::run_job(move |log| {
                let mut prog = |v: u32| {
                    let _ = ptx.send_blocking(v);
                };
                updater::install(&inf, &mut prog, &|x| log(x))
            }, move |res| {
                let w = win();
                match res {
                    Err(e) => {
                        w.task_end(false, "");
                        let s = e.to_string();
                        w.set_status(&if s.contains("offline") { t("upd.offline") } else { format!("{}: {}", t("failed"), s) });
                    }
                    Ok(path) => {
                        w.task_end(true, "");
                        w.set_status(&tf("upd.done", kw!("ver" => info.version)));
                        dialogs::confirm(w.parent().as_ref(), &t("upd.check"), &tf("upd.restart", kw!("ver" => info.version)),
                            Some(&t("app.restart")), false, move |ok| {
                                if ok {
                                    let _ = util::spawn_detached(&[&path.to_string_lossy()]);
                                    win().quit();
                                }
                            });
                    }
                }
            });
        });
    }

    pub fn quit(&self) {
        save_cfg(&self.cfg.borrow());
        if let Some(app) = self.window.application() {
            app.quit();
        }
    }

    pub fn set_language(self: &Rc<Self>, code: &str) {
        self.cfg.borrow_mut().insert("lang".into(), json!(code));
        i18n::set_lang(code);
        self.save();
        self.retranslate();
        self.panel.sync(&self.cfg.borrow(), self.backend.as_deref());
    }

    pub fn set_theme(self: &Rc<Self>, name: &str) {
        self.cfg.borrow_mut().insert("theme".into(), json!(name));
        self.save();
        self.apply_theme();
        self.refresh_pages();
        if let Some(p) = self.settings_page.get() {
            p.apply_accent();
        }
        self.panel.sync(&self.cfg.borrow(), self.backend.as_deref());
    }

    pub fn ui_factor(&self) -> f64 {
        let c = self.cfg.borrow();
        let v = match c.get("ui_scale") {
            Some(Value::String(s)) => s.parse::<f64>().ok(),
            Some(Value::Number(n)) => n.as_f64(),
            _ => Some(100.0),
        };
        v.map(|x| x / 100.0).unwrap_or(1.0)
    }

    /// Live, in beide Richtungen, alle offenen Fenster – immer aus den Grundgrößen gerechnet.
    pub fn set_ui_scale(self: &Rc<Self>, value: &str) {
        self.cfg.borrow_mut().insert("ui_scale".into(), json!(value));
        self.save();
        self.apply_theme();
        self.set_status(&format!("{}: {} %", t("app.scale"), value));
    }

    pub fn apply_theme(&self) {
        style::apply(&self.accent(), self.ui_factor());
        self.gear.set_child(Some(&app_icon(26)));
        hyprstudio::ui::widgets::set_tool_icon(&self.btn_min, "min", theme::MUTED, 16);
        hyprstudio::ui::widgets::set_tool_icon(&self.btn_max, "max", theme::MUTED, 16);
        hyprstudio::ui::widgets::set_tool_icon(&self.btn_close, "close", theme::MUTED, 16);
        hyprstudio::ui::widgets::recolor_switches(&self.accent());
    }

    pub fn retranslate(self: &Rc<Self>) {
        self.gear.set_tooltip_text(Some(&t("app.settings")));
        self.panel.retranslate();
        self.btn_min.set_tooltip_text(Some(&t("minimize")));
        self.btn_max.set_tooltip_text(Some(&t("maximize")));
        self.btn_close.set_tooltip_text(Some(&t("close")));
        self.btn_log.set_label(&t("log"));
        self.tabs.retranslate();
        if let Some(s) = self.search.get() {
            s.retranslate();
        }
        if let Some(p) = self.de_page.get() {
            p.retranslate();
        }
        if let Some(p) = self.sddm_page.get() {
            p.retranslate();
        }
        if let Some(p) = self.ply_page.get() {
            p.retranslate();
        }
        if let Some(p) = self.settings_page.get() {
            p.retranslate();
        }
        if !self.busy() {
            self.set_status(&t("ready"));
        }
    }

    pub fn present(&self) {
        self.window.present();
    }
}

impl Shell for Win {
    fn set_status(&self, text: &str) {
        Win::set_status(self, text)
    }
    fn log(&self, text: &str) {
        Win::log(self, text)
    }
    fn bar_busy(&self, on: bool) {
        self.bar.set_visible(on);
        if on {
            self.bar.pulse();
        }
    }
    fn accent(&self) -> String {
        Win::accent(self)
    }
    fn cfg(&self) -> Rc<RefCell<Obj>> {
        self.cfg.clone()
    }
    fn save_later(&self) {
        self.save()
    }
    fn window(&self) -> Option<gtk::Window> {
        self.parent()
    }
    fn show_log(&self) {
        Win::show_log(self)
    }
}

/// Eigene Programmdatei nach /tmp kopieren (root darf FUSE-Mounts wie AppImage/Cloud nicht lesen).
fn stage_helper(me: &Path) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("linuxaio-helper-{}-{}", std::process::id(), stamp));
    std::fs::create_dir_all(&dir).ok()?;
    let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755));
    let dest = dir.join("linux-aio");
    std::fs::copy(me, &dest).ok()?;
    let _ = std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755));
    Some(dest)
}

/// Über-Dialog im App-Look: rahmenlos, rund, eigene Titelzeile.
fn about_dialog(w: &Win) {
    let f = dialogs::Frame::new(w.parent().as_ref(), &t("about"), 460);
    f.title.set_visible(false);
    f.body.set_spacing(10);
    let top = hbox(16);
    top.append(&app_icon(72));
    let col = vbox(0);
    col.append(&label(ed().name, "DeName"));
    col.append(&label(&format!("{} {} · LabtosoStudios", t("about.version"), ed().version), "ALabel"));
    top.append(&col);
    f.body.append(&top);
    let markup = tf("about.text", kw!("url" => crate::REPO_URL)).replace("<br>", "\n");
    let txt = gtk::Label::new(None);
    txt.set_markup(&markup);
    txt.set_wrap(true);
    txt.set_xalign(0.0);
    txt.set_size_request(410, -1);
    txt.set_max_width_chars(52);
    f.body.append(&txt);
    let row = hbox(0);
    let gh = gtk::Button::with_label("GitHub");
    cursor_hand(&gh);
    gh.connect_clicked(|_| {
        let _ = gtk::gio::AppInfo::launch_default_for_uri(crate::REPO_URL, gtk::gio::AppLaunchContext::NONE);
    });
    let ok = gtk::Button::with_label(&t("close"));
    ok.add_css_class("Primary");
    cursor_hand(&ok);
    row.append(&gh);
    row.append(&spacer());
    row.append(&ok);
    f.body.append(&row);
    let fw: Weak<dialogs::Frame> = Rc::downgrade(&f);
    ok.connect_clicked(move |_| {
        if let Some(f) = fw.upgrade() {
            f.finish(true);
        }
    });
    f.run(|_| {});
}

/// Start: Fenster, Aufräumen, Update-Hinweis, Bildschirmfotos (Testhilfe).
pub fn activate(app: &gtk::Application) {
    if let Some(w) = app.active_window() {
        w.present();
        return;
    }
    let w = Win::new(app);
    w.present();
    shell::later(800, || integrate());
    shell::later(1200, || {
        shell::run_bg(safety::cleanup, |res| {
            if !res.is_empty() {
                win().set_status(&t("safety.cleaned"));
            }
        });
    });
    let shot = std::env::var("DEMANAGER_SHOT").unwrap_or_default();
    let hint = w.cfg.borrow().get("update_hint").map(hyprstudio::jsonx::truthy).unwrap_or(true);
    if hint && shot.is_empty() {
        shell::later(4000, || win().check_update(true));
    }
    if !shot.is_empty() {
        let steps = std::env::var("LINUXAIO_STEPS").unwrap_or_default();
        shell::later(600, move || shots(shot, steps));
    }
}

/// AppImage/einzelne Datei: Startmenü-Eintrag + Symbol mit der App-ID des Fensters.
fn integrate() {
    let render = |size: i32, path: &Path| {
        if let Some(pb) = super::img::from_bytes_at(crate::edition::ed().icon_svg, size, size) {
            let _ = pb.savev(path, "png", &[]);
        }
    };
    let _ = crate::integrate::ensure(Some(&render), &|x| usercmd::log(x));
}

/// Testhilfe: LINUXAIO_STEPS='settings:gnome:cursor,sddm' → je ein Bild (Tab[:Desktop[:Bereich[:Unterseite]]]).
fn shots(shot: String, steps: String) {
    let items: Vec<String> = steps.split(',').filter(|s| !s.is_empty()).map(String::from).collect();
    let single = items.len() <= 1;
    let items = if items.is_empty() { vec![String::new()] } else { items };
    let wait: u64 = std::env::var("LINUXAIO_WAIT").ok().and_then(|s| s.parse().ok()).unwrap_or(12);
    fn step(i: usize, items: Vec<String>, shot: String, single: bool, wait: u64) {
        let w = win();
        if i >= items.len() {
            w.quit();
            std::process::exit(0);
        }
        let st = items[i].clone();
        if !st.is_empty() {
            let parts: Vec<&str> = st.split(':').collect();
            if parts[0] == "go" {
                // Suche testen: Text eintippen, ersten Treffer öffnen
                if let Some(s) = w.search.get() {
                    s.entry.set_text(parts[1]);
                    s.debug_dump();
                    s.entry.emit_activate();
                }
            } else {
                w.set_tab(parts[0]);
                // sddm:store / ply:store → Store-Ansicht
                if parts.len() > 1 && (parts[0] == "sddm" || parts[0] == "ply") {
                    let p = if parts[0] == "sddm" { w.sddm_page.get() } else { w.ply_page.get() };
                    if let Some(p) = p {
                        p.show_source(parts[1]);
                    }
                }
            }
            if let Some(sp) = w.settings_page.get() {
                if parts[0] == "settings" && parts.len() > 2 && parts[1] == "adv" {
                    sp.select_adv(parts[2]);
                } else if parts[0] == "settings" && parts.len() > 1 {
                    sp.select(parts[1]);
                    if parts.len() > 2 {
                        sp.show_section(parts[2], parts.get(3).copied());
                    }
                }
            }
        }
        shell::later(wait * 100, move || {
            let w = win();
            let path = if single { shot.clone() } else { shot.replace(".png", &format!("_{i}.png")) };
            hyprstudio::ui::studio::save_png(w.window.upcast_ref(), &path);
            step(i + 1, items, shot, single, wait);
        });
    }
    step(0, items, shot, single, wait);
}

#[allow(dead_code)]
fn _unused(_: draw::Rgba) {}
