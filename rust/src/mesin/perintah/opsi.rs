use super::super::penyedia;
use super::super::rute::*;
use super::PERINTAH;

// ------------------------------------------------------------------ opsi

#[derive(Clone, Debug)]
pub struct Opsi {
    pub perintah: String,
    /// Folder di komputer (D:\OneDrive\Backup) atau "<akun>:<folder>" (pd-onedrive:Backup).
    pub sumber: Option<String>,
    /// Sama dengan sumber; tanpa nama akun dianggap folder di akun Google utama (pd-gdrive).
    pub tujuan: Option<String>,
    /// Akun untuk hubungkan/lepas/situs/pustaka. Alias: google, onedrive, sharepoint.
    pub akun: String,
    pub jenis: String,
    pub label: Option<String>,
    /// Google: izin baca-tulis penuh, wajib bila akun ini dipakai sebagai SUMBER.
    pub izin_penuh: bool,
    /// Salin di latar belakang (jendela boleh ditutup).
    pub latar: bool,
    /// Lewati pertanyaan konfirmasi (persetujuan sudah diberikan).
    pub ya: bool,
    /// Paksa login ulang.
    pub ulang: bool,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub simpan_bawaan: bool,
    /// hubungkan (Google): login memakai client ID aplikasi (Pengaturan > Client ID Google).
    pub client_aplikasi: bool,
    /// klien: hapus client ID aplikasi yang disimpan di komputer ini. jadwal: hapus jadwal -Id.
    pub hapus: bool,
    /// jadwal: -Tambah (jadwal baru), -Jalankan (dipakai tugas Windows), -Aktifkan / -Nonaktifkan.
    pub tambah: bool,
    pub jalankan: bool,
    /// pembaruan: unduh & cek berkas rilis (tanpa menjalankannya).
    pub unduh: bool,
    pub aktifkan: bool,
    pub nonaktifkan: bool,
    pub tanpa_verifikasi: bool,
    pub id: Option<String>,
    pub nama: Option<String>,
    pub jam: Option<String>,
    pub hari: Option<String>,
    pub cari: Option<String>,
    pub situs_id: Option<String>,
    pub situs_nama: Option<String>,
    pub pustaka: Option<String>,
    pub pustaka_nama: Option<String>,
    /// Untuk jendela aplikasi: cetak ringkasan hasil sebagai "##HASIL {json}".
    pub mesin: bool,
    /// Internal: proses latar belakang yang menjalankan rclone.
    pub log_latar: Option<String>,
    /// Batas kecepatan salin dalam bentuk --bwlimit rclone ("5M"); "off" = tanpa batas.
    /// None: pakai yang tersimpan di pengaturan.json.
    pub batas_kecepatan: Option<String>,
    /// Filter file: -Sertakan "pdf,docx", -Lewati "*.tmp", -Sejak 2026-01-31, -TanpaFilter.
    /// None = pakai yang tersimpan; diberikan = disimpan untuk perintah berikutnya.
    pub sertakan: Option<String>,
    pub lewati: Option<String>,
    pub sejak: Option<String>,
    pub tanpa_filter: bool,
    /// riwayat: -Jumlah N, -Csv <berkas>, -Laporan <berkas.html>.
    pub jumlah: Option<String>,
    pub csv: Option<String>,
    pub laporan: Option<String>,
    /// hubungkan (penyedia selain Google/Microsoft): -Opsi kunci=nilai (boleh berulang) untuk isian
    /// yang tidak rahasia; -RahasiaStdin = isian rahasia dikirim sebagai JSON lewat stdin.
    pub opsi_rclone: Vec<(String, String)>,
    pub rahasia_stdin: bool,
    /// hubungkan -Lanjut -State S [-Jawab X]: menjawab pertanyaan lanjutan rclone (mis. kode 2FA).
    pub lanjut: bool,
    pub state: Option<String>,
    pub jawab: Option<String>,
    /// email: -Login sandi|google (Gmail), -SertakanSampah, -TanpaArsip, -AbaikanSertifikat,
    /// -HanyaPeriksa, -ClientMicrosoft <client ID Azure>.
    pub login: Option<String>,
    pub sertakan_sampah: bool,
    pub tanpa_arsip: bool,
    pub abaikan_sertifikat: bool,
    pub hanya_periksa: bool,
    pub client_microsoft: Option<String>,
}

impl Default for Opsi {
    fn default() -> Self {
        Opsi {
            perintah: "menu".into(),
            sumber: None,
            tujuan: None,
            akun: "google".into(),
            jenis: String::new(),
            label: None,
            izin_penuh: false,
            latar: false,
            ya: false,
            ulang: false,
            client_id: None,
            client_secret: None,
            simpan_bawaan: false,
            client_aplikasi: false,
            hapus: false,
            tambah: false,
            jalankan: false,
            unduh: false,
            aktifkan: false,
            nonaktifkan: false,
            tanpa_verifikasi: false,
            id: None,
            nama: None,
            jam: None,
            hari: None,
            cari: None,
            situs_id: None,
            situs_nama: None,
            pustaka: None,
            pustaka_nama: None,
            mesin: false,
            log_latar: None,
            batas_kecepatan: None,
            sertakan: None,
            lewati: None,
            sejak: None,
            tanpa_filter: false,
            jumlah: None,
            csv: None,
            laporan: None,
            opsi_rclone: Vec::new(),
            rahasia_stdin: false,
            lanjut: false,
            state: None,
            jawab: None,
            login: None,
            sertakan_sampah: false,
            tanpa_arsip: false,
            abaikan_sertifikat: false,
            hanya_periksa: false,
            client_microsoft: None,
        }
    }
}

