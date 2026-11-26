//! Pemasang LELEMOVE (dulu Pindah Drive): per pengguna, tanpa hak admin (pengganti installer\Setup.cs).
//!
//!   LELEMOVE-Setup.exe                    jendela pemasangan
//!   LELEMOVE-Setup.exe /silent            pasang tanpa pertanyaan (pintasan Desktop dibuat)
//!   ... /silent /nodesktop                tanpa pintasan Desktop
//!   ... /silent /perbarui                 pembaruan dari dalam LELEMOVE: tunggu LELEMOVE tertutup,
//!                                         pertahankan pintasan Desktop apa adanya, lalu buka lagi
//!   uninstall.exe /uninstall [/silent]    hapus (dipanggil dari Settings > Apps)
//!
//! Isi aplikasi (termasuk rclone.exe) tertanam sebagai payload.zip (include_bytes!). Program yang
//! sama tanpa muatan adalah uninstall.exe. Dipasang ke %LOCALAPPDATA%\Programs\LELEMOVE,
//! pintasan di Start Menu (dan Desktop), serta terdaftar di HKCU ...\Uninstall.
//! Pemasangan lama bernama "Pindah Drive" (folder, pintasan, exe) dipindahkan ke nama baru.
//! Koneksi akun (%APPDATA%\rclone) dan data kerja (%LOCALAPPDATA%\pindah-drive) tidak disentuh.

use eframe::egui::{self, vec2, Align, Color32, Frame, Layout, Margin, RichText, TextEdit};
use std::io::Cursor;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use winreg::enums::{HKEY_CURRENT_USER, REG_DWORD};
use winreg::{RegKey, RegValue};

use pindah_drive::gaya::{self, tebal, tombol, tombol_utama, HIJAU, JALUR, MERAH, PUTIH, REDUP, TINTA};
use pindah_drive::{win, MUTEX_JENDELA, NAMA_APP as NAMA, VERSI};

const MUATAN: &[u8] = include_bytes!(env!("PINDAH_PAYLOAD_FILE"));
const IKON: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../aset/ikon.ico"));
/// Kunci registry tetap "PindahDrive" agar pemasangan lama dikenali sebagai versi sebelumnya.
const KUNCI_UNINSTALL: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\PindahDrive";
const EXE_APP: &str = "LELEMOVE.exe";
/// Nama sebelum LELEMOVE: pintasan, folder bawaan, dan exe-nya dibersihkan saat memperbarui.
const NAMA_LAMA: &str = "Pindah Drive";
const EXE_LAMA: &str = "Pindah Drive.exe";
/// Berkas versi PowerShell (1.x) yang tidak dipakai lagi sejak versi 2.
const BERKAS_LAMA: &[&str] = &["gui.ps1", "pindah.ps1", "tampilan.xaml"];
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn folder_programs() -> PathBuf {
    PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_default()).join("Programs")
}

fn folder_bawaan() -> PathBuf {
    folder_programs().join(NAMA)
}

/// Folder pemasangan yang diusulkan: lokasi lama bila pengguna dulu memilih folder sendiri,
/// tetapi folder bawaan lama ("Programs\Pindah Drive") pindah ke folder bawaan baru.
fn folder_usulan() -> PathBuf {
    match folder_terpasang().map(PathBuf::from) {
        Some(t) if !sama(&t, &folder_programs().join(NAMA_LAMA)) => t,
        _ => folder_bawaan(),
    }
}

fn pintasan_bernama(nama: &str) -> Vec<(bool, PathBuf)> {
    let mut p = Vec::new();
    if let Some(d) = win::folder(win::Folder::Program) {
        p.push((false, d.join(format!("{nama}.lnk"))));
    }
    if let Some(d) = win::folder(win::Folder::Desktop) {
        p.push((true, d.join(format!("{nama}.lnk"))));
    }
    p
}

fn pintasan() -> Vec<(bool, PathBuf)> {
    pintasan_bernama(NAMA)
}

fn kunci_uninstall() -> Option<RegKey> {
    RegKey::predef(HKEY_CURRENT_USER).open_subkey(KUNCI_UNINSTALL).ok()
}

fn folder_terpasang() -> Option<String> {
    kunci_uninstall()?.get_value("InstallLocation").ok()
}

