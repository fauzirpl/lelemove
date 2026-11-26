use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;

use super::super::akun::*;
use super::super::dasar::*;
use super::super::rclone::{cari_rclone, daftar_remote, panggil, Mode};
use super::super::rute::*;
use super::{Mesin, argumen_tujuan, json_dari};
use crate::v;

static RE_UKURAN_DRY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\(size ([\d.]+)\s*([A-Za-z]*)\)").unwrap());
static RE_MASALAH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"ERROR|non file|non-regular|irregular|symlink|Can't transfer|Can't follow").unwrap()
});

impl Mesin {
    // -------------------------------------------------------------- periksa & uji

    /// Kode keluar: 0 siap, 1 belum siap/error, 2 ruang tujuan kurang, 3 ada peringatan.
    pub(super) fn periksa(&mut self) -> i32 {
        let r = self.ambil_rute();
        if !self.rute_siap(&r, false) {
            return 1;
        }
        let Some(f) = self.ambil_filter() else { return 1 };
        let (src, dst) = (r.sumber.clone().unwrap(), r.tujuan.clone().unwrap());
        let mut hasil = 0;
        let mut src_bytes: i64;

        judul("Sumber");
        info(&src);
        self.info_filter(&f);
        if adalah_lokal(&src) {
            info("Menghitung isi folder (hanya membaca daftar file, tidak mengunduh apa pun)...");
            let u = ukur_lokal(&src);
            src_bytes = u.bytes;
            let mut jumlah = u.jumlah;
            // Dengan filter, yang dihitung hanya file yang lolos (oleh rclone, aturan yang sama dengan salin).
            if !f.kosong() && cari_rclone().is_some() {
                let mut a = v!["size", &src, "--json"];
                a.extend(f.argumen());
                let h = panggil(&a, Mode::Tangkap);
                if h.kode == 0 {
                    let j = json_dari(&h.out);
                    src_bytes = bil(j.get("bytes")).unwrap_or(0);
                    jumlah = bil(j.get("count")).unwrap_or(0);
                }
            }
            self.catat("SumberJumlah", jumlah);
            self.catat("SumberBytes", src_bytes);
            self.catat("HanyaCloudJumlah", u.jumlah_hanya_cloud);
            self.catat("HanyaCloudBytes", u.hanya_cloud);
            self.catat("DiskCukup", true);
            ok(&format!("{} file, total {}", angka(jumlah), format_ukuran(src_bytes as f64)));
            if !f.kosong() {
                info(&format!("(dari {} file di folder ini; sisanya tidak lolos filter)", angka(u.jumlah)));
            }
            if u.terbesar > 0 {
                info(&format!("File terbesar: {} ({})", u.terbesar_nama, format_ukuran(u.terbesar as f64)));
            }
            if u.jumlah_hanya_cloud > 0 {
                let bebas = ruang_disk(&src);
                self.catat("DiskBebas", bebas);
                info(&format!(
                    "{} file ({}) masih berstatus \"hanya online\" di OneDrive.",
                    angka(u.jumlah_hanya_cloud),
                    format_ukuran(u.hanya_cloud as f64)
                ));
                info(&format!(
                    "Saat disalin, OneDrive mengunduhnya dulu ke disk ini. Sisa ruang disk: {}",
                    format_ukuran(bebas as f64)
                ));
                if u.hanya_cloud as f64 > bebas as f64 - 2.0 * GB {
                    self.catat("DiskCukup", false);
                    awas("Ruang disk tidak cukup untuk mengunduh semuanya sekaligus.");
                    info("Salin per subfolder, lalu klik kanan subfolder itu di OneDrive > \"Kosongkan ruang\",");
                    info("atau pakai OneDrive lewat internet: pindah hubungkan -Akun onedrive");
                    hasil = 3;
                }
            }
            info("Catatan: hanya folder yang disinkronkan OneDrive ke komputer ini yang ikut terhitung.");
        } else if self.wajib_rclone() {
            info("Menghitung isi folder di cloud...");
            let mut a = v!["size", &src, "--json"];
            a.extend(f.argumen());
            let h = panggil(&a, Mode::Tangkap);
            if h.kode != 0 {
                gagal("Tidak bisa membaca sumber.");
                info(h.err.trim());
                self.saran_login(&h.err, &[&src]);
                return 1;
            }
            let j = json_dari(&h.out);
            src_bytes = bil(j.get("bytes")).unwrap_or(0);
            let n = bil(j.get("count")).unwrap_or(0);
            self.catat("SumberJumlah", n);
            self.catat("SumberBytes", src_bytes);
            ok(&format!("{} file, total {}", angka(n), format_ukuran(src_bytes as f64)));
        } else {
            return 1;
        }

        judul("Tujuan");
        info(&dst);
        self.catat("TujuanTerhubung", false);
        let rem_t = remote_dari(&dst);
        let mut jenis_t = String::new();
        if !rem_t.is_empty() {
            if !self.wajib_rclone() {
                return 1;
            }
            if !daftar_remote().contains(&rem_t) {
                awas(&format!("Akun '{rem_t}' belum terhubung. Jalankan: pindah hubungkan -Akun {rem_t}"));
                return 1;
            }
            jenis_t = jenis_remote(&rem_t);
        }
        self.catat("TujuanJenis", jenis_t.clone());

        let mut sudah: i64 = 0;
        if !rem_t.is_empty() || Path::new(&dst).is_dir() {
            // Yang sudah ada di tujuan dihitung dengan filter yang sama, supaya selisihnya sebanding.
            let mut a = v!["size", &dst, "--json"];
            if !f.kosong() {
                a.extend(f.argumen());
            }
            let h = panggil(&a, Mode::Tangkap);
            if h.kode == 0 {
                sudah = bil(json_dari(&h.out).get("bytes")).unwrap_or(0);
                if sudah > 0 {
                    info(&format!("Sudah ada di tujuan: {} (yang sama akan dilewati)", format_ukuran(sudah as f64)));
                }
            }
        }
        let butuh = (src_bytes - sudah).max(0);
        self.catat("TujuanSudah", sudah);
        self.catat("Butuh", butuh);

        let (mut total, mut terpakai, mut bebas): (i64, i64, i64) = (0, 0, -1);
        if !rem_t.is_empty() {
            let h = panggil(&v!["about", format!("{rem_t}:"), "--json"], Mode::Tangkap);
            // FTP, SFTP tertentu, S3, B2, Azure: tidak ada info kuota. Bukan galat; pastikan saja tujuan bisa dibuka.
            let tanpa_kuota = h.kode != 0 && tanpa_about(&h.err);
            if tanpa_kuota {
                if let Err(e) = self.uji_akses(&rem_t) {
                    if !e.to_lowercase().contains("accessdenied") && !e.contains("403") {
                        gagal(&format!("Tujuan tidak bisa dibuka: {e}"));
                        self.catat("TujuanGalat", true);
                        return 1;
                    }
                }
            } else if h.kode != 0 {
                gagal("Tidak bisa membaca kuota akun tujuan.");
                info(h.err.trim());
                self.catat("TujuanGalat", true);
                self.saran_login(&h.err, &[&dst]);
                return 1;
            }
            let a = json_dari(&h.out);
            if let Some(t) = bil(a.get("total")).filter(|t| *t > 0) {
                total = t;
                terpakai = bil(a.get("used")).unwrap_or(0);
                bebas = total - terpakai;
                if let Some(f) = bil(a.get("free")).filter(|f| *f > 0) {
                    bebas = f;
                }
                if let Some(s) = bil(a.get("trashed")).filter(|s| *s > 0) {
                    self.catat("TujuanSampah", s);
                    info(&format!("Sampah di akun ini memakai {} (ikut dihitung dalam kuota).", format_ukuran(s as f64)));
                }
            }
        } else {
            bebas = ruang_disk(&dst);
            total = bebas;
        }
        self.catat("TujuanTerhubung", true);
        if bebas >= 0 {
            self.catat("TujuanTotal", total);
            self.catat("TujuanTerpakai", terpakai);
            self.catat("TujuanBebas", bebas);
            self.catat("TujuanCukup", butuh <= bebas);
            if !rem_t.is_empty() {
                info(&format!(
                    "Kapasitas {}, terpakai {}, sisa {}",
                    format_ukuran(total as f64),
                    format_ukuran(terpakai as f64),
                    format_ukuran(bebas as f64)
                ));
            } else {
                info(&format!("Sisa ruang disk tujuan: {}", format_ukuran(bebas as f64)));
            }
            info(&format!("Perlu disalin sekitar {}", format_ukuran(butuh as f64)));
            if butuh > bebas {
                gagal("Ruang di tujuan TIDAK cukup.");
                info("Pilihan: kosongkan sampah/ruang di tujuan, salin per subfolder, atau tambah kuota.");
                return 2;
            }
        } else {
            info("Kuota tidak terbatas atau tidak dilaporkan.");
            self.catat("TujuanCukup", true);
        }
        if jenis_t == "google" && butuh as f64 > 700.0 * GB {
            awas("Google membatasi unggahan 750 GB per hari; penyalinan akan berhenti sendiri dan bisa dilanjutkan besok.");
        }
        if hasil == 0 {
            ok("Siap disalin.");
        }
        hasil
    }

