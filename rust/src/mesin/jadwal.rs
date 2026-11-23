//! Backup terjadwal lewat Task Scheduler Windows.
//!
//! Setiap jadwal = satu tugas Windows per pengguna ("LELEMOVE - Backup <id>", tanpa hak admin,
//! hanya berjalan saat pengguna masuk Windows) yang menjalankan `LELEMOVE.exe --jadwal <id>`.
//! LELEMOVE.exe adalah program berjendela, jadi tidak ada konsol yang muncul; ia meneruskan ke
//! `pindah.exe jadwal -Jalankan -Id <id>` secara tersembunyi, yang menyalin rute milik jadwal itu
//! (tanpa mengubah rute yang sedang dibuka di jendela), lalu memverifikasi bila diminta.
//! StartWhenAvailable: jadwal yang terlewat (komputer mati) dijalankan begitu komputer menyala.
//!
//! Daftar jadwal disimpan di %LOCALAPPDATA%\pindah-drive\jadwal.json.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::dasar::{baca_json, jalur, tulis_json, waktu_lokal};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub const HARI_PENDEK: [&str; 7] = ["Min", "Sen", "Sel", "Rab", "Kam", "Jum", "Sab"];
pub const HARI_PANJANG: [&str; 7] = ["Minggu", "Senin", "Selasa", "Rabu", "Kamis", "Jumat", "Sabtu"];
const HARI_XML: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
/// Urutan tampil: Senin dulu.
pub const URUTAN_HARI: [u8; 7] = [1, 2, 3, 4, 5, 6, 0];

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Terakhir {
    pub mulai: String,
    #[serde(default)]
    pub selesai: String,
    /// berjalan | selesai | selisih | galat | dilewati
    pub hasil: String,
    #[serde(default)]
    pub ket: String,
    #[serde(default)]
    pub log: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Jadwal {
    pub id: String,
    pub nama: String,
    pub sumber: String,
    pub tujuan: String,
    /// "22:00"
    pub jam: String,
    /// 0 = Minggu ... 6 = Sabtu; kosong = setiap hari.
    #[serde(default)]
    pub hari: Vec<u8>,
    #[serde(default = "benar")]
    pub verifikasi: bool,
    #[serde(default = "benar")]
    pub aktif: bool,
    #[serde(default)]
    pub terakhir: Option<Terakhir>,
}

fn benar() -> bool {
    true
}

fn berkas() -> PathBuf {
    jalur().data.join("jadwal.json")
}

pub fn baca_semua() -> Vec<Jadwal> {
    baca_json(&berkas()).and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default()
}

pub fn simpan_semua(j: &[Jadwal]) {
    tulis_json(&berkas(), &serde_json::to_value(j).unwrap_or(Value::Array(Vec::new())));
}

pub fn cari(id: &str) -> Option<Jadwal> {
    baca_semua().into_iter().find(|j| j.id == id)
}

/// Menyimpan (menambah atau mengganti berdasarkan id) satu jadwal.
pub fn simpan(j: &Jadwal) {
    let mut semua = baca_semua();
    match semua.iter_mut().find(|x| x.id == j.id) {
        Some(x) => *x = j.clone(),
        None => semua.push(j.clone()),
    }
    simpan_semua(&semua);
}

pub fn hapus_data(id: &str) {
    let mut semua = baca_semua();
    semua.retain(|j| j.id != id);
    simpan_semua(&semua);
}

pub fn catat_terakhir(id: &str, t: Terakhir) {
    let mut semua = baca_semua();
    if let Some(j) = semua.iter_mut().find(|j| j.id == id) {
        j.terakhir = Some(t);
        simpan_semua(&semua);
    }
}

pub fn id_baru() -> String {
    let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    format!("{:06x}", (n ^ (std::process::id() as u128) << 20) & 0xFF_FFFF)
}

/// "22:00", "7:5", "22.00" -> Some((22, 0)).
pub fn urai_jam(s: &str) -> Option<(u8, u8)> {
    let (h, m) = s.trim().split_once([':', '.'])?;
    let (h, m): (u8, u8) = (h.trim().parse().ok()?, m.trim().parse().ok()?);
    (h < 24 && m < 60).then_some((h, m))
}

/// "sen,rab,jum" / "senin, rabu" / "setiap" -> [1, 3, 5] / [] (setiap hari).
pub fn urai_hari(s: &str) -> Result<Vec<u8>, String> {
    let s = s.trim().to_lowercase();
    if s.is_empty() || s == "setiap" || s == "harian" || s == "setiap hari" {
        return Ok(Vec::new());
    }
    let mut hasil = Vec::new();
    for bag in s.split([',', ' ', ';']).filter(|b| !b.is_empty()) {
        let i = HARI_PENDEK
            .iter()
            .position(|h| bag.starts_with(&h.to_lowercase()))
            .ok_or_else(|| format!("Hari tidak dikenal: \"{bag}\". Contoh: sen,rab,jum atau setiap."))?;
        if !hasil.contains(&(i as u8)) {
            hasil.push(i as u8);
        }
    }
    hasil.sort_by_key(|h| URUTAN_HARI.iter().position(|u| u == h));
    if hasil.len() == 7 {
        hasil.clear();
    }
    Ok(hasil)
}

fn jam_tampil(jam: &str) -> String {
    urai_jam(jam).map(|(h, m)| format!("{h:02}.{m:02}")).unwrap_or_else(|| jam.to_string())
}

/// "Setiap hari pukul 22.00" / "Sen, Rab, Jum pukul 22.00"
pub fn uraian(j: &Jadwal) -> String {
    if j.hari.is_empty() {
        return format!("Setiap hari pukul {}", jam_tampil(&j.jam));
    }
    let mut hari = j.hari.clone();
    hari.sort_by_key(|h| URUTAN_HARI.iter().position(|u| u == h));
    let nama: Vec<&str> = hari.iter().map(|h| HARI_PENDEK[*h as usize % 7]).collect();
    format!("{} pukul {}", nama.join(", "), jam_tampil(&j.jam))
}

/// Berapa hari lagi jadwal berikutnya, dari hari `hari` (0 = Minggu) pukul `jam`:`menit` sekarang.
pub fn hari_lagi(j: &Jadwal, hari: u8, jam: u8, menit: u8) -> Option<u8> {
    let (h, m) = urai_jam(&j.jam)?;
    let sudah_lewat = (jam, menit) >= (h, m);
    (0..=7u8).find(|d| {
        if *d == 0 && sudah_lewat {
            return false;
        }
        j.hari.is_empty() || j.hari.contains(&((hari + d) % 7))
    })
}

/// "hari ini 22.00", "besok 22.00", "Senin 22.00"; None bila nonaktif.
pub fn berikutnya(j: &Jadwal) -> Option<String> {
    if !j.aktif {
        return None;
    }
    let (_, _, _, jam, menit, _) = waktu_lokal();
    let hari = super::dasar::hari_ini();
    let d = hari_lagi(j, hari, jam as u8, menit as u8)?;
    let kapan = match d {
        0 => "hari ini".to_string(),
        1 => "besok".to_string(),
        _ => HARI_PANJANG[((hari + d) % 7) as usize].to_string(),
    };
    Some(format!("{kapan} {}", jam_tampil(&j.jam)))
}

// ------------------------------------------------------------------ tugas Windows

pub fn nama_tugas(id: &str) -> String {
    format!("LELEMOVE - Backup {id}")
}

/// Program yang dijalankan tugas: LELEMOVE.exe (terpasang) atau pindah-drive.exe (hasil build).
pub fn exe_pemicu() -> PathBuf {
    let d = &jalur().dir_app;
    let lele = d.join("LELEMOVE.exe");
    if lele.is_file() {
        lele
    } else {
        d.join("pindah-drive.exe")
    }
}

fn escape_xml(t: &str) -> String {
    t.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Definisi tugas Task Scheduler (skema 1.2). `data_uji`: folder data pengganti (pengujian).
pub fn xml_tugas(j: &Jadwal, exe: &Path, tanggal: (u16, u16, u16), data_uji: Option<&str>) -> String {
    let (h, m) = urai_jam(&j.jam).unwrap_or((22, 0));
    let (y, mo, d) = tanggal;
    let jadwal = if j.hari.is_empty() {
        "<ScheduleByDay><DaysInterval>1</DaysInterval></ScheduleByDay>".to_string()
    } else {
        let hari: String = j.hari.iter().map(|x| format!("<{}/>", HARI_XML[*x as usize % 7])).collect();
        format!("<ScheduleByWeek><DaysOfWeek>{hari}</DaysOfWeek><WeeksInterval>1</WeeksInterval></ScheduleByWeek>")
    };
    let mut arg = format!("--jadwal {}", j.id);
    if let Some(dir) = data_uji {
        arg.push_str(&format!(" --data \"{dir}\""));
    }
    let kerja = exe.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Author>LELEMOVE</Author>
    <Description>LELEMOVE: backup terjadwal "{nama}" ({uraian}). Diatur dari LELEMOVE &gt; Jadwal backup.</Description>
  </RegistrationInfo>
  <Triggers>
    <CalendarTrigger>
      <StartBoundary>{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:00</StartBoundary>
      <Enabled>true</Enabled>
      {jadwal}
    </CalendarTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>LeastPrivilege</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>true</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <IdleSettings>
      <StopOnIdleEnd>false</StopOnIdleEnd>
      <RestartOnIdle>false</RestartOnIdle>
    </IdleSettings>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>{aktif}</Enabled>
    <Hidden>false</Hidden>
    <RunOnlyIfIdle>false</RunOnlyIfIdle>
    <WakeToRun>false</WakeToRun>
    <ExecutionTimeLimit>PT1H</ExecutionTimeLimit>
    <Priority>7</Priority>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{cmd}</Command>
      <Arguments>{arg}</Arguments>
      <WorkingDirectory>{kerja}</WorkingDirectory>
    </Exec>
  </Actions>
</Task>
"#,
        nama = escape_xml(&j.nama),
        uraian = escape_xml(&uraian(j)),
        aktif = j.aktif,
        cmd = escape_xml(&exe.to_string_lossy()),
        arg = escape_xml(&arg),
        kerja = escape_xml(&kerja),
    )
}

fn schtasks(args: &[&str]) -> Result<String, String> {
    let o = Command::new("schtasks.exe")
        .args(args)
        .stdin(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("schtasks tidak bisa dijalankan: {e}"))?;
    let teks = |b: &[u8]| String::from_utf8_lossy(b).trim().to_string();
    if o.status.success() {
        Ok(teks(&o.stdout))
    } else {
        let e = teks(&o.stderr);
        Err(if e.is_empty() { teks(&o.stdout) } else { e })
    }
}

/// Membuat atau mengganti tugas Windows untuk jadwal ini.
pub fn pasang_tugas(j: &Jadwal) -> Result<(), String> {
    let exe = exe_pemicu();
    if !exe.is_file() {
        return Err(format!("Program pemicu tidak ditemukan: {}", exe.display()));
    }
    let (y, mo, d, _, _, _) = waktu_lokal();
    let data_uji = std::env::var("PINDAH_DRIVE_DATA").ok().filter(|s| !s.is_empty());
    let xml = xml_tugas(j, &exe, (y, mo, d), data_uji.as_deref());
    // schtasks /XML membaca berkas UTF-16 LE ber-BOM (sesuai deklarasi encoding di XML).
    let mut b: Vec<u8> = vec![0xFF, 0xFE];
    for u in xml.encode_utf16() {
        b.extend_from_slice(&u.to_le_bytes());
    }
    let f = std::env::temp_dir().join(format!("lelemove-tugas-{}.xml", j.id));
    std::fs::write(&f, b).map_err(|e| format!("Berkas tugas tidak bisa ditulis: {e}"))?;
    let h = schtasks(&["/Create", "/TN", &nama_tugas(&j.id), "/XML", &f.to_string_lossy(), "/F"]);
    let _ = std::fs::remove_file(&f);
    h.map(|_| ())
}

pub fn hapus_tugas(id: &str) -> Result<(), String> {
    if !tugas_ada(id) {
        return Ok(());
    }
    schtasks(&["/Delete", "/TN", &nama_tugas(id), "/F"]).map(|_| ())
}

pub fn tugas_ada(id: &str) -> bool {
    schtasks(&["/Query", "/TN", &nama_tugas(id)]).is_ok()
}

pub fn atur_aktif_tugas(id: &str, aktif: bool) -> Result<(), String> {
    schtasks(&["/Change", "/TN", &nama_tugas(id), if aktif { "/ENABLE" } else { "/DISABLE" }]).map(|_| ())
}

/// Program yang dijalankan tugas Windows jadwal ini, dibaca dari definisi XML-nya.
pub fn exe_tugas(id: &str) -> Option<String> {
    let x = schtasks(&["/Query", "/TN", &nama_tugas(id), "/XML"]).ok()?;
    let a = x.find("<Command>")? + "<Command>".len();
    let b = x[a..].find("</Command>")? + a;
    Some(x[a..b].replace("&quot;", "\"").replace("&amp;", "&").trim().trim_matches('"').to_string())
}

/// Uninstaller: hapus tugas Windows setiap jadwal yang menjalankan program di folder `dir`
/// (tugas milik pemasangan lain dibiarkan). jadwal.json tetap ada, seperti data kerja lain;
/// bila LELEMOVE dipasang lagi, jadwal tampil "tugas Windows hilang" dan cukup diaktifkan.
pub fn hapus_tugas_di(dir: &Path) -> usize {
    let d = dir.to_string_lossy().trim_end_matches('\\').to_lowercase();
    let mut n = 0;
    for j in baca_semua() {
        let Some(exe) = exe_tugas(&j.id) else { continue };
        let e = exe.to_lowercase();
        // Keluaran schtasks memakai halaman kode konsol: jalur non-ASCII tidak bisa dibandingkan persis.
        let cocok = e.starts_with(&format!("{d}\\")) || ((!e.is_ascii() || !d.is_ascii()) && e.ends_with("\\lelemove.exe"));
        if cocok && hapus_tugas(&j.id).is_ok() {
            n += 1;
        }
    }
    n
}

#[cfg(test)]
mod uji {
    use super::*;

    fn contoh(hari: Vec<u8>) -> Jadwal {
        Jadwal {
            id: "abc123".into(),
            nama: "Arsip & Kantor".into(),
            sumber: "pd-onedrive:Kerja".into(),
            tujuan: "pd-gdrive:Arsip".into(),
            jam: "22:00".into(),
            hari,
            verifikasi: true,
            aktif: true,
            terakhir: None,
        }
    }

    #[test]
    fn jam_dan_hari() {
        assert_eq!(urai_jam("22:00"), Some((22, 0)));
        assert_eq!(urai_jam("7.5"), Some((7, 5)));
        assert_eq!(urai_jam("24:00"), None);
        assert_eq!(urai_hari("sen, rab,jum").unwrap(), vec![1, 3, 5]);
        assert_eq!(urai_hari("minggu senin").unwrap(), vec![1, 0]);
        assert!(urai_hari("setiap").unwrap().is_empty());
        assert!(urai_hari("sen,sel,rab,kam,jum,sab,min").unwrap().is_empty());
        assert!(urai_hari("libur").is_err());
    }

    #[test]
    fn uraian_dan_berikutnya() {
        assert_eq!(uraian(&contoh(vec![])), "Setiap hari pukul 22.00");
        assert_eq!(uraian(&contoh(vec![5, 1, 3])), "Sen, Rab, Jum pukul 22.00");
        let harian = contoh(vec![]);
        assert_eq!(hari_lagi(&harian, 1, 21, 0), Some(0)); // Senin 21.00 -> hari ini
        assert_eq!(hari_lagi(&harian, 1, 22, 0), Some(1)); // tepat 22.00 -> besok
        let snj = contoh(vec![1, 3, 5]);
        assert_eq!(hari_lagi(&snj, 1, 23, 0), Some(2)); // Senin malam -> Rabu
        assert_eq!(hari_lagi(&snj, 5, 23, 0), Some(3)); // Jumat malam -> Senin
        assert_eq!(hari_lagi(&snj, 6, 10, 0), Some(2)); // Sabtu -> Senin
    }

    #[test]
    fn xml() {
        let x = xml_tugas(&contoh(vec![1, 5]), Path::new("C:\\Apps\\LELEMOVE\\LELEMOVE.exe"), (2026, 11, 23), None);
        assert!(x.contains("<StartBoundary>2026-11-23T22:00:00</StartBoundary>"));
        assert!(x.contains("<DaysOfWeek><Monday/><Friday/></DaysOfWeek>"));
        assert!(x.contains("<Arguments>--jadwal abc123</Arguments>"));
        assert!(x.contains("Arsip &amp; Kantor"));
        assert!(x.contains("<StartWhenAvailable>true</StartWhenAvailable>"));
        assert!(x.contains("<WorkingDirectory>C:\\Apps\\LELEMOVE</WorkingDirectory>"));
        let u = xml_tugas(&contoh(vec![]), Path::new("C:\\x.exe"), (2026, 1, 2), Some("D:\\uji data"));
        assert!(u.contains("<ScheduleByDay><DaysInterval>1</DaysInterval></ScheduleByDay>"));
        assert!(u.contains("--data &quot;D:\\uji data&quot;"));
    }
}
