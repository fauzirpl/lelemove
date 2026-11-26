use serde_json::{json, Value};
use std::path::Path;

use super::super::dasar::*;
use super::super::filter::Filter;
use super::super::riwayat::{self, Catatan};
use super::super::job::*;
use super::super::lanjut;
use super::super::rclone::{self, token_kedaluwarsa};
use super::super::rute::*;
use super::{Mesin, RE_AWAL_ERROR, argumen_tujuan, nama_tempat, potong};
use crate::{notifikasi, v, win};

// ------------------------------------------------------------------ notifikasi akhir salin

/// "369 file baru (5,00 GB)" dari blok statistik terakhir rclone.
fn ringkas_tersalin(s: &Statistik) -> String {
    let n: i64 = s.file.split('/').next().and_then(|x| x.trim().parse().ok()).unwrap_or(0);
    if n == 0 {
        return "Tidak ada file baru: semua sudah ada di tujuan.".into();
    }
    let data = s.data.split('/').next().unwrap_or("").trim();
    let (a, satuan) = data.split_once(' ').unwrap_or((data, ""));
    format!("{} file baru ({}). Buka LELEMOVE untuk verifikasi.", angka(n), format_ukuran(ke_byte(a, satuan)))
}

/// (file, byte) yang tersalin menurut statistik terakhir di log rclone.
fn angka_tersalin(log: &str) -> (i64, i64) {
    let Some(s) = baca_statistik(log) else { return (0, 0) };
    let n: i64 = s.file.split('/').next().and_then(|x| x.trim().parse().ok()).unwrap_or(0);
    let data = s.data.split('/').next().unwrap_or("").trim();
    let (a, satuan) = data.split_once(' ').unwrap_or((data, ""));
    (n, ke_byte(a, satuan) as i64)
}

/// Judul & isi notifikasi saat penyalinan latar berakhir (selesai atau berhenti karena galat).
/// Menambah satu baris ke log rclone (format waktu sama dengan rclone).
fn tulis_baris_log(log: &str, t: &str) {
    use std::io::Write;
    let (y, mo, d, h, mi, s) = waktu_lokal();
    if let Ok(mut f) = std::fs::OpenOptions::new().append(true).create(true).open(log) {
        let _ = writeln!(f, "{y:04}/{mo:02}/{d:02} {h:02}:{mi:02}:{s:02} NOTICE: {t}");
    }
}

/// Menunggu paling sedikit `minimal`, lalu sampai semua `host` bisa dihubungi lagi (diuji tiap
/// 15 detik). false bila internet tidak kembali dalam TUNGGU_JARINGAN_MAKS.
fn tunggu_jaringan(minimal: std::time::Duration, host: &[(String, u16)]) -> bool {
    let mulai = std::time::Instant::now();
    std::thread::sleep(minimal);
    loop {
        if host.iter().all(|(h, p)| lanjut::bisa_dihubungi(h, *p)) {
            return true;
        }
        if mulai.elapsed() >= lanjut::TUNGGU_JARINGAN_MAKS {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_secs(15));
    }
}

fn isi_notifikasi(src: &str, dst: &str, log: &str, kode: &Value) -> (String, Vec<String>) {
    let rute = format!("{} \u{2192} {}", nama_tempat(src), nama_tempat(dst));
    let stat = baca_statistik(log);
    if teks(Some(kode)) == "0" {
        let ringkas = stat.as_ref().map(ringkas_tersalin).unwrap_or_else(|| "Buka LELEMOVE untuk verifikasi.".into());
        return ("Penyalinan selesai".into(), vec![rute, ringkas]);
    }
    let ekor = baca_ekor(Path::new(log), 50).join("\n");
    if ekor.contains("upload limit") || ekor.contains("uploadLimitExceeded") {
        return (
            "Penyalinan berhenti: batas harian Google".into(),
            vec![rute, "Batas unggah 750 GB per hari tercapai. Lanjutkan besok; file yang sudah sampai dilewati.".into()],
        );
    }
    let detail = if token_kedaluwarsa(&ekor) || stat.as_ref().is_some_and(|s| s.perlu_login) {
        "Login salah satu akun kedaluwarsa. Buka LELEMOVE, klik Login ulang, lalu Lanjutkan.".to_string()
    } else {
        match baris_error(log, 1).pop() {
            Some(e) => potong(&RE_AWAL_ERROR.replace(&e, ""), 160),
            None => format!("rclone berhenti dengan kode {}. Buka LELEMOVE untuk melanjutkan.", teks(Some(kode))),
        }
    };
    ("Penyalinan berhenti karena galat".into(), vec![rute, detail])
}

