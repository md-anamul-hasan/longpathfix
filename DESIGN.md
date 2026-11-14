---
name: LongPathFix
description: Precision Windows filesystem diagnostic and remediation workbench for over-long paths and invalid filenames.
colors:
  cobalt-accent: "#1d6fe8"
  cobalt-accent-hover: "#1761cf"
  sky-accent-text: "#72b2ff"
  focus-sky: "#7ab8ff"
  cobalt-light: "#0f62d6"
  cobalt-light-hover: "#0b51b5"
  emerald-ok: "#2ea05a"
  mint-ok-text: "#6de39c"
  forest-ok-light: "#157347"
  pine-ok-text-light: "#115e3a"
  amber-warn: "#d4891c"
  gold-warn-text: "#f6c162"
  ochre-warn-light: "#9a5b00"
  bronze-warn-text-light: "#804b00"
  crimson-danger: "#d94a53"
  coral-danger-text: "#ff949a"
  brick-danger-light: "#c02a21"
  garnet-danger-text-light: "#9e1e16"
  gauge-before: "#c95b63"
  gauge-after: "#3ca367"
  gauge-before-light: "#c8484f"
  gauge-after-light: "#1d8a55"
  obsidian-canvas: "#111317"
  slate-inset: "#13161c"
  graphite-rail: "#15181e"
  charcoal-surface: "#191d24"
  steel-elevated: "#20252f"
  slate-hover: "#252b36"
  gauge-track-dark: "#222731"
  hairline-dark: "#282e39"
  hairline-strong-dark: "#394252"
  frost-ink: "#f0f3f7"
  pewter-ink: "#a8b2c1"
  ash-ink: "#8490a3"
  porcelain-canvas: "#f4f6f9"
  mist-rail: "#eceff4"
  pure-surface: "#ffffff"
  cloud-elevated: "#f8f9fb"
  fog-inset: "#f0f2f6"
  vapor-hover: "#e7ebf2"
  gauge-track-light: "#e4e8ef"
  hairline-light: "#d8dee7"
  hairline-strong-light: "#b8c2d1"
  ink-charcoal: "#111827"
  slate-ink-secondary: "#435061"
  slate-ink-muted: "#596579"
typography:
  display:
    fontFamily: '"Segoe UI Variable Display", "Segoe UI", system-ui, sans-serif'
    fontSize: "20px"
    fontWeight: 600
    lineHeight: 1.25
    letterSpacing: "-0.015em"
  headline:
    fontFamily: '"Segoe UI Variable Display", "Segoe UI", system-ui, sans-serif'
    fontSize: "16px"
    fontWeight: 600
    lineHeight: 1.25
    letterSpacing: "0em"
  title:
    fontFamily: '"Segoe UI Variable Text", "Segoe UI", system-ui, -apple-system, sans-serif'
    fontSize: "14.5px"
    fontWeight: 600
    lineHeight: 1.48
    letterSpacing: "0em"
  body:
    fontFamily: '"Segoe UI Variable Text", "Segoe UI", system-ui, -apple-system, sans-serif'
    fontSize: "13.5px"
    fontWeight: 400
    lineHeight: 1.48
    letterSpacing: "0em"
  label:
    fontFamily: '"Segoe UI Variable Text", "Segoe UI", system-ui, -apple-system, sans-serif'
    fontSize: "12px"
    fontWeight: 600
    lineHeight: 1.45
    letterSpacing: "0em"
  caption:
    fontFamily: '"Segoe UI Variable Text", "Segoe UI", system-ui, -apple-system, sans-serif'
    fontSize: "11px"
    fontWeight: 500
    lineHeight: 1.4
    letterSpacing: "0em"
  mono:
    fontFamily: '"Cascadia Mono", "Consolas", ui-monospace, monospace'
    fontSize: "12px"
    fontWeight: 400
    lineHeight: 1.45
    letterSpacing: "0em"
rounded:
  sm: "6px"
  md: "8px"
  lg: "10px"
  pill: "999px"
spacing:
  space-2: "8px"
  space-3: "12px"
  space-4: "16px"
  space-5: "20px"
  space-6: "24px"
