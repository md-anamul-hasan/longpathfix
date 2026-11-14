# LongPathFix 2.0 Security Audit & Threat Model

This document records the security controls implemented and verified during the transformation of LongPathFix `2.0.0` to Rust + Tauri 2 on Windows 10/11 (x64).

## 1. Threat Surface & Mitigations

### 1.1 Tauri WebView, CSP & IPC Boundaries

| Surface | Control Implemented | Verification |
| --- | --- | --- |
| **Content Security Policy (CSP)** | Strict CSP in `crates/longpathfix-app/tauri.conf.json`: `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' asset: https://asset.localhost data:; font-src 'self' data:; connect-src 'none'; object-src 'none'; base-uri 'self'; form-action 'none'; frame-ancestors 'none'`. Blocks remote code, iframes, and outbound network fetches (`connect-src 'none'`). | Audited `tauri.conf.json` |
| **Capabilities** | `crates/longpathfix-app/capabilities/default.json` grants only `"core:default"`, `"dialog:allow-open"`, `"dialog:allow-save"` to the `"main"` window. Shell execution, arbitrary filesystem plugins, HTTP, and global shortcuts are absent. | Audited `capabilities/default.json` |
| **IPC Validation** | All filesystem paths, filenames, batch IDs, search limits (`clamp(40, 1000)`), and page sizes (`clamp(1, 500)`) are re-validated in Rust commands (`commands.rs` + `security.rs`). Frontend validation (`validate_custom_name`, `analyze_path_input`) calls Rust rather than trusting JS state. | Audited `commands.rs` |

### 1.2 Path Traversal, Symlink/Junction & TOCTOU Controls

LongPathFix never alters file contents, deletes files, or moves items between folders—only single-segment renames inside the parent folder are permitted.

- **Lexical Root Boundary (`security::verify_rename_pair`):**
  - Verifies `is_within(dir_plain, &root.plain)` using component-aware case-insensitive matching so sibling prefix traps like `C:\data\project2` cannot pass as inside `C:\data\project`.
  - Rejects empty names, `"."`, `".."`, `'\\'`, `'/'`, control characters (`< 32`), and segments over `255` UTF-16 code units (`validate_single_name`).
  - Re-validates batch IDs via `storage::validate_batch_id` (ASCII alphanumeric + `-` / `_`, max length 64) before constructing `history\<id>.json` paths.
- **Symlink / Junction / Reparse Point Protections:**
  - `scanner` checks `FILE_ATTRIBUTE_REPARSE_POINT` on every directory entry via `FindFirstFileExW` and records links/junctions without descending into them.
  - Before every individual rename or undo (`verify_rename_pair`), LongPathFix re-queries `GetFileAttributesExW` on the parent directory (`\\?\...`): if the parent disappeared, ceased to be a directory, or turned into a reparse point since the scan (TOCTOU symlink/junction swap), the rename is refused (`Skipped: parent folder is a symlink or junction`).
- **No-Overwrite Rename Guarantee:**
  - `filesystem::rename_no_overwrite` calls `MoveFileExW(from, to, 0)` (no `MOVEFILE_REPLACE_EXISTING` flag) after checking destination existence, preventing accidental clobbering of existing files, including on case-preserving NTFS folders.

### 1.3 Privilege Separation & Elevation

- **`asInvoker` Default Execution:** `crates/longpathfix-app/app.manifest` declares `requestedExecutionLevel level="asInvoker" uiAccess="false"`. Normal scans, previews, renames, exports, and undos run strictly unprivileged.
- **Explicit User-Initiated Elevation (`--enable-long-paths`):** The optional **Enable Windows Long Paths…** action in Diagnostics calls `windows::request_enable_long_paths_elevated`, triggering standard Windows UAC via `ShellExecuteExW` (`lpVerb = "runas"`, `lpFile = current_exe()`, `lpParameters = "--enable-long-paths"`). In `main.rs`, this CLI flag writes `HKLM\SYSTEM\CurrentControlSet\Control\FileSystem\LongPathsEnabled = 1` (`REG_DWORD`) and exits immediately without starting the WebView GUI inside an elevated context.

