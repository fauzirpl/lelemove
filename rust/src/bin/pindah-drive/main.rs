//! LELEMOVE: jendela aplikasi (egui). Pengganti gui.ps1 + tampilan.xaml.
//!
//! Pekerjaan berat (pasang, login, periksa, uji, salin, verifikasi) dijalankan oleh pindah.exe
//! sebagai proses terpisah, sehingga jendela tidak pernah membeku. Penyalinan berjalan di latar
//! belakang dan tetap jalan walau aplikasi ditutup; saat dibuka lagi, progresnya muncul kembali.
//!
//! Pratinjau (dokumentasi/pengujian):
//!   "LELEMOVE.exe" --pratinjau layar.png [--dialog akun-google|client|pengaturan|jadwal|jadwal-baru|info:<tab>|jelajah:<akun>|sharepoint:<akun>:<kata>]
//!                      [--log] [--lebar 760] [--tinggi 820] [--tunda 2500]
//!
//! Tugas Windows jadwal backup memanggil `LELEMOVE.exe --jadwal <id>` (tanpa jendela).
#![windows_subsystem = "windows"]

mod aksi;
mod dialog;
mod info;
mod jadwal;

use eframe::egui::{
    self, pos2, vec2, Align, Align2, Color32, ComboBox, FontId, Frame, Id, Label, Layout, Margin, Rect,
    RichText, ScrollArea, Sense, Stroke, StrokeKind, TextEdit, Ui, UiBuilder, ViewportCommand,
};
use egui::popup::{popup_below_widget, PopupCloseBehavior};
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};

use pindah_drive::gaya::{
    self, ik, kecil, keluarga_ikon, label_kelompok, potong, tebal, tombol, ABU, GARIS, GARIS_HALUS, HIJAU, HOVER, JALUR,
    KUNING, LATAR2, MAP, MERAH, PILIH, PUTIH, REDUP, TAUTAN, TINTA,
};
use pindah_drive::mesin::akun::{self, Akun, Sambungan};
use pindah_drive::mesin::dasar::{angka, bil, format_ukuran as ukuran, jalur, ke_byte, teks, waktu_lokal, waktu_tampil};
use pindah_drive::mesin::jadwal as mj;
use pindah_drive::mesin::job::{baca_job, baca_statistik, baris_error, job_berjalan};
use pindah_drive::mesin::rclone;
use pindah_drive::mesin::rute::{self, adalah_lokal, daftar_onedrive, rapikan_sumber, tujuan_bawaan};
use pindah_drive::mesin::{Mesin, Opsi};
use pindah_drive::{v, win, APP_ID, MUTEX_JENDELA};

use aksi::{Aksi, Isi, Kabar, Setelah};
use dialog::{AkunBaru, AkunGoogle, ClientId, Dialog, Hasil, Jelajah, Konfirmasi, Pemuat, Pengaturan, SharePoint, Tanya};

const IKON: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../aset/ikon.ico"));
const TITIK: &str = "\u{00B7}";
const ELIPSIS: &str = "\u{2026}";
const CENTANG: &str = "\u{2713}";
/// Jeda sebelum akun yang gagal karena internet/DNS diuji lagi.
const UJI_ULANG_JARINGAN: Duration = Duration::from_secs(30);
/// Panel kiri (sumber) dan kanan (tujuan).
const DARI: usize = 0;
const KE: usize = 1;
/// Tinggi baris daftar folder.
const TINGGI_BARIS: f32 = 26.0;
/// Pilihan batas kecepatan di toolbar: nilai --bwlimit rclone dan labelnya.
const PILIHAN_BATAS: [(&str, &str); 4] =
    [("off", "Tanpa batas"), ("1M", "1 MB/dtk"), ("5M", "5 MB/dtk"), ("10M", "10 MB/dtk")];

type Teks = (String, Color32);

fn kosong() -> Teks {
    (String::new(), REDUP)
}

static RE_PERSEN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\d+)%").unwrap());
static RE_KOMA: LazyLock<Regex> = LazyLock::new(|| Regex::new(r",\s*").unwrap());
static RE_DARI_TOTAL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(.+?)\s*/\s*(.+)$").unwrap());
static RE_UKURAN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"([\d.]+)\s*([KMGTP]?i?B)?").unwrap());
static RE_FILE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d+)\s*/\s*(\d+)").unwrap());
static RE_ETA: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"ETA\s+(\S+)").unwrap());
static RE_NOL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^0\b").unwrap());
static RE_KURUNG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*\(.*$").unwrap());
static RE_LOGIN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(http://127\.0\.0\.1:\d+/auth\S*)").unwrap());
static RE_AWAL_ERROR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\S+\s+\S+\s+ERROR\s*:\s*").unwrap());
/// Awalan baris log rclone: "2026/11/23 11:30:26 CRITICAL: ".
static RE_AWAL_LOG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\d{4}/\d\d/\d\d \d\d:\d\d:\d\d \w+\s*:\s*").unwrap());
static RE_HARI: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\d+)d").unwrap());
static RE_JAM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\d+)h").unwrap());
static RE_MENIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\d+)m(?:[^s]|$)").unwrap());
static RE_DETIK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"([\d.]+)s").unwrap());

/// "1.200 GiB" (format rclone) -> "1,17 GB"
fn ubah_ukuran(t: &str) -> String {
    match RE_UKURAN.captures(t) {
        Some(c) => ukuran(ke_byte(&c[1], c.get(2).map_or("", |m| m.as_str()))),
        None => t.to_string(),
    }
}

/// "1h2m3s" (format rclone) -> "1 jam 2 mnt"
fn ubah_durasi(d: &str) -> String {
    if d.is_empty() || d == "-" {
        return String::new();
    }
    let mut h = Vec::new();
    if let Some(c) = RE_HARI.captures(d) {
        h.push(format!("{} hari", &c[1]));
    }
    if let Some(c) = RE_JAM.captures(d) {
        h.push(format!("{} jam", &c[1]));
    }
    if let Some(c) = RE_MENIT.captures(d) {
        h.push(format!("{} mnt", &c[1]));
    }
    if h.len() < 2 {
        if let Some(c) = RE_DETIK.captures(d) {
            h.push(format!("{} dtk", c[1].parse::<f64>().unwrap_or(0.0) as i64));
        }
    }
    h.join(" ")
}

/// "pd-onedrive:Kerja/DLH" -> ("pd-onedrive", "Kerja/DLH")
fn pecah(j: &str) -> (String, String) {
    let rem = akun::remote_dari(j);
    if rem.is_empty() {
        return (String::new(), j.to_string());
    }
    (rem.clone(), j[rem.len() + 1..].trim_matches('/').to_string())
}

fn gabung(remote: &str, path: &str) -> String {
    let p = path.trim().trim_matches('"');
    if remote.is_empty() {
        return rapikan_sumber(p);
    }
    format!("{remote}:{}", p.replace('\\', "/").trim_matches('/'))
}

/// "Google Drive", "Google Drive 2", "OneDrive · Kantor"
fn nama_lokasi(a: Option<&Akun>) -> String {
    let Some(a) = a else { return "Komputer ini".into() };
    let dasar = match a.jenis.as_str() {
        "google" => "Google Drive",
        "sharepoint" => "SharePoint",
        _ => "OneDrive",
    };
    if a.label.starts_with(dasar) {
        a.label.clone()
    } else {
        format!("{dasar} {TITIK} {}", a.label)
    }
}

fn h_ada(h: &Value, k: &str) -> bool {
    h.get(k).is_some_and(|v| !v.is_null())
}

fn h_bil(h: &Value, k: &str) -> i64 {
    bil(h.get(k)).unwrap_or(0)
}

fn h_bool(h: &Value, k: &str, nilai: bool) -> bool {
    h.get(k) == Some(&Value::Bool(nilai))
}

#[derive(Default)]
struct Periksa {
    hasil_sumber: Teks,
    ket_sumber: Teks,
    hasil_tujuan: Teks,
    ket_tujuan: Teks,
    uji: Teks,
}

struct Progres {
    judul: String,
    persen: u32,
    warna: Color32,
    detail: String,
    file: String,
    catatan: Option<Teks>,
}

/// Satu panel penjelajah folder (Dari atau Ke).
#[derive(Default)]
struct Panel {
    /// "lokasi\u{1}jalur" yang terakhir dimuat; berubah = muat ulang.
    kunci: Option<String>,
    gen: u64,
    /// None = sedang dimuat.
    isi: Option<Isi>,
    pilih: Option<usize>,
    /// Alamat yang sedang diketik (klik bilah alamat untuk mengetik).
    edit: Option<String>,
    fokus_edit: bool,
}

struct Pratinjau {
    berkas: PathBuf,
    mulai: Instant,
    tunda: Duration,
    diminta: bool,
}

struct App {
    ctx: egui::Context,
    tx: Sender<Kabar>,
    rx: Receiver<Kabar>,
    exe_mesin: PathBuf,

    rclone: Option<PathBuf>,
    versi_rclone: String,
    status_siap: bool,
    akun: Vec<Akun>,
    sambungan: HashMap<String, Sambungan>,
    /// Akun yang gagal diuji karena internet/DNS putus, dan kapan diuji ulang.
    uji_ulang: HashMap<String, Instant>,
    /// Jumlah thread uji akun yang masih berjalan; uji ulang menunggu sampai nol.
    menguji: Arc<AtomicUsize>,
    ada_client_bawaan: bool,

    lok_sumber: String,
    isi_sumber: String,
    lok_tujuan: String,
    isi_tujuan: String,
    saran_sumber: Vec<(String, String)>,
    jenis_sumber: HashMap<String, String>,
    tujuan_manual: bool,
    rute_siap: bool,
    info_sumber: Teks,
    info_tujuan: Teks,

    periksa: Option<Periksa>,
    sudah_diperiksa: bool,
    ringkas_uji: Option<Value>,
    salin_setelah_uji: bool,

    progres: Option<Progres>,
    job_jalan: bool,
    progres_kunci: String,
    progres_sejak: Instant,
    progres_sejak_jam: String,
    label_salin: String,
    /// Batas kecepatan salin dalam bentuk --bwlimit rclone ("5M"), "off" = tanpa batas.
    batas: String,
    verifikasi: Teks,

    aksi: Option<Aksi>,
    url_login: Option<String>,
    pesan: Teks,
    log: String,
    log_terbuka: bool,
    dialog: Option<Dialog>,
    gen: u64,
    detak: Instant,
    boleh_tutup: bool,
    pratinjau: Option<Pratinjau>,

    /// Panel Dari (0) dan Ke (1).
    panel: [Panel; 2],
    /// Panel yang menerima lokasi saat item di daftar kiri diklik.
    aktif: usize,
    /// Pintasan di kelompok "Komputer ini": (nama, folder, glyph).
    pintasan: Vec<Pintasan>,
    logo: Option<egui::TextureHandle>,
}

impl App {
    fn baru(ctx: egui::Context, pratinjau: Option<Pratinjau>, dialog_awal: Option<String>, log_terbuka: bool) -> Self {
        let (tx, rx) = channel();
        let mut app = App {
            ctx,
            tx,
            rx,
            exe_mesin: jalur().dir_app.join("pindah.exe"),
            rclone: None,
            versi_rclone: String::new(),
            status_siap: false,
            akun: Vec::new(),
            sambungan: HashMap::new(),
            uji_ulang: HashMap::new(),
            menguji: Arc::new(AtomicUsize::new(0)),
            ada_client_bawaan: false,
            lok_sumber: String::new(),
            isi_sumber: String::new(),
            lok_tujuan: String::new(),
            isi_tujuan: String::new(),
            saran_sumber: Vec::new(),
            jenis_sumber: HashMap::new(),
            tujuan_manual: false,
            rute_siap: false,
            info_sumber: kosong(),
            info_tujuan: kosong(),
            periksa: None,
            sudah_diperiksa: false,
            ringkas_uji: None,
            salin_setelah_uji: false,
            progres: None,
            job_jalan: false,
            progres_kunci: String::new(),
            progres_sejak: Instant::now(),
            progres_sejak_jam: String::new(),
            label_salin: "Mulai salin".into(),
            batas: rute::baca_batas_kecepatan(),
            verifikasi: kosong(),
            aksi: None,
            url_login: None,
            pesan: ("Siap.".into(), REDUP),
            log: String::new(),
            log_terbuka,
            dialog: None,
            gen: 0,
            detak: Instant::now(),
            boleh_tutup: false,
            pratinjau,
            panel: Default::default(),
            aktif: DARI,
            pintasan: pintasan_lokal(),
            logo: None,
        };
        if !app.exe_mesin.is_file() {
            app.set_pesan(&format!("pindah.exe tidak ditemukan di {}", jalur().dir_app.display()), MERAH);
        }
        app.segarkan_status();
        app.perbarui_progres();
        if log_terbuka {
            app.tulis_log("Contoh baris log.");
        }
        if let Some(d) = dialog_awal {
            app.buka_dialog_pratinjau(&d);
        }
        app
    }

    // -------------------------------------------------------------- dasar

    fn set_pesan(&mut self, t: &str, w: Color32) {
        self.pesan = (t.to_string(), w);
    }

    fn tulis_log(&mut self, t: &str) {
        let (_, _, _, h, m, s) = waktu_lokal();
        self.log.push_str(&format!("{h:02}:{m:02}:{s:02}  {t}\n"));
        if self.log.len() > 400_000 {
            let potong = self.log.len() - 300_000;
            let i = self.log[potong..].find('\n').map_or(potong, |i| potong + i + 1);
            self.log.drain(..i);
        }
    }

    fn tanya(&mut self, judul: &str, isi: String, ya: &str, awas: bool, tujuan: Konfirmasi) {
        self.dialog = Some(Dialog::Tanya(Tanya {
            judul: judul.into(),
            isi,
            ya: ya.into(),
            tidak: "Batal".into(),
            awas,
            tujuan,
        }));
    }

    /// "" (Komputer ini) atau akun yang masih ada dan siap dipakai.
    fn lokasi_ada(&self, remote: &str) -> bool {
        remote.is_empty() || self.akun.iter().any(|a| a.nama == remote && a.siap)
    }

    fn akun_dari(&self, remote: &str) -> Option<Akun> {
        if remote.is_empty() {
            return None;
        }
        self.akun.iter().find(|a| a.nama == remote).cloned()
    }

    // -------------------------------------------------------------- kabar dari utas latar

    fn terima_kabar(&mut self) {
        while let Ok(k) = self.rx.try_recv() {
            match k {
                Kabar::Baris(b) => self.terima_baris(&b),
                Kabar::Hasil(v) => {
                    if let Some(a) = &mut self.aksi {
                        a.hasil = Some(v);
                    }
                }
                Kabar::AksiSelesai(kode) => self.aksi_selesai(kode),
                Kabar::Status { rclone, versi, akun, client_bawaan } => {
                    self.terima_status(rclone, versi, akun, client_bawaan)
                }
                Kabar::Sambungan(n, s) => {
                    if matches!(s, Sambungan::Jaringan(_)) {
                        self.uji_ulang.insert(n.clone(), Instant::now() + UJI_ULANG_JARINGAN);
                    } else {
                        self.uji_ulang.remove(&n);
                    }
                    self.sambungan.insert(n, s);
                }
                Kabar::Daftar { gen, isi } => {
                    if let Some(p) = self.panel.iter_mut().find(|p| p.gen == gen) {
                        let galat = match &isi {
                            Isi::Galat(e) => Some(e.clone()),
                            _ => None,
                        };
                        p.isi = Some(isi);
                        if let Some(e) = galat {
                            self.tulis_log(&format!("Folder tidak bisa dibuka: {}", e.trim()));
                        }
                    }
                }
                Kabar::Jelajah { gen, hasil } => {
                    if let Some(Dialog::Jelajah(j)) = &mut self.dialog {
                        if j.gen == gen {
                            j.terima(hasil);
                        }
                    }
                }
                Kabar::Situs { gen, hasil } => {
                    if let Some(Dialog::SharePoint(s)) = &mut self.dialog {
                        if s.gen == gen {
                            let mut p = Pemuat { tx: &self.tx, ctx: &self.ctx, gen: &mut self.gen };
                            s.terima_situs(hasil, &mut p);
                        }
                    }
                }
                Kabar::Pustaka { gen, hasil } => {
                    if let Some(Dialog::SharePoint(s)) = &mut self.dialog {
                        if s.gen == gen {
                            s.terima_pustaka(hasil);
                        }
                    }
                }
                Kabar::Jadwal { gen, daftar } => {
                    let baris: Vec<jadwal::Baris> = daftar
                        .into_iter()
                        .map(|(j, tugas, hidup)| {
                            let rute = self.rute_jadwal(&j.sumber, &j.tujuan);
                            jadwal::Baris { j, tugas, hidup, rute }
                        })
                        .collect();
                    if let Some(Dialog::Jadwal(d)) = &mut self.dialog {
                        if d.gen == gen {
                            d.baris = Some(baris);
                        }
                    }
                }
            }
        }
    }

    fn terima_baris(&mut self, b: &str) {
        let t = b.trim();
        if t.is_empty() || t.starts_with("==") {
            return;
        }
        self.tulis_log(t);
        if let Some(c) = RE_LOGIN.captures(t) {
            self.url_login = Some(c[1].to_string());
        }
    }

    fn segarkan_status(&mut self) {
        aksi::latar(&self.tx, &self.ctx, || {
            let rc = rclone::cari_rclone();
            let versi = if rc.is_some() { rclone::versi().trim_start_matches("rclone").trim().to_string() } else { String::new() };
            Kabar::Status { rclone: rc, versi, akun: akun::daftar_akun(), client_bawaan: akun::baca_client_bawaan().is_some() }
        });
    }

    fn terima_status(&mut self, rc: Option<PathBuf>, versi: String, akun_baru: Vec<Akun>, client_bawaan: bool) {
        self.rclone = rc;
        self.versi_rclone = versi;
        self.akun = akun_baru;
        self.ada_client_bawaan = client_bawaan;
        self.status_siap = true;
        // Pilihan lokasi yang akunnya sudah tidak ada kembali ke "Komputer ini".
        if !self.lokasi_ada(&self.lok_sumber) {
            self.lok_sumber.clear();
            self.isi_saran_sumber();
        }
        if !self.lokasi_ada(&self.lok_tujuan) {
            self.lok_tujuan.clear();
        }
        if self.rute_siap {
            self.sumber_berubah();
            self.tujuan_berubah();
        } else {
            self.isi_rute();
        }
        let nama: Vec<String> = self.akun.iter().filter(|a| a.siap).map(|a| a.nama.clone()).collect();
        self.sambungan.clear();
        self.uji_ulang.clear();
        self.uji_akun(nama);
        // Pengaturan yang sedang terbuka ikut diperbarui (mis. setelah client ID disimpan).
        if let Some(Dialog::Pengaturan(p)) = &self.dialog {
            let pesan = p.pesan.clone();
            self.dialog = Some(Dialog::Pengaturan(self.isi_pengaturan(pesan)));
        }
        // Isian jadwal baru yang dibuka sebelum rute termuat: isi dengan folder Dari/Ke sekarang.
        let kini = self.rute_kini();
        let nama = kini.as_ref().map(|r| self.nama_jadwal_bawaan(&r.sumber)).unwrap_or_default();
        if let Some(Dialog::JadwalIsian(s)) = &mut self.dialog {
            if s.id.is_none() && s.rute.is_none() {
                s.rute = kini.clone();
                if s.nama.is_empty() {
                    s.nama = nama;
                }
            }
            s.rute_kini = kini;
        }
    }

