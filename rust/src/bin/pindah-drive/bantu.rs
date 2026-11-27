//! Jendela utama: regex statik dan fungsi bantu (bukan metode `App`).
use super::*;

pub(crate) static RE_PERSEN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\d+)%").unwrap());
pub(crate) static RE_KOMA: LazyLock<Regex> = LazyLock::new(|| Regex::new(r",\s*").unwrap());
pub(crate) static RE_DARI_TOTAL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(.+?)\s*/\s*(.+)$").unwrap());
pub(crate) static RE_UKURAN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"([\d.]+)\s*([KMGTP]?i?B)?").unwrap());
pub(crate) static RE_FILE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d+)\s*/\s*(\d+)").unwrap());
pub(crate) static RE_ETA: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"ETA\s+(\S+)").unwrap());
pub(crate) static RE_NOL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^0\b").unwrap());
pub(crate) static RE_KURUNG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*\(.*$").unwrap());
pub(crate) static RE_LOGIN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(http://127\.0\.0\.1:\d+/auth\S*)").unwrap());
pub(crate) static RE_AWAL_ERROR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\S+\s+\S+\s+ERROR\s*:\s*").unwrap());
/// Awalan baris log rclone: "2026/11/23 11:30:26 CRITICAL: ".
pub(crate) static RE_AWAL_LOG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\d{4}/\d\d/\d\d \d\d:\d\d:\d\d \w+\s*:\s*").unwrap());
pub(crate) static RE_HARI: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\d+)d").unwrap());
pub(crate) static RE_JAM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\d+)h").unwrap());
pub(crate) static RE_MENIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\d+)m(?:[^s]|$)").unwrap());
pub(crate) static RE_DETIK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"([\d.]+)s").unwrap());

/// "1.200 GiB" (format rclone) -> "1,17 GB"
pub(crate) fn ubah_ukuran(t: &str) -> String {
    match RE_UKURAN.captures(t) {
        Some(c) => ukuran(ke_byte(&c[1], c.get(2).map_or("", |m| m.as_str()))),
        None => t.to_string(),
    }
}

/// "1h2m3s" (format rclone) -> "1 jam 2 mnt"
pub(crate) fn ubah_durasi(d: &str) -> String {
    if d.is_empty() || d == "-" {
        return String::new();
    }
    let mut h = Vec::new();
    if let Some(c) = RE_HARI.captures(d) {
        h.push(format!("{} hari", &c[1]));
    }
    if let Some(c) = RE_JAM.captures(d) {
        h.push(format!("{} jam", &c[1]));
    }
    if let Some(c) = RE_MENIT.captures(d) {
        h.push(format!("{} mnt", &c[1]));
    }
    if h.len() < 2 {
        if let Some(c) = RE_DETIK.captures(d) {
            h.push(format!("{} dtk", c[1].parse::<f64>().unwrap_or(0.0) as i64));
        }
    }
    h.join(" ")
}

/// "pd-onedrive:Kerja/DLH" -> ("pd-onedrive", "Kerja/DLH")
pub(crate) fn pecah(j: &str) -> (String, String) {
    let rem = akun::remote_dari(j);
    if rem.is_empty() {
        return (String::new(), j.to_string());
    }
    (rem.clone(), j[rem.len() + 1..].trim_matches('/').to_string())
}

pub(crate) fn gabung(remote: &str, path: &str) -> String {
    let p = path.trim().trim_matches('"');
    if remote.is_empty() {
        return rapikan_sumber(p);
    }
    format!("{remote}:{}", p.replace('\\', "/").trim_matches('/'))
}

/// "Google Drive", "Google Drive 2", "OneDrive · Kantor"
pub(crate) fn nama_lokasi(a: Option<&Akun>) -> String {
    let Some(a) = a else { return "Komputer ini".into() };
    let dasar = jenis_tampil(&a.jenis);
    if a.label.starts_with(&dasar) || a.label == a.nama {
        a.label.clone()
    } else {
        format!("{dasar} {TITIK} {}", a.label)
    }
}

pub(crate) fn h_ada(h: &Value, k: &str) -> bool {
    h.get(k).is_some_and(|v| !v.is_null())
}

pub(crate) fn h_bil(h: &Value, k: &str) -> i64 {
    bil(h.get(k)).unwrap_or(0)
}

pub(crate) fn h_bool(h: &Value, k: &str, nilai: bool) -> bool {
    h.get(k) == Some(&Value::Bool(nilai))
}

/// Desktop, Dokumen, dan folder OneDrive yang tersinkron di komputer ini.
pub(crate) fn pintasan_lokal() -> Vec<Pintasan> {
    let mut v = Vec::new();
    for (f, nama, glyph) in [(win::Folder::Desktop, "Desktop", ik::DESKTOP), (win::Folder::Dokumen, "Dokumen", ik::DOKUMEN)] {
        if let Some(p) = win::folder(f).filter(|p| p.is_dir()) {
            v.push(Pintasan {
                nama: nama.into(),
                folder: p.to_string_lossy().to_string(),
                glyph,
                petak: "lokal",
                ket: "Folder di komputer ini".into(),
            });
        }
    }
    for o in daftar_onedrive() {
        let nama = Path::new(&o.folder).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "OneDrive".into());
        let ket = match o.jenis.as_str() {
            "Pribadi" => "Folder OneDrive pribadi yang tersinkron di komputer ini",
            "Kantor/Sekolah" => "Folder OneDrive kantor/sekolah yang tersinkron di komputer ini",
            _ => "Folder OneDrive yang tersinkron di komputer ini",
        };
        v.push(Pintasan { nama, folder: o.folder, glyph: ik::MAP_SINKRON, petak: "sinkron", ket: ket.into() });
    }
    v
}