    /// Kode keluar: 0 aman, 1 error, 3 ada file yang bermasalah/terlewat.
    pub(super) fn uji(&mut self) -> i32 {
        let r = self.ambil_rute();
        if !self.rute_siap(&r, true) {
            return 1;
        }
        let Some(f) = self.ambil_filter() else { return 1 };
        let (src, dst) = (r.sumber.clone().unwrap(), r.tujuan.clone().unwrap());
        judul("Uji coba (dry run): tidak ada file yang disalin");
        self.info_filter(&f);
        let log = path_log("uji", "log");
        let mut a = v!["copy", &src, &dst, "--dry-run"];
        if f.buat_folder_kosong() {
            a.push("--create-empty-src-dirs".into());
        }
        a.extend(f.argumen());
        a.extend(argumen_tujuan(&dst));
        a.extend(v!["--log-level", "NOTICE", "--stats", "1h", "--stats-log-level", "NOTICE"]);
        let h = panggil(&a, Mode::Tangkap);
        let _ = std::fs::write(&log, &h.err);

        if h.kode != 0 {
            gagal(&format!("rclone berhenti dengan kode {}:", h.kode));
            for b in h.galat_terakhir(8) {
                info(&b);
            }
            info(&format!("Log lengkap: {}", log.display()));
            self.saran_login(&h.err, &[&src, &dst]);
            return 1;
        }

        let baris: Vec<&str> = h.err.lines().collect();
        let salin: Vec<&str> = baris.iter().copied().filter(|b| b.contains("Skipped copy as --dry-run")).collect();
        let total: f64 = salin
            .iter()
            .filter_map(|b| RE_UKURAN_DRY.captures(b))
            .map(|c| ke_byte(&c[1], &c[2]))
            .sum();
        let masalah: Vec<String> = baris.iter().filter(|b| RE_MASALAH.is_match(b)).map(|s| s.to_string()).collect();
        self.catat("AkanDisalin", salin.len() as i64);
        self.catat("AkanBytes", total as i64);
        self.catat("Masalah", masalah.len() as i64);
        self.catat("MasalahContoh", masalah.iter().take(5).cloned().collect::<Vec<_>>());
        self.catat("Terlewat", 0);

        if salin.is_empty() {
            ok("Tidak ada yang perlu disalin: semua file sudah ada di tujuan.");
        } else {
            ok(&format!("{} file akan disalin (sekitar {}).", angka(salin.len() as i64), format_ukuran(total)));
        }

        let mut hasil = 0;
        // Dengan filter, selisih jumlah file memang disengaja: perbandingan ini tidak berarti.
        if adalah_lokal(&src) && f.kosong() {
            // Bandingkan jumlah file versi Windows dengan versi rclone: selisih berarti ada yang terlewat
            // (mis. placeholder OneDrive yang dianggap "bukan file biasa").
            let u = ukur_lokal(&src);
            let mut a = v!["size", &src, "--json"];
            a.extend(f.argumen());
            let s = panggil(&a, Mode::Tangkap);
            if s.kode == 0 {
                let nr = bil(json_dari(&s.out).get("count")).unwrap_or(0);
                if nr < u.jumlah {
                    self.catat("Terlewat", u.jumlah - nr);
                    awas(&format!(
                        "rclone hanya melihat {} dari {} file. {} file akan TERLEWAT.",
                        angka(nr),
                        angka(u.jumlah),
                        angka(u.jumlah - nr)
                    ));
                    info("Solusi: pakai OneDrive lewat internet (pindah hubungkan -Akun onedrive,");
                    info("lalu -Sumber \"pd-onedrive:<folder>\").");
                    hasil = 3;
                } else {
                    ok(&format!("Semua {} file di folder terbaca oleh rclone.", angka(u.jumlah)));
                }
            }
        }
        if !masalah.is_empty() {
            awas(&format!("{} catatan masalah:", masalah.len()));
            for m in masalah.iter().take(10) {
                info(m);
            }
            hasil = 3;
        }
        info(&format!("Log lengkap: {}", log.display()));
        hasil
    }
}
