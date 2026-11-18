//! Mauszeiger bzw. Symbole: Kacheln mit Vorschau, „Installieren“/„Installiert“ und „Anwenden“/„Angewendet“.
//! Quellordner einstellbar; installiert wird nach ~/.local/share/icons (ohne root).

use super::form::{flow_grid, tbtn, Page};
use super::img::{self, Raw};
use super::widgets::{chip, search_entry, seg, set_applied};
use super::win::{win, AioPage};
use crate::i18n::{t, tf};
use crate::looks::{self, Item};
use crate::remote::{self, Manifest, SourceError};
use crate::usercmd;
use gtk::gdk_pixbuf::{Colorspace, InterpType, Pixbuf};
use gtk::prelude::*;
use gtk::{cairo, glib};
use hyprstudio::jsonx::Get;
use hyprstudio::kw;
use hyprstudio::ui::draw::{self, Rgba, R};
use hyprstudio::ui::shell;
use hyprstudio::ui::widgets::{hbox, label, vbox, ComboItem, SearchCombo, Segmented};
use serde_json::{json, Map, Value};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub const TILE_W: i32 = 262;
pub const TILE_H: i32 = 206;

fn category(kind: &str) -> &'static str {
    match kind {
        "cursor" => "cursors",
        "icon" => "icons",
        _ => "login",
    }
}

fn src_err(e: SourceError) -> String {
    format!("{}: {}", e.key, e.detail)
}

/// Xcursor-Bytes (BGRA, vormultipliziert) → Pixbuf (RGBA).
fn xcursor_pixbuf(w: u32, h: u32, data: &[u8]) -> Option<Pixbuf> {
    let mut out = Vec::with_capacity(data.len());
    for px in data.chunks_exact(4) {
        let (b, g, r, a) = (px[0] as u32, px[1] as u32, px[2] as u32, px[3] as u32);
        let un = |c: u32| if a == 0 { 0 } else { ((c * 255 + a / 2) / a).min(255) as u8 };
        out.extend_from_slice(&[un(r), un(g), un(b), a as u8]);
    }
    let bytes = glib::Bytes::from_owned(out);
    Some(Pixbuf::from_bytes(&bytes, Colorspace::Rgb, true, 8, w as i32, h as i32, (w * 4) as i32))
}

fn strip_image(cells: Vec<Pixbuf>, cell: i32, pad: i32) -> Option<Pixbuf> {
    if cells.is_empty() {
        return None;
    }
    let out = Pixbuf::new(Colorspace::Rgb, true, 8, cell * cells.len() as i32, cell)?;
    out.fill(0);
    let inner = cell - 2 * pad;
    for (i, pb) in cells.iter().enumerate() {
        let f = (inner as f64 / pb.width() as f64).min(inner as f64 / pb.height() as f64);
        let (w, h) = (((pb.width() as f64) * f).round().max(1.0) as i32, ((pb.height() as f64) * f).round().max(1.0) as i32);
        let Some(s) = pb.scale_simple(w, h, InterpType::Bilinear) else { continue };
        let x = i as i32 * cell + (cell - w) / 2;
        let y = (cell - h) / 2;
        s.composite(&out, x, y, w, h, x as f64, y as f64, 1.0, 1.0, InterpType::Nearest, 255);
    }
    Some(out)
}

/// Vier Zeiger (Pfeil, Hand, Text, Warten) nebeneinander aus den Xcursor-Dateien.
pub fn cursor_preview(theme_dir: &Path) -> Option<Pixbuf> {
    let imgs: Vec<Pixbuf> = looks::CURSOR_PREVIEW.iter()
        .filter_map(|n| looks::cursor_file(theme_dir, n))
        .filter_map(|f| looks::read_xcursor(&f, 48))
        .filter_map(|(w, h, d)| xcursor_pixbuf(w, h, &d)).collect();
    strip_image(imgs, 44, 0)
}

fn inherits(theme_dir: &Path) -> Option<PathBuf> {
    let text = std::fs::read_to_string(theme_dir.join("index.theme")).ok()?;
    for ln in text.lines() {
        if let Some(v) = ln.strip_prefix("Inherits=") {
            for name in v.split(',') {
                let name = name.trim();
                if name.is_empty() {
                    continue;
                }
                let mut bases = vec![theme_dir.parent().map(Path::to_path_buf).unwrap_or_default()];
                bases.extend(looks::icon_dirs());
                for base in bases {
                    let p = base.join(name);
                    if p.is_dir() && p != theme_dir {
                        return Some(p);
                    }
                }
            }
        }
    }
    None
}

