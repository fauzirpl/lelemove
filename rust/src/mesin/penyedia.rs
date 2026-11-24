//! Daftar layanan penyimpanan yang bisa ditambahkan sebagai akun (semuanya lewat backend rclone).
//!
//! Setiap penyedia = satu entri data: backend rclone, cara login (browser OAuth atau isian),
//! kolom isiannya, nilai tetap (mis. vendor WebDAV atau provider S3), dan ke mana file pergi saat
//! "Pindahkan" menghapusnya dari sumber. Google Drive, OneDrive, dan SharePoint punya alur khusus
//! (client ID, pilih drive, pilih situs) di perintah `hubungkan`; sisanya memakai alur umum.
//!
//! Remote yang dibuat LELEMOVE bernama `<prefiks>`, `<prefiks>-2`, ... (mis. pd-dropbox-2).

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kelompok {
    /// Penyimpanan cloud pribadi/kantor (Google Drive, Dropbox, ...).
    Cloud,
    /// Server sendiri: NAS, Nextcloud, SFTP, FTP, ...
    Server,
    /// Object storage: S3, R2, B2, Azure, ...
    Objek,
}

impl Kelompok {
    pub fn judul(self) -> &'static str {
        match self {
            Kelompok::Cloud => "Layanan cloud",
            Kelompok::Server => "Server kantor & NAS",
            Kelompok::Objek => "Object storage (arsip)",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum JenisKolom {
    Teks,
    /// Kata sandi (opsi rclone ber-IsPassword): diketik tersamar, dikirim lewat stdin, diobfuskasi
    /// sebelum masuk argumen rclone, dan disimpan terobfuskasi.
    Rahasia,
    /// Kunci rahasia yang BUKAN tipe password di rclone (secret S3, key B2/Azure): diketik tersamar
    /// dan dikirim lewat stdin, tetapi rclone memakainya apa adanya (tidak boleh diobfuskasi).
    Sensitif,
    Angka,
    /// (nilai, label)
    Pilihan(&'static [(&'static str, &'static str)]),
    /// "true"/"false"
    Centang,
}

#[derive(Clone, Copy, Debug)]
pub struct Kolom {
    /// Nama opsi rclone.
    pub kunci: &'static str,
    pub label: &'static str,
    pub contoh: &'static str,
    pub jenis: JenisKolom,
    pub wajib: bool,
    pub bawaan: &'static str,
    pub ket: &'static str,
}

#[derive(Debug)]
pub struct Penyedia {
    pub kunci: &'static str,
    pub nama: &'static str,
    pub backend: &'static str,
    pub prefiks: &'static str,
    pub kelompok: Kelompok,
    /// Login lewat browser.
    pub oauth: bool,
    /// Alur khusus yang sudah ada (google, onedrive, sharepoint).
    pub khusus: bool,
    pub kolom: &'static [Kolom],
    /// Opsi rclone yang selalu dipasang (vendor WebDAV, provider S3, ...).
    pub tetap: &'static [(&'static str, &'static str)],
    /// Ke mana file yang dihapus "Pindahkan" pergi. None = layanan tanpa tempat sampah: Pindahkan ditolak.
    pub sampah: Option<&'static str>,
    /// Host API untuk menguji sambungan saat menunggu internet. None = dari konfigurasi (url/host/endpoint).
    pub host: Option<&'static str>,
    /// Warna petak: latar dan tinta (RGB).
    pub warna: [u8; 6],
    /// Inisial di petak ikon ("" = ikon awan).
    pub huruf: &'static str,
    pub ket: &'static str,
    /// Dukungan rclone untuk layanan ini masih beta, atau loginnya rumit.
    pub beta: bool,
}

const fn teks(kunci: &'static str, label: &'static str, contoh: &'static str, wajib: bool) -> Kolom {
    Kolom { kunci, label, contoh, jenis: JenisKolom::Teks, wajib, bawaan: "", ket: "" }
}

const fn rahasia(kunci: &'static str, label: &'static str, wajib: bool) -> Kolom {
    Kolom { kunci, label, contoh: "", jenis: JenisKolom::Rahasia, wajib, bawaan: "", ket: "" }
}

const fn sensitif(kunci: &'static str, label: &'static str) -> Kolom {
    Kolom { kunci, label, contoh: "", jenis: JenisKolom::Sensitif, wajib: true, bawaan: "", ket: "" }
}

impl Kolom {
    /// Diketik tersamar dan dikirim lewat stdin (bukan argumen).
    pub fn tersamar(&self) -> bool {
        matches!(self.jenis, JenisKolom::Rahasia | JenisKolom::Sensitif)
    }
}

const fn port(bawaan: &'static str) -> Kolom {
    Kolom { kunci: "port", label: "Port", contoh: bawaan, jenis: JenisKolom::Angka, wajib: false, bawaan, ket: "" }
}

const fn ket(mut k: Kolom, ket: &'static str) -> Kolom {
    k.ket = ket;
    k
}

const fn bawaan(mut k: Kolom, nilai: &'static str) -> Kolom {
    k.bawaan = nilai;
    k
}

/// Penyedia OAuth tanpa isian.
const fn oauth(
    kunci: &'static str,
    nama: &'static str,
    backend: &'static str,
    prefiks: &'static str,
    sampah: Option<&'static str>,
    host: &'static str,
    warna: [u8; 6],
    huruf: &'static str,
    ket: &'static str,
) -> Penyedia {
    Penyedia {
        kunci,
        nama,
        backend,
        prefiks,
        kelompok: Kelompok::Cloud,
        oauth: true,
        khusus: false,
        kolom: &[],
        tetap: &[],
        sampah,
        host: Some(host),
        warna,
        huruf,
        ket,
        beta: false,
    }
}

const S3_KUNCI: [Kolom; 2] = [teks("access_key_id", "Access key ID", "", true), sensitif("secret_access_key", "Secret access key")];

const WEBDAV: [Kolom; 3] = [
    ket(
        teks("url", "Alamat WebDAV", "https://cloud.contoh.go.id/remote.php/dav/files/nama/", true),
        "Nextcloud/ownCloud: buka Berkas > Pengaturan (kiri bawah) > WebDAV, salin alamatnya.",
    ),
    teks("user", "Nama pengguna", "", true),
    ket(rahasia("pass", "Kata sandi aplikasi", true), "Buat di Pengaturan > Keamanan > Kata sandi aplikasi; lebih aman daripada kata sandi login."),
];

pub static SEMUA: &[Penyedia] = &[
    // ---------------------------------------------------------------- layanan cloud
    Penyedia {
        kunci: "google",
        nama: "Google Drive",
        backend: "drive",
        prefiks: "pd-gdrive",
        kelompok: Kelompok::Cloud,
        oauth: true,
        khusus: true,
        kolom: &[],
        tetap: &[],
        sampah: Some("Sampah Google Drive (dipulihkan dalam 30 hari)"),
        host: Some("www.googleapis.com"),
        warna: [0xE4, 0xF3, 0xE8, 0x1E, 0x7E, 0x3E],
        huruf: "",
        ket: "Akun Google pribadi atau Google Workspace.",
        beta: false,
    },
    Penyedia {
        kunci: "onedrive",
        nama: "OneDrive",
        backend: "onedrive",
        prefiks: "pd-onedrive",
        kelompok: Kelompok::Cloud,
        oauth: true,
        khusus: true,
        kolom: &[],
        tetap: &[],
        sampah: Some("Recycle bin OneDrive"),
        host: Some("graph.microsoft.com"),
        warna: [0xE3, 0xEF, 0xFB, 0x0B, 0x5C, 0xAD],
        huruf: "",
        ket: "Akun Microsoft pribadi atau kantor/sekolah.",
        beta: false,
    },
    Penyedia {
        kunci: "sharepoint",
        nama: "SharePoint",
        backend: "onedrive",
        prefiks: "pd-sharepoint",
        kelompok: Kelompok::Cloud,
        oauth: true,
        khusus: true,
        kolom: &[],
        tetap: &[],
        sampah: Some("Recycle bin SharePoint"),
        host: Some("graph.microsoft.com"),
        warna: [0xDF, 0xF2, 0xF1, 0x0B, 0x6E, 0x6A],
        huruf: "",
        ket: "Pustaka dokumen situs SharePoint (Microsoft 365).",
        beta: false,
    },
    oauth(
        "dropbox",
        "Dropbox",
        "dropbox",
        "pd-dropbox",
        Some("file terhapus Dropbox (dipulihkan lewat dropbox.com)"),
        "api.dropboxapi.com",
        [0xE5, 0xEE, 0xFF, 0x00, 0x61, 0xFE],
        "Db",
        "Login di browser.",
    ),
    oauth("box", "Box", "box", "pd-box", Some("Sampah Box"), "api.box.com", [0xE6, 0xF2, 0xFB, 0x00, 0x61, 0xD5], "Bx", "Login di browser."),
    oauth(
        "pcloud",
        "pCloud",
        "pcloud",
        "pd-pcloud",
        Some("Sampah pCloud"),
        "api.pcloud.com",
        [0xE2, 0xF5, 0xF7, 0x13, 0x86, 0x9A],
        "pC",
        "Login di browser (server AS maupun Eropa).",
    ),
    oauth(
        "yandex",
        "Yandex Disk",
        "yandex",
        "pd-yandex",
        Some("Sampah Yandex Disk"),
        "cloud-api.yandex.net",
        [0xFF, 0xF1, 0xE0, 0xB4, 0x5A, 0x00],
        "Ya",
        "Login di browser.",
    ),
    oauth("hidrive", "HiDrive (STRATO)", "hidrive", "pd-hidrive", None, "api.hidrive.strato.com", [0xFF, 0xEE, 0xE4, 0xD0, 0x4A, 0x02], "Hi", "Login di browser."),
    Penyedia {
        kunci: "zoho",
        nama: "Zoho WorkDrive",
        backend: "zoho",
        prefiks: "pd-zoho",
        kelompok: Kelompok::Cloud,
        oauth: true,
        khusus: false,
        kolom: &[Kolom {
            kunci: "region",
            label: "Wilayah akun Zoho",
            contoh: "",
            jenis: JenisKolom::Pilihan(&[
                ("com", "Global (zoho.com)"),
                ("eu", "Eropa (zoho.eu)"),
                ("in", "India (zoho.in)"),
                ("com.au", "Australia (zoho.com.au)"),
                ("jp", "Jepang (zoho.jp)"),
                ("com.cn", "Tiongkok (zoho.com.cn)"),
            ]),
            wajib: true,
            bawaan: "com",
            ket: "Lihat alamat saat Anda login ke Zoho.",
        }],
        tetap: &[],
        sampah: None,
        host: Some("www.zohoapis.com"),
        warna: [0xFD, 0xEC, 0xEC, 0xC8, 0x1E, 0x2B],
        huruf: "Zo",
        ket: "Login di browser.",
        beta: true,
    },
    Penyedia {
        kunci: "mega",
        nama: "MEGA",
        backend: "mega",
        prefiks: "pd-mega",
        kelompok: Kelompok::Cloud,
        oauth: false,
        khusus: false,
        kolom: &[
            teks("user", "Email", "nama@contoh.com", true),
            rahasia("pass", "Kata sandi", true),
            ket(teks("2fa", "Kode 2FA (bila aktif)", "123456", false), "Kode 6 angka dari aplikasi autentikator saat ini."),
        ],
        tetap: &[],
        sampah: Some("Rubbish Bin MEGA"),
        host: Some("g.api.mega.co.nz"),
        warna: [0xFD, 0xE8, 0xE8, 0xD9, 0x00, 0x07],
        huruf: "Mg",
        ket: "Email dan kata sandi MEGA.",
        beta: false,
    },
    Penyedia {
        kunci: "koofr",
        nama: "Koofr",
        backend: "koofr",
        prefiks: "pd-koofr",
        kelompok: Kelompok::Cloud,
        oauth: false,
        khusus: false,
        kolom: &[
            teks("user", "Email", "nama@contoh.com", true),
            ket(rahasia("password", "Kata sandi aplikasi", true), "Buat di koofr.net > Preferences > Password > App passwords."),
        ],
        tetap: &[("provider", "koofr")],
        sampah: None,
        host: Some("app.koofr.net"),
        warna: [0xE6, 0xF4, 0xEA, 0x2E, 0x7D, 0x32],
        huruf: "Ko",
        ket: "Email dan kata sandi aplikasi Koofr.",
        beta: false,
    },
    Penyedia {
        kunci: "icloud",
        nama: "iCloud Drive",
        backend: "iclouddrive",
        prefiks: "pd-icloud",
        kelompok: Kelompok::Cloud,
        oauth: false,
        khusus: false,
        kolom: &[
            teks("apple_id", "Apple ID", "nama@icloud.com", true),
            rahasia("password", "Kata sandi Apple ID", true),
        ],
        tetap: &[("service", "drive")],
        sampah: None,
        host: Some("www.icloud.com"),
        warna: [0xEC, 0xF3, 0xFD, 0x1F, 0x6F, 0xD6],
        huruf: "iC",
        ket: "Butuh kode 2FA dari iPhone/Mac. \"Advanced Data Protection\" harus nonaktif. Login perlu diulang sekitar tiap 30 hari.",
        beta: true,
    },
    Penyedia {
        kunci: "proton",
        nama: "Proton Drive",
        backend: "protondrive",
        prefiks: "pd-proton",
        kelompok: Kelompok::Cloud,
        oauth: false,
        khusus: false,
        kolom: &[
            teks("username", "Email Proton", "nama@proton.me", true),
            rahasia("password", "Kata sandi", true),
            ket(teks("2fa", "Kode 2FA (bila aktif)", "123456", false), "Kode 6 angka dari aplikasi autentikator saat ini."),
        ],
        tetap: &[],
        sampah: Some("Sampah Proton Drive"),
        host: Some("drive-api.proton.me"),
        warna: [0xF0, 0xEB, 0xFD, 0x6D, 0x4A, 0xFF],
        huruf: "Pr",
        ket: "Dukungan rclone masih beta.",
        beta: true,
    },
    Penyedia {
        kunci: "jottacloud",
        nama: "Jottacloud",
        backend: "jottacloud",
        prefiks: "pd-jotta",
        kelompok: Kelompok::Cloud,
        oauth: false,
        khusus: false,
        kolom: &[],
        tetap: &[],
        sampah: None,
        host: Some("api.jottacloud.com"),
        warna: [0xE8, 0xF1, 0xFA, 0x1A, 0x55, 0x9E],
        huruf: "Jo",
        ket: "Memakai token login sekali pakai dari jottacloud.com (ditanyakan setelah ini).",
        beta: true,
    },
    // ---------------------------------------------------------------- server kantor & NAS
    Penyedia {
        kunci: "nextcloud",
        nama: "Nextcloud",
        backend: "webdav",
        prefiks: "pd-nextcloud",
        kelompok: Kelompok::Server,
        oauth: false,
        khusus: false,
        kolom: &WEBDAV,
        tetap: &[("vendor", "nextcloud")],
        sampah: Some("Sampah Nextcloud"),
        host: None,
        warna: [0xE3, 0xF0, 0xFB, 0x00, 0x82, 0xC9],
        huruf: "Nc",
        ket: "Server Nextcloud kantor atau pribadi.",
        beta: false,
    },
    Penyedia {
        kunci: "owncloud",
        nama: "ownCloud",
        backend: "webdav",
        prefiks: "pd-owncloud",
        kelompok: Kelompok::Server,
        oauth: false,
        khusus: false,
        kolom: &WEBDAV,
        tetap: &[("vendor", "owncloud")],
        sampah: Some("Sampah ownCloud"),
        host: None,
        warna: [0xE6, 0xEE, 0xF7, 0x04, 0x1E, 0x42],
        huruf: "oC",
        ket: "Server ownCloud.",
        beta: false,
    },
    Penyedia {
        kunci: "webdav",
        nama: "WebDAV lain",
        backend: "webdav",
        prefiks: "pd-webdav",
        kelompok: Kelompok::Server,
        oauth: false,
        khusus: false,
        kolom: &[
            teks("url", "Alamat WebDAV", "https://server.contoh.go.id/dav/", true),
            teks("user", "Nama pengguna", "", false),
            rahasia("pass", "Kata sandi", false),
        ],
        tetap: &[("vendor", "other")],
        sampah: None,
        host: None,
        warna: [0xEE, 0xEF, 0xF1, 0x3B, 0x44, 0x4F],
        huruf: "DAV",
        ket: "Synology/QNAP WebDAV, Seafile WebDAV, dan server WebDAV lain.",
        beta: false,
    },
    Penyedia {
        kunci: "smb",
        nama: "Folder bersama (SMB/NAS)",
        backend: "smb",
        prefiks: "pd-smb",
        kelompok: Kelompok::Server,
        oauth: false,
        khusus: false,
        kolom: &[
            ket(teks("host", "Alamat server", "192.168.1.10 atau nas-kantor", true), "Tanpa \\\\ di depan. Nama share dipilih sesudahnya di panel."),
            teks("user", "Nama pengguna", "", false),
            rahasia("pass", "Kata sandi", false),
            teks("domain", "Domain (bila ada)", "WORKGROUP", false),
        ],
        tetap: &[],
        sampah: None,
        host: None,
        warna: [0xEA, 0xEE, 0xF3, 0x34, 0x4E, 0x6B],
        huruf: "SMB",
        ket: "Folder bersama Windows/NAS dengan akun berbeda dari login Windows. Folder yang sudah bisa dibuka di Explorer (\\\\server\\share) bisa langsung dipakai lewat Komputer ini.",
        beta: false,
    },
    Penyedia {
        kunci: "sftp",
        nama: "SFTP / SSH",
        backend: "sftp",
        prefiks: "pd-sftp",
        kelompok: Kelompok::Server,
        oauth: false,
        khusus: false,
        kolom: &[
            teks("host", "Alamat server", "sftp.contoh.go.id", true),
            port("22"),
            teks("user", "Nama pengguna", "", true),
            rahasia("pass", "Kata sandi", false),
            ket(teks("key_file", "Berkas kunci SSH (opsional)", "C:\\Users\\nama\\.ssh\\id_ed25519", false), "Isi bila login memakai kunci, bukan kata sandi."),
        ],
        tetap: &[],
        sampah: None,
        host: None,
        warna: [0xEC, 0xEE, 0xF0, 0x2D, 0x33, 0x3A],
        huruf: "SSH",
        ket: "Server Linux, hosting, atau NAS dengan SSH.",
        beta: false,
    },
    Penyedia {
        kunci: "ftp",
        nama: "FTP / FTPS",
        backend: "ftp",
        prefiks: "pd-ftp",
        kelompok: Kelompok::Server,
        oauth: false,
        khusus: false,
        kolom: &[
            teks("host", "Alamat server", "ftp.contoh.go.id", true),
            port("21"),
            bawaan(teks("user", "Nama pengguna", "", false), "anonymous"),
            rahasia("pass", "Kata sandi", false),
            Kolom {
                kunci: "explicit_tls",
                label: "Pakai FTPS (TLS eksplisit)",
                contoh: "",
                jenis: JenisKolom::Centang,
                wajib: false,
                bawaan: "true",
                ket: "Matikan hanya bila server tidak mendukung enkripsi.",
            },
        ],
        tetap: &[],
        sampah: None,
        host: None,
        warna: [0xF2, 0xEE, 0xE6, 0x6B, 0x55, 0x2B],
        huruf: "FTP",
        ket: "Server FTP lama atau hosting web.",
        beta: false,
    },
    Penyedia {
        kunci: "seafile",
        nama: "Seafile",
        backend: "seafile",
        prefiks: "pd-seafile",
        kelompok: Kelompok::Server,
        oauth: false,
        khusus: false,
        kolom: &[
            teks("url", "Alamat server", "https://seafile.contoh.go.id", true),
            teks("user", "Email/nama pengguna", "", true),
            rahasia("pass", "Kata sandi", true),
            ket(teks("library", "Pustaka (opsional)", "Dokumen Saya", false), "Kosongkan untuk melihat semua pustaka."),
        ],
        tetap: &[],
        sampah: None,
        host: None,
        warna: [0xFE, 0xF1, 0xE2, 0xE0, 0x7A, 0x0B],
        huruf: "Sf",
        ket: "Server Seafile.",
        beta: false,
    },
    // ---------------------------------------------------------------- object storage
    Penyedia {
        kunci: "s3",
        nama: "Amazon S3",
        backend: "s3",
        prefiks: "pd-s3",
        kelompok: Kelompok::Objek,
        oauth: false,
        khusus: false,
        kolom: &[
            S3_KUNCI[0],
            S3_KUNCI[1],
            ket(teks("region", "Region", "ap-southeast-3", false), "ap-southeast-3 = Jakarta; ap-southeast-1 = Singapura."),
        ],
        tetap: &[("provider", "AWS")],
        sampah: None,
        host: Some("s3.amazonaws.com"),
        warna: [0xFF, 0xF2, 0xE0, 0xB3, 0x5C, 0x00],
        huruf: "S3",
        ket: "Bucket dipilih sesudahnya di panel (bucket/folder).",
        beta: false,
    },
    Penyedia {
        kunci: "r2",
        nama: "Cloudflare R2",
        backend: "s3",
        prefiks: "pd-r2",
        kelompok: Kelompok::Objek,
        oauth: false,
        khusus: false,
        kolom: &[
            S3_KUNCI[0],
            S3_KUNCI[1],
            ket(teks("endpoint", "Endpoint", "https://<ID-akun>.r2.cloudflarestorage.com", true), "Ada di dasbor Cloudflare > R2 > Manage R2 API Tokens."),
        ],
        tetap: &[("provider", "Cloudflare"), ("region", "auto")],
        sampah: None,
        host: None,
        warna: [0xFE, 0xF0, 0xE4, 0xD9, 0x5F, 0x0A],
        huruf: "R2",
        ket: "Tanpa biaya unduh (egress).",
        beta: false,
    },
    Penyedia {
        kunci: "wasabi",
        nama: "Wasabi",
        backend: "s3",
        prefiks: "pd-wasabi",
        kelompok: Kelompok::Objek,
        oauth: false,
        khusus: false,
        kolom: &[
            S3_KUNCI[0],
            S3_KUNCI[1],
            bawaan(
                ket(teks("endpoint", "Endpoint", "s3.ap-southeast-1.wasabisys.com", true), "Sesuai region bucket, mis. s3.ap-southeast-1.wasabisys.com (Singapura)."),
                "s3.wasabisys.com",
            ),
        ],
        tetap: &[("provider", "Wasabi")],
        sampah: None,
        host: None,
        warna: [0xE9, 0xF6, 0xEC, 0x15, 0x80, 0x3D],
        huruf: "Wa",
        ket: "Object storage murah, kompatibel S3.",
        beta: false,
    },
    Penyedia {
        kunci: "b2",
        nama: "Backblaze B2",
        backend: "b2",
        prefiks: "pd-b2",
        kelompok: Kelompok::Objek,
        oauth: false,
        khusus: false,
        kolom: &[teks("account", "Key ID", "", true), sensitif("key", "Application key")],
        tetap: &[],
        sampah: Some("versi tersembunyi di bucket B2 (dipulihkan lewat web Backblaze)"),
        host: Some("api.backblazeb2.com"),
        warna: [0xFD, 0xEA, 0xEA, 0xC8, 0x1E, 0x1E],
        huruf: "B2",
        ket: "Buat key di Backblaze > App Keys.",
        beta: false,
    },
    Penyedia {
        kunci: "azureblob",
        nama: "Azure Blob Storage",
        backend: "azureblob",
        prefiks: "pd-azure",
        kelompok: Kelompok::Objek,
        oauth: false,
        khusus: false,
        kolom: &[teks("account", "Nama storage account", "", true), sensitif("key", "Access key")],
        tetap: &[],
        sampah: None,
        host: None,
        warna: [0xE5, 0xF1, 0xFB, 0x00, 0x78, 0xD4],
        huruf: "Az",
        ket: "Container dipilih sesudahnya di panel.",
        beta: false,
    },
    Penyedia {
        kunci: "minio",
        nama: "MinIO",
        backend: "s3",
        prefiks: "pd-minio",
        kelompok: Kelompok::Objek,
        oauth: false,
        khusus: false,
        kolom: &[S3_KUNCI[0], S3_KUNCI[1], teks("endpoint", "Endpoint", "https://minio.contoh.go.id:9000", true)],
        tetap: &[("provider", "Minio")],
        sampah: None,
        host: None,
        warna: [0xFC, 0xE8, 0xEC, 0xC7, 0x2C, 0x48],
        huruf: "Mi",
        ket: "Server MinIO sendiri.",
        beta: false,
    },
    Penyedia {
        kunci: "spaces",
        nama: "DigitalOcean Spaces",
        backend: "s3",
        prefiks: "pd-spaces",
        kelompok: Kelompok::Objek,
        oauth: false,
        khusus: false,
        kolom: &[S3_KUNCI[0], S3_KUNCI[1], teks("endpoint", "Endpoint", "sgp1.digitaloceanspaces.com", true)],
        tetap: &[("provider", "DigitalOcean")],
        sampah: None,
        host: None,
        warna: [0xE5, 0xF0, 0xFF, 0x00, 0x69, 0xFF],
        huruf: "DO",
        ket: "sgp1 = Singapura.",
        beta: false,
    },
    Penyedia {
        kunci: "s3lain",
        nama: "S3 kompatibel lain",
        backend: "s3",
        prefiks: "pd-s3x",
        kelompok: Kelompok::Objek,
        oauth: false,
        khusus: false,
        kolom: &[
            S3_KUNCI[0],
            S3_KUNCI[1],
            ket(teks("endpoint", "Endpoint", "https://s3.contoh-penyedia.id", true), "Dari penyedia Anda (mis. Biznet Gio NEO Object Storage, IDCloudHost, Ceph, Synology C2)."),
            teks("region", "Region (bila diminta)", "", false),
        ],
        tetap: &[("provider", "Other")],
        sampah: None,
        host: None,
        warna: [0xEE, 0xEF, 0xF1, 0x3B, 0x44, 0x4F],
        huruf: "S3",
        ket: "Object storage lain yang mengikuti API S3.",
        beta: false,
    },
];

pub fn cari(kunci: &str) -> Option<&'static Penyedia> {
    SEMUA.iter().find(|p| p.kunci == kunci)
}

/// "pd-dropbox", "pd-dropbox-2" cocok dengan prefiks "pd-dropbox"; "pd-dropboxku" tidak.
pub fn cocok_prefiks(nama: &str, prefiks: &str) -> bool {
    match nama.strip_prefix(prefiks) {
        Some("") => true,
        Some(sisa) => sisa.strip_prefix('-').is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())),
        None => false,
    }
}

