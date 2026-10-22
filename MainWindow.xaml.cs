using System;
using System.Windows;
using System.Windows.Interop;
using LongPathFix.Services;

namespace LongPathFix;

public partial class MainWindow : Window
{
    public MainWindow()
    {
        InitializeComponent();
        Mdah.Windowing.MaximizeFix.Attach(this);
        Mdah.Windowing.ResponsiveScale.Attach(this, Body);
    }

    protected override void OnSourceInitialized(EventArgs e)
    {
        base.OnSourceInitialized(e);
        NativeMethods.EnableWindows11DarkTheme(new WindowInteropHelper(this).Handle);
    }
}
