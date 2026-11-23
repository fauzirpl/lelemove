//! Lanjut otomatis saat jaringan putus.
//!
//! Pekerja latar (`salin -LogLatar`) menjalankan rclone. Bila rclone berhenti dan galat
//! terakhirnya galat jaringan (DNS menolak, koneksi putus, batas waktu habis), pekerja menunggu
//! dengan jeda yang makin panjang sambil menguji sambungan ke server layanan cloud, lalu
//! menjalankan rclone lagi. rclone copy melewati file yang sudah sampai, jadi tidak ada yang diulang.
//! Login kedaluwarsa dan batas unggah harian Google tidak diulang: keduanya butuh tindakan pengguna.

use std::net::{TcpStream, ToSocketAddrs};
use std::path::Path;
use std::time::Duration;

use super::akun::{jenis_dari_nama, remote_dari};
use super::dasar::waktu_lokal;
use super::rclone::{galat_jaringan, token_kedaluwarsa};

/// Percobaan menjalankan rclone paling banyak (termasuk yang pertama). Dengan jeda di bawah,
/// pekerja bertahan kira-kira 4 jam sebelum menyerah. Bisa diganti lewat PINDAH_DRIVE_MAKS_ULANG
/// (untuk pengujian).
pub const MAKS_PERCOBAAN: u32 = 25;

/// Lama paling lama menunggu internet kembali dalam satu jeda, sebelum menyerah.
pub const TUNGGU_JARINGAN_MAKS: Duration = Duration::from_secs(6 * 3600);

pub fn maks_percobaan() -> u32 {
    std::env::var("PINDAH_DRIVE_MAKS_ULANG").ok().and_then(|v| v.parse().ok()).filter(|n| *n >= 1).unwrap_or(MAKS_PERCOBAAN)
}

/// Jeda sebelum percobaan berikutnya, setelah percobaan ke-`n` (mulai 1) gagal:
/// 30 detik, 1 menit, 2 menit, 5 menit, lalu 10 menit. PINDAH_DRIVE_JEDA_ULANG (detik)
/// menggantinya untuk pengujian.
pub fn jeda(n: u32) -> Duration {
    if let Some(d) = std::env::var("PINDAH_DRIVE_JEDA_ULANG").ok().and_then(|v| v.parse::<u64>().ok()) {
        return Duration::from_secs(d);
    }
    Duration::from_secs(match n {
        0 | 1 => 30,
        2 => 60,
        3 => 120,
        4 => 300,
        _ => 600,
    })
}

#[derive(Debug, PartialEq)]
pub enum Putusan {
    Selesai,
    /// Galat jaringan: tunggu, lalu jalankan rclone lagi.
    Ulang,
    /// Galat lain (login, kuota harian, berkas): berhenti dan kabari pengguna.
    Berhenti,
}

/// Baris log rclone yang menandai kegagalan (galat file, galat fatal).
fn baris_galat(l: &str) -> bool {
    l.contains(" ERROR ") || l.contains("CRITICAL") || l.contains("Failed to copy") || l.contains("Fatal error")
}

/// `potongan` = bagian log yang ditulis rclone pada percobaan ini saja.
pub fn putuskan(kode: i32, potongan: &str) -> Putusan {
    if kode == 0 {
        return Putusan::Selesai;
    }
    if potongan.contains("upload limit") || potongan.contains("uploadLimitExceeded") || token_kedaluwarsa(potongan) {
        return Putusan::Berhenti;
    }
    // Yang menentukan adalah galat TERAKHIR: galat jaringan yang sempat pulih di tengah jalan tidak
    // berarti penyalinan berhenti karena jaringan.
    match potongan.lines().rev().find(|l| baris_galat(l)) {
        Some(l) if galat_jaringan(l) => Putusan::Ulang,
        Some(_) => Putusan::Berhenti,
        // Tanpa baris galat (mis. rclone mati sebelum sempat menulis): ulang hanya bila ada jejak jaringan.
        None if galat_jaringan(potongan) => Putusan::Ulang,
        None => Putusan::Berhenti,
    }
}

