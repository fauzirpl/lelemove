---
name: pindah-drive
description: Menyalin atau memindahkan file antar OneDrive, SharePoint, Google Drive, dan folder komputer (ke arah mana pun, termasuk antar akun) di Windows memakai rclone lewat pindah.exe. Gunakan ketika pengguna ingin memindahkan, menyalin, migrasi, atau membackup file antar layanan cloud, menambah/melepas akun, memantau progres, menangani penyalinan yang macet atau login yang kedaluwarsa, atau memverifikasi bahwa semua file sudah sampai.
---

# LELEMOVE (versi 2, Rust)

Semua pekerjaan dilakukan oleh **`pindah.exe`**. Cari di urutan ini:

1. Aplikasi terpasang: `%LOCALAPPDATA%\Programs\LELEMOVE\pindah.exe` (pemasangan sebelum
   ganti nama: `%LOCALAPPDATA%\Programs\Pindah Drive\pindah.exe`)
2. Hasil build di plugin ini: `<akar plugin>\rust\target\release\pindah.exe`
   (akar plugin = dua tingkat di atas direktori skill ini)

```powershell
& "$env:LOCALAPPDATA\Programs\LELEMOVE\pindah.exe" <perintah> [opsi]
```

Opsi bergaya PowerShell dan tidak peka huruf besar-kecil (`-Sumber X`, `-Ya`); `--sumber X` juga
diterima. Tambahkan `-Mesin` bila butuh ringkasan terstruktur: baris terakhir `##HASIL {json}`.

Kalau pengguna lebih suka mengerjakannya sendiri lewat jendela, arahkan ke aplikasi
`LELEMOVE.exe` di folder yang sama. Aplikasi itu memakai mesin dan data kerja yang sama,
jadi progres penyalinan yang dimulai dari sini juga tampil di sana, dan sebaliknya.

## Aturan yang tidak boleh dilanggar

- **Menyalin, bukan menghapus.** Jangan pernah menjalankan `rclone sync`, `move`, `delete`,
  `purge`, `deletefile`, atau `cleanup` sendiri. Satu-satunya jalan menghapus adalah
  `pindah pindahkan`, dan hanya bila pengguna sendiri meminta file di SUMBER dihapus setelah
  verifikasi. Jalankan dulu tanpa `-Ya` (memverifikasi ulang dan menyebut jumlah file), sampaikan
  jumlah dan tempat sampahnya (Recycle Bin / Sampah Google Drive / recycle bin OneDrive), lalu
  tambahkan `-Ya` hanya setelah pengguna menyetujui angka itu di chat. Jangan pernah untuk
  tujuan, untuk drive jaringan/flashdisk, atau di dalam jadwal.
- **Login dilakukan pengguna di browser.** Jangan meminta, mengetik, atau menampilkan password,
  kode, maupun token. Jangan membaca atau menampilkan `%APPDATA%\rclone\rclone.conf`
  (isinya token).
- **Client ID Google** dibuat pengguna sendiri di Google Cloud Console; jangan membuatkannya
  tanpa izin. Jangan pernah mengetik atau menyalin client secret; pengguna yang menempelkannya.
- **Minta izin di chat dulu** sebelum `pasang` (mengunduh & memasang rclone lewat winget,
  sekaligus menyetujui ketentuan winget) dan sebelum `salin` (mulai mengunggah). Setelah
  pengguna setuju, baru tambahkan `-Ya`. Juga sebelum `hubungkan ... -IzinPenuh`.
- Jangan menebak folder sumber, situs, atau pustaka SharePoint. Tampilkan pilihannya dan
  biarkan pengguna memilih.

## Akun

- Akun cloud = remote rclone berawalan `pd-` (`pd-gdrive`, `pd-gdrive-2`, `pd-onedrive`,
  `pd-sharepoint`, ...). Lihat dengan `akun`. Tambah: `hubungkan -Akun <nama-baru>
  -Jenis google|onedrive|sharepoint -Label "X"`. Lepas: `lepas -Akun <nama>` (hanya koneksi).
- **Login kedaluwarsa** (`invalid_grant`): `periksa`/`uji`/`verifikasi` mencetak
  "Login akun X sudah kedaluwarsa" dan `##HASIL` berisi `"PerluLogin": "<akun>"`. Minta
  pengguna login ulang: `hubungkan -Akun <akun> -Ulang -Ya` (browser terbuka). Penyebab umum:
  project Google Cloud pemilik client ID masih berstatus *Testing*, sehingga login berlaku 7 hari.
- Google sebagai SUMBER butuh izin penuh: `hubungkan -Akun <nama> -IzinPenuh`.
- Penyalinan macet di 0 B/s, atau `status` melaporkan rate limit, berarti client ID Google
  bersama rclone sedang dibatasi. Solusinya client ID aplikasi (satu untuk seluruh aplikasi; di
  jendela: Pengaturan > Client ID Google). Cek dengan `klien`; pengguna menyimpannya sendiri
  lewat jendela atau `klien -ClientId X -ClientSecret Y`, lalu akun lama login ulang dengan
  `hubungkan -Akun <nama> -ClientAplikasi -Ya` (browser terbuka; minta izin dulu).
- SharePoint: `hubungkan -Akun pd-sharepoint -Jenis sharepoint` (login), `situs -Akun
  pd-sharepoint -Cari <kata>`, `pustaka -Akun pd-sharepoint -SitusId <Id>`, lalu
  `pustaka -Akun pd-sharepoint -Pustaka <Id> -SitusNama X -PustakaNama Y`. Sebelum pustaka
  dipilih, akun ditolak sebagai sumber/tujuan (masih menunjuk OneDrive pribadi).

