//! Sumber & tujuan: folder di komputer (D:\Data) atau "<akun>:<folder>" (pd-onedrive:Kerja).

use serde_json::{Map, Value};
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};

use super::akun::{jenis_dari_nama, remote_dari};
use super::dasar::{baca_objek, cocok_kecualian, jalur, teks, tulis_json, ATRIBUT_HANYA_CLOUD, REMOTE_GOOGLE};
use super::rclone::{panggil, Mode};
use crate::{v, win};

#[derive(Clone, Default, Debug)]
pub struct Rute {
    pub sumber: Option<String>,
    pub tujuan: Option<String>,
}

pub fn adalah_lokal(s: &str) -> bool {
    let b = s.as_bytes();
    (b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && b[2] == b'\\') || s.starts_with("\\\\")
}

pub fn rapikan_sumber(s: &str) -> String {
    let s = s.trim().trim_matches('"');
    let b = s.as_bytes();
    // "D:" atau "D:\" -> "D:\"
    if (b.len() == 2 || (b.len() == 3 && b[2] == b'\\')) && b[0].is_ascii_alphabetic() && b[1] == b':' {
        return format!("{}\\", &s[..2]);
    }
    if adalah_lokal(s) {
        return s.trim_end_matches('\\').to_string();
    }
    s.replace('\\', "/")
}

pub fn normalisasi_tujuan(d: &str) -> String {
    let d = d.trim().trim_matches('"');
    if adalah_lokal(d) {
        return d.to_string();
    }
    let mut d = d.to_string();
    if remote_dari(&d).is_empty() {
        d = format!("{REMOTE_GOOGLE}:{}", d.trim_start_matches(['/', '\\']));
    }
    d.replace('\\', "/")
}

#[derive(Clone, Debug)]
pub struct OneDriveLokal {
    pub folder: String,
    /// Pribadi | Kantor/Sekolah | OneDrive
    pub jenis: String,
}

/// Folder OneDrive yang tersinkron di komputer ini (dari registry dan variabel lingkungan).
pub fn daftar_onedrive() -> Vec<OneDriveLokal> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;
    let mut hasil: Vec<OneDriveLokal> = Vec::new();
    if let Ok(k) = RegKey::predef(HKEY_CURRENT_USER).open_subkey("Software\\Microsoft\\OneDrive\\Accounts") {
        for nama in k.enum_keys().flatten() {
            let Ok(a) = k.open_subkey(&nama) else { continue };
            let folder: String = a.get_value("UserFolder").unwrap_or_default();
            if folder.is_empty() || !Path::new(&folder).is_dir() {
                continue;
            }
            let bisnis: u32 = a.get_value("Business").unwrap_or(0);
            let jenis = if bisnis == 1 { "Kantor/Sekolah" } else { "Pribadi" };
            hasil.push(OneDriveLokal { folder, jenis: jenis.into() });
        }
    }
    for var in ["OneDriveCommercial", "OneDriveConsumer", "OneDrive"] {
        if let Ok(v) = std::env::var(var) {
            if !v.is_empty() && Path::new(&v).is_dir() && !hasil.iter().any(|o| o.folder.eq_ignore_ascii_case(&v)) {
                hasil.push(OneDriveLokal { folder: v, jenis: "OneDrive".into() });
            }
        }
    }
    hasil
}

pub fn di_onedrive(path: &str) -> bool {
    let p = path.to_lowercase();
    daftar_onedrive().iter().any(|o| {
        let akar = o.folder.trim_end_matches('\\').to_lowercase();
        p == akar || p.starts_with(&format!("{akar}\\"))
    })
}

