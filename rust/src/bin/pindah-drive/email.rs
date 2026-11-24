//! Dialog "Migrasi email": menyalin semua folder email dari Gmail, Outlook/Microsoft 365, atau
//! server IMAP lain ke server email sendiri. Pekerjaannya dilakukan `pindah email` di proses
//! terpisah; dialog ini berisi isian, lalu progres, lalu hasil per folder. Kata sandi hanya
//! dikirim lewat stdin ke proses itu dan tidak disimpan.

use eframe::egui::{self, vec2, Align, Color32, ComboBox, Layout, RichText, ScrollArea, TextEdit, Ui};
use serde_json::{json, Value};

use pindah_drive::gaya::{self, kecil, tebal, tombol, tombol_utama, HIJAU, KUNING, MERAH, REDUP, TAUTAN, TINTA};
use pindah_drive::mesin::dasar::{angka, format_ukuran, teks};
use pindah_drive::mesin::email::{self as em, Akun, Jenis, Setelan};
use pindah_drive::mesin::imap::Keamanan;
use pindah_drive::win;

use crate::dialog::Hasil;

const TITIK: &str = "\u{00B7}";
const PANAH: &str = "\u{2192}";

pub const LAYANAN: [&str; 3] = ["Gmail / Google Workspace", "Outlook / Microsoft 365", "Server IMAP lain"];
const AMAN: [(Keamanan, &str); 3] =
    [(Keamanan::Ssl, "SSL/TLS"), (Keamanan::StartTls, "STARTTLS"), (Keamanan::Tanpa, "Tanpa enkripsi")];
const PANDUAN_AZURE: &str = "https://github.com/fauzirpl/lelemove#outlook-butuh-client-id-azure";
const SANDI_APLIKASI_GOOGLE: &str = "https://myaccount.google.com/apppasswords";

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tahap {
    Isi,
    Jalan { periksa: bool },
    /// kode: 0 berhasil, lain = gagal/sebagian, -1 dihentikan.
    Selesai { periksa: bool, kode: i32 },
}

/// Isian satu server IMAP.
#[derive(Clone, Default)]
pub struct Server {
    pub host: String,
    pub port: String,
    pub aman: usize,
    pub pengguna: String,
    pub sandi: String,
}

impl Server {
    fn dari(a: &Akun) -> Server {
        Server {
            host: a.host.clone(),
            port: a.port.to_string(),
            aman: AMAN.iter().position(|(k, _)| *k == a.keamanan).unwrap_or(0),
            pengguna: a.pengguna.clone(),
            sandi: String::new(),
        }
    }

    /// "imap://pengguna@server:port" (skema sesuai keamanan).
    fn alamat(&self, sisi: &str) -> Result<String, String> {
        let host = self.host.trim().trim_start_matches("imaps://").trim_start_matches("imap://").trim_end_matches('/');
        let pengguna = self.pengguna.trim();
        if host.is_empty() || host.contains(char::is_whitespace) {
            return Err(format!("Isi alamat server {sisi}, misalnya mail.kantor.go.id."));
        }
        if pengguna.is_empty() {
            return Err(format!("Isi nama pengguna {sisi} (biasanya alamat email lengkap)."));
        }
        let port: u16 = self.port.trim().parse().map_err(|_| format!("Port {sisi} harus angka, misalnya 993."))?;
        let skema = match AMAN[self.aman.min(2)].0 {
            Keamanan::Ssl => "imap",
            Keamanan::StartTls => "imap+starttls",
            Keamanan::Tanpa => "imap+polos",
        };
        let a = format!("{skema}://{pengguna}@{host}:{port}");
        Akun::urai(&a).map(|_| a)
    }
}

pub struct Email {
    /// 0 Gmail, 1 Outlook, 2 IMAP lain.
    pub layanan: usize,
    pub email: String,
    pub login_google: bool,
    pub sandi_aplikasi: String,
    pub sumber: Server,
    pub client_ms: String,
    pub client_ms_ada: bool,
    pub ada_client_google: bool,
    pub tujuan: Server,
    pub sertakan_sampah: bool,
    pub arsip: bool,
    pub abaikan_sertifikat: bool,
    pub tahap: Tahap,
    pub progres: Option<Value>,
    pub hasil: Option<Value>,
    /// Baris terakhir dari proses (ditampilkan selama berjalan).
    pub baris: String,
    pub pesan: Option<(String, Color32)>,
    /// Aksi lain sedang berjalan: migrasi belum boleh dimulai (diisi jendela setiap frame).
    pub sibuk: bool,
}

