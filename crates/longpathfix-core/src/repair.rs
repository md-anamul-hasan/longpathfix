//! Safe dry-run verification, repair execution (deepest-first), and batch undo (reverse order).
//!
//! Only names change: nothing is moved between directories, deleted, or rewritten.
//! Every rename passes boundary/reparse checks, checks for destination collisions, executes via
//! Win32 `MoveFileExW(..., 0)`, verifies the target exists afterward, and saves the batch log
//! step-by-step so a crash or mid-batch cancellation leaves an accurate undo record.

use crate::filesystem;
use crate::path::{self, to_extended};
use crate::planner::RenameOp;
use crate::security::{validate_scan_root, verify_rename_pair};
use crate::storage::{DoneRename, HistoryStore, RenameBatch};
use crate::windows::LocalTimestamp;
use crate::{AppPaths, Error, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

/// Readiness classification of one planned rename before touching disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DryRunStatus {
    Ready,
    ReadOnly,
    TargetAlreadyExists,
    SourceMissing,
    LockedByProcess,
    InvalidBoundary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DryRunItem {
    pub rel: String,
    pub old_name: String,
    pub new_name: String,
    pub is_directory: bool,
    pub status: DryRunStatus,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DryRunReport {
    pub items: Vec<DryRunItem>,
    pub ready_count: usize,
    pub readonly_count: usize,
    pub conflict_count: usize,
    pub locked_count: usize,
    pub missing_count: usize,
    pub blocked_count: usize,
}

/// Per-item execution outcome for apply or undo.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameOutcome {
    pub path: String,
    pub ok: bool,
    pub message: String,
    pub skipped: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyProgress {
    pub completed: usize,
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub current_path: String,
}

#[derive(Debug, Clone)]
pub struct Renamer {
    store: HistoryStore,
}

impl Renamer {
    pub fn from_app_paths(paths: &AppPaths) -> Self {
        Self {
            store: HistoryStore::from_app_paths(paths),
        }
    }

    pub fn new(history_folder: Option<PathBuf>) -> Result<Self> {
        match history_folder {
            Some(dir) => Ok(Self {
                store: HistoryStore::with_history_dir(dir),
            }),
            None => Ok(Self::from_app_paths(&AppPaths::from_env()?)),
        }
    }

    pub fn store(&self) -> &HistoryStore {
        &self.store
    }

    /// Inspects every planned rename operation on disk without modifying anything.
    pub fn dry_run(&self, root_input: &str, ops: &[RenameOp]) -> Result<DryRunReport> {
        let root = validate_scan_root(root_input)?;
        let ordered = order_deepest_first(ops);

        let mut items = Vec::with_capacity(ordered.len());
        let mut ready_count = 0usize;
        let mut readonly_count = 0usize;
        let mut conflict_count = 0usize;
        let mut locked_count = 0usize;
        let mut missing_count = 0usize;
        let mut blocked_count = 0usize;

        for op in ordered {
            let dir_plain = root.join_plain(op.parent());
            let (from_ext, to_ext) =
                match verify_rename_pair(&root, &dir_plain, &op.old_name, &op.new_name) {
                    Ok(pair) => pair,
                    Err(e) => {
                        blocked_count += 1;
                        items.push(DryRunItem {
                            rel: op.rel.clone(),
                            old_name: op.old_name.clone(),
                            new_name: op.new_name.clone(),
                            is_directory: op.is_directory,
                            status: DryRunStatus::InvalidBoundary,
                            message: e.to_string(),
                        });
                        continue;
                    }
                };

            let src_meta = filesystem::metadata_extended(&from_ext);
            let exists_matching_kind = src_meta
                .as_ref()
                .map(|m| m.is_dir == op.is_directory)
                .unwrap_or(false);
            if !exists_matching_kind {
                missing_count += 1;
                items.push(DryRunItem {
                    rel: op.rel.clone(),
                    old_name: op.old_name.clone(),
                    new_name: op.new_name.clone(),
                    is_directory: op.is_directory,
                    status: DryRunStatus::SourceMissing,
                    message: "No longer exists on disk.".into(),
                });
                continue;
            }

            if filesystem::metadata_extended(&to_ext).is_some() {
                conflict_count += 1;
                items.push(DryRunItem {
                    rel: op.rel.clone(),
                    old_name: op.old_name.clone(),
                    new_name: op.new_name.clone(),
                    is_directory: op.is_directory,
                    status: DryRunStatus::TargetAlreadyExists,
                    message: format!("Target '{}' already exists.", op.new_name),
                });
                continue;
            }

            if filesystem::is_locked_for_rename(&from_ext, op.is_directory) {
                locked_count += 1;
                items.push(DryRunItem {
                    rel: op.rel.clone(),
                    old_name: op.old_name.clone(),
                    new_name: op.new_name.clone(),
                    is_directory: op.is_directory,
                    status: DryRunStatus::LockedByProcess,
                    message: "File is open or locked by another program.".into(),
                });
                continue;
            }

            if src_meta.as_ref().is_some_and(|m| m.is_readonly) {
                readonly_count += 1;
                items.push(DryRunItem {
                    rel: op.rel.clone(),
                    old_name: op.old_name.clone(),
                    new_name: op.new_name.clone(),
                    is_directory: op.is_directory,
                    status: DryRunStatus::ReadOnly,
                    message: "Read-only attribute is set.".into(),
                });
                continue;
            }

            ready_count += 1;
            items.push(DryRunItem {
                rel: op.rel.clone(),
                old_name: op.old_name.clone(),
                new_name: op.new_name.clone(),
                is_directory: op.is_directory,
                status: DryRunStatus::Ready,
                message: "Ready".into(),
            });
        }

        Ok(DryRunReport {
            items,
            ready_count,
            readonly_count,
            conflict_count,
            locked_count,
            missing_count,
            blocked_count,
        })
    }

    /// Synchronous convenience wrapper over [`Self::apply_with_progress`].
    pub fn apply(
        &self,
        root_input: &str,
        ops: &[RenameOp],
    ) -> Result<(Option<RenameBatch>, Vec<RenameOutcome>)> {
        let cancel = AtomicBool::new(false);
        self.apply_with_progress(root_input, ops, true, &cancel, |_| {})
    }

    /// Applies `ops` deepest-first with safety checks, optional read-only clearing/restoration,
    /// per-step persistence, cancellation, and progress events.
    pub fn apply_with_progress(
        &self,
        root_input: &str,
        ops: &[RenameOp],
        clear_readonly: bool,
        cancel: &AtomicBool,
        on_progress: impl Fn(ApplyProgress),
    ) -> Result<(Option<RenameBatch>, Vec<RenameOutcome>)> {
        let root = validate_scan_root(root_input)?;
        let ordered = order_deepest_first(ops);
        let total = ordered.len();
        if total == 0 {
            return Ok((None, Vec::new()));
        }

        let ts = LocalTimestamp::now();
        let mut batch = RenameBatch {
            id: ts.batch_id(),
            time: ts.iso_local(),
            root: root.plain.trim_end_matches('\\').to_string(),
            done: Vec::new(),
            undone: false,
        };
        std::fs::create_dir_all(self.store.history_dir())
            .map_err(|e| Error::io("Could not create history directory", e))?;
        self.store.save_batch(&batch)?;

        let mut outcomes = Vec::with_capacity(total);
        let mut succeeded = 0usize;
        let mut failed = 0usize;

        for (idx, op) in ordered.iter().enumerate() {
            if cancel.load(Ordering::Relaxed) {
                break;
            }

            let dir_plain = root.join_plain(op.parent());
            let display_from = join_display(&dir_plain, &op.old_name);

            on_progress(ApplyProgress {
                completed: idx,
                total,
                succeeded,
                failed,
                current_path: display_from.clone(),
            });

            let (from_ext, to_ext) =
                match verify_rename_pair(&root, &dir_plain, &op.old_name, &op.new_name) {
                    Ok(pair) => pair,
                    Err(e) => {
                        failed += 1;
                        outcomes.push(RenameOutcome {
                            path: display_from,
                            ok: false,
                            message: e.to_string(),
                            skipped: true,
                        });
                        continue;
                    }
                };

            let Some(src_meta) = filesystem::metadata_extended(&from_ext) else {
                failed += 1;
                outcomes.push(RenameOutcome {
                    path: display_from,
                    ok: false,
                    message: "Skipped: it no longer exists".into(),
                    skipped: true,
                });
                continue;
            };
            if src_meta.is_dir != op.is_directory {
                failed += 1;
                outcomes.push(RenameOutcome {
                    path: display_from,
                    ok: false,
                    message: "Skipped: it no longer exists".into(),
                    skipped: true,
                });
                continue;
            }

            if filesystem::metadata_extended(&to_ext).is_some() {
                failed += 1;
                outcomes.push(RenameOutcome {
                    path: display_from,
                    ok: false,
                    message: format!(
                        "Skipped: a file or folder named {} already exists",
                        op.new_name
                    ),
                    skipped: true,
                });
                continue;
            }

            let was_ro = src_meta.is_readonly;
            if was_ro && !clear_readonly {
                failed += 1;
                outcomes.push(RenameOutcome {
                    path: display_from,
                    ok: false,
                    message: "Skipped: read-only attribute is set (enable 'Also rename read-only items' to allow)".into(),
                    skipped: true,
                });
                continue;
            }

            if was_ro && clear_readonly {
                if let Err(e) = filesystem::set_readonly(&from_ext, false) {
                    failed += 1;
                    outcomes.push(RenameOutcome {
                        path: display_from,
                        ok: false,
                        message: format!("Not renamed: {}", filesystem::describe_io_error(&e)),
                        skipped: false,
                    });
                    continue;
                }
            }

            match filesystem::rename_no_overwrite(&from_ext, &to_ext) {
                Ok(()) => {
                    if was_ro {
                        let _ = filesystem::set_readonly(&to_ext, true);
                    }
                    batch.done.push(DoneRename {
                        dir: dir_plain.trim_end_matches('\\').to_string(),
                        old_name: op.old_name.clone(),
                        new_name: op.new_name.clone(),
                        is_directory: op.is_directory,
                    });
                    let _ = self.store.save_batch(&batch);
                    succeeded += 1;
                    outcomes.push(RenameOutcome {
                        path: display_from,
                        ok: true,
                        message: format!("Renamed to {}", op.new_name),
                        skipped: false,
                    });
                }
                Err(e) => {
                    if was_ro {
                        let _ = filesystem::set_readonly(&from_ext, true);
                    }
                    failed += 1;
                    outcomes.push(RenameOutcome {
                        path: display_from,
                        ok: false,
                        message: format!("Not renamed: {}", filesystem::describe_io_error(&e)),
                        skipped: false,
                    });
                }
            }
        }

        on_progress(ApplyProgress {
            completed: total,
            total,
            succeeded,
            failed,
            current_path: String::new(),
        });

        if batch.done.is_empty() {
            self.store.delete_batch_quiet(&batch.id);
            return Ok((None, outcomes));
        }

        self.store.append_csv_log(&batch, ts);
        Ok((Some(batch), outcomes))
    }

    pub fn undo(&self, batch: &mut RenameBatch) -> Vec<RenameOutcome> {
        let cancel = AtomicBool::new(false);
        self.undo_with_progress(batch, true, &cancel, |_| {})
            .unwrap_or_default()
    }

    /// Reverses a recorded batch (reverse order: shallowest parent directories restored before their renamed children).
    pub fn undo_with_progress(
        &self,
        batch: &mut RenameBatch,
        clear_readonly: bool,
        cancel: &AtomicBool,
        on_progress: impl Fn(ApplyProgress),
    ) -> Result<Vec<RenameOutcome>> {
        let root_norm = path::normalize_absolute(&batch.root)?;
        let root = path::Root::new(&root_norm)?;
        let total = batch.done.len();
        let mut outcomes = Vec::with_capacity(total);
        let mut succeeded = 0usize;
        let mut failed = 0usize;

        for (idx, d) in batch.done.iter().rev().enumerate() {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            let current_display = join_display(&d.dir, &d.new_name);
            let old_display = join_display(&d.dir, &d.old_name);

            on_progress(ApplyProgress {
                completed: idx,
                total,
                succeeded,
                failed,
                current_path: current_display.clone(),
            });

            let dir_norm = match path::normalize_absolute(&d.dir) {
                Ok(n) => n,
                Err(_) => d.dir.clone(),
            };
            let (from_ext, to_ext) = if path::is_within(&dir_norm, &root.plain) {
                match verify_rename_pair(&root, &dir_norm, &d.new_name, &d.old_name) {
                    Ok(pair) => pair,
                    Err(e) => {
                        failed += 1;
                        outcomes.push(RenameOutcome {
                            path: current_display,
                            ok: false,
                            message: e.to_string(),
                            skipped: true,
                        });
                        continue;
                    }
                }
            } else {
                failed += 1;
                outcomes.push(RenameOutcome {
                    path: current_display,
                    ok: false,
                    message: "Skipped: batch entry points outside its original root folder".into(),
                    skipped: true,
                });
                continue;
            };

            let Some(src_meta) = filesystem::metadata_extended(&from_ext) else {
                failed += 1;
                outcomes.push(RenameOutcome {
                    path: current_display,
                    ok: false,
                    message: "Skipped: it was renamed or removed since".into(),
                    skipped: true,
                });
                continue;
            };
            if src_meta.is_dir != d.is_directory {
                failed += 1;
                outcomes.push(RenameOutcome {
                    path: current_display,
                    ok: false,
                    message: "Skipped: it was renamed or removed since".into(),
                    skipped: true,
                });
                continue;
            }

            if filesystem::metadata_extended(&to_ext).is_some() {
                failed += 1;
                outcomes.push(RenameOutcome {
                    path: current_display,
                    ok: false,
                    message: "Skipped: the old name is taken again".into(),
                    skipped: true,
                });
                continue;
            }

            let was_ro = src_meta.is_readonly;
            if was_ro && clear_readonly {
                let _ = filesystem::set_readonly(&from_ext, false);
            }

            match filesystem::rename_no_overwrite(&from_ext, &to_ext) {
                Ok(()) => {
                    if was_ro && clear_readonly {
                        let _ = filesystem::set_readonly(&to_ext, true);
                    }
                    succeeded += 1;
                    outcomes.push(RenameOutcome {
                        path: old_display,
                        ok: true,
                        message: "Restored the old name".into(),
                        skipped: false,
                    });
                }
                Err(e) => {
                    if was_ro && clear_readonly {
                        let _ = filesystem::set_readonly(&from_ext, true);
                    }
                    failed += 1;
                    outcomes.push(RenameOutcome {
                        path: current_display,
                        ok: false,
                        message: format!("Not restored: {}", filesystem::describe_io_error(&e)),
                        skipped: false,
                    });
                }
            }
        }

        // Handle plain-path fallback if root was a UNC or drive root without trailing slash matching
        let _ = to_extended(&root.plain);

        on_progress(ApplyProgress {
            completed: total,
            total,
            succeeded,
            failed,
            current_path: String::new(),
        });

        batch.undone = !cancel.load(Ordering::Relaxed)
            && outcomes.len() == total
            && outcomes.iter().all(|o| o.ok);
        let _ = self.store.save_batch(batch);
        Ok(outcomes)
    }
}

fn order_deepest_first(ops: &[RenameOp]) -> Vec<&RenameOp> {
    let mut ordered: Vec<&RenameOp> = ops.iter().collect();
    ordered.sort_by(|a, b| {
        let da = a.rel.bytes().filter(|&x| x == b'\\').count();
        let db = b.rel.bytes().filter(|&x| x == b'\\').count();
        db.cmp(&da).then_with(|| a.rel.cmp(&b.rel))
    });
    ordered
}

fn join_display(dir: &str, name: &str) -> String {
    if dir.ends_with('\\') {
        format!("{dir}{name}")
    } else {
        format!("{dir}\\{name}")
    }
}
