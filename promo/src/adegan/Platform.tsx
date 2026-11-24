// Adegan 7: platform. Windows & macOS, gratis, sumber terbuka, tanpa hak admin.
import type React from "react";
import {
  AbsoluteFill,
  Easing,
  Interactive,
  interpolate,
  useCurrentFrame,
  type InteractivitySchema,
} from "remotion";
import "../tema";

const PlatformInner: React.FC<{ keterangan: string }> = ({ keterangan }) => {
  const frame = useCurrentFrame();

  return (
    <>
      <AbsoluteFill name="Latar krem" style={{ backgroundColor: "#F5F1E6" }} />
      <Interactive.Div
        name="Judul"
        style={{
          position: "absolute",
          left: 0,
          right: 0,
          top: 250,
          textAlign: "center",
          fontFamily: "Plus Jakarta Sans",
          fontWeight: 800,
          fontSize: 150,
          letterSpacing: -4,
          lineHeight: 1,
          color: "#1F2328",
          opacity: interpolate(frame, [2, 16], [0, 1], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
          }),
          scale: interpolate(frame, [2, 26], [0.9, 1], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
            easing: Easing.spring({ damping: 14 }),
            output: "perceptual-scale",
          }),
        }}
      >
        Windows <span style={{ color: "#0E5A52" }}>&</span> macOS
      </Interactive.Div>
      <div
        style={{
          position: "absolute",
          left: 0,
          right: 0,
          top: 480,
          display: "flex",
          justifyContent: "center",
          gap: 30,
        }}
      >
        {["Windows 10 / 11", "macOS 11+ · Apple Silicon & Intel"].map(
          (t, i) => (
            <div
              key={t}
              style={{
                padding: "16px 34px",
                borderRadius: 40,
                border: "3px solid #0E5A52",
                color: "#0E5A52",
                fontFamily: "Plus Jakarta Sans",
                fontWeight: 700,
                fontSize: 38,
                opacity: interpolate(frame, [22 + i * 8, 34 + i * 8], [0, 1], {
                  extrapolateLeft: "clamp",
                  extrapolateRight: "clamp",
                }),
                translate: interpolate(
                  frame,
                  [22 + i * 8, 42 + i * 8],
                  ["0px 30px", "0px 0px"],
                  {
                    extrapolateLeft: "clamp",
                    extrapolateRight: "clamp",
                    easing: Easing.bezier(0.16, 1, 0.3, 1),
                  },
                ),
              }}
            >
              {t}
            </div>
          ),
        )}
      </div>
      <Interactive.Div
        name="Keterangan"
        style={{
          position: "absolute",
          left: 0,
          right: 0,
          top: 680,
          textAlign: "center",
          fontFamily: "Plus Jakarta Sans",
          fontWeight: 700,
          fontSize: 56,
          color: "#1F2328",
          opacity: interpolate(frame, [46, 62], [0, 1], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
          }),
        }}
      >
        {keterangan}
      </Interactive.Div>
    </>
  );
};

const platformSchema = {
  keterangan: {
    type: "string",
    default: "Gratis · Sumber terbuka · Tanpa hak admin",
    description: "Keterangan",
  },
} as const satisfies InteractivitySchema;

export const Platform = Interactive.withSchema({
  Component: PlatformInner,
  componentName: "Platform",
  schema: platformSchema,
  wrapInSequence: true,
  layout: "absolute-fill",
});
