//! Menemukan dan menjalankan rclone.

use std::ffi::OsStr;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use super::dasar::jalur;
use crate::win;

pub fn cari_rclone() -> Option<PathBuf> {
    let ada = |p: &Path| p.is_file();
    if let Ok(p) = std::env::var("PINDAH_DRIVE_RCLONE") {
        let p = PathBuf::from(p);
        if ada(&p) {
            return Some(p);
        }
    }
    let lokal = jalur().dir_app.join("rclone.exe");
    if ada(&lokal) {
        return Some(lokal);
    }
    if let Some(path) = std::env::var_os("PATH") {
        for d in std::env::split_paths(&path) {
            let p = d.join("rclone.exe");
            if ada(&p) {
                return Some(p);
            }
        }
    }
    let lad = PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_default());
    // winget memasang di sini; PATH di jendela yang sudah terbuka belum diperbarui.
    let link = lad.join("Microsoft\\WinGet\\Links\\rclone.exe");
    if ada(&link) {
        return Some(link);
    }
    if let Ok(isi) = std::fs::read_dir(lad.join("Microsoft\\WinGet\\Packages")) {
        for e in isi.flatten() {
            if e.file_name().to_string_lossy().starts_with("Rclone.Rclone") {
                if let Some(p) = cari_berkas(&e.path(), "rclone.exe", 3) {
                    return Some(p);
                }
            }
        }
    }
    // Aplikasi yang terpasang lewat installer (folder sekarang, lalu nama lama sebelum LELEMOVE).
    for folder in ["Programs\\LELEMOVE", "Programs\\Pindah Drive"] {
        let terpasang = lad.join(folder).join("rclone.exe");
        if ada(&terpasang) {
            return Some(terpasang);
        }
    }
    None
}

fn cari_berkas(dir: &Path, nama: &str, dalam: u32) -> Option<PathBuf> {
    let isi = std::fs::read_dir(dir).ok()?;
    let mut sub = Vec::new();
    for e in isi.flatten() {
        let p = e.path();
        if p.is_file() && e.file_name().to_string_lossy().eq_ignore_ascii_case(nama) {
            return Some(p);
        }
        if dalam > 0 && p.is_dir() {
            sub.push(p);
        }
    }
    sub.into_iter().find_map(|d| cari_berkas(&d, nama, dalam - 1))
}

pub struct Hasil {
    pub kode: i32,
    pub out: String,
    pub err: String,
}

