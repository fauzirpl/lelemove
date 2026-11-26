//! Jendela utama, bagian `impl App`: riwayat & pindahkan.
use super::*;

impl App {
    // -------------------------------------------------------------- riwayat & pindahkan

    /// Membuka riwayat; riwayat.jsonl dibaca di utas latar.
    pub(crate) fn buka_riwayat(&mut self, saring: riwayat::Saring, pesan: Option<(String, Color32)>) {
        self.gen += 1;
        let gen = self.gen;
        aksi::latar(&self.tx, &self.ctx, move || Kabar::Riwayat { gen, daftar: mr::baca(0) });
        self.dialog = Some(Dialog::Riwayat(riwayat::Riwayat { baris: None, gen, saring, pesan }));
    }

    /// Jumlah file yang bisa dipindahkan: hanya bila verifikasi terakhir untuk rute yang sedang
    /// dibuka dan tidak ada aksi/penyalinan yang berjalan.
    pub(crate) fn pindahkan_tersedia(&self) -> Option<i64> {
        let (n, _, s, t) = self.bisa_pindahkan.as_ref()?;
        (self.aksi.is_none() && !self.job_jalan && *s == self.jalur_sumber() && *t == self.jalur_tujuan()).then_some(*n)
    }

    pub(crate) fn klik_pindahkan(&mut self) {
        let Some((n, lain, _, _)) = self.bisa_pindahkan.clone() else { return };
        let s = self.jalur_sumber();
        if adalah_lokal(&s) && !win::drive_tetap(Path::new(&s)) {
            self.set_pesan(
                if cfg!(windows) {
                    "Folder sumber ada di drive jaringan atau flashdisk tanpa Recycle Bin; LELEMOVE tidak menghapus di sana."
                } else {
                    "Folder sumber ada di volume jaringan tanpa Tong Sampah; LELEMOVE tidak menghapus di sana."
                },
                KUNING,
            );
            return;
        }
        let mut isi = format!(
            "Hapus dari sumber {} file yang sudah terbukti sama di tujuan?\n\nDari:\n    {}\nSalinannya tetap di:\n    {}\n\nSebelum menghapus, LELEMOVE mencocokkan ulang setiap file; hanya yang benar-benar sama yang dihapus. Yang dihapus masuk ke {}, jadi masih bisa dipulihkan.",
            angka(n),
            self.nama_jalur(&s),
            self.nama_jalur(&self.jalur_tujuan()),
            tempat_sampah(&s)
        );
        if lain > 0 {
            isi += &format!("\n\n{} file lain belum cocok dan tidak akan dihapus.", angka(lain));
        }
        if !self.filter.kosong() {
            isi += &format!("\n\nFilter: {}.", self.filter.uraian());
        }
        self.tanya("Pindahkan", isi, "Lanjut", true, Konfirmasi::Pindahkan1);
    }

    /// Pemeriksaan/pemasangan pembaruan selesai. Bila installer sudah dijalankan (Windows), jendela
    /// ditutup agar installer bisa mengganti berkas; installer membuka LELEMOVE lagi sesudahnya.
    pub(crate) fn selesai_pembaruan(&mut self, kode: i32, h: &Value, pasang: bool) {
        let galat = teks(h.get("Galat"));
        if kode != 0 || !galat.is_empty() {
            let e = if galat.is_empty() { "Pembaruan gagal. Lihat log aktivitas.".to_string() } else { galat };
            self.set_pesan(&e, MERAH);
            self.set_pembaruan(info::Pembaruan::Galat(e));
            return;
        }
        if pasang && h.get("Dipasang").and_then(Value::as_bool) == Some(true) {
            self.set_pesan(&format!("Memperbarui ke {}: LELEMOVE ditutup dan dibuka lagi\u{2026}", teks(h.get("Terbaru"))), TINTA);
            self.tulis_log("Installer pembaruan dijalankan; jendela ditutup agar berkas bisa diganti.");
            self.boleh_tutup = true;
            self.ctx.send_viewport_cmd(ViewportCommand::Close);
            return;
        }
        if pasang && h.get("Dibuka").and_then(Value::as_bool) == Some(true) {
            self.set_pembaruan(info::Pembaruan::DmgDibuka);
            return;
        }
        if h.get("Ada").and_then(Value::as_bool) == Some(true) {
            let versi = teks(h.get("Terbaru"));
            self.set_pesan(&format!("Versi {versi} tersedia (Bantuan & info > Tentang)."), TINTA);
            self.set_pembaruan(info::Pembaruan::Ada { versi, halaman: teks(h.get("Halaman")), catatan: teks(h.get("Catatan")) });
        } else {
            self.set_pesan("LELEMOVE sudah versi terbaru.", HIJAU);
            self.set_pembaruan(info::Pembaruan::Terbaru);
        }
    }

