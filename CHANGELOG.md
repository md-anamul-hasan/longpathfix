# Changelog

All notable changes to LongPathFix are documented in this file. Versioning follows [Semantic Versioning](https://semver.org/); the canonical product version lives in `Cargo.toml`.

## [2.1.0] - 2026-11-19

### Changed
- New look: Technical Glassmorphism v1.0, the design system of mdanamulhasan.com. Slate canvas, Precision Cobalt accent with PLM teal highlights in dark mode, hairline borders, crisp 6/10/14 px corners, restrained shadows.

## [2.0.0] - 2026-11-14

Complete architectural transformation from `.NET 10 / WPF` to a 100% native **Rust + Tauri 2** Windows desktop application with a redesigned Fluent-inspired TypeScript/React interface.

### Added
- Native Win32 engine (`crates/longpathfix-core`) using `windows-sys` (`FindFirstFileExW`, `GetFileAttributesExW`, `MoveFileExW`, `CreateFileW`, `GetVolumeInformationW`) over `\\?\` and `\\?\UNC\` extended paths up to ~32,767 UTF-16 code units.
- Bounded-parallel directory scanner with cancellation, symlink/junction loop protection (`FILE_ATTRIBUTE_REPARSE_POINT`), inaccessible-folder accounting, and live progress updates.
- Dedicated security boundary module enforcing lexical root normalization, single-segment name checks, TOCTOU parent reparse-point checks before every rename/undo, hardened DLL search order (`SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32)`), and strict Tauri CSP/capabilities.
- Dry-run verification stage classifying every planned rename (`Ready`, `ReadOnly`, `TargetAlreadyExists`, `SourceMissing`, `LockedByProcess`, `InvalidBoundary`) before modifying disk.
- System Diagnostics view reporting `HKLM\...\LongPathsEnabled`, Windows build number, embedded PE `longPathAware` manifest state, mounted volume filesystem capabilities (`NTFS`, `ReFS`, `exFAT`, `FAT32`, case sensitivity, reparse support), and an explicit UAC-elevated `"Enable Windows Long Paths…"` helper (`ShellExecuteExW` `"runas"` `--enable-long-paths`).
- Redesigned UI (`src-ui/`) built with React 19, TypeScript 5.8, and Vite 6 following the documented `DESIGN.md` system: dark/light/system themes, live PathGauge before/after length visualization, Path Inspector drawer, sortable/filterable findings table, Repair Preview diff modal, History batch undo, Settings & Field Guide, keyboard navigation, and WCAG 2.2 AA accessibility attributes.
- Automated `cargo test` suite (40 unit tests + 8 disk integration tests), `cargo-deny` policy (`deny.toml`), CI workflow (`.github/workflows/ci.yml`), portable ZIP packaging, and generated `THIRD_PARTY_NOTICES.md`.

### Changed
- Application metadata and Inno Setup installer updated to `v2.0.0` while preserving `AppId={{25FFAF8A-4D8A-5C88-B27E-FD990DE9FB1B}}` for seamless in-place upgrades over `1.1.0` (automatically cleaning legacy `.NET` DLLs and runtime JSON files).
- History storage (`%APPDATA%\LongPathFix\history\*.json`), `rename-log.csv`, and settings (`settings.json`) remain compatible with `1.x` files so existing undo batches continue to work.
- Logging expanded to rotated JSON-lines logs under `%APPDATA%\LongPathFix\logs\longpathfix.log` plus `error.log`.

### Removed
- All legacy C#, WPF, XAML, `.csproj`, `.slnx`, and `.NET 10` runtime dependencies and build scripts.

## [1.1.0] - 2026-10-22

First open-source release (MIT). Earlier builds were a paid product; this release removed licensing entirely so every feature is free.

### Added
- Finds long paths, over-long names, trailing dots and spaces, reserved names, illegal characters and case clashes.
- Shows the full rename plan before anything changes; limits of 259, 240 or 200 characters.
- Renames safely (only names change), logs every rename and undoes a batch from the History page.
- Exports the findings as CSV.
- Light and dark themes; self-contained installer.

### Removed
- Licence-key activation, the licence dialog, evaluation limits and the licence-server connection.
