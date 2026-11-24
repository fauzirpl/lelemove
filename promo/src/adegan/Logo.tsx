// Adegan 2: jawaban. Garis gerak mint melesat, ikon lele memantul masuk, lalu nama dan slogan.
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

const LogoInner: React.FC<{ slogan: string }> = ({ slogan }) => {
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
      {[0, 1, 2].map((n) => (
        <div
          key={n}
          style={{
            position: "absolute",
            top: 300 + n * 70,
            left: 520 + (n === 1 ? -40 : 0),
            width: n === 1 ? 190 : 140,
            height: 22,
            borderRadius: 11,
            backgroundColor: "#5FD3B4",
            opacity: interpolate(
              frame,
              [n * 3, n * 3 + 8, 40, 55],
              [0, 1, 1, 0.55],
              {
                extrapolateLeft: "clamp",
                extrapolateRight: "clamp",
              },
            ),
            translate: interpolate(
              frame,
              [n * 3, n * 3 + 22],
              ["-700px 0px", "0px 0px"],
              {
                extrapolateLeft: "clamp",
                extrapolateRight: "clamp",
                easing: Easing.bezier(0.16, 1, 0.3, 1),
              },
            ),
          }}
        />
      ))}
      <CanvasImage
        name="Ikon lele"
        src={staticFile("ikon.png")}
        premountFor={fps}
        style={{
          position: "absolute",
          left: 760,
          top: 170,
          width: 400,
          height: 400,
          scale: interpolate(frame, [6, 36], [0.35, 1], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
            easing: Easing.spring({ damping: 12 }),
            output: "perceptual-scale",
          }),
          rotate: interpolate(frame, [6, 36], ["-14deg", "0deg"], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
            easing: Easing.spring({ damping: 14 }),
          }),
          opacity: interpolate(frame, [6, 14], [0, 1], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
          }),
        }}
      />
      <Interactive.Div
        name="Nama aplikasi"
        style={{
          position: "absolute",
          left: 0,
          right: 0,
          top: 600,
          textAlign: "center",
          fontFamily: "Plus Jakarta Sans",
          fontWeight: 800,
          fontSize: 156,
          letterSpacing: 6,
          lineHeight: 1,
          opacity: interpolate(frame, [26, 40], [0, 1], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
          }),
          translate: interpolate(frame, [26, 50], ["0px 50px", "0px 0px"], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
            easing: Easing.bezier(0.16, 1, 0.3, 1),
          }),
        }}
      >
        <span style={{ color: "#F6ECD0" }}>LELE</span>
        <span style={{ color: "#5FD3B4" }}>MOVE</span>
      </Interactive.Div>
      <Interactive.Div
        name="Slogan"
        style={{
          position: "absolute",
          left: 220,
          right: 220,
          top: 800,
          textAlign: "center",
          fontFamily: "Plus Jakarta Sans",
          fontWeight: 500,
          fontSize: 52,
          color: "#F6ECD0",
          opacity: interpolate(frame, [52, 72], [0, 0.9], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
          }),
        }}
      >
        {slogan}
      </Interactive.Div>
    </>
  );
};

const logoSchema = {
  slogan: {
    type: "string",
    default: "Salin file antar cloud dan komputer, ke arah mana pun.",
    description: "Slogan",
  },
} as const satisfies InteractivitySchema;

export const Logo = Interactive.withSchema({
  Component: LogoInner,
  componentName: "Logo",
  schema: logoSchema,
  wrapInSequence: true,
  layout: "absolute-fill",
});
