//! Dialog "Jadwal backup": daftar backup terjadwal dan isian jadwal baru/ubah.
//!
//! Dialog hanya menampilkan dan mengumpulkan isian. Perubahan (pasang/ubah/hapus tugas Windows)
//! dijalankan oleh `pindah.exe jadwal ...` lewat aksi biasa, supaya jendela tidak membeku saat
//! schtasks berjalan dan hasilnya tercatat di log aktivitas.

use eframe::egui::{self, vec2, Align, Color32, ComboBox, Layout, RichText, ScrollArea, Ui};

use pindah_drive::gaya::{self, kecil, tebal, tombol, tombol_utama, HIJAU, KUNING, MERAH, REDUP, TINTA};
use pindah_drive::mesin::dasar::waktu_tampil;
use pindah_drive::mesin::jadwal::{self as mj, Filter, Jadwal, HARI_PENDEK, URUTAN_HARI};

use pindah_drive::win;
use crate::dialog::Hasil;

const ELIPSIS: &str = "\u{2026}";
const TITIK: &str = "\u{00B7}";
const PANAH: &str = "\u{2192}";

/// Sumber dan tujuan: jalur mentah (untuk pindah.exe) dan nama yang mudah dibaca.
#[derive(Clone, PartialEq)]
pub struct Rute {
    pub sumber: String,
    pub tujuan: String,
    pub nama_sumber: String,
    pub nama_tujuan: String,
}

/// Satu jadwal di daftar.
pub struct Baris {
    pub j: Jadwal,
    /// Tugas Windows-nya masih terdaftar di Task Scheduler.
    pub tugas: bool,
    /// pindah.exe untuk jadwal ini sedang berjalan (menyalin atau memverifikasi).
    pub hidup: bool,
    pub rute: Rute,
}

pub struct Daftar {
    /// None = sedang dimuat.
    pub baris: Option<Vec<Baris>>,
    pub gen: u64,
    pub pesan: Option<(String, Color32)>,
    /// Ada aksi yang sedang berjalan: tombol yang mengubah sesuatu dinonaktifkan.
    pub sibuk: bool,
    /// Ada penyalinan yang sedang berjalan: "Jalankan sekarang" dinonaktifkan.
    pub job_jalan: bool,
}

pub struct Isian {
    /// None = jadwal baru.
    pub id: Option<String>,
    pub nama: String,
    pub rute: Option<Rute>,
    /// Rute yang sedang dibuka di panel Dari/Ke (bila beda dengan `rute`, bisa dipakai).
    pub rute_kini: Option<Rute>,
    pub jam: u8,
    pub menit: u8,
    /// Indeks 0 = Minggu ... 6 = Sabtu.
    pub hari: [bool; 7],
    pub verifikasi: bool,
    pub galat: Option<String>,
    /// Filter milik jadwal ini, dan filter panel Dari saat ini (ikut dipakai bersama rute_kini).
    pub filter: Filter,
    pub filter_kini: Filter,
}

impl Isian {
    pub fn baru(rute_kini: Option<Rute>, nama: String, filter_kini: Filter) -> Self {
        Isian {
            id: None,
            nama,
            rute: rute_kini.clone(),
            rute_kini,
            jam: 22,
            menit: 0,
            hari: [true; 7],
            verifikasi: true,
            galat: None,
            filter: filter_kini.clone(),
            filter_kini,
        }
    }

    pub fn dari(j: &Jadwal, rute: Rute, rute_kini: Option<Rute>, filter_kini: Filter) -> Self {
        let (jam, menit) = mj::urai_jam(&j.jam).unwrap_or((22, 0));
        let mut hari = [j.hari.is_empty(); 7];
        for h in &j.hari {
            hari[*h as usize % 7] = true;
        }
        Isian {
            id: Some(j.id.clone()),
            nama: j.nama.clone(),
            rute: Some(rute),
            rute_kini,
            jam,
            menit,
            hari,
            verifikasi: j.verifikasi,
            galat: None,
            filter: j.filter.clone(),
            filter_kini,
        }
    }