/// Argumen bergaya PowerShell (-Sumber X, -Ya), tanpa membedakan huruf besar-kecil;
/// --sumber X juga diterima.
pub fn urai_argumen<I: IntoIterator<Item = String>>(args: I) -> Result<Opsi, String> {
    let mut o = Opsi::default();
    let mut it = args.into_iter();
    let mut ada_perintah = false;
    while let Some(a) = it.next() {
        if a.starts_with('-') && a.len() > 1 {
            let mentah = a.trim_start_matches('-').to_lowercase();
            let (nama, sisip) = match mentah.split_once(':') {
                Some((n, v)) => (n.to_string(), Some(v.to_string())),
                None => (mentah, None),
            };
            match nama.as_str() {
                "izinpenuh" => o.izin_penuh = true,
                "latar" => o.latar = true,
                "ya" | "y" => o.ya = true,
                "ulang" => o.ulang = true,
                "simpanbawaan" => o.simpan_bawaan = true,
                "clientaplikasi" => o.client_aplikasi = true,
                "hapus" => o.hapus = true,
                "tambah" => o.tambah = true,
                "jalankan" => o.jalankan = true,
                "unduh" => o.unduh = true,
                "aktifkan" => o.aktifkan = true,
                "nonaktifkan" => o.nonaktifkan = true,
                "tanpaverifikasi" => o.tanpa_verifikasi = true,
                "tanpafilter" => o.tanpa_filter = true,
                "rahasiastdin" => o.rahasia_stdin = true,
                "lanjut" => o.lanjut = true,
                "sertakansampah" => o.sertakan_sampah = true,
                "tanpaarsip" => o.tanpa_arsip = true,
                "abaikansertifikat" => o.abaikan_sertifikat = true,
                "hanyaperiksa" => o.hanya_periksa = true,
                "mesin" => o.mesin = true,
                "h" | "help" | "?" | "bantuan" => o.perintah = "bantuan".into(),
                _ => {
                    let nilai = match sisip {
                        Some(v) => v,
                        None => it.next().ok_or_else(|| format!("Opsi {a} butuh nilai."))?,
                    };
                    let isi = Some(nilai.clone()).filter(|s| !s.is_empty());
                    match nama.as_str() {
                        "perintah" => o.perintah = nilai.to_lowercase(),
                        "sumber" => o.sumber = isi,
                        "tujuan" => o.tujuan = isi,
                        "akun" => o.akun = isi.unwrap_or_else(|| "google".into()),
                        "jenis" => o.jenis = nilai.to_lowercase(),
                        "label" => o.label = isi,
                        "clientid" => o.client_id = isi,
                        "clientsecret" => o.client_secret = isi,
                        "cari" => o.cari = isi,
                        "situsid" => o.situs_id = isi,
                        "situsnama" => o.situs_nama = isi,
                        "pustaka" => o.pustaka = isi,
                        "pustakanama" => o.pustaka_nama = isi,
                        "loglatar" => o.log_latar = isi,
                        "id" => o.id = isi,
                        "nama" => o.nama = isi,
                        "jam" => o.jam = isi,
                        "hari" => o.hari = isi,
                        "bataskecepatan" => o.batas_kecepatan = Some(normalisasi_batas(&nilai)?),
                        // Boleh kosong: -Sertakan "" menghapus bagian itu dari filter tersimpan.
                        "sertakan" => o.sertakan = Some(nilai),
                        "lewati" => o.lewati = Some(nilai),
                        "sejak" => o.sejak = Some(nilai),
                        "jumlah" => o.jumlah = isi,
                        "csv" => o.csv = isi,
                        "laporan" => o.laporan = isi,
                        "opsi" => {
                            let (k, v) = nilai
                                .split_once('=')
                                .ok_or_else(|| format!("-Opsi harus berbentuk kunci=nilai: {nilai}"))?;
                            o.opsi_rclone.push((k.trim().to_string(), v.to_string()));
                        }
                        "state" => o.state = isi,
                        "jawab" => o.jawab = Some(nilai),
                        "login" => o.login = Some(nilai.to_lowercase()),
                        "clientmicrosoft" => o.client_microsoft = isi,
                        _ => return Err(format!("Opsi tidak dikenal: {a}")),
                    }
                }
            }
        } else if !ada_perintah {
            o.perintah = a.to_lowercase();
            ada_perintah = true;
        } else {
            return Err(format!("Argumen tidak dikenal: {a}"));
        }
    }
    if !PERINTAH.contains(&o.perintah.as_str()) {
        return Err(format!("Perintah tidak dikenal: {}", o.perintah));
    }
    if !o.jenis.is_empty() && penyedia::cari(&o.jenis).is_none() {
        return Err(format!("Jenis akun tidak dikenal: {}. Lihat daftarnya: pindah penyedia", o.jenis));
    }
    Ok(o)
}