/// Server yang harus bisa dihubungi agar penyalinan bisa dilanjutkan.
pub fn host_rute(src: &str, dst: &str) -> Vec<&'static str> {
    let mut h = Vec::new();
    for j in [src, dst] {
        let host = match jenis_dari_nama(&remote_dari(j)) {
            "google" => "www.googleapis.com",
            "onedrive" | "sharepoint" => "graph.microsoft.com",
            _ => continue,
        };
        if !h.contains(&host) {
            h.push(host);
        }
    }
    h
}

/// DNS + sambungan TCP ke port 443 (tanpa mengirim apa pun).
pub fn bisa_dihubungi(host: &str) -> bool {
    match (host, 443).to_socket_addrs() {
        Ok(alamat) => alamat.take(3).any(|a| TcpStream::connect_timeout(&a, Duration::from_secs(5)).is_ok()),
        Err(_) => false,
    }
}

/// "HH.MM" waktu lokal `detik` dari sekarang (untuk "dicoba lagi pukul ...").
pub fn jam_nanti(detik: u64) -> String {
    let (_, _, _, h, m, s) = waktu_lokal();
    let total = (h as u64 * 3600 + m as u64 * 60 + s as u64 + detik) % 86_400;
    format!("{:02}.{:02}", total / 3600, (total % 3600) / 60)
}

/// Isi log mulai dari `awal` (byte), paling banyak 512 KB terakhir.
pub fn potongan_log(log: &Path, awal: u64) -> String {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut f) = std::fs::File::open(log) else { return String::new() };
    let pjg = f.metadata().map(|m| m.len()).unwrap_or(0);
    let mulai = awal.max(pjg.saturating_sub(512 * 1024)).min(pjg);
    if f.seek(SeekFrom::Start(mulai)).is_err() {
        return String::new();
    }
    let mut b = Vec::new();
    let _ = f.read_to_end(&mut b);
    String::from_utf8_lossy(&b).into_owned()
}

#[cfg(test)]
mod uji {
    use super::*;

    const DNS: &str = "2026/11/23 15:00:01 ERROR : a.zip: Failed to copy: Post \"https://www.googleapis.com/upload/drive/v3/files\": dial tcp: lookup www.googleapis.com: no such host";

    #[test]
    fn keputusan() {
        assert_eq!(putuskan(0, DNS), Putusan::Selesai);
        assert_eq!(putuskan(1, DNS), Putusan::Ulang);
        assert_eq!(
            putuskan(7, "ERROR : x: Failed to copy: googleapi: Error 403: User rate limit exceeded, uploadLimitExceeded"),
            Putusan::Berhenti
        );
        assert_eq!(putuskan(1, "CRITICAL: Failed to create file system: couldn't fetch token: invalid_grant"), Putusan::Berhenti);
        // Galat terakhir bukan jaringan (mis. izin ditolak) -> jangan diulang terus.
        let campur = format!("{DNS}\n2026/11/23 15:01:00 ERROR : b.doc: Failed to copy: Access is denied.");
        assert_eq!(putuskan(1, &campur), Putusan::Berhenti);
        // Sebaliknya: galat lain lalu jaringan putus -> ulang.
        let campur = format!("2026/11/23 15:00:00 ERROR : b.doc: Failed to copy: Access is denied.\n{DNS}");
        assert_eq!(putuskan(1, &campur), Putusan::Ulang);
        assert_eq!(putuskan(1, ""), Putusan::Berhenti);
    }

    #[test]
    fn host() {
        assert_eq!(host_rute("pd-onedrive:Kerja", "pd-gdrive:Arsip"), vec!["graph.microsoft.com", "www.googleapis.com"]);
        assert_eq!(host_rute("D:\\Data", "pd-sharepoint:Dok"), vec!["graph.microsoft.com"]);
        assert!(host_rute("D:\\Data", "E:\\Salinan").is_empty());
    }

    #[test]
    fn jeda_bertahap() {
        if std::env::var("PINDAH_DRIVE_JEDA_ULANG").is_ok() {
            return;
        }
        assert_eq!(jeda(1), Duration::from_secs(30));
        assert_eq!(jeda(4), Duration::from_secs(300));
        assert_eq!(jeda(9), Duration::from_secs(600));
    }
}
