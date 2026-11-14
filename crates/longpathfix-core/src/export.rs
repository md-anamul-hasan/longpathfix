//! UTF-8 BOM CSV exports for findings and rename execution outcomes.

use crate::filesystem::atomic_write;
use crate::planner::Finding;
use crate::repair::RenameOutcome;
use crate::storage::csv_escape;
use crate::Result;
use std::path::Path;

pub fn format_findings_csv(findings: &[Finding]) -> Vec<u8> {
    let mut out = Vec::with_capacity(findings.len() * 140 + 64);
    out.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    out.extend_from_slice(b"Path,Type,Current length,Planned length,Problems,Planned new name,Cannot be fixed because\r\n");

    for f in findings {
        let kind = if f.node.is_reparse_point {
            "link/junction"
        } else if f.node.is_directory {
            "folder"
        } else {
            "file"
        };
        let row = format!(
            "{},{},{},{},{},{},{}\r\n",
            csv_escape(&f.node.rel),
            kind,
            f.current_len,
            f.planned_len,
            csv_escape(&f.issue_text),
            csv_escape(f.new_name.as_deref().unwrap_or("")),
            csv_escape(f.unresolved.as_deref().unwrap_or("")),
        );
        out.extend_from_slice(row.as_bytes());
    }
    out
}

pub fn export_findings_csv(path: &Path, findings: &[Finding]) -> Result<()> {
    let bytes = format_findings_csv(findings);
    atomic_write(path, &bytes)
}

pub fn format_outcomes_csv(outcomes: &[RenameOutcome]) -> Vec<u8> {
    let mut out = Vec::with_capacity(outcomes.len() * 110 + 48);
    out.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    out.extend_from_slice(b"Path,Status,Details\r\n");

    for o in outcomes {
        let status = if o.ok {
            "ok"
        } else if o.skipped {
            "skipped"
        } else {
            "error"
        };
        let row = format!(
            "{},{},{}\r\n",
            csv_escape(&o.path),
            status,
            csv_escape(&o.message),
        );
        out.extend_from_slice(row.as_bytes());
    }
    out
}

pub fn export_outcomes_csv(path: &Path, outcomes: &[RenameOutcome]) -> Result<()> {
    let bytes = format_outcomes_csv(outcomes);
    atomic_write(path, &bytes)
}
