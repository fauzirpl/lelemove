//! Alat build: menanam ikon (.ico) dan manifest ke sebuah .exe lewat UpdateResourceW.
//! Toolchain Rust GNU tidak membawa windres, jadi resource dipasang setelah kompilasi.
//!
//!   tanam-ikon <berkas.exe> --ikon <ikon.ico> [--manifest <app.manifest>]

use std::ffi::c_void;
use std::process::ExitCode;
use windows_sys::Win32::System::LibraryLoader::{BeginUpdateResourceW, EndUpdateResourceW, UpdateResourceW};

use pindah_drive::win::lebar;

const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;
const RT_MANIFEST: u16 = 24;
const BAHASA_NETRAL: u16 = 0;

fn id(n: u16) -> *const u16 {
    n as usize as *const u16
}

fn main() -> ExitCode {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let ambil = |n: &str| a.iter().position(|x| x == n).and_then(|i| a.get(i + 1)).cloned();
    let (Some(exe), Some(ikon)) = (a.first().cloned(), ambil("--ikon")) else {
        eprintln!("pakai: tanam-ikon <berkas.exe> --ikon <ikon.ico> [--manifest <berkas>]");
        return ExitCode::from(2);
    };
    let ico = match std::fs::read(&ikon) {
        Ok(b) if b.len() >= 6 => b,
        _ => {
            eprintln!("ikon tidak terbaca: {ikon}");
            return ExitCode::from(1);
        }
    };
    let manifest = match ambil("--manifest").map(std::fs::read) {
        Some(Ok(m)) => Some(m),
        Some(Err(e)) => {
            eprintln!("manifest tidak terbaca: {e}");
            return ExitCode::from(1);
        }
        None => None,
    };

    // ICONDIR (6 byte) + ICONDIRENTRY (16 byte) x n; di resource, offset diganti nomor RT_ICON.
    let n = u16::from_le_bytes([ico[4], ico[5]]) as usize;
    let mut grup: Vec<u8> = vec![0, 0, 1, 0];
    grup.extend_from_slice(&(n as u16).to_le_bytes());
    let mut gambar: Vec<&[u8]> = Vec::new();
    for i in 0..n {
        let o = 6 + 16 * i;
        let pjg = u32::from_le_bytes(ico[o + 8..o + 12].try_into().unwrap()) as usize;
        let off = u32::from_le_bytes(ico[o + 12..o + 16].try_into().unwrap()) as usize;
        if off + pjg > ico.len() {
            eprintln!("ikon rusak (bingkai {i})");
            return ExitCode::from(1);
        }
        grup.extend_from_slice(&ico[o..o + 12]);
        grup.extend_from_slice(&((i + 1) as u16).to_le_bytes());
        gambar.push(&ico[off..off + pjg]);
    }

    let w = lebar(&exe);
    unsafe {
        let h = BeginUpdateResourceW(w.as_ptr(), 0);
        if h.is_null() {
            eprintln!("tidak bisa membuka {exe}: {}", std::io::Error::last_os_error());
            return ExitCode::from(1);
        }
        let mut ok = true;
        for (i, g) in gambar.iter().enumerate() {
            ok &= UpdateResourceW(h, id(RT_ICON), id(i as u16 + 1), BAHASA_NETRAL, g.as_ptr() as *const c_void, g.len() as u32) != 0;
        }
        ok &= UpdateResourceW(h, id(RT_GROUP_ICON), id(1), BAHASA_NETRAL, grup.as_ptr() as *const c_void, grup.len() as u32) != 0;
        if let Some(m) = &manifest {
            ok &= UpdateResourceW(h, id(RT_MANIFEST), id(1), BAHASA_NETRAL, m.as_ptr() as *const c_void, m.len() as u32) != 0;
        }
        if !ok {
            let e = std::io::Error::last_os_error();
            EndUpdateResourceW(h, 1);
            eprintln!("gagal menulis resource: {e}");
            return ExitCode::from(1);
        }
        if EndUpdateResourceW(h, 0) == 0 {
            eprintln!("gagal menyimpan {exe}: {}", std::io::Error::last_os_error());
            return ExitCode::from(1);
        }
    }
    println!("ikon{} ditanam: {exe}", if manifest.is_some() { " + manifest" } else { "" });
    ExitCode::SUCCESS
}
