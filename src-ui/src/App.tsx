import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { DiagnosticsView } from "./components/DiagnosticsView";
import { FindingsWorkspace } from "./components/FindingsWorkspace";
import { HistoryView } from "./components/HistoryView";
import { RepairPreviewModal } from "./components/RepairPreviewModal";
import { SettingsGuideView } from "./components/SettingsGuideView";
import { Sidebar } from "./components/Sidebar";
import type {
  AppConfig,
  ApplyProgress,
  BootstrapDto,
  CommandErrorPayload,
  Finding,
  FindingsPageDto,
  IssueFilter,
  LongPathPolicy,
  NavTab,
  PlanSummaryDto,
  RepairCompleteDto,
  ScanProgress,
  SortBy,
  ThemePreference,
} from "./types";
import { FIX_KINDS, formatError } from "./types";

const DEFAULT_CONFIG: AppConfig = {
  theme: "system",
  defaultLimit: 259,
  customLimit: 220,
  defaultFixKinds: FIX_KINDS.ALL,
  clearReadonlyWhenAuthorized: false,
  recentRoots: [],
};

const EMPTY_PAGE: FindingsPageDto = {
  totalMatching: 0,
  fixableMatching: 0,
  items: [],
};

const PAGE_SIZE = 100;

function resolveTheme(pref: ThemePreference): "dark" | "light" {
  if (pref === "dark" || pref === "light") return pref;
  if (
    typeof window !== "undefined" &&
    window.matchMedia?.("(prefers-color-scheme: light)").matches
  ) {
    return "light";
  }
  return "dark";
}