impl Mesin {
    // -------------------------------------------------------------- salin

    fn ringkas_hasil(&self, log: &str, kode: &Value) {
        judul("Hasil penyalinan");
        if let Some(s) = baca_statistik(log) {
            info(&format!("Data   : {}", s.data));
            if !s.file.is_empty() {
                info(&format!("File   : {}", s.file));
            }
            if !s.error.is_empty() {
                awas(&format!("Error  : {}", s.error));
            }
            if !s.waktu.is_empty() {
                info(&format!("Waktu  : {}", s.waktu));
            }
        }
        let ekor = baca_ekor(Path::new(log), 50).join("\n");
        if teks(Some(kode)) == "0" {
            ok("Selesai. Lanjutkan dengan verifikasi: pindah verifikasi");
        } else if ekor.contains("upload limit") || ekor.contains("uploadLimitExceeded") {
            awas("Batas unggah harian Google (750 GB) tercapai. Jalankan \"salin\" lagi besok; file yang sudah ada dilewati.");
        } else {
            gagal(&format!("Ada yang gagal (kode {}).", teks(Some(kode))));
            for e in baris_error(log, 5) {
                info(&e);
            }
            if token_kedaluwarsa(&ekor) {
                info("Login salah satu akun kedaluwarsa: login ulang akun itu, lalu jalankan \"salin\" lagi.");
            }
            info("Jalankan \"salin\" lagi: file yang sudah tersalin dilewati, sisanya dicoba ulang.");
        }
        info(&format!("Log: {log}"));
    }

