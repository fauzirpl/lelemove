//! Berkas mbox (Google Takeout, Thunderbird, Apple Mail, mutt) sebagai sumber pemulihan email.
//!
//! Berkas dibaca sekali untuk mencatat posisi dan header setiap email (isinya tidak disimpan di
//! memori, jadi berkas berukuran puluhan GB pun bisa), lalu isi email diambil satu per satu saat
//! disalin. Google Takeout menaruh semua email di satu berkas dengan label Gmail di header
//! X-Gmail-Labels: label menjadi folder seperti saat migrasi langsung dari Gmail, dan email tanpa
//! label (diarsipkan) masuk ke folder Archive. Thunderbird menyimpan satu berkas mbox per folder
//! (tanpa ekstensi; subfolder di direktori "<nama>.sbd").

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use super::email_lokal::{crlf, Waktu};
use super::imap::{kunci_pesan, urai_header, Folder, Ringkas};

/// Header satu email dibaca paling banyak sekian byte.
const MAKS_HEADER: usize = 256 * 1024;
/// Kabar kemajuan membaca setiap sekian byte.
const LAPOR_SETIAP: u64 = 512 * 1024 * 1024;

struct Entri {
    berkas: u32,
    posisi: u64,
    panjang: u64,
    folder: Vec<u32>,
    bendera: Vec<String>,
    tanggal: String,
    kunci: String,
}

/// Email yang sedang dibaca: posisi isi (sesudah baris "From ") dan header-nya.
struct Calon {
    posisi: u64,
    from: String,
    header: Vec<u8>,
    di_header: bool,
}

pub struct Mbox {
    pub akar: PathBuf,
    berkas: Vec<PathBuf>,
    folder: Vec<String>,
    id_folder: HashMap<String, u32>,
    entri: Vec<Entri>,
    /// Ada email berlabel Gmail (Google Takeout).
    pub takeout: bool,
    /// Email dilewati: percakapan chat Gmail, atau bertanda terhapus.
    pub dilewati: u64,
    buka: Option<(u32, File)>,
}

/// Baris pemisah "From pengirim tanggal" (bukan sekadar kalimat yang diawali "From ").
fn tampak_from(baris: &[u8]) -> bool {
    let t = String::from_utf8_lossy(baris);
    let token: Vec<&str> = t.split_whitespace().collect();
    token.len() >= 4
        && token.iter().any(|x| x.len() >= 4 && x.matches(':').count() >= 1 && x.split(':').all(|b| !b.is_empty() && b.bytes().all(|c| c.is_ascii_digit())))
        && token.iter().any(|x| x.len() == 4 && x.bytes().all(|c| c.is_ascii_digit()))
}

/// Berkas yang tampak seperti mbox: berekstensi .mbox, atau tanpa ekstensi dan diawali "From ".
pub fn tampak_mbox(p: &Path) -> bool {
    match p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()) {
        Some(e) if e == "mbox" || e == "mbx" => true,
        Some(_) => false,
        None => {
            let mut awal = [0u8; 5];
            File::open(p).and_then(|mut f| f.read_exact(&mut awal)).is_ok() && &awal == b"From "
        }
    }
}

/// Nama folder dari jalur berkas mbox relatif: "Kerja.sbd/Proyek" -> "Kerja/Proyek", "Inbox.mbox" -> "Inbox".
fn nama_folder(rel: &Path) -> String {
    let mut k: Vec<String> = rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    if let Some(akhir) = k.last_mut() {
        for ext in [".mbox", ".mbx"] {
            if akhir.len() > ext.len() && akhir.to_ascii_lowercase().ends_with(ext) {
                akhir.truncate(akhir.len() - ext.len());
            }
        }
    }
    for x in k.iter_mut() {
        if x.to_ascii_lowercase().ends_with(".sbd") {
            x.truncate(x.len() - 4);
        }
    }
    k.retain(|x| !x.is_empty());
    k.join("/")
}

/// Label Gmail (Google Takeout) -> folder; None untuk label sistem yang bukan folder.
fn folder_label(label: &str) -> Option<String> {
    let l = label.trim();
    let k = l.to_lowercase();
    let baku = match k.as_str() {
        "inbox" | "kotak masuk" => "INBOX",
        "sent" | "sent mail" | "terkirim" => "Sent",
        "drafts" | "draft" | "draf" => "Drafts",
        "spam" => "Junk",
        "trash" | "sampah" => "Trash",
        "opened" | "unread" | "important" | "starred" | "archived" | "chat" | "chats" | "muted" | "dibuka" | "belum dibaca" | "penting"
        | "berbintang" | "diarsipkan" => return None,
        _ if k.starts_with("category ") || k.starts_with("kategori ") || k.starts_with("imap_") || k.is_empty() => return None,
        _ => return Some(l.to_string()),
    };
    Some(baku.to_string())
}