components:
  button-primary:
    backgroundColor: "{colors.cobalt-accent}"
    textColor: "{colors.pure-surface}"
    rounded: "{rounded.sm}"
    height: "34px"
    padding: "0 13px"
  button-primary-hover:
    backgroundColor: "{colors.cobalt-accent-hover}"
  button-secondary:
    backgroundColor: "{colors.steel-elevated}"
    textColor: "{colors.frost-ink}"
    rounded: "{rounded.sm}"
    height: "34px"
    padding: "0 13px"
  button-secondary-hover:
    backgroundColor: "{colors.slate-hover}"
  button-danger:
    textColor: "{colors.coral-danger-text}"
    rounded: "{rounded.sm}"
    height: "34px"
    padding: "0 13px"
  button-ghost:
    textColor: "{colors.pewter-ink}"
    rounded: "{rounded.sm}"
    height: "34px"
    padding: "0 13px"
  input-field:
    backgroundColor: "{colors.slate-inset}"
    textColor: "{colors.frost-ink}"
    rounded: "{rounded.sm}"
    height: "34px"
    padding: "0 11px"
  surface-panel:
    backgroundColor: "{colors.charcoal-surface}"
    textColor: "{colors.frost-ink}"
    rounded: "{rounded.lg}"
    padding: "16px"
  filter-pill:
    backgroundColor: "{colors.steel-elevated}"
    textColor: "{colors.pewter-ink}"
    rounded: "{rounded.pill}"
    height: "26px"
    padding: "0 9px"
  filter-pill-active:
    textColor: "{colors.sky-accent-text}"
  status-tag:
    backgroundColor: "{colors.steel-elevated}"
    textColor: "{colors.pewter-ink}"
    rounded: "{rounded.pill}"
    padding: "1px 7px"
  nav-rail:
    backgroundColor: "{colors.graphite-rail}"
    width: "224px"
    padding: "16px 12px"
  nav-item:
    textColor: "{colors.pewter-ink}"
    rounded: "{rounded.md}"
    padding: "9px 10px"
  nav-item-active:
    backgroundColor: "{colors.charcoal-surface}"
    textColor: "{colors.frost-ink}"
---

# Design System: LongPathFix

## Overview

**Creative North Star: "The Calibrated Filesystem Workbench"**

LongPathFix is a native Windows desktop utility rendered inside Tauri 2 + WebView2, designed around the precision and restraint of Windows Terminal, Task Manager, and Sysinternals tooling. Because users arrive with failing backups, broken ZIP extractions, or stalled cloud syncs—and because every rename touches real paths on disk—the interface replaces decorative marketing chrome with immediate diagnostic clarity: a fixed navigation rail, crisp `1px` hairline structural rules, tabular UTF-16 path measurements, and a calibrated before/after distribution ruler (`PathGauge`) sitting directly above a high-density findings table.

The system defaults to a low-glare obsidian/slate dark palette (`color-scheme: dark`) and mirrors every surface, hairline, ink, and status token into a high-contrast porcelain light theme (`data-theme="light"`), tracking Windows system preference when set to Auto. Consistent with the native Windows 11 desktop brief, LongPathFix rejects decorative backdrop blur, neon gradients, uppercase eyebrow kickers above headings, and modal interruptions for routine inspection.

**Key Characteristics:**
- Native Windows 11 `operate` workspace with persistent `224px` left rail (`--rail-width`) and full-viewport `100vh` scroll-contained layout.
- Dual-theme CSS custom properties (`[data-theme="dark"]` and `[data-theme="light"]`) built around neutral slate/porcelain tiers and a single Windows Cobalt primary accent.
- Calibrated UTF-16 code-unit measurement everywhere paths appear, pairing `Cascadia Mono` with `font-variant-numeric: tabular-nums`.
- Tonal layering and `1px` hairline borders at rest; drop shadows appear strictly on floating popovers and focus-trapped modals.
- Full keyboard operation, global `:focus-visible` rings, `prefers-reduced-motion` support, and `aria-live` progress/status announcements.

## Colors

A dual-theme neutral palette grounded in obsidian/slate (dark) and cool porcelain (light), punctuated by a single Windows Cobalt primary accent and three calibrated semantic status families.

