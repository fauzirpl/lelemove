use regex::Regex;
use std::sync::LazyLock;

use super::super::akun::*;
use super::super::dasar::*;
use super::super::filter::Filter;
use super::super::riwayat::{self, Catatan};
use super::super::job::*;
use super::super::rclone::{panggil, Mode};
use super::Mesin;
use crate::v;

static RE_OFFICE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\.(docx|docm|dotx|xlsx|xlsm|xltx|pptx|pptm|potx|vsdx)$").unwrap());

impl Mesin {
    // -------------------------------------------------------------- verifikasi

    pub(super) fn verifikasi(&mut self) -> i32 {
        let r = self.ambil_rute();
        if !self.rute_siap(&r, true) {
            return 1;
        }
        let (src, dst) = (r.sumber.clone().unwrap(), r.tujuan.clone().unwrap());
        if baca_job().is_some_and(|j| job_berjalan(&j)) {
            awas("Tunggu penyalinan selesai dulu (lihat: pindah status).");
            return 1;
        }
        let Some(f) = self.ambil_filter() else { return 1 };
        self.verifikasi_rute(&src, &dst, &f)
    }

    /// Mencocokkan `src` dengan `dst` (dipakai `verifikasi`, `pindahkan`, dan backup terjadwal).
    pub(super) fn verifikasi_rute(&mut self, src: &str, dst: &str, f: &Filter) -> i32 {
        let mulai = sekarang_iso();
        let kode = self.verifikasi_inti(src, dst, f);
        let n = |k: &str| bil(self.hasil.get(k)).unwrap_or(0);
        let ada_laporan = kode == 0 || n("Cocok") + n("Hilang") + n("Beda") + n("Galat") > 0;
        riwayat::catat(Catatan {
            jenis: "verifikasi".into(),
            mulai,
            sumber: src.into(),
            tujuan: dst.into(),
            hasil: if kode == 0 {
                "cocok"
            } else if ada_laporan {
                "selisih"
            } else {
                "galat"
            }
            .into(),
            ket: if ada_laporan { String::new() } else { "Verifikasi tidak berjalan; lihat log aktivitas.".into() },
            file: n("Cocok"),
            hilang: n("Hilang"),
            beda: n("Beda"),
            galat: n("Galat"),
            metode: teks(self.hasil.get("Metode")),
            filter: f.uraian(),
            jadwal: self.nama_jadwal.clone(),
            laporan: teks(self.hasil.get("Laporan")),
            ..Default::default()
        });
        kode
    }

    fn verifikasi_inti(&mut self, src: &str, dst: &str, f: &Filter) -> i32 {
        let (src, dst) = (src.to_string(), dst.to_string());
        judul("Verifikasi: mencocokkan sumber dengan tujuan");
        self.info_filter(f);
        info("Membandingkan setiap file: dengan checksum bila kedua sisi punya jenis yang sama, selain itu nama & ukuran...");
        let lap = path_log("verifikasi", "txt");
        let laps = lap.to_string_lossy().to_string();
        let mut a = v!["check", &src, &dst, "--one-way", "--combined", &laps];
        a.extend(f.argumen());
        a.extend(v!["--log-level", "NOTICE"]);
        let h = panggil(&a, Mode::Tangkap);
        // rclone: "No common hash found - not using a hash for checks" (mis. OneDrive <-> Google Drive)
        let metode = if h.err.contains("No common hash") || h.err.contains("no hashes in common") { "ukuran" } else { "checksum" };
        self.catat("Metode", metode);
        if metode == "ukuran" {
            info("Kedua layanan tidak punya jenis checksum yang sama; dibandingkan per nama & ukuran.");
        } else {
            info("Dibandingkan dengan checksum (isi file).");
        }

        let isi: Vec<String> = std::fs::read_to_string(&lap)
            .map(|s| s.lines().map(str::to_string).collect())
            .unwrap_or_default();
        let ambil = |awalan: &str| -> Vec<String> { isi.iter().filter(|l| l.starts_with(awalan)).cloned().collect() };
        let mut sama = ambil("= ");
        let hilang = ambil("- ");
        let mut beda = ambil("* ");
        let galat = ambil("! ");
        // Tujuan SharePoint: file Office yang "berbeda" adalah akibat metadata yang ditulis SharePoint.
        let mut office = Vec::new();
        let rem_t = remote_dari(&dst);
        if !rem_t.is_empty() && jenis_dari_nama(&rem_t) == "sharepoint" {
            office = beda.iter().filter(|l| RE_OFFICE.is_match(l)).cloned().collect::<Vec<_>>();
            beda.retain(|l| !RE_OFFICE.is_match(l));
            sama.extend(office.iter().cloned());
            if !office.is_empty() {
                info(&format!(
                    "{} file Office berbeda ukuran karena metadata yang ditambahkan SharePoint; dianggap cocok.",
                    angka(office.len() as i64)
                ));
            }
        }
        let contoh: Vec<String> = hilang.iter().chain(beda.iter()).chain(galat.iter()).take(5).cloned().collect();
        self.catat("OfficeSharePoint", office.len() as i64);
        self.catat("Cocok", sama.len() as i64);
        self.catat("Hilang", hilang.len() as i64);
        self.catat("Beda", beda.len() as i64);
        self.catat("Galat", galat.len() as i64);
        self.catat("Contoh", contoh);
        self.catat("Laporan", laps.clone());

        info(&format!("Cocok                      : {} file", angka(sama.len() as i64)));
        if !hilang.is_empty() {
            awas(&format!("Belum ada di tujuan        : {} file", angka(hilang.len() as i64)));
        }
        if !beda.is_empty() {
            awas(&format!("Isinya berbeda             : {} file", angka(beda.len() as i64)));
        }
        if !galat.is_empty() {
            awas(&format!("Gagal dibaca               : {} file", angka(galat.len() as i64)));
        }

        self.catat("Sumber", src.clone());
        if !isi.is_empty() && hilang.is_empty() && beda.is_empty() && galat.is_empty() {
            ok(&format!("SEMUA FILE COCOK: {} file dari {src}", angka(sama.len() as i64)));
            info(&format!("sudah ada di {dst}."));
            info("File di sumber tidak diubah.");
            info(&format!("Laporan: {laps}"));
            return 0;
        }
        if isi.is_empty() && h.kode == 0 {
            ok("Tidak ada file untuk dicocokkan (folder kosong, atau tidak ada yang lolos filter).");
            return 0;
        }
        if isi.is_empty() {
            gagal(&format!("Verifikasi tidak berjalan (kode {}):", h.kode));
            for b in h.galat_terakhir(5) {
                info(&b);
            }
            self.saran_login(&h.err, &[&src, &dst]);
            return 1;
        }
        for l in hilang.iter().chain(beda.iter()).chain(galat.iter()).take(10) {
            info(l);
        }
        info("Jalankan \"salin\" lagi untuk melengkapi, lalu verifikasi ulang.");
        info(&format!("Laporan lengkap: {laps}"));
        1
    }
}
