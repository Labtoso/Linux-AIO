use hyprstudio::jsonx::Get;
use hyprstudio::{hyprlock, kitty, launcher, model, paper, perf};
use std::path::Path;

fn first_diff(a: &str, b: &str) -> String {
    for (i, (x, y)) in a.lines().zip(b.lines()).enumerate() {
        if x != y {
            return format!("line {}:\n  rust: {}\n  py:   {}", i + 1, x, y);
        }
    }
    format!("lengths differ {} vs {}", a.len(), b.len())
}

#[test]
fn ext_match_python() {
    model::set_dry(true);
    let data: serde_json::Value = serde_json::from_str(include_str!("golden/ext.json")).unwrap();
    let bad = std::cell::RefCell::new(Vec::<String>::new());
    let cmp = |what: String, a: &str, b: &str| {
        if a != b {
            bad.borrow_mut().push(format!("{}: {}", what, first_diff(a, b)));
        }
    };
    for c in data.arr("kitty") {
        let s = kitty::state(c.obj("state"));
        let start = c.get("start").and_then(|v| v.as_str()).map(Path::new);
        cmp(format!("kitty {}", c.s("name")), &kitty::conf(&s, start), c.s("conf"));
        if let Some(sc) = c.get("script").and_then(|v| v.as_str()) {
            cmp("kitty script".into(), &kitty::start_script(&s), sc);
        }
    }
    let h = Path::new("/H/.config");
    for c in data.arr("launcher") {
        let s = launcher::state(c.obj("state"));
        let files = launcher::files(&s, h, None);
        let py = c.arr("files");
        if files.len() != py.len() {
            bad.borrow_mut().push(format!("launcher {}: {} files vs {}", c.s("name"), files.len(), py.len()));
            continue;
        }
        for ((p, txt), f) in files.iter().zip(py) {
            let pp = f[0].as_str().unwrap();
            if p.to_str().unwrap() != pp {
                bad.borrow_mut().push(format!("launcher {}: path {} vs {}", c.s("name"), p.display(), pp));
            }
            cmp(format!("launcher {} {}", c.s("name"), pp), txt, f[1].as_str().unwrap());
        }
    }
    for c in data.arr("lock") {
        let s = hyprlock::lock_state(c.obj("state"));
        let wp = if c.s("name") == "x1" { "" } else { "/W/pic.jpg" };
        cmp(format!("lock {}", c.s("name")), &hyprlock::lock_conf(&s, wp), c.s("conf"));
    }
    for c in data.arr("paper") {
        let s = paper::paper_state(c.obj("state"));
        cmp("paper".into(), &paper::paper_conf(&s), c.s("conf"));
    }
    for c in data.arr("perf") {
        let s = perf::perf_state(c.obj("state"));
        cmp(format!("perf {}", c.s("fmt")), &perf::content(&s, c.s("fmt")), c.s("out"));
    }
    let bad = bad.into_inner();
    assert!(bad.is_empty(), "{} Abweichungen:\n{}", bad.len(), bad.iter().take(15).cloned().collect::<Vec<_>>().join("\n"));
}
