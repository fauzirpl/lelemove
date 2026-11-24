<#
.SYNOPSIS
    Pindah Drive: salin backup dari OneDrive ke Google Drive memakai rclone.

.DESCRIPTION
    Cara paling mudah: klik dua kali Pindah.cmd, lalu pilih menu 1.

    Atau jalankan perintah langsung:
        .\pindah.ps1 cek
        .\pindah.ps1 pasang       [-Ya]
        .\pindah.ps1 hubungkan    [-Akun google|onedrive]
        .\pindah.ps1 periksa      [-Sumber <folder>] [-Tujuan <folder di Google Drive>]
        .\pindah.ps1 uji
        .\pindah.ps1 salin        [-Latar] [-Ya]
        .\pindah.ps1 status
        .\pindah.ps1 berhenti
        .\pindah.ps1 verifikasi

    Skrip ini hanya MENYALIN (rclone copy). Tidak ada perintah yang menghapus
    atau memindahkan file, baik di OneDrive maupun di Google Drive.

    Data kerja (pengaturan, log, status) disimpan di %LOCALAPPDATA%\pindah-drive.
    Token login disimpan oleh rclone sendiri di %APPDATA%\rclone\rclone.conf.
#>
[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [ValidateSet('menu', 'cek', 'akun', 'pasang', 'hubungkan', 'lepas', 'bawaan', 'situs', 'pustaka', 'periksa', 'uji', 'salin', 'status', 'berhenti', 'verifikasi')]
    [string]$Perintah = 'menu',

    # Sumber & tujuan: folder di komputer (mis. D:\OneDrive\Backup) atau
    # "<akun>:<folder>" (mis. pd-onedrive:Backup, pd-gdrive:Arsip/2024).
    # Tujuan tanpa nama akun dianggap folder di akun Google utama (pd-gdrive).
    [string]$Sumber,
    [string]$Tujuan,

    # Akun (remote rclone) untuk hubungkan/lepas. Alias: google = pd-gdrive, onedrive = pd-onedrive.
    [string]$Akun = 'google',

    # hubungkan akun baru: jenisnya, dan nama tampilan bebas.
    [ValidateSet('', 'google', 'onedrive', 'sharepoint')]
    [string]$Jenis = '',
    [string]$Label,

    # hubungkan (Google): izin baca-tulis penuh, wajib bila akun ini dipakai sebagai SUMBER.
    [switch]$IzinPenuh,

    # Jalankan penyalinan di latar belakang (jendela boleh ditutup).
    [switch]$Latar,

    # Lewati pertanyaan konfirmasi (dipakai bila persetujuan sudah diberikan).
    [switch]$Ya,

    # hubungkan: paksa login ulang (mis. untuk ganti akun).
    [switch]$Ulang,

    # hubungkan (Google): client ID OAuth milik sendiri, pengganti client ID bersama rclone.
    [string]$ClientId,
    [string]$ClientSecret,
    # ...dan simpan sebagai client ID bawaan aplikasi (dipakai otomatis & ikut dibundel installer).
    [switch]$SimpanBawaan,

    # SharePoint: situs -Cari <kata>; pustaka -SitusId <id> (daftar) atau -Pustaka <driveId> (pakai).
    [string]$Cari,
    [string]$SitusId,
    [string]$SitusNama,
    [string]$Pustaka,
    [string]$PustakaNama,

    # Untuk aplikasi (GUI): cetak ringkasan hasil sebagai satu baris "##HASIL {json}".
    [switch]$Mesin,

    # Internal: dipakai saat skrip menjalankan dirinya sendiri di latar belakang.
    [string]$LogLatar
)

# Native command (rclone, winget) menulis ke stderr; 'Stop' di PowerShell 5.1
# akan menganggapnya error fatal. Kesalahan ditangani manual lewat kode keluar.
$ErrorActionPreference = 'Continue'

$RemoteGoogle   = 'pd-gdrive'
$RemoteOneDrive = 'pd-onedrive'
$FolderInduk    = 'Pindahan OneDrive'
$DirData        = Join-Path $env:LOCALAPPDATA 'pindah-drive'
if ($env:PINDAH_DRIVE_DATA) { $DirData = $env:PINDAH_DRIVE_DATA }   # untuk pengujian
$DirLog         = Join-Path $DirData 'log'
$FilePengaturan = Join-Path $DirData 'pengaturan.json'
$FileJob        = Join-Path $DirData 'job.json'
$FileAkun       = Join-Path $DirData 'akun.json'      # nama tampilan & drive OneDrive per akun

# Client ID Google bawaan: milik pengguna ini ($DirData) atau yang dibundel installer (folder aplikasi).
# Sengaja tidak disimpan di folder proyek: folder itu ada di bawah web root Laragon.
$FileClientPengguna = Join-Path $DirData 'client-google.json'
$FileClientApp      = Join-Path $PSScriptRoot 'client-google.json'

# File sistem OneDrive/Windows yang tidak perlu ikut disalin.
$Kecualian = @(
    'desktop.ini',
    'Thumbs.db',
    '~$*',
    '.849C9593-D756-4E56-8D6E-42412F2A707B',
    'Personal Vault.lnk'
)

# FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS (0x400000) | FILE_ATTRIBUTE_OFFLINE (0x1000):
# penanda file OneDrive yang "hanya online" (belum ada isinya di disk).
$AtributHanyaCloud = 0x401000

# Ringkasan hasil untuk GUI (diisi lewat Catat, dicetak di akhir bila -Mesin).
$HasilMesin = [ordered]@{}
function Catat([string]$kunci, $nilai) { $script:HasilMesin[$kunci] = $nilai }

if ($Mesin) {
    # GUI membaca keluaran sebagai UTF-8 (nama file bisa mengandung karakter non-ASCII).
    try { [Console]::OutputEncoding = New-Object System.Text.UTF8Encoding $false } catch { }
}

# ---------------------------------------------------------------- tampilan

function Judul([string]$t) { Write-Host ''; Write-Host "== $t ==" -ForegroundColor Cyan }
function Ok([string]$t)    { Write-Host "[OK]    $t" -ForegroundColor Green }
function Info([string]$t)  { Write-Host "        $t" }
function Awas([string]$t)  { Write-Host "[AWAS]  $t" -ForegroundColor Yellow }
function Gagal([string]$t) { Write-Host "[GAGAL] $t" -ForegroundColor Red }

function Format-Ukuran([double]$b) {
    if ($b -ge 1TB) { return ('{0:N2} TB' -f ($b / 1TB)) }
    if ($b -ge 1GB) { return ('{0:N2} GB' -f ($b / 1GB)) }
    if ($b -ge 1MB) { return ('{0:N1} MB' -f ($b / 1MB)) }
    return ('{0:N0} KB' -f ($b / 1KB))
}

function Bisa-Bertanya { return (-not [Console]::IsInputRedirected) }

function Konfirmasi([string]$tanya) {
    if ($Ya) { return $true }
    if (-not (Bisa-Bertanya)) {
        Awas "Butuh persetujuan: $tanya"
        Info 'Ulangi perintah dengan tambahan -Ya setelah disetujui.'
        return $false
    }
    $j = Read-Host "        $tanya (y/n)"
    return ($j.Trim() -match '^(y|ya|yes)$')
}

function Baca-Angka([string]$label, [int]$min, [int]$max) {
    while ($true) {
        $j = (Read-Host "        $label").Trim()
        $n = 0
        if ([int]::TryParse($j, [ref]$n) -and $n -ge $min -and $n -le $max) { return $n }
        Awas "Masukkan angka $min sampai $max."
    }
}

# ---------------------------------------------------------------- data kerja

function Pastikan-Dir {
    if (-not (Test-Path -LiteralPath $DirLog)) { New-Item -ItemType Directory -Path $DirLog -Force | Out-Null }
}

function Path-Log([string]$jenis, [string]$ext = 'log') {
    Pastikan-Dir
    return (Join-Path $DirLog ('{0}-{1}.{2}' -f $jenis, (Get-Date -Format 'yyyyMMdd-HHmmss'), $ext))
}

function Baca-Json([string]$file) {
    if (-not (Test-Path -LiteralPath $file)) { return $null }
    try { return (Get-Content -LiteralPath $file -Raw -Encoding UTF8 | ConvertFrom-Json) } catch { return $null }
}

function Tulis-Json([string]$file, $obj) {
    Pastikan-Dir
    $obj | ConvertTo-Json | Set-Content -LiteralPath $file -Encoding UTF8
}

function Simpan-Pengaturan([string]$src, [string]$dst) {
    Tulis-Json $FilePengaturan ([pscustomobject]@{ Sumber = $src; Tujuan = $dst })
}

function Baca-Job { return (Baca-Json $FileJob) }

# Membaca job.json, menimpa field yang diberikan, lalu menyimpannya lagi.
function Perbarui-Job([hashtable]$ubah, [switch]$Baru) {
    $h = @{}
    if (-not $Baru) {
        $lama = Baca-Job
        if ($lama) { foreach ($pr in $lama.PSObject.Properties) { $h[$pr.Name] = $pr.Value } }
    }
    foreach ($k in $ubah.Keys) { $h[$k] = $ubah[$k] }
    Tulis-Json $FileJob ([pscustomobject]$h)
}

function Proses-Hidup($id, [string]$nama) {
    if (-not $id) { return $false }
    $p = Get-Process -Id ([int]$id) -ErrorAction SilentlyContinue
    return [bool]($p -and $p.ProcessName -eq $nama)
}

function Job-Berjalan($job) {
    if (-not $job) { return $false }
    if ($job.Selesai) { return $false }
    if ($job.PidRclone) { return (Proses-Hidup $job.PidRclone 'rclone') }
    return (Proses-Hidup $job.PidInduk 'powershell')
}

# ---------------------------------------------------------------- rclone

function Cari-Rclone {
    $lokal = Join-Path $PSScriptRoot 'rclone.exe'
    if (Test-Path -LiteralPath $lokal) { return $lokal }
    $c = Get-Command rclone -ErrorAction SilentlyContinue
    if ($c) { return $c.Source }
    # winget memasang di sini; PATH di jendela yang sudah terbuka belum diperbarui.
    $link = Join-Path $env:LOCALAPPDATA 'Microsoft\WinGet\Links\rclone.exe'
    if (Test-Path -LiteralPath $link) { return $link }
    $pkg = Join-Path $env:LOCALAPPDATA 'Microsoft\WinGet\Packages'
    if (Test-Path -LiteralPath $pkg) {
        $f = Get-ChildItem -LiteralPath $pkg -Directory -Filter 'Rclone.Rclone*' -ErrorAction SilentlyContinue |
            ForEach-Object { Get-ChildItem -LiteralPath $_.FullName -Recurse -Filter 'rclone.exe' -ErrorAction SilentlyContinue } |
            Select-Object -First 1
        if ($f) { return $f.FullName }
    }
    return $null
}

function Wajib-Rclone {
    if (Cari-Rclone) { return $true }
    Gagal 'rclone belum terpasang. Jalankan: pindah.ps1 pasang'
    return $false
}

