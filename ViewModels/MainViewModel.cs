using System.Collections.ObjectModel;
using System.IO;
using System.Text;
using System.Windows;
using System.Windows.Input;
using LongPathFix.Services;
using Microsoft.Win32;

namespace LongPathFix.ViewModels;

public sealed record FindingRow(string Path, string Issues, string NewName, string Unresolved)
{
    public bool HasNew => NewName.Length > 0;
    public bool HasUnresolved => Unresolved.Length > 0;
}

public sealed class MainViewModel : ShellViewModel
{
    public const int RowLimit = 500;
    public static readonly int[] Limits = [259, 240, 200];

    private readonly Renamer _renamer;
    private List<Node> _nodes = new();
    private Plan? _plan;
    private string _root = string.Empty, _statusText = "Ready";
    private int _limitIndex;
    private bool _isBusy, _fixLong = true, _fixIllegal = true, _fixTrailing = true, _fixReserved = true, _fixCase = true;
    private CancellationTokenSource? _cts;

    public MainViewModel() : this(null) { }

    public MainViewModel(Renamer? renamer)
    {
        _renamer = renamer ?? new Renamer();
        BrowseCommand = new RelayCommand(Browse);
        ScanCommand = new RelayCommand(async () => await ScanAsync(), () => !IsBusy);
        ApplyCommand = new RelayCommand(async () => await ApplyAsync(), () => !IsBusy);
        ExportCommand = new RelayCommand(Export);
        UndoCommand = new RelayCommand(p => Undo(p as RenameBatch));
        OpenHistoryFolderCommand = new RelayCommand(() => { Directory.CreateDirectory(_renamer.HistoryFolder); OpenPath(_renamer.HistoryFolder); });
        RefreshHistory();
    }

    public string StatusText { get => _statusText; private set => SetProperty(ref _statusText, value); }
    public bool IsBusy { get => _isBusy; private set { if (SetProperty(ref _isBusy, value)) System.Windows.Input.CommandManager.InvalidateRequerySuggested(); } }
    public string Root { get => _root; set => SetProperty(ref _root, value); }
    public int LimitIndex { get => _limitIndex; set { if (SetProperty(ref _limitIndex, value)) Replan(); } }
    public bool FixLong { get => _fixLong; set { if (SetProperty(ref _fixLong, value)) Replan(); } }
    public bool FixIllegal { get => _fixIllegal; set { if (SetProperty(ref _fixIllegal, value)) Replan(); } }
    public bool FixTrailing { get => _fixTrailing; set { if (SetProperty(ref _fixTrailing, value)) Replan(); } }
    public bool FixReserved { get => _fixReserved; set { if (SetProperty(ref _fixReserved, value)) Replan(); } }
    public bool FixCase { get => _fixCase; set { if (SetProperty(ref _fixCase, value)) Replan(); } }

    public ObservableCollection<FindingRow> Findings { get; } = new();
    public ObservableCollection<RenameBatch> History { get; } = new();

    public ICommand BrowseCommand { get; }
    public ICommand ScanCommand { get; }
    public ICommand ApplyCommand { get; }
    public ICommand ExportCommand { get; }
    public ICommand UndoCommand { get; }
    public ICommand OpenHistoryFolderCommand { get; }

    public bool HasPlan => _plan != null;
    public bool HasOps => _plan is { Ops.Count: > 0 };
    public bool HasHistory => History.Count > 0;
    public bool ShowNothingFound => _plan != null && _plan.Findings.Count == 0;
    public string Summary => _plan == null ? string.Empty
        : _plan.Findings.Count == 0 ? $"{_plan.Scanned:N0} files and folders checked: nothing to fix."
        : $"{_plan.Scanned:N0} files and folders checked: {_plan.Findings.Count:N0} with a problem, {_plan.Ops.Count:N0} renames planned" + (_plan.Unresolved > 0 ? $", {_plan.Unresolved:N0} that renaming cannot fix." : ".");
    public string ShownNote => _plan != null && _plan.Findings.Count > RowLimit ? $"Showing the first {RowLimit} of {_plan.Findings.Count:N0}. Apply and Export use all of them." : string.Empty;
    public bool HasShownNote => ShownNote.Length > 0;

    private FixKinds Kinds => (FixLong ? FixKinds.LongPaths : 0) | (FixIllegal ? FixKinds.IllegalChars : 0) | (FixTrailing ? FixKinds.TrailingDotsAndSpaces : 0)
                            | (FixReserved ? FixKinds.ReservedNames : 0) | (FixCase ? FixKinds.CaseClashes : 0);

    private void Browse()
    {
        var dlg = new OpenFolderDialog { Title = "Choose the folder to check" };
        if (dlg.ShowDialog() == true) { Root = dlg.FolderName; _ = ScanAsync(); }
    }

