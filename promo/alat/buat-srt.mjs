// Membuat subtitle SRT dari naskah.json + src/linimasa.json, mis. untuk LinkedIn yang memutar video
// tanpa suara. Waktu tiap potongan dibagi sebanding panjang teks di dalam jendela narasi adegannya.
// Pemakaian (dari folder promo): node alat/buat-srt.mjs [keluaran.srt]   (bawaan: out/lelemove-promo.srt)
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const AKAR = join(dirname(fileURLToPath(import.meta.url)), "..");
const naskah = JSON.parse(readFileSync(join(AKAR, "naskah.json"), "utf8"));
const lini = JSON.parse(readFileSync(join(AKAR, "src", "linimasa.json"), "utf8"));
const manifest = JSON.parse(readFileSync(join(AKAR, "public", "suara", "manifest.json"), "utf8"));
const keluaran = process.argv[2] ?? join(AKAR, "out", "lelemove-promo.srt");
const MAKS_BARIS = 42;

// Ejaan di layar: nama aplikasi dan angka seperti pada tampilan video.
const tampil = (t) => t.replace(/Lele Move/g, "LELEMOVE").replace(/Dua puluh sembilan/g, "29");

// Pecah per kalimat; kalimat lebih dari dua baris dipecah lagi di koma.
function potong(teks) {
  const kalimat = teks.match(/[^.?!:]+[.?!:]?/g).map((s) => s.trim()).filter(Boolean);
  return kalimat.flatMap((k) => {
    if (k.length <= MAKS_BARIS * 2) return [k];
    const hasil = [];
    let kini = "";
    for (const bagian of k.split(/(?<=,)\s+/)) {
      if (kini && (kini + " " + bagian).length > MAKS_BARIS * 2) {
        hasil.push(kini);
        kini = bagian;
      } else kini = kini ? `${kini} ${bagian}` : bagian;
    }
    return kini ? [...hasil, kini] : hasil;
  });
}

// Bungkus ke paling banyak dua baris: utamakan patah sesudah koma, lalu panjang yang seimbang.
function bungkus(t) {
  if (t.length <= MAKS_BARIS) return t;
  const kata = t.split(" ");
  let terbaik = t;
  let skor = Infinity;
  for (let i = 1; i < kata.length; i++) {
    const a = kata.slice(0, i).join(" ");
    const b = kata.slice(i).join(" ");
    if (a.length > MAKS_BARIS || b.length > MAKS_BARIS) continue;
    const s = Math.abs(a.length - b.length) - (a.endsWith(",") ? 20 : 0);
    if (s < skor) {
      skor = s;
      terbaik = `${a}\n${b}`;
    }
  }
  return terbaik;
}

const waktu = (dtk) => {
  const ms = Math.round(dtk * 1000);
  const p = (n, l = 2) => String(n).padStart(l, "0");
  return `${p(Math.floor(ms / 3600000))}:${p(Math.floor(ms / 60000) % 60)}:${p(Math.floor(ms / 1000) % 60)},${p(ms % 1000, 3)}`;
};

const isyarat = [];
for (const a of lini.adegan) {
  const teks = naskah.adegan.find((n) => n.id === a.id).teks;
  const mulai = a.narasiMulai / lini.fps + (manifest[a.id]?.terukur?.mulai ?? 0);
  const akhir = a.narasiAkhir / lini.fps;
  const potongan = potong(tampil(teks));
  const total = potongan.reduce((s, p) => s + p.length, 0);
  let t = mulai;
  for (const p of potongan) {
    const d = ((akhir - mulai) * p.length) / total;
    isyarat.push({ dari: t, sampai: t + d, teks: bungkus(p) });
    t += d;
  }
}

const srt = isyarat.map((c, i) => `${i + 1}\n${waktu(c.dari)} --> ${waktu(c.sampai)}\n${c.teks}\n`).join("\n");
mkdirSync(dirname(keluaran), { recursive: true });
writeFileSync(keluaran, srt);
console.log(`${isyarat.length} isyarat -> ${keluaran}`);