impl Email {
    /// Isian awal dari setelan migrasi terakhir (email.json).
    pub fn baru(set: &Setelan, client_ms_ada: bool, ada_client_google: bool) -> Email {
        let mut e = Email {
            layanan: 0,
            email: String::new(),
            login_google: set.login_google && ada_client_google,
            sandi_aplikasi: String::new(),
            sumber: Server { port: "993".into(), ..Default::default() },
            client_ms: String::new(),
            client_ms_ada,
            ada_client_google,
            tujuan: Server { port: "993".into(), ..Default::default() },
            sertakan_sampah: set.sertakan_sampah,
            arsip: !set.tanpa_arsip,
            abaikan_sertifikat: set.abaikan_sertifikat,
            tahap: Tahap::Isi,
            progres: None,
            hasil: None,
            baris: String::new(),
            pesan: None,
            sibuk: false,
        };
        if let Ok(a) = Akun::urai(&set.sumber) {
            match a.jenis {
                Jenis::Gmail => e.email = a.pengguna,
                Jenis::Outlook => {
                    e.layanan = 1;
                    e.email = a.pengguna;
                }
                Jenis::Imap => {
                    e.layanan = 2;
                    e.sumber = Server::dari(&a);
                }
            }
        }
        if let Ok(a) = Akun::urai(&set.tujuan) {
            e.tujuan = Server::dari(&a);
        }
        e
    }

    /// Kata sandi dihapus dari memori (dialog ditutup saat migrasi berjalan di latar).
    pub fn lupakan_sandi(&mut self) {
        self.sandi_aplikasi.clear();
        self.sumber.sandi.clear();
        self.tujuan.sandi.clear();
    }

    pub fn berjalan(&self) -> bool {
        matches!(self.tahap, Tahap::Jalan { .. })
    }

    /// Argumen `pindah email` dan isi stdin (kata sandi sebagai JSON).
    fn perintah(&mut self, periksa: bool) -> Result<(Vec<String>, String), String> {
        let mut args: Vec<String> = vec!["email".into()];
        let mut rahasia = serde_json::Map::new();
        let sumber = match self.layanan {
            0 | 1 => {
                let e = self.email.trim();
                if !e.contains('@') || e.contains(char::is_whitespace) {
                    return Err("Isi alamat email lama yang lengkap, misalnya nama@gmail.com.".into());
                }
                if self.layanan == 0 {
                    if self.login_google {
                        args.extend(["-Login".into(), "google".into()]);
                    } else {
                        let s: String = self.sandi_aplikasi.chars().filter(|c| !c.is_whitespace()).collect();
                        if s.is_empty() {
                            return Err("Isi sandi aplikasi Google (16 huruf), atau pilih Login Google di browser.".into());
                        }
                        rahasia.insert("Sumber".into(), Value::String(s));
                        args.extend(["-Login".into(), "sandi".into()]);
                    }
                    format!("gmail:{e}")
                } else {
                    if !self.client_ms_ada {
                        let id = self.client_ms.trim();
                        if !em::client_microsoft_sah(id) {
                            return Err("Isi Application (client) ID dari aplikasi Azure Anda (lihat panduan).".into());
                        }
                        args.extend(["-ClientMicrosoft".into(), id.to_string()]);
                    }
                    format!("outlook:{e}")
                }
            }
            _ => {
                let a = self.sumber.alamat("lama")?;
                if self.sumber.sandi.is_empty() {
                    return Err("Isi kata sandi akun email lama.".into());
                }
                rahasia.insert("Sumber".into(), Value::String(self.sumber.sandi.clone()));
                a
            }
        };
        let tujuan = self.tujuan.alamat("baru")?;
        if self.tujuan.sandi.is_empty() {
            return Err("Isi kata sandi akun di server email baru.".into());
        }
        rahasia.insert("Tujuan".into(), Value::String(self.tujuan.sandi.clone()));
        args.extend(["-Sumber".into(), sumber, "-Tujuan".into(), tujuan, "-RahasiaStdin".into(), "-Ya".into()]);
        if periksa {
            args.push("-HanyaPeriksa".into());
        }
        if self.sertakan_sampah {
            args.push("-SertakanSampah".into());
        }
        if self.layanan == 0 && !self.arsip {
            args.push("-TanpaArsip".into());
        }
        if self.abaikan_sertifikat {
            args.push("-AbaikanSertifikat".into());
        }
        Ok((args, Value::Object(rahasia).to_string()))
    }
}

