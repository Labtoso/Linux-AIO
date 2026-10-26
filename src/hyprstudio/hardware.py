"""Gerät erkennen (Hersteller, Modell, Laptop?) und die Befehle für Helligkeit/Lautstärke wählen. Ohne Qt.

Quelle: /sys/class/dmi/id – Lenovo schreibt den echten Modellnamen in product_version (product_name ist nur
eine Nummer wie „82LM“). Laptop = Gehäusetyp Notebook/Laptop/Convertible … oder es gibt einen Akku/Backlight.
"""

from pathlib import Path

from . import model

DMI = Path("/sys/class/dmi/id")
LAPTOP_CHASSIS = {8, 9, 10, 11, 14, 30, 31, 32}  # Portable, Laptop, Notebook, Hand Held, Sub Notebook, Tablet …
VENDORS = {"LENOVO": "Lenovo", "HP": "HP", "HEWLETT-PACKARD": "HP", "DELL INC.": "Dell", "ASUSTEK COMPUTER INC.": "ASUS",
           "ACER": "Acer", "MICRO-STAR INTERNATIONAL CO., LTD.": "MSI", "SAMSUNG ELECTRONICS CO., LTD.": "Samsung",
           "MICROSOFT CORPORATION": "Microsoft", "FRAMEWORK": "Framework", "HUAWEI": "Huawei", "XIAOMI": "Xiaomi",
           "APPLE INC.": "Apple", "TUXEDO": "TUXEDO", "SCHENKER": "Schenker", "CLEVO": "Clevo", "GIGABYTE": "Gigabyte"}
GENERIC = ("", "to be filled by o.e.m.", "default string", "system product name", "not applicable", "none", "0")


def _read(name: str) -> str:
    try:
        return (DMI / name).read_text(encoding="utf-8", errors="replace").strip()
    except OSError:
        return ""


def detect() -> dict:
    """{vendor, model, laptop, backlight: [..], battery} – im Testmodus ein Beispiel-Lenovo."""
    if model.DRY:
        return {"vendor": "Lenovo", "model": "IdeaPad 5 14ALC05", "laptop": True, "backlight": ["amdgpu_bl1"],
                "battery": True}
    raw_vendor = _read("sys_vendor")
    vendor = VENDORS.get(raw_vendor.upper(), raw_vendor.title() if raw_vendor.isupper() else raw_vendor)
    name, version = _read("product_name"), _read("product_version")
    if raw_vendor.upper() == "LENOVO" and version.lower() not in GENERIC:
        dev = version  # z. B. „IdeaPad 5 14ALC05“, product_name wäre „82LM“
    else:
        dev = name if name.lower() not in GENERIC else version
    try:
        chassis = int(_read("chassis_type") or 0)
    except ValueError:
        chassis = 0
    bl = Path("/sys/class/backlight")
    backlight = sorted(p.name for p in bl.iterdir()) if bl.is_dir() else []
    ps = Path("/sys/class/power_supply")
    battery = ps.is_dir() and any(p.name.startswith("BAT") for p in ps.iterdir())
    return {"vendor": vendor, "model": dev, "laptop": chassis in LAPTOP_CHASSIS or battery or bool(backlight),
            "backlight": backlight, "battery": battery}


def label(d: dict) -> str:
    text = " ".join(x for x in (d["vendor"], d["model"]) if x and x.lower() not in GENERIC)
    return text or "?"


def volume_tool() -> str:
    """wpctl (PipeWire/WirePlumber, Standard auf Arch) oder pactl (PulseAudio)."""
    if model.DRY or model.which("wpctl"):
        return "wpctl"
    return "pactl" if model.which("pactl") else "wpctl"


def tools() -> dict:
    """Welche Befehle für die Tasten da sind."""
    if model.DRY:
        return {"brightnessctl": True, "volume": True, "playerctl": True}
    vol = volume_tool()
    return {"brightnessctl": bool(model.which("brightnessctl")), "volume": bool(model.which(vol)),
            "playerctl": bool(model.which("playerctl"))}
