<#
.SYNOPSIS
    Membuat ikon (aset\ikon.ico) dan peluncur "Pindah Drive.exe".

.DESCRIPTION
    Cukup dijalankan sekali, atau setelah Peluncur.cs diubah:
        powershell -ExecutionPolicy Bypass -File .\buat-aplikasi.ps1
    Memakai csc.exe bawaan .NET Framework 4.x; tidak perlu memasang apa pun.
#>
$ErrorActionPreference = 'Stop'
$akar = $PSScriptRoot
Add-Type -AssemblyName System.Drawing

# ---- ikon: kotak tinta membulat + panah putih, dalam beberapa ukuran (PNG di dalam ICO)
function Png-Ikon([int]$s) {
    $bmp = New-Object System.Drawing.Bitmap $s, $s, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.Clear([System.Drawing.Color]::Transparent)

    $w = [single]($s - 1)
    $d = [single]([Math]::Max(2, $s * 0.44))
    $kotak = New-Object System.Drawing.Drawing2D.GraphicsPath
    $kotak.AddArc(0, 0, $d, $d, 180, 90)
    $kotak.AddArc($w - $d, 0, $d, $d, 270, 90)
    $kotak.AddArc($w - $d, $w - $d, $d, $d, 0, 90)
    $kotak.AddArc(0, $w - $d, $d, $d, 90, 90)
    $kotak.CloseFigure()
    $g.FillPath((New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 31, 35, 40))), $kotak)

    $pena = New-Object System.Drawing.Pen ([System.Drawing.Color]::White), ([single][Math]::Max(1.6, $s * 0.105))
    $pena.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $pena.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
    $pena.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round
    $y = [single]($s * 0.5)
    $g.DrawLine($pena, [single]($s * 0.25), $y, [single]($s * 0.73), $y)
    $kepala = [System.Drawing.PointF[]]@(
        (New-Object System.Drawing.PointF ([single]($s * 0.52)), ([single]($s * 0.29))),
        (New-Object System.Drawing.PointF ([single]($s * 0.74)), $y),
        (New-Object System.Drawing.PointF ([single]($s * 0.52)), ([single]($s * 0.71)))
    )
    $g.DrawLines($pena, $kepala)
    $g.Dispose()

    $ms = New-Object System.IO.MemoryStream
    $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    return , $ms.ToArray()
}

$dirAset = Join-Path $akar 'aset'
if (-not (Test-Path -LiteralPath $dirAset)) { New-Item -ItemType Directory -Path $dirAset | Out-Null }
$fileIkon = Join-Path $dirAset 'ikon.ico'

$ukuran = @(256, 64, 48, 32, 24, 16)
$gambar = @()
foreach ($s in $ukuran) { $gambar += , (Png-Ikon $s) }

$ms = New-Object System.IO.MemoryStream
$bw = New-Object System.IO.BinaryWriter $ms
$bw.Write([uint16]0)               # reserved
$bw.Write([uint16]1)               # tipe: ikon
$bw.Write([uint16]$ukuran.Count)
$offset = 6 + 16 * $ukuran.Count
for ($i = 0; $i -lt $ukuran.Count; $i++) {
    $s = $ukuran[$i]
    $b = [byte]($s % 256)          # 256 ditulis sebagai 0
    $bw.Write($b); $bw.Write($b); $bw.Write([byte]0); $bw.Write([byte]0)
    $bw.Write([uint16]1); $bw.Write([uint16]32)
    $bw.Write([uint32]$gambar[$i].Length); $bw.Write([uint32]$offset)
    $offset += $gambar[$i].Length
}
foreach ($g in $gambar) { $bw.Write([byte[]]$g) }
$bw.Flush()
$baru = $ms.ToArray()
$sama = (Test-Path -LiteralPath $fileIkon) -and ([Convert]::ToBase64String([System.IO.File]::ReadAllBytes($fileIkon)) -eq [Convert]::ToBase64String($baru))
if (-not $sama) { [System.IO.File]::WriteAllBytes($fileIkon, $baru) }
Write-Host "Ikon    : $fileIkon"

# ---- peluncur exe
$csc = Join-Path ([System.Runtime.InteropServices.RuntimeEnvironment]::GetRuntimeDirectory()) 'csc.exe'
if (-not (Test-Path -LiteralPath $csc)) { throw "csc.exe tidak ditemukan di $csc" }
$sumber = Join-Path $akar 'peluncur\Peluncur.cs'
$exe = Join-Path $akar 'Pindah Drive.exe'
& $csc /nologo /target:winexe /optimize+ /platform:anycpu "/win32icon:$fileIkon" "/out:$exe" /reference:System.Windows.Forms.dll $sumber
if ($LASTEXITCODE -ne 0) { throw "Kompilasi peluncur gagal (kode $LASTEXITCODE)." }
Write-Host "Peluncur: $exe"