    /// Nilai -Hari untuk pindah.exe: "setiap" atau "sen,rab,jum".
    fn arg_hari(&self) -> String {
        if self.hari.iter().all(|h| *h) {
            return "setiap".into();
        }
        URUTAN_HARI
            .iter()
            .filter(|h| self.hari[**h as usize])
            .map(|h| HARI_PENDEK[*h as usize].to_lowercase())
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Jadwal sementara untuk uraian & "berikutnya" yang tampil langsung saat isian diubah.
    fn contoh(&self) -> Jadwal {
        Jadwal {
            id: String::new(),
            nama: String::new(),
            sumber: String::new(),
            tujuan: String::new(),
            jam: format!("{:02}:{:02}", self.jam, self.menit),
            hari: if self.hari.iter().all(|h| *h) {
                Vec::new()
            } else {
                (0..7u8).filter(|h| self.hari[*h as usize]).collect()
            },
            verifikasi: self.verifikasi,
            aktif: true,
            terakhir: None,
            filter: self.filter.clone(),
        }
    }
}

fn judul(ui: &mut Ui, t: &str) {
    ui.label(RichText::new(t).family(tebal()).size(16.5).color(TINTA));
}

fn titik(ui: &mut Ui, warna: Color32) {
    let (r, _) = ui.allocate_exact_size(vec2(8.0, 8.0), egui::Sense::hover());
    ui.painter().circle_filled(r.center(), 4.0, warna);
}

fn bingkai() -> egui::Frame {
    egui::Frame::new()
        .fill(gaya::LATAR2)
        .stroke(egui::Stroke::new(1.0_f32, gaya::GARIS))
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(14, 10))
}

/// "Terakhir 23 Nov 22.00: selesai. 12 file baru (1,20 GB)."
/// `hidup`: proses pindah.exe jadwal ini masih jalan; bila tidak, "berjalan" yang tertinggal berarti terputus.
fn terakhir(ui: &mut Ui, j: &Jadwal, hidup: bool) -> Option<Hasil> {
    let t = j.terakhir.as_ref()?;
    let (warna, status) = match t.hasil.as_str() {
        "selesai" => (HIJAU, "selesai"),
        "selisih" => (KUNING, "ada selisih"),
        "dilewati" => (KUNING, "dilewati"),
        "berjalan" if hidup => (TINTA, "sedang berjalan"),
        "berjalan" => (KUNING, "terputus (komputer mati atau proses ditutup); dilanjutkan pada jadwal berikutnya"),
        _ => (MERAH, "gagal"),
    };
    let mut h = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.label(kecil(&format!("Terakhir {}:", waktu_tampil(&t.mulai)), REDUP));
        ui.add(egui::Label::new(kecil(status, warna)).truncate());
        if !t.log.is_empty() && std::path::Path::new(&t.log).is_file() && ui.link(kecil("Lihat log", gaya::TAUTAN)).clicked() {
            h = Some(Hasil::JadwalLog(t.log.clone()));
        }
    });
    // Keterangan bisa memuat jalur panjang tanpa spasi: satu baris terpotong, lengkapnya saat disorot.
    if !t.ket.is_empty() {
        ui.add(egui::Label::new(kecil(&t.ket, REDUP)).truncate()).on_hover_text(&t.ket);
    }
    h
}

