// Pemasang Pindah Drive: per pengguna, tanpa hak admin.
//
//   PindahDrive-Setup.exe                 jendela pemasangan
//   PindahDrive-Setup.exe /silent         pasang tanpa pertanyaan (pintasan Desktop dibuat)
//   ... /silent /nodesktop                tanpa pintasan Desktop
//   uninstall.exe /uninstall [/silent]    hapus (dipanggil dari Apps & features)
//
// Isi aplikasi (termasuk rclone.exe) tertanam sebagai resource "payload.zip".
// Dipasang ke %LOCALAPPDATA%\Programs\Pindah Drive, pintasan di Start Menu (dan Desktop),
// serta terdaftar di HKCU ...\Uninstall agar bisa dihapus dari Settings > Apps.
// Koneksi akun (%APPDATA%\rclone) dan data kerja (%LOCALAPPDATA%\pindah-drive) tidak disentuh.
//
// Dikompilasi oleh buat-installer.ps1 memakai csc.exe bawaan .NET Framework 4.x (C# 5).

using System;
using System.Diagnostics;
using System.Drawing;
using System.IO;
using System.IO.Compression;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Threading;
using System.Windows.Forms;
using Microsoft.Win32;

[assembly: AssemblyTitle("Pindah Drive Setup")]
[assembly: AssemblyProduct("Pindah Drive")]
[assembly: AssemblyDescription("Pemasang Pindah Drive")]
[assembly: AssemblyVersion(Info.Versi + ".0")]
[assembly: AssemblyFileVersion(Info.Versi + ".0")]

static class Pemasang
{
    public const string Nama = "Pindah Drive";
    const string KunciUninstall = @"Software\Microsoft\Windows\CurrentVersion\Uninstall\PindahDrive";
    const string NamaMutex = @"Local\PindahDriveGui";

