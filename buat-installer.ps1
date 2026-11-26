<#
.SYNOPSIS
    Membuat installer LELEMOVE (dulu Pindah Drive): dist\LELEMOVE-Setup-<versi>.exe

.DESCRIPTION
    1. Mengompilasi crate rust\ (pindah.exe, LELEMOVE.exe, uninstall.exe, tanam-ikon.exe)
       dengan toolchain Rust GNU (rustup, per pengguna).
    2. Menanam ikon & manifest ke setiap .exe (tanam-ikon.exe; toolchain GNU tidak membawa windres).
    3. Menyusun isi aplikasi + rclone.exe ke payload.zip.
    4. Mengompilasi ulang setup.exe dengan payload.zip tertanam (PINDAH_PAYLOAD).

        powershell -ExecutionPolicy Bypass -File .\buat-installer.ps1 [-Rclone C:\path\rclone.exe] [-TanpaClient]
#>
param(
    # rclone.exe yang ikut dibundel; bawaan: yang dipakai di komputer ini.
    [string]$Rclone,

    # Client ID Google bawaan yang ikut dibundel (disimpan aplikasi lewat "Jadikan bawaan").
    [string]$ClientGoogle = (Join-Path $env:LOCALAPPDATA 'pindah-drive\client-google.json'),
    # Jangan bundel client ID (installer untuk dibagikan ke publik).
    [switch]$TanpaClient
)
$ErrorActionPreference = 'Stop'
$akar = $PSScriptRoot
$crate = Join-Path $akar 'rust'
$rilis = Join-Path $crate 'target\release'

# Versi diambil dari rust\Cargo.toml (satu sumber kebenaran).
$Versi = ([regex]::Match((Get-Content (Join-Path $crate 'Cargo.toml') -Raw), '(?m)^version\s*=\s*"([^"]+)"')).Groups[1].Value

Write-Host "1/5 Toolchain Rust (versi aplikasi $Versi)"
$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
$tc = Get-ChildItem (Join-Path $env:USERPROFILE '.rustup\toolchains') -Directory -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -like 'stable-*windows-gnu' } | Select-Object -First 1
if (-not (Test-Path (Join-Path $cargoBin 'cargo.exe')) -or -not $tc) {
    throw 'Toolchain Rust GNU tidak ditemukan. Pasang: winget install Rustlang.Rustup --custom "--default-host x86_64-pc-windows-gnu --profile minimal"'
}
# dlltool & linker GNU bawaan rustup ada di folder self-contained.
$env:Path = "$cargoBin;$($tc.FullName)\lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained;$env:Path"
Remove-Item Env:\PINDAH_PAYLOAD -ErrorAction SilentlyContinue

function Jalankan-Cargo([string[]]$argumen) {
    # cargo menulis kemajuan ke stderr; dengan 'Stop', PowerShell 5.1 menganggapnya galat.
    $ErrorActionPreference = 'Continue'
    Push-Location $crate
    try {
        # --offline bila paket sudah di cache; bila belum, unduh (DNS di jaringan ini kadang putus).
        $env:CARGO_NET_RETRY = '20'
        & (Join-Path $cargoBin 'cargo.exe') @argumen 2>&1 | ForEach-Object { "$_" } |
            Where-Object { $_ -match '^(error|warning: unused)|Finished|Compiling pindah' } | ForEach-Object { Write-Host "    $_" }
        if ($LASTEXITCODE -ne 0) { throw "cargo $($argumen -join ' ') gagal (kode $LASTEXITCODE)." }
    }
    finally { Pop-Location }
}

Write-Host '2/5 Kompilasi'
# Daftar pustaka pihak ketiga (nama versi|lisensi|repositori) untuk dialog Bantuan > Atribusi dan
# PIHAK-KETIGA.txt. Dibuat dari pohon dependensi yang benar-benar ikut di build Windows.
$atribusi = Join-Path $crate 'aset\atribusi.txt'
Push-Location $crate
try {
    $ErrorActionPreference = 'Continue'
    $baris = & (Join-Path $cargoBin 'cargo.exe') tree --offline --target x86_64-pc-windows-gnu -e normal --prefix none --format '{p}|{l}|{r}' 2>$null |
        ForEach-Object { "$_" -replace ' \(\*\)$', '' -replace ' \(proc-macro\)', '' } |
        Where-Object { $_ -and $_ -notmatch '^pindah-drive ' } | Sort-Object -Unique
    $ErrorActionPreference = 'Stop'
}
finally { Pop-Location }
if (@($baris).Count -lt 20) { throw 'cargo tree tidak menghasilkan daftar atribusi.' }
[IO.File]::WriteAllText($atribusi, (($baris -join "`n") + "`n"), (New-Object Text.UTF8Encoding $false))
Write-Host ('    atribusi: {0} pustaka' -f @($baris).Count)
Jalankan-Cargo @('build', '--release')
$ikon = Join-Path $akar 'aset\ikon.ico'
$manifest = Join-Path $crate 'aset\app.manifest'
$tanam = Join-Path $rilis 'tanam-ikon.exe'