# Menyusun baris perintah Windows dengan aturan kutip yang benar
# (path OneDrive hampir selalu mengandung spasi, mis. "OneDrive - Kantor").
function Gabung-Argumen([string[]]$daftar) {
    $hasil = foreach ($a in $daftar) {
        if ($a -eq '') { '""' }
        elseif ($a -match '[\s"]') { '"' + (($a -replace '(\\*)"', '$1$1\"') -replace '(\\+)$', '$1$1') + '"' }
        else { $a }
    }
    return ($hasil -join ' ')
}

# -Tangkap     : kumpulkan stdout & stderr (tidak tampil).
# -BuangStdout : stdout dibuang (mis. agar token tidak tercetak), stderr tetap tampil.
function Panggil-Rclone([string[]]$Argumen, [switch]$Tangkap, [switch]$BuangStdout) {
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = Cari-Rclone
    $psi.Arguments = Gabung-Argumen $Argumen
    $psi.UseShellExecute = $false
    if ($Tangkap -or $BuangStdout) {
        $psi.RedirectStandardOutput = $true
        $psi.StandardOutputEncoding = [System.Text.Encoding]::UTF8
    }
    if ($Tangkap) {
        $psi.RedirectStandardError = $true
        $psi.StandardErrorEncoding = [System.Text.Encoding]::UTF8
    }
    $p = [System.Diagnostics.Process]::Start($psi)
    $o = $null; $e = $null
    if ($psi.RedirectStandardOutput) { $o = $p.StandardOutput.ReadToEndAsync() }
    if ($psi.RedirectStandardError)  { $e = $p.StandardError.ReadToEndAsync() }
    $p.WaitForExit()
    $out = ''; $err = ''
    if ($o -and $Tangkap) { $out = $o.Result }
    if ($e) { $err = $e.Result }
    return [pscustomobject]@{ Kode = $p.ExitCode; Out = $out; Err = $err }
}

function Mulai-Rclone([string[]]$Argumen) {
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = Cari-Rclone
    $psi.Arguments = Gabung-Argumen $Argumen
    $psi.UseShellExecute = $false
    return [System.Diagnostics.Process]::Start($psi)
}

function Daftar-Remote {
    if (-not (Cari-Rclone)) { return @() }
    $h = Panggil-Rclone @('listremotes') -Tangkap
    return @($h.Out -split "`r?`n" | ForEach-Object { $_.Trim().TrimEnd(':') } | Where-Object { $_ })
}

# ---------------------------------------------------------------- akun (remote rclone milik Pindah Drive)
# Setiap akun cloud adalah satu remote rclone berawalan "pd-": pd-gdrive, pd-gdrive-2, pd-onedrive, ...

function Nama-Remote([string]$a) {
    if ($a -eq 'google') { return $RemoteGoogle }
    if ($a -eq 'onedrive') { return $RemoteOneDrive }
    return $a.Trim().TrimEnd(':')
}

# "pd-onedrive:Kerja/DLH" -> "pd-onedrive";  "D:\Data" -> ""
function Remote-Dari([string]$jalur) {
    if (-not $jalur -or (Adalah-Lokal $jalur)) { return '' }
    if ($jalur -match '^([^:\\/]+):') { return $matches[1] }
    return ''
}

# Jenis ditebak dari nama (tanpa memanggil rclone): cukup untuk label & nama folder bawaan.
function Jenis-DariNama([string]$remote) {
    if ($remote -like 'pd-gdrive*' -or $remote -like 'pd-google*') { return 'google' }
    if ($remote -like 'pd-onedrive*') { return 'onedrive' }
    if ($remote -like 'pd-sharepoint*') { return 'sharepoint' }
    return ''
}

function Baca-MetaAkun {
    $h = @{}
    $j = Baca-Json $FileAkun
    if ($j) { foreach ($p in $j.PSObject.Properties) { $h[$p.Name] = $p.Value } }
    return $h
}

function Simpan-MetaAkun([string]$nama, [hashtable]$ubah, [switch]$Hapus) {
    $semua = Baca-MetaAkun
    if ($Hapus) { $semua.Remove($nama) }
    else {
        $isi = [ordered]@{}
        if ($semua[$nama]) { foreach ($p in $semua[$nama].PSObject.Properties) { $isi[$p.Name] = $p.Value } }
        foreach ($k in $ubah.Keys) { $isi[$k] = $ubah[$k] }
        $semua[$nama] = [pscustomobject]$isi
    }
    $urut = [ordered]@{}
    foreach ($k in ($semua.Keys | Sort-Object)) { $urut[$k] = $semua[$k] }
    Tulis-Json $FileAkun ([pscustomobject]$urut)
}

function Label-Bawaan([string]$nama, [string]$jenis) {
    $dasar = 'Akun'
    if ($jenis -eq 'google') { $dasar = 'Google Drive' }
    elseif ($jenis -eq 'onedrive') { $dasar = 'OneDrive' }
    elseif ($jenis -eq 'sharepoint') { $dasar = 'SharePoint' }
    if ($nama -match '-(\d+)$') { return "$dasar $($matches[1])" }
    return $dasar
}

# Pengaturan akun yang tidak rahasia (rclone sendiri menyensor token & secret).
function Konfig-Akun([string]$nama) {
    $teks = (Panggil-Rclone @('config', 'redacted', "${nama}:") -Tangkap).Out
    $k = @{ Tipe = ''; Scope = ''; ClientSendiri = $false }
    foreach ($b in ($teks -split "`r?`n")) {
        if ($b -match '^type\s*=\s*(\S+)') { $k.Tipe = $matches[1] }
        elseif ($b -match '^scope\s*=\s*(\S+)') { $k.Scope = $matches[1] }
        elseif ($b -match '^client_id\s*=\s*\S') { $k.ClientSendiri = $true }
    }
    return $k
}

function Jenis-Remote([string]$nama) {
    $t = (Konfig-Akun $nama).Tipe
    if ($t -eq 'drive') { return 'google' }
    if ($t -eq 'onedrive') {
        # SharePoint & OneDrive memakai backend yang sama; dibedakan lewat nama remote.
        if ((Jenis-DariNama $nama) -eq 'sharepoint') { return 'sharepoint' }
        return 'onedrive'
    }
    return $t
}

function Daftar-Akun {
    $hasil = @()
    if (-not (Cari-Rclone)) { return , $hasil }
    $meta = Baca-MetaAkun
    $daftar = @()
    # PowerShell 5.1: ConvertFrom-Json mengembalikan array JSON sebagai satu objek; pipeline membukanya.
    try { $daftar = @((Panggil-Rclone @('listremotes', '--json') -Tangkap).Out | ConvertFrom-Json | ForEach-Object { $_ }) } catch { }
    foreach ($r in $daftar) {
        if ("$($r.name)" -notlike 'pd-*') { continue }
        $k = Konfig-Akun $r.name
        $jenis = $r.type
        if ($r.type -eq 'drive') { $jenis = 'google' }
        if ($r.type -eq 'onedrive' -and (Jenis-DariNama $r.name) -eq 'sharepoint') { $jenis = 'sharepoint' }
        $m = $meta[$r.name]
        $label = ''; $drive = ''; $situs = ''; $driveId = ''
        if ($m) {
            $label = "$($m.Label)"; $drive = "$($m.Drive)"
            if ($m.PSObject.Properties['Situs']) { $situs = "$($m.Situs)" }
            if ($m.PSObject.Properties['DriveId']) { $driveId = "$($m.DriveId)" }
        }
        if (-not $label) { $label = Label-Bawaan $r.name $jenis }
        if (-not $drive -and $r.name -eq $RemoteOneDrive) {
            # pengaturan versi lama
            $lama = Baca-Json (Join-Path $DirData 'onedrive.json')
            if ($lama -and $lama.Nama) { $drive = "$($lama.Nama) ($($lama.Jenis))" }
        }
        $hasil += [pscustomobject]@{
            Nama = $r.name; Jenis = $jenis; Label = $label; Drive = $drive
            Scope = $k.Scope; ClientSendiri = $k.ClientSendiri
            Situs = $situs; DriveId = $driveId
            # SharePoint tanpa pustaka terpilih masih menunjuk OneDrive pribadi: belum boleh dipakai.
            Siap = ($jenis -ne 'sharepoint' -or [bool]$driveId)
        }
    }
    return , $hasil
}

function Baca-ClientBawaan {
    foreach ($f in @($FileClientPengguna, $FileClientApp)) {
        $j = Baca-Json $f
        if ($j -and $j.ClientId -and $j.ClientSecret) { return $j }
    }
    return $null
}

function Simpan-ClientBawaan([string]$id, [string]$rahasia) {
    Tulis-Json $FileClientPengguna ([pscustomobject]@{ ClientId = $id; ClientSecret = $rahasia })
}

# Nama remote baru yang belum terpakai: pd-gdrive, pd-gdrive-2, pd-gdrive-3, ...
function Remote-Baru([string]$jenis) {
    $dasar = $RemoteOneDrive
    if ($jenis -eq 'google') { $dasar = $RemoteGoogle }
    elseif ($jenis -eq 'sharepoint') { $dasar = 'pd-sharepoint' }
    $ada = @(Daftar-Remote)
    if ($ada -notcontains $dasar) { return $dasar }
    for ($i = 2; $i -lt 100; $i++) { if ($ada -notcontains "$dasar-$i") { return "$dasar-$i" } }
    return "$dasar-$(Get-Random)"
}

function Argumen-Kecualian {
    $a = @()
    foreach ($k in $Kecualian) { $a += '--exclude'; $a += $k }
    return $a
}

function Cocok-Kecualian([string]$nama) {
    foreach ($k in $Kecualian) { if ($nama -like $k) { return $true } }
    return $false
}

function Ke-Byte([string]$angka, [string]$satuan) {
    $n = [double]::Parse($angka, [System.Globalization.CultureInfo]::InvariantCulture)
    if ($satuan -match '^K') { return $n * 1KB }
    if ($satuan -match '^M') { return $n * 1MB }
    if ($satuan -match '^G') { return $n * 1GB }
    if ($satuan -match '^T') { return $n * 1TB }
    return $n
}

