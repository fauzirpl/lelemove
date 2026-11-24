//! Alat build Windows (ikon & manifest .exe); isinya di ../tanam-ikon.rs. Tidak dipakai di macOS:
//! ikon .app dibuat dari aset/ikon.ico oleh workflow build.

#[cfg(windows)]
#[path = "../tanam-ikon.rs"]
mod isi;

#[cfg(windows)]
fn main() -> std::process::ExitCode {
    isi::main()
}

#[cfg(not(windows))]
fn main() -> std::process::ExitCode {
    eprintln!("tanam-ikon hanya untuk Windows.");
    std::process::ExitCode::FAILURE
}