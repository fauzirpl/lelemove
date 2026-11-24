//! Migrasi email IMAP -> IMAP (perintah `pindah email`): Gmail, Outlook/Microsoft 365, atau
//! server IMAP lain sebagai sumber; server email sendiri (IMAP + kata sandi) sebagai tujuan.
//!
//! Seperti penyalinan file, migrasi hanya MENYALIN: sumber dibuka baca-saja (EXAMINE), tidak ada
//! email yang dihapus atau ditandai di sumber, dan di tujuan hanya ditambahkan email yang belum
//! ada (dicocokkan lewat Message-ID). Karena itu migrasi aman diulang: yang sudah tersalin
//! dilewati, termasuk setelah sambungan putus atau batas harian Gmail tercapai.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::time::{Duration, Instant};

use super::dasar::*;
use super::imap::{bendera_salin, dari_utf7, ke_utf7, Folder, Galat, Hasil, Imap, Keamanan, Ringkas};
use super::oauth::{self, Klien, Penyedia, Token};

/// Satu kali FETCH isi: paling banyak sekian email atau sekian byte (hemat memori).
const BATCH_EMAIL: usize = 25;
const BATCH_BYTE: u64 = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Jenis {
    Gmail,
    Outlook,
    Imap,
}

/// Akun email: "gmail:nama@gmail.com", "outlook:nama@outlook.com", atau
/// "imap://pengguna@server[:port]" (SSL/TLS), "imap+starttls://...", "imap+polos://...".
#[derive(Clone, Debug, PartialEq)]
pub struct Akun {
    pub jenis: Jenis,
    pub host: String,
    pub port: u16,
    pub keamanan: Keamanan,
    pub pengguna: String,
}

fn email_sah(s: &str) -> Result<String, String> {
    let e = s.trim().trim_start_matches("//").trim();
    if e.contains('@') && !e.contains(char::is_whitespace) && !e.starts_with('@') && !e.ends_with('@') {
        Ok(e.to_string())
    } else {
        Err(format!("Alamat email tidak sah: {e}"))
    }
}

impl Akun {
    pub fn urai(s: &str) -> Result<Akun, String> {
        let s = s.trim();
        let (skema, sisa) = s
            .split_once(':')
            .ok_or_else(|| format!("Akun email \"{s}\" tidak dikenal. Contoh: gmail:nama@gmail.com, outlook:nama@outlook.com, imap://pengguna@mail.server.id"))?;
        let skema = skema.to_lowercase();
        match skema.as_str() {
            "gmail" | "google" => Ok(Akun { jenis: Jenis::Gmail, host: "imap.gmail.com".into(), port: 993, keamanan: Keamanan::Ssl, pengguna: email_sah(sisa)? }),
            "outlook" | "microsoft" | "office365" | "hotmail" => {
                Ok(Akun { jenis: Jenis::Outlook, host: "outlook.office365.com".into(), port: 993, keamanan: Keamanan::Ssl, pengguna: email_sah(sisa)? })
            }
            "imap" | "imaps" | "imap+ssl" | "imap+starttls" | "imap+polos" => {
                let sisa = sisa.strip_prefix("//").ok_or_else(|| format!("Tulis {skema}://pengguna@server"))?.trim_end_matches('/');
                let (pengguna, alamat) = sisa
                    .rsplit_once('@')
                    .ok_or_else(|| "Tulis pengguna@server, mis. imap://info@dinas.go.id@mail.dinas.go.id".to_string())?;
                let keamanan = match skema.as_str() {
                    "imap+starttls" => Keamanan::StartTls,
                    "imap+polos" => Keamanan::Tanpa,
                    _ => Keamanan::Ssl,
                };
                let bawaan = if keamanan == Keamanan::Ssl { 993 } else { 143 };
                let (host, port) = match alamat.rsplit_once(':') {
                    Some((h, p)) => (h, p.parse::<u16>().map_err(|_| format!("Port tidak sah: {p}"))?),
                    None => (alamat, bawaan),
                };
                if pengguna.is_empty() || host.is_empty() {
                    return Err("Pengguna dan server email harus diisi.".into());
                }
                Ok(Akun { jenis: Jenis::Imap, host: host.to_lowercase(), port, keamanan, pengguna: pengguna.to_string() })
            }
            _ => Err(format!("Jenis akun email tidak dikenal: {skema}. Pakai gmail:, outlook:, atau imap://")),
        }
    }

    /// Bentuk teks yang bisa diurai lagi (disimpan di email.json).
    pub fn alamat(&self) -> String {
        match self.jenis {
            Jenis::Gmail => format!("gmail:{}", self.pengguna),
            Jenis::Outlook => format!("outlook:{}", self.pengguna),
            Jenis::Imap => {
                let skema = match self.keamanan {
                    Keamanan::Ssl => "imap",
                    Keamanan::StartTls => "imap+starttls",
                    Keamanan::Tanpa => "imap+polos",
                };
                let bawaan = if self.keamanan == Keamanan::Ssl { 993 } else { 143 };
                let port = if self.port == bawaan { String::new() } else { format!(":{}", self.port) };
                format!("{skema}://{}@{}{port}", self.pengguna, self.host)
            }
        }
    }

