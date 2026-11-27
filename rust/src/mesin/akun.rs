//! Akun cloud = remote rclone berawalan "pd-": pd-gdrive, pd-gdrive-2, pd-onedrive, pd-dropbox, ...
//! (lihat penyedia.rs), ditambah remote rclone yang sudah ada dan sengaja dipakai ("Impor").
//! Nama tampilan, jenis, drive OneDrive, dan pustaka SharePoint disimpan di akun.json.

use serde::Serialize;
use serde_json::{Map, Value};

use super::dasar::{baca_objek, jalur, teks, tulis_json, REMOTE_GOOGLE, REMOTE_ONEDRIVE, REMOTE_SHAREPOINT};
use super::penyedia;
use super::rclone::{self, daftar_remote, galat_jaringan, panggil, token_kedaluwarsa, Mode};
use super::rute::adalah_lokal;
use crate::v;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Akun {
    pub nama: String,
    /// Kunci penyedia (penyedia.rs): google, onedrive, sharepoint, dropbox, nextcloud, s3, ...
    /// Remote impor dengan backend yang tidak dikenal: nama backend rclone-nya.
    pub jenis: String,
    /// Remote rclone milik pengguna yang dipakai LELEMOVE (bukan dibuat LELEMOVE): "Lepas" hanya
    /// menyembunyikannya, tidak menghapus remote itu.
    pub impor: bool,
    pub label: String,
    pub drive: String,
    pub scope: String,
    /// Google: akun memakai client ID OAuth sendiri (bukan client ID bersama rclone).
    pub client_sendiri: bool,
    /// Google: client ID akun ini sama dengan client ID aplikasi (Pengaturan > Client ID Google).
    pub client_aplikasi: bool,
    pub situs: String,
    pub drive_id: String,
    /// SharePoint tanpa pustaka terpilih masih menunjuk OneDrive pribadi: belum boleh dipakai.
    pub siap: bool,
}

/// Alias: google = pd-gdrive, onedrive = pd-onedrive.
pub fn nama_remote(a: &str) -> String {
    match a {
        "google" => REMOTE_GOOGLE.into(),
        "onedrive" => REMOTE_ONEDRIVE.into(),
        "sharepoint" => REMOTE_SHAREPOINT.into(),
        _ => a.trim().trim_end_matches(':').to_string(),
    }
}

/// "pd-onedrive:Kerja/DLH" -> "pd-onedrive";  "D:\Data" -> ""
pub fn remote_dari(j: &str) -> String {
    if j.is_empty() || adalah_lokal(j) {
        return String::new();
    }
    match j.find(':') {
        Some(i) if i > 0 && !j[..i].contains(['\\', '/']) => j[..i].to_string(),
        _ => String::new(),
    }
}

/// Jenis ditebak dari nama (tanpa memanggil rclone): cukup untuk label, tempat sampah, dan nama
/// folder bawaan. Remote impor: dari akun.json.
pub fn jenis_dari_nama(remote: &str) -> &'static str {
    if let Some(p) = penyedia::dari_nama(remote) {
        return p.kunci;
    }
    if remote.is_empty() {
        return "";
    }
    penyedia::cari(&meta(remote, "Jenis")).map_or("", |p| p.kunci)
}

/// Nama layanan untuk ditampilkan: "Dropbox", "Amazon S3"; remote impor tak dikenal: "Akun".
pub fn nama_layanan(jenis: &str) -> &'static str {
    penyedia::cari(jenis).map_or("Akun", |p| p.nama)
}

pub fn baca_meta_akun() -> Map<String, Value> {
    baca_objek(&jalur().akun).unwrap_or_default()
}

pub fn meta(nama: &str, kunci: &str) -> String {
    teks(baca_meta_akun().get(nama).and_then(|m| m.get(kunci)))
}

/// Menimpa field metadata satu akun (atau menghapus akun itu dari akun.json).
pub fn simpan_meta_akun(nama: &str, ubah: &[(&str, &str)], hapus: bool) {
    let mut semua = baca_meta_akun();
    if hapus {
        semua.remove(nama);
    } else {
        let mut isi = match semua.get(nama) {
            Some(Value::Object(m)) => m.clone(),
            _ => Map::new(),
        };
        for (k, v) in ubah {
            isi.insert((*k).to_string(), Value::String((*v).to_string()));
        }
        semua.insert(nama.to_string(), Value::Object(isi));
    }
    let mut kunci: Vec<String> = semua.keys().cloned().collect();
    kunci.sort();
    let mut urut = Map::new();
    for k in kunci {
        if let Some(v) = semua.remove(&k) {
            urut.insert(k, v);
        }
    }
    tulis_json(&jalur().akun, &Value::Object(urut));
}

