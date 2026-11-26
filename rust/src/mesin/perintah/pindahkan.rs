use std::path::{Path, PathBuf};

use super::super::akun::*;
use super::super::dasar::*;
use super::super::riwayat::{self, Catatan};
use super::super::job::*;
use super::super::penyedia;
use super::super::rclone::{panggil, Mode};
use super::super::rute::*;
use super::Mesin;
use crate::{v, win};

/// Folder yang menjadi kosong karena isinya dipindahkan ikut dihapus (dari yang terdalam),
/// tanpa menyentuh folder yang memang kosong sejak awal atau folder sumber itu sendiri.
fn folder_induk(relatif: &[String]) -> Vec<String> {
    let mut d: Vec<String> = Vec::new();
    for p in relatif {
        let mut s = p.as_str();
        while let Some(i) = s.rfind('/') {
            s = &s[..i];
            if !d.iter().any(|x| x == s) {
                d.push(s.to_string());
            }
        }
    }
    d.sort_by_key(|x| std::cmp::Reverse(x.matches('/').count()));
    d
}

fn hapus_folder_kosong_lokal(akar: &Path, dihapus: &[PathBuf]) {
    let rel: Vec<String> = dihapus
        .iter()
        .filter(|p| !p.exists())
        .filter_map(|p| p.strip_prefix(akar).ok().map(|r| r.to_string_lossy().replace('\\', "/")))
        .collect();
    for d in folder_induk(&rel) {
        let _ = std::fs::remove_dir(akar.join(d.replace('/', std::path::MAIN_SEPARATOR_STR))); // gagal bila tidak kosong: dibiarkan
    }
}

fn hapus_folder_kosong_cloud(src: &str, dihapus: &[String]) {
    // Satu panggilan rclone per folder; batasi supaya tidak berjalan terlalu lama.
    for d in folder_induk(dihapus).into_iter().take(100) {
        let _ = panggil(&v!["rmdir", gabung_path(src, &d), "--drive-use-trash"], Mode::Tangkap);
    }
}

