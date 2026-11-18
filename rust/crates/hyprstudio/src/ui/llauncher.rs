//! Fenster des L Launchers: Eingabe oben, darunter Treffer als Liste oder Raster. Aussehen aus dem Launcher-Tab.
//! Enter startet den markierten Treffer, Esc oder Klick daneben schließt, zweites Drücken der Taste auch.

use super::draw::Rgba;
use crate::applauncher::{self as al, App};
use crate::jsonx::{Get, Obj};
use crate::{launcher, util};
use gtk::prelude::*;
use gtk::{gdk, glib};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

fn rgba(c: &str) -> String {
    let q = Rgba::hex(c);
    format!("rgba({},{},{},{:.3})", (q.0 * 255.0).round(), (q.1 * 255.0).round(), (q.2 * 255.0).round(), q.3)
}

/// Startet den L Launcher (eigene GTK-Anwendung). Rückgabe = Exit-Code.
pub fn main() -> i32 {
    let s0 = launcher::state(Some(&al::load_json(&al::settings_file())));
    if s0.b("toggle") && al::toggle_running() {
        return 0;
    }
    al::write_pid();
    let app = gtk::Application::builder().application_id("de.labtosostudios.llauncher")
        .flags(gtk::gio::ApplicationFlags::NON_UNIQUE).build();
    let s = Rc::new(s0);
    app.connect_activate(move |app| build(app, &s));
    let code = app.run_with_args::<&str>(&[]);
    al::remove_pid();
    code.into()
}

fn icon_setup(s: &Obj) {
    let Some(display) = gdk::Display::default() else { return };
    let it = gtk::IconTheme::for_display(&display);
    // Symbolthema wie im System (KDE z. B. Sweet), auch aus ~/.local/share/icons und ~/.icons
    let home = util::home();
    for p in [home.join(".local/share/icons"), home.join(".icons")] {
        if p.is_dir() {
            it.add_search_path(&p);
        }
    }
    for d in al::app_dirs() {
        if let Some(parent) = d.parent() {
            let p = parent.join("icons");
            if p.is_dir() {
                it.add_search_path(&p);
            }
        }
    }
    let theme = launcher::icon_theme(s);
    if !theme.is_empty() {
        it.set_theme_name(Some(&theme));
    } else {
        let cur = it.theme_name().to_string();
        if cur.is_empty() || cur == "hicolor" || cur == "Adwaita" {
            for name in ["Papirus-Dark", "Papirus", "breeze-dark", "breeze", "Adwaita"] {
                if al::app_dirs().iter().any(|d| d.parent().map(|p| p.join("icons").join(name).is_dir()).unwrap_or(false)) {
                    it.set_theme_name(Some(name));
                    break;
                }
            }
        }
    }
}