## Alur kerja

1. **Cek:** `cek`. Laporkan apakah rclone terpasang, akun apa saja yang terhubung, dan folder
   OneDrive mana saja yang ada di komputer.
2. **Pasang** bila rclone belum ada (biasanya sudah ikut di aplikasi terpasang): `pasang -Ya`.
3. **Pilih rute.** Sumber/tujuan berupa folder komputer (`D:\Data`) atau `<akun>:<folder>`.
   Bawaan tujuan: `<akun tujuan>:Pindahan <asal>/<nama folder sumber>`. Rute disimpan otomatis
   saat `-Sumber`/`-Tujuan` diberikan ke perintah apa pun.
4. **Periksa:** `periksa -Sumber "<sumber>" -Tujuan "<tujuan>"`. Kode keluar: 0 siap, 1 galat
   (baca pesan; bisa login kedaluwarsa), 2 ruang tujuan kurang, 3 ada peringatan.
5. **Uji:** `uji` (dry run). Kode 3 berarti rclone tidak membaca sebagian file. Sarankan sumber
   lewat akun cloud (`pd-onedrive:`). Jangan lanjut tanpa persetujuan.
6. **Salin:** setelah pengguna setuju, `salin -Latar -Ya`. Proses berjalan tersembunyi dan tidak
   terikat jendela. Cek `status` sekitar 30 detik kemudian, lalu setiap kali pengguna bertanya.
   Jangan memantau dengan polling rapat. Hentikan: `berhenti`; lanjutkan: `salin -Latar -Ya`
   lagi (file yang sudah ada dilewati). Bila pengguna ingin internetnya tetap lancar, tambahkan
   `-BatasKecepatan 5M` (5 MB/dtk; diingat untuk penyalinan berikutnya, `off` = tanpa batas).
   Saat selesai atau berhenti karena galat, Windows menampilkan notifikasi ke pengguna.
7. **Verifikasi:** `verifikasi` setelah `status` menunjukkan SELESAI. Checksum dipakai bila kedua
   sisi punya jenis yang sama; OneDrive ↔ Google Drive hanya nama & ukuran.
8. **Laporan (bila diminta):** `riwayat` menampilkan catatan beserta `-Id`-nya;
   `riwayat -Id <id> -Laporan <berkas.html>` membuat laporan per file untuk audit,
   `riwayat -Csv <berkas.csv>` mengekspor semua.

**Filter:** bila pengguna hanya ingin jenis file tertentu, tambahkan `-Sertakan "pdf,docx"`,
`-Lewati "*.tmp,Folder/"`, atau `-Sejak 2026-01-31` ke `periksa`/`uji`/`salin`/`verifikasi`.
Filter diingat di `pengaturan.json` untuk perintah berikutnya; `-TanpaFilter` menghapusnya.
Sebutkan filter yang aktif saat melaporkan hasil, karena file di luar filter memang tidak disalin.

Cara kerja data: antar akun berbeda, data dialirkan lewat komputer (unduh + unggah sekaligus di
memori, tidak disimpan ke disk). Komputer harus tetap menyala selama menyalin.

## Backup terjadwal

Untuk "backup otomatis setiap hari/minggu": `jadwal -Tambah -Nama "X" -Sumber "<sumber>"
-Tujuan "<tujuan>" -Jam 22:00 [-Hari sen,rab,jum] [-TanpaVerifikasi]`. Ini membuat tugas Task
Scheduler Windows `LELEMOVE - Backup <id>` milik pengguna (tanpa admin) yang menyalin pada jam
itu walau jendela tidak dibuka, dan memverifikasi bila diminta. Minta izin di chat dulu (sama
dengan `salin`: setiap jadwal mengunggah tanpa ditanya lagi), dan biarkan pengguna menentukan jam,
hari, sumber, dan tujuannya. Kelola dengan `jadwal` (daftar, jadwal berikutnya, hasil terakhir),
`jadwal -Id <id> -Nonaktifkan|-Aktifkan|-Hapus`. `jadwal -Id <id> -Jalankan` menyalin langsung di
konsol (minta izin dulu). Jadwal tidak mengubah rute yang tersimpan di `pengaturan.json`.

## Membaca masalah umum

| Gejala | Arti dan tindakan |
|---|---|
| `invalid_grant` / "Login akun X kedaluwarsa" | Login ulang: `hubungkan -Akun X -Ulang -Ya` |
| `403 rateLimitExceeded` di log | Client ID bersama rclone dibatasi. Pakai client ID sendiri. |
| Kode 7 / `upload limit` | Batas unggah Google 750 GB/hari. Lanjutkan besok dengan `salin`. |
| `storageQuotaExceeded` | Kuota Google Drive penuh. Kosongkan sampah atau tambah kuota. |
| "Need admin approval" saat login Microsoft | Kebijakan admin tenant. Pakai folder OneDrive di komputer. |
| Isi `pd-onedrive:` hanya `WebPartCache` dsb. | Drive salah terpilih: `hubungkan -Akun pd-onedrive -Ulang -Ya`. |
| File `.url` di sumber | Notebook OneNote. Isinya tidak ikut, hanya pintasan. |
| Personal Vault | Tidak ikut tersalin. Pengguna harus memindahkannya manual. |

Data kerja ada di `%LOCALAPPDATA%\pindah-drive\` (`pengaturan.json`, `job.json`, `akun.json`,
`jadwal.json`, `riwayat.jsonl`, folder `log\`). Bila butuh detail, baca log terbaru di sana, bukan `rclone.conf`.