fn glob_svg(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir).map(|r| r.flatten().map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "svg").unwrap_or(false)).collect()).unwrap_or_default();
    v.sort();
    v
}

pub fn icon_preview(theme_dir: &Path) -> Option<Pixbuf> {
    let mut files: Vec<PathBuf> = looks::ICON_PREVIEW.iter().filter_map(|n| looks::icon_file(theme_dir, n)).take(4).collect();
    if files.len() < 2 {
        // Ergänzungs-Themes (z. B. MoreWaita): Eltern-Theme oder Programm-Symbole
        if let Some(parent) = inherits(theme_dir) {
            let need = 4 - files.len();
            files.extend(looks::ICON_PREVIEW.iter().filter_map(|n| looks::icon_file(&parent, n)).take(need));
        }
    }
    if files.len() < 2 {
        for sub in ["scalable/apps", "48x48/apps", "apps/48", "48/apps", "scalable/places"] {
            let need = 4usize.saturating_sub(files.len());
            files.extend(glob_svg(&theme_dir.join(sub)).into_iter().take(need));
            if files.len() >= 4 {
                break;
            }
        }
    }
    if files.is_empty() {
        return None;
    }
    let cells: Vec<Pixbuf> = files.iter().filter_map(|f| Pixbuf::from_file_at_scale(f, 36, 36, true).ok()).collect();
    if cells.is_empty() {
        return None;
    }
    strip_image(cells, 44, 4)
}

/// Heller Vorschau-Streifen (dunkle und helle Zeiger sichtbar).
pub struct Preview {
    pub area: gtk::DrawingArea,
    img: RefCell<Option<Rc<cairo::ImageSurface>>>,
}

impl Preview {
    pub fn new() -> Rc<Preview> {
        let area = gtk::DrawingArea::new();
        area.set_content_height(92);
        area.set_hexpand(true);
        let p = Rc::new(Preview { area, img: RefCell::new(None) });
        let w = Rc::downgrade(&p);
        p.area.set_draw_func(move |_, cr, wd, ht| {
            let Some(p) = w.upgrade() else { return };
            let r = R::new(0.0, 0.0, wd as f64, ht as f64);
            draw::fill_rounded(cr, r, 10.0, Rgba::hex("#dfe5ec"));
            let cur = p.img.borrow().clone();
            match cur.as_ref() {
                Some(s) => {
                    let (iw, ih) = (s.width() as f64, s.height() as f64);
                    let f = ((r.w - 16.0) / iw).min((r.h - 16.0) / ih);
                    let (w, h) = ((iw * f).floor(), (ih * f).floor());
                    super::img::draw_into(cr, s, R::new(((r.w - w) / 2.0).floor(), ((r.h - h) / 2.0).floor(), w, h));
                }
                None => {
                    let f = draw::Font::new(13.0, gtk::pango::Weight::Normal);
                    draw::text(cr, &f, r, "…", Rgba::hex("#7a8796"), draw::HAlign::Center, false);
                }
            }
        });
        p
    }

    pub fn set_image(&self, s: Option<Rc<cairo::ImageSurface>>) {
        *self.img.borrow_mut() = s;
        self.area.queue_draw();
    }
}

#[derive(Clone)]
pub struct LItem {
    pub item: Item,
    pub remote: Option<Map<String, Value>>,
}

pub struct LookTile {
    pub root: gtk::Box,
    pub item: RefCell<LItem>,
    pub pre: Rc<Preview>,
    b_inst: gtk::Button,
    b_apply: gtk::Button,
}

