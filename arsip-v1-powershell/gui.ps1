<#
.SYNOPSIS
    Pindah Drive: aplikasi berjendela (WPF).

.DESCRIPTION
    Dibuka lewat "Pindah Drive.exe". Salin file antar akun Google Drive, OneDrive, dan
    folder di komputer ini, ke arah mana pun.

    Pekerjaan berat (pasang, login, periksa, uji, salin, verifikasi) dijalankan oleh
    pindah.ps1 sebagai proses terpisah, sehingga jendela tidak pernah membeku.
    Penyalinan berjalan di latar belakang dan tetap jalan walau aplikasi ditutup;
    saat dibuka lagi, progresnya muncul kembali.

    pindah.ps1 mencetak baris "##HASIL {json}" (opsi -Mesin) berisi angka-angka
    hasil; baris lain masuk ke panel "Log aktivitas".
#>
param(
    # Pengujian/dokumentasi: render jendela ke berkas PNG ini lalu keluar.
    [string]$Pratinjau
)

$ErrorActionPreference = 'Continue'
Add-Type -AssemblyName PresentationFramework, PresentationCore, WindowsBase, System.Windows.Forms

# Hanya satu jendela pada satu waktu.
$kunciBaru = $false
$Kunci = [System.Threading.Mutex]::new($true, 'Local\PindahDriveGui', [ref]$kunciBaru)
if (-not $kunciBaru -and -not $Pratinjau) {
    [void][System.Windows.MessageBox]::Show('Pindah Drive sudah terbuka.', 'Pindah Drive', 'OK', 'Information')
    exit 0
}

# Ikon taskbar sendiri (bukan ikon PowerShell).
try {
    Add-Type -Namespace PindahDrive -Name Shell -MemberDefinition '[DllImport("shell32.dll")] public static extern int SetCurrentProcessExplicitAppUserModelID([MarshalAs(UnmanagedType.LPWStr)] string id);'
    [void][PindahDrive.Shell]::SetCurrentProcessExplicitAppUserModelID('PindahDrive.Aplikasi')
} catch { }

$Akar          = $PSScriptRoot
$SkripMesin    = Join-Path $Akar 'pindah.ps1'
$PowerShellExe = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
$NsX           = 'http://schemas.microsoft.com/winfx/2006/xaml'

$TitikTengah = [string][char]0x00B7
$Elipsis     = [string][char]0x2026
$Centang     = [string][char]0x2713
$BudayaId    = [System.Globalization.CultureInfo]::GetCultureInfo('id-ID')

$script:Aksi            = $null    # proses pindah.ps1 yang sedang berjalan
$script:Rclone          = $null
$script:VersiRclone     = $null
$script:DaftarAkun      = @()      # hasil Daftar-Akun (pindah.ps1)
$script:JobJalan        = $false
$script:SudahDiperiksa  = $false
$script:RingkasUji      = $null
$script:TujuanManual    = $false
$script:IsiOtomatis     = $false   # perubahan isian oleh kode, bukan oleh pengguna
$script:RuteSiap        = $false
$script:UrlLogin        = $null
$script:JenisSumber     = @{}
$script:SalinSetelahUji = $false   # "Mulai salin" menunggu periksa + uji coba selesai
$script:ProgresKunci    = ''       # untuk mendeteksi penyalinan yang tidak bergerak
$script:ProgresSejak    = Get-Date
$script:J               = $null    # keadaan dialog jelajah folder cloud
$script:AdaClientBawaan = $false   # client-google.json tersedia (lihat Baca-ClientBawaan)
$script:Detak           = 0

# ---------------------------------------------------------------- format

function Kuas([string]$hex) { return (New-Object System.Windows.Media.BrushConverter).ConvertFromString($hex) }
$Warna = @{
    Tinta  = Kuas '#1F2328'
    Redup  = Kuas '#59636E'
    Abu    = Kuas '#8C959F'
    Hijau  = Kuas '#1A7F37'
    Kuning = Kuas '#9A6700'
    Merah  = Kuas '#CF222E'
}

function Angka($n) { return ([double]$n).ToString('N0', $BudayaId) }

function Ukuran($b) {
    $b = [double]$b
    if ($b -ge 1TB) { return ($b / 1TB).ToString('N2', $BudayaId) + ' TB' }
    if ($b -ge 1GB) { return ($b / 1GB).ToString('N2', $BudayaId) + ' GB' }
    if ($b -ge 1MB) { return ($b / 1MB).ToString('N1', $BudayaId) + ' MB' }
    return ($b / 1KB).ToString('N0', $BudayaId) + ' KB'
}

# "1.200 GiB" (format rclone) -> "1,17 GB"
function Ubah-Ukuran([string]$t) {
    if ($t -match '([\d.]+)\s*([KMGTP]?i?B)?') { return (Ukuran (Ke-Byte $matches[1] ([string]$matches[2]))) }
    return $t
}

# "1h2m3s" (format rclone) -> "1 jam 2 mnt"
function Ubah-Durasi([string]$d) {
    if (-not $d -or $d -eq '-') { return '' }
    $h = @()
    if ($d -match '(\d+)d') { $h += "$($matches[1]) hari" }
    if ($d -match '(\d+)h') { $h += "$($matches[1]) jam" }
    if ($d -match '(\d+)m(?!s)') { $h += "$($matches[1]) mnt" }
    if ($h.Count -lt 2 -and $d -match '([\d.]+)s') {
        $h += ('{0} dtk' -f [int][double]::Parse($matches[1], [System.Globalization.CultureInfo]::InvariantCulture))
    }
    return ($h -join ' ')
}

function Waktu-Tampil([string]$iso) {
    try { return ([datetime]::Parse($iso)).ToString("'pukul' HH.mm, d MMM yyyy", $BudayaId) } catch { return $iso }
}

# ---------------------------------------------------------------- tampilan dasar

function Tulis-Log([string]$t) {
    $ui.Log.AppendText(('{0}  {1}' -f (Get-Date -Format 'HH:mm:ss'), $t) + "`r`n")
    $ui.Log.ScrollToEnd()
}

function Set-Pesan([string]$t, [string]$w = 'Redup') {
    $ui.TeksStatus.Text = $t
    $ui.TeksStatus.Foreground = $Warna[$w]
}

function Set-Teks($el, [string]$t, [string]$w = 'Tinta') {
    $el.Text = $t
    $el.Foreground = $Warna[$w]
}

# Layar pendek: gulir ke bagian yang baru saja berubah.
function Tunjukkan($el) {
    $Jendela.UpdateLayout()
    $el.BringIntoView()
}

function Tanya([string]$isi, [string]$judul, [string]$ikon = 'Question', [string]$tombol = 'YesNo') {
    $j = [System.Windows.MessageBox]::Show($Jendela, $isi, $judul, $tombol, $ikon)
    return ("$j" -eq 'Yes' -or "$j" -eq 'OK')
}

# Semua handler dibungkus: konsol tersembunyi, jadi error harus tampil di jendela.
function Aman([scriptblock]$sb) {
    try { & $sb }
    catch {
        Tulis-Log ('Kesalahan: ' + $_.Exception.Message)
        Set-Pesan ('Terjadi kesalahan: ' + $_.Exception.Message) 'Merah'
    }
}

function Simpan-Png($visual, [double]$lebar, [double]$tinggi, [string]$png) {
    $rtb = New-Object System.Windows.Media.Imaging.RenderTargetBitmap ([int]$lebar), ([int]$tinggi), 96, 96, ([System.Windows.Media.PixelFormats]::Pbgra32)
    $rtb.Render($visual)
    $enc = New-Object System.Windows.Media.Imaging.PngBitmapEncoder
    $enc.Frames.Add([System.Windows.Media.Imaging.BitmapFrame]::Create($rtb))
    $fs = [System.IO.File]::Create($png)
    try { $enc.Save($fs) } finally { $fs.Close() }
}

# ---------------------------------------------------------------- dialog

# Memuat XAML dialog, memakai gaya jendela utama ({DynamicResource ...}), dan
# membuat semua Hyperlink membuka browser.
function Muat-Dialog([string]$teks) {
    [xml]$x = $teks
    $w = [System.Windows.Markup.XamlReader]::Load((New-Object System.Xml.XmlNodeReader $x))
    $w.Resources = $Jendela.Resources
    if ($Jendela.IsVisible) { $w.Owner = $Jendela }
    if ($Jendela.Icon) { $w.Icon = $Jendela.Icon }
    $u = @{}
    foreach ($n in $x.SelectNodes('//*')) {
        $nama = $n.GetAttribute('Name', $NsX)
        if ($nama) { $u[$nama] = $w.FindName($nama) }
    }
    foreach ($el in @($u.Values)) {
        if ($el -is [System.Windows.Documents.Hyperlink]) {
            $el.Add_RequestNavigate({ param($s, $e) Start-Process $e.Uri.AbsoluteUri; $e.Handled = $true })
        }
    }
    return @{ W = $w; UI = $u }
}

function Tutup-Ok($pengirim) { [System.Windows.Window]::GetWindow($pengirim).DialogResult = $true }

$XamlAkunBaru = @'
<Window xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
        xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
        Width="480" SizeToContent="Height" ResizeMode="NoResize" WindowStartupLocation="CenterOwner"
        ShowInTaskbar="False" Background="#FFFFFF" FontFamily="Segoe UI Variable Text, Segoe UI" FontSize="13"
        Foreground="#1F2328" UseLayoutRounding="True" TextOptions.TextFormattingMode="Display">
  <StackPanel Margin="24,20,24,20" Background="#FFFFFF">
    <TextBlock x:Name="Judul" FontSize="16" FontWeight="SemiBold"/>
    <TextBlock x:Name="Keterangan" Margin="0,6,0,0" Foreground="#59636E" TextWrapping="Wrap"/>
    <TextBlock Margin="0,16,0,6" Foreground="#59636E" Text="Nama tampilan, misalnya Pribadi atau Kantor"/>
    <TextBox x:Name="IsiLabel" Height="34" Padding="6,0" VerticalContentAlignment="Center" BorderBrush="#D1D9E0"/>
    <StackPanel x:Name="PanelGoogle">
      <TextBlock Margin="0,16,0,6" Foreground="#59636E" Text="Akun ini akan dipakai sebagai"/>
      <RadioButton x:Name="HanyaTujuan" IsChecked="True" Margin="0,3">
        <TextBlock TextWrapping="Wrap" Text="Tujuan saja (izin terbatas: hanya melihat file yang disalin Pindah Drive)"/>
      </RadioButton>
      <RadioButton x:Name="JugaSumber" Margin="0,3">
        <TextBlock TextWrapping="Wrap" Text="Sumber dan tujuan (izin baca-tulis penuh ke Google Drive ini)"/>
      </RadioButton>
      <CheckBox x:Name="PakaiClient" IsChecked="True" Margin="0,14,0,0">
        <TextBlock TextWrapping="Wrap" Text="Pakai client ID Google sendiri (disarankan: client ID bersama rclone sering dibatasi Google sehingga penyalinan macet)"/>
      </CheckBox>
    </StackPanel>
    <StackPanel Orientation="Horizontal" HorizontalAlignment="Right" Margin="0,22,0,0">
      <Button x:Name="Batal" Style="{DynamicResource Tombol}" IsCancel="True" Content="Batal"/>
      <Button x:Name="Lanjut" Style="{DynamicResource TombolUtama}" Height="32" Margin="8,0,0,0" IsDefault="True" Content="Lanjut"/>
    </StackPanel>
  </StackPanel>
</Window>
'@

