import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { FolderIcon, HistoryIcon, TrashIcon, UndoIcon } from "./Icons";
import type {
  HistorySummaryDto,
  RenameBatch,
  RenameOutcome,
} from "../types";
import { formatError } from "../types";

interface HistoryViewProps {
  defaultClearReadonly: boolean;
  onHistoryChanged: () => void;
}

export function HistoryView({
  defaultClearReadonly,
  onHistoryChanged,
}: HistoryViewProps) {
  const [batches, setBatches] = useState<HistorySummaryDto[]>([]);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<RenameBatch | null>(null);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);
  const [feedback, setFeedback] = useState<{
    tone: "ok" | "warn" | "danger" | "info";
    text: string;
  } | null>(null);

  async function refresh() {
    try {
      const list = await invoke<HistorySummaryDto[]>("get_history");
      setBatches(list);
      onHistoryChanged();
    } catch (e) {
      setFeedback({ tone: "danger", text: formatError(e) });
    }
  }

  useEffect(() => {
    void refresh();
  }, []);

  async function toggleDetail(id: string) {
    if (expandedId === id) {
      setExpandedId(null);
      setDetail(null);
      return;
    }
    try {
      const b = await invoke<RenameBatch>("get_history_batch_detail", { id });
      setExpandedId(id);
      setDetail(b);
    } catch (e) {
      setFeedback({ tone: "danger", text: formatError(e) });
    }
  }

  async function handleUndo(id: string) {
    setBusyId(id);
    setFeedback(null);
    try {
      const outcomes = await invoke<RenameOutcome[]>("undo_batch", {
        id,
        clearReadonly: defaultClearReadonly,
      });
      const okCount = outcomes.filter((o) => o.ok).length;
      const failCount = outcomes.filter((o) => !o.ok).length;
      setFeedback({
        tone: failCount === 0 ? "ok" : "warn",
        text: `Undo completed for batch ${id}: ${okCount} restored, ${failCount} failed.`,
      });
      if (expandedId === id) {
        const updated = await invoke<RenameBatch>("get_history_batch_detail", { id });
        setDetail(updated);
      }
      await refresh();
    } catch (e) {
      setFeedback({ tone: "danger", text: formatError(e) });
    } finally {
      setBusyId(null);
    }
  }

  async function handleDelete(id: string) {
    setBusyId(id);
    setConfirmDeleteId(null);
    try {
      await invoke("delete_history_batch", { id });
      if (expandedId === id) {
        setExpandedId(null);
        setDetail(null);
      }
      setFeedback({ tone: "info", text: `Removed history record ${id}.` });
      await refresh();
    } catch (e) {
      setFeedback({ tone: "danger", text: formatError(e) });
    } finally {
      setBusyId(null);
    }
  }

  async function openFolder(sub: "history" | "logs") {
    try {
      await invoke("open_data_folder", { sub });
    } catch (e) {
      setFeedback({ tone: "danger", text: formatError(e) });
    }
  }

  return (
    <div className="scroll-view">
      <header className="view-header">
        <div>
          <h1 className="view-title">History & Undo</h1>
          <p className="view-subtitle">
            Every applied repair batch is persisted locally as JSON (compatible
            with LongPathFix 1.x history) so names can be restored in reverse
            order.
          </p>
        </div>

        <div className="header-actions">
          <button type="button" className="btn" onClick={() => openFolder("history")}>
            <FolderIcon size={15} />
            Open History Folder
          </button>
          <button type="button" className="btn" onClick={() => openFolder("logs")}>
            <FolderIcon size={15} />
            Open Logs Folder
          </button>
        </div>
      </header>

      {feedback && (
        <div className={`banner ${feedback.tone}`} role="status">
          <span>{feedback.text}</span>
          <button
            type="button"
            className="btn btn-sm btn-ghost"
            onClick={() => setFeedback(null)}
          >
            Dismiss
          </button>
        </div>
      )}

      {batches.length === 0 ? (
        <section className="surface empty-state">
          <div className="empty-icon">
            <HistoryIcon size={20} />
          </div>
          <strong>No repair batches recorded yet</strong>
          <p className="muted-note">
            Scan a folder and apply renames to create an undoable batch log under{" "}
            <span className="mono">%APPDATA%\LongPathFix\history</span>.
          </p>
        </section>
      ) : (
        <div className="history-list">
          {batches.map((b) => {
            const isOpen = expandedId === b.id && detail;
            return (
              <article key={b.id} className="history-item">
                <div className="history-row">
                  <div style={{ minWidth: 0 }}>
                    <div style={{ display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
                      <strong>{b.title}</strong>
                      <span className={`tag ${b.undone ? "" : "ok"}`}>
                        {b.undone ? "Undone" : "Active"}
                      </span>
                      <span className="mono muted-note">{b.displayTime}</span>
                    </div>
                    <div className="mono muted-note" style={{ marginTop: 3, wordBreak: "break-all" }}>
                      {b.root} · id {b.id}
                    </div>
                  </div>

                  <div className="header-actions">
                    <button
                      type="button"
                      className="btn btn-sm"
                      onClick={() => toggleDetail(b.id)}
                    >
                      {isOpen ? "Hide entries" : "Inspect entries"}
                    </button>

                    {!b.undone && (
                      <button
                        type="button"
                        className="btn btn-sm btn-primary"
                        disabled={busyId === b.id}
                        onClick={() => handleUndo(b.id)}
                      >
                        <UndoIcon size={13} />
                        {busyId === b.id ? "Restoring…" : "Undo Batch"}
                      </button>
                    )}

                    {confirmDeleteId === b.id ? (
                      <button
                        type="button"
                        className="btn btn-sm btn-danger"
                        disabled={busyId === b.id}
                        onClick={() => handleDelete(b.id)}
                      >
                        Confirm Remove
                      </button>
                    ) : (
                      <button
                        type="button"
                        className="btn btn-sm"
                        aria-label={`Delete history record ${b.id}`}
                        disabled={busyId === b.id}
                        onClick={() => setConfirmDeleteId(b.id)}
                      >
                        <TrashIcon size={13} />
                      </button>
                    )}
                  </div>
                </div>

                {isOpen && detail && (
                  <div className="history-detail">
                    <table className="data-table">
                      <thead>
                        <tr>
                          <th style={{ width: 70 }}>Type</th>
                          <th>Directory</th>
                          <th>Old Name → New Name</th>
                        </tr>
                      </thead>
                      <tbody>
                        {detail.Done.map((d, idx) => (
                          <tr key={`${d.Dir}-${d.OldName}-${idx}`}>
                            <td>{d.IsDirectory ? "Folder" : "File"}</td>
                            <td className="mono">{d.Dir}</td>
                            <td className="mono">
                              {d.OldName} → <strong>{d.NewName}</strong>
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                )}
              </article>
            );
          })}
        </div>
      )}
    </div>
  );
}