/// Galat rclone saat membuka folder, diringkas untuk panel: (judul, keterangan). Teks lengkap masuk log.
pub(crate) fn ringkas_galat(e: &str) -> (&'static str, String) {
    let k = e.to_lowercase();
    let jaringan = ["no such host", "dial tcp", "lookup ", "i/o timeout", "connectex", "wsarecv", "network is unreachable", "connection reset"];
    if jaringan.iter().any(|j| k.contains(j)) {
        return ("Internet sedang putus", "Folder belum bisa dimuat. Klik Coba lagi setelah internet tersambung.".into());
    }
    let baris = e.lines().map(str::trim).rfind(|l| !l.is_empty()).unwrap_or("");
    let mut t = RE_AWAL_LOG.replace(baris, "").to_string();
    if t.chars().count() > 160 {
        t = t.chars().take(157).collect::<String>() + ELIPSIS;
    }
    ("Folder tidak bisa dibuka", format!("{t} (detail di log aktivitas)"))
}

/// "Google Drive", "Dropbox", "Amazon S3"; remote impor dengan backend tak dikenal: nama backend-nya.
pub(crate) fn jenis_tampil(j: &str) -> String {
    match mp::cari(j) {
        Some(p) => p.nama.to_string(),
        None if j.is_empty() => "Akun".into(),
        None => j.to_string(),
    }
}

/// Isi folder di komputer ini (hanya subfolder, tanpa folder tersembunyi/sistem). Jalur kosong = daftar drive.
pub(crate) fn isi_lokal(p: &str) -> Isi {
    if p.trim().is_empty() {
        return Isi::Drive(win::daftar_drive());
    }
    if !adalah_lokal(p) {
        return Isi::Galat(format!("Tulis alamat lengkap, misalnya {}.", win::CONTOH_DATA));
    }
    let path = Path::new(p);
    if !path.is_dir() {
        return if path.exists() { Isi::Galat("Ini berkas, bukan folder.".into()) } else { Isi::BelumAda };
    }
    match std::fs::read_dir(path) {
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => Isi::Galat(
            if cfg!(windows) {
                "Akses ke folder ini ditolak Windows."
            } else {
                "Akses ke folder ini ditolak macOS. Izinkan di Pengaturan Sistem > Privasi & Keamanan > File dan Folder."
            }
            .into(),
        ),
        Err(e) => Isi::Galat(e.to_string()),
        Ok(it) => {
            // Tanpa folder tersembunyi/sistem ($RECYCLE.BIN di Windows; .Trash, /usr di macOS).
            let mut n: Vec<String> = it
                .flatten()
                .filter(|e| e.metadata().is_ok_and(|m| m.is_dir() && !win::tersembunyi(&m, &e.file_name())))
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect();
            n.sort_by_key(|a| a.to_lowercase());
            Isi::Folder(n)
        }
    }
}

/// Pratinjau: hasil verifikasi contoh untuk rute tersimpan (yang akan dibuka panel).
pub(crate) fn rute_contoh(cocok: i64, lain: i64) -> (i64, i64, String, String) {
    let r = rute::baca_pengaturan();
    (cocok, lain, r.sumber.unwrap_or_default(), r.tujuan.unwrap_or_default())
}

/// Ke mana berkas yang "dipindahkan" dari sumber ini pergi.
pub(crate) fn tempat_sampah(sumber: &str) -> String {
    let rem = akun::remote_dari(sumber);
    if rem.is_empty() {
        format!("{} {}", win::NAMA_SAMPAH, win::NAMA_OS)
    } else if akun::jenis_dari_nama(&rem) == "google" {
        "Sampah Google Drive (dipulihkan dalam 30 hari)".into()
    } else {
        "Recycle bin OneDrive/SharePoint".into()
    }
}

/// Jendela "Simpan sebagai". None = dibatalkan.
pub(crate) fn simpan_ekspor(nama: &str, jenis: &str, isi: &str) -> Option<Result<PathBuf, String>> {
    let ext = nama.rsplit('.').next().unwrap_or("txt").to_string();
    let p = rfd::FileDialog::new()
        .set_title(format!("Simpan {jenis}"))
        .set_file_name(nama)
        .add_filter(jenis, &[ext.as_str()])
        .set_directory(win::folder(win::Folder::Dokumen).unwrap_or_default())
        .save_file()?;
    Some(std::fs::write(&p, isi).map(|_| p.clone()).map_err(|e| format!("{} tidak bisa disimpan: {e}", p.display())))
}

/// Pesan untuk dialog riwayat setelah ekspor; laporan HTML langsung dibuka di browser.
pub(crate) fn pesan_ekspor(h: Option<Result<PathBuf, String>>, buka: bool) -> Option<(String, Color32)> {
    match h? {
        Ok(p) => {
            if buka {
                win::buka(&p.to_string_lossy());
            }
            let ket = if buka { format!(" Tekan {} di browser untuk menyimpannya sebagai PDF.", win::PINTASAN_CETAK) } else { String::new() };
            Some((format!("Tersimpan: {}.{ket}", p.display()), HIJAU))
        }
        Err(e) => Some((e, MERAH)),
    }
}
