// Adegan 4: aplikasinya. Tangkapan layar asli (data demo) masuk; sorotan panel Dari lalu Ke,
// tombol Salin ditekan, dan bilah progres menyalin terisi sampai selesai.
import { Audio } from "@remotion/media";
import type React from "react";
import {
  AbsoluteFill,
  CanvasImage,
  Easing,
  Interactive,
  interpolate,
  staticFile,
  useCurrentFrame,
  useVideoConfig,
  type InteractivitySchema,
} from "remotion";
import "../tema";

// Tangkapan layar 1225 x 900 ditampilkan selebar 1110 px (skala 0,906).
const S = 1110 / 1225;
const KIRI = 700;
const ATAS = 150;

const AplikasiInner: React.FC<{ kicker: string; judul: string }> = ({
  kicker,
  judul,
}) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const progres = interpolate(frame, [172, 252], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
    easing: Easing.bezier(0.45, 0, 0.25, 1),
  });
  const selesai = frame >= 252;
  const sorot = (mulai: number, akhir: number) =>
    interpolate(frame, [mulai, mulai + 8, akhir - 8, akhir], [0, 1, 1, 0], {
      extrapolateLeft: "clamp",
      extrapolateRight: "clamp",
    });

  return (
    <>
      <AbsoluteFill
        name="Latar kolam"
        style={{
          background: "linear-gradient(135deg, #0E5A52 0%, #06302B 70%)",
        }}
      />
      <Interactive.Div
        name="Kicker"
        style={{
          position: "absolute",
          left: 120,
          top: 250,
          fontFamily: "Plus Jakarta Sans",
          fontWeight: 700,
          fontSize: 30,
          letterSpacing: 5,
          color: "#5FD3B4",
          opacity: interpolate(frame, [6, 20], [0, 1], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
          }),
        }}
      >
        {kicker}
      </Interactive.Div>
      <Interactive.Div
        name="Judul"
        style={{
          position: "absolute",
          left: 120,
          top: 300,
          width: 520,
          fontFamily: "Plus Jakarta Sans",
          fontWeight: 800,
          fontSize: 84,
          lineHeight: 1.05,
          letterSpacing: -1.5,
          color: "#F6ECD0",
          opacity: interpolate(frame, [10, 26], [0, 1], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
          }),
          translate: interpolate(frame, [10, 34], ["0px 30px", "0px 0px"], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
            easing: Easing.bezier(0.16, 1, 0.3, 1),
          }),
        }}
      >
        {judul}
      </Interactive.Div>
      {[
        "Jelajahi folder lokal & cloud",
        "Ruang & uji coba diperiksa dulu",
        "Menyalin di latar; boleh ditutup",
      ].map((t, n) => (
        <div
          key={t}
          style={{
            position: "absolute",
            left: 120,
            top: 640 + n * 70,
            width: 580,
            display: "flex",
            alignItems: "center",
            gap: 18,
            fontFamily: "Plus Jakarta Sans",
            fontWeight: 500,
            fontSize: 34,
            color: "#F6ECD0",
            opacity: interpolate(frame, [40 + n * 14, 54 + n * 14], [0, 0.92], {
              extrapolateLeft: "clamp",
              extrapolateRight: "clamp",
            }),
          }}
        >
          <div
            style={{
              width: 14,
              height: 14,
              borderRadius: 7,
              backgroundColor: "#5FD3B4",
              flexShrink: 0,
            }}
          />
          {t}
        </div>
      ))}
      <div
        style={{
          position: "absolute",
          left: KIRI,
          top: ATAS,
          width: 1225 * S,
          height: 900 * S,
          borderRadius: 18,
          overflow: "hidden",
          boxShadow: "0 40px 90px rgba(0, 0, 0, 0.45)",
          outline: "1px solid rgba(255, 255, 255, 0.12)",
          opacity: interpolate(frame, [0, 18], [0, 1], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
          }),
          translate: interpolate(frame, [0, 30], ["0px 90px", "0px 0px"], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
            easing: Easing.bezier(0.16, 1, 0.3, 1),
          }),
          scale: interpolate(frame, [0, 30], [0.95, 1], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
            easing: Easing.bezier(0.16, 1, 0.3, 1),
            output: "perceptual-scale",
          }),
        }}
      >
        <CanvasImage
          name="Tangkapan layar"
          src={staticFile("aplikasi.png")}
          premountFor={fps}
          style={{ width: 1225 * S, height: 900 * S }}
        />
        {/* Kartu progres menutupi bilah "Penyalinan selesai" bawaan tangkapan layar. */}
        <div
          style={{
            position: "absolute",
            left: 225 * S,
            top: 748 * S,
            width: 1000 * S,
            height: 102 * S,
            backgroundColor: "#FFFFFF",
            padding: `${18 * S}px ${20 * S}px`,
            boxSizing: "border-box",
            fontFamily: "Plus Jakarta Sans",
            opacity: interpolate(frame, [164, 172], [0, 1], {
              extrapolateLeft: "clamp",
              extrapolateRight: "clamp",
            }),
          }}
        >
          <div
            style={{
              display: "flex",
              justifyContent: "space-between",
              fontSize: 17,
              color: "#1F2328",
            }}
          >
            <span style={{ fontWeight: 700 }}>
              {selesai ? "Penyalinan selesai" : "Menyalin"}{" "}
              <span style={{ fontWeight: 500, color: "#59636E" }}>
                {Math.round(progres * 1288).toLocaleString("id-ID")} dari 1.288
                file
              </span>
            </span>
            <span style={{ fontWeight: 700 }}>
              {Math.round(progres * 100)}%
            </span>
          </div>
          <div
            style={{
              marginTop: 12,
              height: 8,
              borderRadius: 4,
              backgroundColor: "#EAEEF2",
            }}
          >
            <div
              style={{
                width: `${progres * 100}%`,
                height: 8,
                borderRadius: 4,
                backgroundColor: selesai ? "#1A7F37" : "#0969DA",
              }}
            />
          </div>
        </div>
        {/* Sorotan panel Dari, panel Ke, lalu tombol Salin. */}
        <div
          style={{
            position: "absolute",
            left: 226 * S,
            top: 62 * S,
            width: 476 * S,
            height: 686 * S,
            borderRadius: 10,
            border: "5px solid #5FD3B4",
            boxShadow: "0 0 40px rgba(95, 211, 180, 0.6)",
            opacity: sorot(52, 112),
          }}
        />
        <div
          style={{
            position: "absolute",
            left: 749 * S,
            top: 62 * S,
            width: 476 * S,
            height: 686 * S,
            borderRadius: 10,
            border: "5px solid #5FD3B4",
            boxShadow: "0 0 40px rgba(95, 211, 180, 0.6)",
            opacity: sorot(108, 168),
          }}
        />
        <div
          style={{
            position: "absolute",
            left: 290 * S,
            top: 6 * S,
            width: 137 * S,
            height: 48 * S,
            borderRadius: 10,
            border: "4px solid #5FD3B4",
            boxShadow: "0 0 30px rgba(95, 211, 180, 0.8)",
            opacity: sorot(150, 178),
            scale: interpolate(frame, [160, 164, 168], [1, 0.9, 1], {
              extrapolateLeft: "clamp",
              extrapolateRight: "clamp",
            }),
          }}
        />
      </div>
      {[
        { t: "DARI", x: 226, mulai: 52, akhir: 112 },
        { t: "KE", x: 749, mulai: 108, akhir: 168 },
      ].map((p) => (
        <div
          key={p.t}
          style={{
            position: "absolute",
            left: KIRI + p.x * S,
            top: ATAS - 58,
            padding: "8px 20px",
            borderRadius: 24,
            backgroundColor: "#5FD3B4",
            color: "#06302B",
            fontFamily: "Plus Jakarta Sans",
            fontWeight: 800,
            fontSize: 26,
            letterSpacing: 3,
            opacity: sorot(p.mulai, p.akhir),
          }}
        >
          {p.t}
        </div>
      ))}
      <Audio
        name="Klik Salin"
        src={staticFile("sfx/mouse-click.wav")}
        from={160}
        premountFor={fps}
        volume={0.6}
      />
      <Audio
        name="Selesai"
        src={staticFile("sfx/ding.wav")}
        from={252}
        premountFor={fps}
        volume={0.25}
      />
    </>
  );
};

const aplikasiSchema = {
  kicker: { type: "string", default: "CARA KERJANYA", description: "Kicker" },
  judul: {
    type: "string",
    default: "Pilih Dari dan Ke. Klik Salin.",
    description: "Judul",
  },
} as const satisfies InteractivitySchema;

export const Aplikasi = Interactive.withSchema({
  Component: AplikasiInner,
  componentName: "Aplikasi",
  schema: aplikasiSchema,
  wrapInSequence: true,
  layout: "absolute-fill",
});
