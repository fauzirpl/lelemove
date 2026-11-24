//! Dialog "Bantuan & info": tanya jawab, ketentuan penggunaan, kebijakan privasi, atribusi, tentang.
//!
//! Teksnya ditanam dari docs\*.md (satu sumber dengan dokumen yang bisa dipublikasikan, mis. sebagai
//! halaman kebijakan privasi untuk Google) dan dari rust\aset\atribusi.txt, yang dibuat ulang oleh
//! buat-installer.ps1 dari `cargo tree` agar daftar pustaka & lisensinya selalu sesuai build.

use eframe::egui::{self, vec2, Align, Layout, RichText, ScrollArea, Ui};

use pindah_drive::gaya::{self, kecil, tebal, tombol, tombol_utama, REDUP, TINTA};
use pindah_drive::{win, VERSI};

use crate::dialog::Hasil;

pub const TANYA_JAWAB: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../docs/tanya-jawab.md"));
pub const KETENTUAN: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../docs/ketentuan.md"));
pub const PRIVASI: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../docs/privasi.md"));
pub const ATRIBUSI: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/aset/atribusi.txt"));

#[derive(Clone, Copy, PartialEq)]
pub enum Tab {
    TanyaJawab,
    Ketentuan,
    Privasi,
    Atribusi,
    Tentang,
}

impl Tab {
    const SEMUA: [(Tab, &'static str); 5] = [
        (Tab::TanyaJawab, "Tanya jawab"),
        (Tab::Ketentuan, "Ketentuan"),
        (Tab::Privasi, "Privasi"),
        (Tab::Atribusi, "Atribusi"),
        (Tab::Tentang, "Tentang"),
    ];

    pub fn dari_nama(n: &str) -> Tab {
        match n {
            "ketentuan" => Tab::Ketentuan,
            "privasi" => Tab::Privasi,
            "atribusi" => Tab::Atribusi,
            "tentang" => Tab::Tentang,
            _ => Tab::TanyaJawab,
        }
    }
}

pub struct Info {
    pub tab: Tab,
    pub versi_rclone: String,
    pub dir_app: String,
    pub dir_data: String,
    pub dir_log: String,
}

// ------------------------------------------------------------------ penampil Markdown sederhana

/// Paragraf; alamat https://... menjadi tautan yang bisa diklik.
fn paragraf(ui: &mut Ui, t: &str) {
    if !t.contains("https://") {
        ui.add(egui::Label::new(RichText::new(t).color(TINTA)).wrap());
        return;
    }
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for kata in t.split(' ') {
            let bersih = kata.trim_end_matches(['.', ',', ')', ';']);
            if bersih.starts_with("https://") {
                ui.hyperlink_to(bersih, bersih);
                let sisa = &kata[bersih.len()..];
                if !sisa.is_empty() {
                    ui.label(RichText::new(sisa).color(TINTA));
                }
            } else {
                ui.label(RichText::new(kata).color(TINTA));
            }
        }
    });
}

fn poin(ui: &mut Ui, t: &str) {
    ui.horizontal_top(|ui| {
        ui.add_space(4.0);
        ui.label(RichText::new("\u{2022}").color(REDUP));
        ui.vertical(|ui| paragraf(ui, t));
    });
}

/// "## Judul", paragraf dipisah baris kosong, dan daftar "- poin". Judul "# ..." dilewati
/// (sudah jadi nama tab).
fn dokumen(ui: &mut Ui, md: &str) {
    for blok in md.replace("\r\n", "\n").split("\n\n") {
        let b = blok.trim();
        if b.is_empty() || (b.starts_with("# ") && !b.contains('\n')) {
            continue;
        }
        if let Some(h) = b.strip_prefix("## ") {
            ui.add_space(8.0);
            ui.label(RichText::new(h).family(tebal()).size(14.5).color(TINTA));
            ui.add_space(1.0);
            continue;
        }
        if b.lines().all(|l| l.trim_start().starts_with("- ")) {
            for l in b.lines() {
                poin(ui, l.trim_start()[2..].trim());
            }
            ui.add_space(4.0);
            continue;
        }
        paragraf(ui, &b.lines().map(str::trim).collect::<Vec<_>>().join(" "));
        ui.add_space(4.0);
    }
}

/// "### Pertanyaan" diikuti jawabannya; tiap pertanyaan bisa dibuka-tutup.
fn tanya_jawab(ui: &mut Ui) {
    let teks = TANYA_JAWAB.replace("\r\n", "\n");
    for (i, bagian) in teks.split("\n### ").skip(1).enumerate() {
        let (tanya, jawab) = bagian.split_once('\n').unwrap_or((bagian, ""));
        egui::CollapsingHeader::new(RichText::new(tanya.trim()).family(tebal()).size(13.5).color(TINTA))
            .id_salt(("tanya-jawab", i))
            .default_open(i == 0)
            .show(ui, |ui| dokumen(ui, jawab));
    }
}

// ------------------------------------------------------------------ atribusi

