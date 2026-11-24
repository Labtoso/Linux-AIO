//! Dunkles Design (Design-Vorlage Nr. 1) mit den Akzentfarben der gemeinsamen Paletten, kein Light-Theme.
//! Gleiche Farben und Maße wie das frühere Qt-Stylesheet; die CSS-Klassen heißen wie die früheren objectNames.

use crate::palettes;
use gtk::prelude::*;
use std::cell::RefCell;

pub const BG: &str = "#0e131a";
pub const SURFACE: &str = "#151c26";
pub const SURFACE2: &str = "#1b2430";
pub const BORDER: &str = "#253142";
pub const TEXT: &str = "#e6edf5";
pub const MUTED: &str = "#8a98ab";
pub const DANGER: &str = "#e5534b";
pub const OK: &str = "#3fb950";
pub const WARN: &str = "#d29922";
/// gewählter Zustand (Segmente, Tabs, Sprache) – neutral statt Akzentfarbe
pub const RAISED: &str = "#2b3747";
/// Rahmen beim Darüberfahren
pub const LINE: &str = "#3a4a5f";
/// Hintergrund beim Darüberfahren
pub const HOVER: &str = "#1f2a38";
pub const DISABLED: &str = "#4d5a6b";
pub const MONO: &str = "\"JetBrains Mono\", \"Hack\", \"DejaVu Sans Mono\", monospace";

/// Akzentfarbe eines Oberflächen-Themas (gemeinsame Paletten), Standard „01“.
pub fn accent_of(name: &str) -> String {
    palettes::theme_color(name).or_else(|| palettes::theme_color("01")).unwrap_or_else(|| "#ffffff".into())
}

pub fn rgb(c: &str) -> (f64, f64, f64) {
    let h = c.trim_start_matches('#');
    let p = |i: usize| u8::from_str_radix(h.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0) as f64 / 255.0;
    (p(0), p(2), p(4))
}

/// Schrift auf der Akzentfarbe: dunkel auf hellen Farben (Weiß, Mint, Orange), sonst weiß.
pub fn on_accent(accent: &str) -> &'static str {
    let (r, g, b) = rgb(accent);
    if 0.2126 * r + 0.7152 * g + 0.0722 * b > 0.55 { BG } else { "#ffffff" }
}

