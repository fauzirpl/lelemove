//! Jendela utama, bagian `impl App`: tampilan (toolbar, kaki, log, transfer, batas kecepatan, dialog, tutup).
use super::*;

impl App {
    // -------------------------------------------------------------- tampilan

    /// Rute (panel Dari/Ke) boleh diubah: tidak sedang menyalin dan tidak ada aksi berjalan.
    pub(crate) fn boleh_rute(&self) -> bool {
        !self.job_jalan && self.aksi.is_none()
    }

    /// Toolbar: logo, empat aksi utama, batas kecepatan.
    pub(crate) fn panel_alat(&mut self, ctx: &egui::Context) {
        let sibuk = self.aksi.is_some();
        let boleh = self.rclone.is_some() && !self.job_jalan && !sibuk;
        let (mut periksa, mut salin, mut henti, mut verif) = (false, false, false, false);
        let batas_lama = self.batas.clone();
        egui::TopBottomPanel::top("alat")
            .frame(Frame::new().fill(PUTIH).inner_margin(Margin { left: 14, right: 16, top: 10, bottom: 10 }))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Logo dan tulisan merek ada di kepala daftar lokasi (kiri).
                    ui.spacing_mut().item_spacing.x = 6.0;
                    periksa = gaya::tombol_alat(ui, ik::CARI, "Periksa", false, boleh)
                        .on_hover_text("Hitung isi sumber dan ruang di tujuan, lalu uji coba tanpa menyalin apa pun.")
                        .clicked();
                    salin = gaya::tombol_alat(ui, ik::MAIN, &self.label_salin, true, boleh)
                        .on_hover_text("Memeriksa dan menguji coba dulu, lalu meminta persetujuan Anda sebelum menyalin.")
                        .clicked();
                    henti = gaya::tombol_alat(ui, ik::HENTI, "Hentikan", false, self.job_jalan && !sibuk).clicked();
                    verif = gaya::tombol_alat(ui, ik::SELESAI, "Verifikasi", false, boleh)
                        .on_hover_text("Mencocokkan setiap file sumber dengan salinannya di tujuan. Berjalan otomatis setelah penyalinan selesai.")
                        .clicked();
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        self.pilih_batas(ui, !self.job_jalan && !sibuk);
                        ui.label(gaya::ikon(ik::SPEEDO, 15.0, REDUP)).on_hover_text("Batas kecepatan penyalinan");
                    });
                });
            });
        if self.batas != batas_lama {
            rute::simpan_batas_kecepatan(&self.batas);
            self.tulis_log(&format!("Batas kecepatan: {}.", rute::tampil_batas(&self.batas)));
        }
        if periksa {
            self.klik_periksa();
        }
        if salin {
            self.klik_salin();
        }
        if verif {
            self.klik_verifikasi();
        }
        if henti {
            self.tanya(
                "Hentikan penyalinan",
                "Hentikan penyalinan?\n\nFile yang sudah sampai tetap ada di tujuan. Klik \"Lanjutkan\" kapan saja untuk meneruskan sisanya.".into(),
                "Hentikan",
                true,
                Konfirmasi::Henti,
            );
        }
    }

    /// Bilah status: pesan terakhir, versi rclone, log, folder log, buka tujuan.
    pub(crate) fn panel_kaki(&mut self, ctx: &egui::Context) {
        let mut pilih = 0;
        egui::TopBottomPanel::bottom("kaki")
            .frame(Frame::new().fill(LATAR2).inner_margin(Margin { left: 18, right: 18, top: 7, bottom: 9 }))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 14.0;
                        if ui.add(egui::Link::new(RichText::new("Buka tujuan").size(12.5))).clicked() {
                            pilih = 1;
                        }
                        if ui.add(egui::Link::new(RichText::new("Folder log").size(12.5))).clicked() {
                            pilih = 2;
                        }
                        let t = if self.log_terbuka { "Tutup log" } else { "Log aktivitas" };
                        if ui.add(egui::Link::new(RichText::new(t).size(12.5))).clicked() {
                            pilih = 3;
                        }
                        if self.rclone.is_some() && !self.versi_rclone.is_empty() && ui.available_width() > 260.0 {
                            ui.label(kecil(&format!("rclone {}", self.versi_rclone), ABU));
                        }
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            ui.add(Label::new(RichText::new(&self.pesan.0).size(12.5).color(self.pesan.1)).truncate());
                        });
                    });
                });
            });
        match pilih {
            1 => self.klik_buka_tujuan(),
            2 => {
                let _ = std::fs::create_dir_all(&jalur().log);
                win::buka(&jalur().log.to_string_lossy());
            }
            3 => self.log_terbuka = !self.log_terbuka,
            _ => {}
        }
    }

    /// Laci log aktivitas di atas bilah status (bisa ditarik lebih tinggi).
    pub(crate) fn panel_log(&mut self, ctx: &egui::Context) {
        if !self.log_terbuka {
            return;
        }
        let mut tutup = false;
        egui::TopBottomPanel::bottom("log")
            .resizable(true)
            .default_height(150.0)
            .height_range(96.0..=380.0)
            .frame(Frame::new().fill(LATAR2).inner_margin(Margin { left: 18, right: 18, top: 10, bottom: 10 }))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(label_kelompok("LOG AKTIVITAS", ABU));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        tutup = gaya::tombol_ikon(ui, ik::TUTUP, 22.0, true, "Tutup log").on_hover_text("Tutup log").clicked();
                    });
                });
                Frame::new()
                    .fill(PUTIH)
                    .stroke(Stroke::new(1.0_f32, GARIS))
                    .corner_radius(10)
                    .inner_margin(Margin::symmetric(14, 10))
                    .show(ui, |ui| {
                        ScrollArea::vertical().stick_to_bottom(true).auto_shrink([false, false]).show(ui, |ui| {
                            let mut t = self.log.as_str();
                            ui.add(
                                TextEdit::multiline(&mut t)
                                    .font(egui::TextStyle::Monospace)
                                    .desired_width(f32::INFINITY)
                                    .frame(false),
                            );
                        });
                    });
            });
        if tutup {
            self.log_terbuka = false;
        }
    }

    /// Bilah transfer di bawah kedua panel: aksi yang berjalan, progres, hasil uji coba, verifikasi.
    pub(crate) fn panel_transfer(&mut self, ctx: &egui::Context) {
        let mut pilih = 0;
        egui::TopBottomPanel::bottom("transfer")
            .show_separator_line(false)
            .frame(Frame::new().fill(LATAR2).inner_margin(Margin { left: 14, right: 14, top: 0, bottom: 4 }))
            .show(ctx, |ui| {
              // Kartu putih di atas latar berkabut, selebar kedua panel di atasnya.
              Frame::new().fill(PUTIH).stroke(Stroke::new(1.0_f32, GARIS)).corner_radius(12).inner_margin(Margin::symmetric(18, 14)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 5.0;
                let mut ada = false;
                if let Some(a) = &self.aksi {
                    ada = true;
                    ui.horizontal(|ui| {
                        ui.add(egui::Spinner::new().size(14.0).color(TAUTAN));
                        ui.add_space(2.0);
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.spacing_mut().item_spacing.x = 14.0;
                            if ui.link("Batalkan").clicked() {
                                pilih = 1;
                            }
                            if self.url_login.is_some() && ui.link("Browser tidak terbuka? Buka halaman login").clicked() {
                                pilih = 2;
                            }
                            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                                ui.add(Label::new(RichText::new(format!("{}{ELIPSIS}", a.judul)).color(TINTA)).truncate());
                            });
                        });
                    });
                }
                if let Some(p) = &self.progres {
                    if ada {
                        ui.add_space(6.0);
                    }
                    ada = true;
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&p.judul).family(tebal()).color(TINTA));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new(format!("{}%", p.persen)).family(tebal()).color(TINTA));
                            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                                if !p.detail.is_empty() {
                                    ui.add(Label::new(RichText::new(&p.detail).size(12.5).color(REDUP)).truncate());
                                }
                            });
                        });
                    });
                    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 8.0), Sense::hover());
                    let pn = ui.painter();
                    pn.rect_filled(r, 4.0, JALUR);
                    if p.persen > 0 {
                        let mut isi = r;
                        isi.set_width((r.width() * p.persen.min(100) as f32 / 100.0).max(8.0));
                        pn.rect_filled(isi, 4.0, p.warna);
                    }
                    if !p.file.is_empty() {
                        ui.add(Label::new(kecil(&p.file, REDUP)).truncate());
                    }
                    if let Some((c, w)) = &p.catatan {
                        ui.add(Label::new(kecil(c, *w)).wrap());
                    }
                }
                if let Some(u) = self.periksa.as_ref().map(|p| &p.uji).filter(|u| !u.0.is_empty()) {
                    if ada {
                        ui.add_space(4.0);
                    }
                    ada = true;
                    ui.add(Label::new(RichText::new(&u.0).color(u.1)).wrap());
                }
                if !self.verifikasi.0.is_empty() {
                    if ada {
                        ui.add_space(4.0);
                    }
                    ada = true;
                    ui.add(Label::new(RichText::new(&self.verifikasi.0).color(self.verifikasi.1)).wrap());
                    if let Some(n) = self.pindahkan_tersedia() {
                        let t = format!("Pindahkan: hapus {} file ini dari sumber{ELIPSIS}", angka(n));
                        if ui
                            .link(kecil(&t, TAUTAN))
                            .on_hover_text(format!("Opsional. Hanya file yang terbukti sama di tujuan yang dihapus, dan masuk ke {} / Sampah sehingga masih bisa dipulihkan.", win::NAMA_SAMPAH))
                            .clicked()
                        {
                            pilih = 3;
                        }
                    }
                }
                if !ada {
                    ui.add(
                        Label::new(kecil(
                            "Buka folder yang mau disalin di panel Dari dan folder tujuan di panel Ke, lalu klik Mulai salin. \
                             Ukuran, ruang tujuan, dan uji coba diperiksa dulu. Menyalin tidak mengubah atau menghapus file di sumber.",
                            REDUP,
                        ))
                        .wrap(),
                    );
                }
              });
            });
        match pilih {
            1 => self.klik_batal(),
            2 => {
                if let Some(u) = self.url_login.clone() {
                    win::buka(&u);
                }
            }
            3 => self.klik_pindahkan(),
            _ => {}
        }
    }

    pub(crate) fn pilih_batas(&mut self, ui: &mut Ui, boleh: bool) {
        let mut pilihan: Vec<(String, String)> = PILIHAN_BATAS.iter().map(|(v, t)| (v.to_string(), t.to_string())).collect();
        if !pilihan.iter().any(|(v, _)| *v == self.batas) {
            pilihan.push((self.batas.clone(), rute::tampil_batas(&self.batas)));
        }
        let teks = pilihan.iter().find(|(v, _)| *v == self.batas).map(|p| p.1.clone()).unwrap_or_default();
        let r = ui
            .add_enabled_ui(boleh, |ui| {
                ComboBox::from_id_salt("batas_kecepatan")
                    .width(124.0)
                    .wrap_mode(egui::TextWrapMode::Truncate)
                    .selected_text(teks)
                    .show_ui(ui, |ui| {
                        ui.set_min_width(150.0);
                        for (v, t) in &pilihan {
                            ui.selectable_value(&mut self.batas, v.clone(), t);
                        }
                    })
            })
            .response;
        if boleh {
            r.on_hover_text("Membatasi kecepatan penyalinan supaya internet tetap lancar untuk keperluan lain. Berlaku saat penyalinan dimulai atau dilanjutkan.");
        } else if self.job_jalan {
            let t = "Hentikan penyalinan dulu untuk mengubah batas kecepatan, lalu klik Lanjutkan.";
            r.on_hover_text(t).on_disabled_hover_text(t);
        }
    }

    pub(crate) fn tampil_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.dialog.take() else { return };
        if let Dialog::Email(e) = &mut d {
            e.sibuk = self.aksi.is_some() && !e.berjalan();
        }
        if let Dialog::Info(i) = &mut d {
            i.sibuk = self.aksi.is_some() || self.job_jalan;
        }
        let hasil = {
            let mut p = Pemuat { tx: &self.tx, ctx: &self.ctx, gen: &mut self.gen };
            dialog::tampil(ctx, &mut d, &mut p)
        };
        match hasil {
            None => self.dialog = Some(d),
            // Dialog Migrasi email tetap terbuka saat migrasi dimulai/dihentikan.
            Some(h @ (Hasil::EmailMulai { .. } | Hasil::EmailHenti | Hasil::PembaruanCek | Hasil::PembaruanPasang)) => {
                self.dialog = Some(d);
                self.tangani_dialog(h);
            }
            Some(h) => {
                // Ditutup saat masih berjalan: simpan agar progres & hasilnya bisa dilihat lagi.
                if let Dialog::Email(mut e) = d {
                    if e.berjalan() {
                        e.lupakan_sandi();
                        self.email_latar = Some(e);
                    }
                }
                self.tangani_dialog(h);
            }
        }
    }

    pub(crate) fn urus_tutup(&mut self, ctx: &egui::Context) {
        if !ctx.input(|i| i.viewport().close_requested()) || self.boleh_tutup {
            return;
        }
        if let Some(a) = &self.aksi {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            let judul = a.judul.clone();
            if !matches!(&self.dialog, Some(Dialog::Tanya(t)) if matches!(t.tujuan, Konfirmasi::TutupApp)) {
                self.tanya(
                    "Tutup LELEMOVE",
                    format!("{judul} masih berjalan.\n\nHentikan dan tutup aplikasi?"),
                    "Hentikan & tutup",
                    true,
                    Konfirmasi::TutupApp,
                );
            }
        }
    }
}
