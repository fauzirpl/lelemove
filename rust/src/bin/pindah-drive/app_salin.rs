//! Jendela utama, bagian `impl App`: salin, verifikasi.
use super::*;

impl App {
    // -------------------------------------------------------------- salin

    pub(crate) fn klik_salin(&mut self) {
        let Some((s, t)) = self.ambil_isian() else { return };
        // Selalu periksa & uji coba dulu, supaya konfirmasi menyebut jumlah file yang sebenarnya.
        let Some(u) = self.ringkas_uji.clone().filter(|_| self.sudah_diperiksa) else {
            self.salin_setelah_uji = true;
            self.klik_periksa();
            return;
        };
        if h_bil(&u, "AkanDisalin") == 0 {
            self.set_pesan("Tidak ada yang perlu disalin: semua file sudah ada di tujuan.", HIJAU);
            return;
        }
        let mut batas = if self.batas == "off" {
            String::new()
        } else {
            format!("\nKecepatan dibatasi {}.", rute::tampil_batas(&self.batas))
        };
        if !self.filter.kosong() {
            batas += &format!("\nFilter: {}.", self.filter.uraian());
        }
        let isi = format!(
            "Dari:\n    {}\n\nKe:\n    {}\n\nAkan disalin: {} file, sekitar {}.{batas}\n\nFile di sumber tidak diubah atau dihapus. Penyalinan berjalan di latar belakang; aplikasi boleh ditutup.",
            self.nama_jalur(&s),
            self.nama_jalur(&t),
            angka(h_bil(&u, "AkanDisalin")),
            ukuran(h_bil(&u, "AkanBytes") as f64)
        );
        self.tanya("Mulai salin", isi, "Mulai salin", false, Konfirmasi::MulaiSalin);
    }

    /// Apakah penyalinan ini melibatkan akun Google yang memakai client ID bersama?
    pub(crate) fn job_client_bersama(&self, job: &serde_json::Map<String, Value>) -> bool {
        ["Sumber", "Tujuan"].iter().any(|k| {
            self.akun_dari(&akun::remote_dari(&teks(job.get(*k))))
                .is_some_and(|a| a.jenis == "google" && !a.client_sendiri)
        })
    }

