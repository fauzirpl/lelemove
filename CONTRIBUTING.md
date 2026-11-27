# Berkontribusi ke LELEMOVE

Terima kasih sudah mau membantu. Laporan masalah, perbaikan, dan pengujian di layanan yang belum
sempat dicoba semuanya berguna.

## Melaporkan masalah

Buka [Issues](https://github.com/fauzirpl/lelemove/issues) dan sertakan:

- versi LELEMOVE (*Bantuan & info > Tentang*) dan sistem operasi;
- layanan sumber dan tujuan (misalnya OneDrive ke Google Drive);
- langkah untuk mengulang masalahnya, dan apa yang Anda harapkan terjadi;
- potongan *Log aktivitas* yang relevan.

**Jangan menempelkan token, kata sandi, client secret, atau isi `rclone.conf`.** Periksa dulu log
yang akan ditempel; alamat email dan nama folder pribadi boleh disamarkan.

Celah keamanan jangan dilaporkan di Issues; lihat [SECURITY.md](SECURITY.md).

## Menyiapkan lingkungan

Windows memakai toolchain Rust **GNU** per pengguna (tanpa Visual Studio):

```powershell
winget install Rustlang.Rustup --custom "--default-host x86_64-pc-windows-gnu --profile minimal"
cd rust
cargo test --release --lib --bin pindah-drive
cargo build --release --bins
```

Kode macOS bisa diperiksa dari Windows dengan
`cargo check --release --target aarch64-apple-darwin --all-targets`. Bagian *Untuk pengembang* di
README menjelaskan susunan kode, cara menguji tanpa menyentuh data asli (`PINDAH_DRIVE_DATA`,
`RCLONE_CONFIG`), dan pratinjau jendela.

## Aturan perubahan

- **Uji dulu.** Uji unit harus lulus di Windows dan macOS; keduanya dijalankan GitHub Actions pada
  setiap pull request. Perubahan pada alur pembaruan diuji dengan `alat\uji-pembaruan.ps1`.
- **Clippy harus bersih.** CI Windows menjalankan `cargo clippy --all-targets -- -D warnings`;
  peringatan apa pun menggagalkan build. Pengecualian yang disengaja ditulis sebagai `#[allow(...)]`
  di tempatnya, dengan komentar alasannya.
- **Jangan pernah meng-commit rahasia**: `client-google.json`, `rclone.conf`, token, atau installer
  di `dist\` (installer pribadi bisa membawa client ID).
- **Fungsi khusus sistem** ditaruh di `win.rs` dan `mac.rs` dengan nama yang sama, supaya kode lain
  cukup memanggil `win::...`.
- **Kode dan komentar berbahasa Indonesia**, mengikuti yang sudah ada. Teks untuk pengguna ditulis
  lugas dan tidak menyalahkan pengguna.
- **Kontrol yang digambar sendiri** diberi nama lewat `gaya::nama_tombol`, supaya terbaca oleh
  pembaca layar dan bisa ditekan oleh alat uji.
- **README tidak disunting langsung.** Sumbernya di `alat/readme/`; lihat [alat/README.md](alat/README.md).
- Jangan memakai nama instansi atau data sungguhan sebagai contoh di kode, dokumen, atau tangkapan layar.

## Lisensi

Dengan mengirim kontribusi, Anda setuju kontribusi itu dirilis dengan [lisensi MIT](LICENSE) yang
sama dengan LELEMOVE.
