//! Schriftarten mit einem Klick installieren: Nerd Fonts von GitHub und Comic Sans MS (Microsoft Core Fonts)
//! nach ~/.local/share/fonts (ohne root).

use crate::{model, net, util};
use std::io::Read;
use std::path::{Path, PathBuf};

pub const URL: &str = "https://github.com/ryanoasis/nerd-fonts/releases/latest/download/{}.tar.xz";
pub const FONT_EXT: &[&str] = &["ttf", "otf"];

/// Microsofts frei verteilte „Core Fonts for the Web“ (Quelle wie beim Arch-Paket ttf-ms-fonts):
/// selbstentpackende .exe mit CAB-Archiv (LZX) → nur die .ttf daraus, nichts wird ausgeführt
pub const COREFONTS: &[(&str, &str)] = &[("ComicSansMS", "https://downloads.sourceforge.net/corefonts/comic32.exe")];

/// (Archivname bei nerd-fonts bzw. COREFONTS, Schriftfamilie nach der Installation)
pub const CATALOG: &[(&str, &str)] = &[
    ("ComicSansMS", "Comic Sans MS"),
    ("JetBrainsMono", "JetBrainsMono Nerd Font"), ("FiraCode", "FiraCode Nerd Font"),
    ("Hack", "Hack Nerd Font"), ("CascadiaCode", "CaskaydiaCove Nerd Font"),
    ("CascadiaMono", "CaskaydiaMono Nerd Font"), ("Meslo", "MesloLGS Nerd Font"),
    ("Iosevka", "Iosevka Nerd Font"), ("IosevkaTerm", "IosevkaTerm Nerd Font"),
    ("SourceCodePro", "SauceCodePro Nerd Font"), ("RobotoMono", "RobotoMono Nerd Font"),
    ("UbuntuMono", "UbuntuMono Nerd Font"), ("Ubuntu", "Ubuntu Nerd Font"), ("UbuntuSans", "UbuntuSans Nerd Font"),
    ("Noto", "NotoSans Nerd Font"), ("DejaVuSansMono", "DejaVuSansM Nerd Font"),
    ("FiraMono", "FiraMono Nerd Font"), ("GeistMono", "GeistMono Nerd Font"),
    ("IBMPlexMono", "BlexMono Nerd Font"), ("Inconsolata", "Inconsolata Nerd Font"),
    ("VictorMono", "VictorMono Nerd Font"), ("Mononoki", "Mononoki Nerd Font"),
    ("SpaceMono", "SpaceMono Nerd Font"), ("ZedMono", "ZedMono Nerd Font"), ("Lilex", "Lilex Nerd Font"),
    ("Monaspace", "MonaspiceNe Nerd Font"), ("0xProto", "0xProto Nerd Font"), ("Agave", "Agave Nerd Font"),
    ("ComicShannsMono", "ComicShannsMono Nerd Font"), ("FantasqueSansMono", "FantasqueSansM Nerd Font"),
    ("Hasklig", "Hasklug Nerd Font"), ("IntelOneMono", "IntoneMono Nerd Font"), ("Terminus", "Terminess Nerd Font"),
    ("CodeNewRoman", "CodeNewRoman Nerd Font"), ("Go-Mono", "GoMono Nerd Font"), ("Arimo", "Arimo Nerd Font"),
    ("Tinos", "Tinos Nerd Font"), ("NerdFontsSymbolsOnly", "Symbols Nerd Font"),
];

pub fn fonts_dir() -> PathBuf {
    if model::dry() {
        return model::cache().join("testmodus").join("fonts");
    }
    util::home().join(".local/share/fonts/HyprStudio")
}

pub fn family_of(archive: &str) -> String {
    CATALOG.iter().find(|(a, _)| *a == archive).map(|(_, f)| f.to_string())
        .unwrap_or_else(|| format!("{} Nerd Font", archive))
}

fn is_font(name: &str) -> bool {
    let low = name.to_lowercase();
    FONT_EXT.iter().any(|e| low.ends_with(&format!(".{}", e)))
}

/// Entpackt ein tar-Archiv (gz, xz oder unkomprimiert) in eine Liste (Name, Daten) nur für Dateien.
pub fn tar_entries(data: &[u8], mut keep: impl FnMut(&str) -> bool) -> Result<Vec<(String, Vec<u8>)>, String> {
    let reader: Box<dyn Read> = if data.starts_with(&[0xfd, b'7', b'z', b'X', b'Z', 0]) {
        Box::new(xz2::read::XzDecoder::new(data))
    } else if data.starts_with(&[0x1f, 0x8b]) {
        Box::new(flate2::read::GzDecoder::new(data))
    } else {
        Box::new(data)
    };
    let mut ar = tar::Archive::new(reader);
    let mut out = vec![];
    for e in ar.entries().map_err(|e| e.to_string())? {
        let mut e = e.map_err(|e| e.to_string())?;
        if !e.header().entry_type().is_file() {
            continue;
        }
        let path = e.path().map_err(|e| e.to_string())?.to_string_lossy().into_owned();
        if !keep(&path) {
            continue;
        }
        let mut buf = vec![];
        e.read_to_end(&mut buf).map_err(|e| e.to_string())?;
        out.push((path, buf));
    }
    Ok(out)
}

