import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { FolderIcon } from "./Icons";
import type {
  AppConfig,
  SystemDiagnosticsDto,
  ThemePreference,
} from "../types";
import { FIX_KINDS } from "../types";

interface SettingsGuideViewProps {
  version: string;
  config: AppConfig;
  dataDir: string;
  onSaveConfig: (next: AppConfig) => void;
}

export function SettingsGuideView({
  version,
  config,
  dataDir,
  onSaveConfig,
}: SettingsGuideViewProps) {
  const [diag, setDiag] = useState<SystemDiagnosticsDto | null>(null);

  useEffect(() => {
    invoke<SystemDiagnosticsDto>("get_system_diagnostics")
      .then(setDiag)
      .catch(() => {});
  }, []);

  function updateField<K extends keyof AppConfig>(key: K, val: AppConfig[K]) {
    onSaveConfig({ ...config, [key]: val });
  }

  function toggleDefaultRule(flag: number) {
    const cur = config.defaultFixKinds;
    const next = (cur & flag) !== 0 ? cur & ~flag : cur | flag;
    updateField("defaultFixKinds", next);
  }

  return (
    <div className="scroll-view">
      <header className="view-header">
        <div>
          <h1 className="view-title">Settings & Guide</h1>
          <p className="view-subtitle">
            Configure default scan rules, theme preference, read-only behavior,
            and review keyboard shortcuts and Windows path rules.
          </p>
        </div>
        <button
          type="button"
          className="btn"
          onClick={() => invoke("open_data_folder", { sub: null })}
        >
          <FolderIcon size={15} />
          Open AppData Folder
        </button>
      </header>

      <div className="two-col-grid">
        <section className="surface" aria-label="Defaults and preferences">
          <h2 className="section-heading">Preferences</h2>
          <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
            <div>
              <label
                htmlFor="pref-theme"
                className="group-label"
                style={{ display: "block", marginBottom: 6 }}
              >
                Color theme
              </label>
              <select
                id="pref-theme"
                className="select select-wide"
                value={config.theme}
                onChange={(e) =>
                  updateField("theme", e.target.value as ThemePreference)
                }
              >
                <option value="system">System default (Windows)</option>
                <option value="dark">Dark</option>
                <option value="light">Light</option>
              </select>
            </div>

            <div>
              <label
                htmlFor="pref-limit"
                className="group-label"
                style={{ display: "block", marginBottom: 6 }}
              >
                Default UTF-16 path limit
              </label>
              <select
                id="pref-limit"
                className="select select-wide"
                value={
                  [259, 240, 200].includes(config.defaultLimit)
                    ? String(config.defaultLimit)
                    : "custom"
                }
                onChange={(e) => {
                  if (e.target.value === "custom") {
                    updateField("defaultLimit", config.customLimit || 220);
                  } else {
                    updateField("defaultLimit", Number(e.target.value));
                  }
                }}
              >
                <option value="259">259 — Classic Windows MAX_PATH ceiling</option>
                <option value="240">240 — Margin for ZIP / backup wrappers</option>
                <option value="200">200 — Conservative cloud / OneDrive sync</option>
                <option value="custom">Custom ({config.customLimit})</option>
              </select>

              {![259, 240, 200].includes(config.defaultLimit) && (
                <div style={{ marginTop: 8, display: "flex", alignItems: "center", gap: 8 }}>
                  <label htmlFor="pref-custom-limit" className="muted-note">
                    Custom threshold (60–32,767):
                  </label>
                  <input
                    id="pref-custom-limit"
                    type="number"
                    min={60}
                    max={32767}
                    className="input custom-limit-input"
                    value={config.customLimit}
                    onChange={(e) => {
                      const val = Number.parseInt(e.target.value, 10);
                      if (!Number.isNaN(val) && val >= 60 && val <= 32767) {
                        onSaveConfig({
                          ...config,
                          customLimit: val,
                          defaultLimit: val,
                        });
                      }
                    }}
                  />
                </div>
              )}
            </div>

            <div>
              <div className="group-label" style={{ marginBottom: 6 }}>
                Default enabled fix categories
              </div>
              <div className="rules-row">
                {[
                  { flag: FIX_KINDS.LONG_PATHS, label: "Long paths" },
                  { flag: FIX_KINDS.ILLEGAL_CHARS, label: "Illegal chars" },
                  { flag: FIX_KINDS.TRAILING_DOTS_AND_SPACES, label: "Leading / trailing" },
                  { flag: FIX_KINDS.RESERVED_NAMES, label: "Reserved names" },
                  { flag: FIX_KINDS.CASE_CLASHES, label: "Case clashes" },
                ].map((r) => (
                  <label key={r.flag} className="check-pill">
                    <input
                      type="checkbox"
                      checked={(config.defaultFixKinds & r.flag) !== 0}
                      onChange={() => toggleDefaultRule(r.flag)}
                    />
                    <span>{r.label}</span>
                  </label>
                ))}
              </div>
            </div>

            <label className="check-pill">
              <input
                type="checkbox"
                checked={config.clearReadonlyWhenAuthorized}
                onChange={(e) =>
                  updateField("clearReadonlyWhenAuthorized", e.target.checked)
                }
              />
              <span>
                Default &ldquo;Clear Read-only attribute&rdquo; to checked in repair
                preview
              </span>
            </label>
          </div>
        </section>

        <section className="surface" aria-label="Keyboard shortcuts and storage">
          <h2 className="section-heading">Keyboard Shortcuts</h2>
          <div className="shortcut-grid">
            <div className="shortcut-row">
              <span>Browse folder</span>
              <kbd>Ctrl + O</kbd>
            </div>
            <div className="shortcut-row">
              <span>Start / re-run scan</span>
              <kbd>F5</kbd>
            </div>
            <div className="shortcut-row">
              <span>Focus findings filter</span>
              <kbd>Ctrl + F</kbd>
            </div>
            <div className="shortcut-row">
              <span>Close drawer / dialog</span>
              <kbd>Esc</kbd>
            </div>
          </div>

          <h2 className="section-heading" style={{ marginTop: 18 }}>
            Local Storage & Privacy
          </h2>
          <p className="muted-note">
            LongPathFix v{version} runs 100% locally with zero network
            connections, zero telemetry, and MIT open-source licensing. Settings,
            JSONL diagnostics, and 1.x-compatible undo history live strictly under:
          </p>
          <div className="code-block-box" style={{ marginTop: 8 }}>
            <div className="code-block-val">
              Data: {dataDir}
              {diag ? `\nLog: ${diag.logFile}` : ""}
            </div>
          </div>
        </section>
      </div>

      <section className="surface" aria-label="Windows Path Rules Reference">
        <h2 className="section-heading">Windows Path & Filename Rules Explained</h2>
        <div className="muted-note" style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          <p style={{ margin: 0 }}>
            <strong>1. Classic MAX_PATH (259 usable characters) vs Extended \\?\ (32,767 UTF-16 code units):</strong>{" "}
            Win32 counts UTF-16 code units (surrogate-pair emojis count as 2). Many
            archivers, backup clients, and installers still enforce 259 characters
            even when <span className="mono">LongPathsEnabled</span> is enabled in the
            registry.
          </p>
          <p style={{ margin: 0 }}>
            <strong>2. Per-segment limit (255 code units):</strong> NTFS, exFAT, and
            ReFS limit each single folder or filename component to 255 UTF-16 code
            units regardless of <span className="mono">\\?\</span> or registry policy.
          </p>
          <p style={{ margin: 0 }}>
            <strong>3. Reserved DOS device names & invalid characters:</strong>{" "}
            Names like <span className="mono">CON</span>, <span className="mono">PRN</span>,{" "}
            <span className="mono">AUX</span>, <span className="mono">NUL</span>,{" "}
            <span className="mono">COM1–COM9</span>, <span className="mono">LPT1–LPT9</span>{" "}
            (plus superscript <span className="mono">COM¹²³/LPT¹²³</span> and any extension
            like <span className="mono">nul.txt</span>), trailing dots/spaces, and{" "}
            <span className="mono">&lt; &gt; : &quot; / \ | ? *</span> are blocked or
            misrouted by Win32 unless handled through extended paths.
          </p>
          <p style={{ margin: 0 }}>
            <strong>4. Case clashes & symlink safety:</strong> LongPathFix never
            follows NTFS junctions, directory symlinks, or mount points during
            traversal, preventing cyclic recursion and cross-volume escapes.
          </p>
        </div>
      </section>
    </div>
  );
}
