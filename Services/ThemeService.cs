// SHARED: one file for every Mdah desktop tool. The canonical copy is 08_templates/ThemeService.cs; the copy inside each app
// must be byte-identical (checked by 09_tools/validate_catalog.py). Change it here, then copy it.
// Same behaviour as WRSD's theme switch (System / Dark / Light, remembered per app in HKCU\Software\<App>).
using System;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Interop;
using System.Windows.Media;
using System.Runtime.InteropServices;
using Microsoft.Win32;

namespace Mdah.Theming;

public enum ThemeMode { System = 0, Dark = 1, Light = 2 }

/// <summary>
/// Dark / Light / System theme for the shared design tokens. The two token files (Themes/DesignTokens.xaml = dark,
/// Themes/DesignTokens.Light.xaml = light) define the same keys; switching swaps the dictionary, and every
/// DynamicResource in the app follows. Call <see cref="Initialize"/> in App.OnStartup before the first window is created.
/// </summary>
public static class Theme
{
    private const string DarkSource = "Themes/DesignTokens.xaml";
    private const string LightSource = "Themes/DesignTokens.Light.xaml";
    private const string Personalize = @"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";

    private static string _app = "Mdah";
    private static bool _hooked;

    public static event Action<ThemeMode, bool>? Changed;

    public static ThemeMode Mode { get; private set; } = ThemeMode.System;
    public static bool IsDark { get; private set; } = true;

    public static void Initialize(string appName)
    {
        _app = appName;
        Mode = Load();
        if (!_hooked)
        {
            _hooked = true;
            EventManager.RegisterClassHandler(typeof(Window), FrameworkElement.LoadedEvent,
                new RoutedEventHandler((s, _) => { if (s is Window w) StyleWindow(w); }));
            try
            {
                SystemEvents.UserPreferenceChanged += (_, _) =>
                {
                    if (Mode == ThemeMode.System) Application.Current?.Dispatcher.Invoke(Apply);
                };
            }
            catch { /* system events are optional */ }
        }
        Apply();
    }

    public static void Set(ThemeMode mode)
    {
        Mode = mode;
        Save(mode);
        Apply();
    }

    /// <summary>System, then Dark, then Light, then System again (same order as WRSD).</summary>
    public static ThemeMode Cycle()
    {
        var next = Mode switch { ThemeMode.System => ThemeMode.Dark, ThemeMode.Dark => ThemeMode.Light, _ => ThemeMode.System };
        Set(next);
        return next;
    }

    public static bool SystemIsDark()
    {
        try
        {
            using var key = Registry.CurrentUser.OpenSubKey(Personalize);
            if (key?.GetValue("AppsUseLightTheme") is int light) return light == 0;
        }
        catch { /* default to dark */ }
        return true;
    }

    public static void Apply()
    {
        IsDark = Mode switch { ThemeMode.Dark => true, ThemeMode.Light => false, _ => SystemIsDark() };
        var app = Application.Current;
        if (app != null)
        {
            var merged = app.Resources.MergedDictionaries;
            var fresh = new ResourceDictionary { Source = new Uri(IsDark ? DarkSource : LightSource, UriKind.Relative) };
            int index = -1;
            for (int i = 0; i < merged.Count; i++)
            {
                string? src = merged[i].Source?.OriginalString;
                if (src != null && (src.EndsWith("DesignTokens.xaml", StringComparison.OrdinalIgnoreCase) ||
                                    src.EndsWith("DesignTokens.Light.xaml", StringComparison.OrdinalIgnoreCase))) { index = i; break; }
            }
            if (index >= 0) merged[index] = fresh; else merged.Insert(0, fresh);
            foreach (Window w in app.Windows) StyleWindow(w);
        }
        Changed?.Invoke(Mode, IsDark);
    }

    /// <summary>Dark or light Windows title/caption area to match the theme (Windows 10 2004+ / Windows 11).</summary>
    public static void StyleWindow(Window window)
    {
        try
        {
            IntPtr hwnd = new WindowInteropHelper(window).Handle;
            if (hwnd == IntPtr.Zero) return;
            int dark = IsDark ? 1 : 0;
            if (DwmSetWindowAttribute(hwnd, 20, ref dark, sizeof(int)) != 0) DwmSetWindowAttribute(hwnd, 19, ref dark, sizeof(int));
        }
        catch { /* cosmetic only */ }
    }

    public static string Label(ThemeMode mode) => mode switch { ThemeMode.Dark => "Dark", ThemeMode.Light => "Light", _ => "System" };

    private static ThemeMode Load()
    {
        try
        {
            using var key = Registry.CurrentUser.OpenSubKey(@"Software\" + _app);
            if (key?.GetValue("ThemeMode") is string s && Enum.TryParse<ThemeMode>(s, out var mode)) return mode;
        }
        catch { /* default */ }
        return ThemeMode.System;
    }

    private static void Save(ThemeMode mode)
    {
        try
        {
            using var key = Registry.CurrentUser.CreateSubKey(@"Software\" + _app);
            key?.SetValue("ThemeMode", mode.ToString());
        }
        catch { /* the choice still applies for this session */ }
    }

    [DllImport("dwmapi.dll")] private static extern int DwmSetWindowAttribute(IntPtr hwnd, int attribute, ref int value, int size);
}

/// <summary>
/// The same "Theme: System / Dark / Light" button in every app (sidebar footer, in the sidebar footer). One click cycles the theme.
/// </summary>
public sealed class ThemeToggleButton : Button
{
    private readonly TextBlock _glyph = new() { FontFamily = new FontFamily("Segoe Fluent Icons, Segoe MDL2 Assets"), FontSize = 13, Margin = new Thickness(0, 0, 8, 0), VerticalAlignment = VerticalAlignment.Center };
    private readonly TextBlock _label = new() { FontSize = 12, FontWeight = FontWeights.SemiBold, VerticalAlignment = VerticalAlignment.Center };

    public ThemeToggleButton()
    {
        HorizontalContentAlignment = HorizontalAlignment.Left;
        Padding = new Thickness(12, 0, 12, 0);
        Height = 34;
        Cursor = System.Windows.Input.Cursors.Hand;
        _glyph.SetResourceReference(TextBlock.ForegroundProperty, "TextSecondary");
        _label.SetResourceReference(TextBlock.ForegroundProperty, "TextPrimary");
        var row = new StackPanel { Orientation = Orientation.Horizontal };
        row.Children.Add(_glyph);
        row.Children.Add(_label);
        Content = row;
        Loaded += (_, _) =>
        {
            if (TryFindResource("SecondaryButton") is Style style) Style = style;
            Refresh();
            Theme.Changed += OnChanged;
        };
        Unloaded += (_, _) => Theme.Changed -= OnChanged;
        Click += (_, _) => Theme.Cycle();
    }

    private void OnChanged(ThemeMode mode, bool dark) => Dispatcher.Invoke(Refresh);

    private void Refresh()
    {
        _glyph.Text = Theme.Mode switch { ThemeMode.Dark => "", ThemeMode.Light => "", _ => "" };
        _label.Text = "Theme: " + Theme.Label(Theme.Mode);
        ToolTip = Theme.Mode == ThemeMode.System
            ? $"Theme follows Windows (now {(Theme.IsDark ? "dark" : "light")}). Click to switch to Dark."
            : Theme.Mode == ThemeMode.Dark ? "Dark theme. Click to switch to Light." : "Light theme. Click to follow Windows.";
    }
}
