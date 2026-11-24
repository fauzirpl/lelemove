//! Lapisan sistem macOS: padanan win.rs dengan nama fungsi dan tanda tangan yang sama, sehingga
//! kode lain cukup memanggil `win::...` di kedua sistem. Semua fungsi aman dipanggil dan tidak
//! panik: kegagalan dilaporkan sebagai false/None/0.
//!
//! Panggilan sistem lewat libc (proses, statfs, flock, localtime); Tong Sampah lewat
//! NSFileManager (objc2), sama seperti Finder, sehingga file bisa dikembalikan dengan "Put Back".

use std::ffi::{CStr, CString, OsStr};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

// Foundation (NSString, NSURL, NSFileManager) dan CoreGraphics ditautkan langsung: program
// `pindah` tidak memakai AppKit, jadi tidak boleh mengandalkan tautan dari eframe/winit.
#[link(name = "Foundation", kind = "framework")]
extern "C" {}

#[repr(C)]
#[derive(Clone, Copy)]
struct CGRect {
    x: f64,
    y: f64,
    lebar: f64,
    tinggi: f64,
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGMainDisplayID() -> u32;
    fn CGDisplayBounds(display: u32) -> CGRect;
}

// ------------------------------------------------------------------ nama & teks khas sistem

pub const NAMA_OS: &str = "macOS";
pub const NAMA_SAMPAH: &str = "Tong Sampah";
pub const PENJADWAL: &str = "launchd macOS";
pub const NAMA_TUGAS: &str = "tugas launchd";
pub const PINTASAN_CETAK: &str = "Cmd+P";
pub const CONTOH_DATA: &str = "/Users/nama/Data";
pub const CONTOH_SALINAN: &str = "/Users/nama/Salinan";

// Nilai sama dengan Win32 agar pemanggil bisa memakai konstanta yang sama di kedua sistem.
pub const MB_OK: u32 = 0x0;
pub const MB_YESNO: u32 = 0x4;
pub const MB_RETRYCANCEL: u32 = 0x5;
pub const MB_ICONERROR: u32 = 0x10;
pub const MB_ICONQUESTION: u32 = 0x20;
pub const MB_ICONWARNING: u32 = 0x30;
pub const MB_ICONINFORMATION: u32 = 0x40;
pub const IDOK: i32 = 1;
pub const IDCANCEL: i32 = 2;
pub const IDRETRY: i32 = 4;
pub const IDYES: i32 = 6;
pub const IDNO: i32 = 7;

/// Lihat win.rs. Di macOS program konsol tidak pernah membuka jendela sendiri.
pub trait PerintahExt {
    fn tanpa_jendela(&mut self) -> &mut Self;
    fn grup_sendiri(&mut self) -> &mut Self;
}

impl PerintahExt for Command {
    fn tanpa_jendela(&mut self) -> &mut Self {
        self
    }
    fn grup_sendiri(&mut self) -> &mut Self {
        self.process_group(0)
    }
}

/// Teks berkutip untuk AppleScript (osascript).
pub fn teks_applescript(s: &str) -> String {
    let mut t = String::with_capacity(s.len() + 2);
    t.push('"');
    for c in s.chars() {
        match c {
            '\\' => t.push_str("\\\\"),
            '"' => t.push_str("\\\""),
            '\n' => t.push_str("\\n"),
            c if (c as u32) < 0x20 => t.push(' '),
            c => t.push(c),
        }
    }
    t.push('"');
    t
}

