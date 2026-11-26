//! Menambah akun selain Google Drive/OneDrive/SharePoint: pemilih layanan, isian kredensial,
//! pertanyaan lanjutan dari rclone (mis. kode 2FA), dan memakai remote rclone yang sudah ada.
//! Daftar layanannya ada di mesin::penyedia; dialog di sini hanya mengumpulkan isian. Login
//! dijalankan pindah.exe (`hubungkan`), dengan isian rahasia dikirim lewat stdin.

use eframe::egui::{self, vec2, Align, ComboBox, Key, Layout, RichText, ScrollArea, TextEdit, Ui};

use pindah_drive::gaya::{self, kecil, tebal, tombol, tombol_utama, KUNING, MERAH, REDUP, TAUTAN, TINTA};
use pindah_drive::mesin::penyedia::{self, JenisKolom, Kelompok, Penyedia};

use crate::dialog::Hasil;

// ------------------------------------------------------------------ pemilih layanan

pub struct Pemilih {
    pub cari: String,
    pub fokus: bool,
}

impl Pemilih {
    pub fn baru() -> Self {
        Pemilih { cari: String::new(), fokus: true }
    }
}

fn cocok_cari(p: &Penyedia, kata: &str) -> bool {
    if kata.is_empty() {
        return true;
    }
    let k = kata.to_lowercase();
    [p.nama, p.ket, p.backend, p.kunci].iter().any(|t| t.to_lowercase().contains(&k))
}

/// Satu kartu layanan: petak ikon, nama, keterangan singkat.
fn kartu(ui: &mut Ui, p: &Penyedia, lebar: f32) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(lebar, 52.0), egui::Sense::click());
    pindah_drive::gaya::nama_tombol(&resp, true, p.nama);
    if ui.is_rect_visible(r) {
        let pn = ui.painter();
        let latar = if resp.hovered() { gaya::HOVER } else { gaya::PUTIH };
        pn.rect_filled(r, 8.0, latar);
        pn.rect_stroke(r, 8.0, egui::Stroke::new(1.0_f32, gaya::GARIS), egui::StrokeKind::Inside);
        let rp = egui::Rect::from_center_size(egui::pos2(r.left() + 26.0, r.center().y), vec2(30.0, 30.0));
        gaya::petak_lukis(pn, rp, p.kunci, gaya::ik::AWAN, None, latar);
        let x = rp.right() + 10.0;
        let mut nama = p.nama.to_string();
        if p.beta {
            nama += "  \u{00B7} beta";
        }
        let g = gaya::potong(ui, &nama, egui::FontId::new(13.5, tebal()), TINTA, r.right() - x - 8.0);
        pn.galley(egui::pos2(x, r.top() + 8.0), g, TINTA);
        let ket = if p.oauth { "Login di browser".to_string() } else { p.ket.to_string() };
        let g = gaya::potong(ui, &ket, egui::FontId::proportional(11.5), REDUP, r.right() - x - 8.0);
        pn.galley(egui::pos2(x, r.top() + 28.0), g, REDUP);
    }
    resp.on_hover_text(p.ket).on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn pemilih(ui: &mut Ui, s: &mut Pemilih) -> Option<Hasil> {
    ui.set_width(640.0);
    let mut h = None;
    ui.label(RichText::new("Tambah akun").family(tebal()).size(16.5).color(TINTA));
    ui.add_space(2.0);
    ui.label(RichText::new("Pilih layanan penyimpanan. Akun yang ditambahkan bisa dipakai sebagai sumber maupun tujuan.").color(REDUP));
    ui.add_space(8.0);
    let r = ui.add(
        TextEdit::singleline(&mut s.cari)
            .hint_text("Cari layanan, mis. dropbox, nas, s3")
            .desired_width(f32::INFINITY)
            .min_size(vec2(0.0, 30.0))
            .vertical_align(Align::Center)
            .margin(pindah_drive::gaya::JARAK_ISIAN),
    );
    if s.fokus {
        r.request_focus();
        s.fokus = false;
    }
    ui.add_space(8.0);
    let kata = s.cari.trim().to_string();
    ScrollArea::vertical().max_height(430.0).auto_shrink([false, true]).show(ui, |ui| {
        ui.set_width(ui.available_width());
        let lebar = ((ui.available_width() - 8.0) / 2.0).floor();
        let mut ada = false;
        for kel in [Kelompok::Cloud, Kelompok::Server, Kelompok::Objek] {
            let daftar: Vec<&Penyedia> = penyedia::SEMUA.iter().filter(|p| p.kelompok == kel && cocok_cari(p, &kata)).collect();
            if daftar.is_empty() {
                continue;
            }
            ada = true;
            ui.add_space(4.0);
            ui.label(gaya::label_kelompok(&kel.judul().to_uppercase(), REDUP));
            ui.add_space(2.0);
            for pasangan in daftar.chunks(2) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    for p in pasangan {
                        if kartu(ui, p, lebar).clicked() {
                            h = Some(Hasil::PilihPenyedia(p.kunci.to_string()));
                        }
                    }
                });
                ui.add_space(6.0);
            }
        }
        if !ada {
            ui.label(RichText::new("Tidak ada layanan yang cocok. Coba \"Lanjutan\" di bawah: rclone mendukung lebih dari 70 layanan.").color(REDUP));
        }
        ui.add_space(4.0);
        ui.label(gaya::label_kelompok("LANJUTAN", REDUP));
        ui.add_space(2.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 14.0;
            if ui
                .link(RichText::new("Pakai remote rclone yang sudah ada\u{2026}").color(TAUTAN))
                .on_hover_text("Remote yang dibuat sendiri dengan rclone (di luar LELEMOVE).")
                .clicked()
            {
                h = Some(Hasil::BukaImpor);
            }
            if ui
                .link(RichText::new("Buat lewat rclone config (semua layanan)\u{2026}").color(TAUTAN))
                .on_hover_text("Membuka jendela rclone config untuk layanan yang tidak ada di daftar ini. Sesudahnya, pakai lewat \"remote rclone yang sudah ada\".")
                .clicked()
            {
                h = Some(Hasil::RcloneConfig);
            }
        });
    });
    ui.add_space(12.0);
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        if ui.add(tombol("Batal").min_size(vec2(0.0, 38.0))).clicked() {
            h = Some(Hasil::Batal);
        }
    });
    h
}

