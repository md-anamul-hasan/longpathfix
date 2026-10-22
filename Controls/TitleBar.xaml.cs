using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;

using System.Windows.Media;

namespace LongPathFix.Controls;

public partial class TitleBar : UserControl
{
    public TitleBar()
    {
        InitializeComponent();
        Loaded += (s, e) =>
        {
            var win = Window.GetWindow(this);
            if (win != null)
            {
                win.StateChanged += (ws, we) => UpdateMaximizeIcon(win.WindowState);
                UpdateMaximizeIcon(win.WindowState);
            }
        };
    }

    private void UpdateMaximizeIcon(WindowState state)
    {
        if (state == WindowState.Maximized)
        {
            // Windows 11 Restore Down icon (overlapping rectangles)
            MaximizePath.Data = Geometry.Parse("M 2.5,0.5 H 9.5 V 7.5 H 7.5 V 9.5 H 0.5 V 2.5 H 2.5 Z M 2.5,2.5 H 7.5 V 7.5");
            MaximizeButton.ToolTip = "Restore Down";
        }
        else
        {
            // Windows 11 Maximize icon (single rectangle)
            MaximizePath.Data = Geometry.Parse("M 0.5,0.5 H 9.5 V 9.5 H 0.5 Z");
            MaximizeButton.ToolTip = "Maximize";
        }
    }

    private void OnTitleBarMouseDown(object sender, MouseButtonEventArgs e)
    {
        if (e.ClickCount == 2)
        {
            ToggleMaximize();
        }
        else if (e.LeftButton == MouseButtonState.Pressed)
        {
            Window.GetWindow(this)?.DragMove();
        }
    }

    private void MinimizeButton_Click(object sender, RoutedEventArgs e)
    {
        var window = Window.GetWindow(this);
        if (window != null)
        {
            window.WindowState = WindowState.Minimized;
        }
    }

    private void MaximizeButton_Click(object sender, RoutedEventArgs e)
    {
        ToggleMaximize();
    }

    private void CloseButton_Click(object sender, RoutedEventArgs e)
    {
        Window.GetWindow(this)?.Close();
    }

    private void ToggleMaximize()
    {
        var window = Window.GetWindow(this);
        if (window != null)
        {
            window.WindowState = window.WindowState == WindowState.Maximized
                ? WindowState.Normal
                : WindowState.Maximized;
        }
    }
}