Write-Host '3/5 Mencari rclone.exe'
if (-not $Rclone) {
    foreach ($c in @(
            (Join-Path $akar 'rclone.exe'),
            (Join-Path $env:LOCALAPPDATA 'Microsoft\WinGet\Links\rclone.exe'),
            (Join-Path $env:LOCALAPPDATA 'Programs\LELEMOVE\rclone.exe'),
            (Join-Path $env:LOCALAPPDATA 'Programs\Pindah Drive\rclone.exe'))) {
        if (Test-Path -LiteralPath $c) { $Rclone = $c; break }
    }
    if (-not $Rclone) {
        $g = Get-Command rclone -ErrorAction SilentlyContinue
        if ($g) { $Rclone = $g.Source }
    }
}
if (-not $Rclone -or -not (Test-Path -LiteralPath $Rclone)) { throw 'rclone.exe tidak ditemukan. Beri -Rclone <path>.' }
# Link winget adalah symlink; salin berkas aslinya.
$item = Get-Item -LiteralPath $Rclone
if ($item.LinkType -and $item.Target) { $Rclone = @($item.Target)[0] }
$versiRclone = ((& $Rclone version 2>$null) | Select-Object -First 1)
Write-Host "    $Rclone ($versiRclone)"

Write-Host '4/5 Menyusun isi aplikasi'
$kerja = Join-Path $env:TEMP ('pindah-drive-build-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
$isi = Join-Path $kerja 'isi'
New-Item -ItemType Directory -Path (Join-Path $isi 'aset') -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $rilis 'pindah-drive.exe') -Destination (Join-Path $isi 'LELEMOVE.exe')
Copy-Item -LiteralPath (Join-Path $rilis 'pindah.exe') -Destination (Join-Path $isi 'pindah.exe')
# setup.exe tanpa muatan = uninstall.exe
Copy-Item -LiteralPath (Join-Path $rilis 'setup.exe') -Destination (Join-Path $isi 'uninstall.exe')
foreach ($exe in 'LELEMOVE.exe', 'pindah.exe', 'uninstall.exe') {
    & $tanam (Join-Path $isi $exe) --ikon $ikon --manifest $manifest | ForEach-Object { Write-Host "    $_" }
    if ($LASTEXITCODE -ne 0) { throw "tanam-ikon gagal untuk $exe" }
}
Copy-Item -LiteralPath $ikon -Destination (Join-Path $isi 'aset')
foreach ($f in 'Pindah.cmd', 'README.md', 'LICENSE') {
    $p = Join-Path $akar $f
    if (Test-Path -LiteralPath $p) { Copy-Item -LiteralPath $p -Destination $isi }
}
Copy-Item -LiteralPath $Rclone -Destination (Join-Path $isi 'rclone.exe')
if (-not $TanpaClient -and $ClientGoogle -and (Test-Path -LiteralPath $ClientGoogle)) {
    $c = Get-Content -LiteralPath $ClientGoogle -Raw | ConvertFrom-Json
    if ($c.ClientId -and $c.ClientSecret) {
        Copy-Item -LiteralPath $ClientGoogle -Destination (Join-Path $isi 'client-google.json')
        Write-Host '    client ID Google bawaan IKUT dibundel (siapa pun pemegang installer bisa membacanya)'
    }
}
else { Write-Host '    tanpa client ID Google bawaan' }
$readmeRclone = Join-Path (Split-Path -Parent $Rclone) 'README.txt'
if (Test-Path -LiteralPath $readmeRclone) { Copy-Item -LiteralPath $readmeRclone -Destination (Join-Path $isi 'rclone-README.txt') }
@"
LELEMOVE menyertakan program pihak ketiga berikut:

rclone ($versiRclone)
  https://rclone.org
  Hak cipta Nick Craig-Wood dan para kontributor rclone.
  Dirilis dengan lisensi MIT. Teks lisensi lengkap:
  https://github.com/rclone/rclone/blob/master/COPYING
  Dokumentasi rclone yang menyertai berkas ini: rclone-README.txt

LELEMOVE sendiri ditulis dalam Rust dan memakai pustaka open source dari crates.io berikut
(nama versi | lisensi | repositori). Daftar yang sama ada di aplikasi: Bantuan & info > Atribusi.

"@ + ((Get-Content -LiteralPath $atribusi -Encoding UTF8 | ForEach-Object { '  ' + ($_ -replace '\|', '  |  ') }) -join "`r`n") + @"


Google Drive adalah merek dagang Google LLC. Microsoft, OneDrive, dan SharePoint adalah merek
dagang Microsoft Corporation. LELEMOVE tidak berafiliasi dengan Google, Microsoft, maupun Lalamove.
"@ | Set-Content -LiteralPath (Join-Path $isi 'PIHAK-KETIGA.txt') -Encoding UTF8

Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = Join-Path $kerja 'payload.zip'
[System.IO.Compression.ZipFile]::CreateFromDirectory($isi, $zip, [System.IO.Compression.CompressionLevel]::Optimal, $false)
Write-Host ('    payload.zip {0:N1} MB' -f ((Get-Item $zip).Length / 1MB))

Write-Host '5/5 Mengompilasi installer'
$env:PINDAH_PAYLOAD = $zip
try { Jalankan-Cargo @('build', '--release', '--bin', 'setup') }
finally { Remove-Item Env:\PINDAH_PAYLOAD -ErrorAction SilentlyContinue }
$dist = Join-Path $akar 'dist'
New-Item -ItemType Directory -Path $dist -Force | Out-Null
$keluar = Join-Path $dist "LELEMOVE-Setup-$Versi.exe"
Copy-Item -LiteralPath (Join-Path $rilis 'setup.exe') -Destination $keluar -Force
& $tanam $keluar --ikon $ikon --manifest $manifest | ForEach-Object { Write-Host "    $_" }
if ($LASTEXITCODE -ne 0) { throw 'tanam-ikon gagal untuk installer' }
Remove-Item -LiteralPath $kerja -Recurse -Force
Write-Host ('Selesai: {0} ({1:N1} MB)' -f $keluar, ((Get-Item $keluar).Length / 1MB))
