// Adegan 3: 29 layanan. Angka besar menghitung naik; kisi petak layanan muncul berurutan.
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
import { LAYANAN } from "../tema";
import { Ubin } from "../Ubin";

const LayananInner: React.FC<{
  label: string;
  keterangan: string;
  lainnya: string;
}> = ({ label, keterangan, lainnya }) => {
  const frame = useCurrentFrame();
  const hitung = Math.round(
    interpolate(frame, [10, 62], [0, 29], {
      extrapolateLeft: "clamp",
      extrapolateRight: "clamp",
      easing: Easing.bezier(0.33, 1, 0.68, 1),
    }),
  );

  return (
    <>
      <AbsoluteFill name="Latar krem" style={{ backgroundColor: "#F5F1E6" }} />
      <Interactive.Div
        name="Angka"
        style={{
          position: "absolute",
          left: 140,
          top: 170,
          fontFamily: "Plus Jakarta Sans",
          fontWeight: 800,
          fontSize: 300,
          lineHeight: 1,
          letterSpacing: -10,
          color: "#0E5A52",
          opacity: interpolate(frame, [4, 16], [0, 1], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
          }),
        }}
      >
        {hitung}
      </Interactive.Div>
      <Interactive.Div
        name="Label"
        style={{
          position: "absolute",
          left: 150,
          top: 480,
          width: 760,
          fontFamily: "Plus Jakarta Sans",
          fontWeight: 800,
          fontSize: 64,
          lineHeight: 1.05,
          letterSpacing: -1.5,
          color: "#1F2328",
          opacity: interpolate(frame, [18, 34], [0, 1], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
          }),
          translate: interpolate(frame, [18, 40], ["0px 30px", "0px 0px"], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
            easing: Easing.bezier(0.16, 1, 0.3, 1),
          }),
        }}
      >
        {label}
      </Interactive.Div>
      <Interactive.Div
        name="Keterangan"
        style={{
          position: "absolute",
          left: 150,
          top: 590,
          width: 680,
          fontFamily: "Plus Jakarta Sans",
          fontWeight: 500,
          fontSize: 42,
          lineHeight: 1.35,
          color: "#4A535D",
          opacity: interpolate(frame, [34, 52], [0, 1], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
          }),
        }}
      >
        {keterangan}
      </Interactive.Div>
      <Interactive.Div
        name="Lainnya"
        style={{
          position: "absolute",
          left: 150,
          top: 830,
          padding: "14px 28px",
          borderRadius: 40,
          backgroundColor: "#0E5A52",
          color: "#F6ECD0",
          fontFamily: "Plus Jakarta Sans",
          fontWeight: 700,
          fontSize: 36,
          opacity: interpolate(frame, [70, 86], [0, 1], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
          }),
          scale: interpolate(frame, [70, 90], [0.8, 1], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
            easing: Easing.spring({ damping: 14 }),
            output: "perceptual-scale",
          }),
        }}
      >
        {lainnya}
      </Interactive.Div>
      <div
        style={{
          position: "absolute",
          left: 880,
          top: 130,
          width: 960,
          display: "grid",
          gridTemplateColumns: "repeat(6, 1fr)",
          rowGap: 22,
          justifyItems: "center",
        }}
      >
        {LAYANAN.map((l, i) => {
          const mulai = 14 + i * 3;
          return (
            <div
              key={l.nama}
              style={{
                opacity: interpolate(frame, [mulai, mulai + 8], [0, 1], {
                  extrapolateLeft: "clamp",
                  extrapolateRight: "clamp",
                }),
                scale: interpolate(frame, [mulai, mulai + 16], [0.4, 1], {
                  extrapolateLeft: "clamp",
                  extrapolateRight: "clamp",
                  easing: Easing.spring({ damping: 11 }),
                  output: "perceptual-scale",
                }),
              }}
            >
              <Ubin l={l} ukuran={104} label warnaLabel="#3B444F" />
            </div>
          );
        })}
      </div>
    </>
  );
};

const layananSchema = {
  label: {
    type: "string",
    default: "layanan penyimpanan",
    description: "Label",
  },
  keterangan: {
    type: "string",
    default:
      "Google Drive, OneDrive, Dropbox, NAS kantor, hingga S3, semua dari satu jendela.",
    description: "Keterangan",
  },
  lainnya: {
    type: "string",
    default: "+ 70 layanan lain lewat rclone",
    description: "Layanan lain",
  },
} as const satisfies InteractivitySchema;

export const Layanan = Interactive.withSchema({
  Component: LayananInner,
  componentName: "Layanan",
  schema: layananSchema,
  wrapInSequence: true,
  layout: "absolute-fill",
});
