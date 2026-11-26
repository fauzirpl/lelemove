"""Rakit README zaman perdjoeangan dari 4 bagian + blok kode asli, lalu periksa.

Pemakaian: python rakit_readme.py <repo> <folder-bagian> [--tulis]
"""
import re
import subprocess
import sys
import unicodedata
from pathlib import Path

repo = Path(sys.argv[1])
bagian = Path(sys.argv[2])
tulis = "--tulis" in sys.argv

asli = subprocess.run(["git", "-C", str(repo), "show", "HEAD:README.md"],
                      capture_output=True, check=True).stdout.decode("utf-8")
asli = asli.replace("\r\n", "\n")

POLA_BLOK = re.compile(r"^```[^\n]*\n.*?^```[ \t]*$", re.M | re.S)
blok_asli = POLA_BLOK.findall(asli)

# Perubahan isi blok kode (fitur baru). Idempoten: dilewati bila teks baru sudah ada.
TAMBALAN = [
    (
        "-Tujuan imap://nama@mail.server.id -Ya\n```",
        "-Tujuan imap://nama@mail.server.id -Ya\n"
        ".\\pindah.exe email -Sumber gmail:nama@gmail.com -Tujuan \"D:\\Cadangan Email\" -Ya            # cadangkan ke komputer (.eml)\n"
        ".\\pindah.exe email -Sumber \"D:\\Cadangan Email\" -Tujuan imap://nama@mail.server.id -Ya      # pulihkan dari cadangan\n"
        ".\\pindah.exe email -Sumber \"D:\\Takeout\\Mail\\All mail Including Spam and Trash.mbox\" -Tujuan imap://nama@mail.server.id -Ya\n```",
    ),
    (
        "  src/mesin/email.rs           migrasi email: peta folder (Gmail/Outlook/cPanel), cocok Message-ID, sambung ulang\n",
        "  src/mesin/email.rs           migrasi & cadangan email: sumber/tujuan server atau komputer, peta folder\n"
        "                               (Gmail/Outlook/cPanel), cocok Message-ID, sambung ulang\n"
        "  src/mesin/email_lokal.rs     cadangan .eml di komputer: indeks per folder, nama berkas, tanggal berkas\n"
        "  src/mesin/mbox.rs            baca mbox (Google Takeout berlabel, Thunderbird) untuk pemulihan email\n",
    ),
]
TAMBALAN.append((
    "pindah cek                                              # rclone, akun, folder OneDrive, rute\n",
    "pindah cek                                              # rclone, akun, folder OneDrive, rute\n"
    "pindah pembaruan [-Unduh | -Jalankan]                   # periksa rilis terbaru; unduh (+cek SHA-256) / pasang\n",
))
TAMBALAN.append((
    "# 1. Naikkan \"version\" di rust\\Cargo.toml (dan .claude-plugin\\plugin.json), commit, lalu push.\n"
    "# 2. Bangun installer publik dan checksum-nya.\n"
    ".\\buat-installer.ps1 -TanpaClient\n"
    "$exe = \".\\dist\\LELEMOVE-Setup-<versi>.exe\"\n"
    "\"$((Get-FileHash $exe -Algorithm SHA256).Hash.ToLower())  $(Split-Path $exe -Leaf)\" | Set-Content -Encoding ascii \"$exe.sha256\"\n"
    "# 3. Terbitkan (tag dibuat di commit terakhir main).\n"
    "gh release create v<versi> $exe \"$exe.sha256\" --target main --title \"LELEMOVE <versi>\" --notes-file catatan-rilis.md\n",
    "# 1. Naikkan \"version\" di rust\\Cargo.toml (dan .claude-plugin\\plugin.json), tulis catatan rilis di\n"
    "#    catatan-rilis\\<versi>.md (baris @@CHECKSUM@@ diisi otomatis), perbarui README, commit, lalu push.\n"
    "# 2. Beri tag. Workflow \"Rilis\" membangun installer Windows dan .dmg macOS, lalu membuat draf rilis.\n"
    "git tag v<versi>\n"
    "git push origin v<versi>\n"
    "# 3. Periksa drafnya (catatan dan empat berkas), lalu terbitkan.\n"
    "gh release edit v<versi> --draft=false --latest\n",
))
TAMBALAN.append((
    "  src/mesin/                   mesin: akun, rute, job & log rclone, semua perintah\n",
    "  src/mesin/                   mesin: akun, rute, job & log rclone, semua perintah\n"
    "  src/mesin/perintah/          satu berkas per kelompok perintah (akun, hubungkan, salin, verifikasi, email, ...)\n"
    "  src/mesin/pembaruan.rs       pembaruan dari GitHub Releases: cek versi, unduh, cocokkan SHA-256\n",
))
TAMBALAN.append((
    "  src/bin/pindah-drive/        LELEMOVE.exe: jendela aplikasi (egui) + dialog\n",
    "  src/bin/pindah-drive/        LELEMOVE.exe: jendela aplikasi (egui) + dialog\n"
    "  src/bin/pindah-drive/app_*.rs bagian jendela utama (kabar utas latar, panel, lokasi, salin, tampilan, ...)\n",
))
for lama, baru in TAMBALAN:
    if any(baru in b for b in blok_asli):
        continue
    cocok = [i for i, b in enumerate(blok_asli) if lama in b]
    if len(cocok) != 1:
        sys.exit(f"tambalan tidak tepat satu blok: {lama[:40]!r} -> {cocok}")
    blok_asli[cocok[0]] = blok_asli[cocok[0]].replace(lama, baru)

