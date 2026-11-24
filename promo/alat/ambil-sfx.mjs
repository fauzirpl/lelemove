// Mengunduh efek suara yang dipakai video dari remotion.media ke public/sfx/ (bila belum ada).
// Berkasnya tidak disimpan di repositori karena lisensinya tidak dinyatakan untuk disebarkan ulang.
// Pemakaian (dari folder promo): node alat/ambil-sfx.mjs
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const folder = join(dirname(fileURLToPath(import.meta.url)), "..", "public", "sfx");
mkdirSync(folder, { recursive: true });

for (const nama of ["whoosh.wav", "ding.wav", "mouse-click.wav"]) {
  const tujuan = join(folder, nama);
  if (existsSync(tujuan)) {
    console.log(`${nama}: sudah ada`);
    continue;
  }
  for (let coba = 1; ; coba++) {
    try {
      const r = await fetch(`https://remotion.media/${nama}`);
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      writeFileSync(tujuan, Buffer.from(await r.arrayBuffer()));
      console.log(`${nama}: diunduh`);
      break;
    } catch (e) {
      if (coba >= 4) throw e;
      await new Promise((s) => setTimeout(s, 2000 * coba));
    }
  }
}