    /// Uji sambungan akun di thread latar, satu per satu (rclone menulis token baru ke rclone.conf).
    fn uji_akun(&mut self, nama: Vec<String>) {
        if nama.is_empty() {
            return;
        }
        let (tx, ctx, menguji) = (self.tx.clone(), self.ctx.clone(), self.menguji.clone());
        menguji.fetch_add(1, Ordering::SeqCst);
        std::thread::spawn(move || {
            for n in nama {
                let s = akun::cek_sambungan(&n);
                if tx.send(Kabar::Sambungan(n, s)).is_err() {
                    break;
                }
                ctx.request_repaint();
            }
            menguji.fetch_sub(1, Ordering::SeqCst);
        });
    }

    /// Akun yang tadi gagal karena internet/DNS diuji lagi sendiri, tanpa perlu membuka ulang aplikasi.
    fn uji_ulang_jaringan(&mut self) {
        if self.uji_ulang.is_empty() || self.menguji.load(Ordering::SeqCst) > 0 {
            return;
        }
        let kini = Instant::now();
        let jatuh: Vec<String> = self.uji_ulang.iter().filter(|(_, t)| **t <= kini).map(|(n, _)| n.clone()).collect();
        for n in &jatuh {
            self.uji_ulang.remove(n);
        }
        self.uji_akun(jatuh);
    }

    // -------------------------------------------------------------- aksi (pindah.exe)

    fn jalankan_aksi(&mut self, judul: &str, args: Vec<String>, setelah: Setelah) {
        if self.aksi.is_some() {
            return;
        }
        match aksi::mulai(&self.exe_mesin, &args, self.tx.clone(), self.ctx.clone()) {
            Ok(pid) => {
                self.aksi = Some(Aksi { judul: judul.into(), pid, hasil: None, setelah, dibatalkan: false });
                self.tulis_log(&format!("--- {judul}"));
                self.set_pesan(&format!("{judul}{ELIPSIS}"), TINTA);
            }
            Err(e) => {
                self.set_pesan(&format!("pindah.exe tidak bisa dijalankan: {e}"), MERAH);
                self.tulis_log(&format!("Kesalahan: pindah.exe tidak bisa dijalankan: {e}"));
                self.salin_setelah_uji = false;
            }
        }
    }

    fn klik_batal(&mut self) {
        if let Some(a) = &mut self.aksi {
            a.dibatalkan = true;
            win::bunuh_pohon(a.pid);
        }
        if self.aksi.is_some() {
            self.tulis_log("Dibatalkan oleh pengguna.");
        }
    }

    fn aksi_selesai(&mut self, kode: i32) {
        let Some(a) = self.aksi.take() else { return };
        self.url_login = None;
        if a.dibatalkan {
            self.salin_setelah_uji = false;
            self.set_pesan("Dibatalkan.", REDUP);
            self.segarkan_status();
            return;
        }
        let h = a.hasil.unwrap_or(Value::Null);
        match a.setelah {
            Setelah::Pasang => self.selesai_akun(
                kode,
                "rclone terpasang. Lanjutkan dengan menambah akun.",
                "Pemasangan rclone gagal. Lihat log aktivitas.",
            ),
            Setelah::Akun { ok, gagal } => self.selesai_akun(kode, &ok, &gagal),
            Setelah::AkunSharePoint => {
                self.selesai_akun(kode, "Login berhasil. Sekarang pilih situs SharePoint-nya.", "Login tidak selesai. Coba lagi.");
                let n = teks(h.get("Akun"));
                if kode == 0 && !n.is_empty() {
                    self.dialog = Some(Dialog::SharePoint(SharePoint::baru(&n)));
                }
            }
            Setelah::Periksa => self.selesai_periksa(kode, &h),
            Setelah::Uji => self.selesai_uji(kode, &h),
            Setelah::Salin => {
                if kode == 0 {
                    self.set_pesan("Penyalinan berjalan di latar belakang.", HIJAU);
                    // Hasil periksa/uji coba sudah basi; "Salin lagi"/"Lanjutkan" memeriksa ulang.
                    self.periksa = None;
                    self.ringkas_uji = None;
                    self.sudah_diperiksa = false;
                } else {
                    self.set_pesan("Penyalinan gagal dimulai. Lihat log aktivitas.", MERAH);
                    self.log_terbuka = true;
                }
                self.perbarui_progres();
            }
            Setelah::Verifikasi => self.selesai_verifikasi(kode, &h),
            Setelah::Jadwal { ok } => {
                if kode == 0 {
                    self.set_pesan(&ok, HIJAU);
                    self.buka_jadwal(Some((ok, HIJAU)));
                } else {
                    // Pesan galat terakhir dari pindah.exe (sudah masuk log aktivitas).
                    let e = self
                        .log
                        .lines()
                        .rev()
                        .map(str::trim)
                        .find(|b| !b.is_empty() && !b.starts_with("---"))
                        .unwrap_or("Lihat log aktivitas.")
                        .to_string();
                    self.set_pesan("Jadwal gagal diubah. Lihat log aktivitas.", MERAH);
                    self.buka_jadwal(Some((e, MERAH)));
                }
            }
        }
    }

    fn selesai_akun(&mut self, kode: i32, ok: &str, gagal: &str) {
        self.segarkan_status();
        if kode == 0 {
            self.set_pesan(ok, HIJAU);
        } else {
            self.set_pesan(gagal, MERAH);
            self.log_terbuka = true;
        }
    }

    // -------------------------------------------------------------- akun

    fn klik_tambah(&mut self, jenis: &str) {
        self.dialog = Some(Dialog::AkunBaru(AkunBaru {
            jenis: jenis.into(),
            label: String::new(),
            juga_sumber: false,
            client_bawaan: self.ada_client_bawaan,
            fokus: true,
        }));
    }

    /// Isi dialog Pengaturan dari keadaan saat ini (client ID aplikasi + akun Google).
    fn isi_pengaturan(&self, pesan: Option<(String, Color32)>) -> Pengaturan {
        Pengaturan {
            client: akun::info_client_bawaan().map(|(id, asal)| (akun::id_pendek(&id), asal.to_string())),
            google: self
                .akun
                .iter()
                .filter(|a| a.jenis == "google")
                .map(|a| AkunGoogle {
                    nama: a.nama.clone(),
                    label: a.label.clone(),
                    client_aplikasi: a.client_aplikasi,
                    client_sendiri: a.client_sendiri,
                })
                .collect(),
            pesan,
            sibuk: self.aksi.is_some() || self.job_jalan,
            lanjut_otomatis: rute::baca_lanjut_otomatis(),
        }
    }

    fn buka_pengaturan(&mut self, pesan: Option<(String, Color32)>) {
        self.dialog = Some(Dialog::Pengaturan(self.isi_pengaturan(pesan)));
    }

    // -------------------------------------------------------------- jadwal backup

    /// Membuka daftar jadwal; isinya dimuat di utas latar (schtasks /Query per jadwal).
    fn buka_jadwal(&mut self, pesan: Option<(String, Color32)>) {
        self.gen += 1;
        let gen = self.gen;
        aksi::latar(&self.tx, &self.ctx, move || {
            // Jadwal yang prosesnya masih hidup (job.json milik jadwal itu, PidInduk = pindah.exe).
            let hidup = baca_job()
                .filter(|x| bil(x.get("PidInduk")).is_some_and(|p| win::proses_hidup(p as u32, &["pindah"])))
                .map(|x| teks(x.get("Jadwal")))
                .unwrap_or_default();
            Kabar::Jadwal {
                gen,
                daftar: mj::baca_semua()
                    .into_iter()
                    .map(|j| {
                        let ada = mj::tugas_ada(&j.id);
                        let jalan = !hidup.is_empty() && hidup == j.id;
                        (j, ada, jalan)
                    })
                    .collect(),
            }
        });
        self.dialog = Some(Dialog::Jadwal(jadwal::Daftar {
            baris: None,
            gen,
            pesan,
            sibuk: self.aksi.is_some(),
            job_jalan: self.job_jalan,
        }));
    }

    /// Perubahan jadwal lewat pindah.exe; daftar tetap terbuka (dengan tanda sibuk) sampai selesai.
    fn aksi_jadwal(&mut self, judul: &str, args: Vec<String>, ok: &str) {
        self.jalankan_aksi(judul, args, Setelah::Jadwal { ok: ok.into() });
        if self.aksi.is_some() {
            self.buka_jadwal(Some((format!("{judul}{ELIPSIS}"), REDUP)));
        }
    }

    fn rute_jadwal(&self, sumber: &str, tujuan: &str) -> jadwal::Rute {
        jadwal::Rute {
            sumber: sumber.into(),
            tujuan: tujuan.into(),
            nama_sumber: self.nama_jalur(sumber),
            nama_tujuan: self.nama_jalur(tujuan),
        }
    }

    /// Folder yang sedang dibuka di panel Dari dan Ke, bila keduanya sudah layak dijadwalkan.
    fn rute_kini(&self) -> Option<jadwal::Rute> {
        let (s, t) = (self.jalur_sumber(), self.jalur_tujuan());
        if s.is_empty() || t.is_empty() {
            return None;
        }
        if self.lok_sumber.is_empty() && (!adalah_lokal(&s) || !Path::new(&s).is_dir()) {
            return None;
        }
        if self.lok_tujuan.is_empty() && !adalah_lokal(&t) {
            return None;
        }
        Some(self.rute_jadwal(&s, &t))
    }

