//! Riwayat penyalinan, verifikasi, dan pemindahan, beserta laporannya (CSV dan HTML siap cetak).
//!
//! Satu baris JSON per kejadian di %LOCALAPPDATA%\pindah-drive\riwayat.jsonl. Ditambahkan di akhir
//! berkas (bukan ditulis ulang), karena pekerja latar, backup terjadwal, dan jendela aplikasi bisa
//! mencatat hampir bersamaan. Riwayat tidak ikut dihapus bersama log lama: ia catatan audit.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use super::akun::{jenis_dari_nama, remote_dari};
use super::dasar::{angka, format_ukuran, jalur, pastikan_dir, sekarang_iso, waktu_lokal, waktu_tampil};

static RE_OFFICE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\.(docx|docm|dotx|xlsx|xlsm|xltx|pptx|pptm|potx|vsdx)$").unwrap());

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct Catatan {
    pub id: String,
    /// salin | verifikasi | pindahkan
    pub jenis: String,
    pub mulai: String,
    pub selesai: String,
    pub sumber: String,
    pub tujuan: String,
    /// salin: selesai | galat | dihentikan | dilewati;  verifikasi: cocok | selisih | galat;
    /// pindahkan: selesai | sebagian | galat
    pub hasil: String,
    pub ket: String,
    /// salin: file baru tersalin;  verifikasi: file cocok;  pindahkan: file dihapus dari sumber.
    pub file: i64,
    /// salin: data tersalin (byte).
    pub byte: i64,
    pub hilang: i64,
    pub beda: i64,
    pub galat: i64,
    /// verifikasi: checksum | ukuran
    pub metode: String,
    /// Uraian filter file yang dipakai ("" = semua file).
    pub filter: String,
    /// Nama jadwal bila dijalankan oleh backup terjadwal.
    pub jadwal: String,
    pub log: String,
    /// verifikasi/pindahkan: daftar status per file (format --combined rclone).
    pub laporan: String,
}

fn berkas() -> PathBuf {
    jalur().data.join("riwayat.jsonl")
}

pub fn id_baru() -> String {
    let (y, mo, d, h, mi, s) = waktu_lokal();
    let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0);
    format!("{y:04}{mo:02}{d:02}-{h:02}{mi:02}{s:02}-{:04x}", (n ^ std::process::id()) & 0xFFFF)
}

/// Menambah satu catatan di akhir riwayat.jsonl.
pub fn catat(mut c: Catatan) {
    if c.id.is_empty() {
        c.id = id_baru();
    }
    if c.selesai.is_empty() {
        c.selesai = sekarang_iso();
    }
    let Ok(mut baris) = serde_json::to_string(&c) else { return };
    baris.push('\n');
    pastikan_dir();
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(berkas()) {
        // Satu panggilan write per baris: penulis lain tidak menyisip di tengah baris.
        let _ = f.write_all(baris.as_bytes());
    }
}

/// Catatan terbaru dulu. `maks` 0 = semua.
pub fn baca(maks: usize) -> Vec<Catatan> {
    let isi = std::fs::read_to_string(berkas()).unwrap_or_default();
    let mut d: Vec<Catatan> = isi.lines().filter_map(|l| serde_json::from_str(l.trim()).ok()).collect();
    d.reverse();
    if maks > 0 {
        d.truncate(maks);
    }
    d
}

pub fn label_jenis(c: &Catatan) -> &'static str {
    match c.jenis.as_str() {
        "verifikasi" => "Verifikasi",
        "pindahkan" => "Pindahkan",
        _ => "Salin",
    }
}

pub fn label_hasil(c: &Catatan) -> &'static str {
    match c.hasil.as_str() {
        "selesai" => "Selesai",
        "cocok" => "Semua cocok",
        "selisih" => "Ada selisih",
        "dihentikan" => "Dihentikan",
        "dilewati" => "Dilewati",
        "sebagian" => "Sebagian",
        _ => "Gagal",
    }
}

/// Hasil yang baik (hijau di aplikasi).
pub fn hasil_baik(c: &Catatan) -> bool {
    matches!(c.hasil.as_str(), "selesai" | "cocok")
}

