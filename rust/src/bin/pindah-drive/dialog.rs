//! Dialog modal: konfirmasi, tambah akun, pengaturan (client ID Google), jelajah folder cloud,
//! situs SharePoint.

use eframe::egui::{self, vec2, Align, Key, Layout, RichText, ScrollArea, TextEdit, Ui};
use regex::Regex;
use std::sync::mpsc::Sender;
use std::sync::LazyLock;

use pindah_drive::gaya::{self, kecil, tebal, tombol, tombol_utama, KUNING, MERAH, REDUP, TINTA};
use pindah_drive::mesin::akun::{self, Pustaka, Situs};

use crate::aksi::{self, Kabar};

const ELIPSIS: &str = "\u{2026}";

#[derive(Clone)]
pub enum Konfirmasi {
    PasangRclone,
    Ulang(String),
    Penuh(String),
    Bawaan(String),
    HapusClient,
    /// Hapus jadwal backup (id).
    HapusJadwal(String),
    /// Pindahkan: konfirmasi pertama (penjelasan), lalu kedua (ketik HAPUS).
    Pindahkan1,
    Pindahkan2,
    Lepas(String),
    MulaiSalin,
    Henti,
    TutupApp,
}

pub struct Tanya {
    pub judul: String,
    pub isi: String,
    pub ya: String,
    pub tidak: String,
    pub awas: bool,
    pub tujuan: Konfirmasi,
}

/// Konfirmasi yang harus diketik (tindakan yang menghapus sesuatu).
pub struct Ketik {
    pub judul: String,
    pub isi: String,
    /// Kata yang harus diketik persis, mis. "HAPUS".
    pub kata: String,
    pub ya: String,
    pub isian: String,
    pub tujuan: Konfirmasi,
    pub fokus: bool,
}

pub struct AkunBaru {
    pub jenis: String,
    pub label: String,
    pub juga_sumber: bool,
    /// Sudah ada client ID aplikasi (Pengaturan); bila belum, dialog memberi catatan.
    pub client_bawaan: bool,
    pub fokus: bool,
}

/// Isian client ID Google aplikasi (dibuka dari Pengaturan).
pub struct ClientId {
    pub id: String,
    pub secret: String,
    pub galat: Option<String>,
}

impl ClientId {
    pub fn baru() -> Self {
        ClientId { id: String::new(), secret: String::new(), galat: None }
    }
}

/// Satu akun Google di bagian Client ID Google pada Pengaturan.
pub struct AkunGoogle {
    pub nama: String,
    pub label: String,
    pub client_aplikasi: bool,
    pub client_sendiri: bool,
}

/// Dialog Pengaturan. Isinya potret saat dibuka; diperbarui saat daftar akun dimuat ulang.
pub struct Pengaturan {
    /// Client ID aplikasi (versi pendek, tanpa secret) dan asalnya.
    pub client: Option<(String, String)>,
    pub google: Vec<AkunGoogle>,
    pub pesan: Option<(String, egui::Color32)>,
    /// Pengaturan dibuka saat aplikasi sedang sibuk (login/penyalinan): tombol yang memulai aksi dinonaktifkan.
    pub sibuk: bool,
    /// Lanjut otomatis saat jaringan putus (pengaturan.json: LanjutOtomatis).
    pub lanjut_otomatis: bool,
}

pub struct Jelajah {
    pub remote: String,
    pub jalur: String,
    pub judul: String,
    pub daftar: Vec<String>,
    pub pilih: Option<usize>,
    pub status: String,
    pub gen: u64,
    pub untuk_sumber: bool,
}

pub struct SharePoint {
    pub remote: String,
    pub cari: String,
    pub situs: Vec<Situs>,
    pub pilih_situs: Option<usize>,
    pub pustaka: Vec<Pustaka>,
    pub pilih_pustaka: Option<usize>,
    pub status: String,
    pub url_tempel: Option<String>,
    pub gen: u64,
    pub fokus: bool,
}

pub enum Dialog {
    Tanya(Tanya),
    Ketik(Ketik),
    /// Filter file (lihat filter.rs).
    Filter(crate::filter::FilterIsian),
    /// Riwayat & laporan (lihat riwayat.rs).
    Riwayat(crate::riwayat::Riwayat),
    /// Tambah akun layanan lain (lihat layanan.rs).
    Pemilih(crate::layanan::Pemilih),
    Kredensial(crate::layanan::Kredensial),
    TanyaKonfig(crate::layanan::TanyaKonfig),
    Impor(crate::layanan::Impor),
    AkunBaru(AkunBaru),
    ClientId(ClientId),
    Pengaturan(Pengaturan),
    /// Bantuan & info: tanya jawab, ketentuan, privasi, atribusi, tentang (lihat info.rs).
    Info(crate::info::Info),
    /// Jadwal backup: daftar dan isian (lihat jadwal.rs).
    Jadwal(crate::jadwal::Daftar),
    JadwalIsian(crate::jadwal::Isian),
    Jelajah(Jelajah),
    SharePoint(SharePoint),
    /// Migrasi email (lihat email.rs).
    Email(crate::email::Email),
}

