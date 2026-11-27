//! LELEMOVE: salin file antar OneDrive, SharePoint, Google Drive, dan folder komputer.
//!
//! Semua pemindahan data dikerjakan rclone. Pustaka ini berisi:
//! - `mesin`: logika yang dulu ada di pindah.ps1 (akun, rute, periksa, uji, salin, verifikasi);
//! - `win`: lapisan sistem operasi (proses, kunci satu-jendela, folder dikenal, layar tetap
//!   menyala, Tong Sampah). Isinya `win.rs` di Windows dan `mac.rs` di macOS, dengan nama
//!   fungsi yang sama, sehingga kode lain tidak perlu tahu sedang berjalan di mana;
//! - `gaya`: tampilan egui bersama untuk jendela aplikasi dan installer;
//! - `notifikasi`: notifikasi sistem saat penyalinan latar berakhir.

#[cfg(not(any(windows, target_os = "macos")))]
compile_error!("LELEMOVE hanya dibuat untuk Windows dan macOS.");

pub mod gaya;
pub mod mesin;

#[cfg_attr(target_os = "macos", path = "notifikasi_mac.rs")]
pub mod notifikasi;

#[cfg_attr(target_os = "macos", path = "mac.rs")]
pub mod win;

pub const VERSI: &str = env!("CARGO_PKG_VERSION");

/// Dukungan sukarela untuk pengembangan LELEMOVE (dibuka di browser; aplikasi tidak mengirim data).
pub const SAWERIA: &str = "https://saweria.co/pemecuteai";

/// Nama aplikasi yang tampil ke pengguna (dulu "Pindah Drive"). Nama internal (folder data
/// `pindah-drive`, `pindah.exe`, remote `pd-*`, APP_ID, mutex) sengaja tidak diganti agar akun,
/// pengaturan, dan pemasangan lama tetap terbaca.
pub const NAMA_APP: &str = "LELEMOVE";

/// AppUserModelID aplikasi: pengelompokan ikon taskbar dan pengirim notifikasi Windows.
pub const APP_ID: &str = "PindahDrive.Aplikasi";

/// Nama berkas catatan galat tak terduga (panik) di folder log.
pub const BERKAS_PANIK: &str = "galat-tak-terduga.txt";

/// Satu baris catatan panik: waktu, program, versi, utas, pesan, lokasi di kode.
fn baris_panik(waktu: &str, program: &str, utas: &str, pesan: &str, lokasi: &str) -> String {
    format!("{waktu} {program} {VERSI} utas {utas}: {pesan} [{lokasi}]\n")
}

/// Mencatat galat tak terduga (panik) ke `<data>/log/galat-tak-terduga.txt`. Program berjendela
/// tidak punya konsol, sehingga tanpa ini ia hanya menghilang tanpa jejak. Dengan `kotak`, panik
/// di utas utama (yang menghentikan program) juga ditampilkan sebagai kotak pesan; panik di utas
/// latar hanya dicatat karena programnya tetap berjalan.
pub fn catat_panik(program: &'static str, kotak: bool) {
    let lama = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let pesan = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "(tanpa pesan)".to_string());
        let lokasi = info.location().map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column())).unwrap_or_default();
        let kini = std::thread::current();
        let utas = kini.name().unwrap_or("latar").to_string();
        let dir = &mesin::dasar::jalur().log;
        let berkas = dir.join(BERKAS_PANIK);
        let _ = std::fs::create_dir_all(dir);
        let baris = baris_panik(&mesin::dasar::sekarang_iso(), program, &utas, &pesan, &lokasi);
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&berkas) {
            use std::io::Write;
            let _ = f.write_all(baris.as_bytes());
        }
        // Hook bawaan mencetak ke stderr (terlihat di konsol dan di log CI).
        lama(info);
        if kotak && utas == "main" {
            win::kotak_pesan(
                NAMA_APP,
                &format!(
                    "{program} berhenti karena galat tak terduga.\n\n{pesan}\n\nRinciannya dicatat di:\n{}\n\nSertakan berkas itu bila melaporkan masalah.",
                    berkas.display()
                ),
                win::MB_OK | win::MB_ICONERROR,
            );
        }
    }));
}


/// Nama mutex jendela aplikasi; installer memakainya untuk tahu aplikasi sedang terbuka.
pub const MUTEX_JENDELA: &str = "Local\\PindahDriveGui";

#[cfg(test)]
mod uji {
    #[test]
    fn baris_panik_memuat_semua_bagian() {
        let b = super::baris_panik("2026-11-27T10:00:00", "LELEMOVE", "main", "indeks di luar batas", "src/x.rs:1:2");
        assert!(b.starts_with("2026-11-27T10:00:00 LELEMOVE "));
        assert!(b.contains(super::VERSI) && b.contains("utas main: indeks di luar batas [src/x.rs:1:2]"));
        assert!(b.ends_with('\n'));
    }
}
