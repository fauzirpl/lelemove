//! Jendela utama, bagian `impl App`: tampilan, daftar lokasi (kiri).
use super::*;

impl App {
    // ---- daftar lokasi (kiri)

    /// Ringkasan akun untuk daftar kiri: (keterangan singkat, warnanya, warna titik status, keterangan lengkap).
    pub(crate) fn keterangan_akun(&self, a: &Akun) -> (String, Color32, Color32, String) {
        let s = self.sambungan.get(&a.nama);
        let jenis = jenis_tampil(&a.jenis);
        let mut detail: Vec<String> = Vec::new();
        let mut singkat = String::new();
        let (mut warna, mut titik) = (REDUP, HIJAU);
        match a.jenis.as_str() {
            "google" => {
                detail.push(if a.scope == "drive" { "izin penuh" } else { "izin terbatas (hanya tujuan)" }.into());
                singkat = if a.scope == "drive" { "izin penuh" } else { "hanya tujuan" }.into();
                if a.client_aplikasi {
                    detail.push("client ID aplikasi".into());
                } else if a.client_sendiri {
                    detail.push("client ID lain (lihat Pengaturan)".into());
                } else {
                    detail.push("client ID bersama (sering dibatasi)".into());
                    singkat = "client ID bersama".into();
                    (warna, titik) = (KUNING, KUNING);
                }
            }
            "sharepoint" => {
                if a.siap {
                    detail.push(format!("pustaka {}, situs {}", a.drive, a.situs));
                    singkat = if a.situs.is_empty() { a.drive.clone() } else { a.situs.clone() };
                } else {
                    detail.push("belum memilih situs & pustaka".into());
                    singkat = "pilih situs & pustaka".into();
                    (warna, titik) = (KUNING, KUNING);
                }
            }
            _ => {
                if !a.drive.is_empty() {
                    detail.push(format!("drive {}", a.drive));
                    singkat = a.drive.clone();
                }
            }
        }
        if a.impor {
            detail.push(format!("remote rclone \"{}\" milik Anda (Lepas tidak menghapusnya)", a.nama));
        }
        let mut singkat = if singkat.is_empty() { jenis.to_string() } else { format!("{jenis} {TITIK} {singkat}") };
        let mut tambahan = String::new();
        match s {
            None if a.siap => titik = ABU,
            Some(Sambungan::Kedaluwarsa) => {
                (warna, titik) = (MERAH, MERAH);
                singkat = "login kedaluwarsa".into();
                detail.insert(0, "login kedaluwarsa".into());
            }
            Some(Sambungan::Jaringan(e)) => {
                (warna, titik) = (KUNING, KUNING);
                singkat = "jaringan terputus".into();
                detail.insert(0, "jaringan terputus, diuji ulang otomatis".into());
                tambahan = format!(
                    "\n\nInternet atau DNS sedang putus saat akun ini diuji; akunnya sendiri tidak bermasalah. \
                     LELEMOVE mengujinya lagi tiap {} detik.\n\n{e}",
                    UJI_ULANG_JARINGAN.as_secs()
                );
            }
            Some(Sambungan::Galat(e)) => {
                (warna, titik) = (KUNING, KUNING);
                singkat = "belum bisa diakses".into();
                detail.insert(0, "belum bisa diakses".into());
                tambahan = format!("\n\n{e}");
            }
            _ => {}
        }
        if a.jenis == "google" && !a.client_aplikasi {
            tambahan += if self.ada_client_bawaan {
                "\n\nAkun ini belum memakai client ID aplikasi. Buka Pengaturan > Client ID Google, lalu \"Login ulang dengan client ID ini\"."
            } else {
                "\n\nClient ID bersama rclone sering dibatasi Google, sehingga penyalinan bisa macet. Pasang client ID sendiri lewat Pengaturan > Client ID Google."
            };
        }
        let lengkap = format!("{jenis} {TITIK} {}\n{}{tambahan}", a.label, detail.join(", "));
        (singkat, warna, titik, lengkap)
    }