/// Folder tujuan bawaan: "<akun tujuan>:Pindahan <asal>/<nama folder sumber>".
/// Tanpa akun tujuan (`remote_tujuan` kosong, folder di komputer): Dokumen\Pindahan <asal>\<nama>.
pub fn tujuan_bawaan(src: &str, remote_tujuan: &str) -> String {
    let mut nama = if adalah_lokal(src) {
        Path::new(src).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
    } else {
        let isi = match src.find(':') {
            Some(i) => &src[i + 1..],
            None => src,
        };
        isi.trim_matches('/').rsplit('/').next().unwrap_or("").to_string()
    };
    let rem_s = remote_dari(src);
    let asal = if !rem_s.is_empty() && jenis_dari_nama(&rem_s) == "google" {
        "Google Drive"
    } else if !rem_s.is_empty() && jenis_dari_nama(&rem_s) == "sharepoint" {
        "SharePoint"
    } else if adalah_lokal(src) && !di_onedrive(src) {
        "Komputer"
    } else {
        "OneDrive"
    };
    if nama.is_empty() || nama.contains(':') {
        nama = asal.to_string();
    }
    if remote_tujuan.is_empty() {
        let dok = win::folder(win::Folder::Dokumen)
            .unwrap_or_else(|| PathBuf::from(std::env::var("USERPROFILE").unwrap_or_default()).join("Documents"));
        return dok.join(format!("Pindahan {asal}")).join(nama).to_string_lossy().to_string();
    }
    format!("{remote_tujuan}:Pindahan {asal}/{nama}")
}

