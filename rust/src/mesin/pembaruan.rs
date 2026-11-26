//! Pembaruan LELEMOVE dari GitHub Releases (perintah `pindah pembaruan`).
//!
//! Hanya berjalan bila diminta (tombol "Periksa pembaruan" atau perintahnya). Versi terbaru dibaca
//! dari pengalihan halaman github.com/<repo>/releases/latest ke /releases/tag/vX.Y.Z, bukan dari
//! API GitHub: API tanpa login dibatasi 60 permintaan per jam per alamat internet, dan di jaringan
//! kantor (satu alamat untuk banyak komputer) batas itu cepat habis. Alamat unduhan berkas rilis
//! berpola tetap (/releases/download/<tag>/<berkas>). Berkas untuk sistem ini (installer Windows
//! atau .dmg macOS) dicocokkan SHA-256-nya dengan berkas `.sha256` di rilis yang sama; yang tidak
//! cocok dihapus dan tidak pernah dijalankan.

use native_tls::TlsStream;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::time::Duration;

use super::kode::sha256;

const REPO: &str = "https://github.com/fauzirpl/lelemove";

#[derive(Clone, Debug, PartialEq)]
pub struct Rilis {
    pub versi: String,
    pub tag: String,
    /// Halaman rilis di GitHub (berisi catatan rilis).
    pub halaman: String,
    /// Alamat berkas untuk sistem ini dan berkas `.sha256`-nya.
    pub berkas: String,
    pub sidik: String,
}

/// "v2.4.1" / "2.4.1" -> [2, 4, 1].
fn nomor(v: &str) -> [u64; 3] {
    let mut n = [0u64; 3];
    for (i, b) in v.trim().trim_start_matches(['v', 'V']).split(['.', '-', '+']).take(3).enumerate() {
        n[i] = b.parse().unwrap_or(0);
    }
    n
}

/// `baru` lebih baru daripada `kini`?
pub fn lebih_baru(baru: &str, kini: &str) -> bool {
    nomor(baru) > nomor(kini)
}

/// Nama berkas rilis untuk sistem ini.
pub fn nama_berkas(versi: &str) -> String {
    if cfg!(windows) {
        format!("LELEMOVE-Setup-{versi}.exe")
    } else {
        format!("LELEMOVE-{versi}-macOS.dmg")
    }
}

/// Rilis dari alamat tujuan pengalihan ".../releases/tag/v2.4.1".
pub fn rilis_dari_lokasi(lokasi: &str) -> Result<Rilis, String> {
    let tag = lokasi
        .split("/releases/tag/")
        .nth(1)
        .map(|t| t.split(['?', '#', '/']).next().unwrap_or(""))
        .filter(|t| !t.is_empty() && t.trim_start_matches(['v', 'V']).starts_with(|c: char| c.is_ascii_digit()))
        .ok_or_else(|| format!("GitHub belum punya rilis yang bisa dipakai ({lokasi})"))?;
    let versi = tag.trim_start_matches(['v', 'V']).to_string();
    let nama = nama_berkas(&versi);
    Ok(Rilis {
        halaman: format!("{REPO}/releases/tag/{tag}"),
        berkas: format!("{REPO}/releases/download/{tag}/{nama}"),
        sidik: format!("{REPO}/releases/download/{tag}/{nama}.sha256"),
        tag: tag.to_string(),
        versi,
    })
}

/// Membaca rilis terbaru dari pengalihan halaman releases/latest.
pub fn cek() -> Result<Rilis, String> {
    let j = buka(&format!("{REPO}/releases/latest"), "text/html")?;
    match (j.status, j.lokasi) {
        (300..=399, Some(l)) => rilis_dari_lokasi(&l),
        (404, _) => Err("repositori atau rilis tidak ditemukan di GitHub".into()),
        (s, _) => Err(format!("GitHub menjawab HTTP {s}")),
    }
}

