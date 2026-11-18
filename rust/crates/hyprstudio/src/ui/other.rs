//! Tab „Sonstiges“: Config-Fehler reparieren, Werkzeuge, Sicherungen wiederherstellen, Pakete, Leistung,
//! Updates, Polkit, Laptop-Tasten, Hell/Dunkel, Scrollrichtung; dazu Kitty, Hintergrundbild, Mauszeiger, Sperre.

use super::dialogs;
use super::gate::Gate;
use super::kitty_page::{kitty_program, KittyEditor, TermWin};
use super::lock_page::LockEditor;
use super::paper_page::{CursorEditor, PaperEditor};
use super::shell;
use super::studio::Studio;
use super::widgets::{button, chip, hbox, label, margins, nav_button, nav_groups, nav_set_text, scroller, set_chip,
                     spacer, vbox, Card, Host, NavHeader, Segmented, Switch};
use crate::i18n::{t, tf, TEXT};
use crate::jsonx::{Get, Obj};
use crate::{appearance, fixer, hardware, kw, laptopkeys, model, online, packages, perf, util};
use gtk::prelude::*;
use gtk::{glib, pango};
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};

pub const SECTIONS: &[&str] = &["fix", "perf", "install", "updates", "polkit", "laptop", "kitty", "look", "scroll",
    "tools", "backups"];

fn nav_layout() -> Vec<(&'static str, Vec<&'static str>)> {
    vec![("system", vec!["fix", "perf", "install", "updates", "polkit", "tools", "backups"]),
        ("devices", vec!["laptop", "scroll"]), ("look", vec!["look", "kitty"])]
}

fn page() -> (gtk::ScrolledWindow, gtk::Box) {
    let inner = vbox(12);
    inner.add_css_class("ScrollInner");
    inner.set_margin_end(6);
    (scroller(&inner), inner)
}

fn pad<W: IsA<gtk::Widget>>(w: &W) {
    w.set_margin_start(18);
    w.set_margin_end(18);
}

/// Widget direkt unter den Kartentitel (bzw. hinter `after`) setzen.
fn insert_after<W: IsA<gtk::Widget>, A: IsA<gtk::Widget>>(card: &Card, w: &W, after: &A) {
    pad(w);
    card.root.insert_child_after(w, Some(after));
}

fn bind_row(l: i32, tp: i32, r: i32, b: i32, spacing: i32) -> gtk::Box {
    let row = hbox(spacing);
    row.add_css_class("BindRow");
    let p = gtk::CssProvider::new();
    p.load_from_string(&format!("box {{ padding: {tp}px {r}px {b}px {l}px; }}"));
    #[allow(deprecated)]
    row.style_context().add_provider(&p, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1);
    row
}

fn btn(text: &str, class: &str) -> gtk::Button {
    button(text, class)
}

struct PkgRow {
    cb: gtk::CheckButton,
    desc: gtk::Label,
    chip: gtk::Label,
    act: gtk::Button,
}

pub struct OtherPage {
    pub root: gtk::Box,
    win: Weak<Studio>,
    errors: RefCell<Vec<fixer::ErrorEntry>>,
    side: gtk::Box,
    side_end: gtk::Box,
    nav: RefCell<Vec<(String, gtk::ToggleButton)>>,
    sections: RefCell<Vec<String>>,
    heads: Vec<Rc<NavHeader>>,
    title: gtk::Label,
    chip: gtk::Label,
    stack: gtk::Stack,
    // Reparatur
    btn_scan: gtk::Button,
    btn_all: gtk::Button,
    btn_reload: gtk::Button,
    err_box: gtk::Box,
    // Werkzeuge
    tool_btns: Vec<(String, gtk::Button)>,
    // Sicherungen
    btn_bak_refresh: gtk::Button,
    bak_box: gtk::Box,
    // Installieren
    btn_inst: gtk::Button,
    btn_rm: gtk::Button,
    pm_label: gtk::Label,
    pkg_heads: Vec<(String, gtk::Label)>,
    pkg_rows: Vec<(String, PkgRow)>,
    proc: Cell<bool>,
    pkg_out: RefCell<Vec<String>>,
    watch: RefCell<Option<glib::SourceId>>,
    // Hell/Dunkel
    look_card: RefCell<Option<Rc<Card>>>,
    look_seg: Rc<Segmented>,
    look_state: gtk::Label,
    look_qt: gtk::Label,
    look_qt_btn: gtk::Button,
    // Leistung
    pf_card: RefCell<Option<Rc<Card>>>,
    pf_state: gtk::Label,
    pf_check: gtk::Button,
    pf_all: gtk::Button,
    pf_rows: Vec<(String, gtk::Label, gtk::Button)>,
    pf_crash_card: RefCell<Option<Rc<Card>>>,
    pf_crash: gtk::Label,
    pf_info: RefCell<Option<Obj>>,
    pf_job: Cell<bool>,
    // Updates
    up_card: RefCell<Option<Rc<Card>>>,
    up_state: gtk::Label,
    up_check: gtk::Button,
    up_all: gtk::Button,
    up_rows: Vec<(String, gtk::Label, gtk::Button)>,
    #[allow(clippy::type_complexity)]
    up_info: RefCell<Option<(HashMap<String, String>, Option<HashMap<String, String>>)>>,
    up_job: Cell<bool>,
    // Polkit
    pk_card: RefCell<Option<Rc<Card>>>,
    pk_chip: gtk::Label,
    pk_run: gtk::Label,
    pk_install: gtk::Button,
    pk_start: gtk::Button,
    pk_sw: Rc<Switch>,
    pk_hint: gtk::Label,
    // Laptop
    lt_card: RefCell<Option<Rc<Card>>>,
    lt_dev: gtk::Label,
    lt_chips: Vec<(String, gtk::Label)>,
    lt_install: gtk::Button,
    lt_sw: Rc<Switch>,
    lt_tests: Vec<(gtk::Button, String)>,
    // Scrollen
    scroll_card: RefCell<Option<Rc<Card>>>,
    scroll_sw: Vec<(String, Rc<Switch>)>,
    accel_sw: Rc<Switch>,
    apply_later: RefCell<Option<glib::SourceId>>,
    // eingebettete Editoren
    pub kitty: RefCell<Option<Rc<KittyEditor>>>,
    pub paper: Rc<PaperEditor>,
    pub cursor: Rc<CursorEditor>,
    pub lock: Rc<LockEditor>,
    gates: RefCell<Vec<(String, Rc<Gate>)>>,
    adv_retr: RefCell<Option<Box<dyn Fn()>>>,
}

thread_local! {
    static OTHERS: RefCell<Vec<Weak<OtherPage>>> = const { RefCell::new(Vec::new()) };
}

impl Host for OtherPage {
    /// Regler unter Maus & Touchpad lesen/schreiben den Hyprland-Zustand.
    fn value(&self, key: &str) -> Value {
        self.win.upgrade().and_then(|w| w.hp()).map(|hp| hp.state.borrow().val(key).clone()).unwrap_or(Value::Null)
    }
    fn set_value(&self, key: &str, v: Value) {
        if let Some(me) = self.me() {
            me.set(key, v);
        }
    }
}

impl OtherPage {
    fn me(&self) -> Option<Rc<OtherPage>> {
        OTHERS.with(|p| p.borrow().iter().filter_map(|w| w.upgrade()).find(|x| std::ptr::eq(x.as_ref(), self)))
    }

    fn win(&self) -> Rc<Studio> {
        self.win.upgrade().expect("Fenster")
    }

