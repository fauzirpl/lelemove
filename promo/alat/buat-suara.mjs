// Membuat narasi video promosi lewat ElevenLabs, satu klip per adegan, lalu menyusun linimasa
// (durasi adegan mengikuti panjang narasi) di src/linimasa.json.
//
// Pemakaian (dari folder promo):  node alat/buat-suara.mjs [--paksa]
// Kunci API dibaca dari variabel lingkungan ELEVENLABS_API_KEY dan tidak pernah dicetak.
// Klip yang teks/suara/pengaturannya tidak berubah tidak dibuat ulang (hemat kuota karakter).
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const AKAR = join(dirname(fileURLToPath(import.meta.url)), "..");
const FPS = 30;
const TRANSISI = 15; // panjang setiap transisi di Promo.tsx
const MASUK_PERTAMA = 12; // narasi adegan pertama mulai setelah 0,4 dtk
const MASUK = TRANSISI + 4; // adegan lain: tunggu transisi selesai
const JEDA = 10; // diam sesudah narasi sebelum transisi berikutnya
const EKOR = 45; // adegan terakhir: 1,5 dtk sesudah narasi (musik memudar)

const naskah = JSON.parse(readFileSync(join(AKAR, "naskah.json"), "utf8"));
const folder = join(AKAR, "public", "suara");
mkdirSync(folder, { recursive: true });
const jalurManifest = join(folder, "manifest.json");
const manifest = existsSync(jalurManifest) ? JSON.parse(readFileSync(jalurManifest, "utf8")) : {};
const paksa = process.argv.includes("--paksa");

// Kunci hanya diperlukan bila ada klip yang harus dibuat; klip tersimpan cukup diukur & dinormalkan.
const kunci = process.env.ELEVENLABS_API_KEY;

async function tts(teks, sebelum, sesudah) {
  if (!kunci) {
    console.error("ELEVENLABS_API_KEY belum diatur, padahal ada klip narasi yang harus dibuat.");
    process.exit(1);
  }
  const url = `https://api.elevenlabs.io/v1/text-to-speech/${naskah.suara}/with-timestamps?output_format=mp3_44100_128`;
  const badan = JSON.stringify({
    text: teks,
    model_id: naskah.model,
    voice_settings: naskah.pengaturan,
    // Konteks kalimat tetangga agar intonasi antar-klip menyambung.
    previous_text: sebelum || undefined,
    next_text: sesudah || undefined,
  });
  for (let coba = 1; ; coba++) {
    try {
      const r = await fetch(url, {
        method: "POST",
        headers: { "xi-api-key": kunci, "Content-Type": "application/json" },
        body: badan,
      });
      if (!r.ok) {
        const pesan = (await r.text()).slice(0, 300);
        throw Object.assign(new Error(`HTTP ${r.status}: ${pesan}`), { tetap: r.status < 500 && r.status !== 429 });
      }
      return await r.json();
    } catch (e) {
      if (e.tetap || coba >= 4) throw e;
      console.log(`  gagal (${e.message.slice(0, 80)}), coba lagi...`);
      await new Promise((s) => setTimeout(s, 3000 * coba));
    }
  }
}

