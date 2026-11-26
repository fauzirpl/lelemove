//! Jendela utama, bagian `impl App`: periksa & uji.
use super::*;

impl App {
    // -------------------------------------------------------------- periksa & uji

    pub(crate) fn klik_periksa(&mut self) {
        if self.ambil_isian().is_none() {
            self.salin_setelah_uji = false;
            return;
        }
        self.periksa = Some(Periksa {
            hasil_sumber: (ELIPSIS.into(), TINTA),
            ket_sumber: kosong(),
            hasil_tujuan: (ELIPSIS.into(), TINTA),
            ket_tujuan: kosong(),
            uji: kosong(),
        });
        self.jalankan_aksi("Memeriksa isi sumber dan ruang tujuan", v!["periksa"], Setelah::Periksa);
    }

    /// Hasil periksa/uji/verifikasi melaporkan login kedaluwarsa: tandai akunnya.
    pub(crate) fn catat_perlu_login(&mut self, h: &Value) -> Option<String> {
        let n = teks(h.get("PerluLogin"));
        if n.is_empty() {
            return None;
        }
        self.sambungan.insert(n.clone(), Sambungan::Kedaluwarsa);
        Some(nama_lokasi(self.akun_dari(&n).as_ref()))
    }

    pub(crate) fn selesai_periksa(&mut self, kode: i32, h: &Value) {
        let perlu_login = self.catat_perlu_login(h);
        let mut p = self.periksa.take().unwrap_or_default();
        if h_ada(h, "SumberJumlah") {
            p.hasil_sumber = (
                format!("{} file  {TITIK}  {}", angka(h_bil(h, "SumberJumlah")), ukuran(h_bil(h, "SumberBytes") as f64)),
                TINTA,
            );
            let mut ket = String::new();
            if h_bil(h, "HanyaCloudJumlah") > 0 {
                ket = format!(
                    "{} file masih \"hanya online\"; OneDrive mengunduhnya dulu ke disk (sisa {}).",
                    angka(h_bil(h, "HanyaCloudJumlah")),
                    ukuran(h_bil(h, "DiskBebas") as f64)
                );
            }
            p.ket_sumber = if h_bool(h, "DiskCukup", false) {
                (format!("{ket} Ruang disk tidak cukup; salin per subfolder."), KUNING)
            } else {
                (ket, REDUP)
            };
        } else {
            p.hasil_sumber = ("Tidak terbaca".into(), MERAH);
            p.ket_sumber = match &perlu_login {
                Some(l) => (format!("Login {l} kedaluwarsa. Klik Login ulang pada akun itu di daftar kiri."), MERAH),
                None => ("Lihat log aktivitas.".into(), REDUP),
            };
        }

        if h_ada(h, "TujuanBebas") {
            let w = if h_bool(h, "TujuanCukup", false) { MERAH } else { TINTA };
            p.hasil_tujuan = (format!("Sisa {}", ukuran(h_bil(h, "TujuanBebas") as f64)), w);
            let mut ket = if !teks(h.get("TujuanJenis")).is_empty() {
                format!("dari {}. Perlu disalin {}.", ukuran(h_bil(h, "TujuanTotal") as f64), ukuran(h_bil(h, "Butuh") as f64))
            } else {
                format!("ruang disk. Perlu disalin {}.", ukuran(h_bil(h, "Butuh") as f64))
            };
            if h_bil(h, "TujuanSudah") > 0 {
                ket += &format!(" {} sudah ada di tujuan.", ukuran(h_bil(h, "TujuanSudah") as f64));
            }
            if h_bool(h, "TujuanCukup", false) {
                ket += " Tidak cukup: kosongkan ruang/sampah di tujuan atau pilih subfolder.";
            }
            p.ket_tujuan = (ket, REDUP);
        } else if h_bool(h, "TujuanTerhubung", true) {
            p.hasil_tujuan = ("Kuota tidak dibatasi".into(), TINTA);
            p.ket_tujuan = kosong();
        } else if h_ada(h, "TujuanGalat") {
            p.hasil_tujuan = ("Tidak bisa diakses".into(), MERAH);
            p.ket_tujuan = match &perlu_login {
                Some(l) => (format!("Login {l} kedaluwarsa. Klik Login ulang pada akun itu di daftar kiri."), MERAH),
                None => ("Coba \"Login ulang\" lewat menu \u{22EF} akun itu di daftar kiri.".into(), REDUP),
            };
        } else {
            p.hasil_tujuan = ("Belum diperiksa".into(), REDUP);
            p.ket_tujuan = ("Lihat log aktivitas.".into(), REDUP);
        }

        if (kode == 0 || kode == 3) && h_bool(h, "TujuanTerhubung", true) {
            p.uji = (format!("Uji coba{ELIPSIS}"), REDUP);
            self.periksa = Some(p);
            self.jalankan_aksi("Uji coba (tidak menyalin apa pun)", v!["uji"], Setelah::Uji);
            return;
        }
        self.salin_setelah_uji = false;
        if kode == 2 {
            p.uji = ("Ruang di tujuan tidak cukup untuk folder ini.".into(), MERAH);
            self.set_pesan("Ruang di tujuan tidak cukup.", MERAH);
        } else {
            p.uji = ("Pemeriksaan belum lengkap. Lihat log aktivitas.".into(), KUNING);
            self.log_terbuka = true;
            self.set_pesan("Pemeriksaan selesai dengan catatan.", KUNING);
        }
        self.periksa = Some(p);
    }

    pub(crate) fn set_uji(&mut self, t: Teks) {
        if let Some(p) = &mut self.periksa {
            p.uji = t;
        }
    }

    pub(crate) fn selesai_uji(&mut self, kode: i32, h: &Value) {
        let perlu_login = self.catat_perlu_login(h);
        if kode == 1 || h.is_null() {
            self.salin_setelah_uji = false;
            let t = match perlu_login {
                Some(l) => format!("Uji coba gagal: login {l} kedaluwarsa. Klik Login ulang pada akun itu."),
                None => "Uji coba gagal. Lihat log aktivitas.".into(),
            };
            self.set_uji((t, MERAH));
            self.log_terbuka = true;
            return;
        }
        self.ringkas_uji = Some(h.clone());
        let akan = h_bil(h, "AkanDisalin");
        let mut w = HIJAU;
        let mut t = if akan == 0 {
            format!("{CENTANG}  Semua file sudah ada di tujuan.")
        } else {
            format!("{CENTANG}  Siap: {} file akan disalin ({}).", angka(akan), ukuran(h_bil(h, "AkanBytes") as f64))
        };
        if h_bil(h, "Terlewat") > 0 {
            t = format!(
                "{} file tidak bisa dibaca dan akan terlewat. Coba pakai akun OneDrive (lewat internet) sebagai sumber.",
                angka(h_bil(h, "Terlewat"))
            );
            w = KUNING;
        } else if h_bil(h, "Masalah") > 0 {
            t += &format!(" Ada {} catatan masalah; lihat log aktivitas.", angka(h_bil(h, "Masalah")));
            w = KUNING;
        }
        self.set_uji((t, w));
        self.sudah_diperiksa = true;
        self.set_pesan("Pemeriksaan selesai.", REDUP);
        // Dipicu dari "Mulai salin": lanjut ke konfirmasi hanya bila uji coba bersih.
        if self.salin_setelah_uji {
            self.salin_setelah_uji = false;
            if w == HIJAU {
                self.klik_salin();
            } else {
                self.set_pesan("Baca hasil uji coba di bawah sebelum menyalin.", KUNING);
            }
        }
    }
}
