//! Hyprland › Hintergrundbild (Hyprpaper) und › Mauszeiger (Hyprcursor/XCursor).
//! Beide haben eine Bibliothek bzw. Suche und können Neues aus dem Netz installieren (online.rs).

use super::dialogs::{self, MenuEntry};
use super::draw::{self, Font, HAlign, Rgba, R};
use super::gate::GateContent;
use super::studio::Studio;
use super::widgets::{button, chip, cursor_hand, flow_grid, hbox, label, no_wheel, scroller, set_chip, spacer, vbox,
                     Card, Host, Segmented, Switch};
use super::workspaces::monitors;
use crate::i18n::{t, tf};
use crate::jsonx::{Get, Obj};
use crate::{kw, model, online, paper, util};
use gtk::gdk::prelude::GdkCairoContextExt;
use gtk::prelude::*;
use gtk::{gdk, gdk_pixbuf::Pixbuf, gio, glib, pango};
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// ---------- Bilder ----------

/// Bild verkleinert laden (schnell auch bei großen Dateien), mindestens w×h (Cover).
fn load(path: &Path, w: f64, h: f64) -> Option<Pixbuf> {
    let (_, iw, ih) = Pixbuf::file_info(path)?;
    let pb = if iw > 0 && ih > 0 {
        let scale = (w / iw as f64).max(h / ih as f64);
        let sw = ((iw as f64 * scale) as i32).max(1);
        let sh = ((ih as f64 * scale) as i32).max(1);
        Pixbuf::from_file_at_scale(path, sw, sh, false).ok()?
    } else {
        Pixbuf::from_file(path).ok()?
    };
    Some(pb.apply_embedded_orientation().unwrap_or(pb))
}

fn from_bytes(data: &[u8]) -> Option<Pixbuf> {
    let stream = gio::MemoryInputStream::from_bytes(&glib::Bytes::from(data));
    Pixbuf::from_stream(&stream, gio::Cancellable::NONE).ok()
}

/// Bild so zeichnen, dass es r füllt (mittig beschnitten) oder ganz hineinpasst.
fn paint_pixbuf(cr: &gtk::cairo::Context, pb: &Pixbuf, r: R, cover: bool) {
    let (iw, ih) = (pb.width() as f64, pb.height() as f64);
    if iw <= 0.0 || ih <= 0.0 {
        return;
    }
    let s = if cover { (r.w / iw).max(r.h / ih) } else { (r.w / iw).min(r.h / ih) };
    let (dw, dh) = (iw * s, ih * s);
    cr.save().ok();
    cr.translate(r.x + (r.w - dw) / 2.0, r.y + (r.h - dh) / 2.0);
    cr.scale(s, s);
    cr.set_source_pixbuf(pb, 0.0, 0.0);
    cr.source().set_filter(gtk::cairo::Filter::Good);
    let _ = cr.paint();
    cr.restore().ok();
}

const SCALE: f64 = 2.0;

/// Vorschaubild mit runden Ecken; Ordner zeigen ihr erstes Bild, ohne Bild „keins“.
fn thumb(path: &str, w: i32, h: i32, radius: f64) -> gdk::Texture {
    let full = if path.is_empty() { None } else { Some(util::expanduser(path)) };
    let (lw, lh) = (w as f64 * SCALE, h as f64 * SCALE);
    let mut img = None;
    if let Some(full) = &full {
        if full.is_dir() {
            let mut files: Vec<PathBuf> = std::fs::read_dir(full).map(|d| d.flatten().map(|e| e.path()).collect())
                .unwrap_or_default();
            files.sort();
            for f in files {
                if paper::is_image(&f) {
                    img = load(&f, lw, lh);
                    if img.is_some() {
                        break;
                    }
                }
            }
        } else {
            img = load(full, lw, lh);
        }
    }
    draw::texture(w, h, SCALE, move |cr| {
        let r = R::new(0.0, 0.0, w as f64, h as f64);
        draw::rounded(cr, r, radius);
        cr.clip();
        match &img {
            Some(pb) => paint_pixbuf(cr, pb, r, true),
            None => {
                draw::fill_rect(cr, r, Rgba::hex("#1b2430"));
                draw::text(cr, &Font::new(13.0, pango::Weight::Normal), r, &t("paper.none"), Rgba::hex("#8a98ab"),
                    HAlign::Center, false);
            }
        }
    })
}

thread_local! {
    static THUMBS: RefCell<HashMap<(String, u64), gdk::Texture>> = RefCell::new(HashMap::new());
}

/// Vorschaubild der Bibliothek merken (nach Pfad und Änderungszeit) – Wiederöffnen ohne neu zu laden.
fn thumb_cached(path: &str) -> gdk::Texture {
    let mtime = std::fs::metadata(path).and_then(|m| m.modified()).ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_nanos() as u64).unwrap_or(0);
    let key = (path.to_string(), mtime);
    if let Some(tex) = THUMBS.with(|m| m.borrow().get(&key).cloned()) {
        return tex;
    }
    let tex = thumb(path, 172, 95, 10.0);
    THUMBS.with(|m| m.borrow_mut().insert(key, tex.clone()));
    tex
}

/// Online-Vorschau (Bytes) als Textur: `cover` füllt, sonst ganz sichtbar auf hellem Grund.
fn pic(data: &[u8], w: i32, h: i32, radius: f64, cover: bool) -> Option<gdk::Texture> {
    let pb = from_bytes(data)?;
    Some(draw::texture(w, h, SCALE, move |cr| {
        let r = R::new(0.0, 0.0, w as f64, h as f64);
        draw::rounded(cr, r, radius);
        cr.clip();
        if !cover {
            draw::fill_rect(cr, r, Rgba::hex("#e9edf2"));
        }
        paint_pixbuf(cr, &pb, r, cover);
    }))
}

fn picture(w: i32, h: i32) -> gtk::Picture {
    let p = gtk::Picture::new();
    p.set_size_request(w, h);
    p.set_can_shrink(true);
    p.set_content_fit(gtk::ContentFit::Fill);
    p.set_halign(gtk::Align::Start);
    p.set_valign(gtk::Align::Center);
    p
}

/// Bild in ein Picture fester Größe (Textur ist ×2 aufgelöst).
fn set_pic(p: &gtk::Picture, tex: &gdk::Texture) {
    let (w, h) = p.size_request();
    p.set_paintable(Some(&draw::logical(tex, w, h)));
}

fn short(path: &str) -> String {
    online::short(Path::new(path))
}

fn btn(primary: bool) -> gtk::Button {
    button("", if primary { "Primary" } else { "" })
}

/// Knopf für die Kopfzeile (Anwenden/Rückgängig) – überall gleich groß.
pub fn action(primary: bool) -> gtk::Button {
    let b = btn(primary);
    b.set_size_request(170, 40);
    b
}

/// Titel der Karte links, Knöpfe rechts in derselben Zeile.
pub fn card_header(card: &Card, widgets: &[&gtk::Widget]) {
    card.root.remove(&card.title);
    let row = hbox(8);
    super::widgets::margins(&row, 18, 14, 18, 0);
    card.title.set_margin_start(0);
    card.title.set_margin_end(0);
    card.title.set_margin_top(0);
    card.title.set_valign(gtk::Align::Center);
    row.append(&card.title);
    row.append(&spacer());
    for w in widgets {
        row.append(*w);
    }
    card.root.prepend(&row);
}