pub fn daftar_subfolder(src: &str) -> Vec<String> {
    if adalah_lokal(src) {
        let mut n: Vec<String> = std::fs::read_dir(src)
            .map(|it| {
                it.flatten()
                    .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect()
            })
            .unwrap_or_default();
        n.sort_by_key(|a| a.to_lowercase());
        return n;
    }
    let h = panggil(&v!["lsf", src, "--dirs-only"], Mode::Tangkap);
    h.out
        .lines()
        .map(|l| l.trim().trim_end_matches('/').to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

pub fn gabung_path(induk: &str, nama: &str) -> String {
    if adalah_lokal(induk) {
        return Path::new(induk).join(nama).to_string_lossy().to_string();
    }
    if induk.ends_with(':') {
        return format!("{induk}{nama}");
    }
    format!("{induk}/{nama}")
}

#[derive(Default, Debug)]
pub struct Ukuran {
    pub jumlah: i64,
    pub bytes: i64,
    pub jumlah_hanya_cloud: i64,
    pub hanya_cloud: i64,
    pub terbesar: i64,
    pub terbesar_nama: String,
}

/// Menghitung isi folder lokal dari metadata saja: tidak memicu unduhan OneDrive.
/// Tautan simbolik dan junction tidak diikuti (menghindari putaran tanpa akhir).
pub fn ukur_lokal(path: &str) -> Ukuran {
    let mut u = Ukuran::default();
    let mut antre = vec![PathBuf::from(path)];
    while let Some(dir) = antre.pop() {
        let Ok(isi) = std::fs::read_dir(&dir) else { continue };
        for e in isi.flatten() {
            let Ok(t) = e.file_type() else { continue };
            if t.is_symlink() {
                continue;
            }
            if t.is_dir() {
                antre.push(e.path());
                continue;
            }
            let nama = e.file_name().to_string_lossy().to_string();
            if cocok_kecualian(&nama) {
                continue;
            }
            let Ok(m) = e.metadata() else { continue };
            let n = m.file_size() as i64;
            u.jumlah += 1;
            u.bytes += n;
            if n > u.terbesar {
                u.terbesar = n;
                u.terbesar_nama = e.path().to_string_lossy().to_string();
            }
            if m.file_attributes() & ATRIBUT_HANYA_CLOUD != 0 {
                u.jumlah_hanya_cloud += 1;
                u.hanya_cloud += n;
            }
        }
    }
    u
}

pub fn ruang_disk(path: &str) -> i64 {
    win::ruang_bebas(Path::new(path)) as i64
}

// ------------------------------------------------------------------ pengaturan.json

pub fn baca_pengaturan() -> Rute {
    let p = baca_objek(&jalur().pengaturan).unwrap_or_default();
    let ambil = |k: &str| Some(teks(p.get(k))).filter(|s| !s.is_empty());
    Rute { sumber: ambil("Sumber"), tujuan: ambil("Tujuan") }
}

/// Mengubah pengaturan.json tanpa membuang field lain (Sumber, Tujuan, BatasKecepatan, ...).
pub(crate) fn ubah_pengaturan(ubah: impl FnOnce(&mut Map<String, Value>)) {
    let mut m = baca_objek(&jalur().pengaturan).unwrap_or_default();
    // Berkas baru tetap memuat Sumber & Tujuan, seperti yang ditulis versi PowerShell.
    for k in ["Sumber", "Tujuan"] {
        m.entry(k).or_insert(Value::Null);
    }
    ubah(&mut m);
    tulis_json(&jalur().pengaturan, &Value::Object(m));
}

pub fn simpan_pengaturan(src: Option<&str>, dst: Option<&str>) {
    let nilai = |s: Option<&str>| s.map(|s| Value::String(s.into())).unwrap_or(Value::Null);
    ubah_pengaturan(|m| {
        m.insert("Sumber".into(), nilai(src));
        m.insert("Tujuan".into(), nilai(dst));
    });
}

/// Batas kecepatan tersimpan dalam bentuk --bwlimit rclone ("5M"), atau "off" (tanpa batas).
pub fn baca_batas_kecepatan() -> String {
    let p = baca_objek(&jalur().pengaturan).unwrap_or_default();
    normalisasi_batas(&teks(p.get("BatasKecepatan"))).unwrap_or_else(|_| "off".into())
}

/// Disimpan sebagai "BatasKecepatan": "5M", atau null bila tanpa batas.
pub fn simpan_batas_kecepatan(batas: &str) {
    let v = match normalisasi_batas(batas) {
        Ok(b) if b != "off" => Value::String(b),
        _ => Value::Null,
    };
    ubah_pengaturan(|m| {
        m.insert("BatasKecepatan".into(), v);
    });
}

/// Lanjut otomatis saat jaringan putus (Pengaturan). Bawaan: aktif.
pub fn baca_lanjut_otomatis() -> bool {
    let p = baca_objek(&jalur().pengaturan).unwrap_or_default();
    !matches!(p.get("LanjutOtomatis"), Some(Value::Bool(false)))
}

pub fn simpan_lanjut_otomatis(aktif: bool) {
    ubah_pengaturan(|m| {
        m.insert("LanjutOtomatis".into(), Value::Bool(aktif));
    });
}

// ------------------------------------------------------------------ batas kecepatan

/// Nilai -BatasKecepatan ke bentuk --bwlimit rclone (satuan 1024, per detik):
/// "2M", "2 MB", "2MiB/s", "1,5m" -> "2M"/"1.5M";  "500k" -> "500K";  "10M:5M" = unggah:unduh.
/// Angka tanpa satuan berarti KB/dtk (sama dengan rclone). "off", "0", "tanpa", "" -> "off".
pub fn normalisasi_batas(s: &str) -> Result<String, String> {
    use regex::Regex;
    use std::sync::LazyLock;
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)^(\d+(?:[.,]\d+)?)\s*([kmgtp]?)(i?b?)(?:/s|/dtk|ps)?$").unwrap()
    });
    static RE_JADWAL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\d{1,2}:\d{2}\s*,").unwrap());
    let s = s.trim();
    if s.is_empty() || ["off", "tanpa", "tanpa batas", "none"].contains(&s.to_lowercase().as_str()) {
        return Ok("off".into());
    }
    // Jadwal rclone ("08:00,512k 18:00,off") tidak didukung; jangan sampai terbaca sebagai unggah:unduh.
    if RE_JADWAL.is_match(s) {
        return Err(format!("Batas kecepatan berjadwal belum didukung: \"{s}\". Pakai satu nilai, mis. 5M."));
    }
    let mut bagian = Vec::new();
    for b in s.split(':') {
        let b = b.trim();
        if b.eq_ignore_ascii_case("off") {
            bagian.push(None);
            continue;
        }
        let c = RE.captures(b).ok_or_else(|| {
            format!("Batas kecepatan tidak dikenal: \"{s}\". Contoh: 5M (5 MB/dtk), 500K, 10M:5M (unggah:unduh), off.")
        })?;
        let angka = c[1].replace(',', ".");
        if angka.parse::<f64>().unwrap_or(0.0) <= 0.0 {
            bagian.push(None);
            continue;
        }
        let satuan = match (c[2].to_ascii_uppercase().as_str(), c[3].is_empty()) {
            ("", true) => "K".to_string(), // rclone: angka polos = KiB/dtk
            ("", false) => "B".to_string(),
            (h, _) => h.to_string(),
        };
        bagian.push(Some(format!("{angka}{satuan}")));
    }
    match bagian.as_slice() {
        [b] => Ok(b.clone().unwrap_or_else(|| "off".into())),
        [None, None] => Ok("off".into()),
        [u, d] => Ok(format!(
            "{}:{}",
            u.clone().unwrap_or_else(|| "off".into()),
            d.clone().unwrap_or_else(|| "off".into())
        )),
        _ => Err(format!("Batas kecepatan tidak dikenal: \"{s}\". Pakai satu nilai (5M) atau unggah:unduh (10M:5M).")),
    }
}

