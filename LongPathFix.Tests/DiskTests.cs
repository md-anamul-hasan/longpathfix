using System.IO;
using System.Security.Cryptography;
using LongPathFix.Services;

namespace LongPathFix.Tests;

/// <summary>The same plan and undo on a real folder, including names and path lengths that Windows normally refuses.</summary>
public class DiskTests : IDisposable
{
    private readonly string _root = Path.Combine(Path.GetTempPath(), "lpf-" + Guid.NewGuid().ToString("N")[..8]);
    private readonly string _hist = Path.Combine(Path.GetTempPath(), "lpf-hist-" + Guid.NewGuid().ToString("N")[..8], "history");

    public DiskTests() => Directory.CreateDirectory(_root);

    public void Dispose()
    {
        try { Directory.Delete(PathScanner.Extended(_root), true); } catch { /* temp */ }
        try { Directory.Delete(Path.GetDirectoryName(_hist)!, true); } catch { /* temp */ }
    }

    private static string Ext(string p) => PathScanner.Extended(p);

    private void Make(string rel, string content)
    {
        string full = Ext(Path.Combine(_root, rel));
        Directory.CreateDirectory(Path.GetDirectoryName(full)!);
        File.WriteAllText(full, content);
    }

    /// <summary>Every file below the root as relative path → SHA-256 of its content.</summary>
    private Dictionary<string, string> Snapshot() =>
        PathScanner.Scan(_root).Where(n => !n.IsDirectory).ToDictionary(n => n.Rel,
            n => Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(Ext(Path.Combine(_root, n.Rel))))));

    private static string Hash(string s) => Convert.ToHexString(SHA256.HashData(System.Text.Encoding.UTF8.GetBytes(s)));

    [Fact]
    public void ScanFindsTrailingDotsReservedNamesAndOverlongPaths()
    {
        Make("trail.", "t");
        Make("space ", "s");
        Make("CON.txt", "c");
        string deep = string.Join('\\', Enumerable.Repeat(new string('d', 45), 7)) + "\\" + new string('f', 40) + ".bin";      // about 350 characters
        Make(deep, "deep");
        var plan = RenamePlanner.Build(_root, PathScanner.Scan(_root));
        Assert.Contains(plan.Findings, f => f.Node.Rel == "trail." && f.Issues.HasFlag(Issue.TrailingDotOrSpace));
        Assert.Contains(plan.Findings, f => f.Node.Rel == "space " && f.Issues.HasFlag(Issue.TrailingDotOrSpace));
        Assert.Contains(plan.Findings, f => f.Node.Rel == "CON.txt" && f.Issues.HasFlag(Issue.ReservedName));
        Assert.Contains(plan.Findings, f => f.Issues.HasFlag(Issue.TooLong));
        Assert.True(deep.Length + _root.Length > 300);
    }

    [Fact]
    public void ApplyRenamesEverythingKeepsContentAndUndoRestoresEveryName()
    {
        Make("trail.", "one");
        Make("space ", "two");
        Make("CON.txt", "three");
        Make("Dir.\\inner ", "four");
        Make("Dir.\\ok.txt", "five");
        string deep = string.Join('\\', Enumerable.Repeat(new string('d', 45), 7)) + "\\" + new string('f', 40) + ".bin";
        Make(deep, "six");
        Make("fine.txt", "seven");

        var before = Snapshot();
        var namesBefore = PathScanner.Scan(_root).Select(n => n.Rel).OrderBy(x => x, StringComparer.Ordinal).ToList();
        var plan = RenamePlanner.Build(_root, PathScanner.Scan(_root));
        Assert.NotEmpty(plan.Ops);

        var renamer = new Renamer(_hist);
        var (batch, outcomes) = renamer.Apply(_root, plan.Ops);
        Assert.NotNull(batch);
        Assert.All(outcomes, o => Assert.True(o.Ok, o.Message));

        // after: nothing left to fix, content identical, names changed
        var again = RenamePlanner.Build(_root, PathScanner.Scan(_root));
        Assert.Empty(again.Ops);
        Assert.Empty(again.Findings);
        var after = Snapshot();
        Assert.Equal(before.Values.OrderBy(x => x), after.Values.OrderBy(x => x));       // same contents, whatever the names
        Assert.Equal(before.Count, after.Count);
        Assert.Contains("fine.txt", after.Keys);                                          // untouched
        Assert.DoesNotContain("trail.", after.Keys);
        Assert.Contains("trail", after.Keys);
        Assert.Contains("CON_.txt", after.Keys);
        Assert.Contains(@"Dir\inner", after.Keys);
        Assert.All(after.Keys, k => Assert.True(_root.Length + 1 + k.Length <= 259, k));

        // history and log
        Assert.Equal(plan.Ops.Count, renamer.History().Single().Done.Count);
        Assert.True(File.Exists(Path.Combine(Path.GetDirectoryName(_hist)!, "rename-log.csv")));

        // undo
        var undo = renamer.Undo(renamer.History().Single());
        Assert.All(undo, o => Assert.True(o.Ok, o.Message));
        Assert.Equal(namesBefore, PathScanner.Scan(_root).Select(n => n.Rel).OrderBy(x => x, StringComparer.Ordinal).ToList());
        Assert.Equal(before, Snapshot());
        Assert.True(renamer.History().Single().Undone);
    }

    [Fact]
    public void RenameIsSkippedWhenTheNewNameAlreadyExists()
    {
        Make("a.", "dotted");
        Make("a", "plain");
        var plan = RenamePlanner.Build(_root, PathScanner.Scan(_root));
        // the planner avoids the clash itself: the dotted name becomes "a (2)"? only when case clashes apply; force the clash instead
        var ops = new[] { new RenameOp("a.", "a.", "a", Issue.TrailingDotOrSpace, false) };
        var (batch, outcomes) = new Renamer(_hist).Apply(_root, ops);
        Assert.Null(batch);
        Assert.Contains("already exists", outcomes.Single().Message);
        Assert.Equal("plain", File.ReadAllText(Ext(Path.Combine(_root, "a"))));
        Assert.Equal("dotted", File.ReadAllText(Ext(Path.Combine(_root, "a."))));
        Assert.NotNull(plan);
    }

    [Fact]
    public void PlannerAvoidsTakingAnExistingName()
    {
        Make("a.", "dotted");
        Make("a", "plain");
        var plan = RenamePlanner.Build(_root, PathScanner.Scan(_root));
        var (_, outcomes) = new Renamer(_hist).Apply(_root, plan.Ops);
        Assert.All(outcomes, o => Assert.True(o.Ok, o.Message));
        var snap = Snapshot();
        Assert.Equal(2, snap.Count);
        Assert.Equal("plain", File.ReadAllText(Ext(Path.Combine(_root, "a"))));
        Assert.Equal("dotted", File.ReadAllText(Ext(Path.Combine(_root, "a (2)"))));
    }

    [Fact]
    public void UndoSkipsWhatChangedAndReportsIt()
    {
        Make("x.", "1");
        var plan = RenamePlanner.Build(_root, PathScanner.Scan(_root));
        var renamer = new Renamer(_hist);
        renamer.Apply(_root, plan.Ops);
        File.Move(Ext(Path.Combine(_root, "x")), Ext(Path.Combine(_root, "moved by user")));
        var undo = renamer.Undo(renamer.History().Single());
        Assert.False(undo.Single().Ok);
        Assert.Contains("renamed or removed", undo.Single().Message);
        Assert.False(renamer.History().Single().Undone);
    }

    [Fact]
    public void NothingToRenameWritesNoHistory()
    {
        var (batch, outcomes) = new Renamer(_hist).Apply(_root, []);
        Assert.Null(batch);
        Assert.Empty(outcomes);
        Assert.Empty(new Renamer(_hist).History());
    }

    [Fact]
    public void ScanDoesNotEnterJunctions()
    {
        string outside = Path.Combine(Path.GetTempPath(), "lpf-out-" + Guid.NewGuid().ToString("N")[..8]);
        Directory.CreateDirectory(outside);
        try
        {
            File.WriteAllText(Path.Combine(outside, "bad."), "x");
            var psi = new System.Diagnostics.ProcessStartInfo("cmd.exe", $"/c mklink /J \"{Path.Combine(_root, "jump")}\" \"{outside}\"") { CreateNoWindow = true, UseShellExecute = false, RedirectStandardOutput = true };
            using (var p = System.Diagnostics.Process.Start(psi)!) { p.WaitForExit(); if (p.ExitCode != 0) return; }
            var rels = PathScanner.Scan(_root).Select(n => n.Rel).ToList();
            Assert.Contains("jump", rels);
            Assert.DoesNotContain(rels, r => r.StartsWith("jump\\", StringComparison.Ordinal));
        }
        finally { try { Directory.Delete(outside, true); } catch { } }
    }

    [Fact]
    public void PlainAndExtendedPathsConvertBothWays()
    {
        Assert.Equal(@"\\?\C:\a\b", PathScanner.Extended(@"C:\a\b"));
        Assert.Equal(@"\\?\UNC\srv\share\x", PathScanner.Extended(@"\\srv\share\x"));
        Assert.Equal(@"C:\a\b", PathScanner.Plain(@"\\?\C:\a\b"));
        Assert.Equal(@"\\srv\share\x", PathScanner.Plain(@"\\?\UNC\srv\share\x"));
        Assert.Equal(@"\\?\C:\a", PathScanner.Extended(@"\\?\C:\a"));
    }
}
