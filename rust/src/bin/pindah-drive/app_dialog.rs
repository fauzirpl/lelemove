//! Jendela utama, bagian `impl App`: konfirmasi dan hasil dialog (lanjutan riwayat & pindahkan).
use super::*;

impl App {
    pub(crate) fn konfirmasi(&mut self, k: Konfirmasi) {
        let akun_ok = |ok: &str, gagal: &str| Setelah::Akun { ok: ok.into(), gagal: gagal.into() };
        match k {
            Konfirmasi::PasangRclone => self.jalankan_aksi("Memasang rclone", v!["pasang", "-Ya"], Setelah::Pasang),
            Konfirmasi::Ulang(n) => self.jalankan_aksi(
                "Menunggu login di browser",
                v!["hubungkan", "-Akun", n, "-Ulang", "-Ya"],
                akun_ok("Login ulang berhasil.", "Login ulang tidak selesai."),
            ),
            Konfirmasi::Penuh(n) => self.jalankan_aksi(
                "Menunggu login Google di browser",
                v!["hubungkan", "-Akun", n, "-IzinPenuh", "-Ya"],
                akun_ok("Izin penuh diberikan.", "Login tidak selesai."),
            ),
            Konfirmasi::Bawaan(n) => self.jalankan_aksi(
                "Menyimpan client ID aplikasi",
                v!["bawaan", "-Akun", n],
                akun_ok("Client ID aplikasi tersimpan.", "Client ID aplikasi gagal disimpan. Lihat log aktivitas."),
            ),
            Konfirmasi::HapusClient => {
                let ada = akun::hapus_client_pengguna();
                self.tulis_log("Client ID aplikasi yang disimpan di komputer ini dihapus.");
                self.segarkan_status();
                let pesan = if ada { "Client ID aplikasi dihapus." } else { "Tidak ada client ID yang disimpan di komputer ini." };
                self.buka_pengaturan(Some((pesan.into(), REDUP)));
            }
            Konfirmasi::HapusJadwal(id) => {
                self.aksi_jadwal("Menghapus jadwal", v!["jadwal", "-Id", id, "-Hapus"], "Jadwal dihapus.")
            }
            Konfirmasi::Pindahkan1 => {
                let Some((n, _, _, _)) = self.bisa_pindahkan.clone() else { return };
                let s = self.jalur_sumber();
                self.dialog = Some(Dialog::Ketik(Ketik {
                    judul: "Konfirmasi terakhir".into(),
                    isi: format!(
                        "Sekitar {} file akan dihapus dari:\n    {}\n\nYang dihapus masuk ke {}.",
                        angka(n),
                        self.nama_jalur(&s),
                        tempat_sampah(&s)
                    ),
                    kata: "HAPUS".into(),
                    ya: "Hapus dari sumber".into(),
                    isian: String::new(),
                    tujuan: Konfirmasi::Pindahkan2,
                    fokus: true,
                }));
            }
            Konfirmasi::Pindahkan2 => {
                if self.ambil_isian().is_none() {
                    return;
                }
                self.verifikasi = (format!("Memverifikasi ulang, lalu menghapus dari sumber{ELIPSIS}"), REDUP);
                self.jalankan_aksi("Memindahkan: verifikasi ulang lalu hapus dari sumber", v!["pindahkan", "-Ya"], Setelah::Pindahkan);
            }
            Konfirmasi::Lepas(n) => self.jalankan_aksi(
                "Melepas akun",
                v!["lepas", "-Akun", n, "-Ya"],
                akun_ok("Akun dilepas.", "Akun gagal dilepas. Lihat log aktivitas."),
            ),
            Konfirmasi::MulaiSalin => {
                self.verifikasi = kosong();
                self.jalankan_aksi(
                    "Memulai penyalinan",
                    v!["salin", "-Latar", "-Ya", "-BatasKecepatan", &self.batas],
                    Setelah::Salin,
                );
            }
            Konfirmasi::Henti => {
                Mesin::baru(Opsi::default()).berhenti();
                self.tulis_log("Penyalinan dihentikan.");
                self.set_pesan("Penyalinan dihentikan.", REDUP);
                self.perbarui_progres();
            }
            Konfirmasi::TutupApp => {
                self.klik_batal();
                self.boleh_tutup = true;
                self.ctx.send_viewport_cmd(ViewportCommand::Close);
            }
        }
    }

