import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useEffect, useMemo, useRef, useState } from "react";
import {
  AlertTriangleIcon,
  ArrowRightIcon,
  ChevronDownIcon,
  DownloadIcon,
  FileIcon,
  FolderIcon,
  FolderSearchIcon,
  SearchIcon,
  ShieldCheckIcon,
} from "./Icons";
import { PathGauge } from "./PathGauge";
import { PathInspectorDrawer } from "./PathInspectorDrawer";
import type {
  AppConfig,
  Finding,
  FindingsPageDto,
  IssueFilter,
  PlanSummaryDto,
  RootInspectionDto,
  ScanProgress,
  SortBy,
} from "../types";
import { FIX_KINDS, formatError, issueTone } from "../types";

interface FindingsWorkspaceProps {
  config: AppConfig;
  rootInput: string;
  onChangeRootInput: (val: string) => void;
  limit: number;
  kinds: number;
  onChangePlanOptions: (limit: number, kinds: number) => void;
  scanning: boolean;
  scanProgress: ScanProgress | null;
  summary: PlanSummaryDto | null;
  pageData: FindingsPageDto;
  pageOffset: number;
  pageSize: number;
  search: string;
  issueFilter: IssueFilter;
  sortBy: SortBy;
  onChangeSearch: (s: string) => void;
  onChangeIssueFilter: (f: IssueFilter) => void;
  onChangeSortBy: (s: SortBy) => void;
  onChangePageOffset: (offset: number) => void;
  onStartScan: (root: string, limit: number, kinds: number) => void;
  onCancelScan: () => void;
  onOpenRepair: (selectedRels: string[] | null) => void;
  selectedRels: Set<string>;
  onToggleSelectedRel: (rel: string) => void;
  onSelectAllPage: (findings: Finding[], select: boolean) => void;
  onClearSelection: () => void;
  inspectedRel: string | null;
  onInspectRel: (rel: string | null) => void;
  bannerMessage: { tone: "info" | "warn" | "danger" | "ok"; text: string } | null;
  onDismissBanner: () => void;
}

const RULE_OPTIONS: Array<{ flag: number; label: string; hint: string }> = [
  { flag: FIX_KINDS.LONG_PATHS, label: "Long paths", hint: "Shorten paths exceeding threshold" },
  { flag: FIX_KINDS.ILLEGAL_CHARS, label: "Illegal chars", hint: "Replace < > : \" / \\ | ? * and control chars" },
  { flag: FIX_KINDS.TRAILING_DOTS_AND_SPACES, label: "Leading / trailing", hint: "Trim spaces and trailing dots" },
  { flag: FIX_KINDS.RESERVED_NAMES, label: "Reserved names", hint: "Prefix CON, PRN, AUX, NUL, COM1-9, LPT1-9" },
  { flag: FIX_KINDS.CASE_CLASHES, label: "Case clashes", hint: "Disambiguate duplicate names differing only by case" },
];

const PRESET_LIMITS = [259, 240, 200] as const;

