# Keamanan

## Melaporkan celah

Jangan membuka Issue publik untuk celah keamanan. Pakai
[laporan kerentanan pribadi di GitHub](https://github.com/fauzirpl/lelemove/security/advisories/new)
(*Security > Report a vulnerability*). Sertakan versi LELEMOVE, sistem operasi, langkah untuk
mengulang, dan dampaknya. Jangan menyertakan token, kata sandi, atau isi `rclone.conf` milik Anda.

Yang didukung hanya rilis terbaru di halaman
[Releases](https://github.com/fauzirpl/lelemove/releases/latest).

## Yang perlu diketahui pengguna

- **Installer belum ditandatangani secara digital.** Windows SmartScreen dan macOS Gatekeeper
  memperingatkan saat pertama dibuka. Unduh hanya dari halaman Releases resmi.
- **Berkas `.sha256` memeriksa keutuhan, bukan keaslian.** Tombol *Unduh & pasang* mencocokkan
  installer dengan berkas `.sha256` dari rilis yang sama. Itu menangkap unduhan yang rusak atau
  terpotong, tetapi tidak melindungi bila akun GitHub proyek ini sendiri yang disalahgunakan.
- **Sambungan akun disimpan rclone** di `rclone.conf` tanpa enkripsi (bawaan rclone). Siapa pun yang
  bisa membaca berkas itu di komputer Anda bisa memakai akun tersebut; lindungi akun Windows/macOS Anda.
- **Kata sandi email tidak disimpan.** Kata sandi diberikan ke mesin lewat stdin dan hanya ada di
  memori selama migrasi atau cadangan berjalan.
- **Installer publik tidak membawa client ID Google.** Setiap rilis diperiksa otomatis
  (`alat/periksa_installer.py`) sebelum diterbitkan.
- **Opsi `-AbaikanSertifikat`** pada migrasi email mematikan pemeriksaan sertifikat server. Pakai
  hanya untuk server sendiri yang sertifikatnya swatanda.