    pub fn new(win: &Rc<Studio>) -> Rc<OtherPage> {
        let conf = win.conf();
        let root = hbox(14);
        let side = vbox(4);
        side.add_css_class("Side");
        side.set_size_request(190, -1);
        let mut nav = vec![];
        let mut first: Option<gtk::ToggleButton> = None;
        for key in SECTIONS {
            let b = nav_button(first.as_ref());
            if first.is_none() {
                first = Some(b.clone());
            }
            nav.push((key.to_string(), b));
        }
        let heads = nav_groups(&side, &nav, &nav_layout(), "other", &conf);
        let side_end = super::widgets::vspacer();
        side.append(&side_end);
        root.append(&side);

        let right = vbox(14);
        right.set_hexpand(true);
        let head = hbox(0);
        let title = label("", "PageTitle");
        title.set_size_request(-1, 40);
        let chip_l = chip();
        chip_l.set_margin_start(10);
        head.append(&title);
        head.append(&chip_l);
        right.append(&head);
        let stack = gtk::Stack::new();
        stack.set_vexpand(true);
        stack.set_hhomogeneous(false);
        stack.set_vhomogeneous(false);
        right.append(&stack);
        root.append(&right);

        // --- Reparatur ---
        let (p_fix, fl) = page();
        let top = hbox(6);
        let btn_scan = btn("", "Primary");
        let btn_all = btn("", "");
        let btn_reload = btn("", "");
        for b in [&btn_scan, &btn_all, &btn_reload] {
            top.append(b);
        }
        fl.append(&top);
        let err_box = vbox(8);
        fl.append(&err_box);
        stack.add_named(&p_fix, Some("fix"));

        // --- Werkzeuge ---
        let (p_tools, tl) = page();
        let grid = gtk::Grid::new();
        grid.set_row_spacing(10);
        grid.set_column_spacing(10);
        grid.set_column_homogeneous(true);
        let mut tool_btns = vec![];
        for (i, k) in ["reload", "waybar", "open_hypr", "open_waybar", "open_launcher", "open_kitty", "open_backups"]
            .iter().enumerate() {
            let b = btn("", "");
            b.set_size_request(-1, 44);
            b.set_hexpand(true);
            grid.attach(&b, (i % 2) as i32, (i / 2) as i32, 1, 1);
            tool_btns.push((k.to_string(), b));
        }
        tl.append(&grid);
        stack.add_named(&p_tools, Some("tools"));

        // --- Sicherungen ---
        let (p_bak, bl) = page();
        let btop = hbox(6);
        let btn_bak_refresh = btn("", "");
        btop.append(&btn_bak_refresh);
        bl.append(&btop);
        let bak_box = vbox(6);
        bl.append(&bak_box);
        stack.add_named(&p_bak, Some("backups"));

        // --- Installieren ---
        let (p_inst, il) = page();
        let itop = hbox(6);
        let btn_inst = btn("", "Primary");
        let btn_rm = btn("", "DangerFill");
        let pm_label = label("", "RowLabel");
        pm_label.set_margin_start(10);
        itop.append(&btn_inst);
        itop.append(&btn_rm);
        itop.append(&pm_label);
        il.append(&itop);
        let mut pkg_heads = vec![];
        let mut pkg_rows = vec![];
        for g in packages::GROUPS {
            let h = label("", "CardTitle");
            il.append(&h);
            pkg_heads.push((g.to_string(), h));
            for p in packages::PACKAGES.iter().filter(|p| p.group == *g) {
                let row = bind_row(14, 8, 12, 8, 6);
                let cb = gtk::CheckButton::new();
                super::widgets::cursor_hand(&cb);
                let desc = label("", "RowLabel");
                desc.set_hexpand(true);
                desc.set_xalign(0.0);
                desc.set_ellipsize(pango::EllipsizeMode::End);
                let ch = chip();
                let act = btn("", "");
                act.set_size_request(120, -1);
                row.append(&cb);
                row.append(&desc);
                row.append(&ch);
                row.append(&act);
                il.append(&row);
                pkg_rows.push((p.key.to_string(), PkgRow { cb, desc, chip: ch, act }));
            }
        }
        stack.add_named(&p_inst, Some("install"));

        let pf_rows: Vec<(String, gtk::Label, gtk::Button)> = ["portal", "swap", "power", "effects", "disk"].iter()
            .map(|k| (k.to_string(), chip(), btn("", ""))).collect();
        let up_rows: Vec<(String, gtk::Label, gtk::Button)> = packages::UPDATE_GROUPS.iter()
            .map(|(g, _)| (g.to_string(), label("", "RowLabel"), btn("", ""))).collect();
        let lt_chips: Vec<(String, gtk::Label)> = ["laptop", "brightnessctl", "volume", "playerctl"].iter()
            .map(|k| (k.to_string(), chip())).collect();
        let lt_tests: Vec<(gtk::Button, String)> = ["bright_down", "bright_up", "vol_down", "vol_up", "mute"].iter()
            .map(|k| (btn("", ""), k.to_string())).collect();
        let scroll_sw: Vec<(String, Rc<Switch>)> = ["mouse_natural", "natural_scroll"].iter()
            .map(|k| (k.to_string(), Switch::new())).collect();

        let paper = PaperEditor::new(win);
        let cursor = CursorEditor::new(win);
        let lock = LockEditor::new(win);
        let o = Rc::new(OtherPage { root, win: Rc::downgrade(win), errors: RefCell::new(vec![]), side, side_end,
            nav: RefCell::new(nav), sections: RefCell::new(SECTIONS.iter().map(|s| s.to_string()).collect()), heads,
            title, chip: chip_l, stack, btn_scan, btn_all, btn_reload, err_box, tool_btns, btn_bak_refresh, bak_box,
            btn_inst, btn_rm, pm_label, pkg_heads, pkg_rows, proc: Cell::new(false), pkg_out: RefCell::new(vec![]),
            watch: RefCell::new(None), look_card: RefCell::new(None),
            look_seg: Segmented::new(appearance::MODES, "look."), look_state: label("", "RowLabel"),
            look_qt: label("", "RowLabel"), look_qt_btn: btn("", ""), pf_card: RefCell::new(None),
            pf_state: label("", "RowLabel"), pf_check: btn("", ""), pf_all: btn("", "Primary"), pf_rows,
            pf_crash_card: RefCell::new(None), pf_crash: label("", "RowLabel"), pf_info: RefCell::new(None),
            pf_job: Cell::new(false), up_card: RefCell::new(None), up_state: label("", "RowLabel"),
            up_check: btn("", ""), up_all: btn("", "Primary"), up_rows, up_info: RefCell::new(None),
            up_job: Cell::new(false), pk_card: RefCell::new(None), pk_chip: chip(), pk_run: chip(),
            pk_install: btn("", "Primary"), pk_start: btn("", ""), pk_sw: Switch::new(), pk_hint: label("", "RowLabel"),
            lt_card: RefCell::new(None), lt_dev: label("", "PageTitle"), lt_chips, lt_install: btn("", "Primary"),
            lt_sw: Switch::new(), lt_tests, scroll_card: RefCell::new(None), scroll_sw, accel_sw: Switch::new(),
            apply_later: RefCell::new(None), kitty: RefCell::new(None), paper, cursor, lock,
            gates: RefCell::new(vec![]), adv_retr: RefCell::new(None) });
        OTHERS.with(|x| x.borrow_mut().push(Rc::downgrade(&o)));
        o.build(win);
        o
    }

