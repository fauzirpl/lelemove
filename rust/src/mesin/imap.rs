//! Klien IMAP kecil untuk migrasi email (RFC 3501 + LITERAL+, SASL-IR, SPECIAL-USE, NAMESPACE).
//!
//! Tanpa crate IMAP: perintah ditulis apa adanya, dan setiap respons diurai menjadi pohon nilai
//! (atom, teks, daftar), termasuk literal `{n}` yang membawa isi email utuh. TLS memakai TLS bawaan
//! sistem lewat native-tls (SChannel di Windows, Security.framework di macOS).

use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use super::kode::{base64, dari_base64, fnv64};

const WAKTU_SAMBUNG: Duration = Duration::from_secs(20);
const WAKTU_BACA: Duration = Duration::from_secs(180);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keamanan {
    /// TLS sejak awal (port 993).
    Ssl,
    /// Sambungan biasa lalu STARTTLS (port 143).
    StartTls,
    /// Tanpa enkripsi (hanya untuk jaringan lokal/uji).
    Tanpa,
}

impl Keamanan {
    pub fn urai(s: &str) -> Option<Keamanan> {
        match s.to_lowercase().as_str() {
            "ssl" | "tls" | "imaps" => Some(Keamanan::Ssl),
            "starttls" => Some(Keamanan::StartTls),
            "tanpa" | "none" | "polos" => Some(Keamanan::Tanpa),
            _ => None,
        }
    }
    pub fn nama(self) -> &'static str {
        match self {
            Keamanan::Ssl => "ssl",
            Keamanan::StartTls => "starttls",
            Keamanan::Tanpa => "tanpa",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Galat {
    /// Sambungan putus, waktu habis, atau server menutup (BYE): menyambung ulang bisa membantu.
    Putus(String),
    /// Server menolak (NO/BAD, login salah, sertifikat tidak sah): mengulang tidak membantu.
    Ditolak(String),
}

impl std::fmt::Display for Galat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Galat::Putus(s) | Galat::Ditolak(s) => f.write_str(s),
        }
    }
}

impl From<io::Error> for Galat {
    fn from(e: io::Error) -> Self {
        Galat::Putus(match e.kind() {
            io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock => "server tidak menjawab (waktu habis)".into(),
            io::ErrorKind::UnexpectedEof => "server menutup sambungan".into(),
            _ => e.to_string(),
        })
    }
}

pub type Hasil<T> = Result<T, Galat>;

// ------------------------------------------------------------------ respons

#[derive(Clone, Debug, PartialEq)]
pub enum Nilai {
    Atom(String),
    Teks(Vec<u8>),
    Daftar(Vec<Nilai>),
    Nil,
}

impl Nilai {
    pub fn atom_sama(&self, s: &str) -> bool {
        matches!(self, Nilai::Atom(a) if a.eq_ignore_ascii_case(s))
    }
    pub fn teks(&self) -> Option<String> {
        match self {
            Nilai::Atom(a) => Some(a.clone()),
            Nilai::Teks(b) => Some(String::from_utf8_lossy(b).into_owned()),
            _ => None,
        }
    }
    pub fn byte(&self) -> Option<&[u8]> {
        match self {
            Nilai::Atom(a) => Some(a.as_bytes()),
            Nilai::Teks(b) => Some(b),
            _ => None,
        }
    }
    pub fn angka(&self) -> Option<u64> {
        match self {
            Nilai::Atom(a) => a.parse().ok(),
            _ => None,
        }
    }
    pub fn daftar(&self) -> Option<&[Nilai]> {
        match self {
            Nilai::Daftar(d) => Some(d),
            _ => None,
        }
    }
}

/// Satu respons server: `tag` = "*", "+", atau tag perintah; `nilai` tanpa tag; `baris` = teks
/// baris pertama tanpa tag (untuk pesan galat).
#[derive(Debug)]
pub struct Respons {
    pub tag: String,
    pub nilai: Vec<Nilai>,
    pub baris: String,
}

enum Potong {
    Teks(Vec<u8>),
    Literal(Vec<u8>),
}

/// "... {123}" atau "{123+}" di akhir baris (di luar kutipan) -> (123, posisi '{').
fn ukuran_literal(b: &[u8]) -> Option<(usize, usize)> {
    if b.last() != Some(&b'}') {
        return None;
    }
    let buka = b.iter().rposition(|c| *c == b'{')?;
    let isi = &b[buka + 1..b.len() - 1];
    let isi = isi.strip_suffix(b"+").unwrap_or(isi);
    if isi.is_empty() || !isi.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let mut kutip = false;
    let mut i = 0;
    while i < buka {
        match b[i] {
            b'\\' if kutip => i += 1,
            b'"' => kutip = !kutip,
            _ => {}
        }
        i += 1;
    }
    if kutip {
        return None;
    }
    Some((std::str::from_utf8(isi).ok()?.parse().ok()?, buka))
}

