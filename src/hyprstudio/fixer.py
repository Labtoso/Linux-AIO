"""Config-Fehler finden und beheben: liest `hyprctl configerrors`, ordnet jede Meldung einer Datei-Zeile zu
und repariert sie. Fehler in Hypr Studios eigener Datei werden über den Zustand behoben (Option weglassen
bzw. Funktion abschalten), damit der Fix beim nächsten Anwenden nicht wieder verschwindet.
Fehler in fremden Dateien werden mit Sicherung auskommentiert. Ohne Qt."""

import json
import re
import shutil
import subprocess
import time
from pathlib import Path

from . import model

TESTFILE = "configerrors.txt"  # Testmodus: Fehlermeldungen kommen aus dieser Datei


def read_errors(hdir: Path, log) -> list:
    """Rohe Fehlermeldungen (eine pro Eintrag)."""
    if model.DRY:
        f = hdir.parent / TESTFILE
        if not f.is_file():
            return []
        return [ln for ln in f.read_text(encoding="utf-8").splitlines() if ln.strip()]
    try:
        p = subprocess.run(["hyprctl", "configerrors", "-j"], capture_output=True, text=True, timeout=8)
    except (OSError, subprocess.TimeoutExpired) as e:
        log(f"hyprctl: {e}")
        return []
    out = p.stdout.strip()
    try:
        data = json.loads(out)
        items = data if isinstance(data, list) else [str(data)]
    except ValueError:
        items = out.splitlines()
    lines = []
    for it in items:
        lines += [x for x in str(it).splitlines() if x.strip()]
    return [x for x in lines if x.strip().lower() not in ("", "no errors", "ok")]


PATTERNS = (
    re.compile(r"(?:config error in )?file\s+(?P<file>\S+?)\s+at line\s+(?P<line>\d+):?\s*(?P<msg>.*)", re.I),
    re.compile(r"(?P<file>[A-Za-z]:[/\\]\S+?\.(?:lua|conf)):(?P<line>\d+):\s*(?P<msg>.*)"),  # Testmodus (Windows)
    re.compile(r"(?P<file>/\S+?\.(?:lua|conf)):(?P<line>\d+):\s*(?P<msg>.*)"),
)


def parse(raw: list) -> list:
    out = []
    for r in raw:
        e = {"raw": r, "file": "", "line": 0, "msg": r.strip()}
        for pat in PATTERNS:
            m = pat.search(r)
            if m:
                e.update(file=m.group("file").strip("'\""), line=int(m.group("line")), msg=m.group("msg").strip() or r)
                break
        out.append(e)
    return out


def line_text(e: dict) -> str:
    try:
        lines = Path(e["file"]).read_text(encoding="utf-8", errors="replace").splitlines()
        return lines[e["line"] - 1].strip() if 0 < e["line"] <= len(lines) else ""
    except OSError:
        return ""


def is_ours(e: dict) -> bool:
    return Path(e["file"]).name.startswith("hyprstudio.")


def option_path(text: str, line: int) -> str | None:
    """Zu einer Zeile in hyprstudio.lua/.conf den Options-Pfad bestimmen, z. B. decoration.blur.xray."""
    lines = text.splitlines()
    if not 0 < line <= len(lines):
        return None
    m = re.match(r"\s*([\w.\-]+)\s*=", lines[line - 1])
    if not m or lines[line - 1].rstrip().endswith("{"):
        return None
    stack = []
    for ln in lines[:line - 1]:
        s = ln.strip()
        o = re.match(r"([\w.]+)\s*(?:=\s*)?\{$", s)
        if o and o.group(1) not in ("hl.config(",):
            stack.append(o.group(1))
        elif s in ("}", "},", "})"):
            if stack:
                stack.pop()
        elif s == "hl.config({":
            stack = []
    return ".".join(stack + [m.group(1)]) if stack else None


FEATURES = (  # Zeilenanfang in der eigenen Datei → Funktion, die abgeschaltet wird
    (('hl.workspace_rule({ workspace = "1', 'hl.workspace_rule({ workspace = "2', 'hl.workspace_rule({ workspace = "3',
      'hl.workspace_rule({ workspace = "4', 'hl.workspace_rule({ workspace = "5', 'hl.workspace_rule({ workspace = "6',
      'hl.workspace_rule({ workspace = "7', 'hl.workspace_rule({ workspace = "8', 'hl.workspace_rule({ workspace = "9',
      "workspace = 1,", "workspace = 2,", "workspace = 3,", "workspace = 4,", "workspace = 5,", "workspace = 6,",
      "workspace = 7,", "workspace = 8,", "workspace = 9,", "workspace = 10,"), {"workspaces_on": False}),
    (("hl.gesture", "gesture ="), {"gesture": 0}),
    (("hl.workspace_rule", "hl.window_rule", "workspace = w[", "workspace = f[", 'name  = "hs-no-gaps', "match = {",
      "border_size = 0", "rounding    = 0"), {"smart_gaps": False}),
    (("hl.monitor", "monitor =", "output   =", "mode     =", "position =", "scale    ="), {"monitor_on": False}),
    (('hl.on("hyprland.start"', "hl.exec_cmd", "exec-once"),
     {"autostart": [], "autostart_waybar": False, "polkit_agent": False}),
    (("hl.env", "env ="), {"cursor_theme": "", "qt_theme": ""}),
    (('hl.curve("hs_linear"', 'leaf = "borderangle"', "animation = borderangle", "bezier = hs_linear"),
     {"border_rotate": False}),
    (("hl.curve", "hl.animation", "bezier =", "animation ="), {"anim": False}),
)


