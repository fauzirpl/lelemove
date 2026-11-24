//! Perintah-perintah mesin (sama dengan pindah.ps1), dipanggil dari pindah.exe.
//!
//! Dengan opsi -Mesin, ringkasan hasil dicetak sebagai satu baris "##HASIL {json}" untuk jendela
//! aplikasi; baris lain adalah teks biasa yang masuk ke panel "Log aktivitas".

use regex::Regex;
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::LazyLock;

use super::akun::*;
use super::dasar::*;
use super::filter::{baca_filter, simpan_filter, Filter};
use super::jadwal;
use super::riwayat::{self, Catatan};
use super::job::*;
use super::lanjut;
use super::penyedia::{self, JenisKolom, Penyedia};
use super::rclone::{self, cari_rclone, daftar_remote, panggil, token_kedaluwarsa, Mode};
use super::rute::*;
use crate::win::PerintahExt;
use crate::{notifikasi, v, win};

pub const PERINTAH: &[&str] = &[
    "menu", "cek", "akun", "pasang", "hubungkan", "lepas", "bawaan", "klien", "situs", "pustaka", "periksa", "uji",
    "salin", "status", "berhenti", "verifikasi", "pindahkan", "jadwal", "riwayat", "penyedia", "impor", "bantuan",
];

static RE_FS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"file system for "([^":]+):"#).unwrap());
static RE_UKURAN_DRY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\(size ([\d.]+)\s*([A-Za-z]*)\)").unwrap());
static RE_MASALAH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"ERROR|non file|non-regular|irregular|symlink|Can't transfer|Can't follow").unwrap()
});
static RE_OFFICE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\.(docx|docm|dotx|xlsx|xlsm|xltx|pptx|pptm|potx|vsdx)$").unwrap());
static RE_DRIVE_SISTEM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"PersonalCache|Preservation|Preservasi|Penangguhan|Cache").unwrap());
static RE_DOCUMENTS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"URL:\s*\S+/Documents/?\s*$").unwrap());
static RE_JENIS_DRIVE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\((\w+)\)\s*$").unwrap());
static RE_AWAL_ERROR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\S+\s+\S+\s+ERROR\s*:\s*").unwrap());

// ------------------------------------------------------------------ opsi

#[derive(Clone, Debug)]
pub struct Opsi {
    pub perintah: String,
    /// Folder di komputer (D:\OneDrive\Backup) atau "<akun>:<folder>" (pd-onedrive:Backup).
    pub sumber: Option<String>,
    /// Sama dengan sumber; tanpa nama akun dianggap folder di akun Google utama (pd-gdrive).
    pub tujuan: Option<String>,
    /// Akun untuk hubungkan/lepas/situs/pustaka. Alias: google, onedrive, sharepoint.
    pub akun: String,
    pub jenis: String,
    pub label: Option<String>,
    /// Google: izin baca-tulis penuh, wajib bila akun ini dipakai sebagai SUMBER.
    pub izin_penuh: bool,
    /// Salin di latar belakang (jendela boleh ditutup).
    pub latar: bool,
    /// Lewati pertanyaan konfirmasi (persetujuan sudah diberikan).
    pub ya: bool,
    /// Paksa login ulang.
    pub ulang: bool,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub simpan_bawaan: bool,
    /// hubungkan (Google): login memakai client ID aplikasi (Pengaturan > Client ID Google).
    pub client_aplikasi: bool,
    /// klien: hapus client ID aplikasi yang disimpan di komputer ini. jadwal: hapus jadwal -Id.
    pub hapus: bool,
    /// jadwal: -Tambah (jadwal baru), -Jalankan (dipakai tugas Windows), -Aktifkan / -Nonaktifkan.
    pub tambah: bool,
    pub jalankan: bool,
    pub aktifkan: bool,
    pub nonaktifkan: bool,
    pub tanpa_verifikasi: bool,
    pub id: Option<String>,
    pub nama: Option<String>,
    pub jam: Option<String>,
    pub hari: Option<String>,
    pub cari: Option<String>,
    pub situs_id: Option<String>,
    pub situs_nama: Option<String>,
    pub pustaka: Option<String>,
    pub pustaka_nama: Option<String>,
    /// Untuk jendela aplikasi: cetak ringkasan hasil sebagai "##HASIL {json}".
    pub mesin: bool,
    /// Internal: proses latar belakang yang menjalankan rclone.
    pub log_latar: Option<String>,
    /// Batas kecepatan salin dalam bentuk --bwlimit rclone ("5M"); "off" = tanpa batas.
    /// None: pakai yang tersimpan di pengaturan.json.
    pub batas_kecepatan: Option<String>,
    /// Filter file: -Sertakan "pdf,docx", -Lewati "*.tmp", -Sejak 2026-01-31, -TanpaFilter.
    /// None = pakai yang tersimpan; diberikan = disimpan untuk perintah berikutnya.
    pub sertakan: Option<String>,
    pub lewati: Option<String>,
    pub sejak: Option<String>,
    pub tanpa_filter: bool,
    /// riwayat: -Jumlah N, -Csv <berkas>, -Laporan <berkas.html>.
    pub jumlah: Option<String>,
    pub csv: Option<String>,
    pub laporan: Option<String>,
    /// hubungkan (penyedia selain Google/Microsoft): -Opsi kunci=nilai (boleh berulang) untuk isian
    /// yang tidak rahasia; -RahasiaStdin = isian rahasia dikirim sebagai JSON lewat stdin.
    pub opsi_rclone: Vec<(String, String)>,
    pub rahasia_stdin: bool,
    /// hubungkan -Lanjut -State S [-Jawab X]: menjawab pertanyaan lanjutan rclone (mis. kode 2FA).
    pub lanjut: bool,
    pub state: Option<String>,
    pub jawab: Option<String>,
}

impl Default for Opsi {
    fn default() -> Self {
        Opsi {
            perintah: "menu".into(),
            sumber: None,
            tujuan: None,
            akun: "google".into(),
            jenis: String::new(),
            label: None,
            izin_penuh: false,
            latar: false,
            ya: false,
            ulang: false,
            client_id: None,
            client_secret: None,
            simpan_bawaan: false,
            client_aplikasi: false,
            hapus: false,
            tambah: false,
            jalankan: false,
            aktifkan: false,
            nonaktifkan: false,
            tanpa_verifikasi: false,
            id: None,
            nama: None,
            jam: None,
            hari: None,
            cari: None,
            situs_id: None,
            situs_nama: None,
            pustaka: None,
            pustaka_nama: None,
            mesin: false,
            log_latar: None,
            batas_kecepatan: None,
            sertakan: None,
            lewati: None,
            sejak: None,
            tanpa_filter: false,
            jumlah: None,
            csv: None,
            laporan: None,
            opsi_rclone: Vec::new(),
            rahasia_stdin: false,
            lanjut: false,
            state: None,
            jawab: None,
        }
    }
}

/// Argumen bergaya PowerShell (-Sumber X, -Ya), tanpa membedakan huruf besar-kecil;
/// --sumber X juga diterima.
pub fn urai_argumen<I: IntoIterator<Item = String>>(args: I) -> Result<Opsi, String> {
    let mut o = Opsi::default();
    let mut it = args.into_iter();
    let mut ada_perintah = false;
    while let Some(a) = it.next() {
        if a.starts_with('-') && a.len() > 1 {
            let mentah = a.trim_start_matches('-').to_lowercase();
            let (nama, sisip) = match mentah.split_once(':') {
                Some((n, v)) => (n.to_string(), Some(v.to_string())),
                None => (mentah, None),
            };
            match nama.as_str() {
                "izinpenuh" => o.izin_penuh = true,
                "latar" => o.latar = true,
                "ya" | "y" => o.ya = true,
                "ulang" => o.ulang = true,
                "simpanbawaan" => o.simpan_bawaan = true,
                "clientaplikasi" => o.client_aplikasi = true,
                "hapus" => o.hapus = true,
                "tambah" => o.tambah = true,
                "jalankan" => o.jalankan = true,
                "aktifkan" => o.aktifkan = true,
                "nonaktifkan" => o.nonaktifkan = true,
                "tanpaverifikasi" => o.tanpa_verifikasi = true,
                "tanpafilter" => o.tanpa_filter = true,
                "rahasiastdin" => o.rahasia_stdin = true,
                "lanjut" => o.lanjut = true,
                "mesin" => o.mesin = true,
                "h" | "help" | "?" | "bantuan" => o.perintah = "bantuan".into(),
                _ => {
                    let nilai = match sisip {
                        Some(v) => v,
                        None => it.next().ok_or_else(|| format!("Opsi {a} butuh nilai."))?,
                    };
                    let isi = Some(nilai.clone()).filter(|s| !s.is_empty());
                    match nama.as_str() {
                        "perintah" => o.perintah = nilai.to_lowercase(),
                        "sumber" => o.sumber = isi,
                        "tujuan" => o.tujuan = isi,
                        "akun" => o.akun = isi.unwrap_or_else(|| "google".into()),
                        "jenis" => o.jenis = nilai.to_lowercase(),
                        "label" => o.label = isi,
                        "clientid" => o.client_id = isi,
                        "clientsecret" => o.client_secret = isi,
                        "cari" => o.cari = isi,
                        "situsid" => o.situs_id = isi,
                        "situsnama" => o.situs_nama = isi,
                        "pustaka" => o.pustaka = isi,
                        "pustakanama" => o.pustaka_nama = isi,
                        "loglatar" => o.log_latar = isi,
                        "id" => o.id = isi,
                        "nama" => o.nama = isi,
                        "jam" => o.jam = isi,
                        "hari" => o.hari = isi,
                        "bataskecepatan" => o.batas_kecepatan = Some(normalisasi_batas(&nilai)?),
                        // Boleh kosong: -Sertakan "" menghapus bagian itu dari filter tersimpan.
                        "sertakan" => o.sertakan = Some(nilai),
                        "lewati" => o.lewati = Some(nilai),
                        "sejak" => o.sejak = Some(nilai),
                        "jumlah" => o.jumlah = isi,
                        "csv" => o.csv = isi,
                        "laporan" => o.laporan = isi,
                        "opsi" => {
                            let (k, v) = nilai
                                .split_once('=')
                                .ok_or_else(|| format!("-Opsi harus berbentuk kunci=nilai: {nilai}"))?;
                            o.opsi_rclone.push((k.trim().to_string(), v.to_string()));
                        }
                        "state" => o.state = isi,
                        "jawab" => o.jawab = Some(nilai),
                        _ => return Err(format!("Opsi tidak dikenal: {a}")),
                    }
                }
            }
        } else if !ada_perintah {
            o.perintah = a.to_lowercase();
            ada_perintah = true;
        } else {
            return Err(format!("Argumen tidak dikenal: {a}"));
        }
    }
    if !PERINTAH.contains(&o.perintah.as_str()) {
        return Err(format!("Perintah tidak dikenal: {}", o.perintah));
    }
    if !o.jenis.is_empty() && penyedia::cari(&o.jenis).is_none() {
        return Err(format!("Jenis akun tidak dikenal: {}. Lihat daftarnya: pindah penyedia", o.jenis));
    }
    Ok(o)
}

pub fn ringkas_akun(a: &Akun) -> String {
    let mut t = format!("{} [{}]", a.label, a.nama);
    match a.jenis.as_str() {
        "google" => {
            t += " - Google Drive";
            t += if a.scope == "drive" { ", izin penuh" } else { ", izin terbatas (hanya tujuan)" };
            t += if a.client_sendiri { ", client ID sendiri" } else { ", client ID bersama (sering dibatasi)" };
        }
        "onedrive" => {
            t += " - OneDrive";
            if !a.drive.is_empty() {
                t += &format!(", drive {}", a.drive);
            }
        }
        "sharepoint" => {
            t += " - SharePoint";
            if !a.drive_id.is_empty() {
                t += &format!(", situs {}, pustaka {}", a.situs, a.drive);
            } else {
                t += ", BELUM memilih situs & pustaka";
            }
        }
        _ => {}
    }
    t
}

/// SharePoint mengubah metadata file Office (.docx/.xlsx/...) saat diunggah, sehingga ukuran &
/// checksum di tujuan berbeda dari sumber. Tanpa flag ini rclone menganggapnya rusak dan mengulang.
fn argumen_tujuan(tujuan: &str) -> Vec<String> {
    let rem = remote_dari(tujuan);
    if !rem.is_empty() && jenis_dari_nama(&rem) == "sharepoint" {
        return v!["--ignore-size", "--ignore-checksum"];
    }
    Vec::new()
}

/// Program pemasang rclone milik sistem: winget (Windows) atau Homebrew (macOS).
struct Pemasang {
    exe: PathBuf,
    args: &'static [&'static str],
    ket: &'static str,
    setuju: &'static str,
}

#[cfg(windows)]
const TANPA_PEMASANG: [&str; 2] = [
    "winget tidak tersedia di komputer ini.",
    "Unduh rclone dari https://rclone.org/downloads/ lalu taruh rclone.exe di folder aplikasi ini.",
];

#[cfg(target_os = "macos")]
const TANPA_PEMASANG: [&str; 2] = [
    "Homebrew tidak tersedia di Mac ini.",
    "Unduh rclone untuk macOS dari https://rclone.org/downloads/ lalu taruh berkas rclone di /usr/local/bin.",
];

#[cfg(windows)]
fn pemasang_rclone() -> Option<Pemasang> {
    cari_winget().map(|exe| Pemasang {
        exe,
        args: &[
            "install", "--id", "Rclone.Rclone", "--exact", "--source", "winget",
            "--accept-source-agreements", "--accept-package-agreements", "--disable-interactivity",
        ],
        ket: "Dipasang lewat winget dari repositori resmi winget (paket Rclone.Rclone, sekitar 25 MB).",
        setuju: "Unduh & pasang rclone sekarang (menyetujui ketentuan sumber/paket winget)?",
    })
}

#[cfg(target_os = "macos")]
fn pemasang_rclone() -> Option<Pemasang> {
    ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"].iter().map(PathBuf::from).find(|p| p.is_file()).map(|exe| Pemasang {
        exe,
        args: &["install", "rclone"],
        ket: "Dipasang lewat Homebrew (formula rclone, sekitar 25 MB).",
        setuju: "Unduh & pasang rclone sekarang lewat Homebrew?",
    })
}

#[cfg(windows)]
fn cari_winget() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("PATH") {
        for d in std::env::split_paths(&path) {
            let p = d.join("winget.exe");
            // winget.exe di WindowsApps adalah alias (reparse point): cukup ada, tidak perlu dibaca.
            if p.symlink_metadata().is_ok() {
                return Some(p);
            }
        }
    }
    let p = PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_default()).join("Microsoft\\WindowsApps\\winget.exe");
    p.symlink_metadata().is_ok().then_some(p)
}

fn json_dari(s: &str) -> Value {
    serde_json::from_str(s).unwrap_or(Value::Null)
}

// ------------------------------------------------------------------ notifikasi akhir salin

/// "pd-gdrive-2:Arsip" -> "Google Drive 2: Arsip";  "D:\Data" tetap.
fn nama_tempat(j: &str) -> String {
    let rem = remote_dari(j);
    if rem.is_empty() {
        return j.to_string();
    }
    let jenis = jenis_dari_nama(&rem);
    let dasar = label_bawaan("", jenis);
    let mut label = meta(&rem, "Label");
    if label.is_empty() {
        label = label_bawaan(&rem, jenis);
    } else if !label.starts_with(&dasar) {
        label = format!("{dasar} \u{00B7} {label}");
    }
    let folder = j[rem.len() + 1..].trim_matches('/');
    if folder.is_empty() {
        label
    } else {
        format!("{label}: {folder}")
    }
}

fn potong(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_string();
    }
    format!("{}\u{2026}", s.chars().take(n).collect::<String>().trim_end())
}

/// "369 file baru (5,00 GB)" dari blok statistik terakhir rclone.
fn ringkas_tersalin(s: &Statistik) -> String {
    let n: i64 = s.file.split('/').next().and_then(|x| x.trim().parse().ok()).unwrap_or(0);
    if n == 0 {
        return "Tidak ada file baru: semua sudah ada di tujuan.".into();
    }
    let data = s.data.split('/').next().unwrap_or("").trim();
    let (a, satuan) = data.split_once(' ').unwrap_or((data, ""));
    format!("{} file baru ({}). Buka LELEMOVE untuk verifikasi.", angka(n), format_ukuran(ke_byte(a, satuan)))
}

/// Folder yang menjadi kosong karena isinya dipindahkan ikut dihapus (dari yang terdalam),
/// tanpa menyentuh folder yang memang kosong sejak awal atau folder sumber itu sendiri.
fn folder_induk(relatif: &[String]) -> Vec<String> {
    let mut d: Vec<String> = Vec::new();
    for p in relatif {
        let mut s = p.as_str();
        while let Some(i) = s.rfind('/') {
            s = &s[..i];
            if !d.iter().any(|x| x == s) {
                d.push(s.to_string());
            }
        }
    }
    d.sort_by_key(|x| std::cmp::Reverse(x.matches('/').count()));
    d
}

