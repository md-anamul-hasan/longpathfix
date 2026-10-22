using System.Text;

namespace LongPathFix.Services;

[Flags]
public enum Issue
{
    None = 0,
    /// <summary>The full path is longer than the chosen limit.</summary>
    TooLong = 1,
    /// <summary>A single name is longer than 255 characters.</summary>
    NameTooLong = 2,
    /// <summary>The name ends with a dot or a space: Explorer, ZIP tools and many apps cannot open it.</summary>
    TrailingDotOrSpace = 4,
    /// <summary>The name starts with a space.</summary>
    LeadingSpace = 8,
    /// <summary>CON, PRN, AUX, NUL, COM1–9, LPT1–9 (with or without an extension).</summary>
    ReservedName = 16,
    /// <summary>Contains one of &lt; &gt; : " / \ | ? * or a control character.</summary>
    IllegalChars = 32,
    /// <summary>Another name in the same folder differs only by capitalisation.</summary>
    CaseClash = 64,
}

/// <summary>What is wrong with a single name, and the clean version of it.</summary>
public static class NameRules
{
    public const int MaxNameLength = 255;
    private static readonly HashSet<string> Reserved = new(StringComparer.OrdinalIgnoreCase)
    {
        "CON", "PRN", "AUX", "NUL", "CLOCK$",
        "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
        "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    };
    private const string IllegalSet = "<>:\"/\\|?*";

    public static bool IsReserved(string name)
    {
        int dot = name.IndexOf('.');
        string stem = (dot < 0 ? name : name[..dot]).TrimEnd(' ');
        return Reserved.Contains(stem);
    }

    public static bool HasIllegalChars(string name) => name.Any(c => c < 32 || IllegalSet.Contains(c));

    /// <summary>Problems that belong to the name itself (length of the whole path and case clashes are decided elsewhere).</summary>
    public static Issue Check(string name)
    {
        var issue = Issue.None;
        if (name.Length > MaxNameLength) issue |= Issue.NameTooLong;
        if (name.Length > 0 && (name[^1] == '.' || name[^1] == ' ')) issue |= Issue.TrailingDotOrSpace;
        if (name.Length > 0 && name[0] == ' ') issue |= Issue.LeadingSpace;
        if (IsReserved(name)) issue |= Issue.ReservedName;
        if (HasIllegalChars(name)) issue |= Issue.IllegalChars;
        return issue;
    }

    /// <summary>The name with the chosen problems removed. Returns the name unchanged when it has none of them.</summary>
    public static string Clean(string name, Issue fix)
    {
        string n = name;
        if (fix.HasFlag(Issue.IllegalChars))
        {
            var sb = new StringBuilder(n.Length);
            foreach (char c in n)
            {
                if (c < 32) continue;
                sb.Append(IllegalSet.Contains(c) ? '_' : c);
            }
            n = sb.ToString();
        }
        if (fix.HasFlag(Issue.LeadingSpace)) n = n.TrimStart(' ');
        if (fix.HasFlag(Issue.TrailingDotOrSpace)) n = n.TrimEnd('.', ' ');
        if (n.Length == 0) n = "_";
        if (fix.HasFlag(Issue.ReservedName) && IsReserved(n))
        {
            int dot = n.IndexOf('.');
            n = dot < 0 ? n.TrimEnd(' ') + "_" : n[..dot].TrimEnd(' ') + "_" + n[dot..];
        }
        return n;
    }

    public static string Text(Issue i)
    {
        var p = new List<string>();
        if (i.HasFlag(Issue.TooLong)) p.Add("path too long");
        if (i.HasFlag(Issue.NameTooLong)) p.Add("name over 255 characters");
        if (i.HasFlag(Issue.TrailingDotOrSpace)) p.Add("ends with dot or space");
        if (i.HasFlag(Issue.LeadingSpace)) p.Add("starts with a space");
        if (i.HasFlag(Issue.ReservedName)) p.Add("reserved name");
        if (i.HasFlag(Issue.IllegalChars)) p.Add("illegal characters");
        if (i.HasFlag(Issue.CaseClash)) p.Add("differs only by case from another name");
        return string.Join(", ", p);
    }
}
