//! Notifikasi Windows (toast) saat penyalinan latar selesai atau berhenti karena galat.
//!
//! Tanpa crate `windows`: API WinRT dipanggil lewat vtable COM mentah, dan fungsi combase.dll
//! dimuat saat dipakai (LoadLibraryW + GetProcAddress). Jadi tidak butuh import library tambahan
//! di toolchain GNU, dan di Windows yang tidak punya WinRT fungsi ini cukup mengembalikan Err.
//!
//! Aplikasi desktop tanpa paket MSIX harus mendaftarkan AppUserModelID-nya agar toast tampil:
//! HKCU\Software\Classes\AppUserModelId\<APP_ID> berisi DisplayName dan IconUri.

use std::ffi::c_void;
use std::path::PathBuf;
use std::ptr::null_mut;

use windows_sys::core::{GUID, HRESULT};
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

use crate::mesin::dasar::{jalur, pastikan_dir};
use crate::win::lebar;
use crate::APP_ID;

const IKON: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../aset/ikon.ico"));

// IID antarmuka WinRT (sama dengan metadata Windows.UI.Notifications & Windows.Data.Xml.Dom).
const IID_XML_DOCUMENT: GUID = GUID::from_u128(0xf7f3a506_1e87_42d6_bcfb_b8c809fa5494);
const IID_XML_DOCUMENT_IO: GUID = GUID::from_u128(0x6cd0e74e_ee65_4489_9ebf_ca43e87ba637);
const IID_TOAST_FACTORY: GUID = GUID::from_u128(0x04124b20_82c6_4229_b109_fd9ed4662b53);
const IID_TOAST_MANAGER_STATICS: GUID = GUID::from_u128(0x50ac103f_d235_4598_bbef_98fe4d1a3ad4);

/// Kunci registry pendaftaran AppUserModelID (dihapus lagi oleh uninstaller).
pub fn kunci_registry() -> String {
    format!(r"Software\Classes\AppUserModelId\{APP_ID}")
}

/// Menampilkan toast: `judul` tebal, lalu paling banyak dua baris isi.
pub fn kirim(judul: &str, baris: &[String]) -> Result<(), String> {
    // Gagal mendaftar bukan alasan untuk tidak mencoba: AUMID mungkin sudah terdaftar sebelumnya.
    let _ = daftarkan();
    let c = Combase::muat()?;
    // RO_INIT_MULTITHREADED. Hasil negatif (mis. thread sudah STA) tetap boleh lanjut.
    let hr = unsafe { (c.ro_initialize)(1) };
    let hasil = unsafe { tampilkan(&c, &xml_toast(judul, baris)) };
    if hr >= 0 {
        unsafe { (c.ro_uninitialize)() };
    }
    hasil
}

