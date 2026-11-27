use regex::Regex;
use serde_json::{json, Map, Value};
use std::sync::LazyLock;

use super::super::akun::*;
use super::super::dasar::*;
use super::super::penyedia::{self, JenisKolom, Penyedia};
use super::super::rclone::{self, daftar_remote, panggil, Mode};
use super::Mesin;
use crate::v;

static RE_DRIVE_SISTEM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"PersonalCache|Preservation|Preservasi|Penangguhan|Cache").unwrap());
static RE_DOCUMENTS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"URL:\s*\S+/Documents/?\s*$").unwrap());
static RE_JENIS_DRIVE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\((\w+)\)\s*$").unwrap());

impl Mesin {
    // -------------------------------------------------------------- hubungkan

    /// Menghubungkan (atau menyambung ulang) satu akun. Akun baru: -Akun <nama-baru> -Jenis google|onedrive|sharepoint.
    pub(super) fn hubungkan(&mut self) -> i32 {
        if !self.wajib_rclone() {
            return 1;
        }
        let nama = nama_remote(&self.o.akun);
        if self.o.lanjut {
            return self.jawab_konfig(&nama);
        }
        let ada = daftar_remote().contains(&nama);
        let mut konf: Option<Konfig> = None;
        let mut jenis = self.o.jenis.clone();
        if ada {
            let k = konfig_akun(&nama);
            if k.tipe == "drive" {
                jenis = "google".into();
            } else if k.tipe == "onedrive" {
                jenis = if jenis_dari_nama(&nama) == "sharepoint" { "sharepoint" } else { "onedrive" }.into();
            }
            konf = Some(k);
        }
        if jenis.is_empty() {
            jenis = jenis_dari_nama(&nama).into();
        }
        if jenis.is_empty() && ada {
            jenis = jenis_konfig(&nama);
        }
        let Some(p) = penyedia::cari(&jenis) else {
            gagal(&format!("Jenis akun '{nama}' tidak dikenal. Tambahkan -Jenis <jenis>; lihat: pindah penyedia"));
            return 1;
        };
        self.catat("Akun", nama.clone());
        let mut label = self.o.label.clone().unwrap_or_default();
        if label.is_empty() {
            label = meta(&nama, "Label");
        }
        if label.is_empty() {
            label = label_bawaan(&nama, &jenis);
        }
        if !p.khusus {
            return self.hubungkan_penyedia(&nama, p, ada, &label);
        }

        let mut pesan: Vec<String> = Vec::new();
        let uji: Vec<String>;
        let paksa: bool;
        let mut arg_buat: Vec<String>;
        let mut arg_ulang: Option<Vec<String>> = None;

        if jenis == "google" {
            judul(&format!("Menghubungkan Google Drive: {label}"));
            let mut scope = konf.as_ref().map(|k| k.scope.clone()).filter(|s| !s.is_empty()).unwrap_or("drive.file".into());
            if self.o.izin_penuh {
                scope = "drive".into();
            }
            pesan.push("Browser akan terbuka. Login ke akun Google yang dimaksud, lalu klik \"Izinkan\"/\"Allow\".".into());
            pesan.push(if scope == "drive" {
                "Izin yang diminta: baca-tulis penuh (dibutuhkan untuk menyalin DARI akun ini).".into()
            } else {
                "Izin yang diminta hanya \"drive.file\": rclone hanya bisa melihat file yang ia buat sendiri.".into()
            });
            if let (Some(id), true) = (&self.o.client_id, self.o.simpan_bawaan) {
                simpan_client_bawaan(id, self.o.client_secret.as_deref().unwrap_or(""));
                ok("Client ID disimpan sebagai bawaan aplikasi.");
            }
            // -ClientAplikasi: login ulang dengan client ID aplikasi. Secret dibaca di dalam proses ini
            // (tidak lewat baris perintah), lalu koneksi dibuat ulang seperti -ClientId.
            if self.o.client_aplikasi && self.o.client_id.is_none() {
                match baca_client_bawaan() {
                    Some((i, s)) => {
                        self.o.client_id = Some(i);
                        self.o.client_secret = Some(s);
                        pesan.push("Memakai client ID aplikasi (Pengaturan > Client ID Google).".into());
                    }
                    None => {
                        gagal("Belum ada client ID aplikasi. Pasang dulu lewat Pengaturan > Client ID Google, atau: pindah klien -ClientId X -ClientSecret Y");
                        return 1;
                    }
                }
            }
            let client_lama = konf.as_ref().is_some_and(|k| k.client_sendiri);
            let (mut cid, mut csec) = (self.o.client_id.clone(), self.o.client_secret.clone());
            if cid.is_none() && !client_lama {
                if let Some((i, s)) = baca_client_bawaan() {
                    cid = Some(i);
                    csec = Some(s);
                    pesan.push("Memakai client ID bawaan aplikasi.".into());
                }
            }
            if cid.is_none() && !client_lama {
                pesan.push("Catatan: memakai client ID bersama rclone, yang sedang dibatasi Google dan bisa membuat penyalinan macet.".into());
            }
            uji = v!["about", format!("{nama}:"), "--json"];
            let ganti_izin = ada && self.o.izin_penuh && konf.as_ref().is_none_or(|k| k.scope != "drive");
            paksa = self.o.ulang || self.o.client_id.is_some() || ganti_izin;
            arg_buat = v!["config", "create", &nama, "drive", format!("scope={scope}")];
            if let Some(i) = &cid {
                arg_buat.push(format!("client_id={i}"));
                arg_buat.push(format!("client_secret={}", csec.clone().unwrap_or_default()));
            }
            // Client ID sendiri yang lama: perbarui di tempat agar secret-nya (yang tidak bisa dibaca
            // kembali dari rclone) tetap tersimpan. Selain itu: buat ulang.
            if client_lama && self.o.client_id.is_none() {
                arg_ulang = Some(v!["config", "update", &nama, format!("scope={scope}")]);
            }
            if ada && !paksa && !client_lama && cid.is_some() {
                info("Akun ini masih memakai client ID bersama. Jalankan dengan -Ulang untuk beralih ke client ID bawaan.");
            }
        } else {
            if jenis == "sharepoint" {
                judul(&format!("Menghubungkan SharePoint: {label}"));
                pesan.push("Browser akan terbuka. Login ke akun Microsoft kantor/sekolah yang punya akses ke situs SharePoint.".into());
                pesan.push(format!("Setelah itu pilih situsnya: pindah situs -Akun {nama} -Cari <kata>"));
            } else {
                judul(&format!("Menghubungkan OneDrive: {label}"));
                pesan.push("Browser akan terbuka. Login ke akun Microsoft pemilik OneDrive ini.".into());
                pesan.push("Sebagian akun kantor/instansi ditolak dengan pesan \"Need admin approval\";".into());
                pesan.push("bila begitu, pakai folder OneDrive di komputer saja.".into());
            }
            uji = v!["lsf", format!("{nama}:"), "--max-depth", "1", "--dirs-only"];
            paksa = self.o.ulang;
            arg_buat = v!["config", "create", &nama, "onedrive"];
        }

        if ada && !paksa {
            let t = panggil(&uji, Mode::Tangkap);
            if t.kode == 0 {
                ok("Sudah terhubung dan bisa diakses.");
                if self.o.label.is_some() {
                    simpan_meta_akun(&nama, &[("Label", &label)], false);
                }
                match jenis.as_str() {
                    "onedrive" => self.pilih_drive_onedrive(&nama),
                    "sharepoint" => {
                        self.pasang_pustaka_tersimpan(&nama);
                    }
                    _ => {}
                }
                return 0;
            }
            awas("Koneksi tersimpan tidak bisa dipakai lagi:");
            for b in t.galat_terakhir(3) {
                info(&b);
            }
            if !self.konfirmasi("Login ulang sekarang?") {
                return 1;
            }
        }

        for p in &pesan {
            info(p);
        }
        info("Setelah browser menampilkan \"Success!\", kembali ke jendela ini.");
        polos("");
        // stdout dibuang supaya token login tidak tercetak di layar/log.
        let h = match (&arg_ulang, ada) {
            (Some(a), true) => panggil(a, Mode::BuangStdout),
            _ => {
                if ada {
                    // Hanya menghapus entri koneksi di rclone.conf; tidak menyentuh file apa pun.
                    let _ = panggil(&v!["config", "delete", &nama], Mode::Tangkap);
                }
                panggil(&arg_buat, Mode::BuangStdout)
            }
        };
        if h.kode != 0 {
            gagal(&format!("Login tidak selesai (kode {}).", h.kode));
            return 1;
        }
        simpan_meta_akun(&nama, &[("Label", &label), ("Jenis", &jenis)], false);

        let t = panggil(&uji, Mode::Tangkap);
        if t.kode == 0 {
            ok("Terhubung.");
            match jenis.as_str() {
                "onedrive" => self.pilih_drive_onedrive(&nama),
                "sharepoint" => {
                    self.pasang_pustaka_tersimpan(&nama);
                }
                _ => {}
            }
            return 0;
        }
        gagal("Login tersimpan tetapi akses gagal:");
        for b in t.galat_terakhir(3) {
            info(&b);
        }
        1
    }