pub enum Hasil {
    Batal,
    BatalSharePoint,
    Ya(Konfirmasi),
    AkunBaru { jenis: String, label: String, izin_penuh: bool },
    /// Simpan client ID aplikasi.
    ClientId { id: String, secret: String },
    /// Kembali ke Pengaturan (isian client ID dibatalkan).
    KembaliPengaturan,
    /// Pengaturan: buka isian client ID.
    GantiClient,
    /// Pengaturan: hapus client ID yang disimpan di komputer ini.
    HapusClient,
    /// Pengaturan: login ulang akun Google ini memakai client ID aplikasi.
    LoginClient(String),
    /// Pengaturan: jadikan client ID akun ini client ID aplikasi.
    AmbilClient(String),
    /// Pengaturan: saklar lanjut otomatis saat jaringan putus diubah.
    LanjutOtomatis(bool),
    /// Jadwal backup: buka isian jadwal baru dari folder yang sedang dibuka.
    JadwalBaru,
    JadwalUbah(String),
    JadwalSimpan {
        id: Option<String>,
        nama: String,
        sumber: String,
        tujuan: String,
        jam: String,
        hari: String,
        verifikasi: bool,
        filter: pindah_drive::mesin::filter::Filter,
    },
    JadwalJalankan(String),
    /// (id, aktif)
    JadwalAktif(String, bool),
    JadwalHapus(String),
    JadwalLog(String),
    KembaliJadwal,
    /// Tambah akun layanan lain: penyedia dipilih (kunci), buka impor, buka rclone config, kembali ke daftar.
    PilihPenyedia(String),
    BukaImpor,
    RcloneConfig,
    KembaliPemilih,
    /// Isian kredensial siap: isian biasa lewat -Opsi, rahasia lewat stdin.
    Kredensial { kunci: String, akun: Option<String>, label: String, opsi: Vec<(String, String)>, rahasia: Vec<(String, String)> },
    /// Jawaban pertanyaan lanjutan rclone.
    JawabKonfig { akun: String, state: String, jawab: String, rahasia: bool, baru: bool },
    /// Pertanyaan lanjutan dibatalkan: akun baru yang setengah jadi dihapus.
    BatalKonfig { akun: String, baru: bool },
    Impor { nama: String, label: String },
    /// Filter file disimpan (kosong = hapus filter).
    FilterSimpan(pindah_drive::mesin::filter::Filter),
    /// Riwayat: buka laporan HTML / simpan CSV per file (id catatan), buka log, ekspor yang tampil.
    /// Saringan dibawa supaya dialog dibuka lagi dengan saringan yang sama.
    RiwayatLaporan(String, crate::riwayat::Saring),
    RiwayatCsvFile(String, crate::riwayat::Saring),
    RiwayatLog(String, crate::riwayat::Saring),
    RiwayatEkspor { html: bool, daftar: Vec<pindah_drive::mesin::riwayat::Catatan>, saring: crate::riwayat::Saring },
    Folder { jalur: String, untuk_sumber: bool },
    Pustaka { remote: String, situs_nama: String, pustaka_id: String, pustaka_nama: String },
    /// Migrasi email: jalankan `pindah email` (kata sandi lewat stdin); dialog tetap terbuka.
    EmailMulai { periksa: bool, args: Vec<String>, rahasia: String },
    EmailHenti,
    /// Bantuan & info > Tentang: periksa rilis terbaru / unduh & pasang.
    PembaruanCek,
    PembaruanPasang,
}

/// Akses untuk memulai pemuatan latar dari dalam dialog.
pub struct Pemuat<'a> {
    pub tx: &'a Sender<Kabar>,
    pub ctx: &'a egui::Context,
    pub gen: &'a mut u64,
}

impl Pemuat<'_> {
    fn gen_baru(&mut self) -> u64 {
        *self.gen += 1;
        *self.gen
    }
}

// ------------------------------------------------------------------ jelajah folder cloud

impl Jelajah {
    pub fn baru(remote: &str, awal: &str, judul: String, untuk_sumber: bool, p: &mut Pemuat) -> Self {
        let mut j = Jelajah {
            remote: remote.to_string(),
            jalur: awal.trim_matches('/').to_string(),
            judul,
            daftar: Vec::new(),
            pilih: None,
            status: String::new(),
            gen: 0,
            untuk_sumber,
        };
        j.muat(p);
        j
    }

    fn muat(&mut self, p: &mut Pemuat) {
        self.daftar.clear();
        self.pilih = None;
        self.status = format!("Memuat daftar folder{ELIPSIS}");
        self.gen = p.gen_baru();
        let (gen, remote, jalur) = (self.gen, self.remote.clone(), self.jalur.clone());
        aksi::latar(p.tx, p.ctx, move || Kabar::Jelajah { gen, hasil: akun::daftar_folder_cloud(&remote, &jalur) });
    }

    pub fn terima(&mut self, hasil: Result<Vec<String>, String>) {
        match hasil {
            Ok(d) => {
                self.status = if d.is_empty() {
                    "Tidak ada subfolder. Klik \"Pilih folder ini\" untuk memakai folder ini.".into()
                } else {
                    format!("{} folder. Klik dua kali untuk membuka.", d.len())
                };
                self.daftar = d;
            }
            Err(e) => self.status = format!("Gagal memuat: {e}"),
        }
    }

