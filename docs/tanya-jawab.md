# Tanya jawab

### Apakah file di sumber ikut dihapus atau dipindahkan?

Tidak, selama Anda hanya menyalin. LELEMOVE menyalin (rclone copy) dan mencocokkan (rclone check); keduanya tidak mengubah file di sumber. File di sumber baru dihapus bila Anda sendiri memilih Pindahkan setelah verifikasi (lihat pertanyaan berikutnya).

### Bagaimana cara memindahkan, bukan sekadar menyalin?

Salin dulu, lalu tunggu verifikasi selesai. Di bawah hasil verifikasi muncul tautan "Pindahkan: hapus ... file ini dari sumber". Setelah dua kali konfirmasi (yang kedua dengan mengetik HAPUS), LELEMOVE mencocokkan ulang setiap file dan hanya menghapus dari sumber yang terbukti sama di tujuan. File yang belum ada, berbeda, atau gagal dibaca tidak disentuh. Yang dihapus masuk ke Recycle Bin Windows, Sampah Google Drive, atau recycle bin OneDrive/SharePoint, jadi masih bisa dipulihkan. Folder di drive jaringan atau flashdisk (tanpa Recycle Bin) tidak didukung.

### Layanan apa saja yang bisa dipakai?

Klik Tambah akun > Layanan lain untuk daftar lengkapnya:
- Layanan cloud: Google Drive, OneDrive, SharePoint, Dropbox, Box, pCloud, Yandex Disk, HiDrive, Zoho WorkDrive, MEGA, Koofr, iCloud Drive, Proton Drive, Jottacloud.
- Server kantor & NAS: Nextcloud, ownCloud, WebDAV, folder bersama SMB, SFTP, FTP, Seafile.
- Object storage: Amazon S3, Cloudflare R2, Wasabi, Backblaze B2, Azure Blob, MinIO, DigitalOcean Spaces, dan S3 kompatibel lain.

Layanan yang tidak ada di daftar tetap bisa dipakai bila didukung rclone (lebih dari 70 layanan): buat dulu lewat "Buat lewat rclone config", lalu "Pakai remote rclone yang sudah ada". Layanan bertanda beta bergantung pada dukungan rclone yang masih berkembang; coba dengan folder kecil dulu.

### Kenapa Pindahkan tidak tersedia untuk NAS, SFTP, FTP, atau S3?

Layanan itu tidak punya tempat sampah, sehingga file yang dihapus langsung hilang permanen. LELEMOVE hanya menghapus bila file itu masih bisa dipulihkan (Recycle Bin, Sampah Google Drive, recycle bin OneDrive, file terhapus Dropbox, dan sejenisnya). Di layanan tanpa tempat sampah, hapus sendiri bila sudah yakin.

### Bisakah hanya menyalin jenis file tertentu?

Bisa. Klik "Filter..." di bawah panel Dari. Anda bisa memilih hanya ekstensi tertentu (misalnya pdf, docx, xlsx), melewati pola tertentu (misalnya *.tmp atau folder Arsip Lama/), atau hanya file yang diubah sejak tanggal tertentu. Filter yang sama dipakai saat periksa, uji coba, salin, verifikasi, dan pindahkan, jadi file yang sengaja tidak disalin tidak dilaporkan "belum ada di tujuan". Jadwal backup menyimpan filternya sendiri.

### Bisakah saya mendapat bukti bahwa semua file sudah tercadangkan?

Bisa. Buka Riwayat & laporan di daftar kiri. Setiap penyalinan, verifikasi, dan pemindahan tercatat di sana, termasuk yang dijalankan jadwal. Untuk satu verifikasi, "Laporan per file" membuka laporan siap cetak berisi status setiap file (simpan sebagai PDF dengan Ctrl+P di browser), dan "CSV per file" menyimpannya untuk Excel. "Ekspor CSV" dan "Laporan HTML" merangkum seluruh riwayat yang tampil.

### Apakah file saya lewat server LELEMOVE?

Tidak ada server LELEMOVE. Data mengalir langsung antara komputer Anda dan layanan cloud (Google atau Microsoft) lewat rclone yang berjalan di komputer Anda. Antar akun yang berbeda, data diunduh dan diunggah sekaligus dalam potongan di memori, tanpa menyimpan berkas utuh ke disk.

### Kenapa kuota internet terpakai dua kali?

Saat menyalin dari satu cloud ke cloud lain (misalnya OneDrive ke Google Drive), setiap byte diunduh ke komputer Anda lalu diunggah lagi. Jadi menyalin 10 GB memakai sekitar 10 GB unduhan dan 10 GB unggahan. Komputer juga harus tetap menyala selama menyalin.

### Apa yang terjadi bila internet putus di tengah penyalinan?

rclone mencoba ulang sendiri untuk gangguan singkat. Bila putusnya lama, LELEMOVE menunggu (30 detik sampai 10 menit, makin lama makin jarang), menguji sambungan ke Google/Microsoft, lalu melanjutkan otomatis begitu tersambung. File yang sudah sampai tidak diulang. Pengaturan ini bisa dimatikan di Pengaturan > Penyalinan.

