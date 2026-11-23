// Peluncur "Pindah Drive.exe": membuka gui.ps1 tanpa jendela terminal.
//
// Dikompilasi oleh buat-aplikasi.ps1 memakai csc.exe bawaan .NET Framework.
// Sengaja tidak memakai shortcut "powershell -WindowStyle Hidden": di Windows 11
// dengan Windows Terminal sebagai terminal bawaan, cara itu tetap memunculkan
// jendela terminal. CreateNoWindow mencegah konsol dibuat sama sekali.

using System;
using System.Diagnostics;
using System.IO;
using System.Reflection;
using System.Windows.Forms;

[assembly: AssemblyTitle("Pindah Drive")]
[assembly: AssemblyProduct("Pindah Drive")]
[assembly: AssemblyDescription("Salin backup dari OneDrive ke Google Drive")]
[assembly: AssemblyVersion("1.0.0.0")]
[assembly: AssemblyFileVersion("1.0.0.0")]

static class Peluncur
{
    [STAThread]
    static int Main()
    {
        string akar = AppDomain.CurrentDomain.BaseDirectory;
        string gui = Path.Combine(akar, "gui.ps1");
        if (!File.Exists(gui))
        {
            MessageBox.Show("File gui.ps1 tidak ditemukan di:\n" + akar, "Pindah Drive",
                MessageBoxButtons.OK, MessageBoxIcon.Error);
            return 1;
        }

        string powershell = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.System),
            @"WindowsPowerShell\v1.0\powershell.exe");

        ProcessStartInfo psi = new ProcessStartInfo(powershell,
            "-NoProfile -ExecutionPolicy Bypass -STA -WindowStyle Hidden -File \"" + gui + "\"");
        psi.UseShellExecute = false;
        psi.CreateNoWindow = true;
        psi.WorkingDirectory = akar;

        try
        {
            Process.Start(psi);
        }
        catch (Exception e)
        {
            MessageBox.Show("Pindah Drive gagal dibuka:\n" + e.Message, "Pindah Drive",
                MessageBoxButtons.OK, MessageBoxIcon.Error);
            return 1;
        }
        return 0;
    }
}