### 1.4 DLL Search-Order Hardening & External Launches

- **DLL Preloading Mitigation:** First statement in `main.rs` calls `SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32)`, confining OS DLL resolution to `%SystemRoot%\System32` before Tauri/WebView2 initialize.
- **Explorer Reveal (`open_folder` / `reveal_in_explorer`):** Always normalizes input to an absolute filesystem path, verifies existence on disk with `GetFileAttributesExW`, and invokes `ShellExecuteW` on verified folders or `explorer.exe /select,"..."`—never executing arbitrary URLs or untrusted file extensions.

### 1.5 Data Persistence & Temporary File Integrity

- **Atomic Writes (`filesystem::atomic_write`):** Config updates (`settings.json`), batch history (`history\<id>.json`), and CSV exports write to a uniquely named sibling temp file in the destination directory, `sync_all()` to disk, and replace atomically via `MoveFileExW(..., MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)`.
- **Log Rotation (`logging.rs`):** JSON-lines logs automatically rotate at `2 MB` to avoid unbounded disk consumption.
- **Protected OS Folders (`security::protected_path_warning`):** Detects drive roots (`C:\`) and protected OS/program paths (`%SystemRoot%`, `%ProgramFiles%`, `%ProgramFiles(x86)%`, `%ProgramData%`) and displays explicit warnings in Diagnostics / root selection before scanning.

### 1.6 Unsafe Rust Inventory

All `unsafe` blocks are isolated strictly within `crates/longpathfix-core/src/filesystem.rs`, `crates/longpathfix-core/src/scanner.rs`, `crates/longpathfix-core/src/windows.rs`, and the single `SetDefaultDllDirectories` startup call in `crates/longpathfix-app/src/main.rs`. Every `unsafe` FFI call is annotated with `// SAFETY:` invariants documenting NUL-terminated UTF-16 lifetime guarantees, explicit handle closing (`CloseHandle`, `FindClose`, `RegCloseKey`), and Win32 error handling (`GetLastError`). Core path math, rules, planner, security, config, and storage modules contain **zero `unsafe` code**.

## 2. Supply-Chain & License Audit Results

- **Policy enforcement (`deny.toml`):** Scoped to `x86_64-pc-windows-msvc`, permits only OSI-approved permissive/weak-copyleft compatible licenses (`MIT`, `Apache-2.0`, `BSD-*`, `ISC`, `Zlib`, `Unicode-*`, `MPL-2.0`, `BSL-1.0`, `CC0-1.0`), denies unknown registries and git dependencies (`allow-git = []`).
- **Automated Verification (`cargo deny check`):** `advisories ok, bans ok, licenses ok, sources ok` (Exit 0).
- **Automated Vulnerability Scan (`cargo audit`):** `0 vulnerabilities found` across 427 lockfile packages (Exit 0; 2 non-vulnerability informational warnings on transitive crates `proc-macro-error 1.0.4` [build-time proc-macro] and `glib 0.18.5` [non-Windows Linux/GTK target crate]).

## 3. Installer & Distribution Security

- **`PrivilegesRequired=lowest` with User Override Option (`installer/longpathfix.iss`):** Allows per-user installation without UAC escalation by default.
- **Upgrade Hygiene (`[InstallDelete]`):** Preserves existing AppId `{25FFAF8A-4D8A-5C88-B27E-FD990DE9FB1B}` and removes stale `.NET 10` `*.dll`, `*.deps.json`, `*.runtimeconfig.json`, `.pdb`, and `createdump.exe` artifacts so old unpatched DLLs do not linger beside `LongPathFix.exe`.
- **Verification & Signing:** All release artifacts are hashed into `SHA256SUMS.txt` (`package-release.ps1`) and prepared for SignPath Foundation authentic-code signing via `.signpath/artifact-configurations/{binaries,installer}.xml`.
