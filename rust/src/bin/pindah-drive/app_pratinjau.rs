//! Jendela utama, bagian `impl App`: pratinjau.
use super::*;

impl App {
    // -------------------------------------------------------------- pratinjau

    pub(crate) fn buka_dialog_pratinjau(&mut self, d: &str) {
        match d {
            "akun-google" | "akun-onedrive" | "akun-sharepoint" => {
                let j = d.trim_start_matches("akun-").to_string();
                self.dialog = Some(Dialog::AkunBaru(AkunBaru {
                    client_bawaan: akun::baca_client_bawaan().is_some(),
                    jenis: j,
                    label: String::new(),
                    juga_sumber: false,
                    fokus: true,
                }));
            }
            "client" => self.dialog = Some(Dialog::ClientId(ClientId::baru())),
            // Isinya diperbarui lagi begitu daftar akun selesai dimuat (terima_status).
            "pengaturan" => self.buka_pengaturan(None),
            "jadwal" => self.buka_jadwal(None),
            "jadwal-baru" => self.tangani_dialog(Hasil::JadwalBaru),
            "filter" => self.dialog = Some(Dialog::Filter(filter::FilterIsian::dari(&self.filter))),
            "layanan" => self.buka_pemilih(),
            "impor" => self.buka_impor(),
            _ if d.starts_with("kredensial:") => self.buka_kredensial(&d["kredensial:".len()..], None),
            "tanya-konfig" => {
                self.dialog = Some(Dialog::TanyaKonfig(layanan::TanyaKonfig {
                    akun: "pd-icloud".into(),
                    layanan: "iCloud Drive".into(),
                    state: "2fa".into(),
                    bantuan: "Two-factor authentication: please enter your 2FA code".into(),
                    rahasia: false,
                    pilihan: Vec::new(),
                    jawab: String::new(),
                    baru: true,
                    galat: None,
                    fokus: true,
                }));
            }
            "riwayat" => self.buka_riwayat(riwayat::Saring::Semua, None),
            "email" => self.buka_email(email::Mode::Migrasi),
            "email-cadangan" => self.buka_email(email::Mode::Cadangkan),
            "email-pulihkan" => {
                self.buka_email(email::Mode::Cadangkan);
                if let Some(Dialog::Email(e)) = &mut self.dialog {
                    e.mode = email::Mode::Pulihkan;
                }
            }
            "pembaruan" => {
                self.pembaruan = info::Pembaruan::Ada {
                    versi: "2.5.0".into(),
                    halaman: "https://github.com/fauzirpl/lelemove/releases/tag/v2.5.0".into(),
                    catatan: String::new(),
                };
                self.buka_info(info::Tab::Tentang);
            }
            "pembaruan-unduh" => {
                self.pembaruan = info::Pembaruan::Mengunduh { versi: "2.5.0".into(), baris: "Mengunduh 23,9 MB dari 39,9 MB (60%)".into() };
                self.buka_info(info::Tab::Tentang);
            }
            "email-hasil" => {
                self.buka_email(email::Mode::Migrasi);
                if let Some(Dialog::Email(e)) = &mut self.dialog {
                    e.hasil = Some(email::contoh_hasil());
                    e.tahap = email::Tahap::Selesai { periksa: false, kode: 0 };
                }
            }
            "pindahkan" => {
                self.bisa_pindahkan = Some(rute_contoh(128, 2));
                self.verifikasi = (format!("{CENTANG}  Verifikasi: semua 128 file sudah ada di tujuan (contoh)."), HIJAU);
                self.konfirmasi(Konfirmasi::Pindahkan1);
            }
            "pindahkan-tautan" => {
                self.bisa_pindahkan = Some(rute_contoh(3, 0));
                self.verifikasi = (format!("{CENTANG}  Verifikasi: semua 3 file sudah ada di tujuan (contoh)."), HIJAU);
            }
            "pindahkan-tanya" => {
                self.bisa_pindahkan = Some(rute_contoh(128, 2));
                self.verifikasi = (format!("{CENTANG}  Verifikasi: 128 cocok, 2 belum ada di tujuan (contoh)."), KUNING);
                self.klik_pindahkan();
            }
            _ if d.starts_with("info") => self.buka_info(info::Tab::dari_nama(d.trim_start_matches("info").trim_start_matches(':'))),
            "tanya" => self.tanya("Lepas akun", "Lepas Google Drive dari LELEMOVE?\n\nFile di akun tersebut TIDAK terhapus; hanya sambungan di komputer ini yang dihapus.".into(), "Lepas akun", true, Konfirmasi::Lepas("pd-x".into())),
            _ if d.starts_with("jelajah:") => {
                let mut p = Pemuat { tx: &self.tx, ctx: &self.ctx, gen: &mut self.gen };
                let j = Jelajah::baru(&d[8..], "", "Pilih folder SUMBER di contoh akun".into(), true, &mut p);
                self.dialog = Some(Dialog::Jelajah(j));
            }
            _ if d.starts_with("sharepoint:") => {
                let bag: Vec<&str> = d.splitn(3, ':').collect();
                let mut s = SharePoint::baru(bag.get(1).copied().unwrap_or(""));
                s.cari = bag.get(2).copied().unwrap_or("").to_string();
                let mut p = Pemuat { tx: &self.tx, ctx: &self.ctx, gen: &mut self.gen };
                s.cari(&mut p);
                self.dialog = Some(Dialog::SharePoint(s));
            }
            _ => {}
        }
    }

    pub(crate) fn urus_pratinjau(&mut self, ctx: &egui::Context) {
        // Tunggu daftar folder kedua panel termuat (paling lama 15 detik lebih dari --tunda).
        let siap = self.status_siap && self.aksi.is_none() && !self.panel_memuat();
        let Some(p) = &mut self.pratinjau else { return };
        let lewat = p.mulai.elapsed();
        if !p.diminta && ((siap && lewat >= p.tunda) || lewat >= p.tunda + Duration::from_secs(15)) {
            ctx.send_viewport_cmd(ViewportCommand::Screenshot(Default::default()));
            p.diminta = true;
        }
        let gambar = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(img) = gambar {
            let mut buf = Vec::with_capacity(img.pixels.len() * 4);
            for c in &img.pixels {
                buf.extend_from_slice(&c.to_array());
            }
            if let Some(g) = image::RgbaImage::from_raw(img.size[0] as u32, img.size[1] as u32, buf) {
                let _ = g.save(&p.berkas);
            }
            self.boleh_tutup = true;
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
    }
}