impl LookTile {
    fn new(it: LItem, page: &Rc<LookPage>) -> Rc<LookTile> {
        let root = vbox(8);
        root.add_css_class("ATile");
        root.set_size_request(TILE_W, TILE_H);
        root.add_css_class("Pad10");
        let pre = Preview::new();
        root.append(&pre.area);
        let text = it.item.color.clone().filter(|c| !c.is_empty()).unwrap_or_else(|| it.item.label.clone());
        let name = label(&text, "name");
        name.set_xalign(0.0);
        name.set_ellipsize(gtk::pango::EllipsizeMode::End);
        name.set_max_width_chars(1);
        name.set_hexpand(true);
        name.set_tooltip_text(Some(&it.item.name));
        root.append(&name);
        let row = hbox(6);
        let b_inst = gtk::Button::with_label("");
        let b_apply = gtk::Button::with_label("");
        b_apply.add_css_class("Primary");
        for b in [&b_inst, &b_apply] {
            hyprstudio::ui::widgets::cursor_hand(b);
            b.set_size_request(-1, 30);
            b.add_css_class("Small");
            b.set_hexpand(true);
            row.append(b);
        }
        root.append(&row);
        let tile = Rc::new(LookTile { root, item: RefCell::new(it), pre, b_inst, b_apply });
        let (wp, wt) = (Rc::downgrade(page), Rc::downgrade(&tile));
        tile.b_inst.connect_clicked(move |_| {
            if let (Some(p), Some(t)) = (wp.upgrade(), wt.upgrade()) {
                p.install(&t);
            }
        });
        let (wp, wt) = (Rc::downgrade(page), Rc::downgrade(&tile));
        tile.b_apply.connect_clicked(move |_| {
            if let (Some(p), Some(t)) = (wp.upgrade(), wt.upgrade()) {
                let it = t.item.borrow().clone();
                p.apply(&it);
            }
        });
        tile
    }

    fn refresh(&self, current: &str, busy: bool) {
        let it = self.item.borrow();
        let installed = it.item.installed.is_some();
        set_applied(&self.b_inst, installed, &t("installed"));
        if !installed {
            self.b_inst.set_label(&t("install"));
            self.b_inst.set_sensitive(!busy && (it.item.src.is_some() || it.remote.is_some()));
        }
        let active = installed && it.item.name == current;
        set_applied(&self.b_apply, active, &t("applied"));
        if !active {
            self.b_apply.set_label(&t("apply"));
            self.b_apply.set_sensitive(installed && !busy);
        }
        if active {
            self.root.add_css_class("act");
        } else {
            self.root.remove_css_class("act");
        }
    }
}

/// Gesamtpaket aus dem Release: nur „Alles installieren“.
pub struct PackCard {
    pub root: gtk::Box,
    pack: Map<String, Value>,
    info: gtk::Label,
    btn: gtk::Button,
}

impl PackCard {
    fn new(pack: Map<String, Value>, page: &Rc<LookPage>) -> Rc<PackCard> {
        let root = hbox(8);
        root.add_css_class("ACard");
        root.add_css_class("APack");
        let col = vbox(2);
        col.set_hexpand(true);
        let tt = pack.s("titel");
        let title = label(if tt.is_empty() { pack.s("name") } else { tt }, "name");
        title.set_xalign(0.0);
        title.add_css_class("Bold");
        let info = label("", "Label");
        info.set_xalign(0.0);
        col.append(&title);
        col.append(&info);
        root.append(&col);
        let btn = gtk::Button::with_label("");
        btn.add_css_class("Primary");
        hyprstudio::ui::widgets::cursor_hand(&btn);
        btn.set_size_request(-1, 34);
        btn.set_valign(gtk::Align::Center);
        root.append(&btn);
        let c = Rc::new(PackCard { root, pack, info, btn });
        let (wp, wc) = (Rc::downgrade(page), Rc::downgrade(&c));
        c.btn.connect_clicked(move |_| {
            if let (Some(p), Some(c)) = (wp.upgrade(), wc.upgrade()) {
                p.install_pack(&c.pack);
            }
        });
        c
    }

    fn refresh(&self, busy: bool, kind: &str) {
        let names: Vec<String> = self.pack.get("enthaelt").and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
        let have = names.iter().filter(|n| looks::installed_path(n, kind).is_some()).count();
        let mb = self.pack.get("groesse").and_then(|v| v.as_f64()).unwrap_or(0.0) / (1024.0 * 1024.0);
        self.info.set_text(&tf("remote.pack.info", kw!("n" => names.len(), "have" => have, "mb" => format!("{mb:.0}"))));
        let done = !names.is_empty() && have == names.len();
        set_applied(&self.btn, done, &t("remote.pack.done"));
        if !done {
            self.btn.set_label(&t("remote.pack.install"));
            self.btn.set_sensitive(!busy);
        }
    }
}

