//! Pembungkus kecil Win32 lewat windows-sys. Semua fungsi aman dipanggil dan tidak panik:
//! kegagalan dilaporkan sebagai false/None.

use std::ffi::{c_void, OsStr};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::process::CommandExt;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, INVALID_HANDLE_VALUE, STILL_ACTIVE,
};
use windows_sys::Win32::System::Threading::{
    CreateMutexW, CreateProcessW, GetExitCodeProcess, OpenMutexW, OpenProcess,
    QueryFullProcessImageNameW, TerminateProcess, CREATE_BREAKAWAY_FROM_JOB,
    CREATE_NEW_PROCESS_GROUP, CREATE_NO_WINDOW, CREATE_UNICODE_ENVIRONMENT,
    PROCESS_INFORMATION, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_TERMINATE, STARTUPINFOW, SYNCHRONIZATION_SYNCHRONIZE,
};

pub use windows_sys::Win32::UI::WindowsAndMessaging::{
    IDRETRY, IDYES, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONQUESTION, MB_ICONWARNING, MB_OK,
    MB_RETRYCANCEL, MB_YESNO,
};

/// String UTF-16 berakhiran nol untuk API Win32.
pub fn lebar(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

fn lebar_os(s: &OsStr) -> Vec<u16> {
    s.encode_wide().chain(Some(0)).collect()
}

// ------------------------------------------------------------------ konsol

/// Proses ini tidak punya jendela konsol (mis. jendela aplikasi). Proses anak yang berupa
/// program konsol (rclone, pindah.exe) lalu perlu CREATE_NO_WINDOW agar tidak memunculkan jendela.
pub fn tanpa_konsol() -> bool {
    unsafe { windows_sys::Win32::System::Console::GetConsoleWindow().is_null() }
}

pub fn bendera_anak() -> u32 {
    if tanpa_konsol() {
        CREATE_NO_WINDOW
    } else {
        0
    }
}

/// Konsol memakai UTF-8 dan (bila bisa) warna ANSI. Hasil: warna aktif.
pub fn atur_konsol() -> bool {
    use windows_sys::Win32::System::Console::{
        GetConsoleMode, GetStdHandle, SetConsoleCP, SetConsoleMode, SetConsoleOutputCP,
        ENABLE_VIRTUAL_TERMINAL_PROCESSING, STD_OUTPUT_HANDLE,
    };
    unsafe {
        SetConsoleOutputCP(65001);
        SetConsoleCP(65001);
        let h = GetStdHandle(STD_OUTPUT_HANDLE);
        let mut mode = 0u32;
        if GetConsoleMode(h, &mut mode) == 0 {
            return false;
        }
        SetConsoleMode(h, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING) != 0
    }
}

// ------------------------------------------------------------------ proses

/// Jalur .exe sebuah proses yang masih hidup.
pub fn jalur_proses(pid: u32) -> Option<PathBuf> {
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() {
            return None;
        }
        let mut kode = 0u32;
        let hidup = GetExitCodeProcess(h, &mut kode) != 0 && kode == STILL_ACTIVE as u32;
        let mut buf = [0u16; 1024];
        let mut n = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut n) != 0;
        CloseHandle(h);
        if !hidup || !ok {
            return None;
        }
        Some(PathBuf::from(String::from_utf16_lossy(&buf[..n as usize])))
    }
}

/// Proses `pid` masih hidup dan namanya (tanpa .exe) salah satu dari `nama`.
pub fn proses_hidup(pid: u32, nama: &[&str]) -> bool {
    match jalur_proses(pid).and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string())) {
        Some(n) => nama.iter().any(|x| n.eq_ignore_ascii_case(x)),
        None => false,
    }
}

pub fn bunuh(pid: u32) -> bool {
    unsafe {
        let h = OpenProcess(PROCESS_TERMINATE, 0, pid);
        if h.is_null() {
            return false;
        }
        let ok = TerminateProcess(h, 1) != 0;
        CloseHandle(h);
        ok
    }
}

/// Menghentikan proses beserta semua anaknya (mis. pindah.exe yang sedang menunggu rclone).
pub fn bunuh_pohon(pid: u32) {
    let _ = Command::new("taskkill.exe")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .status();
}

