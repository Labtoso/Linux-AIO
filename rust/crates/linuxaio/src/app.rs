//! Programmstart (gemeinsam für Linux AIO und SDDM Studio).

use crate::edition::{self, Edition};
use gtk::prelude::*;

pub fn main(e: &'static Edition) -> i32 {
    edition::set(e);
    let args: Vec<String> = std::env::args().collect();
    // Der Root-Helfer lädt keine Oberfläche (läuft per pkexec ohne Desktop-Umgebung)
    if args.len() > 1 && args[1] == "--helper" {
        return crate::helper::main(&args[2..]);
    }
    // Testmodus: auch Hypr Studios eigene Einstellungen nicht im echten Home
    if crate::usercmd::dry() {
        hyprstudio::config::set_path(crate::usercmd::cache().join("testmodus").join("hyprstudio.json"));
    }
    let app = gtk::Application::builder()
        .application_id(e.id)
        .flags(gtk::gio::ApplicationFlags::NON_UNIQUE | gtk::gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();
    app.connect_command_line(|app, _| {
        app.activate();
        0.into()
    });
    app.connect_activate(crate::ui::win::activate);
    app.run().into()
}
