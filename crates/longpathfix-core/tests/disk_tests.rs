//! End-to-end filesystem integration tests matching LongPathFix 1.x `DiskTests` plus new
//! tests for read-only attributes, file locks, dry-run previews, and security boundaries.

use longpathfix_core::filesystem;
use longpathfix_core::path::{to_extended, utf16_len};
use longpathfix_core::planner::{Planner, RenameOp, CLASSIC_LIMIT};
use longpathfix_core::repair::{DryRunStatus, Renamer};
use longpathfix_core::rules::{FixKinds, Issue};
use longpathfix_core::scanner::scan;
use std::collections::{BTreeMap, HashMap};
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(1);

struct TempFixture {
    root: PathBuf,
    hist: PathBuf,
}

impl TempFixture {
    fn new(tag: &str) -> Self {
        let seq = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let stamp = format!("{}-{}-{}", tag, std::process::id(), seq);
        let root = std::env::temp_dir().join(format!("lpf-dt-{stamp}"));
        let hist = std::env::temp_dir()
            .join(format!("lpf-dth-{stamp}"))
            .join("history");
        std::fs::create_dir_all(&root).unwrap();
        Self { root, hist }
    }

    fn root_str(&self) -> String {
        self.root.to_string_lossy().into_owned()
    }

    fn make(&self, rel: &str, content: &str) {
        let full = to_extended(&format!("{}\\{rel}", self.root_str())).unwrap();
        if let Some((parent, _)) = full.rsplit_once('\\') {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&full, content.as_bytes()).unwrap();
    }

    fn snapshot(&self) -> BTreeMap<String, Vec<u8>> {
        let mut map = BTreeMap::new();
        for n in scan(&self.root_str()).unwrap() {
            if !n.is_directory {
                let full = to_extended(&format!("{}\\{}", self.root_str(), n.rel)).unwrap();
                map.insert(n.rel, std::fs::read(&full).unwrap());
            }
        }
        map
    }
}

impl Drop for TempFixture {
    fn drop(&mut self) {
        if let Ok(ext) = to_extended(&self.root_str()) {
            // Clear readonly on anything left behind before cleanup
            if let Ok(nodes) = scan(&self.root_str()) {
                for n in nodes.iter().rev() {
                    if n.is_readonly {
                        if let Ok(p) = to_extended(&format!("{}\\{}", self.root_str(), n.rel)) {
                            let _ = filesystem::set_readonly(&p, false);
                        }
                    }
                }
            }
            let _ = std::fs::remove_dir_all(&ext);
        }
        if let Some(parent) = self.hist.parent() {
            let _ = std::fs::remove_dir_all(parent);
        }
    }
}

#[test]
fn scan_finds_trailing_dots_reserved_names_and_overlong_paths() {
    let fx = TempFixture::new("scan");
    fx.make("trail.", "t");
    fx.make("space ", "s");
    fx.make("CON.txt", "c");
    let deep = format!(
        "{}\\{}.bin",
        vec!["d".repeat(45); 7].join("\\"),
        "f".repeat(40)
    );
    fx.make(&deep, "deep");

    let nodes = scan(&fx.root_str()).unwrap();
    let plan = Planner::build(&fx.root_str(), &nodes, CLASSIC_LIMIT, FixKinds::ALL);

    assert!(plan
        .findings
        .iter()
        .any(|f| f.node.rel == "trail." && f.issues.contains(Issue::TRAILING_DOT_OR_SPACE)));
    assert!(plan
        .findings
        .iter()
        .any(|f| f.node.rel == "space " && f.issues.contains(Issue::TRAILING_DOT_OR_SPACE)));
    assert!(plan
        .findings
        .iter()
        .any(|f| f.node.rel == "CON.txt" && f.issues.contains(Issue::RESERVED_NAME)));
    assert!(plan
        .findings
        .iter()
        .any(|f| f.node.rel == deep && f.issues.contains(Issue::TOO_LONG)));
}