// ------------------------------------------------------------------ isian kredensial

pub struct Kredensial {
    pub kunci: String,
    /// Akun yang dilogin ulang (None = akun baru).
    pub akun: Option<String>,
    pub label: String,
    /// Nilai setiap kolom penyedia (urutan sama dengan `Penyedia::kolom`).
    pub nilai: Vec<String>,
    pub galat: Option<String>,
    pub fokus: bool,
}

impl Kredensial {
    pub fn baru(p: &Penyedia, akun: Option<String>, label: String) -> Self {
        Kredensial {
            kunci: p.kunci.to_string(),
            akun,
            label,
            nilai: p.kolom.iter().map(|k| k.bawaan.to_string()).collect(),
            galat: None,
            fokus: true,
        }
    }
}

fn isian(ui: &mut Ui, teks: &mut String, contoh: &str, sandi: bool) -> egui::Response {
    ui.add(
        TextEdit::singleline(teks)
            .password(sandi)
            .hint_text(contoh)
            .desired_width(f32::INFINITY)
            .min_size(vec2(0.0, 30.0))
            .vertical_align(Align::Center)
            .margin(pindah_drive::gaya::JARAK_ISIAN),
    )
}

pub fn kredensial(ui: &mut Ui, s: &mut Kredensial) -> Option<Hasil> {
    ui.set_width(540.0);
    let Some(p) = penyedia::cari(&s.kunci) else { return Some(Hasil::Batal) };
    let mut h = None;
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(30.0, 30.0), egui::Sense::hover());
        gaya::petak_lukis(ui.painter(), r, p.kunci, gaya::ik::AWAN, None, ui.visuals().panel_fill);
        let judul = if s.akun.is_some() { format!("Login ulang {}", p.nama) } else { format!("Tambah {}", p.nama) };
        ui.label(RichText::new(judul).family(tebal()).size(16.5).color(TINTA));
    });
    ui.add_space(4.0);
    ui.add(egui::Label::new(RichText::new(p.ket).color(REDUP)).wrap());
    if p.beta {
        ui.add(egui::Label::new(kecil("Dukungan untuk layanan ini masih beta: coba dengan folder kecil dulu.", KUNING)).wrap());
    }
    ui.add_space(10.0);
    ui.label(RichText::new("Nama tampilan, misalnya Kantor atau Arsip").color(REDUP));
    let r = isian(ui, &mut s.label, p.nama, false);
    if s.fokus {
        r.request_focus();
        s.fokus = false;
    }
    let mut enter = r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
    for (i, k) in p.kolom.iter().enumerate() {
        ui.add_space(8.0);
        let Some(v) = s.nilai.get_mut(i) else { continue };
        match k.jenis {
            JenisKolom::Centang => {
                let mut on = v == "true";
                if ui.checkbox(&mut on, k.label).changed() {
                    *v = on.to_string();
                }
            }
            JenisKolom::Pilihan(pilihan) => {
                ui.label(RichText::new(k.label).color(REDUP));
                let teks = pilihan.iter().find(|(n, _)| n == v).map_or(v.clone(), |(_, l)| l.to_string());
                ComboBox::from_id_salt(("kolom", k.kunci)).width(300.0).selected_text(teks).show_ui(ui, |ui| {
                    for (n, l) in pilihan {
                        ui.selectable_value(v, n.to_string(), *l);
                    }
                });
            }
            _ => {
                let wajib = if k.wajib { "" } else { " (opsional)" };
                ui.label(RichText::new(format!("{}{wajib}", k.label)).color(REDUP));
                let r = isian(ui, v, k.contoh, k.tersamar());
                enter |= r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
            }
        }
        if !k.ket.is_empty() {
            ui.add(egui::Label::new(kecil(k.ket, REDUP)).wrap());
        }
    }
    ui.add_space(10.0);
    let catatan = if p.oauth {
        "Setelah ini browser terbuka untuk login. Kembali ke LELEMOVE setelah browser menampilkan \"Success!\"."
    } else {
        "Kata sandi dan kunci dikirim langsung ke rclone dan disimpan di rclone.conf di komputer ini (terobfuskasi), tidak ke tempat lain."
    };
    ui.add(egui::Label::new(kecil(catatan, REDUP)).wrap());
    if let Some(g) = &s.galat {
        ui.add_space(6.0);
        ui.add(egui::Label::new(RichText::new(g).color(MERAH)).wrap());
    }
    ui.add_space(14.0);
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        let lanjut = if p.oauth { "Lanjut ke login" } else { "Hubungkan" };
        if ui.add(tombol_utama(lanjut)).clicked() || enter {
            let kosong = p.kolom.iter().zip(&s.nilai).find(|(k, v)| k.wajib && v.trim().is_empty());
            if let Some((k, _)) = kosong {
                s.galat = Some(format!("Isi \"{}\".", k.label));
            } else {
                let mut opsi = Vec::new();
                let mut rahasia = Vec::new();
                for (k, v) in p.kolom.iter().zip(&s.nilai) {
                    if v.is_empty() {
                        continue;
                    }
                    if k.tersamar() {
                        rahasia.push((k.kunci.to_string(), v.clone()));
                    } else {
                        opsi.push((k.kunci.to_string(), v.trim().to_string()));
                    }
                }
                h = Some(Hasil::Kredensial { kunci: s.kunci.clone(), akun: s.akun.clone(), label: s.label.trim().to_string(), opsi, rahasia });
            }
        }
        if ui.add(tombol("Kembali").min_size(vec2(0.0, 38.0))).clicked() {
            h = Some(if s.akun.is_some() { Hasil::Batal } else { Hasil::KembaliPemilih });
        }
    });
    h
}

