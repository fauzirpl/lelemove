//! Cadangan email di komputer: satu berkas .eml per email, dalam folder yang mengikuti kotak surat
//! (INBOX, Sent, Kerja/Proyek, ...). Status baca/bintang, tanggal terima asli, dan kunci pencocokan
//! (Message-ID) dicatat di `.lelemove-indeks.jsonl` per folder, satu baris per email, sehingga
//! cadangan bisa diperbarui (hanya email baru yang diunduh) dan dipulihkan ke server lain lengkap
//! dengan statusnya. Berkas .eml bisa dibuka langsung di Outlook, Thunderbird, atau Windows Mail;
//! tanggal berkasnya diatur sama dengan tanggal terima email.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::imap::{kunci_pesan, urai_header, Folder, Ringkas};
use super::kode::{dari_base64, fnv64};

/// Catatan per folder: satu baris JSON per email.
pub const INDEKS: &str = ".lelemove-indeks.jsonl";
const BULAN: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
/// Header dibaca paling banyak sekian byte.
const MAKS_HEADER: usize = 256 * 1024;

/// Teks yang berupa jalur di komputer, bukan akun email: "D:\...", "\\server\...", "/...", "~/...".
pub fn tampak_jalur(s: &str) -> bool {
    let b = s.as_bytes();
    (b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && matches!(b[2], b'\\' | b'/'))
        || s.starts_with("\\\\")
        || s.starts_with('/')
        || s.starts_with("~/")
        || s.starts_with("~\\")
        || s.starts_with("./")
        || s.starts_with(".\\")
}

// ------------------------------------------------------------------ waktu (tanpa crate tanggal)

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Waktu {
    pub tahun: i64,
    pub bulan: u32,
    pub hari: u32,
    pub jam: u32,
    pub menit: u32,
    pub detik: u32,
    /// Selisih terhadap UTC, dalam menit.
    pub zona: i32,
}

