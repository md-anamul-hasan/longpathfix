using System.IO;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace LongPathFix.Services;

/// <summary>One rename that was done: <c>Dir\OldName</c> became <c>Dir\NewName</c> (Dir is the folder as it was when the rename happened).</summary>
public sealed record DoneRename(string Dir, string OldName, string NewName, bool IsDirectory);

public sealed class RenameBatch
{
    public string Id { get; set; } = string.Empty;
    public DateTime Time { get; set; }
    public string Root { get; set; } = string.Empty;
    public List<DoneRename> Done { get; set; } = new();     // in the order they were done
    public bool Undone { get; set; }
    [JsonIgnore] public string Title => $"{Time:yyyy-MM-dd HH:mm}  ·  {Done.Count} rename(s)  ·  {Root}" + (Undone ? "  ·  undone" : string.Empty);
}

public sealed record RenameOutcome(string Path, bool Ok, string Message);

/// <summary>
/// Applies a rename plan and undoes it. Renames are done deepest first (so every folder still has its old path when its content is
/// renamed) and undone in the opposite order. Only names change: a rename moves nothing and never touches file content.
/// </summary>
public sealed class Renamer
{
    private readonly string _history;

    public Renamer(string? historyFolder = null) =>
        _history = historyFolder ?? Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "LongPathFix", "history");

    public string HistoryFolder => _history;

    public (RenameBatch? Batch, List<RenameOutcome> Outcomes) Apply(string root, IEnumerable<RenameOp> ops)
    {
        var outcomes = new List<RenameOutcome>();
        string plainRoot = Path.GetFullPath(PathScanner.Plain(root)).TrimEnd('\\');
        var batch = new RenameBatch { Id = DateTime.Now.ToString("yyyyMMdd-HHmmss-fff"), Time = DateTime.Now, Root = plainRoot };
        Directory.CreateDirectory(_history);
        Save(batch);                                         // the log exists before the first rename

        foreach (var op in ops.OrderByDescending(o => o.Rel.Count(c => c == '\\')).ThenBy(o => o.Rel, StringComparer.Ordinal))
        {
            string dir = op.Parent.Length == 0 ? plainRoot : plainRoot + "\\" + op.Parent;
            string from = PathScanner.Extended(dir + "\\" + op.OldName), to = PathScanner.Extended(dir + "\\" + op.NewName);
            try
            {
                if (!(op.IsDirectory ? Directory.Exists(from) : File.Exists(from))) { outcomes.Add(new RenameOutcome(dir + "\\" + op.OldName, false, "Skipped: it no longer exists")); continue; }
                if (File.Exists(to) || Directory.Exists(to)) { outcomes.Add(new RenameOutcome(dir + "\\" + op.OldName, false, "Skipped: a file or folder named " + op.NewName + " already exists")); continue; }
                if (op.IsDirectory) Directory.Move(from, to); else File.Move(from, to);
                batch.Done.Add(new DoneRename(dir, op.OldName, op.NewName, op.IsDirectory));
                Save(batch);
                outcomes.Add(new RenameOutcome(dir + "\\" + op.OldName, true, "Renamed to " + op.NewName));
            }
            catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
            {
                outcomes.Add(new RenameOutcome(dir + "\\" + op.OldName, false, "Not renamed: " + ex.Message));
            }
        }
        if (batch.Done.Count == 0) { Delete(batch); return (null, outcomes); }
        AppendLog(batch);
        return (batch, outcomes);
    }

    public List<RenameOutcome> Undo(RenameBatch batch)
    {
        var outcomes = new List<RenameOutcome>();
        foreach (var d in Enumerable.Reverse(batch.Done))
        {
            string from = PathScanner.Extended(d.Dir + "\\" + d.NewName), to = PathScanner.Extended(d.Dir + "\\" + d.OldName);
            try
            {
                if (!(d.IsDirectory ? Directory.Exists(from) : File.Exists(from))) { outcomes.Add(new RenameOutcome(d.Dir + "\\" + d.NewName, false, "Skipped: it was renamed or removed since")); continue; }
                if (File.Exists(to) || Directory.Exists(to)) { outcomes.Add(new RenameOutcome(d.Dir + "\\" + d.NewName, false, "Skipped: the old name is taken again")); continue; }
                if (d.IsDirectory) Directory.Move(from, to); else File.Move(from, to);
                outcomes.Add(new RenameOutcome(d.Dir + "\\" + d.OldName, true, "Restored the old name"));
            }
            catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
            {
                outcomes.Add(new RenameOutcome(d.Dir + "\\" + d.NewName, false, "Not restored: " + ex.Message));
            }
        }
        batch.Undone = outcomes.All(o => o.Ok);
        Save(batch);
        return outcomes;
    }

    public IReadOnlyList<RenameBatch> History()
    {
        var list = new List<RenameBatch>();
        if (!Directory.Exists(_history)) return list;
        foreach (var f in Directory.GetFiles(_history, "*.json").OrderByDescending(x => x, StringComparer.Ordinal))
        {
            try { var b = JsonSerializer.Deserialize<RenameBatch>(File.ReadAllText(f)); if (b != null) list.Add(b); }
            catch (Exception ex) when (ex is IOException or JsonException) { /* skip a damaged file */ }
        }
        return list;
    }

    private void Save(RenameBatch b) =>
        File.WriteAllText(Path.Combine(_history, b.Id + ".json"), JsonSerializer.Serialize(b, new JsonSerializerOptions { WriteIndented = true }));

    private void Delete(RenameBatch b) { try { File.Delete(Path.Combine(_history, b.Id + ".json")); } catch (IOException) { } }

    private void AppendLog(RenameBatch b)
    {
        try
        {
            string log = Path.Combine(Path.GetDirectoryName(_history)!, "rename-log.csv");
            var sb = new StringBuilder();
            if (!File.Exists(log)) sb.Append("Time,Folder,Old name,New name\r\n");
            foreach (var d in b.Done) sb.Append(b.Time.ToString("yyyy-MM-dd HH:mm:ss")).Append(',').Append(Csv(d.Dir)).Append(',').Append(Csv(d.OldName)).Append(',').Append(Csv(d.NewName)).Append("\r\n");
            File.AppendAllText(log, sb.ToString(), new UTF8Encoding(true));
        }
        catch (IOException) { /* the batch file is what undo uses */ }
    }

    public static string Csv(string s) => s.IndexOfAny([',', '"', '\r', '\n']) >= 0 ? "\"" + s.Replace("\"", "\"\"") + "\"" : s;
}