/// Seite mit Bildlauf; liefert (äußeres Widget, Inhalt).
pub fn scroll_page() -> (gtk::ScrolledWindow, gtk::Box) {
    let inner = vbox(14);
    inner.add_css_class("ScrollInner");
    inner.set_margin_end(6);
    (scroller(&inner), inner)
}

fn pick(parent: Option<gtk::Window>, folder: bool, start: &str, done: impl FnOnce(String) + 'static) {
    let start = if !start.is_empty() {
        util::expanduser(start).to_string_lossy().to_string()
    } else if online::wall_dir().is_dir() {
        online::wall_dir().to_string_lossy().to_string()
    } else {
        util::home().to_string_lossy().to_string()
    };
    let f = move |p: Option<PathBuf>| {
        if let Some(p) = p {
            done(p.to_string_lossy().to_string());
        }
    };
    if folder {
        dialogs::open_dir(parent.as_ref(), &t("paper.folder"), &start, f);
    } else {
        dialogs::open_file(parent.as_ref(), &t("paper.image"), &start,
            &[("Bilder", &["*.png", "*.jpg", "*.jpeg", "*.webp", "*.jxl", "*.bmp"])], f);
    }
}

/// Vorschaubilder nacheinander laden; `got` kommt im Hauptthread. Rückgabe = Abbruch-Schalter.
fn pic_job(items: Vec<(String, String)>, got: impl Fn(String, Vec<u8>) + 'static) -> Arc<AtomicBool> {
    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = stop.clone();
    let (tx, rx) = async_channel::unbounded::<(String, Vec<u8>)>();
    gio::spawn_blocking(move || {
        for (key, url) in items {
            if stop2.load(Ordering::Relaxed) {
                return;
            }
            if let Ok(data) = online::get(&url, 20) {
                if tx.send_blocking((key, data)).is_err() {
                    return;
                }
            }
        }
    });
    glib::spawn_future_local(async move {
        while let Ok((k, d)) = rx.recv().await {
            got(k, d);
        }
    });
    stop
}

fn clickable<W: IsA<gtk::Widget>>(w: &W, f: impl Fn() + 'static) {
    cursor_hand(w);
    let c = gtk::GestureClick::new();
    c.set_button(1);
    c.connect_released(move |_, _, _, _| f());
    w.add_controller(c);
}

/// Zeile im Stil „BindRow“ mit eigenem Innenabstand (links, oben, rechts, unten).
fn row_frame(l: i32, tp: i32, r: i32, b: i32, spacing: i32) -> gtk::Box {
    let row = hbox(spacing);
    row.add_css_class("BindRow");
    let p = gtk::CssProvider::new();
    p.load_from_string(&format!("box {{ padding: {tp}px {r}px {b}px {l}px; }}"));
    #[allow(deprecated)]
    row.style_context().add_provider(&p, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1);
    row
}

pub const WALL_SORTS: &[&str] = &["toplist", "date_added", "favorites", "random"];
const WALL_FILTERS: &[(&str, &[&str], &str)] = &[
    ("wall_sort", WALL_SORTS, "wsort."),
    ("wall_cat", &["all", "general", "anime"], "wcat."),
    ("wall_res", &["all", "fhd", "qhd", "uhd"], "wres."),
];

// ---------- Hintergrundbild ----------

struct LibThumb {
    path: String,
    frame: gtk::Box,
    pic: gtk::Picture,
}

pub struct PaperEditor {
    pub root: gtk::ScrolledWindow,
    win: Weak<Studio>,
    pub state: RefCell<Obj>,
    cards: RefCell<Vec<Rc<Card>>>,
    segs: RefCell<Vec<Rc<Segmented>>>,
    busy: Cell<bool>,
    thumbs: RefCell<Vec<Rc<LibThumb>>>,
    todo: RefCell<Vec<Rc<LibThumb>>>,
    timer: RefCell<Option<glib::SourceId>>,
    lib_files: RefCell<Option<Vec<PathBuf>>>,
    btn_undo: gtk::Button,
    btn_apply: gtk::Button,
    thumb: gtk::Picture,
    path_lbl: gtk::Label,
    btn_img: gtk::Button,
    btn_dir: gtk::Button,
    lib_card: RefCell<Option<Rc<Card>>>,
    btn_get: gtk::Button,
    btn_lib_open: gtk::Button,
    lib_empty: gtk::Label,
    grid: gtk::FlowBox,
    web_card: RefCell<Option<Rc<Card>>>,
    wsearch: gtk::Entry,
    wf: RefCell<Vec<(String, Rc<Segmented>, &'static [&'static str])>>,
    web_state: gtk::Label,
    web_grid: gtk::FlowBox,
    btn_wmore: gtk::Button,
    wgen: Cell<u64>,
    wpage: Cell<i64>,
    wlast: Cell<i64>,
    wtiles: RefCell<HashMap<String, gtk::Picture>>,
    wstop: RefCell<Option<Arc<AtomicBool>>>,
    wtimer: RefCell<Option<glib::SourceId>>,
    wloaded: Cell<bool>,
    mon_card: RefCell<Option<Rc<Card>>>,
    mon_box: gtk::Box,
    auto: Rc<Switch>,
}

thread_local! {
    static PAPERS: RefCell<Vec<Weak<PaperEditor>>> = const { RefCell::new(Vec::new()) };
}

impl Host for PaperEditor {
    fn value(&self, key: &str) -> Value {
        self.state.borrow().val(key).clone()
    }
    fn set_value(&self, key: &str, v: Value) {
        if let Some(me) = self.me() {
            me.set(key, v);
        }
    }
}

impl PaperEditor {
    fn me(&self) -> Option<Rc<PaperEditor>> {
        PAPERS.with(|p| p.borrow().iter().filter_map(|w| w.upgrade()).find(|x| std::ptr::eq(x.as_ref(), self)))
    }

    fn win(&self) -> Rc<Studio> {
        self.win.upgrade().expect("Fenster")
    }