/// Hari sejak 1970-01-01 (algoritme kalender sipil Howard Hinnant).
fn hari_sipil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn sipil(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

fn bulan_dari(s: &str) -> Option<u32> {
    let s = s.get(..3)?;
    BULAN.iter().position(|b| b.eq_ignore_ascii_case(s)).map(|i| i as u32 + 1)
}

fn zona_dari(s: &str) -> Option<i32> {
    let s = s.trim();
    if let Some(r) = s.strip_prefix('+').map(|r| (1, r)).or_else(|| s.strip_prefix('-').map(|r| (-1, r))) {
        let (tanda, angka) = r;
        if angka.len() == 4 && angka.bytes().all(|c| c.is_ascii_digit()) {
            let j: i32 = angka[..2].parse().ok()?;
            let m: i32 = angka[2..].parse().ok()?;
            return Some(tanda * (j * 60 + m));
        }
        return None;
    }
    let j = match s.to_ascii_uppercase().as_str() {
        "GMT" | "UT" | "UTC" | "Z" => 0,
        "EDT" => -4,
        "EST" | "CDT" => -5,
        "CST" | "MDT" => -6,
        "MST" | "PDT" => -7,
        "PST" => -8,
        _ => return None,
    };
    Some(j * 60)
}

fn jam_dari(s: &str) -> Option<(u32, u32, u32)> {
    let b: Vec<&str> = s.split(':').collect();
    if !(2..=3).contains(&b.len()) {
        return None;
    }
    let j = b[0].parse().ok()?;
    let m = b[1].parse().ok()?;
    let d = if b.len() == 3 { b[2].parse().ok()? } else { 0 };
    Some((j, m, d))
}

impl Waktu {
    fn sah(self) -> Option<Waktu> {
        ((1900..=9999).contains(&self.tahun)
            && (1..=12).contains(&self.bulan)
            && (1..=31).contains(&self.hari)
            && self.jam < 24
            && self.menit < 60
            && self.detik <= 60)
            .then_some(self)
    }

    /// INTERNALDATE IMAP: "17-Jul-1996 02:44:25 -0700" (hari bisa diawali spasi).
    pub fn urai_internal(s: &str) -> Option<Waktu> {
        let t: Vec<&str> = s.split_whitespace().collect();
        if t.len() != 3 {
            return None;
        }
        let d: Vec<&str> = t[0].split('-').collect();
        if d.len() != 3 {
            return None;
        }
        let (jam, menit, detik) = jam_dari(t[1])?;
        Waktu { tahun: d[2].parse().ok()?, bulan: bulan_dari(d[1])?, hari: d[0].parse().ok()?, jam, menit, detik, zona: zona_dari(t[2])? }.sah()
    }

    /// Header Date (RFC 5322): "Mon, 1 Jan 2024 10:00:00 +0700", hari & detik boleh tidak ada.
    pub fn urai_rfc2822(s: &str) -> Option<Waktu> {
        // Buang komentar "(WIB)".
        let mut bersih = String::new();
        let mut dalam = 0;
        for c in s.chars() {
            match c {
                '(' => dalam += 1,
                ')' if dalam > 0 => dalam -= 1,
                _ if dalam == 0 => bersih.push(if c == ',' { ' ' } else { c }),
                _ => {}
            }
        }
        let mut t: Vec<&str> = bersih.split_whitespace().collect();
        if t.first().is_some_and(|x| !x.starts_with(|c: char| c.is_ascii_digit())) {
            t.remove(0);
        }
        if t.len() < 4 {
            return None;
        }
        let mut tahun: i64 = t[2].parse().ok()?;
        if tahun < 100 {
            tahun += if tahun < 50 { 2000 } else { 1900 };
        }
        let (jam, menit, detik) = jam_dari(t[3])?;
        let zona = t.get(4).and_then(|z| zona_dari(z)).unwrap_or(0);
        Waktu { tahun, bulan: bulan_dari(t[1])?, hari: t[0].parse().ok()?, jam, menit, detik, zona }.sah()
    }

    /// Baris pemisah mbox sesudah "From <pengirim>": "Fri Jul  8 12:08:34 2011" atau
    /// "Fri Jan 05 10:30:00 +0000 2024" (Google Takeout). Tanpa zona dianggap UTC.
    pub fn urai_asctime(t: &[&str]) -> Option<Waktu> {
        let i = t.iter().position(|x| x.len() >= 3 && bulan_dari(x).is_some() && x.chars().all(|c| c.is_ascii_alphabetic()))?;
        let bulan = bulan_dari(t[i])?;
        let hari = t.get(i + 1)?.parse().ok()?;
        let (jam, menit, detik) = jam_dari(t.get(i + 2)?)?;
        let (zona, tahun) = match zona_dari(t.get(i + 3)?) {
            Some(z) => (z, t.get(i + 4)?.parse().ok()?),
            None => (0, t.get(i + 3)?.parse().ok()?),
        };
        Waktu { tahun, bulan, hari, jam, menit, detik, zona }.sah()
    }

    pub fn dari_epoch(detik: i64) -> Waktu {
        let (tahun, bulan, hari) = sipil(detik.div_euclid(86_400));
        let s = detik.rem_euclid(86_400) as u32;
        Waktu { tahun, bulan, hari, jam: s / 3600, menit: s / 60 % 60, detik: s % 60, zona: 0 }
    }

    pub fn epoch(&self) -> i64 {
        hari_sipil(self.tahun, self.bulan, self.hari) * 86_400 + i64::from(self.jam * 3600 + self.menit * 60 + self.detik) - i64::from(self.zona) * 60
    }

    /// Bentuk INTERNALDATE untuk APPEND.
    pub fn internal(&self) -> String {
        let z = self.zona.abs();
        format!(
            "{:02}-{}-{:04} {:02}:{:02}:{:02} {}{:02}{:02}",
            self.hari,
            BULAN[(self.bulan - 1) as usize],
            self.tahun,
            self.jam,
            self.menit,
            self.detik,
            if self.zona < 0 { '-' } else { '+' },
            z / 60,
            z % 60
        )
    }

    /// "2024-01-05 1030": awal nama berkas .eml (waktu setempat pengirim/server, apa adanya).
    pub fn untuk_nama(&self) -> String {
        format!("{:04}-{:02}-{:02} {:02}{:02}", self.tahun, self.bulan, self.hari, self.jam, self.menit)
    }
}

fn waktu_sistem(t: SystemTime) -> Option<Waktu> {
    t.duration_since(UNIX_EPOCH).ok().map(|d| Waktu::dari_epoch(d.as_secs() as i64))
}

// ------------------------------------------------------------------ header

/// Bagian header email (sampai baris kosong pertama), paling banyak MAKS_HEADER byte.
pub fn potong_header(data: &[u8]) -> &[u8] {
    let batas = data.len().min(MAKS_HEADER);
    let d = &data[..batas];
    let mut i = 0;
    while i < d.len() {
        if d[i] == b'\n' {
            match d.get(i + 1) {
                Some(b'\n') => return &d[..i + 2],
                Some(b'\r') if d.get(i + 2) == Some(&b'\n') => return &d[..i + 3],
                _ => {}
            }
        }
        i += 1;
    }
    d
}

/// Tanggal dari header Date.
pub fn tanggal_header(header: &[u8]) -> Option<Waktu> {
    urai_header(header).iter().find(|(k, _)| k == "date").and_then(|(_, v)| Waktu::urai_rfc2822(v))
}

/// Baris akhir LF tunggal menjadi CRLF (IMAP APPEND dan berkas .eml memakai CRLF).
pub fn crlf(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + data.len() / 40);
    let mut sebelum = 0u8;
    for &b in data {
        if b == b'\n' && sebelum != b'\r' {
            out.push(b'\r');
        }
        out.push(b);
        sebelum = b;
    }
    out
}

