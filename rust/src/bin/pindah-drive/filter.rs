//! Dialog "Filter file": hanya jenis file tertentu, lewati pola tertentu, atau hanya file yang
//! diubah sejak tanggal tertentu. Disimpan di pengaturan.json dan dipakai periksa/uji/salin/
//! verifikasi/pindahkan (lihat mesin::filter).

use eframe::egui::{vec2, Align, Layout, RichText, TextEdit, Ui};

use pindah_drive::gaya::{kecil, tebal, tombol, tombol_utama, HIJAU, MERAH, REDUP, TAUTAN, TINTA};
use pindah_drive::mesin::filter::{awal_bulan_ini, awal_tahun_ini, tanggal_tampil, Filter};

use crate::dialog::Hasil;

/// Pilihan cepat: (label, isi yang ditambahkan).
const JENIS: [(&str, &str); 3] = [
    ("Dokumen kantor", "pdf, doc, docx, xls, xlsx, ppt, pptx, odt, ods, csv, txt"),
    ("Gambar", "jpg, jpeg, png, heic, gif, webp, tif, tiff"),
    ("Video", "mp4, mov, mkv, avi"),
];
const LEWATI: [(&str, &str); 2] = [
    ("File sementara", "*.tmp, *.temp, *.bak, *.part, *.crdownload"),
    ("Folder pengembang", "node_modules/, .git/"),
];

pub struct FilterIsian {
    pub sertakan: String,
    pub lewati: String,
    pub sejak: String,
    pub galat: Option<String>,
}

impl FilterIsian {
    pub fn dari(f: &Filter) -> Self {
        FilterIsian {
            sertakan: f.teks_sertakan(),
            // "node_modules/**" ditampilkan kembali sebagai "node_modules/" (bentuk yang diketik).
            lewati: f.lewati.iter().map(|l| l.strip_suffix("**").unwrap_or(l).to_string()).collect::<Vec<_>>().join(", "),
            sejak: f.sejak.clone().unwrap_or_default(),
            galat: None,
        }
    }
}

/// Menambah butir ke daftar berkoma tanpa duplikat.
fn tambah_ke(daftar: &mut String, isi: &str) {
    let mut ada: Vec<String> = daftar.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    for b in isi.split(',').map(str::trim) {
        if !ada.iter().any(|x| x.eq_ignore_ascii_case(b)) {
            ada.push(b.to_string());
        }
    }
    *daftar = ada.join(", ");
}

fn isian(ui: &mut Ui, teks: &mut String, contoh: &str) {
    ui.add(
        TextEdit::singleline(teks)
            .hint_text(contoh)
            .desired_width(f32::INFINITY)
            .min_size(vec2(0.0, 30.0))
            .vertical_align(Align::Center)
            .margin(pindah_drive::gaya::JARAK_ISIAN),
    );
}

fn subjudul(ui: &mut Ui, t: &str) {
    ui.label(RichText::new(t).family(tebal()).size(14.0).color(TINTA));
}