/// Teks berkutip tunggal untuk /bin/sh.
fn kutip_sh(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

// ------------------------------------------------------------------ konsol

/// Terminal macOS memahami UTF-8 dan warna ANSI tanpa pengaturan apa pun.
pub fn atur_konsol() -> bool {
    true
}

// ------------------------------------------------------------------ proses

/// Proses anak yang tidak ditunggu menjadi zombie sampai induknya keluar, dan zombie masih
/// terhitung ada. Utas kecil ini menunggunya agar langsung dibersihkan begitu selesai.
fn tunggu_latar(mut anak: Child) {
    std::thread::spawn(move || {
        let _ = anak.wait();
    });
}

/// Jalur program sebuah proses yang masih hidup.
pub fn jalur_proses(pid: u32) -> Option<PathBuf> {
    if pid == 0 {
        return None;
    }
    let mut buf = vec![0u8; 4096]; // PROC_PIDPATHINFO_MAXSIZE
    let n = unsafe { libc::proc_pidpath(pid as libc::c_int, buf.as_mut_ptr().cast(), buf.len() as u32) };
    if n <= 0 {
        return None;
    }
    buf.truncate(n as usize);
    Some(PathBuf::from(OsStr::from_bytes(&buf)))
}

/// Proses `pid` masih hidup dan nama programnya salah satu dari `nama`.
pub fn proses_hidup(pid: u32, nama: &[&str]) -> bool {
    match jalur_proses(pid).and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string())) {
        Some(n) => nama.iter().any(|x| n.eq_ignore_ascii_case(x)),
        None => false,
    }
}

pub fn bunuh(pid: u32) -> bool {
    pid > 0 && pid != std::process::id() && unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) } == 0
}

/// Menghentikan proses beserta turunannya. Proses yang dimulai dengan `grup_sendiri` atau
/// `mulai_terlepas` memimpin grupnya sendiri, jadi seluruh grup (mis. pindah + rclone) dihentikan.
pub fn bunuh_pohon(pid: u32) {
    if pid == 0 || pid == std::process::id() {
        return;
    }
    let p = pid as libc::pid_t;
    unsafe {
        if libc::getpgid(p) == p && p != libc::getpgrp() {
            libc::kill(-p, libc::SIGKILL);
        }
        libc::kill(p, libc::SIGKILL);
    }
}

/// Menjalankan program yang terlepas dari proses ini: sesi baru (tidak ikut berhenti saat
/// terminal atau jendela induknya ditutup), tanpa stdin/stdout/stderr bersama.
pub fn mulai_terlepas(exe: &Path, args: &[String]) -> std::io::Result<u32> {
    let mut c = Command::new(exe);
    c.args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    unsafe {
        c.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    let anak = c.spawn()?;
    let pid = anak.id();
    tunggu_latar(anak);
    Ok(pid)
}

/// Menjalankan program konsol interaktif (mis. `rclone config`) di jendela Terminal baru.
/// Lewat berkas .command sementara: tidak perlu izin "mengendalikan Terminal" (Automation).
pub fn buka_di_terminal(exe: &Path, args: &[&str]) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut baris = kutip_sh(&exe.to_string_lossy());
    for a in args {
        baris.push(' ');
        baris.push_str(&kutip_sh(a));
    }
    let mut skrip = String::from("#!/bin/sh\nrm -f \"$0\"\nclear\n");
    if let Ok(k) = std::env::var("RCLONE_CONFIG") {
        skrip.push_str(&format!("export RCLONE_CONFIG={}\n", kutip_sh(&k)));
    }
    skrip.push_str(&baris);
    skrip.push('\n');
    let f = std::env::temp_dir().join(format!("lelemove-{}.command", std::process::id()));
    std::fs::write(&f, skrip)?;
    std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o700))?;
    let anak = Command::new("/usr/bin/open").args(["-a", "Terminal"]).arg(&f).stdin(Stdio::null()).spawn()?;
    tunggu_latar(anak);
    Ok(())
}

// ------------------------------------------------------------------ sistem

static KAFEIN: Mutex<Option<Child>> = Mutex::new(None);

