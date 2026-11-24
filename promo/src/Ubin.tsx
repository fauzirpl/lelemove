// Petak layanan seperti di daftar akun aplikasi: persegi membulat berwarna berisi inisial.
import type React from "react";
import type { Layanan } from "./tema";

type UbinProps = {
  readonly l: Layanan;
  readonly ukuran: number;
  /** Tampilkan nama layanan di bawah petak. */
  readonly label?: boolean;
  readonly warnaLabel?: string;
  readonly style?: React.CSSProperties;
};

export const Ubin: React.FC<UbinProps> = ({
  l,
  ukuran,
  label,
  warnaLabel,
  style,
}) => {
  const panjang = l.huruf.length;
  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        gap: ukuran * 0.12,
        width: ukuran * 1.5,
        ...style,
      }}
    >
      <div
        style={{
          width: ukuran,
          height: ukuran,
          borderRadius: ukuran * 0.24,
          backgroundColor: l.latar,
          color: l.tinta,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          fontFamily: "Plus Jakarta Sans",
          fontWeight: 800,
          fontSize: ukuran * (panjang >= 3 ? 0.3 : 0.4),
          letterSpacing: -0.5,
          boxShadow: "0 10px 30px rgba(0, 0, 0, 0.18)",
        }}
      >
        {l.huruf}
      </div>
      {label ? (
        <div
          style={{
            fontFamily: "Plus Jakarta Sans",
            fontWeight: 600,
            fontSize: ukuran * 0.17,
            color: warnaLabel ?? "#3B444F",
            whiteSpace: "nowrap",
          }}
        >
          {l.nama}
        </div>
      ) : null}
    </div>
  );
};