/// kind: cursor | icon
pub struct LookPage {
    pub page: Rc<Page>,
    kind: String,
    de: String,
    items: RefCell<Vec<LItem>>,
    tiles: RefCell<Vec<Rc<LookTile>>>,
    busy_name: RefCell<String>,
    previews: RefCell<HashMap<String, Option<Rc<cairo::ImageSurface>>>>,
    dry_applied: RefCell<String>,
    folder: RefCell<PathBuf>,
    search: gtk::Entry,
    source: RefCell<String>,
    src_seg: Rc<Segmented>,
    repo: RefCell<String>,
    manifest: RefCell<Option<Manifest>>,
    pack_cards: RefCell<Vec<Rc<PackCard>>>,
    size: Option<Rc<SearchCombo>>,
    l_folder: gtk::Label,
    path: gtk::Label,
    b_folder: gtk::Button,
    b_reload: gtk::Button,
    b_repo: gtk::Button,
    empty: gtk::Label,
    packs_box: gtk::Box,
    grid: gtk::FlowBox,
    loaded: Cell<bool>,
    gen: Cell<u32>,
}

impl LookPage {
    pub fn new(kind: &str, de: &str) -> Rc<LookPage> {
        let w = win();
        let page = Page::new(&format!("look.{kind}"), true);
        let cfg = w.cfg.borrow().clone();
        let default = if kind == "cursor" { looks::default_cursor_dir() } else { looks::default_icon_dir() };
        let fk = cfg.s(&format!("{kind}_dir"));
        let folder = if fk.is_empty() { default } else { PathBuf::from(fk) };
        let search = search_entry(170);
        let mut source = cfg.s(&format!("{kind}_source")).to_string();
        if source != "online" && source != "folder" {
            source = "online".into();
        }
        let src_seg = seg(&["online", "folder"], "remote.src.");
        src_seg.select(&source);
        page.tools.append(&src_seg.root);
        page.tools.append(&search);
        let repo = if cfg.s("remote_repo").is_empty() { remote::DEFAULT_REPO.to_string() } else { cfg.s("remote_repo").to_string() };
        let size = if kind == "cursor" {
            let c = super::form::combo();
            c.button.set_size_request(90, -1);
            let items: Vec<ComboItem> = [24, 32, 40, 48, 64].iter().map(|s| ComboItem::new(&format!("{s} px"), &s.to_string())).collect();
            c.set_items(items);
            let cur = cfg.get("cursor_size").and_then(|v| v.as_i64()).unwrap_or(24);
            c.set_current(c.find_data(&cur.to_string()).max(0));
            page.tools.append(&c.button);
            Some(c)
        } else {
            None
        };
        let bar = hbox(10);
        bar.add_css_class("DmBar");
        bar.add_css_class("PadBar");
        let l_folder = label("", "Label");
        let path = label("", "AMono");
        path.set_selectable(true);
        path.set_hexpand(true);
        path.set_xalign(0.0);
        path.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        let b_folder = tbtn("look.folder", false);
        let b_reload = tbtn("look.reload", false);
        let b_repo = tbtn("remote.repo.change", false);
        bar.append(&l_folder);
        bar.append(&path);
        bar.append(&b_reload);
        bar.append(&b_folder);
        bar.append(&b_repo);
        page.body.append(&bar);
        let empty = label("", "Label");
        empty.set_wrap(true);
        empty.set_xalign(0.0);
        empty.set_visible(false);
        page.body.append(&empty);
        let packs_box = vbox(8);
        page.body.append(&packs_box);
        let grid = flow_grid();
        page.body.append(&grid);
        let lp = Rc::new(LookPage { page, kind: kind.into(), de: de.into(), items: RefCell::new(vec![]),
            tiles: RefCell::new(vec![]), busy_name: RefCell::new(String::new()), previews: RefCell::new(HashMap::new()),
            dry_applied: RefCell::new(String::new()), folder: RefCell::new(folder), search, source: RefCell::new(source),
            src_seg, repo: RefCell::new(repo), manifest: RefCell::new(None), pack_cards: RefCell::new(vec![]), size,
            l_folder, path, b_folder, b_reload, b_repo, empty, packs_box, grid, loaded: Cell::new(false), gen: Cell::new(0) });
        // Zurücksetzen: nur wo gsettings den Zeiger/die Symbole hält
        let gs = ["gnome", "budgie", "pantheon", "cosmic", "hyprland", "sway", "i3", "niri"].contains(&de);
        if gs {
            let iface = crate::gnome::IFACE.to_string();
            let keys = if kind == "cursor" {
                vec![(iface.clone(), "cursor-theme".to_string()), (iface, "cursor-size".to_string())]
            } else {
                vec![(iface, "icon-theme".to_string())]
            };
            *lp.page.keys_override.borrow_mut() = Some(keys);
        } else {
            *lp.page.keys_override.borrow_mut() = Some(vec![]);
        }
        lp.page.finish();
        let wl = Rc::downgrade(&lp);
        *lp.page.reset_extra.borrow_mut() = Some(Rc::new(move |_| {
            if let Some(l) = wl.upgrade() {
                l.dry_applied.borrow_mut().clear();
            }
        }));
        let wl = Rc::downgrade(&lp);
        *lp.page.on_refresh.borrow_mut() = Some(Rc::new(move || {
            if let Some(l) = wl.upgrade() {
                if !l.tiles.borrow().is_empty() {
                    l.refresh_tiles();
                }
            }
        }));
        let wl = Rc::downgrade(&lp);
        lp.search.connect_changed(move |_| {
            if let Some(l) = wl.upgrade() {
                l.filter();
            }
        });
        let wl = Rc::downgrade(&lp);
        lp.src_seg.connect(move |s| {
            if let Some(l) = wl.upgrade() {
                *l.source.borrow_mut() = s.clone();
                let w = win();
                w.cfg.borrow_mut().insert(format!("{}_source", l.kind), json!(s));
                w.save();
                l.reload(false);
            }
        });
        if let Some(c) = &lp.size {
            let wc = Rc::downgrade(c);
            c.connect(move |_| {
                if let Some(c) = wc.upgrade() {
                    let w = win();
                    w.cfg.borrow_mut().insert("cursor_size".into(), json!(c.current_data().parse::<i64>().unwrap_or(24)));
                    w.save();
                }
            });
        }
        let wl = Rc::downgrade(&lp);
        lp.b_reload.connect_clicked(move |_| {
            if let Some(l) = wl.upgrade() {
                l.reload(true);
            }
        });
        let wl = Rc::downgrade(&lp);
        lp.b_folder.connect_clicked(move |_| {
            if let Some(l) = wl.upgrade() {
                l.choose_folder();
            }
        });
        let wl = Rc::downgrade(&lp);
        lp.b_repo.connect_clicked(move |_| {
            if let Some(l) = wl.upgrade() {
                l.choose_repo();
            }
        });
        // erst laden, wenn die Seite sichtbar wird
        let wl = Rc::downgrade(&lp);
        lp.page.root.connect_map(move |_| {
            if let Some(l) = wl.upgrade() {
                if !l.loaded.get() {
                    l.loaded.set(true);
                    let w2 = Rc::downgrade(&l);
                    glib::idle_add_local_once(move || {
                        if let Some(l) = w2.upgrade() {
                            l.reload(false);
                        }
                    });
                }
            }
        });
        lp
    }