    /// "Backup Dokumen" dari folder sumber; "Backup OneDrive" bila seluruh isi akun.
    fn nama_jadwal_bawaan(&self, sumber: &str) -> String {
        let (rem, p) = pecah(sumber);
        let akhir = p.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).next().unwrap_or("").trim_end_matches(':');
        if akhir.is_empty() {
            return format!("Backup {}", nama_lokasi(self.akun_dari(&rem).as_ref()));
        }
        format!("Backup {akhir}")
    }

    /// Akun Google baru otomatis memakai client ID aplikasi bila ada (diatur di Pengaturan).
    fn hubungkan_baru(&mut self, jenis: &str, label: &str, izin_penuh: bool) {
        let nama = akun::remote_baru(jenis);
        let mut a = v!["hubungkan", "-Akun", nama, "-Jenis", jenis, "-Ya"];
        if !label.is_empty() {
            a.extend(v!["-Label", label]);
        }
        if izin_penuh {
            a.push("-IzinPenuh".into());
        }
        let layanan = if jenis == "google" { "Google" } else { "Microsoft" };
        let setelah = if jenis == "sharepoint" {
            Setelah::AkunSharePoint
        } else {
            Setelah::Akun { ok: "Akun terhubung.".into(), gagal: "Login tidak selesai. Coba lagi.".into() }
        };
        self.jalankan_aksi(&format!("Menunggu login {layanan} di browser"), a, setelah);
    }

    fn akun_aksi(&mut self, nama: &str, aksi_akun: &str) {
        let label = nama_lokasi(self.akun_dari(nama).as_ref());
        let n = nama.to_string();
        match aksi_akun {
            "ulang" => self.tanya(
                "Login ulang",
                format!("Login ulang ke {label}?\n\nBrowser akan terbuka untuk login."),
                "Login ulang",
                false,
                Konfirmasi::Ulang(n),
            ),
            "penuh" => self.tanya(
                "Izin penuh",
                format!("Beri {label} izin baca-tulis penuh?\n\nIzin ini dibutuhkan agar file di akun ini bisa disalin ke tempat lain. Browser akan terbuka untuk login dan menyetujui izin baru."),
                "Lanjut ke login",
                false,
                Konfirmasi::Penuh(n),
            ),
            "situs" => self.dialog = Some(Dialog::SharePoint(SharePoint::baru(&n))),
            "lepas" => self.tanya(
                "Lepas akun",
                format!("Lepas {label} dari LELEMOVE?\n\nFile di akun tersebut TIDAK terhapus; hanya sambungan di komputer ini yang dihapus."),
                "Lepas akun",
                true,
                Konfirmasi::Lepas(n),
            ),
            _ => {}
        }
    }

    fn konfirmasi(&mut self, k: Konfirmasi) {
        let akun_ok = |ok: &str, gagal: &str| Setelah::Akun { ok: ok.into(), gagal: gagal.into() };
        match k {
            Konfirmasi::PasangRclone => self.jalankan_aksi("Memasang rclone", v!["pasang", "-Ya"], Setelah::Pasang),
            Konfirmasi::Ulang(n) => self.jalankan_aksi(
                "Menunggu login di browser",
                v!["hubungkan", "-Akun", n, "-Ulang", "-Ya"],
                akun_ok("Login ulang berhasil.", "Login ulang tidak selesai."),
            ),
            Konfirmasi::Penuh(n) => self.jalankan_aksi(
                "Menunggu login Google di browser",
                v!["hubungkan", "-Akun", n, "-IzinPenuh", "-Ya"],
                akun_ok("Izin penuh diberikan.", "Login tidak selesai."),
            ),
            Konfirmasi::Bawaan(n) => self.jalankan_aksi(
                "Menyimpan client ID aplikasi",
                v!["bawaan", "-Akun", n],
                akun_ok("Client ID aplikasi tersimpan.", "Client ID aplikasi gagal disimpan. Lihat log aktivitas."),
            ),
            Konfirmasi::HapusClient => {
                let ada = akun::hapus_client_pengguna();
                self.tulis_log("Client ID aplikasi yang disimpan di komputer ini dihapus.");
                self.segarkan_status();
                let pesan = if ada { "Client ID aplikasi dihapus." } else { "Tidak ada client ID yang disimpan di komputer ini." };
                self.buka_pengaturan(Some((pesan.into(), REDUP)));
            }
            Konfirmasi::HapusJadwal(id) => {
                self.aksi_jadwal("Menghapus jadwal", v!["jadwal", "-Id", id, "-Hapus"], "Jadwal dihapus.")
            }
            Konfirmasi::Lepas(n) => self.jalankan_aksi(
                "Melepas akun",
                v!["lepas", "-Akun", n, "-Ya"],
                akun_ok("Akun dilepas.", "Akun gagal dilepas. Lihat log aktivitas."),
            ),
            Konfirmasi::MulaiSalin => {
                self.verifikasi = kosong();
                self.jalankan_aksi(
                    "Memulai penyalinan",
                    v!["salin", "-Latar", "-Ya", "-BatasKecepatan", &self.batas],
                    Setelah::Salin,
                );
            }
            Konfirmasi::Henti => {
                Mesin::baru(Opsi::default()).berhenti();
                self.tulis_log("Penyalinan dihentikan.");
                self.set_pesan("Penyalinan dihentikan.", REDUP);
                self.perbarui_progres();
            }
            Konfirmasi::TutupApp => {
                self.klik_batal();
                self.boleh_tutup = true;
                self.ctx.send_viewport_cmd(ViewportCommand::Close);
            }
        }
    }

    fn tangani_dialog(&mut self, h: Hasil) {
        match h {
            Hasil::Batal => {}
            Hasil::BatalSharePoint => self.set_pesan("Situs SharePoint belum dipilih; akun ini belum bisa dipakai.", KUNING),
            Hasil::Ya(k) => self.konfirmasi(k),
            Hasil::AkunBaru { jenis, label, izin_penuh } => self.hubungkan_baru(&jenis, &label, izin_penuh),
            Hasil::ClientId { id, secret } => {
                // Disimpan langsung oleh jendela (secret tidak lewat baris perintah).
                akun::simpan_client_bawaan(&id, &secret);
                self.tulis_log("Client ID aplikasi disimpan.");
                self.ada_client_bawaan = true;
                self.segarkan_status();
                self.buka_pengaturan(Some((
                    "Client ID aplikasi tersimpan. Akun Google baru otomatis memakainya; akun yang sudah ada perlu login ulang lewat tombol di bawah.".into(),
                    HIJAU,
                )));
            }
            Hasil::KembaliPengaturan => self.buka_pengaturan(None),
            Hasil::GantiClient => self.dialog = Some(Dialog::ClientId(ClientId::baru())),
            Hasil::HapusClient => self.tanya(
                "Hapus client ID aplikasi",
                "Hapus client ID Google yang disimpan di komputer ini?\n\nAkun Google yang sudah terhubung tetap bisa dipakai. Akun Google baru akan memakai client ID bawaan installer (bila ada) atau client ID bersama rclone.".into(),
                "Hapus",
                true,
                Konfirmasi::HapusClient,
            ),
            Hasil::LoginClient(nama) => self.jalankan_aksi(
                "Menunggu login Google di browser",
                v!["hubungkan", "-Akun", nama, "-ClientAplikasi", "-Ya"],
                Setelah::Akun {
                    ok: "Akun Google kini memakai client ID aplikasi.".into(),
                    gagal: "Login tidak selesai. Coba lagi lewat Pengaturan.".into(),
                },
            ),
            Hasil::LanjutOtomatis(aktif) => {
                rute::simpan_lanjut_otomatis(aktif);
                self.tulis_log(if aktif {
                    "Lanjut otomatis saat jaringan putus: aktif."
                } else {
                    "Lanjut otomatis saat jaringan putus: nonaktif."
                });
                // Dialog tetap terbuka; hanya pesannya yang diperbarui.
                let pesan = if aktif {
                    "Berlaku untuk penyalinan berikutnya (atau saat Lanjutkan)."
                } else {
                    "Nonaktif: penyalinan yang putus karena jaringan berhenti dan menunggu Anda klik Lanjutkan."
                };
                self.buka_pengaturan(Some((pesan.into(), REDUP)));
            }
            Hasil::AmbilClient(nama) => {
                let label = nama_lokasi(self.akun_dari(&nama).as_ref());
                self.tanya(
                    "Client ID aplikasi",
                    format!("Jadikan client ID milik {label} sebagai client ID aplikasi?\n\nClient ID ini akan dipakai untuk akun Google yang ditambahkan berikutnya, dan ikut dibawa installer yang dibangun sesudahnya. Siapa pun yang memegang installer itu bisa membaca client ID dan secret-nya."),
                    "Jadikan client ID aplikasi",
                    false,
                    Konfirmasi::Bawaan(nama),
                );
            }
            Hasil::KembaliJadwal => self.buka_jadwal(None),
            Hasil::JadwalBaru => {
                let kini = self.rute_kini();
                let nama = kini.as_ref().map(|r| self.nama_jadwal_bawaan(&r.sumber)).unwrap_or_default();
                self.dialog = Some(Dialog::JadwalIsian(jadwal::Isian::baru(kini, nama)));
            }
            Hasil::JadwalUbah(id) => match mj::cari(&id) {
                Some(j) => {
                    let rute = self.rute_jadwal(&j.sumber, &j.tujuan);
                    self.dialog = Some(Dialog::JadwalIsian(jadwal::Isian::dari(&j, rute, self.rute_kini())));
                }
                None => self.buka_jadwal(Some(("Jadwal itu sudah tidak ada.".into(), KUNING))),
            },
            Hasil::JadwalSimpan { id, nama, sumber, tujuan, jam, hari, verifikasi } => {
                let mut a = v!["jadwal", "-Tambah", "-Nama", nama, "-Sumber", sumber, "-Tujuan", tujuan, "-Jam", jam, "-Hari", hari];
                if !verifikasi {
                    a.push("-TanpaVerifikasi".into());
                }
                let baru = id.is_none();
                if let Some(id) = id {
                    a.extend(v!["-Id", id]);
                }
                let ok = if baru { "Jadwal dipasang." } else { "Jadwal diperbarui." };
                self.aksi_jadwal("Menyimpan jadwal", a, ok);
            }
            Hasil::JadwalJalankan(id) => {
                // Berjalan lepas dari jendela (seperti tugas Windows), progresnya dibaca dari job.json.
                match win::mulai_terlepas(&self.exe_mesin, &v!["jadwal", "-Jalankan", "-Id", id]) {
                    Ok(_) => {
                        self.tulis_log("--- Backup terjadwal dijalankan sekarang");
                        self.set_pesan("Backup terjadwal berjalan di latar belakang.", HIJAU);
                        self.verifikasi = kosong();
                    }
                    Err(e) => self.buka_jadwal(Some((format!("pindah.exe tidak bisa dijalankan: {e}"), MERAH))),
                }
            }
            Hasil::JadwalAktif(id, aktif) => {
                let (judul, saklar, ok) = if aktif {
                    ("Mengaktifkan jadwal", "-Aktifkan", "Jadwal aktif.")
                } else {
                    ("Menonaktifkan jadwal", "-Nonaktifkan", "Jadwal nonaktif; tugas Windows-nya tidak berjalan sampai diaktifkan lagi.")
                };
                self.aksi_jadwal(judul, v!["jadwal", "-Id", id, saklar], ok);
            }
            Hasil::JadwalHapus(id) => {
                let nama = mj::cari(&id).map(|j| j.nama).unwrap_or_else(|| id.clone());
                self.tanya(
                    "Hapus jadwal",
                    format!("Hapus jadwal \"{nama}\"?\n\nTugas Windows-nya ikut dihapus. File yang sudah dicadangkan di tujuan TIDAK terhapus."),
                    "Hapus jadwal",
                    true,
                    Konfirmasi::HapusJadwal(id),
                );
            }
            Hasil::JadwalLog(log) => {
                win::buka(&log);
                self.buka_jadwal(None);
            }
            Hasil::Folder { jalur, untuk_sumber } => {
                if untuk_sumber {
                    self.isi_sumber = jalur;
                    self.sumber_berubah();
                } else {
                    self.isi_tujuan = jalur;
                    self.tujuan_manual = true;
                    self.sudah_diperiksa = false;
                    self.tujuan_berubah();
                }
            }
            Hasil::Pustaka { remote, situs_nama, pustaka_id, pustaka_nama } => self.jalankan_aksi(
                "Memakai pustaka SharePoint",
                v!["pustaka", "-Akun", remote, "-Pustaka", pustaka_id, "-SitusNama", situs_nama, "-PustakaNama", pustaka_nama],
                Setelah::Akun {
                    ok: "Pustaka SharePoint siap dipakai sebagai sumber atau tujuan.".into(),
                    gagal: "Pustaka gagal dipakai. Lihat log aktivitas.".into(),
                },
            ),
        }
    }

    // -------------------------------------------------------------- sumber & tujuan

    fn jalur_sumber(&self) -> String {
        gabung(&self.lok_sumber, &self.isi_sumber)
    }

    fn jalur_tujuan(&self) -> String {
        gabung(&self.lok_tujuan, &self.isi_tujuan)
    }

    /// Nama yang mudah dibaca untuk dialog dan hasil verifikasi.
    fn nama_jalur(&self, j: &str) -> String {
        let (rem, p) = pecah(j);
        if rem.is_empty() {
            return match self.jenis_sumber.get(j) {
                Some(k) => format!("{j} ({k})"),
                None => j.to_string(),
            };
        }
        let n = nama_lokasi(self.akun_dari(&rem).as_ref());
        if p.is_empty() {
            format!("{n} (seluruh isi)")
        } else {
            format!("{n} / {p}")
        }
    }

    /// Nama singkat untuk bilah transfer: nama folder terakhir di komputer, atau "Akun / folder" di cloud.
    fn nama_pendek(&self, j: &str) -> String {
        if adalah_lokal(j) {
            return Path::new(j).file_name().map(|n| format!("\"{}\"", n.to_string_lossy())).unwrap_or_else(|| j.to_string());
        }
        self.nama_jalur(j)
    }

    fn lokasi_tujuan_bawaan(&self) -> String {
        let siap = || self.akun.iter().filter(|a| a.siap);
        siap().find(|a| a.jenis == "google").or_else(|| siap().next()).map(|a| a.nama.clone()).unwrap_or_default()
    }

    /// Saran folder sumber: folder OneDrive yang tersinkron di komputer ini.
    fn isi_saran_sumber(&mut self) {
        self.saran_sumber.clear();
        self.jenis_sumber.clear();
        if !self.lok_sumber.is_empty() {
            return;
        }
        for o in daftar_onedrive() {
            let j = match o.jenis.as_str() {
                "Pribadi" => "OneDrive pribadi",
                "Kantor/Sekolah" => "OneDrive kantor/sekolah",
                _ => "Folder OneDrive",
            };
            self.jenis_sumber.insert(o.folder.clone(), j.into());
            self.saran_sumber.push((o.folder, j.into()));
        }
    }

    fn set_tujuan_bawaan(&mut self) {
        let s = self.jalur_sumber();
        let rem_t = self.lok_tujuan.clone();
        self.isi_tujuan = if s.is_empty() {
            String::new()
        } else {
            let b = tujuan_bawaan(&s, &rem_t);
            if rem_t.is_empty() {
                b
            } else {
                pecah(&b).1
            }
        };
        self.tujuan_manual = false;
        self.tujuan_berubah();
    }

    /// Muat rute tersimpan (atau bawaan) ke isian saat aplikasi dibuka.
    fn isi_rute(&mut self) {
        let r = rute::baca_pengaturan();
        let sumber = r.sumber.as_deref().map(pecah).filter(|(rem, _)| self.lokasi_ada(rem));
        let tujuan = r.tujuan.as_deref().map(pecah).filter(|(rem, _)| self.lokasi_ada(rem));
        match sumber {
            Some((rem, p)) => {
                self.lok_sumber = rem;
                self.isi_saran_sumber();
                self.isi_sumber = p;
            }
            _ => {
                self.lok_sumber.clear();
                self.isi_saran_sumber();
                self.isi_sumber = self.saran_sumber.first().map(|s| s.0.clone()).unwrap_or_default();
            }
        }
        match tujuan {
            Some((rem, p)) => {
                self.lok_tujuan = rem.clone();
                self.isi_tujuan = p;
                self.tujuan_manual = r.sumber.as_deref().map(|s| tujuan_bawaan(s, &rem)) != r.tujuan;
            }
            _ => {
                self.lok_tujuan = self.lokasi_tujuan_bawaan();
                self.tujuan_manual = false;
            }
        }
        self.rute_siap = true;
        self.sumber_berubah();
        self.tujuan_berubah();
    }

    // -------------------------------------------------------------- panel Dari/Ke

    fn lok(&self, sisi: usize) -> &str {
        if sisi == DARI {
            &self.lok_sumber
        } else {
            &self.lok_tujuan
        }
    }

    /// Jalur folder di dalam lokasinya: "D:\Data" (komputer) atau "Arsip/2024" (cloud, tanpa garis miring awal).
    fn dalam(&self, sisi: usize) -> String {
        let j = if sisi == DARI { self.jalur_sumber() } else { self.jalur_tujuan() };
        if self.lok(sisi).is_empty() {
            j
        } else {
            pecah(&j).1
        }
    }

    /// Mengganti lokasi satu panel. `isi` None = folder bawaan lokasi itu.
    fn pilih_lokasi(&mut self, sisi: usize, lok: &str, isi: Option<String>) {
        self.aktif = sisi;
        self.panel[sisi].edit = None;
        if sisi == DARI {
            self.lok_sumber = lok.to_string();
            self.isi_saran_sumber();
            self.isi_sumber = isi.unwrap_or_default();
            self.sumber_berubah();
            return;
        }
        let ganti = self.lok_tujuan != lok;
        self.lok_tujuan = lok.to_string();
        match isi {
            Some(i) => self.set_isi(KE, i),
            None if ganti => self.lokasi_tujuan_berubah(),
            None => {}
        }
    }

    /// Pindah ke folder lain di lokasi yang sama (klik dua kali, bilah alamat, Naik).
    fn set_isi(&mut self, sisi: usize, isi: String) {
        self.aktif = sisi;
        self.panel[sisi].edit = None;
        if sisi == DARI {
            self.isi_sumber = isi;
            self.sumber_berubah();
        } else {
            self.isi_tujuan = isi;
            self.tujuan_manual = true;
            self.sudah_diperiksa = false;
            self.tujuan_berubah();
        }
    }

    fn buka_sub(&mut self, sisi: usize, nama: &str) {
        let skrg = self.dalam(sisi);
        let baru = if !self.lok(sisi).is_empty() {
            if skrg.is_empty() {
                nama.to_string()
            } else {
                format!("{skrg}/{nama}")
            }
        } else if skrg.is_empty() {
            nama.to_string() // akar drive, mis. "D:\"
        } else {
            Path::new(&skrg).join(nama).to_string_lossy().to_string()
        };
        self.set_isi(sisi, baru);
    }

    fn naik(&mut self, sisi: usize) {
        let skrg = self.dalam(sisi);
        let baru = if !self.lok(sisi).is_empty() {
            skrg.rfind('/').map(|i| skrg[..i].to_string()).unwrap_or_default()
        } else {
            Path::new(&skrg).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()
        };
        self.set_isi(sisi, baru);
    }

    fn bisa_naik(&self, sisi: usize) -> bool {
        !self.dalam(sisi).is_empty()
    }

    /// Memuat isi folder panel di thread latar bila lokasi/jalurnya berubah (atau `paksa`).
    fn muat_panel(&mut self, sisi: usize, paksa: bool) {
        if !self.rute_siap {
            return;
        }
        let lok = self.lok(sisi).to_string();
        let dalam = self.dalam(sisi);
        let kunci = format!("{lok}\u{1}{dalam}");
        if !paksa && self.panel[sisi].kunci.as_deref() == Some(kunci.as_str()) {
            return;
        }
        self.gen += 1;
        let gen = self.gen;
        let p = &mut self.panel[sisi];
        p.kunci = Some(kunci);
        p.gen = gen;
        p.isi = None;
        p.pilih = None;
        aksi::latar(&self.tx, &self.ctx, move || {
            let isi = if lok.is_empty() {
                isi_lokal(&dalam)
            } else {
                match akun::daftar_folder_cloud(&lok, &dalam) {
                    Ok(d) => Isi::Folder(d),
                    Err(e) if e.to_lowercase().contains("not found") => Isi::BelumAda,
                    Err(e) => Isi::Galat(e),
                }
            };
            Kabar::Daftar { gen, isi }
        });
    }

    fn panel_memuat(&self) -> bool {
        self.panel.iter().any(|p| p.kunci.is_some() && p.isi.is_none())
    }

    fn sumber_berubah(&mut self) {
        let rem = self.lok_sumber.clone();
        let s = self.jalur_sumber();
        self.sudah_diperiksa = false;
        self.ringkas_uji = None;
        self.periksa = None;
        self.info_sumber = if rem.is_empty() {
            if s.is_empty() {
                ("Buka drive dan folder yang mau disalin.".into(), REDUP)
            } else if adalah_lokal(&s) && Path::new(&s).is_dir() {
                match self.jenis_sumber.get(&s) {
                    Some(j) => (format!("{j}. Seluruh isinya disalin."), REDUP),
                    None => ("Seluruh isi folder ini disalin.".into(), REDUP),
                }
            } else if adalah_lokal(&s) {
                ("Folder tidak ditemukan.".into(), MERAH)
            } else {
                ("Tulis alamat lengkap, misalnya D:\\Data.".into(), KUNING)
            }
        } else {
            let a = self.akun_dari(&rem);
            let p = pecah(&s).1;
            let mut t = if !p.is_empty() {
                "Seluruh isi folder ini disalin".to_string()
            } else if a.as_ref().is_some_and(|a| a.jenis == "sharepoint") {
                "Seluruh isi pustaka disalin".to_string()
            } else {
                "Seluruh isi akun disalin".to_string()
            };
            let mut w = REDUP;
            match &a {
                Some(a) if a.jenis == "google" && a.scope != "drive" => {
                    t = "Akun ini hanya diizinkan sebagai tujuan. Klik \u{22EF} pada akun ini di daftar kiri, lalu \"Beri izin penuh\" agar bisa jadi sumber.".into();
                    w = KUNING;
                }
                Some(a) if a.jenis == "sharepoint" => t += &format!(" {TITIK} pustaka {}, situs {}.", a.drive, a.situs),
                Some(a) if !a.drive.is_empty() => t += &format!(" {TITIK} drive {}.", a.drive),
                _ => t.push('.'),
            }
            (t, w)
        };
        if !self.tujuan_manual {
            self.set_tujuan_bawaan();
        }
    }

    fn tujuan_berubah(&mut self) {
        let rem_t = self.lok_tujuan.clone();
        let t = self.jalur_tujuan();
        if rem_t.is_empty() {
            self.info_tujuan = if t.is_empty() {
                ("Buka drive dan folder tujuan.".into(), REDUP)
            } else if adalah_lokal(&t) && Path::new(&t).is_dir() {
                ("Disalin ke folder ini.".into(), REDUP)
            } else if adalah_lokal(&t) {
                ("Disalin ke folder ini; dibuat saat penyalinan dimulai.".into(), REDUP)
            } else {
                ("Tulis alamat lengkap, misalnya D:\\Salinan.".into(), KUNING)
            };
            return;
        }
        let a = self.akun_dari(&rem_t);
        let p = pecah(&t).1;
        let mut teks_info =
            if p.is_empty() { "Disalin langsung ke folder teratas akun".to_string() } else { "Disalin ke folder ini (dibuat bila belum ada)".to_string() };
        let mut w = REDUP;
        match &a {
            Some(a) if a.jenis == "google" && !a.client_sendiri => {
                teks_info += ". Akun ini memakai client ID bersama rclone yang sering dibatasi Google.";
                w = KUNING;
            }
            Some(a) if a.jenis == "sharepoint" => {
                teks_info += &format!(
                    " {TITIK} pustaka {}, situs {}. File Office akan ditambahi metadata oleh SharePoint (wajar).",
                    a.drive, a.situs
                )
            }
            Some(a) if !a.drive.is_empty() => teks_info += &format!(" {TITIK} drive {}.", a.drive),
            _ => teks_info.push('.'),
        }
        self.info_tujuan = (teks_info, w);
    }

    fn lokasi_tujuan_berubah(&mut self) {
        if !self.tujuan_manual {
            self.set_tujuan_bawaan();
            return;
        }
        // Pindah antara komputer dan cloud: format jalurnya berbeda, jadi isi ulang.
        let lokal = adalah_lokal(&self.isi_tujuan);
        if self.lok_tujuan.is_empty() != lokal {
            self.set_tujuan_bawaan();
            return;
        }
        self.sudah_diperiksa = false;
        self.tujuan_berubah();
    }

    /// Membaca isian, memvalidasi, dan menyimpannya sebagai rute (dibaca juga oleh pindah.exe).
    fn ambil_isian(&mut self) -> Option<(String, String)> {
        let s = self.jalur_sumber();
        let t = self.jalur_tujuan();
        if self.lok_sumber.is_empty() {
            if s.is_empty() {
                self.set_pesan("Pilih folder sumber dulu.", MERAH);
                return None;
            }
            if !adalah_lokal(&s) || !Path::new(&s).is_dir() {
                self.set_pesan("Folder sumber tidak ditemukan.", MERAH);
                return None;
            }
        }
        if self.lok_tujuan.is_empty() && (t.is_empty() || !adalah_lokal(&t)) {
            self.set_pesan("Tulis folder tujuan lengkap, misalnya D:\\Salinan.", MERAH);
            return None;
        }
        if self.rclone.is_none() {
            self.set_pesan("Pasang rclone dulu (tombol di daftar kiri).", MERAH);
            return None;
        }
        rute::simpan_pengaturan(Some(&s), Some(&t));
        Some((s, t))
    }

    fn klik_telusuri(&mut self, untuk_sumber: bool) {
        let (lok, jalur_skrg) = if untuk_sumber {
            (self.lok_sumber.clone(), self.jalur_sumber())
        } else {
            (self.lok_tujuan.clone(), self.jalur_tujuan())
        };
        if !lok.is_empty() {
            let judul = format!(
                "Pilih folder {} di {}",
                if untuk_sumber { "SUMBER" } else { "TUJUAN" },
                nama_lokasi(self.akun_dari(&lok).as_ref())
            );
            let awal = pecah(&jalur_skrg).1;
            let mut p = Pemuat { tx: &self.tx, ctx: &self.ctx, gen: &mut self.gen };
            let j = Jelajah::baru(&lok, &awal, judul, untuk_sumber, &mut p);
            self.dialog = Some(Dialog::Jelajah(j));
            return;
        }
        let mut fd = rfd::FileDialog::new().set_title(if untuk_sumber {
            "Pilih folder yang mau disalin"
        } else {
            "Pilih folder tujuan (boleh membuat folder baru)"
        });
        if adalah_lokal(&jalur_skrg) && Path::new(&jalur_skrg).is_dir() {
            fd = fd.set_directory(&jalur_skrg);
        }
        if let Some(p) = fd.pick_folder() {
            let s = p.to_string_lossy().to_string();
            if untuk_sumber {
                self.isi_sumber = s;
                self.sumber_berubah();
            } else {
                self.isi_tujuan = s;
                self.tujuan_manual = true;
                self.sudah_diperiksa = false;
                self.tujuan_berubah();
            }
        }
    }

    fn klik_tukar(&mut self) {
        let (rem_s, rem_t) = (self.lok_sumber.clone(), self.lok_tujuan.clone());
        let (p_s, p_t) = (self.isi_sumber.clone(), self.isi_tujuan.clone());
        self.lok_sumber = rem_t;
        self.isi_saran_sumber();
        self.isi_sumber = p_t;
        self.lok_tujuan = rem_s;
        self.isi_tujuan = p_s;
        self.tujuan_manual = true;
        self.sumber_berubah();
        self.tujuan_berubah();
        self.set_pesan("Sumber dan tujuan ditukar.", REDUP);
    }

    fn klik_buka_tujuan(&mut self) {
        if self.lok_tujuan.is_empty() {
            let t = self.jalur_tujuan();
            if !t.is_empty() && Path::new(&t).is_dir() {
                win::buka(&t);
            } else {
                self.set_pesan("Folder tujuan belum ada.", REDUP);
            }
            return;
        }
        match self.akun_dari(&self.lok_tujuan).map(|a| a.jenis) .as_deref() {
            Some("google") => win::buka("https://drive.google.com/drive/my-drive"),
            Some("sharepoint") => win::buka("https://www.office.com/launch/sharepoint"),
            _ => win::buka("https://www.office.com/launch/onedrive"),
        }
    }

    // -------------------------------------------------------------- periksa & uji

    fn klik_periksa(&mut self) {
        if self.ambil_isian().is_none() {
            self.salin_setelah_uji = false;
            return;
        }
        self.periksa = Some(Periksa {
            hasil_sumber: (ELIPSIS.into(), TINTA),
            ket_sumber: kosong(),
            hasil_tujuan: (ELIPSIS.into(), TINTA),
            ket_tujuan: kosong(),
            uji: kosong(),
        });
        self.jalankan_aksi("Memeriksa isi sumber dan ruang tujuan", v!["periksa"], Setelah::Periksa);
    }

    /// Hasil periksa/uji/verifikasi melaporkan login kedaluwarsa: tandai akunnya.
    fn catat_perlu_login(&mut self, h: &Value) -> Option<String> {
        let n = teks(h.get("PerluLogin"));
        if n.is_empty() {
            return None;
        }
        self.sambungan.insert(n.clone(), Sambungan::Kedaluwarsa);
        Some(nama_lokasi(self.akun_dari(&n).as_ref()))
    }

    fn selesai_periksa(&mut self, kode: i32, h: &Value) {
        let perlu_login = self.catat_perlu_login(h);
        let mut p = self.periksa.take().unwrap_or_default();
        if h_ada(h, "SumberJumlah") {
            p.hasil_sumber = (
                format!("{} file  {TITIK}  {}", angka(h_bil(h, "SumberJumlah")), ukuran(h_bil(h, "SumberBytes") as f64)),
                TINTA,
            );
            let mut ket = String::new();
            if h_bil(h, "HanyaCloudJumlah") > 0 {
                ket = format!(
                    "{} file masih \"hanya online\"; OneDrive mengunduhnya dulu ke disk (sisa {}).",
                    angka(h_bil(h, "HanyaCloudJumlah")),
                    ukuran(h_bil(h, "DiskBebas") as f64)
                );
            }
            p.ket_sumber = if h_bool(h, "DiskCukup", false) {
                (format!("{ket} Ruang disk tidak cukup; salin per subfolder."), KUNING)
            } else {
                (ket, REDUP)
            };
        } else {
            p.hasil_sumber = ("Tidak terbaca".into(), MERAH);
            p.ket_sumber = match &perlu_login {
                Some(l) => (format!("Login {l} kedaluwarsa. Klik Login ulang pada akun itu di daftar kiri."), MERAH),
                None => ("Lihat log aktivitas.".into(), REDUP),
            };
        }

        if h_ada(h, "TujuanBebas") {
            let w = if h_bool(h, "TujuanCukup", false) { MERAH } else { TINTA };
            p.hasil_tujuan = (format!("Sisa {}", ukuran(h_bil(h, "TujuanBebas") as f64)), w);
            let mut ket = if !teks(h.get("TujuanJenis")).is_empty() {
                format!("dari {}. Perlu disalin {}.", ukuran(h_bil(h, "TujuanTotal") as f64), ukuran(h_bil(h, "Butuh") as f64))
            } else {
                format!("ruang disk. Perlu disalin {}.", ukuran(h_bil(h, "Butuh") as f64))
            };
            if h_bil(h, "TujuanSudah") > 0 {
                ket += &format!(" {} sudah ada di tujuan.", ukuran(h_bil(h, "TujuanSudah") as f64));
            }
            if h_bool(h, "TujuanCukup", false) {
                ket += " Tidak cukup: kosongkan ruang/sampah di tujuan atau pilih subfolder.";
            }
            p.ket_tujuan = (ket, REDUP);
        } else if h_bool(h, "TujuanTerhubung", true) {
            p.hasil_tujuan = ("Kuota tidak dibatasi".into(), TINTA);
            p.ket_tujuan = kosong();
        } else if h_ada(h, "TujuanGalat") {
            p.hasil_tujuan = ("Tidak bisa diakses".into(), MERAH);
            p.ket_tujuan = match &perlu_login {
                Some(l) => (format!("Login {l} kedaluwarsa. Klik Login ulang pada akun itu di daftar kiri."), MERAH),
                None => ("Coba \"Login ulang\" lewat menu \u{22EF} akun itu di daftar kiri.".into(), REDUP),
            };
        } else {
            p.hasil_tujuan = ("Belum diperiksa".into(), REDUP);
            p.ket_tujuan = ("Lihat log aktivitas.".into(), REDUP);
        }

        if (kode == 0 || kode == 3) && h_bool(h, "TujuanTerhubung", true) {
            p.uji = (format!("Uji coba{ELIPSIS}"), REDUP);
            self.periksa = Some(p);
            self.jalankan_aksi("Uji coba (tidak menyalin apa pun)", v!["uji"], Setelah::Uji);
            return;
        }
        self.salin_setelah_uji = false;
        if kode == 2 {
            p.uji = ("Ruang di tujuan tidak cukup untuk folder ini.".into(), MERAH);
            self.set_pesan("Ruang di tujuan tidak cukup.", MERAH);
        } else {
            p.uji = ("Pemeriksaan belum lengkap. Lihat log aktivitas.".into(), KUNING);
            self.log_terbuka = true;
            self.set_pesan("Pemeriksaan selesai dengan catatan.", KUNING);
        }
        self.periksa = Some(p);
    }

    fn set_uji(&mut self, t: Teks) {
        if let Some(p) = &mut self.periksa {
            p.uji = t;
        }
    }

    fn selesai_uji(&mut self, kode: i32, h: &Value) {
        let perlu_login = self.catat_perlu_login(h);
        if kode == 1 || h.is_null() {
            self.salin_setelah_uji = false;
            let t = match perlu_login {
                Some(l) => format!("Uji coba gagal: login {l} kedaluwarsa. Klik Login ulang pada akun itu."),
                None => "Uji coba gagal. Lihat log aktivitas.".into(),
            };
            self.set_uji((t, MERAH));
            self.log_terbuka = true;
            return;
        }
        self.ringkas_uji = Some(h.clone());
        let akan = h_bil(h, "AkanDisalin");
        let mut w = HIJAU;
        let mut t = if akan == 0 {
            format!("{CENTANG}  Semua file sudah ada di tujuan.")
        } else {
            format!("{CENTANG}  Siap: {} file akan disalin ({}).", angka(akan), ukuran(h_bil(h, "AkanBytes") as f64))
        };
        if h_bil(h, "Terlewat") > 0 {
            t = format!(
                "{} file tidak bisa dibaca dan akan terlewat. Coba pakai akun OneDrive (lewat internet) sebagai sumber.",
                angka(h_bil(h, "Terlewat"))
            );
            w = KUNING;
        } else if h_bil(h, "Masalah") > 0 {
            t += &format!(" Ada {} catatan masalah; lihat log aktivitas.", angka(h_bil(h, "Masalah")));
            w = KUNING;
        }
        self.set_uji((t, w));
        self.sudah_diperiksa = true;
        self.set_pesan("Pemeriksaan selesai.", REDUP);
        // Dipicu dari "Mulai salin": lanjut ke konfirmasi hanya bila uji coba bersih.
        if self.salin_setelah_uji {
            self.salin_setelah_uji = false;
            if w == HIJAU {
                self.klik_salin();
            } else {
                self.set_pesan("Baca hasil uji coba di bawah sebelum menyalin.", KUNING);
            }
        }
    }

    // -------------------------------------------------------------- salin

    fn klik_salin(&mut self) {
        let Some((s, t)) = self.ambil_isian() else { return };
        // Selalu periksa & uji coba dulu, supaya konfirmasi menyebut jumlah file yang sebenarnya.
        let Some(u) = self.ringkas_uji.clone().filter(|_| self.sudah_diperiksa) else {
            self.salin_setelah_uji = true;
            self.klik_periksa();
            return;
        };
        if h_bil(&u, "AkanDisalin") == 0 {
            self.set_pesan("Tidak ada yang perlu disalin: semua file sudah ada di tujuan.", HIJAU);
            return;
        }
        let batas = if self.batas == "off" {
            String::new()
        } else {
            format!("\nKecepatan dibatasi {}.", rute::tampil_batas(&self.batas))
        };
        let isi = format!(
            "Dari:\n    {}\n\nKe:\n    {}\n\nAkan disalin: {} file, sekitar {}.{batas}\n\nFile di sumber tidak diubah atau dihapus. Penyalinan berjalan di latar belakang; aplikasi boleh ditutup.",
            self.nama_jalur(&s),
            self.nama_jalur(&t),
            angka(h_bil(&u, "AkanDisalin")),
            ukuran(h_bil(&u, "AkanBytes") as f64)
        );
        self.tanya("Mulai salin", isi, "Mulai salin", false, Konfirmasi::MulaiSalin);
    }

    /// Apakah penyalinan ini melibatkan akun Google yang memakai client ID bersama?
    fn job_client_bersama(&self, job: &serde_json::Map<String, Value>) -> bool {
        ["Sumber", "Tujuan"].iter().any(|k| {
            self.akun_dari(&akun::remote_dari(&teks(job.get(*k))))
                .is_some_and(|a| a.jenis == "google" && !a.client_sendiri)
        })
    }

    /// Dibaca ulang tiap 1,6 detik dari job.json + log rclone.
    fn perbarui_progres(&mut self) {
        let job = baca_job().filter(|j| !teks(j.get("Log")).is_empty());
        let Some(job) = job else {
            self.progres = None;
            self.job_jalan = false;
            return;
        };
        let jalan = job_berjalan(&job);
        let log = teks(job.get("Log"));
        let s = baca_statistik(&log);

        let mut persen = 0u32;
        let mut detail: Vec<String> = Vec::new();
        if let Some(s) = &s {
            if let Some(c) = RE_PERSEN.captures(&s.data) {
                persen = c[1].parse().unwrap_or(0);
            }
            let bag: Vec<&str> = RE_KOMA.split(&s.data).collect();
            if let Some(c) = bag.first().and_then(|b| RE_DARI_TOTAL.captures(b)) {
                let total_b = RE_UKURAN
                    .captures(&c[2])
                    .map(|u| ke_byte(&u[1], u.get(2).map_or("", |m| m.as_str())))
                    .unwrap_or(0.0);
                if total_b > 0.0 {
                    detail.push(format!("{} dari {}", ubah_ukuran(&c[1]), ubah_ukuran(&c[2])));
                } else if jalan {
                    detail.push(format!("Menghitung daftar file{ELIPSIS}"));
                } else {
                    detail.push("Tidak ada file baru".into());
                }
            }
            if let Some(c) = RE_FILE.captures(&s.file) {
                detail.push(format!(
                    "{} dari {} file",
                    angka(c[1].parse().unwrap_or(0)),
                    angka(c[2].parse().unwrap_or(0))
                ));
            }
            if jalan && bag.len() > 2 && bag[2].ends_with("/s") {
                detail.push(format!("{}/dtk", ubah_ukuran(bag[2].trim_end_matches("/s"))));
            }
            if jalan {
                if let Some(c) = RE_ETA.captures(&s.data) {
                    let e = ubah_durasi(&c[1]);
                    if !e.is_empty() {
                        detail.push(format!("sisa {e}"));
                    }
                }
            }
            if !s.error.is_empty() && !RE_NOL.is_match(&s.error) {
                detail.push(format!("{} error", RE_KURUNG.replace(&s.error, "")));
            }
        }

        let mut catatan: Option<Teks> = None;
        let saran_client = if self.job_client_bersama(&job) {
            if self.ada_client_bawaan {
                " Akun Google di penyalinan ini memakai client ID bersama rclone: hentikan, buka Pengaturan > Client ID Google > \"Login ulang dengan client ID ini\", lalu klik Lanjutkan."
            } else {
                " Akun Google di penyalinan ini memakai client ID bersama rclone: hentikan, pasang client ID sendiri lewat Pengaturan > Client ID Google, login ulang akun itu, lalu klik Lanjutkan."
            }
        } else {
            ""
        };
        let saran_login = "Login salah satu akun kedaluwarsa (invalid_grant). Hentikan, klik Login ulang pada akun itu di daftar kiri, lalu klik Lanjutkan.";
        let kode = teks(job.get("Kode"));
        // Penyalinan milik jadwal backup: rutenya bisa beda dengan panel Dari/Ke, jadi jendela
        // tidak memverifikasi atau menawarkan "Lanjutkan" untuknya (pindah.exe jadwal sudah memverifikasi).
        let nama_jadwal = teks(job.get("JadwalNama"));
        let dari_jadwal = !teks(job.get("Jadwal")).is_empty();
        let mut warna = HIJAU;
        let (mut judul, mut file);
        if jalan && teks(job.get("Menunggu")) == "jaringan" {
            // Pekerja latar sedang menunggu internet kembali (lihat mesin::lanjut).
            warna = KUNING;
            judul = "Menunggu jaringan".to_string();
            file = format!(
                "Dicoba lagi otomatis sekitar pukul {} (percobaan {} dari {}).",
                teks(job.get("UlangPukul")),
                bil(job.get("Percobaan")).unwrap_or(1) + 1,
                teks(job.get("PercobaanMaks"))
            );
            self.label_salin = "Mulai salin".into();
            catatan = Some((
                "Internet terputus. LELEMOVE melanjutkan sendiri begitu tersambung lagi; file yang sudah sampai tidak diulang. Aplikasi boleh ditutup. Klik Hentikan untuk membatalkan.".into(),
                KUNING,
            ));
        } else if jalan {
            judul = "Sedang menyalin".to_string();
            file = match &s {
                Some(s) if !s.sedang.is_empty() => format!("Sekarang: {}", s.sedang.join(", ")),
                _ => String::new(),
            };
            self.label_salin = "Mulai salin".into();
            let mut c = ("Aplikasi boleh ditutup. Penyalinan tetap berjalan dan progresnya muncul lagi saat aplikasi dibuka.".to_string(), REDUP);
            // Tidak bergerak: byte tersalin sama selama 90 detik, atau log memuat penolakan rate limit.
            if let Some(s) = &s {
                let kunci = format!("{log}|{}", s.data.split(',').next().unwrap_or(""));
                if kunci != self.progres_kunci {
                    self.progres_kunci = kunci;
                    self.progres_sejak = Instant::now();
                    let (_, _, _, jam, mnt, _) = waktu_lokal();
                    self.progres_sejak_jam = format!("{jam:02}.{mnt:02}");
                }
                let diam = self.progres_sejak.elapsed().as_secs();
                if s.perlu_login {
                    c = (saran_login.into(), MERAH);
                    warna = KUNING;
                } else if diam >= 30 && s.jaringan {
                    c = (
                        "Koneksi internet sedang bermasalah; rclone mencoba lagi. Bila putus lama, LELEMOVE menunggu lalu melanjutkan otomatis.".into(),
                        KUNING,
                    );
                    warna = KUNING;
                } else if s.dibatasi || diam >= 90 {
                    let mut t = if s.dibatasi {
                        "Layanan cloud sedang membatasi permintaan (rate limit). rclone menunggu lalu mencoba lagi otomatis.".to_string()
                    } else {
                        format!(
                            "Tidak ada kemajuan sejak {}. Biasanya karena layanan cloud membatasi permintaan; rclone menunggu lalu mencoba lagi otomatis.",
                            self.progres_sejak_jam
                        )
                    };
                    t += saran_client;
                    c = (t, KUNING);
                    warna = KUNING;
                }
            }
            catatan = Some(c);
        } else if kode == "0" {
            persen = 100;
            judul = "Penyalinan selesai".into();
            file = format!("Selesai {}.", waktu_tampil(&teks(job.get("Selesai"))));
            self.label_salin = "Salin lagi".into();
        } else {
            warna = KUNING;
            if kode == "dihentikan" {
                judul = "Dihentikan".into();
                file = "Klik \"Lanjutkan\" untuk meneruskan; file yang sudah sampai dilewati.".into();
            } else if !teks(job.get("Selesai")).is_empty() {
                judul = "Berhenti karena ada kesalahan".into();
                file = baris_error(&log, 1)
                    .first()
                    .map(|e| RE_AWAL_ERROR.replace(e, "").to_string())
                    .unwrap_or_else(|| "Lihat log untuk detailnya.".into());
            } else {
                judul = "Terputus".into();
                file = "Komputer mati atau proses ditutup. Klik \"Lanjutkan\" untuk meneruskan.".into();
            }
            if let Some(s) = &s {
                if s.perlu_login {
                    catatan = Some((saran_login.into(), MERAH));
                } else if s.dibatasi {
                    catatan = Some((format!("Log menunjukkan layanan cloud membatasi permintaan (rate limit).{saran_client}"), KUNING));
                }
            }
            self.label_salin = "Lanjutkan".into();
        }
        if dari_jadwal {
            let nama = if nama_jadwal.is_empty() { "backup terjadwal".to_string() } else { format!("\u{201C}{nama_jadwal}\u{201D}") };
            if jalan && teks(job.get("Menunggu")) != "jaringan" {
                judul = format!("Backup terjadwal {nama}");
            } else if !jalan && kode == "0" {
                judul = format!("Backup terjadwal {nama} selesai");
                file += " Hasil verifikasinya ada di Jadwal backup.";
            } else if !jalan {
                judul = format!("Backup terjadwal {nama}: {}", judul.to_lowercase());
                file = format!("{} Dicoba lagi pada jadwal berikutnya, atau klik Jalankan sekarang di Jadwal backup.", file.trim_end());
            }
            if !jalan {
                self.label_salin = "Mulai salin".into();
            }
        }
        self.progres = Some(Progres { judul, persen, warna, detail: detail.join(&format!("   {TITIK}   ")), file, catatan });

        // Penyalinan baru berhenti: folder tujuan kini (mungkin) sudah ada, jadi muat ulang panel Ke.
        if self.job_jalan && !jalan {
            self.tujuan_berubah();
            self.muat_panel(KE, true);
        }
        // Penyalinan baru saja selesai saat aplikasi terbuka: langsung verifikasi.
        if self.job_jalan && !jalan && kode == "0" && self.aksi.is_none() && !dari_jadwal {
            self.job_jalan = false;
            self.set_pesan(&format!("Penyalinan selesai. Memverifikasi{ELIPSIS}"), HIJAU);
            self.klik_verifikasi();
        }
        self.job_jalan = jalan;
    }

    // -------------------------------------------------------------- verifikasi

    fn klik_verifikasi(&mut self) {
        if self.ambil_isian().is_none() {
            return;
        }
        self.verifikasi = (format!("Mencocokkan setiap file{ELIPSIS}"), REDUP);
        self.jalankan_aksi("Memverifikasi hasil salinan", v!["verifikasi"], Setelah::Verifikasi);
    }

    fn selesai_verifikasi(&mut self, kode: i32, h: &Value) {
        let perlu_login = self.catat_perlu_login(h);
        if !h_ada(h, "Cocok") {
            self.verifikasi = match perlu_login {
                Some(l) => (format!("Verifikasi tidak berjalan: login {l} kedaluwarsa. Klik Login ulang pada akun itu."), MERAH),
                None => ("Verifikasi tidak berjalan. Lihat log aktivitas.".into(), MERAH),
            };
            self.log_terbuka = true;
            return;
        }
        let mut cara = if teks(h.get("Metode")) == "ukuran" {
            "dicocokkan per nama & ukuran, karena kedua layanan tidak punya jenis checksum yang sama".to_string()
        } else {
            "dicocokkan dengan checksum".to_string()
        };
        if h_bil(h, "OfficeSharePoint") > 0 {
            cara += &format!(
                "; {} file Office diubah metadatanya oleh SharePoint dan dianggap cocok",
                angka(h_bil(h, "OfficeSharePoint"))
            );
        }
        if kode == 0 {
            self.verifikasi = (
                format!(
                    "{CENTANG}  Verifikasi: semua {} file dari {} sudah ada di tujuan ({cara}).",
                    angka(h_bil(h, "Cocok")),
                    self.nama_pendek(&teks(h.get("Sumber")))
                ),
                HIJAU,
            );
            self.set_pesan("Selesai. Salinan di tujuan lengkap.", HIJAU);
            return;
        }
        let mut bag = vec![format!("{} cocok", angka(h_bil(h, "Cocok")))];
        if h_bil(h, "Hilang") > 0 {
            bag.push(format!("{} belum ada di tujuan", angka(h_bil(h, "Hilang"))));
        }
        if h_bil(h, "Beda") > 0 {
            bag.push(format!("{} isinya berbeda", angka(h_bil(h, "Beda"))));
        }
        if h_bil(h, "Galat") > 0 {
            bag.push(format!("{} gagal dibaca", angka(h_bil(h, "Galat"))));
        }
        self.verifikasi = (format!("{}. Klik \"Lanjutkan\" untuk melengkapi, lalu verifikasi lagi.", bag.join(", ")), KUNING);
        self.label_salin = "Lanjutkan".into();
        self.set_pesan("Verifikasi menemukan selisih.", KUNING);
    }

    // -------------------------------------------------------------- tampilan

    /// Rute (panel Dari/Ke) boleh diubah: tidak sedang menyalin dan tidak ada aksi berjalan.
    fn boleh_rute(&self) -> bool {
        !self.job_jalan && self.aksi.is_none()
    }

    /// Toolbar: logo, empat aksi utama, batas kecepatan.
    fn panel_alat(&mut self, ctx: &egui::Context) {
        if self.logo.is_none() {
            self.logo = gaya::png_dari_ico(IKON)
                .and_then(|png| image::load_from_memory_with_format(png, image::ImageFormat::Png).ok())
                .map(|g| {
                    let g = g.resize(64, 64, image::imageops::FilterType::Lanczos3).to_rgba8();
                    let ci = egui::ColorImage::from_rgba_unmultiplied([g.width() as usize, g.height() as usize], g.as_raw());
                    ctx.load_texture("logo", ci, egui::TextureOptions::LINEAR)
                });
        }
        let sibuk = self.aksi.is_some();
        let boleh = self.rclone.is_some() && !self.job_jalan && !sibuk;
        let (mut periksa, mut salin, mut henti, mut verif) = (false, false, false, false);
        let batas_lama = self.batas.clone();
        egui::TopBottomPanel::top("alat")
            .frame(Frame::new().fill(PUTIH).inner_margin(Margin { left: 12, right: 12, top: 7, bottom: 7 }))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    if let Some(t) = &self.logo {
                        ui.add(egui::Image::new((t.id(), vec2(22.0, 22.0))));
                    }
                    // Tulisan merek hanya bila lega; judul jendela sudah menyebutnya.
                    if ui.available_width() > 690.0 {
                        ui.add_space(5.0);
                        ui.spacing_mut().item_spacing.x = 0.0;
                        let huruf = |t: &str, w: Color32| RichText::new(t).family(tebal()).size(15.0).color(w).extra_letter_spacing(0.6);
                        ui.label(huruf("LELE", Color32::from_rgb(0x0E, 0x5A, 0x52)));
                        ui.label(huruf("MOVE", TINTA));
                        ui.spacing_mut().item_spacing.x = 4.0;
                    }
                    ui.add_space(8.0);
                    let (r, _) = ui.allocate_exact_size(vec2(1.0, 22.0), Sense::hover());
                    ui.painter().rect_filled(r, 0.0, GARIS);
                    ui.add_space(8.0);
                    periksa = gaya::tombol_alat(ui, ik::CARI, "Periksa", false, boleh)
                        .on_hover_text("Hitung isi sumber dan ruang di tujuan, lalu uji coba tanpa menyalin apa pun.")
                        .clicked();
                    salin = gaya::tombol_alat(ui, ik::MAIN, &self.label_salin, true, boleh)
                        .on_hover_text("Memeriksa dan menguji coba dulu, lalu meminta persetujuan Anda sebelum menyalin.")
                        .clicked();
                    henti = gaya::tombol_alat(ui, ik::HENTI, "Hentikan", false, self.job_jalan && !sibuk).clicked();
                    verif = gaya::tombol_alat(ui, ik::SELESAI, "Verifikasi", false, boleh)
                        .on_hover_text("Mencocokkan setiap file sumber dengan salinannya di tujuan. Berjalan otomatis setelah penyalinan selesai.")
                        .clicked();
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        self.pilih_batas(ui, !self.job_jalan && !sibuk);
                        ui.label(gaya::ikon(ik::SPEEDO, 15.0, REDUP)).on_hover_text("Batas kecepatan penyalinan");
                    });
                });
            });
        if self.batas != batas_lama {
            rute::simpan_batas_kecepatan(&self.batas);
            self.tulis_log(&format!("Batas kecepatan: {}.", rute::tampil_batas(&self.batas)));
        }
        if periksa {
            self.klik_periksa();
        }
        if salin {
            self.klik_salin();
        }
        if verif {
            self.klik_verifikasi();
        }
        if henti {
            self.tanya(
                "Hentikan penyalinan",
                "Hentikan penyalinan?\n\nFile yang sudah sampai tetap ada di tujuan. Klik \"Lanjutkan\" kapan saja untuk meneruskan sisanya.".into(),
                "Hentikan",
                true,
                Konfirmasi::Henti,
            );
        }
    }

    /// Bilah status: pesan terakhir, versi rclone, log, folder log, buka tujuan.
    fn panel_kaki(&mut self, ctx: &egui::Context) {
        let mut pilih = 0;
        egui::TopBottomPanel::bottom("kaki")
            .frame(Frame::new().fill(LATAR2).inner_margin(Margin { left: 12, right: 12, top: 5, bottom: 6 }))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 14.0;
                        if ui.add(egui::Link::new(RichText::new("Buka tujuan").size(12.5))).clicked() {
                            pilih = 1;
                        }
                        if ui.add(egui::Link::new(RichText::new("Folder log").size(12.5))).clicked() {
                            pilih = 2;
                        }
                        let t = if self.log_terbuka { "Tutup log" } else { "Log aktivitas" };
                        if ui.add(egui::Link::new(RichText::new(t).size(12.5))).clicked() {
                            pilih = 3;
                        }
                        if self.rclone.is_some() && !self.versi_rclone.is_empty() && ui.available_width() > 260.0 {
                            ui.label(kecil(&format!("rclone {}", self.versi_rclone), ABU));
                        }
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            ui.add(Label::new(RichText::new(&self.pesan.0).size(12.5).color(self.pesan.1)).truncate());
                        });
                    });
                });
            });
        match pilih {
            1 => self.klik_buka_tujuan(),
            2 => {
                let _ = std::fs::create_dir_all(&jalur().log);
                win::buka(&jalur().log.to_string_lossy());
            }
            3 => self.log_terbuka = !self.log_terbuka,
            _ => {}
        }
    }

    /// Laci log aktivitas di atas bilah status (bisa ditarik lebih tinggi).
    fn panel_log(&mut self, ctx: &egui::Context) {
        if !self.log_terbuka {
            return;
        }
        let mut tutup = false;
        egui::TopBottomPanel::bottom("log")
            .resizable(true)
            .default_height(150.0)
            .height_range(96.0..=380.0)
            .frame(Frame::new().fill(LATAR2).inner_margin(Margin { left: 12, right: 12, top: 6, bottom: 8 }))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(label_kelompok("LOG AKTIVITAS", ABU));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        tutup = gaya::tombol_ikon(ui, ik::TUTUP, 22.0, true).on_hover_text("Tutup log").clicked();
                    });
                });
                Frame::new()
                    .fill(PUTIH)
                    .stroke(Stroke::new(1.0_f32, GARIS))
                    .corner_radius(6)
                    .inner_margin(Margin::same(6))
                    .show(ui, |ui| {
                        ScrollArea::vertical().stick_to_bottom(true).auto_shrink([false, false]).show(ui, |ui| {
                            let mut t = self.log.as_str();
                            ui.add(
                                TextEdit::multiline(&mut t)
                                    .font(egui::TextStyle::Monospace)
                                    .desired_width(f32::INFINITY)
                                    .frame(false),
                            );
                        });
                    });
            });
        if tutup {
            self.log_terbuka = false;
        }
    }

    /// Bilah transfer di bawah kedua panel: aksi yang berjalan, progres, hasil uji coba, verifikasi.
    fn panel_transfer(&mut self, ctx: &egui::Context) {
        let mut pilih = 0;
        egui::TopBottomPanel::bottom("transfer")
            .frame(Frame::new().fill(PUTIH).inner_margin(Margin { left: 16, right: 16, top: 10, bottom: 11 }))
            .show(ctx, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 4.0;
                let mut ada = false;
                if let Some(a) = &self.aksi {
                    ada = true;
                    ui.horizontal(|ui| {
                        ui.add(egui::Spinner::new().size(14.0).color(TAUTAN));
                        ui.add_space(2.0);
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.spacing_mut().item_spacing.x = 14.0;
                            if ui.link("Batalkan").clicked() {
                                pilih = 1;
                            }
                            if self.url_login.is_some() && ui.link("Browser tidak terbuka? Buka halaman login").clicked() {
                                pilih = 2;
                            }
                            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                                ui.add(Label::new(RichText::new(format!("{}{ELIPSIS}", a.judul)).color(TINTA)).truncate());
                            });
                        });
                    });
                }
                if let Some(p) = &self.progres {
                    if ada {
                        ui.add_space(6.0);
                    }
                    ada = true;
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&p.judul).family(tebal()).color(TINTA));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new(format!("{}%", p.persen)).family(tebal()).color(TINTA));
                            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                                if !p.detail.is_empty() {
                                    ui.add(Label::new(RichText::new(&p.detail).size(12.5).color(REDUP)).truncate());
                                }
                            });
                        });
                    });
                    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 6.0), Sense::hover());
                    let pn = ui.painter();
                    pn.rect_filled(r, 3.0, JALUR);
                    if p.persen > 0 {
                        let mut isi = r;
                        isi.set_width((r.width() * p.persen.min(100) as f32 / 100.0).max(6.0));
                        pn.rect_filled(isi, 3.0, p.warna);
                    }
                    if !p.file.is_empty() {
                        ui.add(Label::new(kecil(&p.file, REDUP)).truncate());
                    }
                    if let Some((c, w)) = &p.catatan {
                        ui.add(Label::new(kecil(c, *w)).wrap());
                    }
                }
                if let Some(u) = self.periksa.as_ref().map(|p| &p.uji).filter(|u| !u.0.is_empty()) {
                    if ada {
                        ui.add_space(4.0);
                    }
                    ada = true;
                    ui.add(Label::new(RichText::new(&u.0).color(u.1)).wrap());
                }
                if !self.verifikasi.0.is_empty() {
                    if ada {
                        ui.add_space(4.0);
                    }
                    ada = true;
                    ui.add(Label::new(RichText::new(&self.verifikasi.0).color(self.verifikasi.1)).wrap());
                }
                if !ada {
                    ui.add(
                        Label::new(kecil(
                            "Buka folder yang mau disalin di panel Dari dan folder tujuan di panel Ke, lalu klik Mulai salin. \
                             Ukuran, ruang tujuan, dan uji coba diperiksa dulu. File di sumber tidak diubah dan tidak dihapus.",
                            REDUP,
                        ))
                        .wrap(),
                    );
                }
            });
        match pilih {
            1 => self.klik_batal(),
            2 => {
                if let Some(u) = self.url_login.clone() {
                    win::buka(&u);
                }
            }
            _ => {}
        }
    }

    // ---- daftar lokasi (kiri)

    /// Ringkasan akun untuk daftar kiri: (keterangan singkat, warnanya, warna titik status, keterangan lengkap).
    fn keterangan_akun(&self, a: &Akun) -> (String, Color32, Color32, String) {
        let s = self.sambungan.get(&a.nama);
        let jenis = jenis_tampil(&a.jenis);
        let mut detail: Vec<String> = Vec::new();
        let mut singkat = String::new();
        let (mut warna, mut titik) = (REDUP, HIJAU);
        match a.jenis.as_str() {
            "google" => {
                detail.push(if a.scope == "drive" { "izin penuh" } else { "izin terbatas (hanya tujuan)" }.into());
                singkat = if a.scope == "drive" { "izin penuh" } else { "hanya tujuan" }.into();
                if a.client_aplikasi {
                    detail.push("client ID aplikasi".into());
                } else if a.client_sendiri {
                    detail.push("client ID lain (lihat Pengaturan)".into());
                } else {
                    detail.push("client ID bersama (sering dibatasi)".into());
                    singkat = "client ID bersama".into();
                    (warna, titik) = (KUNING, KUNING);
                }
            }
            "sharepoint" => {
                if a.siap {
                    detail.push(format!("pustaka {}, situs {}", a.drive, a.situs));
                    singkat = if a.situs.is_empty() { a.drive.clone() } else { a.situs.clone() };
                } else {
                    detail.push("belum memilih situs & pustaka".into());
                    singkat = "pilih situs & pustaka".into();
                    (warna, titik) = (KUNING, KUNING);
                }
            }
            _ => {
                if !a.drive.is_empty() {
                    detail.push(format!("drive {}", a.drive));
                    singkat = a.drive.clone();
                }
            }
        }
        let mut singkat = if singkat.is_empty() { jenis.to_string() } else { format!("{jenis} {TITIK} {singkat}") };
        let mut tambahan = String::new();
        match s {
            None if a.siap => titik = ABU,
            Some(Sambungan::Kedaluwarsa) => {
                (warna, titik) = (MERAH, MERAH);
                singkat = "login kedaluwarsa".into();
                detail.insert(0, "login kedaluwarsa".into());
            }
            Some(Sambungan::Jaringan(e)) => {
                (warna, titik) = (KUNING, KUNING);
                singkat = "jaringan terputus".into();
                detail.insert(0, "jaringan terputus, diuji ulang otomatis".into());
                tambahan = format!(
                    "\n\nInternet atau DNS sedang putus saat akun ini diuji; akunnya sendiri tidak bermasalah. \
                     LELEMOVE mengujinya lagi tiap {} detik.\n\n{e}",
                    UJI_ULANG_JARINGAN.as_secs()
                );
            }
            Some(Sambungan::Galat(e)) => {
                (warna, titik) = (KUNING, KUNING);
                singkat = "belum bisa diakses".into();
                detail.insert(0, "belum bisa diakses".into());
                tambahan = format!("\n\n{e}");
            }
            _ => {}
        }
        if a.jenis == "google" && !a.client_aplikasi {
            tambahan += if self.ada_client_bawaan {
                "\n\nAkun ini belum memakai client ID aplikasi. Buka Pengaturan > Client ID Google, lalu \"Login ulang dengan client ID ini\"."
            } else {
                "\n\nClient ID bersama rclone sering dibatasi Google, sehingga penyalinan bisa macet. Pasang client ID sendiri lewat Pengaturan > Client ID Google."
            };
        }
        let lengkap = format!("{jenis} {TITIK} {}\n{}{tambahan}", a.label, detail.join(", "));
        (singkat, warna, titik, lengkap)
    }

    /// Panel mana yang sedang membuka lokasi ini: "DARI", "KE", "DARI \u{00B7} KE", atau "".
    fn tanda_pakai(&self, cocok: impl Fn(usize) -> bool) -> String {
        match (cocok(DARI), cocok(KE)) {
            (true, true) => format!("DARI {TITIK} KE"),
            (true, false) => "DARI".into(),
            (false, true) => "KE".into(),
            _ => String::new(),
        }
    }

    /// Pintasan "Komputer ini" yang memuat folder panel ini (yang paling spesifik), bila ada.
    fn pintasan_sisi(&self, sisi: usize) -> Option<usize> {
        if !self.lok(sisi).is_empty() {
            return None;
        }
        let d = self.dalam(sisi).to_lowercase();
        self.pintasan
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                let f = p.folder.trim_end_matches('\\').to_lowercase();
                d == f || d.starts_with(&format!("{f}\\"))
            })
            .max_by_key(|(_, p)| p.folder.len())
            .map(|(i, _)| i)
    }

    fn panel_lokasi(&mut self, ctx: &egui::Context) {
        let lebar = if ctx.screen_rect().width() < 720.0 { 156.0 } else { 180.0 };
        let boleh_rute = self.boleh_rute();
        let boleh_akun = self.rclone.is_some() && !self.job_jalan && self.aksi.is_none();
        let ada = self.rclone.is_some();
        let mut buka: Option<(usize, String, Option<String>)> = None;
        let mut aksi_akun: Option<(String, &'static str)> = None;
        let mut tambah: Option<&'static str> = None;
        let mut pasang = false;
        let mut atur = false;
        let mut bantuan = false;
        let mut jadwal = false;
        egui::SidePanel::left("lokasi")
            .exact_width(lebar)
            .resizable(false)
            .frame(Frame::new().fill(LATAR2).inner_margin(Margin { left: 8, right: 8, top: 10, bottom: 8 }))
            .show(ctx, |ui| {
                // Pengaturan menempel di dasar sidebar; daftar lokasi di atasnya tetap bisa digulir.
                egui::TopBottomPanel::bottom("lokasi-bawah")
                    .frame(Frame::new().fill(LATAR2).inner_margin(Margin { left: 0, right: 0, top: 6, bottom: 0 }))
                    .show_inside(ui, |ui| {
                        let r = baris_lokasi(ui, "pengaturan", ik::JADWAL, "Jadwal backup", "", REDUP, None, "", false, true);
                        jadwal = r.baris.on_hover_text("Salin otomatis setiap hari atau pada hari tertentu, lewat Task Scheduler Windows.").clicked();
                        let r = baris_lokasi(ui, "pengaturan", ik::BANTUAN, "Bantuan & info", "", REDUP, None, "", false, true);
                        bantuan = r.baris.on_hover_text("Tanya jawab, ketentuan, privasi, atribusi, dan tentang LELEMOVE.").clicked();
                        let r = baris_lokasi(ui, "pengaturan", ik::PENGATURAN, "Pengaturan", "", REDUP, None, "", false, true);
                        atur = r.baris.on_hover_text("Client ID Google, lanjut otomatis, dan pengaturan lain.").clicked();
                    });
                ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 1.0;
                    if !self.status_siap {
                        ui.horizontal(|ui| {
                            ui.add(egui::Spinner::new().size(12.0).color(ABU));
                            ui.label(kecil(&format!("Memeriksa rclone{ELIPSIS}"), REDUP));
                        });
                        ui.add_space(8.0);
                    } else if !ada {
                        Frame::new()
                            .fill(PUTIH)
                            .stroke(Stroke::new(1.0_f32, GARIS))
                            .corner_radius(6)
                            .inner_margin(Margin::same(8))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.label(RichText::new("rclone belum terpasang").family(tebal()).size(13.0).color(MERAH));
                                ui.label(kecil("LELEMOVE menyalin memakai rclone.", REDUP));
                                ui.add_space(4.0);
                                pasang = ui.add_enabled(self.aksi.is_none(), tombol("Pasang rclone")).clicked();
                            });
                        ui.add_space(10.0);
                    }

                    // ---- Komputer ini
                    ui.label(label_kelompok("KOMPUTER INI", ABU));
                    ui.add_space(3.0);
                    let ps = [self.pintasan_sisi(DARI), self.pintasan_sisi(KE)];
                    let tanda = self.tanda_pakai(|s| self.lok(s).is_empty() && ps[s].is_none());
                    let sorot = self.lok(self.aktif).is_empty() && ps[self.aktif].is_none();
                    let r = baris_lokasi(ui, "lokal", ik::KOMPUTER, "Komputer ini", "", REDUP, None, &tanda, sorot, boleh_rute);
                    if r.baris.clicked() {
                        buka = Some((self.aktif, String::new(), Some(String::new())));
                    }
                    let _ = r.baris.on_hover_text("Semua drive di komputer ini");
                    for (i, p) in self.pintasan.iter().enumerate() {
                        let tanda = self.tanda_pakai(|s| ps[s] == Some(i));
                        let sorot = ps[self.aktif] == Some(i);
                        let r = baris_lokasi(ui, p.petak, p.glyph, &p.nama, "", REDUP, None, &tanda, sorot, boleh_rute);
                        if r.baris.clicked() {
                            buka = Some((self.aktif, String::new(), Some(p.folder.clone())));
                        }
                        let _ = r.baris.on_hover_text(format!("{}\n{}", p.ket, p.folder));
                    }

                    // ---- Cloud
                    ui.add_space(12.0);
                    ui.label(label_kelompok("CLOUD", ABU));
                    ui.add_space(3.0);
                    for a in &self.akun {
                        let (singkat, warna, titik, lengkap) = self.keterangan_akun(a);
                        let tanda = self.tanda_pakai(|s| self.lok(s) == a.nama);
                        let sorot = self.lok(self.aktif) == a.nama;
                        let kedaluwarsa = self.sambungan.get(&a.nama) == Some(&Sambungan::Kedaluwarsa);
                        let r = baris_lokasi(
                            ui,
                            &a.jenis,
                            ik::AWAN,
                            &a.label,
                            &singkat,
                            warna,
                            Some(titik),
                            &tanda,
                            sorot,
                            boleh_akun,
                        );
                        let menu = r.menu.clone();
                        if r.baris.clicked() {
                            if a.siap {
                                buka = Some((self.aktif, a.nama.clone(), None));
                            } else {
                                aksi_akun = Some((a.nama.clone(), "situs"));
                            }
                        }
                        let id_menu = Id::new(("menu-akun", &a.nama));
                        if menu.as_ref().is_some_and(|m| m.clicked()) {
                            ui.memory_mut(|m| m.toggle_popup(id_menu));
                        }
                        let mut pilih: Option<&'static str> = None;
                        if let Some(m) = &menu {
                            popup_below_widget(ui, id_menu, m, PopupCloseBehavior::CloseOnClick, |ui| {
                                ui.set_width(250.0);
                                menu_akun(ui, a, &mut pilih);
                            });
                        }
                        let baris = r.baris.on_hover_text(lengkap);
                        if boleh_akun {
                            baris.context_menu(|ui| {
                                ui.set_width(250.0);
                                menu_akun(ui, a, &mut pilih);
                                if pilih.is_some() {
                                    ui.close_menu();
                                }
                            });
                        }
                        if kedaluwarsa {
                            ui.horizontal(|ui| {
                                ui.add_space(42.0);
                                if ui.add_enabled(boleh_akun, egui::Link::new(RichText::new("Login ulang").size(12.5))).clicked() {
                                    pilih = Some("ulang");
                                }
                            });
                            ui.add_space(3.0);
                        }
                        match pilih {
                            Some("dari") => buka = Some((DARI, a.nama.clone(), None)),
                            Some("ke") => buka = Some((KE, a.nama.clone(), None)),
                            Some(x) => aksi_akun = Some((a.nama.clone(), x)),
                            None => {}
                        }
                    }
                    if ada && self.status_siap && self.akun.is_empty() {
                        ui.add_space(2.0);
                        ui.add(Label::new(kecil("Belum ada akun cloud. Tambahkan Google Drive, OneDrive, atau SharePoint.", REDUP)).wrap());
                        ui.add_space(4.0);
                    }
                    let r = baris_lokasi(ui, "tambah", ik::TAMBAH, "Tambah akun", "", REDUP, None, "", false, ada && self.aksi.is_none());
                    let id_tambah = Id::new("menu-tambah-akun");
                    if r.baris.clicked() {
                        ui.memory_mut(|m| m.toggle_popup(id_tambah));
                    }
                    popup_below_widget(ui, id_tambah, &r.baris, PopupCloseBehavior::CloseOnClick, |ui| {
                        ui.set_width(220.0);
                        for (jenis, t) in [("google", "Google Drive"), ("onedrive", "OneDrive"), ("sharepoint", "SharePoint")] {
                            if item_menu(ui, Some(ik::AWAN), t, TINTA).clicked() {
                                tambah = Some(jenis);
                            }
                        }
                    });
                });
            });
        if pasang {
            self.tanya(
                "Pasang rclone",
                "LELEMOVE memakai rclone, program open source (lisensi MIT) untuk menyalin file antar-layanan cloud.\n\nrclone akan diunduh (sekitar 25 MB) dan dipasang lewat winget dari repositori resmi. Dengan melanjutkan, Anda menyetujui ketentuan sumber dan paket winget.\n\nPasang sekarang?".into(),
                "Pasang",
                false,
                Konfirmasi::PasangRclone,
            );
        }
        if let Some((sisi, lok, isi)) = buka {
            self.pilih_lokasi(sisi, &lok, isi);
        }
        if let Some((n, x)) = aksi_akun {
            self.akun_aksi(&n, x);
        }
        if let Some(j) = tambah {
            self.klik_tambah(j);
        }
        if atur {
            self.buka_pengaturan(None);
        }
        if bantuan {
            self.buka_info(info::Tab::TanyaJawab);
        }
        if jadwal {
            self.buka_jadwal(None);
        }
    }

    fn buka_info(&mut self, tab: info::Tab) {
        let j = jalur();
        self.dialog = Some(Dialog::Info(info::Info {
            tab,
            versi_rclone: if self.versi_rclone.is_empty() { String::new() } else { format!("rclone {}", self.versi_rclone) },
            dir_app: j.dir_app.to_string_lossy().to_string(),
            dir_data: j.data.to_string_lossy().to_string(),
            dir_log: j.log.to_string_lossy().to_string(),
        }));
    }

    // ---- panel Dari | Ke

    fn panel_tengah(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().frame(Frame::new().fill(PUTIH)).show(ctx, |ui| {
            let r = ui.max_rect();
            let celah = 36.0;
            let w = ((r.width() - celah) / 2.0).floor();
            let kiri = Rect::from_min_size(r.min, vec2(w, r.height()));
            let tengah = Rect::from_min_max(pos2(kiri.right(), r.top()), pos2(r.right() - w, r.bottom()));
            let kanan = Rect::from_min_max(pos2(tengah.right(), r.top()), r.max);
            // Klik di mana pun dalam panel menjadikannya panel aktif.
            if ui.input(|i| i.pointer.primary_pressed()) {
                if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                    if kiri.contains(pos) {
                        self.aktif = DARI;
                    } else if kanan.contains(pos) {
                        self.aktif = KE;
                    }
                }
            }
            self.panel_jelajah(ui, DARI, kiri);
            self.celah_tengah(ui, tengah);
            self.panel_jelajah(ui, KE, kanan);
        });
    }

    /// Lokasi panel untuk kepala panel: (jenis petak, glyph, nama, jenis lokasi, titik status).
    fn tampilan_lokasi(&self, sisi: usize) -> (String, char, String, &'static str, Option<Color32>) {
        let lok = self.lok(sisi);
        let Some(a) = self.akun_dari(lok) else {
            // Di dalam folder OneDrive yang tersinkron: tampilkan sebagai folder OneDrive itu.
            if let Some(p) = self.pintasan_sisi(sisi).map(|i| &self.pintasan[i]).filter(|p| p.petak == "sinkron") {
                return (p.petak.into(), p.glyph, p.nama.clone(), "Komputer ini", None);
            }
            return ("lokal".into(), ik::KOMPUTER, "Komputer ini".into(), "", None);
        };
        let titik = self.keterangan_akun(&a).2;
        (a.jenis.clone(), ik::AWAN, a.label.clone(), jenis_tampil(&a.jenis), Some(titik))
    }

    /// Segmen bilah alamat: (teks, jalur bila diklik).
    fn segmen(&self, sisi: usize) -> Vec<(String, String)> {
        let lok = self.lok(sisi);
        let d = self.dalam(sisi);
        let mut v = Vec::new();
        if lok.is_empty() {
            v.push(("Komputer ini".to_string(), String::new()));
            let mut awalan = String::new();
            let mut acc = PathBuf::new();
            for c in Path::new(&d).components() {
                match c {
                    std::path::Component::Prefix(p) => {
                        awalan = p.as_os_str().to_string_lossy().to_string();
                        acc.push(p.as_os_str());
                    }
                    std::path::Component::RootDir => {
                        acc.push("\\");
                        v.push((awalan.clone(), acc.to_string_lossy().to_string()));
                    }
                    std::path::Component::Normal(n) => {
                        acc.push(n);
                        v.push((n.to_string_lossy().to_string(), acc.to_string_lossy().to_string()));
                    }
                    _ => {}
                }
            }
        } else {
            let nama = self.akun_dari(lok).map(|a| a.label).unwrap_or_else(|| lok.to_string());
            v.push((nama, String::new()));
            let mut acc = String::new();
            for s in d.split('/').filter(|s| !s.is_empty()) {
                if !acc.is_empty() {
                    acc.push('/');
                }
                acc.push_str(s);
                v.push((s.to_string(), acc.clone()));
            }
        }
        v
    }

    fn panel_jelajah(&mut self, ui: &mut Ui, sisi: usize, rect: Rect) {
        let boleh = self.boleh_rute();
        let aktif = self.aktif == sisi;
        let id = Id::new(("panel", sisi));
        let mut ui = ui.new_child(UiBuilder::new().max_rect(rect).layout(Layout::top_down(Align::Min)).id_salt(id));
        ui.set_clip_rect(rect);
        let tinggi_kepala = 88.0;
        let kepala = Rect::from_min_size(rect.min, vec2(rect.width(), tinggi_kepala));
        if aktif {
            ui.painter().line_segment(
                [rect.left_top() + vec2(0.0, 1.0), rect.right_top() + vec2(0.0, 1.0)],
                Stroke::new(2.0_f32, TAUTAN),
            );
        }

        // ---- baris 1: petak lokasi, label DARI/KE, nama lokasi (menu), tombol ikon
        let (jenis, glyph, nama, jenis_lok, titik) = self.tampilan_lokasi(sisi);
        let b1 = Rect::from_min_size(kepala.min + vec2(12.0, 12.0), vec2(kepala.width() - 20.0, 32.0));
        let mut ub = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_size(b1.min, vec2(32.0, 32.0))));
        gaya::petak(&mut ub, &jenis, glyph, 32.0, titik);
        let p = ui.painter().clone();
        let x_teks = b1.left() + 42.0;
        let lebar_teks = (b1.right() - 62.0 - x_teks).max(40.0);
        // Baris kecil: "DARI · Google Drive"
        let g_label = p.layout_no_wrap(
            if sisi == DARI { "DARI".into() } else { "KE".into() },
            FontId::new(10.5, tebal()),
            if aktif { TAUTAN } else { ABU },
        );
        let lebar_label = g_label.size().x;
        p.galley(pos2(x_teks, b1.top() + 1.0), g_label, TINTA);
        if !jenis_lok.is_empty() {
            let g = potong(&ui, &format!("{TITIK}  {jenis_lok}"), FontId::proportional(11.5), ABU, lebar_teks - lebar_label - 6.0);
            p.galley(pos2(x_teks + lebar_label + 6.0, b1.top()), g, ABU);
        }
        let g = potong(&ui, &nama, FontId::new(14.0, tebal()), TINTA, lebar_teks - 18.0);
        let r_nama = Rect::from_min_size(pos2(x_teks - 4.0, b1.top() + 13.0), vec2(g.size().x + 24.0, 20.0));
        let resp_nama = ui.interact(r_nama, id.with("lokasi"), if boleh { Sense::click() } else { Sense::hover() });
        if boleh && resp_nama.hovered() {
            p.rect_filled(r_nama, 4.0, HOVER);
        }
        p.galley(pos2(x_teks, r_nama.center().y - g.size().y / 2.0), g.clone(), TINTA);
        p.text(
            pos2(x_teks + g.size().x + 10.0, r_nama.center().y + 1.0),
            Align2::CENTER_CENTER,
            ik::BAWAH,
            FontId::new(9.0, keluarga_ikon()),
            REDUP,
        );
        let resp_nama = resp_nama.on_hover_text(if boleh {
            "Ganti lokasi panel ini"
        } else {
            "Lokasi tidak bisa diganti selama penyalinan atau pemeriksaan berjalan"
        });
        let id_menu = id.with("menu-lokasi");
        if resp_nama.clicked() {
            ui.memory_mut(|m| m.toggle_popup(id_menu));
        }
        let mut pilih_lok: Option<(String, Option<String>)> = None;
        popup_below_widget(&ui, id_menu, &resp_nama, PopupCloseBehavior::CloseOnClick, |ui| {
            ui.set_width(260.0);
            if item_menu(ui, Some(ik::KOMPUTER), "Komputer ini", TINTA).clicked() {
                pilih_lok = Some((String::new(), Some(String::new())));
            }
            for p in &self.pintasan {
                if item_menu(ui, Some(p.glyph), &p.nama, TINTA).clicked() {
                    pilih_lok = Some((String::new(), Some(p.folder.clone())));
                }
            }
            let siap: Vec<&Akun> = self.akun.iter().filter(|a| a.siap).collect();
            if !siap.is_empty() {
                ui.separator();
            }
            for a in siap {
                if item_menu(ui, Some(ik::AWAN), &nama_lokasi(Some(a)), TINTA).clicked() {
                    pilih_lok = Some((a.nama.clone(), None));
                }
            }
        });
        // tombol ikon di kanan
        let lokal = self.lok(sisi).is_empty();
        let r_seg = Rect::from_center_size(pos2(b1.right() - 13.0, b1.center().y), vec2(26.0, 26.0));
        let r_tel = Rect::from_center_size(pos2(b1.right() - 41.0, b1.center().y), vec2(26.0, 26.0));
        let segarkan = {
            let mut u = ui.new_child(UiBuilder::new().max_rect(r_seg));
            gaya::tombol_ikon(&mut u, ik::SEGARKAN, 26.0, true).on_hover_text("Muat ulang daftar folder").clicked()
        };
        let telusuri = {
            let mut u = ui.new_child(UiBuilder::new().max_rect(r_tel));
            gaya::tombol_ikon(&mut u, ik::BUKA_MAP, 26.0, boleh)
                .on_hover_text(if lokal { "Pilih folder lewat jendela Windows (bisa membuat folder baru)" } else { "Pilih folder di jendela terpisah" })
                .clicked()
        };

        // ---- baris 2: naik + bilah alamat
        let b2 = Rect::from_min_size(kepala.min + vec2(10.0, 52.0), vec2(kepala.width() - 18.0, 28.0));
        let r_naik = Rect::from_min_size(b2.min, vec2(28.0, 28.0));
        let naik = {
            let mut u = ui.new_child(UiBuilder::new().max_rect(r_naik));
            gaya::tombol_ikon(&mut u, ik::NAIK, 28.0, boleh && self.bisa_naik(sisi)).on_hover_text("Naik satu tingkat").clicked()
        };
        let r_alamat = Rect::from_min_max(pos2(b2.left() + 32.0, b2.top()), b2.max);
        let alamat = self.bilah_alamat(&mut ui, sisi, r_alamat, boleh);

        // ---- daftar folder + kaki panel
        let garis_atas = kepala.bottom();
        p.line_segment([pos2(rect.left(), garis_atas), pos2(rect.right(), garis_atas)], Stroke::new(1.0_f32, GARIS_HALUS));
        let tinggi_kaki = self.tinggi_kaki(&ui, sisi, rect.width());
        let daftar = Rect::from_min_max(pos2(rect.left(), garis_atas + 1.0), pos2(rect.right(), rect.bottom() - tinggi_kaki));
        let kaki = Rect::from_min_max(pos2(rect.left(), daftar.bottom()), rect.max);
        let mut ud = ui.new_child(UiBuilder::new().max_rect(daftar.shrink2(vec2(6.0, 4.0))).layout(Layout::top_down(Align::Min)));
        let (masuk, ke_atas, coba_lagi, login) = self.daftar_folder(&mut ud, sisi, boleh);
        p.line_segment([kaki.left_top(), kaki.right_top()], Stroke::new(1.0_f32, GARIS_HALUS));
        let mut uk = ui.new_child(UiBuilder::new().max_rect(kaki).layout(Layout::top_down(Align::Min)));
        self.kaki_panel(&mut uk, sisi);

        // ---- tindakan (setelah semua digambar)
        if let Some((lok, isi)) = pilih_lok {
            self.pilih_lokasi(sisi, &lok, isi);
        } else if let Some(j) = alamat {
            self.set_isi(sisi, j);
        } else if naik || ke_atas {
            self.naik(sisi);
        } else if let Some(n) = masuk {
            self.buka_sub(sisi, &n);
        } else if segarkan || coba_lagi {
            self.muat_panel(sisi, true);
        } else if telusuri {
            self.klik_telusuri(sisi == DARI);
        } else if login {
            let n = self.lok(sisi).to_string();
            self.akun_aksi(&n, "ulang");
        }
    }

    /// Bilah alamat: segmen yang bisa diklik; klik ruang kosongnya untuk mengetik alamat.
    fn bilah_alamat(&mut self, ui: &mut Ui, sisi: usize, r: Rect, boleh: bool) -> Option<String> {
        let id = Id::new(("alamat", sisi));
        let lokal = self.lok(sisi).is_empty();
        if let Some(mut t) = self.panel[sisi].edit.take() {
            let mut u = ui.new_child(UiBuilder::new().max_rect(r));
            let te = u.add(
                TextEdit::singleline(&mut t)
                    .id(id.with("edit"))
                    .desired_width(r.width())
                    .min_size(r.size())
                    .vertical_align(Align::Center)
                    .margin(Margin::symmetric(8, 4))
                    .hint_text(if lokal { "D:\\Data" } else { "Folder/Subfolder" }),
            );
            if self.panel[sisi].fokus_edit {
                te.request_focus();
                self.panel[sisi].fokus_edit = false;
            }
            let enter = te.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if enter {
                let t = t.trim().trim_matches('"');
                return Some(if lokal { t.to_string() } else { t.replace('\\', "/").trim_matches('/').to_string() });
            }
            if !te.lost_focus() && !ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.panel[sisi].edit = Some(t);
            }
            return None;
        }

        let resp = ui.interact(r, id, if boleh { Sense::click() } else { Sense::hover() });
        let p = ui.painter().clone();
        p.rect(r, 6.0, LATAR2, Stroke::new(1.0_f32, if boleh && resp.hovered() { ABU } else { GARIS }), StrokeKind::Inside);
        let seg = self.segmen(sisi);
        let huruf = FontId::proportional(13.0);
        let huruf_akhir = FontId::new(13.0, tebal());
        let galley: Vec<_> = seg
            .iter()
            .enumerate()
            .map(|(i, (t, _))| {
                let akhir = i + 1 == seg.len();
                p.layout_no_wrap(t.clone(), if akhir { huruf_akhir.clone() } else { huruf.clone() }, if akhir { TINTA } else { REDUP })
            })
            .collect();
        // Semua segmen bila muat; kalau tidak, segmen dari belakang yang muat dan sisanya diganti "…".
        let pemisah = 18.0;
        let ruang = r.width() - 16.0;
        let total: f32 = galley.iter().map(|g| g.size().x).sum::<f32>() + pemisah * (galley.len().saturating_sub(1)) as f32;
        let mut mulai = 0;
        if total > ruang {
            let tersedia = ruang - 14.0 - pemisah; // tempat "…" dan pemisahnya
            mulai = galley.len() - 1;
            let mut lebar = galley[mulai].size().x;
            while mulai > 0 && lebar + pemisah + galley[mulai - 1].size().x <= tersedia {
                lebar += pemisah + galley[mulai - 1].size().x;
                mulai -= 1;
            }
        }
        let mut x = r.left() + 8.0;
        let cy = r.center().y;
        if mulai > 0 {
            p.text(pos2(x, cy), Align2::LEFT_CENTER, ELIPSIS, huruf.clone(), REDUP);
            x += 14.0;
        }
        let mut hasil = None;
        for (i, g) in galley.iter().enumerate().skip(mulai) {
            if i > 0 {
                p.text(pos2(x + 9.0, cy + 1.0), Align2::CENTER_CENTER, ik::KANAN, FontId::new(8.0, keluarga_ikon()), ABU);
                x += pemisah;
            }
            let lebar_g = g.size().x.min(r.right() - 8.0 - x).max(0.0);
            let rs = Rect::from_min_size(pos2(x - 3.0, r.top() + 3.0), vec2(lebar_g + 6.0, r.height() - 6.0));
            let akhir = i + 1 == galley.len();
            if !akhir && boleh {
                let rr = ui.interact(rs, id.with(i), Sense::click());
                if rr.hovered() {
                    p.rect_filled(rs, 4.0, JALUR);
                }
                if rr.clicked() {
                    hasil = Some(seg[i].1.clone());
                }
            }
            p.with_clip_rect(r.shrink(2.0)).galley(pos2(x, cy - g.size().y / 2.0), g.clone(), TINTA);
            x += lebar_g;
        }
        if hasil.is_none() && resp.clicked() {
            let d = self.dalam(sisi);
            self.panel[sisi].edit = Some(if lokal { d } else if d.is_empty() { String::new() } else { format!("/{d}") });
            self.panel[sisi].fokus_edit = true;
        }
        if boleh {
            resp.on_hover_text("Klik nama folder untuk kembali ke sana, atau klik ruang kosong untuk mengetik alamat");
        }
        hasil
    }

    /// Daftar subfolder. Hasil: (folder yang dibuka, naik, coba lagi, login ulang).
    fn daftar_folder(&mut self, ui: &mut Ui, sisi: usize, boleh: bool) -> (Option<String>, bool, bool, bool) {
        let mut masuk = None;
        let mut naik = false;
        let mut coba = false;
        let mut login = false;
        let bisa_naik = self.bisa_naik(sisi);
        let kedaluwarsa = self.sambungan.get(self.lok(sisi)) == Some(&Sambungan::Kedaluwarsa);
        let putus = matches!(self.sambungan.get(self.lok(sisi)), Some(Sambungan::Jaringan(_)));
        let lokal = self.lok(sisi).is_empty();
        let mut pilih = self.panel[sisi].pilih;
        ScrollArea::vertical().id_salt(("daftar", sisi)).auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            if bisa_naik {
                let r = baris_folder(ui, ik::NAIK, ABU, "..", false, boleh);
                if r.double_clicked() {
                    naik = true;
                }
            }
            match &self.panel[sisi].isi {
                None => {
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        ui.add(egui::Spinner::new().size(14.0).color(ABU));
                        ui.label(kecil(&format!("Memuat folder{ELIPSIS}"), REDUP));
                    });
                    if putus {
                        // rclone terus mencoba ulang selama internet putus, jadi pemuatan bisa lama.
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.add_space(12.0);
                            ui.add(
                                Label::new(kecil(
                                    "Internet sedang putus, jadi pemuatan bisa lama. Akun diuji ulang otomatis; klik \u{21BB} di kepala panel untuk memuat lagi.",
                                    KUNING,
                                ))
                                .wrap(),
                            );
                        });
                    }
                }
                Some(Isi::Drive(d)) => {
                    for (i, (akar, label)) in d.iter().enumerate() {
                        let r = baris_folder(ui, ik::DISK, REDUP, label, pilih == Some(i), boleh);
                        if r.clicked() {
                            pilih = Some(i);
                        }
                        if r.double_clicked() {
                            masuk = Some(akar.clone());
                        }
                    }
                }
                Some(Isi::Folder(d)) => {
                    for (i, n) in d.iter().enumerate() {
                        let r = baris_folder(ui, ik::MAP, MAP, n, pilih == Some(i), boleh);
                        if r.clicked() {
                            pilih = Some(i);
                        }
                        if r.double_clicked() {
                            masuk = Some(n.clone());
                        }
                    }
                    if d.is_empty() {
                        pesan_kosong(ui, ik::MAP, "Tidak ada subfolder", if sisi == DARI { "Seluruh isi folder ini yang disalin." } else { "" }, REDUP);
                    }
                }
                Some(Isi::BelumAda) => {
                    if sisi == KE {
                        pesan_kosong(ui, ik::MAP, "Folder ini belum ada", "Dibuat otomatis saat penyalinan dimulai.", REDUP);
                    } else {
                        pesan_kosong(ui, ik::AWAS, "Folder tidak ditemukan", "Naik satu tingkat atau pilih folder lain.", MERAH);
                    }
                }
                Some(Isi::Galat(e)) => {
                    let (judul, ket) = ringkas_galat(e);
                    if kedaluwarsa || e.contains("invalid_grant") {
                        pesan_kosong(ui, ik::GALAT, "Login akun ini kedaluwarsa", "Login ulang untuk membuka foldernya.", MERAH);
                    } else {
                        pesan_kosong(ui, ik::GALAT, judul, &ket, MERAH);
                    }
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        if kedaluwarsa && !lokal {
                            login = ui.add_enabled(boleh, tombol("Login ulang")).clicked();
                        }
                        coba = ui.add(tombol("Coba lagi")).clicked();
                    });
                }
            }
        });
        self.panel[sisi].pilih = pilih;
        (masuk, naik, coba, login)
    }

    /// Tinggi kaki panel (diukur dari teks yang akan ditampilkan, dibungkus selebar panel).
    fn tinggi_kaki(&self, ui: &Ui, sisi: usize, lebar: f32) -> f32 {
        let w = lebar - 24.0;
        let mut h = 9.0 + 9.0;
        let mut tambah = |t: &str, huruf: FontId| {
            if !t.is_empty() {
                h += ui.fonts(|f| f.layout(t.to_string(), huruf, TINTA, w)).size().y + 2.0;
            }
        };
        let (judul, ket) = self.ringkasan_kaki(sisi);
        tambah(&judul.0, FontId::new(13.5, tebal()));
        tambah(&ket.0, FontId::proportional(12.5));
        let info = if sisi == DARI { &self.info_sumber } else { &self.info_tujuan };
        tambah(&info.0, FontId::proportional(12.5));
        h.max(44.0)
    }

    /// Baris tebal dan keterangannya di kaki panel: hasil Periksa, atau jumlah isi folder.
    fn ringkasan_kaki(&self, sisi: usize) -> (Teks, Teks) {
        if let Some(p) = &self.periksa {
            return if sisi == DARI {
                (p.hasil_sumber.clone(), p.ket_sumber.clone())
            } else {
                (p.hasil_tujuan.clone(), p.ket_tujuan.clone())
            };
        }
        let jumlah = match &self.panel[sisi].isi {
            Some(Isi::Folder(d)) if !d.is_empty() => format!("{} subfolder {TITIK} klik dua kali untuk membuka", angka(d.len() as i64)),
            Some(Isi::Drive(d)) => format!("{} drive {TITIK} klik dua kali untuk membuka", d.len()),
            _ => String::new(),
        };
        ((jumlah, REDUP), kosong())
    }

    fn kaki_panel(&self, ui: &mut Ui, sisi: usize) {
        Frame::new().inner_margin(Margin { left: 12, right: 12, top: 8, bottom: 8 }).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 2.0;
            let (judul, ket) = self.ringkasan_kaki(sisi);
            if !judul.0.is_empty() {
                let tebal_ = self.periksa.is_some();
                let rt = RichText::new(&judul.0).size(if tebal_ { 13.5 } else { 12.5 }).color(judul.1);
                ui.add(Label::new(if tebal_ { rt.family(tebal()) } else { rt }).wrap());
            }
            if !ket.0.is_empty() {
                ui.add(Label::new(kecil(&ket.0, ket.1)).wrap());
            }
            let info = if sisi == DARI { &self.info_sumber } else { &self.info_tujuan };
            if !info.0.is_empty() {
                ui.add(Label::new(kecil(&info.0, info.1)).wrap());
            }
        });
    }

    /// Celah di antara kedua panel: panah arah salin dan tombol tukar.
    fn celah_tengah(&mut self, ui: &mut Ui, r: Rect) {
        let p = ui.painter().clone();
        p.rect_filled(r, 0.0, LATAR2);
        p.line_segment([r.left_top(), r.left_bottom()], Stroke::new(1.0_f32, GARIS));
        p.line_segment([r.right_top(), r.right_bottom()], Stroke::new(1.0_f32, GARIS));
        let c = pos2(r.center().x, r.top() + 12.0 + 16.0);
        p.circle(c, 12.0, PUTIH, Stroke::new(1.0_f32, GARIS));
        p.text(c, Align2::CENTER_CENTER, ik::PANAH, FontId::new(11.0, keluarga_ikon()), TINTA);
        let rt = Rect::from_center_size(pos2(r.center().x, r.top() + 52.0 + 14.0), vec2(26.0, 26.0));
        let boleh = self.boleh_rute();
        let mut u = ui.new_child(UiBuilder::new().max_rect(rt));
        let resp = gaya::tombol_ikon(&mut u, ik::TUKAR, 26.0, boleh).on_hover_text("Tukar sumber \u{21C4} tujuan");
        if resp.clicked() {
            self.klik_tukar();
        }
    }

    fn pilih_batas(&mut self, ui: &mut Ui, boleh: bool) {
        let mut pilihan: Vec<(String, String)> = PILIHAN_BATAS.iter().map(|(v, t)| (v.to_string(), t.to_string())).collect();
        if !pilihan.iter().any(|(v, _)| *v == self.batas) {
            pilihan.push((self.batas.clone(), rute::tampil_batas(&self.batas)));
        }
        let teks = pilihan.iter().find(|(v, _)| *v == self.batas).map(|p| p.1.clone()).unwrap_or_default();
        let r = ui
            .add_enabled_ui(boleh, |ui| {
                ComboBox::from_id_salt("batas_kecepatan")
                    .width(124.0)
                    .wrap_mode(egui::TextWrapMode::Truncate)
                    .selected_text(teks)
                    .show_ui(ui, |ui| {
                        ui.set_min_width(150.0);
                        for (v, t) in &pilihan {
                            ui.selectable_value(&mut self.batas, v.clone(), t);
                        }
                    })
            })
            .response;
        if boleh {
            r.on_hover_text("Membatasi kecepatan penyalinan supaya internet tetap lancar untuk keperluan lain. Berlaku saat penyalinan dimulai atau dilanjutkan.");
        } else if self.job_jalan {
            let t = "Hentikan penyalinan dulu untuk mengubah batas kecepatan, lalu klik Lanjutkan.";
            r.on_hover_text(t).on_disabled_hover_text(t);
        }
    }

    fn tampil_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.dialog.take() else { return };
        let hasil = {
            let mut p = Pemuat { tx: &self.tx, ctx: &self.ctx, gen: &mut self.gen };
            dialog::tampil(ctx, &mut d, &mut p)
        };
        match hasil {
            None => self.dialog = Some(d),
            Some(h) => self.tangani_dialog(h),
        }
    }

    fn urus_tutup(&mut self, ctx: &egui::Context) {
        if !ctx.input(|i| i.viewport().close_requested()) || self.boleh_tutup {
            return;
        }
        if let Some(a) = &self.aksi {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            let judul = a.judul.clone();
            if !matches!(&self.dialog, Some(Dialog::Tanya(t)) if matches!(t.tujuan, Konfirmasi::TutupApp)) {
                self.tanya(
                    "Tutup LELEMOVE",
                    format!("{judul} masih berjalan.\n\nHentikan dan tutup aplikasi?"),
                    "Hentikan & tutup",
                    true,
                    Konfirmasi::TutupApp,
                );
            }
        }
    }

    // -------------------------------------------------------------- pratinjau

    fn buka_dialog_pratinjau(&mut self, d: &str) {
        match d {
            "akun-google" | "akun-onedrive" | "akun-sharepoint" => {
                let j = d.trim_start_matches("akun-").to_string();
                self.dialog = Some(Dialog::AkunBaru(AkunBaru {
                    client_bawaan: akun::baca_client_bawaan().is_some(),
                    jenis: j,
                    label: String::new(),
                    juga_sumber: false,
                    fokus: true,
                }));
            }
            "client" => self.dialog = Some(Dialog::ClientId(ClientId::baru())),
            // Isinya diperbarui lagi begitu daftar akun selesai dimuat (terima_status).
            "pengaturan" => self.buka_pengaturan(None),
            "jadwal" => self.buka_jadwal(None),
            "jadwal-baru" => self.tangani_dialog(Hasil::JadwalBaru),
            _ if d.starts_with("info") => self.buka_info(info::Tab::dari_nama(d.trim_start_matches("info").trim_start_matches(':'))),
            "tanya" => self.tanya("Lepas akun", "Lepas Google Drive dari LELEMOVE?\n\nFile di akun tersebut TIDAK terhapus; hanya sambungan di komputer ini yang dihapus.".into(), "Lepas akun", true, Konfirmasi::Lepas("pd-x".into())),
            _ if d.starts_with("jelajah:") => {
                let mut p = Pemuat { tx: &self.tx, ctx: &self.ctx, gen: &mut self.gen };
                let j = Jelajah::baru(&d[8..], "", "Pilih folder SUMBER di contoh akun".into(), true, &mut p);
                self.dialog = Some(Dialog::Jelajah(j));
            }
            _ if d.starts_with("sharepoint:") => {
                let bag: Vec<&str> = d.splitn(3, ':').collect();
                let mut s = SharePoint::baru(bag.get(1).copied().unwrap_or(""));
                s.cari = bag.get(2).copied().unwrap_or("").to_string();
                let mut p = Pemuat { tx: &self.tx, ctx: &self.ctx, gen: &mut self.gen };
                s.cari(&mut p);
                self.dialog = Some(Dialog::SharePoint(s));
            }
            _ => {}
        }
    }

    fn urus_pratinjau(&mut self, ctx: &egui::Context) {
        // Tunggu daftar folder kedua panel termuat (paling lama 15 detik lebih dari --tunda).
        let siap = self.status_siap && self.aksi.is_none() && !self.panel_memuat();
        let Some(p) = &mut self.pratinjau else { return };
        let lewat = p.mulai.elapsed();
        if !p.diminta && ((siap && lewat >= p.tunda) || lewat >= p.tunda + Duration::from_secs(15)) {
            ctx.send_viewport_cmd(ViewportCommand::Screenshot(Default::default()));
            p.diminta = true;
        }
        let gambar = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(img) = gambar {
            let mut buf = Vec::with_capacity(img.pixels.len() * 4);
            for c in &img.pixels {
                buf.extend_from_slice(&c.to_array());
            }
            if let Some(g) = image::RgbaImage::from_raw(img.size[0] as u32, img.size[1] as u32, buf) {
                let _ = g.save(&p.berkas);
            }
            self.boleh_tutup = true;
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
    }
}

