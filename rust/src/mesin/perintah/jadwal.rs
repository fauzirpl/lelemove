use serde_json::{json, Value};
use std::path::Path;

use super::super::akun::*;
use super::super::dasar::*;
use super::super::filter::Filter;
use super::super::jadwal;
use super::super::riwayat::{self, Catatan};
use super::super::job::*;
use super::super::rclone::{cari_rclone, daftar_remote};
use super::super::rute::*;
use super::{Mesin, RE_AWAL_ERROR, nama_tempat, potong};
use crate::{notifikasi, win};

/// "12 file baru (1,20 GB)." atau "Tidak ada file baru." (untuk backup terjadwal).
fn ringkas_baru(s: &Statistik) -> String {
    let n: i64 = s.file.split('/').next().and_then(|x| x.trim().parse().ok()).unwrap_or(0);
    if n == 0 {
        return "Tidak ada file baru.".into();
    }
    let data = s.data.split('/').next().unwrap_or("").trim();
    let (a, satuan) = data.split_once(' ').unwrap_or((data, ""));
    format!("{} file baru ({}).", angka(n), format_ukuran(ke_byte(a, satuan)))
}

impl Mesin {
    // -------------------------------------------------------------- jadwal

    /// Backup terjadwal: daftar; -Tambah; -Id X dengan -Hapus / -Aktifkan / -Nonaktifkan;
    /// -Jalankan -Id X (dipanggil tugas Windows lewat LELEMOVE.exe --jadwal X).
    pub(super) fn jadwal(&mut self) -> i32 {
        if self.o.jalankan {
            let Some(id) = self.o.id.clone() else {
                gagal("Tambahkan -Id <id>.");
                return 1;
            };
            return self.jadwal_jalankan(&id);
        }
        if self.o.tambah {
            return self.jadwal_tambah();
        }
        if let Some(id) = self.o.id.clone() {
            let Some(mut j) = jadwal::cari(&id) else {
                gagal(&format!("Jadwal {id} tidak ada."));
                return 1;
            };
            if self.o.hapus {
                if let Err(e) = jadwal::hapus_tugas(&id) {
                    gagal(&format!("{} tidak bisa dihapus: {e}", kapital(win::NAMA_TUGAS)));
                    return 1;
                }
                jadwal::hapus_data(&id);
                ok(&format!("Jadwal \"{}\" dihapus.", j.nama));
                return 0;
            }
            if self.o.aktifkan || self.o.nonaktifkan {
                j.aktif = self.o.aktifkan;
                // Tugas yang hilang (dihapus dari Task Scheduler) dibuat ulang.
                let h = if jadwal::tugas_ada(&id) { jadwal::atur_aktif_tugas(&id, j.aktif) } else { jadwal::pasang_tugas(&j) };
                if let Err(e) = h {
                    gagal(&format!("{} tidak bisa diubah: {e}", kapital(win::NAMA_TUGAS)));
                    return 1;
                }
                jadwal::simpan(&j);
                ok(&format!("Jadwal \"{}\" {}.", j.nama, if j.aktif { "aktif" } else { "nonaktif" }));
                return 0;
            }
        }
        judul("Backup terjadwal");
        let semua = jadwal::baca_semua();
        if semua.is_empty() {
            info(&format!("Belum ada. Contoh: pindah jadwal -Tambah -Nama \"Arsip\" -Sumber \"{}\" -Tujuan \"pd-gdrive:Arsip\" -Jam 22:00 [-Hari sen,rab,jum]", win::CONTOH_DATA));
        }
        for j in &semua {
            let status = if !j.aktif {
                "nonaktif".to_string()
            } else if !jadwal::tugas_ada(&j.id) {
                format!("{} hilang; pasang ulang: pindah jadwal -Aktifkan -Id {}", win::NAMA_TUGAS, j.id)
            } else {
                format!("berikutnya {}", jadwal::berikutnya(j).unwrap_or_default())
            };
            ok(&format!("[{}] {}: {} ({status})", j.id, j.nama, jadwal::uraian(j)));
            info(&format!("    {}  ->  {}", j.sumber, j.tujuan));
            if !j.filter.kosong() {
                info(&format!("    filter: {}", j.filter.uraian()));
            }
            if let Some(t) = &j.terakhir {
                info(&format!("    terakhir {} ({}): {}", t.mulai, t.hasil, t.ket));
            }
        }
        self.catat("Jadwal", serde_json::to_value(&semua).unwrap_or(Value::Null));
        0
    }