teks = "".join((bagian / f"{i}.md").read_text(encoding="utf-8") for i in range(1, 5))
teks = teks.replace("\r\n", "\n")
teks = re.sub(r"\n{3,}", "\n\n", teks).rstrip("\n") + "\n"

dipakai = []
def ganti(m):
    n = int(m.group(1))
    dipakai.append(n)
    return blok_asli[n - 1]
teks = re.sub(r"^@@KODE (\d+)@@$", ganti, teks, flags=re.M)

masalah = []
if sorted(dipakai) != list(range(1, len(blok_asli) + 1)):
    masalah.append(f"placeholder {dipakai} tidak cocok dengan {len(blok_asli)} blok asli")
blok_baru = POLA_BLOK.findall(teks)
if blok_baru != blok_asli:
    masalah.append("blok kode berbeda dari aslinya")
if "@@KODE" in teks:
    masalah.append("masih ada placeholder")


def jangkar(judul, sudah):
    j = judul.strip().lower()
    j = j.replace("`", "")
    j = "".join(c for c in j if c in " -_" or unicodedata.category(c)[0] in "LN")
    j = j.replace(" ", "-")
    dasar, n = j, 1
    while j in sudah:
        j = f"{dasar}-{n}"
        n += 1
    sudah.add(j)
    return j


def daftar_jangkar(md):
    tanpa_kode = POLA_BLOK.sub("", md)
    sudah = set()
    for m in re.finditer(r"^#{1,6} (.+)$", tanpa_kode, re.M):
        jangkar(m.group(1), sudah)
    nama = set(re.findall(r'<a name="([^"]+)"></a>', tanpa_kode))
    return sudah, nama, tanpa_kode


j_asli, _, _ = daftar_jangkar(asli)
j_baru, n_baru, prosa = daftar_jangkar(teks)
semua = j_baru | n_baru

hilang = sorted(j_asli - semua)
if hilang:
    masalah.append(f"jangkar lama hilang: {hilang}")
for tautan in sorted(set(re.findall(r"\]\(#([^)]+)\)", prosa))):
    if tautan not in semua:
        masalah.append(f"tautan rusak: #{tautan}")
for n in sorted(n_baru & j_baru):
    masalah.append(f"<a name> bentrok dengan judul: {n}")

# Sisa ejaan modern di prosa: buang kode sebaris, URL, dan teks miring (label UI).
bersih = re.sub(r"`[^`]*`", " ", prosa)
bersih = re.sub(r"\]\([^)]*\)|<https?://[^>]+>|<a name=\"[^\"]+\"></a>", " ", bersih)
bersih = re.sub(r"(?<![*\w])\*(?!\*)[^*\n]+\*(?!\*)", " ", bersih)
modern = ["yang", "untuk", "juga", "sudah", "cukup", "jadi", "jika", "Anda", "harus", "perlu",
          "buka", "baru", "lalu", "itu", "hanya", "satu", "dua", "akun", "tujuan", "sumber",
          "menyalin", "penyalinan", "jadwal", "jaringan", "kecepatan", "dukung", "dulu", "belum"]
sisa = {}
for kata in modern:
    k = len(re.findall(rf"(?<![\w-]){kata}(?![\w-])", bersih))
    if k:
        sisa[kata] = k

print(f"blok kode asli: {len(blok_asli)}, baris README baru: {teks.count(chr(10))}")
print(f"jangkar judul baru: {len(j_baru)}, <a name>: {len(n_baru)}")
print("sisa kata ejaan baru (di luar kode/label UI):", sisa or "tidak ada")
if masalah:
    print("MASALAH:")
    for m in masalah:
        print(" -", m)
    sys.exit(1)
print("periksa: lolos")
if tulis:
    (repo / "README.md").write_bytes(teks.encode("utf-8"))
    print("README.md ditulis (UTF-8 tanpa BOM, LF)")