    public static string FolderBawaan
    {
        get { return Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), @"Programs\Pindah Drive"); }
    }
    static string PintasanStart
    {
        get { return Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.Programs), Nama + ".lnk"); }
    }
    static string PintasanDesktop
    {
        get { return Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory), Nama + ".lnk"); }
    }

    [STAThread]
    static int Main(string[] args)
    {
        Application.EnableVisualStyles();
        Application.SetCompatibleTextRenderingDefault(false);
        bool copot = false, diam = false, tanpaDesktop = false;
        string dir = null;
        for (int i = 0; i < args.Length; i++)
        {
            string a = args[i].ToLowerInvariant();
            if (a == "/uninstall") copot = true;
            else if (a == "/silent" || a == "/s") diam = true;
            else if (a == "/nodesktop") tanpaDesktop = true;
            else if (a == "/dir" && i + 1 < args.Length) dir = args[++i];
        }
        // uninstall.exe adalah build yang sama tanpa muatan: dibuka langsung pun berarti "hapus".
        if (!copot && Assembly.GetExecutingAssembly().GetManifestResourceInfo("payload.zip") == null) copot = true;
        if (copot) return Copot(dir, diam);
        if (diam)
        {
            try { Pasang(dir ?? FolderBawaan, !tanpaDesktop, null); return 0; }
            catch { return 1; }
        }
        Application.Run(new FormPasang());
        return 0;
    }

    // ------------------------------------------------------------ keadaan

    public static string FolderTerpasang()
    {
        using (RegistryKey k = Registry.CurrentUser.OpenSubKey(KunciUninstall))
        {
            if (k == null) return null;
            return k.GetValue("InstallLocation") as string;
        }
    }

    public static string VersiTerpasang()
    {
        using (RegistryKey k = Registry.CurrentUser.OpenSubKey(KunciUninstall))
        {
            if (k == null) return null;
            return k.GetValue("DisplayVersion") as string;
        }
    }

    public static bool AplikasiTerbuka()
    {
        Mutex m;
        if (Mutex.TryOpenExisting(NamaMutex, out m)) { m.Dispose(); return true; }
        return false;
    }

    // rclone milik folder ini sedang dipakai (penyalinan latar belakang berjalan)?
    public static bool RcloneBerjalan(string dir)
    {
        string akar = Path.GetFullPath(dir).TrimEnd('\\') + "\\";
        foreach (Process p in Process.GetProcessesByName("rclone"))
        {
            try
            {
                if (p.MainModule.FileName.StartsWith(akar, StringComparison.OrdinalIgnoreCase)) return true;
            }
            catch { }
        }
        return false;
    }

    static long UkuranFolder(string dir)
    {
        long n = 0;
        foreach (string f in Directory.GetFiles(dir, "*", SearchOption.AllDirectories)) n += new FileInfo(f).Length;
        return n;
    }

    // ------------------------------------------------------------ pasang

    public static void Pasang(string dir, bool desktop, Action<int, string> lapor)
    {
        if (lapor == null) lapor = delegate (int p, string s) { };
        dir = Path.GetFullPath(dir);
        if (RcloneBerjalan(dir))
            throw new InvalidOperationException("Penyalinan sedang berjalan. Hentikan dulu dari aplikasi Pindah Drive, lalu ulangi.");
        Directory.CreateDirectory(dir);
        string akar = dir.TrimEnd('\\') + "\\";

        using (Stream s = Assembly.GetExecutingAssembly().GetManifestResourceStream("payload.zip"))
        using (ZipArchive zip = new ZipArchive(s, ZipArchiveMode.Read))
        {
            int n = 0, total = zip.Entries.Count;
            foreach (ZipArchiveEntry e in zip.Entries)
            {
                n++;
                string target = Path.GetFullPath(Path.Combine(dir, e.FullName));
                if (!target.StartsWith(akar, StringComparison.OrdinalIgnoreCase)) continue;   // jalur aneh di zip
                if (e.FullName.EndsWith("/") || e.FullName.EndsWith("\\")) { Directory.CreateDirectory(target); continue; }
                Directory.CreateDirectory(Path.GetDirectoryName(target));
                lapor(n * 90 / Math.Max(1, total), "Menyalin " + e.FullName);
                using (Stream masuk = e.Open())
                using (FileStream keluar = File.Create(target))
                {
                    masuk.CopyTo(keluar);
                }
                File.SetLastWriteTime(target, e.LastWriteTime.LocalDateTime);
            }
        }

        string exe = Path.Combine(dir, "Pindah Drive.exe");
        string uninst = Path.Combine(dir, "uninstall.exe");
        // uninstall.exe kecil biasanya ikut di muatan; bila tidak, pakai salinan installer ini.
        string saya = Path.GetFullPath(Assembly.GetExecutingAssembly().Location);
        if (!File.Exists(uninst) && !string.Equals(saya, uninst, StringComparison.OrdinalIgnoreCase)) File.Copy(saya, uninst, true);

        lapor(93, "Membuat pintasan");
        BuatPintasan(PintasanStart, exe, dir);
        if (desktop) BuatPintasan(PintasanDesktop, exe, dir);

        lapor(97, "Mendaftarkan di Settings > Apps");
        using (RegistryKey k = Registry.CurrentUser.CreateSubKey(KunciUninstall))
        {
            k.SetValue("DisplayName", Nama);
            k.SetValue("DisplayVersion", Info.Versi);
            k.SetValue("Publisher", Nama);
            k.SetValue("InstallLocation", dir);
            k.SetValue("DisplayIcon", exe);
            k.SetValue("UninstallString", "\"" + uninst + "\" /uninstall");
            k.SetValue("QuietUninstallString", "\"" + uninst + "\" /uninstall /silent");
            k.SetValue("NoModify", 1, RegistryValueKind.DWord);
            k.SetValue("NoRepair", 1, RegistryValueKind.DWord);
            k.SetValue("EstimatedSize", (int)(UkuranFolder(dir) / 1024), RegistryValueKind.DWord);
        }
        lapor(100, "Selesai");
    }

    // Pintasan .lnk lewat WScript.Shell (tanpa pustaka tambahan).
    static void BuatPintasan(string lnk, string target, string kerja)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(lnk));
        Type t = Type.GetTypeFromProgID("WScript.Shell");
        object sh = Activator.CreateInstance(t);
        try
        {
            object s = t.InvokeMember("CreateShortcut", BindingFlags.InvokeMethod, null, sh, new object[] { lnk });
            Type st = s.GetType();
            st.InvokeMember("TargetPath", BindingFlags.SetProperty, null, s, new object[] { target });
            st.InvokeMember("WorkingDirectory", BindingFlags.SetProperty, null, s, new object[] { kerja });
            st.InvokeMember("IconLocation", BindingFlags.SetProperty, null, s, new object[] { target + ",0" });
            st.InvokeMember("Description", BindingFlags.SetProperty, null, s, new object[] { "Salin file antar OneDrive, Google Drive, dan komputer" });
            st.InvokeMember("Save", BindingFlags.InvokeMethod, null, s, null);
            Marshal.ReleaseComObject(s);
        }
        finally { Marshal.ReleaseComObject(sh); }
    }

    // ------------------------------------------------------------ hapus

    // Hanya folder yang memang hasil pemasangan ini yang boleh dihapus.
    static bool FolderAman(string dir)
    {
        if (string.IsNullOrEmpty(dir)) return false;
        dir = Path.GetFullPath(dir).TrimEnd('\\');
        if (dir.Length < 10) return false;
        if (!File.Exists(Path.Combine(dir, "Pindah Drive.exe")) || !File.Exists(Path.Combine(dir, "pindah.ps1"))) return false;
        string terdaftar = FolderTerpasang();
        if (terdaftar != null) return string.Equals(Path.GetFullPath(terdaftar).TrimEnd('\\'), dir, StringComparison.OrdinalIgnoreCase);
        string programs = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Programs");
        return dir.StartsWith(programs + "\\", StringComparison.OrdinalIgnoreCase);
    }

    static int Copot(string dir, bool diam)
    {
        string saya = Path.GetFullPath(Assembly.GetExecutingAssembly().Location);
        if (dir == null) dir = Path.GetDirectoryName(saya);
        dir = Path.GetFullPath(dir).TrimEnd('\\');

        // uninstall.exe berada di dalam folder yang akan dihapus: jalankan ulang dari salinan di TEMP.
        if (saya.StartsWith(dir + "\\", StringComparison.OrdinalIgnoreCase))
        {
            string salinan = Path.Combine(Path.GetTempPath(), "PindahDrive-uninstall-" + Guid.NewGuid().ToString("N").Substring(0, 8) + ".exe");
            File.Copy(saya, salinan, true);
            ProcessStartInfo psi = new ProcessStartInfo(salinan, "/uninstall /dir \"" + dir + "\"" + (diam ? " /silent" : ""));
            psi.UseShellExecute = false;
            Process.Start(psi);
            return 0;
        }

        if (!FolderAman(dir))
        {
            if (!diam) MessageBox.Show("Folder ini bukan pemasangan Pindah Drive, jadi tidak dihapus:\n" + dir, Nama, MessageBoxButtons.OK, MessageBoxIcon.Warning);
            return 3;
        }
        if (!diam && MessageBox.Show("Hapus Pindah Drive dari komputer ini?\n\nKoneksi akun, pengaturan, dan log tidak ikut dihapus.",
                Nama, MessageBoxButtons.YesNo, MessageBoxIcon.Question) != DialogResult.Yes) return 1;

        while (AplikasiTerbuka())
        {
            if (diam) return 2;
            if (MessageBox.Show("Tutup jendela Pindah Drive dulu, lalu klik Retry.", Nama, MessageBoxButtons.RetryCancel, MessageBoxIcon.Information) != DialogResult.Retry) return 1;
        }
        if (RcloneBerjalan(dir))
        {
            if (!diam) MessageBox.Show("Penyalinan sedang berjalan. Hentikan dulu dari aplikasi Pindah Drive, lalu ulangi.", Nama, MessageBoxButtons.OK, MessageBoxIcon.Warning);
            return 2;
        }

        foreach (string lnk in new string[] { PintasanStart, PintasanDesktop })
        {
            try { if (File.Exists(lnk)) File.Delete(lnk); } catch { }
        }
        Registry.CurrentUser.DeleteSubKeyTree(KunciUninstall, false);
        for (int i = 0; i < 5; i++)
        {
            try { Directory.Delete(dir, true); break; }
            catch { Thread.Sleep(800); }
        }

        if (!diam)
        {
            string data = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "pindah-drive");
            string conf = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), @"rclone\rclone.conf");
            MessageBox.Show("Pindah Drive sudah dihapus.\n\nYang masih tersisa (boleh dihapus sendiri):\n- " + data +
                "  (pengaturan & log)\n- " + conf + "  (koneksi akun)", Nama, MessageBoxButtons.OK, MessageBoxIcon.Information);
        }

        // Hapus salinan TEMP ini setelah proses selesai.
        ProcessStartInfo hapus = new ProcessStartInfo("cmd.exe", "/c ping 127.0.0.1 -n 3 >nul & del /f /q \"" + saya + "\"");
        hapus.UseShellExecute = false;
        hapus.CreateNoWindow = true;
        Process.Start(hapus);
        return 0;
    }
}