/// Ada proses bernama salah satu `nama` (mis. "rclone.exe") yang .exe-nya berada di `dir`.
pub fn proses_di_folder(dir: &Path, nama: &[&str]) -> bool {
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    let akar = format!("{}\\", dir.to_string_lossy().trim_end_matches('\\').to_lowercase());
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE || snap.is_null() {
            return false;
        }
        let mut e: PROCESSENTRY32W = std::mem::zeroed();
        e.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut ada = false;
        if Process32FirstW(snap, &mut e) != 0 {
            loop {
                let pjg = e.szExeFile.iter().position(|&c| c == 0).unwrap_or(e.szExeFile.len());
                let n = String::from_utf16_lossy(&e.szExeFile[..pjg]);
                if nama.iter().any(|x| n.eq_ignore_ascii_case(x)) {
                    if let Some(p) = jalur_proses(e.th32ProcessID) {
                        if p.to_string_lossy().to_lowercase().starts_with(&akar) {
                            ada = true;
                            break;
                        }
                    }
                }
                if Process32NextW(snap, &mut e) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snap);
        ada
    }
}

/// Aturan kutip baris perintah Windows (CommandLineToArgvW).
pub fn kutip(a: &str) -> String {
    if a.is_empty() {
        return "\"\"".into();
    }
    if !a.chars().any(|c| c.is_whitespace() || c == '"') {
        return a.into();
    }
    let mut s = String::from("\"");
    let mut garis_miring = 0usize;
    for c in a.chars() {
        match c {
            '\\' => garis_miring += 1,
            '"' => {
                s.push_str(&"\\".repeat(garis_miring * 2 + 1));
                s.push('"');
                garis_miring = 0;
            }
            _ => {
                s.push_str(&"\\".repeat(garis_miring));
                garis_miring = 0;
                s.push(c);
            }
        }
    }
    s.push_str(&"\\".repeat(garis_miring * 2));
    s.push('"');
    s
}

/// Menjalankan program yang terlepas dari proses ini: tanpa jendela, tanpa mewarisi handle
/// (pipa keluaran jendela aplikasi tidak ikut terbawa), dan tetap jalan bila induknya ditutup.
pub fn mulai_terlepas(exe: &Path, args: &[String]) -> std::io::Result<u32> {
    let mut baris = format!("{}", kutip(&exe.to_string_lossy()));
    for a in args {
        baris.push(' ');
        baris.push_str(&kutip(a));
    }
    let exe_w = lebar_os(exe.as_os_str());
    let dasar = CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP | CREATE_UNICODE_ENVIRONMENT;
    // Coba lepas dari job object induk (mis. terminal yang menutup semua anaknya); tidak semua
    // job mengizinkannya, jadi ulangi tanpa bendera itu bila ditolak.
    for bendera in [dasar | CREATE_BREAKAWAY_FROM_JOB, dasar] {
        let mut baris_w = lebar(&baris);
        let mut si: STARTUPINFOW = unsafe { std::mem::zeroed() };
        si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
        let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
        let ok = unsafe {
            CreateProcessW(
                exe_w.as_ptr(),
                baris_w.as_mut_ptr(),
                null(),
                null(),
                0,
                bendera,
                null(),
                null(),
                &si,
                &mut pi,
            )
        };
        if ok != 0 {
            unsafe {
                CloseHandle(pi.hThread);
                CloseHandle(pi.hProcess);
            }
            return Ok(pi.dwProcessId);
        }
    }
    Err(std::io::Error::last_os_error())
}

// ------------------------------------------------------------------ sistem

/// Mencegah Windows tidur otomatis selama penyalinan (dilepas otomatis saat proses berakhir).
pub fn jaga_bangun(aktif: bool) {
    use windows_sys::Win32::System::Power::{
        SetThreadExecutionState, ES_CONTINUOUS, ES_SYSTEM_REQUIRED,
    };
    unsafe {
        SetThreadExecutionState(if aktif { ES_CONTINUOUS | ES_SYSTEM_REQUIRED } else { ES_CONTINUOUS });
    }
}

fn akar_path(p: &Path) -> String {
    let mut s = PathBuf::new();
    for c in p.components() {
        match c {
            Component::Prefix(_) | Component::RootDir => s.push(c.as_os_str()),
            _ => break,
        }
    }
    let mut t = s.to_string_lossy().to_string();
    if !t.ends_with('\\') {
        t.push('\\');
    }
    t
}

