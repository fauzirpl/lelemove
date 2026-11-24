// Adegan 5: aman. Tiga langkah (Periksa, Salin, Verifikasi) dengan centang yang tergambar.
import { Audio } from "@remotion/media";
import type React from "react";
import {
  AbsoluteFill,
  Easing,
  Interactive,
  interpolate,
  staticFile,
  useCurrentFrame,
  useVideoConfig,
  type InteractivitySchema,
} from "remotion";
import "../tema";

const LANGKAH = [
  {
    n: "1",
    judul: "Periksa",
    isi: "Ruang tujuan dan uji coba dicek sebelum menyalin.",
  },
  {
    n: "2",
    judul: "Salin",
    isi: "Berjalan di latar, lanjut sendiri saat internet putus.",
  },
  {
    n: "3",
    judul: "Verifikasi",
    isi: "Setiap file dicocokkan, ada laporannya.",
  },
];

const AmanInner: React.FC<{ judul: string; catatan: string }> = ({
  judul,
  catatan,
}) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();

  return (
    <>
      <AbsoluteFill name="Latar krem" style={{ backgroundColor: "#F5F1E6" }} />
      <Interactive.Div
        name="Judul"
        style={{
          position: "absolute",
          left: 120,
          right: 120,
          top: 120,
          textAlign: "center",
          fontFamily: "Plus Jakarta Sans",
          fontWeight: 800,
          fontSize: 96,
          letterSpacing: -2,
          color: "#1F2328",
          opacity: interpolate(frame, [4, 20], [0, 1], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
          }),
          translate: interpolate(frame, [4, 28], ["0px 30px", "0px 0px"], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
            easing: Easing.bezier(0.16, 1, 0.3, 1),
          }),
        }}
      >
        {judul}
      </Interactive.Div>
      {LANGKAH.map((l, i) => {
        const mulai = 18 + i * 28;
        const garis = interpolate(frame, [mulai + 14, mulai + 30], [1, 0], {
          extrapolateLeft: "clamp",
          extrapolateRight: "clamp",
          easing: Easing.bezier(0.65, 0, 0.35, 1),
        });
        return (
          <div
            key={l.n}
            style={{
              position: "absolute",
              left: 150 + i * 560,
              top: 330,
              width: 500,
              height: 440,
              borderRadius: 32,
              backgroundColor: "#FFFFFF",
              boxShadow: "0 24px 60px rgba(14, 90, 82, 0.14)",
              padding: 48,
              boxSizing: "border-box",
              fontFamily: "Plus Jakarta Sans",
              opacity: interpolate(frame, [mulai, mulai + 12], [0, 1], {
                extrapolateLeft: "clamp",
                extrapolateRight: "clamp",
              }),
              translate: interpolate(
                frame,
                [mulai, mulai + 22],
                ["0px 60px", "0px 0px"],
                {
                  extrapolateLeft: "clamp",
                  extrapolateRight: "clamp",
                  easing: Easing.bezier(0.16, 1, 0.3, 1),
                },
              ),
            }}
          >
            <svg width={110} height={110} viewBox="0 0 110 110">
              <circle cx={55} cy={55} r={52} fill="#0E5A52" />
              <path
                d="M32 57 L48 73 L79 40"
                fill="none"
                stroke="#5FD3B4"
                strokeWidth={10}
                strokeLinecap="round"
                strokeLinejoin="round"
                pathLength={1}
                strokeDasharray={1}
                strokeDashoffset={garis}
              />
            </svg>
            <div
              style={{
                marginTop: 34,
                fontSize: 30,
                fontWeight: 700,
                color: "#0E5A52",
                letterSpacing: 3,
              }}
            >
              LANGKAH {l.n}
            </div>
            <div
              style={{
                marginTop: 6,
                fontSize: 64,
                fontWeight: 800,
                color: "#1F2328",
                letterSpacing: -1,
              }}
            >
              {l.judul}
            </div>
            <div
              style={{
                marginTop: 14,
                fontSize: 32,
                fontWeight: 500,
                lineHeight: 1.35,
                color: "#4A535D",
              }}
            >
              {l.isi}
            </div>
          </div>
        );
      })}
      <Interactive.Div
        name="Catatan"
        style={{
          position: "absolute",
          left: 120,
          right: 120,
          top: 850,
          textAlign: "center",
          fontFamily: "Plus Jakarta Sans",
          fontWeight: 700,
          fontSize: 50,
          color: "#0E5A52",
          opacity: interpolate(frame, [120, 138], [0, 1], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
          }),
        }}
      >
        {catatan}
      </Interactive.Div>
      <Audio
        name="Centang 1"
        src={staticFile("sfx/ding.wav")}
        from={32}
        premountFor={fps}
        volume={0.18}
      />
      <Audio
        name="Centang 2"
        src={staticFile("sfx/ding.wav")}
        from={60}
        premountFor={fps}
        volume={0.18}
      />
      <Audio
        name="Centang 3"
        src={staticFile("sfx/ding.wav")}
        from={88}
        premountFor={fps}
        volume={0.18}
      />
    </>
  );
};

const amanSchema = {
  judul: {
    type: "string",
    default: "Aman dari awal sampai akhir",
    description: "Judul",
  },
  catatan: {
    type: "string",
    default: "File di sumber tidak diubah atau dihapus.",
    description: "Catatan",
  },
} as const satisfies InteractivitySchema;

export const Aman = Interactive.withSchema({
  Component: AmanInner,
  componentName: "Aman",
  schema: amanSchema,
  wrapInSequence: true,
  layout: "absolute-fill",
});