### Primary
- **Cobalt Accent** (`#1d6fe8` dark, hover `#1761cf`; `#0f62d6` light, hover `#0b51b5`): Reserved for primary affirmative actions (`Scan Folder`, `Preview & Fix`, `Apply Safe Renames`, `Undo Batch`, `Done`), active checkboxes (`accent-color`), progress fill bars, `::selection` highlights, and the `.skip-link` focus pill, paired exclusively with pure white ink (`#ffffff`, 4.68:1 dark / 5.62:1 light).
- **Sky Accent Text** (`#72b2ff` dark; `#0b51b5` light): Accessible accent ink used for folder glyphs in table rows, active filter-pill labels, `PathGauge` threshold markers, and the brand mark inside `--accent-tint` (`rgba(29, 111, 232, 0.14)` dark / `rgba(15, 98, 214, 0.09)` light) and `--accent-border` (`rgba(114, 178, 255, 0.34)` dark / `rgba(15, 98, 214, 0.30)` light).
- **Focus Sky** (`#7ab8ff` dark; `#0b51b5` light): High-visibility `2px` `:focus-visible` outline ring (8.15:1 dark / 7.35:1 light on primary surfaces).

### Neutral
- **Dark Workbench Surfaces**: Stepped neutral grays avoid pure black crush and establish containment without heavy shadows: **Obsidian Canvas** (`#111317`) viewport ground, **Slate Inset** (`#13161c`) recessed inputs, segmented tracks, and copyable path wells, **Graphite Rail** (`#15181e`) navigation rail, **Charcoal Surface** (`#191d24`) cards, tables, and modals, **Steel Elevated** (`#20252f`) sticky headers, inspector drawer, default buttons, and table footers, and **Slate Hover** (`#252b36`) hover fill.
- **Light Porcelain Surfaces**: Mirrored cool neutrals keep high daylight legibility: **Porcelain Canvas** (`#f4f6f9`) viewport ground, **Fog Inset** (`#f0f2f6`) input wells and path blocks, **Mist Rail** (`#eceff4`) navigation rail, **Pure Surface** (`#ffffff`) cards, tables, and modals, **Cloud Elevated** (`#f8f9fb`) sticky headers, drawer, and secondary buttons, and **Vapor Hover** (`#e7ebf2`) hover fill.
- **Dark & Light Ink Ramp**: **Frost Ink** (`#f0f3f7` dark, 15.19:1 on surface) / **Ink Charcoal** (`#111827` light, 17.74:1 on surface) for headings, paths, and primary copy; **Pewter Ink** (`#a8b2c1` dark, 7.89:1) / **Slate Ink Secondary** (`#435061` light, 8.21:1) for subtitles, nav items, and table headers; **Ash Ink** (`#8490a3` dark, 5.23:1) / **Slate Ink Muted** (`#596579` light, 5.90:1) for placeholders, metric captions, and key-value labels.
- **Hairline Borders & Selection**: **Hairline Dark / Light** (`#282e39` / `#d8dee7`) for `1px` panel, row, and drawer boundaries; **Hairline Strong Dark / Light** (`#394252` / `#b8c2d1`) for interactive button/input strokes, sticky table dividers, active nav outlines, and `<kbd>` borders; translucent row selection tint (`rgba(45, 130, 240, 0.16)` dark / `rgba(15, 98, 214, 0.10)` light).

### Semantic & Data Visualization (PathGauge)
- **Success / Planned Fix**: **Emerald Ok** (`#2ea05a` dark) / **Forest Ok Light** (`#157347` light) for the policy-active dot; **Mint Ok Text** (`#6de39c` dark) / **Pine Ok Text Light** (`#115e3a` light) over translucent `--ok-tint` and `--ok-border` for positive KPIs, valid replacement checks, and planned rename pills.
- **Warning / Over-Threshold**: **Amber Warn** (`#d4891c` dark) / **Ochre Warn Light** (`#9a5b00` light) for policy-off indicators; **Gold Warn Text** (`#f6c162` dark) / **Bronze Warn Text Light** (`#804b00` light) over `--warn-tint` and `--warn-border` for path-over-limit warnings, read-only flags, and manual-review badges.
- **Danger / Structural Violation**: **Crimson Danger** (`#d94a53` dark) / **Brick Danger Light** (`#c02a21` light) for destructive hover fills and error status; **Coral Danger Text** (`#ff949a` dark) / **Garnet Danger Text Light** (`#9e1e16` light) over `--danger-tint` and `--danger-border` for segment >255, illegal characters, reserved DOS names, invalid UTF-16, and dry-run conflicts.
- **PathGauge Distribution Bars**: **Gauge Before** (`#c95b63` dark / `#c8484f` light) and **Gauge After** (`#3ca367` dark / `#1d8a55` light) drawn over **Gauge Track** (`#222731` dark / `#e4e8ef` light), calibrated at `3.55:1–4.72:1` non-text contrast.