    pub fn new(win: &Rc<Studio>) -> Rc<PaperEditor> {
        let state = paper::paper_state(win.cfg.borrow().obj("paper"));
        let (root, lay) = scroll_page();
        let thumb = picture(320, 180);
        let path_lbl = label("", "Mono");
        path_lbl.set_wrap(true);
        path_lbl.set_wrap_mode(pango::WrapMode::WordChar);
        let wsearch = super::widgets::clear_entry();
        wsearch.set_size_request(-1, 40);
        let grid = flow_grid();
        let web_grid = flow_grid();
        let mon_box = vbox(8);
        let e = Rc::new(PaperEditor { root, win: Rc::downgrade(win), state: RefCell::new(state),
            cards: RefCell::new(vec![]), segs: RefCell::new(vec![]), busy: Cell::new(false),
            thumbs: RefCell::new(vec![]), todo: RefCell::new(vec![]), timer: RefCell::new(None),
            lib_files: RefCell::new(None), btn_undo: action(false), btn_apply: action(true), thumb, path_lbl,
            btn_img: btn(false), btn_dir: btn(false), lib_card: RefCell::new(None), btn_get: btn(true),
            btn_lib_open: btn(false), lib_empty: label("", "RowLabel"), grid, web_card: RefCell::new(None), wsearch,
            wf: RefCell::new(vec![]), web_state: label("", "RowLabel"), web_grid, btn_wmore: btn(false),
            wgen: Cell::new(0), wpage: Cell::new(1), wlast: Cell::new(1), wtiles: RefCell::new(HashMap::new()),
            wstop: RefCell::new(None), wtimer: RefCell::new(None), wloaded: Cell::new(false),
            mon_card: RefCell::new(None), mon_box, auto: Switch::new() });
        PAPERS.with(|x| x.borrow_mut().push(Rc::downgrade(&e)));
        let hw: Weak<PaperEditor> = Rc::downgrade(&e);
        let w = Rc::downgrade(&e);
        let on = move |f: fn(&Rc<PaperEditor>)| {
            let w = w.clone();
            move || {
                if let Some(e) = w.upgrade() {
                    f(&e);
                }
            }
        };
        let f = on(|e| e.apply());
        e.btn_apply.connect_clicked(move |_| f());
        let f = on(|e| e.undo());
        e.btn_undo.connect_clicked(move |_| f());

        // Alle Monitore
        let c = Card::new("paper.all", hw.clone());
        e.cards.borrow_mut().push(c.clone());
        let row = hbox(16);
        row.append(&e.thumb);
        let col = vbox(8);
        col.append(&e.path_lbl);
        for b in [&e.btn_img, &e.btn_dir] {
            b.set_size_request(220, -1);
            b.set_halign(gtk::Align::Start);
            col.append(b);
        }
        col.set_hexpand(true);
        row.append(&col);
        c.append(&row);
        let f = on(|e| e.choose("", false));
        e.btn_img.connect_clicked(move |_| f());
        let f = on(|e| e.choose("", true));
        e.btn_dir.connect_clicked(move |_| f());
        c.choice("fit", paper::FITS, "fit.", None, Some("paper.fit"));
        lay.append(&c.root);

        // Bibliothek: installierte Bilder, Klick = für alle Monitore
        let lib = Card::new("paper.lib", hw.clone());
        e.cards.borrow_mut().push(lib.clone());
        let f = on(|e| e.get_menu());
        e.btn_get.connect_clicked(move |_| f());
        e.btn_lib_open.connect_clicked(|_| online::open_dir(&online::wall_dir()));
        card_header(&lib, &[e.btn_lib_open.upcast_ref(), e.btn_get.upcast_ref()]);
        e.lib_empty.set_wrap(true);
        lib.append(&e.lib_empty);
        lib.append(&e.grid);
        *e.lib_card.borrow_mut() = Some(lib.clone());

        // Online suchen (wallhaven.cc, nur jugendfrei)
        let web = Card::new("paper.web", hw.clone());
        e.cards.borrow_mut().push(web.clone());
        web.append(&e.wsearch);
        let frow = hbox(10);
        for (key, opts, pre) in WALL_FILTERS {
            let sg = Segmented::new(opts, pre);
            let w = Rc::downgrade(&e);
            let k = key.to_string();
            sg.connect(move |v| {
                if let Some(e) = w.upgrade() {
                    e.wall_filter(&k, &v);
                }
            });
            e.segs.borrow_mut().push(sg.clone());
            frow.append(&sg.root);
            e.wf.borrow_mut().push((key.to_string(), sg, opts));
        }
        web.append(&frow);
        e.web_state.set_wrap(true);
        web.append(&e.web_state);
        web.append(&e.web_grid);
        e.btn_wmore.set_halign(gtk::Align::Center);
        e.btn_wmore.set_visible(false);
        let f = on(|e| e.wall_search(true));
        e.btn_wmore.connect_clicked(move |_| f());
        web.append(&e.btn_wmore);
        lay.append(&web.root);
        lay.append(&lib.root); // Bibliothek unter der Suche
        *e.web_card.borrow_mut() = Some(web);
        let w = Rc::downgrade(&e);
        e.wsearch.connect_changed(move |_| {
            let Some(e) = w.upgrade() else { return };
            if let Some(id) = e.wtimer.borrow_mut().take() {
                id.remove();
            }
            let w = Rc::downgrade(&e);
            let id = glib::timeout_add_local_once(std::time::Duration::from_millis(500), move || {
                if let Some(e) = w.upgrade() {
                    e.wtimer.borrow_mut().take();
                    e.wall_search(false);
                }
            });
            *e.wtimer.borrow_mut() = Some(id);
        });

        // Pro Monitor
        let mc = Card::new("paper.per_monitor", hw.clone());
        e.cards.borrow_mut().push(mc.clone());
        mc.append(&e.mon_box);
        lay.append(&mc.root);
        *e.mon_card.borrow_mut() = Some(mc);

        // Diashow + Sonstiges
        let c = Card::new("paper.extra", hw.clone());
        e.cards.borrow_mut().push(c.clone());
        c.slider("timeout", 5.0, 600.0, 5.0, "{:.0f} s", Some("tip.paper.timeout"), Some("paper.timeout"));
        c.toggle("random", None, Some("paper.random"));
        c.toggle("recursive", None, Some("paper.recursive"));
        c.toggle("splash", None, Some("paper.splash"));
        let w = Rc::downgrade(&e);
        e.auto.connect(move |on| {
            if let Some(e) = w.upgrade() {
                e.autostart(on);
            }
        });
        c.add_row("paper.autostart", e.auto.area.upcast_ref(), None, Some("tip.paper.autostart"), None,
            Some("paper.autostart"));
        lay.append(&c.root);
        e.refresh_controls();
        e
    }

    // ----- Zustand -----

    pub fn set(self: &Rc<Self>, key: &str, value: Value) {
        self.state.borrow_mut().insert(key.into(), value);
        self.changed();
    }

    pub fn changed(&self) {
        let win = self.win();
        win.cfg.borrow_mut().insert("paper".into(), Value::Object(self.state.borrow().clone()));
        win.save_later();
        let p = self.state.borrow().s("path").to_string();
        set_pic(&self.thumb, &thumb(&p, 320, 180, 12.0));
        self.path_lbl.set_text(&short(&p));
        self.path_lbl.set_visible(!p.is_empty());
        self.mark();
    }

    pub fn refresh_controls(&self) {
        let s = self.state.borrow().clone();
        for c in self.cards.borrow().iter() {
            c.refresh(&s);
        }
        let on = self.win().hp().map(|hp| hp.state.borrow().strs("autostart").iter().any(|c| c == "hyprpaper"))
            .unwrap_or(false);
        self.auto.set_now(on);
        self.monitors();
        self.changed();
    }

    /// Beim Öffnen des Bereichs: Bibliothek neu einlesen, Online-Suche einmal starten.
    pub fn refresh_impl(self: &Rc<Self>) {
        self.load_library();
        for (key, sg, _) in self.wf.borrow().iter() {
            sg.select(&self.wfv(key));
        }
        if !self.wloaded.get() {
            self.wloaded.set(true);
            self.wall_search(false);
        }
    }

    // ----- Online suchen -----

    fn wfv(&self, key: &str) -> String {
        let opts = self.wf.borrow().iter().find(|(k, _, _)| k == key).map(|x| x.2).unwrap_or(&[]);
        let v = self.win().cfg.borrow().s(key).to_string();
        if opts.contains(&v.as_str()) { v } else { opts.first().unwrap_or(&"").to_string() }
    }

