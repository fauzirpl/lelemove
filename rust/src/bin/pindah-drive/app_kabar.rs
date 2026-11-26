//! Jendela utama, bagian `impl App`: kabar dari utas latar.
use super::*;

impl App {
    // -------------------------------------------------------------- kabar dari utas latar

    pub(crate) fn terima_kabar(&mut self) {
        while let Ok(k) = self.rx.try_recv() {
            match k {
                Kabar::Baris(b) => self.terima_baris(&b),
                Kabar::Hasil(v) => {
                    if let Some(a) = &mut self.aksi {
                        a.hasil = Some(v);
                    }
                }
                Kabar::AksiSelesai(kode) => self.aksi_selesai(kode),
                Kabar::Progres(v) => {
                    if let Some(e) = self.email_mut() {
                        email::terima_progres(e, v);
                    }
                }
                Kabar::Status { rclone, versi, akun, client_bawaan } => {
                    self.terima_status(rclone, versi, akun, client_bawaan)
                }
                Kabar::Sambungan(n, s) => {
                    if matches!(s, Sambungan::Jaringan(_)) {
                        self.uji_ulang.insert(n.clone(), Instant::now() + UJI_ULANG_JARINGAN);
                    } else {
                        self.uji_ulang.remove(&n);
                    }
                    self.sambungan.insert(n, s);
                }
                Kabar::Daftar { gen, isi } => {
                    if let Some(p) = self.panel.iter_mut().find(|p| p.gen == gen) {
                        let galat = match &isi {
                            Isi::Galat(e) => Some(e.clone()),
                            _ => None,
                        };
                        p.isi = Some(isi);
                        if let Some(e) = galat {
                            self.tulis_log(&format!("Folder tidak bisa dibuka: {}", e.trim()));
                        }
                    }
                }
                Kabar::Jelajah { gen, hasil } => {
                    if let Some(Dialog::Jelajah(j)) = &mut self.dialog {
                        if j.gen == gen {
                            j.terima(hasil);
                        }
                    }
                }
                Kabar::Situs { gen, hasil } => {
                    if let Some(Dialog::SharePoint(s)) = &mut self.dialog {
                        if s.gen == gen {
                            let mut p = Pemuat { tx: &self.tx, ctx: &self.ctx, gen: &mut self.gen };
                            s.terima_situs(hasil, &mut p);
                        }
                    }
                }
                Kabar::Pustaka { gen, hasil } => {
                    if let Some(Dialog::SharePoint(s)) = &mut self.dialog {
                        if s.gen == gen {
                            s.terima_pustaka(hasil);
                        }
                    }
                }
                Kabar::Jadwal { gen, daftar } => {
                    let baris: Vec<jadwal::Baris> = daftar
                        .into_iter()
                        .map(|(j, tugas, hidup)| {
                            let rute = self.rute_jadwal(&j.sumber, &j.tujuan);
                            jadwal::Baris { j, tugas, hidup, rute }
                        })
                        .collect();
                    if let Some(Dialog::Jadwal(d)) = &mut self.dialog {
                        if d.gen == gen {
                            d.baris = Some(baris);
                        }
                    }
                }
                Kabar::Impor { gen, calon } => {
                    if let Some(Dialog::Impor(d)) = &mut self.dialog {
                        if d.gen == gen {
                            d.calon = Some(calon);
                        }
                    }
                }
                Kabar::Riwayat { gen, daftar } => {
                    let baris: Vec<riwayat::Baris> = daftar
                        .into_iter()
                        .map(|c| riwayat::Baris { sumber: self.nama_jalur(&c.sumber), tujuan: self.nama_jalur(&c.tujuan), c })
                        .collect();
                    if let Some(Dialog::Riwayat(r)) = &mut self.dialog {
                        if r.gen == gen {
                            r.baris = Some(baris);
                        }
                    }
                }
            }
        }
    }

    /// Dialog Migrasi email yang sedang terbuka, atau yang berjalan di latar.
    pub(crate) fn email_mut(&mut self) -> Option<&mut email::Email> {
        match &mut self.dialog {
            Some(Dialog::Email(e)) => Some(e),
            _ => self.email_latar.as_mut(),
        }
    }

    /// Dialog Migrasi email (mode Migrasi) atau Cadangan email (Cadangkan/Pulihkan; arahnya
    /// mengikuti pemakaian terakhir). Penyalinan email yang sedang berjalan di latar selalu
    /// ditampilkan lebih dulu, karena hanya satu yang bisa berjalan.
    pub(crate) fn buka_email(&mut self, mode: email::Mode) {
        if let Some(e) = self.email_latar.take() {
            self.dialog = Some(Dialog::Email(e));
            return;
        }
        use pindah_drive::mesin::email as em;
        let ms = em::client_microsoft().is_some();
        let e = if mode.cadangan() {
            let mut e = email::Email::baru(&em::baca_setelan_cadangan(), mode, ms, self.ada_client_bawaan);
            // Isian server yang masih kosong dilengkapi dari migrasi terakhir.
            e.isi_dari(&em::baca_setelan());
            e
        } else {
            email::Email::baru(&em::baca_setelan(), mode, ms, self.ada_client_bawaan)
        };
        self.dialog = Some(Dialog::Email(e));
    }