export function App() {
  const [version, setVersion] = useState("2.0.0");
  const [config, setConfig] = useState<AppConfig>(DEFAULT_CONFIG);
  const [policy, setPolicy] = useState<LongPathPolicy | null>(null);
  const [dataDir, setDataDir] = useState("");
  const [historyCount, setHistoryCount] = useState(0);
  const [activeTab, setActiveTab] = useState<NavTab>("scan");

  const [rootInput, setRootInput] = useState("");
  const [limit, setLimit] = useState(259);
  const [kinds, setKinds] = useState<number>(FIX_KINDS.ALL);

  const [scanning, setScanning] = useState(false);
  const [scanProgress, setScanProgress] = useState<ScanProgress | null>(null);
  const [summary, setSummary] = useState<PlanSummaryDto | null>(null);
  const [pageData, setPageData] = useState<FindingsPageDto>(EMPTY_PAGE);
  const [pageOffset, setPageOffset] = useState(0);
  const [search, setSearch] = useState("");
  const [issueFilter, setIssueFilter] = useState<IssueFilter>("all");
  const [sortBy, setSortBy] = useState<SortBy>("length_desc");

  const [selectedRels, setSelectedRels] = useState<Set<string>>(new Set());
  const [inspectedRel, setInspectedRel] = useState<string | null>(null);
  const [banner, setBanner] = useState<{
    tone: "info" | "warn" | "danger" | "ok";
    text: string;
  } | null>(null);

  const [repairModalOpen, setRepairModalOpen] = useState(false);
  const [repairTargets, setRepairTargets] = useState<string[] | null>(null);
  const [repairRunning, setRepairRunning] = useState(false);
  const [repairProgress, setRepairProgress] = useState<ApplyProgress | null>(null);
  const [repairComplete, setRepairComplete] = useState<RepairCompleteDto | null>(null);
  const [repairError, setRepairError] = useState<string | null>(null);

  useEffect(() => {
    const apply = () => {
      document.documentElement.setAttribute(
        "data-theme",
        resolveTheme(config.theme)
      );
    };
    apply();
    const mq = window.matchMedia?.("(prefers-color-scheme: light)");
    mq?.addEventListener?.("change", apply);
    return () => mq?.removeEventListener?.("change", apply);
  }, [config.theme]);

  const refreshBootstrap = useCallback(async () => {
    try {
      const boot = await invoke<BootstrapDto>("get_bootstrap");
      setVersion(boot.version);
      setConfig(boot.config);
      setPolicy(boot.policy);
      setDataDir(boot.dataDir);
      setHistoryCount(boot.historyCount);
      setLimit(boot.config.defaultLimit || 259);
      setKinds(boot.config.defaultFixKinds ?? FIX_KINDS.ALL);
      if (boot.config.recentRoots.length > 0 && !rootInput) {
        setRootInput(boot.config.recentRoots[0]);
      }
    } catch (e) {
      setBanner({ tone: "danger", text: formatError(e) });
    }
  }, [rootInput]);

  useEffect(() => {
    void refreshBootstrap();
    // Run once on startup
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const fetchFindingsPage = useCallback(
    async (
      nextOffset: number,
      nextSearch: string,
      nextFilter: IssueFilter,
      nextSort: SortBy
    ) => {
      try {
        const res = await invoke<FindingsPageDto>("query_findings", {
          query: {
            search: nextSearch || null,
            issueFilter: nextFilter,
            sortBy: nextSort,
            offset: nextOffset,
            limit: PAGE_SIZE,
          },
        });
        setPageData(res);
      } catch {
        setPageData(EMPTY_PAGE);
      }
    },
    []
  );

  useEffect(() => {
    if (!summary) {
      setPageData(EMPTY_PAGE);
      return;
    }
    if (!search) {
      void fetchFindingsPage(pageOffset, search, issueFilter, sortBy);
      return;
    }
    const timer = window.setTimeout(() => {
      void fetchFindingsPage(pageOffset, search, issueFilter, sortBy);
    }, 60);
    return () => window.clearTimeout(timer);
  }, [summary, pageOffset, search, issueFilter, sortBy, fetchFindingsPage]);

  useEffect(() => {
    const unlisteners: Array<Promise<() => void>> = [];

    unlisteners.push(
      listen<ScanProgress>("scan-progress", (e) => {
        setScanProgress(e.payload);
      })
    );

    unlisteners.push(
      listen<PlanSummaryDto>("scan-complete", (e) => {
        setScanning(false);
        setScanProgress(null);
        setSummary(e.payload);
        setPageOffset(0);
        setSelectedRels(new Set());
        setInspectedRel(null);
        void invoke<BootstrapDto>("get_bootstrap")
          .then((boot) => {
            setConfig(boot.config);
            setHistoryCount(boot.historyCount);
          })
          .catch(() => {});
      })
    );

    unlisteners.push(
      listen("scan-cancelled", () => {
        setScanning(false);
        setScanProgress(null);
        setBanner({ tone: "info", text: "Scan cancelled." });
      })
    );

    unlisteners.push(
      listen<CommandErrorPayload>("scan-error", (e) => {
        setScanning(false);
        setScanProgress(null);
        setBanner({ tone: "danger", text: formatError(e.payload) });
      })
    );

    unlisteners.push(
      listen<ApplyProgress>("repair-progress", (e) => {
        setRepairProgress(e.payload);
      })
    );

    unlisteners.push(
      listen<RepairCompleteDto>("repair-complete", (e) => {
        setRepairRunning(false);
        setRepairProgress(null);
        setRepairComplete(e.payload);
        setSelectedRels(new Set());
        void invoke<BootstrapDto>("get_bootstrap")
          .then((boot) => setHistoryCount(boot.historyCount))
          .catch(() => {});
      })
    );

    unlisteners.push(
      listen<CommandErrorPayload>("repair-error", (e) => {
        setRepairRunning(false);
        setRepairProgress(null);
        setRepairError(formatError(e.payload));
      })
    );

    return () => {
      for (const p of unlisteners) {
        void p.then((fn) => fn());
      }
    };
  }, []);

  async function handleSaveConfig(next: AppConfig) {
    setConfig(next);
    try {
      const saved = await invoke<AppConfig>("set_config", { config: next });
      setConfig(saved);
    } catch (e) {
      setBanner({ tone: "danger", text: formatError(e) });
    }
  }

  function handleChangeTheme(theme: ThemePreference) {
    void handleSaveConfig({ ...config, theme });
  }

  async function handleStartScan(target: string, lim: number, k: number) {
    setBanner(null);
    setScanning(true);
    setScanProgress(null);
    try {
      await invoke("start_scan", {
        rootInput: target,
        limit: lim,
        kinds: k,
      });
    } catch (e) {
      setScanning(false);
      setBanner({ tone: "danger", text: formatError(e) });
    }
  }

  async function handleCancelOperation() {
    try {
      await invoke("cancel_operation");
    } catch (e) {
      setBanner({ tone: "danger", text: formatError(e) });
    }
  }

  async function handleChangePlanOptions(nextLimit: number, nextKinds: number) {
    setLimit(nextLimit);
    setKinds(nextKinds);
    setPageOffset(0);
    setSelectedRels(new Set());
    if (![259, 240, 200].includes(nextLimit) && config.customLimit !== nextLimit) {
      void handleSaveConfig({ ...config, customLimit: nextLimit });
    }
    if (!summary) return;
    try {
      const updated = await invoke<PlanSummaryDto>("update_plan_options", {
        limit: nextLimit,
        kinds: nextKinds,
      });
      setSummary(updated);
    } catch (e) {
      setBanner({ tone: "danger", text: formatError(e) });
    }
  }

  function handleOpenRepair(rels: string[] | null) {
    setRepairTargets(rels);
    setRepairComplete(null);
    setRepairError(null);
    setRepairProgress(null);
    setRepairModalOpen(true);
  }

  async function handleExecuteRepair(clearReadonly: boolean) {
    setRepairRunning(true);
    setRepairError(null);
    setRepairProgress(null);
    try {
      await invoke("execute_repair", {
        selectedRels: repairTargets,
        clearReadonly,
      });
    } catch (e) {
      setRepairRunning(false);
      setRepairError(formatError(e));
    }
  }

  async function handleUndoFromModal(batchId: string, clearReadonly: boolean) {
    try {
      await invoke("undo_batch", { id: batchId, clearReadonly });
      setRepairModalOpen(false);
      setBanner({
        tone: "ok",
        text: `Batch ${batchId} restored. Re-scan to inspect current disk state.`,
      });
      void refreshBootstrap();
      if (rootInput.trim()) {
        void handleStartScan(rootInput, limit, kinds);
      }
    } catch (e) {
      setRepairError(formatError(e));
    }
  }

  function toggleSelectedRel(rel: string) {
    setSelectedRels((prev) => {
      const next = new Set(prev);
      if (next.has(rel)) next.delete(rel);
      else next.add(rel);
      return next;
    });
  }

  function selectAllPage(findings: Finding[], select: boolean) {
    setSelectedRels((prev) => {
      const next = new Set(prev);
      for (const f of findings) {
        if (select) next.add(f.node.rel);
        else next.delete(f.node.rel);
      }
      return next;
    });
  }

  return (
    <div className="app-shell">
      <a href="#main-content" className="skip-link">
        Skip to main content
      </a>
      <Sidebar
        version={version}
        activeTab={activeTab}
        onSelectTab={setActiveTab}
        findingsCount={summary ? summary.findingsCount : null}
        historyCount={historyCount}
        policy={policy}
        theme={config.theme}
        onChangeTheme={handleChangeTheme}
      />

      <main className="main-pane" id="main-content" tabIndex={-1}>
        {activeTab === "scan" && (
          <FindingsWorkspace
            config={config}
            rootInput={rootInput}
            onChangeRootInput={setRootInput}
            limit={limit}
            kinds={kinds}
            onChangePlanOptions={handleChangePlanOptions}
            scanning={scanning}
            scanProgress={scanProgress}
            summary={summary}
            pageData={pageData}
            pageOffset={pageOffset}
            pageSize={PAGE_SIZE}
            search={search}
            issueFilter={issueFilter}
            sortBy={sortBy}
            onChangeSearch={(s) => {
              setSearch(s);
              setPageOffset(0);
            }}
            onChangeIssueFilter={(f) => {
              setIssueFilter(f);
              setPageOffset(0);
            }}
            onChangeSortBy={(s) => {
              setSortBy(s);
              setPageOffset(0);
            }}
            onChangePageOffset={setPageOffset}
            onStartScan={handleStartScan}
            onCancelScan={handleCancelOperation}
            onOpenRepair={handleOpenRepair}
            selectedRels={selectedRels}
            onToggleSelectedRel={toggleSelectedRel}
            onSelectAllPage={selectAllPage}
            onClearSelection={() => setSelectedRels(new Set())}
            inspectedRel={inspectedRel}
            onInspectRel={setInspectedRel}
            bannerMessage={banner}
            onDismissBanner={() => setBanner(null)}
          />
        )}

        {activeTab === "diagnostics" && (
          <DiagnosticsView
            limit={limit}
            onPolicyUpdated={() => void refreshBootstrap()}
          />
        )}

        {activeTab === "history" && (
          <HistoryView
            defaultClearReadonly={config.clearReadonlyWhenAuthorized}
            onHistoryChanged={() => void refreshBootstrap()}
          />
        )}

        {activeTab === "settings" && (
          <SettingsGuideView
            version={version}
            config={config}
            dataDir={dataDir}
            onSaveConfig={(next) => void handleSaveConfig(next)}
          />
        )}
      </main>

      {repairModalOpen && (
        <RepairPreviewModal
          selectedRels={repairTargets}
          defaultClearReadonly={config.clearReadonlyWhenAuthorized}
          progress={repairProgress}
          complete={repairComplete}
          error={repairError}
          running={repairRunning}
          onExecute={(cr) => void handleExecuteRepair(cr)}
          onCancelRunning={() => void handleCancelOperation()}
          onUndoBatch={(id, cr) => void handleUndoFromModal(id, cr)}
          onClose={() => setRepairModalOpen(false)}
        />
      )}
    </div>
  );
}

export default App;