# Mengambil blok statistik terakhir yang ditulis rclone ke log (tiap --stats).
function Baca-Statistik([string]$log) {
    if (-not $log -or -not (Test-Path -LiteralPath $log)) { return $null }
    $b = @(Get-Content -LiteralPath $log -Tail 400 -Encoding UTF8)
    $i = -1
    for ($k = $b.Count - 1; $k -ge 0; $k--) {
        if ($b[$k] -match '^\s*Transferred:.*B\s*/') { $i = $k; break }
    }
    if ($i -lt 0) { return $null }
    $h = [ordered]@{ Data = ($b[$i] -replace '^\s*Transferred:\s*', '').Trim(); File = ''; Error = ''; Waktu = ''; Sedang = @(); Dibatasi = $false }
    # Google/Microsoft menolak karena terlalu banyak permintaan (rate limit / kuota project).
    $h.Dibatasi = [bool]($b | Where-Object { $_ -match 'rateLimitExceeded|RATE_LIMIT_EXCEEDED|Quota exceeded|activityLimitReached|429 Too Many|throttled' } | Select-Object -First 1)
    $k = $i + 1
    for (; $k -lt [Math]::Min($b.Count, $i + 15); $k++) {
        $l = $b[$k]
        if ($l -match '^\s*Errors:\s*(.+)$') { $h.Error = $matches[1].Trim() }
        elseif ($l -match '^\s*Transferred:\s*(.+)$') { $h.File = $matches[1].Trim() }
        elseif ($l -match '^\s*Elapsed time:\s*(.+)$') { $h.Waktu = $matches[1].Trim(); break }
    }
    # Blok "Transferring:" berisi file yang sedang diunggah, mis. " * DLH/a.zip: 40% /480Mi, ..."
    for ($k++; $k -lt $b.Count; $k++) {
        $l = $b[$k]
        if ($l -match '^\s*Transferring:') { continue }
        if ($l -match '^\s*\*\s+(.+?):\s*(\d+)%') { $h.Sedang += ('{0} ({1}%)' -f $matches[1], $matches[2]); continue }
        if ($l -match '^\s*\*\s+(.+?):\s') { $h.Sedang += $matches[1]; continue }
        break
    }
    return [pscustomobject]$h
}

function Baris-Error([string]$log, [int]$n = 5) {
    if (-not $log -or -not (Test-Path -LiteralPath $log)) { return @() }
    return @(Get-Content -LiteralPath $log -Encoding UTF8 | Where-Object { $_ -like '* ERROR : *' } | Select-Object -Last $n)
}

# Mencegah Windows tidur otomatis selama penyalinan (dilepas saat proses selesai).
function Jaga-Bangun([bool]$aktif) {
    try {
        if (-not ('PindahDrive.Daya' -as [type])) {
            Add-Type -Namespace PindahDrive -Name Daya -MemberDefinition '[DllImport("kernel32.dll")] public static extern uint SetThreadExecutionState(uint esFlags);'
        }
        # ES_CONTINUOUS = 0x80000000, ES_SYSTEM_REQUIRED = 0x1
        if ($aktif) { [void][PindahDrive.Daya]::SetThreadExecutionState([uint32]2147483649) }
        else        { [void][PindahDrive.Daya]::SetThreadExecutionState([uint32]2147483648) }
    } catch { }
}

# ---------------------------------------------------------------- sumber & tujuan

function Adalah-Lokal([string]$s) { return ($s -match '^[A-Za-z]:\\' -or $s -match '^\\\\') }

