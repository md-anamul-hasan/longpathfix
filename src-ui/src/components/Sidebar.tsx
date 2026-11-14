import type { ReactElement } from "react";
import {
  BrandIcon,
  FolderSearchIcon,
  HistoryIcon,
  ShieldCheckIcon,
  SlidersIcon,
} from "./Icons";
import type { LongPathPolicy, NavTab, ThemePreference } from "../types";

interface SidebarProps {
  version: string;
  activeTab: NavTab;
  onSelectTab: (tab: NavTab) => void;
  findingsCount: number | null;
  historyCount: number;
  policy: LongPathPolicy | null;
  theme: ThemePreference;
  onChangeTheme: (theme: ThemePreference) => void;
}

export function Sidebar({
  version,
  activeTab,
  onSelectTab,
  findingsCount,
  historyCount,
  policy,
  theme,
  onChangeTheme,
}: SidebarProps) {
  const navItems: Array<{
    id: NavTab;
    label: string;
    badge?: number | null;
    icon: ReactElement;
  }> = [
    {
      id: "scan",
      label: "Scan & Fix",
      badge: findingsCount,
      icon: <FolderSearchIcon size={16} />,
    },
    {
      id: "diagnostics",
      label: "Path Lab & System",
      icon: <ShieldCheckIcon size={16} />,
    },
    {
      id: "history",
      label: "History & Undo",
      badge: historyCount > 0 ? historyCount : null,
      icon: <HistoryIcon size={16} />,
    },
    {
      id: "settings",
      label: "Settings & Guide",
      icon: <SlidersIcon size={16} />,
    },
  ];

  const policyOk = policy?.longPathsEnabled === true;

  return (
    <aside className="rail" aria-label="Application navigation">
      <div className="rail-top">
        <div className="brand">
          <div className="brand-mark">
            <BrandIcon size={18} />
          </div>
          <div className="brand-text">
            <span className="brand-name">LongPathFix</span>
            <span className="brand-version">v{version}</span>
          </div>
        </div>

        <nav className="nav-list" aria-label="Primary sections">
          {navItems.map((item) => (
            <button
              key={item.id}
              type="button"
              className={`nav-item ${activeTab === item.id ? "active" : ""}`}
              aria-current={activeTab === item.id ? "page" : undefined}
              onClick={() => onSelectTab(item.id)}
            >
              <span className="nav-item-main">
                {item.icon}
                <span>{item.label}</span>
              </span>
              {item.badge !== undefined && item.badge !== null && (
                <span className="nav-badge">{item.badge.toLocaleString()}</span>
              )}
            </button>
          ))}
        </nav>
      </div>

      <div className="rail-footer">
        <button
          type="button"
          className="policy-pill"
          onClick={() => onSelectTab("diagnostics")}
          title="Inspect Windows LongPathsEnabled policy and volume compatibility"
        >
          <span className="nav-item-main">
            <span
              className={`status-dot ${policyOk ? "ok" : "warn"}`}
              aria-hidden="true"
            />
            <span>
              {policy === null
                ? "Checking policy…"
                : policyOk
                  ? "LongPathsEnabled: On"
                  : "LongPathsEnabled: Off"}
            </span>
          </span>
        </button>

        <div className="theme-switch" role="group" aria-label="Color theme">
          {(["system", "dark", "light"] as ThemePreference[]).map((t) => (
            <button
              key={t}
              type="button"
              className={`theme-btn ${theme === t ? "active" : ""}`}
              aria-pressed={theme === t}
              onClick={() => onChangeTheme(t)}
            >
              {t === "system" ? "Auto" : t === "dark" ? "Dark" : "Light"}
            </button>
          ))}
        </div>
      </div>
    </aside>
  );
}
