# LongPathFix Privacy Policy

_Last updated: 2026-11-14_

LongPathFix runs **100% offline on your Windows PC**. It makes zero network connections: no telemetry, no analytics, no automatic update checks, no cloud uploads, and no license server calls.

## Data LongPathFix reads

- LongPathFix inspects only filenames, folder structures, filesystem attributes, volume metadata, and Windows registry/manifest policy flags (`LongPathsEnabled`, `CurrentBuildNumber`) when you initiate a scan or open Diagnostics.
- File contents are **never** opened, uploaded, indexed, or modified.

## Data stored locally

All persistent state stays strictly inside your local Windows user profile under `%APPDATA%\LongPathFix`:

- `settings.json` — your theme preference, path limit, rule toggles, and recent folder paths.
- `history\*.json` — step-by-step rename batch records enabling Undo.
- `rename-log.csv` — local append-only audit log of renames and undos.
- `logs\longpathfix.log` and `error.log` — rotated diagnostic logs (auto-rotated at 2 MB).

## Your control

There is no account, telemetry identifier, or hidden background service. Uninstalling LongPathFix removes the application binaries; deleting `%APPDATA%\LongPathFix` permanently erases all local history, settings, and logs.

Questions: contact@mdanamulhasan.com