fn judul(ui: &mut Ui, t: &str) {
    ui.label(RichText::new(t).family(tebal()).size(16.5).color(TINTA));
}

fn label_kiri(ui: &mut Ui, t: &str) {
    ui.add_sized(vec2(84.0, 26.0), egui::Label::new(RichText::new(t).color(REDUP)));
}

fn isian(ui: &mut Ui, t: &mut String, sandi: bool, petunjuk: &str, lebar: f32) -> egui::Response {
    ui.add(
        TextEdit::singleline(t)
            .password(sandi)
            .hint_text(petunjuk)
            .desired_width(lebar)
            .min_size(vec2(0.0, 30.0))
            .vertical_align(Align::Center)
            .margin(egui::Margin::symmetric(8, 4)),
    )
}

fn bingkai() -> egui::Frame {
    egui::Frame::new()
        .fill(gaya::LATAR2)
        .stroke(egui::Stroke::new(1.0_f32, gaya::GARIS))
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(14, 10))
}

fn kepala(ui: &mut Ui, t: &str, ket: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(t).family(tebal()).color(TINTA));
        ui.label(kecil(ket, REDUP));
    });
    ui.add_space(4.0);
}

/// Server, port, keamanan, pengguna, sandi.
fn isian_server(ui: &mut Ui, id: &str, s: &mut Server, contoh_host: &str, contoh_pengguna: &str) {
    ui.horizontal(|ui| {
        label_kiri(ui, "Server");
        isian(ui, &mut s.host, false, contoh_host, 220.0);
        ui.label(RichText::new("Port").color(REDUP));
        isian(ui, &mut s.port, false, "993", 52.0);
        let lama = s.aman;
        ComboBox::from_id_salt(format!("aman-{id}")).width(118.0).selected_text(AMAN[s.aman.min(2)].1).show_ui(ui, |ui| {
            for (i, (_, n)) in AMAN.iter().enumerate() {
                ui.selectable_value(&mut s.aman, i, *n);
            }
        });
        // Port bawaan ikut berganti bila pengguna belum mengubahnya.
        if s.aman != lama && matches!(s.port.trim(), "993" | "143" | "") {
            s.port = if s.aman == 0 { "993".into() } else { "143".into() };
        }
    });
    ui.horizontal(|ui| {
        label_kiri(ui, "Pengguna");
        isian(ui, &mut s.pengguna, false, contoh_pengguna, f32::INFINITY);
    });
    ui.horizontal(|ui| {
        label_kiri(ui, "Kata sandi");
        isian(ui, &mut s.sandi, true, "", f32::INFINITY);
    });
}

