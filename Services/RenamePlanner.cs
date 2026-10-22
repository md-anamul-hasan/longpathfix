using System.Security.Cryptography;
using System.Text;

namespace LongPathFix.Services;

public sealed record RenameOp(string Rel, string OldName, string NewName, Issue Issues, bool IsDirectory)
{
    public string Parent => Rel.Contains('\\') ? Rel[..Rel.LastIndexOf('\\')] : string.Empty;
}

/// <param name="NewName">null when nothing is planned for it.</param>
public sealed record Finding(Node Node, Issue Issues, string? NewName, string? Unresolved)
{
    public string Text => NameRules.Text(Issues);
}

public sealed record Plan(IReadOnlyList<Finding> Findings, IReadOnlyList<RenameOp> Ops, int RootLength, int Limit, int Scanned)
{
    public int Unresolved => Findings.Count(f => f.Unresolved != null);
}

[Flags]
public enum FixKinds
{
    None = 0,
    LongPaths = 1,
    IllegalChars = 2,
    TrailingDotsAndSpaces = 4,
    ReservedNames = 8,
    CaseClashes = 16,
    All = LongPaths | IllegalChars | TrailingDotsAndSpaces | ReservedNames | CaseClashes,
}

/// <summary>Finds what is wrong and plans the renames. Pure logic over the list of names: nothing here touches the disk.</summary>
public static class RenamePlanner
{
    public const int MinBase = 8;           // the shortest a name is cut to (before "~hash" and the extension)
    public const int HashPart = 5;          // "~" and four hex digits
    public const int ClassicLimit = 259;    // MAX_PATH 260 including the terminating character

    private static Issue Wanted(FixKinds k) =>
        (k.HasFlag(FixKinds.IllegalChars) ? Issue.IllegalChars : 0)
        | (k.HasFlag(FixKinds.TrailingDotsAndSpaces) ? Issue.TrailingDotOrSpace | Issue.LeadingSpace : 0)
        | (k.HasFlag(FixKinds.ReservedNames) ? Issue.ReservedName : 0);

    private static string ParentOf(string rel) => rel.Contains('\\') ? rel[..rel.LastIndexOf('\\')] : string.Empty;

    public static Plan Build(string root, IEnumerable<Node> scanned, int limit = ClassicLimit, FixKinds kinds = FixKinds.All)
    {
        int rootLen = PathScanner.Plain(root).TrimEnd('\\').Length;
        var nodes = scanned.Where(n => n.Rel.Length > 0).DistinctBy(n => n.Rel, StringComparer.Ordinal)
                           .OrderBy(n => n.Depth).ThenBy(n => n.Rel, StringComparer.Ordinal).ToList();
        var byRel = nodes.ToDictionary(n => n.Rel, StringComparer.Ordinal);
        var newName = nodes.ToDictionary(n => n.Rel, n => n.Name, StringComparer.Ordinal);
        var issues = nodes.ToDictionary(n => n.Rel, n => NameRules.Check(n.Name), StringComparer.Ordinal);
        var unresolved = new Dictionary<string, string>(StringComparer.Ordinal);
        bool markCase = kinds.HasFlag(FixKinds.CaseClashes);

        // length of the path after the planned renames
        int NewLen(string rel)
        {
            int len = rootLen;
            for (string cur = rel; cur.Length > 0; cur = ParentOf(cur)) len += 1 + (newName.TryGetValue(cur, out var nn) ? nn.Length : cur.Length);
            return len;
        }
        List<string> Chain(string rel)      // the node first, then its folders
        {
            var list = new List<string>();
            for (string cur = rel; cur.Length > 0; cur = ParentOf(cur)) list.Add(cur);
            return list;
        }

        // 1. problems inside one name
        var fix = Wanted(kinds);
        foreach (var n in nodes)
        {
            var present = NameRules.Check(n.Name) & fix;
            if (present != Issue.None) newName[n.Rel] = NameRules.Clean(n.Name, present);
        }
        ResolveClashes(nodes, newName, issues, markCase);

        // 2. names over 255 and paths over the limit: the longest paths first, the name itself before its folders
        if (kinds.HasFlag(FixKinds.LongPaths))
        {
            foreach (var n in nodes.Where(n => newName[n.Rel].Length > NameRules.MaxNameLength).ToList())
                newName[n.Rel] = Cut(newName[n.Rel], n, Math.Max(MinBase, NameRules.MaxNameLength - (newName[n.Rel].Length - BaseLength(newName[n.Rel], n.IsDirectory)) - HashPart));
            foreach (var n in nodes.OrderByDescending(x => NewLen(x.Rel)).ToList())
            {
                if (NewLen(n.Rel) <= limit) continue;
                foreach (string rel in Chain(n.Rel))
                {
                    int excess = NewLen(n.Rel) - limit;
                    if (excess <= 0) break;
                    var node = byRel[rel];
                    string cur = newName[rel];
                    int baseLen = BaseLength(cur, node.IsDirectory);
                    int keep = Math.Max(MinBase, baseLen - excess - HashPart);
                    if (keep + HashPart >= baseLen) continue;
                    newName[rel] = Cut(cur, node, keep);
                }
                if (NewLen(n.Rel) > limit)
                    unresolved[n.Rel] = $"Even with every name cut to {MinBase} characters the path stays at {NewLen(n.Rel)} characters. Move the folder closer to the drive root.";
            }
            ResolveClashes(nodes, newName, issues, markCase);
        }

        // 3. report what was found and what is planned
        var findings = new List<Finding>();
        var ops = new List<RenameOp>();
        foreach (var n in nodes)
        {
            var found = issues[n.Rel];
            if (rootLen + 1 + n.Rel.Length > limit) found |= Issue.TooLong;
            string target = newName[n.Rel];
            bool changes = !string.Equals(target, n.Name, StringComparison.Ordinal);
            unresolved.TryGetValue(n.Rel, out var why);
            if (why == null && kinds.HasFlag(FixKinds.LongPaths) && NewLen(n.Rel) > limit) why = "Still too long after the planned renames.";
            if (found == Issue.None && !changes && why == null) continue;
            findings.Add(new Finding(n, found, changes ? target : null, why));
            if (changes) ops.Add(new RenameOp(n.Rel, n.Name, target, found, n.IsDirectory));
        }
        return new Plan(findings, ops, rootLen, limit, nodes.Count);
    }