    fn masuk(&mut self, nama: &str, p: &mut Pemuat) {
        self.jalur = if self.jalur.is_empty() { nama.to_string() } else { format!("{}/{nama}", self.jalur) };
        self.muat(p);
    }

    fn naik(&mut self, p: &mut Pemuat) {
        self.jalur = match self.jalur.rfind('/') {
            Some(i) => self.jalur[..i].to_string(),
            None => String::new(),
        };
        self.muat(p);
    }
}

// ------------------------------------------------------------------ SharePoint

static RE_SITUS_URL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"/(?:sites|teams)/([^/]+)").unwrap());
static RE_PUSTAKA_UTAMA: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(Dokumen|Documents|Shared Documents|Dokumen Bersama)$").unwrap());

fn dekode_persen(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            let hex = |c: u8| (c as char).to_digit(16);
            if let (Some(a), Some(c)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push((a * 16 + c) as u8);
                i += 3;
                continue;
            }
        }
        out.push(if b[i] == b'+' { b' ' } else { b[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

impl SharePoint {
    pub fn baru(remote: &str) -> Self {
        SharePoint {
            remote: remote.to_string(),
            cari: String::new(),
            situs: Vec::new(),
            pilih_situs: None,
            pustaka: Vec::new(),
            pilih_pustaka: None,
            status: "Contoh: ketik \"keuangan\" atau tempel alamat situs, lalu tekan Enter.".into(),
            url_tempel: None,
            gen: 0,
            fokus: true,
        }
    }

    pub fn cari(&mut self, p: &mut Pemuat) {
        let teks = self.cari.trim().to_string();
        if teks.is_empty() {
            self.status = "Ketik nama situs atau tempel alamatnya dulu.".into();
            return;
        }
        self.url_tempel = None;
        let mut kata = teks.clone();
        // Alamat yang ditempel: cari dengan nama situsnya, lalu pilih yang alamatnya sama persis.
        if teks.starts_with("http://") || teks.starts_with("https://") {
            let url = teks.split(['?', '#']).next().unwrap_or("").trim_end_matches('/').to_string();
            kata = match RE_SITUS_URL.captures(&url) {
                Some(c) => dekode_persen(&c[1]),
                None => "*".into(),
            };
            self.url_tempel = Some(url);
        }
        self.situs.clear();
        self.pustaka.clear();
        self.pilih_situs = None;
        self.pilih_pustaka = None;
        self.status = format!("Mencari situs \"{kata}\"{ELIPSIS}");
        self.gen = p.gen_baru();
        let (gen, remote) = (self.gen, self.remote.clone());
        aksi::latar(p.tx, p.ctx, move || Kabar::Situs { gen, hasil: akun::cari_situs(&remote, &kata) });
    }

    fn muat_pustaka(&mut self, p: &mut Pemuat) {
        let Some(s) = self.pilih_situs.and_then(|i| self.situs.get(i)).cloned() else { return };
        self.pustaka.clear();
        self.pilih_pustaka = None;
        self.status = format!("Memuat pustaka di \"{}\"{ELIPSIS}", s.nama);
        self.gen = p.gen_baru();
        let (gen, remote) = (self.gen, self.remote.clone());
        aksi::latar(p.tx, p.ctx, move || Kabar::Pustaka { gen, hasil: akun::daftar_pustaka(&remote, &s.id) });
    }

    pub fn terima_situs(&mut self, hasil: Result<Vec<Situs>, String>, p: &mut Pemuat) {
        match hasil {
            Err(e) => self.status = format!("Pencarian gagal: {e}"),
            Ok(d) => {
                self.situs = d;
                let cocok = self.url_tempel.as_ref().and_then(|u| {
                    self.situs.iter().position(|s| s.url.trim_end_matches('/').eq_ignore_ascii_case(u))
                });
                if self.situs.is_empty() {
                    self.status = "Tidak ada situs yang cocok, atau akun ini tidak punya akses ke situs tersebut.".into();
                } else if let Some(i) = cocok {
                    self.pilih_situs = Some(i);
                    self.muat_pustaka(p);
                } else if self.url_tempel.is_some() {
                    self.status = "Alamat yang ditempel tidak ditemukan persis; pilih situsnya dari daftar.".into();
                } else {
                    self.status = format!("{} situs ditemukan. Pilih salah satu.", self.situs.len());
                }
            }
        }
    }

    pub fn terima_pustaka(&mut self, hasil: Result<Vec<Pustaka>, String>) {
        match hasil {
            Err(e) => self.status = format!("Pustaka tidak bisa dibaca: {e}"),
            Ok(d) => {
                self.pustaka = d;
                if self.pustaka.is_empty() {
                    self.status = "Situs ini tidak punya pustaka dokumen yang bisa Anda akses.".into();
                } else {
                    self.pilih_pustaka =
                        Some(self.pustaka.iter().position(|p| RE_PUSTAKA_UTAMA.is_match(&p.nama)).unwrap_or(0));
                    self.status = format!("{} pustaka. Pilih lalu klik \"Pakai pustaka ini\".", self.pustaka.len());
                }
            }
        }
    }
}

// ------------------------------------------------------------------ tampilan

/// Satu baris daftar: selebar kotak, rata kiri, teks panjang dipotong.
fn item(ui: &mut Ui, teks: &str, terpilih: bool) -> egui::Response {
    let w = ui.available_width();
    ui.allocate_ui_with_layout(vec2(w, 26.0), Layout::left_to_right(Align::Center).with_main_justify(true), |ui| {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
        ui.add(egui::SelectableLabel::new(terpilih, teks))
    })
    .inner
}

/// Satu baris folder di jendela Jelajah: ikon map + nama (sama dengan panel utama).
fn item_map(ui: &mut Ui, teks: &str, terpilih: bool) -> egui::Response {
    let w = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(vec2(w, 26.0), egui::Sense::click());
    pindah_drive::gaya::nama_tombol(&resp, true, teks);
    if ui.is_rect_visible(r) {
        let p = ui.painter();
        if terpilih {
            p.rect_filled(r, 4.0, gaya::PILIH);
        } else if resp.hovered() {
            p.rect_filled(r, 4.0, gaya::HOVER);
        }
        gaya::lukis_map(p, egui::pos2(r.left() + 15.0, r.center().y), 16.0);
        let g = gaya::potong(ui, teks, egui::FontId::proportional(13.5), TINTA, w - 40.0);
        p.galley(egui::pos2(r.left() + 30.0, r.center().y - g.size().y / 2.0), g, TINTA);
    }
    resp
}

fn judul(ui: &mut Ui, t: &str) {
    ui.label(RichText::new(t).family(tebal()).size(16.5).color(TINTA));
}

fn baris_tombol(ui: &mut Ui, isi: impl FnOnce(&mut Ui)) {
    ui.add_space(18.0);
    ui.with_layout(Layout::right_to_left(Align::Center), isi);
}

fn isian(ui: &mut Ui, teks: &mut String, sandi: bool) -> egui::Response {
    ui.add(
        TextEdit::singleline(teks)
            .password(sandi)
            .desired_width(f32::INFINITY)
            .min_size(vec2(0.0, 36.0))
            .vertical_align(Align::Center)
            .margin(gaya::JARAK_ISIAN),
    )
}

pub fn tampil(ctx: &egui::Context, d: &mut Dialog, p: &mut Pemuat) -> Option<Hasil> {
    // Teks dialog tidak menempel ke tepi: 24 px di kiri-kanan, sedikit kurang di atas-bawah.
    let bingkai = egui::Frame::popup(&ctx.style())
        .inner_margin(egui::Margin { left: 24, right: 24, top: 22, bottom: 20 })
        .corner_radius(14);
    let tirai = egui::Color32::from_rgba_unmultiplied(0x08, 0x28, 0x24, 120);
    let r = egui::Modal::new(egui::Id::new("dialog-pindah-drive")).frame(bingkai).backdrop_color(tirai).show(ctx, |ui| match d {
        Dialog::Tanya(t) => tanya(ui, t),
        Dialog::Ketik(k) => ketik(ui, k),
        Dialog::Filter(f) => crate::filter::tampil(ui, f),
        Dialog::Riwayat(r) => crate::riwayat::tampil(ui, r),
        Dialog::Pemilih(s) => crate::layanan::pemilih(ui, s),
        Dialog::Kredensial(s) => crate::layanan::kredensial(ui, s),
        Dialog::TanyaKonfig(s) => crate::layanan::tanya_konfig(ui, s),
        Dialog::Impor(s) => crate::layanan::impor(ui, s),
        Dialog::AkunBaru(a) => akun_baru(ui, a),
        Dialog::ClientId(c) => client_id(ui, c),
        Dialog::Pengaturan(s) => pengaturan(ui, s),
        Dialog::Info(i) => crate::info::tampil(ui, i),
        Dialog::Jadwal(j) => crate::jadwal::daftar(ui, j),
        Dialog::JadwalIsian(j) => crate::jadwal::isian(ui, j),
        Dialog::Jelajah(j) => jelajah(ui, j, p),
        Dialog::SharePoint(s) => sharepoint(ui, s, p),
        Dialog::Email(e) => crate::email::tampil(ui, e),
    });
    if r.inner.is_some() {
        return r.inner;
    }
    // Escape menutup dialog (klik di luar dialog tidak, supaya isian tidak hilang tak sengaja).
    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        return Some(match d {
            Dialog::SharePoint(_) => Hasil::BatalSharePoint,
            Dialog::ClientId(_) => Hasil::KembaliPengaturan,
            Dialog::JadwalIsian(_) => Hasil::KembaliJadwal,
            Dialog::Kredensial(k) if k.akun.is_none() => Hasil::KembaliPemilih,
            Dialog::Impor(_) => Hasil::KembaliPemilih,
            Dialog::TanyaKonfig(t) => Hasil::BatalKonfig { akun: t.akun.clone(), baru: t.baru },
            _ => Hasil::Batal,
        });
    }
    None
}

fn tanya(ui: &mut Ui, t: &mut Tanya) -> Option<Hasil> {
    ui.set_width(440.0);
    ui.horizontal(|ui| {
        if t.awas {
            ui.label(gaya::ikon(gaya::ik::AWAS, 16.0, KUNING));
        }
        ui.label(RichText::new(&t.judul).family(tebal()).size(16.5).color(if t.awas { KUNING } else { TINTA }));
    });
    ui.add_space(8.0);
    ui.label(RichText::new(&t.isi).color(TINTA));
    let mut h = None;
    baris_tombol(ui, |ui| {
        if ui.add(tombol_utama(&t.ya)).clicked() {
            h = Some(Hasil::Ya(t.tujuan.clone()));
        }
        if !t.tidak.is_empty() && ui.add(tombol(&t.tidak).min_size(vec2(0.0, 38.0))).clicked() {
            h = Some(Hasil::Batal);
        }
    });
    h
}

/// Konfirmasi kedua untuk tindakan yang menghapus: tombolnya aktif setelah kata diketik persis.
fn ketik(ui: &mut Ui, k: &mut Ketik) -> Option<Hasil> {
    ui.set_width(460.0);
    ui.horizontal(|ui| {
        ui.label(gaya::ikon(gaya::ik::AWAS, 16.0, KUNING));
        ui.label(RichText::new(&k.judul).family(tebal()).size(16.5).color(KUNING));
    });
    ui.add_space(8.0);
    ui.label(RichText::new(&k.isi).color(TINTA));
    ui.add_space(10.0);
    ui.label(RichText::new(format!("Ketik {} untuk melanjutkan:", k.kata)).color(REDUP));
    let r = isian(ui, &mut k.isian, false);
    if k.fokus {
        r.request_focus();
        k.fokus = false;
    }
    let cocok = k.isian.trim() == k.kata;
    let enter = cocok && r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
    let mut h = None;
    baris_tombol(ui, |ui| {
        if ui.add_enabled(cocok, tombol_utama(&k.ya)).clicked() || enter {
            h = Some(Hasil::Ya(k.tujuan.clone()));
        }
        if ui.add(tombol("Batal").min_size(vec2(0.0, 38.0))).clicked() {
            h = Some(Hasil::Batal);
        }
    });
    h
}

fn akun_baru(ui: &mut Ui, a: &mut AkunBaru) -> Option<Hasil> {
    ui.set_width(480.0);
    let (j, mut ket, lanjut) = match a.jenis.as_str() {
        "sharepoint" => (
            "Tambah situs SharePoint",
            "Setelah ini browser terbuka untuk login dengan akun Microsoft kantor/sekolah yang punya akses ke situs SharePoint. Sesudah login, Anda memilih situs dan pustaka dokumennya. Nama tampilan boleh dikosongkan; akan diisi otomatis dari nama situs.".to_string(),
            "Lanjut ke login",
        ),
        "google" => (
            "Tambah akun Google Drive",
            "Setelah ini browser terbuka untuk login ke akun Google tersebut.".to_string(),
            "Lanjut",
        ),
        _ => (
            "Tambah akun OneDrive",
            "Setelah ini browser terbuka untuk login ke akun Microsoft (pribadi atau kantor/sekolah). Sebagian akun kantor ditolak dengan pesan \"Need admin approval\"; bila begitu, pakai folder OneDrive di komputer.".to_string(),
            "Lanjut ke login",
        ),
    };
    if a.jenis == "google" && a.client_bawaan {
        ket += " Memakai client ID aplikasi (Pengaturan).";
    }
    judul(ui, j);
    ui.add_space(4.0);
    ui.label(RichText::new(ket).color(REDUP));
    ui.add_space(12.0);
    ui.label(RichText::new("Nama tampilan, misalnya Pribadi atau Kantor").color(REDUP));
    let r = isian(ui, &mut a.label, false);
    if a.fokus {
        r.request_focus();
        a.fokus = false;
    }
    if a.jenis == "google" {
        ui.add_space(12.0);
        ui.label(RichText::new("Akun ini akan dipakai sebagai").color(REDUP));
        ui.radio_value(&mut a.juga_sumber, false, "Tujuan saja (izin terbatas: hanya melihat file yang disalin LELEMOVE)");
        ui.radio_value(&mut a.juga_sumber, true, "Sumber dan tujuan (izin baca-tulis penuh ke Google Drive ini)");
        if !a.client_bawaan {
            ui.add_space(8.0);
            ui.label(kecil(
                "Belum ada client ID aplikasi, jadi akun ini memakai client ID bersama rclone yang sering dibatasi Google (penyalinan bisa macet). Pasang client ID lewat Pengaturan, lalu login ulang akun ini.",
                KUNING,
            ));
        }
    }
    let mut h = None;
    let enter = r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
    baris_tombol(ui, |ui| {
        if ui.add(tombol_utama(lanjut)).clicked() || enter {
            h = Some(Hasil::AkunBaru {
                jenis: a.jenis.clone(),
                label: a.label.trim().to_string(),
                izin_penuh: a.jenis == "google" && a.juga_sumber,
            });
        }
        if ui.add(tombol("Batal").min_size(vec2(0.0, 38.0))).clicked() {
            h = Some(Hasil::Batal);
        }
    });
    h
}

fn langkah(ui: &mut Ui, n: &str, sebelum: &str, tautan: &str, url: &str, sesudah: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label(format!("{n}.  {sebelum}"));
        ui.hyperlink_to(tautan, url);
        ui.label(sesudah);
    });
}