    private async Task ScanAsync()
    {
        string path = Root.Trim().Trim('"');
        if (path.Length == 0 || !Directory.Exists(path)) { ShowBanner("Choose an existing folder first.", "Warning"); return; }
        _cts?.Cancel();
        _cts = new CancellationTokenSource();
        var ct = _cts.Token;
        IsBusy = true;
        StatusText = "Scanning…";
        try
        {
            _nodes = await Task.Run(() => PathScanner.Scan(path, ct).ToList(), ct);
            Replan();
            StatusText = _plan!.Findings.Count == 0 ? "Nothing to fix" : $"{_plan.Findings.Count:N0} problems, {_plan.Ops.Count:N0} renames planned";
        }
        catch (OperationCanceledException) { StatusText = "Cancelled"; }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException or ArgumentException)
        {
            ShowBanner("Could not scan: " + ex.Message, "Error");
            StatusText = "Could not scan";
        }
        finally { IsBusy = false; }
    }

    private void Replan()
    {
        if (_nodes.Count == 0 && _plan == null && Root.Trim().Length == 0) return;
        _plan = RenamePlanner.Build(Root.Trim().Trim('"'), _nodes, Limits[Math.Clamp(LimitIndex, 0, Limits.Length - 1)], Kinds);
        Findings.Clear();
        foreach (var f in _plan.Findings.Take(RowLimit))
            Findings.Add(new FindingRow(f.Node.Rel + (f.Node.IsDirectory ? "\\" : ""), f.Text, f.NewName ?? string.Empty, f.Unresolved ?? string.Empty));
        foreach (var n in new[] { nameof(HasPlan), nameof(HasOps), nameof(ShowNothingFound), nameof(Summary), nameof(ShownNote), nameof(HasShownNote) }) OnPropertyChanged(n);
    }

    private async Task ApplyAsync()
    {
        if (_plan == null || _plan.Ops.Count == 0) { ShowBanner("There is nothing to rename. Scan a folder first.", "Warning"); return; }
        var answer = MessageBox.Show($"Rename {_plan.Ops.Count:N0} file(s) and folder(s) in\n{Root}?\n\nOnly names change; no file content is touched. Every rename is logged and can be undone from the History page.",
            "LongPathFix", MessageBoxButton.OKCancel, MessageBoxImage.Question);
        if (answer != MessageBoxResult.OK) return;
        IsBusy = true;
        try
        {
            string root = Root.Trim().Trim('"');
            var ops = _plan.Ops;
            var (batch, outcomes) = await Task.Run(() => _renamer.Apply(root, ops));
            int ok = outcomes.Count(o => o.Ok), failed = outcomes.Count - ok;
            ShowBanner($"Renamed {ok:N0}" + (failed > 0 ? $"; {failed:N0} skipped ({outcomes.First(o => !o.Ok).Message})." : "."), failed > 0 ? "Warning" : "Success");
            RefreshHistory();
            _nodes = await Task.Run(() => PathScanner.Scan(root).ToList());
            Replan();
        }
        finally { IsBusy = false; }
    }

    private void Undo(RenameBatch? batch)
    {
        if (batch == null) return;
        var outcomes = _renamer.Undo(batch);
        int ok = outcomes.Count(o => o.Ok), failed = outcomes.Count - ok;
        ShowBanner($"Restored {ok:N0} name(s)" + (failed > 0 ? $"; {failed:N0} skipped ({outcomes.First(o => !o.Ok).Message})." : "."), failed > 0 ? "Warning" : "Success");
        RefreshHistory();
    }

    private void RefreshHistory()
    {
        History.Clear();
        foreach (var b in _renamer.History()) History.Add(b);
        OnPropertyChanged(nameof(HasHistory));
    }

    private void Export()
    {
        if (_plan == null || _plan.Findings.Count == 0) { ShowBanner("There are no findings to export.", "Warning"); return; }
        var dlg = new SaveFileDialog { Title = "Save the findings", FileName = "longpathfix-findings.csv", Filter = "CSV (*.csv)|*.csv" };
        if (dlg.ShowDialog() != true) return;
        var sb = new StringBuilder("Path,Type,Problems,Planned new name,Cannot be fixed because\r\n");
        foreach (var f in _plan.Findings)
            sb.Append(Renamer.Csv(f.Node.Rel)).Append(',').Append(f.Node.IsDirectory ? "folder" : "file").Append(',').Append(Renamer.Csv(f.Text)).Append(',')
              .Append(Renamer.Csv(f.NewName ?? "")).Append(',').Append(Renamer.Csv(f.Unresolved ?? "")).Append("\r\n");
        try { File.WriteAllText(dlg.FileName, sb.ToString(), new UTF8Encoding(true)); ShowBanner("Saved " + Path.GetFileName(dlg.FileName), "Success"); }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException) { ShowBanner("Could not save: " + ex.Message, "Error"); }
    }
}