    fn source_widgets(&self) {
        let online = *self.source.borrow() == "online";
        self.b_folder.set_visible(!online);
        self.b_repo.set_visible(online);
        self.l_folder.set_text(&if online { t("remote.repo") } else { t("look.source") });
        self.path.set_text(&if online { self.repo.borrow().clone() } else { self.folder.borrow().display().to_string() });
    }

    fn reload(self: &Rc<Self>, force: bool) {
        self.source_widgets();
        self.empty.set_visible(false);
        self.gen.set(self.gen.get() + 1);
        let gen = self.gen.get();
        let wl = Rc::downgrade(self);
        if *self.source.borrow() == "online" {
            let need = force || self.manifest.borrow().as_ref().map(|m| m.repo != *self.repo.borrow()).unwrap_or(true);
            if need {
                let repo = self.repo.borrow().clone();
                win().set_status(&tf("remote.loading", kw!("repo" => repo.clone())));
                shell::run_bg(move || remote::load_manifest(&repo).map_err(src_err), move |res| {
                    if let Some(l) = wl.upgrade() {
                        if l.gen.get() == gen {
                            l.manifest_done(res);
                        }
                    }
                });
            } else {
                self.show_remote();
            }
            return;
        }
        let folder = self.folder.borrow().clone();
        let kind = self.kind.clone();
        shell::run_bg(move || looks::items(&folder, &kind), move |items| {
            if let Some(l) = wl.upgrade() {
                if l.gen.get() == gen {
                    let li = items.into_iter().map(|item| LItem { item, remote: None }).collect();
                    l.items_done(li, vec![], None, false);
                }
            }
        });
    }