fn versi_terpasang() -> Option<String> {
    kunci_uninstall()?.get_value("DisplayVersion").ok()
}

fn aplikasi_terbuka() -> bool {
    win::mutex_ada(MUTEX_JENDELA)
}

/// Penyalinan latar belakang milik folder ini sedang berjalan?
fn salin_berjalan(dir: &Path) -> bool {
    win::proses_di_folder(dir, &["rclone.exe", "pindah.exe"])
}

fn ukuran_folder(dir: &Path) -> u64 {
    let mut n = 0;
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        match e.file_type() {
            Ok(t) if t.is_dir() => n += ukuran_folder(&e.path()),
            Ok(_) => n += e.metadata().map(|m| m.len()).unwrap_or(0),
            Err(_) => {}
        }
    }
    n
}

fn mutlak(p: &Path) -> PathBuf {
    std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf())
}

fn sama(a: &Path, b: &Path) -> bool {
    let n = |p: &Path| mutlak(p).to_string_lossy().trim_end_matches('\\').to_lowercase();
    n(a) == n(b)
}

// ------------------------------------------------------------------ pasang

fn pasang(dir: &Path, desktop: bool, lapor: &mut dyn FnMut(u32, &str)) -> Result<(), String> {
    let dir = mutlak(dir);
    // Pemasangan sebelumnya (dibaca sebelum registry ditimpa); bisa di folder lain bila namanya berubah.
    let lama = folder_terpasang().map(|t| mutlak(Path::new(&t))).filter(|t| !sama(t, &dir));
    if aplikasi_terbuka() {
        return Err(format!("Tutup jendela {NAMA} dulu, lalu ulangi."));
    }
    if salin_berjalan(&dir) || lama.as_deref().is_some_and(salin_berjalan) {
        return Err(format!("Penyalinan sedang berjalan. Hentikan dulu dari aplikasi {NAMA}, lalu ulangi."));
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("Folder tidak bisa dibuat: {e}"))?;
    for f in BERKAS_LAMA.iter().chain([&EXE_LAMA]) {
        let _ = std::fs::remove_file(dir.join(f));
    }

    let mut zip = zip::ZipArchive::new(Cursor::new(MUATAN)).map_err(|e| format!("Muatan installer rusak: {e}"))?;
    let total = zip.len().max(1);
    for i in 0..zip.len() {
        let mut e = zip.by_index(i).map_err(|e| format!("Muatan installer rusak: {e}"))?;
        let Some(rel) = e.enclosed_name() else { continue }; // jalur aneh di zip
        let target = dir.join(rel);
        if e.is_dir() {
            let _ = std::fs::create_dir_all(&target);
            continue;
        }
        if let Some(induk) = target.parent() {
            let _ = std::fs::create_dir_all(induk);
        }
        lapor(((i + 1) * 90 / total) as u32, &format!("Menyalin {}", e.name()));
        let mut keluar = std::fs::File::create(&target).map_err(|er| format!("{}: {er}", target.display()))?;
        std::io::copy(&mut e, &mut keluar).map_err(|er| format!("{}: {er}", target.display()))?;
    }

    let exe = dir.join(EXE_APP);
    let uninst = dir.join("uninstall.exe");
    // uninstall.exe biasanya ikut di muatan; bila tidak, pakai salinan installer ini.
    if !uninst.exists() {
        if let Ok(saya) = std::env::current_exe() {
            if !sama(&saya, &uninst) {
                let _ = std::fs::copy(saya, &uninst);
            }
        }
    }

    lapor(93, "Membuat pintasan");
    for (di_desktop, lnk) in pintasan() {
        if !di_desktop || desktop {
            buat_pintasan(&lnk, &exe, &dir)?;
        }
    }

    lapor(97, "Mendaftarkan di Settings > Apps");
    let (k, _) = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey(KUNCI_UNINSTALL)
        .map_err(|e| format!("Registry: {e}"))?;
    let teks = |n: &str, v: &str| k.set_value(n, &v.to_string());
    let dword = |n: &str, v: u32| k.set_raw_value(n, &RegValue { vtype: REG_DWORD, bytes: v.to_le_bytes().to_vec() });
    let hasil = [
        teks("DisplayName", NAMA),
        teks("DisplayVersion", VERSI),
        teks("Publisher", NAMA),
        teks("InstallLocation", &dir.to_string_lossy()),
        teks("DisplayIcon", &exe.to_string_lossy()),
        teks("UninstallString", &format!("\"{}\" /uninstall", uninst.display())),
        teks("QuietUninstallString", &format!("\"{}\" /uninstall /silent", uninst.display())),
        dword("NoModify", 1),
        dword("NoRepair", 1),
        dword("EstimatedSize", (ukuran_folder(&dir) / 1024) as u32),
    ];
    if let Some(Err(e)) = hasil.into_iter().find(|h| h.is_err()) {
        return Err(format!("Registry: {e}"));
    }
    lapor(99, "Membersihkan pemasangan lama");
    bersihkan_lama(lama.as_deref());
    lapor(100, "Selesai");
    Ok(())
}