    fn on(self: &Rc<Self>, f: impl Fn(&Rc<OtherPage>) + 'static) -> impl Fn() + 'static {
        let w = Rc::downgrade(self);
        move || {
            if let Some(o) = w.upgrade() {
                f(&o);
            }
        }
    }

    fn click(self: &Rc<Self>, b: &gtk::Button, f: impl Fn(&Rc<OtherPage>) + 'static) {
        let g = self.on(f);
        b.connect_clicked(move |_| g());
    }

    fn build(self: &Rc<Self>, win: &Rc<Studio>) {
        let hw: Weak<OtherPage> = Rc::downgrade(self);
        let host: Weak<dyn Host> = hw.clone();
        for (k, b) in self.nav.borrow().iter() {
            let k = k.clone();
            let w = Rc::downgrade(self);
            b.connect_clicked(move |_| {
                if let Some(o) = w.upgrade() {
                    o.show_section(&k);
                }
            });
        }
        self.click(&self.btn_scan, |o| {
            o.scan(false);
        });
        self.click(&self.btn_all, |o| {
            let errs = o.errors.borrow().clone();
            o.fix(errs);
        });
        self.click(&self.btn_reload, |o| o.reload());
        for (k, b) in &self.tool_btns {
            match k.as_str() {
                "reload" => self.click(b, |o| o.reload()),
                "waybar" => self.click(b, |o| {
                    let win = o.win();
                    model::restart_waybar(&|s| win.log(s));
                    win.set_status(&t("done"));
                }),
                "open_hypr" => self.click(b, |o| o.open_dir("hypr")),
                "open_waybar" => self.click(b, |o| o.open_dir("waybar")),
                "open_launcher" => self.click(b, |o| {
                    let app = o.win().lp().map(|lp| lp.state.borrow().s("app").to_string()).unwrap_or_default();
                    o.open_dir(&app);
                }),
                "open_kitty" => self.click(b, |o| o.open_dir("kitty")),
                _ => self.click(b, |o| o.show_section("backups")),
            }
        }
        self.click(&self.btn_bak_refresh, |o| o.load_backups());
        self.click(&self.btn_inst, |o| o.install_selected());
        self.click(&self.btn_rm, |o| o.remove_selected());
        for (key, r) in &self.pkg_rows {
            let g = self.on(|o| o.inst_count());
            r.cb.connect_toggled(move |_| g());
            let k = key.clone();
            self.click(&r.act, move |o| o.row_action(&k));
        }
        let page_box = |name: &str| -> gtk::Box {
            let (sa, inner) = page();
            self.stack.add_named(&sa, Some(name));
            inner
        };

        // --- Hell/Dunkel für alle Programme ---
        let ll = page_box("look");
        let c = Card::new("look.title", host.clone());
        let w = Rc::downgrade(self);
        self.look_seg.connect(move |m| {
            if let Some(o) = w.upgrade() {
                o.set_look(&m);
            }
        });
        c.append(&self.look_seg.root);
        c.append(&self.look_state);
        // Qt-/KDE-Apps (Kate, KWrite, Dolphin …) brauchen unter Hyprland das KDE-Plattform-Theme
        let qrow = hbox(6);
        qrow.append(&self.look_qt);
        qrow.append(&self.look_qt_btn);
        c.append(&qrow);
        self.click(&self.look_qt_btn, |o| o.look_install());
        ll.append(&c.root);
        *self.look_card.borrow_mut() = Some(c);

        // --- Leistung ---
        let pfl = page_box("perf");
        let c = Card::new("perf.title", host.clone());
        let head = hbox(6);
        self.pf_state.set_hexpand(true);
        self.pf_state.set_xalign(0.0);
        head.append(&self.pf_state);
        head.append(&self.pf_check);
        head.append(&self.pf_all);
        insert_after(&c, &head, &c.title);
        self.click(&self.pf_check, |o| o.refresh_perf(true));
        self.click(&self.pf_all, |o| o.perf_fix_all());
        for (key, st, b) in &self.pf_rows {
            let row = hbox(0);
            row.set_size_request(620, -1);
            b.set_size_request(170, -1);
            row.append(st);
            row.append(&spacer());
            row.append(b);
            let k = key.clone();
            self.click(b, move |o| o.perf_fix(&k));
            c.add_row(&format!("perfl.{key}"), row.upcast_ref(), None, Some(&format!("tip.perf.{key}")), None,
                Some(&format!("perfl.{key}")));
        }
        pfl.append(&c.root);
        *self.pf_card.borrow_mut() = Some(c);
        let c = Card::new("perf.crashes", host.clone());
        self.pf_crash.set_wrap(true);
        self.pf_crash.set_xalign(0.0);
        c.append(&self.pf_crash);
        pfl.append(&c.root);
        *self.pf_crash_card.borrow_mut() = Some(c);

        // --- Updates ---
        let upl = page_box("updates");
        let c = Card::new("upd.title", host.clone());
        let head = hbox(6);
        self.up_state.set_hexpand(true);
        self.up_state.set_xalign(0.0);
        head.append(&self.up_state);
        head.append(&self.up_check);
        head.append(&self.up_all);
        c.append(&head);
        self.click(&self.up_check, |o| o.refresh_updates(true));
        self.click(&self.up_all, |o| o.run_update(None));
        for (group, ver, b) in &self.up_rows {
            let row = hbox(0);
            row.set_size_request(520, -1); // Knöpfe untereinander in einer Spalte
            ver.set_hexpand(true);
            ver.set_xalign(0.0);
            row.append(ver);
            row.append(b);
            let g = group.clone();
            self.click(b, move |o| o.run_update(Some(&g)));
            c.add_row(&format!("upd.{group}"), row.upcast_ref(), None, None, None, Some(&format!("upd.{group}")));
        }
        upl.append(&c.root);
        *self.up_card.borrow_mut() = Some(c);

        // --- Polkit-Agent ---
        let pkl = page_box("polkit");
        let c = Card::new("polkit.title", host.clone());
        let prow = hbox(10);
        prow.append(&self.pk_chip);
        prow.append(&self.pk_run);
        prow.append(&spacer());
        prow.append(&self.pk_install);
        prow.append(&self.pk_start);
        c.append(&prow);
        self.click(&self.pk_install, |o| o.install_keys(&["hyprpolkitagent".to_string()], false));
        self.click(&self.pk_start, |o| o.start_polkit());
        let w = Rc::downgrade(self);
        self.pk_sw.connect(move |on| {
            if let Some(o) = w.upgrade() {
                o.set_polkit(on);
            }
        });
        c.add_row("polkit.auto", self.pk_sw.area.upcast_ref(), None, None, None, Some("polkit.auto"));
        self.pk_hint.set_wrap(true);
        self.pk_hint.set_xalign(0.0);
        c.append(&self.pk_hint);
        pkl.append(&c.root);
        *self.pk_card.borrow_mut() = Some(c);

        // --- Laptop-Tasten ---
        let lpl = page_box("laptop");
        let c = Card::new("laptop.title", host.clone());
        self.lt_dev.set_xalign(0.0);
        insert_after(&c, &self.lt_dev, &c.title); // Gerät ganz oben, unter dem Kartentitel
        let trow = hbox(8);
        for (_, l) in &self.lt_chips {
            trow.append(l);
        }
        trow.append(&spacer());
        trow.append(&self.lt_install);
        insert_after(&c, &trow, &self.lt_dev);
        self.click(&self.lt_install, |o| o.install_keys(&["media".to_string()], false));
        let w = Rc::downgrade(self);
        self.lt_sw.connect(move |on| {
            if let Some(o) = w.upgrade() {
                o.set_laptop(on);
            }
        });
        c.add_row("laptop.keys", self.lt_sw.area.upcast_ref(), None, Some("tip.laptop.keys"), None,
            Some("laptop.keys"));
        // Sofort ausprobieren (zeigt Fehler wie „keine Geräte“ im Protokoll)
        let test = hbox(8);
        for ((b, _), cmd) in self.lt_tests.iter().zip(["bd", "bu", "vd", "vu", "m"]) {
            test.append(b);
            self.click(b, move |o| o.test_key(cmd));
        }
        c.add_row("laptop.test", test.upcast_ref(), None, None, None, Some("laptop.test"));
        lpl.append(&c.root);
        *self.lt_card.borrow_mut() = Some(c);

        // --- Scrollrichtung (schreibt über Hyprland und wendet sofort an) ---
        let scl = page_box("scroll");
        let c = Card::new("scroll.title", host.clone());
        for (key, sw) in &self.scroll_sw {
            let w = Rc::downgrade(self);
            let k = key.clone();
            sw.connect(move |on| {
                if let Some(o) = w.upgrade() {
                    o.set_scroll(&k, on);
                }
            });
            c.add_row(&format!("scroll.{key}"), sw.area.upcast_ref(), None, Some("tip.scroll"), None,
                Some(&format!("scroll.{key}")));
        }
        // Mausbeschleunigung: aus = accel_profile flat, an = Standard (adaptiv)
        let w = Rc::downgrade(self);
        self.accel_sw.connect(move |on| {
            if let Some(o) = w.upgrade() {
                o.set_accel(on);
            }
        });
        c.add_row("scroll.accel", self.accel_sw.area.upcast_ref(), None, Some("tip.accel"), None, Some("scroll.accel"));
        c.slider("sensitivity", -1.0, 1.0, 0.05, "{:+.2f}", Some("tip.sens"), Some("scroll.sens"));
        c.slider("scroll_factor", 0.1, 3.0, 0.1, "{:.1f}×", Some("tip.scroll_factor"), Some("scroll.speed"));
        scl.append(&c.root);
        *self.scroll_card.borrow_mut() = Some(c);

        // Kitty hier; Hintergrundbild, Mauszeiger und Sperre hängt das Hauptfenster in den Hyprland-Tab
        let tw: Rc<dyn TermWin> = win.clone();
        let kitty = KittyEditor::new(tw, kitty_program());
        let gates = vec![
            ("kitty".to_string(), Gate::new(kitty.clone(), "kitty", win)),
            ("wallpaper".to_string(), Gate::new(self.paper.clone(), "hyprpaper", win)),
            ("cursor".to_string(), Gate::new(self.cursor.clone(), "hyprcursor", win)),
            ("lock".to_string(), Gate::new(self.lock.clone(), "hyprlock", win)),
        ];
        self.stack.add_named(&gates[0].1.stack, Some("kitty"));
        *self.kitty.borrow_mut() = Some(kitty);
        *self.gates.borrow_mut() = gates;
        let sec = win.cfg.borrow().s("other_section").to_string();
        self.show_section(if sec.is_empty() { "fix" } else { &sec });
    }

    /// Gate eines eingebetteten Editors (für den Hyprland-Tab).
    pub fn gate(&self, key: &str) -> Option<Rc<Gate>> {
        self.gates.borrow().iter().find(|(k, _)| k == key).map(|x| x.1.clone())
    }

    pub fn refresh_gates(&self) {
        let gates = self.gates.borrow().clone();
        for (_, g) in gates {
            g.refresh();
        }
    }

    pub fn show_section(self: &Rc<Self>, key: &str) {
        let key = if self.sections.borrow().iter().any(|s| s == key) { key.to_string() } else { "fix".to_string() };
        if let Some((_, b)) = self.nav.borrow().iter().find(|(k, _)| *k == key) {
            b.set_active(true);
        }
        self.stack.set_visible_child_name(&key);
        let win = self.win();
        win.cfg.borrow_mut().insert("other_section".into(), json!(key));
        win.save_later();
        self.title.set_text(&t(&format!("sec.{key}")));
        self.chip.set_visible(key == "fix");
        if let Some(g) = self.gate(&key) {
            g.refresh();
        }
        match key.as_str() {
            "look" => self.refresh_look(),
            "scroll" => self.refresh_scroll(),
            "polkit" => self.refresh_polkit(),
            "updates" => self.refresh_updates(false),
            "perf" => self.refresh_perf(false),
            "laptop" => self.refresh_laptop(),
            "backups" => self.load_backups(),
            "install" => self.refresh_packages(),
            "fix" if self.root.is_mapped() => {
                self.scan(true);
            }
            "advanced" => {
                if let Some(f) = self.adv_retr.borrow().as_ref() {
                    f();
                }
            }
            _ => {}
        }
    }

    // ----- Hell/Dunkel -----

    pub fn refresh_look(&self) {
        let win = self.win();
        let mode = {
            let c = win.cfg.borrow();
            if c.s("appearance").is_empty() { appearance::DEFAULT.to_string() } else { c.s("appearance").to_string() }
        };
        if let Some(c) = self.look_card.borrow().as_ref() {
            c.retranslate();
        }
        self.look_seg.retranslate();
        self.look_seg.select(&mode);
        self.look_state.set_text(&tf("look.active", kw!("name" => t(&format!("look.{mode}")))));
        let ok = packages::installed("qtdark");
        let portal = appearance::gtk_portal();
        if !portal {
            // ohne GTK-Portal bleiben Firefox, Chromium, Electron und GTK4-Apps hell
            self.look_qt.set_text(&t("look.portal_missing"));
            self.look_qt_btn.set_label(&t("look.portal_install"));
        } else {
            self.look_qt.set_text(&if ok { t("look.qt_ok") } else { t("look.qt_missing") });
            self.look_qt_btn.set_label(&t("look.qt_install"));
        }
        self.look_qt_btn.set_visible(!(ok && portal));
        if let Some(hp) = win.hp() {
            let qt = appearance::qt_platform_theme();
            if ok && hp.state.borrow().s("qt_theme") != qt {
                // nur den Hyprland-Zustand nachziehen (wirkt mit Anwenden) – Anzeigen darf nie das System ändern
                hp.state.borrow_mut().insert("qt_theme".into(), json!(qt));
                hp.changed();
            }
        }
    }

    fn look_install(self: &Rc<Self>) {
        if !appearance::gtk_portal() {
            self.run_root(perf::portal_command(packages::manager()), vec![], false);
        } else {
            self.install_keys(&["qtdark".to_string()], false);
        }
    }

    fn set_look(self: &Rc<Self>, mode: &str) {
        let win = self.win();
        win.log("");
        appearance::apply(mode, &model::config_home(), &|s| win.log(s));
        win.cfg.borrow_mut().insert("appearance".into(), json!(mode));
        win.save_later();
        // Hyprland sagt jeder neu gestarteten Qt-App, dass sie dem KDE-Farbschema folgen soll
        let Some(hp) = win.hp() else { return };
        let qt = appearance::qt_platform_theme();
        if hp.state.borrow().s("qt_theme") != qt {
            hp.state.borrow_mut().insert("qt_theme".into(), json!(qt));
            hp.changed();
            if !self.apply_hypr_if_managed() {
                win.log(&t("look.qt_hint"));
            }
        }
    }

    /// Hyprland nur anwenden, wenn es schon von Hypr Studio gestaltet wird (sonst beim nächsten Anwenden).
    fn apply_hypr_if_managed(&self) -> bool {
        let hdir = model::config_home().join("hypr");
        if let Some(fmt) = model::hypr_format(&hdir) {
            if hdir.join(format!("hyprstudio.{fmt}")).is_file() {
                self.apply_hypr();
                return true;
            }
        }
        false
    }

    // ----- Leistung -----

    fn perf_state(&self) -> Obj {
        perf::perf_state(self.win().cfg.borrow().obj("perf"))
    }

    /// Prüfen im Hintergrund (coredumpctl, powerprofilesctl) – die Seite bleibt bedienbar.
    pub fn refresh_perf(self: &Rc<Self>, force: bool) {
        for c in [&self.pf_card, &self.pf_crash_card] {
            if let Some(c) = c.borrow().as_ref() {
                c.retranslate();
            }
        }
        self.pf_check.set_label(&t("perf.check"));
        self.pf_all.set_label(&t("perf.all"));
        self.pf_all.set_tooltip_text(Some(&t("tip.perf.all")));
        if self.pf_job.get() {
            return;
        }
        if self.pf_info.borrow().is_some() && !force {
            self.show_perf();
            return;
        }
        self.pf_state.set_text(&t("perf.checking"));
        let hdir = model::config_home().join("hypr");
        let hs = self.win().hp().map(|hp| hp.state.borrow().clone()).unwrap_or_default();
        let ps = self.perf_state();
        self.pf_job.set(true);
        let w = Rc::downgrade(self);
        shell::run_bg(move || perf::check(&hdir, &hs, &ps), move |r| {
            let Some(o) = w.upgrade() else { return };
            o.pf_job.set(false);
            *o.pf_info.borrow_mut() = Some(r);
            o.show_perf();
        });
    }

    fn show_perf(&self) {
        let Some(r) = self.pf_info.borrow().clone() else { return };
        let g = |a: &str| r.obj(a).cloned().unwrap_or_default();
        let (portal, swap, power, effects, disk) = (g("portal"), g("swap"), g("power"), g("effects"), g("disk"));
        let sz = |o: &Obj, k: &str| perf::size(o.val(k).as_u64().unwrap_or(0));
        let missing = portal.strs("missing");
        let text = |key: &str| -> String {
            match key {
                "portal" => {
                    if portal.b("ok") {
                        t("perf.portal_ok")
                    } else if !missing.is_empty() {
                        tf("perf.portal_missing", kw!("names" => missing.join(", ")))
                    } else {
                        t("perf.portal_env")
                    }
                }
                "swap" => {
                    if swap.b("ok") {
                        tf("perf.swap_ok", kw!("ram" => sz(&swap, "ram"), "swap" => sz(&swap, "swap")))
                    } else {
                        tf("perf.swap_none", kw!("ram" => sz(&swap, "ram")))
                    }
                }
                "power" => {
                    let p = power.s("profile");
                    tf("perf.power", kw!("p" => if p.is_empty() { t("perf.power_none") } else { p.to_string() }))
                }
                "effects" => {
                    if effects.b("ok") {
                        t("perf.effects_ok")
                    } else {
                        tf("perf.effects", kw!("n" => crate::jsonx::py_str(effects.val("passes"))))
                    }
                }
                _ => tf("perf.disk", kw!("free" => sz(&disk, "free"), "total" => sz(&disk, "total"))),
            }
        };
        let light = self.perf_state().b("light");
        let mut bad = 0;
        for (key, st, b) in &self.pf_rows {
            let ok = r.obj(key).map(|o| o.b("ok")).unwrap_or(false);
            if !ok {
                bad += 1;
            }
            set_chip(st, ok, &text(key));
            b.set_label(&if key != "effects" || !light { t(&format!("perf.fix_{key}")) } else { t("perf.effects_off") });
            b.set_visible(key != "disk" && (!ok || (key == "effects" && light)));
        }
        self.pf_state.set_text(&if bad > 0 { tf("perf.bad", kw!("n" => bad)) } else { t("perf.good") });
        let mut lines: Vec<String> = r.arr("crashes").iter().take(8).filter_map(|c| {
            let a = c.as_array()?;
            Some(format!("{}  ×{}", a.first()?.as_str()?, a.get(1)?))
        }).collect();
        let hc = r.i("hypr_crashes");
        if hc != 0 {
            lines.insert(0, tf("perf.hypr_crashes", kw!("n" => hc)));
        }
        self.pf_crash.set_text(&if lines.is_empty() { t("perf.no_crashes") } else { lines.join("\n") });
    }

    fn perf_write(&self, ps: &Obj) {
        let win = self.win();
        win.cfg.borrow_mut().insert("perf".into(), Value::Object(ps.clone()));
        win.save_later();
        if perf::write(&model::config_home().join("hypr"), ps, &|s| win.log(s)) {
            model::run(&["hyprctl", "reload"], &|s| win.log(s), true);
        }
    }

    fn after_perf(self: &Rc<Self>) {
        self.win().set_status(&t("perf.done"));
        *self.pf_info.borrow_mut() = None;
        let w = Rc::downgrade(self);
        shell::later(1500, move || {
            if let Some(o) = w.upgrade() {
                o.refresh_perf(true);
            }
        });
    }

    fn perf_fix(self: &Rc<Self>, key: &str) {
        let win = self.win();
        win.log("");
        let pm = packages::manager();
        let mut ps = self.perf_state();
        let r = self.pf_info.borrow().clone().unwrap_or_default();
        match key {
            "portal" => {
                ps.insert("portal_env".into(), json!(true));
                self.perf_write(&ps);
                if r.obj("portal").map(|p| !p.arr("missing").is_empty()).unwrap_or(false) {
                    self.run_root(perf::portal_command(pm), vec![], false);
                }
            }
            "swap" => {
                let cmd = perf::zram_command(pm);
                if !cmd.is_empty() {
                    self.run_root(cmd, vec![], false);
                }
            }
            "power" => {
                let cmd = perf::power_command("performance");
                if !cmd.is_empty() {
                    let c: Vec<&str> = cmd.iter().map(|s| s.as_str()).collect();
                    model::run(&c, &|s| win.log(s), false);
                }
            }
            "effects" => {
                let light = ps.b("light");
                ps.insert("light".into(), json!(!light));
                self.perf_write(&ps);
            }
            _ => {}
        }
        self.after_perf();
    }

    /// Alles auf einmal – nur eine Passwortabfrage für Portal-Pakete und zram.
    fn perf_fix_all(self: &Rc<Self>) {
        let win = self.win();
        win.log("");
        let pm = packages::manager();
        let r = self.pf_info.borrow().clone().unwrap_or_default();
        let mut ps = self.perf_state();
        ps.insert("portal_env".into(), json!(true));
        ps.insert("light".into(), json!(true));
        self.perf_write(&ps);
        let mut root = vec![];
        if r.obj("portal").map(|p| !p.arr("missing").is_empty()).unwrap_or(false) {
            root.push(perf::portal_command(pm).join(" "));
        }
        if !r.obj("swap").and_then(|p| p.get("ok")).and_then(|v| v.as_bool()).unwrap_or(true) {
            let z = perf::zram_command(pm);
            if let Some(c) = z.get(2) {
                root.push(c.clone());
            }
        }
        let power = r.obj("power").cloned().unwrap_or_default();
        let pok = power.get("ok").and_then(|v| v.as_bool()).unwrap_or(true);
        let ac = power.get("ac").and_then(|v| v.as_bool()).unwrap_or(true);
        if !pok && ac {
            let cmd = perf::power_command("performance");
            if !cmd.is_empty() {
                let c: Vec<&str> = cmd.iter().map(|s| s.as_str()).collect();
                model::run(&c, &|s| win.log(s), false);
            }
        }
        if !root.is_empty() {
            let joined = root.iter().map(|c| format!("( {c} )")).collect::<Vec<_>>().join(" && ");
            self.run_root(vec!["sh".into(), "-c".into(), joined], vec![], false);
        }
        self.after_perf();
    }

    // ----- Updates -----

    /// Versionen im Hintergrund holen (checkupdates kann dauern) – die Seite bleibt bedienbar.
    pub fn refresh_updates(self: &Rc<Self>, force: bool) {
        if let Some(c) = self.up_card.borrow().as_ref() {
            c.retranslate();
        }
        self.up_check.set_label(&t("upd.check"));
        self.up_all.set_label(&t("upd.all"));
        self.up_all.set_tooltip_text(Some(&t("tip.upd.all")));
        if self.up_job.get() {
            return;
        }
        if self.up_info.borrow().is_some() && !force {
            self.show_updates();
            return;
        }
        self.up_state.set_text(&t("upd.checking"));
        let pm = packages::manager().map(|s| s.to_string());
        let names: Vec<String> = packages::UPDATE_GROUPS.iter().flat_map(|(_, ps)| ps.iter().map(|p| p.to_string()))
            .collect();
        self.up_job.set(true);
        let w = Rc::downgrade(self);
        shell::run_bg(move || {
            let pm = pm.as_deref();
            (packages::versions(&names, pm), packages::newer(pm))
        }, move |(have, new)| {
            let Some(o) = w.upgrade() else { return };
            o.up_job.set(false);
            let have: HashMap<String, String> = have.into_iter().collect();
            let new = new.map(|n| n.into_iter().collect::<HashMap<String, String>>());
            *o.up_info.borrow_mut() = Some((have, new));
            o.show_updates();
        });
    }

    fn show_updates(&self) {
        let Some((have, new)) = self.up_info.borrow().clone() else { return };
        let pm = packages::manager();
        let mut pending = 0;
        for (group, ver, b) in &self.up_rows {
            let all = packages::UPDATE_GROUPS.iter().find(|(g, _)| g == group).map(|x| x.1).unwrap_or(&[]);
            let pkgs: Vec<&str> = all.iter().copied().filter(|p| have.contains_key(*p)).collect();
            b.set_label(&t("upd.one"));
            b.set_tooltip_text(if pm == Some("pacman") { Some(t("tip.upd.arch")) } else { None }.as_deref());
            if pkgs.is_empty() {
                ver.set_text(&t("upd.none"));
                b.set_sensitive(false);
                continue;
            }
            let main = pkgs[0];
            let ups: Vec<&str> = pkgs.iter().copied().filter(|p| new.as_ref().map(|n| n.contains_key(*p))
                .unwrap_or(false)).collect();
            pending += ups.len();
            match &new {
                None => ver.set_text(&tf("upd.ver", kw!("ver" => have[main]))),
                Some(n) if !ups.is_empty() => {
                    let nv = n.get(main).or_else(|| n.get(ups[0])).cloned().unwrap_or_default();
                    ver.set_text(&tf(if ups.len() == 1 { "upd.newer1" } else { "upd.newer" },
                        kw!("ver" => have[main], "new" => nv, "n" => ups.len())));
                }
                Some(_) => ver.set_text(&tf("upd.current", kw!("ver" => have[main]))),
            }
            b.set_sensitive(!ups.is_empty() || new.is_none());
        }
        if new.is_none() {
            self.up_state.set_text(&t("upd.unknown"));
        } else {
            self.up_state.set_text(&if pending > 0 { tf("upd.pending", kw!("n" => pending)) } else { t("upd.uptodate") });
        }
    }

    fn run_update(self: &Rc<Self>, group: Option<&str>) {
        let pm = packages::manager();
        let have = self.up_info.borrow().as_ref().map(|x| x.0.clone()).unwrap_or_default();
        let pkgs: Vec<String> = match group {
            Some(g) => packages::UPDATE_GROUPS.iter().find(|(x, _)| *x == g).map(|x| x.1).unwrap_or(&[]).iter()
                .filter(|p| have.contains_key(**p)).map(|p| p.to_string()).collect(),
            None => vec![],
        };
        let cmd = packages::update_command(&pkgs, pm, group.is_none());
        if cmd.is_empty() {
            self.win().set_status(&t("pkg.nopm"));
            return;
        }
        *self.up_info.borrow_mut() = None; // nach dem Update neu prüfen
        self.run_root(cmd, vec![], false);
    }

    // ----- Polkit-Agent -----

    pub fn refresh_polkit(&self) {
        if let Some(c) = self.pk_card.borrow().as_ref() {
            c.retranslate();
        }
        let Some(hp) = self.win().hp() else { return };
        let inst = packages::installed("hyprpolkitagent");
        let avail = packages::available("hyprpolkitagent", packages::manager());
        set_chip(&self.pk_chip, inst, &if inst {
            t("pkg.installed")
        } else if avail {
            t("pkg.missing")
        } else {
            t("pkg.unavailable")
        });
        let running = inst && packages::polkit_agent();
        set_chip(&self.pk_run, running, &if running { t("polkit.running") } else { t("polkit.stopped") });
        self.pk_run.set_visible(inst);
        self.pk_install.set_label(&tf("gate.install", kw!("name" => "hyprpolkitagent")));
        self.pk_install.set_visible(!inst);
        self.pk_install.set_sensitive(avail);
        self.pk_start.set_label(&t("polkit.start"));
        self.pk_start.set_sensitive(inst);
        self.pk_hint.set_text(&t("polkit.hint"));
        // noch nie selbst geschaltet: an, sobald installiert oder wenn die Config den Agent schon startet
        let managed = hp.state.borrow().b("polkit_managed");
        if !managed && (inst || model::detect_polkit(&model::config_home().join("hypr"), &hp.state.borrow())) {
            self.set_polkit(true);
        }
        self.pk_sw.set_now(hp.state.borrow().b("polkit_agent"));
    }

    fn set_polkit(&self, on: bool) {
        let win = self.win();
        let Some(hp) = win.hp() else { return };
        {
            let s = hp.state.borrow();
            if s.b("polkit_managed") && s.b("polkit_agent") == on {
                return;
            }
        }
        {
            let mut s = hp.state.borrow_mut();
            s.insert("polkit_agent".into(), json!(on));
            s.insert("polkit_managed".into(), json!(true));
            let cmds: Vec<String> = s.strs("autostart").into_iter().filter(|c| !model::is_polkit_cmd(c)).collect();
            s.insert("autostart".into(), json!(cmds));
        }
        hp.refresh_controls();
        hp.changed();
        win.log("");
        win.log(&if on { t("polkit.on") } else { t("polkit.off") });
        if !self.apply_hypr_if_managed() {
            win.log(&t("polkit.later"));
        }
    }

    fn start_polkit(self: &Rc<Self>) {
        let win = self.win();
        win.log("");
        let ok = model::run(&["systemctl", "--user", "start", "hyprpolkitagent"], &|s| win.log(s), false);
        win.set_status(&if ok { t("polkit.started") } else { t("polkit.start_fail") });
        let w = Rc::downgrade(self);
        shell::later(800, move || {
            if let Some(o) = w.upgrade() {
                o.refresh_polkit();
            }
        });
        self.refresh_look();
    }

    // ----- Laptop-Tasten -----

    pub fn refresh_laptop(&self) {
        if let Some(c) = self.lt_card.borrow().as_ref() {
            c.retranslate();
        }
        let win = self.win();
        let Some(hp) = win.hp() else { return };
        let dev = hardware::detect();
        let (bright, vol, player) = hardware::tools();
        self.lt_dev.set_text(&hardware::label(&dev));
        for (key, l) in &self.lt_chips {
            match key.as_str() {
                "laptop" => set_chip(l, dev.laptop, &if dev.laptop { t("laptop.is_laptop") } else { t("laptop.desktop") }),
                "brightnessctl" => set_chip(l, bright, "brightnessctl"),
                "volume" => set_chip(l, vol, hardware::volume_tool()),
                _ => set_chip(l, player, "playerctl"),
            }
        }
        let missing = !(bright && player);
        self.lt_install.set_label(&tf("gate.install", kw!("name" => "brightnessctl + playerctl")));
        self.lt_install.set_visible(missing);
        self.lt_install.set_sensitive(packages::available("media", packages::manager()));
        for (b, key) in &self.lt_tests {
            b.set_label(&t(&format!("laptop.{key}")));
        }
        // auf Laptops schon beim Start an
        let changed = laptopkeys::auto(&mut hp.state.borrow_mut(), &|s| win.log(s));
        if changed {
            hp.changed();
        }
        self.lt_sw.set_now(hp.state.borrow().b("laptop_keys"));
    }

    fn set_laptop(&self, on: bool) {
        let win = self.win();
        let Some(hp) = win.hp() else { return };
        {
            let s = hp.state.borrow();
            if s.b("laptop_managed") && s.b("laptop_keys") == on {
                return;
            }
        }
        {
            let mut s = hp.state.borrow_mut();
            s.insert("laptop_keys".into(), json!(on));
            s.insert("laptop_managed".into(), json!(true));
            s.insert("vol_tool".into(), json!(hardware::volume_tool()));
            if on {
                // gleiche Tasten aus der eigenen Kürzel-Liste nehmen, sonst doppelt
                let binds: Vec<Value> = s.arr("binds").iter()
                    .filter(|b| !b.as_object().map(model::is_laptop_bind).unwrap_or(false)).cloned().collect();
                s.insert("binds".into(), Value::Array(binds));
            }
        }
        hp.refresh_controls();
        hp.changed();
        win.log("");
        win.log(&if on { t("laptop.on") } else { t("laptop.off") });
        let st = hp.state.borrow().clone();
        if laptopkeys::write(&model::config_home().join("hypr"), &st, &|s| win.log(s)) {
            model::run(&["hyprctl", "reload"], &|s| win.log(s), true);
        }
        self.apply_hypr_if_managed(); // eigene Kürzel ohne die Laptop-Tasten neu schreiben
    }

    fn test_key(&self, which: &str) {
        let [up, down, mute, _] = model::volume_cmds(hardware::volume_tool());
        let cmd = match which {
            "bu" => model::BRIGHT_UP,
            "bd" => model::BRIGHT_DOWN,
            "vu" => up,
            "vd" => down,
            _ => mute,
        };
        let win = self.win();
        win.log("");
        let ok = model::run(&["sh", "-c", cmd], &|s| win.log(s), false);
        win.set_status(&if ok { t("laptop.test_ok") } else { t("laptop.test_fail") });
    }

    // ----- Scrollrichtung -----

    /// Regler unter Maus & Touchpad: Wert merken, kurz nach dem Loslassen anwenden.
    fn set(self: &Rc<Self>, key: &str, value: Value) {
        let Some(hp) = self.win().hp() else { return };
        if hp.state.borrow().get(key).map(|v| super::editor::py_eq(v, &value)).unwrap_or(false) {
            return;
        }
        hp.state.borrow_mut().insert(key.into(), value);
        hp.refresh_controls();
        hp.changed();
        if let Some(id) = self.apply_later.borrow_mut().take() {
            id.remove();
        }
        let w = Rc::downgrade(self);
        let id = glib::timeout_add_local_once(std::time::Duration::from_millis(600), move || {
            if let Some(o) = w.upgrade() {
                o.apply_later.borrow_mut().take();
                o.apply_hypr();
            }
        });
        *self.apply_later.borrow_mut() = Some(id);
    }

    fn apply_hypr(&self) {
        let win = self.win();
        win.log("");
        if let Some(hp) = win.hp() {
            hp.apply();
        }
    }

    pub fn refresh_scroll(&self) {
        let win = self.win();
        let Some(hp) = win.hp() else { return };
        let hs = hp.state.borrow().clone();
        if let Some(c) = self.scroll_card.borrow().as_ref() {
            c.retranslate();
            c.refresh(&hs);
        }
        for (k, sw) in &self.scroll_sw {
            sw.set_now(crate::jsonx::truthy(hs.val(k)));
        }
        self.accel_sw.set_now(hs.s("accel") != "flat");
    }

    fn set_accel(&self, on: bool) {
        let win = self.win();
        let Some(hp) = win.hp() else { return };
        let want = if on { "default" } else { "flat" };
        let cur = hp.state.borrow().s("accel").to_string();
        if cur == want || (on && cur != "flat") {
            return;
        }
        hp.state.borrow_mut().insert("accel".into(), json!(want));
        hp.refresh_controls();
        hp.changed();
        win.log("");
        hp.apply();
    }

    fn set_scroll(&self, key: &str, on: bool) {
        let win = self.win();
        let Some(hp) = win.hp() else { return };
        if crate::jsonx::truthy(hp.state.borrow().val(key)) == on && hp.state.borrow().get(key).is_some() {
            return;
        }
        hp.state.borrow_mut().insert(key.into(), json!(on));
        hp.refresh_controls();
        hp.changed();
        win.log("");
        hp.apply();
    }

    // ----- Reparatur -----

    fn hdir(&self) -> PathBuf {
        model::config_home().join("hypr")
    }

    pub fn scan(self: &Rc<Self>, quiet: bool) -> usize {
        let win = self.win();
        let raw = fixer::read_errors(&self.hdir(), &|s| win.log(s));
        let errors = fixer::parse(&raw);
        super::widgets::clear_box(&self.err_box);
        for e in &errors {
            self.err_box.append(&self.error_row(e));
        }
        let n = errors.len();
        *self.errors.borrow_mut() = errors;
        let txt = if n == 0 { t("fix.none") } else { tf("fix.count", kw!("n" => n)) };
        set_chip(&self.chip, n == 0, &txt);
        self.btn_all.set_sensitive(n > 0);
        if !quiet {
            win.set_status(&txt);
        }
        n
    }

    fn error_row(self: &Rc<Self>, e: &fixer::ErrorEntry) -> gtk::Box {
        let row = bind_row(14, 10, 10, 10, 12);
        let col = vbox(4);
        col.set_hexpand(true);
        let where_ = if e.file.is_empty() {
            t("fix.nofile")
        } else {
            format!("{}:{}", Path::new(&e.file).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                e.line)
        };
        let wl = label(&where_, "Mono");
        wl.set_xalign(0.0);
        wl.set_tooltip_text(Some(&e.file));
        let msg = label(&e.msg, "");
        msg.set_wrap(true);
        msg.set_xalign(0.0);
        col.append(&wl);
        col.append(&msg);
        let src = if e.file.is_empty() { String::new() } else { fixer::line_text(e) };
        if !src.is_empty() {
            let code = label(&src, "Code");
            code.set_selectable(true);
            code.set_xalign(0.0);
            col.append(&code);
        }
        row.append(&col);
        let p = fixer::plan(e);
        let b = btn(&t("fix.one"), "");
        b.set_valign(gtk::Align::Start);
        b.set_sensitive(p.kind() != "none");
        let tip = match &p {
            fixer::Plan::Skip(opt) => tf("fix.tip.skip", kw!("opt" => opt)),
            fixer::Plan::State(_) => t("fix.tip.state"),
            fixer::Plan::Bind(_) => t("fix.tip.bind"),
            fixer::Plan::Comment => t("fix.tip.comment"),
            fixer::Plan::None => t("fix.tip.none"),
        };
        b.set_tooltip_text(Some(&tip));
        let e2 = e.clone();
        self.click(&b, move |o| o.fix(vec![e2.clone()]));
        row.append(&b);
        row
    }

    pub fn fix(self: &Rc<Self>, errors: Vec<fixer::ErrorEntry>) {
        if errors.is_empty() {
            return;
        }
        let win = self.win();
        win.log("");
        let Some(hp) = win.hp() else { return };
        let mut own = false;
        let mut fixed = vec![];
        for e in &errors {
            let kind = fixer::plan(e).kind();
            let ok = fixer::apply_fix(e, &mut hp.state.borrow_mut(), &|s| win.log(s));
            if ok {
                fixed.push(e.clone());
                own = own || ["skip", "state", "bind"].contains(&kind);
            }
        }
        if own {
            // eigene Datei neu schreiben (lädt Hyprland auch neu)
            hp.refresh_controls();
            hp.changed();
            hp.apply();
        } else {
            model::run(&["hyprctl", "reload"], &|s| win.log(s), false);
        }
        if model::dry() {
            // Testmodus: behobene Meldungen aus der Testdatei nehmen
            if let Some(parent) = self.hdir().parent() {
                let f = parent.join(fixer::TESTFILE);
                if f.is_file() {
                    let gone: Vec<&str> = fixed.iter().map(|e| e.raw.as_str()).collect();
                    let text = util::read_text(&f);
                    let keep: Vec<&str> = text.lines().filter(|x| !gone.contains(x)).collect();
                    let _ = std::fs::write(&f, keep.join("\n") + "\n");
                }
            }
        }
        // Hyprland braucht einen Moment für das Neuladen
        let n = fixed.len();
        let w = Rc::downgrade(self);
        shell::later(700, move || {
            if let Some(o) = w.upgrade() {
                let left = o.scan(true);
                o.win().set_status(&tf("fix.done", kw!("n" => n, "left" => left)));
            }
        });
    }

    fn reload(self: &Rc<Self>) {
        let win = self.win();
        win.log("");
        model::run(&["hyprctl", "reload"], &|s| win.log(s), false);
        let w = Rc::downgrade(self);
        shell::later(700, move || {
            if let Some(o) = w.upgrade() {
                o.scan(false);
            }
        });
    }

    // ----- Installieren -----

    pub fn refresh_packages(&self) {
        let pm = packages::manager();
        self.pm_label.set_text(&match pm {
            Some(p) => tf("pkg.pm", kw!("pm" => p)),
            None => t("pkg.nopm"),
        });
        let busy = self.proc.get();
        for (key, r) in &self.pkg_rows {
            let inst = packages::installed(key);
            let avail = packages::available(key, pm);
            r.cb.set_label(Some(&t(&format!("pkg.{key}"))));
            r.desc.set_text(&format!("{}  ·  {}", t(&format!("pkgd.{key}")),
                packages::pkgs_for(key, pm.unwrap_or("pacman")).join(" ")));
            r.cb.set_sensitive(avail && !busy);
            if !avail {
                r.cb.set_active(false);
            }
            r.act.set_label(&if inst { t("pkg.remove_one") } else { t("pkg.install_one") });
            r.act.remove_css_class(if inst { "Primary" } else { "DangerFill" });
            r.act.add_css_class(if inst { "DangerFill" } else { "Primary" });
            r.act.set_sensitive(avail && !busy);
            set_chip(&r.chip, inst, &if inst {
                t("pkg.installed")
            } else if avail {
                t("pkg.missing")
            } else {
                t("pkg.unavailable")
            });
        }
        self.inst_count();
    }

    fn selected(&self) -> Vec<String> {
        self.pkg_rows.iter().filter(|(_, r)| r.cb.is_active()).map(|(k, _)| k.clone()).collect()
    }

    fn inst_count(&self) {
        let sel = self.selected();
        let n = sel.iter().filter(|k| !packages::installed(k)).count();
        let r = sel.iter().filter(|k| packages::installed(k)).count();
        let busy = self.proc.get();
        self.btn_inst.set_label(&if n > 0 { tf("pkg.install", kw!("n" => n)) } else { t("pkg.install0") });
        self.btn_inst.set_sensitive(n > 0 && !busy);
        self.btn_rm.set_label(&if r > 0 { tf("pkg.remove", kw!("n" => r)) } else { t("pkg.remove0") });
        self.btn_rm.set_sensitive(r > 0 && !busy);
    }

    fn install_selected(self: &Rc<Self>) {
        let keys: Vec<String> = self.selected().into_iter().filter(|k| !packages::installed(k)).collect();
        self.install_keys(&keys, false);
    }

    fn remove_selected(self: &Rc<Self>) {
        let keys: Vec<String> = self.selected().into_iter().filter(|k| packages::installed(k)).collect();
        self.remove_keys(keys);
    }

    fn row_action(self: &Rc<Self>, key: &str) {
        if packages::installed(key) {
            self.remove_keys(vec![key.to_string()]);
        } else {
            self.install_keys(&[key.to_string()], false);
        }
    }

    fn remove_keys(self: &Rc<Self>, keys: Vec<String>) {
        let Some(pm) = packages::manager() else { return };
        if keys.is_empty() {
            return;
        }
        let names = keys.iter().map(|k| t(&format!("pkg.{k}"))).collect::<Vec<_>>().join(", ");
        let kr: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();
        let pkgs = packages::pkg_names(&kr, pm).join(" ");
        let w = Rc::downgrade(self);
        dialogs::confirm(shell::toplevel_of(&self.root).as_ref(), &t("pkg.remove_title"),
            &tf("pkg.remove_q", kw!("names" => names, "pkgs" => pkgs)), Some(&t("pkg.remove_one")), true, move |ok| {
                if let (true, Some(o)) = (ok, w.upgrade()) {
                    o.install_keys(&keys, true);
                }
            });
    }

    pub fn install_keys(self: &Rc<Self>, keys: &[String], remove: bool) {
        let Some(pm) = packages::manager() else { return };
        let kr: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();
        let cmd = if remove { packages::remove_command(&kr, pm) } else { packages::command(&kr, pm) };
        if cmd.is_empty() {
            return;
        }
        for (k, r) in &self.pkg_rows {
            if keys.contains(k) {
                r.cb.set_active(false);
            }
        }
        self.run_root(cmd, keys.to_vec(), remove);
    }

    /// Befehl als root: pkexec mit Passwortfenster, ohne Polkit-Agent im Terminal mit sudo.
    pub fn run_root(self: &Rc<Self>, cmd: Vec<String>, keys: Vec<String>, remove: bool) {
        let win = self.win();
        win.log("");
        if model::dry() {
            win.log(&format!("$ pkexec {}  (Testmodus)", cmd.join(" ")));
            {
                let mut ins = packages::DRY_INSTALLED.lock().unwrap();
                let mut rem = packages::DRY_REMOVED.lock().unwrap();
                for k in &keys {
                    if remove {
                        rem.insert(k.clone());
                        ins.remove(k);
                    } else {
                        ins.insert(k.clone());
                        rem.remove(k);
                    }
                }
            }
            self.refresh_packages();
            win.set_status(&t("pkg.done"));
            win.refresh_gates();
            if !win.log_visible() {
                win.show_log();
            }
            return;
        }
        if !util::has("pkexec") || !packages::polkit_agent() {
            // ohne Polkit-Agent scheitert pkexec lautlos → sichtbares Terminal mit sudo-Passwortabfrage
            win.log(&t("pkg.noagent"));
            self.terminal(&cmd);
            self.watch(keys, remove);
            return;
        }
        self.pkg_out.borrow_mut().clear();
        self.proc.set(true);
        win.log(&format!("$ pkexec {}", cmd.join(" ")));
        win.set_status(&t("pkg.busy"));
        win.bar_busy(true);
        let w = Rc::downgrade(self);
        let cmd2 = cmd.clone();
        shell::run_job(move |log| {
            use std::io::BufRead;
            let mut args = vec!["-c".to_string(), "exec pkexec \"$@\" 2>&1".to_string(), "sh".to_string()];
            args.extend(cmd2);
            let child = std::process::Command::new("sh").args(&args).current_dir("/tmp")
                .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::piped()).spawn();
            let mut out = vec![];
            let Ok(mut child) = child else { return (127, out) };
            if let Some(so) = child.stdout.take() {
                for line in std::io::BufReader::new(so).lines().map_while(Result::ok) {
                    if !line.trim().is_empty() {
                        log(&line);
                        out.push(line);
                    }
                }
            }
            let code = child.wait().ok().and_then(|s| s.code()).unwrap_or(-1);
            (code, out)
        }, move |(code, out)| {
            if let Some(o) = w.upgrade() {
                *o.pkg_out.borrow_mut() = out;
                o.installed(code, &cmd, keys, remove);
            }
        });
        self.refresh_packages();
    }