    fn wall_filter(self: &Rc<Self>, key: &str, value: &str) {
        let win = self.win();
        win.cfg.borrow_mut().insert(key.into(), json!(value));
        win.save_later();
        self.wall_search(false);
    }

    pub fn wall_search(self: &Rc<Self>, more: bool) {
        if more {
            self.wpage.set(self.wpage.get() + 1);
        } else {
            self.wgen.set(self.wgen.get() + 1);
            self.wpage.set(1);
            self.web_grid.remove_all();
            self.wtiles.borrow_mut().clear();
            if let Some(s) = self.wstop.borrow_mut().take() {
                s.store(true, Ordering::Relaxed);
            }
        }
        let (gen, page) = (self.wgen.get(), self.wpage.get());
        let q = self.wsearch.text().to_string();
        let (sort, cat, res) = (self.wfv("wall_sort"), self.wfv("wall_cat"), self.wfv("wall_res"));
        self.web_state.set_text(&t("paper.web_loading"));
        self.web_state.set_visible(true);
        self.btn_wmore.set_visible(false);
        let w = Rc::downgrade(self);
        super::shell::run_bg(move || online::wall_search(&q, &sort, &cat, &res, page), move |r| {
            if let Some(e) = w.upgrade() {
                e.wall_loaded(gen, r);
            }
        });
    }

    fn wall_loaded(self: &Rc<Self>, gen: u64, r: Result<(Vec<online::WallItem>, i64), String>) {
        if gen != self.wgen.get() {
            return;
        }
        let (items, last) = match r {
            Ok(x) => x,
            Err(_) => {
                self.web_state.set_text(&t("cur.offline"));
                return;
            }
        };
        self.wlast.set(last);
        let mut pics = vec![];
        for it in &items {
            let tile = row_frame(8, 8, 8, 8, 6);
            tile.set_orientation(gtk::Orientation::Vertical);
            tile.set_size_request(196, 146);
            tile.set_halign(gtk::Align::Start);
            let p = picture(180, 101);
            p.add_css_class("PicBg");
            let info = label(&format!("{}  ·  {}", it.res.replace('x', "×"), tf("paper.favs", kw!("n" => it.favs))),
                "RowLabel");
            tile.append(&p);
            tile.append(&info);
            tile.set_tooltip_text(Some(&t("tip.wall_tile")));
            let w = Rc::downgrade(self);
            let item = it.clone();
            clickable(&tile, move || {
                if let Some(e) = w.upgrade() {
                    e.wall_pick(item.clone());
                }
            });
            self.web_grid.insert(&tile, -1);
            self.wtiles.borrow_mut().insert(it.id.clone(), p);
            pics.push((it.id.clone(), it.thumb.clone()));
        }
        let empty = self.web_grid.first_child().is_none();
        self.web_state.set_text(&t("cur.nomatch"));
        self.web_state.set_visible(empty);
        self.btn_wmore.set_label(&t("cur.more"));
        self.btn_wmore.set_visible(self.wpage.get() < self.wlast.get() && !items.is_empty());
        if !pics.is_empty() {
            let w = Rc::downgrade(self);
            let stop = pic_job(pics, move |k, data| {
                let Some(e) = w.upgrade() else { return };
                if gen != e.wgen.get() {
                    return;
                }
                let pic_w = e.wtiles.borrow().get(&k).cloned();
                if let Some(p) = pic_w {
                    if let Some(tex) = pic(&data, 180, 101, 10.0, true) {
                        set_pic(&p, &tex);
                    }
                }
            });
            *self.wstop.borrow_mut() = Some(stop);
        }
    }

    fn wall_pick(self: &Rc<Self>, item: online::WallItem) {
        if self.busy.get() {
            return;
        }
        self.busy.set(true);
        let win = self.win();
        win.log("");
        win.set_status(&tf("dl.loading", kw!("name" => item.res)));
        let w = Rc::downgrade(self);
        super::shell::run_job(move |log| online::wall_install(&item, log), move |r| {
            let Some(e) = w.upgrade() else { return };
            e.busy.set(false);
            let win = e.win();
            match r {
                Err(err) => {
                    win.log(&format!("Fehler: {err}"));
                    win.set_status(&tf("dl.fail", kw!("err" => err)));
                }
                Ok(p) => {
                    e.state.borrow_mut().insert("path".into(), json!(p.to_string_lossy()));
                    e.changed();
                    e.monitors();
                    e.load_library();
                    let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    win.set_status(&tf("paper.picked", kw!("name" => name)));
                }
            }
        });
    }

    // ----- Bibliothek -----

    pub fn load_library(self: &Rc<Self>) {
        let files = online::library_images(60);
        if self.lib_files.borrow().as_ref() == Some(&files) {
            // nichts Neues → Vorschaubilder behalten
            self.mark();
            return;
        }
        if let Some(id) = self.timer.borrow_mut().take() {
            id.remove();
        }
        self.grid.remove_all();
        let mut thumbs = vec![];
        for f in &files {
            let fr = gtk::Box::new(gtk::Orientation::Vertical, 0);
            fr.add_css_class("Thumb");
            fr.set_size_request(176, 99);
            fr.set_halign(gtk::Align::Start);
            fr.set_tooltip_text(Some(&online::short(f)));
            let p = picture(172, 95);
            fr.append(&p);
            let th = Rc::new(LibThumb { path: f.to_string_lossy().to_string(), frame: fr, pic: p });
            let w = Rc::downgrade(self);
            let path = th.path.clone();
            clickable(&th.frame, move || {
                if let Some(e) = w.upgrade() {
                    e.lib_pick(&path);
                }
            });
            self.grid.insert(&th.frame, -1);
            thumbs.push(th);
        }
        *self.todo.borrow_mut() = thumbs.clone();
        *self.thumbs.borrow_mut() = thumbs;
        self.lib_empty.set_text(&tf("paper.lib_empty", kw!("dir" => online::short(&online::wall_dir()))));
        self.lib_empty.set_visible(files.is_empty());
        *self.lib_files.borrow_mut() = Some(files);
        self.mark();
        let w = Rc::downgrade(self);
        let id = glib::timeout_add_local(std::time::Duration::from_millis(15), move || {
            let Some(e) = w.upgrade() else { return glib::ControlFlow::Break };
            for _ in 0..3 {
                // ein paar pro Takt, damit die Oberfläche flüssig bleibt
                let next = {
                    let mut todo = e.todo.borrow_mut();
                    if todo.is_empty() { None } else { Some(todo.remove(0)) }
                };
                let Some(th) = next else {
                    e.timer.borrow_mut().take();
                    return glib::ControlFlow::Break;
                };
                set_pic(&th.pic, &thumb_cached(&th.path));
            }
            glib::ControlFlow::Continue
        });
        *self.timer.borrow_mut() = Some(id);
    }

    fn mark(&self) {
        let cur = util::expanduser(self.state.borrow().s("path")).to_string_lossy().to_string();
        for th in self.thumbs.borrow().iter() {
            if th.path == cur {
                th.frame.add_css_class("sel");
            } else {
                th.frame.remove_css_class("sel");
            }
        }
    }