/// Membuang sisa nama lama ("Pindah Drive"): pintasannya, dan folder pemasangan sebelumnya bila
/// pemasangan pindah ke folder lain. Folder itu berasal dari registry pemasangan ini dan hanya
/// dihapus bila jelas berisi aplikasi ini (sama seperti pengaman uninstall).
fn bersihkan_lama(lama: Option<&Path>) {
    for (_, lnk) in pintasan_bernama(NAMA_LAMA) {
        let _ = std::fs::remove_file(lnk);
    }
    let Some(d) = lama else { return };
    let ada_app = d.join(EXE_LAMA).is_file() || d.join(EXE_APP).is_file();
    if d.to_string_lossy().trim_end_matches('\\').len() < 10 || !ada_app || !d.join("pindah.exe").is_file() {
        return;
    }
    for _ in 0..5 {
        if std::fs::remove_dir_all(d).is_ok() || !d.exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(800));
    }
}

fn buat_pintasan(lnk: &Path, target: &Path, kerja: &Path) -> Result<(), String> {
    if let Some(d) = lnk.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let mut sl = mslnk::ShellLink::new(target).map_err(|e| format!("Pintasan: {e:?}"))?;
    sl.set_working_dir(Some(kerja.to_string_lossy().to_string()));
    sl.set_icon_location(Some(target.to_string_lossy().to_string()));
    sl.set_name(Some("Salin file antar OneDrive, SharePoint, Google Drive, dan komputer".into()));
    sl.create_lnk(lnk).map_err(|e| format!("Pintasan {}: {e:?}", lnk.display()))
}

// ------------------------------------------------------------------ hapus

/// Hanya folder yang memang hasil pemasangan ini yang boleh dihapus.
fn folder_aman(dir: &Path) -> bool {
    let d = mutlak(dir);
    if d.to_string_lossy().trim_end_matches('\\').len() < 10 {
        return false;
    }
    let ada_mesin = d.join("pindah.exe").is_file() || d.join("pindah.ps1").is_file();
    if !(d.join(EXE_APP).is_file() || d.join(EXE_LAMA).is_file()) || !ada_mesin {
        return false;
    }
    if let Some(t) = folder_terpasang() {
        return sama(Path::new(&t), &d);
    }
    let programs = folder_programs();
    d.to_string_lossy().to_lowercase().starts_with(&format!("{}\\", programs.to_string_lossy().to_lowercase()))
}

