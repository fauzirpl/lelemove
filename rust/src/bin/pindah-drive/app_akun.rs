//! Jendela utama, bagian `impl App`: akun, layanan lain, jadwal backup.
use super::*;

impl App {
    // -------------------------------------------------------------- akun

    pub(crate) fn klik_tambah(&mut self, jenis: &str) {
        self.dialog = Some(Dialog::AkunBaru(AkunBaru {
            jenis: jenis.into(),
            label: String::new(),
            juga_sumber: false,
            client_bawaan: self.ada_client_bawaan,
            fokus: true,
        }));
    }

    // -------------------------------------------------------------- layanan lain

    pub(crate) fn buka_pemilih(&mut self) {
        self.dialog = Some(Dialog::Pemilih(layanan::Pemilih::baru()));
    }

    /// Isian kredensial untuk akun baru (akun None) atau login ulang akun yang ada.
    pub(crate) fn buka_kredensial(&mut self, kunci: &str, akun: Option<String>) {
        let Some(p) = mp::cari(kunci) else { return };
        let label = akun.as_ref().and_then(|n| self.akun_dari(n)).map(|a| a.label).unwrap_or_default();
        let mut k = layanan::Kredensial::baru(p, akun.clone(), label);
        // Login ulang: isian yang tidak rahasia diisi dari konfigurasi yang ada (rahasia diketik lagi).
        if let Some(n) = &akun {
            let kunci_biasa: Vec<&str> = p.kolom.iter().filter(|c| !c.tersamar()).map(|c| c.kunci).collect();
            for (kk, v) in akun::opsi_konfig(n, &kunci_biasa) {
                if let Some(i) = p.kolom.iter().position(|c| c.kunci == kk) {
                    k.nilai[i] = v;
                }
            }
        }
        self.dialog = Some(Dialog::Kredensial(k));
    }

    pub(crate) fn hubungkan_kredensial(&mut self, kunci: &str, akun_lama: Option<String>, label: &str, opsi: Vec<(String, String)>, rahasia: Vec<(String, String)>) {
        let Some(p) = mp::cari(kunci) else { return };
        let nama = akun_lama.clone().unwrap_or_else(|| akun::remote_baru(kunci));
        let mut a = v!["hubungkan", "-Akun", nama, "-Jenis", kunci, "-Ya"];
        if !label.is_empty() {
            a.extend(v!["-Label", label]);
        }
        if akun_lama.is_some() {
            a.push("-Ulang".into());
        }
        for (k, v) in &opsi {
            a.extend(v!["-Opsi", format!("{k}={v}")]);
        }
        // Rahasia lewat stdin sebagai JSON (selalu dikirim, walau kosong, supaya mesin tidak menunggu).
        let mut m = serde_json::Map::new();
        for (k, v) in rahasia {
            m.insert(k, Value::String(v));
        }
        a.push("-RahasiaStdin".into());
        let judul = if p.oauth { format!("Menunggu login {} di browser", p.nama) } else { format!("Menghubungkan {}", p.nama) };
        self.jalankan_aksi_masukan(&judul, a, Some(Value::Object(m).to_string()), Setelah::AkunPenyedia { nama, kunci: kunci.into() });
    }

    pub(crate) fn selesai_akun_penyedia(&mut self, kode: i32, h: &Value, nama: &str, kunci: &str) {
        let layanan_nama = mp::cari(kunci).map_or("Akun", |p| p.nama);
        // rclone bertanya lagi (kode 2FA, token, pilihan): tampilkan pertanyaannya.
        if let Some(t) = h.get("Tanya").filter(|t| !t.is_null()) {
            let pilihan: Vec<(String, String)> = t
                .get("Pilihan")
                .and_then(Value::as_array)
                .map(|a| a.iter().map(|x| (teks(x.get("Nilai")), teks(x.get("Ket")))).collect())
                .unwrap_or_default();
            let bawaan = teks(t.get("Bawaan"));
            let galat = teks(t.get("Galat"));
            self.set_pesan(&format!("{layanan_nama} meminta satu langkah lagi."), KUNING);
            self.dialog = Some(Dialog::TanyaKonfig(layanan::TanyaKonfig {
                akun: nama.to_string(),
                layanan: layanan_nama.to_string(),
                state: teks(t.get("State")),
                bantuan: teks(t.get("Bantuan")),
                rahasia: h_bool(t, "Rahasia", true),
                jawab: if pilihan.iter().any(|(v, _)| *v == bawaan) { bawaan } else { String::new() },
                pilihan,
                baru: h_bool(t, "Baru", true),
                galat: (!galat.is_empty()).then_some(galat),
                fokus: true,
            }));
            return;
        }
        self.segarkan_status();
        if kode == 0 {
            self.kredensial_terakhir = None;
            self.set_pesan(&format!("{layanan_nama} terhubung."), HIJAU);
            return;
        }
        // Gagal: buka lagi isiannya dengan pesan galat terakhir dari pindah.exe.
        let baris: Vec<&str> = self.log.lines().map(str::trim).filter(|b| !b.is_empty() && !b.starts_with("---")).collect();
        let e = baris
            .iter()
            .rev()
            .find(|b| b.starts_with("[GAGAL]"))
            .or(baris.last())
            .copied()
            .unwrap_or("Lihat log aktivitas.")
            .trim_start_matches("[GAGAL]")
            .trim()
            .to_string();
        self.set_pesan(&format!("{layanan_nama} belum terhubung."), MERAH);
        if let Some(mut k) = self.kredensial_terakhir.take() {
            k.galat = Some(e);
            self.dialog = Some(Dialog::Kredensial(k));
        } else {
            self.log_terbuka = true;
        }
    }