/// Penyedia dari nama remote buatan LELEMOVE.
pub fn dari_nama(remote: &str) -> Option<&'static Penyedia> {
    if remote.starts_with("pd-google") {
        return cari("google");
    }
    SEMUA.iter().find(|p| cocok_prefiks(remote, p.prefiks))
}

/// Penyedia dari isi konfigurasi rclone (untuk remote yang dipakai dari luar LELEMOVE).
pub fn dari_konfig(tipe: &str, provider: &str, vendor: &str) -> Option<&'static Penyedia> {
    let kunci = match (tipe, provider.to_ascii_lowercase().as_str(), vendor) {
        ("drive", _, _) => "google",
        ("onedrive", _, _) => "onedrive",
        ("s3", "aws", _) => "s3",
        ("s3", "cloudflare", _) => "r2",
        ("s3", "wasabi", _) => "wasabi",
        ("s3", "minio", _) => "minio",
        ("s3", "digitalocean", _) => "spaces",
        ("s3", _, _) => "s3lain",
        ("webdav", _, "nextcloud") => "nextcloud",
        ("webdav", _, "owncloud") => "owncloud",
        ("webdav", _, _) => "webdav",
        (t, _, _) => return SEMUA.iter().find(|p| p.backend == t && !p.khusus),
    };
    cari(kunci)
}

