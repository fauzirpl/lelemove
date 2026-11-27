//! Lokasi data, keluaran teks, format angka, dan baca-tulis JSON.

use serde_json::{Map, Value};
use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

pub const REMOTE_GOOGLE: &str = "pd-gdrive";
pub const REMOTE_ONEDRIVE: &str = "pd-onedrive";
pub const REMOTE_SHAREPOINT: &str = "pd-sharepoint";

/// File sistem OneDrive/Windows yang tidak perlu ikut disalin.
#[cfg(windows)]
pub const KECUALIAN: &[&str] = &[
    "desktop.ini",
    "Thumbs.db",
    "~$*",
    ".849C9593-D756-4E56-8D6E-42412F2A707B",
    "Personal Vault.lnk",
];

/// Di macOS ditambah berkas catatan Finder (.DS_Store, .localized) dan berkas AppleDouble (._*)
/// yang dibuat Finder di disk berformat non-Mac.
#[cfg(target_os = "macos")]
pub const KECUALIAN: &[&str] = &[
    "desktop.ini",
    "Thumbs.db",
    "~$*",
    ".849C9593-D756-4E56-8D6E-42412F2A707B",
    "Personal Vault.lnk",
    ".DS_Store",
    ".localized",
    "._*",
];

/// FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS (0x400000) | FILE_ATTRIBUTE_OFFLINE (0x1000):
/// penanda file OneDrive yang "hanya online" (belum ada isinya di disk).
pub const ATRIBUT_HANYA_CLOUD: u32 = 0x401000;

pub const KB: f64 = 1024.0;
pub const MB: f64 = KB * 1024.0;
pub const GB: f64 = MB * 1024.0;
pub const TB: f64 = GB * 1024.0;

// ------------------------------------------------------------------ lokasi

pub struct Jalur {
    pub data: PathBuf,
    pub log: PathBuf,
    pub pengaturan: PathBuf,
    pub job: PathBuf,
    pub akun: PathBuf,
    /// Client ID Google bawaan milik pengguna ini.
    pub client_pengguna: PathBuf,
    /// Client ID Google yang dibundel installer (di folder aplikasi).
    pub client_app: PathBuf,
    /// Folder tempat .exe ini berada.
    pub dir_app: PathBuf,
}

pub fn jalur() -> &'static Jalur {
    static J: OnceLock<Jalur> = OnceLock::new();
    J.get_or_init(|| {
        let data = match std::env::var("PINDAH_DRIVE_DATA") {
            Ok(d) if !d.is_empty() => PathBuf::from(d), // untuk pengujian
            // %LOCALAPPDATA%\pindah-drive  /  ~/Library/Application Support/pindah-drive
            _ => crate::win::folder_data().join("pindah-drive"),
        };
        let dir_app = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .unwrap_or_default();
        Jalur {
            log: data.join("log"),
            pengaturan: data.join("pengaturan.json"),
            job: data.join("job.json"),
            akun: data.join("akun.json"),
            client_pengguna: data.join("client-google.json"),
            client_app: dir_app.join("client-google.json"),
            dir_app,
            data,
        }
    })
}

pub fn pastikan_dir() {
    let _ = std::fs::create_dir_all(&jalur().log);
}

/// Waktu lokal: (tahun, bulan, tanggal, jam, menit, detik).
pub fn waktu_lokal() -> (u16, u16, u16, u16, u16, u16) {
    let (y, mo, d, h, mi, s, _) = crate::win::jam_lokal();
    (y, mo, d, h, mi, s)
}

/// Hari dalam minggu, waktu lokal: 0 = Minggu ... 6 = Sabtu.
pub fn hari_ini() -> u8 {
    crate::win::jam_lokal().6
}

/// Nama berkas program di folder aplikasi: "pindah" -> pindah.exe (Windows) / pindah (macOS).
pub fn nama_exe(dasar: &str) -> String {
    format!("{dasar}{}", std::env::consts::EXE_SUFFIX)
}

/// "tugas Windows" -> "Tugas Windows".
pub fn kapital(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|a| a.to_uppercase().chain(c).collect()).unwrap_or_default()
}

/// Program mesin (pindah.exe / pindah) di samping program ini.
pub fn exe_mesin() -> PathBuf {
    jalur().dir_app.join(nama_exe("pindah"))
}

