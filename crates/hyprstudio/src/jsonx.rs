//! JSON-Hilfen: Zustände sind (wie in der Python-Fassung) geordnete JSON-Objekte. Dazu Ausgabe im Format von
//! Pythons `json.dumps` (gleiche Dateien wie vorher) und bequeme Lesefunktionen.

use serde_json::{Map, Value};
use std::io;

pub type Obj = Map<String, Value>;

/// Python-`json.dumps(obj, ensure_ascii=False)`: Trenner ", " und ": ".
pub fn dumps(v: &Value) -> String {
    let mut out = Vec::new();
    let mut ser = serde_json::Serializer::with_formatter(&mut out, PyFormatter);
    serde::Serialize::serialize(v, &mut ser).expect("json");
    String::from_utf8(out).expect("utf8")
}

/// Python-`json.dumps(obj, indent=n, ensure_ascii=False)`.
pub fn dumps_indent(v: &Value, indent: usize) -> String {
    let ind = " ".repeat(indent);
    let mut out = Vec::new();
    let fmt = serde_json::ser::PrettyFormatter::with_indent(ind.as_bytes());
    let mut ser = serde_json::Serializer::with_formatter(&mut out, fmt);
    serde::Serialize::serialize(v, &mut ser).expect("json");
    String::from_utf8(out).expect("utf8")
}

struct PyFormatter;

impl serde_json::ser::Formatter for PyFormatter {
    fn begin_array_value<W: ?Sized + io::Write>(&mut self, w: &mut W, first: bool) -> io::Result<()> {
        if first { Ok(()) } else { w.write_all(b", ") }
    }
    fn begin_object_key<W: ?Sized + io::Write>(&mut self, w: &mut W, first: bool) -> io::Result<()> {
        if first { Ok(()) } else { w.write_all(b", ") }
    }
    fn begin_object_value<W: ?Sized + io::Write>(&mut self, w: &mut W) -> io::Result<()> {
        w.write_all(b": ")
    }
    fn write_f64<W: ?Sized + io::Write>(&mut self, w: &mut W, value: f64) -> io::Result<()> {
        w.write_all(py_float(value).as_bytes())
    }
}

/// Pythons `repr(float)`: kürzeste Darstellung, immer mit Punkt („1.0“), Exponent ab 1e16 bzw. unter 1e-4.
pub fn py_float(v: f64) -> String {
    if v.is_nan() {
        return "NaN".into();
    }
    if v.is_infinite() {
        return if v > 0.0 { "Infinity".into() } else { "-Infinity".into() };
    }
    let a = v.abs();
    if a != 0.0 && (a >= 1e16 || a < 1e-4) {
        let s = format!("{:e}", v); // 1e-5, 1.5e20
        let (m, e) = s.split_once('e').unwrap();
        let e: i32 = e.parse().unwrap();
        return format!("{}e{}{:02}", m, if e < 0 { "-" } else { "+" }, e.abs());
    }
    let s = format!("{}", v);
    if s.contains('.') { s } else { format!("{}.0", s) }
}

/// Wert eines Zustands als Text (Zahlen wie Python `str()`).
pub fn py_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => if *b { "True".into() } else { "False".into() },
        Value::Null => "None".into(),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_string()
            } else if let Some(u) = n.as_u64() {
                u.to_string()
            } else {
                py_float(n.as_f64().unwrap_or(0.0))
            }
        }
        other => dumps(other),
    }
}

/// Gleicher JSON-Typ wie in Python (`type(a) is type(b)`): bool, int, float, str, list, dict.
pub fn same_type(a: &Value, b: &Value) -> bool {
    kind(a) == kind(b)
}

pub fn kind(v: &Value) -> u8 {
    match v {
        Value::Null => 0,
        Value::Bool(_) => 1,
        Value::Number(n) => if n.is_f64() { 3 } else { 2 },
        Value::String(_) => 4,
        Value::Array(_) => 5,
        Value::Object(_) => 6,
    }
}

pub fn is_float(v: &Value) -> bool {
    matches!(v, Value::Number(n) if n.is_f64())
}

pub fn is_int(v: &Value) -> bool {
    matches!(v, Value::Number(n) if !n.is_f64())
}

pub fn f(v: f64) -> Value {
    serde_json::Number::from_f64(v).map(Value::Number).unwrap_or(Value::Null)
}

/// Bequemer Zugriff auf Objekt-Felder.
pub trait Get {
    fn val(&self, k: &str) -> &Value;
    fn s(&self, k: &str) -> &str {
        self.val(k).as_str().unwrap_or("")
    }
    fn i(&self, k: &str) -> i64 {
        let v = self.val(k);
        v.as_i64().or_else(|| v.as_f64().map(|x| x as i64)).unwrap_or(0)
    }
    fn n(&self, k: &str) -> f64 {
        let v = self.val(k);
        v.as_f64().or_else(|| v.as_bool().map(|b| b as i64 as f64)).unwrap_or(0.0)
    }
    fn b(&self, k: &str) -> bool {
        truthy(self.val(k))
    }
    fn arr(&self, k: &str) -> &[Value] {
        self.val(k).as_array().map(|a| a.as_slice()).unwrap_or(&[])
    }
    fn strs(&self, k: &str) -> Vec<String> {
        self.arr(k).iter().filter_map(|x| x.as_str().map(String::from)).collect()
    }
    fn obj(&self, k: &str) -> Option<&Obj> {
        self.val(k).as_object()
    }
}

static NULL: Value = Value::Null;

impl Get for Obj {
    fn val(&self, k: &str) -> &Value {
        self.get(k).unwrap_or(&NULL)
    }
}

impl Get for Value {
    fn val(&self, k: &str) -> &Value {
        self.get(k).unwrap_or(&NULL)
    }
}

/// Python-Wahrheitswert.
pub fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|x| x != 0.0).unwrap_or(false),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

pub fn strs(v: &[&str]) -> Value {
    Value::Array(v.iter().map(|s| Value::String(s.to_string())).collect())
}

pub fn obj(v: Value) -> Obj {
    match v {
        Value::Object(o) => o,
        _ => Obj::new(),
    }
}

/// Liest JSON aus eingebetteten Daten.
pub fn parse(text: &str) -> Value {
    serde_json::from_str(text).expect("eingebettete Daten")
}

/// Wie Python `json.loads(json.dumps(x))`: tiefe Kopie.
pub fn copy(v: &Value) -> Value {
    v.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn python_like_dumps() {
        let v = json!({"a": 1, "b": [1.0, "x"], "c": {"d": true}});
        assert_eq!(dumps(&v), r#"{"a": 1, "b": [1.0, "x"], "c": {"d": true}}"#);
        assert_eq!(py_float(0.0117), "0.0117");
        assert_eq!(py_float(1e-5), "1e-05");
        assert_eq!(py_float(2.0), "2.0");
    }
}
