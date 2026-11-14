import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { CheckIcon, CloseIcon, CopyIcon, ExternalLinkIcon } from "./Icons";
import type { CustomNameCheckDto, PathInspectionDto } from "../types";
import { formatError, issueLabelTone } from "../types";

interface PathInspectorDrawerProps {
  rel: string;
  onClose: () => void;
}

function formatBytes(bytes: number | null): string {
  if (bytes === null) return "Folder";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024)
    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

export function PathInspectorDrawer({
  rel,
  onClose,
}: PathInspectorDrawerProps) {
  const [data, setData] = useState<PathInspectionDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [copiedKey, setCopiedKey] = useState<string | null>(null);
  const [testName, setTestName] = useState("");
  const [nameCheck, setNameCheck] = useState<CustomNameCheckDto | null>(null);

  useEffect(() => {
    let active = true;
    setError(null);
    setData(null);
    invoke<PathInspectionDto>("inspect_path", { rel })
      .then((res) => {
        if (!active) return;
        setData(res);
        const lastSeg = res.segments[res.segments.length - 1];
        setTestName(lastSeg?.newName ?? lastSeg?.name ?? "");
      })
      .catch((e) => {
        if (active) setError(formatError(e));
      });
    return () => {
      active = false;
    };
  }, [rel]);

  useEffect(() => {
    if (!testName) {
      setNameCheck(null);
      return;
    }
    let active = true;
    invoke<CustomNameCheckDto>("validate_custom_name", { name: testName })
      .then((res) => {
        if (active) setNameCheck(res);
      })
      .catch(() => {});
    return () => {
      active = false;
    };
  }, [testName]);

  async function handleCopy(key: string, text: string) {
    try {
      await navigator.clipboard.writeText(text);
      setCopiedKey(key);
      setTimeout(() => setCopiedKey(null), 1500);
    } catch {
      // Clipboard may be restricted in headless contexts
    }
  }

  async function handleReveal() {
    if (!data) return;
    try {
      await invoke("open_in_explorer", {
        targetPath: data.extendedPath,
        select: true,
      });
    } catch (e) {
      setError(formatError(e));
    }
  }

  return (
    <aside className="inspector-drawer" aria-label="Selected path inspector">
      <div className="drawer-header">
        <h2 className="drawer-title">Path Inspector</h2>
        <button
          type="button"
          className="btn btn-sm btn-ghost"
          onClick={onClose}
          aria-label="Close path inspector"
        >
          <CloseIcon size={15} />
        </button>
      </div>

      <div className="drawer-body">
        {error && <div className="banner danger">{error}</div>}
        {!data && !error && <p className="muted-note">Inspecting path segments…</p>}

        {data && (
          <>
            <div className="kv-grid">
              <div className="kv-cell">
                <div className="kv-k">Current length</div>
                <div className="kv-v mono">{data.currentLen} / {data.limit}</div>
              </div>
              <div className="kv-cell">
                <div className="kv-k">Planned length</div>
                <div className="kv-v mono">
                  {data.plannedLen}{" "}
                  {data.plannedLen <= data.limit ? "(fits)" : "(over limit)"}
                </div>
              </div>
              <div className="kv-cell">
                <div className="kv-k">Entry size</div>
                <div className="kv-v mono">{formatBytes(data.size)}</div>
              </div>
              <div className="kv-cell">
                <div className="kv-k">Access & attributes</div>
                <div className="kv-v">
                  {data.isLocked
                    ? "Locked by process"
                    : data.isReadonly
                      ? "Read-only"
                      : data.isReparsePoint
                        ? "Reparse point"
                        : "Writable"}
                </div>
              </div>
            </div>

            {data.issueLabels.length > 0 && (
              <div className="filter-pills">
                {data.issueLabels.map((lbl) => (
                  <span key={lbl} className={`tag ${issueLabelTone(lbl)}`}>
                    {lbl}
                  </span>
                ))}
              </div>
            )}

            {data.unresolved && (
              <div className="banner warn">{data.unresolved}</div>
            )}

            <div className="code-block-box">
              <div className="code-block-head">
                <span>Standard Windows path</span>
                <button
                  type="button"
                  className="btn btn-sm btn-ghost"
                  onClick={() => handleCopy("plain", data.plainPath)}
                >
                  {copiedKey === "plain" ? (
                    <>
                      <CheckIcon size={13} /> Copied
                    </>
                  ) : (
                    <>
                      <CopyIcon size={13} /> Copy
                    </>
                  )}
                </button>
              </div>
              <div className="code-block-val">{data.plainPath}</div>
            </div>

            <div className="code-block-box">
              <div className="code-block-head">
                <span>Extended path (Win32 API form)</span>
                <button
                  type="button"
                  className="btn btn-sm btn-ghost"
                  onClick={() => handleCopy("ext", data.extendedPath)}
                >
                  {copiedKey === "ext" ? (
                    <>
                      <CheckIcon size={13} /> Copied
                    </>
                  ) : (
                    <>
                      <CopyIcon size={13} /> Copy
                    </>
                  )}
                </button>
              </div>
              <div className="code-block-val">{data.extendedPath}</div>
            </div>

            <button
              type="button"
              className="btn"
              onClick={handleReveal}
            >
              <ExternalLinkIcon size={14} />
              Reveal in File Explorer
            </button>

            <div>
              <h3 className="section-heading">Segment breakdown</h3>
              <div className="segment-list">
                {data.segments.map((seg, i) => (
                  <div key={`${seg.name}-${i}`} className="segment-row">
                    <div className="segment-top">
                      <span className="segment-name" title={seg.name}>
                        {seg.isRoot ? `${seg.name} (root)` : seg.name}
                      </span>
                      <span className="mono">
                        {seg.utf16Len}u
                        {seg.newName ? ` → ${seg.newUtf16Len}u` : ""}
                      </span>
                    </div>
                    {seg.newName && (
                      <div className="muted-note mono">
                        Rename: {seg.newName}
                      </div>
                    )}
                    {seg.issues.length > 0 && (
                      <div className="filter-pills">
                        {seg.issues.map((iss) => (
                          <span key={iss} className="tag danger">
                            {iss}
                          </span>
                        ))}
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </div>

            <div>
              <label
                htmlFor="inspector-name-check"
                className="section-heading"
                style={{ display: "block" }}
              >
                Test replacement filename
              </label>
              <input
                id="inspector-name-check"
                type="text"
                className="input input-mono"
                style={{ width: "100%" }}
                value={testName}
                onChange={(e) => setTestName(e.target.value)}
                placeholder="Type a candidate segment name…"
              />
              {nameCheck && (
                <div style={{ marginTop: 6 }}>
                  {nameCheck.valid ? (
                    <span className="tag ok">
                      Valid Windows name ({nameCheck.utf16Len} / 255 UTF-16)
                    </span>
                  ) : (
                    <span className="tag danger">
                      {nameCheck.error ?? nameCheck.issues.join(", ")}
                    </span>
                  )}
                </div>
              )}
            </div>
          </>
        )}
      </div>
    </aside>
  );
}
