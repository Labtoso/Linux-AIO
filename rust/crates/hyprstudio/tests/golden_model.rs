use hyprstudio::jsonx::Get;
use hyprstudio::model;

fn first_diff(a: &str, b: &str) -> String {
    for (i, (x, y)) in a.lines().zip(b.lines()).enumerate() {
        if x != y {
            return format!("line {}:\n  rust: {}\n  py:   {}", i + 1, x, y);
        }
    }
    format!("lengths differ {} vs {}", a.len(), b.len())
}

#[test]
fn hypr_and_waybar_match_python() {
    let data: serde_json::Value = serde_json::from_str(include_str!("golden/model.json")).unwrap();
    let mut bad = vec![];
    for c in data.arr("hypr") {
        let s = model::hypr_state(c.obj("state"));
        let lua = model::hypr_lua(&s);
        if lua != c.s("lua") { bad.push(format!("{} lua: {}", c.s("name"), first_diff(&lua, c.s("lua")))); }
        let conf = model::hypr_conf(&s);
        if conf != c.s("conf") { bad.push(format!("{} conf: {}", c.s("name"), first_diff(&conf, c.s("conf")))); }
    }
    for c in data.arr("waybar") {
        let s = model::waybar_state(c.obj("state"));
        let j = model::waybar_json(&s);
        if j != c.s("json") { bad.push(format!("{} json: {}", c.s("name"), first_diff(&j, c.s("json")))); }
        let css = model::waybar_css(&s);
        if css != c.s("css") { bad.push(format!("{} css: {}", c.s("name"), first_diff(&css, c.s("css")))); }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}
