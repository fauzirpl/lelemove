<#
.SYNOPSIS
    ARSIP versi 1.x (PowerShell + WPF + C#). Tidak dipakai lagi sejak versi 2 (Rust);
    installer baru dibuat oleh ..\buat-installer.ps1.

    Membuat installer: dist\PindahDrive-Setup-<versi>.exe

.DESCRIPTION
    1. Membuat ulang ikon & "Pindah Drive.exe" (buat-aplikasi.ps1).
    2. Menyusun isi aplikasi + rclone.exe ke sebuah ZIP.
    3. Mengompilasi installer\Setup.cs dengan ZIP itu sebagai resource (csc.exe bawaan
       .NET Framework 4.x; tidak perlu memasang apa pun).

    Catatan arsip: skrip ini dipindah dari akar proyek ke arsip-v1-powershell\, sehingga folder
    aset\ dan dist\ kini ada satu tingkat di atasnya ($akarProyek).

        powershell -ExecutionPolicy Bypass -File .\buat-installer-v1.ps1 [-Versi 1.2.0] [-Rclone C:\path\rclone.exe]
#>
param(
    [string]$Versi = '1.2.0',
    # rclone.exe yang ikut dibundel; bawaan: yang dipakai di komputer ini.
    [string]$Rclone,

    # Client ID Google bawaan yang ikut dibundel (disimpan aplikasi lewat "Jadikan bawaan").
    [string]$ClientGoogle = (Join-Path $env:LOCALAPPDATA 'pindah-drive\client-google.json'),
    # Jangan bundel client ID (installer untuk dibagikan ke publik).
    [switch]$TanpaClient
)
$ErrorActionPreference = 'Stop'
$akar = $PSScriptRoot
$akarProyek = Split-Path -Parent $akar

Write-Host '1/4 Ikon & peluncur'
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $akar 'buat-aplikasi.ps1')
if ($LASTEXITCODE -ne 0) { throw 'buat-aplikasi.ps1 gagal.' }

Write-Host '2/4 Mencari rclone.exe'
if (-not $Rclone) {
    foreach ($c in @(
            (Join-Path $akar 'rclone.exe'),
            (Join-Path $env:LOCALAPPDATA 'Microsoft\WinGet\Links\rclone.exe'))) {
        if (Test-Path -LiteralPath $c) { $Rclone = $c; break }
    }
    if (-not $Rclone) {
        $g = Get-Command rclone -ErrorAction SilentlyContinue
        if ($g) { $Rclone = $g.Source }
    }
}
if (-not $Rclone -or -not (Test-Path -LiteralPath $Rclone)) { throw 'rclone.exe tidak ditemukan. Pasang dulu (pindah.ps1 pasang) atau beri -Rclone <path>.' }
# Link winget adalah symlink; salin berkas aslinya.
$item = Get-Item -LiteralPath $Rclone
if ($item.LinkType -and $item.Target) { $Rclone = @($item.Target)[0] }
$versiRclone = ((& $Rclone version 2>$null) | Select-Object -First 1)
Write-Host "    $Rclone ($versiRclone)"

Write-Host '3/4 Menyusun isi aplikasi'
$kerja = Join-Path $env:TEMP ('pindah-drive-build-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
$isi = Join-Path $kerja 'isi'
New-Item -ItemType Directory -Path (Join-Path $isi 'aset') -Force | Out-Null
foreach ($f in 'Pindah Drive.exe', 'gui.ps1', 'pindah.ps1', 'tampilan.xaml') {
    Copy-Item -LiteralPath (Join-Path $akar $f) -Destination $isi
}
foreach ($f in 'Pindah.cmd', 'README.md') {
    $p = Join-Path $akarProyek $f
    if (Test-Path -LiteralPath $p) { Copy-Item -LiteralPath $p -Destination $isi }
}
foreach ($f in 'ikon.ico', 'tangkapan-layar.png') {
    $p = Join-Path $akarProyek "aset\$f"
    if (Test-Path -LiteralPath $p) { Copy-Item -LiteralPath $p -Destination (Join-Path $isi 'aset') }
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
Pindah Drive menyertakan program pihak ketiga berikut:

rclone ($versiRclone)
  https://rclone.org
  Hak cipta Nick Craig-Wood dan para kontributor rclone.
  Dirilis dengan lisensi MIT. Teks lisensi lengkap:
  https://github.com/rclone/rclone/blob/master/COPYING
  Dokumentasi rclone yang menyertai berkas ini: rclone-README.txt
"@ | Set-Content -LiteralPath (Join-Path $isi 'PIHAK-KETIGA.txt') -Encoding UTF8

$infoCs = Join-Path $kerja 'Info.cs'
"static class Info { public const string Versi = `"$Versi`"; }" | Set-Content -LiteralPath $infoCs -Encoding ASCII
$csc = Join-Path ([System.Runtime.InteropServices.RuntimeEnvironment]::GetRuntimeDirectory()) 'csc.exe'
$argCsc = @('/nologo', '/target:winexe', '/optimize+', '/platform:anycpu',
    "/win32icon:$(Join-Path $akarProyek 'aset\ikon.ico')",
    "/win32manifest:$(Join-Path $akar 'installer\setup.manifest')",
    '/reference:System.IO.Compression.dll', '/reference:System.Windows.Forms.dll', '/reference:System.Drawing.dll')

# uninstall.exe: kode yang sama tanpa muatan (beberapa puluh KB, bukan 30 MB).
& $csc @argCsc "/out:$(Join-Path $isi 'uninstall.exe')" (Join-Path $akar 'installer\Setup.cs') $infoCs
if ($LASTEXITCODE -ne 0) { throw "Kompilasi uninstall.exe gagal (kode $LASTEXITCODE)." }

Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = Join-Path $kerja 'payload.zip'
[System.IO.Compression.ZipFile]::CreateFromDirectory($isi, $zip, [System.IO.Compression.CompressionLevel]::Optimal, $false)
Write-Host ('    payload.zip {0:N1} MB' -f ((Get-Item $zip).Length / 1MB))

Write-Host '4/4 Mengompilasi installer'
$dist = Join-Path $akarProyek 'dist'
New-Item -ItemType Directory -Path $dist -Force | Out-Null
$keluar = Join-Path $dist "PindahDrive-Setup-$Versi.exe"
& $csc @argCsc "/resource:$zip,payload.zip" "/out:$keluar" (Join-Path $akar 'installer\Setup.cs') $infoCs
if ($LASTEXITCODE -ne 0) { throw "Kompilasi installer gagal (kode $LASTEXITCODE)." }
Remove-Item -LiteralPath $kerja -Recurse -Force
Write-Host ('Selesai: {0} ({1:N1} MB)' -f $keluar, ((Get-Item $keluar).Length / 1MB))
