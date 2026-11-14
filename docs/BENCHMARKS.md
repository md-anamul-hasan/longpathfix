# LongPathFix 2.0 Performance & Size Benchmarks

Benchmarks compare the legacy `.NET 10 / WPF` (`1.1.0`) implementation against the native Rust + Tauri 2 (`2.0.0`) implementation on the same Windows workstation.

## 1. Engine Throughput & Peak Working Set (`103,821` Entries)

**Test fixture (`crates/longpathfix-core/examples/bench.rs`):**
- Synthetic NTFS directory tree on local storage (`D:\lpf-bench-tree`)
- `entries = 103,821` total files & directories
- `findings = 13,000` rule violations / long paths
- `ops = 13,000` planned rename operations
- Metrics measured: parallel directory scan time (`scan_ms`), pure rename-plan computation (`plan_ms`), combined total (`total_ms`), and OS process peak working set (`peak_ws_mb`).

### Baseline: Legacy .NET 10 Engine

| Run | Scan (ms) | Plan (ms) | Total (ms) | Peak Working Set (MB) |
| --- | ---: | ---: | ---: | ---: |
| Run 0 | 157 | 386 | 543 | 111 |
| Run 1 | 132 | 368 | 500 | 146 |
| Run 2 | 124 | 284 | 408 | 150 |

### Native Rust Engine (`v2.0.0`, Release LTO Build, 3 Invocations × 3 Runs)

| Invocation | Run | Scan (ms) | Plan (ms) | Total (ms) | Peak Working Set (MB) |
| --- | --- | ---: | ---: | ---: | ---: |
| Invocation 0 | Run 0 | 132 | 146 | 278 | 60 |
| Invocation 0 | Run 1 | 45 | 145 | 191 | 61 |
| Invocation 0 | Run 2 | 44 | 142 | 186 | 62 |
| Invocation 1 | Run 0 | 54 | 150 | 205 | 60 |
| Invocation 1 | Run 1 | 45 | 145 | 191 | 61 |
| Invocation 1 | Run 2 | 45 | 145 | 191 | 61 |
| Invocation 2 | Run 0 | 48 | 146 | 195 | 60 |
| Invocation 2 | Run 1 | 44 | 142 | 186 | 61 |
| Invocation 2 | Run 2 | **42** | **141** | **184** | **61** |

### Engine Summary (Warm Best Run: `.NET 10` Run 2 vs Rust Inv. 2 Run 2)

- **Parallel scan speed:** `124 ms` -> `42 ms` (**2.95× faster**) via bounded-parallel Win32 `FindFirstFileExW` (`FindExInfoBasic` + `FIND_FIRST_EX_LARGE_FETCH`) with zero managed allocations or GC stalls.
- **Rename planner speed:** `284 ms` -> `141 ms` (**2.01× faster**).
- **Combined scan + plan latency:** `408 ms` -> `184 ms` (**2.22× faster**; cold start `543 ms` -> `195–278 ms`).
- **Peak working set:** `150 MB` -> `61 MB` (**~2.46× lower memory footprint**, flat across repeated runs).

## 2. Distribution & Binary Footprint

| Artifact | Legacy `.NET 10` (`1.1.0`) | Rust + Tauri 2 (`2.0.0`) | Reduction |
| --- | ---: | ---: | --- |
| Standalone executable (`LongPathFix.exe`) | 61,675,812 bytes (~58.82 MiB) | 5,140,480 bytes (~4.90 MiB) | **12.0× smaller** (-91.7%) |
| Windows installer (`LongPathFix-Setup-*.exe`) | 56,652,556 bytes (~54.03 MiB) | 3,644,513 bytes (~3.48 MiB) | **15.5× smaller** (-93.6%) |
| Portable archive (`LongPathFix-2.0.0-win64-portable.zip`) | — | 2,100,358 bytes (~2.00 MiB) | New in 2.0.0 |