fn form(ui: &mut Ui, s: &mut Email) {
    bingkai().show(ui, |ui| {
        ui.set_width(ui.available_width());
        kepala(ui, "Dari", "akun email lama");
        ui.horizontal(|ui| {
            label_kiri(ui, "Layanan");
            ComboBox::from_id_salt("email-layanan").width(260.0).selected_text(LAYANAN[s.layanan.min(2)]).show_ui(ui, |ui| {
                for (i, n) in LAYANAN.iter().enumerate() {
                    ui.selectable_value(&mut s.layanan, i, *n);
                }
            });
        });
        match s.layanan {
            0 => {
                ui.horizontal(|ui| {
                    label_kiri(ui, "Email");
                    isian(ui, &mut s.email, false, "nama@gmail.com", f32::INFINITY);
                });
                ui.horizontal(|ui| {
                    label_kiri(ui, "Masuk dengan");
                    ui.radio_value(&mut s.login_google, false, "Sandi aplikasi");
                    ui.add_enabled_ui(s.ada_client_google, |ui| {
                        ui.radio_value(&mut s.login_google, true, "Login Google di browser")
                            .on_disabled_hover_text("Butuh client ID Google (Pengaturan > Client ID Google).");
                    });
                });
                if s.login_google {
                    ui.horizontal(|ui| {
                        label_kiri(ui, "");
                        ui.label(kecil("Browser dibuka saat migrasi dimulai. Memakai client ID Google di Pengaturan.", REDUP));
                    });
                } else {
                    ui.horizontal(|ui| {
                        label_kiri(ui, "Sandi aplikasi");
                        isian(ui, &mut s.sandi_aplikasi, true, "16 huruf dari Google", 220.0);
                        if ui.link(kecil("Buat sandi aplikasi", TAUTAN)).on_hover_text(SANDI_APLIKASI_GOOGLE).clicked() {
                            win::buka(SANDI_APLIKASI_GOOGLE);
                        }
                    });
                    ui.horizontal(|ui| {
                        label_kiri(ui, "");
                        ui.label(kecil("Gmail tidak menerima kata sandi biasa. Sandi aplikasi butuh verifikasi 2 langkah aktif.", REDUP));
                    });
                }
            }
            1 => {
                ui.horizontal(|ui| {
                    label_kiri(ui, "Email");
                    isian(ui, &mut s.email, false, "nama@outlook.com / nama@kantor.onmicrosoft.com", f32::INFINITY);
                });
                if s.client_ms_ada {
                    ui.horizontal(|ui| {
                        label_kiri(ui, "");
                        ui.label(kecil("Login Microsoft di browser saat migrasi dimulai (client ID Azure tersimpan).", REDUP));
                    });
                } else {
                    ui.horizontal(|ui| {
                        label_kiri(ui, "Client ID");
                        isian(ui, &mut s.client_ms, false, "Application (client) ID dari Azure", 300.0);
                        if ui.link(kecil("Panduan", TAUTAN)).on_hover_text(PANDUAN_AZURE).clicked() {
                            win::buka(PANDUAN_AZURE);
                        }
                    });
                    ui.horizontal(|ui| {
                        label_kiri(ui, "");
                        ui.label(kecil("Microsoft tidak lagi menerima kata sandi untuk IMAP; login lewat aplikasi Azure milik Anda.", REDUP));
                    });
                }
            }
            _ => isian_server(ui, "sumber", &mut s.sumber, "imap.server-lama.com", "nama@domain-lama.com"),
        }
    });
    ui.add_space(8.0);
    bingkai().show(ui, |ui| {
        ui.set_width(ui.available_width());
        kepala(ui, "Ke", "server email baru (IMAP)");
        isian_server(ui, "tujuan", &mut s.tujuan, "mail.kantor.go.id", "nama@kantor.go.id");
    });
    ui.add_space(8.0);
    ui.checkbox(&mut s.sertakan_sampah, "Sertakan Sampah dan Spam");
    if s.layanan == 0 {
        ui.checkbox(&mut s.arsip, "Salin email arsip Gmail (tanpa label) ke folder Archive")
            .on_hover_text("Email yang diarsipkan di Gmail hanya ada di \"Semua Email\". Tanpa ini, email itu tidak ikut pindah.");
    }
    ui.checkbox(&mut s.abaikan_sertifikat, "Terima sertifikat swatanda")
        .on_hover_text("Untuk server email di jaringan kantor yang sertifikat TLS-nya dibuat sendiri. Biarkan mati untuk server di internet.");
}

fn progres(ui: &mut Ui, s: &Email, periksa: bool) {
    bingkai().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.add(egui::Spinner::new().size(14.0).color(TAUTAN));
            ui.label(RichText::new(if periksa { "Memeriksa kedua server" } else { "Menyalin email" }).family(tebal()).color(TINTA));
        });
        ui.add_space(6.0);
        let p = s.progres.as_ref();
        let n = |k: &str| p.and_then(|v| v.get(k)).and_then(Value::as_u64).unwrap_or(0);
        if !periksa && n("Total") > 0 {
            let frak = n("Selesai") as f32 / n("Total").max(1) as f32;
            ui.add(egui::ProgressBar::new(frak.min(1.0)).desired_height(10.0).fill(TAUTAN));
            ui.add_space(4.0);
            ui.label(kecil(
                &format!(
                    "Folder {} dari {} {TITIK} {} {TITIK} {} dari {} email diproses {TITIK} {} disalin ({})",
                    n("FolderKe"),
                    n("FolderJumlah"),
                    p.and_then(|v| v.get("Folder")).and_then(Value::as_str).unwrap_or(""),
                    angka(n("Selesai") as i64),
                    angka(n("Total") as i64),
                    angka(n("Disalin") as i64),
                    format_ukuran(n("Byte") as f64)
                ),
                TINTA,
            ));
        }
        if !s.baris.is_empty() {
            ui.add(egui::Label::new(kecil(&s.baris, REDUP)).truncate());
        }
    });
    ui.add_space(6.0);
    ui.label(kecil(
        "Boleh ditutup: migrasi tetap berjalan selama LELEMOVE terbuka. Bila terhenti, jalankan lagi; email yang sudah tersalin dilewati.",
        REDUP,
    ));
}

