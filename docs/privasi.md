# Kebijakan Privasi

Berlaku sejak 23 November 2026.

## Ringkasnya

LELEMOVE berjalan sepenuhnya di komputer Anda. Tidak ada server LELEMOVE, tidak ada akun LELEMOVE, tidak ada analitik atau telemetri, dan tidak ada data yang dikirim ke pengembang LELEMOVE.

## Data yang diakses

- File dan folder di akun yang Anda hubungkan (Google Drive, OneDrive, SharePoint, Dropbox, server NAS/Nextcloud/SFTP, object storage S3, dan layanan lain), serta folder komputer yang Anda pilih.
- Data itu dibaca dan ditulis hanya untuk menjalankan perintah Anda: menelusuri folder, memeriksa ukuran dan kuota, menyalin, dan mencocokkan hasil salinan.
- Isi file tidak dianalisis, tidak dipakai untuk tujuan lain, dan tidak dibagikan kepada pihak mana pun selain layanan tujuan yang Anda pilih.

## Ke mana data mengalir

Data mengalir langsung antara komputer Anda dan server layanan yang Anda hubungkan (Google, Microsoft, Dropbox, server kantor Anda, dan sebagainya), lewat rclone yang berjalan di komputer Anda. Saat menyalin antar akun cloud, data melewati memori komputer Anda tanpa disimpan utuh ke disk.

## Data yang disimpan di komputer Anda

- Token login akun cloud, disimpan oleh rclone di %APPDATA%\rclone\rclone.conf.
- Untuk layanan yang memakai isian (NAS, Nextcloud, SFTP, FTP, S3, MEGA, dan sejenisnya): alamat server, nama pengguna, dan kata sandi atau kunci akses, juga di rclone.conf. Kata sandi disimpan terobfuskasi; ini menyamarkan, bukan enkripsi, sehingga siapa pun yang bisa membaca berkas itu bisa memulihkannya. Pakai kata sandi aplikasi atau kunci akses khusus bila layanannya menyediakan. Kata sandi dikirim ke rclone lewat jalur masukan proses, bukan lewat baris perintah. Kunci akses object storage (secret S3, key B2/Azure) disimpan apa adanya karena begitulah rclone memakainya.
- Pengaturan, daftar akun, status penyalinan, dan log di %LOCALAPPDATA%\pindah-drive. Log memuat nama dan jalur file yang disalin, serta pesan galat. Log yang lebih tua dari 60 hari dihapus otomatis.
- Client ID Google (bila dipasang) di %LOCALAPPDATA%\pindah-drive\client-google.json.
- Jadwal backup (nama, folder sumber dan tujuan, jam, hasil terakhir) di %LOCALAPPDATA%\pindah-drive\jadwal.json, ditambah satu tugas Task Scheduler Windows per jadwal ("LELEMOVE - Backup ...") milik pengguna Windows Anda.
- Riwayat penyalinan, verifikasi, dan pemindahan (waktu, folder sumber dan tujuan, jumlah file, hasil) di %LOCALAPPDATA%\pindah-drive\riwayat.jsonl. Riwayat tidak dihapus otomatis karena dipakai sebagai catatan audit; laporan per file merujuk ke berkas verifikasi di folder log, yang ikut terhapus setelah 60 hari.
- Laporan HTML yang Anda buat memuat nama komputer dan nama pengguna Windows sebagai keterangan pembuat laporan. Laporan dan CSV hasil ekspor disimpan di tempat yang Anda pilih.

Semua data ini tetap di komputer Anda dan tidak dikirim ke mana pun.

## Izin akun Google

- Izin terbatas (drive.file): LELEMOVE hanya bisa melihat dan mengubah file yang dibuatnya sendiri. Ini pilihan bawaan untuk akun yang hanya menjadi tujuan.
- Izin penuh (drive): dibutuhkan bila akun Google menjadi sumber penyalinan, agar file Anda bisa dibaca dan disalin.

LELEMOVE's use and transfer to any other app of information received from Google APIs will adhere to the Google API Services User Data Policy, including the Limited Use requirements. Artinya: pemakaian dan pemindahan informasi yang diterima LELEMOVE dari Google API mematuhi Kebijakan Data Pengguna Layanan Google API, termasuk ketentuan Penggunaan Terbatas.

## Izin akun Microsoft

Untuk OneDrive dan SharePoint, LELEMOVE meminta izin membaca dan menulis file yang bisa Anda akses, serta izin akses berkelanjutan (offline_access) agar penyalinan bisa berjalan tanpa login berulang. Izin itu dipakai hanya untuk menjalankan perintah Anda.

## Mencabut akses dan menghapus data

- Lepas akun: klik ⋯ > Lepas akun ini. Sambungan di komputer ini dihapus; file di cloud tidak tersentuh. Untuk remote rclone yang Anda buat sendiri dan dipakai lewat "Pakai remote rclone yang sudah ada", LELEMOVE hanya berhenti memakainya; remote itu tetap ada di rclone.
- Cabut izin sepenuhnya: https://myaccount.google.com/permissions untuk Google, pengaturan aplikasi akun Microsoft Anda, atau halaman aplikasi terhubung/kata sandi aplikasi di layanan lain.
- Hapus data lokal: hapus folder %LOCALAPPDATA%\pindah-drive dan berkas %APPDATA%\rclone\rclone.conf setelah menghapus aplikasi.

## Layanan pihak ketiga

File yang tersimpan di layanan cloud tunduk pada kebijakan privasi penyedia masing-masing (Google, Microsoft, Dropbox, dan lainnya). LELEMOVE tidak mengendalikan bagaimana penyedia tersebut memproses data di layanannya.

## Perubahan

Kebijakan ini bisa diperbarui bersama versi aplikasi berikutnya. Kebijakan yang berlaku selalu yang tampil di Bantuan & info pada versi yang Anda pakai.