fn client_id(ui: &mut Ui, c: &mut ClientId) -> Option<Hasil> {
    ui.set_width(600.0);
    judul(ui, "Client ID Google aplikasi");
    ui.add_space(4.0);
    ui.label(RichText::new("Client ID bersama milik rclone dipakai semua pengguna rclone di dunia dan sering kehabisan kuota, sehingga penyalinan macet. Client ID sendiri gratis dan kuotanya khusus untuk Anda. Cukup dibuat sekali, sekitar 5 menit:").color(REDUP));
    ui.add_space(8.0);
    langkah(ui, "1", "", "Buat project baru", "https://console.cloud.google.com/projectcreate", " di Google Cloud (nama bebas, misalnya LELEMOVE).");
    langkah(ui, "2", "", "Aktifkan Google Drive API", "https://console.cloud.google.com/apis/library/drive.googleapis.com", " untuk project itu.");
    langkah(ui, "3", "Isi ", "Branding", "https://console.cloud.google.com/auth/branding", ": nama aplikasi dan email Anda; pilih Audience \"External\".");
    langkah(ui, "4", "Di ", "Clients", "https://console.cloud.google.com/auth/clients", ", buat client jenis \"Desktop app\", lalu salin Client ID dan Client secret ke bawah.");
    langkah(ui, "5", "Di ", "Audience", "https://console.cloud.google.com/auth/audience", ": selama status \"Testing\", tambahkan email setiap pemakai di Test users; login berlaku 7 hari, lalu cukup Login ulang. Agar tidak kedaluwarsa, klik \"Publish app\" (Google meminta alamat beranda dan kebijakan privasi di Branding).");
    ui.add_space(10.0);
    ui.label(RichText::new("Client ID").color(REDUP));
    isian(ui, &mut c.id, false);
    ui.add_space(6.0);
    ui.label(RichText::new("Client secret").color(REDUP));
    isian(ui, &mut c.secret, true);
    ui.add_space(6.0);
    ui.label(kecil(
        "Saat login, Google bisa menampilkan \"Google hasn't verified this app\". Karena aplikasinya milik Anda sendiri, klik Advanced (Lanjutan) lalu \"Go to ... (unsafe)\".",
        REDUP,
    ));
    ui.add_space(6.0);
    ui.label(kecil(
        "Client ID ini dipakai untuk login semua akun Google di LELEMOVE dan ikut dibawa installer yang dibangun dari komputer ini, sehingga pengguna lain tidak perlu menyiapkannya. Siapa pun yang memegang installer itu bisa membaca client ID dan secret-nya.",
        REDUP,
    ));
    if let Some(g) = &c.galat {
        ui.add_space(6.0);
        ui.label(RichText::new(g).color(MERAH));
    }
    let mut h = None;
    baris_tombol(ui, |ui| {
        if ui.add(tombol_utama("Simpan")).clicked() {
            let (id, rahasia) = (c.id.trim().to_string(), c.secret.trim().to_string());
            if !id.ends_with(".apps.googleusercontent.com") {
                c.galat = Some("Client ID biasanya berakhiran \".apps.googleusercontent.com\". Salin ulang dari halaman Clients.".into());
            } else if rahasia.is_empty() {
                c.galat = Some("Client secret belum diisi.".into());
            } else {
                h = Some(Hasil::ClientId { id, secret: rahasia });
            }
        }
        if ui.add(tombol("Batal").min_size(vec2(0.0, 38.0))).clicked() {
            h = Some(Hasil::KembaliPengaturan);
        }
    });
    h
}