def plan(e: dict) -> dict:
    """Was der Fix tun würde: {kind: 'skip'|'state'|'bind'|'comment'|'none', ...}."""
    if not e["file"] or not e["line"]:
        return {"kind": "none"}
    if not is_ours(e):
        return {"kind": "comment"}
    try:
        text = Path(e["file"]).read_text(encoding="utf-8", errors="replace")
    except OSError:
        return {"kind": "none"}
    path = option_path(text, e["line"])
    if path:
        return {"kind": "skip", "option": path}
    src = text.splitlines()[e["line"] - 1].strip() if e["line"] <= len(text.splitlines()) else ""
    if src.startswith(("hl.bind(", "bind", "unbind")):
        return {"kind": "bind", "src": src}
    for prefixes, change in FEATURES:
        if src.startswith(prefixes):
            return {"kind": "state", "change": change}
    return {"kind": "comment"}


def apply_fix(e: dict, hs: dict, log) -> bool:
    """Behebt einen Fehler. hs (Hyprland-Zustand) wird bei eigenen Fehlern angepasst; danach neu anwenden."""
    p = plan(e)
    if p["kind"] == "skip":
        if p["option"] not in hs["skip_opts"]:
            hs["skip_opts"].append(p["option"])
        log(f"Option wird nicht mehr geschrieben: {p['option']}")
        return True
    if p["kind"] == "state":
        hs.update(json.loads(json.dumps(p["change"])))
        log(f"Abgeschaltet: {', '.join(p['change'])}")
        return True
    if p["kind"] == "bind":
        before = len(hs["binds"])
        src = p["src"]
        hs["binds"] = [b for b in hs["binds"]
                       if not (src == model.bind_lua(b) or src in model.bind_conf(b) if model.bind_ok(b) else False)]
        log(f"Kürzel entfernt: {src}" if len(hs["binds"]) < before else f"Kürzel nicht gefunden: {src}")
        return len(hs["binds"]) < before
    if p["kind"] == "comment":
        f = Path(e["file"])
        prefix = "-- [Hypr Studio Fix] " if f.suffix == ".lua" else "# [Hypr Studio Fix] "
        return comment_line(f, e["line"], prefix, log)
    return False


def comment_line(f: Path, line: int, prefix: str, log) -> bool:
    try:
        lines = f.read_text(encoding="utf-8", errors="replace").splitlines(keepends=True)
    except OSError as ex:
        log(f"{f}: {ex}")
        return False
    if not 0 < line <= len(lines) or lines[line - 1].lstrip().startswith(prefix.strip()):
        return False
    bak = model.stamp(f)
    shutil.copy2(f, bak)
    log(f"Sicherung: {bak}")
    lines[line - 1] = prefix + lines[line - 1]
    f.write_text("".join(lines), encoding="utf-8")
    log(f"Auskommentiert: {f.name}:{line}")
    return True


# ---------- Sicherungen ----------

BACKUP_RE = re.compile(r"^(?P<orig>.+?)\.(?:hyprstudio-original|hyprstudio-\d{8}-\d{6}(?:-\d+)?|bak|vor-deinstallation)$")


def backups(cfg_home: Path) -> list:
    """[(sicherung, original)] in den Ordnern, die Hypr Studio anfasst; neueste zuerst."""
    out = []
    for d in ("hypr", "waybar", "rofi", "wofi", "fuzzel"):
        folder = cfg_home / d
        if not folder.is_dir():
            continue
        for f in folder.iterdir():
            m = BACKUP_RE.match(f.name)
            if f.is_file() and m:
                out.append((f, folder / m.group("orig")))
    return sorted(out, key=lambda x: x[0].stat().st_mtime, reverse=True)


def restore(backup: Path, target: Path, log) -> bool:
    """Sicherung zurückkopieren; die aktuelle Datei wird vorher selbst gesichert (nichts geht verloren)."""
    if target.is_file():
        keep = model.stamp(target)
        shutil.copy2(target, keep)
        log(f"Sicherung: {keep}")
    shutil.copy2(backup, target)
    log(f"Wiederhergestellt: {target} ← {backup.name}")
    return True