    /// Panel mana yang sedang membuka lokasi ini: "DARI", "KE", "DARI \u{00B7} KE", atau "".
    pub(crate) fn tanda_pakai(&self, cocok: impl Fn(usize) -> bool) -> String {
        match (cocok(DARI), cocok(KE)) {
            (true, true) => format!("DARI {TITIK} KE"),
            (true, false) => "DARI".into(),
            (false, true) => "KE".into(),
            _ => String::new(),
        }
    }

    /// Pintasan "Komputer ini" yang memuat folder panel ini (yang paling spesifik), bila ada.
    pub(crate) fn pintasan_sisi(&self, sisi: usize) -> Option<usize> {
        if !self.lok(sisi).is_empty() {
            return None;
        }
        let d = self.dalam(sisi);
        self.pintasan
            .iter()
            .enumerate()
            .filter(|(_, p)| rute::di_dalam(&d, &p.folder))
            .max_by_key(|(_, p)| p.folder.len())
            .map(|(i, _)| i)
    }

    pub(crate) fn panel_lokasi(&mut self, ctx: &egui::Context) {
        let lebar = if ctx.screen_rect().width() < 720.0 { 164.0 } else { 196.0 };
        if self.logo.is_none() {
            self.logo = gaya::png_dari_ico(IKON)
                .and_then(|png| image::load_from_memory_with_format(png, image::ImageFormat::Png).ok())
                .map(|g| {
                    let g = g.resize(64, 64, image::imageops::FilterType::Lanczos3).to_rgba8();
                    let ci = egui::ColorImage::from_rgba_unmultiplied([g.width() as usize, g.height() as usize], g.as_raw());
                    ctx.load_texture("logo", ci, egui::TextureOptions::LINEAR)
                });
        }
        let logo = self.logo.clone();
        let boleh_rute = self.boleh_rute();
        let boleh_akun = self.rclone.is_some() && !self.job_jalan && self.aksi.is_none();
        let ada = self.rclone.is_some();
        let mut buka: Option<(usize, String, Option<String>)> = None;
        let mut aksi_akun: Option<(String, &'static str)> = None;
        let mut tambah: Option<&'static str> = None;
        let mut pasang = false;
        let mut atur = false;
        let mut bantuan = false;
        let mut jadwal = false;
        let mut riwayat_ = false;
        let mut email_ = false;
        let mut cadangan_ = false;
        egui::SidePanel::left("lokasi")
            .exact_width(lebar)
            .resizable(false)
            .show_separator_line(false)
            .frame(Frame::new().fill(gaya::KOLAM_TUA).inner_margin(Margin { left: 10, right: 10, top: 14, bottom: 10 }))
            .show(ctx, |ui| {
                // Latar gelap: tautan mint, garis pemisah putih tipis, teks bawaan terang.
                let v = ui.visuals_mut();
                v.hyperlink_color = gaya::MINT;
                v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 30));
                v.override_text_color = Some(gaya::TEKS_SISI);
                // Kepala: logo dan tulisan merek.
                egui::TopBottomPanel::top("lokasi-merek")
                    .show_separator_line(false)
                    .frame(Frame::new().fill(gaya::KOLAM_TUA).inner_margin(Margin { left: 6, right: 0, top: 2, bottom: 16 }))
                    .show_inside(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            if let Some(t) = &logo {
                                ui.add(egui::Image::new((t.id(), vec2(26.0, 26.0))).corner_radius(6));
                                ui.add_space(10.0);
                            }
                            let huruf = |t: &str, w: Color32| RichText::new(t).family(tebal()).size(16.0).color(w).extra_letter_spacing(1.2);
                            ui.label(huruf("LELE", gaya::KREM));
                            ui.label(huruf("MOVE", gaya::MINT));
                        });
                    });
                // Pengaturan menempel di dasar sidebar; daftar lokasi di atasnya tetap bisa digulir.
                egui::TopBottomPanel::bottom("lokasi-bawah")
                    .frame(Frame::new().fill(gaya::KOLAM_TUA).inner_margin(Margin { left: 0, right: 0, top: 10, bottom: 0 }))
                    .show_inside(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        let r = baris_lokasi(ui, "dukung", ik::HATI, "Dukung LELEMOVE", "", REDUP, None, "", false, true);
                        if r.baris.on_hover_text(format!("LELEMOVE gratis. Bila terbantu, dukung pengembangannya lewat Saweria ({}).", pindah_drive::SAWERIA)).clicked() {
                            win::buka(pindah_drive::SAWERIA);
                        }
                        let r = baris_lokasi(ui, "pengaturan", ik::EMAIL, "Migrasi email", "", REDUP, None, "", false, true);
                        email_ = r.baris.on_hover_text("Pindahkan semua email dari Gmail, Outlook, atau server lain ke server email Anda.").clicked();
                        let r = baris_lokasi(ui, "pengaturan", ik::CADANGAN, "Cadangan email", "", REDUP, None, "", false, true);
                        cadangan_ = r
                            .baris
                            .on_hover_text("Cadangkan email ke komputer (.eml) dan pulihkan ke server, termasuk dari berkas .mbox Google Takeout.")
                            .clicked();
                        let r = baris_lokasi(ui, "pengaturan", ik::RIWAYAT, "Riwayat & laporan", "", REDUP, None, "", false, true);
                        riwayat_ = r.baris.on_hover_text("Semua penyalinan, verifikasi, dan pemindahan; laporan CSV dan siap cetak untuk audit.").clicked();
                        let r = baris_lokasi(ui, "pengaturan", ik::JADWAL, "Jadwal backup", "", REDUP, None, "", false, true);
                        jadwal = r.baris.on_hover_text(format!("Salin otomatis setiap hari atau pada hari tertentu, lewat {}.", win::PENJADWAL)).clicked();
                        let r = baris_lokasi(ui, "pengaturan", ik::BANTUAN, "Bantuan & info", "", REDUP, None, "", false, true);
                        bantuan = r.baris.on_hover_text("Tanya jawab, ketentuan, privasi, atribusi, dan tentang LELEMOVE.").clicked();
                        let r = baris_lokasi(ui, "pengaturan", ik::PENGATURAN, "Pengaturan", "", REDUP, None, "", false, true);
                        atur = r.baris.on_hover_text("Client ID Google, lanjut otomatis, dan pengaturan lain.").clicked();
                    });
                ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    if !self.status_siap {
                        ui.horizontal(|ui| {
                            ui.add_space(8.0);
                            ui.add(egui::Spinner::new().size(12.0).color(gaya::MINT));
                            ui.label(kecil(&format!("Memeriksa rclone{ELIPSIS}"), gaya::REDUP_SISI));
                        });
                        ui.add_space(8.0);
                    } else if !ada {
                        Frame::new()
                            .fill(PUTIH)
                            .stroke(Stroke::new(1.0_f32, GARIS))
                            .corner_radius(10)
                            .inner_margin(Margin::same(12))
                            .show(ui, |ui| {
                                ui.visuals_mut().override_text_color = None;
                                ui.set_width(ui.available_width());
                                ui.label(RichText::new("rclone belum terpasang").family(tebal()).size(13.0).color(MERAH));
                                ui.label(kecil("LELEMOVE menyalin memakai rclone.", REDUP));
                                ui.add_space(4.0);
                                pasang = ui.add_enabled(self.aksi.is_none(), tombol("Pasang rclone")).clicked();
                            });
                        ui.add_space(10.0);
                    }

                    // ---- Komputer ini
                    ui.horizontal(|ui| {
                        ui.add_space(10.0);
                        ui.label(label_kelompok("KOMPUTER INI", gaya::ABU_SISI));
                    });
                    ui.add_space(4.0);
                    let ps = [self.pintasan_sisi(DARI), self.pintasan_sisi(KE)];
                    let tanda = self.tanda_pakai(|s| self.lok(s).is_empty() && ps[s].is_none());
                    let sorot = self.lok(self.aktif).is_empty() && ps[self.aktif].is_none();
                    let r = baris_lokasi(ui, "lokal", ik::KOMPUTER, "Komputer ini", "", REDUP, None, &tanda, sorot, boleh_rute);
                    if r.baris.clicked() {
                        buka = Some((self.aktif, String::new(), Some(String::new())));
                    }
                    let _ = r.baris.on_hover_text("Semua drive di komputer ini");
                    for (i, p) in self.pintasan.iter().enumerate() {
                        let tanda = self.tanda_pakai(|s| ps[s] == Some(i));
                        let sorot = ps[self.aktif] == Some(i);
                        let r = baris_lokasi(ui, p.petak, p.glyph, &p.nama, "", REDUP, None, &tanda, sorot, boleh_rute);
                        if r.baris.clicked() {
                            buka = Some((self.aktif, String::new(), Some(p.folder.clone())));
                        }
                        let _ = r.baris.on_hover_text(format!("{}\n{}", p.ket, p.folder));
                    }

                    // ---- Cloud
                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        ui.add_space(10.0);
                        ui.label(label_kelompok("CLOUD", gaya::ABU_SISI));
                    });
                    ui.add_space(4.0);
                    for a in &self.akun {
                        let (singkat, warna, titik, lengkap) = self.keterangan_akun(a);
                        let tanda = self.tanda_pakai(|s| self.lok(s) == a.nama);
                        let sorot = self.lok(self.aktif) == a.nama;
                        let kedaluwarsa = self.sambungan.get(&a.nama) == Some(&Sambungan::Kedaluwarsa);
                        let r = baris_lokasi(
                            ui,
                            &a.jenis,
                            ik::AWAN,
                            &a.label,
                            &singkat,
                            warna,
                            Some(titik),
                            &tanda,
                            sorot,
                            boleh_akun,
                        );
                        let menu = r.menu.clone();
                        if r.baris.clicked() {
                            if a.siap {
                                buka = Some((self.aktif, a.nama.clone(), None));
                            } else {
                                aksi_akun = Some((a.nama.clone(), "situs"));
                            }
                        }
                        let id_menu = Id::new(("menu-akun", &a.nama));
                        if menu.as_ref().is_some_and(|m| m.clicked()) {
                            ui.memory_mut(|m| m.toggle_popup(id_menu));
                        }
                        let mut pilih: Option<&'static str> = None;
                        if let Some(m) = &menu {
                            popup_below_widget(ui, id_menu, m, PopupCloseBehavior::CloseOnClick, |ui| {
                                ui.set_width(250.0);
                                menu_akun(ui, a, &mut pilih);
                            });
                        }
                        let baris = r.baris.on_hover_text(lengkap);
                        if boleh_akun {
                            baris.context_menu(|ui| {
                                ui.set_width(250.0);
                                menu_akun(ui, a, &mut pilih);
                                if pilih.is_some() {
                                    ui.close_menu();
                                }
                            });
                        }
                        if kedaluwarsa {
                            ui.horizontal(|ui| {
                                ui.add_space(48.0);
                                if ui.add_enabled(boleh_akun, egui::Link::new(RichText::new("Login ulang").size(12.5))).clicked() {
                                    pilih = Some("ulang");
                                }
                            });
                            ui.add_space(3.0);
                        }
                        match pilih {
                            Some("dari") => buka = Some((DARI, a.nama.clone(), None)),
                            Some("ke") => buka = Some((KE, a.nama.clone(), None)),
                            Some(x) => aksi_akun = Some((a.nama.clone(), x)),
                            None => {}
                        }
                    }
                    if ada && self.status_siap && self.akun.is_empty() {
                        ui.add_space(2.0);
                        ui.horizontal(|ui| { ui.add_space(10.0); ui.add(Label::new(kecil("Belum ada akun cloud. Tambahkan Google Drive, OneDrive, SharePoint, Dropbox, NAS, S3, dan lainnya.", gaya::REDUP_SISI)).wrap()); });
                        ui.add_space(4.0);
                    }
                    let r = baris_lokasi(ui, "tambah", ik::TAMBAH, "Tambah akun", "", REDUP, None, "", false, ada && self.aksi.is_none());
                    let id_tambah = Id::new("menu-tambah-akun");
                    if r.baris.clicked() {
                        ui.memory_mut(|m| m.toggle_popup(id_tambah));
                    }
                    popup_below_widget(ui, id_tambah, &r.baris, PopupCloseBehavior::CloseOnClick, |ui| {
                        ui.set_width(240.0);
                        for (jenis, t) in [("google", "Google Drive"), ("onedrive", "OneDrive"), ("sharepoint", "SharePoint")] {
                            if item_menu(ui, Some(ik::AWAN), t, TINTA).clicked() {
                                tambah = Some(jenis);
                            }
                        }
                        ui.separator();
                        if item_menu(ui, Some(ik::TAMBAH), &format!("Layanan lain (Dropbox, NAS, S3{ELIPSIS}){ELIPSIS}"), TINTA).clicked() {
                            tambah = Some("lain");
                        }
                    });
                });
            });
        if pasang {
            self.tanya(
                "Pasang rclone",
                if cfg!(windows) {
                    "LELEMOVE memakai rclone, program open source (lisensi MIT) untuk menyalin file antar-layanan cloud.\n\nrclone akan diunduh (sekitar 25 MB) dan dipasang lewat winget dari repositori resmi. Dengan melanjutkan, Anda menyetujui ketentuan sumber dan paket winget.\n\nPasang sekarang?"
                } else {
                    "LELEMOVE memakai rclone, program open source (lisensi MIT) untuk menyalin file antar-layanan cloud.\n\nrclone akan diunduh (sekitar 25 MB) dan dipasang lewat Homebrew.\n\nPasang sekarang?"
                }
                .into(),
                "Pasang",
                false,
                Konfirmasi::PasangRclone,
            );
        }
        if let Some((sisi, lok, isi)) = buka {
            self.pilih_lokasi(sisi, &lok, isi);
        }
        if let Some((n, x)) = aksi_akun {
            self.akun_aksi(&n, x);
        }
        match tambah {
            Some("lain") => self.buka_pemilih(),
            Some(j) => self.klik_tambah(j),
            None => {}
        }
        if atur {
            self.buka_pengaturan(None);
        }
        if bantuan {
            self.buka_info(info::Tab::TanyaJawab);
        }
        if jadwal {
            self.buka_jadwal(None);
        }
        if riwayat_ {
            self.buka_riwayat(riwayat::Saring::Semua, None);
        }
        if email_ {
            self.buka_email(email::Mode::Migrasi);
        }
        if cadangan_ {
            self.buka_email(email::Mode::Cadangkan);
        }
    }

    pub(crate) fn buka_info(&mut self, tab: info::Tab) {
        let j = jalur();
        self.dialog = Some(Dialog::Info(info::Info {
            tab,
            versi_rclone: if self.versi_rclone.is_empty() { String::new() } else { format!("rclone {}", self.versi_rclone) },
            dir_app: j.dir_app.to_string_lossy().to_string(),
            dir_data: j.data.to_string_lossy().to_string(),
            dir_log: j.log.to_string_lossy().to_string(),
            pembaruan: self.pembaruan.clone(),
            sibuk: false,
        }));
    }
}
