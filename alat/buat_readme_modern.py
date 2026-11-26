"""Buat README-ejaan-sekarang.md dari README.md berejaan zaman perjuangan.

Kata dibalik (oe->u, tj->c, dj->j, nj->ny, sj->sy, ch->kh, j->y) hanya bila hasilnya ada di kosakata
modern (README lama sebelum ejaan 1945 + dokumen berejaan sekarang); kata lain dibiarkan, dan kata yang
tidak dikenal dilaporkan. Blok kode, kode sebaris, URL, dan tag HTML tidak disentuh.
Pemakaian: python buat_readme_modern.py <repo> [--tulis]
"""
import re
import subprocess
import sys
import unicodedata
from pathlib import Path

repo = Path(sys.argv[1])
tulis = "--tulis" in sys.argv
teks = (repo / "README.md").read_text(encoding="utf-8")


def git(*a):
    return subprocess.run(["git", "-C", str(repo), *a], capture_output=True, check=True).stdout.decode("utf-8")


KATA = re.compile(r"[A-Za-zÀ-ÿ]+")
kosakata = set()
sumber = [git("show", "dfc4d64:README.md")]
for p in list((repo / "docs").glob("*.md")) + list((repo / "dist" / "rilis").glob("catatan-rilis-*.md")) + list((repo / "catatan-rilis").glob("*.md")) + [repo / "promo" / "README.md", repo / "alat" / "README.md", repo / "CONTRIBUTING.md", repo / "SECURITY.md"]:
    if p.exists():
        sumber.append(p.read_text(encoding="utf-8"))
for s in sumber:
    kosakata.update(w.lower() for w in KATA.findall(s))
# Kata modern yang belum ada di sumber mana pun.
kosakata.update("""narasi mengikuti sumbernya beta lolos otomatis menemukan masalah laporkan sungguhan
diuji dipakai login banyak periksa dulu jalankan jendela jaringan percobaan landjut lanjut yakni bunyi
pemicu picu berlaku muncul dimuat huruf pratinjau tersembunyi pemeliharaan sukarela dukungan yaitu
ditulis sesudahnya ejaan rekan""".split())

POLA_BLOK = re.compile(r"^```[^\n]*\n.*?^```[ \t]*$", re.M | re.S)
# Potongan yang tidak boleh diubah: kode sebaris, target tautan, autolink, tag HTML, URL telanjang.
LINDUNG = re.compile(r"`[^`\n]*`|\]\([^)]*\)|<https?://[^>]+>|<[^>\n]+>|https?://\S+")

tidak_dikenal = {}
ambigu = {}
# Kata berejaan sekarang yang bentuk baliknya juga kata nyata (mis. label tombol macOS "Buka Saja"
# jangan menjadi "Buka Saya"). Kata lain yang ambigu dilaporkan agar diputuskan manual.
TETAP = {"saja"}


def balik(kata):
    k = kata.lower()
    k = k.replace("oe", "u").replace("tj", "c").replace("dj", "\x01").replace("nj", "ny")
    k = k.replace("sj", "sy").replace("ch", "kh").replace("j", "y").replace("\x01", "j")
    return k


def bentuk(asli, baru):
    if asli.isupper() and len(asli) > 1:
        return baru.upper()
    if asli[0].isupper():
        return baru[0].upper() + baru[1:]
    return baru


def ganti_kata(m):
    w = m.group(0)
    calon = balik(w)
    if calon == w.lower():
        if w.lower() not in kosakata:
            tidak_dikenal[w] = tidak_dikenal.get(w, 0) + 1
        return w
    if w.lower() in TETAP:
        return w
    if calon in kosakata:
        if w.lower() in kosakata:
            ambigu[f"{w}->{calon}"] = ambigu.get(f"{w}->{calon}", 0) + 1
        return bentuk(w, calon)
    if w.lower() in kosakata:
        return w
    tidak_dikenal[w] = tidak_dikenal.get(w, 0) + 1
    return bentuk(w, calon)


def ubah_prosa(s):
    hasil, pos = [], 0
    for m in LINDUNG.finditer(s):
        hasil.append(KATA.sub(ganti_kata, s[pos:m.start()]))
        hasil.append(m.group(0))
        pos = m.end()
    hasil.append(KATA.sub(ganti_kata, s[pos:]))
    return "".join(hasil)


# Pisahkan blok kode; ubah hanya prosa.
bagian, pos = [], 0
for m in POLA_BLOK.finditer(teks):
    bagian.append(ubah_prosa(teks[pos:m.start()]))
    bagian.append(m.group(0))
    pos = m.end()
bagian.append(ubah_prosa(teks[pos:]))
modern = "".join(bagian)

