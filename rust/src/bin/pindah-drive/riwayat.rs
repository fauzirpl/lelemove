//! Dialog "Riwayat & laporan": semua penyalinan, verifikasi, dan pemindahan, dengan ekspor CSV
//! dan laporan HTML siap cetak (lihat mesin::riwayat). Data dibaca di utas latar.

use eframe::egui::{self, Align, Color32, Layout, RichText, ScrollArea, Ui};

use pindah_drive::gaya::{self, kecil, tebal, tombol, tombol_utama, HIJAU, KUNING, MERAH, REDUP, TAUTAN, TINTA};
use pindah_drive::mesin::dasar::waktu_tampil;
use pindah_drive::mesin::riwayat::{self as mr, Catatan};

use pindah_drive::win;
use crate::dialog::Hasil;

const TITIK: &str = "\u{00B7}";
const PANAH: &str = "\u{2192}";

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Saring {
    #[default]
    Semua,
    Salin,
    Verifikasi,
    Pindahkan,
}

impl Saring {
    fn cocok(self, c: &Catatan) -> bool {
        match self {
            Saring::Semua => true,
            Saring::Salin => c.jenis == "salin",
            Saring::Verifikasi => c.jenis == "verifikasi",
            Saring::Pindahkan => c.jenis == "pindahkan",
        }
    }
}

/// Satu catatan beserta nama tempat yang mudah dibaca.
pub struct Baris {
    pub c: Catatan,
    pub sumber: String,
    pub tujuan: String,
}

pub struct Riwayat {
    /// None = sedang dimuat.
    pub baris: Option<Vec<Baris>>,
    pub gen: u64,
    pub saring: Saring,
    pub pesan: Option<(String, Color32)>,
}

fn warna(c: &Catatan) -> Color32 {
    match c.hasil.as_str() {
        "selesai" | "cocok" => HIJAU,
        "galat" => MERAH,
        _ => KUNING,
    }
}

/// Hanya catatan yang lolos saringan (untuk ekspor sesuai yang tampil).
pub fn tersaring(r: &Riwayat) -> Vec<Catatan> {
    r.baris.iter().flatten().filter(|b| r.saring.cocok(&b.c)).map(|b| b.c.clone()).collect()
}

pub fn tampil(ui: &mut Ui, r: &mut Riwayat) -> Option<Hasil> {
    ui.set_width(660.0);
    let mut h = None;
    ui.label(RichText::new("Riwayat & laporan").family(tebal()).size(16.5).color(TINTA));
    ui.add_space(2.0);
    ui.label(
        RichText::new("Setiap penyalinan, verifikasi, dan pemindahan tercatat di sini, termasuk yang dijalankan jadwal. Laporan verifikasi memuat status setiap file, berguna sebagai bukti untuk audit.")
            .color(REDUP),
    );
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        for (s, l) in [(Saring::Semua, "Semua"), (Saring::Salin, "Salin"), (Saring::Verifikasi, "Verifikasi"), (Saring::Pindahkan, "Pindahkan")] {
            ui.selectable_value(&mut r.saring, s, l);
        }
    });
    ui.add_space(6.0);

    match &r.baris {
        None => {
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new().size(12.0));
                ui.label(kecil("Memuat riwayat\u{2026}", REDUP));
            });
        }
        Some(semua) => {
            let tampil: Vec<&Baris> = semua.iter().filter(|b| r.saring.cocok(&b.c)).collect();
            if tampil.is_empty() {
                ui.label(RichText::new("Belum ada riwayat. Riwayat dicatat sejak versi ini dipasang.").color(REDUP));
            }
            egui::Frame::new()
                .stroke(egui::Stroke::new(1.0_f32, gaya::GARIS))
                .corner_radius(6)
                .inner_margin(egui::Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    ScrollArea::vertical().max_height(380.0).auto_shrink([false, true]).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        for (i, b) in tampil.iter().enumerate() {
                            if i > 0 {
                                ui.separator();
                            }
                            let c = &b.c;
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(mr::label_jenis(c)).family(tebal()).color(TINTA));
                                ui.label(RichText::new(mr::label_hasil(c)).color(warna(c)));
                                ui.label(kecil(&format!("{TITIK} {}", waktu_tampil(&c.mulai)), REDUP));
                                if !c.jadwal.is_empty() {
                                    ui.label(kecil(&format!("{TITIK} jadwal {}", c.jadwal), REDUP));
                                }
                            });
                            ui.add(egui::Label::new(kecil(&format!("{}  {PANAH}  {}", b.sumber, b.tujuan), TINTA)).truncate())
                                .on_hover_text(format!("{}\n{PANAH} {}", c.sumber, c.tujuan));
                            let mut ket = mr::ringkas(c);
                            if !c.filter.is_empty() {
                                ket += &format!("  {TITIK}  filter: {}", c.filter);
                            }
                            if !c.ket.is_empty() && c.ket != ket && c.jenis != "salin" {
                                ket += &format!("  {TITIK}  {}", c.ket);
                            }
                            ui.add(egui::Label::new(kecil(&ket, REDUP)).truncate()).on_hover_text(&ket);
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 12.0;
                                if !c.laporan.is_empty() && std::path::Path::new(&c.laporan).is_file() {
                                    if ui.link(kecil("Laporan per file", TAUTAN)).on_hover_text(format!("Buka laporan HTML siap cetak ({} untuk PDF).", win::PINTASAN_CETAK)).clicked() {
                                        h = Some(Hasil::RiwayatLaporan(c.id.clone(), r.saring));
                                    }
                                    if ui.link(kecil("CSV per file", TAUTAN)).clicked() {
                                        h = Some(Hasil::RiwayatCsvFile(c.id.clone(), r.saring));
                                    }
                                }
                                if !c.log.is_empty() && std::path::Path::new(&c.log).is_file() && ui.link(kecil("Log", TAUTAN)).clicked() {
                                    h = Some(Hasil::RiwayatLog(c.log.clone(), r.saring));
                                }
                            });
                        }
                    });
                });
        }
    }

    if let Some((p, w)) = &r.pesan {
        ui.add_space(6.0);
        ui.add(egui::Label::new(RichText::new(p).color(*w)).wrap());
    }
    ui.add_space(14.0);
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        if ui.add(tombol_utama("Tutup")).clicked() {
            h = Some(Hasil::Batal);
        }
        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            let ada = r.baris.as_ref().is_some_and(|b| b.iter().any(|x| r.saring.cocok(&x.c)));
            if ui.add_enabled(ada, tombol("Ekspor CSV\u{2026}")).on_hover_text("Catatan yang tampil, untuk Excel.").clicked() {
                h = Some(Hasil::RiwayatEkspor { html: false, daftar: tersaring(r), saring: r.saring });
            }
            if ui
                .add_enabled(ada, tombol("Laporan HTML\u{2026}"))
                .on_hover_text(format!("Laporan siap cetak; simpan sebagai PDF lewat {} di browser.", win::PINTASAN_CETAK))
                .clicked()
            {
                h = Some(Hasil::RiwayatEkspor { html: true, daftar: tersaring(r), saring: r.saring });
            }
        });
    });
    h
}
