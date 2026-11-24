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

/// Nama aplikasi yang tampil ke pengguna (dulu "Pindah Drive"). Nama internal (folder data
/// `pindah-drive`, `pindah.exe`, remote `pd-*`, APP_ID, mutex) sengaja tidak diganti agar akun,
/// pengaturan, dan pemasangan lama tetap terbaca.
pub const NAMA_APP: &str = "LELEMOVE";

/// AppUserModelID aplikasi: pengelompokan ikon taskbar dan pengirim notifikasi Windows.
pub const APP_ID: &str = "PindahDrive.Aplikasi";

/// Nama mutex jendela aplikasi; installer memakainya untuk tahu aplikasi sedang terbuka.
pub const MUTEX_JENDELA: &str = "Local\\PindahDriveGui";
