# LongPathFix 2.0 Architecture

LongPathFix uses a strict **engine / shell / presentation** split with zero duplicated domain logic:

1. **Engine crate (`crates/longpathfix-core`)** — 100% pure Rust + native Win32 (`windows-sys`), owning all path parsing, rules, directory traversal, rename planning, security validation, dry-run/apply/undo execution, history persistence, configuration, logging, and CSV export.
2. **Desktop shell (`crates/longpathfix-app`, package `longpathfix`)** — Tauri 2 host managing process hardening, window lifecycle/theme integration, single background-operation locking (`AppState`), background worker threads emitting streaming progress events, and thin IPC command DTO translation.
3. **Presentation layer (`src-ui/`)** — React 19 + TypeScript 5.8 + Vite 6 static UI bundle. Performs **no filesystem calls and no rename math**; every rule evaluation, path normalization, filter query, dry-run verification, and repair batch is executed in Rust and rendered cleanly via Tauri IPC.

## 1. Engine Module Dependency Graph (`longpathfix-core`)

From bottom (primitives) to top (orchestration & I/O):

```text
errors              Centralized typed Error & Result + serialized error payloads
  ▲
path                Windows PathForm classification, lexical normalization, \\?\ & \\?\UNC\, UTF-16 lengths
  ▲
rules               Single-segment Issue bitflags, FixKinds toggles, NFC folding, reserved/illegal/dot/space clean()
  ▲
windows             Win32 Registry (LongPathsEnabled, CurrentBuildNumber), RT_MANIFEST check, UAC elevation hook,
                    GetVolumeInformationW / GetDriveTypeW, GetLocalTime, Explorer open/reveal
  ▲
filesystem          Extended-path metadata (GetFileAttributesExW), CreateFileW lock probing,
                    FILE_ATTRIBUTE_READONLY toggling, no-overwrite MoveFileExW, atomic temp-file replacement
  ▲
security            validate_scan_root, protected_path_warning, validate_single_name, TOCTOU verify_rename_pair
  ▲
scanner             Bounded-parallel FindFirstFileExW / FindNextFileW worker pool with cancellation & reparse skip
  ▲
planner             Deterministic rename planner: leaf-first trimming, SHA-1 ~xxxx suffixes, collision & case resolution
  ▲
storage             1.x-compatible PascalCase JSON batch history (history\<id>.json) + append-only rename-log.csv
  ▲
repair              Renamer::dry_run, deepest-first apply_with_progress (step-by-step persistence), reverse undo
  ▲
config / logging / export
```

## 2. Core Dataflows

### Scan -> Plan -> Query (`AppState::session`)