/// Pintasan folder di kelompok "Komputer ini" pada daftar kiri.
struct Pintasan {
    nama: String,
    folder: String,
    glyph: char,
    /// Jenis petak (warna): "lokal" atau "sinkron" (folder OneDrive yang tersinkron).
    petak: &'static str,
    ket: String,
}

/// Desktop, Dokumen, dan folder OneDrive yang tersinkron di komputer ini.
fn pintasan_lokal() -> Vec<Pintasan> {
    let mut v = Vec::new();
    for (f, nama, glyph) in [(win::Folder::Desktop, "Desktop", ik::DESKTOP), (win::Folder::Dokumen, "Dokumen", ik::DOKUMEN)] {
        if let Some(p) = win::folder(f).filter(|p| p.is_dir()) {
            v.push(Pintasan {
                nama: nama.into(),
                folder: p.to_string_lossy().to_string(),
                glyph,
                petak: "lokal",
                ket: "Folder di komputer ini".into(),
            });
        }
    }
    for o in daftar_onedrive() {
        let nama = Path::new(&o.folder).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "OneDrive".into());
        let ket = match o.jenis.as_str() {
            "Pribadi" => "Folder OneDrive pribadi yang tersinkron di komputer ini",
            "Kantor/Sekolah" => "Folder OneDrive kantor/sekolah yang tersinkron di komputer ini",
            _ => "Folder OneDrive yang tersinkron di komputer ini",
        };
        v.push(Pintasan { nama, folder: o.folder, glyph: ik::MAP_SINKRON, petak: "sinkron", ket: ket.into() });
    }
    v
}

