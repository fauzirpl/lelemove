//! Tampilan egui bersama (jendela aplikasi & installer): warna, huruf Segoe UI + ikon Segoe MDL2, tombol.

use eframe::egui::{
    self, vec2, Align2, Button, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, RichText,
    Sense, Stroke, TextStyle, Ui,
};
#[cfg(windows)]
use std::path::PathBuf;
use std::sync::Arc;

pub const PUTIH: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
pub const TINTA: Color32 = Color32::from_rgb(0x1F, 0x23, 0x28);
pub const TINTA_HOVER: Color32 = Color32::from_rgb(0x37, 0x3E, 0x47);
pub const REDUP: Color32 = Color32::from_rgb(0x59, 0x63, 0x6E);
pub const ABU: Color32 = Color32::from_rgb(0x8C, 0x95, 0x9F);
pub const HIJAU: Color32 = Color32::from_rgb(0x1A, 0x7F, 0x37);
pub const KUNING: Color32 = Color32::from_rgb(0x9A, 0x67, 0x00);
pub const MERAH: Color32 = Color32::from_rgb(0xCF, 0x22, 0x2E);
pub const TAUTAN: Color32 = Color32::from_rgb(0x09, 0x69, 0xDA);
pub const GARIS: Color32 = Color32::from_rgb(0xD1, 0xD9, 0xE0);
pub const LATAR2: Color32 = Color32::from_rgb(0xF6, 0xF8, 0xFA);
pub const HOVER: Color32 = Color32::from_rgb(0xF3, 0xF4, 0xF6);
pub const JALUR: Color32 = Color32::from_rgb(0xEA, 0xEE, 0xF2);
/// Latar baris terpilih (daftar folder, lokasi yang sedang dibuka).
pub const PILIH: Color32 = Color32::from_rgb(0xDD, 0xF4, 0xFF);
/// Garis tipis di dalam bingkai (pemisah baris, tepi panel yang tidak aktif).
pub const GARIS_HALUS: Color32 = Color32::from_rgb(0xE6, 0xEA, 0xEF);
/// Warna map pada daftar folder (kuning map ala Explorer, sedikit diredam).
pub const MAP: Color32 = Color32::from_rgb(0xDB, 0xA5, 0x2B);

/// Keluarga huruf tebal (Segoe UI Semibold).
pub fn tebal() -> FontFamily {
    FontFamily::Name("tebal".into())
}

/// Keluarga huruf ikon (Segoe MDL2 Assets / Segoe Fluent Icons, bawaan Windows 10/11).
pub fn keluarga_ikon() -> FontFamily {
    FontFamily::Name("ikon".into())
}

/// Glyph ikon jendela: Segoe MDL2 di Windows, simbol Unicode di macOS.
#[cfg(windows)]
pub use ik_mdl2 as ik;
#[cfg(not(windows))]
pub use ik_unicode as ik;

/// Pengganti ikon di macOS: simbol Unicode yang ada di huruf bawaan egui (Noto Emoji,
/// emoji-icon-font) sehingga selalu tampil, apa pun huruf yang terpasang di Mac.
pub mod ik_unicode {
    pub const MAP: char = '\u{1F5C0}';
    pub const NAIK: char = '\u{2B06}';
    pub const PANAH: char = '\u{27A1}';
    pub const TUKAR: char = '\u{1F503}';
    pub const KOMPUTER: char = '\u{1F5B3}';
    pub const DISK: char = '\u{1F5B4}';
    pub const AWAN: char = '\u{2601}';
    pub const MAP_SINKRON: char = '\u{1F501}';
    pub const TAMBAH: char = '\u{2795}';
    pub const LAIN: char = '\u{2026}';
    pub const SEGARKAN: char = '\u{1F504}';
    pub const CARI: char = '\u{1F50D}';
    pub const MAIN: char = '\u{25B6}';
    pub const HENTI: char = '\u{23F9}';
    pub const SELESAI: char = '\u{2714}';
    pub const AWAS: char = '\u{26A0}';
    pub const GALAT: char = '\u{26D4}';
    pub const SPEEDO: char = '\u{23F1}';
    pub const PENGATURAN: char = '\u{2699}';
    pub const BANTUAN: char = '\u{2753}';
    pub const JADWAL: char = '\u{1F4C5}';
    pub const RIWAYAT: char = '\u{1F553}';
    pub const BUKA_MAP: char = '\u{1F5C1}';
    pub const BAWAH: char = '\u{23F7}';
    pub const KANAN: char = '\u{23F5}';
    pub const TUTUP: char = '\u{1F5D9}';
    pub const DESKTOP: char = '\u{1F5B5}';
    pub const DOKUMEN: char = '\u{1F5CB}';
    pub const EMAIL: char = '\u{2709}';
    pub const HATI: char = '\u{2665}';