fn heks(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Mengunduh berkas rilis untuk sistem ini ke folder sementara dan mencocokkan SHA-256-nya.
/// `kabar(diterima, total)` dipanggil selama mengunduh.
pub fn unduh(r: &Rilis, kabar: &mut dyn FnMut(u64, u64)) -> Result<PathBuf, String> {
    let nama = nama_berkas(&r.versi);
    let mut teks = Vec::new();
    minta(&r.sidik, &mut |b| {
        teks.extend_from_slice(b);
        Ok(())
    }, &mut |_, _| {})
    .map_err(|e| format!("berkas {nama}.sha256 tidak bisa diunduh ({e}); demi keamanan installer tidak diunduh"))?;
    let harap = String::from_utf8_lossy(&teks).split_whitespace().next().unwrap_or("").to_lowercase();
    if harap.len() != 64 || !harap.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err("isi berkas .sha256 di rilis tidak sah".into());
    }
    let dir = std::env::temp_dir().join("lelemove-pembaruan");
    fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let tujuan = dir.join(&nama);
    let sementara = dir.join(format!("{nama}.unduhan"));
    {
        let mut f = File::create(&sementara).map_err(|e| format!("{}: {e}", sementara.display()))?;
        minta(&r.berkas, &mut |b| f.write_all(b), kabar).map_err(|e| format!("{nama}: {e}"))?;
    }
    let data = fs::read(&sementara).map_err(|e| e.to_string())?;
    let nyata = heks(&sha256(&data));
    if nyata != harap {
        let _ = fs::remove_file(&sementara);
        return Err(format!("SHA-256 unduhan ({nyata}) tidak cocok dengan rilis ({harap}); berkas dihapus"));
    }
    let _ = fs::remove_file(&tujuan);
    fs::rename(&sementara, &tujuan).map_err(|e| e.to_string())?;
    Ok(tujuan)
}

/// Jawaban HTTP yang header-nya sudah dibaca; badan masih di `r`.
struct Jawaban {
    status: u16,
    panjang: Option<u64>,
    chunked: bool,
    lokasi: Option<String>,
    r: BufReader<TlsStream<TcpStream>>,
}

/// GET https satu kali (tanpa mengikuti pengalihan): status dan header.
fn buka(url: &str, terima: &str) -> Result<Jawaban, String> {
    let sisa = url.strip_prefix("https://").ok_or_else(|| format!("bukan alamat https: {url}"))?;
    let (host, jalur) = sisa.split_once('/').map(|(h, j)| (h, format!("/{j}"))).unwrap_or((sisa, "/".into()));
    let a = (host, 443).to_socket_addrs().map_err(|e| format!("{host} tidak ditemukan ({e})"))?.next().ok_or("alamat kosong")?;
    let tcp = TcpStream::connect_timeout(&a, Duration::from_secs(20)).map_err(|e| format!("{host}: {e}"))?;
    let _ = tcp.set_read_timeout(Some(Duration::from_secs(60)));
    let tls = native_tls::TlsConnector::new().map_err(|e| e.to_string())?;
    let mut s = tls.connect(host, tcp).map_err(|e| format!("sambungan aman ke {host} gagal: {e}"))?;
    let req = format!("GET {jalur} HTTP/1.1\r\nHost: {host}\r\nUser-Agent: LELEMOVE/{}\r\nAccept: {terima}\r\nConnection: close\r\n\r\n", crate::VERSI);
    s.write_all(req.as_bytes()).map_err(|e| e.to_string())?;
    let mut r = BufReader::with_capacity(1 << 16, s);
    let mut baris = String::new();
    r.read_line(&mut baris).map_err(|e| format!("jawaban {host} terputus: {e}"))?;
    let status: u16 = baris.split_whitespace().nth(1).and_then(|x| x.parse().ok()).ok_or("status HTTP tidak terbaca")?;
    let mut j = Jawaban { status, panjang: None, chunked: false, lokasi: None, r };
    loop {
        baris.clear();
        j.r.read_line(&mut baris).map_err(|e| format!("jawaban {host} terputus: {e}"))?;
        let t = baris.trim_end();
        if t.is_empty() {
            break;
        }
        if let Some((k, v)) = t.split_once(':') {
            match k.trim().to_ascii_lowercase().as_str() {
                "content-length" => j.panjang = v.trim().parse().ok(),
                "transfer-encoding" => j.chunked = v.to_ascii_lowercase().contains("chunked"),
                "location" => {
                    let l = v.trim();
                    j.lokasi = Some(if l.starts_with("https://") { l.to_string() } else { format!("https://{host}{l}") });
                }
                _ => {}
            }
        }
    }
    Ok(j)
}