/// Nur Schriftdateien flach in dest ablegen (keine Pfade aus dem Archiv übernehmen).
pub fn extract(data: &[u8], dest: &Path) -> Result<Vec<PathBuf>, String> {
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    let mut out = vec![];
    let entries = tar_entries(data, |p| {
        let name = Path::new(p).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        is_font(&name) && !name.starts_with('.')
    })?;
    for (p, bytes) in entries {
        let name = Path::new(&p).file_name().unwrap().to_string_lossy().into_owned();
        let target = dest.join(&name);
        if !target.exists() {
            std::fs::write(&target, bytes).map_err(|e| e.to_string())?;
        }
        out.push(target);
    }
    Ok(out)
}

/// Das CAB-Archiv aus einer selbstentpackenden .exe schneiden (echter Kopf: reserviert = 0, Version 1.3).
pub fn find_cab(data: &[u8]) -> Result<&[u8], String> {
    let mut i = 0;
    while let Some(p) = data[i..].windows(4).position(|w| w == b"MSCF") {
        let at = i + p;
        if data.len() >= at + 26 && data[at + 4..at + 8] == [0, 0, 0, 0] && data[at + 24..at + 26] == [3, 1] {
            let size = u32::from_le_bytes([data[at + 8], data[at + 9], data[at + 10], data[at + 11]]) as usize;
            return Ok(&data[at..(at + size).min(data.len())]);
        }
        i = at + 1;
    }
    Err("kein CAB-Archiv gefunden".into())
}

/// Entpacker für CAB/LZX: bsdtar (libarchive, auf Arch immer da), sonst cabextract oder 7z.
pub fn cab_tool() -> Option<Vec<&'static str>> {
    if util::has("bsdtar") {
        return Some(vec!["bsdtar", "-xf", "{cab}", "-C", "{out}"]);
    }
    if util::has("cabextract") {
        return Some(vec!["cabextract", "-q", "-d", "{out}", "{cab}"]);
    }
    for z in ["7z", "7za"] {
        if util::has(z) {
            return Some(vec![z, "x", "-y", "-o{out}", "{cab}"]);
        }
    }
    None
}

pub fn extract_cab(data: &[u8], dest: &Path) -> Result<Vec<PathBuf>, String> {
    let tool = cab_tool().ok_or("bsdtar/cabextract fehlt (Paket libarchive oder cabextract installieren)")?;
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    let tmp = tempdir()?;
    let cab = tmp.join("fonts.cab");
    let unpacked = tmp.join("out");
    std::fs::write(&cab, find_cab(data)?).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&unpacked).map_err(|e| e.to_string())?;
    let cmd: Vec<String> = tool.iter().map(|a| a.replace("{cab}", &cab.to_string_lossy())
        .replace("{out}", &unpacked.to_string_lossy())).collect();
    let refs: Vec<&str> = cmd.iter().map(|s| s.as_str()).collect();
    let o = util::capture(&refs, Some(60)).map_err(|e| e.to_string())?;
    if !o.ok() {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(format!("Command '{:?}' returned non-zero exit status {}.", cmd, o.code));
    }
    let mut out = vec![];
    for f in walk(&unpacked) {
        if is_font(&util::file_name(&f)) {
            let target = dest.join(f.file_name().unwrap());
            if !target.exists() {
                let _ = std::fs::copy(&f, &target);
            }
            out.push(target);
        }
    }
    let _ = std::fs::remove_dir_all(&tmp);
    Ok(out)
}

/// Alle Dateien unterhalb eines Ordners (rekursiv).
pub fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = vec![];
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

pub fn tempdir() -> Result<PathBuf, String> {
    let base = std::env::temp_dir();
    for i in 0..1000u32 {
        let d = base.join(format!("hyprstudio-{}-{}", std::process::id(), i));
        if std::fs::create_dir(&d).is_ok() {
            return Ok(d);
        }
    }
    Err("kein Temp-Ordner".into())
}

/// Archiv laden, entpacken, Font-Cache erneuern. Gibt die Schriftdateien zurück.
pub fn install(archive: &str, log: &dyn Fn(&str)) -> Result<Vec<PathBuf>, String> {
    let core = COREFONTS.iter().find(|(a, _)| *a == archive).map(|(_, u)| u.to_string());
    let url = core.clone().unwrap_or_else(|| URL.replace("{}", archive));
    log(&format!("↓ {}", url));
    let data = net::get_bytes(&url, 180, &net::user_agent())?;
    let dest = fonts_dir().join(archive);
    let files = if core.is_some() { extract_cab(&data, &dest)? } else { extract(&data, &dest)? };
    log(&format!("→ {} ({})", dest.display(), files.len()));
    if !model::dry() {
        if let Err(e) = util::capture(&["fc-cache", "-f", &fonts_dir().to_string_lossy()], Some(120)) {
            log(&format!("fc-cache: {}", e));
        }
    }
    Ok(files)
}

/// Bereits über Hypr Studio installierte Schriftdateien.
pub fn local_files() -> Vec<PathBuf> {
    let d = fonts_dir();
    if !d.is_dir() {
        return vec![];
    }
    walk(&d).into_iter().filter(|f| is_font(&util::file_name(f))).collect()
}
