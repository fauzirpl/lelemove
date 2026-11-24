#!/bin/sh
# Mengemas LELEMOVE untuk macOS: LELEMOVE.app universal (Apple Silicon + Intel) di dalam
# dist/macos/LELEMOVE-<versi>-macOS.dmg. Dijalankan di Mac (lokal atau GitHub Actions):
#
#   ./buat-dmg.sh                       build + kemas, rclone versi terbaru
#   RCLONE_VERSI=v1.76.0 ./buat-dmg.sh  rclone versi tertentu
#   LEWATI_BUILD=1 ./buat-dmg.sh        pakai hasil build yang sudah ada
#
# Butuh: rustup dengan target aarch64-apple-darwin dan x86_64-apple-darwin, serta Xcode Command
# Line Tools (lipo, codesign, hdiutil). Client ID Google TIDAK pernah dibundel di sini.
#
# Isi bundel:
#   Contents/MacOS/LELEMOVE      jendela aplikasi (bin pindah-drive)
#   Contents/MacOS/pindah        mesin + baris perintah
#   Contents/MacOS/rclone        rclone resmi (diunduh dari downloads.rclone.org, SHA-256 dicek)
#   Contents/Resources/LELEMOVE.icns, rclone-README.txt
#
# Aplikasi ditandatangani ad-hoc (tanpa Apple Developer ID), jadi Gatekeeper menahannya saat
# pertama dibuka: Pengaturan Sistem > Privasi & Keamanan > Buka Saja (lihat README).
set -eu

cd "$(dirname "$0")"
AKAR=$(pwd)
VERSI=$(sed -n 's/^version = "\(.*\)"$/\1/p' rust/Cargo.toml | head -n 1)
DIST="$AKAR/dist/macos"
APP="$DIST/LELEMOVE.app"
KERJA="$DIST/kerja"
TARGET_ARM=aarch64-apple-darwin
TARGET_INTEL=x86_64-apple-darwin
export MACOSX_DEPLOYMENT_TARGET=11.0

echo "LELEMOVE $VERSI untuk macOS"

# ------------------------------------------------------------------ 1. program
if [ -z "${LEWATI_BUILD:-}" ]; then
    echo "1/5 Build Apple Silicon dan Intel"
    for t in "$TARGET_ARM" "$TARGET_INTEL"; do
        cargo build --manifest-path rust/Cargo.toml --locked --release --target "$t" --bin pindah --bin pindah-drive
    done
else
    echo "1/5 Build dilewati (LEWATI_BUILD)"
fi

rm -rf "$DIST"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" "$KERJA"
gabung() { # gabung <bin> <nama di bundel>
    lipo -create -output "$APP/Contents/MacOS/$2" \
        "rust/target/$TARGET_ARM/release/$1" "rust/target/$TARGET_INTEL/release/$1"
}
gabung pindah-drive LELEMOVE
gabung pindah pindah

# ------------------------------------------------------------------ 2. rclone
echo "2/5 rclone"
RV=${RCLONE_VERSI:-$(curl -fsSL --retry 5 https://downloads.rclone.org/version.txt | awk '{print $2}')}
case "$RV" in v[0-9]*) ;; *) echo "Versi rclone tidak dikenali: '$RV'" >&2; exit 1 ;; esac
echo "    $RV"
(
    cd "$KERJA"
    for a in arm64 amd64; do
        curl -fsSL --retry 5 -o "rclone-$RV-osx-$a.zip" "https://downloads.rclone.org/$RV/rclone-$RV-osx-$a.zip"
    done
    curl -fsSL --retry 5 -o SHA256SUMS "https://downloads.rclone.org/$RV/SHA256SUMS"
    grep -E "  rclone-$RV-osx-(arm64|amd64)\.zip" SHA256SUMS | tr -d '\r' > cek.txt
    [ "$(wc -l < cek.txt | tr -d ' ')" = 2 ] || { echo "SHA256SUMS tidak memuat kedua berkas" >&2; exit 1; }
    shasum -a 256 -c cek.txt
    for a in arm64 amd64; do unzip -q "rclone-$RV-osx-$a.zip"; done
)
lipo -create -output "$APP/Contents/MacOS/rclone" \
    "$KERJA/rclone-$RV-osx-arm64/rclone" "$KERJA/rclone-$RV-osx-amd64/rclone"
cp "$KERJA/rclone-$RV-osx-arm64/README.txt" "$APP/Contents/Resources/rclone-README.txt"
chmod 755 "$APP/Contents/MacOS/"*

# ------------------------------------------------------------------ 3. bundel
echo "3/5 Info.plist dan ikon"
cp aset/ikon.icns "$APP/Contents/Resources/LELEMOVE.icns"
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key><string>id</string>
  <key>CFBundleName</key><string>LELEMOVE</string>
  <key>CFBundleDisplayName</key><string>LELEMOVE</string>
  <key>CFBundleExecutable</key><string>LELEMOVE</string>
  <key>CFBundleIconFile</key><string>LELEMOVE</string>
  <key>CFBundleIdentifier</key><string>io.github.fauzirpl.lelemove</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$VERSI</string>
  <key>CFBundleVersion</key><string>$VERSI</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.utilities</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSHumanReadableCopyright</key><string>LELEMOVE memakai rclone ($RV), hak cipta Nick Craig-Wood dan para kontributor rclone.</string>
  <key>NSDesktopFolderUsageDescription</key><string>LELEMOVE menyalin folder di Desktop yang Anda pilih.</string>
  <key>NSDocumentsFolderUsageDescription</key><string>LELEMOVE menyalin folder di Dokumen yang Anda pilih.</string>
  <key>NSDownloadsFolderUsageDescription</key><string>LELEMOVE menyalin folder di Unduhan yang Anda pilih.</string>
  <key>NSRemovableVolumesUsageDescription</key><string>LELEMOVE menyalin folder di disk eksternal yang Anda pilih.</string>
  <key>NSNetworkVolumesUsageDescription</key><string>LELEMOVE menyalin folder di volume jaringan yang Anda pilih.</string>
</dict>
</plist>
PLIST
plutil -lint "$APP/Contents/Info.plist"

# ------------------------------------------------------------------ 4. tanda tangan ad-hoc
echo "4/5 Tanda tangan ad-hoc"
for b in rclone pindah LELEMOVE; do
    codesign --force --sign - --timestamp=none "$APP/Contents/MacOS/$b"
done
codesign --force --sign - --timestamp=none "$APP"
codesign --verify --deep --strict "$APP"
lipo -archs "$APP/Contents/MacOS/LELEMOVE"

# ------------------------------------------------------------------ 5. dmg
echo "5/5 .dmg"
ISI="$KERJA/isi-dmg"
mkdir -p "$ISI"
cp -R "$APP" "$ISI/"
ln -s /Applications "$ISI/Applications"
DMG="$DIST/LELEMOVE-$VERSI-macOS.dmg"
# hdiutil kadang gagal "Resource busy" sesaat setelah build; ulangi beberapa kali.
n=0
until hdiutil create -volname "LELEMOVE $VERSI" -srcfolder "$ISI" -fs HFS+ -format UDZO -ov "$DMG" >/dev/null; do
    n=$((n + 1))
    [ "$n" -lt 4 ] || { echo "hdiutil gagal" >&2; exit 1; }
    sleep 5
done
rm -rf "$KERJA"
( cd "$DIST" && shasum -a 256 "$(basename "$DMG")" | tee "$(basename "$DMG").sha256" )
echo "Selesai: $DMG"