impl Mesin {
    pub(super) fn pindahkan(&mut self) -> i32 {
        let r = self.ambil_rute();
        if !self.rute_siap(&r, true) {
            return 1;
        }
        let Some(f) = self.ambil_filter() else { return 1 };
        let (src, dst) = (r.sumber.clone().unwrap(), r.tujuan.clone().unwrap());
        if baca_job().is_some_and(|j| job_berjalan(&j)) {
            awas("Tunggu penyalinan selesai dulu (lihat: pindah status).");
            return 1;
        }
        let lokal = adalah_lokal(&src);
        if lokal && !win::drive_tetap(Path::new(&src)) {
            gagal(if cfg!(windows) {
                "Folder sumber ada di drive jaringan atau flashdisk yang tidak punya Recycle Bin; file di sana akan hilang permanen."
            } else {
                "Folder sumber ada di volume jaringan yang tidak punya Tong Sampah; file di sana akan hilang permanen."
            });
            info("LELEMOVE tidak menghapusnya. Hapus sendiri bila sudah yakin.");
            return 1;
        }
        let tempat: String = if lokal {
            format!("{} {}", win::NAMA_SAMPAH, win::NAMA_OS)
        } else {
            let p = penyedia::cari(jenis_dari_nama(&remote_dari(&src)));
            match p.and_then(|p| p.sampah) {
                Some(s) => s.to_string(),
                None => {
                    let n = p.map_or("Layanan ini", |p| p.nama);
                    gagal(&format!("{n} tidak punya tempat sampah: file yang dihapus akan hilang permanen."));
                    info("LELEMOVE tidak menghapus di sana. Hapus sendiri bila sudah yakin.");
                    return 1;
                }
            }
        };
        let mulai = sekarang_iso();

        // 1. Verifikasi ulang sekarang: hasil verifikasi sebelumnya bisa basi.
        let v = self.verifikasi_rute(&src, &dst, &f);
        let laporan = teks(self.hasil.get("Laporan"));
        let per_file = riwayat::baca_per_file(Path::new(&laporan));
        let cocok: Vec<String> = per_file.iter().filter(|(s, _)| *s == '=').map(|(_, p)| p.clone()).collect();
        let lain = per_file.iter().filter(|(s, _)| *s != '=' && *s != '+').count() as i64;
        judul("Pindahkan: hapus dari sumber file yang sudah terbukti sama di tujuan");
        self.catat("AkanDihapus", cocok.len() as i64);
        self.catat("TidakDihapus", lain);
        if cocok.is_empty() {
            info("Tidak ada file yang terbukti sama di tujuan; tidak ada yang dihapus.");
            return if v == 0 { 0 } else { 1 };
        }
        info(&format!("{} file di {src} terbukti sama di {dst}.", angka(cocok.len() as i64)));
        info(&format!("File itu dipindahkan ke {tempat}, jadi masih bisa dipulihkan."));
        if lain > 0 {
            awas(&format!("{} file lain belum cocok dan TIDAK dihapus.", angka(lain)));
        }
        if !self.o.ya {
            if !bisa_bertanya() {
                awas("Butuh persetujuan. Ulangi perintah dengan tambahan -Ya setelah disetujui.");
                return 3;
            }
            if baca_baris("        Ketik HAPUS untuk melanjutkan: ").trim() != "HAPUS" {
                info("Dibatalkan; tidak ada yang dihapus.");
                return 1;
            }
        }

        // 2. Hapus (ke tempat sampah masing-masing layanan).
        let log = path_log("pindahkan", "log");
        let logs = log.to_string_lossy().to_string();
        let dihapus = if lokal {
            let akar = PathBuf::from(&src);
            let berkas: Vec<PathBuf> = cocok.iter().map(|p| akar.join(p.replace('/', std::path::MAIN_SEPARATOR_STR))).collect();
            let n = win::ke_recycle_bin(&berkas);
            let catatan: String = berkas
                .iter()
                .map(|p| format!("{} {}\r\n", if p.exists() { "GAGAL  " } else { "Dihapus" }, p.display()))
                .collect();
            let _ = std::fs::write(&log, catatan);
            hapus_folder_kosong_lokal(&akar, &berkas);
            n
        } else {
            let daftar = path_log("pindahkan-daftar", "txt");
            let _ = std::fs::write(&daftar, cocok.join("\n") + "\n");
            let a = v![
                "delete", &src, "--files-from-raw", daftar.to_string_lossy(),
                // Google Drive: ke Sampah (bawaan rclone, ditegaskan). OneDrive: ke recycle bin (bawaan).
                "--drive-use-trash", "--log-file", &logs, "--log-level", "INFO",
            ];
            let h = panggil(&a, Mode::Tangkap);
            let n = std::fs::read_to_string(&log).unwrap_or_default().lines().filter(|l| l.contains(": Deleted")).count();
            if h.kode != 0 {
                awas(&format!("rclone berhenti dengan kode {}; sebagian file mungkin belum terhapus.", h.kode));
                for b in h.galat_terakhir(5) {
                    info(&b);
                }
                self.saran_login(&h.err, &[&src]);
            }
            let _ = std::fs::remove_file(&daftar);
            hapus_folder_kosong_cloud(&src, &cocok);
            n
        };

        let semua = dihapus == cocok.len();
        let mut ket = format!("{} file ke {tempat}", angka(dihapus as i64));
        if !semua {
            ket += &format!("; {} gagal dihapus", angka((cocok.len() - dihapus) as i64));
        }
        if lain > 0 {
            ket += &format!("; {} file belum cocok tidak dihapus", angka(lain));
        }
        riwayat::catat(Catatan {
            jenis: "pindahkan".into(),
            mulai,
            sumber: src.clone(),
            tujuan: dst.clone(),
            hasil: if semua { "selesai" } else if dihapus > 0 { "sebagian" } else { "galat" }.into(),
            ket: ket.clone(),
            file: dihapus as i64,
            hilang: bil(self.hasil.get("Hilang")).unwrap_or(0),
            beda: bil(self.hasil.get("Beda")).unwrap_or(0),
            galat: bil(self.hasil.get("Galat")).unwrap_or(0),
            filter: f.uraian(),
            log: logs.clone(),
            laporan,
            ..Default::default()
        });
        self.catat("Dihapus", dihapus as i64);
        self.catat("Tempat", tempat.as_str());
        if semua {
            ok(&format!("{} file dipindahkan: kini hanya ada di {dst}.", angka(dihapus as i64)));
        } else {
            awas(&format!("Hanya {} dari {} file yang terhapus dari sumber.", angka(dihapus as i64), angka(cocok.len() as i64)));
        }
        info(&format!("Yang dihapus ada di {tempat} bila perlu dipulihkan. Log: {logs}"));
        if semua {
            0
        } else {
            1
        }
    }
}
