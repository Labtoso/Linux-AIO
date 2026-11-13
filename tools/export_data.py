"""Exports the data tables of the former Python version (texts, palettes, presets, catalogues) as JSON
for the Rust crates. Only needed once while porting / when the old tables change."""
import json
import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")
os.environ["DEMANAGER_DRYRUN"] = "1"

HS = ROOT / "crates" / "hyprstudio" / "data"
LA = ROOT / "crates" / "linuxaio" / "data"


def dump(path: Path, obj) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(obj, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")


def tup(x):
    """tuples → lists recursively (JSON)."""
    if isinstance(x, (list, tuple)):
        return [tup(v) for v in x]
    if isinstance(x, dict):
        return {str(k): tup(v) for k, v in x.items()}
    return x


def main(which):
    import importlib
    import pkgutil
    import hyprstudio
    for m in pkgutil.iter_modules(hyprstudio.__path__):
        importlib.import_module("hyprstudio." + m.name)
    import hyprstudio.i18n as hi
    from hyprstudio import palettes, model
    out = {"i18n": hi.TEXT, "palettes": {"order": palettes.ORDER, "font": palettes.FONT,
                                          "font_size": palettes.FONT_SIZE, "palettes": palettes.PALETTES,
                                          "legacy": palettes.LEGACY_ACCENT}}
    for name in which.get("hyprstudio", []):
        mod, attr = name.split(":")
        out[f"{mod}.{attr}"] = tup(getattr(importlib.import_module("hyprstudio." + mod), attr))
    for k, v in out.items():
        dump(HS / f"{k}.json", tup(v))
    try:
        import linuxaio
        for m in pkgutil.iter_modules(linuxaio.__path__):
            if m.name not in ("__main__",):
                importlib.import_module("linuxaio." + m.name)
        from linuxaio import i18n as li
        dump(LA / "i18n.json", li.TEXT)
        for name in which.get("linuxaio", []):
            mod, attr = name.split(":")
            dump(LA / f"{mod}.{attr}.json", tup(getattr(importlib.import_module("linuxaio." + mod), attr)))
    except ImportError:
        pass


if __name__ == "__main__":
    spec = json.loads((ROOT / "tools" / "export_list.json").read_text())
    main(spec)
