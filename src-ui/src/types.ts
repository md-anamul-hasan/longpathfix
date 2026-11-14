export type ThemePreference = "system" | "dark" | "light";

export type NavTab = "scan" | "diagnostics" | "history" | "settings";

export const FIX_KINDS = {
  NONE: 0,
  LONG_PATHS: 1,
  ILLEGAL_CHARS: 2,
  TRAILING_DOTS_AND_SPACES: 4,
  RESERVED_NAMES: 8,
  CASE_CLASHES: 16,
  ALL: 31,
} as const;

export const ISSUE_BITS = {
  TOO_LONG: 1,
  NAME_TOO_LONG: 2,
  TRAILING_DOT_OR_SPACE: 4,
  LEADING_SPACE: 8,
  RESERVED_NAME: 16,
  ILLEGAL_CHARS: 32,
  CASE_CLASH: 64,
  INVALID_UNICODE: 128,
} as const;

export type IssueFilter =
  | "all"
  | "fixable"
  | "too_long"
  | "name_too_long"
  | "trailing"
  | "reserved"
  | "illegal"
  | "case_clash"
  | "invalid_unicode"
  | "unresolved"
  | "readonly"
  | "reparse";

export type SortBy = "length_desc" | "length_asc" | "path_asc" | "severity";

export interface AppConfig {
  theme: ThemePreference;
  defaultLimit: number;
  customLimit: number;
  defaultFixKinds: number;
  clearReadonlyWhenAuthorized: boolean;
  recentRoots: string[];
}

export interface LongPathPolicy {
  longPathsEnabled: boolean;
  registryValue: number | null;
  processLongPathAware: boolean;
  isElevated: boolean;
  osBuild: number;
  classicMaxPath: number;
  extendedMaxPath: number;
}

export interface VolumeInfo {
  rootPath: string;
  volumeLabel: string;
  filesystemName: string;
  maxComponentLength: number;
  supportsUnicodeOnDisk: boolean;
  supportsReparsePoints: boolean;
  supportsCaseSensitiveDirs: boolean;
  isReadOnly: boolean;
  driveType: string;
}

export interface BootstrapDto {
  version: string;
  config: AppConfig;
  policy: LongPathPolicy;
  volumes: VolumeInfo[];
  dataDir: string;
  historyCount: number;
}

export interface RootInspectionDto {
  valid: boolean;
  plain: string | null;
  extended: string | null;
  len16: number;
  warning: string | null;
  volume: VolumeInfo | null;
  error: string | null;
}

export interface ScanProgress {
  scannedFiles: number;
  scannedDirs: number;
  skippedReparsePoints: number;
  inaccessibleDirs: number;
  maxDepth: number;
  maxPathLen: number;
  currentDir: string;
}

export interface ScanWarning {
  path: string;
  message: string;
}

export interface IssueCounts {
  tooLong: number;
  nameTooLong: number;
  trailingDotOrSpace: number;
  leadingSpace: number;
  reservedName: number;
  illegalChars: number;
  caseClash: number;
  invalidUnicode: number;
}

export interface LengthBucket {
  label: string;
  min: number;
  max: number;
  before: number;
  after: number;
}

export interface PlanSummaryDto {
  rootPlain: string;
  rootExtended: string;
  rootLength: number;
  limit: number;
  kinds: number;
  scanned: number;
  scannedFiles: number;
  scannedDirs: number;
  skippedReparsePoints: number;
  readonlyFindings: number;
  findingsCount: number;
  opsCount: number;
  unresolvedCount: number;
  maxLenBefore: number;
  maxLenAfter: number;
  elapsedMs: number;
  warningsCount: number;
  warnings: ScanWarning[];
  issueCounts: IssueCounts;
  histogram: LengthBucket[];
}

export interface NodeDto {
  rel: string;
  isDirectory: boolean;
  isReparsePoint: boolean;
  isReadonly: boolean;
  invalidUnicode: boolean;
}

export interface Finding {
  node: NodeDto;
  issues: number;
  newName: string | null;
  unresolved: string | null;
  currentLen: number;
  plannedLen: number;
  issueText: string;
}

export interface FindingsPageDto {
  totalMatching: number;
  fixableMatching: number;
  items: Finding[];
}