    pub fn tampil(&self) -> String {
        match self.jenis {
            Jenis::Gmail => format!("Gmail {}", self.pengguna),
            Jenis::Outlook => format!("Outlook {}", self.pengguna),
            Jenis::Imap => format!("{} di {}", self.pengguna, self.host),
        }
    }
}

// ------------------------------------------------------------------ setelan tersimpan

/// Setelan migrasi terakhir (tanpa kata sandi), di <data>/email.json.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct Setelan {
    pub sumber: String,
    pub tujuan: String,
    /// Gmail: login Google (OAuth, client ID LELEMOVE) alih-alih sandi aplikasi.
    pub login_google: bool,
    pub sertakan_sampah: bool,
    /// Gmail: jangan salin email arsip (tanpa label) ke folder Archive.
    pub tanpa_arsip: bool,
    /// Terima sertifikat swatanda di server tujuan/sumber IMAP lain.
    pub abaikan_sertifikat: bool,
}

pub fn baca_setelan() -> Setelan {
    baca_json(&jalur().data.join("email.json")).and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default()
}

pub fn simpan_setelan(s: &Setelan) {
    tulis_json(&jalur().data.join("email.json"), &serde_json::to_value(s).unwrap_or(Value::Null));
}

/// Client ID aplikasi Azure untuk login Outlook/Microsoft 365 (bukan rahasia).
pub fn client_microsoft() -> Option<String> {
    let v = baca_json(&jalur().data.join("client-microsoft.json"))?;
    v.get("ClientId").and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).map(str::to_string)
}

pub fn simpan_client_microsoft(id: &str) {
    tulis_json(&jalur().data.join("client-microsoft.json"), &json!({ "ClientId": id.trim() }));
}

/// Client ID Azure berbentuk GUID (8-4-4-4-12 heksadesimal).
pub fn client_microsoft_sah(id: &str) -> bool {
    let b: Vec<&str> = id.trim().split('-').collect();
    b.len() == 5 && b.iter().zip([8, 4, 4, 4, 12]).all(|(x, n)| x.len() == n && x.chars().all(|c| c.is_ascii_hexdigit()))
}

// ------------------------------------------------------------------ rencana folder

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Khusus {
    Masuk,
    Terkirim,
    Draf,
    Sampah,
    Spam,
    Arsip,
    /// Gmail "Semua Email": semua email sekaligus; diproses terakhir sebagai arsip.
    Semua,
    /// Folder virtual Gmail (Berbintang, Penting): isinya duplikat folder lain.
    Virtual,
}

const NAMA_KHUSUS: [(Khusus, &[&str]); 5] = [
    (Khusus::Terkirim, &["sent", "sent items", "sent mail", "sent messages", "item terkirim", "terkirim", "pesan terkirim"]),
    (Khusus::Draf, &["drafts", "draft", "draf", "konsep"]),
    (Khusus::Sampah, &["trash", "deleted items", "deleted messages", "bin", "sampah", "item terhapus", "tempat sampah"]),
    (Khusus::Spam, &["junk", "junk email", "junk e-mail", "spam", "bulk mail", "email sampah"]),
    (Khusus::Arsip, &["archive", "archives", "arsip"]),
];

/// Folder Outlook yang bukan email (kalender, kontak, ...), juga nama Indonesianya.
const BUKAN_EMAIL_OUTLOOK: &[&str] = &[
    "calendar", "contacts", "journal", "notes", "tasks", "sync issues", "outbox", "conversation history", "rss feeds",
    "rss subscriptions", "kalender", "kontak", "jurnal", "catatan", "tugas", "masalah sinkronisasi", "kotak keluar",
    "riwayat percakapan",
];

fn baku(k: Khusus) -> &'static str {
    match k {
        Khusus::Terkirim => "Sent",
        Khusus::Draf => "Drafts",
        Khusus::Sampah => "Trash",
        Khusus::Spam => "Junk",
        _ => "Archive",
    }
}

/// Komponen jalur folder (UTF-7 diurai, awalan ruang nama dan "[Gmail]" dibuang).
fn komponen(f: &Folder, awalan: &str) -> Vec<String> {
    let mut nama = dari_utf7(&f.nama);
    if !awalan.is_empty() && nama.len() > awalan.len() && nama[..awalan.len()].eq_ignore_ascii_case(awalan) {
        nama = nama[awalan.len()..].to_string();
    }
    let mut k: Vec<String> = match f.pemisah {
        Some(p) => nama.split(p).map(str::to_string).collect(),
        None => vec![nama],
    };
    if k.len() > 1 && (k[0] == "[Gmail]" || k[0] == "[Google Mail]") {
        k.remove(0);
    }
    k.retain(|x| !x.is_empty());
    k
}