/// Titik status kecil (hijau = siap, kuning = perlu perhatian).
fn titik(ui: &mut Ui, warna: egui::Color32) {
    let (r, _) = ui.allocate_exact_size(vec2(8.0, 8.0), egui::Sense::hover());
    ui.painter().circle_filled(r.center(), 4.0, warna);
}

fn subjudul(ui: &mut Ui, t: &str) {
    ui.label(RichText::new(t).family(tebal()).size(14.5).color(TINTA));
}

/// Pengaturan aplikasi. Saat ini berisi client ID Google (dulu ada di menu tiap akun).
fn pengaturan(ui: &mut Ui, s: &mut Pengaturan) -> Option<Hasil> {
    ui.set_width(560.0);
    judul(ui, "Pengaturan");
    ui.add_space(10.0);
    subjudul(ui, "Client ID Google");
    ui.add_space(2.0);
    ui.label(RichText::new("Dipakai untuk login semua akun Google di LELEMOVE. Client ID bersama milik rclone sering kehabisan kuota sehingga penyalinan macet; client ID sendiri gratis dan kuotanya khusus untuk Anda.").color(REDUP));
    ui.add_space(8.0);
    let mut h = None;
    egui::Frame::new()
        .fill(gaya::LATAR2)
        .stroke(egui::Stroke::new(1.0_f32, gaya::GARIS))
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                match &s.client {
                    Some((id, asal)) => {
                        titik(ui, gaya::HIJAU);
                        ui.label(RichText::new("Terpasang").family(tebal()).color(TINTA));
                        ui.label(RichText::new(id).family(egui::FontFamily::Monospace).size(12.5).color(TINTA));
                        ui.label(kecil(&format!("(dari {asal})"), REDUP));
                    }
                    None => {
                        titik(ui, KUNING);
                        ui.label(RichText::new("Belum ada").family(tebal()).color(TINTA));
                        ui.label(kecil("akun Google memakai client ID bersama rclone", REDUP));
                    }
                }
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let t = if s.client.is_some() { format!("Ganti client ID{ELIPSIS}") } else { format!("Pasang client ID{ELIPSIS}") };
                if ui.add(tombol(&t)).clicked() {
                    h = Some(Hasil::GantiClient);
                }
                if s.client.as_ref().is_some_and(|(_, asal)| asal == "komputer ini") && ui.add(tombol("Hapus")).clicked() {
                    h = Some(Hasil::HapusClient);
                }
            });
            // Belum ada client ID aplikasi, tetapi ada akun yang sudah memakai client ID sendiri.
            if s.client.is_none() {
                for a in s.google.iter().filter(|a| a.client_sendiri) {
                    ui.add_space(4.0);
                    if ui.add_enabled(!s.sibuk, tombol(&format!("Ambil dari akun {}", a.label))).clicked() {
                        h = Some(Hasil::AmbilClient(a.nama.clone()));
                    }
                }
            }
        });

    if !s.google.is_empty() {
        ui.add_space(12.0);
        subjudul(ui, "Akun Google");
        ui.add_space(2.0);
        for a in &s.google {
            ui.horizontal(|ui| {
                ui.set_min_height(32.0);
                let (warna, ket) = if a.client_aplikasi {
                    (gaya::HIJAU, "memakai client ID aplikasi")
                } else if a.client_sendiri {
                    (KUNING, "memakai client ID lain")
                } else {
                    (KUNING, "memakai client ID bersama rclone")
                };
                titik(ui, warna);
                ui.label(RichText::new(&a.label).color(TINTA));
                ui.label(kecil(&format!("\u{00B7}  {ket}"), REDUP));
                if s.client.is_some() && !a.client_aplikasi {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add_enabled(!s.sibuk, tombol("Login ulang dengan client ID ini"))
                            .on_hover_text("Browser terbuka untuk login ke akun Google ini.")
                            .clicked()
                        {
                            h = Some(Hasil::LoginClient(a.nama.clone()));
                        }
                    });
                }
            });
        }
    }

    ui.add_space(10.0);
    ui.label(kecil(
        "Selama project Google Cloud pemilik client ID masih berstatus \"Testing\", login Google kedaluwarsa setiap 7 hari (cukup Login ulang). Klik \"Publish app\" di Google Cloud Console agar tidak kedaluwarsa.",
        REDUP,
    ));

    ui.add_space(14.0);
    ui.separator();
    ui.add_space(6.0);
    subjudul(ui, "Penyalinan");
    ui.add_space(2.0);
    let mut lanjut = s.lanjut_otomatis;
    if ui.checkbox(&mut lanjut, "Lanjutkan otomatis bila jaringan putus").changed() {
        s.lanjut_otomatis = lanjut;
        h = Some(Hasil::LanjutOtomatis(lanjut));
    }
    ui.label(kecil(
        "Bila internet atau DNS putus di tengah penyalinan, LELEMOVE menunggu (30 detik sampai 10 menit, makin lama makin jarang), menguji sambungan ke Google/Microsoft, lalu melanjutkan sendiri. File yang sudah sampai tidak diulang. Berhenti setelah 25 percobaan, atau bila internet tidak kembali dalam 6 jam. Login kedaluwarsa dan batas unggah harian Google tetap butuh tindakan Anda.",
        REDUP,
    ));
    if let Some((p, w)) = &s.pesan {
        ui.add_space(8.0);
        ui.label(RichText::new(p).color(*w));
    }
    baris_tombol(ui, |ui| {
        if ui.add(tombol_utama("Tutup")).clicked() {
            h = Some(Hasil::Batal);
        }
    });
    h
}

