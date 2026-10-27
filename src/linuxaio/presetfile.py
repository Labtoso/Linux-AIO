"""Export / import of everything Linux AIO set: app settings (incl. Kitty, Alacritty, launcher, Hyprland states),
Hypr Studio settings, GNOME settings of our pages, cursor/icon theme, fonts installed by Linux AIO.
File: *.linuxaio (JSON). Import: preview of the changes → backup of the current state → apply. No Qt."""

import json
import time
from pathlib import Path

import linuxaio
from . import desktops, fonts, gnome, looks, usercmd

FORMAT = "linuxaio-preset"
SKIP_CFG = {"version", "lang", "tab", "geometry", "settings_adv", "settings_de", "side_collapsed", "nav_collapsed"}


def _gkeys() -> list:
    out = []
    for specs in gnome.SECTIONS.values():
        for spec in specs:
            if len(spec) > 2 and spec[1] and spec[2]:
                out.append((spec[1], spec[2]))
    return sorted(set(out))


def _hs_cfg_path() -> Path:
    return usercmd.config_dir() / "LabtosoStudios" / "hyprstudio.json"


def _read_json(p: Path) -> dict:
    try:
        return json.loads(usercmd.readable(p).read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return {}


def collect(cfg: dict) -> dict:
    de = desktops.current() or "gnome"
    g = {}
    for schema, key in _gkeys():
        v = usercmd.gget(schema, key)
        if v is not None:
            g[f"{schema} {key}"] = v
    return {"format": FORMAT, "version": linuxaio.__version__, "created": time.strftime("%Y-%m-%d %H:%M"),
            "app": {k: v for k, v in cfg.items() if k not in SKIP_CFG},
            "hyprstudio": {k: v for k, v in _read_json(_hs_cfg_path()).items() if k not in SKIP_CFG},
            "gsettings": g,
            "cursor": looks.current(de, "cursor"), "icons": looks.current(de, "icon"),
            "fonts": sorted(fonts.list_installed())}


def export(cfg: dict, path: Path) -> Path:
    path = Path(path)
    if path.suffix != ".linuxaio":
        path = path.with_suffix(".linuxaio")
    path.write_text(json.dumps(collect(cfg), ensure_ascii=False, indent=1), encoding="utf-8")
    return path


class BadFile(ValueError):
    pass


def load(path: Path) -> dict:
    try:
        d = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, ValueError) as e:
        raise BadFile(str(e)) from e
    if not isinstance(d, dict) or d.get("format") != FORMAT:
        raise BadFile("not a Linux AIO preset")
    for k, typ in (("app", dict), ("hyprstudio", dict), ("gsettings", dict), ("fonts", list)):
        if not isinstance(d.get(k, typ()), typ):
            raise BadFile(k)
    return d


def changes(data: dict, cfg: dict) -> list:
    """Readable list of what the import changes: (area, name, old, new)."""
    cur = collect(cfg)
    out = []
    allowed = set(f"{s} {k}" for s, k in _gkeys())
    for k, v in data.get("gsettings", {}).items():
        if k in allowed and cur["gsettings"].get(k) != v:
            out.append(("GNOME", k.split(" ", 1)[1], cur["gsettings"].get(k), v))
    for kind in ("cursor", "icons"):
        if data.get(kind) and data[kind] != cur.get(kind):
            out.append(("Theme", kind, cur.get(kind), data[kind]))
    for f in sorted(set(data.get("fonts", [])) - set(cur["fonts"])):
        out.append(("Font", f, None, "install"))
    for area in ("app", "hyprstudio"):
        for k, v in data.get(area, {}).items():
            if k not in SKIP_CFG and cur[area].get(k) != v:
                out.append(("Linux AIO" if area == "app" else "Hypr Studio", k, "…", "…"))
    return out


def backup_current(cfg: dict) -> Path:
    d = usercmd.config_dir() / "LabtosoStudios" / "backups" / (time.strftime("%Y-%m-%d_%H%M%S") + "-before-import")
    d.mkdir(parents=True, exist_ok=True)
    return export(cfg, d / "before-import.linuxaio")


def apply(data: dict, cfg: dict, log=usercmd.log) -> dict:
    """Write everything (call backup_current first). Returns a summary; missing themes/fonts are downloaded."""
    res = {"gsettings": 0, "themes": [], "fonts": [], "errors": []}
    allowed = set(f"{s} {k}" for s, k in _gkeys())
    for k, v in data.get("gsettings", {}).items():
        if k in allowed:
            schema, key = k.split(" ", 1)
            if usercmd.gset(schema, key, v):
                res["gsettings"] += 1
    for k, v in data.get("app", {}).items():
        if k not in SKIP_CFG:
            cfg[k] = v
    hs = _read_json(_hs_cfg_path())
    hs.update({k: v for k, v in data.get("hyprstudio", {}).items() if k not in SKIP_CFG})
    p = _hs_cfg_path()
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(hs, ensure_ascii=False, indent=1), encoding="utf-8")
    de = desktops.current() or "gnome"
    for kind, fn in (("cursor", looks.apply_cursor), ("icons", looks.apply_icons)):
        name = data.get(kind)
        if not name:
            continue
        if looks.installed_path(name, "cursor" if kind == "cursor" else "icon") is None:
            try:
                _download_theme(name, "cursor" if kind == "cursor" else "icon", log)
            except Exception as e:  # noqa: BLE001  (weiter mit dem Rest)
                res["errors"].append(f"{name}: {e}")
                continue
        if fn(de, name, log=log):
            res["themes"].append(name)
    want = set(data.get("fonts", [])) - set(fonts.list_installed())
    if want:
        try:
            catalog, _err = fonts.catalog()
        except Exception as e:  # noqa: BLE001
            catalog = []
            res["errors"].append(str(e))
        for item in catalog:
            if fonts.safe_family(item["family"]) in want:
                try:
                    fonts.install(item, log)
                    res["fonts"].append(item["family"])
                except Exception as e:  # noqa: BLE001
                    res["errors"].append(f"{item['family']}: {e}")
    return res


def _download_theme(name: str, kind: str, log) -> None:
    from . import remote
    man = remote.load_manifest()
    items, _ = remote.items(man, "cursors" if kind == "cursor" else "icons")
    entry = next((e for e in items if e.get("name") == name), None)
    if entry is None:
        raise remote.SourceError("remote.empty", name)
    remote.install(entry, looks.user_icons())
    log(f"↓ {name}")
