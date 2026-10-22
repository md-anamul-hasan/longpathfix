// SHARED: one file for every Mdah desktop tool. The canonical copy is 08_templates/ResponsiveScale.cs; the copy inside each app
// must be byte-identical (checked by 09_tools/validate_catalog.py). Change it here, then copy it.
using System;
using System.Windows;
using System.Windows.Media;

namespace Mdah.Windowing;

/// <summary>
/// Keeps the layout readable on large and high-resolution screens (27" 2560x1440, 4K): below the reference size the layout
/// is untouched, above it the content area is scaled up smoothly (never more than <see cref="MaxScale"/>) so text and
/// controls grow with the window instead of floating in an empty canvas. The title bar is left alone (its drag area is
/// fixed by WindowChrome). Call <see cref="Attach"/> once from the window constructor with the element below the title bar.
/// </summary>
public static class ResponsiveScale
{
    /// <summary>Window size (device-independent pixels) at which the layout is shown at 100 %.</summary>
    public const double ReferenceWidth = 1600;
    public const double ReferenceHeight = 900;
    public const double MaxScale = 1.5;

    /// <summary>Scale for a window of this size: 1.0 up to the reference size, then proportional, in 5 % steps.</summary>
    public static double ScaleFor(double width, double height)
    {
        if (width <= 0 || height <= 0) return 1.0;
        double raw = Math.Min(width / ReferenceWidth, height / ReferenceHeight);
        double clamped = Math.Min(MaxScale, Math.Max(1.0, raw));
        return Math.Floor(clamped * 20) / 20;
    }

    /// <param name="includesTitleBar">True when <paramref name="body"/> also contains the custom title bar: the drag area of the
    /// window chrome then grows with the scale so the title bar stays fully draggable.</param>
    public static void Attach(Window window, FrameworkElement body, bool includesTitleBar = false)
    {
        var transform = new ScaleTransform(1, 1);
        body.LayoutTransform = transform;
        var chrome = includesTitleBar ? System.Windows.Shell.WindowChrome.GetWindowChrome(window) : null;
        double captionHeight = chrome?.CaptionHeight ?? 0;
        void Update()
        {
            double s = ScaleFor(window.ActualWidth, window.ActualHeight);
            if (Math.Abs(transform.ScaleX - s) <= 0.001) return;
            transform.ScaleX = s;
            transform.ScaleY = s;
            if (chrome != null) chrome.CaptionHeight = captionHeight * s;
        }
        window.SizeChanged += (_, _) => Update();
        window.Loaded += (_, _) => Update();
    }
}
