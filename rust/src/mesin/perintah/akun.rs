use serde_json::Value;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use super::super::akun::*;
use super::super::dasar::*;
use super::super::job::*;
use super::super::rclone::{self, cari_rclone, daftar_remote, panggil, Mode};
use super::super::rute::*;
use super::{Mesin, json_dari};
use crate::win::PerintahExt;
use crate::{v, win};

pub fn ringkas_akun(a: &Akun) -> String {
    let mut t = format!("{} [{}]", a.label, a.nama);
    match a.jenis.as_str() {
        "google" => {
            t += " - Google Drive";
            t += if a.scope == "drive" { ", izin penuh" } else { ", izin terbatas (hanya tujuan)" };
            t += if a.client_sendiri { ", client ID sendiri" } else { ", client ID bersama (sering dibatasi)" };
        }
        "onedrive" => {
            t += " - OneDrive";
            if !a.drive.is_empty() {
                t += &format!(", drive {}", a.drive);
            }
        }
        "sharepoint" => {
            t += " - SharePoint";
            if !a.drive_id.is_empty() {
                t += &format!(", situs {}, pustaka {}", a.situs, a.drive);
            } else {
                t += ", BELUM memilih situs & pustaka";
            }
        }
        _ => {}
    }
    t
}

/// Program pemasang rclone milik sistem: winget (Windows) atau Homebrew (macOS).
struct Pemasang {
    exe: PathBuf,
    args: &'static [&'static str],
    ket: &'static str,
    setuju: &'static str,
}

#[cfg(windows)]
const TANPA_PEMASANG: [&str; 2] = [
    "winget tidak tersedia di komputer ini.",
    "Unduh rclone dari https://rclone.org/downloads/ lalu taruh rclone.exe di folder aplikasi ini.",
];

#[cfg(target_os = "macos")]
const TANPA_PEMASANG: [&str; 2] = [
    "Homebrew tidak tersedia di Mac ini.",
    "Unduh rclone untuk macOS dari https://rclone.org/downloads/ lalu taruh berkas rclone di /usr/local/bin.",
];

#[cfg(windows)]
fn pemasang_rclone() -> Option<Pemasang> {
    cari_winget().map(|exe| Pemasang {
        exe,
        args: &[
            "install", "--id", "Rclone.Rclone", "--exact", "--source", "winget",
            "--accept-source-agreements", "--accept-package-agreements", "--disable-interactivity",
        ],
        ket: "Dipasang lewat winget dari repositori resmi winget (paket Rclone.Rclone, sekitar 25 MB).",
        setuju: "Unduh & pasang rclone sekarang (menyetujui ketentuan sumber/paket winget)?",
    })
}

#[cfg(target_os = "macos")]
fn pemasang_rclone() -> Option<Pemasang> {
    ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"].iter().map(PathBuf::from).find(|p| p.is_file()).map(|exe| Pemasang {
        exe,
        args: &["install", "rclone"],
        ket: "Dipasang lewat Homebrew (formula rclone, sekitar 25 MB).",
        setuju: "Unduh & pasang rclone sekarang lewat Homebrew?",
    })
}

#[cfg(windows)]
fn cari_winget() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("PATH") {
        for d in std::env::split_paths(&path) {
            let p = d.join("winget.exe");
            // winget.exe di WindowsApps adalah alias (reparse point): cukup ada, tidak perlu dibaca.
            if p.symlink_metadata().is_ok() {
                return Some(p);
            }
        }
    }
    let p = PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_default()).join("Microsoft\\WindowsApps\\winget.exe");
    p.symlink_metadata().is_ok().then_some(p)
}

impl Mesin {
    // -------------------------------------------------------------- cek, akun, pasang

