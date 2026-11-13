//! Texte Deutsch/Englisch (gleiche Schlüssel und Texte wie die Python-Fassung, aus `data/i18n.json`).

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::RwLock;

/// Sprachtabelle: Sprache → Schlüssel → Text. Andere Teile (Linux AIO) dürfen Texte ergänzen.
pub struct Table {
    pub text: RwLock<HashMap<String, HashMap<String, String>>>,
    pub lang: RwLock<String>,
    pub fallback: &'static str,
}

impl Table {
    pub fn from_json(src: &str, start: &str, fallback: &'static str) -> Table {
        let v: serde_json::Value = serde_json::from_str(src).expect("i18n");
        let mut text = HashMap::new();
        if let Some(o) = v.as_object() {
            for (lang, tbl) in o {
                let mut m = HashMap::new();
                if let Some(t) = tbl.as_object() {
                    for (k, s) in t {
                        if let Some(s) = s.as_str() {
                            m.insert(k.clone(), s.to_string());
                        }
                    }
                }
                text.insert(lang.clone(), m);
            }
        }
        Table { text: RwLock::new(text), lang: RwLock::new(start.into()), fallback }
    }

    pub fn set_lang(&self, lang: &str) {
        let ok = self.text.read().unwrap().contains_key(lang);
        *self.lang.write().unwrap() = if ok { lang.into() } else { "de".into() };
    }

    pub fn lang(&self) -> String {
        self.lang.read().unwrap().clone()
    }

    /// `TEXT[lang].get(key) or TEXT["de"].get(key) or key`
    pub fn t(&self, key: &str) -> String {
        let lang = self.lang();
        let text = self.text.read().unwrap();
        for l in [lang.as_str(), self.fallback] {
            if let Some(s) = text.get(l).and_then(|m| m.get(key)) {
                if !s.is_empty() {
                    return s.clone();
                }
            }
        }
        key.to_string()
    }

    pub fn has(&self, lang: &str, key: &str) -> bool {
        self.text.read().unwrap().get(lang).map(|m| m.contains_key(key)).unwrap_or(false)
    }

    pub fn update(&self, lang: &str, items: &[(&str, &str)]) {
        let mut text = self.text.write().unwrap();
        let m = text.entry(lang.to_string()).or_default();
        for (k, v) in items {
            m.insert(k.to_string(), v.to_string());
        }
    }

    pub fn update_owned(&self, lang: &str, items: Vec<(String, String)>) {
        let mut text = self.text.write().unwrap();
        let m = text.entry(lang.to_string()).or_default();
        for (k, v) in items {
            m.insert(k, v);
        }
    }

    /// Alle Texte der aktuellen Sprache (für die Suche).
    pub fn all(&self) -> Vec<(String, String)> {
        let lang = self.lang();
        let text = self.text.read().unwrap();
        let mut v: Vec<_> = text.get(&lang).map(|m| m.iter().map(|(a, b)| (a.clone(), b.clone())).collect())
            .unwrap_or_default();
        v.sort();
        v
    }
}

/// Python-`str.format(**kw)` für einfache Platzhalter `{name}`.
pub fn format(s: &str, kw: &[(&str, String)]) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    let mut rest = s;
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        if let Some(stripped) = after.strip_prefix('{') {
            out.push('{');
            rest = stripped;
            continue;
        }
        match after.find('}') {
            Some(j) => {
                let name = &after[..j];
                match kw.iter().find(|(k, _)| *k == name) {
                    Some((_, v)) => out.push_str(v),
                    None => {
                        out.push('{');
                        out.push_str(name);
                        out.push('}');
                    }
                }
                rest = &after[j + 1..];
            }
            None => {
                out.push_str(&rest[i..]);
                rest = "";
            }
        }
    }
    out.push_str(&rest.replace("}}", "}"));
    out
}

pub static TEXT: Lazy<Table> = Lazy::new(|| Table::from_json(include_str!("../data/i18n.json"), "en", "de"));

pub fn set_lang(lang: &str) {
    TEXT.set_lang(lang)
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

pub fn has(key: &str) -> bool {
    TEXT.has("de", key)
}

/// Kurzform für Platzhalter: `kw!("n" => 3, "name" => x)`.
#[macro_export]
macro_rules! kw {
    ($($k:literal => $v:expr),* $(,)?) => { &[$(($k, ($v).to_string())),*] };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_works() {
        assert_eq!(format("{n} aktiv", &[("n", "3".into())]), "3 aktiv");
        assert_eq!(format("a {x} b", &[]), "a {x} b");
    }

    #[test]
    fn texts_loaded() {
        assert_eq!(TEXT.text.read().unwrap().get("de").unwrap().get("apply").unwrap(), "Anwenden");
    }
}