    /// -Tambah -Sumber S -Tujuan T [-Nama N] [-Jam 22:00] [-Hari sen,rab] [-TanpaVerifikasi] [-Id X untuk mengubah]
    fn jadwal_tambah(&mut self) -> i32 {
        let (Some(src), Some(dst)) = (self.o.sumber.clone(), self.o.tujuan.clone()) else {
            gagal("Tambahkan -Sumber dan -Tujuan.");
            return 1;
        };
        let (src, dst) = (rapikan_sumber(&src), normalisasi_tujuan(&dst));
        let jam = self.o.jam.clone().unwrap_or_else(|| "22:00".into());
        let Some((h, m)) = jadwal::urai_jam(&jam) else {
            gagal(&format!("Jam tidak dikenal: \"{jam}\". Pakai format 22:00."));
            return 1;
        };
        let hari = match jadwal::urai_hari(self.o.hari.as_deref().unwrap_or("")) {
            Ok(h) => h,
            Err(e) => {
                gagal(&e);
                return 1;
            }
        };
        // Filter milik jadwal ini: hanya dari -Sertakan/-Lewati/-Sejak (bukan filter jendela).
        let filter = match Filter::urai(
            self.o.sertakan.as_deref().unwrap_or(""),
            self.o.lewati.as_deref().unwrap_or(""),
            self.o.sejak.as_deref().unwrap_or(""),
        ) {
            Ok(f) => f,
            Err(e) => {
                gagal(&e);
                return 1;
            }
        };
        let r = Rute { sumber: Some(src.clone()), tujuan: Some(dst.clone()) };
        if !self.rute_siap(&r, true) {
            return 1;
        }
        let id = self.o.id.clone().unwrap_or_else(jadwal::id_baru);
        let lama = jadwal::cari(&id);
        let j = jadwal::Jadwal {
            nama: self.o.nama.clone().unwrap_or_else(|| format!("Backup {}", nama_tempat(&src))),
            id: id.clone(),
            sumber: src,
            tujuan: dst,
            jam: format!("{h:02}:{m:02}"),
            hari,
            verifikasi: !self.o.tanpa_verifikasi,
            aktif: true,
            terakhir: lama.and_then(|l| l.terakhir),
            filter,
        };
        if let Err(e) = jadwal::pasang_tugas(&j) {
            gagal(&format!("{} tidak bisa dibuat: {e}", kapital(win::NAMA_TUGAS)));
            return 1;
        }
        jadwal::simpan(&j);
        ok(&format!("Jadwal \"{}\" dipasang: {}.", j.nama, jadwal::uraian(&j)));
        if !j.filter.kosong() {
            info(&format!("Filter: {}", j.filter.uraian()));
        }
        info(&format!("Berikutnya: {}. {}: {}", jadwal::berikutnya(&j).unwrap_or_default(), kapital(win::NAMA_TUGAS), jadwal::nama_tugas(&id)));
        self.catat("Id", id);
        0
    }

