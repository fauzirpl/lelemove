//! LELEMOVE: jendela aplikasi (egui). Pengganti gui.ps1 + tampilan.xaml.
//!
//! Pekerjaan berat (pasang, login, periksa, uji, salin, verifikasi) dijalankan oleh pindah.exe
//! sebagai proses terpisah, sehingga jendela tidak pernah membeku. Penyalinan berjalan di latar
//! belakang dan tetap jalan walau aplikasi ditutup; saat dibuka lagi, progresnya muncul kembali.
//!
//! Pratinjau (dokumentasi/pengujian):
//!   "LELEMOVE.exe" --pratinjau layar.png [--dialog akun-google|client|pengaturan|jadwal|jadwal-baru|filter|riwayat|
//!                      pindahkan|pindahkan-tanya|info:<tab>|jelajah:<akun>|sharepoint:<akun>:<kata>]
//!                      [--log] [--lebar 760] [--tinggi 820] [--tunda 2500]
//!
//! Tugas Windows jadwal backup memanggil `LELEMOVE.exe --jadwal <id>` (tanpa jendela).
#![windows_subsystem = "windows"]

mod aksi;
mod dialog;
mod email;
mod filter;
mod info;
mod jadwal;
mod layanan;
mod riwayat;

// Pecahan berkas ini: bagian-bagian `impl App`, fungsi bantu, dan fungsi gambar.
mod app_akun;
mod app_aksi;
mod app_dialog;
mod app_jelajah;
mod app_kabar;
mod app_lokasi;
mod app_panel;
mod app_periksa;
mod app_pratinjau;
mod app_riwayat;
mod app_rute;
mod app_salin;
mod app_tampilan;
mod bantu;
mod gambar;

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
use pindah_drive::mesin::dasar::{angka, bil, exe_mesin, format_ukuran as ukuran, jalur, kapital, ke_byte, teks, waktu_lokal, waktu_tampil};
use pindah_drive::mesin::filter::{self as mf, Filter};
use pindah_drive::mesin::jadwal as mj;
use pindah_drive::mesin::penyedia as mp;
use pindah_drive::mesin::riwayat as mr;
use pindah_drive::mesin::job::{baca_job, baca_statistik, baris_error, job_berjalan};
use pindah_drive::mesin::rclone;
use pindah_drive::mesin::rute::{self, adalah_lokal, daftar_onedrive, rapikan_sumber, tujuan_bawaan};
use pindah_drive::mesin::{Mesin, Opsi};
use pindah_drive::{v, win, APP_ID, MUTEX_JENDELA};

use aksi::{Aksi, Isi, Kabar, Setelah};
use dialog::{AkunBaru, AkunGoogle, ClientId, Dialog, Hasil, Jelajah, Ketik, Konfirmasi, Pemuat, Pengaturan, SharePoint, Tanya};
use bantu::*;
use gambar::*;

const IKON: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../aset/ikon.ico"));
/// Nama berkas program mesin, untuk pesan galat.
const MESIN: &str = if cfg!(windows) { "pindah.exe" } else { "pindah" };
const TITIK: &str = "\u{00B7}";
const ELIPSIS: &str = "\u{2026}";
const CENTANG: &str = "\u{2713}";
/// Hati di baris "Dukung LELEMOVE": merah hangat, beda dari merah galat.
const MERAH_HATI: Color32 = Color32::from_rgb(0xE0, 0x4F, 0x6A);
/// Jeda sebelum akun yang gagal karena internet/DNS diuji lagi.
const UJI_ULANG_JARINGAN: Duration = Duration::from_secs(30);
/// Panel kiri (sumber) dan kanan (tujuan).
const DARI: usize = 0;
const KE: usize = 1;
/// Tinggi baris daftar folder.
const TINGGI_BARIS: f32 = 30.0;
/// Pilihan batas kecepatan di toolbar: nilai --bwlimit rclone dan labelnya.
const PILIHAN_BATAS: [(&str, &str); 4] =
    [("off", "Tanpa batas"), ("1M", "1 MB/dtk"), ("5M", "5 MB/dtk"), ("10M", "10 MB/dtk")];