/// "3 file baru (1,20 GB)" / "120 file cocok" / "12 file dipindahkan".
pub fn ringkas(c: &Catatan) -> String {
    match c.jenis.as_str() {
        "verifikasi" if c.hasil == "galat" && c.file == 0 => c.ket.clone(),
        "verifikasi" => {
            let mut t = format!("{} file cocok", angka(c.file));
            for (n, k) in [(c.hilang, "belum ada"), (c.beda, "berbeda"), (c.galat, "gagal dibaca")] {
                if n > 0 {
                    t += &format!(", {} {k}", angka(n));
                }
            }
            t
        }
        "pindahkan" => format!("{} file dihapus dari sumber", angka(c.file)),
        _ if matches!(c.hasil.as_str(), "selesai") && c.file == 0 => "Tidak ada file baru".into(),
        _ if c.hasil == "selesai" => format!("{} file baru ({})", angka(c.file), format_ukuran(c.byte as f64)),
        _ => c.ket.clone(),
    }
}

// ------------------------------------------------------------------ per file (laporan verifikasi)

/// Satu baris laporan --combined rclone: ("=", "Kerja/a.pdf").
pub fn baca_per_file(laporan: &Path) -> Vec<(char, String)> {
    std::fs::read_to_string(laporan)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let mut c = l.chars();
            let s = c.next()?;
            let sisa = c.as_str().strip_prefix(' ')?;
            "=-+*!".contains(s).then(|| (s, sisa.to_string()))
        })
        .collect()
}

/// Status per file dalam kata-kata. Tujuan SharePoint: file Office yang "berbeda" karena metadata.
pub fn status_file(s: char, file: &str, tujuan: &str) -> &'static str {
    match s {
        '=' => "Cocok",
        '-' => "Belum ada di tujuan",
        '+' => "Hanya ada di tujuan",
        '*' if sharepoint(tujuan) && RE_OFFICE.is_match(file) => "Cocok (metadata Office diubah SharePoint)",
        '*' => "Isinya berbeda",
        _ => "Gagal dibaca",
    }
}

fn sharepoint(tujuan: &str) -> bool {
    let r = remote_dari(tujuan);
    !r.is_empty() && jenis_dari_nama(&r) == "sharepoint"
}

// ------------------------------------------------------------------ CSV

fn sel_csv(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r', ';']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn baris_csv(sel: &[String]) -> String {
    sel.iter().map(|s| sel_csv(s)).collect::<Vec<_>>().join(",") + "\r\n"
}

/// CSV riwayat (UTF-8 ber-BOM agar Excel membaca huruf non-ASCII dengan benar).
pub fn csv(daftar: &[Catatan]) -> String {
    let mut t = String::from("\u{feff}");
    t += &baris_csv(
        &[
            "Mulai", "Selesai", "Jenis", "Jadwal", "Sumber", "Tujuan", "Filter", "Hasil", "File", "Byte", "Belum ada",
            "Berbeda", "Gagal dibaca", "Metode", "Keterangan", "Log", "Laporan per file",
        ]
        .map(String::from),
    );
    for c in daftar {
        t += &baris_csv(&[
            c.mulai.replace('T', " "),
            c.selesai.replace('T', " "),
            label_jenis(c).into(),
            c.jadwal.clone(),
            c.sumber.clone(),
            c.tujuan.clone(),
            c.filter.clone(),
            label_hasil(c).into(),
            c.file.to_string(),
            c.byte.to_string(),
            c.hilang.to_string(),
            c.beda.to_string(),
            c.galat.to_string(),
            c.metode.clone(),
            c.ket.clone(),
            c.log.clone(),
            c.laporan.clone(),
        ]);
    }
    t
}

/// CSV status per file dari satu verifikasi.
pub fn csv_per_file(c: &Catatan) -> String {
    let mut t = String::from("\u{feff}");
    t += &baris_csv(&["Status".to_string(), "File".to_string()]);
    for (s, f) in baca_per_file(Path::new(&c.laporan)) {
        t += &baris_csv(&[status_file(s, &f, &c.tujuan).to_string(), f]);
    }
    t
}

