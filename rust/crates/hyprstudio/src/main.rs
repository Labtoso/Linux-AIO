//! Hypr Studio starten: Fenster, `--laptop-keys` (nach der Installation, ohne Fenster) oder `--launcher` (L Launcher).

use gtk::prelude::*;
use hyprstudio::ui::studio::{self, Opts, Studio};
use std::process::ExitCode;

fn code(c: i32) -> ExitCode {
    ExitCode::from(c.clamp(0, 255) as u8)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--laptop-keys") {
        return code(hyprstudio::laptopkeys::cli());
    }
    if args.iter().any(|a| a == "--launcher") {
        return code(hyprstudio::ui::llauncher::main());
    }
    // Testhilfe (nur Testmodus): Pakete als installiert ansehen, z. B. HYPRSTUDIO_DRY_INSTALLED=kitty,hyprpaper
    if hyprstudio::model::dry() {
        if let Ok(v) = std::env::var("HYPRSTUDIO_DRY_INSTALLED") {
            let mut set = hyprstudio::packages::DRY_INSTALLED.lock().unwrap();
            set.extend(v.split(',').filter(|s| !s.is_empty()).map(String::from));
        }
    }
    let app = gtk::Application::builder().application_id(hyprstudio::APP_ID).build();
    app.connect_activate(|app| {
        if let Some(w) = app.active_window() {
            w.present();
            return;
        }
        let s = Studio::new(Some(app), Opts::default());
        s.present();
        if let Ok(shot) = std::env::var("HYPRSTUDIO_SHOT") {
            if !shot.is_empty() {
                let steps = std::env::var("HYPRSTUDIO_STEPS").unwrap_or_default();
                let s2 = s.clone();
                gtk::glib::timeout_add_local_once(std::time::Duration::from_millis(600), move || {
                    studio::shots(s2, shot, steps)
                });
            }
        }
        // Fenster lebt so lange wie die Anwendung
        std::mem::forget(s);
    });
    code(app.run_with_args::<&str>(&[]).into())
}