    pub(super) fn jalankan_salin(&mut self, src: &str, dst: &str, log: &str, batas: &str, tampil: bool, f: &Filter) -> i32 {
        let mulai = sekarang_iso();
        let mut a = v!["copy", src, dst];
        if f.buat_folder_kosong() {
            a.push("--create-empty-src-dirs".into());
        }
        a.extend(f.argumen());
        a.extend(argumen_tujuan(dst));
        a.extend(v![
            "--transfers", "4", "--checkers", "8",
            // low-level-retries rendah: saat Google membatasi (rate limit), error cepat muncul di log
            // alih-alih rclone diam menunggu berpuluh menit; --retries mengulang file yang gagal.
            "--retries", "5", "--low-level-retries", "10",
            "--drive-chunk-size", "64M", "--drive-stop-on-upload-limit",
            "--log-file", log, "--log-level", "INFO", "--stats", "5s",
        ]);
        if batas != "off" {
            a.extend(v!["--bwlimit", batas]);
        }
        if tampil {
            a.push("--progress".into());
        }
        // Hanya pekerja latar yang melanjutkan sendiri; salin di konsol tetap berhenti dan melapor.
        let lanjut_otomatis = self.o.log_latar.is_some() && baca_lanjut_otomatis();
        let maks = lanjut::maks_percobaan();
        let host = lanjut::host_rute(src, dst);
        let mut percobaan = 1u32;
        win::jaga_bangun(true);
        let kode: Value = loop {
            let awal_log = std::fs::metadata(log).map(|m| m.len()).unwrap_or(0);
            let kode = match rclone::mulai(&a) {
                Ok(mut p) => {
                    perbarui_job(
                        &[
                            ("PidInduk", json!(std::process::id())),
                            ("PidRclone", json!(p.id())),
                            ("Percobaan", json!(percobaan)),
                            ("Menunggu", Value::Null),
                        ],
                        false,
                    );
                    match p.wait() {
                        Ok(s) => json!(s.code().unwrap_or(1)),
                        Err(_) => json!("terputus"),
                    }
                }
                Err(e) => {
                    gagal(&format!("rclone tidak bisa dijalankan: {e}"));
                    json!(1)
                }
            };
            let k = bil(Some(&kode)).unwrap_or(1) as i32;
            if !lanjut_otomatis || percobaan >= maks {
                break kode;
            }
            if lanjut::putuskan(k, &lanjut::potongan_log(Path::new(log), awal_log)) != lanjut::Putusan::Ulang {
                break kode;
            }
            // Jaringan putus: tunggu, lalu jalankan rclone lagi (file yang sudah sampai dilewati).
            let tunggu = lanjut::jeda(percobaan);
            let pukul = lanjut::jam_nanti(tunggu.as_secs());
            perbarui_job(
                &[
                    ("PidRclone", Value::Null),
                    ("Menunggu", json!("jaringan")),
                    ("MenungguSejak", json!(sekarang_iso())),
                    ("UlangPukul", json!(pukul)),
                    ("PercobaanMaks", json!(maks)),
                ],
                false,
            );
            tulis_baris_log(
                log,
                &format!("LELEMOVE: jaringan terputus; dicoba lagi otomatis sekitar pukul {pukul} (percobaan {} dari {maks}).", percobaan + 1),
            );
            if percobaan == 1 {
                let rute = format!("{} \u{2192} {}", nama_tempat(src), nama_tempat(dst));
                let _ = notifikasi::kirim(
                    "Jaringan terputus",
                    &[rute, "LELEMOVE melanjutkan penyalinan otomatis begitu internet tersambung lagi.".into()],
                );
            }
            if !tunggu_jaringan(tunggu, &host) {
                tulis_baris_log(log, "LELEMOVE: internet tidak kembali dalam 6 jam; penyalinan dihentikan.");
                break kode;
            }
            tulis_baris_log(log, "LELEMOVE: internet tersambung lagi; penyalinan dilanjutkan.");
            percobaan += 1;
        };
        win::jaga_bangun(false);
        perbarui_job(&[("Selesai", json!(sekarang_iso())), ("Kode", kode.clone())], false);
        let (n_file, n_byte) = angka_tersalin(log);
        let berhasil = bil(Some(&kode)) == Some(0);
        riwayat::catat(Catatan {
            jenis: "salin".into(),
            mulai,
            sumber: src.into(),
            tujuan: dst.into(),
            hasil: if berhasil { "selesai" } else { "galat" }.into(),
            ket: if berhasil {
                String::new()
            } else {
                baris_error(log, 1)
                    .pop()
                    .map(|e| potong(&RE_AWAL_ERROR.replace(&e, ""), 200))
                    .unwrap_or_else(|| format!("rclone berhenti dengan kode {}.", teks(Some(&kode))))
            },
            file: n_file,
            byte: n_byte,
            filter: f.uraian(),
            jadwal: self.nama_jadwal.clone(),
            log: log.into(),
            ..Default::default()
        });
        if tampil {
            self.ringkas_hasil(log, &kode);
        }
        // Pekerja latar: jendela mungkin sudah ditutup, jadi kabari lewat notifikasi Windows.
        // (Dihentikan manual tidak sampai ke sini: `berhenti` mematikan proses ini lebih dulu.)
        // Backup terjadwal yang berhasil dikabari setelah verifikasi (satu notifikasi saja).
        if self.o.log_latar.is_some() && !(self.dari_jadwal && bil(Some(&kode)) == Some(0)) {
            let (judul, baris) = isi_notifikasi(src, dst, log, &kode);
            let hasil = match notifikasi::kirim(&judul, &baris) {
                Ok(()) => "terkirim".to_string(),
                Err(e) => format!("gagal: {e}"),
            };
            perbarui_job(&[("Notifikasi", json!(hasil))], false);
        }
        bil(Some(&kode)).unwrap_or(1) as i32
    }