    pub(crate) fn buka_impor(&mut self) {
        self.gen += 1;
        let gen = self.gen;
        aksi::latar(&self.tx, &self.ctx, move || Kabar::Impor { gen, calon: akun::remote_luar() });
        self.dialog = Some(Dialog::Impor(layanan::Impor { calon: None, gen, pilih: None, label: String::new() }));
    }

    /// Jendela konsol `rclone config` untuk layanan yang tidak ada di daftar (semua backend rclone).
    pub(crate) fn buka_rclone_config(&mut self) {
        let Some(rc) = rclone::cari_rclone() else {
            self.set_pesan("Pasang rclone dulu (tombol di daftar kiri).", MERAH);
            return;
        };
        match win::buka_di_terminal(&rc, &["config"]) {
            Ok(_) => {
                self.set_pesan("Jendela rclone config terbuka. Setelah selesai, pilih Tambah akun > Layanan lain > Pakai remote rclone yang sudah ada.", TINTA);
                self.tulis_log("--- rclone config dibuka di jendela terpisah");
            }
            Err(e) => self.set_pesan(&format!("rclone config tidak bisa dibuka: {e}"), MERAH),
        }
    }

    /// Isi dialog Pengaturan dari keadaan saat ini (client ID aplikasi + akun Google).
    pub(crate) fn isi_pengaturan(&self, pesan: Option<(String, Color32)>) -> Pengaturan {
        Pengaturan {
            client: akun::info_client_bawaan().map(|(id, asal)| (akun::id_pendek(&id), asal.to_string())),
            google: self
                .akun
                .iter()
                .filter(|a| a.jenis == "google")
                .map(|a| AkunGoogle {
                    nama: a.nama.clone(),
                    label: a.label.clone(),
                    client_aplikasi: a.client_aplikasi,
                    client_sendiri: a.client_sendiri,
                })
                .collect(),
            pesan,
            sibuk: self.aksi.is_some() || self.job_jalan,
            lanjut_otomatis: rute::baca_lanjut_otomatis(),
        }
    }

    pub(crate) fn buka_pengaturan(&mut self, pesan: Option<(String, Color32)>) {
        self.dialog = Some(Dialog::Pengaturan(self.isi_pengaturan(pesan)));
    }

    // -------------------------------------------------------------- jadwal backup

    /// Membuka daftar jadwal; isinya dimuat di utas latar (schtasks /Query per jadwal).
    pub(crate) fn buka_jadwal(&mut self, pesan: Option<(String, Color32)>) {
        self.gen += 1;
        let gen = self.gen;
        aksi::latar(&self.tx, &self.ctx, move || {
            // Jadwal yang prosesnya masih hidup (job.json milik jadwal itu, PidInduk = pindah.exe).
            let hidup = baca_job()
                .filter(|x| bil(x.get("PidInduk")).is_some_and(|p| win::proses_hidup(p as u32, &["pindah"])))
                .map(|x| teks(x.get("Jadwal")))
                .unwrap_or_default();
            Kabar::Jadwal {
                gen,
                daftar: mj::baca_semua()
                    .into_iter()
                    .map(|j| {
                        let ada = mj::tugas_ada(&j.id);
                        let jalan = !hidup.is_empty() && hidup == j.id;
                        (j, ada, jalan)
                    })
                    .collect(),
            }
        });
        self.dialog = Some(Dialog::Jadwal(jadwal::Daftar {
            baris: None,
            gen,
            pesan,
            sibuk: self.aksi.is_some(),
            job_jalan: self.job_jalan,
        }));
    }
}
