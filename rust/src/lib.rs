//! LELEMOVE: salin file antar OneDrive, SharePoint, Google Drive, dan folder komputer.
//!
//! Semua pemindahan data dikerjakan rclone. Pustaka ini berisi:
//! - `mesin`: logika yang dulu ada di pindah.ps1 (akun, rute, periksa, uji, salin, verifikasi);
//! - `win`: pembungkus kecil Win32 (proses, mutex, folder dikenal, layar tetap menyala);
//! - `gaya`: tampilan egui bersama untuk jendela aplikasi dan installer;
//! - `notifikasi`: notifikasi Windows (toast) saat penyalinan latar berakhir.

pub mod gaya;
pub mod mesin;
pub mod notifikasi;
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

/// Nama mutex jendela aplikasi; installer memakainya untuk tahu aplikasi sedang terbuka.
pub const MUTEX_JENDELA: &str = "Local\\PindahDriveGui";
