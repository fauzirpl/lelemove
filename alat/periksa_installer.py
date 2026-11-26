"""Periksa installer publik: tidak boleh membawa client ID/secret Google.

Tidak pernah mencetak nilai rahasia; hanya ya/tidak dan jumlah.
Pemakaian: python periksa_installer.py <installer.exe>
"""
import io
import json
import os
import sys
import zipfile
from pathlib import Path

exe = Path(sys.argv[1]).read_bytes()
klien = Path(os.environ["LOCALAPPDATA"]) / "pindah-drive" / "client-google.json"
rahasia = []
if klien.exists():
    d = json.loads(klien.read_text(encoding="utf-8-sig"))
    for k in ("client_id", "ClientId", "client_secret", "ClientSecret"):
        if isinstance(d.get(k), str) and len(d[k]) > 8:
            rahasia.append(d[k].encode())
print(f"nilai pembanding dari komputer ini: {len(rahasia)}")

masalah = []
print("'client-google.json' di exe mentah:", exe.count(b"client-google.json"))
for r in rahasia + [b"GOCSPX"]:
    if r in exe:
        masalah.append("nilai rahasia ditemukan di exe mentah")

# Cari payload.zip di tengah exe: dari setiap catatan akhir (EOCD, PK\x05\x06) hitung awal zip
# = posisi EOCD - ukuran central directory - offset-nya, lalu potong tepat batas zip itu.
zf = None
pos = exe.find(b"PK\x05\x06")
while pos != -1 and zf is None:
    cd_size = int.from_bytes(exe[pos + 12:pos + 16], "little")
    cd_off = int.from_bytes(exe[pos + 16:pos + 20], "little")
    komentar = int.from_bytes(exe[pos + 20:pos + 22], "little")
    awal = pos - cd_size - cd_off
    if awal >= 0 and exe[awal:awal + 4] == b"PK\x03\x04":
        try:
            z = zipfile.ZipFile(io.BytesIO(exe[awal:pos + 22 + komentar]))
            if z.namelist():
                zf = z
                print(f"payload zip di offset {awal}, {pos + 22 + komentar - awal} bita")
        except zipfile.BadZipFile:
            pass
    pos = exe.find(b"PK\x05\x06", pos + 4)

if zf is None:
    masalah.append("payload zip tidak ditemukan")
else:
    nama = zf.namelist()
    print(f"payload: {len(nama)} berkas: {', '.join(nama)}")
    if any(n.lower().endswith("client-google.json") for n in nama):
        masalah.append("client-google.json ikut di payload")
    for n in nama:
        isi = zf.read(n)
        for i, r in enumerate(rahasia):
            if r in isi:
                masalah.append(f"nilai lokal #{i + 1} (milik komputer ini) ditemukan di {n}")
        # Contoh di dokumentasi resmi rclone bukan rahasia; buang dulu sebelum mencari awalan.
        isi = isi.replace(b"GOCSPX-your-client-secret", b"")
        if b"GOCSPX" in isi:
            # Awalan umum secret Google: tunjukkan konteksnya hanya bila bukan nilai lokal.
            for j in range(isi.count(b"GOCSPX")):
                p = isi.find(b"GOCSPX", 0 if j == 0 else p + 6)
                konteks = isi[max(0, p - 80):p + 60]
                if any(r in isi[p - 1:p + 80] for r in rahasia):
                    continue
                print(f"awalan GOCSPX di {n}, konteks: {konteks!r}")
            masalah.append(f"awalan GOCSPX ada di {n} (lihat konteks)")

if masalah:
    print("GAGAL:", *sorted(set(masalah)), sep="\n - ")
    sys.exit(1)
print("lolos: installer bersih dari client ID/secret Google")