fn hapus_folder_kosong_lokal(akar: &Path, dihapus: &[PathBuf]) {
    let rel: Vec<String> = dihapus
        .iter()
        .filter(|p| !p.exists())
        .filter_map(|p| p.strip_prefix(akar).ok().map(|r| r.to_string_lossy().replace('\\', "/")))
        .collect();
    for d in folder_induk(&rel) {
        let _ = std::fs::remove_dir(akar.join(d.replace('/', std::path::MAIN_SEPARATOR_STR))); // gagal bila tidak kosong: dibiarkan
    }
}

fn hapus_folder_kosong_cloud(src: &str, dihapus: &[String]) {
    // Satu panggilan rclone per folder; batasi supaya tidak berjalan terlalu lama.
    for d in folder_induk(dihapus).into_iter().take(100) {
        let _ = panggil(&v!["rmdir", gabung_path(src, &d), "--drive-use-trash"], Mode::Tangkap);
    }
}

/// (file, byte) yang tersalin menurut statistik terakhir di log rclone.
fn angka_tersalin(log: &str) -> (i64, i64) {
    let Some(s) = baca_statistik(log) else { return (0, 0) };
    let n: i64 = s.file.split('/').next().and_then(|x| x.trim().parse().ok()).unwrap_or(0);
    let data = s.data.split('/').next().unwrap_or("").trim();
    let (a, satuan) = data.split_once(' ').unwrap_or((data, ""));
    (n, ke_byte(a, satuan) as i64)
}

/// "12 file baru (1,20 GB)." atau "Tidak ada file baru." (untuk backup terjadwal).
fn ringkas_baru(s: &Statistik) -> String {
    let n: i64 = s.file.split('/').next().and_then(|x| x.trim().parse().ok()).unwrap_or(0);
    if n == 0 {
        return "Tidak ada file baru.".into();
    }
    let data = s.data.split('/').next().unwrap_or("").trim();
    let (a, satuan) = data.split_once(' ').unwrap_or((data, ""));
    format!("{} file baru ({}).", angka(n), format_ukuran(ke_byte(a, satuan)))
}

/// Judul & isi notifikasi saat penyalinan latar berakhir (selesai atau berhenti karena galat).
/// Menambah satu baris ke log rclone (format waktu sama dengan rclone).
fn tulis_baris_log(log: &str, t: &str) {
    use std::io::Write;
    let (y, mo, d, h, mi, s) = waktu_lokal();
    if let Ok(mut f) = std::fs::OpenOptions::new().append(true).create(true).open(log) {
        let _ = writeln!(f, "{y:04}/{mo:02}/{d:02} {h:02}:{mi:02}:{s:02} NOTICE: {t}");
    }
}

/// Menunggu paling sedikit `minimal`, lalu sampai semua `host` bisa dihubungi lagi (diuji tiap
/// 15 detik). false bila internet tidak kembali dalam TUNGGU_JARINGAN_MAKS.
fn tunggu_jaringan(minimal: std::time::Duration, host: &[(String, u16)]) -> bool {
    let mulai = std::time::Instant::now();
    std::thread::sleep(minimal);
    loop {
        if host.iter().all(|(h, p)| lanjut::bisa_dihubungi(h, *p)) {
            return true;
        }
        if mulai.elapsed() >= lanjut::TUNGGU_JARINGAN_MAKS {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_secs(15));
    }
}

fn isi_notifikasi(src: &str, dst: &str, log: &str, kode: &Value) -> (String, Vec<String>) {
    let rute = format!("{} \u{2192} {}", nama_tempat(src), nama_tempat(dst));
    let stat = baca_statistik(log);
    if teks(Some(kode)) == "0" {
        let ringkas = stat.as_ref().map(ringkas_tersalin).unwrap_or_else(|| "Buka LELEMOVE untuk verifikasi.".into());
        return ("Penyalinan selesai".into(), vec![rute, ringkas]);
    }
    let ekor = baca_ekor(Path::new(log), 50).join("\n");
    if ekor.contains("upload limit") || ekor.contains("uploadLimitExceeded") {
        return (
            "Penyalinan berhenti: batas harian Google".into(),
            vec![rute, "Batas unggah 750 GB per hari tercapai. Lanjutkan besok; file yang sudah sampai dilewati.".into()],
        );
    }
    let detail = if token_kedaluwarsa(&ekor) || stat.as_ref().is_some_and(|s| s.perlu_login) {
        "Login salah satu akun kedaluwarsa. Buka LELEMOVE, klik Login ulang, lalu Lanjutkan.".to_string()
    } else {
        match baris_error(log, 1).pop() {
            Some(e) => potong(&RE_AWAL_ERROR.replace(&e, ""), 160),
            None => format!("rclone berhenti dengan kode {}. Buka LELEMOVE untuk melanjutkan.", teks(Some(kode))),
        }
    };
    ("Penyalinan berhenti karena galat".into(), vec![rute, detail])
}

// ------------------------------------------------------------------ mesin

pub struct Mesin {
    pub o: Opsi,
    pub hasil: Map<String, Value>,
    /// Penyalinan ini dijalankan oleh jadwal: notifikasi "selesai" dikirim setelah verifikasi.
    dari_jadwal: bool,
    /// Nama jadwal yang sedang dijalankan (dicatat di riwayat).
    nama_jadwal: String,
    /// Isian rahasia dari stdin (-RahasiaStdin), dibaca sekali.
    rahasia: Option<Map<String, Value>>,
}

impl Mesin {
    pub fn baru(o: Opsi) -> Self {
        Mesin { o, hasil: Map::new(), dari_jadwal: false, nama_jadwal: String::new(), rahasia: None }
    }

    /// Filter file untuk perintah ini: yang tersimpan, atau dari -Sertakan/-Lewati/-Sejak
    /// (lalu disimpan, seperti -Sumber/-Tujuan). -TanpaFilter menghapusnya. None = isian salah.
    fn ambil_filter(&mut self) -> Option<Filter> {
        if self.o.tanpa_filter {
            simpan_filter(&Filter::default());
            return Some(Filter::default());
        }
        let lama = baca_filter();
        if self.o.sertakan.is_none() && self.o.lewati.is_none() && self.o.sejak.is_none() {
            return Some(lama);
        }
        let s = self.o.sertakan.clone().unwrap_or_else(|| lama.teks_sertakan());
        let l = self.o.lewati.clone().unwrap_or_else(|| lama.teks_lewati());
        let t = self.o.sejak.clone().unwrap_or_else(|| lama.sejak.clone().unwrap_or_default());
        match Filter::urai(&s, &l, &t) {
            Ok(f) => {
                simpan_filter(&f);
                Some(f)
            }
            Err(e) => {
                gagal(&e);
                None
            }
        }
    }

    fn info_filter(&mut self, f: &Filter) {
        self.catat("Filter", f.uraian());
        if !f.kosong() {
            info(&format!("Filter: {}", f.uraian()));
        }
    }

    fn catat(&mut self, k: &str, v: impl Into<Value>) {
        self.hasil.insert(k.to_string(), v.into());
    }

    pub fn cetak_hasil(&self) {
        if self.o.mesin && !self.hasil.is_empty() {
            polos(&format!("##HASIL {}", Value::Object(self.hasil.clone())));
        }
    }

    fn konfirmasi(&self, tanya: &str) -> bool {
        if self.o.ya {
            return true;
        }
        if !bisa_bertanya() {
            awas(&format!("Butuh persetujuan: {tanya}"));
            info("Ulangi perintah dengan tambahan -Ya setelah disetujui.");
            return false;
        }
        let j = baca_baris(&format!("        {tanya} (y/n) ")).to_lowercase();
        matches!(j.as_str(), "y" | "ya" | "yes")
    }

    fn wajib_rclone(&self) -> bool {
        if cari_rclone().is_some() {
            return true;
        }
        gagal("rclone belum terpasang. Jalankan: pindah pasang");
        false
    }

    pub fn jalankan(&mut self) -> i32 {
        match self.o.perintah.clone().as_str() {
            "menu" => {
                self.menu();
                0
            }
            "cek" => self.cek(),
            "akun" => self.perintah_akun(),
            "pasang" => self.pasang(),
            "hubungkan" => self.hubungkan(),
            "lepas" => self.lepas(),
            "bawaan" => self.bawaan(),
            "klien" => self.klien(),
            "jadwal" => self.jadwal(),
            "situs" => self.situs(),
            "pustaka" => self.pustaka(),
            "periksa" => self.periksa(),
            "uji" => self.uji(),
            "salin" => self.salin(),
            "status" => self.status(),
            "berhenti" => self.berhenti(),
            "verifikasi" => self.verifikasi(),
            "pindahkan" => self.pindahkan(),
            "riwayat" => self.riwayat(),
            "penyedia" => self.daftar_penyedia(),
            "impor" => self.impor(),
            _ => {
                tampilkan_bantuan();
                0
            }
        }
    }

    /// Bila galat rclone berarti login kedaluwarsa: beri tahu akun mana yang perlu login ulang.
    fn saran_login(&mut self, err: &str, jalur_terkait: &[&str]) {
        if !token_kedaluwarsa(err) {
            return;
        }
        let rem = RE_FS.captures(err).map(|c| c[1].to_string()).unwrap_or_else(|| {
            jalur_terkait.iter().map(|j| remote_dari(j)).find(|r| !r.is_empty()).unwrap_or_default()
        });
        if rem.is_empty() {
            return;
        }
        awas(&format!("Login akun {rem} sudah kedaluwarsa atau dicabut."));
        info(&format!("Login ulang: di aplikasi klik kanan akun itu di daftar kiri > Login ulang, atau jalankan: pindah hubungkan -Akun {rem} -Ulang"));
        self.catat("PerluLogin", rem);
    }

    // -------------------------------------------------------------- rute

    /// Menggabungkan -Sumber/-Tujuan dari baris perintah dengan pengaturan tersimpan.
    fn ambil_rute(&self) -> Rute {
        let p = baca_pengaturan();
        let (mut src, mut dst) = (p.sumber, p.tujuan);
        if let Some(s) = &self.o.sumber {
            let s = rapikan_sumber(s);
            if self.o.tujuan.is_none() {
                dst = Some(tujuan_bawaan(&s, REMOTE_GOOGLE));
            }
            src = Some(s);
        }
        if let Some(t) = &self.o.tujuan {
            dst = Some(normalisasi_tujuan(t));
        }
        if let (Some(s), None) = (&src, &dst) {
            dst = Some(tujuan_bawaan(s, REMOTE_GOOGLE));
        }
        if self.o.sumber.is_some() || self.o.tujuan.is_some() {
            simpan_pengaturan(src.as_deref(), dst.as_deref());
        }
        Rute { sumber: src, tujuan: dst }
    }

    fn rute_siap(&self, r: &Rute, perlu_akun: bool) -> bool {
        let Some(src) = r.sumber.as_deref() else {
            gagal("Folder sumber belum dipilih. Tambahkan -Sumber \"<folder>\" atau pilih lewat menu.");
            return false;
        };
        let Some(dst) = r.tujuan.as_deref() else {
            gagal("Folder tujuan belum dipilih. Tambahkan -Tujuan \"<akun>:<folder>\".");
            return false;
        };
        if adalah_lokal(src) && !Path::new(src).is_dir() {
            gagal(&format!("Folder sumber tidak ditemukan: {src}"));
            return false;
        }
        // Menyalin ke dalam dirinya sendiri akan berulang tanpa akhir.
        let s = src.replace('\\', "/").trim_end_matches('/').to_lowercase();
        let t = dst.replace('\\', "/").trim_end_matches('/').to_lowercase();
        let awalan = if s.ends_with(':') { s.clone() } else { format!("{s}/") };
        if t == s || t.starts_with(&awalan) {
            gagal("Tujuan tidak boleh sama dengan sumber atau berada di dalam folder sumber.");
            return false;
        }
        if perlu_akun {
            if !self.wajib_rclone() {
                return false;
            }
            let remote = daftar_remote();
            for (peran, j) in [("sumber", src), ("tujuan", dst)] {
                let rem = remote_dari(j);
                if !rem.is_empty() && !remote.contains(&rem) {
                    gagal(&format!("Akun '{rem}' untuk {peran} belum terhubung. Jalankan: pindah hubungkan -Akun {rem}"));
                    return false;
                }
            }
            for j in [src, dst] {
                let rem = remote_dari(j);
                if !rem.is_empty() && jenis_dari_nama(&rem) == "sharepoint" && meta(&rem, "DriveId").is_empty() {
                    gagal(&format!(
                        "Akun SharePoint '{rem}' belum memilih situs & pustaka. Jalankan: pindah situs -Akun {rem} -Cari <kata>"
                    ));
                    return false;
                }
            }
            let rem_s = remote_dari(src);
            if !rem_s.is_empty() {
                let k = konfig_akun(&rem_s);
                if k.tipe == "drive" && k.scope == "drive.file" {
                    gagal(&format!(
                        "Akun Google '{rem_s}' hanya diberi izin terbatas (drive.file), jadi tidak bisa membaca file Anda."
                    ));
                    info(&format!("Beri izin penuh dulu: pindah hubungkan -Akun {rem_s} -IzinPenuh"));
                    return false;
                }
            }
        }
        true
    }

    // -------------------------------------------------------------- cek, akun, pasang

    fn cek(&mut self) -> i32 {
        judul("Kesiapan");
        if let Some(rc) = cari_rclone() {
            ok(&format!("rclone terpasang: {}", rclone::versi()));
            info(&format!("lokasi: {}", rc.display()));
            judul("Akun cloud");
            let akun = daftar_akun();
            if akun.is_empty() {
                awas("Belum ada. Contoh: pindah hubungkan -Akun google");
            }
            for a in &akun {
                ok(&ringkas_akun(a));
            }
        } else {
            awas("rclone belum terpasang. Jalankan: pindah pasang");
        }
        judul("Folder OneDrive di komputer ini");
        let od = daftar_onedrive();
        if od.is_empty() {
            awas("Tidak ditemukan. Pakai OneDrive lewat internet: pindah hubungkan -Akun onedrive");
        }
        for o in od {
            info(&format!("[{}] {}", o.jenis, o.folder));
        }
        judul("Rute penyalinan");
        let r = self.ambil_rute();
        match (&r.sumber, &r.tujuan) {
            (Some(s), t) => {
                info(&format!("Sumber : {s}"));
                info(&format!("Tujuan : {}", t.clone().unwrap_or_default()));
            }
            _ => info(&format!("Belum dipilih. Contoh: pindah periksa -Sumber \"{}\"", win::CONTOH_DATA)),
        }
        if let Some(job) = baca_job() {
            if job_berjalan(&job) {
                awas(&format!("Penyalinan sedang berjalan sejak {}. Lihat: pindah status", teks(job.get("Mulai"))));
            }
        }
        0
    }

    fn perintah_akun(&mut self) -> i32 {
        judul("Akun cloud yang terhubung");
        if !self.wajib_rclone() {
            return 1;
        }
        let akun = daftar_akun();
        if akun.is_empty() {
            info("Belum ada. Contoh: pindah hubungkan -Akun google");
        }
        for a in &akun {
            info(&ringkas_akun(a));
        }
        self.catat("Akun", serde_json::to_value(&akun).unwrap_or(Value::Null));
        0
    }

    fn pasang(&mut self) -> i32 {
        judul("Memasang rclone");
        if let Some(rc) = cari_rclone() {
            ok(&format!("rclone sudah terpasang: {}", rc.display()));
            return 0;
        }
        let Some(p) = pemasang_rclone() else {
            gagal(TANPA_PEMASANG[0]);
            info(TANPA_PEMASANG[1]);
            return 1;
        };
        info("rclone adalah program open source (lisensi MIT) untuk menyalin file antar-layanan cloud.");
        info(p.ket);
        if !self.konfirmasi(p.setuju) {
            return 1;
        }
        let _ = Command::new(&p.exe).args(p.args).stdin(Stdio::null()).tanpa_jendela().status();
        if let Some(rc) = cari_rclone() {
            ok(&format!("rclone terpasang: {}", rc.display()));
            return 0;
        }
        gagal("Pemasangan tidak berhasil. Tutup lalu buka lagi jendela ini, atau pasang manual dari https://rclone.org/downloads/");
        1
    }

    // -------------------------------------------------------------- hubungkan