    /// Dibaca ulang tiap 1,6 detik dari job.json + log rclone.
    pub(crate) fn perbarui_progres(&mut self) {
        let job = baca_job().filter(|j| !teks(j.get("Log")).is_empty());
        let Some(job) = job else {
            self.progres = None;
            self.job_jalan = false;
            return;
        };
        let jalan = job_berjalan(&job);
        let log = teks(job.get("Log"));
        let s = baca_statistik(&log);

        let mut persen = 0u32;
        let mut detail: Vec<String> = Vec::new();
        if let Some(s) = &s {
            if let Some(c) = RE_PERSEN.captures(&s.data) {
                persen = c[1].parse().unwrap_or(0);
            }
            let bag: Vec<&str> = RE_KOMA.split(&s.data).collect();
            if let Some(c) = bag.first().and_then(|b| RE_DARI_TOTAL.captures(b)) {
                let total_b = RE_UKURAN
                    .captures(&c[2])
                    .map(|u| ke_byte(&u[1], u.get(2).map_or("", |m| m.as_str())))
                    .unwrap_or(0.0);
                if total_b > 0.0 {
                    detail.push(format!("{} dari {}", ubah_ukuran(&c[1]), ubah_ukuran(&c[2])));
                } else if jalan {
                    detail.push(format!("Menghitung daftar file{ELIPSIS}"));
                } else {
                    detail.push("Tidak ada file baru".into());
                }
            }
            if let Some(c) = RE_FILE.captures(&s.file) {
                detail.push(format!(
                    "{} dari {} file",
                    angka(c[1].parse().unwrap_or(0)),
                    angka(c[2].parse().unwrap_or(0))
                ));
            }
            if jalan && bag.len() > 2 && bag[2].ends_with("/s") {
                detail.push(format!("{}/dtk", ubah_ukuran(bag[2].trim_end_matches("/s"))));
            }
            if jalan {
                if let Some(c) = RE_ETA.captures(&s.data) {
                    let e = ubah_durasi(&c[1]);
                    if !e.is_empty() {
                        detail.push(format!("sisa {e}"));
                    }
                }
            }
            if !s.error.is_empty() && !RE_NOL.is_match(&s.error) {
                detail.push(format!("{} error", RE_KURUNG.replace(&s.error, "")));
            }
        }

        let mut catatan: Option<Teks> = None;
        let saran_client = if self.job_client_bersama(&job) {
            if self.ada_client_bawaan {
                " Akun Google di penyalinan ini memakai client ID bersama rclone: hentikan, buka Pengaturan > Client ID Google > \"Login ulang dengan client ID ini\", lalu klik Lanjutkan."
            } else {
                " Akun Google di penyalinan ini memakai client ID bersama rclone: hentikan, pasang client ID sendiri lewat Pengaturan > Client ID Google, login ulang akun itu, lalu klik Lanjutkan."
            }
        } else {
            ""
        };
        let saran_login = "Login salah satu akun kedaluwarsa (invalid_grant). Hentikan, klik Login ulang pada akun itu di daftar kiri, lalu klik Lanjutkan.";
        let kode = teks(job.get("Kode"));
        // Penyalinan milik jadwal backup: rutenya bisa beda dengan panel Dari/Ke, jadi jendela
        // tidak memverifikasi atau menawarkan "Lanjutkan" untuknya (pindah.exe jadwal sudah memverifikasi).
        let nama_jadwal = teks(job.get("JadwalNama"));
        let dari_jadwal = !teks(job.get("Jadwal")).is_empty();
        let mut warna = HIJAU;
        let (mut judul, mut file);
        if jalan && teks(job.get("Menunggu")) == "jaringan" {
            // Pekerja latar sedang menunggu internet kembali (lihat mesin::lanjut).
            warna = KUNING;
            judul = "Menunggu jaringan".to_string();
            file = format!(
                "Dicoba lagi otomatis sekitar pukul {} (percobaan {} dari {}).",
                teks(job.get("UlangPukul")),
                bil(job.get("Percobaan")).unwrap_or(1) + 1,
                teks(job.get("PercobaanMaks"))
            );
            self.label_salin = "Mulai salin".into();
            catatan = Some((
                "Internet terputus. LELEMOVE melanjutkan sendiri begitu tersambung lagi; file yang sudah sampai tidak diulang. Aplikasi boleh ditutup. Klik Hentikan untuk membatalkan.".into(),
                KUNING,
            ));
        } else if jalan {
            judul = "Sedang menyalin".to_string();
            file = match &s {
                Some(s) if !s.sedang.is_empty() => format!("Sekarang: {}", s.sedang.join(", ")),
                _ => String::new(),
            };
            self.label_salin = "Mulai salin".into();
            let mut c = ("Aplikasi boleh ditutup. Penyalinan tetap berjalan dan progresnya muncul lagi saat aplikasi dibuka.".to_string(), REDUP);
            // Tidak bergerak: byte tersalin sama selama 90 detik, atau log memuat penolakan rate limit.
            if let Some(s) = &s {
                let kunci = format!("{log}|{}", s.data.split(',').next().unwrap_or(""));
                if kunci != self.progres_kunci {
                    self.progres_kunci = kunci;
                    self.progres_sejak = Instant::now();
                    let (_, _, _, jam, mnt, _) = waktu_lokal();
                    self.progres_sejak_jam = format!("{jam:02}.{mnt:02}");
                }
                let diam = self.progres_sejak.elapsed().as_secs();
                if s.perlu_login {
                    c = (saran_login.into(), MERAH);
                    warna = KUNING;
                } else if diam >= 30 && s.jaringan {
                    c = (
                        "Koneksi internet sedang bermasalah; rclone mencoba lagi. Bila putus lama, LELEMOVE menunggu lalu melanjutkan otomatis.".into(),
                        KUNING,
                    );
                    warna = KUNING;
                } else if s.dibatasi || diam >= 90 {
                    let mut t = if s.dibatasi {
                        "Layanan cloud sedang membatasi permintaan (rate limit). rclone menunggu lalu mencoba lagi otomatis.".to_string()
                    } else {
                        format!(
                            "Tidak ada kemajuan sejak {}. Biasanya karena layanan cloud membatasi permintaan; rclone menunggu lalu mencoba lagi otomatis.",
                            self.progres_sejak_jam
                        )
                    };
                    t += saran_client;
                    c = (t, KUNING);
                    warna = KUNING;
                }
            }
            catatan = Some(c);
        } else if kode == "0" {
            persen = 100;
            judul = "Penyalinan selesai".into();
            file = format!("Selesai {}.", waktu_tampil(&teks(job.get("Selesai"))));
            self.label_salin = "Salin lagi".into();
        } else {
            warna = KUNING;
            if kode == "dihentikan" {
                judul = "Dihentikan".into();
                file = "Klik \"Lanjutkan\" untuk meneruskan; file yang sudah sampai dilewati.".into();
            } else if !teks(job.get("Selesai")).is_empty() {
                judul = "Berhenti karena ada kesalahan".into();
                file = baris_error(&log, 1)
                    .first()
                    .map(|e| RE_AWAL_ERROR.replace(e, "").to_string())
                    .unwrap_or_else(|| "Lihat log untuk detailnya.".into());
            } else {
                judul = "Terputus".into();
                file = "Komputer mati atau proses ditutup. Klik \"Lanjutkan\" untuk meneruskan.".into();
            }
            if let Some(s) = &s {
                if s.perlu_login {
                    catatan = Some((saran_login.into(), MERAH));
                } else if s.dibatasi {
                    catatan = Some((format!("Log menunjukkan layanan cloud membatasi permintaan (rate limit).{saran_client}"), KUNING));
                }
            }
            self.label_salin = "Lanjutkan".into();
        }
        if dari_jadwal {
            let nama = if nama_jadwal.is_empty() { "backup terjadwal".to_string() } else { format!("\u{201C}{nama_jadwal}\u{201D}") };
            if jalan && teks(job.get("Menunggu")) != "jaringan" {
                judul = format!("Backup terjadwal {nama}");
            } else if !jalan && kode == "0" {
                judul = format!("Backup terjadwal {nama} selesai");
                file += " Hasil verifikasinya ada di Jadwal backup.";
            } else if !jalan {
                judul = format!("Backup terjadwal {nama}: {}", judul.to_lowercase());
                file = format!("{} Dicoba lagi pada jadwal berikutnya, atau klik Jalankan sekarang di Jadwal backup.", file.trim_end());
            }
            if !jalan {
                self.label_salin = "Mulai salin".into();
            }
        }
        self.progres = Some(Progres { judul, persen, warna, detail: detail.join(&format!("   {TITIK}   ")), file, catatan });

        // Penyalinan baru berhenti: folder tujuan kini (mungkin) sudah ada, jadi muat ulang panel Ke.
        if self.job_jalan && !jalan {
            self.tujuan_berubah();
            self.muat_panel(KE, true);
        }
        // Penyalinan baru saja selesai saat aplikasi terbuka: langsung verifikasi.
        if self.job_jalan && !jalan && kode == "0" && self.aksi.is_none() && !dari_jadwal {
            self.job_jalan = false;
            self.set_pesan(&format!("Penyalinan selesai. Memverifikasi{ELIPSIS}"), HIJAU);
            self.klik_verifikasi();
        }
        self.job_jalan = jalan;
    }

