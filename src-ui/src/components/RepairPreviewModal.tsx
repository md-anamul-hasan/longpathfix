import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { useEffect, useRef, useState } from "react";
import { AlertTriangleIcon, CloseIcon, DownloadIcon, UndoIcon } from "./Icons";
import type {
  ApplyProgress,
  DryRunReport,
  DryRunStatus,
  RepairCompleteDto,
} from "../types";
import { formatError } from "../types";

interface RepairPreviewModalProps {
  selectedRels: string[] | null;
  defaultClearReadonly: boolean;
  progress: ApplyProgress | null;
  complete: RepairCompleteDto | null;
  error: string | null;
  running: boolean;
  onExecute: (clearReadonly: boolean) => void;
  onCancelRunning: () => void;
  onUndoBatch: (batchId: string, clearReadonly: boolean) => void;
  onClose: () => void;
}

function statusBadge(status: DryRunStatus): { label: string; cls: string } {
  switch (status) {
    case "ready":
      return { label: "Ready", cls: "ok" };
    case "read_only":
      return { label: "Read-only", cls: "warn" };
    case "target_already_exists":
      return { label: "Conflict", cls: "danger" };
    case "source_missing":
      return { label: "Missing", cls: "danger" };
    case "locked_by_process":
      return { label: "Locked", cls: "danger" };
    case "invalid_boundary":
      return { label: "Blocked", cls: "danger" };
  }
}