    pub(super) fn salin(&mut self) -> i32 {
        let r = self.ambil_rute();
        if !self.rute_siap(&r, true) {
            return 1;
        }
        let (src, dst) = (r.sumber.clone().unwrap(), r.tujuan.clone().unwrap());
        let batas = self.o.batas_kecepatan.clone().unwrap_or_else(baca_batas_kecepatan);
        let Some(f) = self.ambil_filter() else { return 1 };

        // Proses anak yang dijalankan oleh "salin -Latar".
        if let Some(log) = self.o.log_latar.clone() {
            return self.jalankan_salin(&src, &dst, &log, &batas, false, &f);
        }
        if baca_job().is_some_and(|j| job_berjalan(&j)) {
            awas("Penyalinan lain masih berjalan. Lihat: pindah status");
            return 1;
        }
        // -BatasKecepatan diingat untuk penyalinan berikutnya, seperti -Sumber/-Tujuan.
        if self.o.batas_kecepatan.is_some() {
            simpan_batas_kecepatan(&batas);
        }

        judul("Mulai menyalin");
        info(&format!("Dari  : {src}"));
        info(&format!("Ke    : {dst}"));
        info("Mode  : SALIN saja. File di sumber tidak diubah atau dihapus.");
        if batas != "off" {
            info(&format!("Batas : {}", tampil_batas(&batas)));
        }
        if !f.kosong() {
            info(&format!("Filter: {}", f.uraian()));
        }
        if !self.konfirmasi("Mulai menyalin sekarang?") {
            return 1;
        }

        let lama = bersihkan_log_lama();
        if lama > 0 {
            info(&format!("{} log lama (lebih dari {UMUR_LOG_HARI} hari) dihapus.", angka(lama as i64)));
        }
        let log = path_log("salin", "log").to_string_lossy().to_string();
        let batas_job = if batas == "off" { Value::Null } else { json!(batas) };
        perbarui_job(
            &[
                ("Log", json!(log)),
                ("Mulai", json!(sekarang_iso())),
                ("Sumber", json!(src)),
                ("Tujuan", json!(dst)),
                ("BatasKecepatan", batas_job),
                ("Filter", if f.kosong() { Value::Null } else { json!(f.uraian()) }),
            ],
            true,
        );

        if self.o.latar {
            let exe = std::env::current_exe().unwrap_or_else(|_| super::super::dasar::exe_mesin());
            return match win::mulai_terlepas(&exe, &v!["salin", "-LogLatar", &log, "-BatasKecepatan", &batas]) {
                Ok(pid) => {
                    perbarui_job(&[("PidInduk", json!(pid)), ("Latar", json!(true))], false);
                    self.catat("Pid", pid);
                    self.catat("Log", log.clone());
                    ok(&format!("Berjalan di latar belakang (PID {pid}). Jendela ini boleh ditutup."));
                    info("Pantau   : pindah status");
                    info("Hentikan : pindah berhenti  (bisa dilanjutkan kapan saja)");
                    0
                }
                Err(e) => {
                    gagal(&format!("Penyalinan latar tidak bisa dimulai: {e}"));
                    perbarui_job(&[("Selesai", json!(sekarang_iso())), ("Kode", json!(1))], false);
                    1
                }
            };
        }
        info("Tekan Ctrl+C untuk berhenti; jalankan lagi nanti untuk melanjutkan.");
        self.jalankan_salin(&src, &dst, &log, &batas, true, &f)
    }

