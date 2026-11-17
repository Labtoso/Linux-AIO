"""Golden files: outputs of the former Python model for many states (Rust tests compare against them)."""
import json, os, sys, copy
from pathlib import Path
ROOT = Path(__file__).resolve().parent.parent.parent
RUST = ROOT / "rust"
sys.path.insert(0, str(ROOT / "src"))
from hyprstudio import model
OUT = RUST / "crates" / "hyprstudio" / "tests" / "golden"
cases = []
for name in model.HYPR_PRESETS:
    s = model.preset_state(name)
    cases.append(("p_" + name, s))
s = model.preset_state("neon")
s.update({"smart_gaps": True, "workspaces_on": True, "gesture": 3, "cursor_theme": "Bibata", "qt_theme": "kde",
          "monitor_on": True, "monitor_scale": "1.25", "autostart": ["waybar", "nm-applet", "hyprpolkitagent"],
          "autostart_waybar": True, "polkit_agent": True, "polkit_managed": True, "laptop_keys": True,
          "binds": model.default_binds() + model.TEMPLATES["media"] + [model._bind(["SUPER"], "SUPER_L", "launcher", "rofi")],
          "skip_opts": ["decoration.blur.xray", "general.col.active_border", "misc.background_color"],
          "anim_windows": False, "anim_tempo": 1.7, "accel": "flat"})
ws = model.default_workspaces()
ws[0].update({"name": "web", "monitor": "DP-1", "gaps_in": 3, "persistent": True, "layout": "master"})
ws[3].update({"no_rounding": True, "on_empty": "kitty", "border": 0, "default": True})
s["workspaces"] = ws
cases.append(("full", s))
s2 = copy.deepcopy(s); s2["anim"] = False; s2["gradient"] = False; s2["no_logo"] = False
cases.append(("full2", s2))
wb = []
for name in model.WAYBAR_PRESETS:
    wb.append(("w_" + name, model.waybar_preset(name)))
w = model.waybar_preset("full"); w.update({"icons": False, "tooltips": False, "clock_fmt": "hms", "ws_format": "dot", "ws_persist": 5, "net_fmt": "ip", "clock24": False})
wb.append(("w_full2", w))
w = model.waybar_preset("rainbow"); w.update({"ws_format": "dot", "net_fmt": "signal"})
wb.append(("w_rainbow2", w))
data = {"hypr": [{"name": n, "state": s, "lua": model.hypr_lua(s), "conf": model.hypr_conf(s)} for n, s in cases],
        "waybar": [{"name": n, "state": s, "json": model.waybar_json(s), "css": model.waybar_css(s)} for n, s in wb]}
lua_text = open(ROOT / "tools" / "sample_hyprland.lua").read() if (ROOT / "tools" / "sample_hyprland.lua").exists() else ""
(OUT / "model.json").write_text(json.dumps(data, ensure_ascii=False, indent=1))
print(len(cases), len(wb))

# ---- kitty, launcher, hyprlock, paper, perf ----
from hyprstudio import kitty, launcher, hyprlock, paper, perf
model.DRY = True
H = Path("/H/.config")
ext = {"kitty": [], "launcher": [], "lock": [], "paper": [], "perf": []}
for name in kitty.PRESETS:
    k = kitty.preset(name)
    ext["kitty"].append({"name": name, "state": k, "conf": kitty.conf(k)})
k = kitty.preset("tokyo"); k.update({"fastfetch": True, "start_cmds": ["echo hi"], "opacity": 0.85})
ext["kitty"].append({"name": "start", "state": k, "conf": kitty.conf(k, H / "kitty" / kitty.START),
                     "script": kitty.start_script(k), "start": str(H / "kitty" / kitty.START)})
for name in launcher.PRESETS:
    for app in launcher.APPS:
        s = launcher.preset(name); s["app"] = app
        ext["launcher"].append({"name": f"{name}/{app}", "state": s,
                                "files": [[str(p), c] for p, c in launcher.files(s, H)]})
for name in hyprlock.PRESETS:
    s = hyprlock.preset_state(name)
    ext["lock"].append({"name": name, "state": s, "conf": hyprlock.lock_conf(s, "/W/pic.jpg")})
s = hyprlock.preset_state("sidebar"); s.update({"avatar": True, "greet_text": "Hallo\nWelt", "clock_fmt": "12"})
ext["lock"].append({"name": "x1", "state": s, "conf": hyprlock.lock_conf(s, "")})
for st in [{"path": "~/a.jpg"}, {"path": "~/Bilder", "random": True, "recursive": True, "timeout": 60, "fit": "tile"},
           {"path": "", "monitors": {"HDMI-A-1": {"path": "/x/b.png", "fit": "bad"}, "DP-1": {"path": "/dir"}}, "splash": True}]:
    s = paper.paper_state(st)
    ext["paper"].append({"state": s, "conf": paper.paper_conf(s)})
for pe in [{"portal_env": False, "light": False}, {"portal_env": True, "light": True}, {"portal_env": True, "light": False}]:
    for fmt in ("lua", "conf"):
        ext["perf"].append({"state": pe, "fmt": fmt, "out": perf.content(pe, fmt)})
(OUT / "ext.json").write_text(json.dumps(ext, ensure_ascii=False, indent=1))
print({k: len(v) for k, v in ext.items()})
