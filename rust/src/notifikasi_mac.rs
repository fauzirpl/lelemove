//! Notifikasi macOS saat penyalinan latar selesai atau berhenti karena galat.
//!
//! Lewat `osascript -e 'display notification ...'`: tidak butuh izin tambahan, dan di Mac yang
//! membatasi notifikasi cukup tidak tampil. Padanan notifikasi.rs (toast Windows).

use std::process::{Command, Stdio};

use crate::win::teks_applescript;

/// Menampilkan notifikasi: judul aplikasi, `judul` sebagai subjudul, lalu paling banyak dua baris isi.
pub fn kirim(judul: &str, baris: &[String]) -> Result<(), String> {
    let o = Command::new("/usr/bin/osascript")
        .arg("-e")
        .arg(skrip(judul, baris))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .output()
        .map_err(|e| format!("osascript tidak bisa dijalankan: {e}"))?;
    if o.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&o.stderr).trim().to_string())
    }
}

pub fn skrip(judul: &str, baris: &[String]) -> String {
    let isi: Vec<&str> = baris.iter().map(String::as_str).take(2).collect();
    format!(
        "display notification {} with title {} subtitle {}",
        teks_applescript(&isi.join("\n")),
        teks_applescript(crate::NAMA_APP),
        teks_applescript(judul)
    )
}

#[cfg(test)]
mod uji {
    #[test]
    fn skrip() {
        let s = super::skrip("Selesai \"rapi\"", &["satu".into(), "dua".into(), "tiga".into()]);
        assert_eq!(s, "display notification \"satu\\ndua\" with title \"LELEMOVE\" subtitle \"Selesai \\\"rapi\\\"\"");
    }
}