/// Berkas log baru: log\<jenis>-yyyyMMdd-HHmmss.<ext>
pub fn path_log(jenis: &str, ext: &str) -> PathBuf {
    pastikan_dir();
    let (y, mo, d, h, mi, s) = waktu_lokal();
    jalur().log.join(format!("{jenis}-{y:04}{mo:02}{d:02}-{h:02}{mi:02}{s:02}.{ext}"))
}

/// "2026-11-23T09:26:30" (format 's' PowerShell, dipakai di job.json).
pub fn sekarang_iso() -> String {
    let (y, mo, d, h, mi, s) = waktu_lokal();
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}")
}

/// "2026-11-23T09:26:30" -> "pukul 09.26, 23 Nov 2026"
pub fn waktu_tampil(iso: &str) -> String {
    const BULAN: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "Mei", "Jun", "Jul", "Agu", "Sep", "Okt", "Nov", "Des"];
    let angka: Vec<u32> = iso
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();
    match angka.as_slice() {
        [y, mo, d, h, mi, ..] if (1..=12).contains(mo) => {
            format!("pukul {h:02}.{mi:02}, {d} {} {y}", BULAN[(*mo - 1) as usize])
        }
        _ => iso.to_string(),
    }
}

// ------------------------------------------------------------------ keluaran

static WARNA: AtomicBool = AtomicBool::new(false);

pub fn atur_warna(aktif: bool) {
    WARNA.store(aktif, Ordering::Relaxed);
}

fn tulis(kode_warna: &str, t: &str) {
    let mut o = std::io::stdout().lock();
    // Kesalahan tulis diabaikan: pembaca (mis. jendela aplikasi) bisa saja sudah ditutup.
    let _ = if WARNA.load(Ordering::Relaxed) && !kode_warna.is_empty() {
        writeln!(o, "\x1b[{kode_warna}m{t}\x1b[0m")
    } else {
        writeln!(o, "{t}")
    };
    let _ = o.flush();
}

pub fn judul(t: &str) {
    tulis("", "");
    tulis("96", &format!("== {t} =="));
}
pub fn ok(t: &str) {
    tulis("92", &format!("[OK]    {t}"));
}
pub fn info(t: &str) {
    tulis("", &format!("        {t}"));
}
pub fn awas(t: &str) {
    tulis("93", &format!("[AWAS]  {t}"));
}
pub fn gagal(t: &str) {
    tulis("91", &format!("[GAGAL] {t}"));
}
pub fn polos(t: &str) {
    tulis("", t);
}
pub fn berwarna(kode: &str, t: &str) {
    tulis(kode, t);
}

pub fn bersihkan_layar() {
    if WARNA.load(Ordering::Relaxed) {
        let mut o = std::io::stdout().lock();
        let _ = write!(o, "\x1b[2J\x1b[H");
        let _ = o.flush();
    } else {
        tulis("", "\n\n");
    }
}

pub fn bisa_bertanya() -> bool {
    std::io::stdin().is_terminal()
}

pub fn baca_baris(tanya: &str) -> String {
    {
        let mut o = std::io::stdout().lock();
        let _ = write!(o, "{tanya}");
        let _ = o.flush();
    }
    let mut s = String::new();
    let _ = std::io::stdin().lock().read_line(&mut s);
    s.trim().to_string()
}

pub fn baca_angka(label: &str, min: i64, max: i64) -> i64 {
    loop {
        let j = baca_baris(&format!("        {label}: "));
        if let Ok(n) = j.parse::<i64>() {
            if n >= min && n <= max {
                return n;
            }
        }
        awas(&format!("Masukkan angka {min} sampai {max}."));
    }
}

// ------------------------------------------------------------------ angka (gaya id-ID)

/// 1234567.891 -> "1.234.567,89" (desimal = 2).
pub fn angka_id(n: f64, desimal: usize) -> String {
    let teks = format!("{:.*}", desimal, n.abs());
    let (bulat, pecahan) = match teks.split_once('.') {
        Some((b, p)) => (b.to_string(), Some(p.to_string())),
        None => (teks.clone(), None),
    };
    let mut kelompok = String::new();
    for (i, c) in bulat.chars().enumerate() {
        if i > 0 && (bulat.len() - i) % 3 == 0 {
            kelompok.push('.');
        }
        kelompok.push(c);
    }
    let mut s = if n < 0.0 { format!("-{kelompok}") } else { kelompok };
    if let Some(p) = pecahan {
        s.push(',');
        s.push_str(&p);
    }
    s
}

