//! Filter file: hanya ekstensi tertentu, lewati pola tertentu, atau hanya file yang diubah sejak
//! tanggal tertentu. Dipakai sama persis oleh periksa, uji, salin, dan verifikasi, supaya file yang
//! sengaja tidak disalin tidak dilaporkan "belum ada di tujuan".
//!
//! Disimpan di pengaturan.json ("Filter") untuk rute yang sedang dibuka, dan di jadwal.json
//! (salinan sendiri per jadwal).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::dasar::{baca_objek, jalur, waktu_lokal, KECUALIAN};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Filter {
    /// Ekstensi yang disertakan, tanpa titik: ["pdf", "docx"]. Kosong = semua jenis file.
    #[serde(default)]
    pub sertakan: Vec<String>,
    /// Pola yang dilewati (glob rclone): ["*.tmp", "node_modules/**"].
    #[serde(default)]
    pub lewati: Vec<String>,
    /// Hanya file yang diubah sejak tanggal ini ("2026-01-31").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sejak: Option<String>,
}

impl Filter {
    pub fn kosong(&self) -> bool {
        self.sertakan.is_empty() && self.lewati.is_empty() && self.sejak.is_none()
    }

    /// Dari isian bebas: sertakan "pdf, .docx, *.xlsx"; lewati "*.tmp; node_modules\";
    /// sejak "2026-01-31", "31-01-2026", atau "31/1/2026". Isian kosong = tidak difilter.
    pub fn urai(sertakan: &str, lewati: &str, sejak: &str) -> Result<Filter, String> {
        let pisah = |s: &str| -> Vec<String> {
            s.split([',', ';', '\n']).map(|b| b.trim().to_string()).filter(|b| !b.is_empty()).collect()
        };
        let mut f = Filter::default();
        for e in pisah(sertakan) {
            let e = e.trim_start_matches('*').trim_start_matches('.').to_lowercase();
            if e.is_empty() || !e.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
                return Err(format!("Jenis file tidak dikenal: \"{e}\". Tulis ekstensinya saja, mis. pdf, docx, xlsx."));
            }
            if !f.sertakan.contains(&e) {
                f.sertakan.push(e);
            }
        }
        for p in pisah(lewati) {
            let mut p = p.replace('\\', "/");
            // "node_modules/" (folder) -> semua isinya, di kedalaman mana pun.
            if p.ends_with('/') {
                p.push_str("**");
            }
            if !f.lewati.contains(&p) {
                f.lewati.push(p);
            }
        }
        let s = sejak.trim();
        if !s.is_empty() {
            f.sejak = Some(urai_tanggal(s).ok_or_else(|| {
                format!("Tanggal tidak dikenal: \"{s}\". Contoh: 2026-01-31 atau 31-01-2026.")
            })?);
        }
        Ok(f)
    }

    /// Argumen rclone: pengecualian bawaan + filter ini. Selalu pakai --filter (bukan campuran
    /// --include/--exclude, yang urutannya tidak pasti) dan --ignore-case (Windows).
    pub fn argumen(&self) -> Vec<String> {
        let mut a = vec!["--ignore-case".to_string()];
        let mut aturan = |t: String| {
            a.push("--filter".into());
            a.push(t);
        };
        for k in KECUALIAN {
            aturan(format!("- {k}"));
        }
        for l in &self.lewati {
            aturan(format!("- {l}"));
        }
        match self.sertakan.as_slice() {
            [] => {}
            [satu] => aturan(format!("+ *.{satu}")),
            banyak => aturan(format!("+ *.{{{}}}", banyak.join(","))),
        }
        if !self.sertakan.is_empty() {
            aturan("- **".into());
        }
        if let Some(t) = &self.sejak {
            a.push("--max-age".into());
            a.push(t.clone());
        }
        a
    }

    /// Folder kosong di sumber ikut dibuat di tujuan hanya bila tidak memilih jenis/tanggal;
    /// dengan filter itu, rclone akan membuat SEMUA subfolder walau isinya tidak ada yang lolos.
    pub fn buat_folder_kosong(&self) -> bool {
        self.sertakan.is_empty() && self.sejak.is_none()
    }

    /// "hanya pdf, docx · lewati *.tmp · diubah sejak 31 Jan 2026"; "" bila tidak ada filter.
    pub fn uraian(&self) -> String {
        let mut b = Vec::new();
        if !self.sertakan.is_empty() {
            b.push(format!("hanya {}", self.sertakan.join(", ")));
        }
        if !self.lewati.is_empty() {
            // "node_modules/**" ditulis seperti yang diketik pengguna: "node_modules/".
            let l: Vec<&str> = self.lewati.iter().map(|l| l.strip_suffix("**").unwrap_or(l)).collect();
            b.push(format!("lewati {}", l.join(", ")));
        }
        if let Some(t) = &self.sejak {
            b.push(format!("diubah sejak {}", tanggal_tampil(t)));
        }
        b.join(" \u{00B7} ")
    }

    /// Isian untuk dialog/baris perintah (kebalikan dari `urai`).
    pub fn teks_sertakan(&self) -> String {
        self.sertakan.join(", ")
    }

    pub fn teks_lewati(&self) -> String {
        self.lewati.join(", ")
    }
}

