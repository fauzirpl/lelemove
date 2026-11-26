//! Jendela utama, bagian `impl App`: panel Dari/Ke.
use super::*;

impl App {
    // -------------------------------------------------------------- panel Dari/Ke

    pub(crate) fn lok(&self, sisi: usize) -> &str {
        if sisi == DARI {
            &self.lok_sumber
        } else {
            &self.lok_tujuan
        }
    }

    /// Jalur folder di dalam lokasinya: "D:\Data" (komputer) atau "Arsip/2024" (cloud, tanpa garis miring awal).
    pub(crate) fn dalam(&self, sisi: usize) -> String {
        let j = if sisi == DARI { self.jalur_sumber() } else { self.jalur_tujuan() };
        if self.lok(sisi).is_empty() {
            j
        } else {
            pecah(&j).1
        }
    }

    /// Mengganti lokasi satu panel. `isi` None = folder bawaan lokasi itu.
    pub(crate) fn pilih_lokasi(&mut self, sisi: usize, lok: &str, isi: Option<String>) {
        self.aktif = sisi;
        self.panel[sisi].edit = None;
        if sisi == DARI {
            self.lok_sumber = lok.to_string();
            self.isi_saran_sumber();
            self.isi_sumber = isi.unwrap_or_default();
            self.sumber_berubah();
            return;
        }
        let ganti = self.lok_tujuan != lok;
        self.lok_tujuan = lok.to_string();
        match isi {
            Some(i) => self.set_isi(KE, i),
            None if ganti => self.lokasi_tujuan_berubah(),
            None => {}
        }
    }

    /// Pindah ke folder lain di lokasi yang sama (klik dua kali, bilah alamat, Naik).
    pub(crate) fn set_isi(&mut self, sisi: usize, isi: String) {
        self.aktif = sisi;
        self.panel[sisi].edit = None;
        if sisi == DARI {
            self.isi_sumber = isi;
            self.sumber_berubah();
        } else {
            self.isi_tujuan = isi;
            self.tujuan_manual = true;
            self.sudah_diperiksa = false;
            self.tujuan_berubah();
        }
    }

    pub(crate) fn buka_sub(&mut self, sisi: usize, nama: &str) {
        let skrg = self.dalam(sisi);
        let baru = if !self.lok(sisi).is_empty() {
            if skrg.is_empty() {
                nama.to_string()
            } else {
                format!("{skrg}/{nama}")
            }
        } else if skrg.is_empty() {
            nama.to_string() // akar drive, mis. "D:\"
        } else {
            Path::new(&skrg).join(nama).to_string_lossy().to_string()
        };
        self.set_isi(sisi, baru);
    }

    pub(crate) fn naik(&mut self, sisi: usize) {
        let skrg = self.dalam(sisi);
        let baru = if !self.lok(sisi).is_empty() {
            skrg.rfind('/').map(|i| skrg[..i].to_string()).unwrap_or_default()
        } else {
            Path::new(&skrg).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()
        };
        self.set_isi(sisi, baru);
    }

    pub(crate) fn bisa_naik(&self, sisi: usize) -> bool {
        !self.dalam(sisi).is_empty()
    }

    /// Memuat isi folder panel di thread latar bila lokasi/jalurnya berubah (atau `paksa`).
    pub(crate) fn muat_panel(&mut self, sisi: usize, paksa: bool) {
        if !self.rute_siap {
            return;
        }
        let lok = self.lok(sisi).to_string();
        let dalam = self.dalam(sisi);
        let kunci = format!("{lok}\u{1}{dalam}");
        if !paksa && self.panel[sisi].kunci.as_deref() == Some(kunci.as_str()) {
            return;
        }
        self.gen += 1;
        let gen = self.gen;
        let p = &mut self.panel[sisi];
        p.kunci = Some(kunci);
        p.gen = gen;
        p.isi = None;
        p.pilih = None;
        aksi::latar(&self.tx, &self.ctx, move || {
            let isi = if lok.is_empty() {
                isi_lokal(&dalam)
            } else {
                match akun::daftar_folder_cloud(&lok, &dalam) {
                    Ok(d) => Isi::Folder(d),
                    Err(e) if e.to_lowercase().contains("not found") => Isi::BelumAda,
                    Err(e) => Isi::Galat(e),
                }
            };
            Kabar::Daftar { gen, isi }
        });
    }