fn baca_mentah(r: &mut impl BufRead) -> io::Result<Vec<Potong>> {
    let mut hasil = Vec::new();
    loop {
        let mut baris = Vec::new();
        if r.read_until(b'\n', &mut baris)? == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "server menutup sambungan"));
        }
        while matches!(baris.last(), Some(b'\n' | b'\r')) {
            baris.pop();
        }
        match ukuran_literal(&baris) {
            Some((n, awal)) => {
                baris.truncate(awal);
                hasil.push(Potong::Teks(baris));
                let mut lit = vec![0u8; n];
                r.read_exact(&mut lit)?;
                hasil.push(Potong::Literal(lit));
            }
            None => {
                hasil.push(Potong::Teks(baris));
                return Ok(hasil);
            }
        }
    }
}

enum Token {
    Buka,
    Tutup,
    Atom(String),
    Teks(Vec<u8>),
    Nil,
}

fn token_baris(b: &[u8], t: &mut Vec<Token>) {
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b' ' => i += 1,
            b'(' => {
                t.push(Token::Buka);
                i += 1;
            }
            b')' => {
                t.push(Token::Tutup);
                i += 1;
            }
            b'"' => {
                i += 1;
                let mut s = Vec::new();
                while i < b.len() && b[i] != b'"' {
                    if b[i] == b'\\' && i + 1 < b.len() {
                        i += 1;
                    }
                    s.push(b[i]);
                    i += 1;
                }
                i += 1;
                t.push(Token::Teks(s));
            }
            _ => {
                // Atom; "[...]" boleh berisi spasi & kurung (BODY[HEADER.FIELDS (MESSAGE-ID)]).
                let awal = i;
                let mut siku = 0;
                while i < b.len() {
                    match b[i] {
                        b'[' => siku += 1,
                        b']' if siku > 0 => siku -= 1,
                        b' ' | b'(' | b')' if siku == 0 => break,
                        _ => {}
                    }
                    i += 1;
                }
                let a = String::from_utf8_lossy(&b[awal..i]).into_owned();
                t.push(if a.eq_ignore_ascii_case("NIL") { Token::Nil } else { Token::Atom(a) });
            }
        }
    }
}

fn susun(potong: Vec<Potong>) -> Vec<Nilai> {
    let mut token = Vec::new();
    for p in potong {
        match p {
            Potong::Teks(b) => token_baris(&b, &mut token),
            Potong::Literal(b) => token.push(Token::Teks(b)),
        }
    }
    let mut tumpuk: Vec<Vec<Nilai>> = vec![Vec::new()];
    for t in token {
        match t {
            Token::Buka => tumpuk.push(Vec::new()),
            Token::Tutup => {
                if tumpuk.len() > 1 {
                    let d = tumpuk.pop().unwrap_or_default();
                    tumpuk.last_mut().unwrap().push(Nilai::Daftar(d));
                }
            }
            Token::Atom(a) => tumpuk.last_mut().unwrap().push(Nilai::Atom(a)),
            Token::Teks(b) => tumpuk.last_mut().unwrap().push(Nilai::Teks(b)),
            Token::Nil => tumpuk.last_mut().unwrap().push(Nilai::Nil),
        }
    }
    while tumpuk.len() > 1 {
        let d = tumpuk.pop().unwrap_or_default();
        tumpuk.last_mut().unwrap().push(Nilai::Daftar(d));
    }
    tumpuk.pop().unwrap_or_default()
}

fn urai_respons(potong: Vec<Potong>) -> Respons {
    let pertama = match potong.first() {
        Some(Potong::Teks(b)) => String::from_utf8_lossy(b).into_owned(),
        _ => String::new(),
    };
    let (tag, sisa) = match pertama.split_once(' ') {
        Some((a, b)) => (a.to_string(), b.to_string()),
        None => (pertama.clone(), String::new()),
    };
    if tag == "+" {
        return Respons { tag, nilai: Vec::new(), baris: sisa };
    }
    let mut nilai = susun(potong);
    if !nilai.is_empty() {
        nilai.remove(0);
    }
    Respons { tag, nilai, baris: sisa }
}

// ------------------------------------------------------------------ sambungan

enum Aliran {
    Polos(TcpStream),
    Tls(Box<native_tls::TlsStream<TcpStream>>),
    /// Pengganti sesaat ketika sambungan biasa ditingkatkan ke TLS (STARTTLS).
    Kosong,
}

fn tanpa_aliran() -> io::Error {
    io::Error::new(io::ErrorKind::NotConnected, "sambungan sedang ditingkatkan ke TLS")
}

impl Read for Aliran {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Aliran::Polos(s) => s.read(buf),
            Aliran::Tls(s) => s.read(buf),
            Aliran::Kosong => Err(tanpa_aliran()),
        }
    }
}

impl Write for Aliran {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Aliran::Polos(s) => s.write(buf),
            Aliran::Tls(s) => s.write(buf),
            Aliran::Kosong => Err(tanpa_aliran()),
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        match self {
            Aliran::Polos(s) => s.flush(),
            Aliran::Tls(s) => s.flush(),
            Aliran::Kosong => Ok(()),
        }
    }
}