// ------------------------------------------------------------ jendela pemasangan

class FormPasang : Form
{
    readonly TextBox isiFolder;
    readonly CheckBox cbDesktop, cbBuka;
    readonly ProgressBar bar;
    readonly Label status;
    readonly Button tPasang, tBatal, tUbah;
    readonly Color redup = Color.FromArgb(89, 99, 110);
    bool selesai;

    public FormPasang()
    {
        Text = "Pasang " + Pemasang.Nama;
        Font = new Font("Segoe UI", 9.75f);
        AutoScaleDimensions = new SizeF(96f, 96f);
        AutoScaleMode = AutoScaleMode.Dpi;
        FormBorderStyle = FormBorderStyle.FixedDialog;
        MaximizeBox = false;
        MinimizeBox = false;
        StartPosition = FormStartPosition.CenterScreen;
        ClientSize = new Size(540, 330);
        BackColor = Color.White;
        try { Icon = Icon.ExtractAssociatedIcon(Application.ExecutablePath); } catch { }

        string terpasang = Pemasang.FolderTerpasang();
        string versiLama = Pemasang.VersiTerpasang();

        Label judul = new Label();
        judul.Text = Pemasang.Nama;
        judul.Font = new Font("Segoe UI Semibold", 15f);
        judul.AutoSize = true;
        judul.Location = new Point(24, 18);

        Label ket = new Label();
        ket.Text = "Salin file antar OneDrive, Google Drive, dan komputer ini.\nVersi " + Info.Versi +
                   ", dipasang untuk akun Windows ini saja (tanpa hak admin).";
        if (versiLama != null) ket.Text += "\nVersi " + versiLama + " sudah terpasang dan akan diperbarui.";
        ket.ForeColor = redup;
        ket.AutoSize = false;
        ket.Location = new Point(26, 56);
        ket.Size = new Size(490, 54);

        Label lblFolder = new Label();
        lblFolder.Text = "Folder pemasangan";
        lblFolder.ForeColor = redup;
        lblFolder.AutoSize = true;
        lblFolder.Location = new Point(26, 118);

        isiFolder = new TextBox();
        isiFolder.Text = terpasang ?? Pemasang.FolderBawaan;
        isiFolder.Location = new Point(28, 140);
        isiFolder.Size = new Size(392, 26);

        tUbah = new Button();
        tUbah.Text = "Ubah…";
        tUbah.Location = new Point(428, 138);
        tUbah.Size = new Size(84, 29);
        tUbah.Click += delegate { PilihFolder(); };

        cbDesktop = new CheckBox();
        cbDesktop.Text = "Buat pintasan di Desktop";
        cbDesktop.Checked = true;
        cbDesktop.AutoSize = true;
        cbDesktop.Location = new Point(28, 182);

        cbBuka = new CheckBox();
        cbBuka.Text = "Buka Pindah Drive setelah selesai";
        cbBuka.Checked = true;
        cbBuka.AutoSize = true;
        cbBuka.Location = new Point(28, 208);

        bar = new ProgressBar();
        bar.Location = new Point(28, 246);
        bar.Size = new Size(484, 10);
        bar.Visible = false;

        status = new Label();
        status.AutoSize = false;
        status.ForeColor = redup;
        status.Location = new Point(26, 262);
        status.Size = new Size(490, 20);

        tBatal = new Button();
        tBatal.Text = "Batal";
        tBatal.Location = new Point(344, 288);
        tBatal.Size = new Size(80, 30);
        tBatal.Click += delegate { Close(); };

        tPasang = new Button();
        tPasang.Text = versiLama != null ? "Perbarui" : "Pasang";
        tPasang.Location = new Point(432, 288);
        tPasang.Size = new Size(80, 30);
        tPasang.Click += delegate { KlikPasang(); };

        AcceptButton = tPasang;
        CancelButton = tBatal;
        Controls.AddRange(new Control[] { judul, ket, lblFolder, isiFolder, tUbah, cbDesktop, cbBuka, bar, status, tBatal, tPasang });
    }

