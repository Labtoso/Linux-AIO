//! Nachbau von Pythons `configparser.ConfigParser(interpolation=None, strict=False)` mit `optionxform = str`:
//! gleiche Lese-Regeln (Kommentare, Fortsetzungszeilen, `=` und `:`) und gleiche Ausgabe beim Schreiben.

use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct Ini {
    pub defaults: Vec<(String, String)>,
    pub sections: Vec<(String, Vec<(String, String)>)>,
    /// Schlüssel klein schreiben (Standard von configparser); `false` = `optionxform = str`
    pub lower_keys: bool,
}

impl Ini {
    pub fn new() -> Ini {
        Ini::default()
    }

    /// Datei lesen; Fehler werden wie in der Python-Fassung verschluckt.
    pub fn read(path: &Path) -> Ini {
        let mut ini = Ini::new();
        ini.read_file(path);
        ini
    }

    pub fn read_file(&mut self, path: &Path) -> bool {
        match std::fs::read(path) {
            Ok(b) => match String::from_utf8(b) {
                Ok(s) => {
                    self.read_str(&s);
                    true
                }
                Err(_) => false,
            },
            Err(_) => false,
        }
    }

    fn key(&self, k: &str) -> String {
        if self.lower_keys { k.to_lowercase() } else { k.to_string() }
    }

    pub fn read_str(&mut self, text: &str) {
        // (Abschnitt, Schlüssel, Einrückung) der letzten Option für Fortsetzungszeilen
        let mut cur: Option<String> = None;
        let mut last: Option<(String, usize)> = None;
        let mut pending_blank = 0usize;
        for raw in text.split('\n') {
            let raw = raw.strip_suffix('\r').unwrap_or(raw);
            let stripped = raw.trim();
            if stripped.starts_with('#') || stripped.starts_with(';') {
                continue;
            }
            if stripped.is_empty() {
                if last.is_some() {
                    pending_blank += 1;
                }
                continue;
            }
            let indent = raw.len() - raw.trim_start().len();
            if let (Some(sec), Some((opt, ind))) = (cur.clone(), last.clone()) {
                if indent > ind {
                    let entry = if sec == "DEFAULT" { self.defaults.iter_mut().find(|(k, _)| *k == opt) } else {
                        self.sections.iter_mut().find(|(n, _)| *n == sec)
                            .and_then(|(_, v)| v.iter_mut().find(|(k, _)| *k == opt))
                    };
                    if let Some((_, v)) = entry {
                        for _ in 0..pending_blank {
                            v.push('\n');
                        }
                        v.push('\n');
                        v.push_str(stripped);
                    }
                    pending_blank = 0;
                    continue;
                }
            }
            pending_blank = 0;
            if stripped.starts_with('[') {
                if let Some(end) = stripped.rfind(']') {
                    if end > 1 {
                        let name = stripped[1..end].to_string();
                        if name != "DEFAULT" && !self.has_section(&name) {
                            self.sections.push((name.clone(), Vec::new()));
                        }
                        cur = Some(name);
                        last = None;
                        continue;
                    }
                }
            }
            let Some(sec) = cur.clone() else {
                return; // MissingSectionHeaderError: Rest der Datei wird nicht gelesen
            };
            let pos = stripped.find(['=', ':']);
            let Some(i) = pos else {
                last = None; // ParsingError: Zeile wird übersprungen
                continue;
            };
            let k = stripped[..i].trim_end();
            if k.is_empty() {
                last = None;
                continue;
            }
            let k = self.key(k);
            let v = stripped[i + 1..].trim().to_string();
            if sec == "DEFAULT" {
                match self.defaults.iter_mut().find(|(a, _)| *a == k) {
                    Some(e) => e.1 = v,
                    None => self.defaults.push((k.clone(), v)),
                }
            } else {
                self.set(&sec, &k, &v);
            }
            last = Some((k, indent));
        }
    }

    pub fn has_section(&self, s: &str) -> bool {
        self.sections.iter().any(|(n, _)| n == s)
    }

    pub fn section_names(&self) -> Vec<String> {
        self.sections.iter().map(|(n, _)| n.clone()).collect()
    }

    pub fn add_section(&mut self, s: &str) {
        if !self.has_section(s) {
            self.sections.push((s.to_string(), Vec::new()));
        }
    }

    pub fn get(&self, section: &str, key: &str) -> Option<String> {
        let k = self.key(key);
        let sec = self.sections.iter().find(|(n, _)| n == section)?;
        sec.1.iter().find(|(a, _)| *a == k).map(|(_, v)| v.clone())
            .or_else(|| self.defaults.iter().find(|(a, _)| *a == k).map(|(_, v)| v.clone()))
    }

    /// `cp.get(section, key, fallback=default)`
    pub fn get_or(&self, section: &str, key: &str, default: &str) -> String {
        self.get(section, key).unwrap_or_else(|| default.to_string())
    }

    pub fn has_option(&self, section: &str, key: &str) -> bool {
        self.get(section, key).is_some()
    }

    pub fn items(&self, section: &str) -> Vec<(String, String)> {
        self.sections.iter().find(|(n, _)| n == section).map(|(_, v)| v.clone()).unwrap_or_default()
    }

    pub fn set(&mut self, section: &str, key: &str, value: &str) {
        self.add_section(section);
        let k = self.key(key);
        let sec = self.sections.iter_mut().find(|(n, _)| n == section).unwrap();
        match sec.1.iter_mut().find(|(a, _)| *a == k) {
            Some(e) => e.1 = value.to_string(),
            None => sec.1.push((k, value.to_string())),
        }
    }

    pub fn remove_option(&mut self, section: &str, key: &str) -> bool {
        let k = self.key(key);
        if let Some(sec) = self.sections.iter_mut().find(|(n, _)| n == section) {
            let n = sec.1.len();
            sec.1.retain(|(a, _)| *a != k);
            return sec.1.len() != n;
        }
        false
    }

    /// `cp.write(f, space_around_delimiters=…)`
    pub fn write_string(&self, spaces: bool) -> String {
        let d = if spaces { " = " } else { "=" };
        let mut out = String::new();
        let mut put = |name: &str, items: &[(String, String)]| {
            out.push_str(&format!("[{}]\n", name));
            for (k, v) in items {
                out.push_str(&format!("{}{}{}\n", k, d, v.replace('\n', "\n\t")));
            }
            out.push('\n');
        };
        if !self.defaults.is_empty() {
            put("DEFAULT", &self.defaults);
        }
        for (n, items) in &self.sections {
            put(n, items);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let mut i = Ini::new();
        i.read_str("# x\n[Theme]\nCurrent = breeze\nFoo: bar\n  more\n[General]\nNumlock=on\n[Theme]\nCursor=x\n");
        assert_eq!(i.get("Theme", "Current").as_deref(), Some("breeze"));
        assert_eq!(i.get("Theme", "Foo").as_deref(), Some("bar\nmore"));
        i.set("Theme", "Current", "abc");
        assert_eq!(i.write_string(false),
            "[Theme]\nCurrent=abc\nFoo=bar\n\tmore\nCursor=x\n\n[General]\nNumlock=on\n\n");
        let mut j = Ini::new();
        j.read_str("a=1\n[X]\nb=2\n");
        assert!(j.sections.is_empty());
    }
}