    /// Keadaan pembaruan: disimpan di jendela dan, bila terbuka, di dialog Bantuan & info.
    pub(crate) fn set_pembaruan(&mut self, p: info::Pembaruan) {
        if let Some(Dialog::Info(i)) = &mut self.dialog {
            i.pembaruan = p.clone();
        }
        self.pembaruan = p;
    }

    pub(crate) fn terima_baris(&mut self, b: &str) {
        if let Some(e) = self.email_mut() {
            if e.berjalan() {
                email::terima_baris(e, b);
            }
        }
        if let info::Pembaruan::Mengunduh { versi, .. } = &self.pembaruan {
            let t = b.trim().trim_start_matches(['[', ']']).trim();
            if t.starts_with("Mengunduh") || t.starts_with("[OK]") || t.contains("SHA-256") {
                let versi = versi.clone();
                self.set_pembaruan(info::Pembaruan::Mengunduh { versi, baris: t.trim_start_matches("OK]").trim().to_string() });
            }
        }
        let t = b.trim();
        if t.is_empty() || t.starts_with("==") {
            return;
        }
        self.tulis_log(t);
        if let Some(c) = RE_LOGIN.captures(t) {
            self.url_login = Some(c[1].to_string());
        }
    }

    pub(crate) fn segarkan_status(&mut self) {
        aksi::latar(&self.tx, &self.ctx, || {
            let rc = rclone::cari_rclone();
            let versi = if rc.is_some() { rclone::versi().trim_start_matches("rclone").trim().to_string() } else { String::new() };
            Kabar::Status { rclone: rc, versi, akun: akun::daftar_akun(), client_bawaan: akun::baca_client_bawaan().is_some() }
        });
    }

    pub(crate) fn terima_status(&mut self, rc: Option<PathBuf>, versi: String, akun_baru: Vec<Akun>, client_bawaan: bool) {
        self.rclone = rc;
        self.versi_rclone = versi;
        self.akun = akun_baru;
        self.ada_client_bawaan = client_bawaan;
        self.status_siap = true;
        // Pilihan lokasi yang akunnya sudah tidak ada kembali ke "Komputer ini".
        if !self.lokasi_ada(&self.lok_sumber) {
            self.lok_sumber.clear();
            self.isi_saran_sumber();
        }
        if !self.lokasi_ada(&self.lok_tujuan) {
            self.lok_tujuan.clear();
        }
        if self.rute_siap {
            self.sumber_berubah();
            self.tujuan_berubah();
        } else {
            self.isi_rute();
        }
        let nama: Vec<String> = self.akun.iter().filter(|a| a.siap).map(|a| a.nama.clone()).collect();
        self.sambungan.clear();
        self.uji_ulang.clear();
        self.uji_akun(nama);
        // Pengaturan yang sedang terbuka ikut diperbarui (mis. setelah client ID disimpan).
        if let Some(Dialog::Pengaturan(p)) = &self.dialog {
            let pesan = p.pesan.clone();
            self.dialog = Some(Dialog::Pengaturan(self.isi_pengaturan(pesan)));
        }
        // Isian jadwal baru yang dibuka sebelum rute termuat: isi dengan folder Dari/Ke sekarang.
        let kini = self.rute_kini();
        let nama = kini.as_ref().map(|r| self.nama_jadwal_bawaan(&r.sumber)).unwrap_or_default();
        if let Some(Dialog::JadwalIsian(s)) = &mut self.dialog {
            if s.id.is_none() && s.rute.is_none() {
                s.rute = kini.clone();
                if s.nama.is_empty() {
                    s.nama = nama;
                }
            }
            s.rute_kini = kini;
        }
    }

    /// Uji sambungan akun di thread latar, satu per satu (rclone menulis token baru ke rclone.conf).
    pub(crate) fn uji_akun(&mut self, nama: Vec<String>) {
        if nama.is_empty() {
            return;
        }
        let (tx, ctx, menguji) = (self.tx.clone(), self.ctx.clone(), self.menguji.clone());
        menguji.fetch_add(1, Ordering::SeqCst);
        std::thread::spawn(move || {
            for n in nama {
                let s = akun::cek_sambungan(&n);
                if tx.send(Kabar::Sambungan(n, s)).is_err() {
                    break;
                }
                ctx.request_repaint();
            }
            menguji.fetch_sub(1, Ordering::SeqCst);
        });
    }

    /// Akun yang tadi gagal karena internet/DNS diuji lagi sendiri, tanpa perlu membuka ulang aplikasi.
    pub(crate) fn uji_ulang_jaringan(&mut self) {
        if self.uji_ulang.is_empty() || self.menguji.load(Ordering::SeqCst) > 0 {
            return;
        }
        let kini = Instant::now();
        let jatuh: Vec<String> = self.uji_ulang.iter().filter(|(_, t)| **t <= kini).map(|(n, _)| n.clone()).collect();
        for n in &jatuh {
            self.uji_ulang.remove(n);
        }
        self.uji_akun(jatuh);
    }
}
