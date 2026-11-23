//! job.json (penyalinan yang sedang/terakhir berjalan) dan pembacaan statistik log rclone.

use regex::Regex;
use serde_json::{Map, Value};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::LazyLock;
use std::time::{Duration, SystemTime};

use super::dasar::{baca_objek, bil, jalur, teks, tulis_json};
use crate::win;

pub fn baca_job() -> Option<Map<String, Value>> {
    baca_objek(&jalur().job)
}

/// Membaca job.json, menimpa field yang diberikan, lalu menyimpannya lagi.
pub fn perbarui_job(ubah: &[(&str, Value)], baru: bool) {
    let mut h = if baru { Map::new() } else { baca_job().unwrap_or_default() };
    for (k, v) in ubah {
        h.insert((*k).to_string(), v.clone());
    }
    tulis_json(&jalur().job, &Value::Object(h));
}

pub fn job_berjalan(job: &Map<String, Value>) -> bool {
    if !teks(job.get("Selesai")).is_empty() {
        return false;
    }
    if let Some(p) = bil(job.get("PidRclone")) {
        return win::proses_hidup(p as u32, &["rclone"]);
    }
    match bil(job.get("PidInduk")) {
        // "powershell": penyalinan yang dimulai oleh versi PowerShell sebelumnya.
        Some(p) => win::proses_hidup(p as u32, &["pindah", "powershell"]),
        None => false,
    }
}

/// N baris terakhir sebuah berkas teks (dibaca dari ekornya saja, log bisa besar).
pub fn baca_ekor(path: &Path, n: usize) -> Vec<String> {
    let Ok(mut f) = std::fs::File::open(path) else { return Vec::new() };
    let pjg = f.metadata().map(|m| m.len()).unwrap_or(0);
    let mulai = pjg.saturating_sub(256 * 1024);
    if f.seek(SeekFrom::Start(mulai)).is_err() {
        return Vec::new();
    }
    let mut buf = Vec::new();
    let _ = f.read_to_end(&mut buf);
    let s = String::from_utf8_lossy(&buf);
    let mut baris: Vec<String> = s.lines().map(|l| l.trim_end_matches('\r').to_string()).collect();
    if mulai > 0 && !baris.is_empty() {
        baris.remove(0); // baris pertama mungkin terpotong
    }
    let awal = baris.len().saturating_sub(n);
    baris.split_off(awal)
}

#[derive(Clone, Debug, Default)]
pub struct Statistik {
    /// "1.234 GiB / 5.000 GiB, 25%, 10.000 MiB/s, ETA 6m"
    pub data: String,
    /// "12 / 369, 3%"
    pub file: String,
    pub error: String,
    pub waktu: String,
    /// File yang sedang diunggah, mis. "DLH/a.zip (40%)".
    pub sedang: Vec<String>,
    /// Google/Microsoft menolak karena terlalu banyak permintaan (rate limit / kuota project).
    pub dibatasi: bool,
    /// Login salah satu akun kedaluwarsa (invalid_grant): perlu login ulang.
    pub perlu_login: bool,
    /// Galat jaringan (DNS/koneksi) di bagian akhir log: internet sedang bermasalah.
    pub jaringan: bool,
}

static RE_BLOK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*Transferred:.*B\s*/").unwrap());
static RE_BATAS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"rateLimitExceeded|RATE_LIMIT_EXCEEDED|Quota exceeded|activityLimitReached|429 Too Many|throttled")
        .unwrap()
});
static RE_ERRORS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*Errors:\s*(.+)$").unwrap());
static RE_TRANSFER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*Transferred:\s*(.+)$").unwrap());
static RE_WAKTU: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*Elapsed time:\s*(.+)$").unwrap());
static RE_SEDANG_PERSEN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*\*\s+(.+?):\s*(\d+)%").unwrap());
static RE_SEDANG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*\*\s+(.+?):\s").unwrap());

/// Mengambil blok statistik terakhir yang ditulis rclone ke log (tiap --stats).
pub fn baca_statistik(log: &str) -> Option<Statistik> {
    if log.is_empty() {
        return None;
    }
    let b = baca_ekor(Path::new(log), 400);
    let i = b.iter().rposition(|l| RE_BLOK.is_match(l))?;
    let mut s = Statistik {
        data: RE_TRANSFER.captures(&b[i]).map(|c| c[1].trim().to_string()).unwrap_or_default(),
        ..Default::default()
    };
    s.dibatasi = b.iter().any(|l| RE_BATAS.is_match(l));
    s.perlu_login = b.iter().any(|l| super::rclone::token_kedaluwarsa(l));
    // Hanya 40 baris terakhir: galat jaringan lama yang sudah pulih tidak perlu ditampilkan lagi.
    s.jaringan = b[b.len().saturating_sub(40)..].iter().any(|l| l.contains("ERROR") && super::rclone::galat_jaringan(l));
    let mut k = i + 1;
    while k < b.len().min(i + 15) {
        let l = &b[k];
        if let Some(c) = RE_ERRORS.captures(l) {
            s.error = c[1].trim().to_string();
        } else if let Some(c) = RE_TRANSFER.captures(l) {
            s.file = c[1].trim().to_string();
        } else if let Some(c) = RE_WAKTU.captures(l) {
            s.waktu = c[1].trim().to_string();
            break;
        }
        k += 1;
    }
    // Blok "Transferring:" berisi file yang sedang diunggah, mis. " * DLH/a.zip: 40% /480Mi, ..."
    k += 1;
    while k < b.len() {
        let l = &b[k];
        k += 1;
        if l.trim_start().starts_with("Transferring:") {
            continue;
        }
        if let Some(c) = RE_SEDANG_PERSEN.captures(l) {
            s.sedang.push(format!("{} ({}%)", &c[1], &c[2]));
            continue;
        }
        if let Some(c) = RE_SEDANG.captures(l) {
            s.sedang.push(c[1].to_string());
            continue;
        }
        break;
    }
    Some(s)
}