/// Sisa ruang disk tempat `path` berada (folder boleh belum ada).
pub fn ruang_bebas(path: &Path) -> u64 {
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let w = lebar(&akar_path(path));
    let mut bebas = 0u64;
    unsafe {
        if GetDiskFreeSpaceExW(w.as_ptr(), &mut bebas, null_mut(), null_mut()) == 0 {
            return 0;
        }
    }
    bebas
}

/// Membuka berkas, folder, atau alamat web dengan program bawaan Windows.
pub fn buka(target: &str) {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let op = lebar("open");
    let t = lebar(target);
    unsafe {
        ShellExecuteW(null_mut(), op.as_ptr(), t.as_ptr(), null(), null(), SW_SHOWNORMAL);
    }
}

pub fn kotak_pesan(judul: &str, isi: &str, gaya: u32) -> i32 {
    use windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW;
    let j = lebar(judul);
    let i = lebar(isi);
    unsafe { MessageBoxW(null_mut(), i.as_ptr(), j.as_ptr(), gaya) }
}

/// Mutex satu-jendela. Selama nilai ini hidup, `mutex_ada` di proses lain bernilai true.
pub struct KunciTunggal(HANDLE);

impl Drop for KunciTunggal {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

/// None bila mutex sudah dipegang proses lain (aplikasi sudah terbuka).
pub fn kunci_tunggal(nama: &str) -> Option<KunciTunggal> {
    let w = lebar(nama);
    unsafe {
        let h = CreateMutexW(null(), 1, w.as_ptr());
        if h.is_null() {
            return None;
        }
        if GetLastError() == ERROR_ALREADY_EXISTS {
            CloseHandle(h);
            return None;
        }
        Some(KunciTunggal(h))
    }
}

pub fn mutex_ada(nama: &str) -> bool {
    let w = lebar(nama);
    unsafe {
        let h = OpenMutexW(SYNCHRONIZATION_SYNCHRONIZE, 0, w.as_ptr());
        if h.is_null() {
            return false;
        }
        CloseHandle(h);
        true
    }
}

/// Ikon taskbar milik aplikasi sendiri, terpisah dari program lain.
pub fn set_app_id(id: &str) {
    use windows_sys::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;
    let w = lebar(id);
    unsafe {
        SetCurrentProcessExplicitAppUserModelID(w.as_ptr());
    }
}

pub enum Folder {
    Dokumen,
    /// Start Menu\Programs milik pengguna.
    Program,
    Desktop,
}

/// Folder dikenal Windows (mengikuti pengalihan, mis. Dokumen yang dipindah ke OneDrive).
pub fn folder(f: Folder) -> Option<PathBuf> {
    use windows_sys::Win32::System::Com::CoTaskMemFree;
    use windows_sys::Win32::UI::Shell::{
        SHGetKnownFolderPath, FOLDERID_Desktop, FOLDERID_Documents, FOLDERID_Programs,
    };
    let id = match f {
        Folder::Dokumen => &FOLDERID_Documents,
        Folder::Program => &FOLDERID_Programs,
        Folder::Desktop => &FOLDERID_Desktop,
    };
    let mut p: *mut u16 = null_mut();
    unsafe {
        let hr = SHGetKnownFolderPath(id, 0, null_mut(), &mut p);
        if hr < 0 || p.is_null() {
            return None;
        }
        let mut n = 0usize;
        while *p.add(n) != 0 {
            n += 1;
        }
        let s = String::from_utf16_lossy(std::slice::from_raw_parts(p, n));
        CoTaskMemFree(p as *const c_void);
        Some(PathBuf::from(s))
    }
}

/// Tinggi area kerja layar utama dalam satuan logis (piksel 96 DPI), tanpa taskbar.
pub fn tinggi_area_kerja() -> Option<f32> {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::HiDpi::GetDpiForSystem;
    use windows_sys::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETWORKAREA};
    let mut r: RECT = unsafe { std::mem::zeroed() };
    unsafe {
        if SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut r as *mut RECT as *mut c_void, 0) == 0 {
            return None;
        }
        let dpi = GetDpiForSystem().max(96) as f32;
        Some((r.bottom - r.top) as f32 * 96.0 / dpi)
    }
}
