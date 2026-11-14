# Security Policy

## Supported Versions

Only the latest `2.x` release of LongPathFix receives security fixes. Older `1.x` (.NET/WPF) builds are unsupported and superseded by `2.0.0`.

| Version | Supported |
| --- | --- |
| 2.0.x (Rust + Tauri 2) | Yes |
| 1.x (.NET / WPF) | No (end of life) |

## Architectural Security Controls

LongPathFix is engineered around offline least-privilege execution on Windows 10/11:

- **Zero network footprint:** No outbound connections, update checks, telemetry, or remote IPC endpoints.
- **Unprivileged standard execution (`asInvoker`):** Scans and renames run with standard user privileges. Elevation is only requested when the user explicitly clicks **Enable Windows Long Paths…** in Diagnostics to set `HKLM\SYSTEM\CurrentControlSet\Control\FileSystem\LongPathsEnabled = 1` via Windows UAC (`ShellExecuteExW` `"runas"` `--enable-long-paths`).
- **Hardened process & WebView boundary:** Calls `SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32)` before startup, enforces a strict CSP (`default-src 'self'`, disallowing remote scripts, objects, or frames), and restricts Tauri capabilities exclusively to `core:default`, `dialog:allow-open`, and `dialog:allow-save`.
- **Path traversal & TOCTOU guards:** Every rename and undo operation validates that the parent directory lies strictly within the scanned root (`is_within`), rejects multi-segment names, control characters, `.` and `..`, checks parent directory `FILE_ATTRIBUTE_REPARSE_POINT` status immediately before renaming, never enters symlinks or junctions during scanning, and refuses to overwrite existing files/folders (`MoveFileExW` flags = `0`).

Full audit findings and mitigations are documented in [docs/SECURITY_AUDIT.md](docs/SECURITY_AUDIT.md).

## Reporting a Vulnerability

If you discover a security issue in LongPathFix (path escape, privilege escalation, WebView/IPC bypass, installer weakness, or dependency vulnerability):

1. **Do not** disclose exploit details publicly in a GitHub issue before a fix is prepared.
2. Email **contact@mdanamulhasan.com** or open a [GitHub Private Vulnerability Report](https://github.com/md-anamul-hasan/longpathfix/security/advisories/new) with reproduction steps, affected version, and Windows build number.
3. Expect an acknowledgement within 72 hours and a status update once the report is reproduced.

## Supply-Chain & Binary Verification

Every release publishes `SHA256SUMS.txt` alongside `LongPathFix-Setup-<version>.exe` and `LongPathFix-<version>-win64-portable.zip`. Verify downloads before running:

```powershell
Get-FileHash .\LongPathFix-Setup-2.0.0.exe -Algorithm SHA256
```