$XamlClientId = @'
<Window xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
        xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
        Title="Client ID Google sendiri" Width="600" SizeToContent="Height" ResizeMode="NoResize"
        WindowStartupLocation="CenterOwner" ShowInTaskbar="False" Background="#FFFFFF"
        FontFamily="Segoe UI Variable Text, Segoe UI" FontSize="13" Foreground="#1F2328"
        UseLayoutRounding="True" TextOptions.TextFormattingMode="Display">
  <StackPanel Margin="24,20,24,20" Background="#FFFFFF">
    <TextBlock FontSize="16" FontWeight="SemiBold" Text="Client ID Google sendiri"/>
    <TextBlock Margin="0,6,0,0" Foreground="#59636E" TextWrapping="Wrap"
               Text="Client ID bersama milik rclone dipakai semua pengguna rclone di dunia dan sering kehabisan kuota, sehingga penyalinan macet. Client ID sendiri gratis dan kuotanya khusus untuk Anda. Cukup dibuat sekali, sekitar 5 menit:"/>
    <TextBlock Margin="0,12,0,0" TextWrapping="Wrap" LineHeight="22">
      <Run Text="1.  "/><Hyperlink x:Name="Tautan1" NavigateUri="https://console.cloud.google.com/projectcreate">Buat project baru</Hyperlink><Run Text=" di Google Cloud (nama bebas, misalnya Pindah Drive)."/><LineBreak/>
      <Run Text="2.  "/><Hyperlink x:Name="Tautan2" NavigateUri="https://console.cloud.google.com/apis/library/drive.googleapis.com">Aktifkan Google Drive API</Hyperlink><Run Text=" untuk project itu."/><LineBreak/>
      <Run Text="3.  Isi "/><Hyperlink x:Name="Tautan3" NavigateUri="https://console.cloud.google.com/auth/branding">Branding</Hyperlink><Run Text=": nama aplikasi dan email Anda; pilih Audience &quot;External&quot;."/><LineBreak/>
      <Run Text="4.  Di "/><Hyperlink x:Name="Tautan4" NavigateUri="https://console.cloud.google.com/auth/audience">Audience</Hyperlink><Run Text=", tambahkan email setiap akun Google pemakai di &quot;Test users&quot; (login berlaku 7 hari, lalu cukup Login ulang)."/><LineBreak/>
      <Run Text="5.  Di "/><Hyperlink x:Name="Tautan5" NavigateUri="https://console.cloud.google.com/auth/clients">Clients</Hyperlink><Run Text=", buat client jenis &quot;Desktop app&quot;, lalu salin Client ID dan Client secret ke bawah."/>
    </TextBlock>
    <TextBlock Margin="0,16,0,6" Foreground="#59636E" Text="Client ID"/>
    <TextBox x:Name="IsiId" Height="34" Padding="6,0" VerticalContentAlignment="Center" BorderBrush="#D1D9E0"/>
    <TextBlock Margin="0,12,0,6" Foreground="#59636E" Text="Client secret"/>
    <TextBox x:Name="IsiSecret" Height="34" Padding="6,0" VerticalContentAlignment="Center" BorderBrush="#D1D9E0"/>
    <TextBlock Margin="0,12,0,0" FontSize="12" Foreground="#59636E" TextWrapping="Wrap"
               Text="Saat login, Google bisa menampilkan &quot;Google hasn't verified this app&quot;. Karena aplikasinya milik Anda sendiri, klik Advanced (Lanjutan) lalu &quot;Go to ... (unsafe)&quot;."/>
    <CheckBox x:Name="JadikanBawaan" IsChecked="True" Margin="0,14,0,0">
      <TextBlock TextWrapping="Wrap" Text="Jadikan client ID bawaan aplikasi: dipakai otomatis untuk akun Google lain dan ikut dibawa installer, sehingga pengguna lain tidak perlu menyiapkannya."/>
    </CheckBox>
    <TextBlock x:Name="Galat" Margin="0,10,0,0" Foreground="#CF222E" TextWrapping="Wrap" Visibility="Collapsed"/>
    <StackPanel Orientation="Horizontal" HorizontalAlignment="Right" Margin="0,20,0,0">
      <Button x:Name="Batal" Style="{DynamicResource Tombol}" IsCancel="True" Content="Batal"/>
      <Button x:Name="Simpan" Style="{DynamicResource TombolUtama}" Height="32" Margin="8,0,0,0" IsDefault="True" Content="Simpan &amp; login"/>
    </StackPanel>
  </StackPanel>
</Window>
'@

$XamlJelajah = @'
<Window xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
        xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
        Title="Pilih folder" Width="540" Height="540" MinWidth="420" MinHeight="380"
        WindowStartupLocation="CenterOwner" ShowInTaskbar="False" Background="#FFFFFF"
        FontFamily="Segoe UI Variable Text, Segoe UI" FontSize="13" Foreground="#1F2328"
        UseLayoutRounding="True" TextOptions.TextFormattingMode="Display">
  <DockPanel Margin="20,16,20,16" Background="#FFFFFF">
    <TextBlock x:Name="Judul" DockPanel.Dock="Top" Foreground="#59636E" TextTrimming="CharacterEllipsis"/>
    <TextBlock x:Name="Jalur" DockPanel.Dock="Top" FontSize="15" FontWeight="SemiBold" Margin="0,2,0,10" TextTrimming="CharacterEllipsis"/>
    <DockPanel DockPanel.Dock="Bottom" Margin="0,12,0,0">
      <StackPanel DockPanel.Dock="Right" Orientation="Horizontal">
        <Button x:Name="Batal" Style="{DynamicResource Tombol}" IsCancel="True" Content="Batal"/>
        <Button x:Name="Pilih" Style="{DynamicResource TombolUtama}" Height="32" Margin="8,0,0,0" Content="Pilih folder ini"/>
      </StackPanel>
      <Button x:Name="Naik" Style="{DynamicResource Tombol}" HorizontalAlignment="Left" Content="Naik satu tingkat"/>
    </DockPanel>
    <TextBlock x:Name="Status" DockPanel.Dock="Bottom" Margin="0,8,0,0" FontSize="12" Foreground="#59636E" TextWrapping="Wrap"/>
    <ListBox x:Name="Daftar" BorderBrush="#D1D9E0" Padding="2"/>
  </DockPanel>
</Window>
'@

$XamlSharePoint = @'
<Window xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
        xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
        Title="Pilih situs SharePoint" Width="640" Height="600" MinWidth="480" MinHeight="460"
        WindowStartupLocation="CenterOwner" ShowInTaskbar="False" Background="#FFFFFF"
        FontFamily="Segoe UI Variable Text, Segoe UI" FontSize="13" Foreground="#1F2328"
        UseLayoutRounding="True" TextOptions.TextFormattingMode="Display">
  <DockPanel Margin="20,16,20,16" Background="#FFFFFF">
    <TextBlock DockPanel.Dock="Top" FontSize="16" FontWeight="SemiBold" Text="Pilih situs dan pustaka dokumen"/>
    <TextBlock DockPanel.Dock="Top" Margin="0,6,0,10" Foreground="#59636E" TextWrapping="Wrap"
               Text="Ketik nama situs, atau tempel alamatnya dari browser (misalnya https://namatenant.sharepoint.com/sites/NamaSitus), lalu klik Cari."/>
    <Grid DockPanel.Dock="Top">
      <Grid.ColumnDefinitions>
        <ColumnDefinition Width="*"/>
        <ColumnDefinition Width="Auto"/>
      </Grid.ColumnDefinitions>
      <TextBox x:Name="IsiCari" Height="34" Padding="6,0" VerticalContentAlignment="Center" BorderBrush="#D1D9E0"/>
      <Button x:Name="TombolCari" Grid.Column="1" Margin="8,0,0,0" Height="34" Style="{DynamicResource Tombol}" Content="Cari"/>
    </Grid>
    <DockPanel DockPanel.Dock="Bottom" Margin="0,12,0,0">
      <StackPanel DockPanel.Dock="Right" Orientation="Horizontal">
        <Button x:Name="Batal" Style="{DynamicResource Tombol}" IsCancel="True" Content="Batal"/>
        <Button x:Name="Pilih" Style="{DynamicResource TombolUtama}" Height="32" Margin="8,0,0,0" IsEnabled="False" Content="Pakai pustaka ini"/>
      </StackPanel>
      <TextBlock x:Name="Status" VerticalAlignment="Center" Margin="0,0,12,0" FontSize="12" Foreground="#59636E" TextWrapping="Wrap"/>
    </DockPanel>
    <Grid Margin="0,12,0,0">
      <Grid.RowDefinitions>
        <RowDefinition Height="Auto"/>
        <RowDefinition Height="*"/>
        <RowDefinition Height="Auto"/>
        <RowDefinition Height="Auto"/>
      </Grid.RowDefinitions>
      <TextBlock Foreground="#59636E" Margin="0,0,0,4" Text="Situs"/>
      <ListBox x:Name="DaftarSitus" Grid.Row="1" BorderBrush="#D1D9E0"/>
      <TextBlock Grid.Row="2" Foreground="#59636E" Margin="0,12,0,4" Text="Pustaka dokumen di situs terpilih"/>
      <ListBox x:Name="DaftarPustaka" Grid.Row="3" Height="130" BorderBrush="#D1D9E0"/>
    </Grid>
  </DockPanel>
</Window>
'@

function Buat-DialogAkun([string]$jenis) {
    $d = Muat-Dialog $XamlAkunBaru
    if ($jenis -eq 'sharepoint') {
        $d.W.Title = 'Tambah SharePoint'
        $d.UI.Judul.Text = 'Tambah situs SharePoint'
        $d.UI.Keterangan.Text = 'Setelah ini browser terbuka untuk login dengan akun Microsoft kantor/sekolah yang punya akses ke situs SharePoint. Sesudah login, Anda memilih situs dan pustaka dokumennya. Nama tampilan boleh dikosongkan; akan diisi otomatis dari nama situs.'
        $d.UI.PanelGoogle.Visibility = 'Collapsed'
        $d.UI.Lanjut.Content = 'Lanjut ke login'
    }
    elseif ($jenis -eq 'google') {
        $d.W.Title = 'Tambah Google Drive'
        $d.UI.Judul.Text = 'Tambah akun Google Drive'
        $d.UI.Keterangan.Text = 'Setelah ini browser terbuka untuk login ke akun Google tersebut.'
        if (Baca-ClientBawaan) {
            # Sudah ada client ID bawaan: langsung dipakai, tidak perlu ditanyakan.
            $d.UI.PakaiClient.IsChecked = $false
            $d.UI.PakaiClient.Visibility = 'Collapsed'
            $d.UI.Keterangan.Text += ' Memakai client ID bawaan aplikasi.'
        }
    }
    else {
        $d.W.Title = 'Tambah OneDrive'
        $d.UI.Judul.Text = 'Tambah akun OneDrive'
        $d.UI.Keterangan.Text = 'Setelah ini browser terbuka untuk login ke akun Microsoft (pribadi atau kantor/sekolah). Sebagian akun kantor ditolak dengan pesan "Need admin approval"; bila begitu, pakai folder OneDrive di komputer.'
        $d.UI.PanelGoogle.Visibility = 'Collapsed'
        $d.UI.Lanjut.Content = 'Lanjut ke login'
    }
    $d.UI.Lanjut.Add_Click({ Tutup-Ok $this })
    return $d
}

# Hasil: @{ Label; IzinPenuh; PakaiClient } atau $null bila dibatalkan.
function Dialog-AkunBaru([string]$jenis) {
    $d = Buat-DialogAkun $jenis
    [void]$d.UI.IsiLabel.Focus()
    if (-not $d.W.ShowDialog()) { return $null }
    return @{
        Label       = $d.UI.IsiLabel.Text.Trim()
        IzinPenuh   = ($jenis -eq 'google' -and [bool]$d.UI.JugaSumber.IsChecked)
        PakaiClient = ($jenis -eq 'google' -and [bool]$d.UI.PakaiClient.IsChecked)
    }
}

function Buat-DialogClientId {
    $d = Muat-Dialog $XamlClientId
    $d.UI.Simpan.Add_Click({
        $w = [System.Windows.Window]::GetWindow($this)
        $id = $w.FindName('IsiId').Text.Trim()
        $rahasia = $w.FindName('IsiSecret').Text.Trim()
        $galat = $w.FindName('Galat')
        if ($id -notmatch '\.apps\.googleusercontent\.com$') {
            $galat.Text = 'Client ID biasanya berakhiran ".apps.googleusercontent.com". Salin ulang dari halaman Clients.'
            $galat.Visibility = 'Visible'
            return
        }
        if (-not $rahasia) { $galat.Text = 'Client secret belum diisi.'; $galat.Visibility = 'Visible'; return }
        $w.DialogResult = $true
    })
    return $d
}

# Hasil: @{ Id; Secret } atau $null.
function Dialog-ClientId {
    $d = Buat-DialogClientId
    if (-not $d.W.ShowDialog()) { return $null }
    return @{ Id = $d.UI.IsiId.Text.Trim(); Secret = $d.UI.IsiSecret.Text.Trim(); Bawaan = [bool]$d.UI.JadikanBawaan.IsChecked }
}

# ---- jelajah folder di akun cloud (daftar diambil rclone lsf, tanpa membekukan jendela)

function Jelajah-Muat {
    $J = $script:J
    if ($J.Proses -and -not $J.Proses.HasExited) { try { $J.Proses.Kill() } catch { } }
    $tampil = '/'
    if ($J.Jalur) { $tampil = '/' + $J.Jalur }
    $J.D.UI.Jalur.Text = $tampil
    $J.D.UI.Daftar.Items.Clear()
    $J.D.UI.Status.Text = 'Memuat daftar folder' + $Elipsis
    $J.D.UI.Naik.IsEnabled = [bool]$J.Jalur
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = Cari-Rclone
    $psi.Arguments = Gabung-Argumen @('lsf', "$($J.Remote):$($J.Jalur)", '--dirs-only', '--max-depth', '1')
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.StandardOutputEncoding = [System.Text.Encoding]::UTF8
    $psi.StandardErrorEncoding = [System.Text.Encoding]::UTF8
    $J.Proses = [System.Diagnostics.Process]::Start($psi)
    $J.Tugas = $J.Proses.StandardOutput.ReadToEndAsync()
    $J.TugasErr = $J.Proses.StandardError.ReadToEndAsync()
}