pub fn angka(n: i64) -> String {
    angka_id(n as f64, 0)
}

pub fn format_ukuran(b: f64) -> String {
    if b >= TB {
        format!("{} TB", angka_id(b / TB, 2))
    } else if b >= GB {
        format!("{} GB", angka_id(b / GB, 2))
    } else if b >= MB {
        format!("{} MB", angka_id(b / MB, 1))
    } else if b >= KB {
        format!("{} KB", angka_id(b / KB, 0))
    } else {
        format!("{} B", angka_id(b.max(0.0), 0))
    }
}

/// "1.200" + "GiB" (format rclone) -> byte.
pub fn ke_byte(angka: &str, satuan: &str) -> f64 {
    let n: f64 = angka.trim().parse().unwrap_or(0.0);
    match satuan.trim().chars().next().map(|c| c.to_ascii_uppercase()) {
        Some('K') => n * KB,
        Some('M') => n * MB,
        Some('G') => n * GB,
        Some('T') => n * TB,
        Some('P') => n * TB * 1024.0,
        _ => n,
    }
}

// ------------------------------------------------------------------ JSON

/// Membaca JSON (BOM dari Set-Content PowerShell diabaikan). None bila tidak ada/rusak.
pub fn baca_json(p: &Path) -> Option<Value> {
    let s = std::fs::read_to_string(p).ok()?;
    serde_json::from_str(s.trim_start_matches('\u{feff}')).ok()
}

pub fn baca_objek(p: &Path) -> Option<Map<String, Value>> {
    match baca_json(p)? {
        Value::Object(m) => Some(m),
        _ => None,
    }
}

pub fn tulis_json(p: &Path, v: &Value) {
    pastikan_dir();
    if let Ok(s) = serde_json::to_string_pretty(v) {
        let _ = std::fs::write(p, s);
    }
}

/// Nilai JSON apa pun sebagai teks ("" untuk null/tidak ada).
pub fn teks(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}

/// Nilai JSON sebagai bilangan bulat (angka atau teks angka).
pub fn bil(v: Option<&Value>) -> Option<i64> {
    match v {
        Some(Value::Number(n)) => n.as_i64().or_else(|| n.as_f64().map(|f| f as i64)),
        Some(Value::String(s)) => s.trim().parse().ok(),
        _ => None,
    }
}

// ------------------------------------------------------------------ pola nama

/// Pencocokan gaya -like PowerShell: * dan ?, tanpa membedakan huruf besar-kecil.
pub fn cocok_pola(nama: &str, pola: &str) -> bool {
    fn cocok(n: &[char], p: &[char]) -> bool {
        match (p.first(), n.first()) {
            (None, None) => true,
            (None, Some(_)) => false,
            (Some('*'), _) => cocok(n, &p[1..]) || (!n.is_empty() && cocok(&n[1..], p)),
            (Some('?'), Some(_)) => cocok(&n[1..], &p[1..]),
            (Some(a), Some(b)) => a == b && cocok(&n[1..], &p[1..]),
            (Some(_), None) => false,
        }
    }
    let n: Vec<char> = nama.to_lowercase().chars().collect();
    let p: Vec<char> = pola.to_lowercase().chars().collect();
    cocok(&n, &p)
}

pub fn cocok_kecualian(nama: &str) -> bool {
    KECUALIAN.iter().any(|k| cocok_pola(nama, k))
}

pub fn argumen_kecualian() -> Vec<String> {
    KECUALIAN.iter().flat_map(|k| ["--exclude".to_string(), k.to_string()]).collect()
}

/// Vec<String> dari campuran &str dan String.
#[macro_export]
macro_rules! v {
    ($($x:expr),* $(,)?) => { Vec::from([$($x.to_string()),*]) };
}

#[cfg(test)]
mod uji {
    use super::*;

    #[test]
    fn angka_indonesia() {
        assert_eq!(angka_id(1234567.891, 2), "1.234.567,89");
        assert_eq!(angka_id(999.0, 0), "999");
        assert_eq!(angka_id(1000.0, 0), "1.000");
        assert_eq!(format_ukuran(1.5 * GB), "1,50 GB");
    }

    #[test]
    fn pola() {
        assert!(cocok_pola("~$Laporan.docx", "~$*"));
        assert!(cocok_pola("THUMBS.DB", "Thumbs.db"));
        assert!(!cocok_pola("Laporan.docx", "~$*"));
    }
}
