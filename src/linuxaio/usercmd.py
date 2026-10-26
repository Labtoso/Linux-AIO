"""Befehle ohne Root (gsettings, plasma-apply-*, hyprctl …) und Dateien im Home.

Im Testmodus (DEMANAGER_DRYRUN=1) wird nichts ausgeführt und nichts im echten Home geschrieben:
Befehle landen nur im Protokoll, gsettings-Werte in einer Testmodus-Datei, Dateien unter dem Testmodus-Ordner.
"""

import ast
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

DRY = os.environ.get("DEMANAGER_DRYRUN") == "1" or sys.platform == "win32"
CACHE = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "LabtosoStudios" / "linuxaio"
DRY_HOME = CACHE / "testmodus" / "home"
DRY_GSET = CACHE / "testmodus" / "gsettings.json"
if DRY:
    os.environ["DEMANAGER_DRYRUN"] = "1"
    os.environ["HYPRSTUDIO_DRYRUN"] = "1"     # eingebautes Hypr Studio schreibt dann auch nur in den Testordner

_log = print


def set_logger(fn) -> None:
    global _log
    _log = fn


def log(text: str) -> None:
    _log(text)


def home() -> Path:
    """Echtes Home bzw. im Testmodus ein eigener Ordner (für alles, was geschrieben wird)."""
    return DRY_HOME if DRY else Path.home()


def real_home() -> Path:
    return Path.home()


def config_dir() -> Path:
    if DRY:
        return DRY_HOME / ".config"
    return Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config"))


def data_dir() -> Path:
    if DRY:
        return DRY_HOME / ".local" / "share"
    return Path(os.environ.get("XDG_DATA_HOME", Path.home() / ".local" / "share"))


def readable(path: Path) -> Path:
    """Zum Lesen: im Testmodus die Testdatei, solange es sie nicht gibt die echte."""
    if DRY and not path.exists():
        try:
            return Path.home() / path.relative_to(DRY_HOME)
        except ValueError:
            return path
    return path


def have(tool: str) -> bool:
    return shutil.which(tool) is not None


def run(cmd: list, timeout: int = 20) -> tuple[bool, str]:
    """Befehl ausführen (Testmodus: nur protokollieren). Gibt (ok, Ausgabe) zurück."""
    log("$ " + " ".join(_q(c) for c in cmd) + ("   [Testmodus]" if DRY else ""))
    if DRY:
        return True, ""
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout, env=_env())
    except (OSError, subprocess.TimeoutExpired) as e:
        log(str(e))
        return False, str(e)
    out = (r.stdout + r.stderr).strip()
    if out:
        log(out)
    return r.returncode == 0, out


def query(cmd: list, timeout: int = 8) -> str | None:
    """Nur lesen (läuft auch im Testmodus)."""
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout, env=_env())
    except (OSError, subprocess.TimeoutExpired):
        return None
    return r.stdout.strip() if r.returncode == 0 else None


def _env() -> dict:
    """Child processes without launch tokens (otherwise GNOME may show a busy cursor)."""
    env = dict(os.environ)
    for k in ("DESKTOP_STARTUP_ID", "XDG_ACTIVATION_TOKEN"):
        env.pop(k, None)
    return env


def _q(s: str) -> str:
    s = str(s)
    return s if s and all(c.isalnum() or c in "-_./:=@,+" for c in s) else "'" + s.replace("'", "'\\''") + "'"


# ---------- gsettings ----------

_schemas = None
_keys = {}


def schemas() -> set:
    global _schemas
    if _schemas is None:
        out = query(["gsettings", "list-schemas"]) if have("gsettings") else None
        _schemas = set(out.split()) if out else set()
    return _schemas


def has_key(schema: str, key: str) -> bool:
    if schema not in schemas():
        return False
    if schema not in _keys:
        out = query(["gsettings", "list-keys", schema])
        _keys[schema] = set(out.split()) if out else set()
    return key in _keys[schema]


def _dry_values() -> dict:
    try:
        return json.loads(DRY_GSET.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return {}


def parse_value(text: str):
    """gsettings-Ausgabe in Python: 'abc' → abc, true → True, uint32 4 → 4, @as [] → []."""
    t = text.strip()
    for pre in ("uint32 ", "int32 ", "int64 ", "uint64 ", "double ", "@as ", "@a{sv} "):
        if t.startswith(pre):
            t = t[len(pre):]
    if t in ("true", "false"):
        return t == "true"
    try:
        return ast.literal_eval(t)
    except (ValueError, SyntaxError):
        return t


def gvariant(value) -> str:
    """Python → GVariant-Text für gsettings set."""
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, (int, float)):
        return repr(value)
    if isinstance(value, (list, tuple)):
        return "[" + ", ".join(gvariant(v) for v in value) + "]"
    s = str(value).replace("\\", "\\\\").replace("'", "\\'")
    return f"'{s}'"


def gget(schema: str, key: str, default=None):
    if DRY:
        v = _dry_values().get(f"{schema} {key}")
        if v is not None:
            return v
    if not has_key(schema, key):
        return default
    out = query(["gsettings", "get", schema, key])
    return default if out is None else parse_value(out)


def gset(schema: str, key: str, value) -> bool:
    if DRY:
        vals = _dry_values()
        vals[f"{schema} {key}"] = value
        try:
            DRY_GSET.parent.mkdir(parents=True, exist_ok=True)
            DRY_GSET.write_text(json.dumps(vals, ensure_ascii=False, indent=1), encoding="utf-8")
        except OSError:
            pass
    if not DRY and not has_key(schema, key):
        log(f"gsettings: {schema} {key} fehlt")
        return False
    return run(["gsettings", "set", schema, key, gvariant(value)])[0]


# ---------- ini-Dateien (ohne configparser, Reihenfolge und Kommentare bleiben) ----------

def ini_set(path: Path, section: str, key: str, value: str) -> None:
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines() if path.is_file() else []
    out, sec, done, found_sec = [], None, False, False
    for ln in lines:
        s = ln.strip()
        if s.startswith("[") and s.endswith("]"):
            if sec == section and not done:
                out.append(f"{key}={value}")
                done = True
            sec = s[1:-1]
            found_sec = found_sec or sec == section
        elif sec == section and not done and s.split("=", 1)[0].strip() == key:
            out.append(f"{key}={value}")
            done = True
            continue
        out.append(ln)
    if not done:
        if not found_sec:
            if out and out[-1].strip():
                out.append("")
            out.append(f"[{section}]")
        out.append(f"{key}={value}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(out) + "\n", encoding="utf-8")
    log(f"→ {path}: [{section}] {key}={value}")


def ini_get(path: Path, section: str, key: str, default: str = "") -> str:
    sec = None
    try:
        for ln in path.read_text(encoding="utf-8", errors="replace").splitlines():
            s = ln.strip()
            if s.startswith("[") and s.endswith("]"):
                sec = s[1:-1]
            elif sec == section and "=" in s and s.split("=", 1)[0].strip() == key:
                return s.split("=", 1)[1].strip()
    except OSError:
        pass
    return default