// ------------------------------------------------------------------ pertanyaan lanjutan rclone

pub struct TanyaKonfig {
    pub akun: String,
    pub layanan: String,
    pub state: String,
    pub bantuan: String,
    pub rahasia: bool,
    pub pilihan: Vec<(String, String)>,
    pub jawab: String,
    pub baru: bool,
    pub galat: Option<String>,
    pub fokus: bool,
}

pub fn tanya_konfig(ui: &mut Ui, s: &mut TanyaKonfig) -> Option<Hasil> {
    ui.set_width(520.0);
    let mut h = None;
    ui.label(RichText::new(format!("{}: satu langkah lagi", s.layanan)).family(tebal()).size(16.5).color(TINTA));
    ui.add_space(4.0);
    ui.label(kecil("Pertanyaan dari rclone untuk layanan ini:", REDUP));
    ui.add_space(2.0);
    // Teks bantuan dari rclone (bahasa Inggris); baris pertama adalah pertanyaannya.
    ui.add(egui::Label::new(RichText::new(s.bantuan.trim()).color(TINTA)).wrap());
    if let Some(g) = &s.galat {
        ui.add_space(4.0);
        ui.add(egui::Label::new(RichText::new(format!("Jawaban sebelumnya ditolak: {g}")).color(MERAH)).wrap());
    }
    ui.add_space(10.0);
    let mut enter = false;
    if s.pilihan.is_empty() {
        let r = isian(ui, &mut s.jawab, "", s.rahasia);
        if s.fokus {
            r.request_focus();
            s.fokus = false;
        }
        enter = r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
    } else {
        ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
            for (n, ket) in &s.pilihan {
                let judul = ket.lines().next().unwrap_or(n);
                let r = ui.radio_value(&mut s.jawab, n.clone(), RichText::new(judul).color(TINTA));
                let sisa: String = ket.lines().skip(1).collect::<Vec<_>>().join(" ");
                if !sisa.is_empty() {
                    r.on_hover_text(sisa);
                }
            }
        });
    }
    ui.add_space(14.0);
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        if ui.add(tombol_utama("Lanjut")).clicked() || enter {
            h = Some(Hasil::JawabKonfig {
                akun: s.akun.clone(),
                state: s.state.clone(),
                jawab: s.jawab.clone(),
                rahasia: s.rahasia,
                baru: s.baru,
            });
        }
        if ui.add(tombol("Batal").min_size(vec2(0.0, 38.0))).clicked() {
            h = Some(Hasil::BatalKonfig { akun: s.akun.clone(), baru: s.baru });
        }
    });
    h
}