pub fn tampil(ui: &mut Ui, s: &mut FilterIsian) -> Option<Hasil> {
    ui.set_width(560.0);
    ui.label(RichText::new("Filter file").family(tebal()).size(16.5).color(TINTA));
    ui.add_space(2.0);
    ui.label(
        RichText::new("Berlaku untuk folder di panel Dari: periksa, uji coba, salin, verifikasi, dan pindahkan. File yang tidak lolos filter tidak disalin dan tidak ikut dicocokkan.")
            .color(REDUP),
    );
    ui.add_space(12.0);

    subjudul(ui, "Hanya jenis file ini");
    isian(ui, &mut s.sertakan, "kosong = semua jenis; contoh: pdf, docx, xlsx");
    ui.horizontal_wrapped(|ui| {
        for (l, isi) in JENIS {
            if ui.link(kecil(&format!("+ {l}"), TAUTAN)).on_hover_text(isi).clicked() {
                tambah_ke(&mut s.sertakan, isi);
            }
        }
    });
    ui.add_space(10.0);

    subjudul(ui, "Lewati");
    isian(ui, &mut s.lewati, "contoh: *.tmp, Arsip Lama/, node_modules/");
    ui.horizontal_wrapped(|ui| {
        for (l, isi) in LEWATI {
            if ui.link(kecil(&format!("+ {l}"), TAUTAN)).on_hover_text(isi).clicked() {
                tambah_ke(&mut s.lewati, isi);
            }
        }
    });
    ui.label(kecil("Akhiri dengan / untuk melewati folder beserta isinya. * berarti teks apa saja.", REDUP));
    ui.add_space(10.0);

    subjudul(ui, "Hanya yang diubah sejak");
    ui.horizontal(|ui| {
        ui.add(
            TextEdit::singleline(&mut s.sejak)
                .hint_text("31-01-2026")
                .desired_width(140.0)
                .min_size(vec2(0.0, 30.0))
                .vertical_align(Align::Center)
                .margin(pindah_drive::gaya::JARAK_ISIAN),
        );
        ui.add_space(6.0);
        if ui.link(kecil("Awal bulan ini", TAUTAN)).clicked() {
            s.sejak = tanggal_tampil(&awal_bulan_ini());
        }
        if ui.link(kecil("Awal tahun ini", TAUTAN)).clicked() {
            s.sejak = tanggal_tampil(&awal_tahun_ini());
        }
        if !s.sejak.is_empty() && ui.link(kecil("Kosongkan", TAUTAN)).clicked() {
            s.sejak.clear();
        }
    });
    ui.add_space(12.0);

    // Pratinjau langsung (dan pemeriksaan isian).
    let sejak = s.sejak.trim().to_string();
    let hasil = Filter::urai(&s.sertakan, &s.lewati, &ubah_tanggal_tampil(&sejak));
    match &hasil {
        Ok(f) if f.kosong() => {
            ui.label(RichText::new("Tanpa filter: seluruh isi folder disalin.").color(REDUP));
        }
        Ok(f) => {
            ui.label(RichText::new(format!("Filter: {}", f.uraian())).color(HIJAU));
        }
        Err(e) => {
            ui.label(RichText::new(e).color(MERAH));
        }
    }
    if let Some(g) = &s.galat {
        ui.label(RichText::new(g).color(MERAH));
    }

    let mut h = None;
    ui.add_space(16.0);
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        if ui.add_enabled(hasil.is_ok(), tombol_utama("Simpan filter")).clicked() {
            match &hasil {
                Ok(f) => h = Some(Hasil::FilterSimpan(f.clone())),
                Err(e) => s.galat = Some(e.clone()),
            }
        }
        if ui.add(tombol("Batal").min_size(vec2(0.0, 38.0))).clicked() {
            h = Some(Hasil::Batal);
        }
        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            if ui.add(tombol("Hapus filter")).on_hover_text("Salin seluruh isi folder lagi.").clicked() {
                h = Some(Hasil::FilterSimpan(Filter::default()));
            }
        });
    });
    h
}

/// "5 Nov 2026" (dari pilihan cepat) -> "2026-11-05"; teks lain dibiarkan untuk diurai mesin.
fn ubah_tanggal_tampil(t: &str) -> String {
    const BULAN: [&str; 12] = ["jan", "feb", "mar", "apr", "mei", "jun", "jul", "agu", "sep", "okt", "nov", "des"];
    let b: Vec<&str> = t.split_whitespace().collect();
    if let [d, m, y] = b.as_slice() {
        if let (Ok(d), Some(m), Ok(y)) = (d.parse::<u32>(), BULAN.iter().position(|x| m.to_lowercase().starts_with(x)), y.parse::<u32>()) {
            return format!("{y:04}-{:02}-{d:02}", m + 1);
        }
    }
    t.to_string()
}

#[cfg(test)]
mod uji {
    use super::*;

    #[test]
    fn isian_bolak_balik() {
        let f = Filter::urai("pdf, docx", "*.tmp, node_modules/", "2026-01-05").unwrap();
        let s = FilterIsian::dari(&f);
        assert_eq!(s.lewati, "*.tmp, node_modules/");
        assert_eq!(Filter::urai(&s.sertakan, &s.lewati, &s.sejak).unwrap(), f);
        assert_eq!(ubah_tanggal_tampil("5 Jan 2026"), "2026-01-05");
        assert_eq!(ubah_tanggal_tampil("31-01-2026"), "31-01-2026");
        let mut d = "pdf, DOCX".to_string();
        tambah_ke(&mut d, "docx, xlsx");
        assert_eq!(d, "pdf, DOCX, xlsx");
    }
}
