//! Login OAuth 2.0 untuk IMAP (Gmail, Outlook/Microsoft 365).
//!
//! Alur kode otorisasi + PKCE dengan pengalihan ke localhost (RFC 8252, aplikasi desktop):
//! browser membuka halaman login penyedia; setelah pengguna setuju, penyedia mengalihkan ke
//! http://127.0.0.1:<port>/?code=... (Google) atau http://localhost:<port>/?code=... (Microsoft),
//! yang diterima server kecil di sini. Kode lalu ditukar dengan token lewat HTTPS. Token hanya ada
//! di memori selama migrasi berjalan; tidak ditulis ke disk.

use serde_json::Value;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use super::kode::{acak, base64_url, dari_url, sha256, url};

const BATAS_LOGIN: Duration = Duration::from_secs(300);

pub struct Penyedia {
    pub nama: &'static str,
    auth: &'static str,
    token: &'static str,
    scope: &'static str,
    /// Awalan alamat pengalihan (port ditambahkan saat login).
    pengalihan: &'static str,
    /// Parameter tambahan halaman login.
    tambahan: &'static str,
}

pub const GOOGLE: Penyedia = Penyedia {
    nama: "Google",
    auth: "https://accounts.google.com/o/oauth2/v2/auth",
    token: "https://oauth2.googleapis.com/token",
    scope: "https://mail.google.com/",
    pengalihan: "http://127.0.0.1",
    // Minta refresh token (migrasi bisa lebih lama dari umur access token, 1 jam).
    tambahan: "&access_type=offline&prompt=consent",
};

pub const MICROSOFT: Penyedia = Penyedia {
    nama: "Microsoft",
    auth: "https://login.microsoftonline.com/common/oauth2/v2.0/authorize",
    token: "https://login.microsoftonline.com/common/oauth2/v2.0/token",
    scope: "https://outlook.office.com/IMAP.AccessAsUser.All offline_access",
    pengalihan: "http://localhost",
    tambahan: "&prompt=select_account",
};

/// Client OAuth: Google memakai client ID + secret aplikasi desktop; Microsoft cukup client ID
/// (klien publik, tanpa secret).
pub struct Klien {
    pub id: String,
    pub rahasia: Option<String>,
}

pub struct Token {
    pub akses: String,
    pub segar: Option<String>,
    pub kedaluwarsa: Instant,
}

impl Token {
    /// Access token tinggal kurang dari 5 menit.
    pub fn hampir_habis(&self) -> bool {
        self.kedaluwarsa.saturating_duration_since(Instant::now()) < Duration::from_secs(300)
    }
}

/// Alamat halaman login (dipisah agar bisa diuji).
pub fn alamat_login(p: &Penyedia, k: &Klien, pengalihan: &str, tantangan: &str, state: &str, email: &str) -> String {
    format!(
        "{}?client_id={}&response_type=code&redirect_uri={}&scope={}&code_challenge={}&code_challenge_method=S256&state={}&login_hint={}{}",
        p.auth,
        url(&k.id),
        url(pengalihan),
        url(p.scope),
        tantangan,
        state,
        url(email),
        p.tambahan
    )
}

