// Tema bersama video promosi LELEMOVE: warna dari ikon & aplikasi, huruf Plus Jakarta Sans
// (OFL, karya studio Indonesia), dan data 29 layanan (warna & inisial sama dengan aplikasi).
import { loadFont } from "@remotion/fonts";
import { staticFile } from "remotion";

export const HURUF = "Plus Jakarta Sans";

// Satu berkas variabel (berat 200-800), disimpan lokal agar render tidak butuh internet.
loadFont({
  family: HURUF,
  url: staticFile("fonts/PlusJakartaSans.ttf"),
  weight: "200 800",
});

export type Layanan = {
  readonly nama: string;
  readonly huruf: string;
  readonly latar: string;
  readonly tinta: string;
};

// Urutan: layanan cloud, server kantor & NAS, object storage (seperti di aplikasi).
export const LAYANAN: Layanan[] = [
  { nama: "Google Drive", huruf: "GD", latar: "#E4F3E8", tinta: "#1E7E3E" },
  { nama: "OneDrive", huruf: "OD", latar: "#E3EFFB", tinta: "#0B5CAD" },
  { nama: "SharePoint", huruf: "SP", latar: "#DFF2F1", tinta: "#0B6E6A" },
  { nama: "Dropbox", huruf: "Db", latar: "#E5EEFF", tinta: "#0061FE" },
  { nama: "Box", huruf: "Bx", latar: "#E6F2FB", tinta: "#0061D5" },
  { nama: "pCloud", huruf: "pC", latar: "#E2F5F7", tinta: "#13869A" },
  { nama: "Yandex Disk", huruf: "Ya", latar: "#FFF1E0", tinta: "#B45A00" },
  { nama: "HiDrive", huruf: "Hi", latar: "#FFEEE4", tinta: "#D04A02" },
  { nama: "Zoho WorkDrive", huruf: "Zo", latar: "#FDECEC", tinta: "#C81E2B" },
  { nama: "MEGA", huruf: "Mg", latar: "#FDE8E8", tinta: "#D90007" },
  { nama: "Koofr", huruf: "Ko", latar: "#E6F4EA", tinta: "#2E7D32" },
  { nama: "iCloud Drive", huruf: "iC", latar: "#ECF3FD", tinta: "#1F6FD6" },
  { nama: "Proton Drive", huruf: "Pr", latar: "#F0EBFD", tinta: "#6D4AFF" },
  { nama: "Jottacloud", huruf: "Jo", latar: "#E8F1FA", tinta: "#1A559E" },
  { nama: "Nextcloud", huruf: "Nc", latar: "#E3F0FB", tinta: "#0082C9" },
  { nama: "ownCloud", huruf: "oC", latar: "#E6EEF7", tinta: "#041E42" },
  { nama: "WebDAV", huruf: "DAV", latar: "#EEEFF1", tinta: "#3B444F" },
  { nama: "NAS (SMB)", huruf: "SMB", latar: "#EAEEF3", tinta: "#344E6B" },
  { nama: "SFTP", huruf: "SSH", latar: "#ECEEF0", tinta: "#2D333A" },
  { nama: "FTP", huruf: "FTP", latar: "#F2EEE6", tinta: "#6B552B" },
  { nama: "Seafile", huruf: "Sf", latar: "#FEF1E2", tinta: "#E07A0B" },
  { nama: "Amazon S3", huruf: "S3", latar: "#FFF2E0", tinta: "#B35C00" },
  { nama: "Cloudflare R2", huruf: "R2", latar: "#FEF0E4", tinta: "#D95F0A" },
  { nama: "Wasabi", huruf: "Wa", latar: "#E9F6EC", tinta: "#15803D" },
  { nama: "Backblaze B2", huruf: "B2", latar: "#FDEAEA", tinta: "#C81E1E" },
  { nama: "Azure Blob", huruf: "Az", latar: "#E5F1FB", tinta: "#0078D4" },
  { nama: "MinIO", huruf: "Mi", latar: "#FCE8EC", tinta: "#C72C48" },
  { nama: "DO Spaces", huruf: "DO", latar: "#E5F0FF", tinta: "#0069FF" },
  { nama: "S3 lainnya", huruf: "S3", latar: "#EEEFF1", tinta: "#3B444F" },
];
