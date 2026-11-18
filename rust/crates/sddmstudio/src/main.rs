//! SDDM Studio: der SDDM-Tab von Linux AIO als eigenes Programm (gleicher Code, gleiches Verhalten).

fn main() {
    std::process::exit(linuxaio::app::main(&linuxaio::edition::SDDM));
}