    fn lib_pick(&self, path: &str) {
        self.state.borrow_mut().insert("path".into(), json!(path));
        self.changed();
        self.monitors();
        let name = Path::new(path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        self.win().set_status(&tf("paper.picked", kw!("name" => name)));
    }

    fn get_menu(self: &Rc<Self>) {
        let entries = online::WALL_PACKS.iter()
            .map(|p| MenuEntry::Item(tf("paper.pack", kw!("name" => p.label, "n" => online::MAX_WALLS))))
            .collect();
        let w = Rc::downgrade(self);
        dialogs::popup_menu(&self.btn_get, None, entries, move |i| {
            if let (Some(e), Some(p)) = (w.upgrade(), online::WALL_PACKS.get(i)) {
                e.install(p);
            }
        });
    }

    fn install(self: &Rc<Self>, pack: &'static online::WallPack) {
        if self.busy.get() {
            return;
        }
        self.busy.set(true);
        let win = self.win();
        win.log("");
        win.set_status(&tf("dl.loading", kw!("name" => pack.label)));
        self.btn_get.set_sensitive(false);
        let w = Rc::downgrade(self);
        super::shell::run_job(move |log| online::install_walls(pack, log), move |r| {
            let Some(e) = w.upgrade() else { return };
            e.busy.set(false);
            e.btn_get.set_sensitive(true);
            let win = e.win();
            match r {
                Err(err) => {
                    win.log(&format!("Fehler: {err}"));
                    win.set_status(&tf("dl.fail", kw!("err" => err)));
                }
                Ok(n) => {
                    win.set_status(&tf("paper.got", kw!("name" => pack.label, "n" => n)));
                    e.load_library();
                }
            }
        });
    }

    // ----- Auswahl -----

    fn choose(self: &Rc<Self>, mon: &str, folder: bool) {
        let cur = {
            let s = self.state.borrow();
            if mon.is_empty() {
                s.s("path").to_string()
            } else {
                s.obj("monitors").and_then(|m| m.obj(mon)).map(|m| m.s("path").to_string()).unwrap_or_default()
            }
        };
        let w = Rc::downgrade(self);
        let mon = mon.to_string();
        pick(super::shell::toplevel_of(&self.root), folder, &cur, move |path| {
            let Some(e) = w.upgrade() else { return };
            if !mon.is_empty() {
                {
                    let mut s = e.state.borrow_mut();
                    let fit = s.val("fit").clone();
                    let m = s.entry("monitors").or_insert_with(|| json!({}));
                    if let Some(m) = m.as_object_mut() {
                        m.insert(mon.clone(), json!({"path": path, "fit": fit}));
                    }
                }
                e.monitors();
            } else {
                e.state.borrow_mut().insert("path".into(), json!(path));
            }
            e.changed();
        });
    }

    fn clear(&self, mon: &str) {
        if let Some(m) = self.state.borrow_mut().get_mut("monitors").and_then(|m| m.as_object_mut()) {
            m.remove(mon);
        }
        self.monitors();
        self.changed();
    }

    fn monitors(&self) {
        super::widgets::clear_box(&self.mon_box);
        let Some(me) = self.me() else { return };
        let (base, mons) = {
            let s = self.state.borrow();
            (s.s("path").to_string(), s.obj("monitors").cloned().unwrap_or_default())
        };
        let mut names: Vec<String> = monitors();
        names.extend(mons.keys().cloned());
        names.sort();
        names.dedup();
        for mon in names {
            let m = mons.obj(&mon);
            let row = row_frame(12, 8, 10, 8, 10);
            let th = picture(96, 54);
            set_pic(&th, &thumb(m.map(|m| m.s("path")).unwrap_or(&base), 96, 54, 8.0));
            let name = label(&mon, "Mono");
            name.set_size_request(110, -1);
            let path = label(&m.map(|m| short(m.s("path"))).unwrap_or_else(|| t("paper.same")), "RowLabel");
            path.set_hexpand(true);
            path.set_ellipsize(pango::EllipsizeMode::Middle);
            row.append(&th);
            row.append(&name);
            row.append(&path);
            let mut btns: Vec<(String, u8)> = vec![(t("paper.image_btn"), 0), (t("paper.folder_btn"), 1)];
            if m.is_some() {
                btns.push((t("paper.reset"), 2));
            }
            for (txt, what) in btns {
                let b = button(&txt, "");
                b.set_size_request(96, -1);
                b.set_valign(gtk::Align::Center);
                let w = Rc::downgrade(&me);
                let x = mon.clone();
                b.connect_clicked(move |_| {
                    let Some(e) = w.upgrade() else { return };
                    match what {
                        0 => e.choose(&x, false),
                        1 => e.choose(&x, true),
                        _ => e.clear(&x),
                    }
                });
                row.append(&b);
            }
            self.mon_box.append(&row);
        }
        if let Some(c) = self.mon_card.borrow().as_ref() {
            c.root.set_visible(self.mon_box.first_child().is_some());
        }
    }

    fn autostart(&self, on: bool) {
        let win = self.win();
        let Some(hp) = win.hp() else { return };
        let mut cmds: Vec<String> = hp.state.borrow().strs("autostart").into_iter().filter(|c| c.trim() != "hyprpaper")
            .collect();
        if on {
            cmds.insert(0, "hyprpaper".into());
        }
        hp.state.borrow_mut().insert("autostart".into(), json!(cmds));
        hp.refresh_controls();
        hp.changed();
        win.set_status(&t("paper.auto_hint"));
    }

    pub fn apply(&self) {
        let win = self.win();
        win.log("");
        let st = self.state.borrow().clone();
        let (ok, info) = paper::apply_paper(&st, &model::config_home().join("hypr"), &|s| win.log(s));
        win.set_status(&if ok { t("done") } else { t(&info) });
    }

    pub fn undo(&self) {
        let win = self.win();
        let ok = model::undo_file(&model::config_home().join("hypr").join("hyprpaper.conf"));
        if ok {
            paper::restart_paper(&|s| win.log(s));
        }
        win.set_status(&if ok { t("undone") } else { t("nothing") });
    }

    pub fn retranslate(&self) {
        for c in self.cards.borrow().iter() {
            c.retranslate();
        }
        for sg in self.segs.borrow().iter() {
            sg.retranslate();
        }
        self.btn_img.set_label(&t("paper.image"));
        self.btn_dir.set_label(&t("paper.folder"));
        self.btn_get.set_label(&t("paper.get"));
        self.wsearch.set_placeholder_text(Some(&t("paper.web_search")));
        self.btn_wmore.set_label(&t("cur.more"));
        self.btn_lib_open.set_label(&t("paper.open"));
        self.btn_undo.set_label(&t("undo"));
        self.btn_apply.set_label(&t("apply"));
        self.lib_empty.set_text(&tf("paper.lib_empty", kw!("dir" => online::short(&online::wall_dir()))));
        self.monitors();
        self.changed();
    }

    pub fn apply_theme(&self, _accent: &str) {
        self.mark();
    }
}

impl GateContent for PaperEditor {
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }
    fn actions(&self) -> Vec<gtk::Button> {
        vec![self.btn_undo.clone(), self.btn_apply.clone()]
    }
    fn refresh(&self) {
        if let Some(me) = self.me() {
            me.refresh_impl();
        }
    }
}

// ---------- Mauszeiger ----------