/// Galat rclone saat membuka folder, diringkas untuk panel: (judul, keterangan). Teks lengkap masuk log.
fn ringkas_galat(e: &str) -> (&'static str, String) {
    let k = e.to_lowercase();
    let jaringan = ["no such host", "dial tcp", "lookup ", "i/o timeout", "connectex", "wsarecv", "network is unreachable", "connection reset"];
    if jaringan.iter().any(|j| k.contains(j)) {
        return ("Internet sedang putus", "Folder belum bisa dimuat. Klik Coba lagi setelah internet tersambung.".into());
    }
    let baris = e.lines().map(str::trim).filter(|l| !l.is_empty()).last().unwrap_or("");
    let mut t = RE_AWAL_LOG.replace(baris, "").to_string();
    if t.chars().count() > 160 {
        t = t.chars().take(157).collect::<String>() + ELIPSIS;
    }
    ("Folder tidak bisa dibuka", format!("{t} (detail di log aktivitas)"))
}

fn jenis_tampil(j: &str) -> &'static str {
    match j {
        "google" => "Google Drive",
        "sharepoint" => "SharePoint",
        _ => "OneDrive",
    }
}

/// Isi folder di komputer ini (hanya subfolder, tanpa folder tersembunyi/sistem). Jalur kosong = daftar drive.
fn isi_lokal(p: &str) -> Isi {
    use std::os::windows::fs::MetadataExt;
    if p.trim().is_empty() {
        return Isi::Drive(daftar_drive());
    }
    if !adalah_lokal(p) {
        return Isi::Galat("Tulis alamat lengkap, misalnya D:\\Data.".into());
    }
    let path = Path::new(p);
    if !path.is_dir() {
        return if path.exists() { Isi::Galat("Ini berkas, bukan folder.".into()) } else { Isi::BelumAda };
    }
    match std::fs::read_dir(path) {
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => Isi::Galat("Akses ke folder ini ditolak Windows.".into()),
        Err(e) => Isi::Galat(e.to_string()),
        Ok(it) => {
            // 0x2 = tersembunyi, 0x4 = sistem ($RECYCLE.BIN, System Volume Information, dsb.)
            let mut n: Vec<String> = it
                .flatten()
                .filter(|e| e.metadata().is_ok_and(|m| m.is_dir() && m.file_attributes() & 0x6 == 0))
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect();
            n.sort_by_key(|a| a.to_lowercase());
            Isi::Folder(n)
        }
    }
}

