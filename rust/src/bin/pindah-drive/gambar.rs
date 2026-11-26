//! Jendela utama: fungsi gambar bebas (baris daftar lokasi, menu akun, baris folder, pesan kosong).
use super::*;

/// Baris di daftar lokasi kiri; `menu` = tombol ⋯ (hanya akun cloud).
pub(crate) struct BarisLokasi {
    pub(crate) baris: egui::Response,
    pub(crate) menu: Option<egui::Response>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn baris_lokasi(
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
    // Daftar lokasi berlatar gelap (hijau kolam tua): semua warna di sini padanan terangnya.
    let warna_detail = gaya::di_gelap(warna_detail);
    let titik = titik.map(gaya::di_gelap);
    let dua = !detail.is_empty();
    let tinggi = if dua { 46.0 } else { 32.0 };
    let w = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(vec2(w, tinggi), if boleh { Sense::click() } else { Sense::hover() });
    let r_menu = Rect::from_center_size(pos2(r.right() - 12.0, r.center().y), vec2(22.0, 26.0));
    let menu = (titik.is_some() && boleh).then(|| ui.interact(r_menu, resp.id.with("menu"), Sense::click()));
    gaya::nama_tombol(&resp, boleh, judul);
    if let Some(m) = &menu {
        gaya::nama_tombol(m, true, &format!("Menu {judul}"));
    }
    if ui.is_rect_visible(r) {
        let p = ui.painter();
        let disorot = boleh && (resp.hovered() || menu.as_ref().is_some_and(|m| m.hovered()));
        let latar = if sorot {
            gaya::KOLAM_PILIH
        } else if disorot {
            gaya::KOLAM_SOROT
        } else {
            gaya::KOLAM_TUA
        };
        p.rect_filled(r, 8.0, latar);
        if sorot {
            // Penanda lokasi yang sedang dibuka: bilah mint di tepi kiri.
            let bilah = Rect::from_min_max(pos2(r.left(), r.top() + 7.0), pos2(r.left() + 3.0, r.bottom() - 7.0));
            p.rect_filled(bilah, 2.0, gaya::MINT);
        }
        let sisi_petak = if dua { 28.0 } else { 22.0 };
        let rp = Rect::from_center_size(pos2(r.left() + 10.0 + sisi_petak / 2.0, r.center().y), vec2(sisi_petak, sisi_petak));
        let putih_tipis = Color32::from_rgba_unmultiplied(255, 255, 255, 26);
        if jenis == "tambah" {
            p.text(rp.center(), Align2::CENTER_CENTER, glyph, FontId::new(12.0, keluarga_ikon()), if boleh { gaya::MINT } else { gaya::ABU_SISI });
        } else if jenis == "pengaturan" {
            p.text(rp.center(), Align2::CENTER_CENTER, glyph, FontId::new(14.0, keluarga_ikon()), gaya::REDUP_SISI);
        } else if jenis == "dukung" {
            p.text(rp.center(), Align2::CENTER_CENTER, glyph, FontId::new(14.0, keluarga_ikon()), MERAH_HATI);
        } else if mp::cari(jenis).is_none() && jenis != "sinkron" {
            // Tempat di komputer ini: petak tembus pandang, bukan kotak terang.
            p.rect_filled(rp, (sisi_petak * 0.24).round(), putih_tipis);
            p.text(rp.center(), Align2::CENTER_CENTER, glyph, FontId::new(sisi_petak * 0.56, keluarga_ikon()), gaya::TEKS_SISI);
        } else {
            gaya::petak_lukis(p, rp, jenis, glyph, titik, latar);
        }
        let x = rp.right() + 10.0;
        let kanan = if menu.is_some() { r_menu.left() - 2.0 } else { r.right() - 8.0 };
        let g_tanda = (!tanda.is_empty()).then(|| p.layout_no_wrap(tanda.to_string(), FontId::new(9.5, tebal()), gaya::MINT));
        let lebar_judul = kanan - x - g_tanda.as_ref().map_or(0.0, |g| g.size().x + 6.0);
        let warna_judul = if jenis == "tambah" {
            if boleh { gaya::MINT } else { gaya::ABU_SISI }
        } else if boleh {
            gaya::TEKS_SISI
        } else {
            gaya::REDUP_SISI
        };
        let huruf = if dua { FontId::new(13.0, tebal()) } else { FontId::proportional(13.5) };
        let gj = potong(ui, judul, huruf, warna_judul, lebar_judul);
        let y = if dua { r.top() + 6.0 } else { r.center().y - gj.size().y / 2.0 };
        let tinggi_judul = gj.size().y;
        p.galley(pos2(x, y), gj, warna_judul);
        if let Some(g) = g_tanda {
            p.galley(pos2(kanan - g.size().x, y + (tinggi_judul - g.size().y) / 2.0 + 0.5), g, gaya::MINT);
        }
        if dua {
            let gd = potong(ui, detail, FontId::proportional(11.5), warna_detail, kanan - x);
            p.galley(pos2(x, r.top() + 25.0), gd, warna_detail);
        }
        if let Some(m) = &menu {
            if m.hovered() {
                p.rect_filled(r_menu, 5.0, putih_tipis);
            }
            let warna = if m.hovered() || disorot || sorot { gaya::TEKS_SISI } else { gaya::ABU_SISI };
            p.text(r_menu.center(), Align2::CENTER_CENTER, ik::LAIN, FontId::new(12.0, keluarga_ikon()), warna);
        }
    }
    let menu = menu.map(|m| m.on_hover_text("Atur akun"));
    BarisLokasi { baris: resp, menu }
}

/// Isi menu Atur (⋯ atau klik kanan) untuk satu akun.
pub(crate) fn menu_akun(ui: &mut Ui, a: &Akun, pilih: &mut Option<&'static str>) {
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
    // Remote impor diatur lewat rclone sendiri; layanan berisian: isi ulang datanya.
    if !a.impor {
        let isian = mp::cari(&a.jenis).is_some_and(|p| !p.oauth);
        item(ui, None, if isian { "Ubah data login\u{2026}" } else { "Login ulang" }, "ulang", TINTA);
    }
    if a.jenis == "google" && a.scope != "drive" {
        item(ui, None, "Beri izin penuh (agar bisa jadi sumber)", "penuh", TINTA);
    }
    // Client ID Google diatur di Pengaturan (bilah alat), bukan per akun.
    ui.separator();
    item(ui, None, if a.impor { "Berhenti memakai remote ini" } else { "Lepas akun ini" }, "lepas", MERAH);
}

/// Satu baris menu: glyph opsional + teks, latar muncul saat disorot.
pub(crate) fn item_menu(ui: &mut Ui, glyph: Option<char>, teks: &str, warna: Color32) -> egui::Response {
    let w = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(vec2(w, 28.0), Sense::click());
    gaya::nama_tombol(&resp, true, teks);
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
pub(crate) fn baris_folder(ui: &mut Ui, glyph: char, warna_glyph: Color32, teks: &str, terpilih: bool, boleh: bool) -> egui::Response {
    let w = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(vec2(w, TINGGI_BARIS), if boleh { Sense::click() } else { Sense::hover() });
    gaya::nama_tombol(&resp, boleh, teks);
    if ui.is_rect_visible(r) {
        let p = ui.painter();
        if terpilih {
            p.rect_filled(r, 7.0, PILIH);
        } else if boleh && resp.hovered() {
            p.rect_filled(r, 7.0, HOVER);
        }
        if glyph == ik::MAP {
            gaya::lukis_map(p, pos2(r.left() + 20.0, r.center().y), 16.0);
        } else {
            p.text(pos2(r.left() + 20.0, r.center().y), Align2::CENTER_CENTER, glyph, FontId::new(15.0, keluarga_ikon()), warna_glyph);
        }
        let warna = if boleh { TINTA } else { REDUP };
        let g = potong(ui, teks, FontId::proportional(13.5), warna, w - 54.0);
        p.galley(pos2(r.left() + 40.0, r.center().y - g.size().y / 2.0), g, warna);
    }
    resp
}

/// Pesan di tempat daftar folder (kosong, belum ada, galat).
pub(crate) fn pesan_kosong(ui: &mut Ui, glyph: char, judul: &str, ket: &str, warna: Color32) {
    let netral = warna == REDUP;
    ui.add_space(14.0);
    ui.horizontal_top(|ui| {
        ui.add_space(12.0);
        ui.label(gaya::ikon(glyph, 15.0, if netral { ABU } else { warna }));
        ui.add_space(4.0);
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