    pub(super) fn cek(&mut self) -> i32 {
        judul("Kesiapan");
        if let Some(rc) = cari_rclone() {
            ok(&format!("rclone terpasang: {}", rclone::versi()));
            info(&format!("lokasi: {}", rc.display()));
            judul("Akun cloud");
            let akun = daftar_akun();
            if akun.is_empty() {
                awas("Belum ada. Contoh: pindah hubungkan -Akun google");
            }
            for a in &akun {
                ok(&ringkas_akun(a));
            }
        } else {
            awas("rclone belum terpasang. Jalankan: pindah pasang");
        }
        judul("Folder OneDrive di komputer ini");
        let od = daftar_onedrive();
        if od.is_empty() {
            awas("Tidak ditemukan. Pakai OneDrive lewat internet: pindah hubungkan -Akun onedrive");
        }
        for o in od {
            info(&format!("[{}] {}", o.jenis, o.folder));
        }
        judul("Rute penyalinan");
        let r = self.ambil_rute();
        match (&r.sumber, &r.tujuan) {
            (Some(s), t) => {
                info(&format!("Sumber : {s}"));
                info(&format!("Tujuan : {}", t.clone().unwrap_or_default()));
            }
            _ => info(&format!("Belum dipilih. Contoh: pindah periksa -Sumber \"{}\"", win::CONTOH_DATA)),
        }
        if let Some(job) = baca_job() {
            if job_berjalan(&job) {
                awas(&format!("Penyalinan sedang berjalan sejak {}. Lihat: pindah status", teks(job.get("Mulai"))));
            }
        }
        0
    }

    pub(super) fn perintah_akun(&mut self) -> i32 {
        judul("Akun cloud yang terhubung");
        if !self.wajib_rclone() {
            return 1;
        }
        let akun = daftar_akun();
        if akun.is_empty() {
            info("Belum ada. Contoh: pindah hubungkan -Akun google");
        }
        for a in &akun {
            info(&ringkas_akun(a));
        }
        self.catat("Akun", serde_json::to_value(&akun).unwrap_or(Value::Null));
        0
    }

    pub(super) fn pasang(&mut self) -> i32 {
        judul("Memasang rclone");
        if let Some(rc) = cari_rclone() {
            ok(&format!("rclone sudah terpasang: {}", rc.display()));
            return 0;
        }
        let Some(p) = pemasang_rclone() else {
            gagal(TANPA_PEMASANG[0]);
            info(TANPA_PEMASANG[1]);
            return 1;
        };
        info("rclone adalah program open source (lisensi MIT) untuk menyalin file antar-layanan cloud.");
        info(p.ket);
        if !self.konfirmasi(p.setuju) {
            return 1;
        }
        let _ = Command::new(&p.exe).args(p.args).stdin(Stdio::null()).tanpa_jendela().status();
        if let Some(rc) = cari_rclone() {
            ok(&format!("rclone terpasang: {}", rc.display()));
            return 0;
        }
        gagal("Pemasangan tidak berhasil. Tutup lalu buka lagi jendela ini, atau pasang manual dari https://rclone.org/downloads/");
        1
    }

    /// Menjadikan client ID milik sebuah akun Google sebagai bawaan aplikasi. Nilainya dibaca dari
    /// rclone.conf di dalam proses ini dan langsung ditulis ke client-google.json; tidak pernah dicetak.
    pub(super) fn bawaan(&mut self) -> i32 {
        if !self.wajib_rclone() {
            return 1;
        }
        let nama = nama_remote(&self.o.akun);
        if !daftar_remote().contains(&nama) {
            gagal(&format!("Akun {nama} tidak terhubung."));
            return 1;
        }
        let semua = json_dari(&panggil(&["config", "dump"], Mode::Tangkap).out);
        let r = semua.get(&nama);
        if r.map(|r| teks(r.get("type"))).as_deref() != Some("drive") {
            gagal(&format!("Akun {nama} bukan akun Google Drive."));
            return 1;
        }
        let r = r.unwrap();
        let (id, rahasia) = (teks(r.get("client_id")), teks(r.get("client_secret")));
        if id.is_empty() || rahasia.is_empty() {
            gagal(&format!("Akun {nama} masih memakai client ID bersama rclone; pasang client ID sendiri dulu."));
            return 1;
        }
        simpan_client_bawaan(&id, &rahasia);
        ok(&format!("Client ID akun {nama} disimpan sebagai bawaan aplikasi."));
        info(&format!("Lokasi: {} (ikut dibawa installer berikutnya).", jalur().client_pengguna.display()));
        0
    }