/// Login di browser. `buka` membuka alamat di browser bawaan; alamatnya juga dicetak lewat
/// `kabar` agar bisa disalin manual bila browser tidak terbuka.
pub fn login(p: &Penyedia, k: &Klien, email: &str, buka: impl Fn(&str), kabar: impl Fn(&str)) -> Result<Token, String> {
    let pemverifikasi = base64_url(&acak(32));
    let tantangan = base64_url(&sha256(pemverifikasi.as_bytes()));
    let state = base64_url(&acak(16));
    let v4 = TcpListener::bind("127.0.0.1:0").map_err(|e| format!("port lokal untuk login tidak bisa dibuka: {e}"))?;
    let port = v4.local_addr().map_err(|e| e.to_string())?.port();
    let mut pendengar = vec![v4];
    // "localhost" di browser bisa berarti ::1; dengarkan juga di sana bila bisa.
    if p.pengalihan.contains("localhost") {
        if let Ok(v6) = TcpListener::bind(("::1", port)) {
            pendengar.push(v6);
        }
    }
    for l in &pendengar {
        l.set_nonblocking(true).map_err(|e| e.to_string())?;
    }
    let pengalihan = format!("{}:{port}", p.pengalihan);
    let alamat = alamat_login(p, k, &pengalihan, &tantangan, &state, email);
    kabar(&format!("Membuka browser untuk login {} ({email}). Bila browser tidak terbuka, buka alamat ini:", p.nama));
    kabar(&alamat);
    buka(&alamat);

    let mulai = Instant::now();
    let kode = loop {
        if mulai.elapsed() > BATAS_LOGIN {
            return Err(format!("Login {} tidak selesai dalam 5 menit.", p.nama));
        }
        let mut dapat = None;
        for l in &pendengar {
            if let Ok((s, _)) = l.accept() {
                dapat = Some(s);
                break;
            }
        }
        let Some(mut s) = dapat else {
            std::thread::sleep(Duration::from_millis(150));
            continue;
        };
        let _ = s.set_nonblocking(false);
        let _ = s.set_read_timeout(Some(Duration::from_secs(10)));
        match terima_pengalihan(&mut s, &state) {
            Ok(Some(k)) => break k,
            Ok(None) => continue, // favicon dsb.
            Err(e) => return Err(e),
        }
    };

    let mut isi = format!(
        "grant_type=authorization_code&code={}&redirect_uri={}&client_id={}&code_verifier={}",
        url(&kode),
        url(&pengalihan),
        url(&k.id),
        pemverifikasi
    );
    if let Some(r) = &k.rahasia {
        isi.push_str(&format!("&client_secret={}", url(r)));
    }
    tukar_token(p, &isi, None)
}

/// Access token baru dari refresh token (tanpa membuka browser).
pub fn segarkan(p: &Penyedia, k: &Klien, t: &Token) -> Result<Token, String> {
    let segar = t.segar.clone().ok_or_else(|| format!("Login {} perlu diulang (tidak ada refresh token).", p.nama))?;
    let mut isi = format!("grant_type=refresh_token&refresh_token={}&client_id={}", url(&segar), url(&k.id));
    if let Some(r) = &k.rahasia {
        isi.push_str(&format!("&client_secret={}", url(r)));
    }
    if p.nama == "Microsoft" {
        isi.push_str(&format!("&scope={}", url(p.scope)));
    }
    tukar_token(p, &isi, Some(segar))
}

fn tukar_token(p: &Penyedia, isi: &str, segar_lama: Option<String>) -> Result<Token, String> {
    let (status, badan) = post_form(p.token, isi)?;
    let j: Value = serde_json::from_str(&badan).unwrap_or(Value::Null);
    let teks = |k: &str| j.get(k).and_then(Value::as_str).map(str::to_string);
    match teks("access_token") {
        Some(akses) if status == 200 => Ok(Token {
            akses,
            segar: teks("refresh_token").or(segar_lama),
            kedaluwarsa: Instant::now() + Duration::from_secs(j.get("expires_in").and_then(Value::as_u64).unwrap_or(3600)),
        }),
        _ => {
            let e = teks("error_description").or_else(|| teks("error")).unwrap_or_else(|| format!("HTTP {status}"));
            Err(format!("Token {} tidak didapat: {}", p.nama, e.lines().next().unwrap_or("")))
        }
    }
}

