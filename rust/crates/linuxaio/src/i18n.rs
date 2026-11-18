//! Texte Deutsch/Englisch (gleiche Schlüssel und Texte wie die Python-Fassung, aus `data/i18n.json`).
//! Die Sprache gilt auch für das eingebaute Hypr Studio.

use hyprstudio::i18n::{format, Table};
use once_cell::sync::Lazy;

pub static TEXT: Lazy<Table> = Lazy::new(|| Table::from_json(include_str!("../data/i18n.json"), "de", "de"));

pub fn set_lang(lang: &str) {
    TEXT.set_lang(lang);
    hyprstudio::i18n::set_lang(&TEXT.lang());
}

pub fn lang() -> String {
    TEXT.lang()
}

pub fn t(key: &str) -> String {
    TEXT.t(key)
}

/// `t(key, name=value, …)`
pub fn tf(key: &str, kw: &[(&str, String)]) -> String {
    format(&TEXT.t(key), kw)
}