// Waktu bicara sebenarnya: alignment ElevenLabs menghitung tanda baca terakhir (+-1 dtk lebih panjang),
// jadi klip didekode ke PCM 16 kHz lewat ffmpeg bawaan Remotion, lalu dicari jendela 10 ms terakhir
// yang lebih keras dari -30 dBFS (dengung ruang di ekor klip ElevenLabs sekitar -42 dBFS; -30 dBFS
// cocok dengan akhir kata hasil transkripsi balik), ditambah 0,1 dtk cadangan.
function ukurBicara(berkas) {
  const r = spawnSync(
    "npx",
    ["remotion", "ffmpeg", "-v", "error", "-i", `"${berkas}"`, "-f", "wav", "-acodec", "pcm_s16le", "-ac", "1", "-ar", "16000", "-"],
    { cwd: AKAR, shell: true, maxBuffer: 64 * 1024 * 1024 },
  );
  if (r.status !== 0) throw new Error(`ffmpeg gagal untuk ${berkas}: ${r.stderr}`);
  // Lewati header WAV: cari potongan "data" (ukurannya di header pipa bisa tidak terisi).
  const buf = r.stdout;
  const posData = buf.indexOf("data", 12, "latin1");
  if (posData < 0) throw new Error(`keluaran WAV tanpa potongan data: ${berkas}`);
  const isi = buf.subarray(posData + 8);
  const salin = Buffer.from(isi.subarray(0, isi.length & ~1)); // sejajarkan 2 bita
  const pcm = new Int16Array(salin.buffer, salin.byteOffset, salin.length >> 1);
  const JENDELA = 160;
  const AMBANG = 32768 * 10 ** (-30 / 20);
  let awal = -1;
  let akhir = 0;
  for (let i = 0; i + JENDELA <= pcm.length; i += JENDELA) {
    let jumlah = 0;
    for (let j = i; j < i + JENDELA; j++) jumlah += pcm[j] * pcm[j];
    if (Math.sqrt(jumlah / JENDELA) > AMBANG) {
      if (awal < 0) awal = i;
      akhir = i + JENDELA;
    }
  }
  const panjang = pcm.length / 16000;
  return { mulai: Math.max(0, awal) / 16000, akhir: Math.min(panjang, akhir / 16000 + 0.1), panjang };
}

// Narasi dinormalkan ke WAV 44,1 kHz mono: RMS bagian bicara (jendela di atas -30 dBFS) dibawa ke
// -18 dBFS, dengan puncak dibatasi -1 dBFS, supaya suara selalu di atas musik dan rata antar-adegan.
function normalkan(mp3, wav) {
  const r = spawnSync(
    "npx",
    ["remotion", "ffmpeg", "-v", "error", "-i", `"${mp3}"`, "-f", "wav", "-acodec", "pcm_s16le", "-ac", "1", "-ar", "44100", "-"],
    { cwd: AKAR, shell: true, maxBuffer: 64 * 1024 * 1024 },
  );
  if (r.status !== 0) throw new Error(`ffmpeg gagal untuk ${mp3}: ${r.stderr}`);
  const posData = r.stdout.indexOf("data", 12, "latin1");
  const isi = Buffer.from(r.stdout.subarray(posData + 8, posData + 8 + ((r.stdout.length - posData - 8) & ~1)));
  const pcm = new Int16Array(isi.buffer, isi.byteOffset, isi.length >> 1);
  const JENDELA = 441;
  const ambang = 32768 * 10 ** (-30 / 20);
  let jumlah = 0;
  let n = 0;
  let puncak = 1;
  for (let i = 0; i + JENDELA <= pcm.length; i += JENDELA) {
    let j = 0;
    for (let k = i; k < i + JENDELA; k++) j += pcm[k] * pcm[k];
    if (Math.sqrt(j / JENDELA) > ambang) {
      jumlah += j;
      n += JENDELA;
    }
  }
  for (const v of pcm) puncak = Math.max(puncak, Math.abs(v));
  const rms = Math.sqrt(jumlah / Math.max(1, n));
  const gain = Math.min((32768 * 10 ** (-18 / 20)) / rms, (32768 * 10 ** (-1 / 20)) / puncak);
  const keluar = Buffer.alloc(44 + pcm.length * 2);
  keluar.write("RIFF", 0, "latin1");
  keluar.writeUInt32LE(36 + pcm.length * 2, 4);
  keluar.write("WAVEfmt ", 8, "latin1");
  keluar.writeUInt32LE(16, 16);
  keluar.writeUInt16LE(1, 20); // PCM
  keluar.writeUInt16LE(1, 22); // mono
  keluar.writeUInt32LE(44100, 24);
  keluar.writeUInt32LE(44100 * 2, 28);
  keluar.writeUInt16LE(2, 32);
  keluar.writeUInt16LE(16, 34);
  keluar.write("data", 36, "latin1");
  keluar.writeUInt32LE(pcm.length * 2, 40);
  for (let i = 0; i < pcm.length; i++) {
    keluar.writeInt16LE(Math.max(-32768, Math.min(32767, Math.round(pcm[i] * gain))), 44 + i * 2);
  }
  writeFileSync(wav, keluar);
  return 20 * Math.log10(gain);
}

