//! pindah.exe: mesin LELEMOVE di baris perintah (pengganti pindah.ps1).
//!
//!     pindah                      menu teks
//!     pindah cek
//!     pindah periksa -Sumber "D:\OneDrive\Backup" -Tujuan "pd-gdrive:Arsip"
//!     pindah salin -Latar -Ya
//!     pindah status
//!
//! Jendela aplikasi menjalankan program ini dengan -Mesin dan membaca baris "##HASIL {json}".

use std::io::IsTerminal;

use pindah_drive::mesin::{dasar, tampilkan_bantuan, urai_argumen, Mesin};
use pindah_drive::win;

fn main() {
    let o = match urai_argumen(std::env::args().skip(1)) {
        Ok(o) => o,
        Err(e) => {
            win::atur_konsol();
            eprintln!("{e}");
            eprintln!();
            tampilkan_bantuan();
            std::process::exit(1);
        }
    };
    // Warna hanya untuk manusia di konsol; jendela aplikasi membaca teks polos (UTF-8).
    let warna = win::atur_konsol() && !o.mesin && std::io::stdout().is_terminal();
    dasar::atur_warna(warna);

    let mut m = Mesin::baru(o);
    let kode = m.jalankan();
    m.cetak_hasil();
    std::process::exit(kode);
}
