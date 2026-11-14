//! Persistent rename batch history (JSON) and append-only CSV rename audit log.
//!
//! File layout and JSON schema are 100% compatible with LongPathFix 1.x (`%APPDATA%\LongPathFix\history\*.json`
//! and `rename-log.csv`), so batches created by 1.1.0 can still be inspected and undone in 2.0.0.

use crate::filesystem::atomic_write;
use crate::windows::LocalTimestamp;
use crate::{AppPaths, Error, Result};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

/// One completed rename recorded inside a batch (`Dir\OldName` -> `Dir\NewName`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DoneRename {
    pub dir: String,
    pub old_name: String,
    pub new_name: String,
    pub is_directory: bool,
}

/// One applied rename session persisted under `%APPDATA%\LongPathFix\history\<Id>.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RenameBatch {
    pub id: String,
    pub time: String,
    pub root: String,
    #[serde(default)]
    pub done: Vec<DoneRename>,
    #[serde(default)]
    pub undone: bool,
}

impl RenameBatch {
    pub fn display_time(&self) -> String {
        if self.time.len() >= 16 {
            self.time[..16].replace('T', " ")
        } else {
            self.time.clone()
        }
    }

    pub fn title(&self) -> String {
        let suffix = if self.undone { "  ·  undone" } else { "" };
        format!(
            "{}  ·  {} rename(s)  ·  {}{}",
            self.display_time(),
            self.done.len(),
            self.root,
            suffix
        )
    }
}

/// Validates a history batch id so IPC arguments can never escape the history directory.
pub fn validate_batch_id(id: &str) -> Result<()> {
    if id.is_empty()
        || id.len() > 64
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(Error::InvalidInput(
            "Invalid history batch identifier.".into(),
        ));
    }
    Ok(())
}

/// Stores and loads rename batches and the CSV audit log.
#[derive(Debug, Clone)]
pub struct HistoryStore {
    history_dir: PathBuf,
    rename_log_path: PathBuf,
}

impl HistoryStore {
    pub fn from_app_paths(paths: &AppPaths) -> Self {
        Self {
            history_dir: paths.history(),
            rename_log_path: paths.rename_log(),
        }
    }

    pub fn with_history_dir(history_dir: impl Into<PathBuf>) -> Self {
        let history_dir = history_dir.into();
        let parent = history_dir
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| history_dir.clone());
        Self {
            history_dir,
            rename_log_path: parent.join("rename-log.csv"),
        }
    }

    pub fn history_dir(&self) -> &Path {
        &self.history_dir
    }

    pub fn rename_log_path(&self) -> &Path {
        &self.rename_log_path
    }

    fn batch_path(&self, id: &str) -> Result<PathBuf> {
        validate_batch_id(id)?;
        Ok(self.history_dir.join(format!("{id}.json")))
    }

    pub fn save_batch(&self, batch: &RenameBatch) -> Result<()> {
        let path = self.batch_path(&batch.id)?;
        let json = serde_json::to_vec_pretty(batch)?;
        atomic_write(&path, &json)
    }

    pub fn delete_batch(&self, id: &str) -> Result<()> {
        let path = self.batch_path(id)?;
        if !path.exists() {
            return Err(Error::NotFound(format!(
                "History batch '{id}' was not found."
            )));
        }
        std::fs::remove_file(&path)
            .map_err(|e| Error::io(format!("Could not remove history batch {id}"), e))
    }

    pub fn delete_batch_quiet(&self, id: &str) {
        if let Ok(path) = self.batch_path(id) {
            let _ = std::fs::remove_file(path);
        }
    }

    pub fn get_batch(&self, id: &str) -> Result<RenameBatch> {
        let path = self.batch_path(id)?;
        let raw = std::fs::read(&path)
            .map_err(|e| Error::io(format!("Could not read history batch {id}"), e))?;
        let batch: RenameBatch = serde_json::from_slice(&raw)?;
        Ok(batch)
    }

    /// Loads all valid history batches ordered newest first. Corrupt or unreadable files are skipped gracefully.
    pub fn list_batches(&self) -> Vec<RenameBatch> {
        let Ok(entries) = std::fs::read_dir(&self.history_dir) else {
            return Vec::new();
        };
        let mut files: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("json"))
                    .unwrap_or(false)
            })
            .collect();
        files.sort_by(|a, b| b.cmp(a));

        let mut batches = Vec::new();
        for f in files {
            if let Ok(bytes) = std::fs::read(&f) {
                if let Ok(b) = serde_json::from_slice::<RenameBatch>(&bytes) {
                    batches.push(b);
                }
            }
        }
        batches
    }

    /// Appends completed renames to `rename-log.csv` (UTF-8 with BOM on creation).
    pub fn append_csv_log(&self, batch: &RenameBatch, ts: LocalTimestamp) {
        if batch.done.is_empty() {
            return;
        }
        if let Some(parent) = self.rename_log_path.parent() {
            if std::fs::create_dir_all(parent).is_err() {
                return;
            }
        }
        let needs_header = !self.rename_log_path.exists()
            || std::fs::metadata(&self.rename_log_path)
                .map(|m| m.len() == 0)
                .unwrap_or(true);
        let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.rename_log_path)
        else {
            return;
        };

        let mut buf = Vec::with_capacity(batch.done.len() * 128 + 64);
        if needs_header {
            buf.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
            buf.extend_from_slice(b"Time,Folder,Old name,New name\r\n");
        }
        let t = ts.csv_time();
        for d in &batch.done {
            let line = format!(
                "{},{},{},{}\r\n",
                t,
                csv_escape(&d.dir),
                csv_escape(&d.old_name),
                csv_escape(&d.new_name)
            );
            buf.extend_from_slice(line.as_bytes());
        }
        let _ = file.write_all(&buf);
    }
}

/// RFC 4180 CSV escaping for commas, double-quotes, CR, and LF.
pub fn csv_escape(s: &str) -> String {
    if s.contains([',', '"', '\r', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_1x_pascal_case_batch_json() {
        let dir = std::env::temp_dir().join(format!(
            "lpf-hist-compat-{}",
            LocalTimestamp::now().batch_id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let legacy_json = r#"{
  "Id": "20261022-213500-123",
  "Time": "2026-10-22T21:35:00.1234567+06:00",
  "Root": "C:\\data",
  "Done": [
    {
      "Dir": "C:\\data",
      "OldName": "CON.txt",
      "NewName": "CON_.txt",
      "IsDirectory": false
    }
  ],
  "Undone": false
}"#;
        std::fs::write(dir.join("20261022-213500-123.json"), legacy_json).unwrap();
        let store = HistoryStore::with_history_dir(&dir);
        let batches = store.list_batches();
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].id, "20261022-213500-123");
        assert_eq!(batches[0].display_time(), "2026-10-22 21:35");
        assert_eq!(batches[0].done[0].new_name, "CON_.txt");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_invalid_batch_ids() {
        for bad in ["", "..", "../evil", "a\\b", "a/b", "foo.json"] {
            assert!(validate_batch_id(bad).is_err(), "{bad:?}");
        }
        assert!(validate_batch_id("20261114-120000-001").is_ok());
    }
}
