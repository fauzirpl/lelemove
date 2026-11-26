//! Jendela utama, bagian `impl App`: tampilan, panel Dari | Ke.
use super::*;

impl App {
    // ---- panel Dari | Ke

    pub(crate) fn panel_tengah(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().frame(Frame::new().fill(LATAR2)).show(ctx, |ui| {
            // Kedua panel berupa kartu putih di atas latar berkabut, dengan celah arah salin di tengah.
            let penuh = ui.max_rect();
            let r = Rect::from_min_max(penuh.min + vec2(14.0, 14.0), penuh.max - vec2(14.0, 12.0));
            let celah = 46.0;
            let w = ((r.width() - celah) / 2.0).floor();
            let kiri = Rect::from_min_size(r.min, vec2(w, r.height()));
            let tengah = Rect::from_min_max(pos2(kiri.right(), r.top()), pos2(r.right() - w, r.bottom()));
            let kanan = Rect::from_min_max(pos2(tengah.right(), r.top()), r.max);
            // Klik di mana pun dalam panel menjadikannya panel aktif.
            if ui.input(|i| i.pointer.primary_pressed()) {
                if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                    if kiri.contains(pos) {
                        self.aktif = DARI;
                    } else if kanan.contains(pos) {
                        self.aktif = KE;
                    }
                }
            }
            self.panel_jelajah(ui, DARI, kiri);
            self.celah_tengah(ui, tengah);
            self.panel_jelajah(ui, KE, kanan);
        });
    }

    /// Lokasi panel untuk kepala panel: (jenis petak, glyph, nama, jenis lokasi, titik status).
    pub(crate) fn tampilan_lokasi(&self, sisi: usize) -> (String, char, String, String, Option<Color32>) {
        let lok = self.lok(sisi);
        let Some(a) = self.akun_dari(lok) else {
            // Di dalam folder OneDrive yang tersinkron: tampilkan sebagai folder OneDrive itu.
            if let Some(p) = self.pintasan_sisi(sisi).map(|i| &self.pintasan[i]).filter(|p| p.petak == "sinkron") {
                return (p.petak.into(), p.glyph, p.nama.clone(), "Komputer ini".into(), None);
            }
            return ("lokal".into(), ik::KOMPUTER, "Komputer ini".into(), String::new(), None);
        };
        let titik = self.keterangan_akun(&a).2;
        (a.jenis.clone(), ik::AWAN, a.label.clone(), jenis_tampil(&a.jenis), Some(titik))
    }

    /// Segmen bilah alamat: (teks, jalur bila diklik).
    pub(crate) fn segmen(&self, sisi: usize) -> Vec<(String, String)> {
        let lok = self.lok(sisi);
        let d = self.dalam(sisi);
        let mut v = Vec::new();
        if lok.is_empty() {
            v.push(("Komputer ini".to_string(), String::new()));
            let mut awalan = String::new();
            let mut acc = PathBuf::new();
            for c in Path::new(&d).components() {
                match c {
                    std::path::Component::Prefix(p) => {
                        awalan = p.as_os_str().to_string_lossy().to_string();
                        acc.push(p.as_os_str());
                    }
                    std::path::Component::RootDir => {
                        acc.push(std::path::MAIN_SEPARATOR_STR);
                        // macOS: jalur tanpa huruf drive; segmen akar memakai nama volume sistem.
                        let label = if awalan.is_empty() { win::nama_akar() } else { awalan.clone() };
                        v.push((label, acc.to_string_lossy().to_string()));
                    }
                    std::path::Component::Normal(n) => {
                        acc.push(n);
                        v.push((n.to_string_lossy().to_string(), acc.to_string_lossy().to_string()));
                    }
                    _ => {}
                }
            }
        } else {
            let nama = self.akun_dari(lok).map(|a| a.label).unwrap_or_else(|| lok.to_string());
            v.push((nama, String::new()));
            let mut acc = String::new();
            for s in d.split('/').filter(|s| !s.is_empty()) {
                if !acc.is_empty() {
                    acc.push('/');
                }
                acc.push_str(s);
                v.push((s.to_string(), acc.clone()));
            }
        }
        v
    }

    pub(crate) fn panel_jelajah(&mut self, ui: &mut Ui, sisi: usize, rect: Rect) {
        let boleh = self.boleh_rute();
        let aktif = self.aktif == sisi;
        let id = Id::new(("panel", sisi));
        let mut ui = ui.new_child(UiBuilder::new().max_rect(rect).layout(Layout::top_down(Align::Min)).id_salt(id));
        ui.set_clip_rect(rect);
        // Kartu: panel aktif bergaris tepi hijau kolam.
        let tepi = if aktif { Stroke::new(1.5_f32, gaya::KOLAM) } else { Stroke::new(1.0_f32, GARIS) };
        ui.painter().rect(rect, 12.0, PUTIH, tepi, egui::StrokeKind::Inside);
        let tinggi_kepala = 98.0;
        let kepala = Rect::from_min_size(rect.min, vec2(rect.width(), tinggi_kepala));

        // ---- baris 1: petak lokasi, label DARI/KE, nama lokasi (menu), tombol ikon
        let (jenis, glyph, nama, jenis_lok, titik) = self.tampilan_lokasi(sisi);
        let b1 = Rect::from_min_size(kepala.min + vec2(16.0, 16.0), vec2(kepala.width() - 28.0, 32.0));
        let mut ub = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_size(b1.min, vec2(32.0, 32.0))));
        gaya::petak(&mut ub, &jenis, glyph, 32.0, titik);
        let p = ui.painter().clone();
        let x_teks = b1.left() + 42.0;
        let lebar_teks = (b1.right() - 62.0 - x_teks).max(40.0);
        // Baris kecil: "DARI · Google Drive"
        let g_label = p.layout_no_wrap(
            if sisi == DARI { "DARI".into() } else { "KE".into() },
            FontId::new(10.5, tebal()),
            if aktif { gaya::KOLAM } else { ABU },
        );
        let lebar_label = g_label.size().x;
        p.galley(pos2(x_teks, b1.top() + 1.0), g_label, TINTA);
        if !jenis_lok.is_empty() {
            let g = potong(&ui, &format!("{TITIK}  {jenis_lok}"), FontId::proportional(11.5), ABU, lebar_teks - lebar_label - 6.0);
            p.galley(pos2(x_teks + lebar_label + 6.0, b1.top()), g, ABU);
        }
        let g = potong(&ui, &nama, FontId::new(14.0, tebal()), TINTA, lebar_teks - 18.0);
        let r_nama = Rect::from_min_size(pos2(x_teks - 4.0, b1.top() + 13.0), vec2(g.size().x + 24.0, 20.0));
        let resp_nama = ui.interact(r_nama, id.with("lokasi"), if boleh { Sense::click() } else { Sense::hover() });
        gaya::nama_tombol(&resp_nama, boleh, &format!("Ganti lokasi: {nama}"));
        if boleh && resp_nama.hovered() {
            p.rect_filled(r_nama, 4.0, HOVER);
        }
        p.galley(pos2(x_teks, r_nama.center().y - g.size().y / 2.0), g.clone(), TINTA);
        p.text(
            pos2(x_teks + g.size().x + 10.0, r_nama.center().y + 1.0),
            Align2::CENTER_CENTER,
            ik::BAWAH,
            FontId::new(9.0, keluarga_ikon()),
            REDUP,
        );
        let resp_nama = resp_nama.on_hover_text(if boleh {
            "Ganti lokasi panel ini"
        } else {
            "Lokasi tidak bisa diganti selama penyalinan atau pemeriksaan berjalan"
        });
        let id_menu = id.with("menu-lokasi");
        if resp_nama.clicked() {
            ui.memory_mut(|m| m.toggle_popup(id_menu));
        }
        let mut pilih_lok: Option<(String, Option<String>)> = None;
        popup_below_widget(&ui, id_menu, &resp_nama, PopupCloseBehavior::CloseOnClick, |ui| {
            ui.set_width(260.0);
            if item_menu(ui, Some(ik::KOMPUTER), "Komputer ini", TINTA).clicked() {
                pilih_lok = Some((String::new(), Some(String::new())));
            }
            for p in &self.pintasan {
                if item_menu(ui, Some(p.glyph), &p.nama, TINTA).clicked() {
                    pilih_lok = Some((String::new(), Some(p.folder.clone())));
                }
            }
            let siap: Vec<&Akun> = self.akun.iter().filter(|a| a.siap).collect();
            if !siap.is_empty() {
                ui.separator();
            }
            for a in siap {
                if item_menu(ui, Some(ik::AWAN), &nama_lokasi(Some(a)), TINTA).clicked() {
                    pilih_lok = Some((a.nama.clone(), None));
                }
            }
        });
        // tombol ikon di kanan
        let lokal = self.lok(sisi).is_empty();
        let r_seg = Rect::from_center_size(pos2(b1.right() - 13.0, b1.center().y), vec2(26.0, 26.0));
        let r_tel = Rect::from_center_size(pos2(b1.right() - 41.0, b1.center().y), vec2(26.0, 26.0));
        let segarkan = {
            let mut u = ui.new_child(UiBuilder::new().max_rect(r_seg));
            gaya::tombol_ikon(&mut u, ik::SEGARKAN, 26.0, true, "Muat ulang daftar folder").on_hover_text("Muat ulang daftar folder").clicked()
        };
        let telusuri = {
            let mut u = ui.new_child(UiBuilder::new().max_rect(r_tel));
            gaya::tombol_ikon(&mut u, ik::BUKA_MAP, 26.0, boleh, "Pilih folder")
                .on_hover_text(if lokal { format!("Pilih folder lewat jendela {} (bisa membuat folder baru)", win::NAMA_OS) } else { "Pilih folder di jendela terpisah".to_string() })
                .clicked()
        };

        // ---- baris 2: naik + bilah alamat
        let b2 = Rect::from_min_size(kepala.min + vec2(14.0, 58.0), vec2(kepala.width() - 28.0, 28.0));
        let r_naik = Rect::from_min_size(b2.min, vec2(28.0, 28.0));
        let naik = {
            let mut u = ui.new_child(UiBuilder::new().max_rect(r_naik));
            gaya::tombol_ikon(&mut u, ik::NAIK, 28.0, boleh && self.bisa_naik(sisi), "Naik satu tingkat").on_hover_text("Naik satu tingkat").clicked()
        };
        let r_alamat = Rect::from_min_max(pos2(b2.left() + 32.0, b2.top()), b2.max);
        let alamat = self.bilah_alamat(&mut ui, sisi, r_alamat, boleh);

        // ---- daftar folder + kaki panel
        let garis_atas = kepala.bottom();
        p.line_segment([pos2(rect.left() + 1.5, garis_atas), pos2(rect.right() - 1.5, garis_atas)], Stroke::new(1.0_f32, GARIS_HALUS));
        let tinggi_kaki = self.tinggi_kaki(&ui, sisi, rect.width());
        let daftar = Rect::from_min_max(pos2(rect.left(), garis_atas + 1.0), pos2(rect.right(), rect.bottom() - tinggi_kaki));
        let kaki = Rect::from_min_max(pos2(rect.left(), daftar.bottom()), rect.max);
        let mut ud = ui.new_child(UiBuilder::new().max_rect(daftar.shrink2(vec2(8.0, 6.0))).layout(Layout::top_down(Align::Min)));
        let (masuk, ke_atas, coba_lagi, login) = self.daftar_folder(&mut ud, sisi, boleh);
        p.line_segment([kaki.left_top() + vec2(1.5, 0.0), kaki.right_top() - vec2(1.5, 0.0)], Stroke::new(1.0_f32, GARIS_HALUS));
        let mut uk = ui.new_child(UiBuilder::new().max_rect(kaki).layout(Layout::top_down(Align::Min)));
        if self.kaki_panel(&mut uk, sisi) {
            self.dialog = Some(Dialog::Filter(filter::FilterIsian::dari(&self.filter)));
        }

        // ---- tindakan (setelah semua digambar)
        if let Some((lok, isi)) = pilih_lok {
            self.pilih_lokasi(sisi, &lok, isi);
        } else if let Some(j) = alamat {
            self.set_isi(sisi, j);
        } else if naik || ke_atas {
            self.naik(sisi);
        } else if let Some(n) = masuk {
            self.buka_sub(sisi, &n);
        } else if segarkan || coba_lagi {
            self.muat_panel(sisi, true);
        } else if telusuri {
            self.klik_telusuri(sisi == DARI);
        } else if login {
            let n = self.lok(sisi).to_string();
            self.akun_aksi(&n, "ulang");
        }
    }

    /// Bilah alamat: segmen yang bisa diklik; klik ruang kosongnya untuk mengetik alamat.
    pub(crate) fn bilah_alamat(&mut self, ui: &mut Ui, sisi: usize, r: Rect, boleh: bool) -> Option<String> {
        let id = Id::new(("alamat", sisi));
        let lokal = self.lok(sisi).is_empty();
        if let Some(mut t) = self.panel[sisi].edit.take() {
            let mut u = ui.new_child(UiBuilder::new().max_rect(r));
            let te = u.add(
                TextEdit::singleline(&mut t)
                    .id(id.with("edit"))
                    .desired_width(r.width())
                    .min_size(r.size())
                    .vertical_align(Align::Center)
                    .margin(Margin::symmetric(12, 4))
                    .hint_text(if lokal { win::CONTOH_DATA } else { "Folder/Subfolder" }),
            );
            if self.panel[sisi].fokus_edit {
                te.request_focus();
                self.panel[sisi].fokus_edit = false;
            }
            let enter = te.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if enter {
                let t = t.trim().trim_matches('"');
                return Some(if lokal { t.to_string() } else { t.replace('\\', "/").trim_matches('/').to_string() });
            }
            if !te.lost_focus() && !ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.panel[sisi].edit = Some(t);
            }
            return None;
        }

        let resp = ui.interact(r, id, if boleh { Sense::click() } else { Sense::hover() });
        gaya::nama_tombol(&resp, boleh, "Alamat folder (klik untuk mengetik)");
        let p = ui.painter().clone();
        p.rect(r, 6.0, LATAR2, Stroke::new(1.0_f32, if boleh && resp.hovered() { ABU } else { GARIS }), StrokeKind::Inside);
        let seg = self.segmen(sisi);
        let huruf = FontId::proportional(13.0);
        let huruf_akhir = FontId::new(13.0, tebal());
        let galley: Vec<_> = seg
            .iter()
            .enumerate()
            .map(|(i, (t, _))| {
                let akhir = i + 1 == seg.len();
                p.layout_no_wrap(t.clone(), if akhir { huruf_akhir.clone() } else { huruf.clone() }, if akhir { TINTA } else { REDUP })
            })
            .collect();
        // Semua segmen bila muat; kalau tidak, segmen dari belakang yang muat dan sisanya diganti "…".
        let pemisah = 18.0;
        let ruang = r.width() - 16.0;
        let total: f32 = galley.iter().map(|g| g.size().x).sum::<f32>() + pemisah * (galley.len().saturating_sub(1)) as f32;
        let mut mulai = 0;
        if total > ruang {
            let tersedia = ruang - 14.0 - pemisah; // tempat "…" dan pemisahnya
            mulai = galley.len() - 1;
            let mut lebar = galley[mulai].size().x;
            while mulai > 0 && lebar + pemisah + galley[mulai - 1].size().x <= tersedia {
                lebar += pemisah + galley[mulai - 1].size().x;
                mulai -= 1;
            }
        }
        let mut x = r.left() + 8.0;
        let cy = r.center().y;
        if mulai > 0 {
            p.text(pos2(x, cy), Align2::LEFT_CENTER, ELIPSIS, huruf.clone(), REDUP);
            x += 14.0;
        }
        let mut hasil = None;
        for (i, g) in galley.iter().enumerate().skip(mulai) {
            if i > 0 {
                p.text(pos2(x + 9.0, cy + 1.0), Align2::CENTER_CENTER, ik::KANAN, FontId::new(8.0, keluarga_ikon()), ABU);
                x += pemisah;
            }
            let lebar_g = g.size().x.min(r.right() - 8.0 - x).max(0.0);
            let rs = Rect::from_min_size(pos2(x - 3.0, r.top() + 3.0), vec2(lebar_g + 6.0, r.height() - 6.0));
            let akhir = i + 1 == galley.len();
            if !akhir && boleh {
                let rr = ui.interact(rs, id.with(i), Sense::click());
                gaya::nama_tombol(&rr, true, g.text());
                if rr.hovered() {
                    p.rect_filled(rs, 4.0, JALUR);
                }
                if rr.clicked() {
                    hasil = Some(seg[i].1.clone());
                }
            }
            p.with_clip_rect(r.shrink(2.0)).galley(pos2(x, cy - g.size().y / 2.0), g.clone(), TINTA);
            x += lebar_g;
        }
        if hasil.is_none() && resp.clicked() {
            let d = self.dalam(sisi);
            self.panel[sisi].edit = Some(if lokal { d } else if d.is_empty() { String::new() } else { format!("/{d}") });
            self.panel[sisi].fokus_edit = true;
        }
        if boleh {
            resp.on_hover_text("Klik nama folder untuk kembali ke sana, atau klik ruang kosong untuk mengetik alamat");
        }
        hasil
    }

    /// Daftar subfolder. Hasil: (folder yang dibuka, naik, coba lagi, login ulang).
    pub(crate) fn daftar_folder(&mut self, ui: &mut Ui, sisi: usize, boleh: bool) -> (Option<String>, bool, bool, bool) {
        let mut masuk = None;
        let mut naik = false;
        let mut coba = false;
        let mut login = false;
        let bisa_naik = self.bisa_naik(sisi);
        let kedaluwarsa = self.sambungan.get(self.lok(sisi)) == Some(&Sambungan::Kedaluwarsa);
        let putus = matches!(self.sambungan.get(self.lok(sisi)), Some(Sambungan::Jaringan(_)));
        let lokal = self.lok(sisi).is_empty();
        let mut pilih = self.panel[sisi].pilih;
        ScrollArea::vertical().id_salt(("daftar", sisi)).auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            if bisa_naik {
                let r = baris_folder(ui, ik::NAIK, ABU, "..", false, boleh);
                if r.double_clicked() {
                    naik = true;
                }
            }
            match &self.panel[sisi].isi {
                None => {
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        ui.add(egui::Spinner::new().size(14.0).color(ABU));
                        ui.label(kecil(&format!("Memuat folder{ELIPSIS}"), REDUP));
                    });
                    if putus {
                        // rclone terus mencoba ulang selama internet putus, jadi pemuatan bisa lama.
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.add_space(12.0);
                            ui.add(
                                Label::new(kecil(
                                    "Internet sedang putus, jadi pemuatan bisa lama. Akun diuji ulang otomatis; klik \u{21BB} di kepala panel untuk memuat lagi.",
                                    KUNING,
                                ))
                                .wrap(),
                            );
                        });
                    }
                }
                Some(Isi::Drive(d)) => {
                    for (i, (akar, label)) in d.iter().enumerate() {
                        let r = baris_folder(ui, ik::DISK, REDUP, label, pilih == Some(i), boleh);
                        if r.clicked() {
                            pilih = Some(i);
                        }
                        if r.double_clicked() {
                            masuk = Some(akar.clone());
                        }
                    }
                }
                Some(Isi::Folder(d)) => {
                    for (i, n) in d.iter().enumerate() {
                        let r = baris_folder(ui, ik::MAP, MAP, n, pilih == Some(i), boleh);
                        if r.clicked() {
                            pilih = Some(i);
                        }
                        if r.double_clicked() {
                            masuk = Some(n.clone());
                        }
                    }
                    if d.is_empty() {
                        pesan_kosong(ui, ik::MAP, "Tidak ada subfolder", if sisi == DARI { "Seluruh isi folder ini yang disalin." } else { "" }, REDUP);
                    }
                }
                Some(Isi::BelumAda) => {
                    if sisi == KE {
                        pesan_kosong(ui, ik::MAP, "Folder ini belum ada", "Dibuat otomatis saat penyalinan dimulai.", REDUP);
                    } else {
                        pesan_kosong(ui, ik::AWAS, "Folder tidak ditemukan", "Naik satu tingkat atau pilih folder lain.", MERAH);
                    }
                }
                Some(Isi::Galat(e)) => {
                    let (judul, ket) = ringkas_galat(e);
                    if kedaluwarsa || e.contains("invalid_grant") {
                        pesan_kosong(ui, ik::GALAT, "Login akun ini kedaluwarsa", "Login ulang untuk membuka foldernya.", MERAH);
                    } else {
                        pesan_kosong(ui, ik::GALAT, judul, &ket, MERAH);
                    }
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        if kedaluwarsa && !lokal {
                            login = ui.add_enabled(boleh, tombol("Login ulang")).clicked();
                        }
                        coba = ui.add(tombol("Coba lagi")).clicked();
                    });
                }
            }
        });
        self.panel[sisi].pilih = pilih;
        (masuk, naik, coba, login)
    }

    /// Tinggi kaki panel (diukur dari teks yang akan ditampilkan, dibungkus selebar panel).
    pub(crate) fn tinggi_kaki(&self, ui: &Ui, sisi: usize, lebar: f32) -> f32 {
        // Harus sama dengan jarak di `kaki_panel`: 16 kiri-kanan, 11 atas, 13 bawah.
        let w = lebar - 32.0;
        let mut h = 11.0 + 13.0;
        let mut tambah = |t: &str, huruf: FontId| {
            if !t.is_empty() {
                h += ui.fonts(|f| f.layout(t.to_string(), huruf, TINTA, w)).size().y + 2.0;
            }
        };
        let (judul, ket) = self.ringkasan_kaki(sisi);
        tambah(&judul.0, FontId::new(13.5, tebal()));
        tambah(&ket.0, FontId::proportional(12.5));
        tambah(&self.info_kaki(sisi).0, FontId::proportional(12.5));
        if sisi == DARI {
            // Baris filter (selalu satu baris, dipotong bila panjang).
            h += 20.0;
        }
        h.max(44.0)
    }

    /// Baris tebal dan keterangannya di kaki panel: hasil Periksa, atau jumlah isi folder.
    pub(crate) fn ringkasan_kaki(&self, sisi: usize) -> (Teks, Teks) {
        if let Some(p) = &self.periksa {
            return if sisi == DARI {
                (p.hasil_sumber.clone(), p.ket_sumber.clone())
            } else {
                (p.hasil_tujuan.clone(), p.ket_tujuan.clone())
            };
        }
        let jumlah = match &self.panel[sisi].isi {
            Some(Isi::Folder(d)) if !d.is_empty() => format!("{} subfolder {TITIK} klik dua kali untuk membuka", angka(d.len() as i64)),
            Some(Isi::Drive(d)) => format!("{} drive {TITIK} klik dua kali untuk membuka", d.len()),
            _ => String::new(),
        };
        ((jumlah, REDUP), kosong())
    }

    /// Kaki panel. Hasil: tautan filter (panel Dari) diklik.
    pub(crate) fn kaki_panel(&self, ui: &mut Ui, sisi: usize) -> bool {
        let mut atur_filter = false;
        Frame::new().inner_margin(Margin { left: 16, right: 16, top: 11, bottom: 13 }).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 2.0;
            if sisi == DARI {
                // Tinggi baris dikunci supaya cocok dengan perhitungan `tinggi_kaki` (18 + 2).
                ui.spacing_mut().interact_size.y = 18.0;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    let boleh = self.boleh_rute();
                    if self.filter.kosong() {
                        ui.label(kecil("Semua jenis file", REDUP));
                        let r = ui.add_enabled(boleh, egui::Link::new(kecil(&format!("Filter{ELIPSIS}"), TAUTAN)));
                        atur_filter = r.on_hover_text("Hanya jenis file tertentu, lewati pola tertentu, atau hanya yang diubah sejak tanggal tertentu.").clicked();
                    } else {
                        let r = ui.add_enabled(boleh, egui::Link::new(kecil("Ubah filter", TAUTAN)));
                        atur_filter = r.clicked();
                        let t = format!("Filter: {}", self.filter.uraian());
                        ui.add(Label::new(kecil(&t, KUNING)).truncate()).on_hover_text(&t);
                    }
                });
            }
            let (judul, ket) = self.ringkasan_kaki(sisi);
            if !judul.0.is_empty() {
                let tebal_ = self.periksa.is_some();
                let rt = RichText::new(&judul.0).size(if tebal_ { 13.5 } else { 12.5 }).color(judul.1);
                ui.add(Label::new(if tebal_ { rt.family(tebal()) } else { rt }).wrap());
            }
            if !ket.0.is_empty() {
                ui.add(Label::new(kecil(&ket.0, ket.1)).wrap());
            }
            let info = self.info_kaki(sisi);
            if !info.0.is_empty() {
                ui.add(Label::new(kecil(&info.0, info.1)).wrap());
            }
        });
        atur_filter
    }

    /// Keterangan terakhir di kaki panel; dengan filter, "Seluruh isi ... disalin" tidak lagi benar.
    pub(crate) fn info_kaki(&self, sisi: usize) -> Teks {
        let info = if sisi == DARI { &self.info_sumber } else { &self.info_tujuan };
        if sisi == DARI && !self.filter.kosong() && info.0.starts_with("Seluruh isi") {
            return ("Hanya file yang lolos filter yang disalin.".into(), info.1);
        }
        info.clone()
    }

    /// Celah di antara kedua panel: panah arah salin dan tombol tukar.
    pub(crate) fn celah_tengah(&mut self, ui: &mut Ui, r: Rect) {
        let p = ui.painter().clone();
        // Lencana arah salin: lingkaran hijau kolam dengan panah putih, sejajar kepala kedua kartu.
        let c = pos2(r.center().x, r.top() + 16.0 + 16.0);
        p.circle_filled(c, 14.0, gaya::KOLAM);
        p.text(c, Align2::CENTER_CENTER, ik::PANAH, FontId::new(12.0, keluarga_ikon()), PUTIH);
        let rt = Rect::from_center_size(pos2(r.center().x, r.top() + 58.0 + 14.0), vec2(26.0, 26.0));
        let boleh = self.boleh_rute();
        let mut u = ui.new_child(UiBuilder::new().max_rect(rt));
        let resp = gaya::tombol_ikon(&mut u, ik::TUKAR, 26.0, boleh, "Tukar sumber dan tujuan").on_hover_text("Tukar sumber \u{21C4} tujuan");
        if resp.clicked() {
            self.klik_tukar();
        }
    }
}
