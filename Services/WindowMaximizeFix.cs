// SHARED: one file for every Mdah desktop tool. The canonical copy is 08_templates/WindowMaximizeFix.cs; the copy inside each app
// must be byte-identical (checked by 09_tools/validate_catalog.py). Change it here, then copy it.
using System;
using System.Runtime.InteropServices;
using System.Windows;
using System.Windows.Interop;
using System.Windows.Media;

namespace Mdah.Windowing;

/// <summary>
/// A borderless window (WindowStyle=None with a custom title bar) is maximised over the whole monitor and hides the taskbar.
/// This keeps it inside the work area, like a normal Windows 11 window, and sets the corners the same way for every app:
/// rounded while the window is restored, square (and without the thin DWM frame) while it is maximised.
/// Call <see cref="Attach"/> once from the window constructor.
/// </summary>
public static class MaximizeFix
{
    private const int WM_GETMINMAXINFO = 0x0024;
    private const int MONITOR_DEFAULTTONEAREST = 2;

    public static void Attach(Window window)
    {
        window.SourceInitialized += (_, _) => ApplyCorners(window);
        window.StateChanged += (_, _) => ApplyCorners(window);
        window.SourceInitialized += (_, _) =>
        {
            var source = (HwndSource?)PresentationSource.FromVisual(window);
            source?.AddHook((IntPtr hwnd, int msg, IntPtr wParam, IntPtr lParam, ref bool handled) =>
            {
                if (msg != WM_GETMINMAXINFO) return IntPtr.Zero;
                var mmi = Marshal.PtrToStructure<MINMAXINFO>(lParam);
                IntPtr monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
                if (monitor != IntPtr.Zero)
                {
                    var info = new MONITORINFO { cbSize = Marshal.SizeOf<MONITORINFO>() };
                    if (GetMonitorInfo(monitor, ref info))
                    {
                        mmi.ptMaxPosition.x = info.rcWork.left - info.rcMonitor.left;
                        mmi.ptMaxPosition.y = info.rcWork.top - info.rcMonitor.top;
                        mmi.ptMaxSize.x = info.rcWork.right - info.rcWork.left;
                        mmi.ptMaxSize.y = info.rcWork.bottom - info.rcWork.top;
                        var dpi = VisualTreeHelper.GetDpi(window);
                        if (window.MinWidth > 0) mmi.ptMinTrackSize.x = (int)Math.Ceiling(window.MinWidth * dpi.DpiScaleX);
                        if (window.MinHeight > 0) mmi.ptMinTrackSize.y = (int)Math.Ceiling(window.MinHeight * dpi.DpiScaleY);
                        // Without a cap Windows may grow the window beyond the work area (seen on a 2560x1440 monitor next to a 1920x1080 one).
                        mmi.ptMaxTrackSize.x = Math.Max(mmi.ptMaxSize.x, mmi.ptMinTrackSize.x);
                        mmi.ptMaxTrackSize.y = Math.Max(mmi.ptMaxSize.y, mmi.ptMinTrackSize.y);
                        Marshal.StructureToPtr(mmi, lParam, true);
                    }
                }
                handled = true;
                return IntPtr.Zero;
            });
        };
    }

    private const int DWMWA_WINDOW_CORNER_PREFERENCE = 33;
    private const int DWMWA_BORDER_COLOR = 34;
    private const int DWMWCP_DONOTROUND = 1;
    private const int DWMWCP_ROUND = 2;

    /// <summary>Windows 11: round the window corners when restored, square them when maximised. Ignored on older Windows.</summary>
    private static void ApplyCorners(Window window)
    {
        try
        {
            IntPtr hwnd = new WindowInteropHelper(window).Handle;
            if (hwnd == IntPtr.Zero) return;
            bool maximised = window.WindowState == WindowState.Maximized;
            int corner = maximised ? DWMWCP_DONOTROUND : DWMWCP_ROUND;
            DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, ref corner, sizeof(int));
            int border = maximised ? unchecked((int)0xFFFFFFFE) : unchecked((int)0xFFFFFFFF);   // DWMWA_COLOR_NONE / DWMWA_COLOR_DEFAULT
            DwmSetWindowAttribute(hwnd, DWMWA_BORDER_COLOR, ref border, sizeof(int));
        }
        catch { /* cosmetic only */ }
    }

    [DllImport("dwmapi.dll")] private static extern int DwmSetWindowAttribute(IntPtr hwnd, int attribute, ref int value, int size);

    [StructLayout(LayoutKind.Sequential)] private struct POINT { public int x, y; }
    [StructLayout(LayoutKind.Sequential)] private struct RECT { public int left, top, right, bottom; }
    [StructLayout(LayoutKind.Sequential)] private struct MINMAXINFO { public POINT ptReserved, ptMaxSize, ptMaxPosition, ptMinTrackSize, ptMaxTrackSize; }
    [StructLayout(LayoutKind.Sequential)] private struct MONITORINFO { public int cbSize; public RECT rcMonitor, rcWork; public int dwFlags; }

    [DllImport("user32.dll")] private static extern IntPtr MonitorFromWindow(IntPtr hwnd, int flags);
    [DllImport("user32.dll")] private static extern bool GetMonitorInfo(IntPtr monitor, ref MONITORINFO info);
}