    fn installed(self: &Rc<Self>, code: i32, cmd: &[String], keys: Vec<String>, remove: bool) {
        packages::forget();
        self.proc.set(false);
        let win = self.win();
        win.bar_busy(false);
        if code == 126 || code == 127 {
            // abgebrochen oder kein Polkit-Agent → im Terminal mit sudo
            win.log(&t("pkg.fallback"));
            self.terminal(cmd);
            self.watch(keys, remove);
        } else if code == 0 {
            win.set_status(&t("pkg.done"));
        } else {
            let why = packages::fail_reason(&self.pkg_out.borrow().join("\n"));
            win.set_status(&(t("pkg.failed") + &why.map(|w| format!(" – {}", t(w))).unwrap_or_default()));
            if !win.log_visible() {
                win.show_log();
            }
        }
        self.refresh_packages();
        win.refresh_gates();
    }

    /// Nach Installation im Terminal: alle 2 s nachsehen, bis das Programm da (bzw. weg) ist – max. 10 min.
    fn watch(self: &Rc<Self>, keys: Vec<String>, remove: bool) {
        if keys.is_empty() {
            return;
        }
        if let Some(id) = self.watch.borrow_mut().take() {
            id.remove();
        }
        let left = Rc::new(Cell::new(300));
        let w = Rc::downgrade(self);
        let id = glib::timeout_add_local(std::time::Duration::from_millis(2000), move || {
            let Some(o) = w.upgrade() else { return glib::ControlFlow::Break };
            packages::forget();
            left.set(left.get() - 1);
            let done = keys.iter().all(|k| packages::installed(k) != remove);
            if done || left.get() <= 0 {
                o.watch.borrow_mut().take();
                if done {
                    o.win().set_status(&t("pkg.done"));
                }
                o.refresh_packages();
                o.win().refresh_gates();
                return glib::ControlFlow::Break;
            }
            glib::ControlFlow::Continue
        });
        *self.watch.borrow_mut() = Some(id);
    }

