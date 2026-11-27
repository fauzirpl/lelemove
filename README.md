# LELEMOVE

*Soesah membatja edjaan lama? [Baca dalam ejaan sekarang](README-ejaan-sekarang.md). English: [README.en.md](README.en.md).*

> **MAKLOEMAT**
>
> Kepada seloeroeh pegawai kantor, pengoeroes arsip, dan rakjat pemilik berkas di seantero
> Noesantara: berkas Saoedara tidak lagi terkoeroeng di satoe tempat. Dengan LELEMOVE, ia boleh
> berpindah ke mana poen Saoedara kehendaki, dengan tertib, tjermat, dan tanpa ada jang tertjetjer.
>
> Merdeka!

Aplikasi Windows dan macOS oentoek menjalin file antar **Google Drive**, **OneDrive**, **SharePoint**,
**Dropbox**, **NAS/Nextcloud/SFTP**, **object storage S3**, lajanan lain jang didoekoeng
[rclone](https://rclone.org), dan **folder di kompoeter**, ke arah mana poen. Tjontohnja OneDrive →
Google Drive, SharePoint → NAS kantor, Dropbox → Google Drive, folder kompoeter → Backblaze B2, atau
cloud → kompoeter.

Menjalin tidak mengoebah atau menghapoes file di soember. File di soember hanja dihapoes bila Saoedara
sendiri memilih **Pindahkan** setelah verifikasi, dan itoe poen masoek ke Recycle Bin / Sampah.

**Film perkenalan** (54 detik, dengan soeara pentjerita; soembernja di [`promo/`](promo/)):

https://github.com/user-attachments/assets/e3d6832b-8cc2-4dd4-a695-b0be344245aa

![Djendela LELEMOVE: daftar lokasi dan akoen di kiri, panel Dari dan Ke, bilah penjalinan di bawah](aset/tangkapan-layar.png)

**Oendoeh:** [halaman Releases](https://github.com/fauzirpl/lelemove/releases/latest): Windows 10/11
64-bit (tanpa hak admin) dan macOS 11 ke atas (Apple Silicon dan Intel, masih beta).

**Doekoeng:** LELEMOVE pertjoema; bila terbantoe, traktirlah pengembangnja di [Saweria](https://saweria.co/pemecuteai).

*Tjatatan edjaan: naskah ini ditoelis dengan edjaan zaman perdjoeangan. Nama tombol dan menoe
ditoelis persis seperti jang tampil di aplikasi (edjaan sekarang), soepaja moedah ditjari.*

<a name="fitur-utama"></a>
## Fitoer oetama

- **29 lajanan penjimpanan** ditambah semoea lajanan lain jang didoekoeng rclone (lebih dari 70),
  dengan banjak akoen per lajanan.
- **Pengelola file doea panel** (*Dari* | *Ke*) oentoek mendjeladjah folder lokal maoepoen cloud.
- **Periksa doeloe, baroe salin**: roeang toedjoean dan oedji tjoba (dry run) diperiksa sebeloem
  penjalinan.
- **Penjalinan di latar belakang**: aplikasi boleh ditoetoep, ada notifikasi, batas ketjepatan,
  dan **landjoet otomatis** bila djaringan poetoes.
- **Filter file**: hanja djenis tertentoe, lewati pola tertentoe, atau hanja jang dioebah sedjak
  tanggal tertentoe.
- **Verifikasi** setiap file (checksum bila memoengkinkan), laloe **Pindahkan** (pilihan) jang
  menghapoes dari soember hanja file jang terboekti sama.
- **Djadwal backup** lewat Task Scheduler (Windows) atau launchd (macOS).
- **Riwajat & laporan**: CSV dan laporan siap tjetak berisi status setiap file, oentoek audit.
- **Migrasi email** (beta): pindahkan semoea folder email dari Gmail, Outlook/Microsoft 365, atau server
  IMAP lain ke server email sendiri, lengkap dengan status batja, bintang, dan tanggal asli.
- **Tjadangan email** (beta): tjadangkan semoea email ke kompoeter (.eml), laloe poelihkan ke server
  mana poen, termasoek dari berkas .mbox Google Takeout atau Thunderbird.
- **Baris perintah** (`pindah.exe`) oentoek semoea fitoer di atas.

Ditoelis dalam **Rust** (djendela memakai egui), tanpa PowerShell, djadi tetap djalan di kompoeter
kantor jang memblokir skrip. Doeloe bernama **Pindah Drive**; nama dalamnja tidak beroebah (mesin
`pindah.exe`, folder data `%LOCALAPPDATA%\pindah-drive`, nama akoen `pd-*` di rclone), djadi akoen
dan pengatoeran lama langsoeng terbatja.

## Daftar isi

- [Pasang](#pasang) · [macOS](#macos) · [Tjara memakai](#tjara-memakai) ·
  [Tjara kerdja penjalinan](#tjara-kerdja-penjalinan)
- [Lajanan jang didoekoeng](#lajanan-jang-didoekoeng) · [Filter file](#filter-file) ·
  [Riwajat & laporan](#riwajat--laporan) · [Pindahkan](#pindahkan-hapoes-soember-setelah-verifikasi) ·
  [Djadwal backup](#djadwal-backup-otomatis) · [Migrasi email](#migrasi-email) · [Tjadangan email](#tjadangan-email)
- [SharePoint](#sharepoint) · [Izin akoen](#izin-akoen) ·
  [Client ID Google](#penjalinan-matjet-pakai-client-id-google-sendiri) ·
  [Verifikasi](#verifikasi-seberapa-teliti) · [Batasan](#batasan) · [Privasi](#privasi)
- [Baris perintah](#baris-perintah) · [Oentoek pengembang](#oentoek-pengembang) · [Doekoeng](#doekoeng) ·
  [Lisensi](#lisensi) · [Riwajat versi](#riwajat-versi)

## Pasang

Oendoeh **`LELEMOVE-Setup-<versi>.exe`** dari halaman
[**Releases**](https://github.com/fauzirpl/lelemove/releases/latest), djalankan, laloe klik *Pasang*
(atau *Perbarui* bila versi lama soedah terpasang; berkas PowerShell versi 1.x dibersihkan otomatis).

- Windows 10/11 64-bit. Dipasang oentoek akoen Windows Saoedara sadja, di `%LOCALAPPDATA%\Programs\LELEMOVE`,
  tanpa hak admin.
- rclone soedah ikoet di dalamnja, djadi tidak perloe mengoendoeh apa-apa lagi.
- Moentjoel di Start Menu (dan Desktop bila ditjentang), serta di *Settings > Apps*, tempat
  aplikasinja dapat dihapoes kembali.
- Pemasangan diam-diam: `LELEMOVE-Setup-<versi>.exe /silent` (tambah `/nodesktop` bila
  tidak menghendaki pintasan Desktop).
- Memperbaroei: boeka *Bantuan & info > Tentang > Periksa pembaruan*, laloe *Unduh & pasang*.
  Installer dioendoeh dari Releases, ditjotjokkan dengan SHA-256 rilisnja, dipasang, dan LELEMOVE
  diboeka lagi. GitHub hanja dihoeboengi saat tombol itoe diklik. Tjara lain: pasang versi baroe di atasnja. Akoen, pengatoeran, djadwal, dan
  riwajat tetap terbatja, termasoek dari versi 1.x (bentoek datanja sama). Toetoep djendela LELEMOVE doeloe.
- Bila sebeloemnja terpasang dengan nama lama di `Programs\Pindah Drive`, installer memindahkannja ke
  `Programs\LELEMOVE` dan mengganti pintasan "Pindah Drive" dengan "LELEMOVE".
- Installer di Releases **tidak membawa client ID Google**. Agar penjalinan Google Drive tidak
  matjet karena dibatasi, boeatlah client ID sendiri lewat *Pengaturan > Client ID Google* (lihat
  [Penjalinan matjet? Pakai client ID Google sendiri](#penjalinan-matjet-pakai-client-id-google-sendiri)).

Installer beloem ditandatangani setjara digital, djadi Windows SmartScreen memperingatkan ketika
pertama diboeka; klik *More info* > *Run anyway*. Keaslian oendoehan dapat ditjek dengan
`Get-FileHash .\LELEMOVE-Setup-<versi>.exe -Algorithm SHA256` laloe ditjotjokkan dengan berkas
`.sha256` di halaman rilis jang sama.

<a name="macos-uji-coba"></a>
### macOS

Oendoeh **`LELEMOVE-<versi>-macOS.dmg`** dari halaman
[**Releases**](https://github.com/fauzirpl/lelemove/releases/latest).

> **Beta.** Versi macOS lolos oedji otomatis di Mac (GitHub Actions), tetapi beloem banjak dipakai di
> Mac soenggoehan. Bila menemoekan masalah, laporkan di [Issues](https://github.com/fauzirpl/lelemove/issues).

- macOS 11 Big Sur atau lebih baroe. Satoe aplikasi oentoek Apple Silicon dan Intel, rclone soedah
  di dalamnja.
- Boeka berkas .dmg, seret **LELEMOVE** ke **Applications**, laloe boeka dari Applications. Djadwal
  backup hanja dapat diboeat bila aplikasinja ada di Applications.
- Aplikasi beloem ditandatangani dengan Apple Developer ID, djadi macOS menahannja ketika pertama
  diboeka. Boeka *Pengaturan Sistem > Privasi & Keamanan*, goeloeng ke bawah, laloe klik *Buka Saja*
  oentoek LELEMOVE. Tjara lain, lewat Terminal: `xattr -dr com.apple.quarantine /Applications/LELEMOVE.app`.
- Fitoernja sama dengan versi Windows, dengan padanan macOS: file jang dipindahkan masoek ke
  **Tong Sampah** (dapat dikembalikan), djadwal backup didjalankan **launchd** (`~/Library/LaunchAgents`),
  hasil penjalinan latar dikabarkan lewat Poesat Notifikasi, Mac tidak tidoer selama menjalin, dan
  folder OneDrive di `~/Library/CloudStorage` dikenali.
- Data kerdja ada di `~/Library/Application Support/pindah-drive`, akoen rclone di `~/.config/rclone/rclone.conf`.
- Ketika pertama memboeka Desktop, Dokoemen, Oendoehan, atau disk loear, macOS meminta izin; klik *Izinkan*.
- Seperti rilis Windows, versi ini tidak membawa client ID Google: boeatlah sendiri lewat
  *Pengaturan > Client ID Google*.

<a name="cara-memakai"></a>
## Tjara memakai

Djendelanja serupa pengelola file doea panel: **daftar lokasi** di kiri, panel **Dari** (soember)
dan **Ke** (toedjoean) berdampingan, bilah alat di atas, dan bilah penjalinan di bawah.

- **Akoen.** Di daftar kiri, klik *Tambah akun* laloe pilih Google Drive, OneDrive, SharePoint,
  atau *Layanan lain* (Dropbox, NAS, S3, dan lain-lain), beri nama (misalnja "Pribadi" atau
  "Kantor"), laloe login di browser atau isi data loginnja. Satoe lajanan boleh berisi lebih dari
  satoe akoen. Ketika aplikasi diboeka, setiap akoen dioedji: titik hidjau di ikonnja berarti siap;
  **"login kedaluwarsa"** (merah) berarti perloe klik **Login ulang**; **"jaringan terputus"**
  (koening) dioedji oelang sendiri. Tombol **⋯** di tiap akoen (atau klik kanan) berisi: boeka di
  panel Dari/Ke, *Login ulang* (atau *Ubah data login…* oentoek lajanan berisian), *Beri izin penuh*
  (Google), *Pilih situs & pustaka…* (SharePoint), dan *Lepas akun ini*.
- **Pengaturan** (ikon ⚙ di dasar daftar kiri). Berisi **Client ID Google** (pasang, ganti,
  atau hapoes client ID aplikasi, dan lihat akoen Google mana jang soedah memakainja; client ID
  tidak lagi diatoer per akoen) dan **Penyalinan > Lanjutkan otomatis bila jaringan putus**.
- **Riwayat & laporan** (ikon djam di dasar daftar kiri). Semoea penjalinan, verifikasi, dan
  pemindahan, beserta laporan CSV dan siap tjetak. Lihat *Riwajat & laporan* di bawah.
- **Jadwal backup** (ikon kalender di dasar daftar kiri). Menjalin otomatis setiap hari atau
  pada hari tertentoe, tanpa perloe memboeka aplikasi. Lihat *Djadwal backup otomatis* di bawah.
- **Migrasi email** (ikon soerat di dasar daftar kiri). Lihat *Migrasi email* di bawah.
- **Cadangan email** (ikon oendoeh di dasar daftar kiri). Lihat *Tjadangan email* di bawah.
- **Dukung LELEMOVE** (ikon hati, paling atas di dasar daftar kiri) memboeka halaman Saweria.
- **Bantuan & info** (ikon ? di atas Pengaturan). Dialog bertab: *Tanya jawab*, *Ketentuan*
  pemakaian, kebidjakan *Privasi*, *Atribusi* (komponen, lajanan/API, dan semoea poestaka Rust
  beserta lisensinja), dan *Tentang* (versi, folder aplikasi, data, log, dan tautan dukungan).
- **Dari mana, ke mana.** Klik panel Dari atau Ke (garis biroe di atasnja menandai panel
  jang aktif), laloe klik lokasi di daftar kiri: *Komputer ini*, Desktop, Dokumen, folder OneDrive
  jang tersinkron, atau salah satoe akoen. Lokasi djoega dapat diganti lewat nama lokasi di kepala
  panel. Klik doea kali folder oentoek masoek, **↑** atau `..` oentoek naik, dan klik nama folder di
  bilah alamat oentoek kembali; klik roeang kosong bilah alamat oentoek mengetik alamatnja. Jang
  disalin ialah **seloeroeh isi folder jang sedang diboeka** di panel Dari, ke folder jang sedang
  diboeka di panel Ke (diboeat bila beloem ada). Tombol **⇄** di antara panel menoekar soember dan
  toedjoean; ikon map terboeka di kepala panel memboeka djendela pilih folder.
- **Salin.** Klik *Mulai salin* di bilah alat. Aplikasi otomatis memeriksa oekoeran, roeang toedjoean,
  dan mendjalankan oedji tjoba doeloe (dapat djoega lewat *Periksa*), laloe menampilkan djoemlah file
  jang akan disalin oentoek Saoedara setoedjoei. Penjalinan berdjalan di latar belakang, djadi
  aplikasi boleh ditoetoep; notifikasi moentjoel bila penjalinan selesai atau berhenti karena galat.
  Batas ketjepatan di kanan bilah alat (Tanpa batas, 1, 5, atau 10 MB/dtk) mendjaga internet tetap
  lantjar oentoek keperloean lain; pilihannja diingat dan berlakoe ketika penjalinan dimoelai atau
  dilandjoetkan.
- **Filter.** Di bawah panel Dari, klik *Filter…* oentoek menjalin hanja djenis file tertentoe,
  melewati pola tertentoe, atau hanja file jang dioebah sedjak tanggal tertentoe. Lihat *Filter file*.
- **Verifikasi.** Berdjalan otomatis setelah penjalinan selesai; dapat djoega diklik di bilah alat.
- **Pindahkan (pilihan).** Setelah verifikasi, tautan *Pindahkan: hapus … file ini dari sumber*
  menghapoes dari soember file jang terboekti sama di toedjoean. Lihat *Pindahkan*.

*Hentikan* menghentikan penjalinan. *Lanjutkan* meneroeskannja, dan file jang soedah sampai dilewati.
*Log aktivitas* di bilah status bawah memboeka tjatatan lengkap setiap langkah.

<a name="cara-kerja-penyalinan"></a>
## Tjara kerdja penjalinan

Antar akoen jang berlainan (misalnja OneDrive → Google Drive), data **dialirkan lewat kompoeter ini**:
rclone mengoendoeh dan mengoenggah sekaligoes dalam potongan di memori, tanpa menjimpan berkas oetoeh
ke disk. Djadi koeota internet terpakai doea kali (oendoeh + oenggah), dan kompoeter haroes tetap
menjala (aplikasi mentjegah kompoeter tidoer selama menjalin). Penjalinan memakai `rclone copy`,
sehingga hanja file baroe atau jang beroebah jang dikirim; penjalinan oelang ke toedjoean jang sama
berdjalan tjepat.

<a name="jaringan-putus-lanjut-otomatis"></a>
### Djaringan poetoes: landjoet otomatis

rclone sendiri mentjoba oelang gangguan singkat. Bila poetoesnja lebih lama sampai rclone berhenti,
proses penjalinan latar LELEMOVE memoetoeskan dari **galat terachir** di log pertjobaan itoe:

| Galat terachir | Tindakan |
|---|---|
| Djaringan (DNS menolak, `no such host`, `dial tcp`, `i/o timeout`, sambungan poetoes, ...) | Toenggoe, laloe landjoet otomatis |
| Login kedaloewarsa (`invalid_grant`) | Berhenti: perloe *Login ulang* |
| Batas oenggah harian Google (750 GB) | Berhenti: landjoetkan esok hari |
| Galat lain (mis. izin ditolak) | Berhenti dan kabari lewat notifikasi |

Selama menoenggoe, djeda bertambah: 30 detik, 1, 2, 5, laloe 10 menit. Setelah djeda, LELEMOVE
menguji sambungan (DNS + TCP) tiap 15 detik ke server lajanan jang dipakai, dan baroe mendjalankan
rclone lagi setelah tersamboeng. Oentoek lajanan cloud, jang dioedji server API-nja (mis.
`www.googleapis.com`, `graph.microsoft.com`, `api.dropboxapi.com`); oentoek server sendiri (NAS,
SFTP, Nextcloud, S3 lain), host dan port dari konfigurasi akoennja. File jang soedah sampai
dilewati. Batasnja 25 pertjobaan, atau 6 djam tanpa internet dalam satoe djeda. Selama menoenggoe,
bilah penjalinan menampilkan **"Menunggu jaringan"** (koening) beserta perkiraan djam pertjobaan
berikoetnja; *Hentikan* tetap dapat dipakai. Notifikasi moentjoel sekali ketika djaringan pertama
kali poetoes. Di log tertjatat baris `LELEMOVE: jaringan terputus ...` dan `... penyalinan
dilanjutkan`. Matikan lewat *Pengaturan > Penyalinan* (`"LanjutOtomatis": false` di
`pengaturan.json`); penjalinan dari konsol tanpa `-Latar` tidak pernah melandjoetkan sendiri.

<a name="layanan-yang-didukung"></a>
## Lajanan jang didoekoeng

*Tambah akun > Layanan lain* menampilkan semoea lajanan di bawah ini (dapat ditjari). Kolom
*Pindahkan* menjeboet ke mana file pergi ketika dihapoes dari soember; "–" berarti lajanan tanpa
tempat sampah, sehingga Pindahkan ditolak di sana.

| Kelompok | Lajanan | Login | Pindahkan |
|---|---|---|---|
| Cloud | Google Drive, OneDrive, SharePoint | Browser | Sampah / recycle bin |
| Cloud | Dropbox, Box, pCloud, Yandex Disk | Browser | File terhapoes / Sampah |
| Cloud | HiDrive, Zoho WorkDrive (beta) | Browser | – |
| Cloud | MEGA | Email + kata sandi (+ kode 2FA) | Rubbish Bin |
| Cloud | Koofr | Email + kata sandi aplikasi | – |
| Cloud | iCloud Drive (beta) | Apple ID + kata sandi, laloe kode 2FA | – |
| Cloud | Proton Drive (beta) | Email + kata sandi (+ kode 2FA) | Sampah |
| Cloud | Jottacloud (beta) | Token login dari sitoesnja (ditanjakan) | – |
| Server & NAS | Nextcloud, ownCloud | Alamat WebDAV + kata sandi aplikasi | Sampah |
| Server & NAS | WebDAV lain, SMB/NAS, SFTP, FTP/FTPS, Seafile | Alamat + penggoena + kata sandi | – |
| Object storage | Amazon S3, Cloudflare R2, Wasabi, MinIO, DigitalOcean Spaces, S3 lain | Access key + secret (+ endpoint) | – |
| Object storage | Backblaze B2 | Key ID + application key | Versi tersemboenji B2 |
| Object storage | Azure Blob Storage | Nama storage account + access key | – |

- **Lajanan lain dari rclone** (lebih dari 70): *Buat lewat rclone config* memboeka djendela
  `rclone config`. Sesoedahnja, *Pakai remote rclone yang sudah ada* memasoekkannja ke LELEMOVE.
  Remote seperti itoe tetap milik Saoedara: *Lepas* hanja berhenti memakainja, tidak menghapoesnja.
- **Folder bersama jang soedah dapat diboeka di Explorer** (`\\server\share`) dapat langsoeng dipakai
  lewat *Komputer ini*. Pilih *SMB/NAS* hanja bila perloe login dengan akoen berlainan.
- **Isian rahasia** (kata sandi, secret) dikirim dari djendela ke `pindah.exe` lewat stdin, laloe
  diobfoeskasi lewat `rclone obscure -` sebeloem masoek `rclone config create`. Djadi kata sandi tidak
  pernah tampak polos di argoemen proses. Obfoeskasi boekan enkripsi; pakai kata sandi aplikasi bila ada.
- **Pertanjaan landjoetan** (kode 2FA iCloud, token Jottacloud, pilihan) datang dari mesin
  konfigurasi rclone (`config create --non-interactive` → `config update --continue`) dan tampil
  sebagai dialog. Akoen baroe jang gagal tersamboeng dihapoes kembali.
- Lajanan tanpa keterangan koeota (FTP, S3, B2, ...) tetap dapat diperiksa: koeotanja ditoelis "tidak dilaporkan".
- Ketika djaringan poetoes, server sendiri (NAS, SFTP, Nextcloud, S3 lain) dioedji ke host dan port
  dari konfigurasi akoennja; lajanan cloud ke server API-nja.

## Filter file

| Isian | Tjontoh | Arti |
|---|---|---|
| Hanya jenis file ini | `pdf, docx, xlsx` | Hanja ekstensi ini jang disalin (hoeroef besar-ketjil sama sadja). Kosong = semoea. |
| Lewati | `*.tmp, Arsip Lama/, node_modules/` | Pola jang tidak disalin. Achiran `/` = folder beserta isinja; `*` = teks apa sadja. |
| Hanya yang diubah sejak | `31-01-2026` | Hanja file jang tanggal oebahnja pada/sesoedah tanggal itoe. |

Filter disimpan di `pengaturan.json` (`Filter`) dan dipakai sama persis oleh periksa, oedji tjoba,
salin, verifikasi, dan pindahkan; file jang sengadja tidak disalin tidak dilaporkan "beloem ada di
toedjoean". Ketika filter djenis atau tanggal aktif, folder kosong di soember tidak ikoet diboeat di
toedjoean (rclone akan memboeat *semoea* soebfolder walau isinja tidak ada jang lolos). File sistem
(`desktop.ini`, `Thumbs.db`, berkas koentji Office `~$*`, dan di macOS `.DS_Store` serta `._*`)
selaloe dilewati. Djadwal backup menjimpan salinan filternja sendiri (*Pakai folder & filter yang
sedang dibuka*).

<a name="riwayat--laporan"></a>
## Riwajat & laporan

Setiap penjalinan (termasoek jang dihentikan, gagal, atau dilewati djadwal), verifikasi, pemindahan,
dan migrasi email ditjatat sebagai satoe baris di `%LOCALAPPDATA%\pindah-drive\riwayat.jsonl`.
Riwajat tidak dihapoes otomatis.

- **Laporan per file** (pada verifikasi/pindahkan): halaman HTML siap tjetak berisi roete, waktoe,
  metode (checksum atau nama & oekoeran), filter, ringkasan, dan status **setiap file**; jang
  bermasalah ditampilkan lebih doeloe. Simpan sebagai PDF lewat *Ctrl+P* (macOS: *Cmd+P*) di browser.
  Tjotjok sebagai boekti audit bahwa salinan lengkap.
- **CSV per file**: daftar jang sama oentoek Excel (UTF-8, pemisah koma).
- **Ekspor CSV / Laporan HTML**: seloeroeh riwajat jang tampil (dapat disaring: Salin, Verifikasi,
  Pindahkan).

Daftar per file dibatja dari berkas `verifikasi-*.txt` di folder log, jang ikoet terhapoes setelah
60 hari; ringkasannja tetap ada di riwajat.

<a name="pindahkan-hapus-sumber-setelah-verifikasi"></a>
## Pindahkan (hapoes soember setelah verifikasi)

Pilihan, dan selaloe atas permintaan Saoedara. Moentjoel sebagai tautan di bawah hasil verifikasi
bila ada file jang tjotjok, dengan doea kali konfirmasi (jang kedoea: ketik `HAPUS`).

1. LELEMOVE **memverifikasi oelang** saat itoe djoega (hasil verifikasi sebeloemnja bisa basi).
2. Hanja file berstatoes *tjotjok persis* jang dihapoes dari **soember**. File jang beloem ada,
   berbeda, gagal dibatja, atau file Office jang metadatanja dioebah SharePoint tidak disentoeh.
3. Ke mana perginja: folder kompoeter → **Recycle Bin** (Windows) atau **Tong Sampah** (macOS);
   Google Drive → **Sampah** (30 hari); OneDrive/SharePoint → **recycle bin**; Dropbox, Box,
   pCloud, Yandex, MEGA, Proton, Nextcloud, ownCloud → tempat sampah masing-masing; Backblaze B2 →
   versi tersemboenji (lihat kolom *Pindahkan* di [Lajanan jang didoekoeng](#lajanan-jang-didoekoeng)).
   Folder jang mendjadi kosong ikoet dihapoes; folder jang memang kosong sedjak awal dibiarkan.
4. Ditolak bila file jang dihapoes tidak dapat dipoelihkan: folder di drive djaringan atau
   flashdisk tanpa Recycle Bin, dan lajanan tanpa tempat sampah (SMB/NAS, SFTP, FTP, WebDAV lain,
   S3, R2, Azure, dan sedjenisnja).

Toedjoean tidak pernah dihapoes apa poen. Djadwal backup tidak dapat memindahkan (selaloe menjalin).

<a name="jadwal-backup-otomatis"></a>
## Djadwal backup otomatis

Boeka folder jang hendak ditjadangkan di panel **Dari** dan toedjoeannja di panel **Ke**, laloe klik
**Jadwal backup > Jadwal baru**. Isi nama, djam (kelipatan 5 menit), hari (semoea hari = setiap
hari), dan apakah hasilnja diverifikasi. Roete djadwal disimpan terpisah, djadi panel Dari/Ke boleh
diganti sesoedahnja tanpa mengoebah djadwal. Di daftar djadwal ada *Jalankan sekarang*, *Ubah*,
*Nonaktifkan*/*Aktifkan*, dan *Hapus*, beserta hasil terachir dan tautan ke log-nja.

Tjara kerdjanja:

| Bagian | Keterangan |
|---|---|
| Pemitjoe (Windows) | Satoe toegas **Task Scheduler** per djadwal, bernama `LELEMOVE - Backup <id>`, milik penggoena Windows jang sedang masoek (tanpa hak admin, "Interactive only"). |
| Pemitjoe (macOS) | Satoe LaunchAgent per djadwal di `~/Library/LaunchAgents` (`io.github.fauzirpl.lelemove.backup.<id>`), didjalankan **launchd**. |
| Jang didjalankan | `LELEMOVE.exe --jadwal <id>` → `pindah.exe jadwal -Jalankan -Id <id>` tersemboenji. |
| Kompoeter mati/tidoer pada djamnja | Windows: didjalankan begitoe kompoeter menjala lagi (*StartWhenAvailable*). macOS: didjalankan begitoe Mac bangoen; bila Mac mati, djadwal itoe dilewati. Kompoeter tidak dibangoenkan. |
| Penjalinan lain sedang berdjalan | Djadwal ini dilewati (tertjatat "dilewati" + notifikasi), ditjoba lagi pada djadwal berikoetnja. |
| Djaringan poetoes | Sama dengan penjalinan latar: landjoet otomatis (bila aktif di Pengaturan). |
| Hasil | Notifikasi: selesai (djoemlah file baroe, hasil verifikasi), ada selisih, atau gagal. Bila aplikasi terboeka, bilah bawah menampilkan "Backup terjadwal ...". |
| Penjimpanan | `%LOCALAPPDATA%\pindah-drive\jadwal.json` (daftar + hasil terachir); log di folder `log\jadwal-*.log`. |

Jang disalin hanja file baroe atau jang beroebah; tidak ada jang dihapoes di toedjoean. Uninstall
menghapoes toegas Windows milik folder aplikasi itoe, tetapi `jadwal.json` tetap ada: bila LELEMOVE
dipasang lagi, djadwal tampil "tugas Windows hilang" dan tjoekoep diklik *Aktifkan*. Djadwal hanja
berdjalan selama Saoedara masoek ke kompoeter (lajar terkoentji tidak mengapa).

## Migrasi email

Memindahkan semoea email dari **Gmail**, **Outlook/Microsoft 365**, atau server IMAP lain ke **server
email sendiri** (cPanel, Plesk, Zimbra, Dovecot, server email dinas). Boeka *Migrasi email* di dasar
daftar kiri:

> **Beta.** Migrasi email soedah dioedji dengan server IMAP oedji, tetapi login Gmail dan Outlook
> soenggoehan beloem banjak dioedji. Djalankan **Periksa** doeloe, dan laporkan masalah di
> [Issues](https://github.com/fauzirpl/lelemove/issues).

1. **Dari**: pilih lajanan laloe isi alamat email lama.
   - Gmail: pakai **sandi aplikasi** ([myaccount.google.com/apppasswords](https://myaccount.google.com/apppasswords);
     akoen haroes memakai verifikasi 2 langkah), atau **Login Google di browser** (memakai client ID
     Google di *Pengaturan*; akoennja haroes terdaftar sebagai penggoena oedji di project Google Cloud itoe).
   - Outlook/Microsoft 365: login Microsoft di browser; boetoeh client ID Azure (lihat di bawah).
   - Server IMAP lain: server, port, keamanan, penggoena, dan kata sandi.
2. **Ke**: server IMAP baroe (mis. `mail.kantor.go.id`, port 993, SSL/TLS), nama penggoena (biasanja
   alamat email lengkap), dan kata sandi.
3. **Periksa** mengoedji login kedoea server dan menghitoeng email jang akan disalin; **Mulai migrasi**
   menjalinnja. Progres per folder tampil di dialog, dan hasilnja tertjatat di *Riwayat & laporan*.

Jang terdjadi ketika migrasi:

- Semoea folder ikoet, termasoek soebfolder, statoes dibatja/beloem, bintang, soedah dibalas, dan tanggal
  terima aslinja. Folder choesoes masoek ke padanannja di server baroe (Sent Mail / Sent Items → Sent,
  Deleted Items → Trash, Drafts → Drafts), termasoek server bergaja cPanel jang foldernja berawalan `INBOX.`.
- Email di akoen lama **tidak dioebah, tidak ditandai dibatja, dan tidak dihapoes** (diboeka batja-sadja).
- Email jang soedah ada di toedjoean (Message-ID sama) dilewati. Djadi migrasi **aman didjalankan oelang**:
  bila terpoetoes atau dihentikan, djalankan lagi dan hanja sisanja jang disalin.
- Gmail: label mendjadi folder. Email arsip tanpa label (hanja ada di *Semua Email*) disalin ke folder
  Archive; *Berbintang* dan *Penting* tidak disalin terpisah karena tandanja ikoet di setiap email.
  Gmail membatasi oendoehan IMAP kira-kira 2,5 GB per hari; kotak soerat jang lebih besar dilandjoetkan esoknja.
- Sampah dan Spam tidak disalin ketjoeali ditjentang. Folder Outlook jang boekan email (Kalender, Kontak,
  Tugas, Catatan) dilewati.
- Kata sandi hanja dikirim ke server email masing-masing dan **tidak disimpan**. Login Google/Microsoft
  memakai token jang hanja ada di memori selama migrasi berdjalan.
- Server email di djaringan kantor jang memakai sertifikat boeatan sendiri: tjentang
  *Terima sertifikat swatanda* (biarkan mati oentoek server di internet).

Lewat baris perintah (kata sandi lewat stdin, atau diketik tanpa tampak):

```powershell
.\pindah.exe email -Sumber gmail:nama@gmail.com -Tujuan imap://nama@kantor.go.id@mail.kantor.go.id -HanyaPeriksa
.\pindah.exe email -Sumber outlook:nama@outlook.com -Tujuan imap://nama@mail.server.id -Ya
.\pindah.exe email -Sumber gmail:nama@gmail.com -Tujuan "D:\Cadangan Email" -Ya            # cadangkan ke komputer (.eml)
.\pindah.exe email -Sumber "D:\Cadangan Email" -Tujuan imap://nama@mail.server.id -Ya      # pulihkan dari cadangan
.\pindah.exe email -Sumber "D:\Takeout\Mail\All mail Including Spam and Trash.mbox" -Tujuan imap://nama@mail.server.id -Ya
```

<a name="outlook-butuh-client-id-azure"></a>
### Outlook: boetoeh client ID Azure

Microsoft tidak lagi menerima kata sandi oentoek IMAP, djadi LELEMOVE login lewat aplikasi Azure milik
Saoedara sendiri (pertjoema, tjoekoep diboeat sekali):

1. Boeka [portal.azure.com](https://portal.azure.com) > *Microsoft Entra ID* > *App registrations* > *New registration*.
2. Name: `LELEMOVE`. Supported account types: *Accounts in any organizational directory and personal
   Microsoft accounts* (atau hanja organisasi Saoedara, oentoek Microsoft 365 kantor).
3. Redirect URI: platform *Public client/native (mobile & desktop)*, isi `http://localhost`, laloe *Register*.
4. *API permissions* > *Add a permission* > *Microsoft Graph* > *Delegated permissions*: tjentang
   `IMAP.AccessAsUser.All` dan `offline_access`. Di Microsoft 365 kantor, admin moengkin perloe memberi
   persetoedjoean (*Grant admin consent*).
5. Salin *Application (client) ID* ke isian *Client ID* di dialog Migrasi email. ID ini disimpan oentoek
   migrasi berikoetnja (boekan rahasia).

Pastikan IMAP aktif di akoen Outlook.com: *Pengaturan > Email > Teruskan dan IMAP*.

> **Akoen kantor (Microsoft 365).** Sebagian organisasi mewadjibkan *persetoedjoean admin* oentoek
> tiap aplikasi loear. Bila login memoentjoelkan "Need admin approval", client ID sendiri poen tidak
> menemboesnja (ini dinding dari IT kantor, boekan LELEMOVE; Thunderbird poen kena hal jang sama).
> Djalan keloearnja: minta IT menjetoedjoei, atau ambil email lewat **Thunderbird** (login Microsoft
> biasa, laloe *Sinkronkan*) dan poelihkan folder mbox-nja lewat [Tjadangan email](#tjadangan-email).

<a name="tjadangan-di-kompoeter-dan-pemoelihan"></a>
## Tjadangan email

Simpan semoea email ke **folder di kompoeter**, laloe poelihkan ke server mana poen kapan sadja.
Tjotjok oentoek akoen jang hendak ditoetoep, atau bila server baroe beloem siap. Boeka
*Cadangan email* di dasar daftar kiri; di atasnja ada doea pilihan, *Cadangkan ke komputer* dan
*Pulihkan ke server*. Akoen soember diisi sama seperti pada [Migrasi email](#migrasi-email).

> **Beta.** Soedah dioedji dengan server IMAP oedji (tjadangan, djalan oelang, pemoelihan ke akoen
> kosong jang hasilnja identik), tetapi beloem banjak dengan akoen Gmail dan Outlook soenggoehan.

- **Tjadangkan ke komputer**: pilih akoen email dan folder toedjoean. Setiap email mendjadi satoe
  berkas **.eml** (dapat diboeka langsoeng di Outlook, Thunderbird, atau Windows Mail) di dalam
  folder jang sama dengan kotak soeratnja (INBOX, Sent, Kerja/Proyek, ...). Nama berkas diawali
  tanggal, mis. `2024-01-05 1030 Rapat besok [1a2b3c4d].eml`, dan tanggal berkasnja disamakan dengan
  tanggal terima email. Statoes batja, bintang, dan tanggal asli ditjatat di `.lelemove-indeks.jsonl`
  di setiap folder.
- **Perbaroei tjadangan**: djalankan lagi kapan sadja; hanja email baroe jang dioendoeh. Email jang
  dihapoes di server tidak ikoet dihapoes dari tjadangan.
- **Pulihkan ke server**: pilih folder tjadangan tadi dan server toedjoean. Folder, statoes batja,
  bintang, dan tanggal asli kembali seperti semoela; email jang soedah ada di server dilewati.
- **Dari Google Takeout**: oendoeh email Gmail sebagai berkas **.mbox** lewat
  [takeout.google.com](https://takeout.google.com), laloe pilih berkas itoe (tombol *.mbox…*) di
  *Pulihkan ke server*. Djalan ini tidak terkena batas oendoehan IMAP Gmail (kira-kira 2,5 GB per
  hari). Label Gmail mendjadi folder, email tanpa label masoek ke Archive, dan obrolan Chat dilewati.
- **Dari Thunderbird**: pilih folder profil Thunderbird jang berisi berkas mbox (mis.
  `Mail\Local Folders`); setiap berkas mendjadi satoe folder.
- Tjadangan .eml berisi seloeroeh email apa adanja, tidak dienkripsi: simpanlah di tempat jang aman.

## SharePoint
Klik **Tambah akun > SharePoint**, login dengan akoen Microsoft kantor/sekolah, laloe tjari sitoesnja.
Saoedara dapat mengetik namanja (misalnja "keuangan") atau menempel alamatnja dari browser
(`https://namatenant.sharepoint.com/sites/NamaSitus`). Pilih sitoes dan **poestaka dokoemennja**
(biasanja "Dokumen"). Satoe akoen SharePoint mewakili satoe poestaka; oentoek poestaka lain, tambah
akoen SharePoint lagi atau ganti lewat *⋯ > Pilih situs & pustaka…* pada akoen itoe.

- Jang tersalin hanja file dan folder. Riwajat versi, izin berbagi, metadata kolom, list,
  dan halaman sitoes tidak ikoet.
- Oentoek menjalin dari SharePoint tjoekoep hak *batja*; oentoek menjalin ke SharePoint perloe hak *edit*.
- Bila SharePoint mendjadi toedjoean, SharePoint menambahkan metadata ke file Office
  (.docx/.xlsx/.pptx), sehingga oekoerannja beroebah. LELEMOVE memakai `--ignore-size
  --ignore-checksum` ketika menjalin ke SharePoint, dan ketika verifikasi file Office sematjam itoe
  dihitoeng tjotjok, disertai tjatatan.
- SharePoint kerap membatasi ladjoe (throttling, HTTP 429). rclone menoenggoe laloe mentjoba lagi.
- Tjara tanpa login tambahan: di SharePoint klik **Sync** atau **Add shortcut to My files**,
  laloe pilih folder hasilnja lewat *Komputer ini*.

<a name="izin-akun"></a>
## Izin akoen

| Akoen | Sebagai toedjoean | Sebagai soember |
|---|---|---|
| Google Drive, izin terbatas (`drive.file`) | Ja | Tidak: hanja dapat melihat file boeatan LELEMOVE |
| Google Drive, izin penoeh (`drive`) | Ja | Ja |
| OneDrive pribadi / kantor | Ja | Ja |
| SharePoint (satoe poestaka dokoemen) | Ja, bila poenja hak edit | Ja |
| Lajanan lain (Dropbox, NAS, S3, ...) | Ja, bila akoen/koentjinja boleh menoelis | Ja, bila boleh membatja |
| Komputer ini | Ja | Ja |

Izin terbatas ialah pilihan bawaan dan paling aman oentoek akoen jang hanja mendjadi toedjoean.
Oentoek menjalin *dari* Google Drive, berilah izin penoeh lewat *⋯ > Beri izin penuh* pada akoen itoe.

Akoen OneDrive kantor sebenarnja mempoenjai beberapa "drive" (poestaka SharePoint), seperti
`PersonalCacheLibrary` dan `Pustaka Penangguhan Preservasi`. LELEMOVE otomatis memilih
OneDrive oetama, jaitoe drive jang alamatnja berachir `/Documents` ("Dokumen" atau "Documents"
tergantoeng bahasa), laloe menampilkan nama drive itoe di daftar akoen.

<a name="penyalinan-macet-pakai-client-id-google-sendiri"></a>
## Penjalinan matjet? Pakai client ID Google sendiri

Gedjalanja: progres berhenti di angka tertentoe dengan ketjepatan 0, sementara
aplikasi menampilkan "Tidak ada kemajuan sejak ..." atau "membatasi permintaan (rate limit)".

Sebabnja: rclone memakai satoe **client ID Google bersama** oentoek semoea pemakainja di seloeroeh
doenia. Client ID itoe sedang dipensioenkan Google dan koeota per menitnja kerap habis
(`Error 403: Quota exceeded ... rateLimitExceeded`). Selama koeota habis, rclone hanja dapat
menoenggoe dan mentjoba lagi.

Djalan keloearnja ialah client ID sendiri. Pertjoema, koeotanja choesoes oentoek Saoedara, dan
tjoekoep diboeat sekali (kira-kira 5 menit). Boeka **Pengaturan > Client ID Google > Pasang client ID…**;
dialognja berisi langkah-langkah beserta tautan ke Google Cloud Console:

1. Boeat project baroe di Google Cloud.
2. Aktifkan Google Drive API.
3. Isi *Branding* (nama aplikasi dan email Saoedara), pilih Audience *External*.
4. Di *Clients*, boeat client djenis *Desktop app*, laloe tempel Client ID dan secret-nja ke aplikasi.
   Google hanja menampilkan secret sekali, ketika client diboeat. Simpan dengan *Download JSON*.
5. Di *Audience*: selama statoes *Testing*, tambahkan email setiap akoen Google pemakai di
   *Test users* (paling banjak 100). **Login dalam mode Testing kedaloewarsa setelah 7 hari**; aplikasi
   menandainja dengan titik merah dan tombol *Login ulang*. Agar tidak kedaloewarsa dan akoen lain
   dapat login tanpa didaftarkan, klik *Publish app*. Google meminta alamat beranda dan kebidjakan
   privasi di *Branding*, tetapi oentoek izin `drive.file` tidak perloe verifikasi. Izin penoeh
   (`drive`) tetap memoentjoelkan lajar "aplikasi belum diverifikasi".

Ketika login, Google boleh djadi menampilkan *"Google hasn't verified this app"*. Karena aplikasinja
milik Saoedara sendiri, klik *Continue* atau *Advanced* laloe *Go to … (unsafe)*.

### Pengaturan > Client ID Google

Client ID Google diatoer **sekali oentoek seloeroeh aplikasi**, di *Pengaturan* (ikon ⚙ di dasar daftar
lokasi), boekan lagi lewat menoe ⋯ tiap akoen atau dialog tambah akoen.

| Di Pengaturan | Artinja |
|---|---|
| **Terpasang** `1234…` (dari komputer ini) | Client ID disimpan lewat Pengaturan di kompoeter ini. |
| **Terpasang** `1234…` (dari installer) | Client ID ikoet dibawa installer; tidak dapat dihapoes dari sini. |
| **Belum ada** | Akoen Google memakai client ID bersama rclone (kerap dibatasi). |
| *Pasang / Ganti client ID…* | Isi Client ID + secret; langsoeng tersimpan, tanpa login. |
| *Hapus* | Menghapoes client ID jang disimpan di kompoeter ini (bawaan installer berlakoe lagi bila ada). |
| *Ambil dari akun X* | Moentjoel bila beloem ada client ID aplikasi tetapi akoen X soedah memakai client ID sendiri. |
| Daftar *Akun Google* | Tiap akoen: "memakai client ID aplikasi", "memakai client ID lain", atau "memakai client ID bersama". |
| *Login ulang dengan client ID ini* | Oentoek akoen jang beloem memakai client ID aplikasi; browser terboeka oentoek login. |

Client ID aplikasi:
- dipakai otomatis oentoek setiap akoen Google jang **ditambahkan** sesoedahnja. Akoen jang soedah ada
  baroe ikoet memakainja setelah *Login ulang dengan client ID ini*;
- disimpan di `%LOCALAPPDATA%\pindah-drive\client-google.json`. Letaknja sengadja di loear folder
  projek, karena folder projek berada di web root Laragon;
- **ikoet dibawa installer** ketika `buat-installer.ps1` didjalankan. Rekan jang memasang langsoeng dapat
  login, asalkan email mereka soedah Saoedara tambahkan di *Test users* (atau aplikasinja soedah di-publish).

Timbanglah sebeloem membagikan: siapa poen jang memegang installer dapat membatja client ID dan
secret itoe, laloe ikoet memakai koeota project Saoedara. Oentoek client djenis *Desktop app*, Google
menganggap ini wadjar, tetapi djangan oenggah installer seperti itoe ke tempat oemoem. Oentoek
installer oemoem, goenakan `.\buat-installer.ps1 -TanpaClient`.

## Verifikasi: seberapa teliti?

Checksum (isi file) dipakai bila kedoea sisi mempoenjai djenis checksum jang sama; selain itoe jang
ditjotjokkan nama dan oekoerannja. Tjontohnja:

| Soember ↔ toedjoean | Tjara mentjotjokkan |
|---|---|
| Kompoeter ↔ Google Drive, Google Drive ↔ Google Drive | checksum MD5 |
| Kompoeter ↔ OneDrive, OneDrive ↔ OneDrive | checksum QuickXorHash |
| Kompoeter ↔ S3; Kompoeter ↔ SFTP (bila server menjediakan `md5sum`/`sha1sum`) | checksum MD5/SHA-1 |
| OneDrive ↔ Google Drive, Kompoeter ↔ WebDAV/FTP | nama & oekoeran sadja (tidak ada djenis checksum jang sama) |

Verifikasi tidak mengoendoeh oelang: checksum dibatja dari metadata jang disimpan tiap lajanan. Hasil
verifikasi selaloe menjeboet tjaranja, dan laporan per file di *Riwayat & laporan* mentjatatnja.

## Batasan

- Koeota toedjoean ditjek sebeloem menjalin, bila lajanannja melaporkan koeota (FTP, S3, B2, dan
  sedjenisnja tidak). Isi sampah ikoet dihitoeng dalam koeota.
- Google membatasi oenggahan **750 GB per hari**. Penjalinan berhenti sendiri dan dapat
  dilandjoetkan esok hari.
- Lajanan bertanda *beta* (Zoho WorkDrive, iCloud Drive, Proton Drive, Jottacloud) bergantoeng
  pada doekoengan rclone jang masih berkembang. Login iCloud perloe dioelang kira-kira tiap 30 hari
  dan memerloekan *Advanced Data Protection* dimatikan.
- Djadwal backup dengan lajanan jang login-nja kedaloewarsa (Google mode Testing, iCloud) gagal
  sampai Saoedara login oelang; hasilnja tampil di notifikasi dan riwajat.
- Bila soembernja folder OneDrive di kompoeter, file jang statoesnja "hanja online" dioendoeh doeloe
  ke disk. Aplikasi menghitoeng apakah roeang disknja tjoekoep.
- **Notebook OneNote** dan **Personal Vault** tidak ikoet tersalin.
- Dokoemen Google (Docs/Sheets/Slides) jang disalin ke loear Google Drive dioebah mendjadi
  `.docx`/`.xlsx`/`.pptx`.

## Privasi

Tidak ada server LELEMOVE, akoen LELEMOVE, analitik, atau telemetri. Kebidjakan lengkapnja ada di
[docs/privasi.md](docs/privasi.md) (djoega di aplikasi: *Bantuan & info > Privasi*).

- Token login dan data login lajanan berisian (alamat, penggoena, kata sandi terobfoeskasi, koentji
  akses) disimpan oleh rclone di `%APPDATA%\rclone\rclone.conf`. Djangan bagikan file ini;
  obfoeskasi boekan enkripsi.
- Pengatoeran, statoes, djadwal, riwajat, dan log ada di `%LOCALAPPDATA%\pindah-drive\`. Log jang
  lebih toea dari 60 hari dihapoes otomatis setiap kali penjalinan dimoelai, ketjoeali log
  penjalinan terachir. Riwajat (`riwayat.jsonl`) tidak dihapoes otomatis karena dipakai oentoek audit.
- *Lepas akun* hanja menghapoes koneksi di kompoeter ini, boekan file di cloud. Oentoek mentjaboet
  izin sepenoehnja, hapoes akses "rclone" di <https://myaccount.google.com/permissions>, di
  pengatoeran akoen Microsoft, atau tjaboet kata sandi aplikasi/koentji akses di lajanan lain.

## Baris perintah

Semoea jang ada di aplikasi dapat poela didjalankan lewat `pindah.exe` (di folder aplikasi).
Daftar lengkap opsinja: `pindah bantuan`. Tambahkan `-Mesin` oentoek ringkasan tersoesoen: baris
terachir keloearannja beroepa `##HASIL {json}`.

```powershell
pindah cek                                              # rclone, akun, folder OneDrive, rute
pindah pembaruan [-Unduh | -Jalankan]                   # periksa rilis terbaru; unduh (+cek SHA-256) / pasang
pindah akun                                             # daftar akun
pindah hubungkan -Akun pd-onedrive-2 -Jenis onedrive -Label "Kantor"
pindah hubungkan -Akun pd-gdrive -Ulang                 # login ulang (token kedaluwarsa)
pindah hubungkan -Akun pd-gdrive -IzinPenuh             # agar bisa jadi sumber
pindah klien                                            # client ID Google aplikasi + akun yang memakainya
pindah klien -ClientId X -ClientSecret Y                # simpan client ID aplikasi (tanpa login)
pindah klien -Hapus                                     # hapus client ID yang disimpan di komputer ini
pindah hubungkan -Akun pd-gdrive -ClientAplikasi        # login ulang memakai client ID aplikasi
pindah hubungkan -Akun pd-sharepoint -Jenis sharepoint  # login Microsoft
pindah situs -Akun pd-sharepoint -Cari keuangan         # daftar situs (+ Id)
pindah pustaka -Akun pd-sharepoint -SitusId <Id>        # daftar pustaka (+ Id)
pindah pustaka -Akun pd-sharepoint -Pustaka <Id> -SitusNama Keuangan -PustakaNama Dokumen
pindah periksa -Sumber "pd-gdrive:Arsip" -Tujuan "pd-onedrive:Pindahan Google Drive/Arsip"
pindah uji
pindah salin -Latar -Ya
pindah salin -Latar -Ya -BatasKecepatan 5M              # batasi 5 MB/dtk (diingat; "off" = tanpa batas)
pindah status
pindah verifikasi
pindah salin -Latar -Ya -Sertakan "pdf,docx" -Lewati "*.tmp,Arsip Lama/" -Sejak 2026-01-01   # filter (diingat)
pindah salin -Latar -Ya -TanpaFilter                    # hapus filter
pindah pindahkan                                        # verifikasi ulang + tampilkan berapa file yang akan dihapus
pindah pindahkan -Ya                                    # hapus dari SUMBER file yang cocok (ke Recycle Bin/Sampah)
pindah penyedia                                         # daftar layanan + isian masing-masing
pindah hubungkan -Akun pd-dropbox -Jenis dropbox        # layanan ber-login browser
'{"pass":"..."}' | pindah hubungkan -Akun pd-sftp -Jenis sftp -Opsi host=nas.kantor -Opsi user=admin -RahasiaStdin
pindah hubungkan -Akun pd-icloud -Lanjut -State <S> -Jawab 123456   # jawab pertanyaan lanjutan (kode keluar 10 = ada pertanyaan)
pindah impor                                            # remote rclone yang bisa dipakai
pindah impor -Akun kantorku -Label "Server lama"        # pakai remote rclone yang sudah ada
pindah riwayat [-Jumlah 50]                             # riwayat salin, verifikasi, pindahkan
pindah riwayat -Csv riwayat.csv | -Laporan laporan.html # ekspor semua
pindah riwayat -Id <id> -Laporan verifikasi.html        # laporan per file satu verifikasi (atau -Csv)
pindah jadwal                                           # daftar backup terjadwal
pindah jadwal -Tambah -Nama "Dokumen" -Sumber "D:\Data" -Tujuan "pd-gdrive:Arsip" -Jam 22:00 -Hari sen,rab,jum
pindah jadwal -Tambah -Id <id> ... [-TanpaVerifikasi]   # ubah jadwal yang ada (isian lengkap lagi)
pindah jadwal -Id <id> -Nonaktifkan                     # atau -Aktifkan (memasang ulang tugas yang hilang)
pindah jadwal -Id <id> -Jalankan                        # jalankan sekarang di konsol ini
pindah jadwal -Id <id> -Hapus
```

Soember dan toedjoean ditoelis sebagai folder kompoeter (`D:\Data`) atau `<akun>:<folder>`; oentoek
object storage `<akun>:<bucket>/<folder>`, oentoek SMB `<akun>:<share>/<folder>`. Tanpa perintah
(atau lewat `Pindah.cmd`), `pindah` memboeka menoe teks.

| Kode keloear | Arti |
|---|---|
| 0 | Berhasil |
| 1 | Galat (pesan di keloearan; mis. login kedaloewarsa, folder tidak ada) |
| 2 | `periksa`: roeang toedjoean tidak tjoekoep |
| 3 | Perloe perhatian: `uji` menemoekan file jang akan terlewat, atau `pindahkan` menoenggoe persetoedjoean (`-Ya`) |
| 10 | `hubungkan`: rclone mengadjoekan pertanjaan landjoetan (lihat `Tanya` di `##HASIL`) |

- `-BatasKecepatan` diteroeskan ke rclone sebagai `--bwlimit`, dengan satoean 1024 per detik:
  `500K`, `5M`, `1.5M`; angka tanpa satoean berarti KB/dtk. Oenggah dan oendoeh dapat dibedakan
  dengan `unggah:unduh`, misalnja `10M:5M`. Nilainja disimpan di `pengaturan.json`
  (`BatasKecepatan`) dan djoega dipakai djendela aplikasi; `-BatasKecepatan off` menghapoesnja.
- `salin -Latar` mengirim notifikasi Windows ketika selesai atau berhenti karena galat, djoega
  bila djendela aplikasi soedah ditoetoep. *Hentikan* tidak memoentjoelkan notifikasi.
- `salin -Latar` melandjoetkan sendiri ketika djaringan poetoes (lihat
  [Djaringan poetoes: landjoet otomatis](#djaringan-poetoes-landjoet-otomatis)); `status` laloe
  menampilkan `Keadaan: MENUNGGU JARINGAN` dan perkiraan djamnja.
- Setiap `salin` menghapoes berkas di `%LOCALAPPDATA%\pindah-drive\log` jang lebih toea dari
  60 hari, ketjoeali log jang tertjatat di `job.json`.
- `jadwal -Hari` menerima `setiap` (bawaan) atau nama hari dipisah koma: `sen,sel,rab,kam,jum,sab,min`
  (nama lengkap djoega boleh). `-Jam` memakai format `22:00`. Djadwal memakai batas ketjepatan jang
  tersimpan di `pengaturan.json` ketika ia berdjalan, dan tidak mengoebah roete jang diboeka di djendela.

<a name="untuk-pengembang"></a>
## Oentoek pengembang

```
rust/                          kode sumber versi 2 (satu crate Rust)
  src/mesin/                   mesin: akun, rute, job & log rclone, semua perintah
  src/mesin/perintah/          satu berkas per kelompok perintah (akun, hubungkan, salin, verifikasi, email, ...)
  src/mesin/pembaruan.rs       pembaruan dari GitHub Releases: cek versi, unduh, cocokkan SHA-256
  src/win.rs                   lapisan sistem Windows (Win32: proses, mutex, folder dikenal, layar tetap menyala)
  src/mac.rs                   padanan macOS dengan nama fungsi yang sama (libc, NSFileManager, osascript)
  src/notifikasi_mac.rs        notifikasi macOS (osascript); notifikasi.rs = toast Windows
  src/gaya.rs                  tampilan egui bersama (warna, huruf Segoe UI / Helvetica Neue,
                               ikon Segoe MDL2 / simbol Unicode, tombol)
  src/bin/pindah.rs            pindah.exe: baris perintah & menu teks
  src/mesin/lanjut.rs          lanjut otomatis saat jaringan putus (keputusan, jeda, uji sambungan)
  src/mesin/jadwal.rs          jadwal backup: jadwal.json, XML Task Scheduler + schtasks (Windows),
                               plist LaunchAgent + launchctl (macOS)
  src/mesin/penyedia.rs        daftar layanan: backend rclone, isian, login, tempat sampah, host API
  src/mesin/filter.rs          filter file -> aturan --filter / --max-age rclone
  src/mesin/riwayat.rs         riwayat.jsonl, CSV, laporan HTML siap cetak
  src/mesin/imap.rs            klien IMAP kecil (TLS sistem lewat native-tls, PLAIN/XOAUTH2, LIST, FETCH, APPEND)
  src/mesin/email.rs           migrasi & cadangan email: sumber/tujuan server atau komputer, peta folder
                               (Gmail/Outlook/cPanel), cocok Message-ID, sambung ulang
  src/mesin/email_lokal.rs     cadangan .eml di komputer: indeks per folder, nama berkas, tanggal berkas
  src/mesin/mbox.rs            baca mbox (Google Takeout berlabel, Thunderbird) untuk pemulihan email
  src/mesin/oauth.rs           login OAuth Google/Microsoft untuk IMAP (PKCE, pengalihan ke localhost)
  src/mesin/kode.rs            base64, SHA-256, percent-encoding (tanpa crate tambahan)
  src/bin/pindah-drive/        LELEMOVE.exe: jendela aplikasi (egui) + dialog
  src/bin/pindah-drive/app_*.rs bagian jendela utama (kabar utas latar, panel, lokasi, salin, tampilan, ...)
  src/bin/pindah-drive/info.rs dialog Bantuan & info (tanya jawab, ketentuan, privasi, atribusi, tentang)
  src/bin/pindah-drive/jadwal.rs dialog Jadwal backup (daftar + isian)
  src/bin/pindah-drive/filter.rs dialog Filter file
  src/bin/pindah-drive/riwayat.rs dialog Riwayat & laporan
  src/bin/pindah-drive/layanan.rs dialog Tambah akun layanan lain, isian, pertanyaan lanjutan, impor
  src/bin/pindah-drive/email.rs dialog Migrasi email (isian, progres, hasil per folder)
  src/bin/setup.rs             installer Windows; tanpa muatan menjadi uninstall.exe
  src/bin/tanam-ikon.rs        alat build Windows: menanam ikon & manifest (UpdateResourceW)
                               (setup/main.rs dan tanam-ikon/main.rs: pembungkus, kosong di macOS)
  aset/app.manifest            asInvoker + Common Controls 6
  aset/atribusi.txt            daftar pustaka Rust + lisensi; dibuat ulang buat-installer.ps1 (cargo tree)
docs/                          tanya-jawab.md, ketentuan.md, privasi.md: ditanam ke aplikasi
                               (include_str!), dan privasi.md bisa dipakai sebagai halaman kebijakan
                               privasi untuk "Publish app" di Google Cloud Console
aset/ikon.ico, aset/ikon.icns  ikon aplikasi (lele) Windows dan macOS; dibuat ulang dengan:
                               python aset\buat-ikon.py aset\ikon.ico pratinjau.png aset\ikon.icns
buat-installer.ps1             membuat dist\LELEMOVE-Setup-<versi>.exe (rclone ikut dibundel;
                               -TanpaClient = tanpa client ID Google, untuk rilis publik)
buat-dmg.sh                    membuat dist/macos/LELEMOVE-<versi>-macOS.dmg di Mac (universal;
                               rclone resmi diunduh dan dicek SHA-256-nya; tanpa client ID Google)
.github/workflows/macos.yml    uji unit di Mac + .dmg sebagai artefak, setiap push ke main / fitur/macos
skills/pindah-drive/SKILL.md   panduan untuk Claude; .claude-plugin/ = manifest plugin lokal
arsip-v1-powershell/           sumber versi 1.x (PowerShell + WPF + C#), tidak dipakai lagi
```

Build memakai toolchain Rust **GNU** per penggoena (tanpa Visual Studio):

```powershell
winget install Rustlang.Rustup --custom "--default-host x86_64-pc-windows-gnu --profile minimal"
.\buat-installer.ps1
```

Tjatatan toolchain: rustup GNU tidak membawa assembler `as.exe`, djadi crate jang me-link lewat
`raw-dylib`/`windows-link` tidak dapat dibangoen. `Cargo.lock` mengoentji beberapa crate ke versi
jang masih memakai import library biasa (lihat komentar di `rust/Cargo.toml`), dan `build.rs`
menjediakan `libshlwapi.a`. Oedji oenit: `cargo test --release --lib --bin pindah-drive` di folder
`rust`. Oentoek mengoedji landjoet otomatis tanpa memoetoes internet: djalankan `salin -Latar` dengan
`HTTPS_PROXY=http://127.0.0.1:9` (semoea permintaan rclone gagal seperti djaringan poetoes),
`PINDAH_DRIVE_JEDA_ULANG=3` (djeda dalam detik), dan `PINDAH_DRIVE_MAKS_ULANG=3`.

Versi macOS dibangoen di Mac: `rustup target add aarch64-apple-darwin x86_64-apple-darwin`, laloe
`./buat-dmg.sh`. Tanpa Mac, kode macOS tetap dapat diperiksa dari Windows dengan
`cargo check --target aarch64-apple-darwin --all-targets` (setelah `rustup target add aarch64-apple-darwin`);
oedji oenitnja didjalankan workflow macOS di GitHub Actions. Foengsi choesoes sistem ada di `win.rs`
dan `mac.rs` dengan nama jang sama (`lib.rs` memilih salah satoenja), sehingga kode lain tjoekoep
memanggil `win::...`.

Oentoek mengoedji djadwal tanpa menjentoeh data asli: isi `PINDAH_DRIVE_DATA` dengan folder kosong,
laloe djalankan `target\release\pindah.exe jadwal -Tambah ...` dengan djam 1-2 menit dari sekarang.
Toegas jang diboeat ketika variabel itoe terpasang membawa `--data "<folder>"`, sehingga pemitjoe dari
Task Scheduler djoega memakai folder oedji. Pitjoe tanpa menoenggoe:
`schtasks /Run /TN "LELEMOVE - Backup <id>"`. Hapoes djadwal oedji dengan `pindah jadwal -Id <id> -Hapus`.

Tjara kerdjanja: setiap tombol di djendela mendjalankan `pindah.exe <perintah> -Mesin` sebagai proses
tersemboenji. Teks keloearannja masoek ke *Log aktivitas*, sedangkan satoe baris `##HASIL {json}`
berisi angka hasilnja. Progres penjalinan dibatja dari `job.json` dan log rclone. Daftar folder
di panel Dari/Ke dimoeat di thread latar (`rclone lsf --dirs-only` oentoek cloud, `read_dir` oentoek
kompoeter ini). Ikon memakai hoeroef Segoe MDL2 Assets bawaan Windows 10/11; di macOS simbol Unicode
dari hoeroef bawaan egui (oedji `ikon_unicode_ada_di_huruf_bawaan` memastikan semoeanja ada); petak
lajanan lain memakai inisial dan warna dari `penyedia.rs`. Pratindjau djendela oentoek dokoementasi:

```powershell
"LELEMOVE.exe" --pratinjau layar.png [--dialog <nama>] [--log] [--lebar 760] [--tinggi 820] [--tunda 2500]
# <nama>: pengaturan, client, akun-google, info:<tab>, jadwal, jadwal-baru, filter, riwayat,
#         layanan, kredensial:<jenis>, tanya-konfig, impor, pindahkan, pindahkan-tanya,
#         jelajah:<akun>, sharepoint:<akun>:<kata>
```

Pakai `PINDAH_DRIVE_DATA=<folder sementara>` (dan `RCLONE_CONFIG=<berkas sementara>`) agar data
dan akoen asli tidak tersentoeh. Tangkapan lajar di README diboeat dengan tjara ini.

Oentoek mengoedji lajanan berisian tanpa akoen soenggoehan: djalankan server lokal dengan rclone dan
pakai konfigurasi rclone terpisah (`RCLONE_CONFIG`), soepaja `rclone.conf` asli tidak tersentoeh:

```powershell
rclone serve webdav C:\uji\dav --addr 127.0.0.1:18081 --user uji --pass rahasia   # juga: serve sftp / ftp / s3 --auth-key AK,SK
$env:RCLONE_CONFIG = "C:\uji\rclone-uji.conf"; $env:PINDAH_DRIVE_DATA = "C:\uji\data"
'{"pass":"rahasia"}' | .\target\release\pindah.exe hubungkan -Akun pd-webdav -Jenis webdav -Opsi url=http://127.0.0.1:18081 -Opsi user=uji -RahasiaStdin
```

### Membuat rilis

Installer oentoek oemoem **wadjib** dibangoen dengan `-TanpaClient`. Tanpa opsi itoe, client ID dan
secret Google dari `%LOCALAPPDATA%\pindah-drive\client-google.json` ikoet masoek ke installer.
Kedoeanja menoelis ke nama berkas jang sama di `dist\`, djadi pindahkan doeloe installer pribadi bila
masih dipakai.

```powershell
# 1. Naikkan "version" di rust\Cargo.toml (dan .claude-plugin\plugin.json), tulis catatan rilis di
#    catatan-rilis\<versi>.md (baris @@CHECKSUM@@ diisi otomatis), perbarui README, commit, lalu push.
# 2. Beri tag. Workflow "Rilis" membangun installer Windows dan .dmg macOS, lalu membuat draf rilis.
git tag v<versi>
git push origin v<versi>
# 3. Periksa drafnya (catatan dan empat berkas), lalu terbitkan.
gh release edit v<versi> --draft=false --latest
```

Workflow *Rilis* memakai installer dari workflow Windows (dibangoen dengan `-TanpaClient`, diperiksa
`alat/periksa_installer.py`, dan dioedji aloer pembaroeannja) serta berkas .dmg dari workflow macOS.
Draf rilis beloem terlihat oleh oemoem dan beloem ditawarkan tombol *Periksa pembaruan* sampai
diterbitkan.

Bila perloe membangoen sendiri di kompoeter: djalankan `.\buat-installer.ps1 -TanpaClient`, laloe
`python alat\periksa_installer.py <installer>`. Djangan mengoenggah installer jang beloem lolos
pemeriksaan itoe. Alat oentoek menjoenting README dan mengoedji pembaroean didjelaskan di
[alat/README.md](alat/README.md).

Memakai sebagai plugin Claude Code:

```powershell
claude plugin marketplace add C:\laragon\www\pindah-drive
claude plugin install pindah-drive@pindah-drive-lokal
```

<a name="dukung"></a>
## Doekoeng

LELEMOVE pertjoema dan soembernja terboeka. Bila aplikasi ini menghemat waktoe Saoedara atau kantor
Saoedara, sokongan lewat **[saweria.co/pemecuteai](https://saweria.co/pemecuteai)** membantoe
pengembangannja (fitoer baroe, pengoedjian lajanan, dan pemeliharaan versi Windows serta macOS).
Tautan jang sama ada di aplikasi (*Bantuan & info > Tentang*) dan di tombol *Sponsor* repositori ini.
Sokongan bersifat soekarela: semoea fitoer tetap terboeka oentoek semoea orang.

Sekali pertjoema, tetap pertjoema!

## Lisensi

LELEMOVE dirilis dengan [lisensi MIT](LICENSE): bebas dipakai, dioebah, dan dibagikan, termasoek
oentoek kantor dan oesaha, selama pemberitahoean hak tjipta dan lisensinja tetap disertakan. Tidak
ada djaminan apa poen. rclone dan poestaka lain jang ikoet dibawa memakai lisensinja masing-masing
(lihat *Bantuan & info > Atribusi*).

<a name="riwayat-versi"></a>
## Riwajat versi

| Versi | Isi oetama |
|---|---|
| [2.5.0](https://github.com/fauzirpl/lelemove/releases/tag/v2.5.0) | Tampilan baroe bertema kolam: daftar lokasi gelap, panel berbentoek kartoe, djarak teks lebih lega; tombol dialog tidak lagi mengambang djaoeh dari isinja; nama tombol terbatja pembatja lajar; memboeka LELEMOVE doea kali menampilkan djendela jang soedah ada; lisensi MIT |
| [2.4.4](https://github.com/fauzirpl/lelemove/releases/tag/v2.4.4) | Pembaroean mentjoba oelang sendiri ketika sambungan ke GitHub gagal (misalnja DNS poetoes sesaat), sampai lima kali; oendoehan jang terpoetoes dioelang |
| [2.4.3](https://github.com/fauzirpl/lelemove/releases/tag/v2.4.3) | Perbaikan *Unduh & pasang*: installer menoenggoe LELEMOVE benar-benar tertoetoep sebeloem mengganti berkas (sebeloemnja kadang gagal dengan pesan berkas sedang dipakai) |
| [2.4.2](https://github.com/fauzirpl/lelemove/releases/tag/v2.4.2) | Tombol *Periksa pembaruan* / *Unduh & pasang* di *Bantuan & info > Tentang* (installer ditjek SHA-256-nja, LELEMOVE diboeka lagi sendiri); tjatatan izin admin oentoek akoen Outlook kantor |
| [2.4.1](https://github.com/fauzirpl/lelemove/releases/tag/v2.4.1) | Menoe *Cadangan email* sendiri di daftar kiri (tab *Cadangkan ke komputer* dan *Pulihkan ke server*); *Migrasi email* kembali choesoes server ke server |
| [2.4.0](https://github.com/fauzirpl/lelemove/releases/tag/v2.4.0) | Tjadangan email ke kompoeter (.eml) dan pemoelihan ke server, termasoek dari berkas .mbox Google Takeout atau Thunderbird; baris *Dukung LELEMOVE* di djendela oetama |
| [2.3.0](https://github.com/fauzirpl/lelemove/releases/tag/v2.3.0) | Versi macOS (beta; Apple Silicon + Intel, .dmg), migrasi email (beta) dari Gmail/Outlook/IMAP ke server email sendiri, tautan doekoengan Saweria, naskah README berëdjaan zaman perdjoeangan |
| [2.2.0](https://github.com/fauzirpl/lelemove/releases/tag/v2.2.0) | 29 lajanan penjimpanan (Dropbox, Box, pCloud, MEGA, iCloud, Nextcloud, SMB/NAS, SFTP, FTP, S3, R2, B2, Azure, ...), pertanjaan landjoetan/2FA, pakai remote rclone jang soedah ada |
| [2.1.0](https://github.com/fauzirpl/lelemove/releases/tag/v2.1.0) | Filter file, riwajat & laporan audit (CSV, HTML siap tjetak), Pindahkan dengan verifikasi oelang |
| [2.0.0](https://github.com/fauzirpl/lelemove/releases/tag/v2.0.0) | Ditoelis oelang dalam Rust: djendela doea panel, nama LELEMOVE, landjoet otomatis ketika djaringan poetoes, djadwal backup, Bantuan & info |
| 1.x | Versi PowerShell + WPF (*Pindah Drive*); soembernja di `arsip-v1-powershell/` |