fn tabel_hasil(ui: &mut Ui, s: &Email, periksa: bool) {
    let h = s.hasil.as_ref();
    let folder: Vec<&Value> = h.and_then(|v| v.get("Folder")).and_then(Value::as_array).map(|a| a.iter().collect()).unwrap_or_default();
    let n = |v: &Value, k: &str| v.get(k).and_then(Value::as_i64).unwrap_or(0);
    if folder.is_empty() {
        let g = h.map(|v| teks(v.get("Galat"))).unwrap_or_default();
        if !g.is_empty() {
            ui.label(RichText::new(g).color(MERAH));
        }
        return;
    }
    ScrollArea::vertical().max_height(260.0).auto_shrink([false, true]).show(ui, |ui| {
        egui::Grid::new("email-hasil").num_columns(5).striped(true).spacing(vec2(14.0, 5.0)).show(ui, |ui| {
            sel_kiri(ui, RichText::new("Folder").family(tebal()).size(12.5).color(REDUP));
            for t in ["Email", "Sudah ada", if periksa { "Akan disalin" } else { "Disalin" }, "Gagal"] {
                ui.label(RichText::new(t).family(tebal()).size(12.5).color(REDUP));
            }
            ui.end_row();
            for f in &folder {
                let lewati = teks(f.get("Lewati"));
                let nama = format!("{} {PANAH} {}", teks(f.get("Sumber")), teks(f.get("Tujuan")));
                sel_kiri(ui, RichText::new(&nama).size(12.5).color(if lewati.is_empty() { TINTA } else { REDUP })).on_hover_text(&nama);
                if !lewati.is_empty() {
                    ui.label(kecil("dilewati", REDUP)).on_hover_text(&lewati);
                    ui.label("");
                    ui.label("");
                    ui.label("");
                } else {
                    let akan = n(f, "Email") - n(f, "SudahAda");
                    ui.label(RichText::new(angka(n(f, "Email"))).size(12.5).color(TINTA));
                    ui.label(RichText::new(angka(n(f, "SudahAda"))).size(12.5).color(TINTA));
                    ui.label(RichText::new(angka(if periksa { akan } else { n(f, "Disalin") })).size(12.5).color(TINTA));
                    let g = n(f, "Gagal");
                    ui.label(RichText::new(angka(g)).size(12.5).color(if g > 0 { MERAH } else { TINTA }));
                }
                ui.end_row();
            }
        });
    });
}

/// Sel kolom Folder: lebar tetap, rata kiri, terpotong dengan elipsis (label terpotong di Grid
/// hanya mengambil lebar minimum).
fn sel_kiri(ui: &mut Ui, t: RichText) -> egui::Response {
    ui.allocate_ui_with_layout(vec2(300.0, 20.0), Layout::left_to_right(Align::Center), |ui| {
        ui.set_min_width(300.0);
        ui.add(egui::Label::new(t).truncate())
    })
    .inner
}

fn ringkasan(s: &Email, periksa: bool, kode: i32) -> (String, Color32) {
    let h = s.hasil.as_ref();
    let n = |k: &str| h.and_then(|v| v.get(k)).and_then(Value::as_i64).unwrap_or(0);
    let galat = h.map(|v| teks(v.get("Galat"))).unwrap_or_default();
    if kode == -1 {
        return ("Dihentikan. Jalankan lagi untuk melanjutkan; email yang sudah tersalin dilewati.".into(), KUNING);
    }
    if !galat.is_empty() {
        return (galat, MERAH);
    }
    if periksa {
        return (
            format!("{} email belum ada di server baru ({}). Klik Mulai migrasi untuk menyalinnya.", angka(n("Baru")), format_ukuran(n("Byte") as f64)),
            if kode == 0 { HIJAU } else { MERAH },
        );
    }
    if kode == 0 {
        (format!("Selesai: {} email disalin ({}), {} sudah ada sebelumnya.", angka(n("Disalin")), format_ukuran(n("Byte") as f64), angka(n("SudahAda"))), HIJAU)
    } else {
        (
            format!("{} email disalin, {} gagal. Jalankan lagi untuk mencoba yang gagal; detail di log aktivitas.", angka(n("Disalin")), angka(n("Gagal"))),
            KUNING,
        )
    }
}