struct Komponen {
    nama: &'static str,
    lisensi: &'static str,
    peran: &'static str,
    url: &'static str,
}

const KOMPONEN: &[Komponen] = &[
    Komponen {
        nama: "rclone",
        lisensi: "MIT",
        peran: "Mesin penyalin, disertakan sebagai rclone.exe. Hak cipta Nick Craig-Wood dan para kontributor rclone.",
        url: "https://rclone.org",
    },
    Komponen {
        nama: "egui & eframe",
        lisensi: "MIT OR Apache-2.0",
        peran: "Tampilan jendela. Hak cipta Emil Ernerfeldt dan para kontributor egui.",
        url: "https://github.com/emilk/egui",
    },
    Komponen {
        nama: "Rust",
        lisensi: "MIT OR Apache-2.0",
        peran: "Bahasa pemrograman dan pustaka standarnya.",
        url: "https://www.rust-lang.org",
    },
    Komponen {
        nama: "Runtime MinGW-w64 & libgcc",
        lisensi: "ZPL 2.1 / domain publik; GPL dengan GCC Runtime Library Exception",
        peran: "Ditautkan oleh toolchain Rust GNU ke setiap .exe.",
        url: "https://www.mingw-w64.org",
    },
    Komponen {
        nama: "Font bawaan egui",
        lisensi: "MIT, Ubuntu Font Licence 1.0, SIL OFL 1.1",
        peran: "Hack (MIT), Ubuntu Light (Ubuntu Font Licence), Noto Emoji (OFL 1.1), emoji-icon-font (MIT); cadangan bila font Windows tidak ada.",
        url: "https://github.com/emilk/egui/tree/master/crates/epaint_default_fonts",
    },
    Komponen {
        nama: "Segoe UI, Segoe MDL2 Assets, Consolas",
        lisensi: "Milik Microsoft, bawaan Windows",
        peran: "Dipakai dari folder font Windows; tidak disertakan di LELEMOVE.",
        url: "https://learn.microsoft.com/typography/font-list/",
    },
];

const LAYANAN: &[Komponen] = &[
    Komponen {
        nama: "Google Drive API",
        lisensi: "Google APIs Terms of Service",
        peran: "Akses Google Drive. Juga tunduk pada Google API Services User Data Policy.",
        url: "https://developers.google.com/terms",
    },
    Komponen {
        nama: "Microsoft Graph API",
        lisensi: "Microsoft APIs Terms of Use",
        peran: "Akses OneDrive dan SharePoint. Layanan lain (Dropbox, Box, pCloud, MEGA, S3, WebDAV, SFTP, dll.) diakses lewat backend rclone dan tunduk pada ketentuan penyedianya masing-masing.",
        url: "https://learn.microsoft.com/legal/microsoft-apis/terms-of-use",
    },
];

fn kartu(ui: &mut Ui, k: &Komponen) {
    egui::Frame::new()
        .fill(gaya::LATAR2)
        .stroke(egui::Stroke::new(1.0_f32, gaya::GARIS))
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(k.nama).family(tebal()).color(TINTA));
                ui.label(kecil(k.lisensi, REDUP));
            });
            ui.add(egui::Label::new(kecil(k.peran, REDUP)).wrap());
            ui.hyperlink_to(kecil(k.url, gaya::TAUTAN), k.url);
        });
    ui.add_space(4.0);
}

fn atribusi(ui: &mut Ui) {
    paragraf(ui, "LELEMOVE dibangun di atas perangkat lunak open source dan layanan berikut. Terima kasih kepada para pembuatnya.");
    ui.add_space(6.0);
    ui.label(RichText::new("Komponen utama").family(tebal()).size(14.5).color(TINTA));
    ui.add_space(2.0);
    for k in KOMPONEN {
        kartu(ui, k);
    }
    ui.add_space(6.0);
    ui.label(RichText::new("Layanan & API").family(tebal()).size(14.5).color(TINTA));
    ui.add_space(2.0);
    for k in LAYANAN {
        kartu(ui, k);
    }
    ui.add(
        egui::Label::new(kecil(
            "Google Drive adalah merek dagang Google LLC. Microsoft, OneDrive, dan SharePoint adalah merek dagang Microsoft Corporation. LELEMOVE tidak berafiliasi dengan, disponsori, atau didukung oleh Google, Microsoft, maupun Lalamove. Ikon LELEMOVE dibuat sendiri.",
            REDUP,
        ))
        .wrap(),
    );
    ui.add_space(10.0);
    let pustaka: Vec<Vec<&str>> = ATRIBUSI.lines().filter(|l| !l.trim().is_empty()).map(|l| l.splitn(3, '|').collect()).collect();
    ui.label(RichText::new(format!("Pustaka Rust ({})", pustaka.len())).family(tebal()).size(14.5).color(TINTA));
    ui.add(egui::Label::new(kecil("Semua pustaka di bawah ini ikut tertanam di LELEMOVE.exe, pindah.exe, dan installer. Semuanya berlisensi permisif. Arahkan kursor ke nama untuk melihat alamat repositorinya.", REDUP)).wrap());
    ui.add_space(4.0);
    egui::Grid::new("daftar-pustaka").striped(true).num_columns(2).spacing(vec2(16.0, 3.0)).show(ui, |ui| {
        for p in &pustaka {
            let nama = p.first().copied().unwrap_or("");
            let lisensi = p.get(1).copied().filter(|s| !s.is_empty()).unwrap_or("(tidak dicantumkan)");
            let r = ui.label(RichText::new(nama).size(12.5).color(TINTA));
            if let Some(url) = p.get(2).filter(|s| !s.is_empty()) {
                r.on_hover_text(*url);
            }
            ui.label(RichText::new(lisensi).size(12.5).color(REDUP));
            ui.end_row();
        }
    });
}