    /// Migrasi/cadangan email selesai: hasil per folder ke dialog (terbuka atau di latar), ringkasan ke bilah status.
    pub(crate) fn selesai_email(&mut self, kode: i32, h: Value, periksa: bool) {
        let n = |k: &str| h.get(k).and_then(Value::as_i64).unwrap_or(0);
        let galat = teks(h.get("Galat"));
        let kerja = self.email_mut().map(|e| e.nama_kerja()).unwrap_or("Migrasi email");
        let (pesan, warna) = if !galat.is_empty() {
            (format!("{kerja} gagal: {galat}"), MERAH)
        } else if periksa {
            (format!("Pemeriksaan email selesai: {} email akan disalin.", angka(n("Baru"))), HIJAU)
        } else if kode == 0 {
            (format!("{kerja} selesai: {} email disalin.", angka(n("Disalin"))), HIJAU)
        } else {
            (format!("{kerja} belum tuntas: {} disalin, {} gagal.", angka(n("Disalin")), angka(n("Gagal"))), KUNING)
        };
        self.set_pesan(&pesan, warna);
        if let Some(e) = self.email_mut() {
            e.hasil = Some(h);
            e.tahap = email::Tahap::Selesai { periksa, kode };
        }
    }

    pub(crate) fn selesai_pindahkan(&mut self, kode: i32, h: &Value) {
        self.bisa_pindahkan = None;
        self.muat_panel(DARI, true);
        self.muat_panel(KE, true);
        let n = h_bil(h, "Dihapus");
        let tempat = teks(h.get("Tempat"));
        if !h_ada(h, "AkanDihapus") {
            self.verifikasi = ("Pindahkan tidak berjalan. Lihat log aktivitas.".into(), MERAH);
            self.log_terbuka = true;
            return;
        }
        if h_bil(h, "AkanDihapus") == 0 {
            self.verifikasi = ("Tidak ada file yang terbukti sama di tujuan; tidak ada yang dihapus.".into(), KUNING);
            return;
        }
        if kode == 0 {
            self.verifikasi = (format!("{CENTANG}  {} file dipindahkan. Yang dihapus dari sumber ada di {tempat}.", angka(n)), HIJAU);
            self.set_pesan("Pindahkan selesai.", HIJAU);
        } else {
            self.verifikasi = (
                format!("Hanya {} dari {} file yang terhapus dari sumber. Lihat log aktivitas.", angka(n), angka(h_bil(h, "AkanDihapus"))),
                KUNING,
            );
            self.log_terbuka = true;
        }
    }

    /// Perubahan jadwal lewat pindah.exe; daftar tetap terbuka (dengan tanda sibuk) sampai selesai.
    pub(crate) fn aksi_jadwal(&mut self, judul: &str, args: Vec<String>, ok: &str) {
        self.jalankan_aksi(judul, args, Setelah::Jadwal { ok: ok.into() });
        if self.aksi.is_some() {
            self.buka_jadwal(Some((format!("{judul}{ELIPSIS}"), REDUP)));
        }
    }

