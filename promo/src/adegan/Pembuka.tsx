// Adegan 1: masalahnya. Petak layanan berhamburan masuk dari segala arah di sekitar pertanyaan.
import type React from "react";
import {
  AbsoluteFill,
  Easing,
  Interactive,
  interpolate,
  useCurrentFrame,
  useVideoConfig,
  type InteractivitySchema,
} from "remotion";
import "../tema";
import { LAYANAN } from "../tema";
import { Ubin } from "../Ubin";

// Posisi akhir petak (x, y, putaran) dan arah datangnya: satu templat untuk semua petak.
const SEBARAN = [
  { i: 0, x: 170, y: 150, r: -12, dx: -500, dy: -300 },
  { i: 1, x: 1560, y: 120, r: 10, dx: 500, dy: -300 },
  { i: 14, x: 1650, y: 720, r: -8, dx: 500, dy: 300 },
  { i: 3, x: 190, y: 740, r: 9, dx: -500, dy: 300 },
  { i: 17, x: 860, y: 70, r: 6, dx: 0, dy: -400 },
  { i: 21, x: 1180, y: 840, r: -6, dx: 200, dy: 400 },
  { i: 2, x: 600, y: 860, r: 11, dx: -200, dy: 400 },
];

const PembukaInner: React.FC<{ judul: string; subjudul: string }> = ({
  judul,
  subjudul,
}) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();

  return (
    <>
      <AbsoluteFill
        name="Latar kolam"
        style={{
          background:
            "radial-gradient(circle at 50% 45%, #0E5A52 0%, #06302B 78%)",
        }}
      />
      {SEBARAN.map((s, n) => {
        const mulai = n * 4;
        const masuk = interpolate(frame, [mulai, mulai + 0.8 * fps], [0, 1], {
          extrapolateLeft: "clamp",
          extrapolateRight: "clamp",
          easing: Easing.bezier(0.16, 1, 0.3, 1),
        });
        const apung = Math.sin((frame + n * 17) / 18) * 10;
        return (
          <div
            key={s.i}
            style={{
              position: "absolute",
              left: s.x,
              top: s.y,
              opacity: masuk,
              translate: `${(1 - masuk) * s.dx}px ${(1 - masuk) * s.dy + apung}px`,
              rotate: `${s.r + (1 - masuk) * s.r * 3}deg`,
            }}
          >
            <Ubin l={LAYANAN[s.i]} ukuran={150} />
          </div>
        );
      })}
      <Interactive.Div
        name="Judul"
        style={{
          position: "absolute",
          left: 300,
          right: 300,
          top: 360,
          textAlign: "center",
          fontFamily: "Plus Jakarta Sans",
          fontWeight: 800,
          fontSize: 112,
          lineHeight: 1.08,
          letterSpacing: -2,
          color: "#F6ECD0",
          opacity: interpolate(frame, [8, 26], [0, 1], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
          }),
          translate: interpolate(frame, [8, 34], ["0px 40px", "0px 0px"], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
            easing: Easing.bezier(0.16, 1, 0.3, 1),
          }),
        }}
      >
        {judul}
      </Interactive.Div>
      <Interactive.Div
        name="Subjudul"
        style={{
          position: "absolute",
          left: 240,
          right: 240,
          top: 640,
          textAlign: "center",
          fontFamily: "Plus Jakarta Sans",
          fontWeight: 500,
          fontSize: 50,
          color: "#5FD3B4",
          opacity: interpolate(frame, [36, 54], [0, 1], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
          }),
        }}
      >
        {subjudul}
      </Interactive.Div>
    </>
  );
};

const pembukaSchema = {
  judul: {
    type: "string",
    default: "File kantor tersebar di mana-mana?",
    description: "Judul",
  },
  subjudul: {
    type: "string",
    default: "Memindahkannya satu per satu menghabiskan waktu.",
    description: "Subjudul",
  },
} as const satisfies InteractivitySchema;

export const Pembuka = Interactive.withSchema({
  Component: PembukaInner,
  componentName: "Pembuka",
  schema: pembukaSchema,
  wrapInSequence: true,
  layout: "absolute-fill",
});