fn copot(dir: Option<PathBuf>, diam: bool) -> i32 {
    let saya = std::env::current_exe().unwrap_or_default();
    let dir = mutlak(&dir.unwrap_or_else(|| saya.parent().map(Path::to_path_buf).unwrap_or_default()));

    // uninstall.exe berada di dalam folder yang akan dihapus: jalankan ulang dari salinan di TEMP.
    let di_dalam = saya.to_string_lossy().to_lowercase().starts_with(&format!(
        "{}\\",
        dir.to_string_lossy().trim_end_matches('\\').to_lowercase()
    ));
    if di_dalam {
        let salinan = std::env::temp_dir().join(format!("LELEMOVE-uninstall-{}.exe", std::process::id()));
        if std::fs::copy(&saya, &salinan).is_ok() {
            let mut a = vec!["/uninstall".to_string(), "/dir".to_string(), dir.to_string_lossy().to_string()];
            if diam {
                a.push("/silent".into());
            }
            let _ = Command::new(&salinan).args(a).spawn();
            return 0;
        }
    }

    if !folder_aman(&dir) {
        if !diam {
            win::kotak_pesan(
                NAMA,
                &format!("Folder ini bukan pemasangan {NAMA}, jadi tidak dihapus:\n{}", dir.display()),
                win::MB_OK | win::MB_ICONWARNING,
            );
        }
        return 3;
    }
    if !diam
        && win::kotak_pesan(
            NAMA,
            &format!("Hapus {NAMA} dari komputer ini?\n\nKoneksi akun, pengaturan, dan log tidak ikut dihapus."),
            win::MB_YESNO | win::MB_ICONQUESTION,
        ) != win::IDYES
    {
        return 1;
    }
    while aplikasi_terbuka() {
        if diam {
            return 2;
        }
        if win::kotak_pesan(NAMA, &format!("Tutup jendela {NAMA} dulu, lalu klik Retry."), win::MB_RETRYCANCEL | win::MB_ICONINFORMATION)
            != win::IDRETRY
        {
            return 1;
        }
    }
    if salin_berjalan(&dir) {
        if !diam {
            win::kotak_pesan(
                NAMA,
                &format!("Penyalinan sedang berjalan. Hentikan dulu dari aplikasi {NAMA}, lalu ulangi."),
                win::MB_OK | win::MB_ICONWARNING,
            );
        }
        return 2;
    }

    for (_, lnk) in pintasan().into_iter().chain(pintasan_bernama(NAMA_LAMA)) {
        let _ = std::fs::remove_file(lnk);
    }
    // Jadwal backup: tugas Windows yang menjalankan program di folder ini akan gagal setelah
    // folder dihapus, jadi ikut dihapus. Daftar jadwal (jadwal.json) tetap ada.
    let tugas = pindah_drive::mesin::jadwal::hapus_tugas_di(&dir);
    let _ = RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(KUNCI_UNINSTALL);
    // Pendaftaran pengirim notifikasi (dibuat pindah.exe saat penyalinan latar berakhir).
    let _ = RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(pindah_drive::notifikasi::kunci_registry());
    for _ in 0..5 {
        if std::fs::remove_dir_all(&dir).is_ok() || !dir.exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(800));
    }

    if !diam {
        let lad = PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_default()).join("pindah-drive");
        let conf = PathBuf::from(std::env::var("APPDATA").unwrap_or_default()).join("rclone\\rclone.conf");
        let jadwal = if tugas > 0 {
            format!("\n\n{tugas} jadwal backup dihentikan (tugas Windows-nya dihapus). Bila {NAMA} dipasang lagi, aktifkan lewat Jadwal backup.")
        } else {
            String::new()
        };
        win::kotak_pesan(
            NAMA,
            &format!(
                "{NAMA} sudah dihapus.{jadwal}\n\nYang masih tersisa (boleh dihapus sendiri):\n- {}  (pengaturan, jadwal & log)\n- {}  (koneksi akun)",
                lad.display(),
                conf.display()
            ),
            win::MB_OK | win::MB_ICONINFORMATION,
        );
    }

    // Salinan di TEMP menghapus dirinya sendiri setelah proses ini selesai.
    let tmp = std::env::temp_dir().to_string_lossy().to_lowercase();
    if saya.to_string_lossy().to_lowercase().starts_with(tmp.trim_end_matches('\\')) {
        let _ = Command::new("cmd.exe")
            .raw_arg(format!("/c ping 127.0.0.1 -n 3 >nul & del /f /q \"{}\"", saya.display()))
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
    }
    0
}

// ------------------------------------------------------------------ jendela pemasangan

enum Lapor {
    Progres(u32, String),
    Selesai(Result<(), String>),
}

struct Jendela {
    folder: String,
    desktop: bool,
    buka: bool,
    versi_lama: Option<String>,
    status: (String, Color32),
    persen: Option<u32>,
    jalan: bool,
    selesai: bool,
    rx: Option<Receiver<Lapor>>,
}