pub fn css(accent: &str) -> String {
    let on = on_accent(accent);
    let a = accent;
    format!(r#"
window, window.background, .hs-window {{ background: transparent; color: {TEXT}; font-size: 10pt; }}
popover, popover.background {{ background: transparent; }}
label {{ color: inherit; }}
tooltip, tooltip.background {{ background: {SURFACE2}; color: {TEXT}; border: 1px solid {BORDER}; border-radius: 6px; padding: 6px; box-shadow: none; }}
tooltip label {{ color: {TEXT}; }}
.Root {{ background: {BG}; border: 1px solid {BORDER}; border-radius: 16px; }}
.Title {{ font-size: 12pt; font-weight: 600; }}
.Side {{ background: {SURFACE}; border: 1px solid {BORDER}; border-radius: 12px; padding: 12px 10px; }}
flowboxchild {{ padding: 0; background: transparent; outline: none; }}
.Thumb {{ border: 2px solid transparent; border-radius: 12px; }}
.Thumb.sel {{ border-color: {a}; }}
.PicBg {{ background: #1b2430; border-radius: 10px; }}
.Strong {{ font-weight: 600; }}
.WsRow {{ padding: 6px 8px 6px 12px; }}
button.Big {{ font-size: 11pt; }}
list.ModList {{ background: {BG}; border: 1px solid {BORDER}; border-radius: 12px; padding: 6px; }}
list.ModList row {{ background: {SURFACE2}; border: 1px solid {BORDER}; border-radius: 8px; padding: 6px 10px; margin: 2px 0; }}
list.ModList row:selected {{ border-color: {a}; }}
scrolledwindow.ModList {{ border: none; background: transparent; }}
.Card {{ background: {SURFACE}; border: 1px solid {BORDER}; border-radius: 12px; }}
.CardTitle {{ font-size: 10.5pt; font-weight: 600; }}
.RowLabel {{ color: {MUTED}; }}
.Value {{ color: {TEXT}; font-family: {MONO}; }}
.PageTitle {{ font-size: 15pt; font-weight: 600; }}
.PanelTitle {{ font-size: 13pt; font-weight: 600; }}
.Label {{ color: {MUTED}; font-size: 9pt; }}
.ExtLink link, .ExtLink link:visited {{ color: {MUTED}; text-decoration: underline; }}
.ExtLink link:hover {{ color: {TEXT}; }}
.Status {{ color: {MUTED}; }}
.Mono {{ font-family: {MONO}; color: {TEXT}; }}
.Code {{ font-family: {MONO}; font-size: 9pt; color: {MUTED}; background: {BG}; border-radius: 6px; padding: 4px 8px; }}
.ChipOk {{ color: {OK}; font-size: 9pt; }}
.ChipWarn {{ color: {WARN}; font-size: 9pt; }}
.chip {{ color: {MUTED}; font-size: 9pt; padding: 2px 4px 2px 0; }}
.Warn {{ border: 1px solid {DANGER}; border-radius: 11px; color: {DANGER}; font-weight: 700; padding: 0 7px; }}

button {{ background: {SURFACE2}; background-image: none; border: 1px solid {BORDER}; border-radius: 7px;
          padding: 8px 16px; color: {TEXT}; box-shadow: none; text-shadow: none; min-height: 0; min-width: 0;
          outline: none; -gtk-icon-shadow: none; }}
button:hover {{ border-color: {LINE}; background: {HOVER}; }}
button:active {{ background: {HOVER}; }}
button:disabled, button:disabled label {{ color: {DISABLED}; border-color: {SURFACE2}; }}
button.Primary {{ background: {a}; border-color: {a}; color: {on}; font-weight: 600; }}
button.Primary label {{ color: {on}; }}
button.Primary:hover {{ background: {a}; border-color: {a}; }}
button.Primary:disabled {{ background: {SURFACE2}; border-color: {SURFACE2}; }}
button.Primary:disabled label {{ color: {DISABLED}; }}
button.Danger:hover, button.Danger:hover label {{ border-color: {DANGER}; color: {DANGER}; }}
button.Outline {{ background: transparent; border: 1px solid {LINE}; color: {TEXT}; }}
button.Outline:hover {{ background: {HOVER}; }}
button.DangerFill {{ background: transparent; border: 1px solid {DANGER}; color: {DANGER}; font-weight: 600; }}
button.DangerFill label {{ color: {DANGER}; }}
button.DangerFill:hover {{ background: {DANGER}; }}
button.DangerFill:hover label {{ color: #ffffff; }}
button.DangerFill:disabled {{ border-color: {SURFACE2}; background: transparent; }}
button.DangerFill:disabled label {{ color: {DISABLED}; }}
button.Nav {{ background: transparent; border: none; border-radius: 6px; padding: 8px 14px; color: {MUTED}; }}
button.Nav label {{ color: {MUTED}; }}
button.Nav:hover, button.Nav:checked {{ background: {SURFACE2}; }}
button.Nav:hover label, button.Nav:checked label {{ color: {TEXT}; }}
button.NavHead {{ background: transparent; border: none; padding: 12px 8px 4px 8px; }}
button.NavHead label {{ color: {MUTED}; font-size: 9pt; font-weight: 600; }}
button.NavHead:hover label {{ color: {TEXT}; }}
button.Seg {{ background: transparent; border: none; border-radius: 5px; padding: 6px 14px; }}
button.Seg label {{ color: {MUTED}; }}
button.Seg:hover label {{ color: {TEXT}; }}
button.Seg:checked {{ background: {RAISED}; }}
button.Seg:checked label {{ color: {TEXT}; font-weight: 600; }}
.SegBox {{ background: {SURFACE2}; border: 1px solid {BORDER}; border-radius: 7px; padding: 3px; }}
.TabBox {{ background: {SURFACE}; border: 1px solid {BORDER}; border-radius: 9px; padding: 3px; }}
.TabBox button.Seg {{ padding: 7px 22px; }}
.TabBox button.Seg label {{ font-weight: 600; }}
button.Lang {{ padding: 6px 14px; }}
button.Lang:checked {{ background: {RAISED}; border-color: {LINE}; }}
button.Lang:checked label {{ font-weight: 600; }}
button.Mod {{ background: transparent; padding: 4px 9px; border-radius: 5px; }}
button.Mod label {{ color: {MUTED}; font-size: 9pt; }}
button.Mod:checked {{ background: {RAISED}; border-color: {LINE}; }}
button.Mod:checked label {{ color: {TEXT}; font-weight: 600; }}
button.KeyBtn {{ background: {BG}; }}
button.KeyBtn label {{ font-family: {MONO}; font-weight: 600; }}
button.Tool {{ background: transparent; border: none; border-radius: 8px; padding: 4px; }}
button.Tool:hover {{ background: {SURFACE2}; }}
button.CloseBtn:hover {{ background: {DANGER}; }}
button.ResetBtn {{ background: transparent; border: none; border-radius: 7px; padding: 4px; }}
button.ResetBtn:hover {{ background: {SURFACE2}; }}
button.Swatch {{ padding: 0; min-width: 32px; min-height: 32px; border-radius: 19px; }}
button.flat {{ background: transparent; border: none; }}

.Preset {{ background: {SURFACE}; border: 1px solid {BORDER}; border-radius: 14px; }}
.Preset.selected {{ background: {SURFACE2}; border: 2px solid {a}; }}
.Preset label.name {{ font-weight: 600; padding-left: 4px; }}

entry, spinbutton {{ background: {SURFACE2}; border: 1px solid {BORDER}; border-radius: 6px; padding: 0 10px;
                     min-height: 30px; color: {TEXT}; box-shadow: none; outline: none; caret-color: {TEXT}; }}
entry:focus-within, spinbutton:focus-within {{ border-color: {a}; box-shadow: none; outline: none; }}
entry text placeholder, entry placeholder {{ color: {MUTED}; }}
entry selection, textview text selection {{ background: {a}; color: {on}; }}
.BindRow entry, .BindRow .combo {{ background: {BG}; }}

.combo {{ background: {SURFACE2}; border: 1px solid {BORDER}; border-radius: 6px; padding: 6px 10px; }}
.combo:hover {{ border-color: {LINE}; background: {SURFACE2}; }}
.combo label.muted {{ color: {MUTED}; }}
.PopupCard {{ background: {SURFACE}; border: 1px solid {BORDER}; border-radius: 10px; padding: 8px; }}
popover > contents {{ background: {SURFACE}; border: 1px solid {BORDER}; border-radius: 10px; padding: 5px;
                      box-shadow: 0 6px 18px rgba(0,0,0,0.35); color: {TEXT}; }}
popover > arrow {{ background: transparent; border: none; min-width: 0; min-height: 0; }}
popover.menu > contents {{ padding: 5px; }}
popover modelbutton {{ padding: 6px 22px 6px 12px; border-radius: 5px; color: {TEXT}; }}
popover modelbutton:hover {{ background: {a}; color: {on}; }}
.PopList, .PopList > row {{ background: transparent; }}
.PopList > row {{ border-radius: 5px; padding: 5px 8px; outline: none; }}
.PopList > row:hover {{ background: {SURFACE2}; }}
.PopList > row:selected {{ background: {a}; }}
.PopList > row:selected label {{ color: {on}; }}
.PopList > row label.muted {{ color: {MUTED}; }}
.MenuItem {{ background: transparent; border: none; border-radius: 5px; padding: 6px 22px 6px 12px; }}
.MenuItem:hover {{ background: {a}; }}
.MenuItem:hover label {{ color: {on}; }}
.MenuSep {{ background: {BORDER}; min-height: 1px; margin: 5px 8px; }}
.MenuHeader {{ color: {MUTED}; font-size: 9pt; font-weight: 600; padding: 6px 12px 2px 12px; }}

scale {{ min-height: 24px; padding: 0 8px; }}
scale trough {{ min-height: 6px; background: {SURFACE2}; border: none; border-radius: 3px; outline: none; }}
scale highlight {{ background: {a}; border: none; border-radius: 3px; min-height: 6px; }}
scale slider {{ background: {TEXT}; min-width: 16px; min-height: 16px; margin: -5px; border-radius: 8px; border: none; box-shadow: none; }}
scale slider:hover {{ background: #ffffff; }}

.ModList {{ background: {BG}; border: 1px solid {BORDER}; border-radius: 12px; padding: 6px; }}
.ModItem {{ background: {SURFACE2}; border: 1px solid {BORDER}; border-radius: 8px; padding: 6px 8px; margin: 2px 0; }}
.ModItem.sel {{ border-color: {a}; }}
.ModList.drop {{ border-color: {a}; }}
.DropMark {{ background: {a}; min-height: 2px; border-radius: 1px; }}

checkbutton {{ padding: 0; }}
checkbutton label {{ margin-left: 4px; }}
checkbutton check {{ min-width: 18px; min-height: 18px; border-radius: 6px; border: 1px solid {BORDER};
                     background: {SURFACE2}; background-image: none; box-shadow: none; -gtk-icon-source: none; }}
checkbutton check:checked {{ background: {a}; border-color: {a}; -gtk-icon-source: -gtk-icontheme("object-select-symbolic"); color: {on}; }}
checkbutton check:disabled {{ background: {BG}; }}
checkbutton.Sure label {{ color: {DANGER}; font-weight: 600; }}
checkbutton.Sure check:checked {{ background: {DANGER}; border-color: {DANGER}; }}

progressbar trough {{ background: {SURFACE2}; border: none; border-radius: 3px; min-height: 6px; }}
progressbar progress {{ background: {a}; border: none; border-radius: 3px; min-height: 6px; }}
progressbar.thin trough, progressbar.thin progress {{ min-height: 4px; }}

scrolledwindow, viewport {{ background: transparent; border: none; }}
scrollbar {{ background: transparent; border: none; }}
scrollbar slider {{ background: {BORDER}; border-radius: 4px; min-width: 8px; min-height: 30px; border: none; }}
scrollbar slider:hover {{ background: {LINE}; }}
scrollbar.horizontal slider {{ min-height: 8px; min-width: 30px; }}
scrollbar button {{ min-width: 0; min-height: 0; padding: 0; border: none; background: none; }}

.BindRow {{ background: {SURFACE2}; border: 1px solid {BORDER}; border-radius: 8px; }}
.ModList {{ background: {BG}; border: 1px solid {BORDER}; border-radius: 12px; padding: 6px; }}
.ModItem {{ background: {SURFACE2}; border: 1px solid {BORDER}; border-radius: 8px; padding: 6px 8px; margin: 2px 0; }}
.ModItem.selected {{ border-color: {a}; }}
.ModList.drop {{ border-color: {a}; }}

textview.Log, textview.Log text {{ background: #0a0e13; color: #b8c4d4; font-family: {MONO}; font-size: 9pt; }}
.LogFrame {{ background: #0a0e13; border: 1px solid {BORDER}; border-radius: 8px; padding: 8px; }}
textview.Edit, textview.Edit text {{ background: {SURFACE2}; color: {TEXT}; font-family: {MONO}; font-size: 9pt; }}
.EditFrame {{ background: {SURFACE2}; border: 1px solid {BORDER}; border-radius: 6px; padding: 8px; }}
.EditFrame:focus-within {{ border-color: {a}; }}

.Panel {{ background: {SURFACE}; border-right: 1px solid {BORDER}; border-radius: 16px 0 0 16px; }}
.Dim {{ background: rgba(5,8,12,0.43); border-radius: 16px; }}
.blurred {{ filter: blur(7px); }}
.DialogFrame {{ background: {SURFACE}; border: 1px solid {BORDER}; border-radius: 16px; }}
.Picker {{ background: {SURFACE}; border: 1px solid {BORDER}; border-radius: 16px; }}
colorchooser, colorchooser * {{ color: {TEXT}; }}
filechooser, filechooser .view, filechooser placessidebar {{ background: {BG}; color: {TEXT}; }}
filechooser .view:selected {{ background: {a}; color: {on}; }}
filechooser pathbar button {{ padding: 4px 8px; }}
columnview, listview, treeview {{ background: {BG}; color: {TEXT}; }}
columnview header button {{ background: {SURFACE2}; border: none; border-right: 1px solid {BORDER}; padding: 6px 8px; }}
.Transparent {{ background: transparent; }}
"#)
}

thread_local! {
    static PROVIDER: RefCell<Option<gtk::CssProvider>> = const { RefCell::new(None) };
    static SCALE: std::cell::Cell<f64> = const { std::cell::Cell::new(1.0) };
}

/// Oberflächengröße (Linux AIO: 100/90/80 %) – alle pt/px-Werte werden aus dem Grundtext neu gerechnet.
pub fn set_scale(f: f64) {
    SCALE.with(|s| s.set(f));
}

pub fn scale() -> f64 {
    SCALE.with(|s| s.get())
}

/// Alle pt/px-Werte eines Stylesheets skalieren (immer vom Grundtext aus; 1px-Rahmen bleiben 1px).
pub fn scale_css(text: &str, factor: f64) -> String {
    if (factor - 1.0).abs() < 0.001 {
        return text.to_string();
    }
    static RE: once_cell::sync::Lazy<regex::Regex> =
        once_cell::sync::Lazy::new(|| regex::Regex::new(r"(\d+(?:\.\d+)?)(pt|px)\b").unwrap());
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for c in RE.captures_iter(text) {
        let m = c.get(0).unwrap();
        let prev = text[..m.start()].chars().next_back();
        if prev.map(|p| p.is_alphanumeric() || p == '_' || p == '#' || p == '.' || p == '-').unwrap_or(false) {
            continue;
        }
        let v: f64 = c[1].parse().unwrap_or(0.0);
        let unit = &c[2];
        out.push_str(&text[last..m.start()]);
        if unit == "px" && v <= 1.0 {
            out.push_str(m.as_str());
        } else if unit == "pt" {
            out.push_str(&format!("{:.1}pt", v * factor));
        } else {
            out.push_str(&format!("{}px", ((v * factor).round() as i64).max(1)));
        }
        last = m.end();
    }
    out.push_str(&text[last..]);
    out
}

/// Stylesheet für die ganze App setzen (bei Akzentwechsel neu).
pub fn apply(accent: &str) {
    let Some(display) = gtk::gdk::Display::default() else { return };
    PROVIDER.with(|p| {
        let mut p = p.borrow_mut();
        if p.is_none() {
            let prov = gtk::CssProvider::new();
            gtk::style_context_add_provider_for_display(&display, &prov,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 10);
            *p = Some(prov);
        }
        p.as_ref().unwrap().load_from_string(&scale_css(&css(accent), scale()));
    });
    if let Some(s) = gtk::Settings::default() {
        s.set_gtk_application_prefer_dark_theme(true);
    }
}

/// Zusätzliches CSS (z. B. Linux AIO) mit eigener Priorität.
pub fn add_css(text: &str) -> gtk::CssProvider {
    let prov = gtk::CssProvider::new();
    prov.load_from_string(text);
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(&display, &prov, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 20);
    }
    prov
}

/// Widget-Klasse setzen (ersetzt die frühere objectName-Zuweisung).
pub fn class<W: IsA<gtk::Widget>>(w: &W, name: &str) {
    w.add_css_class(name);
}
