using System.Security.Cryptography;
using LongPathFix.Services;

namespace LongPathFix.Tests;

public class PlannerTests
{
    private const string Root = @"C:\data";           // 7 characters

    private static Node F(string rel) => new(rel, false);
    private static Node D(string rel) => new(rel, true);

    private static Plan Run(IEnumerable<Node> nodes, int limit = 259, FixKinds kinds = FixKinds.All) => RenamePlanner.Build(Root, nodes, limit, kinds);

    /// <summary>The tree as it would look after the plan was applied.</summary>
    private static List<string> After(IEnumerable<Node> nodes, Plan plan)
    {
        var ops = plan.Ops.ToDictionary(o => o.Rel);
        var newName = nodes.ToDictionary(n => n.Rel, n => ops.TryGetValue(n.Rel, out var o) ? o.NewName : n.Name);
        string Rel(string rel)
        {
            var parts = new List<string>();
            for (string c = rel; c.Length > 0; c = c.Contains('\\') ? c[..c.LastIndexOf('\\')] : "") parts.Add(newName[c]);
            parts.Reverse();
            return string.Join('\\', parts);
        }
        return nodes.Select(n => Rel(n.Rel)).ToList();
    }

    private static Plan Single(string name, FixKinds k = FixKinds.All) => Run([F(name)], kinds: k);
    private static string NewNameOf(Plan p, string rel) => p.Ops.Single(o => o.Rel == rel).NewName;

    // ── Names ────────────────────────────────────────────────────

    [Fact]
    public void CleanTreeHasNoFindings()
    {
        var p = Run([D("src"), F("src\\main.c"), F("readme.md"), F("a b (1).txt"), F("CON1.txt"), F("console.log"), F("COM10.txt")]);
        Assert.Empty(p.Findings);
        Assert.Empty(p.Ops);
        Assert.Equal(7, p.Scanned);
    }

    [Theory]
    [InlineData("report<final>.txt", "report_final_.txt")]
    [InlineData("a:b|c?d*e\"f.txt", "a_b_c_d_e_f.txt")]
    [InlineData("tab\there.txt", "tabhere.txt")]
    public void IllegalCharactersAreReplaced(string name, string expected)
    {
        var p = Single(name);
        Assert.Equal(expected, NewNameOf(p, name));
        Assert.True(p.Findings.Single().Issues.HasFlag(Issue.IllegalChars));
    }

    [Theory]
    [InlineData("notes.", "notes")]
    [InlineData("notes...", "notes")]
    [InlineData("notes ", "notes")]
    [InlineData("folder. ", "folder")]
    [InlineData(" lead.txt", "lead.txt")]
    public void TrailingDotsAndSpacesAndLeadingSpacesAreCut(string name, string expected)
    {
        Assert.Equal(expected, NewNameOf(Single(name), name));
    }

    [Theory]
    [InlineData("CON", "CON_")]
    [InlineData("con.txt", "con_.txt")]
    [InlineData("Nul.tar.gz", "Nul_.tar.gz")]
    [InlineData("COM1", "COM1_")]
    [InlineData("lpt9.log", "lpt9_.log")]
    [InlineData("aux .txt", "aux_.txt")]
    public void ReservedNamesGetAnUnderscore(string name, string expected)
    {
        var p = Single(name);
        Assert.Equal(expected, NewNameOf(p, name));
        Assert.True(p.Findings.Single().Issues.HasFlag(Issue.ReservedName));
    }

    [Fact]
    public void OnlyTheChosenKindsAreFixed()
    {
        var nodes = new[] { F("a?.txt"), F("b."), F("CON.txt") };
        var p = Run(nodes, kinds: FixKinds.ReservedNames);
        Assert.Equal(["CON.txt"], p.Ops.Select(o => o.OldName));
        Assert.Equal(3, p.Findings.Count);                      // everything is still reported
        Assert.Empty(Run(nodes, kinds: FixKinds.None).Ops);
    }

    [Fact]
    public void CaseClashesAreReportedOnBothSidesAndTheSecondIsNumbered()
    {
        var nodes = new[] { F("Readme.md"), F("README.md"), F("other.txt") };
        var p = Run(nodes);
        Assert.Equal(2, p.Findings.Count(f => f.Issues.HasFlag(Issue.CaseClash)));
        Assert.Equal("Readme (2).md", NewNameOf(p, "Readme.md"));        // 'README.md' sorts before 'Readme.md': the plain order decides who keeps the name
        Assert.DoesNotContain(p.Ops, o => o.Rel == "README.md");
        Assert.Empty(Run(nodes, kinds: FixKinds.All & ~FixKinds.CaseClashes).Ops);
    }

    [Fact]
    public void CleaningThatMakesTwoNamesEqualKeepsBoth()
    {
        var p = Run([F("a?.txt"), F("a_.txt"), F("a*.txt")], kinds: FixKinds.IllegalChars);
        var names = After([F("a?.txt"), F("a_.txt"), F("a*.txt")], p);
        Assert.Equal(3, names.Distinct().Count());
        Assert.Contains("a_.txt", names);                          // the name that needed no change keeps it
        Assert.Contains("a_ (2).txt", names);
    }

    [Fact]
    public void FoldersAreCleanedAndTheirContentFollows()
    {
        var nodes = new[] { D("old."), F("old.\\file.txt"), D("old.\\sub?"), F("old.\\sub?\\x.txt") };
        var p = Run(nodes);
        Assert.Equal(["old", "sub_"], p.Ops.OrderBy(o => o.Rel.Length).Select(o => o.NewName));
        Assert.Equal(["old", "old\\file.txt", "old\\sub_", "old\\sub_\\x.txt"], After(nodes, p));
    }

    // ── Length ───────────────────────────────────────────────────