    private static int BaseLength(string name, bool isDirectory)
    {
        if (isDirectory) return name.Length;
        int dot = name.LastIndexOf('.');
        return dot <= 0 ? name.Length : dot;
    }

    /// <summary>Keeps <paramref name="keepBase"/> characters of the base name, then "~" and four hex digits (from the original path, so the result is stable), then the extension.</summary>
    private static string Cut(string name, Node node, int keepBase)
    {
        int baseLen = BaseLength(name, node.IsDirectory);
        string stem = name[..baseLen], ext = name[baseLen..];
        if (keepBase >= stem.Length) return name;
        string hash = Convert.ToHexString(SHA1.HashData(Encoding.UTF8.GetBytes(node.Rel)))[..4].ToLowerInvariant();
        string cut = stem[..keepBase].TrimEnd(' ', '.');
        if (cut.Length == 0) cut = "_";
        return cut + "~" + hash + ext;
    }

    private static string Fold(string s) => s.Normalize(NormalizationForm.FormC).ToUpperInvariant();

    /// <summary>
    /// Two names in one folder that become the same (after cleaning, or differing only by case) get " (2)", " (3)" … The one that
    /// keeps its name is the first in plain order; names this tool did not change are never renamed unless case clashes are being fixed.
    /// </summary>
    private static void ResolveClashes(List<Node> nodes, Dictionary<string, string> newName, Dictionary<string, Issue> issues, bool markCase)
    {
        foreach (var folder in nodes.GroupBy(n => ParentOf(n.Rel), StringComparer.Ordinal))
        {
            var siblings = folder.ToList();
            if (siblings.Count < 2) continue;
            var used = new HashSet<string>(markCase ? StringComparer.OrdinalIgnoreCase : StringComparer.Ordinal);
            string Key(string s) => markCase ? Fold(s) : s;
            bool Changed(Node n) => !string.Equals(newName[n.Rel], n.Name, StringComparison.Ordinal);

            foreach (var n in siblings.OrderBy(n => Changed(n) ? 1 : 0).ThenBy(n => n.Name, StringComparer.Ordinal))
            {
                string name = newName[n.Rel];
                if (used.Add(Key(name))) continue;
                int baseLen = BaseLength(name, n.IsDirectory);
                for (int i = 2; ; i++)
                {
                    string candidate = name[..baseLen] + $" ({i})" + name[baseLen..];
                    if (used.Add(Key(candidate))) { newName[n.Rel] = candidate; break; }
                }
            }
            if (markCase)
                foreach (var g in siblings.GroupBy(n => Fold(n.Name)).Where(g => g.Count() > 1))
                    foreach (var n in g) issues[n.Rel] |= Issue.CaseClash;
        }
    }
}