// ------------------------------------------------------------------ HTML siap cetak

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn kepala_html(judul: &str) -> String {
    let komputer = crate::win::nama_komputer();
    let pengguna = crate::win::nama_pengguna();
    format!(
        r#"<!doctype html>
<html lang="id"><head><meta charset="utf-8"><title>{j}</title>
<style>
  :root {{ color-scheme: light; }}
  body {{ font: 13px/1.45 "Segoe UI", system-ui, sans-serif; color: #1c1f1d; margin: 32px auto; max-width: 1040px; padding: 0 20px; }}
  h1 {{ font-size: 20px; margin: 0 0 4px; }}
  h2 {{ font-size: 15px; margin: 22px 0 8px; }}
  .meta {{ color: #5d6560; margin-bottom: 18px; }}
  table {{ border-collapse: collapse; width: 100%; }}
  th, td {{ border: 1px solid #d5dbd7; padding: 5px 8px; text-align: left; vertical-align: top; }}
  th {{ background: #eef2ef; font-weight: 600; }}
  td.n {{ text-align: right; white-space: nowrap; }}
  td.j {{ word-break: break-all; }}
  .baik {{ color: #1f7a3f; font-weight: 600; }}
  .awas {{ color: #9a6200; font-weight: 600; }}
  .gagal {{ color: #b3261e; font-weight: 600; }}
  dl {{ display: grid; grid-template-columns: max-content 1fr; gap: 4px 16px; margin: 0; }}
  dt {{ color: #5d6560; }} dd {{ margin: 0; word-break: break-all; }}
  .catatan {{ color: #5d6560; margin-top: 18px; font-size: 12px; }}
  @media print {{ body {{ margin: 0; max-width: none; }} a {{ color: inherit; }} }}
</style></head><body>
<h1>{j}</h1>
<div class="meta">Dibuat {w} oleh LELEMOVE {v} &middot; komputer {k} &middot; pengguna {p}</div>
"#,
        j = esc(judul),
        w = esc(&waktu_tampil(&sekarang_iso())),
        v = esc(crate::VERSI),
        k = esc(&komputer),
        p = esc(&pengguna),
    )
}

fn kelas(c: &Catatan) -> &'static str {
    match c.hasil.as_str() {
        "selesai" | "cocok" => "baik",
        "selisih" | "dihentikan" | "dilewati" | "sebagian" => "awas",
        _ => "gagal",
    }
}

/// Laporan riwayat (daftar kejadian), urut dari yang terbaru.
pub fn html_riwayat(daftar: &[Catatan]) -> String {
    let mut t = kepala_html("Laporan riwayat LELEMOVE");
    if let (Some(baru), Some(lama)) = (daftar.first(), daftar.last()) {
        t += &format!(
            "<p>{} kejadian, dari {} sampai {}.</p>\n",
            angka(daftar.len() as i64),
            esc(&waktu_tampil(&lama.mulai)),
            esc(&waktu_tampil(&baru.selesai))
        );
    } else {
        t += "<p>Belum ada riwayat.</p>\n";
    }
    t += "<table><tr><th>Waktu</th><th>Jenis</th><th>Sumber &rarr; tujuan</th><th>Hasil</th><th>Ringkasan</th></tr>\n";
    for c in daftar {
        let mut jenis = label_jenis(c).to_string();
        if !c.jadwal.is_empty() {
            jenis += &format!("<br><small>jadwal: {}</small>", esc(&c.jadwal));
        }
        let mut rute = format!("{}<br>&rarr; {}", esc(&c.sumber), esc(&c.tujuan));
        if !c.filter.is_empty() {
            rute += &format!("<br><small>filter: {}</small>", esc(&c.filter));
        }
        t += &format!(
            "<tr><td>{}</td><td>{jenis}</td><td class=\"j\">{rute}</td><td class=\"{}\">{}</td><td>{}</td></tr>\n",
            esc(&waktu_tampil(&c.mulai)),
            kelas(c),
            label_hasil(c),
            esc(&ringkas(c)),
        );
    }
    t += "</table>\n";
    t += CATATAN_CETAK;
    t
}

/// Laporan satu verifikasi beserta status setiap file: bukti bahwa salinan lengkap.
pub fn html_verifikasi(c: &Catatan) -> String {
    let mut t = kepala_html("Laporan verifikasi LELEMOVE");
    let status = match c.hasil.as_str() {
        "cocok" => "<span class=\"baik\">SEMUA FILE COCOK</span>",
        "selisih" => "<span class=\"awas\">ADA SELISIH</span>",
        _ => "<span class=\"gagal\">VERIFIKASI GAGAL</span>",
    };
    let metode = if c.metode == "ukuran" {
        "nama dan ukuran (kedua layanan tidak punya jenis checksum yang sama)"
    } else {
        "checksum isi file"
    };
    t += "<dl>\n";
    for (k, v) in [
        ("Hasil", status.to_string()),
        ("Waktu", esc(&waktu_tampil(&c.mulai))),
        ("Sumber", esc(&c.sumber)),
        ("Tujuan", esc(&c.tujuan)),
        ("Dicocokkan dengan", metode.to_string()),
        ("Filter", if c.filter.is_empty() { "semua file".into() } else { esc(&c.filter) }),
        ("Cocok", angka(c.file)),
        ("Belum ada di tujuan", angka(c.hilang)),
        ("Isinya berbeda", angka(c.beda)),
        ("Gagal dibaca", angka(c.galat)),
    ] {
        t += &format!("<dt>{k}</dt><dd>{v}</dd>\n");
    }
    if !c.jadwal.is_empty() {
        t += &format!("<dt>Jadwal</dt><dd>{}</dd>\n", esc(&c.jadwal));
    }
    t += "</dl>\n";
    let per_file = baca_per_file(Path::new(&c.laporan));
    if per_file.is_empty() {
        t += "<p class=\"catatan\">Daftar per file tidak tersedia (berkas laporannya sudah terhapus atau verifikasi gagal berjalan).</p>\n";
    } else {
        const MAKS: usize = 50_000;
        t += &format!("<h2>Daftar file ({})</h2>\n", angka(per_file.len() as i64));
        t += "<table><tr><th>No</th><th>Status</th><th>File</th></tr>\n";
        // Yang bermasalah ditampilkan lebih dulu.
        let mut urut: Vec<&(char, String)> = per_file.iter().collect();
        urut.sort_by_key(|(s, f)| (*s == '=' || (*s == '*' && sharepoint(&c.tujuan) && RE_OFFICE.is_match(f)), f.to_lowercase()));
        for (i, (s, f)) in urut.iter().take(MAKS).enumerate() {
            let st = status_file(*s, f, &c.tujuan);
            let k = if st.starts_with("Cocok") { "baik" } else { "awas" };
            t += &format!("<tr><td class=\"n\">{}</td><td class=\"{k}\">{st}</td><td class=\"j\">{}</td></tr>\n", i + 1, esc(f));
        }
        t += "</table>\n";
        if per_file.len() > MAKS {
            t += &format!(
                "<p class=\"catatan\">Hanya {} baris pertama yang ditampilkan; daftar lengkap ada di CSV per file.</p>\n",
                angka(MAKS as i64)
            );
        }
    }
    t += CATATAN_CETAK;
    t
}

/// Petunjuk simpan-ke-PDF di akhir laporan HTML.
#[cfg(windows)]
const CATATAN_CETAK: &str = "<p class=\"catatan\">Simpan sebagai PDF lewat menu Cetak browser (Ctrl+P &gt; Save as PDF / Microsoft Print to PDF).</p>\n</body></html>\n";

#[cfg(target_os = "macos")]
const CATATAN_CETAK: &str = "<p class=\"catatan\">Simpan sebagai PDF lewat menu Cetak browser (Cmd+P &gt; PDF &gt; Simpan sebagai PDF).</p>\n</body></html>\n";
#[cfg(test)]
mod uji {
    use super::*;

    #[test]
    fn csv_dan_ringkas() {
        let c = Catatan {
            jenis: "salin".into(),
            mulai: "2026-11-23T16:24:01".into(),
            sumber: "D:\\Data, \"lama\"".into(),
            tujuan: "pd-gdrive:Arsip".into(),
            hasil: "selesai".into(),
            file: 12,
            byte: 1_288_490_189,
            ..Default::default()
        };
        assert_eq!(ringkas(&c), "12 file baru (1,20 GB)");
        let t = csv(std::slice::from_ref(&c));
        assert!(t.starts_with('\u{feff}'));
        assert!(t.contains("2026-11-23 16:24:01,,Salin,,\"D:\\Data, \"\"lama\"\"\",pd-gdrive:Arsip,,Selesai,12,1288490189"));
        let v = Catatan { jenis: "verifikasi".into(), hasil: "selisih".into(), file: 10, hilang: 2, ..Default::default() };
        assert_eq!(ringkas(&v), "10 file cocok, 2 belum ada");
        assert_eq!(label_hasil(&v), "Ada selisih");
    }

    #[test]
    fn per_file() {
        let d = std::env::temp_dir().join(format!("lelemove-uji-riwayat-{}.txt", std::process::id()));
        std::fs::write(&d, "= a.pdf\n- b/c.xlsx\n* Laporan.docx\n! rusak.bin\ntidak valid\n").unwrap();
        let f = baca_per_file(&d);
        assert_eq!(f.len(), 4);
        assert_eq!(f[1], ('-', "b/c.xlsx".to_string()));
        assert_eq!(status_file('*', "Laporan.docx", "pd-gdrive:x"), "Isinya berbeda");
        let c = Catatan { jenis: "verifikasi".into(), hasil: "selisih".into(), laporan: d.to_string_lossy().into(), ..Default::default() };
        let h = html_verifikasi(&c);
        assert!(h.contains("ADA SELISIH") && h.contains("b/c.xlsx") && h.contains("Daftar file (4)"));
        // Yang bermasalah tampil sebelum yang cocok.
        assert!(h.find("b/c.xlsx").unwrap() < h.find("a.pdf").unwrap());
        let _ = std::fs::remove_file(&d);
    }
}