/// Drive yang ada: (akar "D:\", nama tampilan "Data (D:)").
fn daftar_drive() -> Vec<(String, String)> {
    use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW};
    let mask = unsafe { GetLogicalDrives() };
    let mut v = Vec::new();
    for i in 0..26u8 {
        if mask & (1 << i) == 0 {
            continue;
        }
        let huruf = (b'A' + i) as char;
        let akar = format!("{huruf}:\\");
        let w = win::lebar(&akar);
        let jenis = unsafe { GetDriveTypeW(w.as_ptr()) };
        let mut nama = [0u16; 261];
        // Drive jaringan tidak ditanya labelnya: bisa lama bila servernya tidak terjangkau.
        let ok = jenis != 4
            && unsafe {
                GetVolumeInformationW(
                    w.as_ptr(),
                    nama.as_mut_ptr(),
                    nama.len() as u32,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    0,
                )
            } != 0;
        let label = if ok { String::from_utf16_lossy(&nama[..nama.iter().position(|&c| c == 0).unwrap_or(0)]) } else { String::new() };
        let label = if label.trim().is_empty() {
            match jenis {
                2 => "Drive lepas",
                4 => "Drive jaringan",
                5 => "CD/DVD",
                _ => "Disk lokal",
            }
            .to_string()
        } else {
            label
        };
        v.push((akar, format!("{label} ({huruf}:)")));
    }
    v
}

