# Video promosi LELEMOVE

Sumber video perkenalan yang tampil di README utama: 54 detik, 1920x1080, 30 fps, dibuat dengan
[Remotion](https://www.remotion.dev). Delapan adegan (`src/adegan/`) disusun di `src/Promo.tsx`
dengan transisi, narasi bahasa Indonesia, musik latar, dan efek suara.

## Membuat ulang

Dari folder `promo`:

```powershell
npm i --loglevel=error
node alat/ambil-sfx.mjs                                      # efek suara dari remotion.media
node alat/buat-suara.mjs                                     # narasi + src/linimasa.json
python alat/buat_musik.py src/linimasa.json public/musik.wav # musik latar (butuh numpy)
npx remotion render Promo out/lelemove-promo.mp4
```

Pratinjau interaktif: `npm run dev` (Remotion Studio).

## Cara kerjanya

1. **Naskah** ada di `naskah.json`: satu kalimat narasi per adegan, suara ElevenLabs, model, dan
   pengaturan suara. `dasar` adalah durasi minimum adegan (frame).
2. **`alat/buat-suara.mjs`** membuat satu klip per adegan lewat ElevenLabs (`with-timestamps`) ke
   `public/suara/<adegan>.mp3`. Klip yang teks, suara, atau kalimat tetangganya tidak berubah tidak
   dibuat ulang (sidiknya di `public/suara/manifest.json`), jadi kunci `ELEVENLABS_API_KEY` hanya
   diperlukan bila naskah diubah. Skrip lalu mengukur akhir bicara dari suaranya sendiri (ambang
   -30 dBFS), menormalkan narasi ke -18 dBFS (`public/suara/<adegan>.wav`), dan menulis
   `src/linimasa.json`: setiap adegan sepanjang narasinya, minimal sepanjang `dasar`.
3. **`alat/buat_musik.py`** mensintesis musik latar dari nol (D mayor, 96 BPM) dengan titik-titik
   dari linimasa: dentum saat logo muncul, drum dari adegan Layanan sampai sebelum Penutup.
4. **`src/Promo.tsx`** membaca linimasa yang sama untuk durasi adegan, posisi narasi, bunyi whoosh,
   dan volume musik yang merunduk selama narasi.

## Aset dan lisensi

- Narasi: ElevenLabs, suara "Satria" (`public/suara/*.mp3`), dibuat dengan paket berbayar.
- Musik latar: disintesis sendiri oleh `alat/buat_musik.py`, tanpa sampel.
- Huruf: Plus Jakarta Sans (`public/fonts/`), lisensi SIL Open Font License (`public/fonts/OFL.txt`).
- Efek suara: diunduh dari remotion.media oleh `alat/ambil-sfx.mjs`, tidak disimpan di repositori.
- Remotion: untuk sebagian organisasi diperlukan lisensi perusahaan, lihat
  [ketentuannya](https://github.com/remotion-dev/remotion/blob/main/LICENSE.md).