pub fn label_bawaan(nama: &str, jenis: &str) -> String {
    let dasar = nama_layanan(jenis);
    if dasar == "Akun" && !nama.starts_with("pd-") {
        // Remote impor yang jenisnya tidak dikenal: pakai namanya sendiri.
        return nama.to_string();
    }
    match nama.rsplit_once('-') {
        Some((_, n)) if !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) => format!("{dasar} {n}"),
        _ => dasar.to_string(),
    }
}

#[derive(Default, Clone)]
pub struct Konfig {
    pub tipe: String,
    pub scope: String,
    pub client_sendiri: bool,
}

/// Pengaturan akun yang tidak rahasia (rclone sendiri menyensor token & secret).
pub fn konfig_akun(nama: &str) -> Konfig {
    let h = panggil(&v!["config", "redacted", format!("{nama}:")], Mode::Tangkap);
    let mut k = Konfig::default();
    for b in h.out.lines() {
        let Some((kunci, nilai)) = b.split_once('=') else { continue };
        let (kunci, nilai) = (kunci.trim(), nilai.trim());
        match kunci {
            "type" => k.tipe = nilai.to_string(),
            "scope" => k.scope = nilai.to_string(),
            "client_id" if !nilai.is_empty() => k.client_sendiri = true,
            _ => {}
        }
    }
    k
}

pub fn jenis_remote(nama: &str) -> String {
    let t = konfig_akun(nama).tipe;
    match t.as_str() {
        "drive" => "google".into(),
        // SharePoint & OneDrive memakai backend yang sama; dibedakan lewat nama remote.
        "onedrive" if jenis_dari_nama(nama) == "sharepoint" => "sharepoint".into(),
        "onedrive" => "onedrive".into(),
        _ => t,
    }
}

/// Semua akun Pindah Drive. Pengaturan dibaca sekali lewat `rclone config dump` di dalam proses ini
/// (memuat token, tetapi hanya type/scope/client_id yang diambil; tidak pernah dicetak). client_id
/// dibutuhkan apa adanya untuk tahu akun mana yang sudah memakai client ID aplikasi, sedangkan
/// `config redacted` menyensornya.
pub fn daftar_akun() -> Vec<Akun> {
    let mut hasil = Vec::new();
    if rclone::cari_rclone().is_none() {
        return hasil;
    }
    let meta = baca_meta_akun();
    let id_aplikasi = baca_client_bawaan().map(|(id, _)| id).unwrap_or_default();
    let h = panggil(&["config", "dump"], Mode::Tangkap);
    let semua: Map<String, Value> = serde_json::from_str(&h.out).unwrap_or_default();
    for (nama, r) in semua {
        let m = meta.get(&nama);
        let ambil = |kunci: &str| teks(m.and_then(|m| m.get(kunci)));
        let impor = ambil("Impor") == "1";
        if !nama.starts_with("pd-") && !impor {
            continue;
        }
        let tipe = teks(r.get("type"));
        let client_id = teks(r.get("client_id"));
        let k = Konfig {
            tipe: tipe.clone(),
            scope: teks(r.get("scope")),
            client_sendiri: !client_id.is_empty(),
        };
        let client_aplikasi = !client_id.is_empty() && client_id == id_aplikasi;
        // Jenis: dari prefiks nama (remote buatan LELEMOVE), dari akun.json, atau dari isi konfigurasi.
        let jenis = match penyedia::dari_nama(&nama).or_else(|| penyedia::cari(&ambil("Jenis"))) {
            Some(p) if p.kunci == "sharepoint" || p.backend == tipe => p.kunci.to_string(),
            _ => penyedia::dari_konfig(&tipe, &teks(r.get("provider")), &teks(r.get("vendor")))
                .map_or_else(|| tipe.clone(), |p| p.kunci.to_string()),
        };
        let mut label = ambil("Label");
        let mut drive = ambil("Drive");
        if label.is_empty() {
            label = label_bawaan(&nama, &jenis);
        }
        if drive.is_empty() && nama == REMOTE_ONEDRIVE {
            // pengaturan versi lama
            if let Some(lama) = baca_objek(&jalur().data.join("onedrive.json")) {
                let n = teks(lama.get("Nama"));
                if !n.is_empty() {
                    drive = format!("{n} ({})", teks(lama.get("Jenis")));
                }
            }
        }
        let drive_id = ambil("DriveId");
        hasil.push(Akun {
            siap: jenis != "sharepoint" || !drive_id.is_empty(),
            situs: ambil("Situs"),
            drive_id,
            nama,
            jenis,
            impor,
            label,
            drive,
            scope: k.scope,
            client_sendiri: k.client_sendiri,
            client_aplikasi,
        });
    }
    hasil
}