    pub(crate) fn panel_memuat(&self) -> bool {
        self.panel.iter().any(|p| p.kunci.is_some() && p.isi.is_none())
    }

    pub(crate) fn sumber_berubah(&mut self) {
        let rem = self.lok_sumber.clone();
        let s = self.jalur_sumber();
        self.sudah_diperiksa = false;
        self.ringkas_uji = None;
        self.periksa = None;
        self.info_sumber = if rem.is_empty() {
            if s.is_empty() {
                ("Buka drive dan folder yang mau disalin.".into(), REDUP)
            } else if adalah_lokal(&s) && Path::new(&s).is_dir() {
                match self.jenis_sumber.get(&s) {
                    Some(j) => (format!("{j}. Seluruh isinya disalin."), REDUP),
                    None => ("Seluruh isi folder ini disalin.".into(), REDUP),
                }
            } else if adalah_lokal(&s) {
                ("Folder tidak ditemukan.".into(), MERAH)
            } else {
                (format!("Tulis alamat lengkap, misalnya {}.", win::CONTOH_DATA), KUNING)
            }
        } else {
            let a = self.akun_dari(&rem);
            let p = pecah(&s).1;
            let mut t = if !p.is_empty() {
                "Seluruh isi folder ini disalin".to_string()
            } else if a.as_ref().is_some_and(|a| a.jenis == "sharepoint") {
                "Seluruh isi pustaka disalin".to_string()
            } else {
                "Seluruh isi akun disalin".to_string()
            };
            let mut w = REDUP;
            match &a {
                Some(a) if a.jenis == "google" && a.scope != "drive" => {
                    t = "Akun ini hanya diizinkan sebagai tujuan. Klik \u{22EF} pada akun ini di daftar kiri, lalu \"Beri izin penuh\" agar bisa jadi sumber.".into();
                    w = KUNING;
                }
                Some(a) if a.jenis == "sharepoint" => t += &format!(" {TITIK} pustaka {}, situs {}.", a.drive, a.situs),
                Some(a) if !a.drive.is_empty() => t += &format!(" {TITIK} drive {}.", a.drive),
                _ => t.push('.'),
            }
            (t, w)
        };
        if !self.tujuan_manual {
            self.set_tujuan_bawaan();
        }
    }

    pub(crate) fn tujuan_berubah(&mut self) {
        let rem_t = self.lok_tujuan.clone();
        let t = self.jalur_tujuan();
        if rem_t.is_empty() {
            self.info_tujuan = if t.is_empty() {
                ("Buka drive dan folder tujuan.".into(), REDUP)
            } else if adalah_lokal(&t) && Path::new(&t).is_dir() {
                ("Disalin ke folder ini.".into(), REDUP)
            } else if adalah_lokal(&t) {
                ("Disalin ke folder ini; dibuat saat penyalinan dimulai.".into(), REDUP)
            } else {
                (format!("Tulis alamat lengkap, misalnya {}.", win::CONTOH_SALINAN), KUNING)
            };
            return;
        }
        let a = self.akun_dari(&rem_t);
        let p = pecah(&t).1;
        let mut teks_info =
            if p.is_empty() { "Disalin langsung ke folder teratas akun".to_string() } else { "Disalin ke folder ini (dibuat bila belum ada)".to_string() };
        let mut w = REDUP;
        match &a {
            Some(a) if a.jenis == "google" && !a.client_sendiri => {
                teks_info += ". Akun ini memakai client ID bersama rclone yang sering dibatasi Google.";
                w = KUNING;
            }
            Some(a) if a.jenis == "sharepoint" => {
                teks_info += &format!(
                    " {TITIK} pustaka {}, situs {}. File Office akan ditambahi metadata oleh SharePoint (wajar).",
                    a.drive, a.situs
                )
            }
            Some(a) if !a.drive.is_empty() => teks_info += &format!(" {TITIK} drive {}.", a.drive),
            _ => teks_info.push('.'),
        }
        self.info_tujuan = (teks_info, w);
    }

    pub(crate) fn lokasi_tujuan_berubah(&mut self) {
        if !self.tujuan_manual {
            self.set_tujuan_bawaan();
            return;
        }
        // Pindah antara komputer dan cloud: format jalurnya berbeda, jadi isi ulang.
        let lokal = adalah_lokal(&self.isi_tujuan);
        if self.lok_tujuan.is_empty() != lokal {
            self.set_tujuan_bawaan();
            return;
        }
        self.sudah_diperiksa = false;
        self.tujuan_berubah();
    }