/// XML toast "ToastGeneric": baris pertama jadi judul.
pub fn xml_toast(judul: &str, baris: &[String]) -> String {
    let mut s = String::from(r#"<toast><visual><binding template="ToastGeneric">"#);
    for t in std::iter::once(judul).chain(baris.iter().map(String::as_str)).take(3) {
        s.push_str("<text>");
        s.push_str(&escape_xml(t));
        s.push_str("</text>");
    }
    s.push_str("</binding></visual></toast>");
    s
}

fn escape_xml(t: &str) -> String {
    let mut s = String::with_capacity(t.len());
    for c in t.chars() {
        match c {
            '&' => s.push_str("&amp;"),
            '<' => s.push_str("&lt;"),
            '>' => s.push_str("&gt;"),
            '"' => s.push_str("&quot;"),
            '\'' => s.push_str("&apos;"),
            // Karakter kontrol tidak sah di XML 1.0.
            c if (c as u32) < 0x20 && !matches!(c, '\t' | '\n' | '\r') => s.push(' '),
            c => s.push(c),
        }
    }
    s
}

/// Nama tampilan & ikon pengirim notifikasi (HKCU, tanpa hak admin). Aman diulang.
fn daftarkan() -> Result<(), String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;
    let (k, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(kunci_registry()).map_err(|e| e.to_string())?;
    k.set_value("DisplayName", &"LELEMOVE").map_err(|e| e.to_string())?;
    if let Some(ikon) = tulis_ikon() {
        k.set_value("IconUri", &ikon.to_string_lossy().to_string()).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Ikon notifikasi = bingkai PNG dari aset\ikon.ico, ditulis ke folder data kerja.
fn tulis_ikon() -> Option<PathBuf> {
    let png = crate::gaya::png_dari_ico(IKON)?;
    let p = jalur().data.join("ikon-notifikasi.png");
    if std::fs::metadata(&p).map(|m| m.len()).ok() != Some(png.len() as u64) {
        pastikan_dir();
        std::fs::write(&p, png).ok()?;
    }
    Some(p)
}

// ------------------------------------------------------------------ WinRT tanpa crate `windows`

type Hstring = *mut c_void;

struct Combase {
    ro_initialize: unsafe extern "system" fn(i32) -> HRESULT,
    ro_uninitialize: unsafe extern "system" fn(),
    ro_activate_instance: unsafe extern "system" fn(Hstring, *mut *mut c_void) -> HRESULT,
    ro_get_activation_factory: unsafe extern "system" fn(Hstring, *const GUID, *mut *mut c_void) -> HRESULT,
    windows_create_string: unsafe extern "system" fn(*const u16, u32, *mut Hstring) -> HRESULT,
    windows_delete_string: unsafe extern "system" fn(Hstring) -> HRESULT,
}

impl Combase {
    fn muat() -> Result<Self, String> {
        let dll = lebar("combase.dll");
        let h = unsafe { LoadLibraryW(dll.as_ptr()) };
        if h.is_null() {
            return Err("combase.dll tidak tersedia (Windows terlalu lama?)".into());
        }
        macro_rules! ambil {
            ($nama:literal) => {
                match unsafe { GetProcAddress(h, concat!($nama, "\0").as_ptr()) } {
                    // Tanda tangan fungsi diambil dari tipe field tujuan, jadi tipenya sengaja
                    // tidak ditulis di sini (berbeda untuk tiap field).
                    #[allow(clippy::missing_transmute_annotations)]
                    Some(f) => unsafe { std::mem::transmute(f) },
                    None => return Err(format!("{} tidak ada di combase.dll", $nama)),
                }
            };
        }
        Ok(Combase {
            ro_initialize: ambil!("RoInitialize"),
            ro_uninitialize: ambil!("RoUninitialize"),
            ro_activate_instance: ambil!("RoActivateInstance"),
            ro_get_activation_factory: ambil!("RoGetActivationFactory"),
            windows_create_string: ambil!("WindowsCreateString"),
            windows_delete_string: ambil!("WindowsDeleteString"),
        })
    }

    fn hstring(&self, s: &str) -> Result<HString<'_>, String> {
        let w: Vec<u16> = s.encode_utf16().collect();
        let mut h = null_mut();
        cek(unsafe { (self.windows_create_string)(w.as_ptr(), w.len() as u32, &mut h) }, "WindowsCreateString")?;
        Ok(HString { h, c: self })
    }

    /// Pabrik (activation factory) kelas WinRT dengan antarmuka `iid`.
    fn pabrik(&self, kelas: &str, iid: &GUID) -> Result<Com, String> {
        let k = self.hstring(kelas)?;
        let mut p = null_mut();
        cek(unsafe { (self.ro_get_activation_factory)(k.h, iid, &mut p) }, kelas)?;
        Ok(Com(p))
    }
}

/// HSTRING yang dilepas otomatis.
struct HString<'a> {
    h: Hstring,
    c: &'a Combase,
}

impl Drop for HString<'_> {
    fn drop(&mut self) {
        unsafe { (self.c.windows_delete_string)(self.h) };
    }
}

/// Pointer antarmuka COM/WinRT yang di-Release otomatis.
struct Com(*mut c_void);