export interface SegmentBreakdownDto {
  name: string;
  newName: string | null;
  utf16Len: number;
  newUtf16Len: number;
  issues: string[];
  isRoot: boolean;
}

export interface PathInspectionDto {
  rel: string;
  plainPath: string;
  extendedPath: string;
  isDirectory: boolean;
  isReparsePoint: boolean;
  isReadonly: boolean;
  isLocked: boolean;
  size: number | null;
  currentLen: number;
  plannedLen: number;
  limit: number;
  issueLabels: string[];
  unresolved: string | null;
  segments: SegmentBreakdownDto[];
}

export type PathForm = "drive" | "unc" | "extended_drive" | "extended_unc";

export interface PathAnalysisDto {
  input: string;
  form: PathForm;
  normalizedPlain: string | null;
  extendedPath: string | null;
  utf16Len: number;
  exceedsClassic: boolean;
  existsOnDisk: boolean;
  isDirectory: boolean | null;
  isReadonly: boolean | null;
  isReparsePoint: boolean | null;
  segments: SegmentBreakdownDto[];
  error: string | null;
}

export interface CustomNameCheckDto {
  valid: boolean;
  utf16Len: number;
  issues: string[];
  error: string | null;
}

export type DryRunStatus =
  | "ready"
  | "read_only"
  | "target_already_exists"
  | "source_missing"
  | "locked_by_process"
  | "invalid_boundary";

export interface DryRunItem {
  rel: string;
  oldName: string;
  newName: string;
  isDirectory: boolean;
  status: DryRunStatus;
  message: string;
}

export interface DryRunReport {
  items: DryRunItem[];
  readyCount: number;
  readonlyCount: number;
  conflictCount: number;
  lockedCount: number;
  missingCount: number;
  blockedCount: number;
}

export interface ApplyProgress {
  completed: number;
  total: number;
  succeeded: number;
  failed: number;
  currentPath: string;
}

export interface RenameOutcome {
  path: string;
  ok: boolean;
  message: string;
  skipped: boolean;
}

/* Preserves 1.x PascalCase disk format in %APPDATA%/LongPathFix/history */
export interface DoneRename {
  Dir: string;
  OldName: string;
  NewName: string;
  IsDirectory: boolean;
}

export interface RenameBatch {
  Id: string;
  Time: string;
  Root: string;
  Done: DoneRename[];
  Undone: boolean;
}

export interface RepairCompleteDto {
  batch: RenameBatch | null;
  outcomes: RenameOutcome[];
  succeeded: number;
  failed: number;
  cancelled: boolean;
}

export interface HistorySummaryDto {
  id: string;
  time: string;
  displayTime: string;
  root: string;
  count: number;
  undone: boolean;
  title: string;
}

export interface SystemDiagnosticsDto {
  policy: LongPathPolicy;
  volumes: VolumeInfo[];
  dataDir: string;
  historyDir: string;
  logFile: string;
}

export interface EnableLongPathsResultDto {
  enabled: boolean;
  policy: LongPathPolicy;
}

export interface CommandErrorPayload {
  kind?: string;
  message?: string;
}

export function formatError(err: unknown): string {
  if (typeof err === "string") return err;
  if (err && typeof err === "object") {
    const p = err as CommandErrorPayload;
    if (typeof p.message === "string" && p.message.length > 0) return p.message;
  }
  return "An unexpected error occurred.";
}

const CRITICAL_ISSUE_MASK =
  ISSUE_BITS.NAME_TOO_LONG |
  ISSUE_BITS.RESERVED_NAME |
  ISSUE_BITS.ILLEGAL_CHARS |
  ISSUE_BITS.INVALID_UNICODE;

export function issueTone(issues: number): "danger" | "warn" {
  return (issues & CRITICAL_ISSUE_MASK) !== 0 ? "danger" : "warn";
}

export function issueLabelTone(label: string): "danger" | "warn" {
  const lower = label.toLowerCase();
  if (
    lower.includes("illegal") ||
    lower.includes("reserved") ||
    lower.includes("255") ||
    lower.includes("utf-16") ||
    lower.includes("unicode")
  ) {
    return "danger";
  }
  return "warn";
}