fn jenis_khusus(f: &Folder, awalan: &str) -> Option<Khusus> {
    if f.nama.eq_ignore_ascii_case("INBOX") {
        return Some(Khusus::Masuk);
    }
    for (a, k) in [
        ("\\sent", Khusus::Terkirim),
        ("\\drafts", Khusus::Draf),
        ("\\trash", Khusus::Sampah),
        ("\\junk", Khusus::Spam),
        ("\\archive", Khusus::Arsip),
        ("\\all", Khusus::Semua),
        ("\\flagged", Khusus::Virtual),
        ("\\important", Khusus::Virtual),
    ] {
        if f.punya(a) {
            return Some(k);
        }
    }
    // Tanpa atribut SPECIAL-USE: tebak dari nama, hanya untuk folder di tingkat teratas.
    let k = komponen(f, awalan);
    let gmail = dari_utf7(&f.nama).starts_with("[Gmail]") || dari_utf7(&f.nama).starts_with("[Google Mail]");
    if k.len() != 1 {
        return None;
    }
    let n = k[0].to_lowercase();
    if gmail && (n == "all mail" || n == "semua email") {
        return Some(Khusus::Semua);
    }
    if gmail && matches!(n.as_str(), "starred" | "important" | "berbintang" | "penting") {
        return Some(Khusus::Virtual);
    }
    NAMA_KHUSUS.iter().find(|(_, d)| d.contains(&n.as_str())).map(|(k, _)| *k)
}

/// Gambaran server tujuan: ruang nama pribadi dan folder yang sudah ada.
#[derive(Clone, Debug)]
pub struct Tujuan {
    pub awalan: String,
    pub pemisah: char,
    pub folder: Vec<Folder>,
}

impl Tujuan {
    fn ada(&self, nama: &str) -> bool {
        self.folder.iter().any(|f| f.nama == nama || (nama.eq_ignore_ascii_case("INBOX") && f.nama.eq_ignore_ascii_case("INBOX")))
    }

    /// Folder khusus di tujuan: menurut atribut SPECIAL-USE, lalu nama umum di tingkat teratas.
    fn khusus(&self, k: Khusus) -> Option<String> {
        let atribut = match k {
            Khusus::Terkirim => "\\sent",
            Khusus::Draf => "\\drafts",
            Khusus::Sampah => "\\trash",
            Khusus::Spam => "\\junk",
            _ => "\\archive",
        };
        if let Some(f) = self.folder.iter().find(|f| f.punya(atribut)) {
            return Some(f.nama.clone());
        }
        let cari = if k == Khusus::Semua { Khusus::Arsip } else { k };
        let nama = NAMA_KHUSUS.iter().find(|(x, _)| *x == cari).map(|(_, d)| *d).unwrap_or(&[]);
        self.folder
            .iter()
            .find(|f| {
                let c = komponen(f, &self.awalan);
                c.len() == 1 && nama.contains(&c[0].to_lowercase().as_str())
            })
            .map(|f| f.nama.clone())
    }

