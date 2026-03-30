using System.Diagnostics;
using System.Runtime.InteropServices;

string? javaw = FindJavaw();

if (javaw is null)
{
    MessageBoxW(0, "Java not found.\nPlease make sure Java is installed and configured in PATH or JAVA_HOME.", "Lineage2 Editor", 0x10);
    return;
}

string workDir = Path.GetDirectoryName(Environment.ProcessPath)!;

Process.Start(new ProcessStartInfo
{
    FileName = javaw,
    Arguments = "-splash:images/splash.png -Dfile.encoding=UTF-8 -Xms1G -Xmx4G -cp \"./lib/*\" com.majestic.studio.Boot",
    WorkingDirectory = workDir,
    UseShellExecute = false,
    CreateNoWindow = true,
});

static string? FindJavaw()
{
    string? javaHome = Environment.GetEnvironmentVariable("JAVA_HOME");
    if (javaHome is not null)
    {
        string candidate = Path.Combine(javaHome, "bin", "javaw.exe");
        if (File.Exists(candidate))
            return candidate;
    }

    string? pathEnv = Environment.GetEnvironmentVariable("PATH");
    if (pathEnv is not null)
    {
        foreach (string dir in pathEnv.Split(Path.PathSeparator))
        {
            string candidate = Path.Combine(dir, "javaw.exe");
            if (File.Exists(candidate))
                return candidate;
        }
    }

    return null;
}

[DllImport("user32.dll", CharSet = CharSet.Unicode)]
static extern int MessageBoxW(nint hWnd, string text, string caption, uint type);