function Jelajah-Tick {
    $J = $script:J
    if (-not $J -or -not $J.Tugas -or -not $J.Tugas.IsCompleted) { return }
    $out = $J.Tugas.Result
    $J.Tugas = $null
    $J.Proses.WaitForExit()
    if ($J.Proses.ExitCode -ne 0) {
        $err = ($J.TugasErr.Result -split "`r?`n" | Where-Object { $_ -and $_ -notmatch 'client_id' } | Select-Object -Last 1)
        $J.D.UI.Status.Text = "Gagal memuat: $err"
        return
    }
    $nama = @($out -split "`r?`n" | ForEach-Object { $_.TrimEnd('/') } | Where-Object { $_ } | Sort-Object)
    foreach ($n in $nama) { [void]$J.D.UI.Daftar.Items.Add($n) }
    if ($nama.Count -eq 0) { $J.D.UI.Status.Text = 'Tidak ada subfolder. Klik "Pilih folder ini" untuk memakai folder ini.' }
    else { $J.D.UI.Status.Text = ('{0} folder. Klik dua kali untuk membuka.' -f $nama.Count) }
}

function Jelajah-Masuk([string]$nama) {
    if (-not $nama) { return }
    if ($script:J.Jalur) { $script:J.Jalur = $script:J.Jalur + '/' + $nama } else { $script:J.Jalur = $nama }
    Jelajah-Muat
}

function Jelajah-Naik {
    $p = $script:J.Jalur
    if (-not $p) { return }
    $i = $p.LastIndexOf('/')
    if ($i -lt 0) { $script:J.Jalur = '' } else { $script:J.Jalur = $p.Substring(0, $i) }
    Jelajah-Muat
}

function Buat-DialogJelajah([string]$remote, [string]$awal, [string]$judul) {
    $d = Muat-Dialog $XamlJelajah
    $d.UI.Judul.Text = $judul
    $script:J = @{ Remote = $remote; Jalur = "$awal".Trim('/'); Proses = $null; Tugas = $null; TugasErr = $null; D = $d }
    $d.UI.Daftar.Add_MouseDoubleClick({ Aman { Jelajah-Masuk ([string]$script:J.D.UI.Daftar.SelectedItem) } })
    $d.UI.Naik.Add_Click({ Aman { Jelajah-Naik } })
    $d.UI.Pilih.Add_Click({ Tutup-Ok $this })
    $t = New-Object System.Windows.Threading.DispatcherTimer
    $t.Interval = [TimeSpan]::FromMilliseconds(150)
    $t.Add_Tick({ Aman { Jelajah-Tick } })
    $script:J.Timer = $t
    $t.Start()
    Jelajah-Muat
    return $d
}

# Hasil: jalur di dalam akun (tanpa "akun:"), atau $null bila dibatalkan.
function Jelajah-Cloud([string]$remote, [string]$awal, [string]$judul) {
    $d = Buat-DialogJelajah $remote $awal $judul
    $ok = $d.W.ShowDialog()
    $script:J.Timer.Stop()
    if ($script:J.Proses -and -not $script:J.Proses.HasExited) { try { $script:J.Proses.Kill() } catch { } }
    if ($ok) { return $script:J.Jalur }
    return $null
}

# ---- pemilih situs & pustaka SharePoint