pub fn daftar(ui: &mut Ui, d: &mut Daftar) -> Option<Hasil> {
    ui.set_width(600.0);
    let mut h = None;
    ui.horizontal(|ui| {
        judul(ui, "Jadwal backup");
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .add_enabled(!d.sibuk, tombol(&format!("Jadwal baru{ELIPSIS}")))
                .on_hover_text("Menjadwalkan folder yang sedang dibuka di panel Dari dan Ke.")
                .clicked()
            {
                h = Some(Hasil::JadwalBaru);
            }
        });
    });
    ui.add_space(2.0);
    ui.label(
        RichText::new("LELEMOVE menyalin sendiri pada hari dan jam yang Anda tentukan, tanpa perlu membuka aplikasi. Hanya file baru atau yang berubah yang disalin; tidak ada yang dihapus di tujuan.")
            .color(REDUP),
    );
    ui.add_space(10.0);

    match &d.baris {
        None => {
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new().size(12.0));
                ui.label(kecil(&format!("Memuat jadwal{ELIPSIS}"), REDUP));
            });
        }
        Some(b) if b.is_empty() => {
            bingkai().show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new("Belum ada jadwal.").color(TINTA));
                ui.label(kecil(
                    "Pilih folder di panel Dari dan Ke, lalu klik \"Jadwal baru\". Contoh: folder Dokumen ke Google Drive setiap hari pukul 22.00.",
                    REDUP,
                ));
            });
        }
        Some(b) => {
            ScrollArea::vertical().max_height(380.0).auto_shrink([false, true]).show(ui, |ui| {
                for x in b {
                    let j = &x.j;
                    bingkai().show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            let (warna, ket) = if !j.aktif {
                                (gaya::ABU, "nonaktif".to_string())
                            } else if !x.tugas {
                                (KUNING, format!("{} hilang; klik Aktifkan untuk memasang ulang", win::NAMA_TUGAS))
                            } else {
                                (HIJAU, format!("berikutnya {}", mj::berikutnya(j).unwrap_or_default()))
                            };
                            titik(ui, warna);
                            ui.add(egui::Label::new(RichText::new(&j.nama).family(tebal()).color(TINTA)).truncate());
                            ui.label(kecil(&format!("{TITIK} {}", mj::uraian(j)), REDUP));
                            ui.label(kecil(&format!("{TITIK} {ket}"), if warna == KUNING { KUNING } else { REDUP }));
                        });
                        ui.add(
                            egui::Label::new(kecil(
                                &format!("{}  {PANAH}  {}{}", x.rute.nama_sumber, x.rute.nama_tujuan, if j.verifikasi { "  (diverifikasi)" } else { "" }),
                                TINTA,
                            ))
                            .truncate(),
                        )
                        .on_hover_text(format!("{}\n{PANAH} {}", j.sumber, j.tujuan));
                        if !j.filter.kosong() {
                            ui.add(egui::Label::new(kecil(&format!("filter: {}", j.filter.uraian()), REDUP)).truncate());
                        }
                        if let Some(r) = terakhir(ui, j, x.hidup) {
                            h = Some(r);
                        }
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            let boleh_jalan = !d.sibuk && !d.job_jalan;
                            let r = ui.add_enabled(boleh_jalan, tombol("Jalankan sekarang"));
                            let r = if d.job_jalan {
                                r.on_disabled_hover_text("Tunggu penyalinan yang sedang berjalan selesai.")
                            } else {
                                r.on_hover_text("Menyalin sekarang di latar belakang; progresnya tampil di jendela utama.")
                            };
                            if r.clicked() {
                                h = Some(Hasil::JadwalJalankan(j.id.clone()));
                            }
                            if ui.add_enabled(!d.sibuk, tombol(&format!("Ubah{ELIPSIS}"))).clicked() {
                                h = Some(Hasil::JadwalUbah(j.id.clone()));
                            }
                            let t = if j.aktif && x.tugas { "Nonaktifkan" } else { "Aktifkan" };
                            if ui.add_enabled(!d.sibuk, tombol(t)).clicked() {
                                h = Some(Hasil::JadwalAktif(j.id.clone(), !(j.aktif && x.tugas)));
                            }
                            if ui.add_enabled(!d.sibuk, tombol("Hapus")).clicked() {
                                h = Some(Hasil::JadwalHapus(j.id.clone()));
                            }
                        });
                    });
                    ui.add_space(6.0);
                }
            });
        }
    }

    ui.add_space(4.0);
    ui.label(kecil(
        if cfg!(windows) {
                "Jadwal dijalankan oleh Task Scheduler Windows selama Anda masuk ke Windows. Bila komputer mati atau tidur pada jamnya, backup dijalankan begitu komputer menyala lagi. Hasilnya dikabarkan lewat notifikasi Windows."
            } else {
                "Jadwal dijalankan oleh launchd macOS selama Anda masuk ke Mac. Bila Mac tidur pada jamnya, backup dijalankan begitu Mac bangun; bila Mac mati, jadwal itu dilewati. Hasilnya dikabarkan lewat notifikasi macOS. LELEMOVE harus berada di folder Applications."
            },
        REDUP,
    ));
    if let Some((p, w)) = &d.pesan {
        ui.add_space(8.0);
        ui.label(RichText::new(p).color(*w));
    }
    ui.add_space(14.0);
    pindah_drive::gaya::baris_tombol(ui, |ui| {
        if ui.add(tombol_utama("Tutup")).clicked() {
            h = Some(Hasil::Batal);
        }
        if d.sibuk {
            ui.add(egui::Spinner::new().size(14.0));
        }
    });
    h
}

