"""Musik latar video promosi LELEMOVE: disintesis dari nol (tanpa sampel, bebas hak cipta).

D mayor, 96 BPM, progresi D - Bm - G - A. Titik-titiknya mengikuti linimasa adegan (src/linimasa.json,
dibuat alat/buat-suara.mjs dari panjang narasi):
  0 - LOGO     pad saja, mengembang, riser 1,5 dtk sebelum LOGO (adegan Pembuka)
  LOGO         dentum + arpeggio (logo muncul)
  DRUM - AKHIR kick, hi-hat, bass (adegan Layanan sampai sebelum Penutup)
  AKHIR - DUR  drum berhenti, akor akhir memudar 3 dtk terakhir
Pemakaian (dari folder promo): python alat/buat_musik.py src/linimasa.json public/musik.wav
"""
import json
import sys
import wave

import numpy as np

lini = json.load(open(sys.argv[1], encoding="utf-8"))
FPS = lini["fps"]
mulai = {a["id"]: a["mulai"] / FPS for a in lini["adegan"]}
DUR = lini["total"] / FPS
LOGO = mulai["logo"]
DRUM = mulai["layanan"]
AKHIR = mulai["penutup"]

SR = 44100
BPM = 96
KETUK = 60 / BPM
BIRAMA = 4 * KETUK
N = int(SR * DUR)
t = np.arange(N) / SR
rng = np.random.default_rng(7)

AKOR = [[50, 54, 57, 62], [47, 50, 54, 59], [43, 47, 50, 55], [45, 49, 52, 57]]  # D Bm G A
AKAR = [38, 35, 31, 33]


def hz(m):
    return 440.0 * 2 ** ((m - 69) / 12)


def jendela(mulai_, panjang):
    i0, i1 = int(mulai_ * SR), min(N, int((mulai_ + panjang) * SR))
    return i0, i1


def kirikanan(sinyal, pan):
    return np.stack([sinyal * (1 - pan) ** 0.5, sinyal * (1 + pan) ** 0.5], axis=1) / 2 ** 0.5


kiri_kanan = np.zeros((N, 2))

# ---- pad: harmonik lembut, dua suara sedikit detune, crossfade antar birama
pad = np.zeros(N)
jumlah_birama = int(np.ceil(DUR / BIRAMA))
for b in range(jumlah_birama):
    awal = b * BIRAMA
    i0, i1 = jendela(awal, BIRAMA + 0.8)
    tt = t[i0:i1] - awal
    env = np.minimum(1, tt / 0.6) * np.clip((BIRAMA + 0.8 - tt) / 0.8, 0, 1)
    for m in AKOR[b % 4]:
        for det in (-0.0015, 0.0015):
            f = hz(m) * (1 + det)
            gel = sum(np.sin(2 * np.pi * f * k * tt) / k ** 1.6 for k in range(1, 6))
            pad[i0:i1] += gel * env * 0.05
# Pembuka lebih redup lalu naik.
pad *= np.interp(t, [0, LOGO, LOGO + 0.5, AKHIR, DUR], [0.35, 0.8, 1.0, 1.0, 0.9])

# ---- kick (empat ketuk) DRUM - AKHIR, juga dipakai untuk "sidechain" pad
kick = np.zeros(N)
sc = np.ones(N)
tk = DRUM
while tk < AKHIR - 0.01:
    i0, i1 = jendela(tk, 0.35)
    tt = t[i0:i1] - tk
    f = 45 + 70 * np.exp(-tt / 0.03)
    fase = 2 * np.pi * np.cumsum(f) / SR
    kick[i0:i1] += np.sin(fase) * np.exp(-tt / 0.12) * 0.55
    sc[i0:i1] = np.minimum(sc[i0:i1], 1 - 0.35 * np.exp(-tt / 0.18))
    tk += KETUK
pad *= sc
kiri_kanan += kirikanan(pad, 0.0)
kiri_kanan += kirikanan(kick, 0.0)

# ---- hi-hat di ketuk lemah
hat = np.zeros(N)
th = DRUM + KETUK / 2
while th < AKHIR - 0.01:
    i0, i1 = jendela(th, 0.08)
    tt = t[i0:i1] - th
    bising = rng.standard_normal(i1 - i0)
    bising = np.diff(np.concatenate([[0], bising]))  # tinggi saja
    hat[i0:i1] += bising * np.exp(-tt / 0.018) * 0.05
    th += KETUK
kiri_kanan += kirikanan(hat, 0.25)

# ---- bass: akar akor di ketuk 1 dan 3, LOGO - AKHIR
bass = np.zeros(N)
for b in range(jumlah_birama):
    for ket in (0, 2, 3.5):
        tb = b * BIRAMA + ket * KETUK
        if not (LOGO <= tb < AKHIR):
            continue
        i0, i1 = jendela(tb, 0.6)
        tt = t[i0:i1] - tb
        f = hz(AKAR[b % 4])
        bass[i0:i1] += np.tanh(1.6 * np.sin(2 * np.pi * f * tt)) * np.exp(-tt / 0.28) * (0.22 if ket != 3.5 else 0.12)
kiri_kanan += kirikanan(bass, 0.0)

# ---- arpeggio petik, seperdelapan, kiri-kanan bergantian, mulai LOGO
pola = [0, 1, 2, 3, 2, 1, 2, 3]
ta = LOGO
langkah = 0
while ta < DUR - 2.0:
    b = int(ta // BIRAMA)
    akor = AKOR[b % 4]
    m = (akor + [akor[0] + 12])[pola[langkah % 8]] + 12
    i0, i1 = jendela(ta, 0.6)
    tt = t[i0:i1] - ta
    f = hz(m)
    not_ = (np.sin(2 * np.pi * f * tt) + 0.3 * np.sin(4 * np.pi * f * tt)) * np.exp(-tt / 0.16) * 0.07
    kiri_kanan[i0:i1] += kirikanan(not_, -0.35 if langkah % 2 == 0 else 0.35)
    ta += KETUK / 2
    langkah += 1

# ---- riser 1,5 dtk sebelum LOGO lalu dentum di LOGO
i0, i1 = jendela(LOGO - 1.5, 1.5)
naik = np.linspace(0, 1, i1 - i0) ** 2.5
bising = np.diff(np.concatenate([[0], rng.standard_normal(i1 - i0)]))
kiri_kanan[i0:i1] += kirikanan(bising * naik * 0.05, 0.0)
i0, i1 = jendela(LOGO, 2.0)
tt = t[i0:i1] - LOGO
dentum = np.sin(2 * np.pi * (40 + 30 * np.exp(-tt / 0.08)) * tt) * np.exp(-tt / 0.7) * 0.6
kiri_kanan[i0:i1] += kirikanan(dentum, 0.0)

# ---- akhir: pudar 3 detik terakhir, batasi puncak
kiri_kanan *= np.interp(t, [0, 0.4, DUR - 3.0, DUR], [0, 1, 1, 0])[:, None]
kiri_kanan = np.tanh(kiri_kanan * 1.2)
kiri_kanan *= 0.89 / np.max(np.abs(kiri_kanan))

data = (kiri_kanan * 32767).astype("<i2").tobytes()
with wave.open(sys.argv[2], "wb") as w:
    w.setnchannels(2)
    w.setsampwidth(2)
    w.setframerate(SR)
    w.writeframes(data)
print(f"selesai {DUR:.2f} dtk (logo {LOGO:.2f}, drum {DRUM:.2f}-{AKHIR:.2f})")