fn build(app: &gtk::Application, s: &Rc<Obj>) {
    icon_setup(s);
    let apps: Rc<Vec<App>> = Rc::new(al::load_apps(None));
    let history = Rc::new(al::load_json(&al::history_file()));
    let fsize = s.n("font_size");
    let isize = s.i("icon_size") as i32;
    let mut row_h = isize.max((fsize * 2.0) as i32) + 12;
    let rows = s.i("lines").max(3) as i32;
    let pad = s.i("padding") as i32;
    let spacing = s.i("spacing") as i32;
    let grid = s.b("grid");
    let cols = if grid || s.i("columns") > 1 { s.i("columns").max(1) as i32 } else { 1 };
    let width = s.i("width") as i32;
    let inner_w = width - 2 * pad - 2 * s.i("border_width") as i32 - 8;
    let cell_w = inner_w / cols;
    if grid {
        row_h = isize + (fsize * 2.2) as i32 + 22;
    }

    let w = gtk::ApplicationWindow::new(app);
    w.set_title(Some("L Launcher"));
    w.set_decorated(false);
    w.set_resizable(false);
    w.add_css_class("LLauncher");
    let card = gtk::Box::new(gtk::Orientation::Vertical, (spacing + 4).max(6));
    card.add_css_class("Card");
    card.set_margin_start(0);
    let inner = gtk::Box::new(gtk::Orientation::Vertical, (spacing + 4).max(6));
    inner.set_margin_start(pad);
    inner.set_margin_end(pad);
    inner.set_margin_top(pad);
    inner.set_margin_bottom(pad);
    card.append(&inner);
    w.set_child(Some(&card));
    let edit = gtk::Entry::new();
    edit.set_placeholder_text(Some(s.s("placeholder")));
    edit.set_hexpand(true);
    if s.b("show_prompt") && !s.s("prompt").is_empty() {
        let top = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let lbl = gtk::Label::new(Some(s.s("prompt")));
        lbl.add_css_class("Prompt");
        top.append(&lbl);
        top.append(&edit);
        inner.append(&top);
    } else {
        inner.append(&edit);
    }
    let list = gtk::FlowBox::new();
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.set_homogeneous(true);
    list.set_min_children_per_line(cols as u32);
    list.set_max_children_per_line(cols as u32);
    list.set_row_spacing(if cols > 1 { 0 } else { spacing as u32 });
    list.set_column_spacing(0);
    list.set_valign(gtk::Align::Start);
    list.set_activate_on_single_click(true);
    let sa = gtk::ScrolledWindow::new();
    sa.set_policy(gtk::PolicyType::Never,
        if s.b("show_scroll") { gtk::PolicyType::Automatic } else { gtk::PolicyType::External });
    sa.set_child(Some(&list));
    let list_h = rows * (row_h + spacing) + 6;
    sa.set_size_request(-1, list_h);
    inner.append(&sa);

    let font = s.s("font");
    let css = format!(
        "window.LLauncher {{ background: transparent; }}\n\
         window.LLauncher * {{ font-family: \"{font}\"; font-size: {fs}pt; color: {fg}; }}\n\
         window.LLauncher .Card {{ background: {bg}; border: {bw}px solid {border}; border-radius: {r}px; }}\n\
         window.LLauncher entry {{ background: {ibg}; border: none; border-radius: {er}px; padding: 8px 12px; \
            box-shadow: none; outline: none; min-height: 0; }}\n\
         window.LLauncher entry selection {{ background: {acc}; }}\n\
         window.LLauncher .Prompt {{ color: {acc}; font-weight: 600; padding-right: 6px; }}\n\
         window.LLauncher flowbox, window.LLauncher scrolledwindow {{ background: transparent; }}\n\
         window.LLauncher flowboxchild {{ border-radius: {er}px; padding: {ip}; outline: none; }}\n\
         window.LLauncher flowboxchild:selected {{ background: {sel}; }}\n\
         window.LLauncher flowboxchild:selected label {{ color: {self_fg}; }}\n\
         window.LLauncher scrollbar slider {{ background: {border}; border-radius: 3px; min-width: 6px; }}\n\
         window.LLauncher scrollbar {{ background: transparent; }}\n",
        fs = fsize, fg = rgba(s.s("fg")), bg = rgba(s.s("bg")), bw = s.i("border_width"), border = rgba(s.s("border")),
        r = s.i("radius"), ibg = rgba(s.s("input_bg")), er = s.i("el_radius"), acc = rgba(s.s("accent")),
        ip = if grid { "6px 4px" } else { "0 8px" }, sel = rgba(s.s("sel")), self_fg = rgba(s.s("sel_fg")));
    let prov = gtk::CssProvider::new();
    prov.load_from_string(&css);
    if let Some(d) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(&d, &prov, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 5);
    }
    // feste Größe → Hyprland lässt das Fenster schweben und zentriert es
    let entry_h = (fsize * 96.0 / 72.0 * 1.25) as i32 + 16;
    w.set_default_size(width, list_h + 2 * pad + entry_h + (spacing + 4).max(6) + 4);
    w.set_size_request(width, list_h + 2 * pad + entry_h + (spacing + 4).max(6) + 4);

    let shown: Rc<RefCell<Vec<App>>> = Rc::new(RefCell::new(vec![]));
    // Symbole nur einmal suchen
    let icons: Rc<RefCell<HashMap<String, gtk::Image>>> = Rc::new(RefCell::new(HashMap::new()));
    let make_icon = {
        let icons = icons.clone();
        move |a: &App| -> gtk::Image {
            if let Some(img) = icons.borrow().get(&a.id) {
                if img.parent().is_none() {
                    return img.clone();
                }
            }
            let img = if Path::new(&a.icon).is_absolute() {
                gtk::Image::from_file(&a.icon)
            } else {
                gtk::Image::from_icon_name(&a.icon)
            };
            img.set_pixel_size(isize);
            icons.borrow_mut().insert(a.id.clone(), img.clone());
            img
        }
    };
    let refresh = {
        let (apps, history, shown, list, edit, s) = (apps.clone(), history.clone(), shown.clone(), list.clone(),
            edit.clone(), s.clone());
        move || {
            let found: Vec<App> = al::filter_apps(&apps, &edit.text(), &history, s.b("insensitive")).into_iter()
                .cloned().collect();
            list.remove_all();
            for a in &found {
                let bx = gtk::Box::new(if grid { gtk::Orientation::Vertical } else { gtk::Orientation::Horizontal },
                    if grid { 4 } else { 8 });
                bx.set_size_request(if cols > 1 { cell_w - 6 } else { -1 }, row_h);
                if s.b("icons") && !a.icon.is_empty() {
                    let img = make_icon(a);
                    img.set_valign(if grid { gtk::Align::End } else { gtk::Align::Center });
                    img.set_vexpand(grid);
                    bx.append(&img);
                }
                let l = gtk::Label::new(Some(&a.name));
                l.set_xalign(if grid { 0.5 } else { 0.0 });
                l.set_ellipsize(gtk::pango::EllipsizeMode::End);
                if cols > 1 {
                    l.set_wrap(true);
                    l.set_lines(2);
                    l.set_justify(gtk::Justification::Center);
                }
                l.set_valign(if grid { gtk::Align::End } else { gtk::Align::Center });
                bx.append(&l);
                list.insert(&bx, -1);
            }
            *shown.borrow_mut() = found;
            if let Some(c) = list.child_at_index(0) {
                list.select_child(&c);
            }
        }
    };
    let refresh = Rc::new(refresh);
    let current = {
        let list = list.clone();
        move || list.selected_children().first().map(|c| c.index()).unwrap_or(-1)
    };
    let start = {
        let (shown, app, s) = (shown.clone(), app.clone(), s.clone());
        Rc::new(move |row: i32| {
            if let Some(a) = shown.borrow().get(row.max(0) as usize).filter(|_| row >= 0) {
                al::remember(&a.id);
                al::launch(a, s.s("terminal"));
            }
            app.quit();
        })
    };
    let r2 = refresh.clone();
    edit.connect_changed(move |_| r2());
    let st = start.clone();
    list.connect_child_activated(move |_, c| st(c.index()));
    let st = start.clone();
    edit.connect_activate(move |_| st(current()));
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let (list2, edit2, app2, st) = (list.clone(), edit.clone(), app.clone(), start.clone());
    keys.connect_key_pressed(move |_, key, _, state| {
        let n = shown.borrow().len() as i32;
        let cur = list2.selected_children().first().map(|c| c.index()).unwrap_or(0);
        let shift = state.contains(gdk::ModifierType::SHIFT_MASK);
        match key {
            gdk::Key::Escape => {
                app2.quit();
                return glib::Propagation::Stop;
            }
            gdk::Key::Return | gdk::Key::KP_Enter => {
                st(if n > 0 { cur } else { -1 });
                return glib::Propagation::Stop;
            }
            _ => {}
        }
        let mut step = match key {
            gdk::Key::Down => Some(cols),
            gdk::Key::Up => Some(-cols),
            gdk::Key::Tab if !shift => Some(1),
            gdk::Key::ISO_Left_Tab | gdk::Key::Tab => Some(-1),
            gdk::Key::Page_Down => Some(rows * cols),
            gdk::Key::Page_Up => Some(-rows * cols),
            _ => None,
        };
        if cols > 1 && edit2.text().is_empty() && (key == gdk::Key::Left || key == gdk::Key::Right) {
            // im Raster links/rechts, solange nichts getippt ist
            step = Some(if key == gdk::Key::Right { 1 } else { -1 });
        }
        if let (Some(st), true) = (step, n > 0) {
            let i = (cur + st).clamp(0, n - 1);
            if let Some(c) = list2.child_at_index(i) {
                list2.select_child(&c);
                c.grab_focus();
                edit2.grab_focus_without_selecting();
            }
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    w.add_controller(keys);
    // Fenster verliert den Fokus → schließen
    let was_active = Rc::new(std::cell::Cell::new(false));
    let app3 = app.clone();
    w.connect_is_active_notify(move |w| {
        if w.is_active() {
            was_active.set(true);
        } else if was_active.get() {
            app3.quit();
        }
    });
    // zweites Drücken → schließen
    let app4 = app.clone();
    glib::unix_signal_add_local(libc::SIGTERM, move || {
        app4.quit();
        glib::ControlFlow::Break
    });
    refresh();
    w.present();
    edit.grab_focus();
}
