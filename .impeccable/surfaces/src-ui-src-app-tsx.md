---
version: 1
slug: "src-ui-src-app-tsx"
primary_target: "src-ui/src/App.tsx"
related_targets: []
---

## Direction contract

- **THESIS:** Precision Windows filesystem diagnostic and remediation workbench built around a calibrated UTF-16 path-length instrument (`PathGauge`), segment-level issue inspection, dry-run collision verification, and child-first reversible repair.
- **OWN-WORLD:** Native Windows 11 desktop `operate` workspace aligned with Windows Terminal, Task Manager, and Sysinternals tooling: restrained 220px navigation rail, crisp 1px hairline rules, neutral obsidian/slate dark & porcelain light surfaces (no purple-washed neon glass), `tabular-nums` alignment, and high-density scannable tables.
- **STORY:** Built for power users, IT helpdesk engineers, and macOS/Linux/NAS migrators whose backups, ZIP archives, or sync tools break on >260 UTF-16 code-unit paths, trailing dots/spaces, reserved DOS device names, illegal characters, or case clashes; scans via extended `\\?\` APIs, explains Windows `LongPathsEnabled` policy honestly, and repairs without data loss.
- **FIRST VIEWPORT:** `Scan & Fix` opens immediately into an interactive workspace: target folder bar with recents + threshold pills (`259` / `240` / `200` / custom) + 5 fix-rule toggles, the `PathGauge` calibrated before/after distribution ruler, and the filterable/sortable findings table with side `PathInspectorDrawer` and dry-run `RepairPreviewModal`.
- **FORM:** Code-led `operate` architecture (`seed:16f9583a4666847b`, assigned index `7`), token-driven CSS (`tokens.css` + `app.css`) supporting system/dark/light themes, WCAG 2.2 AA contrast, full keyboard navigation, `prefers-reduced-motion`, 1.5px stroke SVGs, styled scrollbars/`::selection`/`:focus-visible`, zero eyebrows/kickers above headings, zero decorative blur, zero placeholder UI.
- **FINISH:** unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance.