    fn manifest_done(self: &Rc<Self>, res: Result<Manifest, String>) {
        if *self.source.borrow() != "online" {
            return;
        }
        match res {
            Err(e) => {
                // Meldung zeigen, darunter wenigstens die schon installierten Themes
                *self.manifest.borrow_mut() = None;
                let msg = self.message(&e);
                win().set_status(&msg);
                let kind = self.kind.clone();
                let wl = Rc::downgrade(self);
                shell::run_bg(move || looks::items(Path::new("/nonexistent"), &kind), move |items| {
                    if let Some(l) = wl.upgrade() {
                        let li = items.into_iter().map(|item| LItem { item, remote: None }).collect();
                        l.items_done(li, vec![], Some(msg), true);
                    }
                });
            }
            Ok(m) => {
                win().set_status(&tf("remote.loaded", kw!("tag" => m.tag.clone(), "repo" => m.repo.clone())));
                *self.manifest.borrow_mut() = Some(m);
                self.show_remote();
            }
        }
    }

    fn show_remote(self: &Rc<Self>) {
        let Some(man) = self.manifest.borrow().clone() else { return };
        let (entries, packs) = remote::items(&man, category(&self.kind));
        let mut result = vec![];
        for e in &entries {
            let name = e.s("name").to_string();
            let farbe = e.s("farbe").to_string();
            let color = if farbe.is_empty() { None } else { Some(farbe.clone()) };
            result.push(LItem {
                item: Item { label: if farbe.is_empty() { name.clone() } else { farbe }, src: None,
                    installed: looks::installed_path(&name, &self.kind), color, name },
                remote: Some(e.clone()),
            });
        }
        let msg = if entries.is_empty() && packs.is_empty() {
            tf("remote.empty", kw!("repo" => self.repo.borrow().clone()))
        } else {
            String::new()
        };
        self.items_done(result, packs, Some(msg), true);
    }

    fn items_done(self: &Rc<Self>, items: Vec<LItem>, packs: Vec<Map<String, Value>>, message: Option<String>, raw: bool) {
        *self.items.borrow_mut() = items.clone();
        for c in self.pack_cards.borrow_mut().drain(..) {
            self.packs_box.remove(&c.root);
        }
        for pk in packs {
            let c = PackCard::new(pk, self);
            self.packs_box.append(&c.root);
            self.pack_cards.borrow_mut().push(c);
        }
        while let Some(c) = self.grid.first_child() {
            self.grid.remove(&c);
        }
        self.tiles.borrow_mut().clear();
        for it in &items {
            let tile = LookTile::new(it.clone(), self);
            self.grid.insert(&tile.root, -1);
            if let Some(s) = self.previews.borrow().get(&it.item.name) {
                tile.pre.set_image(s.clone());
            }
            self.tiles.borrow_mut().push(tile);
        }
        match message {
            Some(m) => {
                self.empty.set_text(&if raw { m.clone() } else { self.message(&m) });
                self.empty.set_visible(!m.is_empty());
            }
            None => {
                let has_src = items.iter().any(|i| i.item.src.is_some());
                self.empty.set_visible(!has_src);
                self.empty.set_text(&if !self.folder.borrow().is_dir() { t("look.nofolder") } else { t("look.nothemes") });
            }
        }
        self.filter();
        self.refresh_tiles();
        // Vorschauen: in Häppchen parallel laden, erscheinen nach und nach
        let todo: Vec<(String, Option<PathBuf>, String)> = items.iter()
            .filter(|it| !self.previews.borrow().contains_key(&it.item.name))
            .map(|it| {
                let path = it.item.installed.clone().or_else(|| it.item.src.clone().filter(|s| s.is_dir()));
                let url = it.remote.as_ref().map(|r| r.s("preview_url").to_string()).filter(|s| !s.is_empty())
                    .or_else(|| local_png(&it.item)).unwrap_or_default();
                (it.item.name.clone(), path, url)
            }).collect();
        let cursor = self.kind == "cursor";
        for part in todo.chunks(4) {
            let part = part.to_vec();
            let wl = Rc::downgrade(self);
            shell::run_bg(move || {
                part.into_iter().map(|(n, path, url)| {
                    let mut pb: Option<Pixbuf> = None;
                    if !url.is_empty() && !url.starts_with("http") {
                        pb = Pixbuf::from_file(&url).ok();
                    } else if !url.is_empty() {
                        pb = remote::preview(&url).and_then(|d| img::from_bytes(&d));
                    }
                    if pb.is_none() {
                        if let Some(p) = &path {
                            pb = if cursor { cursor_preview(p) } else { icon_preview(p) };
                        }
                    }
                    (n, pb.map(|p| Raw::from(&p)))
                }).collect::<Vec<(String, Option<Raw>)>>()
            }, move |res| {
                let Some(l) = wl.upgrade() else { return };
                for (name, raw) in res {
                    let s = raw.and_then(|r| img::surface(&r.pixbuf())).map(Rc::new);
                    l.previews.borrow_mut().insert(name.clone(), s.clone());
                    if let Some(tile) = l.tiles.borrow().iter().find(|t| t.item.borrow().item.name == name) {
                        tile.pre.set_image(s);
                    }
                }
            });
        }
    }

