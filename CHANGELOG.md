# Changelog

All notable changes to LongPathFix. Versioning follows [Semantic Versioning](https://semver.org/); the version lives in `Directory.Build.props`.

## [1.1.0] - 2026-10-22

First open-source release (MIT). Earlier builds were a paid product; this release removes licensing entirely, so every feature is free.

### Added
- Finds long paths, over-long names, trailing dots and spaces, reserved names, illegal characters and case clashes.
- Shows the full rename plan before anything changes; limits of 259, 240 or 200 characters.
- Renames safely (only names change), logs every rename and undoes a batch from the History page.
- Exports the findings as CSV.
- Light and dark themes; self-contained installer.

### Removed
- Licence-key activation, the licence dialog, evaluation limits and the licence-server connection. Everything is unlocked.
