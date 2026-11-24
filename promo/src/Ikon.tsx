// Ikon garis sederhana untuk kartu fitur (digambar sendiri, tanpa pustaka ikon).
import type React from "react";

type IkonProps = {
  readonly ukuran: number;
  readonly warna: string;
};

const svg = (ukuran: number, isi: React.ReactNode, warna: string) => (
  <svg
    width={ukuran}
    height={ukuran}
    viewBox="0 0 48 48"
    fill="none"
    stroke={warna}
    strokeWidth={3.2}
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    {isi}
  </svg>
);

export const IkonKalender: React.FC<IkonProps> = ({ ukuran, warna }) =>
  svg(
    ukuran,
    <>
      <rect x="7" y="10" width="34" height="31" rx="5" />
      <path d="M7 19h34M16 6v8M32 6v8" />
      <path d="M17 29l5 5 9-10" />
    </>,
    warna,
  );

export const IkonCorong: React.FC<IkonProps> = ({ ukuran, warna }) =>
  svg(ukuran, <path d="M7 9h34L28 25v12l-8 4V25z" />, warna);

export const IkonLaporan: React.FC<IkonProps> = ({ ukuran, warna }) =>
  svg(
    ukuran,
    <>
      <path d="M12 5h17l9 9v29H12z" />
      <path d="M29 5v9h9M18 25h14M18 32h14M18 18h6" />
    </>,
    warna,
  );

export const IkonAmplop: React.FC<IkonProps> = ({ ukuran, warna }) =>
  svg(
    ukuran,
    <>
      <rect x="6" y="11" width="36" height="27" rx="4" />
      <path d="M7 13l17 13 17-13" />
    </>,
    warna,
  );
