//! Mesin LELEMOVE (dulu pindah.ps1).
//!
//! MENYALIN (rclone copy) dan MENCOCOKKAN (rclone check). Satu-satunya perintah yang menghapus
//! adalah `pindahkan`, atas permintaan eksplisit: ia memverifikasi ulang, lalu hanya menghapus
//! dari SUMBER file yang cocok persis di tujuan, ke Recycle Bin / Sampah layanan itu.
//! Tidak ada perintah yang menghapus apa pun di tujuan.
//!
//! Data kerja (pengaturan, log, status) ada di %LOCALAPPDATA%\pindah-drive, dengan format JSON
//! yang sama dengan versi PowerShell, sehingga keduanya bisa saling membaca.
//! Token login disimpan oleh rclone sendiri di %APPDATA%\rclone\rclone.conf.

pub mod akun;
pub mod dasar;
pub mod filter;
pub mod jadwal;
pub mod job;
pub mod lanjut;
pub mod penyedia;
pub mod perintah;
pub mod rclone;
pub mod riwayat;
pub mod rute;

pub use perintah::{tampilkan_bantuan, urai_argumen, Mesin, Opsi};
