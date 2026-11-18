//! Welche App läuft: das ganze Linux AIO oder SDDM Studio (nur der SDDM-Tab als eigenes Programm).
//! Beide teilen sich denselben Code; die Ausgabe legt Name, ID, Version, Konfiguration und Root-Helfer fest.

use once_cell::sync::OnceCell;

pub struct Edition {
    pub key: &'static str,
    pub name: &'static str,
    pub id: &'static str,
    pub version: &'static str,
    /// Konfigurationsdatei unter ~/.config/LabtosoStudios
    pub cfg_file: &'static str,
    /// Root-Helfer aus dem Paket (polkit-Aktion zeigt genau auf diese Datei)
    pub helper: &'static str,
    /// Programmdatei im Release: <prefix><version>-x86_64
    pub asset_prefix: &'static str,
    /// Name der einzelnen Programmdatei in ~/.local/bin
    pub local_name: &'static str,
    /// sichtbare Tabs
    pub tabs: &'static [&'static str],
    pub icon_svg: &'static [u8],
    pub comment: &'static str,
    pub comment_de: &'static str,
    pub keywords: &'static str,
}

pub static AIO: Edition = Edition {
    key: "linuxaio",
    name: crate::APP_NAME,
    id: crate::APP_ID,
    version: crate::VERSION,
    cfg_file: "linuxaio.json",
    helper: "/usr/lib/labtoso-linuxaio/linuxaio-helper",
    asset_prefix: "Linux-AIO-",
    local_name: "Linux-AIO",
    tabs: &["de", "sddm", "ply", "settings"],
    icon_svg: include_bytes!("../data/linux-aio.svg"),
    comment: "Desktops, SDDM, Plymouth and desktop settings in one place",
    comment_de: "Desktops, SDDM, Plymouth und Desktop-Einstellungen an einem Ort",
    keywords: "desktop;gnome;plasma;hyprland;sddm;plymouth;cursor;fonts;tweaks;",
};

pub static SDDM: Edition = Edition {
    key: "sddmstudio",
    name: "SDDM Studio",
    id: "de.labtosostudios.sddmstudio",
    version: SDDM_VERSION,
    cfg_file: "sddmstudio.json",
    helper: "/usr/lib/labtoso-sddmstudio/sddmstudio-helper",
    asset_prefix: "SDDM-Studio-",
    local_name: "SDDM-Studio",
    tabs: &["sddm"],
    icon_svg: include_bytes!("../data/sddm-studio.svg"),
    comment: "Apply, test and install SDDM login themes",
    comment_de: "SDDM-Anmelde-Themes anwenden, testen und installieren",
    keywords: "sddm;login;theme;greeter;",
};

/// Version von SDDM Studio (eigene Zählung, steht auch in crates/sddmstudio/Cargo.toml).
pub const SDDM_VERSION: &str = "1.0.0";

static CUR: OnceCell<&'static Edition> = OnceCell::new();

/// Einmal ganz am Anfang (vor dem Fenster).
pub fn set(e: &'static Edition) {
    let _ = CUR.set(e);
}

pub fn ed() -> &'static Edition {
    CUR.get().copied().unwrap_or(&AIO)
}

pub fn is_aio() -> bool {
    ed().key == AIO.key
}