# Kata/frasa lama yang maknanya bergeser atau terasa kuno.
FRASA = [
    ("LELEMOVE percuma; bila terbantu", "LELEMOVE gratis; bila terbantu"),
    ("LELEMOVE percuma dan sumbernya terbuka", "LELEMOVE gratis dan sumbernya terbuka"),
    ("(percuma, cukup dibuat sekali)", "(gratis, cukup dibuat sekali)"),
    ("Percuma, kuotanya khusus untuk Saudara", "Gratis, kuotanya khusus untuk Anda"),
    ("Sebabnya: rclone", "Penyebabnya: rclone"),
    ("Jalan keluarnya ialah client ID sendiri.", "Solusinya adalah client ID sendiri."),
    ("Timbanglah sebelum membagikan", "Pertimbangkan sebelum membagikan"),
    ("traktirlah pengembangnya", "traktir pengembangnya"),
    ("berilah izin penuh", "beri izin penuh"),
    ("Google boleh jadi menampilkan", "Google bisa menampilkan"),
    ("sokongan lewat", "dukungan lewat"),
    ("Sokongan bersifat", "Dukungan bersifat"),
    ("dapat pula dijalankan", "juga bisa dijalankan"),
    ("dilanjutkan esok hari", "dilanjutkan besok"),
    ("lanjutkan esok hari", "lanjutkan besok"),
    ("gulung ke bawah", "gulir ke bawah"),
    ("boleh jadi", "bisa"),
    ("Film perkenalan** (54 detik, dengan suara pencerita;", "Video perkenalan** (54 detik, dengan narasi;"),
    ("naskah README berëjaan zaman perjuangan", "README edisi ejaan zaman perjuangan"),
]
for lama, baru in FRASA:
    modern = modern.replace(lama, baru)
for lama, baru in [("Saudara", "Anda"), ("ialah", "adalah"), ("kerap", "sering")]:
    modern = re.sub(rf"\b{lama}\b", baru, modern)

# Buang bagian khas edisi 1945, ganti tautan di atas dengan keterangan.
modern = re.sub(r"\*Susah membaca ejaan lama\?[^\n]*\n", "*Versi ejaan sekarang dari [README.md](README.md), yang ditulis dengan ejaan zaman perjuangan. Isinya sama. English: [README.en.md](README.en.md).*\n", modern)
modern = re.sub(r"> \*\*MAKLUMAT\*\*\n(?:>[^\n]*\n)+\n", "", modern)
modern = re.sub(r"\*Catatan ejaan:[^*]+\*\n\n", "", modern)
modern = modern.replace("Sekali gratis, tetap gratis!\n\n", "").replace("Sekali percuma, tetap percuma!\n\n", "")
modern = re.sub(r'<a name="[^"]+"></a>\n', "", modern)


# Jangkar: petakan jangkar judul edisi 1945 -> judul modern menurut urutan.
def jangkar(judul):
    j = judul.strip().lower().replace("`", "")
    j = "".join(c for c in j if c in " -_" or unicodedata.category(c)[0] in "LN")
    return j.replace(" ", "-")


def judul_judul(md):
    return re.findall(r"^#{1,6} (.+)$", POLA_BLOK.sub("", md), re.M)


lama_j, baru_j = judul_judul(teks), judul_judul(modern)
assert len(lama_j) == len(baru_j), (len(lama_j), len(baru_j))
peta = {jangkar(a): jangkar(b) for a, b in zip(lama_j, baru_j)}
modern = re.sub(r"\]\(#([^)]+)\)", lambda m: f"]](#{peta.get(m.group(1), m.group(1))})"[1:], modern)

# Periksa.
masalah = []
if POLA_BLOK.findall(modern) != POLA_BLOK.findall(teks):
    masalah.append("blok kode berubah")
ada = {jangkar(b) for b in baru_j}
for t in set(re.findall(r"\]\(#([^)]+)\)", POLA_BLOK.sub("", modern))):
    if t not in ada:
        masalah.append(f"tautan rusak #{t}")
sisa = sorted({w for w in KATA.findall(LINDUNG.sub(" ", POLA_BLOK.sub("", modern)))
               if re.search(r"oe|dj|tj|nj|sj", w.lower()) and w.lower() not in kosakata})
for kuno in ("percuma", "Saudara", "esok hari", "MAKLUMAT", "Merdeka", "berëjaan", "pencerita"):
    if kuno in modern:
        masalah.append(f"masih ada '{kuno}'")
print("kata tak dikenal (sebelum frasa):", ", ".join(f"{k}x{v}" for k, v in sorted(tidak_dikenal.items())) or "-")
print("sisa ejaan lama:", ", ".join(sisa) or "-")
print("AMBIGU (asli & hasil sama-sama kata nyata):", ", ".join(f"{k}x{v}" for k, v in sorted(ambigu.items())) or "-")
print("judul:", " | ".join(baru_j[:12]), "...")
if masalah:
    print("MASALAH:", masalah)
    sys.exit(1)
if tulis:
    (repo / "README-ejaan-sekarang.md").write_bytes(modern.encode("utf-8"))
    print("README-ejaan-sekarang.md ditulis")
