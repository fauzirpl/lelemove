//! Mesin LELEMOVE (dulu pindah.ps1).
//!
//! Hanya MENYALIN (rclone copy) dan MENCOCOKKAN (rclone check). Tidak ada perintah yang
//! menghapus atau memindahkan file, baik di sumber maupun di tujuan.
//!
//! Data kerja (pengaturan, log, status) ada di %LOCALAPPDATA%\pindah-drive, dengan format JSON
//! yang sama dengan versi PowerShell, sehingga keduanya bisa saling membaca.
//! Token login disimpan oleh rclone sendiri di %APPDATA%\rclone\rclone.conf.

pub mod akun;
pub mod dasar;
pub mod jadwal;
pub mod job;
pub mod lanjut;
pub mod perintah;
pub mod rclone;
pub mod rute;

pub use perintah::{tampilkan_bantuan, urai_argumen, Mesin, Opsi};
