//! Perintah-perintah mesin (sama dengan pindah.ps1), dipanggil dari pindah.exe.
//!
//! Dengan opsi -Mesin, ringkasan hasil dicetak sebagai satu baris "##HASIL {json}" untuk jendela
//! aplikasi; baris lain adalah teks biasa yang masuk ke panel "Log aktivitas".

use regex::Regex;
use serde_json::{Map, Value};
use std::path::Path;
use std::sync::LazyLock;

use super::akun::*;
use super::dasar::*;
use super::filter::{baca_filter, simpan_filter, Filter};
use super::rclone::{cari_rclone, daftar_remote, token_kedaluwarsa};
use super::rute::*;
use crate::v;

mod akun;
mod bantuan;
mod email;
mod hubungkan;
mod jadwal;
mod menu;
mod opsi;
mod pembaruan;
mod periksa;
mod pindahkan;
mod riwayat;
mod salin;
mod verifikasi;

pub use akun::ringkas_akun;
pub use bantuan::tampilkan_bantuan;
pub use opsi::{urai_argumen, Opsi};

pub const PERINTAH: &[&str] = &[
    "menu", "cek", "akun", "pasang", "hubungkan", "lepas", "bawaan", "klien", "situs", "pustaka", "periksa", "uji",
    "salin", "status", "berhenti", "verifikasi", "pindahkan", "jadwal", "riwayat", "penyedia", "impor", "email", "pembaruan", "bantuan",
];