    /// Semua glyph di atas (untuk uji ketersediaan huruf).
    pub const SEMUA: [char; 30] = [
        MAP, NAIK, PANAH, TUKAR, KOMPUTER, DISK, AWAN, MAP_SINKRON, TAMBAH, LAIN, SEGARKAN, CARI, MAIN, HENTI,
        SELESAI, AWAS, GALAT, SPEEDO, PENGATURAN, BANTUAN, JADWAL, RIWAYAT, BUKA_MAP, BAWAH, KANAN, TUTUP,
        DESKTOP, DOKUMEN, EMAIL, HATI,
    ];
}

/// Kode glyph Segoe MDL2 Assets yang dipakai jendela.
pub mod ik_mdl2 {
    pub const MAP: char = '\u{F12B}';
    pub const NAIK: char = '\u{E74A}';
    pub const PANAH: char = '\u{E72A}';
    pub const TUKAR: char = '\u{E8AB}';
    pub const KOMPUTER: char = '\u{E977}';
    pub const DISK: char = '\u{EDA2}';
    pub const AWAN: char = '\u{E753}';
    pub const MAP_SINKRON: char = '\u{E8F7}';
    pub const TAMBAH: char = '\u{E710}';
    pub const LAIN: char = '\u{E712}';
    pub const SEGARKAN: char = '\u{E72C}';
    pub const CARI: char = '\u{E721}';
    pub const MAIN: char = '\u{E768}';
    pub const HENTI: char = '\u{E73B}';
    pub const SELESAI: char = '\u{E930}';
    pub const AWAS: char = '\u{E7BA}';
    pub const GALAT: char = '\u{E783}';
    pub const SPEEDO: char = '\u{EC4A}';
    pub const PENGATURAN: char = '\u{E713}';
    pub const BANTUAN: char = '\u{E897}';
    pub const JADWAL: char = '\u{E787}';
    pub const RIWAYAT: char = '\u{E81C}';
    pub const BUKA_MAP: char = '\u{ED25}';
    pub const BAWAH: char = '\u{E70D}';
    pub const KANAN: char = '\u{E76C}';
    pub const TUTUP: char = '\u{E711}';
    pub const DESKTOP: char = '\u{E7F4}';
    pub const DOKUMEN: char = '\u{E8A5}';
    pub const EMAIL: char = '\u{E715}';
    /// HeartFill.
    pub const HATI: char = '\u{E00B}';
}

/// Ikon map di daftar folder, dilukis sendiri (dua warna kuning) agar mirip map di Explorer.
pub fn lukis_map(p: &egui::Painter, c: egui::Pos2, lebar: f32) {
    let t = (lebar * 0.78).round();
    let r = egui::Rect::from_center_size(c, vec2(lebar, t));
    let belakang = Color32::from_rgb(0xD6, 0x9A, 0x1E);
    let depan = Color32::from_rgb(0xF1, 0xC0, 0x4A);
    let tab = egui::Rect::from_min_size(r.min, vec2((lebar * 0.42).round(), (t * 0.34).round()));
    p.rect_filled(tab, CornerRadius { nw: 2, ne: 2, sw: 0, se: 0 }, belakang);
    p.rect_filled(egui::Rect::from_min_max(egui::pos2(r.left(), r.top() + (t * 0.18).round()), r.max), CornerRadius::same(2), belakang);
    p.rect_filled(egui::Rect::from_min_max(egui::pos2(r.left(), r.top() + (t * 0.36).round()), r.max), CornerRadius::same(2), depan);
}