### Kenapa akun Google saya tertulis "login kedaluwarsa"?

Selama project Google Cloud pemilik client ID masih berstatus "Testing", Google mencabut login setiap 7 hari. Klik Login ulang pada akun itu. Agar tidak terulang, pemilik project perlu klik "Publish app" di Google Cloud Console.

### Apa itu client ID Google, dan kenapa perlu client ID sendiri?

Client ID adalah identitas aplikasi saat meminta izin ke Google. Tanpa client ID sendiri, rclone memakai client ID bersama yang dipakai pengguna rclone di seluruh dunia, sehingga kuotanya sering habis dan penyalinan macet. Client ID sendiri gratis; pasang lewat Pengaturan > Client ID Google.

### Penyalinan berhenti di persentase tertentu dan tidak bergerak. Kenapa?

Biasanya layanan cloud sedang membatasi permintaan (rate limit), paling sering karena akun Google masih memakai client ID bersama. LELEMOVE menandainya dengan warna kuning. Pasang client ID sendiri di Pengaturan, login ulang akun Google itu, lalu klik Lanjutkan.

### Kenapa penyalinan ke Google Drive berhenti sendiri setelah ratusan GB?

Google membatasi unggahan 750 GB per akun per hari. Penyalinan berhenti dan bisa dilanjutkan besok; file yang sudah sampai dilewati.

### Kenapa verifikasi hanya membandingkan nama dan ukuran?

Verifikasi memakai checksum (sidik jari isi file) bila kedua layanan punya jenis checksum yang sama, misalnya komputer dengan Google Drive (MD5) atau komputer dengan OneDrive (QuickXorHash). OneDrive dan Google Drive tidak punya jenis checksum yang sama, jadi di antara keduanya yang dibandingkan nama dan ukuran.

### Kenapa file Office yang disalin ke SharePoint ukurannya berbeda?

SharePoint menambahkan metadata ke file .docx, .xlsx, dan .pptx saat diunggah. Isinya sama; LELEMOVE menghitungnya cocok dan menandainya dengan catatan.

### Ada file "hanya online" di folder OneDrive. Apakah ikut tersalin?

Ya. Saat disalin, aplikasi OneDrive mengunduhnya dulu ke disk. Periksa menghitung apakah ruang disk cukup. Bila tidak cukup, salin per subfolder, atau hubungkan akun OneDrive lewat internet dan pakai sebagai sumber.

### Apa yang tidak ikut tersalin?

Notebook OneNote, Personal Vault OneDrive, riwayat versi, izin berbagi, dan metadata kolom SharePoint. Dokumen Google (Docs, Sheets, Slides) yang disalin ke luar Google Drive diubah menjadi .docx, .xlsx, dan .pptx.

### Bolehkah aplikasi ditutup saat menyalin?

Boleh. Penyalinan berjalan di latar belakang. Saat aplikasi dibuka lagi, progresnya muncul kembali, dan Windows menampilkan notifikasi saat penyalinan selesai atau berhenti.

### Bisakah LELEMOVE mencadangkan sendiri setiap hari?

Bisa. Buka folder sumber di panel Dari dan tujuannya di panel Ke, lalu klik Jadwal backup > Jadwal baru dan pilih jam serta harinya. Jadwal dijalankan oleh Task Scheduler Windows selama Anda masuk ke Windows, walau jendela LELEMOVE tidak dibuka. Bila komputer mati atau tidur pada jamnya, backup dijalankan begitu komputer menyala lagi. Hanya file baru atau yang berubah yang disalin, dan hasilnya dikabarkan lewat notifikasi Windows.

### Kenapa backup terjadwal saya "dilewati"?

Pada jam itu ada penyalinan lain yang masih berjalan, jadi jadwal tidak memulai penyalinan kedua. Jadwal dicoba lagi pada waktu berikutnya; klik Jalankan sekarang di Jadwal backup untuk menjalankannya segera.

### Bagaimana melanjutkan penyalinan yang berhenti?

Klik Lanjutkan (tombol Mulai salin). rclone membandingkan isi tujuan dan hanya menyalin yang belum ada atau berbeda.

### Bagaimana mencabut akses LELEMOVE ke akun saya?

Klik ⋯ > Lepas akun ini untuk menghapus sambungan di komputer ini. Untuk mencabut izin sepenuhnya, hapus akses aplikasi di https://myaccount.google.com/permissions (Google) atau di pengaturan aplikasi akun Microsoft Anda.

### Bagaimana menghapus LELEMOVE sepenuhnya?

Hapus lewat Settings > Apps > LELEMOVE. Tugas Windows untuk jadwal backup ikut dihapus. Koneksi akun (%APPDATA%\rclone\rclone.conf) dan pengaturan, daftar jadwal, riwayat, serta log (%LOCALAPPDATA%\pindah-drive) sengaja tidak ikut terhapus; hapus kedua folder itu sendiri bila tidak diperlukan lagi.