    /// Menghubungkan (atau menyambung ulang) satu akun. Akun baru: -Akun <nama-baru> -Jenis google|onedrive|sharepoint.
    fn hubungkan(&mut self) -> i32 {
        if !self.wajib_rclone() {
            return 1;
        }
        let nama = nama_remote(&self.o.akun);
        if self.o.lanjut {
            return self.jawab_konfig(&nama);
        }
        let ada = daftar_remote().contains(&nama);
        let mut konf: Option<Konfig> = None;
        let mut jenis = self.o.jenis.clone();
        if ada {
            let k = konfig_akun(&nama);
            if k.tipe == "drive" {
                jenis = "google".into();
            } else if k.tipe == "onedrive" {
                jenis = if jenis_dari_nama(&nama) == "sharepoint" { "sharepoint" } else { "onedrive" }.into();
            }
            konf = Some(k);
        }
        if jenis.is_empty() {
            jenis = jenis_dari_nama(&nama).into();
        }
        if jenis.is_empty() && ada {
            jenis = jenis_konfig(&nama);
        }
        let Some(p) = penyedia::cari(&jenis) else {
            gagal(&format!("Jenis akun '{nama}' tidak dikenal. Tambahkan -Jenis <jenis>; lihat: pindah penyedia"));
            return 1;
        };
        self.catat("Akun", nama.clone());
        let mut label = self.o.label.clone().unwrap_or_default();
        if label.is_empty() {
            label = meta(&nama, "Label");
        }
        if label.is_empty() {
            label = label_bawaan(&nama, &jenis);
        }
        if !p.khusus {
            return self.hubungkan_penyedia(&nama, p, ada, &label);
        }

        let mut pesan: Vec<String> = Vec::new();
        let uji: Vec<String>;
        let paksa: bool;
        let mut arg_buat: Vec<String>;
        let mut arg_ulang: Option<Vec<String>> = None;

        if jenis == "google" {
            judul(&format!("Menghubungkan Google Drive: {label}"));
            let mut scope = konf.as_ref().map(|k| k.scope.clone()).filter(|s| !s.is_empty()).unwrap_or("drive.file".into());
            if self.o.izin_penuh {
                scope = "drive".into();
            }
            pesan.push("Browser akan terbuka. Login ke akun Google yang dimaksud, lalu klik \"Izinkan\"/\"Allow\".".into());
            pesan.push(if scope == "drive" {
                "Izin yang diminta: baca-tulis penuh (dibutuhkan untuk menyalin DARI akun ini).".into()
            } else {
                "Izin yang diminta hanya \"drive.file\": rclone hanya bisa melihat file yang ia buat sendiri.".into()
            });
            if let (Some(id), true) = (&self.o.client_id, self.o.simpan_bawaan) {
                simpan_client_bawaan(id, self.o.client_secret.as_deref().unwrap_or(""));
                ok("Client ID disimpan sebagai bawaan aplikasi.");
            }
            // -ClientAplikasi: login ulang dengan client ID aplikasi. Secret dibaca di dalam proses ini
            // (tidak lewat baris perintah), lalu koneksi dibuat ulang seperti -ClientId.
            if self.o.client_aplikasi && self.o.client_id.is_none() {
                match baca_client_bawaan() {
                    Some((i, s)) => {
                        self.o.client_id = Some(i);
                        self.o.client_secret = Some(s);
                        pesan.push("Memakai client ID aplikasi (Pengaturan > Client ID Google).".into());
                    }
                    None => {
                        gagal("Belum ada client ID aplikasi. Pasang dulu lewat Pengaturan > Client ID Google, atau: pindah klien -ClientId X -ClientSecret Y");
                        return 1;
                    }
                }
            }
            let client_lama = konf.as_ref().is_some_and(|k| k.client_sendiri);
            let (mut cid, mut csec) = (self.o.client_id.clone(), self.o.client_secret.clone());
            if cid.is_none() && !client_lama {
                if let Some((i, s)) = baca_client_bawaan() {
                    cid = Some(i);
                    csec = Some(s);
                    pesan.push("Memakai client ID bawaan aplikasi.".into());
                }
            }
            if cid.is_none() && !client_lama {
                pesan.push("Catatan: memakai client ID bersama rclone, yang sedang dibatasi Google dan bisa membuat penyalinan macet.".into());
            }
            uji = v!["about", format!("{nama}:"), "--json"];
            let ganti_izin = ada && self.o.izin_penuh && konf.as_ref().map_or(true, |k| k.scope != "drive");
            paksa = self.o.ulang || self.o.client_id.is_some() || ganti_izin;
            arg_buat = v!["config", "create", &nama, "drive", format!("scope={scope}")];
            if let Some(i) = &cid {
                arg_buat.push(format!("client_id={i}"));
                arg_buat.push(format!("client_secret={}", csec.clone().unwrap_or_default()));
            }
            // Client ID sendiri yang lama: perbarui di tempat agar secret-nya (yang tidak bisa dibaca
            // kembali dari rclone) tetap tersimpan. Selain itu: buat ulang.
            if client_lama && self.o.client_id.is_none() {
                arg_ulang = Some(v!["config", "update", &nama, format!("scope={scope}")]);
            }
            if ada && !paksa && !client_lama && cid.is_some() {
                info("Akun ini masih memakai client ID bersama. Jalankan dengan -Ulang untuk beralih ke client ID bawaan.");
            }
        } else {
            if jenis == "sharepoint" {
                judul(&format!("Menghubungkan SharePoint: {label}"));
                pesan.push("Browser akan terbuka. Login ke akun Microsoft kantor/sekolah yang punya akses ke situs SharePoint.".into());
                pesan.push(format!("Setelah itu pilih situsnya: pindah situs -Akun {nama} -Cari <kata>"));
            } else {
                judul(&format!("Menghubungkan OneDrive: {label}"));
                pesan.push("Browser akan terbuka. Login ke akun Microsoft pemilik OneDrive ini.".into());
                pesan.push("Sebagian akun kantor/instansi ditolak dengan pesan \"Need admin approval\";".into());
                pesan.push("bila begitu, pakai folder OneDrive di komputer saja.".into());
            }
            uji = v!["lsf", format!("{nama}:"), "--max-depth", "1", "--dirs-only"];
            paksa = self.o.ulang;
            arg_buat = v!["config", "create", &nama, "onedrive"];
        }

        if ada && !paksa {
            let t = panggil(&uji, Mode::Tangkap);
            if t.kode == 0 {
                ok("Sudah terhubung dan bisa diakses.");
                if self.o.label.is_some() {
                    simpan_meta_akun(&nama, &[("Label", &label)], false);
                }
                match jenis.as_str() {
                    "onedrive" => self.pilih_drive_onedrive(&nama),
                    "sharepoint" => {
                        self.pasang_pustaka_tersimpan(&nama);
                    }
                    _ => {}
                }
                return 0;
            }
            awas("Koneksi tersimpan tidak bisa dipakai lagi:");
            for b in t.galat_terakhir(3) {
                info(&b);
            }
            if !self.konfirmasi("Login ulang sekarang?") {
                return 1;
            }
        }

        for p in &pesan {
            info(p);
        }
        info("Setelah browser menampilkan \"Success!\", kembali ke jendela ini.");
        polos("");
        // stdout dibuang supaya token login tidak tercetak di layar/log.
        let h = match (&arg_ulang, ada) {
            (Some(a), true) => panggil(a, Mode::BuangStdout),
            _ => {
                if ada {
                    // Hanya menghapus entri koneksi di rclone.conf; tidak menyentuh file apa pun.
                    let _ = panggil(&v!["config", "delete", &nama], Mode::Tangkap);
                }
                panggil(&arg_buat, Mode::BuangStdout)
            }
        };
        if h.kode != 0 {
            gagal(&format!("Login tidak selesai (kode {}).", h.kode));
            return 1;
        }
        simpan_meta_akun(&nama, &[("Label", &label), ("Jenis", &jenis)], false);

        let t = panggil(&uji, Mode::Tangkap);
        if t.kode == 0 {
            ok("Terhubung.");
            match jenis.as_str() {
                "onedrive" => self.pilih_drive_onedrive(&nama),
                "sharepoint" => {
                    self.pasang_pustaka_tersimpan(&nama);
                }
                _ => {}
            }
            return 0;
        }
        gagal("Login tersimpan tetapi akses gagal:");
        for b in t.galat_terakhir(3) {
            info(&b);
        }
        1
    }

    // -------------------------------------------------------------- penyedia lain

    /// Isian rahasia dari stdin (-RahasiaStdin): objek JSON {"pass": "...", "Jawab": "..."}.
    /// Dibaca sekali; tidak pernah dicetak.
    fn baca_rahasia(&mut self) -> Map<String, Value> {
        if !self.o.rahasia_stdin {
            return Map::new();
        }
        if self.rahasia.is_none() {
            use std::io::Read;
            let mut s = String::new();
            let _ = std::io::stdin().read_to_string(&mut s);
            self.rahasia = Some(serde_json::from_str(s.trim_start_matches('\u{feff}')).unwrap_or_default());
        }
        self.rahasia.clone().unwrap_or_default()
    }

    /// Penyedia umum (penyedia.rs): login lewat browser, atau isian kredensial lalu pertanyaan
    /// lanjutan dari rclone bila ada (mis. kode 2FA). Kata sandi diobfuskasi lewat stdin rclone
    /// sebelum masuk argumen `config create`.
    fn hubungkan_penyedia(&mut self, nama: &str, p: &'static Penyedia, ada: bool, label: &str) -> i32 {
        judul(&format!("Menghubungkan {}: {label}", p.nama));
        let isian_baru = !self.o.opsi_rclone.is_empty() || self.o.rahasia_stdin;
        if ada && !self.o.ulang && !isian_baru {
            match self.uji_akses(nama) {
                Ok(()) => {
                    ok("Sudah terhubung dan bisa diakses.");
                    if self.o.label.is_some() {
                        simpan_meta_akun(nama, &[("Label", label)], false);
                    }
                    return 0;
                }
                Err(e) => {
                    awas(&format!("Koneksi tersimpan tidak bisa dipakai: {e}"));
                    if !p.oauth {
                        info("Isi ulang datanya: Login ulang di aplikasi, atau jalankan hubungkan lagi dengan -Opsi.");
                        return 1;
                    }
                }
            }
        }
        let rahasia = self.baca_rahasia();
        let mut arg = v!["config", "create", nama, p.backend];
        for (k, v) in p.tetap {
            arg.push(format!("{k}={v}"));
        }
        for k in p.kolom {
            let dari_opsi = self.o.opsi_rclone.iter().find(|(x, _)| x == k.kunci).map(|(_, v)| v.clone());
            let mut nilai = dari_opsi.unwrap_or_else(|| teks(rahasia.get(k.kunci)));
            if !k.tersamar() {
                nilai = nilai.trim().to_string();
            }
            if nilai.is_empty() {
                nilai = k.bawaan.to_string();
            }
            if nilai.is_empty() {
                if k.wajib {
                    gagal(&format!("Isi \"{}\".", k.label));
                    return 1;
                }
                continue;
            }
            if k.jenis == JenisKolom::Rahasia {
                match rclone::obscure(&nilai) {
                    Ok(o) => nilai = o,
                    Err(e) => {
                        gagal(&e);
                        return 1;
                    }
                }
            }
            arg.push(format!("{}={nilai}", k.kunci));
        }
        // Opsi lanjutan lain dari -Opsi (mis. no_check_certificate=true untuk server uji).
        for (k, v) in &self.o.opsi_rclone {
            if !p.kolom.iter().any(|c| c.kunci == k) && !p.tetap.iter().any(|(t, _)| t == k) {
                arg.push(format!("{k}={v}"));
            }
        }
        if ada {
            // Hanya menghapus entri koneksi di rclone.conf; tidak menyentuh file apa pun.
            let _ = panggil(&v!["config", "delete", nama], Mode::Tangkap);
        }
        simpan_meta_akun(nama, &[("Label", label), ("Jenis", p.kunci)], false);
        if p.oauth {
            info("Browser akan terbuka. Login ke akun yang dimaksud, lalu izinkan akses rclone.");
            info("Setelah browser menampilkan \"Success!\", kembali ke jendela ini.");
            polos("");
            // stdout dibuang supaya token login tidak tercetak di layar/log.
            let h = panggil(&arg, Mode::BuangStdout);
            if h.kode != 0 {
                gagal(&format!("Login tidak selesai (kode {}).", h.kode));
                self.hapus_akun_gagal(nama, ada);
                return 1;
            }
            return self.selesai_hubungkan(nama, ada);
        }
        arg.extend(v!["--non-interactive", "--no-obscure"]);
        let h = panggil(&arg, Mode::Tangkap);
        self.langkah_lanjut(nama, ada, &h.out, &h.err)
    }

    /// Jawaban `config create/update --non-interactive`: selesai, galat, atau pertanyaan berikutnya.
    /// Pertanyaan dicatat sebagai "Tanya" (dibaca jendela) dan kode keluar 10.
    fn langkah_lanjut(&mut self, nama: &str, ada: bool, out: &str, err: &str) -> i32 {
        let j: Value = serde_json::from_str(out.trim()).unwrap_or(Value::Null);
        if j.is_null() {
            gagal("rclone tidak memberi jawaban konfigurasi yang dikenali.");
            for b in err.lines().map(str::trim).filter(|b| !b.is_empty() && !b.contains("not found - using defaults")).take(3) {
                info(b);
            }
            self.hapus_akun_gagal(nama, ada);
            return 1;
        }
        let e = teks(j.get("Error"));
        let state = teks(j.get("State"));
        let opsi = j.get("Option").filter(|o| !o.is_null());
        if !e.is_empty() && opsi.is_none() {
            gagal(&e);
            self.hapus_akun_gagal(nama, ada);
            return 1;
        }
        if let (false, Some(o)) = (state.is_empty(), opsi) {
            let bantuan = teks(o.get("Help"));
            awas(&format!("rclone bertanya: {}", bantuan.lines().next().unwrap_or("")));
            if !e.is_empty() {
                info(&format!("Jawaban sebelumnya ditolak: {e}"));
            }
            self.catat(
                "Tanya",
                json!({
                    "Akun": nama,
                    "State": state,
                    "Nama": teks(o.get("Name")),
                    "Bantuan": bantuan,
                    "Bawaan": teks(o.get("DefaultStr")),
                    "Rahasia": o.get("IsPassword") == Some(&Value::Bool(true)),
                    "Wajib": o.get("Required") == Some(&Value::Bool(true)),
                    "Galat": e,
                    "Pilihan": contoh_pilihan(&j).into_iter().map(|(v, h)| json!({"Nilai": v, "Ket": h})).collect::<Vec<_>>(),
                    "Baru": !ada,
                }),
            );
            info(&format!("Jawab dengan: pindah hubungkan -Akun {nama} -Lanjut -State \"{state}\" -Jawab <jawaban>"));
            return 10;
        }
        self.selesai_hubungkan(nama, ada)
    }

    /// -Lanjut: menjawab satu pertanyaan konfigurasi rclone (jawaban rahasia lewat stdin "Jawab").
    fn jawab_konfig(&mut self, nama: &str) -> i32 {
        let Some(state) = self.o.state.clone() else {
            gagal("Tambahkan -State dari pertanyaan sebelumnya.");
            return 1;
        };
        let rahasia = self.baca_rahasia();
        let mut jawab = self.o.jawab.clone().unwrap_or_else(|| teks(rahasia.get("Jawab")));
        // Jawaban kata sandi diobfuskasi dulu; rclone mengenali dan menyimpannya apa adanya.
        if rahasia.get("Rahasia") == Some(&Value::Bool(true)) && !jawab.is_empty() {
            match rclone::obscure(&jawab) {
                Ok(o) => jawab = o,
                Err(e) => {
                    gagal(&e);
                    return 1;
                }
            }
        }
        self.catat("Akun", nama);
        let baru = teks(rahasia.get("Baru")) == "true" || rahasia.get("Baru") == Some(&Value::Bool(true));
        let h = panggil(
            &v!["config", "update", nama, "--continue", "--non-interactive", "--no-obscure", "--state", state, "--result", jawab],
            Mode::Tangkap,
        );
        self.langkah_lanjut(nama, !baru, &h.out, &h.err)
    }

    /// Akun baru yang gagal tersambung dihapus lagi supaya tidak menumpuk akun rusak.
    fn hapus_akun_gagal(&mut self, nama: &str, ada: bool) {
        if !ada {
            let _ = panggil(&v!["config", "delete", nama], Mode::Tangkap);
            simpan_meta_akun(nama, &[], true);
        }
    }

    /// Uji akses ringan: daftar folder teratas.
    fn uji_akses(&self, nama: &str) -> Result<(), String> {
        let h = panggil(
            &v!["lsf", format!("{nama}:"), "--max-depth", "1", "--dirs-only", "--contimeout", "20s", "--timeout", "40s", "--low-level-retries", "2"],
            Mode::Tangkap,
        );
        if h.kode == 0 {
            return Ok(());
        }
        Err(h.galat_terakhir(1).join(" "))
    }

