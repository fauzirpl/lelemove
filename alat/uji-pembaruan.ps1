<#
.SYNOPSIS
    Menguji alur "Unduh & pasang" (pembaruan dari dalam LELEMOVE) terhadap installer lokal.

.DESCRIPTION
    Memakai dua kait uji di pindah.exe:
      PINDAH_DRIVE_VERSI_UJI=0.0.1         aplikasi mengira dirinya versi lama
      PINDAH_DRIVE_INSTALLER_UJI=<berkas>  installer lokal dipakai alih-alih unduhan
    Pemeriksaan versi tetap menghubungi github.com (butuh internet).

    PERHATIAN: -Installer dipasang MENIMPA pemasangan LELEMOVE di komputer ini (akun, pengaturan,
    jadwal, dan riwayat tidak disentuh). Sesudah uji, komputer memakai build yang diuji; pasang
    lagi installer resmi bila itu bukan yang diinginkan.

    Tanpa -TanpaJendela (bawaan, di komputer pengembang): LELEMOVE dibuka, tombol ditekan lewat
    UI Automation (Bantuan & info > Tentang > Periksa pembaruan > Unduh & pasang), lalu diperiksa
    bahwa berkas diganti dan LELEMOVE terbuka lagi. Ini menguji keadaan yang dulu gagal: installer
    mulai ketika proses LELEMOVE belum benar-benar berakhir.

    Dengan -TanpaJendela (CI, tanpa OpenGL): `pindah pembaruan -Jalankan` dipanggil langsung.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File alat\uji-pembaruan.ps1 -Installer dist\LELEMOVE-Setup-2.5.0.exe
#>
param(
    [Parameter(Mandatory)][string]$Installer,
    [switch]$TanpaJendela,
    # Berapa kali alur tombol diulang (galat berkas terkunci bergantung waktu).
    [int]$Ulang = 2
)
$ErrorActionPreference = 'Stop'
$Installer = (Resolve-Path -LiteralPath $Installer).Path
$dir = Join-Path $env:LOCALAPPDATA 'Programs\LELEMOVE'
$app = Join-Path $dir 'LELEMOVE.exe'
$mesin = Join-Path $dir 'pindah.exe'

Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
Add-Type -Namespace LeleUji -Name W -MemberDefinition @'
public delegate bool CB(System.IntPtr h, System.IntPtr l);
[DllImport("user32.dll")] public static extern bool EnumWindows(CB f, System.IntPtr l);
[DllImport("user32.dll")] public static extern bool EnumChildWindows(System.IntPtr p, CB f, System.IntPtr l);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(System.IntPtr h, out uint p);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(System.IntPtr h);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(System.IntPtr h, System.Text.StringBuilder s, int n);
'@

function Teks-Jendela($h) {
    $sb = New-Object System.Text.StringBuilder 2048
    [void][LeleUji.W]::GetWindowText($h, $sb, 2048)
    $sb.ToString()
}

# Kotak pesan galat milik setup (bila ada): isi teksnya.
function Pesan-Setup {
    $ids = @(Get-Process | Where-Object { $_.Name -like 'LELEMOVE-Setup*' } | ForEach-Object Id)
    if (-not $ids) { return $null }
    $script:jendela = @()
    $cb = [LeleUji.W+CB] { param($h, $l) $p = 0; [void][LeleUji.W]::GetWindowThreadProcessId($h, [ref]$p)
        if ($ids -contains $p -and [LeleUji.W]::IsWindowVisible($h)) { $script:jendela += $h }; $true }
    [void][LeleUji.W]::EnumWindows($cb, [IntPtr]::Zero)
    foreach ($h in $script:jendela) {
        $script:isi = @()
        $cb2 = [LeleUji.W+CB] { param($c, $l) $t = Teks-Jendela $c; if ($t) { $script:isi += $t }; $true }
        [void][LeleUji.W]::EnumChildWindows($h, $cb2, [IntPtr]::Zero)
        return (($script:isi -join ' / ') -replace '\r?\n', ' ')
    }
    $null
}

function Jendela-Lele {
    $p = Get-Process LELEMOVE -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $p) { return $null }
    $syarat = New-Object Windows.Automation.PropertyCondition([Windows.Automation.AutomationElement]::ProcessIdProperty, $p.Id)
    [Windows.Automation.AutomationElement]::RootElement.FindFirst([Windows.Automation.TreeScope]::Children, $syarat)
}

function Cari-Tombol($win, [string]$nama) {
    if (-not $win) { return $null }
    $syarat = New-Object Windows.Automation.PropertyCondition([Windows.Automation.AutomationElement]::NameProperty, $nama)
    foreach ($e in $win.FindAll([Windows.Automation.TreeScope]::Descendants, $syarat)) {
        if ($e.Current.ControlType -ne [Windows.Automation.ControlType]::Text) { return $e }
    }
    $null
}

function Tekan($e) {
    $pola = $e.GetSupportedPatterns() | ForEach-Object ProgrammaticName
    if ($pola -match 'Invoke') { $e.GetCurrentPattern([Windows.Automation.InvokePattern]::Pattern).Invoke() }
    elseif ($pola -match 'SelectionItem') { $e.GetCurrentPattern([Windows.Automation.SelectionItemPattern]::Pattern).Select() }
    elseif ($pola -match 'Toggle') { $e.GetCurrentPattern([Windows.Automation.TogglePattern]::Pattern).Toggle() }
    else { throw "Kontrol '$($e.Current.Name)' tidak bisa ditekan lewat UI Automation ($($pola -join ', '))." }
}