#[test]
fn apply_and_undo_preserve_every_file_content() {
    let fx = TempFixture::new("apply-undo");
    fx.make("fine.txt", "untouched");
    fx.make("trail.", "trailing");
    fx.make("CON.txt", "reserved");
    fx.make(r"Dir.\inner ", "nested");
    let deep = format!(
        "{}\\{}.txt",
        vec!["p".repeat(50); 6].join("\\"),
        "longname".repeat(6)
    );
    fx.make(&deep, "deep-content");

    let before = fx.snapshot();
    let names_before: Vec<String> = before.keys().cloned().collect();

    let nodes = scan(&fx.root_str()).unwrap();
    let plan = Planner::build(&fx.root_str(), &nodes, CLASSIC_LIMIT, FixKinds::ALL);
    let renamer = Renamer::new(Some(fx.hist.clone())).unwrap();
    let (batch_opt, outcomes) = renamer.apply(&fx.root_str(), &plan.ops).unwrap();

    assert!(outcomes.iter().all(|o| o.ok), "{outcomes:?}");
    let batch = batch_opt.expect("batch saved");

    // Re-scan must find zero remaining issues
    let after_nodes = scan(&fx.root_str()).unwrap();
    let again = Planner::build(&fx.root_str(), &after_nodes, CLASSIC_LIMIT, FixKinds::ALL);
    assert!(again.ops.is_empty());
    assert!(again.findings.is_empty());

    let after = fx.snapshot();
    assert_eq!(before.len(), after.len());
    let mut before_vals: Vec<_> = before.values().cloned().collect();
    let mut after_vals: Vec<_> = after.values().cloned().collect();
    before_vals.sort();
    after_vals.sort();
    assert_eq!(before_vals, after_vals);
    assert!(after.contains_key("fine.txt"));
    assert!(!after.contains_key("trail."));
    assert!(after.contains_key("trail"));
    assert!(after.contains_key("CON_.txt"));
    assert!(after.contains_key(r"Dir\inner"));
    for k in after.keys() {
        assert!(
            fx.root_str().len() + 1 + utf16_len(k) <= CLASSIC_LIMIT,
            "{k}"
        );
    }

    assert_eq!(renamer.store().list_batches().len(), 1);
    assert_eq!(renamer.store().list_batches()[0].done.len(), plan.ops.len());
    assert!(renamer.store().rename_log_path().exists());

    // Undo restores exact paths and contents
    let mut loaded_batch = renamer.store().get_batch(&batch.id).unwrap();
    let undo_outcomes = renamer.undo(&mut loaded_batch);
    assert!(undo_outcomes.iter().all(|o| o.ok), "{undo_outcomes:?}");
    assert_eq!(fx.snapshot(), before);
    let names_restored: Vec<String> = fx.snapshot().keys().cloned().collect();
    assert_eq!(names_restored, names_before);
    assert!(renamer.store().get_batch(&batch.id).unwrap().undone);
}

#[test]
fn rename_is_skipped_when_target_already_exists() {
    let fx = TempFixture::new("conflict");
    fx.make("a.", "dotted");
    fx.make("a", "plain");

    let ops = [RenameOp {
        rel: "a.".into(),
        old_name: "a.".into(),
        new_name: "a".into(),
        issues: Issue::TRAILING_DOT_OR_SPACE,
        is_directory: false,
    }];
    let renamer = Renamer::new(Some(fx.hist.clone())).unwrap();
    let (batch, outcomes) = renamer.apply(&fx.root_str(), &ops).unwrap();
    assert!(batch.is_none());
    assert!(!outcomes[0].ok);
    assert!(outcomes[0].message.contains("already exists"));
    assert_eq!(
        std::fs::read_to_string(to_extended(&format!("{}\\a", fx.root_str())).unwrap()).unwrap(),
        "plain"
    );
    assert_eq!(
        std::fs::read_to_string(to_extended(&format!("{}\\a.", fx.root_str())).unwrap()).unwrap(),
        "dotted"
    );
}

#[test]
fn planner_avoids_taking_an_existing_name() {
    let fx = TempFixture::new("avoid-clash");
    fx.make("a.", "dotted");
    fx.make("a", "plain");

    let nodes = scan(&fx.root_str()).unwrap();
    let plan = Planner::build(&fx.root_str(), &nodes, CLASSIC_LIMIT, FixKinds::ALL);
    let renamer = Renamer::new(Some(fx.hist.clone())).unwrap();
    let (_, outcomes) = renamer.apply(&fx.root_str(), &plan.ops).unwrap();
    assert!(outcomes.iter().all(|o| o.ok));

    let snap = fx.snapshot();
    assert_eq!(snap.len(), 2);
    assert_eq!(snap["a"], b"plain");
    assert_eq!(snap["a (2)"], b"dotted");
}