/// Satu "encoded-word" RFC 2047 di awal `s` ("=?utf-8?B?...?="): teks dan panjangnya.
fn kata_kode(s: &str) -> Option<(String, usize)> {
    let isi = s.strip_prefix("=?")?;
    let (charset, sisa) = isi.split_once('?')?;
    let (kode, sisa) = sisa.split_once('?')?;
    let akhir = sisa.find("?=")?;
    let teks = &sisa[..akhir];
    if teks.contains(char::is_whitespace) || charset.contains(char::is_whitespace) {
        return None;
    }
    let byte: Vec<u8> = match kode {
        "B" | "b" => dari_base64(teks)?,
        "Q" | "q" => {
            let b = teks.as_bytes();
            let mut v = Vec::with_capacity(b.len());
            let mut i = 0;
            while i < b.len() {
                match b[i] {
                    b'_' => v.push(b' '),
                    b'=' if i + 2 < b.len() => {
                        let h = std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok());
                        match h {
                            Some(x) => {
                                v.push(x);
                                i += 2;
                            }
                            None => v.push(b'='),
                        }
                    }
                    c => v.push(c),
                }
                i += 1;
            }
            v
        }
        _ => return None,
    };
    let cs = charset.split('*').next().unwrap_or("").to_ascii_lowercase();
    let t = match cs.as_str() {
        "iso-8859-1" | "latin1" | "iso-8859-15" | "windows-1252" | "cp1252" | "us-ascii" => byte.iter().map(|&c| c as char).collect(),
        _ => String::from_utf8_lossy(&byte).into_owned(),
    };
    Some((t, 2 + charset.len() + 1 + kode.len() + 1 + akhir + 2))
}