    // -------------------------------------------------------------- penyedia lain

    /// Isian rahasia dari stdin (-RahasiaStdin): objek JSON {"pass": "...", "Jawab": "..."}.
    /// Dibaca sekali; tidak pernah dicetak.
    pub(super) fn baca_rahasia(&mut self) -> Map<String, Value> {
        if !self.o.rahasia_stdin {
            return Map::new();
        }
        if self.rahasia.is_none() {
            use std::io::Read;
            let mut s = String::new();
            let _ = std::io::stdin().read_to_string(&mut s);
            self.rahasia = Some(serde_json::from_str(s.trim_start_matches('\u{feff}')).unwrap_or_default());
        }
        self.rahasia.clone().unwrap_or_default()
    }

    /// Penyedia umum (penyedia.rs): login lewat browser, atau isian kredensial lalu pertanyaan
    /// lanjutan dari rclone bila ada (mis. kode 2FA). Kata sandi diobfuskasi lewat stdin rclone
    /// sebelum masuk argumen `config create`.
    fn hubungkan_penyedia(&mut self, nama: &str, p: &'static Penyedia, ada: bool, label: &str) -> i32 {
        judul(&format!("Menghubungkan {}: {label}", p.nama));
        let isian_baru = !self.o.opsi_rclone.is_empty() || self.o.rahasia_stdin;
        if ada && !self.o.ulang && !isian_baru {
            match self.uji_akses(nama) {
                Ok(()) => {
                    ok("Sudah terhubung dan bisa diakses.");
                    if self.o.label.is_some() {
                        simpan_meta_akun(nama, &[("Label", label)], false);
                    }
                    return 0;
                }
                Err(e) => {
                    awas(&format!("Koneksi tersimpan tidak bisa dipakai: {e}"));
                    if !p.oauth {
                        info("Isi ulang datanya: Login ulang di aplikasi, atau jalankan hubungkan lagi dengan -Opsi.");
                        return 1;
                    }
                }
            }
        }
        let rahasia = self.baca_rahasia();
        let mut arg = v!["config", "create", nama, p.backend];
        for (k, v) in p.tetap {
            arg.push(format!("{k}={v}"));
        }
        for k in p.kolom {
            let dari_opsi = self.o.opsi_rclone.iter().find(|(x, _)| x == k.kunci).map(|(_, v)| v.clone());
            let mut nilai = dari_opsi.unwrap_or_else(|| teks(rahasia.get(k.kunci)));
            if !k.tersamar() {
                nilai = nilai.trim().to_string();
            }
            if nilai.is_empty() {
                nilai = k.bawaan.to_string();
            }
            if nilai.is_empty() {
                if k.wajib {
                    gagal(&format!("Isi \"{}\".", k.label));
                    return 1;
                }
                continue;
            }
            if k.jenis == JenisKolom::Rahasia {
                match rclone::obscure(&nilai) {
                    Ok(o) => nilai = o,
                    Err(e) => {
                        gagal(&e);
                        return 1;
                    }
                }
            }
            arg.push(format!("{}={nilai}", k.kunci));
        }
        // Opsi lanjutan lain dari -Opsi (mis. no_check_certificate=true untuk server uji).
        for (k, v) in &self.o.opsi_rclone {
            if !p.kolom.iter().any(|c| c.kunci == k) && !p.tetap.iter().any(|(t, _)| t == k) {
                arg.push(format!("{k}={v}"));
            }
        }
        if ada {
            // Hanya menghapus entri koneksi di rclone.conf; tidak menyentuh file apa pun.
            let _ = panggil(&v!["config", "delete", nama], Mode::Tangkap);
        }
        simpan_meta_akun(nama, &[("Label", label), ("Jenis", p.kunci)], false);
        if p.oauth {
            info("Browser akan terbuka. Login ke akun yang dimaksud, lalu izinkan akses rclone.");
            info("Setelah browser menampilkan \"Success!\", kembali ke jendela ini.");
            polos("");
            // stdout dibuang supaya token login tidak tercetak di layar/log.
            let h = panggil(&arg, Mode::BuangStdout);
            if h.kode != 0 {
                gagal(&format!("Login tidak selesai (kode {}).", h.kode));
                self.hapus_akun_gagal(nama, ada);
                return 1;
            }
            return self.selesai_hubungkan(nama, ada);
        }
        arg.extend(v!["--non-interactive", "--no-obscure"]);
        let h = panggil(&arg, Mode::Tangkap);
        self.langkah_lanjut(nama, ada, &h.out, &h.err)
    }

