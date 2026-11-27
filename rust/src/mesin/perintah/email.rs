use serde_json::Value;

use super::super::akun::*;
use super::super::dasar::*;
use super::super::email::{self as em, Akun as AkunEmail, Jenis as JenisEmail, Kred, Migrasi, OpsiMigrasi, Ujung};
use super::super::oauth::{self, Klien};
use super::super::riwayat::{self, Catatan};
use super::Mesin;
use crate::{notifikasi, win};

impl Mesin {
    // -------------------------------------------------------------- migrasi email

    /// Kredensial satu sisi: Outlook selalu OAuth (client ID Azure), Gmail OAuth bila dipilih
    /// (client ID Google LELEMOVE), selain itu kata sandi dari stdin JSON {"Sumber","Tujuan"} atau
    /// diketik di terminal (tidak ditampilkan).
    fn kred_email(&mut self, a: &AkunEmail, sisi: &str, login_google: bool) -> Result<Kred, String> {
        match a.jenis {
            JenisEmail::Outlook => {
                let id = em::client_microsoft().ok_or(
                    "Login Outlook/Microsoft 365 butuh client ID aplikasi Azure. Simpan dulu: pindah email -ClientMicrosoft <client ID> (panduan di README, bagian Migrasi email).",
                )?;
                Ok(Kred::OAuth { p: &oauth::MICROSOFT, klien: Klien { id, rahasia: None }, token: None })
            }
            JenisEmail::Gmail if login_google => {
                let (id, rahasia) = baca_client_bawaan()
                    .ok_or("Login Google butuh client ID Google (Pengaturan > Client ID Google). Atau pakai sandi aplikasi Gmail.")?;
                Ok(Kred::OAuth { p: &oauth::GOOGLE, klien: Klien { id, rahasia: Some(rahasia) }, token: None })
            }
            _ => {
                let r = self.baca_rahasia();
                if let Some(s) = r.get(sisi).and_then(Value::as_str).filter(|s| !s.is_empty()) {
                    return Ok(Kred::Sandi(s.to_string()));
                }
                if self.o.rahasia_stdin || !bisa_bertanya() {
                    return Err(format!("Kata sandi {} belum diberikan.", a.tampil()));
                }
                polos(&format!("        Kata sandi {} (tidak ditampilkan): ", a.tampil()));
                let s = win::baca_sandi();
                if s.is_empty() {
                    return Err("Kata sandi kosong.".into());
                }
                Ok(Kred::Sandi(s))
            }
        }
    }