/// Port bawaan untuk menguji sambungan ke server milik sendiri.
pub fn port_bawaan(backend: &str) -> u16 {
    match backend {
        "sftp" => 22,
        "ftp" => 21,
        "smb" => 445,
        _ => 443,
    }
}

/// "https://cloud.contoh.go.id:8443/remote.php" -> ("cloud.contoh.go.id", Some(8443))
pub fn host_dari_alamat(alamat: &str) -> Option<(String, Option<u16>)> {
    let tanpa_skema = alamat.split_once("://").map_or(alamat, |(_, s)| s);
    let otoritas = tanpa_skema.split(['/', '?', '#']).next()?.rsplit('@').next()?;
    if otoritas.is_empty() {
        return None;
    }
    // [::1]:8080 tidak didukung; cukup nama host / IPv4.
    match otoritas.rsplit_once(':') {
        Some((h, p)) if p.chars().all(|c| c.is_ascii_digit()) && !p.is_empty() => Some((h.to_string(), p.parse().ok())),
        _ => Some((otoritas.to_string(), None)),
    }
}

#[cfg(test)]
mod uji {
    use super::*;

    #[test]
    fn daftar_unik_dan_lengkap() {
        for (i, a) in SEMUA.iter().enumerate() {
            for b in &SEMUA[i + 1..] {
                assert_ne!(a.kunci, b.kunci);
                // Satu prefiks tidak boleh "memakan" prefiks lain.
                assert!(!cocok_prefiks(b.prefiks, a.prefiks) && !cocok_prefiks(a.prefiks, b.prefiks), "{} / {}", a.prefiks, b.prefiks);
            }
            assert!(a.prefiks.starts_with("pd-"));
            assert!(a.oauth || a.khusus || !a.kolom.is_empty() || a.kunci == "jottacloud", "{} tanpa isian", a.kunci);
        }
        assert!(SEMUA.len() >= 25);
    }