/// Dialog Migrasi email.
pub fn tampil(ui: &mut Ui, s: &mut Email) -> Option<Hasil> {
    let sibuk = s.sibuk;
    ui.set_width(620.0);
    judul(ui, "Migrasi email");
    ui.label(kecil(
        "Salin semua folder email beserta status baca, bintang, dan tanggal aslinya ke server email Anda. Email di akun lama tidak diubah atau dihapus.",
        REDUP,
    ));
    ui.add_space(10.0);
    let tinggi = (ui.ctx().screen_rect().height() - 230.0).max(260.0);
    let mut h = None;
    match s.tahap {
        Tahap::Isi => {
            ScrollArea::vertical().max_height(tinggi).auto_shrink([false, true]).show(ui, |ui| form(ui, s));
            if let Some((t, w)) = &s.pesan {
                ui.add_space(6.0);
                ui.label(RichText::new(t).color(*w));
            }
            ui.add_space(14.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let mut mulai = None;
                if ui.add_enabled(!sibuk, tombol_utama("Mulai migrasi")).clicked() {
                    mulai = Some(false);
                }
                if ui.add_enabled(!sibuk, tombol("Periksa")).on_hover_text("Uji login kedua server dan hitung email yang akan disalin.").clicked() {
                    mulai = Some(true);
                }
                if ui.add(tombol("Tutup")).clicked() {
                    h = Some(Hasil::Batal);
                }
                if sibuk {
                    ui.label(kecil("Tunggu aksi lain selesai dulu.", KUNING));
                }
                if let Some(periksa) = mulai {
                    match s.perintah(periksa) {
                        Ok((args, rahasia)) => h = Some(Hasil::EmailMulai { periksa, args, rahasia }),
                        Err(e) => s.pesan = Some((e, MERAH)),
                    }
                }
            });
        }
        Tahap::Jalan { periksa } => {
            progres(ui, s, periksa);
            ui.add_space(14.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.add(tombol("Hentikan")).clicked() {
                    h = Some(Hasil::EmailHenti);
                }
                if ui.add(tombol_utama("Tutup")).on_hover_text("Migrasi tetap berjalan; buka lagi dari Migrasi email.").clicked() {
                    h = Some(Hasil::Batal);
                }
            });
        }
        Tahap::Selesai { periksa, kode } => {
            let (t, w) = ringkasan(s, periksa, kode);
            ui.label(RichText::new(t).color(w));
            ui.add_space(8.0);
            tabel_hasil(ui, s, periksa);
            ui.add_space(14.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let label = if periksa { "Mulai migrasi" } else { "Jalankan lagi" };
                if ui.add_enabled(!sibuk, tombol_utama(label)).clicked() {
                    match s.perintah(false) {
                        Ok((args, rahasia)) => h = Some(Hasil::EmailMulai { periksa: false, args, rahasia }),
                        Err(e) => {
                            s.tahap = Tahap::Isi;
                            s.pesan = Some((e, MERAH));
                        }
                    }
                }
                if ui.add(tombol("Ubah isian")).clicked() {
                    s.tahap = Tahap::Isi;
                    s.pesan = None;
                }
                if ui.add(tombol("Tutup")).clicked() {
                    h = Some(Hasil::Batal);
                }
            });
        }
    }
    h
}

/// Isi `progres` / `baris` dari proses yang berjalan.
pub fn terima_progres(s: &mut Email, v: Value) {
    s.progres = Some(v);
}

pub fn terima_baris(s: &mut Email, b: &str) {
    let t = b.trim();
    if !t.is_empty() && !t.starts_with("==") && !t.starts_with("##") {
        s.baris = t.trim_start_matches(['[', ']']).to_string();
    }
}

