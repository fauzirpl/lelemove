// 1. Muatan installer (payload.zip) ditanam lewat include_bytes!. Tanpa PINDAH_PAYLOAD dipakai
//    berkas kosong, sehingga setup.exe yang dihasilkan berperan sebagai uninstall.exe.
// 2. Toolchain Rust GNU minimal tidak membawa libshlwapi.a, padahal crate `webbrowser` (dipakai
//    eframe untuk membuka tautan) me-link shlwapi. Import library lengkap Win32 milik crate
//    windows_x86_64_gnu (sudah ada di cache cargo) memuat fungsi shlwapi juga, jadi disalin
//    dengan nama libshlwapi.a ke OUT_DIR.
use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=PINDAH_PAYLOAD");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let muatan = match env::var("PINDAH_PAYLOAD") {
        Ok(p) if !p.trim().is_empty() => {
            println!("cargo:rerun-if-changed={p}");
            PathBuf::from(p)
        }
        _ => {
            let kosong = out.join("payload-kosong.zip");
            fs::write(&kosong, b"").unwrap();
            kosong
        }
    };
    println!("cargo:rustc-env=PINDAH_PAYLOAD_FILE={}", muatan.display());

    if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
        match cari_lib_windows() {
            Some(lib) => {
                fs::copy(&lib, out.join("libshlwapi.a")).unwrap();
                println!("cargo:rustc-link-search=native={}", out.display());
            }
            None => println!("cargo:warning=libwindows*.a (crate windows_x86_64_gnu) tidak ditemukan; link shlwapi bisa gagal"),
        }
    }
}

fn cari_lib_windows() -> Option<PathBuf> {
    let home = env::var("CARGO_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env::var("USERPROFILE").unwrap_or_default()).join(".cargo"));
    let mut kandidat = Vec::new();
    for reg in fs::read_dir(home.join("registry").join("src")).ok()?.flatten() {
        for c in fs::read_dir(reg.path()).into_iter().flatten().flatten() {
            if c.file_name().to_string_lossy().starts_with("windows_x86_64_gnu-") {
                for f in fs::read_dir(c.path().join("lib")).into_iter().flatten().flatten() {
                    if f.path().extension().is_some_and(|e| e == "a") {
                        kandidat.push(f.path());
                    }
                }
            }
        }
    }
    kandidat.sort();
    kandidat.pop()
}