/// Baris di daftar lokasi kiri; `menu` = tombol ⋯ (hanya akun cloud).
struct BarisLokasi {
    baris: egui::Response,
    menu: Option<egui::Response>,
}

#[allow(clippy::too_many_arguments)]
fn baris_lokasi(
    ui: &mut Ui,
    jenis: &str,
    glyph: char,
    judul: &str,
    detail: &str,
    warna_detail: Color32,
    titik: Option<Color32>,
    tanda: &str,
    sorot: bool,
    boleh: bool,
) -> BarisLokasi {
    let dua = !detail.is_empty();
    let tinggi = if dua { 42.0 } else { 30.0 };
    let w = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(vec2(w, tinggi), if boleh { Sense::click() } else { Sense::hover() });
    let r_menu = Rect::from_center_size(pos2(r.right() - 12.0, r.center().y), vec2(22.0, 26.0));
    let menu = (titik.is_some() && boleh).then(|| ui.interact(r_menu, resp.id.with("menu"), Sense::click()));
    if ui.is_rect_visible(r) {
        let p = ui.painter();
        let disorot = boleh && (resp.hovered() || menu.as_ref().is_some_and(|m| m.hovered()));
        let latar = if sorot {
            PILIH
        } else if disorot {
            JALUR
        } else {
            LATAR2
        };
        p.rect_filled(r, 6.0, latar);
        let sisi_petak = if dua { 28.0 } else { 22.0 };
        let rp = Rect::from_center_size(pos2(r.left() + 5.0 + sisi_petak / 2.0, r.center().y), vec2(sisi_petak, sisi_petak));
        if jenis == "tambah" {
            p.text(rp.center(), Align2::CENTER_CENTER, glyph, FontId::new(12.0, keluarga_ikon()), if boleh { TAUTAN } else { ABU });
        } else if jenis == "pengaturan" {
            p.text(rp.center(), Align2::CENTER_CENTER, glyph, FontId::new(14.0, keluarga_ikon()), REDUP);
        } else {
            gaya::petak_lukis(p, rp, jenis, glyph, titik, latar);
        }
        let x = rp.right() + 8.0;
        let kanan = if menu.is_some() { r_menu.left() - 2.0 } else { r.right() - 6.0 };
        let g_tanda = (!tanda.is_empty()).then(|| p.layout_no_wrap(tanda.to_string(), FontId::new(9.5, tebal()), TAUTAN));
        let lebar_judul = kanan - x - g_tanda.as_ref().map_or(0.0, |g| g.size().x + 6.0);
        let warna_judul = if jenis == "tambah" { if boleh { TAUTAN } else { ABU } } else { TINTA };
        let huruf = if dua { FontId::new(13.0, tebal()) } else { FontId::proportional(13.0) };
        let gj = potong(ui, judul, huruf, warna_judul, lebar_judul);
        let y = if dua { r.top() + 5.0 } else { r.center().y - gj.size().y / 2.0 };
        let tinggi_judul = gj.size().y;
        p.galley(pos2(x, y), gj, warna_judul);
        if let Some(g) = g_tanda {
            p.galley(pos2(kanan - g.size().x, y + (tinggi_judul - g.size().y) / 2.0 + 0.5), g, TAUTAN);
        }
        if dua {
            let gd = potong(ui, detail, FontId::proportional(11.5), warna_detail, kanan - x);
            p.galley(pos2(x, r.top() + 23.0), gd, warna_detail);
        }
        if let Some(m) = &menu {
            if m.hovered() {
                p.rect_filled(r_menu, 4.0, GARIS);
            }
            let warna = if m.hovered() || disorot || sorot { TINTA } else { ABU };
            p.text(r_menu.center(), Align2::CENTER_CENTER, ik::LAIN, FontId::new(12.0, keluarga_ikon()), warna);
        }
    }
    let menu = menu.map(|m| m.on_hover_text("Atur akun"));
    BarisLokasi { baris: resp, menu }
}

