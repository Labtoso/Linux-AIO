//! `json.dumps` genau wie Python (Einrückung, sort_keys, ensure_ascii), damit Zustandsdateien gleich bleiben.

use hyprstudio::jsonx::py_float;
use serde_json::Value;

pub fn dumps(v: &Value, indent: Option<usize>, sort: bool, ascii: bool) -> String {
    let mut out = String::new();
    write(&mut out, v, indent, sort, ascii, 0);
    out
}

fn string(out: &mut String, s: &str, ascii: bool) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c if ascii && (c as u32) > 0x7e => {
                let mut buf = [0u16; 2];
                for u in c.encode_utf16(&mut buf) {
                    out.push_str(&format!("\\u{:04x}", u));
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn write(out: &mut String, v: &Value, indent: Option<usize>, sort: bool, ascii: bool, level: usize) {
    let nl = |out: &mut String, l: usize| {
        if let Some(n) = indent {
            out.push('\n');
            out.push_str(&" ".repeat(n * l));
        }
    };
    let sep = if indent.is_some() { "," } else { ", " };
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => {
            if n.is_f64() {
                out.push_str(&py_float(n.as_f64().unwrap()));
            } else {
                out.push_str(&n.to_string());
            }
        }
        Value::String(s) => string(out, s, ascii),
        Value::Array(a) => {
            if a.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push_str(sep);
                }
                nl(out, level + 1);
                write(out, x, indent, sort, ascii, level + 1);
            }
            nl(out, level);
            out.push(']');
        }
        Value::Object(m) => {
            if m.is_empty() {
                out.push_str("{}");
                return;
            }
            let mut keys: Vec<&String> = m.keys().collect();
            if sort {
                keys.sort();
            }
            out.push('{');
            for (i, k) in keys.into_iter().enumerate() {
                if i > 0 {
                    out.push_str(sep);
                }
                nl(out, level + 1);
                string(out, k, ascii);
                out.push_str(": ");
                write(out, &m[k], indent, sort, ascii, level + 1);
            }
            nl(out, level);
            out.push('}');
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn same_as_python() {
        let v: serde_json::Value = serde_json::from_str(r#"{"b": [1, 2.5, "ä"], "a": {}, "c": []}"#).unwrap();
        assert_eq!(super::dumps(&v, Some(2), true, true),
                   "{\n  \"a\": {},\n  \"b\": [\n    1,\n    2.5,\n    \"\\u00e4\"\n  ],\n  \"c\": []\n}");
        assert_eq!(super::dumps(&v, None, false, false), "{\"b\": [1, 2.5, \"ä\"], \"a\": {}, \"c\": []}");
    }
}