/// Teks satu glyph ikon.
pub fn ikon(c: char, ukuran: f32, warna: Color32) -> RichText {
    RichText::new(c.to_string()).family(keluarga_ikon()).size(ukuran).color(warna)
}

/// Warna petak lokasi (latar, glyph) per jenis: "google", "onedrive", "sharepoint", selain itu komputer.
pub fn warna_jenis(jenis: &str) -> (Color32, Color32) {
    if jenis == "sinkron" {
        return (Color32::from_rgb(0xE3, 0xEF, 0xFB), Color32::from_rgb(0x0B, 0x5C, 0xAD));
    }
    match crate::mesin::penyedia::cari(jenis) {
        Some(p) => {
            let w = p.warna;
            (Color32::from_rgb(w[0], w[1], w[2]), Color32::from_rgb(w[3], w[4], w[5]))
        }
        None => (JALUR, Color32::from_rgb(0x3B, 0x44, 0x4F)),
    }
}

/// Petak ikon lokasi (persegi membulat berisi glyph), dengan titik status opsional di pojok kanan bawah.
pub fn petak(ui: &mut Ui, jenis: &str, glyph: char, sisi: f32, titik: Option<Color32>) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(sisi, sisi), Sense::hover());
    petak_lukis(ui.painter(), r, jenis, glyph, titik, ui.visuals().panel_fill);
    resp
}

/// Melukis petak lokasi di `r`; `latar` = warna di belakang petak (cincin titik status).
pub fn petak_lukis(p: &egui::Painter, r: egui::Rect, jenis: &str, glyph: char, titik: Option<Color32>, latar: Color32) {
    let sisi = r.width();
    let (bg, fg) = warna_jenis(jenis);
    p.rect_filled(r, CornerRadius::same((sisi * 0.24) as u8), bg);
    // Layanan selain Google/Microsoft: inisial ("Db", "S3", "SMB") alih-alih ikon awan.
    let huruf = crate::mesin::penyedia::cari(jenis).map_or("", |x| x.huruf);
    if huruf.is_empty() {
        p.text(r.center(), Align2::CENTER_CENTER, glyph, FontId::new(sisi * 0.56, keluarga_ikon()), fg);
    } else {
        let ukuran = if huruf.chars().count() >= 3 { 0.32 } else { 0.42 };
        p.text(r.center(), Align2::CENTER_CENTER, huruf, FontId::new(sisi * ukuran, tebal()), fg);
    }
    if let Some(w) = titik {
        let c = r.right_bottom() - vec2(1.0, 1.0);
        p.circle(c, (sisi * 0.15).max(4.0), w, Stroke::new(2.0_f32, latar));
    }
}

/// Tombol toolbar: glyph + teks tanpa bingkai; latar muncul saat disorot. `utama` = blok gelap.
pub fn tombol_alat(ui: &mut Ui, glyph: char, teks: &str, utama: bool, aktif: bool) -> egui::Response {
    let huruf = FontId::new(13.5, tebal());
    let galley = ui.painter().layout_no_wrap(teks.to_string(), huruf, TINTA);
    let lebar = 12.0 + 16.0 + 7.0 + galley.size().x + 14.0;
    let (r, resp) = ui.allocate_exact_size(vec2(lebar, 34.0), if aktif { Sense::click() } else { Sense::hover() });
    let p = ui.painter();
    let (isi, tinta) = if !aktif {
        (if utama { JALUR } else { Color32::TRANSPARENT }, ABU)
    } else if utama {
        (if resp.hovered() { TINTA_HOVER } else { TINTA }, PUTIH)
    } else if resp.is_pointer_button_down_on() {
        (JALUR, TINTA)
    } else if resp.hovered() {
        (HOVER, TINTA)
    } else {
        (Color32::TRANSPARENT, TINTA)
    };
    p.rect_filled(r, CornerRadius::same(6), isi);
    if !utama && aktif && resp.hovered() {
        p.rect_stroke(r, CornerRadius::same(6), Stroke::new(1.0_f32, GARIS), egui::StrokeKind::Inside);
    }
    let x = r.left() + 12.0;
    p.text(egui::pos2(x + 8.0, r.center().y), Align2::CENTER_CENTER, glyph, FontId::new(14.0, keluarga_ikon()), tinta);
    p.galley_with_override_text_color(egui::pos2(x + 23.0, r.center().y - galley.size().y / 2.0), galley, tinta);
    if aktif {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}

/// Teks satu baris yang dipotong dengan elipsis bila lebih dari `lebar`.
pub fn potong(ui: &Ui, teks: &str, huruf: FontId, warna: Color32, lebar: f32) -> Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::single_section(teks.to_string(), egui::TextFormat::simple(huruf, warna));
    job.wrap = egui::text::TextWrapping { max_width: lebar.max(1.0), max_rows: 1, break_anywhere: true, overflow_character: Some('\u{2026}') };
    ui.fonts(|f| f.layout_job(job))
}

