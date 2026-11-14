use crate::state::{AppState, OperationGuard, PlanSummaryDto, ScanSession};
use longpathfix_core::config::{AppConfig, ThemePreference};
use longpathfix_core::export;
use longpathfix_core::filesystem;
use longpathfix_core::path::{
    self, normalize_absolute, to_extended, to_plain, utf16_len, PathForm,
};
use longpathfix_core::planner::{Finding, Plan, Planner, RenameOp};
use longpathfix_core::repair::{DryRunReport, RenameOutcome};
use longpathfix_core::rules::{self, FixKinds, Issue};
use longpathfix_core::scanner::scan_with_progress;
use longpathfix_core::security::{
    protected_path_warning, validate_scan_root, validate_single_name,
};
use longpathfix_core::storage::RenameBatch;
use longpathfix_core::windows::{
    self, inspect_volume_for_path, list_logical_volumes, LongPathPolicy, VolumeInfo,
};
use longpathfix_core::{Error, Result, VERSION};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, State, Theme};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapDto {
    pub version: &'static str,
    pub config: AppConfig,
    pub policy: LongPathPolicy,
    pub volumes: Vec<VolumeInfo>,
    pub data_dir: String,
    pub history_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RootInspectionDto {
    pub valid: bool,
    pub plain: Option<String>,
    pub extended: Option<String>,
    pub len16: usize,
    pub warning: Option<String>,
    pub volume: Option<VolumeInfo>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FindingsQuery {
    pub search: Option<String>,
    pub issue_filter: Option<String>,
    pub sort_by: Option<String>,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FindingsPageDto {
    pub total_matching: usize,
    pub fixable_matching: usize,
    pub items: Vec<Finding>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentBreakdownDto {
    pub name: String,
    pub new_name: Option<String>,
    pub utf16_len: usize,
    pub new_utf16_len: usize,
    pub issues: Vec<&'static str>,
    pub is_root: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathInspectionDto {
    pub rel: String,
    pub plain_path: String,
    pub extended_path: String,
    pub is_directory: bool,
    pub is_reparse_point: bool,
    pub is_readonly: bool,
    pub is_locked: bool,
    pub size: Option<u64>,
    pub current_len: usize,
    pub planned_len: usize,
    pub limit: usize,
    pub issue_labels: Vec<&'static str>,
    pub unresolved: Option<String>,
    pub segments: Vec<SegmentBreakdownDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathAnalysisDto {
    pub input: String,
    pub form: PathForm,
    pub normalized_plain: Option<String>,
    pub extended_path: Option<String>,
    pub utf16_len: usize,
    pub exceeds_classic: bool,
    pub exists_on_disk: bool,
    pub is_directory: Option<bool>,
    pub is_readonly: Option<bool>,
    pub is_reparse_point: Option<bool>,
    pub segments: Vec<SegmentBreakdownDto>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairCompleteDto {
    pub batch: Option<RenameBatch>,
    pub outcomes: Vec<RenameOutcome>,
    pub succeeded: usize,
    pub failed: usize,
    pub cancelled: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistorySummaryDto {
    pub id: String,
    pub time: String,
    pub display_time: String,
    pub root: String,
    pub count: usize,
    pub undone: bool,
    pub title: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemDiagnosticsDto {
    pub policy: LongPathPolicy,
    pub volumes: Vec<VolumeInfo>,
    pub data_dir: String,
    pub history_dir: String,
    pub log_file: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnableLongPathsResultDto {
    pub enabled: bool,
    pub policy: LongPathPolicy,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomNameCheckDto {
    pub valid: bool,
    pub utf16_len: usize,
    pub issues: Vec<&'static str>,
    pub error: Option<String>,
}

pub(crate) fn apply_window_theme(app: &AppHandle, pref: ThemePreference) {
    if let Some(win) = app.get_webview_window("main") {
        let tauri_theme = match pref {
            ThemePreference::System => None,
            ThemePreference::Dark => Some(Theme::Dark),
            ThemePreference::Light => Some(Theme::Light),
        };
        let _ = win.set_theme(tauri_theme);
    }
}

#[tauri::command]
pub fn get_bootstrap(app: AppHandle, state: State<'_, AppState>) -> Result<BootstrapDto> {
    let config = state
        .config
        .lock()
        .map_err(|_| Error::Busy("Config lock poisoned".into()))?
        .clone();
    apply_window_theme(&app, config.theme);
    let history_count = state.history_store.list_batches().len();
    Ok(BootstrapDto {
        version: VERSION,
        config,
        policy: LongPathPolicy::detect(),
        volumes: list_logical_volumes(),
        data_dir: state.app_paths.data.to_string_lossy().into_owned(),
        history_count,
    })
}

#[tauri::command]
pub fn set_config(
    app: AppHandle,
    state: State<'_, AppState>,
    config: AppConfig,
) -> Result<AppConfig> {
    let clean = config.sanitized();
    state.config_store.save(&clean)?;
    apply_window_theme(&app, clean.theme);
    let mut guard = state
        .config
        .lock()
        .map_err(|_| Error::Busy("Config lock poisoned".into()))?;
    *guard = clean.clone();
    Ok(clean)
}

#[tauri::command]
pub fn inspect_root(root_input: String) -> RootInspectionDto {
    match validate_scan_root(&root_input) {
        Ok(root) => RootInspectionDto {
            valid: true,
            warning: protected_path_warning(&root.plain),
            volume: inspect_volume_for_path(&root.plain),
            len16: root.len16,
            plain: Some(root.plain),
            extended: Some(root.extended),
            error: None,
        },
        Err(e) => RootInspectionDto {
            valid: false,
            plain: None,
            extended: None,
            len16: 0,
            warning: None,
            volume: None,
            error: Some(e.to_string()),
        },
    }
}

#[tauri::command]
pub fn start_scan(
    app: AppHandle,
    state: State<'_, AppState>,
    root_input: String,
    limit: usize,
    kinds: u32,
) -> Result<()> {
    let validated_root = validate_scan_root(&root_input)?;
    state.try_begin_operation("scan")?;

    let clamped_limit = limit.clamp(40, 1000);
    let fix_kinds = FixKinds(kinds);
    let cancel = state.cancel_flag.clone();
    let app_for_thread = app.clone();

    std::thread::spawn(move || {
        let app_state = app_for_thread.state::<AppState>();
        let _guard = OperationGuard::new(&app_state);
        let emit_app = app_for_thread.clone();

        match scan_with_progress(&validated_root.plain, &cancel, move |prog| {
            let _ = emit_app.emit("scan-progress", &prog);
        }) {
            Ok(out) => {
                let plan = Planner::build(&out.root.plain, &out.nodes, clamped_limit, fix_kinds);
                let session = ScanSession {
                    root: out.root.clone(),
                    nodes: out.nodes,
                    warnings: out.warnings,
                    elapsed_ms: out.elapsed_ms,
                    plan,
                    kinds: fix_kinds,
                };
                let summary = PlanSummaryDto::from_session(&session);
                if let Ok(mut guard) = app_state.session.lock() {
                    *guard = Some(session);
                }
                if let Ok(mut cfg) = app_state.config.lock() {
                    cfg.push_recent_root(&out.root.plain);
                    let _ = app_state.config_store.save(&cfg);
                }
                app_state.logger.info(
                    "scan",
                    &format!(
                        "Scanned {} entries under '{}' in {} ms ({} findings, {} ops)",
                        summary.scanned,
                        summary.root_plain,
                        summary.elapsed_ms,
                        summary.findings_count,
                        summary.ops_count
                    ),
                );
                let _ = app_for_thread.emit("scan-complete", &summary);
            }
            Err(Error::Cancelled) => {
                app_state.logger.info("scan", "Scan cancelled by user");
                let _ = app_for_thread.emit("scan-cancelled", ());
            }
            Err(e) => {
                app_state.logger.error("scan", &e.to_string());
                let _ = app_for_thread.emit("scan-error", &e);
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub fn cancel_operation(state: State<'_, AppState>) {
    state.cancel_flag.store(true, Ordering::SeqCst);
}

#[tauri::command]
pub fn update_plan_options(
    state: State<'_, AppState>,
    limit: usize,
    kinds: u32,
) -> Result<PlanSummaryDto> {
    state.rebuild_plan(limit, FixKinds(kinds))
}

fn filter_findings<'a>(plan: &'a Plan, search: &str, issue_filter: &str) -> Vec<&'a Finding> {
    let needle = search.trim().to_lowercase();
    plan.findings
        .iter()
        .filter(|f| {
            let matches_issue = match issue_filter {
                "" | "all" => true,
                "too_long" => f.issues.contains(Issue::TOO_LONG),
                "name_too_long" => f.issues.contains(Issue::NAME_TOO_LONG),
                "trailing" => {
                    f.issues.contains(Issue::TRAILING_DOT_OR_SPACE)
                        || f.issues.contains(Issue::LEADING_SPACE)
                }
                "reserved" => f.issues.contains(Issue::RESERVED_NAME),
                "illegal" => f.issues.contains(Issue::ILLEGAL_CHARS),
                "case_clash" => f.issues.contains(Issue::CASE_CLASH),
                "invalid_unicode" => f.issues.contains(Issue::INVALID_UNICODE),
                "unresolved" => f.unresolved.is_some(),
                "readonly" => f.node.is_readonly,
                "reparse" => f.node.is_reparse_point,
                "fixable" => f.new_name.is_some(),
                _ => true,
            };
            if !matches_issue {
                return false;
            }
            if needle.is_empty() {
                return true;
            }
            f.node.rel.to_lowercase().contains(&needle)
                || f.new_name
                    .as_deref()
                    .map(|n| n.to_lowercase().contains(&needle))
                    .unwrap_or(false)
                || f.issue_text.to_lowercase().contains(&needle)
        })
        .collect()
}

#[tauri::command]
pub fn query_findings(state: State<'_, AppState>, query: FindingsQuery) -> Result<FindingsPageDto> {
    let guard = state
        .session
        .lock()
        .map_err(|_| Error::Busy("Session lock poisoned".into()))?;
    let Some(sess) = guard.as_ref() else {
        return Ok(FindingsPageDto {
            total_matching: 0,
            fixable_matching: 0,
            items: Vec::new(),
        });
    };

    let search = query.search.as_deref().unwrap_or("");
    let issue_filter = query.issue_filter.as_deref().unwrap_or("all");
    let mut matching = filter_findings(&sess.plan, search, issue_filter);

    match query.sort_by.as_deref().unwrap_or("length_desc") {
        "length_desc" => matching.sort_by(|a, b| {
            b.current_len
                .cmp(&a.current_len)
                .then_with(|| a.node.rel.cmp(&b.node.rel))
        }),
        "length_asc" => matching.sort_by(|a, b| {
            a.current_len
                .cmp(&b.current_len)
                .then_with(|| a.node.rel.cmp(&b.node.rel))
        }),
        "path_asc" => matching.sort_by(|a, b| a.node.rel.cmp(&b.node.rel)),
        "severity" => matching.sort_by(|a, b| {
            b.unresolved
                .is_some()
                .cmp(&a.unresolved.is_some())
                .then_with(|| b.issues.0.count_ones().cmp(&a.issues.0.count_ones()))
                .then_with(|| b.current_len.cmp(&a.current_len))
        }),
        _ => {}
    }

    let total_matching = matching.len();
    let fixable_matching = matching.iter().filter(|f| f.new_name.is_some()).count();
    let limit = query.limit.clamp(1, 500);
    let items = matching
        .into_iter()
        .skip(query.offset)
        .take(limit)
        .cloned()
        .collect();

    Ok(FindingsPageDto {
        total_matching,
        fixable_matching,
        items,
    })
}

#[tauri::command]
pub fn inspect_path(state: State<'_, AppState>, rel: String) -> Result<PathInspectionDto> {
    let guard = state
        .session
        .lock()
        .map_err(|_| Error::Busy("Session lock poisoned".into()))?;
    let Some(sess) = guard.as_ref() else {
        return Err(Error::NotFound("Scan a folder first.".into()));
    };

    let finding = sess
        .plan
        .findings
        .iter()
        .find(|f| f.node.rel == rel)
        .ok_or_else(|| Error::NotFound(format!("Item '{rel}' is not in the current plan.")))?;

    let ops_by_rel: std::collections::HashMap<&str, &RenameOp> =
        sess.plan.ops.iter().map(|o| (o.rel.as_str(), o)).collect();
    let findings_by_rel: std::collections::HashMap<&str, &Finding> = sess
        .plan
        .findings
        .iter()
        .map(|f| (f.node.rel.as_str(), f))
        .collect();

    let plain_path = sess.root.join_plain(&rel);
    let extended_path = to_extended(&plain_path)?;
    let disk_meta = filesystem::metadata_extended(&extended_path);
    let is_locked = filesystem::is_locked_for_rename(&extended_path, finding.node.is_directory);

    let mut segments = Vec::new();
    segments.push(SegmentBreakdownDto {
        name: sess.root.plain.clone(),
        new_name: None,
        utf16_len: sess.root.len16,
        new_utf16_len: sess.root.len16,
        issues: Vec::new(),
        is_root: true,
    });

    let mut acc = String::new();
    for part in rel.split('\\') {
        if acc.is_empty() {
            acc.push_str(part);
        } else {
            acc.push('\\');
            acc.push_str(part);
        }
        let seg_new = ops_by_rel.get(acc.as_str()).map(|o| o.new_name.clone());
        let seg_issues = findings_by_rel
            .get(acc.as_str())
            .map(|f| rules::labels(f.issues))
            .unwrap_or_else(|| rules::labels(rules::check(part)));
        let len16 = utf16_len(part);
        let new_len16 = seg_new.as_deref().map(utf16_len).unwrap_or(len16);
        segments.push(SegmentBreakdownDto {
            name: part.to_string(),
            new_name: seg_new,
            utf16_len: len16,
            new_utf16_len: new_len16,
            issues: seg_issues,
            is_root: false,
        });
    }

    Ok(PathInspectionDto {
        rel,
        plain_path,
        extended_path,
        is_directory: finding.node.is_directory,
        is_reparse_point: finding.node.is_reparse_point,
        is_readonly: disk_meta
            .as_ref()
            .map(|m| m.is_readonly)
            .unwrap_or(finding.node.is_readonly),
        is_locked,
        size: disk_meta.map(|m| m.size),
        current_len: finding.current_len,
        planned_len: finding.planned_len,
        limit: sess.plan.limit,
        issue_labels: rules::labels(finding.issues),
        unresolved: finding.unresolved.clone(),
        segments,
    })
}

#[tauri::command]
pub fn validate_custom_name(name: String) -> CustomNameCheckDto {
    let issues = rules::labels(rules::check(&name));
    match validate_single_name(&name) {
        Ok(()) => CustomNameCheckDto {
            valid: issues.is_empty(),
            utf16_len: utf16_len(&name),
            issues,
            error: None,
        },
        Err(e) => CustomNameCheckDto {
            valid: false,
            utf16_len: utf16_len(&name),
            issues,
            error: Some(e.to_string()),
        },
    }
}

#[tauri::command]
pub fn analyze_path_input(input: String, limit: usize) -> PathAnalysisDto {
    let trimmed = input.trim();
    let form = path::classify(trimmed);
    let len16 = utf16_len(trimmed);
    let clamped_limit = limit.clamp(40, 1000);

    if trimmed.is_empty() {
        return PathAnalysisDto {
            input,
            form,
            normalized_plain: None,
            extended_path: None,
            utf16_len: 0,
            exceeds_classic: false,
            exists_on_disk: false,
            is_directory: None,
            is_readonly: None,
            is_reparse_point: None,
            segments: Vec::new(),
            error: None,
        };
    }

    match normalize_absolute(trimmed) {
        Ok(plain) => {
            let ext = to_extended(&plain).ok();
            let meta = ext.as_deref().and_then(filesystem::metadata_extended);
            let norm_len16 = utf16_len(&plain);
            let mut segments = Vec::new();
            for (idx, part) in plain.split('\\').enumerate() {
                if part.is_empty() {
                    continue;
                }
                let is_root = idx == 0 && (part.ends_with(':') || plain.starts_with(r"\\"));
                let seg_issues = if is_root {
                    Vec::new()
                } else {
                    rules::labels(rules::check(part))
                };
                let cleaned = if is_root || seg_issues.is_empty() {
                    None
                } else {
                    let i = rules::check(part);
                    Some(rules::clean(part, i))
                };
                let l16 = utf16_len(part);
                let nl16 = cleaned.as_deref().map(utf16_len).unwrap_or(l16);
                segments.push(SegmentBreakdownDto {
                    name: part.to_string(),
                    new_name: cleaned,
                    utf16_len: l16,
                    new_utf16_len: nl16,
                    issues: seg_issues,
                    is_root,
                });
            }

            PathAnalysisDto {
                input,
                form,
                normalized_plain: Some(plain),
                extended_path: ext,
                utf16_len: norm_len16,
                exceeds_classic: norm_len16 > clamped_limit,
                exists_on_disk: meta.is_some(),
                is_directory: meta.as_ref().map(|m| m.is_dir),
                is_readonly: meta.as_ref().map(|m| m.is_readonly),
                is_reparse_point: meta.as_ref().map(|m| m.is_reparse_point),
                segments,
                error: None,
            }
        }
        Err(_) => {
            // Also allow checking bare filenames in Path Lab
            let mut segments = Vec::new();
            for part in trimmed.split(['\\', '/']).filter(|s| !s.is_empty()) {
                let i = rules::check(part);
                let cleaned = if i.is_empty() {
                    None
                } else {
                    Some(rules::clean(part, i))
                };
                let l16 = utf16_len(part);
                let nl16 = cleaned.as_deref().map(utf16_len).unwrap_or(l16);
                segments.push(SegmentBreakdownDto {
                    name: part.to_string(),
                    new_name: cleaned,
                    utf16_len: l16,
                    new_utf16_len: nl16,
                    issues: rules::labels(i),
                    is_root: false,
                });
            }
            PathAnalysisDto {
                input,
                form,
                normalized_plain: None,
                extended_path: None,
                utf16_len: len16,
                exceeds_classic: len16 > clamped_limit,
                exists_on_disk: false,
                is_directory: None,
                is_readonly: None,
                is_reparse_point: None,
                segments,
                error: Some(
                    "Not an absolute Windows path — showing per-segment filename checks.".into(),
                ),
            }
        }
    }
}

fn select_ops(plan: &Plan, selected_rels: Option<Vec<String>>) -> Vec<RenameOp> {
    match selected_rels {
        None => plan.ops.clone(),
        Some(rels) => {
            let set: HashSet<String> = rels.into_iter().collect();
            plan.ops
                .iter()
                .filter(|o| set.contains(&o.rel))
                .cloned()
                .collect()
        }
    }
}

#[tauri::command]
pub fn preview_repair(
    state: State<'_, AppState>,
    selected_rels: Option<Vec<String>>,
) -> Result<DryRunReport> {
    let guard = state
        .session
        .lock()
        .map_err(|_| Error::Busy("Session lock poisoned".into()))?;
    let Some(sess) = guard.as_ref() else {
        return Err(Error::NotFound("Scan a folder first.".into()));
    };
    let ops = select_ops(&sess.plan, selected_rels);
    state.renamer.dry_run(&sess.root.plain, &ops)
}

#[tauri::command]
pub fn execute_repair(
    app: AppHandle,
    state: State<'_, AppState>,
    selected_rels: Option<Vec<String>>,
    clear_readonly: bool,
) -> Result<()> {
    let (root_plain, limit, kinds, ops) = {
        let guard = state
            .session
            .lock()
            .map_err(|_| Error::Busy("Session lock poisoned".into()))?;
        let Some(sess) = guard.as_ref() else {
            return Err(Error::NotFound("Scan a folder first.".into()));
        };
        let ops = select_ops(&sess.plan, selected_rels);
        if ops.is_empty() {
            return Err(Error::InvalidInput(
                "There are no selected renames to apply.".into(),
            ));
        }
        (sess.root.plain.clone(), sess.plan.limit, sess.kinds, ops)
    };

    state.try_begin_operation("rename")?;
    let cancel = state.cancel_flag.clone();
    let app_for_thread = app.clone();

    std::thread::spawn(move || {
        let app_state = app_for_thread.state::<AppState>();
        let _guard = OperationGuard::new(&app_state);
        let emit_app = app_for_thread.clone();

        match app_state.renamer.apply_with_progress(
            &root_plain,
            &ops,
            clear_readonly,
            &cancel,
            move |prog| {
                let _ = emit_app.emit("repair-progress", &prog);
            },
        ) {
            Ok((batch, outcomes)) => {
                let succeeded = outcomes.iter().filter(|o| o.ok).count();
                let failed = outcomes.len() - succeeded;
                let cancelled = cancel.load(Ordering::Relaxed);
                app_state.logger.info(
                    "repair",
                    &format!("Applied batch under '{root_plain}': {succeeded} succeeded, {failed} skipped/failed"),
                );

                // Re-scan quietly so the workspace stays in sync with disk
                if succeeded > 0 {
                    let no_cancel = std::sync::atomic::AtomicBool::new(false);
                    if let Ok(out) = scan_with_progress(&root_plain, &no_cancel, |_| {}) {
                        let plan = Planner::build(&out.root.plain, &out.nodes, limit, kinds);
                        let session = ScanSession {
                            root: out.root,
                            nodes: out.nodes,
                            warnings: out.warnings,
                            elapsed_ms: out.elapsed_ms,
                            plan,
                            kinds,
                        };
                        let summary = PlanSummaryDto::from_session(&session);
                        if let Ok(mut guard) = app_state.session.lock() {
                            *guard = Some(session);
                        }
                        let _ = app_for_thread.emit("scan-complete", &summary);
                    }
                }

                let _ = app_for_thread.emit(
                    "repair-complete",
                    &RepairCompleteDto {
                        batch,
                        outcomes,
                        succeeded,
                        failed,
                        cancelled,
                    },
                );
            }
            Err(e) => {
                app_state.logger.error("repair", &e.to_string());
                let _ = app_for_thread.emit("repair-error", &e);
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub fn get_history(state: State<'_, AppState>) -> Vec<HistorySummaryDto> {
    state
        .history_store
        .list_batches()
        .into_iter()
        .map(|b| HistorySummaryDto {
            display_time: b.display_time(),
            title: b.title(),
            count: b.done.len(),
            id: b.id,
            time: b.time,
            root: b.root,
            undone: b.undone,
        })
        .collect()
}

#[tauri::command]
pub fn get_history_batch_detail(state: State<'_, AppState>, id: String) -> Result<RenameBatch> {
    state.history_store.get_batch(&id)
}

#[tauri::command]
pub fn undo_batch(
    state: State<'_, AppState>,
    id: String,
    clear_readonly: bool,
) -> Result<Vec<RenameOutcome>> {
    state.try_begin_operation("undo")?;
    let _guard = OperationGuard::new(&state);

    let mut batch = state.history_store.get_batch(&id)?;
    let cancel = state.cancel_flag.clone();
    let outcomes = state
        .renamer
        .undo_with_progress(&mut batch, clear_readonly, &cancel, |_| {})?;
    let ok_count = outcomes.iter().filter(|o| o.ok).count();
    state.logger.info(
        "undo",
        &format!("Undid batch '{id}': {ok_count}/{} restored", outcomes.len()),
    );
    Ok(outcomes)
}

#[tauri::command]
pub fn delete_history_batch(state: State<'_, AppState>, id: String) -> Result<()> {
    state.history_store.delete_batch(&id)
}

#[tauri::command]
pub fn export_findings(state: State<'_, AppState>, save_path: String) -> Result<usize> {
    let guard = state
        .session
        .lock()
        .map_err(|_| Error::Busy("Session lock poisoned".into()))?;
    let Some(sess) = guard.as_ref() else {
        return Err(Error::NotFound("There are no findings to export.".into()));
    };
    if sess.plan.findings.is_empty() {
        return Err(Error::InvalidInput(
            "There are no findings to export.".into(),
        ));
    }
    let path_norm = normalize_absolute(&save_path)?;
    export::export_findings_csv(&PathBuf::from(path_norm), &sess.plan.findings)?;
    Ok(sess.plan.findings.len())
}

#[tauri::command]
pub fn export_outcomes(save_path: String, outcomes: Vec<RenameOutcome>) -> Result<usize> {
    let path_norm = normalize_absolute(&save_path)?;
    export::export_outcomes_csv(&PathBuf::from(path_norm), &outcomes)?;
    Ok(outcomes.len())
}

#[tauri::command]
pub fn get_system_diagnostics(state: State<'_, AppState>) -> SystemDiagnosticsDto {
    SystemDiagnosticsDto {
        policy: LongPathPolicy::detect(),
        volumes: list_logical_volumes(),
        data_dir: state.app_paths.data.to_string_lossy().into_owned(),
        history_dir: state.app_paths.history().to_string_lossy().into_owned(),
        log_file: state
            .app_paths
            .logs()
            .join("longpathfix.log")
            .to_string_lossy()
            .into_owned(),
    }
}

#[tauri::command]
pub fn enable_system_long_paths(state: State<'_, AppState>) -> Result<EnableLongPathsResultDto> {
    let enabled = windows::request_enable_long_paths_elevated()?;
    if enabled {
        state.logger.info(
            "policy",
            "Enabled HKLM LongPathsEnabled=1 via elevated UAC prompt",
        );
    }
    Ok(EnableLongPathsResultDto {
        enabled,
        policy: LongPathPolicy::detect(),
    })
}

#[tauri::command]
pub fn open_in_explorer(target_path: String, select: bool) -> Result<()> {
    let plain = to_plain(&target_path);
    if select {
        windows::reveal_in_explorer(&plain)
    } else {
        windows::open_folder(&plain)
    }
}

#[tauri::command]
pub fn open_data_folder(state: State<'_, AppState>, sub: Option<String>) -> Result<()> {
    let dir = match sub.as_deref() {
        Some("history") => state.app_paths.history(),
        Some("logs") => state.app_paths.logs(),
        _ => state.app_paths.data.clone(),
    };
    std::fs::create_dir_all(&dir)
        .map_err(|e| Error::io("Could not create LongPathFix data folder", e))?;
    let s = dir.to_string_lossy().into_owned();
    windows::open_folder(&s)
}