    /// Jawaban `config create/update --non-interactive`: selesai, galat, atau pertanyaan berikutnya.
    /// Pertanyaan dicatat sebagai "Tanya" (dibaca jendela) dan kode keluar 10.
    fn langkah_lanjut(&mut self, nama: &str, ada: bool, out: &str, err: &str) -> i32 {
        let j: Value = serde_json::from_str(out.trim()).unwrap_or(Value::Null);
        if j.is_null() {
            gagal("rclone tidak memberi jawaban konfigurasi yang dikenali.");
            for b in err.lines().map(str::trim).filter(|b| !b.is_empty() && !b.contains("not found - using defaults")).take(3) {
                info(b);
            }
            self.hapus_akun_gagal(nama, ada);
            return 1;
        }
        let e = teks(j.get("Error"));
        let state = teks(j.get("State"));
        let opsi = j.get("Option").filter(|o| !o.is_null());
        if !e.is_empty() && opsi.is_none() {
            gagal(&e);
            self.hapus_akun_gagal(nama, ada);
            return 1;
        }
        if let (false, Some(o)) = (state.is_empty(), opsi) {
            let bantuan = teks(o.get("Help"));
            awas(&format!("rclone bertanya: {}", bantuan.lines().next().unwrap_or("")));
            if !e.is_empty() {
                info(&format!("Jawaban sebelumnya ditolak: {e}"));
            }
            self.catat(
                "Tanya",
                json!({
                    "Akun": nama,
                    "State": state,
                    "Nama": teks(o.get("Name")),
                    "Bantuan": bantuan,
                    "Bawaan": teks(o.get("DefaultStr")),
                    "Rahasia": o.get("IsPassword") == Some(&Value::Bool(true)),
                    "Wajib": o.get("Required") == Some(&Value::Bool(true)),
                    "Galat": e,
                    "Pilihan": contoh_pilihan(&j).into_iter().map(|(v, h)| json!({"Nilai": v, "Ket": h})).collect::<Vec<_>>(),
                    "Baru": !ada,
                }),
            );
            info(&format!("Jawab dengan: pindah hubungkan -Akun {nama} -Lanjut -State \"{state}\" -Jawab <jawaban>"));
            return 10;
        }
        self.selesai_hubungkan(nama, ada)
    }

