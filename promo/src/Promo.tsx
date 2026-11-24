// Video promosi LELEMOVE (1920x1080, 30 fps): delapan adegan bersambung dengan transisi, narasi
// ElevenLabs per adegan, musik latar sintesis sendiri yang merunduk saat narasi, dan bunyi "whoosh"
// di setiap perpindahan adegan. Durasi adegan diambil dari src/linimasa.json (alat/buat-suara.mjs).
import { Audio } from "@remotion/media";
import { linearTiming, TransitionSeries } from "@remotion/transitions";
import { fade } from "@remotion/transitions/fade";
import { slide } from "@remotion/transitions/slide";
import { wipe } from "@remotion/transitions/wipe";
import { interpolate, staticFile, useCurrentFrame, useVideoConfig } from "remotion";
import { Aman } from "./adegan/Aman";
import { Aplikasi } from "./adegan/Aplikasi";
import { Fitur } from "./adegan/Fitur";
import { Layanan } from "./adegan/Layanan";
import { Logo } from "./adegan/Logo";
import { Pembuka } from "./adegan/Pembuka";
import { Penutup } from "./adegan/Penutup";
import { Platform } from "./adegan/Platform";
import linimasa from "./linimasa.json";

const adegan = linimasa.adegan;
const durasi = (id: string) => adegan.find((a) => a.id === id)!.durasi;
const transisi = linearTiming({ durationInFrames: linimasa.transisi });

// Narasi sudah dinormalkan ke -18 dBFS (alat/buat-suara.mjs). Musik 0,3 di sela narasi, turun ke 30%
// selama narasi: turun dalam 8 frame, naik lagi dalam 12 frame.
const MUSIK = 0.3;
const RUNDUK = 0.3;
const titikMusik = [0, ...adegan.flatMap((a) => [a.narasiMulai - 8, a.narasiMulai, a.narasiAkhir, a.narasiAkhir + 12])];
const nilaiMusik = [1, ...adegan.flatMap(() => [1, RUNDUK, RUNDUK, 1])];

export const Promo: React.FC = () => {
  const { fps } = useVideoConfig();
  const frame = useCurrentFrame();
  const runduk = interpolate(frame, titikMusik, nilaiMusik, {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  return (
    <>
      <TransitionSeries name="Adegan">
        <TransitionSeries.Sequence
          name="Pembuka"
          durationInFrames={durasi("pembuka")}
          premountFor={fps}
        >
          <Pembuka
            judul="File kantor tersebar di mana-mana?"
            subjudul="Memindahkannya satu per satu menghabiskan waktu."
          />
        </TransitionSeries.Sequence>
        <TransitionSeries.Transition presentation={fade()} timing={transisi} />
        <TransitionSeries.Sequence
          name="Logo"
          durationInFrames={durasi("logo")}
          premountFor={fps}
        >
          <Logo slogan="Salin file antar cloud dan komputer, ke arah mana pun." />
        </TransitionSeries.Sequence>
        <TransitionSeries.Transition
          presentation={slide({ direction: "from-right" })}
          timing={transisi}
        />
        <TransitionSeries.Sequence
          name="Layanan"
          durationInFrames={durasi("layanan")}
          premountFor={fps}
        >
          <Layanan
            label="layanan penyimpanan"
            keterangan="Google Drive, OneDrive, Dropbox, NAS kantor, hingga S3, semua dari satu jendela."
            lainnya="+ 70 layanan lain lewat rclone"
          />
        </TransitionSeries.Sequence>
        <TransitionSeries.Transition
          presentation={slide({ direction: "from-bottom" })}
          timing={transisi}
        />
        <TransitionSeries.Sequence
          name="Aplikasi"
          durationInFrames={durasi("aplikasi")}
          premountFor={fps}
        >
          <Aplikasi
            kicker="CARA KERJANYA"
            judul="Pilih Dari dan Ke. Klik Salin."
          />
        </TransitionSeries.Sequence>
        <TransitionSeries.Transition
          presentation={wipe({ direction: "from-left" })}
          timing={transisi}
        />
        <TransitionSeries.Sequence
          name="Aman"
          durationInFrames={durasi("aman")}
          premountFor={fps}
        >
          <Aman
            judul="Aman dari awal sampai akhir"
            catatan="File di sumber tidak diubah atau dihapus."
          />
        </TransitionSeries.Sequence>
        <TransitionSeries.Transition
          presentation={slide({ direction: "from-right" })}
          timing={transisi}
        />
        <TransitionSeries.Sequence
          name="Fitur"
          durationInFrames={durasi("fitur")}
          premountFor={fps}
        >
          <Fitur judul="Lebih dari sekadar menyalin" />
        </TransitionSeries.Sequence>
        <TransitionSeries.Transition presentation={fade()} timing={transisi} />
        <TransitionSeries.Sequence
          name="Platform"
          durationInFrames={durasi("platform")}
          premountFor={fps}
        >
          <Platform keterangan="Gratis · Sumber terbuka · Tanpa hak admin" />
        </TransitionSeries.Sequence>
        <TransitionSeries.Transition
          presentation={wipe({ direction: "from-right" })}
          timing={transisi}
        />
        <TransitionSeries.Sequence
          name="Penutup"
          durationInFrames={durasi("penutup")}
          premountFor={fps}
        >
          <Penutup
            ajakan="Unduh gratis"
            alamat="github.com/fauzirpl/lelemove"
            dukung="Dukung pengembangannya: saweria.co/pemecuteai"
          />
        </TransitionSeries.Sequence>
      </TransitionSeries>
      <Audio
        name="Musik latar"
        src={staticFile("musik.wav")}
        premountFor={fps}
        volume={MUSIK * runduk}
      />
      {adegan.map((a) => (
        <Audio
          key={`narasi-${a.id}`}
          name={`Narasi ${a.id}`}
          src={staticFile(`suara/${a.id}.wav`)}
          from={a.narasiMulai}
          premountFor={fps}
        />
      ))}
      {adegan.slice(1).map((a) => (
        <Audio
          key={`whoosh-${a.id}`}
          name={`Whoosh ke ${a.id}`}
          src={staticFile("sfx/whoosh.wav")}
          from={a.mulai - 5}
          premountFor={fps}
          volume={0.25}
        />
      ))}
    </>
  );
};
