//! Jendela utama, bagian `impl App`: aksi (pindah.exe).
use super::*;

impl App {
    // -------------------------------------------------------------- aksi (pindah.exe)

    pub(crate) fn jalankan_aksi(&mut self, judul: &str, args: Vec<String>, setelah: Setelah) {
        self.jalankan_aksi_masukan(judul, args, None, setelah);
    }

    /// Aksi dengan masukan stdin (isian rahasia sebagai JSON; tidak pernah lewat argumen atau log).
    pub(crate) fn jalankan_aksi_masukan(&mut self, judul: &str, args: Vec<String>, masukan: Option<String>, setelah: Setelah) {
        if self.aksi.is_some() {
            return;
        }
        match aksi::mulai_masukan(&self.exe_mesin, &args, masukan, self.tx.clone(), self.ctx.clone()) {
            Ok(pid) => {
                self.aksi = Some(Aksi { judul: judul.into(), pid, hasil: None, setelah, dibatalkan: false });
                self.tulis_log(&format!("--- {judul}"));
                self.set_pesan(&format!("{judul}{ELIPSIS}"), TINTA);
            }
            Err(e) => {
                self.set_pesan(&format!("{MESIN} tidak bisa dijalankan: {e}"), MERAH);
                self.tulis_log(&format!("Kesalahan: {MESIN} tidak bisa dijalankan: {e}"));
                self.salin_setelah_uji = false;
            }
        }
    }

    pub(crate) fn klik_batal(&mut self) {
        if let Some(a) = &mut self.aksi {
            a.dibatalkan = true;
            win::bunuh_pohon(a.pid);
        }
        if self.aksi.is_some() {
            self.tulis_log("Dibatalkan oleh pengguna.");
        }
    }

    pub(crate) fn aksi_selesai(&mut self, kode: i32) {
        let Some(a) = self.aksi.take() else { return };
        self.url_login = None;
        if a.dibatalkan {
            if let Setelah::Pembaruan { .. } = a.setelah {
                self.set_pembaruan(info::Pembaruan::Galat("Dibatalkan.".into()));
            }
            if let Setelah::Email { periksa } = a.setelah {
                if let Some(e) = self.email_mut() {
                    e.tahap = email::Tahap::Selesai { periksa, kode: -1 };
                }
            }
            self.salin_setelah_uji = false;
            self.set_pesan("Dibatalkan.", REDUP);
            self.segarkan_status();
            return;
        }
        let h = a.hasil.unwrap_or(Value::Null);
        match a.setelah {
            Setelah::Pasang => self.selesai_akun(
                kode,
                "rclone terpasang. Lanjutkan dengan menambah akun.",
                "Pemasangan rclone gagal. Lihat log aktivitas.",
            ),
            Setelah::Akun { ok, gagal } => self.selesai_akun(kode, &ok, &gagal),
            Setelah::AkunSharePoint => {
                self.selesai_akun(kode, "Login berhasil. Sekarang pilih situs SharePoint-nya.", "Login tidak selesai. Coba lagi.");
                let n = teks(h.get("Akun"));
                if kode == 0 && !n.is_empty() {
                    self.dialog = Some(Dialog::SharePoint(SharePoint::baru(&n)));
                }
            }
            Setelah::Periksa => self.selesai_periksa(kode, &h),
            Setelah::Uji => self.selesai_uji(kode, &h),
            Setelah::Salin => {
                if kode == 0 {
                    self.set_pesan("Penyalinan berjalan di latar belakang.", HIJAU);
                    // Hasil periksa/uji coba sudah basi; "Salin lagi"/"Lanjutkan" memeriksa ulang.
                    self.periksa = None;
                    self.ringkas_uji = None;
                    self.sudah_diperiksa = false;
                } else {
                    self.set_pesan("Penyalinan gagal dimulai. Lihat log aktivitas.", MERAH);
                    self.log_terbuka = true;
                }
                self.perbarui_progres();
            }
            Setelah::Verifikasi => self.selesai_verifikasi(kode, &h),
            Setelah::Pindahkan => self.selesai_pindahkan(kode, &h),
            Setelah::AkunPenyedia { nama, kunci } => self.selesai_akun_penyedia(kode, &h, &nama, &kunci),
            Setelah::Email { periksa } => self.selesai_email(kode, h, periksa),
            Setelah::Pembaruan { pasang } => self.selesai_pembaruan(kode, &h, pasang),
            Setelah::Jadwal { ok } => {
                if kode == 0 {
                    self.set_pesan(&ok, HIJAU);
                    self.buka_jadwal(Some((ok, HIJAU)));
                } else {
                    // Pesan galat terakhir dari pindah.exe (sudah masuk log aktivitas).
                    let e = self
                        .log
                        .lines()
                        .rev()
                        .map(str::trim)
                        .find(|b| !b.is_empty() && !b.starts_with("---"))
                        .unwrap_or("Lihat log aktivitas.")
                        .to_string();
                    self.set_pesan("Jadwal gagal diubah. Lihat log aktivitas.", MERAH);
                    self.buka_jadwal(Some((e, MERAH)));
                }
            }
        }
    }

    pub(crate) fn selesai_akun(&mut self, kode: i32, ok: &str, gagal: &str) {
        self.segarkan_status();
        if kode == 0 {
            self.set_pesan(ok, HIJAU);
        } else {
            self.set_pesan(gagal, MERAH);
            self.log_terbuka = true;
        }
    }
}
