import { Composition, Folder } from "remotion";
import { Aman } from "./adegan/Aman";
import { Aplikasi } from "./adegan/Aplikasi";
import { Fitur } from "./adegan/Fitur";
import { Layanan } from "./adegan/Layanan";
import { Logo } from "./adegan/Logo";
import { Pembuka } from "./adegan/Pembuka";
import { Penutup } from "./adegan/Penutup";
import { Platform } from "./adegan/Platform";
import linimasa from "./linimasa.json";
import { Promo } from "./Promo";

// Durasi utama mengikuti narasi: src/linimasa.json dibuat alat/buat-suara.mjs
// (jumlah durasi adegan dikurangi tujuh transisi 15 frame).
export const RemotionRoot: React.FC = () => {
  return (
    <>
      <Composition
        id="Promo"
        component={Promo}
        durationInFrames={linimasa.total}
        fps={30}
        width={1920}
        height={1080}
      />
      <Folder name="Adegan">
        <Composition
          id="Pembuka"
          component={Pembuka}
          durationInFrames={120}
          fps={30}
          width={1920}
          height={1080}
          defaultProps={{
            judul: "File kantor tersebar di mana-mana?",
            subjudul: "Memindahkannya satu per satu menghabiskan waktu.",
          }}
        />
        <Composition
          id="Logo"
          component={Logo}
          durationInFrames={150}
          fps={30}
          width={1920}
          height={1080}
          defaultProps={{
            slogan: "Salin file antar cloud dan komputer, ke arah mana pun.",
          }}
        />
        <Composition
          id="Layanan"
          component={Layanan}
          durationInFrames={210}
          fps={30}
          width={1920}
          height={1080}
          defaultProps={{
            label: "layanan penyimpanan",
            keterangan:
              "Google Drive, OneDrive, Dropbox, NAS kantor, hingga S3, semua dari satu jendela.",
            lainnya: "+ 70 layanan lain lewat rclone",
          }}
        />
        <Composition
          id="Aplikasi"
          component={Aplikasi}
          durationInFrames={270}
          fps={30}
          width={1920}
          height={1080}
          defaultProps={{
            kicker: "CARA KERJANYA",
            judul: "Pilih Dari dan Ke. Klik Salin.",
          }}
        />
        <Composition
          id="Aman"
          component={Aman}
          durationInFrames={195}
          fps={30}
          width={1920}
          height={1080}
          defaultProps={{
            judul: "Aman dari awal sampai akhir",
            catatan: "File di sumber tidak diubah atau dihapus.",
          }}
        />
        <Composition
          id="Fitur"
          component={Fitur}
          durationInFrames={240}
          fps={30}
          width={1920}
          height={1080}
          defaultProps={{ judul: "Lebih dari sekadar menyalin" }}
        />
        <Composition
          id="Platform"
          component={Platform}
          durationInFrames={135}
          fps={30}
          width={1920}
          height={1080}
          defaultProps={{
            keterangan: "Gratis · Sumber terbuka · Tanpa hak admin",
          }}
        />
        <Composition
          id="Penutup"
          component={Penutup}
          durationInFrames={225}
          fps={30}
          width={1920}
          height={1080}
          defaultProps={{
            ajakan: "Unduh gratis",
            alamat: "github.com/fauzirpl/lelemove",
            dukung: "Dukung pengembangannya: saweria.co/pemecuteai",
          }}
        />
      </Folder>
    </>
  );
};