### Named Rules
**The One Primary Accent Rule.** Cobalt (`--accent`) marks only primary actions, selection, active checkboxes, and progress; semantic colors (emerald, amber, crimson) never replace cobalt on primary workflow buttons, and cobalt never signals severity.
**The Tinted Ink Pairing Rule.** Colored status labels (`--ok-text`, `--warn-text`, `--danger-text`, `--accent-text`) always pair with their translucent `--*-tint` and `--*-border` tokens on neutral surfaces so every badge and banner maintains >= 6.3:1 contrast in both dark and light modes.

## Typography

**Display Font:** Segoe UI Variable Display (with `Segoe UI`, `system-ui`, `sans-serif`)
**Body Font:** Segoe UI Variable Text (with `Segoe UI`, `system-ui`, `-apple-system`, `sans-serif`)
**Label/Mono Font:** Cascadia Mono (with `Consolas`, `ui-monospace`, `monospace`)

**Character:** Pure native Windows 11 typography that renders crisply at 100%–200% DPI scaling inside WebView2 with zero external font loading, pairing slightly tightened variable display headings with dense `13.5px` UI text and aligned `Cascadia Mono` path telemetry.

### Hierarchy
- **Display** (`600`, `20px`, `1.25` line-height, `-0.015em` tracking): Primary view headings (`h1.view-title`) across `Scan & Fix`, `Path Lab & System Diagnostics`, `History & Undo`, and `Settings & Guide`.
- **Headline** (`600`, `16px`, `1.25` line-height): Modal dialog titles (`h2.modal-title`) and the `15px` (`1.2` line-height) sidebar wordmark (`brand-name`), rendered in `Segoe UI Variable Display`.
- **Title** (`600`, `14.5px`, `1.48` line-height): Section headings inside panels (`h2.section-heading`, `h3.section-heading`) and the `13.5px` (`600`) `PathInspectorDrawer` header (`h2.drawer-title`).
- **Body** (`400`, `13.5px`, `1.48` line-height): Root UI copy, banners, table cells (`12.5px`), and explanatory Windows path documentation (`muted-note` at `12.5px` / `1.5` line-height; explanatory copy constrained to `480px` max width in empty states).
- **Label** (`600`, `12px`, `1.45` line-height): Control group labels (`12px`, `600`), table column headers (`11.5px`, `600`), filter pills, and secondary compact buttons (`12px`).
- **Caption** (`500`, `11px`, `1.4` line-height): Micro-metadata across status tags (`.tag`), key-value field captions (`.kv-k`), code-block headers (`.code-block-head`), `PathGauge` bucket captions, sidebar version, navigation badges, and `<kbd>` badges.
- **Mono** (`400`/`600`, `12px`–`11px`, `1.45` line-height, `tabular-nums`): Filesystem paths, UTF-16 counts, `PathGauge` KPI metrics (`18px`, `600`), bucket ranges (`11px`), navigation count badges (`11px`), preset limit toggles (`12px`), and `<kbd>` shortcut chips (`11px`).

### Named Rules
**The Tabular Measurement Rule.** Every numeric counter, length delta, bucket tally, and timestamp applies `font-variant-numeric: tabular-nums` so scanning down columns and updating live scan counts never shifts horizontal alignment.
**The Honest Monospace Rule.** `Cascadia Mono` is reserved strictly for filesystem paths, UTF-16 code-unit lengths, registry keys, batch IDs, and keycaps—never as decorative styling on headings or body prose.

## Layout