pub const SORTS: &[&str] = &["rec", "high", "new", "down"];

pub struct CursorEditor {
    pub root: gtk::Box,
    win: Weak<Studio>,
    busy: Cell<bool>,
    stop: RefCell<Option<Arc<AtomicBool>>>,
    gen: Cell<u64>,
    page_no: Cell<i64>,
    total: Cell<i64>,
    store_count: Cell<usize>,
    btn_apply: gtk::Button,
    search: gtk::Entry,
    search_timer: RefCell<Option<glib::SourceId>>,
    inst_card: RefCell<Option<Rc<Card>>>,
    btn_refresh: gtk::Button,
    btn_open: gtk::Button,
    list: gtk::ListBox,
    names: RefCell<Vec<String>>,
    loading: Cell<bool>,
    l_size: gtk::Label,
    size: gtk::Scale,
    size_val: gtk::Label,
    l_gtk: gtk::Label,
    gtk_sw: Rc<Switch>,
    web_card: RefCell<Option<Rc<Card>>>,
    sort: Rc<Segmented>,
    web_state: gtk::Label,
    rows: gtk::Box,
    row_btns: RefCell<Vec<gtk::Button>>,
    row_pics: RefCell<HashMap<String, gtk::Picture>>,
    btn_more: gtk::Button,
    sort_key: RefCell<String>,
}

struct NoHost;
impl Host for NoHost {
    fn value(&self, _: &str) -> Value {
        Value::Null
    }
    fn set_value(&self, _: &str, _: Value) {}
}

thread_local! {
    static NOHOST: Rc<NoHost> = Rc::new(NoHost);
}

fn no_host() -> Weak<dyn Host> {
    NOHOST.with(|h| {
        let rc: Rc<dyn Host> = h.clone();
        Rc::downgrade(&rc)
    })
}

impl CursorEditor {
    fn win(&self) -> Rc<Studio> {
        self.win.upgrade().expect("Fenster")
    }

    pub fn new(win: &Rc<Studio>) -> Rc<CursorEditor> {
        let root = vbox(0);
        let (sa, lay) = scroll_page();
        root.append(&sa);
        let search = super::widgets::clear_entry();
        search.set_size_request(-1, 40);
        lay.append(&search);
        let list = gtk::ListBox::new();
        list.add_css_class("ModList");
        list.set_selection_mode(gtk::SelectionMode::Single);
        let list_sa = gtk::ScrolledWindow::new();
        list_sa.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        list_sa.set_child(Some(&list));
        list_sa.set_min_content_height(200);
        list_sa.set_max_content_height(320);
        list_sa.set_propagate_natural_height(true);
        list_sa.add_css_class("ModList");
        let adj = gtk::Adjustment::new(8.0, 8.0, 32.0, 1.0, 1.0, 0.0); // ×2 → 16 … 64
        let size = gtk::Scale::new(gtk::Orientation::Horizontal, Some(&adj));
        size.set_draw_value(false);
        size.set_digits(0);
        size.set_round_digits(0);
        size.set_hexpand(true);
        cursor_hand(&size);
        no_wheel(&size);
        let size_val = label("", "Value");
        size_val.set_xalign(1.0);
        size_val.set_size_request(44, -1);
        let gtk_sw = Switch::new();
        gtk_sw.set_now(win.cfg.borrow().get("cursor_gtk").map(crate::jsonx::truthy).unwrap_or(true));
        let sort = Segmented::new(SORTS, "cur.sort.");
        let sort_key = {
            let c = win.cfg.borrow();
            if SORTS.contains(&c.s("cursor_sort")) { c.s("cursor_sort").to_string() } else { "rec".into() }
        };
        let e = Rc::new(CursorEditor { root, win: Rc::downgrade(win), busy: Cell::new(false), stop: RefCell::new(None),
            gen: Cell::new(0), page_no: Cell::new(0), total: Cell::new(0), store_count: Cell::new(0),
            btn_apply: action(true), search, search_timer: RefCell::new(None), inst_card: RefCell::new(None),
            btn_refresh: btn(false), btn_open: btn(false), list, names: RefCell::new(vec![]), loading: Cell::new(false),
            l_size: label("", "RowLabel"), size, size_val, l_gtk: label("", "RowLabel"), gtk_sw,
            web_card: RefCell::new(None), sort, web_state: label("", "RowLabel"), rows: vbox(8),
            row_btns: RefCell::new(vec![]), row_pics: RefCell::new(HashMap::new()), btn_more: btn(false),
            sort_key: RefCell::new(sort_key) });
        CURSORS_ED.with(|c| c.borrow_mut().push(Rc::downgrade(&e)));
        let w = Rc::downgrade(&e);
        e.btn_apply.connect_clicked(move |_| {
            if let Some(e) = w.upgrade() {
                e.apply();
            }
        });
        let w = Rc::downgrade(&e);
        e.search.connect_changed(move |_| {
            if let Some(e) = w.upgrade() {
                e.search_changed();
            }
        });

        // Installiert
        let ic = Card::new("cur.title", no_host());
        let w = Rc::downgrade(&e);
        e.btn_refresh.connect_clicked(move |_| {
            if let Some(e) = w.upgrade() {
                e.load();
            }
        });
        e.btn_open.connect_clicked(|_| online::open_dir(&online::icons_dir()));
        card_header(&ic, &[e.btn_refresh.upcast_ref(), e.btn_open.upcast_ref()]);
        ic.append(&list_sa);
        let w = Rc::downgrade(&e);
        e.list.connect_row_selected(move |_, row| {
            let Some(e) = w.upgrade() else { return };
            if e.loading.get() {
                return;
            }
            if let Some(n) = row.and_then(|r| e.names.borrow().get(r.index() as usize).cloned()) {
                if !n.is_empty() {
                    e.win().set_status(&tf("cur.picked", kw!("name" => n)));
                }
            }
        });
        let grid = gtk::Grid::new();
        grid.set_column_spacing(14);
        grid.set_row_spacing(10);
        e.l_size.set_size_request(130, -1);
        e.l_size.set_xalign(0.0);
        e.l_gtk.set_xalign(0.0);
        let sv = e.size_val.clone();
        e.size.connect_value_changed(move |s| sv.set_text(&format!("{} px", s.value().round() as i64 * 2)));
        grid.attach(&e.l_size, 0, 0, 1, 1);
        grid.attach(&e.size, 1, 0, 1, 1);
        grid.attach(&e.size_val, 2, 0, 1, 1);
        grid.attach(&e.l_gtk, 0, 1, 1, 1);
        grid.attach(&e.gtk_sw.area, 1, 1, 1, 1);
        ic.append(&grid);
        lay.append(&ic.root);
        *e.inst_card.borrow_mut() = Some(ic);

        // Online installieren
        let wc = Card::new("cur.online", no_host());
        let w = Rc::downgrade(&e);
        e.sort.connect(move |k| {
            if let Some(e) = w.upgrade() {
                e.sort_changed(&k);
            }
        });
        card_header(&wc, &[e.sort.root.upcast_ref()]);
        e.web_state.set_wrap(true);
        wc.append(&e.web_state);
        wc.append(&e.rows);
        e.btn_more.set_halign(gtk::Align::Center);
        e.btn_more.set_visible(false);
        let w = Rc::downgrade(&e);
        e.btn_more.connect_clicked(move |_| {
            if let Some(e) = w.upgrade() {
                e.reload_store(true);
            }
        });
        wc.append(&e.btn_more);
        lay.append(&wc.root);
        *e.web_card.borrow_mut() = Some(wc);
        e
    }