    /// -Lanjut: menjawab satu pertanyaan konfigurasi rclone (jawaban rahasia lewat stdin "Jawab").
    fn jawab_konfig(&mut self, nama: &str) -> i32 {
        let Some(state) = self.o.state.clone() else {
            gagal("Tambahkan -State dari pertanyaan sebelumnya.");
            return 1;
        };
        let rahasia = self.baca_rahasia();
        let mut jawab = self.o.jawab.clone().unwrap_or_else(|| teks(rahasia.get("Jawab")));
        // Jawaban kata sandi diobfuskasi dulu; rclone mengenali dan menyimpannya apa adanya.
        if rahasia.get("Rahasia") == Some(&Value::Bool(true)) && !jawab.is_empty() {
            match rclone::obscure(&jawab) {
                Ok(o) => jawab = o,
                Err(e) => {
                    gagal(&e);
                    return 1;
                }
            }
        }
        self.catat("Akun", nama);
        let baru = teks(rahasia.get("Baru")) == "true" || rahasia.get("Baru") == Some(&Value::Bool(true));
        let h = panggil(
            &v!["config", "update", nama, "--continue", "--non-interactive", "--no-obscure", "--state", state, "--result", jawab],
            Mode::Tangkap,
        );
        self.langkah_lanjut(nama, !baru, &h.out, &h.err)
    }

