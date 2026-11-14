# LongPathFix — User Guide

LongPathFix finds the file and folder names that cause backup, ZIP, cloud-sync, and copy failures on Windows and plans safe, reversible renames without changing file contents or moving items between folders.

## 1. Choosing a folder and scan threshold

1. Click **Choose folder…** (or pick a recent location) in **Scan & Fix**.
2. Choose a path-length limit that fits your workflow:
   - **259 (Classic MAX_PATH):** Maximum visible characters for legacy Win32 APIs (`MAX_PATH = 260` minus NUL).
   - **240 (ZIP / Copy margin):** Recommended headroom when creating archives or syncing nested containers.
   - **200 (Extra margin):** Extra margin if the files will be moved under a deeper cloud-sync root (e.g., OneDrive, SharePoint, NAS share).
   - **Custom (`40`–`1000`):** Exact custom UTF-16 length threshold.
3. Optional: check or uncheck individual rule toggles (`Long paths`, `Long names`, `Trailing dot/space`, `Leading space`, `Reserved names`, `Illegal chars`, `Case clashes`, `Invalid Unicode`) or allow clearing `Read-only` attributes during repairs.

> Note: LongPathFix warns before scanning drive roots (such as `C:\`) or protected system locations (`C:\Windows`, `C:\Program Files`, `C:\ProgramData`) so you do not break installed software.

## 2. What LongPathFix detects and how names are planned

| Problem | Why tools fail | Rename plan strategy |
|---|---|---|
| **Path over limit** | Explorer, ZIP tools, backup software, or non-long-path-aware apps fail past 259 UTF-16 chars | Shortens the leaf filename first; if still too long, trims ancestor folders from deepest upward. Cut ends become `~xxxx` (deterministic 4-hex SHA-1 suffix) preserving the file extension; never shortens a segment below 8 characters |
| **Name over 255 characters** | NTFS, ReFS, exFAT, and FAT32 enforce a 255 UTF-16 code-unit per-component limit | Trims segment with `~xxxx` while keeping the extension |
| **Trailing dot or space / leading space** | Classic Win32 normalization strips trailing dots/spaces (`notes.` -> `notes`), causing "file not found" errors | Strips leading spaces and trailing dots/spaces |
| **Reserved Windows device name** | `CON`, `PRN`, `AUX`, `NUL`, `CLOCK$`, `CONIN$`, `CONOUT$`, `COM0`–`9`, `LPT0`–`9` (with any extension, e.g., `CON.txt`) resolve to kernel devices | Appends `_` to the stem (`CON.txt` -> `CON_.txt`) |
| **Illegal characters** | `< > : " / \ | ? *` and ASCII control characters imported from macOS/Linux/NAS are illegal on Windows filesystems | Removes control characters and replaces `< > : " / \ | ? *` with `_` |
| **Invalid Unicode** | Malformed filenames with replacement characters (`U+FFFD`) corrupt cross-platform sync | Flags items clearly for inspection and cleaning |
| **Case-only / NFC clash** | Two sibling items differ only by uppercase/lowercase or Unicode normalization (macOS NFD vs NFC) and overwrite each other on Windows | Appends ` (2)`, ` (3)`, … to colliding siblings without altering clean existing items |

If a directory hierarchy is so deeply nested that trimming every segment down to 8 characters still exceeds your chosen limit, LongPathFix honestly marks those items as **Cannot fix by renaming alone** and recommends moving the top parent folder closer to the drive root.

## 3. Inspecting findings, filtering, and exporting CSV

- Click any row in **Scan & Fix** to open the **Path Inspector** drawer showing original vs proposed segments, UTF-16 before/after length (`PathGauge`), filesystem location, and quick action to reveal the item in Windows Explorer.
- Filter by problem category (`All issues`, `Fixable`, `Unfixable`, or specific rule tags), sort columns, or search by substring.
- Click **Export CSV…** to save a UTF-8 BOM spreadsheet listing every finding, problem summary, proposed new name, and unfixable reason.

## 4. Dry-run preview and applying renames

1. Select the findings you want to repair (all fixable findings are checked by default, or toggle individual rows).
2. Click **Review & Apply…** to run a **live dry-run check** against disk.
3. The preview modal checks each planned rename for:
   - `Ready`
   - `Read-only` (will temporarily clear `FILE_ATTRIBUTE_READONLY` during rename if enabled in Settings)
   - `Collision` (destination name already exists)
   - `Locked` (`ERROR_SHARING_VIOLATION` / `ERROR_LOCK_VIOLATION`—another program has the file open)
   - `Missing` / `Blocked` (item disappeared or parent changed into a reparse point)
4. Click **Apply N Renames** only when satisfied. LongPathFix renames items deepest-first via Win32 `MoveFileExW` over extended `\\?\` paths and verifies the target exists on disk before recording success.

## 5. History & Undo

Every applied repair batch is saved step-by-step under `%APPDATA%\LongPathFix\history\<batch-id>.json` (100% compatible with LongPathFix 1.x history files) and logged in `rename-log.csv`.

- Open **History** to inspect past repair batches and per-item outcomes.
- Click **Undo Batch** to restore original names in reverse order (parent folders first, then children). Items moved or renamed externally since the batch ran are safely skipped and reported.

## 6. System Diagnostics (OS Policy & Drives)

Open **Diagnostics** to inspect:
- **OS `LongPathsEnabled` policy** (`HKLM\SYSTEM\CurrentControlSet\Control\FileSystem\LongPathsEnabled`), Windows build number, and process `longPathAware` manifest status.
- **Mounted volumes:** filesystem type (`NTFS`, `ReFS`, `exFAT`, `FAT32`), `maxComponentLength`, Unicode on disk, reparse point support, and case-sensitivity flags.
- **Enable Windows Long Paths…:** Optional elevated action (explicitly prompts Windows UAC) to set `LongPathsEnabled = 1` for long-path-aware applications that support standard paths past 259 chars.

## 7. Troubleshooting

Diagnostic logs rotate automatically under `%APPDATA%\LongPathFix\logs\longpathfix.log` (structured JSON lines) with fallback error summaries in `%APPDATA%\LongPathFix\error.log`. When reporting an issue at [github.com/md-anamul-hasan/longpathfix/issues](https://github.com/md-anamul-hasan/longpathfix/issues), attach relevant log lines after removing any private file or folder names.