impl Jendela {
    fn klik_pasang(&mut self, ctx: &egui::Context) {
        if self.selesai {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        let dir = self.folder.trim().to_string();
        if dir.is_empty() {
            return;
        }
        while aplikasi_terbuka() {
            if win::kotak_pesan(NAMA, &format!("Tutup jendela {NAMA} dulu, lalu klik Retry."), win::MB_RETRYCANCEL | win::MB_ICONINFORMATION)
                != win::IDRETRY
            {
                return;
            }
        }
        let (tx, rx) = channel();
        self.rx = Some(rx);
        self.jalan = true;
        self.persen = Some(0);
        self.status = ("Menyiapkan\u{2026}".into(), REDUP);
        let (desktop, ctx) = (self.desktop, ctx.clone());
        std::thread::spawn(move || {
            let mut lapor = |p: u32, s: &str| {
                let _ = tx.send(Lapor::Progres(p, s.to_string()));
                ctx.request_repaint();
            };
            let h = pasang(Path::new(&dir), desktop, &mut lapor);
            let _ = tx.send(Lapor::Selesai(h));
            ctx.request_repaint();
        });
    }

    fn terima(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.rx else { return };
        let mut habis = false;
        while let Ok(l) = rx.try_recv() {
            match l {
                Lapor::Progres(p, s) => {
                    self.persen = Some(p.min(100));
                    self.status = (s, REDUP);
                }
                Lapor::Selesai(Ok(())) => {
                    habis = true;
                    self.selesai = true;
                    let tempat = if self.desktop { "Start Menu dan Desktop" } else { "Start Menu" };
                    self.status = (format!("Selesai. {NAMA} ada di {tempat}."), HIJAU);
                    if self.buka {
                        let dir = mutlak(Path::new(self.folder.trim()));
                        let _ = Command::new(dir.join(EXE_APP)).current_dir(&dir).spawn();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
                Lapor::Selesai(Err(e)) => {
                    habis = true;
                    self.persen = None;
                    self.status = (format!("Gagal: {e}"), MERAH);
                }
            }
        }
        if habis {
            self.jalan = false;
            self.rx = None;
        }
    }
}

impl eframe::App for Jendela {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.terima(ctx);
        let mut pasang_diklik = false;
        egui::CentralPanel::default()
            .frame(Frame::new().fill(PUTIH).inner_margin(Margin { left: 26, right: 26, top: 20, bottom: 18 }))
            .show(ctx, |ui| {
                ui.label(RichText::new(NAMA).family(tebal()).size(21.0).color(TINTA));
                ui.add_space(4.0);
                let mut ket = format!(
                    "Salin file antar OneDrive, SharePoint, Google Drive, dan komputer ini.\nVersi {VERSI}, dipasang untuk akun Windows ini saja (tanpa hak admin)."
                );
                if let Some(v) = &self.versi_lama {
                    ket += &format!("\nVersi {v} sudah terpasang dan akan diperbarui.");
                }
                ui.label(RichText::new(ket).color(REDUP));
                ui.add_space(14.0);
                ui.label(RichText::new("Folder pemasangan").color(REDUP));
                let boleh = !self.jalan && !self.selesai;
                ui.horizontal(|ui| {
                    let w = ui.available_width() - 92.0;
                    ui.add_enabled(
                        boleh,
                        TextEdit::singleline(&mut self.folder)
                            .desired_width(w)
                            .min_size(vec2(w, 30.0))
                            .vertical_align(Align::Center)
                            .margin(Margin::symmetric(8, 4)),
                    );
                    if ui.add_enabled(boleh, tombol("Ubah\u{2026}").min_size(vec2(84.0, 32.0))).clicked() {
                        if let Some(p) = rfd::FileDialog::new()
                            .set_title(format!("Pilih folder induk; {NAMA} dipasang di subfolder \"{NAMA}\"."))
                            .pick_folder()
                        {
                            self.folder = p.join(NAMA).to_string_lossy().to_string();
                        }
                    }
                });
                ui.add_space(8.0);
                ui.add_enabled(boleh, egui::Checkbox::new(&mut self.desktop, "Buat pintasan di Desktop"));
                ui.add_enabled(boleh, egui::Checkbox::new(&mut self.buka, format!("Buka {NAMA} setelah selesai")));
                ui.add_space(10.0);
                if let Some(p) = self.persen {
                    ui.scope(|ui| {
                        ui.visuals_mut().extreme_bg_color = JALUR;
                        ui.add(egui::ProgressBar::new(p as f32 / 100.0).desired_height(8.0).fill(HIJAU));
                    });
                }
                ui.add(egui::Label::new(RichText::new(&self.status.0).color(self.status.1)).truncate());
                ui.with_layout(Layout::bottom_up(Align::Max), |ui| {
                    ui.horizontal(|ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let label = if self.selesai {
                                "Tutup"
                            } else if self.versi_lama.is_some() {
                                "Perbarui"
                            } else {
                                "Pasang"
                            };
                            if ui.add_enabled(!self.jalan, tombol_utama(label).min_size(vec2(96.0, 36.0))).clicked() {
                                pasang_diklik = true;
                            }
                            if !self.selesai && ui.add_enabled(!self.jalan, tombol("Batal").min_size(vec2(84.0, 36.0))).clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        });
                    });
                });
            });
        if pasang_diklik {
            self.klik_pasang(ctx);
        }
        // Jangan tutup di tengah pemasangan.
        if self.jalan && ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
    }
}