    /// Melepas akun dari LELEMOVE: hanya menghapus koneksi di rclone.conf, bukan file di cloud.
    /// Client ID Google aplikasi: tampilkan, simpan (-ClientId X -ClientSecret Y), atau hapus (-Hapus).
    /// Secret tidak pernah dicetak.
    pub(super) fn klien(&mut self) -> i32 {
        judul("Client ID Google aplikasi");
        if self.o.hapus {
            if hapus_client_pengguna() {
                ok("Client ID aplikasi yang disimpan di komputer ini dihapus.");
            } else {
                info("Tidak ada client ID yang disimpan di komputer ini.");
            }
        } else if let Some(id) = self.o.client_id.clone() {
            let rahasia = self.o.client_secret.clone().unwrap_or_default();
            if !id.ends_with(".apps.googleusercontent.com") {
                gagal("Client ID biasanya berakhiran \".apps.googleusercontent.com\".");
                return 1;
            }
            if rahasia.is_empty() {
                gagal("Tambahkan -ClientSecret.");
                return 1;
            }
            simpan_client_bawaan(&id, &rahasia);
            ok("Client ID aplikasi disimpan.");
        }
        match info_client_bawaan() {
            Some((id, asal)) => {
                ok(&format!("Terpasang: {} (dari {asal})", id_pendek(&id)));
                self.catat("Terpasang", true);
                self.catat("Asal", asal);
            }
            None => {
                awas("Belum ada. Akun Google memakai client ID bersama rclone yang sering dibatasi Google.");
                self.catat("Terpasang", false);
            }
        }
        let google: Vec<Akun> = daftar_akun().into_iter().filter(|a| a.jenis == "google").collect();
        let mut perlu = Vec::new();
        for a in &google {
            let status = if a.client_aplikasi {
                "memakai client ID aplikasi"
            } else if a.client_sendiri {
                "memakai client ID lain"
            } else {
                "memakai client ID bersama rclone"
            };
            info(&format!("{} [{}]: {status}", a.label, a.nama));
            if !a.client_aplikasi {
                perlu.push(a.nama.clone());
            }
        }
        if !perlu.is_empty() && info_client_bawaan().is_some() {
            info("Agar ikut memakai client ID aplikasi, login ulang: pindah hubungkan -Akun <akun> -ClientAplikasi");
        }
        self.catat("PerluLoginUlang", perlu);
        0
    }

    pub(super) fn lepas(&mut self) -> i32 {
        if !self.wajib_rclone() {
            return 1;
        }
        let nama = nama_remote(&self.o.akun);
        if !daftar_remote().contains(&nama) {
            info(&format!("Akun {nama} tidak terhubung."));
            simpan_meta_akun(&nama, &[], true);
            return 0;
        }
        if let Some(job) = baca_job() {
            let dipakai = remote_dari(&teks(job.get("Sumber"))) == nama || remote_dari(&teks(job.get("Tujuan"))) == nama;
            if job_berjalan(&job) && dipakai {
                gagal("Akun ini sedang dipakai oleh penyalinan yang berjalan. Hentikan dulu.");
                return 1;
            }
        }
        if !self.konfirmasi(&format!("Lepas akun {nama} dari LELEMOVE? File di cloud tidak terhapus.")) {
            return 1;
        }
        // Remote impor milik pengguna: hanya berhenti dipakai, remote rclone-nya dibiarkan.
        if meta(&nama, "Impor") == "1" {
            simpan_meta_akun(&nama, &[], true);
            ok(&format!("Remote {nama} tidak dipakai lagi oleh LELEMOVE (remote rclone-nya tetap ada)."));
            return 0;
        }
        let _ = panggil(&v!["config", "delete", &nama], Mode::Tangkap);
        simpan_meta_akun(&nama, &[], true);
        ok(&format!("Akun {nama} dilepas."));
        info("Untuk mencabut izin sepenuhnya, hapus akses \"rclone\" di pengaturan keamanan akun layanan itu.");
        0
    }
}
