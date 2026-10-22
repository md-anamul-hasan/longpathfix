using System.IO;
using System.IO.Enumeration;

namespace LongPathFix.Services;

/// <summary>A file or folder below the scanned root. <see cref="Rel"/> uses "\" and has no leading separator.</summary>
public sealed record Node(string Rel, bool IsDirectory)
{
    public string Name => Rel.Contains('\\') ? Rel[(Rel.LastIndexOf('\\') + 1)..] : Rel;
    public string Parent => Rel.Contains('\\') ? Rel[..Rel.LastIndexOf('\\')] : string.Empty;
    public int Depth => Rel.Count(c => c == '\\');
}

public static class PathScanner
{
    /// <summary>The path with the "\\?\" prefix, so names that Windows normally refuses (trailing dot, CON) and paths over 260 characters can be used.</summary>
    public static string Extended(string path)
    {
        if (path.StartsWith(@"\\?\", StringComparison.Ordinal)) return path;
        // an absolute path is only prefixed, never normalised: GetFullPath would cut a trailing dot or space off the last name
        bool absolute = path.StartsWith(@"\\", StringComparison.Ordinal) || (path.Length >= 3 && char.IsAsciiLetter(path[0]) && path[1] == ':' && path[2] == '\\');
        string full = absolute ? path : Path.GetFullPath(path);
        return full.StartsWith(@"\\", StringComparison.Ordinal) ? @"\\?\UNC\" + full[2..] : @"\\?\" + full;
    }

    /// <summary>The normal form of a path (without the prefix).</summary>
    public static string Plain(string path) =>
        path.StartsWith(@"\\?\UNC\", StringComparison.Ordinal) ? @"\\" + path[8..] : path.StartsWith(@"\\?\", StringComparison.Ordinal) ? path[4..] : path;

    public static IEnumerable<Node> Scan(string root, CancellationToken ct = default)
    {
        string ext = Extended(Path.GetFullPath(PathScanner.Plain(root))).TrimEnd('\\');
        var options = new EnumerationOptions
        {
            RecurseSubdirectories = true,
            IgnoreInaccessible = true,
            AttributesToSkip = 0,
        };
        var items = new FileSystemEnumerable<(string Path, bool Dir, bool Link)>(ext + "\\",
            (ref FileSystemEntry e) => (e.ToFullPath(), e.IsDirectory, (e.Attributes & FileAttributes.ReparsePoint) != 0), options)
        {
            ShouldRecursePredicate = (ref FileSystemEntry e) => (e.Attributes & FileAttributes.ReparsePoint) == 0,
        };
        foreach (var (path, dir, link) in items)
        {
            ct.ThrowIfCancellationRequested();
            if (link && dir) { yield return new Node(Path.GetRelativePath(ext, path), true); continue; }   // listed, never entered
            yield return new Node(Path.GetRelativePath(ext, path), dir);
        }
    }
}
