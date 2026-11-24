//! Installer Windows; isinya di ../setup.rs. Di macOS LELEMOVE dikemas sebagai .dmg, jadi
//! program ini hanya ada agar `cargo build` semua target tetap berjalan di Mac.
#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
#[path = "../setup.rs"]
mod isi;

#[cfg(windows)]
fn main() {
    isi::main();
}

#[cfg(not(windows))]
fn main() {
    eprintln!("Installer ini hanya untuk Windows. Di macOS, pasang LELEMOVE dari berkas .dmg.");
    std::process::exit(1);
}