#[test]
fn undo_skips_what_changed_and_reports_it() {
    let fx = TempFixture::new("undo-skip");
    fx.make("x.", "1");
    let nodes = scan(&fx.root_str()).unwrap();
    let plan = Planner::build(&fx.root_str(), &nodes, CLASSIC_LIMIT, FixKinds::ALL);
    let renamer = Renamer::new(Some(fx.hist.clone())).unwrap();
    let (batch, _) = renamer.apply(&fx.root_str(), &plan.ops).unwrap();
    let mut batch = batch.unwrap();

    std::fs::rename(
        to_extended(&format!("{}\\x", fx.root_str())).unwrap(),
        to_extended(&format!("{}\\moved by user", fx.root_str())).unwrap(),
    )
    .unwrap();

    let undo = renamer.undo(&mut batch);
    assert!(!undo[0].ok);
    assert!(undo[0].message.contains("renamed or removed"));
    assert!(!renamer.store().get_batch(&batch.id).unwrap().undone);
}

#[test]
fn nothing_to_rename_writes_no_history() {
    let fx = TempFixture::new("empty");
    let renamer = Renamer::new(Some(fx.hist.clone())).unwrap();
    let (batch, outcomes) = renamer.apply(&fx.root_str(), &[]).unwrap();
    assert!(batch.is_none());
    assert!(outcomes.is_empty());
    assert!(renamer.store().list_batches().is_empty());
}

#[test]
fn dry_run_detects_readonly_locked_and_boundary_violations() {
    let fx = TempFixture::new("dryrun");
    fx.make("ro.", "readonly");
    fx.make("locked.", "locked");
    fx.make("ok.", "ok");

    let ro_ext = to_extended(&format!("{}\\ro.", fx.root_str())).unwrap();
    filesystem::set_readonly(&ro_ext, true).unwrap();

    let locked_ext = to_extended(&format!("{}\\locked.", fx.root_str())).unwrap();
    // Open with exclusive share_mode=0 (no sharing) so rename lock check detects it
    use std::os::windows::fs::OpenOptionsExt;
    let _lock_guard = OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(&locked_ext)
        .unwrap();

    let nodes = scan(&fx.root_str()).unwrap();
    let plan = Planner::build(&fx.root_str(), &nodes, CLASSIC_LIMIT, FixKinds::ALL);
    let renamer = Renamer::new(Some(fx.hist.clone())).unwrap();

    let mut ops = plan.ops.clone();
    ops.push(RenameOp {
        rel: r"..\..\escape\evil.txt".into(),
        old_name: "evil.txt".into(),
        new_name: r"..\outside.txt".into(),
        issues: Issue::NONE,
        is_directory: false,
    });

    let report = renamer.dry_run(&fx.root_str(), &ops).unwrap();
    let by_rel: HashMap<&str, _> = report
        .items
        .iter()
        .map(|i| (i.rel.as_str(), i.status))
        .collect();
    assert_eq!(by_rel["ro."], DryRunStatus::ReadOnly);
    assert_eq!(by_rel["locked."], DryRunStatus::LockedByProcess);
    assert_eq!(by_rel["ok."], DryRunStatus::Ready);
    assert_eq!(
        by_rel[r"..\..\escape\evil.txt"],
        DryRunStatus::InvalidBoundary
    );
}

#[test]
fn unicode_and_emoji_filenames_scan_and_rename_safely() {
    let fx = TempFixture::new("unicode");
    fx.make("Café résumé.", "fr");
    fx.make("日本語フォルダ\\テスト😀 ", "jp");

    let nodes = scan(&fx.root_str()).unwrap();
    let plan = Planner::build(&fx.root_str(), &nodes, CLASSIC_LIMIT, FixKinds::ALL);
    let renamer = Renamer::new(Some(fx.hist.clone())).unwrap();
    let (_, outcomes) = renamer.apply(&fx.root_str(), &plan.ops).unwrap();
    assert!(outcomes.iter().all(|o| o.ok), "{outcomes:?}");

    let snap = fx.snapshot();
    assert_eq!(snap["Café résumé"], b"fr");
    assert_eq!(snap["日本語フォルダ\\テスト😀"], b"jp");
}