// ------------------------------------------------------------------ remote rclone yang sudah ada

pub struct Impor {
    /// None = sedang dimuat. (nama remote, backend)
    pub calon: Option<Vec<(String, String)>>,
    pub gen: u64,
    pub pilih: Option<usize>,
    pub label: String,
}

pub fn impor(ui: &mut Ui, s: &mut Impor) -> Option<Hasil> {
    ui.set_width(520.0);
    let mut h = None;
    ui.label(RichText::new("Pakai remote rclone yang sudah ada").family(tebal()).size(16.5).color(TINTA));
    ui.add_space(4.0);
    ui.add(
        egui::Label::new(
            RichText::new("Remote yang Anda buat sendiri dengan rclone. LELEMOVE hanya memakainya; \"Lepas\" nanti tidak menghapus remote itu dari rclone.")
                .color(REDUP),
        )
        .wrap(),
    );
    ui.add_space(8.0);
    match &s.calon {
        None => {
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new().size(12.0));
                ui.label(kecil("Membaca konfigurasi rclone\u{2026}", REDUP));
            });
        }
        Some(c) if c.is_empty() => {
            ui.label(RichText::new("Belum ada remote rclone di luar LELEMOVE.").color(REDUP));
        }
        Some(c) => {
            egui::Frame::new()
                .stroke(egui::Stroke::new(1.0_f32, gaya::GARIS))
                .corner_radius(6)
                .inner_margin(egui::Margin::same(4))
                .show(ui, |ui| {
                    ScrollArea::vertical().max_height(240.0).auto_shrink([false, true]).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        for (i, (n, b)) in c.iter().enumerate() {
                            let nama = penyedia::dari_konfig(b, "", "").map_or(b.as_str(), |p| p.nama);
                            if ui.selectable_label(s.pilih == Some(i), format!("{n}    ({nama})")).clicked() {
                                s.pilih = Some(i);
                                if s.label.is_empty() {
                                    s.label = n.clone();
                                }
                            }
                        }
                    });
                });
            ui.add_space(8.0);
            ui.label(RichText::new("Nama tampilan").color(REDUP));
            isian(ui, &mut s.label, "", false);
        }
    }
    ui.add_space(14.0);
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        let terpilih = s.pilih.and_then(|i| s.calon.as_ref().and_then(|c| c.get(i)));
        if ui.add_enabled(terpilih.is_some(), tombol_utama("Pakai")).clicked() {
            if let Some((n, _)) = terpilih {
                h = Some(Hasil::Impor { nama: n.clone(), label: s.label.trim().to_string() });
            }
        }
        if ui.add(tombol("Kembali").min_size(vec2(0.0, 38.0))).clicked() {
            h = Some(Hasil::KembaliPemilih);
        }
    });
    h
}
