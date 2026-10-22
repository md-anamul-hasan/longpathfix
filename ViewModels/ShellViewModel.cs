using System;
using System.Diagnostics;
using System.Reflection;
using System.Threading.Tasks;
using System.Windows.Input;
using System.Windows.Threading;
using LongPathFix.Services;

namespace LongPathFix.ViewModels;

/// <summary>Navigation and toast banner.</summary>
public abstract class ShellViewModel : ViewModelBase
{
    private readonly DispatcherTimer _toastTimer;

    private int _selectedTabIndex;
    private string _statusBannerMessage = string.Empty;
    private string _statusBannerType = "Info";
    private bool _isStatusBannerVisible;

    protected ShellViewModel()
    {
        OpenRepositoryCommand = new RelayCommand(() => OpenUrl(RepositoryUrl));
        DismissBannerCommand = new RelayCommand(() =>
        {
            _toastTimer?.Stop();
            StatusBannerMessage = string.Empty;
            IsStatusBannerVisible = false;
        });

        _toastTimer = new DispatcherTimer { Interval = TimeSpan.FromSeconds(7) };
        _toastTimer.Tick += (s, e) =>
        {
            _toastTimer.Stop();
            IsStatusBannerVisible = false;
        };
    }

    public const string RepositoryUrl = "https://github.com/md-anamul-hasan/longpathfix";

    public string AppVersion { get; } =
        Assembly.GetExecutingAssembly().GetCustomAttribute<AssemblyInformationalVersionAttribute>()?.InformationalVersion.Split('+')[0]
        ?? Assembly.GetExecutingAssembly().GetName().Version?.ToString(3) ?? "1.0.0";

    public int SelectedTabIndex
    {
        get => _selectedTabIndex;
        set => SetProperty(ref _selectedTabIndex, value);
    }

    public ICommand DismissBannerCommand { get; }

    public ICommand OpenRepositoryCommand { get; }

    // ── Toast banner ────────────────────────────────────────────
    public string StatusBannerMessage
    {
        get => _statusBannerMessage;
        set
        {
            if (SetProperty(ref _statusBannerMessage, value))
                OnPropertyChanged(nameof(IsStatusBannerVisible));
        }
    }

    public string StatusBannerType
    {
        get => _statusBannerType;
        set => SetProperty(ref _statusBannerType, value);
    }

    public bool IsStatusBannerVisible
    {
        get => _isStatusBannerVisible && !string.IsNullOrWhiteSpace(_statusBannerMessage);
        set
        {
            if (SetProperty(ref _isStatusBannerVisible, value))
            {
                if (!value) _statusBannerMessage = string.Empty;
            }
        }
    }

    protected void ShowBanner(string message, string type = "Info")
    {
        StatusBannerType = type;
        StatusBannerMessage = message;
        IsStatusBannerVisible = true;
        _toastTimer.Stop();
        _toastTimer.Start();
    }

    protected static void OpenUrl(string url)
    {
        try { Process.Start(new ProcessStartInfo { FileName = url, UseShellExecute = true }); } catch { /* no browser */ }
    }

    protected static void OpenPath(string path)
    {
        try { Process.Start(new ProcessStartInfo { FileName = path, UseShellExecute = true }); } catch { /* nothing to open it with */ }
    }
}