/// "5M" -> "5 MB/dtk";  "1.5M" -> "1,5 MB/dtk";  "10M:5M" -> "unggah 10 MB/dtk, unduh 5 MB/dtk".
pub fn tampil_batas(batas: &str) -> String {
    fn satu(b: &str) -> String {
        if b == "off" {
            return "tanpa batas".into();
        }
        let (angka, h) = b.split_at(b.len().saturating_sub(1));
        let satuan = if h == "B" { "B".to_string() } else { format!("{h}B") };
        format!("{} {satuan}/dtk", angka.replace('.', ","))
    }
    match batas.split_once(':') {
        Some((u, d)) => format!("unggah {}, unduh {}", satu(u), satu(d)),
        None => satu(batas),
    }
}

#[cfg(test)]
mod uji {
    use super::*;

    #[test]
    fn rapikan() {
        assert_eq!(rapikan_sumber("\"D:\\Data\\\""), "D:\\Data");
        assert_eq!(rapikan_sumber("D:"), "D:\\");
        assert_eq!(rapikan_sumber("pd-onedrive:Kerja\\DLH"), "pd-onedrive:Kerja/DLH");
        assert_eq!(normalisasi_tujuan("Arsip/2024"), "pd-gdrive:Arsip/2024");
        assert_eq!(normalisasi_tujuan("pd-gdrive-2:Arsip"), "pd-gdrive-2:Arsip");
        assert_eq!(normalisasi_tujuan("E:\\Salinan"), "E:\\Salinan");
    }

    #[test]
    fn batas_kecepatan() {
        let n = |s: &str| normalisasi_batas(s).unwrap();
        assert_eq!(n("2M"), "2M");
        assert_eq!(n("2 mb"), "2M");
        assert_eq!(n("2MiB/s"), "2M");
        assert_eq!(n("1,5m"), "1.5M");
        assert_eq!(n("500k"), "500K");
        assert_eq!(n("1024"), "1024K");
        assert_eq!(n("800B"), "800B");
        assert_eq!(n("10M:5M"), "10M:5M");
        assert_eq!(n("10M:0"), "10M:off");
        assert_eq!(n("1,5M:500k"), "1.5M:500K");
        for off in ["", "off", "OFF", "0", "tanpa", "0:0"] {
            assert_eq!(n(off), "off", "{off}");
        }
        assert!(normalisasi_batas("cepat").is_err());
        assert!(normalisasi_batas("5X").is_err());
        assert!(normalisasi_batas("08:00,512k").is_err());
        assert_eq!(tampil_batas("5M"), "5 MB/dtk");
        assert_eq!(tampil_batas("1.5M"), "1,5 MB/dtk");
        assert_eq!(tampil_batas("800B"), "800 B/dtk");
        assert_eq!(tampil_batas("10M:off"), "unggah 10 MB/dtk, unduh tanpa batas");
        assert_eq!(tampil_batas("off"), "tanpa batas");
    }

    #[test]
    fn bawaan() {
        assert_eq!(tujuan_bawaan("pd-onedrive:Kerja/DLH", "pd-gdrive"), "pd-gdrive:Pindahan OneDrive/DLH");
        assert_eq!(tujuan_bawaan("pd-gdrive:", "pd-onedrive"), "pd-onedrive:Pindahan Google Drive/Google Drive");
        assert_eq!(tujuan_bawaan("pd-sharepoint:Arsip", "pd-gdrive"), "pd-gdrive:Pindahan SharePoint/Arsip");
    }
}