    /// SourceError-Text „remote.xyz: detail“ → verständliche Meldung.
    fn message(&self, text: &str) -> String {
        let (key, detail) = match text.split_once(':') {
            Some((k, d)) => (k.trim().to_string(), d.trim().to_string()),
            None => (text.trim().to_string(), String::new()),
        };
        let repo = self.repo.borrow().clone();
        let is_remote = key.starts_with("remote.");
        let msg = if is_remote { tf(&key, kw!("repo" => repo.clone())) } else { text.to_string() };
        if !detail.is_empty() && detail != repo && is_remote {
            format!("{msg} ({detail})")
        } else {
            msg
        }
    }

    fn filter(&self) {
        let q = self.search.text().trim().to_lowercase();
        for tile in self.tiles.borrow().iter() {
            let it = tile.item.borrow();
            let hay = format!("{} {} {}", it.item.name, it.item.label, it.item.color.clone().unwrap_or_default()).to_lowercase();
            if let Some(child) = tile.root.parent() {
                child.set_visible(q.is_empty() || hay.contains(&q));
            }
        }
    }

    fn current(&self) -> String {
        let cur = looks::current(&self.de, &self.kind);
        let d = self.dry_applied.borrow().clone();
        if usercmd::dry() && !d.is_empty() { d } else { cur }
    }

    fn refresh_tiles(&self) {
        let cur = self.current();
        let busy = !self.busy_name.borrow().is_empty() || win().busy();
        for tile in self.tiles.borrow().iter() {
            tile.refresh(&cur, busy);
        }
        for c in self.pack_cards.borrow().iter() {
            c.refresh(busy, &self.kind);
        }
        while let Some(c) = self.page.chips.first_child() {
            self.page.chips.remove(&c);
        }
        let text = if cur.is_empty() { t("look.unknown") } else { format!("{}: {}", t("look.active"), cur) };
        self.page.chips.append(&chip(&text, if cur.is_empty() { "Chip" } else { "ChipAccent" }));
    }

    // ----- Aktionen -----

    fn install(self: &Rc<Self>, tile: &Rc<LookTile>) {
        let it = tile.item.borrow().clone();
        if !self.busy_name.borrow().is_empty() || (it.item.src.is_none() && it.remote.is_none()) {
            return;
        }
        *self.busy_name.borrow_mut() = it.item.name.clone();
        win().set_status(&tf("look.installing", kw!("name" => it.item.name.clone())));
        self.refresh_tiles();
        let wl = Rc::downgrade(self);
        let wt = Rc::downgrade(tile);
        let done = move |res: Result<PathBuf, String>| {
            let (Some(l), Some(tile)) = (wl.upgrade(), wt.upgrade()) else { return };
            l.busy_name.borrow_mut().clear();
            let w = win();
            match res {
                Ok(p) => {
                    let name = {
                        let mut it = tile.item.borrow_mut();
                        it.item.installed = Some(p.clone());
                        it.item.name.clone()
                    };
                    for x in l.items.borrow_mut().iter_mut().filter(|x| x.item.name == name) {
                        x.item.installed = Some(p.clone());
                    }
                    w.log(&format!("→ {}", p.display()));
                    w.set_status(&tf("look.installed", kw!("name" => name)));
                }
                Err(e) => {
                    w.log(&e);
                    w.set_status(&format!("{}: {}", t("failed"), l.message(&e)));
                }
            }
            l.refresh_tiles();
        };
        if let Some(entry) = it.remote.clone() {
            let dest = looks::user_icons();
            let name = it.item.name.clone();
            shell::run_bg(move || {
                let target = dest.join(&name);
                if target.exists() {
                    return Ok(target);
                }
                remote::install(&entry, &dest, None).map(|_| target).map_err(src_err)
            }, done);
        } else if let Some(src) = it.item.src.clone() {
            shell::run_bg(move || looks::install(&src, &|_| {}), done);
        }
    }