    /// Akun baru yang gagal tersambung dihapus lagi supaya tidak menumpuk akun rusak.
    fn hapus_akun_gagal(&mut self, nama: &str, ada: bool) {
        if !ada {
            let _ = panggil(&v!["config", "delete", nama], Mode::Tangkap);
            simpan_meta_akun(nama, &[], true);
        }
    }

    /// Uji akses ringan: daftar folder teratas.
    pub(super) fn uji_akses(&self, nama: &str) -> Result<(), String> {
        let h = panggil(
            &v!["lsf", format!("{nama}:"), "--max-depth", "1", "--dirs-only", "--contimeout", "20s", "--timeout", "40s", "--low-level-retries", "2"],
            Mode::Tangkap,
        );
        if h.kode == 0 {
            return Ok(());
        }
        Err(h.galat_terakhir(1).join(" "))
    }

    fn selesai_hubungkan(&mut self, nama: &str, ada: bool) -> i32 {
        match self.uji_akses(nama) {
            Ok(()) => {
                ok("Terhubung.");
                0
            }
            Err(e) => {
                let e_kecil = e.to_lowercase();
                // Kunci object storage yang tidak boleh melihat daftar bucket tetap sah dipakai.
                // Kunci/secret yang salah juga 403, tetapi dengan kode galat yang lain.
                let kunci_salah = ["signaturedoesnotmatch", "invalidaccesskeyid", "invalidclienttokenid", "401", "unauthorized", "authenticationfailed"]
                    .iter()
                    .any(|t| e_kecil.contains(t));
                let terbatas = !kunci_salah && ["accessdenied", "access denied"].iter().any(|t| e_kecil.contains(t));
                if terbatas {
                    awas("Tersambung, tetapi akun ini tidak boleh melihat daftar folder teratas.");
                    info("Ketik nama bucket/folder langsung di bilah alamat panel, mis. nama-bucket/arsip.");
                    return 0;
                }
                gagal(&format!("Tersimpan tetapi akses gagal: {e}"));
                self.hapus_akun_gagal(nama, ada);
                1
            }
        }
    }