    pub(crate) fn tangani_dialog(&mut self, h: Hasil) {
        match h {
            Hasil::Batal => {}
            Hasil::EmailMulai { periksa, args, rahasia } => {
                let kerja = self.email_mut().map(|e| e.nama_kerja()).unwrap_or("Migrasi email");
                let judul = if periksa { format!("Memeriksa {}", kerja.to_lowercase()) } else { kerja.to_string() };
                self.jalankan_aksi_masukan(&judul, args, Some(rahasia), Setelah::Email { periksa });
                let mulai = self.aksi.is_some();
                if let Some(e) = self.email_mut() {
                    if mulai {
                        e.tahap = email::Tahap::Jalan { periksa };
                        e.progres = None;
                        e.hasil = None;
                        e.baris = String::new();
                        e.pesan = None;
                    } else {
                        e.pesan = Some((format!("{MESIN} tidak bisa dijalankan; lihat log aktivitas."), MERAH));
                    }
                }
            }
            Hasil::EmailHenti => self.klik_batal(),
            Hasil::PembaruanCek => {
                self.set_pembaruan(info::Pembaruan::Memeriksa);
                self.jalankan_aksi("Memeriksa pembaruan", vec!["pembaruan".into()], Setelah::Pembaruan { pasang: false });
                if self.aksi.is_none() {
                    self.set_pembaruan(info::Pembaruan::Galat(format!("{MESIN} tidak bisa dijalankan; lihat log aktivitas.")));
                }
            }
            Hasil::PembaruanPasang => {
                let versi = match &self.pembaruan {
                    info::Pembaruan::Ada { versi, .. } => versi.clone(),
                    _ => String::new(),
                };
                if self.job_jalan {
                    self.set_pembaruan(info::Pembaruan::Galat("Penyalinan sedang berjalan; perbarui sesudah selesai.".into()));
                } else {
                    self.set_pembaruan(info::Pembaruan::Mengunduh { versi, baris: String::new() });
                    self.jalankan_aksi("Memperbarui LELEMOVE", vec!["pembaruan".into(), "-Jalankan".into()], Setelah::Pembaruan { pasang: true });
                    if self.aksi.is_none() {
                        self.set_pembaruan(info::Pembaruan::Galat(format!("{MESIN} tidak bisa dijalankan; lihat log aktivitas.")));
                    }
                }
            }
            Hasil::BatalSharePoint => self.set_pesan("Situs SharePoint belum dipilih; akun ini belum bisa dipakai.", KUNING),
            Hasil::Ya(k) => self.konfirmasi(k),
            Hasil::AkunBaru { jenis, label, izin_penuh } => self.hubungkan_baru(&jenis, &label, izin_penuh),
            Hasil::ClientId { id, secret } => {
                // Disimpan langsung oleh jendela (secret tidak lewat baris perintah).
                akun::simpan_client_bawaan(&id, &secret);
                self.tulis_log("Client ID aplikasi disimpan.");
                self.ada_client_bawaan = true;
                self.segarkan_status();
                self.buka_pengaturan(Some((
                    "Client ID aplikasi tersimpan. Akun Google baru otomatis memakainya; akun yang sudah ada perlu login ulang lewat tombol di bawah.".into(),
                    HIJAU,
                )));
            }
            Hasil::KembaliPengaturan => self.buka_pengaturan(None),
            Hasil::GantiClient => self.dialog = Some(Dialog::ClientId(ClientId::baru())),
            Hasil::HapusClient => self.tanya(
                "Hapus client ID aplikasi",
                "Hapus client ID Google yang disimpan di komputer ini?\n\nAkun Google yang sudah terhubung tetap bisa dipakai. Akun Google baru akan memakai client ID bawaan installer (bila ada) atau client ID bersama rclone.".into(),
                "Hapus",
                true,
                Konfirmasi::HapusClient,
            ),
            Hasil::LoginClient(nama) => self.jalankan_aksi(
                "Menunggu login Google di browser",
                v!["hubungkan", "-Akun", nama, "-ClientAplikasi", "-Ya"],
                Setelah::Akun {
                    ok: "Akun Google kini memakai client ID aplikasi.".into(),
                    gagal: "Login tidak selesai. Coba lagi lewat Pengaturan.".into(),
                },
            ),
            Hasil::LanjutOtomatis(aktif) => {
                rute::simpan_lanjut_otomatis(aktif);
                self.tulis_log(if aktif {
                    "Lanjut otomatis saat jaringan putus: aktif."
                } else {
                    "Lanjut otomatis saat jaringan putus: nonaktif."
                });
                // Dialog tetap terbuka; hanya pesannya yang diperbarui.
                let pesan = if aktif {
                    "Berlaku untuk penyalinan berikutnya (atau saat Lanjutkan)."
                } else {
                    "Nonaktif: penyalinan yang putus karena jaringan berhenti dan menunggu Anda klik Lanjutkan."
                };
                self.buka_pengaturan(Some((pesan.into(), REDUP)));
            }
            Hasil::AmbilClient(nama) => {
                let label = nama_lokasi(self.akun_dari(&nama).as_ref());
                self.tanya(
                    "Client ID aplikasi",
                    format!("Jadikan client ID milik {label} sebagai client ID aplikasi?\n\nClient ID ini akan dipakai untuk akun Google yang ditambahkan berikutnya, dan ikut dibawa installer yang dibangun sesudahnya. Siapa pun yang memegang installer itu bisa membaca client ID dan secret-nya."),
                    "Jadikan client ID aplikasi",
                    false,
                    Konfirmasi::Bawaan(nama),
                );
            }
            Hasil::KembaliJadwal => self.buka_jadwal(None),
            Hasil::KembaliPemilih => self.buka_pemilih(),
            Hasil::PilihPenyedia(k) => match mp::cari(&k) {
                Some(p) if p.khusus => self.klik_tambah(p.kunci),
                Some(_) => self.buka_kredensial(&k, None),
                None => {}
            },
            Hasil::BukaImpor => self.buka_impor(),
            Hasil::RcloneConfig => self.buka_rclone_config(),
            Hasil::Kredensial { kunci, akun, label, opsi, rahasia } => {
                // Disimpan untuk dibuka lagi (dengan pesan galat) bila login gagal.
                if let Some(p) = mp::cari(&kunci) {
                    let mut k = layanan::Kredensial::baru(p, akun.clone(), label.clone());
                    for (i, c) in p.kolom.iter().enumerate() {
                        if let Some((_, v)) = opsi.iter().chain(rahasia.iter()).find(|(x, _)| x == c.kunci) {
                            k.nilai[i] = v.clone();
                        }
                    }
                    k.fokus = false;
                    self.kredensial_terakhir = Some(k);
                }
                self.hubungkan_kredensial(&kunci, akun, &label, opsi, rahasia);
            }
            Hasil::JawabKonfig { akun: nama, state, jawab, rahasia, baru } => {
                let kunci = akun::jenis_dari_nama(&nama).to_string();
                let masukan = serde_json::json!({ "Jawab": jawab, "Rahasia": rahasia, "Baru": baru }).to_string();
                self.jalankan_aksi_masukan(
                    "Melanjutkan login",
                    v!["hubungkan", "-Akun", nama, "-Lanjut", "-State", state, "-RahasiaStdin"],
                    Some(masukan),
                    Setelah::AkunPenyedia { nama, kunci },
                );
            }
            Hasil::BatalKonfig { akun: nama, baru } => {
                if baru {
                    // Akun setengah jadi dihapus lagi (hanya entri rclone.conf, bukan file).
                    self.jalankan_aksi(
                        "Membatalkan akun baru",
                        v!["lepas", "-Akun", nama, "-Ya"],
                        Setelah::Akun { ok: "Penambahan akun dibatalkan.".into(), gagal: "Akun setengah jadi belum terhapus. Lihat log aktivitas.".into() },
                    );
                }
            }
            Hasil::Impor { nama, label } => {
                let mut a = v!["impor", "-Akun", nama];
                if !label.is_empty() {
                    a.extend(v!["-Label", label]);
                }
                self.jalankan_aksi(
                    "Memakai remote rclone",
                    a,
                    Setelah::Akun { ok: "Remote rclone dipakai sebagai akun.".into(), gagal: "Remote belum bisa dipakai. Lihat log aktivitas.".into() },
                );
            }
            Hasil::JadwalBaru => {
                let kini = self.rute_kini();
                let nama = kini.as_ref().map(|r| self.nama_jadwal_bawaan(&r.sumber)).unwrap_or_default();
                self.dialog = Some(Dialog::JadwalIsian(jadwal::Isian::baru(kini, nama, self.filter.clone())));
            }
            Hasil::JadwalUbah(id) => match mj::cari(&id) {
                Some(j) => {
                    let rute = self.rute_jadwal(&j.sumber, &j.tujuan);
                    self.dialog = Some(Dialog::JadwalIsian(jadwal::Isian::dari(&j, rute, self.rute_kini(), self.filter.clone())));
                }
                None => self.buka_jadwal(Some(("Jadwal itu sudah tidak ada.".into(), KUNING))),
            },
            Hasil::JadwalSimpan { id, nama, sumber, tujuan, jam, hari, verifikasi, filter } => {
                let mut a = v!["jadwal", "-Tambah", "-Nama", nama, "-Sumber", sumber, "-Tujuan", tujuan, "-Jam", jam, "-Hari", hari];
                if !verifikasi {
                    a.push("-TanpaVerifikasi".into());
                }
                if !filter.kosong() {
                    a.extend(v!["-Sertakan", filter.teks_sertakan(), "-Lewati", filter.teks_lewati()]);
                    if let Some(t) = &filter.sejak {
                        a.extend(v!["-Sejak", t]);
                    }
                }
                let baru = id.is_none();
                if let Some(id) = id {
                    a.extend(v!["-Id", id]);
                }
                let ok = if baru { "Jadwal dipasang." } else { "Jadwal diperbarui." };
                self.aksi_jadwal("Menyimpan jadwal", a, ok);
            }
            Hasil::JadwalJalankan(id) => {
                // Berjalan lepas dari jendela (seperti tugas Windows), progresnya dibaca dari job.json.
                match win::mulai_terlepas(&self.exe_mesin, &v!["jadwal", "-Jalankan", "-Id", id]) {
                    Ok(_) => {
                        self.tulis_log("--- Backup terjadwal dijalankan sekarang");
                        self.set_pesan("Backup terjadwal berjalan di latar belakang.", HIJAU);
                        self.verifikasi = kosong();
                    }
                    Err(e) => self.buka_jadwal(Some((format!("{MESIN} tidak bisa dijalankan: {e}"), MERAH))),
                }
            }
            Hasil::JadwalAktif(id, aktif) => {
                let (judul, saklar, ok) = if aktif {
                    ("Mengaktifkan jadwal", "-Aktifkan", "Jadwal aktif.")
                } else {
                    ("Menonaktifkan jadwal", "-Nonaktifkan", if cfg!(windows) { "Jadwal nonaktif; tugas Windows-nya tidak berjalan sampai diaktifkan lagi." } else { "Jadwal nonaktif; tugas launchd-nya tidak berjalan sampai diaktifkan lagi." })
                };
                self.aksi_jadwal(judul, v!["jadwal", "-Id", id, saklar], ok);
            }
            Hasil::JadwalHapus(id) => {
                let nama = mj::cari(&id).map(|j| j.nama).unwrap_or_else(|| id.clone());
                self.tanya(
                    "Hapus jadwal",
                    format!("Hapus jadwal \"{nama}\"?\n\n{}-nya ikut dihapus. File yang sudah dicadangkan di tujuan TIDAK terhapus.", kapital(win::NAMA_TUGAS)),
                    "Hapus jadwal",
                    true,
                    Konfirmasi::HapusJadwal(id),
                );
            }
            Hasil::JadwalLog(log) => {
                win::buka(&log);
                self.buka_jadwal(None);
            }
            Hasil::FilterSimpan(f) => {
                mf::simpan_filter(&f);
                self.tulis_log(&if f.kosong() { "Filter file dihapus.".to_string() } else { format!("Filter file: {}", f.uraian()) });
                self.filter = f;
                // Hasil periksa, uji coba, dan verifikasi sebelumnya dihitung tanpa filter ini.
                self.periksa = None;
                self.ringkas_uji = None;
                self.sudah_diperiksa = false;
                self.verifikasi = kosong();
                self.bisa_pindahkan = None;
                self.set_pesan(if self.filter.kosong() { "Filter dihapus: seluruh isi folder disalin." } else { "Filter disimpan." }, HIJAU);
            }
            Hasil::RiwayatLog(log, saring) => {
                win::buka(&log);
                self.buka_riwayat(saring, None);
            }
            Hasil::RiwayatLaporan(id, saring) => {
                let pesan = match mr::baca(0).into_iter().find(|c| c.id == id) {
                    Some(c) => {
                        let p = jalur().log.join(format!("laporan-{}-{id}.html", c.jenis));
                        match std::fs::write(&p, mr::html_verifikasi(&c)) {
                            Ok(()) => {
                                win::buka(&p.to_string_lossy());
                                (format!("Laporan dibuka di browser. Tekan {} untuk menyimpannya sebagai PDF.", win::PINTASAN_CETAK), HIJAU)
                            }
                            Err(e) => (format!("Laporan tidak bisa dibuat: {e}"), MERAH),
                        }
                    }
                    None => ("Catatan riwayat itu tidak ditemukan.".to_string(), MERAH),
                };
                self.buka_riwayat(saring, Some(pesan));
            }
            Hasil::RiwayatCsvFile(id, saring) => {
                let pesan = match mr::baca(0).into_iter().find(|c| c.id == id) {
                    Some(c) => pesan_ekspor(simpan_ekspor(&format!("verifikasi-{}.csv", c.id), "CSV", &mr::csv_per_file(&c)), false),
                    None => Some(("Catatan riwayat itu tidak ditemukan.".to_string(), MERAH)),
                };
                self.buka_riwayat(saring, pesan);
            }
            Hasil::RiwayatEkspor { html, daftar, saring } => {
                let (y, mo, d, _, _, _) = waktu_lokal();
                let pesan = if html {
                    let isi = mr::html_riwayat(&daftar);
                    pesan_ekspor(simpan_ekspor(&format!("laporan-lelemove-{y:04}{mo:02}{d:02}.html"), "HTML", &isi), true)
                } else {
                    pesan_ekspor(simpan_ekspor(&format!("riwayat-lelemove-{y:04}{mo:02}{d:02}.csv"), "CSV", &mr::csv(&daftar)), false)
                };
                self.buka_riwayat(saring, pesan);
            }
            Hasil::Folder { jalur, untuk_sumber } => {
                if untuk_sumber {
                    self.isi_sumber = jalur;
                    self.sumber_berubah();
                } else {
                    self.isi_tujuan = jalur;
                    self.tujuan_manual = true;
                    self.sudah_diperiksa = false;
                    self.tujuan_berubah();
                }
            }
            Hasil::Pustaka { remote, situs_nama, pustaka_id, pustaka_nama } => self.jalankan_aksi(
                "Memakai pustaka SharePoint",
                v!["pustaka", "-Akun", remote, "-Pustaka", pustaka_id, "-SitusNama", situs_nama, "-PustakaNama", pustaka_nama],
                Setelah::Akun {
                    ok: "Pustaka SharePoint siap dipakai sebagai sumber atau tujuan.".into(),
                    gagal: "Pustaka gagal dipakai. Lihat log aktivitas.".into(),
                },
            ),
        }
    }
}
