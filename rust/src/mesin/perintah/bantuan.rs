use super::super::dasar::*;
use crate::win;

pub fn tampilkan_bantuan() {
    polos(&format!(
        "LELEMOVE {}: salin file antar Google Drive, OneDrive, SharePoint, Dropbox, NAS, S3, layanan rclone lain, dan folder komputer.",
        crate::VERSI
    ));
    polos("");
    polos("Perintah (pindah <perintah> [opsi]):");
    polos("  cek                         status rclone, akun, folder OneDrive, rute");
    polos("  akun                        daftar akun cloud yang terhubung");
    polos("  pasang [-Ya]                pasang rclone lewat winget");
    polos("  hubungkan -Akun <nama> [-Jenis google|onedrive|sharepoint] [-Label X] [-IzinPenuh] [-Ulang]");
    polos("            [-ClientAplikasi | -ClientId X -ClientSecret Y [-SimpanBawaan]]   login akun (di browser)");
    polos("  lepas -Akun <nama>          lepas akun (file di cloud tidak terhapus)");
    polos("  klien [-ClientId X -ClientSecret Y | -Hapus]   client ID Google aplikasi (lihat/simpan/hapus)");
    polos("  jadwal                      daftar backup terjadwal");
    polos("  jadwal -Tambah -Sumber X -Tujuan Y [-Nama N] [-Jam 22:00] [-Hari sen,rab,jum] [-TanpaVerifikasi] [-Id X]");
    polos("         [-Sertakan ..] [-Lewati ..] [-Sejak ..]   filter khusus jadwal ini");
    polos("  jadwal -Id X -Hapus | -Aktifkan | -Nonaktifkan | -Jalankan");
    polos("  bawaan -Akun <nama>         jadikan client ID akun Google itu client ID aplikasi");
    polos("  situs -Akun <sharepoint> -Cari <kata>              cari situs SharePoint");
    polos("  pustaka -Akun <sharepoint> -SitusId <id>           daftar pustaka dokumen situs");
    polos("  pustaka -Akun <sharepoint> -Pustaka <id> [-SitusNama X -PustakaNama Y]   pakai pustaka");
    polos("  periksa [-Sumber X] [-Tujuan Y]     ukuran sumber vs ruang tujuan");
    polos(&format!("          X/Y: {}  atau  <akun>:<folder>, mis. pd-onedrive:Kerja  pd-gdrive-2:Arsip", win::CONTOH_DATA));
    polos("  uji                         dry run, deteksi file yang akan terlewat");
    polos("  salin [-Latar] [-Ya]        mulai menyalin (bisa dilanjutkan)");
    polos("        [-BatasKecepatan 5M]  batasi kecepatan (5M = 5 MB/dtk, 500K, off); diingat untuk berikutnya");
    polos("  status | berhenti           pantau / hentikan penyalinan latar");
    polos("  verifikasi                  cocokkan isi sumber dan tujuan");
    polos("  pindahkan [-Ya]             verifikasi ulang, lalu hapus dari SUMBER file yang terbukti sama");
    polos(&format!("                              (ke {} / Sampah; file yang tidak cocok tidak disentuh)", win::NAMA_SAMPAH));
    polos("  penyedia                    daftar layanan yang bisa ditambahkan (Dropbox, NAS, S3, ...)");
    polos("  hubungkan -Akun pd-<x> -Jenis <jenis> [-Opsi kunci=nilai ...] [-RahasiaStdin]");
    polos("            akun selain Google/Microsoft; rahasia dikirim sebagai JSON lewat stdin");
    polos("  hubungkan -Akun <nama> -Lanjut -State S -Jawab X   jawab pertanyaan lanjutan (mis. kode 2FA)");
    polos("  impor [-Akun <remote>] [-Label X]   pakai remote rclone yang sudah ada");
    polos("  riwayat [-Jumlah 50]        riwayat salin, verifikasi, pindahkan");
    polos("          [-Csv x.csv | -Laporan x.html]   ekspor semua riwayat");
    polos("          -Id <id> [-Csv x.csv | -Laporan x.html]   laporan per file satu verifikasi");
    polos("  Filter (periksa, uji, salin, verifikasi, pindahkan; diingat untuk berikutnya):");
    polos("    -Sertakan \"pdf,docx\"  hanya jenis file ini   -Lewati \"*.tmp,node_modules/\"  lewati pola ini");
    polos("    -Sejak 2026-01-31     hanya yang diubah sejak tanggal itu   -TanpaFilter  hapus filter");
    polos("  pembaruan [-Unduh | -Jalankan]   periksa rilis terbaru di GitHub; unduh (+cek SHA-256) / pasang");
    polos("  email -Sumber <akun|jalur> -Tujuan <akun|jalur> [-HanyaPeriksa] [-Ya]   migrasi/cadangan email (semua folder)");
    polos("        akun: gmail:nama@gmail.com | outlook:nama@outlook.com | imap://pengguna@server[:port]");
    polos("              (imap+starttls://, imap+polos://); kata sandi lewat -RahasiaStdin {\"Sumber\":..,\"Tujuan\":..}");
    polos("        jalur: folder di komputer. Sebagai tujuan: cadangan .eml (satu berkas per email). Sebagai");
    polos("              sumber: folder cadangan .eml, berkas .mbox (Google Takeout), atau folder mbox Thunderbird");
    polos("        [-Login google]  Gmail lewat login Google (bukan sandi aplikasi)   [-SertakanSampah] [-TanpaArsip]");
    polos("        [-AbaikanSertifikat]  terima sertifikat swatanda   -ClientMicrosoft <id>  client ID Azure (Outlook)");
    polos("  menu                        menu teks (bawaan bila tanpa perintah)");
    polos("");
    polos(&format!("Dukung pengembangan LELEMOVE: {}", crate::SAWERIA));
}