LongPathFix uses a fixed full-viewport CSS Grid workspace (`height: 100vh; overflow: hidden`) divided into a persistent left navigation rail (`--rail-width: 224px`, shipped 4px wider than the 220px contract target) and a flex-column main pane (`minmax(0, 1fr)`). Vertical scrolling happens inside `.scroll-view` (`padding: 20px 24px` via `--space-5` and `--space-6`, `gap: 16px` via `--space-4`), keeping the rail, active tab state, and policy indicator anchored during long table reviews.

Within `Scan & Fix`, vertical rhythm stacks in strict task order: header (`view-header`), optional dismissible status banner, target folder bar & rule controls (`surface.scan-controls`), optional live scan progress strip (`surface-tight.progress-strip`), `PathGauge` distribution card, and the expandable `results-shell` (`min-height: 360px`) pairing the scrollable `data-table` (`table-layout: fixed`, `min-width: 640px`, `42px` checkbox column) alongside the optional `390px` `PathInspectorDrawer` (`--drawer-width: 390px`). Secondary views (`Path Lab & System`, `Settings & Guide`) arrange paired panels on a 2-column grid (`.two-col-grid`, `gap: 16px`), while inspector summaries use compact 2-column key-value grids (`.kv-grid`, `gap: 8px`).

**Responsive Breakpoints:**
- **`<= 1360px`**: `.inspector-drawer` detaches from the flex split into a right-docked overlay (`position: absolute; width: min(390px, 92vw); z-index: 25`) elevated with `--shadow-modal` so the findings table retains full horizontal column width.
- **`<= 1080px`**: `.two-col-grid` collapses from two columns to a single stacked column (`1fr`).
- **`<= 760px`**: `.app-shell` releases `100vh` lock and stacks vertically (`grid-template-columns: 1fr`), `.rail` becomes a compact wrapping top header (`padding: 10px 12px`), `.scroll-view` tightens to `14px` padding, `.path-row` stacks vertically, `.histogram-grid` folds from 6 buckets to 3 (`repeat(3, minmax(0, 1fr))`), and `.search-input` spans `100%` width.

### Named Rules
**The First-Viewport Workbench Rule.** `Scan & Fix` opens directly into interactive scan controls, threshold presets (`259`, `240`, `200`, `Custom`), fix toggles, and the findings container with zero marketing splash or interstitial onboarding.

## Elevation & Depth

Surfaces sit on a flat tonal hierarchy rather than stacked drop shadows. Depth between adjacent regions—canvas (`--bg-canvas`), rail (`--bg-rail`), primary cards (`--bg-surface`), elevated headers/drawers (`--bg-elevated`), and recessed code/input wells (`--bg-inset`)—is established strictly through subtle luminance stepping and `1px solid var(--border)` hairline borders (`--border-strong` for control edges and sticky `thead th` boundaries). Decorative glassmorphism and `backdrop-filter` blur are absent across the entire system.

### Shadow Vocabulary
- **Popover Shadow** (`--shadow-popover`: `0 10px 28px rgba(0, 0, 0, 0.42)` dark / `0 10px 26px rgba(17, 24, 39, 0.12)` light): Applied exclusively to the floating `.recent-menu` folder picker dropdown (`z-index: 40`).
- **Modal Shadow** (`--shadow-modal`: `0 20px 56px rgba(0, 0, 0, 0.55)` dark / `0 20px 52px rgba(17, 24, 39, 0.20)` light): Applied exclusively to `.modal-panel` inside the `.modal-backdrop` scrim (`rgba(8, 10, 14, 0.62)`, `z-index: 100`) and to `.inspector-drawer` when overlaid below `1360px` (`z-index: 25`).

### Named Rules
**The Flat-By-Default Rule.** Cards, tables, rail items, and drawers stay flat at rest, separated by tonal surface steps and `1px` hairlines; shadows appear only when an overlay (`recent-menu`, `RepairPreviewModal`, or `<= 1360px` overlay drawer) breaks out of document flow.

## Shapes