/// Contoh hasil untuk pratinjau dokumentasi.
pub fn contoh_hasil() -> Value {
    json!({
        "Disalin": 1288, "SudahAda": 12, "Gagal": 0, "Byte": 734_003_200,
        "Folder": [
            {"Sumber": "INBOX", "Tujuan": "INBOX", "Email": 1004, "SudahAda": 12, "Disalin": 992, "Gagal": 0, "Lewati": ""},
            {"Sumber": "[Gmail]/Sent Mail", "Tujuan": "INBOX.Sent", "Email": 231, "SudahAda": 0, "Disalin": 231, "Gagal": 0, "Lewati": ""},
            {"Sumber": "Kantor/Laporan", "Tujuan": "INBOX.Kantor.Laporan", "Email": 48, "SudahAda": 0, "Disalin": 48, "Gagal": 0, "Lewati": ""},
            {"Sumber": "[Gmail]/All Mail", "Tujuan": "INBOX.Archive", "Email": 17, "SudahAda": 0, "Disalin": 17, "Gagal": 0, "Lewati": ""},
            {"Sumber": "[Gmail]/Spam", "Tujuan": "INBOX.Junk", "Email": 0, "SudahAda": 0, "Disalin": 0, "Gagal": 0, "Lewati": "sampah/spam tidak disertakan"}
        ]
    })
}

#[cfg(test)]
mod uji {
    use super::*;

    #[test]
    fn perintah_dari_isian() {
        let mut e = Email::baru(&Setelan::default(), false, false);
        assert!(e.perintah(false).unwrap_err().contains("alamat email lama"));
        e.email = "nama@gmail.com".into();
        e.sandi_aplikasi = "abcd efgh ijkl mnop".into();
        e.tujuan = Server { host: " mail.kantor.go.id ".into(), port: "993".into(), aman: 0, pengguna: "nama@kantor.go.id".into(), sandi: "s\"andi".into() };
        let (args, rahasia) = e.perintah(true).unwrap();
        assert_eq!(
            args,
            ["email", "-Login", "sandi", "-Sumber", "gmail:nama@gmail.com", "-Tujuan", "imap://nama@kantor.go.id@mail.kantor.go.id:993", "-RahasiaStdin", "-Ya", "-HanyaPeriksa"]
        );
        // Sandi aplikasi tanpa spasi; tanda kutip di-escape oleh JSON.
        let j: Value = serde_json::from_str(&rahasia).unwrap();
        assert_eq!(j["Sumber"], "abcdefghijklmnop");
        assert_eq!(j["Tujuan"], "s\"andi");
        // Outlook tanpa client ID tersimpan: wajib diisi dan ikut dikirim.
        e.layanan = 1;
        assert!(e.perintah(false).unwrap_err().contains("client"));
        e.client_ms = "0c3f1a2b-1234-4abc-9def-0123456789ab".into();
        let (args, rahasia) = e.perintah(false).unwrap();
        assert!(args.windows(2).any(|w| w == ["-ClientMicrosoft", "0c3f1a2b-1234-4abc-9def-0123456789ab"]));
        assert!(args.windows(2).any(|w| w == ["-Sumber", "outlook:nama@gmail.com"]));
        assert!(serde_json::from_str::<Value>(&rahasia).unwrap().get("Sumber").is_none());
        // Server IMAP lain dengan STARTTLS.
        e.layanan = 2;
        e.sumber = Server { host: "imap.lama.id".into(), port: "143".into(), aman: 1, pengguna: "budi".into(), sandi: "x".into() };
        e.abaikan_sertifikat = true;
        let (args, _) = e.perintah(false).unwrap();
        assert!(args.windows(2).any(|w| w == ["-Sumber", "imap+starttls://budi@imap.lama.id:143"]));
        assert!(args.contains(&"-AbaikanSertifikat".to_string()) && !args.contains(&"-TanpaArsip".to_string()));
        e.tujuan.port = "abc".into();
        assert!(e.perintah(false).unwrap_err().contains("Port"));
    }

    #[test]
    fn isian_dari_setelan() {
        let s = Setelan { sumber: "imap+starttls://budi@imap.lama.id:143".into(), tujuan: "imap://n@x.id@mail.x.id".into(), ..Default::default() };
        let e = Email::baru(&s, true, false);
        assert_eq!((e.layanan, e.sumber.host.as_str(), e.sumber.aman, e.sumber.port.as_str()), (2, "imap.lama.id", 1, "143"));
        assert_eq!((e.tujuan.pengguna.as_str(), e.tujuan.host.as_str()), ("n@x.id", "mail.x.id"));
        assert!(e.arsip && !e.login_google);
    }
}