function Rapikan-Sumber([string]$s) {
    $s = $s.Trim().Trim('"')
    if ($s -match '^[A-Za-z]:\\?$') { return ($s.Substring(0, 2) + '\') }
    if (Adalah-Lokal $s) { return $s.TrimEnd('\') }
    return ($s -replace '\\', '/')
}

function Normalisasi-Tujuan([string]$d) {
    $d = $d.Trim().Trim('"')
    if ($d -match '^[A-Za-z]:\\') { return $d }
    if ($d -notmatch '^[^/\\:]+:') { $d = "${RemoteGoogle}:" + $d.TrimStart('/', '\') }
    return ($d -replace '\\', '/')
}

function Di-OneDrive([string]$path) {
    foreach ($o in (Daftar-OneDrive)) {
        $akar = $o.Folder.TrimEnd('\')
        if ($path -eq $akar -or $path.StartsWith($akar + '\', [StringComparison]::OrdinalIgnoreCase)) { return $true }
    }
    return $false
}

# Folder tujuan bawaan: "<akun tujuan>:Pindahan <asal>/<nama folder sumber>".
# Tanpa akun tujuan (folder di komputer): Dokumen\Pindahan <asal>\<nama>.
function Tujuan-Bawaan([string]$src, [string]$remoteTujuan = $RemoteGoogle) {
    if (Adalah-Lokal $src) { $nama = Split-Path -Leaf $src }
    else { $nama = (($src -replace '^[^:]+:', '').Trim('/') -split '/')[-1] }
    $asal = 'OneDrive'
    $remS = Remote-Dari $src
    if ($remS -and (Jenis-DariNama $remS) -eq 'google') { $asal = 'Google Drive' }
    elseif ($remS -and (Jenis-DariNama $remS) -eq 'sharepoint') { $asal = 'SharePoint' }
    elseif ((Adalah-Lokal $src) -and -not (Di-OneDrive $src)) { $asal = 'Komputer' }
    if (-not $nama -or $nama -match ':') { $nama = $asal }
    if (-not $remoteTujuan) {
        return (Join-Path ([Environment]::GetFolderPath('MyDocuments')) "Pindahan $asal\$nama")
    }
    return "${remoteTujuan}:Pindahan $asal/$nama"
}

# Menggabungkan -Sumber/-Tujuan dari baris perintah dengan pengaturan tersimpan.
function Ambil-Rute {
    $p = Baca-Json $FilePengaturan
    $src = $null; $dst = $null
    if ($p) { $src = $p.Sumber; $dst = $p.Tujuan }
    if ($Sumber) {
        $src = Rapikan-Sumber $Sumber
        if (-not $Tujuan) { $dst = Tujuan-Bawaan $src }
    }
    if ($Tujuan) { $dst = Normalisasi-Tujuan $Tujuan }
    if ($src -and -not $dst) { $dst = Tujuan-Bawaan $src }
    if ($Sumber -or $Tujuan) { Simpan-Pengaturan $src $dst }
    return [pscustomobject]@{ Sumber = $src; Tujuan = $dst }
}

function Rute-Siap($r, [switch]$PerluAkun) {
    if (-not $r.Sumber) {
        Gagal 'Folder sumber belum dipilih. Tambahkan -Sumber "<folder>" atau pilih lewat menu.'
        return $false
    }
    if (-not $r.Tujuan) {
        Gagal 'Folder tujuan belum dipilih. Tambahkan -Tujuan "<akun>:<folder>".'
        return $false
    }
    if ((Adalah-Lokal $r.Sumber) -and -not (Test-Path -LiteralPath $r.Sumber -PathType Container)) {
        Gagal "Folder sumber tidak ditemukan: $($r.Sumber)"
        return $false
    }
    # Menyalin ke dalam dirinya sendiri akan berulang tanpa akhir.
    $s = ($r.Sumber -replace '\\', '/').TrimEnd('/')
    $t = ($r.Tujuan -replace '\\', '/').TrimEnd('/')
    $awalan = $s + '/'
    if ($s.EndsWith(':')) { $awalan = $s }
    if ($t -eq $s -or $t.StartsWith($awalan, [StringComparison]::OrdinalIgnoreCase)) {
        Gagal 'Tujuan tidak boleh sama dengan sumber atau berada di dalam folder sumber.'
        return $false
    }
    if ($PerluAkun) {
        if (-not (Wajib-Rclone)) { return $false }
        $remote = @(Daftar-Remote)
        foreach ($p in @(@('sumber', $r.Sumber), @('tujuan', $r.Tujuan))) {
            $rem = Remote-Dari $p[1]
            if ($rem -and $remote -notcontains $rem) {
                Gagal "Akun '$rem' untuk $($p[0]) belum terhubung. Jalankan: pindah.ps1 hubungkan -Akun $rem"
                return $false
            }
        }
        foreach ($rem in @((Remote-Dari $r.Sumber), (Remote-Dari $r.Tujuan))) {
            if ($rem -and (Jenis-DariNama $rem) -eq 'sharepoint') {
                $m = (Baca-MetaAkun)[$rem]
                if (-not ($m -and $m.PSObject.Properties['DriveId'] -and $m.DriveId)) {
                    Gagal "Akun SharePoint '$rem' belum memilih situs & pustaka. Jalankan: pindah.ps1 situs -Akun $rem -Cari <kata>"
                    return $false
                }
            }
        }
        $remS = Remote-Dari $r.Sumber
        if ($remS) {
            $k = Konfig-Akun $remS
            if ($k.Tipe -eq 'drive' -and $k.Scope -eq 'drive.file') {
                Gagal "Akun Google '$remS' hanya diberi izin terbatas (drive.file), jadi tidak bisa membaca file Anda."
                Info "Beri izin penuh dulu: pindah.ps1 hubungkan -Akun $remS -IzinPenuh"
                return $false
            }
        }
    }
    return $true
}

function Daftar-OneDrive {
    $hasil = @()
    $kunci = 'HKCU:\Software\Microsoft\OneDrive\Accounts'
    if (Test-Path $kunci) {
        foreach ($a in Get-ChildItem $kunci) {
            $p = Get-ItemProperty $a.PSPath
            if ($p.PSObject.Properties['UserFolder'] -and $p.UserFolder -and (Test-Path -LiteralPath $p.UserFolder)) {
                $jenis = 'Pribadi'
                if ($p.PSObject.Properties['Business'] -and $p.Business -eq 1) { $jenis = 'Kantor/Sekolah' }
                $hasil += [pscustomobject]@{ Folder = $p.UserFolder; Jenis = $jenis }
            }
        }
    }
    foreach ($v in @($env:OneDriveCommercial, $env:OneDriveConsumer, $env:OneDrive)) {
        if ($v -and (Test-Path -LiteralPath $v) -and -not ($hasil | Where-Object { $_.Folder -eq $v })) {
            $hasil += [pscustomobject]@{ Folder = $v; Jenis = 'OneDrive' }
        }
    }
    return , $hasil
}

function Daftar-Subfolder([string]$src) {
    if (Adalah-Lokal $src) {
        return @(Get-ChildItem -LiteralPath $src -Directory -ErrorAction SilentlyContinue | ForEach-Object { $_.Name })
    }
    $h = Panggil-Rclone @('lsf', $src, '--dirs-only') -Tangkap
    return @($h.Out -split "`r?`n" | ForEach-Object { $_.Trim().TrimEnd('/') } | Where-Object { $_ })
}

function Gabung-Path([string]$induk, [string]$nama) {
    if (Adalah-Lokal $induk) { return (Join-Path $induk $nama) }
    if ($induk.EndsWith(':')) { return "$induk$nama" }
    return "$induk/$nama"
}

# Menghitung isi folder lokal dari metadata saja: tidak memicu unduhan OneDrive.
function Ukur-Lokal([string]$path) {
    $u = [ordered]@{ Jumlah = 0; Bytes = 0L; JumlahHanyaCloud = 0; HanyaCloud = 0L; Terbesar = 0L; TerbesarNama = '' }
    Get-ChildItem -LiteralPath $path -Recurse -File -Force -ErrorAction SilentlyContinue | ForEach-Object {
        if (Cocok-Kecualian $_.Name) { return }
        $u.Jumlah++
        $u.Bytes += $_.Length
        if ($_.Length -gt $u.Terbesar) { $u.Terbesar = $_.Length; $u.TerbesarNama = $_.FullName }
        if (([int]$_.Attributes -band $AtributHanyaCloud) -ne 0) {
            $u.JumlahHanyaCloud++
            $u.HanyaCloud += $_.Length
        }
    }
    return [pscustomobject]$u
}

function Ruang-Disk([string]$path) {
    try { return (New-Object System.IO.DriveInfo ([System.IO.Path]::GetPathRoot($path))).AvailableFreeSpace } catch { return 0L }
}

# ---------------------------------------------------------------- perintah

function Perintah-Cek {
    Judul 'Kesiapan'
    $rc = Cari-Rclone
    if ($rc) {
        $v = ((Panggil-Rclone @('version') -Tangkap).Out -split "`r?`n")[0]
        Ok "rclone terpasang: $($v.Trim())"
        Info "lokasi: $rc"
        Judul 'Akun cloud'
        $akun = Daftar-Akun
        if ($akun.Count -eq 0) { Awas 'Belum ada. Contoh: pindah.ps1 hubungkan -Akun google' }
        foreach ($a in $akun) { Ok (Ringkas-Akun $a) }
    }
    else {
        Awas 'rclone belum terpasang. Jalankan: pindah.ps1 pasang'
    }

    Judul 'Folder OneDrive di komputer ini'
    $od = Daftar-OneDrive
    if ($od.Count -eq 0) { Awas 'Tidak ditemukan. Pakai OneDrive lewat internet: pindah.ps1 hubungkan -Akun onedrive' }
    foreach ($o in $od) { Info ('[{0}] {1}' -f $o.Jenis, $o.Folder) }

    Judul 'Rute penyalinan'
    $r = Ambil-Rute
    if ($r.Sumber) { Info "Sumber : $($r.Sumber)"; Info "Tujuan : $($r.Tujuan)" }
    else { Info 'Belum dipilih. Contoh: pindah.ps1 periksa -Sumber "D:\OneDrive\Backup"' }

    $job = Baca-Job
    if (Job-Berjalan $job) { Awas "Penyalinan sedang berjalan sejak $($job.Mulai). Lihat: pindah.ps1 status" }
    return 0
}

function Perintah-Pasang {
    Judul 'Memasang rclone'
    $rc = Cari-Rclone
    if ($rc) { Ok "rclone sudah terpasang: $rc"; return 0 }
    if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
        Gagal 'winget tidak tersedia di komputer ini.'
        Info 'Unduh rclone dari https://rclone.org/downloads/ lalu taruh rclone.exe di folder skrip ini.'
        return 1
    }
    Info 'rclone adalah program open source (lisensi MIT) untuk menyalin file antar-layanan cloud.'
    Info 'Dipasang lewat winget dari repositori resmi winget (paket Rclone.Rclone, sekitar 25 MB).'
    if (-not (Konfirmasi 'Unduh & pasang rclone sekarang (menyetujui ketentuan sumber/paket winget)?')) { return 1 }
    & winget install --id Rclone.Rclone --exact --source winget --accept-source-agreements --accept-package-agreements --disable-interactivity | Out-Host
    $rc = Cari-Rclone
    if ($rc) { Ok "rclone terpasang: $rc"; return 0 }
    Gagal 'Pemasangan tidak berhasil. Tutup lalu buka lagi jendela ini, atau pasang manual dari https://rclone.org/downloads/'
    return 1
}

# Akun OneDrive bisnis punya beberapa "drive" (pustaka SharePoint), dan rclone memilih yang
# pertama, yang sering berupa pustaka sistem (PersonalCacheLibrary). Nama drive mengikuti
# bahasa akun ("Dokumen"/"Documents"), jadi yang dipakai sebagai penanda adalah alamatnya:
# OneDrive utama selalu beralamat .../Documents. Memakai token yang sudah tersimpan.
function Pilih-DriveOneDrive([string]$remote = $RemoteOneDrive) {
    $dasar = @('config', 'update', $remote, '--continue', '--non-interactive')
    $h = Panggil-Rclone ($dasar + @('--state', 'choose_type_done', '--result', 'onedrive')) -Tangkap
    $daftar = @()
    try { $daftar = @(($h.Out | ConvertFrom-Json).Option.Examples | Where-Object { $_ }) } catch { }
    if ($daftar.Count -eq 0) { Awas 'Daftar drive OneDrive tidak terbaca; memakai pilihan bawaan rclone.'; return }

    $pola = 'PersonalCache|Preservation|Preservasi|Penangguhan|Cache'
    $urut = @($daftar | Where-Object { $_.Help -notmatch $pola }) + @($daftar | Where-Object { $_.Help -match $pola })
    $pilihan = $null
    foreach ($d in $urut) {
        $j = $null
        try { $j = (Panggil-Rclone ($dasar + @('--state', 'driveid_final', '--result', $d.Value)) -Tangkap).Out | ConvertFrom-Json } catch { }
        if (-not $j -or -not $j.State) { continue }
        if ($daftar.Count -eq 1 -or "$($j.Option.Help)" -match 'URL:\s*\S+/Documents/?\s*$') {
            [void](Panggil-Rclone ($dasar + @('--state', $j.State, '--result', 'true')) -Tangkap)
            $pilihan = $d
            break
        }
    }
    if (-not $pilihan) {
        # Tidak ada yang beralamat /Documents: pakai kandidat pertama yang bukan pustaka sistem.
        $pilihan = $urut[0]
        $j = (Panggil-Rclone ($dasar + @('--state', 'driveid_final', '--result', $pilihan.Value)) -Tangkap).Out | ConvertFrom-Json
        if ($j -and $j.State) { [void](Panggil-Rclone ($dasar + @('--state', $j.State, '--result', 'true')) -Tangkap) }
        Awas "OneDrive utama tidak dikenali pasti; memakai drive: $($pilihan.Help)"
    }
    $nama = ($pilihan.Help -replace '\s*\(\w+\)\s*$', '').Trim()
    $jenis = ''
    if ($pilihan.Help -match '\((\w+)\)\s*$') { $jenis = $matches[1] }
    Ok "Memakai drive OneDrive: $nama ($jenis)"
    Simpan-MetaAkun $remote @{ Drive = "$nama ($jenis)" }
    if ($daftar.Count -gt 1) { Info ('Drive lain di akun ini (tidak dipakai): ' + ((@($daftar | Where-Object { $_.Value -ne $pilihan.Value }) | ForEach-Object { $_.Help }) -join ', ')) }
    Catat 'DriveNama' $nama
    Catat 'DriveJenis' $jenis
}

function Ringkas-Akun($a) {
    $t = "$($a.Label) [$($a.Nama)]"
    if ($a.Jenis -eq 'google') {
        $t += ' - Google Drive'
        if ($a.Scope -eq 'drive') { $t += ', izin penuh' } else { $t += ', izin terbatas (hanya tujuan)' }
        if ($a.ClientSendiri) { $t += ', client ID sendiri' } else { $t += ', client ID bersama (sering dibatasi)' }
    }
    elseif ($a.Jenis -eq 'onedrive') {
        $t += ' - OneDrive'
        if ($a.Drive) { $t += ", drive $($a.Drive)" }
    }
    elseif ($a.Jenis -eq 'sharepoint') {
        $t += ' - SharePoint'
        if ($a.DriveId) { $t += ", situs $($a.Situs), pustaka $($a.Drive)" } else { $t += ', BELUM memilih situs & pustaka' }
    }
    return $t
}

# SharePoint mengubah metadata file Office (.docx/.xlsx/...) saat diunggah, sehingga ukuran &
# checksum di tujuan berbeda dari sumber. Tanpa flag ini rclone menganggapnya rusak dan mengulang.
function Argumen-Tujuan($r) {
    $rem = Remote-Dari $r.Tujuan
    if ($rem -and (Jenis-DariNama $rem) -eq 'sharepoint') { return @('--ignore-size', '--ignore-checksum') }
    return @()
}

$PolaOffice = '\.(docx|docm|dotx|xlsx|xlsm|xltx|pptx|pptm|potx|vsdx)$'

function Perintah-Akun {
    Judul 'Akun cloud yang terhubung'
    if (-not (Wajib-Rclone)) { return 1 }
    $akun = Daftar-Akun
    if ($akun.Count -eq 0) { Info 'Belum ada. Contoh: pindah.ps1 hubungkan -Akun google' }
    foreach ($a in $akun) { Info (Ringkas-Akun $a) }
    Catat 'Akun' @($akun)
    return 0
}

# Menghubungkan (atau menyambung ulang) satu akun. Akun baru: -Akun <nama-baru> -Jenis google|onedrive.
function Perintah-Hubungkan {
    if (-not (Wajib-Rclone)) { return 1 }
    $nama = Nama-Remote $Akun
    $ada = @(Daftar-Remote) -contains $nama
    $konf = $null
    $jenis = $Jenis
    if ($ada) {
        $konf = Konfig-Akun $nama
        if ($konf.Tipe -eq 'drive') { $jenis = 'google' }
        elseif ($konf.Tipe -eq 'onedrive') { $jenis = 'onedrive'; if ((Jenis-DariNama $nama) -eq 'sharepoint') { $jenis = 'sharepoint' } }
    }
    if (-not $jenis) { $jenis = Jenis-DariNama $nama }
    if ($jenis -notin @('google', 'onedrive', 'sharepoint')) {
        Gagal "Jenis akun '$nama' tidak dikenal. Tambahkan -Jenis google, onedrive, atau sharepoint."
        return 1
    }
    Catat 'Akun' $nama
    $meta = (Baca-MetaAkun)[$nama]
    $label = $Label
    if (-not $label -and $meta) { $label = "$($meta.Label)" }
    if (-not $label) { $label = Label-Bawaan $nama $jenis }

    if ($jenis -eq 'google') {
        Judul "Menghubungkan Google Drive: $label"
        $scope = 'drive.file'
        if ($konf -and $konf.Scope) { $scope = $konf.Scope }
        if ($IzinPenuh) { $scope = 'drive' }
        $pesan = @('Browser akan terbuka. Login ke akun Google yang dimaksud, lalu klik "Izinkan"/"Allow".')
        if ($scope -eq 'drive') { $pesan += 'Izin yang diminta: baca-tulis penuh (dibutuhkan untuk menyalin DARI akun ini).' }
        else { $pesan += 'Izin yang diminta hanya "drive.file": rclone hanya bisa melihat file yang ia buat sendiri.' }
        if ($ClientId -and $SimpanBawaan) {
            Simpan-ClientBawaan $ClientId $ClientSecret
            Ok 'Client ID disimpan sebagai bawaan aplikasi.'
        }
        $clientLama = [bool]($konf -and $konf.ClientSendiri)
        $cid = $ClientId
        $csec = $ClientSecret
        if (-not $cid -and -not $clientLama) {
            $b = Baca-ClientBawaan
            if ($b) { $cid = "$($b.ClientId)"; $csec = "$($b.ClientSecret)"; $pesan += 'Memakai client ID bawaan aplikasi.' }
        }
        if (-not $cid -and -not $clientLama) {
            $pesan += 'Catatan: memakai client ID bersama rclone, yang sedang dibatasi Google dan bisa membuat penyalinan macet.'
        }
        $uji = @('about', "${nama}:", '--json')
        $paksa = $Ulang -or [bool]$ClientId -or ($ada -and $IzinPenuh -and $konf.Scope -ne 'drive')
        # Client ID baru/bawaan atau client ID bersama: buat ulang. Client ID sendiri yang lama: perbarui
        # di tempat agar secret-nya (yang tidak bisa dibaca kembali) tetap tersimpan.
        $argBuat = @('config', 'create', $nama, 'drive', "scope=$scope")
        if ($cid) { $argBuat += "client_id=$cid"; $argBuat += "client_secret=$csec" }
        $argUlang = $null
        if ($clientLama -and -not $ClientId) { $argUlang = @('config', 'update', $nama, "scope=$scope") }
        if ($ada -and -not $paksa -and -not $clientLama -and $cid) {
            Info 'Akun ini masih memakai client ID bersama. Jalankan dengan -Ulang untuk beralih ke client ID bawaan.'
        }
    }
    else {
        if ($jenis -eq 'sharepoint') {
            Judul "Menghubungkan SharePoint: $label"
            $pesan = @(
                'Browser akan terbuka. Login ke akun Microsoft kantor/sekolah yang punya akses ke situs SharePoint.',
                'Setelah itu pilih situsnya: pindah.ps1 situs -Akun ' + $nama + ' -Cari <kata>'
            )
        }
        else {
            Judul "Menghubungkan OneDrive: $label"
            $pesan = @(
                'Browser akan terbuka. Login ke akun Microsoft pemilik OneDrive ini.',
                'Sebagian akun kantor/instansi ditolak dengan pesan "Need admin approval";',
                'bila begitu, pakai folder OneDrive di komputer saja.'
            )
        }
        $uji = @('lsf', "${nama}:", '--max-depth', '1', '--dirs-only')
        $paksa = [bool]$Ulang
        $argBuat = @('config', 'create', $nama, 'onedrive')
        $argUlang = $null
    }

    if ($ada -and -not $paksa) {
        $t = Panggil-Rclone $uji -Tangkap
        if ($t.Kode -eq 0) {
            Ok 'Sudah terhubung dan bisa diakses.'
            if ($Label) { Simpan-MetaAkun $nama @{ Label = $label } }
            if ($jenis -eq 'onedrive') { Pilih-DriveOneDrive $nama }
            if ($jenis -eq 'sharepoint') { [void](Pasang-PustakaTersimpan $nama) }
            return 0
        }
        Awas 'Koneksi tersimpan tidak bisa dipakai lagi:'
        ($t.Err -split "`r?`n" | Where-Object { $_ } | Select-Object -Last 3) | ForEach-Object { Info $_ }
        if (-not (Konfirmasi 'Login ulang sekarang?')) { return 1 }
    }

    $pesan | ForEach-Object { Info $_ }
    Info 'Setelah browser menampilkan "Success!", kembali ke jendela ini.'
    Write-Host ''
    # stdout dibuang supaya token login tidak tercetak di layar/log.
    if ($ada -and $argUlang) {
        $h = Panggil-Rclone $argUlang -BuangStdout
    }
    else {
        # Hanya menghapus entri koneksi di rclone.conf; tidak menyentuh file apa pun.
        if ($ada) { [void](Panggil-Rclone @('config', 'delete', $nama) -Tangkap) }
        $h = Panggil-Rclone $argBuat -BuangStdout
    }
    if ($h.Kode -ne 0) { Gagal "Login tidak selesai (kode $($h.Kode))."; return 1 }
    Simpan-MetaAkun $nama @{ Label = $label; Jenis = $jenis }

    $t = Panggil-Rclone $uji -Tangkap
    if ($t.Kode -eq 0) {
        Ok 'Terhubung.'
        if ($jenis -eq 'onedrive') { Pilih-DriveOneDrive $nama }
        if ($jenis -eq 'sharepoint') { [void](Pasang-PustakaTersimpan $nama) }
        return 0
    }
    Gagal 'Login tersimpan tetapi akses gagal:'
    ($t.Err -split "`r?`n" | Where-Object { $_ } | Select-Object -Last 3) | ForEach-Object { Info $_ }
    return 1
}

# ---------------------------------------------------------------- SharePoint
# Situs & pustaka dipilih lewat mesin konfigurasi rclone yang "dilompati" langsung ke langkahnya:
#   search_end   (kata kunci) -> daftar situs
#   search_sites (id situs)   -> daftar pustaka dokumen
#   driveid_final (id drive)  -> pakai pustaka itu
# Alur "url" bawaan rclone gagal di tenant tertentu ("Invalid hostname"), jadi URL diubah jadi pencarian.

# Langkah konfigurasi butuh access token yang masih berlaku; operasi biasa memperbaruinya.
function Segarkan-Token([string]$nama) { [void](Panggil-Rclone @('about', "${nama}:") -Tangkap) }

function Langkah-Konfig([string]$nama, [string]$state, [string]$hasil) {
    $h = Panggil-Rclone @('config', 'update', $nama, '--continue', '--non-interactive', '--state', $state, '--result', $hasil) -Tangkap
    try { return ($h.Out | ConvertFrom-Json) } catch { return $null }
}

function Atur-Pustaka([string]$nama, [string]$driveId) {
    Segarkan-Token $nama
    $j = Langkah-Konfig $nama 'driveid_final' $driveId
    if (-not $j -or -not $j.State) {
        $e = ''; if ($j) { $e = $j.Error }
        Gagal "Pustaka tidak bisa dipakai. $e"
        return $false
    }
    [void](Langkah-Konfig $nama $j.State 'true')
    return $true
}

# Setelah login ulang (koneksi dibuat ulang), pasang lagi pustaka yang dulu dipilih.
function Pasang-PustakaTersimpan([string]$nama) {
    $m = (Baca-MetaAkun)[$nama]
    if ($m -and $m.PSObject.Properties['DriveId'] -and $m.DriveId) {
        if (Atur-Pustaka $nama "$($m.DriveId)") { Ok "Memakai pustaka: $($m.Drive)"; return $true }
        return $false
    }
    Info "Belum memilih situs & pustaka. Lanjutkan: pindah.ps1 situs -Akun $nama -Cari <kata>"
    return $false
}

function Perintah-Situs {
    if (-not (Wajib-Rclone)) { return 1 }
    $nama = Nama-Remote $Akun
    if (@(Daftar-Remote) -notcontains $nama) { Gagal "Akun $nama belum terhubung."; return 1 }
    if (-not $Cari) { Gagal 'Tambahkan kata kunci: -Cari <kata>'; return 1 }
    Judul "Mencari situs SharePoint: $Cari"
    Segarkan-Token $nama
    $j = Langkah-Konfig $nama 'search_end' $Cari
    if (-not $j -or $j.Error) {
        $e = ''; if ($j) { $e = $j.Error }
        Gagal "Pencarian gagal. $e"
        Catat 'Situs' @()
        return 1
    }
    $situs = @(@($j.Option.Examples) | Where-Object { $_ } | ForEach-Object {
            $teks = "$($_.Help)"; $url = ''; $nm = $teks
            if ($teks -match '^(.*?)\s*\((https?://[^)]+)\)\s*$') { $nm = $matches[1]; $url = $matches[2] }
            [pscustomobject]@{ Nama = $nm; Url = $url; Id = "$($_.Value)" }
        })
    if ($situs.Count -eq 0) { Info 'Tidak ada situs yang cocok.' }
    foreach ($s in $situs) { Info ('{0}  {1}' -f $s.Nama, $s.Url) }
    Catat 'Situs' $situs
    return 0
}

function Perintah-Pustaka {
    if (-not (Wajib-Rclone)) { return 1 }
    $nama = Nama-Remote $Akun
    if (@(Daftar-Remote) -notcontains $nama) { Gagal "Akun $nama belum terhubung."; return 1 }

    if ($Pustaka) {
        Judul 'Memakai pustaka SharePoint'
        if (-not (Atur-Pustaka $nama $Pustaka)) { return 1 }
        $t = Panggil-Rclone @('lsf', "${nama}:", '--max-depth', '1') -Tangkap
        if ($t.Kode -ne 0) { Gagal 'Pustaka terpasang tetapi tidak bisa dibaca (periksa hak akses Anda di situs itu).'; return 1 }
        $ubah = @{ Jenis = 'sharepoint'; DriveId = $Pustaka; Drive = "$PustakaNama"; Situs = "$SitusNama" }
        $m = (Baca-MetaAkun)[$nama]
        $labelLama = ''
        if ($m) { $labelLama = "$($m.Label)" }
        if (-not $labelLama -or $labelLama -like 'SharePoint*') { $ubah.Label = ('{0} - {1}' -f $SitusNama, $PustakaNama).Trim(' -') }
        Simpan-MetaAkun $nama $ubah
        Ok ('Memakai situs "{0}", pustaka "{1}".' -f $SitusNama, $PustakaNama)
        return 0
    }

    if (-not $SitusId) { Gagal 'Tambahkan -SitusId <id> (dari perintah situs) atau -Pustaka <id>.'; return 1 }
    Judul "Pustaka dokumen di situs $SitusNama"
    Segarkan-Token $nama
    $j = Langkah-Konfig $nama 'search_sites' $SitusId
    if (-not $j -or $j.Error) {
        $e = ''; if ($j) { $e = $j.Error }
        Gagal "Pustaka tidak bisa dibaca. $e"
        Catat 'Pustaka' @()
        return 1
    }
    $daftar = @(@($j.Option.Examples) | Where-Object { $_ } | ForEach-Object {
            [pscustomobject]@{ Nama = ("$($_.Help)" -replace '\s*\(\w+\)\s*$', '').Trim(); Id = "$($_.Value)" }
        })
    foreach ($p in $daftar) { Info $p.Nama }
    Catat 'Pustaka' $daftar
    return 0
}

# Menjadikan client ID milik sebuah akun Google sebagai bawaan aplikasi. Nilainya dibaca dari
# rclone.conf di dalam proses ini dan langsung ditulis ke client-google.json; tidak pernah dicetak.
function Perintah-Bawaan {
    if (-not (Wajib-Rclone)) { return 1 }
    $nama = Nama-Remote $Akun
    if (@(Daftar-Remote) -notcontains $nama) { Gagal "Akun $nama tidak terhubung."; return 1 }
    $semua = $null
    try { $semua = (Panggil-Rclone @('config', 'dump') -Tangkap).Out | ConvertFrom-Json } catch { }
    $r = $null
    if ($semua -and $semua.PSObject.Properties[$nama]) { $r = $semua.PSObject.Properties[$nama].Value }
    if (-not $r -or $r.type -ne 'drive') { Gagal "Akun $nama bukan akun Google Drive."; return 1 }
    if (-not $r.client_id -or -not $r.client_secret) {
        Gagal "Akun $nama masih memakai client ID bersama rclone; pasang client ID sendiri dulu."
        return 1
    }
    Simpan-ClientBawaan "$($r.client_id)" "$($r.client_secret)"
    Ok "Client ID akun $nama disimpan sebagai bawaan aplikasi."
    Info "Lokasi: $FileClientPengguna (ikut dibawa installer berikutnya)."
    return 0
}

# Melepas akun dari Pindah Drive: hanya menghapus koneksi di rclone.conf, bukan file di cloud.
function Perintah-Lepas {
    if (-not (Wajib-Rclone)) { return 1 }
    $nama = Nama-Remote $Akun
    if (@(Daftar-Remote) -notcontains $nama) { Info "Akun $nama tidak terhubung."; Simpan-MetaAkun $nama @{} -Hapus; return 0 }
    $job = Baca-Job
    if ((Job-Berjalan $job) -and ((Remote-Dari $job.Sumber) -eq $nama -or (Remote-Dari $job.Tujuan) -eq $nama)) {
        Gagal 'Akun ini sedang dipakai oleh penyalinan yang berjalan. Hentikan dulu.'
        return 1
    }
    if (-not (Konfirmasi "Lepas akun $nama dari Pindah Drive? File di cloud tidak terhapus.")) { return 1 }
    [void](Panggil-Rclone @('config', 'delete', $nama) -Tangkap)
    Simpan-MetaAkun $nama @{} -Hapus
    Ok "Akun $nama dilepas."
    Info 'Untuk mencabut izin sepenuhnya, hapus akses "rclone" di pengaturan keamanan akun Google/Microsoft Anda.'
    return 0
}

# Kode keluar: 0 siap, 1 belum siap/error, 2 ruang Google Drive kurang, 3 ada peringatan.
function Perintah-Periksa {
    $r = Ambil-Rute
    if (-not (Rute-Siap $r)) { return 1 }
    $hasil = 0
    $srcBytes = 0L

    Judul 'Sumber'
    Info $r.Sumber
    if (Adalah-Lokal $r.Sumber) {
        Info 'Menghitung isi folder (hanya membaca daftar file, tidak mengunduh apa pun)...'
        $u = Ukur-Lokal $r.Sumber
        $srcBytes = $u.Bytes
        Catat 'SumberJumlah' $u.Jumlah; Catat 'SumberBytes' $u.Bytes
        Catat 'HanyaCloudJumlah' $u.JumlahHanyaCloud; Catat 'HanyaCloudBytes' $u.HanyaCloud
        Catat 'DiskCukup' $true
        Ok ('{0} file, total {1}' -f $u.Jumlah, (Format-Ukuran $u.Bytes))
        if ($u.Terbesar -gt 0) { Info ('File terbesar: {0} ({1})' -f $u.TerbesarNama, (Format-Ukuran $u.Terbesar)) }
        if ($u.JumlahHanyaCloud -gt 0) {
            $bebas = Ruang-Disk $r.Sumber
            Catat 'DiskBebas' $bebas
            Info ('{0} file ({1}) masih berstatus "hanya online" di OneDrive.' -f $u.JumlahHanyaCloud, (Format-Ukuran $u.HanyaCloud))
            Info ('Saat disalin, OneDrive mengunduhnya dulu ke disk ini. Sisa ruang disk: {0}' -f (Format-Ukuran $bebas))
            if ($u.HanyaCloud -gt ($bebas - 2GB)) {
                Catat 'DiskCukup' $false
                Awas 'Ruang disk tidak cukup untuk mengunduh semuanya sekaligus.'
                Info 'Salin per subfolder, lalu klik kanan subfolder itu di OneDrive > "Kosongkan ruang",'
                Info 'atau pakai OneDrive lewat internet: pindah.ps1 hubungkan -Akun onedrive'
                $hasil = 3
            }
        }
        Info 'Catatan: hanya folder yang disinkronkan OneDrive ke komputer ini yang ikut terhitung.'
    }
    elseif (Wajib-Rclone) {
        Info 'Menghitung isi folder di cloud...'
        $h = Panggil-Rclone (@('size', $r.Sumber, '--json') + (Argumen-Kecualian)) -Tangkap
        if ($h.Kode -ne 0) { Gagal 'Tidak bisa membaca sumber.'; Info $h.Err.Trim(); return 1 }
        $j = $h.Out | ConvertFrom-Json
        $srcBytes = [int64]$j.bytes
        Catat 'SumberJumlah' ([int64]$j.count); Catat 'SumberBytes' $srcBytes
        Ok ('{0} file, total {1}' -f $j.count, (Format-Ukuran $j.bytes))
    }
    else { return 1 }

    Judul 'Tujuan'
    Info $r.Tujuan
    Catat 'TujuanTerhubung' $false
    $remT = Remote-Dari $r.Tujuan
    $jenisT = ''
    if ($remT) {
        if (-not (Wajib-Rclone)) { return 1 }
        if (@(Daftar-Remote) -notcontains $remT) { Awas "Akun '$remT' belum terhubung. Jalankan: pindah.ps1 hubungkan -Akun $remT"; return 1 }
        $jenisT = Jenis-Remote $remT
    }
    Catat 'TujuanJenis' $jenisT

    $sudah = 0L
    if ($remT -or (Test-Path -LiteralPath $r.Tujuan -PathType Container)) {
        $h = Panggil-Rclone @('size', $r.Tujuan, '--json') -Tangkap
        if ($h.Kode -eq 0) {
            $sudah = [int64]($h.Out | ConvertFrom-Json).bytes
            if ($sudah -gt 0) { Info ('Sudah ada di tujuan: {0} (yang sama akan dilewati)' -f (Format-Ukuran $sudah)) }
        }
    }
    # [int64] wajib: [Math]::Max(0, ...) memilih versi Int32 dan gagal untuk ukuran > 2 GB.
    $butuh = [Math]::Max([int64]0, [int64]$srcBytes - [int64]$sudah)
    Catat 'TujuanSudah' $sudah; Catat 'Butuh' $butuh

    $total = 0L; $terpakai = 0L; $bebas = -1L
    if ($remT) {
        $h = Panggil-Rclone @('about', "${remT}:", '--json') -Tangkap
        if ($h.Kode -ne 0) {
            Gagal 'Tidak bisa membaca kuota akun tujuan.'; Info $h.Err.Trim()
            Catat 'TujuanGalat' $true
            return 1
        }
        $a = $h.Out | ConvertFrom-Json
        if ($a.PSObject.Properties['total'] -and $a.total) {
            $total = [int64]$a.total; $terpakai = [int64]$a.used
            $bebas = $total - $terpakai
            if ($a.PSObject.Properties['free'] -and $a.free) { $bebas = [int64]$a.free }
            if ($a.PSObject.Properties['trashed'] -and [int64]$a.trashed -gt 0) {
                Catat 'TujuanSampah' ([int64]$a.trashed)
                Info ('Sampah di akun ini memakai {0} (ikut dihitung dalam kuota).' -f (Format-Ukuran $a.trashed))
            }
        }
    }
    else {
        $bebas = [int64](Ruang-Disk $r.Tujuan)
        $total = $bebas
    }
    Catat 'TujuanTerhubung' $true
    if ($bebas -ge 0) {
        Catat 'TujuanTotal' $total; Catat 'TujuanTerpakai' $terpakai; Catat 'TujuanBebas' $bebas
        Catat 'TujuanCukup' ($butuh -le $bebas)
        if ($remT) { Info ('Kapasitas {0}, terpakai {1}, sisa {2}' -f (Format-Ukuran $total), (Format-Ukuran $terpakai), (Format-Ukuran $bebas)) }
        else { Info ('Sisa ruang disk tujuan: {0}' -f (Format-Ukuran $bebas)) }
        Info ('Perlu disalin sekitar {0}' -f (Format-Ukuran $butuh))
        if ($butuh -gt $bebas) {
            Gagal 'Ruang di tujuan TIDAK cukup.'
            Info 'Pilihan: kosongkan sampah/ruang di tujuan, salin per subfolder, atau tambah kuota.'
            return 2
        }
    }
    else { Info 'Kuota tidak terbatas atau tidak dilaporkan.'; Catat 'TujuanCukup' $true }
    if ($jenisT -eq 'google' -and $butuh -gt 700GB) { Awas 'Google membatasi unggahan 750 GB per hari; penyalinan akan berhenti sendiri dan bisa dilanjutkan besok.' }

    if ($hasil -eq 0) { Ok 'Siap disalin.' }
    return $hasil
}

# Kode keluar: 0 aman, 1 error, 3 ada file yang bermasalah/terlewat.
function Perintah-Uji {
    $r = Ambil-Rute
    if (-not (Rute-Siap $r -PerluAkun)) { return 1 }
    Judul 'Uji coba (dry run): tidak ada file yang disalin'
    $log = Path-Log 'uji'
    $arg = @('copy', $r.Sumber, $r.Tujuan, '--dry-run', '--create-empty-src-dirs') + (Argumen-Kecualian) + (Argumen-Tujuan $r) +
           @('--log-level', 'NOTICE', '--stats', '1h', '--stats-log-level', 'NOTICE')
    $h = Panggil-Rclone $arg -Tangkap
    Set-Content -LiteralPath $log -Value $h.Err -Encoding UTF8
    $baris = @($h.Err -split "`r?`n")

    if ($h.Kode -ne 0) {
        Gagal "rclone berhenti dengan kode $($h.Kode):"
        $baris | Where-Object { $_ } | Select-Object -Last 8 | ForEach-Object { Info $_ }
        Info "Log lengkap: $log"
        return 1
    }

    $salin = @($baris | Where-Object { $_ -match 'Skipped copy as --dry-run' })
    $total = 0.0
    foreach ($b in $salin) { if ($b -match '\(size ([\d.]+)\s*([A-Za-z]*)\)') { $total += Ke-Byte $matches[1] $matches[2] } }
    $masalah = @($baris | Where-Object { $_ -match "ERROR|non file|non-regular|irregular|symlink|Can't transfer|Can't follow" })
    Catat 'AkanDisalin' $salin.Count; Catat 'AkanBytes' ([int64]$total)
    Catat 'Masalah' $masalah.Count; Catat 'MasalahContoh' @($masalah | Select-Object -First 5)
    Catat 'Terlewat' 0

    if ($salin.Count -eq 0) { Ok 'Tidak ada yang perlu disalin: semua file sudah ada di tujuan.' }
    else { Ok ('{0} file akan disalin (sekitar {1}).' -f $salin.Count, (Format-Ukuran $total)) }

    $hasil = 0
    if (Adalah-Lokal $r.Sumber) {
        # Bandingkan jumlah file versi Windows dengan versi rclone: selisih berarti ada yang terlewat
        # (mis. placeholder OneDrive yang dianggap "bukan file biasa").
        $u = Ukur-Lokal $r.Sumber
        $s = Panggil-Rclone (@('size', $r.Sumber, '--json') + (Argumen-Kecualian)) -Tangkap
        if ($s.Kode -eq 0) {
            $nr = [int64]($s.Out | ConvertFrom-Json).count
            if ($nr -lt $u.Jumlah) {
                Catat 'Terlewat' ($u.Jumlah - $nr)
                Awas ('rclone hanya melihat {0} dari {1} file. {2} file akan TERLEWAT.' -f $nr, $u.Jumlah, ($u.Jumlah - $nr))
                Info 'Solusi: pakai OneDrive lewat internet (pindah.ps1 hubungkan -Akun onedrive,'
                Info 'lalu -Sumber "pd-onedrive:<folder>").'
                $hasil = 3
            }
            else { Ok ('Semua {0} file di folder terbaca oleh rclone.' -f $u.Jumlah) }
        }
    }
    if ($masalah.Count -gt 0) {
        Awas ('{0} catatan masalah:' -f $masalah.Count)
        $masalah | Select-Object -First 10 | ForEach-Object { Info $_ }
        $hasil = 3
    }
    Info "Log lengkap: $log"
    return $hasil
}

function Ringkas-Hasil([string]$log, $kode) {
    Judul 'Hasil penyalinan'
    $s = Baca-Statistik $log
    if ($s) {
        Info "Data   : $($s.Data)"
        if ($s.File)  { Info "File   : $($s.File)" }
        if ($s.Error) { Awas "Error  : $($s.Error)" }
        if ($s.Waktu) { Info "Waktu  : $($s.Waktu)" }
    }
    if ("$kode" -eq '0') {
        Ok 'Selesai. Lanjutkan dengan verifikasi: pindah.ps1 verifikasi'
    }
    elseif ((Get-Content -LiteralPath $log -Tail 50 -Encoding UTF8 -ErrorAction SilentlyContinue) -match 'upload limit|uploadLimitExceeded') {
        Awas 'Batas unggah harian Google (750 GB) tercapai. Jalankan "salin" lagi besok; file yang sudah ada dilewati.'
    }
    else {
        Gagal "Ada yang gagal (kode $kode)."
        Baris-Error $log 5 | ForEach-Object { Info $_ }
        Info 'Jalankan "salin" lagi: file yang sudah tersalin dilewati, sisanya dicoba ulang.'
    }
    Info "Log: $log"
}

function Jalankan-Salin($r, [string]$log, [bool]$tampil) {
    $arg = @('copy', $r.Sumber, $r.Tujuan, '--create-empty-src-dirs') + (Argumen-Kecualian) + (Argumen-Tujuan $r) + @(
        '--transfers', '4', '--checkers', '8',
        # low-level-retries rendah: saat Google membatasi (rate limit), error cepat muncul di log
        # alih-alih rclone diam menunggu berpuluh menit; --retries mengulang file yang gagal.
        '--retries', '5', '--low-level-retries', '10',
        '--drive-chunk-size', '64M', '--drive-stop-on-upload-limit',
        '--log-file', $log, '--log-level', 'INFO', '--stats', '5s')
    if ($tampil) { $arg += '--progress' }

    $kode = 'terputus'
    Jaga-Bangun $true
    try {
        $p = Mulai-Rclone $arg
        Perbarui-Job @{ PidInduk = $PID; PidRclone = $p.Id }
        $p.WaitForExit()
        $kode = $p.ExitCode
    }
    finally {
        Jaga-Bangun $false
        Perbarui-Job @{ Selesai = (Get-Date).ToString('s'); Kode = $kode }
    }
    if ($tampil) { Ringkas-Hasil $log $kode }
    return $kode
}

function Perintah-Salin {
    $r = Ambil-Rute
    if (-not (Rute-Siap $r -PerluAkun)) { return 1 }

    # Proses anak yang dijalankan oleh "salin -Latar".
    if ($LogLatar) { return (Jalankan-Salin $r $LogLatar $false) }

    if (Job-Berjalan (Baca-Job)) { Awas 'Penyalinan lain masih berjalan. Lihat: pindah.ps1 status'; return 1 }

    Judul 'Mulai menyalin'
    Info "Dari : $($r.Sumber)"
    Info "Ke   : $($r.Tujuan)"
    Info 'Mode : SALIN saja. File di sumber tidak diubah atau dihapus.'
    if (-not (Konfirmasi 'Mulai menyalin sekarang?')) { return 1 }

    $log = Path-Log 'salin'
    Perbarui-Job -Baru @{ Log = $log; Mulai = (Get-Date).ToString('s'); Sumber = $r.Sumber; Tujuan = $r.Tujuan }

    if ($Latar) {
        $argPs = Gabung-Argumen @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $PSCommandPath, 'salin', '-LogLatar', $log)
        $p = Start-Process -FilePath 'powershell.exe' -ArgumentList $argPs -WindowStyle Hidden -PassThru
        Perbarui-Job @{ PidInduk = $p.Id }
        Catat 'Pid' $p.Id; Catat 'Log' $log
        Ok "Berjalan di latar belakang (PID $($p.Id)). Jendela ini boleh ditutup."
        Info 'Pantau   : pindah.ps1 status'
        Info 'Hentikan : pindah.ps1 berhenti  (bisa dilanjutkan kapan saja)'
        return 0
    }
    Info 'Tekan Ctrl+C untuk berhenti; jalankan lagi nanti untuk melanjutkan.'
    return (Jalankan-Salin $r $log $true)
}

function Perintah-Status {
    $job = Baca-Job
    if (-not $job) { Info 'Belum pernah ada penyalinan.'; return 0 }
    Judul 'Status penyalinan'
    Info "Dari   : $($job.Sumber)"
    Info "Ke     : $($job.Tujuan)"
    Info "Mulai  : $($job.Mulai)"
    if (Job-Berjalan $job) { Ok 'Keadaan: SEDANG BERJALAN' }
    elseif ("$($job.Kode)" -eq '0') { Ok "Keadaan: SELESAI ($($job.Selesai))" }
    elseif ($job.Selesai) { Awas "Keadaan: berhenti, kode $($job.Kode) ($($job.Selesai))" }
    else { Awas 'Keadaan: berhenti (jendela ditutup, komputer mati, atau dihentikan)' }

    $s = Baca-Statistik $job.Log
    if ($s) {
        Info "Data   : $($s.Data)"
        if ($s.File)  { Info "File   : $($s.File)" }
        if ($s.Error) { Awas "Error  : $($s.Error)" }
        if ($s.Waktu) { Info "Waktu  : $($s.Waktu)" }
    }
    else { Info 'Statistik belum tersedia (ditulis tiap 5 detik).' }
    if ($s -and $s.Dibatasi) {
        Awas 'Layanan cloud sedang membatasi permintaan (rate limit); rclone menunggu lalu mencoba lagi.'
        Info 'Untuk Google Drive, pakai client ID sendiri: pindah.ps1 hubungkan -Akun <akun> -ClientId X -ClientSecret Y'
    }
    $err = @(Baris-Error $job.Log 5)
    if ($err.Count -gt 0) { Awas 'Error terakhir:'; $err | ForEach-Object { Info $_ } }
    Info "Log    : $($job.Log)"
    return 0
}

function Perintah-Berhenti {
    $job = Baca-Job
    if (-not (Job-Berjalan $job)) { Info 'Tidak ada penyalinan yang sedang berjalan.'; return 0 }
    if (Proses-Hidup $job.PidRclone 'rclone') { Stop-Process -Id ([int]$job.PidRclone) -Force -ErrorAction SilentlyContinue }
    if (Proses-Hidup $job.PidInduk 'powershell') {
        # Proses induk latar belakang juga dihentikan; jendela menu milik pengguna tidak.
        if ([int]$job.PidInduk -ne $PID -and $job.Log -and (Get-CimInstance Win32_Process -Filter "ProcessId=$([int]$job.PidInduk)").CommandLine -match 'LogLatar') {
            Stop-Process -Id ([int]$job.PidInduk) -Force -ErrorAction SilentlyContinue
        }
    }
    Perbarui-Job @{ Selesai = (Get-Date).ToString('s'); Kode = 'dihentikan' }
    Ok 'Penyalinan dihentikan. Jalankan "salin" lagi untuk melanjutkan; file yang sudah tersalin dilewati.'
    return 0
}

function Perintah-Verifikasi {
    $r = Ambil-Rute
    if (-not (Rute-Siap $r -PerluAkun)) { return 1 }
    if (Job-Berjalan (Baca-Job)) { Awas 'Tunggu penyalinan selesai dulu (lihat: pindah.ps1 status).'; return 1 }

    Judul 'Verifikasi: mencocokkan sumber dengan tujuan'
    Info 'Membandingkan setiap file: dengan checksum bila kedua sisi punya jenis yang sama, selain itu nama & ukuran...'
    $lap = Path-Log 'verifikasi' 'txt'
    $arg = @('check', $r.Sumber, $r.Tujuan, '--one-way', '--combined', $lap) + (Argumen-Kecualian) + @('--log-level', 'NOTICE')
    $h = Panggil-Rclone $arg -Tangkap
    # rclone: "No common hash found - not using a hash for checks" (mis. OneDrive <-> Google Drive)
    $metode = 'checksum'
    if ($h.Err -match 'No common hash|no hashes in common') { $metode = 'ukuran' }
    Catat 'Metode' $metode
    if ($metode -eq 'ukuran') { Info 'Kedua layanan tidak punya jenis checksum yang sama; dibandingkan per nama & ukuran.' }
    else { Info 'Dibandingkan dengan checksum (isi file).' }

    $isi = @()
    if (Test-Path -LiteralPath $lap) { $isi = @(Get-Content -LiteralPath $lap -Encoding UTF8) }
    $sama   = @($isi | Where-Object { $_.StartsWith('= ') })
    $hilang = @($isi | Where-Object { $_.StartsWith('- ') })
    $beda   = @($isi | Where-Object { $_.StartsWith('* ') })
    $galat  = @($isi | Where-Object { $_.StartsWith('! ') })
    # Tujuan SharePoint: file Office yang "berbeda" adalah akibat metadata yang ditulis SharePoint.
    $office = @()
    $remT = Remote-Dari $r.Tujuan
    if ($remT -and (Jenis-DariNama $remT) -eq 'sharepoint') {
        $office = @($beda | Where-Object { $_ -match $PolaOffice })
        $beda = @($beda | Where-Object { $_ -notmatch $PolaOffice })
        $sama = @($sama + $office)
        if ($office.Count -gt 0) { Info ('{0} file Office berbeda ukuran karena metadata yang ditambahkan SharePoint; dianggap cocok.' -f $office.Count) }
    }
    Catat 'OfficeSharePoint' $office.Count
    Catat 'Cocok' $sama.Count; Catat 'Hilang' $hilang.Count; Catat 'Beda' $beda.Count; Catat 'Galat' $galat.Count
    Catat 'Contoh' @(@($hilang + $beda + $galat) | Select-Object -First 5)
    Catat 'Laporan' $lap

    Info ('Cocok                      : {0} file' -f $sama.Count)
    if ($hilang.Count) { Awas ('Belum ada di tujuan        : {0} file' -f $hilang.Count) }
    if ($beda.Count)   { Awas ('Isinya berbeda             : {0} file' -f $beda.Count) }
    if ($galat.Count)  { Awas ('Gagal dibaca               : {0} file' -f $galat.Count) }

    Catat 'Sumber' $r.Sumber
    if ($isi.Count -gt 0 -and $hilang.Count -eq 0 -and $beda.Count -eq 0 -and $galat.Count -eq 0) {
        Ok ('SEMUA FILE COCOK: {0} file dari {1}' -f $sama.Count, $r.Sumber)
        Info "sudah ada di $($r.Tujuan)."
        Info 'File di sumber tidak diubah. Hapus sendiri dari sumber bila sudah yakin.'
        Info "Laporan: $lap"
        return 0
    }
    if ($isi.Count -eq 0) {
        Gagal "Verifikasi tidak berjalan (kode $($h.Kode)):"
        ($h.Err -split "`r?`n" | Where-Object { $_ } | Select-Object -Last 5) | ForEach-Object { Info $_ }
        return 1
    }
    @($hilang + $beda + $galat) | Select-Object -First 10 | ForEach-Object { Info $_ }
    Info 'Jalankan "salin" lagi untuk melengkapi, lalu verifikasi ulang.'
    Info "Laporan lengkap: $lap"
    return 1
}

# ---------------------------------------------------------------- menu

function Pilih-Rute {
    Judul 'Pilih folder SUMBER (OneDrive)'
    $opsi = @()
    foreach ($o in (Daftar-OneDrive)) { $opsi += [pscustomobject]@{ Label = "[$($o.Jenis)] $($o.Folder)"; Nilai = $o.Folder } }
    $opsi += [pscustomobject]@{ Label = 'Ketik alamat folder lain'; Nilai = '__ketik' }
    $opsi += [pscustomobject]@{ Label = 'OneDrive lewat internet (login Microsoft)'; Nilai = '__cloud' }
    for ($i = 0; $i -lt $opsi.Count; $i++) { Info ('{0}. {1}' -f ($i + 1), $opsi[$i].Label) }
    $pilih = $opsi[(Baca-Angka 'Nomor' 1 $opsi.Count) - 1].Nilai

    if ($pilih -eq '__ketik') {
        $pilih = Rapikan-Sumber (Read-Host '        Alamat folder')
        if (-not (Test-Path -LiteralPath $pilih -PathType Container)) { Gagal 'Folder tidak ditemukan.'; return $false }
    }
    elseif ($pilih -eq '__cloud') {
        if ((Daftar-Remote) -notcontains $RemoteOneDrive) {
            $script:Akun = 'onedrive'
            $k = Perintah-Hubungkan
            $script:Akun = 'google'
            if ($k -ne 0) { return $false }
        }
        $pilih = "${RemoteOneDrive}:"
    }

    # @() wajib: PowerShell "membuka" array berisi satu elemen menjadi string biasa.
    $sub = @(Daftar-Subfolder $pilih)
    if ($sub.Count -gt 0) {
        Judul 'Salin semua isinya, atau satu subfolder saja?'
        Info '0. Semua isi'
        for ($i = 0; $i -lt $sub.Count; $i++) { Info ('{0}. {1}' -f ($i + 1), $sub[$i]) }
        $n = Baca-Angka 'Nomor' 0 $sub.Count
        if ($n -gt 0) { $pilih = Gabung-Path $pilih $sub[$n - 1] }
    }

    $usul = Tujuan-Bawaan $pilih
    Judul 'Folder TUJUAN di Google Drive'
    Info ('Usulan: {0}' -f ($usul -replace "^${RemoteGoogle}:", ''))
    $t = (Read-Host '        Tekan Enter untuk memakai usulan, atau ketik nama lain').Trim()
    $tujuan = $usul
    if ($t) { $tujuan = Normalisasi-Tujuan $t }
    Simpan-Pengaturan $pilih $tujuan
    Ok "Sumber: $pilih"
    Ok "Tujuan: $tujuan"
    return $true
}

function Langkah-Dipandu {
    Judul 'LANGKAH 1/6 - rclone'
    if (Cari-Rclone) { Ok 'rclone sudah terpasang.' }
    elseif ((Perintah-Pasang) -ne 0) { return 1 }

    Judul 'LANGKAH 2/6 - Akun Google Drive'
    $script:Akun = 'google'
    if ((Perintah-Hubungkan) -ne 0) { return 1 }

    Judul 'LANGKAH 3/6 - Folder sumber & tujuan'
    $r = Ambil-Rute
    $pakai = $false
    if ($r.Sumber) {
        Info "Pilihan tersimpan: $($r.Sumber)  ->  $($r.Tujuan)"
        $pakai = Konfirmasi 'Pakai pilihan ini?'
    }
    if (-not $pakai -and -not (Pilih-Rute)) { return 1 }

    Judul 'LANGKAH 4/6 - Periksa ukuran & ruang'
    $k = Perintah-Periksa
    if ($k -eq 1 -or $k -eq 2) { return 1 }
    if ($k -eq 3 -and -not (Konfirmasi 'Ada peringatan di atas. Tetap lanjut?')) { return 1 }

    Judul 'LANGKAH 5/6 - Uji coba'
    $k = Perintah-Uji
    if ($k -eq 1) { return 1 }
    if ($k -eq 3 -and -not (Konfirmasi 'Uji coba menemukan masalah. Tetap lanjut?')) { return 1 }

    Judul 'LANGKAH 6/6 - Salin lalu verifikasi'
    $k = Perintah-Salin
    if ("$k" -ne '0') { return 1 }
    return (Perintah-Verifikasi)
}

function Tampilkan-Bantuan {
    Write-Host 'Pindah Drive: salin file antar OneDrive, Google Drive, dan folder komputer (rclone).'
    Write-Host ''
    Write-Host 'Perintah:'
    Write-Host '  cek                         status rclone, akun, folder OneDrive, rute'
    Write-Host '  akun                        daftar akun cloud yang terhubung'
    Write-Host '  pasang [-Ya]                pasang rclone lewat winget'
    Write-Host '  hubungkan -Akun <nama> [-Jenis google|onedrive] [-Label X] [-IzinPenuh] [-Ulang]'
    Write-Host '            [-ClientId X -ClientSecret Y [-SimpanBawaan]]   login akun (di browser)'
    Write-Host '  lepas -Akun <nama>          lepas akun (file di cloud tidak terhapus)'
    Write-Host '  bawaan -Akun <nama>         jadikan client ID akun Google itu bawaan aplikasi'
    Write-Host '  situs -Akun <sharepoint> -Cari <kata>              cari situs SharePoint'
    Write-Host '  pustaka -Akun <sharepoint> -SitusId <id>           daftar pustaka dokumen situs'
    Write-Host '  pustaka -Akun <sharepoint> -Pustaka <id> [-SitusNama X -PustakaNama Y]   pakai pustaka'
    Write-Host '  periksa [-Sumber X] [-Tujuan Y]     ukuran sumber vs ruang tujuan'
    Write-Host '          X/Y: D:\Folder  atau  <akun>:<folder>, mis. pd-onedrive:Kerja  pd-gdrive-2:Arsip'
    Write-Host '  uji                         dry run, deteksi file yang akan terlewat'
    Write-Host '  salin [-Latar] [-Ya]        mulai menyalin (bisa dilanjutkan)'
    Write-Host '  status | berhenti           pantau / hentikan penyalinan latar'
    Write-Host '  verifikasi                  cocokkan isi sumber dan tujuan'
}

function Tampilkan-Menu {
    if (-not (Bisa-Bertanya)) { Tampilkan-Bantuan; return }
    while ($true) {
        Clear-Host
        $r = Ambil-Rute
        $s = '(belum dipilih)'; if ($r.Sumber) { $s = $r.Sumber }
        $t = '(belum dipilih)'; if ($r.Tujuan) { $t = $r.Tujuan -replace "^${RemoteGoogle}:", 'Google Drive: ' }
        Write-Host ''
        Write-Host '  PINDAH DRIVE  |  OneDrive  ->  Google Drive' -ForegroundColor Cyan
        Write-Host '  -------------------------------------------'
        Write-Host "  Sumber : $s"
        Write-Host "  Tujuan : $t"
        $job = Baca-Job
        if (Job-Berjalan $job) {
            $st = Baca-Statistik $job.Log
            $info = 'sedang menyalin'
            if ($st) { $info += " - $($st.Data)" }
            Write-Host "  Status : $info" -ForegroundColor Yellow
        }
        Write-Host ''
        Write-Host '  1. Jalankan semua langkah (dipandu)   <- paling mudah' -ForegroundColor Green
        Write-Host ''
        Write-Host '  2. Pasang rclone'
        Write-Host '  3. Hubungkan akun Google Drive'
        Write-Host '  4. Pilih folder sumber & tujuan'
        Write-Host '  5. Periksa ukuran & ruang kosong'
        Write-Host '  6. Uji coba (tidak menyalin apa pun)'
        Write-Host '  7. Mulai menyalin'
        Write-Host '  8. Lihat status'
        Write-Host '  9. Verifikasi hasil'
        Write-Host '  0. Keluar'
        Write-Host ''
        $p = (Read-Host '  Pilih').Trim()
        $dikenal = $true
        switch ($p) {
            '1' { [void](Langkah-Dipandu) }
            '2' { [void](Perintah-Pasang) }
            '3' { $script:Akun = 'google'; [void](Perintah-Hubungkan) }
            '4' { [void](Pilih-Rute) }
            '5' { [void](Perintah-Periksa) }
            '6' { [void](Perintah-Uji) }
            '7' { [void](Perintah-Salin) }
            '8' { [void](Perintah-Status) }
            '9' { [void](Perintah-Verifikasi) }
            '0' { return }
            default { $dikenal = $false }
        }
        if ($dikenal) {
            Write-Host ''
            [void](Read-Host '  Tekan Enter untuk kembali ke menu')
        }
    }
}

# ---------------------------------------------------------------- jalankan

# Saat di-dot-source oleh gui.ps1, cukup muat fungsi-fungsinya saja.
if ($MyInvocation.InvocationName -eq '.') { return }

$kode = 0
switch ($Perintah) {
    'menu'       { Tampilkan-Menu }
    'cek'        { $kode = Perintah-Cek }
    'akun'       { $kode = Perintah-Akun }
    'pasang'     { $kode = Perintah-Pasang }
    'hubungkan'  { $kode = Perintah-Hubungkan }
    'lepas'      { $kode = Perintah-Lepas }
    'bawaan'     { $kode = Perintah-Bawaan }
    'situs'      { $kode = Perintah-Situs }
    'pustaka'    { $kode = Perintah-Pustaka }
    'periksa'    { $kode = Perintah-Periksa }
    'uji'        { $kode = Perintah-Uji }
    'salin'      { $kode = Perintah-Salin }
    'status'     { $kode = Perintah-Status }
    'berhenti'   { $kode = Perintah-Berhenti }
    'verifikasi' { $kode = Perintah-Verifikasi }
}
if ($Mesin -and $HasilMesin.Count -gt 0) {
    Write-Host ('##HASIL ' + (([pscustomobject]$HasilMesin) | ConvertTo-Json -Compress -Depth 4))
}
if ("$kode" -match '^\d+$') { exit ([int]"$kode") }
exit 1
