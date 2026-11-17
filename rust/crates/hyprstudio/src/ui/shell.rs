//! Das Hauptfenster aus Sicht der Bausteine: Status, Protokoll, Einstellungen, Akzent, Hintergrund-Aufgaben.
//! Hypr Studio und Linux AIO setzen hier ihr Fenster ein (eins pro Prozess).

use crate::jsonx::Obj;
use gtk::glib;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub trait Shell {
    fn set_status(&self, text: &str);
    fn log(&self, text: &str);
    fn bar_busy(&self, on: bool);
    fn accent(&self) -> String;
    fn cfg(&self) -> Rc<RefCell<Obj>>;
    fn save_later(&self);
    fn window(&self) -> Option<gtk::Window>;
    /// Protokoll aufklappen (z. B. nach einem Fehler).
    fn show_log(&self) {}
}

/// Einstellungen eines Fensters + „später speichern“ (Hypr Studio und Linux AIO haben je eigene).
#[derive(Clone)]
pub struct Conf {
    pub cfg: Rc<RefCell<Obj>>,
    pub save: Rc<dyn Fn()>,
}

impl Conf {
    pub fn save_later(&self) {
        (self.save)()
    }
}

thread_local! {
    static SHELL: RefCell<Option<Rc<dyn Shell>>> = const { RefCell::new(None) };
}

pub fn set_shell(s: Rc<dyn Shell>) {
    SHELL.with(|x| *x.borrow_mut() = Some(s));
}

pub fn shell() -> Rc<dyn Shell> {
    SHELL.with(|x| x.borrow().clone()).expect("Fenster fehlt")
}

pub fn has_shell() -> bool {
    SHELL.with(|x| x.borrow().is_some())
}

pub fn log(text: &str) {
    if has_shell() {
        shell().log(text);
    }
}

pub fn status(text: &str) {
    if has_shell() {
        shell().set_status(text);
    }
}

/// Aufgabe im Hintergrund (Installieren, Herunterladen …): Zeilen gehen live ins Protokoll, danach `done` im
/// Hauptthread – wie die früheren QThread-Jobs.
pub fn run_job<T: Send + 'static>(
    work: impl FnOnce(&dyn Fn(&str)) -> T + Send + 'static,
    done: impl FnOnce(T) + 'static,
) {
    let (tx, rx) = async_channel::unbounded::<String>();
    let handle = gtk::gio::spawn_blocking(move || {
        let send = |s: &str| {
            let _ = tx.send_blocking(s.to_string());
        };
        work(&send)
    });
    glib::spawn_future_local(async move {
        let reader = glib::spawn_future_local(async move {
            while let Ok(line) = rx.recv().await {
                log(&line);
            }
        });
        let res = handle.await;
        let _ = reader.await;
        if let Ok(r) = res {
            done(r);
        }
    });
}

/// Nur rechnen im Hintergrund (ohne Protokoll), Ergebnis im Hauptthread.
pub fn run_bg<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static, done: impl FnOnce(T) + 'static) {
    let handle = gtk::gio::spawn_blocking(work);
    glib::spawn_future_local(async move {
        if let Ok(r) = handle.await {
            done(r);
        }
    });
}

/// Einmal später im Hauptthread ausführen (wie QTimer.singleShot).
pub fn later(ms: u64, f: impl FnOnce() + 'static) {
    glib::timeout_add_local_once(std::time::Duration::from_millis(ms), f);
}

/// Fenster der Shell (für Dialoge).
pub fn parent_window() -> Option<gtk::Window> {
    if has_shell() { shell().window() } else { None }
}

pub fn toplevel_of<W: IsA<gtk::Widget>>(w: &W) -> Option<gtk::Window> {
    w.root().and_then(|r| r.downcast::<gtk::Window>().ok()).or_else(parent_window)
}