    private static List<Node> DeepTree(int dirs, int dirLen, int fileLen, string fileExt = ".dat")
    {
        var nodes = new List<Node>();
        string cur = "";
        for (int i = 0; i < dirs; i++)
        {
            cur = cur.Length == 0 ? new string((char)('a' + i), dirLen) : cur + "\\" + new string((char)('a' + i), dirLen);
            nodes.Add(D(cur));
        }
        nodes.Add(F(cur + "\\" + new string('f', fileLen) + fileExt));
        return nodes;
    }

    [Fact]
    public void LongFileNameIsCutFirstAndTheExtensionIsKept()
    {
        var nodes = DeepTree(4, 30, 150);                         // 7 + 4*31 + 1+150+4 = 286
        var p = Run(nodes);
        Assert.True(p.Findings.Any(f => f.Issues.HasFlag(Issue.TooLong)));
        Assert.Equal(["fffffff"[..0] + new string('f', 150) + ".dat"], p.Ops.Where(o => !o.IsDirectory).Select(o => o.OldName));
        var after = After(nodes, p);
        Assert.All(after, rel => Assert.True(Root.Length + 1 + rel.Length <= 259, rel.Length.ToString()));
        Assert.EndsWith(".dat", after.Last());
        Assert.Contains("~", after.Last());
        Assert.DoesNotContain(p.Ops, o => o.IsDirectory);        // folders stay when the file name alone fixes it
    }

    [Fact]
    public void FoldersAreCutWhenTheFileNameIsNotEnough()
    {
        var nodes = DeepTree(8, 40, 10);                          // 7 + 8*41 + 15 = 350
        var p = Run(nodes);
        var after = After(nodes, p);
        Assert.All(after, rel => Assert.True(Root.Length + 1 + rel.Length <= 259));
        Assert.Contains(p.Ops, o => o.IsDirectory);
        Assert.Equal(0, p.Unresolved);
        Assert.Equal(after.Count, after.Select(a => a.ToUpperInvariant()).Distinct().Count());
    }

    [Fact]
    public void SameTreeAlwaysGivesTheSamePlan()
    {
        var nodes = DeepTree(8, 40, 10);
        Assert.Equal(Run(nodes).Ops, Run(nodes).Ops);
    }

    [Fact]
    public void SiblingsThatShareALongPrefixStayUnique()
    {
        var prefix = new string('x', 60);
        var nodes = new List<Node> { D(new string('d', 100)) };
        for (int i = 0; i < 20; i++) nodes.Add(F(new string('d', 100) + "\\" + prefix + i + prefix + ".txt"));
        var p = Run(nodes, limit: 150);
        var after = After(nodes, p);
        Assert.Equal(21, after.Select(a => a.ToUpperInvariant()).Distinct().Count());
        Assert.All(after, rel => Assert.True(Root.Length + 1 + rel.Length <= 150));
    }

    [Fact]
    public void ImpossibleLimitIsReportedNotHidden()
    {
        var nodes = DeepTree(6, 40, 10);
        var p = Run(nodes, limit: 40);
        Assert.True(p.Unresolved > 0);
        Assert.Contains("Move the folder", p.Findings.First(f => f.Unresolved != null).Unresolved);
    }

    [Fact]
    public void NamesOver255AreCutEvenWithAShortPath()
    {
        string name = new string('n', 300) + ".log";
        var p = Single(name);
        Assert.True(p.Findings.Single().Issues.HasFlag(Issue.NameTooLong));
        string cut = NewNameOf(p, name);
        Assert.True(cut.Length <= 255, cut.Length.ToString());
        Assert.EndsWith(".log", cut);
    }

    [Fact]
    public void LongPathFixKindOffLeavesLongPathsAlone()
    {
        var nodes = DeepTree(8, 40, 10);
        var p = Run(nodes, kinds: FixKinds.All & ~FixKinds.LongPaths);
        Assert.Empty(p.Ops);
        Assert.Contains(p.Findings, f => f.Issues.HasFlag(Issue.TooLong));
    }

    [Fact]
    public void RandomTreesEndCleanAndUnique()
    {
        var rng = new Random(7);
        string[] bits = ["a", "B", "con", "NUL", "x.", " y", "q?", "ä", "Ä", "e\u0301", "é", "long" + new string('z', 70), "dot.", "OK name", "COM3"];
        for (int round = 0; round < 40; round++)
        {
            var nodes = new List<Node>();
            var dirs = new List<string> { "" };
            for (int i = 0; i < 60; i++)
            {
                string parent = dirs[rng.Next(dirs.Count)];
                string name = bits[rng.Next(bits.Length)] + (rng.Next(3) == 0 ? "" : bits[rng.Next(bits.Length)]) + (rng.Next(2) == 0 ? ".txt" : "");
                string rel = parent.Length == 0 ? name : parent + "\\" + name;
                if (nodes.Any(n => n.Rel == rel)) continue;
                bool dir = rng.Next(3) == 0 && rel.Length < 120;
                nodes.Add(new Node(rel, dir));
                if (dir) dirs.Add(rel);
            }
            var p = Run(nodes, limit: 140);
            var after = After(nodes, p);
            Assert.Equal(nodes.Count, after.Select(a => a.Normalize(System.Text.NormalizationForm.FormC).ToUpperInvariant()).Distinct().Count());
            foreach (var (n, rel) in nodes.Zip(after))
            {
                string name = rel[(rel.LastIndexOf('\\') + 1)..];
                Assert.Equal(Issue.None, NameRules.Check(name));
                if (!p.Findings.Any(f => f.Node.Rel == n.Rel && f.Unresolved != null)) Assert.True(Root.Length + 1 + rel.Length <= 140, rel);
            }
        }
    }
}