/// Pembaruan yang dimulai dari dalam LELEMOVE (`pindah pembaruan -Jalankan`): tunggu jendela
/// LELEMOVE dan pindah.exe pengunduhnya selesai (paling lama 2 menit), pasang dengan pintasan
/// Desktop seperti sebelumnya, lalu buka LELEMOVE lagi. Pengguna tidak sedang melihat apa-apa, jadi
/// kegagalan ditampilkan sebagai kotak pesan.
fn perbarui_dari_aplikasi(dir: PathBuf) -> i32 {
    use std::time::{Duration, Instant};
    let batas = Instant::now() + Duration::from_secs(120);
    while (aplikasi_terbuka() || salin_berjalan(&dir)) && Instant::now() < batas {
        std::thread::sleep(Duration::from_millis(500));
    }
    let desktop = pintasan().iter().any(|(di_desktop, p)| *di_desktop && p.exists());
    match pasang(&dir, desktop, &mut |_, _| {}) {
        Ok(()) => {
            let dir = mutlak(&dir);
            let _ = Command::new(dir.join(EXE_APP)).current_dir(&dir).spawn();
            0
        }
        Err(e) => {
            win::kotak_pesan(NAMA, &format!("Pembaruan {NAMA} ke versi {VERSI} gagal:\n\n{e}"), win::MB_OK | win::MB_ICONWARNING);
            1
        }
    }
}

pub fn main() {
    let mut copot_diminta = false;
    let mut diam = false;
    let mut tanpa_desktop = false;
    let mut perbarui = false;
    let mut dir: Option<PathBuf> = None;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].to_lowercase().as_str() {
            "/uninstall" => copot_diminta = true,
            "/silent" | "/s" => diam = true,
            "/nodesktop" => tanpa_desktop = true,
            "/perbarui" => {
                perbarui = true;
                diam = true;
            }
            "/dir" if i + 1 < args.len() => {
                i += 1;
                dir = Some(PathBuf::from(&args[i]));
            }
            _ => {}
        }
        i += 1;
    }
    // uninstall.exe adalah build yang sama tanpa muatan: dibuka langsung pun berarti "hapus".
    if copot_diminta || MUATAN.is_empty() {
        std::process::exit(copot(dir, diam));
    }
    if perbarui {
        std::process::exit(perbarui_dari_aplikasi(dir.unwrap_or_else(folder_usulan)));
    }
    if diam {
        let h = pasang(&dir.unwrap_or_else(folder_usulan), !tanpa_desktop, &mut |_, _| {});
        std::process::exit(if h.is_ok() { 0 } else { 1 });
    }

    let versi_lama = versi_terpasang();
    let folder = folder_usulan().to_string_lossy().to_string();
    let mut vp = egui::ViewportBuilder::default()
        .with_title(format!("Pasang {NAMA}"))
        .with_inner_size([560.0, 360.0])
        .with_resizable(false)
        .with_maximize_button(false);
    if let Some(i) = gaya::ikon_dari_ico(IKON) {
        vp = vp.with_icon(Arc::new(i));
    }
    let opsi = eframe::NativeOptions { viewport: vp, centered: true, ..Default::default() };
    let _ = eframe::run_native(
        "LELEMOVE Setup",
        opsi,
        Box::new(move |cc| {
            gaya::pasang(&cc.egui_ctx);
            Ok(Box::new(Jendela {
                folder,
                desktop: true,
                buka: true,
                versi_lama,
                status: (String::new(), REDUP),
                persen: None,
                jalan: false,
                selesai: false,
                rx: None,
            }))
        }),
    );
}