/// Header yang berisi encoded-word RFC 2047 menjadi teks biasa (untuk nama berkas).
pub fn dekode_header(s: &str) -> String {
    let mut out = String::new();
    let mut sisa = s;
    let mut kode_sebelum = false;
    while let Some(i) = sisa.find("=?") {
        let (sebelum, dari) = sisa.split_at(i);
        match kata_kode(dari) {
            Some((t, n)) => {
                // Spasi di antara dua encoded-word tidak ikut ditampilkan.
                if !(kode_sebelum && sebelum.chars().all(char::is_whitespace)) {
                    out.push_str(sebelum);
                }
                out.push_str(&t);
                sisa = &dari[n..];
                kode_sebelum = true;
            }
            None => {
                out.push_str(sebelum);
                out.push_str("=?");
                sisa = &dari[2..];
                kode_sebelum = false;
            }
        }
    }
    out.push_str(sisa);
    out
}

// ------------------------------------------------------------------ nama berkas & folder

/// Satu komponen nama berkas/folder yang aman di Windows, macOS, dan Linux.
pub fn aman_nama(s: &str, maks: usize) -> String {
    let t: String = s.chars().map(|c| if c.is_control() || "<>:\"/\\|?*".contains(c) { '_' } else { c }).collect();
    let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut t: String = t.chars().take(maks).collect();
    while t.ends_with(['.', ' ']) {
        t.pop();
    }
    if t.is_empty() {
        return "_".into();
    }
    let dasar = t.split('.').next().unwrap_or("").trim().to_ascii_uppercase();
    let khusus = ["CON", "PRN", "AUX", "NUL"].contains(&dasar.as_str())
        || ((dasar.starts_with("COM") || dasar.starts_with("LPT")) && dasar.len() == 4 && dasar.as_bytes()[3].is_ascii_digit());
    if khusus {
        format!("_{t}")
    } else {
        t
    }
}

/// "2024-01-05 1030 Rapat besok [1a2b3c4d].eml": urut menurut tanggal di Explorer/Finder; kode di
/// akhir (dari Message-ID) membedakan email bersubjek sama.
pub fn nama_berkas(waktu: Option<Waktu>, subjek: &str, kunci: &str, ke: u32) -> String {
    let t = waktu.map(|w| w.untuk_nama()).unwrap_or_else(|| "0000-00-00 0000".into());
    let s = aman_nama(if subjek.trim().is_empty() { "(tanpa subjek)" } else { subjek }, 60);
    let ulang = if ke > 1 { format!(" ({ke})") } else { String::new() };
    format!("{t} {s} [{:08x}]{ulang}.eml", fnv64(kunci.as_bytes()) as u32)
}

fn adalah_eml(nama: &str) -> bool {
    nama.len() > 4 && nama[nama.len() - 4..].eq_ignore_ascii_case(".eml")
}

// ------------------------------------------------------------------ indeks & folder cadangan

/// Satu baris indeks.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct Entri {
    pub berkas: String,
    pub kunci: String,
    pub bendera: Vec<String>,
    pub tanggal: String,
    pub ukuran: u64,
}

fn tambah_indeks(dir: &Path, entri: &[Entri]) -> io::Result<()> {
    let mut f = OpenOptions::new().create(true).append(true).open(dir.join(INDEKS))?;
    let mut teks = String::new();
    for e in entri {
        teks.push_str(&serde_json::to_string(e).unwrap_or_default());
        teks.push('\n');
    }
    f.write_all(teks.as_bytes())
}

/// Entri untuk .eml yang belum tercatat (disalin sendiri, atau terputus sebelum indeks ditulis).
fn entri_dari_berkas(p: &Path, nama: &str) -> io::Result<Entri> {
    let f = File::open(p)?;
    let meta = f.metadata()?;
    let mut buf = Vec::new();
    f.take(MAKS_HEADER as u64).read_to_end(&mut buf)?;
    let h = potong_header(&buf);
    let tanggal = tanggal_header(h).or_else(|| meta.modified().ok().and_then(waktu_sistem)).map(|w| w.internal()).unwrap_or_default();
    // Email hasil ekspor biasanya sudah dibaca.
    Ok(Entri { berkas: nama.into(), kunci: kunci_pesan(h), bendera: vec!["\\Seen".into()], tanggal, ukuran: meta.len() })
}

