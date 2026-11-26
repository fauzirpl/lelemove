//! Jendela utama, bagian `impl App`: sumber & tujuan.
use super::*;

impl App {
    // -------------------------------------------------------------- sumber & tujuan

    pub(crate) fn jalur_sumber(&self) -> String {
        gabung(&self.lok_sumber, &self.isi_sumber)
    }

    pub(crate) fn jalur_tujuan(&self) -> String {
        gabung(&self.lok_tujuan, &self.isi_tujuan)
    }

    /// Nama yang mudah dibaca untuk dialog dan hasil verifikasi.
    pub(crate) fn nama_jalur(&self, j: &str) -> String {
        let (rem, p) = pecah(j);
        if rem.is_empty() {
            return match self.jenis_sumber.get(j) {
                Some(k) => format!("{j} ({k})"),
                None => j.to_string(),
            };
        }
        let n = nama_lokasi(self.akun_dari(&rem).as_ref());
        if p.is_empty() {
            format!("{n} (seluruh isi)")
        } else {
            format!("{n} / {p}")
        }
    }

    /// Nama singkat untuk bilah transfer: nama folder terakhir di komputer, atau "Akun / folder" di cloud.
    pub(crate) fn nama_pendek(&self, j: &str) -> String {
        if adalah_lokal(j) {
            return Path::new(j).file_name().map(|n| format!("\"{}\"", n.to_string_lossy())).unwrap_or_else(|| j.to_string());
        }
        self.nama_jalur(j)
    }

    pub(crate) fn lokasi_tujuan_bawaan(&self) -> String {
        let siap = || self.akun.iter().filter(|a| a.siap);
        siap().find(|a| a.jenis == "google").or_else(|| siap().next()).map(|a| a.nama.clone()).unwrap_or_default()
    }

    /// Saran folder sumber: folder OneDrive yang tersinkron di komputer ini.
    pub(crate) fn isi_saran_sumber(&mut self) {
        self.saran_sumber.clear();
        self.jenis_sumber.clear();
        if !self.lok_sumber.is_empty() {
            return;
        }
        for o in daftar_onedrive() {
            let j = match o.jenis.as_str() {
                "Pribadi" => "OneDrive pribadi",
                "Kantor/Sekolah" => "OneDrive kantor/sekolah",
                _ => "Folder OneDrive",
            };
            self.jenis_sumber.insert(o.folder.clone(), j.into());
            self.saran_sumber.push((o.folder, j.into()));
        }
    }

    pub(crate) fn set_tujuan_bawaan(&mut self) {
        let s = self.jalur_sumber();
        let rem_t = self.lok_tujuan.clone();
        self.isi_tujuan = if s.is_empty() {
            String::new()
        } else {
            let b = tujuan_bawaan(&s, &rem_t);
            if rem_t.is_empty() {
                b
            } else {
                pecah(&b).1
            }
        };
        self.tujuan_manual = false;
        self.tujuan_berubah();
    }

    /// Muat rute tersimpan (atau bawaan) ke isian saat aplikasi dibuka.
    pub(crate) fn isi_rute(&mut self) {
        let r = rute::baca_pengaturan();
        let sumber = r.sumber.as_deref().map(pecah).filter(|(rem, _)| self.lokasi_ada(rem));
        let tujuan = r.tujuan.as_deref().map(pecah).filter(|(rem, _)| self.lokasi_ada(rem));
        match sumber {
            Some((rem, p)) => {
                self.lok_sumber = rem;
                self.isi_saran_sumber();
                self.isi_sumber = p;
            }
            _ => {
                self.lok_sumber.clear();
                self.isi_saran_sumber();
                self.isi_sumber = self.saran_sumber.first().map(|s| s.0.clone()).unwrap_or_default();
            }
        }
        match tujuan {
            Some((rem, p)) => {
                self.lok_tujuan = rem.clone();
                self.isi_tujuan = p;
                self.tujuan_manual = r.sumber.as_deref().map(|s| tujuan_bawaan(s, &rem)) != r.tujuan;
            }
            _ => {
                self.lok_tujuan = self.lokasi_tujuan_bawaan();
                self.tujuan_manual = false;
            }
        }
        self.rute_siap = true;
        self.sumber_berubah();
        self.tujuan_berubah();
    }
}