/// Bagian perintah: kata perintah apa adanya, string IMAP (atom/kutip/literal otomatis), literal.
pub enum Arg<'a> {
    Polos(&'a str),
    Str(&'a str),
    Lit(&'a [u8]),
}

/// String IMAP sebagai atom atau teks berkutip; None bila harus literal (non-ASCII, CR/LF).
fn bentuk_astring(s: &str) -> Option<String> {
    if !s.is_empty() && s.bytes().all(|c| c.is_ascii_graphic() && !b"(){%*\"\\]".contains(&c)) {
        return Some(s.to_string());
    }
    if s.bytes().all(|c| (0x20..0x7f).contains(&c)) {
        return Some(format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")));
    }
    None
}

#[derive(Clone, Debug, PartialEq)]
pub struct Folder {
    /// Nama di server (UTF-7 IMAP), dipakai apa adanya di perintah.
    pub nama: String,
    pub pemisah: Option<char>,
    /// Atribut huruf kecil: "\noselect", "\sent", "\all", ...
    pub atribut: Vec<String>,
}

impl Folder {
    pub fn punya(&self, a: &str) -> bool {
        self.atribut.iter().any(|x| x.eq_ignore_ascii_case(a))
    }
    pub fn bisa_dipilih(&self) -> bool {
        !self.punya("\\noselect") && !self.punya("\\nonexistent")
    }
    /// Nama yang bisa dibaca manusia (UTF-7 IMAP diurai).
    pub fn tampil(&self) -> String {
        dari_utf7(&self.nama)
    }
}

/// Ringkasan satu email di folder (tanpa isinya).
#[derive(Clone, Debug, PartialEq)]
pub struct Ringkas {
    pub uid: u32,
    pub bendera: Vec<String>,
    /// INTERNALDATE apa adanya ("17-Jul-1996 02:44:25 -0700").
    pub tanggal: String,
    pub ukuran: u64,
    /// Kunci pencocokan antar-server (Message-ID, atau sidik Date/Subject/From).
    pub kunci: String,
}

/// Bendera yang ikut disalin: bendera sistem saja (kata kunci bebas bisa ditolak server tujuan).
pub fn bendera_salin(b: &[String]) -> Vec<String> {
    const BOLEH: [&str; 4] = ["\\Seen", "\\Answered", "\\Flagged", "\\Draft"];
    b.iter().filter_map(|x| BOLEH.iter().find(|y| y.eq_ignore_ascii_case(x)).map(|y| y.to_string())).collect()
}

pub struct Imap {
    r: BufReader<Aliran>,
    tag: u32,
    pub kemampuan: Vec<String>,
}

const KOLOM_HEADER: &str = "BODY.PEEK[HEADER.FIELDS (MESSAGE-ID DATE SUBJECT FROM)]";

impl Imap {
    /// Menyambung ke server dan membaca salam. `abaikan_sertifikat`: terima sertifikat swatanda
    /// (server sendiri di jaringan kantor).
    pub fn sambung(host: &str, port: u16, keamanan: Keamanan, abaikan_sertifikat: bool) -> Hasil<Imap> {
        let alamat: Vec<_> = (host, port)
            .to_socket_addrs()
            .map_err(|e| Galat::Putus(format!("alamat {host} tidak ditemukan ({e})")))?
            .collect();
        let mut tcp = None;
        let mut galat = String::new();
        for a in &alamat {
            match TcpStream::connect_timeout(a, WAKTU_SAMBUNG) {
                Ok(s) => {
                    tcp = Some(s);
                    break;
                }
                Err(e) => galat = e.to_string(),
            }
        }
        let tcp = tcp.ok_or_else(|| Galat::Putus(format!("tidak bisa tersambung ke {host}:{port} ({galat})")))?;
        tcp.set_read_timeout(Some(WAKTU_BACA))?;
        tcp.set_write_timeout(Some(WAKTU_BACA))?;
        let aliran = match keamanan {
            Keamanan::Ssl => Aliran::Tls(Box::new(tls(host, tcp, abaikan_sertifikat)?)),
            _ => Aliran::Polos(tcp),
        };
        let mut s = Imap { r: BufReader::with_capacity(64 * 1024, aliran), tag: 0, kemampuan: Vec::new() };
        let salam = s.baca()?;
        if salam.tag != "*" || !(salam.nilai.first().is_some_and(|n| n.atom_sama("OK") || n.atom_sama("PREAUTH"))) {
            return Err(Galat::Ditolak(format!("server menolak sambungan: {}", salam.baris)));
        }
        if s.kemampuan.is_empty() {
            s.kapabilitas()?;
        }
        if keamanan == Keamanan::StartTls {
            if !s.punya("STARTTLS") {
                return Err(Galat::Ditolak(format!("{host} tidak mendukung STARTTLS; pilih SSL/TLS (port 993).")));
            }
            let t = s.kirim(&[Arg::Polos("STARTTLS")])?;
            s.selesai(&t, |_| {})?;
            // Setelah "OK" server diam menunggu jabat tangan TLS, jadi penyangga baca kosong.
            let aliran = std::mem::replace(&mut s.r, BufReader::new(Aliran::Kosong));
            let tcp = match aliran.into_inner() {
                Aliran::Polos(t) => t,
                _ => return Err(Galat::Ditolak("STARTTLS pada sambungan yang sudah aman".into())),
            };
            s.r = BufReader::with_capacity(64 * 1024, Aliran::Tls(Box::new(tls(host, tcp, abaikan_sertifikat)?)));
            s.kemampuan.clear();
            s.kapabilitas()?;
        }
        Ok(s)
    }

    pub fn punya(&self, k: &str) -> bool {
        self.kemampuan.iter().any(|x| x.eq_ignore_ascii_case(k))
    }

    fn tulis(&mut self, b: &[u8]) -> Hasil<()> {
        let w = self.r.get_mut();
        w.write_all(b)?;
        w.flush()?;
        Ok(())
    }

    fn baca(&mut self) -> Hasil<Respons> {
        let r = urai_respons(baca_mentah(&mut self.r)?);
        // Kemampuan bisa datang lewat "* CAPABILITY ..." atau kode "[CAPABILITY ...]".
        if r.tag == "*" && r.nilai.first().is_some_and(|n| n.atom_sama("CAPABILITY")) {
            self.kemampuan = r.nilai[1..].iter().filter_map(Nilai::teks).collect();
        } else if let Some(k) = r.nilai.get(1).and_then(Nilai::teks) {
            if let Some(isi) = k.strip_prefix("[CAPABILITY ").and_then(|x| x.strip_suffix(']')) {
                self.kemampuan = isi.split_whitespace().map(str::to_string).collect();
            }
        }
        Ok(r)
    }

    fn kirim(&mut self, args: &[Arg]) -> Hasil<String> {
        self.tag += 1;
        let tag = format!("L{:04}", self.tag);
        let mut buf: Vec<u8> = tag.as_bytes().to_vec();
        for a in args {
            buf.push(b' ');
            match a {
                Arg::Polos(s) => buf.extend_from_slice(s.as_bytes()),
                Arg::Str(s) => match bentuk_astring(s) {
                    Some(q) => buf.extend_from_slice(q.as_bytes()),
                    None => self.literal(&mut buf, s.as_bytes(), &tag)?,
                },
                Arg::Lit(b) => self.literal(&mut buf, b, &tag)?,
            }
        }
        buf.extend_from_slice(b"\r\n");
        self.tulis(&buf)?;
        Ok(tag)
    }

    fn literal(&mut self, buf: &mut Vec<u8>, isi: &[u8], tag: &str) -> Hasil<()> {
        if self.punya("LITERAL+") {
            buf.extend_from_slice(format!("{{{}+}}\r\n", isi.len()).as_bytes());
        } else {
            buf.extend_from_slice(format!("{{{}}}\r\n", isi.len()).as_bytes());
            self.tulis(buf)?;
            buf.clear();
            self.tunggu_lanjut(tag)?;
        }
        buf.extend_from_slice(isi);
        Ok(())
    }

    /// Menunggu "+" (server siap menerima literal/lanjutan SASL).
    fn tunggu_lanjut(&mut self, tag: &str) -> Hasil<Respons> {
        loop {
            let r = self.baca()?;
            if r.tag == "+" {
                return Ok(r);
            }
            if r.tag == tag {
                return Err(Galat::Ditolak(tanpa_status(&r.baris)));
            }
            cek_bye(&r)?;
        }
    }

    /// Membaca respons sampai jawaban bertag `tag`; respons tak bertag diteruskan ke `f`.
    fn selesai<F: FnMut(Respons)>(&mut self, tag: &str, mut f: F) -> Hasil<Respons> {
        loop {
            let r = self.baca()?;
            if r.tag == tag {
                return if r.nilai.first().is_some_and(|n| n.atom_sama("OK")) {
                    Ok(r)
                } else {
                    Err(Galat::Ditolak(tanpa_status(&r.baris)))
                };
            }
            if r.tag == "*" {
                cek_bye(&r)?;
                f(r);
            }
        }
    }

    fn sederhana(&mut self, args: &[Arg]) -> Hasil<Vec<Respons>> {
        let t = self.kirim(args)?;
        let mut v = Vec::new();
        self.selesai(&t, |r| v.push(r))?;
        Ok(v)
    }

    pub fn kapabilitas(&mut self) -> Hasil<()> {
        self.sederhana(&[Arg::Polos("CAPABILITY")]).map(|_| ())
    }

    /// SASL dengan respons awal: satu baris bila server punya SASL-IR, atau lewat "+".
    fn sasl(&mut self, mekanisme: &str, data: &[u8]) -> Hasil<()> {
        let b = base64(data);
        let tag = if self.punya("SASL-IR") {
            self.kirim(&[Arg::Polos("AUTHENTICATE"), Arg::Polos(mekanisme), Arg::Polos(&b)])?
        } else {
            let t = self.kirim(&[Arg::Polos("AUTHENTICATE"), Arg::Polos(mekanisme)])?;
            self.tunggu_lanjut(&t)?;
            self.tulis(format!("{b}\r\n").as_bytes())?;
            t
        };
        loop {
            let r = self.baca()?;
            if r.tag == "+" {
                // Tantangan berisi galat (XOAUTH2: JSON base64). Jawab kosong agar server mengakhiri.
                let isi = dari_base64(r.baris.trim()).map(|v| String::from_utf8_lossy(&v).into_owned()).unwrap_or_default();
                self.tulis(b"\r\n")?;
                let akhir = self.selesai(&tag, |_| {});
                let pesan = akhir.err().map(|e| e.to_string()).unwrap_or_default();
                return Err(Galat::Ditolak(format!("{pesan} {isi}").trim().to_string()));
            }
            if r.tag == tag {
                if r.nilai.first().is_some_and(|n| n.atom_sama("OK")) {
                    break;
                }
                return Err(Galat::Ditolak(tanpa_status(&r.baris)));
            }
            cek_bye(&r)?;
        }
        self.kapabilitas()
    }

    /// Login dengan kata sandi: AUTHENTICATE PLAIN bila didukung, selain itu LOGIN.
    pub fn masuk_sandi(&mut self, pengguna: &str, sandi: &str) -> Hasil<()> {
        if self.punya("AUTH=PLAIN") {
            return self.sasl("PLAIN", format!("\0{pengguna}\0{sandi}").as_bytes());
        }
        if self.punya("LOGINDISABLED") {
            return Err(Galat::Ditolak("server tidak mengizinkan login sandi pada sambungan ini; pakai SSL/TLS atau STARTTLS.".into()));
        }
        self.sederhana(&[Arg::Polos("LOGIN"), Arg::Str(pengguna), Arg::Str(sandi)])?;
        self.kapabilitas()
    }

    /// Login OAuth 2.0 (Gmail, Outlook): SASL XOAUTH2 dengan access token.
    pub fn masuk_oauth(&mut self, pengguna: &str, token: &str) -> Hasil<()> {
        self.sasl("XOAUTH2", format!("user={pengguna}\x01auth=Bearer {token}\x01\x01").as_bytes())
    }

    /// Ruang nama pribadi: (awalan, pemisah), mis. ("INBOX.", '.') di cPanel atau ("", '/').
    pub fn namespace(&mut self) -> Hasil<Option<(String, Option<char>)>> {
        if !self.punya("NAMESPACE") {
            return Ok(None);
        }
        let v = self.sederhana(&[Arg::Polos("NAMESPACE")])?;
        for r in v {
            if r.nilai.first().is_some_and(|n| n.atom_sama("NAMESPACE")) {
                let pribadi = r.nilai.get(1).and_then(Nilai::daftar).and_then(|d| d.first()).and_then(Nilai::daftar);
                if let Some(p) = pribadi {
                    let awalan = p.first().and_then(Nilai::teks).unwrap_or_default();
                    let pemisah = p.get(1).and_then(Nilai::teks).and_then(|s| s.chars().next());
                    return Ok(Some((awalan, pemisah)));
                }
            }
        }
        Ok(None)
    }

    pub fn daftar_folder(&mut self) -> Hasil<Vec<Folder>> {
        let mut args = vec![Arg::Polos("LIST"), Arg::Polos("\"\""), Arg::Polos("\"*\"")];
        if self.punya("SPECIAL-USE") && self.punya("LIST-EXTENDED") {
            args.push(Arg::Polos("RETURN (SPECIAL-USE)"));
        }
        let v = self.sederhana(&args)?;
        Ok(v.iter().filter_map(urai_list).collect())
    }

    /// EXAMINE (baca saja: status "sudah dibaca" di server sumber tidak berubah). Hasil: jumlah email.
    pub fn periksa(&mut self, folder: &str) -> Hasil<u32> {
        let v = self.sederhana(&[Arg::Polos("EXAMINE"), Arg::Str(folder)])?;
        Ok(v.iter()
            .find_map(|r| (r.nilai.get(1).is_some_and(|n| n.atom_sama("EXISTS"))).then(|| r.nilai[0].angka()).flatten())
            .unwrap_or(0) as u32)
    }

    /// Ringkasan semua email di folder yang sedang dibuka (`periksa` dulu).
    pub fn ringkasan(&mut self, jumlah: u32) -> Hasil<Vec<Ringkas>> {
        if jumlah == 0 {
            return Ok(Vec::new());
        }
        let perintah = format!("FETCH 1:* (UID FLAGS INTERNALDATE RFC822.SIZE {KOLOM_HEADER})");
        let v = self.sederhana(&[Arg::Polos("UID"), Arg::Polos(&perintah)])?;
        let mut hasil: Vec<Ringkas> = v.iter().filter_map(urai_ringkas).collect();
        hasil.sort_by_key(|r| r.uid);
        hasil.dedup_by_key(|r| r.uid);
        Ok(hasil)
    }

    /// Isi lengkap (RFC 822) beberapa email; urutan hasil mengikuti server.
    pub fn ambil(&mut self, uid: &[u32]) -> Hasil<Vec<(u32, Vec<u8>)>> {
        if uid.is_empty() {
            return Ok(Vec::new());
        }
        let set: Vec<String> = uid.iter().map(u32::to_string).collect();
        let perintah = format!("FETCH {} (UID BODY.PEEK[])", set.join(","));
        let t = self.kirim(&[Arg::Polos("UID"), Arg::Polos(&perintah)])?;
        let mut hasil = Vec::new();
        self.selesai(&t, |r| {
            if let Some(x) = urai_isi(r) {
                hasil.push(x);
            }
        })?;
        Ok(hasil)
    }

    pub fn buat_folder(&mut self, nama: &str) -> Hasil<()> {
        self.sederhana(&[Arg::Polos("CREATE"), Arg::Str(nama)])?;
        // Langganan agar folder tampil di webmail/aplikasi email; gagal pun tidak masalah.
        let _ = self.sederhana(&[Arg::Polos("SUBSCRIBE"), Arg::Str(nama)]);
        Ok(())
    }

    /// APPEND: menaruh satu email di folder dengan bendera dan tanggal terima aslinya.
    pub fn tambah(&mut self, folder: &str, bendera: &[String], tanggal: &str, isi: &[u8]) -> Hasil<()> {
        let b = format!("({})", bendera.join(" "));
        let tgl = (!tanggal.is_empty()).then(|| format!("\"{tanggal}\""));
        let mut args = vec![Arg::Polos("APPEND"), Arg::Str(folder), Arg::Polos(&b)];
        if let Some(t) = &tgl {
            args.push(Arg::Polos(t));
        }
        args.push(Arg::Lit(isi));
        self.sederhana(&args).map(|_| ())
    }

    pub fn keluar(&mut self) {
        if let Ok(t) = self.kirim(&[Arg::Polos("LOGOUT")]) {
            // BYE lalu OK; galat diabaikan karena sambungan memang ditutup.
            loop {
                match urai_respons_aman(&mut self.r) {
                    Some(r) if r.tag == t => break,
                    Some(_) => {}
                    None => break,
                }
            }
        }
    }
}

fn urai_respons_aman(r: &mut BufReader<Aliran>) -> Option<Respons> {
    baca_mentah(r).ok().map(urai_respons)
}

/// "NO [AUTHENTICATIONFAILED] Invalid credentials" -> "[AUTHENTICATIONFAILED] Invalid credentials".
fn tanpa_status(baris: &str) -> String {
    for s in ["NO ", "BAD ", "OK "] {
        if let Some(x) = baris.strip_prefix(s) {
            return x.trim().to_string();
        }
    }
    baris.trim().to_string()
}

fn cek_bye(r: &Respons) -> Hasil<()> {
    if r.tag == "*" && r.nilai.first().is_some_and(|n| n.atom_sama("BYE")) {
        return Err(Galat::Putus(format!("server menutup sambungan: {}", r.baris)));
    }
    Ok(())
}

fn tls(host: &str, tcp: TcpStream, abaikan_sertifikat: bool) -> Hasil<native_tls::TlsStream<TcpStream>> {
    let mut b = native_tls::TlsConnector::builder();
    if abaikan_sertifikat {
        b.danger_accept_invalid_certs(true).danger_accept_invalid_hostnames(true);
    }
    let c = b.build().map_err(|e| Galat::Ditolak(format!("TLS tidak tersedia: {e}")))?;
    c.connect(host, tcp).map_err(|e| match e {
        native_tls::HandshakeError::Failure(e) => {
            Galat::Ditolak(format!("sambungan aman ke {host} gagal: {e}. Bila server memakai sertifikat swatanda, centang \"Terima sertifikat swatanda\"."))
        }
        native_tls::HandshakeError::WouldBlock(_) => Galat::Putus("jabat tangan TLS terhenti".into()),
    })
}

fn urai_list(r: &Respons) -> Option<Folder> {
    if !(r.nilai.first()?.atom_sama("LIST") || r.nilai.first()?.atom_sama("XLIST")) {
        return None;
    }
    let atribut = r.nilai.get(1)?.daftar()?.iter().filter_map(Nilai::teks).map(|s| s.to_lowercase()).collect();
    let pemisah = r.nilai.get(2).and_then(Nilai::teks).and_then(|s| s.chars().next());
    let nama = r.nilai.get(3)?.teks()?;
    Some(Folder { nama, pemisah, atribut })
}

/// Nilai setelah kunci `k` di daftar FETCH (kunci tanpa membedakan huruf).
fn ambil_kunci<'a>(d: &'a [Nilai], k: &str) -> Option<&'a Nilai> {
    d.iter().position(|n| matches!(n, Nilai::Atom(a) if a.eq_ignore_ascii_case(k))).and_then(|i| d.get(i + 1))
}

fn daftar_fetch(r: &Respons) -> Option<&[Nilai]> {
    (r.nilai.get(1)?.atom_sama("FETCH")).then_some(())?;
    r.nilai.get(2)?.daftar()
}

fn urai_ringkas(r: &Respons) -> Option<Ringkas> {
    let d = daftar_fetch(r)?;
    let uid = ambil_kunci(d, "UID")?.angka()? as u32;
    let bendera = ambil_kunci(d, "FLAGS").and_then(Nilai::daftar).map(|f| f.iter().filter_map(Nilai::teks).collect()).unwrap_or_default();
    let tanggal = ambil_kunci(d, "INTERNALDATE").and_then(Nilai::teks).unwrap_or_default();
    let ukuran = ambil_kunci(d, "RFC822.SIZE").and_then(Nilai::angka).unwrap_or(0);
    let header = d
        .iter()
        .position(|n| matches!(n, Nilai::Atom(a) if a.to_ascii_uppercase().starts_with("BODY[HEADER")))
        .and_then(|i| d.get(i + 1))
        .and_then(Nilai::byte)
        .unwrap_or(&[]);
    Some(Ringkas { uid, bendera, tanggal, ukuran, kunci: kunci_pesan(header) })
}

fn urai_isi(r: Respons) -> Option<(u32, Vec<u8>)> {
    let d = daftar_fetch(&r)?;
    let uid = ambil_kunci(d, "UID")?.angka()? as u32;
    let i = d.iter().position(|n| matches!(n, Nilai::Atom(a) if a.eq_ignore_ascii_case("BODY[]")))?;
    match d.get(i + 1)? {
        Nilai::Teks(b) => Some((uid, b.clone())),
        _ => None,
    }
}

// ------------------------------------------------------------------ header & kunci

/// Kolom header (nama huruf kecil, isi dengan baris lanjutan digabung).
pub fn urai_header(h: &[u8]) -> Vec<(String, String)> {
    let teks = String::from_utf8_lossy(h);
    let mut v: Vec<(String, String)> = Vec::new();
    for baris in teks.split('\n') {
        let baris = baris.trim_end_matches('\r');
        if baris.is_empty() {
            continue;
        }
        if baris.starts_with([' ', '\t']) {
            if let Some(akhir) = v.last_mut() {
                akhir.1.push(' ');
                akhir.1.push_str(baris.trim());
            }
        } else if let Some((k, isi)) = baris.split_once(':') {
            v.push((k.trim().to_lowercase(), isi.trim().to_string()));
        }
    }
    v
}

/// Kunci pencocokan email antar-server: Message-ID (tanpa spasi); bila tidak ada, sidik dari
/// Date + Subject + From yang spasinya dirapikan.
pub fn kunci_pesan(header: &[u8]) -> String {
    let h = urai_header(header);
    let ambil = |k: &str| h.iter().find(|(n, _)| n == k).map(|(_, v)| v.split_whitespace().collect::<Vec<_>>().join(" ")).unwrap_or_default();
    let id: String = ambil("message-id").split_whitespace().collect();
    if !id.is_empty() {
        return format!("id:{id}");
    }
    let s = format!("{}\n{}\n{}", ambil("date"), ambil("subject"), ambil("from"));
    format!("h:{:016x}", fnv64(s.as_bytes()))
}

// ------------------------------------------------------------------ UTF-7 IMAP (RFC 3501 5.1.3)

/// "Arsip &AOk-t&AOk-" -> "Arsip été".
pub fn dari_utf7(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.split('&');
    out.push_str(it.next().unwrap_or(""));
    for bagian in it {
        let (kode, sisa) = match bagian.find('-') {
            Some(i) => (&bagian[..i], &bagian[i + 1..]),
            None => (bagian, ""),
        };
        if kode.is_empty() {
            out.push('&');
        } else {
            let b = dari_base64(&kode.replace(',', "/")).unwrap_or_default();
            let u: Vec<u16> = b.chunks(2).filter(|c| c.len() == 2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
            out.push_str(&String::from_utf16_lossy(&u));
        }
        out.push_str(sisa);
    }
    out
}

/// "Arsip été" -> "Arsip &AOk-t&AOk-".
pub fn ke_utf7(s: &str) -> String {
    let mut out = String::new();
    let mut tunda: Vec<u16> = Vec::new();
    let tutup = |out: &mut String, tunda: &mut Vec<u16>| {
        if !tunda.is_empty() {
            let b: Vec<u8> = tunda.iter().flat_map(|u| u.to_be_bytes()).collect();
            out.push('&');
            out.push_str(&base64(&b).trim_end_matches('=').replace('/', ","));
            out.push('-');
            tunda.clear();
        }
    };
    for c in s.chars() {
        if (' '..='~').contains(&c) {
            tutup(&mut out, &mut tunda);
            if c == '&' {
                out.push_str("&-");
            } else {
                out.push(c);
            }
        } else {
            let mut b = [0u16; 2];
            tunda.extend_from_slice(c.encode_utf16(&mut b));
        }
    }
    tutup(&mut out, &mut tunda);
    out
}

#[cfg(test)]
mod uji {
    use super::*;

    fn respons(teks: &[u8]) -> Respons {
        let mut r = BufReader::new(teks);
        urai_respons(baca_mentah(&mut r).unwrap())
    }

    #[test]
    fn list_dan_namespace() {
        let r = respons(b"* LIST (\\HasNoChildren \\Sent) \"/\" \"[Gmail]/Sent Mail\"\r\n");
        let f = urai_list(&r).unwrap();
        assert_eq!(f.nama, "[Gmail]/Sent Mail");
        assert_eq!(f.pemisah, Some('/'));
        assert!(f.punya("\\Sent") && f.bisa_dipilih());
        let r = respons(b"* LIST (\\Noselect \\HasChildren) \".\" INBOX\r\n");
        let f = urai_list(&r).unwrap();
        assert_eq!((f.nama.as_str(), f.pemisah), ("INBOX", Some('.')));
        assert!(!f.bisa_dipilih());
        // Nama sebagai literal.
        let r = respons(b"* LIST () \"/\" {10}\r\nProyek \"A\"\r\n");
        assert_eq!(urai_list(&r).unwrap().nama, "Proyek \"A\"");
        let r = respons(b"* LIST () NIL Arsip\r\n");
        assert_eq!(urai_list(&r).unwrap().pemisah, None);
        let r = respons(b"* NAMESPACE ((\"INBOX.\" \".\")) NIL ((\"#shared.\" \".\"))\r\n");
        let p = r.nilai[1].daftar().unwrap()[0].daftar().unwrap();
        assert_eq!(p[0].teks().unwrap(), "INBOX.");
    }

    #[test]
    fn fetch_ringkas_dan_isi() {
        let h = b"Message-ID: <abc@x>\r\nSubject: Halo\r\n\r\n";
        let mut b = format!(
            "* 3 FETCH (UID 77 FLAGS (\\Seen \\Recent $Forwarded) INTERNALDATE \"17-Jul-1996 02:44:25 -0700\" RFC822.SIZE 4286 BODY[HEADER.FIELDS (MESSAGE-ID DATE SUBJECT FROM)] {{{}}}\r\n",
            h.len()
        )
        .into_bytes();
        b.extend_from_slice(h);
        b.extend_from_slice(b")\r\n");
        let r = urai_ringkas(&respons(&b)).unwrap();
        assert_eq!(r.uid, 77);
        assert_eq!(r.tanggal, "17-Jul-1996 02:44:25 -0700");
        assert_eq!(r.ukuran, 4286);
        assert_eq!(r.kunci, "id:<abc@x>");
        assert_eq!(bendera_salin(&r.bendera), vec!["\\Seen".to_string()]);
        // Isi email dengan baris yang berakhiran "{5}" (bukan penanda literal karena di dalam literal).
        let isi = b"Subject: x\r\n\r\nharga {5}\r\n";
        let mut b = format!("* 1 FETCH (UID 5 BODY[] {{{}}}\r\n", isi.len()).into_bytes();
        b.extend_from_slice(isi);
        b.extend_from_slice(b")\r\n");
        assert_eq!(urai_isi(respons(&b)).unwrap(), (5, isi.to_vec()));
    }

    #[test]
    fn literal_di_kutipan_bukan_literal() {
        assert!(ukuran_literal(b"* LIST () \"/\" \"aneh {5}\"").is_none());
        assert_eq!(ukuran_literal(b"* 1 FETCH (BODY[] {12}"), Some((12, 18)));
        assert_eq!(ukuran_literal(b"A APPEND x {3+}"), Some((3, 11)));
    }

    #[test]
    fn kunci_tanpa_message_id() {
        let a = kunci_pesan(b"Date: Mon, 1 Jan 2024 10:00:00 +0700\r\nSubject:  Rapat\r\n  besok\r\nFrom: a@b\r\n\r\n");
        let b = kunci_pesan(b"From: a@b\r\nSubject: Rapat besok\r\nDate: Mon, 1 Jan 2024 10:00:00 +0700\r\n\r\n");
        assert!(a.starts_with("h:"));
        assert_eq!(a, b);
        assert_eq!(kunci_pesan(b"Message-ID:\r\n <x@y>\r\n\r\n"), "id:<x@y>");
    }

    #[test]
    fn utf7() {
        for (u, m) in [("INBOX", "INBOX"), ("Arsip été", "Arsip &AOk-t&AOk-"), ("A&B", "A&-B"), ("日本語", "&ZeVnLIqe-"), ("Terkirim/2024", "Terkirim/2024")] {
            assert_eq!(ke_utf7(u), m, "{u}");
            assert_eq!(dari_utf7(m), u, "{m}");
        }
    }

    #[test]
    fn astring() {
        assert_eq!(bentuk_astring("INBOX").unwrap(), "INBOX");
        assert_eq!(bentuk_astring("Sent Items").unwrap(), "\"Sent Items\"");
        assert_eq!(bentuk_astring("a\"b\\").unwrap(), "\"a\\\"b\\\\\"");
        assert!(bentuk_astring("sandi é").is_none());
        assert_eq!(bentuk_astring("").unwrap(), "\"\"");
    }
}