fn jelajah(ui: &mut Ui, j: &mut Jelajah, p: &mut Pemuat) -> Option<Hasil> {
    ui.set_width(520.0);
    ui.add(egui::Label::new(RichText::new(&j.judul).color(REDUP)).truncate());
    let tampil = if j.jalur.is_empty() { "/".to_string() } else { format!("/{}", j.jalur) };
    ui.add(egui::Label::new(RichText::new(tampil).family(tebal()).size(15.5).color(TINTA)).truncate());
    ui.add_space(6.0);
    let mut masuk: Option<String> = None;
    egui::Frame::new()
        .stroke(egui::Stroke::new(1.0_f32, gaya::GARIS))
        .corner_radius(6)
        .inner_margin(egui::Margin::same(4))
        .show(ui, |ui| {
            ScrollArea::vertical().min_scrolled_height(300.0).max_height(300.0).auto_shrink([false, false]).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 0.0;
                for (i, n) in j.daftar.iter().enumerate() {
                    let r = item_map(ui, n, j.pilih == Some(i));
                    if r.clicked() {
                        j.pilih = Some(i);
                    }
                    if r.double_clicked() {
                        masuk = Some(n.clone());
                    }
                }
            });
        });
    ui.add_space(4.0);
    ui.label(kecil(&j.status, REDUP));
    if let Some(n) = masuk {
        j.masuk(&n, p);
    }
    let mut h = None;
    let mut naik = false;
    let mut buka = false;
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        if ui.add_enabled(!j.jalur.is_empty(), tombol("Naik satu tingkat")).clicked() {
            naik = true;
        }
        if ui.add_enabled(j.pilih.is_some(), tombol("Buka")).clicked() {
            buka = true;
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.add(tombol_utama("Pilih folder ini")).clicked() {
                h = Some(Hasil::Folder { jalur: j.jalur.clone(), untuk_sumber: j.untuk_sumber });
            }
            if ui.add(tombol("Batal").min_size(vec2(0.0, 38.0))).clicked() {
                h = Some(Hasil::Batal);
            }
        });
    });
    if naik {
        j.naik(p);
    }
    if buka {
        if let Some(n) = j.pilih.and_then(|i| j.daftar.get(i)).cloned() {
            j.masuk(&n, p);
        }
    }
    h
}

