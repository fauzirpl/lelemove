// Adegan 8: ajakan. Logo, "Unduh gratis", alamat GitHub, dan tautan dukungan Saweria.
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

const PenutupInner: React.FC<{
  ajakan: string;
  alamat: string;
  dukung: string;
}> = ({ ajakan, alamat, dukung }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();

  return (
    <>
      <AbsoluteFill
        name="Latar kolam"
        style={{
          background:
            "radial-gradient(circle at 50% 40%, #0E5A52 0%, #06302B 78%)",
        }}
      />
      <div
        style={{
          position: "absolute",
          left: 0,
          right: 0,
          top: 150,
          display: "flex",
          justifyContent: "center",
          alignItems: "center",
          gap: 34,
        }}
      >
        <CanvasImage
          name="Ikon lele"
          src={staticFile("ikon.png")}
          premountFor={fps}
          style={{
            width: 210,
            height: 210,
            opacity: interpolate(frame, [0, 12], [0, 1], {
              extrapolateLeft: "clamp",
              extrapolateRight: "clamp",
            }),
            scale: interpolate(frame, [0, 24], [0.6, 1], {
              extrapolateLeft: "clamp",
              extrapolateRight: "clamp",
              easing: Easing.spring({ damping: 12 }),
              output: "perceptual-scale",
            }),
          }}
        />
        <Interactive.Div
          name="Nama aplikasi"
          style={{
            fontFamily: "Plus Jakarta Sans",
            fontWeight: 800,
            fontSize: 118,
            letterSpacing: 4,
            lineHeight: 1,
            opacity: interpolate(frame, [6, 20], [0, 1], {
              extrapolateLeft: "clamp",
              extrapolateRight: "clamp",
            }),
            translate: interpolate(frame, [6, 30], ["40px 0px", "0px 0px"], {
              extrapolateLeft: "clamp",
              extrapolateRight: "clamp",
              easing: Easing.bezier(0.16, 1, 0.3, 1),
            }),
          }}
        >
          <span style={{ color: "#F6ECD0" }}>LELE</span>
          <span style={{ color: "#5FD3B4" }}>MOVE</span>
        </Interactive.Div>
      </div>
      <Interactive.Div
        name="Ajakan"
        style={{
          position: "absolute",
          left: 0,
          right: 0,
          top: 450,
          textAlign: "center",
          fontFamily: "Plus Jakarta Sans",
          fontWeight: 800,
          fontSize: 100,
          letterSpacing: -2,
          color: "#F6ECD0",
          opacity: interpolate(frame, [24, 40], [0, 1], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
          }),
          translate: interpolate(frame, [24, 48], ["0px 30px", "0px 0px"], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
            easing: Easing.bezier(0.16, 1, 0.3, 1),
          }),
        }}
      >
        {ajakan}
      </Interactive.Div>
      <div
        style={{
          position: "absolute",
          left: 0,
          right: 0,
          top: 620,
          display: "flex",
          justifyContent: "center",
        }}
      >
        <Interactive.Div
          name="Alamat unduh"
          style={{
            padding: "22px 48px",
            borderRadius: 60,
            backgroundColor: "#5FD3B4",
            color: "#06302B",
            fontFamily: "Plus Jakarta Sans",
            fontWeight: 800,
            fontSize: 60,
            opacity: interpolate(frame, [40, 54], [0, 1], {
              extrapolateLeft: "clamp",
              extrapolateRight: "clamp",
            }),
            scale: interpolate(frame, [40, 64], [0.85, 1], {
              extrapolateLeft: "clamp",
              extrapolateRight: "clamp",
              easing: Easing.spring({ damping: 12 }),
              output: "perceptual-scale",
            }),
          }}
        >
          {alamat}
        </Interactive.Div>
      </div>
      <Interactive.Div
        name="Dukung"
        style={{
          position: "absolute",
          left: 0,
          right: 0,
          top: 830,
          textAlign: "center",
          fontFamily: "Plus Jakarta Sans",
          fontWeight: 600,
          fontSize: 44,
          color: "#F6ECD0",
          opacity: interpolate(frame, [66, 84], [0, 0.88], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
          }),
        }}
      >
        {dukung}
      </Interactive.Div>
    </>
  );
};

const penutupSchema = {
  ajakan: { type: "string", default: "Unduh gratis", description: "Ajakan" },
  alamat: {
    type: "string",
    default: "github.com/fauzirpl/lelemove",
    description: "Alamat unduh",
  },
  dukung: {
    type: "string",
    default: "Dukung pengembangannya: saweria.co/pemecuteai",
    description: "Dukungan",
  },
} as const satisfies InteractivitySchema;

export const Penutup = Interactive.withSchema({
  Component: PenutupInner,
  componentName: "Penutup",
  schema: penutupSchema,
  wrapInSequence: true,
  layout: "absolute-fill",
});