LongPathFix uses crisp, restrained corner rounding matched to Windows 11 control hierarchy across four named tokens:
- **Small (`--radius-sm`: `6px`)**: Interactive controls (`.btn`, `.input`, `.select`), `.seg-control`, `.bucket-bars`, `.kv-cell`, `.code-block-box`, `.segment-row`, `.shortcut-row`, and `.skip-link`.
- **Medium (`--radius-md`: `8px`)**: Navigation and mid-level containers (`.brand-mark`, `.nav-item`, `.policy-pill`, `.theme-switch`, `.banner`, `.recent-menu`, `.empty-icon`).
- **Large (`--radius-lg`: `10px`)**: Top-level workspace containers (`.surface`, `.surface-tight`, `.results-shell`, `.modal-panel`, `.history-item`).
- **Pill (`--radius-pill`: `999px`)**: Compact status and filter badges (`.nav-badge`, `.filter-pill`, `.tag`, `.progress-track`, `.progress-fill`, thin `8px` custom scrollbars).

All 18 custom inline SVGs (`Icons.tsx`) share a `24x24` coordinate grid with uniform `1.6` stroke weight (shipped at `strokeWidth={1.6}`, `0.1px` heavier than the 1.5px direction estimate), round caps/joins, `fill="none"`, and `aria-hidden="true"`.

## Components

### Buttons
Compact `34px` (`--radius-sm` `6px`) action controls (`28px` for `.btn-sm` inside toolbars, banners, and table footers), with `120ms` cubic-bezier transitions (`--transition-fast`) and `opacity: 0.45` disabled state.
- **Shape:** Subtle rounded rectangle (`6px` radius via `--radius-sm`).
- **Primary:** Solid Cobalt (`--accent`), `1px` matching border, `#ffffff` ink (`font-weight: 600`), `0 13px` horizontal padding (`height: 34px`).
- **Hover / Focus:** Shifts to `--accent-hover` (`#1761cf` dark / `#0b51b5` light); keyboard focus shows a global `2px solid var(--focus-ring)` outline with `2px` offset.
- **Secondary / Danger / Ghost:** Default `.btn` uses `--bg-elevated`, `1px solid var(--border-strong)`, and `--text-primary`, hovering to `--bg-hover`. `.btn-danger` rests in `--danger-tint` with `--danger-border` and `--danger-text`, elevating to solid `--danger` on hover. `.btn-ghost` uses a transparent background/border with `--text-secondary` for low-emphasis close/copy/dismiss actions.

### Inputs / Fields
Recessed `34px` single-line fields (`--radius-sm` `6px`) engineered for long Windows and UNC paths.
- **Style:** `--bg-inset` background, `1px solid var(--border-strong)` stroke, `0 11px` padding (`0 34px 0 11px` when right-cleared; `0 10px 0 30px` in `.search-input` with absolute left `SearchIcon`). `.input-mono` switches to `Cascadia Mono` at `12.5px`.
- **Focus / Hover:** Hover deepens border to `--text-muted`; `:focus-visible` renders the global `2px` `--focus-ring` outline.
- **Segmented Presets & Toggles:** `.seg-control` wraps `26px`-tall `.seg-btn` threshold presets (`259`, `240`, `200`, `Custom`) inside a `--bg-inset` tray; `.check-pill` pairs native `14px` checkboxes (`accent-color: var(--accent)`) with plain labels.

### Chips & Status Tags
Pill-shaped (`--radius-pill` `999px`) category filters and inline severity badges.
- **Filter Pills (`.filter-pill`):** `26px` height, `0 9px` padding, `--bg-elevated`, `1px solid var(--border)`, optional `.tone-warn` / `.tone-danger` border and text tint; `.active` state shifts to `--accent-tint`, `--accent-border`, and `--accent-text` (`font-weight: 600`).
- **Status Tags (`.tag`):** `11px` (`font-weight: 500`), `1px 7px` padding, single-line `.truncate` overflow clipping, styled in neutral or `.ok` / `.warn` / `.danger` / `.accent` tint-border-text triples.

### Cards / Containers & Banners
- **Corner Style:** `10px` radius (`--radius-lg`) on `.surface`, `.surface-tight`, `.results-shell`, and `.history-item`; `8px` (`--radius-md`) on `.banner`.
- **Background:** `--bg-surface` on neutral containers; semantic `--*-tint` + `--*-border` on `.banner` variants (`info`, `warn`, `danger`, `ok`).
- **Shadow Strategy:** Flat at rest (`box-shadow: none`).
- **Border:** `1px solid var(--border)` (`--border-strong` on `.modal-panel`).
- **Internal Padding:** `16px` (`--space-4`) on `.surface`; `12px 16px` on `.surface-tight`; `10px 14px` on `.banner`.