    /// Menjalankan satu jadwal di proses ini (tersembunyi; dipicu tugas Windows atau "Jalankan sekarang").
    fn jadwal_jalankan(&mut self, id: &str) -> i32 {
        let Some(j) = jadwal::cari(id) else {
            gagal(&format!("Jadwal {id} tidak ada."));
            return 1;
        };
        let mulai = sekarang_iso();
        self.nama_jadwal = j.nama.clone();
        let catat = |hasil: &str, ket: &str, log: &str| {
            jadwal::catat_terakhir(
                id,
                jadwal::Terakhir { mulai: mulai.clone(), selesai: sekarang_iso(), hasil: hasil.into(), ket: ket.into(), log: log.into() },
            )
        };
        // Jadwal yang tidak sempat menyalin juga masuk riwayat (bukti untuk audit).
        let riwayat_batal = |hasil: &str, ket: &str| {
            riwayat::catat(Catatan {
                jenis: "salin".into(),
                mulai: mulai.clone(),
                sumber: j.sumber.clone(),
                tujuan: j.tujuan.clone(),
                hasil: hasil.into(),
                ket: ket.into(),
                filter: j.filter.uraian(),
                jadwal: j.nama.clone(),
                ..Default::default()
            })
        };
        let rute = format!("{} \u{2192} {}", nama_tempat(&j.sumber), nama_tempat(&j.tujuan));
        if baca_job().is_some_and(|x| job_berjalan(&x)) {
            let ket = "Dilewati: penyalinan lain sedang berjalan.";
            awas(ket);
            catat("dilewati", ket, "");
            riwayat_batal("dilewati", ket);
            let _ = notifikasi::kirim(
                &format!("Backup terjadwal dilewati: {}", j.nama),
                &[rute, "Penyalinan lain sedang berjalan; dicoba lagi pada jadwal berikutnya.".into()],
            );
            return 0;
        }
        let r = Rute { sumber: Some(j.sumber.clone()), tujuan: Some(j.tujuan.clone()) };
        if !self.rute_siap(&r, true) {
            // Alasan yang spesifik untuk riwayat & notifikasi (pesan gagal() tidak terlihat di proses tersembunyi).
            let ket = if adalah_lokal(&j.sumber) && !Path::new(&j.sumber).is_dir() {
                format!("Folder sumber tidak ditemukan: {}", j.sumber)
            } else if cari_rclone().is_none() {
                "rclone belum terpasang. Buka LELEMOVE untuk memasangnya.".to_string()
            } else {
                let ada = daftar_remote();
                let lepas = [&j.sumber, &j.tujuan].iter().map(|x| remote_dari(x)).find(|r| !r.is_empty() && !ada.contains(r));
                match lepas {
                    Some(r) => format!("Akun {} sudah dilepas dari LELEMOVE; hubungkan lagi atau ubah jadwalnya.", nama_tempat(&format!("{r}:"))),
                    None => "Tujuan tidak boleh sama dengan sumber atau berada di dalam folder sumber.".to_string(),
                }
            };
            catat("galat", &ket, "");
            riwayat_batal("galat", &ket);
            let _ = notifikasi::kirim(&format!("Backup terjadwal gagal: {}", j.nama), &[rute, ket]);
            return 1;
        }
        bersihkan_log_lama();
        let log = path_log("jadwal", "log").to_string_lossy().to_string();
        let batas = baca_batas_kecepatan();
        perbarui_job(
            &[
                ("Log", json!(log)),
                ("Mulai", json!(sekarang_iso())),
                ("Sumber", json!(j.sumber)),
                ("Tujuan", json!(j.tujuan)),
                ("BatasKecepatan", if batas == "off" { Value::Null } else { json!(batas) }),
                ("PidInduk", json!(std::process::id())),
                ("Latar", json!(true)),
                ("Jadwal", json!(j.id)),
                ("JadwalNama", json!(j.nama)),
                ("Filter", if j.filter.kosong() { Value::Null } else { json!(j.filter.uraian()) }),
            ],
            true,
        );
        catat("berjalan", "", &log);
        // Sama dengan pekerja latar: lanjut otomatis saat jaringan putus + notifikasi galat.
        self.o.log_latar = Some(log.clone());
        self.dari_jadwal = true;
        let kode = self.jalankan_salin(&j.sumber, &j.tujuan, &log, &batas, false, &j.filter);
        if kode != 0 {
            let ket = baris_error(&log, 1)
                .pop()
                .map(|e| potong(&RE_AWAL_ERROR.replace(&e, ""), 160))
                .unwrap_or_else(|| format!("rclone berhenti dengan kode {kode}."));
            catat("galat", &ket, &log);
            return kode;
        }
        let baru = baca_statistik(&log).map(|s| ringkas_baru(&s)).unwrap_or_default();
        if !j.verifikasi {
            catat("selesai", &baru, &log);
            let _ = notifikasi::kirim(&format!("Backup terjadwal selesai: {}", j.nama), &[rute, baru]);
            return 0;
        }
        catat("berjalan", &format!("{baru} Memverifikasi hasil salinan\u{2026}"), &log);
        let v = self.verifikasi_rute(&j.sumber, &j.tujuan, &j.filter);
        let n = |k: &str| bil(self.hasil.get(k)).unwrap_or(0);
        if v == 0 {
            let ket = format!("{baru} Terverifikasi: {} file cocok.", angka(n("Cocok")));
            catat("selesai", &ket, &log);
            let _ = notifikasi::kirim(&format!("Backup terjadwal selesai: {}", j.nama), &[rute, ket]);
        } else {
            let ket = format!(
                "Verifikasi menemukan selisih: {} belum ada, {} berbeda, {} gagal dibaca.",
                angka(n("Hilang")),
                angka(n("Beda")),
                angka(n("Galat"))
            );
            catat("selisih", &ket, &log);
            let _ = notifikasi::kirim(
                &format!("Backup terjadwal: ada selisih ({})", j.nama),
                &[rute, format!("{ket} Buka LELEMOVE untuk melanjutkan.")],
            );
        }
        v
    }
}
