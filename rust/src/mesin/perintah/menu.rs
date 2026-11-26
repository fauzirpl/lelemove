use std::path::Path;

use super::super::akun::*;
use super::super::dasar::*;
use super::super::job::*;
use super::super::rclone::{cari_rclone, daftar_remote};
use super::super::rute::*;
use super::{Mesin, tampilkan_bantuan};

impl Mesin {
    // -------------------------------------------------------------- menu teks

    fn pilih_rute(&mut self) -> bool {
        judul("Pilih folder SUMBER");
        let mut opsi: Vec<(String, String)> = daftar_onedrive()
            .into_iter()
            .map(|o| (format!("[{}] {}", o.jenis, o.folder), o.folder))
            .collect();
        for a in daftar_akun().into_iter().filter(|a| a.siap) {
            opsi.push((format!("{} [{}]", a.label, a.nama), format!("{}:", a.nama)));
        }
        opsi.push(("Ketik alamat folder lain".into(), "__ketik".into()));
        opsi.push(("OneDrive lewat internet (login Microsoft)".into(), "__cloud".into()));
        for (i, (l, _)) in opsi.iter().enumerate() {
            info(&format!("{}. {l}", i + 1));
        }
        let n = baca_angka("Nomor", 1, opsi.len() as i64) as usize;
        let mut pilih = opsi[n - 1].1.clone();

        if pilih == "__ketik" {
            pilih = rapikan_sumber(&baca_baris("        Alamat folder: "));
            if !Path::new(&pilih).is_dir() {
                gagal("Folder tidak ditemukan.");
                return false;
            }
        } else if pilih == "__cloud" {
            if !daftar_remote().iter().any(|r| r == REMOTE_ONEDRIVE) {
                let lama = std::mem::replace(&mut self.o.akun, "onedrive".into());
                let k = self.hubungkan();
                self.o.akun = lama;
                if k != 0 {
                    return false;
                }
            }
            pilih = format!("{REMOTE_ONEDRIVE}:");
        }

        let sub = daftar_subfolder(&pilih);
        if !sub.is_empty() {
            judul("Salin semua isinya, atau satu subfolder saja?");
            info("0. Semua isi");
            for (i, s) in sub.iter().enumerate() {
                info(&format!("{}. {s}", i + 1));
            }
            let n = baca_angka("Nomor", 0, sub.len() as i64) as usize;
            if n > 0 {
                pilih = gabung_path(&pilih, &sub[n - 1]);
            }
        }

        let usul = tujuan_bawaan(&pilih, REMOTE_GOOGLE);
        judul("Folder TUJUAN di Google Drive");
        info(&format!("Usulan: {}", usul.trim_start_matches(&format!("{REMOTE_GOOGLE}:"))));
        let t = baca_baris("        Tekan Enter untuk memakai usulan, atau ketik nama lain: ");
        let tujuan = if t.is_empty() { usul } else { normalisasi_tujuan(&t) };
        simpan_pengaturan(Some(&pilih), Some(&tujuan));
        ok(&format!("Sumber: {pilih}"));
        ok(&format!("Tujuan: {tujuan}"));
        true
    }

    fn langkah_dipandu(&mut self) -> i32 {
        judul("LANGKAH 1/6 - rclone");
        if cari_rclone().is_some() {
            ok("rclone sudah terpasang.");
        } else if self.pasang() != 0 {
            return 1;
        }

        judul("LANGKAH 2/6 - Akun Google Drive");
        self.o.akun = "google".into();
        if self.hubungkan() != 0 {
            return 1;
        }

        judul("LANGKAH 3/6 - Folder sumber & tujuan");
        let r = self.ambil_rute();
        let mut pakai = false;
        if let Some(s) = &r.sumber {
            info(&format!("Pilihan tersimpan: {s}  ->  {}", r.tujuan.clone().unwrap_or_default()));
            pakai = self.konfirmasi("Pakai pilihan ini?");
        }
        if !pakai && !self.pilih_rute() {
            return 1;
        }

        judul("LANGKAH 4/6 - Periksa ukuran & ruang");
        let k = self.periksa();
        if k == 1 || k == 2 {
            return 1;
        }
        if k == 3 && !self.konfirmasi("Ada peringatan di atas. Tetap lanjut?") {
            return 1;
        }

        judul("LANGKAH 5/6 - Uji coba");
        let k = self.uji();
        if k == 1 {
            return 1;
        }
        if k == 3 && !self.konfirmasi("Uji coba menemukan masalah. Tetap lanjut?") {
            return 1;
        }

        judul("LANGKAH 6/6 - Salin lalu verifikasi");
        if self.salin() != 0 {
            return 1;
        }
        self.verifikasi()
    }

    pub(super) fn menu(&mut self) {
        if !bisa_bertanya() {
            tampilkan_bantuan();
            return;
        }
        loop {
            bersihkan_layar();
            let r = self.ambil_rute();
            let s = r.sumber.clone().unwrap_or_else(|| "(belum dipilih)".into());
            let t = r
                .tujuan
                .clone()
                .map(|t| t.replacen(&format!("{REMOTE_GOOGLE}:"), "Google Drive: ", 1))
                .unwrap_or_else(|| "(belum dipilih)".into());
            polos("");
            berwarna("96", "  PINDAH DRIVE  |  salin antar OneDrive, SharePoint, Google Drive, komputer");
            polos("  --------------------------------------------------------------------------");
            polos(&format!("  Sumber : {s}"));
            polos(&format!("  Tujuan : {t}"));
            if let Some(job) = baca_job().filter(job_berjalan) {
                let mut st = "sedang menyalin".to_string();
                if let Some(x) = baca_statistik(&teks(job.get("Log"))) {
                    st += &format!(" - {}", x.data);
                }
                berwarna("93", &format!("  Status : {st}"));
            }
            polos("");
            berwarna("92", "  1. Jalankan semua langkah (dipandu)   <- paling mudah");
            polos("");
            polos("  2. Pasang rclone");
            polos("  3. Hubungkan akun Google Drive");
            polos("  4. Pilih folder sumber & tujuan");
            polos("  5. Periksa ukuran & ruang kosong");
            polos("  6. Uji coba (tidak menyalin apa pun)");
            polos("  7. Mulai menyalin");
            polos("  8. Lihat status");
            polos("  9. Verifikasi hasil");
            polos("  0. Keluar");
            polos("");
            let p = baca_baris("  Pilih: ");
            let dikenal = match p.as_str() {
                "1" => {
                    self.langkah_dipandu();
                    true
                }
                "2" => {
                    self.pasang();
                    true
                }
                "3" => {
                    self.o.akun = "google".into();
                    self.hubungkan();
                    true
                }
                "4" => {
                    self.pilih_rute();
                    true
                }
                "5" => {
                    self.periksa();
                    true
                }
                "6" => {
                    self.uji();
                    true
                }
                "7" => {
                    self.salin();
                    true
                }
                "8" => {
                    self.status();
                    true
                }
                "9" => {
                    self.verifikasi();
                    true
                }
                "0" => return,
                _ => false,
            };
            if dikenal {
                polos("");
                let _ = baca_baris("  Tekan Enter untuk kembali ke menu");
            }
        }
    }
}
