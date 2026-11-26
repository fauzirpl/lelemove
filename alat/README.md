# Alat pengembang

Skrip yang dipakai saat menyunting README dan membuat rilis. Semuanya Python 3 tanpa pustaka
tambahan, kecuali disebut lain. Jalankan dari akar repositori.

## README dua ejaan

`README.md` berejaan zaman perjuangan (van Ophuijsen) dan **tidak disunting langsung**. Sumbernya
empat bagian di `alat/readme/1.md` sampai `4.md`; blok kodenya diambil dari `README.md` di commit
terakhir (penanda `@@KODE n@@`), supaya perintah dan nama berkas tidak ikut berubah ejaan.

```powershell
python alat\rakit_readme.py . alat\readme --tulis     # tulis README.md, periksa jangkar dan ejaan
python alat\buat_readme_modern.py . --tulis           # tulis README-ejaan-sekarang.md dari README.md
```

Tanpa `--tulis` keduanya hanya memeriksa. Aturannya:

- Ubah prosa di `alat/readme/*.md`, lalu jalankan kedua perintah di atas. Kedua README di-commit bersama.
- Perintah baru di dalam blok kode ditambahkan lewat daftar `TAMBALAN` di `rakit_readme.py`.
- Jangkar lama (`<a name="...">`) dipertahankan: tautan dari aplikasi dan catatan rilis memakainya.
- Label antarmuka (teks miring), kode, jalur, dan URL tetap ejaan sekarang.

## Memeriksa installer publik

```powershell
python alat\periksa_installer.py dist\LELEMOVE-Setup-<versi>.exe
```

Membongkar muatan installer dan memastikan tidak ada client ID/secret Google di dalamnya. Nilai dari
komputer ini (bila ada) hanya dipakai sebagai pembanding dan tidak pernah dicetak. Wajib lolos
sebelum installer diunggah ke Releases.

## Menguji tombol pembaruan

```powershell
powershell -ExecutionPolicy Bypass -File alat\uji-pembaruan.ps1 -Installer dist\LELEMOVE-Setup-<versi>.exe
```

Menjalankan alur *Unduh & pasang* terhadap installer lokal: memasangnya, membuka LELEMOVE, menekan
tombol-tombolnya lewat UI Automation, lalu memeriksa bahwa berkas diganti dan aplikasi dibuka lagi.
Installer itu **menimpa pemasangan LELEMOVE di komputer ini** (akun, pengaturan, jadwal, dan riwayat
tidak disentuh), jadi pasang lagi installer resmi sesudahnya bila perlu. CI menjalankannya dengan
`-TanpaJendela`. Lihat keterangan di kepala skrip.