pub fn baca_client_bawaan() -> Option<(String, String)> {
    for f in [&jalur().client_pengguna, &jalur().client_app] {
        if let Some(j) = baca_objek(f) {
            let (id, rahasia) = (teks(j.get("ClientId")), teks(j.get("ClientSecret")));
            if !id.is_empty() && !rahasia.is_empty() {
                return Some((id, rahasia));
            }
        }
    }
    None
}

pub fn simpan_client_bawaan(id: &str, rahasia: &str) {
    let mut m = Map::new();
    m.insert("ClientId".into(), Value::String(id.into()));
    m.insert("ClientSecret".into(), Value::String(rahasia.into()));
    tulis_json(&jalur().client_pengguna, &Value::Object(m));
}

/// Client ID aplikasi yang berlaku (tanpa secret) dan asalnya:
/// "komputer ini" (disimpan lewat Pengaturan) atau "installer" (dibundel saat aplikasi dipasang).
pub fn info_client_bawaan() -> Option<(String, &'static str)> {
    for (f, asal) in [(&jalur().client_pengguna, "komputer ini"), (&jalur().client_app, "installer")] {
        if let Some(j) = baca_objek(f) {
            let id = teks(j.get("ClientId"));
            if !id.is_empty() && !teks(j.get("ClientSecret")).is_empty() {
                return Some((id, asal));
            }
        }
    }
    None
}

/// Menghapus client ID yang disimpan lewat Pengaturan. Client ID bawaan installer (bila ada)
/// berlaku lagi. Hasil: ada yang dihapus.
pub fn hapus_client_pengguna() -> bool {
    std::fs::remove_file(&jalur().client_pengguna).is_ok()
}

/// "202264815644-abcdefgh.apps.googleusercontent.com" -> "202264815644-abcd…"
pub fn id_pendek(id: &str) -> String {
    let inti = id.trim_end_matches(".apps.googleusercontent.com");
    let n = inti.find('-').map_or(12, |i| i + 5);
    if inti.chars().count() <= n {
        return inti.to_string();
    }
    format!("{}\u{2026}", inti.chars().take(n).collect::<String>())
}

/// Nama remote baru yang belum terpakai: pd-gdrive, pd-gdrive-2, pd-dropbox, pd-dropbox-2, ...
pub fn remote_baru(jenis: &str) -> String {
    let dasar = match jenis {
        "google" => REMOTE_GOOGLE,
        "sharepoint" => REMOTE_SHAREPOINT,
        "onedrive" => REMOTE_ONEDRIVE,
        j => penyedia::cari(j).map_or(REMOTE_ONEDRIVE, |p| p.prefiks),
    };
    let ada = daftar_remote();
    if !ada.iter().any(|r| r == dasar) {
        return dasar.to_string();
    }
    (2..100)
        .map(|i| format!("{dasar}-{i}"))
        .find(|n| !ada.contains(n))
        .unwrap_or_else(|| format!("{dasar}-{}", std::process::id()))
}

/// Galat rclone "layanan ini tidak mendukung about" (info kuota).
pub fn tanpa_about(err: &str) -> bool {
    let e = err.to_lowercase();
    e.contains("doesn't support about") || e.contains("does not support about") || e.contains("about not supported")
        || (e.contains("about") && e.contains("not supported"))
}

/// Remote rclone yang belum dipakai LELEMOVE: (nama, backend). Untuk "Remote rclone yang sudah ada".
pub fn remote_luar() -> Vec<(String, String)> {
    let meta = baca_meta_akun();
    let h = panggil(&["config", "dump"], Mode::Tangkap);
    let semua: Map<String, Value> = serde_json::from_str(&h.out).unwrap_or_default();
    semua
        .into_iter()
        .filter(|(n, _)| !n.starts_with("pd-") && teks(meta.get(n).and_then(|m| m.get("Impor"))) != "1")
        .map(|(n, r)| (n, teks(r.get("type"))))
        .collect()
}

/// Jenis (kunci penyedia) untuk remote yang sudah ada, dari isi konfigurasinya.
pub fn jenis_konfig(nama: &str) -> String {
    let h = panggil(&["config", "dump"], Mode::Tangkap);
    let semua: Map<String, Value> = serde_json::from_str(&h.out).unwrap_or_default();
    let Some(r) = semua.get(nama) else { return String::new() };
    let tipe = teks(r.get("type"));
    penyedia::dari_konfig(&tipe, &teks(r.get("provider")), &teks(r.get("vendor"))).map_or(tipe, |p| p.kunci.to_string())
}