/// Isi satu folder: catatan indeks untuk berkas yang masih ada, ditambah .eml yang belum tercatat.
fn muat_folder(dir: &Path) -> io::Result<Vec<Entri>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut tercatat: HashMap<String, Entri> = HashMap::new();
    let mut urutan: Vec<String> = Vec::new();
    if let Ok(t) = fs::read_to_string(dir.join(INDEKS)) {
        // Baris terakhir yang terpotong (aplikasi berhenti saat menulis) dilewati.
        for b in t.lines() {
            if let Ok(e) = serde_json::from_str::<Entri>(b) {
                if e.berkas.is_empty() {
                    continue;
                }
                if !tercatat.contains_key(&e.berkas) {
                    urutan.push(e.berkas.clone());
                }
                tercatat.insert(e.berkas.clone(), e);
            }
        }
    }
    let mut berkas: Vec<String> = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| adalah_eml(n))
        .collect();
    berkas.sort();
    let ada: HashSet<&String> = berkas.iter().collect();
    let mut hasil: Vec<Entri> = urutan.iter().filter(|n| ada.contains(n)).filter_map(|n| tercatat.get(n).cloned()).collect();
    let baru: Vec<Entri> = berkas.iter().filter(|n| !tercatat.contains_key(*n)).filter_map(|n| entri_dari_berkas(&dir.join(n), n).ok()).collect();
    if !baru.is_empty() {
        // Gagal menulis (mis. cadangan di media baca-saja) tidak masalah: dibaca ulang lain kali.
        let _ = tambah_indeks(dir, &baru);
        hasil.extend(baru);
    }
    Ok(hasil)
}

/// Folder cadangan .eml, sebagai tujuan (cadangkan) maupun sumber (pulihkan).
pub struct Cadangan {
    pub akar: PathBuf,
    /// Nama folder -> direktorinya (dari `daftar_folder`; folder lain: akar + komponen nama).
    peta: HashMap<String, PathBuf>,
    /// Folder yang terakhir dimuat beserta isinya.
    muatan: Option<(String, Vec<Entri>)>,
}

impl Cadangan {
    pub fn baru(akar: PathBuf) -> Cadangan {
        Cadangan { akar, peta: HashMap::new(), muatan: None }
    }

    fn dir(&self, nama: &str) -> PathBuf {
        if let Some(p) = self.peta.get(nama) {
            return p.clone();
        }
        let mut p = self.akar.clone();
        for k in nama.split('/').filter(|k| !k.is_empty()) {
            p.push(k);
        }
        p
    }

    /// Folder yang berisi email (.eml atau indeks), nama relatif terhadap akar dengan pemisah "/".
    /// Email langsung di akar memakai nama folder akar itu sendiri.
    pub fn daftar_folder(&mut self) -> io::Result<Vec<Folder>> {
        self.peta.clear();
        let mut hasil = Vec::new();
        if !self.akar.is_dir() {
            return Ok(hasil);
        }
        let mut tumpuk = vec![(self.akar.clone(), String::new())];
        while let Some((d, nama)) = tumpuk.pop() {
            let mut ada_email = false;
            for e in fs::read_dir(&d)? {
                let e = e?;
                let n = e.file_name().to_string_lossy().into_owned();
                let t = e.file_type()?;
                if t.is_dir() {
                    if !n.starts_with('.') {
                        tumpuk.push((e.path(), if nama.is_empty() { n } else { format!("{nama}/{n}") }));
                    }
                } else if n == INDEKS || adalah_eml(&n) {
                    ada_email = true;
                }
            }
            if ada_email {
                let nm = if nama.is_empty() {
                    self.akar.file_name().map(|x| aman_nama(&x.to_string_lossy(), 80)).unwrap_or_else(|| "INBOX".into())
                } else {
                    nama
                };
                self.peta.insert(nm.clone(), d);
                hasil.push(Folder { nama: nm, pemisah: Some('/'), atribut: Vec::new() });
            }
        }
        hasil.sort_by_key(|f| f.nama.to_lowercase());
        Ok(hasil)
    }