# Menjalankan pindah.ps1 untuk dialog (tanpa memakai panel log/status jendela utama).
function Mulai-Mesin([string[]]$argumen) {
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $PowerShellExe
    $psi.Arguments = Gabung-Argumen (@('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $SkripMesin) + $argumen + @('-Mesin'))
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.StandardOutputEncoding = [System.Text.Encoding]::UTF8
    $psi.StandardErrorEncoding = [System.Text.Encoding]::UTF8
    $p = [System.Diagnostics.Process]::Start($psi)
    $p.StandardInput.Close()
    return @{ Proses = $p; Out = $p.StandardOutput.ReadToEndAsync(); Err = $p.StandardError.ReadToEndAsync() }
}

# $null bila belum selesai; selain itu @{ Kode; Hasil (dari ##HASIL); Pesan (baris terakhir) }.
function Hasil-Mesin($t) {
    if (-not $t -or -not $t.Out.IsCompleted -or -not $t.Err.IsCompleted) { return $null }
    $t.Proses.WaitForExit()
    $hasil = $null
    $pesan = ''
    foreach ($b in ($t.Out.Result -split "`r?`n")) {
        if ($b.StartsWith('##HASIL ')) { try { $hasil = $b.Substring(8) | ConvertFrom-Json } catch { } }
        elseif ($b.Trim() -and -not $b.Trim().StartsWith('==')) { $pesan = ($b -replace '^\[(OK|AWAS|GAGAL)\]\s*', '').Trim() }
    }
    return @{ Kode = $t.Proses.ExitCode; Hasil = $hasil; Pesan = $pesan }
}

function Hentikan-Mesin($t) {
    if ($t -and $t.Proses -and -not $t.Proses.HasExited) { try { $t.Proses.Kill() } catch { } }
}

function Item-Daftar([string]$teks, $data) {
    $i = New-Object System.Windows.Controls.ListBoxItem
    $i.Content = $teks
    $i.Tag = $data
    return $i
}

function SP-Cari {
    $S = $script:SP
    $teks = $S.D.UI.IsiCari.Text.Trim()
    if (-not $teks) { $S.D.UI.Status.Text = 'Ketik nama situs atau tempel alamatnya dulu.'; return }
    $S.UrlTempel = $null
    $kata = $teks
    # Alamat yang ditempel: cari dengan nama situsnya, lalu pilih yang alamatnya sama persis.
    if ($teks -match '^https?://') {
        $S.UrlTempel = ($teks -replace '[?#].*$', '').TrimEnd('/')
        if ($S.UrlTempel -match '/(sites|teams)/([^/]+)') { $kata = [uri]::UnescapeDataString($matches[2]) }
        else { $kata = '*' }
    }
    Hentikan-Mesin $S.Tugas
    $S.D.UI.DaftarSitus.Items.Clear()
    $S.D.UI.DaftarPustaka.Items.Clear()
    $S.D.UI.Pilih.IsEnabled = $false
    $S.D.UI.Status.Text = ('Mencari situs "{0}"' -f $kata) + $Elipsis
    $S.Mode = 'situs'
    $S.Tugas = Mulai-Mesin @('situs', '-Akun', $S.Remote, '-Cari', $kata)
}

function SP-MuatPustaka {
    $S = $script:SP
    $item = $S.D.UI.DaftarSitus.SelectedItem
    if (-not $item) { return }
    $situs = $item.Tag
    $S.Situs = $situs
    Hentikan-Mesin $S.Tugas
    $S.D.UI.DaftarPustaka.Items.Clear()
    $S.D.UI.Pilih.IsEnabled = $false
    $S.D.UI.Status.Text = ('Memuat pustaka di "{0}"' -f $situs.Nama) + $Elipsis
    $S.Mode = 'pustaka'
    $S.Tugas = Mulai-Mesin @('pustaka', '-Akun', $S.Remote, '-SitusId', $situs.Id, '-SitusNama', $situs.Nama)
}

function SP-Tick {
    $S = $script:SP
    if (-not $S -or -not $S.Tugas) { return }
    $h = Hasil-Mesin $S.Tugas
    if (-not $h) { return }
    $S.Tugas = $null
    if ($S.Mode -eq 'situs') {
        $daftar = @()
        if ($h.Hasil -and $h.Hasil.Situs) { $daftar = @($h.Hasil.Situs | ForEach-Object { $_ }) }
        $cocok = $null
        # Catatan: variabel PowerShell tidak peka huruf besar-kecil; "$s" akan menimpa "$S".
        foreach ($satu in $daftar) {
            $i = Item-Daftar ("{0}   {1}" -f $satu.Nama, $satu.Url) $satu
            [void]$S.D.UI.DaftarSitus.Items.Add($i)
            if ($S.UrlTempel -and "$($satu.Url)".TrimEnd('/') -eq $S.UrlTempel) { $cocok = $i }
        }
        if ($h.Kode -ne 0) { $S.D.UI.Status.Text = "Pencarian gagal: $($h.Pesan)" }
        elseif ($daftar.Count -eq 0) { $S.D.UI.Status.Text = 'Tidak ada situs yang cocok, atau akun ini tidak punya akses ke situs tersebut.' }
        else { $S.D.UI.Status.Text = ('{0} situs ditemukan. Pilih salah satu.' -f $daftar.Count) }
        if ($cocok) { $S.D.UI.DaftarSitus.SelectedItem = $cocok; $cocok.BringIntoView() }
        elseif ($S.UrlTempel -and $daftar.Count -gt 0) { $S.D.UI.Status.Text = 'Alamat yang ditempel tidak ditemukan persis; pilih situsnya dari daftar.' }
    }
    else {
        $daftar = @()
        if ($h.Hasil -and $h.Hasil.Pustaka) { $daftar = @($h.Hasil.Pustaka | ForEach-Object { $_ }) }
        $utama = $null
        foreach ($p in $daftar) {
            $i = Item-Daftar $p.Nama $p
            [void]$S.D.UI.DaftarPustaka.Items.Add($i)
            if (-not $utama -and $p.Nama -match '^(Dokumen|Documents|Shared Documents|Dokumen Bersama)$') { $utama = $i }
        }
        if ($h.Kode -ne 0) { $S.D.UI.Status.Text = "Pustaka tidak bisa dibaca: $($h.Pesan)" }
        elseif ($daftar.Count -eq 0) { $S.D.UI.Status.Text = 'Situs ini tidak punya pustaka dokumen yang bisa Anda akses.' }
        else {
            if (-not $utama) { $utama = $S.D.UI.DaftarPustaka.Items[0] }
            $S.D.UI.DaftarPustaka.SelectedItem = $utama
            $S.D.UI.Status.Text = ('{0} pustaka. Pilih lalu klik "Pakai pustaka ini".' -f $daftar.Count)
        }
    }
}

function Buat-DialogSharePoint([string]$remote) {
    $d = Muat-Dialog $XamlSharePoint
    $script:SP = @{ Remote = $remote; D = $d; Tugas = $null; Mode = ''; Situs = $null; UrlTempel = $null }
    $d.UI.TombolCari.Add_Click({ Aman { SP-Cari } })
    $d.UI.IsiCari.Add_KeyDown({ param($s, $e) if ("$($e.Key)" -eq 'Return') { $e.Handled = $true; Aman { SP-Cari } } })
    $d.UI.DaftarSitus.Add_SelectionChanged({ Aman { SP-MuatPustaka } })
    $d.UI.DaftarPustaka.Add_SelectionChanged({ $script:SP.D.UI.Pilih.IsEnabled = [bool]$script:SP.D.UI.DaftarPustaka.SelectedItem })
    $d.UI.Pilih.Add_Click({ Tutup-Ok $this })
    $t = New-Object System.Windows.Threading.DispatcherTimer
    $t.Interval = [TimeSpan]::FromMilliseconds(200)
    $t.Add_Tick({ Aman { SP-Tick } })
    $script:SP.Timer = $t
    $t.Start()
    $d.UI.Status.Text = 'Contoh: ketik "keuangan" atau tempel alamat situs, lalu tekan Enter.'
    return $d
}

# Hasil: @{ SitusNama; PustakaId; PustakaNama } atau $null.
function Dialog-PilihSharePoint([string]$remote) {
    $d = Buat-DialogSharePoint $remote
    [void]$d.UI.IsiCari.Focus()
    $ok = $d.W.ShowDialog()
    $script:SP.Timer.Stop()
    Hentikan-Mesin $script:SP.Tugas
    if (-not $ok) { return $null }
    $p = $d.UI.DaftarPustaka.SelectedItem.Tag
    return @{ SitusNama = "$($script:SP.Situs.Nama)"; PustakaId = "$($p.Id)"; PustakaNama = "$($p.Nama)" }
}

function Pakai-PustakaSharePoint([string]$nama) {
    $c = Dialog-PilihSharePoint $nama
    if (-not $c) { Set-Pesan 'Situs SharePoint belum dipilih; akun ini belum bisa dipakai.' 'Kuning'; return }
    Jalankan-Aksi 'Memakai pustaka SharePoint' @('pustaka', '-Akun', $nama, '-Pustaka', $c.PustakaId, '-SitusNama', $c.SitusNama, '-PustakaNama', $c.PustakaNama) {
        param($kode, $hasil) Selesai-Akun $kode 'Pustaka SharePoint siap dipakai sebagai sumber atau tujuan.' 'Pustaka gagal dipakai. Lihat log aktivitas.'
    }
}

# ---------------------------------------------------------------- proses latar (pindah.ps1)

function Jalankan-Aksi([string]$judul, [string[]]$argumen, [scriptblock]$selesai) {
    if ($script:Aksi) { return }
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $PowerShellExe
    $psi.Arguments = Gabung-Argumen (@('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $SkripMesin) + $argumen + @('-Mesin'))
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.RedirectStandardInput = $true      # proses anak tidak pernah menunggu ketikan
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.StandardOutputEncoding = [System.Text.Encoding]::UTF8
    $psi.StandardErrorEncoding = [System.Text.Encoding]::UTF8
    $p = [System.Diagnostics.Process]::Start($psi)
    $p.StandardInput.Close()
    $script:Aksi = @{
        Judul = $judul; Proses = $p; Hasil = $null; Selesai = $selesai; Dibatalkan = $false
        Out = $p.StandardOutput.ReadLineAsync()
        Err = $p.StandardError.ReadLineAsync()
    }
    Tulis-Log "--- $judul"
    Set-Sibuk $true $judul
}

# Dipanggil timer (UI thread): ambil baris yang sudah siap tanpa memblokir jendela.
function Proses-Aksi {
    $a = $script:Aksi
    if (-not $a) { return }
    foreach ($k in @('Out', 'Err')) {
        while ($a[$k] -and $a[$k].IsCompleted) {
            if ("$($a[$k].Status)" -ne 'RanToCompletion') { $a[$k] = $null; break }
            $baris = $a[$k].Result
            if ($null -eq $baris) { $a[$k] = $null; break }
            Terima-Baris $a $baris
            if ($k -eq 'Out') { $a[$k] = $a.Proses.StandardOutput.ReadLineAsync() }
            else { $a[$k] = $a.Proses.StandardError.ReadLineAsync() }
        }
    }
    if ($a.Out -or $a.Err) { return }
    $a.Proses.WaitForExit()
    $kode = $a.Proses.ExitCode
    $script:Aksi = $null
    $script:UrlLogin = $null
    $ui.TombolBukaLogin.Visibility = 'Collapsed'
    Set-Sibuk $false
    if ($a.Dibatalkan) { $script:SalinSetelahUji = $false; Set-Pesan 'Dibatalkan.'; Segarkan-Status; return }
    if ($a.Selesai) { & $a.Selesai $kode $a.Hasil }
}

function Terima-Baris($a, [string]$baris) {
    if ($baris.StartsWith('##HASIL ')) {
        try { $a.Hasil = $baris.Substring(8) | ConvertFrom-Json } catch { }
        return
    }
    $t = $baris.Trim()
    if (-not $t -or $t.StartsWith('==')) { return }
    Tulis-Log $t
    if ($t -match '(http://127\.0\.0\.1:\d+/auth\S*)') {
        $script:UrlLogin = $matches[1]
        $ui.TombolBukaLogin.Visibility = 'Visible'
    }
}

function Klik-Batal {
    $a = $script:Aksi
    if (-not $a) { return }
    $a.Dibatalkan = $true
    $k = New-Object System.Diagnostics.ProcessStartInfo 'taskkill.exe', "/PID $($a.Proses.Id) /T /F"
    $k.UseShellExecute = $false
    $k.CreateNoWindow = $true
    [void]([System.Diagnostics.Process]::Start($k)).WaitForExit(5000)
    Tulis-Log 'Dibatalkan oleh pengguna.'
}

# ---------------------------------------------------------------- keadaan tombol

function Set-Sibuk([bool]$sibuk, [string]$judul = '') {
    foreach ($n in @('TombolPasang', 'TombolTambahGoogle', 'TombolTambahOneDrive', 'TombolTambahSharePoint', 'DaftarAkun',
                     'LokasiSumber', 'PilihSumber', 'TombolTelusuri', 'LokasiTujuan', 'IsiTujuan', 'TombolTelusuriTujuan',
                     'TombolTukar', 'TombolPeriksa', 'TombolSalin', 'TombolHenti', 'TombolVerifikasi')) {
        $ui[$n].IsEnabled = -not $sibuk
    }
    if ($sibuk) {
        $ui.TombolBatal.Visibility = 'Visible'
        Set-Pesan ($judul + $Elipsis) 'Tinta'
        $Jendela.Cursor = [System.Windows.Input.Cursors]::AppStarting
    }
    else {
        $ui.TombolBatal.Visibility = 'Collapsed'
        $Jendela.Cursor = $null
        Segarkan-Tombol
    }
}

function Segarkan-Tombol {
    if ($script:Aksi) { return }
    $ada   = [bool]$script:Rclone
    $jalan = [bool]$script:JobJalan
    $ui.TombolPasang.IsEnabled         = -not $ada
    $ui.TombolTambahGoogle.IsEnabled   = $ada
    $ui.TombolTambahOneDrive.IsEnabled = $ada
    $ui.TombolTambahSharePoint.IsEnabled = $ada
    $ui.DaftarAkun.IsEnabled           = $ada -and -not $jalan
    foreach ($n in @('LokasiSumber', 'PilihSumber', 'TombolTelusuri', 'LokasiTujuan', 'IsiTujuan', 'TombolTelusuriTujuan', 'TombolTukar', 'TombolPeriksa')) {
        $ui[$n].IsEnabled = -not $jalan
    }
    $ui.TombolSalin.IsEnabled      = $ada -and -not $jalan
    $ui.TombolVerifikasi.IsEnabled = $ada -and -not $jalan
    $ui.TombolHenti.IsEnabled      = $jalan
    if ($jalan) { $ui.TombolHenti.Visibility = 'Visible' } else { $ui.TombolHenti.Visibility = 'Collapsed' }
}

# ---------------------------------------------------------------- akun

function Akun-Dari([string]$remote) {
    if (-not $remote) { return $null }
    return ($script:DaftarAkun | Where-Object { $_.Nama -eq $remote } | Select-Object -First 1)
}

# "Google Drive", "Google Drive 2", "OneDrive <titik tengah> Kantor"
function Nama-Lokasi($a) {
    if (-not $a) { return 'Komputer ini' }
    $dasar = 'OneDrive'
    if ($a.Jenis -eq 'google') { $dasar = 'Google Drive' }
    elseif ($a.Jenis -eq 'sharepoint') { $dasar = 'SharePoint' }
    if ("$($a.Label)".StartsWith($dasar)) { return "$($a.Label)" }
    return "$dasar $TitikTengah $($a.Label)"
}

function Tambah-Menu($menu, [string]$teks, [string]$tag) {
    $mi = New-Object System.Windows.Controls.MenuItem
    $mi.Header = $teks
    $mi.Tag = $tag
    $mi.Add_Click({ $t = [string]$this.Tag; Aman { Akun-Aksi $t } })
    [void]$menu.Items.Add($mi)
}

function Buat-BarisAkun($a) {
    $g = New-Object System.Windows.Controls.Grid
    $g.Height = 38
    $k1 = New-Object System.Windows.Controls.ColumnDefinition; $k1.Width = New-Object System.Windows.GridLength 110
    $k2 = New-Object System.Windows.Controls.ColumnDefinition; $k2.Width = New-Object System.Windows.GridLength 1, ([System.Windows.GridUnitType]::Star)
    $k3 = New-Object System.Windows.Controls.ColumnDefinition; $k3.Width = [System.Windows.GridLength]::Auto
    $g.ColumnDefinitions.Add($k1); $g.ColumnDefinitions.Add($k2); $g.ColumnDefinitions.Add($k3)

    $jenis = New-Object System.Windows.Controls.TextBlock
    $jenis.Style = $Jendela.FindResource('Label')
    $jenis.VerticalAlignment = 'Center'
    $jenis.Text = 'OneDrive'
    if ($a.Jenis -eq 'google') { $jenis.Text = 'Google Drive' }
    elseif ($a.Jenis -eq 'sharepoint') { $jenis.Text = 'SharePoint' }
    [void]$g.Children.Add($jenis)

    $detail = @()
    $warnaTitik = 'Hijau'
    $warnaDetail = 'Redup'
    if ($a.Jenis -eq 'google') {
        if ($a.Scope -eq 'drive') { $detail += 'izin penuh' } else { $detail += 'izin terbatas (hanya tujuan)' }
        if ($a.ClientSendiri) { $detail += 'client ID sendiri' }
        else { $detail += 'client ID bersama (sering dibatasi)'; $warnaTitik = 'Kuning'; $warnaDetail = 'Kuning' }
    }
    elseif ($a.Jenis -eq 'sharepoint') {
        if ($a.Siap) { $detail += "pustaka $($a.Drive), situs $($a.Situs)" }
        else { $detail += 'belum memilih situs & pustaka'; $warnaTitik = 'Kuning'; $warnaDetail = 'Kuning' }
    }
    elseif ($a.Drive) { $detail += "drive $($a.Drive)" }

    $sp = New-Object System.Windows.Controls.StackPanel
    $sp.Orientation = 'Horizontal'
    $sp.VerticalAlignment = 'Center'
    [System.Windows.Controls.Grid]::SetColumn($sp, 1)
    $titik = New-Object System.Windows.Shapes.Ellipse
    $titik.Width = 8; $titik.Height = 8; $titik.Fill = $Warna[$warnaTitik]
    $titik.Margin = New-Object System.Windows.Thickness 0, 0, 8, 0
    [void]$sp.Children.Add($titik)
    $nama = New-Object System.Windows.Controls.TextBlock
    $nama.Text = "$($a.Label)"
    [void]$sp.Children.Add($nama)
    if ($detail.Count -gt 0) {
        $ket = New-Object System.Windows.Controls.TextBlock
        $ket.Text = "   $TitikTengah   " + ($detail -join ", ")
        $ket.FontSize = 12
        $ket.VerticalAlignment = 'Center'
        $ket.Foreground = $Warna[$warnaDetail]
        $ket.TextTrimming = 'CharacterEllipsis'
        [void]$sp.Children.Add($ket)
    }
    [void]$g.Children.Add($sp)

    $b = New-Object System.Windows.Controls.Button
    $b.Style = $Jendela.FindResource('Tombol')
    $b.Content = "Atur $([char]0x25BE)"
    [System.Windows.Controls.Grid]::SetColumn($b, 2)
    $menu = New-Object System.Windows.Controls.ContextMenu
    if ($a.Jenis -eq 'sharepoint') { Tambah-Menu $menu ('Pilih situs & pustaka' + $Elipsis) "$($a.Nama)|situs" }
    Tambah-Menu $menu 'Login ulang' "$($a.Nama)|ulang"
    if ($a.Jenis -eq 'google' -and $a.Scope -ne 'drive') { Tambah-Menu $menu 'Beri izin penuh (agar bisa jadi sumber)' "$($a.Nama)|penuh" }
    if ($a.Jenis -eq 'google') { Tambah-Menu $menu ('Pakai client ID sendiri' + $Elipsis) "$($a.Nama)|client" }
    if ($a.Jenis -eq 'google' -and $a.ClientSendiri) { Tambah-Menu $menu 'Jadikan client ID ini bawaan aplikasi' "$($a.Nama)|bawaan" }
    [void]$menu.Items.Add((New-Object System.Windows.Controls.Separator))
    Tambah-Menu $menu 'Lepas akun ini' "$($a.Nama)|lepas"
    $b.ContextMenu = $menu
    $b.Add_Click({
        $this.ContextMenu.PlacementTarget = $this
        $this.ContextMenu.Placement = 'Bottom'
        $this.ContextMenu.IsOpen = $true
    })
    [void]$g.Children.Add($b)
    return $g
}

function Item-Lokasi([string]$teks, [string]$tag) {
    $i = New-Object System.Windows.Controls.ComboBoxItem
    $i.Content = $teks
    $i.Tag = $tag
    return $i
}

function Tag-Terpilih($cb) {
    if ($cb.SelectedItem) { return [string]$cb.SelectedItem.Tag }
    return $null
}

function Pilih-Tag($cb, [string]$tag) {
    foreach ($i in $cb.Items) { if ([string]$i.Tag -eq $tag) { $cb.SelectedItem = $i; return $true } }
    if ($cb.Items.Count -gt 0) { $cb.SelectedIndex = 0 }
    return $false
}

# Pilihan "Dari"/"Ke": Komputer ini + semua akun; pilihan sebelumnya dipertahankan.
function Isi-Lokasi {
    foreach ($cb in @($ui.LokasiSumber, $ui.LokasiTujuan)) {
        $tag = Tag-Terpilih $cb
        $script:IsiOtomatis = $true
        try {
            $cb.Items.Clear()
            [void]$cb.Items.Add((Item-Lokasi 'Komputer ini' ''))
            # SharePoint yang belum memilih pustaka masih menunjuk OneDrive pribadi: jangan ditawarkan.
            foreach ($a in $script:DaftarAkun) {
                if ($a.PSObject.Properties['Siap'] -and -not $a.Siap) { continue }
                [void]$cb.Items.Add((Item-Lokasi (Nama-Lokasi $a) $a.Nama))
            }
            if ($null -ne $tag) { [void](Pilih-Tag $cb $tag) }
        }
        finally { $script:IsiOtomatis = $false }
    }
}

function Segarkan-Status {
    $script:Rclone = Cari-Rclone
    if ($script:Rclone) {
        if (-not $script:VersiRclone) {
            $v = ((Panggil-Rclone @('version') -Tangkap).Out -split "`r?`n")[0]
            $script:VersiRclone = ($v -replace '^rclone\s+', '').Trim()
        }
        $ui.TitikRclone.Fill = $Warna.Hijau
        $ui.StatusRclone.Text = 'Terpasang ' + $script:VersiRclone
        $ui.TombolPasang.Visibility = 'Collapsed'
    }
    else {
        $ui.TitikRclone.Fill = $Warna.Merah
        $ui.StatusRclone.Text = 'Belum terpasang'
        $ui.TombolPasang.Visibility = 'Visible'
    }

    $script:DaftarAkun = Daftar-Akun
    $ui.DaftarAkun.Children.Clear()
    foreach ($a in $script:DaftarAkun) { [void]$ui.DaftarAkun.Children.Add((Buat-BarisAkun $a)) }
    if ($script:Rclone -and $script:DaftarAkun.Count -eq 0) {
        $t = New-Object System.Windows.Controls.TextBlock
        $t.Text = 'Belum ada akun cloud. Tambahkan Google Drive atau OneDrive di bawah ini.'
        $t.Foreground = $Warna.Redup
        $t.Margin = New-Object System.Windows.Thickness 0, 4, 0, 0
        [void]$ui.DaftarAkun.Children.Add($t)
    }
    $bersama = @($script:DaftarAkun | Where-Object { $_.Jenis -eq 'google' -and -not $_.ClientSendiri })
    if ($bersama.Count -gt 0) { $ui.PanelPeringatanClient.Visibility = 'Visible' } else { $ui.PanelPeringatanClient.Visibility = 'Collapsed' }
    $script:AdaClientBawaan = [bool](Baca-ClientBawaan)
    if ($script:AdaClientBawaan) {
        $ui.TeksPeringatanClient.Text = 'Akun Google yang memakai client ID bersama rclone sering dibatasi Google, sehingga penyalinan bisa macet. Klik Atur lalu "Login ulang" untuk beralih ke client ID bawaan aplikasi.'
    }
    else {
        $ui.TeksPeringatanClient.Text = 'Akun Google yang memakai client ID bersama rclone sering dibatasi Google, sehingga penyalinan bisa macet. Pakai client ID sendiri lewat tombol Atur.'
    }

    Isi-Lokasi
    if ($script:RuteSiap) { Sumber-Berubah; Tujuan-Berubah }
    Segarkan-Tombol
}

function Selesai-Akun($kode, [string]$ok, [string]$gagal) {
    Segarkan-Status
    if ($kode -eq 0) { Set-Pesan $ok 'Hijau' } else { Set-Pesan $gagal 'Merah'; $ui.PanelLog.IsExpanded = $true }
}

function Klik-Pasang {
    $isi = "Pindah Drive memakai rclone, program open source (lisensi MIT) untuk menyalin file antar-layanan cloud.`n`n" +
           "rclone akan diunduh (sekitar 25 MB) dan dipasang lewat winget dari repositori resmi. " +
           "Dengan melanjutkan, Anda menyetujui ketentuan sumber dan paket winget.`n`nPasang sekarang?"
    if (-not (Tanya $isi 'Pasang rclone')) { return }
    Jalankan-Aksi 'Memasang rclone' @('pasang', '-Ya') {
        param($kode, $hasil)
        $script:VersiRclone = $null
        Selesai-Akun $kode 'rclone terpasang. Lanjutkan dengan menambah akun.' 'Pemasangan rclone gagal. Lihat log aktivitas.'
    }
}

function Klik-Tambah([string]$jenis) {
    $p = Dialog-AkunBaru $jenis
    if (-not $p) { return }
    $c = $null
    if ($p.PakaiClient) {
        $c = Dialog-ClientId
        if (-not $c) { return }
    }
    $nama = Remote-Baru $jenis
    $arg = @('hubungkan', '-Akun', $nama, '-Jenis', $jenis, '-Ya')
    if ($p.Label) { $arg += @('-Label', $p.Label) }
    if ($p.IzinPenuh) { $arg += '-IzinPenuh' }
    if ($c) {
        $arg += @('-ClientId', $c.Id, '-ClientSecret', $c.Secret)
        if ($c.Bawaan) { $arg += '-SimpanBawaan' }
    }
    $layanan = 'Microsoft'
    if ($jenis -eq 'google') { $layanan = 'Google' }
    if ($jenis -eq 'sharepoint') {
        # Setelah login: langsung pilih situs & pustaka.
        Jalankan-Aksi "Menunggu login $layanan di browser" $arg {
            param($kode, $hasil)
            Selesai-Akun $kode 'Login berhasil. Sekarang pilih situs SharePoint-nya.' 'Login tidak selesai. Coba lagi.'
            if ($kode -eq 0 -and $hasil -and $hasil.Akun) { Pakai-PustakaSharePoint "$($hasil.Akun)" }
        }
        return
    }
    Jalankan-Aksi "Menunggu login $layanan di browser" $arg {
        param($kode, $hasil)
        Selesai-Akun $kode 'Akun terhubung.' 'Login tidak selesai. Coba lagi.'
    }
}

function Akun-Aksi([string]$tag) {
    $bagian = $tag -split '\|', 2
    $nama = $bagian[0]
    $aksi = $bagian[1]
    $a = Akun-Dari $nama
    $label = Nama-Lokasi $a
    if ($aksi -eq 'ulang') {
        if (-not (Tanya "Login ulang ke $label?`n`nBrowser akan terbuka untuk login." 'Login ulang' 'Information' 'OKCancel')) { return }
        Jalankan-Aksi 'Menunggu login di browser' @('hubungkan', '-Akun', $nama, '-Ulang', '-Ya') {
            param($kode, $hasil) Selesai-Akun $kode 'Login ulang berhasil.' 'Login ulang tidak selesai.'
        }
    }
    elseif ($aksi -eq 'penuh') {
        $isi = "Beri $label izin baca-tulis penuh?`n`nIzin ini dibutuhkan agar file di akun ini bisa disalin ke tempat lain. " +
               "Browser akan terbuka untuk login dan menyetujui izin baru."
        if (-not (Tanya $isi 'Izin penuh' 'Question' 'OKCancel')) { return }
        Jalankan-Aksi 'Menunggu login Google di browser' @('hubungkan', '-Akun', $nama, '-IzinPenuh', '-Ya') {
            param($kode, $hasil) Selesai-Akun $kode 'Izin penuh diberikan.' 'Login tidak selesai.'
        }
    }
    elseif ($aksi -eq 'client') {
        $c = Dialog-ClientId
        if (-not $c) { return }
        $arg = @('hubungkan', '-Akun', $nama, '-ClientId', $c.Id, '-ClientSecret', $c.Secret, '-Ya')
        if ($c.Bawaan) { $arg += '-SimpanBawaan' }
        Jalankan-Aksi 'Menunggu login Google di browser' $arg {
            param($kode, $hasil) Selesai-Akun $kode 'Client ID sendiri terpasang.' 'Login tidak selesai. Periksa Client ID dan secret.'
        }
    }
    elseif ($aksi -eq 'situs') {
        Pakai-PustakaSharePoint $nama
    }
    elseif ($aksi -eq 'bawaan') {
        $isi = "Jadikan client ID milik $label sebagai bawaan aplikasi?`n`n" +
               "Client ID ini akan dipakai otomatis untuk akun Google yang ditambahkan berikutnya, dan ikut dibawa installer " +
               "yang dibuat sesudahnya. Siapa pun yang memegang installer itu bisa membaca client ID dan secret-nya."
        if (-not (Tanya $isi 'Client ID bawaan' 'Question' 'OKCancel')) { return }
        Jalankan-Aksi 'Menyimpan client ID bawaan' @('bawaan', '-Akun', $nama) {
            param($kode, $hasil) Selesai-Akun $kode 'Client ID bawaan tersimpan.' 'Client ID bawaan gagal disimpan. Lihat log aktivitas.'
        }
    }
    elseif ($aksi -eq 'lepas') {
        $isi = "Lepas $label dari Pindah Drive?`n`nFile di akun tersebut TIDAK terhapus; hanya sambungan di komputer ini yang dihapus."
        if (-not (Tanya $isi 'Lepas akun' 'Warning')) { return }
        Jalankan-Aksi 'Melepas akun' @('lepas', '-Akun', $nama, '-Ya') {
            param($kode, $hasil) Selesai-Akun $kode 'Akun dilepas.' 'Akun gagal dilepas. Lihat log aktivitas.'
        }
    }
}

# ---------------------------------------------------------------- sumber & tujuan

# "pd-onedrive:Kerja/DLH" -> @{ Remote = 'pd-onedrive'; Path = 'Kerja/DLH' }
function Pecah([string]$jalur) {
    $rem = Remote-Dari $jalur
    if ($rem) { return @{ Remote = $rem; Path = $jalur.Substring($rem.Length + 1).Trim('/') } }
    return @{ Remote = ''; Path = $jalur }
}

function Gabung([string]$remote, [string]$path) {
    $p = "$path".Trim().Trim('"')
    if (-not $remote) { return (Rapikan-Sumber $p) }
    return "${remote}:" + (($p -replace '\\', '/').Trim('/'))
}

function Jalur-Sumber { return (Gabung (Tag-Terpilih $ui.LokasiSumber) ([string]$ui.PilihSumber.Text)) }
function Jalur-Tujuan { return (Gabung (Tag-Terpilih $ui.LokasiTujuan) ([string]$ui.IsiTujuan.Text)) }

# Nama yang mudah dibaca untuk dialog dan hasil verifikasi.
function Nama-Jalur([string]$j) {
    $rem = Remote-Dari $j
    if (-not $rem) {
        $k = $script:JenisSumber[$j]
        if ($k) { return "$j ($k)" }
        return $j
    }
    $n = Nama-Lokasi (Akun-Dari $rem)
    $p = $j.Substring($rem.Length + 1).Trim('/')
    if (-not $p) { return "$n (seluruh isi)" }
    return "$n / $p"
}

function Lokasi-TujuanBawaan {
    $g = $script:DaftarAkun | Where-Object { $_.Jenis -eq 'google' } | Select-Object -First 1
    if ($g) { return $g.Nama }
    $a = $script:DaftarAkun | Select-Object -First 1
    if ($a) { return $a.Nama }
    return ''
}

# Saran folder sumber: folder OneDrive yang tersinkron di komputer ini.
function Isi-SaranSumber {
    $ui.PilihSumber.Items.Clear()
    $script:JenisSumber = @{}
    if (Tag-Terpilih $ui.LokasiSumber) { return }
    foreach ($o in (Daftar-OneDrive)) {
        [void]$ui.PilihSumber.Items.Add($o.Folder)
        $j = 'Folder OneDrive'
        if ($o.Jenis -eq 'Pribadi') { $j = 'OneDrive pribadi' }
        elseif ($o.Jenis -eq 'Kantor/Sekolah') { $j = 'OneDrive kantor/sekolah' }
        $script:JenisSumber[$o.Folder] = $j
    }
}

function Set-TujuanBawaan {
    $s = Jalur-Sumber
    $remT = Tag-Terpilih $ui.LokasiTujuan
    $b = ''
    if ($s) {
        $b = Tujuan-Bawaan $s ([string]$remT)
        if ($remT) { $b = (Pecah $b).Path }
    }
    $script:IsiOtomatis = $true
    try { $ui.IsiTujuan.Text = $b } finally { $script:IsiOtomatis = $false }
    $script:TujuanManual = $false
    Tujuan-Berubah
}

# Muat rute tersimpan (atau bawaan) ke isian saat aplikasi dibuka.
function Isi-Rute {
    $r = Ambil-Rute
    $s = $null; $t = $null
    if ($r.Sumber) { $s = Pecah $r.Sumber }
    if ($r.Tujuan) { $t = Pecah $r.Tujuan }
    $script:IsiOtomatis = $true
    try {
        if ($s -and (Pilih-Tag $ui.LokasiSumber $s.Remote)) {
            Isi-SaranSumber
            $ui.PilihSumber.Text = $s.Path
        }
        else {
            [void](Pilih-Tag $ui.LokasiSumber '')
            Isi-SaranSumber
            if ($ui.PilihSumber.Items.Count -gt 0) { $ui.PilihSumber.Text = [string]$ui.PilihSumber.Items[0] }
        }
        if ($t -and (Pilih-Tag $ui.LokasiTujuan $t.Remote)) {
            $ui.IsiTujuan.Text = $t.Path
            $script:TujuanManual = ($r.Tujuan -ne (Tujuan-Bawaan $r.Sumber ([string]$t.Remote)))
        }
        else {
            [void](Pilih-Tag $ui.LokasiTujuan (Lokasi-TujuanBawaan))
            $script:TujuanManual = $false
        }
    }
    finally { $script:IsiOtomatis = $false }
    $script:RuteSiap = $true
    Sumber-Berubah
    Tujuan-Berubah
}

function Lokasi-SumberBerubah {
    $script:IsiOtomatis = $true
    try {
        Isi-SaranSumber
        $p = ''
        if (-not (Tag-Terpilih $ui.LokasiSumber) -and $ui.PilihSumber.Items.Count -gt 0) { $p = [string]$ui.PilihSumber.Items[0] }
        $ui.PilihSumber.Text = $p
    }
    finally { $script:IsiOtomatis = $false }
    Sumber-Berubah
}

function Sumber-Berubah {
    $rem = Tag-Terpilih $ui.LokasiSumber
    $s = Jalur-Sumber
    $script:SudahDiperiksa = $false
    $script:RingkasUji = $null
    $ui.PanelPeriksa.Visibility = 'Collapsed'
    if (-not $rem) {
        if (-not $s) { Set-Teks $ui.InfoSumber 'Pilih atau ketik folder di komputer ini, misalnya D:\Data' 'Redup' }
        elseif ((Adalah-Lokal $s) -and (Test-Path -LiteralPath $s -PathType Container)) {
            $j = $script:JenisSumber[$s]
            if (-not $j) { $j = 'Folder di komputer ini' }
            Set-Teks $ui.InfoSumber $j 'Redup'
        }
        elseif (Adalah-Lokal $s) { Set-Teks $ui.InfoSumber 'Folder tidak ditemukan' 'Merah' }
        else { Set-Teks $ui.InfoSumber 'Tulis alamat folder lengkap, misalnya D:\Data' 'Kuning' }
    }
    else {
        $a = Akun-Dari $rem
        $p = (Pecah $s).Path
        $t = 'Seluruh isi akun'
        if ($a -and $a.Jenis -eq 'sharepoint') { $t = 'Seluruh isi pustaka' }
        if ($p) { $t = "Folder /$p" }
        $w = 'Redup'
        if ($a -and $a.Jenis -eq 'google' -and $a.Scope -ne 'drive') {
            $t = 'Akun ini hanya diizinkan sebagai tujuan. Buka Atur di bagian Akun, lalu "Beri izin penuh" untuk memakainya sebagai sumber.'
            $w = 'Kuning'
        }
        elseif ($a -and $a.Jenis -eq 'sharepoint') { $t += " $TitikTengah pustaka $($a.Drive), situs $($a.Situs)" }
        elseif ($a -and $a.Drive) { $t += " $TitikTengah drive $($a.Drive)" }
        if ($w -eq 'Redup') { $t += '. Klik Telusuri untuk memilih subfolder.' }
        Set-Teks $ui.InfoSumber $t $w
    }
    if (-not $script:TujuanManual) { Set-TujuanBawaan }
}

function Tujuan-Berubah {
    $remT = Tag-Terpilih $ui.LokasiTujuan
    $t = Jalur-Tujuan
    if (-not $remT) {
        if (-not $t) { Set-Teks $ui.InfoTujuan 'Ketik folder tujuan di komputer ini, misalnya D:\Salinan' 'Redup' }
        elseif ((Adalah-Lokal $t) -and (Test-Path -LiteralPath $t -PathType Container)) { Set-Teks $ui.InfoTujuan 'Folder di komputer ini (sudah ada)' 'Redup' }
        elseif (Adalah-Lokal $t) { Set-Teks $ui.InfoTujuan 'Folder di komputer ini (akan dibuat)' 'Redup' }
        else { Set-Teks $ui.InfoTujuan 'Tulis alamat folder lengkap, misalnya D:\Salinan' 'Kuning' }
        return
    }
    $a = Akun-Dari $remT
    $p = (Pecah $t).Path
    $teks = 'Langsung di folder teratas akun'
    if ($p) { $teks = "Folder /$p (dibuat bila belum ada)" }
    $w = 'Redup'
    if ($a -and $a.Jenis -eq 'google' -and -not $a.ClientSendiri) {
        $teks += '. Akun ini memakai client ID bersama rclone yang sering dibatasi Google.'
        $w = 'Kuning'
    }
    elseif ($a -and $a.Jenis -eq 'sharepoint') {
        $teks += " $TitikTengah pustaka $($a.Drive), situs $($a.Situs). File Office akan ditambahi metadata oleh SharePoint (wajar)."
    }
    elseif ($a -and $a.Drive) { $teks += " $TitikTengah drive $($a.Drive)" }
    Set-Teks $ui.InfoTujuan $teks $w
}

function Lokasi-TujuanBerubah {
    if (-not $script:TujuanManual) { Set-TujuanBawaan; return }
    # Pindah antara komputer dan cloud: format jalurnya berbeda, jadi isi ulang.
    $remT = Tag-Terpilih $ui.LokasiTujuan
    $lokal = Adalah-Lokal ([string]$ui.IsiTujuan.Text)
    if ((-not $remT -and -not $lokal) -or ($remT -and $lokal)) { Set-TujuanBawaan; return }
    $script:SudahDiperiksa = $false
    Tujuan-Berubah
}

# Membaca isian, memvalidasi, dan menyimpannya sebagai rute (dibaca juga oleh pindah.ps1).
function Ambil-Isian {
    $remS = Tag-Terpilih $ui.LokasiSumber
    $remT = Tag-Terpilih $ui.LokasiTujuan
    $s = Jalur-Sumber
    $t = Jalur-Tujuan
    if (-not $remS) {
        if (-not $s) { Set-Pesan 'Pilih folder sumber dulu.' 'Merah'; return $null }
        if (-not (Adalah-Lokal $s) -or -not (Test-Path -LiteralPath $s -PathType Container)) { Set-Pesan 'Folder sumber tidak ditemukan.' 'Merah'; return $null }
    }
    if (-not $remT -and (-not $t -or -not (Adalah-Lokal $t))) { Set-Pesan 'Tulis folder tujuan lengkap, misalnya D:\Salinan.' 'Merah'; return $null }
    if (-not $script:Rclone) { Set-Pesan 'Pasang rclone dulu (bagian Akun).' 'Merah'; return $null }
    Simpan-Pengaturan $s $t
    return [pscustomobject]@{ Sumber = $s; Tujuan = $t }
}

function Pilih-FolderLokal([string]$awal, [string]$judul, [bool]$baru) {
    $d = New-Object System.Windows.Forms.FolderBrowserDialog
    $d.Description = $judul
    $d.ShowNewFolderButton = $baru
    if ($awal -and (Adalah-Lokal $awal) -and (Test-Path -LiteralPath $awal -PathType Container)) { $d.SelectedPath = $awal }
    if ("$($d.ShowDialog())" -eq 'OK') { return $d.SelectedPath }
    return $null
}

function Klik-Telusuri {
    $rem = Tag-Terpilih $ui.LokasiSumber
    if ($rem) {
        $p = Jelajah-Cloud $rem ((Pecah (Jalur-Sumber)).Path) ('Pilih folder SUMBER di ' + (Nama-Lokasi (Akun-Dari $rem)))
        if ($null -ne $p) { $ui.PilihSumber.Text = $p }
        return
    }
    $p = Pilih-FolderLokal (Jalur-Sumber) 'Pilih folder yang mau disalin.' $false
    if ($p) { $ui.PilihSumber.Text = $p }
}

function Klik-TelusuriTujuan {
    $rem = Tag-Terpilih $ui.LokasiTujuan
    if ($rem) {
        $p = Jelajah-Cloud $rem ((Pecah (Jalur-Tujuan)).Path) ('Pilih folder TUJUAN di ' + (Nama-Lokasi (Akun-Dari $rem)))
        if ($null -ne $p) { $ui.IsiTujuan.Text = $p }
        return
    }
    $p = Pilih-FolderLokal (Jalur-Tujuan) 'Pilih folder tujuan (boleh membuat folder baru).' $true
    if ($p) { $ui.IsiTujuan.Text = $p }
}

function Klik-Tukar {
    $remS = Tag-Terpilih $ui.LokasiSumber
    $remT = Tag-Terpilih $ui.LokasiTujuan
    $pS = [string]$ui.PilihSumber.Text
    $pT = [string]$ui.IsiTujuan.Text
    $script:IsiOtomatis = $true
    try {
        [void](Pilih-Tag $ui.LokasiSumber ([string]$remT))
        Isi-SaranSumber
        $ui.PilihSumber.Text = $pT
        [void](Pilih-Tag $ui.LokasiTujuan ([string]$remS))
        $ui.IsiTujuan.Text = $pS
    }
    finally { $script:IsiOtomatis = $false }
    $script:TujuanManual = $true
    Sumber-Berubah
    Tujuan-Berubah
    Set-Pesan 'Sumber dan tujuan ditukar.'
}

function Klik-BukaTujuan {
    $remT = Tag-Terpilih $ui.LokasiTujuan
    if (-not $remT) {
        $t = Jalur-Tujuan
        if ($t -and (Test-Path -LiteralPath $t -PathType Container)) { Start-Process -FilePath 'explorer.exe' -ArgumentList ('"' + $t + '"') }
        else { Set-Pesan 'Folder tujuan belum ada.' }
        return
    }
    $a = Akun-Dari $remT
    if ($a -and $a.Jenis -eq 'google') { Start-Process 'https://drive.google.com/drive/my-drive' }
    elseif ($a -and $a.Jenis -eq 'sharepoint') { Start-Process 'https://www.office.com/launch/sharepoint' }
    else { Start-Process 'https://www.office.com/launch/onedrive' }
}

# ---------------------------------------------------------------- periksa & uji

function Klik-Periksa {
    $r = Ambil-Isian
    if (-not $r) { $script:SalinSetelahUji = $false; return }
    $ui.PanelPeriksa.Visibility = 'Visible'
    $ui.JudulTujuanPanel.Text = Nama-Lokasi (Akun-Dari (Remote-Dari $r.Tujuan))
    Set-Teks $ui.HasilSumber $Elipsis
    Set-Teks $ui.HasilSumberKet ''
    Set-Teks $ui.HasilTujuan $Elipsis
    Set-Teks $ui.HasilTujuanKet ''
    Set-Teks $ui.HasilUji ''
    Tunjukkan $ui.PanelPeriksa
    Jalankan-Aksi 'Memeriksa isi sumber dan ruang tujuan' @('periksa') { param($kode, $hasil) Selesai-Periksa $kode $hasil }
}

function Selesai-Periksa($kode, $h) {
    if ($h -and $null -ne $h.SumberJumlah) {
        Set-Teks $ui.HasilSumber ('{0} file  {1}  {2}' -f (Angka $h.SumberJumlah), $TitikTengah, (Ukuran $h.SumberBytes))
        $ket = ''
        if ($h.HanyaCloudJumlah -gt 0) {
            $ket = ('{0} file masih "hanya online"; OneDrive mengunduhnya dulu ke disk (sisa {1}).' -f (Angka $h.HanyaCloudJumlah), (Ukuran $h.DiskBebas))
        }
        if ($h.DiskCukup -eq $false) { Set-Teks $ui.HasilSumberKet ($ket + ' Ruang disk tidak cukup; salin per subfolder.') 'Kuning' }
        else { Set-Teks $ui.HasilSumberKet $ket 'Redup' }
    }
    else {
        Set-Teks $ui.HasilSumber 'Tidak terbaca' 'Merah'
        Set-Teks $ui.HasilSumberKet 'Lihat log aktivitas.' 'Redup'
    }

    if ($h -and $null -ne $h.TujuanBebas) {
        $w = 'Tinta'
        if ($h.TujuanCukup -eq $false) { $w = 'Merah' }
        Set-Teks $ui.HasilTujuan ('Sisa {0}' -f (Ukuran $h.TujuanBebas)) $w
        if ($h.TujuanJenis) { $ket = ('dari {0}. Perlu disalin {1}.' -f (Ukuran $h.TujuanTotal), (Ukuran $h.Butuh)) }
        else { $ket = ('ruang disk. Perlu disalin {0}.' -f (Ukuran $h.Butuh)) }
        if ($h.TujuanSudah -gt 0) { $ket += (' {0} sudah ada di tujuan.' -f (Ukuran $h.TujuanSudah)) }
        if ($h.TujuanCukup -eq $false) { $ket += ' Tidak cukup: kosongkan ruang/sampah di tujuan atau pilih subfolder.' }
        Set-Teks $ui.HasilTujuanKet $ket 'Redup'
    }
    elseif ($h -and $h.TujuanTerhubung -eq $true) {
        Set-Teks $ui.HasilTujuan 'Kuota tidak dibatasi'
        Set-Teks $ui.HasilTujuanKet ''
    }
    elseif ($h -and $h.TujuanGalat) {
        Set-Teks $ui.HasilTujuan 'Tidak bisa diakses' 'Merah'
        Set-Teks $ui.HasilTujuanKet 'Coba "Login ulang" lewat Atur di bagian Akun.' 'Redup'
    }
    else {
        Set-Teks $ui.HasilTujuan 'Belum diperiksa' 'Redup'
        Set-Teks $ui.HasilTujuanKet 'Lihat log aktivitas.' 'Redup'
    }

    if (($kode -eq 0 -or $kode -eq 3) -and $h -and $h.TujuanTerhubung -eq $true) {
        Set-Teks $ui.HasilUji ('Uji coba' + $Elipsis) 'Redup'
        Jalankan-Aksi 'Uji coba (tidak menyalin apa pun)' @('uji') { param($k, $h2) Selesai-Uji $k $h2 }
        return
    }
    $script:SalinSetelahUji = $false
    if ($kode -eq 2) {
        Set-Teks $ui.HasilUji 'Ruang di tujuan tidak cukup untuk folder ini.' 'Merah'
        Set-Pesan 'Ruang di tujuan tidak cukup.' 'Merah'
    }
    else {
        Set-Teks $ui.HasilUji 'Pemeriksaan belum lengkap. Lihat log aktivitas.' 'Kuning'
        $ui.PanelLog.IsExpanded = $true
        Set-Pesan 'Pemeriksaan selesai dengan catatan.' 'Kuning'
    }
}

function Selesai-Uji($kode, $h) {
    if ($kode -eq 1 -or -not $h) {
        $script:SalinSetelahUji = $false
        Set-Teks $ui.HasilUji 'Uji coba gagal. Lihat log aktivitas.' 'Merah'
        $ui.PanelLog.IsExpanded = $true
        return
    }
    $script:RingkasUji = $h
    $w = 'Hijau'
    if ($h.AkanDisalin -eq 0) { $t = "$Centang  Semua file sudah ada di tujuan." }
    else { $t = ("$Centang  Siap: {0} file akan disalin ({1})." -f (Angka $h.AkanDisalin), (Ukuran $h.AkanBytes)) }
    if ($h.Terlewat -gt 0) {
        $t = ('{0} file tidak bisa dibaca dan akan terlewat. Coba pakai akun OneDrive (lewat internet) sebagai sumber.' -f (Angka $h.Terlewat))
        $w = 'Kuning'
    }
    elseif ($h.Masalah -gt 0) {
        $t += (' Ada {0} catatan masalah; lihat log aktivitas.' -f (Angka $h.Masalah))
        $w = 'Kuning'
    }
    Set-Teks $ui.HasilUji $t $w
    $script:SudahDiperiksa = $true
    Tunjukkan $ui.PanelPeriksa
    Set-Pesan 'Pemeriksaan selesai.'

    # Dipicu dari "Mulai salin": lanjut ke konfirmasi hanya bila uji coba bersih.
    if ($script:SalinSetelahUji) {
        $script:SalinSetelahUji = $false
        if ($w -eq 'Hijau') { Klik-Salin }
        else { Set-Pesan 'Baca hasil uji coba di atas sebelum menyalin.' 'Kuning' }
    }
}

# ---------------------------------------------------------------- salin

function Klik-Salin {
    $r = Ambil-Isian
    if (-not $r) { return }
    # Selalu periksa & uji coba dulu, supaya konfirmasi menyebut jumlah file yang sebenarnya.
    if (-not $script:SudahDiperiksa -or -not $script:RingkasUji) {
        $script:SalinSetelahUji = $true
        Klik-Periksa
        return
    }
    if ($script:RingkasUji.AkanDisalin -eq 0) {
        Set-Pesan 'Tidak ada yang perlu disalin: semua file sudah ada di tujuan.' 'Hijau'
        return
    }
    $isi = "Dari:`n    $(Nama-Jalur $r.Sumber)`n`nKe:`n    $(Nama-Jalur $r.Tujuan)"
    $isi += ("`n`nAkan disalin: {0} file, sekitar {1}." -f (Angka $script:RingkasUji.AkanDisalin), (Ukuran $script:RingkasUji.AkanBytes))
    $isi += "`n`nFile di sumber tidak diubah atau dihapus. Penyalinan berjalan di latar belakang; aplikasi boleh ditutup.`n`nMulai sekarang?"
    if (-not (Tanya $isi 'Mulai salin')) { return }
    Set-Teks $ui.HasilVerifikasi ''
    Jalankan-Aksi 'Memulai penyalinan' @('salin', '-Latar', '-Ya') {
        param($kode, $hasil)
        if ($kode -eq 0) { Set-Pesan 'Penyalinan berjalan di latar belakang.' 'Hijau' }
        else { Set-Pesan 'Penyalinan gagal dimulai. Lihat log aktivitas.' 'Merah'; $ui.PanelLog.IsExpanded = $true }
        Perbarui-Progres
    }
}

function Klik-Henti {
    $isi = "Hentikan penyalinan?`n`nFile yang sudah sampai tetap ada di tujuan. Klik ""Lanjutkan"" kapan saja untuk meneruskan sisanya."
    if (-not (Tanya $isi 'Hentikan penyalinan' 'Warning')) { return }
    [void](Perintah-Berhenti)
    Tulis-Log 'Penyalinan dihentikan.'
    Set-Pesan 'Penyalinan dihentikan.'
    Perbarui-Progres
}

# Apakah penyalinan ini melibatkan akun Google yang memakai client ID bersama?
function Job-ClientBersama($job) {
    foreach ($j in @($job.Sumber, $job.Tujuan)) {
        $a = Akun-Dari (Remote-Dari "$j")
        if ($a -and $a.Jenis -eq 'google' -and -not $a.ClientSendiri) { return $true }
    }
    return $false
}

# Dibaca ulang tiap ~1,6 detik dari job.json + log rclone.
function Perbarui-Progres {
    $job = Baca-Job
    if (-not $job -or -not $job.Log) {
        $ui.PanelProgres.Visibility = 'Collapsed'
        $script:JobJalan = $false
        Segarkan-Tombol
        return
    }
    $jalan = Job-Berjalan $job
    $s = Baca-Statistik $job.Log
    $baruTampil = ("$($ui.PanelProgres.Visibility)" -ne 'Visible')
    $ui.PanelProgres.Visibility = 'Visible'
    if ($baruTampil -and $jalan) { Tunjukkan $ui.PanelProgres }

    $persen = 0
    $detail = @()
    if ($s) {
        if ($s.Data -match '(\d+)%') { $persen = [int]$matches[1] }
        $bag = @($s.Data -split ',\s*')
        if ($bag[0] -match '^(.+?)\s*/\s*(.+)$') {
            $sudahT = $matches[1]
            $totalT = $matches[2]
            $totalB = 0
            if ($totalT -match '([\d.]+)\s*([KMGTP]?i?B)?') { $totalB = Ke-Byte $matches[1] ([string]$matches[2]) }
            if ($totalB -gt 0) { $detail += ('{0} dari {1}' -f (Ubah-Ukuran $sudahT), (Ubah-Ukuran $totalT)) }
            else { $detail += ('Menghitung daftar file' + $Elipsis) }
        }
        if ($s.File -match '^(\d+)\s*/\s*(\d+)') { $detail += ('{0} dari {1} file' -f (Angka $matches[1]), (Angka $matches[2])) }
        if ($jalan -and $bag.Count -gt 2 -and $bag[2] -match '/s$') { $detail += ((Ubah-Ukuran ($bag[2] -replace '/s$', '')) + '/dtk') }
        if ($jalan -and $s.Data -match 'ETA\s+(\S+)') {
            $e = Ubah-Durasi $matches[1]
            if ($e) { $detail += "sisa $e" }
        }
        if ($s.Error -and $s.Error -notmatch '^0\b') { $detail += ('{0} error' -f ($s.Error -replace '\s*\(.*$', '')) }
    }

    $catatan = 'Aplikasi boleh ditutup. Penyalinan tetap berjalan dan progresnya muncul lagi saat aplikasi dibuka.'
    $warnaCatatan = 'Redup'
    $saranClient = ''
    if (Job-ClientBersama $job) {
        if ($script:AdaClientBawaan) { $saranClient = ' Akun Google di penyalinan ini memakai client ID bersama rclone: hentikan, klik Atur > Login ulang (beralih ke client ID bawaan), lalu klik Lanjutkan.' }
        else { $saranClient = ' Akun Google di penyalinan ini memakai client ID bersama rclone: hentikan, pasang client ID sendiri (Akun > Atur), lalu klik Lanjutkan.' }
    }

    $ui.Progres.Foreground = $Warna.Hijau
    if ($jalan) {
        $ui.ProgresJudul.Text = 'Sedang menyalin'
        $f = ''
        if ($s -and @($s.Sedang).Count -gt 0) { $f = 'Sekarang: ' + (@($s.Sedang) -join ', ') }
        $ui.ProgresFile.Text = $f
        $ui.TombolSalin.Content = 'Mulai salin'
        # Tidak bergerak: byte tersalin sama selama 90 detik, atau log memuat penolakan rate limit.
        if ($s) {
            $kunci = "$($job.Log)|" + (($s.Data -split ',')[0])
            if ($kunci -ne $script:ProgresKunci) { $script:ProgresKunci = $kunci; $script:ProgresSejak = Get-Date }
            $diam = ((Get-Date) - $script:ProgresSejak).TotalSeconds
            if ($s.Dibatasi -or $diam -ge 90) {
                if ($s.Dibatasi) { $catatan = 'Layanan cloud sedang membatasi permintaan (rate limit). rclone menunggu lalu mencoba lagi otomatis.' }
                else { $catatan = ('Tidak ada kemajuan sejak {0}. Biasanya karena layanan cloud membatasi permintaan; rclone menunggu lalu mencoba lagi otomatis.' -f $script:ProgresSejak.ToString('HH.mm')) }
                $catatan += $saranClient
                $warnaCatatan = 'Kuning'
                $ui.Progres.Foreground = $Warna.Kuning
            }
        }
        $ui.ProgresCatatan.Visibility = 'Visible'
    }
    elseif ("$($job.Kode)" -eq '0') {
        $persen = 100
        $ui.ProgresJudul.Text = 'Penyalinan selesai'
        $ui.ProgresFile.Text = 'Selesai ' + (Waktu-Tampil $job.Selesai) + '.'
        $ui.ProgresCatatan.Visibility = 'Collapsed'
        $ui.TombolSalin.Content = 'Salin lagi'
    }
    else {
        $ui.Progres.Foreground = $Warna.Kuning
        if ("$($job.Kode)" -eq 'dihentikan') {
            $ui.ProgresJudul.Text = 'Dihentikan'
            $ui.ProgresFile.Text = 'Klik "Lanjutkan" untuk meneruskan; file yang sudah sampai dilewati.'
        }
        elseif ($job.Selesai) {
            $ui.ProgresJudul.Text = 'Berhenti karena ada kesalahan'
            $err = @(Baris-Error $job.Log 1)
            $f = 'Lihat log untuk detailnya.'
            if ($err.Count -gt 0) { $f = ($err[0] -replace '^\S+\s+\S+\s+ERROR\s*:\s*', '') }
            $ui.ProgresFile.Text = $f
        }
        else {
            $ui.ProgresJudul.Text = 'Terputus'
            $ui.ProgresFile.Text = 'Komputer mati atau proses ditutup. Klik "Lanjutkan" untuk meneruskan.'
        }
        $ui.ProgresCatatan.Visibility = 'Collapsed'
        if ($s -and $s.Dibatasi) {
            $catatan = 'Log menunjukkan layanan cloud membatasi permintaan (rate limit).' + $saranClient
            $warnaCatatan = 'Kuning'
            $ui.ProgresCatatan.Visibility = 'Visible'
        }
        $ui.TombolSalin.Content = 'Lanjutkan'
    }
    $ui.ProgresCatatan.Text = $catatan
    $ui.ProgresCatatan.Foreground = $Warna[$warnaCatatan]
    $ui.Progres.Value = $persen
    $ui.ProgresPersen.Text = "$persen%"
    $ui.ProgresDetail.Text = ($detail -join "   $TitikTengah   ")

    # Penyalinan baru saja selesai saat aplikasi terbuka: langsung verifikasi.
    if ($script:JobJalan -and -not $jalan -and "$($job.Kode)" -eq '0' -and -not $script:Aksi) {
        $script:JobJalan = $false
        Set-Pesan ('Penyalinan selesai. Memverifikasi' + $Elipsis) 'Hijau'
        Klik-Verifikasi
    }
    $script:JobJalan = $jalan
    Segarkan-Tombol
}

# ---------------------------------------------------------------- verifikasi

function Klik-Verifikasi {
    $r = Ambil-Isian
    if (-not $r) { return }
    Set-Teks $ui.HasilVerifikasi ('Mencocokkan setiap file' + $Elipsis) 'Redup'
    Tunjukkan $ui.HasilVerifikasi
    Jalankan-Aksi 'Memverifikasi hasil salinan' @('verifikasi') { param($kode, $hasil) Selesai-Verifikasi $kode $hasil }
}

function Selesai-Verifikasi($kode, $h) {
    if (-not $h -or $null -eq $h.Cocok) {
        Set-Teks $ui.HasilVerifikasi 'Verifikasi tidak berjalan. Lihat log aktivitas.' 'Merah'
        $ui.PanelLog.IsExpanded = $true
        return
    }
    $cara = 'dicocokkan dengan checksum'
    if ($h.Metode -eq 'ukuran') { $cara = 'dicocokkan per nama & ukuran, karena kedua layanan tidak punya jenis checksum yang sama' }
    if ($h.OfficeSharePoint -gt 0) { $cara += ('; {0} file Office diubah metadatanya oleh SharePoint dan dianggap cocok' -f (Angka $h.OfficeSharePoint)) }
    if ($kode -eq 0) {
        Set-Teks $ui.HasilVerifikasi ("$Centang  Semua {0} file dari {1} sudah ada di tujuan ({2})." -f (Angka $h.Cocok), (Nama-Jalur "$($h.Sumber)"), $cara) 'Hijau'
        Set-Pesan 'Selesai. Salinan di tujuan lengkap.' 'Hijau'
        return
    }
    $bag = @(('{0} cocok' -f (Angka $h.Cocok)))
    if ($h.Hilang -gt 0) { $bag += ('{0} belum ada di tujuan' -f (Angka $h.Hilang)) }
    if ($h.Beda -gt 0)   { $bag += ('{0} isinya berbeda' -f (Angka $h.Beda)) }
    if ($h.Galat -gt 0)  { $bag += ('{0} gagal dibaca' -f (Angka $h.Galat)) }
    Set-Teks $ui.HasilVerifikasi (($bag -join ', ') + '. Klik "Lanjutkan" untuk melengkapi, lalu verifikasi lagi.') 'Kuning'
    $ui.TombolSalin.Content = 'Lanjutkan'
    Set-Pesan 'Verifikasi menemukan selisih.' 'Kuning'
}

# ---------------------------------------------------------------- mulai

try {
    . $SkripMesin   # muat fungsi mesin (Cari-Rclone, Daftar-Akun, Baca-Job, Baca-Statistik, ...)

    [xml]$xml = [System.IO.File]::ReadAllText((Join-Path $Akar 'tampilan.xaml'), [System.Text.Encoding]::UTF8)
    $Jendela = [System.Windows.Markup.XamlReader]::Load((New-Object System.Xml.XmlNodeReader $xml))
    $ui = @{}
    foreach ($n in $xml.SelectNodes('//*')) {
        $nama = $n.GetAttribute('Name', $NsX)
        if ($nama) { $ui[$nama] = $Jendela.FindName($nama) }
    }

    # Layar laptop kecil: jangan lebih tinggi dari area kerja.
    $area = [System.Windows.SystemParameters]::WorkArea
    if ($Jendela.Height -gt ($area.Height - 24)) { $Jendela.Height = [Math]::Max($Jendela.MinHeight, $area.Height - 24) }

    # Dimuat ke memori (OnLoad) lalu file ditutup: BitmapFrame.Create(Uri) mengunci ikon.ico
    # selama aplikasi terbuka, sehingga build/installer tidak bisa menimpanya.
    $ikon = Join-Path $Akar 'aset\ikon.ico'
    if (Test-Path -LiteralPath $ikon) {
        try {
            $fs = [System.IO.File]::OpenRead($ikon)
            try { $Jendela.Icon = [System.Windows.Media.Imaging.BitmapFrame]::Create($fs, 'None', 'OnLoad') } finally { $fs.Close() }
        } catch { }
    }

    $ui.TombolPasang.Add_Click({ Aman { Klik-Pasang } })
    $ui.TombolTambahGoogle.Add_Click({ Aman { Klik-Tambah 'google' } })
    $ui.TombolTambahOneDrive.Add_Click({ Aman { Klik-Tambah 'onedrive' } })
    $ui.TombolTambahSharePoint.Add_Click({ Aman { Klik-Tambah 'sharepoint' } })
    $ui.TombolTelusuri.Add_Click({ Aman { Klik-Telusuri } })
    $ui.TombolTelusuriTujuan.Add_Click({ Aman { Klik-TelusuriTujuan } })
    $ui.TombolTukar.Add_Click({ Aman { Klik-Tukar } })
    $ui.TombolPeriksa.Add_Click({ Aman { Klik-Periksa } })
    $ui.TombolSalin.Add_Click({ Aman { Klik-Salin } })
    $ui.TombolHenti.Add_Click({ Aman { Klik-Henti } })
    $ui.TombolVerifikasi.Add_Click({ Aman { Klik-Verifikasi } })
    $ui.TombolBatal.Add_Click({ Aman { Klik-Batal } })
    $ui.TombolBukaLogin.Add_Click({ Aman { if ($script:UrlLogin) { Start-Process $script:UrlLogin } } })
    $ui.TombolLog.Add_Click({ Aman { Pastikan-Dir; Start-Process -FilePath 'explorer.exe' -ArgumentList ('"' + $DirLog + '"') } })
    $ui.TombolDrive.Add_Click({ Aman { Klik-BukaTujuan } })

    $ui.LokasiSumber.Add_SelectionChanged({ if (-not $script:IsiOtomatis -and $script:RuteSiap) { Aman { Lokasi-SumberBerubah } } })
    $ui.LokasiTujuan.Add_SelectionChanged({ if (-not $script:IsiOtomatis -and $script:RuteSiap) { Aman { Lokasi-TujuanBerubah } } })
    $ui.PilihSumber.AddHandler(
        [System.Windows.Controls.Primitives.TextBoxBase]::TextChangedEvent,
        [System.Windows.Controls.TextChangedEventHandler]{ if (-not $script:IsiOtomatis -and $script:RuteSiap) { Aman { Sumber-Berubah } } })
    $ui.IsiTujuan.Add_TextChanged({
        if (-not $script:IsiOtomatis -and $script:RuteSiap) {
            $script:TujuanManual = [bool]$ui.IsiTujuan.Text.Trim()
            $script:SudahDiperiksa = $false
            Aman { Tujuan-Berubah }
        }
    })

    $Timer = New-Object System.Windows.Threading.DispatcherTimer
    $Timer.Interval = [TimeSpan]::FromMilliseconds(200)
    $Timer.Add_Tick({
        Aman { Proses-Aksi }
        $script:Detak++
        if ($script:Detak % 8 -eq 0) { Aman { Perbarui-Progres } }
    })

    $Jendela.Add_Closing({
        param($pengirim, $e)
        if ($script:Aksi) {
            $isi = "$($script:Aksi.Judul) masih berjalan.`n`nHentikan dan tutup aplikasi?"
            if (-not (Tanya $isi 'Tutup Pindah Drive' 'Warning')) { $e.Cancel = $true; return }
            Klik-Batal
        }
        $Timer.Stop()
    })

    if ($Pratinjau) {
        $Jendela.WindowStartupLocation = 'Manual'
        $Jendela.Left = -20000
        $Jendela.Top = 0
        $Jendela.ShowActivated = $false
        $Jendela.ShowInTaskbar = $false
        if ($env:PINDAH_DRIVE_PRATINJAU_LOG) { $ui.PanelLog.IsExpanded = $true }
        if ($env:PINDAH_DRIVE_PRATINJAU_TINGGI) { $Jendela.Height = [int]$env:PINDAH_DRIVE_PRATINJAU_TINGGI }
        $Jendela.Add_ContentRendered({
            Aman { Segarkan-Status; Isi-Rute; Perbarui-Progres; Tulis-Log 'Contoh baris log.' }
            $Jendela.UpdateLayout()
            $isi = $Jendela.Content
            Simpan-Png $isi $isi.ActualWidth $isi.ActualHeight $Pratinjau
            # Opsional: render juga satu dialog (akun-google, akun-onedrive, client, jelajah:<akun>).
            $dlg = "$env:PINDAH_DRIVE_PRATINJAU_DIALOG"
            if ($dlg) {
                $d = $null
                if ($dlg -eq 'akun-google') { $d = Buat-DialogAkun 'google' }
                elseif ($dlg -eq 'akun-onedrive') { $d = Buat-DialogAkun 'onedrive' }
                elseif ($dlg -eq 'akun-sharepoint') { $d = Buat-DialogAkun 'sharepoint' }
                elseif ($dlg -like 'sharepoint:*') {
                    # sharepoint:<akun>:<kata kunci>
                    $bag = $dlg.Split(':')
                    $d = Buat-DialogSharePoint $bag[1]
                    $d.UI.IsiCari.Text = $bag[2]
                    SP-Cari
                    for ($i = 0; $i -lt 120 -and ($script:SP.Tugas -or $script:SP.Mode -eq 'situs'); $i++) {
                        Start-Sleep -Milliseconds 250
                        SP-Tick
                        if ($script:SP.Mode -eq 'situs' -and -not $script:SP.Tugas -and $d.UI.DaftarSitus.Items.Count -gt 0) {
                            $d.UI.DaftarSitus.SelectedIndex = 0   # memicu SP-MuatPustaka lewat event
                        }
                    }
                    $script:SP.Timer.Stop()
                }
                elseif ($dlg -eq 'client') { $d = Buat-DialogClientId }
                elseif ($dlg -like 'jelajah:*') {
                    $d = Buat-DialogJelajah ($dlg.Substring(8)) '' 'Pilih folder SUMBER di contoh akun'
                    for ($i = 0; $i -lt 60 -and $script:J.Tugas; $i++) { Start-Sleep -Milliseconds 250; Jelajah-Tick }
                    $script:J.Timer.Stop()
                }
                if ($d) {
                    $d.W.WindowStartupLocation = 'Manual'; $d.W.Left = -20000; $d.W.Top = 0
                    $d.W.ShowActivated = $false
                    $d.W.Show()
                    $d.W.UpdateLayout()
                    $k = $d.W.Content
                    Simpan-Png $k $k.ActualWidth $k.ActualHeight ($Pratinjau -replace '\.png$', '-dialog.png')
                    $d.W.Close()
                }
            }
            $Jendela.Close()
        })
    }
    else {
        $Jendela.Add_ContentRendered({
            Aman { Segarkan-Status; Isi-Rute; Perbarui-Progres }
            if (-not $script:Rclone) { Set-Pesan 'Mulai dari langkah 1: pasang rclone.' 'Tinta' }
            elseif ($script:DaftarAkun.Count -eq 0) { Set-Pesan 'Langkah berikutnya: tambahkan akun Google Drive atau OneDrive.' 'Tinta' }
            $Timer.Start()
        })
    }

    [void]$Jendela.ShowDialog()
}
catch {
    $pesan = ($_ | Out-String) + "`r`n" + $_.ScriptStackTrace
    if ($_.Exception.InnerException -and $_.Exception.InnerException.ErrorRecord) {
        $pesan += "`r`n--- penyebab:`r`n" + $_.Exception.InnerException.ErrorRecord.ScriptStackTrace
    }
    try {
        $dir = Join-Path $env:LOCALAPPDATA 'pindah-drive'
        if (-not (Test-Path -LiteralPath $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }
        Add-Content -LiteralPath (Join-Path $dir 'gui-galat.log') -Value ("[{0}]`r`n{1}" -f (Get-Date -Format s), $pesan)
    } catch { }
    if ($Pratinjau) { Write-Error $pesan; exit 1 }
    [void][System.Windows.MessageBox]::Show("Pindah Drive gagal dibuka:`n`n$($_.Exception.Message)", 'Pindah Drive', 'OK', 'Error')
    exit 1
}
finally {
    if ($kunciBaru) { try { $Kunci.ReleaseMutex() } catch { } }
}