    /// Membaca isian, memvalidasi, dan menyimpannya sebagai rute (dibaca juga oleh pindah.exe).
    pub(crate) fn ambil_isian(&mut self) -> Option<(String, String)> {
        let s = self.jalur_sumber();
        let t = self.jalur_tujuan();
        if self.lok_sumber.is_empty() {
            if s.is_empty() {
                self.set_pesan("Pilih folder sumber dulu.", MERAH);
                return None;
            }
            if !adalah_lokal(&s) || !Path::new(&s).is_dir() {
                self.set_pesan("Folder sumber tidak ditemukan.", MERAH);
                return None;
            }
        }
        if self.lok_tujuan.is_empty() && (t.is_empty() || !adalah_lokal(&t)) {
            self.set_pesan(&format!("Tulis folder tujuan lengkap, misalnya {}.", win::CONTOH_SALINAN), MERAH);
            return None;
        }
        if self.rclone.is_none() {
            self.set_pesan("Pasang rclone dulu (tombol di daftar kiri).", MERAH);
            return None;
        }
        rute::simpan_pengaturan(Some(&s), Some(&t));
        Some((s, t))
    }

    pub(crate) fn klik_telusuri(&mut self, untuk_sumber: bool) {
        let (lok, jalur_skrg) = if untuk_sumber {
            (self.lok_sumber.clone(), self.jalur_sumber())
        } else {
            (self.lok_tujuan.clone(), self.jalur_tujuan())
        };
        if !lok.is_empty() {
            let judul = format!(
                "Pilih folder {} di {}",
                if untuk_sumber { "SUMBER" } else { "TUJUAN" },
                nama_lokasi(self.akun_dari(&lok).as_ref())
            );
            let awal = pecah(&jalur_skrg).1;
            let mut p = Pemuat { tx: &self.tx, ctx: &self.ctx, gen: &mut self.gen };
            let j = Jelajah::baru(&lok, &awal, judul, untuk_sumber, &mut p);
            self.dialog = Some(Dialog::Jelajah(j));
            return;
        }
        let mut fd = rfd::FileDialog::new().set_title(if untuk_sumber {
            "Pilih folder yang mau disalin"
        } else {
            "Pilih folder tujuan (boleh membuat folder baru)"
        });
        if adalah_lokal(&jalur_skrg) && Path::new(&jalur_skrg).is_dir() {
            fd = fd.set_directory(&jalur_skrg);
        }
        if let Some(p) = fd.pick_folder() {
            let s = p.to_string_lossy().to_string();
            if untuk_sumber {
                self.isi_sumber = s;
                self.sumber_berubah();
            } else {
                self.isi_tujuan = s;
                self.tujuan_manual = true;
                self.sudah_diperiksa = false;
                self.tujuan_berubah();
            }
        }
    }

    pub(crate) fn klik_tukar(&mut self) {
        let (rem_s, rem_t) = (self.lok_sumber.clone(), self.lok_tujuan.clone());
        let (p_s, p_t) = (self.isi_sumber.clone(), self.isi_tujuan.clone());
        self.lok_sumber = rem_t;
        self.isi_saran_sumber();
        self.isi_sumber = p_t;
        self.lok_tujuan = rem_s;
        self.isi_tujuan = p_s;
        self.tujuan_manual = true;
        self.sumber_berubah();
        self.tujuan_berubah();
        self.set_pesan("Sumber dan tujuan ditukar.", REDUP);
    }

    pub(crate) fn klik_buka_tujuan(&mut self) {
        if self.lok_tujuan.is_empty() {
            let t = self.jalur_tujuan();
            if !t.is_empty() && Path::new(&t).is_dir() {
                win::buka(&t);
            } else {
                self.set_pesan("Folder tujuan belum ada.", REDUP);
            }
            return;
        }
        match self.akun_dari(&self.lok_tujuan).map(|a| a.jenis) .as_deref() {
            Some("google") => win::buka("https://drive.google.com/drive/my-drive"),
            Some("sharepoint") => win::buka("https://www.office.com/launch/sharepoint"),
            _ => win::buka("https://www.office.com/launch/onedrive"),
        }
    }
}