/// "2026-01-31" | "31-01-2026" | "31/1/2026" | "31.01.2026" -> "2026-01-31".
pub fn urai_tanggal(s: &str) -> Option<String> {
    let n: Vec<u32> = s.split(['-', '/', '.']).map(|b| b.trim().parse().ok()).collect::<Option<Vec<_>>>()?;
    let (y, m, d) = match n.as_slice() {
        [y, m, d] if *y >= 1000 => (*y, *m, *d),
        [d, m, y] if *y >= 1000 => (*y, *m, *d),
        _ => return None,
    };
    let hari_bulan = [31, if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if !(1970..=2200).contains(&y) || !(1..=12).contains(&m) || d == 0 || d > hari_bulan[m as usize - 1] {
        return None;
    }
    Some(format!("{y:04}-{m:02}-{d:02}"))
}

/// "2026-01-31" -> "31 Jan 2026".
pub fn tanggal_tampil(t: &str) -> String {
    const BULAN: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "Mei", "Jun", "Jul", "Agu", "Sep", "Okt", "Nov", "Des"];
    let n: Vec<usize> = t.split('-').filter_map(|b| b.parse().ok()).collect();
    match n.as_slice() {
        [y, m, d] if (1..=12).contains(m) => format!("{d} {} {y}", BULAN[m - 1]),
        _ => t.to_string(),
    }
}

/// Tanggal awal bulan / awal tahun ini (pilihan cepat di dialog filter).
pub fn awal_bulan_ini() -> String {
    let (y, m, _, _, _, _) = waktu_lokal();
    format!("{y:04}-{m:02}-01")
}

pub fn awal_tahun_ini() -> String {
    let (y, _, _, _, _, _) = waktu_lokal();
    format!("{y:04}-01-01")
}

/// Filter rute yang sedang dibuka (pengaturan.json).
pub fn baca_filter() -> Filter {
    baca_objek(&jalur().pengaturan)
        .and_then(|p| p.get("Filter").cloned())
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

pub fn simpan_filter(f: &Filter) {
    let v = if f.kosong() { Value::Null } else { serde_json::to_value(f).unwrap_or(Value::Null) };
    super::rute::ubah_pengaturan(|m| {
        m.insert("Filter".into(), v);
    });
}

#[cfg(test)]
mod uji {
    use super::*;

    #[test]
    fn urai_dan_argumen() {
        let f = Filter::urai(" PDF, .docx ,*.xlsx, pdf", "*.tmp; node_modules\\ ", "31/1/2026").unwrap();
        assert_eq!(f.sertakan, vec!["pdf", "docx", "xlsx"]);
        assert_eq!(f.lewati, vec!["*.tmp", "node_modules/**"]);
        assert_eq!(f.sejak.as_deref(), Some("2026-01-31"));
        let a = f.argumen().join(" ");
        assert!(a.starts_with("--ignore-case --filter - desktop.ini"));
        assert!(a.contains("--filter - *.tmp --filter - node_modules/** --filter + *.{pdf,docx,xlsx} --filter - **"));
        assert!(a.ends_with("--max-age 2026-01-31"));
        assert!(!f.buat_folder_kosong());
        assert_eq!(f.uraian(), "hanya pdf, docx, xlsx \u{00B7} lewati *.tmp, node_modules/ \u{00B7} diubah sejak 31 Jan 2026");

        let satu = Filter::urai("pdf", "", "").unwrap();
        assert!(satu.argumen().join(" ").contains("--filter + *.pdf --filter - **"));
        let kosong = Filter::urai("", " ", "").unwrap();
        assert!(kosong.kosong() && kosong.buat_folder_kosong() && kosong.uraian().is_empty());
        assert!(!kosong.argumen().iter().any(|x| x == "- **" || x == "--max-age"));
    }

    #[test]
    fn tanggal() {
        assert_eq!(urai_tanggal("2026-02-28").as_deref(), Some("2026-02-28"));
        assert_eq!(urai_tanggal("29-02-2024").as_deref(), Some("2024-02-29"));
        assert_eq!(urai_tanggal("29-02-2026"), None);
        assert_eq!(urai_tanggal("2026-13-01"), None);
        assert_eq!(urai_tanggal("kemarin"), None);
        assert!(Filter::urai("p.df", "", "").is_err());
        assert!(Filter::urai("", "", "32-01-2026").is_err());
        assert_eq!(tanggal_tampil("2026-11-05"), "5 Nov 2026");
    }
}
