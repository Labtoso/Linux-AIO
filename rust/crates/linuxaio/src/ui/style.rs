//! Linux-AIO-eigene Klassen zusätzlich zum Hypr-Studio-Design (gleiche Farben und Maße wie das frühere
//! Qt-Stylesheet; Klassen mit „A“ davor, damit das eingebettete Hypr Studio unverändert bleibt).

use hyprstudio::ui::theme::{self, BG, BORDER, HOVER, LINE, MONO, MUTED, OK, RAISED, SURFACE, SURFACE2, TEXT};
use std::cell::RefCell;

pub const DANGER: &str = theme::DANGER;
pub const WARN: &str = "#d29922";
pub const DISABLED: &str = "#4d5a6b";

pub fn css(accent: &str) -> String {
    let a = accent;
    let _ = (BG, HOVER);
    format!(r#"
.ATabBox {{ background: {SURFACE}; border: 1px solid {BORDER}; border-radius: 12px; padding: 3px; }}
.ATabBox button.Seg {{ padding: 7px 22px; border-radius: 6px; }}
.ATabBox button.Seg label {{ font-weight: 600; }}
.SegBox {{ border-radius: 8px; }}
button.Seg {{ border-radius: 6px; }}
button.Seg:disabled label {{ color: #3c4757; }}
.AHero {{ background: {SURFACE}; border: 1px solid {BORDER}; border-radius: 16px; }}
.DeName {{ font-size: 22pt; font-weight: 700; }}
.AChip {{ background: {SURFACE2}; border: 1px solid {BORDER}; border-radius: 9px; padding: 2px 9px; color: {MUTED}; font-size: 9pt; }}
.AChipAccent {{ background: transparent; border: 1px solid {a}; border-radius: 9px; padding: 2px 9px; color: {a}; font-size: 9pt; }}
.AChipOk {{ background: transparent; border: 1px solid {OK}; border-radius: 9px; padding: 2px 9px; color: {OK}; font-size: 9pt; }}
.AChipWarn {{ background: transparent; border: 1px solid {WARN}; border-radius: 9px; padding: 2px 9px; color: {WARN}; font-size: 9pt; }}
entry.Search {{ background: {SURFACE2}; border: 1px solid {BORDER}; border-radius: 6px; padding: 0 10px; min-height: 32px; }}
entry.Search:focus-within {{ border-color: {a}; }}
entry.Search image {{ color: {MUTED}; }}
.DmBar {{ background: {SURFACE}; border: 1px solid {BORDER}; border-radius: 12px; }}
button.applied, button.applied:disabled {{ background: transparent; border: 1px solid {OK}; }}
button.applied label, button.applied:disabled label {{ color: {OK}; font-weight: 600; }}
.LogDrawer {{ background: #0a0e13; border: 1px solid {LINE}; border-radius: 12px; padding: 10px; }}
textview.ALog, textview.ALog text {{ background: #0a0e13; color: #b8c4d4; font-family: "Hack", "JetBrains Mono", "DejaVu Sans Mono", monospace; font-size: 9pt; }}
.SideBox {{ background: {SURFACE}; border: 1px solid {BORDER}; border-radius: 12px; }}
list.DeList {{ background: transparent; }}
list.NavList {{ background: {SURFACE}; border: 1px solid {BORDER}; border-radius: 12px; padding: 8px; }}
list.DeList > row, list.NavList > row {{ border-radius: 8px; padding: 4px 10px; color: {TEXT}; outline: none; background: transparent; }}
list.DeList > row:hover, list.NavList > row:hover {{ background: {SURFACE2}; }}
list.DeList > row:disabled:hover {{ background: transparent; }}
list.DeList > row:selected, list.NavList > row:selected {{ background: {RAISED}; }}
list.DeList > row:selected label, list.NavList > row:selected label {{ font-weight: 600; color: {TEXT}; }}
list.DeList > row:disabled label {{ color: {DISABLED}; }}
.APageTitle {{ font-size: 15pt; font-weight: 700; }}
.ACard {{ background: {SURFACE}; border: 1px solid {BORDER}; border-radius: 12px; }}
.ACardTitle {{ color: {MUTED}; font-size: 9pt; font-weight: 600; padding-bottom: 2px; }}
.AMono {{ color: {MUTED}; font-family: "JetBrains Mono", "Hack", monospace; font-size: 9pt; }}
.ALabel {{ color: {MUTED}; font-size: 9pt; }}
.ATile {{ background: {SURFACE}; border: 1px solid {BORDER}; border-radius: 14px; }}
.ATile.sel {{ background: {SURFACE2}; border: 2px solid {a}; }}
.ATile.act {{ background: {SURFACE}; border: 2px solid {a}; }}
.ATile label.name {{ font-weight: 600; }}
.Dot {{ border-radius: 4px; min-width: 8px; min-height: 8px; }}
.ImageBox {{ background: {SURFACE2}; border: 1px solid {BORDER}; border-radius: 10px; color: {MUTED}; }}
.Sample {{ background: {SURFACE2}; border-radius: 8px; padding: 6px 8px; }}
button.Missing {{ background: {SURFACE2}; border: 1px solid {BORDER}; border-radius: 8px; padding: 4px; min-width: 30px; min-height: 30px; }}
button.Missing:hover {{ border-color: {WARN}; }}
button.UseBtn {{ background: {SURFACE2}; border: 1px solid {BORDER}; border-radius: 8px; padding: 4px 10px; }}
button.Small {{ padding: 4px 8px; }}
.WarnText {{ color: {WARN}; }}
.Flash {{ color: {TEXT}; background: {RAISED}; border-radius: 6px; }}
.SearchPop > row {{ padding: 6px 10px; border-radius: 6px; }}
.SearchPop > row:selected {{ background: {RAISED}; }}
.SearchPop > row label.where {{ color: {MUTED}; }}
.AMonoPath {{ color: {MUTED}; font-family: {MONO}; font-size: 9pt; }}
"#)
}

thread_local! {
    static PROVIDER: RefCell<Option<gtk::CssProvider>> = const { RefCell::new(None) };
}

/// Hypr-Studio-Design + Linux-AIO-Klassen, skaliert mit der Oberflächengröße.
pub fn apply(accent: &str, factor: f64) {
    theme::set_scale(factor);
    theme::apply(accent);
    let text = theme::scale_css(&css(accent), factor);
    PROVIDER.with(|p| {
        let mut p = p.borrow_mut();
        match p.as_ref() {
            Some(prov) => prov.load_from_string(&text),
            None => *p = Some(theme::add_css(&text)),
        }
    });
}