/// Mencegah Mac tidur karena diam selama penyalinan. `caffeinate -w` berhenti sendiri bila
/// proses ini berakhir, jadi tidak ada yang tertinggal walau proses dihentikan paksa.
pub fn jaga_bangun(aktif: bool) {
    let Ok(mut k) = KAFEIN.lock() else { return };
    if let Some(mut c) = k.take() {
        let _ = c.kill();
        let _ = c.wait();
    }
    if aktif {
        *k = Command::new("/usr/bin/caffeinate")
            .args(["-i", "-w", &std::process::id().to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok();
    }
}

/// statfs folder `p` atau induk terdekatnya yang sudah ada (tujuan boleh belum dibuat).
fn statfs(p: &Path) -> Option<libc::statfs> {
    let ada = p.ancestors().find(|a| !a.as_os_str().is_empty() && a.exists())?;
    let c = CString::new(ada.as_os_str().as_bytes()).ok()?;
    let mut s: libc::statfs = unsafe { std::mem::zeroed() };
    (unsafe { libc::statfs(c.as_ptr(), &mut s) } == 0).then_some(s)
}

/// Sisa ruang disk tempat `path` berada (folder boleh belum ada).
pub fn ruang_bebas(path: &Path) -> u64 {
    statfs(path).map_or(0, |s| s.f_bavail as u64 * s.f_bsize as u64)
}

/// Volume lokal (disk internal, SSD/flashdisk USB) punya Tong Sampah; volume jaringan
/// (SMB/AFP/NFS) tidak: berkas yang "dibuang" di sana langsung hilang permanen.
pub fn drive_tetap(path: &Path) -> bool {
    const MNT_LOCAL: u32 = 0x0000_1000;
    statfs(path).is_some_and(|s| s.f_flags & MNT_LOCAL != 0)
}

/// Memindahkan berkas ke Tong Sampah (bisa dikembalikan lewat Finder > Put Back).
/// Hasil: berkas yang benar-benar sudah tidak ada di tempat asalnya.
pub fn ke_recycle_bin(berkas: &[PathBuf]) -> usize {
    use objc2::rc::autoreleasepool;
    use objc2::runtime::{AnyObject, Bool};
    use objc2::{class, msg_send};
    for p in berkas.iter().filter(|p| p.is_file()) {
        let Ok(c) = CString::new(p.as_os_str().as_bytes()) else { continue };
        autoreleasepool(|_| unsafe {
            let s: *mut AnyObject = msg_send![class!(NSString), stringWithUTF8String: c.as_ptr()];
            if s.is_null() {
                return;
            }
            let url: *mut AnyObject = msg_send![class!(NSURL), fileURLWithPath: s];
            if url.is_null() {
                return;
            }
            let fm: *mut AnyObject = msg_send![class!(NSFileManager), defaultManager];
            let _: Bool = msg_send![
                fm,
                trashItemAtURL: url,
                resultingItemURL: std::ptr::null_mut::<*mut AnyObject>(),
                error: std::ptr::null_mut::<*mut AnyObject>()
            ];
        });
    }
    berkas.iter().filter(|p| !p.exists()).count()
}

/// Membuka berkas, folder, atau alamat web dengan program bawaan (seperti klik dua kali di Finder).
pub fn buka(target: &str) {
    if let Ok(a) = Command::new("/usr/bin/open").arg(target).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn() {
        tunggu_latar(a);
    }
}

/// Kotak pesan sistem lewat osascript. `gaya` memakai konstanta MB_* seperti di Windows.
pub fn kotak_pesan(judul: &str, isi: &str, gaya: u32) -> i32 {
    let (tombol, bawaan): (&[&str], &str) = match gaya & 0xF {
        MB_YESNO => (&["Tidak", "Ya"], "Ya"),
        MB_RETRYCANCEL => (&["Batal", "Coba lagi"], "Coba lagi"),
        _ => (&["OK"], "OK"),
    };
    let jenis = match gaya & 0xF0 {
        MB_ICONERROR => "critical",
        MB_ICONWARNING => "warning",
        _ => "informational",
    };
    let daftar: Vec<String> = tombol.iter().map(|t| teks_applescript(t)).collect();
    let skrip = format!(
        "display alert {} message {} as {jenis} buttons {{{}}} default button {}",
        teks_applescript(judul),
        teks_applescript(isi),
        daftar.join(", "),
        teks_applescript(bawaan)
    );
    let keluar = Command::new("/usr/bin/osascript").arg("-e").arg(skrip).stdin(Stdio::null()).output();
    let dipilih = keluar
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    // Keluaran osascript: "button returned:Ya"
    match dipilih.rsplit(':').next().unwrap_or("") {
        "Ya" => IDYES,
        "Tidak" => IDNO,
        "Coba lagi" => IDRETRY,
        "OK" => IDOK,
        _ => IDCANCEL,
    }
}

/// Kunci satu-jendela (flock pada berkas di folder sementara milik pengguna). Dilepas otomatis
/// saat proses berakhir, termasuk bila dihentikan paksa.
pub struct KunciTunggal(#[allow(dead_code)] Option<std::fs::File>);

fn berkas_kunci(nama: &str) -> PathBuf {
    let bersih: String = nama.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    std::env::temp_dir().join(format!("lelemove{bersih}.kunci"))
}

/// None bila kunci sudah dipegang proses lain (aplikasi sudah terbuka).
pub fn kunci_tunggal(nama: &str) -> Option<KunciTunggal> {
    use std::os::fd::AsRawFd;
    let f = match std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(berkas_kunci(nama)) {
        Ok(f) => f,
        // Berkas kunci tidak bisa dibuat: jangan halangi aplikasi dibuka.
        Err(_) => return Some(KunciTunggal(None)),
    };
    if unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
        return Some(KunciTunggal(Some(f)));
    }
    let e = std::io::Error::last_os_error();
    (e.raw_os_error() != Some(libc::EWOULDBLOCK)).then_some(KunciTunggal(None))
}

pub fn mutex_ada(nama: &str) -> bool {
    kunci_tunggal(nama).is_none()
}

/// Tidak ada padanannya di macOS: ikon Dock mengikuti bundel .app.
pub fn set_app_id(_id: &str) {}

pub enum Folder {
    Dokumen,
    /// Folder Applications.
    Program,
    Desktop,
}

/// Mengisi `buf` dengan byte acak dari generator sistem (arc4random).
pub fn isi_acak(buf: &mut [u8]) {
    unsafe { libc::arc4random_buf(buf.as_mut_ptr().cast(), buf.len()) };
}

/// Membaca satu baris dari terminal tanpa menampilkan ketikan (kata sandi).
pub fn baca_sandi() -> String {
    use std::os::fd::AsRawFd;
    let fd = std::io::stdin().as_raw_fd();
    let mut lama: libc::termios = unsafe { std::mem::zeroed() };
    let terminal = unsafe { libc::tcgetattr(fd, &mut lama) } == 0;
    if terminal {
        let mut baru = lama;
        baru.c_lflag &= !libc::ECHO;
        unsafe { libc::tcsetattr(fd, libc::TCSANOW, &baru) };
    }
    let mut s = String::new();
    let _ = std::io::stdin().read_line(&mut s);
    if terminal {
        unsafe { libc::tcsetattr(fd, libc::TCSANOW, &lama) };
        println!();
    }
    s.trim_end_matches(['\r', '\n']).to_string()
}

/// Folder pribadi pengguna ($HOME, atau dari basis data pengguna bila HOME kosong).
pub fn folder_rumah() -> PathBuf {
    if let Some(h) = std::env::var_os("HOME").filter(|h| !h.is_empty()) {
        return PathBuf::from(h);
    }
    unsafe {
        let pw = libc::getpwuid(libc::getuid());
        if !pw.is_null() && !(*pw).pw_dir.is_null() {
            return PathBuf::from(OsStr::from_bytes(CStr::from_ptr((*pw).pw_dir).to_bytes()));
        }
    }
    PathBuf::new()
}

pub fn folder(f: Folder) -> Option<PathBuf> {
    let r = folder_rumah();
    if r.as_os_str().is_empty() {
        return None;
    }
    Some(match f {
        Folder::Dokumen => r.join("Documents"),
        Folder::Desktop => r.join("Desktop"),
        Folder::Program => PathBuf::from("/Applications"),
    })
}

/// ~/Library/Application Support (padanan %LOCALAPPDATA%).
pub fn folder_data() -> PathBuf {
    folder_rumah().join("Library").join("Application Support")
}

/// Nama komputer di jaringan, tanpa akhiran ".local".
pub fn nama_komputer() -> String {
    let mut b = [0u8; 256];
    if unsafe { libc::gethostname(b.as_mut_ptr().cast(), b.len()) } != 0 {
        return String::new();
    }
    let n = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    let s = String::from_utf8_lossy(&b[..n]).to_string();
    s.strip_suffix(".local").map(str::to_string).unwrap_or(s)
}

pub fn nama_pengguna() -> String {
    if let Ok(u) = std::env::var("USER") {
        if !u.is_empty() {
            return u;
        }
    }
    unsafe {
        let pw = libc::getpwuid(libc::getuid());
        if !pw.is_null() && !(*pw).pw_name.is_null() {
            return CStr::from_ptr((*pw).pw_name).to_string_lossy().to_string();
        }
    }
    String::new()
}

/// Waktu lokal: (tahun, bulan, tanggal, jam, menit, detik, hari dalam minggu 0 = Minggu).
pub fn jam_lokal() -> (u16, u16, u16, u16, u16, u16, u8) {
    unsafe {
        let t = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&t, &mut tm);
        (
            (tm.tm_year + 1900) as u16,
            (tm.tm_mon + 1) as u16,
            tm.tm_mday as u16,
            tm.tm_hour as u16,
            tm.tm_min as u16,
            tm.tm_sec.min(59) as u16,
            tm.tm_wday as u8,
        )
    }
}

/// Folder yang disembunyikan Finder: berawalan titik atau bertanda UF_HIDDEN (/bin, /usr, ...).
pub fn tersembunyi(m: &std::fs::Metadata, nama: &OsStr) -> bool {
    use std::os::macos::fs::MetadataExt;
    const UF_HIDDEN: u32 = 0x0000_8000;
    nama.as_bytes().first() == Some(&b'.') || m.st_flags() & UF_HIDDEN != 0
}

/// File iCloud Drive/OneDrive yang isinya belum diunduh (SF_DATALESS).
pub fn hanya_cloud(m: &std::fs::Metadata) -> bool {
    use std::os::macos::fs::MetadataExt;
    const SF_DATALESS: u32 = 0x4000_0000;
    m.st_flags() & SF_DATALESS != 0
}

/// Nama volume sistem ("Macintosh HD"): /Volumes memuat tautan bernama itu ke "/".
pub fn nama_akar() -> String {
    if let Ok(isi) = std::fs::read_dir("/Volumes") {
        for e in isi.flatten() {
            if std::fs::read_link(e.path()).is_ok_and(|t| t == Path::new("/")) {
                return e.file_name().to_string_lossy().to_string();
            }
        }
    }
    "Macintosh HD".into()
}

/// Lokasi akar di "Komputer ini": folder pribadi, volume sistem, lalu volume lain (/Volumes).
pub fn daftar_drive() -> Vec<(String, String)> {
    let mut v = Vec::new();
    let rumah = folder_rumah();
    if rumah.is_dir() {
        v.push((rumah.to_string_lossy().to_string(), format!("Folder pribadi ({})", nama_pengguna())));
    }
    v.push(("/".into(), nama_akar()));
    if let Ok(isi) = std::fs::read_dir("/Volumes") {
        // Jenis dari entri direktori saja: volume jaringan yang tidak terjangkau tidak ditanya.
        let mut lain: Vec<(String, String)> = isi
            .flatten()
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .filter(|e| !e.file_name().as_bytes().starts_with(b"."))
            .map(|e| (e.path().to_string_lossy().to_string(), e.file_name().to_string_lossy().to_string()))
            .collect();
        lain.sort_by_key(|(_, n)| n.to_lowercase());
        v.extend(lain);
    }
    v
}

/// Tinggi layar utama dalam point, dikurangi perkiraan bilah menu dan Dock.
pub fn tinggi_area_kerja() -> Option<f32> {
    let r = unsafe { CGDisplayBounds(CGMainDisplayID()) };
    (r.tinggi > 0.0).then(|| (r.tinggi - 70.0) as f32)
}

#[cfg(test)]
mod uji {
    use super::*;

    #[test]
    fn kutipan() {
        assert_eq!(teks_applescript("a \"b\" \\ c\nd"), "\"a \\\"b\\\" \\\\ c\\nd\"");
        assert_eq!(kutip_sh("it's"), "'it'\\''s'");
    }

    #[test]
    fn proses_sendiri() {
        let saya = std::process::id();
        let nama = std::env::current_exe().unwrap().file_stem().unwrap().to_string_lossy().to_string();
        assert!(proses_hidup(saya, &[nama.as_str()]));
        assert!(!proses_hidup(saya, &["bukan-ini"]));
        assert!(!bunuh(saya));
    }

    #[test]
    fn terlepas_dan_dihentikan() {
        let pid = mulai_terlepas(Path::new("/bin/sleep"), &["30".into()]).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(200));
        assert!(proses_hidup(pid, &["sleep"]));
        bunuh_pohon(pid);
        let mut mati = false;
        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            if !proses_hidup(pid, &["sleep"]) {
                mati = true;
                break;
            }
        }
        assert!(mati, "proses terlepas tidak berhenti");
    }

    #[test]
    fn disk_dan_folder() {
        let tmp = std::env::temp_dir();
        assert!(ruang_bebas(&tmp.join("belum/ada")) > 0);
        assert!(drive_tetap(&tmp));
        assert!(folder(Folder::Dokumen).is_some());
        assert!(folder_data().ends_with("Library/Application Support"));
        let (y, mo, d, h, mi, _, w) = jam_lokal();
        assert!(y >= 2024 && (1..=12).contains(&mo) && (1..=31).contains(&d) && h < 24 && mi < 60 && w < 7);
        let drive = daftar_drive();
        assert!(drive.iter().any(|(a, _)| a == "/"));
        assert!(!nama_komputer().is_empty());
    }

    #[test]
    fn tersembunyi_dan_sampah() {
        let d = std::env::temp_dir().join(format!("lelemove-uji-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("buang aku.txt");
        std::fs::write(&f, b"isi").unwrap();
        let m = std::fs::metadata(&f).unwrap();
        assert!(!tersembunyi(&m, OsStr::new("buang aku.txt")));
        assert!(tersembunyi(&m, OsStr::new(".DS_Store")));
        assert!(!hanya_cloud(&m));
        assert_eq!(ke_recycle_bin(&[f.clone()]), 1, "berkas tidak masuk Tong Sampah");
        assert!(!f.exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn kunci_satu_jendela() {
        let nama = format!("uji-{}", std::process::id());
        let k = kunci_tunggal(&nama).expect("kunci pertama harus didapat");
        assert!(k.0.is_some());
        // flock per berkas terbuka: pembukaan kedua di proses yang sama juga ditolak.
        assert!(kunci_tunggal(&nama).is_none());
        drop(k);
        assert!(kunci_tunggal(&nama).is_some());
        let _ = std::fs::remove_file(berkas_kunci(&nama));
    }
}