/// GET https mengikuti pengalihan (unduhan aset dialihkan ke server berkas GitHub). Badan
/// diteruskan potong demi potong ke `tulis`, jadi berkas besar tidak ditampung di memori.
fn minta(url: &str, tulis: &mut dyn FnMut(&[u8]) -> io::Result<()>, kabar: &mut dyn FnMut(u64, u64)) -> Result<(), String> {
    let mut url = url.to_string();
    for _ in 0..6 {
        let mut j = buka(&url, "application/octet-stream")?;
        if (300..400).contains(&j.status) {
            url = j.lokasi.ok_or("pengalihan tanpa alamat")?;
            continue;
        }
        if j.status != 200 {
            return Err(match j.status {
                404 => "berkas tidak ada di rilis".into(),
                s => format!("GitHub menjawab HTTP {s}"),
            });
        }
        let total = j.panjang.unwrap_or(0);
        let mut n: u64 = 0;
        let mut buf = vec![0u8; 1 << 16];
        let kapasitas = buf.len() as u64;
        if j.chunked {
            let mut baris = String::new();
            loop {
                baris.clear();
                j.r.read_line(&mut baris).map_err(|e| e.to_string())?;
                let ukuran = u64::from_str_radix(baris.trim().split(';').next().unwrap_or("").trim(), 16).unwrap_or(0);
                if ukuran == 0 {
                    break;
                }
                let mut sisa = ukuran;
                while sisa > 0 {
                    let k = j.r.read(&mut buf[..sisa.min(kapasitas) as usize]).map_err(|e| e.to_string())?;
                    if k == 0 {
                        return Err("unduhan terputus".into());
                    }
                    tulis(&buf[..k]).map_err(|e| e.to_string())?;
                    n += k as u64;
                    sisa -= k as u64;
                    kabar(n, total);
                }
                baris.clear();
                let _ = j.r.read_line(&mut baris);
            }
        } else {
            loop {
                let maks = match j.panjang {
                    Some(p) if n >= p => break,
                    Some(p) => (p - n).min(kapasitas) as usize,
                    None => buf.len(),
                };
                let k = match j.r.read(&mut buf[..maks]) {
                    Ok(0) => break,
                    Ok(k) => k,
                    // Sebagian server menutup TLS tanpa close_notify.
                    Err(e) if e.kind() == io::ErrorKind::UnexpectedEof && j.panjang.is_none_or(|p| n >= p) => break,
                    Err(e) => return Err(format!("unduhan terputus: {e}")),
                };
                tulis(&buf[..k]).map_err(|e| e.to_string())?;
                n += k as u64;
                kabar(n, total);
            }
            if j.panjang.is_some_and(|p| n < p) {
                return Err(format!("unduhan terputus ({n} dari {total} bita)"));
            }
        }
        return Ok(());
    }
    Err("terlalu banyak pengalihan".into())
}

#[cfg(test)]
mod uji {
    use super::*;

    #[test]
    fn bandingkan_versi() {
        assert!(lebih_baru("v2.4.1", "2.4.0") && lebih_baru("2.10.0", "2.9.9") && lebih_baru("3.0", "2.99.99"));
        assert!(!lebih_baru("2.4.1", "2.4.1") && !lebih_baru("v2.4.0", "2.4.1"));
    }

    #[test]
    fn rilis_dari_pengalihan() {
        let r = rilis_dari_lokasi("https://github.com/fauzirpl/lelemove/releases/tag/v2.5.0").unwrap();
        let n = nama_berkas("2.5.0");
        assert_eq!((r.versi.as_str(), r.tag.as_str()), ("2.5.0", "v2.5.0"));
        assert_eq!(r.berkas, format!("https://github.com/fauzirpl/lelemove/releases/download/v2.5.0/{n}"));
        assert_eq!(r.sidik, format!("{}.sha256", r.berkas));
        // Tanpa rilis, GitHub mengalihkan ke daftar rilis (bukan /tag/).
        assert!(rilis_dari_lokasi("https://github.com/fauzirpl/lelemove/releases").is_err());
        assert!(rilis_dari_lokasi("https://github.com/x/y/releases/tag/nightly").is_err());
    }
}