    pub fn ada_folder(&self, nama: &str) -> bool {
        self.dir(nama).is_dir()
    }

    pub fn buat_folder(&mut self, nama: &str) -> io::Result<()> {
        let d = self.dir(nama);
        fs::create_dir_all(&d)?;
        self.peta.insert(nama.to_string(), d);
        Ok(())
    }

    fn muat(&mut self, nama: &str) -> io::Result<&mut Vec<Entri>> {
        if !self.muatan.as_ref().is_some_and(|(n, _)| n == nama) {
            let isi = muat_folder(&self.dir(nama))?;
            self.muatan = Some((nama.to_string(), isi));
        }
        Ok(&mut self.muatan.as_mut().expect("baru dimuat").1)
    }

    pub fn jumlah(&mut self, nama: &str) -> io::Result<u64> {
        Ok(self.muat(nama)?.len() as u64)
    }

    pub fn kunci(&mut self, nama: &str) -> io::Result<HashSet<String>> {
        Ok(self.muat(nama)?.iter().map(|e| e.kunci.clone()).collect())
    }

    /// Ringkasan isi folder; UID = urutan berkas (1, 2, ...).
    pub fn ringkasan(&mut self, nama: &str) -> io::Result<Vec<Ringkas>> {
        Ok(self
            .muat(nama)?
            .iter()
            .enumerate()
            .map(|(i, e)| Ringkas { uid: i as u32 + 1, bendera: e.bendera.clone(), tanggal: e.tanggal.clone(), ukuran: e.ukuran, kunci: e.kunci.clone() })
            .collect())
    }

    /// Isi email menurut UID dari `ringkasan`; berkas yang tidak terbaca tidak disertakan.
    pub fn ambil(&mut self, nama: &str, uid: &[u32]) -> io::Result<Vec<(u32, Vec<u8>)>> {
        let dir = self.dir(nama);
        let entri = self.muat(nama)?;
        let mut hasil = Vec::new();
        for &u in uid {
            let Some(e) = entri.get((u as usize).wrapping_sub(1)) else { continue };
            if let Ok(d) = fs::read(dir.join(&e.berkas)) {
                hasil.push((u, d));
            }
        }
        Ok(hasil)
    }