export function RepairPreviewModal({
  selectedRels,
  defaultClearReadonly,
  progress,
  complete,
  error,
  running,
  onExecute,
  onCancelRunning,
  onUndoBatch,
  onClose,
}: RepairPreviewModalProps) {
  const [report, setReport] = useState<DryRunReport | null>(null);
  const [previewErr, setPreviewErr] = useState<string | null>(null);
  const [clearReadonly, setClearReadonly] = useState(defaultClearReadonly);
  const [exportNote, setExportNote] = useState<string | null>(null);
  const panelRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const previousFocus = document.activeElement as HTMLElement | null;
    panelRef.current?.focus();
    return () => {
      previousFocus?.focus?.();
    };
  }, []);

  useEffect(() => {
    let active = true;
    invoke<DryRunReport>("preview_repair", { selectedRels })
      .then((res) => {
        if (active) setReport(res);
      })
      .catch((e) => {
        if (active) setPreviewErr(formatError(e));
      });
    return () => {
      active = false;
    };
  }, [selectedRels]);

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape" && !running) {
        onClose();
        return;
      }
      if (e.key === "Tab" && panelRef.current) {
        const focusable = Array.from(
          panelRef.current.querySelectorAll<HTMLElement>(
            'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'
          )
        );
        if (focusable.length === 0) return;
        const first = focusable[0];
        const last = focusable[focusable.length - 1];
        const active = document.activeElement as HTMLElement | null;
        if (e.shiftKey) {
          if (active === first || active === panelRef.current || !panelRef.current.contains(active)) {
            e.preventDefault();
            last.focus();
          }
        } else if (active === last || !panelRef.current.contains(active)) {
          e.preventDefault();
          first.focus();
        }
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [running, onClose]);

  async function handleExportOutcomes() {
    if (!complete) return;
    try {
      const savePath = await save({
        title: "Export Repair Outcomes CSV",
        defaultPath: "longpathfix-repair-outcomes.csv",
        filters: [{ name: "CSV", extensions: ["csv"] }],
      });
      if (!savePath) return;
      const count = await invoke<number>("export_outcomes", {
        savePath,
        outcomes: complete.outcomes,
      });
      setExportNote(`Exported ${count} outcome rows to ${savePath}`);
    } catch (e) {
      setExportNote(`Export failed: ${formatError(e)}`);
    }
  }

  const actionableCount = report
    ? report.readyCount + (clearReadonly ? report.readonlyCount : 0)
    : 0;

  return (
    <div
      className="modal-backdrop"
      role="dialog"
      aria-modal="true"
      aria-labelledby="repair-modal-title"
      onClick={(e) => {
        if (e.target === e.currentTarget && !running) onClose();
      }}
    >
      <div ref={panelRef} className="modal-panel" tabIndex={-1}>
        <div className="modal-header">
          <h2 id="repair-modal-title" className="modal-title">
            {complete
              ? "Repair Execution Summary"
              : running
                ? "Applying Planned Renames"
                : "Dry-Run Repair Verification"}
          </h2>
          {!running && (
            <button
              type="button"
              className="btn btn-sm btn-ghost"
              onClick={onClose}
              aria-label="Close repair dialog"
            >
              <CloseIcon size={16} />
            </button>
          )}
        </div>

        <div className="modal-body">
          {(previewErr || error) && (
            <div className="banner danger">{previewErr || error}</div>
          )}

          {exportNote && <div className="banner info">{exportNote}</div>}

          {!complete && !running && !report && !previewErr && (
            <p className="muted-note">
              Checking filesystem state, file locks, read-only attributes, and
              destination collisions…
            </p>
          )}

          {!complete && !running && report && (
            <>
              <div className="banner info">
                <div className="banner-main">
                  <AlertTriangleIcon size={17} style={{ flexShrink: 0, marginTop: 2 }} />
                  <div>
                    Only filenames and folder names change in-place. No files are
                    moved, rewritten, or deleted. Renames run deepest-first, and an
                    undo batch is saved in AppData before finishing.
                  </div>
                </div>
              </div>

              <div className="gauge-metrics">
                <div className="metric-pair">
                  <span className="metric-val ok tabular">{report.readyCount}</span>
                  <span className="metric-lbl">ready</span>
                </div>
                <div className="metric-pair">
                  <span className="metric-val warn tabular">{report.readonlyCount}</span>
                  <span className="metric-lbl">read-only</span>
                </div>
                <div className="metric-pair">
                  <span className="metric-val danger tabular">
                    {report.conflictCount + report.lockedCount + report.missingCount + report.blockedCount}
                  </span>
                  <span className="metric-lbl">blocked/conflicts</span>
                </div>
              </div>

              {report.readonlyCount > 0 && (
                <label className="check-pill">
                  <input
                    type="checkbox"
                    checked={clearReadonly}
                    onChange={(e) => setClearReadonly(e.target.checked)}
                  />
                  <span>
                    Clear Read-only attribute before renaming ({report.readonlyCount}{" "}
                    {report.readonlyCount === 1 ? "item" : "items"} affected)
                  </span>
                </label>
              )}

              <div className="table-scroll" style={{ maxHeight: 330, border: "1px solid var(--border)", borderRadius: "var(--radius-md)" }}>
                <table className="data-table">
                  <thead>
                    <tr>
                      <th style={{ width: 96 }}>Status</th>
                      <th style={{ width: 250 }}>Relative path</th>
                      <th>Current &amp; proposed name</th>
                    </tr>
                  </thead>
                  <tbody>
                    {report.items.map((item) => {
                      const st = statusBadge(item.status);
                      return (
                        <tr key={item.rel}>
                          <td>
                            <span className={`tag ${st.cls}`}>{st.label}</span>
                          </td>
                          <td className="mono" style={{ overflowWrap: "break-word" }} title={item.rel}>
                            {item.rel}
                          </td>
                          <td>
                            <div className="mono" style={{ overflowWrap: "break-word" }}>
                              {item.oldName}
                            </div>
                            <div
                              className="mono"
                              style={{ color: "var(--ok-text)", overflowWrap: "break-word", marginTop: 2 }}
                            >
                              → <strong>{item.newName}</strong>
                            </div>
                            {item.status !== "ready" && (
                              <div className="muted-note" style={{ fontSize: 11.5, marginTop: 2 }}>
                                {item.message}
                              </div>
                            )}
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            </>
          )}

          {running && (
            <div className="progress-strip">
              <div style={{ display: "flex", justifyContent: "space-between", fontSize: 13 }}>
                <span>
                  Renaming {progress ? `${progress.completed} / ${progress.total}` : "…"}
                </span>
                <span className="mono">
                  {progress
                    ? `${progress.succeeded} ok · ${progress.failed} failed`
                    : ""}
                </span>
              </div>
              <div className="progress-track">
                <div
                  className="progress-fill"
                  style={{
                    width: progress && progress.total > 0
                      ? `${Math.round((progress.completed / progress.total) * 100)}%`
                      : "12%",
                  }}
                />
              </div>
              {progress?.currentPath && (
                <div className="mono muted-note" style={{ wordBreak: "break-all" }}>
                  {progress.currentPath}
                </div>
              )}
            </div>
          )}

          {complete && (
            <>
              <div className={`banner ${complete.failed > 0 ? "warn" : "ok"}`}>
                <div>
                  <strong>
                    {complete.cancelled ? "Stopped mid-batch. " : ""}
                    {complete.succeeded} succeeded, {complete.failed} failed.
                  </strong>{" "}
                  {complete.batch
                    ? `Undo batch ${complete.batch.Id} saved.`
                    : "No changes were written to disk."}
                </div>
              </div>

              <div className="table-scroll" style={{ maxHeight: 310, border: "1px solid var(--border)", borderRadius: "var(--radius-md)" }}>
                <table className="data-table">
                  <thead>
                    <tr>
                      <th style={{ width: 92 }}>Outcome</th>
                      <th style={{ width: 250 }}>Path</th>
                      <th>Detail</th>
                    </tr>
                  </thead>
                  <tbody>
                    {complete.outcomes.map((o, idx) => (
                      <tr key={`${o.path}-${idx}`}>
                        <td>
                          <span
                            className={`tag ${
                              o.ok ? "ok" : o.skipped ? "warn" : "danger"
                            }`}
                          >
                            {o.ok ? "Renamed" : o.skipped ? "Skipped" : "Failed"}
                          </span>
                        </td>
                        <td className="mono" title={o.path}>
                          {o.path}
                        </td>
                        <td>{o.message}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </>
          )}
        </div>

        <div className="modal-footer">
          {!complete && !running && (
            <>
              <span className="muted-note">
                {actionableCount > 0
                  ? `${actionableCount} item${actionableCount === 1 ? "" : "s"} will be renamed on confirmation.`
                  : "No items are currently eligible to rename."}
              </span>
              <div className="header-actions">
                <button type="button" className="btn" onClick={onClose}>
                  Cancel
                </button>
                <button
                  type="button"
                  className="btn btn-primary"
                  disabled={actionableCount === 0}
                  onClick={() => onExecute(clearReadonly)}
                >
                  Apply {actionableCount} Safe Renames
                </button>
              </div>
            </>
          )}

          {running && (
            <>
              <span className="muted-note">
                Completed steps before cancellation remain recorded in History.
              </span>
              <button
                type="button"
                className="btn btn-danger"
                onClick={onCancelRunning}
              >
                Cancel Operation
              </button>
            </>
          )}

          {complete && (
            <>
              <button
                type="button"
                className="btn"
                onClick={handleExportOutcomes}
              >
                <DownloadIcon size={14} />
                Export CSV
              </button>
              <div className="header-actions">
                {complete.batch && !complete.batch.Undone && (
                  <button
                    type="button"
                    className="btn"
                    onClick={() => onUndoBatch(complete.batch!.Id, clearReadonly)}
                  >
                    <UndoIcon size={14} />
                    Undo Batch Now
                  </button>
                )}
                <button
                  type="button"
                  className="btn btn-primary"
                  onClick={onClose}
                >
                  Done
                </button>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