    #[test]
    fn prefiks_dan_konfig() {
        assert_eq!(dari_nama("pd-dropbox-2").unwrap().kunci, "dropbox");
        assert_eq!(dari_nama("pd-gdrive").unwrap().kunci, "google");
        assert_eq!(dari_nama("pd-s3x").unwrap().kunci, "s3lain");
        assert_eq!(dari_nama("pd-s3").unwrap().kunci, "s3");
        assert!(dari_nama("pd-dropboxku").is_none());
        assert!(dari_nama("dropbox").is_none());
        assert_eq!(dari_konfig("s3", "Cloudflare", "").unwrap().kunci, "r2");
        assert_eq!(dari_konfig("s3", "IDrive", "").unwrap().kunci, "s3lain");
        assert_eq!(dari_konfig("webdav", "", "nextcloud").unwrap().kunci, "nextcloud");
        assert_eq!(dari_konfig("mega", "", "").unwrap().kunci, "mega");
        assert!(dari_konfig("sia", "", "").is_none());
    }

    #[test]
    fn alamat() {
        assert_eq!(host_dari_alamat("https://cloud.x.go.id/remote.php/dav/"), Some(("cloud.x.go.id".into(), None)));
        assert_eq!(host_dari_alamat("http://127.0.0.1:8081"), Some(("127.0.0.1".into(), Some(8081))));
        assert_eq!(host_dari_alamat("s3.wasabisys.com"), Some(("s3.wasabisys.com".into(), None)));
        assert_eq!(host_dari_alamat("https://u:p@h.id:9000/x"), Some(("h.id".into(), Some(9000))));
    }
}