function Tunggu-Tombol([string]$nama, [int]$detik, [scriptblock]$selagi) {
    $batas = (Get-Date).AddSeconds($detik)
    $i = 0
    while ((Get-Date) -lt $batas) {
        $t = Cari-Tombol (Jendela-Lele) $nama
        if ($t) { return $t }
        if ($selagi -and ($i % 8 -eq 0)) { & $selagi }
        $i++
        Start-Sleep -Milliseconds 800
    }
    throw "Tombol '$nama' tidak muncul dalam $detik detik."
}

function Tutup-Lele {
    Get-Process LELEMOVE -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Milliseconds 800
}

function Tunggu-Hasil([datetime]$sebelum, [switch]$HarusTerbuka) {
    $batas = (Get-Date).AddSeconds(240)
    while ((Get-Date) -lt $batas) {
        Start-Sleep -Milliseconds 500
        $pesan = Pesan-Setup
        if ($pesan) {
            Get-Process | Where-Object { $_.Name -like 'LELEMOVE-Setup*' } | Stop-Process -Force
            throw "Setup menampilkan galat: $pesan"
        }
        $setup = @(Get-Process | Where-Object { $_.Name -like 'LELEMOVE-Setup*' })
        $diganti = (Get-Item -LiteralPath $app).LastWriteTime -gt $sebelum
        if ($diganti -and $setup.Count -eq 0) {
            if (-not $HarusTerbuka) { return }
            if (Get-Process LELEMOVE -ErrorAction SilentlyContinue) { return }
        }
    }
    throw 'Pembaruan tidak selesai dalam 240 detik (berkas belum diganti atau LELEMOVE tidak dibuka lagi).'
}

Write-Host "1/3 Memasang $([IO.Path]::GetFileName($Installer))"
Tutup-Lele
$p = Start-Process -FilePath $Installer -ArgumentList '/silent' -Wait -PassThru
if ($p.ExitCode -ne 0) { throw "Pemasangan gagal (kode $($p.ExitCode))." }
if (-not (Test-Path -LiteralPath $mesin)) { throw "Tidak ada $mesin sesudah memasang." }

# Salinan installer, seperti berkas hasil unduhan.
$kerja = Join-Path $env:TEMP ('lelemove-uji-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
New-Item -ItemType Directory -Path $kerja -Force | Out-Null
$salinan = Join-Path $kerja ([IO.Path]::GetFileName($Installer))
Copy-Item -LiteralPath $Installer -Destination $salinan
$env:PINDAH_DRIVE_VERSI_UJI = '0.0.1'
$env:PINDAH_DRIVE_INSTALLER_UJI = $salinan

try {
    if ($TanpaJendela) {
        Write-Host '2/3 pindah pembaruan -Jalankan'
        $sebelum = (Get-Item -LiteralPath $app).LastWriteTime
        Start-Sleep -Seconds 2
        $ErrorActionPreference = 'Continue'
        $keluaran = & $mesin pembaruan -Jalankan -Mesin 2>&1 | ForEach-Object { "$_" }
        $kode = $LASTEXITCODE
        $ErrorActionPreference = 'Stop'
        $keluaran | Select-Object -Last 4 | ForEach-Object { Write-Host "    $_" }
        if ($kode -ne 0) { throw "pindah pembaruan gagal (kode $kode)." }
        Write-Host '3/3 Menunggu installer mengganti berkas'
        Tunggu-Hasil $sebelum
        Tutup-Lele
    }
    else {
        for ($n = 1; $n -le $Ulang; $n++) {
            Write-Host "2/3 Alur tombol, putaran $n dari $Ulang"
            Tutup-Lele
            Start-Process -FilePath $app
            [void](Tunggu-Tombol 'Bantuan & info' 30)
            Start-Sleep -Seconds 1
            $sebelum = (Get-Item -LiteralPath $app).LastWriteTime
            Tekan (Cari-Tombol (Jendela-Lele) 'Bantuan & info')
            Tekan (Tunggu-Tombol 'Tentang' 15)
            # Pemeriksaan versi bisa gagal sesaat (DNS): tekan lagi selama tombolnya masih ada.
            $pasang = Tunggu-Tombol 'Unduh & pasang' 120 {
                $t = Cari-Tombol (Jendela-Lele) 'Periksa pembaruan'
                if ($t -and $t.Current.IsEnabled) { Tekan $t }
            }
            Tekan $pasang
            Write-Host '3/3 Menunggu LELEMOVE ditutup, diperbarui, dan dibuka lagi'
            Tunggu-Hasil $sebelum -HarusTerbuka
            Write-Host "    putaran $n lolos"
        }
        Tutup-Lele
    }
    Write-Host 'LOLOS: alur pembaruan berjalan.'
}
finally {
    Remove-Item Env:\PINDAH_DRIVE_VERSI_UJI, Env:\PINDAH_DRIVE_INSTALLER_UJI -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $kerja -Recurse -Force -ErrorAction SilentlyContinue
}