/// Isi menu Atur (⋯ atau klik kanan) untuk satu akun.
fn menu_akun(ui: &mut Ui, a: &Akun, pilih: &mut Option<&'static str>) {
    let mut item = |ui: &mut Ui, g: Option<char>, t: &str, x: &'static str, w: Color32| {
        if item_menu(ui, g, t, w).clicked() {
            *pilih = Some(x);
        }
    };
    if a.siap {
        item(ui, Some(ik::PANAH), "Buka di panel Dari", "dari", TINTA);
        item(ui, Some(ik::PANAH), "Buka di panel Ke", "ke", TINTA);
        ui.separator();
    }
    if a.jenis == "sharepoint" {
        item(ui, None, &format!("Pilih situs & pustaka{ELIPSIS}"), "situs", TINTA);
    }
    item(ui, None, "Login ulang", "ulang", TINTA);
    if a.jenis == "google" && a.scope != "drive" {
        item(ui, None, "Beri izin penuh (agar bisa jadi sumber)", "penuh", TINTA);
    }
    // Client ID Google diatur di Pengaturan (bilah alat), bukan per akun.
    ui.separator();
    item(ui, None, "Lepas akun ini", "lepas", MERAH);
}

/// Satu baris menu: glyph opsional + teks, latar muncul saat disorot.
fn item_menu(ui: &mut Ui, glyph: Option<char>, teks: &str, warna: Color32) -> egui::Response {
    let w = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(vec2(w, 28.0), Sense::click());
    let p = ui.painter();
    if resp.hovered() {
        p.rect_filled(r, 4.0, HOVER);
    }
    let mut x = r.left() + 8.0;
    if let Some(g) = glyph {
        p.text(pos2(x + 7.0, r.center().y), Align2::CENTER_CENTER, g, FontId::new(12.5, keluarga_ikon()), REDUP);
        x += 22.0;
    }
    let g = potong(ui, teks, FontId::proportional(13.5), warna, r.right() - x - 6.0);
    p.galley(pos2(x, r.center().y - g.size().y / 2.0), g, warna);
    resp
}

/// Satu baris daftar folder/drive.
fn baris_folder(ui: &mut Ui, glyph: char, warna_glyph: Color32, teks: &str, terpilih: bool, boleh: bool) -> egui::Response {
    let w = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(vec2(w, TINGGI_BARIS), if boleh { Sense::click() } else { Sense::hover() });
    if ui.is_rect_visible(r) {
        let p = ui.painter();
        if terpilih {
            p.rect_filled(r, 4.0, PILIH);
        } else if boleh && resp.hovered() {
            p.rect_filled(r, 4.0, HOVER);
        }
        if glyph == ik::MAP {
            gaya::lukis_map(p, pos2(r.left() + 16.0, r.center().y), 16.0);
        } else {
            p.text(pos2(r.left() + 16.0, r.center().y), Align2::CENTER_CENTER, glyph, FontId::new(15.0, keluarga_ikon()), warna_glyph);
        }
        let warna = if boleh { TINTA } else { REDUP };
        let g = potong(ui, teks, FontId::proportional(13.5), warna, w - 44.0);
        p.galley(pos2(r.left() + 32.0, r.center().y - g.size().y / 2.0), g, warna);
    }
    resp
}

/// Pesan di tempat daftar folder (kosong, belum ada, galat).
fn pesan_kosong(ui: &mut Ui, glyph: char, judul: &str, ket: &str, warna: Color32) {
    let netral = warna == REDUP;
    ui.add_space(12.0);
    ui.horizontal_top(|ui| {
        ui.add_space(10.0);
        ui.label(gaya::ikon(glyph, 15.0, if netral { ABU } else { warna }));
        ui.add_space(2.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            ui.label(RichText::new(judul).family(tebal()).size(13.0).color(if netral { TINTA } else { warna }));
            if !ket.is_empty() {
                ui.add(Label::new(kecil(ket, REDUP)).wrap());
            }
        });
    });
    ui.add_space(10.0);
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.terima_kabar();
        if self.detak.elapsed() >= Duration::from_millis(1600) {
            self.detak = Instant::now();
            self.perbarui_progres();
            self.uji_ulang_jaringan();
        }
        self.muat_panel(DARI, false);
        self.muat_panel(KE, false);
        self.urus_tutup(ctx);
        // Urutan panel egui: yang dipanggil lebih dulu mengambil tepi lebih luar.
        self.panel_alat(ctx);
        self.panel_kaki(ctx);
        self.panel_lokasi(ctx);
        self.panel_log(ctx);
        self.panel_transfer(ctx);
        self.panel_tengah(ctx);
        self.tampil_dialog(ctx);
        self.urus_pratinjau(ctx);
        let jeda = if self.aksi.is_some() || self.job_jalan || self.pratinjau.is_some() || self.panel_memuat() { 250 } else { 1600 };
        ctx.request_repaint_after(Duration::from_millis(jeda));
    }
}

fn ambil_arg(args: &[String], nama: &str) -> Option<String> {
    args.iter().position(|a| a == nama).and_then(|i| args.get(i + 1)).cloned()
}

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    // Dipanggil tugas Windows (Task Scheduler): teruskan ke pindah.exe secara tersembunyi lalu
    // keluar. LELEMOVE.exe program berjendela, jadi tidak ada konsol yang sempat berkedip.
    if let Some(id) = ambil_arg(&args, "--jadwal") {
        if let Some(d) = ambil_arg(&args, "--data") {
            // Hanya untuk pengujian (tugas dibuat saat PINDAH_DRIVE_DATA terpasang).
            std::env::set_var("PINDAH_DRIVE_DATA", d);
        }
        let exe = jalur().dir_app.join("pindah.exe");
        if let Err(e) = win::mulai_terlepas(&exe, &v!["jadwal", "-Jalankan", "-Id", id]) {
            let _ = pindah_drive::notifikasi::kirim(
                "Backup terjadwal gagal dimulai",
                &[format!("pindah.exe tidak bisa dijalankan: {e}")],
            );
        }
        return Ok(());
    }
    let pratinjau = ambil_arg(&args, "--pratinjau").map(|b| Pratinjau {
        berkas: PathBuf::from(b),
        mulai: Instant::now(),
        tunda: Duration::from_millis(ambil_arg(&args, "--tunda").and_then(|t| t.parse().ok()).unwrap_or(2500)),
        diminta: false,
    });
    // Hanya satu jendela pada satu waktu (installer juga memakai mutex ini).
    let _kunci = if pratinjau.is_none() {
        match win::kunci_tunggal(MUTEX_JENDELA) {
            Some(k) => Some(k),
            None => {
                win::kotak_pesan("LELEMOVE", "LELEMOVE sudah terbuka.", win::MB_OK | win::MB_ICONINFORMATION);
                return Ok(());
            }
        }
    } else {
        None
    };
    win::set_app_id(APP_ID);

    // Layar laptop kecil: jangan lebih tinggi dari area kerja.
    let tinggi = ambil_arg(&args, "--tinggi")
        .and_then(|t| t.parse::<f32>().ok())
        .unwrap_or_else(|| win::tinggi_area_kerja().map_or(820.0, |h| (h - 56.0).clamp(560.0, 820.0)));
    let lebar = ambil_arg(&args, "--lebar").and_then(|t| t.parse::<f32>().ok()).unwrap_or(760.0);
    let mut vp = egui::ViewportBuilder::default()
        .with_title("LELEMOVE")
        .with_inner_size([lebar, tinggi])
        .with_min_inner_size([640.0, 520.0]);
    if let Some(i) = gaya::ikon_dari_ico(IKON) {
        vp = vp.with_icon(Arc::new(i));
    }
    let opsi = eframe::NativeOptions { viewport: vp, centered: true, ..Default::default() };
    let dialog_awal = ambil_arg(&args, "--dialog");
    let log_terbuka = args.iter().any(|a| a == "--log");
    eframe::run_native(
        "LELEMOVE",
        opsi,
        Box::new(move |cc| {
            gaya::pasang(&cc.egui_ctx);
            Ok(Box::new(App::baru(cc.egui_ctx.clone(), pratinjau, dialog_awal, log_terbuka)))
        }),
    )
}