### Navigation
- **Rail Shell (`.rail`):** `224px` wide vertical column on `--bg-rail` with `1px solid var(--border)` right hairline, `16px 12px` padding (`--space-4` `--space-3`). Houses `.brand` (`32px` `--accent-tint` mark + `LongPathFix` + mono `v2.0.0`), 4 `.nav-item` buttons (`9px 10px` padding, `8px` radius, `font-weight: 500`, optional mono `.nav-badge`), and `.rail-footer` holding the `LongPathsEnabled` status pill (`.status-dot.ok`/`.warn`) and 3-way `Auto` / `Dark` / `Light` `.theme-switch`.
- **States:** Inactive items sit transparent on `--text-secondary`; hover fills `--bg-hover` with `--text-primary`; active (`aria-current="page"`) elevates to `--bg-surface` with `1px solid var(--border-strong)` and `font-weight: 600`. Below `760px`, the rail folds into a horizontal wrapping bar.

### PathGauge (Signature Component)
Calibrated path-length instrument (`PathGauge.tsx`) summarizing scan totals, issue counts, planned rename operations, `maxLenBefore → maxLenAfter` UTF-16 ceiling delta, and scan duration above a 6-bucket comparative histogram (`.histogram-grid`). Each bucket renders paired `--bar-before` and `--bar-after` bars (`180ms ease` height transition, minimum `8%` height when non-zero) inside a `54px` `--bar-track` slot (`--accent-border` + `limit` tag marks the bucket containing the active threshold; `--warn-border` marks populated buckets above the limit), followed directly by interactive issue-category filter pills.

### PathInspectorDrawer & RepairPreviewModal
- **PathInspectorDrawer (`390px`):** Right-hand detail pane on `--bg-elevated` displaying a 2-column `.kv-grid`, severity tags, copyable plain and Win32 `\\?\` `.code-block-box` wells, `Reveal in File Explorer`, a per-segment UTF-16 length/issue breakdown (`.segment-list`), and a live replacement filename validator.
- **RepairPreviewModal (`min(940px, 100%)`):** Focus-trapped `role="dialog" aria-modal="true"` (`Esc` to close when idle, Tab cycling, focus restoration) handling pre-flight dry-run status counts, read-only attribute override toggle, live determinate progress bar, and post-repair outcome CSV export / immediate batch undo.

## Do's and Don'ts

### Do:
- **Do** measure and display every path and filename length in UTF-16 code units with `tabular-nums` (`Cascadia Mono`) so Windows API ceilings (`259`, `240`, `200`, `255`-char segment) remain unambiguous.
- **Do** isolate semantic status tokens (`--ok-*`, `--warn-*`, `--danger-*`) to diagnostic states and keep Cobalt (`--accent`) reserved strictly for primary actions, selection, and progress.
- **Do** support both `[data-theme="dark"]` and `[data-theme="light"]` tokens across every custom component and verify `>= 4.5:1` text and `>= 3:1` non-text contrast in both themes.
- **Do** preserve flat tonal containment (`1px solid var(--border)`) at rest and reserve drop shadows exclusively for popovers and modal/overlay dialogs.
- **Do** gate destructive or disk-mutating actions (`Apply Safe Renames`, `Confirm Remove`, `Enable Windows Long Paths`) behind explicit dry-run verification, two-step confirmation, or UAC prompts.

### Don't:
- **Don't** add uppercase eyebrows/kickers above headings, decorative gradient text, or `backdrop-filter` glass blur to workbench panels.
- **Don't** introduce external icon fonts, emoji icons, or webfont requests; draw UI glyphs within the uniform `24x24` `1.6px`-stroke inline SVG system and use native Windows 11 typography (`Segoe UI Variable` + `Cascadia Mono`).
- **Don't** color primary workflow buttons with status colors (`--ok`, `--warn`, `--danger`) or use `--accent` to indicate severity.
- **Don't** drop keyboard focus rings (`:focus-visible`), `prefers-reduced-motion` overrides, or screen-reader live regions (`role="status"`, `aria-live="polite"`) on scan, repair, or modal flows.
