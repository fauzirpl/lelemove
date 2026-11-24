//! Menjalankan pindah.exe sebagai proses terpisah (tanpa jendela) dan meneruskan keluarannya
//! baris demi baris ke jendela lewat kanal, sehingga jendela tidak pernah membeku.

use eframe::egui;
use serde_json::Value;
use std::io::{BufRead, BufReader, Read};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;
use std::thread;

use pindah_drive::mesin::akun::{Akun, Pustaka, Sambungan, Situs};
use pindah_drive::mesin::jadwal::Jadwal;
use pindah_drive::mesin::riwayat::Catatan;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Kabar dari utas latar ke jendela.
pub enum Kabar {
    /// Baris teks dari aksi yang berjalan (masuk log aktivitas).
    Baris(String),
    /// Baris "##HASIL {json}" dari aksi yang berjalan.
    Hasil(Value),
    AksiSelesai(i32),
    Status { rclone: Option<PathBuf>, versi: String, akun: Vec<Akun>, client_bawaan: bool },
    Sambungan(String, Sambungan),
    Jelajah { gen: u64, hasil: Result<Vec<String>, String> },
    /// Isi folder untuk panel Dari/Ke.
    Daftar { gen: u64, isi: Isi },
    Situs { gen: u64, hasil: Result<Vec<Situs>, String> },
    Pustaka { gen: u64, hasil: Result<Vec<Pustaka>, String> },
    /// Daftar jadwal backup + (tugas Windows-nya masih ada, prosesnya sedang berjalan).
    Jadwal { gen: u64, daftar: Vec<(Jadwal, bool, bool)> },
    /// Isi riwayat.jsonl (terbaru dulu).
    Riwayat { gen: u64, daftar: Vec<Catatan> },
    /// Remote rclone di luar LELEMOVE: (nama, backend).
    Impor { gen: u64, calon: Vec<(String, String)> },
}

/// Isi satu folder di panel Dari/Ke (hanya subfolder; yang disalin selalu seluruh isi folder).
pub enum Isi {
    Folder(Vec<String>),
    /// Akar "Komputer ini": daftar drive (akar, nama tampilan).
    Drive(Vec<(String, String)>),
    /// Folder belum ada (wajar untuk tujuan: dibuat saat menyalin).
    BelumAda,
    Galat(String),
}

/// Apa yang dilakukan jendela setelah aksi selesai.
pub enum Setelah {
    Pasang,
    Akun { ok: String, gagal: String },
    /// Setelah login SharePoint: langsung pilih situs & pustaka.
    AkunSharePoint,
    Periksa,
    Uji,
    Salin,
    Verifikasi,
    /// Perubahan jadwal backup: buka lagi daftar jadwal dengan pesan ini.
    Jadwal { ok: String },
    /// Pindahkan (hapus dari sumber file yang terverifikasi).
    Pindahkan,
    /// Menghubungkan akun layanan lain: bisa berakhir dengan pertanyaan lanjutan dari rclone.
    AkunPenyedia { nama: String, kunci: String },
}

pub struct Aksi {
    pub judul: String,
    pub pid: u32,
    pub hasil: Option<Value>,
    pub setelah: Setelah,
    pub dibatalkan: bool,
}

fn baca_baris<R: Read + Send + 'static>(r: R, tx: Sender<Kabar>, ctx: egui::Context) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        for b in BufReader::new(r).split(b'\n') {
            let Ok(b) = b else { break };
            let s = String::from_utf8_lossy(&b).trim_end_matches('\r').to_string();
            let kabar = match s.strip_prefix("##HASIL ") {
                Some(j) => match serde_json::from_str(j) {
                    Ok(v) => Kabar::Hasil(v),
                    Err(_) => continue,
                },
                None => Kabar::Baris(s),
            };
            if tx.send(kabar).is_err() {
                break;
            }
            ctx.request_repaint();
        }
    })
}

/// Menjalankan `pindah.exe <args> -Mesin`. Hasilnya datang sebagai Kabar::Baris/Hasil, lalu
/// Kabar::AksiSelesai setelah semua keluaran terbaca.
/// Dengan `masukan` (mis. JSON isian rahasia), masukan dikirim lewat stdin lalu ditutup.
/// Tanpa masukan, stdin kosong: proses anak tidak pernah menunggu ketikan.
pub fn mulai_masukan(exe: &Path, args: &[String], masukan: Option<String>, tx: Sender<Kabar>, ctx: egui::Context) -> std::io::Result<u32> {
    use std::io::Write;
    let mut anak = Command::new(exe)
        .args(args)
        .arg("-Mesin")
        .stdin(if masukan.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()?;
    if let (Some(m), Some(mut s)) = (masukan, anak.stdin.take()) {
        let _ = s.write_all(m.as_bytes());
    }
    let pid = anak.id();
    let out = anak.stdout.take().unwrap();
    let err = anak.stderr.take().unwrap();
    thread::spawn(move || {
        let a = baca_baris(out, tx.clone(), ctx.clone());
        let b = baca_baris(err, tx.clone(), ctx.clone());
        let _ = a.join();
        let _ = b.join();
        let kode = anak.wait().ok().and_then(|s| s.code()).unwrap_or(1);
        let _ = tx.send(Kabar::AksiSelesai(kode));
        ctx.request_repaint();
    });
    Ok(pid)
}

/// Menjalankan pekerjaan ringan di utas latar; hasilnya dikirim sebagai satu Kabar.
pub fn latar<F>(tx: &Sender<Kabar>, ctx: &egui::Context, kerja: F)
where
    F: FnOnce() -> Kabar + Send + 'static,
{
    let (tx, ctx) = (tx.clone(), ctx.clone());
    thread::spawn(move || {
        let _ = tx.send(kerja());
        ctx.request_repaint();
    });
}