const daftar = naskah.adegan;
let dipakai = 0;
for (let i = 0; i < daftar.length; i++) {
  const a = daftar[i];
  const sidik = createHash("sha256")
    .update(JSON.stringify([a.teks, naskah.suara, naskah.model, naskah.pengaturan, daftar[i - 1]?.teks, daftar[i + 1]?.teks]))
    .digest("hex")
    .slice(0, 16);
  const berkas = join(folder, `${a.id}.mp3`);
  if (!paksa && manifest[a.id]?.sidik === sidik && existsSync(berkas)) {
    console.log(`${a.id}: tetap (${manifest[a.id].bicara.toFixed(2)} dtk)`);
    continue;
  }
  const hasil = await tts(a.teks, daftar[i - 1]?.teks, daftar[i + 1]?.teks);
  writeFileSync(berkas, Buffer.from(hasil.audio_base64, "base64"));
  const akhir = hasil.alignment?.character_end_times_seconds ?? [];
  const awal = hasil.alignment?.character_start_times_seconds ?? [];
  const bicara = akhir.length ? akhir[akhir.length - 1] : 0;
  manifest[a.id] = { sidik, bicara, mulaiSuara: awal[0] ?? 0, karakter: a.teks.length };
  dipakai += a.teks.length;
  console.log(`${a.id}: dibuat, ${bicara.toFixed(2)} dtk, ${a.teks.length} karakter`);
}
for (const a of daftar) {
  const u = ukurBicara(join(folder, `${a.id}.mp3`));
  manifest[a.id].terukur = u;
  const penguat = normalkan(join(folder, `${a.id}.mp3`), join(folder, `${a.id}.wav`));
  console.log(
    `  ${a.id.padEnd(9)} suara ${u.mulai.toFixed(2)}-${u.akhir.toFixed(2)} dtk (alignment ${manifest[a.id].bicara.toFixed(2)}, berkas ${u.panjang.toFixed(2)}), penguat ${penguat >= 0 ? "+" : ""}${penguat.toFixed(1)} dB`,
  );
}
writeFileSync(jalurManifest, JSON.stringify(manifest, null, 2) + "\n");

// Linimasa: setiap adegan sepanjang max(durasi dasar, masuk + narasi + jeda + transisi berikutnya).
const adegan = [];
let mulai = 0;
daftar.forEach((a, i) => {
  const terakhir = i === daftar.length - 1;
  const masuk = i === 0 ? MASUK_PERTAMA : MASUK;
  const bicara = Math.ceil(manifest[a.id].terukur.akhir * FPS);
  const durasi = Math.max(a.dasar, masuk + bicara + (terakhir ? EKOR : JEDA + TRANSISI));
  adegan.push({ id: a.id, mulai, durasi, narasiMulai: mulai + masuk, narasiAkhir: mulai + masuk + bicara });
  mulai += durasi - (terakhir ? 0 : TRANSISI);
});
const total = adegan[adegan.length - 1].mulai + adegan[adegan.length - 1].durasi;
const linimasa = { fps: FPS, transisi: TRANSISI, total, adegan };
writeFileSync(join(AKAR, "src", "linimasa.json"), JSON.stringify(linimasa, null, 2) + "\n");

console.log(`\nkarakter terpakai kali ini: ${dipakai}`);
console.log(`total: ${total} frame (${(total / FPS).toFixed(1)} dtk)`);
for (const a of adegan) {
  console.log(`  ${a.id.padEnd(9)} mulai ${String(a.mulai).padStart(4)}  durasi ${String(a.durasi).padStart(3)}  narasi ${a.narasiMulai}-${a.narasiAkhir}`);
}
