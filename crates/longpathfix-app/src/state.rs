use longpathfix_core::config::{AppConfig, ConfigStore};
use longpathfix_core::logging::Logger;
use longpathfix_core::path::Root;
use longpathfix_core::planner::{IssueCounts, LengthBucket, Plan, Planner};
use longpathfix_core::repair::Renamer;
use longpathfix_core::rules::FixKinds;
use longpathfix_core::scanner::{Node, ScanWarning};
use longpathfix_core::storage::HistoryStore;
use longpathfix_core::{AppPaths, Error, Result};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct ScanSession {
    pub root: Root,
    pub nodes: Vec<Node>,
    pub warnings: Vec<ScanWarning>,
    pub elapsed_ms: u64,
    pub plan: Plan,
    pub kinds: FixKinds,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanSummaryDto {
    pub root_plain: String,
    pub root_extended: String,
    pub root_length: usize,
    pub limit: usize,
    pub kinds: u32,
    pub scanned: usize,
    pub scanned_files: usize,
    pub scanned_dirs: usize,
    pub skipped_reparse_points: usize,
    pub readonly_findings: usize,
    pub findings_count: usize,
    pub ops_count: usize,
    pub unresolved_count: usize,
    pub max_len_before: usize,
    pub max_len_after: usize,
    pub elapsed_ms: u64,
    pub warnings_count: usize,
    pub warnings: Vec<ScanWarning>,
    pub issue_counts: IssueCounts,
    pub histogram: Vec<LengthBucket>,
}

impl PlanSummaryDto {
    pub fn from_session(s: &ScanSession) -> Self {
        Self {
            root_plain: s.root.plain.clone(),
            root_extended: s.root.extended.clone(),
            root_length: s.plan.root_length,
            limit: s.plan.limit,
            kinds: s.kinds.0,
            scanned: s.plan.scanned,
            scanned_files: s.plan.scanned_files,
            scanned_dirs: s.plan.scanned_dirs,
            skipped_reparse_points: s.plan.skipped_reparse_points,
            readonly_findings: s.plan.readonly_findings,
            findings_count: s.plan.findings.len(),
            ops_count: s.plan.ops.len(),
            unresolved_count: s.plan.unresolved_count,
            max_len_before: s.plan.max_len_before,
            max_len_after: s.plan.max_len_after,
            elapsed_ms: s.elapsed_ms,
            warnings_count: s.warnings.len(),
            warnings: s.warnings.iter().take(50).cloned().collect(),
            issue_counts: s.plan.issue_counts.clone(),
            histogram: s.plan.histogram.clone(),
        }
    }
}

pub struct AppState {
    pub app_paths: AppPaths,
    pub config_store: ConfigStore,
    pub history_store: HistoryStore,
    pub renamer: Renamer,
    pub logger: Arc<Logger>,
    pub config: Mutex<AppConfig>,
    pub session: Mutex<Option<ScanSession>>,
    pub active_op: Mutex<Option<&'static str>>,
    pub cancel_flag: Arc<AtomicBool>,
}

impl AppState {
    pub fn initialize() -> Self {
        let app_paths = AppPaths::from_env()
            .unwrap_or_else(|_| AppPaths::with_root(std::env::temp_dir().join("LongPathFix")));
        let config_store = ConfigStore::new(&app_paths);
        let config = config_store.load();
        let history_store = HistoryStore::from_app_paths(&app_paths);
        let renamer = Renamer::from_app_paths(&app_paths);
        let logger = Arc::new(Logger::from_app_paths(&app_paths));

        Self {
            app_paths,
            config_store,
            history_store,
            renamer,
            logger,
            config: Mutex::new(config),
            session: Mutex::new(None),
            active_op: Mutex::new(None),
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn try_begin_operation(&self, op_name: &'static str) -> Result<()> {
        let mut guard = self
            .active_op
            .lock()
            .map_err(|_| Error::Busy("State lock poisoned".into()))?;
        if let Some(current) = *guard {
            return Err(Error::Busy(format!(
                "A {current} operation is already running."
            )));
        }
        self.cancel_flag.store(false, Ordering::SeqCst);
        *guard = Some(op_name);
        Ok(())
    }

    pub fn finish_operation(&self) {
        if let Ok(mut guard) = self.active_op.lock() {
            *guard = None;
        }
    }

    pub fn rebuild_plan(&self, limit: usize, kinds: FixKinds) -> Result<PlanSummaryDto> {
        let clamped = limit.clamp(40, 1000);
        let mut guard = self
            .session
            .lock()
            .map_err(|_| Error::Busy("Session lock poisoned".into()))?;
        let Some(sess) = guard.as_mut() else {
            return Err(Error::NotFound("Scan a folder first.".into()));
        };
        sess.plan = Planner::build(&sess.root.plain, &sess.nodes, clamped, kinds);
        sess.kinds = kinds;
        Ok(PlanSummaryDto::from_session(sess))
    }
}

pub struct OperationGuard<'a>(&'a AppState);

impl<'a> OperationGuard<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self(state)
    }
}

impl Drop for OperationGuard<'_> {
    fn drop(&mut self) {
        self.0.finish_operation();
    }
}