/// Isi X-Gmail-Labels dipisah koma; label berkoma ditulis dalam tanda kutip.
fn pisah_label(s: &str) -> Vec<String> {
    let mut v = Vec::new();
    let mut kini = String::new();
    let mut kutip = false;
    for c in s.chars() {
        match c {
            '"' => kutip = !kutip,
            ',' if !kutip => v.push(std::mem::take(&mut kini)),
            _ => kini.push(c),
        }
    }
    v.push(kini);
    v.into_iter().map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()
}

/// mboxrd: baris ">From ", ">>From ", ... kehilangan satu ">" (dilindungi saat ditulis ke mbox).
fn buka_lindungan(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    for baris in data.split_inclusive(|&b| b == b'\n') {
        let n = baris.iter().take_while(|&&b| b == b'>').count();
        if n > 0 && baris[n..].starts_with(b"From ") {
            out.extend_from_slice(&baris[1..]);
        } else {
            out.extend_from_slice(baris);
        }
    }
    out
}

impl Mbox {
    /// Membaca satu berkas mbox, atau semua berkas mbox di dalam folder (rekursif). `arsip`: email
    /// Takeout tanpa label folder masuk ke "Archive" (bila tidak, dilewati).
    pub fn buka(akar: &Path, arsip: bool, mut kabar: impl FnMut(&str)) -> io::Result<Mbox> {
        let mut m = Mbox {
            akar: akar.to_path_buf(),
            berkas: Vec::new(),
            folder: Vec::new(),
            id_folder: HashMap::new(),
            entri: Vec::new(),
            takeout: false,
            dilewati: 0,
            buka: None,
        };
        let mut daftar: Vec<(PathBuf, String)> = Vec::new();
        if akar.is_file() {
            daftar.push((akar.to_path_buf(), nama_folder(Path::new(akar.file_name().unwrap_or_default()))));
        } else {
            let mut tumpuk = vec![akar.to_path_buf()];
            while let Some(d) = tumpuk.pop() {
                for e in fs::read_dir(&d)? {
                    let e = e?;
                    let p = e.path();
                    let t = e.file_type()?;
                    if t.is_dir() {
                        tumpuk.push(p);
                    } else if t.is_file() && tampak_mbox(&p) {
                        let rel = p.strip_prefix(akar).unwrap_or(&p).to_path_buf();
                        daftar.push((p, nama_folder(&rel)));
                    }
                }
            }
            daftar.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));
        }
        if daftar.is_empty() {
            return Err(io::Error::new(io::ErrorKind::NotFound, format!("tidak ada berkas mbox di {}", akar.display())));
        }
        for (i, (p, nama)) in daftar.into_iter().enumerate() {
            let f = m.id(&nama);
            m.berkas.push(p.clone());
            m.pindai(i as u32, &p, f, arsip, &mut kabar)?;
        }
        Ok(m)
    }

    fn id(&mut self, nama: &str) -> u32 {
        if let Some(i) = self.id_folder.get(nama) {
            return *i;
        }
        let i = self.folder.len() as u32;
        self.folder.push(nama.to_string());
        self.id_folder.insert(nama.to_string(), i);
        i
    }

    fn pindai(&mut self, idx: u32, p: &Path, folder_berkas: u32, arsip: bool, kabar: &mut impl FnMut(&str)) -> io::Result<()> {
        let f = File::open(p)?;
        let total = f.metadata()?.len();
        let mut r = BufReader::with_capacity(1 << 20, f);
        let mut baris = Vec::with_capacity(4096);
        let mut pos: u64 = 0;
        let mut kosong_sebelum = true;
        let mut panjang_kosong: u64 = 0;
        let mut kini: Option<Calon> = None;
        let mut lapor = LAPOR_SETIAP;
        loop {
            baris.clear();
            let n = r.read_until(b'\n', &mut baris)? as u64;
            if n == 0 {
                break;
            }
            let awal = pos;
            pos += n;
            if kosong_sebelum && baris.starts_with(b"From ") && tampak_from(&baris) {
                if let Some(c) = kini.take() {
                    self.selesai(c, idx, awal - panjang_kosong, folder_berkas, arsip);
                }
                kini = Some(Calon { posisi: pos, from: String::from_utf8_lossy(&baris).trim().to_string(), header: Vec::new(), di_header: true });
                kosong_sebelum = false;
                panjang_kosong = 0;
                continue;
            }
            let kosong = baris == b"\n" || baris == b"\r\n";
            if let Some(c) = kini.as_mut() {
                if c.di_header {
                    if kosong {
                        c.di_header = false;
                    } else if c.header.len() < MAKS_HEADER {
                        c.header.extend_from_slice(&baris);
                    }
                }
            }
            kosong_sebelum = kosong;
            panjang_kosong = if kosong { n } else { 0 };
            if pos >= lapor {
                kabar(&format!("Membaca {}: {} dari {} MB", p.file_name().unwrap_or_default().to_string_lossy(), pos >> 20, total >> 20));
                lapor += LAPOR_SETIAP;
            }
        }
        if let Some(c) = kini.take() {
            self.selesai(c, idx, pos - panjang_kosong, folder_berkas, arsip);
        }
        Ok(())
    }

    fn selesai(&mut self, c: Calon, berkas: u32, akhir: u64, folder_berkas: u32, arsip: bool) {
        if akhir <= c.posisi {
            return;
        }
        let h = urai_header(&c.header);
        let ambil = |k: &str| h.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
        let mut baca: Option<bool> = None;
        let (mut dibalas, mut bintang, mut draf, mut hapus) = (false, false, false, false);
        if let Some(v) = ambil("x-mozilla-status").and_then(|x| u32::from_str_radix(x.trim(), 16).ok()) {
            baca = Some(v & 0x1 != 0);
            dibalas |= v & 0x2 != 0;
            bintang |= v & 0x4 != 0;
            hapus |= v & 0x8 != 0;
        }
        if let Some(s) = ambil("status") {
            baca = baca.or(Some(s.contains('R')));
        }
        if let Some(s) = ambil("x-status") {
            dibalas |= s.contains('A');
            bintang |= s.contains('F');
            draf |= s.contains('T');
            hapus |= s.contains('D');
        }
        let folder = match ambil("x-gmail-labels") {
            Some(lbl) => {
                self.takeout = true;
                let label = pisah_label(lbl);
                let kecil: Vec<String> = label.iter().map(|l| l.to_lowercase()).collect();
                if kecil.iter().any(|l| l == "chat" || l == "chats") {
                    // Percakapan Google Chat/Hangouts, bukan email.
                    self.dilewati += 1;
                    return;
                }
                baca = baca.or(Some(!kecil.iter().any(|l| l == "unread" || l == "belum dibaca")));
                bintang |= kecil.iter().any(|l| l == "starred" || l == "berbintang");
                let mut nama: Vec<String> = Vec::new();
                for l in &label {
                    if let Some(f) = folder_label(l) {
                        if !nama.contains(&f) {
                            nama.push(f);
                        }
                    }
                }
                if nama.is_empty() {
                    if !arsip {
                        self.dilewati += 1;
                        return;
                    }
                    nama.push("Archive".into());
                }
                draf |= nama.iter().any(|x| x == "Drafts");
                nama.iter().map(|x| self.id(x)).collect()
            }
            None => vec![folder_berkas],
        };
        if hapus {
            self.dilewati += 1;
            return;
        }
        let tanggal = Waktu::urai_asctime(&c.from.split_whitespace().skip(2).collect::<Vec<_>>())
            .or_else(|| ambil("date").and_then(Waktu::urai_rfc2822))
            .map(|w| w.internal())
            .unwrap_or_default();
        let mut bendera = Vec::new();
        for (ya, b) in [(baca.unwrap_or(true), "\\Seen"), (dibalas, "\\Answered"), (bintang, "\\Flagged"), (draf, "\\Draft")] {
            if ya {
                bendera.push(b.to_string());
            }
        }
        self.entri.push(Entri { berkas, posisi: c.posisi, panjang: akhir - c.posisi, folder, bendera, tanggal, kunci: kunci_pesan(&c.header) });
    }

    pub fn jumlah_email(&self) -> usize {
        self.entri.len()
    }

    /// Folder yang berisi email.
    pub fn daftar_folder(&self) -> Vec<Folder> {
        let mut isi = vec![0u64; self.folder.len()];
        for e in &self.entri {
            for f in &e.folder {
                isi[*f as usize] += 1;
            }
        }
        self.folder
            .iter()
            .enumerate()
            .filter(|(i, _)| isi[*i] > 0)
            .map(|(_, n)| Folder { nama: n.clone(), pemisah: Some('/'), atribut: Vec::new() })
            .collect()
    }

    /// Ringkasan email di satu folder; UID = nomor urut email di seluruh mbox (1, 2, ...).
    pub fn ringkasan(&self, nama: &str) -> Vec<Ringkas> {
        let Some(&id) = self.id_folder.get(nama) else { return Vec::new() };
        self.entri
            .iter()
            .enumerate()
            .filter(|(_, e)| e.folder.contains(&id))
            .map(|(i, e)| Ringkas { uid: i as u32 + 1, bendera: e.bendera.clone(), tanggal: e.tanggal.clone(), ukuran: e.panjang, kunci: e.kunci.clone() })
            .collect()
    }

    pub fn jumlah(&self, nama: &str) -> u64 {
        self.id_folder.get(nama).map(|id| self.entri.iter().filter(|e| e.folder.contains(id)).count() as u64).unwrap_or(0)
    }

    /// Isi email (CRLF, lindungan ">From " dibuka) menurut UID dari `ringkasan`.
    pub fn ambil(&mut self, uid: &[u32]) -> io::Result<Vec<(u32, Vec<u8>)>> {
        let mut hasil = Vec::with_capacity(uid.len());
        for &u in uid {
            let Some(e) = self.entri.get((u as usize).wrapping_sub(1)) else { continue };
            let (berkas, posisi, panjang) = (e.berkas, e.posisi, e.panjang);
            if !self.buka.as_ref().is_some_and(|(b, _)| *b == berkas) {
                self.buka = Some((berkas, File::open(&self.berkas[berkas as usize])?));
            }
            let f = &mut self.buka.as_mut().expect("baru dibuka").1;
            f.seek(SeekFrom::Start(posisi))?;
            let mut data = vec![0u8; panjang as usize];
            f.read_exact(&mut data)?;
            hasil.push((u, crlf(&buka_lindungan(&data))));
        }
        Ok(hasil)
    }
}

