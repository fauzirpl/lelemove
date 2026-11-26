use serde_json::Value;

use super::super::dasar::*;
use super::super::riwayat;
use super::{Mesin, nama_tempat};

impl Mesin {
    // -------------------------------------------------------------- riwayat

    /// Daftar riwayat; -Csv / -Laporan mengekspor. Dengan -Id: satu catatan (laporan per file).
    pub(super) fn riwayat(&mut self) -> i32 {
        let semua = riwayat::baca(0);
        let tulis = |p: &str, isi: &str, apa: &str| -> i32 {
            match std::fs::write(p, isi) {
                Ok(()) => {
                    ok(&format!("{apa} disimpan: {p}"));
                    0
                }
                Err(e) => {
                    gagal(&format!("{apa} tidak bisa disimpan ke {p}: {e}"));
                    1
                }
            }
        };
        if let Some(id) = self.o.id.clone() {
            let Some(c) = semua.iter().find(|c| c.id == id) else {
                gagal(&format!("Riwayat {id} tidak ada."));
                return 1;
            };
            let per_file = !c.laporan.is_empty();
            if let Some(p) = self.o.laporan.clone() {
                let isi = if per_file { riwayat::html_verifikasi(c) } else { riwayat::html_riwayat(std::slice::from_ref(c)) };
                return tulis(&p, &isi, "Laporan");
            }
            if let Some(p) = self.o.csv.clone() {
                let isi = if per_file { riwayat::csv_per_file(c) } else { riwayat::csv(std::slice::from_ref(c)) };
                return tulis(&p, &isi, "CSV");
            }
            self.catat("Riwayat", serde_json::to_value(vec![c]).unwrap_or(Value::Null));
            judul(&format!("{} {}", riwayat::label_jenis(c), waktu_tampil(&c.mulai)));
            info(&format!("Dari  : {}", c.sumber));
            info(&format!("Ke    : {}", c.tujuan));
            info(&format!("Hasil : {} ({})", riwayat::label_hasil(c), riwayat::ringkas(c)));
            if !c.ket.is_empty() {
                info(&format!("Ket   : {}", c.ket));
            }
            if !c.log.is_empty() {
                info(&format!("Log   : {}", c.log));
            }
            return 0;
        }
        if let Some(p) = self.o.csv.clone() {
            return tulis(&p, &riwayat::csv(&semua), "CSV riwayat");
        }
        if let Some(p) = self.o.laporan.clone() {
            return tulis(&p, &riwayat::html_riwayat(&semua), "Laporan riwayat");
        }
        let n = self.o.jumlah.as_deref().and_then(|j| j.parse::<usize>().ok()).unwrap_or(20).min(semua.len());
        judul("Riwayat");
        if semua.is_empty() {
            info("Belum ada riwayat. Riwayat dicatat sejak versi yang memuat fitur ini.");
        }
        for c in &semua[..n] {
            let baris = format!(
                "{}  {} {}: {} -> {}",
                waktu_tampil(&c.mulai),
                riwayat::label_jenis(c),
                riwayat::label_hasil(c).to_lowercase(),
                nama_tempat(&c.sumber),
                nama_tempat(&c.tujuan)
            );
            if riwayat::hasil_baik(c) {
                ok(&baris);
            } else {
                awas(&baris);
            }
            let mut ket = format!("    {}", riwayat::ringkas(c));
            if !c.jadwal.is_empty() {
                ket += &format!(" (jadwal {})", c.jadwal);
            }
            info(&format!("{ket}  [-Id {}]", c.id));
        }
        if semua.len() > n {
            info(&format!("... {} lagi. Tampilkan lebih banyak: pindah riwayat -Jumlah 100", angka((semua.len() - n) as i64)));
        }
        self.catat("Riwayat", serde_json::to_value(&semua[..n]).unwrap_or(Value::Null));
        self.catat("Jumlah", semua.len() as i64);
        0
    }
}
