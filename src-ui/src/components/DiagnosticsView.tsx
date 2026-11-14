import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { AlertTriangleIcon, ShieldCheckIcon } from "./Icons";
import type {
  EnableLongPathsResultDto,
  PathAnalysisDto,
  SystemDiagnosticsDto,
} from "../types";
import { formatError } from "../types";

interface DiagnosticsViewProps {
  limit: number;
  onPolicyUpdated: () => void;
}

export function DiagnosticsView({
  limit,
  onPolicyUpdated,
}: DiagnosticsViewProps) {
  const [diag, setDiag] = useState<SystemDiagnosticsDto | null>(null);
  const [labInput, setLabInput] = useState(
    String.raw`C:\Projects\legacy-export\CON\quarterly report .txt`
  );
  const [labResult, setLabResult] = useState<PathAnalysisDto | null>(null);
  const [enabling, setEnabling] = useState(false);
  const [statusMsg, setStatusMsg] = useState<{
    tone: "ok" | "warn" | "danger" | "info";
    text: string;
  } | null>(null);

  useEffect(() => {
    invoke<SystemDiagnosticsDto>("get_system_diagnostics")
      .then(setDiag)
      .catch((e) =>
        setStatusMsg({ tone: "danger", text: formatError(e) })
      );
  }, []);

  useEffect(() => {
    if (!labInput.trim()) {
      setLabResult(null);
      return;
    }
    const timer = window.setTimeout(() => {
      invoke<PathAnalysisDto>("analyze_path_input", {
        input: labInput,
        limit,
      })
        .then(setLabResult)
        .catch(() => {});
    }, 150);
    return () => window.clearTimeout(timer);
  }, [labInput, limit]);

  async function handleEnableLongPaths() {
    setEnabling(true);
    setStatusMsg(null);
    try {
      const res = await invoke<EnableLongPathsResultDto>(
        "enable_system_long_paths"
      );
      if (diag) {
        setDiag({ ...diag, policy: res.policy });
      }
      onPolicyUpdated();
      setStatusMsg({
        tone: res.enabled ? "ok" : "warn",
        text: res.enabled
          ? "HKLM LongPathsEnabled is now set to 1. Restart legacy-aware applications for registry reload."
          : "UAC elevation was cancelled; Windows LongPathsEnabled was left unchanged.",
      });
    } catch (e) {
      setStatusMsg({ tone: "danger", text: formatError(e) });
    } finally {
      setEnabling(false);
    }
  }

  return (
    <div className="scroll-view">
      <header className="view-header">
        <div>
          <h1 className="view-title">Path Lab & System Diagnostics</h1>
          <p className="view-subtitle">
            Inspect arbitrary Windows paths, verify Windows LongPathsEnabled policy
            and application manifest readiness, and review mounted filesystem limits.
          </p>
        </div>
      </header>

      {statusMsg && <div className={`banner ${statusMsg.tone}`}>{statusMsg.text}</div>}

      <div className="two-col-grid">
        <section className="surface" aria-label="Windows Long-Path Policy">
          <h2 className="section-heading">Windows Long-Path Policy & Process</h2>
          {diag ? (
            <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              <div className="kv-grid">
                <div className="kv-cell">
                  <div className="kv-k">Registry LongPathsEnabled</div>
                  <div className="kv-v">
                    <span
                      className={`tag ${
                        diag.policy.longPathsEnabled ? "ok" : "warn"
                      }`}
                    >
                      {diag.policy.longPathsEnabled
                        ? `Enabled (${diag.policy.registryValue ?? 1})`
                        : `Disabled (${diag.policy.registryValue ?? "unset"})`}
                    </span>
                  </div>
                </div>
                <div className="kv-cell">
                  <div className="kv-k">Process Manifest Awareness</div>
                  <div className="kv-v">
                    <span
                      className={`tag ${
                        diag.policy.processLongPathAware ? "ok" : "warn"
                      }`}
                    >
                      {diag.policy.processLongPathAware
                        ? "longPathAware = true"
                        : "Not declared"}
                    </span>
                  </div>
                </div>
                <div className="kv-cell">
                  <div className="kv-k">Windows Build</div>
                  <div className="kv-v mono">{diag.policy.osBuild}</div>
                </div>
                <div className="kv-cell">
                  <div className="kv-k">Current Privilege</div>
                  <div className="kv-v">
                    {diag.policy.isElevated ? "Elevated Administrator" : "Standard User"}
                  </div>
                </div>
              </div>

              <p className="muted-note">
                Enabling{" "}
                <span className="mono">
                  HKLM\<wbr />SYSTEM\<wbr />CurrentControlSet\<wbr />Control\<wbr />FileSystem\<wbr />LongPathsEnabled
                </span>{" "}
                allows opted-in Win32 apps to exceed {diag.policy.classicMaxPath}{" "}
                UTF-16 units without <span className="mono">\\?\</span>, but it does
                NOT fix File Explorer, ZIP tools, OneDrive, or non-aware apps, nor
                does it bypass reserved names or illegal characters.
              </p>

              <div className="header-actions">
                <button
                  type="button"
                  className="btn btn-primary"
                  disabled={diag.policy.longPathsEnabled || enabling}
                  onClick={handleEnableLongPaths}
                >
                  <ShieldCheckIcon size={15} />
                  {diag.policy.longPathsEnabled
                    ? "LongPathsEnabled Already Active"
                    : enabling
                      ? "Waiting for UAC…"
                      : "Enable Windows Long Paths…"}
                </button>
              </div>
            </div>
          ) : (
            <p className="muted-note">Loading policy state…</p>
          )}
        </section>

        <section className="surface" aria-label="Path Lab Analyzer">
          <h2 className="section-heading">Interactive Path Lab</h2>
          <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
            <label htmlFor="path-lab-input" className="muted-note">
              Paste any local, UNC (<span className="mono">\\server\share</span>), or
              extended (<span className="mono">\\?\</span>) path to inspect how Win32
              parses every segment:
            </label>
            <input
              id="path-lab-input"
              type="text"
              className="input input-mono"
              value={labInput}
              onChange={(e) => setLabInput(e.target.value)}
              placeholder="C:\path\to\analyze…"
            />

            {labResult?.error && (
              <div className="banner danger">{labResult.error}</div>
            )}

            {labResult && !labResult.error && (
              <>
                <div className="kv-grid">
                  <div className="kv-cell">
                    <div className="kv-k">Detected path form</div>
                    <div className="kv-v mono">{labResult.form}</div>
                  </div>
                  <div className="kv-cell">
                    <div className="kv-k">UTF-16 code units</div>
                    <div className="kv-v mono">
                      {labResult.utf16Len}{" "}
                      {labResult.exceedsClassic ? "(exceeds 259)" : "(<= 259)"}
                    </div>
                  </div>
                </div>

                {labResult.extendedPath && (
                  <div className="code-block-box">
                    <div className="code-block-head">
                      <span>Canonical Win32 extended form</span>
                      <span>
                        {labResult.existsOnDisk
                          ? labResult.isDirectory
                            ? "Exists (Folder)"
                            : "Exists (File)"
                          : "Not on local disk"}
                      </span>
                    </div>
                    <div className="code-block-val">{labResult.extendedPath}</div>
                  </div>
                )}

                <div className="segment-list" style={{ maxHeight: 190, overflowY: "auto" }}>
                  {labResult.segments.map((seg, i) => (
                    <div key={`${seg.name}-${i}`} className="segment-row">
                      <div className="segment-top">
                        <span className="segment-name">
                          {seg.isRoot ? `${seg.name} (root)` : seg.name}
                        </span>
                        <span className="mono">{seg.utf16Len}u</span>
                      </div>
                      {seg.issues.length > 0 && (
                        <div className="filter-pills">
                          {seg.issues.map((iss) => (
                            <span key={iss} className="tag danger">
                              <AlertTriangleIcon size={11} />
                              {iss}
                            </span>
                          ))}
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              </>
            )}
          </div>
        </section>
      </div>

      <section className="surface" aria-label="Mounted filesystems">
        <h2 className="section-heading">Mounted Volumes & Filesystem Support</h2>
        {diag ? (
          <div className="table-scroll">
            <table className="data-table">
              <thead>
                <tr>
                  <th style={{ width: 76 }}>Root</th>
                  <th>Volume Label</th>
                  <th style={{ width: 100 }}>Filesystem</th>
                  <th style={{ width: 115 }}>Drive Type</th>
                  <th style={{ width: 120 }}>Max Segment</th>
                  <th style={{ width: 250 }}>Features</th>
                </tr>
              </thead>
              <tbody>
                {diag.volumes.map((v) => (
                  <tr key={v.rootPath}>
                    <td className="mono">{v.rootPath}</td>
                    <td>{v.volumeLabel || "—"}</td>
                    <td className="mono">{v.filesystemName || "Unknown"}</td>
                    <td>{v.driveType}</td>
                    <td className="mono">{v.maxComponentLength} chars</td>
                    <td>
                      <span className="tag">
                        {[
                          v.supportsUnicodeOnDisk ? "Unicode" : null,
                          v.supportsReparsePoints ? "Reparse" : null,
                          v.supportsCaseSensitiveDirs ? "CaseDir" : null,
                          v.isReadOnly ? "ReadOnly" : null,
                        ]
                          .filter(Boolean)
                          .join(" · ") || "Basic"}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <p className="muted-note">Enumerating mounted volumes…</p>
        )}
      </section>
    </div>
  );
}