    // ----- Laden -----

    pub fn refresh_impl(self: &Rc<Self>) {
        self.load();
        self.show_online();
    }

    pub fn load(&self) {
        let Some(hp) = self.win().hp() else { return };
        let (cur, csize) = {
            let hs = hp.state.borrow();
            (hs.s("cursor_theme").to_string(), hs.i("cursor_size"))
        };
        self.loading.set(true);
        super::widgets::clear_listbox(&self.list);
        let items = paper::cursor_themes();
        let mut names = vec![];
        let mut sel = None;
        for (i, (name, kind)) in items.iter().enumerate() {
            let l = label(&format!("{name}    ·  {}", t(&format!("cur.{kind}"))), "");
            l.set_xalign(0.0);
            self.list.append(&l);
            names.push(name.clone());
            if *name == cur {
                sel = Some(i);
            }
        }
        if items.is_empty() {
            let l = label(&t("cur.none"), "");
            l.set_xalign(0.0);
            self.list.append(&l);
            if let Some(r) = self.list.row_at_index(0) {
                r.set_selectable(false);
                r.set_activatable(false);
            }
            names.push(String::new());
        }
        *self.names.borrow_mut() = names;
        if let Some(i) = sel {
            self.list.select_row(self.list.row_at_index(i as i32).as_ref());
        }
        self.loading.set(false);
        self.size.set_value((csize / 2).clamp(8, 32) as f64);
        self.size_val.set_text(&format!("{} px", self.size.value().round() as i64 * 2));
        self.filter_installed();
    }

    fn filter_installed(&self) {
        let q = self.search.text().trim().to_lowercase();
        for (i, name) in self.names.borrow().iter().enumerate() {
            if let Some(r) = self.list.row_at_index(i as i32) {
                let n = name.to_lowercase();
                r.set_visible(!(!q.is_empty() && !n.is_empty() && !n.contains(&q)));
            }
        }
    }

    fn search_changed(self: &Rc<Self>) {
        self.filter_installed();
        if *self.sort_key.borrow() == "rec" {
            self.show_online();
        } else {
            if let Some(id) = self.search_timer.borrow_mut().take() {
                id.remove();
            }
            let w = Rc::downgrade(self);
            let id = glib::timeout_add_local_once(std::time::Duration::from_millis(500), move || {
                if let Some(e) = w.upgrade() {
                    e.search_timer.borrow_mut().take();
                    e.reload_store(false);
                }
            });
            *self.search_timer.borrow_mut() = Some(id);
        }
    }

    fn sort_changed(self: &Rc<Self>, key: &str) {
        if *self.sort_key.borrow() == key {
            return;
        }
        *self.sort_key.borrow_mut() = key.into();
        let win = self.win();
        win.cfg.borrow_mut().insert("cursor_sort".into(), json!(key));
        win.save_later();
        self.show_online();
    }

    // ----- Online -----

    fn row(self: &Rc<Self>, title: &str, meta: &str, key: &str) -> (gtk::Box, gtk::Picture, gtk::Label, gtk::Button) {
        let row = row_frame(10, 8, 10, 8, 12);
        let p = picture(112, 64);
        row.append(&p);
        let col = vbox(2);
        col.set_valign(gtk::Align::Center);
        col.set_hexpand(true);
        let name = label(title, "Strong");
        name.set_xalign(0.0);
        name.set_wrap(true);
        let sub = label(meta, "RowLabel");
        sub.set_xalign(0.0);
        col.append(&name);
        col.append(&sub);
        row.append(&col);
        let ch = chip();
        ch.set_valign(gtk::Align::Center);
        row.append(&ch);
        let b = btn(true);
        b.set_size_request(130, -1);
        b.set_valign(gtk::Align::Center);
        b.set_label(&t("cur.install"));
        b.set_sensitive(!self.busy.get());
        row.append(&b);
        self.rows.append(&row);
        self.row_btns.borrow_mut().push(b.clone());
        self.row_pics.borrow_mut().insert(key.into(), p.clone());
        (row, p, ch, b)
    }

    fn clear_rows(&self) {
        super::widgets::clear_box(&self.rows);
        self.row_btns.borrow_mut().clear();
        self.row_pics.borrow_mut().clear();
    }

    pub fn show_online(self: &Rc<Self>) {
        self.gen.set(self.gen.get() + 1);
        if let Some(s) = self.stop.borrow_mut().take() {
            s.store(true, Ordering::Relaxed);
        }
        let key = self.sort_key.borrow().clone();
        self.sort.select(&key);
        self.clear_rows();
        self.btn_more.set_visible(false);
        if key != "rec" {
            self.store_count.set(0);
            self.page_no.set(0);
            self.total.set(0);
            self.reload_store(false);
            return;
        }
        let q = self.search.text().trim().to_lowercase();
        let mut hits = 0;
        for c in online::CURSORS {
            if !q.is_empty() && !c.label.to_lowercase().contains(&q) && !c.names.iter().any(|n| n.to_lowercase().contains(&q)) {
                continue;
            }
            hits += 1;
            let src = c.repo.split("/releases").next().unwrap_or(c.repo);
            let src = format!("github.com/{}", src.split("github.com/").last().unwrap_or(src));
            let (_, p, ch, b) = self.row(c.label, &src, c.id);
            p.set_visible(false); // keine Vorschau für GitHub-Releases
            let inst = online::installed_cursor(c);
            set_chip(&ch, true, &t("cur.installed"));
            ch.set_visible(inst);
            b.set_visible(!inst);
            let w = Rc::downgrade(self);
            b.connect_clicked(move |_| {
                if let Some(e) = w.upgrade() {
                    e.install(c.label.to_string(), Box::new(move |log| online::install_cursor(c, log)));
                }
            });
        }
        self.web_state.set_text(&t("cur.nomatch"));
        self.web_state.set_visible(hits == 0);
    }

    pub fn reload_store(self: &Rc<Self>, more: bool) {
        if *self.sort_key.borrow() == "rec" {
            return;
        }
        if !more {
            self.gen.set(self.gen.get() + 1);
            self.clear_rows();
            self.store_count.set(0);
            self.page_no.set(0);
        } else {
            self.page_no.set(self.page_no.get() + 1);
        }
        let (gen, q, sort, page) = (self.gen.get(), self.search.text().to_string(), self.sort_key.borrow().clone(),
            self.page_no.get());
        self.web_state.set_text(&t("cur.loading_list"));
        self.web_state.set_visible(true);
        self.btn_more.set_visible(false);
        let w = Rc::downgrade(self);
        super::shell::run_bg(move || online::store_search(&q, &sort, page), move |r| {
            if let Some(e) = w.upgrade() {
                e.store_loaded(gen, r);
            }
        });
    }

