# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

(Rendered inside a native Windows desktop shell: Tauri 2 with WebView2. The design language follows Windows 11, not a website.)

## Users

People whose backup, ZIP, cloud sync or copy jobs fail because of over-long paths or bad file and folder names: power users, IT and helpdesk staff, and anyone moving files between Windows and macOS, Linux, a NAS or a network share. They arrive with a concrete failure ("the copy stopped at 240 files", "OneDrive will not sync this folder") and want it fixed without losing data.

## Product Purpose

LongPathFix finds the names that break file tools on Windows (paths over the limit, names over 255 characters, trailing dots and spaces, leading spaces, reserved device names such as CON or COM1, illegal characters, names that differ only by case) and renames them safely. Success: the user sees exactly what is wrong and exactly what will change, approves it, and ends with a folder that copies, zips and syncs, with every rename logged and undoable.

## Positioning

A planner, not a bulk renamer: the full rename plan is computed before anything touches the disk, names are only shortened as much as needed (file name first, then folders from the deepest up, stable `~xxxx` suffixes, extensions kept), it says honestly when renaming cannot fix a path, and every batch can be undone. Free and open source (MIT), no network, no account.

## Operating Context

- Runs on Windows 10 (21H2+) and Windows 11, x64, as a normal (non-elevated) user.
- Typical targets: project folders, user documents, extracted archives, synced folders, NAS and UNC shares, folders copied from macOS/Linux.
- Sessions are task-shaped: pick a folder, scan, review, rename, verify, maybe undo later from History.
- Scans can cover hundreds of thousands of entries; the UI must stay responsive and cancellable.

## Capabilities and Constraints

- Finds: path over a chosen limit (259 classic, 240 safe for ZIP/backup, 200 strict, or a custom value), name over 255 UTF-16 units, trailing dot/space, leading space, reserved names, illegal characters and control characters, case and Unicode-normalization clashes, names that are not valid Unicode.
- Plans renames, previews them, checks them against the disk (dry run), applies them with no-overwrite renames, logs each one as it happens and undoes a batch.
- Lists links, junctions and other reparse points but never enters them.
- Detects and explains the Windows long-path policy (`LongPathsEnabled`) and offers an explicit, UAC-prompted "Enable" that sets only that value.
- Exports findings as CSV.
- Only names change: nothing is moved between folders, deleted or rewritten.
- Terminology: "scan", "finding", "plan", "rename", "batch", "undo", "limit". Lengths are counted in UTF-16 code units, the unit Windows uses.

## Brand Commitments

- Name: LongPathFix. Publisher: Md Anamul Hasan. License: MIT.
- Existing icon: `crates/longpathfix-app/icons/` (carried over from `Assets/icon.svg`).
- Voice: plain, short, factual English; says what happens and what does not; no hype.
- The owner's gold standard for desktop UI/UX and release process is WDM (`D:\00_DEV\Projects\wdm`).

## Evidence on Hand

- Real behaviour and copy: `docs/USER_GUIDE.md`, `README.md`, the 1.1.0 test-suite (35 tests) ported to Rust.
- No customers, testimonials, download counts or benchmarks beyond those measured in `docs/BENCHMARKS.md`. Do not invent any.

## Product Principles

1. Nothing changes on disk without an explicit, informed confirmation.
2. Show the whole truth: every finding, every planned name, and what cannot be fixed and why.
3. Every change is reversible or, when it cannot be, says so.
4. Precise over clever: the smallest rename that fixes the problem.
5. Private by construction: no network, no telemetry.

## Accessibility & Inclusion

WCAG 2.2 AA: full keyboard operation, visible focus, screen-reader names and live regions for progress and results, reduced-motion support, Windows display scaling and high contrast respected.