    fn nama_untuk(&self, k: &[String]) -> String {
        let mut k = k.to_vec();
        // Ruang nama "INBOX." (cPanel/Courier): semua folder sudah di bawah INBOX.
        if self.awalan.to_uppercase().starts_with("INBOX") && k.len() > 1 && k[0].eq_ignore_ascii_case("INBOX") {
            k.remove(0);
        }
        let bagian: Vec<String> = k.iter().map(|x| x.replace(self.pemisah, "_")).collect();
        ke_utf7(&format!("{}{}", self.awalan, bagian.join(&self.pemisah.to_string())))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Rencana {
    /// Nama folder di server sumber/tujuan (UTF-7 IMAP, dipakai di perintah).
    pub sumber: String,
    pub tujuan: String,
    pub tampil_sumber: String,
    pub tampil_tujuan: String,
    /// Folder tujuan sudah ada.
    pub ada: bool,
    /// Gmail "Semua Email": hanya email yang tidak ada di folder lain (arsip tanpa label).
    pub arsip: bool,
    /// Alasan dilewati (None = disalin).
    pub lewati: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct OpsiMigrasi {
    pub sertakan_sampah: bool,
    pub arsip_gmail: bool,
}

/// Peta folder sumber -> tujuan. INBOX dulu, lalu urut nama, arsip Gmail terakhir.
pub fn susun_rencana(sumber: &[Folder], awalan_sumber: &str, t: &Tujuan, jenis: Jenis, opsi: &OpsiMigrasi) -> Vec<Rencana> {
    let mut v: Vec<Rencana> = Vec::new();
    for f in sumber {
        let tampil_sumber = dari_utf7(&f.nama);
        let k = jenis_khusus(f, awalan_sumber);
        let komp = komponen(f, awalan_sumber);
        let mut r = Rencana {
            sumber: f.nama.clone(),
            tujuan: String::new(),
            tampil_sumber: tampil_sumber.clone(),
            tampil_tujuan: String::new(),
            ada: false,
            arsip: k == Some(Khusus::Semua),
            lewati: None,
        };
        if !f.bisa_dipilih() {
            continue; // induk saja ([Gmail]), tidak berisi email
        }
        r.lewati = match k {
            Some(Khusus::Virtual) => Some("folder virtual Gmail (isinya ada di folder lain)".into()),
            Some(Khusus::Semua) if !opsi.arsip_gmail => Some("Semua Email Gmail (isinya ada di folder lain)".into()),
            Some(Khusus::Sampah | Khusus::Spam) if !opsi.sertakan_sampah => Some("sampah/spam tidak disertakan".into()),
            _ if jenis == Jenis::Outlook && komp.first().is_some_and(|x| BUKAN_EMAIL_OUTLOOK.contains(&x.to_lowercase().as_str())) => {
                Some("bukan folder email (kalender/kontak/tugas)".into())
            }
            _ => None,
        };
        r.tujuan = match k {
            Some(Khusus::Masuk) => "INBOX".into(),
            Some(x @ (Khusus::Terkirim | Khusus::Draf | Khusus::Sampah | Khusus::Spam | Khusus::Arsip | Khusus::Semua)) => {
                t.khusus(x).unwrap_or_else(|| t.nama_untuk(&[baku(x).to_string()]))
            }
            _ => t.nama_untuk(&komp),
        };
        r.ada = t.ada(&r.tujuan);
        r.tampil_tujuan = dari_utf7(&r.tujuan);
        v.push(r);
    }
    v.sort_by_key(|r| (r.arsip, !r.sumber.eq_ignore_ascii_case("INBOX"), r.lewati.is_some(), r.tampil_sumber.to_lowercase()));
    v
}

// ------------------------------------------------------------------ sambungan dengan sambung ulang

pub enum Kred {
    Sandi(String),
    OAuth { p: &'static Penyedia, klien: Klien, token: Option<Token> },
}

struct Sisi {
    akun: Akun,
    kred: Kred,
    abaikan_sertifikat: bool,
    imap: Option<Imap>,
    /// Folder yang sedang dibuka (EXAMINE) dan jumlah emailnya.
    buka: Option<(String, u32)>,
}

/// Petunjuk untuk galat login yang umum.
fn petunjuk_login(a: &Akun, oauth: bool, e: Galat) -> Galat {
    let Galat::Ditolak(t) = e else { return e };
    let k = t.to_lowercase();
    let saran = match a.jenis {
        Jenis::Gmail if !oauth && (k.contains("application-specific") || k.contains("invalid credentials") || k.contains("authenticationfailed")) => {
            " Gmail menolak kata sandi biasa: pakai sandi aplikasi (myaccount.google.com/apppasswords, akun harus memakai verifikasi 2 langkah), atau pilih Login Google."
        }
        Jenis::Gmail if oauth => " Pastikan akun ini terdaftar sebagai pengguna uji di Google Cloud Console (project client ID LELEMOVE).",
        Jenis::Outlook => " Pastikan izin IMAP.AccessAsUser.All sudah ditambahkan di aplikasi Azure, dan IMAP aktif di Outlook (Pengaturan > Email > Teruskan dan IMAP).",
        _ if k.contains("auth") || k.contains("login") || k.contains("credentials") || k.contains("password") => " Periksa nama pengguna dan kata sandi.",
        _ => "",
    };
    Galat::Ditolak(format!("login {} ditolak: {t}.{saran}", a.tampil()))
}

impl Sisi {
    fn sambung(&mut self) -> Hasil<()> {
        let mut s = Imap::sambung(&self.akun.host, self.akun.port, self.akun.keamanan, self.abaikan_sertifikat)?;
        let akun = self.akun.clone();
        match &mut self.kred {
            Kred::Sandi(p) => s.masuk_sandi(&akun.pengguna, p).map_err(|e| petunjuk_login(&akun, false, e))?,
            Kred::OAuth { p, klien, token } => {
                let t = match token.take() {
                    Some(t) if !t.hampir_habis() => t,
                    Some(t) => oauth::segarkan(p, klien, &t).map_err(Galat::Ditolak)?,
                    None => oauth::login(p, klien, &akun.pengguna, |a| crate::win::buka(a), info).map_err(Galat::Ditolak)?,
                };
                let r = s.masuk_oauth(&akun.pengguna, &t.akses);
                *token = Some(t);
                r.map_err(|e| petunjuk_login(&akun, true, e))?;
            }
        }
        self.imap = Some(s);
        self.buka = None;
        Ok(())
    }

    /// Menjalankan `f` pada sambungan yang hidup, dengan `folder` dibuka (EXAMINE) bila diberikan;
    /// `f` menerima jumlah email folder itu. Bila sambungan putus: sambung ulang lalu ulangi.
    fn coba<T>(&mut self, folder: Option<&str>, mut f: impl FnMut(&mut Imap, u32) -> Hasil<T>) -> Hasil<T> {
        const JEDA: [u64; 6] = [0, 5, 15, 30, 60, 120];
        let mut terakhir = String::new();
        for (i, jeda) in JEDA.iter().enumerate() {
            if *jeda > 0 {
                awas(&format!("Sambungan ke {} putus ({terakhir}); mencoba lagi dalam {jeda} detik ({i}/{}).", self.akun.host, JEDA.len() - 1));
                std::thread::sleep(Duration::from_secs(*jeda));
            }
            match self.langkah(folder, &mut f) {
                Ok(v) => return Ok(v),
                Err(Galat::Putus(e)) => {
                    terakhir = e;
                    self.imap = None;
                    self.buka = None;
                }
                Err(e) => return Err(e),
            }
        }
        Err(Galat::Putus(format!("sambungan ke {} terus putus: {terakhir}", self.akun.host)))
    }

    fn langkah<T>(&mut self, folder: Option<&str>, f: &mut impl FnMut(&mut Imap, u32) -> Hasil<T>) -> Hasil<T> {
        if self.imap.is_none() {
            self.sambung()?;
        }
        let imap = self.imap.as_mut().expect("tersambung");
        let jumlah = match folder {
            Some(fl) => match &self.buka {
                Some((n, j)) if n == fl => *j,
                _ => {
                    let j = imap.periksa(fl)?;
                    self.buka = Some((fl.to_string(), j));
                    j
                }
            },
            None => 0,
        };
        f(imap, jumlah)
    }

    /// Buka ulang folder (jumlah email terbaru), mis. setelah folder dibuat atau diisi.
    fn lupakan_folder(&mut self) {
        self.buka = None;
    }

    fn tutup(&mut self) {
        if let Some(mut s) = self.imap.take() {
            s.keluar();
        }
    }
}

// ------------------------------------------------------------------ migrasi

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct HasilFolder {
    pub sumber: String,
    pub tujuan: String,
    pub email: u64,
    pub sudah_ada: u64,
    pub disalin: u64,
    pub gagal: u64,
    pub byte: u64,
    pub lewati: String,
}

pub struct Migrasi {
    src: Sisi,
    dst: Sisi,
    pub opsi: OpsiMigrasi,
    /// Cetak "##PROGRES {json}" untuk jendela aplikasi.
    pub mesin: bool,
    pub folder: Vec<HasilFolder>,
    total_email: u64,
    selesai_email: u64,
    progres_terakhir: Option<Instant>,
    contoh_gagal: Vec<String>,
}

impl Migrasi {
    pub fn baru(sumber: Akun, kred_sumber: Kred, tujuan: Akun, kred_tujuan: Kred, abaikan_sertifikat: bool, opsi: OpsiMigrasi, mesin: bool) -> Migrasi {
        Migrasi {
            src: Sisi { akun: sumber, kred: kred_sumber, abaikan_sertifikat, imap: None, buka: None },
            dst: Sisi { akun: tujuan, kred: kred_tujuan, abaikan_sertifikat, imap: None, buka: None },
            opsi,
            mesin,
            folder: Vec::new(),
            total_email: 0,
            selesai_email: 0,
            progres_terakhir: None,
            contoh_gagal: Vec::new(),
        }
    }

    pub fn contoh_gagal(&self) -> &[String] {
        &self.contoh_gagal
    }

    fn progres(&mut self, folder: &str, urutan: usize, jumlah_folder: usize, paksa: bool) {
        if !self.mesin {
            return;
        }
        if !paksa && self.progres_terakhir.is_some_and(|t| t.elapsed() < Duration::from_millis(400)) {
            return;
        }
        self.progres_terakhir = Some(Instant::now());
        let disalin: u64 = self.folder.iter().map(|f| f.disalin).sum();
        let byte: u64 = self.folder.iter().map(|f| f.byte).sum();
        polos(&format!(
            "##PROGRES {}",
            json!({
                "Folder": folder, "FolderKe": urutan, "FolderJumlah": jumlah_folder,
                "Selesai": self.selesai_email, "Total": self.total_email, "Disalin": disalin, "Byte": byte
            })
        ));
    }

    /// Menyambung dan login ke kedua server, lalu menyusun peta folder.
    pub fn siapkan(&mut self) -> Result<Vec<Rencana>, String> {
        info(&format!("Sumber: {} ({}:{}, {})", self.src.akun.tampil(), self.src.akun.host, self.src.akun.port, self.src.akun.keamanan.nama()));
        self.src.coba(None, |_, _| Ok(())).map_err(|e| e.to_string())?;
        ok("Sumber tersambung.");
        info(&format!("Tujuan: {} ({}:{}, {})", self.dst.akun.tampil(), self.dst.akun.host, self.dst.akun.port, self.dst.akun.keamanan.nama()));
        self.dst.coba(None, |_, _| Ok(())).map_err(|e| e.to_string())?;
        ok("Tujuan tersambung.");

        let (f_sumber, ns_sumber) = self.src.coba(None, |s, _| Ok((s.daftar_folder()?, s.namespace()?))).map_err(|e| e.to_string())?;
        let (f_tujuan, ns_tujuan) = self.dst.coba(None, |s, _| Ok((s.daftar_folder()?, s.namespace()?))).map_err(|e| e.to_string())?;
        let awalan_sumber = ns_sumber.map(|(a, _)| a).unwrap_or_default();
        let pemisah = ns_tujuan
            .as_ref()
            .and_then(|(_, p)| *p)
            .or_else(|| f_tujuan.iter().find(|f| f.nama.eq_ignore_ascii_case("INBOX")).and_then(|f| f.pemisah))
            .or_else(|| f_tujuan.iter().find_map(|f| f.pemisah))
            .unwrap_or('/');
        let t = Tujuan { awalan: ns_tujuan.map(|(a, _)| a).unwrap_or_default(), pemisah, folder: f_tujuan };
        Ok(susun_rencana(&f_sumber, &awalan_sumber, &t, self.src.akun.jenis, &self.opsi))
    }

    /// Hanya memeriksa: jumlah email per folder dan berapa yang belum ada di tujuan.
    pub fn periksa(&mut self, rencana: &[Rencana]) -> Result<(), String> {
        let mut terlihat: HashSet<String> = HashSet::new();
        for r in rencana {
            let mut h = HasilFolder { sumber: r.tampil_sumber.clone(), tujuan: r.tampil_tujuan.clone(), ..Default::default() };
            if let Some(l) = &r.lewati {
                h.lewati = l.clone();
                self.folder.push(h);
                continue;
            }
            let daftar = self.src.coba(Some(&r.sumber), |s, n| s.ringkasan(n)).map_err(|e| e.to_string())?;
            let mut ada = if r.ada { self.kunci_tujuan(&r.tujuan)? } else { HashSet::new() };
            for m in &daftar {
                if r.arsip && terlihat.contains(&m.kunci) {
                    continue;
                }
                h.email += 1;
                // Email ganda di sumber (Message-ID sama) hanya disalin sekali.
                if !ada.insert(m.kunci.clone()) {
                    h.sudah_ada += 1;
                } else {
                    h.byte += m.ukuran;
                }
            }
            if !r.arsip {
                terlihat.extend(daftar.into_iter().map(|m| m.kunci));
            }
            info(&format!(
                "{} -> {}: {} email, {} sudah ada, {} akan disalin ({})",
                r.tampil_sumber,
                r.tampil_tujuan,
                angka(h.email as i64),
                angka(h.sudah_ada as i64),
                angka((h.email - h.sudah_ada) as i64),
                format_ukuran(h.byte as f64)
            ));
            self.folder.push(h);
        }
        Ok(())
    }

    fn kunci_tujuan(&mut self, folder: &str) -> Result<HashSet<String>, String> {
        self.dst.lupakan_folder();
        let d = self.dst.coba(Some(folder), |s, n| s.ringkasan(n)).map_err(|e| e.to_string())?;
        Ok(d.into_iter().map(|m| m.kunci).collect())
    }

    /// Menyalin semua folder menurut rencana. Galat per email dicatat lalu dilanjutkan; galat
    /// sambungan yang tidak pulih menghentikan migrasi (aman diulang).
    pub fn salin(&mut self, rencana: &[Rencana]) -> Result<(), String> {
        // Jumlah email semua folder (EXAMINE saja, ringan) untuk bilah progres.
        let aktif: Vec<&Rencana> = rencana.iter().filter(|r| r.lewati.is_none()).collect();
        for r in &aktif {
            self.total_email += self.src.coba(Some(&r.sumber), |_, n| Ok(n as u64)).map_err(|e| e.to_string())?;
        }
        for r in rencana.iter().filter(|r| r.lewati.is_some()) {
            self.folder.push(HasilFolder {
                sumber: r.tampil_sumber.clone(),
                tujuan: r.tampil_tujuan.clone(),
                lewati: r.lewati.clone().unwrap_or_default(),
                ..Default::default()
            });
        }
        let mut terlihat: HashSet<String> = HashSet::new();
        let jumlah = aktif.len();
        for (i, r) in aktif.into_iter().enumerate() {
            judul(&format!("{} -> {}", r.tampil_sumber, r.tampil_tujuan));
            self.progres(&r.tampil_sumber, i + 1, jumlah, true);
            let h = self.salin_folder(r, &mut terlihat, i + 1, jumlah)?;
            let pesan = format!(
                "{} email: {} sudah ada, {} disalin ({}){}",
                angka(h.email as i64),
                angka(h.sudah_ada as i64),
                angka(h.disalin as i64),
                format_ukuran(h.byte as f64),
                if h.gagal > 0 { format!(", {} gagal", angka(h.gagal as i64)) } else { String::new() }
            );
            if h.gagal > 0 {
                awas(&pesan);
            } else {
                ok(&pesan);
            }
        }
        self.progres("", jumlah, jumlah, true);
        Ok(())
    }

    fn salin_folder(&mut self, r: &Rencana, terlihat: &mut HashSet<String>, urutan: usize, jumlah: usize) -> Result<HasilFolder, String> {
        let idx = self.folder.len();
        self.folder.push(HasilFolder { sumber: r.tampil_sumber.clone(), tujuan: r.tampil_tujuan.clone(), ..Default::default() });
        let mut daftar: Vec<Ringkas> = self.src.coba(Some(&r.sumber), |s, n| s.ringkasan(n)).map_err(|e| e.to_string())?;
        let ada_awal = self.dst.coba(None, |s, _| Ok(s.daftar_folder()?.iter().any(|f| f.nama == r.tujuan || (r.tujuan.eq_ignore_ascii_case("INBOX") && f.nama.eq_ignore_ascii_case("INBOX"))))).map_err(|e| e.to_string())?;
        if !ada_awal {
            info(&format!("Membuat folder {} di tujuan.", r.tampil_tujuan));
            self.dst.coba(None, |s, _| s.buat_folder(&r.tujuan)).map_err(|e| format!("folder {} tidak bisa dibuat: {e}", r.tampil_tujuan))?;
        }
        let mut ada = if ada_awal { self.kunci_tujuan(&r.tujuan)? } else { HashSet::new() };

        // Email bertanda hapus (menunggu expunge) dan, untuk arsip Gmail, email yang sudah ada di folder lain.
        let n_awal = daftar.len() as u64;
        daftar.retain(|m| !m.bendera.iter().any(|b| b.eq_ignore_ascii_case("\\Deleted")));
        if r.arsip {
            daftar.retain(|m| !terlihat.contains(&m.kunci));
            self.selesai_email += n_awal - daftar.len() as u64;
        } else {
            self.selesai_email += n_awal - daftar.len() as u64;
            terlihat.extend(daftar.iter().map(|m| m.kunci.clone()));
        }
        self.folder[idx].email = daftar.len() as u64;
        let mut antre: Vec<Ringkas> = Vec::new();
        for m in daftar {
            if ada.contains(&m.kunci) {
                self.folder[idx].sudah_ada += 1;
                self.selesai_email += 1;
            } else {
                antre.push(m);
            }
        }
        if antre.is_empty() {
            return Ok(self.folder[idx].clone());
        }
        info(&format!("{} email akan disalin ({}).", angka(antre.len() as i64), format_ukuran(antre.iter().map(|m| m.ukuran).sum::<u64>() as f64)));

        let mut i = 0;
        while i < antre.len() {
            // Satu batch: sampai BATCH_EMAIL email atau BATCH_BYTE byte (minimal satu email).
            let mut j = i;
            let mut byte = 0;
            while j < antre.len() && j - i < BATCH_EMAIL && (j == i || byte + antre[j].ukuran <= BATCH_BYTE) {
                byte += antre[j].ukuran;
                j += 1;
            }
            let uid: Vec<u32> = antre[i..j].iter().map(|m| m.uid).collect();
            let isi = self.src.coba(Some(&r.sumber), |s, _| s.ambil(&uid)).map_err(|e| e.to_string())?;
            for m in &antre[i..j] {
                if ada.contains(&m.kunci) {
                    // Email ganda di sumber (Message-ID sama): cukup satu di tujuan.
                    self.folder[idx].sudah_ada += 1;
                    self.selesai_email += 1;
                    continue;
                }
                let Some((_, data)) = isi.iter().find(|(u, _)| *u == m.uid) else {
                    self.catat_gagal(idx, &r.tampil_sumber, m, "isinya tidak dikirim server sumber (mungkin baru dihapus)");
                    continue;
                };
                let bendera = bendera_salin(&m.bendera);
                match self.dst.coba(None, |s, _| s.tambah(&r.tujuan, &bendera, &m.tanggal, data)) {
                    Ok(()) => {
                        ada.insert(m.kunci.clone());
                        self.folder[idx].disalin += 1;
                        self.folder[idx].byte += data.len() as u64;
                        self.selesai_email += 1;
                    }
                    Err(Galat::Ditolak(e)) => self.catat_gagal(idx, &r.tampil_sumber, m, &e),
                    Err(e) => return Err(e.to_string()),
                }
                self.progres(&r.tampil_sumber, urutan, jumlah, false);
            }
            i = j;
        }
        self.dst.lupakan_folder();
        Ok(self.folder[idx].clone())
    }

    fn catat_gagal(&mut self, idx: usize, folder: &str, m: &Ringkas, alasan: &str) {
        self.folder[idx].gagal += 1;
        self.selesai_email += 1;
        if self.contoh_gagal.len() < 20 {
            let t = format!("{folder}, email {} ({}): {alasan}", m.uid, format_ukuran(m.ukuran as f64));
            awas(&t);
            self.contoh_gagal.push(t);
        }
    }

    pub fn tutup(&mut self) {
        self.src.tutup();
        self.dst.tutup();
    }

    pub fn jumlah(&self) -> (u64, u64, u64, u64) {
        self.folder.iter().fold((0, 0, 0, 0), |a, f| (a.0 + f.sudah_ada, a.1 + f.disalin, a.2 + f.gagal, a.3 + f.byte))
    }
}

#[cfg(test)]
mod uji {
    use super::*;

    fn f(nama: &str, pemisah: char, atribut: &[&str]) -> Folder {
        Folder { nama: nama.into(), pemisah: Some(pemisah), atribut: atribut.iter().map(|s| s.to_lowercase()).collect() }
    }

    #[test]
    fn alamat_akun() {
        let a = Akun::urai("gmail:Nama@Gmail.com").unwrap();
        assert_eq!((a.jenis, a.host.as_str(), a.port), (Jenis::Gmail, "imap.gmail.com", 993));
        assert_eq!(Akun::urai("outlook:a@outlook.com").unwrap().host, "outlook.office365.com");
        let b = Akun::urai("imap://info@dinas.go.id@Mail.Dinas.go.id").unwrap();
        assert_eq!((b.pengguna.as_str(), b.host.as_str(), b.port, b.keamanan), ("info@dinas.go.id", "mail.dinas.go.id", 993, Keamanan::Ssl));
        let c = Akun::urai("imap+starttls://budi@10.0.0.5:1143").unwrap();
        assert_eq!((c.port, c.keamanan), (1143, Keamanan::StartTls));
        assert_eq!(Akun::urai("imap+polos://u@h").unwrap().port, 143);
        for s in ["gmail:bukan-email", "ftp://x@y", "imap://tanpa-server", "imap://u@h:99999", "nama@gmail.com"] {
            assert!(Akun::urai(s).is_err(), "{s}");
        }
        for s in ["gmail:a@gmail.com", "outlook:a@b.c", "imap://u@h", "imap+starttls://u@h", "imap://info@x.id@mail.x.id:2993"] {
            assert_eq!(Akun::urai(s).unwrap().alamat(), s);
        }
        assert!(client_microsoft_sah("0c3f1a2b-1234-4abc-9def-0123456789ab") && !client_microsoft_sah("abc"));
    }

    #[test]
    fn rencana_gmail_ke_cpanel() {
        let sumber = vec![
            f("INBOX", '/', &["\\HasNoChildren"]),
            f("[Gmail]", '/', &["\\Noselect", "\\HasChildren"]),
            f("[Gmail]/All Mail", '/', &["\\All"]),
            f("[Gmail]/Sent Mail", '/', &["\\Sent"]),
            f("[Gmail]/Starred", '/', &["\\Flagged"]),
            f("[Gmail]/Trash", '/', &["\\Trash"]),
            f("[Gmail]/Spam", '/', &["\\Junk"]),
            f("Kerja/Proyek 2.0", '/', &[]),
            f("Arsip &AOk-t&AOk-", '/', &[]),
        ];
        let t = Tujuan {
            awalan: "INBOX.".into(),
            pemisah: '.',
            folder: vec![f("INBOX", '.', &[]), f("INBOX.Sent", '.', &["\\Sent"]), f("INBOX.Trash", '.', &[])],
        };
        let r = susun_rencana(&sumber, "", &t, Jenis::Gmail, &OpsiMigrasi { sertakan_sampah: false, arsip_gmail: true });
        let peta: Vec<(&str, &str, bool, bool)> = r.iter().map(|x| (x.tampil_sumber.as_str(), x.tampil_tujuan.as_str(), x.lewati.is_none(), x.ada)).collect();
        assert_eq!(
            peta,
            vec![
                ("INBOX", "INBOX", true, true),
                ("[Gmail]/Sent Mail", "INBOX.Sent", true, true),
                ("Arsip été", "INBOX.Arsip été", true, false),
                ("Kerja/Proyek 2.0", "INBOX.Kerja.Proyek 2_0", true, false),
                ("[Gmail]/Spam", "INBOX.Junk", false, false),
                ("[Gmail]/Starred", "INBOX.Starred", false, false),
                ("[Gmail]/Trash", "INBOX.Trash", false, true),
                ("[Gmail]/All Mail", "INBOX.Archive", true, false),
            ]
        );
        assert!(r.last().unwrap().arsip);
        // Nama tujuan dikirim dalam UTF-7 IMAP.
        assert_eq!(r[2].tujuan, "INBOX.Arsip &AOk-t&AOk-");
    }

    #[test]
    fn rencana_outlook_tanpa_special_use() {
        let sumber = vec![
            f("INBOX", '/', &[]),
            f("Sent Items", '/', &[]),
            f("Deleted Items", '/', &[]),
            f("Calendar", '/', &[]),
            f("Calendar/Birthdays", '/', &[]),
            f("Proyek/Sent Items", '/', &[]),
        ];
        let t = Tujuan { awalan: String::new(), pemisah: '/', folder: vec![f("INBOX", '/', &[]), f("Sent", '/', &[])] };
        let r = susun_rencana(&sumber, "", &t, Jenis::Outlook, &OpsiMigrasi { sertakan_sampah: true, arsip_gmail: true });
        let cari = |s: &str| r.iter().find(|x| x.tampil_sumber == s).unwrap();
        assert_eq!(cari("Sent Items").tampil_tujuan, "Sent");
        assert!(cari("Sent Items").ada);
        assert_eq!(cari("Deleted Items").tampil_tujuan, "Trash");
        assert!(cari("Deleted Items").lewati.is_none());
        assert!(cari("Calendar").lewati.is_some() && cari("Calendar/Birthdays").lewati.is_some());
        // Subfolder bernama "Sent Items" bukan folder Terkirim.
        assert_eq!(cari("Proyek/Sent Items").tampil_tujuan, "Proyek/Sent Items");
    }

    #[test]
    fn awalan_sumber_inbox_titik() {
        // Sumber cPanel (INBOX.) ke server tanpa awalan.
        let sumber = vec![f("INBOX", '.', &[]), f("INBOX.Laporan.2024", '.', &[]), f("INBOX.Drafts", '.', &[])];
        let t = Tujuan { awalan: String::new(), pemisah: '/', folder: vec![f("INBOX", '/', &[])] };
        let r = susun_rencana(&sumber, "INBOX.", &t, Jenis::Imap, &OpsiMigrasi::default());
        let tujuan: Vec<&str> = r.iter().map(|x| x.tampil_tujuan.as_str()).collect();
        assert_eq!(tujuan, vec!["INBOX", "Drafts", "Laporan/2024"]);
    }
}