    fn store_loaded(self: &Rc<Self>, gen: u64, r: Result<(Vec<online::StoreItem>, i64), String>) {
        if gen != self.gen.get() {
            return;
        }
        let (items, total) = match r {
            Ok(x) => x,
            Err(_) => {
                self.web_state.set_text(&t("cur.offline"));
                return;
            }
        };
        self.total.set(total);
        self.web_state.set_visible(items.is_empty() && self.store_count.get() == 0);
        self.web_state.set_text(&t("cur.nomatch"));
        let mut pics = vec![];
        for it in &items {
            let date = if it.changed.is_empty() {
                String::new()
            } else {
                it.changed.split('-').rev().collect::<Vec<_>>().join(".")
            };
            let meta = tf("cur.meta", kw!("score" => format!("{:.1}", it.score as f64 / 10.0),
                "dl" => thousands(it.downloads), "date" => date));
            let (_, _, ch, b) = self.row(&it.name, &meta, &it.id);
            ch.set_visible(false);
            let w = Rc::downgrade(self);
            let item = it.clone();
            b.connect_clicked(move |b| {
                if let Some(e) = w.upgrade() {
                    e.store_click(&item, b);
                }
            });
            if !it.preview.is_empty() {
                pics.push((it.id.clone(), it.preview.clone()));
            }
        }
        self.store_count.set(self.store_count.get() + items.len());
        self.btn_more.set_label(&t("cur.more"));
        self.btn_more.set_visible((self.page_no.get() + 1) * 10 < self.total.get() && !items.is_empty());
        if !pics.is_empty() {
            let w = Rc::downgrade(self);
            let stop = pic_job(pics, move |k, data| {
                let Some(e) = w.upgrade() else { return };
                if gen != e.gen.get() {
                    return;
                }
                let pic_w = e.row_pics.borrow().get(&k).cloned();
                if let Some(p) = pic_w {
                    if let Some(tex) = pic(&data, 112, 64, 8.0, false) {
                        set_pic(&p, &tex);
                    }
                }
            });
            *self.stop.borrow_mut() = Some(stop);
        }
    }

    fn store_click(self: &Rc<Self>, item: &online::StoreItem, b: &gtk::Button) {
        if item.files.len() == 1 {
            let i = item.files[0].0;
            let it = item.clone();
            self.install(item.name.clone(), Box::new(move |log| online::store_install(&it, i, log)));
            return;
        }
        // mehrere Varianten (Farben, Größen) → auswählen
        let entries = item.files.iter()
            .map(|(_, name, size)| MenuEntry::Item(if *size != 0 { format!("{name}   ·  {size} KB") } else { name.clone() }))
            .collect();
        let w = Rc::downgrade(self);
        let it = item.clone();
        dialogs::popup_menu(b, None, entries, move |n| {
            let (Some(e), Some(f)) = (w.upgrade(), it.files.get(n)) else { return };
            let i = f.0;
            let it2 = it.clone();
            e.install(it.name.clone(), Box::new(move |log| online::store_install(&it2, i, log)));
        });
    }

    #[allow(clippy::type_complexity)]
    fn install(self: &Rc<Self>, label: String,
        f: Box<dyn FnOnce(model::Log) -> Result<Vec<String>, String> + Send>) {
        if self.busy.get() {
            return;
        }
        let win = self.win();
        win.log("");
        win.set_status(&tf("dl.loading", kw!("name" => label)));
        self.set_busy(true);
        let w = Rc::downgrade(self);
        super::shell::run_job(move |log| f(log), move |r| {
            if let Some(e) = w.upgrade() {
                e.installed(&label, r);
            }
        });
    }

    fn set_busy(&self, on: bool) {
        self.busy.set(on);
        for b in self.row_btns.borrow().iter() {
            b.set_sensitive(!on);
        }
    }

    fn installed(self: &Rc<Self>, label: &str, r: Result<Vec<String>, String>) {
        self.set_busy(false);
        let win = self.win();
        let names = match r {
            Err(err) => {
                win.log(&format!("Fehler: {err}"));
                win.set_status(&tf("dl.fail", kw!("err" => err)));
                return;
            }
            Ok(n) => n,
        };
        self.load();
        if *self.sort_key.borrow() == "rec" {
            self.show_online();
        }
        if let Some(first) = names.first() {
            // neues Thema gleich auswählen
            let idx = self.names.borrow().iter().position(|n| n == first);
            if let Some(i) = idx {
                self.list.select_row(self.list.row_at_index(i as i32).as_ref());
            }
        }
        win.set_status(&if names.is_empty() {
            tf("cur.nothing_new", kw!("name" => label))
        } else {
            tf("cur.got", kw!("name" => label))
        });
    }

    fn current(&self) -> String {
        self.list.selected_row().and_then(|r| self.names.borrow().get(r.index() as usize).cloned()).unwrap_or_default()
    }

    pub fn apply(&self) {
        let win = self.win();
        let Some(hp) = win.hp() else { return };
        let theme = self.current();
        let size = self.size.value().round() as i64 * 2;
        let gtk_on = self.gtk_sw.is_on();
        {
            let mut hs = hp.state.borrow_mut();
            if !theme.is_empty() {
                hs.insert("cursor_theme".into(), json!(theme));
            }
            hs.insert("cursor_size".into(), json!(size));
        }
        win.cfg.borrow_mut().insert("cursor_gtk".into(), json!(gtk_on));
        hp.refresh_controls();
        hp.changed();
        hp.apply();
        let th = hp.state.borrow().s("cursor_theme").to_string();
        paper::set_cursor_live(&th, size, gtk_on, &|s| win.log(s));
        win.set_status(&tf("cur.done", kw!("name" => if th.is_empty() { "–".to_string() } else { th }, "size" => size)));
    }

    pub fn retranslate(&self) {
        if let Some(c) = self.inst_card.borrow().as_ref() {
            c.retranslate();
        }
        if let Some(c) = self.web_card.borrow().as_ref() {
            c.retranslate();
        }
        self.sort.retranslate();
        self.search.set_placeholder_text(Some(&t("cur.search")));
        self.btn_refresh.set_label(&t("bak.refresh"));
        self.btn_open.set_label(&t("cur.open"));
        self.btn_more.set_label(&t("cur.more"));
        self.l_size.set_text(&t("cursor_size"));
        self.l_gtk.set_text(&t("cur.gtk"));
        self.btn_apply.set_label(&t("apply"));
        self.load();
        self.sort.select(&self.sort_key.borrow());
    }

    pub fn apply_theme(&self, _accent: &str) {}
}

thread_local! {
    static CURSORS_ED: RefCell<Vec<Weak<CursorEditor>>> = const { RefCell::new(Vec::new()) };
}

impl GateContent for CursorEditor {
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }
    fn actions(&self) -> Vec<gtk::Button> {
        vec![self.btn_apply.clone()]
    }
    fn refresh(&self) {
        // Rc zu self finden (GateContent bekommt nur &self)
        let me = CURSORS_ED.with(|c| c.borrow().iter().filter_map(|w| w.upgrade())
            .find(|x| std::ptr::eq(x.as_ref(), self)));
        if let Some(me) = me {
            me.refresh_impl();
        }
    }
}

/// 1234567 → "1.234.567"
fn thousands(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(ch);
    }
    if n < 0 { format!("-{out}") } else { out }
}

#[cfg(test)]
mod tests {
    #[test]
    fn thousands() {
        assert_eq!(super::thousands(1234567), "1.234.567");
        assert_eq!(super::thousands(12), "12");
        assert_eq!(super::thousands(1000), "1.000");
    }
}
