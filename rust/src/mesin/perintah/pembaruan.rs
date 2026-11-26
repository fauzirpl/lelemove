use super::super::dasar::*;
use super::Mesin;
use crate::win;

impl Mesin {
    /// pindah pembaruan [-Unduh | -Jalankan]: periksa rilis terbaru di GitHub; -Unduh mengunduh dan
    /// mencocokkan SHA-256 berkas untuk sistem ini; -Jalankan juga menjalankannya (Windows:
    /// installer diam yang menunggu LELEMOVE tertutup lalu membukanya lagi; macOS: membuka .dmg).
    pub(super) fn pembaruan(&mut self) -> i32 {
        use super::super::pembaruan as pb;
        judul("Pembaruan LELEMOVE");
        // Untuk menguji alur unduh tanpa rilis baru: PINDAH_DRIVE_VERSI_UJI=2.0.0
        let kini = std::env::var("PINDAH_DRIVE_VERSI_UJI").unwrap_or_else(|_| crate::VERSI.to_string());
        info(&format!("Versi terpasang: {kini}"));
        let r = match pb::cek() {
            Ok(r) => r,
            Err(e) => {
                gagal(&format!("Rilis di GitHub tidak bisa diperiksa: {e}"));
                self.catat("Galat", e);
                return 1;
            }
        };
        let ada = pb::lebih_baru(&r.versi, &kini);
        self.catat("Versi", kini.clone());
        self.catat("Terbaru", r.versi.clone());
        self.catat("Ada", ada);
        self.catat("Halaman", r.halaman.clone());
        if !ada {
            ok(&format!("Sudah versi terbaru ({kini})."));
            return 0;
        }
        ok(&format!("Versi baru tersedia: {} ({}).", r.versi, r.halaman));
        if !self.o.unduh && !self.o.jalankan {
            info("Unduh dan pasang: pindah pembaruan -Jalankan");
            return 0;
        }
        let mut lapor = 0u64;
        // Untuk menguji installer yang belum dirilis lewat alur tombol: PINDAH_DRIVE_INSTALLER_UJI=<berkas>
        let uji = std::env::var_os("PINDAH_DRIVE_INSTALLER_UJI").map(std::path::PathBuf::from);
        let hasil = match uji.clone() {
            Some(p) => Ok(p),
            None => {
                info(&format!("Mengunduh {}.", pb::nama_berkas(&r.versi)));
                pb::unduh(&r, &mut |n, total| {
                    if total > 0 && n * 10 / total > lapor {
                        lapor = n * 10 / total;
                        info(&format!("Mengunduh {} dari {} ({}%)", format_ukuran(n as f64), format_ukuran(total as f64), n * 100 / total));
                    }
                })
            }
        };
        let berkas = match hasil {
            Ok(p) => p,
            Err(e) => {
                gagal(&format!("Unduhan gagal: {e}"));
                self.catat("Galat", e);
                return 1;
            }
        };
        if uji.is_some() {
            info(&format!("Mode uji: memakai installer lokal tanpa mengunduh ({}).", berkas.display()));
        } else {
            ok(&format!("Diunduh dan cocok dengan SHA-256 rilis: {}", berkas.display()));
        }
        self.catat("Berkas", berkas.display().to_string());
        if !self.o.jalankan {
            return 0;
        }
        if cfg!(windows) {
            // Installer diam yang menunggu jendela LELEMOVE dan proses ini tertutup, memasang, lalu
            // membuka LELEMOVE lagi. Terlepas agar tetap berjalan setelah pindah.exe selesai.
            let mut argumen = vec!["/silent".to_string(), "/perbarui".to_string()];
            // Perbarui pemasangan tempat program ini berada, bukan yang tercatat di registry
            // (catatan itu bisa basi atau menunjuk pemasangan lain). Saat dijalankan dari folder
            // build (tanpa LELEMOVE.exe di sebelahnya), installer memakai folder bawaannya.
            if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())) {
                if dir.join("LELEMOVE.exe").is_file() {
                    argumen.push("/dir".to_string());
                    argumen.push(dir.to_string_lossy().into_owned());
                }
            }
            match win::mulai_terlepas(&berkas, &argumen) {
                Ok(_) => {
                    ok("Installer dijalankan: LELEMOVE ditutup, diperbarui, lalu dibuka lagi.");
                    self.catat("Dipasang", true);
                    0
                }
                Err(e) => {
                    gagal(&format!("Installer tidak bisa dijalankan: {e}"));
                    self.catat("Galat", e.to_string());
                    1
                }
            }
        } else {
            win::buka(&berkas.to_string_lossy());
            ok("Berkas .dmg dibuka: seret LELEMOVE ke Applications (ganti yang lama), lalu buka lagi.");
            self.catat("Dibuka", true);
            0
        }
    }
}