    void PilihFolder()
    {
        using (FolderBrowserDialog d = new FolderBrowserDialog())
        {
            d.Description = "Pilih folder induk; Pindah Drive dipasang di subfolder \"Pindah Drive\".";
            if (d.ShowDialog(this) == DialogResult.OK) isiFolder.Text = Path.Combine(d.SelectedPath, "Pindah Drive");
        }
    }

    void KlikPasang()
    {
        if (selesai) { Close(); return; }
        string dir = isiFolder.Text.Trim();
        if (dir.Length == 0) return;

        while (Pemasang.AplikasiTerbuka())
        {
            if (MessageBox.Show(this, "Tutup jendela Pindah Drive dulu, lalu klik Retry.", Pemasang.Nama,
                    MessageBoxButtons.RetryCancel, MessageBoxIcon.Information) != DialogResult.Retry) return;
        }

        foreach (Control c in new Control[] { isiFolder, tUbah, cbDesktop, cbBuka, tPasang, tBatal }) c.Enabled = false;
        bar.Visible = true;
        Cursor = Cursors.WaitCursor;
        try
        {
            Pemasang.Pasang(dir, cbDesktop.Checked, delegate (int persen, string pesan)
            {
                bar.Value = Math.Min(100, Math.Max(0, persen));
                status.Text = pesan;
                Application.DoEvents();
            });
        }
        catch (Exception e)
        {
            Cursor = Cursors.Default;
            status.ForeColor = Color.FromArgb(207, 34, 46);
            status.Text = "Gagal: " + e.Message;
            foreach (Control c in new Control[] { isiFolder, tUbah, cbDesktop, cbBuka, tPasang, tBatal }) c.Enabled = true;
            return;
        }
        Cursor = Cursors.Default;
        selesai = true;
        status.ForeColor = Color.FromArgb(26, 127, 55);
        status.Text = "Selesai. Pindah Drive ada di Start Menu" + (cbDesktop.Checked ? " dan Desktop." : ".");

        if (cbBuka.Checked)
        {
            try { Process.Start(Path.Combine(Path.GetFullPath(dir), "Pindah Drive.exe")); } catch { }
            Close();
            return;
        }
        tPasang.Text = "Tutup";
        tPasang.Enabled = true;
    }
}
