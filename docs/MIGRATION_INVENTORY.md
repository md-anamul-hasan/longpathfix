# LongPathFix v1.x (.NET 10 / WPF) -> v2.0.0 (Rust + Tauri 2) Migration Inventory

Every legacy capability, service, UI screen, test suite, and packaging workflow from LongPathFix `1.1.0` was audited and mapped cleanly to native Rust, Win32 (`windows-sys`), and TypeScript/React modules.

## 1. Engine & System Services Mapping

| Legacy 1.x Component (.NET / WPF) | 2.0.0 Native Replacement | Key Enhancements |
| --- | --- | --- |
| `Services/PathHandling` / `LongPathHelper` | `crates/longpathfix-core/src/path.rs` | Full classification (`PathForm`: Drive, UNC, ExtendedDrive, ExtendedUnc, Device, DriveRelative, RootRelative, Relative, Empty), lexical normalization, `\\?\` & `\\?\UNC\` conversion, UTF-16 length accounting |
| `Services/Rules` | `crates/longpathfix-core/src/rules.rs` | `Issue` bitflags & `FixKinds` toggles covering `>259`/`custom` paths, `>255` names, trailing dot/space, leading space, reserved stems (`CON..LPT9`, `CLOCK$`, `CONIN$`, `CONOUT$`), illegal chars/control chars, invalid Unicode `U+FFFD`, and NFC + case folding |
| `Services/Scanner` | `crates/longpathfix-core/src/scanner.rs` | Bounded-parallel `FindFirstFileExW` / `FindNextFileW` (`FindExInfoBasic`, `FIND_FIRST_EX_LARGE_FETCH`), symlink/junction tracking without recursion, streaming progress & responsive `AtomicBool` cancellation |
| `Services/RenamePlanner` | `crates/longpathfix-core/src/planner.rs` | Deterministic leaf-first shortening with SHA-1 `~xxxx` suffixes, minimum 8-char stems, preserved extensions, collision numbering `(2)`, honest unfixable path diagnostics |
| `Services/Renamer` & Undo | `crates/longpathfix-core/src/repair.rs` + `filesystem.rs` | Live pre-apply `dry_run` checks (`Ready`, `ReadOnly`, `LockedByProcess`, `TargetAlreadyExists`, `SourceMissing`, `InvalidBoundary`), optional `FILE_ATTRIBUTE_READONLY` clearing, `MoveFileExW(..., 0)` + target verification |
| `Services/HistoryStore` | `crates/longpathfix-core/src/storage.rs` | Reads and writes `%APPDATA%\LongPathFix\history\*.json` using the **exact 1.x PascalCase schema** plus `rename-log.csv`, preserving full backward compatibility with v1.x undo history |
| `Services/Settings` & `ErrorLogger` | `crates/longpathfix-core/src/config.rs` & `logging.rs` | Atomic `settings.json` persistence (sanitized limits `40`–`1000`, recent folders capped at 8) + 2 MB rotating JSON-lines logs + `error.log` fallback |
| `Services/CsvExport` | `crates/longpathfix-core/src/export.rs` | UTF-8 BOM CSV exports for findings and post-repair outcomes with RFC-4180 escaping |
| `Services/WindowHelpers` & Win32 P/Invoke | `crates/longpathfix-core/src/windows.rs` + `app.manifest` | `LongPathsEnabled` HKLM inspection, `RT_MANIFEST` `longPathAware` probe, `GetVolumeInformationW` / `GetDriveTypeW` per-drive analysis, optional UAC `--enable-long-paths`, Explorer open/reveal |
| *(New)* | `crates/longpathfix-core/src/security.rs` | Scan root validation, protected OS folder warnings, single-name validation, and TOCTOU parent reparse-point check on every rename/undo |

## 2. UI & Presentation Mapping

| Legacy 1.x Screen / Control | 2.0.0 Screen / Component (`src-ui/`) | Notes |
| --- | --- | --- |
| `MainWindow.xaml` / `ShellViewModel` | `App.tsx`, `Sidebar.tsx` | Responsive shell, live progress toast/bar, keyboard navigation, Light/Dark/System themes matching Windows 11 tokens (`DESIGN.md`) |
| Scan & Plan tab (`MainViewModel`) | `FindingsWorkspace.tsx`, `PathGauge.tsx` | Paginated IPC findings table (500-row windowing cleanly handles 100k+ entries), filter chips, sorting, PathGauge before/after UTF-16 visual scale |
| *(New)* | `PathInspectorDrawer.tsx` | Side drawer breaking down each path segment before/after, live lock/read-only disk probe, extended `\\?\` view, Reveal in Explorer |
| Rename confirmation dialog | `RepairPreviewModal.tsx` | Executes live `preview_repair` dry-run prior to mutation, shows readiness counters and collision/lock diagnostics |
| *(New)* | `DiagnosticsView.tsx` | Interactive **Path Lab** (`analyze_path_input`), Windows OS `LongPathsEnabled` & `longPathAware` check, optional UAC fixer, and mounted volume table |
| History tab | `HistoryView.tsx` | Compatible batch list (including 1.x batches), per-rename inspection, one-click folder-first Undo & CSV export |
| How It Works & Theme menu | `SettingsGuideView.tsx` | Theme switcher, threshold defaults, rule toggles, read-only behavior, `%APPDATA%` folder shortcuts, field guide |

## 3. Tests, Tooling & Legacy Removal

- **Test Suite Replacement:** Replaced legacy `LongPathFix.Tests` (xUnit) with **40 Rust unit tests** (`longpathfix-core`) + **8 on-disk integration tests** (`crates/longpathfix-core/tests/disk_tests.rs`) covering extended paths, emoji/Unicode surrogate UTF-16 counting, NFC folding, trailing dot/space, reserved device names, read-only attributes, lock contention, TOCTOU boundaries, 1.x history JSON parsing, and full apply/undo cycles.
- **Removed Legacy Artifacts:** Deleted `Controls/`, `Converters/`, `Services/`, `Themes/`, `ViewModels/`, `LongPathFix.Tests/`, `bin/`, `obj/`, `App.xaml*`, `MainWindow.xaml*`, `AssemblyInfo.cs`, `Directory.Build.props`, `LongPathFix.csproj`, and `LongPathFix.slnx`.
- **Packaging & CI Replacement:** Replaced MSBuild / `dotnet publish` scripts with Cargo release builds (LTO + symbol stripping), Vite 6 frontend bundling, updated Inno Setup `installer/longpathfix.iss` (keeping `AppId={{25FFAF8A-4D8A-5C88-B27E-FD990DE9FB1B}}` and `[InstallDelete]` cleanup for 1.x `.NET` DLLs), portable `.zip` packaging (`scripts/package-release.ps1`), and GitHub Actions workflows (`.github/workflows/{ci,release}.yml`).