#[cfg(test)]
mod uji {
    use super::*;

    fn tulis(p: &Path, isi: &str) {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, isi).unwrap();
    }

    #[test]
    fn takeout_label_jadi_folder() {
        let d = std::env::temp_dir().join(format!("lelemove-uji-mbox-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        let p = d.join("All mail Including Spam and Trash.mbox");
        tulis(
            &p,
            "From 1700000001@xxx Fri Jan 05 10:30:00 +0000 2024\r\n\
             X-Gmail-Labels: Inbox,Opened,Category Personal,\"Kerja/Proyek, 2024\"\r\n\
             Message-ID: <satu@x>\r\n\
             Subject: Halo\r\n\
             \r\n\
             Isi satu\r\n\
             >From kalimat yang dilindungi\r\n\
             \r\n\
             From 1700000002@xxx Sat Jan 06 08:00:00 +0000 2024\r\n\
             X-Gmail-Labels: Unread,Starred,Archived\r\n\
             Message-ID: <dua@x>\r\n\
             \r\n\
             Isi dua\r\n\
             \r\n\
             From 1700000003@xxx Sat Jan 06 09:00:00 +0000 2024\r\n\
             X-Gmail-Labels: Chat\r\n\
             \r\n\
             obrolan\r\n\
             \r\n\
             From 1700000004@xxx Sun Jan 07 07:00:00 +0000 2024\r\n\
             X-Gmail-Labels: Sent,Opened\r\n\
             Message-ID: <tiga@x>\r\n\
             \r\n\
             Isi tiga tanpa baris kosong di akhir",
        );
        let mut m = Mbox::buka(&p, true, |_| {}).unwrap();
        assert!(m.takeout);
        assert_eq!((m.jumlah_email(), m.dilewati), (3, 1));
        let f: Vec<String> = m.daftar_folder().into_iter().map(|f| f.nama).collect();
        // Folder bawaan berkas (nama berkas) kosong karena semua email berlabel, jadi tidak tampil.
        assert_eq!(f, vec!["INBOX", "Kerja/Proyek, 2024", "Archive", "Sent"]);
        let inbox = m.ringkasan("INBOX");
        assert_eq!(inbox.len(), 1);
        assert_eq!((inbox[0].kunci.as_str(), inbox[0].tanggal.as_str()), ("id:<satu@x>", "05-Jan-2024 10:30:00 +0000"));
        assert_eq!(inbox[0].bendera, vec!["\\Seen"]);
        assert_eq!(m.ringkasan("Kerja/Proyek, 2024")[0].uid, inbox[0].uid);
        let arsip = m.ringkasan("Archive");
        assert_eq!(arsip[0].bendera, vec!["\\Flagged"]);
        let isi = m.ambil(&[inbox[0].uid, m.ringkasan("Sent")[0].uid]).unwrap();
        assert_eq!(
            String::from_utf8(isi[0].1.clone()).unwrap(),
            "X-Gmail-Labels: Inbox,Opened,Category Personal,\"Kerja/Proyek, 2024\"\r\nMessage-ID: <satu@x>\r\nSubject: Halo\r\n\r\nIsi satu\r\nFrom kalimat yang dilindungi\r\n"
        );
        assert!(String::from_utf8(isi[1].1.clone()).unwrap().ends_with("Isi tiga tanpa baris kosong di akhir"));
        // Tanpa arsip: email berlabel sistem saja dilewati.
        let m2 = Mbox::buka(&p, false, |_| {}).unwrap();
        assert_eq!((m2.jumlah_email(), m2.dilewati), (2, 2));
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn thunderbird_satu_berkas_per_folder() {
        let d = std::env::temp_dir().join(format!("lelemove-uji-tb-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        // LF saja (bukan CRLF), seperti Thunderbird di macOS/Linux.
        tulis(
            &d.join("Inbox"),
            "From - Thu Jan  4 09:00:00 2024\nX-Mozilla-Status: 0001\nMessage-ID: <a@x>\nDate: Thu, 4 Jan 2024 16:00:00 +0700\n\nhalo\n\n\
             From - Thu Jan  4 10:00:00 2024\nX-Mozilla-Status: 0009\nMessage-ID: <terhapus@x>\n\nx\n\n\
             From - Fri Jan  5 11:00:00 2024\nX-Mozilla-Status: 0006\nMessage-ID: <b@x>\n\nFrom the desk of budi\n",
        );
        tulis(&d.join("Inbox.msf"), "// indeks Thunderbird, bukan mbox");
        tulis(&d.join("Kerja.sbd").join("Laporan"), "From - Sat Jan  6 12:00:00 2024\nStatus: RO\nX-Status: F\nSubject: lap\n\nisi\n");
        let mut m = Mbox::buka(&d, true, |_| {}).unwrap();
        assert!(!m.takeout);
        let f: Vec<String> = m.daftar_folder().into_iter().map(|f| f.nama).collect();
        assert_eq!(f, vec!["Inbox", "Kerja/Laporan"]);
        let r = m.ringkasan("Inbox");
        assert_eq!(r.len(), 2, "email bertanda terhapus dilewati");
        assert_eq!(r[0].bendera, vec!["\\Seen"]);
        assert_eq!(r[1].bendera, vec!["\\Answered", "\\Flagged"]);
        assert_eq!(r[0].tanggal, "04-Jan-2024 09:00:00 +0000");
        let l = m.ringkasan("Kerja/Laporan");
        assert_eq!(l[0].bendera, vec!["\\Seen", "\\Flagged"]);
        // "From the desk" sesudah baris kosong bukan pemisah (tidak ada jam & tahun); LF menjadi CRLF.
        let isi = m.ambil(&[r[1].uid]).unwrap();
        assert_eq!(String::from_utf8(isi[0].1.clone()).unwrap(), "X-Mozilla-Status: 0006\r\nMessage-ID: <b@x>\r\n\r\nFrom the desk of budi\r\n");
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn bagian_kecil() {
        assert!(tampak_from(b"From MAILER-DAEMON Fri Jul  8 12:08:34 2011\n"));
        assert!(tampak_from(b"From - Thu Jan  4 09:00:00 2024\r\n"));
        assert!(!tampak_from(b"From the desk of budi\n"));
        assert_eq!(pisah_label("Inbox, \"a,b\" ,Sent,"), vec!["Inbox", "a,b", "Sent"]);
        assert_eq!(folder_label("Category Promotions"), None);
        assert_eq!(folder_label("Spam").as_deref(), Some("Junk"));
        assert_eq!(nama_folder(Path::new("Kerja.sbd/Proyek.sbd/2024.mbox")), "Kerja/Proyek/2024");
        assert_eq!(buka_lindungan(b">From a\n>>From b\n> From c\n"), b"From a\n>From b\n> From c\n");
    }
}