    // -------------------------------------------------------------- verifikasi

    pub(crate) fn klik_verifikasi(&mut self) {
        if self.ambil_isian().is_none() {
            return;
        }
        self.verifikasi = (format!("Mencocokkan setiap file{ELIPSIS}"), REDUP);
        self.bisa_pindahkan = None;
        self.jalankan_aksi("Memverifikasi hasil salinan", v!["verifikasi"], Setelah::Verifikasi);
    }

    pub(crate) fn selesai_verifikasi(&mut self, kode: i32, h: &Value) {
        // File yang cocok boleh dipindahkan (dihapus dari sumber) atas permintaan pengguna.
        let cocok = h_bil(h, "Cocok") - h_bil(h, "OfficeSharePoint");
        let lain = h_bil(h, "Hilang") + h_bil(h, "Beda") + h_bil(h, "Galat") + h_bil(h, "OfficeSharePoint");
        self.bisa_pindahkan =
            (h_ada(h, "Cocok") && cocok > 0).then(|| (cocok, lain, self.jalur_sumber(), self.jalur_tujuan()));
        let perlu_login = self.catat_perlu_login(h);
        if !h_ada(h, "Cocok") {
            self.verifikasi = match perlu_login {
                Some(l) => (format!("Verifikasi tidak berjalan: login {l} kedaluwarsa. Klik Login ulang pada akun itu."), MERAH),
                None => ("Verifikasi tidak berjalan. Lihat log aktivitas.".into(), MERAH),
            };
            self.log_terbuka = true;
            return;
        }
        let mut cara = if teks(h.get("Metode")) == "ukuran" {
            "dicocokkan per nama & ukuran, karena kedua layanan tidak punya jenis checksum yang sama".to_string()
        } else {
            "dicocokkan dengan checksum".to_string()
        };
        if h_bil(h, "OfficeSharePoint") > 0 {
            cara += &format!(
                "; {} file Office diubah metadatanya oleh SharePoint dan dianggap cocok",
                angka(h_bil(h, "OfficeSharePoint"))
            );
        }
        if kode == 0 {
            self.verifikasi = (
                format!(
                    "{CENTANG}  Verifikasi: semua {} file dari {} sudah ada di tujuan ({cara}).",
                    angka(h_bil(h, "Cocok")),
                    self.nama_pendek(&teks(h.get("Sumber")))
                ),
                HIJAU,
            );
            self.set_pesan("Selesai. Salinan di tujuan lengkap.", HIJAU);
            return;
        }
        let mut bag = vec![format!("{} cocok", angka(h_bil(h, "Cocok")))];
        if h_bil(h, "Hilang") > 0 {
            bag.push(format!("{} belum ada di tujuan", angka(h_bil(h, "Hilang"))));
        }
        if h_bil(h, "Beda") > 0 {
            bag.push(format!("{} isinya berbeda", angka(h_bil(h, "Beda"))));
        }
        if h_bil(h, "Galat") > 0 {
            bag.push(format!("{} gagal dibaca", angka(h_bil(h, "Galat"))));
        }
        self.verifikasi = (format!("{}. Klik \"Lanjutkan\" untuk melengkapi, lalu verifikasi lagi.", bag.join(", ")), KUNING);
        self.label_salin = "Lanjutkan".into();
        self.set_pesan("Verifikasi menemukan selisih.", KUNING);
    }
}