/// Judul kelompok kecil berhuruf kapital (sidebar, panel).
pub fn label_kelompok(teks: &str, warna: Color32) -> RichText {
    RichText::new(teks).family(tebal()).size(11.0).color(warna).extra_letter_spacing(0.8)
}

/// Tombol kecil berisi satu glyph (segarkan, tukar, menu ⋯).
pub fn tombol_ikon(ui: &mut Ui, glyph: char, sisi: f32, aktif: bool) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(sisi, sisi), if aktif { Sense::click() } else { Sense::hover() });
    let p = ui.painter();
    if aktif && resp.hovered() {
        p.rect_filled(r, CornerRadius::same(5), if resp.is_pointer_button_down_on() { JALUR } else { HOVER });
    }
    p.text(r.center(), Align2::CENTER_CENTER, glyph, FontId::new(sisi * 0.5, keluarga_ikon()), if aktif { REDUP } else { GARIS });
    resp
}

#[cfg(windows)]
fn muat_huruf(fonts: &mut FontDefinitions, nama: &str, berkas: &str) -> bool {
    let dir = PathBuf::from(std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into())).join("Fonts");
    match std::fs::read(dir.join(berkas)) {
        Ok(b) => {
            fonts.font_data.insert(nama.into(), Arc::new(FontData::from_owned(b)));
            true
        }
        Err(_) => false,
    }
}

/// Huruf Mac pertama yang ada dari `berkas`. Berkas .ttc memuat banyak wajah: dipilih wajah
/// tegak berlebar normal dengan ketebalan terdekat ke `berat` (400 biasa, 600 semibold).
#[cfg(target_os = "macos")]
fn muat_huruf_mac(fonts: &mut FontDefinitions, nama: &str, berkas: &[&str], berat: i32) -> bool {
    for b in berkas {
        let Ok(data) = std::fs::read(b) else { continue };
        let jumlah = ttf_parser::fonts_in_collection(&data).unwrap_or(1);
        let indeks = (0..jumlah)
            .filter_map(|i| ttf_parser::Face::parse(&data, i).ok().map(|f| (i, f)))
            .filter(|(_, f)| !f.is_italic() && !f.is_oblique() && f.width() == ttf_parser::Width::Normal)
            .min_by_key(|(_, f)| {
                let w = f.weight().to_number() as i32;
                ((w - berat).abs(), w)
            })
            .map(|(i, _)| i);
        let Some(indeks) = indeks else { continue };
        let mut fd = FontData::from_owned(data);
        fd.index = indeks;
        fonts.font_data.insert(nama.into(), Arc::new(fd));
        return true;
    }
    false
}

/// (biasa, semibold, mono, simbol) berhasil dimuat; ikon memakai huruf proporsional.
#[cfg(target_os = "macos")]
fn huruf_sistem(fonts: &mut FontDefinitions) -> (bool, bool, bool, bool, Vec<String>) {
    const BIASA: &[&str] = &["/System/Library/Fonts/HelveticaNeue.ttc", "/System/Library/Fonts/Helvetica.ttc", "/System/Library/Fonts/Supplemental/Arial.ttf"];
    const TEBAL: &[&str] = &["/System/Library/Fonts/HelveticaNeue.ttc", "/System/Library/Fonts/Helvetica.ttc", "/System/Library/Fonts/Supplemental/Arial Bold.ttf"];
    let biasa = muat_huruf_mac(fonts, "segoe", BIASA, 400);
    let sb = muat_huruf_mac(fonts, "segoe-sb", TEBAL, 600);
    let mono = muat_huruf_mac(fonts, "consolas", &["/System/Library/Fonts/Menlo.ttc", "/System/Library/Fonts/Monaco.ttf"], 400);
    let simbol = muat_huruf_mac(fonts, "segoe-simbol", &["/System/Library/Fonts/Apple Symbols.ttf"], 400);
    (biasa, sb, mono, simbol, Vec::new())
}