// ------------------------------------------------------------------ tentang

fn baris_folder(ui: &mut Ui, label: &str, jalur: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(REDUP));
        ui.add(egui::Label::new(RichText::new(jalur).size(12.5).family(egui::FontFamily::Monospace).color(TINTA)).truncate());
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.add(tombol("Buka")).clicked() {
                let _ = std::fs::create_dir_all(jalur);
                win::buka(jalur);
            }
        });
    });
}

fn tentang(ui: &mut Ui, i: &Info) {
    ui.label(RichText::new("LELEMOVE").family(tebal()).size(22.0).color(gaya::HIJAU));
    ui.label(RichText::new(format!("Versi {VERSI}")).color(TINTA));
    ui.add_space(6.0);
    paragraf(ui, "Salin file antar OneDrive, SharePoint, Google Drive, dan folder komputer, ke arah mana pun. Menyalin dan mencocokkan; file di sumber tidak diubah, kecuali Anda sendiri memilih Pindahkan setelah verifikasi (yang dihapus masuk ke Recycle Bin / Sampah).");
    ui.add_space(8.0);
    egui::Grid::new("tentang").num_columns(2).spacing(vec2(14.0, 4.0)).show(ui, |ui| {
        let mut baris = |k: &str, v: &str| {
            ui.label(RichText::new(k).color(REDUP));
            ui.label(RichText::new(v).color(TINTA));
            ui.end_row();
        };
        baris("Mesin penyalin", if i.versi_rclone.is_empty() { "rclone (belum terpasang)" } else { &i.versi_rclone });
        baris("Dibangun dengan", "Rust, egui/eframe, toolchain GNU");
        baris("Sistem", "Windows 10 dan 11, dipasang per pengguna tanpa hak admin");
        baris("Data dikirim ke pengembang", "Tidak ada (lihat tab Privasi)");
    });
    ui.add_space(10.0);
    baris_folder(ui, "Folder aplikasi", &i.dir_app);
    baris_folder(ui, "Data & pengaturan", &i.dir_data);
    baris_folder(ui, "Log", &i.dir_log);
}

// ------------------------------------------------------------------ dialog

pub fn tampil(ui: &mut Ui, i: &mut Info) -> Option<Hasil> {
    ui.set_width(680.0);
    ui.label(RichText::new("Bantuan & info").family(tebal()).size(16.5).color(TINTA));
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        for (tab, nama) in Tab::SEMUA {
            let dipilih = i.tab == tab;
            let teks = RichText::new(nama).size(13.5).color(if dipilih { TINTA } else { REDUP });
            let teks = if dipilih { teks.family(tebal()) } else { teks };
            if ui.selectable_label(dipilih, teks).clicked() {
                i.tab = tab;
            }
        }
    });
    ui.separator();
    let tinggi = (ui.ctx().screen_rect().height() - 260.0).clamp(240.0, 470.0);
    ScrollArea::vertical()
        .id_salt(("info", i.tab as u8))
        .max_height(tinggi)
        .min_scrolled_height(tinggi)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_width(ui.available_width() - 10.0);
            match i.tab {
                Tab::TanyaJawab => tanya_jawab(ui),
                Tab::Ketentuan => dokumen(ui, KETENTUAN),
                Tab::Privasi => dokumen(ui, PRIVASI),
                Tab::Atribusi => atribusi(ui),
                Tab::Tentang => tentang(ui, i),
            }
        });
    let mut h = None;
    ui.add_space(12.0);
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        if ui.add(tombol_utama("Tutup")).clicked() {
            h = Some(Hasil::Batal);
        }
    });
    h
}

#[cfg(test)]
mod uji {
    use super::*;

    #[test]
    fn teks_tertanam() {
        assert!(TANYA_JAWAB.matches("\n### ").count() >= 10, "tanya jawab terlalu sedikit");
        assert!(PRIVASI.contains("Limited Use"));
        assert!(KETENTUAN.contains("sebagaimana adanya"));
        assert!(ATRIBUSI.lines().count() > 50);
        assert!(ATRIBUSI.lines().all(|l| l.split('|').count() == 3), "format atribusi: nama versi|lisensi|repo");
    }
}