static RE_FS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"file system for "([^":]+):"#).unwrap());
static RE_AWAL_ERROR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\S+\s+\S+\s+ERROR\s*:\s*").unwrap());

/// SharePoint mengubah metadata file Office (.docx/.xlsx/...) saat diunggah, sehingga ukuran &
/// checksum di tujuan berbeda dari sumber. Tanpa flag ini rclone menganggapnya rusak dan mengulang.
fn argumen_tujuan(tujuan: &str) -> Vec<String> {
    let rem = remote_dari(tujuan);
    if !rem.is_empty() && jenis_dari_nama(&rem) == "sharepoint" {
        return v!["--ignore-size", "--ignore-checksum"];
    }
    Vec::new()
}

fn json_dari(s: &str) -> Value {
    serde_json::from_str(s).unwrap_or(Value::Null)
}

/// "pd-gdrive-2:Arsip" -> "Google Drive 2: Arsip";  "D:\Data" tetap.
fn nama_tempat(j: &str) -> String {
    let rem = remote_dari(j);
    if rem.is_empty() {
        return j.to_string();
    }
    let jenis = jenis_dari_nama(&rem);
    let dasar = label_bawaan("", jenis);
    let mut label = meta(&rem, "Label");
    if label.is_empty() {
        label = label_bawaan(&rem, jenis);
    } else if !label.starts_with(&dasar) {
        label = format!("{dasar} \u{00B7} {label}");
    }
    let folder = j[rem.len() + 1..].trim_matches('/');
    if folder.is_empty() {
        label
    } else {
        format!("{label}: {folder}")
    }
}

fn potong(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_string();
    }
    format!("{}\u{2026}", s.chars().take(n).collect::<String>().trim_end())
}

// ------------------------------------------------------------------ mesin

pub struct Mesin {
    pub o: Opsi,
    pub hasil: Map<String, Value>,
    /// Penyalinan ini dijalankan oleh jadwal: notifikasi "selesai" dikirim setelah verifikasi.
    dari_jadwal: bool,
    /// Nama jadwal yang sedang dijalankan (dicatat di riwayat).
    nama_jadwal: String,
    /// Isian rahasia dari stdin (-RahasiaStdin), dibaca sekali.
    rahasia: Option<Map<String, Value>>,
}

impl Mesin {
    pub fn baru(o: Opsi) -> Self {
        Mesin { o, hasil: Map::new(), dari_jadwal: false, nama_jadwal: String::new(), rahasia: None }
    }

    /// Filter file untuk perintah ini: yang tersimpan, atau dari -Sertakan/-Lewati/-Sejak
    /// (lalu disimpan, seperti -Sumber/-Tujuan). -TanpaFilter menghapusnya. None = isian salah.
    fn ambil_filter(&mut self) -> Option<Filter> {
        if self.o.tanpa_filter {
            simpan_filter(&Filter::default());
            return Some(Filter::default());
        }
        let lama = baca_filter();
        if self.o.sertakan.is_none() && self.o.lewati.is_none() && self.o.sejak.is_none() {
            return Some(lama);
        }
        let s = self.o.sertakan.clone().unwrap_or_else(|| lama.teks_sertakan());
        let l = self.o.lewati.clone().unwrap_or_else(|| lama.teks_lewati());
        let t = self.o.sejak.clone().unwrap_or_else(|| lama.sejak.clone().unwrap_or_default());
        match Filter::urai(&s, &l, &t) {
            Ok(f) => {
                simpan_filter(&f);
                Some(f)
            }
            Err(e) => {
                gagal(&e);
                None
            }
        }
    }

    fn info_filter(&mut self, f: &Filter) {
        self.catat("Filter", f.uraian());
        if !f.kosong() {
            info(&format!("Filter: {}", f.uraian()));
        }
    }

    fn catat(&mut self, k: &str, v: impl Into<Value>) {
        self.hasil.insert(k.to_string(), v.into());
    }

    pub fn cetak_hasil(&self) {
        if self.o.mesin && !self.hasil.is_empty() {
            polos(&format!("##HASIL {}", Value::Object(self.hasil.clone())));
        }
    }

    fn konfirmasi(&self, tanya: &str) -> bool {
        if self.o.ya {
            return true;
        }
        if !bisa_bertanya() {
            awas(&format!("Butuh persetujuan: {tanya}"));
            info("Ulangi perintah dengan tambahan -Ya setelah disetujui.");
            return false;
        }
        let j = baca_baris(&format!("        {tanya} (y/n) ")).to_lowercase();
        matches!(j.as_str(), "y" | "ya" | "yes")
    }

    fn wajib_rclone(&self) -> bool {
        if cari_rclone().is_some() {
            return true;
        }
        gagal("rclone belum terpasang. Jalankan: pindah pasang");
        false
    }

    pub fn jalankan(&mut self) -> i32 {
        match self.o.perintah.clone().as_str() {
            "menu" => {
                self.menu();
                0
            }
            "cek" => self.cek(),
            "akun" => self.perintah_akun(),
            "pasang" => self.pasang(),
            "hubungkan" => self.hubungkan(),
            "lepas" => self.lepas(),
            "bawaan" => self.bawaan(),
            "klien" => self.klien(),
            "jadwal" => self.jadwal(),
            "situs" => self.situs(),
            "pustaka" => self.pustaka(),
            "periksa" => self.periksa(),
            "uji" => self.uji(),
            "salin" => self.salin(),
            "status" => self.status(),
            "berhenti" => self.berhenti(),
            "verifikasi" => self.verifikasi(),
            "pindahkan" => self.pindahkan(),
            "riwayat" => self.riwayat(),
            "penyedia" => self.daftar_penyedia(),
            "impor" => self.impor(),
            "email" => self.email(),
            "pembaruan" => self.pembaruan(),
            _ => {
                tampilkan_bantuan();
                0
            }
        }
    }

    /// Bila galat rclone berarti login kedaluwarsa: beri tahu akun mana yang perlu login ulang.
    fn saran_login(&mut self, err: &str, jalur_terkait: &[&str]) {
        if !token_kedaluwarsa(err) {
            return;
        }
        let rem = RE_FS.captures(err).map(|c| c[1].to_string()).unwrap_or_else(|| {
            jalur_terkait.iter().map(|j| remote_dari(j)).find(|r| !r.is_empty()).unwrap_or_default()
        });
        if rem.is_empty() {
            return;
        }
        awas(&format!("Login akun {rem} sudah kedaluwarsa atau dicabut."));
        info(&format!("Login ulang: di aplikasi klik kanan akun itu di daftar kiri > Login ulang, atau jalankan: pindah hubungkan -Akun {rem} -Ulang"));
        self.catat("PerluLogin", rem);
    }

    // -------------------------------------------------------------- rute

    /// Menggabungkan -Sumber/-Tujuan dari baris perintah dengan pengaturan tersimpan.
    fn ambil_rute(&self) -> Rute {
        let p = baca_pengaturan();
        let (mut src, mut dst) = (p.sumber, p.tujuan);
        if let Some(s) = &self.o.sumber {
            let s = rapikan_sumber(s);
            if self.o.tujuan.is_none() {
                dst = Some(tujuan_bawaan(&s, REMOTE_GOOGLE));
            }
            src = Some(s);
        }
        if let Some(t) = &self.o.tujuan {
            dst = Some(normalisasi_tujuan(t));
        }
        if let (Some(s), None) = (&src, &dst) {
            dst = Some(tujuan_bawaan(s, REMOTE_GOOGLE));
        }
        if self.o.sumber.is_some() || self.o.tujuan.is_some() {
            simpan_pengaturan(src.as_deref(), dst.as_deref());
        }
        Rute { sumber: src, tujuan: dst }
    }

    fn rute_siap(&self, r: &Rute, perlu_akun: bool) -> bool {
        let Some(src) = r.sumber.as_deref() else {
            gagal("Folder sumber belum dipilih. Tambahkan -Sumber \"<folder>\" atau pilih lewat menu.");
            return false;
        };
        let Some(dst) = r.tujuan.as_deref() else {
            gagal("Folder tujuan belum dipilih. Tambahkan -Tujuan \"<akun>:<folder>\".");
            return false;
        };
        if adalah_lokal(src) && !Path::new(src).is_dir() {
            gagal(&format!("Folder sumber tidak ditemukan: {src}"));
            return false;
        }
        // Menyalin ke dalam dirinya sendiri akan berulang tanpa akhir.
        let s = src.replace('\\', "/").trim_end_matches('/').to_lowercase();
        let t = dst.replace('\\', "/").trim_end_matches('/').to_lowercase();
        let awalan = if s.ends_with(':') { s.clone() } else { format!("{s}/") };
        if t == s || t.starts_with(&awalan) {
            gagal("Tujuan tidak boleh sama dengan sumber atau berada di dalam folder sumber.");
            return false;
        }
        if perlu_akun {
            if !self.wajib_rclone() {
                return false;
            }
            let remote = daftar_remote();
            for (peran, j) in [("sumber", src), ("tujuan", dst)] {
                let rem = remote_dari(j);
                if !rem.is_empty() && !remote.contains(&rem) {
                    gagal(&format!("Akun '{rem}' untuk {peran} belum terhubung. Jalankan: pindah hubungkan -Akun {rem}"));
                    return false;
                }
            }
            for j in [src, dst] {
                let rem = remote_dari(j);
                if !rem.is_empty() && jenis_dari_nama(&rem) == "sharepoint" && meta(&rem, "DriveId").is_empty() {
                    gagal(&format!(
                        "Akun SharePoint '{rem}' belum memilih situs & pustaka. Jalankan: pindah situs -Akun {rem} -Cari <kata>"
                    ));
                    return false;
                }
            }
            let rem_s = remote_dari(src);
            if !rem_s.is_empty() {
                let k = konfig_akun(&rem_s);
                if k.tipe == "drive" && k.scope == "drive.file" {
                    gagal(&format!(
                        "Akun Google '{rem_s}' hanya diberi izin terbatas (drive.file), jadi tidak bisa membaca file Anda."
                    ));
                    info(&format!("Beri izin penuh dulu: pindah hubungkan -Akun {rem_s} -IzinPenuh"));
                    return false;
                }
            }
        }
        true
    }
}