/// Segoe UI, Consolas, Segoe UI Symbol, dan huruf ikon Windows.
#[cfg(windows)]
fn huruf_sistem(fonts: &mut FontDefinitions) -> (bool, bool, bool, bool, Vec<String>) {
    let biasa = muat_huruf(fonts, "segoe", "segoeui.ttf");
    let sb = muat_huruf(fonts, "segoe-sb", "seguisb.ttf");
    let mono = muat_huruf(fonts, "consolas", "consola.ttf");
    let simbol = muat_huruf(fonts, "segoe-simbol", "seguisym.ttf");
    // Ikon: Segoe MDL2 Assets ada sejak Windows 10; Segoe Fluent Icons (Windows 11) sebagai cadangan.
    let mut ikon = Vec::new();
    for (nama, berkas) in [("mdl2", "segmdl2.ttf"), ("fluent", "SegoeIcons.ttf")] {
        if muat_huruf(fonts, nama, berkas) {
            ikon.push(nama.to_string());
        }
    }
    (biasa, sb, mono, simbol, ikon)
}

/// Huruf sistem (Segoe UI/Consolas di Windows, Helvetica Neue/Menlo di macOS) bila ada;
/// huruf bawaan egui tetap jadi cadangan.
pub fn pasang(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let (biasa, sb, mono, simbol, mut ikon) = huruf_sistem(&mut fonts);

    let bawaan_prop = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let bawaan_mono = fonts.families.get(&FontFamily::Monospace).cloned().unwrap_or_default();
    let mut prop = Vec::new();
    if biasa {
        prop.push("segoe".to_string());
    }
    if simbol {
        prop.push("segoe-simbol".to_string());
    }
    prop.extend(bawaan_prop.iter().cloned());
    let mut tbl = Vec::new();
    if sb {
        tbl.push("segoe-sb".to_string());
    }
    tbl.extend(prop.iter().cloned());
    let mut m = Vec::new();
    if mono {
        m.push("consolas".to_string());
    }
    m.extend(bawaan_mono);
    if simbol {
        m.push("segoe-simbol".to_string());
    }
    // Keluarga "ikon" harus selalu ada (egui panik pada keluarga yang tidak terdaftar).
    ikon.extend(prop.iter().cloned());
    fonts.families.insert(keluarga_ikon(), ikon);
    fonts.families.insert(FontFamily::Proportional, prop);
    fonts.families.insert(tebal(), tbl);
    fonts.families.insert(FontFamily::Monospace, m);
    ctx.set_fonts(fonts);

    ctx.set_theme(egui::ThemePreference::Light);
    ctx.style_mut_of(egui::Theme::Light, |s| {
        s.text_styles = [
            (TextStyle::Heading, FontId::new(20.0, tebal())),
            (TextStyle::Body, FontId::proportional(14.0)),
            (TextStyle::Button, FontId::proportional(14.0)),
            (TextStyle::Small, FontId::proportional(12.5)),
            (TextStyle::Monospace, FontId::monospace(12.5)),
        ]
        .into();
        s.spacing.item_spacing = vec2(8.0, 6.0);
        s.spacing.button_padding = vec2(14.0, 6.0);
        s.spacing.interact_size = vec2(40.0, 30.0);
        s.spacing.combo_height = 320.0;

        let v = &mut s.visuals;
        *v = egui::Visuals::light();
        v.panel_fill = PUTIH;
        v.window_fill = PUTIH;
        v.extreme_bg_color = PUTIH;
        v.faint_bg_color = LATAR2;
        v.window_stroke = Stroke::new(1.0_f32, GARIS);
        v.window_corner_radius = CornerRadius::same(10);
        v.menu_corner_radius = CornerRadius::same(8);
        v.hyperlink_color = TAUTAN;
        v.selection.bg_fill = Color32::from_rgb(0xDD, 0xF4, 0xFF);
        v.selection.stroke = Stroke::new(1.0_f32, TAUTAN);
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, GARIS);
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TINTA);
        v.widgets.noninteractive.corner_radius = CornerRadius::same(6);
        for (w, isi) in [
            (&mut v.widgets.inactive, PUTIH),
            (&mut v.widgets.hovered, HOVER),
            (&mut v.widgets.active, JALUR),
            (&mut v.widgets.open, HOVER),
        ] {
            w.weak_bg_fill = isi;
            w.bg_fill = isi;
            w.bg_stroke = Stroke::new(1.0_f32, GARIS);
            w.fg_stroke = Stroke::new(1.0_f32, TINTA);
            w.corner_radius = CornerRadius::same(6);
            w.expansion = 0.0;
        }
    });
}