    pub fn status(&mut self) -> i32 {
        let Some(job) = baca_job() else {
            info("Belum pernah ada penyalinan.");
            return 0;
        };
        judul("Status penyalinan");
        info(&format!("Dari   : {}", teks(job.get("Sumber"))));
        info(&format!("Ke     : {}", teks(job.get("Tujuan"))));
        info(&format!("Mulai  : {}", teks(job.get("Mulai"))));
        let batas = teks(job.get("BatasKecepatan"));
        if !batas.is_empty() {
            info(&format!("Batas  : {}", tampil_batas(&batas)));
        }
        let kode = teks(job.get("Kode"));
        let selesai = teks(job.get("Selesai"));
        if job_berjalan(&job) && teks(job.get("Menunggu")) == "jaringan" {
            awas(&format!(
                "Keadaan: MENUNGGU JARINGAN. Dilanjutkan otomatis begitu internet tersambung (sekitar pukul {}, percobaan {} dari {}).",
                teks(job.get("UlangPukul")),
                bil(job.get("Percobaan")).unwrap_or(1) + 1,
                teks(job.get("PercobaanMaks"))
            ));
        } else if job_berjalan(&job) {
            ok("Keadaan: SEDANG BERJALAN");
        } else if kode == "0" {
            ok(&format!("Keadaan: SELESAI ({selesai})"));
        } else if !selesai.is_empty() {
            awas(&format!("Keadaan: berhenti, kode {kode} ({selesai})"));
        } else {
            awas("Keadaan: berhenti (jendela ditutup, komputer mati, atau dihentikan)");
        }
        let log = teks(job.get("Log"));
        match baca_statistik(&log) {
            Some(s) => {
                info(&format!("Data   : {}", s.data));
                if !s.file.is_empty() {
                    info(&format!("File   : {}", s.file));
                }
                if !s.error.is_empty() {
                    awas(&format!("Error  : {}", s.error));
                }
                if !s.waktu.is_empty() {
                    info(&format!("Waktu  : {}", s.waktu));
                }
                if s.dibatasi {
                    awas("Layanan cloud sedang membatasi permintaan (rate limit); rclone menunggu lalu mencoba lagi.");
                    info("Untuk Google Drive, pakai client ID sendiri: pindah hubungkan -Akun <akun> -ClientId X -ClientSecret Y");
                }
                if s.perlu_login {
                    awas("Login salah satu akun kedaluwarsa (invalid_grant). Hentikan, login ulang akun itu, lalu salin lagi.");
                }
            }
            None => info("Statistik belum tersedia (ditulis tiap 5 detik)."),
        }
        let err = baris_error(&log, 5);
        if !err.is_empty() {
            awas("Error terakhir:");
            for e in err {
                info(&e);
            }
        }
        info(&format!("Log    : {log}"));
        0
    }

    pub fn berhenti(&mut self) -> i32 {
        let job = baca_job().unwrap_or_default();
        if !job_berjalan(&job) {
            info("Tidak ada penyalinan yang sedang berjalan.");
            return 0;
        }
        // Induk latar belakang dihentikan dulu, supaya ia tidak sempat menimpa status "dihentikan".
        let saya = std::process::id();
        if let Some(p) = bil(job.get("PidInduk")).map(|p| p as u32) {
            let latar = job.get("Latar") == Some(&Value::Bool(true));
            if p != saya && (win::proses_hidup(p, &["pindah"]) || (latar && win::proses_hidup(p, &["powershell"]))) {
                win::bunuh(p);
            }
        }
        if let Some(p) = bil(job.get("PidRclone")).map(|p| p as u32) {
            if win::proses_hidup(p, &["rclone"]) {
                win::bunuh(p);
            }
        }
        perbarui_job(&[("Selesai", json!(sekarang_iso())), ("Kode", json!("dihentikan"))], false);
        // Pekerja latar sudah dimatikan, jadi ia tidak sempat mencatat; catat di sini.
        let log = teks(job.get("Log"));
        let (n_file, n_byte) = angka_tersalin(&log);
        riwayat::catat(Catatan {
            jenis: "salin".into(),
            mulai: teks(job.get("Mulai")),
            sumber: teks(job.get("Sumber")),
            tujuan: teks(job.get("Tujuan")),
            hasil: "dihentikan".into(),
            ket: "Dihentikan oleh pengguna; bisa dilanjutkan.".into(),
            file: n_file,
            byte: n_byte,
            filter: teks(job.get("Filter")),
            jadwal: teks(job.get("JadwalNama")),
            log,
            ..Default::default()
        });
        ok("Penyalinan dihentikan. Jalankan \"salin\" lagi untuk melanjutkan; file yang sudah tersalin dilewati.");
        0
    }
}
