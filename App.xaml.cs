using System.Threading.Tasks;
using System.Windows;

namespace LongPathFix;

public partial class App : Application
{
    protected override void OnStartup(StartupEventArgs e)
    {
        Mdah.Theming.Theme.Initialize("LongPathFix");
        base.OnStartup(e);

        DispatcherUnhandledException += (s, args) =>
        {
            LogCrash(args.Exception);
            MessageBox.Show(
                "LongPathFix hit an unexpected problem but is still running.\n\n" +
                "Details were saved to %AppData%\\LongPathFix\\error.log. If it keeps happening, email contact@mdanamulhasan.com and attach that file.",
                "LongPathFix", MessageBoxButton.OK, MessageBoxImage.Warning);
            args.Handled = true;
        };

        TaskScheduler.UnobservedTaskException += (s, args) => args.SetObserved();
    }

    private static void LogCrash(Exception ex)
    {
        try
        {
            string dir = System.IO.Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "LongPathFix");
            System.IO.Directory.CreateDirectory(dir);
            System.IO.File.AppendAllText(System.IO.Path.Combine(dir, "error.log"), $"[{DateTime.Now:u}] {ex}\n\n");
        }
        catch { /* logging must never crash the app */ }
    }
}