    /// pindah email -Sumber <akun|jalur> -Tujuan <akun|jalur> [-HanyaPeriksa] [-Ya]: salin semua
    /// folder email antar server, ke folder cadangan di komputer, atau dari cadangan/mbox ke server.
    pub(super) fn email(&mut self) -> i32 {
        if let Some(id) = self.o.client_microsoft.clone() {
            judul("Migrasi email");
            if !em::client_microsoft_sah(&id) {
                gagal("Client ID Microsoft harus berbentuk Application (client) ID dari Azure, mis. 0c3f1a2b-1234-4abc-9def-0123456789ab.");
                return 1;
            }
            em::simpan_client_microsoft(&id);
            ok("Client ID Microsoft disimpan.");
            self.catat("ClientMicrosoft", true);
            if self.o.sumber.is_none() && self.o.tujuan.is_none() {
                return 0;
            }
        }
        let mut set = em::baca_setelan();
        if let Some(s) = &self.o.sumber {
            set.sumber = s.clone();
        }
        if let Some(t) = &self.o.tujuan {
            set.tujuan = t.clone();
        }
        if let Some(l) = &self.o.login {
            set.login_google = matches!(l.as_str(), "google" | "oauth");
        }
        set.sertakan_sampah = self.o.sertakan_sampah;
        set.tanpa_arsip = self.o.tanpa_arsip;
        set.abaikan_sertifikat = self.o.abaikan_sertifikat;
        if set.sumber.is_empty() || set.tujuan.is_empty() {
            judul("Migrasi email");
            gagal("Sumber dan tujuan email belum diisi.");
            info("Contoh: pindah email -Sumber gmail:nama@gmail.com -Tujuan imap://nama@kantor.go.id@mail.kantor.go.id");
            info("        pindah email -Sumber gmail:nama@gmail.com -Tujuan \"D:\\Cadangan Email\"          (cadangkan ke komputer)");
            info("        pindah email -Sumber \"D:\\Cadangan Email\" -Tujuan imap://nama@mail.server.id     (pulihkan)");
            info("        pindah email -Sumber \"D:\\Takeout\\Mail\\semua.mbox\" -Tujuan imap://nama@mail.server.id");
            return 1;
        }
        let (sumber, tujuan) = match (Ujung::urai(&set.sumber), Ujung::urai(&set.tujuan)) {
            (Ok(s), Ok(t)) => (s, t),
            (Err(e), _) | (_, Err(e)) => {
                judul("Migrasi email");
                gagal(&e);
                return 1;
            }
        };
        let kerja = em::nama_kerja(&sumber, &tujuan);
        judul(kerja);
        let sama = match (&sumber, &tujuan) {
            (Ujung::Server(a), Ujung::Server(b)) => a.host == b.host && a.pengguna.eq_ignore_ascii_case(&b.pengguna),
            (Ujung::Lokal(a), Ujung::Lokal(b)) => a == b || (a.canonicalize().ok().is_some() && a.canonicalize().ok() == b.canonicalize().ok()),
            _ => false,
        };
        if sama {
            gagal("Sumber dan tujuan adalah kotak surat yang sama.");
            return 1;
        }
        // Simpan bentuk baku (jalur "~/..." sudah diperluas, tanda kutip dibuang).
        set.sumber = sumber.alamat();
        set.tujuan = tujuan.alamat();
        if sumber.lokal() || tujuan.lokal() {
            em::simpan_setelan_cadangan(&set);
        } else {
            em::simpan_setelan(&set);
        }
        self.catat("Sumber", sumber.tampil());
        self.catat("Tujuan", tujuan.tampil());
        self.catat("Kerja", kerja);
        let kred = |me: &mut Self, u: &Ujung, sisi: &str, google: bool| -> Result<Option<Kred>, String> {
            match u {
                Ujung::Server(a) => me.kred_email(a, sisi, google).map(Some),
                Ujung::Lokal(_) => Ok(None),
            }
        };
        let k = kred(self, &sumber, "Sumber", set.login_google).and_then(|s| Ok((s, kred(self, &tujuan, "Tujuan", false)?)));
        let (kred_s, kred_t) = match k {
            Ok(k) => k,
            Err(e) => {
                gagal(&e);
                self.catat("Galat", e);
                return 1;
            }
        };
        if set.abaikan_sertifikat && (!sumber.lokal() || !tujuan.lokal()) {
            awas("Sertifikat TLS tidak diperiksa (sertifikat swatanda diterima).");
        }
        let opsi = OpsiMigrasi { sertakan_sampah: set.sertakan_sampah, arsip_gmail: !set.tanpa_arsip };
        let mut m = match Migrasi::baru(sumber.clone(), kred_s, tujuan.clone(), kred_t, set.abaikan_sertifikat, opsi, self.o.mesin) {
            Ok(m) => m,
            Err(e) => {
                gagal(&e);
                self.catat("Galat", e);
                return 1;
            }
        };
        let rencana = match m.siapkan() {
            Ok(r) => r,
            Err(e) => {
                gagal(&e);
                self.catat("Galat", e);
                m.tutup();
                return 1;
            }
        };
        judul("Peta folder");
        for r in &rencana {
            match &r.lewati {
                Some(l) => info(&format!("{}: dilewati ({l})", r.tampil_sumber)),
                None => info(&format!("{} -> {}{}", r.tampil_sumber, r.tampil_tujuan, if r.ada { "" } else { " (baru)" })),
            }
        }
        if self.o.hanya_periksa {
            judul("Memeriksa isi folder");
            let h = m.periksa(&rencana);
            m.tutup();
            self.catat("Folder", serde_json::to_value(&m.folder).unwrap_or(Value::Null));
            if let Err(e) = h {
                gagal(&e);
                self.catat("Galat", e);
                return 1;
            }
            let baru: u64 = m.folder.iter().map(|f| f.email - f.sudah_ada).sum();
            let byte: u64 = m.folder.iter().map(|f| f.byte).sum();
            ok(&format!("{} email belum ada di tujuan ({}).", angka(baru as i64), format_ukuran(byte as f64)));
            if sumber.akun().is_some_and(|a| a.jenis == JenisEmail::Gmail) && byte > 2_000_000_000 {
                awas("Gmail membatasi unduhan IMAP sekitar 2,5 GB per hari: migrasi mungkin perlu dijalankan ulang esok hari (yang sudah tersalin dilewati).");
                info("Jalan pintas untuk kotak surat besar: unduh berkas .mbox lewat Google Takeout (takeout.google.com), lalu pulihkan dari berkas itu.");
            }
            self.catat("Baru", baru as i64);
            self.catat("Byte", byte as i64);
            return 0;
        }
        if !self.konfirmasi("Mulai menyalin email? Email di sumber tidak diubah atau dihapus.") {
            m.tutup();
            return 1;
        }
        let mulai = sekarang_iso();
        let hasil = m.salin(&rencana);
        m.tutup();
        let (sudah, disalin, gagal_n, byte) = m.jumlah();
        self.catat("Folder", serde_json::to_value(&m.folder).unwrap_or(Value::Null));
        self.catat("SudahAda", sudah as i64);
        self.catat("Disalin", disalin as i64);
        self.catat("Gagal", gagal_n as i64);
        self.catat("Byte", byte as i64);
        judul(&format!("Hasil {}", kerja.to_lowercase()));
        let (status, ket, kode) = match &hasil {
            Ok(()) if gagal_n == 0 => ("selesai", String::new(), 0),
            Ok(()) => ("sebagian", format!("{} email gagal disalin", angka(gagal_n as i64)), 1),
            Err(e) => ("galat", e.clone(), 1),
        };
        riwayat::catat(Catatan {
            jenis: "email".into(),
            mulai,
            sumber: sumber.tampil(),
            tujuan: tujuan.tampil(),
            hasil: status.into(),
            ket: ket.clone(),
            file: disalin as i64,
            byte: byte as i64,
            galat: gagal_n as i64,
            ..Default::default()
        });
        match hasil {
            Ok(()) => {
                ok(&format!(
                    "{} email disalin ({}), {} sudah ada sebelumnya.",
                    angka(disalin as i64),
                    format_ukuran(byte as f64),
                    angka(sudah as i64)
                ));
                if gagal_n > 0 {
                    awas(&format!("{} email gagal disalin (lihat daftar di atas). Jalankan lagi untuk mencoba ulang.", angka(gagal_n as i64)));
                }
            }
            Err(e) => {
                gagal(&e);
                info("Jalankan lagi untuk melanjutkan: email yang sudah tersalin dilewati.");
                self.catat("Galat", e);
            }
        }
        let _ = notifikasi::kirim(
            &format!("{kerja} {}", if kode == 0 { "selesai" } else { "belum tuntas" }),
            &[format!("{} -> {}", sumber.tampil(), tujuan.tampil()), format!("{} email disalin", angka(disalin as i64))],
        );
        kode
    }
}