    /// Menyimpan satu email sebagai .eml (lewat berkas sementara lalu diganti nama, agar tidak ada
    /// .eml setengah jadi), mengatur tanggal berkas, lalu mencatatnya di indeks.
    pub fn simpan(&mut self, nama: &str, bendera: &[String], tanggal: &str, kunci: &str, data: &[u8]) -> io::Result<()> {
        let dir = self.dir(nama);
        fs::create_dir_all(&dir)?;
        let h = potong_header(data);
        let subjek = urai_header(h).iter().find(|(k, _)| k == "subject").map(|(_, v)| dekode_header(v)).unwrap_or_default();
        let waktu = Waktu::urai_internal(tanggal).or_else(|| tanggal_header(h));
        let mut ke = 1;
        let mut berkas = nama_berkas(waktu, &subjek, kunci, ke);
        while dir.join(&berkas).exists() {
            ke += 1;
            berkas = nama_berkas(waktu, &subjek, kunci, ke);
        }
        let sementara = dir.join(format!("{berkas}.sementara"));
        {
            let mut f = File::create(&sementara)?;
            f.write_all(data)?;
            if let Some(w) = waktu.filter(|w| w.epoch() > 0) {
                let _ = f.set_modified(UNIX_EPOCH + Duration::from_secs(w.epoch() as u64));
            }
        }
        fs::rename(&sementara, dir.join(&berkas))?;
        let e = Entri {
            berkas,
            kunci: kunci.to_string(),
            bendera: bendera.to_vec(),
            tanggal: if tanggal.is_empty() { waktu.map(|w| w.internal()).unwrap_or_default() } else { tanggal.to_string() },
            ukuran: data.len() as u64,
        };
        tambah_indeks(&dir, std::slice::from_ref(&e))?;
        if let Some((n, v)) = &mut self.muatan {
            if n == nama {
                v.push(e);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod uji {
    use super::*;

    #[test]
    fn jalur_atau_akun() {
        for s in ["D:\\Cadangan Email", "c:/x", "\\\\nas\\arsip", "/home/u/mail", "~/Email", ".\\cadangan"] {
            assert!(tampak_jalur(s), "{s}");
        }
        for s in ["gmail:a@gmail.com", "imap://u@h", "outlook:x@y.z", "D:", "nama"] {
            assert!(!tampak_jalur(s), "{s}");
        }
    }

    #[test]
    fn waktu_bolak_balik() {
        let w = Waktu::urai_internal(" 7-Jul-1996 02:44:25 -0700").unwrap();
        assert_eq!((w.tahun, w.bulan, w.hari, w.zona), (1996, 7, 7, -420));
        assert_eq!(w.internal(), "07-Jul-1996 02:44:25 -0700");
        assert_eq!(w.epoch(), 836_732_665);
        assert_eq!(Waktu::dari_epoch(836_732_665).internal(), "07-Jul-1996 09:44:25 +0000");
        assert_eq!(Waktu::dari_epoch(0).internal(), "01-Jan-1970 00:00:00 +0000");
        assert_eq!(Waktu::dari_epoch(951_782_400).untuk_nama(), "2000-02-29 0000");
        let d = Waktu::urai_rfc2822("Mon, 1 Jan 2024 10:00:00 +0700 (WIB)").unwrap();
        assert_eq!(d.internal(), "01-Jan-2024 10:00:00 +0700");
        assert_eq!(Waktu::urai_rfc2822("5 Feb 24 08:15 GMT").unwrap().internal(), "05-Feb-2024 08:15:00 +0000");
        assert_eq!(Waktu::urai_rfc2822("Tue, 3 Mar 2020 9:05:01 EST").unwrap().zona, -300);
        assert!(Waktu::urai_rfc2822("kemarin sore").is_none());
        let t = |s: &str| Waktu::urai_asctime(&s.split_whitespace().collect::<Vec<_>>()).map(|w| w.internal());
        assert_eq!(t("Fri Jul  8 12:08:34 2011").unwrap(), "08-Jul-2011 12:08:34 +0000");
        assert_eq!(t("Fri Jan 05 10:30:00 +0000 2024").unwrap(), "05-Jan-2024 10:30:00 +0000");
        assert!(t("Fri Jan 05 2024").is_none());
    }

    #[test]
    fn subjek_dan_nama_berkas() {
        assert_eq!(dekode_header("=?UTF-8?B?UmFwYXQgYmVzb2s=?="), "Rapat besok");
        assert_eq!(dekode_header("=?utf-8?Q?Laporan_=C3=A9t=C3=A9?= 2024"), "Laporan été 2024");
        assert_eq!(dekode_header("=?UTF-8?B?QQ==?= =?UTF-8?B?Qg==?="), "AB");
        assert_eq!(dekode_header("=?iso-8859-1?Q?caf=E9?="), "café");
        assert_eq!(dekode_header("biasa =? bukan"), "biasa =? bukan");
        assert_eq!(aman_nama("Re: a/b\\c <x> | y?*. ", 60), "Re_ a_b_c _x_ _ y__");
        assert_eq!(aman_nama("con.txt", 60), "_con.txt");
        assert_eq!(aman_nama("...", 60), "_");
        let w = Waktu::urai_internal("05-Jan-2024 10:30:00 +0700");
        let n = nama_berkas(w, "Rapat: anggaran", "id:<a@b>", 1);
        assert!(n.starts_with("2024-01-05 1030 Rapat_ anggaran [") && n.ends_with("].eml"), "{n}");
        assert!(nama_berkas(None, "", "k", 2).starts_with("0000-00-00 0000 (tanpa subjek) [") && nama_berkas(None, "", "k", 2).ends_with("] (2).eml"));
        assert_eq!(crlf(b"a\nb\r\nc\n"), b"a\r\nb\r\nc\r\n");
        assert_eq!(potong_header(b"Subject: x\r\n\r\nisi"), b"Subject: x\r\n\r\n");
        assert_eq!(potong_header(b"Subject: x\n\nisi\n"), b"Subject: x\n\n");
    }

    #[test]
    fn cadangan_simpan_lalu_baca() {
        let akar = std::env::temp_dir().join(format!("lelemove-uji-cadangan-{}", std::process::id()));
        let _ = fs::remove_dir_all(&akar);
        let mut c = Cadangan::baru(akar.clone());
        let email = b"Message-ID: <satu@x>\r\nDate: Mon, 1 Jan 2024 10:00:00 +0700\r\nSubject: =?UTF-8?Q?Rapat_=C3=A9?=\r\n\r\nIsi\r\n";
        c.simpan("Kerja/Proyek", &["\\Seen".into(), "\\Flagged".into()], "01-Jan-2024 10:00:00 +0700", "id:<satu@x>", email).unwrap();
        c.simpan("INBOX", &[], "", "id:<dua@x>", b"Subject: tanpa tanggal\r\n\r\nx\r\n").unwrap();
        // .eml yang disalin sendiri ke folder (tanpa catatan indeks) ikut terbaca.
        fs::write(akar.join("INBOX").join("lain.eml"), b"Message-ID: <tiga@x>\r\nDate: 2 Feb 2024 08:00:00 +0000\r\n\r\nz\r\n").unwrap();

        let mut b = Cadangan::baru(akar.clone());
        let f: Vec<String> = b.daftar_folder().unwrap().into_iter().map(|f| f.nama).collect();
        assert_eq!(f, vec!["INBOX", "Kerja/Proyek"]);
        let r = b.ringkasan("Kerja/Proyek").unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!((r[0].kunci.as_str(), r[0].tanggal.as_str()), ("id:<satu@x>", "01-Jan-2024 10:00:00 +0700"));
        assert_eq!(r[0].bendera, vec!["\\Seen", "\\Flagged"]);
        let isi = b.ambil("Kerja/Proyek", &[1]).unwrap();
        assert_eq!(isi[0].1, email);
        let berkas: Vec<String> = fs::read_dir(akar.join("Kerja").join("Proyek")).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        assert!(berkas.iter().any(|n| n.starts_with("2024-01-01 1000 Rapat é [")), "{berkas:?}");
        // Tanggal berkas = tanggal terima email.
        let p = akar.join("Kerja").join("Proyek").join(berkas.iter().find(|n| n.ends_with(".eml")).unwrap());
        let mtime = fs::metadata(p).unwrap().modified().unwrap().duration_since(UNIX_EPOCH).unwrap().as_secs();
        assert_eq!(mtime as i64, Waktu::urai_internal("01-Jan-2024 10:00:00 +0700").unwrap().epoch());
        let k = b.kunci("INBOX").unwrap();
        assert!(k.contains("id:<dua@x>") && k.contains("id:<tiga@x>") && k.len() == 2);
        // Berkas tambahan kini tercatat di indeks; tanggalnya dari header Date.
        let r = Cadangan::baru(akar.clone()).ringkasan("INBOX").unwrap();
        assert!(r.iter().any(|x| x.kunci == "id:<tiga@x>" && x.tanggal == "02-Feb-2024 08:00:00 +0000" && x.bendera == vec!["\\Seen"]));
        assert_eq!(fs::read_to_string(akar.join("INBOX").join(INDEKS)).unwrap().lines().count(), 2);
        fs::remove_dir_all(&akar).unwrap();
    }
}