/// Log & laporan di folder log yang lebih tua dari ini dihapus saat penyalinan dimulai.
pub const UMUR_LOG_HARI: u64 = 60;

/// Menghapus berkas biasa langsung di `dir` (subfolder tidak disentuh) yang terakhir diubah lebih
/// dari `umur` sebelum `sekarang`, kecuali yang bernama `dijaga` (tanpa membedakan huruf besar-kecil).
/// Hasil: jumlah berkas yang terhapus.
pub fn hapus_log_lama(dir: &Path, dijaga: &str, umur: Duration, sekarang: SystemTime) -> usize {
    let Ok(isi) = std::fs::read_dir(dir) else { return 0 };
    let mut n = 0;
    for e in isi.flatten() {
        // symlink_metadata: tautan tidak diikuti, jadi berkas di luar folder log tidak terhapus.
        let Ok(m) = e.path().symlink_metadata() else { continue };
        if !m.file_type().is_file() {
            continue;
        }
        if !dijaga.is_empty() && e.file_name().to_string_lossy().eq_ignore_ascii_case(dijaga) {
            continue;
        }
        let tua = m.modified().ok().and_then(|t| sekarang.duration_since(t).ok()).is_some_and(|d| d > umur);
        if tua && std::fs::remove_file(e.path()).is_ok() {
            n += 1;
        }
    }
    n
}

/// Membersihkan folder log data kerja. Log penyalinan yang tercatat di job.json selalu disimpan,
/// karena status & progres (juga di jendela aplikasi) dibaca dari sana.
pub fn bersihkan_log_lama() -> usize {
    let log = baca_job().map(|j| teks(j.get("Log"))).unwrap_or_default();
    let dijaga = Path::new(&log).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    hapus_log_lama(&jalur().log, &dijaga, Duration::from_secs(UMUR_LOG_HARI * 86_400), SystemTime::now())
}

pub fn baris_error(log: &str, n: usize) -> Vec<String> {
    if log.is_empty() {
        return Vec::new();
    }
    let e: Vec<String> = baca_ekor(Path::new(log), 2000).into_iter().filter(|l| l.contains(" ERROR : ")).collect();
    let awal = e.len().saturating_sub(n);
    e[awal..].to_vec()
}

#[cfg(test)]
mod uji {
    use super::*;

    #[test]
    fn statistik() {
        let d = std::env::temp_dir().join("pindah-drive-uji-statistik.log");
        std::fs::write(
            &d,
            "2026/11/13 14:33:12 INFO  : \n\
Transferred:   \t  1.234 GiB / 5.000 GiB, 25%, 10.000 MiB/s, ETA 6m\n\
Errors:                 0\n\
Checks:                 0 / 0, -\n\
Transferred:           12 / 369, 3%\n\
Elapsed time:       2m0.5s\n\
Transferring:\n\
 *                                   DLH/a.zip: 40% /480Mi, 5Mi/s, 1m0s\n\
\n",
        )
        .unwrap();
        let s = baca_statistik(&d.to_string_lossy()).unwrap();
        assert_eq!(s.data, "1.234 GiB / 5.000 GiB, 25%, 10.000 MiB/s, ETA 6m");
        assert_eq!(s.file, "12 / 369, 3%");
        assert_eq!(s.error, "0");
        assert_eq!(s.waktu, "2m0.5s");
        assert_eq!(s.sedang, vec!["DLH/a.zip (40%)".to_string()]);
        let _ = std::fs::remove_file(d);
    }

    #[test]
    fn log_lama() {
        let d = std::env::temp_dir().join(format!("pindah-drive-uji-log-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("sub")).unwrap();
        let sekarang = SystemTime::now();
        let hari = |n: u64| sekarang - Duration::from_secs(n * 86_400);
        for (nama, umur) in [("salin-lama.log", 61), ("uji-baru.log", 3), ("SALIN-DIJAGA.log", 90), ("verifikasi-batas.txt", 59)] {
            let p = d.join(nama);
            std::fs::write(&p, "isi").unwrap();
            std::fs::File::options().write(true).open(&p).unwrap().set_modified(hari(umur)).unwrap();
        }
        let n = hapus_log_lama(&d, "salin-dijaga.log", Duration::from_secs(60 * 86_400), sekarang);
        assert_eq!(n, 1);
        assert!(!d.join("salin-lama.log").exists());
        for tetap in ["uji-baru.log", "SALIN-DIJAGA.log", "verifikasi-batas.txt", "sub"] {
            assert!(d.join(tetap).exists(), "{tetap} harus tetap ada");
        }
        let _ = std::fs::remove_dir_all(d);
    }
}