export function FindingsWorkspace({
  config,
  rootInput,
  onChangeRootInput,
  limit,
  kinds,
  onChangePlanOptions,
  scanning,
  scanProgress,
  summary,
  pageData,
  pageOffset,
  pageSize,
  search,
  issueFilter,
  sortBy,
  onChangeSearch,
  onChangeIssueFilter,
  onChangeSortBy,
  onChangePageOffset,
  onStartScan,
  onCancelScan,
  onOpenRepair,
  selectedRels,
  onToggleSelectedRel,
  onSelectAllPage,
  onClearSelection,
  inspectedRel,
  onInspectRel,
  bannerMessage,
  onDismissBanner,
}: FindingsWorkspaceProps) {
  const [rootInfo, setRootInfo] = useState<RootInspectionDto | null>(null);
  const [recentOpen, setRecentOpen] = useState(false);
  const [showWarnings, setShowWarnings] = useState(false);
  const [exportMsg, setExportMsg] = useState<string | null>(null);
  const [customInput, setCustomInput] = useState(String(config.customLimit || 220));
  const [customMode, setCustomMode] = useState(
    !PRESET_LIMITS.includes(limit as 259 | 240 | 200)
  );
  const searchInputRef = useRef<HTMLInputElement | null>(null);
  const recentRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    setCustomMode(!PRESET_LIMITS.includes(limit as 259 | 240 | 200));
    if (!PRESET_LIMITS.includes(limit as 259 | 240 | 200)) {
      setCustomInput(String(limit));
    }
  }, [limit]);

  useEffect(() => {
    if (!rootInput.trim()) {
      setRootInfo(null);
      return;
    }
    const timer = window.setTimeout(() => {
      invoke<RootInspectionDto>("inspect_root", { rootInput })
        .then(setRootInfo)
        .catch(() => setRootInfo(null));
    }, 180);
    return () => window.clearTimeout(timer);
  }, [rootInput]);

  useEffect(() => {
    function onDocClick(e: MouseEvent) {
      if (recentRef.current && !recentRef.current.contains(e.target as Node)) {
        setRecentOpen(false);
      }
    }
    document.addEventListener("mousedown", onDocClick);
    return () => document.removeEventListener("mousedown", onDocClick);
  }, []);

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.ctrlKey && e.key.toLowerCase() === "o") {
        e.preventDefault();
        void handleBrowseFolder();
      } else if ((e.ctrlKey && e.key.toLowerCase() === "f") || (e.ctrlKey && e.key.toLowerCase() === "k")) {
        e.preventDefault();
        searchInputRef.current?.focus();
      } else if (e.key === "F5" && !scanning && rootInput.trim()) {
        e.preventDefault();
        onStartScan(rootInput, limit, kinds);
      } else if (e.key === "Escape") {
        if (recentOpen) {
          setRecentOpen(false);
        } else if (inspectedRel) {
          onInspectRel(null);
        }
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  });

  async function handleBrowseFolder() {
    if (scanning) return;
    try {
      const picked = await open({
        directory: true,
        multiple: false,
        title: "Choose root folder to scan",
      });
      if (typeof picked === "string" && picked.trim()) {
        onChangeRootInput(picked);
        onStartScan(picked, limit, kinds);
      }
    } catch (e) {
      setExportMsg(`Folder picker error: ${formatError(e)}`);
    }
  }

  async function handleExportCsv() {
    if (!summary) return;
    try {
      const savePath = await save({
        title: "Export Scan Findings to CSV",
        defaultPath: "longpathfix-findings.csv",
        filters: [{ name: "CSV", extensions: ["csv"] }],
      });
      if (!savePath) return;
      const rows = await invoke<number>("export_findings", { savePath });
      setExportMsg(`Saved ${rows.toLocaleString()} finding rows to ${savePath}`);
    } catch (e) {
      setExportMsg(`Export failed: ${formatError(e)}`);
    }
  }

  function toggleKind(flag: number) {
    const next = (kinds & flag) !== 0 ? kinds & ~flag : kinds | flag;
    onChangePlanOptions(limit, next);
  }

  function applyCustomLimit(raw: string) {
    setCustomInput(raw);
    const parsed = Number.parseInt(raw, 10);
    if (!Number.isNaN(parsed) && parsed >= 60 && parsed <= 32767) {
      onChangePlanOptions(parsed, kinds);
    }
  }

  const pageFixable = useMemo(
    () => pageData.items.filter((f) => f.newName !== null),
    [pageData.items]
  );

  const allPageFixableSelected =
    pageFixable.length > 0 &&
    pageFixable.every((f) => selectedRels.has(f.node.rel));

  const repairTargetCount =
    selectedRels.size > 0 ? selectedRels.size : summary?.opsCount ?? 0;

  const totalPages = Math.max(1, Math.ceil(pageData.totalMatching / pageSize));
  const currentPage = Math.floor(pageOffset / pageSize) + 1;

  return (
    <div className="scroll-view">
      <header className="view-header">
        <div>
          <h1 className="view-title">Scan & Fix</h1>
          <p className="view-subtitle">
            Scan deep folders via extended Windows paths, inspect offending
            segments, and preview safe renames before changing disk.
          </p>
        </div>

        <div className="header-actions">
          <button
            type="button"
            className="btn"
            disabled={!summary || summary.findingsCount === 0 || scanning}
            onClick={handleExportCsv}
          >
            <DownloadIcon size={15} />
            Export CSV
          </button>
          <button
            type="button"
            className="btn btn-primary"
            disabled={!summary || repairTargetCount === 0 || scanning}
            onClick={() =>
              onOpenRepair(selectedRels.size > 0 ? Array.from(selectedRels) : null)
            }
          >
            Preview & Fix ({repairTargetCount.toLocaleString()})
          </button>
        </div>
      </header>

      {bannerMessage && (
        <div className={`banner ${bannerMessage.tone}`} role="status">
          <div className="banner-main">
            <AlertTriangleIcon size={16} style={{ flexShrink: 0, marginTop: 2 }} />
            <span>{bannerMessage.text}</span>
          </div>
          <button
            type="button"
            className="btn btn-sm btn-ghost"
            onClick={onDismissBanner}
          >
            Dismiss
          </button>
        </div>
      )}

      {exportMsg && (
        <div className="banner info" role="status">
          <span>{exportMsg}</span>
          <button
            type="button"
            className="btn btn-sm btn-ghost"
            onClick={() => setExportMsg(null)}
          >
            Dismiss
          </button>
        </div>
      )}

      <section className="surface scan-controls" aria-label="Scan target and rules">
        <div className="path-row">
          <div className="path-input-wrap">
            <input
              type="text"
              className="input input-mono"
              aria-label="Target folder path"
              placeholder="Enter or paste a folder path (e.g. D:\Archives or \\server\share)…"
              value={rootInput}
              disabled={scanning}
              onChange={(e) => onChangeRootInput(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !scanning && rootInput.trim()) {
                  onStartScan(rootInput, limit, kinds);
                }
              }}
            />
          </div>

          <div className="header-actions">
            <button
              type="button"
              className="btn"
              disabled={scanning}
              onClick={handleBrowseFolder}
            >
              <FolderIcon size={15} />
              Browse…
            </button>

            <div className="recent-wrap" ref={recentRef}>
              <button
                type="button"
                className="btn"
                disabled={scanning || config.recentRoots.length === 0}
                aria-expanded={recentOpen}
                aria-haspopup="menu"
                onClick={() => setRecentOpen((v) => !v)}
              >
                Recent
                <ChevronDownIcon size={14} />
              </button>
              {recentOpen && config.recentRoots.length > 0 && (
                <div className="recent-menu" role="menu">
                  {config.recentRoots.map((recent) => (
                    <button
                      key={recent}
                      type="button"
                      role="menuitem"
                      className="recent-item"
                      onClick={() => {
                        setRecentOpen(false);
                        onChangeRootInput(recent);
                        onStartScan(recent, limit, kinds);
                      }}
                    >
                      {recent}
                    </button>
                  ))}
                </div>
              )}
            </div>
          </div>

          {!scanning ? (
            <button
              type="button"
              className="btn btn-primary"
              disabled={!rootInput.trim() || rootInfo?.valid === false}
              onClick={() => onStartScan(rootInput, limit, kinds)}
            >
              <FolderSearchIcon size={15} />
              Scan Folder
            </button>
          ) : (
            <button
              type="button"
              className="btn btn-danger"
              onClick={onCancelScan}
            >
              Cancel Scan
            </button>
          )}
        </div>

        {rootInfo?.error && (
          <div className="banner danger">{rootInfo.error}</div>
        )}

        {!rootInfo?.error && rootInfo?.warning && (
          <div className="banner warn">
            <div className="banner-main">
              <AlertTriangleIcon size={16} style={{ flexShrink: 0, marginTop: 2 }} />
              <span>{rootInfo.warning}</span>
            </div>
          </div>
        )}

        <div className="config-row">
          <div className="threshold-group">
            <span className="group-label">UTF-16 limit</span>
            <div className="seg-control" role="group" aria-label="Path length threshold presets">
              {PRESET_LIMITS.map((preset) => (
                <button
                  key={preset}
                  type="button"
                  className={`seg-btn ${!customMode && limit === preset ? "active" : ""}`}
                  aria-pressed={!customMode && limit === preset}
                  onClick={() => {
                    setCustomMode(false);
                    onChangePlanOptions(preset, kinds);
                  }}
                >
                  {preset}
                </button>
              ))}
              <button
                type="button"
                className={`seg-btn ${customMode ? "active" : ""}`}
                aria-pressed={customMode}
                onClick={() => {
                  setCustomMode(true);
                  const parsed = Number.parseInt(customInput, 10);
                  if (!Number.isNaN(parsed) && parsed >= 60 && parsed <= 32767) {
                    onChangePlanOptions(parsed, kinds);
                  }
                }}
              >
                Custom
              </button>
            </div>
            {customMode && (
              <input
                type="number"
                min={60}
                max={32767}
                className="input custom-limit-input"
                aria-label="Custom UTF-16 character limit"
                value={customInput}
                onChange={(e) => applyCustomLimit(e.target.value)}
              />
            )}
            {rootInfo?.volume && (
              <span className="tag" title={rootInfo.volume.driveType}>
                {rootInfo.volume.filesystemName || "FS"} · {rootInfo.volume.maxComponentLength}-char segment
              </span>
            )}
          </div>

          <div className="rules-row" role="group" aria-label="Issue fix categories">
            {RULE_OPTIONS.map((rule) => (
              <label key={rule.flag} className="check-pill" title={rule.hint}>
                <input
                  type="checkbox"
                  checked={(kinds & rule.flag) !== 0}
                  onChange={() => toggleKind(rule.flag)}
                />
                <span>{rule.label}</span>
              </label>
            ))}
          </div>
        </div>
      </section>

      {scanning && (
        <section className="surface-tight progress-strip" aria-live="polite">
          <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap" }}>
            <strong>
              Scanning tree…{" "}
              <span className="mono">
                {scanProgress
                  ? `${scanProgress.scannedFiles.toLocaleString()} files · ${scanProgress.scannedDirs.toLocaleString()} folders`
                  : "starting"}
              </span>
            </strong>
            <span className="mono muted-note">
              {scanProgress
                ? `max len ${scanProgress.maxPathLen} · depth ${scanProgress.maxDepth} · skipped links ${scanProgress.skippedReparsePoints}`
                : ""}
            </span>
          </div>
          <div className="progress-track">
            <div className="progress-fill indeterminate" />
          </div>
          {scanProgress?.currentDir && (
            <div className="mono muted-note" style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
              {scanProgress.currentDir}
            </div>
          )}
        </section>
      )}

      {summary && (
        <PathGauge
          summary={summary}
          activeFilter={issueFilter}
          onSelectFilter={onChangeIssueFilter}
        />
      )}

      {summary && summary.warningsCount > 0 && (
        <div className="banner warn">
          <div className="banner-main">
            <AlertTriangleIcon size={16} style={{ flexShrink: 0, marginTop: 2 }} />
            <div>
              <strong>
                {summary.warningsCount} folder warning
                {summary.warningsCount === 1 ? "" : "s"}
              </strong>{" "}
              (access denied or unreadable entries during scan) and{" "}
              {summary.skippedReparsePoints} skipped symlinks/junctions.
              {showWarnings && (
                <ul style={{ margin: "6px 0 0", paddingLeft: 18, fontSize: 12 }}>
                  {summary.warnings.map((w, idx) => (
                    <li key={`${w.path}-${idx}`} className="mono">
                      {w.path}: {w.message}
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </div>
          <button
            type="button"
            className="btn btn-sm"
            onClick={() => setShowWarnings((v) => !v)}
          >
            {showWarnings ? "Hide details" : "View details"}
          </button>
        </div>
      )}

      <section className="results-shell" aria-label="Findings workspace">
        <div className="table-column">
          {summary && (
            <div className="toolbar">
              <div className="header-actions">
                <div className="search-box">
                  <SearchIcon size={14} className="search-icon" />
                  <input
                    ref={searchInputRef}
                    type="search"
                    className="input search-input"
                    aria-label="Filter findings by path, rename, or issue"
                    placeholder="Filter path, rename, or issue…"
                    value={search}
                    onChange={(e) => onChangeSearch(e.target.value)}
                  />
                </div>

                {selectedRels.size > 0 && (
                  <button
                    type="button"
                    className="btn btn-sm"
                    onClick={onClearSelection}
                  >
                    Clear {selectedRels.size} selected
                  </button>
                )}
              </div>

              <div className="toolbar-right">
                <label className="group-label" htmlFor="sort-findings-select">
                  Sort
                </label>
                <select
                  id="sort-findings-select"
                  className="select"
                  value={sortBy}
                  onChange={(e) => onChangeSortBy(e.target.value as SortBy)}
                >
                  <option value="length_desc">Longest path first</option>
                  <option value="length_asc">Shortest path first</option>
                  <option value="severity">Highest severity</option>
                  <option value="path_asc">Path A → Z</option>
                </select>
              </div>
            </div>
          )}

          {!summary && !scanning && (
            <div className="empty-state">
              <div className="empty-icon">
                <FolderSearchIcon size={20} />
              </div>
              <strong>Select a folder to analyze paths and filenames</strong>
              <p className="muted-note" style={{ maxWidth: 480 }}>
                LongPathFix uses extended <span className="mono">\\?\</span>{" "}
                Windows APIs to traverse deep directories beyond 260 characters
                without triggering MAX_PATH truncation. Nothing is modified until
                you review the dry-run preview and confirm.
              </p>
            </div>
          )}

          {summary && summary.findingsCount === 0 && !scanning && (
            <div className="empty-state">
              <div className="empty-icon">
                <ShieldCheckIcon size={20} />
              </div>
              <strong>No issues found under UTF-16 limit {summary.limit}</strong>
              <p className="muted-note">
                Scanned {summary.scanned.toLocaleString()} items in{" "}
                <span className="mono">{summary.rootPlain}</span> (max length{" "}
                <span className="mono">{summary.maxLenBefore}</span> code units).
              </p>
            </div>
          )}

          {summary && summary.findingsCount > 0 && (
            <>
              <div className="table-scroll">
                <table className="data-table">
                  <thead>
                    <tr>
                      <th className="col-check">
                        <input
                          type="checkbox"
                          aria-label="Select all fixable rows on this page"
                          checked={allPageFixableSelected}
                          disabled={pageFixable.length === 0}
                          onChange={(e) =>
                            onSelectAllPage(pageFixable, e.target.checked)
                          }
                        />
                      </th>
                      <th>Relative Path</th>
                      <th style={{ width: 190 }}>Proposed Rename</th>
                      <th style={{ width: 100 }}>UTF-16 Len</th>
                      <th style={{ width: 185 }}>Detected Issues</th>
                    </tr>
                  </thead>
                  <tbody>
                    {pageData.items.map((item) => {
                      const isSelected = inspectedRel === item.node.rel;
                      const checked = selectedRels.has(item.node.rel);
                      const canSelect = item.newName !== null;
                      return (
                        <tr
                          key={item.node.rel}
                          tabIndex={0}
                          aria-selected={isSelected}
                          className={isSelected ? "selected" : ""}
                          onClick={() => onInspectRel(item.node.rel)}
                          onKeyDown={(e) => {
                            if (e.key === "Enter" || e.key === " ") {
                              e.preventDefault();
                              onInspectRel(item.node.rel);
                            }
                          }}
                        >
                          <td className="col-check" onClick={(e) => e.stopPropagation()}>
                            <input
                              type="checkbox"
                              disabled={!canSelect}
                              checked={checked}
                              aria-label={`Select ${item.node.rel} for repair`}
                              onChange={() => onToggleSelectedRel(item.node.rel)}
                            />
                          </td>
                          <td>
                            <div className="cell-path" title={item.node.rel}>
                              {item.node.isDirectory ? (
                                <FolderIcon size={14} style={{ flexShrink: 0, color: "var(--accent-text)" }} />
                              ) : (
                                <FileIcon size={14} style={{ flexShrink: 0, color: "var(--text-muted)" }} />
                              )}
                              <span className="cell-path-text">{item.node.rel}</span>
                            </div>
                          </td>
                          <td>
                            {item.newName ? (
                              <span className="tag ok mono" title={item.newName}>
                                <ArrowRightIcon size={11} style={{ flexShrink: 0 }} />
                                <span className="truncate">{item.newName}</span>
                              </span>
                            ) : item.unresolved ? (
                              <span className="tag warn" title={item.unresolved}>
                                <span className="truncate">Manual review</span>
                              </span>
                            ) : (
                              <span className="tag">
                                <span className="truncate">Skipped by filter</span>
                              </span>
                            )}
                          </td>
                          <td>
                            <span className="len-change">
                              <span>{item.currentLen}</span>
                              {item.plannedLen !== item.currentLen && (
                                <>
                                  <span>→</span>
                                  <strong
                                    style={{
                                      color:
                                        item.plannedLen <= summary.limit
                                          ? "var(--ok-text)"
                                          : "var(--warn-text)",
                                    }}
                                  >
                                    {item.plannedLen}
                                  </strong>
                                </>
                              )}
                            </span>
                          </td>
                          <td>
                            <span
                              className={`tag ${issueTone(item.issues)}`}
                              title={item.issueText}
                            >
                              <span className="truncate">
                                {item.node.isReadonly ? "Read-only · " : ""}
                                {item.issueText}
                              </span>
                            </span>
                          </td>
                        </tr>
                      );
                    })}
                    {pageData.items.length === 0 && (
                      <tr>
                        <td colSpan={5} style={{ textAlign: "center", padding: 28 }}>
                          No findings match the current search or filter.
                        </td>
                      </tr>
                    )}
                  </tbody>
                </table>
              </div>

              <div className="table-footer">
                <span className="mono">
                  Showing {pageData.totalMatching === 0 ? 0 : pageOffset + 1}–
                  {Math.min(pageOffset + pageSize, pageData.totalMatching)} of{" "}
                  {pageData.totalMatching.toLocaleString()} matching (
                  {pageData.fixableMatching.toLocaleString()} fixable)
                </span>
                <div className="header-actions">
                  <button
                    type="button"
                    className="btn btn-sm"
                    disabled={pageOffset === 0}
                    onClick={() =>
                      onChangePageOffset(Math.max(0, pageOffset - pageSize))
                    }
                  >
                    Previous
                  </button>
                  <span className="mono">
                    Page {currentPage} / {totalPages}
                  </span>
                  <button
                    type="button"
                    className="btn btn-sm"
                    disabled={pageOffset + pageSize >= pageData.totalMatching}
                    onClick={() => onChangePageOffset(pageOffset + pageSize)}
                  >
                    Next
                  </button>
                </div>
              </div>
            </>
          )}
        </div>

        {inspectedRel && (
          <PathInspectorDrawer
            rel={inspectedRel}
            onClose={() => onInspectRel(null)}
          />
        )}
      </section>
    </div>
  );
}
