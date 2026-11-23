"""Ikon LELEMOVE: ikan lele yang melaju (garis gerak), di petak hijau kolam.

Digambar 4x lebih besar lalu diperkecil (antialias), disimpan sebagai .ico multi-ukuran.
Ukuran kecil (<= 32 px) memakai versi sederhana: ikan lebih besar, kumis lebih tebal, tanpa sirip bawah.
Pemakaian: python buat_ikon.py <keluaran.ico> <pratinjau.png>
"""
import math
import sys

from PIL import Image, ImageDraw

S = 4  # supersampling
N = 1024 * S

PETAK = (14, 90, 82)          # hijau kolam tua
LELE = (246, 236, 208)        # krem
GARIS_GERAK = (95, 211, 180)  # mint


def gambar(kecil=False):
    im = Image.new("RGBA", (N, N), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    d.rounded_rectangle(((36 * S, 36 * S), (988 * S, 988 * S)), radius=210 * S, fill=PETAK)

    # Semua koordinat ikan di bawah ini dalam ruang 1024, lalu diskalakan di sekitar titik tengahnya.
    K = 1.22 if kecil else 1.14
    tx, ty = (470, 540) if kecil else (430, 540)  # titik tengah ikan (ruang gambar)

    def p(x, y):
        return (((x - tx) * K + 512) * S, ((y - ty) * K + 512) * S)

    def garis(titik, lebar, warna):
        pts = [p(x, y) for x, y in titik]
        w = lebar * K * S
        d.line(pts, fill=warna, width=int(w), joint="curve")
        r = w / 2
        for x, y in (pts[0], pts[-1]):
            d.ellipse((x - r, y - r, x + r, y + r), fill=warna)

    def bezier(a, b, c, n=48):
        return [((1 - t) ** 2 * a[0] + 2 * (1 - t) * t * b[0] + t * t * c[0],
                 (1 - t) ** 2 * a[1] + 2 * (1 - t) * t * b[1] + t * t * c[1]) for t in (i / n for i in range(n + 1))]

    x0, x1 = 300, 760  # pangkal ekor, ujung moncong

    def ux(u):
        return x0 + (x1 - x0) * u

    def cy(u):
        return 590 - 60 * u + 14 * math.sin(u * math.pi * 1.6)

    def tebal(u):
        if u <= 0.76:
            return 40 + 104 * (u / 0.76) ** 0.8
        k = (u - 0.76) / 0.24
        return 144 * math.sqrt(max(0.0, 1 - k * k))

    def y_atas(u):
        return cy(u) - tebal(u) * 0.90

    def y_bawah(u):
        return cy(u) + tebal(u) * 0.80

    # Sirip anal panjang khas lele: pita yang menempel di perut, tepinya bergelombang.
    if not kecil:
        tepi, dalam = [], []
        for i in range(41):
            u = 0.08 + 0.52 * i / 40
            dalam.append((ux(u), y_bawah(u) - 10))
            tepi.append((ux(u), y_bawah(u) - 6 + (36 + 5 * math.sin(i / 40 * math.pi * 6)) * math.sin(i / 40 * math.pi) ** 0.7))
        d.polygon([p(x, y) for x, y in dalam + tepi[::-1]], fill=LELE)

    # Badan.
    n = 140
    atas = [(ux(i / n), y_atas(i / n)) for i in range(n + 1)]
    bawah = [(ux(i / n), y_bawah(i / n)) for i in range(n + 1)]
    d.polygon([p(x, y) for x, y in atas + bawah[::-1]], fill=LELE)

    # Ekor: kipas membulat.
    cx_e, cy_e = x0 + 20, cy(0)
    kipas = [p(x0 + 24, cy_e - 40)]
    for i in range(41):
        a = math.radians(122 + i * (116 / 40))
        kipas.append(p(cx_e + 150 * math.cos(a), cy_e - 118 * math.sin(a)))
    kipas.append(p(x0 + 24, cy_e + 40))
    d.polygon(kipas, fill=LELE)

    # Sirip punggung.
    d.polygon([p(ux(0.40), y_atas(0.40) + 8), p(ux(0.47), y_atas(0.47) - 78), p(ux(0.58), y_atas(0.58) + 6)], fill=LELE)
    # Sirip dada di bawah kepala.
    xd, yd = ux(0.72), y_bawah(0.72)
    d.polygon([p(xd + 34, yd - 10), p(xd - 64, yd + 70), p(xd - 46, yd - 6)], fill=LELE)

    # Mata.
    xm, ym = ux(0.86), y_atas(0.86) + 46
    r = 19 if not kecil else 24
    d.ellipse((p(xm - r, ym - r), p(xm + r, ym + r)), fill=PETAK)
    if not kecil:
        rr = 6
        d.ellipse((p(xm + 5 - rr, ym - 6 - rr), p(xm + 5 + rr, ym - 6 + rr)), fill=LELE)
        # Mulut lebar di ujung moncong.
        garis(bezier((x1 - 58, cy(1) + 30), (x1 - 30, cy(1) + 44), (x1 - 6, cy(1) + 26)), 10, PETAK)

    # Kumis (sungut): satu melengkung ke atas, satu ke bawah.
    k_tebal = 36 if kecil else 22
    sx, sy = x1 - 16, cy(1) - 4
    jangkau = 0.7 if kecil else 1.0
    garis(bezier((sx, sy - 6), (sx + 118 * jangkau, sy - 28), (sx + 80 * jangkau, sy - 160 * jangkau)), k_tebal, LELE)
    garis(bezier((sx, sy + 14), (sx + 128 * jangkau, sy + 56), (sx + 64 * jangkau, sy + 182 * jangkau)), k_tebal, LELE)

    # Garis gerak di belakang ekor.
    if not kecil:
        y0 = cy(0)
        for (dy, xa, xb) in ((-100, 84, 142), (4, 62, 136), (108, 92, 142)):
            garis([(xa, y0 + dy), (xb, y0 + dy)], 26, GARIS_GERAK)

    return im


def main():
    keluar_ico, pratinjau = sys.argv[1], sys.argv[2]
    besar = gambar().resize((1024, 1024), Image.LANCZOS)
    kecil = gambar(kecil=True).resize((1024, 1024), Image.LANCZOS)
    ukuran = [256, 128, 64, 48, 40, 32, 24, 20, 16]
    bingkai = [(besar if u > 32 else kecil).resize((u, u), Image.LANCZOS) for u in ukuran]
    bingkai[0].save(keluar_ico, format="ICO", sizes=[(u, u) for u in ukuran], append_images=bingkai[1:])

    lembar = Image.new("RGBA", (900, 330), (255, 255, 255, 255))
    lembar.paste(Image.new("RGBA", (900, 165), (32, 32, 32, 255)), (0, 165))
    x = 10
    for u in (256, 64, 48, 32, 24, 16):
        b = bingkai[ukuran.index(u)]
        if u == 256:
            b = b.resize((150, 150), Image.LANCZOS)
        for yy in (8, 173):
            lembar.alpha_composite(b, (x, yy))
        x += (150 if u == 256 else u) + 24
    # Versi 4x dari ukuran kecil, untuk melihat piksel.
    for i, u in enumerate((32, 16)):
        b = bingkai[ukuran.index(u)].resize((u * 4, u * 4), Image.NEAREST)
        lembar.alpha_composite(b, (600 + i * 150, 8))
    lembar.save(pratinjau)
    besar.save(pratinjau.replace(".png", "-1024.png"))


if __name__ == "__main__":
    main()