    fn selesai_hubungkan(&mut self, nama: &str, ada: bool) -> i32 {
        match self.uji_akses(nama) {
            Ok(()) => {
                ok("Terhubung.");
                0
            }
            Err(e) => {
                let e_kecil = e.to_lowercase();
                // Kunci object storage yang tidak boleh melihat daftar bucket tetap sah dipakai.
                // Kunci/secret yang salah juga 403, tetapi dengan kode galat yang lain.
                let kunci_salah = ["signaturedoesnotmatch", "invalidaccesskeyid", "invalidclienttokenid", "401", "unauthorized", "authenticationfailed"]
                    .iter()
                    .any(|t| e_kecil.contains(t));
                let terbatas = !kunci_salah && ["accessdenied", "access denied"].iter().any(|t| e_kecil.contains(t));
                if terbatas {
                    awas("Tersambung, tetapi akun ini tidak boleh melihat daftar folder teratas.");
                    info("Ketik nama bucket/folder langsung di bilah alamat panel, mis. nama-bucket/arsip.");
                    return 0;
                }
                gagal(&format!("Tersimpan tetapi akses gagal: {e}"));
                self.hapus_akun_gagal(nama, ada);
                1
            }
        }
    }

    /// Daftar penyedia yang bisa ditambahkan (dan opsi isiannya).
    fn daftar_penyedia(&mut self) -> i32 {
        judul("Penyedia yang bisa ditambahkan");
        for kel in [penyedia::Kelompok::Cloud, penyedia::Kelompok::Server, penyedia::Kelompok::Objek] {
            info("");
            ok(kel.judul());
            for p in penyedia::SEMUA.iter().filter(|p| p.kelompok == kel) {
                let login = if p.oauth {
                    "login di browser".to_string()
                } else {
                    p.kolom
                        .iter()
                        .map(|k| {
                            let r = if k.tersamar() { " (rahasia)" } else { "" };
                            format!("{}{}{r}", k.kunci, if k.wajib { "*" } else { "" })
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                let beta = if p.beta { " [beta]" } else { "" };
                info(&format!("  -Jenis {:<11} {}{beta}: {login}", p.kunci, p.nama));
            }
        }
        info("");
        info("Contoh: pindah hubungkan -Akun pd-sftp -Jenis sftp -Opsi host=nas.kantor -Opsi user=admin -RahasiaStdin");
        info("        (isian rahasia lewat stdin sebagai JSON: {\"pass\": \"...\"})");
        info("Remote rclone yang sudah ada: pindah impor [-Akun <nama>]");
        let semua: Vec<Value> = penyedia::SEMUA
            .iter()
            .map(|p| json!({"Jenis": p.kunci, "Nama": p.nama, "Backend": p.backend, "Oauth": p.oauth, "Beta": p.beta}))
            .collect();
        self.catat("Penyedia", semua);
        0
    }

    /// Memakai remote rclone yang sudah ada (dibuat di luar LELEMOVE). Tanpa -Akun: daftar calon.
    fn impor(&mut self) -> i32 {
        if !self.wajib_rclone() {
            return 1;
        }
        let calon = remote_luar();
        // -Akun bawaan Opsi adalah "google": berarti tidak diisi.
        let nama = if self.o.akun == "google" { String::new() } else { self.o.akun.trim_end_matches(':').to_string() };
        if nama.is_empty() || !calon.iter().any(|(n, _)| *n == nama) {
            judul("Remote rclone yang bisa dipakai");
            if calon.is_empty() {
                info("Tidak ada. Buat dulu dengan: rclone config");
            }
            for (n, t) in &calon {
                info(&format!("  {n}  ({t})"));
            }
            self.catat("Calon", calon.iter().map(|(n, t)| json!({"Nama": n, "Backend": t})).collect::<Vec<_>>());
            if !nama.is_empty() {
                gagal(&format!("Remote '{nama}' tidak ada atau sudah dipakai."));
                return 1;
            }
            return 0;
        }
        let jenis = jenis_konfig(&nama);
        let label = self.o.label.clone().unwrap_or_else(|| label_bawaan(&nama, &jenis));
        simpan_meta_akun(&nama, &[("Label", &label), ("Jenis", &jenis), ("Impor", "1")], false);
        self.catat("Akun", nama.clone());
        match self.uji_akses(&nama) {
            Ok(()) => ok(&format!("Remote {nama} ({}) dipakai sebagai akun \"{label}\".", nama_layanan(&jenis))),
            Err(e) => awas(&format!("Remote {nama} dipakai, tetapi belum bisa diakses: {e}")),
        }
        0
    }

    /// Akun OneDrive bisnis punya beberapa "drive" (pustaka SharePoint), dan rclone memilih yang
    /// pertama, yang sering berupa pustaka sistem (PersonalCacheLibrary). Nama drive mengikuti
    /// bahasa akun ("Dokumen"/"Documents"), jadi yang dipakai sebagai penanda adalah alamatnya:
    /// OneDrive utama selalu beralamat .../Documents. Memakai token yang sudah tersimpan.
    fn pilih_drive_onedrive(&mut self, remote: &str) {
        let langkah = |state: &str, hasil: &str| -> Option<Value> {
            let h = panggil(
                &v!["config", "update", remote, "--continue", "--non-interactive", "--state", state, "--result", hasil],
                Mode::Tangkap,
            );
            serde_json::from_str(&h.out).ok()
        };
        let daftar: Vec<(String, String)> = langkah("choose_type_done", "onedrive")
            .map(|j| contoh_pilihan(&j))
            .unwrap_or_default()
            .into_iter()
            .filter(|(id, _)| !id.is_empty())
            .collect();
        if daftar.is_empty() {
            awas("Daftar drive OneDrive tidak terbaca; memakai pilihan bawaan rclone.");
            return;
        }
        let mut urut: Vec<(String, String)> = daftar.iter().filter(|(_, h)| !RE_DRIVE_SISTEM.is_match(h)).cloned().collect();
        urut.extend(daftar.iter().filter(|(_, h)| RE_DRIVE_SISTEM.is_match(h)).cloned());
        let mut pilihan = None;
        for d in &urut {
            let Some(j) = langkah("driveid_final", &d.0) else { continue };
            let state = teks(j.get("State"));
            if state.is_empty() {
                continue;
            }
            let help = teks(j.get("Option").and_then(|o| o.get("Help")));
            if daftar.len() == 1 || RE_DOCUMENTS.is_match(&help) {
                let _ = langkah(&state, "true");
                pilihan = Some(d.clone());
                break;
            }
        }
        let pilihan = match pilihan {
            Some(p) => p,
            None => {
                // Tidak ada yang beralamat /Documents: pakai kandidat pertama yang bukan pustaka sistem.
                let p = urut[0].clone();
                if let Some(j) = langkah("driveid_final", &p.0) {
                    let st = teks(j.get("State"));
                    if !st.is_empty() {
                        let _ = langkah(&st, "true");
                    }
                }
                awas(&format!("OneDrive utama tidak dikenali pasti; memakai drive: {}", p.1));
                p
            }
        };
        let nama = tanpa_jenis(&pilihan.1);
        let jenis = RE_JENIS_DRIVE.captures(&pilihan.1).map(|c| c[1].to_string()).unwrap_or_default();
        ok(&format!("Memakai drive OneDrive: {nama} ({jenis})"));
        simpan_meta_akun(remote, &[("Drive", &format!("{nama} ({jenis})"))], false);
        if daftar.len() > 1 {
            let lain: Vec<String> = daftar.iter().filter(|d| d.0 != pilihan.0).map(|d| d.1.clone()).collect();
            info(&format!("Drive lain di akun ini (tidak dipakai): {}", lain.join(", ")));
        }
        self.catat("DriveNama", nama);
        self.catat("DriveJenis", jenis);
    }

    /// Setelah login ulang (koneksi dibuat ulang), pasang lagi pustaka yang dulu dipilih.
    fn pasang_pustaka_tersimpan(&mut self, nama: &str) -> bool {
        let id = meta(nama, "DriveId");
        if id.is_empty() {
            info(&format!("Belum memilih situs & pustaka. Lanjutkan: pindah situs -Akun {nama} -Cari <kata>"));
            return false;
        }
        match atur_pustaka(nama, &id) {
            Ok(()) => {
                ok(&format!("Memakai pustaka: {}", meta(nama, "Drive")));
                true
            }
            Err(e) => {
                gagal(&format!("Pustaka tidak bisa dipakai. {e}"));
                false
            }
        }
    }

    // -------------------------------------------------------------- SharePoint

    fn situs(&mut self) -> i32 {
        if !self.wajib_rclone() {
            return 1;
        }
        let nama = nama_remote(&self.o.akun);
        if !daftar_remote().contains(&nama) {
            gagal(&format!("Akun {nama} belum terhubung."));
            return 1;
        }
        let Some(kata) = self.o.cari.clone() else {
            gagal("Tambahkan kata kunci: -Cari <kata>");
            return 1;
        };
        judul(&format!("Mencari situs SharePoint: {kata}"));
        match cari_situs(&nama, &kata) {
            Ok(situs) => {
                if situs.is_empty() {
                    info("Tidak ada situs yang cocok.");
                }
                for s in &situs {
                    info(&format!("{}  {}", s.nama, s.url));
                }
                self.catat("Situs", serde_json::to_value(&situs).unwrap_or(Value::Null));
                0
            }
            Err(e) => {
                gagal(&format!("Pencarian gagal. {e}"));
                self.catat("Situs", Value::Array(Vec::new()));
                1
            }
        }
    }

    fn pustaka(&mut self) -> i32 {
        if !self.wajib_rclone() {
            return 1;
        }
        let nama = nama_remote(&self.o.akun);
        if !daftar_remote().contains(&nama) {
            gagal(&format!("Akun {nama} belum terhubung."));
            return 1;
        }
        let situs_nama = self.o.situs_nama.clone().unwrap_or_default();
        if let Some(id) = self.o.pustaka.clone() {
            judul("Memakai pustaka SharePoint");
            if let Err(e) = atur_pustaka(&nama, &id) {
                gagal(&format!("Pustaka tidak bisa dipakai. {e}"));
                return 1;
            }
            let t = panggil(&v!["lsf", format!("{nama}:"), "--max-depth", "1"], Mode::Tangkap);
            if t.kode != 0 {
                gagal("Pustaka terpasang tetapi tidak bisa dibaca (periksa hak akses Anda di situs itu).");
                return 1;
            }
            let pustaka_nama = self.o.pustaka_nama.clone().unwrap_or_default();
            let mut ubah: Vec<(&str, String)> = vec![
                ("Jenis", "sharepoint".into()),
                ("DriveId", id.clone()),
                ("Drive", pustaka_nama.clone()),
                ("Situs", situs_nama.clone()),
            ];
            let label_lama = meta(&nama, "Label");
            if label_lama.is_empty() || label_lama.starts_with("SharePoint") {
                let l = format!("{situs_nama} - {pustaka_nama}");
                ubah.push(("Label", l.trim_matches([' ', '-']).to_string()));
            }
            let ubah_ref: Vec<(&str, &str)> = ubah.iter().map(|(k, v)| (*k, v.as_str())).collect();
            simpan_meta_akun(&nama, &ubah_ref, false);
            ok(&format!("Memakai situs \"{situs_nama}\", pustaka \"{pustaka_nama}\"."));
            return 0;
        }
        let Some(situs_id) = self.o.situs_id.clone() else {
            gagal("Tambahkan -SitusId <id> (dari perintah situs) atau -Pustaka <id>.");
            return 1;
        };
        judul(&format!("Pustaka dokumen di situs {situs_nama}"));
        match daftar_pustaka(&nama, &situs_id) {
            Ok(d) => {
                for p in &d {
                    info(&p.nama);
                }
                self.catat("Pustaka", serde_json::to_value(&d).unwrap_or(Value::Null));
                0
            }
            Err(e) => {
                gagal(&format!("Pustaka tidak bisa dibaca. {e}"));
                self.catat("Pustaka", Value::Array(Vec::new()));
                1
            }
        }
    }

    /// Menjadikan client ID milik sebuah akun Google sebagai bawaan aplikasi. Nilainya dibaca dari
    /// rclone.conf di dalam proses ini dan langsung ditulis ke client-google.json; tidak pernah dicetak.
    fn bawaan(&mut self) -> i32 {
        if !self.wajib_rclone() {
            return 1;
        }
        let nama = nama_remote(&self.o.akun);
        if !daftar_remote().contains(&nama) {
            gagal(&format!("Akun {nama} tidak terhubung."));
            return 1;
        }
        let semua = json_dari(&panggil(&["config", "dump"], Mode::Tangkap).out);
        let r = semua.get(&nama);
        if r.map(|r| teks(r.get("type"))).as_deref() != Some("drive") {
            gagal(&format!("Akun {nama} bukan akun Google Drive."));
            return 1;
        }
        let r = r.unwrap();
        let (id, rahasia) = (teks(r.get("client_id")), teks(r.get("client_secret")));
        if id.is_empty() || rahasia.is_empty() {
            gagal(&format!("Akun {nama} masih memakai client ID bersama rclone; pasang client ID sendiri dulu."));
            return 1;
        }
        simpan_client_bawaan(&id, &rahasia);
        ok(&format!("Client ID akun {nama} disimpan sebagai bawaan aplikasi."));
        info(&format!("Lokasi: {} (ikut dibawa installer berikutnya).", jalur().client_pengguna.display()));
        0
    }

    /// Melepas akun dari LELEMOVE: hanya menghapus koneksi di rclone.conf, bukan file di cloud.
    /// Client ID Google aplikasi: tampilkan, simpan (-ClientId X -ClientSecret Y), atau hapus (-Hapus).
    /// Secret tidak pernah dicetak.
    fn klien(&mut self) -> i32 {
        judul("Client ID Google aplikasi");
        if self.o.hapus {
            if hapus_client_pengguna() {
                ok("Client ID aplikasi yang disimpan di komputer ini dihapus.");
            } else {
                info("Tidak ada client ID yang disimpan di komputer ini.");
            }
        } else if let Some(id) = self.o.client_id.clone() {
            let rahasia = self.o.client_secret.clone().unwrap_or_default();
            if !id.ends_with(".apps.googleusercontent.com") {
                gagal("Client ID biasanya berakhiran \".apps.googleusercontent.com\".");
                return 1;
            }
            if rahasia.is_empty() {
                gagal("Tambahkan -ClientSecret.");
                return 1;
            }
            simpan_client_bawaan(&id, &rahasia);
            ok("Client ID aplikasi disimpan.");
        }
        match info_client_bawaan() {
            Some((id, asal)) => {
                ok(&format!("Terpasang: {} (dari {asal})", id_pendek(&id)));
                self.catat("Terpasang", true);
                self.catat("Asal", asal);
            }
            None => {
                awas("Belum ada. Akun Google memakai client ID bersama rclone yang sering dibatasi Google.");
                self.catat("Terpasang", false);
            }
        }
        let google: Vec<Akun> = daftar_akun().into_iter().filter(|a| a.jenis == "google").collect();
        let mut perlu = Vec::new();
        for a in &google {
            let status = if a.client_aplikasi {
                "memakai client ID aplikasi"
            } else if a.client_sendiri {
                "memakai client ID lain"
            } else {
                "memakai client ID bersama rclone"
            };
            info(&format!("{} [{}]: {status}", a.label, a.nama));
            if !a.client_aplikasi {
                perlu.push(a.nama.clone());
            }
        }
        if !perlu.is_empty() && info_client_bawaan().is_some() {
            info("Agar ikut memakai client ID aplikasi, login ulang: pindah hubungkan -Akun <akun> -ClientAplikasi");
        }
        self.catat("PerluLoginUlang", perlu);
        0
    }

    // -------------------------------------------------------------- jadwal

    /// Backup terjadwal: daftar; -Tambah; -Id X dengan -Hapus / -Aktifkan / -Nonaktifkan;
    /// -Jalankan -Id X (dipanggil tugas Windows lewat LELEMOVE.exe --jadwal X).
    fn jadwal(&mut self) -> i32 {
        if self.o.jalankan {
            let Some(id) = self.o.id.clone() else {
                gagal("Tambahkan -Id <id>.");
                return 1;
            };
            return self.jadwal_jalankan(&id);
        }
        if self.o.tambah {
            return self.jadwal_tambah();
        }
        if let Some(id) = self.o.id.clone() {
            let Some(mut j) = jadwal::cari(&id) else {
                gagal(&format!("Jadwal {id} tidak ada."));
                return 1;
            };
            if self.o.hapus {
                if let Err(e) = jadwal::hapus_tugas(&id) {
                    gagal(&format!("{} tidak bisa dihapus: {e}", kapital(win::NAMA_TUGAS)));
                    return 1;
                }
                jadwal::hapus_data(&id);
                ok(&format!("Jadwal \"{}\" dihapus.", j.nama));
                return 0;
            }
            if self.o.aktifkan || self.o.nonaktifkan {
                j.aktif = self.o.aktifkan;
                // Tugas yang hilang (dihapus dari Task Scheduler) dibuat ulang.
                let h = if jadwal::tugas_ada(&id) { jadwal::atur_aktif_tugas(&id, j.aktif) } else { jadwal::pasang_tugas(&j) };
                if let Err(e) = h {
                    gagal(&format!("{} tidak bisa diubah: {e}", kapital(win::NAMA_TUGAS)));
                    return 1;
                }
                jadwal::simpan(&j);
                ok(&format!("Jadwal \"{}\" {}.", j.nama, if j.aktif { "aktif" } else { "nonaktif" }));
                return 0;
            }
        }
        judul("Backup terjadwal");
        let semua = jadwal::baca_semua();
        if semua.is_empty() {
            info(&format!("Belum ada. Contoh: pindah jadwal -Tambah -Nama \"Arsip\" -Sumber \"{}\" -Tujuan \"pd-gdrive:Arsip\" -Jam 22:00 [-Hari sen,rab,jum]", win::CONTOH_DATA));
        }
        for j in &semua {
            let status = if !j.aktif {
                "nonaktif".to_string()
            } else if !jadwal::tugas_ada(&j.id) {
                format!("{} hilang; pasang ulang: pindah jadwal -Aktifkan -Id {}", win::NAMA_TUGAS, j.id)
            } else {
                format!("berikutnya {}", jadwal::berikutnya(j).unwrap_or_default())
            };
            ok(&format!("[{}] {}: {} ({status})", j.id, j.nama, jadwal::uraian(j)));
            info(&format!("    {}  ->  {}", j.sumber, j.tujuan));
            if !j.filter.kosong() {
                info(&format!("    filter: {}", j.filter.uraian()));
            }
            if let Some(t) = &j.terakhir {
                info(&format!("    terakhir {} ({}): {}", t.mulai, t.hasil, t.ket));
            }
        }
        self.catat("Jadwal", serde_json::to_value(&semua).unwrap_or(Value::Null));
        0
    }

    /// -Tambah -Sumber S -Tujuan T [-Nama N] [-Jam 22:00] [-Hari sen,rab] [-TanpaVerifikasi] [-Id X untuk mengubah]
    fn jadwal_tambah(&mut self) -> i32 {
        let (Some(src), Some(dst)) = (self.o.sumber.clone(), self.o.tujuan.clone()) else {
            gagal("Tambahkan -Sumber dan -Tujuan.");
            return 1;
        };
        let (src, dst) = (rapikan_sumber(&src), normalisasi_tujuan(&dst));
        let jam = self.o.jam.clone().unwrap_or_else(|| "22:00".into());
        let Some((h, m)) = jadwal::urai_jam(&jam) else {
            gagal(&format!("Jam tidak dikenal: \"{jam}\". Pakai format 22:00."));
            return 1;
        };
        let hari = match jadwal::urai_hari(self.o.hari.as_deref().unwrap_or("")) {
            Ok(h) => h,
            Err(e) => {
                gagal(&e);
                return 1;
            }
        };
        // Filter milik jadwal ini: hanya dari -Sertakan/-Lewati/-Sejak (bukan filter jendela).
        let filter = match Filter::urai(
            self.o.sertakan.as_deref().unwrap_or(""),
            self.o.lewati.as_deref().unwrap_or(""),
            self.o.sejak.as_deref().unwrap_or(""),
        ) {
            Ok(f) => f,
            Err(e) => {
                gagal(&e);
                return 1;
            }
        };
        let r = Rute { sumber: Some(src.clone()), tujuan: Some(dst.clone()) };
        if !self.rute_siap(&r, true) {
            return 1;
        }
        let id = self.o.id.clone().unwrap_or_else(jadwal::id_baru);
        let lama = jadwal::cari(&id);
        let j = jadwal::Jadwal {
            nama: self.o.nama.clone().unwrap_or_else(|| format!("Backup {}", nama_tempat(&src))),
            id: id.clone(),
            sumber: src,
            tujuan: dst,
            jam: format!("{h:02}:{m:02}"),
            hari,
            verifikasi: !self.o.tanpa_verifikasi,
            aktif: true,
            terakhir: lama.and_then(|l| l.terakhir),
            filter,
        };
        if let Err(e) = jadwal::pasang_tugas(&j) {
            gagal(&format!("{} tidak bisa dibuat: {e}", kapital(win::NAMA_TUGAS)));
            return 1;
        }
        jadwal::simpan(&j);
        ok(&format!("Jadwal \"{}\" dipasang: {}.", j.nama, jadwal::uraian(&j)));
        if !j.filter.kosong() {
            info(&format!("Filter: {}", j.filter.uraian()));
        }
        info(&format!("Berikutnya: {}. {}: {}", jadwal::berikutnya(&j).unwrap_or_default(), kapital(win::NAMA_TUGAS), jadwal::nama_tugas(&id)));
        self.catat("Id", id);
        0
    }

    /// Menjalankan satu jadwal di proses ini (tersembunyi; dipicu tugas Windows atau "Jalankan sekarang").
    fn jadwal_jalankan(&mut self, id: &str) -> i32 {
        let Some(j) = jadwal::cari(id) else {
            gagal(&format!("Jadwal {id} tidak ada."));
            return 1;
        };
        let mulai = sekarang_iso();
        self.nama_jadwal = j.nama.clone();
        let catat = |hasil: &str, ket: &str, log: &str| {
            jadwal::catat_terakhir(
                id,
                jadwal::Terakhir { mulai: mulai.clone(), selesai: sekarang_iso(), hasil: hasil.into(), ket: ket.into(), log: log.into() },
            )
        };
        // Jadwal yang tidak sempat menyalin juga masuk riwayat (bukti untuk audit).
        let riwayat_batal = |hasil: &str, ket: &str| {
            riwayat::catat(Catatan {
                jenis: "salin".into(),
                mulai: mulai.clone(),
                sumber: j.sumber.clone(),
                tujuan: j.tujuan.clone(),
                hasil: hasil.into(),
                ket: ket.into(),
                filter: j.filter.uraian(),
                jadwal: j.nama.clone(),
                ..Default::default()
            })
        };
        let rute = format!("{} \u{2192} {}", nama_tempat(&j.sumber), nama_tempat(&j.tujuan));
        if baca_job().is_some_and(|x| job_berjalan(&x)) {
            let ket = "Dilewati: penyalinan lain sedang berjalan.";
            awas(ket);
            catat("dilewati", ket, "");
            riwayat_batal("dilewati", ket);
            let _ = notifikasi::kirim(
                &format!("Backup terjadwal dilewati: {}", j.nama),
                &[rute, "Penyalinan lain sedang berjalan; dicoba lagi pada jadwal berikutnya.".into()],
            );
            return 0;
        }
        let r = Rute { sumber: Some(j.sumber.clone()), tujuan: Some(j.tujuan.clone()) };
        if !self.rute_siap(&r, true) {
            // Alasan yang spesifik untuk riwayat & notifikasi (pesan gagal() tidak terlihat di proses tersembunyi).
            let ket = if adalah_lokal(&j.sumber) && !Path::new(&j.sumber).is_dir() {
                format!("Folder sumber tidak ditemukan: {}", j.sumber)
            } else if cari_rclone().is_none() {
                "rclone belum terpasang. Buka LELEMOVE untuk memasangnya.".to_string()
            } else {
                let ada = daftar_remote();
                let lepas = [&j.sumber, &j.tujuan].iter().map(|x| remote_dari(x)).find(|r| !r.is_empty() && !ada.contains(r));
                match lepas {
                    Some(r) => format!("Akun {} sudah dilepas dari LELEMOVE; hubungkan lagi atau ubah jadwalnya.", nama_tempat(&format!("{r}:"))),
                    None => "Tujuan tidak boleh sama dengan sumber atau berada di dalam folder sumber.".to_string(),
                }
            };
            catat("galat", &ket, "");
            riwayat_batal("galat", &ket);
            let _ = notifikasi::kirim(&format!("Backup terjadwal gagal: {}", j.nama), &[rute, ket]);
            return 1;
        }
        bersihkan_log_lama();
        let log = path_log("jadwal", "log").to_string_lossy().to_string();
        let batas = baca_batas_kecepatan();
        perbarui_job(
            &[
                ("Log", json!(log)),
                ("Mulai", json!(sekarang_iso())),
                ("Sumber", json!(j.sumber)),
                ("Tujuan", json!(j.tujuan)),
                ("BatasKecepatan", if batas == "off" { Value::Null } else { json!(batas) }),
                ("PidInduk", json!(std::process::id())),
                ("Latar", json!(true)),
                ("Jadwal", json!(j.id)),
                ("JadwalNama", json!(j.nama)),
                ("Filter", if j.filter.kosong() { Value::Null } else { json!(j.filter.uraian()) }),
            ],
            true,
        );
        catat("berjalan", "", &log);
        // Sama dengan pekerja latar: lanjut otomatis saat jaringan putus + notifikasi galat.
        self.o.log_latar = Some(log.clone());
        self.dari_jadwal = true;
        let kode = self.jalankan_salin(&j.sumber, &j.tujuan, &log, &batas, false, &j.filter);
        if kode != 0 {
            let ket = baris_error(&log, 1)
                .pop()
                .map(|e| potong(&RE_AWAL_ERROR.replace(&e, ""), 160))
                .unwrap_or_else(|| format!("rclone berhenti dengan kode {kode}."));
            catat("galat", &ket, &log);
            return kode;
        }
        let baru = baca_statistik(&log).map(|s| ringkas_baru(&s)).unwrap_or_default();
        if !j.verifikasi {
            catat("selesai", &baru, &log);
            let _ = notifikasi::kirim(&format!("Backup terjadwal selesai: {}", j.nama), &[rute, baru]);
            return 0;
        }
        catat("berjalan", &format!("{baru} Memverifikasi hasil salinan\u{2026}"), &log);
        let v = self.verifikasi_rute(&j.sumber, &j.tujuan, &j.filter);
        let n = |k: &str| bil(self.hasil.get(k)).unwrap_or(0);
        if v == 0 {
            let ket = format!("{baru} Terverifikasi: {} file cocok.", angka(n("Cocok")));
            catat("selesai", &ket, &log);
            let _ = notifikasi::kirim(&format!("Backup terjadwal selesai: {}", j.nama), &[rute, ket]);
        } else {
            let ket = format!(
                "Verifikasi menemukan selisih: {} belum ada, {} berbeda, {} gagal dibaca.",
                angka(n("Hilang")),
                angka(n("Beda")),
                angka(n("Galat"))
            );
            catat("selisih", &ket, &log);
            let _ = notifikasi::kirim(
                &format!("Backup terjadwal: ada selisih ({})", j.nama),
                &[rute, format!("{ket} Buka LELEMOVE untuk melanjutkan.")],
            );
        }
        v
    }

    fn lepas(&mut self) -> i32 {
        if !self.wajib_rclone() {
            return 1;
        }
        let nama = nama_remote(&self.o.akun);
        if !daftar_remote().contains(&nama) {
            info(&format!("Akun {nama} tidak terhubung."));
            simpan_meta_akun(&nama, &[], true);
            return 0;
        }
        if let Some(job) = baca_job() {
            let dipakai = remote_dari(&teks(job.get("Sumber"))) == nama || remote_dari(&teks(job.get("Tujuan"))) == nama;
            if job_berjalan(&job) && dipakai {
                gagal("Akun ini sedang dipakai oleh penyalinan yang berjalan. Hentikan dulu.");
                return 1;
            }
        }
        if !self.konfirmasi(&format!("Lepas akun {nama} dari LELEMOVE? File di cloud tidak terhapus.")) {
            return 1;
        }
        // Remote impor milik pengguna: hanya berhenti dipakai, remote rclone-nya dibiarkan.
        if meta(&nama, "Impor") == "1" {
            simpan_meta_akun(&nama, &[], true);
            ok(&format!("Remote {nama} tidak dipakai lagi oleh LELEMOVE (remote rclone-nya tetap ada)."));
            return 0;
        }
        let _ = panggil(&v!["config", "delete", &nama], Mode::Tangkap);
        simpan_meta_akun(&nama, &[], true);
        ok(&format!("Akun {nama} dilepas."));
        info("Untuk mencabut izin sepenuhnya, hapus akses \"rclone\" di pengaturan keamanan akun layanan itu.");
        0
    }

    // -------------------------------------------------------------- periksa & uji

    /// Kode keluar: 0 siap, 1 belum siap/error, 2 ruang tujuan kurang, 3 ada peringatan.
    fn periksa(&mut self) -> i32 {
        let r = self.ambil_rute();
        if !self.rute_siap(&r, false) {
            return 1;
        }
        let Some(f) = self.ambil_filter() else { return 1 };
        let (src, dst) = (r.sumber.clone().unwrap(), r.tujuan.clone().unwrap());
        let mut hasil = 0;
        let mut src_bytes: i64;

        judul("Sumber");
        info(&src);
        self.info_filter(&f);
        if adalah_lokal(&src) {
            info("Menghitung isi folder (hanya membaca daftar file, tidak mengunduh apa pun)...");
            let u = ukur_lokal(&src);
            src_bytes = u.bytes;
            let mut jumlah = u.jumlah;
            // Dengan filter, yang dihitung hanya file yang lolos (oleh rclone, aturan yang sama dengan salin).
            if !f.kosong() && cari_rclone().is_some() {
                let mut a = v!["size", &src, "--json"];
                a.extend(f.argumen());
                let h = panggil(&a, Mode::Tangkap);
                if h.kode == 0 {
                    let j = json_dari(&h.out);
                    src_bytes = bil(j.get("bytes")).unwrap_or(0);
                    jumlah = bil(j.get("count")).unwrap_or(0);
                }
            }
            self.catat("SumberJumlah", jumlah);
            self.catat("SumberBytes", src_bytes);
            self.catat("HanyaCloudJumlah", u.jumlah_hanya_cloud);
            self.catat("HanyaCloudBytes", u.hanya_cloud);
            self.catat("DiskCukup", true);
            ok(&format!("{} file, total {}", angka(jumlah), format_ukuran(src_bytes as f64)));
            if !f.kosong() {
                info(&format!("(dari {} file di folder ini; sisanya tidak lolos filter)", angka(u.jumlah)));
            }
            if u.terbesar > 0 {
                info(&format!("File terbesar: {} ({})", u.terbesar_nama, format_ukuran(u.terbesar as f64)));
            }
            if u.jumlah_hanya_cloud > 0 {
                let bebas = ruang_disk(&src);
                self.catat("DiskBebas", bebas);
                info(&format!(
                    "{} file ({}) masih berstatus \"hanya online\" di OneDrive.",
                    angka(u.jumlah_hanya_cloud),
                    format_ukuran(u.hanya_cloud as f64)
                ));
                info(&format!(
                    "Saat disalin, OneDrive mengunduhnya dulu ke disk ini. Sisa ruang disk: {}",
                    format_ukuran(bebas as f64)
                ));
                if u.hanya_cloud as f64 > bebas as f64 - 2.0 * GB {
                    self.catat("DiskCukup", false);
                    awas("Ruang disk tidak cukup untuk mengunduh semuanya sekaligus.");
                    info("Salin per subfolder, lalu klik kanan subfolder itu di OneDrive > \"Kosongkan ruang\",");
                    info("atau pakai OneDrive lewat internet: pindah hubungkan -Akun onedrive");
                    hasil = 3;
                }
            }
            info("Catatan: hanya folder yang disinkronkan OneDrive ke komputer ini yang ikut terhitung.");
        } else if self.wajib_rclone() {
            info("Menghitung isi folder di cloud...");
            let mut a = v!["size", &src, "--json"];
            a.extend(f.argumen());
            let h = panggil(&a, Mode::Tangkap);
            if h.kode != 0 {
                gagal("Tidak bisa membaca sumber.");
                info(h.err.trim());
                self.saran_login(&h.err, &[&src]);
                return 1;
            }
            let j = json_dari(&h.out);
            src_bytes = bil(j.get("bytes")).unwrap_or(0);
            let n = bil(j.get("count")).unwrap_or(0);
            self.catat("SumberJumlah", n);
            self.catat("SumberBytes", src_bytes);
            ok(&format!("{} file, total {}", angka(n), format_ukuran(src_bytes as f64)));
        } else {
            return 1;
        }

        judul("Tujuan");
        info(&dst);
        self.catat("TujuanTerhubung", false);
        let rem_t = remote_dari(&dst);
        let mut jenis_t = String::new();
        if !rem_t.is_empty() {
            if !self.wajib_rclone() {
                return 1;
            }
            if !daftar_remote().contains(&rem_t) {
                awas(&format!("Akun '{rem_t}' belum terhubung. Jalankan: pindah hubungkan -Akun {rem_t}"));
                return 1;
            }
            jenis_t = jenis_remote(&rem_t);
        }
        self.catat("TujuanJenis", jenis_t.clone());

        let mut sudah: i64 = 0;
        if !rem_t.is_empty() || Path::new(&dst).is_dir() {
            // Yang sudah ada di tujuan dihitung dengan filter yang sama, supaya selisihnya sebanding.
            let mut a = v!["size", &dst, "--json"];
            if !f.kosong() {
                a.extend(f.argumen());
            }
            let h = panggil(&a, Mode::Tangkap);
            if h.kode == 0 {
                sudah = bil(json_dari(&h.out).get("bytes")).unwrap_or(0);
                if sudah > 0 {
                    info(&format!("Sudah ada di tujuan: {} (yang sama akan dilewati)", format_ukuran(sudah as f64)));
                }
            }
        }
        let butuh = (src_bytes - sudah).max(0);
        self.catat("TujuanSudah", sudah);
        self.catat("Butuh", butuh);

        let (mut total, mut terpakai, mut bebas): (i64, i64, i64) = (0, 0, -1);
        if !rem_t.is_empty() {
            let h = panggil(&v!["about", format!("{rem_t}:"), "--json"], Mode::Tangkap);
            // FTP, SFTP tertentu, S3, B2, Azure: tidak ada info kuota. Bukan galat; pastikan saja tujuan bisa dibuka.
            let tanpa_kuota = h.kode != 0 && tanpa_about(&h.err);
            if tanpa_kuota {
                if let Err(e) = self.uji_akses(&rem_t) {
                    if !e.to_lowercase().contains("accessdenied") && !e.contains("403") {
                        gagal(&format!("Tujuan tidak bisa dibuka: {e}"));
                        self.catat("TujuanGalat", true);
                        return 1;
                    }
                }
            } else if h.kode != 0 {
                gagal("Tidak bisa membaca kuota akun tujuan.");
                info(h.err.trim());
                self.catat("TujuanGalat", true);
                self.saran_login(&h.err, &[&dst]);
                return 1;
            }
            let a = json_dari(&h.out);
            if let Some(t) = bil(a.get("total")).filter(|t| *t > 0) {
                total = t;
                terpakai = bil(a.get("used")).unwrap_or(0);
                bebas = total - terpakai;
                if let Some(f) = bil(a.get("free")).filter(|f| *f > 0) {
                    bebas = f;
                }
                if let Some(s) = bil(a.get("trashed")).filter(|s| *s > 0) {
                    self.catat("TujuanSampah", s);
                    info(&format!("Sampah di akun ini memakai {} (ikut dihitung dalam kuota).", format_ukuran(s as f64)));
                }
            }
        } else {
            bebas = ruang_disk(&dst);
            total = bebas;
        }
        self.catat("TujuanTerhubung", true);
        if bebas >= 0 {
            self.catat("TujuanTotal", total);
            self.catat("TujuanTerpakai", terpakai);
            self.catat("TujuanBebas", bebas);
            self.catat("TujuanCukup", butuh <= bebas);
            if !rem_t.is_empty() {
                info(&format!(
                    "Kapasitas {}, terpakai {}, sisa {}",
                    format_ukuran(total as f64),
                    format_ukuran(terpakai as f64),
                    format_ukuran(bebas as f64)
                ));
            } else {
                info(&format!("Sisa ruang disk tujuan: {}", format_ukuran(bebas as f64)));
            }
            info(&format!("Perlu disalin sekitar {}", format_ukuran(butuh as f64)));
            if butuh > bebas {
                gagal("Ruang di tujuan TIDAK cukup.");
                info("Pilihan: kosongkan sampah/ruang di tujuan, salin per subfolder, atau tambah kuota.");
                return 2;
            }
        } else {
            info("Kuota tidak terbatas atau tidak dilaporkan.");
            self.catat("TujuanCukup", true);
        }
        if jenis_t == "google" && butuh as f64 > 700.0 * GB {
            awas("Google membatasi unggahan 750 GB per hari; penyalinan akan berhenti sendiri dan bisa dilanjutkan besok.");
        }
        if hasil == 0 {
            ok("Siap disalin.");
        }
        hasil
    }

    /// Kode keluar: 0 aman, 1 error, 3 ada file yang bermasalah/terlewat.
    fn uji(&mut self) -> i32 {
        let r = self.ambil_rute();
        if !self.rute_siap(&r, true) {
            return 1;
        }
        let Some(f) = self.ambil_filter() else { return 1 };
        let (src, dst) = (r.sumber.clone().unwrap(), r.tujuan.clone().unwrap());
        judul("Uji coba (dry run): tidak ada file yang disalin");
        self.info_filter(&f);
        let log = path_log("uji", "log");
        let mut a = v!["copy", &src, &dst, "--dry-run"];
        if f.buat_folder_kosong() {
            a.push("--create-empty-src-dirs".into());
        }
        a.extend(f.argumen());
        a.extend(argumen_tujuan(&dst));
        a.extend(v!["--log-level", "NOTICE", "--stats", "1h", "--stats-log-level", "NOTICE"]);
        let h = panggil(&a, Mode::Tangkap);
        let _ = std::fs::write(&log, &h.err);

        if h.kode != 0 {
            gagal(&format!("rclone berhenti dengan kode {}:", h.kode));
            for b in h.galat_terakhir(8) {
                info(&b);
            }
            info(&format!("Log lengkap: {}", log.display()));
            self.saran_login(&h.err, &[&src, &dst]);
            return 1;
        }

        let baris: Vec<&str> = h.err.lines().collect();
        let salin: Vec<&str> = baris.iter().copied().filter(|b| b.contains("Skipped copy as --dry-run")).collect();
        let total: f64 = salin
            .iter()
            .filter_map(|b| RE_UKURAN_DRY.captures(b))
            .map(|c| ke_byte(&c[1], &c[2]))
            .sum();
        let masalah: Vec<String> = baris.iter().filter(|b| RE_MASALAH.is_match(b)).map(|s| s.to_string()).collect();
        self.catat("AkanDisalin", salin.len() as i64);
        self.catat("AkanBytes", total as i64);
        self.catat("Masalah", masalah.len() as i64);
        self.catat("MasalahContoh", masalah.iter().take(5).cloned().collect::<Vec<_>>());
        self.catat("Terlewat", 0);

        if salin.is_empty() {
            ok("Tidak ada yang perlu disalin: semua file sudah ada di tujuan.");
        } else {
            ok(&format!("{} file akan disalin (sekitar {}).", angka(salin.len() as i64), format_ukuran(total)));
        }

        let mut hasil = 0;
        // Dengan filter, selisih jumlah file memang disengaja: perbandingan ini tidak berarti.
        if adalah_lokal(&src) && f.kosong() {
            // Bandingkan jumlah file versi Windows dengan versi rclone: selisih berarti ada yang terlewat
            // (mis. placeholder OneDrive yang dianggap "bukan file biasa").
            let u = ukur_lokal(&src);
            let mut a = v!["size", &src, "--json"];
            a.extend(f.argumen());
            let s = panggil(&a, Mode::Tangkap);
            if s.kode == 0 {
                let nr = bil(json_dari(&s.out).get("count")).unwrap_or(0);
                if nr < u.jumlah {
                    self.catat("Terlewat", u.jumlah - nr);
                    awas(&format!(
                        "rclone hanya melihat {} dari {} file. {} file akan TERLEWAT.",
                        angka(nr),
                        angka(u.jumlah),
                        angka(u.jumlah - nr)
                    ));
                    info("Solusi: pakai OneDrive lewat internet (pindah hubungkan -Akun onedrive,");
                    info("lalu -Sumber \"pd-onedrive:<folder>\").");
                    hasil = 3;
                } else {
                    ok(&format!("Semua {} file di folder terbaca oleh rclone.", angka(u.jumlah)));
                }
            }
        }
        if !masalah.is_empty() {
            awas(&format!("{} catatan masalah:", masalah.len()));
            for m in masalah.iter().take(10) {
                info(m);
            }
            hasil = 3;
        }
        info(&format!("Log lengkap: {}", log.display()));
        hasil
    }

    // -------------------------------------------------------------- salin

    fn ringkas_hasil(&self, log: &str, kode: &Value) {
        judul("Hasil penyalinan");
        if let Some(s) = baca_statistik(log) {
            info(&format!("Data   : {}", s.data));
            if !s.file.is_empty() {
                info(&format!("File   : {}", s.file));
            }
            if !s.error.is_empty() {
                awas(&format!("Error  : {}", s.error));
            }
            if !s.waktu.is_empty() {
                info(&format!("Waktu  : {}", s.waktu));
            }
        }
        let ekor = baca_ekor(Path::new(log), 50).join("\n");
        if teks(Some(kode)) == "0" {
            ok("Selesai. Lanjutkan dengan verifikasi: pindah verifikasi");
        } else if ekor.contains("upload limit") || ekor.contains("uploadLimitExceeded") {
            awas("Batas unggah harian Google (750 GB) tercapai. Jalankan \"salin\" lagi besok; file yang sudah ada dilewati.");
        } else {
            gagal(&format!("Ada yang gagal (kode {}).", teks(Some(kode))));
            for e in baris_error(log, 5) {
                info(&e);
            }
            if token_kedaluwarsa(&ekor) {
                info("Login salah satu akun kedaluwarsa: login ulang akun itu, lalu jalankan \"salin\" lagi.");
            }
            info("Jalankan \"salin\" lagi: file yang sudah tersalin dilewati, sisanya dicoba ulang.");
        }
        info(&format!("Log: {log}"));
    }

    fn jalankan_salin(&mut self, src: &str, dst: &str, log: &str, batas: &str, tampil: bool, f: &Filter) -> i32 {
        let mulai = sekarang_iso();
        let mut a = v!["copy", src, dst];
        if f.buat_folder_kosong() {
            a.push("--create-empty-src-dirs".into());
        }
        a.extend(f.argumen());
        a.extend(argumen_tujuan(dst));
        a.extend(v![
            "--transfers", "4", "--checkers", "8",
            // low-level-retries rendah: saat Google membatasi (rate limit), error cepat muncul di log
            // alih-alih rclone diam menunggu berpuluh menit; --retries mengulang file yang gagal.
            "--retries", "5", "--low-level-retries", "10",
            "--drive-chunk-size", "64M", "--drive-stop-on-upload-limit",
            "--log-file", log, "--log-level", "INFO", "--stats", "5s",
        ]);
        if batas != "off" {
            a.extend(v!["--bwlimit", batas]);
        }
        if tampil {
            a.push("--progress".into());
        }
        // Hanya pekerja latar yang melanjutkan sendiri; salin di konsol tetap berhenti dan melapor.
        let lanjut_otomatis = self.o.log_latar.is_some() && baca_lanjut_otomatis();
        let maks = lanjut::maks_percobaan();
        let host = lanjut::host_rute(src, dst);
        let mut percobaan = 1u32;
        win::jaga_bangun(true);
        let kode: Value = loop {
            let awal_log = std::fs::metadata(log).map(|m| m.len()).unwrap_or(0);
            let kode = match rclone::mulai(&a) {
                Ok(mut p) => {
                    perbarui_job(
                        &[
                            ("PidInduk", json!(std::process::id())),
                            ("PidRclone", json!(p.id())),
                            ("Percobaan", json!(percobaan)),
                            ("Menunggu", Value::Null),
                        ],
                        false,
                    );
                    match p.wait() {
                        Ok(s) => json!(s.code().unwrap_or(1)),
                        Err(_) => json!("terputus"),
                    }
                }
                Err(e) => {
                    gagal(&format!("rclone tidak bisa dijalankan: {e}"));
                    json!(1)
                }
            };
            let k = bil(Some(&kode)).unwrap_or(1) as i32;
            if !lanjut_otomatis || percobaan >= maks {
                break kode;
            }
            if lanjut::putuskan(k, &lanjut::potongan_log(Path::new(log), awal_log)) != lanjut::Putusan::Ulang {
                break kode;
            }
            // Jaringan putus: tunggu, lalu jalankan rclone lagi (file yang sudah sampai dilewati).
            let tunggu = lanjut::jeda(percobaan);
            let pukul = lanjut::jam_nanti(tunggu.as_secs());
            perbarui_job(
                &[
                    ("PidRclone", Value::Null),
                    ("Menunggu", json!("jaringan")),
                    ("MenungguSejak", json!(sekarang_iso())),
                    ("UlangPukul", json!(pukul)),
                    ("PercobaanMaks", json!(maks)),
                ],
                false,
            );
            tulis_baris_log(
                log,
                &format!("LELEMOVE: jaringan terputus; dicoba lagi otomatis sekitar pukul {pukul} (percobaan {} dari {maks}).", percobaan + 1),
            );
            if percobaan == 1 {
                let rute = format!("{} \u{2192} {}", nama_tempat(src), nama_tempat(dst));
                let _ = notifikasi::kirim(
                    "Jaringan terputus",
                    &[rute, "LELEMOVE melanjutkan penyalinan otomatis begitu internet tersambung lagi.".into()],
                );
            }
            if !tunggu_jaringan(tunggu, &host) {
                tulis_baris_log(log, "LELEMOVE: internet tidak kembali dalam 6 jam; penyalinan dihentikan.");
                break kode;
            }
            tulis_baris_log(log, "LELEMOVE: internet tersambung lagi; penyalinan dilanjutkan.");
            percobaan += 1;
        };
        win::jaga_bangun(false);
        perbarui_job(&[("Selesai", json!(sekarang_iso())), ("Kode", kode.clone())], false);
        let (n_file, n_byte) = angka_tersalin(log);
        let berhasil = bil(Some(&kode)) == Some(0);
        riwayat::catat(Catatan {
            jenis: "salin".into(),
            mulai,
            sumber: src.into(),
            tujuan: dst.into(),
            hasil: if berhasil { "selesai" } else { "galat" }.into(),
            ket: if berhasil {
                String::new()
            } else {
                baris_error(log, 1)
                    .pop()
                    .map(|e| potong(&RE_AWAL_ERROR.replace(&e, ""), 200))
                    .unwrap_or_else(|| format!("rclone berhenti dengan kode {}.", teks(Some(&kode))))
            },
            file: n_file,
            byte: n_byte,
            filter: f.uraian(),
            jadwal: self.nama_jadwal.clone(),
            log: log.into(),
            ..Default::default()
        });
        if tampil {
            self.ringkas_hasil(log, &kode);
        }
        // Pekerja latar: jendela mungkin sudah ditutup, jadi kabari lewat notifikasi Windows.
        // (Dihentikan manual tidak sampai ke sini: `berhenti` mematikan proses ini lebih dulu.)
        // Backup terjadwal yang berhasil dikabari setelah verifikasi (satu notifikasi saja).
        if self.o.log_latar.is_some() && !(self.dari_jadwal && bil(Some(&kode)) == Some(0)) {
            let (judul, baris) = isi_notifikasi(src, dst, log, &kode);
            let hasil = match notifikasi::kirim(&judul, &baris) {
                Ok(()) => "terkirim".to_string(),
                Err(e) => format!("gagal: {e}"),
            };
            perbarui_job(&[("Notifikasi", json!(hasil))], false);
        }
        bil(Some(&kode)).unwrap_or(1) as i32
    }

    fn salin(&mut self) -> i32 {
        let r = self.ambil_rute();
        if !self.rute_siap(&r, true) {
            return 1;
        }
        let (src, dst) = (r.sumber.clone().unwrap(), r.tujuan.clone().unwrap());
        let batas = self.o.batas_kecepatan.clone().unwrap_or_else(baca_batas_kecepatan);
        let Some(f) = self.ambil_filter() else { return 1 };

        // Proses anak yang dijalankan oleh "salin -Latar".
        if let Some(log) = self.o.log_latar.clone() {
            return self.jalankan_salin(&src, &dst, &log, &batas, false, &f);
        }
        if baca_job().is_some_and(|j| job_berjalan(&j)) {
            awas("Penyalinan lain masih berjalan. Lihat: pindah status");
            return 1;
        }
        // -BatasKecepatan diingat untuk penyalinan berikutnya, seperti -Sumber/-Tujuan.
        if self.o.batas_kecepatan.is_some() {
            simpan_batas_kecepatan(&batas);
        }

        judul("Mulai menyalin");
        info(&format!("Dari  : {src}"));
        info(&format!("Ke    : {dst}"));
        info("Mode  : SALIN saja. File di sumber tidak diubah atau dihapus.");
        if batas != "off" {
            info(&format!("Batas : {}", tampil_batas(&batas)));
        }
        if !f.kosong() {
            info(&format!("Filter: {}", f.uraian()));
        }
        if !self.konfirmasi("Mulai menyalin sekarang?") {
            return 1;
        }

        let lama = bersihkan_log_lama();
        if lama > 0 {
            info(&format!("{} log lama (lebih dari {UMUR_LOG_HARI} hari) dihapus.", angka(lama as i64)));
        }
        let log = path_log("salin", "log").to_string_lossy().to_string();
        let batas_job = if batas == "off" { Value::Null } else { json!(batas) };
        perbarui_job(
            &[
                ("Log", json!(log)),
                ("Mulai", json!(sekarang_iso())),
                ("Sumber", json!(src)),
                ("Tujuan", json!(dst)),
                ("BatasKecepatan", batas_job),
                ("Filter", if f.kosong() { Value::Null } else { json!(f.uraian()) }),
            ],
            true,
        );

        if self.o.latar {
            let exe = std::env::current_exe().unwrap_or_else(|_| super::dasar::exe_mesin());
            return match win::mulai_terlepas(&exe, &v!["salin", "-LogLatar", &log, "-BatasKecepatan", &batas]) {
                Ok(pid) => {
                    perbarui_job(&[("PidInduk", json!(pid)), ("Latar", json!(true))], false);
                    self.catat("Pid", pid);
                    self.catat("Log", log.clone());
                    ok(&format!("Berjalan di latar belakang (PID {pid}). Jendela ini boleh ditutup."));
                    info("Pantau   : pindah status");
                    info("Hentikan : pindah berhenti  (bisa dilanjutkan kapan saja)");
                    0
                }
                Err(e) => {
                    gagal(&format!("Penyalinan latar tidak bisa dimulai: {e}"));
                    perbarui_job(&[("Selesai", json!(sekarang_iso())), ("Kode", json!(1))], false);
                    1
                }
            };
        }
        info("Tekan Ctrl+C untuk berhenti; jalankan lagi nanti untuk melanjutkan.");
        self.jalankan_salin(&src, &dst, &log, &batas, true, &f)
    }

    pub fn status(&mut self) -> i32 {
        let Some(job) = baca_job() else {
            info("Belum pernah ada penyalinan.");
            return 0;
        };
        judul("Status penyalinan");
        info(&format!("Dari   : {}", teks(job.get("Sumber"))));
        info(&format!("Ke     : {}", teks(job.get("Tujuan"))));
        info(&format!("Mulai  : {}", teks(job.get("Mulai"))));
        let batas = teks(job.get("BatasKecepatan"));
        if !batas.is_empty() {
            info(&format!("Batas  : {}", tampil_batas(&batas)));
        }
        let kode = teks(job.get("Kode"));
        let selesai = teks(job.get("Selesai"));
        if job_berjalan(&job) && teks(job.get("Menunggu")) == "jaringan" {
            awas(&format!(
                "Keadaan: MENUNGGU JARINGAN. Dilanjutkan otomatis begitu internet tersambung (sekitar pukul {}, percobaan {} dari {}).",
                teks(job.get("UlangPukul")),
                bil(job.get("Percobaan")).unwrap_or(1) + 1,
                teks(job.get("PercobaanMaks"))
            ));
        } else if job_berjalan(&job) {
            ok("Keadaan: SEDANG BERJALAN");
        } else if kode == "0" {
            ok(&format!("Keadaan: SELESAI ({selesai})"));
        } else if !selesai.is_empty() {
            awas(&format!("Keadaan: berhenti, kode {kode} ({selesai})"));
        } else {
            awas("Keadaan: berhenti (jendela ditutup, komputer mati, atau dihentikan)");
        }
        let log = teks(job.get("Log"));
        match baca_statistik(&log) {
            Some(s) => {
                info(&format!("Data   : {}", s.data));
                if !s.file.is_empty() {
                    info(&format!("File   : {}", s.file));
                }
                if !s.error.is_empty() {
                    awas(&format!("Error  : {}", s.error));
                }
                if !s.waktu.is_empty() {
                    info(&format!("Waktu  : {}", s.waktu));
                }
                if s.dibatasi {
                    awas("Layanan cloud sedang membatasi permintaan (rate limit); rclone menunggu lalu mencoba lagi.");
                    info("Untuk Google Drive, pakai client ID sendiri: pindah hubungkan -Akun <akun> -ClientId X -ClientSecret Y");
                }
                if s.perlu_login {
                    awas("Login salah satu akun kedaluwarsa (invalid_grant). Hentikan, login ulang akun itu, lalu salin lagi.");
                }
            }
            None => info("Statistik belum tersedia (ditulis tiap 5 detik)."),
        }
        let err = baris_error(&log, 5);
        if !err.is_empty() {
            awas("Error terakhir:");
            for e in err {
                info(&e);
            }
        }
        info(&format!("Log    : {log}"));
        0
    }

    pub fn berhenti(&mut self) -> i32 {
        let job = baca_job().unwrap_or_default();
        if !job_berjalan(&job) {
            info("Tidak ada penyalinan yang sedang berjalan.");
            return 0;
        }
        // Induk latar belakang dihentikan dulu, supaya ia tidak sempat menimpa status "dihentikan".
        let saya = std::process::id();
        if let Some(p) = bil(job.get("PidInduk")).map(|p| p as u32) {
            let latar = job.get("Latar") == Some(&Value::Bool(true));
            if p != saya && (win::proses_hidup(p, &["pindah"]) || (latar && win::proses_hidup(p, &["powershell"]))) {
                win::bunuh(p);
            }
        }
        if let Some(p) = bil(job.get("PidRclone")).map(|p| p as u32) {
            if win::proses_hidup(p, &["rclone"]) {
                win::bunuh(p);
            }
        }
        perbarui_job(&[("Selesai", json!(sekarang_iso())), ("Kode", json!("dihentikan"))], false);
        // Pekerja latar sudah dimatikan, jadi ia tidak sempat mencatat; catat di sini.
        let log = teks(job.get("Log"));
        let (n_file, n_byte) = angka_tersalin(&log);
        riwayat::catat(Catatan {
            jenis: "salin".into(),
            mulai: teks(job.get("Mulai")),
            sumber: teks(job.get("Sumber")),
            tujuan: teks(job.get("Tujuan")),
            hasil: "dihentikan".into(),
            ket: "Dihentikan oleh pengguna; bisa dilanjutkan.".into(),
            file: n_file,
            byte: n_byte,
            filter: teks(job.get("Filter")),
            jadwal: teks(job.get("JadwalNama")),
            log,
            ..Default::default()
        });
        ok("Penyalinan dihentikan. Jalankan \"salin\" lagi untuk melanjutkan; file yang sudah tersalin dilewati.");
        0
    }

    // -------------------------------------------------------------- verifikasi

    fn verifikasi(&mut self) -> i32 {
        let r = self.ambil_rute();
        if !self.rute_siap(&r, true) {
            return 1;
        }
        let (src, dst) = (r.sumber.clone().unwrap(), r.tujuan.clone().unwrap());
        if baca_job().is_some_and(|j| job_berjalan(&j)) {
            awas("Tunggu penyalinan selesai dulu (lihat: pindah status).");
            return 1;
        }
        let Some(f) = self.ambil_filter() else { return 1 };
        self.verifikasi_rute(&src, &dst, &f)
    }

    /// Mencocokkan `src` dengan `dst` (dipakai `verifikasi`, `pindahkan`, dan backup terjadwal).
    fn verifikasi_rute(&mut self, src: &str, dst: &str, f: &Filter) -> i32 {
        let mulai = sekarang_iso();
        let kode = self.verifikasi_inti(src, dst, f);
        let n = |k: &str| bil(self.hasil.get(k)).unwrap_or(0);
        let ada_laporan = kode == 0 || n("Cocok") + n("Hilang") + n("Beda") + n("Galat") > 0;
        riwayat::catat(Catatan {
            jenis: "verifikasi".into(),
            mulai,
            sumber: src.into(),
            tujuan: dst.into(),
            hasil: if kode == 0 {
                "cocok"
            } else if ada_laporan {
                "selisih"
            } else {
                "galat"
            }
            .into(),
            ket: if ada_laporan { String::new() } else { "Verifikasi tidak berjalan; lihat log aktivitas.".into() },
            file: n("Cocok"),
            hilang: n("Hilang"),
            beda: n("Beda"),
            galat: n("Galat"),
            metode: teks(self.hasil.get("Metode")),
            filter: f.uraian(),
            jadwal: self.nama_jadwal.clone(),
            laporan: teks(self.hasil.get("Laporan")),
            ..Default::default()
        });
        kode
    }

    fn verifikasi_inti(&mut self, src: &str, dst: &str, f: &Filter) -> i32 {
        let (src, dst) = (src.to_string(), dst.to_string());
        judul("Verifikasi: mencocokkan sumber dengan tujuan");
        self.info_filter(f);
        info("Membandingkan setiap file: dengan checksum bila kedua sisi punya jenis yang sama, selain itu nama & ukuran...");
        let lap = path_log("verifikasi", "txt");
        let laps = lap.to_string_lossy().to_string();
        let mut a = v!["check", &src, &dst, "--one-way", "--combined", &laps];
        a.extend(f.argumen());
        a.extend(v!["--log-level", "NOTICE"]);
        let h = panggil(&a, Mode::Tangkap);
        // rclone: "No common hash found - not using a hash for checks" (mis. OneDrive <-> Google Drive)
        let metode = if h.err.contains("No common hash") || h.err.contains("no hashes in common") { "ukuran" } else { "checksum" };
        self.catat("Metode", metode);
        if metode == "ukuran" {
            info("Kedua layanan tidak punya jenis checksum yang sama; dibandingkan per nama & ukuran.");
        } else {
            info("Dibandingkan dengan checksum (isi file).");
        }

        let isi: Vec<String> = std::fs::read_to_string(&lap)
            .map(|s| s.lines().map(str::to_string).collect())
            .unwrap_or_default();
        let ambil = |awalan: &str| -> Vec<String> { isi.iter().filter(|l| l.starts_with(awalan)).cloned().collect() };
        let mut sama = ambil("= ");
        let hilang = ambil("- ");
        let mut beda = ambil("* ");
        let galat = ambil("! ");
        // Tujuan SharePoint: file Office yang "berbeda" adalah akibat metadata yang ditulis SharePoint.
        let mut office = Vec::new();
        let rem_t = remote_dari(&dst);
        if !rem_t.is_empty() && jenis_dari_nama(&rem_t) == "sharepoint" {
            office = beda.iter().filter(|l| RE_OFFICE.is_match(l)).cloned().collect::<Vec<_>>();
            beda.retain(|l| !RE_OFFICE.is_match(l));
            sama.extend(office.iter().cloned());
            if !office.is_empty() {
                info(&format!(
                    "{} file Office berbeda ukuran karena metadata yang ditambahkan SharePoint; dianggap cocok.",
                    angka(office.len() as i64)
                ));
            }
        }
        let contoh: Vec<String> = hilang.iter().chain(beda.iter()).chain(galat.iter()).take(5).cloned().collect();
        self.catat("OfficeSharePoint", office.len() as i64);
        self.catat("Cocok", sama.len() as i64);
        self.catat("Hilang", hilang.len() as i64);
        self.catat("Beda", beda.len() as i64);
        self.catat("Galat", galat.len() as i64);
        self.catat("Contoh", contoh);
        self.catat("Laporan", laps.clone());

        info(&format!("Cocok                      : {} file", angka(sama.len() as i64)));
        if !hilang.is_empty() {
            awas(&format!("Belum ada di tujuan        : {} file", angka(hilang.len() as i64)));
        }
        if !beda.is_empty() {
            awas(&format!("Isinya berbeda             : {} file", angka(beda.len() as i64)));
        }
        if !galat.is_empty() {
            awas(&format!("Gagal dibaca               : {} file", angka(galat.len() as i64)));
        }

        self.catat("Sumber", src.clone());
        if !isi.is_empty() && hilang.is_empty() && beda.is_empty() && galat.is_empty() {
            ok(&format!("SEMUA FILE COCOK: {} file dari {src}", angka(sama.len() as i64)));
            info(&format!("sudah ada di {dst}."));
            info("File di sumber tidak diubah.");
            info(&format!("Laporan: {laps}"));
            return 0;
        }
        if isi.is_empty() && h.kode == 0 {
            ok("Tidak ada file untuk dicocokkan (folder kosong, atau tidak ada yang lolos filter).");
            return 0;
        }
        if isi.is_empty() {
            gagal(&format!("Verifikasi tidak berjalan (kode {}):", h.kode));
            for b in h.galat_terakhir(5) {
                info(&b);
            }
            self.saran_login(&h.err, &[&src, &dst]);
            return 1;
        }
        for l in hilang.iter().chain(beda.iter()).chain(galat.iter()).take(10) {
            info(l);
        }
        info("Jalankan \"salin\" lagi untuk melengkapi, lalu verifikasi ulang.");
        info(&format!("Laporan lengkap: {laps}"));
        1
    }

    // -------------------------------------------------------------- pindahkan

    /// Menghapus dari SUMBER file yang terbukti sama di tujuan. Selalu memverifikasi ulang saat
    /// itu juga dan hanya menghapus file berstatus cocok persis ("=" di laporan rclone); file lain
    /// (belum ada, berbeda, gagal dibaca, Office yang diubah SharePoint) tidak disentuh.
    /// Komputer: ke Recycle Bin. Google Drive: ke Sampah. OneDrive/SharePoint: ke Recycle bin.
    fn pindahkan(&mut self) -> i32 {
        let r = self.ambil_rute();
        if !self.rute_siap(&r, true) {
            return 1;
        }
        let Some(f) = self.ambil_filter() else { return 1 };
        let (src, dst) = (r.sumber.clone().unwrap(), r.tujuan.clone().unwrap());
        if baca_job().is_some_and(|j| job_berjalan(&j)) {
            awas("Tunggu penyalinan selesai dulu (lihat: pindah status).");
            return 1;
        }
        let lokal = adalah_lokal(&src);
        if lokal && !win::drive_tetap(Path::new(&src)) {
            gagal(if cfg!(windows) {
                "Folder sumber ada di drive jaringan atau flashdisk yang tidak punya Recycle Bin; file di sana akan hilang permanen."
            } else {
                "Folder sumber ada di volume jaringan yang tidak punya Tong Sampah; file di sana akan hilang permanen."
            });
            info("LELEMOVE tidak menghapusnya. Hapus sendiri bila sudah yakin.");
            return 1;
        }
        let tempat: String = if lokal {
            format!("{} {}", win::NAMA_SAMPAH, win::NAMA_OS)
        } else {
            let p = penyedia::cari(jenis_dari_nama(&remote_dari(&src)));
            match p.and_then(|p| p.sampah) {
                Some(s) => s.to_string(),
                None => {
                    let n = p.map_or("Layanan ini", |p| p.nama);
                    gagal(&format!("{n} tidak punya tempat sampah: file yang dihapus akan hilang permanen."));
                    info("LELEMOVE tidak menghapus di sana. Hapus sendiri bila sudah yakin.");
                    return 1;
                }
            }
        };
        let mulai = sekarang_iso();

        // 1. Verifikasi ulang sekarang: hasil verifikasi sebelumnya bisa basi.
        let v = self.verifikasi_rute(&src, &dst, &f);
        let laporan = teks(self.hasil.get("Laporan"));
        let per_file = riwayat::baca_per_file(Path::new(&laporan));
        let cocok: Vec<String> = per_file.iter().filter(|(s, _)| *s == '=').map(|(_, p)| p.clone()).collect();
        let lain = per_file.iter().filter(|(s, _)| *s != '=' && *s != '+').count() as i64;
        judul("Pindahkan: hapus dari sumber file yang sudah terbukti sama di tujuan");
        self.catat("AkanDihapus", cocok.len() as i64);
        self.catat("TidakDihapus", lain);
        if cocok.is_empty() {
            info("Tidak ada file yang terbukti sama di tujuan; tidak ada yang dihapus.");
            return if v == 0 { 0 } else { 1 };
        }
        info(&format!("{} file di {src} terbukti sama di {dst}.", angka(cocok.len() as i64)));
        info(&format!("File itu dipindahkan ke {tempat}, jadi masih bisa dipulihkan."));
        if lain > 0 {
            awas(&format!("{} file lain belum cocok dan TIDAK dihapus.", angka(lain)));
        }
        if !self.o.ya {
            if !bisa_bertanya() {
                awas("Butuh persetujuan. Ulangi perintah dengan tambahan -Ya setelah disetujui.");
                return 3;
            }
            if baca_baris("        Ketik HAPUS untuk melanjutkan: ").trim() != "HAPUS" {
                info("Dibatalkan; tidak ada yang dihapus.");
                return 1;
            }
        }

        // 2. Hapus (ke tempat sampah masing-masing layanan).
        let log = path_log("pindahkan", "log");
        let logs = log.to_string_lossy().to_string();
        let dihapus = if lokal {
            let akar = PathBuf::from(&src);
            let berkas: Vec<PathBuf> = cocok.iter().map(|p| akar.join(p.replace('/', std::path::MAIN_SEPARATOR_STR))).collect();
            let n = win::ke_recycle_bin(&berkas);
            let catatan: String = berkas
                .iter()
                .map(|p| format!("{} {}\r\n", if p.exists() { "GAGAL  " } else { "Dihapus" }, p.display()))
                .collect();
            let _ = std::fs::write(&log, catatan);
            hapus_folder_kosong_lokal(&akar, &berkas);
            n
        } else {
            let daftar = path_log("pindahkan-daftar", "txt");
            let _ = std::fs::write(&daftar, cocok.join("\n") + "\n");
            let a = v![
                "delete", &src, "--files-from-raw", daftar.to_string_lossy(),
                // Google Drive: ke Sampah (bawaan rclone, ditegaskan). OneDrive: ke recycle bin (bawaan).
                "--drive-use-trash", "--log-file", &logs, "--log-level", "INFO",
            ];
            let h = panggil(&a, Mode::Tangkap);
            let n = std::fs::read_to_string(&log).unwrap_or_default().lines().filter(|l| l.contains(": Deleted")).count();
            if h.kode != 0 {
                awas(&format!("rclone berhenti dengan kode {}; sebagian file mungkin belum terhapus.", h.kode));
                for b in h.galat_terakhir(5) {
                    info(&b);
                }
                self.saran_login(&h.err, &[&src]);
            }
            let _ = std::fs::remove_file(&daftar);
            hapus_folder_kosong_cloud(&src, &cocok);
            n
        };

        let semua = dihapus == cocok.len();
        let mut ket = format!("{} file ke {tempat}", angka(dihapus as i64));
        if !semua {
            ket += &format!("; {} gagal dihapus", angka((cocok.len() - dihapus) as i64));
        }
        if lain > 0 {
            ket += &format!("; {} file belum cocok tidak dihapus", angka(lain));
        }
        riwayat::catat(Catatan {
            jenis: "pindahkan".into(),
            mulai,
            sumber: src.clone(),
            tujuan: dst.clone(),
            hasil: if semua { "selesai" } else if dihapus > 0 { "sebagian" } else { "galat" }.into(),
            ket: ket.clone(),
            file: dihapus as i64,
            hilang: bil(self.hasil.get("Hilang")).unwrap_or(0),
            beda: bil(self.hasil.get("Beda")).unwrap_or(0),
            galat: bil(self.hasil.get("Galat")).unwrap_or(0),
            filter: f.uraian(),
            log: logs.clone(),
            laporan,
            ..Default::default()
        });
        self.catat("Dihapus", dihapus as i64);
        self.catat("Tempat", tempat.as_str());
        if semua {
            ok(&format!("{} file dipindahkan: kini hanya ada di {dst}.", angka(dihapus as i64)));
        } else {
            awas(&format!("Hanya {} dari {} file yang terhapus dari sumber.", angka(dihapus as i64), angka(cocok.len() as i64)));
        }
        info(&format!("Yang dihapus ada di {tempat} bila perlu dipulihkan. Log: {logs}"));
        if semua {
            0
        } else {
            1
        }
    }

    // -------------------------------------------------------------- riwayat

    /// Daftar riwayat; -Csv / -Laporan mengekspor. Dengan -Id: satu catatan (laporan per file).
    fn riwayat(&mut self) -> i32 {
        let semua = riwayat::baca(0);
        let tulis = |p: &str, isi: &str, apa: &str| -> i32 {
            match std::fs::write(p, isi) {
                Ok(()) => {
                    ok(&format!("{apa} disimpan: {p}"));
                    0
                }
                Err(e) => {
                    gagal(&format!("{apa} tidak bisa disimpan ke {p}: {e}"));
                    1
                }
            }
        };
        if let Some(id) = self.o.id.clone() {
            let Some(c) = semua.iter().find(|c| c.id == id) else {
                gagal(&format!("Riwayat {id} tidak ada."));
                return 1;
            };
            let per_file = !c.laporan.is_empty();
            if let Some(p) = self.o.laporan.clone() {
                let isi = if per_file { riwayat::html_verifikasi(c) } else { riwayat::html_riwayat(std::slice::from_ref(c)) };
                return tulis(&p, &isi, "Laporan");
            }
            if let Some(p) = self.o.csv.clone() {
                let isi = if per_file { riwayat::csv_per_file(c) } else { riwayat::csv(std::slice::from_ref(c)) };
                return tulis(&p, &isi, "CSV");
            }
            self.catat("Riwayat", serde_json::to_value(vec![c]).unwrap_or(Value::Null));
            judul(&format!("{} {}", riwayat::label_jenis(c), waktu_tampil(&c.mulai)));
            info(&format!("Dari  : {}", c.sumber));
            info(&format!("Ke    : {}", c.tujuan));
            info(&format!("Hasil : {} ({})", riwayat::label_hasil(c), riwayat::ringkas(c)));
            if !c.ket.is_empty() {
                info(&format!("Ket   : {}", c.ket));
            }
            if !c.log.is_empty() {
                info(&format!("Log   : {}", c.log));
            }
            return 0;
        }
        if let Some(p) = self.o.csv.clone() {
            return tulis(&p, &riwayat::csv(&semua), "CSV riwayat");
        }
        if let Some(p) = self.o.laporan.clone() {
            return tulis(&p, &riwayat::html_riwayat(&semua), "Laporan riwayat");
        }
        let n = self.o.jumlah.as_deref().and_then(|j| j.parse::<usize>().ok()).unwrap_or(20).min(semua.len());
        judul("Riwayat");
        if semua.is_empty() {
            info("Belum ada riwayat. Riwayat dicatat sejak versi yang memuat fitur ini.");
        }
        for c in &semua[..n] {
            let baris = format!(
                "{}  {} {}: {} -> {}",
                waktu_tampil(&c.mulai),
                riwayat::label_jenis(c),
                riwayat::label_hasil(c).to_lowercase(),
                nama_tempat(&c.sumber),
                nama_tempat(&c.tujuan)
            );
            if riwayat::hasil_baik(c) {
                ok(&baris);
            } else {
                awas(&baris);
            }
            let mut ket = format!("    {}", riwayat::ringkas(c));
            if !c.jadwal.is_empty() {
                ket += &format!(" (jadwal {})", c.jadwal);
            }
            info(&format!("{ket}  [-Id {}]", c.id));
        }
        if semua.len() > n {
            info(&format!("... {} lagi. Tampilkan lebih banyak: pindah riwayat -Jumlah 100", angka((semua.len() - n) as i64)));
        }
        self.catat("Riwayat", serde_json::to_value(&semua[..n]).unwrap_or(Value::Null));
        self.catat("Jumlah", semua.len() as i64);
        0
    }

    // -------------------------------------------------------------- menu teks

    fn pilih_rute(&mut self) -> bool {
        judul("Pilih folder SUMBER");
        let mut opsi: Vec<(String, String)> = daftar_onedrive()
            .into_iter()
            .map(|o| (format!("[{}] {}", o.jenis, o.folder), o.folder))
            .collect();
        for a in daftar_akun().into_iter().filter(|a| a.siap) {
            opsi.push((format!("{} [{}]", a.label, a.nama), format!("{}:", a.nama)));
        }
        opsi.push(("Ketik alamat folder lain".into(), "__ketik".into()));
        opsi.push(("OneDrive lewat internet (login Microsoft)".into(), "__cloud".into()));
        for (i, (l, _)) in opsi.iter().enumerate() {
            info(&format!("{}. {l}", i + 1));
        }
        let n = baca_angka("Nomor", 1, opsi.len() as i64) as usize;
        let mut pilih = opsi[n - 1].1.clone();

        if pilih == "__ketik" {
            pilih = rapikan_sumber(&baca_baris("        Alamat folder: "));
            if !Path::new(&pilih).is_dir() {
                gagal("Folder tidak ditemukan.");
                return false;
            }
        } else if pilih == "__cloud" {
            if !daftar_remote().iter().any(|r| r == REMOTE_ONEDRIVE) {
                let lama = std::mem::replace(&mut self.o.akun, "onedrive".into());
                let k = self.hubungkan();
                self.o.akun = lama;
                if k != 0 {
                    return false;
                }
            }
            pilih = format!("{REMOTE_ONEDRIVE}:");
        }

        let sub = daftar_subfolder(&pilih);
        if !sub.is_empty() {
            judul("Salin semua isinya, atau satu subfolder saja?");
            info("0. Semua isi");
            for (i, s) in sub.iter().enumerate() {
                info(&format!("{}. {s}", i + 1));
            }
            let n = baca_angka("Nomor", 0, sub.len() as i64) as usize;
            if n > 0 {
                pilih = gabung_path(&pilih, &sub[n - 1]);
            }
        }

        let usul = tujuan_bawaan(&pilih, REMOTE_GOOGLE);
        judul("Folder TUJUAN di Google Drive");
        info(&format!("Usulan: {}", usul.trim_start_matches(&format!("{REMOTE_GOOGLE}:"))));
        let t = baca_baris("        Tekan Enter untuk memakai usulan, atau ketik nama lain: ");
        let tujuan = if t.is_empty() { usul } else { normalisasi_tujuan(&t) };
        simpan_pengaturan(Some(&pilih), Some(&tujuan));
        ok(&format!("Sumber: {pilih}"));
        ok(&format!("Tujuan: {tujuan}"));
        true
    }

    fn langkah_dipandu(&mut self) -> i32 {
        judul("LANGKAH 1/6 - rclone");
        if cari_rclone().is_some() {
            ok("rclone sudah terpasang.");
        } else if self.pasang() != 0 {
            return 1;
        }

        judul("LANGKAH 2/6 - Akun Google Drive");
        self.o.akun = "google".into();
        if self.hubungkan() != 0 {
            return 1;
        }

        judul("LANGKAH 3/6 - Folder sumber & tujuan");
        let r = self.ambil_rute();
        let mut pakai = false;
        if let Some(s) = &r.sumber {
            info(&format!("Pilihan tersimpan: {s}  ->  {}", r.tujuan.clone().unwrap_or_default()));
            pakai = self.konfirmasi("Pakai pilihan ini?");
        }
        if !pakai && !self.pilih_rute() {
            return 1;
        }

        judul("LANGKAH 4/6 - Periksa ukuran & ruang");
        let k = self.periksa();
        if k == 1 || k == 2 {
            return 1;
        }
        if k == 3 && !self.konfirmasi("Ada peringatan di atas. Tetap lanjut?") {
            return 1;
        }

        judul("LANGKAH 5/6 - Uji coba");
        let k = self.uji();
        if k == 1 {
            return 1;
        }
        if k == 3 && !self.konfirmasi("Uji coba menemukan masalah. Tetap lanjut?") {
            return 1;
        }

        judul("LANGKAH 6/6 - Salin lalu verifikasi");
        if self.salin() != 0 {
            return 1;
        }
        self.verifikasi()
    }

    fn menu(&mut self) {
        if !bisa_bertanya() {
            tampilkan_bantuan();
            return;
        }
        loop {
            bersihkan_layar();
            let r = self.ambil_rute();
            let s = r.sumber.clone().unwrap_or_else(|| "(belum dipilih)".into());
            let t = r
                .tujuan
                .clone()
                .map(|t| t.replacen(&format!("{REMOTE_GOOGLE}:"), "Google Drive: ", 1))
                .unwrap_or_else(|| "(belum dipilih)".into());
            polos("");
            berwarna("96", "  PINDAH DRIVE  |  salin antar OneDrive, SharePoint, Google Drive, komputer");
            polos("  --------------------------------------------------------------------------");
            polos(&format!("  Sumber : {s}"));
            polos(&format!("  Tujuan : {t}"));
            if let Some(job) = baca_job().filter(job_berjalan) {
                let mut st = "sedang menyalin".to_string();
                if let Some(x) = baca_statistik(&teks(job.get("Log"))) {
                    st += &format!(" - {}", x.data);
                }
                berwarna("93", &format!("  Status : {st}"));
            }
            polos("");
            berwarna("92", "  1. Jalankan semua langkah (dipandu)   <- paling mudah");
            polos("");
            polos("  2. Pasang rclone");
            polos("  3. Hubungkan akun Google Drive");
            polos("  4. Pilih folder sumber & tujuan");
            polos("  5. Periksa ukuran & ruang kosong");
            polos("  6. Uji coba (tidak menyalin apa pun)");
            polos("  7. Mulai menyalin");
            polos("  8. Lihat status");
            polos("  9. Verifikasi hasil");
            polos("  0. Keluar");
            polos("");
            let p = baca_baris("  Pilih: ");
            let dikenal = match p.as_str() {
                "1" => {
                    self.langkah_dipandu();
                    true
                }
                "2" => {
                    self.pasang();
                    true
                }
                "3" => {
                    self.o.akun = "google".into();
                    self.hubungkan();
                    true
                }
                "4" => {
                    self.pilih_rute();
                    true
                }
                "5" => {
                    self.periksa();
                    true
                }
                "6" => {
                    self.uji();
                    true
                }
                "7" => {
                    self.salin();
                    true
                }
                "8" => {
                    self.status();
                    true
                }
                "9" => {
                    self.verifikasi();
                    true
                }
                "0" => return,
                _ => false,
            };
            if dikenal {
                polos("");
                let _ = baca_baris("  Tekan Enter untuk kembali ke menu");
            }
        }
    }
}

pub fn tampilkan_bantuan() {
    polos(&format!(
        "LELEMOVE {}: salin file antar Google Drive, OneDrive, SharePoint, Dropbox, NAS, S3, layanan rclone lain, dan folder komputer.",
        crate::VERSI
    ));
    polos("");
    polos("Perintah (pindah <perintah> [opsi]):");
    polos("  cek                         status rclone, akun, folder OneDrive, rute");
    polos("  akun                        daftar akun cloud yang terhubung");
    polos("  pasang [-Ya]                pasang rclone lewat winget");
    polos("  hubungkan -Akun <nama> [-Jenis google|onedrive|sharepoint] [-Label X] [-IzinPenuh] [-Ulang]");
    polos("            [-ClientAplikasi | -ClientId X -ClientSecret Y [-SimpanBawaan]]   login akun (di browser)");
    polos("  lepas -Akun <nama>          lepas akun (file di cloud tidak terhapus)");
    polos("  klien [-ClientId X -ClientSecret Y | -Hapus]   client ID Google aplikasi (lihat/simpan/hapus)");
    polos("  jadwal                      daftar backup terjadwal");
    polos("  jadwal -Tambah -Sumber X -Tujuan Y [-Nama N] [-Jam 22:00] [-Hari sen,rab,jum] [-TanpaVerifikasi] [-Id X]");
    polos("         [-Sertakan ..] [-Lewati ..] [-Sejak ..]   filter khusus jadwal ini");
    polos("  jadwal -Id X -Hapus | -Aktifkan | -Nonaktifkan | -Jalankan");
    polos("  bawaan -Akun <nama>         jadikan client ID akun Google itu client ID aplikasi");
    polos("  situs -Akun <sharepoint> -Cari <kata>              cari situs SharePoint");
    polos("  pustaka -Akun <sharepoint> -SitusId <id>           daftar pustaka dokumen situs");
    polos("  pustaka -Akun <sharepoint> -Pustaka <id> [-SitusNama X -PustakaNama Y]   pakai pustaka");
    polos("  periksa [-Sumber X] [-Tujuan Y]     ukuran sumber vs ruang tujuan");
    polos(&format!("          X/Y: {}  atau  <akun>:<folder>, mis. pd-onedrive:Kerja  pd-gdrive-2:Arsip", win::CONTOH_DATA));
    polos("  uji                         dry run, deteksi file yang akan terlewat");
    polos("  salin [-Latar] [-Ya]        mulai menyalin (bisa dilanjutkan)");
    polos("        [-BatasKecepatan 5M]  batasi kecepatan (5M = 5 MB/dtk, 500K, off); diingat untuk berikutnya");
    polos("  status | berhenti           pantau / hentikan penyalinan latar");
    polos("  verifikasi                  cocokkan isi sumber dan tujuan");
    polos("  pindahkan [-Ya]             verifikasi ulang, lalu hapus dari SUMBER file yang terbukti sama");
    polos(&format!("                              (ke {} / Sampah; file yang tidak cocok tidak disentuh)", win::NAMA_SAMPAH));
    polos("  penyedia                    daftar layanan yang bisa ditambahkan (Dropbox, NAS, S3, ...)");
    polos("  hubungkan -Akun pd-<x> -Jenis <jenis> [-Opsi kunci=nilai ...] [-RahasiaStdin]");
    polos("            akun selain Google/Microsoft; rahasia dikirim sebagai JSON lewat stdin");
    polos("  hubungkan -Akun <nama> -Lanjut -State S -Jawab X   jawab pertanyaan lanjutan (mis. kode 2FA)");
    polos("  impor [-Akun <remote>] [-Label X]   pakai remote rclone yang sudah ada");
    polos("  riwayat [-Jumlah 50]        riwayat salin, verifikasi, pindahkan");
    polos("          [-Csv x.csv | -Laporan x.html]   ekspor semua riwayat");
    polos("          -Id <id> [-Csv x.csv | -Laporan x.html]   laporan per file satu verifikasi");
    polos("  Filter (periksa, uji, salin, verifikasi, pindahkan; diingat untuk berikutnya):");
    polos("    -Sertakan \"pdf,docx\"  hanya jenis file ini   -Lewati \"*.tmp,node_modules/\"  lewati pola ini");
    polos("    -Sejak 2026-01-31     hanya yang diubah sejak tanggal itu   -TanpaFilter  hapus filter");
    polos("  menu                        menu teks (bawaan bila tanpa perintah)");
    polos("");
    polos(&format!("Dukung pengembangan LELEMOVE: {}", crate::SAWERIA));
}