    /// Daftar penyedia yang bisa ditambahkan (dan opsi isiannya).
    pub(super) fn daftar_penyedia(&mut self) -> i32 {
        judul("Penyedia yang bisa ditambahkan");
        for kel in [penyedia::Kelompok::Cloud, penyedia::Kelompok::Server, penyedia::Kelompok::Objek] {
            info("");
            ok(kel.judul());
            for p in penyedia::SEMUA.iter().filter(|p| p.kelompok == kel) {
                let login = if p.oauth {
                    "login di browser".to_string()
                } else {
                    p.kolom
                        .iter()
                        .map(|k| {
                            let r = if k.tersamar() { " (rahasia)" } else { "" };
                            format!("{}{}{r}", k.kunci, if k.wajib { "*" } else { "" })
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                let beta = if p.beta { " [beta]" } else { "" };
                info(&format!("  -Jenis {:<11} {}{beta}: {login}", p.kunci, p.nama));
            }
        }
        info("");
        info("Contoh: pindah hubungkan -Akun pd-sftp -Jenis sftp -Opsi host=nas.kantor -Opsi user=admin -RahasiaStdin");
        info("        (isian rahasia lewat stdin sebagai JSON: {\"pass\": \"...\"})");
        info("Remote rclone yang sudah ada: pindah impor [-Akun <nama>]");
        let semua: Vec<Value> = penyedia::SEMUA
            .iter()
            .map(|p| json!({"Jenis": p.kunci, "Nama": p.nama, "Backend": p.backend, "Oauth": p.oauth, "Beta": p.beta}))
            .collect();
        self.catat("Penyedia", semua);
        0
    }

    /// Memakai remote rclone yang sudah ada (dibuat di luar LELEMOVE). Tanpa -Akun: daftar calon.
    pub(super) fn impor(&mut self) -> i32 {
        if !self.wajib_rclone() {
            return 1;
        }
        let calon = remote_luar();
        // -Akun bawaan Opsi adalah "google": berarti tidak diisi.
        let nama = if self.o.akun == "google" { String::new() } else { self.o.akun.trim_end_matches(':').to_string() };
        if nama.is_empty() || !calon.iter().any(|(n, _)| *n == nama) {
            judul("Remote rclone yang bisa dipakai");
            if calon.is_empty() {
                info("Tidak ada. Buat dulu dengan: rclone config");
            }
            for (n, t) in &calon {
                info(&format!("  {n}  ({t})"));
            }
            self.catat("Calon", calon.iter().map(|(n, t)| json!({"Nama": n, "Backend": t})).collect::<Vec<_>>());
            if !nama.is_empty() {
                gagal(&format!("Remote '{nama}' tidak ada atau sudah dipakai."));
                return 1;
            }
            return 0;
        }
        let jenis = jenis_konfig(&nama);
        let label = self.o.label.clone().unwrap_or_else(|| label_bawaan(&nama, &jenis));
        simpan_meta_akun(&nama, &[("Label", &label), ("Jenis", &jenis), ("Impor", "1")], false);
        self.catat("Akun", nama.clone());
        match self.uji_akses(&nama) {
            Ok(()) => ok(&format!("Remote {nama} ({}) dipakai sebagai akun \"{label}\".", nama_layanan(&jenis))),
            Err(e) => awas(&format!("Remote {nama} dipakai, tetapi belum bisa diakses: {e}")),
        }
        0
    }

    /// Akun OneDrive bisnis punya beberapa "drive" (pustaka SharePoint), dan rclone memilih yang
    /// pertama, yang sering berupa pustaka sistem (PersonalCacheLibrary). Nama drive mengikuti
    /// bahasa akun ("Dokumen"/"Documents"), jadi yang dipakai sebagai penanda adalah alamatnya:
    /// OneDrive utama selalu beralamat .../Documents. Memakai token yang sudah tersimpan.
    fn pilih_drive_onedrive(&mut self, remote: &str) {
        let langkah = |state: &str, hasil: &str| -> Option<Value> {
            let h = panggil(
                &v!["config", "update", remote, "--continue", "--non-interactive", "--state", state, "--result", hasil],
                Mode::Tangkap,
            );
            serde_json::from_str(&h.out).ok()
        };
        let daftar: Vec<(String, String)> = langkah("choose_type_done", "onedrive")
            .map(|j| contoh_pilihan(&j))
            .unwrap_or_default()
            .into_iter()
            .filter(|(id, _)| !id.is_empty())
            .collect();
        if daftar.is_empty() {
            awas("Daftar drive OneDrive tidak terbaca; memakai pilihan bawaan rclone.");
            return;
        }
        let mut urut: Vec<(String, String)> = daftar.iter().filter(|(_, h)| !RE_DRIVE_SISTEM.is_match(h)).cloned().collect();
        urut.extend(daftar.iter().filter(|(_, h)| RE_DRIVE_SISTEM.is_match(h)).cloned());
        let mut pilihan = None;
        for d in &urut {
            let Some(j) = langkah("driveid_final", &d.0) else { continue };
            let state = teks(j.get("State"));
            if state.is_empty() {
                continue;
            }
            let help = teks(j.get("Option").and_then(|o| o.get("Help")));
            if daftar.len() == 1 || RE_DOCUMENTS.is_match(&help) {
                let _ = langkah(&state, "true");
                pilihan = Some(d.clone());
                break;
            }
        }
        let pilihan = match pilihan {
            Some(p) => p,
            None => {
                // Tidak ada yang beralamat /Documents: pakai kandidat pertama yang bukan pustaka sistem.
                let p = urut[0].clone();
                if let Some(j) = langkah("driveid_final", &p.0) {
                    let st = teks(j.get("State"));
                    if !st.is_empty() {
                        let _ = langkah(&st, "true");
                    }
                }
                awas(&format!("OneDrive utama tidak dikenali pasti; memakai drive: {}", p.1));
                p
            }
        };
        let nama = tanpa_jenis(&pilihan.1);
        let jenis = RE_JENIS_DRIVE.captures(&pilihan.1).map(|c| c[1].to_string()).unwrap_or_default();
        ok(&format!("Memakai drive OneDrive: {nama} ({jenis})"));
        simpan_meta_akun(remote, &[("Drive", &format!("{nama} ({jenis})"))], false);
        if daftar.len() > 1 {
            let lain: Vec<String> = daftar.iter().filter(|d| d.0 != pilihan.0).map(|d| d.1.clone()).collect();
            info(&format!("Drive lain di akun ini (tidak dipakai): {}", lain.join(", ")));
        }
        self.catat("DriveNama", nama);
        self.catat("DriveJenis", jenis);
    }

    /// Setelah login ulang (koneksi dibuat ulang), pasang lagi pustaka yang dulu dipilih.
    fn pasang_pustaka_tersimpan(&mut self, nama: &str) -> bool {
        let id = meta(nama, "DriveId");
        if id.is_empty() {
            info(&format!("Belum memilih situs & pustaka. Lanjutkan: pindah situs -Akun {nama} -Cari <kata>"));
            return false;
        }
        match atur_pustaka(nama, &id) {
            Ok(()) => {
                ok(&format!("Memakai pustaka: {}", meta(nama, "Drive")));
                true
            }
            Err(e) => {
                gagal(&format!("Pustaka tidak bisa dipakai. {e}"));
                false
            }
        }
    }

    // -------------------------------------------------------------- SharePoint

    pub(super) fn situs(&mut self) -> i32 {
        if !self.wajib_rclone() {
            return 1;
        }
        let nama = nama_remote(&self.o.akun);
        if !daftar_remote().contains(&nama) {
            gagal(&format!("Akun {nama} belum terhubung."));
            return 1;
        }
        let Some(kata) = self.o.cari.clone() else {
            gagal("Tambahkan kata kunci: -Cari <kata>");
            return 1;
        };
        judul(&format!("Mencari situs SharePoint: {kata}"));
        match cari_situs(&nama, &kata) {
            Ok(situs) => {
                if situs.is_empty() {
                    info("Tidak ada situs yang cocok.");
                }
                for s in &situs {
                    info(&format!("{}  {}", s.nama, s.url));
                }
                self.catat("Situs", serde_json::to_value(&situs).unwrap_or(Value::Null));
                0
            }
            Err(e) => {
                gagal(&format!("Pencarian gagal. {e}"));
                self.catat("Situs", Value::Array(Vec::new()));
                1
            }
        }
    }

    pub(super) fn pustaka(&mut self) -> i32 {
        if !self.wajib_rclone() {
            return 1;
        }
        let nama = nama_remote(&self.o.akun);
        if !daftar_remote().contains(&nama) {
            gagal(&format!("Akun {nama} belum terhubung."));
            return 1;
        }
        let situs_nama = self.o.situs_nama.clone().unwrap_or_default();
        if let Some(id) = self.o.pustaka.clone() {
            judul("Memakai pustaka SharePoint");
            if let Err(e) = atur_pustaka(&nama, &id) {
                gagal(&format!("Pustaka tidak bisa dipakai. {e}"));
                return 1;
            }
            let t = panggil(&v!["lsf", format!("{nama}:"), "--max-depth", "1"], Mode::Tangkap);
            if t.kode != 0 {
                gagal("Pustaka terpasang tetapi tidak bisa dibaca (periksa hak akses Anda di situs itu).");
                return 1;
            }
            let pustaka_nama = self.o.pustaka_nama.clone().unwrap_or_default();
            let mut ubah: Vec<(&str, String)> = vec![
                ("Jenis", "sharepoint".into()),
                ("DriveId", id.clone()),
                ("Drive", pustaka_nama.clone()),
                ("Situs", situs_nama.clone()),
            ];
            let label_lama = meta(&nama, "Label");
            if label_lama.is_empty() || label_lama.starts_with("SharePoint") {
                let l = format!("{situs_nama} - {pustaka_nama}");
                ubah.push(("Label", l.trim_matches([' ', '-']).to_string()));
            }
            let ubah_ref: Vec<(&str, &str)> = ubah.iter().map(|(k, v)| (*k, v.as_str())).collect();
            simpan_meta_akun(&nama, &ubah_ref, false);
            ok(&format!("Memakai situs \"{situs_nama}\", pustaka \"{pustaka_nama}\"."));
            return 0;
        }
        let Some(situs_id) = self.o.situs_id.clone() else {
            gagal("Tambahkan -SitusId <id> (dari perintah situs) atau -Pustaka <id>.");
            return 1;
        };
        judul(&format!("Pustaka dokumen di situs {situs_nama}"));
        match daftar_pustaka(&nama, &situs_id) {
            Ok(d) => {
                for p in &d {
                    info(&p.nama);
                }
                self.catat("Pustaka", serde_json::to_value(&d).unwrap_or(Value::Null));
                0
            }
            Err(e) => {
                gagal(&format!("Pustaka tidak bisa dibaca. {e}"));
                self.catat("Pustaka", Value::Array(Vec::new()));
                1
            }
        }
    }
}
