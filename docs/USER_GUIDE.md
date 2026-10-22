# LongPathFix — User Guide

LongPathFix finds the file and folder names that cause trouble when you back up, zip, sync or copy, and renames them safely.

## What it finds
| Problem | Why it hurts | The plan |
|---|---|---|
| Path longer than the limit | Copy, ZIP and backup tools fail above 260 characters | The file name is cut first, then its folders from the deepest up. The cut part becomes `~` and four letters or digits so the name stays unique; the extension is kept; no name is cut below 8 characters. |
| Name over 255 characters | Not allowed on NTFS | Cut the same way |
| Ends with a dot or space, or starts with a space | Explorer, ZIP tools and many programs cannot open it | The dots and spaces are cut |
| Reserved name (CON, PRN, AUX, NUL, COM1–9, LPT1–9, with or without extension) | Windows treats it as a device | An underscore is added: `CON.txt` becomes `CON_.txt` |
| Illegal characters (< > : " / \ | ? * and control characters) | Not allowed on Windows; they appear on files from Linux, macOS or the network | Replaced by `_` (control characters are removed) |
| Names that differ only by case | The two overwrite each other on Windows and macOS | The later one gets `(2)`, `(3)` |

Two names that become equal after cleaning also get `(2)`. A name is never renamed to one that already exists.

## When a path cannot be fixed by renaming
If the folder itself is so deep that the path is too long even with every name cut to 8 characters, it is listed with the reason: move the folder closer to the drive root.

## Safe by design
- You see every rename before anything happens, and you can switch kinds of fixes off.
- Only names change. Nothing is moved, deleted or rewritten.
- The log is written as each rename is done; a rename is skipped and reported if the new name is taken.
- **Undo** (History page) restores the old names, folders first. Items that were renamed or removed since are skipped and reported.
- Links and junctions are listed but never entered.
- **Warning:** shortcuts, project files and scripts that point to a renamed file use the old path and need updating.

## Export
**Export findings (CSV)** writes every finding with its problem, planned name and, if any, the reason it cannot be fixed.

## Notes
Case clashes can only exist on case-sensitive folders (Linux and WSL shares, network drives, folders where case sensitivity is switched on); on a normal Windows folder you will not see any.


## Problems
Errors are logged to `%AppData%\LongPathFix\error.log`. Open an [issue on GitHub](https://github.com/md-anamul-hasan/longpathfix/issues) and attach that file (remove anything private first).
