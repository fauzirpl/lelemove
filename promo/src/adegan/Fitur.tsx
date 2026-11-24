// Adegan 6: fitur lain. Empat kartu masuk bergantian; Migrasi email (baru) disorot terakhir.
import type React from "react";
import {
  AbsoluteFill,
  Easing,
  Interactive,
  interpolate,
  useCurrentFrame,
  type InteractivitySchema,
} from "remotion";
import { IkonAmplop, IkonCorong, IkonKalender, IkonLaporan } from "../Ikon";
import "../tema";

const KARTU = [
  {
    Ikon: IkonKalender,
    judul: "Jadwal backup otomatis",
    isi: "Menyalin sendiri setiap hari atau pada hari tertentu.",
    baru: false,
  },
  {
    Ikon: IkonCorong,
    judul: "Filter file",
    isi: "Hanya PDF & dokumen Office, atau yang berubah sejak tanggal tertentu.",
    baru: false,
  },
  {
    Ikon: IkonLaporan,
    judul: "Riwayat & laporan",
    isi: "CSV dan laporan siap cetak untuk keperluan audit.",
    baru: false,
  },
  {
    Ikon: IkonAmplop,
    judul: "Migrasi email",
    isi: "Gmail atau Outlook ke server email kantor, lengkap dengan folder & tanda baca.",
    baru: true,
  },
];

const FiturInner: React.FC<{ judul: string }> = ({ judul }) => {
  const frame = useCurrentFrame();

  return (
    <>
      <AbsoluteFill
        name="Latar kolam"
        style={{
          background:
            "radial-gradient(circle at 50% 30%, #0E5A52 0%, #06302B 80%)",
        }}
      />
      <Interactive.Div
        name="Judul"
        style={{
          position: "absolute",
          left: 120,
          right: 120,
          top: 110,
          textAlign: "center",
          fontFamily: "Plus Jakarta Sans",
          fontWeight: 800,
          fontSize: 96,
          letterSpacing: -2,
          color: "#F6ECD0",
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
      {KARTU.map((k, i) => {
        const mulai = 22 + i * 24;
        const kolom = i % 2;
        const baris = Math.floor(i / 2);
        const sinar = k.baru ? 0.35 + 0.25 * Math.sin(frame / 9) : 0;
        return (
          <div
            key={k.judul}
            style={{
              position: "absolute",
              left: 150 + kolom * 830,
              top: 300 + baris * 330,
              width: 790,
              height: 290,
              borderRadius: 30,
              backgroundColor: k.baru
                ? "rgba(95, 211, 180, 0.14)"
                : "rgba(255, 255, 255, 0.07)",
              border: k.baru
                ? "3px solid #5FD3B4"
                : "2px solid rgba(255, 255, 255, 0.14)",
              boxShadow: k.baru
                ? `0 0 60px rgba(95, 211, 180, ${sinar})`
                : "none",
              padding: "44px 48px",
              boxSizing: "border-box",
              display: "flex",
              gap: 36,
              fontFamily: "Plus Jakarta Sans",
              opacity: interpolate(frame, [mulai, mulai + 12], [0, 1], {
                extrapolateLeft: "clamp",
                extrapolateRight: "clamp",
              }),
              translate: interpolate(
                frame,
                [mulai, mulai + 24],
                [`${kolom === 0 ? -80 : 80}px 0px`, "0px 0px"],
                {
                  extrapolateLeft: "clamp",
                  extrapolateRight: "clamp",
                  easing: Easing.bezier(0.16, 1, 0.3, 1),
                },
              ),
            }}
          >
            <div
              style={{
                width: 120,
                height: 120,
                flexShrink: 0,
                borderRadius: 28,
                backgroundColor: "rgba(95, 211, 180, 0.16)",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
              }}
            >
              <k.Ikon ukuran={72} warna="#5FD3B4" />
            </div>
            <div>
              <div style={{ display: "flex", alignItems: "center", gap: 18 }}>
                <div
                  style={{
                    fontSize: 50,
                    fontWeight: 800,
                    color: "#F6ECD0",
                    letterSpacing: -0.5,
                  }}
                >
                  {k.judul}
                </div>
                {k.baru ? (
                  <div
                    style={{
                      padding: "6px 16px",
                      borderRadius: 18,
                      backgroundColor: "#5FD3B4",
                      color: "#06302B",
                      fontSize: 24,
                      fontWeight: 800,
                      letterSpacing: 2,
                    }}
                  >
                    BARU
                  </div>
                ) : null}
              </div>
              <div
                style={{
                  marginTop: 14,
                  fontSize: 33,
                  fontWeight: 500,
                  lineHeight: 1.35,
                  color: "rgba(246, 236, 208, 0.82)",
                }}
              >
                {k.isi}
              </div>
            </div>
          </div>
        );
      })}
    </>
  );
};

const fiturSchema = {
  judul: {
    type: "string",
    default: "Lebih dari sekadar menyalin",
    description: "Judul",
  },
} as const satisfies InteractivitySchema;

export const Fitur = Interactive.withSchema({
  Component: FiturInner,
  componentName: "Fitur",
  schema: fiturSchema,
  wrapInSequence: true,
  layout: "absolute-fill",
});