fn label_kiri(ui: &mut Ui, t: &str) {
    ui.add_sized(vec2(64.0, 24.0), egui::Label::new(RichText::new(t).color(REDUP)));
}

pub fn isian(ui: &mut Ui, s: &mut Isian) -> Option<Hasil> {
    ui.set_width(560.0);
    judul(ui, if s.id.is_some() { "Ubah jadwal" } else { "Jadwal baru" });
    ui.add_space(10.0);

    ui.horizontal(|ui| {
        label_kiri(ui, "Nama");
        ui.add(
            egui::TextEdit::singleline(&mut s.nama)
                .desired_width(f32::INFINITY)
                .min_size(vec2(0.0, 30.0))
                .vertical_align(Align::Center)
                .margin(pindah_drive::gaya::JARAK_ISIAN),
        );
    });
    ui.add_space(8.0);

    let mut pakai_kini = false;
    bingkai().show(ui, |ui| {
        ui.set_width(ui.available_width());
        match &s.rute {
            Some(r) => {
                for (k, n, mentah) in [("Dari", &r.nama_sumber, &r.sumber), ("Ke", &r.nama_tujuan, &r.tujuan)] {
                    ui.horizontal(|ui| {
                        label_kiri(ui, k);
                        ui.add(egui::Label::new(RichText::new(n).color(TINTA)).truncate()).on_hover_text(mentah);
                    });
                }
                ui.horizontal(|ui| {
                    label_kiri(ui, "Filter");
                    let t = if s.filter.kosong() { "semua file".to_string() } else { s.filter.uraian() };
                    ui.add(egui::Label::new(RichText::new(&t).color(TINTA)).truncate()).on_hover_text(&t);
                });
            }
            None => {
                ui.label(RichText::new("Belum ada folder yang dipilih.").color(KUNING));
                ui.label(kecil(
                    "Tutup dialog ini, pilih folder di panel Dari dan Ke di jendela utama, lalu buka Jadwal backup lagi.",
                    REDUP,
                ));
            }
        }
        if let Some(k) = &s.rute_kini {
            if s.rute.as_ref() != Some(k) || s.filter != s.filter_kini {
                ui.add_space(4.0);
                let mut t = format!("{}  {PANAH}  {}", k.nama_sumber, k.nama_tujuan);
                if !s.filter_kini.kosong() {
                    t += &format!("\nfilter: {}", s.filter_kini.uraian());
                }
                if ui.add(tombol("Pakai folder & filter yang sedang dibuka")).on_hover_text(t).clicked() {
                    pakai_kini = true;
                }
            }
        }
    });
    if pakai_kini {
        s.rute = s.rute_kini.clone();
        s.filter = s.filter_kini.clone();
    }
    ui.add_space(10.0);

    ui.horizontal(|ui| {
        label_kiri(ui, "Pukul");
        ComboBox::from_id_salt("jadwal-jam")
            .width(56.0)
            .selected_text(format!("{:02}", s.jam))
            .height(260.0)
            .show_ui(ui, |ui| {
                for j in 0..24u8 {
                    ui.selectable_value(&mut s.jam, j, format!("{j:02}"));
                }
            });
        ui.label(RichText::new(":").color(TINTA));
        ComboBox::from_id_salt("jadwal-menit")
            .width(56.0)
            .selected_text(format!("{:02}", s.menit))
            .height(260.0)
            .show_ui(ui, |ui| {
                for m in (0..60u8).step_by(5) {
                    ui.selectable_value(&mut s.menit, m, format!("{m:02}"));
                }
            });
    });
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        label_kiri(ui, "Hari");
        for h in URUTAN_HARI {
            let i = h as usize;
            if ui.add_sized(vec2(42.0, 28.0), egui::SelectableLabel::new(s.hari[i], HARI_PENDEK[i])).clicked() {
                s.hari[i] = !s.hari[i];
            }
        }
        ui.add_space(6.0);
        if ui.link(kecil("Setiap hari", gaya::TAUTAN)).clicked() {
            s.hari = [true; 7];
        }
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        label_kiri(ui, "");
        let c = s.contoh();
        let t = if s.hari.iter().any(|h| *h) {
            format!("{} {TITIK} berikutnya {}", mj::uraian(&c), mj::berikutnya(&c).unwrap_or_default())
        } else {
            "Pilih paling sedikit satu hari.".into()
        };
        ui.label(kecil(&t, if s.hari.iter().any(|h| *h) { TINTA } else { KUNING }));
    });
    ui.add_space(10.0);
    ui.checkbox(&mut s.verifikasi, "Verifikasi setelah menyalin");
    ui.label(kecil(
        "Mencocokkan setiap file di sumber dan tujuan sesudah penyalinan selesai. Lebih lama, tetapi Anda langsung tahu bila ada file yang tidak sampai.",
        REDUP,
    ));
    if let Some(g) = &s.galat {
        ui.add_space(6.0);
        ui.label(RichText::new(g).color(MERAH));
    }

    let mut h = None;
    ui.add_space(18.0);
    pindah_drive::gaya::baris_tombol(ui, |ui| {
        if ui.add_enabled(s.rute.is_some(), tombol_utama("Simpan jadwal")).clicked() {
            let nama = s.nama.trim().to_string();
            if nama.is_empty() {
                s.galat = Some("Beri nama jadwal ini, misalnya \"Dokumen kantor\".".into());
            } else if !s.hari.iter().any(|h| *h) {
                s.galat = Some("Pilih paling sedikit satu hari.".into());
            } else if let Some(r) = &s.rute {
                h = Some(Hasil::JadwalSimpan {
                    id: s.id.clone(),
                    nama,
                    sumber: r.sumber.clone(),
                    tujuan: r.tujuan.clone(),
                    jam: format!("{:02}:{:02}", s.jam, s.menit),
                    hari: s.arg_hari(),
                    verifikasi: s.verifikasi,
                    filter: s.filter.clone(),
                });
            }
        }
        if ui.add(tombol("Batal").min_size(vec2(0.0, 38.0))).clicked() {
            h = Some(Hasil::KembaliJadwal);
        }
    });
    h
}

#[cfg(test)]
mod uji {
    use super::*;

    #[test]
    fn arg_hari() {
        let mut s = Isian::baru(None, "x".into(), Filter::default());
        assert_eq!(s.arg_hari(), "setiap");
        s.hari = [false, true, false, true, false, true, false];
        assert_eq!(s.arg_hari(), "sen,rab,jum");
        s.hari = [true, false, false, false, false, false, true];
        assert_eq!(s.arg_hari(), "sab,min");
        assert_eq!(mj::urai_hari(&s.arg_hari()).unwrap(), vec![6, 0]);
        assert_eq!(mj::uraian(&s.contoh()), "Sab, Min pukul 22.00");
    }
}