    pub(crate) fn rute_jadwal(&self, sumber: &str, tujuan: &str) -> jadwal::Rute {
        jadwal::Rute {
            sumber: sumber.into(),
            tujuan: tujuan.into(),
            nama_sumber: self.nama_jalur(sumber),
            nama_tujuan: self.nama_jalur(tujuan),
        }
    }

    /// Folder yang sedang dibuka di panel Dari dan Ke, bila keduanya sudah layak dijadwalkan.
    pub(crate) fn rute_kini(&self) -> Option<jadwal::Rute> {
        let (s, t) = (self.jalur_sumber(), self.jalur_tujuan());
        if s.is_empty() || t.is_empty() {
            return None;
        }
        if self.lok_sumber.is_empty() && (!adalah_lokal(&s) || !Path::new(&s).is_dir()) {
            return None;
        }
        if self.lok_tujuan.is_empty() && !adalah_lokal(&t) {
            return None;
        }
        Some(self.rute_jadwal(&s, &t))
    }

    /// "Backup Dokumen" dari folder sumber; "Backup OneDrive" bila seluruh isi akun.
    pub(crate) fn nama_jadwal_bawaan(&self, sumber: &str) -> String {
        let (rem, p) = pecah(sumber);
        let akhir = p.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).next().unwrap_or("").trim_end_matches(':');
        if akhir.is_empty() {
            return format!("Backup {}", nama_lokasi(self.akun_dari(&rem).as_ref()));
        }
        format!("Backup {akhir}")
    }

    /// Akun Google baru otomatis memakai client ID aplikasi bila ada (diatur di Pengaturan).
    pub(crate) fn hubungkan_baru(&mut self, jenis: &str, label: &str, izin_penuh: bool) {
        let nama = akun::remote_baru(jenis);
        let mut a = v!["hubungkan", "-Akun", nama, "-Jenis", jenis, "-Ya"];
        if !label.is_empty() {
            a.extend(v!["-Label", label]);
        }
        if izin_penuh {
            a.push("-IzinPenuh".into());
        }
        let layanan = if jenis == "google" { "Google" } else { "Microsoft" };
        let setelah = if jenis == "sharepoint" {
            Setelah::AkunSharePoint
        } else {
            Setelah::Akun { ok: "Akun terhubung.".into(), gagal: "Login tidak selesai. Coba lagi.".into() }
        };
        self.jalankan_aksi(&format!("Menunggu login {layanan} di browser"), a, setelah);
    }

    pub(crate) fn akun_aksi(&mut self, nama: &str, aksi_akun: &str) {
        let label = nama_lokasi(self.akun_dari(nama).as_ref());
        let n = nama.to_string();
        // Layanan berisian (NAS, S3, MEGA, ...): login ulang = isi ulang datanya.
        let jenis = self.akun_dari(nama).map(|a| a.jenis).unwrap_or_default();
        if aksi_akun == "ulang" && mp::cari(&jenis).is_some_and(|p| !p.khusus && !p.oauth) {
            self.buka_kredensial(&jenis, Some(n));
            return;
        }
        match aksi_akun {
            "ulang" => self.tanya(
                "Login ulang",
                format!("Login ulang ke {label}?\n\nBrowser akan terbuka untuk login."),
                "Login ulang",
                false,
                Konfirmasi::Ulang(n),
            ),
            "penuh" => self.tanya(
                "Izin penuh",
                format!("Beri {label} izin baca-tulis penuh?\n\nIzin ini dibutuhkan agar file di akun ini bisa disalin ke tempat lain. Browser akan terbuka untuk login dan menyetujui izin baru."),
                "Lanjut ke login",
                false,
                Konfirmasi::Penuh(n),
            ),
            "situs" => self.dialog = Some(Dialog::SharePoint(SharePoint::baru(&n))),
            "lepas" => self.tanya(
                "Lepas akun",
                format!("Lepas {label} dari LELEMOVE?\n\nFile di akun tersebut TIDAK terhapus; hanya sambungan di komputer ini yang dihapus."),
                "Lepas akun",
                true,
                Konfirmasi::Lepas(n),
            ),
            _ => {}
        }
    }
}