/// Membaca permintaan GET dari browser. Ok(Some(kode)) bila berhasil, Ok(None) untuk permintaan
/// lain (favicon), Err bila penyedia mengirim galat atau state tidak cocok.
fn terima_pengalihan(s: &mut TcpStream, state: &str) -> Result<Option<String>, String> {
    let mut buf = vec![0u8; 8192];
    let mut n = 0;
    while n < buf.len() {
        match s.read(&mut buf[n..]) {
            Ok(0) => break,
            Ok(k) => {
                n += k;
                if buf[..n].windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    let req = String::from_utf8_lossy(&buf[..n]).into_owned();
    let jalur = req.split_whitespace().nth(1).unwrap_or("");
    let (dasar, query) = jalur.split_once('?').unwrap_or((jalur, ""));
    if dasar != "/" {
        let _ = s.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        return Ok(None);
    }
    let ambil = |k: &str| query.split('&').find_map(|p| p.split_once('=').filter(|(a, _)| *a == k).map(|(_, b)| dari_url(b)));
    let (judul, pesan, hasil) = if let Some(e) = ambil("error") {
        let ket = ambil("error_description").unwrap_or_default();
        ("Login dibatalkan", format!("{e}. {ket}"), Err(format!("Login ditolak atau dibatalkan: {e} {ket}").trim().to_string()))
    } else if ambil("state").as_deref() != Some(state) {
        ("Login tidak sah", "Permintaan ini bukan dari LELEMOVE.".into(), Err("Jawaban login tidak cocok (state berbeda); ulangi login.".into()))
    } else if let Some(k) = ambil("code") {
        ("Login berhasil", "Anda boleh menutup tab ini dan kembali ke LELEMOVE.".into(), Ok(Some(k)))
    } else {
        ("Login tidak lengkap", "Kode login tidak ada.".into(), Err("Kode login tidak diterima.".into()))
    };
    let html = format!(
        "<!doctype html><meta charset=utf-8><title>LELEMOVE</title><body style=\"font-family:sans-serif;margin:3em\"><h2>{judul}</h2><p>{}</p>",
        pesan.replace('<', "&lt;")
    );
    let _ = s.write_all(
        format!("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{html}", html.len())
            .as_bytes(),
    );
    hasil
}

/// POST form ke alamat https:// lalu kembalikan (status, badan). Sambungan ditutup setelahnya.
fn post_form(alamat: &str, isi: &str) -> Result<(u16, String), String> {
    let sisa = alamat.strip_prefix("https://").ok_or("hanya https yang didukung")?;
    let (host, jalur) = sisa.split_once('/').map(|(h, j)| (h, format!("/{j}"))).unwrap_or((sisa, "/".into()));
    let a = (host, 443).to_socket_addrs().map_err(|e| format!("{host} tidak ditemukan ({e})"))?.next().ok_or("alamat kosong")?;
    let tcp = TcpStream::connect_timeout(&a, Duration::from_secs(20)).map_err(|e| format!("{host}: {e}"))?;
    let _ = tcp.set_read_timeout(Some(Duration::from_secs(60)));
    let tls = native_tls::TlsConnector::new().map_err(|e| e.to_string())?;
    let mut s = tls.connect(host, tcp).map_err(|e| format!("sambungan aman ke {host} gagal: {e}"))?;
    let req = format!(
        "POST {jalur} HTTP/1.1\r\nHost: {host}\r\nUser-Agent: LELEMOVE/{}\r\nAccept: application/json\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{isi}",
        crate::VERSI,
        isi.len()
    );
    s.write_all(req.as_bytes()).map_err(|e| e.to_string())?;
    let mut data = Vec::new();
    let mut buf = [0u8; 16384];
    loop {
        match s.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                data.extend_from_slice(&buf[..n]);
                if respons_lengkap(&data) {
                    break;
                }
            }
            // Sebagian server menutup TLS tanpa close_notify: cukup bila respons sudah lengkap.
            Err(e) if respons_lengkap(&data) || !data.is_empty() && e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(format!("jawaban {host} terputus: {e}")),
        }
    }
    urai_http(&data)
}

fn pisah_header(data: &[u8]) -> Option<(String, &[u8])> {
    let i = data.windows(4).position(|w| w == b"\r\n\r\n")?;
    Some((String::from_utf8_lossy(&data[..i]).to_lowercase(), &data[i + 4..]))
}

fn respons_lengkap(data: &[u8]) -> bool {
    let Some((h, badan)) = pisah_header(data) else { return false };
    if let Some(n) = h.lines().find_map(|l| l.strip_prefix("content-length:")).and_then(|v| v.trim().parse::<usize>().ok()) {
        return badan.len() >= n;
    }
    h.contains("transfer-encoding: chunked") && badan.ends_with(b"0\r\n\r\n")
}

/// (status, badan) dari respons HTTP/1.1, termasuk Transfer-Encoding: chunked.
fn urai_http(data: &[u8]) -> Result<(u16, String), String> {
    let (h, badan) = pisah_header(data).ok_or("jawaban HTTP tidak lengkap")?;
    let status = h.split_whitespace().nth(1).and_then(|s| s.parse().ok()).ok_or("status HTTP tidak terbaca")?;
    let badan = if h.contains("transfer-encoding: chunked") {
        let mut out = Vec::new();
        let mut i = 0;
        while i < badan.len() {
            let Some(akhir) = badan[i..].windows(2).position(|w| w == b"\r\n") else { break };
            let ukuran = usize::from_str_radix(String::from_utf8_lossy(&badan[i..i + akhir]).split(';').next().unwrap_or("").trim(), 16).unwrap_or(0);
            i += akhir + 2;
            if ukuran == 0 || i + ukuran > badan.len() {
                break;
            }
            out.extend_from_slice(&badan[i..i + ukuran]);
            i += ukuran + 2;
        }
        out
    } else {
        badan.to_vec()
    };
    Ok((status, String::from_utf8_lossy(&badan).into_owned()))
}

#[cfg(test)]
mod uji {
    use super::*;

    #[test]
    fn alamat() {
        let k = Klien { id: "abc.apps".into(), rahasia: None };
        let a = alamat_login(&MICROSOFT, &k, "http://localhost:5000", "TANTANGAN", "STATE", "a@outlook.com");
        assert!(a.starts_with("https://login.microsoftonline.com/common/oauth2/v2.0/authorize?client_id=abc.apps&"));
        assert!(a.contains("&redirect_uri=http%3A%2F%2Flocalhost%3A5000&"));
        assert!(a.contains("&scope=https%3A%2F%2Foutlook.office.com%2FIMAP.AccessAsUser.All%20offline_access&"));
        assert!(a.contains("&code_challenge=TANTANGAN&code_challenge_method=S256&state=STATE&login_hint=a%40outlook.com&prompt=select_account"));
        let g = alamat_login(&GOOGLE, &k, "http://127.0.0.1:1", "T", "S", "x@gmail.com");
        assert!(g.contains("scope=https%3A%2F%2Fmail.google.com%2F&") && g.ends_with("&access_type=offline&prompt=consent"));
    }

    #[test]
    fn http() {
        let a = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 7\r\n\r\n{\"a\":1}";
        assert!(respons_lengkap(a));
        assert_eq!(urai_http(a).unwrap(), (200, "{\"a\":1}".into()));
        let c = b"HTTP/1.1 400 Bad Request\r\nTransfer-Encoding: chunked\r\n\r\n4\r\n{\"e\"\r\n3\r\n:1}\r\n0\r\n\r\n";
        assert!(respons_lengkap(c));
        assert_eq!(urai_http(c).unwrap(), (400, "{\"e\":1}".into()));
        assert!(!respons_lengkap(b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\n\r\n{}"));
    }

    #[test]
    fn pengalihan_lokal() {
        // Browser tiruan: kirim GET ke server pengalihan dan baca halaman yang dikembalikan.
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let browser = std::thread::spawn(move || {
            let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
            s.write_all(b"GET /?state=S1&code=4%2F0Ab-x HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").unwrap();
            let mut t = String::new();
            let _ = s.read_to_string(&mut t);
            t
        });
        let (mut s, _) = l.accept().unwrap();
        assert_eq!(terima_pengalihan(&mut s, "S1").unwrap(), Some("4/0Ab-x".to_string()));
        drop(s);
        assert!(browser.join().unwrap().contains("Login berhasil"));
        let browser = std::thread::spawn(move || {
            let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
            s.write_all(b"GET /?state=LAIN&code=x HTTP/1.1\r\n\r\n").unwrap();
        });
        let (mut s, _) = l.accept().unwrap();
        assert!(terima_pengalihan(&mut s, "S1").is_err());
        browser.join().unwrap();
    }
}