    fn terminal(&self, cmd: &[String]) {
        let line = format!("cd /tmp && sudo {}; echo; read -r -p 'Enter = schließen' _", cmd.join(" "));
        let l = line.as_str();
        let terms: [(&str, Vec<&str>); 7] = [
            ("kitty", vec!["sh", "-c", l]), ("foot", vec!["sh", "-c", l]), ("alacritty", vec!["-e", "sh", "-c", l]),
            ("wezterm", vec!["start", "--", "sh", "-c", l]), ("konsole", vec!["-e", "sh", "-c", l]),
            ("gnome-terminal", vec!["--", "sh", "-c", l]), ("xterm", vec!["-e", "sh", "-c", l])];
        let win = self.win();
        for (term, args) in terms {
            if util::has(term) {
                let mut c = vec![term];
                c.extend(args);
                let _ = util::spawn_detached_in(&c, Some(Path::new("/tmp")));
                win.set_status(&tf("pkg.terminal", kw!("term" => term)));
                return;
            }
        }
        win.set_status(&t("pkg.noterm"));
    }

    // ----- Werkzeuge -----

    fn open_dir(&self, name: &str) {
        online::open_dir(&model::config_home().join(name));
    }

    // ----- Sicherungen -----

    pub fn load_backups(self: &Rc<Self>) {
        super::widgets::clear_box(&self.bak_box);
        let items = fixer::backups(&model::config_home());
        if items.is_empty() {
            let lb = label(&t("bak.none"), "RowLabel");
            lb.set_xalign(0.0);
            self.bak_box.append(&lb);
        }
        for (bak, orig) in items.into_iter().take(80) {
            let row = bind_row(14, 8, 10, 8, 6);
            let parent = bak.parent().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let fname = bak.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let name = label(&format!("{parent}/{fname}"), "Mono");
            name.set_hexpand(true);
            name.set_xalign(0.0);
            name.set_ellipsize(pango::EllipsizeMode::Middle);
            let mtime = std::fs::metadata(&bak).and_then(|m| m.modified()).ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64).unwrap_or(0);
            let when = glib::DateTime::from_unix_local(mtime).ok().and_then(|d| d.format("%d.%m.%Y %H:%M").ok())
                .map(|s| s.to_string()).unwrap_or_default();
            let wl = label(&when, "RowLabel");
            wl.set_margin_end(10);
            let oname = orig.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let b = btn(&t("bak.restore"), "");
            b.set_tooltip_text(Some(&tf("bak.tip", kw!("target" => oname))));
            let (b2, o2) = (bak.clone(), orig.clone());
            self.click(&b, move |o| o.restore(b2.clone(), o2.clone()));
            row.append(&name);
            row.append(&wl);
            row.append(&b);
            self.bak_box.append(&row);
        }
    }

    fn restore(self: &Rc<Self>, bak: PathBuf, orig: PathBuf) {
        let bn = bak.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let on = orig.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let w = Rc::downgrade(self);
        dialogs::confirm(shell::toplevel_of(&self.root).as_ref(), &t("bak.restore"),
            &tf("bak.confirm", kw!("src" => bn, "target" => on)), Some(&t("bak.restore")), false, move |ok| {
                let (true, Some(o)) = (ok, w.upgrade()) else { return };
                let win = o.win();
                win.log("");
                fixer::restore(&bak, &orig, &|s| win.log(s));
                let pdir = orig.parent().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                if pdir == "hypr" {
                    model::run(&["hyprctl", "reload"], &|s| win.log(s), false);
                } else if pdir == "waybar" {
                    model::restart_waybar(&|s| win.log(s));
                }
                let on = orig.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                win.set_status(&tf("bak.done", kw!("target" => on)));
                o.load_backups();
            });
    }

    // ----- Kitty / Linux AIO -----

    pub fn kitty_reset_state(&self) {
        if let Some(k) = self.kitty.borrow().as_ref() {
            k.reset_state();
        }
    }

    /// Linux AIO: Kitty steckt dort unter Erweitert.
    pub fn hide_kitty(&self) {
        if let Some((_, b)) = self.nav.borrow().iter().find(|(k, _)| k == "kitty") {
            b.set_visible(false);
        }
        self.sections.borrow_mut().retain(|k| k != "kitty");
        let win = self.win();
        if win.cfg.borrow().s("other_section") == "kitty" {
            win.cfg.borrow_mut().insert("other_section".into(), json!("fix"));
        }
        for h in &self.heads {
            h.sync();
        }
    }

    /// Linux AIO: Bereich „Erweitert“ in der Seitenleiste (Config-Format + alles zurücksetzen).
    pub fn add_advanced(self: &Rc<Self>) {
        TEXT.update("de", &[("sec.advanced", "Erweitert"), ("adv.reset", "Alles zurücksetzen"),
            ("adv.reset.tip", "Nimmt alle Änderungen von Hypr Studio an Hyprland, Waybar und Launchern zurück")]);
        TEXT.update("en", &[("sec.advanced", "Advanced"), ("adv.reset", "Reset everything"),
            ("adv.reset.tip", "Undoes all Hypr Studio changes to Hyprland, Waybar and launchers")]);
        let pg = vbox(12);
        let row = hbox(0);
        let lbl = label("", "");
        let seg = Segmented::new(&["conf", "lua"], "fmtsw.");
        let ww = self.win.clone();
        seg.connect(move |f| {
            if let Some(w) = ww.upgrade() {
                w.set_format(&f);
            }
        });
        row.append(&lbl);
        row.append(&spacer());
        row.append(&seg.root);
        pg.append(&row);
        let reset = btn("", "DangerFill");
        reset.set_halign(gtk::Align::Start);
        let ww = self.win.clone();
        reset.connect_clicked(move |_| {
            if let Some(w) = ww.upgrade() {
                w.reset_all();
            }
        });
        pg.append(&reset);
        self.stack.add_named(&pg, Some("advanced"));
        self.sections.borrow_mut().push("advanced".into());
        let first = self.nav.borrow().first().map(|x| x.1.clone());
        let b = nav_button(first.as_ref());
        let w = Rc::downgrade(self);
        b.connect_clicked(move |_| {
            if let Some(o) = w.upgrade() {
                o.show_section("advanced");
            }
        });
        self.side.insert_child_after(&b, self.side_end.prev_sibling().as_ref());
        self.nav.borrow_mut().push(("advanced".into(), b.clone()));
        let ww = self.win.clone();
        let retr = move || {
            nav_set_text(&b, &t("sec.advanced"));
            lbl.set_text(&t("adv.format"));
            reset.set_label(&t("adv.reset"));
            reset.set_tooltip_text(Some(&t("adv.reset.tip")));
            seg.retranslate();
            let cur = ww.upgrade().map(|w| w.cfg.borrow().s("hypr_format").to_string()).unwrap_or_default();
            let cur = if !cur.is_empty() {
                cur
            } else {
                model::hypr_format(&model::config_home().join("hypr")).unwrap_or_else(|| "lua".into())
            };
            seg.select(&cur);
        };
        retr();
        *self.adv_retr.borrow_mut() = Some(Box::new(retr));
        let sec = self.win().cfg.borrow().s("other_section").to_string();
        if sec == "advanced" {
            self.show_section("advanced");
        }
    }

    pub fn apply_theme(&self, accent: &str) {
        if let Some(k) = self.kitty.borrow().as_ref() {
            k.apply_theme(accent);
        }
        self.paper.apply_theme(accent);
        self.cursor.apply_theme(accent);
        self.lock.apply_theme(accent);
    }

    pub fn retranslate(self: &Rc<Self>) {
        for (k, b) in self.nav.borrow().iter() {
            if k != "advanced" {
                nav_set_text(b, &t(&format!("sec.{k}")));
            }
        }
        for h in &self.heads {
            h.retranslate();
        }
        let sec = self.win().cfg.borrow().s("other_section").to_string();
        self.title.set_text(&t(&format!("sec.{}", if sec.is_empty() { "fix" } else { &sec })));
        self.btn_scan.set_label(&t("fix.scan"));
        self.btn_all.set_label(&t("fix.all"));
        self.btn_reload.set_label(&t("fix.reload"));
        for (k, b) in &self.tool_btns {
            b.set_label(&t(&format!("tool.{k}")));
        }
        self.btn_bak_refresh.set_label(&t("bak.refresh"));
        if let Some(k) = self.kitty.borrow().as_ref() {
            k.retranslate();
        }
        self.paper.retranslate();
        self.cursor.retranslate();
        self.lock.retranslate();
        self.refresh_gates();
        for (g, h) in &self.pkg_heads {
            h.set_text(&t(&format!("pkgg.{g}")));
        }
        self.refresh_packages();
        if let Some(f) = self.adv_retr.borrow().as_ref() {
            f();
        }
        if !self.chip.text().is_empty() {
            self.scan(true);
        }
    }
}