type Teks = (String, Color32);

fn kosong() -> Teks {
    (String::new(), REDUP)
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
    /// Migrasi email yang dialognya ditutup saat masih berjalan (dibuka lagi dari sidebar).
    email_latar: Option<email::Email>,
    /// Keadaan pembaruan aplikasi (ditampilkan di Bantuan & info > Tentang).
    pembaruan: info::Pembaruan,

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
    /// Filter file rute ini (pengaturan.json; dibaca juga oleh pindah.exe).
    filter: Filter,
    verifikasi: Teks,
    /// Verifikasi terakhir: (file cocok, file lain, sumber, tujuan). Tautan "Pindahkan" ditawarkan
    /// selama panel Dari/Ke masih menunjuk rute yang sama.
    bisa_pindahkan: Option<(i64, i64, String, String)>,
    /// Isian kredensial terakhir, dibuka lagi (dengan pesan galat) bila login gagal.
    kredensial_terakhir: Option<layanan::Kredensial>,

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

/// Pintasan folder di kelompok "Komputer ini" pada daftar kiri.
struct Pintasan {
    nama: String,
    folder: String,
    glyph: char,
    /// Jenis petak (warna): "lokal" atau "sinkron" (folder OneDrive yang tersinkron).
    petak: &'static str,
    ket: String,
}

impl App {
    fn baru(ctx: egui::Context, pratinjau: Option<Pratinjau>, dialog_awal: Option<String>, log_terbuka: bool) -> Self {
        let (tx, rx) = channel();
        let mut app = App {
            ctx,
            tx,
            rx,
            exe_mesin: exe_mesin(),
            email_latar: None,
            pembaruan: info::Pembaruan::Belum,
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
            filter: mf::baca_filter(),
            verifikasi: kosong(),
            bisa_pindahkan: None,
            kredensial_terakhir: None,
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
            app.set_pesan(&format!("{MESIN} tidak ditemukan di {}", jalur().dir_app.display()), MERAH);
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
        // Daftar lokasi (sidebar gelap) setinggi jendela; toolbar dan bilah status di sebelah kanannya.
        self.panel_lokasi(ctx);
        self.panel_alat(ctx);
        self.panel_kaki(ctx);
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
        let exe = exe_mesin();
        if let Err(e) = win::mulai_terlepas(&exe, &v!["jadwal", "-Jalankan", "-Id", id]) {
            let _ = pindah_drive::notifikasi::kirim(
                "Backup terjadwal gagal dimulai",
                &[format!("{MESIN} tidak bisa dijalankan: {e}")],
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
                // Bawa jendela yang sudah ada ke depan; kotak pesan hanya bila itu tidak berhasil.
                if !win::tampilkan_jendela("LELEMOVE") {
                    win::kotak_pesan("LELEMOVE", "LELEMOVE sudah terbuka.", win::MB_OK | win::MB_ICONINFORMATION);
                }
                return Ok(());
            }
        }
    } else {
        None
    };
    if pratinjau.is_none() {
        // Installer pembaruan yang sudah terpasang tidak perlu disimpan. Ditunda sebentar: installer
        // yang baru saja membuka jendela ini mungkin masih berjalan.
        std::thread::spawn(|| {
            std::thread::sleep(Duration::from_secs(20));
            pindah_drive::mesin::pembaruan::bersihkan_unduhan();
        });
    }
    win::set_app_id(APP_ID);

    // Layar laptop kecil: jangan lebih tinggi dari area kerja.
    let tinggi = ambil_arg(&args, "--tinggi")
        .and_then(|t| t.parse::<f32>().ok())
        .unwrap_or_else(|| win::tinggi_area_kerja().map_or(820.0, |h| (h - 56.0).clamp(560.0, 820.0)));
    let lebar = ambil_arg(&args, "--lebar").and_then(|t| t.parse::<f32>().ok()).unwrap_or(880.0);
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