impl Com {
    /// Entri ke-`i` vtable. Urutan IInspectable: 0 QueryInterface, 1 AddRef, 2 Release,
    /// 3-5 GetIids/GetRuntimeClassName/GetTrustLevel; metode antarmuka mulai dari 6.
    unsafe fn metode<F: Copy>(&self, i: usize) -> F {
        let vt = *(self.0 as *const *const usize);
        std::mem::transmute_copy(&*vt.add(i))
    }

    fn qi(&self, iid: &GUID) -> Result<Com, String> {
        let f: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT = unsafe { self.metode(0) };
        let mut p = null_mut();
        cek(unsafe { f(self.0, iid, &mut p) }, "QueryInterface")?;
        Ok(Com(p))
    }
}

impl Drop for Com {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                let f: unsafe extern "system" fn(*mut c_void) -> u32 = self.metode(2);
                f(self.0);
            }
        }
    }
}

fn cek(hr: HRESULT, apa: &str) -> Result<(), String> {
    if hr < 0 {
        Err(format!("{apa} gagal (0x{:08X})", hr as u32))
    } else {
        Ok(())
    }
}

/// XmlDocument.LoadXml -> new ToastNotification(xml) -> ToastNotificationManager
/// .CreateToastNotifier(APP_ID).Show(toast)
unsafe fn tampilkan(c: &Combase, xml: &str) -> Result<(), String> {
    type Fn1 = unsafe extern "system" fn(*mut c_void, *mut c_void) -> HRESULT;
    type FnH = unsafe extern "system" fn(*mut c_void, Hstring) -> HRESULT;
    type FnBuat = unsafe extern "system" fn(*mut c_void, *mut c_void, *mut *mut c_void) -> HRESULT;
    type FnBuatH = unsafe extern "system" fn(*mut c_void, Hstring, *mut *mut c_void) -> HRESULT;

    let kelas = c.hstring("Windows.Data.Xml.Dom.XmlDocument")?;
    let mut p = null_mut();
    cek((c.ro_activate_instance)(kelas.h, &mut p), "XmlDocument")?;
    let dok = Com(p);
    let io = dok.qi(&IID_XML_DOCUMENT_IO)?;
    let isi = c.hstring(xml)?;
    cek(io.metode::<FnH>(6)(io.0, isi.h), "LoadXml")?;
    let dok_xml = dok.qi(&IID_XML_DOCUMENT)?;

    let pabrik = c.pabrik("Windows.UI.Notifications.ToastNotification", &IID_TOAST_FACTORY)?;
    let mut p = null_mut();
    cek(pabrik.metode::<FnBuat>(6)(pabrik.0, dok_xml.0, &mut p), "CreateToastNotification")?;
    let toast = Com(p);

    let manajer = c.pabrik("Windows.UI.Notifications.ToastNotificationManager", &IID_TOAST_MANAGER_STATICS)?;
    let id = c.hstring(APP_ID)?;
    let mut p = null_mut();
    cek(manajer.metode::<FnBuatH>(7)(manajer.0, id.h, &mut p), "CreateToastNotifierWithId")?;
    let pengirim = Com(p);
    cek(pengirim.metode::<Fn1>(6)(pengirim.0, toast.0), "Show")
}

#[cfg(test)]
mod uji {
    use super::*;

    #[test]
    fn xml() {
        let x = xml_toast("Selesai & beres", &["D:\\Data \u{2192} <Google>".into(), "a\u{7}b".into(), "lebih".into()]);
        assert_eq!(
            x,
            "<toast><visual><binding template=\"ToastGeneric\"><text>Selesai &amp; beres</text>\
<text>D:\\Data \u{2192} &lt;Google&gt;</text><text>a b</text></binding></visual></toast>"
        );
    }

    #[test]
    fn ikon_png() {
        let png = crate::gaya::png_dari_ico(IKON).expect("ikon.ico harus memuat bingkai PNG");
        assert!(png.starts_with(b"\x89PNG"));
    }
}
