use gtk::prelude::*;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // Der Root-Helfer lädt keine Oberfläche (läuft per pkexec ohne Desktop-Umgebung)
    if args.len() > 1 && args[1] == "--helper" {
        std::process::exit(linuxaio::helper::main(&args[2..]));
    }
    // Testmodus: auch Hypr Studios eigene Einstellungen nicht im echten Home
    if linuxaio::usercmd::dry() {
        hyprstudio::config::set_path(linuxaio::usercmd::cache().join("testmodus").join("hyprstudio.json"));
    }
    let app = gtk::Application::builder()
        .application_id(linuxaio::APP_ID)
        .flags(gtk::gio::ApplicationFlags::NON_UNIQUE | gtk::gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();
    app.connect_command_line(|app, _| {
        app.activate();
        0.into()
    });
    app.connect_activate(linuxaio::ui::win::activate);
    std::process::exit(app.run().into());
}