fn sharepoint(ui: &mut Ui, s: &mut SharePoint, p: &mut Pemuat) -> Option<Hasil> {
    ui.set_width(600.0);
    judul(ui, "Pilih situs dan pustaka dokumen");
    ui.add_space(4.0);
    ui.label(RichText::new("Ketik nama situs, atau tempel alamatnya dari browser (misalnya https://namatenant.sharepoint.com/sites/NamaSitus), lalu klik Cari.").color(REDUP));
    ui.add_space(8.0);
    let mut cari = false;
    ui.horizontal(|ui| {
        let w = ui.available_width() - 80.0;
        let r = ui.add(
            TextEdit::singleline(&mut s.cari)
                .desired_width(w)
                .min_size(vec2(w, 32.0))
                .vertical_align(Align::Center)
                .margin(pindah_drive::gaya::JARAK_ISIAN),
        );
        if s.fokus {
            r.request_focus();
            s.fokus = false;
        }
        if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
            cari = true;
        }
        if ui.add(tombol("Cari").min_size(vec2(72.0, 32.0))).clicked() {
            cari = true;
        }
    });
    if cari {
        s.cari(p);
    }
    ui.add_space(8.0);
    ui.label(RichText::new("Situs").color(REDUP));
    let mut situs_baru: Option<usize> = None;
    egui::Frame::new()
        .stroke(egui::Stroke::new(1.0_f32, gaya::GARIS))
        .corner_radius(6)
        .inner_margin(egui::Margin::same(4))
        .show(ui, |ui| {
            ScrollArea::vertical().id_salt("situs").min_scrolled_height(190.0).max_height(190.0).auto_shrink([false, false]).show(ui, |ui| {
                ui.set_width(ui.available_width());
                for (i, st) in s.situs.iter().enumerate() {
                    let teks = format!("{}   {}", st.nama, st.url);
                    let r = item(ui, &teks, s.pilih_situs == Some(i));
                    if r.clicked() && s.pilih_situs != Some(i) {
                        situs_baru = Some(i);
                    }
                }
            });
        });
    if let Some(i) = situs_baru {
        s.pilih_situs = Some(i);
        s.muat_pustaka(p);
    }
    ui.add_space(8.0);
    ui.label(RichText::new("Pustaka dokumen di situs terpilih").color(REDUP));
    egui::Frame::new()
        .stroke(egui::Stroke::new(1.0_f32, gaya::GARIS))
        .corner_radius(6)
        .inner_margin(egui::Margin::same(4))
        .show(ui, |ui| {
            ScrollArea::vertical().id_salt("pustaka").min_scrolled_height(110.0).max_height(110.0).auto_shrink([false, false]).show(ui, |ui| {
                ui.set_width(ui.available_width());
                for (i, pk) in s.pustaka.iter().enumerate() {
                    let r = item(ui, &&pk.nama, s.pilih_pustaka == Some(i));
                    if r.clicked() {
                        s.pilih_pustaka = Some(i);
                    }
                }
            });
        });
    ui.add_space(4.0);
    ui.label(kecil(&s.status, REDUP));
    let mut h = None;
    baris_tombol(ui, |ui| {
        if ui.add_enabled(s.pilih_pustaka.is_some(), tombol_utama("Pakai pustaka ini")).clicked() {
            let situs = s.pilih_situs.and_then(|i| s.situs.get(i));
            let pustaka = s.pilih_pustaka.and_then(|i| s.pustaka.get(i));
            if let (Some(st), Some(pk)) = (situs, pustaka) {
                h = Some(Hasil::Pustaka {
                    remote: s.remote.clone(),
                    situs_nama: st.nama.clone(),
                    pustaka_id: pk.id.clone(),
                    pustaka_nama: pk.nama.clone(),
                });
            }
        }
        if ui.add(tombol("Batal").min_size(vec2(0.0, 38.0))).clicked() {
            h = Some(Hasil::BatalSharePoint);
        }
    });
    h
}