impl Hasil {
    /// Baris terakhir stderr yang tidak kosong (untuk pesan galat singkat).
    pub fn galat_terakhir(&self, n: usize) -> Vec<String> {
        let b: Vec<String> = self
            .err
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
        b[b.len().saturating_sub(n)..].to_vec()
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    /// Kumpulkan stdout & stderr (tidak tampil).
    Tangkap,
    /// stdout dibuang (agar token login tidak tercetak), stderr tetap tampil.
    BuangStdout,
}

pub fn panggil<S: AsRef<OsStr>>(args: &[S], mode: Mode) -> Hasil {
    let Some(rc) = cari_rclone() else {
        return Hasil { kode: -1, out: String::new(), err: "rclone tidak ditemukan".into() };
    };
    let mut c = Command::new(rc);
    c.args(args).stdin(Stdio::null()).creation_flags(win::bendera_anak());
    match mode {
        Mode::Tangkap => match c.output() {
            Ok(o) => Hasil {
                kode: o.status.code().unwrap_or(1),
                out: String::from_utf8_lossy(&o.stdout).into_owned(),
                err: String::from_utf8_lossy(&o.stderr).into_owned(),
            },
            Err(e) => Hasil { kode: -1, out: String::new(), err: e.to_string() },
        },
        Mode::BuangStdout => {
            c.stdout(Stdio::null()).stderr(Stdio::inherit());
            match c.status() {
                Ok(s) => Hasil { kode: s.code().unwrap_or(1), out: String::new(), err: String::new() },
                Err(e) => Hasil { kode: -1, out: String::new(), err: e.to_string() },
            }
        }
    }
}

/// Seperti `panggil` (Mode::Tangkap), dengan `masukan` dikirim lewat stdin.
pub fn panggil_masukan<S: AsRef<OsStr>>(args: &[S], masukan: &str) -> Hasil {
    use std::io::Write;
    let Some(rc) = cari_rclone() else {
        return Hasil { kode: -1, out: String::new(), err: "rclone tidak ditemukan".into() };
    };
    let anak = Command::new(rc)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(win::bendera_anak())
        .spawn();
    let mut anak = match anak {
        Ok(a) => a,
        Err(e) => return Hasil { kode: -1, out: String::new(), err: e.to_string() },
    };
    if let Some(mut s) = anak.stdin.take() {
        let _ = s.write_all(masukan.as_bytes());
    } // stdin ditutup di sini
    match anak.wait_with_output() {
        Ok(o) => Hasil {
            kode: o.status.code().unwrap_or(1),
            out: String::from_utf8_lossy(&o.stdout).into_owned(),
            err: String::from_utf8_lossy(&o.stderr).into_owned(),
        },
        Err(e) => Hasil { kode: -1, out: String::new(), err: e.to_string() },
    }
}

/// Kata sandi -> bentuk terobfuskasi rclone (seperti yang disimpan di rclone.conf). Kata sandi
/// dikirim lewat stdin, jadi tidak pernah muncul polos di argumen proses mana pun.
pub fn obscure(rahasia: &str) -> Result<String, String> {
    let h = panggil_masukan(&["obscure", "-"], &format!("{rahasia}\n"));
    let o = h.out.trim().to_string();
    if h.kode != 0 || o.is_empty() {
        return Err(format!("rclone obscure gagal: {}", h.err.trim()));
    }
    Ok(o)
}

/// rclone yang berjalan lama (salin); keluaran ke konsol/log-file.
pub fn mulai<S: AsRef<OsStr>>(args: &[S]) -> std::io::Result<Child> {
    let rc = cari_rclone().ok_or_else(|| std::io::Error::other("rclone tidak ditemukan"))?;
    Command::new(rc).args(args).stdin(Stdio::null()).creation_flags(win::bendera_anak()).spawn()
}

pub fn versi() -> String {
    let h = panggil(&["version"], Mode::Tangkap);
    h.out.lines().next().unwrap_or("").trim().to_string()
}

pub fn daftar_remote() -> Vec<String> {
    if cari_rclone().is_none() {
        return Vec::new();
    }
    let h = panggil(&["listremotes"], Mode::Tangkap);
    h.out
        .lines()
        .map(|l| l.trim().trim_end_matches(':').to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// Pesan rclone yang berarti login sudah kedaluwarsa/dicabut dan harus diulang.
pub fn token_kedaluwarsa(err: &str) -> bool {
    let e = err.to_lowercase();
    e.contains("invalid_grant")
        || e.contains("token expired")
        || e.contains("expired or revoked")
        || e.contains("token has been expired")
        || e.contains("interaction_required")
        || e.contains("aadsts700082")
}

/// Galat jaringan (DNS menolak, koneksi putus, batas waktu habis): bukan masalah akun,
/// biasanya hilang sendiri bila dicoba lagi. Teks galat Go di Windows, mis.
/// `dial tcp: lookup www.googleapis.com: no such host` atau `getaddrinfow: ...`.
pub fn galat_jaringan(err: &str) -> bool {
    const TANDA: &[&str] = &[
        "no such host",
        "getaddrinfow",
        "dnsquery",
        "temporary error during hostname resolution",
        "dial tcp",
        "dial udp",
        "connectex",
        "wsarecv",
        "wsasend",
        "i/o timeout",
        "tls handshake timeout",
        "connection refused",
        "connection reset",
        "network is unreachable",
        "no route to host",
        "context deadline exceeded",
        "client.timeout exceeded",
        "timeout awaiting response headers",
    ];
    let e = err.to_lowercase();
    TANDA.iter().any(|t| e.contains(t))
}

#[cfg(test)]
mod uji {
    use super::*;

    #[test]
    fn jaringan() {
        assert!(galat_jaringan(
            r#"Failed to create file system for "pd-gdrive:": couldn't fetch token: Post "https://oauth2.googleapis.com/token": dial tcp: lookup oauth2.googleapis.com: no such host"#
        ));
        assert!(galat_jaringan("Get \"https://www.googleapis.com/drive/v3/about\": net/http: TLS handshake timeout"));
        assert!(galat_jaringan("read tcp 10.0.0.2:5000->142.250.4.95:443: wsarecv: An existing connection was forcibly closed"));
        assert!(!galat_jaringan("couldn't get about: googleapi: Error 403: Insufficient Permission"));
        assert!(!galat_jaringan("oauth2: \"invalid_grant\" \"Token has been expired or revoked.\""));
    }
}
