import type { IssueFilter, PlanSummaryDto } from "../types";

interface PathGaugeProps {
  summary: PlanSummaryDto;
  activeFilter: IssueFilter;
  onSelectFilter: (filter: IssueFilter) => void;
}

export function PathGauge({
  summary,
  activeFilter,
  onSelectFilter,
}: PathGaugeProps) {
  const maxBucketCount = Math.max(
    1,
    ...summary.histogram.map((b) => Math.max(b.before, b.after))
  );

  type ChipDef = {
    filter: IssueFilter;
    label: string;
    count: number;
    tone?: "warn" | "danger";
  };

  const allChips: ChipDef[] = [
    { filter: "all", label: "All findings", count: summary.findingsCount },
    { filter: "too_long", label: "Path > limit", count: summary.issueCounts.tooLong, tone: "warn" },
    { filter: "name_too_long", label: "Segment > 255", count: summary.issueCounts.nameTooLong, tone: "danger" },
    {
      filter: "trailing",
      label: "Leading/trailing",
      count: summary.issueCounts.trailingDotOrSpace + summary.issueCounts.leadingSpace,
    },
    { filter: "reserved", label: "Reserved device", count: summary.issueCounts.reservedName, tone: "danger" },
    { filter: "illegal", label: "Illegal chars", count: summary.issueCounts.illegalChars, tone: "danger" },
    { filter: "case_clash", label: "Case clash", count: summary.issueCounts.caseClash, tone: "warn" },
    { filter: "invalid_unicode", label: "Invalid UTF-16", count: summary.issueCounts.invalidUnicode, tone: "danger" },
    { filter: "unresolved", label: "Unresolved", count: summary.unresolvedCount, tone: "warn" },
    { filter: "readonly", label: "Read-only", count: summary.readonlyFindings },
  ];

  const issueChips = allChips.filter((c) => c.filter === "all" || c.count > 0);

  const afterExceedsLimit = summary.maxLenAfter > summary.limit;

  return (
    <section className="surface gauge-card" aria-label="Path length distribution and plan metrics">
      <div className="gauge-head">
        <div className="gauge-metrics">
          <div className="metric-pair">
            <span className="metric-val tabular">{summary.scanned.toLocaleString()}</span>
            <span className="metric-lbl">
              entries ({summary.scannedFiles.toLocaleString()} files,{" "}
              {summary.scannedDirs.toLocaleString()} folders)
            </span>
          </div>
          <div className="metric-pair">
            <span
              className={`metric-val tabular ${
                summary.findingsCount === 0 ? "ok" : "warn"
              }`}
            >
              {summary.findingsCount.toLocaleString()}
            </span>
            <span className="metric-lbl">issues</span>
          </div>
          <div className="metric-pair">
            <span className="metric-val tabular ok">
              {summary.opsCount.toLocaleString()}
            </span>
            <span className="metric-lbl">planned renames</span>
          </div>
          <div className="metric-pair">
            <span className="metric-val tabular">
              {summary.maxLenBefore} →{" "}
              <span className={afterExceedsLimit ? "warn" : "ok"}>
                {summary.maxLenAfter}
              </span>
            </span>
            <span className="metric-lbl">max UTF-16 units (limit {summary.limit})</span>
          </div>
          <div className="metric-pair">
            <span className="metric-val tabular">{summary.elapsedMs} ms</span>
            <span className="metric-lbl">scan time</span>
          </div>
        </div>

        <div className="gauge-legend" aria-hidden="true">
          <span className="legend-item">
            <span className="legend-swatch before" />
            Before
          </span>
          <span className="legend-item">
            <span className="legend-swatch after" />
            After plan
          </span>
        </div>
      </div>

      <div className="histogram-grid" role="group" aria-label="Path length histogram buckets">
        {summary.histogram.map((bucket) => {
          const beforePct = Math.round((bucket.before / maxBucketCount) * 100);
          const afterPct = Math.round((bucket.after / maxBucketCount) * 100);
          const hasEntries = bucket.before > 0 || bucket.after > 0;
          const isThreshold = bucket.min <= summary.limit && summary.limit <= bucket.max;
          const overLimit = hasEntries && bucket.min > summary.limit;
          return (
            <div
              key={bucket.label}
              className="bucket-col"
              aria-label={`${bucket.label} UTF-16 code units: ${bucket.before} before, ${bucket.after} after planned renames`}
            >
              <div
                className={`bucket-bars ${overLimit ? "over-limit" : ""} ${
                  isThreshold ? "threshold-boundary" : ""
                }`}
              >
                <div
                  className="bar-before"
                  style={{ height: `${bucket.before > 0 ? Math.max(8, beforePct) : 0}%` }}
                  title={`Before: ${bucket.before.toLocaleString()}`}
                />
                <div
                  className="bar-after"
                  style={{ height: `${bucket.after > 0 ? Math.max(8, afterPct) : 0}%` }}
                  title={`After plan: ${bucket.after.toLocaleString()}`}
                />
              </div>
              <div className="bucket-meta">
                <span className="bucket-range">
                  {bucket.label}
                  {isThreshold && (
                    <span className="bucket-threshold-pill" title={`Limit ${summary.limit}`}>
                      limit
                    </span>
                  )}
                </span>
                <span className="bucket-count">
                  {bucket.before}→{bucket.after}
                </span>
              </div>
            </div>
          );
        })}
      </div>

      <div className="filter-pills" role="group" aria-label="Filter findings by issue category">
        {issueChips.map((chip) => (
          <button
            key={chip.filter}
            type="button"
            className={`filter-pill ${chip.tone ? `tone-${chip.tone}` : ""} ${
              activeFilter === chip.filter ? "active" : ""
            }`}
            aria-pressed={activeFilter === chip.filter}
            onClick={() => onSelectFilter(chip.filter)}
          >
            <span>{chip.label}</span>
            <span className="mono">{chip.count.toLocaleString()}</span>
          </button>
        ))}
      </div>
    </section>
  );
}