1. The UI invokes `start_scan(root_input, limit, kinds)`.
2. `validate_scan_root` lexically normalizes the path (`C:\...` or `\\server\share\...`), maps to `\\?\` / `\\?\UNC\`, and confirms it exists and is a directory.
3. `AppState::try_begin_operation("scan")` acquires the single-operation mutex/flag (`busy`) so concurrent scans, repairs, or undos cannot race.
4. A background thread calls `scanner::scan_with_progress` using bounded parallel directory workers (`FindFirstFileExW` with `FindExInfoBasic | FIND_FIRST_EX_LARGE_FETCH`).
   - Reparse points (symlinks, junctions, mount points with `FILE_ATTRIBUTE_REPARSE_POINT`) are recorded as nodes so users can inspect them, **but never descended into**.
   - `AtomicBool` cancellation halts workers promptly without panicking.
   - Progress events (`scan-progress`) stream counts cleanly without pushing the entire node vector across IPC.
5. `Planner::build` inspects all discovered nodes in memory:
   - Measures paths in **UTF-16 code units** (matching Win32 `MAX_PATH = 259` visible chars and NTFS/ReFS/exFAT/FAT32 `255`-unit segment limits).
   - Evaluates single-segment issues and sibling case/NFC collisions.
   - Plans longest/invalid paths by cleaning invalid characters/dots/spaces/reserved stems first, trimming the leaf filename first, then ancestor directories from deepest upward using deterministic `~xxxx` (4 hex chars of SHA-1 of the relative path) while preserving file extensions and a minimum `8`-char base length.
   - Resolves naming collisions dynamically (`" (2)"`, `" (3)"`, …) without overwriting existing clean siblings.
   - Flags unfixable deep paths honestly (`unresolved`) instead of proposing truncated invalid operations.
6. The completed `ScanSession` stays in backend `AppState::session` memory. The frontend fetches paginated slices via `query_findings({ search, issueFilter, sortBy, offset, limit })`, keeping memory pressure low even on 100k+ entry trees.

### Dry-Run Preview -> Deepest-First Repair -> Undo

1. `preview_repair(selected_rels)` executes `Renamer::dry_run`, classifying every selected `RenameOp` live on disk (`Ready`, `ReadOnly`, `TargetAlreadyExists`, `SourceMissing`, `LockedByProcess`, `InvalidBoundary`).
2. `execute_repair(selected_rels, clear_readonly)` spawns a worker thread calling `Renamer::apply_with_progress`:
   - Re-verifies every rename via `security::verify_rename_pair` right before touching disk.
   - Optionally clears `FILE_ATTRIBUTE_READONLY` (restoring it on failure/completion as appropriate when `clear_readonly` is enabled).
   - Executes `MoveFileExW(from_ext, to_ext, 0)` (refusing to overwrite), verifies the renamed item exists on disk afterward, appends to `rename-log.csv`, and updates `%APPDATA%\LongPathFix\history\<id>.json` **after each rename** so cancellation or power loss never loses the undo record.
   - Emits `repair-progress` and `repair-complete`, then re-scans cleanly to resynchronize UI state.
3. `undo_batch(id, clear_readonly)` validates `id` strictly via `validate_batch_id`, loads the batch JSON (preserving 1.x PascalCase compatibility), and restores names in reverse order (`Parent` folders first so children remain reachable).

## 3. Windows Integration & Embedding

- **`crates/longpathfix-app/build.rs`** embeds `app.manifest` cleanly into the PE image, declaring:
  - `requestedExecutionLevel level="asInvoker"` (standard unprivileged startup)
  - `dpiAwareness = PerMonitorV2`
  - `<longPathAware xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">true</longPathAware>`
- **Startup hardening (`crates/longpathfix-app/src/main.rs`)** invokes `SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32)` before creating any window or loading plugins, and handles the non-GUI `--enable-long-paths` path when relaunched elevated via user-initiated UAC.
- **Explicit Distinction in Engine & UI:**
  1. *Path syntax:* Classic (`C:\...`) vs Extended (`\\?\C:\...`, `\\?\UNC\...`).
  2. *Filesystem rules:* 255 UTF-16 segment limits and character rules (`GetVolumeInformationW`).
  3. *Process manifest:* `RT_MANIFEST` resource check for `<longPathAware>true</longPathAware>`.
  4. *OS policy:* Live registry read of `HKLM\SYSTEM\CurrentControlSet\Control\FileSystem\LongPathsEnabled`.

## 4. Storage Layout (`%APPDATA%\LongPathFix`)

| Path | Format | Purpose |
| --- | --- | --- |
| `settings.json` | UTF-8 JSON (atomic write) | Theme, default limit (`40`–`1000`), rule bitflags, read-only behavior, up to `8` recent folders |
| `history\<id>.json` | UTF-8 JSON (1.x PascalCase schema) | Applied rename batch records & `Undone` state |
| `rename-log.csv` | UTF-8 append-only CSV | Persistent chronological audit trail of renames and undos |
| `logs\longpathfix.log` | JSON Lines (2 MB rotation) | Structured diagnostic telemetry (`info`, `warn`, `error`) local to machine |
| `error.log` | Plain text | Human-readable fallback log compatible with v1.x support troubleshooting |