/// Nilai opsi konfigurasi yang tidak rahasia (url, host, port, endpoint) untuk menguji sambungan.
pub fn opsi_konfig(nama: &str, kunci: &[&str]) -> Vec<(String, String)> {
    let h = panggil(&v!["config", "redacted", format!("{nama}:")], Mode::Tangkap);
    h.out
        .lines()
        .filter_map(|b| b.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .filter(|(k, v)| kunci.contains(&k.as_str()) && !v.is_empty() && v != "XXX")
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
pub enum Sambungan {
    Baik,
    /// Token login kedaluwarsa/dicabut: perlu login ulang.
    Kedaluwarsa,
    /// Internet/DNS putus saat diuji (bukan masalah akun); jendela menguji ulang sendiri.
    Jaringan(String),
    Galat(String),
}

/// Menguji akun dengan permintaan ringan (kuota). Dipakai jendela saat dibuka.
/// Galat jaringan dicoba lagi dua kali (jeda 2 dan 5 detik): DNS sebagian jaringan kantor
/// sesekali menolak permintaan, padahal akunnya baik-baik saja.
pub fn cek_sambungan(nama: &str) -> Sambungan {
    let mut hasil = Sambungan::Galat(String::new());
    for jeda in [0u64, 2, 5] {
        if jeda > 0 {
            std::thread::sleep(std::time::Duration::from_secs(jeda));
        }
        let mut h = panggil(
            &v!["about", format!("{nama}:"), "--json", "--contimeout", "15s", "--timeout", "30s", "--low-level-retries", "2"],
            Mode::Tangkap,
        );
        // FTP, S3, B2, dll. tidak melaporkan kuota: uji dengan membaca daftar folder teratas.
        if h.kode != 0 && tanpa_about(&h.err) {
            h = panggil(
                &v!["lsf", format!("{nama}:"), "--max-depth", "1", "--contimeout", "15s", "--timeout", "30s", "--low-level-retries", "2"],
                Mode::Tangkap,
            );
        }
        if h.kode == 0 {
            return Sambungan::Baik;
        }
        if token_kedaluwarsa(&h.err) {
            return Sambungan::Kedaluwarsa;
        }
        let pesan = h.galat_terakhir(1).join(" ");
        if !galat_jaringan(&h.err) {
            return Sambungan::Galat(pesan);
        }
        hasil = Sambungan::Jaringan(pesan);
    }
    hasil
}

// ------------------------------------------------------------------ SharePoint
// Situs & pustaka dipilih lewat mesin konfigurasi rclone yang "dilompati" langsung ke langkahnya:
//   search_end   (kata kunci) -> daftar situs
//   search_sites (id situs)   -> daftar pustaka dokumen
//   driveid_final (id drive)  -> pakai pustaka itu
// Alur "url" bawaan rclone gagal di tenant tertentu ("Invalid hostname"), jadi URL diubah jadi pencarian.

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Situs {
    pub nama: String,
    pub url: String,
    pub id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Pustaka {
    pub nama: String,
    pub id: String,
}

/// Langkah konfigurasi butuh access token yang masih berlaku; operasi biasa memperbaruinya.
pub fn segarkan_token(nama: &str) {
    let _ = panggil(&v!["about", format!("{nama}:")], Mode::Tangkap);
}

pub fn langkah_konfig(nama: &str, state: &str, hasil: &str) -> Option<Value> {
    let h = panggil(
        &v!["config", "update", nama, "--continue", "--non-interactive", "--state", state, "--result", hasil],
        Mode::Tangkap,
    );
    serde_json::from_str(&h.out).ok()
}

/// Contoh pilihan (Option.Examples) dari jawaban mesin konfigurasi rclone: (Value, Help).
pub fn contoh_pilihan(j: &Value) -> Vec<(String, String)> {
    j.get("Option")
        .and_then(|o| o.get("Examples"))
        .and_then(Value::as_array)
        .map(|a| a.iter().map(|e| (teks(e.get("Value")), teks(e.get("Help")))).collect())
        .unwrap_or_default()
}

fn galat_langkah(j: &Option<Value>) -> Option<String> {
    match j {
        None => Some("Tidak ada jawaban dari rclone.".into()),
        Some(v) => {
            let e = teks(v.get("Error"));
            if e.is_empty() {
                None
            } else {
                Some(e)
            }
        }
    }
}

/// "Nama Situs (https://x.sharepoint.com/sites/A)" -> (nama, url)
fn pecah_situs(help: &str) -> (String, String) {
    if let Some(buka) = help.rfind(" (http") {
        let sisa = help[buka + 2..].trim_end();
        if let Some(url) = sisa.strip_suffix(')') {
            return (help[..buka].trim().to_string(), url.to_string());
        }
    }
    (help.trim().to_string(), String::new())
}

/// "Dokumen (documentLibrary)" -> "Dokumen"
pub fn tanpa_jenis(help: &str) -> String {
    let t = help.trim_end();
    if t.ends_with(')') {
        if let Some(i) = t.rfind(" (") {
            let isi = &t[i + 2..t.len() - 1];
            if !isi.is_empty() && isi.chars().all(|c| c.is_alphanumeric() || c == '_') {
                return t[..i].trim().to_string();
            }
        }
    }
    t.to_string()
}

pub fn cari_situs(nama: &str, kata: &str) -> Result<Vec<Situs>, String> {
    segarkan_token(nama);
    let j = langkah_konfig(nama, "search_end", kata);
    if let Some(e) = galat_langkah(&j) {
        return Err(e);
    }
    Ok(contoh_pilihan(j.as_ref().unwrap())
        .into_iter()
        .filter(|(id, _)| !id.is_empty())
        .map(|(id, help)| {
            let (nama, url) = pecah_situs(&help);
            Situs { nama, url, id }
        })
        .collect())
}

pub fn daftar_pustaka(nama: &str, situs_id: &str) -> Result<Vec<Pustaka>, String> {
    segarkan_token(nama);
    let j = langkah_konfig(nama, "search_sites", situs_id);
    if let Some(e) = galat_langkah(&j) {
        return Err(e);
    }
    Ok(contoh_pilihan(j.as_ref().unwrap())
        .into_iter()
        .filter(|(id, _)| !id.is_empty())
        .map(|(id, help)| Pustaka { nama: tanpa_jenis(&help), id })
        .collect())
}

/// Memasang pustaka (drive) ke remote SharePoint. Err berisi pesan dari rclone.
pub fn atur_pustaka(nama: &str, drive_id: &str) -> Result<(), String> {
    segarkan_token(nama);
    let j = langkah_konfig(nama, "driveid_final", drive_id);
    let state = j.as_ref().map(|v| teks(v.get("State"))).unwrap_or_default();
    if state.is_empty() {
        return Err(j.as_ref().map(|v| teks(v.get("Error"))).unwrap_or_default());
    }
    let _ = langkah_konfig(nama, &state, "true");
    Ok(())
}

/// Isi folder di akun cloud (satu tingkat, hanya folder), untuk jendela "Telusuri".
pub fn daftar_folder_cloud(remote: &str, jalur: &str) -> Result<Vec<String>, String> {
    let h = panggil(
        &v!["lsf", format!("{remote}:{jalur}"), "--dirs-only", "--max-depth", "1"],
        Mode::Tangkap,
    );
    if h.kode != 0 {
        let e = h
            .err
            .lines()
            .map(str::trim)
            .rfind(|l| !l.is_empty() && !l.contains("client_id"))
            .unwrap_or("")
            .to_string();
        return Err(e);
    }
    let mut n: Vec<String> = h
        .out
        .lines()
        .map(|l| l.trim().trim_end_matches('/').to_string())
        .filter(|l| !l.is_empty())
        .collect();
    n.sort_by_key(|a| a.to_lowercase());
    Ok(n)
}

#[cfg(test)]
mod uji {
    use super::*;

    #[test]
    fn remote() {
        assert_eq!(remote_dari("pd-onedrive:Kerja/DLH"), "pd-onedrive");
        assert_eq!(remote_dari(crate::win::CONTOH_DATA), "");
        assert_eq!(remote_dari("pd-gdrive:"), "pd-gdrive");
        assert_eq!(label_bawaan("pd-gdrive-2", "google"), "Google Drive 2");
        assert_eq!(label_bawaan("pd-onedrive", "onedrive"), "OneDrive");
    }

    #[test]
    fn situs() {
        let (n, u) = pecah_situs("Bagian Keuangan (https://x.sharepoint.com/sites/Keuangan)");
        assert_eq!(n, "Bagian Keuangan");
        assert_eq!(u, "https://x.sharepoint.com/sites/Keuangan");
        assert_eq!(tanpa_jenis("Dokumen (documentLibrary)"), "Dokumen");
        assert_eq!(tanpa_jenis("OneDrive (business)"), "OneDrive");
    }
}