    fn install_pack(self: &Rc<Self>, pack: &Map<String, Value>) {
        if !self.busy_name.borrow().is_empty() {
            return;
        }
        *self.busy_name.borrow_mut() = pack.s("name").to_string();
        let tt = pack.s("titel");
        win().set_status(&tf("look.installing", kw!("name" => if tt.is_empty() { pack.s("name") } else { tt })));
        self.refresh_tiles();
        let dest = looks::user_icons();
        let pk = pack.clone();
        let wl = Rc::downgrade(self);
        shell::run_bg(move || remote::install(&pk, &dest, None).map_err(src_err), move |res| {
            let Some(l) = wl.upgrade() else { return };
            l.busy_name.borrow_mut().clear();
            let w = win();
            match res {
                Ok(paths) => {
                    for p in &paths {
                        w.log(&format!("→ {}", p.display()));
                    }
                    for it in l.items.borrow_mut().iter_mut() {
                        if it.item.installed.is_none() {
                            it.item.installed = looks::installed_path(&it.item.name, &l.kind);
                        }
                    }
                    for tile in l.tiles.borrow().iter() {
                        let mut it = tile.item.borrow_mut();
                        if it.item.installed.is_none() {
                            it.item.installed = looks::installed_path(&it.item.name, &l.kind);
                        }
                    }
                    w.set_status(&tf("remote.pack.ok", kw!("n" => paths.len())));
                }
                Err(e) => {
                    w.log(&e);
                    w.set_status(&format!("{}: {}", t("failed"), l.message(&e)));
                }
            }
            l.refresh_tiles();
        });
    }

    fn choose_repo(self: &Rc<Self>) {
        let wl = Rc::downgrade(self);
        let cur = self.repo.borrow().clone();
        hyprstudio::ui::dialogs::ask_text(win().parent().as_ref(), &t("remote.repo.change"), &t("remote.repo.ask"), &cur,
            move |text| {
                let Some(l) = wl.upgrade() else { return };
                let text = text.unwrap_or_default().trim().to_string();
                if text.is_empty() {
                    return;
                }
                let w = win();
                if !remote::valid_repo(&text) {
                    w.set_status(&tf("remote.badrepo", kw!("repo" => text)));
                    return;
                }
                *l.repo.borrow_mut() = text.clone();
                w.cfg.borrow_mut().insert("remote_repo".into(), json!(text));
                w.save();
                l.reload(true);
            });
    }

    fn apply(&self, it: &LItem) {
        let w = win();
        w.log("");
        let ok = if self.kind == "cursor" {
            let size = self.size.as_ref().and_then(|c| c.current_data().parse::<i64>().ok()).unwrap_or(24);
            looks::apply_cursor(&self.de, &it.item.name, size, &|s| win().log(s))
        } else {
            looks::apply_icons(&self.de, &it.item.name)
        };
        if usercmd::dry() {
            *self.dry_applied.borrow_mut() = it.item.name.clone();
        }
        w.set_status(&if ok { t("done") } else { t("failed") });
        self.refresh_tiles();
    }

    fn choose_folder(self: &Rc<Self>) {
        let wl = Rc::downgrade(self);
        let start = self.folder.borrow().display().to_string();
        hyprstudio::ui::dialogs::open_dir(win().parent().as_ref(), &t("look.folder"), &start, move |d| {
            let (Some(l), Some(d)) = (wl.upgrade(), d) else { return };
            *l.folder.borrow_mut() = d.clone();
            let w = win();
            w.cfg.borrow_mut().insert(format!("{}_dir", l.kind), json!(d.display().to_string()));
            w.save();
            l.reload(false);
        });
    }
}

fn local_png(it: &Item) -> Option<String> {
    let src = it.src.as_ref()?;
    if !src.is_file() {
        return None;
    }
    // Archiv im Ordner: Vorschaubild daneben
    let mut pngs: Vec<PathBuf> = std::fs::read_dir(src.parent()?).ok()?.flatten().map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "png").unwrap_or(false)).collect();
    pngs.sort();
    pngs.first().map(|p| p.display().to_string())
}

impl AioPage for LookPage {
    fn widget(&self) -> gtk::Widget {
        self.page.root.clone().upcast()
    }

    fn retranslate(&self) {
        self.page.retranslate();
        self.src_seg.retranslate();
        self.source_widgets();
        self.search.set_placeholder_text(Some(&t("search")));
        if !self.tiles.borrow().is_empty() {
            self.refresh_tiles();
        }
    }

    fn refresh(&self) {
        if !self.tiles.borrow().is_empty() {
            self.refresh_tiles();
        }
    }
}