/// Tombol biasa: putih bergaris tepi.
pub fn tombol(teks: &str) -> Button<'static> {
    Button::new(RichText::new(teks).family(tebal()).size(13.5).color(TINTA)).min_size(vec2(0.0, 32.0))
}

/// Tombol utama: satu-satunya blok gelap di layar.
pub fn tombol_utama(teks: &str) -> Button<'static> {
    Button::new(RichText::new(teks).family(tebal()).size(14.0).color(PUTIH))
        .fill(TINTA)
        .stroke(Stroke::new(1.0_f32, TINTA))
        .min_size(vec2(0.0, 38.0))
}

pub fn kecil(teks: &str, warna: Color32) -> RichText {
    RichText::new(teks).size(12.5).color(warna)
}

/// Ikon jendela dari aset\ikon.ico (bingkai PNG 256 px).
pub fn ikon_dari_ico(ico: &[u8]) -> Option<egui::IconData> {
    eframe::icon_data::from_png_bytes(png_dari_ico(ico)?).ok()
}

/// Bingkai PNG terbesar di dalam berkas .ico (ikon Windows Vista ke atas menyimpan 256 px sebagai PNG).
pub fn png_dari_ico(ico: &[u8]) -> Option<&[u8]> {
    if ico.len() < 6 {
        return None;
    }
    let n = u16::from_le_bytes([ico[4], ico[5]]) as usize;
    let mut terbaik: Option<(u32, &[u8])> = None;
    for i in 0..n {
        let o = 6 + 16 * i;
        if o + 16 > ico.len() {
            break;
        }
        let w = if ico[o] == 0 { 256 } else { ico[o] as u32 };
        let pjg = u32::from_le_bytes([ico[o + 8], ico[o + 9], ico[o + 10], ico[o + 11]]) as usize;
        let off = u32::from_le_bytes([ico[o + 12], ico[o + 13], ico[o + 14], ico[o + 15]]) as usize;
        if off + pjg <= ico.len() && ico[off..].starts_with(b"\x89PNG") && terbaik.is_none_or(|(tw, _)| w > tw) {
            terbaik = Some((w, &ico[off..off + pjg]));
        }
    }
    terbaik.map(|(_, png)| png)
}

#[cfg(test)]
mod uji {
    /// Ikon versi macOS harus ada di salah satu huruf bawaan egui (cadangan yang selalu ikut),
    /// agar tidak tampil sebagai kotak kosong di Mac mana pun.
    #[test]
    fn ikon_unicode_ada_di_huruf_bawaan() {
        let f = eframe::egui::FontDefinitions::default();
        let wajah: Vec<ttf_parser::Face> = f.font_data.values().filter_map(|d| ttf_parser::Face::parse(&d.font, d.index).ok()).collect();
        assert!(!wajah.is_empty());
        let hilang: Vec<String> = super::ik_unicode::SEMUA
            .iter()
            .filter(|c| !wajah.iter().any(|w| w.glyph_index(**c).is_some()))
            .map(|c| format!("U+{:04X}", *c as u32))
            .collect();
        assert!(hilang.is_empty(), "tidak ada di huruf bawaan egui: {hilang:?}");
    }
}